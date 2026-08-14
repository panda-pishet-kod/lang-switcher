//! Enumerating HKLs, building and refreshing the mapping cache, choosing the target
//! layout, cycling through layouts.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module will cover: FR-20, FR-21, FR-35 — implemented here by task
//! T-02-1; FR-30, FR-31, FR-32, FR-33, FR-34 — target layout selection and cycling, left
//! to task T-05-2.
//! Moved out to match the backlog (decision R-17): FR-52 to module `switch`, task T-05-1.
//! Implemented by backlog tasks: T-02-1 (done), T-05-2.
//!
//! # What the cache is
//!
//! FR-20 asks for a full sweep: virtual keys `0x08..=0xFF` times the eight modifier
//! combinations of [`Mods`] times every participating layout, decoded through
//! `ToUnicodeEx`. The result is a two-way mapping per layout:
//!
//! * forward, [`LayoutMap::lookup`] — physical key plus modifiers to the characters that
//!   key produces. This is the path FR-22 walks: conversion is driven by the scan code of
//!   the stroke, never by a character translation table;
//! * backward, [`LayoutMap::find_key`] — character to the key that produces it. This is
//!   the path step 5 of FR-61 walks, where the selection carries characters and the scan
//!   codes are gone.
//!
//! # Threading contract
//!
//! Per section 6.3 of SPEC the cache is written by the input thread when it rebuilds and
//! read by the same input thread while converting. It is therefore an ordinary owned
//! value: no `Mutex`, no `RwLock`, no interior mutability, no atomics. The borrow checker
//! alone separates the two roles, `&mut self` for [`LayoutCache::rebuild`] and `&self` for
//! every lookup. NFR-04 forbids blocking primitives on the hook path, and the cheapest way
//! to obey a ban is to have nothing to ban.
//!
//! * **Reading** — [`LayoutMap::lookup`] and [`LayoutMap::find_key`] are called from the
//!   `WH_KEYBOARD_LL` callback (task T-03-1). Both are allocation free (NFR-03) and take a
//!   bounded, input-independent number of steps (NFR-01).
//! * **Rebuilding** — [`LayoutCache::rebuild`] is called from the window procedure of the
//!   hidden input-thread window on the messages of [`REBUILD_MESSAGES`] (FR-21), *never*
//!   from the hook callback: a rebuild issues thousands of Win32 calls and would break
//!   NFR-01 and NFR-02 by three orders of magnitude. Subscribing to those messages belongs
//!   to the window procedure (task T-01-2) and, for `WM_DEVICECHANGE`, to the device
//!   notification registration of the watchdog (task T-06-2).
//! * **Failure** — a rebuild that fails leaves the previous cache untouched and reports the
//!   failure to the caller, so that the hardwired fallback of FR-25 (module `convert`, task
//!   T-02-2) can be reached. A cache value can never be empty: [`LayoutCache::from_maps`]
//!   rejects that case, which is what makes "the cache did not build" distinguishable from
//!   "these keys carry no characters".
//!
//! # Privacy
//!
//! SEC-01 and SEC-07: nothing decoded here is ever written to a log, a file or a panic
//! message. The cache is an in-memory structure and none of the types below implement
//! `Display` over decoded characters.

use core::ffi::c_void;
use core::fmt;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayoutList, HKL, MAPVK_VK_TO_VSC, MapVirtualKeyExW, ToUnicodeEx, VK_CAPITAL,
    VK_CONTROL, VK_LCONTROL, VK_LSHIFT, VK_MENU, VK_RMENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{WM_DEVICECHANGE, WM_INPUTLANGCHANGE};

/// `ToUnicodeEx` flag `wFlags = 0x4`, "do not change the keyboard state".
///
/// FR-06 makes this flag mandatory and it is the only value this module ever passes.
/// Without it every call mutates the layout's dead key state, and building a cache means
/// thousands of calls in a row: the user's next keystroke in an unrelated application
/// would compose against state this program left behind. The flag is why section 3 of SPEC
/// raises the minimum system to Windows 10 1607.
const TO_UNICODE_NO_STATE: u32 = 0x4;

/// First virtual key of the FR-20 sweep. Everything below `0x08` is a mouse button.
const FIRST_VK: u32 = 0x08;

/// Last virtual key of the FR-20 sweep.
const LAST_VK: u32 = 0xFF;

/// Number of scan codes the forward table indexes: the whole single byte space.
///
/// `MapVirtualKeyExW(MAPVK_VK_TO_VSC)` yields a one byte scan code; the `E0` prefix of an
/// extended key is not part of it and travels in the extended flag of FR-05 instead.
const SCAN_CODES: usize = 256;

/// The eight modifier combinations of FR-20.
const MOD_COMBINATIONS: usize = 8;

/// Longest `ToUnicodeEx` result the cache stores, in UTF-16 code units.
///
/// Matches `Stroke::chars` of FR-04. A result longer than this cannot travel through the
/// stroke buffer, so storing a truncated prefix of it would be a quiet lie; such a key is
/// recorded as producing nothing instead.
pub const MAX_UNITS: usize = 4;

/// Scratch buffer handed to `ToUnicodeEx`, deliberately wider than [`MAX_UNITS`] so that an
/// over-long result is seen as over-long rather than silently clipped by the OS.
const DECODE_UNITS: usize = 16;

/// Capacity of the per-layout reverse index. A power of two, so the modulo is a mask.
///
/// The forward table holds at most `SCAN_CODES * MOD_COMBINATIONS` = 2048 entries and each
/// contributes at most one reverse key, so the load factor never exceeds one half. That is
/// the textbook working range for linear probing and it bounds the probe count by a
/// compile-time constant that does not depend on the keystroke being looked up.
const REVERSE_CAPACITY: usize = 4096;

/// Mask that keeps a slot index inside [`REVERSE_CAPACITY`].
const REVERSE_MASK: usize = REVERSE_CAPACITY - 1;

/// Fibonacci hashing multiplier, the odd integer nearest to 2^64 divided by the golden
/// ratio. Spreads the low, dense Unicode blocks a keyboard layout actually produces.
const REVERSE_MULTIPLIER: u64 = 0x9E37_79B9_7F4A_7C15;

/// Shift that keeps the high 12 bits of the product, `4096 == 1 << 12`.
const REVERSE_SHIFT: u32 = 64 - 12;

/// Marker of a free reverse slot. `u32::MAX` is not a Unicode scalar value, so no real key
/// can collide with it.
const REVERSE_FREE: u32 = u32::MAX;

/// Window messages that oblige the owner of the cache to call [`LayoutCache::rebuild`].
///
/// FR-21. `WM_INPUTLANGCHANGE` covers the user adding, removing or switching a layout;
/// `WM_DEVICECHANGE` covers a keyboard being plugged in or unplugged, which can bring a
/// different physical scan code map with it.
pub const REBUILD_MESSAGES: [u32; 2] = [WM_INPUTLANGCHANGE, WM_DEVICECHANGE];

/// Answers whether `message` is one of [`REBUILD_MESSAGES`].
///
/// Offered so that the window procedure of task T-01-2 does not have to restate FR-21.
pub fn needs_rebuild(message: u32) -> bool {
    REBUILD_MESSAGES.contains(&message)
}

// ---------------------------------------------------------------------------------------
// Layout identity
// ---------------------------------------------------------------------------------------

/// A loaded keyboard layout, held as the numeric value of its `HKL`.
///
/// The raw value is kept instead of the `HKL` itself on purpose. `HKL` is a raw pointer
/// newtype, so a structure holding one is neither `Send` nor `Sync` and could not be moved
/// to the input thread without an `unsafe impl`. The value is a handle, never dereferenced,
/// and `usize` describes it honestly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayoutId(usize);

impl LayoutId {
    /// Wraps the numeric value of an `HKL`.
    pub const fn from_raw(raw: usize) -> Self {
        Self(raw)
    }

    /// The numeric value of the `HKL`, for example `0x0409_0409` for US.
    pub const fn raw(self) -> usize {
        self.0
    }

    /// Low word of the handle: the language identifier, `0x0419` for Russian.
    pub const fn language_id(self) -> u16 {
        self.0 as u16
    }

    /// High word of the handle: the device handle that selects which layout or text
    /// service serves the language of [`LayoutId::language_id`].
    pub const fn device_handle(self) -> u16 {
        (self.0 >> 16) as u16
    }

    /// Whether this layout is served by a TSF/IME text service and must be excluded from
    /// the participating set — FR-35.
    ///
    /// The test is a property of the handle itself: Windows encodes the kind of the input
    /// processor in the top nibble of the device handle, and the value `0xE` is reserved
    /// for IMEs. So `0xE020_0804`, Chinese Simplified Microsoft Pinyin, is an IME, while
    /// `0x0409_0409` is a plain keyboard layout.
    ///
    /// The nibble is compared for equality, not for "has the high bit set". Layouts loaded
    /// under an explicit layout identifier — the Dvorak variants, for instance — carry the
    /// device handle `0xF0nn`, and a "high bit" test would throw those away even though
    /// they are ordinary keyboard layouts with no composition step at all.
    ///
    /// A language based blacklist was rejected: it both over-shoots, since a Chinese user
    /// may have a plain US keyboard loaded under a Chinese language identifier, and
    /// under-shoots, since any language can acquire a third party text service.
    pub const fn is_ime(self) -> bool {
        self.device_handle() & 0xF000 == 0xE000
    }

    /// Rebuilds the `HKL` for a Win32 call.
    ///
    /// Creating a pointer is a safe operation; only a dereference would not be, and this
    /// value is a handle that neither this module nor the OS ever dereferences.
    fn hkl(self) -> HKL {
        HKL(self.0 as *mut c_void)
    }
}

impl fmt::Display for LayoutId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08X}", self.0)
    }
}

// ---------------------------------------------------------------------------------------
// Modifier combinations
// ---------------------------------------------------------------------------------------

/// One of the eight modifier combinations of FR-20.
///
/// Only the three modifiers that change which character a key produces are represented.
/// `Ctrl` and `Alt` on their own mark a command rather than text and reset the buffer
/// (FR-10); the extended flag of FR-05 belongs to the stroke, not to the mapping. The
/// wider stroke mask of FR-04 is narrowed to this type by the buffer module (task T-03-2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mods(u8);

impl Mods {
    /// No modifier held.
    pub const NONE: Self = Self(0b000);
    /// `Shift` held.
    pub const SHIFT: Self = Self(0b001);
    /// `CapsLock` toggled on.
    pub const CAPS: Self = Self(0b010);
    /// `AltGr` held, which the keyboard reports as `Ctrl` plus right `Alt`.
    pub const ALTGR: Self = Self(0b100);

    /// All eight combinations, in the order the forward table indexes them.
    ///
    /// The order matters twice over: it makes the cache reproducible for FR-21, and it
    /// gives the reverse index its preference rule, since the first writer of a character
    /// wins and the plainest combination comes first.
    pub const ALL: [Self; MOD_COMBINATIONS] = [
        Self(0),
        Self(1),
        Self(2),
        Self(3),
        Self(4),
        Self(5),
        Self(6),
        Self(7),
    ];

    /// Builds a combination from the three flags.
    pub const fn new(shift: bool, caps: bool, altgr: bool) -> Self {
        let mut bits = 0u8;
        if shift {
            bits |= Self::SHIFT.0;
        }
        if caps {
            bits |= Self::CAPS.0;
        }
        if altgr {
            bits |= Self::ALTGR.0;
        }
        Self(bits)
    }

    /// Keeps the three meaningful bits of `bits` and drops the rest.
    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits & 0b111)
    }

    /// The three meaningful bits.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Whether `Shift` is part of the combination.
    pub const fn shift(self) -> bool {
        self.0 & Self::SHIFT.0 != 0
    }

    /// Whether `CapsLock` is part of the combination.
    pub const fn caps(self) -> bool {
        self.0 & Self::CAPS.0 != 0
    }

    /// Whether `AltGr` is part of the combination.
    pub const fn altgr(self) -> bool {
        self.0 & Self::ALTGR.0 != 0
    }

    /// Position of this combination inside a forward table row, `0..8`.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A physical key together with the modifiers that make it produce a given character.
///
/// The answer of the reverse lookup, and the coordinates the conversion of FR-22 works in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyPress {
    /// Scan code of the physical key, low byte only.
    pub scan: u16,
    /// Modifier combination.
    pub mods: Mods,
}

// ---------------------------------------------------------------------------------------
// One cache entry
// ---------------------------------------------------------------------------------------

/// Which of the four outcomes of `ToUnicodeEx` produced a [`KeyMapping`].
///
/// NFR-13 requires every one of them to be handled explicitly, and they mean four different
/// things to the conversion path, so they are kept apart rather than collapsed into
/// "some characters or none".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MappingKind {
    /// Return value `0`: the key produces no character in this layout. A normal outcome,
    /// not an error, and the case FR-23 turns into "carry the character over unchanged".
    #[default]
    None,
    /// Return value `1`: exactly one UTF-16 code unit.
    Char,
    /// Return value greater than `1`: a ligature in the sense of section 11.1 of SPEC, or a
    /// single character outside the BMP delivered as a surrogate pair.
    Ligature,
    /// Negative return value: a dead key — FR-24. The stroke is carried through conversion
    /// unchanged; the code units below hold the dead character itself.
    Dead,
}

/// What one physical key produces under one modifier combination in one layout.
///
/// Plain `Copy` data of twelve bytes: a lookup returns it by value and allocates nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyMapping {
    kind: MappingKind,
    len: u8,
    units: [u16; MAX_UNITS],
}

impl KeyMapping {
    /// The key produces nothing.
    pub const EMPTY: Self = Self {
        kind: MappingKind::None,
        len: 0,
        units: [0; MAX_UNITS],
    };

    /// A mapping that produces `ch`.
    ///
    /// Offered for the hardwired RU/EN fallback table of FR-25, which module `convert`
    /// builds without asking the OS anything (task T-02-2), and for tests. The live cache
    /// goes through [`KeyMapping::from_to_unicode`] instead.
    pub fn from_char(ch: char) -> Self {
        let mut units = [0u16; MAX_UNITS];
        let written = ch.encode_utf16(&mut units).len();
        Self {
            kind: if written == 1 {
                MappingKind::Char
            } else {
                MappingKind::Ligature
            },
            len: written as u8,
            units,
        }
    }

    /// A dead key mapping carrying `ch` as its dead character — FR-24.
    pub fn dead(ch: char) -> Self {
        Self {
            kind: MappingKind::Dead,
            ..Self::from_char(ch)
        }
    }

    /// Turns one `ToUnicodeEx` result into a cache entry.
    ///
    /// `produced` is the raw return value and `buffer` the buffer the call filled. All four
    /// outcomes of NFR-13 are decided here and nowhere else.
    pub fn from_to_unicode(produced: i32, buffer: &[u16]) -> Self {
        if produced < 0 {
            // Dead key. The call reports the dead character itself in the first unit; a
            // negative return with an empty buffer would mean the OS contradicted its own
            // contract, and the key is then recorded as producing nothing.
            return match buffer.first() {
                Some(&unit) => {
                    let mut units = [0u16; MAX_UNITS];
                    units[0] = unit;
                    Self {
                        kind: MappingKind::Dead,
                        len: 1,
                        units,
                    }
                }
                None => Self::EMPTY,
            };
        }

        let produced = produced as usize;
        if produced == 0 {
            // No character on this key in this layout. Normal, and not an error.
            return Self::EMPTY;
        }

        // The OS never writes past the buffer it was given, but the length is what indexes
        // memory below, so it is clamped rather than trusted.
        let produced = produced.min(buffer.len());
        if produced > MAX_UNITS {
            // Longer than a stroke of FR-04 can carry. Storing the first MAX_UNITS units
            // would be a mapping that silently differs from what the key really produces,
            // so the key is recorded as producing nothing instead.
            return Self::EMPTY;
        }

        let mut units = [0u16; MAX_UNITS];
        units[..produced].copy_from_slice(&buffer[..produced]);
        Self {
            kind: if produced == 1 {
                MappingKind::Char
            } else {
                MappingKind::Ligature
            },
            len: produced as u8,
            units,
        }
    }

    /// Which outcome of `ToUnicodeEx` this entry came from.
    pub const fn kind(self) -> MappingKind {
        self.kind
    }

    /// The UTF-16 code units the key produces, borrowed from the entry itself.
    ///
    /// Empty for [`MappingKind::None`]. No allocation: the units live inline.
    pub fn units(&self) -> &[u16] {
        &self.units[..self.len as usize]
    }

    /// Whether the key produces nothing in this layout.
    pub const fn is_empty(self) -> bool {
        matches!(self.kind, MappingKind::None)
    }

    /// Whether the key is a dead key — FR-24.
    pub const fn is_dead(self) -> bool {
        matches!(self.kind, MappingKind::Dead)
    }

    /// Whether the key produces more than one character — section 11.1 of SPEC.
    pub const fn is_ligature(self) -> bool {
        matches!(self.kind, MappingKind::Ligature)
    }

    /// The single Unicode scalar value this entry produces, if it produces exactly one.
    ///
    /// A surrogate pair counts as one. A true ligature of several scalar values does not,
    /// and neither does an empty entry.
    pub fn single_char(&self) -> Option<char> {
        let mut decoded = char::decode_utf16(self.units().iter().copied());
        match (decoded.next(), decoded.next()) {
            (Some(Ok(ch)), None) => Some(ch),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------------------
// Per-layout map
// ---------------------------------------------------------------------------------------

/// One slot of the reverse index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReverseSlot {
    /// Unicode scalar value, or [`REVERSE_FREE`] when the slot is free.
    key: u32,
    scan: u16,
    mods: Mods,
}

impl ReverseSlot {
    const FREE: Self = Self {
        key: REVERSE_FREE,
        scan: 0,
        mods: Mods::NONE,
    };
}

/// Position of a `(scan, mods)` pair inside the forward table.
///
/// Only the low byte of the scan code selects a row, which is exactly what
/// `MapVirtualKeyExW(MAPVK_VK_TO_VSC)` and `KBDLLHOOKSTRUCT::scanCode` deliver. The mask
/// also guarantees the index stays inside the table for any input.
const fn forward_index(scan: u16, mods: Mods) -> usize {
    (scan as usize & 0xFF) * MOD_COMBINATIONS + mods.index()
}

/// Slot a character hashes to.
fn reverse_slot(key: u32) -> usize {
    ((u64::from(key).wrapping_mul(REVERSE_MULTIPLIER) >> REVERSE_SHIFT) as usize) & REVERSE_MASK
}

/// The two-way mapping of one keyboard layout — FR-20.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutMap {
    layout: LayoutId,
    /// `SCAN_CODES * MOD_COMBINATIONS` entries, indexed by [`forward_index`].
    forward: Box<[KeyMapping]>,
    /// `REVERSE_CAPACITY` slots, open addressed with linear probing.
    reverse: Box<[ReverseSlot]>,
    /// Number of forward entries that carry a character.
    filled: u32,
}

impl LayoutMap {
    /// Which layout this map describes.
    pub fn layout(&self) -> LayoutId {
        self.layout
    }

    /// What the physical key `scan` produces under `mods` — the forward path of FR-22.
    ///
    /// One index computation and one array read: no allocation (NFR-03), no lock (NFR-04),
    /// no branch that depends on how full the table is (NFR-01). Safe to call from the hook
    /// callback.
    pub fn lookup(&self, scan: u16, mods: Mods) -> KeyMapping {
        self.forward[forward_index(scan, mods)]
    }

    /// Which key produces `ch` — the backward path of step 5 of FR-61.
    ///
    /// Returns the plainest modifier combination that produces the character, since the
    /// index is filled in the order of [`Mods::ALL`] and the first writer of a character
    /// keeps the slot. Dead keys are deliberately absent from the index: replaying one
    /// would start a composition instead of inserting a character.
    ///
    /// Reads a fixed-capacity open addressed table filled to at most one half, so the probe
    /// walk is bounded by a compile-time constant and allocates nothing.
    pub fn find_key(&self, ch: char) -> Option<KeyPress> {
        let key = ch as u32;
        let mut slot = reverse_slot(key);
        for _ in 0..REVERSE_CAPACITY {
            let entry = self.reverse[slot];
            if entry.key == REVERSE_FREE {
                return None;
            }
            if entry.key == key {
                return Some(KeyPress {
                    scan: entry.scan,
                    mods: entry.mods,
                });
            }
            slot = (slot + 1) & REVERSE_MASK;
        }
        None
    }

    /// How many key and modifier combinations carry a character in this layout.
    pub fn len(&self) -> usize {
        self.filled as usize
    }

    /// Whether the layout produced no characters at all.
    ///
    /// True only for a map that failed to build; [`LayoutCache::from_maps`] refuses a cache
    /// in which every map is empty.
    pub fn is_empty(&self) -> bool {
        self.filled == 0
    }
}

/// Fills a [`LayoutMap`] entry by entry, then freezes it.
///
/// Two callers: the sweep of FR-20 below, and the hardwired fallback table of FR-25 that
/// module `convert` will build without the OS (task T-02-2). The reverse index is derived
/// once in [`LayoutMapBuilder::finish`], so the two directions cannot drift apart.
#[derive(Clone, Debug)]
pub struct LayoutMapBuilder {
    layout: LayoutId,
    forward: Box<[KeyMapping]>,
    filled: u32,
}

impl LayoutMapBuilder {
    /// An empty builder for `layout`.
    pub fn new(layout: LayoutId) -> Self {
        Self {
            layout,
            forward: vec![KeyMapping::EMPTY; SCAN_CODES * MOD_COMBINATIONS].into_boxed_slice(),
            filled: 0,
        }
    }

    /// Records `mapping` for `scan` under `mods` and reports whether it was stored.
    ///
    /// An entry that produces nothing is never stored, and an occupied slot is never
    /// overwritten. Together the two rules settle the one real collision of the sweep:
    /// several virtual keys map to the same scan code — `VK_INSERT` and `VK_NUMPAD0` both
    /// map to `0x52` — and the rule keeps the one that carries a character regardless of
    /// the order they are offered in. That makes the result reproducible, which is what
    /// FR-21 needs from a rebuild.
    pub fn set(&mut self, scan: u16, mods: Mods, mapping: KeyMapping) -> bool {
        if mapping.is_empty() {
            return false;
        }
        let index = forward_index(scan, mods);
        if !self.forward[index].is_empty() {
            return false;
        }
        self.forward[index] = mapping;
        self.filled += 1;
        true
    }

    /// Derives the reverse index and freezes the map.
    pub fn finish(self) -> LayoutMap {
        let mut reverse = vec![ReverseSlot::FREE; REVERSE_CAPACITY].into_boxed_slice();
        for scan in 0..SCAN_CODES {
            for mods in Mods::ALL {
                let mapping = self.forward[scan * MOD_COMBINATIONS + mods.index()];
                if mapping.is_empty() || mapping.is_dead() {
                    continue;
                }
                let Some(ch) = mapping.single_char() else {
                    // A ligature of several scalar values has no single character to be
                    // found by, and matching a sequence is the business of the selection
                    // path, not of a constant time index.
                    continue;
                };
                insert_reverse(&mut reverse, ch as u32, scan as u16, mods);
            }
        }
        LayoutMap {
            layout: self.layout,
            forward: self.forward,
            reverse,
            filled: self.filled,
        }
    }
}

/// Inserts one character into the reverse index, keeping the first writer.
///
/// The table holds at most 2048 keys in 4096 slots, so the walk always meets a free slot;
/// the bound on the loop is a guard against a future change to those two numbers, not an
/// expected outcome.
fn insert_reverse(table: &mut [ReverseSlot], key: u32, scan: u16, mods: Mods) {
    let mut slot = reverse_slot(key);
    for _ in 0..REVERSE_CAPACITY {
        if table[slot].key == key {
            return;
        }
        if table[slot].key == REVERSE_FREE {
            table[slot] = ReverseSlot { key, scan, mods };
            return;
        }
        slot = (slot + 1) & REVERSE_MASK;
    }
}

// ---------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------

/// Why the cache could not be built.
///
/// The point of the type is point 4 of FR-25: a caller must be able to tell "the cache did
/// not build" from "the keys carry no characters". A built cache is never empty, so the two
/// answers never look alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// `GetKeyboardLayoutList` failed. It returns `0` on error, and NFR-13 forbids reading
    /// that as "the session has no layouts": a session always has at least one.
    Enumeration,
    /// Every loaded layout is served by an IME and was excluded by FR-35, so there is
    /// nothing this program can convert between.
    NoUsableLayouts,
    /// Layouts were enumerated but not one of them produced a single character. Either
    /// `ToUnicodeEx` is failing wholesale or the layouts are unusable; either way the
    /// fallback of FR-25 is the right answer, not an empty cache.
    Empty,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Enumeration => "the system layout list could not be read",
            Self::NoUsableLayouts => "every loaded layout is IME based and excluded by FR-35",
            Self::Empty => "no loaded layout produced a single character",
        };
        f.write_str(text)
    }
}

impl std::error::Error for LayoutError {}

// ---------------------------------------------------------------------------------------
// Enumeration — FR-20, FR-35
// ---------------------------------------------------------------------------------------

/// Every keyboard layout loaded in this session, in system order, IMEs included.
///
/// Uses the two-call pattern `GetKeyboardLayoutList` mandates: the first call learns the
/// count, the second fills a buffer of exactly that size.
pub fn enumerate_all() -> Result<Vec<LayoutId>, LayoutError> {
    // SAFETY: the counting form of the two-call pattern. `None` is passed straight through
    // as a count of zero and a null buffer pointer, which is the documented way to ask for
    // the count, and the OS writes nothing in that form. The return value is an `i32` count
    // and is checked below: `0` means the call failed (NFR-13), not that the session has no
    // layouts.
    let count = unsafe { GetKeyboardLayoutList(None) };
    if count <= 0 {
        return Err(LayoutError::Enumeration);
    }

    let mut buffer = vec![HKL::default(); count as usize];
    // SAFETY: `buffer` is exactly `count` elements long, the count the OS itself just
    // reported, and the wrapper derives the `nBuff` argument from the slice, so the OS
    // cannot write past the end even if the layout list grew between the two calls — it
    // would then fill the buffer and report the smaller number. The return value is checked
    // and clamped to the buffer length before anything is read.
    let written = unsafe { GetKeyboardLayoutList(Some(&mut buffer)) };
    if written <= 0 {
        return Err(LayoutError::Enumeration);
    }

    let written = (written as usize).min(buffer.len());
    Ok(buffer[..written]
        .iter()
        .map(|handle| LayoutId::from_raw(handle.0 as usize))
        .collect())
}

/// The layouts this program participates with: [`enumerate_all`] minus the IMEs of FR-35.
pub fn enumerate() -> Result<Vec<LayoutId>, LayoutError> {
    let mut layouts = enumerate_all()?;
    layouts.retain(|layout| !layout.is_ime());
    if layouts.is_empty() {
        return Err(LayoutError::NoUsableLayouts);
    }
    Ok(layouts)
}

// ---------------------------------------------------------------------------------------
// The sweep — FR-20
// ---------------------------------------------------------------------------------------

/// The keyboard state array `ToUnicodeEx` reads, for one modifier combination.
///
/// The high bit marks a key held down, the low bit marks a toggle that is on. Both the
/// neutral and the sided virtual keys are set: layouts differ in which one they consult,
/// and `AltGr` is only recognised as `Ctrl` plus *right* `Alt`.
fn keyboard_state(mods: Mods) -> [u8; 256] {
    const DOWN: u8 = 0x80;
    const TOGGLED: u8 = 0x01;

    let mut state = [0u8; 256];
    if mods.shift() {
        state[VK_SHIFT.0 as usize] = DOWN;
        state[VK_LSHIFT.0 as usize] = DOWN;
    }
    if mods.altgr() {
        state[VK_CONTROL.0 as usize] = DOWN;
        state[VK_LCONTROL.0 as usize] = DOWN;
        state[VK_MENU.0 as usize] = DOWN;
        state[VK_RMENU.0 as usize] = DOWN;
    }
    if mods.caps() {
        state[VK_CAPITAL.0 as usize] = TOGGLED;
    }
    state
}

/// Scan code of the physical key that carries `vk` in `layout`, or `0` if there is none.
fn scan_code_of(vk: u32, layout: LayoutId) -> u16 {
    // SAFETY: MapVirtualKeyExW takes three values and returns one; it reads nothing through
    // a pointer and writes nothing back, so there is no buffer whose size could be wrong.
    // The handle came from GetKeyboardLayoutList and is a layout loaded in this session, and
    // an unloaded or invalid handle would only make the call return `0`, which the caller
    // treats as "no such key" (NFR-13).
    let scan = unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC, Some(layout.hkl())) };
    // MAPVK_VK_TO_VSC yields a one byte scan code; the mask states that expectation rather
    // than assuming it.
    (scan & 0xFF) as u16
}

/// What `vk` produces in `layout` under the keyboard state `state` — FR-06.
fn decode(vk: u32, scan: u16, state: &[u8; 256], layout: LayoutId) -> KeyMapping {
    let mut buffer = [0u16; DECODE_UNITS];
    // SAFETY: `state` is exactly the 256 byte array the API contract demands, and the
    // wrapper derives `cchBuff` from `buffer` itself, so the OS cannot write past the end of
    // either. The handle came from GetKeyboardLayoutList. TO_UNICODE_NO_STATE is FR-06 and
    // is what makes the call re-entrant with respect to the user's own typing: without it
    // this sweep would leave the layout's dead key state altered for every other
    // application in the session. The return value is an `i32` whose four cases are decided
    // in full by KeyMapping::from_to_unicode.
    let produced = unsafe {
        ToUnicodeEx(
            vk,
            u32::from(scan),
            state,
            &mut buffer,
            TO_UNICODE_NO_STATE,
            Some(layout.hkl()),
        )
    };
    KeyMapping::from_to_unicode(produced, &buffer)
}

/// Sweeps every virtual key and modifier combination of FR-20 for one layout.
fn build_layout_map(layout: LayoutId) -> LayoutMap {
    let states: [[u8; 256]; MOD_COMBINATIONS] = core::array::from_fn(|index| {
        keyboard_state(Mods::from_bits_truncate(
            u8::try_from(index).unwrap_or_default(),
        ))
    });

    let mut builder = LayoutMapBuilder::new(layout);
    for vk in FIRST_VK..=LAST_VK {
        let scan = scan_code_of(vk, layout);
        if scan == 0 {
            // The layout has no physical key for this virtual key. Nothing to record, and
            // scan code `0` must not become a bucket that everything falls into.
            continue;
        }
        for mods in Mods::ALL {
            builder.set(scan, mods, decode(vk, scan, &states[mods.index()], layout));
        }
    }
    builder.finish()
}

// ---------------------------------------------------------------------------------------
// The cache — FR-20, FR-21
// ---------------------------------------------------------------------------------------

/// The mapping cache of FR-20: one [`LayoutMap`] per participating layout.
///
/// Owned by the input thread and never shared; see the threading contract in the module
/// documentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutCache {
    maps: Vec<LayoutMap>,
}

impl LayoutCache {
    /// Enumerates the layouts of FR-35 and sweeps every one of them — FR-20.
    ///
    /// Called once at start-up and again on every message of [`REBUILD_MESSAGES`]. Never
    /// from the hook callback: the sweep costs thousands of Win32 calls.
    pub fn build() -> Result<Self, LayoutError> {
        let layouts = enumerate()?;
        let mut maps = Vec::with_capacity(layouts.len());
        for layout in layouts {
            maps.push(build_layout_map(layout));
        }
        Self::from_maps(maps)
    }

    /// Assembles a cache from maps that are already built.
    ///
    /// The seam between "where the mappings came from" and "what a usable cache is", so
    /// that the fallback table of FR-25 can produce a cache without the OS. Rejects the two
    /// shapes that would be indistinguishable from a working but silent cache: no layouts
    /// at all, and layouts none of which produced a character.
    pub fn from_maps(maps: Vec<LayoutMap>) -> Result<Self, LayoutError> {
        if maps.is_empty() {
            return Err(LayoutError::NoUsableLayouts);
        }
        if maps.iter().all(LayoutMap::is_empty) {
            return Err(LayoutError::Empty);
        }
        Ok(Self { maps })
    }

    /// Rebuilds the cache in place — FR-21.
    ///
    /// Called by the window procedure of the input thread when [`needs_rebuild`] answers
    /// yes. Idempotent: the same system state yields an equal cache. Safe to call again
    /// after a failure and safe to call repeatedly, because the new cache is built to
    /// completion first and only then replaces the old one — a failed rebuild leaves the
    /// previous cache exactly as it was rather than a half-filled one.
    pub fn rebuild(&mut self) -> Result<(), LayoutError> {
        let rebuilt = Self::build()?;
        *self = rebuilt;
        Ok(())
    }

    /// The per-layout maps, in the order the system listed the layouts.
    pub fn maps(&self) -> &[LayoutMap] {
        &self.maps
    }

    /// How many layouts the cache covers. Never zero.
    pub fn len(&self) -> usize {
        self.maps.len()
    }

    /// Always false: a cache with no layouts is rejected at construction. Present because
    /// [`LayoutCache::len`] is, and because callers should not have to know the invariant to
    /// write the obvious check.
    pub fn is_empty(&self) -> bool {
        self.maps.is_empty()
    }

    /// Position of `layout` in [`LayoutCache::maps`].
    ///
    /// The hook path resolves a layout to its position once and then keeps the position, so
    /// that the per-keystroke work is the array read of [`LayoutMap::lookup`] alone. The
    /// scan itself is over a handful of elements — the system layout list — and allocates
    /// nothing.
    pub fn index_of(&self, layout: LayoutId) -> Option<usize> {
        self.maps.iter().position(|map| map.layout == layout)
    }

    /// The map of `layout`, if the layout participates.
    pub fn get(&self, layout: LayoutId) -> Option<&LayoutMap> {
        self.maps.iter().find(|map| map.layout == layout)
    }

    /// The map at `index` of [`LayoutCache::maps`].
    pub fn map_at(&self, index: usize) -> Option<&LayoutMap> {
        self.maps.get(index)
    }

    /// Whether `layout` participates.
    pub fn contains(&self, layout: LayoutId) -> bool {
        self.get(layout).is_some()
    }
}
