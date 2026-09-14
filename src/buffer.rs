//! Ring buffer of key strokes, flush rules, race resolution by timestamps, zeroing of
//! memory.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-04 (the stroke record of section 4.1), FR-05
//! (`LLKHF_EXTENDED` kept in the mask), FR-06 (decoding by `ToUnicodeEx` with the mandatory
//! flag `0x4` — through the cache module `layouts` built with it, see below), FR-07 (a ring
//! of 256 strokes that evicts the oldest), FR-10 (the four LL-hook rows of the flush table),
//! FR-11 (a layout change flushes nothing) and SEC-02 (the memory is overwritten with zeroes,
//! never merely marked empty) — task T-03-2.
//! Task **T-03-3** added FR-12, the resolution of asynchronous flush races against the
//! timestamps this module was already recording: [`Recorder::reset_up_to`] and the wrap-safe
//! comparison [`is_newer_than`] it rests on.
//! FR-13, Raw Input, is **not** here and is not this module's: the module table of section 6.2
//! gives "подписки на системные события" to `watchdog`, which is where task T-03-3 put the
//! `RegisterRawInputDevices` registration and the `WM_INPUT` parsing. What arrives here is the
//! flush that subscription produces, through [`reset_up_to`], exactly as the flush produced by
//! the `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS` subscriptions does.
//! Task **T-05-2** added the position counter of the cycle and the two requirements that live
//! with it: **FR-32**, in the half this module owns — the buffer holds the strokes the user
//! really made and is *never* written back with the result of a conversion, so that every press
//! of the hotkey renders the same original scan codes into the next layout; and **FR-34**, the
//! counter is reset by every rule of FR-10 without exception, which is why it sits in
//! [`Recorder`] beside the ring and is zeroed by the one function that empties it,
//! [`Recorder::clear_ring`]. FR-34 was listed by module `layouts` until this task; the cycle is
//! chosen there and the counter is reset here, and the backlog (decision R-17) puts a
//! requirement where its code is. The same task carried out decision **Р-44**: the exception of
//! FR-11 now covers the last row of the FR-10 table as well — see [`Recorder::record`].
//! Task **T-03-4** added the length mirror of SEC-04a: [`Ring::set_len`] is now the one writer
//! of the ring's length, and under the `testing` feature — and only under it — every write
//! publishes the new length into [`crate::control`], which is the only way a thread other than
//! the input one can learn a number that lives in a thread-local (section 6.3). Nothing else
//! about the buffer changed, and in a build without the feature the mirror does not exist.
//! Task **T-05-2a** gave the position counter of FR-32 the same treatment and for the same
//! reason: [`Recorder::advance_cycle`] and [`Recorder::clear_ring`] publish it into
//! [`crate::control`] under the `testing` feature, so that `cycle_position` on the channel of
//! SEC-04a says where the cycle really is. The counter, the cycle and the choice of the target
//! layout are untouched — the task made the existing behaviour observable, not different.
//! Task **T-69-3** finished that treatment: the counter is written only through
//! `Recorder::set_cycle`, which is the one place it is published from — the construction
//! [`Ring::set_len`] has always had for the length.
//! Implemented by backlog tasks: T-03-2 (done), T-03-2a (done), T-03-3 (done), T-03-4 (done),
//! T-05-2a (done).
//! Task T-03-2a took the physical half of the stroke out of a thread-local of its own and put
//! it into the fields of [`KeyEvent`], where it belonged all along, and wired this module into
//! the running program: `app` installs the buffer, publishes the cache of FR-20 into it and
//! keeps the active layout of FR-04 up to date.
//! NFR-01 to NFR-05 are properties of [`Recorder::record`] and are argued where it is defined.
//!
//! # What is stored, and why it is a scan code
//!
//! Section 4.1 calls it a matter of principle rather than an optimisation, and gives three
//! reasons: independence from the particular pair of layouts, keys whose meaning differs
//! between layouts — `Shift+2` is `@` in English and `"` in Russian — and exact cycling
//! through three layouts and more (section 4.4). A buffer of characters would answer none of
//! them, because the character is the *result* of the key and the layout together, and the
//! conversion of FR-22 needs the key alone.
//!
//! [`Stroke`] therefore carries the physical key, the modifier mask, the layout that was
//! active at the time (FR-26 decides direction from it, and FR-11 is the reason it is stored
//! per stroke rather than once for the buffer), the timestamp FR-12 resolves races against,
//! and what the key produced when it was pressed.
//!
//! # Why decoding is a table read and not a call — FR-06
//!
//! `ToUnicodeEx` is **never called from here**, and the word "never" is load-bearing: this
//! code runs inside the `WH_KEYBOARD_LL` callback, where NFR-01 allows a hundred microseconds
//! at the 99th percentile and NFR-02 one millisecond absolutely, and where a callback that
//! overruns `LowLevelHooksTimeout` has its hook removed by the system **silently** (FR-80).
//! A call into the layout machinery of the OS on every keystroke is exactly the kind of thing
//! that budget cannot hold.
//!
//! FR-06 is satisfied all the same, because the decoding it asks for has already happened:
//! task T-02-1 built [`crate::layouts::LayoutCache`] by calling `ToUnicodeEx` with the
//! mandatory flag `wFlags = 0x4` for every combination of scan code, extended flag and
//! modifiers, and [`crate::layouts::LayoutMap::lookup`] answers from it with one
//! multiplication, one addition and one array read. FR-06 says the decoding must be done by
//! that call with that flag; it does not say it must be done twice.
//!
//! A combination the cache has no answer for is an outcome and not an error: the stroke is
//! stored with no characters and `len` zero, and FR-23 is what decides what becomes of it.
//!
//! ⚠ **What FR-23 does with it is not "carries it through unchanged", and task Т-22-10 corrected
//! this sentence** (finding м3 of the audit of 2026-09-01, which found the same claim in four
//! places). [`crate::convert::convert_stroke`] guards on the **candidate**, not on the source: an
//! empty stroke that is not a dead key is looked up in the target layout, and if that layout has
//! a character on the key the candidate is what comes out. An empty stroke survives conversion
//! unchanged only when the target is silent on that key as well — which is the ordinary case for
//! a key no layout carries anything on, and is why the claim held up for as long as it did.
//!
//! # Threading — section 6.3
//!
//! "Буфер набора записывается исключительно потоком ввода. Читается — потоком ввода при
//! конвертации." So the state lives in a thread-local and there is no mutex, no lock, no
//! atomic and nothing that can block anywhere in this module — NFR-04, and section 6.3 forbids
//! mutexes on the hook path in as many words. [`Recorder`] is a plain owned value; the
//! thread-local layer at the bottom of this file is the only place that knows it is a
//! singleton, and every function there works on the recorder of the **calling** thread, which
//! is what makes the tests of `tests\buffer.rs` independent of each other.
//!
//! # SEC-02 — zeroed, not marked empty
//!
//! "Буфер набора явно перезаписывается нулями при каждом сбросе, а не просто помечается
//! пустым." Every path that frees a slot writes zeroes over it first: the full flush of FR-10,
//! the eviction of FR-07, the single element `Backspace` takes out, and the drop of the buffer
//! itself. A ring that merely moved its indices would leave the user's last sentence sitting
//! in memory behind them, which is the state SEC-02 exists to forbid.
//!
//! The write is [`core::ptr::write_volatile`], and that is the whole answer to "what stops the
//! optimiser from deleting a store nobody reads": a volatile store may not be removed, merged
//! with a neighbouring store, or reordered against another volatile access — it is the one
//! operation in the language whose *presence in the object code* is guaranteed. A plain
//! assignment or a `write_bytes` would be a dead store the moment the slot is overwritten or
//! dropped, and the compiler would be within its rights to drop it. A
//! [`core::sync::atomic::compiler_fence`] follows each write so that the zeroes cannot be
//! sunk past whatever the caller does next. This is the same construction the `zeroize` crate
//! is built on, written out by hand because SEC-03 closes the dependency list.
//!
//! # SEC-01, SEC-07
//!
//! Nothing here is written to a log, a file or a panic message; there is no `println!`, no
//! `format!` and no I/O of any kind. **[`Stroke`] and [`StrokeMods`] deliberately do not
//! derive `Debug`,** and that is not tidiness: a derived `Debug` on this structure would be a
//! ready-made keylogger — one `dbg!`, one `{:?}` in a future panic message or in a future
//! diagnostic line, and the virtual key, the scan code and the decoded characters of whatever
//! the user was typing would be in a string. The trait is absent so that such a line cannot be
//! written by accident; a caller that needs to compare strokes has `PartialEq`, and the tests
//! compare with `==` rather than with `assert_eq!` for exactly this reason.
//!
//! `MappingKind`, `KeyMapping` and `LayoutId` of module `layouts` do derive `Debug`, and that
//! is theirs to answer for; nothing in this module formats them.

use core::ptr;
use core::sync::atomic::{Ordering, compiler_fence};
use std::cell::RefCell;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_BACK, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT,
    VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE,
    VK_MEDIA_PREV_TRACK, VK_MEDIA_STOP, VK_MENU, VK_NEXT, VK_OEM_3, VK_PACKET, VK_PRIOR,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
    VK_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE, VK_VOLUME_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{LLKHF_ALTDOWN, LLKHF_EXTENDED};

use crate::convert::Keystroke;
use crate::hook::{Edge, KeyEvent};
use crate::layouts::{KeyMapping, LayoutCache, LayoutId, MAX_UNITS, Mods};
use crate::settings;

// ---------------------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------------------

/// Capacity FR-07 names, and the default of `[buffer] capacity` in section 7.
///
/// The value is a *default* and not the setting: [`Recorder::with_capacity`] takes whatever
/// the configuration carries. A unit test pins this constant to
/// `settings::Buffer::default().capacity` so the two cannot drift apart.
pub const DEFAULT_CAPACITY: usize = 256;

/// Largest ring this module will build, whatever the configuration says.
///
/// `[buffer] capacity` is a hand-edited number in a file, and a slipped digit must not turn
/// into a multi-megabyte allocation that NFR-06 would then have to answer for. Sixteen times
/// the value of FR-07 is more strokes than a user types between two flushes by a wide margin,
/// and the whole ring is still well under a hundred kilobytes.
pub const MAX_CAPACITY: usize = 4096;

/// How many strokes a buffer asked for `requested` slots really holds.
///
/// The capacity is clamped into `1..=`[`MAX_CAPACITY`], and a configured zero is read as
/// [`DEFAULT_CAPACITY`]: section 7 documents the field as a size, a file saying `0` is a
/// mistake rather than a request for a program that quietly stops working, and there is no one
/// to report it to from here.
///
/// Public because the answer is not the question. `app` publishes the configured capacity to
/// the input thread and re-installs the buffer only when what the configuration asks for
/// differs from what the buffer already has; comparing the *raw* number against
/// [`Recorder::capacity`] would find a difference on every look for any value outside the
/// range above, and would re-install — and so wipe — the buffer over and over.
pub const fn effective_capacity(requested: usize) -> usize {
    match requested {
        0 => DEFAULT_CAPACITY,
        requested if requested > MAX_CAPACITY => MAX_CAPACITY,
        requested => requested,
    }
}

// ---------------------------------------------------------------------------------------
// FR-15 — the idle timeout, task T-52-4
// ---------------------------------------------------------------------------------------

/// Shortest `[buffer] idle_timeout_s` this module will act on — **FR-15**, task T-52-4.
///
/// The rule is checked on the thirty-second liveness tick of FR-80 and on nothing else (no new
/// timer is created, and none may be: `SetTimer` in the callback is forbidden). A timeout
/// shorter than the tick could therefore not be honoured anyway — it would round up to the
/// tick — and a user who wrote `5` would get thirty seconds while believing they had five. The
/// floor says out loud what the mechanism can do.
pub const MIN_IDLE_TIMEOUT_S: u32 = 30;

/// Milliseconds in a second — the one conversion FR-15 needs, named rather than written as a
/// literal in three places.
const MS_PER_SECOND: u32 = 1_000;

/// Longest `[buffer] idle_timeout_s` this module will act on — one day.
///
/// Above a day the rule stops being «набранное слово не живёт вечно» and becomes a promise the
/// program cannot keep across a logon anyway: FR-90's suspension, a session lock (row 8 of the
/// FR-10 table) and a reboot all empty the ring long before. A hand-edited file may hold any
/// number; this is what the program will do with it.
pub const MAX_IDLE_TIMEOUT_S: u32 = 86_400;

/// The idle timeout as it is **published** — FR-15, task T-52-4.
///
/// [`effective_capacity`] for the timeout, and public for the same reason: `app` clamps the
/// configured value where it crosses to the input thread, exactly as task T-13-13 clamps the
/// three millisecond fields of section 7 there.
///
/// ⚠ **Zero is kept as zero, and that is the difference from [`effective_capacity`].** A
/// capacity of zero is a mistake — a buffer of no strokes is a program that quietly stopped
/// working — whereas `idle_timeout_s = 0` is the documented way to switch the rule off, which is
/// a request and not a slip. It is the same reading task T-13-13 gives the millisecond fields,
/// where zero means «не ждать».
pub const fn effective_idle_timeout_s(requested: u32) -> u32 {
    match requested {
        0 => 0,
        requested if requested < MIN_IDLE_TIMEOUT_S => MIN_IDLE_TIMEOUT_S,
        requested if requested > MAX_IDLE_TIMEOUT_S => MAX_IDLE_TIMEOUT_S,
        requested => requested,
    }
}

// ---------------------------------------------------------------------------------------
// The modifier mask of FR-04
// ---------------------------------------------------------------------------------------

/// The modifier mask of FR-04: `Shift|Ctrl|Alt|AltGr|Caps|Extended`.
///
/// Six bits, in the order section 4.1 lists them. Two of the six are not modifiers in the
/// ordinary sense and are here because section 4.1 puts them here:
///
/// * `Caps` is a toggle rather than a held key, and it changes which character a key
///   produces, so the cache of FR-20 keys on it;
/// * `Extended` is the `LLKHF_EXTENDED` flag of FR-05. It does not modify a key, it *names*
///   one — the keypad `/` and the `/?` key of the main block both report the scan code
///   `0x35` and only the `E0` prefix tells them apart (task T-02-1a). FR-05 requires it to be
///   kept here and replayed on injection, which is task T-04-1.
///
/// `Win` is deliberately **not** among the bits: section 4.1 does not list it, and FR-10
/// needs it only at the instant the key is pressed, to tell a command from text. It is
/// tracked in [`Recorder`] and never stored in a stroke.
///
/// No `Debug`: see the note on SEC-07 in the module documentation.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrokeMods(u8);

impl StrokeMods {
    /// Nothing held.
    pub const NONE: Self = Self(0);
    /// `Shift` held, either one.
    pub const SHIFT: Self = Self(0b0000_0001);
    /// `Ctrl` held, either one.
    pub const CTRL: Self = Self(0b0000_0010);
    /// `Alt` held, either one.
    pub const ALT: Self = Self(0b0000_0100);
    /// `AltGr` held — `Ctrl` plus the *right* `Alt`, which is how the keyboard reports it.
    pub const ALTGR: Self = Self(0b0000_1000);
    /// `CapsLock` toggled on.
    pub const CAPS: Self = Self(0b0001_0000);
    /// The `LLKHF_EXTENDED` flag of FR-05.
    pub const EXTENDED: Self = Self(0b0010_0000);

    /// Every bit this type defines.
    const ALL_BITS: u8 = 0b0011_1111;

    /// Keeps the six meaningful bits of `bits` and drops the rest.
    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits & Self::ALL_BITS)
    }

    /// The six meaningful bits — the `mods: u8` of section 4.1.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// This mask with the bits of `other` added.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every bit of `other` is set here.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether `Shift` was held.
    pub const fn shift(self) -> bool {
        self.contains(Self::SHIFT)
    }

    /// Whether `Ctrl` was held.
    pub const fn ctrl(self) -> bool {
        self.contains(Self::CTRL)
    }

    /// Whether `Alt` was held.
    pub const fn alt(self) -> bool {
        self.contains(Self::ALT)
    }

    /// Whether `AltGr` was held.
    pub const fn altgr(self) -> bool {
        self.contains(Self::ALTGR)
    }

    /// Whether `CapsLock` was on.
    pub const fn caps(self) -> bool {
        self.contains(Self::CAPS)
    }

    /// Whether the key was an extended one — FR-05.
    pub const fn extended(self) -> bool {
        self.contains(Self::EXTENDED)
    }

    /// Narrows the mask of FR-04 to the three modifiers the cache of FR-20 is keyed by.
    ///
    /// Module `layouts` says it in its own documentation: "The wider stroke mask of FR-04 is
    /// narrowed to this type by the buffer module." `Ctrl` and `Alt` on their own do not
    /// change which character a key produces — they mark a command, and FR-10 flushes on it —
    /// so they have no place in a cache key. `Extended` leaves the mask too, and moves into
    /// the *key* half of the lookup, which is where task T-02-1a put it.
    pub const fn to_layout_mods(self) -> Mods {
        Mods::new(self.shift(), self.caps(), self.altgr())
    }
}

// ---------------------------------------------------------------------------------------
// The stroke of FR-04
// ---------------------------------------------------------------------------------------

/// One recorded keystroke — the structure section 4.1 prints, field for field.
///
/// | Section 4.1 | Here |
/// |---|---|
/// | `vk: u16` | [`Stroke::vk`] |
/// | `scan: u16` | [`Stroke::scan`] |
/// | `mods: u8` | [`Stroke::mods`], as the six named bits of [`StrokeMods`] |
/// | `hkl: isize` | [`Stroke::hkl`], as [`LayoutId`] — the same handle value, in the type the rest of the program carries it in |
/// | `time: u32` | [`Stroke::time`] |
/// | `chars: [u16; 4]` | [`Stroke::chars`] |
/// | `len: u8` | [`Stroke::len`] |
///
/// The last two are held as the [`KeyMapping`] the cache answered with, which *is* the pair
/// `chars` and `len` plus one thing section 4.1 does not have room for and FR-24 requires: the
/// mark that the key was a dead key. Splitting them apart and putting the mark back would
/// produce the same three values in three fields instead of one, and would let them disagree.
///
/// Plain `Copy` data, thirty-two bytes; building one allocates nothing.
///
/// No `Debug` and no `Display`: see the note on SEC-01 and SEC-07 in the module documentation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Stroke {
    vk: u16,
    scan: u16,
    mods: StrokeMods,
    hkl: LayoutId,
    time: u32,
    produced: KeyMapping,
}

impl Stroke {
    /// The value every free slot of the ring holds — SEC-02.
    ///
    /// A real value of the type rather than a pattern of bytes, so that the zeroing write is
    /// an ordinary typed store that no `unsafe` assumption about representations rests on.
    pub const ZEROED: Self = Self {
        vk: 0,
        scan: 0,
        mods: StrokeMods::NONE,
        hkl: LayoutId::from_raw(0),
        time: 0,
        produced: KeyMapping::EMPTY,
    };

    /// Assembles a stroke from its recorded parts.
    ///
    /// Offered for task T-04-1 and for the tests; the product builds strokes through
    /// [`Recorder::record`], which is the only place that knows where the parts come from.
    pub const fn new(
        vk: u16,
        scan: u16,
        mods: StrokeMods,
        hkl: LayoutId,
        time: u32,
        produced: KeyMapping,
    ) -> Self {
        Self {
            vk,
            scan,
            mods,
            hkl,
            time,
            produced,
        }
    }

    /// Virtual-key code — `KBDLLHOOKSTRUCT.vkCode`.
    pub const fn vk(self) -> u16 {
        self.vk
    }

    /// Scan code of the physical key — `KBDLLHOOKSTRUCT.scanCode`, low byte.
    pub const fn scan(self) -> u16 {
        self.scan
    }

    /// The modifier mask held when the key went down, `Extended` included — FR-04, FR-05.
    pub const fn mods(self) -> StrokeMods {
        self.mods
    }

    /// The layout that was active when the key went down — the `hkl` of section 4.1.
    ///
    /// Stored per stroke, which is what FR-11 rests on: the user switching layouts mid-word
    /// does not flush anything, because every stroke remembers what it was typed under.
    pub const fn hkl(self) -> LayoutId {
        self.hkl
    }

    /// `KBDLLHOOKSTRUCT.time` — the timestamp [`Recorder::reset_up_to`] resolves flush races
    /// against (FR-12).
    ///
    /// Documented by Windows as "equivalent to what `GetMessageTime` would return", that is,
    /// the value of the system tick counter at the moment the keystroke was generated. That
    /// identity is what makes it comparable with the timestamps of the two asynchronous flush
    /// sources; see [`is_newer_than`].
    pub const fn time(self) -> u32 {
        self.time
    }

    /// What the key produced in the layout it was typed under.
    pub const fn produced(self) -> KeyMapping {
        self.produced
    }

    /// The UTF-16 code units the key produced, borrowed in place — the `chars` of section 4.1
    /// down to `len`.
    pub fn units(&self) -> &[u16] {
        self.produced.units()
    }

    /// The `chars: [u16; 4]` of section 4.1, padded with zeroes exactly as the structure there
    /// is.
    pub fn chars(self) -> [u16; MAX_UNITS] {
        let mut chars = [0u16; MAX_UNITS];
        let units = self.produced.units();
        chars[..units.len()].copy_from_slice(units);
        chars
    }

    /// The `len: u8` of section 4.1 — how many of [`Stroke::chars`] are meaningful.
    pub fn len(self) -> usize {
        self.produced.units().len()
    }

    /// Whether the key produced no characters at all.
    ///
    /// True for a key the layout has nothing on, and for one the cache had no answer for.
    ///
    /// Not an error in either case. What FR-23 then does with it is decided by the **target**
    /// layout and not here — see the module documentation, task Т-22-10: such a stroke goes
    /// through [`crate::convert::convert_stroke`] unchanged only if the target is silent on that
    /// key too, and takes the target's own character otherwise.
    pub fn is_empty(self) -> bool {
        self.produced.units().is_empty()
    }

    /// Whether the stroke was a dead key — FR-24.
    pub const fn is_dead(self) -> bool {
        self.produced.is_dead()
    }

    /// The canonical conversion form of this stroke — decision Р-22.
    ///
    /// `convert::Keystroke` owns what conversion needs; this type adds the timestamps and the
    /// layout the buffer needs and **produces** `Keystroke` values rather than replacing the
    /// type with its own. No allocation, no borrow: five `Copy` fields.
    pub const fn keystroke(self) -> Keystroke {
        Keystroke::new(
            self.hkl,
            self.scan,
            self.mods.extended(),
            self.mods.to_layout_mods(),
            self.produced,
        )
    }
}

// ---------------------------------------------------------------------------------------
// The ring — FR-07, SEC-02
// ---------------------------------------------------------------------------------------

/// The ring of FR-07: a fixed array, the oldest stroke evicted when it is full.
///
/// Allocated once, when the buffer is created, and never again — growing inside the hook
/// callback is exactly the allocation NFR-03 forbids. The two indices are the whole
/// bookkeeping: `head` is where the oldest stroke sits and `len` how many are live.
struct Ring {
    /// The backing array. The only `Box` in this module, and it is filled once by
    /// [`Ring::with_capacity`], outside the callback.
    slots: Box<[Stroke]>,
    /// Index of the oldest live stroke.
    head: usize,
    /// How many strokes are live, `0..=capacity`.
    ///
    /// Written **only** by [`Ring::set_len`] — see there for why that matters.
    len: usize,
}

impl Ring {
    /// A ring of exactly `capacity` slots, all zero.
    fn with_capacity(capacity: usize) -> Self {
        let mut ring = Self {
            slots: core::iter::repeat_n(Stroke::ZEROED, capacity).collect(),
            head: 0,
            len: 0,
        };

        // The literal above is a write of `len` like any other, and it is routed back through
        // the one function that publishes so that the rule of `set_len` has no exception at
        // all. See [`Ring::set_len`].
        ring.set_len(0);

        ring
    }

    /// The one place `len` is written — and, under the `testing` feature, the one place the
    /// length mirror of SEC-04a is published from.
    ///
    /// # Why every write goes through here
    ///
    /// The debug channel of SEC-04a reports `buffer_len`, and the acceptance bench of
    /// section 11.5 takes that number for the truth. A mirror that lagged behind the buffer
    /// would be worse than no mirror at all: an absent number is noticed, a stale one is
    /// believed. Section 6.3 makes the buffer a thread-local of the input thread, so no other
    /// thread can read it and the value has to be *published* rather than fetched — and the
    /// only way to be sure the publication is complete is for it to be impossible to change
    /// the length without performing it. Hence one writer, and `len` private to it.
    ///
    /// # NFR-01 to NFR-05
    ///
    /// This is on the callback path of the keyboard hook. What the `testing` feature adds to
    /// that path is one relaxed atomic store and nothing else: no allocation (NFR-03), no
    /// lock (NFR-04), no I/O (NFR-05). In a build without the feature — every Release build,
    /// by SEC-04a condition 1 — the call is not compiled at all and this is a plain field
    /// assignment.
    fn set_len(&mut self, len: usize) {
        self.len = len;

        #[cfg(feature = "testing")]
        crate::control::note_buffer_len(len);
    }

    /// How many strokes fit.
    fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// How many strokes are live.
    fn len(&self) -> usize {
        self.len
    }

    /// The `index`-th live stroke, oldest first.
    fn get(&self, index: usize) -> Option<Stroke> {
        if index >= self.len {
            return None;
        }
        Some(self.slots[(self.head + index) % self.capacity()])
    }

    /// Adds a stroke and reports whether the oldest one was evicted — FR-07.
    ///
    /// The evicted slot is zeroed **before** the new stroke is written into it. The write
    /// looks redundant, since the slot is overwritten on the next line, and it is not: the
    /// task puts eviction among the flushes SEC-02 speaks of, and a slot that happens to be
    /// covered by the stroke replacing it meets the requirement by luck rather than by
    /// construction. Being volatile, the zeroing cannot be folded into the store that follows.
    fn push(&mut self, stroke: Stroke) -> bool {
        let capacity = self.capacity();
        let evicted = self.len == capacity;
        let index = (self.head + self.len) % capacity;

        self.zero_slot(index);
        self.slots[index] = stroke;

        if evicted {
            self.head = (self.head + 1) % capacity;
        } else {
            self.set_len(self.len + 1);
        }

        evicted
    }

    /// Takes the newest stroke out and reports whether there was one — the `Backspace` row of
    /// FR-10.
    ///
    /// The freed slot is zeroed, because SEC-02 says "at every flush" and a stroke the user
    /// has just deleted is the last one that should be left lying in memory.
    fn pop(&mut self) -> bool {
        if self.len == 0 {
            return false;
        }

        let index = (self.head + self.len - 1) % self.capacity();
        self.zero_slot(index);
        self.set_len(self.len - 1);

        true
    }

    /// Empties the ring and overwrites **the whole backing array** with zeroes — SEC-02.
    ///
    /// The whole array and not merely the live window: a flush that moved the indices would
    /// leave the strokes sitting behind them, which is the "просто помечается пустым" SEC-02
    /// names. At the capacity of FR-07 this is eight kilobytes of stores, a fraction of a
    /// microsecond, which is what makes the literal reading affordable inside the callback
    /// (NFR-01).
    fn clear(&mut self) {
        for index in 0..self.capacity() {
            self.zero_slot(index);
        }

        self.head = 0;
        self.set_len(0);
    }

    /// Drops every stroke recorded at or before `event_time` and reports how many survived —
    /// FR-12.
    ///
    /// Every slot the removal frees is zeroed, and so is every slot a surviving stroke is moved
    /// out of: SEC-02 says the memory is overwritten and not merely marked empty, and a partial
    /// flush frees memory exactly as a full one does. The slot a survivor is moved *into* is
    /// zeroed before it is written as well — the same discipline [`Ring::push`] follows, and for
    /// the same reason: a slot covered by the value replacing it meets SEC-02 by luck.
    ///
    /// # Two passes, and why the second one exists
    ///
    /// Strokes enter the ring in the order the system generated them and each carries the tick
    /// count of that moment, so the strokes an event covers are a **prefix** of the ring, and
    /// the first pass removes it the cheap way: zero the slots, move `head` past them. That is
    /// the only pass that ever does anything in this program.
    ///
    /// The second pass is there because FR-12 says "все нажатия с `time <= T`" and not "первые
    /// нажатия": it compacts anything the event covers that is left deeper in the ring. Nothing
    /// in this program can produce such a ring today — it would take a clock that ran backwards
    /// between two keystrokes — and the requirement is met by construction rather than by that
    /// argument, which costs one comparison per live stroke on a path that runs in the message
    /// loop and not in the callback.
    fn retain_after(&mut self, event_time: u32) -> usize {
        let capacity = self.capacity();

        // Pass one: the leading run of strokes the event covers.
        let mut leading = 0;
        while leading < self.len
            && !is_newer_than(
                self.slots[(self.head + leading) % capacity].time,
                event_time,
            )
        {
            leading += 1;
        }

        for offset in 0..leading {
            self.zero_slot((self.head + offset) % capacity);
        }

        self.head = (self.head + leading) % capacity;
        self.set_len(self.len - leading);

        // Pass two: anything the event covers that is not in that run.
        let mut kept = 0;

        for offset in 0..self.len {
            let slot = (self.head + offset) % capacity;
            let stroke = self.slots[slot];

            if !is_newer_than(stroke.time, event_time) {
                continue;
            }

            let target = (self.head + kept) % capacity;

            if target != slot {
                self.zero_slot(target);
                self.slots[target] = stroke;
            }

            kept += 1;
        }

        for offset in kept..self.len {
            self.zero_slot((self.head + offset) % capacity);
        }

        self.set_len(kept);

        kept
    }

    /// How many live strokes were made strictly after `event_time` — FR-12.
    ///
    /// Asked before anything is removed, because the answer is what decides between the two
    /// halves of the requirement: zero survivors means the event is newer than every stroke and
    /// the flush is the full one, anything else means the flush is partial at most.
    ///
    /// Every live stroke is examined rather than the newest one alone. The newest stroke is the
    /// last one only while the clock runs forward, and this way the count owes that nothing.
    fn newer_than(&self, event_time: u32) -> usize {
        let capacity = self.capacity();

        (0..self.len)
            .filter(|offset| {
                is_newer_than(self.slots[(self.head + offset) % capacity].time, event_time)
            })
            .count()
    }

    /// Whether any live stroke was made inside the half-open window `(event_time − delta,
    /// event_time]` — **FR-14**, task Т-48-2.
    ///
    /// The mirror of [`Ring::newer_than`], and it borrows the whole of its arithmetic: both ends
    /// are [`is_newer_than`], so the wrap of the 32-bit tick counter is not a special case here
    /// either. Every live stroke is examined for the reason given there — the newest stroke is
    /// the last one only while the clock runs forward.
    fn any_within(&self, event_time: u32, delta: u32) -> bool {
        let capacity = self.capacity();
        let opened = event_time.wrapping_sub(delta);

        (0..self.len).any(|offset| {
            let time = self.slots[(self.head + offset) % capacity].time;

            !is_newer_than(time, event_time) && is_newer_than(time, opened)
        })
    }

    /// Writes zeroes over one slot so that no optimiser may remove the write — SEC-02.
    fn zero_slot(&mut self, index: usize) {
        // Bounds-checked here, in safe code, so that the raw write below cannot be the place
        // an index mistake turns into memory corruption.
        let slot: *mut Stroke = &mut self.slots[index];

        // SAFETY: `slot` comes from a mutable borrow of an element of `self.slots` taken one
        // line above, so it is non-null, aligned, in bounds and valid for a write of one
        // `Stroke` for the whole of this call; nothing else can be reading it, because the
        // borrow of `self` is exclusive. `Stroke::ZEROED` is an ordinary value of the type,
        // not a byte pattern assumed to be one, so the write leaves a valid value behind.
        // Volatile is the requirement, not a flourish: SEC-02 asks for the write to happen,
        // and a plain assignment into a slot that is about to be overwritten or dropped is a
        // dead store the compiler is allowed to delete.
        unsafe { ptr::write_volatile(slot, Stroke::ZEROED) };

        // Volatile accesses may not be reordered against each other, but they may be against
        // ordinary memory operations; the fence keeps the zeroes from being sunk past whatever
        // the caller does next. It is a compile-time barrier only and costs no instruction.
        compiler_fence(Ordering::SeqCst);
    }
}

impl Drop for Ring {
    /// SEC-02 once more, at the end of the buffer's life.
    ///
    /// The heap block is about to go back to the allocator, where the next owner can read
    /// whatever is left in it. What is left in it is zeroes.
    fn drop(&mut self) {
        self.clear();
    }
}

// ---------------------------------------------------------------------------------------
// The same write, for the working buffers outside the ring — SEC-01, SEC-02
// ---------------------------------------------------------------------------------------

/// Writes zeroes over every element of `slice` so that no optimiser may remove the writes.
///
/// [`Ring::zero_slot`] above is this write for one slot of the typing buffer; this is the same
/// write for a slice somebody else owns, and it exists because the typing buffer is not the only
/// place the user's text lives. Module `inject` holds the converted text, the `INPUT` packet and
/// the copy of the strokes; module `selection` holds the clipboard snapshot, the decoded text,
/// the recoded text and the blocks on the way in and out of the clipboard. Both modules promise
/// in their own documentation that those buffers get the treatment SEC-02 prescribes here — and
/// until task **T-13-15** (audit of 2026-08-24) both kept that promise with `slice.fill(0)`.
///
/// A `fill` immediately before the vector is released is precisely the store the module
/// documentation above calls deletable: **a plain assignment into a slot that is about to be
/// overwritten or dropped is a dead store the compiler is allowed to delete**, and the Release
/// profile of section 3.2 — `opt-level 3`, fat LTO, `codegen-units 1` — is the configuration in
/// which it is most likely to be noticed and deleted, because LLVM knows what the deallocator
/// that follows does. [`core::ptr::write_volatile`] is the one write whose presence in the object
/// code the language guarantees, and [`core::sync::atomic::compiler_fence`] keeps the zeroes from
/// being sunk past whatever the caller does next. One implementation rather than six copies, so
/// that the promise and the code cannot drift apart again.
///
/// This is a *strengthening of a write that was already there* and not a new behaviour: nothing
/// observable changes, no text reaches a log or a panic message (SEC-01, SEC-07), and nothing
/// here is reachable from the hook callback — every caller is on the input thread after the
/// handoff or on the UI thread (NFR-01…NFR-05).
///
/// # Why `Copy + Default` and not a byte fill
///
/// The elements to be zeroed are of four types, and only two of them are numbers: `u8` and `u16`
/// of the text buffers, `INPUT` of the injected packet and [`crate::convert::Keystroke`] of the
/// stroke copy. `Default::default()` is the zero of all four — `0` for the integers,
/// `core::mem::zeroed()` for `INPUT`, the derived all-zero value for `Keystroke` — so one
/// function covers them without a byte-level write that would have to argue about padding and
/// about which byte patterns are valid values of the type. The bound is what makes the volatile
/// write sound in both directions:
///
/// * `Default` gives a **value of the type** to write, not a byte pattern assumed to be one;
/// * `Copy` means the element being overwritten has no destructor, so writing over it without
///   running one leaks nothing. A `T` that owned a heap block would need its `Drop` instead, and
///   the bound keeps such a type from ever reaching this function.
pub(crate) fn zero_slice<T: Copy + Default>(slice: &mut [T]) {
    for element in slice {
        // Taken from the exclusive borrow the iterator hands out, in safe code, so that the raw
        // write below cannot be the place an indexing mistake turns into memory corruption.
        let slot: *mut T = element;

        // SAFETY: `slot` is the address of an element of `slice`, obtained from the mutable
        // borrow the iterator yields one line above. It is therefore non-null, in bounds, and
        // aligned for `T` — the elements of a slice are laid out at `align_of::<T>()` by the
        // language, and the slice reference itself is required to be aligned — and it is valid
        // for a write of one `T` for the whole of this call, because the borrow is exclusive and
        // nothing else can be reading or writing it. `T::default()` is an ordinary value of the
        // type, and `T: Copy` means the value it replaces has no destructor being skipped.
        // Volatile is the requirement and not a flourish: the write has to survive an optimiser
        // that can see the buffer is never read again.
        unsafe { ptr::write_volatile(slot, T::default()) };

        // Volatile accesses may not be reordered against each other, but they may be against
        // ordinary memory operations; the fence keeps the zeroes from being sunk past whatever
        // the caller does next. It is a compile-time barrier only and costs no instruction —
        // the same construction, per element, that `zero_slot` uses per slot.
        compiler_fence(Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------------------
// Modifier tracking
// ---------------------------------------------------------------------------------------

/// Which modifier a virtual key is.
///
/// ⭐ **Task T-19-2, finding 4 of the audit of 2026-08-31: every held modifier is sided.** Until
/// that task `Shift`, `Ctrl` and `Win` had one variant each while `Alt` had two, and the
/// asymmetry was the defect rather than a simplification — see [`Held::apply`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// The left `Shift`.
    ShiftLeft,
    /// The right `Shift`.
    ShiftRight,
    /// The left `Ctrl`, including the one the keyboard fakes in front of `AltGr`.
    CtrlLeft,
    /// The right `Ctrl`.
    CtrlRight,
    /// The left `Alt` — a command modifier and nothing else.
    AltLeft,
    /// The right `Alt` — half of `AltGr` when `Ctrl` is down with it.
    AltRight,
    /// The left `Win`.
    WinLeft,
    /// The right `Win`.
    WinRight,
    /// `CapsLock`, a toggle rather than a held key.
    Caps,
}

/// Which modifier `vk` is, if it is one.
///
/// `WH_KEYBOARD_LL` reports the sided codes — `VK_LSHIFT` rather than `VK_SHIFT` — which is
/// what makes `AltGr` recognisable at all, and since task T-19-2 it is what every one of these
/// rows is built on. The neutral codes are accepted as well and each of them is read as the
/// *left* key, which is the conservative reading in the one place it can matter: a stroke that
/// might be `AltGr` and might be a command is treated as a command, and FR-10 flushes. Losing a
/// buffer is recoverable; converting a menu accelerator into text is not. For the other three
/// the side carries no meaning of its own — what the sides exist for is the pair — and reading
/// the neutral code as the left key keeps one arbitrary rule instead of three.
const fn modifier_role(vk: u16) -> Option<Role> {
    match vk {
        v if v == VK_SHIFT.0 || v == VK_LSHIFT.0 => Some(Role::ShiftLeft),
        v if v == VK_RSHIFT.0 => Some(Role::ShiftRight),
        v if v == VK_CONTROL.0 || v == VK_LCONTROL.0 => Some(Role::CtrlLeft),
        v if v == VK_RCONTROL.0 => Some(Role::CtrlRight),
        v if v == VK_MENU.0 || v == VK_LMENU.0 => Some(Role::AltLeft),
        v if v == VK_RMENU.0 => Some(Role::AltRight),
        v if v == VK_LWIN.0 => Some(Role::WinLeft),
        v if v == VK_RWIN.0 => Some(Role::WinRight),
        v if v == VK_CAPITAL.0 => Some(Role::Caps),
        _ => None,
    }
}

/// The seven volume and media keys of FR-10 — task **T-52-1**.
///
/// ⭐ **The user's decision of 2026-09-09.** Turning the volume down in the middle of a word is
/// neither writing nor a command over the text: the key changes nothing on the screen and moves
/// no caret, so the word the user is in the middle of typing has to survive it. Without a role
/// of their own these seven would fall into the emptiness rule below and end the word, which is
/// the one consequence of that rule the user did not want.
///
/// **The list is closed.** Seven codes, every one of them documented by Windows, and it cannot
/// grow without another decision: `VK_LAUNCH_*` and `VK_BROWSER_*` are deliberately *not* here —
/// they start an application or move a page, which is a boundary in exactly the sense the right
/// mouse button is.
const MEDIA_KEYS: [u16; 7] = [
    VK_VOLUME_MUTE.0,
    VK_VOLUME_DOWN.0,
    VK_VOLUME_UP.0,
    VK_MEDIA_NEXT_TRACK.0,
    VK_MEDIA_PREV_TRACK.0,
    VK_MEDIA_STOP.0,
    VK_MEDIA_PLAY_PAUSE.0,
];

/// Whether `vk` is the key half of a "switch to this language" combination — task **T-52-2**.
///
/// The **top-row** digits `0`…`9` and the grave key: what the Windows language dialog offers
/// beside `Alt+Shift` and `Ctrl+Shift`, and the whole list it offers. `VK_NUMPAD0`…`VK_NUMPAD9`
/// are deliberately absent — they are different physical keys and the dialog does not bind
/// them. The modifier half of the combination is at the call site in [`Recorder::record`], with
/// the reasoning for every one of its four conditions.
///
/// A range test and one comparison; `const` for the same reason [`modifier_role`] is.
const fn is_layout_switch_key(vk: u16) -> bool {
    matches!(vk, VK_TOP_ROW_0..=VK_TOP_ROW_9) || vk == VK_OEM_3.0
}

/// `VK_0` — the first of the ten top-row digit codes, which Windows defines by their ASCII
/// values and the `windows` crate does not name.
const VK_TOP_ROW_0: u16 = 0x30;
/// `VK_9`, the last of them.
const VK_TOP_ROW_9: u16 = 0x39;

/// Whether `vk` is one of the seven keys of [`MEDIA_KEYS`].
///
/// Beside [`modifier_role`] and asked the same way, because it answers the same kind of
/// question: what *role* the key has in the FR-10 table, before anything about characters is
/// consulted. Seven comparisons on the path of an ordinary letter, no allocation (NFR-03), no
/// lock (NFR-04), no I/O (NFR-05) and no Win32 call (NFR-02) — the same terms `modifier_role`
/// has been paying since task T-03-2.
fn is_media_key(vk: u16) -> bool {
    MEDIA_KEYS.contains(&vk)
}

/// What the input thread believes is held down right now.
///
/// Maintained from the hook stream itself rather than asked of the system: `GetAsyncKeyState`
/// on every keystroke would put three to five system calls on the hot path for information the
/// callback is already being handed, and `GetKeyState` would answer from the input thread's own
/// queue, which never has the keyboard focus and would report "nothing is held" for ever — the
/// same argument module `hook` makes where it chooses `GetAsyncKeyState` for FR-96.
///
/// The known limitation of tracking rather than asking: a modifier that went down before the
/// hook was installed, or while the program was suspended (FR-90, when [`Recorder::record`] is
/// never reached), is not seen. Its release is seen, and clears it, so the state cannot stay
/// wrong indefinitely; and `CapsLock` can be seeded from a caller that has a trustworthy source
/// through [`Recorder::set_caps_lock`] — which task T-13-4 finally gave callers, five of them,
/// listed at [`crate::buffer::set_caps_lock`]. Until then it described a mechanism nobody used and
/// `caps` started `false` in every session, whatever the keyboard's light said.
///
/// ⚠ **Task T-10-12 corrected the sentence above: "its release is seen" was an assumption, and
/// it is false.** A release that never arrives leaves the belief raised for ever, and defect D
/// is what that costs — see [`Held::reconcile`] and the comment inside [`Recorder::record`].
/// Until task **T-39-3** that repair covered `Ctrl`, `Alt` and `Win` and not `Shift`, which had no
/// way back at all (finding С8); it now comes down once per word — see [`Held::reconcile_shift`].
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Held {
    shift_left: bool,
    shift_right: bool,
    ctrl_left: bool,
    ctrl_right: bool,
    alt_left: bool,
    alt_right: bool,
    win_left: bool,
    win_right: bool,
    caps: bool,
    /// Whether `CapsLock` itself is being **held down** right now — task **Т-22-3**.
    ///
    /// Not a modifier state and never read as one: the only thing it does is tell a first press
    /// from the auto-repeat behind it, so that [`Held::apply`] flips [`caps`](Self::caps) once
    /// per press instead of once per event. See that function for the measurement it rests on.
    ///
    /// Deliberately outside [`Held::reconcile`]'s reach, like every other `CapsLock` bit: the
    /// system's answer about a toggle key's "physical state" means something else entirely, and
    /// the worst a stale `true` here can do is skip one flip — never latch the command row of
    /// FR-10, which is the defect that function exists for.
    caps_down: bool,
}

impl Held {
    /// Applies one modifier key event.
    ///
    /// ⭐ **Task T-19-2, finding 4 of the audit of 2026-08-31.** One bit per *side*, because the
    /// assignment is unconditional and the hook stream is sided. With one bit for the pair,
    /// `LShift` down, `RShift` down, `LShift` up left the tracker saying "no Shift is held"
    /// while the user was still holding one: the buffer then recorded the unshifted character
    /// for what the screen showed shifted, and the bit-for-bit rollback of FR-32 restored the
    /// wrong case. `Ctrl` was worse — a `Ctrl+V` in that state was recorded into the ring as
    /// text instead of taking the command row of FR-10 and flushing it.
    ///
    /// The fix is not "ignore the release when the other side is down", which would be a guess
    /// about a key nothing observed; it is one bit per key, which is what the events are.
    fn apply(&mut self, role: Role, edge: Edge) {
        let down = matches!(edge, Edge::Down);

        match role {
            Role::ShiftLeft => self.shift_left = down,
            Role::ShiftRight => self.shift_right = down,
            Role::CtrlLeft => self.ctrl_left = down,
            Role::CtrlRight => self.ctrl_right = down,
            Role::AltLeft => self.alt_left = down,
            Role::AltRight => self.alt_right = down,
            Role::WinLeft => self.win_left = down,
            Role::WinRight => self.win_right = down,
            // ⭐ **Task Т-22-3, finding №14 of the audit of 2026-08-31 — measured, then
            // repaired.** A toggle: it flips on the press, does nothing on the release, and
            // **does nothing on the auto-repeat behind the press either**.
            //
            // The line above used to flip on every `Down` and the comment used to say that this
            // "is what the keyboard does to the light as well". It is not. The probe crate
            // `<dev>\sandbox\probes\capsrepeat` sends the system a run of repeated `Down`
            // events for `VK_CAPITAL` with no `Up` between them — which is exactly what
            // typematic is on the wire, repeated make codes with no break code — and reads
            // `GetKeyState(VK_CAPITAL) & 1` after each. Over five repeats the system flipped the
            // toggle **once**, in three runs out of three, each preceded by a positive control
            // that showed the reading does follow a real press. One press, one flip, however
            // long it is held.
            //
            // What the old line cost: the belief and the machine part company whenever the
            // repeat count is even. Hold `CapsLock` down for two events and the program thinks
            // the toggle is off while the machine has it on, so every stroke afterwards is
            // recorded in the wrong case — and FR-32's bit-for-bit rollback then restores the
            // wrong case too, for the rest of the session or until the seed of task T-13-4
            // happens to put it right.
            Role::Caps => {
                if down {
                    if !self.caps_down {
                        self.caps = !self.caps;
                    }

                    self.caps_down = true;
                } else {
                    self.caps_down = false;
                }
            }
        }
    }

    /// Whether either `Shift` is down.
    const fn shift(self) -> bool {
        self.shift_left || self.shift_right
    }

    /// Whether either `Ctrl` is down.
    const fn ctrl(self) -> bool {
        self.ctrl_left || self.ctrl_right
    }

    /// Whether either `Alt` is down.
    const fn alt(self) -> bool {
        self.alt_left || self.alt_right
    }

    /// Whether either `Win` is down.
    const fn win(self) -> bool {
        self.win_left || self.win_right
    }

    /// Whether this is `AltGr` — the right `Alt` with `Ctrl`, which is how the keyboard
    /// reports it: the layout driver emits a fake left `Ctrl` in front of the right `Alt`.
    ///
    /// The distinction is the difference between text and a command. `AltGr+E` is `€` on a
    /// German layout and must reach the buffer; `Ctrl+Alt+E` is an accelerator and must flush
    /// it (FR-10).
    const fn altgr(self) -> bool {
        self.ctrl() && self.alt_right
    }

    /// Drops every command modifier the system says is **not** physically down, and answers
    /// whether anything changed — **the repair of defect D**, task T-10-12.
    ///
    /// # Why this exists
    ///
    /// The belief above is built from the hook stream alone, so it is right only while the
    /// stream is complete. One missing release — and task T-10-12 measured that a release can
    /// go missing without the program losing the hook, without a counter moving and without a
    /// single one of its own error paths being taken — raises a flag that nothing in the
    /// program could ever lower again. Every subsequent keystroke then takes the command row
    /// of FR-10, the ring is cleared and nothing is ever recorded, in every application, until
    /// the process is restarted. The buffer stops working and the product still reports itself
    /// healthy, which is the part that cost this project three tasks of searching.
    ///
    /// # Clearing only, never raising
    ///
    /// A bit the system reports **down** is left exactly as the stream left it. Raising bits
    /// from the system would widen the command row of FR-10 — it would turn text into a
    /// command for a modifier pressed before the hook went up, which is a documented and
    /// accepted limitation of this design and **not** the defect this task was given. Clearing
    /// can only ever turn a keystroke that was being thrown away back into text, which is the
    /// defect and nothing besides it.
    ///
    /// `Shift` and `CapsLock` are not touched: neither appears in the command row, so neither
    /// can latch the defect, and `CapsLock` is a toggle whose "physical state" means something
    /// else entirely.
    ///
    /// # Sides — task T-19-2
    ///
    /// [`Physical`] carries one bit for `Ctrl` and one for `Win`, because that is what
    /// `GetAsyncKeyState(VK_CONTROL)` answers, and it is the same shape
    /// [`crate::hook::physical_modifiers`] has always produced. Each *side* of the belief is
    /// masked with it, so a side comes down only when the system says no `Ctrl` — or no `Win` —
    /// is down at all. That is exactly the behaviour of the single bit this replaced:
    /// `(left && p) || (right && p)` is `(left || right) && p`, so [`Held::ctrl`] and
    /// [`Held::win`] answer after reconciliation precisely as they did before, and no side is
    /// ever lowered while the system says the key is held. The two `Alt` bits keep the sided
    /// probe they already had.
    ///
    /// ⭐ **Task T-39-3 (finding С8, decision 122.1) adds the two `Shift` bits**, sided like `Alt`.
    /// The argument of task T-19-2 is what makes it safe: masking can only *lower* a belief, so a
    /// `Shift` the user really holds stays up and only one whose release was lost comes down.
    fn reconcile(&mut self, physical: Physical) -> bool {
        let before = *self;

        self.ctrl_left &= physical.ctrl;
        self.ctrl_right &= physical.ctrl;
        self.alt_left &= physical.alt_left;
        self.alt_right &= physical.alt_right;
        self.win_left &= physical.win;
        self.win_right &= physical.win;
        self.reconcile_shift(physical);

        *self != before
    }

    /// The `Shift` half of [`Held::reconcile`] alone — **task T-39-3**, for the check
    /// [`Recorder::record`] makes once per word. Answers whether the belief changed.
    ///
    /// ⛔ **`Shift` and nothing else**, on purpose: lowering a stuck `Ctrl` or `Alt` here would
    /// change `mods.altgr()`, which chooses the character `lookup` returns — that belongs to the
    /// command row of FR-10, which has its own check.
    fn reconcile_shift(&mut self, physical: Physical) -> bool {
        let before = (self.shift_left, self.shift_right);

        self.shift_left &= physical.shift_left;
        self.shift_right &= physical.shift_right;

        (self.shift_left, self.shift_right) != before
    }
}

/// The modifiers as the **system** reports them, against the belief of [`Held`].
///
/// Produced by [`crate::hook::physical_modifiers`], which is where the Win32 call belongs: the
/// argument for `GetAsyncKeyState` over `GetKeyState` is already written out there for FR-96,
/// and module `inject` makes the same choice for the same reason (`src\inject.rs:253`) — the
/// asynchronous state is the desktop's, and this program's input thread never holds the
/// keyboard focus that `GetKeyState` would answer from.
///
/// Carried as a plain `Copy` value so that [`Recorder::record`] stays a function of its
/// arguments and can be tested without a keyboard: NFR-03 (nothing allocated), NFR-04 (no lock,
/// no atomic) and NFR-05 (no I/O) all survive it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Physical {
    /// Either `Ctrl`.
    pub ctrl: bool,
    /// The left `Alt`.
    pub alt_left: bool,
    /// The right `Alt` — `AltGr` on the layouts that have one.
    pub alt_right: bool,
    /// Either `Win`.
    pub win: bool,
    /// The left `Shift` — **finding С8, task T-39-3**. Sided like the two `Alt` bits, because
    /// [`Held`] tracks the two keys separately (task T-19-2) and a real left `Shift` must not keep
    /// up a right one whose release was lost.
    pub shift_left: bool,
    /// The right `Shift`.
    pub shift_right: bool,
}

/// How [`Recorder`] asks the system what is really held.
///
/// A bare function pointer and not a closure or a trait object: it is read on the command row
/// of FR-10 and — since task T-39-3 — on the first stroke of a word while a `Shift` is believed
/// held, and nowhere else; it allocates nothing, and it leaves [`Recorder`] `Send`-neutral
/// and free of any interior mutability the callback would have to synchronise on.
pub type PhysicalProbe = fn() -> Physical;

/// How [`Recorder`] asks the system which layout the user is **really** typing in —
/// **the repair of defect E**, task T-10-14.
///
/// A bare function pointer for the same three reasons [`PhysicalProbe`] is one: it is read on
/// one rare row of [`Recorder::record`] and nowhere else, it allocates nothing, and it leaves
/// [`Recorder`] free of interior mutability the callback would have to synchronise on.
///
/// ⭐ **The product's implementation is [`crate::switch::current`], and it is not a new one.**
/// That function *is* FR-52 — `GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(),
/// null))`, the expression the requirement gives word for word, with each of the three returns
/// examined as NFR-13 demands — and it was already public and already the reading the selection
/// path of FR-60/FR-61 makes at the moment of the press. Task T-10-14 wrote no reader of its own
/// and deliberately did not: a second copy of those three calls is a second thing to keep right,
/// and this module's problem was never *how* to read the layout but **when**.
///
/// A test supplies its own and thereby **states**, explicitly, what the layout of the foreground
/// window is while it feeds its synthetic strokes — which is what makes the detector of criterion
/// 11 two-sided without a live keyboard, exactly as `PhysicalProbe` did for defect D.
pub type LayoutProbe = fn() -> LayoutId;

/// How a caller tells [`Recorder`] what the machine's `CapsLock` really is — **task T-13-4**.
///
/// A bare function pointer for the same three reasons [`PhysicalProbe`] and [`LayoutProbe`] are
/// ones: it allocates nothing, it leaves [`Recorder`] free of interior mutability, and it lets a
/// test **state** the machine's toggle instead of asking a keyboard nobody is holding.
///
/// ⚠ **It is never read from inside the hook callback and must never be.** NFR-01 to NFR-05 give
/// the callback no new call of any kind, and this one is a `GetKeyState`. Every point that seeds
/// goes through [`crate::buffer::set_caps_lock`], reached from the message loop of the input
/// thread —
/// see there for the whole list and for why the thread is the right one.
///
/// The product's implementation is [`crate::hook::caps_lock_on`].
pub type CapsProbe = fn() -> bool;

// ---------------------------------------------------------------------------------------
// The flush rules of FR-10 — the LL-hook rows only
// ---------------------------------------------------------------------------------------

/// Row 1 of the FR-10 table: the keys that end a word or a field **at once**.
///
/// ⭐ **`Space` left this list in task Т-24-2** — the user's decision on question 82. It is still
/// a boundary of row 1 and it is now the **soft** one: the stroke goes into the ring and the
/// buffer stands in «слово + хвост» until the next key that writes. The rule is not a list, so it
/// lives in [`Recorder::record`] beside the others rather than here; see the soft-boundary block
/// there for the whole of it.
///
/// The three that are left did not move, and FR-10 says why: after `Enter` the text has usually
/// been sent already, so a deferred erasure would land in a text that is not the one it was
/// recorded from, and `Tab` takes the focus away, where the flush of the focus change would empty
/// the ring before any softness could apply.
const BOUNDARY_KEYS: [u16; 3] = [VK_TAB.0, VK_RETURN.0, VK_ESCAPE.0];

/// Row 3 of the FR-10 table: editing and navigation.
///
/// Every one of them moves the caret or removes text somewhere the buffer cannot see, so the
/// record of what was typed stops matching what is on the screen. `Backspace` is the one
/// exception in the table and is not here: it removes exactly the last character, which the
/// buffer *can* follow, so FR-10 takes one element out instead of flushing.
const EDITING_KEYS: [u16; 10] = [
    VK_DELETE.0,
    VK_LEFT.0,
    VK_UP.0,
    VK_RIGHT.0,
    VK_DOWN.0,
    VK_HOME.0,
    VK_END.0,
    VK_PRIOR.0,
    VK_NEXT.0,
    VK_INSERT.0,
];

/// Whether `vk` is one of row 3 of the FR-10 table — **the keys typing itself needs**.
///
/// Asked from `settings::capture` since task T-36-5 (finding Н9): a hotkey is suppressed in every
/// application while the program is active (FR-95), so assigning `Delete` or `Home` takes that key
/// away everywhere, and the dialog has to say so instead of quietly accepting it.
///
/// ⭐ **Read from here rather than copied there.** The table of FR-10 is this list, and a second
/// copy of it in `src\settings.rs` would be a second thing to keep right; `hook::NAMED_KEYS` is
/// not that table either — it is the spelling of the names section 7 stores. Row 1
/// ([`BOUNDARY_KEYS`]) is deliberately **not** included: `Escape` never reaches `capture` (it
/// cancels the capture — FR-94), and `Tab` and `Enter` are already refused as keys section 7 has
/// no name for.
pub(crate) fn is_editing_key(vk: u16) -> bool {
    EDITING_KEYS.contains(&vk)
}

/// Whether `vk` is one of the keys FR-10 flushes the whole buffer on.
fn flushes(vk: u16) -> bool {
    BOUNDARY_KEYS.contains(&vk) || EDITING_KEYS.contains(&vk)
}

// ---------------------------------------------------------------------------------------
// FR-12 — the race between the synchronous hook and the asynchronous flush sources
// ---------------------------------------------------------------------------------------

/// Whether `time` is strictly newer than `reference` on the 32-bit millisecond counter both
/// come from — the comparison the whole of FR-12 rests on.
///
/// # Why this is not `time > reference`
///
/// The counter is `GetTickCount`: milliseconds since the machine started, in 32 bits, which
/// wraps to zero every 49 days 17 hours. A plain `>` gets exactly one thing wrong, and it gets
/// it wrong at the worst possible moment: for the roughly one minute around the wrap, strokes
/// recorded just before it hold values near `u32::MAX` and every event arriving just after it
/// holds a value near zero, so `>` would call every one of those events *older* than every
/// stroke in the buffer — and FR-12 would keep a buffer that a click was supposed to flush.
///
/// The fix is the arithmetic of RFC 1982, serial numbers: the difference is taken modulo 2³²
/// and read as a **signed** number, so "newer" means "a positive distance forward", and the
/// wrap is simply not a special case — `0x0000_0064.wrapping_sub(0xFFFF_FF9C)` is `200`, which
/// is the true distance in milliseconds across the wrap.
///
/// The construction is correct as long as the two values are less than 2³¹ ms — 24 days 20
/// hours — apart, which is a bound this program is nowhere near: a stroke in the buffer is at
/// most a few minutes old, because every flush rule of FR-10 empties it long before that, and
/// a flush event is processed within milliseconds of being generated. The exact half-way case,
/// a difference of precisely 2³¹, reads as "not newer", that is, as "the event covers the
/// stroke", which is the side that loses a buffer rather than the side that keeps one it
/// should have dropped.
///
/// `const` and free of Win32 on purpose: it is the one piece of FR-12 that can be exhaustively
/// tested, wrap included, without a clock.
pub const fn is_newer_than(time: u32, reference: u32) -> bool {
    (time.wrapping_sub(reference) as i32) > 0
}

/// What a flush carrying a timestamp did to the buffer — FR-12.
///
/// Returned rather than kept, for the same reason [`Recorded`] is: the rule can be driven from
/// a test one event at a time, and the debug channel of SEC-04a has something to report that is
/// a count and never a stroke (SEC-07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetOutcome {
    /// The event was newer than every stroke, so the whole buffer went — the "полная очистка"
    /// of FR-12, which is what the unconditional flush of FR-10 would have done anyway.
    ///
    /// An empty buffer takes this arm too, with `removed` zero: "newer than all of them" is
    /// vacuously true of no strokes at all, and the whole backing array is zeroed either way
    /// (SEC-02).
    Cleared {
        /// How many strokes were live when the event arrived.
        removed: usize,
    },
    /// Some strokes were at or before the event and some after it: the first group was removed
    /// and its slots zeroed, the second group **survived**. This is the arm FR-12 exists for.
    Partial {
        /// How many strokes the event took out.
        removed: usize,
        /// How many were made after the event and are still in the buffer.
        kept: usize,
    },
    /// Every stroke was made after the event: nothing was removed.
    ///
    /// The case a naive "a click empties the buffer" would have got wrong outright — the user
    /// clicked, then typed, and the click is only being processed now.
    Kept {
        /// How many strokes are still in the buffer.
        kept: usize,
    },
}

// ---------------------------------------------------------------------------------------
// What one call of `record` did
// ---------------------------------------------------------------------------------------

/// The outcome of [`Recorder::record`].
///
/// Returned rather than kept, so that the rules of FR-10 can be driven from a test one stroke
/// at a time, and so that the debug channel of SEC-04a (task T-03-4) has something to report
/// that is not the content of the buffer. Every variant is a category of event; none of them
/// carries a key code or a character (SEC-07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// The stroke went into the ring.
    Stored,
    /// The stroke went into the ring and the oldest one was evicted — FR-07.
    Evicted,
    /// `Backspace` took the newest stroke out — FR-10.
    Popped,
    /// The whole buffer was flushed and zeroed — FR-10.
    Flushed,
    /// A modifier key: the held state was updated and nothing else happened. A modifier is
    /// not text, and holding one is not yet a command.
    Modifier,
    /// Nothing happened: a key release, an empty `Backspace`, or a thread with no buffer.
    Ignored,
}

/// Why a read could not be delivered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The caller's buffer is shorter than the number of live strokes — NFR-03.
    ///
    /// `needed` is how many there are, so the caller can size a buffer and ask again rather
    /// than probe. Nothing is written when this is returned: a prefix of the strokes is not a
    /// shorter answer, it is a wrong one, which is the rule [`crate::convert::convert_strokes`]
    /// already follows for its own output.
    OutputTooSmall {
        /// How many strokes are live.
        needed: usize,
    },
}

impl core::fmt::Display for ReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            // A count of strokes, never a stroke — SEC-07.
            Self::OutputTooSmall { needed } => write!(f, "the buffer holds {needed} strokes"),
        }
    }
}

impl std::error::Error for ReadError {}

// ---------------------------------------------------------------------------------------
// The recorder
// ---------------------------------------------------------------------------------------

/// The typing buffer of section 6.2: the ring, the flush rules, the modifier state and the
/// layout every stroke is recorded under.
///
/// A plain owned value with no interior mutability and no synchronisation of any kind, exactly
/// as section 6.3 describes the buffer: written by the input thread, read by the input thread.
/// The thread-local singleton the product uses is at the bottom of this file; keeping the state
/// in a value rather than in statics is what lets every rule of FR-10 be driven deterministically
/// from a test, the same split module `hook` made for [`crate::hook::classify`].
///
/// No `Debug`: it holds strokes.
pub struct Recorder {
    ring: Ring,
    /// The mapping cache of FR-20, once somebody has built one. `None` until then, and a
    /// stroke recorded meanwhile carries no characters — an outcome, not an error.
    cache: Option<LayoutCache>,
    /// The layout strokes are being recorded under — the `hkl` of FR-04.
    active: LayoutId,
    /// Position of `active` in the cache, resolved when either of them is published so that
    /// the per-keystroke work stays one array read (module `layouts` asks for exactly this).
    active_index: Option<usize>,
    held: Held,
    /// A conversion has run and the next key ends the session — the last row of FR-10.
    converted: bool,
    /// **The position counter of FR-32**: how many presses of the hotkey have been applied to
    /// the strokes the ring is holding right now.
    ///
    /// `0` means "what is on the screen is what was typed". It is kept **reduced modulo the
    /// length of the cycle** by [`Recorder::advance_cycle`], the one function that raises it, so
    /// it is a position and not a running total: there is no value it can grow into and no wrap
    /// to reason about.
    ///
    /// It is a plain field of a thread-local value and not an atomic, for the reason section 6.3
    /// gives the buffer itself: the counter is written by the input thread on the hotkey path
    /// and read by the same thread, and there is no second thread to synchronise with. That is
    /// stronger than "atomic operations only" rather than weaker — an ordinary load and store of
    /// a `usize` is indivisible, allocates nothing (NFR-03), blocks nothing (NFR-04) and touches
    /// no file (NFR-05).
    ///
    /// **FR-34.** It is zeroed by [`Recorder::clear_ring`], which every rule of FR-10 that empties
    /// the ring goes through — and, since task T-39-2, the resize of [`Recorder::set_capacity`]
    /// as well — so it cannot be left behind by a flush; and by the partial arm of
    /// [`Recorder::reset_up_to`], which takes strokes out without emptying the ring (finding Н6,
    /// task T-39-1).
    ///
    /// Written **only** by [`Recorder::set_cycle`] — see there for why that matters — with the
    /// one named exception of the struct literal that builds a recorder (task T-69-3).
    cycle: usize,
    /// How to check [`Recorder::held`] against the system — **the repair of defect D**, task
    /// T-10-12. `None` means "trust the stream", which is what this module did unconditionally
    /// before that task.
    ///
    /// # Why it is a field and not a direct call
    ///
    /// `record` has to stay a function of its arguments. A `GetAsyncKeyState` wired straight
    /// into it would make every test of the FR-10 table depend on what the keyboard of the
    /// machine running the tests happens to be doing — a synthetic `Win`↓ fed by a test is not
    /// a key anybody is holding, so the system would answer "up" and the FR-11 exception would
    /// evaporate under every existing test that exercises it. The seam lets a test state its
    /// premise instead of hiding it, and it is what makes the detector of criterion 11
    /// two-sided without a live keyboard.
    ///
    /// **The product always sets it**: [`install`] is the one path by which the input thread
    /// gets a buffer, and `app::restore_buffer` puts back the very same value it parked, so the
    /// probe travels with the recorder across the FR-70 gate.
    verify: Option<PhysicalProbe>,
    /// How to read the layout the user is **really** typing in — **the repair of defect E**,
    /// task T-10-14. `None` means "believe [`Recorder::active`]", which is what this module did
    /// unconditionally before that task.
    ///
    /// # Why the stamp was not enough on its own
    ///
    /// [`Recorder::active`] is written from outside, by `app::publish_active_layout`, when an
    /// *event* says the layout may have moved. Task T-10-14 measured that the event can arrive
    /// **before the system has applied the switch**: a synthetic `Alt+Shift` into the window
    /// that already has the focus moved the layout to `ru-RU`, the probe of T-03-3c fired on the
    /// modifier release and read `en-US` — the layout on its way out — and nothing corrected it
    /// afterwards, six rounds out of six. Three earlier tasks (T-10-5, T-10-0f, T-10-13) each
    /// added an *occasion* to re-read; none of them changed the fact that the value is read
    /// ahead of the moment it is used.
    ///
    /// **The product always sets it**: [`install`] is the one path by which the input thread
    /// gets a buffer, and `app::restore_buffer` puts back the very same value it parked, so the
    /// probe travels with the recorder across the FR-70 gate — the same arrangement `verify`
    /// already relies on.
    stamp: Option<LayoutProbe>,
    /// **When the user last edited what they are typing — FR-14**, task Т-48-2. `None` while
    /// nothing has been typed since the last flush.
    ///
    /// "Edited" is a press that was recorded **or** a `Backspace` that took one out, and the
    /// two have to be one field rather than a reading of the ring, for the reason the user's
    /// second finding gives: «стираем набранный текст, набираем yandex.ru и нажимаем pause
    /// получаем `yaтвучюкг`». Erasing empties the ring; the suggestion list of the address bar
    /// closes with it and re-opens on the first stroke of the new word, raising the focus event
    /// FR-14 is about. A rule that read «the newest stroke in the ring» would see a user who had
    /// typed nothing and would let that event cut the word in half — which is exactly the
    /// measurement of 2026-09-09.
    ///
    /// It is **not** a second copy of the buffer and does not outlive it: [`Recorder::clear_ring`]
    /// clears it with the strokes, and [`Recorder::reset_up_to`] applies FR-12 to it on the same
    /// terms as to a stroke — an event at or after the edit covers it.
    ///
    /// NFR-01 to NFR-05: one `Option<u32>` written in the hook callback on the two paths that
    /// change the text, and read on the input thread's message loop. No allocation, no lock, no
    /// I/O, and nothing that can panic.
    last_edit: Option<u32>,
    /// **How long the ring may stand untouched before it is emptied, milliseconds — FR-15**,
    /// task T-52-4. Zero switches the rule off.
    ///
    /// Held in milliseconds because that is the unit of [`Stroke::time`] and of
    /// [`Recorder::last_edit`]: the configuration is in seconds, and converting once when the
    /// value is set is one multiplication instead of one on every check. The conversion cannot
    /// overflow — [`MAX_IDLE_TIMEOUT_S`] is a day, which is 86 400 000 milliseconds.
    idle_timeout_ms: u32,
}

impl Recorder {
    /// A buffer of `capacity` strokes — FR-07, with the value from `[buffer] capacity`.
    ///
    /// The number asked for is put through [`effective_capacity`], which is what a caller
    /// comparing "the buffer I have" against "the buffer the configuration asks for" has to
    /// use as well.
    ///
    /// This is the one allocation in the module and the only place there can ever be one:
    /// FR-07 says the ring is sized at creation, and everything after this runs in the hook
    /// callback where NFR-03 allows none.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            ring: Ring::with_capacity(effective_capacity(capacity)),
            cache: None,
            active: LayoutId::default(),
            active_index: None,
            held: Held::default(),
            converted: false,
            cycle: 0,
            verify: None,
            stamp: None,
            last_edit: None,
            // FR-15, task T-52-4. A recorder built without a configuration behaves as section 7
            // describes the program, which is what every other default in this constructor does.
            idle_timeout_ms: settings::Buffer::default().idle_timeout_s * MS_PER_SECOND,
        }
    }

    /// Gives this recorder a way to check its belief about the modifiers against the system —
    /// task T-10-12, and see [`Held::reconcile`] for what it is for.
    ///
    /// Set by [`install`] on the product's path. A test sets it to state, explicitly, what the
    /// keyboard is doing while the test feeds its synthetic strokes.
    pub fn verify_held_with(&mut self, probe: PhysicalProbe) {
        self.verify = Some(probe);
    }

    /// Gives this recorder a way to read the layout the user is really typing in — **the repair
    /// of defect E**, task T-10-14, and see [`Recorder::restamp`] for what it is for.
    ///
    /// Set by [`install`] on the product's path. A test sets it to state, explicitly, what the
    /// layout of the foreground window is while the test feeds its synthetic strokes.
    pub fn stamp_layout_with(&mut self, probe: LayoutProbe) {
        self.stamp = Some(probe);
    }

    /// A buffer sized by the `[buffer]` section of the configuration — FR-07.
    ///
    /// The capacity is *read*, not hard-wired: the constant of FR-07 lives in section 7 as a
    /// default, and this is the path the product takes.
    pub fn from_config(config: &settings::Buffer) -> Self {
        let mut recorder = Self::with_capacity(config.capacity);

        // FR-15, task T-52-4.
        recorder.set_idle_timeout_s(config.idle_timeout_s);

        recorder
    }

    /// Sets `[buffer] idle_timeout_s` — **FR-15**, task T-52-4.
    ///
    /// The value is put through [`effective_idle_timeout_s`], so a caller comparing "the
    /// timeout I have" against "the timeout the configuration asks for" reads the same number
    /// this stores — the rule [`effective_capacity`] already establishes for the capacity.
    ///
    /// Cheap enough to call on every message of the input thread, which is what
    /// `app::apply_configured_buffer` does: one comparison and, at most, one store.
    pub fn set_idle_timeout_s(&mut self, seconds: u32) {
        self.idle_timeout_ms = effective_idle_timeout_s(seconds) * MS_PER_SECOND;
    }

    /// The idle timeout in force, seconds — FR-15.
    pub fn idle_timeout_s(&self) -> u32 {
        self.idle_timeout_ms / MS_PER_SECOND
    }

    /// How many strokes fit.
    pub fn capacity(&self) -> usize {
        self.ring.capacity()
    }

    /// Resizes the ring through the general flush, keeping the cache, the layout and the settings
    /// this recorder knows — FR-07.
    ///
    /// # Why a resize exists at all
    ///
    /// `[buffer] capacity` is read from a file by the UI thread, and the input thread must not
    /// wait for it: NFR-08 gives the hook fifty milliseconds from start-up. So the buffer is
    /// installed on the default of section 7 and the configured value arrives afterwards —
    /// usually within the same few milliseconds, but the UI thread is also talking to the shell
    /// about a tray icon, and that can take its time.
    ///
    /// Re-installing the whole recorder would answer that too, and would throw away the cache
    /// of FR-20 and the layout of FR-04 with it — that is, it would pay for a second sweep of
    /// every layout in the session in order to change one number. This changes the one number.
    ///
    /// The old ring is dropped, which overwrites it with zeroes (SEC-02, [`Ring::drop`]), so
    /// whatever was in the buffer is gone. That is a property of resizing an array and not a
    /// row of the FR-10 table, it happens once and at start-up, and it is **not** the case FR-11
    /// speaks about: a layout change flushes nothing and goes through
    /// [`Recorder::set_active_layout`], which touches no slot at all.
    ///
    /// ⭐ **Since task T-39-2 (finding Т3) the strokes go the way every flush sends them** —
    /// through [`Recorder::reset`], before the ring is replaced — so that what a flush clears is
    /// one rule rather than two copies of it: the position counter of FR-32, the conversion
    /// session of the last row of FR-10 and the memory of the last edit (FR-14, FR-15) go with
    /// the strokes. What the resize keeps is what it was written to keep: the cache of FR-20, the
    /// layout of FR-04, both probes and the idle timeout.
    ///
    /// Allocates, exactly once, like [`Recorder::with_capacity`] — so it is called from the
    /// input thread's message loop and never from the callback (NFR-03).
    pub fn set_capacity(&mut self, capacity: usize) {
        // ⭐ **Finding Т3 — task T-39-2: the general flush first, so that what a flush clears is
        // written once for everyone.** Until this task the resize zeroed the position counter by
        // its own hand and left the rest standing: the conversion session of the last row of
        // FR-10 and the memory of the last edit — FR-14, and the idle clock of FR-15 that reads
        // it — outlived the strokes they were about. `reset` reaches `clear_ring`, which zeroes
        // the old ring (SEC-02), zeroes the counter of FR-34 and publishes that zero to the mirror
        // of SEC-04a, forgets the edit, and then ends the session.
        //
        // The mirrors still agree afterwards, which is what task T-04-3-3 closed: `clear_ring`
        // publishes the counter's zero, and the new ring below publishes its length's zero
        // through `set_len`, so no snapshot can show `buffer_len=0` beside a stale
        // `cycle_position`.
        self.reset();

        self.ring = Ring::with_capacity(effective_capacity(capacity));
    }

    /// How many strokes are live. The one number SEC-04a allows the debug channel to publish.
    pub fn len(&self) -> usize {
        self.ring.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.ring.len() == 0
    }

    /// Publishes the mapping cache of FR-20, and rebuilds of FR-21 with it.
    ///
    /// Called from the input thread, outside the callback, by whoever owns the building of the
    /// cache. Until it is called the buffer records strokes with no characters, which is the
    /// same outcome as a key the layouts have nothing on.
    pub fn set_cache(&mut self, cache: LayoutCache) {
        self.cache = Some(cache);
        self.resolve_active();
    }

    /// Whether a cache has been published.
    pub fn has_cache(&self) -> bool {
        self.cache.is_some()
    }

    /// The mapping cache of FR-20 as it stands — `None` until one has been published.
    ///
    /// # Why this accessor exists — NFR-09
    ///
    /// The choice of the target layout (section 4.4) needs two things the cache holds: the list
    /// of participating layouts, which is what FR-30 means by "первые две раскладки системного
    /// списка", and the map of the layout it chose, which is what the conversion of FR-22 is
    /// driven by. Both are here already, built once at start-up and rebuilt only on the messages
    /// of FR-21.
    ///
    /// Without a way to read them, the hotkey path would have to build a map of its own on every
    /// press — the sweep of FR-20 is thousands of `ToUnicodeEx` calls and about five
    /// milliseconds on this machine, against the thirty NFR-09 gives the *whole* path from the
    /// press to the last event of the replacement. So this is the accessor task T-05-2 needed
    /// and the reason the interim of task T-04-1 could be removed: the target now comes out of
    /// the live cache, and the hardwired table of FR-25 goes back to being what FR-25 says it
    /// is — the emergency reserve, reached when the cache could not be built at all.
    ///
    /// Reading it never leaves the process: a `LayoutMap` is a keyboard layout, not a recording
    /// of anybody's typing, and nothing here is formatted (SEC-01, SEC-07).
    pub fn cache(&self) -> Option<&LayoutCache> {
        self.cache.as_ref()
    }

    /// **The position counter of FR-32** — how many presses of the hotkey the text on the screen
    /// is away from what the buffer holds.
    ///
    /// `0` is "as typed", and it is what every flush of FR-10 leaves behind (FR-34).
    pub fn cycle_position(&self) -> usize {
        self.cycle
    }

    /// Advances the counter along a cycle of `len` layouts and answers the new position —
    /// FR-31, FR-32.
    ///
    /// Called once per press of the hotkey, by the code that has just decided a replacement will
    /// really happen. `len` comes from [`crate::layouts::Cycle::len`]; the counter is reduced by
    /// it here, which is what keeps it a position in `0..len` rather than a total that would one
    /// day have to wrap. A `len` of zero — no cycle at all — leaves the counter at zero, since
    /// there is nothing to be a position in.
    ///
    /// One addition, one remainder, one store. No allocation, no lock, no I/O: NFR-01 to NFR-05
    /// hold here as they do everywhere else in this module.
    ///
    /// Under the `testing` feature the new position is published into [`crate::control`] by
    /// [`Recorder::set_cycle`], the one writer of the counter (task T-69-3).
    pub fn advance_cycle(&mut self, len: usize) -> usize {
        self.set_cycle(if len == 0 { 0 } else { (self.cycle + 1) % len });

        self.cycle
    }

    /// The one place the position counter is written — and, under the `testing` feature, the one
    /// place its mirror of SEC-04a is published from. Task **T-69-3**, backlog line Э39-Б-1.
    ///
    /// # Why every write goes through here
    ///
    /// For the reason [`Ring::set_len`] gives for the length, and it is the same problem: the
    /// channel of SEC-04a reports `cycle_position`, the acceptance bench of section 11.5 takes
    /// that number for the truth, and a stale number is worse than an absent one. Until this task
    /// the counter was written in three places — [`Recorder::advance_cycle`],
    /// [`Recorder::clear_ring`] and the partial arm of [`Recorder::reset_up_to`] — and each of
    /// them wrote its publication out by hand, so a fourth write that forgot would have made the
    /// mirror a liar without a sound. Now the field cannot be written without the publication,
    /// and `tests\buffer.rs`, `the_position_counter_has_one_writer_and_that_writer_publishes`,
    /// sweeps the module for a write that goes around it.
    ///
    /// ⚠ **The struct literal of [`Recorder::with_capacity`] is the one write that does not pass
    /// here, and on purpose.** Routed back through this setter the way `Ring::with_capacity`
    /// routes its `len`, it would add a publication: every new recorder would zero a mirror that
    /// describes the process rather than the value being built. The setter changed how the
    /// counter is written and not how often the mirror moves — it moves in exactly the cases it
    /// moved before.
    ///
    /// # NFR-01 to NFR-05
    ///
    /// Reached from the hotkey path and from inside the hook callback, through the flush of
    /// FR-10. What the feature adds to either path is one relaxed atomic store — no allocation
    /// (NFR-03), no lock (NFR-04), no I/O (NFR-05); in a build without the feature — every
    /// Release build, by SEC-04a condition 1 — this is a plain field assignment.
    fn set_cycle(&mut self, position: usize) {
        self.cycle = position;

        #[cfg(feature = "testing")]
        crate::control::note_cycle_position(position);
    }

    /// Publishes the layout strokes are recorded under — the `hkl` of FR-04.
    ///
    /// **FR-11: this does not flush the buffer, and must never be made to.** A user switching
    /// layout in the middle of a word is not a signal that what came before is void; it is the
    /// case the per-stroke `hkl` exists for. The temptation is to hang a flush on
    /// `WM_INPUTLANGCHANGE`, which is also the message FR-21 rebuilds the cache on, and FR-11
    /// forbids it in as many words.
    pub fn set_active_layout(&mut self, layout: LayoutId) {
        self.active = layout;
        self.resolve_active();
    }

    /// The layout strokes are being recorded under.
    pub fn active_layout(&self) -> LayoutId {
        self.active
    }

    /// Seeds `CapsLock` from a caller that has a trustworthy source for it.
    ///
    /// The tracker follows every `CapsLock` press the hook sees, so it is right from the first
    /// toggle onwards; it cannot know the state the machine was already in when the program
    /// started. Nothing in this module asks the system, and see [`Held`] for why.
    ///
    /// ⚠ **Task T-13-4 wired this up.** It was written by task T-03-2, documented, and then
    /// called from nowhere in the product, which is the whole of the defect the audit of
    /// 2026-08-24 recorded: a program started with `CapsLock` on — the autostart of FR-93 among
    /// other ways — recorded every stroke of the session under a cleared `CAPS` bit, keyed the
    /// cache of FR-20 with it and injected the replacement of FR-22 in the wrong case, because
    /// `KEYEVENTF_UNICODE` carries the character literally and knows nothing of the real
    /// `CapsLock`. The callers are named at [`crate::buffer::set_caps_lock`], the module-level
    /// function of the same name that drives this one; the setter itself is unchanged.
    ///
    /// **A seed, not a toggle.** The value is *assigned*, so seeding twice in a row is the same
    /// as seeding once, and a point that fires when nothing was missed writes back what was
    /// already there. That is what lets the five points be as generous as they are.
    pub fn set_caps_lock(&mut self, on: bool) {
        self.held.caps = on;
    }

    /// The modifiers held right now, as the mask of FR-04 minus the per-key `Extended` bit.
    ///
    /// Offered for step 3 of FR-40 (task T-04-1), which has to release what the user is
    /// holding before it injects anything.
    pub fn held(&self) -> StrokeMods {
        self.mods_now(0)
    }

    /// Records that a conversion has run — the last row of FR-10.
    ///
    /// Called by whoever performs the conversion (tasks T-04-1 and T-05-2) once the
    /// replacement is on its way. The buffer is deliberately **not** flushed here: FR-32 keeps
    /// the original strokes so that the next press of the hotkey renders the same scan codes
    /// into the next layout of the cycle. It is the next *key* that ends the session, and
    /// [`Recorder::record`] flushes then.
    pub fn note_conversion(&mut self) {
        self.converted = true;
    }

    /// Whether a conversion session is open — the state the next key ends.
    pub fn in_conversion(&self) -> bool {
        self.converted
    }

    /// The whole rule set of FR-10 and the recording of FR-04, for one keystroke.
    ///
    /// Called from [`crate::hook::classify`], which means: inside the hook callback, after
    /// FR-99, FR-03 and FR-90 have had their say and before anything else. What it is allowed
    /// to do is what the callback is allowed to do:
    ///
    /// * **NFR-01, NFR-02.** Comparisons, a lookup that is a multiplication and an array read
    ///   (FR-06), and a bounded number of stores. The one path that is not constant time is a
    ///   flush, which writes the whole ring — eight kilobytes at the capacity of FR-07, and it
    ///   does not depend on the stroke.
    /// * **NFR-03.** No allocation: the ring exists already and every value here is `Copy`.
    /// * **NFR-04.** No lock, no atomic, no channel. The recorder belongs to the input thread
    ///   (section 6.3).
    /// * **NFR-05.** No I/O, no journalling, nothing formatted.
    ///
    /// The order of the rules is the order of the FR-10 table, with two decisions of its own:
    /// a modifier key is handled first, because holding `Ctrl` is not yet a command and
    /// `Shift` is not text; and the conversion session of the last row is ended by the first
    /// key that is not a modifier, so that `Shift` and then a letter is one session end and
    /// not two.
    pub fn record(&mut self, key: KeyEvent) -> Recorded {
        if let Some(role) = modifier_role(key.vk) {
            self.held.apply(role, key.edge);
            return Recorded::Modifier;
        }

        // ⭐ **The seven keys of task T-52-1, and they stand exactly here.** The row is beside
        // the modifier row because it is the same kind of row: a *role*, decided before
        // anything about characters or commands is asked. Standing here it also answers for
        // both edges at once and for a media key pressed while a modifier is held — which is
        // deliberate. `Ctrl` plus the volume key is not a command over the text either; the
        // user's decision names the seven keys without conditions, and a condition invented
        // here would be one this file cannot justify. See [`MEDIA_KEYS`].
        if is_media_key(key.vk) {
            return Recorded::Ignored;
        }

        // FR-04 records presses. A release adds nothing to what was typed, and recording both
        // edges would double every character in the buffer.
        if matches!(key.edge, Edge::Up) {
            return Recorded::Ignored;
        }

        let mut mods = self.mods_now(key.flags);

        // ⚠ **The command row of FR-10, computed here and applied further down** — hoisted out
        // of its `if` by task T-10-12 so that the check below can be gated on it without
        // evaluating it twice. Moving it changes nothing: the only statement between here and
        // the row is the conversion-session flush, which touches neither `held` nor `mods`.
        let mut command = (mods.ctrl() || mods.alt() || self.held.win()) && !mods.altgr();

        // ---------------------------------------------------------------------------------
        // ⭐ **Defect D — the repair.** Task T-10-12.
        //
        // `self.held` is the program's *belief*, built from the hook stream and from nothing
        // else. It is right only while the stream is complete, and task T-10-12 measured that
        // it is not always complete: pressing `Win+E`, clicking in the search box of the
        // Explorer window that opens and typing there killed the buffer of a live installation
        // outright — every keystroke thrown away, in every application, for six and a half
        // minutes, until a human pressed and released `Win` and it recovered in the same
        // second. The release of that `Win+E` never reached the callback at all. One missed
        // release, and the `Win` bit of `held` stays raised for the life of the process.
        //
        // What makes it the worst kind of defect is that nothing shows: the callback is alive
        // and counting, no flush counter moves — the ring is cleared *here*, on a path that
        // never touches them — and every health indicator the product publishes says it is
        // well. Three tasks searched for it in the wrong place.
        //
        // **The check runs only on the row that is about to throw the stroke away.** Ordinary
        // typing believes no command modifier is down, `command` is false, and the branch is
        // not taken: a letter costs the boolean it was going to cost anyway (NFR-01, NFR-02).
        // It is the shape module `hook` already uses for FR-96 — the cheap test first, the
        // system call only once the answer is "yes".
        //
        // **The exception of FR-11 is covered by the same gate**, and deliberately so. A
        // believed `Win+Space` implies `command`: `held.win()` alone puts the row true, and
        // `altgr` cannot be set without `Ctrl`, which `Win+Space` does not have. So the
        // reconciliation below happens *before* the FR-11 test reads `held.win()`, and both rows
        // decide on the same corrected belief rather than on two different ones.
        //
        // ⚠ **This does not touch FR-10 or FR-11 themselves** (decision Р-44, the user's
        // decision on question 43). When the user really is holding the key, the system says so,
        // nothing is lowered and both rows answer exactly as they always did — which is what
        // `the_physical_check_leaves_fr10_and_fr11_alone_when_the_key_really_is_held` pins.
        // [`Held::reconcile`] can only ever *lower* a belief, so no stroke that used to be text
        // can become a command; only the reverse, which is the defect.
        //
        // ⛔ **The reinstallation of the hook is not involved and is not touched.** Task T-10-12
        // measured the gap at two microseconds once every thirty seconds and showed the lost
        // release did not fall in one. `watchdog` is deliberately unchanged by this repair.
        // ---------------------------------------------------------------------------------
        if command
            && let Some(probe) = self.verify
            && self.held.reconcile(probe())
        {
            mods = self.mods_now(key.flags);
            command = (mods.ctrl() || mods.alt() || self.held.win()) && !mods.altgr();
        }

        // ⚠ **FR-11, and it stands in front of every flush row below — decision Р-44.**
        //
        // FR-11 names two combinations — `Alt+Shift` and `Win+Space` — and says a layout switch
        // "**не сбрасывает** буфер: HKL хранится по каждому нажатию отдельно". `Alt+Shift` never
        // reaches this line: it is modifiers alone and the first statement of this function has
        // already answered `Modifier` for both halves of it, which task T-03-3b measured rather
        // than assumed. `Win+Space` does reach it, because `Space` is not a modifier.
        //
        // **Why the check moved here, above the two rows that flush.** The user's decision on
        // question 43 (commit e6ba407) put this exception inside the "Ctrl/Alt/Win + клавиша"
        // row, where it answered that row alone. But `Win+Space` pressed **right after a
        // conversion** also falls under the *last* row of the table, "любая клавиша после
        // конвертации — полный сброс", and that row cleared the ring before the exception was
        // ever consulted. The user who converted a word, noticed the layout was still wrong,
        // pressed `Win+Space` and reached for the hotkey again found the buffer gone — that is,
        // lost exactly the rollback of FR-33 that the counter below exists to provide. The text
        // of FR-11 is unconditional and carries no "кроме как после конвертации"; decision
        // Р-44 says so and gives the repair to this task.
        //
        // The exception stays as narrow as the requirement is: `Space`, with `Win` held, and
        // with neither `Ctrl` nor `Alt` in the combination — `Ctrl+Win+Space` and `Alt+Win+Space`
        // switch no layout and remain commands. Everything else keeps the behaviour it had:
        // `Win+R` is a command and flushes, and a bare `Space` with no modifier at all is the
        // boundary key of the first row of the FR-10 table — the soft one since task Т-24-2,
        // answered further down and not here.
        //
        // Nothing is recorded either, and the conversion session is left open. The return is
        // `Ignored` and not a fall-through, because the stroke is a switch the system consumes:
        // it puts no space into the text, and pushing one into the ring would make the
        // conversion of FR-22 produce a character the user never typed. The position counter of
        // FR-32 stays where it was for the same reason the strokes do — a layout change is not a
        // flush, so FR-34 has nothing to say about it.
        if key.vk == VK_SPACE.0 && self.held.win() && !mods.ctrl() && !mods.alt() {
            return Recorded::Ignored;
        }

        // ---------------------------------------------------------------------------------
        // ⭐ **The rest of FR-11 — task T-52-2**, and it stands beside the row above because it
        // is the same requirement answered for the combinations the code had missed.
        //
        // FR-11 is about **the user switching the layout themselves**, and the Windows language
        // dialog offers more ways to do that than the two the code knew. Beside "switch to the
        // next layout" — `Alt+Shift` and `Ctrl+Shift`, which are modifiers alone and never
        // reach this function — the same dialog binds **"switch to a particular language"** to
        // `(Ctrl | Alt) + Shift + <top-row digit>`, and to the grave key in the same list. Every
        // one of those arrived here as `Ctrl`-or-`Alt` plus a key, which is exactly the shape of
        // the command row below, and threw the word away. FR-11 says a switch does not flush,
        // and it says it without exceptions.
        //
        // **The shape of the combination, and every half of it is deliberate:**
        //
        // * **`Ctrl` or `Alt`, but never both.** `Ctrl` together with `Alt` is `AltGr` — a
        //   *symbol* modifier of FR-20, the one combination that produces characters — and the
        //   dialog binds one modifier or the other, never the pair. `!=` on the two booleans is
        //   the whole of it;
        // * **`Shift`, either side.** Without it `Ctrl+1` and `Alt+1` are ordinary application
        //   commands and stay commands;
        // * **no `Win`.** `Win+Ctrl+Shift+1` switches no layout and is a command like any other;
        // * **the top row only.** `VK_NUMPAD0`…`VK_NUMPAD9` are different physical keys and the
        //   dialog does not bind them, so [`is_layout_switch_key`] takes the top row and the
        //   grave key and nothing else.
        //
        // ⚠ **`Alt+Shift+<цифра>` arrives as `WM_SYSKEYDOWN`, and that is why the mask is read
        // rather than the tracker.** [`Recorder::mods_now`] believes `LLKHF_ALTDOWN` — the
        // system's own answer for this very event — alongside `held`, so an `Alt` that went
        // down before the hook was installed is still seen here.
        //
        // **What it does is what `Win+Space` does, and for the same reasons.** Nothing is
        // recorded — the stroke puts no character on the screen, and a stroke in the ring would
        // make FR-22 produce a character the user never typed. The position counter of FR-32 is
        // not touched: a layout change is not a flush, so FR-34 has nothing to say about it. And
        // the conversion session is left **open**, which is the whole point of standing above
        // the last row of the table (Р-44): the user who converted a word, saw the layout was
        // still not the one they meant, switched to it by its number and reached for the hotkey
        // again must still have the rollback of FR-33.
        //
        // ⚠ **The bare grave key is not here and cannot be** — section 10. The dialog lets the
        // user bind it with no modifiers at all, and from inside the hook that stroke is
        // indistinguishable from typing the character. We read it as the character, because
        // that is what it is for everybody who has not bound it; asking the registry which it
        // is was refused with the rest of the registry reading.
        //
        // **NFR-01 and NFR-02.** For an ordinary letter the first test — `Shift` in the mask —
        // is usually false and the row ends there; a capital letter pays the second, which is
        // two boolean reads. No allocation (NFR-03), no lock (NFR-04), no I/O (NFR-05), no
        // Win32 call (NFR-02).
        // ---------------------------------------------------------------------------------
        if mods.shift()
            && (mods.ctrl() != mods.alt())
            && !self.held.win()
            && is_layout_switch_key(key.vk)
        {
            return Recorded::Ignored;
        }

        // FR-10, last row: "любая клавиша после конвертации — полный сброс (завершение сессии
        // конвертации)". The flush happens here and the key itself is then treated normally,
        // so the first letter of the next word is the first stroke of the next buffer.
        if self.converted {
            self.converted = false;
            self.clear_ring();
        }

        // FR-10: "Ctrl/Alt/Win + клавиша, **кроме комбинаций смены раскладки, названных
        // FR-11** — полный сброс (команда, а не текст)". `AltGr` is the exception the mask
        // itself carries: it is `Ctrl` plus the right `Alt` by construction and it produces
        // characters, so a layout that puts text on it keeps working. The other exception, the
        // `Win+Space` of FR-11, has been answered above.
        if command {
            self.clear_ring();
            return Recorded::Flushed;
        }

        // FR-10: `Backspace` takes one element out and does **not** flush. It is the one key in
        // the table whose effect on the text the buffer can follow exactly.
        //
        // ⭐ **It is the tail's exception too, and by construction rather than by a branch** —
        // task Т-24-2. «слово␣» + `Backspace` is «слово»: the pop takes the space out, the last
        // stroke is a letter again, and the state below is closed without anybody closing it.
        if key.vk == VK_BACK.0 {
            return if self.ring.pop() {
                // **FR-14** — task Т-48-2, and the reason the memory is not simply "the newest
                // stroke in the ring". Erasing takes strokes *out*: «стираем набранный текст,
                // набираем yandex.ru» leaves an empty ring behind, and the focus event the next
                // keystroke provokes would find no evidence that the user had been editing at
                // all. A pop is an edit and is remembered as one.
                self.last_edit = Some(key.time);
                Recorded::Popped
            } else {
                Recorded::Ignored
            };
        }

        if flushes(key.vk) {
            self.clear_ring();
            return Recorded::Flushed;
        }

        // ---------------------------------------------------------------------------------
        // ⭐ **The soft boundary of FR-10** — task Т-24-2, the user's decision on question 82.
        //
        // The complaint, word for word: «часто набираю слово, потом ставлю пробел, а только
        // потом вижу что выбрана не та раскладка и нажимаю `Pause`, но переключения уже не
        // происходит». It could not happen. `Space` sat in [`BOUNDARY_KEYS`] and the line above
        // threw the word away on it, so by the time the hotkey was pressed there was nothing
        // left to convert — and a word is noticed to be in the wrong layout **after** it has
        // been separated from the next one far more often than before.
        //
        // FR-10 now divides the boundaries in two. The three hard ones flush above. `Space` is
        // the one soft boundary, and the whole of it is these two arms:
        //
        // * **it writes.** The stroke falls through to the recording below and becomes an
        //   ordinary member of the ring, so the buffer stands in «слово + хвост» and the hotkey
        //   converts both — the space bar gives a space in the target layout too, so FR-22
        //   carries the tail over by the ordinary rule and no exception is needed anywhere on
        //   the conversion path. Further spaces fall through the same way and pile up;
        // * **the next key that writes and is not a space flushes first, then records.** That
        //   is the `else if` below, and it is the deferred half of the boundary: the old word
        //   dies exactly when a new one begins.
        //
        // **An empty ring knows no tail.** With nothing to hold, the first arm keeps the
        // behaviour the key always had — the flush, and the zeroing that goes with it. It
        // matters more than it looks: the last row of the table empties the ring on the first
        // key after a conversion, so a space pressed there finds an empty ring and opens no
        // state over what it has just emptied.
        //
        // **Why the state is read off the ring and not kept in a flag.** A flag would be a
        // second copy of something the ring already knows, and every flush in this file would
        // have to remember to clear it — `clear_ring`, `reset_up_to`, the pop of `Backspace`,
        // the eviction of FR-07, `app::park_buffer` across the FR-70 gate. The reading below
        // cannot fall out of step with any of them: a hard flush empties the ring, and an empty
        // ring has no last stroke; `Backspace` takes the space out and the answer changes with
        // it; eviction drops the **oldest** stroke and never touches the newest. The state of
        // the soft boundary is therefore not state at all, which is what keeps SEC-02 and FR-12
        // exactly where task Т-13-7 and task T-03-4 left them.
        //
        // ⚠ **A stroke of `VK_SPACE` can only have come from here.** The `Win+Space` of FR-11
        // returns `Ignored` above and stores nothing, and `Ctrl`/`Alt`+`Space` is a command and
        // flushes above; this is the one path by which a space reaches the ring at all, which is
        // what makes reading the last stroke a sound test for «хвост открыт».
        //
        // **NFR-01 and NFR-02.** One comparison for every key that is not a space, and for the
        // key that ends a tail, the flush that was going to happen on the previous keystroke
        // anyway. Nothing is added to the path of an ordinary letter but the comparison.
        // ---------------------------------------------------------------------------------
        if key.vk == VK_SPACE.0 {
            if self.ring.len() == 0 {
                self.clear_ring();
                return Recorded::Flushed;
            }
        } else if self.tail_is_open() {
            self.clear_ring();
        }

        // ---------------------------------------------------------------------------------
        // ⭐ **`VK_PACKET` — task T-52-1, and it is the one row of FR-10 that never asks what
        // the key produced.**
        //
        // `VK_PACKET` is not a key. It is how the system reports an event sent with
        // `KEYEVENTF_UNICODE`: a password manager filling a field, a second switcher replacing
        // a word, an AutoHotkey script expanding an abbreviation. The text on the screen has
        // just changed by characters this program cannot decode, so whatever the ring holds no
        // longer describes what is in the field — and an FR-22 conversion over a buffer that
        // has drifted from the screen sends `Backspace` into somebody else's text. That is the
        // same reason the right mouse button flushes.
        //
        // ⚠ **And the lookup below would not merely be useless on it — it was wrong.** In a
        // `KEYEVENTF_UNICODE` event `wScan` carries the **UTF-16 code unit of the character**,
        // not a scan code, and [`Recorder::lookup`] read it as one. `!` is U+0021, scan 0x21 is
        // the `F` key, and the ring gained an `f` nobody had typed. So the row stands before
        // the lookup, and skipping the lookup is the point rather than an optimisation.
        //
        // ⚠ **Our own injection of FR-22 arrives as `VK_PACKET` too, and never gets here.**
        // [`crate::hook::classify`] drops a stroke carrying `INJECTED_SIGNATURE` before the
        // buffer is reached — FR-03, and the signature rather than `LLKHF_INJECTED` is what
        // tells ours from everybody else's. Pinned by
        // `our_own_unicode_injection_never_reaches_the_recorder` in `tests\buffer.rs`, because
        // without that pin this row would empty the buffer the program had just refilled.
        //
        // **Where it stands.** After the conversion-session row and the command row above, so
        // that a session is closed exactly once and `Ctrl`+anything is still a command; before
        // the lookup, for the reason above. One comparison on the path of an ordinary letter.
        // ---------------------------------------------------------------------------------
        if key.vk == VK_PACKET.0 {
            self.clear_ring();
            return Recorded::Flushed;
        }

        // ---------------------------------------------------------------------------------
        // ⭐ **Defect E — the repair.** Task T-10-14.
        //
        // The layout is read **here**, where it is used, instead of being believed from a value
        // somebody read earlier on an event. Everything below this line — the decoding of FR-06
        // and the stamp of FR-04 that FR-26 takes its direction from — then rests on what the
        // foreground window is running **now**, at the moment these strokes are being typed.
        //
        // ⚠ **Why the earlier value could not be trusted.** `Recorder::active` is written from
        // `app::publish_active_layout` when an event says the layout may have moved. Task
        // T-10-14 measured that the event outruns the switch: a synthetic `Alt+Shift` into the
        // window that already had the focus moved it to `ru-RU`, the probe of T-03-3c fired on
        // the modifier release, read `en-US`, and nothing corrected it — six rounds out of six,
        // `layout_probes` rising every time and `focus_changes` never moving. The next word was
        // then decoded through the `en-US` map and converted `en-US → ru-RU`, producing the text
        // that was already on the screen. That is the «моргание» three earlier tasks each
        // answered by adding one more *occasion* to re-read.
        //
        // **On the first stroke of a new word and on nothing else.** The ring is empty exactly
        // when a word is starting — every flush row of FR-10 above has already run — and the
        // stamp only has to be right for strokes that are about to be recorded. Typing `ghbdtn`
        // costs the three Win32 reads once, on the `g`; the other five keys take the branch not
        // taken and pay a comparison. It is the shape module `hook` already uses for FR-96 and
        // the repair of defect D: the cheap test first, the system call only once the answer is
        // "yes".
        //
        // ⚠ **FR-11 is untouched, and this is not a flush** (Р-44, the user's decision on
        // question 43). A layout change still throws nothing away; `Alt+Shift` still never
        // reaches this line at all, `Win+Space` still returns `Ignored` above, and `Win+R` still
        // flushes as a command. Nothing here empties the ring — it cannot, the branch runs only
        // when the ring is already empty.
        // ---------------------------------------------------------------------------------
        // ⭐ **Task T-10-15 gives this branch a voice, and changes nothing about it.** The five
        // outcomes of [`Recorder::restamp`] — this `else`, and the four exits of the function
        // itself — were indistinguishable from outside the process: four of the five leave the
        // stamp exactly as it was, so `active_layout` reads the same for all of them. See
        // [`crate::control::RESTAMP_OUTCOMES`].
        //
        // ⚠ **The `else` is a count and not a decision.** The condition, the call and everything
        // either of them does are the lines task T-10-14 left here; the arm below adds one relaxed
        // `fetch_add` on the path a keystroke that is *not* the first of a word takes, compiled
        // out of every build that is not a `testing` one — the same terms `note_buffer_len` has
        // been called under from this file since task T-03-4.
        if self.ring.len() == 0 {
            self.restamp();

            // ---------------------------------------------------------------------------------
            // ⭐ **Finding С8 — task T-39-3, decision 122.1: a lost `Shift` release comes
            // down once per word.**
            //
            // The check of defect D above asks the system only on the command row, and a
            // `Shift` alone never reaches that row: a release lost over another window left
            // every stroke afterwards recorded «with Shift» — `ghbdtn` converting into
            // «ПРИВЕТ» — for the rest of the session. The first stroke of a word is where
            // this function already pays a Win32 read (the restamp above), so the `Shift`
            // half of the belief is checked here too.
            //
            // ⛔⛔ **The order decides the outcome.** `mods` was computed at the top of this
            // function; lowering the belief without recomputing it would record the first
            // letter of the word as a capital all the same. So `mods` is recomputed here,
            // before `lookup` below, as the command row recomputes it after its own check.
            //
            // **NFR-01, NFR-02: the cheap test first.** A stroke with no `Shift` believed held
            // costs one boolean; the system is asked only when the belief says a `Shift` is
            // down — a capital first letter, or a stuck one. And only `Shift`: see
            // [`Held::reconcile_shift`] for why `Ctrl` and `Alt` are the command row's.
            //
            // ⚠ **FR-11 stands above this point and is not reached from here.** A `Shift`
            // really held for `Ctrl+Shift+<цифра>` reads as held and nothing is lowered; a
            // stuck one meets the command row's check first, since [`Held::reconcile`]
            // covers `Shift` as well.
            // ---------------------------------------------------------------------------------
            if self.held.shift()
                && let Some(probe) = self.verify
                && self.held.reconcile_shift(probe())
            {
                mods = self.mods_now(key.flags);
            }
        } else {
            #[cfg(feature = "testing")]
            crate::control::note_restamp(crate::control::Restamp::Skipped);
        }

        // FR-06 through the cache of FR-20, and FR-04: what the key gave, at the moment it was
        // pressed, in the layout that was active then.
        let produced = self.lookup(key.scan, mods);

        // ---------------------------------------------------------------------------------
        // ⭐ **The empty stroke — task T-52-1, the last row of FR-10 to be added and the one
        // that finally makes the table a rule instead of a list.**
        //
        // Until this task everything that was not a modifier, not a command, not `Backspace`,
        // not a space and not one of the thirteen keys of [`BOUNDARY_KEYS`] and
        // [`EDITING_KEYS`] went **into the ring**. `F5` reloaded the page and was recorded as
        // the seventh letter of the word; `F2` renamed the file, the browser's `Back` key left
        // the page, and the `Application` key opened the very menu the right mouse button opens
        // — which the program already treats as a boundary (`watchdog::BUTTON_DOWN_BITS`). The
        // buffer went on living over text that was no longer there, and the next conversion
        // sent N `Backspace` into somebody else's field.
        //
        // A list of key codes would have to be maintained for ever and would be wrong on the
        // next keyboard. [`Recorder::stroke_writes_nowhere`] asks the question the list was
        // approximating: **did this stroke write anything, anywhere?** A key that writes is
        // text and belongs in the ring; a key that writes nowhere and has no role of its own in
        // the table is an action, and an action ends the word.
        //
        // **NFR-01 and NFR-02, and this is the whole cost argument.** For an ordinary letter
        // `produced` carries a character, the first test of `stroke_writes_nowhere` is false and
        // the function returns — one comparison, nothing else, and the other layouts of the
        // cache are never consulted. Only a stroke that came back empty from the active layout
        // pays the scan of the cache, and such a stroke is by definition not part of a word.
        // No allocation (NFR-03), no lock (NFR-04), no I/O (NFR-05), no Win32 call (NFR-02).
        //
        // **The flush is the ordinary one**: `clear_ring` zeroes the slots (SEC-02), resets the
        // position counter of FR-32 and — FR-14, task Т-48-2 — forgets the moment of the last
        // edit, because a flush is the opposite of editing and an exemption resting on typing
        // the program has just thrown away would be an exemption for nothing.
        // ---------------------------------------------------------------------------------
        if self.stroke_writes_nowhere(produced, key.scan, mods) {
            self.clear_ring();
            return Recorded::Flushed;
        }

        let stroke = Stroke::new(key.vk, key.scan, mods, self.active, key.time, produced);

        // **FR-14** — task Т-48-2. The moment of the edit, beside the ring. One store of an
        // `Option<u32>` on the path of a key that writes, no allocation (NFR-03), no lock
        // (NFR-04), no I/O (NFR-05); see [`Recorder::last_edit`] for why the ring alone could
        // not answer the question.
        self.last_edit = Some(key.time);

        if self.ring.push(stroke) {
            Recorded::Evicted
        } else {
            Recorded::Stored
        }
    }

    /// **Whether the user was editing in the window `(event_time − delta, event_time]`** — the
    /// whole test of **FR-14**, task Т-48-2.
    ///
    /// `true` means: a stroke of theirs was recorded, or a `Backspace` of theirs took one out,
    /// at a moment no later than the event and no earlier than `delta` before it. That is what
    /// makes an `EVENT_OBJECT_FOCUS` of the foreground window the application's answer to the
    /// typing rather than the user moving to another field — see `watchdog::take_typing_induced_flush`,
    /// which is the one caller and which owns the rest of the rule.
    ///
    /// # Why both halves are asked
    ///
    /// [`Recorder::last_edit`] holds the **newest** edit, which is the ordinary answer and the
    /// cheap one. It is not the whole answer: an event can arrive stamped between two strokes —
    /// the user goes on typing while the message crosses to the input thread — and then the
    /// newest edit is *after* the event while an older stroke sits squarely inside the window.
    /// The ring is asked in that case, and [`Ring::any_within`] answers it.
    ///
    /// # Cost
    ///
    /// One comparison in the ordinary case. The ring is walked only when the newest edit is
    /// outside the window, which is once per focus event at most — this runs on the input
    /// thread's message loop and never in the hook callback (NFR-01 to NFR-05); nothing here
    /// allocates, locks or does I/O.
    pub fn edited_within(&self, event_time: u32, delta: u32) -> bool {
        let opened = event_time.wrapping_sub(delta);

        if self
            .last_edit
            .is_some_and(|edit| !is_newer_than(edit, event_time) && is_newer_than(edit, opened))
        {
            return true;
        }

        self.ring.any_within(event_time, delta)
    }

    /// Whether the buffer stands in «слово + хвост» — the soft boundary of FR-10, task Т-24-2.
    ///
    /// The whole of that state, and it is a question asked of the ring rather than a field kept
    /// beside it: the tail is open exactly while the newest stroke is a space. See the
    /// soft-boundary block of [`Recorder::record`] for why it is read and not remembered, and
    /// for why a space in the ring can only ever be one this boundary put there.
    ///
    /// One comparison and one array read, on the path of every key that is not a space — the
    /// same budget the rest of the function keeps (NFR-01, NFR-02).
    fn tail_is_open(&self) -> bool {
        let len = self.ring.len();

        len != 0
            && self
                .ring
                .get(len - 1)
                .is_some_and(|stroke| stroke.vk() == VK_SPACE.0)
    }

    /// Flushes the buffer and overwrites it with zeroes — FR-10, SEC-02.
    ///
    /// **This is the entry point the other flush sources of the FR-10 table attach to** and the
    /// reason it is public. Three of them attach here, and they are named as they are actually
    /// wired rather than as the table lists them:
    ///
    /// 1. the full-clearance arm of [`Recorder::reset_up_to`], which is where the three
    ///    asynchronous sources — the mouse click of FR-13 and the two `WinEvent` subscriptions —
    ///    end up whenever FR-12 says the event is newer than everything in the buffer;
    /// 2. rows 8 and 9 — «Блокировка сессии, смена пользователя» and «Приостановка программы
    ///    пользователем», both of which flush unconditionally and both of which are observed on
    ///    the **UI** thread. They reach this function through `watchdog::WM_APP_WIPE`, posted by
    ///    `watchdog::request_wipe` and answered by the input thread. Row 8 fires for the three
    ///    subtypes `watchdog::WIPING_SESSION_EVENTS` names and not for every
    ///    `WM_WTSSESSION_CHANGE`; row 9 fires on the "off" edge of `tray::Tray::toggle_state`
    ///    and not on the resumption;
    /// 3. `app::park_buffer`, the wipe FR-70 asks for when the focus enters a password field.
    ///
    /// ⚠ **Point 2 was a claim before task Т-13-7 and is a fact after it.** This comment used to
    /// state the two rows as wired, and the audit of 2026-08-24 (direction *tests*) found that
    /// nothing in the product called this function for either of them — a grep for
    /// `WTS_SESSION_LOCK` over the whole repository returned nothing at all. The wire, not the
    /// wording, was what had to change; the wording is corrected here to say which subtypes and
    /// which edge, because "every session change" and "the tray" were both wider than the truth.
    ///
    /// Having one flush for all of them to call, which zeroes the memory exactly once and in one
    /// place, is what keeps SEC-02 a property of the module rather than of each caller.
    ///
    /// The conversion session ends with it, and so does the position counter of FR-32 — which is
    /// FR-34, and which is why the flush is one function and not one per caller.
    pub fn reset(&mut self) {
        self.clear_ring();
        self.converted = false;
    }

    /// Empties the buffer if nothing has happened to it for `idle_timeout_s` — **FR-15**, task
    /// T-52-4. Answers whether it did.
    ///
    /// # What the requirement is for
    ///
    /// A typed word lives in this process until the next boundary of the FR-10 table, and there
    /// may be no boundary for hours: a user types half a line, is called away, and the strokes
    /// stand in memory for the rest of the day. **SEC-01** is the first reason that is wrong.
    /// The second is arithmetical: the comment at [`is_newer_than`] rests on «a stroke in the
    /// buffer is never minutes old», and until this rule existed nothing made that true.
    ///
    /// # The clock, and why it is the one of FR-12
    ///
    /// `now_ms` and [`Recorder::last_edit`] are both `GetTickCount` — milliseconds since the
    /// machine started, in 32 bits, wrapping to zero every 49 days and 17 hours. A plain
    /// `now >= last_edit + timeout` would answer backwards across that wrap and would keep a
    /// word typed just before it until something else emptied the ring. [`is_newer_than`] is
    /// the comparison FR-12 already wrote for exactly this, and it is asked the question in the
    /// form it answers: **the deadline is not newer than now.**
    ///
    /// # The three ways this does nothing
    ///
    /// * **The rule is off** — `idle_timeout_s = 0`, which section 7 documents as a request and
    ///   not a mistake;
    /// * **the ring is empty** — there is nothing to forget, and answering `true` there would
    ///   make the counter this drives report a flush every thirty seconds for the rest of the
    ///   session;
    /// * **the deadline has not come.** Not before `idle_timeout_s`; and because the only clock
    ///   that calls this is the thirty-second liveness tick of FR-80, not later than
    ///   `idle_timeout_s + 30 s`. A machine that slept through the deadline flushes on the
    ///   first tick after it wakes, which is the wanted behaviour and not an oversight.
    ///
    /// # What the flush is
    ///
    /// [`Recorder::reset`] — the same «полный сброс» every asynchronous row of the FR-10 table
    /// performs, and therefore the same guarantees: the ring's memory is overwritten (SEC-02),
    /// the position counter of FR-32 goes to zero (FR-34), the moment of the last edit is
    /// forgotten (FR-14) and the conversion session is closed. ⚠ **After this the rollback of
    /// FR-33 is no longer possible**, which is accepted: five minutes after a conversion nobody
    /// is still deciding whether to undo it.
    ///
    /// **Silent.** No sound of FR-100: the user is not at the keyboard, and a program that made
    /// a noise at somebody's empty chair would be reporting its own housekeeping.
    ///
    /// # Where it is called from
    ///
    /// The `WM_TIMER` of `watchdog::LIVENESS_TIMER_ID`, on the **input** thread's message loop
    /// — the thread that owns the buffer. Not from the hook callback, and no timer of its own:
    /// `SetTimer` on the callback path is forbidden outright, and a second timer for a rule
    /// that already has a thirty-second heartbeat to ride on would be a second thing to keep
    /// alive.
    pub fn flush_if_idle(&mut self, now_ms: u32) -> bool {
        if self.idle_timeout_ms == 0 || self.ring.len() == 0 {
            return false;
        }

        // A non-empty ring always has an edit behind it — every push and every pop writes the
        // moment, and `clear_ring` empties the two together — so this is a state that cannot
        // arise rather than a case with a meaning. Answering "not idle" is the conservative
        // reading of it: a buffer whose age is unknown is not thrown away on a guess.
        let Some(last_edit) = self.last_edit else {
            return false;
        };

        let deadline = last_edit.wrapping_add(self.idle_timeout_ms);

        if is_newer_than(deadline, now_ms) {
            return false;
        }

        self.reset();

        true
    }

    /// Empties the ring and zeroes the position counter — **FR-10, FR-34, SEC-02.**
    ///
    /// The one place the ring is emptied, and therefore the one place FR-34 has to be obeyed.
    /// Every rule of the FR-10 table that says "полный сброс" arrives here: the boundary keys
    /// and the editing keys of [`Recorder::record`], the command combinations of the same
    /// function, the last row — any key after a conversion — and, through [`Recorder::reset`],
    /// the mouse click of FR-13, the two `WinEvent` subscriptions, and — since task **Т-13-7**,
    /// which built the wire this sentence used to describe before it existed — the session lock
    /// and the user's own pause from the tray, both of them over `watchdog::WM_APP_WIPE`. The
    /// exact subtypes of the session message and the exact edge of the pause are named at
    /// [`Recorder::reset`], because "every session change" and "the tray" are both wider than
    /// what is actually wired.
    ///
    /// It is one function precisely because FR-34 says "**вместе с** буфером": a counter zeroed
    /// at each call site would be a rule that holds until somebody adds the eleventh call site
    /// and forgets. Here there is nothing to forget — the ring cannot be emptied without the
    /// counter going with it.
    ///
    /// SEC-02 is [`Ring::clear`]'s, unchanged: the whole backing array is overwritten with
    /// zeroes, live window and free slots alike. The counter is a `usize` and zeroing it *is*
    /// its overwrite.
    ///
    /// # The mirror of SEC-04a — task T-05-2a
    ///
    /// Under the `testing` feature, and only under it, the zero goes to [`crate::control`] as
    /// well — through [`Recorder::set_cycle`]. This is the publication that keeps the mirror from
    /// lagging **after a flush**, which is the case that matters: an absent number is noticed, a
    /// stale one is believed, and a channel still reporting position 2 over a buffer FR-34 has
    /// just emptied would tell the acceptance bench of section 11.5 the opposite of the truth.
    ///
    /// # One writer — task T-69-3
    ///
    /// [`Ring::set_len`] has always had the stronger construction — `len` is private to a single
    /// setter, so the length cannot be changed without publishing — and until task T-69-3 the
    /// counter did not. Task T-05-2a was permitted to add the publication at `advance_cycle` and
    /// here, and **not** to change how the counter is written; tasks T-04-3-3, T-39-1 and T-39-2
    /// moved the other writes around, and each write kept its own copy of the publication. Task
    /// T-69-3 was the one allowed to change the shape of this type, and it built the setter: all
    /// three writes — `advance_cycle`, this function and the partial arm of
    /// [`Recorder::reset_up_to`] — now go through [`Recorder::set_cycle`], and the publication is
    /// written once, there.
    ///
    /// NFR-01 to NFR-05: this function is reached from inside the hook callback, and what the
    /// feature adds to that path is one relaxed atomic store. In a build without it — every
    /// Release build, by SEC-04a condition 1 — the call is not compiled at all.
    fn clear_ring(&mut self) {
        self.ring.clear();
        self.set_cycle(0);

        // **FR-14** — task Т-48-2, and it is here for the reason the counter of FR-34 is: every
        // rule of the FR-10 table that empties the ring arrives at this one function, so there
        // is no call site that can forget. A buffer with nothing in it has no edit behind it,
        // and an exemption resting on a memory the flush left standing would be an exemption
        // for typing the user has already had thrown away.
        self.last_edit = None;
    }

    /// Flushes the buffer of everything typed at or before `event_time` — **FR-12**.
    ///
    /// This is the flush the *asynchronous* sources of the FR-10 table use, and the difference
    /// from [`Recorder::reset`] is the whole of FR-12. A mouse click over Raw Input and a
    /// `WinEvent` are delivered to a message queue and are processed whenever that queue is
    /// pumped; the hook callback, by contrast, runs synchronously, inside the system's own
    /// keystroke delivery. So the order in which the two reach this module is **not** the order
    /// in which they happened: a click can arrive here after a keystroke the user made after
    /// the click, and an unconditional flush would then erase text the user typed *later* than
    /// the event that is supposedly flushing it.
    ///
    /// The requirement resolves it with the timestamps this module has been recording since
    /// task T-03-2: every stroke with `time <= event_time` is removed, and the whole buffer is
    /// cleared **only** if `event_time` is newer than every stroke in it. Strokes made after the
    /// event survive, which is the point.
    ///
    /// SEC-02 is unaffected by any of this: every slot the removal frees is overwritten with
    /// zeroes, and a full clearance goes through [`Recorder::reset`], which zeroes the whole
    /// backing array including the slots outside the live window.
    ///
    /// The conversion session of the last row of FR-10 ends with a **full** clearance and
    /// survives a partial one. A session is about the strokes that were converted: while some
    /// of them are still in the buffer, the next key still ends a session that is still open,
    /// and when none of them are left there is no session to end.
    ///
    /// Timestamps come from three different Win32 sources — `KBDLLHOOKSTRUCT.time` for the
    /// stroke, `GetMessageTime` for the `WM_INPUT` of a click and `dwmsEventTime` for a
    /// `WinEvent`. All three are the same 32-bit millisecond tick counter; see
    /// [`is_newer_than`] for what makes them comparable across its wrap.
    pub fn reset_up_to(&mut self, event_time: u32) -> ResetOutcome {
        let live = self.ring.len();
        let newer = self.ring.newer_than(event_time);

        // **FR-14 under FR-12** — task Т-48-2. The memory of the edit is treated exactly as a
        // stroke of the same age would be: an event at or after it removes it, an event older
        // than it leaves it. Written before the branches below so that the full-clearance arm,
        // which reaches [`Recorder::clear_ring`] and would clear the field anyway, and the two
        // arms that do not, all end with the same rule applied.
        if self
            .last_edit
            .is_some_and(|edit| !is_newer_than(edit, event_time))
        {
            self.last_edit = None;
        }

        if newer == 0 {
            // The event is newer than every stroke — including the case of no strokes at all.
            // FR-12: "Полная очистка выполняется только если метка события новее всех нажатий".
            self.reset();
            return ResetOutcome::Cleared { removed: live };
        }

        if newer == live {
            // Everything in the buffer was typed after the event. Nothing to remove, and this
            // is exactly the situation the requirement was written for.
            return ResetOutcome::Kept { kept: live };
        }

        let kept = self.ring.retain_after(event_time);

        // **Finding Н6 — task T-39-1.** The strokes that went took with them the text the
        // position counter of FR-32 was a position over, so the counter goes too: FR-34 for the
        // part of the ring this clearance removes. `clear_ring` is not called — FR-12 keeps the
        // newer strokes and the conversion session they belong to, which is the canon of
        // `the_conversion_session_ends_with_a_full_clearance_and_survives_a_partial_one`.
        //
        // Here and not before the branches, where the rule of FR-14 stands: the `Kept` arm
        // removes nothing and must leave the position where it is, and the full-clearance arm
        // reaches `clear_ring`, which zeroes the counter already.
        //
        // Through the one writer, which publishes the mirror of SEC-04a (task T-69-3).
        self.set_cycle(0);

        ResetOutcome::Partial {
            removed: live - kept,
            kept,
        }
    }

    /// Copies the live strokes into `out`, oldest first, and returns how many.
    ///
    /// No allocation: the caller owns the memory, exactly as
    /// [`crate::convert::convert_strokes`] arranges it. `out` shorter than the buffer is
    /// [`ReadError::OutputTooSmall`] and nothing is written.
    pub fn strokes(&self, out: &mut [Stroke]) -> Result<usize, ReadError> {
        let live = self.ring.len();

        if out.len() < live {
            return Err(ReadError::OutputTooSmall { needed: live });
        }

        for (slot, index) in out.iter_mut().zip(0..live) {
            if let Some(stroke) = self.ring.get(index) {
                *slot = stroke;
            }
        }

        Ok(live)
    }

    /// Renders the live strokes as the conversion form of decision Р-22, oldest first.
    ///
    /// The buffer *produces* [`Keystroke`] values; it does not hold them, and module `convert`
    /// does not learn about this one. Allocation-free for the same reason
    /// [`Recorder::strokes`] is: `out` belongs to the caller, and
    /// [`crate::convert::max_units`] sizes what comes after it.
    pub fn keystrokes(&self, out: &mut [Keystroke]) -> Result<usize, ReadError> {
        let live = self.ring.len();

        if out.len() < live {
            return Err(ReadError::OutputTooSmall { needed: live });
        }

        for (slot, index) in out.iter_mut().zip(0..live) {
            if let Some(stroke) = self.ring.get(index) {
                *slot = stroke.keystroke();
            }
        }

        Ok(live)
    }

    /// The `index`-th live stroke, oldest first.
    pub fn stroke(&self, index: usize) -> Option<Stroke> {
        self.ring.get(index)
    }

    /// The whole backing array, live strokes and free slots alike — the audit view of SEC-02.
    ///
    /// It exists so that "the memory is overwritten with zeroes" can be *checked* rather than
    /// asserted: a test reads this after a flush and every slot has to be [`Stroke::ZEROED`],
    /// including the slots outside the live window, where a ring that merely moved its indices
    /// would have left the previous strokes. Reading it never leaves the process — [`Stroke`]
    /// has no `Debug` and no `Display`, so there is nothing this view can be turned into
    /// except more strokes (SEC-01, SEC-07).
    pub fn slots(&self) -> &[Stroke] {
        &self.ring.slots
    }

    /// The mask of FR-04 as of this instant, with the per-key flags of `flags` folded in.
    fn mods_now(&self, flags: u32) -> StrokeMods {
        let mut mods = StrokeMods::NONE;

        if self.held.shift() {
            mods = mods.with(StrokeMods::SHIFT);
        }
        if self.held.ctrl() {
            mods = mods.with(StrokeMods::CTRL);
        }
        // `LLKHF_ALTDOWN` is the system's own answer for this very event, so it is believed
        // alongside the tracker: an `Alt` that went down before the hook was installed is
        // invisible to the tracker and still sets the flag here.
        if self.held.alt() || flags & LLKHF_ALTDOWN.0 != 0 {
            mods = mods.with(StrokeMods::ALT);
        }
        if self.held.altgr() {
            mods = mods.with(StrokeMods::ALTGR);
        }
        if self.held.caps {
            mods = mods.with(StrokeMods::CAPS);
        }
        // FR-05. The flag is the ninth bit of the physical key and the only way the keypad `/`
        // is told from the `/?` key of the main block; task T-04-1 replays it on injection.
        if flags & LLKHF_EXTENDED.0 != 0 {
            mods = mods.with(StrokeMods::EXTENDED);
        }

        mods
    }

    /// What the cache says this key produces — FR-06, one array read.
    fn lookup(&self, scan: u16, mods: StrokeMods) -> KeyMapping {
        match self.active_map() {
            Some(map) => map.lookup(scan, mods.extended(), mods.to_layout_mods()),
            // No cache yet, or the active layout is not one of the participating layouts of
            // FR-35. The stroke is kept with an empty `chars` and `len` zero: what was typed is
            // still known by its **scan code**, which is what FR-23 converts from — and which is
            // why an empty `chars` costs nothing here. Task Т-22-10: what comes out of the
            // conversion is the target layout's own character whenever that layout has one on the
            // key, and the empty stroke unchanged only when it has not; see the module
            // documentation.
            None => KeyMapping::EMPTY,
        }
    }

    /// The map of the active layout, if there is a cache and the layout is in it.
    fn active_map(&self) -> Option<&crate::layouts::LayoutMap> {
        self.cache.as_ref()?.map_at(self.active_index?)
    }

    /// Whether this stroke wrote **nothing, in any layout the session has** — task T-52-1.
    ///
    /// The criterion of the new boundary row of FR-10. `produced` is what
    /// [`Recorder::lookup`] has just answered for the active layout, handed in rather than
    /// looked up again, so the row costs no second lookup.
    ///
    /// # The three answers, in the order they are cheapest
    ///
    /// * **The key wrote here.** `produced` carries characters — a letter, a digit, a space, or
    ///   the dead character of a dead key, which `ToUnicodeEx` reports with a negative return
    ///   and which [`KeyMapping::is_empty`] therefore already calls non-empty (FR-24). One
    ///   comparison, and it is the answer every keystroke of an ordinary word gets.
    /// * **⚠ There is no cache.** Strokes made before FR-20 has swept the layouts. The
    ///   criterion is not computable — there is no layout to ask — and "I do not know" is not
    ///   "nothing". The rule stands aside and the stroke is recorded exactly as this module
    ///   recorded it before this task. A boundary invented out of an unbuilt cache would empty
    ///   the buffer of every key typed in the first moments of a session.
    /// * **The other layouts are asked.** [`crate::layouts::LayoutCache::produces_in_any_map`],
    ///   one array read per layout, stopping at the first that answers yes. A key blank in the
    ///   layout the user is typing in and significant in another one **is text**: that is
    ///   precisely the stroke FR-22 and FR-23 exist to carry over, and flushing it would
    ///   contradict FR-23 from the other side.
    ///
    /// ⚠ **The active layout being absent from the cache is not the same as there being no
    /// cache.** `lookup` answers empty for every key then, letters included — and the scan of
    /// the cache is exactly what saves them: the letter writes in some layout the cache holds,
    /// so it is recorded, while `F5` writes in none and ends the word. The row behaves
    /// correctly in a state [`Recorder::restamp`] deliberately leaves alone.
    ///
    /// NFR-01 to NFR-05: see the comment at the call site in [`Recorder::record`].
    fn stroke_writes_nowhere(&self, produced: KeyMapping, scan: u16, mods: StrokeMods) -> bool {
        if !produced.is_empty() {
            return false;
        }

        let Some(cache) = self.cache.as_ref() else {
            return false;
        };

        !cache.produces_in_any_map(scan, mods.extended(), mods.to_layout_mods())
    }

    /// Reads the layout the user is really typing in and adopts it — **the repair of defect E**,
    /// task T-10-14.
    ///
    /// Called from [`Recorder::record`] on the first stroke of a new word and from nowhere else.
    /// See the comment at the call site for the measurement.
    ///
    /// # The three cases it deliberately does nothing in
    ///
    /// * **No probe.** A recorder built by a test that did not state what the keyboard is doing
    ///   behaves exactly as this module behaved before this task. That is what keeps every
    ///   existing test of the FR-10 table a statement about FR-10 and not about the layout of
    ///   the machine running it.
    /// * **A zero answer.** No foreground window at all — the desktop is switching, or this is
    ///   the secure desktop. Not a change and not treated as one, the same rule
    ///   `app::layout_refresh_needed` already applies: publishing it would tell the buffer to
    ///   record under a layout no cache contains.
    /// * **⚠ A layout the cache of FR-20 does not have.** The stamp is left alone on purpose.
    ///   The honest repair would be to rebuild the cache, and FR-20 is a sweep of every virtual
    ///   key against eight modifier combinations for every layout in the session — five
    ///   milliseconds, measured, which is fifty times the whole budget NFR-01 gives this
    ///   callback. So this narrows itself to what it can do without lying: adopting a layout the
    ///   cache already holds can only ever make [`Recorder::lookup`] more correct, and never
    ///   turns a working lookup into an empty one. A layout genuinely new to the session is the
    ///   business of FR-21's rebuild on the message path, which is unchanged and still runs.
    ///   The case is narrow — `GetKeyboardLayoutList` gives the cache every layout of the
    ///   session, so a switch the user can make is in it unless the list itself has just grown.
    ///
    /// # NFR-01 to NFR-05
    ///
    /// Three leaf Win32 reads ([`crate::switch::current`]): `GetForegroundWindow` reads a value
    /// the window manager keeps for the desktop, `GetWindowThreadProcessId` the window's own
    /// record and `GetKeyboardLayout` the thread's. None allocates (NFR-03), none can block
    /// (NFR-04), none journals anything (NFR-05) and none is re-entrant into this callback. Then
    /// one comparison against the layouts the cache holds — a scan of the session's layout list,
    /// two entries on this machine — and two stores.
    ///
    /// **Measured, not argued** (report of T-10-14, `--experiment-latency --words`, a volley of
    /// 10 290 presses in which every seventh key starts a word, so one callback sample in
    /// fourteen takes this branch): `callback_p50_ns` 2000 before and 2000 after,
    /// `callback_p99_ns` 2900–3000 before and 2900–3000 after. The branch does not show at the
    /// percentiles NFR-01 is written in.
    fn restamp(&mut self) {
        let Some(probe) = self.stamp else {
            // ⭐ **Outcome 2 of task T-10-15.** See the note at the call site in
            // [`Recorder::record`] for why every exit of this function counts itself.
            #[cfg(feature = "testing")]
            crate::control::note_restamp(crate::control::Restamp::NoProbe);
            return;
        };

        let observed = probe();

        if observed == LayoutId::default() || observed == self.active {
            // ⭐ **Outcome 3 of task T-10-15.** This is what a *healthy* first stroke looks like:
            // the stamp already names the layout the probe reads, so there is nothing to correct.
            #[cfg(feature = "testing")]
            crate::control::note_restamp(crate::control::Restamp::Unchanged);
            return;
        }

        if !self
            .cache
            .as_ref()
            .is_none_or(|cache| cache.contains(observed))
        {
            // ⭐ **Outcome 4 of task T-10-15 — the silent refusal, and the one that task was
            // written around.** The probe read a real layout, different from the one stamped, and
            // the cache of FR-20 has no map for it. Accepting would be worse than the defect: the
            // stamp would name a layout `lookup` cannot decode through and `resolve_active` would
            // leave `active_index` at `None`. A callback cannot rebuild the cache, so refusing is
            // the only correct thing left — and *saying so* is what was missing. A stale cache
            // makes the repair of defect E do nothing at all, and before this counter the channel
            // showed that state and a probe that was never consulted as the same reading.
            #[cfg(feature = "testing")]
            crate::control::note_restamp(crate::control::Restamp::Uncached);
            return;
        }

        self.active = observed;
        self.resolve_active();

        // ⚠ **The mirror of SEC-04a has to see this.** Until this task
        // `app::publish_active_layout` was the one place in the program that wrote
        // `Recorder::active`, and therefore the one place that could say what the stamp holds.
        // It is now the second; a stamp corrected here and not mirrored would make
        // `active_layout` on the channel a value the program has already stopped using, and the
        // detector of this very defect reads that key. One relaxed atomic store — the same cost
        // `advance_cycle` pays a few lines up, and compiled out of every build that is not a
        // `testing` one.
        #[cfg(feature = "testing")]
        crate::control::note_active_layout(observed.raw());

        // ⭐ **Outcome 5 of task T-10-15** — the repair of defect E doing its work. It is counted
        // beside the mirror above rather than instead of it: `active_layout` says what the stamp
        // *holds*, and it holds the same value whether it was corrected here once or a hundred
        // times. The count is what turns «штамп совпал» into «штамп исправили».
        #[cfg(feature = "testing")]
        crate::control::note_restamp(crate::control::Restamp::Accepted);
    }

    /// Re-resolves the position of the active layout in the cache.
    ///
    /// Done when either the cache or the active layout is published, and never in the
    /// callback, which is the arrangement module `layouts` asks for: "The hook path resolves a
    /// layout to its position once and then keeps the position."
    fn resolve_active(&mut self) {
        self.active_index = self
            .cache
            .as_ref()
            .and_then(|cache| cache.index_of(self.active));
    }
}

// ---------------------------------------------------------------------------------------
// The thread-local singleton — section 6.3
// ---------------------------------------------------------------------------------------

thread_local! {
    /// The buffer of the calling thread. In the product there is exactly one and it belongs to
    /// the input thread; section 6.3 gives it no other owner.
    ///
    /// `const`-initialised, so reading it is a plain thread-local access with no
    /// lazy-initialisation branch and no allocation — the property the hot path needs (NFR-01,
    /// NFR-03), and the same construction `hook::HOTKEY_STATE` is built on.
    ///
    /// `RefCell` rather than a lock: it cannot block, and it cannot even fail here, because a
    /// low-level keyboard callback is not re-entered by the thread that is running it.
    static RECORDER: RefCell<Option<Recorder>> = const { RefCell::new(None) };
}

/// Installs a buffer on the calling thread, sized by the configuration — FR-07.
///
/// Called by the input thread before the hook goes up. Any buffer already installed is
/// dropped, which zeroes it (SEC-02).
///
/// **This is where the repair of defect D is armed** (task T-10-12): the recorder the product
/// runs on is the one that can check its belief about the modifiers against the system. It is
/// the single path by which the input thread ever gets a buffer, and `app::restore_buffer`
/// re-installs the very value `app::park_buffer` took away, so the probe survives the FR-70
/// gate without anybody having to remember it.
/// **The repair of defect E is armed on the same line** (task T-10-14): the recorder the product
/// runs on is the one that can read the layout the user is really typing in, at the moment it
/// records a stroke. Both probes travel with the recorder across the FR-70 gate for the same
/// reason and by the same route.
///
/// ⭐ The layout probe is [`crate::switch::current`] — FR-52 itself, already public, already the
/// reading the selection path of FR-60/FR-61 makes at the moment of the press. No new reader was
/// written: see [`LayoutProbe`].
pub fn install(config: &settings::Buffer) {
    let mut recorder = Recorder::from_config(config);
    recorder.verify_held_with(crate::hook::physical_modifiers);
    recorder.stamp_layout_with(crate::switch::current);
    install_recorder(recorder);
}

/// Installs `recorder` on the calling thread, replacing and zeroing whatever was there.
pub fn install_recorder(recorder: Recorder) {
    RECORDER.with(|cell| {
        cell.replace(Some(recorder));
    });
}

/// Removes the buffer of the calling thread, zeroing it, and reports whether there was one.
///
/// The zeroing is [`Ring::drop`]; the block goes back to the allocator holding nothing.
pub fn uninstall() -> bool {
    RECORDER.with(|cell| cell.replace(None).is_some())
}

/// Whether this thread has a buffer.
pub fn is_installed() -> bool {
    RECORDER.with(|cell| cell.borrow().is_some())
}

/// Runs `action` against the buffer of the calling thread, if there is one.
///
/// The general accessor: publishing the cache (FR-20, FR-21), publishing the layout (FR-04),
/// reading the strokes out for conversion (tasks T-04-1 and T-05-2) and the length the debug
/// channel of SEC-04a is allowed to report all go through it.
pub fn with<R>(action: impl FnOnce(&mut Recorder) -> R) -> Option<R> {
    RECORDER.with(|cell| cell.borrow_mut().as_mut().map(action))
}

/// Records one keystroke — called by [`crate::hook::classify`] at the point FR-03, FR-90 and
/// FR-99 have already been decided.
///
/// A thread with no buffer — every thread of the program except the input one, and every test
/// of `tests\hook.rs` — takes one thread-local read and returns [`Recorded::Ignored`].
pub fn record(key: KeyEvent) -> Recorded {
    with(|recorder| recorder.record(key)).unwrap_or(Recorded::Ignored)
}

/// Seeds the `CapsLock` of the buffer of the calling thread from `probe` — **task T-13-4**.
///
/// Answers whether this thread had a buffer to seed.
///
/// Named after the method it drives, the way [`record`], [`reset`] and [`note_conversion`] are
/// named after theirs: this is [`Recorder::set_caps_lock`] as the rest of the program is allowed
/// to reach it. The argument differs because the value cannot be — see below.
///
/// # The five points
///
/// The tracker of [`Held`] flips on the `CapsLock` presses that reach [`Recorder::record`], and
/// there are three ways for a press not to reach it: the program is suspended (FR-90, where
/// `hook::classify` answers `PASS` before `record`), the buffer is parked (FR-70 for a password
/// field, FR-84 for an excluded process), and the hook is not up yet. So the machine's own
/// toggle is read again at every point past which a press could have been missed:
///
/// 1. **the buffer is installed at start-up** — `app::install_buffer`;
/// 2. **the hook is installed, first time and every reinstallation** — [`crate::hook::install`],
///    the single door `watchdog::reinstall_hook` and start-up both go through;
/// 3. **FR-90 is resumed** — [`crate::hook::set_active`] on the `false → true` edge, which
///    happens on the UI thread and therefore travels as [`crate::hook::WM_APP_SEED_CAPS`];
/// 4. **the buffer comes back from the FR-70 gate** — `app::restore_buffer`;
/// 5. **the session comes back, or the machine wakes** — the receiving side of
///    `watchdog::WM_APP_LAYOUT` in `app::window_proc`, where the watchdog already asks the input
///    thread for a fresh layout stamp for exactly the same reason.
///
/// # Why the probe is called in here and not by the caller
///
/// `GetKeyState` answers from the **calling thread's** input state, so it is only worth asking on
/// the thread that owns the queue the hook feeds — section 6.3's input thread. The probe is
/// therefore invoked *inside* [`with`], which answers `None` on every thread that owns no buffer:
/// the UI and watcher threads cannot reach the Win32 call at all, however a point is wired, and
/// point 3 above is free to publish from the UI thread without the reading following it there.
///
/// # NFR-01 to NFR-05
///
/// **The boundary of this repair runs at the hook callback and none of this crosses it.** Every
/// one of the five points is on the message loop of the input thread or in `install`; the
/// callback gained no call, no branch and no thread-local of its own. `Recorder::record` is
/// untouched by task T-13-4.
pub fn set_caps_lock(probe: CapsProbe) -> bool {
    with(|recorder| recorder.set_caps_lock(probe())).is_some()
}

/// Flushes and zeroes the buffer of the calling thread — FR-10, SEC-02.
///
/// The public flush the other sources of the FR-10 table attach to; see [`Recorder::reset`].
pub fn reset() -> bool {
    with(Recorder::reset).is_some()
}

/// Empties the buffer of the calling thread if it has stood untouched for `idle_timeout_s` —
/// **FR-15**, SEC-01, SEC-02, task T-52-4.
///
/// [`Recorder::flush_if_idle`] as the rest of the program reaches it. The one caller is the
/// thirty-second liveness tick of FR-80 on the input thread (`watchdog::flush_buffer_if_idle`),
/// which is the thread that owns the buffer — a thread with no buffer answers `false`, which is
/// the honest answer for a thread that has recorded nothing.
pub fn flush_if_idle(now_ms: u32) -> bool {
    with(|recorder| recorder.flush_if_idle(now_ms)).unwrap_or(false)
}

/// Flushes the buffer of the calling thread of everything typed at or before `event_time`, and
/// answers `None` on a thread that has no buffer — **FR-12**, SEC-02.
///
/// The entry point of the asynchronous flush sources of the FR-10 table: the mouse click of
/// FR-13 over Raw Input and the `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS`
/// subscriptions, all three of which module `watchdog` owns. See [`Recorder::reset_up_to`].
pub fn reset_up_to(event_time: u32) -> Option<ResetOutcome> {
    with(|recorder| recorder.reset_up_to(event_time))
}

/// **FR-14** — whether the buffer of the calling thread was edited inside `(event_time − delta,
/// event_time]`, task Т-48-2.
///
/// [`Recorder::edited_within`] as the rest of the program reaches it, named after the method it
/// drives exactly as [`record`] and [`reset_up_to`] are. A thread with no buffer — every thread
/// but the input one — answers `false`, which is the honest answer: a thread that records
/// nothing has seen nobody type.
pub fn edited_within(event_time: u32, delta: u32) -> bool {
    with(|recorder| recorder.edited_within(event_time, delta)).unwrap_or(false)
}

/// Records that a conversion has run — the last row of FR-10.
pub fn note_conversion() -> bool {
    with(Recorder::note_conversion).is_some()
}

/// How many strokes are live. Zero when there is no buffer.
pub fn len() -> usize {
    with(|recorder| recorder.len()).unwrap_or(0)
}

/// Whether the buffer is empty, or absent.
pub fn is_empty() -> bool {
    len() == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The system answering "nothing is held" — the premise of the unit tests of finding С8.
    fn nothing_physically_held() -> Physical {
        Physical {
            ctrl: false,
            alt_left: false,
            alt_right: false,
            win: false,
            shift_left: false,
            shift_right: false,
        }
    }

    /// **Finding С8 — task T-39-3.** A `Shift` the stream raised and the system says is not held
    /// comes down on reconciliation, as every other modifier the check covers does. Until this task
    /// reconciliation masked six bits and `Shift` was not among them, so a lost release of it had
    /// no way back at all.
    #[test]
    fn reconciliation_lowers_a_shift_the_system_says_is_not_held() {
        let mut held = Held::default();
        held.apply(Role::ShiftLeft, Edge::Down);
        held.apply(Role::ShiftRight, Edge::Down);
        assert!(
            held.shift(),
            "the premise: the stream raised both Shift keys"
        );

        let changed = held.reconcile(nothing_physically_held());
        let still_held = held.shift();

        assert!(
            changed && !still_held,
            "С8: reconciliation with nothing held — the belief changed: {changed}; Shift is still \
             believed held: {still_held}"
        );
    }

    /// **Task T-39-3: masking only lowers.** A `Shift` the system says is held stays up — the left
    /// one here — while the right one, whose release was lost, comes down.
    #[test]
    fn reconciliation_keeps_a_shift_the_system_says_is_held() {
        let mut held = Held::default();
        held.apply(Role::ShiftLeft, Edge::Down);
        held.apply(Role::ShiftRight, Edge::Down);

        let physical = Physical {
            shift_left: true,
            ..nothing_physically_held()
        };

        assert!(held.reconcile_shift(physical), "the right Shift came down");
        assert!(
            held.shift_left,
            "the left Shift is really held and stays up"
        );
        assert!(
            !held.shift_right,
            "the right Shift's release was lost and it comes down"
        );
    }

    /// **Task T-39-3: the once-per-word check is `Shift` and nothing else.** A stuck `Ctrl` and a
    /// stuck right `Alt` stay up for the command row to put right, because lowering them here would
    /// change the `AltGr` of the mask and with it the character `lookup` returns.
    #[test]
    fn the_shift_check_leaves_every_other_modifier_alone() {
        let mut held = Held::default();
        held.apply(Role::ShiftLeft, Edge::Down);
        held.apply(Role::CtrlLeft, Edge::Down);
        held.apply(Role::AltRight, Edge::Down);

        assert!(held.reconcile_shift(nothing_physically_held()));
        assert!(!held.shift());
        assert!(
            held.ctrl() && held.alt(),
            "Ctrl and Alt are the command row's to lower"
        );
    }

    /// FR-07 and section 7 agree on the default, and the two are wired to each other rather
    /// than written down twice.
    #[test]
    fn the_default_capacity_is_the_one_section_7_configures() {
        assert_eq!(DEFAULT_CAPACITY, 256);
        assert_eq!(settings::Buffer::default().capacity, DEFAULT_CAPACITY);
    }

    /// The mask of FR-04 narrows to the cache key of FR-20 without losing the three modifiers
    /// that decide which character a key produces.
    #[test]
    fn the_mask_narrows_to_the_three_modifiers_the_cache_is_keyed_by() {
        let mods = StrokeMods::NONE
            .with(StrokeMods::SHIFT)
            .with(StrokeMods::CTRL)
            .with(StrokeMods::CAPS)
            .with(StrokeMods::EXTENDED);

        let narrowed = mods.to_layout_mods();

        assert!(narrowed.shift());
        assert!(narrowed.caps());
        assert!(!narrowed.altgr());
        // `Ctrl` and the extended flag have no place in a cache key: one marks a command, the
        // other names the key rather than modifying it.
        assert_eq!(narrowed, Mods::new(true, true, false));
    }

    /// What the ring of FR-07 costs, in the terms NFR-06 is argued in.
    ///
    /// The number is asserted rather than described so that a field added to [`Stroke`] later
    /// cannot quietly turn the buffer into something worth measuring.
    #[test]
    fn a_stroke_is_a_small_copy_value_and_the_whole_ring_is_kilobytes() {
        assert_eq!(core::mem::size_of::<Stroke>(), 32);
        assert_eq!(core::mem::size_of::<Stroke>() * DEFAULT_CAPACITY, 8_192);
        assert!(core::mem::size_of::<Stroke>() * MAX_CAPACITY <= 128 * 1024);
    }

    /// A capacity of zero in a hand-edited file is a mistake, not a request for a program that
    /// quietly stops working; an absurd one is clamped rather than allocated.
    #[test]
    fn the_capacity_is_clamped_into_a_range_that_can_be_allocated() {
        assert_eq!(Recorder::with_capacity(0).capacity(), DEFAULT_CAPACITY);
        assert_eq!(Recorder::with_capacity(1).capacity(), 1);
        assert_eq!(Recorder::with_capacity(usize::MAX).capacity(), MAX_CAPACITY);
    }

    // -----------------------------------------------------------------------------------
    // `zero_slice` — the SEC-02 write for the buffers outside the ring. Task T-13-15.
    // -----------------------------------------------------------------------------------
    //
    // These live here rather than in `tests\buffer.rs` for the reason the function signature
    // gives: [`zero_slice`] is `pub(crate)`, and an integration test is a separate crate that
    // cannot see it. The tests of the ring itself stay where they are.

    /// The two integer element types the text buffers are made of are left holding zeroes.
    ///
    /// Same shape as the SEC-02 tests of the ring in `tests\buffer.rs`: put something in, call
    /// the zeroing, and read the memory back — the check is on the slot, not on a length.
    #[test]
    fn zero_slice_leaves_the_text_buffers_holding_zeroes() {
        // The code units of a converted word, in the type `convert` and `selection` carry.
        let mut units: Vec<u16> = "привет".encode_utf16().collect();
        assert!(
            units.iter().any(|unit| *unit != 0),
            "there is text to erase"
        );

        zero_slice(&mut units);

        assert!(units.iter().all(|unit| *unit == 0), "SEC-02: every unit");
        assert_eq!(units.len(), 6, "the length is not what was changed");

        // The bytes of a `CF_UNICODETEXT` block, the type the clipboard path carries.
        let mut bytes: Vec<u8> = vec![1, 2, 3, 0xFF];

        zero_slice(&mut bytes);

        assert!(bytes.iter().all(|byte| *byte == 0), "SEC-02: every byte");
    }

    /// The two structure element types are zeroed as whole values, not as bytes.
    ///
    /// `INPUT` and [`Keystroke`] are what the injection path releases, and neither is a number.
    /// The zero written into them is `Default::default()`, which is `mem::zeroed()` for `INPUT`
    /// and the derived all-zero value for `Keystroke`.
    #[test]
    fn zero_slice_leaves_the_injection_buffers_holding_default_values() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, VIRTUAL_KEY,
        };

        let mut events = vec![
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0x41),
                        wScan: 0x1E,
                        dwExtraInfo: 0xABCD,
                        ..Default::default()
                    },
                },
            };
            3
        ];

        assert!(
            events.iter().any(|event| event.r#type.0 != 0),
            "there is a packet to erase"
        );

        zero_slice(&mut events);

        for event in &events {
            assert_eq!(event.r#type.0, 0, "SEC-02: the discriminant is zeroed");

            // SAFETY: `INPUT_0` is a union of plain-data structures of the same C layout, and
            // every one of them is valid for any bit pattern; the keyboard arm is the largest
            // this program ever writes and is read here only to prove that its bytes are zero.
            let keyboard = unsafe { event.Anonymous.ki };

            assert_eq!(keyboard.wVk.0, 0, "SEC-02: the virtual key is zeroed");
            assert_eq!(keyboard.wScan, 0, "SEC-02: the scan code is zeroed");
            assert_eq!(keyboard.dwExtraInfo, 0, "SEC-02: the signature is zeroed");
        }

        // The copy of the strokes `on_hotkey` releases, built the way `Stroke::keystroke`
        // builds one.
        let mut strokes = vec![
            Keystroke::new(
                LayoutId::from_raw(0x0409),
                0x1E,
                false,
                Mods::new(true, false, false),
                KeyMapping::EMPTY,
            );
            4
        ];

        assert!(
            strokes.iter().all(|stroke| *stroke != Keystroke::default()),
            "there are strokes to erase"
        );

        zero_slice(&mut strokes);

        assert!(
            strokes.iter().all(|stroke| *stroke == Keystroke::default()),
            "SEC-02: every stroke of the copy"
        );
    }

    /// An empty slice is an ordinary argument, not a special case.
    ///
    /// It happens for real: a press with nothing to erase gives `replace` a zero-length text,
    /// and a clipboard block of a single terminator decodes to nothing at all.
    #[test]
    fn zero_slice_of_an_empty_slice_writes_nothing_and_returns() {
        let mut nothing: Vec<u16> = Vec::new();

        zero_slice(&mut nothing);

        assert!(nothing.is_empty());
    }
}
