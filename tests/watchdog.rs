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

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lang_switcher::buffer::{self, Recorder, ResetOutcome, Stroke};
use lang_switcher::guard::{self, Field};
use lang_switcher::hook::{Edge, HotkeyMemory, KeyEvent};
use lang_switcher::layouts;
use lang_switcher::watchdog::{
    self, Cause, Counters, FLUSH_EVENTS, Rebuild, WIPING_SESSION_EVENTS, WM_APP_FLUSH,
    WM_APP_LAYOUT, WM_APP_WIPE,
};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{GetRegisteredRawInputDevices, RAWINPUTDEVICE};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, DispatchMessageW, EVENT_OBJECT_FOCUS,
    EVENT_SYSTEM_DESKTOPSWITCH, EVENT_SYSTEM_FOREGROUND, GetForegroundWindow, GetMessageTime,
    GetWindowThreadProcessId, HWND_MESSAGE, MSG, OBJID_CLIENT, OBJID_WINDOW,
    PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND, PM_REMOVE, PeekMessageW, RI_MOUSE_BUTTON_1_DOWN,
    RI_MOUSE_BUTTON_1_UP, RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_2_UP, RI_MOUSE_BUTTON_3_DOWN,
    RI_MOUSE_BUTTON_3_UP, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP, RI_MOUSE_BUTTON_5_DOWN,
    RI_MOUSE_BUTTON_5_UP, RI_MOUSE_HWHEEL, RI_MOUSE_WHEEL, WINDOW_STYLE, WM_DEVICECHANGE, WM_INPUT,
    WM_INPUT_DEVICE_CHANGE, WM_INPUTLANGCHANGE, WM_POWERBROADCAST, WM_TIMER, WM_WTSSESSION_CHANGE,
    WTS_CONSOLE_CONNECT, WTS_CONSOLE_DISCONNECT, WTS_REMOTE_CONNECT, WTS_REMOTE_DISCONNECT,
    WTS_SESSION_CREATE, WTS_SESSION_LOCK, WTS_SESSION_LOGOFF, WTS_SESSION_LOGON,
    WTS_SESSION_REMOTE_CONTROL, WTS_SESSION_TERMINATE, WTS_SESSION_UNLOCK,
};
use windows::core::{PCWSTR, w};

/// What every `hook::install` of this file asks for — task T-36-3.
///
/// These installations stand in for the start-up one, where the belief about a held hotkey is
/// fresh and clearing it is a no-op said out loud. Which of the two answers a *reinstallation*
/// deserves is `watchdog::clears_hotkey_state`, and its whole table is measured in
/// `tests\hook.rs`: a pure function needs no hook to drive it.
const FORGET: HotkeyMemory = HotkeyMemory::Forget;

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
        watchdog::coalesced(0, 0, Cause::WindowChange).and_then(watchdog::pending_time),
        Some(0)
    );

    let filled =
        watchdog::coalesced(0, 500, Cause::WindowChange).expect("an empty cell takes any request");
    assert_eq!(watchdog::pending_time(filled), Some(500));

    // A newer request replaces it: a flush stamped `T` removes everything at or before `T`, so
    // the newer of two pending flushes does everything the older one would have and more.
    let newer =
        watchdog::coalesced(filled, 900, Cause::WindowChange).expect("900 is newer than 500");
    assert_eq!(watchdog::pending_time(newer), Some(900));

    // An older one, and the same one, add nothing and are dropped.
    assert_eq!(watchdog::coalesced(newer, 500, Cause::WindowChange), None);
    assert_eq!(watchdog::coalesced(newer, 900, Cause::WindowChange), None);

    // Across the wrap of the counter. `u32::max` on the raw values would keep the *older* of
    // these two and lose the flush; the comparison of FR-12 keeps the right one.
    let before_wrap =
        watchdog::coalesced(0, u32::MAX - 10, Cause::WindowChange).expect("an empty cell");
    let after_wrap = watchdog::coalesced(before_wrap, 10, Cause::WindowChange)
        .expect("21 ms later, past the turn");

    assert_eq!(watchdog::pending_time(after_wrap), Some(10));
    assert_eq!(
        watchdog::coalesced(after_wrap, u32::MAX - 10, Cause::WindowChange),
        None
    );
}

/// **The kind of the collapsed request is the stricter of the two** — FR-14, task Т-48-2.
///
/// Two events between two turns of the input thread's message loop become one request, and the
/// one they become may only be exempt from FR-10 if **both** of them could have been. A window
/// change anywhere in the mix means the user moved, and no amount of recent typing makes that
/// the application's answer to a keystroke.
#[test]
fn coalescing_keeps_the_stricter_of_two_causes() {
    // A focus event into an empty cell is a focus request.
    let focus = watchdog::coalesced(0, 500, Cause::FocusChange).expect("an empty cell");
    assert_eq!(watchdog::pending_time(focus), Some(500));
    assert!(watchdog::focus_pending(focus));

    // A second focus event, newer, keeps the kind and moves the stamp.
    let later = watchdog::coalesced(focus, 900, Cause::FocusChange).expect("900 is newer");
    assert_eq!(watchdog::pending_time(later), Some(900));
    assert!(watchdog::focus_pending(later));

    // ⚠ A window change joining them takes the kind away — and it does so **even when it adds
    // nothing to the timestamp**, which is the case a rule written as «the newer wins» would
    // have dropped on the floor.
    let mixed = watchdog::coalesced(later, 900, Cause::WindowChange)
        .expect("the kind still has to be written, though the stamp does not");
    assert_eq!(watchdog::pending_time(mixed), Some(900));
    assert!(
        !watchdog::focus_pending(mixed),
        "a window change is never exempt"
    );

    // …and it does not come back when another focus event arrives behind it.
    let after = watchdog::coalesced(mixed, 1_500, Cause::FocusChange).expect("1500 is newer");
    assert!(!watchdog::focus_pending(after));

    // An empty cell is not a pending focus change; neither is a window request.
    assert!(!watchdog::focus_pending(0));
    let window = watchdog::coalesced(0, 500, Cause::WindowChange).expect("an empty cell");
    assert!(!watchdog::focus_pending(window));

    // The two kinds of an otherwise identical request are different cells, which is what makes
    // the bit readable at all.
    assert_ne!(focus, window);
    assert_eq!(watchdog::pending_time(window), Some(500));
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
        focus_repeats: after.focus_repeats - before.focus_repeats,
        recovery_probes: after.recovery_probes - before.recovery_probes,
        wipe_requests: after.wipe_requests - before.wipe_requests,
        focus_after_typing: after.focus_after_typing - before.focus_after_typing,
        idle_flushes: after.idle_flushes - before.idle_flushes,
    }
}

/// The one test that touches the process-wide flush cell and the process-wide counters, and it
/// is one test on purpose: `cargo test` runs the tests of a binary in parallel, and two tests
/// asserting on the same statics would be asserting on each other's timing.
///
/// ⭐ **Task Т-13-23 — and the whole of that task's window half is here**, for the same reason:
/// `PARTIAL_CLEARS` and `KEPT_EVENTS` are process-wide, so the proof that a window flush no
/// longer moves them has to be made where the flush cell is already owned rather than in a
/// second test racing this one. The audit of 2026-08-24 (hook-buffer, «FR-12 для событий смены
/// окна/фокуса вычисляется и тут же аннулируется») is what these assertions close, and the
/// classification they rest on is checked separately and without any static at all by
/// `only_the_window_flush_is_parked_behind_the_gate`.
///
/// ⚠ **The turn is taken since task Т-13-23.** `coming_back_from_an_absence_asks_for_a_fresh_
/// layout_stamp` reads `window_flushes` as flat under [`NOTICE_TURN`] and this test is what
/// moves it; the flush requests below went from three to five, and a neighbour asserting "FR-11:
/// a probe never flushes" must not be asserting on this test's timing.
#[test]
fn a_flush_request_travels_from_the_watcher_thread_to_a_zeroed_slot() {
    let _turn = notice_turn();

    let before = watchdog::counters();

    // No buffer on this thread yet — every thread but the input one is in that position, and
    // the window procedure runs on all three of them (section 6.3).
    watchdog::request_flush(1_000, Cause::WindowChange);
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
    watchdog::request_flush(1_001, Cause::WindowChange);
    watchdog::request_flush(1_003, Cause::WindowChange);
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

    // ⭐ ---------------------------------------------------------------------------------
    // Task Т-13-23 — the two arms whose survivors the FR-70 gate takes away
    // ---------------------------------------------------------------------------------

    // Three strokes in the ring and a flush older than all of them: `Kept`, the arm FR-12 exists
    // for, and the arm the audit found being counted as though the user were left with it.
    press_at(2_001);
    press_at(2_002);
    assert_eq!(buffer::len(), 3);

    watchdog::request_flush(1_999, Cause::WindowChange);
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Kept { kept: 3 }),
        "FR-12 itself is untouched: the event is older than every stroke"
    );

    // ⭐ **The behaviour is untouched, and this is the proof of it.** FR-12 leaves the three
    // standing; `app::park_buffer` takes them a few lines later — `buffer::reset()` first, then
    // the recorder off the thread — because a window flush empties the buffer «независимо от
    // исхода временно́го разрешения FR-12» (SPEC §10, record 10; decision П-2). Staged the way
    // that function makes it, and read the way the SEC-02 tests of `tests\buffer.rs` read it: on
    // the backing array itself, the slots outside the live window included.
    assert_eq!(
        buffer::len(),
        3,
        "FR-12 kept them; the park has not run yet"
    );
    assert!(buffer::reset(), "the park empties what the flush kept");
    assert_eq!(buffer::len(), 0);
    assert_eq!(
        non_zero_slots(),
        Some(0),
        "SEC-02: the ring a window flush leaves behind is empty and zeroed"
    );

    // The same again for `Partial`: two strokes at or before the event, one after it.
    for time in [3_000, 3_001, 3_002] {
        press_at(time);
    }

    watchdog::request_flush(3_001, Cause::WindowChange);
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Partial {
            removed: 2,
            kept: 1
        }),
        "FR-12 itself is untouched here as well"
    );
    assert_eq!(buffer::len(), 1, "one stroke is newer than the event");

    assert!(buffer::reset(), "and the park takes that one too");
    assert_eq!(
        non_zero_slots(),
        Some(0),
        "SEC-02 after the second window flush"
    );

    buffer::uninstall();

    // ⭐ **The mouse row of FR-10, driven where the arithmetic lives.** A real `WM_INPUT` needs a
    // live raw-input packet, which no test process can stage — the forged one above is refused by
    // `GetRawInputData` before anything is counted at all — so `note_flush_outcome` is called
    // directly. This is the half of task Т-13-23 that is easiest to break: `Partial` and `Kept`
    // do reach the user on this row, nothing parks the buffer behind a click, and these readings
    // have to be exactly what they always were.
    watchdog::note_flush_outcome(
        WM_INPUT,
        ResetOutcome::Partial {
            removed: 2,
            kept: 3,
        },
    );
    watchdog::note_flush_outcome(WM_INPUT, ResetOutcome::Kept { kept: 3 });
    watchdog::note_flush_outcome(WM_INPUT, ResetOutcome::Cleared { removed: 4 });

    let counted = delta(before, watchdog::counters());

    assert_eq!(counted.window_flushes, 5, "five requests were raised");
    assert_eq!(counted.window_flushes_taken, 4, "four of them were taken");

    // ⭐ **The finding, in two numbers.** Three window flushes above resolved as `Partial` or
    // `Kept` — with «выжившие» strokes and all — and not one of them is counted here; what is
    // left is the single mouse `Partial` and the single mouse `Kept`. Before task Т-13-23 the
    // same run read `partial_clears=3` and `kept_events=2`, and every one of those extra counts
    // was a claim about strokes `app::park_buffer` had already taken away.
    assert_eq!(
        counted.partial_clears, 1,
        "only the mouse row may claim that strokes survived a flush (was 3)"
    );
    assert_eq!(
        counted.kept_events, 1,
        "the window `Kept` is not counted; the mouse one is (was 2)"
    );

    // Deliberately unchanged: neither of these is a claim about survivors. A `Cleared` emptied
    // the ring wherever it came from, and every stroke `reset_up_to` removed is still counted.
    assert_eq!(
        counted.full_clears, 2,
        "one window `Cleared`, one mouse `Cleared`"
    );
    assert_eq!(
        counted.strokes_removed, 13,
        "window: 2 by the partial, 3 by the full, 2 by the partial of Т-13-23; mouse: 2 + 4"
    );

    assert_eq!(
        counted.mouse_packets, 0,
        "the forged WM_INPUT was not a packet"
    );
    assert_eq!(counted.mouse_flushes, 0);
}

// ---------------------------------------------------------------------------------------
// FR-14 — a focus event the typing itself caused is not a boundary; question 111, task Т-48-2
// ---------------------------------------------------------------------------------------

/// **The user's first and second findings, in the arithmetic that produced them** — 2026-09-09,
/// question **111**:
///
/// > открываем браузер EDGE набираем нфтвучюкг нажимаем pause получаем нфndex.ru
///
/// The address bar of Edge announces the selected row of its suggestion list as the **active
/// child** of the field, and MSAA delivers that as `EVENT_OBJECT_FOCUS` on the same top-level
/// window with a different `idChild`. The triple differs, so `focus_repeated` does not absorb
/// it, and before task Т-48-2 the input thread answered it with `apply_flush` — the strokes
/// typed before the message was dispatched were cut away, and `Shift+Left` then covered only
/// what was left. Two letters out of nine: `нфтвуч.ru` → `нфndex.ru`.
///
/// FR-14 is the rule that answers it: an `EVENT_OBJECT_FOCUS` of the foreground window with a
/// stroke of the user's own in `(T − Δ, T]` is the application's answer to the typing, not the
/// user leaving the field. The buffer stands.
#[test]
fn a_focus_event_the_typing_itself_caused_keeps_the_buffer() {
    let _turn = notice_turn();

    buffer::install_recorder(Recorder::with_capacity(16));

    // Nine strokes, «нфтвучюкг», in the tick domain the event timestamps live in, and the
    // event stamped a few milliseconds after the last of them — which is what an event the
    // typing *caused* looks like.
    let typed_at = 500_000_u32;
    for offset in 0..9 {
        press_at(typed_at + offset);
    }
    assert_eq!(buffer::len(), 9);

    watchdog::request_flush(typed_at + 12, Cause::FocusChange);

    let before = watchdog::counters();

    // The pair the input thread runs, in its order: the rule first, the arithmetic of FR-12
    // only if the rule did not answer.
    assert!(
        watchdog::take_typing_induced_flush(WM_APP_FLUSH),
        "FR-14: a focus event within Δ of the user's own typing is not a boundary"
    );
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        None,
        "the request was taken by the rule, so nothing is left to apply"
    );
    assert_eq!(
        buffer::len(),
        9,
        "FR-14: every stroke of the word survives the suggestion list opening"
    );

    let counted = delta(before, watchdog::counters());

    assert_eq!(
        counted.focus_after_typing, 1,
        "and the exemption is counted"
    );
    assert_eq!(
        counted.window_flushes_taken, 1,
        "the request really was taken out of the cell"
    );
    assert_eq!(
        (counted.full_clears, counted.strokes_removed),
        (0, 0),
        "no flush happened, so no flush is reported (SEC-04a)"
    );

    buffer::reset();
    buffer::uninstall();
}

/// **The canon of FR-10 where FR-14 does not reach: strokes older than Δ are still cut away.**
///
/// The exemption is bounded in time on purpose — a focus change that arrives with no recent
/// typing behind it is the user moving to another field, which is the row of the FR-10 table
/// FR-14 explicitly does not touch. Five seconds is more than three times Δ.
#[test]
fn a_focus_event_older_than_the_typing_window_still_flushes() {
    let _turn = notice_turn();

    buffer::install_recorder(Recorder::with_capacity(16));

    let typed_at = 600_000_u32;
    for offset in 0..6 {
        press_at(typed_at + offset);
    }
    assert_eq!(buffer::len(), 6);

    // Δ is 1500 ms; the event is five seconds after the last stroke.
    assert_eq!(watchdog::TYPING_WINDOW_MS, 1_500);
    watchdog::request_flush(typed_at + 5_000, Cause::FocusChange);

    assert!(
        !watchdog::take_typing_induced_flush(WM_APP_FLUSH),
        "the typing window has closed, so the rule does not answer"
    );
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Cleared { removed: 6 }),
        "FR-10 is untouched where the typing window has closed"
    );
    assert_eq!(buffer::len(), 0);

    buffer::uninstall();
}

/// **`EVENT_SYSTEM_FOREGROUND` never answers to FR-14** — the user changing windows is a change
/// of the input context whatever they were typing a moment earlier.
#[test]
fn a_foreground_change_is_never_caused_by_typing() {
    let _turn = notice_turn();

    buffer::install_recorder(Recorder::with_capacity(16));

    let typed_at = 700_000_u32;
    for offset in 0..6 {
        press_at(typed_at + offset);
    }

    // The same shape as the exempt case above — a stroke well inside `(T − Δ, T]` — and the
    // only difference is the kind of event that raised the request.
    watchdog::request_flush(typed_at + 12, Cause::WindowChange);

    assert!(
        !watchdog::take_typing_induced_flush(WM_APP_FLUSH),
        "FR-14: «Смена активного окна правилу не подчиняется никогда»"
    );
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Cleared { removed: 6 }),
        "a window change flushes however recently the user typed"
    );
    assert_eq!(buffer::len(), 0);

    buffer::uninstall();
}

/// **SEC-06 is not weakened: `Pending` and `Password` take the old road** — decision П-4, task
/// Т-48-2.
///
/// FR-14 acts only while recording is allowed. The two states that answer `false` are the two
/// the requirement is protecting — `Pending` is «the user has just moved into a field and the
/// probe has not answered yet», `Password` is FR-70 itself — and in neither is a focus event
/// exempt, however recently the user typed.
///
/// ⚠ **Why the gate is read rather than staged.** `Field::Password` is published by
/// `guard::run_pending_probe` off the real foreground window, and `Field::Pending` cannot be
/// held in a test process at all: with no watcher window to answer, `note_focus_moved` resolves
/// it to `Undetermined` before it returns (FR-73). The same wall
/// `a_move_out_of_a_password_element_in_one_window_returns_buffering` runs into. So the two
/// halves are checked where each of them lives: the rule, through `buffering_allowed_for`, and
/// the wire, in the one line of `take_typing_induced_flush` that consults it.
#[test]
fn the_typing_rule_acts_only_while_recording_is_allowed() {
    assert!(
        !guard::buffering_allowed_for(Field::Password),
        "FR-70: nothing is recorded in a password field"
    );
    assert!(
        !guard::buffering_allowed_for(Field::Pending),
        "FR-71: nothing is recorded while the verdict is owed"
    );
    assert!(guard::buffering_allowed_for(Field::Ordinary));
    assert!(
        guard::buffering_allowed_for(Field::Undetermined),
        "FR-73: an undeterminable field records"
    );

    // The wire. `take_typing_induced_flush` is the one place FR-14 is decided, and one of its
    // five conditions is this gate; a rule that forgot it would exempt a focus event **inside**
    // a password field, which is the SEC-06 hole П-4 forbids.
    let source = source_of("watchdog.rs");
    let rule = body_of(&source, "pub fn take_typing_induced_flush");

    assert_eq!(
        code_lines_with(rule, "crate::guard::buffering_allowed()").len(),
        1,
        "FR-14 asks the gate of FR-70/FR-71 exactly once, and it does ask it"
    );
    assert_eq!(
        code_lines_with(rule, "crate::buffer::edited_within(").len(),
        1,
        "…and the measurement of FR-14 itself"
    );
    assert_eq!(
        code_lines_with(rule, "focus_pending(cell)").len(),
        1,
        "…and the kind of the event, so that a window change is never exempt"
    );
}

/// How many slots of the backing array of this thread's buffer are not zero, or `None` on a
/// thread that owns no buffer.
///
/// The SEC-02 reading of `tests\buffer.rs` — «явно перезаписывается нулями, а не просто
/// помечается пустым» — taken from the other side of `buffer::with`, so that a parked ring can be
/// examined here without a `Recorder` of this file's own.
fn non_zero_slots() -> Option<usize> {
    buffer::with(|recorder| {
        recorder
            .slots()
            .iter()
            .filter(|slot| **slot != Stroke::ZEROED)
            .count()
    })
}

/// ⭐ **Task Т-13-23 — which flushes the FR-70 gate parks, and which it leaves alone.**
///
/// The classification the two conditional arms of `watchdog::note_flush_outcome` rest on, checked
/// on its own: it is a function of the message and of nothing else, so it needs no buffer, no
/// window and no process-wide counter, and it can therefore be a test of its own beside the
/// serialised one above.
///
/// `WM_APP_FLUSH` is the two `WinEvent` rows of the FR-10 table, and `app::window_proc` answers it
/// with an unconditional `app::park_buffer` (SPEC §10, record 10; decision П-2). `WM_INPUT` is the
/// mouse row and is parked behind nothing — which is why it is the one row on which `Partial` and
/// `Kept` were ever true, and why task Т-13-23 left its counting exactly as it found it.
#[test]
fn only_the_window_flush_is_parked_behind_the_gate() {
    assert!(
        watchdog::parks_the_buffer(WM_APP_FLUSH),
        "SPEC §10 record 10: a window or focus change empties the buffer whatever FR-12 decided"
    );

    assert!(
        !watchdog::parks_the_buffer(WM_INPUT),
        "FR-13: nothing parks the buffer behind a mouse click"
    );

    // Neither is anything else a flush that parks: the classification is closed, and the three
    // messages this module posts at the input thread for other errands are not flushes at all.
    //
    // ⭐ **`WM_APP_WIPE` is on this list deliberately — task Т-13-7.** Rows 8 and 9 of the FR-10
    // table do empty the ring, but they do not travel through `apply_flush` at all: the message
    // carries no timestamp, `flush_time` answers `None` for it, and `note_flush_outcome` is
    // therefore never reached on that path. So it moves none of the four counters task Т-13-23
    // separated, and it must answer `false` here — a `true` would be a claim that
    // `app::park_buffer` runs behind it, which nothing does.
    for message in [
        WM_APP_LAYOUT,
        watchdog::WM_APP_REHOOK,
        WM_APP_WIPE,
        WM_TIMER,
        0x0401,
    ] {
        assert!(
            !watchdog::parks_the_buffer(message),
            "only WM_APP_FLUSH is answered by a park"
        );
    }
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

/// The turn-taking lock for every test that makes this process's **one** notice window or its
/// **one** input window, or that asserts on the counters the messages of FR-80 move — tasks
/// **T-10-13** and **Т-13-3**.
///
/// ⚠ `NOTICE_WINDOW` is a process-wide cell and `register_session_notice` is what fills it, so
/// two tests holding registrations at once would each be answering `is_ui_window` for the
/// other's window; and `POWER_RESUMES`, `SESSION_CHANGES` and `RECOVERY_PROBES` are process-wide
/// counters, so a test asserting "nothing moved" beside a test that moves them is asserting on
/// its neighbour's timing. `cargo test` runs the tests of a binary in parallel, which is exactly
/// how the Raw Input pair above first failed; the same cure, for the same reason.
///
/// ⚠ Task **Т-13-3** puts a second cell of the same kind under this lock: `INPUT_WINDOW`, which
/// `start_liveness_timer` fills and `Liveness::drop` empties, and which is now what answers
/// `is_input_window`. `PENDING_REASON` belongs to the same turn — it is process-wide, the two
/// system arms arm it, and the rehook arm is what empties it.
///
/// ⚠ Task **Т-13-7** puts one more counter under it: `WIPE_REQUESTS`, read through
/// [`Counters::wipe_requests`]. It is process-wide like the three above, it is moved by the
/// session arm and by the tray, and `the_session_lock_wipes_the_ring_and_the_unlock_does_not`
/// asserts both that it grew and that it stood still.
static NOTICE_TURN: Mutex<()> = Mutex::new(());

/// Takes the turn, ignoring poisoning — see [`raw_input_turn`] for why.
fn notice_turn() -> std::sync::MutexGuard<'static, ()> {
    NOTICE_TURN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Criterion 10: the registration of FR-80 is made and — the half the task specification calls
/// obligatory — withdrawn.
#[test]
fn the_session_registration_of_fr80_is_accepted_and_withdrawn() {
    let _turn = notice_turn();
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

    // Task Т-13-3: starting the timer is also what publishes `INPUT_WINDOW`, so this test takes
    // the turn its neighbours take.
    let _turn = notice_turn();
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
///
/// ⚠ **The list is every one this crate makes public, and task Т-13-7 lengthened it** — the new
/// `WM_APP_WIPE` is `WM_APP + 16`, one past `hook::WM_APP_SEED_CAPS`, so a list that stopped at
/// `WM_APP_REHOOK` would have compared the new number against nothing near it. Three numbers
/// cannot be reached from an integration test at all, because their modules keep them private:
/// `app`'s wake-up (`WM_APP + 1`) and configuration nudge (`WM_APP + 5`), and the tray callback
/// (`WM_APP + 2`). They are named in the doc comment of every constant here, which is the only
/// place the whole map is written down.
///
/// ⚠ **Task Т-49-2 lengthened it again, and found four numbers that were never in it at all.**
/// The list stopped at `WM_APP + 16` while the program had gone on to `+ 19`: the two sound
/// messages of `app` (`+ 17`, `+ 18`) and the feed message of `letters` (`+ 19`) had never been
/// compared against anything. The third sound message, `WM_APP_SOUND_REFUSED` (`+ 20`), was
/// therefore being added against a list that could not have caught a collision with any of its
/// three nearest neighbours — which is exactly the case the test exists for.
#[test]
fn the_private_messages_of_this_process_are_all_distinct() {
    let messages = [
        lang_switcher::hook::WM_APP_HOTKEY,
        lang_switcher::hook::WM_APP_FAIL_SAFE,
        WM_APP_FLUSH,
        WM_APP_LAYOUT,
        lang_switcher::switch::WM_APP_SWITCH,
        watchdog::WM_APP_REHOOK,
        guard::WM_APP_PROBE,
        guard::WM_APP_FIELD,
        lang_switcher::selection::WM_APP_SELECTION,
        lang_switcher::selection::WM_APP_BUFFER_PATH,
        lang_switcher::settings::WM_APP_SYSTEM_THEME,
        lang_switcher::hook::WM_APP_SEED_CAPS,
        WM_APP_WIPE,
        lang_switcher::app::WM_APP_SOUND_DONE,
        lang_switcher::app::WM_APP_SOUND_IDLE,
        lang_switcher::letters::WM_APP_FEED,
        lang_switcher::app::WM_APP_SOUND_REFUSED,
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
    let _turn = notice_turn();

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
    let counted_before = watchdog::counters();

    for (message, wparam) in [
        (WM_POWERBROADCAST, WPARAM(PBT_APMRESUMEAUTOMATIC as usize)),
        (WM_WTSSESSION_CHANGE, WPARAM(WTS_SESSION_UNLOCK as usize)),
        // Task Т-13-7: the subtype that **does** wipe, aimed at a window that is not the UI one.
        // SEC-05 — the arm is not entered, so nothing is asked of the input thread either.
        (WM_WTSSESSION_CHANGE, WPARAM(WTS_SESSION_LOCK as usize)),
        (WM_TIMER, WPARAM(watchdog::LIVENESS_TIMER_ID)),
        (watchdog::WM_APP_REHOOK, WPARAM(0)),
        (WM_APP_WIPE, WPARAM(0)),
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
    assert_eq!(
        delta(counted_before, watchdog::counters()).wipe_requests,
        0,
        "SEC-05, task Т-13-7: a lock aimed at a window that is not the UI one asks for no wipe"
    );

    // The one that matters: no keyboard hook exists in this process, and the keyboard of
    // whoever is running `cargo test` is untouched.
    assert!(!lang_switcher::hook::is_installed());
}

// ---------------------------------------------------------------------------------------
// Task Т-13-3 — the gates of FR-80 ask the window, not the typing buffer
// ---------------------------------------------------------------------------------------

/// ⭐ **The repair of task Т-13-3, the detector.** The arms of FR-80 are bound to the input
/// **window**, so the watchdog goes on working while `guard` has the typing buffer parked.
///
/// # What was wrong
///
/// `watchdog::is_input_window` answered `buffer::is_installed()`, and `app::park_buffer` calls
/// `buffer::uninstall` for the whole of `Field::Pending`, of a password field (FR-70) and of an
/// excluded process (FR-84 — a game, for hours). In those states every arm below fell into
/// `DefWindowProcW`: the liveness tick stopped ticking and the rehook request was dropped with
/// `PENDING_REASON` still armed and nobody left to repost it. Audit of 2026-08-24,
/// guard-watchdog finding 2.
///
/// # What this test shows, and what the two `#[ignore]`d tests below add to it
///
/// This one is about the **gate** and nothing else: whether the arm is entered at all, which is
/// `Some` against `None`. It drives `WM_APP_REHOOK` with `PENDING_REASON` deliberately empty —
/// the arm's own SEC-05 path — so that no hook is installed in a process that is not the
/// product, which is the rule this file's module documentation states and the reason
/// `the_gap_without_a_hook_is_microseconds` is ignored. The reinstallation the open gate leads
/// to, and the counters it moves, are the two ignored tests further down.
#[test]
fn a_parked_buffer_no_longer_shuts_the_gates_of_fr80() {
    let _turn = notice_turn();

    assert!(
        !lang_switcher::hook::is_installed(),
        "no test in this binary may install a keyboard hook"
    );

    // ⚠ `PENDING_REASON` is process-wide and the two system arms of FR-80 arm it — the neighbour
    // test that wakes the machine leaves it armed. `Reason::None` is what the rehook arm reads
    // as "nothing was requested", so publishing it here is how this test guarantees that the
    // open gate below cannot reach `reinstall_hook`. It is also the state SEC-05 describes: a
    // forgery finds emptiness.
    watchdog::request_rehook(watchdog::Reason::None);

    let window = TestWindow::new();
    let foreign = TestWindow::new();

    // **The park, staged the way `app::park_buffer` makes it**: a buffer on this thread, and
    // then `buffer::uninstall` taking it away. This is the state FR-70 and FR-84 hold for
    // minutes and for hours.
    buffer::install_recorder(Recorder::with_capacity(16));
    assert!(buffer::is_installed(), "the precondition of a park");
    assert!(buffer::uninstall(), "the park takes the buffer away");
    assert!(
        !buffer::is_installed(),
        "parked: the thread that owns the hook owns no typing buffer"
    );

    let before = watchdog::health();

    // Before the registration there is no input window at all, and the gate is shut — which is
    // also what makes the assertion after it mean something: what opens the gate is the
    // registration, and the buffer is absent on both sides of it.
    assert!(
        watchdog::handle_watchdog_message(window.handle, watchdog::WM_APP_REHOOK, WPARAM(0))
            .is_none(),
        "with no input window published the arm belongs to no window"
    );

    let liveness =
        watchdog::start_liveness_timer(window.handle).expect("the liveness timer must start");

    // ⭐ The line the finding is about. Before task Т-13-3 this answered `None`, because this
    // thread owns no typing buffer — which is exactly the state FR-70 puts the **input** thread
    // in, and exactly when FR-80 has to keep working.
    assert_eq!(
        watchdog::handle_watchdog_message(window.handle, watchdog::WM_APP_REHOOK, WPARAM(0)),
        Some(LRESULT(0)),
        "FR-80: a parked buffer must not swallow the rehook request"
    );

    // **SEC-05, undiminished.** The same two messages aimed at a window of ours that is not the
    // input one still do nothing whatsoever.
    for (message, wparam) in [
        (watchdog::WM_APP_REHOOK, WPARAM(0)),
        (WM_TIMER, WPARAM(watchdog::LIVENESS_TIMER_ID)),
    ] {
        assert!(
            watchdog::handle_watchdog_message(foreign.handle, message, wparam).is_none(),
            "SEC-05: message {message:#x} on a foreign window must fall through"
        );
    }

    let after = watchdog::health();

    // The other half of SEC-05: the arm was entered and found `PENDING_REASON` empty, so it
    // touched no hook. The gate by window is an addition to that emptiness, never a substitute.
    assert_eq!(
        after.recoveries, before.recoveries,
        "an empty request reinstalls nothing"
    );
    assert_eq!(after.install_failures, before.install_failures);
    assert_eq!(after.liveness_ticks, before.liveness_ticks);
    assert_eq!(after.last_reason, before.last_reason);
    assert!(
        !lang_switcher::hook::is_installed(),
        "and no hook was installed in a process that is not the product"
    );

    // The handle is taken back by the hand that published it: after the timer dies no `WM_TIMER`
    // can arrive for that window, and neither may anything else answer on it.
    drop(liveness);

    assert!(
        watchdog::handle_watchdog_message(window.handle, watchdog::WM_APP_REHOOK, WPARAM(0))
            .is_none(),
        "a window stops being the input window when its liveness timer dies"
    );
}

// ---------------------------------------------------------------------------------------
// Task T-10-13 — the stamp after an absence
// ---------------------------------------------------------------------------------------

/// ⭐ **Defect E, the detector.** Coming back from sleep or from a locked session asks the input
/// thread to re-read the layout, and it does so **before** the user can type.
///
/// # What this is about
///
/// The direction of FR-26 is taken from the layout stamp, and `Stroke::new` copies that stamp
/// into **every stroke as it is recorded** (`src\buffer.rs`). A stamp that is stale while the
/// user types therefore converts the word «в себя» — the «моргает» of the acceptance run — and
/// no refresh made afterwards can undo it: the strokes already carry the wrong layout. So the
/// requirement is not "refresh eventually" but "refresh before the first keystroke", and the
/// three mechanisms of FR-80 that observe an external event are the only moments at which this
/// program learns that it was away at all.
///
/// Before this task the two arms below did `request_rehook` and nothing else. Measured on the
/// **running product** (task report, §2): the same six keys typed under a layout the stamp did
/// not name blinked on the first press and converted on the second, and a delivered
/// `PBT_APMRESUMEAUTOMATIC` — `posted=True` from a relay at high integrity, because UIPI refuses
/// a plain post — changed nothing; one `WM_APP_LAYOUT` posted into the same product repaired the
/// very first press.
///
/// # Why the counter and not `layout_probes`
///
/// `layout_probes` counts probes the **input thread answered**, and this process has no input
/// thread; it is flat here whatever the product does, so a test reading it would be green for
/// the wrong reason. `recovery_probes` counts the probe where it is **sent**, which is the half
/// of the mechanism that lives in this module and the half this test can see.
///
/// # The positive controls
///
/// `power_resumes` and `session_changes` growing is the proof the message really reached the arm
/// — a run with every counter flat would otherwise be indistinguishable from a message nobody
/// delivered, which is precisely the trap the live measurement fell into and was caught by
/// examining the Win32 return. `recoveries` staying flat is the proof that no hook was installed
/// on the way, and `window_flushes` staying flat is FR-11: a layout question is never a flush.
#[test]
fn coming_back_from_an_absence_asks_for_a_fresh_layout_stamp() {
    let _turn = notice_turn();

    let window = TestWindow::new();

    // The registration is what publishes `NOTICE_WINDOW`, and `is_ui_window` reading that cell
    // is what binds the two system arms to this window. Without it the messages below fall
    // through to `DefWindowProcW` and this test would assert on nothing (SEC-05).
    let notice = watchdog::register_session_notice(window.handle)
        .expect("WTSRegisterSessionNotification must be accepted");

    let before = watchdog::counters();
    let before_health = watchdog::health();

    // **The machine woke.** `PBT_APMRESUMEAUTOMATIC` is delivered on every resume, including the
    // ones nobody was present for — FR-80's third mechanism.
    assert_eq!(
        watchdog::handle_watchdog_message(
            window.handle,
            WM_POWERBROADCAST,
            WPARAM(PBT_APMRESUMEAUTOMATIC as usize)
        ),
        Some(LRESULT(1)),
        "a power notification at the UI window is answered TRUE"
    );

    let after_resume = watchdog::counters();
    let health_after_resume = watchdog::health();

    assert_eq!(
        health_after_resume.power_resumes,
        before_health.power_resumes + 1,
        "positive control: the broadcast reached the arm"
    );
    assert_eq!(
        delta(before, after_resume).recovery_probes,
        1,
        "T-10-13: waking up must ask the input thread to re-read the layout (FR-21, FR-26)"
    );

    // **The session came back.** Lock and unlock deliver `WM_WTSSESSION_CHANGE`, and the
    // acceptance run passed position 19 only because the password typed at the lock screen
    // released a modifier and that released modifier posted a probe — the product's own
    // `LAYOUT_PROBE_MODIFIERS` path, measured arm F of the report. Nothing about a session
    // change guarantees a modifier, so the same probe has to be posted here.
    assert_eq!(
        watchdog::handle_watchdog_message(
            window.handle,
            WM_WTSSESSION_CHANGE,
            WPARAM(WTS_SESSION_UNLOCK as usize)
        ),
        Some(LRESULT(0))
    );

    let after_unlock = watchdog::counters();
    let health_after_unlock = watchdog::health();

    assert_eq!(
        health_after_unlock.session_changes,
        before_health.session_changes + 1,
        "positive control: the session change reached the arm"
    );
    assert_eq!(
        delta(before, after_unlock).recovery_probes,
        2,
        "T-10-13: an unlocked session must ask for the layout too"
    );

    // **The negative control.** Suspending is not coming back, and the other `PBT_*` subtypes
    // must leave both counters alone: a probe on every power notification would be a mechanism
    // that fires when nothing has happened.
    assert_eq!(
        watchdog::handle_watchdog_message(
            window.handle,
            WM_POWERBROADCAST,
            WPARAM(PBT_APMSUSPEND as usize)
        ),
        Some(LRESULT(1))
    );

    let after_suspend = watchdog::counters();

    assert_eq!(
        watchdog::health().power_resumes,
        health_after_unlock.power_resumes,
        "a suspend is not a resume and is counted by nobody"
    );
    assert_eq!(
        delta(before, after_suspend).recovery_probes,
        2,
        "going to sleep is not coming back from it"
    );

    // Nothing else moved. No hook was installed — `reinstall_hook` belongs to the input window
    // and none of the arms above is it — and FR-11 holds: a layout question is not a flush.
    let whole = delta(before, after_suspend);

    assert_eq!(watchdog::health().recoveries, before_health.recoveries);
    assert_eq!(whole.window_flushes, 0, "FR-11: a probe never flushes");
    assert_eq!(whole.window_flushes_taken, 0);
    assert_eq!(whole.strokes_removed, 0);
    assert!(!lang_switcher::hook::is_installed());

    drop(notice);
}

// ---------------------------------------------------------------------------------------
// FR-10 rows 8 and 9 — the session lock and the tray's pause, task Т-13-7
// ---------------------------------------------------------------------------------------
//
// ⭐ **What the audit of 2026-08-24 found here, and what these two tests are.** Direction
// *tests*, finding «Два правила сброса FR-10 … не покрыты ни одним тестом, а тест-комментарий
// заявляет их покрытие; проводка в продукте отсутствует»: `tests\buffer.rs` claimed rows 8 and 9
// in a comment and called `recorder.reset()` by hand, while in the product nothing called it for
// either row — `grep WTS_SESSION_LOCK` over the whole repository returned nothing at all. The
// comment is corrected where it stood; the wire is these two tests' subject.
//
// They are split the way the code is: the classification of the subtypes is a function of one
// number and needs nothing at all, while the route needs both of this process's windows and moves
// process-wide counters, so it takes the turn its neighbours take.

/// **Which `WM_WTSSESSION_CHANGE` subtypes are row 8 of the FR-10 table — and which are not.**
///
/// Row 8 reads «Блокировка сессии, смена пользователя», not «любое сообщение о сессии», and the
/// rehook of FR-80 in the same arm deliberately does *not* tell the subtypes apart. So the list
/// is written as data (`WIPING_SESSION_EVENTS`) and checked here against every subtype Windows
/// defines, both directions: a member missed and a member added are both defects, and the second
/// one throws away a word the user is still typing.
#[test]
fn the_wiping_session_subtypes_are_the_two_halves_of_row_eight_and_nothing_else() {
    assert_eq!(
        WIPING_SESSION_EVENTS,
        [
            WTS_SESSION_LOCK,
            WTS_CONSOLE_DISCONNECT,
            WTS_REMOTE_DISCONNECT
        ],
        "FR-10 row 8: the lock, and the two shapes of «смена пользователя»"
    );

    for code in WIPING_SESSION_EVENTS {
        assert!(
            watchdog::session_event_wipes(code),
            "the list and the predicate must be the same rule: {code}"
        );
    }

    // Every other subtype Windows defines, one by one and by name. `WTS_SESSION_UNLOCK` is the
    // one the task specification calls out — the user is coming back to a ring the lock already
    // emptied — and `WTS_SESSION_LOGOFF`/`WTS_SESSION_TERMINATE` are FR-83's «обнуление буфера»
    // rather than FR-10's.
    for code in [
        WTS_CONSOLE_CONNECT,
        WTS_REMOTE_CONNECT,
        WTS_SESSION_LOGON,
        WTS_SESSION_LOGOFF,
        WTS_SESSION_UNLOCK,
        WTS_SESSION_REMOTE_CONTROL,
        WTS_SESSION_CREATE,
        WTS_SESSION_TERMINATE,
    ] {
        assert!(
            !watchdog::session_event_wipes(code),
            "subtype {code} must not throw away what the user has typed"
        );
    }

    // SEC-05: the subtype arrives as the `wparam` of a message any process at the same integrity
    // level can post, so a number nobody has heard of has to mean "not one of ours".
    for code in [0, 12, 0xFFFF_FFFF] {
        assert!(!watchdog::session_event_wipes(code));
    }
}

/// ⭐ **The wire of task Т-13-7, end to end — FR-10 rows 8 and 9, SEC-02, SEC-05, FR-11.**
///
/// # Why the route is driven in two legs
///
/// It is a route between two threads, and this process has neither of them. `request_wipe` posts
/// through `app::post_to_input_thread`, whose register of windows is filled by `app::serve_window`
/// — a function no test can call — so in a test binary the post is dropped on the floor. That is
/// exactly the trap `coming_back_from_an_absence_asks_for_a_fresh_layout_stamp` was written
/// around, and the cure is the same one: the **send** side is counted where it is sent
/// (`Counters::wipe_requests`), so the first leg is read as a number, and the **receive** side is
/// then driven directly at the input window. A test that only did the second leg would be green
/// over an unwired product — which is precisely the defect the audit found.
///
/// # Why one window wears both roles
///
/// In the product the notice window belongs to the UI thread and the input window to the input
/// thread, and `is_ui_window`/`is_input_window` read two different cells. Here one handle is
/// published into both cells, because what is under test is the pair of arms and not the pair of
/// threads; `foreign` below is the window that is in neither cell, and it is what SEC-05 is
/// checked against.
#[test]
fn the_session_lock_wipes_the_ring_and_the_unlock_does_not() {
    let _turn = notice_turn();

    assert!(
        !lang_switcher::hook::is_installed(),
        "no test in this binary may install a keyboard hook"
    );

    let window = TestWindow::new();
    let foreign = TestWindow::new();

    // `NOTICE_WINDOW` — without it `is_ui_window` is false and the session arm is never entered.
    let notice = watchdog::register_session_notice(window.handle)
        .expect("WTSRegisterSessionNotification must be accepted");

    // `INPUT_WINDOW` — task Т-13-3 made this the cell `is_input_window` reads, and it is what
    // binds the `WM_APP_WIPE` arm. ⚠ No `WM_TIMER` and no `WM_APP_REHOOK` is driven while it is
    // published: either would reach `reinstall_hook` and put a `WH_KEYBOARD_LL` hook into a
    // process that pumps no messages.
    let liveness =
        watchdog::start_liveness_timer(window.handle).expect("the liveness timer must start");

    buffer::install_recorder(Recorder::with_capacity(16));

    for time in [10_000, 10_001, 10_002] {
        press_at(time);
    }

    assert_eq!(buffer::len(), 3, "a half-typed word is in the ring");
    assert_eq!(
        non_zero_slots(),
        Some(3),
        "the positive control of SEC-02: the slots really do hold something now"
    );

    // ---- Leg 1: the lock reaches the UI window and asks the input thread ----------------
    let before = watchdog::counters();

    assert_eq!(
        watchdog::handle_watchdog_message(
            window.handle,
            WM_WTSSESSION_CHANGE,
            WPARAM(WTS_SESSION_LOCK as usize)
        ),
        Some(LRESULT(0)),
        "a session change at the UI window is answered"
    );

    assert_eq!(
        delta(before, watchdog::counters()).wipe_requests,
        1,
        "FR-10 row 8: «блокировка сессии … полный сброс + обнуление памяти» was asked for"
    );

    // The ring is untouched *so far*, and that is the point of the two legs: the UI thread cannot
    // reach a thread-local of the input thread, so the work is still in the post.
    assert_eq!(buffer::len(), 3, "the UI thread emptied nothing itself");

    // ---- Leg 2: the input window answers -----------------------------------------------
    assert_eq!(
        watchdog::handle_watchdog_message(window.handle, WM_APP_WIPE, WPARAM(0)),
        Some(LRESULT(0)),
        "the wipe is answered on the input window"
    );

    assert_eq!(buffer::len(), 0, "FR-10: полный сброс");
    assert_eq!(
        non_zero_slots(),
        Some(0),
        "SEC-02: обнуление памяти — the backing array itself, live window and free slots alike"
    );

    // ---- The unlock is not a wipe -------------------------------------------------------
    for time in [11_000, 11_001] {
        press_at(time);
    }

    let before_unlock = watchdog::counters();

    assert_eq!(
        watchdog::handle_watchdog_message(
            window.handle,
            WM_WTSSESSION_CHANGE,
            WPARAM(WTS_SESSION_UNLOCK as usize)
        ),
        Some(LRESULT(0)),
        "the unlock still reaches the arm — the rehook of FR-80 wants every subtype"
    );

    assert_eq!(
        delta(before_unlock, watchdog::counters()).wipe_requests,
        0,
        "coming back is not going away: `WTS_SESSION_UNLOCK` asks for no wipe"
    );
    assert_eq!(
        buffer::len(),
        2,
        "and what the user typed after the unlock is still theirs"
    );

    // ---- FR-11 is not what this arm answers ---------------------------------------------
    //
    // The one thing easiest to break here. FR-11 says a layout change of the user's own —
    // `Alt+Shift`, `Win+Space` — must **not** flush, and the messages that carry a layout
    // question travel to this very window. None of them is claimed by this procedure at all, and
    // none of them asks for a wipe.
    let before_layout = watchdog::counters();

    for message in [WM_APP_LAYOUT, WM_INPUTLANGCHANGE, WM_DEVICECHANGE] {
        assert!(
            watchdog::handle_watchdog_message(window.handle, message, WPARAM(0)).is_none(),
            "message {message:#x} is not the watchdog's to answer"
        );
    }

    assert_eq!(
        delta(before_layout, watchdog::counters()).wipe_requests,
        0,
        "FR-11: a layout question is never a reset"
    );
    assert_eq!(buffer::len(), 2, "FR-11: and the strokes are still there");

    // ---- SEC-05: the same messages on a window that is neither of ours -------------------
    let before_forgery = watchdog::counters();

    for (message, wparam) in [
        (WM_APP_WIPE, WPARAM(0)),
        (WM_WTSSESSION_CHANGE, WPARAM(WTS_SESSION_LOCK as usize)),
    ] {
        assert!(
            watchdog::handle_watchdog_message(foreign.handle, message, wparam).is_none(),
            "SEC-05: message {message:#x} must fall through on a window of neither role"
        );
    }

    let forged = delta(before_forgery, watchdog::counters());

    assert_eq!(forged.wipe_requests, 0, "a forged lock asks for nothing");
    assert_eq!(forged.window_flushes, 0);
    assert_eq!(forged.window_flushes_taken, 0);
    assert_eq!(forged.strokes_removed, 0);
    assert_eq!(
        buffer::len(),
        2,
        "SEC-05: a forged wipe finds a window that owns no ring"
    );
    assert_eq!(
        non_zero_slots(),
        Some(2),
        "and the ring it could not reach still holds what it held"
    );

    // Nothing above installed a hook, which is the standing rule of this file.
    assert!(!lang_switcher::hook::is_installed());

    buffer::uninstall();

    // ⚠ Left as the neighbours expect to find it. The session arm arms `PENDING_REASON`, which is
    // process-wide; `a_parked_buffer_no_longer_shuts_the_gates_of_fr80` publishes `Reason::None`
    // for exactly this reason, and disarming it here as well is what keeps `INPUT_WINDOW` and an
    // armed reason from ever being published at the same time.
    watchdog::request_rehook(watchdog::Reason::None);

    drop(liveness);
    drop(notice);
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

    let installed = lang_switcher::hook::install(window.handle, instance, FORGET)
        .expect("the hook must install");

    // **Criterion 14, at the level where FR-01 is enforced.** A second installation is refused
    // while one is registered, so no code path — this module's included — can hold two.
    assert!(
        lang_switcher::hook::install(window.handle, instance, FORGET).is_err(),
        "FR-01: a second hook must be refused while one is registered"
    );

    for round in 0..ROUNDS {
        assert!(
            watchdog::reinstall_hook(window.handle, watchdog::Reason::Timer),
            "round {round} must end with a hook"
        );
        assert!(lang_switcher::hook::is_installed());

        // And still exactly one after the reinstallation, not two.
        assert!(lang_switcher::hook::install(window.handle, instance, FORGET).is_err());

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

/// Pumps this thread's queue for `patience`, answering nothing in particular — the shape
/// `drain_the_queue` has, given time for input that has not arrived yet.
#[cfg(test)]
fn pump_for(patience: Duration) {
    let deadline = Instant::now() + patience;

    loop {
        drain_the_queue();

        if Instant::now() >= deadline {
            return;
        }

        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Pumps this thread's queue until module `hook` has handed off `target` presses or patience runs
/// out, and answers with the count reached.
///
/// `WM_APP_HOTKEY` is handed to `hook::handle_input_message` rather than dispatched, because that
/// is what `app::window_proc` does with it on the input thread and it is what moves
/// `hotkey_handoffs`. The window this file creates is a `STATIC`, whose procedure knows nothing
/// of this program's private messages.
#[cfg(test)]
fn pump_until_handoffs(target: u32, patience: Duration) -> u32 {
    let deadline = Instant::now() + patience;
    let mut message = MSG::default();

    loop {
        // SAFETY: as in `drain_the_queue` above — a live `MSG` of this frame, no window filter
        // and no message filter.
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            if lang_switcher::hook::handle_input_message(
                message.message,
                message.wParam,
                message.lParam,
            )
            .is_none()
            {
                // SAFETY: `message` was just filled by `PeekMessageW` and is passed unchanged.
                unsafe { DispatchMessageW(&message) };
            }
        }

        let handoffs = lang_switcher::hook::hotkey_handoffs();

        if handoffs >= target || Instant::now() >= deadline {
            return handoffs;
        }

        std::thread::sleep(Duration::from_millis(2));
    }
}

/// One synthetic edge of the hotkey, delivered to the machine's input stream the way a keyboard
/// delivers one.
///
/// `dwExtraInfo` is zero and deliberately not `INJECTED_SIGNATURE`: FR-03 has to take this for
/// the user's own hand, which is the whole point of the stroke. ⚠ Decision Р-42: it is sent only
/// while this file's hook is installed and armed, so FR-95 suppresses it and no window on the
/// machine ever sees a `Pause`.
#[cfg(test)]
fn send_pause(edge: Edge) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
        VIRTUAL_KEY,
    };

    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(lang_switcher::hook::DEFAULT_HOTKEY_VK),
                wScan: 0,
                dwFlags: match edge {
                    Edge::Down => KEYBD_EVENT_FLAGS(0),
                    Edge::Up => KEYEVENTF_KEYUP,
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    // SAFETY: one live `INPUT` of this frame, described by its own size — which is all
    // `SendInput` reads. The return is the number of events queued and is examined (NFR-13).
    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

    assert_eq!(
        sent, 1,
        "the synthetic {edge:?} must reach the input stream"
    );
}

/// ⭐ **Task Т-22-1 — finding м2 of the mini-audit of 2026-09-01.**
///
/// The down/up state of FR-08 is the input thread's memory of one key, and a reinstallation is
/// exactly the moment that memory can be wrong: `uninstall` takes the hook off, a release made
/// while it is off is seen by nobody, and `install` used to bring the program back still
/// believing the hotkey was held. The next real press was then read as auto-repeat — suppressed
/// by FR-95 but handed off to nobody — so the user pressed `Pause` and nothing happened, once
/// per lost release, silently.
///
/// The staging is the defect's own: press, take the hook away and put it back through the FR-80
/// door, press again. The second press has to hand off too. The release is never sent — it does
/// not have to be, because the state a lost release leaves behind is precisely "we still think
/// it is down", and that is the state the first press already put here.
///
/// ⚠ `#[ignore]`d, and named to sort after `the_gap_without_a_hook_is_microseconds`, for the
/// reasons that test states: it installs a **live global `WH_KEYBOARD_LL` hook**, and it moves
/// `recoveries` by one.
#[test]
#[ignore = "installs a live global WH_KEYBOARD_LL hook and sends synthetic Pause; run deliberately with --ignored --test-threads=1"]
fn the_hotkey_state_of_fr08_does_not_survive_a_reinstallation() {
    let _turn = notice_turn();

    // SAFETY: `None` asks for the handle of the running executable, which cannot be unloaded
    // under us.
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.expect("the module handle");
    let instance = HINSTANCE(module.0);

    let window = TestWindow::new();

    // The mode the callback will read, written out rather than assumed: both of these are
    // published values that another test in this binary is free to have moved.
    lang_switcher::hook::set_hotkey_vk(lang_switcher::hook::DEFAULT_HOTKEY_VK);
    lang_switcher::hook::set_active(true);
    assert!(
        !lang_switcher::hook::fail_safe(),
        "FR-99 must not have disarmed the program before this test"
    );

    let installed = lang_switcher::hook::install(window.handle, instance, FORGET)
        .expect("the hook must install");

    let before = lang_switcher::hook::hotkey_handoffs();

    // Act 1 — a press the hook sees. FR-08 remembers it as down.
    send_pause(Edge::Down);

    let after_first = pump_until_handoffs(before + 1, Duration::from_secs(2));

    assert_eq!(
        after_first,
        before + 1,
        "the first press must reach the far end of the handoff"
    );

    // Act 2 — the gap of FR-80, through the door the watchdog uses.
    assert!(
        watchdog::reinstall_hook(window.handle, watchdog::Reason::Timer),
        "the reinstallation must end with a hook"
    );

    drain_the_queue();

    // Act 3 — the next press. It is the first press of a program that has just started seeing
    // the keyboard again, and FR-08 has to read it as one.
    send_pause(Edge::Down);

    let after_second = pump_until_handoffs(after_first + 1, Duration::from_secs(2));

    // The release, now that there is a hook to eat it: the machine must not be left with `Pause`
    // held down. Sent before the assertion, so that a red test leaves the keyboard as it found
    // it.
    send_pause(Edge::Up);
    pump_for(Duration::from_millis(200));

    drop(installed);
    drain_the_queue();

    assert!(!lang_switcher::hook::is_installed());

    assert_eq!(
        after_second,
        after_first + 1,
        "Т-22-1: a reinstallation must forget the FR-08 down state, or the first press after it \
         is read as auto-repeat and handed off to nobody"
    );
}

/// ⭐ **Task Т-13-3, the whole of the repair.** With the typing buffer parked, the liveness tick
/// of FR-80 **and** the rehook request both reach `reinstall_hook`, and a foreign window still
/// reaches nothing.
///
/// ⚠ `#[ignore]`d for the reason `the_gap_without_a_hook_is_microseconds` above is: entering
/// these two arms is entering `reinstall_hook`, which installs a **live global
/// `WH_KEYBOARD_LL` hook** in a process that is not the product. It is deliberate and short, it
/// pumps the queue so that a stroke arriving meanwhile is answered rather than left to
/// `LowLevelHooksTimeout`, and it takes the hook off itself at the end — `reinstall_hook` keeps
/// it on purpose (`ManuallyDrop`), because in the product the guard `app::serve_window` holds is
/// what removes it.
///
/// ⚠ The name sorts after `the_gap_without_a_hook_is_microseconds`, and that is not an accident:
/// `libtest` runs tests in name order, that test asserts on `recoveries` as an absolute number,
/// and these reinstallations would be added to its total if they ran first.
///
/// The gate itself — `Some` against `None`, with no hook anywhere near it — is
/// `a_parked_buffer_no_longer_shuts_the_gates_of_fr80` above.
#[test]
#[ignore = "installs a live global WH_KEYBOARD_LL hook; run deliberately with --ignored --test-threads=1"]
fn the_parked_buffer_still_gets_its_liveness_tick_and_its_rehook() {
    let _turn = notice_turn();

    let window = TestWindow::new();
    let foreign = TestWindow::new();

    // The park, staged the way `app::park_buffer` makes it — FR-70, FR-84.
    buffer::install_recorder(Recorder::with_capacity(16));
    assert!(buffer::uninstall(), "the park takes the buffer away");
    assert!(!buffer::is_installed(), "parked");

    let liveness =
        watchdog::start_liveness_timer(window.handle).expect("the liveness timer must start");

    let before = watchdog::health();

    // **FR-80, fourth mechanism.** The tick that answers the `LowLevelHooksTimeout` case, which
    // announces itself with nothing whatsoever — and which, before this task, was silently
    // dropped for as long as a password field or an excluded game held the foreground.
    assert_eq!(
        watchdog::handle_watchdog_message(
            window.handle,
            WM_TIMER,
            WPARAM(watchdog::LIVENESS_TIMER_ID)
        ),
        Some(LRESULT(0)),
        "FR-80: the liveness tick is answered on the input window, parked buffer or not"
    );

    drain_the_queue();

    let after_tick = watchdog::health();

    println!(
        "tick:   liveness_ticks {} -> {}, recoveries {} -> {}, reason {} -> {}",
        before.liveness_ticks,
        after_tick.liveness_ticks,
        before.recoveries,
        after_tick.recoveries,
        before.last_reason.name(),
        after_tick.last_reason.name()
    );

    assert_eq!(after_tick.liveness_ticks, before.liveness_ticks + 1);
    assert_eq!(after_tick.recoveries, before.recoveries + 1);
    assert_eq!(after_tick.install_failures, before.install_failures);
    assert_eq!(after_tick.last_reason, watchdog::Reason::Timer);
    assert!(
        lang_switcher::hook::is_installed(),
        "the tick put the hook back"
    );

    // **FR-80, the far end of the other three mechanisms.** A request raised while the buffer is
    // parked is answered while the buffer is parked.
    watchdog::request_rehook(watchdog::Reason::PowerResume);
    assert!(!buffer::is_installed(), "still parked");

    assert_eq!(
        watchdog::handle_watchdog_message(window.handle, watchdog::WM_APP_REHOOK, WPARAM(0)),
        Some(LRESULT(0))
    );

    drain_the_queue();

    let after_rehook = watchdog::health();

    println!(
        "rehook: recoveries {} -> {}, reason {} -> {}",
        after_tick.recoveries,
        after_rehook.recoveries,
        after_tick.last_reason.name(),
        after_rehook.last_reason.name()
    );

    assert_eq!(after_rehook.recoveries, after_tick.recoveries + 1);
    assert_eq!(after_rehook.last_reason, watchdog::Reason::PowerResume);
    assert_eq!(after_rehook.install_failures, before.install_failures);
    assert!(lang_switcher::hook::is_installed());

    // **SEC-05.** The same two messages on a window of ours that is not the input one still do
    // nothing at all — the gate by window did not replace that, it added to it.
    for (message, wparam) in [
        (WM_TIMER, WPARAM(watchdog::LIVENESS_TIMER_ID)),
        (watchdog::WM_APP_REHOOK, WPARAM(0)),
    ] {
        assert!(
            watchdog::handle_watchdog_message(foreign.handle, message, wparam).is_none(),
            "SEC-05: message {message:#x} on a foreign window must fall through"
        );
    }

    let after_foreign = watchdog::health();

    assert_eq!(after_foreign.recoveries, after_rehook.recoveries);
    assert_eq!(after_foreign.liveness_ticks, after_rehook.liveness_ticks);

    // No hook survives a test in this binary.
    drop(liveness);

    assert!(
        lang_switcher::hook::uninstall(),
        "the hook these two reinstallations left must come off"
    );

    drain_the_queue();

    assert!(!lang_switcher::hook::is_installed());
}

/// ⭐ **The race of the audit, staged**: park → rehook → restore. An unlock arms the request, the
/// flush that parks the buffer is answered **first**, and the rehook that arrives second is
/// still spent on a reinstallation instead of being dropped.
///
/// # The order that used to lose
///
/// `WM_WTSSESSION_CHANGE` arrives at the UI window, arms `PENDING_REASON` and posts
/// `WM_APP_REHOOK` to the input thread. An unlocking session also delivers focus events, so
/// `WM_APP_FLUSH` may reach the input thread first; `guard` answers it with `Field::Pending` and
/// `app::park_buffer` takes the buffer away. The `WM_APP_REHOOK` behind it then found
/// `buffer::is_installed()` false, fell into `DefWindowProcW` and was gone — `PENDING_REASON`
/// left armed with nobody to repost it, and up to thirty seconds without a hook.
///
/// ⚠ `#[ignore]`d, and named to sort after `the_gap_without_a_hook_is_microseconds`, for the
/// reasons the test above states.
#[test]
#[ignore = "installs a live global WH_KEYBOARD_LL hook; run deliberately with --ignored --test-threads=1"]
fn the_unlock_race_parks_the_buffer_and_the_rehook_survives_it() {
    let _turn = notice_turn();

    let ui = TestWindow::new();
    let input = TestWindow::new();

    // The two registrations are what bind the two halves of the race to their windows: the
    // session notice publishes the UI window `is_ui_window` reads, the liveness timer publishes
    // the input window `is_input_window` reads.
    let notice = watchdog::register_session_notice(ui.handle)
        .expect("WTSRegisterSessionNotification must be accepted");
    let liveness =
        watchdog::start_liveness_timer(input.handle).expect("the liveness timer must start");

    // Before the lock the input thread has its buffer, as section 6.3 gives it.
    buffer::install_recorder(Recorder::with_capacity(16));
    assert!(buffer::is_installed());

    let before = watchdog::health();

    // **1. The session comes back.** The UI thread arms the request and posts; it touches no
    // hook, because the hook belongs to the input thread.
    assert_eq!(
        watchdog::handle_watchdog_message(
            ui.handle,
            WM_WTSSESSION_CHANGE,
            WPARAM(WTS_SESSION_UNLOCK as usize)
        ),
        Some(LRESULT(0))
    );

    let after_unlock = watchdog::health();

    assert_eq!(after_unlock.session_changes, before.session_changes + 1);
    assert_eq!(
        after_unlock.recoveries, before.recoveries,
        "the UI thread never reinstalls the hook itself"
    );

    // **2. The flush is answered first** — the losing order. `guard` parks the buffer for the
    // duration of `Field::Pending`.
    assert!(buffer::uninstall(), "the park");
    assert!(!buffer::is_installed());

    // **3. The rehook arrives second**, into the parked state.
    assert_eq!(
        watchdog::handle_watchdog_message(input.handle, watchdog::WM_APP_REHOOK, WPARAM(0)),
        Some(LRESULT(0))
    );

    drain_the_queue();

    let after_rehook = watchdog::health();

    println!(
        "race:   recoveries {} -> {}, reason {} -> {}",
        after_unlock.recoveries,
        after_rehook.recoveries,
        after_unlock.last_reason.name(),
        after_rehook.last_reason.name()
    );

    // The request was not lost, and it arrived carrying the reason the session change gave it.
    assert_eq!(
        after_rehook.recoveries,
        after_unlock.recoveries + 1,
        "park → rehook → restore: the hook is put back"
    );
    assert_eq!(after_rehook.last_reason, watchdog::Reason::SessionChange);
    assert_eq!(after_rehook.install_failures, before.install_failures);
    assert!(lang_switcher::hook::is_installed());

    // And it was **spent**, not repeated: a second copy of the message finds `PENDING_REASON`
    // empty and reinstalls nothing (SEC-05).
    assert_eq!(
        watchdog::handle_watchdog_message(input.handle, watchdog::WM_APP_REHOOK, WPARAM(0)),
        Some(LRESULT(0))
    );

    assert_eq!(
        watchdog::health().recoveries,
        after_rehook.recoveries,
        "a spent request reinstalls nothing"
    );

    drop(liveness);
    drop(notice);

    assert!(
        lang_switcher::hook::uninstall(),
        "the hook this reinstallation left must come off"
    );

    drain_the_queue();

    assert!(!lang_switcher::hook::is_installed());
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

/// Serialises the tests that write the **process-wide focus memory** of module `watchdog` —
/// the window and the element of the last focus event, one location for the whole process,
/// and the `focus_repeats` counter these tests assert exact deltas of.
///
/// The same device, and the same reason, as `RAW_INPUT_TURN` above: `cargo test` runs the tests
/// of a binary in parallel, and two tests staging focus events at once would be measuring each
/// other. The staged-churn tests further down drive the same memory through a real
/// subscription and are `#[ignore]`d, so they never run beside these.
static FOCUS_MEMORY_TURN: Mutex<()> = Mutex::new(());

/// Takes the turn, ignoring poisoning — see `raw_input_turn`.
fn focus_memory_turn() -> std::sync::MutexGuard<'static, ()> {
    FOCUS_MEMORY_TURN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// **Task T-10-0e, the decision level.** A focus event that re-announces the element the focus
/// is already on is a repeat — the frontmost window's own churn, measured to erase the user's
/// typing — and a repeat neither flushes nor probes. A focus event on another window is a
/// transfer and keeps flushing; `EVENT_SYSTEM_FOREGROUND` is never a repeat, whatever handle
/// it carries — a window change always flushes.
///
/// Every event here names the same element of its window (`OBJID_CLIENT`, child `0` — the
/// triple a Win32 provider raises for a control that *is* the window), so what this test varies
/// is the handle and nothing else: the window half of the memory, exactly as task T-10-0e left
/// it. The element half is the test below.
///
/// The handles below are made-up values: `focus_repeated` compares them and never
/// dereferences them, which is the property the fix relies on for stale handles too.
#[test]
fn a_focus_event_on_the_same_window_is_a_repeat_and_a_window_change_never_is() {
    let _turn = focus_memory_turn();

    let first = HWND(0x1000_0F01_usize as *mut core::ffi::c_void);
    let second = HWND(0x1000_0F02_usize as *mut core::ffi::c_void);

    let before = watchdog::counters().focus_repeats;

    // The first focus event ever seen: nothing is remembered yet, so it must flush.
    assert!(
        !watchdog::focus_repeated(EVENT_OBJECT_FOCUS, first, OBJID_CLIENT.0, 0),
        "the first focus event is not a repeat"
    );

    // The same window and the same element again — the churn of task T-10-0e — is a repeat.
    assert!(watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        first,
        OBJID_CLIENT.0,
        0
    ));

    // A transfer to another window is not, and it moves the memory.
    assert!(
        !watchdog::focus_repeated(EVENT_OBJECT_FOCUS, second, OBJID_CLIENT.0, 0),
        "a transfer to a different hwnd must keep flushing (FR-10)"
    );
    assert!(watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        second,
        OBJID_CLIENT.0,
        0
    ));

    // A window change is never a repeat, same handle or not: leaving and coming back is a
    // change of the user's input context (Р-60), and the flush must stay.
    assert!(
        !watchdog::focus_repeated(EVENT_SYSTEM_FOREGROUND, second, OBJID_CLIENT.0, 0),
        "EVENT_SYSTEM_FOREGROUND is not subject to the deduplication"
    );

    // And it does not clobber the memory either: the focus is still where it was, so the
    // frontmost churn that follows a foreground event is still recognised as churn.
    assert!(watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        second,
        OBJID_CLIENT.0,
        0
    ));

    assert_eq!(
        watchdog::counters().focus_repeats,
        before + 3,
        "exactly the three repeats above were counted"
    );
}

/// **Task Т-13-1 — the memory is the triple, and that is what gives browsers their probe back.**
///
/// The audit of 2026-08-24 (guard-watchdog, finding 1) established that comparing the window
/// alone made the deduplication a hole in SEC-06: in Chromium, Electron and Qt the whole page
/// is one window, so login → `Tab` → password raised `EVENT_OBJECT_FOCUS` with the **same**
/// handle, the event was dropped as churn, and `guard::note_focus_moved` — which only ever runs
/// on the `WM_APP_FLUSH` this decision gates, and which is the only thing that starts the three
/// checks of FR-72 — never ran. The verdict stayed `Ordinary` and the password went into the
/// typing buffer.
///
/// Two synthetic focus events with **one** hwnd and **different** `idChild` must therefore both
/// pass: two `WM_APP_FLUSH`, and so two probes. `win_event_proc` posts that message and the
/// layout question on the straight line right after this verdict, with nothing between them to
/// decide anything further, which is why the verdict is the whole of what there is to check
/// here; the same thing through a live subscription is
/// `the_same_hwnd_churn_with_moving_children_is_a_transfer_and_still_flushes` below, and live
/// in a real browser it is task Т-13-2.
///
/// The other half of the criterion is that task T-10-0e keeps what it bought: the **same**
/// triple over and over is still a repeat and still raises nothing.
///
/// The first half of the body is the number the repair is measured against, and it is measured
/// rather than argued: comparing the handle alone — the rule this function had before task
/// Т-13-1 — *is* this function with the element held constant, so driving the six events of one
/// page with one element counts what the old rule counted on the real six. Five repeats of six
/// events and a single one let through, against three and three after the repair.
#[test]
fn two_focus_events_in_one_window_with_different_children_are_both_flushes() {
    let _turn = focus_memory_turn();

    // One `Chrome_RenderWidgetHostHWND`: the page, its login field and its password field all
    // live behind this single handle, and the provider tells its elements apart by `idChild`
    // (Chromium assigns one per accessible node, negative).
    let page = HWND(0x1000_0F0A_usize as *mut core::ffi::c_void);
    // Another window, so that both passes below start from a memory holding neither the page
    // nor any element of it, whatever ran before them.
    let elsewhere = HWND(0x1000_0F09_usize as *mut core::ffi::c_void);
    const LOGIN: i32 = -3;
    const PASSWORD: i32 = -7;

    // ---- the number before the repair ---------------------------------------------------
    //
    // The same six events of one page, told apart by the handle alone. Only the first is let
    // through, so the one probe of the six describes the *login* field, and the password typed
    // after it goes into the buffer under an `Ordinary` verdict: the finding, in numbers.
    let mark = watchdog::counters().focus_repeats;

    assert!(!watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        elsewhere,
        OBJID_CLIENT.0,
        0
    ));

    for _ in 0..6 {
        watchdog::focus_repeated(EVENT_OBJECT_FOCUS, page, OBJID_CLIENT.0, 0);
    }

    assert_eq!(
        watchdog::counters().focus_repeats - mark,
        5,
        "the window-only rule takes five of the six events of one page for churn"
    );

    // ---- and after it ---------------------------------------------------------------------

    let before = watchdog::counters().focus_repeats;

    assert!(!watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        elsewhere,
        OBJID_CLIENT.0,
        0
    ));

    // The click into the login field. The memory holds another window, so this flushes and
    // probes.
    assert!(!watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        page,
        OBJID_CLIENT.0,
        LOGIN
    ));

    // `Tab` into the password field of the same page — the event of the finding. Before this
    // task it answered `true`, the probe of FR-72 was skipped and FR-70 with it.
    assert!(
        !watchdog::focus_repeated(EVENT_OBJECT_FOCUS, page, OBJID_CLIENT.0, PASSWORD),
        "FR-70, SEC-06: a move to another element of the same window is not churn — it must \
         reach the probe"
    );

    // The echo of *that* element — the churn task T-10-0e measured — is still a repeat, and so
    // is the next one: the saving of T-10-0e is untouched.
    assert!(watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        page,
        OBJID_CLIENT.0,
        PASSWORD
    ));
    assert!(
        watchdog::focus_repeated(EVENT_OBJECT_FOCUS, page, OBJID_CLIENT.0, PASSWORD),
        "the identical triple stays suppressed however often it repeats"
    );

    // `idObject` tells elements apart in its own right: the window's own object is not the
    // client area inside it, and one field differing is enough.
    assert!(
        !watchdog::focus_repeated(EVENT_OBJECT_FOCUS, page, OBJID_WINDOW.0, PASSWORD),
        "a triple that differs in idObject alone is not a repeat either"
    );

    // And the memory carries the whole triple forward: the event just seen is the one the next
    // is compared against, both halves of it.
    assert!(watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        page,
        OBJID_WINDOW.0,
        PASSWORD
    ));

    assert_eq!(
        watchdog::counters().focus_repeats,
        before + 3,
        "three of the same six events are repeats now — the identical triples — and the three \
         that move inside the page are not: two more probes than the rule above allowed, and \
         the one that matters is the move into the password field"
    );
}

/// **Task Т-13-1, the reverse direction: `Password` → `Ordinary` in one window returns
/// buffering.**
///
/// The finding broke both ways. A page that opens with the focus in its password field
/// publishes `Field::Password`, buffering is off (FR-70), and the user then `Tab`s out into an
/// ordinary field of the *same* page: with the window-only memory that event was dropped as
/// churn, no probe ran, the verdict stayed `Password` — and the program stopped recording for
/// as long as the page kept the focus.
///
/// The two halves of the repair are asserted here as one chain, because the second is what the
/// first is for:
///
/// 1. the move back to the ordinary element of the same window is **not** a repeat, so
///    `win_event_proc` posts `WM_APP_FLUSH`;
/// 2. answering that message is `guard::note_focus_moved`, and that call leaves the published
///    verdict behind whatever it was — `Pending` while a probe is asked for, and in a process
///    with no watcher window to ask, the FR-73 default, which is buffering **on**.
///
/// ⚠ What no unit test can stage is the *starting* verdict: `Field::Password` is published by
/// `guard::run_pending_probe`, which reads the real foreground window, so putting it there
/// needs a live password field in front — the stand of task Т-13-2. What is asserted instead is
/// the rule that governs it (`buffering_allowed_for(Field::Password)` is `false`, FR-70) and
/// the live state the chain actually reaches.
#[test]
fn a_move_out_of_a_password_element_in_one_window_returns_buffering() {
    let _turn = focus_memory_turn();

    let page = HWND(0x1000_0F0B_usize as *mut core::ffi::c_void);
    const PASSWORD: i32 = -11;
    const ORDINARY: i32 = -12;

    // The page opened with the focus in its password field: this is the event that made the
    // probe answer `Password`, and while that verdict stands nothing is recorded (FR-70).
    assert!(!watchdog::focus_repeated(
        EVENT_OBJECT_FOCUS,
        page,
        OBJID_CLIENT.0,
        PASSWORD
    ));
    assert!(
        !guard::buffering_allowed_for(Field::Password),
        "FR-70: with the password verdict standing, the buffer is not kept"
    );

    // `Tab` out into an ordinary field of the same page. One window, another element.
    assert!(
        !watchdog::focus_repeated(EVENT_OBJECT_FOCUS, page, OBJID_CLIENT.0, ORDINARY),
        "the way out of a password field inside one window must reach the probe, or the \
         verdict never changes and the program never records again"
    );

    // ...which is what the input thread answers with, on the `WM_APP_FLUSH` that event posts.
    guard::note_focus_moved();

    assert_ne!(
        guard::field(),
        Field::Password,
        "the password verdict does not survive a focus change"
    );
    assert!(
        guard::buffering_allowed(),
        "FR-73: nobody could answer in a test process, and an undeterminable field buffers"
    );
}

// ---------------------------------------------------------------------------------------
// Reading `src\` as text — the shape `tests\guard.rs` and `tests\selection.rs` settled on
// ---------------------------------------------------------------------------------------

/// One named module of `src\`, read as text with line endings normalised.
fn source_of(module: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(module);

    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));

    text.replace("\r\n", "\n")
}

/// The body of the item whose signature line starts with `signature` — from that line to the
/// first `}` in column 0 after it.
///
/// A region and not the whole module, for the reason `tests\selection.rs` gives its own cut: a
/// sweep over everything would count the prose of the module header and would be a sweep nobody
/// could keep green.
fn body_of<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source.find(signature).unwrap_or_else(|| {
        panic!("src\\watchdog.rs must contain \"{signature}\" — the sweep below is about it")
    });

    let rest = &source[start..];
    let end = rest
        .find("\n}")
        .expect("the item must end at a closing brace in column 0");

    let region = &rest[..end];

    assert!(
        !region.is_empty() && region.len() < rest.len(),
        "\"{signature}\" was bounded rather than taken as the rest of the module"
    );

    region
}

/// Lines of `text` that contain `needle` and are not comment lines.
///
/// The point of these sweeps is to separate a **call** from a sentence about a call: this module
/// documents its rules at length, and a sweep that counted prose would assert nothing.
fn code_lines_with<'a>(text: &'a str, needle: &str) -> Vec<(usize, &'a str)> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.starts_with("//"))
        .filter(|(_, line)| line.contains(needle))
        .collect()
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

// ---------------------------------------------------------------------------------------
// Task T-10-0e — the focus memory against the frontmost window's own churn
// ---------------------------------------------------------------------------------------
//
// ⚠ The `WINEVENT_SKIPOWNPROCESS` trap of the section above applies with full force: the
// generator of the same-hwnd events MUST be a foreign process, or the subscription never
// sees them and a green test has checked nothing. `NotifyWinEvent` is what makes the
// generator deterministic: a foreign helper raises `EVENT_OBJECT_FOCUS` with exactly the
// hwnd it chooses, on exactly the schedule it chooses, without the focus actually moving.
// The positive control is `focus_repeats`: growth there is the proof of delivery.

/// The helper of the same-hwnd churn: takes the foreground honestly, holds it, and raises
/// `EVENT_OBJECT_FOCUS` for its own top-level window every 40 ms — the frontmost-window
/// churn of the acceptance machine (measured: same hwnd all 24 times), staged at a higher
/// rate from a foreign process.
const REPEATER_PS1: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class RepeatNative
{
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern void NotifyWinEvent(uint ev, IntPtr hwnd, int idObject, int idChild);
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
$form.Text = 'LangSw-T10e-repeater'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(600, 40)
$form.Size = New-Object System.Drawing.Size(240, 100)
$hold = New-Object System.Windows.Forms.Timer
$hold.Interval = 400
$hold.Add_Tick({ [RepeatNative]::TakeForeground($form.Handle) })
$churn = New-Object System.Windows.Forms.Timer
$churn.Interval = 40
$churn.Add_Tick({ [RepeatNative]::NotifyWinEvent(0x8005, $form.Handle, -4, 0) })
$stop = New-Object System.Windows.Forms.Timer
$stop.Interval = 60000
$stop.Add_Tick({ [System.Windows.Forms.Application]::Exit() })
$form.Add_Shown({ [RepeatNative]::TakeForeground($form.Handle); $hold.Start(); $churn.Start(); $stop.Start() })
[System.Windows.Forms.Application]::Run($form)
"#;

/// The control helper: the same foreign foreground window, but the events alternate between
/// the hwnds of its two child controls every 150 ms — every event a transfer to a
/// *different* hwnd, which the memory must keep flushing on.
const ALTERNATOR_PS1: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class AltNative
{
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern void NotifyWinEvent(uint ev, IntPtr hwnd, int idObject, int idChild);
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
$form.Text = 'LangSw-T10e-alternator'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(600, 180)
$form.Size = New-Object System.Drawing.Size(260, 140)
$tb1 = New-Object System.Windows.Forms.TextBox
$tb1.Location = New-Object System.Drawing.Point(10, 10)
$tb2 = New-Object System.Windows.Forms.TextBox
$tb2.Location = New-Object System.Drawing.Point(10, 40)
$form.Controls.Add($tb1)
$form.Controls.Add($tb2)
$hold = New-Object System.Windows.Forms.Timer
$hold.Interval = 400
$hold.Add_Tick({ [AltNative]::TakeForeground($form.Handle) })
$script:flip = $false
$churn = New-Object System.Windows.Forms.Timer
$churn.Interval = 150
$churn.Add_Tick({
    $script:flip = -not $script:flip
    $target = if ($script:flip) { $tb1.Handle } else { $tb2.Handle }
    [AltNative]::NotifyWinEvent(0x8005, $target, -4, 0)
})
$stop = New-Object System.Windows.Forms.Timer
$stop.Interval = 60000
$stop.Add_Tick({ [System.Windows.Forms.Application]::Exit() })
$form.Add_Shown({ [AltNative]::TakeForeground($form.Handle); $hold.Start(); $churn.Start(); $stop.Start() })
[System.Windows.Forms.Application]::Run($form)
"#;

/// The helper of task **Т-13-1**: the same foreign foreground window and the same handle in
/// every event — but the **element** moves, `idChild` alternating every 150 ms. This is what a
/// browser raises when the user `Tab`s between the fields of one page: one
/// `Chrome_RenderWidgetHostHWND`, another node of the accessibility tree. Every one of these is
/// a transfer and must flush and probe.
const CHILD_ALTERNATOR_PS1: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ChildNative
{
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")] static extern bool AttachThreadInput(uint a, uint b, bool attach);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] public static extern void NotifyWinEvent(uint ev, IntPtr hwnd, int idObject, int idChild);
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
$form.Text = 'LangSw-T13-1-children'
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(600, 320)
$form.Size = New-Object System.Drawing.Size(260, 100)
$hold = New-Object System.Windows.Forms.Timer
$hold.Interval = 400
$hold.Add_Tick({ [ChildNative]::TakeForeground($form.Handle) })
$script:flip = $false
$churn = New-Object System.Windows.Forms.Timer
$churn.Interval = 150
$churn.Add_Tick({
    $script:flip = -not $script:flip
    $child = if ($script:flip) { -3 } else { -7 }
    [ChildNative]::NotifyWinEvent(0x8005, $form.Handle, -4, $child)
})
$stop = New-Object System.Windows.Forms.Timer
$stop.Interval = 60000
$stop.Add_Tick({ [System.Windows.Forms.Application]::Exit() })
$form.Add_Shown({ [ChildNative]::TakeForeground($form.Handle); $hold.Start(); $churn.Start(); $stop.Start() })
[System.Windows.Forms.Application]::Run($form)
"#;

/// **Criterion 12, the churn half.** A foreign process holds the foreground and re-raises
/// `EVENT_OBJECT_FOCUS` for the same window over and over — the frontmost-window churn that
/// erased the typing on the acceptance machine. The subscription must see every event
/// (positive control: `focus_repeats` grows), raise not a single flush request, and leave
/// the typed strokes exactly where they were.
///
/// Before the memory of task T-10-0e this scenario read `window_flushes` up with every
/// event and the buffer emptied — section 3 of the task report carries the live pre-repair
/// run (11 of 36 strokes erased, one hotkey press of six on an empty buffer).
#[test]
#[ignore = "spawns PowerShell helper windows and takes the foreground; run deliberately with --ignored --test-threads=1"]
fn the_same_hwnd_churn_of_the_foreground_window_is_turned_away_and_the_strokes_survive() {
    let repeater = SpawnedScript::spawn("langsw-t10e-repeater.ps1", REPEATER_PS1);

    assert!(
        wait_until(Duration::from_secs(10), || foreground_pid()
            == repeater.id()),
        "the repeater window never became the foreground; its churn would be background \
         churn and the scenario would be the one of task T-10-0, not this one"
    );

    let watching = watchdog::watch().expect("the subscriptions must install");

    buffer::install_recorder(Recorder::with_capacity(16));

    // Priming: the first delivered event finds the memory empty (or stale) and legitimately
    // flushes once while storing the hwnd; every event after it is a repeat. Growth of
    // `focus_repeats` here is already the positive control that the churn is being
    // delivered into this process.
    let start = watchdog::counters();

    assert!(
        pump_until(Duration::from_secs(10), || {
            watchdog::counters().focus_repeats - start.focus_repeats >= 1
        }),
        "positive control failed: the staged churn never reached the subscription — \
         a green verdict here would have checked nothing (WINEVENT_SKIPOWNPROCESS)"
    );

    // Take whatever the priming event left pending, so the assertions below are about the
    // steady churn and nothing else.
    let _ = watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0));

    // What the user had typed. The stamps do not matter: nothing may flush from here on.
    for time in [10, 20, 30, 40, 50, 60] {
        press_at(time);
    }
    assert_eq!(buffer::len(), 6);

    let before = watchdog::counters();

    let seen = pump_until(Duration::from_secs(12), || {
        watchdog::counters().focus_repeats - before.focus_repeats >= 20
    });

    let counted = delta(before, watchdog::counters());

    assert!(
        seen,
        "positive control failed: {} repeats in 12 s — the churn stopped being delivered",
        counted.focus_repeats
    );

    assert_eq!(
        counted.window_flushes, 0,
        "the same-hwnd churn of the foreground window must not raise a single flush \
         request (T-10-0e)"
    );
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        None,
        "and must leave no flush pending for the input thread"
    );
    assert_eq!(
        buffer::len(),
        6,
        "the strokes typed under the frontmost churn survived it — the defect of the \
         acceptance session, repaired"
    );

    buffer::uninstall();
    drop(watching);
}

/// **Criterion 12, the transfer half.** The same foreign foreground window, but every event
/// names a *different* hwnd than the one before it — a real focus transfer between two
/// controls. The memory must not eat these: each one is a change of the user's input
/// target, and the flush of FR-10 must arrive and empty the buffer.
#[test]
#[ignore = "spawns PowerShell helper windows and takes the foreground; run deliberately with --ignored --test-threads=1"]
fn a_focus_transfer_to_a_different_hwnd_by_a_foreign_process_still_flushes() {
    let alternator = SpawnedScript::spawn("langsw-t10e-alternator.ps1", ALTERNATOR_PS1);

    assert!(
        wait_until(Duration::from_secs(10), || {
            foreground_pid() == alternator.id()
        }),
        "the alternator window never became the foreground"
    );

    let watching = watchdog::watch().expect("the subscriptions must install");

    buffer::install_recorder(Recorder::with_capacity(16));

    let start = watchdog::counters();

    // Delivery first: without at least one flush request there is nothing to judge.
    assert!(
        pump_until(Duration::from_secs(15), || {
            watchdog::counters().window_flushes - start.window_flushes >= 1
        }),
        "no alternating event was delivered; FR-10 cannot be judged by this run"
    );

    let _ = watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0));

    // Strokes stamped from the message stream this thread has just pumped — the tick domain
    // the event timestamps live in (FR-12).
    //
    // SAFETY: takes no arguments, touches no memory of ours, and returns the tick count of
    // the last message this thread retrieved; the queue was pumped a moment ago. The
    // `as u32` is the documented reinterpretation of a tick the system hands back signed.
    let typed_at = unsafe { GetMessageTime() } as u32;

    // ⚠ **Five seconds back, and that is the canon of question 111.4.** Until task Т-48-2 these
    // strokes were stamped «now», and the assertion below then read "a focus transfer flushes
    // what the user typed a moment ago" — which is precisely what FR-14 now denies, because an
    // application raising a focus event milliseconds after a keystroke is answering the
    // keystroke. What FR-10 still says, and what this test is now the statement of, is that a
    // focus transfer flushes typing the window of FR-14 no longer covers.
    for _ in 0..6 {
        press_at(typed_at.wrapping_sub(5_000));
    }
    assert_eq!(buffer::len(), 6);

    let mid = watchdog::counters();

    // Two more events guarantee at least one raised strictly after the strokes.
    assert!(
        pump_until(Duration::from_secs(15), || {
            watchdog::counters().window_flushes - mid.window_flushes >= 2
        }),
        "the alternating churn stopped arriving"
    );

    assert!(
        !watchdog::take_typing_induced_flush(WM_APP_FLUSH),
        "FR-14 does not reach typing that is older than Δ"
    );
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Cleared { removed: 6 }),
        "FR-10: a focus transfer to a different hwnd still flushes what was typed before it"
    );
    assert_eq!(buffer::len(), 0);

    buffer::uninstall();
    drop(watching);
}

/// **Task Т-13-1 through a real subscription.** The same foreign foreground window and the
/// **same handle** in every event, with only `idChild` moving — the shape a browser raises when
/// the user moves between the fields of one page, and the shape the window-only memory of task
/// T-10-0e ate whole.
///
/// Every event must be taken for the transfer it is: `window_flushes` grows — that is the
/// `WM_APP_FLUSH` behind which `guard::note_focus_moved` starts the probe of FR-72 — and
/// `focus_repeats` does not, because not one of these events repeats the triple before it.
/// The unit-level statement of the same rule is
/// `two_focus_events_in_one_window_with_different_children_are_both_flushes`; live in a browser
/// with a real `<input type="password">` it is task Т-13-2.
#[test]
#[ignore = "spawns PowerShell helper windows and takes the foreground; run deliberately with --ignored --test-threads=1"]
fn the_same_hwnd_churn_with_moving_children_is_a_transfer_and_still_flushes() {
    let children = SpawnedScript::spawn("langsw-t13-1-children.ps1", CHILD_ALTERNATOR_PS1);

    assert!(
        wait_until(Duration::from_secs(10), || {
            foreground_pid() == children.id()
        }),
        "the helper window never became the foreground; its events would be background churn \
         and the scenario would be the one of task T-10-0, not this one"
    );

    let watching = watchdog::watch().expect("the subscriptions must install");

    buffer::install_recorder(Recorder::with_capacity(16));

    let start = watchdog::counters();

    // Delivery first: without at least one flush request there is nothing to judge.
    assert!(
        pump_until(Duration::from_secs(15), || {
            watchdog::counters().window_flushes - start.window_flushes >= 1
        }),
        "positive control failed: no same-hwnd event with a moving child was delivered — \
         before task Т-13-1 this is exactly what the run looked like, and it is what the \
         defect was (WINEVENT_SKIPOWNPROCESS is the other way to get here)"
    );

    let _ = watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0));

    // SAFETY: takes no arguments, touches no memory of ours, and returns the tick count of the
    // last message this thread retrieved; the queue was pumped a moment ago. The `as u32` is
    // the documented reinterpretation of a tick the system hands back signed.
    let typed_at = unsafe { GetMessageTime() } as u32;

    // ⚠ **Five seconds back — the canon of question 111.4**, the same rewrite as in
    // `a_focus_transfer_to_a_different_hwnd_by_a_foreign_process_still_flushes` and for the same
    // reason. This is the very shape of the user's finding: the same window, the child moving,
    // milliseconds after a keystroke. FR-14 now exempts that shape, and what stays canon is the
    // shape without the typing behind it.
    for _ in 0..6 {
        press_at(typed_at.wrapping_sub(5_000));
    }
    assert_eq!(buffer::len(), 6);

    let mid = watchdog::counters();

    // Two more events guarantee at least one raised strictly after the strokes.
    assert!(
        pump_until(Duration::from_secs(15), || {
            watchdog::counters().window_flushes - mid.window_flushes >= 2
        }),
        "the alternating children stopped arriving"
    );

    let counted = delta(mid, watchdog::counters());

    assert_eq!(
        counted.focus_repeats, 0,
        "not one of these events repeats the triple before it, so not one is churn"
    );
    assert!(
        !watchdog::take_typing_induced_flush(WM_APP_FLUSH),
        "FR-14 does not reach typing that is older than Δ"
    );
    assert_eq!(
        watchdog::apply_flush(WM_APP_FLUSH, LPARAM(0)),
        Some(ResetOutcome::Cleared { removed: 6 }),
        "FR-10: a move to another element of the same window flushes what was typed before it"
    );
    assert_eq!(buffer::len(), 0);

    buffer::uninstall();
    drop(watching);
}

// ---------------------------------------------------------------------------------------
// Task T-52-4 — FR-15: the idle timeout rides on the liveness tick of FR-80
// ---------------------------------------------------------------------------------------

/// **FR-15 through the module that owns the heartbeat** — task T-52-4.
///
/// The rule itself lives in `buffer` and is measured there, one stroke and one moment at a time.
/// What belongs here is the wiring: the thirty-second tick of FR-80 asks the question, the
/// answer is counted beside the other flush counts of the FR-10 table, and no timer of its own
/// was created for it.
///
/// ⚠ **The `WM_TIMER` arm cannot be driven from a test** — the reinstallation beside this call
/// would put a `WH_KEYBOARD_LL` hook into a process that pumps no messages, which section 4.11
/// forbids and which would freeze the keyboard of whoever is running `cargo test`. So the call
/// is made directly, with a moment of this test's own choosing, and that the arm makes it is
/// pinned by reading the source below.
#[test]
fn the_idle_timeout_of_fr15_is_asked_and_counted_by_the_watchdog() {
    let _turn = notice_turn();

    buffer::install_recorder(Recorder::with_capacity(16));

    // An empty buffer is not a flush, however long ago the moment is.
    let before = watchdog::counters();
    assert!(!watchdog::flush_buffer_if_idle(9_000_000));
    assert_eq!(
        delta(before, watchdog::counters()).idle_flushes,
        0,
        "пустой буфер не сброс"
    );

    // A word, and a tick that is not late enough.
    for time in [1_000_000, 1_000_001, 1_000_002] {
        press_at(time);
    }

    assert_eq!(buffer::len(), 3, "a half-typed word is in the ring");

    let before = watchdog::counters();
    assert!(!watchdog::flush_buffer_if_idle(1_000_002 + 299_000));
    assert_eq!(buffer::len(), 3, "не раньше срока");
    assert_eq!(delta(before, watchdog::counters()).idle_flushes, 0);

    // And a tick that is.
    let before = watchdog::counters();
    assert!(watchdog::flush_buffer_if_idle(1_000_002 + 300_000));

    assert_eq!(buffer::len(), 0, "FR-15: полный сброс");
    assert_eq!(
        non_zero_slots(),
        Some(0),
        "SEC-02: обнуление памяти, как у всякого сброса FR-10"
    );
    assert_eq!(
        delta(before, watchdog::counters()).idle_flushes,
        1,
        "сброс по покою не посчитан"
    );

    buffer::uninstall();
}

/// **The tick is the only clock FR-15 has, and no second timer was created for it** — task
/// T-52-4.
///
/// Two claims about the source, and both of them are what the requirement's precision rests on:
/// the arm of the thirty-second timer asks the question, so a buffer is emptied no later than
/// `idle_timeout_s + 30 s`; and this module creates exactly the timers it created before, so
/// there is no second heartbeat to keep alive and nothing new on the callback path.
#[test]
fn fr15_rides_on_the_liveness_tick_and_adds_no_timer() {
    let source = source_of("watchdog.rs");

    let arm = source
        .split("WM_TIMER if wparam.0 == LIVENESS_TIMER_ID")
        .nth(1)
        .expect("the liveness arm is still where FR-80 put it");
    let arm = arm
        .split("WM_APP_REHOOK")
        .next()
        .expect("the arm ends before the next one");

    assert!(
        arm.contains("flush_buffer_if_idle("),
        "FR-15: такт живости не спрашивает про покой буфера"
    );

    // `SetTimer` is called for the liveness timer of FR-80 and for the one-shot of the probe,
    // and for nothing else. A third would be a new heartbeat, which this task was not to build.
    let timers: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .filter(|line| line.contains("SetTimer("))
        .collect();

    assert_eq!(
        timers.len(),
        2,
        "FR-15 не должен был заводить своего таймера, а SetTimer теперь зовётся так: {timers:?}"
    );

    // And the callback path is untouched: the flush is asked for on the message loop.
    let hook = source_of("hook.rs");

    assert!(
        !hook.contains("flush_if_idle"),
        "FR-15 не должен появляться на пути callback"
    );
}
