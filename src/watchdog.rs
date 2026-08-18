//! Subscriptions to system events, restoring the hook.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-13 (mouse clicks over **Raw Input**, never
//! `WH_MOUSE_LL`), the three asynchronous rows of the FR-10 flush table — a mouse button over
//! Raw Input, `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS` — the **delivery** of
//! FR-21, that is, making the layout cache actually rebuild on a machine whose hidden windows
//! never receive `WM_INPUTLANGCHANGE` — task T-03-3 — and **FR-80, the hook watchdog proper**,
//! with all four of its mechanisms: the `EVENT_SYSTEM_DESKTOPSWITCH` subscription, the
//! `WM_WTSSESSION_CHANGE` registration, `WM_POWERBROADCAST` with `PBT_APMRESUMEAUTOMATIC`, and
//! the thirty-second liveness timer — task T-06-2.
//! FR-12, the race resolution these subscriptions feed, belongs to [`crate::buffer`]: the module
//! table gives "разрешение гонок по временны́м меткам" to the buffer, and this module hands it a
//! timestamp and nothing else.
//! Moved out to match the backlog (decision R-17): FR-82 (single instance) to the process
//! level, task T-01-2; FR-83 (clean shutdown) to the tray, task T-01-4, module `tray`.
//! Implemented by backlog tasks: T-03-3 (done), T-06-2 (done), T-08-4 (done).
//!
//! # ⚠ Why there is no keyboard entry in the Raw Input registration — task T-08-4
//!
//! There was one until task T-08-4, and it cost this program every keystroke it was supposed to
//! see. **A process that registers any keyboard top-level collection with
//! `RegisterRawInputDevices` stops the whole chain of low-level keyboard hooks — its own and
//! every other process's — for as long as a window of that process holds the foreground.** The
//! keystroke is not lost: it is delivered to that process as `WM_INPUT`, by a road the hooks are
//! not on.
//!
//! That is measured, on a dummy built out of PowerShell with no line of this program in it, and
//! the measurement is in the report of task T-08-4. Three of its results decide the shape of
//! this module:
//!
//! * **the flag is irrelevant.** `RIDEV_DEVNOTIFY`, `RIDEV_INPUTSINK`, both together, and even a
//!   null `hwndTarget` all starve the chain alike. What does it is *being a raw keyboard client*,
//!   not what one asks for as a client;
//! * **the window is irrelevant.** The registration this program made named the input thread's
//!   `HWND_MESSAGE` window, which can never be in the foreground, and it starved the chain
//!   exactly the same. It is a property of the **process**;
//! * **the mouse entry is innocent.** Mouse-only registrations, with `RIDEV_INPUTSINK` and even
//!   with `RIDEV_DEVNOTIFY`, do not starve anything. FR-13 keeps the entry it always had.
//!
//! What the keyboard entry was *for* is the `WM_DEVICECHANGE` half of the FR-21 delivery, and
//! that has not been given up: [`register_device_notice`] asks for the same news through
//! `RegisterDeviceNotificationW`, which is not a raw input registration at all — see below.
//!
//! ⚠ The comment this module used to carry said the entry cost "exactly zero messages while the
//! user types" because "our window never has the keyboard focus". Both halves were wrong: the
//! settings dialog of FR-92 and the tray menu of FR-91 are windows of ours that do hold the
//! foreground, and in that state the entry cost not zero messages but **all** of them, including
//! the emergency combination of FR-96 — the one way out of a wedged hook.
//!
//! # Why Raw Input and not `WH_MOUSE_LL` — FR-13
//!
//! FR-13 does not merely prefer Raw Input, it forbids the alternative and gives the reason:
//! `WH_MOUSE_LL` is called **on every movement of the cursor** and puts this process in the
//! system's critical path for mouse input. Every pixel the user moves the mouse would be a
//! cross-process call into our callback, and every millisecond spent there would be a
//! millisecond the cursor did not move — the same `LowLevelHooksTimeout` regime the keyboard
//! hook lives under (FR-80), but on a stream that is two orders of magnitude denser.
//!
//! Raw Input inverts that: `RegisterRawInputDevices` with `RIDEV_INPUTSINK` asks the system to
//! **post** `WM_INPUT` to a window of ours, so the data arrives in our own message queue, is
//! read whenever this process gets round to it, and nothing in the system waits for us. The
//! word "запрещён" in FR-13 is honoured mechanically: the string `WH_MOUSE_LL` does not occur
//! anywhere under `src\`, and `SetWindowsHookExW` is called exactly once in this program, in
//! [`crate::hook`], with `WH_KEYBOARD_LL`.
//!
//! # How FR-21 learns that a keyboard arrived — task T-08-4
//!
//! FR-21 says the cache is rebuilt "по сообщениям `WM_INPUTLANGCHANGE` и `WM_DEVICECHANGE`", and
//! [`crate::layouts::REBUILD_MESSAGES`] is that list. Task T-03-2a wrote the handler for both and
//! then found that neither message can reach this program on its own: `WM_INPUTLANGCHANGE` goes
//! to the window with the keyboard focus, and `WM_DEVICECHANGE` at the interface level goes only
//! to windows that asked for device notifications. Task T-03-3 answered the second half with the
//! keyboard entry of the Raw Input registration, whose `RIDEV_DEVNOTIFY` turns a keyboard
//! arrival into `WM_INPUT_DEVICE_CHANGE` — a stand-in for `WM_DEVICECHANGE` rather than the
//! message itself.
//!
//! [`register_device_notice`] replaces the stand-in with the real thing: a
//! `RegisterDeviceNotificationW` on the **input thread's window**, filtered to the keyboard
//! device interface class, which delivers `WM_DEVICECHANGE` there and rebuilds through the
//! handler task T-03-2a wrote for it and that had never once fired. Three properties, each
//! measured rather than assumed (report of task T-08-4):
//!
//! * **it does not starve the hooks.** It is not a raw input registration; the witness hook in a
//!   third process keeps getting its two calls per keystroke with our window in front;
//! * **it reaches a message-only window.** Registered device notifications do, where the
//!   *broadcast* ones reach top-level windows only — which is why this can stay on the input
//!   thread's window and needs no cross-thread hand-off;
//! * **it is precise.** Filtered to one interface class, it stays silent for every device that
//!   is not a keyboard. The alternative — the free `DBT_DEVNODES_CHANGED` broadcast at the UI
//!   window — would run FR-20's sweep of thousands of `ToUnicodeEx` calls every time anything at
//!   all appeared in the machine's device tree.
//!
//! Task T-03-3 rejected `RegisterDeviceNotification` because the filter structure and the class
//! GUID lived behind a `windows` feature outside section 3.2. In `windows` 0.62.2 — the version
//! this program is pinned to — `RegisterDeviceNotificationW`, `UnregisterDeviceNotification`,
//! `DEV_BROADCAST_DEVICEINTERFACE_W`, `DBT_DEVTYP_DEVICEINTERFACE` and
//! `DEVICE_NOTIFY_WINDOW_HANDLE` are all in `Win32::UI::WindowsAndMessaging`, a feature this
//! program already enables. `Cargo.toml` is untouched.
//!
//! # FR-80 — the watchdog, and why it cannot ask whether the hook is alive
//!
//! FR-80 names four ways the system takes a low-level hook away **without saying so**: the
//! callback overrunning `LowLevelHooksTimeout` (`HKCU\Control Panel\Desktop`, about five
//! seconds by default), a switch to a secure desktop (a UAC prompt, `Ctrl+Alt+Del`), a session
//! change, and a resume from sleep. In every one of them the program keeps running, the tray
//! icon keeps sitting there, [`crate::hook::is_installed`] keeps answering `true` — and not one
//! keystroke reaches the callback ever again.
//!
//! **There is no call that answers "is my hook still registered".** `UnhookWindowsHookEx` is
//! the only function in the API that touches an installed `HHOOK`, and it is destructive.
//! Nothing enumerates the hook chain. The absence of callbacks proves nothing: the user may
//! simply not be typing. That is not a detail of this implementation, it is the shape of the
//! problem, and it decides the design:
//!
//! * three of the four cases announce themselves with an **event**, and this module subscribes
//!   to all three — [`WATCHED_EVENTS`] adds `EVENT_SYSTEM_DESKTOPSWITCH` to the subscription
//!   [`watch`] already made, [`register_session_notice`] asks for `WM_WTSSESSION_CHANGE`, and
//!   the `PBT_APMRESUMEAUTOMATIC` arm of [`handle_watchdog_message`] answers the resume;
//! * the fourth — the timeout — announces nothing at all, so the thirty-second timer of FR-80
//!   **reinstalls unconditionally** rather than checking first, because there is nothing to
//!   check. [`reinstall_hook`] is the single path all four mechanisms end in.
//!
//! What an unconditional reinstall costs, and what it does not:
//!
//! * **the gap.** Between `UnhookWindowsHookEx` and `SetWindowsHookExW` this process has no
//!   hook. Keystrokes made in that window are not seen by the callback, so they are not
//!   buffered — they reach the application the user is typing into exactly as they always
//!   would, because this program suppresses nothing but its own hotkey. The gap is measured
//!   rather than assumed: [`Health::last_gap_us`] and [`Health::max_gap_us`];
//! * **the order is forced, and it is the right one.** [`crate::hook::install`] refuses while a
//!   hook is registered — FR-01 says *one* hook — so "install then uninstall" is not
//!   expressible against the accepted module at all. It is also the order that cannot produce
//!   two live hooks for even an instant, and two live hooks would mean every stroke recorded
//!   twice: `gghbbddttnn` in the buffer instead of `ghbdtn`;
//! * **what it cannot tell in advance** is whether the hook needed replacing. It can tell
//!   afterwards, and does: if `UnhookWindowsHookEx` fails on a handle this program believed
//!   live, the system had already taken it, and that is counted as
//!   [`Health::silent_removals`]. That count is a diagnostic, not a decision — the reinstall
//!   happens either way.
//!
//! ⚠ **Synthetic input is not used as a pulse.** Sending a keystroke with `SendInput` to see
//! whether it comes back through the callback would put keystrokes into whatever window the
//! user has in front of them, on a timer, and would entangle the watchdog with the injected-
//! input filter of FR-03. There is no `SendInput` in this module and no other module is asked
//! to make one on its behalf.
//!
//! # Which window receives what — FR-81's rake, stepped on twice
//!
//! `WM_POWERBROADCAST` is delivered to **top-level windows**. Two of this program's three
//! windows are `HWND_MESSAGE` children (`app::Window::create`), and a message-only window is
//! not top-level: it is out of `EnumWindows`, out of the Z order, and out of every broadcast.
//! Measured, not argued — the report of task T-06-2 enumerates the three windows of a running
//! process and posts the message to each of them in turn.
//! This is the same rake FR-81 already stepped on — `RegisterWindowMessage("TaskbarCreated")`
//! never reached `HWND_MESSAGE` either, which is why decision R-20 point 2 made the UI thread's
//! window a hidden top-level one with `WS_EX_TOOLWINDOW`. **That window is the only one of ours
//! a power broadcast can reach**, so [`handle_watchdog_message`] answers `WM_POWERBROADCAST`
//! and `WM_WTSSESSION_CHANGE` on it and on no other — see [`is_ui_window`]. The registration of
//! [`register_session_notice`] names the same window for the same reason.
//!
//! Section 6.1 then decides where the *work* happens, and it is not there: the line "таймер
//! сторожа" stands under the **input thread**, and a `WH_KEYBOARD_LL` hook is called back on
//! the thread that installed it, so the hook must be reinstalled by that thread and by no
//! other. The UI window and the `WinEvent` callback therefore only **mark and wake** —
//! [`request_rehook`] is one atomic store and one `PostMessageW` — and [`reinstall_hook`] runs
//! on the input thread, out of its ordinary message loop.
//!
//! # Threading — section 6.1
//!
//! Section 6.1 puts each of these where it belongs and this module follows it exactly:
//!
//! * **input thread** — `RegisterRawInputDevices (RIDEV_INPUTSINK)`, the reception of `WM_INPUT`
//!   and, from task T-08-4, the device notification of FR-21 and the `WM_DEVICECHANGE` it
//!   delivers. The registration is bound to that thread's message-only window, and the buffer
//!   the flush lands in is a thread-local of that same thread (section 6.3), so the whole path
//!   from the click to the zeroed slot happens without a single cross-thread hand-off. Section
//!   6.1 also puts "таймер сторожа" here in so many words, and [`start_liveness_timer`] is that
//!   line: the timer is set on the input window, so `WM_TIMER` and every reinstall it causes
//!   happen on the one thread that is allowed to own the hook;
//! * **watcher thread** — `SetWinEventHook` for `EVENT_SYSTEM_FOREGROUND`, `EVENT_OBJECT_FOCUS`
//!   and, from task T-06-2, `EVENT_SYSTEM_DESKTOPSWITCH` — the three events section 6.1 lists
//!   under this thread. An out-of-context `WinEvent` hook calls back on the thread that
//!   installed it, so installing it there is what puts the callback there;
//! * **UI thread** — `WTSRegisterSessionNotification` and the two messages the system delivers
//!   to a top-level window. Not a choice: that thread owns the only top-level window in the
//!   process, and no other window can be given these messages.
//!
//! # NFR-01 to NFR-05, NFR-10 — what the callbacks are allowed to do
//!
//! A `WinEvent` callback is called by the system, synchronously, from inside the event source's
//! own delivery, exactly like a hook callback: a slow one holds up whatever generated the event.
//! [`win_event_proc`] is therefore built to the same budget as the keyboard callback even though
//! no requirement names a number for it — **two read-only window queries
//! (`GetForegroundWindow` and `GetAncestor`, the gate of task T-10-0), one atomic swap (the
//! focus memory of task T-10-0e), one compare-and-swap on an atomic, two counter increments
//! and one `PostMessageW`**, none of which blocks: the two queries read the window manager's
//! session state without entering any other process and without taking any lock an
//! application could hold, the same standing `GetMessageTime` has on the `WM_INPUT` path. No
//! allocation (NFR-03), no lock (NFR-04), no I/O (NFR-05), no Win32 call that can be made to
//! wait, and nothing that can panic across the `extern "system"` boundary. The work the event
//! implies — flushing a ring of strokes, and possibly rebuilding the layout cache — is done
//! later, by the input thread, in its message loop. For the overwhelmingly common event — the
//! background churn [`concerns_the_foreground`] turns away — the callback now does *less*
//! than it did before task T-10-0: the two queries and one increment, and neither post. The
//! churn of the foreground window itself — task T-10-0e, [`focus_repeated`] — stops one step
//! later, at the swap, and skips the compare-and-swap and both posts.
//!
//! The `WM_INPUT` path is not a callback at all: it is a message, handled in the message loop
//! of the input thread. Its cost is one `GetRawInputData` into a stack buffer and a comparison
//! of two bits, and for the overwhelmingly common packet — a cursor movement — it stops at that
//! comparison and touches the buffer not at all.
//!
//! NFR-10, "загрузка ЦП в покое неотличима от нуля": nothing here polls, and every one of the
//! subscriptions is silent until the user moves the mouse, clicks, or changes window — that is,
//! until the machine is by definition not at rest. The one thing that does wake the process on
//! its own is the timer FR-80 asks for, and it is the coarsest kind there is: a single
//! `SetTimer` at [`LIVENESS_INTERVAL_MS`], thirty seconds, which the system coalesces with
//! whatever else it is waking the machine for. Two wake-ups a minute, each of them two Win32
//! calls long. No thread of this program sleeps in a loop, and no interval anywhere in this
//! module is shorter than that one.
//!
//! # SEC-05
//!
//! Every message this module answers can be posted to our windows by any process at the same
//! integrity level, and none of them initiates a privileged action:
//!
//! * a forged `WM_INPUT` carries a handle `GetRawInputData` rejects, and a rejected read is an
//!   early return;
//! * a forged [`WM_APP_FLUSH`] finds no pending request — the timestamp travels in an atomic of
//!   this process and never in the message — and does nothing at all;
//! * a forged [`WM_APP_LAYOUT`], `WM_DEVICECHANGE` or `WM_INPUT_DEVICE_CHANGE` buys the sender a
//!   re-read of the system's own layout list into memory of ours, which is idempotent, and is the
//!   same standing the wake-up message of [`crate::app`] has. The device notification of
//!   [`register_device_notice`] changes nothing here: `WM_DEVICECHANGE` was already forgeable at
//!   any of our windows before the registration existed, and all it has ever bought is that
//!   re-read;
//! * a forged [`WM_APP_REHOOK`], `WM_POWERBROADCAST` or `WM_WTSSESSION_CHANGE` buys the sender
//!   one reinstallation of **our own** hook — an operation this program performs on itself
//!   every thirty seconds anyway, which changes no state the sender can see and grants no
//!   privilege. The rehook message carries nothing: the reason travels in [`PENDING_REASON`],
//!   an atomic of this process, and a forged message that finds it empty does nothing at all.
//!   The two system messages are answered on the UI window and on no other, so a copy aimed at
//!   either message-only window is ignored where the real one could never have arrived;
//! * `WM_TIMER` is answered only for [`LIVENESS_TIMER_ID`] and only on the thread that owns the
//!   buffer, so a forged one aimed at another window of ours falls through to `DefWindowProcW`.
//!
//! # SEC-01, SEC-07
//!
//! Nothing here is written to a log, a file or a panic message. What crosses this module is a
//! button-flag word, a millisecond timestamp, a count of events and a reason code that is one of
//! five named constants; no key code, no scan code and no character passes through it, and the
//! [`Counters`] and [`Health`] this module publishes are counts, durations and that reason code
//! and nothing else. [`Reason`] deliberately has no arm that could carry a key: the four
//! mechanisms of FR-80 are the four arms, and a stroke is not one of them.

use core::ffi::c_void;
use core::mem::ManuallyDrop;
use core::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Input::{
    GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT,
    RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEMOUSE, RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DBT_DEVTYP_DEVICEINTERFACE, DEV_BROADCAST_DEVICEINTERFACE_W, DEVICE_NOTIFY_WINDOW_HANDLE,
    EVENT_OBJECT_FOCUS, EVENT_SYSTEM_DESKTOPSWITCH, EVENT_SYSTEM_FOREGROUND, GA_ROOT, GetAncestor,
    GetForegroundWindow, GetMessageTime, HDEVNOTIFY, KillTimer, PBT_APMRESUMEAUTOMATIC,
    RI_MOUSE_BUTTON_1_DOWN, RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_3_DOWN, RI_MOUSE_BUTTON_4_DOWN,
    RI_MOUSE_BUTTON_5_DOWN, RegisterDeviceNotificationW, SetTimer, UnregisterDeviceNotification,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP, WM_DEVICECHANGE, WM_INPUT,
    WM_INPUT_DEVICE_CHANGE, WM_POWERBROADCAST, WM_TIMER, WM_WTSSESSION_CHANGE,
};
use windows::core::{Error as WinError, GUID, PCWSTR, Result as WinResult};

use crate::buffer::{ResetOutcome, is_newer_than};

// ---------------------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------------------

/// The private message that tells the input thread a flush request is waiting for it.
///
/// `WM_APP + 6`: `WM_APP + 1` is the wake-up of [`crate::app`], `WM_APP + 2` the tray callback,
/// `WM_APP + 3` and `WM_APP + 4` belong to [`crate::hook`] and `WM_APP + 5` is the
/// configuration nudge of [`crate::app`].
///
/// SEC-05: it carries nothing. The timestamp of the event travels in [`PENDING_FLUSH`], an
/// atomic of this process no sender can reach, and the handler does nothing whatsoever when that
/// atomic is empty.
pub const WM_APP_FLUSH: u32 = WM_APP + 6;

/// The private message that asks the input thread to re-read the keyboard layout — the delivery
/// of FR-21.
///
/// `WM_APP + 7`, the next free number after [`WM_APP_FLUSH`]. Posted by [`win_event_proc`] when
/// the foreground window changes and by [`crate::app`] when the shell reports a language change.
///
/// SEC-05: it carries nothing either. The handler asks the *system* what the foreground window's
/// layout is; the sender cannot put a layout into it.
pub const WM_APP_LAYOUT: u32 = WM_APP + 7;

/// The private message that asks the input thread to put the hook back — FR-80.
///
/// `WM_APP + 9`, the next free number: `+ 1` is the wake-up of [`crate::app`], `+ 2` the tray
/// callback, `+ 3` and `+ 4` belong to [`crate::hook`], `+ 5` is the configuration nudge of
/// [`crate::app`], `+ 6` and `+ 7` are [`WM_APP_FLUSH`] and [`WM_APP_LAYOUT`], and **`+ 8` is
/// [`crate::switch::WM_APP_SWITCH`]** — which is why this is `+ 9` and not the number that
/// followed [`WM_APP_LAYOUT`] when this module was last touched. A test asserts the whole set is
/// distinct, because the two collide silently: both are private messages of this process and
/// nothing but the number tells them apart.
///
/// Posted by [`win_event_proc`] on `EVENT_SYSTEM_DESKTOPSWITCH`, and by the UI window on
/// `WM_WTSSESSION_CHANGE` and on `WM_POWERBROADCAST`/`PBT_APMRESUMEAUTOMATIC`. It exists because
/// none of those three places is the input thread, and FR-01 puts the hook on the input thread.
///
/// SEC-05: it carries nothing. The reason travels in [`PENDING_REASON`], an atomic of this
/// process no sender can reach, and the handler does nothing whatsoever when that atomic is
/// empty.
pub const WM_APP_REHOOK: u32 = WM_APP + 9;

/// The `WinEvent` events the flush table of FR-10 lists — "смена активного окна" and "смена
/// фокуса внутри окна".
///
/// Named as data rather than written into a condition so that a test can assert the list itself,
/// the same shape [`crate::layouts::REBUILD_MESSAGES`] uses for the messages of FR-21.
///
/// `EVENT_SYSTEM_DESKTOPSWITCH` is deliberately **absent from this list**, and that is a
/// statement about the flush table and not about the subscription: a switch to the secure
/// desktop is not one of the rows of FR-10, so it must not throw away what the user has typed.
/// It is subscribed to all the same — see [`WATCHED_EVENTS`] — because FR-80 needs it.
pub const FLUSH_EVENTS: [u32; 2] = [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_FOCUS];

/// Every `WinEvent` event this program subscribes to — the two of FR-10 and the one of FR-80.
///
/// **One list, one loop, one guard.** FR-80 asks for `EVENT_SYSTEM_DESKTOPSWITCH` and this is
/// how it is delivered: by lengthening the array [`watch`] already iterates over, not by
/// building a second subscription mechanism beside it. `WATCHED_EVENTS[..2]` is
/// [`FLUSH_EVENTS`], and [`win_event_proc`] tells the two purposes apart by the event code it is
/// handed.
///
/// Section 6.1 lists exactly these three under the watcher thread: "SetWinEventHook: FOREGROUND,
/// OBJECT_FOCUS, DESKTOPSWITCH".
pub const WATCHED_EVENTS: [u32; 3] = [
    EVENT_SYSTEM_FOREGROUND,
    EVENT_OBJECT_FOCUS,
    EVENT_SYSTEM_DESKTOPSWITCH,
];

/// The interval of the liveness timer of FR-80, in milliseconds — "с интервалом 30 с".
pub const LIVENESS_INTERVAL_MS: u32 = 30_000;

/// `nIDEvent` of the liveness timer. Window-scoped, so it only has to be unique among the timers
/// of the input window, and it is the only one there in a shipping build.
pub const LIVENESS_TIMER_ID: usize = 1;

/// HID usage page 1, "generic desktop controls" — the page both the mouse and the keyboard are
/// on.
const HID_USAGE_PAGE_GENERIC: u16 = 0x01;

/// HID usage 2 of the generic page: a mouse.
///
/// ⚠ **Usage 6, the keyboard, deliberately has no constant here any more.** Task T-08-4 measured
/// what a keyboard entry in this registration costs — the whole chain of low-level keyboard
/// hooks, while any window of this process is in front — and took it out. A constant left behind
/// would be an invitation to put the entry back; see the module documentation.
const HID_USAGE_MOUSE: u16 = 0x02;

/// `GUID_DEVINTERFACE_KEYBOARD`, `{884b96c3-56ef-11d1-bc8c-00a0c91405dd}` — the device interface
/// class [`register_device_notice`] filters on.
///
/// Written out rather than imported, and that is not a shortcut: the constant lives in
/// `windows::Win32::Devices::HumanInterfaceDevice`, behind a feature section 3.2 does not list,
/// and `Cargo.toml` is closed (SEC-03). What is written out is a **number of the operating
/// system's own ABI**, not a dependency — the same standing the raw event code `0x0020` has in
/// `tests\watchdog.rs`. That it is the right number is measured rather than trusted: the report
/// of task T-08-4 enumerates the present device interfaces of this class and finds exactly the
/// machine's keyboards, the same count the old `RIDEV_DEVNOTIFY` registration reported.
const GUID_DEVINTERFACE_KEYBOARD: GUID = GUID::from_u128(0x884b96c3_56ef_11d1_bc8c_00a0c91405dd);

/// The `usButtonFlags` bits that mean "a button went **down**" — the FR-10 row "нажатие любой
/// кнопки мыши".
///
/// Five buttons: left, right, middle and the two side buttons. The matching `_UP` bits are
/// deliberately not here — a release is not a press, and flushing on both would flush twice for
/// one click. The wheel bits (`RI_MOUSE_WHEEL`, `RI_MOUSE_HWHEEL`) are not here either:
/// scrolling is not a button and does not move the caret, and FR-10 says "нажатие любой
/// **кнопки**". The wheel pressed as a button is `RI_MOUSE_BUTTON_3_DOWN` and is covered.
const BUTTON_DOWN_BITS: u16 = (RI_MOUSE_BUTTON_1_DOWN
    | RI_MOUSE_BUTTON_2_DOWN
    | RI_MOUSE_BUTTON_3_DOWN
    | RI_MOUSE_BUTTON_4_DOWN
    | RI_MOUSE_BUTTON_5_DOWN) as u16;

// ---------------------------------------------------------------------------------------
// Pure decisions — everything here is testable without a window, a mouse or a hook
// ---------------------------------------------------------------------------------------

/// Whether a `WinEvent` event code is one of the two the FR-10 table flushes on.
pub fn is_flush_event(event: u32) -> bool {
    FLUSH_EVENTS.contains(&event)
}

/// Whether a `RAWMOUSE.usButtonFlags` word says a button went down — FR-10, FR-13.
///
/// **Cursor movement answers `false`**, which is the requirement rather than an optimisation:
/// a mouse that has only moved reports `usButtonFlags == 0`, and moving the mouse does not move
/// the caret, so it must leave the buffer alone. It is also what makes the flood of movement
/// packets `RIDEV_INPUTSINK` delivers cost two instructions each.
pub const fn mouse_button_pressed(button_flags: u16) -> bool {
    button_flags & BUTTON_DOWN_BITS != 0
}

/// What the delivery of FR-21 asks of the input thread for `message`, if anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rebuild {
    /// The keyboard layout may have changed. Re-read it and rebuild the cache **only if it
    /// really did** — the layout of the window the user just switched to is usually the one that
    /// was already active, and FR-20 is thousands of `ToUnicodeEx` calls.
    IfLayoutChanged,
    /// A keyboard arrived or left. Rebuild whatever the layout says: the scan-code map of the
    /// new device can differ while the `HKL` does not change at all.
    Unconditional,
}

/// The rebuild `message` asks for — the delivery half of FR-21.
///
/// [`WM_APP_LAYOUT`] stands in for the `WM_INPUTLANGCHANGE` of FR-21, which cannot reach this
/// program because it goes to the window with the keyboard focus — see the module documentation
/// of [`crate::app`] and the report of task T-03-3.
///
/// ⚠ `WM_INPUT_DEVICE_CHANGE` was the other stand-in, for `WM_DEVICECHANGE`, and **task T-08-4
/// retired it**: the registration that produced it is gone, and the real `WM_DEVICECHANGE`
/// arrives instead, through [`register_device_notice`], to be handled where FR-21's own messages
/// are handled — the [`crate::layouts::needs_rebuild`] arm of the window procedure. The arm below
/// stays because the answer it gives is still the right one for a message that says a keyboard
/// arrived, and because deleting it would be a claim that no such message can ever arrive, which
/// is a claim about the whole system rather than about this program.
///
/// **The two lists must not overlap.** `WM_DEVICECHANGE` is deliberately *not* an arm here: the
/// window procedure asks [`crate::layouts::needs_rebuild`] first and this function second, so a
/// message in both would be rebuilt twice. A test asserts the disjointness.
///
/// The two arms are counted, so that a run can say **which** of the two delivery paths produced a
/// rebuild rather than only that the number of rebuilds went up. Counting here rather than at the
/// call site is what keeps the two lists — the messages and the counters — from drifting apart.
pub fn rebuild_for(message: u32) -> Option<Rebuild> {
    match message {
        WM_INPUT_DEVICE_CHANGE => {
            DEVICE_CHANGES.fetch_add(1, Ordering::Relaxed);
            Some(Rebuild::Unconditional)
        }
        WM_APP_LAYOUT => {
            LAYOUT_PROBES.fetch_add(1, Ordering::Relaxed);
            Some(Rebuild::IfLayoutChanged)
        }
        _ => None,
    }
}

/// Bit that distinguishes "a flush is pending, with timestamp zero" from "nothing is pending" in
/// [`PENDING_FLUSH`].
///
/// A timestamp of exactly zero is a real value the tick counter takes once every 49 days, so
/// zero cannot double as the empty state. The cell is 64 bits and the flag lives above the
/// timestamp.
const PENDING_MARK: u64 = 1 << 32;

/// The value the pending-flush cell should take to also cover an event stamped `time`, or `None`
/// when the cell already covers it.
///
/// Coalescing is not an optimisation, it is the correct answer: a flush with timestamp `T`
/// removes every stroke made at or before `T`, so of two pending flushes the **newer** one does
/// everything the older one would have done and more. Two events that arrive between two turns
/// of the input thread's message loop therefore collapse into one without losing anything.
///
/// "Newer" is [`is_newer_than`] and not `>`, for the reason given there: `u64::max` on the raw
/// cell would pick the wrong one of two timestamps that straddle the wrap of the 32-bit counter.
pub const fn coalesced(cell: u64, time: u32) -> Option<u64> {
    let wanted = PENDING_MARK | time as u64;

    match pending_time(cell) {
        Some(pending) if !is_newer_than(time, pending) => None,
        _ => Some(wanted),
    }
}

/// The timestamp a pending-flush cell carries, or `None` when it is empty.
pub const fn pending_time(cell: u64) -> Option<u32> {
    if cell & PENDING_MARK == 0 {
        return None;
    }

    Some(cell as u32)
}

// ---------------------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------------------

/// The flush request waiting for the input thread — `0` when there is none, otherwise
/// [`PENDING_MARK`] with the timestamp of the newest event in the low half.
///
/// An atomic and not a queue, because coalescing to the newest timestamp is lossless (see
/// [`coalesced`]) and because the producer is a `WinEvent` callback, where NFR-04 allows nothing
/// that can block. One compare-and-swap loop that in practice never spins twice.
static PENDING_FLUSH: AtomicU64 = AtomicU64::new(0);

/// `WM_INPUT` packets from a mouse that the input thread looked at.
///
/// Mostly cursor movement. Published so that "перемещение курсора сбросом не является" can be
/// *shown* on a live run — thousands of packets against zero flushes — rather than asserted.
static MOUSE_PACKETS: AtomicU32 = AtomicU32::new(0);

/// `WM_INPUT` packets that were a button going down, that is, flushes of FR-10.
static MOUSE_FLUSHES: AtomicU32 = AtomicU32::new(0);

/// `WinEvent` flush requests raised by [`win_event_proc`].
static WINDOW_FLUSHES: AtomicU32 = AtomicU32::new(0);

/// `WinEvent` flush events **ignored** because they did not concern the user's foreground —
/// task T-10-0, decision Р-60.
///
/// The observable half of the repair: the storm this counter absorbs used to be
/// [`WINDOW_FLUSHES`], 288 of them in the seven minutes of the acceptance run, and every one
/// of them threw away what the user had typed. A test that stages the storm from a foreign
/// process reads this number as its **positive control** — growth here is the proof that the
/// storm reached the product and was turned away, where a green run with both counters flat
/// would prove only that nothing was delivered at all.
static BACKGROUND_SKIPS: AtomicU32 = AtomicU32::new(0);

/// The window of the last `EVENT_OBJECT_FOCUS` that concerned the user's foreground — the
/// memory of [`focus_repeated`], task **T-10-0e**.
///
/// The handle is stored **as a value** (`HWND` is pointer-sized; an `AtomicIsize` holds it)
/// and is never dereferenced: the only thing ever done with it is an equality comparison
/// against the next event's handle. Zero — no focus event seen yet — equals no live handle,
/// so the first event always differs and flushes.
///
/// Deliberately written only by focus events that **passed the gate** of
/// [`concerns_the_foreground`]: the churn of a background window says nothing about where
/// the user's input goes, so it must not overwrite the memory of where it actually goes.
/// `EVENT_SYSTEM_FOREGROUND` does not write it either — a window change always flushes
/// (it is not subject to the deduplication), and the focus event that follows it carries
/// the new target and updates the memory itself.
static LAST_FOCUS_TARGET: AtomicIsize = AtomicIsize::new(0);

/// `EVENT_OBJECT_FOCUS` events of the foreground window skipped because their window was
/// already the focus target — task **T-10-0e**.
///
/// The observable half of that repair, the same standing [`BACKGROUND_SKIPS`] has for task
/// T-10-0: a test that stages the same-hwnd churn from a **foreign** process (the
/// `WINEVENT_SKIPOWNPROCESS` trap again) reads growth here as its positive control — the
/// churn was delivered and turned away — while `window_flushes` standing still beside it is
/// the repair itself.
static FOCUS_REPEATS: AtomicU32 = AtomicU32::new(0);

/// Flush requests the input thread actually took out of [`PENDING_FLUSH`].
///
/// Lower than [`WINDOW_FLUSHES`] whenever two events coalesced, which is exactly what the
/// difference between the two numbers means.
static WINDOW_FLUSHES_TAKEN: AtomicU32 = AtomicU32::new(0);

/// Flushes that emptied the whole buffer — [`ResetOutcome::Cleared`].
static FULL_CLEARS: AtomicU32 = AtomicU32::new(0);

/// Flushes that removed some strokes and left others — [`ResetOutcome::Partial`], the arm FR-12
/// exists for.
static PARTIAL_CLEARS: AtomicU32 = AtomicU32::new(0);

/// Flushes that removed nothing because every stroke was newer than the event —
/// [`ResetOutcome::Kept`].
static KEPT_EVENTS: AtomicU32 = AtomicU32::new(0);

/// Strokes removed by all flushes together.
static STROKES_REMOVED: AtomicU32 = AtomicU32::new(0);

/// Device changes the input thread answered — the `WM_DEVICECHANGE` half of the FR-21 delivery.
///
/// Since task T-08-4 that means the real `WM_DEVICECHANGE` of FR-21, counted by
/// [`note_device_change`] and delivered by [`register_device_notice`]. The
/// `WM_INPUT_DEVICE_CHANGE` arm of [`rebuild_for`] still adds to the same number: it is the same
/// event under the name the old delivery gave it, and no registration of this program can produce
/// it any more, so what it now counts is a forged message and nothing else.
static DEVICE_CHANGES: AtomicU32 = AtomicU32::new(0);

/// [`WM_APP_LAYOUT`] messages the input thread answered — the `WM_INPUTLANGCHANGE` half of the
/// FR-21 delivery. Higher than the number of rebuilds they caused, because most of them find the
/// layout unchanged.
static LAYOUT_PROBES: AtomicU32 = AtomicU32::new(0);

/// `UnregisterDeviceNotification` refusals — NFR-13, task T-08-4.
///
/// The record of a failure this module cannot journal by name; `DeviceNotice::drop` says why at
/// length. Zero on every healthy run, and nothing in the program reads it as a decision.
static DEVICE_NOTICE_FAILURES: AtomicU32 = AtomicU32::new(0);

/// Counts of what the subscriptions of this module have done — SEC-07 allows counts and nothing
/// else, and these are counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Mouse packets seen over Raw Input, movement included.
    pub mouse_packets: u32,
    /// Mouse packets that were a button press and therefore a flush — FR-10.
    pub mouse_flushes: u32,
    /// Flush requests raised by the two `WinEvent` subscriptions.
    pub window_flushes: u32,
    /// Flush requests the input thread took; the shortfall against `window_flushes` is what
    /// coalescing saved.
    pub window_flushes_taken: u32,
    /// Flushes that emptied the buffer completely.
    pub full_clears: u32,
    /// Flushes that removed only the strokes older than the event — FR-12.
    pub partial_clears: u32,
    /// Flushes that removed nothing because the buffer was newer than the event — FR-12.
    pub kept_events: u32,
    /// Strokes removed by all flushes together.
    pub strokes_removed: u32,
    /// Keyboard arrivals and removals the input thread answered — FR-21 delivery. A count of
    /// events, never a device name (SEC-01, SEC-07).
    pub device_changes: u32,
    /// Layout probes the input thread answered — FR-21 delivery.
    pub layout_probes: u32,
    /// `UnregisterDeviceNotification` refusals — NFR-13, task T-08-4. See
    /// [`DEVICE_NOTICE_FAILURES`] and the `Drop` of [`DeviceNotice`] for why this failure is a
    /// count rather than a line in the journal.
    pub device_notice_failures: u32,
    /// Flush events ignored as background noise — task T-10-0, decision Р-60. See
    /// [`BACKGROUND_SKIPS`]: growth here under a staged storm is the positive control that the
    /// storm was delivered and turned away rather than never seen.
    pub background_skips: u32,
    /// Foreground focus events skipped because their window was already the focus target —
    /// task T-10-0e. See [`FOCUS_REPEATS`]: growth here under a staged same-hwnd churn is
    /// the positive control that the churn was delivered and turned away.
    pub focus_repeats: u32,
}

/// What the subscriptions of this module have done so far.
pub fn counters() -> Counters {
    Counters {
        mouse_packets: MOUSE_PACKETS.load(Ordering::Relaxed),
        mouse_flushes: MOUSE_FLUSHES.load(Ordering::Relaxed),
        window_flushes: WINDOW_FLUSHES.load(Ordering::Relaxed),
        window_flushes_taken: WINDOW_FLUSHES_TAKEN.load(Ordering::Relaxed),
        full_clears: FULL_CLEARS.load(Ordering::Relaxed),
        partial_clears: PARTIAL_CLEARS.load(Ordering::Relaxed),
        kept_events: KEPT_EVENTS.load(Ordering::Relaxed),
        strokes_removed: STROKES_REMOVED.load(Ordering::Relaxed),
        device_changes: DEVICE_CHANGES.load(Ordering::Relaxed),
        layout_probes: LAYOUT_PROBES.load(Ordering::Relaxed),
        device_notice_failures: DEVICE_NOTICE_FAILURES.load(Ordering::Relaxed),
        background_skips: BACKGROUND_SKIPS.load(Ordering::Relaxed),
        focus_repeats: FOCUS_REPEATS.load(Ordering::Relaxed),
    }
}

// ---------------------------------------------------------------------------------------
// FR-13 — Raw Input on the input thread
// ---------------------------------------------------------------------------------------

/// The Raw Input registration of FR-13, removed when this value is dropped.
///
/// A registration belongs to the *process*, not to the window, which is why it is undone
/// explicitly: leaving it in place while the window it names is destroyed would leave the system
/// posting `WM_INPUT` to a handle that no longer exists.
///
/// ⚠ "Belongs to the process" is not a detail of the cleanup. It is also why task T-08-4 could
/// not fix the keyboard starvation by moving the registration to another window of ours: there
/// is no window it could have been moved to. See the module documentation.
pub struct RawInput {
    /// Kept so that the removal names the same devices the registration did.
    devices: [RAWINPUTDEVICE; 1],
}

/// Registers for the raw mouse input of FR-13, delivered to `target` — FR-13, and nothing else.
///
/// # The one entry
///
/// **mouse, `RIDEV_INPUTSINK`.** The flag is what FR-13 names, and what it means is "deliver
/// this even when the target window is not in the foreground" — which is the only mode of any
/// use to a program whose windows are hidden and normally not focused. `WM_INPUT` then arrives
/// for every mouse packet, movement included; [`mouse_button_pressed`] is what separates the two.
///
/// # ⚠ The keyboard entry that used to be here, and why it is gone — task T-08-4
///
/// Tasks T-03-3 through T-06-2 registered a second entry, `HID_USAGE_KEYBOARD` with
/// `RIDEV_DEVNOTIFY`, to be told when a keyboard arrives or leaves. The comment beside it said
/// that without `RIDEV_INPUTSINK` the raw keystrokes go only to the window with the keyboard
/// focus, "which ours never is", so the entry cost "exactly zero messages while the user types".
///
/// Both halves were false, and the second one catastrophically. Our windows **do** hold the
/// foreground — the settings dialog of FR-92 and the tray menu of FR-91 — and in that state a
/// keyboard entry in this registration takes the keystrokes away from **every low-level keyboard
/// hook in the session**, this program's and every other program's alike. The user pressed
/// `Ctrl+Alt+Shift+F12` with the settings dialog open and nothing happened: FR-96, the only way
/// out of a wedged hook, was unreachable exactly when a dialog of ours was in front.
///
/// The measurement is in the report of task T-08-4 and it is worth one line here: **the flag did
/// not matter and the window did not matter.** `RIDEV_INPUTSINK` alone starves the chain just as
/// `RIDEV_DEVNOTIFY` does, and so does a registration whose `hwndTarget` is the message-only
/// window that can never be in the foreground. Being a raw keyboard client is what does it.
///
/// FR-21 keeps its news by another road — [`register_device_notice`].
///
/// The entry that remains does not set `RIDEV_NOLEGACY`, so ordinary mouse messages continue to
/// reach every application, this one included, exactly as before. Registration is per-process
/// and changes nothing for anybody else.
pub fn register_raw_input(target: HWND) -> WinResult<RawInput> {
    let devices = [RAWINPUTDEVICE {
        usUsagePage: HID_USAGE_PAGE_GENERIC,
        usUsage: HID_USAGE_MOUSE,
        dwFlags: RIDEV_INPUTSINK,
        hwndTarget: target,
    }];

    // SAFETY: `devices` is a fully initialised array owned by this frame for the whole call, and
    // the call only reads through it — the registration the system keeps is a copy. `target` is
    // the caller's live window, which `RIDEV_INPUTSINK` requires to be non-null, and the guard
    // returned here is dropped before that window is destroyed because `app::serve_window`
    // declares it after the window. `size_of::<RAWINPUTDEVICE>()` is the element size the call
    // demands and is computed rather than written out.
    unsafe {
        RegisterRawInputDevices(
            &devices,
            u32::try_from(size_of::<RAWINPUTDEVICE>()).unwrap_or(0),
        )?;
    }

    Ok(RawInput { devices })
}

impl Drop for RawInput {
    fn drop(&mut self) {
        let removals = self.devices.map(|device| RAWINPUTDEVICE {
            dwFlags: RIDEV_REMOVE,
            // Documented requirement of `RIDEV_REMOVE`: the target must be null, or the call
            // fails with ERROR_INVALID_PARAMETER.
            hwndTarget: HWND::default(),
            ..device
        });

        // SAFETY: same argument as the registration — a live array of this frame, read only.
        // The usage page and usage pairs are the ones that were registered, which is what
        // identifies the entries to remove; they are copied out of `self` rather than written
        // again so the two lists cannot drift apart.
        let removed = unsafe {
            RegisterRawInputDevices(
                &removals,
                u32::try_from(size_of::<RAWINPUTDEVICE>()).unwrap_or(0),
            )
        };

        if let Err(error) = removed {
            // NFR-13: examined, not discarded. A failure here is benign — the process is on its
            // way out and the registration dies with it.
            crate::app::report_non_critical("RegisterRawInputDevices(RIDEV_REMOVE)", &error);
        }
    }
}

// ---------------------------------------------------------------------------------------
// FR-21, the device half — task T-08-4
// ---------------------------------------------------------------------------------------

/// The keyboard device notification of FR-21, withdrawn when this value is dropped.
///
/// The registration is a resource the system holds **against the window**, exactly as
/// [`SessionNotice`] is, which is why the withdrawal is explicit rather than left to the
/// window's destruction.
pub struct DeviceNotice {
    /// The handle `RegisterDeviceNotificationW` returned, which is what withdraws it.
    handle: HDEVNOTIFY,
}

/// Asks for `WM_DEVICECHANGE` at `window` whenever a keyboard arrives or leaves — the device
/// half of the FR-21 delivery, and the replacement for the keyboard entry task T-08-4 took out
/// of [`register_raw_input`].
///
/// # Why this and not `RIDEV_DEVNOTIFY`
///
/// Because `RIDEV_DEVNOTIFY` had to be paid for with a keyboard entry in the raw input
/// registration, and that entry cost this program — and every other program in the session —
/// the whole chain of low-level keyboard hooks whenever a window of ours was in front. The
/// module documentation has the measurement. This call is not a raw input registration at all
/// and costs nothing of the sort.
///
/// # ⚠ `window` must be the **input** thread's window
///
/// Not for the reason [`register_session_notice`] has, which is the opposite one. A *broadcast*
/// device notification reaches top-level windows only, but a **registered** one is posted to the
/// handle named here whatever kind of window it is — measured on a message-only window in the
/// report of task T-08-4 — and the rebuild it asks for is FR-20's sweep over the layout cache,
/// which is a thread-local of the input thread (section 6.3). Naming any other window would put
/// the message on a thread that cannot act on it.
///
/// # The filter, and what it is worth
///
/// `DBT_DEVTYP_DEVICEINTERFACE` with [`GUID_DEVINTERFACE_KEYBOARD`] and not
/// `DEVICE_NOTIFY_ALL_INTERFACE_CLASSES`: FR-21 is about keyboards, and the unfiltered
/// registration would run the sweep of FR-20 — thousands of `ToUnicodeEx` calls — every time
/// anything at all appeared in the machine's device tree. That is not a guess either: the
/// measurement mounted a virtual CD-ROM and watched the filtered registration stay silent while
/// the unfiltered one took three arrivals.
///
/// # NFR-13
///
/// `RegisterDeviceNotificationW` reports failure with a null handle, which the `windows` crate
/// turns into `Err`, and it is propagated: a program that silently stopped rebuilding its cache
/// when the user plugged in a keyboard is the failure FR-21 exists to prevent. `app::serve_window`
/// makes that fatal to the thread, which is the same standing the raw input registration has.
pub fn register_device_notice(window: HWND) -> WinResult<DeviceNotice> {
    let filter = DEV_BROADCAST_DEVICEINTERFACE_W {
        // The documented requirement: the size of the whole structure, not of the header.
        dbcc_size: u32::try_from(size_of::<DEV_BROADCAST_DEVICEINTERFACE_W>()).unwrap_or(0),
        dbcc_devicetype: DBT_DEVTYP_DEVICEINTERFACE.0,
        dbcc_reserved: 0,
        dbcc_classguid: GUID_DEVINTERFACE_KEYBOARD,
        // The name is an output field of the notifications, never an input to the filter; the
        // structure carries one element of it and the system writes past it into the buffer it
        // hands the recipient, not into this one.
        dbcc_name: [0],
    };

    // SAFETY: `filter` is a fully initialised structure owned by this frame for the whole call,
    // and the call only reads through the pointer — what the system keeps is a copy, which is
    // why it may live on the stack. `dbcc_size` says how far the read may go and is computed
    // from the type rather than written out. `window` is the caller's live window, and the guard
    // returned here is dropped before that window is destroyed because `app::serve_window`
    // declares it after the window. `DEVICE_NOTIFY_WINDOW_HANDLE` is what makes the first
    // argument a window handle rather than a service handle, which is the only other thing it
    // could be.
    let handle = unsafe {
        RegisterDeviceNotificationW(
            HANDLE(window.0),
            (&raw const filter).cast::<c_void>(),
            DEVICE_NOTIFY_WINDOW_HANDLE,
        )
    }?;

    Ok(DeviceNotice { handle })
}

impl Drop for DeviceNotice {
    fn drop(&mut self) {
        // SAFETY: `handle` came from a successful `RegisterDeviceNotificationW` and is withdrawn
        // exactly once, because this type is neither `Copy` nor `Clone`. The window it names is
        // still alive: `app::serve_window` declares this guard after the window and drop order
        // takes it first.
        let withdrawn = unsafe { UnregisterDeviceNotification(self.handle) };

        if let Err(error) = withdrawn {
            // **NFR-13: examined, counted and journalled.** The repair the previous version of
            // this comment described for whoever opened `src\diag.rs` next was made by task
            // T-09-1: `("UnregisterDeviceNotification", Kind::Hook)` is a row of `OPERATIONS`,
            // so the name survives `Operation::from_name` and this block is the ordinary
            // `report_non_critical` every neighbour of it already is.
            //
            // **Both records are kept, and that is the idiom of this module rather than
            // redundancy.** `reinstall_hook` below does the same thing for the same reason: the
            // journal is a ring of the last few hundred events and says *when* against
            // everything else that happened, while the counter is a running total the snapshot
            // of `Counters::device_notice_failures` reports, and one does not replace the
            // other. Two accepted tests read the counter — `tests\watchdog.rs` and
            // `tests\control.rs`.
            //
            // The failure itself stays non-critical, and what it can cost is small and bounded:
            // the call can only fail on a handle that is not a live registration, this one came
            // from a successful `RegisterDeviceNotificationW` and is withdrawn once, and the
            // process is on its way out with the window about to be destroyed under it either
            // way.
            DEVICE_NOTICE_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::app::report_non_critical("UnregisterDeviceNotification", &error);
        }
    }
}

/// Counts a `WM_DEVICECHANGE` that the input thread answered — the device half of the FR-21
/// delivery, seen from the outside through [`Counters::device_changes`].
///
/// Called by the window procedure of [`crate::app`] from the arm that
/// [`crate::layouts::needs_rebuild`] guards, which is where the rebuild itself happens: FR-21
/// names `WM_DEVICECHANGE` and `layouts` owns that list, so the message is not this module's to
/// handle — only to count, because this module is the one that asked for it.
///
/// Answers whether it counted anything, so that the call site reads as a question and a test can
/// drive it without a window, a keyboard or a registration.
///
/// **SEC-01, SEC-07.** A count and nothing else. The `WM_DEVICECHANGE` this counts carries a
/// device name in its `lparam`, and that name is never read, never stored and never journalled:
/// the only thing this program wants from the message is the fact that it happened.
pub fn note_device_change(message: u32) -> bool {
    if message != WM_DEVICECHANGE {
        return false;
    }

    DEVICE_CHANGES.fetch_add(1, Ordering::Relaxed);
    true
}

// ---------------------------------------------------------------------------------------
// The flush path — FR-10, FR-12, FR-13
// ---------------------------------------------------------------------------------------

/// Applies the flush `message` carries, if it carries one — the asynchronous rows of FR-10,
/// resolved by FR-12.
///
/// Called from the window procedure of [`crate::app`], on every message of every window, and it
/// answers `None` for all but two of them. The first thing it does is establish that this is the
/// input thread, by asking whether this thread owns the typing buffer (section 6.3 gives it to
/// that thread and to no other): a `WM_INPUT` posted at the UI window by another process must
/// not be read, and a forged [`WM_APP_FLUSH`] must not be able to consume a pending request the
/// input thread has not seen yet.
pub fn apply_flush(message: u32, lparam: LPARAM) -> Option<ResetOutcome> {
    if !crate::buffer::is_installed() {
        return None;
    }

    let event_time = flush_time(message, lparam)?;
    let outcome = crate::buffer::reset_up_to(event_time)?;

    match outcome {
        ResetOutcome::Cleared { removed } => {
            FULL_CLEARS.fetch_add(1, Ordering::Relaxed);
            note_removed(removed);
        }
        ResetOutcome::Partial { removed, .. } => {
            PARTIAL_CLEARS.fetch_add(1, Ordering::Relaxed);
            note_removed(removed);
        }
        ResetOutcome::Kept { .. } => {
            KEPT_EVENTS.fetch_add(1, Ordering::Relaxed);
        }
    }

    Some(outcome)
}

/// Adds `removed` to the running total of strokes taken out by flushes.
///
/// A count of strokes and never a stroke — SEC-07. The saturation is not a real case at a ring
/// of at most [`crate::buffer::MAX_CAPACITY`] slots; it is there because a wrapping counter in a
/// number a report prints would be worse than one that stops.
fn note_removed(removed: usize) {
    STROKES_REMOVED.fetch_add(
        u32::try_from(removed).unwrap_or(u32::MAX),
        Ordering::Relaxed,
    );
}

/// The timestamp of the flush `message` carries, or `None` when it carries none.
fn flush_time(message: u32, lparam: LPARAM) -> Option<u32> {
    match message {
        WM_INPUT => mouse_button_time(lparam),
        WM_APP_FLUSH => take_pending_flush(),
        _ => None,
    }
}

/// The timestamp of a mouse **button press** in a `WM_INPUT`, or `None` for anything else —
/// FR-13, FR-12.
///
/// # Where the timestamp comes from
///
/// FR-12 names "`RAWINPUT`" as the source of the mouse event's timestamp. The `RAWINPUT`
/// structure has no timestamp field — its header carries the type, the size, the device handle
/// and the `wParam` of the message, and `RAWMOUSE` carries flags, buttons and displacements —
/// so the requirement's parenthesis cannot be taken literally; see the report of task T-03-3.
/// What it is asking for is available and is exactly one call away: `GetMessageTime` returns the
/// tick count at which the message being handled was posted, and that is the same counter, in
/// the same unit, that `KBDLLHOOKSTRUCT.time` and `dwmsEventTime` are taken from.
fn mouse_button_time(lparam: LPARAM) -> Option<u32> {
    let mut raw = RAWINPUT::default();
    let mut size = u32::try_from(size_of::<RAWINPUT>()).unwrap_or(0);

    // SAFETY: `lparam` of a `WM_INPUT` is documented to be the `HRAWINPUT` of the packet; the
    // conversion is the reverse of the one the system performed and is exact. `raw` is a live,
    // properly aligned `RAWINPUT` owned by this frame, and `size` says how many bytes may be
    // written into it, so the call cannot overrun it — a packet larger than that is refused by
    // the call rather than truncated into our stack. The header size is computed from the type.
    // A forged message from another process (SEC-05) gets no further than this call: an
    // `HRAWINPUT` that is not a live packet of this process is rejected, and the rejection is
    // checked below.
    let written = unsafe {
        GetRawInputData(
            HRAWINPUT(lparam.0 as *mut c_void),
            RID_INPUT,
            Some((&raw mut raw).cast::<c_void>()),
            &mut size,
            u32::try_from(size_of::<RAWINPUTHEADER>()).unwrap_or(0),
        )
    };

    // NFR-13. `GetRawInputData` reports failure with `(UINT)-1`, and a short read cannot be a
    // packet at all; both are answered by leaving the buffer alone. Nothing is journalled: a
    // forged message is not an event worth a line, and the counters below are the record.
    if written == u32::MAX || (written as usize) < size_of::<RAWINPUTHEADER>() {
        return None;
    }

    if raw.header.dwType != RIM_TYPEMOUSE.0 {
        // The keyboard entry of the registration asks for device notifications only, so this
        // should not happen; it is checked because the union below is only meaningful for a
        // mouse packet.
        return None;
    }

    MOUSE_PACKETS.fetch_add(1, Ordering::Relaxed);

    // SAFETY: `raw.data` is a union whose active member is decided by `raw.header.dwType`, and
    // that field was just established to be `RIM_TYPEMOUSE`; `RAWMOUSE::Anonymous` is a union of
    // one `u32` and a pair of `u16` occupying the same four bytes, so both members are valid for
    // every bit pattern and reading either is defined whatever the system wrote. The whole
    // structure was filled by the call above, which reported how many bytes it wrote.
    let button_flags = unsafe { raw.data.mouse.Anonymous.Anonymous.usButtonFlags };

    if !mouse_button_pressed(button_flags) {
        // FR-10 and FR-13: cursor movement is **not** a flush. This is the arm the overwhelming
        // majority of packets take, and it costs one mask and one comparison.
        return None;
    }

    MOUSE_FLUSHES.fetch_add(1, Ordering::Relaxed);

    // SAFETY: `GetMessageTime` takes no arguments, touches no memory of ours and returns the
    // tick count of the message this thread is currently handling — which is the `WM_INPUT`
    // whose `lparam` was just read, because a window procedure runs inside the dispatch of that
    // very message. It has no failure value, so NFR-13 has nothing to check. The `as u32` is the
    // documented reinterpretation of a tick count that Windows hands back as a signed `LONG`.
    Some(unsafe { GetMessageTime() } as u32)
}

/// Records that a flush event stamped `time` has happened, for the input thread to apply.
///
/// The producer half of [`PENDING_FLUSH`], and the whole body of [`win_event_proc`] besides the
/// posts. It is a compare-and-swap loop and not a lock because it is called from a callback the
/// system drives, where NFR-04 forbids anything that can block; the loop retries only if another
/// thread wrote between the load and the exchange, which takes two `WinEvent` callbacks in
/// flight at once.
///
/// Public because it is the funnel the remaining asynchronous rows of the FR-10 table will use —
/// `EVENT_SYSTEM_DESKTOPSWITCH` and `WM_WTSSESSION_CHANGE`, task T-06-2 — and because it is the
/// half of the path that can be driven from a test without a mouse and without a window.
pub fn request_flush(time: u32) {
    let mut cell = PENDING_FLUSH.load(Ordering::Acquire);

    while let Some(wanted) = coalesced(cell, time) {
        match PENDING_FLUSH.compare_exchange_weak(cell, wanted, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => break,
            Err(seen) => cell = seen,
        }
    }

    WINDOW_FLUSHES.fetch_add(1, Ordering::Relaxed);
}

/// Takes the pending flush request, leaving the cell empty.
fn take_pending_flush() -> Option<u32> {
    let taken = pending_time(PENDING_FLUSH.swap(0, Ordering::AcqRel))?;

    WINDOW_FLUSHES_TAKEN.fetch_add(1, Ordering::Relaxed);

    Some(taken)
}

// ---------------------------------------------------------------------------------------
// The `WinEvent` subscriptions of section 6.1 — FR-10
// ---------------------------------------------------------------------------------------

/// The `WinEvent` subscriptions of [`WATCHED_EVENTS`], removed when this value is dropped.
///
/// The value never leaves the thread that created it, which is what makes the `Drop` correct:
/// an out-of-context hook is served by the installing thread's message queue and is unhooked
/// from that same thread.
pub struct Watching {
    hooks: [HWINEVENTHOOK; WATCHED_EVENTS.len()],
}

/// Subscribes to the `WinEvent` events of [`WATCHED_EVENTS`] on the calling thread.
///
/// Called by the **watcher** thread and by no other: section 6.1 puts `SetWinEventHook` there,
/// and an out-of-context hook calls back on the thread that installed it, so where it is
/// installed is where the callback runs.
///
/// # The flags
///
/// * `WINEVENT_OUTOFCONTEXT` — the callback lives in this process and the system posts the event
///   to this thread's queue. The alternative, an in-context hook, would inject a DLL of ours into
///   **every process in the session** and run our code inside them, which SEC-04 and the whole
///   posture of section 9 rule out;
/// * `WINEVENT_SKIPOWNPROCESS` — events generated by this program itself are not delivered.
///   Without it the tray menu opening would look like a foreground change and would flush the
///   buffer the user is about to convert.
///
/// A separate hook per event, rather than one hook over the range between them: the codes are
/// `0x0003`, `0x8005` and `0x0020`, and a single subscription spanning that range would ask the
/// system to deliver every accessibility event there is, for every process in the session, so
/// that we could discard all but three of them. Task T-06-2 lengthened the array by one entry
/// and changed nothing else here — the loop, the flags, the failure path and the guard are the
/// ones task T-03-3 wrote.
pub fn watch() -> WinResult<Watching> {
    let mut hooks = [HWINEVENTHOOK::default(); WATCHED_EVENTS.len()];

    for (slot, event) in hooks.iter_mut().zip(WATCHED_EVENTS) {
        // SAFETY: `None` for the module handle is what `WINEVENT_OUTOFCONTEXT` requires and
        // means no DLL is loaded anywhere; the callback is a `'static` function of this image,
        // which cannot be unloaded while the process runs. The two zeroes ask for every process
        // and every thread, which is what a global subscription is. The call returns a handle by
        // value and writes nothing through a pointer of ours.
        let hook = unsafe {
            SetWinEventHook(
                event,
                event,
                None,
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };

        // NFR-13: `SetWinEventHook` reports failure with a null handle. The hooks already
        // installed are unhooked by the `Drop` of the partially filled guard below.
        if hook.0.is_null() {
            let failure = WinError::from_thread();
            drop(Watching { hooks });
            return Err(failure);
        }

        *slot = hook;
    }

    Ok(Watching { hooks })
}

impl Drop for Watching {
    fn drop(&mut self) {
        for hook in self.hooks {
            if hook.0.is_null() {
                // A slot that was never filled — the failure path of `watch`.
                continue;
            }

            // SAFETY: `hook` came from a successful `SetWinEventHook` and is unhooked exactly
            // once, because this type is neither `Copy` nor `Clone` and the array is consumed by
            // value here. The call runs on the thread that installed the hook: the guard is
            // created inside `app::serve_window` and never leaves that frame.
            let unhooked = unsafe { UnhookWinEvent(hook) };

            if !unhooked.as_bool() {
                // NFR-13: examined. Nothing can be done about it, and the system removes the
                // subscription when this thread ends in any case.
                crate::app::report_non_critical("UnhookWinEvent", &WinError::from_thread());
            }
        }
    }
}

/// Whether a flush event's window is part of the user's actual input context — the user's
/// foreground window — rather than the internal churn of some background process. Task
/// **T-10-0**, decision **Р-60**.
///
/// # Why this exists — the defect that broke the acceptance session
///
/// The `EVENT_OBJECT_FOCUS` subscription hears the whole session, and idle background
/// Electron windows raise ~0.8 focus events a second without being touched (measured by two
/// independent instruments — `ПРИЁМКА.md`). A human types for 2–3 seconds before pressing the
/// hotkey, so a background event almost always landed inside that window and threw the typing
/// away: `strokes_removed=18` of 18 over seven minutes, all nine hotkey presses on an empty
/// buffer. Р-60 fixes the reading of FR-10: "смена активного окна и смена фокуса" is a change
/// **at the user**, not the private business of any process in the session.
///
/// # The predicate, and why it is this one — measured, not assumed
///
/// The root of the event's window (`GetAncestor(GA_ROOT)`) is compared with
/// `GetForegroundWindow()` at callback time. On 297 recorded events (task report, section 2)
/// the predicate separated without a single error:
///
/// * background churn while another window held the foreground — root differs, **0 of 253**
///   passed;
/// * real foreground changes and the focus events that follow them — **10 of 10** passed: an
///   out-of-context callback runs 0–32 ms after the event, by which time the foreground has
///   already moved, so the event's root and the answer agree;
/// * churn of the window that itself **is** the foreground — passes, and must: that is the
///   FR-10 row «смена фокуса внутри окна», and Р-60 keeps it flushing.
///
/// The alternative reading of the hypothesis — compare the event's **process** with the
/// foreground window's process — measured equivalent on the same data but coarser: a
/// background window of the foreground process (a second editor window of the same
/// application) would pass it wrongly, and it costs one Win32 call more.
///
/// # What a skipped event cannot lose
///
/// A transitional event whose window is no longer (or not yet) the foreground is skipped, and
/// that is lossless the same way coalescing is: the change that completes raises its own event
/// with a **newer** timestamp, and a flush stamped later removes everything the skipped one
/// would have removed and more (`reset_up_to`). A null foreground — the moment between two
/// windows, or a secure desktop — answers `false` for the same reason: the gain that follows
/// carries the flush.
///
/// # Cost — NFR-01…NFR-05, NFR-10
///
/// Two read-only `user32` queries. `GetForegroundWindow` reads one pointer out of the session's
/// window-manager state; `GetAncestor(GA_ROOT)` walks the parent chain in the same shared
/// state. Neither enters another process, neither takes a lock an application could hold,
/// neither can be made to wait — the same standing `GetMessageTime` already has on the
/// `WM_INPUT` path. No allocation, no I/O, nothing that can panic.
///
/// # NFR-14
///
/// Both calls report failure with a null handle, and both nulls are examined by the one
/// comparison: a null root never equals a live foreground, and a null foreground is refused
/// outright before the second call is made.
///
/// Public because the verdict is the whole of what task T-10-0 changed, and the tests of
/// `tests\watchdog.rs` drive it with real windows — their own, and the foreground's.
pub fn concerns_the_foreground(window: HWND) -> bool {
    // SAFETY: takes no arguments and touches no memory of ours; returns the current foreground
    // window by value, or null when there is none — which the check below treats as "not the
    // user's context", the conservative answer for a moment between two windows.
    let foreground = unsafe { GetForegroundWindow() };

    if foreground.0.is_null() {
        return false;
    }

    // SAFETY: `window` is read by value; `GetAncestor` walks the ancestor chain in the window
    // manager's own state and dereferences nothing of ours. A handle that is stale or null —
    // both possible for an asynchronous event — comes back null, and null fails the comparison
    // against a foreground that was just established to be non-null.
    let root = unsafe { GetAncestor(window, GA_ROOT) };

    root == foreground
}

/// Whether a flush event is an `EVENT_OBJECT_FOCUS` whose window is the one the focus is
/// already on — the internal churn of the foreground window itself. Task **T-10-0e**.
///
/// # The defect this answers — measured, not assumed (Р-39)
///
/// The gate of task T-10-0 turns away the churn of *background* processes, but the frontmost
/// window's own churn passes it legitimately — its root **is** the foreground. An Electron
/// application in front (VS Code, measured) re-raises `EVENT_OBJECT_FOCUS` on the **same**
/// window as a trailing echo of ordinary activity — a click, an Enter, this program's own
/// injection — at 0.3–1.5 s delay, and every such echo flushed what the user had typed
/// since: 11 of 36 strokes erased over six rounds, one hotkey press of six landing on a
/// fully empty buffer (task report, section 3). On 24 recorded churn events of the
/// frontmost window the handle was the same all 24 times — the same handle the *legitimate*
/// focus event carried when the user entered the window — while every real focus transfer
/// in a native application carried the hwnd of the newly focused control, different each
/// time (task report, section 4).
///
/// # Why skipping a same-window event loses nothing
///
/// A focus event whose window equals the previous focus event's window does not move the
/// user's input target: the strokes keep going where they went. Every gesture that *does*
/// move the caret inside one window flushes by its own row of FR-10 — the mouse click over
/// Raw Input, `Tab` and the navigation keys over the LL hook (`BOUNDARY_KEYS`,
/// `EDITING_KEYS` of `crate::buffer`) — measured live in the task report: with the
/// deduplication active, `Tab` and a click in the frontmost Electron window still empty the
/// buffer. The only events lost are the ones that report no change, and the probe of FR-21
/// is skipped with the flush for the same reason: the layout of a thread the focus never
/// left cannot have changed by that event.
///
/// **`EVENT_SYSTEM_FOREGROUND` is deliberately not subject to this**: a window change
/// always flushes, however often it lands on the same window — leaving a window and coming
/// back *is* a change of the user's input context, whatever handle it reuses.
///
/// # Cost — NFR-01…NFR-05
///
/// One atomic swap, and on the skip path one counter increment; **no new system call** over
/// what the callback already did (the budget of task T-10-0 stands). The handle is compared
/// as a value and never dereferenced. A same-handle skip *saves* the compare-and-swap loop
/// and both posts.
///
/// Public for the same reason [`concerns_the_foreground`] is: the verdict is the whole of
/// what task T-10-0e changed, and `tests\watchdog.rs` drives it directly with handle values
/// of its own alongside the staged-churn tests that drive it through a real subscription.
pub fn focus_repeated(event: u32, window: HWND) -> bool {
    if event != EVENT_OBJECT_FOCUS {
        return false;
    }

    let handle = window.0 as isize;
    let last = LAST_FOCUS_TARGET.swap(handle, Ordering::AcqRel);

    if last == handle {
        FOCUS_REPEATS.fetch_add(1, Ordering::Relaxed);
        return true;
    }

    false
}

/// The `WinEvent` callback — FR-10, half of the FR-21 delivery, and the first mechanism of
/// FR-80.
///
/// Runs on the watcher thread, called by the system out of that thread's message queue.
/// **Everything it does is two read-only window queries, an atomic and a `PostMessageW`**, for
/// the reason given in the module documentation: a `WinEvent` callback is as much in the
/// system's way as a hook callback is. That applies with particular force to
/// `EVENT_SYSTEM_DESKTOPSWITCH`, which the system raises while it is switching desktops:
/// reinstalling the hook from here would put a `SetWindowsHookExW` inside the system's own
/// desktop switch. The arm below marks and wakes, and [`reinstall_hook`] runs later, on the
/// input thread, out of its ordinary message loop.
///
/// The buffer cannot be flushed from here even if it were free to be: the buffer is a
/// thread-local of the *input* thread (section 6.3), so the only correct thing to do with the
/// event is to hand its timestamp to that thread, which is what happens.
///
/// # Safety
///
/// Called by the OS with the arguments of a `WinEvent`. Every argument is read by value, none
/// is dereferenced — the window handle travels by value into [`concerns_the_foreground`],
/// which hands it to the window manager and dereferences nothing — so the caller owes this
/// function nothing. Nothing in the body can panic, which is what keeps the `extern "system"`
/// boundary sound: there is no allocation, no indexing, no `unwrap` and no arithmetic that can
/// overflow.
unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    _object_id: i32,
    _child_id: i32,
    _thread_id: u32,
    event_time: u32,
) {
    if event == EVENT_SYSTEM_DESKTOPSWITCH {
        // **FR-80, first mechanism.** A switch to the secure desktop — a UAC prompt,
        // `Ctrl+Alt+Del` — is one of the four cases in which the system takes the hook away
        // without saying so. It is deliberately **not** a flush: FR-10 does not list it, and the
        // user coming back from a UAC prompt has typed nothing that should be thrown away.
        //
        // Two events arrive for one prompt, one for each direction, and the second one finds the
        // hook this program put back after the first. That costs one more reinstallation and is
        // exactly what makes the mechanism correct without knowing which direction is which.
        DESKTOP_SWITCHES.fetch_add(1, Ordering::Relaxed);
        request_rehook(Reason::DesktopSwitch);
        return;
    }

    if !is_flush_event(event) {
        // A subscription is per event code, so this cannot happen; it is here because a callback
        // the system drives is the wrong place to assume anything.
        return;
    }

    if !concerns_the_foreground(window) {
        // **Task T-10-0, Р-60: the internal churn of a background process is not a change of
        // the user's input context.** Before this gate, the idle focus traffic of background
        // Electron windows — ~0.8 events a second, measured — flushed the buffer between the
        // user's typing and the hotkey, and the acceptance session failed on it. The event is
        // counted and dropped whole: no flush, and no layout probe either, because the layout
        // question below is about the thread of the **foreground** window, which a background
        // event says nothing about. Every event that passes the gate still asks it.
        BACKGROUND_SKIPS.fetch_add(1, Ordering::Relaxed);
        return;
    }

    if focus_repeated(event, window) {
        // **Task T-10-0e: the frontmost window's own churn re-announces the focus it
        // already has.** The window equals the previous focus event's window, so the user's
        // input target has not moved and there is nothing to protect by flushing — while
        // flushing here is exactly what erased the user's typing on the acceptance machine
        // (VS Code in front, measured). Counted and dropped whole: no flush, and no layout
        // probe either — the layout of a thread the focus never left has not changed.
        // `EVENT_SYSTEM_FOREGROUND` never answers true here: a window change always flushes.
        return;
    }

    // FR-10, rows "смена активного окна" and "смена фокуса внутри окна". FR-12 is why the
    // timestamp travels with the request instead of being taken when it is applied.
    request_flush(event_time);
    crate::app::post_to_input_thread(WM_APP_FLUSH);

    // FR-21 delivery. The layout is a property of the thread that owns the window the user is
    // typing into, so a new foreground window — or a new focus inside one — is the moment to ask
    // whether it changed. The question is asked *by the input thread*, not here:
    // `GetForegroundWindow` and `GetKeyboardLayout` are cheap, but they are still Win32 calls and
    // this callback owes the system its speed. The input thread answers it in
    // `app::refresh_layout_and_cache` and rebuilds only if the answer differs from what it has.
    //
    // ⚠ FR-11: this is a layout *question*, never a flush. The flush above is the window change
    // itself, which the FR-10 table lists in its own right and which would happen with or
    // without this line.
    crate::app::post_to_input_thread(WM_APP_LAYOUT);
}

// ---------------------------------------------------------------------------------------
// FR-80 — the watchdog proper
// ---------------------------------------------------------------------------------------

/// Why the hook was last put back — the four mechanisms of FR-80, and "not yet".
///
/// **SEC-01, SEC-07.** Five named constants and nothing else. There is no arm here that could
/// carry a key code, a scan code or a character, and none may ever be added: this value is
/// published through the channel of SEC-04a, and the whole argument for letting it out is that
/// the set of things it can say is closed and listed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Reason {
    /// The hook has not been reinstalled since the program started.
    #[default]
    None = 0,
    /// The liveness timer of FR-80 — the fourth mechanism, and the only one that fires when
    /// nothing at all has happened.
    Timer = 1,
    /// `EVENT_SYSTEM_DESKTOPSWITCH` — a UAC prompt or `Ctrl+Alt+Del`.
    DesktopSwitch = 2,
    /// `WM_WTSSESSION_CHANGE` — the session was locked, unlocked, connected or disconnected.
    SessionChange = 3,
    /// `WM_POWERBROADCAST` with `PBT_APMRESUMEAUTOMATIC` — the machine came back from sleep.
    PowerResume = 4,
}

impl Reason {
    /// The word the channel of SEC-04a prints for this reason.
    ///
    /// Written out here rather than in [`crate::control`] so that the list of words and the list
    /// of arms cannot drift apart — the same shape `control::method_name` uses for FR-42.
    pub const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Timer => "timer",
            Self::DesktopSwitch => "desktop_switch",
            Self::SessionChange => "session_change",
            Self::PowerResume => "power_resume",
        }
    }

    /// The reason a stored code names, or [`Reason::None`] for anything else.
    ///
    /// Total, because the value comes out of an atomic and a total function is one less thing
    /// that can panic on a path the window procedure runs.
    const fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Timer,
            2 => Self::DesktopSwitch,
            3 => Self::SessionChange,
            4 => Self::PowerResume,
            _ => Self::None,
        }
    }
}

/// Completed reinstallations of the hook — every one of them, whatever the reason.
///
/// This is the number FR-90 and SEC-04a ask for, and it counts *reinstallations* rather than
/// *rescues* because the program cannot tell the two apart in advance; see the module
/// documentation. [`SILENT_REMOVALS`] is the part of it that was demonstrably a rescue.
static RECOVERIES: AtomicU32 = AtomicU32::new(0);

/// Reinstallations at which `UnhookWindowsHookEx` failed on a handle this program believed live
/// — that is, at which the system had already taken the hook away silently.
///
/// The retrospective detector of FR-80, and the only evidence of a silent removal there is.
static SILENT_REMOVALS: AtomicU32 = AtomicU32::new(0);

/// Reinstallations at which the hook was already known to be absent before the attempt.
///
/// Distinct from [`SILENT_REMOVALS`]: this is the case in which this program's own bookkeeping
/// already said there was no hook — a failed previous reinstallation, or FR-83's `WM_ENDSESSION`
/// path having taken it off.
static ABSENT_AT_CHECK: AtomicU32 = AtomicU32::new(0);

/// `SetWindowsHookExW` refusals during a reinstallation — NFR-13.
///
/// A non-zero value here is the one state in which this program is running without a hook and
/// knows it; [`hook_down`] is what reports it and FR-90 is what the tray does about it.
static INSTALL_FAILURES: AtomicU32 = AtomicU32::new(0);

/// [`Reason`] of the last completed reinstallation, as a code — see [`Reason::from_code`].
static LAST_REASON: AtomicU32 = AtomicU32::new(0);

/// The reason waiting for the input thread, set by whoever posted [`WM_APP_REHOOK`].
///
/// SEC-05: this is why the message itself carries nothing. A forged [`WM_APP_REHOOK`] finds this
/// cell empty and the handler returns without touching the hook.
static PENDING_REASON: AtomicU32 = AtomicU32::new(0);

/// Ticks of the liveness timer that reached the handler — NFR-10 measured rather than asserted.
static LIVENESS_TICKS: AtomicU32 = AtomicU32::new(0);

/// `EVENT_SYSTEM_DESKTOPSWITCH` events seen.
static DESKTOP_SWITCHES: AtomicU32 = AtomicU32::new(0);

/// `WM_WTSSESSION_CHANGE` messages seen at the UI window.
static SESSION_CHANGES: AtomicU32 = AtomicU32::new(0);

/// `WM_POWERBROADCAST` messages with `PBT_APMRESUMEAUTOMATIC` seen at the UI window.
static POWER_RESUMES: AtomicU32 = AtomicU32::new(0);

/// Microseconds the last reinstallation left this process without a hook.
static LAST_GAP_US: AtomicU32 = AtomicU32::new(0);

/// The longest such gap so far.
static MAX_GAP_US: AtomicU32 = AtomicU32::new(0);

/// The value [`NOTICE_WINDOW`] holds when there is no such window.
///
/// Zero is not a window handle, which is what lets one atomic carry both the handle and its
/// absence.
const NO_WINDOW: usize = 0;

/// The window [`register_session_notice`] was given, as a raw value — the process's one
/// top-level window, and the only one `WM_POWERBROADCAST` and `WM_WTSSESSION_CHANGE` can reach.
///
/// Raw rather than an `HWND` for the reason `app::ui_window_raw` states: `HWND` is a raw pointer
/// and therefore not `Send`, and a static that promised otherwise would be a lie about a value
/// two threads really do look at. Nothing is ever done with it but a comparison — see
/// [`is_ui_window`] — so it is never converted back.
static NOTICE_WINDOW: AtomicUsize = AtomicUsize::new(NO_WINDOW);

/// Whether a reinstallation is between its `UnhookWindowsHookEx` and its `SetWindowsHookExW`.
///
/// Read by [`hook_down`] and by nothing else: without it the tray of FR-90 could sample
/// [`crate::hook::is_installed`] inside the microsecond gap of a healthy reinstallation and
/// report a program that is perfectly well as suspended.
static REINSTALLING: AtomicBool = AtomicBool::new(false);

/// What the watchdog of FR-80 has done — counts, durations and a reason code.
///
/// **SEC-01, SEC-07.** Nine numbers and one of five named words. No key code, no scan code, no
/// character, and nothing derived from one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Health {
    /// Completed reinstallations of the hook — [`RECOVERIES`].
    pub recoveries: u32,
    /// Of those, the ones at which the system had demonstrably taken the hook already.
    pub silent_removals: u32,
    /// Of those, the ones at which this program already knew it had no hook.
    pub absent_at_check: u32,
    /// `SetWindowsHookExW` refusals — NFR-13.
    pub install_failures: u32,
    /// Why the hook was last put back.
    pub last_reason: Reason,
    /// Microseconds without a hook during the last reinstallation.
    pub last_gap_us: u32,
    /// The longest such gap so far.
    pub max_gap_us: u32,
    /// Ticks of the thirty-second timer — NFR-10.
    pub liveness_ticks: u32,
    /// `EVENT_SYSTEM_DESKTOPSWITCH` events seen.
    pub desktop_switches: u32,
    /// `WM_WTSSESSION_CHANGE` messages seen.
    pub session_changes: u32,
    /// `PBT_APMRESUMEAUTOMATIC` broadcasts seen.
    pub power_resumes: u32,
}

/// What the watchdog of FR-80 has done so far.
pub fn health() -> Health {
    Health {
        recoveries: RECOVERIES.load(Ordering::Relaxed),
        silent_removals: SILENT_REMOVALS.load(Ordering::Relaxed),
        absent_at_check: ABSENT_AT_CHECK.load(Ordering::Relaxed),
        install_failures: INSTALL_FAILURES.load(Ordering::Relaxed),
        last_reason: Reason::from_code(LAST_REASON.load(Ordering::Relaxed)),
        last_gap_us: LAST_GAP_US.load(Ordering::Relaxed),
        max_gap_us: MAX_GAP_US.load(Ordering::Relaxed),
        liveness_ticks: LIVENESS_TICKS.load(Ordering::Relaxed),
        desktop_switches: DESKTOP_SWITCHES.load(Ordering::Relaxed),
        session_changes: SESSION_CHANGES.load(Ordering::Relaxed),
        power_resumes: POWER_RESUMES.load(Ordering::Relaxed),
    }
}

/// Whether this program currently has **no** keyboard hook — FR-90, the "приостановлена" of the
/// tray when it is not the user who suspended it.
///
/// ⚠ The honest reading of this function is "as far as this program can tell". A hook the system
/// has taken away silently still answers `true` to [`crate::hook::is_installed`] until the
/// watchdog next reinstalls, which is what FR-80 exists for and what no call can shorten. What
/// this *does* report truthfully is the state that matters to the user: a
/// `SetWindowsHookExW` that refused, leaving the program running and deaf until the next tick.
///
/// The microsecond during which a healthy reinstallation has taken the hook off and not yet put
/// it back is deliberately not reported as down — see [`REINSTALLING`].
///
/// Displaying this is task T-08-1's; publishing it is this module's.
pub fn hook_down() -> bool {
    !crate::hook::is_installed() && !REINSTALLING.load(Ordering::Acquire)
}

/// Records that the hook should be put back, and wakes the thread that will do it.
///
/// **The whole of what a `WinEvent` callback and a window procedure of another thread are
/// allowed to do about FR-80**: one relaxed store and one `PostMessageW`, which does not block.
/// No allocation (NFR-03), no lock (NFR-04), no I/O (NFR-05), and nothing that can panic.
///
/// Two requests that arrive before the input thread gets round to either collapse into one, and
/// that is lossless: the reinstallation the second would have caused is the same operation as
/// the first, and the reason the input thread ends up acting on is the later of the two.
pub fn request_rehook(reason: Reason) {
    PENDING_REASON.store(reason as u32, Ordering::Release);
    crate::app::post_to_input_thread(WM_APP_REHOOK);
}

/// Takes the hook off and puts it back — the single recovery path of FR-80, and the only place
/// in this module that touches [`crate::hook`].
///
/// ⚠ **Input thread only.** A `WH_KEYBOARD_LL` hook is called back on the thread that installed
/// it, so installing it anywhere else would move the callback off the thread section 6.1 built
/// for it. Every caller in this module checks [`is_input_window`] first.
///
/// # Order, and why there is no choice about it
///
/// Uninstall, then install. [`crate::hook::install`] refuses while a hook is registered —
/// FR-01 says *one* hook, and two would mean every stroke recorded twice — so the other order
/// cannot even be written against the accepted module. It is also the only order that cannot
/// leave two hooks live for an instant.
///
/// # What is lost
///
/// The strokes made between the two calls, which is a few microseconds' worth
/// ([`Health::max_gap_us`] is the measured figure). They are not swallowed — this program
/// suppresses nothing but its own hotkey — so they reach the application the user is typing
/// into exactly as they always would; what is lost is their place in the typing buffer, and
/// with it the ability to convert that fragment. FR-96 is not armed during the gap either,
/// which is the one part of it that a reinstallation cannot carry across.
///
/// # NFR-13
///
/// Both Win32 results are examined. `UnhookWindowsHookEx` is examined twice over, in fact: its
/// failure counter is read before and after, because a failure on a handle this program
/// believed live is the only evidence of a silent removal that exists anywhere.
pub fn reinstall_hook(notify: HWND, reason: Reason) -> bool {
    let instance = match module_instance() {
        Ok(instance) => instance,
        Err(error) => {
            INSTALL_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::app::report_non_critical("GetModuleHandleW (watchdog)", &error);
            return false;
        }
    };

    let believed_installed = crate::hook::is_installed();
    let (unhook_failures_before, _) = crate::hook::unhook_failures();

    if !believed_installed {
        ABSENT_AT_CHECK.fetch_add(1, Ordering::Relaxed);
    }

    REINSTALLING.store(true, Ordering::Release);
    let started = Instant::now();

    crate::hook::uninstall();

    let (unhook_failures_after, _) = crate::hook::unhook_failures();

    if believed_installed && unhook_failures_after != unhook_failures_before {
        // The handle this program was holding is no longer a hook. Nothing else can produce
        // that: `uninstall` takes the handle out of its atomic with a single swap, so a second
        // caller cannot have unhooked it, and the handle came from a successful
        // `SetWindowsHookExW`. This is a silent removal by the system, observed after the fact.
        SILENT_REMOVALS.fetch_add(1, Ordering::Relaxed);
    }

    let outcome = crate::hook::install(notify, instance);
    let gap_us = u32::try_from(started.elapsed().as_micros()).unwrap_or(u32::MAX);

    REINSTALLING.store(false, Ordering::Release);

    match outcome {
        Ok(installed) => {
            // ⚠ The guard must **not** run. `Installed::drop` calls `hook::uninstall`, so
            // dropping it here would take off the hook that was just put on, and the program
            // would be deaf until the next tick. The guard `app::serve_window` has held since
            // start-up is what removes the hook at the end: `uninstall` reads the handle out of
            // an atomic, so it removes whichever hook is current, including this one.
            let _kept = ManuallyDrop::new(installed);

            LAST_GAP_US.store(gap_us, Ordering::Relaxed);
            MAX_GAP_US.fetch_max(gap_us, Ordering::Relaxed);
            LAST_REASON.store(reason as u32, Ordering::Relaxed);
            RECOVERIES.fetch_add(1, Ordering::Relaxed);
            true
        }
        Err(error) => {
            // NFR-13. The program is now running without a hook and knows it — `hook_down`
            // answers `true` — and the next tick of the liveness timer tries again.
            INSTALL_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::app::report_non_critical("SetWindowsHookExW (watchdog)", &error);
            false
        }
    }
}

/// The window-procedure entry point of the watchdog, called by `app::window_proc`.
///
/// `Some` means the message was handled and that value has to be returned to Windows; `None`
/// means it is none of ours. Mirrors [`crate::hook::handle_input_message`] and
/// `crate::tray::handle_ui_message`, and for the same reason: the window procedure belongs to
/// `app` and the knowledge of which messages matter belongs to the module that defined them.
///
/// **Every arm is bound to a window.** The two system messages are answered on the UI window,
/// which is the only top-level window this program has and therefore the only one they can
/// arrive at; the timer and the rehook request are answered on the input window, which is the
/// only thread allowed to own the hook. SEC-05: a copy of any of them aimed at the wrong window
/// of ours falls through to `DefWindowProcW` and does nothing.
pub fn handle_watchdog_message(window: HWND, message: u32, wparam: WPARAM) -> Option<LRESULT> {
    match message {
        // **FR-80, third mechanism.** `PBT_APMRESUMEAUTOMATIC` is delivered on every resume,
        // including the ones nobody was present for, which is exactly the case FR-80 is about:
        // the machine woke, the hook did not, and there is no user to notice.
        //
        // `RegisterPowerSettingNotification` is **not** used and is not needed: it exists for
        // `PBT_POWERSETTINGCHANGE`, which FR-80 does not name, and its feature is absent from
        // section 3.2. The `PBT_APM*` notifications arrive at every top-level window without any
        // registration at all.
        WM_POWERBROADCAST if is_ui_window(window) => {
            if wparam.0 as u32 == PBT_APMRESUMEAUTOMATIC {
                POWER_RESUMES.fetch_add(1, Ordering::Relaxed);
                request_rehook(Reason::PowerResume);
            }

            // TRUE, which is what a window that has finished with a power notification answers.
            // The other `PBT_*` subtypes fall in here too and are counted by nobody: this
            // program has nothing to say about a suspend request and grants it by answering
            // TRUE, which is also what `DefWindowProcW` would have done.
            Some(LRESULT(1))
        }

        // **FR-80, second mechanism.** Every subtype is treated alike — lock, unlock, console
        // connect and disconnect, logon and logoff. Telling them apart would be a distinction
        // without a difference: the hook has to go back either way, and reinstalling it when it
        // did not need it costs the microseconds `reinstall_hook` documents.
        WM_WTSSESSION_CHANGE if is_ui_window(window) => {
            SESSION_CHANGES.fetch_add(1, Ordering::Relaxed);
            request_rehook(Reason::SessionChange);
            Some(LRESULT(0))
        }

        // **FR-80, fourth mechanism** — the only one that fires when nothing has happened, and
        // the only one that answers the `LowLevelHooksTimeout` case, which announces itself with
        // nothing whatsoever.
        WM_TIMER if wparam.0 == LIVENESS_TIMER_ID && is_input_window() => {
            LIVENESS_TICKS.fetch_add(1, Ordering::Relaxed);
            reinstall_hook(window, Reason::Timer);
            Some(LRESULT(0))
        }

        // The far end of [`request_rehook`], for the three mechanisms that observe the event on
        // another thread. SEC-05: the message carries nothing, and a forged one finds
        // [`PENDING_REASON`] empty and returns without touching the hook.
        WM_APP_REHOOK if is_input_window() => {
            let pending = PENDING_REASON.swap(Reason::None as u32, Ordering::AcqRel);

            match Reason::from_code(pending) {
                Reason::None => {}
                reason => {
                    reinstall_hook(window, reason);
                }
            }

            Some(LRESULT(0))
        }

        // SEC-04a, feature `testing`, absent from the Release configuration: the fault injection
        // that lets an acceptance run watch the watchdog work.
        #[cfg(feature = "testing")]
        WM_TIMER if wparam.0 == fault::DROP_HOOK_TIMER_ID && is_input_window() => {
            fault::drop_the_hook(window);
            Some(LRESULT(0))
        }

        _ => None,
    }
}

/// Whether `window` is the top-level window of the UI thread — the only window of this process a
/// broadcast to top-level windows can reach.
///
/// # Why the handle comes from [`NOTICE_WINDOW`] and not from `app`
///
/// The obvious spelling — comparing against `app::ui_window_raw` — does not compile in the
/// shipping configuration: that accessor is `#[cfg(panic = "unwind")]`, it exists for FR-99, and
/// the Release profile of section 3.2 sets `panic = "abort"`. The mistake was caught by
/// `cargo build --release` and not by reasoning, which is the reason the acceptance criteria ask
/// for that build separately.
///
/// So this module remembers the handle itself, at the one place it is handed one:
/// [`register_session_notice`], which `app::serve_window` calls on the UI thread and on no
/// other. The two questions are the same question — "which window of ours can a message aimed at
/// top-level windows reach" — because the session registration is made on that window for
/// exactly the reason the power broadcast needs it. One relaxed atomic load and no Win32 call.
fn is_ui_window(window: HWND) -> bool {
    let notice = NOTICE_WINDOW.load(Ordering::Acquire);

    notice != NO_WINDOW && window.0 as usize == notice
}

/// Whether the calling thread is the input thread.
///
/// Section 6.3 gives the typing buffer to that thread and to no other, so owning it is what
/// being the input thread means — the same test `app::window_proc` already uses to tell the
/// three windows apart.
fn is_input_window() -> bool {
    crate::buffer::is_installed()
}

/// Handle of this module, the `hInstance` `SetWindowsHookExW` wants.
///
/// `app` has a private function of the same shape; this one exists so that a reinstallation is
/// self-contained and `app` keeps to the three lines task T-06-2 was allowed to add to it. The
/// call costs a lookup in the loader's table, once per reinstallation, which is twice a minute.
fn module_instance() -> WinResult<HINSTANCE> {
    // SAFETY: `None` asks for the handle of the file used to create the calling process, which
    // is the documented way to name the running executable and cannot refer to a module that
    // could be unloaded under us. The result is a borrowed handle that must not be freed, and
    // nothing here frees it.
    let module = unsafe { GetModuleHandleW(PCWSTR::null())? };

    Ok(HINSTANCE(module.0))
}

/// The session-change registration of FR-80, undone when this value is dropped.
///
/// The registration is a resource the system holds **against the window**, which is why the
/// undoing is explicit and not left to the window's destruction: FR-80 names
/// `WTSUnRegisterSessionNotification` and the task specification of T-06-2 calls it obligatory.
pub struct SessionNotice {
    /// The window the registration names, so that the removal names the same one.
    window: HWND,
}

/// Asks for `WM_WTSSESSION_CHANGE` at `window` — FR-80, second mechanism.
///
/// `NOTIFY_FOR_THIS_SESSION` and not `NOTIFY_FOR_ALL_SESSIONS`: FR-82 makes this program one
/// instance per user session, its hook is a hook of that session, and the sessions of other
/// users are none of its business.
///
/// ⚠ `window` must be the **top-level** window of the UI thread. A message-only window is a
/// child of `HWND_MESSAGE`, and the whole family of session and power notifications is aimed at
/// windows that are not.
pub fn register_session_notice(window: HWND) -> WinResult<SessionNotice> {
    // SAFETY: `window` is the caller's live window and is passed by value; the call registers
    // the handle with the terminal-services subsystem and dereferences nothing of ours. The
    // guard returned here is dropped before that window is destroyed, because
    // `app::serve_window` declares it after the window.
    unsafe { WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION) }?;

    // Published only after the registration is known good, and it is what [`is_ui_window`] reads:
    // this is the moment the module learns which of the three windows is the top-level one.
    NOTICE_WINDOW.store(window.0 as usize, Ordering::Release);

    Ok(SessionNotice { window })
}

impl Drop for SessionNotice {
    fn drop(&mut self) {
        // Unpublished first, so that no message can be answered on a window whose registration
        // is about to go — the same order `app::Window::drop` uses for the same reason.
        NOTICE_WINDOW.store(NO_WINDOW, Ordering::Release);

        // SAFETY: `self.window` is the handle a successful `WTSRegisterSessionNotification`
        // was given, and this type is neither `Copy` nor `Clone`, so the registration is
        // removed exactly once. The window is still alive: `app::serve_window` declares this
        // guard after the window and drop order takes it first.
        if let Err(error) = unsafe { WTSUnRegisterSessionNotification(self.window) } {
            // NFR-13: examined. Nothing can be done about it, and the system drops the
            // registration when the window is destroyed in any case.
            crate::app::report_non_critical("WTSUnRegisterSessionNotification", &error);
        }
    }
}

/// The liveness timer of FR-80, killed when this value is dropped.
pub struct Liveness {
    /// The window the timer belongs to, so that the removal names the same one.
    window: HWND,
}

/// Starts the thirty-second liveness timer of FR-80 on `window` — FR-80, fourth mechanism.
///
/// ⚠ `window` must be the **input** thread's window: `WM_TIMER` is delivered to the thread that
/// owns the window, and the reinstallation it causes has to happen on the thread that owns the
/// hook. Section 6.1 puts "таймер сторожа" under the input thread for that reason.
///
/// A window timer and not a thread of its own: a fourth thread would be outside section 6.1, and
/// a sleeping loop would be a thread waking up on a schedule of its own instead of the system's.
/// `SetTimer` costs nothing while the queue is idle and NFR-10 gets the coarsest wake-up the
/// requirement allows.
pub fn start_liveness_timer(window: HWND) -> WinResult<Liveness> {
    // SAFETY: `window` is the caller's live window, on the calling thread, which is what a
    // window timer requires; `None` for the callback asks for `WM_TIMER` in the ordinary message
    // queue rather than a `TIMERPROC` pointer, so no function of ours is registered anywhere and
    // nothing can dangle. The guard returned here is dropped before the window is destroyed.
    let started = unsafe { SetTimer(Some(window), LIVENESS_TIMER_ID, LIVENESS_INTERVAL_MS, None) };

    // NFR-13: `SetTimer` reports failure with zero. A program whose liveness timer never
    // started is a program that has lost the only mechanism answering the `LowLevelHooksTimeout`
    // case of FR-80, so this is an error and not a warning.
    if started == 0 {
        return Err(WinError::from_thread());
    }

    #[cfg(feature = "testing")]
    fault::arm_from_environment(window);

    Ok(Liveness { window })
}

impl Drop for Liveness {
    fn drop(&mut self) {
        #[cfg(feature = "testing")]
        fault::disarm(self.window);

        // SAFETY: the pair `(window, id)` is the one a successful `SetTimer` returned for, and
        // this type is neither `Copy` nor `Clone`, so the timer is killed exactly once. It runs
        // on the thread that set it: the guard is created inside `app::serve_window` and never
        // leaves that frame.
        if let Err(error) = unsafe { KillTimer(Some(self.window), LIVENESS_TIMER_ID) } {
            // NFR-13: examined. The timer dies with the window in any case.
            crate::app::report_non_critical("KillTimer", &error);
        }
    }
}

/// Fault injection for the acceptance run — SEC-04a, feature `testing`, absent from the Release
/// configuration.
///
/// FR-80 is about a failure nobody can stage: waiting for the system to take the hook away is
/// waiting for a UAC prompt, a sleep or a wedged callback. Point 21 of the acceptance criteria
/// asks instead for the hook to be **taken off from inside the program** and for the watchdog to
/// be caught putting it back, and this is the switch that does it: one environment variable, one
/// one-shot window timer, one call to the same `hook::uninstall` FR-83 and FR-96 use.
///
/// It stages the *consequence* of a silent removal — no hook — and not its cause, and the report
/// of task T-06-2 says so plainly. What it cannot stage is the system's own bookkeeping: after
/// this the program knows it has no hook, where after a real silent removal it would not.
#[cfg(feature = "testing")]
mod fault {
    use core::sync::atomic::{AtomicBool, Ordering};

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

    /// Environment variable naming the delay, in milliseconds, after which the hook is dropped.
    ///
    /// Absent means armed by nothing, which is what every run that did not ask for it gets.
    const DROP_HOOK_ENV_VAR: &str = "LANGSW_TESTING_DROP_HOOK_MS";

    /// `nIDEvent` of the one-shot timer. Distinct from [`super::LIVENESS_TIMER_ID`].
    pub const DROP_HOOK_TIMER_ID: usize = 2;

    /// Whether the one-shot timer is running, so that [`disarm`] does not try to kill a timer
    /// that was never set and journal the refusal.
    static ARMED: AtomicBool = AtomicBool::new(false);

    /// Sets the one-shot timer if the environment asks for it.
    pub fn arm_from_environment(window: HWND) {
        let Ok(raw) = std::env::var(DROP_HOOK_ENV_VAR) else {
            return;
        };

        let Ok(delay_ms) = raw.trim().parse::<u32>() else {
            return;
        };

        if delay_ms == 0 {
            return;
        }

        // SAFETY: the same invariants `super::start_liveness_timer` states — the caller's live
        // window, on the calling thread, and `None` for the callback so that no function pointer
        // of ours is registered.
        let started = unsafe { SetTimer(Some(window), DROP_HOOK_TIMER_ID, delay_ms, None) };

        ARMED.store(started != 0, Ordering::Release);
    }

    /// Kills the one-shot timer if it is still running.
    pub fn disarm(window: HWND) {
        if !ARMED.swap(false, Ordering::AcqRel) {
            return;
        }

        // SAFETY: the pair `(window, id)` is the one `arm_from_environment` set the timer for,
        // on this thread, and `ARMED` makes the kill happen exactly once.
        if let Err(error) = unsafe { KillTimer(Some(window), DROP_HOOK_TIMER_ID) } {
            crate::app::report_non_critical("KillTimer (fault)", &error);
        }
    }

    /// Takes the hook off and stops the one-shot timer — the injected fault itself.
    pub fn drop_the_hook(window: HWND) {
        disarm(window);
        crate::hook::uninstall();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-10, FR-13: a press is a flush and a movement is not. The whole of point 11 of the
    /// acceptance criteria, at the level where the decision is actually made.
    #[test]
    fn a_button_press_is_a_flush_and_a_movement_is_not() {
        // What a mouse that has only moved reports.
        assert!(!mouse_button_pressed(0));

        for down in [
            RI_MOUSE_BUTTON_1_DOWN,
            RI_MOUSE_BUTTON_2_DOWN,
            RI_MOUSE_BUTTON_3_DOWN,
            RI_MOUSE_BUTTON_4_DOWN,
            RI_MOUSE_BUTTON_5_DOWN,
        ] {
            assert!(mouse_button_pressed(down as u16), "button {down:#x}");
        }
    }

    /// The two events of the FR-10 table and nothing else. `EVENT_SYSTEM_DESKTOPSWITCH` is
    /// subscribed to by task T-06-2 but is **not** a flush: a UAC prompt must not throw away
    /// what the user has typed.
    #[test]
    fn the_subscription_list_is_the_two_rows_of_fr10() {
        assert_eq!(FLUSH_EVENTS, [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_FOCUS]);
        assert!(is_flush_event(EVENT_SYSTEM_FOREGROUND));
        assert!(is_flush_event(EVENT_OBJECT_FOCUS));

        // 0x0020 — `EVENT_SYSTEM_DESKTOPSWITCH`, written out rather than imported so that adding
        // the constant to the list is a deliberate act and not an import away.
        assert!(!is_flush_event(0x0020));
    }

    /// FR-80: the desktop switch joined the list [`watch`] iterates over, and the list is the
    /// three events section 6.1 puts under the watcher thread.
    #[test]
    fn the_watched_list_is_the_two_of_fr10_plus_the_one_of_fr80() {
        assert_eq!(
            WATCHED_EVENTS,
            [
                EVENT_SYSTEM_FOREGROUND,
                EVENT_OBJECT_FOCUS,
                EVENT_SYSTEM_DESKTOPSWITCH
            ]
        );
        assert_eq!(WATCHED_EVENTS[..FLUSH_EVENTS.len()], FLUSH_EVENTS);
        assert_eq!(EVENT_SYSTEM_DESKTOPSWITCH, 0x0020);
    }

    /// Every code the channel of SEC-04a can print, and the round trip that keeps the words and
    /// the arms from drifting apart.
    #[test]
    fn every_reason_survives_the_atomic_it_travels_in() {
        for reason in [
            Reason::None,
            Reason::Timer,
            Reason::DesktopSwitch,
            Reason::SessionChange,
            Reason::PowerResume,
        ] {
            assert_eq!(Reason::from_code(reason as u32), reason, "{reason:?}");
        }

        // Anything that is not one of the five is "not yet", never a panic: the value comes out
        // of an atomic and is read inside a window procedure.
        assert_eq!(Reason::from_code(5), Reason::None);
        assert_eq!(Reason::from_code(u32::MAX), Reason::None);
    }
}
