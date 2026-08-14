//! Assembling `SendInput` packets, modifier hygiene, replacement modes.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-40 (the order of operations of a recognised hotkey
//! press, and the ban on `SendInput` inside the callback), FR-41 (one `SendInput` call with the
//! whole array, `N` backspaces by *characters*, the recoded text through `KEYEVENTF_UNICODE`,
//! surrogate pairs), FR-43 (the replacement is formed and sent **before** the layout is
//! switched), FR-44 (the configurable pause between events), FR-45 (the return value of
//! `SendInput` is compared with the number of events sent).
//! Boundary: **FR-42**, the compatibility mode through a selection, belongs to task T-04-2 and
//! is not started here; the layout switch of FR-50 that FR-40 step 5 asks for belongs to task
//! T-05-1 and is left as a marked connection point in [`replace`]; the *choice* of the target
//! layout belongs to task T-05-2 and arrives here as a parameter.
//! Implemented by backlog tasks: T-04-1 (this one), T-04-2.
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
//! that calls Win32 directly is invisible from outside it. So the four things steps 3 to 6 need
//! from the world — the modifier query, `SendInput`, the pause and the layout switch of step 5
//! — are the four members of [`Environment`], of which [`System`] is the real one.
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
//! # SEC-01, SEC-02, SEC-07
//!
//! This module holds the user's text in plain form — that is what it is for — and it is the
//! last place in the program that does. Nothing here is formatted, journalled or written
//! anywhere: [`InjectError`] carries counts and never characters, there is no `Debug` on
//! anything that holds text, and the working buffers of [`replace`] are overwritten with zeroes
//! before they are released, which is the rule module `buffer` lives by (SEC-02) applied to the
//! one other place a keystroke exists in memory.

use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, SendInput, VIRTUAL_KEY, VK_BACK,
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN,
};

use crate::convert::{self, ConvertError, Keystroke};
use crate::hook::INJECTED_SIGNATURE;
use crate::layouts::{LayoutId, LayoutMap};

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
        }
    }
}

impl std::error::Error for InjectError {}

// ---------------------------------------------------------------------------------------
// FR-40 steps 3 and 6 — the modifiers
// ---------------------------------------------------------------------------------------

/// The modifier keys FR-40 steps 3 and 6 take down and put back.
///
/// One bit per **physical** key rather than one per role: `Shift` is two keys, and a user
/// holding the right one must get the right one back. FR-40 names `Shift`, `Ctrl`, `Alt` and
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

    /// How many keys the mask names — the number of events either step will build.
    pub const fn count(self) -> usize {
        self.0.count_ones() as usize
    }
}

/// How many `INPUT` structures either of the two modifier steps can ever need.
pub const MODIFIER_COUNT: usize = 8;

/// One row of the modifier table: the bit, the key, and whether the key is an extended one.
type ModifierRow = (Modifiers, VIRTUAL_KEY, bool);

/// The eight keys of FR-40 steps 3 and 6, in the order they are released and restored.
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
/// * FR-40 step 6 says "модификаторы, которые пользователь удерживает **физически**", and the
///   whole point of asking again at step 6 is that the answer may differ from step 3's. A
///   tracker fed by hook events cannot answer better than the desktop's own key state, and it
///   cannot answer at all for a key that went down before the hook was installed.
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

/// **FR-40 step 6.** Writes a press for every key of `mods`.
///
/// `mods` must be what the user is holding **now** — [`held_modifiers`] called at step 6 and
/// not the value step 3 released. The requirement is explicit about it, and the case is
/// ordinary: the user lets go of `Shift` while the replacement is on its way, and putting it
/// back would leave the application holding a key nobody is pressing.
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

/// How many `INPUT` structures a replacement of `erase` characters by `units` code units takes.
///
/// `erase * 2` because FR-41 asks for `Backspace` **down and up**, `units` because a character
/// is one `KEYEVENTF_UNICODE` event per UTF-16 code unit — which is what makes a character
/// outside the BMP two structures without anything here having to know about it.
pub const fn replacement_events(erase: usize, units: usize) -> usize {
    erase * 2 + units
}

/// **`N` of FR-41: how many characters the recorded strokes put on the screen.**
///
/// Counted over the characters the keys *produced* and not over the keys themselves, which is
/// the whole point of the requirement: a ligature is one keystroke and several characters, and
/// erasing it with one `Backspace` would leave the rest of it on the screen. A dead key that
/// produced nothing yet contributes nothing, and a key the layout has no character for
/// contributes nothing either (FR-23) — in both cases there is nothing on the screen to erase.
///
/// Counted in Unicode scalar values and not in UTF-16 code units. The two differ only for a
/// character outside the BMP, and there the scalar value is the right unit: one `Backspace`
/// removes the whole character in every control that treats a surrogate pair as one, which is
/// what `Backspace` means, while two would eat the character before it.
pub fn typed_chars(strokes: &[Keystroke]) -> usize {
    strokes
        .iter()
        .map(|stroke| chars_in(stroke.produced().units()))
        .sum()
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

/// Whether `unit` is the leading half of a surrogate pair.
const fn is_high_surrogate(unit: u16) -> bool {
    unit >= HIGH_SURROGATE_FIRST && unit <= HIGH_SURROGATE_LAST
}

/// **FR-41.** Builds the whole replacement packet into `out`, in the order the requirement
/// fixes: `erase` × (`Backspace` down + up), then `text` as `KEYEVENTF_UNICODE` events.
///
/// Returns how many events were written, which is always [`replacement_events`] of the two
/// lengths. `out` shorter than that is [`InjectError::OutputTooSmall`] and **nothing is
/// written**: FR-41 is about a packet that is whole and in order, and a half-written one is
/// worse than none.
///
/// `text` is UTF-16 as [`crate::convert::convert_strokes`] produced it, so a character outside
/// the BMP is already the two code units of its surrogate pair and becomes two adjacent `INPUT`
/// structures here without a special case. That adjacency is a requirement — a pair split
/// between two `SendInput` calls is two undefined characters — and it is what
/// [`dispatch_with`] keeps whole when FR-44 asks for portions.
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
    for &unit in text {
        out[written] = unicode_event(unit);
        written += 1;
    }

    Ok(written)
}

// ---------------------------------------------------------------------------------------
// Building one `INPUT` — FR-03
// ---------------------------------------------------------------------------------------

/// One keyboard `INPUT` for a virtual key.
///
/// ⚠ **FR-03.** `dwExtraInfo` is [`crate::hook::INJECTED_SIGNATURE`] here and in
/// [`unicode_event`], and those two functions are the only places in the program that build an
/// `INPUT`. There is deliberately no third: a structure assembled anywhere else could miss the
/// signature, `hook::classify` would take it for the user's own typing, it would go into the
/// buffer and the next hotkey press would convert the program's own output.
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
/// See [`key_event`] for `dwExtraInfo`, `time` and why these are the only two builders.
fn unicode_event(unit: u16) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: KEYEVENTF_UNICODE,
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

/// Publishes `[replacement] inter_event_delay_ms` — FR-44.
///
/// Section 6.3 fixes the direction: the configuration is published by the UI thread, which is
/// the only thread allowed to read the file, and the input thread reads the atomic. Nothing
/// here reads a file, which is what NFR-09 requires of everything on the replacement path.
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

/// Everything steps 3 to 6 of FR-40 need from outside this program.
///
/// Four members, and each of them is a requirement rather than a convenience: the modifier
/// query of steps 3 and 6, the `SendInput` of FR-41, the pause of FR-44 and the layout switch
/// of step 5. [`System`] is the real one and is what the program runs on.
///
/// # Why it is a trait and not four direct calls
///
/// FR-40 is an **order**, and an order is not observable from outside a function that performs
/// it. With the outside world behind this trait, `tests\inject.rs` can watch the sequence steps
/// 3 to 6 really produce — that the modifiers come off before the first character is replaced,
/// that the replacement is complete before the layout switch is reached, that only what is held
/// at step 6 is put back — without a keyboard, without a foreground window and without sending
/// one event into the machine. There is no other honest way to check an order.
///
/// It is also the seam the next two tasks attach to: **T-05-1** fills in [`switch_layout`], and
/// **T-04-2** builds a different packet for the same three steps.
///
/// [`switch_layout`]: Environment::switch_layout
pub trait Environment {
    /// The modifier keys held **at this instant** — FR-40 steps 3 and 6.
    ///
    /// Asked twice per replacement and expected to answer differently: step 6 restores what the
    /// user is holding then, not what step 3 found.
    fn held(&mut self) -> Modifiers;

    /// Hands one portion to the system and answers how many events it accepted — FR-41, FR-45.
    fn send(&mut self, events: &[INPUT]) -> u32;

    /// The pause of FR-44. Reached only when the configured delay is above zero.
    fn pause(&mut self, delay_ms: u32);

    /// **FR-40 step 5 — the layout switch. The connection point of task T-05-1.**
    ///
    /// Called after the replacement has been sent and before the modifiers are restored, which
    /// is the position **FR-43** fixes: "Замена выполняется до переключения раскладки". The
    /// default does nothing, and that is the whole of what task T-04-1 leaves here — §4.6
    /// (FR-50 to FR-52) is task T-05-1's and is not started.
    ///
    /// It is a member of this trait rather than a comment in the body so that the position of
    /// the switch is something a test can *see*: FR-43 is a statement about order, and an order
    /// nobody can observe is an order nobody can check.
    fn switch_layout(&mut self) {}
}

/// The real machine — the [`Environment`] the program runs on.
///
/// A unit type: everything it does is a system call, so there is no state to carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct System;

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

    // `switch_layout` is deliberately not overridden. TODO(T-05-1): the chain of FR-50 goes
    // here, and this is the only place in the program that has to change for it.
}

/// **FR-41 and FR-44.** Sends `events` through the real system, and records what happened
/// (FR-45).
///
/// See [`dispatch_in`] for the rule.
pub fn dispatch(events: &[INPUT], delay_ms: u32) -> Dispatched {
    dispatch_in(&mut System, events, delay_ms)
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
/// One event, except that a surrogate pair is never split: FR-41 says a character outside the
/// BMP goes as two `INPUT` structures, and two halves of a character delivered in separate
/// calls are two undefined characters, not one character delivered slowly. The pair is
/// recognised from the events themselves — a `KEYEVENTF_UNICODE` event whose `wScan` is a
/// leading surrogate takes its neighbour with it — so no side table has to be carried from the
/// builder to here and the two cannot disagree.
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

/// Where the portion starting at `start` ends — one event, or two for a surrogate pair.
fn portion_end(events: &[INPUT], start: usize) -> usize {
    let pair = start + 1 < events.len()
        && keyboard(&events[start]).is_some_and(|key| {
            key.dwFlags.0 & KEYEVENTF_UNICODE.0 != 0 && is_high_surrogate(key.wScan)
        });

    if pair { start + 2 } else { start + 1 }
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
    /// Characters erased — the `N` of FR-41, by [`typed_chars`].
    pub erased: usize,
    /// UTF-16 code units typed back.
    pub typed: usize,
    /// Modifiers taken down by step 3.
    pub released: Modifiers,
    /// Modifiers put back by step 6 — only those held physically at that moment.
    pub restored: Modifiers,
    /// What step 3 did.
    pub release: Dispatched,
    /// What step 4 did — the replacement itself, FR-41.
    pub replacement: Dispatched,
    /// What step 6 did.
    pub restore: Dispatched,
}

/// **FR-40, steps 3 to 6, for one recognised hotkey press.**
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
/// 5. **step 5** — the layout switch, task T-05-1, marked below and deliberately empty;
/// 6. **step 6** — the modifiers held *at that moment* are pressed again.
///
/// **FR-43** is the position of step 5 and nothing else: the replacement is formed and sent
/// before the layout is switched. `KEYEVENTF_UNICODE` carries a character literally, past the
/// layout, so there is no race to lose — but the order still has to be built in, because a
/// switch performed first would be visible to every event that followed it.
///
/// # Errors
///
/// Only sizing failures, and neither is reachable from [`on_hotkey`], which sizes both working
/// buffers from the lengths it is about to use. They are returned rather than asserted because
/// a panic on this path would take a resident program down over a full buffer.
pub fn replace(
    strokes: &[Keystroke],
    target: &LayoutMap,
    delay_ms: u32,
) -> Result<Replaced, InjectError> {
    replace_in(&mut System, strokes, target, delay_ms)
}

/// **FR-40, steps 3 to 6**, against any [`Environment`] — see [`replace`] for the requirement.
///
/// This is the function the order lives in, and the order is the requirement, so this is what
/// `tests\inject.rs` drives.
pub fn replace_in(
    env: &mut impl Environment,
    strokes: &[Keystroke],
    target: &LayoutMap,
    delay_ms: u32,
) -> Result<Replaced, InjectError> {
    // FR-41: `N` is the number of characters on the screen, not the number of keys pressed.
    let erased = typed_chars(strokes);

    // The whole packet is *formed* here, before anything is sent and long before step 5 —
    // FR-43. `max_units` is the length no conversion of these strokes can exceed, so the
    // error below is unreachable and is handled anyway (NFR-13).
    let mut text = vec![0u16; convert::max_units(strokes.len())];
    let typed = convert::convert_strokes(strokes, target, &mut text)
        .map_err(|ConvertError::BufferTooSmall { needed }| InjectError::TextTooLong { needed })?;

    let mut events = vec![INPUT::default(); replacement_events(erased, typed)];
    let built = build_replacement(erased, &text[..typed], &mut events);

    // The working buffers hold the user's text in plain form and are the only place outside
    // module `buffer` that ever does. Whatever happens below, they are zeroed before this frame
    // releases them — SEC-01, SEC-02.
    let outcome = built.map(|len| run_steps(env, &events[..len], erased, typed, delay_ms));

    text.fill(0);
    events.fill(INPUT::default());

    outcome
}

/// Steps 3 to 6 of FR-40 around a packet that is already built.
fn run_steps(
    env: &mut impl Environment,
    packet: &[INPUT],
    erased: usize,
    typed: usize,
    delay_ms: u32,
) -> Replaced {
    let mut modifier_events = [INPUT::default(); MODIFIER_COUNT];

    // ---- step 3 -----------------------------------------------------------------------
    let released = env.held();
    let release = match build_release(released, &mut modifier_events) {
        // The array is `MODIFIER_COUNT` long and `released.count()` can never exceed it, so
        // the error is unreachable; it is answered by sending nothing rather than by a panic.
        Err(_) => Dispatched::default(),
        Ok(len) => dispatch_in(env, &modifier_events[..len], delay_ms),
    };

    // ---- step 4, FR-41 ----------------------------------------------------------------
    let replacement = dispatch_in(env, packet, delay_ms);

    // ---- step 5 — the connection point of task T-05-1 ----------------------------------
    //
    // FR-40 step 5 switches the layout of the foreground window and §4.6 (FR-50 to FR-52) says
    // how. It is task T-05-1's, and what task T-04-1 leaves is this call to a trait member
    // whose default body is empty.
    //
    // ⚠ **FR-43: the call goes here and nowhere earlier.** Everything above has already been
    // sent; everything below only puts back what step 3 took away. A switch moved in front of
    // step 4 would be the race FR-43 was written to rule out.
    env.switch_layout();

    // ---- step 6 -----------------------------------------------------------------------
    //
    // Asked again, and this is the requirement: "модификаторы, которые пользователь удерживает
    // физически" — at *this* moment, not at step 3's. The user may have let go while the
    // replacement was on its way, and pressing a key nobody is holding would be worse than
    // leaving it alone.
    let restored = env.held();
    let restore = match build_restore(restored, &mut modifier_events) {
        Err(_) => Dispatched::default(),
        Ok(len) => dispatch_in(env, &modifier_events[..len], delay_ms),
    };

    Replaced {
        erased,
        typed,
        released,
        restored,
        release,
        replacement,
        restore,
    }
}

// ---------------------------------------------------------------------------------------
// The far end of the FR-02 handoff — what `app::window_proc` calls
// ---------------------------------------------------------------------------------------

/// **FR-40 for the hotkey press the input thread has just taken off its queue.**
///
/// This is the whole of what `app::window_proc` does with [`crate::hook::WM_APP_HOTKEY`]: the
/// callback recognised the press, suppressed it (step 1) and posted the message (step 2), and
/// this runs steps 3 to 6 on the input thread, outside the callback, where section 6.1 puts
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
/// * the target layout is task **T-05-2**'s choice; until it exists, [`interim_target`] stands
///   in and is documented there as the interim it is;
/// * `[replacement] method = "selection"` (FR-42) branches here in task **T-04-2**; this
///   version always takes the `backspace` mode of FR-41;
/// * the layout switch of FR-40 step 5 is task **T-05-1**'s and is marked inside [`replace`].
pub fn on_hotkey() -> Option<Replaced> {
    let (mut strokes, active) = take_strokes()?;

    let outcome = interim_target(active)
        .map(|target| replace(&strokes, &target, inter_event_delay_ms()))
        .and_then(Result::ok);

    // The copy holds the user's text; it is zeroed before it is released — SEC-01, SEC-02.
    strokes.fill(Keystroke::default());
    drop(strokes);

    if outcome.is_some() {
        // The last row of the FR-10 table, as module `buffer` describes it: the session is
        // marked converted and the buffer is deliberately **not** flushed, so that FR-32 can
        // render the same scan codes into the next layout on the next press. It is the next
        // ordinary key that ends the session, and `Recorder::record` flushes then.
        crate::buffer::note_conversion();
    }

    outcome
}

/// Copies the live strokes of this thread's buffer out, with the layout they were typed under.
///
/// `None` on a thread with no buffer, and on an empty buffer — there is nothing to replace.
fn take_strokes() -> Option<(Vec<Keystroke>, LayoutId)> {
    let taken = crate::buffer::with(|recorder| {
        let mut strokes = vec![Keystroke::default(); recorder.len()];

        // The slice was sized from `recorder.len()` under this very borrow, so
        // `ReadError::OutputTooSmall` cannot happen; it is answered with zero rather than with
        // an `expect`, because a resident program must not end on a full buffer.
        let live = recorder.keystrokes(&mut strokes).unwrap_or(0);
        strokes.truncate(live);

        (strokes, recorder.active_layout())
    })?;

    if taken.0.is_empty() {
        return None;
    }

    Some(taken)
}

/// The layout the conversion targets, **until task T-05-2 chooses it**.
///
/// ⚠ This is an interim and is marked as one. FR-30 to FR-34 give the choice its own rules —
/// the "Пара" and "Цикл" modes, the position counter, `[layouts]` of section 7 — and task
/// T-05-2 owns them; task T-04-1 may not start on them and does not. What it may not do either
/// is leave the running program with no target at all, because then the whole path this task
/// exists to build would be unreachable and unobservable.
///
/// So the interim rule is the narrowest one that works and cannot be mistaken for the real
/// thing: the counterpart of the recorded layout inside the hardwired RU/EN table of FR-25, and
/// `None` for every other layout, which means no replacement rather than a guess. It reaches
/// only module `convert`, allocates two small maps and touches neither the OS nor a file, so it
/// stays inside NFR-09.
///
/// TODO(T-05-2): replace with the target layout chosen by `[layouts]` and the cycle of FR-31.
fn interim_target(active: LayoutId) -> Option<LayoutMap> {
    let counterpart = if active == convert::FALLBACK_US {
        convert::FALLBACK_RUSSIAN
    } else if active == convert::FALLBACK_RUSSIAN {
        convert::FALLBACK_US
    } else {
        return None;
    };

    convert::fallback_map(counterpart)
}

// ---------------------------------------------------------------------------------------
// Tests of what needs no Win32 at all
// ---------------------------------------------------------------------------------------

/// The private helpers `tests\inject.rs` cannot reach.
///
/// Everything a requirement is stated about is public and is tested there; these three are
/// implementation details whose behaviour the public functions rest on.
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
}
