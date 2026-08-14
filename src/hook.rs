//! Installing and removing the LL hook, the callback, filtering out the program's own
//! injected input, the hook watchdog.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-01 (one `WH_KEYBOARD_LL` hook on the input thread),
//! FR-02 (the hotkey is recognised inside the callback, never through `RegisterHotKey`),
//! FR-03 (own input filtered by the signature in `dwExtraInfo`), FR-08 (auto-repeat of the
//! hotkey suppressed), FR-95 (the hotkey is suppressed whenever the program is active),
//! FR-96 (the emergency combination), FR-99 (the fail-safe callback), and the hook-removal
//! half of FR-97, FR-98 and FR-83, whose other half lives in [`crate::app`] and
//! [`crate::tray`]. NFR-01 to NFR-05 are properties of [`keyboard_hook_proc`] and are argued
//! where that function is defined.
//! Moved out to match the backlog (decision R-17): FR-05, FR-06 (decoding through
//! `ToUnicodeEx`) and FR-13 to module `buffer`, tasks T-03-2 and T-03-3; FR-80 (the hook
//! watchdog) to module `watchdog`, task T-06-2.
//! Implemented by backlog tasks: T-03-1 (done), T-03-2a (done).
//!
//! # The shape of the callback
//!
//! [`keyboard_hook_proc`] is the most dangerous function in the program, for a reason that
//! is not obvious: when it takes longer than `LowLevelHooksTimeout`
//! (`HKCU\Control Panel\Desktop`, about five seconds by default) the system removes the hook
//! **silently, without telling anybody** — FR-80. A slow callback does not produce a slow
//! program, it produces a quietly dead one. That is why NFR-01 to NFR-05 are absolute here
//! and why the body is built out of three layers that can be read separately:
//!
//! 1. **FR-96, first and unconditional.** Two integer comparisons, and only if they match, a
//!    modifier query. Nothing precedes it that could fail or loop.
//! 2. **The decision, guarded.** [`classify`] is a function of a [`Mode`], a [`HotkeyState`]
//!    and a [`KeyEvent`] and of nothing else: no Win32, no globals, no allocation, and
//!    therefore fully covered by `tests\hook.rs`. It is not *pure* — since task T-03-2 it
//!    hands the stroke to [`crate::buffer::record`] at the one point where the stroke is the
//!    user's text, and that call writes the buffer of the calling thread. It is pure of Win32,
//!    of allocation and of anything that can block, which is what NFR-01 to NFR-05 ask for. In
//!    a build that unwinds it runs inside `catch_unwind`, which is FR-99.
//! 3. **The effects**, which are exactly three: one `PostMessageW` when the hotkey has been
//!    recognised, one `PostMessageW` when a modifier has been released and the keyboard layout
//!    is worth re-reading (FR-21, task T-03-3c), and `CallNextHookEx` when the stroke is not
//!    suppressed. Never more than one of the two posts on the same stroke: a modifier is not
//!    the hotkey.
//!
//! # How a recognised hotkey press leaves this module — the interface T-03-2 attaches to
//!
//! The callback does **not** convert anything. It posts [`WM_APP_HOTKEY`] to the input
//! thread's own message-only window and returns; the conversion then runs in
//! [`handle_input_message`], on the input thread, outside the callback. Section 6.1 requires
//! exactly that — "Выполнение `SendInput` (вне callback хука)" — and NFR-01 makes it
//! unavoidable: the callback has a hundred microseconds and the buffer path (NFR-09) has
//! thirty milliseconds, three hundred times as much.
//!
//! `PostMessageW` is the right primitive and not merely a convenient one: it queues and
//! returns, it never blocks, and it never dispatches, so it cannot deadlock against a thread
//! that is holding something (NFR-04). The message carries no parameters — a keystroke must
//! never travel in a window message (SEC-01, SEC-07); everything the conversion needs is in
//! the buffer T-03-2 owns.
//!
//! # Synchronisation — section 6.3
//!
//! There is no mutex, no lock and no channel anywhere in this module; section 6.3 forbids
//! them on the hook path outright. What crosses a thread boundary crosses it as an atomic:
//! the hook handle, the notification targets, the hotkey code, the "active" flag, the
//! fail-safe flag and the counters. What does **not** cross a thread boundary — the down/up
//! state of FR-08 — is a thread-local `Cell`, because it is written and read by the input
//! thread and by no other, which is what section 6.3 says of it.
//!
//! # SEC-01, SEC-07
//!
//! No key code and no character leaves this module. The counters are counters, the messages
//! carry zeroes, and the panic payload of FR-99 is dropped unread. This is the one place in
//! the program where a keystroke is physically in a variable next to code that could format
//! a string, and nothing here formats anything.

use std::cell::Cell;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_APPS, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_END, VK_ESCAPE, VK_F1, VK_F12,
    VK_HOME, VK_INSERT, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_NUMLOCK,
    VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT,
    VK_SNAPSHOT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, PostMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use windows::core::{Error as WinError, Result as WinResult};

// ---------------------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------------------

/// The magic signature FR-03 filters on — the one and only definition of it.
///
/// Task T-04-1 puts this value into `KEYBDINPUT.dwExtraInfo` of every `INPUT` it sends, and
/// [`classify`] compares it against `KBDLLHOOKSTRUCT.dwExtraInfo`. The two must never drift
/// apart, which is why there is one constant and not two.
///
/// ⚠ **`LLKHF_INJECTED` is not used, and must never be**, in this module or anywhere else.
/// FR-03 forbids it in as many words, and the reason is not stylistic: the flag is set on
/// *every* programmatically generated stroke, including the on-screen keyboard, accessibility
/// tools and the autofill of a password manager. Filtering on it would silently break other
/// people's software. A signature we choose ourselves is set by nobody but us.
///
/// The value is not random: the low half spells `LSW` and a generation number, so that a
/// stray `dwExtraInfo` from another program colliding with it is not merely improbable but
/// recognisable in a debugger. Anything a third party sets tends to be small — a window
/// handle, a device index, a zero — and this is none of those.
pub const INJECTED_SIGNATURE: usize = 0x4C53_575F_494E_4A01;

/// Virtual-key code of the hotkey used until the UI thread publishes the configured one.
///
/// `Pause`, from section 7 of SPEC (`[hotkey] key = "Pause"`) and FR-92. It is a *default*
/// and not the setting itself: [`install`] must not read a file (NFR-08), so the hook starts
/// on this value and [`set_hotkey_vk`] replaces it as soon as the UI thread has the
/// configuration in hand. See [`install`] for why that split exists.
pub const DEFAULT_HOTKEY_VK: u16 = VK_PAUSE.0;

/// Virtual-key code of the emergency combination of FR-96 — `Ctrl+Alt+Shift+F12`.
pub const EMERGENCY_VK: u16 = VK_F12.0;

/// Message the callback posts to the input thread's window when the hotkey of FR-02 has been
/// recognised. **This is the interface task T-03-2 attaches to.**
///
/// `WM_APP + 3`: `WM_APP + 1` is the wake-up of [`crate::app`] and `WM_APP + 2` is the tray
/// callback of [`crate::tray`].
///
/// SEC-05: any process at the same integrity level can post this too, and the program is
/// built so that this buys the sender nothing privileged — the message starts a conversion of
/// text the user has already typed, using data the sender cannot see or influence, and it
/// carries no parameters that are read.
pub const WM_APP_HOTKEY: u32 = WM_APP + 3;

/// Message the callback posts to the UI thread's window when FR-99 has disarmed buffering,
/// so that the tray icon can show the "приостановлена" state FR-99 asks for.
pub const WM_APP_FAIL_SAFE: u32 = WM_APP + 4;

/// How many panics in a row FR-99 tolerates before buffering is disarmed.
///
/// FR-99 says "более трёх раз подряд" — strictly more than three — so the *fourth*
/// consecutive panic is the one that trips the fail-safe, and the first three are absorbed
/// with the program carrying on unchanged. The counter is reset by every callback that
/// returns normally: "подряд" is consecutive, not cumulative.
pub const MAX_CONSECUTIVE_PANICS: u32 = 3;

/// Exit code the process is terminated with by the emergency combination of FR-96.
///
/// Distinct from [`crate::app::EXIT_ALREADY_RUNNING`] and from `ExitCode::FAILURE`, so that
/// the acceptance run can tell "the user pressed the panic button" apart from every other way
/// this process can end.
pub const EXIT_EMERGENCY: u32 = 3;

// ---------------------------------------------------------------------------------------
// The pure decision — the part `tests\hook.rs` can reach
// ---------------------------------------------------------------------------------------

/// Which way a key is travelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// `WM_KEYDOWN` or `WM_SYSKEYDOWN`.
    Down,
    /// `WM_KEYUP` or `WM_SYSKEYUP`.
    Up,
}

/// What the callback must do with a stroke.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Hand it to `CallNextHookEx`; the application under the cursor sees it.
    Pass,
    /// Return a non-zero result; the stroke stops here and reaches nobody.
    Suppress,
}

/// The stroke, as much of `KBDLLHOOKSTRUCT` as this program has any use for.
///
/// Deliberately not `KBDLLHOOKSTRUCT`: that type can only be produced by the system, and a
/// decision that could only be tested by pressing a key on a live machine would not be tested
/// at all.
///
/// The first three fields are what [`classify`] decides on. The last three are the physical
/// half of the stroke, which the decision never reads and [`crate::buffer::record`] records:
/// task T-03-2 had to publish them through a thread-local because `tests\hook.rs` was outside
/// its file scope, and task T-03-2a made them fields, which is what they should have been.
/// One value, one nesting of `KeyEvent`, one call — the invariant "one publication, one
/// stroke" is now a property of the type rather than of the order of two calls in the most
/// dangerous function of the program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    /// `KBDLLHOOKSTRUCT.vkCode`, which the documentation confines to 1..=254.
    pub vk: u16,
    /// Derived from the message, [`edge_of`].
    pub edge: Edge,
    /// `KBDLLHOOKSTRUCT.dwExtraInfo` — the field FR-03 is about.
    pub extra_info: usize,
    /// `KBDLLHOOKSTRUCT.scanCode` — the physical key of FR-04, which is what the buffer
    /// stores and what the conversion of FR-22 is driven by.
    pub scan: u16,
    /// `KBDLLHOOKSTRUCT.flags` — carries `LLKHF_EXTENDED` (FR-05), which names the key rather
    /// than modifying it, and `LLKHF_ALTDOWN`, which corroborates the tracked `Alt` of FR-10.
    pub flags: u32,
    /// `KBDLLHOOKSTRUCT.time` — the timestamp FR-12 resolves asynchronous flush races against.
    pub time: u32,
}

/// Everything about the program's own state that the decision depends on.
///
/// Passed in rather than read from the statics inside [`classify`], so that the decision is a
/// function of its arguments and every combination of the three flags is reachable from a
/// test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    /// `general.enabled` of section 7 — the "активна" of FR-90 and FR-95.
    pub active: bool,
    /// FR-99 has disarmed buffering; everything is passed through untouched.
    pub fail_safe: bool,
    /// Virtual-key code of the hotkey of FR-02.
    pub hotkey_vk: u16,
}

/// The down/up state of the hotkey — FR-08, and nothing else.
///
/// Section 6.3 and NFR-04: this is written and read by the input thread and by no other
/// thread, so it needs neither a mutex nor an atomic, and the task specification says so in
/// as many words. It lives in a thread-local [`Cell`] in [`decide_here`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HotkeyState {
    /// Whether the hotkey is held down right now, as far as this program has seen.
    pub hotkey_down: bool,
}

/// The result of [`classify`]: what to do with the stroke, and whether a hotkey press has
/// just been recognised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Suppress the stroke or pass it on.
    pub decision: Decision,
    /// A hotkey press has been recognised and the conversion has to be started — the
    /// `PostMessageW` of [`WM_APP_HOTKEY`]. False for auto-repeat (FR-08) and for the
    /// release.
    pub fire_hotkey: bool,
    /// A modifier key has just been released and the keyboard layout is worth re-reading —
    /// the `PostMessageW` of [`crate::watchdog::WM_APP_LAYOUT`], task T-03-3c. See
    /// [`is_layout_probe`] for which strokes these are and why.
    ///
    /// A flag on the way out rather than a call inside [`classify`], for the reason the whole
    /// of this type exists: the decision stays a function of its arguments, so `tests\hook.rs`
    /// can *measure* that each of the three layout switchers of FR-11 ends in one of these
    /// instead of taking it on trust. The `PostMessageW` itself is in
    /// [`keyboard_hook_proc`], next to the one [`fire_hotkey`](Self::fire_hotkey) drives.
    pub probe_layout: bool,
}

impl Outcome {
    /// "Not ours, hand it on."
    const PASS: Self = Self {
        decision: Decision::Pass,
        fire_hotkey: false,
        probe_layout: false,
    };
}

/// Virtual-key codes of the modifier keys whose **release** is worth a layout probe —
/// task T-03-3c.
///
/// Both the sided codes and the neutral ones, for the reason [`crate::buffer`] accepts both:
/// `WH_KEYBOARD_LL` reports the sided ones, which is what makes `AltGr` recognisable at all,
/// and a neutral code arriving from anywhere else is still the same physical key.
///
/// `CapsLock` is deliberately **not** here, although [`crate::buffer`] counts it a modifier. It
/// is a toggle rather than a held key, it appears in none of the combinations FR-11 names, and
/// every release of it would be a probe that can never find anything.
const LAYOUT_PROBE_MODIFIERS: [u16; 11] = [
    VK_SHIFT.0,
    VK_LSHIFT.0,
    VK_RSHIFT.0,
    VK_CONTROL.0,
    VK_LCONTROL.0,
    VK_RCONTROL.0,
    VK_MENU.0,
    VK_LMENU.0,
    VK_RMENU.0,
    VK_LWIN.0,
    VK_RWIN.0,
];

/// Whether this stroke is the moment to ask the input thread to re-read the keyboard layout —
/// **the probe that closes the open limit of FR-21**, task T-03-3c.
///
/// # The limit
///
/// FR-21 rebuilds the cache "по сообщениям `WM_INPUTLANGCHANGE` и `WM_DEVICECHANGE`", and
/// neither reaches this program — task T-03-2a measured that. Task T-03-3 replaced them with a
/// question asked at `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS`, and that mechanism
/// stays exactly as it is. What it cannot see is a user who switches layout **without leaving
/// the window they are typing in**: no foreground changes and no focus moves, so nothing asks,
/// and every stroke until the next window change is recorded under the previous `hkl` — which
/// is the value FR-26 then converts by.
///
/// # Why a modifier release, and why that is a measured claim and not an assumed one
///
/// The three layout switchers of Windows — `Alt+Shift`, `Ctrl+Shift` and `Win+Space` — differ
/// in composition, and four earlier mechanisms failed on properties only two of them had. The
/// one property all three do share is that **all three end with a modifier being released**,
/// and a release is an event this hook already sees. `tests\hook.rs` drives each of the three
/// through [`classify`] one event at a time and asserts it; nothing here rests on the claim
/// being plausible.
///
/// The **release** and not the press, because the system performs the switch after this
/// callback has returned: a probe fired on the press would read the layout that is on its way
/// out. Both releases of a two-modifier switcher fire one, which is deliberate — the later of
/// the two is the one with the best chance of finding the switch already complete, and the
/// earlier one costs a probe that answers "unchanged".
///
/// # What a false probe costs, and why there is no threshold guarding this
///
/// Releasing `Shift` after a capital letter is a modifier release too, so this answers `true`
/// for it. That is the whole design and not an oversight: the probe **asks**, it does not
/// rebuild. `app::layout_refresh_needed` compares the layout the foreground window is running
/// against the one the buffer is recording under and returns before anything is built when
/// they agree, so an ordinary capital letter costs one `PostMessageW` here and three cheap
/// Win32 reads on the input thread — never the sweep of FR-20.
///
/// ⚠ **FR-11: this flushes nothing.** It asks a question about the layout and that is all it
/// does; the buffer is not touched here, and the path this message ends on
/// (`app::refresh_layout_and_cache`) has no `reset` on it either.
fn is_layout_probe(key: KeyEvent) -> bool {
    matches!(key.edge, Edge::Up) && LAYOUT_PROBE_MODIFIERS.contains(&key.vk)
}

/// Turns the `wParam` of the callback into an [`Edge`].
///
/// `None` for anything else. `WH_KEYBOARD_LL` is documented to deliver only these four
/// messages; a fifth would mean the system is telling us something we do not understand, and
/// the only safe answer to that is to pass the stroke on untouched.
pub fn edge_of(message: u32) -> Option<Edge> {
    match message {
        WM_KEYDOWN | WM_SYSKEYDOWN => Some(Edge::Down),
        WM_KEYUP | WM_SYSKEYUP => Some(Edge::Up),
        _ => None,
    }
}

/// The half of FR-96 that does not need Win32: is this the key of the emergency combination,
/// going down?
///
/// Split from the modifier query on purpose. This is two integer comparisons and runs on
/// **every** stroke; the three `GetAsyncKeyState` calls run only when it has already said
/// yes, that is, when `F12` goes down. Asking the system for the modifiers first would put
/// three system calls on the path of every keystroke in the machine for no gain (NFR-01).
pub fn is_emergency_key(vk: u16, message: u32) -> bool {
    vk == EMERGENCY_VK && matches!(edge_of(message), Some(Edge::Down))
}

/// The whole decision, as a function of its three arguments and of nothing else.
///
/// One effect, and it is deliberate: at the point where the stroke has been established to be
/// the user's own text, the stroke is handed to [`crate::buffer::record`] — task T-03-2, and
/// see the comment at that line for why the point is exactly there. Everything else here is a
/// comparison. No Win32, no global, no allocation, nothing that can block (NFR-01 to NFR-05).
///
/// The order of the tests is fixed by the requirements and is not free:
///
/// * FR-99 first, because a fail-safe program looks at nothing at all;
/// * then FR-03, because our own injected input must never be mistaken for the user's — and
///   it must still reach the application, so it is passed on, not suppressed;
/// * then FR-90/FR-95, because a suspended program suppresses nothing, not even the hotkey;
/// * then the hotkey itself, FR-02, FR-08 and FR-95.
///
/// FR-96 is not here: it is handled before this function is even called. See
/// [`keyboard_hook_proc`].
pub fn classify(mode: Mode, state: &mut HotkeyState, key: KeyEvent) -> Outcome {
    // FR-99. Buffering is disarmed and "весь ввод пропускается без обработки"; the hotkey is
    // not suppressed either, because a fail-safe program is not an active one and FR-95 only
    // speaks of the active case. The remembered down state is dropped so that a hotkey held
    // across the transition cannot come back as a stale "still down".
    if mode.fail_safe {
        state.hotkey_down = false;
        return Outcome::PASS;
    }

    // FR-03. Our own stroke, recognised by the signature we put there ourselves and by
    // nothing else — never by `LLKHF_INJECTED`, which is set on the on-screen keyboard, on
    // accessibility tools and on the autofill of a password manager as well. It is passed on
    // untouched: it was sent *to* the application, and suppressing it would mean replacing
    // text with nothing. What it must not do is come back into the buffer, which is why the
    // return is here and not further down (T-03-2 buffers below this line).
    if key.extra_info == INJECTED_SIGNATURE {
        return Outcome::PASS;
    }

    // FR-90, the "приостановлена" state. FR-95 makes the hotkey disappear only "когда
    // программа активна"; a suspended program has to leave `Pause` working for whatever else
    // wants it.
    if !mode.active {
        state.hotkey_down = false;
        return Outcome::PASS;
    }

    if key.vk != mode.hotkey_vk {
        // An ordinary stroke of an active program. Task T-03-2 puts it in the ring buffer
        // here; T-03-1 ended at this line by the terms of the task.
        //
        // The point is here and not higher up because everything above it is a reason *not*
        // to buffer: FR-99 has disarmed the program, FR-03 has recognised our own injected
        // input, or FR-90 has suspended it. `record` applies the flush rules of FR-10 itself
        // and returns a `Copy` value; it allocates nothing, takes no lock and calls nothing
        // of Win32 (NFR-01 to NFR-05).
        crate::buffer::record(key);

        // **FR-21, the limit task T-03-3 left open** — task T-03-3c. A modifier going up is
        // the one event all three layout switchers of FR-11 have in common, so this is where
        // the program asks whether the layout moved. It is a flag and not a call: the answer
        // travels out with the outcome and `keyboard_hook_proc` posts it, which keeps this
        // function free of Win32 (NFR-01, NFR-05) and keeps the rule reachable from a test.
        //
        // Below the three early returns above on purpose. A fail-safe program (FR-99) asks
        // nothing, our own injected input (FR-03) switches no layout, and a suspended one
        // (FR-90) has no buffer to record under — all three answer `Outcome::PASS`, whose
        // `probe_layout` is false.
        return Outcome {
            probe_layout: is_layout_probe(key),
            ..Outcome::PASS
        };
    }

    match key.edge {
        Edge::Down => {
            // FR-08. The program tracks its own down/up state and reacts to the first press
            // only: holding the key produces one conversion, not a stream of them. The system
            // sends auto-repeat as a run of `WM_KEYDOWN` with no `WM_KEYUP` between them, so
            // "first" is exactly "we did not already think it was down".
            let first_press = !state.hotkey_down;
            state.hotkey_down = true;

            Outcome {
                // FR-95: suppressed always, repeats included. The decision to suppress is
                // taken here, synchronously, and whether there is anything to convert is not
                // known until the buffer is consulted, long after this has returned.
                decision: Decision::Suppress,
                fire_hotkey: first_press,
                // The hotkey of FR-02 is not a modifier and switches no layout.
                probe_layout: false,
            }
        }

        Edge::Up => {
            state.hotkey_down = false;

            Outcome {
                // FR-95 again, and not a detail: a suppressed press whose release was let
                // through leaves the application with a key it never saw go down.
                decision: Decision::Suppress,
                fire_hotkey: false,
                probe_layout: false,
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// Names of virtual keys — section 7, `[hotkey] key`
// ---------------------------------------------------------------------------------------

/// The named keys [`vk_from_name`] understands, beyond the function keys and the single
/// characters.
///
/// Matched case-insensitively. Several spellings map to one code on purpose: a configuration
/// file is written by hand, and `Break`, `PgUp` and `Esc` are what people write.
const NAMED_KEYS: &[(&str, u16)] = &[
    ("Pause", VK_PAUSE.0),
    ("Break", VK_PAUSE.0),
    ("Escape", VK_ESCAPE.0),
    ("Esc", VK_ESCAPE.0),
    ("Insert", VK_INSERT.0),
    ("Ins", VK_INSERT.0),
    ("Delete", VK_DELETE.0),
    ("Del", VK_DELETE.0),
    ("Home", VK_HOME.0),
    ("End", VK_END.0),
    ("PageUp", VK_PRIOR.0),
    ("PgUp", VK_PRIOR.0),
    ("Prior", VK_PRIOR.0),
    ("PageDown", VK_NEXT.0),
    ("PgDn", VK_NEXT.0),
    ("Next", VK_NEXT.0),
    ("ScrollLock", VK_SCROLL.0),
    ("Scroll", VK_SCROLL.0),
    ("NumLock", VK_NUMLOCK.0),
    ("CapsLock", VK_CAPITAL.0),
    ("PrintScreen", VK_SNAPSHOT.0),
    ("Snapshot", VK_SNAPSHOT.0),
    ("Apps", VK_APPS.0),
];

/// Highest function key Windows has a virtual-key code for.
const HIGHEST_FUNCTION_KEY: u8 = 24;

/// Turns the `[hotkey] key` of section 7 into a virtual-key code.
///
/// `None` for a name this program does not know, which the caller is expected to read as
/// "keep the default" rather than as "no hotkey": a resident utility whose hotkey silently
/// stopped existing because of a typo in a file would be worse than one that answers to
/// `Pause`.
///
/// Understood: a single ASCII letter or digit, whose virtual-key code *is* its uppercase
/// ASCII value; `F1` to `F24`; and the names in [`NAMED_KEYS`]. FR-92 warns the user when the
/// key chosen is a text key — the warning is the settings dialog's business (task T-08-2),
/// and this function does not second-guess the choice.
///
/// Allocation-free, and called from the UI thread rather than from the callback: the result
/// is published into an atomic and the callback only ever reads that.
pub fn vk_from_name(name: &str) -> Option<u16> {
    let name = name.trim();

    // A single ASCII letter or digit is its own virtual-key code: `VK_A` is 0x41, which is
    // `b'A'`, and `VK_0` is 0x30, which is `b'0'`. This is a documented property of the
    // virtual-key space, not a coincidence worth hiding behind a table of 36 entries.
    if let Some(byte) = single_ascii_byte(name) {
        let upper = byte.to_ascii_uppercase();

        if upper.is_ascii_uppercase() || upper.is_ascii_digit() {
            return Some(u16::from(upper));
        }
    }

    if let Some(number) = function_key_number(name) {
        return Some(VK_F1.0 + u16::from(number) - 1);
    }

    NAMED_KEYS
        .iter()
        .find(|(label, _)| label.eq_ignore_ascii_case(name))
        .map(|(_, vk)| *vk)
}

/// The single byte of `name`, if `name` is exactly one ASCII character.
fn single_ascii_byte(name: &str) -> Option<u8> {
    let mut bytes = name.bytes();
    let first = bytes.next()?;

    if bytes.next().is_none() && first.is_ascii() {
        Some(first)
    } else {
        None
    }
}

/// The `n` of `Fn`, if `name` is a function key name in range.
fn function_key_number(name: &str) -> Option<u8> {
    let digits = name.strip_prefix(['F', 'f'])?;
    let number = digits.parse::<u8>().ok()?;

    if (1..=HIGHEST_FUNCTION_KEY).contains(&number) {
        Some(number)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------------------

/// Value meaning "there is no hook" in [`HOOK`] and "there is no window" in the two targets.
const NO_HANDLE: usize = 0;

/// The one hook of FR-01, as a raw pointer.
///
/// `HHOOK` is a raw pointer and therefore neither `Sync` nor storable in an atomic; the
/// conversion is round-trip exact in both directions, exactly as `app` does with its window
/// handles.
///
/// An atomic and not a lock because of who reads it: the callback (FR-96), the panic hook
/// (FR-98) on whichever thread panicked, the timeout thread (FR-97), the UI thread (FR-83)
/// and the input thread's own guard. A mutex here would be a mutex on the hook path, which
/// section 6.3 forbids outright, and a panic hook that waited on one could deadlock against
/// the thread that panicked while holding it.
static HOOK: AtomicUsize = AtomicUsize::new(NO_HANDLE);

/// The input thread's window, where [`WM_APP_HOTKEY`] is posted.
static HOTKEY_TARGET: AtomicUsize = AtomicUsize::new(NO_HANDLE);

/// Virtual-key code of the hotkey — FR-02, published by the UI thread (section 6.3).
static HOTKEY_VK: AtomicU32 = AtomicU32::new(DEFAULT_HOTKEY_VK as u32);

/// `general.enabled` as the callback sees it — FR-90, FR-95. Published by the UI thread.
static ACTIVE: AtomicBool = AtomicBool::new(true);

/// FR-99 has disarmed buffering.
static FAIL_SAFE: AtomicBool = AtomicBool::new(false);

/// Panics inside the callback since the last one that returned normally — FR-99.
static CONSECUTIVE_PANICS: AtomicU32 = AtomicU32::new(0);

/// How many times [`WM_APP_HOTKEY`] has been taken off the queue and acted on.
///
/// The end-to-end proof that the handoff of FR-02 works: the callback recognised the press,
/// the message survived the queue, and the input thread ran the handler. Task T-03-2 replaces
/// the body of that handler with the conversion and this counter stays as its witness.
static HOTKEY_HANDOFFS: AtomicU32 = AtomicU32::new(0);

/// `PostMessageW` calls from the callback that failed — NFR-13 without journalling.
///
/// NFR-05 forbids I/O and journalling inside the callback, and NFR-13 forbids discarding the
/// result of a Win32 call. A counter satisfies both: the result is examined and recorded, and
/// recording it costs one relaxed increment and touches no file. When module `diag`
/// (task T-06-4) arrives it may read this, and it must not put a call to itself in here.
static POST_FAILURES: AtomicU32 = AtomicU32::new(0);

/// `UnhookWindowsHookEx` calls that failed — NFR-13, same argument as [`POST_FAILURES`].
static UNHOOK_FAILURES: AtomicU32 = AtomicU32::new(0);

/// `HRESULT` of the last failed `UnhookWindowsHookEx`, or zero.
static LAST_UNHOOK_ERROR: AtomicU32 = AtomicU32::new(0);

/// `TerminateProcess` of FR-96 refused to end this process.
///
/// Almost unreachable, and recorded rather than ignored because of what it would mean: the
/// program is still running and the keyboard has already been released, so the machine is
/// usable and the acceptance run needs to know that FR-96 got half-way.
static EMERGENCY_TERMINATE_FAILED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// The down/up state of FR-08. Section 6.3: input thread only, no atomic, no mutex.
    ///
    /// `const`-initialised so that reading it compiles to a plain thread-local access with no
    /// lazy-initialisation check, no allocation and no branch worth measuring — which is what
    /// NFR-01 and NFR-03 need of the hot path.
    static HOTKEY_STATE: Cell<HotkeyState> = const { Cell::new(HotkeyState { hotkey_down: false }) };
}

// ---------------------------------------------------------------------------------------
// Public surface — installation
// ---------------------------------------------------------------------------------------

/// The installed hook, removed when this value is dropped.
///
/// Held by `app::serve_window` on the input thread for as long as that thread pumps
/// messages. Dropping it is the ordinary exit path; FR-96, FR-97, FR-98 and FR-83 all reach
/// [`uninstall`] ahead of it, and the drop then finds nothing left to do.
pub struct Installed {
    /// Neither `Send` nor `Sync`. FR-01 puts the hook on the input thread and `app` holds
    /// this value in that thread's frame; a guard that claimed the right to travel would
    /// invite a drop somewhere the hook was never installed from.
    _not_send: PhantomData<*const ()>,
}

impl Drop for Installed {
    fn drop(&mut self) {
        uninstall();
    }
}

/// Installs the one `WH_KEYBOARD_LL` hook of FR-01 on the calling thread.
///
/// `notify` is the window [`WM_APP_HOTKEY`] is posted to — the input thread's own
/// message-only window, created by `app` immediately before this call.
///
/// # Why this takes no configuration
///
/// NFR-08 gives the program under fifty milliseconds from start-up to an installed hook, and
/// the task specification names the way to lose them: reading a file, building the layout
/// cache, touching COM. So the hook goes up on [`DEFAULT_HOTKEY_VK`] — `Pause`, which is what
/// section 7 says the setting defaults to — and the UI thread, which reads the configuration
/// anyway for the tray, publishes the configured code through [`set_hotkey_vk`] a moment
/// later. Section 6.3 prescribes exactly that direction: "Конфигурация публикуется потоком
/// UI".
///
/// # Errors
///
/// Fails if a hook is already installed — FR-01 says *one* hook and two would mean every
/// stroke seen twice — or if `SetWindowsHookExW` refuses. NFR-13: a null return is an error
/// and not a success, and it is checked twice here, once by the binding and once in plain
/// sight.
pub fn install(notify: HWND, instance: HINSTANCE) -> WinResult<Installed> {
    if HOOK.load(Ordering::Acquire) != NO_HANDLE {
        return Err(WinError::from_hresult(
            windows::Win32::Foundation::E_UNEXPECTED,
        ));
    }

    // Published before the hook exists, so that the very first stroke the callback sees
    // already has somewhere to post to. Release pairs with the Relaxed load in the callback
    // on the same thread and with the Acquire elsewhere.
    HOTKEY_TARGET.store(notify.0 as usize, Ordering::Release);

    // SAFETY: `keyboard_hook_proc` is a `'static` function of this module with the exact
    // signature `HOOKPROC` demands, so the pointer the system keeps can never dangle.
    // `instance` is the module handle of the running executable, which is the module the
    // procedure lives in — the pairing `SetWindowsHookExW` requires. The thread id is zero,
    // which is what makes the hook global; that is FR-01, and it is legal for
    // `WH_KEYBOARD_LL` specifically, unlike most hook types, because a low-level keyboard
    // hook is called back in the installing thread's context rather than injected into other
    // processes. The call registers the procedure and returns; it dereferences nothing of
    // ours.
    let hook =
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), Some(instance), 0) }?;

    // NFR-13, stated in the code rather than left to the binding: `SetWindowsHookExW` reports
    // failure with a null handle, and a null handle read as success would leave the program
    // convinced it has a hook it does not have — the exact failure FR-80 is about, arrived at
    // by a different road.
    if hook.is_invalid() {
        return Err(WinError::from_thread());
    }

    HOOK.store(hook.0 as usize, Ordering::Release);

    // SEC-04a, feature `testing`, absent from the Release configuration. Read here rather
    // than in the callback, where NFR-05 forbids touching the environment.
    #[cfg(feature = "testing")]
    fault::arm_from_environment();

    Ok(Installed {
        _not_send: PhantomData,
    })
}

/// Removes the hook if there is one, and reports whether this call was the one that removed
/// it.
///
/// **Callable from any thread and at any time, including from inside the callback** (FR-96)
/// and from the panic hook (FR-98). Idempotent by construction: the handle is taken out of
/// the atomic with a single `swap`, so exactly one caller can ever see a non-null value and
/// the other paths find nothing to do. That is what lets FR-96, FR-97, FR-98, FR-83 and
/// [`Installed::drop`] all call it without coordinating.
///
/// NFR-13: the `BOOL` is examined. It is recorded in a counter rather than journalled,
/// because this function is on the FR-96 path, which runs inside the callback where NFR-05
/// forbids journalling. See [`UNHOOK_FAILURES`].
pub fn uninstall() -> bool {
    let raw = HOOK.swap(NO_HANDLE, Ordering::AcqRel);

    if raw == NO_HANDLE {
        return false;
    }

    // SAFETY: `raw` was published by `install` from the handle `SetWindowsHookExW` returned
    // and is taken out of the atomic by the `swap` above, so this thread now holds the only
    // copy of it and no second `UnhookWindowsHookEx` can be issued for the same handle.
    // Unhooking from a thread other than the installing one is permitted for `WH_KEYBOARD_LL`
    // and is what FR-97 and FR-98 need — the whole point of the timeout is that the input
    // thread may be the one that is stuck. Unhooking from *inside* the callback is permitted
    // as well: the system keeps the procedure alive until it returns and only then frees the
    // hook, which is the sequence FR-96 relies on.
    match unsafe { UnhookWindowsHookEx(HHOOK(raw as *mut c_void)) } {
        Ok(()) => true,
        Err(error) => {
            UNHOOK_FAILURES.fetch_add(1, Ordering::Relaxed);
            LAST_UNHOOK_ERROR.store(error.code().0 as u32, Ordering::Relaxed);
            false
        }
    }
}

/// Whether a hook is installed right now, as far as this program knows.
///
/// "As far as this program knows" is the whole caveat of FR-80: the system removes a
/// low-level hook silently, and this flag would still say `true`. Proving the hook is alive
/// is the watchdog's business, task T-06-2.
pub fn is_installed() -> bool {
    HOOK.load(Ordering::Acquire) != NO_HANDLE
}

// ---------------------------------------------------------------------------------------
// Public surface — the state the UI thread publishes, and the counters
// ---------------------------------------------------------------------------------------

/// Publishes the hotkey of FR-02. Called by the UI thread; see [`install`].
pub fn set_hotkey_vk(vk: u16) {
    HOTKEY_VK.store(u32::from(vk), Ordering::Relaxed);
}

/// The hotkey the callback is currently matching against.
pub fn hotkey_vk() -> u16 {
    HOTKEY_VK.load(Ordering::Relaxed) as u16
}

/// Publishes `general.enabled` — FR-90, FR-95. Called by the UI thread.
pub fn set_active(active: bool) {
    ACTIVE.store(active, Ordering::Relaxed);
}

/// Whether the program is armed. FR-95 suppresses the hotkey exactly when this is true.
pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Whether FR-99 has disarmed buffering.
pub fn fail_safe() -> bool {
    FAIL_SAFE.load(Ordering::Relaxed)
}

/// Panics inside the callback since the last one that returned normally — FR-99.
pub fn consecutive_panics() -> u32 {
    CONSECUTIVE_PANICS.load(Ordering::Relaxed)
}

/// How many recognised hotkey presses have reached [`handle_input_message`].
pub fn hotkey_handoffs() -> u32 {
    HOTKEY_HANDOFFS.load(Ordering::Relaxed)
}

/// Failed `PostMessageW` calls made from the callback — NFR-13.
pub fn post_failures() -> u32 {
    POST_FAILURES.load(Ordering::Relaxed)
}

/// Failed `UnhookWindowsHookEx` calls, and the `HRESULT` of the last of them — NFR-13.
pub fn unhook_failures() -> (u32, u32) {
    (
        UNHOOK_FAILURES.load(Ordering::Relaxed),
        LAST_UNHOOK_ERROR.load(Ordering::Relaxed),
    )
}

/// Whether the `TerminateProcess` of FR-96 failed — NFR-13.
pub fn emergency_terminate_failed() -> bool {
    EMERGENCY_TERMINATE_FAILED.load(Ordering::Relaxed)
}

/// The [`Mode`] the callback would use right now.
pub fn current_mode() -> Mode {
    Mode {
        active: is_active(),
        fail_safe: fail_safe(),
        hotkey_vk: hotkey_vk(),
    }
}

// ---------------------------------------------------------------------------------------
// The input thread's side of the handoff
// ---------------------------------------------------------------------------------------

/// The window-procedure entry point of this module, called by `app::window_proc`.
///
/// `Some` means the message was handled and that value has to be returned to Windows; `None`
/// means it is none of ours. Mirrors [`crate::tray::handle_ui_message`], and for the same
/// reason: the window procedure belongs to `app` and the knowledge of which messages matter
/// belongs to the module that defined them.
///
/// SEC-05: the list is closed and explicit, both entries carry no parameters that are read,
/// and neither initiates a privileged action.
pub fn handle_input_message(message: u32, _wparam: WPARAM, _lparam: LPARAM) -> Option<LRESULT> {
    match message {
        // FR-02, the far end of the handoff. This runs on the input thread, in its ordinary
        // message loop, with the callback long returned — which is what section 6.1 means by
        // "Выполнение SendInput (вне callback хука)".
        //
        // Both of those are done, and deliberately not here: `app::window_proc` reads the ring
        // buffer and sends the replacement through module `inject` — every `INPUT` stamped with
        // `INJECTED_SIGNATURE` so that FR-03 recognises it coming back — immediately **before**
        // it calls this function, which goes on counting every handoff below (T-03-2, T-04-1).
        WM_APP_HOTKEY => {
            HOTKEY_HANDOFFS.fetch_add(1, Ordering::Relaxed);

            // SEC-04a, feature `testing`. Outside the callback on purpose: this is the panic
            // FR-99 must *not* absorb and FR-98 must answer by removing the hook.
            #[cfg(feature = "testing")]
            fault::panic_if_handoff_armed();

            Some(LRESULT(0))
        }

        // FR-99: buffering has been disarmed and the icon has to say so. `with_tray` answers
        // `None` on any thread that is not the UI thread, so posting this to the wrong window
        // is harmless by construction. `toggle_state` is the only public way the tray offers
        // to reach the "приостановлена" icon; the guard keeps it from toggling a program that
        // is already suspended back to active.
        WM_APP_FAIL_SAFE => {
            crate::tray::with_tray(|tray| {
                if tray.enabled() {
                    tray.toggle_state();
                }
            });

            set_active(false);

            Some(LRESULT(0))
        }

        _ => None,
    }
}

// ---------------------------------------------------------------------------------------
// FR-96 — the emergency combination
// ---------------------------------------------------------------------------------------

/// Whether `Ctrl`, `Alt` and `Shift` are all held — the Win32 half of FR-96.
///
/// `GetAsyncKeyState` and not `GetKeyState`: the latter answers from the calling thread's own
/// input state, and the input thread of this program is never the keyboard focus, so it would
/// answer "nothing is held" for ever. `GetAsyncKeyState` reads the asynchronous key state of
/// the desktop, which is updated before hooks are called and is unaffected by any suppression
/// this program performs.
///
/// The three codes are the *combined* ones, so either the left or the right key of each pair
/// satisfies FR-96, which is what a user reaching for an emergency exit will expect.
fn emergency_modifiers_held() -> bool {
    // SAFETY: each call takes a virtual-key code by value, returns a `SHORT`, touches no
    // memory of ours and cannot block — it reads a table the system keeps per desktop. It is
    // callable from any thread and from inside a hook callback, which is where this runs. The
    // three are one block rather than three so that NFR-14 — a safety comment for every
    // `unsafe` — is satisfied by construction, instead of by one comment doing duty for its
    // two neighbours.
    let (ctrl, alt, shift) = unsafe {
        (
            GetAsyncKeyState(i32::from(VK_CONTROL.0)),
            GetAsyncKeyState(i32::from(VK_MENU.0)),
            GetAsyncKeyState(i32::from(VK_SHIFT.0)),
        )
    };

    is_held(ctrl) && is_held(alt) && is_held(shift)
}

/// Whether a `GetAsyncKeyState` result means "held down".
///
/// The high bit is the state; the low bit means "was pressed since the last call" and is
/// deliberately ignored, because it would make the answer depend on who called before us.
/// The value is signed, so the test is written on the unsigned reinterpretation rather than
/// on a comparison that would be wrong for exactly the values that matter.
fn is_held(state: i16) -> bool {
    (state as u16) & 0x8000 != 0
}

/// FR-96: release the keyboard and end the process, in that order.
///
/// The order is the requirement. Unhooking first means that whatever happens to the
/// termination afterwards, the machine has its keyboard back — which is the entire purpose of
/// FR-96 and the reason §4.11 of SPEC exists.
///
/// `TerminateProcess` and not `ExitProcess`: this runs inside the hook callback, called by
/// the system, and `ExitProcess` would run the loader's detach path from that context.
/// Nothing is cleaned up on the way out, and that is the trade FR-96 makes explicitly —
/// "немедленное снятие всех хуков и завершение процесса". The tray icon can be left behind as
/// a ghost until the shell next repaints the notification area; that is a cosmetic price for
/// a guaranteed escape, and the user pressing this combination has a worse problem.
fn emergency_exit() {
    uninstall();

    // SAFETY: `GetCurrentProcess` returns the process pseudo-handle, a constant that names
    // the calling process, needs no closing and cannot be invalid. `TerminateProcess` on it
    // is the documented way for a process to end itself immediately; it takes an exit code by
    // value and dereferences nothing.
    if unsafe { TerminateProcess(GetCurrentProcess(), EXIT_EMERGENCY) }.is_err() {
        // NFR-13: examined, and recorded without journalling because this is the callback —
        // NFR-05. There is nothing else to be done: the hook is already gone.
        EMERGENCY_TERMINATE_FAILED.store(true, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------------------
// FR-99 — the fail-safe callback
// ---------------------------------------------------------------------------------------

/// Runs the decision under `catch_unwind` — FR-99.
///
/// Present only in a build that unwinds. See the sibling definition below for why that is a
/// `cfg` on the panic strategy rather than on `debug_assertions`.
#[cfg(panic = "unwind")]
fn guarded_decision(key: KeyEvent) -> Outcome {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    IN_CALLBACK.with(|flag| flag.set(true));

    // `AssertUnwindSafe` is the truth here rather than a promise: the only state the closure
    // touches is the thread-local `Cell` of FR-08 and a handful of atomics, all of which are
    // plain values with no invariant that a half-finished update could break. The worst a
    // panic can leave behind is a stale "the hotkey is down", and the very next release, or
    // the fail-safe transition itself, clears it.
    let outcome = catch_unwind(AssertUnwindSafe(|| decide_here(key)));

    IN_CALLBACK.with(|flag| flag.set(false));

    match outcome {
        Ok(outcome) => {
            // "Три подряд, а не три за всё время": every callback that returns normally puts
            // the streak back to zero.
            CONSECUTIVE_PANICS.store(0, Ordering::Relaxed);
            outcome
        }

        Err(payload) => {
            // SEC-01, SEC-07. The payload is dropped **unread**: it is the one object in this
            // program that could have been built next to a key code, and formatting it, or
            // even looking at it, is exactly what those two requirements forbid.
            drop(payload);

            count_callback_panic();

            // FR-99: "весь ввод пропускается без обработки". A stroke the program failed to
            // think about must still reach the application.
            Outcome::PASS
        }
    }
}

/// Runs the decision — the `panic = "abort"` build.
///
/// The Release profile of section 3.2 sets `panic = "abort"`, and under it a panic inside the
/// callback ends the process before any recovery could run: `catch_unwind` catches nothing,
/// three panics in a row cannot happen, and the counter, the streak and the transition would
/// all be code that can never execute.
///
/// So the guard is selected on `cfg(panic = ...)` — the actual panic strategy — and not on
/// `cfg(debug_assertions)`. The difference matters: keying on the profile would leave the
/// machinery compiled into any Release build that unwinds and would break `cargo test`, which
/// unwinds while `debug_assertions` is on. Keying on the strategy states the real
/// precondition, and it is what keeps FR-99 from leaving a single line of unreachable code —
/// or a single `#[allow(dead_code)]` — in the shipped binary. Acceptance point 29.
#[cfg(not(panic = "unwind"))]
fn guarded_decision(key: KeyEvent) -> Outcome {
    decide_here(key)
}

/// Counts one panic and trips the fail-safe if FR-99's threshold has been passed.
#[cfg(panic = "unwind")]
fn count_callback_panic() {
    let streak = CONSECUTIVE_PANICS
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);

    // FR-99, "более трёх раз подряд": strictly more than three, so this fires on the fourth.
    // `swap` makes the transition happen once however many panics follow it.
    if streak > MAX_CONSECUTIVE_PANICS && !FAIL_SAFE.swap(true, Ordering::Relaxed) {
        signal_fail_safe();
    }
}

/// Asks the UI thread to show the "приостановлена" icon of FR-99.
#[cfg(panic = "unwind")]
fn signal_fail_safe() {
    let raw = crate::app::ui_window_raw();

    if raw == NO_HANDLE {
        // The UI thread has no window — it has not created one yet, or it is already gone. In
        // both cases there is no icon to change and nothing to report.
        return;
    }

    // SAFETY: `raw` was published by `app::Window::create` from a handle `CreateWindowExW`
    // returned, and is cleared before that window is destroyed, so it names either a live
    // window of this process or `NO_HANDLE`, which was filtered out above. `PostMessageW`
    // queues the message and returns; it dereferences neither parameter, both of which are
    // zero, and it does not block — which is what makes it callable from here (NFR-04).
    let posted = unsafe {
        PostMessageW(
            Some(HWND(raw as *mut c_void)),
            WM_APP_FAIL_SAFE,
            WPARAM(0),
            LPARAM(0),
        )
    };

    if posted.is_err() {
        POST_FAILURES.fetch_add(1, Ordering::Relaxed);
    }
}

thread_local! {
    /// Whether this thread is inside the guarded region of the callback right now.
    ///
    /// Read by the panic hook of FR-98, which runs on the panicking thread and has to tell a
    /// panic FR-99 is about to absorb from one that means the program is finished. Without
    /// it the panic hook would ask the whole process to shut down on the first panic in the
    /// callback and FR-99 — "программа остаётся запущенной" — could never be satisfied.
    #[cfg(panic = "unwind")]
    static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
}

/// Whether the panic now unwinding is one FR-99 will absorb.
///
/// The panic hook of FR-98 asks this first: when the answer is yes it returns without
/// removing the hook and without asking the process to come down, because the callback is
/// about to catch the panic, count it and pass the stroke through.
#[cfg(panic = "unwind")]
pub fn panic_is_absorbed() -> bool {
    IN_CALLBACK.with(Cell::get)
}

/// Whether the panic now unwinding is one FR-99 will absorb — the `panic = "abort"` build.
///
/// Always false: nothing is unwinding, the process is already on its way down, and FR-98's
/// duty to release the keyboard is the only thing left that matters.
#[cfg(not(panic = "unwind"))]
pub fn panic_is_absorbed() -> bool {
    false
}

// ---------------------------------------------------------------------------------------
// Fault injection for the acceptance run — SEC-04a, feature `testing`
// ---------------------------------------------------------------------------------------

/// Raises a panic inside the hook callback on demand, so that FR-98 and FR-99 can be
/// *demonstrated* rather than asserted.
///
/// # Why this exists
///
/// FR-98 and FR-99 are two of the four conditions the user's decision on question 18 of
/// `DECISIONS.md` makes a precondition of running this program at all. Neither can be
/// exercised from outside a finished binary: SEC-04 removes every inter-process entry point
/// on purpose, so there is no way to ask the program to panic, and a safety mechanism nobody
/// has ever seen work is exactly the kind of thing §4.11 of SPEC was written about.
///
/// It is compiled **only** under the cargo feature `testing`, which SEC-04a defines for this
/// purpose — automating the acceptance checks of §11.5 — and which is absent from the Release
/// configuration; acceptance criterion 8 of §13 of SPEC checks that the shipped binary carries
/// no trace of the feature.
///
/// # SEC-01, SEC-07
///
/// The panic message is a fixed literal. It carries no virtual-key code, no scan code, no
/// character and nothing derived from one, and the key that armed the fault is compared
/// against but never formatted. That is the same rule the rest of the module lives by, and it
/// is stated here because this is the one place in the program that raises a panic on purpose.
#[cfg(feature = "testing")]
mod fault {
    use std::sync::atomic::{AtomicU32, Ordering};

    use std::sync::atomic::AtomicBool;

    /// Environment variable naming the virtual-key code that raises the fault, in decimal.
    ///
    /// Panics **inside** the callback, which is the case FR-99 absorbs.
    const FAULT_ENV_VAR: &str = "LANGSW_TESTING_PANIC_ON_VK";

    /// Environment variable that makes the hotkey handoff panic instead.
    ///
    /// Panics **outside** the callback, on the input thread's message loop, which is the case
    /// FR-99 does *not* absorb and FR-98 therefore has to handle: remove the hook, then let
    /// the process go down. The two faults are separate variables because they exercise two
    /// different requirements and the difference between them is exactly which one is being
    /// tested.
    const HANDOFF_FAULT_ENV_VAR: &str = "LANGSW_TESTING_PANIC_ON_HANDOFF";

    /// Value meaning "no fault armed". Zero is not a virtual-key code.
    const DISARMED: u32 = 0;

    /// The armed key, read once at installation and never again.
    static PANIC_ON_VK: AtomicU32 = AtomicU32::new(DISARMED);

    /// Whether the hotkey handoff is armed.
    static PANIC_ON_HANDOFF: AtomicBool = AtomicBool::new(false);

    /// Reads the environment once, from [`super::install`].
    ///
    /// Called at installation and not from the callback: NFR-05 forbids the callback to touch
    /// the environment, and NFR-08 would not survive it either.
    pub fn arm_from_environment() {
        let armed = std::env::var(FAULT_ENV_VAR)
            .ok()
            .and_then(|raw| raw.trim().parse::<u32>().ok())
            .unwrap_or(DISARMED);

        PANIC_ON_VK.store(armed, Ordering::Relaxed);

        PANIC_ON_HANDOFF.store(
            std::env::var(HANDOFF_FAULT_ENV_VAR).is_ok(),
            Ordering::Relaxed,
        );
    }

    /// Panics if the hotkey handoff is armed — the FR-98 case.
    pub fn panic_if_handoff_armed() {
        if PANIC_ON_HANDOFF.load(Ordering::Relaxed) {
            // SEC-01, SEC-07: a literal, as above.
            panic!("langsw: injected test fault on the hotkey handoff");
        }
    }

    /// Panics if `vk` is the armed key. One relaxed load otherwise.
    pub fn panic_if_armed(vk: u16) {
        let armed = PANIC_ON_VK.load(Ordering::Relaxed);

        if armed != DISARMED && armed == u32::from(vk) {
            // SEC-01, SEC-07: a literal. Nothing about the stroke reaches this string, and
            // nothing about the stroke may ever be added to it.
            panic!("langsw: injected test fault inside the hook callback");
        }
    }
}

// ---------------------------------------------------------------------------------------
// The callback — NFR-01 to NFR-05
// ---------------------------------------------------------------------------------------

/// The value returned to suppress a stroke: anything non-zero.
const SUPPRESS: LRESULT = LRESULT(1);

/// The low-level keyboard hook procedure of FR-01.
///
/// # What this function is allowed to do
///
/// * **NFR-01, p99 under 100 µs, and NFR-02, an absolute ceiling of 1 ms.** Everything on the
///   path is a comparison, a relaxed atomic load, a thread-local read, or one of exactly two
///   Win32 calls — `CallNextHookEx`, which the system requires, and `PostMessageW`, which
///   queues and returns. `GetAsyncKeyState` is reached only when `F12` goes down. Nothing
///   here touches the disk, the registry, COM, or a window of another process.
/// * **NFR-03, zero allocations.** There is no `String`, no `Vec`, no `format!`, no
///   `to_string`, no `Box` and no collection of any kind in this function or in anything it
///   calls on the working path. [`classify`] operates on three `Copy` values.
/// * **NFR-04, no blocking primitives.** No `Mutex`, no `RwLock`, no channel, no `OnceLock`
///   — atomics and a thread-local `Cell`, which block nothing and can be entered from a
///   context that must never wait.
/// * **NFR-05, no I/O and no journalling.** No `println!`, no `eprintln!`, no `dbg!`, no
///   file, and no call into `diag`. Where NFR-13 requires the result of a Win32 call to be
///   examined, it is examined and recorded in an atomic counter.
///
/// # Safety
///
/// Called by the system with the arguments of a low-level keyboard hook. Two obligations are
/// the caller's and are met by the system on every documented path: for `code == HC_ACTION`,
/// `lparam` points at a `KBDLLHOOKSTRUCT` that is valid, initialised and properly aligned for
/// the duration of the call; and for any negative `code` the procedure must pass the event on
/// without inspecting it, which is the first thing this body does.
unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // The system's contract, and the precondition of the dereference below: a negative `code`
    // means "do not process this", and `lparam` is then not a `KBDLLHOOKSTRUCT` at all. This
    // is not logic that FR-96 has to come before — it is the condition under which there is a
    // key event to speak of, and skipping it would make the next line unsound.
    if code != HC_ACTION as i32 {
        // SAFETY: forwarding the three arguments unchanged, which is what the documentation
        // requires of a hook that does not handle the event. `None` for the handle asks the
        // system to find the next hook itself, which is the supported form since Windows XP
        // and avoids reading our own handle out of the atomic on a path that must not fail.
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    // SAFETY: `code == HC_ACTION`, so by the documented contract of `WH_KEYBOARD_LL` the
    // system has passed a pointer to a `KBDLLHOOKSTRUCT` it owns, fully initialised and
    // aligned, and it keeps that structure alive for the whole of this call. The reference
    // is used for reads only, it does not escape this function — the three fields taken from
    // it are `Copy` scalars — and its lifetime is confined to the borrow below, so it cannot
    // outlive the call. `KBDLLHOOKSTRUCT` is `#[repr(C)]` and matches the system layout,
    // which is what makes the field offsets correct.
    let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };

    // `vkCode` is documented to be in 1..=254, so the truncation cannot lose information;
    // `u16` is the width every virtual-key code in this program is carried at.
    let vk = event.vkCode as u16;
    let message = wparam.0 as u32;

    // -----------------------------------------------------------------------------------
    // FR-96 — first, before any other logic, in both build configurations.
    //
    // Nothing above this point can fail, loop or block, which is the property FR-96 needs:
    // an emergency exit standing behind code that can hang is not an emergency exit. There
    // is no `cfg` on it and there must never be one — §4.11 calls it a safeguard for the
    // user, not a debugging aid, and the decision on question 18 of DECISIONS.md makes its
    // presence in both configurations a condition of running the program at all.
    // -----------------------------------------------------------------------------------
    if is_emergency_key(vk, message) && emergency_modifiers_held() {
        emergency_exit();

        // FR-96: "Комбинация подавляется." Reached only if `TerminateProcess` refused, in
        // which case the hook is already gone and this return is the last thing the callback
        // ever does.
        return SUPPRESS;
    }
    // ----------------------------- end of the FR-96 section ------------------------------

    let Some(edge) = edge_of(message) else {
        // Not one of the four messages `WH_KEYBOARD_LL` is documented to deliver. Passed on
        // untouched rather than guessed at.
        //
        // SAFETY: as for the forwarding call above.
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    };

    // The whole stroke, read out of `KBDLLHOOKSTRUCT` in one place: the three fields the
    // decision needs and the three the record of FR-04 needs — the scan code, the flags
    // carrying `LLKHF_EXTENDED` of FR-05, and the timestamp FR-12 resolves races against.
    // Six `Copy` scalars into a value on the stack; nothing is allocated and nothing is
    // published anywhere for a later call to pick up (NFR-01, NFR-03).
    //
    // `scanCode` is documented as the scan code of the key and is carried at `u16` throughout
    // this program, which is the width `KEYBDINPUT::wScan` takes it back at (task T-04-1).
    let key = KeyEvent {
        vk,
        edge,
        extra_info: event.dwExtraInfo,
        scan: event.scanCode as u16,
        flags: event.flags.0,
        time: event.time,
    };

    let outcome = guarded_decision(key);

    if outcome.fire_hotkey {
        post_hotkey();
    }

    // FR-21, task T-03-3c. One `PostMessageW` and nothing else, on the same terms as the one
    // above: `classify` decided, this posts. The two are mutually exclusive in practice — the
    // hotkey of FR-02 is not a modifier — so no stroke ever costs both.
    if outcome.probe_layout {
        post_layout_probe();
    }

    match outcome.decision {
        Decision::Suppress => SUPPRESS,

        // FR-01: `CallNextHookEx` on every path where the stroke is not suppressed. Omitting
        // it does not merely skip the hooks behind ours — it is how a program silently breaks
        // the keyboard for everything else on the machine.
        //
        // SAFETY: as for the forwarding call above.
        Decision::Pass => unsafe { CallNextHookEx(None, code, wparam, lparam) },
    }
}

/// The decision, against this thread's FR-08 state and the published mode.
///
/// Separate from [`classify`] so that the decision keeps taking its state as arguments and
/// this — the four lines that touch a thread-local and three atomics — is the only thing
/// between it and the system.
fn decide_here(key: KeyEvent) -> Outcome {
    // Inside the guarded region on purpose: FR-98 and FR-99 are only observable if the fault
    // is raised where a real one would be. Compiled out of every build that does not ask for
    // the `testing` feature, which the Release configuration does not.
    #[cfg(feature = "testing")]
    fault::panic_if_armed(key.vk);

    let mode = current_mode();

    HOTKEY_STATE.with(|cell| {
        let mut state = cell.get();
        let outcome = classify(mode, &mut state, key);
        cell.set(state);
        outcome
    })
}

/// Posts [`WM_APP_HOTKEY`] to the input thread's window — the handoff of FR-02.
fn post_hotkey() {
    let raw = HOTKEY_TARGET.load(Ordering::Relaxed);

    if raw == NO_HANDLE {
        // No window to post to. Recorded rather than ignored (NFR-13); it can only happen
        // between the window being destroyed and the hook being removed.
        POST_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    // SAFETY: `raw` was published by `install` from the handle `CreateWindowExW` returned for
    // the input thread's own message-only window, and that window outlives the hook —
    // `app::serve_window` declares it before the hook guard, so the guard is dropped and the
    // hook removed before the window is destroyed. `PostMessageW` queues the message and
    // returns without dereferencing either parameter, both of which are zero, and without
    // blocking, which is what NFR-01, NFR-02 and NFR-04 require of anything called from here.
    let posted = unsafe {
        PostMessageW(
            Some(HWND(raw as *mut c_void)),
            WM_APP_HOTKEY,
            WPARAM(0),
            LPARAM(0),
        )
    };

    if posted.is_err() {
        // NFR-13: examined and recorded. NFR-05: recorded in an atomic, not in a journal —
        // this is the callback.
        POST_FAILURES.fetch_add(1, Ordering::Relaxed);
    }
}

/// Posts [`crate::watchdog::WM_APP_LAYOUT`] to the input thread's window — **the layout probe
/// of FR-21**, task T-03-3c. See [`is_layout_probe`] for when and why.
///
/// The same window and the same message as the delivery task T-03-3 built: the input thread's
/// message-only window answers `WM_APP_LAYOUT` by re-reading the layout of the foreground
/// window and rebuilding the cache **only if it really changed**. That mechanism is reused
/// whole and is not touched — this adds a second occasion to ask, not a second way to answer.
///
/// Written here rather than as a call to `app::post_to_input_thread`, which does the same thing
/// for module `watchdog`, for one reason: this runs inside the hook callback, and a failure has
/// to end in an atomic counter (NFR-05, NFR-13) rather than in whatever `app` reports a
/// non-critical error through. It is one atomic load and one `PostMessageW`, which is the whole
/// of what NFR-01 to NFR-05 permit on this path.
///
/// The message carries no parameters — SEC-01 and SEC-07: a keystroke must never travel in a
/// window message, and this one says only "the layout may have moved", never which key moved
/// it. SEC-05: any process at the same integrity level can post it, and all that buys the
/// sender is a re-read of the system's own layout list into memory of ours.
fn post_layout_probe() {
    let raw = HOTKEY_TARGET.load(Ordering::Relaxed);

    if raw == NO_HANDLE {
        // No window to post to — the same window and the same window-less window of time as
        // `post_hotkey`, and recorded rather than ignored for the same reason (NFR-13).
        POST_FAILURES.fetch_add(1, Ordering::Relaxed);
        return;
    }

    // SAFETY: identical to `post_hotkey` above, and for the identical reason: `raw` is the
    // handle `install` published for the input thread's message-only window, that window
    // outlives the hook, and `PostMessageW` queues the message and returns without
    // dereferencing either parameter — both are zero — and without blocking (NFR-01, NFR-02,
    // NFR-04).
    let posted = unsafe {
        PostMessageW(
            Some(HWND(raw as *mut c_void)),
            crate::watchdog::WM_APP_LAYOUT,
            WPARAM(0),
            LPARAM(0),
        )
    };

    if posted.is_err() {
        // NFR-13: examined and recorded, in an atomic and not in a journal (NFR-05).
        POST_FAILURES.fetch_add(1, Ordering::Relaxed);
    }
}
