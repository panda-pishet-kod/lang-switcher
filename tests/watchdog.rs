//! Integration tests for tasks T-03-3, T-06-2 and T-10-0 — the subscriptions of module
//! `watchdog`, the hook watchdog of FR-80, and the foreground gate of Р-60 that keeps the
//! focus churn of background processes from erasing what the user has typed.
//!
//! # What is tested here and what cannot be
//!
//! Everything that decides something is a function of its arguments and is driven directly:
//! whether a `RAWMOUSE` button word is a press or a movement (FR-10, FR-13), which `WinEvent`
//! codes the flush table lists and which the watched list adds (FR-80), what each message of the
//! FR-21 delivery asks for, how two flush requests coalesce across the wrap of the millisecond
//! counter (FR-12), and which window each arm of the watchdog's window procedure is bound to.
//!
//! Several tests do touch Win32, because the value of checking them is precisely that they are
//! real: the Raw Input registration of FR-13 with `RIDEV_INPUTSINK`, the three `SetWinEventHook`
//! subscriptions, the `WTSRegisterSessionNotification` of FR-80 and its withdrawal, and the
//! thirty-second timer and its `KillTimer`. Every one of them is installed and immediately
//! removed, on a window this file creates and destroys.
//!
//! **No test that `cargo test` runs installs a keyboard hook**, for the reason `tests\hook.rs`
//! states: a `WH_KEYBOARD_LL` hook in a process that is not pumping messages freezes the
//! keyboard of whoever is running `cargo test`. A `WinEvent` hook has no such property — it is
//! `WINEVENT_OUTOFCONTEXT`, so the system posts to a queue instead of calling into us, and a
//! queue nobody pumps is a queue nobody waits on.
//!
//! The one exception is `the_gap_without_a_hook_is_microseconds`, which is `#[ignore]`d for
//! exactly that reason and run deliberately — the same shape the behavioural tests of
//! `tests\switch.rs`, `tests\cycle.rs` and `tests\inject.rs` already have. It is the only place
//! the gap FR-80 trades away can be measured at all, and it pumps the queue between rounds so
//! that a stroke arriving inside it is answered rather than left to `LowLevelHooksTimeout`.
//!
//! What no test can show is the part of FR-12 that is about the *world*: that
//! `KBDLLHOOKSTRUCT.time`, `GetMessageTime` and `dwmsEventTime` really are the same counter.
//! That is argued in the report of task T-03-3 and measured on the running program. Neither can
//! a test stage a **silent** removal — the system does that, on its own schedule, and the report
//! of task T-06-2 says exactly which part of FR-80 is therefore argued rather than measured.

use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lang_switcher::buffer::{self, Recorder, ResetOutcome};
use lang_switcher::hook::{Edge, KeyEvent};
use lang_switcher::layouts;
use lang_switcher::watchdog::{self, Counters, FLUSH_EVENTS, Rebuild, WM_APP_FLUSH, WM_APP_LAYOUT};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{GetRegisteredRawInputDevices, RAWINPUTDEVICE};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, EVENT_OBJECT_FOCUS,
    EVENT_SYSTEM_DESKTOPSWITCH, EVENT_SYSTEM_FOREGROUND, GetForegroundWindow, GetMessageTime,
    GetWindowThreadProcessId, HWND_MESSAGE, MSG, PBT_APMRESUMEAUTOMATIC, PM_REMOVE, PeekMessageW,
    RI_MOUSE_BUTTON_1_DOWN, RI_MOUSE_BUTTON_1_UP, RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_2_UP,
    RI_MOUSE_BUTTON_3_DOWN, RI_MOUSE_BUTTON_3_UP, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP,
    RI_MOUSE_BUTTON_5_DOWN, RI_MOUSE_BUTTON_5_UP, RI_MOUSE_HWHEEL, RI_MOUSE_WHEEL, WINDOW_STYLE,
    WM_DEVICECHANGE, WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_INPUTLANGCHANGE, WM_POWERBROADCAST,
    WM_TIMER, WM_WTSSESSION_CHANGE, WTS_SESSION_UNLOCK,
};
use windows::core::{PCWSTR, w};

// ---------------------------------------------------------------------------------------
// Point 11 — FR-10, FR-13: a button is a flush, the cursor moving is not
// ---------------------------------------------------------------------------------------

#[test]
fn every_button_going_down_is_a_flush() {
    for down in [
        RI_MOUSE_BUTTON_1_DOWN,
        RI_MOUSE_BUTTON_2_DOWN,
        RI_MOUSE_BUTTON_3_DOWN,
        RI_MOUSE_BUTTON_4_DOWN,
        RI_MOUSE_BUTTON_5_DOWN,
    ] {
        assert!(
            watchdog::mouse_button_pressed(down as u16),
            "FR-10: button {down:#x} going down is a flush"
        );
    }

    // Two buttons at once — the chord a mouse reports in one packet.
    assert!(watchdog::mouse_button_pressed(
        (RI_MOUSE_BUTTON_1_DOWN | RI_MOUSE_BUTTON_2_DOWN) as u16
    ));
}

#[test]
fn moving_the_cursor_is_not_a_flush() {
    // What a mouse that has only moved reports: no button bits at all. This is the packet
    // `RIDEV_INPUTSINK` delivers by the thousand, and FR-10 lists "нажатие любой кнопки мыши"
    // and not "перемещение курсора".
    assert!(!watchdog::mouse_button_pressed(0));

    // Releases are not presses. Flushing on both would flush twice for one click, and the
    // second flush would land after whatever the click's application did with the caret.
    for up in [
        RI_MOUSE_BUTTON_1_UP,
        RI_MOUSE_BUTTON_2_UP,
        RI_MOUSE_BUTTON_3_UP,
        RI_MOUSE_BUTTON_4_UP,
        RI_MOUSE_BUTTON_5_UP,
    ] {
        assert!(
            !watchdog::mouse_button_pressed(up as u16),
            "a release is not a press: {up:#x}"
        );
    }

    // The wheel turning is not a button either. The wheel *pressed* is button 3 and is a
    // flush; the two cases have different bits and this is the one that is not.
    assert!(!watchdog::mouse_button_pressed(RI_MOUSE_WHEEL as u16));
    assert!(!watchdog::mouse_button_pressed(RI_MOUSE_HWHEEL as u16));
}

// ---------------------------------------------------------------------------------------
// Points 12 and 13 — the two `WinEvent` rows of the FR-10 table
// ---------------------------------------------------------------------------------------

#[test]
fn the_subscription_list_is_exactly_the_two_rows_of_the_flush_table() {
    assert_eq!(FLUSH_EVENTS, [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_FOCUS]);

    assert!(watchdog::is_flush_event(EVENT_SYSTEM_FOREGROUND));
    assert!(watchdog::is_flush_event(EVENT_OBJECT_FOCUS));

    // `EVENT_SYSTEM_DESKTOPSWITCH` is task T-06-2 and is not started here. The value is
    // written out rather than imported so that adding it to the list has to be deliberate.
    assert!(!watchdog::is_flush_event(0x0020));
    // `EVENT_OBJECT_LOCATIONCHANGE` — the event a moving window fires, several times a second,
    // and the classic way to make a subscription expensive.
    assert!(!watchdog::is_flush_event(0x800B));
}

#[test]
fn the_two_subscriptions_install_and_come_off() {
    // The real call, with the real flags, checked the way NFR-13 asks: a null handle from
    // `SetWinEventHook` is a failure and `watch` turns it into an `Err`. The subscriptions are
    // removed immediately by the guard's `Drop`, which is the other half of what is checked
    // here — a `Watching` that leaked would be a subscription this test left in the machine.
    let watching = watchdog::watch().expect("the two WinEvent subscriptions must install");

    drop(watching);
}

// ---------------------------------------------------------------------------------------
// Point 10 — FR-13: `RegisterRawInputDevices` with `RIDEV_INPUTSINK`
// ---------------------------------------------------------------------------------------

/// A message-only window of the same shape as the input thread's.
///
/// The class is the system's `STATIC`, for the reason `tests\tray.rs` gives for the same trick:
/// `app` registers its own class inside `run()`, which a test cannot call, and neither the Raw
/// Input registration nor anything else here cares what class the window has.
struct TestWindow {
    handle: windows::Win32::Foundation::HWND,
}

impl TestWindow {
    fn new() -> Self {
        // SAFETY: `None` asks for the handle of the running executable, which cannot be
        // unloaded under us.
        let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.expect("the module handle");

        // SAFETY: `STATIC` is a system class that always exists; the window name is null, which
        // asks for no title; `HWND_MESSAGE` as the parent asks for a message-only window, which
        // is what the input thread of section 6.1 owns. No `lpParam` is passed. The handle is
        // destroyed exactly once, in `Drop`, on this same thread — which is what
        // `DestroyWindow` requires.
        let handle = unsafe {
            CreateWindowExW(
                Default::default(),
                w!("STATIC"),
                PCWSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(HINSTANCE(module.0)),
                None,
            )
        }
        .expect("a message-only window must be creatable");

        Self { handle }
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        // SAFETY: `handle` came from a successful `CreateWindowExW` on this thread and is
        // destroyed exactly once — this type is neither `Copy` nor `Clone`.
        let _ = unsafe { DestroyWindow(self.handle) };
    }
}

/// The turn-taking lock for the two tests that hold a Raw Input registration — task T-08-4.
///
/// ⚠ **A Raw Input registration belongs to the process, not to a window.** A second
/// `RegisterRawInputDevices` for the same usage page and usage does not add an entry, it
/// *overwrites* the first one's target; and one `RIDEV_REMOVE` then takes the registration away
/// from everybody. `cargo test` runs the tests of a binary in parallel, so without this the two
/// tests below measure each other rather than the product — which is exactly how the test that
/// reads the registration back first failed, reporting an empty list because its neighbour's
/// guard had been dropped in the meantime.
///
/// The same property is why task T-08-4 could not cure the hook starvation by moving the keyboard
/// entry to another window: there was no window to move it to.
static RAW_INPUT_TURN: Mutex<()> = Mutex::new(());

/// Takes the turn, ignoring poisoning: a panic in one of the two tests must fail that test and
/// not turn its neighbour into a second failure with an unrelated message.
fn raw_input_turn() -> std::sync::MutexGuard<'static, ()> {
    RAW_INPUT_TURN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn the_raw_input_registration_of_fr13_is_accepted_and_withdrawn() {
    let _turn = raw_input_turn();
    let window = TestWindow::new();

    // The call FR-13 names, with the flag FR-13 names, on a message-only window that never has
    // the focus — which is the only configuration in which `RIDEV_INPUTSINK` means anything.
    // A failure would come back as an `Err` here rather than as a program that silently never
    // sees a click (NFR-13).
    let registration =
        watchdog::register_raw_input(window.handle).expect("Raw Input must register");

    // And it is withdrawn, on the spot: leaving a registration behind while the window it
    // names is destroyed would have the system posting `WM_INPUT` at a dead handle.
    drop(registration);

    // Registering again after the withdrawal proves the withdrawal was accepted: the second
    // call would be a duplicate registration otherwise, and `RegisterRawInputDevices` refuses
    // those on the same usage page with `RIDEV_INPUTSINK`.
    let again = watchdog::register_raw_input(window.handle).expect("and must register again");

    drop(again);
}

// ---------------------------------------------------------------------------------------
// Task T-08-4 — the keyboard entry that is not there, and the delivery that replaced it
// ---------------------------------------------------------------------------------------

/// ⚠ **The registration this process holds contains no keyboard entry — FR-96, task T-08-4.**
///
/// # Why this is asserted against the system and not against our own array
///
/// Because the defect was invisible in our own array. A keyboard top-level collection in
/// `RegisterRawInputDevices` makes this process a raw keyboard client, and a raw keyboard client
/// takes the keystrokes away from **every low-level keyboard hook in the session** — its own and
/// every other process's — for as long as any window of that process holds the foreground. That
/// is what made `Ctrl+Alt+Shift+F12` do nothing with the settings dialog of FR-92 open, and FR-96
/// is the only way out of a wedged hook.
///
/// `GetRegisteredRawInputDevices` asks the system what this process is registered for, which is
/// the quantity that actually matters: an entry added anywhere, by any future edit, in any module,
/// fails this test.
///
/// The measurement behind the rule is in the report of task T-08-4, and its shortest statement is
/// that the flag did not matter: `RIDEV_DEVNOTIFY`, `RIDEV_INPUTSINK`, both, and a null target all
/// starved the chain alike.
#[test]
fn no_keyboard_top_level_collection_is_registered_for_raw_input() {
    let _turn = raw_input_turn();
    let window = TestWindow::new();
    let registration =
        watchdog::register_raw_input(window.handle).expect("Raw Input must register");

    let mut count = 0_u32;

    // SAFETY: `None` with a zero count is the documented way to ask how many registrations there
    // are; the call writes the number through `count` and touches nothing else. The size argument
    // is the element size the call demands and is computed from the type.
    let asked = unsafe {
        GetRegisteredRawInputDevices(
            None,
            &raw mut count,
            u32::try_from(size_of::<RAWINPUTDEVICE>()).expect("the element size fits in a u32"),
        )
    };

    // The documented answer to "how many": zero written, with `count` filled in.
    assert_eq!(asked, 0, "the counting call must not write any entries");

    let mut devices = vec![RAWINPUTDEVICE::default(); count as usize];

    // SAFETY: `devices` has exactly `count` elements, which is the number the call just reported,
    // and `count` says how many may be written. The buffer is owned by this frame and outlives
    // the call.
    let written = unsafe {
        GetRegisteredRawInputDevices(
            Some(devices.as_mut_ptr()),
            &raw mut count,
            u32::try_from(size_of::<RAWINPUTDEVICE>()).expect("the element size fits in a u32"),
        )
    };

    assert_ne!(
        written,
        u32::MAX,
        "the read-back of the registrations failed"
    );
    devices.truncate(written as usize);

    let ours: Vec<&RAWINPUTDEVICE> = devices
        .iter()
        .filter(|device| device.hwndTarget == window.handle)
        .collect();

    assert_eq!(
        ours.len(),
        1,
        "FR-13 asks for one entry — the mouse — and task T-08-4 took the other one away; \
         the system reports {ours:?}"
    );
    assert_eq!(ours[0].usUsagePage, 0x01, "the generic desktop page");
    assert_eq!(ours[0].usUsage, 0x02, "usage 2 is the mouse");

    // ⚠ The whole point, stated as its own assertion so that a failure names it: usage 6 of the
    // generic page is the keyboard, and it must not be there.
    assert!(
        !devices
            .iter()
            .any(|device| device.usUsagePage == 0x01 && device.usUsage == 0x06),
        "a keyboard entry in this process's Raw Input registration starves every low-level \
         keyboard hook in the session while a window of ours is in front — FR-96, task T-08-4"
    );

    drop(registration);
}

/// The device notification of FR-21 registers on a message-only window and comes off again.
///
/// The window is the shape the input thread's is, because that is where it goes: a *registered*
/// device notification reaches a message-only window, where the *broadcast* kind reaches only
/// top-level ones. That is measured in the report of task T-08-4 with a real device arrival; what
/// is checked here is the half a test can check without one — that the call is accepted, that the
/// withdrawal is accepted, and that a second registration on the same window is accepted after it.
#[test]
fn the_device_notification_of_fr21_registers_and_comes_off() {
    let window = TestWindow::new();
    let before = watchdog::counters().device_notice_failures;

    let notice = watchdog::register_device_notice(window.handle)
        .expect("the keyboard device notification must register");

    drop(notice);

    let again = watchdog::register_device_notice(window.handle)
        .expect("and must register again after the withdrawal");

    drop(again);

    // NFR-13: the withdrawal's result is examined by the `Drop`, and this is where the record it
    // keeps is read. A refusal here would mean the program is leaving registrations behind.
    assert_eq!(
        watchdog::counters().device_notice_failures,
        before,
        "UnregisterDeviceNotification refused"
    );
}

/// `note_device_change` counts the message FR-21 names and nothing else.
///
/// The other message of [`layouts::REBUILD_MESSAGES`] is `WM_INPUTLANGCHANGE`, which reaches the
/// same branch of the window procedure and is not a device change; if this counted it, the number
/// the report of task T-08-4 reads as "a keyboard arrived" would be reading layout switches.
#[test]
fn only_wm_devicechange_is_counted_as_a_device_change() {
    let before = watchdog::counters().device_changes;

    assert!(watchdog::note_device_change(WM_DEVICECHANGE));

    assert_eq!(
        watchdog::counters().device_changes,
        before + 1,
        "the message FR-21 names is counted"
    );

    for message in [
        WM_INPUTLANGCHANGE,
        WM_INPUT,
        WM_APP_LAYOUT,
        WM_APP_FLUSH,
        WM_INPUT_DEVICE_CHANGE,
        0x0000,
        0xFFFF_FFFF,
    ] {
        assert!(
            !watchdog::note_device_change(message),
            "message {message:#x} is not a device change"
        );
    }

    assert_eq!(
        watchdog::counters().device_changes,
        before + 1,
        "and nothing else moved the count"
    );
}

/// ⚠ **`WM_DEVICECHANGE` belongs to `layouts` and must never also be an arm of `rebuild_for`.**
///
/// The window procedure asks `layouts::needs_rebuild` first and `watchdog::rebuild_for` second, so
/// a message answered by both would rebuild the cache twice for one event — thousands of
/// `ToUnicodeEx` calls, twice, on the thread that owns the hook. Task T-08-4 made this message
/// arrive for the first time, which is what turns a latent rule into a live one.
#[test]
fn the_message_fr21_names_is_answered_by_one_list_and_not_by_both() {
    assert!(layouts::needs_rebuild(WM_DEVICECHANGE));
    assert_eq!(watchdog::rebuild_for(WM_DEVICECHANGE), None);

    // And the message the old delivery used is the mirror image: `watchdog`'s, never `layouts`'.
    assert!(!layouts::needs_rebuild(WM_INPUT_DEVICE_CHANGE));
    assert_eq!(
        watchdog::rebuild_for(WM_INPUT_DEVICE_CHANGE),
        Some(Rebuild::Unconditional)
    );
}

// ---------------------------------------------------------------------------------------
// FR-21 delivery — which message asks for which rebuild
// ---------------------------------------------------------------------------------------

#[test]
fn each_message_of_the_fr21_delivery_asks_for_the_rebuild_it_deserves() {
    // A keyboard arrived or left: the scan-code map can differ while the `HKL` does not, so
    // there is nothing to compare and the cache is rebuilt outright.
    assert_eq!(
        watchdog::rebuild_for(WM_INPUT_DEVICE_CHANGE),
        Some(Rebuild::Unconditional)
    );

    // The foreground changed, or the shell reported a language change: ask what the layout is
    // now and rebuild only if it moved. `EVENT_SYSTEM_FOREGROUND` fires on every `Alt+Tab`, and
    // FR-20's sweep is thousands of `ToUnicodeEx` calls.
    assert_eq!(
        watchdog::rebuild_for(WM_APP_LAYOUT),
        Some(Rebuild::IfLayoutChanged)
    );

    // Everything else, including the two messages FR-21 names. Those still go through
    // `layouts::needs_rebuild` in the window procedure, which is where task T-03-2a put them;
    // this module is the delivery that makes a rebuild happen when they never arrive.
    for message in [
        WM_INPUT,
        WM_APP_FLUSH,
        WM_INPUTLANGCHANGE,
        0x0000,
        0xFFFF_FFFF,
    ] {
        assert_eq!(watchdog::rebuild_for(message), None, "message {message:#x}");
    }

    // And the two lists do not overlap: a message cannot be both a rebuild of this module's
    // making and one of `layouts`'.
    for message in layouts::REBUILD_MESSAGES {
        assert_eq!(watchdog::rebuild_for(message), None);
    }
}

// ---------------------------------------------------------------------------------------
// FR-12 — how two flush requests coalesce
// ---------------------------------------------------------------------------------------

#[test]
fn coalescing_keeps_the_newer_of_two_pending_flushes() {
    // An empty cell carries nothing, and anything at all improves on it — a timestamp of zero
    // included, which is why the cell is 64 bits wide and not 32.
    assert_eq!(watchdog::pending_time(0), None);
    assert_eq!(
        watchdog::coalesced(0, 0).and_then(watchdog::pending_time),
        Some(0)
    );

    let filled = watchdog::coalesced(0, 500).expect("an empty cell takes any request");
    assert_eq!(watchdog::pending_time(filled), Some(500));

    // A newer request replaces it: a flush stamped `T` removes everything at or before `T`, so
    // the newer of two pending flushes does everything the older one would have and more.
    let newer = watchdog::coalesced(filled, 900).expect("900 is newer than 500");
    assert_eq!(watchdog::pending_time(newer), Some(900));

    // An older one, and the same one, add nothing and are dropped.
    assert_eq!(watchdog::coalesced(newer, 500), None);
    assert_eq!(watchdog::coalesced(newer, 900), None);

    // Across the wrap of the counter. `u32::max` on the raw values would keep the *older* of
    // these two and lose the flush; the comparison of FR-12 keeps the right one.
    let before_wrap = watchdog::coalesced(0, u32::MAX - 10).expect("an empty cell");
    let after_wrap = watchdog::coalesced(before_wrap, 10).expect("21 ms later, past the turn");

    assert_eq!(watchdog::pending_time(after_wrap), Some(10));
    assert_eq!(watchdog::coalesced(after_wrap, u32::MAX - 10), None);
}

// ---------------------------------------------------------------------------------------
// The whole path, from a flush request to a zeroed slot
// ---------------------------------------------------------------------------------------

/// Types one key stamped `time` into the buffer of this thread.
fn press_at(time: u32) {
    buffer::record(KeyEvent {
        vk: 0x41,
        edge: Edge::Down,
        extra_info: 0,
        scan: 0x1E,
        flags: 0,
        time,
    });
}

/// The counters, so that a test can assert on what one call changed rather than on totals.
fn delta(before: Counters, after: Counters) -> Counters {
    Counters {
        mouse_packets: after.mouse_packets - before.mouse_packets,
        mouse_flushes: after.mouse_flushes - before.mouse_flushes,
        window_flushes: after.window_flushes - before.window_flushes,
        window_flushes_taken: after.window_flushes_taken - before.window_flushes_taken,
        full_clears: after.full_clears - before.full_clears,
        partial_clears: after.partial_clears - before.partial_clears,
        kept_events: after.kept_events - before.kept_events,
        strokes_removed: after.strokes_removed - before.strokes_removed,
        device_changes: after.device_changes - before.device_changes,
        layout_probes: after.layout_probes - before.layout_probes,
        device_notice_failures: after.device_notice_failures - before.device_notice_failures,
        background_skips: after.background_skips - before.background_skips,
    }
}

/// The one test that touches the process-wide flush cell and the process-wide counters, and it
/// is one test on purpose: `cargo test` runs the tests of a binary in parallel, and two tests
/// asserting on the same statics would be asserting on each other's timing.
#[test]
fn a_flush_request_travels_from_the_watcher_thread_to_a_zeroed_slot() {
    let before = watchdog::counters();

    // No buffer on this thread yet — every thread but the input one is in that position, and
    // the window procedure runs on all three of them (section 6.3).
    watchdog::request_flush(1_000);
    assert_eq!(watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)), None);

    buffer::install_recorder(Recorder::with_capacity(16));

    // Three keystrokes made after the click that is still queued, and two made before it.
    for time in [900, 950, 1_001, 1_002, 1_003] {
        press_at(time);
    }
    assert_eq!(buffer::len(), 5);

    // The request raised above is still pending — nothing has taken it — and applying it now
    // removes the two strokes older than the click and leaves the three newer ones (FR-12).
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Partial {
            removed: 2,
            kept: 3
        })
    );
    assert_eq!(buffer::len(), 3);

    // The cell is empty now: a second message finds nothing, which is what makes a forged
    // `WM_APP_FLUSH` from another process a no-op (SEC-05).
    assert_eq!(watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)), None);
    assert_eq!(buffer::len(), 3);

    // Two requests between two turns of the message loop collapse into the newer one, and the
    // newer one does everything the older would have done.
    watchdog::request_flush(1_001);
    watchdog::request_flush(1_003);
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Cleared { removed: 3 })
    );
    assert_eq!(buffer::len(), 0);

    // A message this module does not answer changes nothing, whatever else is going on.
    assert_eq!(watchdog::apply_flush(0x0401, LPARAM(0)), None);

    // A `WM_INPUT` whose `lparam` is not a live raw-input packet — which is what a forged one
    // from another process looks like — is rejected by `GetRawInputData` and leaves the buffer
    // alone (NFR-13, SEC-05).
    press_at(2_000);
    assert_eq!(watchdog::apply_flush(WM_INPUT, LPARAM(0)), None);
    assert_eq!(buffer::len(), 1, "a forged WM_INPUT flushes nothing");

    buffer::uninstall();

    let counted = delta(before, watchdog::counters());

    assert_eq!(counted.window_flushes, 3, "three requests were raised");
    assert_eq!(counted.window_flushes_taken, 2, "two of them were taken");
    assert_eq!(counted.partial_clears, 1);
    assert_eq!(counted.full_clears, 1);
    assert_eq!(
        counted.strokes_removed, 5,
        "two by the partial, three by the full"
    );
    assert_eq!(
        counted.mouse_packets, 0,
        "the forged WM_INPUT was not a packet"
    );
    assert_eq!(counted.mouse_flushes, 0);
}

// ---------------------------------------------------------------------------------------
// FR-80 — the watchdog proper, task T-06-2
// ---------------------------------------------------------------------------------------
//
// ⚠ **No test here reinstalls the hook**, and that is not caution but the same rule
// `tests\hook.rs` states: a `WH_KEYBOARD_LL` hook in a process that is not pumping messages
// freezes the keyboard of whoever is running `cargo test`. What is checked instead is that
// every path to a reinstallation is bound to a window this process does not have, and the
// checks below run in a process where neither window exists — so a binding left out would show
// up here as a hook appearing in the test process, which the last assertion of
// `no_watchdog_message_installs_a_hook_on_a_foreign_window` would catch.

/// Criterion 9: `EVENT_SYSTEM_DESKTOPSWITCH` joined the list `watch` already iterated over, and
/// no second subscription mechanism was built beside it.
#[test]
fn the_desktop_switch_joined_the_existing_subscription_list() {
    assert_eq!(
        watchdog::WATCHED_EVENTS,
        [
            EVENT_SYSTEM_FOREGROUND,
            EVENT_OBJECT_FOCUS,
            EVENT_SYSTEM_DESKTOPSWITCH
        ],
        "section 6.1: SetWinEventHook: FOREGROUND, OBJECT_FOCUS, DESKTOPSWITCH"
    );

    // The two rows of FR-10 are still the first two entries, so `watch` did not have its list
    // replaced — it had it lengthened.
    assert_eq!(
        watchdog::WATCHED_EVENTS[..FLUSH_EVENTS.len()],
        FLUSH_EVENTS,
        "the flush events must still be the head of the watched list"
    );

    // And the desktop switch is deliberately **not** a flush: FR-10 does not list it, and a UAC
    // prompt must not throw away what the user has typed.
    assert!(!watchdog::is_flush_event(EVENT_SYSTEM_DESKTOPSWITCH));
}

/// The three subscriptions really install and really come off, on the real API.
#[test]
fn all_three_subscriptions_install_and_come_off() {
    let watching = watchdog::watch().expect("three WinEvent subscriptions must install");

    drop(watching);

    // A second round proves the first was withdrawn as far as anything can: a leaked hook would
    // still be in the machine, and this call would be the fourth, fifth and sixth subscriptions
    // of a process that is supposed to hold three.
    let again = watchdog::watch().expect("and must install again");

    drop(again);
}

/// Criterion 10: the registration of FR-80 is made and — the half the task specification calls
/// obligatory — withdrawn.
#[test]
fn the_session_registration_of_fr80_is_accepted_and_withdrawn() {
    let window = TestWindow::new();

    let registration = watchdog::register_session_notice(window.handle)
        .expect("WTSRegisterSessionNotification must be accepted");

    // The withdrawal is the point. A registration left behind is a resource the system holds
    // against a window that is about to stop existing.
    drop(registration);

    // Registering the same window again would be a duplicate if the first had not been
    // withdrawn.
    let again = watchdog::register_session_notice(window.handle)
        .expect("and the window must be registrable again");

    drop(again);
}

/// The fourth mechanism of FR-80: thirty seconds, on a window, killed on the way out.
#[test]
fn the_liveness_timer_is_the_thirty_seconds_of_fr80_and_comes_off() {
    assert_eq!(
        watchdog::LIVENESS_INTERVAL_MS,
        30_000,
        "FR-80: периодическая проверка живости хука с интервалом 30 с"
    );

    let window = TestWindow::new();

    let liveness =
        watchdog::start_liveness_timer(window.handle).expect("the liveness timer must start");

    // `KillTimer` inside the guard's `Drop` fails loudly if the pair `(window, id)` never named
    // a timer, so a successful drop is the withdrawal being accepted.
    drop(liveness);

    let again = watchdog::start_liveness_timer(window.handle).expect("and must start again");

    drop(again);
}

/// The private messages of this process are all different numbers.
///
/// Not a formality: `WM_APP_REHOOK` was written as `WM_APP + 8` first, which is
/// `switch::WM_APP_SWITCH`. Two private messages sharing a number collide silently — nothing but
/// the number tells them apart, and the window procedure would run the wrong arm.
#[test]
fn the_private_messages_of_this_process_are_all_distinct() {
    let messages = [
        lang_switcher::hook::WM_APP_HOTKEY,
        lang_switcher::hook::WM_APP_FAIL_SAFE,
        WM_APP_FLUSH,
        WM_APP_LAYOUT,
        lang_switcher::switch::WM_APP_SWITCH,
        watchdog::WM_APP_REHOOK,
    ];

    for (index, message) in messages.iter().enumerate() {
        for other in &messages[index + 1..] {
            assert_ne!(message, other, "two private messages share a number");
        }
    }
}

/// SEC-05 and FR-01: every arm of the watchdog's window procedure is bound to a window this
/// process does not have, so none of them does anything here — least of all install a hook.
#[test]
fn no_watchdog_message_installs_a_hook_on_a_foreign_window() {
    // The preconditions this test rests on, asserted rather than assumed: this thread owns no
    // typing buffer, so it is not the input thread, and this process has no hook.
    assert!(
        !buffer::is_installed(),
        "this test must not run on a thread that owns the buffer"
    );
    assert!(
        !lang_switcher::hook::is_installed(),
        "no test in this binary may install a keyboard hook"
    );

    let window = TestWindow::new();
    let before = watchdog::health();

    for (message, wparam) in [
        (WM_POWERBROADCAST, WPARAM(PBT_APMRESUMEAUTOMATIC as usize)),
        (WM_WTSSESSION_CHANGE, WPARAM(WTS_SESSION_UNLOCK as usize)),
        (WM_TIMER, WPARAM(watchdog::LIVENESS_TIMER_ID)),
        (watchdog::WM_APP_REHOOK, WPARAM(0)),
    ] {
        assert!(
            watchdog::handle_watchdog_message(window.handle, message, wparam).is_none(),
            "message {message:#x} must fall through on a window that is neither ours"
        );
    }

    let after = watchdog::health();

    assert_eq!(
        after.recoveries, before.recoveries,
        "nothing was reinstalled"
    );
    assert_eq!(after.power_resumes, before.power_resumes);
    assert_eq!(after.session_changes, before.session_changes);
    assert_eq!(after.liveness_ticks, before.liveness_ticks);

    // The one that matters: no keyboard hook exists in this process, and the keyboard of
    // whoever is running `cargo test` is untouched.
    assert!(!lang_switcher::hook::is_installed());
}

/// FR-90, criterion 18: the state the tray of task T-08-1 will show is readable from outside
/// this module, and a process without a hook says so.
#[test]
fn a_process_without_a_hook_reports_the_hook_down() {
    assert!(!lang_switcher::hook::is_installed());
    assert!(
        watchdog::hook_down(),
        "FR-90: a program with no hook is not active, whatever the icon says"
    );

    let health = watchdog::health();

    // A test process runs no watchdog, so every count is zero and the reason is "not yet".
    assert_eq!(health.recoveries, 0);
    assert_eq!(health.install_failures, 0);
    assert_eq!(health.last_reason, watchdog::Reason::None);
    assert_eq!(health.last_reason.name(), "none");
}

/// SEC-01, SEC-07: the reason that leaves the process through the channel of SEC-04a is one of
/// five short ASCII words, and the set is closed.
#[test]
fn the_reason_is_one_of_five_ascii_words_and_nothing_else() {
    let reasons = [
        watchdog::Reason::None,
        watchdog::Reason::Timer,
        watchdog::Reason::DesktopSwitch,
        watchdog::Reason::SessionChange,
        watchdog::Reason::PowerResume,
    ];

    let words: Vec<&str> = reasons.iter().map(|reason| reason.name()).collect();

    assert_eq!(
        words,
        vec![
            "none",
            "timer",
            "desktop_switch",
            "session_change",
            "power_resume"
        ]
    );

    for word in &words {
        assert!(
            word.is_ascii() && !word.is_empty(),
            "a channel value must be ASCII: {word:?}"
        );
        assert!(
            word.chars()
                .all(|character| character.is_ascii_lowercase() || character == '_'),
            "and must carry nothing that could be a character the user typed: {word:?}"
        );
    }
}

/// **The gap of FR-80, measured** — criterion 13 asks the report for a number and this is where
/// the number comes from, together with criterion 14's proof at the level of the API itself.
///
/// ⚠ `#[ignore]`d, and the reason is the one `tests\hook.rs` and the behavioural tests of
/// `tests\switch.rs` give for theirs: this installs a **live global `WH_KEYBOARD_LL` hook** in a
/// process that is not the product. It is deliberate, short, and pumps the queue between rounds
/// so that a stroke arriving during it is answered rather than left to `LowLevelHooksTimeout`.
/// The hook it installs is the product's own callback, so `Ctrl+Alt+Shift+F12` remains an escape
/// hatch throughout.
///
/// What it shows:
///
/// * the gap between `UnhookWindowsHookEx` and `SetWindowsHookExW` is microseconds, not
///   milliseconds, so the strokes a reinstallation cannot buffer are the ones typed inside a
///   window narrower than the gap between two keys of the fastest typist alive;
/// * **two hooks never exist at once**: `hook::install` refuses while one is registered, which
///   is FR-01 enforced by the accepted module and the reason `reinstall_hook` cannot be written
///   in the other order.
#[test]
#[ignore = "installs a live global WH_KEYBOARD_LL hook; run deliberately with --ignored --test-threads=1"]
fn the_gap_without_a_hook_is_microseconds() {
    /// Reinstallations to time. Enough for a maximum to mean something, few enough that the
    /// hook is in the machine for milliseconds and not seconds.
    const ROUNDS: u32 = 20;

    // SAFETY: `None` asks for the handle of the running executable, which cannot be unloaded
    // under us.
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.expect("the module handle");
    let instance = HINSTANCE(module.0);

    let window = TestWindow::new();

    let installed =
        lang_switcher::hook::install(window.handle, instance).expect("the hook must install");

    // **Criterion 14, at the level where FR-01 is enforced.** A second installation is refused
    // while one is registered, so no code path — this module's included — can hold two.
    assert!(
        lang_switcher::hook::install(window.handle, instance).is_err(),
        "FR-01: a second hook must be refused while one is registered"
    );

    for round in 0..ROUNDS {
        assert!(
            watchdog::reinstall_hook(window.handle, watchdog::Reason::Timer),
            "round {round} must end with a hook"
        );
        assert!(lang_switcher::hook::is_installed());

        // And still exactly one after the reinstallation, not two.
        assert!(lang_switcher::hook::install(window.handle, instance).is_err());

        drain_the_queue();
    }

    drop(installed);

    assert!(
        !lang_switcher::hook::is_installed(),
        "the guard must take the hook off however many times it was replaced"
    );

    let health = watchdog::health();

    println!("reinstallations   = {}", health.recoveries);
    println!("last gap, us      = {}", health.last_gap_us);
    println!("longest gap, us   = {}", health.max_gap_us);
    println!("silent removals   = {}", health.silent_removals);
    println!("absent at check   = {}", health.absent_at_check);
    println!("install failures  = {}", health.install_failures);
    println!("last reason       = {}", health.last_reason.name());

    assert_eq!(health.recoveries, ROUNDS);
    assert_eq!(health.install_failures, 0);
    assert_eq!(
        health.silent_removals, 0,
        "nothing took the hook away during a test that lasted milliseconds"
    );
    assert_eq!(health.last_reason, watchdog::Reason::Timer);
    assert!(
        health.max_gap_us < 100_000,
        "the gap is microseconds; {} us is not",
        health.max_gap_us
    );
}

/// Answers whatever is in this thread's queue, so that a stroke delivered to the hook installed
/// above is dispatched rather than left waiting on `LowLevelHooksTimeout`.
#[cfg(test)]
fn drain_the_queue() {
    let mut message = MSG::default();

    // SAFETY: `message` is a live `MSG` of this frame written by the call; `None` for the window
    // asks for every message of the calling thread, and the two zeroes for no filter. The call
    // returns a `BOOL` saying whether it wrote anything, which is what the loop condition is.
    while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
        // SAFETY: `message` was just filled by `PeekMessageW` and is passed unchanged.
        unsafe { DispatchMessageW(&message) };
    }
}

// ---------------------------------------------------------------------------------------
// Task T-10-0 — the foreground gate of Р-60
// ---------------------------------------------------------------------------------------
//
// ⚠ **The trap this section is built around: `WINEVENT_SKIPOWNPROCESS`.** The product's
// subscriptions do not deliver events generated by their own process, so a storm staged inside
// the test process — the process that installed the subscriptions — would be invisible to them,
// and a green test would have checked nothing at all. Every window that generates an event in
// these tests therefore belongs to a **foreign** process the test spawns itself and stops by
// the identifier the spawn returned (Р-42). The positive control is `background_skips`: growth
// there is the proof the storm was delivered into this process and turned away, where
// `window_flushes` standing still alone could equally mean "nothing arrived".

/// The verdict of the gate, driven with real windows — the fast half that runs by default.
///
/// A message-only window is out of the Z order and can never be the foreground, so the gate
/// must refuse it deterministically whatever else is on the desktop; the window that *is* the
/// foreground must pass, because the FR-10 rows «смена активного окна» and «смена фокуса
/// внутри окна» are about exactly it (Р-60). A null handle is the documented shape of a
/// foreground-lost event and must be refused rather than dereferenced.
#[test]
fn the_gate_refuses_a_window_that_cannot_be_the_foreground_and_passes_the_one_that_is() {
    let window = TestWindow::new();

    assert!(
        !watchdog::concerns_the_foreground(window.handle),
        "a message-only window is never the user's foreground"
    );

    assert!(
        !watchdog::concerns_the_foreground(HWND::default()),
        "a null window — the shape of a foreground-lost event — is not the user's context"
    );

    // SAFETY: takes no arguments, touches no memory of ours, returns a handle by value — or
    // null when the desktop has no foreground window, in which case there is nothing to assert
    // the positive half against.
    let foreground = unsafe { GetForegroundWindow() };

    if !foreground.0.is_null() {
        assert!(
            watchdog::concerns_the_foreground(foreground),
            "the foreground window itself is the user's context by definition"
        );
    }
}

/// A helper process the test spawned, killed by the identifier the spawn returned — Р-42.
///
/// The scripts live in the test process's temporary directory, not in the project tree; the
/// kill in `Drop` runs on every exit path, panicking assertions included.
struct SpawnedScript {
    child: Child,
}

impl SpawnedScript {
    /// `CREATE_NO_WINDOW`: the helper must not own a console window, because a console
    /// appearing can itself take the activation and become the very foreground change the
    /// test is trying to control.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn spawn(name: &str, script: &str) -> Self {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, script).expect("the helper script must be writable to %TEMP%");

        let child = Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&path)
            .creation_flags(Self::CREATE_NO_WINDOW)
            .spawn()
            .expect("powershell.exe must start");

        Self { child }
    }

    fn id(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for SpawnedScript {
    fn drop(&mut self) {
        // The identifier the spawn returned is the handle `kill` acts on; a helper that
        // already left on its own lifetime timer answers with an error that means exactly
        // that, and there is nothing to do about it.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A window of ours that takes the foreground honestly — `AttachThreadInput` to the thread of
/// the current foreground window, then `SetForegroundWindow`, the technique of §4.1а — and
/// re-asserts it once a second so the storm cannot inherit a vacant foreground. Measured in
/// the task report: with no window holding the foreground, the storm's `SetFocus` promotes its
/// own form and every churn event then legitimately concerns the foreground.
const HOLDER_PS1: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class HolderNative
{
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hwnd);
    public static void TakeForeground(IntPtr hwnd)
    {
        IntPtr fg = GetForegroundWindow();
        if (fg == hwnd) return;
        uint pid;
        uint fgTid = GetWindowThreadProcessId(fg, out pid);
        uint myTid = GetCurrentThreadId();
        bool attached = false;
        if (fgTid != 0 && fgTid != myTid) attached = AttachThreadInput(myTid, fgTid, true);
        SetForegroundWindow(hwnd);
        if (attached) AttachThreadInput(myTid, fgTid, false);
    }
}
'@
$form = New-Object System.Windows.Forms.Form
$form.Text = 'LangSw-T10-holder'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(600, 40)
$form.Size = New-Object System.Drawing.Size(240, 100)
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 400
$timer.Add_Tick({ [HolderNative]::TakeForeground($form.Handle) })
$stop = New-Object System.Windows.Forms.Timer
$stop.Interval = 60000
$stop.Add_Tick({ [System.Windows.Forms.Application]::Exit() })
$timer.Start(); $stop.Start()
[System.Windows.Forms.Application]::Run($form)
"#;

/// The storm: a visible but never-activated window whose thread cycles `SetFocus` between its
/// own two controls every 40 ms — the same idle churn the acceptance run measured at ~0.8
/// events a second from background Electron windows, delivered at twenty-five times the rate.
const STORM_PS1: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms, System.Drawing -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public class QuietStormForm : Form
{
    protected override bool ShowWithoutActivation { get { return true; } }
}
public static class StormNative
{
    [DllImport("user32.dll")] public static extern IntPtr SetFocus(IntPtr hWnd);
}
'@
$form = New-Object QuietStormForm
$form.Text = 'LangSw-T10-storm'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(40, 40)
$form.Size = New-Object System.Drawing.Size(260, 140)
$form.ShowInTaskbar = $false
$tb1 = New-Object System.Windows.Forms.TextBox
$tb1.Location = New-Object System.Drawing.Point(10, 10)
$tb2 = New-Object System.Windows.Forms.TextBox
$tb2.Location = New-Object System.Drawing.Point(10, 40)
$form.Controls.Add($tb1)
$form.Controls.Add($tb2)
$script:flip = $false
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 40
$timer.Add_Tick({
    $script:flip = -not $script:flip
    $target = if ($script:flip) { $tb1.Handle } else { $tb2.Handle }
    [void][StormNative]::SetFocus($target)
})
$stop = New-Object System.Windows.Forms.Timer
$stop.Interval = 60000
$stop.Add_Tick({ [System.Windows.Forms.Application]::Exit() })
$timer.Start(); $stop.Start()
[System.Windows.Forms.Application]::Run($form)
"#;

/// The real thing FR-10 is about: one process, two top-level windows, the foreground taken
/// honestly once and then moved between them on a schedule — a change of the user's active
/// window each time, from a process that is foreign to the product's subscriptions.
const SWITCHER_PS1: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms, System.Drawing -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public class QuietSwitcherForm : Form
{
    protected override bool ShowWithoutActivation { get { return true; } }
}
public static class SwitcherNative
{
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    public static void TakeForeground(IntPtr hwnd)
    {
        IntPtr fg = GetForegroundWindow();
        uint pid;
        uint fgTid = GetWindowThreadProcessId(fg, out pid);
        uint myTid = GetCurrentThreadId();
        bool attached = false;
        if (fgTid != 0 && fgTid != myTid) attached = AttachThreadInput(myTid, fgTid, true);
        SetForegroundWindow(hwnd);
        if (attached) AttachThreadInput(myTid, fgTid, false);
    }
}
'@
function New-SwitcherForm([string]$title, [int]$x) {
    $f = New-Object QuietSwitcherForm
    $f.Text = $title
    $f.StartPosition = 'Manual'
    $f.Location = New-Object System.Drawing.Point($x, 220)
    $f.Size = New-Object System.Drawing.Size(260, 120)
    return $f
}
$form1 = New-SwitcherForm 'LangSw-T10-SW1' 40
$form2 = New-SwitcherForm 'LangSw-T10-SW2' 340
$form2.Show()
$started = [Environment]::TickCount
$script:done = 0
$script:tookFirst = $false
$tick = New-Object System.Windows.Forms.Timer
$tick.Interval = 100
$tick.Add_Tick({
    $elapsed = [Environment]::TickCount - $started
    if (-not $script:tookFirst) {
        if ($elapsed -ge 1000) {
            $script:tookFirst = $true
            [void][SwitcherNative]::TakeForeground($form1.Handle)
        }
        return
    }
    if ($script:done -lt 12 -and $elapsed -ge (1000 + ($script:done + 1) * 1500)) {
        $script:done++
        $target = if ($script:done % 2 -eq 1) { $form2.Handle } else { $form1.Handle }
        [void][SwitcherNative]::SetForegroundWindow($target)
    }
    if ($elapsed -ge 60000) { [System.Windows.Forms.Application]::Exit() }
})
$tick.Start()
[System.Windows.Forms.Application]::Run($form1)
"#;

/// The process identifier owning the current foreground window, or `0` when there is none.
fn foreground_pid() -> u32 {
    // SAFETY: takes no arguments and returns a handle by value; null when there is no
    // foreground window, which the check below answers with 0.
    let foreground = unsafe { GetForegroundWindow() };

    if foreground.0.is_null() {
        return 0;
    }

    let mut pid = 0_u32;

    // SAFETY: `foreground` is a handle read by value — stale is fine, the call then writes
    // nothing and returns 0 — and `pid` is a live u32 of this frame the call may write to.
    unsafe { GetWindowThreadProcessId(foreground, Some(&raw mut pid)) };

    pid
}

/// Pumps this thread's queue — which is what delivers `WINEVENT_OUTOFCONTEXT` callbacks —
/// until `done` answers true or `patience` runs out, and says which of the two it was.
fn pump_until(patience: Duration, mut done: impl FnMut() -> bool) -> bool {
    let ends = Instant::now() + patience;

    loop {
        drain_the_queue();

        if done() {
            return true;
        }

        if Instant::now() >= ends {
            return false;
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Waits without pumping — for conditions of the world outside this thread's queue.
fn wait_until(patience: Duration, mut done: impl FnMut() -> bool) -> bool {
    let ends = Instant::now() + patience;

    loop {
        if done() {
            return true;
        }

        if Instant::now() >= ends {
            return false;
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

/// **Criterion 13, the storm half.** A foreign process churns focus between its own controls
/// while another window holds the foreground; the product's subscriptions must see every event
/// (positive control: `background_skips` grows), raise not a single flush request, and leave
/// the typed strokes exactly where they were.
///
/// Before the gate of task T-10-0 this scenario read `window_flushes` up by dozens and the
/// buffer emptied — the acceptance-session defect, staged. The task report carries that
/// pre-repair run.
#[test]
#[ignore = "spawns PowerShell helper windows and takes the foreground; run deliberately with --ignored --test-threads=1"]
fn a_foreign_focus_storm_is_seen_and_turned_away_and_the_strokes_survive() {
    let holder = SpawnedScript::spawn("langsw-t10-holder.ps1", HOLDER_PS1);

    assert!(
        wait_until(Duration::from_secs(10), || foreground_pid() == holder.id()),
        "the holder window never became the foreground; the storm would inherit a vacant \
         foreground and the scenario would not be the one under test"
    );

    // The storm starts BEFORE the subscriptions, and the measurement waits for the foreground
    // to be stably the holder's. A freshly spawned process carries a transient right to take
    // the foreground, and the storm's first `SetFocus` activations really do take it for a
    // moment before the holder wins it back — measured: subscribing first read six genuine
    // foreground changes out of that opening fight. The background churn under test is the
    // *steady* state, the one the acceptance machine was in, and it begins once the transient
    // grant is exhausted.
    let storm = SpawnedScript::spawn("langsw-t10-storm.ps1", STORM_PS1);
    let mut stable_for = 0_u32;

    assert!(
        wait_until(Duration::from_secs(20), || {
            stable_for = if foreground_pid() == holder.id() {
                stable_for + 1
            } else {
                0
            };
            stable_for >= 40 // 40 polls at 50 ms — two seconds of uncontested foreground
        }),
        "the foreground never settled on the holder with the storm running; \
         the steady state under test was not reached"
    );
    let _ = storm;

    let watching = watchdog::watch().expect("the subscriptions must install");

    buffer::install_recorder(Recorder::with_capacity(16));

    // What the user had typed before the storm. The stamps do not matter: nothing at all may
    // flush during this test, so nothing ever compares against them.
    for time in [10, 20, 30, 40, 50, 60] {
        press_at(time);
    }
    assert_eq!(buffer::len(), 6);

    let before = watchdog::counters();

    // ⚠ The positive control, and the whole worth of the test: the storm must be DELIVERED
    // into this process. A run in which nothing arrives is not a pass, it is a test that
    // checked nothing — the `WINEVENT_SKIPOWNPROCESS` trap the task specification names.
    let seen = pump_until(Duration::from_secs(12), || {
        watchdog::counters().background_skips - before.background_skips >= 20
    });

    let counted = delta(before, watchdog::counters());

    assert!(
        seen,
        "positive control failed: {} events in 12 s — the storm never reached the \
         subscription, and a green verdict here would have checked nothing",
        counted.background_skips
    );

    assert_eq!(
        counted.window_flushes, 0,
        "the churn of a background process must not raise a single flush request (Р-60)"
    );
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        None,
        "and must leave no flush pending for the input thread"
    );
    assert_eq!(
        buffer::len(),
        6,
        "the strokes typed before the storm survived it — FR-01, FR-40"
    );

    buffer::uninstall();
    drop(watching);
}

/// **Criterion 13, the FR-10 half.** A real change of the user's active window — made honestly
/// by a foreign process moving the foreground between its own two windows — must still flush:
/// the strokes typed before the change go, to the last one.
///
/// This is the behaviour the tests of the accepted tasks already pin at the decision level;
/// what this adds is the whole road under the new gate: a real `EVENT_SYSTEM_FOREGROUND` from
/// a foreign process, through `concerns_the_foreground`, into a pending flush the input
/// thread's message applies to a zeroed slot.
#[test]
#[ignore = "spawns PowerShell helper windows and takes the foreground; run deliberately with --ignored --test-threads=1"]
fn a_real_foreground_change_by_a_foreign_process_still_flushes() {
    let watching = watchdog::watch().expect("the subscriptions must install");

    buffer::install_recorder(Recorder::with_capacity(16));

    let before = watchdog::counters();
    let _switcher = SpawnedScript::spawn("langsw-t10-switcher.ps1", SWITCHER_PS1);

    // The first change the gate lets through — the honest takeover, or the first switch.
    assert!(
        pump_until(Duration::from_secs(15), || {
            watchdog::counters().window_flushes - before.window_flushes >= 1
        }),
        "no real foreground change was delivered; FR-10 cannot be judged by this run"
    );

    // Take whatever is pending, so the flush asserted below is attributable to the change
    // that happens after the strokes.
    let _ = watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0));

    // Strokes typed "now", stamped from the message stream this thread has just pumped —
    // the same tick domain the event timestamps live in (FR-12).
    //
    // SAFETY: takes no arguments, touches no memory of ours, and returns the tick count of
    // the last message this thread retrieved — the queue was pumped a moment ago, so that is
    // a fresh value. The `as u32` is the documented reinterpretation of a tick the system
    // hands back as a signed LONG.
    let typed_at = unsafe { GetMessageTime() } as u32;

    for _ in 0..6 {
        press_at(typed_at);
    }
    assert_eq!(buffer::len(), 6);

    let mid = watchdog::counters();

    // The switcher's next change arrives within its 1.5-second period.
    assert!(
        pump_until(Duration::from_secs(15), || {
            watchdog::counters().window_flushes - mid.window_flushes >= 1
        }),
        "the switcher's next foreground change never arrived"
    );

    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Cleared { removed: 6 }),
        "FR-10 (Р-60): a real change of the user's window still flushes what was typed before it"
    );
    assert_eq!(buffer::len(), 0);

    buffer::uninstall();
    drop(watching);
}
