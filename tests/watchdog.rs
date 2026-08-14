//! Integration tests for task T-03-3 — the subscriptions of module `watchdog`.
//!
//! # What is tested here and what cannot be
//!
//! Everything that decides something is a function of its arguments and is driven directly:
//! whether a `RAWMOUSE` button word is a press or a movement (FR-10, FR-13), which `WinEvent`
//! codes the flush table lists, what each message of the FR-21 delivery asks for, and how two
//! flush requests coalesce across the wrap of the millisecond counter (FR-12).
//!
//! Two tests do touch Win32, because the value of checking them is precisely that they are
//! real: the Raw Input registration of FR-13 with `RIDEV_INPUTSINK`, and the two
//! `SetWinEventHook` subscriptions. Both are installed and immediately removed, on a window
//! this file creates and destroys.
//!
//! **No test here installs a keyboard hook**, for the reason `tests\hook.rs` states: a
//! `WH_KEYBOARD_LL` hook in a process that is not pumping messages freezes the keyboard of
//! whoever is running `cargo test`. A `WinEvent` hook has no such property — it is
//! `WINEVENT_OUTOFCONTEXT`, so the system posts to a queue instead of calling into us, and a
//! queue nobody pumps is a queue nobody waits on.
//!
//! What no test can show is the part of FR-12 that is about the *world*: that
//! `KBDLLHOOKSTRUCT.time`, `GetMessageTime` and `dwmsEventTime` really are the same counter.
//! That is argued in the report of task T-03-3 and measured on the running program.

use lang_switcher::buffer::{self, Recorder, ResetOutcome};
use lang_switcher::hook::{Edge, KeyEvent};
use lang_switcher::layouts;
use lang_switcher::watchdog::{self, Counters, FLUSH_EVENTS, Rebuild, WM_APP_FLUSH, WM_APP_LAYOUT};
use windows::Win32::Foundation::{HINSTANCE, LPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, HWND_MESSAGE,
    RI_MOUSE_BUTTON_1_DOWN, RI_MOUSE_BUTTON_1_UP, RI_MOUSE_BUTTON_2_DOWN, RI_MOUSE_BUTTON_2_UP,
    RI_MOUSE_BUTTON_3_DOWN, RI_MOUSE_BUTTON_3_UP, RI_MOUSE_BUTTON_4_DOWN, RI_MOUSE_BUTTON_4_UP,
    RI_MOUSE_BUTTON_5_DOWN, RI_MOUSE_BUTTON_5_UP, RI_MOUSE_HWHEEL, RI_MOUSE_WHEEL, WINDOW_STYLE,
    WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_INPUTLANGCHANGE,
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

#[test]
fn the_raw_input_registration_of_fr13_is_accepted_and_withdrawn() {
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
