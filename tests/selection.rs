//! Integration checks of module `selection` — the clipboard primitives of FR-62, FR-63 and
//! FR-64, and the selection path of FR-60, FR-61 and FR-65 built on top of them.
//!
//! Tasks **T-07-1** (the primitives) and **T-07-2** (the path).
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
//! # The selection path — task T-07-2
//!
//! The eight steps of FR-61 are an **order**, and an order is not observable from outside the
//! function that performs it. Module `selection` therefore puts the outside world behind a trait
//! — `selection::Path`, the same device `inject::Environment` is for FR-40 — and the second half
//! of this file drives the eight steps against a [`Bench`] that records every call, can be told
//! to fail at any step and can be told to panic at any step. **No clipboard, no keyboard and no
//! foreground window are involved in any of it**, which is what lets the checks that matter most
//! — that step 8 runs when steps 4 to 7 did not — run on every `cargo test`.
//!
//! Step 5 is a pure function of its arguments and is driven directly, against the hardwired
//! RU/EN maps of FR-25 (`convert::fallback_map`), so that what is asserted about scripts,
//! unmapped characters and reversibility is asserted about real layouts rather than about
//! whatever happens to be installed.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, path::Path};

use lang_switcher::convert;
use lang_switcher::diag::{Kind, Operation};
use lang_switcher::inject::{self, Dispatched, Modifiers};
use lang_switcher::layouts::{Cycle, KeyMapping, LayoutId, LayoutMap, LayoutMapBuilder, Mods};
use lang_switcher::selection::{
    self, CF_UNICODETEXT, CHORD_EVENTS, ClipboardError, OPEN_ATTEMPTS, OPEN_RETRY_INTERVAL, Origin,
    Outcome, Path as SelectionPath, Plan, Refusal, SNAPSHOT_BUDGET_BYTES, Snapshot, Update,
    WORST_CASE_OPEN, Wait,
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

    let accounted =
        snapshot.captured_formats() + snapshot.handle_formats() + snapshot.refused_formats();

    // ⚠ **The identity holds below the ceiling of FR-64 and not above it — task Т-48-6.**
    //
    // Above it the requirement itself says what happens: «при превышении лимита сохраняется
    // только `CF_UNICODETEXT`». Every other memory format is then deliberately skipped, and a
    // skipped format is not captured, is not a handle format and was not refused — it is the
    // fourth thing FR-64 asks for, and the sum is short by exactly the number of them.
    //
    // This assertion used to be written without the truncated case and had simply never met a
    // clipboard over four megabytes. On 2026-09-09 it met one — on a **pristine** tree, before
    // any change of stage Э48 — and read `1` against `8`, which looked like a defect of the
    // product and was a defect of the premise. The user's word: «Починить сейчас, до закрытия
    // Э48». What the truncated case can still say is the inequality, and it is the sharp half:
    // nothing may be *invented*, and `CF_UNICODETEXT` is what has to survive.
    if snapshot.is_truncated() {
        assert!(
            accounted <= snapshot.listed_formats(),
            "a truncated snapshot accounts for no more than the clipboard listed: \
             {accounted} of {}",
            snapshot.listed_formats()
        );
        assert!(
            snapshot.formats().all(|format| format == CF_UNICODETEXT),
            "FR-64: above the ceiling only CF_UNICODETEXT is kept"
        );
    } else {
        assert_eq!(
            accounted,
            snapshot.listed_formats(),
            "every listed format is either captured or explained"
        );
    }

    // Nothing captured is a handle format, and every captured one is memory-backed. True in
    // both cases: truncation drops formats, it never adds one.
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
        if snapshot.is_truncated() {
            // ⚠ **What the second arm of this assertion used to say could never be true** —
            // task Т-48-6. It read `total_bytes() > SNAPSHOT_BUDGET_BYTES`, and `total_bytes`
            // counts the bytes that were **captured**: above the ceiling that is the one text
            // block FR-64 keeps, which is small by construction. So the arm was unreachable and
            // the whole line was «a truncated snapshot fails», which is what a clipboard over
            // four megabytes on this machine made it do on 2026-09-09.
            //
            // The quantity the old wording wanted — the sum *before* the decision — is not
            // published by `Snapshot` at all, so a test cannot read it; that half of FR-64 rests
            // on `snapshot`'s own arithmetic. What is observable is that the truncation did its
            // work, and that is asserted here.
            assert!(
                snapshot.total_bytes() <= SNAPSHOT_BUDGET_BYTES,
                "a truncation exists to bring the snapshot under the ceiling, and this one \
                 kept {} bytes",
                snapshot.total_bytes()
            );
            assert!(
                snapshot.captured_formats() <= 1,
                "FR-64: above the ceiling at most CF_UNICODETEXT is kept, and {} formats were",
                snapshot.captured_formats()
            );
        } else {
            assert!(snapshot.total_bytes() <= SNAPSHOT_BUDGET_BYTES);
        }
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
        selection::restore_after(window.0, &Snapshot::empty(), Duration::from_millis(200), 0),
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

    // ⚠ Read back with the **raw** oracle and not with the module that wrote it — the doctrine
    // of every other check in this file, and here it is also a necessity: since the ceiling of
    // step 4 (finding №7 of the audit of 2026-08-31, decision 77) `read_unicode_text` refuses a
    // block this big by design, so it can no longer serve as the witness that the restore worked.
    let back = raw_read(window.0, CF_UNICODETEXT).expect("the text is back on the clipboard");

    assert_eq!(
        back.len(),
        (big_text.len() + 1) * 2,
        "the whole of the text came back, terminator included"
    );

    // And the ceiling itself, on a real clipboard of a real size — the honest live half of
    // `the_ceiling_of_step_four_is_the_four_megabytes_of_fr64_and_not_a_second_number`. Over the
    // budget the read behaves exactly as «there is no text»: `Ok(None)`, and not an error, so
    // step 4 lands on `Refusal::NoText` and the snapshot goes back by the path of Э18.
    assert!(
        big_text.len() * 2 > SNAPSHOT_BUDGET_BYTES,
        "the block really is over the ceiling"
    );

    let journal_before_read = lang_switcher::diag::recorded();

    // ⚠ Asserted with `is_none` and not with `assert_eq!(…, None)`: the failure message of the
    // latter prints the value it found, and the value here is five megabytes of clipboard. The
    // doctrine of the module is that nothing of the clipboard reaches a log, and a test that
    // breaks it on its way to red breaks it all the same.
    assert!(
        selection::read_unicode_text(window.0)
            .expect("the read is not an error")
            .is_none(),
        "⚠ a selection over the ceiling reads as «there is no text» — and nothing was copied"
    );
    assert!(
        lang_switcher::diag::recorded() > journal_before_read,
        "the refusal left its row in the journal"
    );

    // The control that keeps the assertion above from passing for the wrong reason: the same
    // clipboard, a block comfortably under the ceiling, still reads.
    let small = "ghbdtn привет";

    assert!(raw_write(window.0, &[(CF_UNICODETEXT, text_block(small))]));
    assert_eq!(
        selection::read_unicode_text(window.0).expect("the read"),
        Some(small.to_owned()),
        "a block under the ceiling is read as it always was"
    );
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
    // The probe number is the "nothing was answered" value: this test is about the delay and
    // about the own-write path, which is the arm `is_own_change` decides. Task Т-18-1.
    let restored = selection::restore_after(window.0, &snapshot, delay, 0).expect("the restore");
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

/// **И-1 of Э18 — the failure paths put the clipboard back, on the machine's real clipboard.**
///
/// The one test in this file that reproduces the whole of the defect the audit of 2026-08-31
/// found, and it needs no window and no keystroke: the three things that decide step 8 are a
/// snapshot, a clipboard the *application* changed in answer to the probe, and the sequence
/// number of that change.
///
/// `raw_write` is what makes it honest. It does not go through the module under test, so nothing
/// marks the change it makes as this program's own — which is exactly what an application
/// answering `Ctrl+C` looks like from here. Before this task, the gate of decision П-5 asked
/// [`selection::is_own_change`] alone, found no mark, and threw the user's clipboard away while
/// journalling it as a lawful П-5 skip.
#[test]
#[ignore = "writes to the machine's clipboard; run with --ignored --test-threads=1"]
fn the_snapshot_goes_back_when_the_path_refused_before_it_ever_wrote() {
    let _serialised = serialised();
    let window = TestWindow::create();
    let _keeper = Keeper::take(window.0);

    // What the user had. Step 1 of FR-61 takes it.
    assert!(raw_write(
        window.0,
        &[(CF_UNICODETEXT, text_block("метка-до — Э18"))]
    ));

    let before = raw_read(window.0, CF_UNICODETEXT).expect("the mark is on the clipboard");
    let snapshot = selection::snapshot(window.0).expect("the snapshot of step 1");

    // Steps 2 and 3: the probe goes out and the application answers it. Not our write — the
    // ring of FR-63 is silent for it, and that silence is the whole defect.
    assert!(raw_write(
        window.0,
        &[(CF_UNICODETEXT, text_block("C:\\selected\\picture.png"))]
    ));

    let probe = selection::sequence_number();

    assert!(
        !selection::is_own_change(probe),
        "the answer to the probe is not a write of ours, and nothing here pretends it is"
    );

    // Steps 4 to 7 refuse — no text, no direction, no write. Step 8 runs through the one door,
    // carrying the number step 3 saw. That number is the whole of the repair.
    let restored = selection::restore_after(window.0, &snapshot, Duration::ZERO, probe)
        .expect("step 8 answers");

    assert_eq!(
        raw_read(window.0, CF_UNICODETEXT),
        Some(before),
        "the user's clipboard came back"
    );
    assert!(
        restored.placed >= 1,
        "and it came back through the restore rather than by accident: {restored:?}"
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

/// A source file of the tree, read with the line endings normalised — task **T-13-30**.
///
/// **The one place this file reads a source file**, and it collapses `\r\n` to `\n` before
/// anybody downstream sees the text. `.gitattributes` declares `* text=auto eol=crlf`, so **the
/// canonical checkout of this repository is CRLF**, while `cargo fmt` writes LF: one commit is
/// one text after a checkout and another after a format. Any sweep whose needle carries a newline
/// answers differently for the two, and the answer it gives on the canonical tree is the wrong
/// one — [`body_after`] below was taking the whole rest of `src\selection.rs` for a function body,
/// 96 939 bytes of it where the body of `decode_utf16` is 310.
///
/// Every reader in this file goes through here, [`source_of`] and the directory sweep of
/// `the_console_classes_and_the_class_read_are_each_written_once` alike: a second `read_to_string`
/// beside it would make the normalisation a property of one call site instead of a property of
/// the file.
///
/// The same reading, and for the same reason, as `read_normalised` in `tests\guard.rs` after task
/// T-13-12. Not reinvented here.
fn read_normalised(path: &Path) -> String {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));

    text.replace("\r\n", "\n")
}

/// The text of a module under `src\`. Line endings normalised — see [`read_normalised`].
fn source_of(module: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(module);

    read_normalised(&path)
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

/// `remainder` up to the first `boundary` — the region a sweep is about, and nothing beyond it.
///
/// ⚠ **The one place this file ends a region, and there is no silent fall-back — task T-13-30.**
///
/// Every sweep in this file used to cut its region with `.split(boundary).next()` followed by
/// `.unwrap_or_default()`, `.unwrap_or(after)`, `.expect(…)` or `unwrap_or_else(|| panic!(…))`.
/// All four are dead code: `.next()` on a `Split` that has just been handed a string **never**
/// answers `None`, so what those lines actually encode is not a fall-back and not a check — it is
/// the sentence «if the boundary is lost, sweep on regardless», which is precisely what a lost
/// boundary must not do. Where the needle carries a newline the boundary really was lost, because
/// `.gitattributes` holds this tree in CRLF and `\r\n}\r\n` does not contain `"\n}\n"`: see
/// [`body_after`], which was reading 96 939 bytes of module where the function it names is 310.
///
/// Two things are refused here rather than trusted, so that a region cannot degenerate whichever
/// way a later edit loses its boundary:
///
/// * a boundary that cannot be found is a **failure**, with the needle printed;
/// * a region that is empty, or that is the whole remainder, is a sweep asserting nothing — and
///   a green sweep asserting nothing is worse than a red one, because nobody comes to look.
///
/// `what` names the region for the failure message. The same shape `tests\guard.rs` settled on in
/// task T-13-12, factored rather than repeated because this file cuts a region in seventeen
/// places and a rule that lives in one of them is not a rule.
fn cut_at<'a>(remainder: &'a str, boundary: &str, what: &str) -> &'a str {
    let end = remainder.find(boundary).unwrap_or_else(|| {
        panic!(
            "src\\selection.rs: {what} is not bounded by \"{}\" — everything asserted about it \
             below would be asserted about the rest of the module instead",
            boundary.escape_debug()
        )
    });

    let region = &remainder[..end];

    assert!(
        !region.is_empty(),
        "{what} is not empty: a boundary at the very start leaves the sweep with nothing to say"
    );
    assert!(
        region.len() < remainder.len(),
        "{what} was bounded ({} bytes) rather than taken as the rest of the module ({} bytes)",
        region.len(),
        remainder.len()
    );

    region
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
    let after_drop = cut_at(
        drop_impl,
        "// ------",
        "the body of impl Drop for Clipboard",
    );

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

    let product = cut_at(&source, "mod tests {", "the product half of the module");

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
    let product = cut_at(&source, "mod tests {", "the product half of the module");

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
    let derives_before_snapshot = cut_at(
        product,
        "pub struct Snapshot",
        "the text above the declaration of Snapshot",
    );

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

/// **The border of the task, as task T-07-2 leaves it.**
///
/// T-07-1 wrote this test to say the selection path had *not* been started: no `SendInput`, no
/// `VK_CONTROL`, no reach into `inject`, `convert`, `layouts` or `switch`. T-07-2 is the task
/// that starts it, so the same statement is made the other way round — the path is here, and it
/// is built out of the accepted modules rather than out of a second copy of them.
///
/// The list is the point. Every one of the four names below is a module this task is **forbidden
/// to modify**, and its presence here is what says the work went through the accepted engine:
/// the reverse index of step 5 is `layouts::LayoutMap::find_key`, the forward half is
/// `convert::convert_stroke` (FR-22), the two chords go out through `inject::dispatch` and the
/// layout switch of step 7 is `switch::to` (§4.6).
#[test]
fn the_selection_path_is_built_out_of_the_accepted_modules() {
    let source = source_of("selection.rs");
    let product = cut_at(&source, "mod tests {", "the product half of the module");

    for required in [
        "crate::inject",
        "crate::convert",
        "crate::layouts",
        "crate::switch",
    ] {
        assert!(
            !code_lines_with(product, required).is_empty(),
            "the selection path reaches {required} instead of re-implementing it"
        );
    }

    // And it re-implements neither the injection nor the conversion: `SendInput` is called by
    // module `inject` and by nobody else in this program, and there is no character table here.
    let sends = code_lines_with(product, "SendInput");
    assert!(
        sends.is_empty(),
        "SendInput belongs to module inject: {sends:?}"
    );
}

/// The module header names the requirements the backlog gives both tasks — decision R-17.
#[test]
fn the_module_header_names_the_requirements_of_the_backlog() {
    let source = source_of("selection.rs");
    let header: String = source
        .lines()
        .take_while(|line| line.starts_with("//!") || line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // T-07-1 covers FR-62, FR-63 and FR-64; T-07-2 covers FR-60, FR-61 and FR-65.
    for covered in [
        "FR-60", "FR-61", "FR-62", "FR-63", "FR-64", "FR-65", "T-07-1", "T-07-2",
    ] {
        assert!(header.contains(covered), "the header must name {covered}");
    }
}

/// **The border in `src\app.rs`** — four places, and the task names every one of them.
///
/// T-07-1 was allowed the registration and the message and this test counted two. Task T-07-2 is
/// allowed «ветвление источника данных на пути горячей клавиши И передача работы потоку UI», and
/// that is the other four: the branch of FR-60, the message it hands back, the far end of the
/// handoff and the publication of `[selection]` the branch reads.
///
/// ⚠ **Task T-13-13 widened the border by two lines, and this census names them rather than
/// loosening.** The two ceilings of the millisecond fields of `[selection]` are read in `app.rs`
/// because that is where the clamp of that task stands — on the one publication, not at the place
/// that waits — so the constants cross this border by design and are counted here like everything
/// else. The point of the census is unchanged: nothing of module `selection` reaches `app.rs`
/// without a line in this table.
#[test]
fn the_wiring_in_app_is_the_registration_the_messages_and_the_branch() {
    let source = source_of("app.rs");

    for (needle, expected) in [
        // T-07-1.
        ("selection::listen", 1),
        ("selection::handle_clipboard_message", 1),
        // T-07-2: FR-65 published to the thread that branches.
        ("selection::publish(", 1),
        // T-07-2: the branch of FR-60, and the message that carries the other half back.
        ("selection::wants_selection_path", 1),
        ("selection::WM_APP_BUFFER_PATH", 2),
        // T-07-2: the far end of the handoff.
        ("selection::handle_selection_message", 1),
        // T-13-13: the two ceilings, read where the publication clamps.
        ("selection::MAX_CLIPBOARD_TIMEOUT_MS", 1),
        ("selection::MAX_CLIPBOARD_RESTORE_DELAY_MS", 1),
    ] {
        let hits = code_lines_with(&source, needle);

        assert_eq!(
            hits.len(),
            expected,
            "{needle} appears {expected} times: {hits:?}"
        );
    }

    // And nothing else of module `selection` reaches `app.rs`: nine lines, all named above.
    let everything = code_lines_with(&source, "selection::");
    assert_eq!(
        everything.len(),
        9,
        "nothing else of module selection reaches app.rs: {everything:?}"
    );
}

// =========================================================================================
// The selection path — FR-60, FR-61, FR-65. Task T-07-2
// =========================================================================================

// ---------------------------------------------------------------------------------------
// A bench for the eight steps
// ---------------------------------------------------------------------------------------

/// One call through `selection::Path`, as [`Bench`] records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// Step 1 of FR-61.
    Snapshot,
    /// Step 3 of FR-40 — the user's modifiers come off.
    Release,
    /// Step 2 of FR-61 — `Ctrl+C`.
    Copy,
    /// Step 3 of FR-61 — the sequence number.
    Wait,
    /// Step 4 of FR-61 — read `CF_UNICODETEXT`.
    Read,
    /// Step 6 of FR-61, first half.
    Write,
    /// Step 6 of FR-61, second half — `Ctrl+V`.
    Paste,
    /// Step 7 of FR-61 — the layout switch.
    Switch,
    /// Step 6 of FR-40 — the user's modifiers go back.
    RestoreModifiers,
    /// **Step 8 of FR-61** — the user's clipboard goes back.
    RestoreClipboard,
    /// **Not a step of FR-61** — the user's clipboard goes back at once, because the `Ctrl+C` of
    /// step 2 was answered after step 3 had given up. Task T-13-11.
    Reclaim,
}

/// The sequence number the clipboard takes when the application answers the probe of step 2.
///
/// Any number other than the baseline of a bench, which is `Snapshot::empty().sequence()` — zero.
/// It was written as a bare `1` inside `Bench::with_selection` until task Т-18-1 made it a thing
/// the tests have to name: it is what step 8 must be handed, and what the gate of decision П-5
/// must accept as this pass's own doing.
const PROBE_ANSWER: u32 = 1;

/// The eight steps with the outside world replaced by a tape recorder.
struct Bench {
    /// Every call, in the order it was made. This *is* the assertion of acceptance point 9.
    steps: Vec<Step>,
    /// The step that is to answer with a failure, if any.
    fail_at: Option<Step>,
    /// The step that is to panic, if any — acceptance point 10 on the unwinding path.
    panic_at: Option<Step>,
    /// What step 3 answers.
    wait: Wait,
    /// What the sequence number reads *after* step 3 gave up — task T-13-11.
    ///
    /// The baseline of a bench is `Snapshot::empty().sequence()`, so the default answers "the
    /// clipboard did not move" and every test written before T-13-11 sees what it saw.
    sequence_now: u32,
    /// How many times the sequence number was asked for outside step 3. **Not a [`Step`]**: the
    /// tape records the steps of FR-61 and FR-40, and this is neither.
    sequence_probes: u32,
    /// The baseline step 3 was given — acceptance point 11.
    baseline_seen: Option<u32>,
    /// The probe number step 8 was handed through [`SelectionPath::restore_clipboard`].
    ///
    /// **Not a [`Step`]** for the same reason `sequence_probes` is not one, and the single most
    /// load-bearing field of this bench since task Т-18-1: the tape says step 8's door opened,
    /// and this says whether what came through it lets the gate of П-5 open too. Before Т-18-1
    /// the first was true on every failure path and the second was false, and the user's
    /// clipboard was the difference.
    restore_probe: Option<u32>,
    /// What step 4 answers.
    reads: Option<String>,
    /// What step 6 wrote.
    written: Option<String>,
    /// What step 7 was asked to switch to.
    switched: Option<LayoutId>,
}

impl Bench {
    /// A bench where every step succeeds and there **is** a selection.
    fn with_selection(text: &str) -> Self {
        Self {
            steps: Vec::new(),
            fail_at: None,
            panic_at: None,
            wait: Wait::Changed {
                sequence: PROBE_ANSWER,
                waited: Duration::from_millis(7),
            },
            sequence_now: Snapshot::empty().sequence(),
            sequence_probes: 0,
            baseline_seen: None,
            restore_probe: None,
            reads: Some(text.to_owned()),
            written: None,
            switched: None,
        }
    }

    /// A bench where step 3 times out — there is no selection.
    fn without_selection() -> Self {
        Self {
            wait: Wait::TimedOut {
                waited: Duration::from_millis(300),
            },
            reads: None,
            ..Self::with_selection("")
        }
    }

    /// A bench where step 3 times out **and the clipboard moves anyway** — the late `Ctrl+C` of
    /// the audit of 2026-08-24, task T-13-11.
    ///
    /// The number is any number other than the baseline: what the eight steps read out of it is
    /// «it is not what step 1 saw», and nothing else.
    fn moving_after_the_timeout() -> Self {
        Self {
            sequence_now: Snapshot::empty().sequence().wrapping_add(4242),
            ..Self::without_selection()
        }
    }

    fn failing_at(mut self, step: Step) -> Self {
        self.fail_at = Some(step);
        self
    }

    fn panicking_at(mut self, step: Step) -> Self {
        self.panic_at = Some(step);
        self
    }

    /// Records `step` and answers whether it is the one told to fail.
    fn note(&mut self, step: Step) -> bool {
        self.steps.push(step);

        if self.panic_at == Some(step) {
            panic!("the bench was told to panic at {step:?}");
        }

        self.fail_at == Some(step)
    }

    fn ran(&self, step: Step) -> bool {
        self.steps.contains(&step)
    }

    fn position_of(&self, step: Step) -> Option<usize> {
        self.steps.iter().position(|&seen| seen == step)
    }
}

impl SelectionPath for Bench {
    fn snapshot(&mut self) -> Result<Snapshot, ClipboardError> {
        if self.note(Step::Snapshot) {
            return Err(ClipboardError::Busy);
        }

        Ok(Snapshot::empty())
    }

    fn release_modifiers(&mut self) -> Modifiers {
        self.note(Step::Release);

        Modifiers::NONE
    }

    fn copy(&mut self) -> Dispatched {
        self.note(Step::Copy);

        Dispatched {
            calls: 1,
            requested: CHORD_EVENTS,
            accepted: CHORD_EVENTS,
        }
    }

    fn wait(&mut self, baseline: u32) -> Wait {
        self.note(Step::Wait);
        self.baseline_seen = Some(baseline);

        self.wait
    }

    fn sequence(&mut self) -> u32 {
        self.sequence_probes += 1;

        self.sequence_now
    }

    fn read(&mut self) -> Result<Option<String>, ClipboardError> {
        if self.note(Step::Read) {
            return Err(ClipboardError::Busy);
        }

        Ok(self.reads.clone())
    }

    fn write(&mut self, text: &str) -> Result<(), ClipboardError> {
        if self.note(Step::Write) {
            return Err(ClipboardError::Busy);
        }

        self.written = Some(text.to_owned());
        Ok(())
    }

    fn paste(&mut self) -> Dispatched {
        self.note(Step::Paste);

        Dispatched {
            calls: 1,
            requested: CHORD_EVENTS,
            accepted: CHORD_EVENTS,
        }
    }

    fn switch(&mut self, target: LayoutId) {
        self.note(Step::Switch);
        self.switched = Some(target);
    }

    fn restore_modifiers(&mut self) -> Modifiers {
        self.note(Step::RestoreModifiers);

        Modifiers::NONE
    }

    fn restore_clipboard(&mut self, _snapshot: &Snapshot, probe: u32) {
        self.note(Step::RestoreClipboard);
        self.restore_probe = Some(probe);
    }

    fn reclaim_clipboard(&mut self, _snapshot: &Snapshot) {
        self.note(Step::Reclaim);
    }
}

/// The RU/EN pair of FR-25 as a plan — the layouts of the machine this product was written for.
///
/// `foreground` is the tie-break of step 5; the two maps come from `convert::fallback_map`, so
/// nothing here depends on which layouts happen to be installed on the machine running the test.
fn pair_plan(foreground: LayoutId) -> Plan {
    let maps: Vec<LayoutMap> = [convert::FALLBACK_US, convert::FALLBACK_RUSSIAN]
        .into_iter()
        .map(|id| convert::fallback_map(id).expect("the hardwired map of FR-25"))
        .collect();

    let cycle = Cycle::from_layouts(&[convert::FALLBACK_US, convert::FALLBACK_RUSSIAN])
        .expect("a cycle of the two layouts of FR-25");

    Plan::new(
        maps,
        cycle,
        foreground,
        Duration::from_millis(300),
        Duration::from_millis(200),
        0,
    )
}

// ---------------------------------------------------------------------------------------
// Acceptance point 9 — the eight steps, in the order FR-61 writes them
// ---------------------------------------------------------------------------------------

/// **Acceptance point 9.** The eight steps of FR-61 run, once each, in the order of the
/// requirement, with the two of FR-40 bracketing them.
#[test]
fn the_eight_steps_of_fr61_run_in_the_order_the_requirement_writes_them() {
    let mut bench = Bench::with_selection("ghbdtn");
    let plan = pair_plan(convert::FALLBACK_US);

    let outcome = selection::run(&mut bench, &plan);

    assert_eq!(
        bench.steps,
        vec![
            Step::Snapshot,         // 1 — save the clipboard, FR-64
            Step::Release,          // FR-40 step 3
            Step::Copy,             // 2 — Ctrl+C
            Step::Wait,             // 3 — the sequence number
            Step::Read,             // 4 — CF_UNICODETEXT
            Step::Write,            // 6a — the recoded text
            Step::Paste,            // 6b — Ctrl+V
            Step::Switch,           // 7 — the layout
            Step::RestoreModifiers, // FR-40 step 6
            Step::RestoreClipboard, // 8 — the user's clipboard, with its delay
        ],
        "the order of FR-61 is the requirement"
    );

    // Step 5 has no call of its own: it is a pure function between steps 4 and 6, and what it
    // did is visible in what step 6 was given.
    assert_eq!(bench.written.as_deref(), Some("привет"));
    assert_eq!(bench.switched, Some(convert::FALLBACK_RUSSIAN));
    assert_eq!(
        outcome,
        Outcome::Converted {
            mapped: 6,
            carried: 0
        }
    );
    assert!(!outcome.falls_back());
}

// ---------------------------------------------------------------------------------------
// Acceptance point 10 — step 8 runs when steps 4 to 7 did not
// ---------------------------------------------------------------------------------------

/// **Acceptance point 10, the behavioural half.** Every step from 4 to 7 is made to fail in turn,
/// and step 8 runs every time.
#[test]
fn step_eight_runs_however_steps_four_to_seven_end() {
    for failing in [Step::Read, Step::Write] {
        let mut bench = Bench::with_selection("ghbdtn").failing_at(failing);
        let plan = pair_plan(convert::FALLBACK_US);

        let outcome = selection::run(&mut bench, &plan);

        assert!(
            bench.ran(Step::RestoreClipboard),
            "step 8 must run when step {failing:?} failed: {:?}",
            bench.steps
        );
        assert!(
            bench.ran(Step::RestoreModifiers),
            "FR-40 step 6 must run when step {failing:?} failed"
        );
        assert_eq!(outcome, Outcome::Refused(Refusal::Clipboard));
        assert!(outcome.falls_back());
    }

    // Step 4 answering "there is no text at all" — the application copied a picture.
    let mut empty = Bench::with_selection("");
    let outcome = selection::run(&mut empty, &pair_plan(convert::FALLBACK_US));
    assert!(empty.ran(Step::RestoreClipboard));
    assert_eq!(outcome, Outcome::Refused(Refusal::NoText));

    // Step 5 unable to decide a direction: nothing distinguishes a layout and the foreground
    // window is in neither of them.
    let mut undecidable = Bench::with_selection("123");
    let outcome = selection::run(&mut undecidable, &pair_plan(LayoutId::default()));
    assert!(undecidable.ran(Step::RestoreClipboard));
    assert_eq!(outcome, Outcome::Refused(Refusal::NoDirection));
}

/// **И-1 and И-4 of Э18 — what comes through step 8's door, and not merely that it opened.**
///
/// The test above proves the door opens on every failure path. It proved that before the audit of
/// 2026-08-31 as well, and the user's clipboard was lost all the same: behind the door stands the
/// gate of decision П-5, and on these four paths this program has written nothing for the ring of
/// FR-63 to recognise. The door and the number are therefore asserted apart, because that is how
/// they failed — one right, one missing, and nothing red anywhere.
///
/// [`PROBE_ANSWER`] is the number step 3 answered with. Handing it to step 8 is what lets the
/// gate tell the answer to this program's own probe from a stranger's copy two hundred
/// milliseconds later; what the gate then does with it is
/// `selection::tests::the_answer_to_the_probe_opens_the_gate_and_a_third_writer_shuts_it`, a unit
/// test, because the gate is private and the decision belongs inside the module.
///
/// The last row is И-4: the happy path hands the same number, and on it the gate has two reasons
/// to open rather than one — step 6's own mark is in the ring as it always was.
#[test]
fn every_failure_path_hands_step_eight_the_number_step_three_saw() {
    let cases: Vec<(&str, Bench, LayoutId, Outcome)> = vec![
        (
            "step 4 found no text — a picture or a file list was selected",
            Bench::with_selection(""),
            convert::FALLBACK_US,
            Outcome::Refused(Refusal::NoText),
        ),
        (
            "step 4 could not read the clipboard",
            Bench::with_selection("ghbdtn").failing_at(Step::Read),
            convert::FALLBACK_US,
            Outcome::Refused(Refusal::Clipboard),
        ),
        (
            "step 5 could not decide a direction",
            Bench::with_selection("123"),
            LayoutId::default(),
            Outcome::Refused(Refusal::NoDirection),
        ),
        (
            "step 6 could not write",
            Bench::with_selection("ghbdtn").failing_at(Step::Write),
            convert::FALLBACK_US,
            Outcome::Refused(Refusal::Clipboard),
        ),
        (
            "nothing failed — И-4, the happy path is what it was",
            Bench::with_selection("ghbdtn"),
            convert::FALLBACK_US,
            Outcome::Converted {
                mapped: 6,
                carried: 0,
            },
        ),
    ];

    for (what, mut bench, foreground, expected) in cases {
        let outcome = selection::run(&mut bench, &pair_plan(foreground));

        assert_eq!(outcome, expected, "{what}");
        assert!(
            bench.ran(Step::RestoreClipboard),
            "{what}: step 8's door opened: {:?}",
            bench.steps
        );
        assert_eq!(
            bench.restore_probe,
            Some(PROBE_ANSWER),
            "{what}: and step 8 was handed the number step 3 answered with"
        );
    }

    // And the path that owes nothing hands nothing. Step 3 said the clipboard never moved, so it
    // is still exactly what the user left there and step 8 must not run at all — `Owed::Nothing`,
    // the reason the restore is armed at step 3 rather than at step 1.
    let mut quiet = Bench::without_selection();

    assert_eq!(
        selection::run(&mut quiet, &pair_plan(convert::FALLBACK_US)),
        Outcome::NoSelection
    );
    assert!(!quiet.ran(Step::RestoreClipboard));
    assert_eq!(
        quiet.restore_probe, None,
        "no probe was answered, so there is no number and no step 8"
    );
}

/// **Acceptance point 10, the hardest path.** A panic between steps 4 and 7 still restores.
///
/// `panic = "abort"` is set for `[profile.release]` only, so a Debug build unwinds — and the
/// unwind runs `Session::drop`, which is the whole mechanism. In Release the process ends and
/// the clipboard is freed by the system with the owning thread, which T-07-1 measured.
#[test]
fn a_panic_between_steps_four_and_seven_still_restores_the_clipboard() {
    for panicking in [Step::Read, Step::Write, Step::Paste, Step::Switch] {
        // The bench has to survive the unwind to be read afterwards, so it is moved into the
        // caught frame behind a pointer the frame gives back.
        let bench = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut bench = Bench::with_selection("ghbdtn").panicking_at(panicking);
            let plan = pair_plan(convert::FALLBACK_US);
            let _ = selection::run(&mut bench, &plan);
            bench
        }));

        // The panic escaped, which is the point: nothing in the module caught it.
        assert!(
            bench.is_err(),
            "the bench was told to panic at {panicking:?}"
        );
    }

    // And now the same thing with the tape kept outside the frame, so that it can be read.
    let tape = std::sync::Arc::new(Mutex::new(Vec::<Step>::new()));
    let recorder = std::sync::Arc::clone(&tape);

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let mut bench = Bench::with_selection("ghbdtn").panicking_at(Step::Write);
        let plan = pair_plan(convert::FALLBACK_US);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            selection::run(&mut bench, &plan)
        }));
        recorder
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(&bench.steps);
        outcome
    }));
    std::panic::set_hook(previous);

    assert!(result.is_ok(), "the outer frame did not panic");
    let steps = tape.lock().unwrap_or_else(PoisonError::into_inner).clone();

    assert!(
        steps.contains(&Step::RestoreClipboard),
        "step 8 must run on the unwinding path too: {steps:?}"
    );
}

/// **Acceptance point 10, the structural half — what guarantees it.**
///
/// Step 8 is reached from exactly one place in the module, and that place is a destructor. A
/// future edit that restored by hand somewhere, or that added an early `return` past a hand-written
/// restore, has to break this test first. The same device, and the same argument, as
/// [`the_clipboard_is_opened_in_one_place_and_closed_only_from_drop`].
#[test]
fn step_eight_is_reached_from_a_destructor_and_from_nowhere_else() {
    let source = source_of("selection.rs");
    let product = cut_at(&source, "mod tests {", "the product half of the module");

    for (call, owner) in [
        ("path.restore_clipboard(", "impl<P: Path> Drop for Session"),
        ("path.restore_modifiers(", "impl<P: Path> Drop for Session"),
        // Task T-13-11 added a second way back to the user's clipboard — the late `Ctrl+C` of
        // step 3's timeout. It is a **second reason**, not a second mechanism: the guard still
        // owns every put-back, and this row is what keeps it that way.
        ("path.reclaim_clipboard(", "impl<P: Path> Drop for Session"),
    ] {
        let hits = code_lines_with(product, call);

        assert_eq!(hits.len(), 1, "{call} is called from one place: {hits:?}");

        let after = product
            .split(owner)
            .nth(1)
            .unwrap_or_else(|| panic!("{owner} exists"));
        let body = cut_at(after, "// ------", owner);

        assert!(body.contains(call), "and that one place is inside {owner}");
    }

    // And `run` itself contains no restore at all: the guard owns both of them.
    let run_body = cut_at(
        product
            .split("pub fn run<P: Path>")
            .nth(1)
            .expect("run exists"),
        "\n/// ",
        "the body of Session::run",
    );

    assert!(
        !run_body.contains("restore_clipboard(")
            && !run_body.contains("restore_modifiers(")
            && !run_body.contains("reclaim_clipboard("),
        "the eight steps must not restore by hand — the guard does it"
    );
}

/// **The note to step 8 of §4.7 — decision П-5, and the order FR-80 needs.**
///
/// Three facts about `restore_after`, and the first of them is older than this task: the thread
/// is asked **before** anything sleeps, because a 200 ms sleep on the thread that owns the hook
/// is the failure FR-80 removes a hook for. The other two are П-5 — the question is asked after
/// the delay, because it is a question about the delay window, and nothing is written before it
/// has been answered.
#[test]
fn the_delayed_restore_asks_the_thread_first_and_the_sequence_number_last() {
    let source = source_of("selection.rs");
    let product = cut_at(&source, "mod tests {", "the product half of the module");

    let body = cut_at(
        product
            .split("pub fn restore_after(")
            .nth(1)
            .expect("the delayed restore exists"),
        "\n// ---",
        "the body of restore_after",
    );

    let thread = body
        .find("require_blocking_thread()")
        .expect("FR-80, NFR-01: the thread is asked");
    let sleeps = body
        .find("sleep(delay)")
        .expect("the delay of step 8 is waited out");
    // ⚠ Т-18-1 moved this needle, and it is worth saying why rather than letting a reader think
    // the sweep was loosened. It used to read `restore_is_due(sequence_number())`. The gate now
    // takes a second argument — the number step 3 saw — and the needle names it, so the sweep
    // still fails if the question stops being asked, and now fails as well if it is asked
    // without the number that makes it answerable on the failure paths.
    let asks = body
        .find("restore_is_due(sequence_number(), probe)")
        .expect("П-5: the sequence number is asked once more, against the probe of step 3");
    let puts = body
        .find("restore(owner, snapshot)")
        .expect("the restore itself is still here");

    assert!(
        thread < sleeps,
        "FR-80, NFR-01: nothing sleeps before the thread has been asked"
    );
    assert!(
        sleeps < asks,
        "П-5 asks about the delay window, so the question comes after the delay"
    );
    assert!(asks < puts, "and nothing is put back before it is answered");

    // And the answer is the classification of FR-63, not a second one invented here.
    let decision = cut_at(
        product
            .split("fn restore_is_due(")
            .nth(1)
            .expect("the decision of П-5 exists"),
        "\n///",
        "the body of restore_is_due",
    );

    assert!(
        decision.contains("is_own_change(current)"),
        "the verdict FR-63 builds is what decides the restore"
    );
    assert!(
        decision.contains("RESTORE_SKIPS") && decision.contains("note_restore_skipped()"),
        "a skipped restore is counted and journalled"
    );
}

/// ⭐ **Task Т-22-6, finding м6 of the audit of 2026-09-01 — the boundary of the third exit.**
///
/// `EmptyClipboard` succeeded and not one format went back: the user's content is gone and this
/// module put nothing in its place. That is the worst state it can produce and it used to leave no
/// trace whatever — `Restored { placed: 0, .. }` is an `Ok`, and both callers drop the answer.
///
/// A pure function of two numbers so that the boundary is driven here rather than by whatever is
/// on somebody's clipboard, which is why `read_refuses_size` is one too.
#[test]
fn a_restore_that_placed_nothing_is_told_from_one_that_had_nothing_to_place() {
    // The failure: something to put back, nothing put back.
    assert!(selection::restore_placed_nothing(0, 1));
    assert!(selection::restore_placed_nothing(0, 7));

    // Not a failure: the clipboard held nothing this module could keep, so putting nothing back is
    // the whole of the correct behaviour. An empty snapshot must never raise the counter.
    assert!(!selection::restore_placed_nothing(0, 0));

    // And any format that did go back means the restore happened, however many were refused —
    // FR-64, acceptance point 12: one format's refusal does not end the restore.
    assert!(!selection::restore_placed_nothing(1, 7));
    assert!(!selection::restore_placed_nothing(7, 7));
}

/// ⭐ **Task Т-22-6.** A restore that is owed, is attempted, and fails leaves a counter and a
/// journal entry — and it is **not** counted as the lawful skip of decision П-5.
///
/// # How the failure is staged, and why nothing on the machine is touched
///
/// Through the module's own first gate. `Clipboard::open` begins with `require_blocking_thread`,
/// and a thread that owns the typing buffer is by definition the input thread of section 6.1 —
/// the one thread FR-80 and NFR-01 forbid to block. Installing a recorder here makes this thread
/// that thread, so `restore` is refused **before** `OpenClipboard` is reached: no clipboard is
/// opened, nothing is emptied, and the machine's clipboard is not touched at all. It is the
/// open-refusal exit of the finding, driven through the product's own path.
///
/// The two counters are asserted together, because conflating them is the other half of the
/// defect: a broken restore must not be able to hide behind a lawful refusal.
#[test]
fn a_failed_restore_leaves_a_counter_and_a_journal_entry_and_is_not_a_skip() {
    use lang_switcher::buffer::{self, Recorder};
    use lang_switcher::diag::Operation;

    let _serialised = serialised();

    let before = selection::counters();
    let journal_before = lang_switcher::diag::recorded();

    // This thread is now the input thread as far as `caller_may_block` is concerned.
    buffer::install_recorder(Recorder::with_capacity(8));

    let outcome = selection::restore(HWND::default(), &Snapshot::empty());

    buffer::uninstall();

    assert!(
        matches!(outcome, Err(ClipboardError::WrongThread)),
        "the restore must be refused before any clipboard call, and it was {outcome:?}"
    );

    let after = selection::counters();

    assert_eq!(
        after.restore_failures,
        before.restore_failures + 1,
        "Т-22-6: a restore that was owed and did not happen may not be invisible"
    );
    assert_eq!(
        after.restore_skips, before.restore_skips,
        "and it is not the lawful П-5 skip — those two must never be one number"
    );

    // The journal grew, and what it grew by is the named row and not `UNLISTED`.
    assert!(lang_switcher::diag::recorded() > journal_before);

    let named = Operation::from_name("clipboard restore failed");

    assert!(
        lang_switcher::diag::snapshot()
            .iter()
            .any(|event| event.ordinal >= journal_before && event.operation == named),
        "the failure leaves the row of the closed table behind it"
    );
}

/// **SEC-07.** The failed restore of task Т-22-6 has a name in the journal's closed vocabulary,
/// and it is a **different** name from the lawful skip beside it.
#[test]
fn the_failed_restore_has_a_name_of_its_own_in_the_journal() {
    use lang_switcher::diag::{Kind, Operation};

    let failed = Operation::from_name("clipboard restore failed");

    assert_ne!(
        failed,
        Operation::UNLISTED,
        "the row task Т-22-6 added is in the table, so the entry is named rather than counted"
    );
    assert_eq!(failed.name(), "clipboard restore failed");
    assert_eq!(failed.kind(), Kind::Selection);

    assert_ne!(
        failed,
        Operation::from_name("clipboard restore skipped"),
        "a failure and a lawful skip are opposite events and may not share a row"
    );
}

/// **SEC-07.** The skipped restore of decision П-5 has a name in the journal's closed vocabulary.
#[test]
fn the_skipped_restore_has_a_name_in_the_journal() {
    use lang_switcher::diag::{Kind, Operation};

    let operation = Operation::from_name("clipboard restore skipped");

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "the row task T-13-11 added is in the table, so the entry is named rather than counted"
    );
    assert_eq!(operation.name(), "clipboard restore skipped");
    assert_eq!(operation.kind(), Kind::Selection);
    assert_eq!(operation.kind().name(), "selection");
}

// ---------------------------------------------------------------------------------------
// Acceptance points 11 and 12 — how "there is a selection" is decided
// ---------------------------------------------------------------------------------------

/// **Acceptance point 11.** The presence of a selection is the answer of step 3 and nothing else:
/// the baseline handed to the wait is the sequence number of the snapshot, and no other question
/// is asked of the system.
#[test]
fn a_selection_is_recognised_by_the_sequence_number_and_by_nothing_else() {
    let mut bench = Bench::with_selection("ghbdtn");
    let plan = pair_plan(convert::FALLBACK_US);

    let _ = selection::run(&mut bench, &plan);

    // The baseline came from step 1's snapshot, taken before `Ctrl+C` went out.
    assert_eq!(bench.baseline_seen, Some(Snapshot::empty().sequence()));

    // And the order is the proof that the question is "did it change": the snapshot is read
    // before the copy, and the wait comes after it.
    assert!(bench.position_of(Step::Snapshot) < bench.position_of(Step::Copy));
    assert!(bench.position_of(Step::Copy) < bench.position_of(Step::Wait));

    // There is no other way of asking, and the module does not invent one: nothing in it looks
    // at a window, a selection or an accessibility interface.
    let source = source_of("selection.rs");
    let product = cut_at(&source, "mod tests {", "the product half of the module");

    for absent in [
        "GetFocus",
        "EM_GETSEL",
        "IUIAutomation",
        "TextPattern",
        "SendMessageTimeout",
    ] {
        assert!(
            code_lines_with(product, absent).is_empty(),
            "there is no way to ask whether a selection exists; {absent} must not appear"
        );
    }
}

/// **Acceptance point 12.** No change within the timeout is "there is no selection", the press
/// falls back to the typing-buffer path — and **nothing was written to the clipboard**, so what
/// the user has there is what they had before.
#[test]
fn no_change_within_the_timeout_means_no_selection_and_falls_back() {
    let mut bench = Bench::without_selection();
    let plan = pair_plan(convert::FALLBACK_US);

    let outcome = selection::run(&mut bench, &plan);

    assert_eq!(outcome, Outcome::NoSelection);
    assert!(
        outcome.falls_back(),
        "FR-60: the press goes down the typing-buffer path"
    );

    assert_eq!(
        bench.steps,
        vec![
            Step::Snapshot,
            Step::Release,
            Step::Copy,
            Step::Wait,
            Step::RestoreModifiers,
        ],
        "steps 4 to 8 do not run when there is no selection"
    );

    // ⚠ And step 8 is deliberately **not** among them. Nothing changed the clipboard, so putting
    // the snapshot back would replace the user's clipboard with this program's copy of it —
    // losing precisely the formats FR-64 admits it cannot capture.
    assert!(!bench.ran(Step::RestoreClipboard));
    assert!(!bench.ran(Step::Write));
    assert!(bench.written.is_none());
}

/// **The late `Ctrl+C` — the audit of 2026-08-24, task T-13-11.** Both outcomes of the question
/// step 3's timeout now asks before it walks away.
///
/// The two halves are one test because [`selection::path_counters`] is process-wide and the
/// assertions are about `late_copies` moving by **exactly** one and by **exactly** nothing, which
/// two tests running beside each other could not both claim.
#[test]
fn a_clipboard_that_moved_after_the_timeout_is_put_back_and_one_that_did_not_is_left_alone() {
    let before = selection::path_counters();

    // ---- the clipboard did not move: everything is as it was before this task ---------------
    let mut quiet = Bench::without_selection();
    let outcome = selection::run(&mut quiet, &pair_plan(convert::FALLBACK_US));

    assert_eq!(outcome, Outcome::NoSelection);
    assert_eq!(
        quiet.steps,
        vec![
            Step::Snapshot,
            Step::Release,
            Step::Copy,
            Step::Wait,
            Step::RestoreModifiers,
        ],
        "a clipboard nobody touched is not written to"
    );
    assert!(!quiet.ran(Step::Reclaim), "there is nothing to put back");
    assert!(!quiet.ran(Step::RestoreClipboard));
    assert_eq!(
        quiet.sequence_probes, 1,
        "the question is asked once, and only on the timeout"
    );

    let quiet_after = selection::path_counters();
    assert_eq!(
        quiet_after.late_copies, before.late_copies,
        "nothing moved, so nothing is counted"
    );

    // ---- the clipboard moved after the wait gave up: the snapshot goes back ------------------
    let mut late = Bench::moving_after_the_timeout();
    let outcome = selection::run(&mut late, &pair_plan(convert::FALLBACK_US));

    // FR-60 is unchanged: there was no selection to convert, so the press still falls back.
    assert_eq!(outcome, Outcome::NoSelection);
    assert!(outcome.falls_back());

    assert_eq!(
        late.steps,
        vec![
            Step::Snapshot,
            Step::Release,
            Step::Copy,
            Step::Wait,
            Step::RestoreModifiers, // FR-40 step 6 first — the user is waiting for their Shift
            Step::Reclaim,          // and then the clipboard they had before the probe
        ],
        "the snapshot goes back, and after the modifiers"
    );

    // ⚠ It is **not** step 8: no delay to wait out, and no refusal to make.
    assert!(
        !late.ran(Step::RestoreClipboard),
        "the delayed, refusable door of step 8 is not the one this path takes"
    );
    assert_eq!(late.sequence_probes, 1);

    let late_after = selection::path_counters();
    assert_eq!(
        late_after.late_copies,
        before.late_copies + 1,
        "the race is counted where it can be read"
    );

    // Steps 4 to 7 still did not run: there was no selection, and nothing was converted.
    assert!(!late.ran(Step::Read));
    assert!(!late.ran(Step::Write));
    assert!(!late.ran(Step::Paste));
    assert!(!late.ran(Step::Switch));
    assert!(late.written.is_none());
}

// ---------------------------------------------------------------------------------------
// Acceptance point 13 — the two chords
// ---------------------------------------------------------------------------------------

/// **Acceptance point 13.** `Ctrl+C` and `Ctrl+V` carry the signature of FR-03, are balanced, and
/// leave through module `inject`.
#[test]
fn the_two_chords_carry_the_signature_of_fr03_and_go_out_through_inject() {
    for (name, chord, key) in [
        ("Ctrl+C", selection::copy_chord(), u16::from(b'C')),
        ("Ctrl+V", selection::paste_chord(), u16::from(b'V')),
    ] {
        assert_eq!(chord.len(), CHORD_EVENTS, "{name} is four events");

        let keyboard: Vec<_> = chord
            .iter()
            .map(|event| inject::keyboard(event).expect("every event is a keyboard event"))
            .collect();

        // FR-03 — the whole point of the check.
        for event in &keyboard {
            assert_eq!(
                event.dwExtraInfo,
                lang_switcher::hook::INJECTED_SIGNATURE,
                "{name}: every event carries the signature of FR-03"
            );
            assert_eq!(event.time, 0, "{name}: the system stamps the event (FR-12)");
            assert_eq!(
                event.wScan, 0,
                "{name}: the system derives the scan code from the virtual key"
            );
        }

        // Modifier down, key down, key up, modifier up — and the modifier is balanced, so
        // FR-40 step 6 cannot mistake ours for the user's.
        let control = windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL.0;
        assert_eq!(keyboard[0].wVk.0, control);
        assert_eq!(keyboard[1].wVk.0, key);
        assert_eq!(keyboard[2].wVk.0, key);
        assert_eq!(keyboard[3].wVk.0, control);

        let up = windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP;
        assert!(!keyboard[0].dwFlags.contains(up), "{name}: Ctrl goes down");
        assert!(!keyboard[1].dwFlags.contains(up));
        assert!(keyboard[2].dwFlags.contains(up));
        assert!(
            keyboard[3].dwFlags.contains(up),
            "{name}: Ctrl comes back up"
        );
    }

    // And the sending itself is `inject`'s: the module calls `inject::dispatch`, which is what
    // counts FR-45 and paces FR-44, rather than `SendInput` of its own.
    let source = source_of("selection.rs");
    let product = cut_at(&source, "mod tests {", "the product half of the module");

    assert_eq!(
        code_lines_with(product, "crate::inject::dispatch(").len(),
        4,
        "four dispatches: the modifier release, the two chords and the modifier restore"
    );
}

// ---------------------------------------------------------------------------------------
// Acceptance points 14, 15 and 16 — step 5
// ---------------------------------------------------------------------------------------

/// Recodes `text` the way step 5 does, and answers what came out.
fn step_five(text: &str, foreground: LayoutId) -> String {
    let plan = pair_plan(foreground);
    let mut bench = Bench::with_selection(text);
    let _ = selection::run(&mut bench, &plan);

    bench.written.unwrap_or_default()
}

/// **Acceptance point 14.** The script is decided by which layout the characters belong to, and
/// a mixed selection is converted whole, in the one direction the majority names.
#[test]
fn the_script_of_the_selection_decides_the_direction() {
    // Latin text: only the English layout produces those six characters.
    assert_eq!(step_five("ghbdtn", convert::FALLBACK_US), "привет");
    // …and the foreground layout does not change the answer, because the text does.
    assert_eq!(step_five("ghbdtn", convert::FALLBACK_RUSSIAN), "привет");

    // Cyrillic text: the same press, the other way.
    assert_eq!(step_five("привет", convert::FALLBACK_US), "ghbdtn");
    assert_eq!(step_five("привет", convert::FALLBACK_RUSSIAN), "ghbdtn");

    // Case is carried: `find_key` answers `Shift` and the target key answers with its own.
    assert_eq!(step_five("Ghbdtn", convert::FALLBACK_US), "Привет");
    assert_eq!(step_five("ПРИВЕТ", convert::FALLBACK_US), "GHBDTN");

    // Digits, spaces and punctuation of both layouts decide nothing, so the letters do.
    assert_eq!(step_five("ghbdtn 123", convert::FALLBACK_US), "привет 123");
    assert_eq!(step_five("привет 123", convert::FALLBACK_US), "ghbdtn 123");

    // ⚠ **Mixed text — the canon this line used to hold was rewritten by the user's word**,
    // question **111.4**, task Т-48-3. It read «`ghbdtn привет 123` → `привет привет 123`»:
    // the whole selection went one way, so the Cyrillic word had no reverse mapping in the
    // source and came out exactly as it went in. Step 5 now decides the script **per word**
    // (FR-61 step 5), so each word goes to the next layout after **its own** source and the
    // foreground layout no longer changes the answer — it only breaks the tie for the text as a
    // whole, which is what step 7 switches to.
    assert_eq!(
        step_five("ghbdtn привет 123", convert::FALLBACK_US),
        "привет ghbdtn 123"
    );
    assert_eq!(
        step_five("ghbdtn привет 123", convert::FALLBACK_RUSSIAN),
        "привет ghbdtn 123"
    );

    // A selection with nothing to decide by and a foreground layout outside the pair is refused
    // rather than converted in a direction nobody chose — FR-30, «остальные раскладки
    // игнорируются».
    let maps: Vec<LayoutMap> = [convert::FALLBACK_US, convert::FALLBACK_RUSSIAN]
        .into_iter()
        .map(|id| convert::fallback_map(id).expect("the hardwired map"))
        .collect();
    assert_eq!(
        selection::detect_source("123", &maps, LayoutId::default()),
        None
    );
    assert_eq!(
        selection::detect_source("123", &maps, convert::FALLBACK_US),
        Some(0)
    );
}

// ---------------------------------------------------------------------------------------
// FR-61 step 5 by words — the user's third finding, question 111.3, task Т-48-3
// ---------------------------------------------------------------------------------------

/// Runs step 5 and answers **both** halves: what step 6 was given, and what step 7 switched to.
///
/// [`step_five`] answers the first alone, and the whole point of question 111.3 is that the two
/// no longer move together: each word is recoded by **its own** source layout, while step 7
/// switches to the target of the decision made over the **whole** text.
fn step_five_switching(text: &str, foreground: LayoutId) -> (String, Option<LayoutId>) {
    let plan = pair_plan(foreground);
    let mut bench = Bench::with_selection(text);
    let _ = selection::run(&mut bench, &plan);

    (bench.written.clone().unwrap_or_default(), bench.switched)
}

/// **The user's third finding, word for word** — 2026-09-09, question **111**:
///
/// > если набрать текст: `z [jntk yfgbcfnm ckjdj нфтвуч yj pf,sk cvtybnm` затем выделить
/// > набранный текст и нажать pause, то получится: `я хотел написать слово нфтвуч но забыл
/// > сменить`
///
/// The sentence is seven words typed on an English keyboard and one — `нфтвуч` — typed on a
/// Russian one. Before task Т-48-3 the script was decided **once for the whole selection**
/// (29 distinguishing Latin characters against 6 Cyrillic ones, so the source was English), and
/// the Russian word had no reverse mapping in the English map: FR-23 read backwards carried it
/// over untouched, which is exactly what the user saw.
///
/// It is `yandex` that the Russian word is owed: `нфтвуч` is what those six keys give in the
/// Russian layout, and the next layout in the cycle after Russian is English.
#[test]
fn the_sentence_of_the_users_third_finding_converts_word_by_word() {
    let typed = "z [jntk yfgbcfnm ckjdj нфтвуч yj pf,sk cvtybnm";
    let owed = "я хотел написать слово yandex но забыл сменить";

    // The foreground layout does not enter into it: every word of this sentence distinguishes a
    // layout on its own, so the tie-break has nothing to break.
    let (written, switched) = step_five_switching(typed, convert::FALLBACK_US);
    assert_eq!(written, owed, "each word follows its own layout");

    // Step 7 follows the decision over the **whole** text — English by a strict majority — so
    // the layout the user is left typing in is Russian, the next one after it.
    assert_eq!(switched, Some(convert::FALLBACK_RUSSIAN));

    let (written, switched) = step_five_switching(typed, convert::FALLBACK_RUSSIAN);
    assert_eq!(
        written, owed,
        "and the foreground layout does not change it"
    );
    assert_eq!(switched, Some(convert::FALLBACK_RUSSIAN));
}

/// **The table of small cases of FR-61 step 5 by words** — acceptance point 2 of the stage.
///
/// Each row is one property of the rule, and the rows that were true before task Т-48-3 are
/// here to say that they still are: the whole-word paths of the feature did not move.
#[test]
fn each_word_of_the_selection_is_recoded_by_its_own_layout() {
    // One word, one layout — unchanged in both directions and under both tie-breaks.
    assert_eq!(step_five("ghbdtn", convert::FALLBACK_US), "привет");
    assert_eq!(step_five("ghbdtn", convert::FALLBACK_RUSSIAN), "привет");
    assert_eq!(step_five("привет", convert::FALLBACK_US), "ghbdtn");
    assert_eq!(step_five("привет", convert::FALLBACK_RUSSIAN), "ghbdtn");

    // Two words of two layouts, in either order: each goes to the next after its own source.
    assert_eq!(
        step_five("ghbdtn привет", convert::FALLBACK_US),
        "привет ghbdtn"
    );
    assert_eq!(
        step_five("привет ghbdtn", convert::FALLBACK_RUSSIAN),
        "ghbdtn привет"
    );

    // `pf,sk` and `z` out of the user's sentence, each on its own: the comma of the English
    // layout is a distinguishing character, because the same key gives `б` in the Russian one.
    assert_eq!(step_five("pf,sk", convert::FALLBACK_US), "забыл");
    assert_eq!(step_five("z", convert::FALLBACK_US), "я");
    assert_eq!(step_five("нфтвуч", convert::FALLBACK_US), "yandex");

    // ⚠ **A word with nothing to decide by follows the decision over the whole text** — and
    // that decision is the tie-break of the foreground layout when the text has nothing to
    // decide by either. Both are what they were before Т-48-3.
    assert_eq!(step_five("123", convert::FALLBACK_US), "123");
    assert_eq!(step_five("!!!", convert::FALLBACK_US), "!!!");
    assert_eq!(step_five("!!!", convert::FALLBACK_RUSSIAN), "!!!");

    // …and beside a word that **does** decide, the digits follow that word's decision over the
    // text rather than the foreground layout.
    assert_eq!(step_five("ghbdtn 123", convert::FALLBACK_US), "привет 123");
    assert_eq!(step_five("привет 123", convert::FALLBACK_US), "ghbdtn 123");

    // Case is carried, per word, exactly as it was per text.
    assert_eq!(
        step_five("Ghbdtn Привет", convert::FALLBACK_US),
        "Привет Ghbdtn"
    );
    assert_eq!(
        step_five("ПРИВЕТ GHBDTN", convert::FALLBACK_US),
        "GHBDTN ПРИВЕТ"
    );

    // ⚠ **Separators are carried through untouched, whatever they are and however many.** The
    // words are the maximal runs of characters that are neither whitespace nor control, so two
    // spaces stay two spaces and a line break stays a line break.
    assert_eq!(
        step_five("ghbdtn  привет", convert::FALLBACK_US),
        "привет  ghbdtn"
    );
    assert_eq!(
        step_five("ghbdtn\nпривет\tghbdtn", convert::FALLBACK_US),
        "привет\nghbdtn\tпривет"
    );

    // A selection that decides nothing and whose foreground layout is outside the pair is still
    // refused rather than converted in a direction nobody chose — FR-30, and step 5 has no word
    // to fall back on when the text itself has no answer.
    let (written, switched) = step_five_switching("123", LayoutId::default());
    assert_eq!(written, "", "nothing was written");
    assert_eq!(switched, None, "and nothing was switched");
}

/// Builds a synthetic layout over three keys, so that a cycle of **three** layouts can be
/// driven without asking the machine what it has installed.
///
/// The space bar is in every one of them with the same character, for the reason
/// `tests\cycle.rs` gives: the space is a physical key like any other, and a synthetic layout
/// that left it blank would make the separator rule vacuous.
fn synthetic_map(layout: LayoutId, characters: [char; 3]) -> LayoutMap {
    /// `A`, `S` and `D` of the middle row — three keys and nothing else is needed.
    const SCANS: [u16; 3] = [0x1E, 0x1F, 0x20];
    const SPACE: u16 = 0x39;
    const MAIN_BLOCK: bool = false;

    let mut builder = LayoutMapBuilder::new(layout);

    for (&scan, &character) in SCANS.iter().zip(characters.iter()) {
        builder.set(
            scan,
            MAIN_BLOCK,
            Mods::NONE,
            KeyMapping::from_char(character),
        );
    }

    builder.set(SPACE, MAIN_BLOCK, Mods::NONE, KeyMapping::from_char(' '));

    builder.finish()
}

/// **A cycle of three layouts sends every word to the next one after its own source** — FR-30,
/// FR-31 and step 5 of FR-61 together.
///
/// The pair is the easy case, because "the other one" is the same answer whichever way you read
/// it. With three layouts the two readings part company, and this is the one that FR-61 asks
/// for: the step is taken from the **word's** source, not from the text's.
#[test]
fn a_cycle_of_three_layouts_sends_each_word_to_the_next_after_its_own_source() {
    let first = LayoutId::from_raw(0x0A0A_0A0A);
    let second = LayoutId::from_raw(0x0B0B_0B0B);
    let third = LayoutId::from_raw(0x0C0C_0C0C);

    let maps = vec![
        synthetic_map(first, ['a', 'b', 'c']),
        synthetic_map(second, ['d', 'e', 'f']),
        synthetic_map(third, ['g', 'h', 'i']),
    ];

    let cycle = Cycle::from_layouts(&[first, second, third]).expect("a cycle of three");
    let plan = Plan::new(
        maps,
        cycle,
        first,
        Duration::from_millis(300),
        Duration::from_millis(200),
        0,
    );

    let mut bench = Bench::with_selection("abc def ghi");
    let _ = selection::run(&mut bench, &plan);

    // `abc` was typed in the first layout and goes to the second; `def` in the second and goes
    // to the third; `ghi` in the third and comes round to the first.
    assert_eq!(bench.written.as_deref(), Some("def ghi abc"));

    // Three distinguishing characters each: the text as a whole decides nothing, the foreground
    // layout breaks the tie, and step 7 switches to the next one after **that**.
    assert_eq!(bench.switched, Some(second));
}

/// **Acceptance point 15.** A character with no reverse mapping in the source layout is carried
/// over unchanged and conversion goes on — FR-23 read backwards.
#[test]
fn a_character_without_a_reverse_mapping_is_carried_over() {
    let maps: Vec<LayoutMap> = [convert::FALLBACK_US, convert::FALLBACK_RUSSIAN]
        .into_iter()
        .map(|id| convert::fallback_map(id).expect("the hardwired map"))
        .collect();

    // An em dash, a currency sign and an emoji: no keyboard layout of this pair produces any of
    // them, so all three come through untouched and the letters around them still convert.
    let recoded = selection::recode("gh—bd€tn😀", &maps[0], &maps[1]);

    assert_eq!(recoded.text(), "пр—ив€ет😀");
    assert_eq!(recoded.mapped(), 6, "the six letters were recoded");
    assert_eq!(recoded.carried(), 3, "the three others were carried over");

    // A surrogate pair is one character and survives whole.
    assert_eq!(recoded.text().chars().count(), 9);

    // Nothing is ever dropped: every character of the input is accounted for.
    assert_eq!(recoded.mapped() + recoded.carried(), 9);
}

/// **Acceptance point 16 — reversibility, and the boundary of it, honestly.**
///
/// On the typing-buffer path FR-32 makes a double press exact **by construction**: the buffer
/// keeps the original scan codes and rendering them into the layout they were typed under gives
/// back what was typed, bit for bit. There is no buffer here. The second press reads the text the
/// first press produced, decides its script again and converts it back — so reversibility is a
/// property of the *mapping*, and it holds exactly where the mapping is injective on the
/// selection.
#[test]
fn a_double_press_comes_back_where_the_mapping_allows_it_and_not_where_it_does_not() {
    // ---- where it holds ----------------------------------------------------------------
    //
    // A run of one script, with the digits, spaces and punctuation of the main block: every
    // character has a reverse mapping in the source layout and a distinct image in the target.
    for (original, once) in [
        ("ghbdtn", "привет"),
        // `,` sits on `Shift` + the `/?` key in Russian, whose English reading is `?` — the very
        // «`Shift+2` = `@` в EN и `"` в RU» correspondence FR-04 gives as the reason scan codes
        // are stored, arriving here through the reverse index instead.
        ("Привет, мир!", "Ghbdtn? vbh!"),
        ("ghbdtn 123", "привет 123"),
        ("Ghbdtn!", "Привет!"),
        // Punctuation that moves between keys: `,` and `.` are on the letter block in Russian
        // and next to it in English, and both directions still come back.
        ("q,w.e/", "йбцюу."),
    ] {
        let there = step_five(original, convert::FALLBACK_US);
        assert_eq!(there, once, "first press of {original:?}");

        let back = step_five(&there, convert::FALLBACK_US);
        assert_eq!(back, original, "second press must give {original:?} back");
    }

    // ⭐ **Mixed scripts came back into this list with task Т-48-3** — question **111.3**, and
    // it is worth its own paragraph because this test used to say the opposite.
    //
    // While each press converted the *whole* selection in one direction, the half already in
    // the target script was dragged across with it and the second press dragged both halves the
    // other way: `ghbdtn привет` → `привет привет` → `ghbdtn ghbdtn`, and the text that came
    // out was not the text that went in. Step 5 now decides the script **per word**, so every
    // word keeps a direction of its own and the second press simply walks each of them one more
    // step round the cycle. With two layouts, one more step is the way back.
    let mixed = "ghbdtn привет";
    let once = step_five(mixed, convert::FALLBACK_US);
    assert_eq!(once, "привет ghbdtn");
    let twice = step_five(&once, convert::FALLBACK_US);
    assert_eq!(
        twice, mixed,
        "a mixed selection now comes back, and it did not before Т-48-3"
    );

    // ---- where it does not, and why ----------------------------------------------------
    //
    // **A character only one of the two layouts can produce, standing next to the image of
    //    another.** `.` is on a key of both layouts, `ю` is on none of the English ones — so in
    //    the English direction `.` becomes `ю` and a `ю` that was already there stays `ю`. Two
    //    different characters have become one, and no second press can tell them apart.
    let maps: Vec<LayoutMap> = [convert::FALLBACK_US, convert::FALLBACK_RUSSIAN]
        .into_iter()
        .map(|id| convert::fallback_map(id).expect("the hardwired map"))
        .collect();

    let collided = selection::recode(".ю", &maps[0], &maps[1]);
    assert_eq!(collided.text(), "юю", "two characters became one");

    let back = selection::recode(collided.text(), &maps[1], &maps[0]);
    assert_eq!(back.text(), "..", "and the pair cannot be told apart again");
    assert_ne!(back.text(), ".ю");

    // 3. **A character neither layout produces** is not a counter-example: it is carried over
    //    both times and comes back untouched.
    let dash = step_five("gh—bdtn", convert::FALLBACK_US);
    assert_eq!(dash, "пр—ивет");
    assert_eq!(step_five(&dash, convert::FALLBACK_US), "gh—bdtn");
}

// ---------------------------------------------------------------------------------------
// Acceptance points 16в, 17, 18 — the branch, and what it does not do
// ---------------------------------------------------------------------------------------

/// **Acceptance point 17 — FR-65.** With `[selection] enabled = false` the branch answers "not
/// mine" without opening, reading or writing the clipboard, and without sending `Ctrl+C`.
#[test]
fn a_disabled_selection_path_does_not_touch_the_clipboard_even_once() {
    let _serialised = serialised();

    let before = selection::counters();
    let path_before = selection::path_counters();

    let off = settings::Selection {
        enabled: false,
        ..settings::Selection::default()
    };
    selection::publish(&off);

    assert!(!selection::path_enabled(), "FR-65: the path is off");

    // The branch of FR-60, called exactly as `app::window_proc` calls it.
    assert!(
        !selection::wants_selection_path(),
        "a disabled path never takes a press"
    );

    let after = selection::counters();

    // ⚠ The whole of acceptance point 17: not one clipboard access of any kind.
    assert_eq!(after.opens, before.opens, "the clipboard was not opened");
    assert_eq!(after.closes, before.closes);
    assert_eq!(after.open_retries, before.open_retries);
    assert_eq!(after.wrong_thread_refusals, before.wrong_thread_refusals);

    // And nothing was handed over, so nothing on the UI thread could have sent `Ctrl+C` either.
    let path_after = selection::path_counters();
    assert_eq!(path_after.handovers, path_before.handovers);

    // The refusal is not even counted: a switched-off feature is not a failure.
    assert_eq!(path_after.refusals, path_before.refusals);

    // Put the default of section 7 back for whatever runs next.
    selection::publish(&settings::Selection::default());
    assert!(selection::path_enabled());
}

/// **Acceptance point 17, the structural half.** FR-65 is decided before anything else in the
/// branch, so no later condition can let a disabled path reach the clipboard.
#[test]
fn fr65_is_the_first_thing_the_branch_asks() {
    let source = source_of("selection.rs");
    let body = source
        .split("pub fn wants_selection_path()")
        .nth(1)
        .expect("the branch exists");

    let enabled = body.find("path_enabled()").expect("FR-65 is asked");

    for later in [
        "password_field()",
        "plan_for_press()",
        "publish_pending(",
        "post_to_ui_thread(",
    ] {
        let position = body
            .find(later)
            .unwrap_or_else(|| panic!("{later} is asked"));

        assert!(enabled < position, "FR-65 must be decided before {later}");
    }
}

/// **Acceptance point 18 — SEC-06, FR-70.** The selection path does not start in a password
/// field.
#[test]
fn the_selection_path_does_not_start_in_a_password_field() {
    let source = source_of("selection.rs");
    let body = source
        .split("pub fn wants_selection_path()")
        .nth(1)
        .expect("the branch exists");

    // The flag module `guard` publishes from the watcher thread (FR-71), read on the input
    // thread as one atomic — the same way the typing buffer's own gate reads it.
    assert!(
        body.contains("crate::guard::password_field()"),
        "the branch asks module guard whether the focus is in a password field"
    );

    // And it asks before it hands anything over.
    let asked = body
        .find("password_field()")
        .expect("the question is asked");
    let handed = body
        .find("post_to_ui_thread(")
        .expect("the handover exists");

    assert!(asked < handed, "SEC-06 is decided before the handover");
}

// ---------------------------------------------------------------------------------------
// Р-62 — a non-empty typing buffer takes the press to the backspace path, not the probe
// ---------------------------------------------------------------------------------------

/// **Decision Р-62, task T-10-0d.** A non-empty typing buffer proves no selection gesture has
/// happened since the last stroke (FR-10), so FR-60 resolves to the typing-buffer path and the
/// `Ctrl+C` probe of FR-61 must not run. The branch answers `false` — and, because that is the
/// ordinary "convert what I typed" case and not a failure, it neither opens the clipboard nor
/// counts a refusal.
#[test]
fn a_non_empty_buffer_takes_the_backspace_path_without_the_probe() {
    use lang_switcher::buffer;
    use lang_switcher::hook::{Edge, KeyEvent};

    let _serialised = serialised();

    // The path is on, so FR-65 cannot be the reason for the `false` below — the buffer is.
    selection::publish(&settings::Selection::default());
    assert!(selection::path_enabled());

    // A buffer on this thread with one real stroke in it. `record` on an ordinary key stores a
    // stroke even with no cache published (its characters are simply empty), which is all this
    // test needs: `is_empty()` must read `false`.
    buffer::install(&settings::Buffer::default());
    let stored = buffer::record(KeyEvent {
        vk: 0x41,
        edge: Edge::Down,
        extra_info: 0x00CA_FE01,
        scan: 0x1E,
        flags: 0,
        time: 0,
    });
    assert_eq!(stored, buffer::Recorded::Stored);
    assert!(!buffer::is_empty(), "the buffer holds a stroke");

    let clip_before = selection::counters();
    let path_before = selection::path_counters();

    // The branch of FR-60, called exactly as `app::window_proc` calls it — on the input thread,
    // which is the thread this test is standing in for by installing the buffer.
    assert!(
        !selection::wants_selection_path(),
        "Р-62: a non-empty buffer means no selection, so the press takes the backspace path"
    );

    let clip_after = selection::counters();
    let path_after = selection::path_counters();

    // ⚠ The whole of criterion 9: not one clipboard access, so not one `Ctrl+C` probe.
    assert_eq!(
        clip_after.opens, clip_before.opens,
        "the clipboard was not opened"
    );
    assert_eq!(clip_after.closes, clip_before.closes);
    assert_eq!(clip_after.open_retries, clip_before.open_retries);

    // Nothing handed over, and — unlike a password field or a missing cache — nothing counted as
    // a refusal: converting typed text is the feature working, not the selection path failing.
    assert_eq!(path_after.handovers, path_before.handovers);
    assert_eq!(path_after.refusals, path_before.refusals);

    // Leave the thread as clean as an ordinary input thread would be between presses.
    buffer::uninstall();
}

/// **Р-62, the separating control.** With the buffer *empty* — the only state a real selection
/// can share — the branch does **not** stop at the Р-62 check: it goes on to the reasons that
/// existed before this task, and a thread with no mapping cache reaches the plan refusal. This
/// is what proves position 15's selection path is left reachable and unchanged.
#[test]
fn an_empty_buffer_still_reaches_the_reasons_that_came_before_p62() {
    use lang_switcher::buffer;

    let _serialised = serialised();

    selection::publish(&settings::Selection::default());
    assert!(selection::path_enabled());

    // An empty buffer on this thread: `is_empty()` is `true`, so the Р-62 check falls through.
    buffer::install(&settings::Buffer::default());
    assert!(buffer::is_empty(), "the buffer is empty");

    let path_before = selection::path_counters();

    // No cache was published into this buffer, so `plan_for_press` finds nothing and the branch
    // answers `false` through the *plan* refusal — the pre-Р-62 reason, reached only because the
    // empty buffer did not short-circuit first.
    assert!(
        !selection::wants_selection_path(),
        "an empty buffer with no cache falls back through the plan refusal, as before"
    );

    let path_after = selection::path_counters();
    assert_eq!(
        path_after.refusals,
        path_before.refusals + 1,
        "the empty-buffer path reaches and counts the plan refusal — Р-62 did not intercept it"
    );

    buffer::uninstall();
}

/// **Р-62, the structural half.** FR-65 is still decided first; the emptiness of the buffer is
/// decided next, and before the password check, the plan and the hand-over — so the `Ctrl+C`
/// probe cannot be reached while the buffer holds text.
#[test]
fn p62_is_asked_after_fr65_and_before_the_probe_can_run() {
    let source = source_of("selection.rs");
    let body = source
        .split("pub fn wants_selection_path()")
        .nth(1)
        .expect("the branch exists");

    let fr65 = body.find("path_enabled()").expect("FR-65 is asked");
    let p62 = body
        .find("crate::buffer::is_empty()")
        .expect("Р-62 reads the buffer length");

    assert!(fr65 < p62, "FR-65 stays the first question");

    for later in ["password_field()", "plan_for_press()", "post_to_ui_thread("] {
        let position = body
            .find(later)
            .unwrap_or_else(|| panic!("{later} is asked"));
        assert!(
            p62 < position,
            "Р-62 must be decided before {later}, so a non-empty buffer never reaches the probe"
        );
    }
}

// ---------------------------------------------------------------------------------------
// The note to §4.7 — decision П-3: a console never gets the probe (audit of 2026-08-24)
// ---------------------------------------------------------------------------------------

/// **The note to §4.7, decision П-3 — the sixth reason, and its own count.**
///
/// Р-62 closes only half of the harm its own comment names: `Enter` flushes the typing buffer
/// completely by FR-10, so in a console the buffer is empty almost always and the `Ctrl+C` of
/// step 2 of FR-61 goes out as a real interrupt. The note answers it — in the console classes of
/// FR-42а the selection path does not run — and the refusal is counted **apart** from the
/// others, because П-3 working and the path failing are not the same event.
///
/// Driven by class name and not by a live window, the split
/// `inject::resolve_auto`/`inject::window_class` exists for.
#[test]
fn a_console_class_refuses_the_selection_path_by_the_sixth_reason() {
    let _serialised = serialised();

    // The list is not repeated here either: it is the one `inject` declares for FR-42а.
    for class in inject::CONSOLE_WINDOW_CLASSES {
        let before = selection::path_counters();

        assert!(
            selection::console_refuses_selection(Some(class)),
            "{class} is a console of FR-42а: the selection path does not run there"
        );

        let after = selection::path_counters();

        assert_eq!(
            after.console_refusals,
            before.console_refusals + 1,
            "the sixth reason is counted once for {class}"
        );

        // ⚠ And into a counter of its own. A missing cache, a password field and a missing UI
        // window are the path *failing*; this one is the path doing what the note prescribes,
        // and an acceptance that could not tell them apart could not tell П-3 from a regression.
        assert_eq!(
            after.refusals, before.refusals,
            "the console refusal is not poured into the common counter"
        );
        assert_eq!(after.handovers, before.handovers, "nothing was handed over");
    }
}

/// **П-3, the case of the name.** Windows compares window class names without regard to case, so
/// the rule does too — the same comparison `inject::resolve_auto` makes for FR-42а.
#[test]
fn the_case_of_a_console_class_name_does_not_matter() {
    let _serialised = serialised();

    for class in [
        "consolewindowclass",
        "CONSOLEWINDOWCLASS",
        "ConSoleWindowCLASS",
        "cascadia_hosting_window_class",
        "Cascadia_Hosting_Window_Class",
    ] {
        let before = selection::path_counters();

        assert!(
            selection::console_refuses_selection(Some(class)),
            "{class} names a console whatever the case"
        );

        let after = selection::path_counters();
        assert_eq!(
            after.console_refusals,
            before.console_refusals + 1,
            "and it is counted like any other spelling of it"
        );
    }
}

/// **П-3, the separating control — an ordinary window is left exactly as it was.**
///
/// Nothing but the two measured console classes refuses, the comparison is of the whole name
/// rather than of a prefix, and an unreadable class (`None`) answers "not a console" — NFR-13,
/// the direction the function documents: a window that cannot be named is going away, and a
/// `Ctrl+C` at a desktop switch has no console session to interrupt.
#[test]
fn an_ordinary_window_class_leaves_the_sixth_reason_alone() {
    let _serialised = serialised();

    let before = selection::path_counters();

    for class in [
        Some("Notepad"),
        Some("Chrome_WidgetWin_1"),
        Some("Edit"),
        Some("TelegramDesktop"),
        // Whole name, not a prefix and not a suffix.
        Some("ConsoleWindowClas"),
        Some("ConsoleWindowClassic"),
        Some("XConsoleWindowClass"),
        // An empty string is a class no registered window can have.
        Some(""),
        // No foreground window at all, or `GetClassNameW` refused — NFR-13.
        None,
    ] {
        assert!(
            !selection::console_refuses_selection(class),
            "{class:?} is not one of the consoles of FR-42а: the path runs as it always did"
        );
    }

    let after = selection::path_counters();

    assert_eq!(
        after.console_refusals, before.console_refusals,
        "the counter of the sixth reason does not move for an ordinary window"
    );
    assert_eq!(after.refusals, before.refusals);
    assert_eq!(after.handovers, before.handovers);
}

/// **П-3, the structural half — where the sixth reason stands in the branch, and what it costs.**
///
/// Two statements at once, and both are prices:
///
/// * it is asked **after** the cheap reasons, so the ordinary press — the one with a non-empty
///   typing buffer — still leaves `wants_selection_path` without a single system call, which is
///   what the comment on the Р-62 read promises;
/// * it is asked **before** the plan is published and before the hand-over, so a console press
///   never leaves a plan behind for a later message and never reaches the probe of FR-61.
#[test]
fn the_console_reason_is_asked_after_the_cheap_ones_and_before_the_handover() {
    let source = source_of("selection.rs");
    let body = source
        .split("pub fn wants_selection_path()")
        .nth(1)
        .expect("the branch exists");

    let console = body
        .find("console_refuses_selection(")
        .expect("the sixth reason is asked");
    let p62 = body
        .find("crate::buffer::is_empty()")
        .expect("Р-62 reads the buffer length");
    let plan = body.find("plan_for_press()").expect("the plan is built");

    assert!(
        p62 < console,
        "the non-empty buffer answers before the class is read: no system call on the usual press"
    );
    assert!(
        plan < console,
        "the console count means presses taken away from a path that would otherwise have run"
    );

    for later in ["publish_pending(", "post_to_ui_thread("] {
        let position = body
            .find(later)
            .unwrap_or_else(|| panic!("{later} is reached"));

        assert!(
            console < position,
            "a console must be answered before {later}: no plan left behind, no probe sent"
        );
    }

    // And the answer to a console is `false` — the path does not run — rather than anything else.
    let from_the_question = &body[console..];
    let refusal = from_the_question
        .find("return false")
        .expect("the console answer is a refusal");
    let publish = from_the_question
        .find("publish_pending(")
        .expect("the hand-over follows");

    assert!(
        refusal < publish,
        "the sixth reason answers `false` before anything is published"
    );
}

/// **Criterion 7 of the task, as a test rather than as a `grep` run once.**
///
/// The console classes of FR-42а are declared in `src\` exactly once, and the selection path
/// reaches that declaration instead of repeating the two names. `GetClassNameW` is not written a
/// second time either: the call, its buffer and the NFR-13 examination of its zero return live in
/// `inject::window_class`, which this path reuses.
#[test]
fn the_console_classes_and_the_class_read_are_each_written_once() {
    let mut declarations = Vec::new();

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(&src).expect("src\\ must be readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }

        let text = read_normalised(&path);
        for (number, line) in code_lines_with(&text, "CONSOLE_WINDOW_CLASSES") {
            if line.contains("const CONSOLE_WINDOW_CLASSES") {
                declarations.push(format!("{}:{number}", path.display()));
            }
        }
    }

    assert_eq!(
        declarations.len(),
        1,
        "the consoles of FR-42а are named once in src\\: {declarations:?}"
    );

    let source = source_of("selection.rs");

    assert!(
        !code_lines_with(&source, "crate::inject::CONSOLE_WINDOW_CLASSES").is_empty(),
        "the selection path reaches the one declaration"
    );
    for copied in [
        "\"ConsoleWindowClass\"",
        "\"CASCADIA_HOSTING_WINDOW_CLASS\"",
    ] {
        assert!(
            code_lines_with(&source, copied).is_empty(),
            "{copied} must not be spelled a second time in selection.rs"
        );
    }

    assert!(
        code_lines_with(&source, "GetClassNameW").is_empty(),
        "the class is read through inject::window_class, not by a second GetClassNameW"
    );
    assert!(
        !code_lines_with(&source, "crate::inject::window_class(").is_empty(),
        "and it really is that function the Win32 half calls"
    );
}

/// **Acceptance point 16в.** The clipboard is never touched by the thread that owns the hook —
/// the branch on that thread posts a message and does nothing else.
///
/// The primitives refusing that thread is T-07-1's guarantee and is checked by
/// [`nothing_that_can_block_runs_on_the_thread_that_owns_the_hook`]. What is checked here is
/// this task's half: that the path built on top of them **hands the work over** rather than
/// running it where the press arrives.
#[test]
fn the_press_is_handed_to_the_ui_thread_and_the_clipboard_is_not_touched_where_it_arrives() {
    let source = source_of("selection.rs");
    let product = cut_at(&source, "mod tests {", "the product half of the module");

    // The branch the input thread runs contains no step of FR-61: it publishes a plan and posts.
    let remainder = product
        .split("pub fn wants_selection_path()")
        .nth(1)
        .expect("the branch exists");

    // ⚠ **Through [`cut_at`], and it is worth being exact about what was wrong here — T-13-30.**
    //
    // This used to end in `.split("\n/// ").next().unwrap_or_default()`. Unlike the `"\n}\n"`
    // cuts of `body_after`, the boundary was **not** lost on the canonical tree: `\r\n/// ` does
    // contain `\n/// `, so `branch` really was the branch — 3582 bytes on this slice against 3512
    // after a `cargo fmt`. Nothing was being asserted about the wrong text.
    //
    // What was wrong is the fall-back itself, and it is not cosmetic: it is the sentence «if the
    // boundary is lost, sweep on regardless» standing where the reader expects a guarantee — the
    // very instruction that cost `body_after` its meaning. One changed needle, one doc comment
    // moved to a different column, and this becomes the same defect.
    let branch = cut_at(remainder, "\n/// ", "the branch of wants_selection_path");

    for forbidden in [
        "snapshot(",
        "read_unicode_text(",
        "write_unicode_text(",
        "wait_for_change(",
    ] {
        assert!(
            !branch.contains(forbidden),
            "the input thread's branch must not call {forbidden}"
        );
    }

    assert!(
        branch.contains("crate::app::post_to_ui_thread(WM_APP_SELECTION)"),
        "the branch hands the work to the UI thread"
    );

    // And the far end is confined to the window the listener was registered on, which is the UI
    // thread's — the same test `handle_clipboard_message` makes.
    let far_end = product
        .split("pub fn handle_selection_message(")
        .nth(1)
        .expect("the far end exists");

    assert!(
        far_end.contains("LISTENER_WINDOW.load(Ordering::Acquire)"),
        "the eight steps run at the registered window and nowhere else"
    );

    // The two messages are distinct numbers, and neither collides with anything already taken.
    assert_ne!(selection::WM_APP_SELECTION, selection::WM_APP_BUFFER_PATH);
    for taken in [
        lang_switcher::hook::WM_APP_HOTKEY,
        lang_switcher::switch::WM_APP_SWITCH,
        lang_switcher::guard::WM_APP_PROBE,
        lang_switcher::guard::WM_APP_FIELD,
        lang_switcher::watchdog::WM_APP_FLUSH,
        lang_switcher::watchdog::WM_APP_LAYOUT,
        lang_switcher::watchdog::WM_APP_REHOOK,
    ] {
        assert_ne!(selection::WM_APP_SELECTION, taken);
        assert_ne!(selection::WM_APP_BUFFER_PATH, taken);
    }
}

/// **Acceptance point 16г — SEC-07.** The clipboard event has a name in the journal's closed
/// vocabulary, and prints under it rather than as `unlisted`.
#[test]
fn the_clipboard_event_has_a_name_in_the_journal() {
    use lang_switcher::diag::{Kind, Operation};

    let operation = Operation::from_name("clipboard snapshot truncated");

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "the row T-07-1 asked for is in the table"
    );
    assert_eq!(operation.name(), "clipboard snapshot truncated");
    assert_eq!(operation.kind(), Kind::Selection);
    assert_eq!(operation.kind().name(), "selection");

    // And nothing of the clipboard can be named: an operation built from content is `unlisted`
    // and the content is dropped unread — the mechanism SEC-07 rests on, unchanged.
    assert_eq!(Operation::from_name("ghbdtn"), Operation::UNLISTED);
}

/// **Acceptance point 19 — SEC-01, SEC-07.** Nothing of the selection reaches the journal, a
/// message or a panic.
#[test]
fn nothing_of_the_selection_can_reach_the_journal_or_a_panic() {
    let source = source_of("selection.rs");
    let product = cut_at(&source, "mod tests {", "the product half of the module");

    // The sweep of T-07-1 still holds for the whole module, the selection path included.
    for forbidden in [
        "format!(",
        "println!",
        "panic!(",
        ".unwrap()",
        ".expect(",
        "unreachable!(",
        "todo!(",
    ] {
        let hits = code_lines_with(product, forbidden);

        assert!(hits.is_empty(), "the module uses {forbidden}: {hits:?}");
    }

    // The recoded text is content, so its `Debug` is hand-written and prints counts.
    let maps: Vec<LayoutMap> = [convert::FALLBACK_US, convert::FALLBACK_RUSSIAN]
        .into_iter()
        .map(|id| convert::fallback_map(id).expect("the hardwired map"))
        .collect();

    let recoded = selection::recode("ghbdtn", &maps[0], &maps[1]);
    let printed = format!("{recoded:?}");

    assert!(!printed.contains("привет"), "the Debug prints no content");
    assert!(!printed.contains("ghbdtn"));
    assert!(printed.contains("chars"), "it prints counts: {printed}");

    // And the outcome that crosses into `app` carries two counts and nothing else.
    let outcome = Outcome::Converted {
        mapped: 6,
        carried: 0,
    };
    let printed = format!("{outcome:?}");
    assert!(!printed.contains("привет") && !printed.contains("ghbdtn"));
}

// ---------------------------------------------------------------------------------------
// SEC-01, SEC-02 — how the working copies of the user's text are released. Task T-13-15.
// ---------------------------------------------------------------------------------------

/// The body of the top-level item `signature` opens, up to the brace in the first column.
///
/// The same reading `tests\hook.rs` uses for the callback: a top-level function closes with `}`
/// at column zero, and everything before that belongs to it. `source` must have come through
/// [`read_normalised`] — the needle carries a newline, and on the CRLF working tree the raw text
/// holds `\r\n}\r\n`.
///
/// ⚠ **No silent fall-back — task T-13-30.**
///
/// This used to end in `.split("\n}\n").next().unwrap_or_else(|| panic!(…))`. `.next()` on a
/// `Split` that has just been handed a non-empty string **never** answers `None`, so the panic
/// was unreachable and the sentence in it was never said; and with the source read raw off a CRLF
/// checkout the needle did not occur at all, so "the first piece" was **the whole rest of
/// `src\selection.rs`** — 96 939 bytes after `fn decode_utf16`, where its body is 310. Both
/// callers were then satisfied by text belonging to other functions, which was measured rather
/// than supposed: moving the zeroing out of `decode_utf16` into a helper beside it passes the old
/// form of `the_three_transit_buffers_of_the_clipboard_path_are_zeroed_as_well` and fails this
/// one.
///
/// A boundary that cannot be found is a broken cut, and a broken cut must fail — the refusal and
/// the two degeneracy checks live in [`cut_at`], which every region of this file now goes through.
fn body_after(source: &str, signature: &str) -> String {
    let remainder = source
        .split(signature)
        .nth(1)
        .unwrap_or_else(|| panic!("{signature} exists in the module"));

    cut_at(remainder, "\n}\n", signature).to_string()
}

/// **The audit of 2026-08-24, the middle finding: `fill` is not a zeroing that survives.**
///
/// Every place in the module that overwrites a copy of the user's text before releasing it goes
/// through `buffer::zero_slice` — the `write_volatile` plus `compiler_fence` the ring is zeroed
/// with — and no place does it with `slice.fill(0)`, which the documentation of module `buffer`
/// calls a dead store the compiler may delete.
///
/// The one `.fill(` left in the module is `Block::fill`, and it is not a zeroing at all: it
/// copies the caller's bytes **into** a freshly allocated clipboard block. The name collides and
/// nothing else does, so it is named here rather than being allowed to fail the sweep later.
#[test]
fn every_working_copy_of_the_text_is_released_through_the_volatile_zeroing() {
    let source = source_of("selection.rs");

    let fills = code_lines_with(&source, ".fill(");

    assert_eq!(
        fills.len(),
        1,
        "the only `.fill(` left is the one that fills a clipboard block with data: {fills:?}"
    );
    assert!(
        fills[0].1.contains("block.fill(bytes)"),
        "and that one is Block::fill: {:?}",
        fills[0]
    );

    // Seven buffers, seven calls: the three the finding named, the three that were not zeroed
    // at all, and the working buffer of `recode_words` — the second recoder, added by task
    // Т-48-3 when step 5 began deciding the script per word. The count is asserted so that an
    // eighth copy of the user's text added later cannot quietly arrive without one; it went
    // from six to seven the moment a seventh copy existed, which is the whole point of it being
    // a number.
    let zeroed = code_lines_with(&source, "buffer::zero_slice(");

    assert_eq!(
        zeroed.len(),
        7,
        "every working copy is zeroed through the helper: {zeroed:?}"
    );
}

/// **The audit of 2026-08-24, the low finding: three transit buffers were not zeroed at all.**
///
/// `read_unicode_text` released the block it read off the clipboard, `decode_utf16` released the
/// code units it decoded out of that block, and `write_unicode_text` released the encoded block
/// it had just put on the clipboard — three copies of the user's text handed back to the
/// allocator as they were, in a module whose own header promises the opposite. Each of the three
/// now zeroes its buffer, and the third does it on the failure paths as well, which is what the
/// closure inside it is for.
#[test]
fn the_three_transit_buffers_of_the_clipboard_path_are_zeroed_as_well() {
    let source = source_of("selection.rs");

    for (signature, what) in [
        (
            "pub fn read_unicode_text",
            "the block read off the clipboard",
        ),
        ("fn decode_utf16", "the code units decoded out of it"),
        ("pub fn write_unicode_text", "the block written back to it"),
    ] {
        let body = body_after(&source, signature);

        assert!(
            body.contains("buffer::zero_slice("),
            "{signature} zeroes {what}"
        );
    }

    // The write is the one with paths that used to leave through a `?`. The zeroing stands
    // **after** the fallible work and **before** the result is unwrapped, so no early return can
    // step over it.
    let body = body_after(&source, "pub fn write_unicode_text");
    let zeroing = body.find("buffer::zero_slice(").expect("the zeroing");
    let unwrap = body.find("outcome?").expect("the result is unwrapped");

    assert!(
        zeroing < unwrap,
        "the block is zeroed before the failure is propagated"
    );
}

/// **И-3 of Э18 — `empty` succeeded, `put` refused, and the clipboard is empty because of us.**
///
/// The worst shape of the defect the audit of 2026-08-31 found, and the only one that ends with
/// the user holding **nothing at all**: `EmptyClipboard` moves the window station's counter by
/// itself, so a refused `SetClipboardData` leaves a clipboard this program emptied and did not
/// fill. The mark of FR-63 used to stand after the `?` that propagates that failure, so the
/// change went unmarked, the gate of П-5 read it as a stranger's, and step 8 — the one restore
/// that could have undone it — was skipped.
///
/// A sweep rather than a run, and deliberately: making `SetClipboardData` refuse while
/// `EmptyClipboard` succeeds needs a fault injected below the seam [`SelectionPath`] provides,
/// and there is no honest way to reach it from here. What can be checked exactly is the shape
/// the fix has — the mark hangs on **having emptied**, and it is put **before** the failure
/// leaves the function.
///
/// ⚠ The needle for the mark carries the argument task **T-19-3** gave `note_own_write`: the
/// number is now read under the open clipboard and handed in, so the call reads
/// `note_own_write(sequence)`. What this sweep is about did not change — only the spelling of
/// the line it looks for, and the guard that carries the mark now names the number as well. See
/// `the_number_of_our_own_write_is_taken_the_instant_the_clipboard_is_released`.
#[test]
fn the_write_marks_the_clipboard_as_ours_whenever_it_emptied_it() {
    let source = source_of("selection.rs");
    let body = body_after(&source, "pub fn write_unicode_text");

    let emptied = body
        .find("emptied = true;")
        .expect("the fact that the clipboard was emptied is remembered");
    let guard = body
        .find("if emptied")
        .expect("and the mark of FR-63 is hung on it");
    let mark = body
        .find("note_own_write(sequence);")
        .expect("the write is still marked as ours — FR-63");
    let unwrap = body
        .find("outcome?")
        .expect("the failure is still propagated");

    assert!(
        emptied < guard && guard < mark,
        "the mark follows the change, not the success"
    );
    assert!(
        mark < unwrap,
        "И-3: the mark is put before the failure leaves the function, or a refused `put` \
         after a successful `empty` costs the user their clipboard for good"
    );
}

/// **Э18-Д-1, finding 6 of the audit of 2026-08-31 — the number of our own write is taken the
/// instant the clipboard is released, and thrown away if it is not ours any more.**
///
/// `note_own_write` used to read `GetClipboardSequenceNumber` **inside itself**, and it is called
/// after the guard is gone — in `write_unicode_text`, after the O(n) zeroing of a block that can
/// be megabytes. Anything that changed the clipboard in that window — a clipboard manager
/// answering `WM_CLIPBOARDUPDATE`, a `Ctrl+C` of the user's — put **its** number into the ring of
/// FR-63. Step 8 then read that stranger's change as ours and overwrote it with the snapshot,
/// which is precisely what decision П-5 forbids.
///
/// ⚠ **The audit asked for the number to be read under the open clipboard, and that is
/// impossible.** Measured on the machine's own clipboard: `after_open=1025 after_empty=1026
/// after_put=1027 after_close=1030` — the counter finishes moving on `CloseClipboard`, where the
/// station synthesises `CF_TEXT`, `CF_OEMTEXT` and `CF_LOCALE`. A number read under the open
/// clipboard is therefore guaranteed to be the wrong one, and taking it there turned
/// `our_own_change_is_recognised_and_a_foreign_one_is_not` red on the spot. `selection::snapshot`
/// is no counter-example: it only reads, so it moves nothing and its baseline is exact.
///
/// A sweep rather than a run: the window is now a single instruction wide and no test can be made
/// to land inside it on purpose. What can be checked exactly is the shape — the marking function
/// is **handed** a number instead of fetching one, the fetch is the statement immediately after
/// the release, and the owner is asked after the number rather than before it.
#[test]
fn the_number_of_our_own_write_is_taken_the_instant_the_clipboard_is_released() {
    let source = source_of("selection.rs");

    assert_eq!(
        code_lines_with(&source, "pub fn note_own_write(sequence: u32)").len(),
        1,
        "the mark of FR-63 is handed the number it stores; a function that fetched one itself \
         could only ever fetch it too late"
    );

    // Both orders compile and only one of them is safe, so the order is pinned here.
    let helper = body_after(&source, "fn own_change_number(owner: HWND) -> Option<u32>");
    let number = helper
        .find("let sequence = sequence_number();")
        .expect("the helper reads the number");
    let asked = helper
        .find("unsafe { GetClipboardOwner() }")
        .expect("the helper asks who owns the clipboard");

    assert!(
        number < asked,
        "the number is read before the owner is asked, or a stranger who wrote between the two \
         would have their number stored as ours — П-5"
    );

    for signature in ["pub fn write_unicode_text", "pub fn restore("] {
        let body = body_after(&source, signature);

        let close = code_lines_with(&body, "drop(clipboard)");
        let read = code_lines_with(&body, "own_change_number(owner)");

        assert_eq!(
            close.len(),
            1,
            "{signature} releases the clipboard exactly once"
        );
        assert_eq!(
            read.len(),
            1,
            "{signature} takes the number of its own change exactly once"
        );
        assert_eq!(
            read[0].0,
            close[0].0 + 1,
            "{signature} must take the number on the very next line after the release — it is \
             line {} against line {} — П-5, Э18-Д-1",
            read[0].0,
            close[0].0
        );
    }
}

// ---------------------------------------------------------------------------------------
// The ceiling of step 4 — finding №7 of the audit of 2026-08-31, decision 77
// ---------------------------------------------------------------------------------------

/// **The ceiling of step 4 is the ceiling of FR-64, and it is one number and not two.**
///
/// The snapshot of FR-64 has a budget; the read of step 4 had none, so a selection of any size at
/// all was copied out of the clipboard, decoded into a second copy and recoded into a third. The
/// decision of question 77 is symmetry: the same four megabytes, and the constant shared rather
/// than spelt again — «не два разных 4 МБ в двух местах».
///
/// The boundaries are asserted on both sides of the ceiling so that a mutation of the comparison
/// (`>` against `>=`, or a ceiling of its own) fails here rather than passing quietly: exactly the
/// budget is allowed through, one byte more is not.
#[test]
fn the_ceiling_of_step_four_is_the_four_megabytes_of_fr64_and_not_a_second_number() {
    assert_eq!(SNAPSHOT_BUDGET_BYTES, 4 * 1024 * 1024);

    assert!(
        !selection::read_refuses_size(0),
        "an empty block is not oversized"
    );
    assert!(
        !selection::read_refuses_size(SNAPSHOT_BUDGET_BYTES - 1),
        "one byte under the budget is read"
    );
    assert!(
        !selection::read_refuses_size(SNAPSHOT_BUDGET_BYTES),
        "exactly the budget is read — the ceiling is a ceiling, not a wall one byte short of it"
    );
    assert!(
        selection::read_refuses_size(SNAPSHOT_BUDGET_BYTES + 1),
        "one byte over the budget is refused"
    );
    assert!(
        selection::read_refuses_size(usize::MAX),
        "and so is anything above it"
    );
}

/// **The ceiling is asked before the copy, and a block over it behaves as «there is no text».**
///
/// Two halves, and they are asserted apart because they can fail apart.
///
/// The first is the one the finding is about: `GlobalSize` reads a header and copies nothing, so
/// the size is known **before** `read_block` allocates a vector of it. A ceiling asked after the
/// copy would refuse the text and still have made the copy the refusal exists to avoid — the
/// sweep therefore pins the order of the two lines rather than merely their presence.
///
/// The second is the coupling to step 8 of Э18: over the ceiling the read answers `Ok(None)`, and
/// `Ok(None)` is what step 4 already turns into [`Refusal::NoText`] — on which the snapshot goes
/// back, because `Owed::StepEight` was armed the moment the probe changed the clipboard. That
/// half works today and is asserted so that it cannot stop working under a path that now reaches
/// it for a second reason.
#[test]
fn an_oversized_block_is_refused_before_the_copy_and_lands_on_no_text_with_the_snapshot_back() {
    let source = source_of("selection.rs");

    let body = body_after(&source, "pub fn read_unicode_text");

    let sized = code_lines_with(&body, "GlobalSize(handle)");
    let refused = code_lines_with(&body, "read_refuses_size(");
    let copied = code_lines_with(&body, "read_block(handle");

    assert_eq!(sized.len(), 1, "the size is taken once: {sized:?}");
    assert_eq!(
        refused.len(),
        1,
        "the ceiling is asked exactly once: {refused:?}"
    );
    assert_eq!(copied.len(), 1, "the block is copied once: {copied:?}");

    assert!(
        sized[0].0 < refused[0].0,
        "the ceiling is asked after the size is known — line {} against line {}",
        refused[0].0,
        sized[0].0
    );
    assert!(
        refused[0].0 < copied[0].0,
        "⚠ the ceiling is asked BEFORE the copy — it is line {} against line {}, so the copy the \
         refusal exists to avoid is made anyway",
        refused[0].0,
        copied[0].0
    );

    // SEC-01, SEC-07: what the journal is told is a row of the closed table and carries no size,
    // no format and no byte of anybody's clipboard.
    let operation = Operation::from_name("clipboard read oversized");

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "the journal line of the refusal is a named row, not an unlisted code"
    );
    assert_eq!(operation.kind(), Kind::Selection);

    // The other half: `Ok(None)` out of step 4 is `NoText`, and the snapshot comes back.
    let mut bench = Bench::with_selection("");
    let outcome = selection::run(&mut bench, &pair_plan(convert::FALLBACK_US));

    assert_eq!(outcome, Outcome::Refused(Refusal::NoText));
    assert!(
        bench.ran(Step::RestoreClipboard),
        "the snapshot goes back on the refusal path of Э18: {:?}",
        bench.steps
    );
}
