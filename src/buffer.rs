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
//! Task **T-03-4** added the length mirror of SEC-04a: [`Ring::set_len`] is now the one writer
//! of the ring's length, and under the `testing` feature — and only under it — every write
//! publishes the new length into [`crate::control`], which is the only way a thread other than
//! the input one can learn a number that lives in a thread-local (section 6.3). Nothing else
//! about the buffer changed, and in a build without the feature the mirror does not exist.
//! Implemented by backlog tasks: T-03-2 (done), T-03-2a (done), T-03-3 (done), T-03-4 (done).
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
//! stored with no characters and `len` zero, and FR-23 carries it through conversion
//! unchanged.
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
    VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RCONTROL,
    VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
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
    /// Not an error in either case: FR-23 carries such a stroke through conversion unchanged.
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
// Modifier tracking
// ---------------------------------------------------------------------------------------

/// Which modifier a virtual key is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Either `Shift`.
    Shift,
    /// Either `Ctrl`, including the one the keyboard fakes in front of `AltGr`.
    Ctrl,
    /// The left `Alt` — a command modifier and nothing else.
    AltLeft,
    /// The right `Alt` — half of `AltGr` when `Ctrl` is down with it.
    AltRight,
    /// Either `Win`.
    Win,
    /// `CapsLock`, a toggle rather than a held key.
    Caps,
}

/// Which modifier `vk` is, if it is one.
///
/// `WH_KEYBOARD_LL` reports the sided codes — `VK_LSHIFT` rather than `VK_SHIFT` — which is
/// what makes `AltGr` recognisable at all. The neutral codes are accepted as well and the
/// neutral `Alt` is read as the *left* one, which is the conservative reading: a stroke that
/// might be `AltGr` and might be a command is treated as a command, and FR-10 flushes. Losing
/// a buffer is recoverable; converting a menu accelerator into text is not.
const fn modifier_role(vk: u16) -> Option<Role> {
    match vk {
        v if v == VK_SHIFT.0 || v == VK_LSHIFT.0 || v == VK_RSHIFT.0 => Some(Role::Shift),
        v if v == VK_CONTROL.0 || v == VK_LCONTROL.0 || v == VK_RCONTROL.0 => Some(Role::Ctrl),
        v if v == VK_MENU.0 || v == VK_LMENU.0 => Some(Role::AltLeft),
        v if v == VK_RMENU.0 => Some(Role::AltRight),
        v if v == VK_LWIN.0 || v == VK_RWIN.0 => Some(Role::Win),
        v if v == VK_CAPITAL.0 => Some(Role::Caps),
        _ => None,
    }
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
/// through [`Recorder::set_caps_lock`].
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Held {
    shift: bool,
    ctrl: bool,
    alt_left: bool,
    alt_right: bool,
    win: bool,
    caps: bool,
}

impl Held {
    /// Applies one modifier key event.
    fn apply(&mut self, role: Role, edge: Edge) {
        let down = matches!(edge, Edge::Down);

        match role {
            Role::Shift => self.shift = down,
            Role::Ctrl => self.ctrl = down,
            Role::AltLeft => self.alt_left = down,
            Role::AltRight => self.alt_right = down,
            Role::Win => self.win = down,
            // A toggle: it flips on the press and does nothing on the release. Auto-repeat of
            // a held `CapsLock` would flip it once per repeat, which is what the keyboard does
            // to the light as well.
            Role::Caps => {
                if down {
                    self.caps = !self.caps;
                }
            }
        }
    }

    /// Whether either `Alt` is down.
    const fn alt(self) -> bool {
        self.alt_left || self.alt_right
    }

    /// Whether this is `AltGr` — the right `Alt` with `Ctrl`, which is how the keyboard
    /// reports it: the layout driver emits a fake left `Ctrl` in front of the right `Alt`.
    ///
    /// The distinction is the difference between text and a command. `AltGr+E` is `€` on a
    /// German layout and must reach the buffer; `Ctrl+Alt+E` is an accelerator and must flush
    /// it (FR-10).
    const fn altgr(self) -> bool {
        self.ctrl && self.alt_right
    }
}

// ---------------------------------------------------------------------------------------
// The flush rules of FR-10 — the LL-hook rows only
// ---------------------------------------------------------------------------------------

/// Row 1 of the FR-10 table: the keys that end a word or a field.
const BOUNDARY_KEYS: [u16; 4] = [VK_SPACE.0, VK_TAB.0, VK_RETURN.0, VK_ESCAPE.0];

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
        }
    }

    /// A buffer sized by the `[buffer]` section of the configuration — FR-07.
    ///
    /// The capacity is *read*, not hard-wired: the constant of FR-07 lives in section 7 as a
    /// default, and this is the path the product takes.
    pub fn from_config(config: &settings::Buffer) -> Self {
        Self::with_capacity(config.capacity)
    }

    /// How many strokes fit.
    pub fn capacity(&self) -> usize {
        self.ring.capacity()
    }

    /// Resizes the ring, keeping everything else this recorder knows — FR-07.
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
    /// flush of FR-10, it happens once and at start-up, and it is **not** the case FR-11 speaks
    /// about: a layout change flushes nothing and goes through
    /// [`Recorder::set_active_layout`], which touches no slot at all.
    ///
    /// Allocates, exactly once, like [`Recorder::with_capacity`] — so it is called from the
    /// input thread's message loop and never from the callback (NFR-03).
    pub fn set_capacity(&mut self, capacity: usize) {
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

        // FR-04 records presses. A release adds nothing to what was typed, and recording both
        // edges would double every character in the buffer.
        if matches!(key.edge, Edge::Up) {
            return Recorded::Ignored;
        }

        // FR-10, last row: "любая клавиша после конвертации — полный сброс (завершение сессии
        // конвертации)". The flush happens here and the key itself is then treated normally,
        // so the first letter of the next word is the first stroke of the next buffer.
        if self.converted {
            self.converted = false;
            self.ring.clear();
        }

        let mods = self.mods_now(key.flags);

        // FR-10: "Ctrl/Alt/Win + клавиша, **кроме комбинаций смены раскладки, названных
        // FR-11** — полный сброс (команда, а не текст)". `AltGr` is the exception the mask
        // itself carries: it is `Ctrl` plus the right `Alt` by construction and it produces
        // characters, so a layout that puts text on it keeps working.
        if (mods.ctrl() || mods.alt() || self.held.win) && !mods.altgr() {
            // ⚠ **FR-11, the second exception, and the narrower of the two.** FR-11 names two
            // combinations by name — `Alt+Shift` and `Win+Space` — and says a layout switch
            // "**не сбрасывает** буфер: HKL хранится по каждому нажатию отдельно". `Alt+Shift`
            // never arrives here: it is modifiers alone and the first line of this function has
            // already answered `Modifier` for both halves of it, which task T-03-3b measured
            // rather than assumed. `Win+Space` does arrive, because `Space` is not a modifier,
            // and until the user's decision on question 43 (commit e6ba407, which is the
            // wording of the row quoted above) it was flushed here — so a user who noticed the
            // wrong layout mid-word, pressed `Win+Space` to fix it and reached for the hotkey
            // found nothing left to convert. That is the very outcome FR-11 exists to prevent.
            //
            // The exception is kept as narrow as the requirement is: `Space`, with `Win` held,
            // and with neither `Ctrl` nor `Alt` in the combination — `Ctrl+Win+Space` and
            // `Alt+Win+Space` switch no layout and stay commands. Everything else keeps the
            // behaviour it had: `Win+R` is a command and flushes, and a bare `Space` with no
            // modifier at all is the boundary key of the first row of the FR-10 table and
            // flushes as well, three rules further down.
            //
            // Nothing is recorded either. The return is `Ignored` and not a fall-through,
            // because the stroke is a switch the system consumes: it puts no space into the
            // text, and pushing one into the ring would make the conversion of FR-22 produce a
            // character the user never typed.
            if key.vk == VK_SPACE.0 && self.held.win && !mods.ctrl() && !mods.alt() {
                return Recorded::Ignored;
            }

            self.ring.clear();
            return Recorded::Flushed;
        }

        // FR-10: `Backspace` takes one element out and does **not** flush. It is the one key in
        // the table whose effect on the text the buffer can follow exactly.
        if key.vk == VK_BACK.0 {
            return if self.ring.pop() {
                Recorded::Popped
            } else {
                Recorded::Ignored
            };
        }

        if flushes(key.vk) {
            self.ring.clear();
            return Recorded::Flushed;
        }

        // FR-06 through the cache of FR-20, and FR-04: what the key gave, at the moment it was
        // pressed, in the layout that was active then.
        let produced = self.lookup(key.scan, mods);

        let stroke = Stroke::new(key.vk, key.scan, mods, self.active, key.time, produced);

        if self.ring.push(stroke) {
            Recorded::Evicted
        } else {
            Recorded::Stored
        }
    }

    /// Flushes the buffer and overwrites it with zeroes — FR-10, SEC-02.
    ///
    /// **This is the entry point the other flush sources of the FR-10 table attach to** and
    /// the reason it is public: the `WM_WTSSESSION_CHANGE` subscription (task T-06-2) and the
    /// "приостановка пользователем" of the tray, both of which flush unconditionally, and the
    /// full-clearance arm of [`Recorder::reset_up_to`], which is where the three asynchronous
    /// sources of the table — the mouse click of FR-13 and the two `WinEvent` subscriptions —
    /// end up whenever FR-12 says the event is newer than everything in the buffer. Having one
    /// flush for all of them to call, which zeroes the memory exactly once and in one place, is
    /// what keeps SEC-02 a property of the module rather than of each caller.
    ///
    /// The conversion session ends with it. Task T-05-2 owns FR-34 and hangs its cycle counter
    /// on this call, which is why the flush is one function and not one per caller.
    pub fn reset(&mut self) {
        self.ring.clear();
        self.converted = false;
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

        if self.held.shift {
            mods = mods.with(StrokeMods::SHIFT);
        }
        if self.held.ctrl {
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
            // still known by its scan code, and FR-23 carries it through unchanged.
            None => KeyMapping::EMPTY,
        }
    }

    /// The map of the active layout, if there is a cache and the layout is in it.
    fn active_map(&self) -> Option<&crate::layouts::LayoutMap> {
        self.cache.as_ref()?.map_at(self.active_index?)
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
pub fn install(config: &settings::Buffer) {
    install_recorder(Recorder::from_config(config));
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

/// Flushes and zeroes the buffer of the calling thread — FR-10, SEC-02.
///
/// The public flush the other sources of the FR-10 table attach to; see [`Recorder::reset`].
pub fn reset() -> bool {
    with(Recorder::reset).is_some()
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
}
