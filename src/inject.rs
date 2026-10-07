//! Assembling `SendInput` packets, modifier hygiene, replacement modes.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-40 (the order of operations of a recognised hotkey
//! press, and the ban on `SendInput` inside the callback), FR-41 (one `SendInput` call with the
//! whole array, `N` backspaces by *characters*, the recoded text through `KEYEVENTF_UNICODE`,
//! surrogate pairs), **FR-42** (the compatibility mode: `N` × `Shift+Left` and an insertion that
//! replaces the selection, instead of `N` × `Backspace`), FR-43 (the replacement is formed and
//! sent **before** the layout is switched), FR-44 (the configurable pause between events),
//! FR-45 (the return value of `SendInput` is compared with the number of events sent).
//! Boundary: the layout switch of FR-50 that FR-40 step 5 asks for belongs to module `switch`
//! (task T-05-1) and is reached from the marked connection point in [`replace_in_with`], which
//! fixes its *position* and nothing else; the *choice* of the target layout
//! belongs to task T-05-2 and arrives here as a parameter; the **selection path of FR-60 to
//! FR-65 is a different requirement and a different task (T-07-2)** — there the text is one the
//! *user* has already selected and it travels through the clipboard, whereas FR-42 selects the
//! program's own freshly typed run and no line of this module opens, reads or writes the
//! clipboard.
//! Implemented by backlog tasks: T-04-1, T-04-2.
//!
//! # Why the work is split into building and sending
//!
//! Everything FR-41 states is a property of an *array*: what is in it, in which order, and how
//! many `INPUT` structures a character outside the BMP takes. So the array is built by pure
//! functions into a slice the caller owns — [`build_replacement`], [`build_release`],
//! [`build_restore`] — and handed to a separate dispatcher, [`dispatch_in`], which is the
//! only place that counts calls and examines return values.
//!
//! FR-40 is not a property of an array but an **order**, and an order performed by a function
//! that calls Win32 directly is invisible from outside it. So the four things steps 3 to 5 need
//! from the world — the modifier query, `SendInput`, the pause and the layout switch of step 5
//! — are the four members of [`Environment`], of which [`System`] is the real one. Step 6 is not
//! performed: see [`run_steps`] (task T-40-3, finding С12, §10 of SPEC).
//!
//! The consequence is that FR-03, FR-40, FR-41, FR-43, FR-44 and FR-45 can all be driven from
//! `tests\inject.rs` without a keyboard, without a foreground window and without sending a
//! single event into the machine, which is the only way a test of this module can be both
//! meaningful and safe to run beside other tests.
//!
//! # ⚠ Why nothing here may be called from the hook callback
//!
//! FR-40 says it outright, and the mechanism is worth restating: `SendInput` with our own
//! `WH_KEYBOARD_LL` hook installed calls that hook back **synchronously, on the calling
//! thread**. Doing it from inside the callback is re-entrancy into a function the system is
//! already running, on a path NFR-01 gives a hundred microseconds. The callback therefore does
//! one thing with a recognised hotkey — `PostMessageW(WM_APP_HOTKEY)` — and this module runs
//! from the input thread's ordinary message loop, where section 6.1 puts `SendInput` and where
//! NFR-09 gives the whole path thirty milliseconds.
//!
//! The same re-entrancy is why [`on_hotkey`] copies the strokes out of the typing buffer
//! **before** it sends anything: [`crate::buffer::with`] is a `RefCell` borrow, our own
//! injected events are filtered out by FR-03 before they reach [`crate::buffer::record`], but a
//! stroke the *user* makes during the dispatch is not, and a borrow held across `SendInput`
//! would turn that into a panic.
//!
//! # Why a character is a down **and an up**
//!
//! A `KEYEVENTF_UNICODE` event carries a code unit in `wScan`, and it would be tempting to send
//! one structure per unit and call the character delivered. That is what this module did until
//! task T-10-4, and it is wrong in a way no ordinary control reveals.
//!
//! Qt renders a `KEYEVENTF_UNICODE` **keydown that is never released** not as the character of
//! that event but as the character currently latched, and it is the **keyup** that advances the
//! latch. Measured in Telegram Desktop 7.0.9 (task T-10-4, and the mechanism in T-10-0b before
//! it): `привет` as six down-only events puts `пппппп` on the screen — the first unit latches
//! and the next five are rendered as it — while the same six characters as down+up pairs put
//! `привет`. Nothing about the packet's *packaging* changes this: one call, one call per event
//! and pauses of 7, 10 and 35 ms between them were all measured and all give `пппппп`. The form
//! of the event is the whole of it.
//!
//! Three consequences are built into this module rather than left to the reader:
//!
//! * every code unit is [`EVENTS_PER_UNIT`] structures, in [`build_replacement`] and in
//!   [`build_selection`] alike — the compatibility mode of FR-42 inserts through the same run of
//!   character events and had the same defect;
//! * a packet therefore **ends on a keyup** and leaves the latch clean for whatever is typed
//!   next, which the down-only form did not: it left the last unit latched, and the first unit
//!   of the *next* packet was swallowed by it. That swallowed unit is the whole of the
//!   "first character is lost" that made the pairs look unusable when they were first measured;
//! * a character outside the BMP is four structures, and [`portion_end`] keeps all four in one
//!   `SendInput` call for the reason FR-41 gives — half a character delivered on its own is not
//!   a character delivered slowly.
//!
//! Ordinary controls are indifferent to all of it: a standard Win32 `EDIT` was measured under
//! both forms — plain text and a surrogate pair, with and without the erasure — and put the
//! same characters on the screen either way. That is why this is a change of form and not of
//! behaviour, and it is what the bench of §11.5 re-checks in Notepad, Word and Chrome.
//!
//! # SEC-01, SEC-02, SEC-07
//!
//! This module holds the user's text in plain form — that is what it is for — and it is the
//! last place in the program that does. Nothing here is formatted, journalled or written
//! anywhere: [`InjectError`] carries counts and never characters, there is no `Debug` on
//! anything that holds text, and the working buffers of [`replace`] are overwritten with zeroes
//! before they are released, which is the rule module `buffer` lives by (SEC-02) applied to the
//! one other place a keystroke exists in memory.
//!
//! **How** they are overwritten is part of that promise and not an implementation detail. Three
//! buffers are zeroed on the way out — the converted text and the `INPUT` packet of [`replace`],
//! and the copy of the strokes [`on_hotkey`] takes out of the recorder — and all three go
//! through `crate::buffer::zero_slice`, the volatile write module `buffer` uses on the ring:
//! `write_volatile` plus a `compiler_fence`. Until the audit of 2026-08-24 (task **T-13-15**)
//! they were zeroed with `slice.fill(...)` instead, which is the store that module's own
//! documentation calls deletable — «a plain assignment into a slot that is about to be
//! overwritten or dropped is a dead store the compiler is allowed to delete» — and under the
//! Release profile of section 3.2, `opt-level 3` with fat LTO, a memset in front of a `Vec`
//! being freed is exactly what dead-store elimination is for. The promise now says what the
//! code does.

use std::fmt;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE, SendInput,
    VIRTUAL_KEY, VK_BACK, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL,
    VK_RMENU, VK_RSHIFT, VK_RWIN,
};
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};

use crate::convert::{self, ConvertError, Keystroke};
use crate::hook::INJECTED_SIGNATURE;
use crate::layouts::{self, LayoutId, LayoutMap};
use crate::settings::ReplacementMethod;

// ---------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------

/// Why a packet could not be built.
///
/// SEC-01, SEC-07: every variant carries a count and nothing else. No character, no scan code
/// and no virtual key may ever be added to this type — it is the one value of this module that
/// is allowed to become a string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InjectError {
    /// The caller's array is shorter than the packet.
    ///
    /// `needed` is the exact length of the whole packet in `INPUT` structures, so a caller can
    /// size an array and call again rather than probe. Nothing is written when this is
    /// returned: a packet that was cut in half would break the very atomicity FR-41 exists for.
    OutputTooSmall {
        /// Length of the complete packet, in `INPUT` structures.
        needed: usize,
    },

    /// The conversion of the recorded strokes did not fit the working buffer.
    ///
    /// Unreachable through [`replace`], which sizes that buffer with
    /// [`crate::convert::max_units`], and kept as a value rather than as a panic because
    /// FR-45's discipline — examine, record, carry on — applies to this path too.
    TextTooLong {
        /// Length of the whole conversion, in UTF-16 code units.
        needed: usize,
    },

    /// ⭐ **The packet is empty — finding Н18 of the audit of 2026-09-04, task T-40-1.**
    ///
    /// The strokes put nothing on the screen and convert into nothing — a ring of mute keys (the
    /// function row, the volume keys), which FR-10 records like any other key. There is nothing to
    /// erase and nothing to type, and "accepted = requested" over zero events would read as
    /// "everything was taken": the idle press used to sound the tone of a replacement, switch the
    /// layout of the foreground window and move the position counter of FR-32.
    ///
    /// Refused **before** steps 3 to 5 run, so an idle press releases no modifier and switches no
    /// layout. Carries nothing at all — there is no count to report.
    EmptyPacket,

    /// ⭐ **The keys of this run cannot be repeated — stage Э96, task T-96-2.**
    ///
    /// The keystroke path of the classic console ([`Delivery::Keystrokes`]) types the replacement by
    /// pressing the user's keys again in the new layout, and that is the replacement only where the
    /// keys mean in the new layout exactly what the conversion of FR-22 renders: a dead key, a key
    /// under `AltGr`, a key the new layout gives nothing for or a dead key of the new layout would
    /// put something else on the line (see [`keys_repeat_the_conversion`]). Such a run is refused
    /// **before any event** — nothing is erased and no layout is switched, the word stays as typed
    /// and the press is answered with the tone of a refusal: the fallback the owner chose for the
    /// classic console (decision 160.8, «запасной — не стирать»). Carries nothing.
    KeysCannotRepeat,
}

impl fmt::Display for InjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Counts, never the contents — SEC-07.
            Self::OutputTooSmall { needed } => {
                write!(f, "the packet needs {needed} input events")
            }
            Self::TextTooLong { needed } => {
                write!(f, "the replacement needs {needed} UTF-16 code units")
            }
            Self::EmptyPacket => write!(f, "the replacement packet is empty"),
            Self::KeysCannotRepeat => write!(f, "the keys of the run cannot be repeated"),
        }
    }
}

impl std::error::Error for InjectError {}

// ---------------------------------------------------------------------------------------
// FR-40 step 3 — the modifiers (step 6 is not performed: task T-40-3, finding С12)
// ---------------------------------------------------------------------------------------

/// The modifier keys FR-40 step 3 takes down.
///
/// One bit per **physical** key rather than one per role: `Shift` is two keys, and the release
/// has to name the one the user is actually holding. FR-40 names `Shift`, `Ctrl`, `Alt` and
/// `Win`, which is these eight keys and no others — `CapsLock` is a lock and not a held
/// modifier, and releasing it would change the state of the machine rather than restore it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers(u8);

impl Modifiers {
    /// Nothing held.
    pub const NONE: Self = Self(0);
    /// `VK_LSHIFT`.
    pub const LEFT_SHIFT: Self = Self(0b0000_0001);
    /// `VK_RSHIFT`.
    pub const RIGHT_SHIFT: Self = Self(0b0000_0010);
    /// `VK_LCONTROL`.
    pub const LEFT_CTRL: Self = Self(0b0000_0100);
    /// `VK_RCONTROL`.
    pub const RIGHT_CTRL: Self = Self(0b0000_1000);
    /// `VK_LMENU` — the left `Alt`.
    pub const LEFT_ALT: Self = Self(0b0001_0000);
    /// `VK_RMENU` — the right `Alt`, which is `AltGr` on the layouts that have one.
    pub const RIGHT_ALT: Self = Self(0b0010_0000);
    /// `VK_LWIN`.
    pub const LEFT_WIN: Self = Self(0b0100_0000);
    /// `VK_RWIN`.
    pub const RIGHT_WIN: Self = Self(0b1000_0000);

    /// Builds a mask from raw bits. Every bit pattern is a valid mask.
    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits)
    }

    /// The raw bits.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// The union of two masks.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every key of `other` is in `self`.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether nothing at all is held.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// How many keys the mask names — the number of events a modifier packet will build.
    pub const fn count(self) -> usize {
        self.0.count_ones() as usize
    }
}

/// How many `INPUT` structures a modifier packet can ever need.
pub const MODIFIER_COUNT: usize = 8;

/// One row of the modifier table: the bit, the key, and whether the key is an extended one.
type ModifierRow = (Modifiers, VIRTUAL_KEY, bool);

/// The eight keys of FR-40 step 3, in the order they are released.
///
/// The third column is the `E0` prefix of the physical key. `RightCtrl`, `RightAlt` and both
/// `Win` keys are extended and the rest are not; the flag is replayed on injection so that the
/// event the application receives names the key the user is actually holding, exactly as
/// FR-05 requires of the recording side.
const MODIFIER_KEYS: [ModifierRow; MODIFIER_COUNT] = [
    (Modifiers::LEFT_SHIFT, VK_LSHIFT, false),
    (Modifiers::RIGHT_SHIFT, VK_RSHIFT, false),
    (Modifiers::LEFT_CTRL, VK_LCONTROL, false),
    (Modifiers::RIGHT_CTRL, VK_RCONTROL, true),
    (Modifiers::LEFT_ALT, VK_LMENU, false),
    (Modifiers::RIGHT_ALT, VK_RMENU, true),
    (Modifiers::LEFT_WIN, VK_LWIN, true),
    (Modifiers::RIGHT_WIN, VK_RWIN, true),
];

/// Which of the eight modifier keys the user is holding **at this instant**.
///
/// `GetAsyncKeyState` and not the tracker of [`crate::buffer::Recorder::held`], for two
/// reasons that are both requirements rather than preferences:
///
/// * FR-40 names `Win`, and `crate::buffer::StrokeMods` has no bit for it — it is a mask of
///   what changes which character a key produces, and `Win` changes nothing;
/// * a tracker fed by hook events cannot answer better than the desktop's own key state, and it
///   cannot answer at all for a key that went down before the hook was installed.
///
/// # ⛔ Why it cannot answer step 6 — task T-40-3, finding С12, decision 127.2
///
/// Step 6 of FR-40 asks for the modifiers the user holds **physically** after the replacement.
/// This call cannot say: the asynchronous state follows the program's own injected events as
/// readily as the user's, so once step 3 has sent the release, it reports the user's `Shift` as
/// up however firmly it is held — and `crate::hook::physical_modifiers` reads the very same
/// state. A step 6 built on it restored the user's keys never, and pressed again only what
/// nobody was holding: a `Shift` of our own left down by a packet. So the main path asks this
/// once, at step 3, and step 6 is not performed; §10 of SPEC names the limitation.
///
/// `GetAsyncKeyState` is also the right *kind* of call here for the same reason
/// [`crate::hook`] uses it for FR-96: it reads the asynchronous state of the desktop, which is
/// updated ahead of the hooks and is unaffected by any suppression this program performs, while
/// `GetKeyState` would answer from the calling thread's input state — and the input thread of
/// this program never has the keyboard focus.
pub fn held_modifiers() -> Modifiers {
    let mut held = Modifiers::NONE;

    for (bit, key, _) in MODIFIER_KEYS {
        // SAFETY: `GetAsyncKeyState` takes a virtual-key code by value, returns a `SHORT`,
        // touches no memory of ours and cannot block — it reads a table the system keeps per
        // desktop. It is callable from any thread. The eight codes come from the constant
        // table above and are all documented virtual keys.
        let state = unsafe { GetAsyncKeyState(i32::from(key.0)) };

        if is_held(state) {
            held = held.with(bit);
        }
    }

    held
}

/// Whether a `GetAsyncKeyState` result means "held down".
///
/// The high bit is the state; the low bit means "was pressed since the last call" and is
/// deliberately ignored, because it would make the answer depend on who called before us. The
/// value is signed, so the test is written on the unsigned reinterpretation rather than on a
/// comparison that would be wrong for exactly the values that matter.
fn is_held(state: i16) -> bool {
    (state as u16) & 0x8000 != 0
}

/// **FR-40 step 3.** Writes an explicit release for every key of `mods`.
///
/// Returns how many events were written. `mods` empty writes nothing and is not an error: a
/// user who is holding nothing needs nothing released.
///
/// Why the step exists at all is in §4.5 of SPEC: a `Shift` held while the hotkey is pressed
/// turns the injected `Backspace` into `Shift+Backspace` and the injected characters into a
/// selection, so the user reaching for the hotkey with their little finger on `Shift` would get
/// text selected instead of text replaced.
pub fn build_release(mods: Modifiers, out: &mut [INPUT]) -> Result<usize, InjectError> {
    build_modifier_events(mods, out, true)
}

/// Writes a press for every key of `mods` — the inverse of [`build_release`].
///
/// ⚠ **No path of the program calls it since task T-40-3** (finding С12, decision 127.2): it was
/// the builder of FR-40 step 6, and step 6 is not performed — [`held_modifiers`] explains why it
/// could not be. It stays the one place a modifier *press* with the signature of FR-03 is built,
/// and that is what the benches of `tests\inject.rs` use it for: the packet that leaves a `Shift`
/// down (the trap of FR-42 the modelled keyboard has to be able to see) and the behavioural
/// checks that hold `Shift` for real. A second, hand-made press in the tests would be a second
/// table of modifier keys to keep in step with [`MODIFIER_KEYS`].
pub fn build_restore(mods: Modifiers, out: &mut [INPUT]) -> Result<usize, InjectError> {
    build_modifier_events(mods, out, false)
}

/// The body of both steps — one edge, one table walk.
fn build_modifier_events(
    mods: Modifiers,
    out: &mut [INPUT],
    up: bool,
) -> Result<usize, InjectError> {
    let needed = mods.count();

    if out.len() < needed {
        return Err(InjectError::OutputTooSmall { needed });
    }

    let mut written = 0;

    for (bit, key, extended) in MODIFIER_KEYS {
        if mods.contains(bit) {
            out[written] = key_event(key, extended, up);
            written += 1;
        }
    }

    Ok(written)
}

// ---------------------------------------------------------------------------------------
// FR-41 — the replacement packet
// ---------------------------------------------------------------------------------------

/// Virtual key of `Backspace`, the key the replacement of FR-41 erases with.
const BACKSPACE: VIRTUAL_KEY = VK_BACK;

/// First UTF-16 code unit of a surrogate pair.
const HIGH_SURROGATE_FIRST: u16 = 0xD800;

/// Last UTF-16 code unit that starts a surrogate pair.
const HIGH_SURROGATE_LAST: u16 = 0xDBFF;

/// How many `INPUT` structures one UTF-16 code unit takes: a keydown **and a keyup**.
///
/// Two, and the second is not decoration — see "Why a character is a down and an up" in the
/// module header. Every count in this module is expressed through this constant so that the
/// form of a character event is stated in exactly one place.
pub const EVENTS_PER_UNIT: usize = 2;

/// How many `INPUT` structures a replacement of `erase` characters by `units` code units takes.
///
/// `erase * 2` because FR-41 asks for `Backspace` **down and up**, and
/// `units * EVENTS_PER_UNIT` because a code unit travels as a down and an up as well — which is
/// what makes a character outside the BMP four structures without anything here having to know
/// about it.
pub const fn replacement_events(erase: usize, units: usize) -> usize {
    erase * 2 + units * EVENTS_PER_UNIT
}

/// **`N` of FR-41: how many characters the recorded strokes put on the screen as the user
/// typed them.**
///
/// Counted over the characters the keys *produced* and not over the keys themselves, which is
/// the whole point of the requirement: a ligature is one keystroke and several characters, and
/// erasing it with one `Backspace` would leave the rest of it on the screen. A key the layout
/// has no character for contributes nothing (FR-23) — there is nothing of its own on the
/// screen to erase.
///
/// ⚠ **A dead key contributes nothing either, and that is a statement about the screen rather
/// than about the buffer** — audit of 2026-08-31, finding №5, stage Э20. The stroke itself is
/// kept with its flag and its accent, exactly as FR-24 asks (`layouts::KeyMapping::dead`), so
/// `produced()` here is *not* empty; what it is not is a character standing on the screen on
/// its own. The prose that used to be here said "a dead key that produced nothing yet
/// contributes nothing" while the code counted its accent as one, and the two had never been
/// compared against the machine.
///
/// The measurement they were finally compared against — probe `deadkeys`, layout
/// `00020409` "United States-International", journal `scratchpad-Э20\замер-композиции.txt`:
///
/// | keys | on the screen | chars | strokes recorded |
/// |---|---|---|---|
/// | `' e` dead + a letter it composes with | `é` | **1** | 2 |
/// | `' t` dead + a letter it does not | `'t` | 2 | 2 |
/// | `' '` dead + dead | `''` | 2 | 2 |
/// | `' ␣` dead + space | `'` | **1** | 2 |
/// | `'` dead with nothing after it | *(nothing)* | **0** | 1 |
/// | `c a f ' e` | `café` | **4** | 5 |
///
/// **Why one rule and not the exact one.** Telling row 1 from row 2 means asking the OS with
/// the dead-key state *accumulating*, and that is what FR-06 forbids: this program decodes
/// with `TO_UNICODE_NO_STATE` precisely so that a sweep of its own never leaves an accent
/// hanging over every other application in the session. So a single rule has to serve all six
/// rows, and it can only err one way. Counting the dead stroke as **0** is exact for rows 1,
/// 4, 5 and 6 and one short for rows 2 and 3 — an accent left standing on the screen, which
/// the person can see and remove. Counting it as **1**, which is what the code did, is one too
/// many for rows 1, 4, 5 and 6 — and one `Backspace` too many silently eats the character
/// *before* the run, which belongs to somebody else's text. Sealed as **DECISIONS question
/// 75** together with the reading of `N` this whole function rests on.
///
/// Counted in Unicode scalar values and not in UTF-16 code units. The two differ only for a
/// character outside the BMP, and there the scalar value is the right unit: one `Backspace`
/// removes the whole character in every control that treats a surrogate pair as one, which is
/// what `Backspace` means, while two would eat the character before it.
pub fn typed_chars(strokes: &[Keystroke]) -> usize {
    strokes
        .iter()
        .map(|stroke| {
            if stroke.is_dead() {
                0
            } else {
                chars_in(stroke.produced().units())
            }
        })
        .sum()
}

/// **How many characters a *previous step of the cycle* left on the screen** — FR-41 for the
/// second press onwards, audit of 2026-08-31 finding №5, stage Э20.
///
/// From the second press on, what stands on the screen is not what the user typed: it is what
/// this program injected last time, rendered into `previous`. The two lengths are equal only
/// while every stroke converts one character to one character, and a dead key breaks that on
/// the very first press — `convert_stroke` carries a dead stroke over unchanged (FR-24), and
/// `KEYEVENTF_UNICODE` composes nothing, so the accent that the OS had folded into `é` while
/// the user typed comes back out as a character of its own.
///
/// Measured live on `00020409`: `c a f ' e` typed as `café` is **four** characters, and the
/// same strokes injected into Russian stand as `сфа'у` — **five**. Same strokes, two different
/// `N`, which is why this count exists separately from [`typed_chars`] and why a dead stroke is
/// **1** here and **0** there. Nothing is composed on this side, so every unit that went out is
/// a character standing on the screen.
fn injected_chars(strokes: &[Keystroke], previous: &LayoutMap) -> usize {
    strokes
        .iter()
        .map(|&stroke| chars_in(convert::convert_stroke(stroke, previous).units()))
        .sum()
}

/// **The `N` of FR-41 — how many characters of this run are on the screen right now.**
///
/// A type and not a bare `usize` for one reason: there are two right answers and they differ,
/// so a number passed by hand is a number that can be the wrong one. The only two ways to build
/// it are the two the program actually has, and both are named after the thing on the screen
/// they describe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OnScreen(usize);

impl OnScreen {
    /// The run as the **user typed it** — the OS composed the dead keys, see [`typed_chars`].
    pub fn as_typed(strokes: &[Keystroke]) -> Self {
        Self(typed_chars(strokes))
    }

    /// The run as a **previous step of the cycle injected it** — see [`injected_chars`].
    pub fn as_injected(strokes: &[Keystroke], previous: &LayoutMap) -> Self {
        Self(injected_chars(strokes, previous))
    }

    /// The count itself.
    pub const fn count(self) -> usize {
        self.0
    }
}

/// **Which layout's injection is standing on the screen right now** — the other half of the
/// `N` of FR-41 from the second press onwards (audit of 2026-08-31, finding №5, stage Э20).
///
/// `None` only while the run has **never been replaced**: then the screen holds what the user
/// typed and [`OnScreen::as_typed`] is the count. Once it has been replaced, the screen holds
/// the injection of the press that moved the counter *to* `position`, and that press rendered
/// the strokes into `Cycle::target(origin, position)`.
///
/// ⚠ **`replaced` and not `position == 0`** — the live experiment of 2026-08-31, taken by the
/// user after the first repair of this stage and against it. Position zero has *two* meanings:
/// no press yet, and a circle that has come all the way round to the origin. They are the same
/// number and they are not the same screen. The rollback press renders the same strokes into
/// the layout they were typed under, and with a dead key in the run **that is not what the user
/// typed**: `café` was typed as four characters, the rollback puts `caf'e` — five — because
/// `KEYEVENTF_UNICODE` composes nothing. Treating the closed circle as "the typing" erased four
/// where five stood, and one character of the run survived every lap: the person watched
/// `12345 cсфа'у` and then `12345 ccсфа'у` accumulate in front of the word.
///
/// So the question is not what the counter says but whether a conversion session is open, which
/// is exactly what `Recorder::in_conversion` answers — FR-10's last row opens it on a
/// replacement and the next ordinary key closes it.
///
/// ⚠ **`position`, not `position + 1`.** `position + 1` is the target of the press being
/// prepared right now — what is *about* to go on the screen, not what is on it.
pub fn showing(
    cycle: &layouts::Cycle,
    origin: LayoutId,
    position: usize,
    replaced: bool,
) -> Option<LayoutId> {
    if !replaced {
        return None;
    }

    cycle.target(origin, position).ok()
}

/// How many characters a run of UTF-16 code units is.
///
/// Every unit is a character except the leading half of a surrogate pair, which is half of the
/// character its neighbour completes.
fn chars_in(units: &[u16]) -> usize {
    units.len()
        - units
            .iter()
            .filter(|&&unit| is_high_surrogate(unit))
            .count()
}

/// How many **different** UTF-16 code units the character events of a built packet carry — task
/// **T-10-6**.
///
/// The measurement that tells a replacement of six different characters from a replacement of one
/// character six times over, and the only thing about the text of a packet that ever leaves this
/// module: it is a count, and no unit of the packet can be recovered from it (SEC-01, SEC-07).
///
/// ⚠ **Read off the `INPUT` structures and not off the text they were built from**, and that is
/// the whole point of the function. A count taken from the text would answer "the conversion came
/// out right", which was never in doubt; what the defect of this task needed answered is "the
/// array handed to `SendInput` really carried six different characters", and the only honest
/// place to ask that is the array. The keydown edge alone is counted, so that the pairing of
/// T-10-4 does not double every unit.
///
/// Quadratic in the length of the packet and deliberately so. It is compiled under the `testing`
/// feature alone — every Release build is without it, by condition 1 of SEC-04a — and even there
/// the run is a word, a few dozen units at the very most.
///
/// # ⭐ Counted over the packet itself, with no copy — task T-40-4, finding Н96
///
/// Until that task the character events were first collected into a `Vec<u16>` and counted
/// there: a **third** copy of the text beside the two working buffers of [`replace_in_with`], and
/// the one copy that went back to the allocator unzeroed — the vector even grew while it was being
/// filled, handing back a partial copy on the way (`tests\inject_zeroing.rs` found both blocks).
/// A copy counted here and zeroed afterwards would have been a repair; no copy at all is the
/// smaller one: each keydown is compared with the keydowns before it in the packet, which is
/// already a buffer this module zeroes. Nothing is allocated, so nothing is left behind.
#[cfg(feature = "testing")]
fn distinct_units(packet: &[INPUT]) -> usize {
    // The code unit of a character keydown — the edge T-10-4 counts — or `None` for every other
    // event of the packet.
    let unit_of = |event: &INPUT| {
        keyboard(event)
            .filter(|key| {
                key.dwFlags.0 & KEYEVENTF_UNICODE.0 != 0 && key.dwFlags.0 & KEYEVENTF_KEYUP.0 == 0
            })
            .map(|key| key.wScan)
    };

    packet
        .iter()
        .enumerate()
        .filter_map(|(index, event)| unit_of(event).map(|unit| (index, unit)))
        .filter(|&(index, unit)| {
            !packet[..index]
                .iter()
                .any(|earlier| unit_of(earlier) == Some(unit))
        })
        .count()
}

/// ⭐ Whether the packet's insertion is **something other than what the strokes already put on
/// the screen** — task **T-10-17**.
///
/// # What it compares, and why those are the two sides
///
/// The product knows two texts here and only two. One is what the strokes produced when they
/// were pressed, `Keystroke::produced`. The other is `inserted`, the conversion of those same
/// strokes into the target layout, as [`crate::convert::convert_strokes`] wrote it. Their
/// concatenations are compared, and the answer is one bit.
///
/// ⚠ It is a comparison of **texts**, and it is deliberately not the `N` of FR-41 — Э20. `N` is
/// a count of what stands on the screen and has two forms ([`OnScreen`]); this has one job, to
/// say whether the two sides of one press differ at all, and the strokes' own characters are the
/// right side of it on every press of the cycle.
///
/// Walked stroke by stroke against a shrinking tail rather than assembled into a second buffer:
/// this is a comparison, so it needs no copy of the user's text, and building one would be a
/// second place holding it in the clear (SEC-01, SEC-02). The per-stroke chunks are the
/// *produced* lengths, which need not match the converted ones — a stroke may give one character
/// in one layout and a ligature in another — so the walk consumes the tail by produced length and
/// requires it to be exhausted at the end, which is concatenation equality and nothing weaker.
///
/// ⚠ **This is not "the screen changed", and it must not be read as one.** Nothing in this
/// program reads the screen. When the stamp is stale the product's belief about what it is
/// erasing is wrong, and then this bit says `true` — the two sides it can see really do differ —
/// while the field does not move. That pair of readings is the measurement, not a defect of it:
/// see [`crate::control::LAST_REPLACEMENT_CHANGED`].
///
/// Compiled under the `testing` feature alone (condition 1 of SEC-04a), as [`distinct_units`] is,
/// and it costs one linear walk over a word.
#[cfg(feature = "testing")]
fn insertion_differs(strokes: &[Keystroke], inserted: &[u16]) -> bool {
    let mut rest = inserted;

    for stroke in strokes {
        let produced = stroke.produced();
        let units = produced.units();

        let Some((head, tail)) = rest.split_at_checked(units.len()) else {
            // The insertion ran out before the strokes did, so the two texts are of different
            // lengths and cannot be equal.
            return true;
        };

        if head != units {
            return true;
        }

        rest = tail;
    }

    // Anything left over is insertion the strokes do not account for.
    !rest.is_empty()
}

/// Whether `unit` is the leading half of a surrogate pair.
const fn is_high_surrogate(unit: u16) -> bool {
    unit >= HIGH_SURROGATE_FIRST && unit <= HIGH_SURROGATE_LAST
}

/// **FR-41.** Builds the whole replacement packet into `out`, in the order the requirement
/// fixes: `erase` × (`Backspace` down + up), then `text` as `KEYEVENTF_UNICODE` events, each
/// code unit a down and an up.
///
/// Returns how many events were written, which is always [`replacement_events`] of the two
/// lengths. `out` shorter than that is [`InjectError::OutputTooSmall`] and **nothing is
/// written**: FR-41 is about a packet that is whole and in order, and a half-written one is
/// worse than none.
///
/// `text` is UTF-16 as [`crate::convert::convert_strokes`] produced it, so a character outside
/// the BMP is already the two code units of its surrogate pair and becomes four adjacent
/// `INPUT` structures here without a special case. That adjacency is a requirement — a pair
/// split between two `SendInput` calls is two undefined characters — and it is what
/// [`portion_end`] keeps whole when FR-44 asks for portions.
///
/// Every structure written carries [`crate::hook::INJECTED_SIGNATURE`] in `dwExtraInfo`
/// (FR-03). Every structure. The signature is what stops the program's own output from
/// returning through the hook as the user's text and being converted a second time, for ever.
pub fn build_replacement(
    erase: usize,
    text: &[u16],
    out: &mut [INPUT],
) -> Result<usize, InjectError> {
    let needed = replacement_events(erase, text.len());

    if out.len() < needed {
        return Err(InjectError::OutputTooSmall { needed });
    }

    let mut written = 0;

    // The erasure first, and in full, before a single character is typed. FR-41 fixes the
    // order and the reason is the application on the other end: characters typed before the
    // erasure would be erased by it.
    for _ in 0..erase {
        out[written] = key_event(BACKSPACE, false, false);
        out[written + 1] = key_event(BACKSPACE, false, true);
        written += 2;
    }

    // `wVk = 0`, `wScan = символ` — FR-41 literally. `KEYEVENTF_UNICODE` delivers the code
    // unit to the application as text, bypassing the keyboard layout entirely, which is what
    // FR-43 rests on: there is no race with the layout switch of step 5 because the switch
    // cannot change what these events mean.
    //
    // Down **and up**, per unit — see "Why a character is a down and an up" in the module
    // header. The up is what advances Qt's latch, and without it every unit after the first is
    // rendered as the first.
    for &unit in text {
        out[written] = unicode_event(unit, false);
        out[written + 1] = unicode_event(unit, true);
        written += EVENTS_PER_UNIT;
    }

    Ok(written)
}

// ---------------------------------------------------------------------------------------
// FR-42 — the compatibility mode, through a selection
// ---------------------------------------------------------------------------------------

/// Virtual key of the left arrow, the key the compatibility mode of FR-42 selects with.
///
/// Sent as an **extended** key: the arrow of the navigation block carries the `E0` prefix and
/// the numeric keypad's `4` does not, and the two are the same virtual key. Without the flag the
/// application would be told the keypad key moved the caret, which is the same movement but the
/// wrong event, and FR-05 holds this program to naming the key it means on the recording side.
const LEFT_ARROW: VIRTUAL_KEY = VK_LEFT;

/// The `Shift` the compatibility mode presses **itself** — not the user's.
///
/// Left rather than right for no deeper reason than that a key had to be chosen and the table of
/// [`MODIFIER_KEYS`] names this one first. Which one it is does not matter, and *whose* it is
/// matters entirely: see [`build_selection`].
const SELECTION_SHIFT: VIRTUAL_KEY = VK_LSHIFT;

/// The two events that bracket the selection: `Shift` down before it, `Shift` up after it.
const SELECTION_SHIFT_EVENTS: usize = 2;

/// How many `INPUT` structures the compatibility packet of FR-42 takes.
///
/// `erase * 2` for the `Left` presses (down and up, as with `Backspace` in FR-41), plus the two
/// events of the bracketing `Shift`, plus [`EVENTS_PER_UNIT`] per UTF-16 code unit of the
/// insertion — the insertion is the same down+up run as FR-41's and is counted the same way.
///
/// Nothing typed is nothing to select: with `erase` at zero there is no selection to make and
/// therefore no `Shift` to press, and the packet is the insertion alone. A `Shift+Left` sent
/// then would select the character *before* the caret — someone else's text — and the insertion
/// would replace it.
pub const fn selection_events(erase: usize, units: usize) -> usize {
    if erase == 0 {
        units * EVENTS_PER_UNIT
    } else {
        SELECTION_SHIFT_EVENTS + erase * 2 + units * EVENTS_PER_UNIT
    }
}

/// **FR-42, the compatibility mode.** Builds the whole packet into `out`: `N` × `Shift+Left`,
/// then the insertion that replaces the selection.
///
/// Returns how many events were written, which is always [`selection_events`] of the two
/// lengths. `out` shorter than that is [`InjectError::OutputTooSmall`] and **nothing is
/// written** — the rule and the reason are [`build_replacement`]'s.
///
/// # Why the mode exists
///
/// §4.5 and constraint 3 of §10: in an application with autocompletion or autocorrection — an
/// IDE, a browser field with suggestions, Word — one `Backspace` may delete not one character
/// but the whole token that was substituted, so `N` backspaces stop corresponding to `N`
/// characters. A selection does not have that failure mode: `N` × `Shift+Left` selects exactly
/// `N` characters whatever the application thinks a word is, and the insertion replaces the
/// selection in one go rather than erasing anything.
///
/// # What is in the packet, and why it is one `Shift` and not `N`
///
/// ```text
/// Shift down                       one event, ours
/// Left down, Left up               × N, extended keys
/// Shift up                         one event, ours — before a single character is inserted
/// the recoded text                 one KEYEVENTF_UNICODE down + up per UTF-16 code unit
/// ```
///
/// `N` × `Shift+Left` is `N` repetitions of the *keystroke*, which is what a person does when
/// they hold `Shift` and tap `Left` `N` times: the modifier goes down once, the arrow repeats,
/// the modifier comes up once. Releasing and re-pressing `Shift` between the arrows would send
/// `4N` events instead of `2N + 2` for the same selection, and every application that watches
/// modifier transitions would see `N` of them where the user made one.
///
/// `N` is the same `N` as in FR-41 — [`OnScreen`], the number of **characters** standing on the
/// screen and not the number of keys pressed — so a ligature is selected whole, exactly as it is
/// erased whole in the other mode.
///
/// The insertion is the same `KEYEVENTF_UNICODE` run [`build_replacement`] ends with, and it is
/// what FR-42 means by "выделение заменяется одним событием": the first character event replaces
/// the whole selection by itself, so no `Delete` and no `Backspace` is sent at all. **The
/// clipboard is not involved** — that is the selection path of FR-60 to FR-65 and task T-07-2,
/// a different requirement about text the user selected.
///
/// # ⚠ The user's `Shift` and ours
///
/// Two `Shift`s meet on this path and confusing them breaks the feature in both directions.
///
/// * The **user's** `Shift` is released by FR-40 step 3, in a packet of its own, *before* this
///   one is sent — [`run_steps`]. It has to be: a user `Shift` still down when the arrows go out
///   would be indistinguishable from ours, and one still down when the characters go out would
///   turn the insertion into capitals.
/// * **Ours** goes down inside this packet and comes back up inside it, before the first
///   character. That is what makes the insertion an insertion and not a further selection, and
///   it is also what leaves the keyboard as it was found: a `Shift` of ours left down would stay
///   down for good, on a machine whose owner is holding nothing. (Until task T-40-3 FR-40 step 6
///   would even have pressed it again, reading it as the user's; step 6 is not performed now —
///   finding С12 — and the balance of this packet is what the hygiene rests on.)
///
/// So the packet is balanced by construction: the only `Shift` events in it are one down and one
/// up, in that order, and both are behind the last arrow and in front of the first character.
pub fn build_selection(
    erase: usize,
    text: &[u16],
    out: &mut [INPUT],
) -> Result<usize, InjectError> {
    let needed = selection_events(erase, text.len());

    if out.len() < needed {
        return Err(InjectError::OutputTooSmall { needed });
    }

    let mut written = 0;

    if erase > 0 {
        // Ours goes down here and comes up below — see the warning above.
        out[written] = key_event(SELECTION_SHIFT, false, false);
        written += 1;

        for _ in 0..erase {
            out[written] = key_event(LEFT_ARROW, true, false);
            out[written + 1] = key_event(LEFT_ARROW, true, true);
            written += 2;
        }

        // Up **before** the insertion, and inside the same packet, so that no ordering anywhere
        // else in the program has to be trusted for it.
        out[written] = key_event(SELECTION_SHIFT, false, true);
        written += 1;
    }

    // The same down+up run as [`build_replacement`], and for the same reason: the mode differs
    // in how the old text is taken off the screen, never in how the new text is put on it.
    for &unit in text {
        out[written] = unicode_event(unit, false);
        out[written + 1] = unicode_event(unit, true);
        written += EVENTS_PER_UNIT;
    }

    Ok(written)
}

// ---------------------------------------------------------------------------------------
// The two modes as one choice — FR-42, section 7
// ---------------------------------------------------------------------------------------

/// `[replacement] method` of section 7 — FR-42 and FR-42а, published by the UI thread.
///
/// Held as a code rather than as the enum because an atomic is the only way the input thread may
/// learn it (section 6.3), and there is no atomic of an enum. [`method_from_code`] is total, so
/// no value this can hold is a value the replacement path has to reason about.
static REPLACEMENT_METHOD: AtomicU8 = AtomicU8::new(AUTO_CODE);

/// [`ReplacementMethod::Backspace`] — the default of section 7 up to schema 1.
const BACKSPACE_CODE: u8 = 0;

/// [`ReplacementMethod::Selection`].
const SELECTION_CODE: u8 = 1;

/// [`ReplacementMethod::Auto`] — the default of section 7 (FR-42а), and the value the atomic
/// starts at.
const AUTO_CODE: u8 = 2;

/// Publishes `[replacement] method` — FR-42.
///
/// Section 6.3 fixes the direction, and it is the same one [`set_inter_event_delay_ms`] follows:
/// the UI thread read the file and publishes, the input thread reads the atomic. **The mode is
/// never read from the file on the replacement path** — NFR-09 leaves no room for a file read
/// between the hotkey and the last event of the replacement.
///
/// The type comes from [`crate::settings`] rather than being re-derived here: section 7 already
/// closes the set of values and `settings` already refuses anything outside it, so a second
/// spelling of the same two words in this module would be a second thing to keep in step.
pub fn set_replacement_method(method: ReplacementMethod) {
    REPLACEMENT_METHOD.store(method_code(method), Ordering::Relaxed);
}

/// The replacement mode as it stands — the **configured** value, [`ReplacementMethod::Auto`]
/// included. [`ReplacementMethod::Auto`] — the default of section 7 and of `settings` — until
/// the UI thread publishes something else. What a press actually runs is
/// [`effective_method`] of this value, decided per replacement.
pub fn replacement_method() -> ReplacementMethod {
    method_from_code(REPLACEMENT_METHOD.load(Ordering::Relaxed))
}

/// The code a mode is stored as.
const fn method_code(method: ReplacementMethod) -> u8 {
    match method {
        ReplacementMethod::Backspace => BACKSPACE_CODE,
        ReplacementMethod::Selection => SELECTION_CODE,
        ReplacementMethod::Auto => AUTO_CODE,
    }
}

/// The mode a stored code means.
///
/// Total on purpose: only [`set_replacement_method`] ever writes the atomic and it writes only
/// the three codes above, but a mode that could not be decided is not something the replacement
/// path may have to handle, so anything else reads as the documented default of section 7.
const fn method_from_code(code: u8) -> ReplacementMethod {
    match code {
        SELECTION_CODE => ReplacementMethod::Selection,
        BACKSPACE_CODE => ReplacementMethod::Backspace,
        _ => ReplacementMethod::Auto,
    }
}

/// How many `INPUT` structures the packet of `method` takes — [`replacement_events`] or
/// [`selection_events`].
///
/// `Auto` counts as `Selection` for the same reason [`build_packet`] builds it so — see there.
pub const fn packet_events(method: ReplacementMethod, erase: usize, units: usize) -> usize {
    match method {
        ReplacementMethod::Backspace => replacement_events(erase, units),
        ReplacementMethod::Selection | ReplacementMethod::Auto => selection_events(erase, units),
    }
}

/// Builds the packet of `method` — [`build_replacement`] (FR-41) or [`build_selection`] (FR-42).
///
/// The one place the two modes are chosen between. Everything downstream of it — the signature
/// of FR-03, the single call of FR-41, the portions of FR-44, the modifier hygiene of FR-40 and
/// the position of the layout switch of FR-43 — is shared by both modes and knows nothing about
/// which one it is carrying, which is the whole reason the choice is made here and not there.
///
/// `Auto` never arrives here through [`on_hotkey`], which resolves it against the foreground
/// window first ([`effective_method`]) — a packet builder is a pure function of its arguments
/// and must stay one, so it cannot ask a window anything. The arm exists because the function
/// has to be total, and it answers with the `Selection` packet: the refusal direction of
/// FR-42а, exactly as [`resolve_auto`] answers when the window class cannot be read.
pub fn build_packet(
    method: ReplacementMethod,
    erase: usize,
    text: &[u16],
    out: &mut [INPUT],
) -> Result<usize, InjectError> {
    match method {
        ReplacementMethod::Backspace => build_replacement(erase, text, out),
        ReplacementMethod::Selection | ReplacementMethod::Auto => build_selection(erase, text, out),
    }
}

// ---------------------------------------------------------------------------------------
// Stage Э96 — the keystroke path of the classic console (decision 160.8)
// ---------------------------------------------------------------------------------------

/// The scan code of the left `Shift` — set 1, main block — the `Shift` the keystroke path presses
/// around a key the user typed with `Shift`.
///
/// Left for the reason [`SELECTION_SHIFT`] is left: a key had to be chosen. Which one it is does not
/// matter; that it is **ours** and balanced inside the packet does — see [`build_keystrokes`].
const SCAN_LEFT_SHIFT: u16 = 0x2A;

/// How many `INPUT` structures the keystroke packet of `strokes` after `erase` backspaces takes.
///
/// `erase * 2` for the backspaces, as FR-41 counts them; two for every key (down, up), and two more
/// for a key typed with `Shift` — ours goes down before it and up after it.
pub fn keystroke_events(erase: usize, strokes: &[Keystroke]) -> usize {
    erase * 2
        + strokes
            .iter()
            .map(|stroke| if stroke.mods().shift() { 4 } else { 2 })
            .sum::<usize>()
}

/// **Whether the user's keys, pressed again under `target`, put on the line exactly what the
/// conversion of FR-22 renders** — the condition of the keystroke path (stage Э96).
///
/// The conversion asks the target layout what each physical key gives (FR-22), and so does a key
/// pressed again in a window switched to that layout — as long as the key really is one key and
/// one character. Where the conversion does anything else, the keys would not repeat it:
///
/// * **a dead key of the run** — the conversion carries it over unchanged (FR-24), while the key
///   pressed again would start a composition in the console;
/// * **a key the target layout gives nothing for, or a dead key there** — the conversion keeps the
///   stroke's own characters (FR-23), the key pressed again would give nothing, or a composition;
/// * **a key under `AltGr`** — the keystroke path presses `Shift` and nothing else around a key;
///   `AltGr` is `Ctrl` with the right `Alt`, a chord the classic console was never measured with
///   (the two layouts of the measuring machine give no letter under it).
///
/// `CapsLock` is not a reason: it is a lock, not a held key, and the console applies its state to
/// the repeated keys exactly as it did to the typed ones.
///
/// A run that fails this is refused whole — [`InjectError::KeysCannotRepeat`] — and the word stays
/// as typed: the fallback the owner chose for the classic console (decision 160.8).
pub fn keys_repeat_the_conversion(strokes: &[Keystroke], target: &LayoutMap) -> bool {
    strokes.iter().all(|&stroke| {
        let candidate = target.lookup(stroke.scan(), stroke.extended(), stroke.mods());

        !stroke.is_dead() && !stroke.mods().altgr() && !candidate.is_empty() && !candidate.is_dead()
    })
}

/// ⭐ **The keystroke packet of the classic console — stage Э96, decision 160.8.** Builds into
/// `out`: `erase` × `Backspace`, then every stroke of the run **again, by its scan code**.
///
/// Returns how many events were written, which is always [`keystroke_events`]. `out` shorter than
/// that is [`InjectError::OutputTooSmall`] and nothing is written — [`build_replacement`]'s rule.
///
/// ```text
/// Backspace down, Backspace up       × N — the characters on the screen, OnScreen, as FR-41
/// [Shift down]                        ours, only before a key the user typed with Shift
/// key down, key up                    KEYEVENTF_SCANCODE, the extended flag of FR-05
/// [Shift up]                          ours, at once — before the next key
/// ```
///
/// **Not one `KEYEVENTF_UNICODE` event.** The layout of the console decides what each key gives,
/// and the switch of FR-50 has already been made when this packet goes out — that order is
/// [`run_steps_ahead`]'s, and it is the whole of the path: the keys the user typed in one layout,
/// pressed in the other, are the replacement. `Backspace` does not depend on the layout, so the
/// erasure and the keys travel in one call (FR-41) after the switch.
///
/// The `Shift` here is **ours**, balanced inside the packet the way the compatibility packet's own
/// `Shift` is ([`build_selection`]): the user's `Shift` was released by step 3 before anything, and
/// a `Shift` of ours left down would stay down on a machine whose owner holds nothing.
///
/// Every structure carries the signature of FR-03 ([`scan_event`], [`key_event`]): the keys this
/// packet repeats come back through the hook as the program's own, not as the user's, and the next
/// press still converts the run the user typed (FR-32, the third press of П4).
pub fn build_keystrokes(
    erase: usize,
    strokes: &[Keystroke],
    out: &mut [INPUT],
) -> Result<usize, InjectError> {
    let needed = keystroke_events(erase, strokes);

    if out.len() < needed {
        return Err(InjectError::OutputTooSmall { needed });
    }

    let mut written = 0;

    for _ in 0..erase {
        out[written] = key_event(BACKSPACE, false, false);
        out[written + 1] = key_event(BACKSPACE, false, true);
        written += 2;
    }

    for stroke in strokes {
        let shift = stroke.mods().shift();

        if shift {
            out[written] = scan_event(SCAN_LEFT_SHIFT, false, false);
            written += 1;
        }

        out[written] = scan_event(stroke.scan(), stroke.extended(), false);
        out[written + 1] = scan_event(stroke.scan(), stroke.extended(), true);
        written += 2;

        if shift {
            out[written] = scan_event(SCAN_LEFT_SHIFT, false, true);
            written += 1;
        }
    }

    Ok(written)
}

// ---------------------------------------------------------------------------------------
// FR-42а — `auto`: the method resolved by the class of the foreground window
// ---------------------------------------------------------------------------------------

/// The window classes whose windows get the `Backspace` path under `method = "auto"` — FR-42а.
///
/// The two are the console hosts of Windows: `ConsoleWindowClass` is conhost — the classic
/// console window `cmd.exe` and every console program get by default — and
/// `CASCADIA_HOSTING_WINDOW_CLASS` is Windows Terminal, which hosts the same console sessions
/// behind its own window.
///
/// # ⚠ Why the list is closed, and where it comes from
///
/// It is the measurement of task T-10-7, not a taxonomy. That task ran both methods over the
/// whole matrix and every reachable live application, and the *only* place `selection` broke
/// was the console COOKED input: `Shift+Left` is not a selection there, so the insertion lands
/// in front of the unerased original (`приветghbdtn`, and worse in a series). `Backspace`, in
/// turn, was measured to hold in exactly those windows (positions 6–7 of the acceptance
/// matrix). So the rule is not "consoles are special", it is "these named window classes were
/// measured to need the other path". A class nobody measured has no business in this list —
/// adding one is a new measurement and a controller decision, not an edit.
///
/// The shells *inside* a console window do not matter and cannot be told apart from outside:
/// PowerShell (PSReadLine) and cmd live behind the same `CASCADIA_HOSTING_WINDOW_CLASS` in
/// Windows Terminal, and PowerShell was measured to work under **both** methods — which is what
/// makes the class, rather than the shell, the right thing to dispatch on (review of T-10-7).
///
/// ⭐ **Since stage Э96 the two no longer get one and the same packet** — see
/// [`CLASSIC_CONSOLE_CLASS`]. The list stays what it is for every other reader of it: the two
/// classes in which the erasure is `Backspace` and the selection path of FR-60 does not apply
/// (the note to §4.7, П-3).
pub const CONSOLE_WINDOW_CLASSES: [&str; 2] =
    ["ConsoleWindowClass", "CASCADIA_HOSTING_WINDOW_CLASS"];

/// ⭐ **The classic console — the one class whose replacement under `auto` is typed by keys**,
/// stage Э96, decision 160.8.
///
/// `ConsoleWindowClass` with Windows PowerShell 5.1 does not take `KEYEVENTF_UNICODE` (Э92, 4 of
/// 4; the product's own press, 10 of 10 — stand `--experiment-console-keys product`, 2026-10-07):
/// the backspaces of FR-41 reach the line, the characters after them do not, and the word the user
/// typed is gone. What the console does take are keys: the bench measured «the switch of FR-50,
/// then six backspaces and the six keys again, in one call» at 20 of 20 in both shells, the way
/// back as well, capitals included, **with no pause** between the switch and the packet (premises
/// П2 and П3 of the stage). So under `auto` this class gets [`Delivery::Keystrokes`]; Windows
/// Terminal, which takes `KEYEVENTF_UNICODE`, keeps [`Delivery::Backspace`].
///
/// One of the two entries of [`CONSOLE_WINDOW_CLASSES`], named apart so that the one rule that
/// separates them reads as a name and not as an index.
pub const CLASSIC_CONSOLE_CLASS: &str = CONSOLE_WINDOW_CLASSES[0];

/// **What a press really runs** — the method of FR-42 and FR-42а resolved, and the one path the
/// configuration does not name: the keys of the classic console (stage Э96, decision 160.8).
///
/// ⚠ **Not a value of section 7, and that is a decision, not an omission.** `[replacement] method`
/// is a closed set (`backspace`, `selection`, `auto`), and a new word in it would make an older
/// build send the whole file to quarantine (`closed-enum-downgrade-cost`, the price paid by schema
/// 4). The keystroke path is what `auto` chooses for one window class; nobody writes it in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    /// FR-41: `N` × `Backspace`, the replacement as `KEYEVENTF_UNICODE`, the switch after (FR-43).
    /// `backspace`, and `auto` in Windows Terminal.
    Backspace,
    /// FR-42: `N` × `Shift+Left`, the insertion, the switch after. `selection`, and `auto`
    /// everywhere but the consoles.
    Selection,
    /// ⭐ Stage Э96: the switch of FR-50 **first**, then one packet — `N` × `Backspace` and the keys
    /// the user pressed, again, by their scan codes. `auto` in a window of
    /// [`CLASSIC_CONSOLE_CLASS`] only. See [`build_keystrokes`].
    Keystrokes,
}

impl From<ReplacementMethod> for Delivery {
    /// The packet a configured method names — the two explicit ones are themselves, and `auto`,
    /// which never reaches a packet unresolved through [`on_hotkey`], answers the refusal direction
    /// of FR-42а exactly as [`build_packet`] does: the `Selection` packet.
    fn from(method: ReplacementMethod) -> Self {
        match method {
            ReplacementMethod::Backspace => Self::Backspace,
            ReplacementMethod::Selection | ReplacementMethod::Auto => Self::Selection,
        }
    }
}

impl PartialEq<ReplacementMethod> for Delivery {
    /// A delivery **is** a configured method when it is that method's packet: `Backspace` and
    /// `Selection` are, `Keystrokes` is no method of section 7, and nothing is `auto` — `auto` is a
    /// question, never an answer.
    fn eq(&self, method: &ReplacementMethod) -> bool {
        matches!(
            (self, method),
            (Self::Backspace, ReplacementMethod::Backspace)
                | (Self::Selection, ReplacementMethod::Selection)
        )
    }
}

/// **FR-42а: the pure half of the choice.** The delivery a window of class `class` gets under
/// `method = "auto"` — since stage Э96 three answers, not two: the classic console
/// ([`CLASSIC_CONSOLE_CLASS`]) gets [`Delivery::Keystrokes`], Windows Terminal
/// [`Delivery::Backspace`], every other window [`Delivery::Selection`].
///
/// `None` means the class could not be read at all — no foreground window, or a
/// `GetClassNameW` refusal ([`foreground_window_class`] documents both) — and an empty string
/// is a class no registered window can have. Both fall through to the `_` arm below.
///
/// # ⚠ The direction of the refusal is a decision, not an accident
///
/// Every unknown, empty or unreadable class answers **`Selection`**, and deliberately so:
/// the measurement of T-10-7 showed `selection` holding everything reachable *except* the
/// consoles — EDIT, Блокнот, Word, Chrome, VS Code, Telegram, PowerShell — while the consoles
/// are a short, enumerable list of window classes. So the closed list names the consoles and
/// everything else gets the method that held everywhere else. Falling back to `Backspace`
/// instead would hand every unknown application the one method that is *known* to race the
/// erasure on repeated presses (the collapsed series of T-10-6) in ordinary edit controls —
/// trading a measured-good default for a measured-bad one exactly where nothing is known.
///
/// Case-insensitively, because window class names are case-insensitive to Windows itself —
/// the same rule `guard::class_is_edit` states for its own comparison.
pub fn resolve_auto(class: Option<&str>) -> Delivery {
    match class {
        Some(class) if class.eq_ignore_ascii_case(CLASSIC_CONSOLE_CLASS) => Delivery::Keystrokes,
        Some(class)
            if CONSOLE_WINDOW_CLASSES
                .iter()
                .any(|console| class.eq_ignore_ascii_case(console)) =>
        {
            Delivery::Backspace
        }
        _ => Delivery::Selection,
    }
}

/// The class name of `window`, or `None` when it cannot be read.
///
/// The Win32 half of FR-42а, split from [`resolve_auto`] so that the resolution rule is a pure
/// function a test can drive with any class name it likes, while the one place that really
/// asks a window is this. Public for the same reason `guard::class_name` is: `tests\inject.rs`
/// puts a real window of its own class under it.
///
/// `None` and not an empty string for the failure, because the caller has to tell "the call
/// refused" apart from nothing at all — FR-42а sends both down the `Selection` path, and the
/// comment in [`resolve_auto`] says why, but the two must arrive as what they are rather than
/// be conflated here (NFR-13: the return value is examined, not smoothed over).
pub fn window_class(window: HWND) -> Option<String> {
    // 256 characters is the documented maximum length of a registered class name, plus room
    // for the terminator the call writes. The same sizing as `guard::class_name`.
    let mut buffer = [0u16; 257];

    // SAFETY: `buffer` is a live local array and the call is given the slice itself, so the
    // binding derives the bound from the array and cannot write past it. The call writes the
    // class name of `window` into it and touches nothing else of ours. NFR-13: a zero (or
    // negative) return is the documented failure — a window that died between the two calls of
    // `effective_method`, or an invalid handle — and is examined below.
    let length = unsafe { GetClassNameW(window, &mut buffer) };

    if length <= 0 {
        // Zero outcome 2 of FR-42а: the window exists as a handle but its class cannot be
        // read. Answered with `None`, which `resolve_auto` turns into `Selection`.
        return None;
    }

    Some(String::from_utf16_lossy(&buffer[..length as usize]))
}

/// **FR-42а, whole.** The method this press really runs: an explicit `backspace` or
/// `selection` passes through untouched — section 7 keeps them as manual overrides without
/// automatics — and `auto` is resolved by the class of the foreground window, right now.
///
/// # Where and when this runs
///
/// Called once per replacement, from [`on_hotkey`] — the input thread's message loop, the same
/// place the configuration is read (FR-42а: «одним вызовом `GetClassNameW` на потоке
/// интерфейса в момент замены»). It is **never** reached from the hook callback: the callback
/// ends at `PostMessageW`, and everything here runs after the handoff, where NFR-09's thirty
/// milliseconds apply rather than NFR-01's hundred microseconds. Two cheap `user32` reads fit
/// that budget; nothing here opens a file, takes a lock or allocates.
///
/// The window asked is the foreground window *at the moment of the replacement* — the same
/// window the packet is about to land in, which is the whole point of resolving late rather
/// than at configuration time.
///
/// Answers a [`Delivery`] since stage Э96: the keystroke path of the classic console is one of the
/// answers of `auto`, and of `auto` only — an explicit `backspace` in a classic console is still
/// the manual override it always was, `KEYEVENTF_UNICODE` included.
pub fn effective_method(configured: ReplacementMethod) -> Delivery {
    match configured {
        ReplacementMethod::Auto => {
            // SAFETY: `GetForegroundWindow` takes no arguments, returns a handle by value and
            // touches no memory of ours; callable from any thread.
            let foreground = unsafe { GetForegroundWindow() };

            let class = if foreground.is_invalid() {
                // Zero outcome 1 of FR-42а: there is no foreground window at all — the
                // documented NULL return, seen around desktop switches and lock screens.
                // `None` reaches the `_` arm of `resolve_auto`: the `Selection` path.
                None
            } else {
                window_class(foreground)
            };

            resolve_auto(class.as_deref())
        }
        explicit => Delivery::from(explicit),
    }
}

// ---------------------------------------------------------------------------------------
// Building one `INPUT` — FR-03
// ---------------------------------------------------------------------------------------

/// One keyboard `INPUT` for a virtual key.
///
/// ⚠ **FR-03.** `dwExtraInfo` is [`crate::hook::INJECTED_SIGNATURE`] here and in
/// [`unicode_event`]. A structure assembled without it would come back through the hook as the
/// user's own typing, `hook::classify` would let it into the buffer, and the next hotkey press
/// would convert the program's own output — for ever.
///
/// # The four builders of this program
///
/// This comment used to say that the two functions of this module were the only places an
/// `INPUT` is built and that there was deliberately no third. That stopped being true at task
/// T-07-2 and stayed on the page, which is the worst state an inventory can be in: the next
/// reader greps by it and stops one place short. Stage Э96 added the fourth, and the heading moved
/// with it in the same commit. The list, in full:
///
/// 1. [`key_event`] — this function. Every virtual key the program sends: the `Backspace` pairs
///    of FR-41, the `Shift+Left` selection of FR-42 and the modifier packets of FR-40 steps 3
///    and 6.
/// 2. [`unicode_event`] — every character of a replacement, `KEYEVENTF_UNICODE`, FR-41.
/// 3. `selection::key_event` in `src\selection.rs` — the four events of `Ctrl+C` and of
///    `Ctrl+V`, the two chords of the selection path of §4.7. **It exists as a third builder
///    rather than as a call of this one because `src\inject.rs` was closed to task T-07-2 and
///    both builders here are private.** That reason is recorded at the duplicate itself and is
///    not an invitation to merge the two: doing so is a refactor of its own, with its own task.
/// 4. [`scan_event`] — stage Э96: every key of the keystroke path of the classic console, the
///    user's keys again by their scan codes and the `Shift` around a key that had it.
///
/// # What actually holds the invariant
///
/// Not this list — a comment holds nothing. The signature is held mechanically, by tests that
/// read the events which really left rather than a builder called in isolation:
///
/// * `tests\inject.rs::every_input_sent_carries_the_injected_signature_of_fr03` — every event
///   of all three packets of a `Backspace` replacement;
/// * `tests\inject.rs::every_input_of_the_compatibility_mode_carries_the_injected_signature_of_fr03`
///   — the same for the `Shift+Left` path, arrows and `Shift` included;
/// * `tests\inject.rs::the_signature_is_on_every_modifier_event_of_both_directions` — the
///   release packet and the modifier press of [`build_restore`], including modifiers a plain
///   replacement never produces;
/// * `tests\selection.rs::the_two_chords_carry_the_signature_of_fr03_and_go_out_through_inject`
///   — every field of every event of the third builder, on both chords;
/// * `tests\inject.rs::every_event_of_the_keystroke_path_carries_the_signature_of_fr03` — the
///   fourth builder, keys and `Shift`, beside the backspaces of the same packet.
///
/// What none of them can see is a **fourth** builder whose events never reach one of those
/// paths. That is why the inventory above is spelled out by name rather than as a count: a
/// number is a thing to update, and a number nobody updated is how this comment became untrue.
///
/// `time` is left at zero, which asks the system to stamp the event itself. That is not
/// cosmetic: FR-12 resolves flush races by comparing `KBDLLHOOKSTRUCT.time` against the
/// timestamps of the asynchronous flush sources, and a timestamp we invented would be
/// comparable with neither.
///
/// `wScan` is left at zero as well. Without `KEYEVENTF_SCANCODE` the system derives the scan
/// code from `wVk` itself, which is the reliable direction: the mapping depends on the
/// keyboard layout of the receiving thread, and the system knows that layout while this thread
/// does not.
fn key_event(vk: VIRTUAL_KEY, extended: bool, up: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);

    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    if up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTED_SIGNATURE,
            },
        },
    }
}

/// One keyboard `INPUT` carrying a UTF-16 code unit — FR-41, `wVk = 0`, `wScan = символ`.
///
/// `up` is the edge, and a unit is always sent as both of them: `KEYEVENTF_UNICODE` names the
/// character and `KEYEVENTF_KEYUP` names the release of it, exactly as they do for a virtual
/// key. Why the release is mandatory rather than tidy is in the module header — Qt renders an
/// unreleased unicode keydown as the latched character, so a run of downs alone comes out as
/// the first character repeated.
///
/// See [`key_event`] for `dwExtraInfo`, `time` and for the inventory of the four `INPUT`
/// builders of this program — this is the second of them. In particular the signature of FR-03
/// is on **both** edges: an up without it would come back through the hook as the user's own
/// keystroke.
fn unicode_event(unit: u16, up: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;

    if up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTED_SIGNATURE,
            },
        },
    }
}

/// One keyboard `INPUT` naming a **physical key by its scan code** — the keystroke path of the
/// classic console, stage Э96, the fourth builder of [`key_event`]'s inventory.
///
/// `KEYEVENTF_SCANCODE`: `wScan` names the key, `wVk` is ignored by the system and left at zero,
/// and the extended flag of FR-05 says which of two keys with one scan code it is. The system
/// turns the key into a virtual key and a character by the layout of the window that receives it —
/// which is the whole point: the keys the user pressed in one layout, pressed again after the
/// switch, are the replacement in the other (decision 160.8). Not a `KEYEVENTF_UNICODE` event,
/// which the classic console with Windows PowerShell 5.1 does not take (Э92).
///
/// See [`key_event`] for `dwExtraInfo` and `time`: the signature of FR-03 is on **every** event,
/// both edges, the `Shift` around a capital included — an event without it would come back
/// through the hook as the user's own stroke and be converted next time.
fn scan_event(scan: u16, extended: bool, up: bool) -> INPUT {
    let mut flags = KEYEVENTF_SCANCODE;

    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    if up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTED_SIGNATURE,
            },
        },
    }
}

/// The keyboard part of `event`, or `None` if `event` is not a keyboard event.
///
/// The one safe way to look inside an `INPUT` this module built. `INPUT` carries a union and
/// reading the wrong arm of it is exactly the kind of thing NFR-14 exists to make deliberate;
/// offering the read once, here, keeps the `unsafe` in one place instead of in every caller and
/// in every test that checks FR-03.
pub fn keyboard(event: &INPUT) -> Option<KEYBDINPUT> {
    if event.r#type != INPUT_KEYBOARD {
        return None;
    }

    // SAFETY: `INPUT::Anonymous` is a union of three `#[repr(C)]` plain-data structures, and
    // `r#type` is the discriminant that says which arm is live — the check above establishes
    // that it is `ki`. The read copies a `KEYBDINPUT`, which is `Copy` and contains no pointer,
    // out of a reference the caller guarantees is valid for the call.
    Some(unsafe { event.Anonymous.ki })
}

// ---------------------------------------------------------------------------------------
// The menu mask — task T-93-3, вопрос 157 (посылка П5)
// ---------------------------------------------------------------------------------------

/// The virtual key of the menu mask — `0x07`, a code Windows leaves unassigned, with no
/// character in any layout and no meaning to any application. Task T-93-3.
pub const MENU_MASK_VK: u16 = 0x07;

/// The two events of the menu mask for a hotkey carrying `modifiers` — or `None` when the set
/// has no `Alt`, and then nothing is ever sent. Task T-93-3, вопрос 157, посылка П5.
///
/// # Why a hotkey with `Alt` needs one
///
/// FR-95 suppresses the key of the hotkey, so of `Alt+F12` the application sees `Alt` go down and
/// `Alt` come up with nothing between — **a lone `Alt`**, which `DefWindowProc` answers by taking
/// the window into its menu (`SC_KEYMENU`). The keystrokes of the replacement would then land in
/// the menu. The remedy is the one AutoHotkey ships as its menu mask key: a press and a release of
/// a key nobody uses, between the `Alt` going down and the `Alt` coming up, so that the release
/// is no longer a lone one. Measured on the bench before it was written (`scratchpad-E93`,
/// `--experiment-combination`): the witness window counts `SC_KEYMENU`.
///
/// Built by [`key_event`], so the mask carries the signature of FR-03 like every event of this
/// program: the hook passes it on untouched and the buffer never records it.
pub fn menu_mask(modifiers: u16) -> Option<[INPUT; 2]> {
    if modifiers & crate::hook::MOD_ALT == 0 {
        return None;
    }

    let mask = VIRTUAL_KEY(MENU_MASK_VK);

    Some([key_event(mask, false, false), key_event(mask, false, true)])
}

/// Sends the menu mask of the published hotkey, if it carries `Alt` — task T-93-3.
///
/// Called by the input thread on `hook::WM_APP_HOTKEY`, **first**, before the press is given to
/// either path: the user still holds `Alt` at that moment, and whatever follows — the release of
/// step 3 of FR-40, the `Ctrl+C` of the selection path, the person letting go — comes after the
/// mask. A hotkey without `Alt` sends nothing. FR-45: a mask the system did not take whole is
/// counted like any other discrepancy.
pub fn send_menu_mask() {
    let (_, modifiers) = crate::hook::hotkey();

    let Some(events) = menu_mask(modifiers) else {
        return;
    };

    let accepted = send_input(&events);

    record_discrepancy(Dispatched {
        calls: 1,
        requested: events.len(),
        accepted: usize::try_from(accepted).unwrap_or(0),
    });
}

// ---------------------------------------------------------------------------------------
// FR-41, FR-44, FR-45 — sending
// ---------------------------------------------------------------------------------------

/// `SendInput` calls whose return value differed from the number of events handed to them.
static SEND_MISMATCHES: AtomicU32 = AtomicU32::new(0);

/// Events lost across every such call — the size of the discrepancy, not merely its count.
static EVENTS_LOST: AtomicU32 = AtomicU32::new(0);

/// `[replacement] inter_event_delay_ms` of section 7 — FR-44, published by the UI thread.
static INTER_EVENT_DELAY_MS: AtomicU32 = AtomicU32::new(0);

/// **FR-45.** How many `SendInput` calls returned a count other than the one they were given,
/// and how many events were lost across all of them.
///
/// FR-45 in its corrected form (SEC-07, module `diag`) asks for the discrepancy to be recorded.
/// Module `diag` is task T-06-4 and does not exist yet, so the discrepancy is *counted* here
/// and offered through this function; wiring it into the journal is that task's, and this is
/// the interface it attaches to. The shape follows [`crate::hook::unhook_failures`], which
/// answers the same kind of question for the same reason.
///
/// SEC-07: two counts. Neither says which events were lost, and neither may ever be made to.
pub fn send_mismatches() -> (u32, u32) {
    (
        SEND_MISMATCHES.load(Ordering::Relaxed),
        EVENTS_LOST.load(Ordering::Relaxed),
    )
}

/// ⭐ **The ceiling of `[replacement] inter_event_delay_ms` — task T-13-13.**
///
/// The *default* of section 7 is zero and is stated two functions below, in
/// [`inter_event_delay_ms`]; this is the other end of the same field, and the pair is the shape
/// [`crate::buffer`] already has for `[buffer] capacity`, where `DEFAULT_CAPACITY` — «the default
/// of `[buffer] capacity` in section 7» — and `MAX_CAPACITY` stand side by side with
/// `effective_capacity` between them.
///
/// ⚠ **Nothing in this module enforces it, and that is deliberate.** The clamp stands on the one
/// publication, in `crate::app::publish_configuration`, and the atomic below therefore already
/// holds a value that is inside this bound. Enforcing it *here*, in `set_inter_event_delay_ms` or
/// in `inter_event_delay_ms`, would be either a second truth (the atomic saying one thing and the
/// readers another) or a comparison paid for on every press — and there are many presses per
/// publication. The constant lives here because the field lives here; the decision lives where
/// the file crosses into the program.
///
/// # Why a thousand and not the `u32` a hand-edited file can hold
///
/// The pause of FR-44 becomes `sleep_ms` — `thread::sleep` — on the **input thread**, which is the
/// thread section 6.1 puts the `WH_KEYBOARD_LL` hook and the watchdog timer on. A sleeping thread
/// pumps no messages and answers no callback, so every millisecond of this field is a millisecond
/// in which the hook callback of every keystroke **of the whole machine** goes unanswered. FR-80
/// names the consequence: the system removes a hook that overruns `LowLevelHooksTimeout`
/// (`HKCU\Control Panel\Desktop`, about five seconds by default) and it removes it **silently** —
/// the watchdog that would put it back is on the same sleeping thread, and the fail-safe of FR-96
/// lives inside the callback of a hook that is no longer installed. So the field a file can set to
/// `u32::MAX` (4 294 967 295 ms, about forty-nine days) is a field one slipped zero can use to
/// freeze the keyboard of the session on the first press of the hotkey. That is the finding of the
/// audit of 2026-08-24 this constant answers.
///
/// A thousand milliseconds keeps a **single** gap inside that `LowLevelHooksTimeout` budget with
/// room to spare, and turns the worst case of a whole replacement from tens of days into tens of
/// seconds: six characters replaced by the `backspace` path are twenty-four events and therefore
/// twenty-three pauses — twenty-three seconds, something a person waits out rather than a machine
/// they restart. It is still far outside the thirty milliseconds NFR-09 gives the whole path, and
/// deliberately so: a pause above zero **is** the explicit trade of that budget for compatibility,
/// which is the only reason FR-44 exists, and this bounds the trade instead of forbidding it.
///
/// The number is not derived here. It is the one the stage-13 repair specification fixes
/// (`reports\ТЗ-Э13-ремонт.md`, task T-13-13), together with the two of [`crate::selection`], so
/// that its origin does not get lost the next time somebody asks where a thousand came from.
pub const MAX_INTER_EVENT_DELAY_MS: u32 = 1_000;

/// Publishes `[replacement] inter_event_delay_ms` — FR-44.
///
/// Section 6.3 fixes the direction: the configuration is published by the UI thread, which is
/// the only thread allowed to read the file, and the input thread reads the atomic. Nothing
/// here reads a file, which is what NFR-09 requires of everything on the replacement path.
///
/// What arrives here is the value `crate::app::publish_configuration` has already put through
/// [`MAX_INTER_EVENT_DELAY_MS`] — see there and see the constant. This function stores what it is
/// given and asks no questions, exactly as it did before task T-13-13.
pub fn set_inter_event_delay_ms(delay_ms: u32) {
    INTER_EVENT_DELAY_MS.store(delay_ms, Ordering::Relaxed);
}

/// The pause of FR-44 as it stands, in milliseconds. Zero — the default of section 7 — is the
/// ordinary case and the one FR-41 describes.
pub fn inter_event_delay_ms() -> u32 {
    INTER_EVENT_DELAY_MS.load(Ordering::Relaxed)
}

/// What one dispatch did — the evidence for FR-41, FR-44 and FR-45.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dispatched {
    /// `SendInput` calls made. **Exactly one** whenever the packet is non-empty and the pause
    /// of FR-44 is zero, which is what FR-41 requires of the ordinary case.
    pub calls: usize,
    /// Events handed to `SendInput`, summed over the calls.
    pub requested: usize,
    /// Events `SendInput` reported it had inserted, summed over the calls — FR-45.
    pub accepted: usize,
}

impl Dispatched {
    /// Whether the system took everything it was given — FR-45.
    pub const fn is_complete(self) -> bool {
        self.accepted == self.requested
    }
}

/// Everything steps 3 to 5 of FR-40 need from outside this program.
///
/// Four members, and each of them is a requirement rather than a convenience: the modifier
/// query of step 3, the `SendInput` of FR-41, the pause of FR-44 and the layout switch of step 5.
/// [`System`] is the real one and is what the program runs on. Step 6 is not performed (task
/// T-40-3, finding С12) and asks this trait for nothing.
///
/// # Why it is a trait and not four direct calls
///
/// FR-40 is an **order**, and an order is not observable from outside a function that performs
/// it. With the outside world behind this trait, `tests\inject.rs` can watch the sequence steps
/// 3 to 5 really produce — that the modifiers come off before the first character is replaced,
/// that the replacement is complete before the layout switch is reached, that nothing is pressed
/// again after it — without a keyboard, without a foreground window and without sending one event
/// into the machine. There is no other honest way to check an order.
///
/// It is also the seam the neighbouring tasks attach to: **T-05-1** fills in [`switch_layout`],
/// and the compatibility mode of FR-42 (task T-04-2) is a different packet handed to these very
/// same steps — which is why the order below is checked once and holds for both modes.
///
/// [`switch_layout`]: Environment::switch_layout
pub trait Environment {
    /// The modifier keys held **at this instant** — FR-40 step 3.
    ///
    /// Asked **once** per replacement since task T-40-3. It used to be asked a second time for
    /// step 6, and on the real machine the second answer could never name the user's keys: the
    /// asynchronous state had already followed step 3's own release — see [`held_modifiers`].
    fn held(&mut self) -> Modifiers;

    /// Hands one portion to the system and answers how many events it accepted — FR-41, FR-45.
    fn send(&mut self, events: &[INPUT]) -> u32;

    /// The pause of FR-44. Reached only when the configured delay is above zero.
    fn pause(&mut self, delay_ms: u32);

    /// **FR-40 step 5 — the layout switch. The connection point of task T-05-1.**
    ///
    /// Called after the replacement has been sent — and only when it was taken whole, task
    /// T-40-2 — which is the position **FR-43** fixes: "Замена выполняется до переключения
    /// раскладки". The default does nothing, so a bench observes the *position* of the switch
    /// without performing one; [`System`] overrides it with the chain of §4.6 (FR-50 to FR-52),
    /// module `switch`.
    ///
    /// It is a member of this trait rather than a comment in the body so that the position of
    /// the switch is something a test can *see*: FR-43 is a statement about order, and an order
    /// nobody can observe is an order nobody can check.
    ///
    /// ⭐ **Stage Э96: the keystroke path of the classic console calls it *before* its packet**
    /// ([`run_steps_ahead`], decision 160.8) — the one exception to FR-43, and the reason the
    /// position of this call is something the tests read rather than assume.
    fn switch_layout(&mut self) {}

    /// ⭐ **The layout back where it was — the keystroke path only, stage Э96.**
    ///
    /// Called after a packet of the keystroke path that did **not** reach the screen whole
    /// ([`Replaced::reached_the_screen`]): the switch was made before the packet, so a blocked
    /// injection would otherwise leave the text as typed under a layout the user did not choose,
    /// and the next word would come out in the other language — finding Н12, decision 127.3,
    /// which the paths that switch *after* the packet keep by not switching at all. The default
    /// does nothing, for the reason [`switch_layout`](Self::switch_layout)'s does.
    fn switch_back(&mut self) {}
}

/// The real machine — the [`Environment`] the program runs on.
///
/// Everything it does is a system call, so the one thing it carries is the argument step 5 needs
/// and the trait cannot pass: see [`System::target`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct System {
    /// **The target of FR-40 step 5** — the layout the replacement was rendered into, and so the
    /// layout the foreground window is switched to once the replacement is out.
    ///
    /// It is a field and not an argument of [`Environment::switch_layout`] because changing that
    /// signature would change the bench of `tests\inject.rs`, which is not task T-05-1's to
    /// touch; the seam task T-04-1 left carries no target of its own. `None` — the value
    /// [`Default`] gives — means "do not switch", which is what [`dispatch`] wants: it sends a
    /// packet and performs no step 5 at all.
    target: Option<LayoutId>,

    /// ⭐ **The classic console — stage Э96.** `Some` with the layout the window was in before the
    /// press, for the keystroke path: its switch is made without a verdict
    /// ([`switch_classic_in`]) and [`Environment::switch_back`] returns to this layout. `None` for
    /// every other path, which switches through the chain of §4.6 and never back.
    back: Option<LayoutId>,
}

impl System {
    /// The real machine for a replacement that ends in the layout switch of FR-40 step 5.
    pub const fn for_target(target: LayoutId) -> Self {
        Self {
            target: Some(target),
            back: None,
        }
    }

    /// ⭐ The real machine for the keystroke path of the classic console — stage Э96: the switch
    /// to `target` ahead of the packet, and `back` to return to when the packet did not reach the
    /// screen.
    pub const fn for_classic_console(target: LayoutId, back: LayoutId) -> Self {
        Self {
            target: Some(target),
            back: Some(back),
        }
    }
}

impl Environment for System {
    fn held(&mut self) -> Modifiers {
        held_modifiers()
    }

    fn send(&mut self, events: &[INPUT]) -> u32 {
        send_input(events)
    }

    fn pause(&mut self, delay_ms: u32) {
        sleep_ms(delay_ms);
    }

    /// **FR-40 step 5 — task T-05-1.** The chain of FR-50, in module `switch`.
    ///
    /// This runs on the input thread, in its message loop, with the hook callback long returned
    /// — module `switch` documents why none of the three methods may be reached from inside the
    /// callback, and it is the same reason `SendInput` may not be.
    ///
    /// # ⚠ The stamp of FR-04 follows the switch — task **T-10-5**
    ///
    /// The outcome used to be dropped, on the argument that module `switch` counts every refusal
    /// itself and an injection can do nothing about a layout that would not change. The first
    /// half is still true; the second half was wrong, and the acceptance session felt the
    /// difference as «первое нажатие моргает».
    ///
    /// **What was measured** (report of task T-10-5, experiments `expA1`, `expA2b`, `expA3`):
    /// this line switches the foreground window, and *nothing* then tells the typing buffer.
    /// The four refresh paths of `app::publish_active_layout` are a probe on a focus change, a
    /// probe on a modifier release (T-03-3c), a device change and start-up — and a switch made
    /// **here** raises none of them: the focus does not move, the user pressed no modifier, and
    /// `WM_INPUTLANGCHANGE` cannot reach this process (T-03-2a). So `Recorder::active` kept the
    /// previous layout while the window ran the new one — measured as `active_layout=0x04090409`
    /// against a window on `0x04190419`, with `layout_probes` and `cache_builds` frozen across
    /// the press. Every stroke of the next word was then stamped with the stale layout
    /// (`buffer::Recorder::record`), [`take_press`] took its `origin` from that stamp, and
    /// `cycle.target(origin, 1)` answered the layout the text was *already* typed in: a full
    /// replacement that put back the very same characters.
    ///
    /// So the publication belongs exactly here, at the one place in the program that changes the
    /// layout without the user touching anything, and it is conditional: a refusal publishes
    /// nothing, because a window that did not move must not be reported as having moved — that
    /// would be the same defect with the sign flipped.
    ///
    /// # ⭐ Which question the condition asks — task **Т-14-6**
    ///
    /// [`crate::switch::stamp_follows`], and until 2026-08-25 it was [`crate::switch::confirmed`].
    /// The difference is one outcome: [`crate::switch::Outcome::Sent`], the classic console window
    /// where FR-52's addendum says no verdict can be taken at all. Under `confirmed` the stamp
    /// stayed put there — and stayed put for ever, because `app::layout_refresh_needed` answers
    /// `false` for an unreadable layout, so nothing later cleared it either. The window ran one
    /// layout, `Recorder::active` claimed another, and FR-26 took its direction from the claim.
    ///
    /// The user decided that on 2026-08-25 — «Двигать штамп на веру» — on the 60 of 60 Т-14-2
    /// measured through a channel this program does not have. ⚠ **The belief is in the predicate
    /// and not here.** This line asks a question; module `switch` owns what the answer means, why
    /// `confirmed` was left alone rather than widened, and what the trust is worth. A second copy
    /// of that reasoning at this call site is exactly what task T-10-5 refused to write.
    ///
    /// ⚠ **FR-32 and FR-33 are untouched by this, and that is a property of the design rather
    /// than a hope.** The stamp is what *future* strokes are recorded under; the strokes already
    /// in the ring keep the layout each was typed under (FR-32 stores them unchanged), and
    /// [`take_press`] reads `origin` from `strokes.first()`. So the second press of a double
    /// press still counts its cycle from the layout the run was typed in, and the rollback of
    /// FR-33 answers exactly what it answered before.
    ///
    /// ⚠ **FR-11 is untouched too:** `publish_active_layout` does not flush the buffer and there
    /// is no `reset` anywhere on this path — which is what makes it safe to move the stamp in the
    /// middle of a conversion session.
    ///
    /// SEC-01, SEC-07: what crosses this line is a layout handle and a boolean, never a stroke.
    ///
    /// ⭐ **Stage Э96: the classic console switches without a verdict** — [`switch_classic_in`],
    /// whenever [`System::back`] is set, which only [`System::for_classic_console`] does.
    fn switch_layout(&mut self) {
        let Some(target) = self.target else {
            return;
        };

        if self.back.is_some() {
            switch_classic_in(&mut crate::switch::System, target);
        } else {
            switch_layout_in(&mut crate::switch::System, target);
        }
    }

    /// The keystroke path's way back — stage Э96. The same switch without a verdict, to the layout
    /// the window was in before the press.
    fn switch_back(&mut self) {
        if let Some(back) = self.back {
            switch_classic_in(&mut crate::switch::System, back);
        }
    }
}

/// ⭐ **The switch of the keystroke path — FR-50 to a classic console window, and no verdict at
/// all** (stage Э96). Answers whether the stamp of FR-04 was published.
///
/// [`switch_layout_in`] asks the chain of §4.6, and the chain asks FR-52 first: a reading that
/// says the window is already on the target is answered by not posting at all, and a reading that
/// does not move after the post is answered with a refusal. In a classic console window neither
/// answer is about the console — **measured by the premise П5 of stage Э96**: the thread the
/// window's handle names is the thread of the program the console hosts, `GetGUIThreadInfo`
/// answered for it (the console of the stand, both shells) and the layout read of it was the one
/// that thread was born with, while the console switched under it 20 times out of 20. On the
/// stand the base build left the console Russian after every other round of `cmd` — by the code of
/// `to_in`, the rollback found the frozen reading already English and posted nothing (the channel
/// carries no counter of it). The keystroke path cannot afford that — its
/// keys mean what the layout of the console makes of them — so it posts the message of FR-50
/// every time, through [`crate::switch::to_classic_console_in`], and believes it, as FR-52's
/// addendum has believed the blind console since Т-14-6: the stamp follows ([`stamp_follows`]
/// answers `true` for [`crate::switch::Outcome::Sent`]).
///
/// [`stamp_follows`]: crate::switch::stamp_follows
pub fn switch_classic_in(machine: &mut impl crate::switch::Machine, target: LayoutId) -> bool {
    let follows =
        crate::switch::stamp_follows(crate::switch::to_classic_console_in(machine, target));

    if follows {
        crate::app::note_layout_switched(target);
    }

    follows
}

/// **FR-40 step 5 against any [`crate::switch::Machine`]** — the body of
/// [`Environment::switch_layout`] with the machine behind the seam module `switch` provides.
/// Answers whether the stamp of FR-04 was published.
///
/// # Why this exists as a function of its own
///
/// Because the decision it holds is one line of branching that no test could otherwise reach, and
/// it is the line task **Т-14-6** changed. [`System`] talks to the real desktop:
/// `crate::switch::to` reads the foreground window, posts to it and re-reads it, so a test of
/// [`Environment::switch_layout`] would need a foreground window in a known layout on the machine
/// a person is sitting at — and the one case that matters, [`crate::switch::Outcome::Sent`], needs
/// a **classic console window**, which is a thing no test can conjure on demand at all.
///
/// The [`Environment`] seam of this module does not help: its fake `switch_layout` records that
/// step 5 happened, which is what FR-43 needs and all it needs, and it never enters module
/// `switch`. So the machine of module `switch` is threaded through instead — the same seam, for
/// the same reason, that [`dispatch_in`] is to [`dispatch`] and `switch::to_in` is to `switch::to`.
/// With it, `tests\switch.rs` drives a blind machine — a console window, exactly — into this
/// decision and watches the stamp move.
///
/// ⚠ It is deliberately **not** where the meaning of the gate lives: the question is
/// [`crate::switch::stamp_follows`]'s and the answer's justification is its documentation.
pub fn switch_layout_in(machine: &mut impl crate::switch::Machine, target: LayoutId) -> bool {
    let follows = crate::switch::stamp_follows(crate::switch::to_in(machine, target));

    if follows {
        crate::app::note_layout_switched(target);
    }

    follows
}

/// **FR-41 and FR-44.** Sends `events` through the real system, and records what happened
/// (FR-45).
///
/// See [`dispatch_in`] for the rule.
pub fn dispatch(events: &[INPUT], delay_ms: u32) -> Dispatched {
    // No target: this sends a packet and performs no step 5. See `System::target`.
    dispatch_in(&mut System::default(), events, delay_ms)
}

/// **FR-41 against FR-44**, against any [`Environment`].
///
/// # The contradiction, and how it is resolved
///
/// FR-41 requires **one** `SendInput` call with the whole array, "атомарность и гарантия
/// порядка". FR-44 requires a configurable pause **between events**, and inside one call there
/// is nowhere to put it. The two cannot both hold for the same packet.
///
/// The resolution: the pause of FR-44 defaults to zero (section 7), and at zero this sends the
/// whole array in a single call, exactly as FR-41 says. A value above zero is the user
/// deliberately trading atomicity for compatibility with an application that drops events when
/// they arrive in a batch, which is the only reason FR-44 exists, and the events then leave in
/// portions with the pause between them. So FR-41 is the behaviour of every ordinary
/// installation and FR-44 is an explicit, opt-in exception rather than a silent weakening.
///
/// # What a portion is
///
/// One event, except that a character outside the BMP is never split: it is two code units and
/// therefore four `INPUT` structures, and two halves of a character delivered in separate calls
/// are two undefined characters, not one character delivered slowly. The character is
/// recognised from the events themselves — a `KEYEVENTF_UNICODE` **keydown** whose `wScan` is a
/// leading surrogate takes the rest of the character with it — so no side table has to be
/// carried from the builder to here and the two cannot disagree. See [`portion_end`] for why
/// the edge has to be part of that test.
///
/// # FR-45
///
/// The return value of every call is added up and compared with what was handed over. A
/// discrepancy is counted in [`send_mismatches`]; the call is not retried, because `SendInput`
/// stops at the first event another thread's injection blocked, and re-sending the tail would
/// duplicate whatever did get through.
pub fn dispatch_in(env: &mut impl Environment, events: &[INPUT], delay_ms: u32) -> Dispatched {
    let mut done = Dispatched::default();

    if events.is_empty() {
        // Nothing to send is not a failure and must not become a `SendInput` call with an
        // empty array, which the system rejects.
        return done;
    }

    if delay_ms == 0 {
        // FR-41, the ordinary case: one call, atomic, in order.
        done.calls = 1;
        done.requested = events.len();
        done.accepted = env.send(events) as usize;
    } else {
        // FR-44, the opt-in case.
        let mut start = 0;

        while start < events.len() {
            if start > 0 {
                // *Between* events, which is what FR-44 says: no pause before the first
                // portion and none after the last.
                env.pause(delay_ms);
            }

            let end = portion_end(events, start);
            let portion = &events[start..end];

            done.calls += 1;
            done.requested += portion.len();
            done.accepted += env.send(portion) as usize;

            start = end;
        }
    }

    record_discrepancy(done);

    done
}

/// Where the portion starting at `start` ends — one event, or the whole of a character outside
/// the BMP.
///
/// A character outside the BMP is two code units and therefore
/// `2 * EVENTS_PER_UNIT` structures: the leading half down and up, then the trailing half down
/// and up. All four travel in one call, because FR-41's rule is about the *character* — half of
/// one delivered on its own is not a character delivered slowly, it is two undefined ones.
///
/// The test is on the **keydown** of the leading half and not merely on `wScan`, which matters
/// now that a unit has two edges: the event after the leading half's keydown is that same
/// half's keyup, carrying the same `wScan`, and a rule that looked only at the code unit would
/// group those two and leave the trailing half to go out on its own — precisely the split this
/// function exists to prevent.
fn portion_end(events: &[INPUT], start: usize) -> usize {
    let character = keyboard(&events[start]).is_some_and(|key| {
        key.dwFlags.0 & KEYEVENTF_UNICODE.0 != 0
            && key.dwFlags.0 & KEYEVENTF_KEYUP.0 == 0
            && is_high_surrogate(key.wScan)
    });

    if character {
        // Bounded by the slice: a packet that ended mid-character would be a bug elsewhere, and
        // the answer to it is a short last portion rather than a panic on this path.
        (start + 2 * EVENTS_PER_UNIT).min(events.len())
    } else {
        start + 1
    }
}

/// **FR-45.** Records a `SendInput` that did not take everything it was given.
fn record_discrepancy(done: Dispatched) {
    if done.is_complete() {
        return;
    }

    SEND_MISMATCHES.fetch_add(1, Ordering::Relaxed);

    // Saturating rather than wrapping: `accepted` cannot exceed `requested`, and a counter that
    // wrapped would be a worse answer than one that stopped.
    let lost = u32::try_from(done.requested.saturating_sub(done.accepted)).unwrap_or(u32::MAX);
    EVENTS_LOST.fetch_add(lost, Ordering::Relaxed);
}

/// The real `SendInput`.
fn send_input(events: &[INPUT]) -> u32 {
    // `cbSize` describes one element and is what tells the system it and this program agree on
    // the layout of `INPUT`. A width that does not fit an `i32` is impossible for this
    // structure; zero would make the call fail, which is the safe direction and is examined by
    // the caller (NFR-13, FR-45).
    let size = i32::try_from(size_of::<INPUT>()).unwrap_or(0);

    // SAFETY: `events` is a live, properly aligned slice of fully initialised `INPUT` values
    // owned by the caller for the whole call; the binding passes its pointer and length
    // together, so the system cannot read past the end. Every structure was built by
    // `key_event` or `unicode_event`, so the `ki` arm of the union is the live one and matches
    // the `INPUT_KEYBOARD` discriminant. The call inserts the events into the input stream and
    // returns; it writes nothing back through a pointer of ours.
    //
    // ⚠ It is called from the input thread's message loop and never from the hook callback —
    // FR-40. `SendInput` calls this process's own low-level hook back synchronously on this
    // thread, so from inside the callback it would be re-entrancy into a running function.
    unsafe { SendInput(events, size) }
}

/// The real pause of FR-44.
///
/// Blocking, and deliberately so: the user asked for a pause between events. It runs on the
/// input thread outside the hook callback, where NFR-04's ban on blocking primitives does not
/// reach, and it is the one thing in this program that can push the path past NFR-09's thirty
/// milliseconds — which is the trade FR-44 offers and the reason its default is zero.
fn sleep_ms(delay_ms: u32) {
    thread::sleep(Duration::from_millis(u64::from(delay_ms)));
}

// ---------------------------------------------------------------------------------------
// FR-40 — the whole order of operations
// ---------------------------------------------------------------------------------------

/// What one replacement did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Replaced {
    /// Characters taken off the screen — the `N` of FR-41, as [`OnScreen`] counted it.
    ///
    /// The same number in both modes and counted the same way: erased by `N` backspaces in the
    /// `backspace` mode, selected by `N` × `Shift+Left` and replaced by the insertion in the
    /// compatibility mode of FR-42.
    ///
    /// Which of the two counts of [`OnScreen`] it is depends on the press: the typing on the
    /// first, the previous injection from the second on — Э20, see [`OnScreen`].
    pub erased: usize,
    /// UTF-16 code units typed back.
    pub typed: usize,
    /// Modifiers taken down by step 3.
    pub released: Modifiers,
    /// What step 3 did.
    pub release: Dispatched,
    /// What step 4 did — the replacement itself, FR-41.
    pub replacement: Dispatched,
    // ⛔ **No `restored` and no `restore` — task T-40-3, finding С12, decision 127.2.** They were
    // what step 6 of FR-40 put back and how, and step 6 is not performed any more: the only thing
    // it could ever read was the asynchronous state after step 3's own release (see
    // `held_modifiers`). A field that is always `Modifiers::NONE` would be the silent change of
    // meaning the task forbids, so the two fields are gone and every reader was changed with them.
}

impl Replaced {
    /// **Whether the replacement itself reached the system whole** — the condition the position
    /// counter of FR-32 and the conversion session of FR-10 are allowed to move on, and since task
    /// T-40-2 (finding Н12) the condition step 5 of FR-40 switches the layout on.
    ///
    /// Step 4 and step 4 only. Step 3 is modifier hygiene: a `Shift` that failed to come down is a
    /// defect of its own, counted where every `SendInput` discrepancy is counted
    /// ([`send_mismatches`]), but it says nothing about whether the *text* on the screen changed —
    /// and what FR-32 counts is positions of the text.
    ///
    /// # Why anything less than whole counts as nothing
    ///
    /// Conservative on purpose: a packet the system took **in part** does not advance the cycle
    /// either. The counter is a promise about the whole run of characters — the press that brings
    /// it back to zero is the rollback of FR-33, "code unit for code unit" — and half a word
    /// replaced breaks that promise for the rest of the session. Of the two ways to be wrong, a
    /// counter that did not move is the one the user repairs by pressing the hotkey again,
    /// because the screen and the counter still agree; a counter that moved over a half-replaced
    /// word is the one nobody notices, because every later press then renders the wrong step of
    /// the cycle against text that is in some other state entirely.
    ///
    /// # This is not a second [`Dispatched::is_complete`]
    ///
    /// It is that very method, asked at the one place on the press path that never asked it —
    /// audit of 2026-08-24, task T-13-25. What `is_complete` compares is `accepted` against
    /// `requested`, both counted in `INPUT` **structures handed to `SendInput`** and summed over
    /// every call the dispatch made. So it is invariant under the split of FR-44: the pause mode
    /// changes how many calls carry the packet, never how many structures the packet has, and
    /// [`portion_end`] keeps the four structures of a character outside the BMP inside one
    /// portion, so a surrogate pair is not split across calls in the first place. A pair
    /// travelling as its own portion is complete exactly when both halves were accepted. Nothing
    /// about the mode of FR-44 or about surrogate pairs can make this answer `false`; only
    /// `SendInput` taking fewer events than it was given can — which is the blocked injection
    /// this asks about.
    ///
    /// The FR-45 side is untouched: the discrepancy is still counted in [`send_mismatches`] by
    /// [`record_discrepancy`], and the packet is still not re-sent, for the reason
    /// [`dispatch_in`] gives — re-sending the tail would duplicate whatever did get through.
    ///
    /// # ⭐ And a replacement of nothing did not reach the screen — task T-40-1, finding Н18
    ///
    /// `is_complete` over **zero** events answers `true` — zero of zero is a complete dispatch, and
    /// that method is left saying so. But zero events changed nothing on the screen, and the press
    /// that sent them must move nothing either: the ring of mute keys (the function row, the
    /// volume keys) used to render into an empty packet and advance the counter of FR-32 all the
    /// same. So the one predicate the press decides by asks the other half too. [`replace_in_with`]
    /// refuses an empty packet before it is sent, which makes this `requested > 0` a second line
    /// rather than the only one: a `Replaced` built anywhere else still cannot pass an empty
    /// replacement off as a real one.
    pub const fn reached_the_screen(self) -> bool {
        self.replacement.requested > 0 && self.replacement.is_complete()
    }
}

/// **FR-40, steps 3 to 5, for one recognised hotkey press.**
///
/// `strokes` is what the typing buffer recorded, `target` is the layout to convert into — task
/// T-05-2 chooses it and it arrives here as a parameter — and `delay_ms` is FR-44.
///
/// The order is §4.5 of SPEC and is not free:
///
/// 1. step 1, suppressing the hotkey, is `hook::classify`'s and has already happened;
/// 2. step 2, the handoff, is `hook::post_hotkey`'s and is why this runs on the message loop;
/// 3. **step 3** — the modifiers the user is holding are released explicitly, so that the
///    injected `Backspace` is a `Backspace` and not `Shift+Backspace`;
/// 4. **step 4** — the replacement, one packet, FR-41;
/// 5. **step 5** — the layout switch of §4.6, marked below and performed by module `switch` —
///    and only when step 4 was taken whole ([`Replaced::reached_the_screen`], task T-40-2);
/// 6. **step 6 is not performed** — task T-40-3, finding С12, decision 127.2: the modifiers the
///    user holds physically cannot be read back after step 3 has released them (see
///    [`held_modifiers`]); FR-40 makes the step «по возможности» and §10 of SPEC names the
///    limitation.
///
/// **FR-43** is the position of step 5 and nothing else: the replacement is formed and sent
/// before the layout is switched. `KEYEVENTF_UNICODE` carries a character literally, past the
/// layout, so there is no race to lose — but the order still has to be built in, because a
/// switch performed first would be visible to every event that followed it.
///
/// # Errors
///
/// Sizing failures, and neither is reachable from [`on_hotkey`], which sizes both working
/// buffers from the lengths it is about to use. They are returned rather than asserted because
/// a panic on this path would take a resident program down over a full buffer.
///
/// And [`InjectError::EmptyPacket`], which **is** reachable and is the ordinary answer to a press
/// over a run of mute keys — task T-40-1: nothing is sent and steps 3 to 5 do not run.
pub fn replace(
    strokes: &[Keystroke],
    target: &LayoutMap,
    delay_ms: u32,
) -> Result<Replaced, InjectError> {
    // FR-40 step 5, task T-05-1: the layout the strokes were just rendered into is the layout the
    // foreground window is switched to. `LayoutMap::layout` is that identifier and it is already
    // here, so step 5 invents no target of its own — choosing one is task T-05-2's.
    replace_in(
        &mut System::for_target(target.layout()),
        strokes,
        target,
        delay_ms,
    )
}

/// **FR-40, steps 3 to 5**, against any [`Environment`] — see [`replace`] for the requirement.
///
/// The `backspace` mode of FR-41, unconditionally — the seam task T-04-1 left, kept as it was
/// when `backspace` was the default of section 7 (the default is `auto` since FR-42а).
/// [`replace_in_with`] is the same steps for a mode chosen by the caller.
pub fn replace_in(
    env: &mut impl Environment,
    strokes: &[Keystroke],
    target: &LayoutMap,
    delay_ms: u32,
) -> Result<Replaced, InjectError> {
    replace_in_with(
        env,
        strokes,
        target,
        OnScreen::as_typed(strokes),
        delay_ms,
        ReplacementMethod::Backspace,
    )
}

/// **FR-40, steps 3 to 5, in the mode of `method`** — the `backspace` packet of FR-41 or the
/// compatibility packet of FR-42.
///
/// This is the function the order lives in, and the order is the requirement, so this is what
/// `tests\inject.rs` drives. `method` is a parameter and not a read of [`replacement_method`]
/// for the same reason `delay_ms` is: what the configuration says is decided once, by
/// [`on_hotkey`], and everything below it is then a pure function of its arguments.
///
/// Only the *packet* depends on the mode. Steps 3 and 5, the pause of FR-44, the single call
/// of FR-41, the signature of FR-03 and the position FR-43 fixes are shared, and deliberately:
/// a mode that could quietly do without the modifier hygiene of FR-40 would be a second
/// requirement, not a second packet.
///
/// ⭐ **Stage Э96, decision 160.8 — the one exception to «only the packet».** The keystroke path
/// of the classic console ([`Delivery::Keystrokes`]) also moves step 5 in front of step 4, and
/// switches back when its packet did not reach the screen — see [`run_steps_ahead`]. `back` is the
/// layout the window was in before this press (what [`take_press`] reads off the screen's state);
/// the other paths never use it.
pub fn replace_with(
    strokes: &[Keystroke],
    target: &LayoutMap,
    on_screen: OnScreen,
    back: LayoutId,
    delay_ms: u32,
    delivery: Delivery,
) -> Result<Replaced, InjectError> {
    // FR-40 step 5, task T-05-1 — see `replace` for why the target is `target.layout()`.
    let mut system = if matches!(delivery, Delivery::Keystrokes) {
        System::for_classic_console(target.layout(), back)
    } else {
        System::for_target(target.layout())
    };

    replace_in_with(&mut system, strokes, target, on_screen, delay_ms, delivery)
}

/// [`replace_with`] against any [`Environment`].
///
/// `delivery` takes a configured [`ReplacementMethod`] as well — the seam task T-04-1 left took a
/// method, and an explicit method is the packet it names ([`Delivery::from`]).
pub fn replace_in_with(
    env: &mut impl Environment,
    strokes: &[Keystroke],
    target: &LayoutMap,
    on_screen: OnScreen,
    delay_ms: u32,
    delivery: impl Into<Delivery>,
) -> Result<Replaced, InjectError> {
    let delivery = delivery.into();

    // FR-41 and FR-42: `N` is the number of characters **on the screen**, not the number of
    // keys pressed, and it is the same `N` in both modes.
    //
    // ⚠ It is handed in rather than computed here — audit of 2026-08-31 finding №5, stage Э20.
    // What stands on the screen is what the user typed only on the *first* press of a run; from
    // the second on it is what the previous step of the cycle injected, and with a dead key in
    // the run the two lengths differ (`café` typed is four characters, the same strokes
    // injected are five). Only the caller knows which press this is — `take_press` reads the
    // position counter of FR-32 — so only the caller can pick the right count. See [`OnScreen`].
    let erased = on_screen.count();

    // The whole packet is *formed* here, before anything is sent and long before step 5 —
    // FR-43. `max_units` is the length no conversion of these strokes can exceed, so the
    // error below is unreachable and is handled anyway (NFR-13).
    let mut text = vec![0u16; convert::max_units(strokes.len())];
    let typed = convert::convert_strokes(strokes, target, &mut text)
        .map_err(|ConvertError::BufferTooSmall { needed }| InjectError::TextTooLong { needed })?;

    // ⭐ **Stage Э96 — the keystroke path builds its own packet, out of the strokes and not out of
    // the text.** The text is still formed — FR-43's «formed before» holds, and the counts the
    // channel publishes below are of it — but what goes out is the user's keys again
    // ([`build_keystrokes`]). A run whose keys would not repeat the conversion is refused here,
    // before steps 3 to 5 and before a single event: nothing is erased, no layout is switched
    // (decision 160.8, «запасной — не стирать»). An empty run stays what it always was, the idle
    // press of finding Н18, and is told apart from the refusal first.
    let (mut events, built) = match delivery {
        Delivery::Keystrokes if erased == 0 && typed == 0 => (Vec::new(), Ok(0)),
        Delivery::Keystrokes if !keys_repeat_the_conversion(strokes, target) => {
            (Vec::new(), Err(InjectError::KeysCannotRepeat))
        }
        Delivery::Keystrokes => {
            let mut events = vec![INPUT::default(); keystroke_events(erased, strokes)];
            let built = build_keystrokes(erased, strokes, &mut events);
            (events, built)
        }
        Delivery::Backspace | Delivery::Selection => {
            let method = if matches!(delivery, Delivery::Backspace) {
                ReplacementMethod::Backspace
            } else {
                ReplacementMethod::Selection
            };
            let mut events = vec![INPUT::default(); packet_events(method, erased, typed)];
            let built = build_packet(method, erased, &text[..typed], &mut events);
            (events, built)
        }
    };

    // ⭐ **Task T-40-1, finding Н18 — an empty packet is refused here, before steps 3 to 5.**
    //
    // Nothing to erase and nothing to type: the run is a ring of mute keys, which FR-10 records
    // like any other key. Sent, the packet would be zero events that `SendInput` "took whole", and
    // the idle press would release the user's modifiers, switch the layout of the foreground
    // window (step 5) and let `note_press` move the counter of FR-32 over a screen that did not
    // change. Refused, `on_hotkey` answers `None` and `app::press_outcome` sounds the idle
    // click — the path of every other press that had nothing to do. The early return of
    // [`dispatch_in`] on an empty array stays exactly as it is: that is a dispatch declining to
    // make a call, and this is the press declining to happen.
    let built = match built {
        Ok(0) => Err(InjectError::EmptyPacket),
        other => other,
    };

    // ⚠ **The shape of the packet, published before it is sent — task T-10-6, SEC-04a.**
    //
    // Everything the channel could say about a press said that the press had gone perfectly while
    // the screen showed six copies of one letter, so there was no way to tell a product that built
    // a collapsed packet from a product that built a correct one somebody else collapsed. Three
    // counts settle it, and they are counts: how many characters are being taken off the screen,
    // how many code units are going back, and how many of the character events of the built array
    // are *different*. See [`crate::control::note_replacement`] — SEC-01 and SEC-07 are argued
    // there — and [`distinct_units`] for why it is read off the array rather than off the text.
    #[cfg(feature = "testing")]
    if let Ok(len) = built {
        crate::control::note_replacement(erased, typed, distinct_units(&events[..len]));

        // ⭐ **Whether this packet returns what it took, and the direction it applied — task
        // T-10-17, SEC-04a.**
        //
        // The three counts above say how *much* moved and never *whether* anything did: the
        // live protocol of defect E reads `6/6/6` on all thirty-four presses of a text the
        // person watched not change. These two are the missing half. The first is the
        // comparison of the product's own two sides — see [`insertion_differs`]. The second is
        // the direction FR-26 decided, `origin` being the layout the strokes carry (the very
        // value `take_press` chose the target from) and `target.layout()` the layout they were
        // rendered into: equal halves are the rollback of FR-32, a legitimate identity, and
        // until now it was unreadable from outside.
        //
        // Two layout handles and one bit, of exactly the kind `active_layout` already
        // publishes — SEC-01 and SEC-07 are argued at `control::note_replacement_outcome`.
        crate::control::note_replacement_outcome(
            insertion_differs(strokes, &text[..typed]),
            strokes.first().map_or(0, |stroke| stroke.layout().raw()),
            target.layout().raw(),
        );
    }

    // The working buffers hold the user's text in plain form, and in the shipped build they are
    // the only heap blocks outside module `buffer` that ever do. Whatever happens below, they are
    // zeroed before this frame releases them — SEC-01, SEC-02.
    //
    // ⚠ «In the shipped build» is the qualifier finding Н96 found missing: under the `testing`
    // feature the publication above used to count distinct units over a third, unzeroed copy.
    // Since task T-40-4 `distinct_units` makes no copy, and `tests\inject_zeroing.rs` watches every
    // block this path frees for the text — so the sentence holds in a `testing` build as well, and
    // it is that check, not this comment, that says so.
    let outcome = built.map(|len| {
        if matches!(delivery, Delivery::Keystrokes) {
            run_steps_ahead(env, &events[..len], erased, typed, delay_ms)
        } else {
            run_steps(env, &events[..len], erased, typed, delay_ms)
        }
    });

    // The write is `crate::buffer::zero_slice` and not `fill`, because a `fill` immediately in
    // front of the deallocation these two vectors are about to have is a dead store the
    // optimiser may delete — the very reason module `buffer` zeroes the ring volatile. Task
    // T-13-15, audit of 2026-08-24.
    crate::buffer::zero_slice(&mut text);
    crate::buffer::zero_slice(&mut events);

    outcome
}

/// Steps 3 to 5 of FR-40 around a packet that is already built — step 6 is not performed (task
/// T-40-3, see the end of the body).
fn run_steps(
    env: &mut impl Environment,
    packet: &[INPUT],
    erased: usize,
    typed: usize,
    delay_ms: u32,
) -> Replaced {
    // ---- step 3 -----------------------------------------------------------------------
    let (released, release) = release_held(env, delay_ms);

    // ---- step 4, FR-41 ----------------------------------------------------------------
    let replacement = dispatch_in(env, packet, delay_ms);

    // ---- step 5 — the connection point of task T-05-1 ----------------------------------
    //
    // FR-40 step 5 switches the layout of the foreground window and §4.6 (FR-50 to FR-52) says
    // how. The how is module `switch` (task T-05-1), reached through `Environment::switch_layout`;
    // what this line owns is the *position*, which is the whole reason it is a trait member.
    //
    // ⚠ **FR-43: the call goes here and nowhere earlier.** Everything above has already been
    // sent, and the switch is the last thing a press does. A switch moved in front of step 4
    // would be the race FR-43 was written to rule out.
    //
    // ⭐ **Task T-40-2, finding Н12, decision 127.3 — and only when the replacement reached the
    // screen.** A blocked injection (`BlockInput`, a window of higher integrity, another
    // program's injection in progress) is taken as zero events, or as a packet cut short: the text
    // on the screen is still what the user typed, and switching the layout under it would make
    // the next word come out in the other language. The question is the one the counter of FR-32
    // is moved by — [`Replaced::reached_the_screen`], asked of the replacement as it stands after
    // step 4, and asked **there** rather than restated here, so that the press and the switch can
    // never disagree about what "reached the screen" means.
    let reached = Replaced {
        replacement,
        ..Replaced::default()
    }
    .reached_the_screen();

    if reached {
        env.switch_layout();
    }

    // ---- step 6 — not performed: task T-40-3, finding С12, decision 127.2 --------------
    //
    // Step 6 of FR-40 asks for the modifiers the user holds «физически» and presses them again.
    // There was a step 6 here, and it could never do that: the only question available is the
    // asynchronous key state (`held_modifiers`, and `hook::physical_modifiers` is the same state),
    // which follows the program's own events too — step 3 has just sent the release, so the user's
    // `Shift` reads as up however firmly it is held. The branch that restored the user's keys ran
    // never; the one that could run pressed again a key nobody held — a `Shift` of our own left
    // down by a packet. Decision 127.2 made the step «по возможности» in FR-40 and named the
    // limitation in §10; a user who held a modifier over the hotkey presses it again after the
    // replacement. What is kept is the hygiene of our own events: every packet this module builds
    // is balanced — the compatibility packet's own `Shift` goes down and up inside it — so nothing
    // of ours is left down to need a step 6 in the first place.
    Replaced {
        erased,
        typed,
        released,
        release,
        replacement,
    }
}

/// **FR-40 step 3** — the modifiers the user is holding, asked once and released in one call.
/// Shared by both orders of the steps, so that the hygiene of step 3 cannot differ between them.
fn release_held(env: &mut impl Environment, delay_ms: u32) -> (Modifiers, Dispatched) {
    let mut modifier_events = [INPUT::default(); MODIFIER_COUNT];

    let released = env.held();
    let release = match build_release(released, &mut modifier_events) {
        // The array is `MODIFIER_COUNT` long and `released.count()` can never exceed it, so
        // the error is unreachable; it is answered by sending nothing rather than by a panic.
        Err(_) => Dispatched::default(),
        Ok(len) => dispatch_in(env, &modifier_events[..len], delay_ms),
    };

    (released, release)
}

/// ⭐ **The steps of the keystroke path — stage Э96, decision 160.8: step 3, step 5, step 4.**
///
/// The classic console (`ConsoleWindowClass`) does not take `KEYEVENTF_UNICODE`, so its
/// replacement is the user's keys again ([`build_keystrokes`]), and keys mean what the layout of
/// the window makes of them — the layout has to be the target **before** they go out. So, and only
/// on this path:
///
/// 1. **step 3** — the modifiers the user holds are released, exactly as on every path;
/// 2. **step 5, ahead** — the switch of FR-50, without a verdict ([`switch_classic_in`]);
/// 3. **no pause** — the premise П2 of the stage measured the switch taken before the very next
///    keys, 20 times of 20 in Windows PowerShell 5.1 and in `cmd`, there and back, at a pause of
///    zero; the waits 10/20/50/100 ms were never needed, and NFR-09's thirty milliseconds stand;
/// 4. **step 4** — the packet, in one call (FR-41): the backspaces first — `Backspace` does not
///    depend on the layout — and the keys after them;
/// 5. **back, when the packet did not reach the screen** ([`Replaced::reached_the_screen`]) — the
///    switch was already made, so the text as typed would stay under a layout the user did not
///    choose and the next word would come out in the other language: finding Н12, decision 127.3,
///    kept here by switching back rather than by not switching.
///
/// FR-43 — «замена выполняется до переключения раскладки» — is suspended here and here only; its
/// reason, that `KEYEVENTF_UNICODE` cannot race the layout, does not apply to a packet that has no
/// such event. If the switch did not take in time, the keys put back the word as it was typed:
/// the text does not change, and nothing is lost.
fn run_steps_ahead(
    env: &mut impl Environment,
    packet: &[INPUT],
    erased: usize,
    typed: usize,
    delay_ms: u32,
) -> Replaced {
    // ---- step 3 -----------------------------------------------------------------------
    let (released, release) = release_held(env, delay_ms);

    // ---- step 5, ahead of the packet — decision 160.8 ---------------------------------
    env.switch_layout();

    // ---- step 4, FR-41 ----------------------------------------------------------------
    let replacement = dispatch_in(env, packet, delay_ms);

    // ---- back, if the packet did not reach the screen — Н12, decision 127.3 -----------
    let replaced = Replaced {
        erased,
        typed,
        released,
        release,
        replacement,
    };

    if !replaced.reached_the_screen() {
        env.switch_back();
    }

    replaced
}

// ---------------------------------------------------------------------------------------
// The far end of the FR-02 handoff — what `app::window_proc` calls
// ---------------------------------------------------------------------------------------

/// **FR-40 for the hotkey press the input thread has just taken off its queue.**
///
/// This is the whole of what `app::window_proc` does with [`crate::hook::WM_APP_HOTKEY`]: the
/// callback recognised the press, suppressed it (step 1) and posted the message (step 2), and
/// this runs steps 3 to 5 on the input thread, outside the callback, where section 6.1 puts
/// `SendInput`.
///
/// `None` when there is nothing to do: no typing buffer on this thread — which is every thread
/// but the input one, and is also what makes an unsolicited `WM_APP_HOTKEY` from another
/// process harmless (SEC-05) — an empty buffer, or a target layout that cannot be determined
/// yet. Nothing is sent in any of those cases.
///
/// # The strokes leave the buffer first
///
/// [`crate::buffer::with`] is a `RefCell` borrow and `SendInput` re-enters this thread's own
/// hook callback synchronously, which reaches [`crate::buffer::record`] for any stroke the user
/// makes while the packet is going out. So the copy is taken, the borrow is dropped, and only
/// then is anything sent. It is also the reason the strokes are copied at all rather than
/// converted in place.
///
/// # Boundaries
///
/// # What the configuration decides, and where it is read
///
/// Both of the published values of section 7 that reach this module are read **here**, once per
/// press, out of the atomics the UI thread stores into: `[replacement] method` through
/// [`replacement_method`] and `[replacement] inter_event_delay_ms` through
/// [`inter_event_delay_ms`]. Nothing below this line reads a configuration and nothing anywhere
/// on this path opens a file — NFR-09 gives the whole path from the hotkey to the last event of
/// the replacement thirty milliseconds, and a file read does not fit into that budget.
///
/// # Boundaries
///
/// * **which** layout the strokes are rendered into is section 4.4 and module `layouts` (task
///   T-05-2), reached from [`take_press`] below; this module asks for it and does not decide it;
/// * the layout switch of FR-40 step 5 is module **`switch`**'s (task T-05-1) and is reached
///   from the marked point inside [`replace_in_with`]; its target is `target.layout()`, the
///   layout the strokes were rendered into.
pub fn on_hotkey() -> Option<Replaced> {
    // Stage Э96: the answer of the previous press is not this press's answer.
    CLASSIC_CONSOLE_REFUSED.set(false);

    let Press {
        mut strokes,
        target,
        on_screen,
        back,
        cycle_len,
    } = take_press()?;

    // **FR-42а.** The configured method is read once per press, and `auto` is resolved against
    // the foreground window here — the input thread's message loop, after the handoff, never
    // the hook callback — so that everything below this line is a pure function of its
    // arguments, exactly as `delay_ms` already is. `delivery` is one of the three real packets by
    // construction; `Auto` does not travel further.
    let delivery = effective_method(replacement_method());

    // The path this very replacement runs, published for the bench of §11.5: without it the
    // choice FR-42а makes is invisible from outside the process, and a rule nobody can observe
    // is a rule nobody can check. A count-free word, same discipline as `note_replacement`.
    #[cfg(feature = "testing")]
    crate::control::note_replacement_method(delivery);

    let result = replace_with(
        &strokes,
        &target,
        on_screen,
        back,
        inter_event_delay_ms(),
        delivery,
    );

    // ⭐ **Stage Э96 — the refusal of the classic console is a fact worth a line and a tone.** Its
    // keys would not repeat the conversion, so nothing was sent at all; the journal gets the name
    // and nothing beside it (SEC-07), and `app` answers the press with the tone of a refusal
    // through [`take_classic_console_refusal`] — decision 160.8, «запасной — не стирать».
    if result == Err(InjectError::KeysCannotRepeat) {
        CLASSIC_CONSOLE_REFUSED.set(true);
        crate::diag::record(
            crate::diag::Operation::from_name(CLASSIC_CONSOLE_REPLACEMENT_REFUSED),
            crate::diag::OsCode::NONE,
        );
    }

    let outcome = result.ok();

    // The copy holds the user's text; it is zeroed before it is released — SEC-01, SEC-02, with
    // the volatile write of `crate::buffer::zero_slice` so that the store in front of the `drop`
    // below cannot be optimised away (task T-13-15).
    crate::buffer::zero_slice(&mut strokes);
    drop(strokes);

    note_press(outcome.as_ref(), cycle_len);

    outcome
}

/// The name the journal records a refusal of the classic console under — a row of the vocabulary
/// of module `diag`, stage Э96. A fact and no value (SEC-01, SEC-07).
const CLASSIC_CONSOLE_REPLACEMENT_REFUSED: &str = "classic console replacement refused";

thread_local! {
    /// ⭐ **Whether the last press on this thread was refused by the keystroke path** — stage Э96.
    ///
    /// Set by [`on_hotkey`] when the classic console's run could not be repeated by its keys
    /// ([`InjectError::KeysCannotRepeat`]), cleared at the start of every press and by the one
    /// reader, [`take_classic_console_refusal`]. A cell of the input thread rather than a value of
    /// [`on_hotkey`], because that function's answer — `None` for a press that sent nothing — is
    /// read in more places than this one, and every one of them is right to read a refusal as
    /// «nothing was sent»; only the tone of FR-100 tells the two apart.
    static CLASSIC_CONSOLE_REFUSED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// **Whether the press [`on_hotkey`] just answered was refused by the classic console**, and the
/// answer taken — stage Э96. Read by `app` on the input thread right after the press, to answer it
/// with the tone of a refusal instead of the idle click (FR-100, decision 160.8).
pub fn take_classic_console_refusal() -> bool {
    CLASSIC_CONSOLE_REFUSED.replace(false)
}

/// **What one press leaves behind in the typing buffer** — the last row of FR-10 and the position
/// counter of FR-32.
///
/// Split out of [`on_hotkey`] and public for one reason: this is the half of the press that
/// *decides*, and the decision is made from a [`Replaced`] that the [`Environment`] seam can
/// produce. `on_hotkey` itself runs against the real machine, and a check that drove it would
/// have to let `SendInput` reach whatever window is in the foreground — which is the one thing
/// `tests\inject.rs` refuses to do. With the decision here, a bench whose `send` accepts nothing
/// (or only part of the packet) can be pointed at the very code the input thread runs.
///
/// It is called from the input thread's message loop, after the handoff of FR-02 and never from
/// the hook callback: one thread-local borrow, one flag, one remainder — no allocation, no
/// blocking primitive, no I/O (NFR-01 to NFR-05).
pub fn note_press(outcome: Option<&Replaced>, cycle_len: usize) {
    // ⚠ **Nothing moves unless the replacement really reached the screen** — audit of
    // 2026-08-24, task T-13-25. `replace_with` answers `Ok(Replaced)` even when `SendInput`
    // accepted nothing at all (a blocked injection: UIPI towards an elevated window from a
    // build without `uiAccess`, `BlockInput`), and a press that changed nothing on the screen
    // must not move a counter that describes the screen. See [`Replaced::reached_the_screen`]
    // for why a *partial* packet counts as nothing here too, and for why the FR-45 bookkeeping
    // is deliberately left exactly as it was.
    if !outcome.is_some_and(|replaced| replaced.reached_the_screen()) {
        return;
    }

    crate::buffer::with(|recorder| {
        // The last row of the FR-10 table, as module `buffer` describes it: the session is
        // marked converted and the buffer is deliberately **not** flushed, so that FR-32 can
        // render the same scan codes into the next layout on the next press. It is the next
        // ordinary key that ends the session, and `Recorder::record` flushes then.
        recorder.note_conversion();

        // **FR-32, the other half.** The strokes stay as they were; what moves is the
        // position counter, and it moves once per replacement that actually happened. The
        // next press therefore renders *these same* strokes one step further along the
        // cycle, and the press that brings the counter back to zero renders them into the
        // layout they were typed under — which is the original text, code unit for code
        // unit.
        recorder.advance_cycle(cycle_len);
    });
}

/// What one press of the hotkey needs, taken under a single borrow of the buffer.
///
/// The strokes are a **copy**: [`crate::buffer::with`] is a `RefCell` borrow and `SendInput`
/// re-enters this thread's own hook callback, so nothing may be sent while the borrow is alive.
/// The buffer itself is left exactly as it was — FR-32.
struct Press {
    /// The strokes as they were recorded, oldest first.
    strokes: Vec<Keystroke>,
    /// The layout they are to be rendered into on this press — section 4.4.
    target: LayoutMap,
    /// How many characters of this run stand on the screen — the `N` of FR-41, chosen by
    /// [`take_press`] from the position counter of FR-32.
    on_screen: OnScreen,
    /// ⭐ **The layout the text on the screen is in** — stage Э96: the layout of the previous
    /// press's injection from the second press on ([`showing`]), the layout the run was typed in on
    /// the first. The keystroke path switches back to it when its packet did not reach the screen
    /// (Н12, decision 127.3); the other paths never read it.
    back: LayoutId,
    /// How many layouts the cycle walks, so that the counter can be advanced by the same number
    /// the target was chosen with.
    cycle_len: usize,
}

/// Copies the strokes out and chooses the layout this press renders them into — **section 4.4**.
///
/// `None` when there is nothing to do, and nothing is sent in any of those cases:
///
/// * no typing buffer on this thread — which is every thread but the input one, and is what
///   makes an unsolicited `WM_APP_HOTKEY` from another process harmless (SEC-05);
/// * an empty buffer;
/// * no mapping cache yet, which is the few milliseconds of start-up before the sweep of FR-20
///   has finished; a stroke recorded then carries no characters anyway;
/// * **a refusal from module `layouts`** — an IME named as a participant (FR-35), fewer than two
///   layouts to switch between, or text typed under a layout that is not one of the participants
///   (FR-30, "остальные раскладки игнорируются"). Every one of them is counted there
///   ([`crate::layouts::selection_failures`]) and none of them switches anything.
///
/// # Where the target comes from — NFR-09, FR-25
///
/// From the **live cache** of FR-20, through [`crate::buffer::Recorder::cache`], and never from
/// a sweep of its own: FR-20 costs thousands of `ToUnicodeEx` calls and about five milliseconds,
/// against the thirty NFR-09 gives the whole path. The map is cloned out of the cache — a copy
/// of an array that is already built, not a rebuild of it — because the borrow has to be dropped
/// before anything is sent.
///
/// The hardwired RU/EN table of FR-25 is still the emergency reserve and is still reached, by
/// exactly the route FR-25 describes: `app::cache_or_fallback` publishes it into the buffer when
/// the sweep fails and there is no working cache to keep — since task T-39-4 (finding Н7) a
/// failed *rebuild* leaves the cache it had — filed since task T-39-6 (finding Н10) under the
/// layouts of the session in its two languages ([`crate::convert::fallback_cache_for`]), and the
/// choice below is then made against that cache like any other.
fn take_press() -> Option<Press> {
    crate::buffer::with(|recorder| {
        let mut strokes = vec![Keystroke::default(); recorder.len()];

        // The slice was sized from `recorder.len()` under this very borrow, so
        // `ReadError::OutputTooSmall` cannot happen; it is answered with zero rather than with
        // an `expect`, because a resident program must not end on a full buffer.
        let live = recorder.keystrokes(&mut strokes).unwrap_or(0);
        strokes.truncate(live);

        if strokes.is_empty() {
            return None;
        }

        let cache = recorder.cache()?;

        // The participating layouts of FR-35, in system order — the list section 4.4 chooses
        // from. A fixed array on the stack: no allocation on the hotkey path.
        let mut available = [LayoutId::default(); layouts::MAX_CYCLE];
        let count = cache.layouts(&mut available);

        // **Task Т-22-5.** Whether the array above holds the session or only the front of it. A
        // pair that resolves against nothing means two opposite things in the two cases, and only
        // the caller knows which list it handed over.
        let session = layouts::Session::of(count, cache.len());

        let cycle = layouts::cycle_for(layouts::published(), &available[..count], session).ok()?;

        // **FR-26: the direction comes from the HKL recorded with the stroke**, and never from
        // the layout that happens to be active now. The difference is the whole of FR-33: step 5
        // of FR-40 switches the foreground window to the layout the *previous* press rendered
        // into, so by the second press "the layout in use" is the target of the first one. A
        // cycle counted from there would answer the same layout again and the rollback would
        // quietly stop working — and only from the second press onwards, which is exactly the
        // kind of defect FR-32 warns about. The strokes are unchanged (FR-32), so the layout the
        // run was typed in is still recorded in them.
        let origin = strokes
            .first()
            .map_or_else(|| recorder.active_layout(), |stroke| stroke.layout());

        // The step this press moves to. The counter is *read* here and advanced only once the
        // replacement has really been sent, so a press that ends in a refusal below leaves the
        // buffer and the counter exactly as it found them.
        let position = recorder.cycle_position();
        let step = position + 1;
        let target = cycle.target(origin, step).ok()?;

        // **What is on the screen right now** — the `N` of FR-41, and the one thing only this
        // function can answer, because only here is the position counter of FR-32 readable
        // (audit of 2026-08-31, finding №5, stage Э20).
        //
        // At position zero the run has never been replaced and the screen holds what the user
        // typed, dead keys composed by the OS. From position one on it holds what the previous
        // press injected — the same strokes rendered into `target(origin, position)`, with
        // nothing composed, because `KEYEVENTF_UNICODE` carries a character literally.
        //
        // A previous target the cache can no longer answer for (a layout removed from the
        // system between two presses) falls back to the typed count rather than refusing the
        // press: that is the behaviour of every build before this one, and a hotkey that goes
        // quiet is worse than a count that is one out on a run nobody is in the middle of.
        let previous = showing(&cycle, origin, position, recorder.in_conversion());
        let on_screen = previous
            .and_then(|previous| cache.get(previous))
            .map_or_else(
                || OnScreen::as_typed(&strokes),
                |previous| OnScreen::as_injected(&strokes, previous),
            );

        Some(Press {
            strokes,
            target: cache.get(target)?.clone(),
            on_screen,
            // Stage Э96: the layout the text on the screen is in — see `Press::back`.
            back: previous.unwrap_or(origin),
            cycle_len: cycle.len(),
        })
    })
    .flatten()
}

// ---------------------------------------------------------------------------------------
// Tests of what needs no Win32 at all
// ---------------------------------------------------------------------------------------

/// The private helpers `tests\inject.rs` cannot reach, and the one test that must not run beside
/// it.
///
/// Almost everything a requirement is stated about is public and is tested in `tests\inject.rs`;
/// what is here is implementation details whose behaviour the public functions rest on — plus
/// [`the_replacement_section_of_the_configuration_reaches_this_module`], which is here for a
/// different reason, given at the test itself.
///
/// [`the_replacement_section_of_the_configuration_reaches_this_module`]:
///     tests::the_replacement_section_of_the_configuration_reaches_this_module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_surrogate_is_recognised_and_a_trailing_one_is_not() {
        assert!(is_high_surrogate(0xD800));
        assert!(is_high_surrogate(0xD83D));
        assert!(is_high_surrogate(0xDBFF));

        // The trailing half, the character before the range and an ordinary letter.
        assert!(!is_high_surrogate(0xDC00));
        assert!(!is_high_surrogate(0xD7FF));
        assert!(!is_high_surrogate(u16::from(b'a')));
    }

    /// The `N` of FR-41 counts characters, and a surrogate pair is one of them.
    #[test]
    fn a_surrogate_pair_is_one_character_and_two_code_units() {
        // `U+1F600`, which UTF-16 spells `D83D DE00`.
        assert_eq!(chars_in(&[0xD83D, 0xDE00]), 1);
        assert_eq!(chars_in(&[u16::from(b'a'), 0xD83D, 0xDE00]), 2);
        assert_eq!(chars_in(&[]), 0);
    }

    /// A `GetAsyncKeyState` answer is read on its high bit and on nothing else.
    #[test]
    fn only_the_high_bit_of_the_key_state_means_held() {
        assert!(is_held(-32768));
        assert!(is_held(-32767));
        assert!(!is_held(0));

        // The low bit — "was pressed since the last call" — is not the state and must not be
        // read as one: it would make the answer depend on who called before us.
        assert!(!is_held(1));
    }

    /// The stored code and the mode it means are one mapping, and reading it back can never
    /// fail — FR-42, section 7.
    #[test]
    fn a_code_that_names_no_mode_reads_as_the_default_of_section_seven() {
        assert_eq!(method_code(ReplacementMethod::Backspace), BACKSPACE_CODE);
        assert_eq!(method_code(ReplacementMethod::Selection), SELECTION_CODE);
        assert_eq!(method_code(ReplacementMethod::Auto), AUTO_CODE);
        assert_eq!(
            method_from_code(BACKSPACE_CODE),
            ReplacementMethod::Backspace
        );
        assert_eq!(
            method_from_code(SELECTION_CODE),
            ReplacementMethod::Selection
        );
        assert_eq!(method_from_code(AUTO_CODE), ReplacementMethod::Auto);

        // Nothing but `set_replacement_method` writes the atomic and it writes only those
        // three, so this is unreachable — and the replacement path must have a mode rather
        // than a question even so, which is what makes the fallback the *documented* default
        // of section 7: `auto` since FR-42а.
        assert_eq!(method_from_code(200), ReplacementMethod::Auto);
        assert_eq!(method_from_code(u8::MAX), ReplacementMethod::Auto);
    }

    /// **Section 7 reaches this module.** `[replacement] method` and
    /// `[replacement] inter_event_delay_ms` travel from a real `config.toml` into the two
    /// atomics the replacement path reads, through exactly the two calls
    /// `app::publish_configuration_to_input_thread` makes — FR-42, FR-44.
    ///
    /// # Why this test is here and not in `tests\inject.rs`
    ///
    /// Both values are process-wide statics, and the tests of one binary run in parallel.
    /// `tests\inject.rs` already owns a test that publishes a pause and puts it back
    /// (`the_pause_of_fr44_defaults_to_zero_and_can_be_published`, task T-04-1), and a second
    /// test writing the same atomic in the same process would be asserting on that test's
    /// timing rather than on a requirement. The unit tests of the library are a **different test
    /// binary** and therefore a different process, so here the two atomics are this test's
    /// alone. It still puts back what it found, for the same reason.
    #[test]
    fn the_replacement_section_of_the_configuration_reaches_this_module() {
        // The default of section 7 is one word in three places: the schema says `auto`
        // (FR-42а), `settings` derives it, and this module starts at it without being told.
        assert_eq!(
            crate::settings::Replacement::default().method,
            ReplacementMethod::Auto
        );
        assert_eq!(replacement_method(), ReplacementMethod::Auto);
        assert_eq!(inter_event_delay_ms(), 0);

        let directory =
            std::env::temp_dir().join(format!("lang_switcher_t04_2_{}", std::process::id()));
        let path = crate::settings::config_path_in(&directory);
        let parent = path
            .parent()
            .expect("the configuration path names a directory");

        std::fs::create_dir_all(parent).expect("a directory of our own under the temporary one");
        std::fs::write(
            &path,
            "[replacement]\nmethod = \"selection\"\ninter_event_delay_ms = 37\n",
        )
        .expect("a file in a directory created a line above");

        let (config, _) = crate::settings::read_from(&path).expect("the file is valid TOML");

        // Only `[replacement]` was written; everything else came from the defaults of section 7,
        // which is the ordinary shape of a hand-edited file.
        assert_eq!(config.replacement.method, ReplacementMethod::Selection);
        assert_eq!(config.replacement.inter_event_delay_ms, 37);

        // The two calls of `app::publish_configuration_to_input_thread`, on that value.
        set_replacement_method(config.replacement.method);
        set_inter_event_delay_ms(config.replacement.inter_event_delay_ms);

        // And they are what the replacement path would read on the next hotkey press.
        assert_eq!(replacement_method(), ReplacementMethod::Selection);
        assert_eq!(inter_event_delay_ms(), 37);

        set_replacement_method(ReplacementMethod::Auto);
        set_inter_event_delay_ms(0);

        let _ = std::fs::remove_dir_all(&directory);
    }
}
