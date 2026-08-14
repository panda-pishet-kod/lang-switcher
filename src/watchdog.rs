//! Subscriptions to system events, restoring the hook.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC.
//!
//! Requirements this module covers: FR-13 (mouse clicks over **Raw Input**, never
//! `WH_MOUSE_LL`), the three asynchronous rows of the FR-10 flush table — a mouse button over
//! Raw Input, `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS` — and the **delivery** of
//! FR-21, that is, making the layout cache actually rebuild on a machine whose hidden windows
//! never receive `WM_INPUTLANGCHANGE` — task T-03-3.
//! FR-12, the race resolution these subscriptions feed, belongs to [`crate::buffer`]: the module
//! table gives "разрешение гонок по временны́м меткам" to the buffer, and this module hands it a
//! timestamp and nothing else.
//! Still to come: FR-80, the hook watchdog proper, and the `EVENT_SYSTEM_DESKTOPSWITCH`,
//! `WM_WTSSESSION_CHANGE` and `POWERBROADCAST` subscriptions — task T-06-2.
//! Moved out to match the backlog (decision R-17): FR-82 (single instance) to the process
//! level, task T-01-2; FR-83 (clean shutdown) to the tray, task T-01-4, module `tray`.
//! Implemented by backlog tasks: T-03-3 (done), T-06-2.
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
//! # Threading — section 6.1
//!
//! Section 6.1 puts each of these where it belongs and this module follows it exactly:
//!
//! * **input thread** — `RegisterRawInputDevices (RIDEV_INPUTSINK)` and the reception of
//!   `WM_INPUT`. The registration is bound to that thread's message-only window, and the buffer
//!   the flush lands in is a thread-local of that same thread (section 6.3), so the whole path
//!   from the click to the zeroed slot happens without a single cross-thread hand-off;
//! * **watcher thread** — `SetWinEventHook` for `EVENT_SYSTEM_FOREGROUND` and
//!   `EVENT_OBJECT_FOCUS`. An out-of-context `WinEvent` hook calls back on the thread that
//!   installed it, so installing it there is what puts the callback there.
//!
//! # NFR-01 to NFR-05, NFR-10 — what the callbacks are allowed to do
//!
//! A `WinEvent` callback is called by the system, synchronously, from inside the event source's
//! own delivery, exactly like a hook callback: a slow one holds up whatever generated the event.
//! [`win_event_proc`] is therefore built to the same budget as the keyboard callback even though
//! no requirement names a number for it — **one compare-and-swap on an atomic, two counter
//! increments and one `PostMessageW`**, which does not block. No allocation (NFR-03), no lock
//! (NFR-04), no I/O (NFR-05), no Win32 call that can be made to wait, and nothing that can
//! panic across the `extern "system"` boundary. The work the event implies — flushing a ring of
//! strokes, and possibly rebuilding the layout cache — is done later, by the input thread, in
//! its message loop.
//!
//! The `WM_INPUT` path is not a callback at all: it is a message, handled in the message loop
//! of the input thread. Its cost is one `GetRawInputData` into a stack buffer and a comparison
//! of two bits, and for the overwhelmingly common packet — a cursor movement — it stops at that
//! comparison and touches the buffer not at all.
//!
//! NFR-10, "загрузка ЦП в покое неотличима от нуля": nothing here polls, nothing here holds a
//! timer, and nothing here wakes the process on its own. Every one of these subscriptions is
//! silent until the user moves the mouse, clicks, or changes window — that is, until the machine
//! is by definition not at rest.
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
//! * a forged [`WM_APP_LAYOUT`] or `WM_INPUT_DEVICE_CHANGE` buys the sender a re-read of the
//!   system's own layout list into memory of ours, which is idempotent, and is the same standing
//!   the wake-up message of [`crate::app`] has.
//!
//! # SEC-01, SEC-07
//!
//! Nothing here is written to a log, a file or a panic message. What crosses this module is a
//! button-flag word, a millisecond timestamp and a count of events; no key code, no scan code
//! and no character passes through it, and the [`Counters`] this module publishes are counts and
//! nothing else.

use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Input::{
    GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT,
    RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEMOUSE, RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, GetMessageTime, RI_MOUSE_BUTTON_1_DOWN,
    RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_3_DOWN, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_5_DOWN,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_APP, WM_INPUT, WM_INPUT_DEVICE_CHANGE,
};
use windows::core::{Error as WinError, Result as WinResult};

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

/// The `WinEvent` events the flush table of FR-10 lists — "смена активного окна" and "смена
/// фокуса внутри окна".
///
/// Named as data rather than written into a condition so that a test can assert the list itself,
/// the same shape [`crate::layouts::REBUILD_MESSAGES`] uses for the messages of FR-21.
///
/// `EVENT_SYSTEM_DESKTOPSWITCH` is deliberately **absent**: it belongs to task T-06-2.
pub const FLUSH_EVENTS: [u32; 2] = [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_FOCUS];

/// HID usage page 1, "generic desktop controls" — the page both the mouse and the keyboard are
/// on.
const HID_USAGE_PAGE_GENERIC: u16 = 0x01;

/// HID usage 2 of the generic page: a mouse.
const HID_USAGE_MOUSE: u16 = 0x02;

/// HID usage 6 of the generic page: a keyboard.
const HID_USAGE_KEYBOARD: u16 = 0x06;

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
/// `WM_INPUT_DEVICE_CHANGE` is what `RIDEV_DEVNOTIFY` turns a keyboard arrival or removal into,
/// and it stands in for the `WM_DEVICECHANGE` of FR-21; [`WM_APP_LAYOUT`] stands in for its
/// `WM_INPUTLANGCHANGE`. Both stand-ins exist because neither message of FR-21 can reach this
/// program — see the module documentation of [`crate::app`] and the report of task T-03-3.
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

/// `WM_INPUT_DEVICE_CHANGE` messages the input thread answered — the `WM_DEVICECHANGE` half of
/// the FR-21 delivery.
static DEVICE_CHANGES: AtomicU32 = AtomicU32::new(0);

/// [`WM_APP_LAYOUT`] messages the input thread answered — the `WM_INPUTLANGCHANGE` half of the
/// FR-21 delivery. Higher than the number of rebuilds they caused, because most of them find the
/// layout unchanged.
static LAYOUT_PROBES: AtomicU32 = AtomicU32::new(0);

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
    /// Keyboard arrivals and removals the input thread answered — FR-21 delivery.
    pub device_changes: u32,
    /// Layout probes the input thread answered — FR-21 delivery.
    pub layout_probes: u32,
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
pub struct RawInput {
    /// Kept so that the removal names the same devices the registration did.
    devices: [RAWINPUTDEVICE; 2],
}

/// Registers for the raw mouse input of FR-13 and for keyboard device notifications, both
/// delivered to `target` — FR-13, and the device half of the FR-21 delivery.
///
/// # The two entries
///
/// * **mouse, `RIDEV_INPUTSINK`.** The flag is what FR-13 names, and what it means is "deliver
///   this even when the target window is not in the foreground" — which is the only mode of any
///   use to a program whose windows are hidden and never focused. `WM_INPUT` then arrives for
///   every mouse packet, movement included; [`mouse_button_pressed`] is what separates the two.
/// * **keyboard, `RIDEV_DEVNOTIFY`, and no `RIDEV_INPUTSINK`.** This entry asks for
///   `WM_INPUT_DEVICE_CHANGE` when a keyboard arrives or leaves, and for nothing else: without
///   `RIDEV_INPUTSINK` the raw keystrokes are delivered only to the window with the keyboard
///   focus, which ours never is, so this costs exactly zero messages while the user types. It is
///   how the `WM_DEVICECHANGE` half of FR-21 is delivered; see the report of task T-03-3 for why
///   this and not `RegisterDeviceNotification`.
///
/// Neither entry sets `RIDEV_NOLEGACY`, so ordinary mouse and keyboard messages continue to
/// reach every application, this one included, exactly as before. Registration is per-process
/// and changes nothing for anybody else.
pub fn register_raw_input(target: HWND) -> WinResult<RawInput> {
    let devices = [
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_MOUSE,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: target,
        },
        RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_KEYBOARD,
            dwFlags: RIDEV_DEVNOTIFY,
            hwndTarget: target,
        },
    ];

    // SAFETY: `devices` is a fully initialised array owned by this frame for the whole call, and
    // the call only reads through it — the registration the system keeps is a copy. `target` is
    // the caller's live window, which `RIDEV_INPUTSINK` and `RIDEV_DEVNOTIFY` both require to be
    // non-null, and the guard returned here is dropped before that window is destroyed because
    // `app::serve_window` declares it after the window. `size_of::<RAWINPUTDEVICE>()` is the
    // element size the call demands and is computed rather than written out.
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

/// The two `WinEvent` subscriptions, removed when this value is dropped.
///
/// The value never leaves the thread that created it, which is what makes the `Drop` correct:
/// an out-of-context hook is served by the installing thread's message queue and is unhooked
/// from that same thread.
pub struct Watching {
    hooks: [HWINEVENTHOOK; FLUSH_EVENTS.len()],
}

/// Subscribes to the two `WinEvent` events of the FR-10 table on the calling thread.
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
/// A separate hook per event, rather than one hook over the range between them: the two codes
/// are `0x0003` and `0x8005`, and a single subscription spanning that range would ask the system
/// to deliver every accessibility event there is, for every process in the session, so that we
/// could discard all but two of them.
pub fn watch() -> WinResult<Watching> {
    let mut hooks = [HWINEVENTHOOK::default(); FLUSH_EVENTS.len()];

    for (slot, event) in hooks.iter_mut().zip(FLUSH_EVENTS) {
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

/// The `WinEvent` callback — FR-10, and half of the FR-21 delivery.
///
/// Runs on the watcher thread, called by the system out of that thread's message queue.
/// **Everything it does is an atomic and a `PostMessageW`**, for the reason given in the module
/// documentation: a `WinEvent` callback is as much in the system's way as a hook callback is.
///
/// The buffer cannot be flushed from here even if it were free to be: the buffer is a
/// thread-local of the *input* thread (section 6.3), so the only correct thing to do with the
/// event is to hand its timestamp to that thread, which is what happens.
///
/// # Safety
///
/// Called by the OS with the arguments of a `WinEvent`. Every argument is read by value, none is
/// dereferenced — the window handle is not even looked at — so the caller owes this function
/// nothing. Nothing in the body can panic, which is what keeps the `extern "system"` boundary
/// sound: there is no allocation, no indexing, no `unwrap` and no arithmetic that can overflow.
unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    _window: HWND,
    _object_id: i32,
    _child_id: i32,
    _thread_id: u32,
    event_time: u32,
) {
    if !is_flush_event(event) {
        // A subscription is per event code, so this cannot happen; it is here because a callback
        // the system drives is the wrong place to assume anything.
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

    /// The two events of the FR-10 table and nothing else. `EVENT_SYSTEM_DESKTOPSWITCH` is task
    /// T-06-2 and must not be answered here yet.
    #[test]
    fn the_subscription_list_is_the_two_rows_of_fr10() {
        assert_eq!(FLUSH_EVENTS, [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_FOCUS]);
        assert!(is_flush_event(EVENT_SYSTEM_FOREGROUND));
        assert!(is_flush_event(EVENT_OBJECT_FOCUS));

        // 0x0020 — `EVENT_SYSTEM_DESKTOPSWITCH`, written out rather than imported so that adding
        // the constant to the list is a deliberate act and not an import away.
        assert!(!is_flush_event(0x0020));
    }
}
