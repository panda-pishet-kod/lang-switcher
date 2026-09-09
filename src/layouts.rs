//! Enumerating HKLs, building and refreshing the mapping cache, choosing the target
//! layout, cycling through layouts.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-20, FR-21, FR-35 — implemented here by task T-02-1;
//! FR-30, FR-31, FR-33 and the target-choosing half of FR-32 — the selection of the target
//! layout and the cycle of section 4.4, task T-05-2.
//! Moved out to match the backlog (decision R-17): FR-52 to module `switch`, task T-05-1;
//! **FR-34 to module `buffer`, task T-05-2** — the position counter is reset by every rule of
//! FR-10 without exception, so it lives beside the ring it is reset with and not beside the
//! cycle it counts along. It was listed here by task T-01-1a and is listed here no longer;
//! `buffer` names it now, which is the documentation debt of section 4.3 of `STATE.md`.
//! Implemented by backlog tasks: T-02-1 (done), T-02-1a (done), T-05-2 (done).
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
//! # What a physical key is here — task T-02-1a
//!
//! A scan code alone does not name a key. The keypad repeats the low byte of the main
//! block: the `/` of the keypad and the `/?` key both report `0x35`, and the hardware tells
//! them apart by the `E0` prefix the hook delivers as `LLKHF_EXTENDED` (FR-05). A cache
//! keyed by the scan code alone therefore lets one of the two keys take the slot of the
//! other, and on the RU/EN pair the visible half of that is a punctuation mark: the `/?` key
//! reads `/` in US and `.` in Russian, so EN `/` has to convert to RU `.` and used to come
//! out as `/` because the keypad had written the slot first.
//!
//! The key of this cache is therefore **nine bits wide**: the low eight are the scan code,
//! the ninth is the extended flag — see [`key_index`]. The sweep reads both from one call,
//! `MapVirtualKeyExW` with `MAPVK_VK_TO_VSC_EX`, which returns the prefix in the high byte
//! (`0xE035` for the keypad `/`, `0x0035` for the `/?` key).
//!
//! `MAPVK_VK_TO_VSC_EX` does not separate every pair that shares a low byte:
//! `VK_DELETE`/`VK_DECIMAL` both report `0x0053`, `VK_HOME`/`VK_NUMPAD7` both `0x0047`,
//! `VK_END`/`VK_NUMPAD1` both `0x004F`, `VK_INSERT`/`VK_NUMPAD0` both `0x0052`. Windows does
//! not mark the navigation block extended in a layout table. Those pairs stay collapsed and
//! that is harmless: each is "a keypad character against a navigation key", both readings
//! agree across RU and EN, and the rule of [`LayoutMapBuilder::set`] keeps whichever of the
//! two carries a character. The pair conversion actually depends on is `0x35`, and
//! `MAPVK_VK_TO_VSC_EX` does separate that one.
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
use core::sync::atomic::{AtomicU8, AtomicU32, AtomicUsize, Ordering, fence};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayoutList, HKL, MAPVK_VK_TO_VSC_EX, MapVirtualKeyExW, ToUnicodeEx, VK_CAPITAL,
    VK_CONTROL, VK_LCONTROL, VK_LSHIFT, VK_MENU, VK_RMENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{WM_DEVICECHANGE, WM_INPUTLANGCHANGE};

use crate::settings::{self, LayoutMode};

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
/// The low byte of `MapVirtualKeyExW(MAPVK_VK_TO_VSC_EX)`, and the whole of
/// `KBDLLHOOKSTRUCT::scanCode`.
const SCAN_CODES: usize = 256;

/// Ninth bit of the cache key: the key is an extended one, the `E0` of FR-05.
const EXTENDED_BIT: usize = 0x100;

/// Number of physical keys the forward table indexes — task T-02-1a.
///
/// Every scan code twice over, once plain and once extended, so that the keypad and the main
/// block never share a slot. Two times 256 is exactly [`EXTENDED_BIT`] doubled, which is what
/// makes [`key_index`] a mask rather than a bounds check.
const KEY_SLOTS: usize = SCAN_CODES * 2;

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
/// The forward table holds at most `KEY_SLOTS * MOD_COMBINATIONS` = 4096 entries and each
/// contributes at most one reverse key, so the load factor never exceeds one half. That is
/// the textbook working range for linear probing and it bounds the probe count by a
/// compile-time constant that does not depend on the keystroke being looked up.
///
/// Task T-02-1a doubled [`KEY_SLOTS`], so this doubled with it: at the previous 4096 slots
/// the load factor of a full table would have been one, at which linear probing degenerates
/// and [`insert_reverse`] could meet no free slot at all.
const REVERSE_CAPACITY: usize = 8192;

/// Mask that keeps a slot index inside [`REVERSE_CAPACITY`].
const REVERSE_MASK: usize = REVERSE_CAPACITY - 1;

/// Fibonacci hashing multiplier, the odd integer nearest to 2^64 divided by the golden
/// ratio. Spreads the low, dense Unicode blocks a keyboard layout actually produces.
const REVERSE_MULTIPLIER: u64 = 0x9E37_79B9_7F4A_7C15;

/// Shift that keeps the high 13 bits of the product, `8192 == 1 << 13`.
const REVERSE_SHIFT: u32 = 64 - 13;

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
/// (FR-10). The extended flag of FR-05 is not among them either, and for a different
/// reason: it does not modify a key, it *names* one, so it belongs to the other half of the
/// cache key — see [`key_index`]. The wider stroke mask of FR-04 is narrowed to this type by
/// the buffer module (task T-03-2).
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
///
/// The three fields are exactly what a replay of the key needs: task T-07-2 puts [`scan`]
/// into `KEYBDINPUT::wScan` and [`extended`] into `KEYEVENTF_EXTENDEDKEY`, which is FR-05
/// read backwards. That is why the extended flag is a field of its own rather than a bit
/// folded into [`scan`]: a scan code that had `0x100` set in it would be injected as a
/// different key.
///
/// [`scan`]: KeyPress::scan
/// [`extended`]: KeyPress::extended
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyPress {
    /// Scan code of the physical key, low byte only.
    pub scan: u16,
    /// Whether the key is an extended one — the `E0` prefix of FR-05. `true` for the keypad
    /// `/`, `false` for the `/?` key of the main block, which share the scan code `0x35`.
    pub extended: bool,
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
/// Plain `Copy` data of ten bytes: a lookup returns it by value and allocates nothing.
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
            // Dead key. The call reports the dead character itself in the first unit, and the
            // check is on that unit rather than on the length of the buffer: the one product
            // caller — `decode` — always hands over a full slice of DECODE_UNITS zeroes, so an
            // OS that returned a dead key without writing the character ("if possible", says
            // the documentation) would leave a zero here, not an empty slice. Such a return
            // contradicts the contract of the call, and the key is then recorded as producing
            // nothing: a dead mapping carrying NUL would be carried over unchanged by FR-24
            // and handed to `SendInput` as a character.
            let unit = buffer.first().copied().unwrap_or(0);
            if unit == 0 {
                return Self::EMPTY;
            }

            let mut units = [0u16; MAX_UNITS];
            units[0] = unit;
            return Self {
                kind: MappingKind::Dead,
                len: 1,
                units,
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
    extended: bool,
    mods: Mods,
}

impl ReverseSlot {
    const FREE: Self = Self {
        key: REVERSE_FREE,
        scan: 0,
        extended: false,
        mods: Mods::NONE,
    };
}

/// The nine bit key of one physical key — task T-02-1a.
///
/// The low eight bits are the scan code, the ninth is the extended flag: `0x035` is the `/?`
/// key of the main block and `0x135` the `/` of the keypad. The two halves of the table are
/// therefore whole scan code spaces rather than interleaved, which keeps a dump of the table
/// readable and makes "the extended keys" a contiguous range.
///
/// The scan code is masked to its low byte rather than checked, so the result is inside
/// `0..KEY_SLOTS` for **every** `u16` — including a scan code the hook reported with the
/// `E0` prefix still in it. The bound is a property of the arithmetic, not of the caller.
const fn key_index(scan: u16, extended: bool) -> usize {
    (scan as usize & 0xFF) | if extended { EXTENDED_BIT } else { 0 }
}

/// Position of a `(scan, extended, mods)` triple inside the forward table.
///
/// Both factors are bounded by construction — [`key_index`] by its mask and [`Mods::index`]
/// by the three bits of [`Mods`] — so the product is below `KEY_SLOTS * MOD_COMBINATIONS`
/// for any input and the table read below can never leave the table.
const fn forward_index(scan: u16, extended: bool, mods: Mods) -> usize {
    key_index(scan, extended) * MOD_COMBINATIONS + mods.index()
}

/// Slot a character hashes to.
fn reverse_slot(key: u32) -> usize {
    ((u64::from(key).wrapping_mul(REVERSE_MULTIPLIER) >> REVERSE_SHIFT) as usize) & REVERSE_MASK
}

/// The two-way mapping of one keyboard layout — FR-20.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutMap {
    layout: LayoutId,
    /// `KEY_SLOTS * MOD_COMBINATIONS` entries, indexed by [`forward_index`].
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
    /// `extended` is the flag of FR-05, `true` for the keypad and the other `E0` keys. It is
    /// part of the key, not a modifier: without it the keypad `/` and the `/?` key of the
    /// main block would be the same coordinate, which is the defect task T-02-1a repaired.
    ///
    /// One index computation and one array read: no allocation (NFR-03), no lock (NFR-04),
    /// no branch that depends on how full the table is (NFR-01), and — by the masking in
    /// [`key_index`] — no input that can leave the table. Safe to call from the hook
    /// callback.
    pub fn lookup(&self, scan: u16, extended: bool, mods: Mods) -> KeyMapping {
        self.forward[forward_index(scan, extended, mods)]
    }

    /// Which key produces `ch` — the backward path of step 5 of FR-61.
    ///
    /// Returns the plainest key that produces the character, and it is plainest in two
    /// senses, because the index is filled in the order of [`key_index`] and then of
    /// [`Mods::ALL`] and the first writer of a character keeps the slot: the main block
    /// before the keypad, and the fewest modifiers before the most. A character carried by
    /// both `/` keys therefore answers with the main block one, which is the key a user
    /// would have pressed and the key task T-07-2 should replay. Dead keys are deliberately
    /// absent from the index: replaying one would start a composition instead of inserting a
    /// character.
    ///
    /// The answer carries the extended flag of FR-05, so a caller can replay the key without
    /// having to guess which block it came from.
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
                    extended: entry.extended,
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
            forward: vec![KeyMapping::EMPTY; KEY_SLOTS * MOD_COMBINATIONS].into_boxed_slice(),
            filled: 0,
        }
    }

    /// Records `mapping` for the key `(scan, extended)` under `mods` and reports whether it
    /// was stored.
    ///
    /// An entry that produces nothing is never stored, and an occupied slot is never
    /// overwritten. Together the two rules settle what remains of the collisions of the
    /// sweep. Since task T-02-1a the keypad no longer collides with the main block wherever
    /// `MAPVK_VK_TO_VSC_EX` reports the `E0` prefix, but Windows withholds that prefix for
    /// the navigation block, so four pairs still share a slot — `VK_DELETE`/`VK_DECIMAL` on
    /// `0x53`, `VK_HOME`/`VK_NUMPAD7` on `0x47`, `VK_END`/`VK_NUMPAD1` on `0x4F`,
    /// `VK_INSERT`/`VK_NUMPAD0` on `0x52`. In each of them the navigation key carries no
    /// character at all, so the rule keeps the keypad reading regardless of the order the
    /// two are offered in, and both layouts of the pair read those keys alike. That makes
    /// the result reproducible, which is what FR-21 needs from a rebuild.
    pub fn set(&mut self, scan: u16, extended: bool, mods: Mods, mapping: KeyMapping) -> bool {
        if mapping.is_empty() {
            return false;
        }
        let index = forward_index(scan, extended, mods);
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
        // Ascending key index, so the whole plain half is walked before the extended one and
        // the main block wins every reverse slot it shares with the keypad.
        for key in 0..KEY_SLOTS {
            for mods in Mods::ALL {
                let mapping = self.forward[key * MOD_COMBINATIONS + mods.index()];
                if mapping.is_empty() || mapping.is_dead() {
                    continue;
                }
                let Some(ch) = mapping.single_char() else {
                    // A ligature of several scalar values has no single character to be
                    // found by, and matching a sequence is the business of the selection
                    // path, not of a constant time index.
                    continue;
                };
                insert_reverse(
                    &mut reverse,
                    ch as u32,
                    (key & 0xFF) as u16,
                    key & EXTENDED_BIT != 0,
                    mods,
                );
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
/// The table holds at most 4096 keys in 8192 slots, so the walk always meets a free slot;
/// the bound on the loop is a guard against a future change to those two numbers, not an
/// expected outcome.
fn insert_reverse(table: &mut [ReverseSlot], key: u32, scan: u16, extended: bool, mods: Mods) {
    let mut slot = reverse_slot(key);
    for _ in 0..REVERSE_CAPACITY {
        if table[slot].key == key {
            return;
        }
        if table[slot].key == REVERSE_FREE {
            table[slot] = ReverseSlot {
                key,
                scan,
                extended,
                mods,
            };
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

/// The physical key that carries `vk` in `layout` as `(scan, extended)`, or `None` if the
/// layout has no key for it — task T-02-1a.
///
/// `MAPVK_VK_TO_VSC_EX` is the only translation this module asks for. It returns the scan
/// code in the low byte and the hardware prefix in the high byte, so one call answers both
/// halves of the cache key; the plain `MAPVK_VK_TO_VSC` returns the low byte alone and is
/// what let the keypad and the main block share a slot.
///
/// Only the `E0` prefix counts as extended, which is precisely the flag the hook reports as
/// `LLKHF_EXTENDED` (FR-05). `E1`, the prefix of `Pause`, is not a hook extended flag and
/// must not be read as one; those keys carry no character in any case.
fn physical_key_of(vk: u32, layout: LayoutId) -> Option<(u16, bool)> {
    // SAFETY: MapVirtualKeyExW takes three values and returns one; it reads nothing through
    // a pointer and writes nothing back, so there is no buffer whose size could be wrong.
    // The handle came from GetKeyboardLayoutList and is a layout loaded in this session, and
    // an unloaded or invalid handle would only make the call return `0`, which is checked
    // below and reported as "no such key" (NFR-13).
    let translated = unsafe { MapVirtualKeyExW(vk, MAPVK_VK_TO_VSC_EX, Some(layout.hkl())) };
    let scan = (translated & 0xFF) as u16;
    if scan == 0 {
        // The layout has no physical key for this virtual key. Nothing to record, and scan
        // code `0` must not become a bucket that everything falls into.
        return None;
    }
    Some((scan, translated & 0xFF00 == 0xE000))
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
        let Some((scan, extended)) = physical_key_of(vk, layout) else {
            continue;
        };
        for mods in Mods::ALL {
            // `ToUnicodeEx` is given the plain scan code: its `wScanCode` parameter is a
            // hardware scan code whose only other meaningful bit is the key-up bit, and the
            // prefix travels in `extended` instead.
            builder.set(
                scan,
                extended,
                mods,
                decode(vk, scan, &states[mods.index()], layout),
            );
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

    /// Copies the layouts the cache covers into `out`, in system order, and returns how many.
    ///
    /// The participating set of FR-35 as the *live cache* holds it, which is what the choice of
    /// section 4.4 is made against: [`LayoutCache::build`] filled it from [`enumerate`], so the
    /// IMEs are already gone and the order is the system's own — the order FR-30 means by
    /// "первые две раскладки системного списка".
    ///
    /// Allocation-free, and that is the point (NFR-09): this is read on the hotkey path, where
    /// [`crate::inject::on_hotkey`] hands it a fixed array of [`MAX_CYCLE`] elements. `out`
    /// shorter than the cache is not an error — the first `out.len()` layouts are copied, which
    /// is what a caller with a bounded list can use anyway.
    pub fn layouts(&self, out: &mut [LayoutId]) -> usize {
        let taken = out.len().min(self.maps.len());

        for (slot, map) in out.iter_mut().zip(&self.maps) {
            *slot = map.layout;
        }

        taken
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

    /// Whether **any** layout of the cache puts a character on this physical key — task T-52-1.
    ///
    /// The second half of the emptiness test of FR-10: a stroke is empty only when no layout
    /// the session has can write with the key. The first half — "the active layout wrote
    /// nothing" — is [`LayoutMap::lookup`] through [`crate::buffer::Recorder::lookup`], and the
    /// gate that keeps this function off the ordinary path is there: it is asked **only** after
    /// that lookup came back empty, so a letter pays nothing for it at all.
    ///
    /// # Why the question is about every layout and not about the active one
    ///
    /// FR-22 converts by the **scan code**, not by the character: a key blank in the layout the
    /// user is typing in and significant in the one they meant to be typing in is exactly the
    /// stroke FR-23 exists to carry over. Flushing it would contradict FR-23 from the other
    /// side, and the two requirements would each be right on their own.
    ///
    /// A **dead key** of FR-24 is not empty and never reaches this function: `ToUnicodeEx`
    /// answers it with the dead character itself, so [`KeyMapping::is_empty`] is already false
    /// for the active layout. Here the same rule applies to the other layouts — a key that is
    /// dead somewhere is a key that writes somewhere.
    ///
    /// # NFR-01 to NFR-05
    ///
    /// One [`LayoutMap::lookup`] — an index computation and an array read — per layout of the
    /// session, stopping at the first that answers. The cache holds one map per installed
    /// layout without an IME, which is two on the machine this was written on and a handful on
    /// any machine. No allocation (NFR-03), no lock (NFR-04), no I/O (NFR-05) and no Win32 call
    /// of any kind (NFR-02): safe from inside the hook callback, which is where it is called.
    pub fn produces_in_any_map(&self, scan: u16, extended: bool, mods: Mods) -> bool {
        self.maps
            .iter()
            .any(|map| !map.lookup(scan, extended, mods).is_empty())
    }
}

// ---------------------------------------------------------------------------------------
// Choosing the target layout — section 4.4, FR-30 to FR-35, task T-05-2
// ---------------------------------------------------------------------------------------

/// Longest list of layouts one press of the hotkey steps along.
///
/// Section 7 puts no bound on `cycle`, and a hand-edited file must not be able to turn into an
/// unbounded array on the hotkey path, exactly as `[buffer] capacity` must not — see
/// [`crate::buffer::MAX_CAPACITY`], which answers the same question for the same reason. Eight
/// is more layouts than a Windows session is ever set up with; decision 19 fixes this machine at
/// two. Entries past the eighth are dropped, not an error.
pub const MAX_CYCLE: usize = 8;

/// A layout identifier **as section 7 writes it** — `"0x00000409"`, not an `HKL`.
///
/// The distinction is load-bearing and it is why this is a type of its own rather than a
/// [`LayoutId`]. The value section 7 prints, `0x00000409`, is the *keyboard layout identifier*:
/// the language in the low word and zero in the high word. The handle the OS hands out for the
/// very same layout is `0x04090409`, the language repeated in both words, and a layout loaded
/// twice, or loaded under an explicit layout id, carries something else again in the high word.
/// Comparing the two for equality would therefore never match anything, and the configuration
/// of section 7 — the one decision 19 confirms and the one shipped as the default — would
/// silently select nothing at all.
///
/// So the rule is stated once, in [`LayoutSpec::matches`], and applied everywhere: a spec whose
/// high word is zero names a **language** and matches any loaded layout serving it; a spec with
/// a high word names a **handle** and matches only that one. Both forms are accepted from the
/// file, and section 7 is not extended by one field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayoutSpec(usize);

impl LayoutSpec {
    /// The spec that names nothing — an absent or unreadable field of section 7.
    pub const NONE: Self = Self(0);

    /// Reads one field of `[layouts]`, in either of the two forms section 7 allows.
    ///
    /// Hexadecimal in both cases, with or without the `0x` prefix, because that is what a
    /// keyboard layout identifier is on Windows: the value in the registry under
    /// `HKLM\SYSTEM\CurrentControlSet\Control\Keyboard Layouts` is the eight hexadecimal digits
    /// `00000409`, and section 7 prints it with the prefix. A field this cannot read is
    /// [`LayoutSpec::NONE`] rather than a failure: a typo in a hand-edited file must leave a
    /// resident program working off the defaults, and the settings dialog of FR-92 (task
    /// T-08-1) is where a bad value is reported to the user.
    ///
    /// Allocation-free — it is called on the UI thread while publishing, and there is nothing
    /// here that needs a string of its own.
    pub fn parse(text: &str) -> Self {
        let trimmed = text.trim();
        let digits = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .unwrap_or(trimmed);

        match usize::from_str_radix(digits, 16) {
            Ok(value) => Self(value),
            Err(_) => Self::NONE,
        }
    }

    /// Wraps a raw identifier — the form the published atomics carry.
    pub const fn from_raw(raw: usize) -> Self {
        Self(raw)
    }

    /// The raw identifier.
    pub const fn raw(self) -> usize {
        self.0
    }

    /// Whether this spec names nothing.
    pub const fn is_none(self) -> bool {
        self.0 == 0
    }

    /// Whether this spec names `layout` — the one rule, stated once.
    pub const fn matches(self, layout: LayoutId) -> bool {
        if self.is_none() {
            return false;
        }

        if self.0 >> 16 == 0 {
            // A keyboard layout identifier: the language alone. `0x00000409` names US whichever
            // handle the session gave it.
            return layout.language_id() as usize == self.0 & 0xFFFF;
        }

        // A handle, written out in full. Compared exactly, so that a session with the same
        // language loaded twice can still be told apart by a user who writes both handles down.
        layout.raw() == self.0
    }

    /// The layout of `available` this spec names, if the session has one.
    pub fn resolve(self, available: &[LayoutId]) -> Option<LayoutId> {
        available
            .iter()
            .copied()
            .find(|&layout| self.matches(layout))
    }
}

/// Why a target layout could not be chosen.
///
/// Every variant is a **refusal**: the caller performs no replacement and no switch. FR-35 asks
/// for exactly that — "обработать как отказ и сосчитать, а не переключаться" — and the counting
/// is [`selection_failures`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionError {
    /// **FR-35.** A layout served by a TSF/IME text service turned up in the pair or in the
    /// cycle list.
    ///
    /// It cannot get there from the live cache: [`enumerate`] drops IMEs before a map is ever
    /// built. So it is either a caller passing a list this module did not produce or a
    /// `config.toml` naming one by hand, and both are defects rather than requests. The
    /// composition step of an IME breaks the correspondence between keystrokes and the field's
    /// contents, which is the whole reason FR-35 exists, so the answer is to do nothing.
    ImeLayout,
    /// Fewer than two layouts to walk: there is nothing to convert *into*.
    ///
    /// A session with one layout, or a cycle list that resolved to one layout or to none of the
    /// layouts this session has.
    NoLayouts,
    /// The layout the strokes were typed under is not in the list — FR-30, "остальные
    /// раскладки игнорируются".
    ///
    /// With three layouts and mode `pair` the user names two of them, and text typed under the
    /// third is not this program's business: converting it would mean guessing a direction the
    /// user did not give.
    OriginOutside,
    /// ⭐ **Task Т-22-5.** Mode `pair`, the configured pair resolved against nothing — and the
    /// list it was resolved against was **not the whole session**: it had been cut short at
    /// [`MAX_CYCLE`] by the caller's fixed array.
    ///
    /// The prefill of FR-30 is the answer when the session really does not have the layouts the
    /// pair names. It is **not** an answer when this program simply did not look at them: the two
    /// are the same slice and opposite facts, and guessing between them is how a user with nine
    /// layouts got a silent conversion between the first two of them instead of the pair they
    /// configured — with the settings dialog showing that pair as perfectly valid, because the
    /// dialog checks against the full list.
    ///
    /// So the answer is a refusal that is counted like every other, and the press changes nothing
    /// at all. [`MAX_CYCLE`] is not widened by this: eight is still more layouts than a session is
    /// ever set up with, and a bound that a hand-edited file cannot turn into an unbounded array
    /// on the hotkey path is the reason it exists.
    TooManyLayouts,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Names of outcomes; no character, no scan code, no layout handle — SEC-01, SEC-07.
        let text = match self {
            Self::ImeLayout => "an IME layout was named as a participant and FR-35 excludes it",
            Self::NoLayouts => "fewer than two layouts to switch between",
            Self::OriginOutside => "the layout in use is not one of the participants",
            Self::TooManyLayouts => {
                "the configured pair names no layout among the first eight of this session"
            }
        };
        f.write_str(text)
    }
}

impl std::error::Error for SelectionError {}

/// How many times each refusal of [`SelectionError`] has been answered — the counting half of
/// FR-35.
///
/// Counts of events, never a keystroke and never a character: the same shape as
/// [`crate::switch::Failures`], and offered to the diagnostic journal of task T-06-4 on the same
/// terms (SEC-01, SEC-07).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionFailures {
    /// [`SelectionError::ImeLayout`] — FR-35.
    pub ime_layout: u32,
    /// [`SelectionError::NoLayouts`].
    pub no_layouts: u32,
    /// [`SelectionError::OriginOutside`].
    pub origin_outside: u32,
    /// [`SelectionError::TooManyLayouts`] — task Т-22-5.
    pub too_many_layouts: u32,
}

static IME_LAYOUT: AtomicU32 = AtomicU32::new(0);
static NO_LAYOUTS: AtomicU32 = AtomicU32::new(0);
static ORIGIN_OUTSIDE: AtomicU32 = AtomicU32::new(0);
static TOO_MANY_LAYOUTS: AtomicU32 = AtomicU32::new(0);

/// Counts one refusal and hands it back, so that a refusal cannot be produced without being
/// counted.
///
/// The one place any of the four counters is raised, and it is reached from the three places a
/// [`SelectionError`] is *created* — [`Cycle::from_layouts`], [`Cycle::target`] and, since task
/// Т-22-5, the pair branch of [`cycle_for`]. Everything above them propagates with `?` and counts
/// nothing, which is what keeps one refusal from being counted twice.
///
/// Relaxed: these are counters and nothing is ordered against them (NFR-04 — no lock anywhere on
/// this path).
fn refuse(error: SelectionError) -> SelectionError {
    let counter = match error {
        SelectionError::ImeLayout => &IME_LAYOUT,
        SelectionError::NoLayouts => &NO_LAYOUTS,
        SelectionError::OriginOutside => &ORIGIN_OUTSIDE,
        SelectionError::TooManyLayouts => &TOO_MANY_LAYOUTS,
    };

    counter.fetch_add(1, Ordering::Relaxed);

    error
}

/// What the four counters stand at.
pub fn selection_failures() -> SelectionFailures {
    SelectionFailures {
        ime_layout: IME_LAYOUT.load(Ordering::Relaxed),
        no_layouts: NO_LAYOUTS.load(Ordering::Relaxed),
        origin_outside: ORIGIN_OUTSIDE.load(Ordering::Relaxed),
        too_many_layouts: TOO_MANY_LAYOUTS.load(Ordering::Relaxed),
    }
}

/// Zeroes the four counters. For tests; the product never calls it.
pub fn reset_selection_failures() {
    IME_LAYOUT.store(0, Ordering::Relaxed);
    NO_LAYOUTS.store(0, Ordering::Relaxed);
    ORIGIN_OUTSIDE.store(0, Ordering::Relaxed);
    TOO_MANY_LAYOUTS.store(0, Ordering::Relaxed);
}

/// The ordered list of layouts one press of the hotkey steps along — **FR-30, FR-31, FR-33**.
///
/// # Why there is one type here and not two
///
/// FR-33: "В режиме «Пара» механизм тот же при длине цикла 2, благодаря чему повторное нажатие
/// горячей клавиши работает как откат **без отдельной реализации**." That is a statement about
/// how the code is built, not a remark about how it behaves, and two branches that happened to
/// agree would not satisfy it — they would part company at the first change to either.
///
/// So "Пара" is not a mode of this type. It is a [`Cycle`] whose list has two elements, produced
/// by the same [`cycle_for`] out of a different part of section 7, and stepped along by the same
/// [`Cycle::target`]. The whole difference between the two modes of FR-30 and FR-31 is **which
/// layouts end up in the list**, and it is spent before this type exists.
///
/// Fixed size and `Copy`: building one allocates nothing (NFR-03), and it is built on the hotkey
/// path, where NFR-09 gives the whole replacement thirty milliseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cycle {
    slots: [LayoutId; MAX_CYCLE],
    len: u8,
}

impl Cycle {
    /// A cycle with nothing in it. Answers [`SelectionError::NoLayouts`] to everything.
    pub const EMPTY: Self = Self {
        slots: [LayoutId::from_raw(0); MAX_CYCLE],
        len: 0,
    };

    /// The cycle `layouts` names: in order, without repetitions, clamped to [`MAX_CYCLE`].
    ///
    /// The one constructor, and therefore the one place FR-35 is enforced: an IME anywhere in
    /// the list refuses the whole selection and is counted. Refusing rather than skipping is
    /// what FR-35 asks for — a list that names an IME is a defect of the caller or a damaged
    /// `config.toml`, and quietly switching to something else instead would hide it.
    ///
    /// Repetitions are dropped rather than refused: a list naming the same layout twice is a
    /// cycle of the layouts it names, and the position counter must not sit still for a press.
    /// A list that has fewer than two distinct layouts left is [`SelectionError::NoLayouts`].
    pub fn from_layouts(layouts: &[LayoutId]) -> Result<Self, SelectionError> {
        let mut cycle = Self::EMPTY;

        for &layout in layouts {
            if layout.is_ime() {
                return Err(refuse(SelectionError::ImeLayout));
            }

            if usize::from(cycle.len) == MAX_CYCLE || cycle.contains(layout) {
                continue;
            }

            cycle.slots[usize::from(cycle.len)] = layout;
            cycle.len += 1;
        }

        if cycle.len < 2 {
            return Err(refuse(SelectionError::NoLayouts));
        }

        Ok(cycle)
    }

    /// The layouts in the order the hotkey walks them.
    pub fn layouts(&self) -> &[LayoutId] {
        &self.slots[..usize::from(self.len)]
    }

    /// How many layouts the cycle walks. Two in mode `pair` — FR-33.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether the cycle walks nothing.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether `layout` takes part.
    pub fn contains(&self, layout: LayoutId) -> bool {
        self.layouts().contains(&layout)
    }

    /// Where `layout` sits in the cycle.
    pub fn position_of(&self, layout: LayoutId) -> Option<usize> {
        self.layouts().iter().position(|&entry| entry == layout)
    }

    /// **The layout `step` presses along the cycle from `origin` — FR-31, FR-32, FR-33.**
    ///
    /// The whole of the cycling, in one expression, for both modes:
    ///
    /// * `step` is the position counter of FR-32, which module `buffer` keeps beside the ring
    ///   and resets with it (FR-34). It counts *presses of the hotkey against this buffer*, and
    ///   `0` is "as typed";
    /// * `origin` is the layout the strokes were recorded under — FR-26 decides the direction
    ///   from the recorded `hkl` and never from the text;
    /// * the answer is the layout at `origin + step`, taken around the ring of participants.
    ///
    /// FR-31 falls out of it: with the list `[A, B, C]` and the text typed under `A`, the first
    /// press answers `B`, the second `C` and the third `A` — "по кругу с возвратом к исходному
    /// варианту". So does FR-33: with `[A, B]` the first press answers `B` and the second `A`,
    /// which is the rollback, and it is this same line that produces it.
    ///
    /// **And so does FR-32.** The answer for a step that is a multiple of the length is `origin`
    /// itself, and rendering the *original* strokes into the layout they were typed under
    /// reproduces what the user typed, code unit for code unit —
    /// [`crate::convert::convert_strokes`] asks the target layout what the same physical key
    /// gives, and for the original layout that is the character the stroke already carries.
    /// Nothing accumulates, because nothing is ever converted from a conversion: the buffer is
    /// never written back.
    ///
    /// The reduction is `step % len` before the addition, so a counter that has been running for
    /// a long time cannot overflow the sum.
    pub fn target(&self, origin: LayoutId, step: usize) -> Result<LayoutId, SelectionError> {
        let len = self.len();

        if len < 2 {
            return Err(refuse(SelectionError::NoLayouts));
        }

        let Some(position) = self.position_of(origin) else {
            return Err(refuse(SelectionError::OriginOutside));
        };

        Ok(self.slots[(position + step % len) % len])
    }
}

/// `[layouts]` of section 7 as the input thread reads it — the published form.
///
/// Plain `Copy` data of fixed size: the hotkey path reads it out of atomics with no lock, no
/// allocation and no file (NFR-04, NFR-03, NFR-09). What it carries is what section 7 carries
/// and nothing besides — the mode, the two fields of the pair and the cycle list — because the
/// schema of section 7 is closed and this task does not extend it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Configured {
    mode: LayoutMode,
    pair: [LayoutSpec; 2],
    cycle: [LayoutSpec; MAX_CYCLE],
    cycle_len: u8,
}

impl Default for Configured {
    /// What the program uses before anything has been published.
    ///
    /// Mode `pair`, as section 7 and decision 19 have it, and **no** identifiers at all — which
    /// is not the same as the identifiers of section 7 written out here a second time. On the
    /// two-layout machine of decision 19 the first bullet of FR-30 answers without consulting a
    /// field, and with three layouts an unpublished configuration falls back to the first two of
    /// the system list, which is precisely the prefill rule the second bullet gives. Repeating
    /// `0x00000409` here would put the defaults of section 7 in a second place, where they could
    /// drift from `settings::Layouts::default`.
    fn default() -> Self {
        Self {
            mode: LayoutMode::Pair,
            pair: [LayoutSpec::NONE; 2],
            cycle: [LayoutSpec::NONE; MAX_CYCLE],
            cycle_len: 0,
        }
    }
}

impl Configured {
    /// Reads `[layouts]` of a loaded configuration. Called on the UI thread, which is the thread
    /// section 6.1 lets touch a file.
    pub fn from_settings(layouts: &settings::Layouts) -> Self {
        let mut configured = Self {
            mode: layouts.mode,
            pair: [
                LayoutSpec::parse(&layouts.pair_source),
                LayoutSpec::parse(&layouts.pair_target),
            ],
            ..Self::default()
        };

        for spec in layouts.cycle.iter().take(MAX_CYCLE) {
            configured.cycle[usize::from(configured.cycle_len)] = LayoutSpec::parse(spec);
            configured.cycle_len += 1;
        }

        configured
    }

    /// The mode of FR-30 and FR-31.
    pub const fn mode(self) -> LayoutMode {
        self.mode
    }

    /// `pair_source` and `pair_target`, in that order.
    pub const fn pair(&self) -> &[LayoutSpec; 2] {
        &self.pair
    }

    /// The `cycle` list, as long as the file made it.
    pub fn cycle(&self) -> &[LayoutSpec] {
        &self.cycle[..usize::from(self.cycle_len)]
    }
}

/// The published `[layouts]`, one atomic per value and all of them under one generation —
/// section 6.3.
///
/// The configuration belongs to the UI thread and the choice is made on the input thread, so the
/// values are *published* rather than fetched, exactly as `[replacement]` is (task T-04-2). Plain
/// atomics and no lock: NFR-04 forbids blocking primitives near the input path, and the cheapest
/// way to obey a ban is to have nothing to ban. What makes the four of them **one** published
/// value rather than four independent ones is [`GENERATION`].
static MODE: AtomicU8 = AtomicU8::new(PAIR_CODE);
static PAIR: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
static CYCLE: [AtomicUsize; MAX_CYCLE] = [const { AtomicUsize::new(0) }; MAX_CYCLE];
static CYCLE_LEN: AtomicUsize = AtomicUsize::new(0);

/// **Odd while [`publish`] is writing, even while what stands above is one configuration.**
///
/// Every cell above is an atomic, so no choice of orderings could make this a data race; what the
/// counter guards against is narrower and real, and the audit of 2026-08-24 named it — «публикация
/// `[layouts]` в поток ввода допускает рваное чтение: россыпь атомиков без поколения, в отличие от
/// seqlock у guard». A press of the hotkey that arrived while the settings dialog was applying
/// could take `PAIR[0]` from the configuration that was standing and `PAIR[1]` from the one being
/// written, and then convert into a pair nobody ever configured. **Ordering the stores does not
/// answer that**, because ordering does not make a *read* atomic: it fixes the sequence the writer
/// is seen to move in, not the instant the reader takes its several samples in.
///
/// The mechanism is the seqlock of [`crate::guard::publish_exclusions`], which solves the same
/// problem for the `[exclusions]` table: the writer makes the counter odd for the whole of the
/// write and even again after it, the reader keeps its answer only if the counter did not move and
/// was even, and a moved counter costs a re-read rather than a wait. Section 6.3 names two
/// mechanisms for publishing the configuration — an `arc_swap`-like pointer, or `PostMessage` —
/// and this is neither; what it is, is the discipline the module next door already applies, so
/// that the two published tables of section 7 are protected the same way rather than one of them
/// only.
///
/// # The barriers are Boehm's, not the ones that read naturally
///
/// Hans-J. Boehm, *"Can seqlocks get along with programming language memory models?"*, MSPC 2012.
/// The canon that paper settles, and the form every seqlock in this program is to be written in:
///
/// * the **reader** loads the counter, reads the fields with `Relaxed` loads, then executes
///   `fence(Acquire)` **before** the control load of the counter, which may itself be `Relaxed`.
///   Putting `Acquire` *on* the control load instead is the trap the paper is about: acquire on a
///   load orders the operations that come **after** it, and what has to be pinned here is the
///   field loads that came **before** — nothing stops them from being sunk past an acquire load;
/// * the **writer** executes `fence(Release)` between making the counter odd and storing the
///   fields, so that no store can be hoisted above the odd value that announces it, and closes
///   with a `Release` bump, which is what publishes the fields to a reader that sees the even
///   value.
///
/// Task T-13-16 of this stage brings the two seqlock readers that predate this one —
/// [`crate::diag`] and [`crate::guard`] — to the same canon. This one is written in it from the
/// start rather than copied from their present shape: the shape is the model, the deviation is
/// not.
static GENERATION: AtomicU32 = AtomicU32::new(0);

/// How many times [`published`] re-reads a configuration that moved under it.
///
/// Three, and the **bound** is the point rather than the number. This is read on the input thread,
/// where the `WH_KEYBOARD_LL` callback of section 6.1 lives; a thread that spins there stops
/// delivering keystrokes to the whole session, and NFR-01 gives the callback a hundred
/// microseconds at p99 with NFR-02 an absolute millisecond over it. An unbounded seqlock retry is
/// a wait by another name, and a wait here is the blocking primitive NFR-04 forbids.
///
/// **Why so few rounds are enough** — it is a property of the writer, not a hope about timing.
/// [`publish`] runs on the UI thread and the window in which the counter is odd is at most twelve
/// plain stores: no loop bounded by anything a user can put in the file, no allocation, no
/// syscall, no file, no lock. So the window cannot be *stretched* — not by a long cycle list, not
/// by a slow disk, not by another thread holding something. For three attempts to lose in a row
/// the UI thread would have to be preempted inside those twelve stores and then rescheduled back
/// into them twice more, while one press of the hotkey spends microseconds here.
///
/// **And the worst outcome of losing all three is not a failure.** [`published`] then returns the
/// last thing it read, which is one press served with a mixed configuration — precisely what every
/// press got before this counter existed, and precisely the impact the audit measured («худший
/// исход — одна конвертация в неверную раскладку в момент нажатия «Применить»»). The retry can
/// only improve that answer and can never spoil it, which is what makes a small bound the right
/// trade on this thread.
const PUBLISHED_READ_ATTEMPTS: u32 = 3;

/// `mode = "pair"` as one byte.
const PAIR_CODE: u8 = 0;
/// `mode = "cycle"` as one byte.
const CYCLE_CODE: u8 = 1;

/// Publishes `[layouts]` to the input thread — the only writer.
///
/// Called by `app` after the configuration has been read, on the UI thread, together with the
/// other published values of section 7. Nothing is applied to anything already built: the next
/// press of the hotkey reads whatever stands here at that moment.
///
/// # The generation
///
/// The whole of the write sits between an odd [`GENERATION`] and an even one, which is what makes
/// the four groups of atomics one value to [`published`]. There is exactly one writer — the UI
/// thread — so the two bumps need no compare-and-swap; `fetch_add` is used because that is what
/// [`crate::guard::publish_exclusions`] uses for the same counter and the two are meant to read
/// alike.
///
/// The field stores are `Relaxed` where they were `Release` before, and **nothing is weakened by
/// that**: the single `fence(Release)` above them does for all twelve what twelve release stores
/// did one at a time, and the closing `Release` bump is what carries them to a reader that sees
/// the even value. See [`GENERATION`] for why the fence and not an ordering on the bump itself.
pub fn publish(configured: Configured) {
    // Odd for the whole of the write — see `GENERATION`.
    GENERATION.fetch_add(1, Ordering::Relaxed);
    fence(Ordering::Release);

    for (slot, spec) in PAIR.iter().zip(configured.pair()) {
        slot.store(spec.raw(), Ordering::Relaxed);
    }

    for (slot, spec) in CYCLE.iter().zip(configured.cycle()) {
        slot.store(spec.raw(), Ordering::Relaxed);
    }

    // The length is stored after the values it bounds, so a reader that sees the new length sees
    // the entries that go with it; and the mode last, because it is what decides which of the two
    // groups is read at all. The order is kept exactly as it was written — module `guard`
    // publishes its own list in this same order and says so — and it is now the second line of
    // the defence rather than the first: it is what keeps the answer of an exhausted retry the
    // mildest one there is, a length that never outruns the values it describes.
    CYCLE_LEN.store(configured.cycle().len(), Ordering::Relaxed);
    MODE.store(
        match configured.mode() {
            LayoutMode::Pair => PAIR_CODE,
            LayoutMode::Cycle => CYCLE_CODE,
        },
        Ordering::Relaxed,
    );

    // Even again: what stands here is a configuration, not one somebody is in the middle of
    // writing.
    GENERATION.fetch_add(1, Ordering::Release);
}

/// One pass over the published atomics, without the generation check around it.
///
/// Split out so that [`published`] reads as the retry it is, the way `guard::search_published` is
/// split out of `guard::is_excluded_name`. Every load is `Relaxed`: what orders them is the pair
/// of fences [`published`] puts around this call — the canon of [`GENERATION`].
///
/// [`MAX_CYCLE`] plus four atomic loads at worst — the mode, the two halves of the pair, the
/// length, and one per cycle entry the length admits.
fn read_published() -> Configured {
    let mut configured = Configured {
        mode: match MODE.load(Ordering::Relaxed) {
            CYCLE_CODE => LayoutMode::Cycle,
            // Nothing but `publish` writes the byte and it writes only the two codes, so this is
            // unreachable — and the hotkey path must have a mode rather than a question even so,
            // which is what makes the fallback the documented default of section 7.
            _ => LayoutMode::Pair,
        },
        ..Configured::default()
    };

    for (spec, slot) in configured.pair.iter_mut().zip(&PAIR) {
        *spec = LayoutSpec::from_raw(slot.load(Ordering::Relaxed));

        // The seam of the interleaving, and it stands **after** the load on purpose: a publication
        // driven in here lands between the two halves of the pair, which is the mixture the audit
        // described. Absent from the shipping build — see `seam`.
        #[cfg(feature = "testing")]
        seam::interleave();
    }

    let len = CYCLE_LEN.load(Ordering::Relaxed).min(MAX_CYCLE);
    for (spec, slot) in configured.cycle.iter_mut().zip(&CYCLE).take(len) {
        *spec = LayoutSpec::from_raw(slot.load(Ordering::Relaxed));
    }
    configured.cycle_len = len as u8;

    configured
}

/// What was published — read once per press of the hotkey.
///
/// One pass is [`MAX_CYCLE`] plus **six** atomic loads and one `fence`, and nothing else: no
/// allocation (NFR-03), no lock (NFR-04), no file (NFR-05, NFR-09). The six are the four
/// [`read_published`] takes — the mode, the two halves of the pair and the length — plus the two
/// loads of [`GENERATION`] this function adds around them. (The line here used to say "plus three"
/// and was one short of its own code even then: `CYCLE_LEN` was never in the count.)
///
/// At most [`PUBLISHED_READ_ATTEMPTS`] passes, and the second and third are reached only by a
/// press that landed inside a publication. Nothing is retried in a loop that a writer could keep
/// alive — see [`PUBLISHED_READ_ATTEMPTS`] for why a bound this small is safe and what the answer
/// is when it runs out.
///
/// # The shape of the loop
///
/// The fields are read on **every** attempt, before the counter is judged, so that the answer to
/// an exhausted retry is literally "the last thing this read" and never a value nobody published.
/// That costs one wasted pass in the case where the counter was already odd when the attempt
/// began, and buys the property that this function has no answer of its own to invent.
pub fn published() -> Configured {
    let mut configured = Configured::default();

    for _ in 0..PUBLISHED_READ_ATTEMPTS {
        let before = GENERATION.load(Ordering::Acquire);

        configured = read_published();

        // Boehm's fence, and it stands **before** the control load rather than inside it — see
        // `GENERATION`. This is what keeps the field loads above from being sunk below the check
        // that is supposed to vouch for them.
        fence(Ordering::Acquire);

        // Even, and unmoved: what was read was one configuration, and the answer stands.
        if before.is_multiple_of(2) && GENERATION.load(Ordering::Relaxed) == before {
            return configured;
        }
    }

    // A publication in flight through every attempt. The answer is the last pass, which is the
    // torn read this counter exists to avoid — and is exactly the answer every press got before it
    // existed. See `PUBLISHED_READ_ATTEMPTS`.
    configured
}

/// Drives one whole publication into the middle of one read — **compiled only under `testing`**.
///
/// A generation cannot be tested by racing two threads at it: the collision either happens on the
/// day the test runs or it does not, and a green run proves nothing either way. This seam turns
/// the collision into an appointment. [`read_published`] calls [`interleave`] after each of its
/// two `PAIR` loads; a test arms a `fn()` that publishes a second configuration, and that whole
/// publication then lands **between** the two halves of the pair — the interleaving the audit of
/// 2026-08-24 described, on demand and in one thread.
///
/// What the seam can prove is therefore [`GENERATION`] and nothing else. Without the counter the
/// read comes back half from one configuration and half from the other; with it the first pass is
/// thrown away and the second comes back whole.
///
/// The gate is SEC-04a's, the one [`crate::control`] and the fault injectors of [`crate::hook`]
/// carry: the `testing` feature is absent from the Release configuration, so the shipping build
/// has neither the arming function nor the load that checks it. Nothing in here is `unsafe` and
/// nothing in here waits — `OnceLock::get` is a load, not a lock (NFR-04).
///
/// [`interleave`]: seam::interleave
#[cfg(feature = "testing")]
mod seam {
    use core::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;

    /// The publication to run, set at most once in the life of a process.
    static PUBLICATION: OnceLock<fn()> = OnceLock::new();

    /// Whether the armed publication is still owed a run. One shot: [`interleave`] takes it.
    static ARMED: AtomicBool = AtomicBool::new(false);

    /// Arms `publication` to run **once**, inside the next read of the published atomics.
    ///
    /// The first caller in a process decides which function that is; a later call with a different
    /// one re-arms the first, which is why the one test that uses this holds a turn that keeps it
    /// alone with the published atomics.
    pub fn arm(publication: fn()) {
        let _ = PUBLICATION.set(publication);
        ARMED.store(true, Ordering::Release);
    }

    /// Runs the armed publication if one is owed, and disarms it.
    ///
    /// The unarmed path — every call from every other test and from the bench — is one relaxed
    /// load and a return.
    pub fn interleave() {
        if !ARMED.load(Ordering::Relaxed) {
            return;
        }

        if !ARMED.swap(false, Ordering::AcqRel) {
            return;
        }

        if let Some(publication) = PUBLICATION.get() {
            publication();
        }
    }
}

/// Arms `publication` to run once **inside** the next [`published`], between the two halves of the
/// pair — the test seam of the interleaving. See the module `seam` this comes from.
#[cfg(feature = "testing")]
pub use seam::arm as interleave_next_read;

/// Whether the list handed to [`cycle_for`] is the whole session — task **Т-22-5**.
///
/// [`LayoutCache::layouts`] copies into the caller's fixed array of [`MAX_CYCLE`] elements and
/// answers how many it copied. A session with more layouts than that leaves the rest out, and the
/// slice that comes back cannot say so: it looks exactly like a whole session of eight. This type
/// is the caller saying which of the two it handed over, and it is a named value rather than a
/// `bool` so that no call site can be read as the opposite of what it is.
///
/// The two product call sites — `inject::take_press` and `selection::plan_for_press` — build it by
/// comparing what `layouts` copied with [`LayoutCache::len`], which is the whole session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Session {
    /// Every layout the session has is in the list.
    #[default]
    Whole,
    /// The session has more layouts than [`MAX_CYCLE`], and the list is the first [`MAX_CYCLE`] of
    /// them in system order.
    Truncated,
}

impl Session {
    /// What a caller that copied `taken` layouts out of a cache holding `total` handed over.
    #[must_use]
    pub const fn of(taken: usize, total: usize) -> Self {
        if taken < total {
            Self::Truncated
        } else {
            Self::Whole
        }
    }
}

/// **The list the hotkey walks, for this configuration and this session — FR-30, FR-31, FR-33.**
///
/// The one function both modes go through, and the only place the two of them differ at all:
/// what ends up in the list. Below this line there is no mode any more — [`Cycle::target`] steps
/// along whatever came out, and a pair is a list of two.
///
/// On the hotkey path `inject::take_press` calls this and then [`Cycle::target`], in that order
/// and with nothing between them; it needs [`Cycle::len`] as well, which is why the two steps are
/// not wrapped into one call. There is no second door: a wrapper that existed here until task
/// T-13-21 was called by no product code and by one test, and its documentation had already
/// drifted into naming a caller it did not have (audit of 2026-08-24).
///
/// # Mode `pair`, and the two bullets of FR-30
///
/// * **Exactly two layouts in the session.** "Работа полностью автоматическая, настройка не
///   требуется: целевая раскладка есть вторая из двух." The configuration is not consulted at
///   all — not even to be overridden — because a machine in the state decision 19 fixes must
///   work with no settings whatever, including with a `config.toml` that names layouts this
///   session does not have.
/// * **Three or more.** The user names the working pair and the rest are ignored. A field that
///   names no layout of this session leaves the pair unusable, and the answer is then the rule
///   FR-30 gives for the dialog — "поля предзаполняются первыми двумя раскладками системного
///   списка" — applied to the choice itself, so that the program keeps working while the user
///   has not opened the settings yet.
///
/// # Mode `cycle` and FR-31
///
/// The configured list, resolved against the session in the order the file gives, which is the
/// order FR-31 walks. Entries naming layouts this session does not have are dropped: a list that
/// mentions a layout the user has since removed is still a list of the others. If fewer than two
/// survive, the answer is the refusal [`SelectionError::NoLayouts`] — **not** the layouts of the
/// session.
///
/// ## Why this branch has no fallback and the one above does
///
/// It had one until the audit of 2026-08-24 ("фолбэк режима «Несколько» шагает по всем раскладкам
/// сессии, включая явно исключённые пользователем"): fewer than two resolved entries walked
/// `available` instead. The list of FR-31 is not a hint, though — FR-92 gives the user a "список
/// раскладок **с галочками участия**", so a layout that is not in it is a layout whose tick was
/// taken off by hand. Section 7 stores only the participants, so a list resolving to one layout
/// cannot be told apart from a list nobody ever wrote, and walking the session in that case walks
/// exactly what the user excluded: with [EN, RU, DE] in the session and [EN, RU] ticked, removing
/// RU from the system made the hotkey convert into DE.
///
/// FR-31 licenses no prefill to fall back to. FR-30 does — "поля предзаполняются первыми двумя
/// раскладками системного списка" — and that sentence is written for the pair and only for the
/// pair, which is the whole of the difference between the two branches here. Refusing costs the
/// user nothing that FR-31 promised: [`SelectionError::NoLayouts`] is a defined answer of this
/// module, it is counted like every other refusal, and the press then changes nothing at all.
///
/// # ⭐ The prefill is not an answer to a question this program did not ask — task Т-22-5
///
/// `session` says whether `available` is the whole session or the first [`MAX_CYCLE`] of a longer
/// one. It is consulted in exactly one place: the prefill above. "The pair names a layout this
/// session does not have" and "the pair names a layout this program did not look at" produce the
/// **same** unresolved slice and are opposite facts, and the prefill of FR-30 is an answer only to
/// the first. Against a list cut short of the session the answer is
/// [`SelectionError::TooManyLayouts`] — counted like every other refusal, and the press then
/// changes nothing at all. See that variant for what the silent substitution cost.
///
/// Allocates nothing: both branches build the list in a fixed array.
pub fn cycle_for(
    configured: Configured,
    available: &[LayoutId],
    session: Session,
) -> Result<Cycle, SelectionError> {
    match configured.mode() {
        LayoutMode::Pair => {
            if available.len() == 2 {
                return Cycle::from_layouts(available);
            }

            let [source, target] = configured.pair();
            match (source.resolve(available), target.resolve(available)) {
                (Some(source), Some(target)) if source != target => {
                    Cycle::from_layouts(&[source, target])
                }
                // ⭐ **Task Т-22-5.** The pair resolved against nothing, and the list it was
                // resolved against is not the session — so "not there" is not a fact this program
                // established. Refuse rather than substitute.
                // ⭐ **Task Т-22-5.** The pair resolved against nothing, and the list it was
                // resolved against is not the session — so "not there" is not a fact this program
                // established. Refuse rather than substitute.
                _ if session == Session::Truncated => Err(refuse(SelectionError::TooManyLayouts)),
                _ => Cycle::from_layouts(available.get(..2).unwrap_or(available)),
            }
        }

        LayoutMode::Cycle => {
            let mut resolved = [LayoutId::from_raw(0); MAX_CYCLE];
            let mut len = 0;

            for spec in configured.cycle() {
                if len == MAX_CYCLE {
                    break;
                }
                if let Some(layout) = spec.resolve(available) {
                    resolved[len] = layout;
                    len += 1;
                }
            }

            // Whatever the ticks of FR-92 left, and nothing besides. Fewer than two of them is
            // `SelectionError::NoLayouts` by the one rule `Cycle::from_layouts` already applies,
            // so the refusal is raised — and counted — in the same place as every other.
            Cycle::from_layouts(&resolved[..len])
        }
    }
}
