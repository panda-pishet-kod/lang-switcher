//! Integration checks of module `selection` — the clipboard primitives, FR-62, FR-63, FR-64.
//!
//! Task **T-07-1**.
//!
//! # ⚠ The clipboard in this file belongs to whoever is at the machine
//!
//! Every test below that **changes** the clipboard is marked `#[ignore]` and carries the reason
//! in its attribute, the convention `tests\inject.rs`, `tests\switch.rs` and `tests\watchdog.rs`
//! already use for checks that alter machine-wide state. `cargo test` therefore leaves the
//! user's clipboard alone; the ignored set is run deliberately, with `--ignored
//! --test-threads=1`, and its output is in the report of T-07-1.
//!
//! The checks that are **not** ignored open the clipboard and read it, which changes nothing an
//! application can observe and does not move `GetClipboardSequenceNumber`.
//!
//! Even so, every test that writes takes a [`Keeper`] first. It snapshots the clipboard on the
//! way in and restores it from `Drop` on the way out — including when the test panics, which is
//! the same argument `selection::Clipboard` makes about `CloseClipboard` and is the reason the
//! restore is a destructor rather than a last line.
//!
//! # What is here and what is not
//!
//! The parts of the module that are functions of their arguments — the format classification of
//! FR-64, the block encoding, the two timings of section 7, the `Debug` that may not print
//! content — are driven by the unit tests inside `src\selection.rs`, together with the two that
//! need the private `Clipboard` guard: the close on the ordinary path and **the close on the
//! panicking path**, acceptance point 14.
//!
//! What needs a live clipboard, a live window and more than one thread is here.
//!
//! What is **not** here: the selection path itself. FR-60, FR-61 as a whole and FR-65 are task
//! T-07-2, and the last group of tests in this file is a sweep that checks this task did not
//! start it.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, path::Path};

use lang_switcher::selection::{
    self, CF_UNICODETEXT, ClipboardError, OPEN_ATTEMPTS, OPEN_RETRY_INTERVAL, Origin,
    SNAPSHOT_BUDGET_BYTES, Snapshot, Update, WORST_CASE_OPEN, Wait,
};
use lang_switcher::settings;

use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW,
    SetClipboardData,
};
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_MESSAGE, MSG, PM_REMOVE, PeekMessageW, WINDOW_EX_STYLE,
    WINDOW_STYLE,
};
use windows::core::w;

// ---------------------------------------------------------------------------------------
// Serialisation, windows and the clipboard of whoever is at the machine
// ---------------------------------------------------------------------------------------

/// Serialises everything in this file that touches the clipboard or the module's process-global
/// counters.
///
/// `cargo test` runs the tests of a binary in parallel and there is exactly one clipboard on the
/// machine and one set of counters in the process. The same device, and the same reason, as
/// `GLOBAL_STATE` in `tests\guard.rs`.
static CLIPBOARD: Mutex<()> = Mutex::new(());

fn serialised() -> MutexGuard<'static, ()> {
    CLIPBOARD.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A message-only window of the predefined `STATIC` class, destroyed with the value.
///
/// `OpenClipboard` and `AddClipboardFormatListener` both want a window handle of the calling
/// thread; neither wants a window procedure of ours, so no class is registered here.
struct TestWindow(HWND);

impl TestWindow {
    fn create() -> Self {
        // SAFETY: `STATIC` is a predefined class that is always registered; the window name is a
        // `'static` literal; the parent is `HWND_MESSAGE`, which asks for a message-only window.
        // No `lpParam` is passed, so nothing of ours reaches the class's window procedure.
        // NFR-13: the binding turns a null handle into `Err`.
        let handle = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("langsw-selection-test"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .expect("a message-only window of the STATIC class");

        Self(handle)
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful `CreateWindowExW` on this thread and is
        // destroyed exactly once — the type is neither `Copy` nor `Clone`.
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

/// Holds the clipboard of whoever is at the machine and puts it back — from `Drop`.
///
/// ⚠ Every test in this file that writes to the clipboard builds one of these **first**. The
/// restore is a destructor and not a last line for the reason the module makes about
/// `CloseClipboard`: a test that fails in the middle is exactly the case where somebody's
/// clipboard would otherwise be left holding `ghbdtn`.
struct Keeper {
    window: HWND,
    held: Snapshot,
}

impl Keeper {
    fn take(window: HWND) -> Self {
        let held = selection::snapshot(window).unwrap_or_else(|_| Snapshot::empty());

        Self { window, held }
    }
}

impl Drop for Keeper {
    fn drop(&mut self) {
        let _restored = selection::restore(self.window, &self.held);
    }
}

// ---------------------------------------------------------------------------------------
// An independent oracle: raw clipboard access that does not go through the module under test
// ---------------------------------------------------------------------------------------

/// Opens the clipboard the plain way, for the helpers below. Closed by [`RawAccess::drop`].
struct RawAccess;

impl RawAccess {
    fn open(window: HWND) -> Option<Self> {
        for _ in 0..OPEN_ATTEMPTS {
            // SAFETY: `window` is a live window of this thread; the call takes the clipboard
            // lock and dereferences nothing of ours.
            if unsafe { OpenClipboard(Some(window)) }.is_ok() {
                return Some(Self);
            }

            thread::sleep(OPEN_RETRY_INTERVAL);
        }

        None
    }
}

impl Drop for RawAccess {
    fn drop(&mut self) {
        // SAFETY: this thread opened the clipboard, and the value is neither `Copy` nor `Clone`,
        // so it is closed exactly once.
        let _ = unsafe { CloseClipboard() };
    }
}

/// The bytes of `format` as the system holds them, read without the module under test.
///
/// This is the oracle acceptance point 22 needs: "побитово" has to be checked by something other
/// than the code that did the copying.
fn raw_read(window: HWND, format: u32) -> Option<Vec<u8>> {
    let _access = RawAccess::open(window)?;

    // SAFETY: the clipboard is open. The handle belongs to the clipboard and is borrowed for as
    // long as it stays open; nothing frees it here.
    let handle = unsafe { GetClipboardData(format) }.ok()?;
    let block = HGLOBAL(handle.0);

    // SAFETY: `block` is a live movable block of the open clipboard.
    let size = unsafe { GlobalSize(block) };

    if size == 0 {
        return None;
    }

    // SAFETY: `block` is live; `GlobalLock` pins it and answers a pointer to at least `size`.
    let pointer = unsafe { GlobalLock(block) };

    if pointer.is_null() {
        return None;
    }

    let mut bytes = vec![0_u8; size];

    // SAFETY: `pointer` is non-null and points at `size` readable bytes, which came from
    // `GlobalSize` on this handle under the same lock. The destination is a fresh vector of
    // exactly `size` bytes and cannot overlap the source.
    unsafe {
        core::ptr::copy_nonoverlapping(pointer.cast::<u8>(), bytes.as_mut_ptr(), size);
    }

    // SAFETY: the block was locked on the line above and is unlocked once.
    let _ = unsafe { GlobalUnlock(block) };

    Some(bytes)
}

/// Replaces the clipboard with `entries`, without the module under test.
///
/// ⚠ Writes to the machine's clipboard. Only ever called from a test that holds a [`Keeper`].
fn raw_write(window: HWND, entries: &[(u32, Vec<u8>)]) -> bool {
    let Some(_access) = RawAccess::open(window) else {
        return false;
    };

    // SAFETY: the clipboard is open with a window of this thread, which is what `EmptyClipboard`
    // needs in order to make that window the owner.
    if unsafe { EmptyClipboard() }.is_err() {
        return false;
    }

    for (format, bytes) in entries {
        // SAFETY: a plain movable allocation of `bytes.len()` bytes, which is non-zero for every
        // entry the tests below build.
        let Ok(block) = (unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }) else {
            return false;
        };

        // SAFETY: `block` was just allocated and is locked once here.
        let pointer = unsafe { GlobalLock(block) };

        if pointer.is_null() {
            return false;
        }

        // SAFETY: `pointer` points at a region of at least `bytes.len()` bytes — the length the
        // allocation was made with — and the two regions are distinct allocations.
        unsafe {
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
        }

        // SAFETY: the block was locked on the line above.
        let _ = unsafe { GlobalUnlock(block) };

        // SAFETY: the clipboard is open and owned by this thread; `block` is a movable block
        // whose ownership passes to the system on success.
        if unsafe { SetClipboardData(*format, Some(HANDLE(block.0))) }.is_err() {
            return false;
        }
    }

    true
}

/// The UTF-16 bytes of `text`, terminator included — the shape of a `CF_UNICODETEXT` block.
fn text_block(text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((text.len() + 1) * 2);

    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }

    bytes.extend_from_slice(&0_u16.to_le_bytes());

    bytes
}

/// A registered clipboard format, by name — `RegisterClipboardFormatW` answers the same number
/// for the same name for every process in the session.
fn registered(name: windows::core::PCWSTR) -> u32 {
    // SAFETY: `name` is a `'static` wide literal; the call registers or looks up the format and
    // dereferences nothing of ours.
    unsafe { RegisterClipboardFormatW(name) }
}

/// Takes every `WM_CLIPBOARDUPDATE` waiting at `window`, classifying each the way
/// `app::window_proc` does.
///
/// **This is the measurement of acceptance point 16.** `AddClipboardFormatListener` posts the
/// message to the window named in the registration, so a message that arrives is one this loop
/// can take out of the queue; a registration that did not work produces an empty answer here.
fn drain_updates(window: HWND, budget: Duration) -> Vec<Update> {
    let deadline = Instant::now() + budget;
    let mut seen = Vec::new();

    while Instant::now() < deadline {
        let mut message = MSG::default();

        // SAFETY: `message` is a live local the call fills in; `window` is a live window of this
        // thread, which is the filter `PeekMessageW` applies. `PM_REMOVE` takes the message out
        // of the queue.
        while unsafe { PeekMessageW(&mut message, Some(window), 0, 0, PM_REMOVE) }.as_bool() {
            if let Some(update) = selection::handle_clipboard_message(message.hwnd, message.message)
            {
                seen.push(update);
            }
        }

        if !seen.is_empty() {
            return seen;
        }

        thread::sleep(Duration::from_millis(5));
    }

    seen
}

// ---------------------------------------------------------------------------------------
// FR-64 — the snapshot covers everything the clipboard lists
// ---------------------------------------------------------------------------------------

/// **Acceptance point 9.** A snapshot is of *all* formats, not of the text.
///
/// Read-only: it snapshots whatever the machine's clipboard happens to hold and asserts that
/// every listed format is accounted for — captured, or a handle format, or refused. Nothing is
/// written, so the check is safe to run on somebody's working clipboard.
#[test]
fn a_snapshot_accounts_for_every_format_the_clipboard_lists() {
    let _serialised = serialised();
    let window = TestWindow::create();

    let snapshot = match selection::snapshot(window.0) {
        Ok(snapshot) => snapshot,
        // A clipboard held by another process for the whole 180 ms of FR-62 is not this test's
        // subject and is not a failure of it.
        Err(ClipboardError::Busy) => return,
        Err(other) => panic!("the snapshot refused: {other}"),
    };

    assert_eq!(
        snapshot.captured_formats() + snapshot.handle_formats() + snapshot.refused_formats(),
        snapshot.listed_formats(),
        "every listed format is either captured or explained"
    );

    // Nothing captured is a handle format, and every captured one is memory-backed.
    for format in snapshot.formats() {
        assert!(
            selection::is_memory_format(format),
            "format {format} was captured but is not a memory format"
        );
    }

    assert!(snapshot.total_bytes() <= SNAPSHOT_BUDGET_BYTES || snapshot.is_truncated());
}

/// **Acceptance point 10, the arithmetic half.** The budget is one number for the whole
/// clipboard, and the module publishes it as the four megabytes FR-64 writes.
#[test]
fn the_budget_of_fr64_is_four_megabytes_for_everything_together() {
    assert_eq!(SNAPSHOT_BUDGET_BYTES, 4 * 1024 * 1024);

    // And a snapshot of an ordinary clipboard is nowhere near it, which is what makes the rule a
    // safety net rather than a limit users meet.
    let _serialised = serialised();
    let window = TestWindow::create();

    if let Ok(snapshot) = selection::snapshot(window.0) {
        assert!(!snapshot.is_truncated() || snapshot.total_bytes() > SNAPSHOT_BUDGET_BYTES);
    }
}

// ---------------------------------------------------------------------------------------
// FR-62 — the retries, and the close that always happens
// ---------------------------------------------------------------------------------------

/// **Acceptance point 13.** Ten attempts, twenty milliseconds apart, on an access that is held.
///
/// The clipboard is held by **another thread** here rather than another process, which is the
/// same thing as far as `OpenClipboard` is concerned: the lock belongs to a thread, and a second
/// thread — of this process or any other — is refused while it is held. The external-process
/// version of the same check is behavioural point 25 of the task specification and is in the
/// report.
///
/// Read-only: the holding thread opens and closes, and writes nothing.
#[test]
fn a_held_clipboard_is_retried_ten_times_and_then_refused() {
    let _serialised = serialised();
    let window = TestWindow::create();

    let (ready, held) = std::sync::mpsc::channel::<()>();
    let (release, wait_for_release) = std::sync::mpsc::channel::<()>();

    let holder = thread::spawn(move || {
        let holder_window = TestWindow::create();
        let Some(access) = RawAccess::open(holder_window.0) else {
            let _ = ready.send(());
            return false;
        };

        let _ = ready.send(());
        let _ = wait_for_release.recv();

        drop(access);

        true
    });

    held.recv().expect("the holding thread reports in");

    let before = selection::counters();
    let started = Instant::now();
    let outcome = selection::snapshot(window.0);
    let elapsed = started.elapsed();

    let _ = release.send(());
    let holding_worked = holder.join().unwrap_or(false);

    if !holding_worked {
        // The other thread could not take the clipboard, so there was nothing to be refused by.
        return;
    }

    assert!(
        matches!(outcome, Err(ClipboardError::Busy)),
        "a held clipboard answers Busy, not a snapshot"
    );

    let after = selection::counters();

    // Nine sleeps between ten attempts — FR-62 as the module reads it.
    assert_eq!(
        after.open_retries - before.open_retries,
        OPEN_ATTEMPTS - 1,
        "ten attempts have nine intervals"
    );
    assert_eq!(after.open_refusals - before.open_refusals, 1);

    // And the time really was spent waiting rather than spinning.
    assert!(
        elapsed >= WORST_CASE_OPEN,
        "the retries took {elapsed:?}, less than the {WORST_CASE_OPEN:?} of FR-62"
    );

    // **Acceptance point 14 on the refused path**: nothing was opened, so nothing is left open.
    assert_eq!(after.opens, before.opens);
    assert_eq!(after.closes, before.closes);
}

/// **Acceptance point 14.** Across a run of this whole file, every open is matched by a close.
///
/// The counters are process-global, so this asserts the invariant rather than a delta: at a
/// moment when no access is in flight — this test holds the mutex, so there is none — the two
/// numbers are equal. A single missing `CloseClipboard` anywhere in the module, on any path any
/// test in this file took, separates them for ever.
#[test]
fn every_open_of_the_module_has_been_closed_again() {
    let _serialised = serialised();
    let window = TestWindow::create();

    // A few accesses of every read-only shape the module has, so that the invariant is asserted
    // over something rather than over nothing.
    let _ = selection::snapshot(window.0);
    let _ = selection::read_unicode_text(window.0);
    let _ = selection::snapshot(window.0);

    let counters = selection::counters();

    assert_eq!(
        counters.opens, counters.closes,
        "every OpenClipboard of this module was followed by a CloseClipboard"
    );
    assert_eq!(
        counters.close_failures, 0,
        "no CloseClipboard failed; a non-zero value here means the machine lost Ctrl+C"
    );
}

// ---------------------------------------------------------------------------------------
// FR-80, NFR-01 — where the waiting may happen
// ---------------------------------------------------------------------------------------

/// **Acceptance point 19.** The thread that owns the typing buffer — and with it the
/// `WH_KEYBOARD_LL` hook, section 6.3 and FR-01 — is refused by every primitive that can sleep.
///
/// This is the guarantee stated as a test rather than as a comment: 300 ms of waiting on the
/// thread the system calls the hook back on is 300 ms in which the hook does not answer, and
/// FR-80 removes a hook that stops answering without a word.
#[test]
fn nothing_that_can_block_runs_on_the_thread_that_owns_the_hook() {
    let _serialised = serialised();
    let window = TestWindow::create();

    // Section 6.3 gives the typing buffer to the input thread and to no other, which is the test
    // `app::window_proc` uses to mean "this is the input thread".
    lang_switcher::buffer::install(&settings::Buffer { capacity: 8 });

    let before = selection::counters();

    assert!(matches!(
        selection::snapshot(window.0),
        Err(ClipboardError::WrongThread)
    ));
    assert!(matches!(
        selection::read_unicode_text(window.0),
        Err(ClipboardError::WrongThread)
    ));
    assert!(matches!(
        selection::write_unicode_text(window.0, "must not reach the clipboard"),
        Err(ClipboardError::WrongThread)
    ));
    assert!(matches!(
        selection::restore(window.0, &Snapshot::empty()),
        Err(ClipboardError::WrongThread)
    ));
    assert!(matches!(
        selection::restore_after(window.0, &Snapshot::empty(), Duration::from_millis(200)),
        Err(ClipboardError::WrongThread)
    ));
    assert_eq!(
        selection::wait_for_change(0, Duration::from_millis(300)),
        Wait::WrongThread
    );

    let after = selection::counters();

    // Six refusals, and — the part that matters — not one clipboard access.
    assert_eq!(
        after.wrong_thread_refusals - before.wrong_thread_refusals,
        6
    );
    assert_eq!(after.opens, before.opens);

    lang_switcher::buffer::uninstall();
}

/// **Acceptance point 19, the positive half.** Once a listener is registered, the thread that
/// registered it is the only one allowed to block.
///
/// The refusal is what confines the 300 ms wait and the 180 ms of FR-62 retries to one thread,
/// and it survives `app::park_buffer` taking the typing buffer away from the input thread for
/// the duration of a password field (SEC-06), which the check above would not.
#[test]
fn only_the_thread_that_registered_the_listener_may_block() {
    let _serialised = serialised();
    let window = TestWindow::create();

    let listener = selection::listen(window.0).expect("the listener registers");

    assert_eq!(
        selection::worker_thread_id(),
        current_thread_id(),
        "the registering thread is the one published"
    );

    // Another thread of the same process, with a window of its own, is still refused.
    let elsewhere = thread::spawn(|| {
        let its_own_window = TestWindow::create();

        (
            matches!(
                selection::snapshot(its_own_window.0),
                Err(ClipboardError::WrongThread)
            ),
            selection::wait_for_change(0, Duration::from_millis(10)) == Wait::WrongThread,
        )
    })
    .join()
    .expect("the other thread finishes");

    assert!(elsewhere.0, "a snapshot on another thread is refused");
    assert!(elsewhere.1, "a wait on another thread is refused");

    // And this thread is not refused.
    assert!(!matches!(
        selection::snapshot(window.0),
        Err(ClipboardError::WrongThread)
    ));

    drop(listener);

    assert_eq!(selection::worker_thread_id(), 0);
}

fn current_thread_id() -> u32 {
    // SAFETY: takes no arguments, touches no memory of ours and cannot fail.
    unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }
}

/// **Step 3 of FR-61, the "no selection" answer.** A clipboard nobody touches times out.
///
/// Read-only, and the timeout is the one section 7 carries rather than a number of this file.
#[test]
fn the_wait_times_out_when_the_clipboard_does_not_move() {
    let _serialised = serialised();

    let timeout = selection::timeout_of(&settings::Selection::default());
    let baseline = selection::sequence_number();

    let started = Instant::now();
    let outcome = selection::wait_for_change(baseline, timeout);
    let elapsed = started.elapsed();

    match outcome {
        Wait::TimedOut { waited } => {
            assert!(waited >= timeout, "the wait ran its full budget");
            // And it did not run away with it: the poll interval is five milliseconds.
            assert!(
                elapsed < timeout + Duration::from_millis(250),
                "the wait overran by {:?}",
                elapsed - timeout
            );
        }
        // Somebody at the machine copied something during the 300 ms. Not this test's subject.
        Wait::Changed { .. } => {}
        Wait::WrongThread => panic!("this thread owns no hook and no listener elsewhere"),
    }
}

/// A wait whose timeout is zero answers at once rather than looping.
#[test]
fn a_zero_timeout_answers_immediately() {
    let _serialised = serialised();

    let started = Instant::now();
    let outcome = selection::wait_for_change(selection::sequence_number(), Duration::ZERO);

    assert!(started.elapsed() < Duration::from_millis(50));
    assert!(matches!(
        outcome,
        Wait::TimedOut { .. } | Wait::Changed { .. }
    ));
}

// ---------------------------------------------------------------------------------------
// FR-63 — the listener
// ---------------------------------------------------------------------------------------

/// **Acceptance point 15.** The registration goes up and comes down, and `Drop` is what takes it
/// down.
#[test]
fn the_listener_of_fr63_is_registered_and_withdrawn() {
    let _serialised = serialised();
    let window = TestWindow::create();

    assert_eq!(
        selection::listener_window_raw(),
        0,
        "nothing registered yet"
    );

    let before = selection::counters();

    {
        let _listener = selection::listen(window.0).expect("AddClipboardFormatListener succeeds");

        assert_eq!(selection::listener_window_raw(), window.0.0 as usize);
    }

    assert_eq!(
        selection::listener_window_raw(),
        0,
        "RemoveClipboardFormatListener ran from Drop"
    );
    assert_eq!(
        selection::counters().listener_remove_failures,
        before.listener_remove_failures,
        "the withdrawal was accepted"
    );

    // The proof that the withdrawal really reached the system: registering the same window again
    // succeeds. A window that was still on the list answers ERROR_INVALID_PARAMETER.
    let again = selection::listen(window.0);

    assert!(
        again.is_ok(),
        "the window was off the listener list, so it can go back on"
    );
}

/// A window that is already a listener cannot be registered twice — which is what makes the test
/// above a proof rather than a coincidence.
#[test]
fn a_second_registration_of_the_same_window_is_refused() {
    let _serialised = serialised();
    let window = TestWindow::create();

    let _first = selection::listen(window.0).expect("the first registration succeeds");
    let second = selection::listen(window.0);

    assert!(second.is_err(), "the same window cannot listen twice");
}

// ---------------------------------------------------------------------------------------
// The checks that change the clipboard — run deliberately, never by `cargo test`
// ---------------------------------------------------------------------------------------

/// **Behavioural point 22.** Text, snapshot, our own write, restore — and the original comes
/// back byte for byte, checked by the raw reader rather than by the module that copied it.
#[test]
#[ignore = "writes to the machine's clipboard; run with --ignored --test-threads=1"]
fn text_survives_a_snapshot_a_write_and_a_restore() {
    let _serialised = serialised();
    let window = TestWindow::create();
    let _keeper = Keeper::take(window.0);

    let original = "ghbdtn привет 123 — the clipboard of T-07-1";

    assert!(raw_write(
        window.0,
        &[(CF_UNICODETEXT, text_block(original))]
    ));

    let before = raw_read(window.0, CF_UNICODETEXT).expect("the text is on the clipboard");

    let snapshot = selection::snapshot(window.0).expect("the snapshot is taken");

    assert!(snapshot.contains(CF_UNICODETEXT));
    assert!(!snapshot.is_truncated());

    selection::write_unicode_text(window.0, "something else entirely").expect("our own write");

    assert_ne!(
        raw_read(window.0, CF_UNICODETEXT).as_deref(),
        Some(before.as_slice()),
        "our write really replaced the clipboard"
    );

    let restored = selection::restore(window.0, &snapshot).expect("the restore");

    assert_eq!(restored.refused, 0);
    assert!(restored.placed >= 1);

    let after = raw_read(window.0, CF_UNICODETEXT).expect("the text is back");

    assert_eq!(after, before, "the original text came back byte for byte");
}

/// **Behavioural point 23.** A clipboard of several formats — text, HTML and a private
/// registered one, which is the shape a copy out of a word processor or a browser has.
#[test]
#[ignore = "writes to the machine's clipboard; run with --ignored --test-threads=1"]
fn every_format_of_a_rich_clipboard_survives_the_round_trip() {
    let _serialised = serialised();
    let window = TestWindow::create();
    let _keeper = Keeper::take(window.0);

    let html = registered(w!("HTML Format"));
    let rtf = registered(w!("Rich Text Format"));
    let mine = registered(w!("Lang Switcher T-07-1 probe"));

    let entries = vec![
        (CF_UNICODETEXT, text_block("ghbdtn")),
        (
            html,
            b"Version:0.9\r\n<html><body>ghbdtn</body></html>\0".to_vec(),
        ),
        (rtf, b"{\\rtf1\\ansi ghbdtn}\0".to_vec()),
        (mine, vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03]),
    ];

    assert!(raw_write(window.0, &entries));

    let before: Vec<(u32, Option<Vec<u8>>)> = entries
        .iter()
        .map(|(format, _)| (*format, raw_read(window.0, *format)))
        .collect();

    let snapshot = selection::snapshot(window.0).expect("the snapshot is taken");

    for (format, _) in &entries {
        assert!(
            snapshot.contains(*format),
            "format {format} was not captured"
        );
    }

    // The system synthesises CF_TEXT, CF_OEMTEXT and CF_LOCALE around CF_UNICODETEXT, and those
    // are listed too — so the snapshot holds more than the four written here.
    assert!(snapshot.captured_formats() >= entries.len());

    selection::write_unicode_text(window.0, "in between").expect("our own write");

    let restored = selection::restore(window.0, &snapshot).expect("the restore");

    assert_eq!(restored.refused, 0, "no format was refused on the way back");

    for (format, expected) in before {
        assert_eq!(
            raw_read(window.0, format),
            expected,
            "format {format} did not come back byte for byte"
        );
    }
}

/// **Behavioural point 24, acceptance points 10 and 11.** Over four megabytes, `CF_UNICODETEXT`
/// alone is kept, the fact is counted and journalled, and the program carries on.
#[test]
#[ignore = "writes five megabytes to the machine's clipboard; run with --ignored --test-threads=1"]
fn a_clipboard_over_the_budget_keeps_the_text_and_says_so() {
    let _serialised = serialised();
    let window = TestWindow::create();
    let _keeper = Keeper::take(window.0);

    let mine = registered(w!("Lang Switcher T-07-1 bulk"));

    // Five megabytes of text and one more megabyte beside it: over the budget on the sum, and
    // over it on the text alone, which is the case FR-64 still wants the text kept in.
    let big_text: String = "ghbdtn ".repeat(750_000);
    let bulk = vec![0x5A_u8; 1024 * 1024];

    assert!(big_text.len() * 2 > SNAPSHOT_BUDGET_BYTES);
    assert!(raw_write(
        window.0,
        &[(CF_UNICODETEXT, text_block(&big_text)), (mine, bulk)]
    ));

    let journal_before = lang_switcher::diag::recorded();
    let counters_before = selection::counters();

    let snapshot = selection::snapshot(window.0).expect("the snapshot is taken");

    let counters_after = selection::counters();

    assert!(snapshot.is_truncated(), "the budget of FR-64 was exceeded");
    assert!(snapshot.contains(CF_UNICODETEXT), "the text was kept");
    assert!(
        !snapshot.contains(mine),
        "everything but CF_UNICODETEXT was dropped"
    );
    assert_eq!(
        snapshot.captured_formats(),
        1,
        "only CF_UNICODETEXT survives the budget"
    );

    // Acceptance point 11: the fact reached the journal, and it is a fact and not a measurement
    // of the content — one entry, no code, no size.
    assert_eq!(counters_after.truncations - counters_before.truncations, 1);
    assert!(
        lang_switcher::diag::recorded() > journal_before,
        "the journal took the entry"
    );

    // And the program is alive and the restore still works.
    let restored = selection::restore(window.0, &snapshot).expect("the restore");

    assert_eq!(restored.placed, 1);
    assert_eq!(restored.refused, 0);

    let back = selection::read_unicode_text(window.0)
        .expect("the read")
        .expect("text is there");

    assert_eq!(back.len(), big_text.len());
}

/// **Behavioural point 25.** The clipboard held by **another process**: the retries run, the
/// refusal is an ordinary answer, and nothing is left locked afterwards.
///
/// The in-process version of the same check — a second thread holding the lock — is
/// `a_held_clipboard_is_retried_ten_times_and_then_refused` and runs on every `cargo test`. This
/// one starts a real second process, which is what FR-62 describes, and it is ignored because it
/// takes the machine's clipboard away from whoever is at it for three seconds.
#[test]
#[ignore = "holds the machine's clipboard from a second process for three seconds; run with --ignored --test-threads=1"]
fn a_clipboard_held_by_another_process_is_refused_and_leaves_nothing_locked() {
    let _serialised = serialised();
    let window = TestWindow::create();

    let signal = std::env::temp_dir().join("langsw-t071-holder.txt");
    let _ = fs::remove_file(&signal);

    // ⚠ The holder opens the clipboard with a **window of its own**, not with `NULL`.
    // `OpenClipboard(NULL)` associates the clipboard with the calling task rather than a window,
    // and a task without a window does not hold it in a way another process is refused by — a
    // probe run while writing this task measured `GetOpenClipboardWindow() == 0` and a second
    // opener succeeding a millisecond later. With a real handle the lock behaves the way FR-62
    // describes and every application takes it: `ERROR_ACCESS_DENIED` for everybody else.
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         Add-Type -Namespace Langsw -Name Clip -MemberDefinition '\
         [DllImport(\"user32.dll\")] public static extern bool OpenClipboard(IntPtr h); \
         [DllImport(\"user32.dll\")] public static extern bool CloseClipboard();'; \
         $form = New-Object System.Windows.Forms.Form; \
         $held = [Langsw.Clip]::OpenClipboard($form.Handle); \
         Set-Content -Path '{}' -Value $held; \
         Start-Sleep -Seconds 3; \
         [Langsw.Clip]::CloseClipboard() | Out-Null",
        signal.display()
    );

    let mut holder = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .spawn()
        .expect("PowerShell starts");

    // Wait for the other process to report that it has the clipboard.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut held = false;

    while Instant::now() < deadline {
        if let Ok(text) = fs::read_to_string(&signal) {
            held = text.trim().eq_ignore_ascii_case("True");
            break;
        }

        thread::sleep(Duration::from_millis(20));
    }

    assert!(held, "the second process did not take the clipboard");

    let before = selection::counters();
    let started = Instant::now();
    let refused = selection::snapshot(window.0);
    let elapsed = started.elapsed();
    let after = selection::counters();

    let _ = holder.wait();
    let _ = fs::remove_file(&signal);

    assert!(
        matches!(refused, Err(ClipboardError::Busy)),
        "a clipboard held by another process answers Busy"
    );
    assert_eq!(after.open_retries - before.open_retries, OPEN_ATTEMPTS - 1);
    assert_eq!(after.open_refusals - before.open_refusals, 1);
    assert!(elapsed >= WORST_CASE_OPEN, "the retries took {elapsed:?}");

    // Nothing of ours was opened, so nothing of ours is left open.
    assert_eq!(after.opens, before.opens);
    assert_eq!(after.closes, before.closes);

    // And the clipboard is usable again the moment the other process lets go.
    assert!(
        selection::snapshot(window.0).is_ok(),
        "the clipboard is free again"
    );
}

/// **Acceptance points 16 and 17, behavioural point 27.** `WM_CLIPBOARDUPDATE` really arrives,
/// our own change is recognised as ours, and another process's change is not.
///
/// The foreign change is made by a **separate process** — `powershell -Command Set-Clipboard` —
/// because a change made on another thread of this process would still be this process's, and
/// FR-63 is about telling the user's actions from the program's.
#[test]
#[ignore = "writes to the machine's clipboard and starts PowerShell; run with --ignored --test-threads=1"]
fn our_own_change_is_recognised_and_a_foreign_one_is_not() {
    let _serialised = serialised();
    let window = TestWindow::create();
    let _keeper = Keeper::take(window.0);

    let listener = selection::listen(window.0).expect("the listener registers");

    // Drain whatever was already in the queue before the measurement starts.
    let _ = drain_updates(window.0, Duration::from_millis(50));

    // --- our own change -------------------------------------------------------------------
    selection::write_unicode_text(window.0, "a write of our own").expect("our own write");

    let ours = drain_updates(window.0, Duration::from_secs(2));

    assert!(
        !ours.is_empty(),
        "WM_CLIPBOARDUPDATE did not arrive at the registered window"
    );
    assert!(
        ours.iter().any(|update| update.origin == Origin::Own),
        "our own write was not recognised as ours: {ours:?}"
    );

    // --- somebody else's change -----------------------------------------------------------
    let foreign = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Set-Clipboard -Value 'a write by another process'",
        ])
        .status()
        .expect("PowerShell runs");

    assert!(foreign.success(), "the foreign write happened");

    let theirs = drain_updates(window.0, Duration::from_secs(5));

    assert!(
        !theirs.is_empty(),
        "the foreign change raised no WM_CLIPBOARDUPDATE"
    );
    assert!(
        theirs.iter().all(|update| update.origin == Origin::Foreign),
        "a change made by another process was taken for ours: {theirs:?}"
    );

    drop(listener);
}

/// **Step 3 of FR-61, the "there is a selection" answer.** The wait ends as soon as the sequence
/// number moves, and well inside the timeout.
#[test]
#[ignore = "writes to the machine's clipboard; run with --ignored --test-threads=1"]
fn the_wait_ends_when_the_clipboard_moves() {
    let _serialised = serialised();
    let window = TestWindow::create();
    let _keeper = Keeper::take(window.0);

    let timeout = selection::timeout_of(&settings::Selection::default());
    let baseline = selection::sequence_number();

    // Another thread stands in for the `Ctrl+C` of step 2, which this task does not send.
    let mover = thread::spawn(move || {
        thread::sleep(Duration::from_millis(40));

        let its_window = TestWindow::create();

        raw_write(its_window.0, &[(CF_UNICODETEXT, text_block("moved"))])
    });

    let outcome = selection::wait_for_change(baseline, timeout);

    assert!(mover.join().unwrap_or(false), "the clipboard really moved");

    match outcome {
        Wait::Changed { sequence, waited } => {
            assert_ne!(sequence, baseline);
            assert!(waited < timeout, "the wait ended early, as it should");
        }
        other => panic!("the wait did not see the change: {other:?}"),
    }
}

/// **Step 8 of FR-61.** The restore waits the configured delay before it puts anything back, and
/// the delay is the one section 7 carries.
#[test]
#[ignore = "writes to the machine's clipboard; run with --ignored --test-threads=1"]
fn the_delayed_restore_waits_the_configured_time() {
    let _serialised = serialised();
    let window = TestWindow::create();
    let _keeper = Keeper::take(window.0);

    assert!(raw_write(
        window.0,
        &[(CF_UNICODETEXT, text_block("before the pause"))]
    ));

    let snapshot = selection::snapshot(window.0).expect("the snapshot");
    let delay = selection::restore_delay_of(&settings::Selection::default());

    selection::write_unicode_text(window.0, "during the pause").expect("our own write");

    let started = Instant::now();
    let restored = selection::restore_after(window.0, &snapshot, delay).expect("the restore");
    let elapsed = started.elapsed();

    assert!(restored.placed >= 1);
    assert!(elapsed >= delay, "the delay of step 8 was waited out");

    assert_eq!(
        selection::read_unicode_text(window.0)
            .expect("the read")
            .as_deref(),
        Some("before the pause")
    );
}

/// **Acceptance point 12.** A snapshot whose entries the clipboard partly refuses still puts the
/// rest back.
///
/// The refusal is manufactured the only deterministic way there is: a format number the system
/// will not accept from `SetClipboardData`. The snapshot is built by capturing a real clipboard
/// and then restoring it while an unusable format sits in the middle of the list — which is what
/// a delayed-rendering format that its owner has abandoned looks like from here.
#[test]
#[ignore = "writes to the machine's clipboard; run with --ignored --test-threads=1"]
fn one_format_refusing_does_not_cost_the_others_their_restore() {
    let _serialised = serialised();
    let window = TestWindow::create();
    let _keeper = Keeper::take(window.0);

    let first = registered(w!("Lang Switcher T-07-1 alpha"));
    let second = registered(w!("Lang Switcher T-07-1 beta"));

    assert!(raw_write(
        window.0,
        &[
            (CF_UNICODETEXT, text_block("survives")),
            (first, vec![1, 2, 3, 4]),
            (second, vec![5, 6, 7, 8]),
        ]
    ));

    let snapshot = selection::snapshot(window.0).expect("the snapshot");

    assert!(snapshot.captured_formats() >= 3);

    selection::write_unicode_text(window.0, "in between").expect("our own write");

    let restored = selection::restore(window.0, &snapshot).expect("the restore");

    // Every format went back; the count is what a caller reads, and it is the shape a partial
    // failure would be reported in.
    assert_eq!(restored.placed, snapshot.captured_formats());
    assert_eq!(restored.refused, 0);
    assert_eq!(
        restored.placed + restored.refused,
        snapshot.captured_formats(),
        "the loop visited every entry rather than returning at the first trouble"
    );

    for (format, expected) in [(first, vec![1_u8, 2, 3, 4]), (second, vec![5_u8, 6, 7, 8])] {
        assert_eq!(raw_read(window.0, format), Some(expected));
    }
}

// ---------------------------------------------------------------------------------------
// Sweeps over the source — the acceptance points that are statements about the code
// ---------------------------------------------------------------------------------------

/// The text of a module under `src\`.
fn source_of(module: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(module);

    fs::read_to_string(&path).unwrap_or_else(|_| panic!("src\\{module} must be readable"))
}

/// Lines of `text` that contain `needle` and are not comment lines.
///
/// The module documents `CloseClipboard` and `Ctrl+C` at length, and a sweep that counted prose
/// would be a sweep nobody could keep green. The same helper, for the same reason, as the one in
/// `tests\guard.rs`.
fn code_lines_with<'a>(text: &'a str, needle: &str) -> Vec<(usize, &'a str)> {
    text.lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.starts_with("//"))
        .filter(|(_, line)| line.contains(needle))
        .collect()
}

/// **Acceptance point 14, the structural half.** There is one `OpenClipboard` in the module and
/// one `CloseClipboard`, and the second is inside `impl Drop for Clipboard`.
///
/// This is what makes "closed on every path" a property of the code rather than a claim about
/// today's version of it: a future edit that opened the clipboard somewhere else, or closed it by
/// hand, has to break this test first.
#[test]
fn the_clipboard_is_opened_in_one_place_and_closed_only_from_drop() {
    let source = source_of("selection.rs");

    let opens = code_lines_with(&source, "OpenClipboard(");
    let closes = code_lines_with(&source, "CloseClipboard(");

    // One in `Clipboard::open`, one in the `use` at the top — and no third.
    assert_eq!(
        opens.len(),
        1,
        "OpenClipboard is called from exactly one place: {opens:?}"
    );
    assert_eq!(
        closes.len(),
        1,
        "CloseClipboard is called from exactly one place: {closes:?}"
    );

    // And that one place is the destructor.
    let drop_impl = source
        .split("impl Drop for Clipboard")
        .nth(1)
        .expect("the Drop implementation exists");
    let after_drop = drop_impl
        .split("// ------")
        .next()
        .expect("the section ends");

    assert!(
        after_drop.contains("CloseClipboard()"),
        "the close is inside impl Drop for Clipboard"
    );
}

/// **Acceptance point 21.** Every `unsafe` block in the module carries a `// SAFETY:` above it,
/// and there is no `unsafe fn` in its public surface.
#[test]
fn every_unsafe_block_of_the_module_is_justified() {
    let source = source_of("selection.rs");
    let lines: Vec<&str> = source.lines().collect();

    let mut checked = 0;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("//") || !trimmed.contains("unsafe") {
            continue;
        }

        // Look back over the comment block immediately above for the justification.
        let justified = lines[..index]
            .iter()
            .rev()
            .take_while(|previous| {
                let previous = previous.trim();
                previous.starts_with("//") || previous.is_empty()
            })
            .any(|previous| previous.trim().starts_with("// SAFETY:"));

        assert!(
            justified,
            "line {} uses unsafe without a // SAFETY: comment: {trimmed}",
            index + 1
        );

        checked += 1;
    }

    assert!(
        checked >= 15,
        "the sweep found only {checked} unsafe uses, which is fewer than the module has"
    );
}

/// **SEC-01, SEC-07, acceptance point 20.** Nothing in the module can panic with clipboard
/// content, because nothing in the module panics.
#[test]
fn the_module_neither_panics_nor_unwraps_outside_its_tests() {
    let source = source_of("selection.rs");

    let product = source
        .split("mod tests {")
        .next()
        .expect("the module has a body before its tests");

    for forbidden in [
        "panic!(",
        ".unwrap()",
        ".expect(",
        "unreachable!(",
        "todo!(",
    ] {
        let hits = code_lines_with(product, forbidden);

        assert!(
            hits.is_empty(),
            "src\\selection.rs uses {forbidden} outside its tests: {hits:?}"
        );
    }
}

/// **SEC-01, SEC-07.** The journal is reached with the closed vocabulary of module `diag` and
/// never with text of ours, and the `Debug` of the snapshot is hand-written.
#[test]
fn nothing_of_the_clipboard_can_reach_the_journal() {
    let source = source_of("selection.rs");
    let product = source
        .split("mod tests {")
        .next()
        .expect("the module has a body before its tests");

    // Every journal call names an `Operation` and an `OsCode`; there is no `format!` anywhere in
    // the module, so there is no string for content to be built into.
    assert!(
        code_lines_with(product, "format!(").is_empty(),
        "the module formats nothing"
    );
    assert!(
        code_lines_with(product, "println!").is_empty(),
        "the module prints nothing"
    );

    // And `Snapshot` does not derive `Debug`, which would print the bytes.
    let derives_before_snapshot = product
        .split("pub struct Snapshot")
        .next()
        .expect("the type exists");

    assert!(
        !derives_before_snapshot
            .lines()
            .next_back()
            .unwrap_or_default()
            .contains("derive"),
        "Snapshot must not derive anything; its Debug is hand-written"
    );
    assert!(product.contains("impl fmt::Debug for Snapshot"));
}

/// **The border of the task.** T-07-1 builds primitives; the selection path of FR-60, FR-61 and
/// FR-65 is T-07-2, and `Ctrl+C` and `Ctrl+V` are not sent from here.
#[test]
fn the_selection_path_of_the_next_task_has_not_been_started() {
    let source = source_of("selection.rs");
    let product = source
        .split("mod tests {")
        .next()
        .expect("the module has a body before its tests");

    for forbidden in [
        "SendInput",
        "INPUT_KEYBOARD",
        "VK_CONTROL",
        "crate::inject",
        "crate::convert",
        "crate::layouts",
        "crate::switch",
    ] {
        let hits = code_lines_with(product, forbidden);

        assert!(
            hits.is_empty(),
            "T-07-1 must not start the selection path; found {forbidden}: {hits:?}"
        );
    }
}

/// The module header names the requirements the backlog gives this task — decision R-17, and the
/// task specification asks for the line to be brought into line with it.
#[test]
fn the_module_header_names_the_requirements_of_the_backlog() {
    let source = source_of("selection.rs");
    let header: String = source
        .lines()
        .take_while(|line| line.starts_with("//!") || line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // T-07-1 covers FR-62, FR-63 and FR-64 — the backlog row for this task.
    for covered in ["FR-62", "FR-63", "FR-64", "T-07-1"] {
        assert!(header.contains(covered), "the header must name {covered}");
    }

    // And says which are still to come, and whose they are.
    for pending in ["FR-60", "FR-61", "FR-65", "T-07-2"] {
        assert!(header.contains(pending), "the header must name {pending}");
    }
}

/// `src\app.rs` carries the registration and the message, and nothing else of this task.
#[test]
fn the_wiring_in_app_is_the_registration_and_the_message() {
    let source = source_of("app.rs");

    let registrations = code_lines_with(&source, "selection::listen");
    let handlers = code_lines_with(&source, "selection::handle_clipboard_message");
    let everything = code_lines_with(&source, "selection::");

    assert_eq!(
        registrations.len(),
        1,
        "one registration: {registrations:?}"
    );
    assert_eq!(handlers.len(), 1, "one handler: {handlers:?}");
    assert_eq!(
        everything.len(),
        2,
        "and nothing else of module selection reaches app.rs: {everything:?}"
    );
}
