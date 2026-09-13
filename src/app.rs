//! Process lifecycle: the single instance, the three threads of section 6.1 of SPEC, the
//! hidden window and message loop each of them owns, and ordered shutdown.
//!
//! This module is absent from the module table of section 6.2 of SPEC and is here by
//! decision R-18: that table distributes *product* responsibility — the hook, the buffer,
//! the layouts, the tray — and carries no process-level entry, while the single instance
//! (FR-82), the three threads, their message loops and their shutdown have to live
//! somewhere. Spreading them across the existing modules would break the responsibility of
//! each: `watchdog` restores a hook, it does not start a process.
//!
//! Requirements this module covers: FR-82 (single instance), FR-97 (debug build timeout),
//! FR-98 (panic hook) — implemented here by task T-01-2 and completed by task T-03-1, which
//! added the removal of the keyboard hook to both and rewrote the timeout so that it works
//! while the UI thread is blocked.
//! Task **T-03-2a** added the assembly of the input path, which is a matter of *calling* what
//! the other modules already built and of calling it from the one thread section 6.1 allows:
//! FR-07 (the buffer is installed on the input thread, at the capacity of section 7), FR-20
//! (the layout cache is built there), FR-25 (a build that fails falls back to the hardwired
//! table of module `convert` instead of taking the program down), FR-21 (the cache is rebuilt
//! on the two messages `layouts::needs_rebuild` names) and FR-11 (neither of those flushes the
//! buffer, and neither may ever be made to).
//! Task **T-03-3** put the subscriptions of module [`crate::watchdog`] on the threads section
//! 6.1 assigns them to — Raw Input on the input thread (FR-13), `SetWinEventHook` on the watcher
//! thread (two rows of the FR-10 table) — routed their messages through [`window_proc`] into the
//! time-ordered flush of FR-12, and made FR-21 **actually happen**: neither message FR-21 names
//! could reach a program whose windows are hidden and normally not focused, so the rebuild was
//! driven by `WM_INPUT_DEVICE_CHANGE` and by the layout probe of [`refresh_layout_and_cache`]
//! instead. FR-11 holds across all of it and is checked separately.
//! Task **T-08-4** took the keyboard entry out of the Raw Input registration, because a process
//! that holds one loses the whole low-level keyboard hook chain — its own and everybody else's —
//! whenever a window of its own is in the foreground, which is what made FR-96 unreachable with
//! the settings dialog of FR-92 open. The device half of FR-21 moved to
//! `watchdog::register_device_notice`, so `WM_DEVICECHANGE` itself now arrives at the input
//! window and the `layouts::needs_rebuild` branch of [`window_proc`] — written by T-03-2a and
//! never once entered — is what answers it.
//! Task **T-04-1** attached the far end of the FR-02 handoff to [`window_proc`]: a
//! `hook::WM_APP_HOTKEY` taken off the input thread's queue runs steps 3 to 6 of FR-40 through
//! [`crate::inject::on_hotkey`], which is where section 6.1 puts `SendInput` and the only place
//! FR-40 allows it. That call is the whole of this module's part in the replacement; everything
//! about the packet, the modifiers and the pause of FR-44 belongs to `inject`.
//! Task **T-03-4** moved the state of the `testing`-only `acceptance` module into
//! [`crate::control`], which SEC-04a makes the single source of the acceptance numbers: what is
//! left here is the file sink, unchanged in keys and in format, and the five `note_*` calls
//! this module makes now name `control` instead.
//! Boundary: FR-83 (`WM_QUERYENDSESSION` / `WM_ENDSESSION`, unhooking, removing the tray
//! icon, wiping the buffer, saving the configuration) is handled in [`crate::tray`] by task
//! T-01-4, which attached itself to [`request_shutdown`] and [`shutdown_requested`]; the
//! "release the mutex" action of FR-83 is the one part of it that lives here, in
//! [`SingleInstance`]'s `Drop`, and the "снятие хуков" action is the other, in
//! [`window_proc`], because `src\tray.rs` was outside the file scope of task T-03-1.
//! FR-01's "one hook on the input thread" is installed here, in [`serve_window`], and
//! implemented in [`crate::hook`].
//! Implemented by backlog tasks: T-01-2 (done), T-01-4 (done), T-03-1 (done), T-03-2a (done).
//!
//! # Thread model — section 6.1
//!
//! Three threads, each with a single responsibility and its own message loop:
//!
//! * **input** — owns a message-only window; task T-03-1 put `WH_KEYBOARD_LL` on it, task
//!   T-03-2a added the typing buffer and T-02-1's layout cache, which is rebuilt on this
//!   thread's messages, task T-03-3 added the Raw Input registration of FR-13 and the
//!   time-ordered flush of FR-12 that its `WM_INPUT` feeds, and `SendInput` will run here;
//! * **UI** — owns a hidden top-level window; task T-01-4 attached the tray icon and menu
//!   to it, and they run on this thread and on no other;
//! * **watcher** — owns a message-only window and a COM STA apartment; task T-03-3 put the
//!   `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS` subscriptions here, where section 6.1
//!   puts them, and tasks T-06-1 and T-06-2 add the rest of `SetWinEventHook` and the UI
//!   Automation password-field probe.
//!
//! The split is not decoration. Section 6.1 states outright that a UI thread stalled for a
//! few seconds makes the system drop the low-level hook silently (FR-80), so nothing that
//! can block — a menu, a dialog, file I/O, COM — may share a thread with the hook.
//!
//! Each window is created by the thread that serves it, because a message queue in Windows
//! belongs to the thread that created the window and no other thread can pump it.
//!
//! Two of the three windows are message-only (`HWND_MESSAGE`): such a window takes part
//! neither in top-level window enumeration, nor in the Z order, nor in `Alt+Tab`, and
//! cannot be shown by accident. The UI window cannot be one of them — decision R-20 point
//! 2: `RegisterWindowMessage("TaskbarCreated")` is broadcast, broadcasts do not reach
//! `HWND_MESSAGE` windows, and FR-81 (task T-01-4) needs exactly that message. It is a
//! top-level window that is never shown: `WS_EX_TOOLWINDOW`, no `WS_VISIBLE`.
//!
//! # Shutdown — decision R-20 point 3
//!
//! An `AtomicBool` plus a wake-up `PostMessageW`, and the decision to leave the loop is
//! taken on **our own flag**, never on the arrival of a message. `WM_CLOSE` is suppressed
//! explicitly. Both are SEC-05: processes at the same integrity level can post to our
//! windows, and the default `DefWindowProcW` behaviour for `WM_CLOSE` would hand any of
//! them a way to destroy our window and stop the process.
//!
//! # Synchronisation — section 6.3
//!
//! Nothing here is a mutex, a lock or a channel. The cross-thread state is a handful of
//! atomics — the shutdown flag, three published window handles, the buffer capacity the UI
//! thread publishes (FR-07) and one failure counter — and the only cross-thread call is
//! `PostMessageW`, which does not block. This is deliberate groundwork: NFR-04 forbids
//! blocking primitives on the hook path and section 6.3 forbids mutexes there outright, so
//! the shutdown interface T-03-1 will call from inside hook-adjacent code must not have
//! any.
//!
//! Section 6.3 also fixes the direction the configuration travels — "Конфигурация публикуется
//! потоком UI" — and task T-03-2a follows it for the buffer exactly as task T-03-1 did for the
//! hotkey: the input thread starts on the default of section 7, the UI thread publishes what
//! the file says into an atomic and nudges the input thread with a message, and the input
//! thread resizes its own buffer. The message carries nothing; the atomic carries the value.
//!
//! # SEC-01, SEC-07
//!
//! No keystroke, key code or buffer content passes through this module, and none may be
//! added later: neither [`report_non_critical`] nor the panic hook is given anything but
//! an operation name and an OS error code. The `testing`-only `acceptance` module below
//! writes out the *length* of the buffer, which is the one number SEC-04a allows out, and no
//! more.

use std::ffi::c_void;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT,
    WPARAM,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
#[cfg(debug_assertions)]
use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
// ⭐ Task T-10-20: the three calls FR-52 is made of are gone from this module. They lived here in
// a second copy of `switch::current`, and keeping two copies of one requirement in step by hand
// is what let four repairs of defect E go past both. See [`foreground_layout`].
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE,
    MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MSG, MessageBoxW, PostMessageW, PostQuitMessage,
    RegisterClassExW, UnregisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE,
    WM_ENDSESSION, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_POPUP,
};
// FR-100, task Т-21-5 and the tuning after the acceptance by ear. `PlaySoundW` is exported by
// `winmm.dll`; the `windows` crate puts it under `Win32::Media::Audio` — hence the import
// standing apart from `WindowsAndMessaging` above, and the feature of the same name in
// `Cargo.toml`. It is a feature of a crate that was already there: `cargo tree` is 83 lines
// before and after, and `Cargo.lock` is byte for byte the same file (measured, stage Э21).
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows::core::{Error as WinError, PCWSTR, Result as WinResult, w};

use crate::layouts::{LayoutCache, LayoutError, LayoutId};
use crate::settings;

// ---------------------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------------------

/// Exit code of an instance that found another one already running — FR-82.
///
/// Non-zero and distinct on purpose: the acceptance check of task T-01-2 and the installer
/// at stage E9 both have to tell "a second copy declined to start" apart from any other
/// kind of failure, which is what [`ExitCode::FAILURE`] would look like.
pub const EXIT_ALREADY_RUNNING: u8 = 2;

/// Runs the program. The process lives for exactly as long as this call.
///
/// The order of the first two steps is fixed by decision R-19 and is not an implementation
/// detail: the panic hook (FR-98) is installed before anything is acquired, so there is no
/// window in which a panic could leave something behind.
pub fn run() -> ExitCode {
    // Instrumentation of the acceptance bench, feature `testing`, absent from the Release
    // configuration. First in the body so that the reference point NFR-08 is measured from is
    // the earliest instant this program can observe of itself.
    #[cfg(feature = "testing")]
    crate::control::note_start();

    // The ring journal of section 6.2, task T-06-4. First of the three edits that task makes
    // to this file, and it is here rather than later because everything below reports into it:
    // `report_non_critical` at the bottom of this function already does, and the panic hook
    // installed on the next line runs on paths that do. The ring itself is a `static` and
    // needs no allocating — what this fixes is the zero the timestamps are measured from.
    crate::diag::init();

    install_panic_hook();

    match SingleInstance::acquire() {
        Ok(acquisition) => {
            // ⭐ **Finding Н24, task T-41-5: the name is not the only question asked.** A held
            // name used to end the matter — «кто-то уже работает», and out. Any program of this
            // user could create an object of that name first and make this one refuse to start
            // for good; nothing was written down, and the person saw «программа не
            // запускается» with no cause and no trace. So a second fact is asked for: is there
            // a window of **our own class** anywhere in this session.
            let taken = matches!(acquisition, Acquisition::AlreadyRunning(_));
            let verdict = instance_verdict(taken, taken && a_window_of_ours_is_up());
            let instance = acquisition.into_guard();

            match verdict {
                InstanceVerdict::TheInstance => run_as_first_instance(instance),

                // There is a window of ours: a real second copy, and FR-82 answers exactly as
                // it always has — the notification, unless this is a debug build on a bench
                // (`already_running_is_silent`, task Т-25-2), and the exit code the acceptance
                // of task T-01-2 and the installer both read.
                InstanceVerdict::SecondCopy => {
                    if !already_running_is_silent() {
                        notify_already_running();
                    }

                    flush_the_journal_of_an_early_exit();

                    ExitCode::from(EXIT_ALREADY_RUNNING)
                }

                // The name is held and no window of ours is anywhere: somebody else has the
                // name. This program starts — and says so in the journal, because a program
                // that carried on silently after finding its own name in a stranger's hands
                // would be the same silence from the other side.
                InstanceVerdict::NameTakenByAStranger => {
                    crate::diag::record(
                        crate::diag::Operation::from_name(NAME_TAKEN_BY_A_STRANGER),
                        crate::diag::OsCode::NONE,
                    );

                    run_as_first_instance(instance)
                }
            }
        }
        Err(error) => {
            // FR-82 cannot be honoured if the mutex cannot be created at all, and starting
            // anyway would mean two copies hooking the keyboard. Refusing to start is the
            // only answer that keeps the requirement true.
            report_non_critical("CreateMutexW", &error);

            // ⭐⭐ **Решение 125а, вариант 2 — this is the one path the journal writes on even
            // when it is switched off.** And it is the path finding Н24 is named for: a name
            // held by an object of **another type** — an event, say — makes `CreateMutexW`
            // *fail* rather than succeed, so the worst case of the finding comes out here and
            // not through the arm above. The program cannot start, there is no window and no
            // second copy to point at, and with `[diagnostics] log_enabled` off — which is the
            // default of section 7 — it used to leave nothing at all. The person saw «программа
            // не запускается», with no cause on the screen and none on the disk.
            //
            // So this one exit writes the ring regardless. It is the single named exception to
            // the promise «выключено — не пишется ничего», and the owner named it himself.
            leave_the_cause_of_a_refused_start_on_the_disk();

            ExitCode::FAILURE
        }
    }
}

/// The journal row of a name held by somebody who is not this program — task T-41-5.
const NAME_TAKEN_BY_A_STRANGER: &str = "single-instance name held by a stranger";

/// What the two facts about the single-instance name add up to — finding Н24, task T-41-5.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstanceVerdict {
    /// The name was free. This process is the instance of FR-82.
    TheInstance,
    /// The name is held **and** a window of this program's own class is up in this session:
    /// a real second copy, and FR-82's answer is unchanged.
    SecondCopy,
    /// The name is held and no window of ours is anywhere. Somebody else took the name — by
    /// accident far more likely than by design, the name being a plain string any program can
    /// name an object with.
    NameTakenByAStranger,
}

/// The rule of FR-82 as it stands after task T-41-5 — pure, and a table of four closes it.
///
/// `our_window_found` is only meaningful when `name_taken`; the caller does not ask the system
/// at all when the name was free, and the table says so by answering the same thing either way.
pub fn instance_verdict(name_taken: bool, our_window_found: bool) -> InstanceVerdict {
    match (name_taken, our_window_found) {
        (false, _) => InstanceVerdict::TheInstance,
        (true, true) => InstanceVerdict::SecondCopy,
        (true, false) => InstanceVerdict::NameTakenByAStranger,
    }
}

/// How long a window of ours is waited for before the name is called a stranger's — task
/// T-41-5. Milliseconds.
///
/// ⚠ **This is not politeness, it is the race.** A real second copy started a tenth of a second
/// after the first finds the name already taken and the first copy's window **not yet created**
/// — the name is acquired here, at the top of `run`, and the windows are built much later, on
/// the three threads. Without the wait that second copy would call the first one a stranger and
/// start, and two copies of this program would hook the keyboard: exactly what FR-82 exists to
/// prevent. Two seconds is far longer than the gap between the two lines and far shorter than
/// anybody's patience.
///
/// It costs nothing in the two ordinary cases. A free name is not asked about at all, and a
/// genuine second copy finds the window on the first look.
const WINDOW_SEARCH_BUDGET_MS: u64 = 2_000;

/// One step of that wait.
const WINDOW_SEARCH_STEP_MS: u64 = 50;

/// Whether a window of this program's own class is up in this session — task T-41-5.
///
/// ⚠ **It cannot find this process's own window, and the order of `run` is why.** The name is
/// acquired on the line above this call; the windows are created inside `run_as_first_instance`,
/// on the threads it spawns, which this process has not reached. There is nothing of ours to
/// find yet, so what a hit means is unambiguous: another copy of this program is up.
fn a_window_of_ours_is_up() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    let steps = WINDOW_SEARCH_BUDGET_MS / WINDOW_SEARCH_STEP_MS;

    for step in 0..=steps {
        // SAFETY: both arguments are `'static` UTF-16 literals of this build, and the call
        // reads a system table and keeps nothing of ours. A class that is not registered
        // anywhere in the session is an `Err`, which is the answer «нет такого окна» and not a
        // failure to report.
        if unsafe { FindWindowW(WINDOW_CLASS_NAME, PCWSTR::null()) }.is_ok() {
            return true;
        }

        if step < steps {
            std::thread::sleep(std::time::Duration::from_millis(WINDOW_SEARCH_STEP_MS));
        }
    }

    false
}

/// Writes the ring to the file on a path that never reaches the threads — task T-41-5, part (а)
/// of finding Н24.
///
/// # Why this exists at all
///
/// `diag::dump_on_shutdown` is called from exactly one place — `thread_body(Role::Ui)`, as the
/// UI thread leaves its message loop — and **both early exits of `run` return before any thread
/// is spawned**. So until this task everything the ring held on those two paths died in memory:
/// the refusal of `CreateMutexW` was recorded and never written, which is the half of finding
/// Н24 the repair of the verdict above does not by itself answer.
///
/// # What it obeys
///
/// The person's own `[diagnostics] log_enabled`. On this path nothing has read the configuration
/// yet — the publication that normally tells `diag` about it happens after the threads are up —
/// so it is read here, once, for this one question. With the journal off nothing is written,
/// which is the promise of `diag::dump_on_shutdown` and is kept here.
///
/// The one exit that does **not** obey it is the refusal of `CreateMutexW`: see
/// [`leave_the_cause_of_a_refused_start_on_the_disk`] and решение 125а.
fn flush_the_journal_of_an_early_exit() {
    let Some(path) = settings::default_config_path() else {
        return;
    };

    let Ok((config, _)) = settings::read_from(&path) else {
        // NFR-13: a file that will not read is not a reason to invent a setting. The dump does
        // not happen, and the person's `log_enabled` is not guessed at.
        return;
    };

    crate::diag::set_log_enabled(config.diagnostics.log_enabled);
    crate::diag::dump_on_shutdown();
}

/// Writes the ring to the file **whatever the setting says** — решение 125а, вариант 2, part (б)
/// of finding Н24.
///
/// # The one named exception, and why it is this path and no other
///
/// `diag::dump_on_shutdown` promises that a journal switched off creates nothing: «no folder, no
/// file, not an empty one». That promise is kept everywhere except here.
///
/// Here the program **cannot start at all**. `CreateMutexW` refused the name of FR-82 — which is
/// what happens when the name is held by an object of another type, or by one whose rights shut
/// us out, and that is the worst case of finding Н24 — and so there is no window, no second copy
/// to point at and nothing for the person to read. Before this task the whole event died in
/// memory; with the journal off it would die in memory still, and «программа не запускается»
/// would stay a sentence with no cause behind it.
///
/// The owner chose this exception by name over the two alternatives (nothing at all, or a window
/// and a new exit code); the cost he accepted is that a file may appear for somebody who switched
/// the journal off, in a case that should never happen at all.
///
/// ⚠ **The setting is published as `true` and not bypassed**, so the dump takes the one road
/// every other dump takes and there is no second writer to keep in step. Nothing is restored
/// afterwards: the process is three lines from its own end.
fn leave_the_cause_of_a_refused_start_on_the_disk() {
    crate::diag::set_log_enabled(true);
    crate::diag::dump_on_shutdown();
}

/// Asks every thread of the process to leave its message loop. Idempotent, callable from
/// any thread, and safe to call before the threads exist or after they are gone.
///
/// This is the interface task T-01-4 attaches to for FR-83, and the one the panic hook
/// (FR-98) uses. Decision R-20 point 3: the flag is what decides, the message only wakes.
pub fn request_shutdown() {
    // ⭐ **Task Т-22-7, finding м7 of the audit of 2026-09-01 — `SeqCst`, and the comment is
    // now literally true.**
    //
    // This store and the load below are one half of a **store-buffer litmus**; the other half
    // is `Window::create` publishing its handle and `serve_window` re-reading this flag right
    // afterwards. Written out, with the two threads side by side:
    //
    // ```text
    //   request_shutdown                     serve_window / Window::create
    //   ----------------                     -----------------------------
    //   store SHUTDOWN_REQUESTED = true      store WAKE_TARGETS[role] = hwnd
    //   load  WAKE_TARGETS[role]  -> ?       load  SHUTDOWN_REQUESTED  -> ?
    // ```
    //
    // Release on a store and Acquire on a load — which is what stood here — order **each
    // thread's own** operations against the data those operations publish. Neither forbids the
    // outcome where both loads answer with the value that was there before either store: a
    // release store may still be sitting in the storing core's buffer while the other thread's
    // load goes ahead. `SeqCst` does forbid it, because it puts all four into one total order,
    // and in any total order at least one of the two stores precedes the other thread's load.
    //
    // That outcome is the whole defect: `request_shutdown` reads `NO_WINDOW` and posts nothing,
    // `serve_window` reads `false` and enters `GetMessageW`, and the thread pumps for ever with
    // nobody left to ask it again. The comment that used to stand here promised the opposite —
    // "guaranteed to observe `true`" — of the one ordering that does not give it.
    //
    // The cost is nothing worth measuring: these are cold paths. This store runs once per
    // process shutdown, the load below three times with it, the publication once per thread at
    // start-up, and the re-read once per thread before its first `GetMessageW`.
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);

    for target in &WAKE_TARGETS {
        // `SeqCst`, and not merely to match: this load is the second operation of the litmus
        // above, and a total order that one of the four is missing from is not a total order.
        let raw = target.load(Ordering::SeqCst);
        if raw == NO_WINDOW {
            // That thread has not created its window yet, or has already destroyed it.
            // Neither needs a wake-up: the thread re-reads the flag right after publishing
            // its window and before entering the loop, so it cannot start pumping without
            // seeing a request that was made before the publication.
            continue;
        }

        // SAFETY: `raw` was published by `Window::create` from the handle CreateWindowExW
        // returned and is cleared by `Window::drop` before the window is destroyed, so the
        // value read here is either a live window of this process or `NO_WINDOW`, which was
        // filtered out above. PostMessageW only queues the message and returns; it never
        // dereferences wparam or lparam, both of which are zero here in any case, and it
        // does not block, which is what makes this callable from the panic hook and, later,
        // from hook-adjacent code (NFR-04).
        let posted = unsafe {
            PostMessageW(
                Some(HWND(raw as *mut c_void)),
                WM_APP_WAKE,
                WPARAM(0),
                LPARAM(0),
            )
        };

        if let Err(error) = posted {
            // NFR-13: the result is examined, not discarded. A failure here is benign and
            // must not stop the loop — it means the target thread destroyed its window
            // between the load above and this call, that is, it is already leaving.
            report_non_critical("PostMessageW", &error);
        }
    }

    #[cfg(debug_assertions)]
    debug_timeout::wake_main_thread();
}

/// Whether shutdown has been requested. Every message loop consults this and nothing else.
///
/// `SeqCst` since task **Т-22-7**: the re-read `serve_window` makes after publishing its window is
/// the fourth operation of the store-buffer litmus written out in [`request_shutdown`], and all
/// four have to be in one total order for the guarantee that comment claims to exist at all. The
/// other callers pay nothing for it — this is a load of a flag on a path that is already leaving.
pub fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

/// Puts the shutdown flag back to its start-up value — tests of this module only.
///
/// The flag is a `static` of the process and the test binary is one process, so a test that
/// observes the *transition* to "requested" has to be able to start from "not requested" and
/// has to leave the binary as it found it. There is no production caller and there must never
/// be one: nothing in a running program may unsay a shutdown somebody asked for.
#[cfg(test)]
fn clear_shutdown_request() {
    SHUTDOWN_REQUESTED.store(false, Ordering::Release);
}

/// How many times the layout cache of FR-20 failed to build and the hardwired table of FR-25
/// was used instead — see [`LAYOUT_CACHE_FAILURES`].
///
/// Non-zero on a machine whose layout list cannot be read, or one on which every loaded layout
/// is IME based (FR-35). The program runs either way; what it loses is the ability to convert
/// anything but the RU/EN pair the fallback table carries.
pub fn layout_cache_failures() -> u32 {
    LAYOUT_CACHE_FAILURES.load(Ordering::Relaxed)
}

/// The UI thread's window as a raw value, or zero when that thread has none right now.
///
/// Exists for FR-99: the fail-safe transition is decided on the input thread, inside the
/// hook callback, and the icon that has to show it belongs to the UI thread. A window handle
/// is the only thing the two threads can exchange without a lock, and `PostMessageW` is the
/// only call `hook` is allowed to make with it (NFR-04).
///
/// Raw rather than an `HWND` on purpose: `HWND` is a raw pointer and therefore not `Send`,
/// and a type that promised otherwise would be a lie about a value that really does cross
/// threads. The conversion back is exact, and `hook` performs it inside the `unsafe` block
/// whose safety comment accounts for it.
///
/// ⚠ **The `cfg(panic = "unwind")` came off in task T-36-4** (finding Н40), and the paragraph it
/// used to carry said why it was there: FR-99 exists only where a panic can be absorbed, so under
/// the `panic = "abort"` of the Release profile (section 3.2) this accessor had no caller and
/// would have been unreachable code. It has one in both configurations now — `hook::post_hotkey`
/// answers a failed handoff with `WM_APP_SOUND_IDLE` at this very window, and that function is
/// not selected on the panic strategy. Criterion 29 of section 13 (no `#[allow(dead_code)]` in
/// the shipped binary) is satisfied by the caller rather than by the `cfg`.
pub(crate) fn ui_window_raw() -> usize {
    WAKE_TARGETS[Role::Ui.index()].load(Ordering::Acquire)
}

// ---------------------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------------------

/// Number of threads section 6.1 prescribes: input, UI, watcher.
const THREAD_COUNT: usize = 3;

/// Value in [`WAKE_TARGETS`] meaning "this thread has no window right now".
const NO_WINDOW: usize = 0;

/// The private message that wakes a message loop so it re-reads [`SHUTDOWN_REQUESTED`].
///
/// SEC-05: any process at the same integrity level can post this too. That is harmless by
/// construction — the handler decides on the flag, so an unsolicited `WM_APP_WAKE` makes
/// the loop re-read a flag that is still `false` and carry on.
const WM_APP_WAKE: u32 = WM_APP + 1;

/// The private message that tells the input thread the configuration has been published.
///
/// `WM_APP + 5`: `WM_APP + 1` is the wake-up above, `WM_APP + 2` is the tray callback of
/// [`crate::tray`], and `WM_APP + 3` and `WM_APP + 4` belong to [`crate::hook`].
///
/// SEC-05: like [`WM_APP_WAKE`], it carries nothing and decides nothing. The handler re-reads
/// [`BUFFER_CAPACITY`] — an atomic of this process that no sender can influence — and does
/// nothing at all unless the buffer it finds is of the wrong size, so an unsolicited post buys
/// the sender one comparison.
const WM_APP_CONFIGURED: u32 = WM_APP + 5;

// ---------------------------------------------------------------------------------------
// FR-100 — the sound of a press. Task Т-21-5, finding №10 of the audit of 2026-08-31
// ---------------------------------------------------------------------------------------

/// Posted to the UI thread when a press **replaced** text — FR-100.
///
/// `WM_APP + 17`, the first number free above `watchdog::WM_APP_WIPE`.
///
/// **Two messages and not one with a `wparam`**, which is the shape [`post_to`] already has:
/// it posts zero in both parameters and its `SAFETY` note rests on that. The same choice the
/// selection path made when it needed a second meaning — `WM_APP_BUFFER_PATH` is a message of
/// its own rather than a re-post of `WM_APP_HOTKEY`.
///
/// SEC-05: any process at the same integrity level can post this, and what it buys the sender
/// is one playback of a click — a sound any process may make for itself, and one that says
/// nothing about what this program is doing. No state of this program is read, changed or
/// decided by the handler; the setting of FR-100 is still obeyed, so a posted message on a
/// machine with the sound switched off is silent exactly as a real press would be.
pub const WM_APP_SOUND_DONE: u32 = WM_APP + 17;

/// Posted to the UI thread when a press was **idle** — FR-100. See [`WM_APP_SOUND_DONE`].
pub const WM_APP_SOUND_IDLE: u32 = WM_APP + 18;

/// Posted to the UI thread when a press was **refused** — FR-100, task **Т-49-2**.
///
/// `WM_APP + 20`, the first free number: `+ 19` is `letters::WM_APP_FEED`, and the whole map is
/// written down in the doc comment of every constant of the family. See [`WM_APP_SOUND_DONE`]
/// for the shape and for what SEC-05 buys a forger, and [`Press::Refused`] for which situations
/// this is and which — `Field::Pending` — it deliberately is not.
pub const WM_APP_SOUND_REFUSED: u32 = WM_APP + 20;

/// The sound of a press that replaced text — FR-100, «чк-чк».
///
/// # Why the sound is the program's own and not the system's
///
/// It was `MessageBeep(MB_OK)` until the acceptance by ear of stage Э21. The user heard both
/// tones, accepted that the feature works, and rejected the tones themselves: what a switch
/// should sound like is a **switch** — two clicks when the layout changed and one when it did
/// not — and no member of the `MessageBeep` family is a click. The system tones are also the
/// user's own property: they are what a message box, an error and a notification sound like on
/// this machine, and borrowing them makes a keystroke sound like an error dialog.
///
/// # Why it is embedded and not a file
///
/// `include_bytes!` puts the whole `.wav` in the image's read-only data. Nothing is installed
/// beside the executable, nothing can be missing at run time, nothing has to be found on disk —
/// and the requirement's «без звуковых файлов» survives the change of mechanism: there are no
/// sound files anywhere on the user's machine, only bytes inside the program. It also makes the
/// `'static` lifetime `SND_ASYNC` needs true by construction — see [`SystemBeeper::beep`].
///
/// The waveform is a click of one 2 kHz mode over a short filtered noise burst, 26 ms, and this
/// one is that click twice, 70 ms apart. Synthesised for this program (the generator and the
/// thirty-one candidates that lost are in `scratchpad-Э21`), so nothing here is anybody's
/// property but this project's.
pub const SOUND_REPLACED: &[u8] = include_bytes!("../res/sound-replaced.wav");

/// The sound of a press that was suppressed and replaced nothing — FR-100, «чк».
///
/// The **same** click as [`SOUND_REPLACED`], once instead of twice. Deliberately the same
/// mechanism: two different noises would be two different objects, while one switch clicking
/// once or twice is a single object saying two things — which is what makes the pair legible
/// without looking at the screen.
pub const SOUND_IDLE: &[u8] = include_bytes!("../res/sound-idle.wav");

/// The sound of a press this program **refused** — FR-100, «чк» с низким телом, task **Т-49-2**.
///
/// # The third thing one switch can say
///
/// The user, 2026-09-09: «в поле пароля когда вводишь, нет никакого звукового сигнала и том что
/// смена не удалась, да и в принципе сознательно не сработала, хорошо бы было такой звуковой
/// сигнал создать, чтобы он был в том же стиле, но чётко означал отказ». There was no sound at
/// all — not the idle click, nothing: the branch that answers a press is gated on the typing
/// buffer being on this thread, and in a password field FR-70 has **parked** it. See
/// [`press_outcome`], which is where that hole is closed.
///
/// # Why it is this sound and not a new one
///
/// The rule [`SOUND_IDLE`] states holds for the third arm too: two different noises would be two
/// different objects. So this is **built out of the shipped click itself** — the very bytes of
/// [`SOUND_IDLE`], which are `clicks\idle-08.wav` of stage Э21, the click the user accepted by
/// ear («звук отличный, приёмка пройдена», 2026-09-01) — with a low body under it that decays in
/// 62 ms. The same switch, hitting its lock: a dull knock rather than a second event. Nothing was
/// re-synthesised from parameters, because the generator round that produced `clicks\` did not
/// survive and re-deriving the timbre would have been guesswork; re-arranging the bytes cannot
/// drift.
///
/// Chosen by ear out of eight candidates, all built the same way — the user, 2026-09-09: «Вот
/// этот вариант подходит: otkaz-4 «чк» с низким телом — глухой стук, а не второе событие». The
/// eight and the generator are in `scratchpad-звук-отказа\`.
///
/// Its peak is the peak of [`SOUND_REPLACED`]: a refusal has to be legible, not loud.
pub const SOUND_REFUSED: &[u8] = include_bytes!("../res/sound-refused.wav");

/// Which of the two sounds a press earned — FR-100.
///
/// An enum and not the raw bytes, because the decision and the playing of it are different
/// jobs: [`tone_for`] is a pure function that answers this, and only [`SystemBeeper`] turns it
/// into a sound. The test bench records these values, which is what makes «one press, one
/// tone» an assertion instead of a hope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// The replacement happened — «чк-чк».
    Replaced,
    /// The press was suppressed and produced nothing — «чк».
    Idle,
    /// **The program refused to act here** — a dull knock, task Т-49-2. See [`Press::Refused`]
    /// for which situations that is and which it deliberately is not.
    Refused,
}

impl Tone {
    /// The bytes this tone is, ready for `SND_MEMORY`.
    pub const fn wave(self) -> &'static [u8] {
        match self {
            Self::Replaced => SOUND_REPLACED,
            Self::Idle => SOUND_IDLE,
            Self::Refused => SOUND_REFUSED,
        }
    }
}

/// What one press of the hotkey **did** — the three outcomes FR-100 tells apart, task Т-49-2.
///
/// Separate from [`Tone`] and one-to-one with it on purpose: this is what happened, that is what
/// the user hears, and the module already keeps the decision and the playing of it apart (see
/// [`Beeper`]). A future tone that answered two outcomes, or an outcome that earned silence,
/// would have nowhere to live if the two were one type.
///
/// **SEC-01, SEC-07.** Three named constants. Nothing here can carry a character, a scan code or
/// the name of a window, and nothing that could may be added: what the user typed never reaches
/// the sound path at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// Text on the screen changed — «чк-чк».
    Replaced,
    /// The press was answered and changed nothing: an empty buffer, a word with no direction,
    /// a verdict that has not come back yet. **Not a refusal** — nothing was forbidden, there
    /// was simply nothing to do.
    Idle,
    /// **This program would not act here** — FR-70 (a password field), FR-84 (an excluded
    /// process), and, since task T-42-6, a **window** that refused what the hand asked of it.
    ///
    /// ⚠ The wording used to say «those two only», and task T-42-6 (finding Н112) is what made
    /// it untrue: the settings window refuses the ninth tick of the cycle list and says so with
    /// this very tone, through [`sound_refusal`]. The doc is widened rather than a fourth value
    /// invented — what the user hears is the same dull knock, and the rule behind it is the
    /// same one: *this program would not act here*. The урок of Э49 is why the sentence is
    /// corrected in the same commit as the code that made it untrue.
    ///
    /// ⚠ [`crate::guard::Field::Pending`] is deliberately **not** one of them, and the
    /// distinction is the whole point of the arm: «ответа ещё нет» is not knowing, not
    /// forbidding, and a program that said «отказано» while its own probe was still running
    /// would be saying something untrue. A press in that window is [`Press::Idle`].
    Refused,
}

/// `[feedback] sound` as the UI thread last published it — FR-100.
///
/// An atomic for the reason every published setting is one (section 6.3): the value is written
/// by [`publish_configuration`] on the UI thread and read where the sound is made. Starts `true`,
/// which is the default of section 7, so a press before the first publication is not silent.
static SOUND_ENABLED: AtomicBool = AtomicBool::new(true);

/// Publishes `[feedback] sound` — FR-100. Called only from [`publish_configuration`].
pub fn set_sound_enabled(on: bool) {
    SOUND_ENABLED.store(on, Ordering::Relaxed);
}

/// What [`set_sound_enabled`] last published — FR-100. For the dump of `control` and the tests.
pub fn sound_enabled() -> bool {
    SOUND_ENABLED.load(Ordering::Relaxed)
}

/// Where a tone goes — the seam FR-100 is tested through.
///
/// One method and one argument. The production implementation is [`SystemBeeper`] and calls
/// `PlaySoundW`; the tests implement it with a `Vec`, which is what makes «one press, one tone»
/// an assertion instead of a hope. The same shape `selection::Path` gives the eight steps of
/// FR-61 and `inject::Environment` gives the packet of FR-44.
pub trait Beeper {
    /// Makes the tone, or writes it down.
    fn beep(&mut self, tone: Tone);
}

/// The [`Beeper`] of the running program: one `PlaySoundW` and nothing else.
struct SystemBeeper;

impl Beeper for SystemBeeper {
    fn beep(&mut self, tone: Tone) {
        let wave = tone.wave();

        // SAFETY: three things, and the third is the one that is easy to get wrong.
        //
        // The pointer. With `SND_MEMORY` the first parameter is not a string at all — it is the
        // address of a `.wav` image in memory, which the signature spells `PCWSTR` because the
        // parameter is overloaded by the flags. `wave` is a slice of the executable's read-only
        // data, so the address is valid and the bytes behind it are a complete RIFF file.
        //
        // The module handle. `None` is correct and required: a handle is read only under
        // `SND_RESOURCE`, and this is `SND_MEMORY`.
        //
        // ⚠ The lifetime. With `SND_ASYNC` the call returns while the sound is still playing,
        // and the documentation requires the buffer to stay alive until it finishes. It does,
        // by construction rather than by care: `include_bytes!` makes these bytes `'static`
        // data of the image, so there is no frame for them to be freed with — which is the
        // whole reason the sound is embedded rather than read into a `Vec` at start-up.
        //
        // `SND_NODEFAULT` so that a machine which cannot play this falls silent instead of
        // substituting the system beep, which is precisely the sound the user rejected.
        //
        // The result says whether anything was played; there is nothing this program could do
        // about «no», and a journal entry per press would be an entry per keystroke — the very
        // thing NFR-01…05 and the ceiling note of T-13-13 argue against.
        let _ = unsafe {
            PlaySoundW(
                PCWSTR(wave.as_ptr().cast()),
                None,
                SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
            )
        };
    }
}

/// **What a press earned, given the three things that decide it** — FR-100, task **Т-49-2**.
///
/// `buffered` is whether the typing buffer is on this thread at all, `refuses` is
/// [`crate::guard::refuses`] — FR-70 or FR-84 — and `replaced` is whether text on the screen
/// changed. `None` means the press is not this function's to answer.
///
/// # The hole this closes, and how it hid
///
/// The branch that answers a press used to be gated on `buffer::is_installed()` and nothing
/// else. In a password field FR-70 **parks** the buffer — `park_buffer` calls
/// `buffer::uninstall` — so the gate was shut and **no message was posted at all**: not the idle
/// click, nothing. The user, 2026-09-09: «в поле пароля когда вводишь, нет никакого звукового
/// сигнала и том что смена не удалась, да и в принципе сознательно не сработала».
///
/// ⚠ **And the doc comment that used to stand here argued from a premise the code did not
/// meet.** It said a password field needed no branch of its own because «silence where every
/// other idle press sounds says exactly the same thing, and louder» — which is true, and which
/// is precisely what the program did: it was silent there and clicked everywhere else. The
/// difference the note set out to avoid existed the whole time, unannounced. What is new here is
/// not the difference; it is that the difference now **says what it means**.
///
/// # The three answers
///
/// * **the buffer is not on this thread and the program refuses** — [`Press::Refused`]. This is
///   the arm that did not exist;
/// * **the buffer is not on this thread and nothing is being refused** — `None`. The thread owns
///   no buffer (the UI and watcher threads), or the input pipeline has not started yet. There is
///   nothing to report and nobody to report it to;
/// * **the buffer is here** — [`Press::Replaced`] or [`Press::Idle`], exactly as before.
///
/// A pure function of three booleans, so the rule can be read in one line and driven by unit
/// tests rather than by the machine's speakers.
pub const fn press_outcome(buffered: bool, refuses: bool, replaced: bool) -> Option<Press> {
    if !buffered {
        return if refuses { Some(Press::Refused) } else { None };
    }

    Some(if replaced {
        Press::Replaced
    } else {
        Press::Idle
    })
}

/// Which tone an outcome earns, or `None` when `[feedback] sound` is off — the whole of FR-100.
///
/// A pure function of an outcome and a setting, so the rule can be read in one line. The
/// mapping is one-to-one today; it is written out rather than derived so that a tone which ever
/// answers two outcomes has a place to be written down.
///
/// ⚠ **The switch of `[feedback] sound` governs all three tones together** — there is no
/// per-tone setting and section 7 grows no key: the user asked for a third sound, not for a
/// third preference.
pub const fn tone_for(press: Press, enabled: bool) -> Option<Tone> {
    if !enabled {
        return None;
    }

    Some(match press {
        Press::Replaced => Tone::Replaced,
        Press::Idle => Tone::Idle,
        Press::Refused => Tone::Refused,
    })
}

/// Answers one press with its tone — FR-100, through the seam.
///
/// # Where this runs — NFR-01…NFR-05
///
/// **On the UI thread, after the outcome of the press is known, and nowhere else.** Not in the
/// hook callback, which returns a verdict to the system and owes it microseconds; not on the
/// input thread either, whose budget NFR-09 puts at thirty milliseconds for the whole of a
/// replacement. The input thread learns the outcome and *posts* it — [`WM_APP_SOUND_DONE`],
/// [`WM_APP_SOUND_IDLE`] or [`WM_APP_SOUND_REFUSED`] — and `PostMessageW` queues and returns.
/// The selection path already runs here and calls this directly.
///
/// ⚠ **"Nowhere else" is enforced since task T-36-4** and was a description before it: the branch
/// of [`window_proc`] that answers the three messages used to answer them at whichever window
/// they arrived at, so a forged one (SEC-05) aimed at the **input** window played the sound on
/// the thread that holds the hook. [`is_ui_window`] is the gate that makes the sentence true.
pub fn answer_press<B: Beeper>(beeper: &mut B, press: Press, enabled: bool) {
    if let Some(tone) = tone_for(press, enabled) {
        beeper.beep(tone);
    }
}

/// Answers one press with the system's own voice — [`answer_press`] with [`SystemBeeper`].
///
/// The one line of the program that is allowed to make a sound, and the only caller of the
/// production [`Beeper`]. Reached from the UI thread's window procedure and from nowhere else —
/// which task T-36-4 turned from a description into a gate, [`is_ui_window`].
fn sound_press(press: Press) {
    answer_press(&mut SystemBeeper, press, sound_enabled());
}

/// The dull knock of FR-100 for a **window** that refused what the hand asked — task T-42-6,
/// finding Н112.
///
/// The one door this module opens to `src\settings.rs`, and it is deliberately narrow: no
/// argument, no choice of tone, nothing a caller could pass that would make a different sound.
/// The settings window refuses the ninth tick of the cycle list ([`crate::layouts::MAX_CYCLE`]
/// is eight) and calls this so that the refusal is **heard** as well as seen.
///
/// # ⛔ The sound is the addition, not the cure
///
/// [`tone_for`] answers `None` when `[feedback] sound` is off, so for a user who turned the
/// sound off this call does nothing at all — and the ninth tick still does not go in. That is
/// the invariant of the task: the refusal is **visible** at every setting, and this is what is
/// laid on top of it. A cure that lived here would be no cure for half the users.
///
/// Runs on the UI thread, inside the modal call of the settings dialog — the thread
/// [`answer_press`] documents as its own.
pub fn sound_refusal() {
    sound_press(Press::Refused);
}

/// Which outcome one of the three sound messages carries, or `None` for every other message.
///
/// The inverse of the posts in the hotkey branch, written as a total function so that the window
/// procedure has one arm for the family instead of three, and so that a forged message
/// (SEC-05) buys its sender one click of a sound the machine can make for itself and nothing
/// else — the argument [`WM_APP_SOUND_DONE`] already makes.
const fn press_of_sound_message(message: u32) -> Option<Press> {
    match message {
        WM_APP_SOUND_DONE => Some(Press::Replaced),
        WM_APP_SOUND_IDLE => Some(Press::Idle),
        WM_APP_SOUND_REFUSED => Some(Press::Refused),
        _ => None,
    }
}

/// Which message carries `press` to the UI thread — the inverse of [`press_of_sound_message`].
const fn sound_message_for(press: Press) -> u32 {
    match press {
        Press::Replaced => WM_APP_SOUND_DONE,
        Press::Idle => WM_APP_SOUND_IDLE,
        Press::Refused => WM_APP_SOUND_REFUSED,
    }
}

/// The one and only shutdown flag of the process.
///
/// Process-global rather than an `Arc` threaded through every frame because the state it
/// describes really is process-global — there is exactly one `run()` per process — and
/// because the window procedure has to reach it without the `GWLP_USERDATA` raw-pointer
/// dance, which would trade a plain atomic for a pointer whose lifetime nobody can check.
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

/// The window each thread wants its wake-up posted to, indexed by [`Role`].
///
/// Stored as `usize` because `HWND` is a raw pointer and therefore neither `Sync` nor
/// storable in an atomic; the conversion is round-trip exact in both directions.
static WAKE_TARGETS: [AtomicUsize; THREAD_COUNT] =
    [const { AtomicUsize::new(NO_WINDOW) }; THREAD_COUNT];

/// `[buffer] capacity` of section 7 as the UI thread published it — FR-07, section 6.3.
///
/// Zero means "the UI thread has not read the file yet", and it is also exactly what
/// [`crate::buffer::effective_capacity`] turns into the default of section 7, so the input
/// thread can install its buffer from this value before anybody has published anything and get
/// the documented default rather than a special case.
static BUFFER_CAPACITY: AtomicUsize = AtomicUsize::new(0);

/// `[buffer] idle_timeout_s` as the input thread sees it — **FR-15**, task T-52-4, **already
/// clamped**.
///
/// [`IDLE_TIMEOUT_UNPUBLISHED`] until the UI thread has read the file. The value cannot use zero
/// as its "nothing yet" marker the way [`BUFFER_CAPACITY`] does, because zero is a **documented
/// value** of this field — it is how section 7 says «выключено» — so the marker is a number the
/// publication can never produce: the ceiling of [`crate::buffer::MAX_IDLE_TIMEOUT_S`] is a day,
/// and the clamp is applied *here*, before the store, exactly as task T-13-13 clamps the three
/// millisecond fields where they cross to their threads.
static BUFFER_IDLE_TIMEOUT_S: AtomicU32 = AtomicU32::new(IDLE_TIMEOUT_UNPUBLISHED);

/// The "the UI thread has not read the file yet" value of [`BUFFER_IDLE_TIMEOUT_S`].
const IDLE_TIMEOUT_UNPUBLISHED: u32 = u32::MAX;

/// How many times [`LayoutCache::build`] failed and the hardwired table of FR-25 was used.
///
/// FR-25 asks for the fallback, not for a diagnosis, and a program that cannot enumerate the
/// layouts of the session must still run — so the failure is *counted* rather than escalated.
/// A counter is also all NFR-05 would allow if this ever moved closer to the callback, and it
/// is what module `diag` (task T-06-4) will journal.
static LAYOUT_CACHE_FAILURES: AtomicU32 = AtomicU32::new(0);

/// What a thread of this process is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Section 6.1: the LL hook, Raw Input, `SendInput`, the layout cache. Never UI, never
    /// file I/O, never COM.
    Input,
    /// Section 6.1: the tray icon, the menu, the settings dialog, reading and writing the
    /// configuration.
    Ui,
    /// Section 6.1: `SetWinEventHook` and the UI Automation password-field probe, which is
    /// why this is the only thread with a COM apartment.
    Watcher,
}

impl Role {
    /// Index of this role in [`WAKE_TARGETS`].
    const fn index(self) -> usize {
        match self {
            Self::Input => 0,
            Self::Ui => 1,
            Self::Watcher => 2,
        }
    }

    /// Name given to the OS thread. Diagnostic only; nothing branches on it.
    const fn thread_name(self) -> &'static str {
        match self {
            Self::Input => "langsw-input",
            Self::Ui => "langsw-ui",
            Self::Watcher => "langsw-watcher",
        }
    }
}

// ---------------------------------------------------------------------------------------
// FR-82 — the single instance
// ---------------------------------------------------------------------------------------

/// Name of the single-instance mutex.
///
/// `Local\` and not `Global\`: FR-82 asks for one instance **per user session**, and a
/// machine-wide object would mean the second user to log on could not run the program at
/// all. The leaf is the `CONFIG_DIR_NAME` form of the name — the underscored, never
/// translated one — because this string is an identity that must survive every rename of
/// the display name.
const MUTEX_NAME: PCWSTR = w!(r"Local\Lang_Switcher.SingleInstance");

/// Text of the FR-82 notification.
///
/// English, and hard-wired rather than taken from a resource, because FR-94 (interface
/// strings in RU and EN resources) is implemented against `app.rc`, which this task may not
/// touch. Moving this string into the resources belongs with the rest of FR-94.
const ALREADY_RUNNING_TEXT: PCWSTR = w!("Lang Switcher is already running in this session.");

/// Outcome of trying to become the one instance of the program.
enum Acquisition {
    /// This process is the instance; the mutex lives as long as the guard.
    Acquired(SingleInstance),
    /// The name was already held when this process asked for it.
    ///
    /// ⭐ **It carries the guard too, since task T-41-5.** The handle is a real, open handle to
    /// the object — `CreateMutexW` answers one whether it made the object or found it — and
    /// until this task it was dropped on this line, because the only thing left to do was
    /// leave. Now there is a second road out of this arm: when no window of ours can be found,
    /// the name is a stranger's and this process starts anyway, and it starts **holding this
    /// handle**. Holding it is the right thing on that road: the name then stays taken for as
    /// long as this copy lives, so a genuine second copy of ours still sees it held — and finds
    /// this one's window, and refuses, exactly as FR-82 says.
    AlreadyRunning(SingleInstance),
}

impl Acquisition {
    /// The guard, whichever arm carried it — task T-41-5.
    fn into_guard(self) -> SingleInstance {
        match self {
            Self::Acquired(instance) | Self::AlreadyRunning(instance) => instance,
        }
    }
}

/// Ownership of the named mutex behind FR-82.
struct SingleInstance {
    handle: HANDLE,
}

impl SingleInstance {
    /// Creates or opens the named mutex and reports which of the two happened.
    ///
    /// The distinction is the whole subtlety of FR-82: `CreateMutexW` returns a perfectly
    /// valid handle when the mutex already exists, so "another instance is running" is
    /// established by `GetLastError() == ERROR_ALREADY_EXISTS` **after a successful
    /// return**, never by a null handle. Reading the error instead of the handle, or the
    /// handle instead of the error, are the two classic ways to get this wrong.
    fn acquire() -> WinResult<Acquisition> {
        // SAFETY: no security attributes are passed, which asks for the default descriptor
        // — the right one, since the object must be reachable by this user's session and by
        // nobody else. `false` for bInitialOwner: the program never waits on this mutex and
        // never needs to own it, only to observe that the name exists, and not owning it
        // means it can never be left abandoned if this process dies. `MUTEX_NAME` is a
        // NUL-terminated `'static` UTF-16 literal, so the pointer the call reads through
        // outlives the call by definition.
        let handle = unsafe { CreateMutexW(None, false, MUTEX_NAME)? };

        // SAFETY: GetLastError reads the calling thread's last-error value and touches no
        // memory of ours. It is called immediately after the CreateMutexW above, on the
        // same thread and with no intervening call of any kind, which is the condition that
        // makes the value meaningful. The `?` above cannot have consumed it: the windows
        // crate builds its error from the thread only on the failure path.
        let last_error = unsafe { GetLastError() };

        // The guard is built in both branches so that the handle is closed on both. The
        // second instance is required to close its handle too: it is a real, open handle to
        // the first instance's object, and leaking it would keep the kernel object alive
        // past the moment the first instance exits.
        //
        // ⭐ **Task T-41-5 hands it out on both roads instead of dropping it here.** It is still
        // closed either way — the guard's `Drop` is what closes it, and `run` lets it go the
        // moment it decides to leave — but the decision of *which* road this is belongs to
        // `run` and not to this function, which knows only that the name was held.
        let instance = Self { handle };

        if last_error == ERROR_ALREADY_EXISTS {
            return Ok(Acquisition::AlreadyRunning(instance));
        }

        Ok(Acquisition::Acquired(instance))
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: `handle` came from a successful CreateMutexW, has not been closed
        // anywhere else — this type is neither `Copy` nor `Clone`, so there is exactly one
        // owner and exactly one close — and is not in use by any wait, because the program
        // never waits on it.
        if let Err(error) = unsafe { CloseHandle(self.handle) } {
            report_non_critical("CloseHandle", &error);
        }
    }
}

/// Whether this second instance leaves **without** the notification of FR-82 — task Т-25-2,
/// finding м-Э24-2, the user's decision 84.2.
///
/// # What this is for
///
/// The notification of FR-82 is a modal `MessageBoxW` and blocks until somebody closes it —
/// decision R-20 point 1, and the right answer for a person who started the program twice. On a
/// bench there is nobody in front of the screen. A second instance raised by a test that then
/// fell over stands on that box for ever: the deadline of FR-97 cannot save it, because a second
/// instance never reaches [`run_as_first_instance`] where the deadline is counted, and even if it
/// did, a modal message loop is not a place a timer thread can reach. Stage Э24 measured the
/// result — `pid 7624`, alive 358 seconds under a deadline of 45.
///
/// # The condition, and why it is this one
///
/// `LANGSW_DEBUG_TIMEOUT_SEC` being set at all. It is the variable every bench in this repository
/// already sets on every launch of the product (decision Р-53: the same variable, no new one), it
/// exists only in a debug build, and a person starting the program by hand does not have it. So
/// "the deadline is armed" is the same fact as "nobody is looking at this window".
///
/// # Release
///
/// This function is `false` and nothing else, with no environment read in it at all. FR-82 is
/// untouched in the shipped product: the same modal notification, the same exit code, the same
/// requirement. The debug half lives inside [`debug_timeout`], so the name of the variable stays
/// where acceptance point 13 of §13 expects to find it — behind `cfg(debug_assertions)`.
#[cfg(debug_assertions)]
fn already_running_is_silent() -> bool {
    debug_timeout::notification_is_suppressed(debug_timeout::deadline_is_armed())
}

/// See the debug half above. In a Release build the notification of FR-82 is never withheld.
#[cfg(not(debug_assertions))]
fn already_running_is_silent() -> bool {
    false
}

/// Tells the user that the program is already running — the "notification" half of FR-82.
///
/// The program is built in the Windows subsystem and owns no console, so the notification
/// can only be a window. Decision R-20 point 1: it stays a modal `MessageBoxW` and the
/// process exits after the user closes it. Blocking is a property of the requirement, not a
/// defect, and self-closing substitutes would not notify anybody.
fn notify_already_running() {
    // The caption is `APP_NAME` itself (decision 7), converted here rather than written out
    // as a second wide literal so that the two can never drift apart. Allocating is fine:
    // this runs once, on the way out of a second instance, nowhere near the hook path where
    // NFR-03 bans allocation.
    let caption: Vec<u16> = crate::APP_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // SAFETY: both string arguments are NUL-terminated UTF-16 buffers that outlive the
    // call — `caption` is owned by this frame and is not moved or dropped until after the
    // call returns, `ALREADY_RUNNING_TEXT` is a `'static` literal — and MessageBoxW only
    // reads through them. `None` for the owner window is the documented way to ask for an
    // unowned box, which is all a process that owns no window yet can do. The call blocks
    // until the user closes the box; that is the point.
    let result = unsafe {
        MessageBoxW(
            None,
            ALREADY_RUNNING_TEXT,
            PCWSTR(caption.as_ptr()),
            MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND,
        )
    };

    if result.0 == 0 {
        // NFR-13: the result is examined. Zero is the documented failure value — out of
        // resources, or an invalid style — and it is not fatal: the process is exiting with
        // EXIT_ALREADY_RUNNING either way.
        report_non_critical("MessageBoxW", &WinError::from_thread());
    }
}

// ---------------------------------------------------------------------------------------
// The three threads of section 6.1
// ---------------------------------------------------------------------------------------

/// Everything that happens once this process is established as the single instance.
fn run_as_first_instance(instance: SingleInstance) -> ExitCode {
    let module = match module_instance() {
        Ok(module) => module,
        Err(error) => {
            report_non_critical("GetModuleHandleW", &error);
            return ExitCode::FAILURE;
        }
    };

    // One class for all three windows. A class registered with `RegisterClassExW` belongs
    // to the process, not to the thread that registered it, so the three threads can create
    // their windows from it; unregistering it here, after every thread has been joined,
    // happens when no window of the class is left.
    let class = match WindowClass::register(module) {
        Ok(class) => class,
        Err(error) => {
            report_non_critical("RegisterClassExW", &error);
            return ExitCode::FAILURE;
        }
    };

    let threads = match spawn_threads() {
        Ok(threads) => threads,
        Err(error) => {
            report_non_critical(&format!("thread::spawn: {error}"), &WinError::from_thread());
            return ExitCode::FAILURE;
        }
    };

    // SEC-04a, task T-03-4-2: the debug control channel, on a thread of its own and only
    // under the `testing` feature — the argument for that thread is in `control::start`.
    // Started after the three threads of section 6.1 rather than before them, because it
    // reports on them and there is nothing to report until they are up.
    //
    // A channel that will not come up is journalled and the program runs on. It is a
    // diagnostic; refusing to run without it would make the diagnostic the most dangerous
    // part of the program.
    #[cfg(feature = "testing")]
    let channel = match crate::control::start() {
        Ok(channel) => Some(channel),
        Err(error) => {
            report_non_critical("control::start", &error);
            None
        }
    };

    // FR-97. In a Release build neither this block nor the module it calls into exists in
    // the compiled code, which is what acceptance point 13 checks by reading the source:
    // the absence of the string `LANGSW_DEBUG_TIMEOUT_SEC` from the Release binary proves
    // nothing on its own, since `strip = true` and `lto = "fat"` would remove it anyway.
    #[cfg(debug_assertions)]
    {
        let expired = debug_timeout::wait_for_deadline();

        request_shutdown();

        if expired {
            // FR-97 as task T-03-1 rewrote it, and the reason is in that task's point 7.
            // Until there was a hook, this path was a convenience: ask the threads to stop
            // and wait for them. With a hook it is the guarantee that a wedged debug build
            // lets go of the keyboard, and "wait for the threads" is exactly the assumption
            // that fails — the UI thread inside `TrackPopupMenuEx` runs a modal message loop
            // of its own and does not leave it for a posted wake-up. Measured before this
            // change: a 45-second deadline, a menu left open, and a process still alive 89
            // seconds in.
            //
            // So this thread, which is the one counting the time and is by construction not
            // blocked, does the two things that matter itself and in this order: the hook
            // comes off first, releasing the keyboard whatever happens afterwards, and only
            // then are the other threads given their chance.
            crate::hook::uninstall();
            debug_timeout::terminate_unless_threads_stop(&threads);
        }
    }

    // A Release build has no deadline: it waits here until something requests shutdown —
    // today the panic hook, from T-01-4 onwards the tray's "Exit" and FR-83.
    let clean = join_all(threads);

    // SEC-04a: the channel comes down after the three threads and not before, so that a bench
    // holding it open sees the program right up to the end. Bounded — see `Channel::stop` —
    // and reached by neither FR-96, which terminates the process from inside the callback, nor
    // by the deadline half of FR-97 above, which has already ended it by this point.
    #[cfg(feature = "testing")]
    if let Some(channel) = channel {
        channel.stop();
    }

    // Instrumentation of the acceptance bench, feature `testing`. Here and nowhere earlier:
    // every thread has been joined, so the numbers are final, and this is the main thread,
    // which owns no window, no hook and no buffer and may therefore touch a file.
    #[cfg(feature = "testing")]
    acceptance::write_report();

    // Order matters and is spelled out rather than left to drop order: every window is gone
    // once the threads are joined, so the class can be unregistered; the mutex is released
    // last, so that no second instance can start while this one still has a window alive.
    drop(class);
    drop(instance);

    if clean {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Starts the three threads of section 6.1, each running [`thread_body`] for its role.
fn spawn_threads() -> std::io::Result<Vec<JoinHandle<WinResult<()>>>> {
    spawn_threads_with(thread_body)
}

/// The wiring behind [`spawn_threads`], with the body of a thread as a parameter.
///
/// **Task T-19-1, finding 3 of the audit of 2026-08-31.** The `request_shutdown` below is the
/// repair, and this split is what makes it checkable: the real [`thread_body`] creates a
/// window, enters a COM apartment and pumps messages, so the guarantee "a thread that ends
/// takes the process with it" could not be driven from a test without it.
///
/// Nothing else about the loop changed. On failure the threads already started are brought
/// down before returning, rather than detached: a detached thread would keep pumping and the
/// process would never exit.
fn spawn_threads_with<F>(body: F) -> std::io::Result<Vec<JoinHandle<WinResult<()>>>>
where
    F: Fn(Role) -> WinResult<()> + Clone + Send + 'static,
{
    let mut threads = Vec::with_capacity(THREAD_COUNT);

    for role in [Role::Input, Role::Ui, Role::Watcher] {
        let body = body.clone();

        let spawned = thread::Builder::new()
            .name(role.thread_name().to_owned())
            .spawn(move || {
                let outcome = body(role);

                // **The repair of task T-19-1.** Whichever thread this is and however its body
                // ended, the process is on its way out: ask the rest to stop as well, here, on
                // the thread that is leaving, rather than leaving it to whoever gets round to
                // joining this handle.
                //
                // Until this line the only place that asked was `join_all`, which joins in
                // order and so does not look at the UI or the watcher handle until the input
                // thread — the one that in a normal session pumps to the very end — has
                // returned. A watcher that failed to create its window, to enter its apartment
                // or to subscribe `watchdog::watch` therefore left the process running for the
                // rest of the session with no password probe of SEC-06 and no flush of FR-10;
                // a UI thread that failed left it with no tray icon and no way out. A panic
                // was covered — the hook of FR-98 asks — an ordinary `Err` was not.
                //
                // It is also what wakes the main thread of a debug build, which spends the
                // session parked on the FR-97 deadline and not in `join_all` at all:
                // `request_shutdown` unparks it.
                //
                // Idempotent and safe from any thread, so the second request `join_all` makes
                // costs an atomic store. A body that panics does not reach this line and does
                // not need to: the panic hook has already asked.
                request_shutdown();

                outcome
            });

        match spawned {
            Ok(handle) => threads.push(handle),
            Err(error) => {
                request_shutdown();
                join_all(threads);
                return Err(error);
            }
        }
    }

    Ok(threads)
}

/// Waits for every thread and reports whether all of them ended cleanly.
fn join_all(threads: Vec<JoinHandle<WinResult<()>>>) -> bool {
    let mut clean = true;

    for handle in threads {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                clean = false;
                report_non_critical("thread body", &error);
            }
            Err(_payload) => {
                // The panic hook (FR-98) has already run on the panicking thread and has
                // already asked everyone to come down. The payload is dropped unread on
                // purpose: SEC-01 and SEC-07 forbid moving panic text anywhere it could be
                // stored, and there is nothing this frame could do with it anyway.
                clean = false;
            }
        }

        // A second, unconditional request, kept after task T-19-1 moved the first one onto the
        // thread that is leaving. It costs an atomic store and it covers the one caller that
        // does not come through `spawn_threads_with`: the failure path of that function, which
        // joins the handles it did manage to start.
        request_shutdown();
    }

    clean
}

/// The body of the watcher thread, with what it serves passed in — `serve_window(Role::Watcher)`
/// in the program, and a body that panics in the test of task T-37-2.
///
/// # ⭐ Two guards, in the order that releases the probe's client inside its apartment
///
/// The watcher thread is the only COM apartment in the process, and it is an STA: section 6.1 puts
/// the level-3 probe of FR-72 here, and that probe requires a single-threaded apartment. Its client
/// lives in a thread-local of this thread (module `guard`), and a COM interface must be released
/// inside the apartment it was created in; a thread-local destructor runs at thread exit, **after**
/// the `CoUninitialize` in `ComApartment::drop`.
///
/// Until task T-37-2 (finding Н36) the release was a line after `serve_window`, and a panic on this
/// thread unwound past it: the apartment was left first and the client was released into an
/// apartment that no longer existed — undefined behaviour wherever a panic unwinds. The release is
/// now `guard::ProbeClientRelease`, declared **after** the apartment and therefore dropped
/// **before** it — an order the language guarantees on a return, on a `?` and on an unwinding
/// alike, which the position of a line never did. The explicit call is gone, not kept beside the
/// guard.
///
/// Split out of [`thread_body`] so that a test can make `serve` panic and read the order the two
/// guards ran in — `ComApartment::drop` and the guard's release each write it down, in tests only.
fn watcher_body(serve: impl FnOnce() -> WinResult<()>) -> WinResult<()> {
    let _apartment = ComApartment::enter_sta()?;
    let _probe_client = crate::guard::ProbeClientRelease::on_this_thread();

    serve()
}

#[cfg(test)]
thread_local! {
    /// The steps of a thread's teardown, in the order they ran — tests of this crate only, task
    /// T-37-2. Per thread, so that two tests reading it cannot read each other.
    static TEARDOWN: std::cell::RefCell<Vec<&'static str>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Writes one step of the teardown of the calling thread into its journal — tests only.
///
/// `try_with`: it is called from a `Drop` that may be running while a panic unwinds, and a journal
/// that is already gone must not become a second panic.
#[cfg(test)]
pub(crate) fn note_teardown(step: &'static str) {
    let _ = TEARDOWN.try_with(|journal| journal.borrow_mut().push(step));
}

/// The body of every one of the three threads.
fn thread_body(role: Role) -> WinResult<()> {
    match role {
        // The watcher thread is the only COM apartment in the process, and it is an STA:
        // section 6.1 puts UI Automation here, and UI Automation requires a single-threaded
        // apartment. The apartment and the release of the probe's client are two guards of
        // [`watcher_body`] since task T-37-2 — see there for why their order is the point.
        Role::Watcher => watcher_body(|| serve_window(role)),
        // Second of the three edits of task T-06-4. Section 6.1 gives file input-output to
        // the UI thread and forbids it to the input thread, so the one place the journal may
        // reach a file is here, on this thread, once, after `serve_window` has returned.
        //
        // After and not inside: the tray attachment, the window and **this thread's**
        // subscriptions are undone by `Drop` implementations *inside* that call, so what they
        // report is in the ring before the file is written. ⚠ **What is NOT in it — task
        // T-34-7, finding Н99:** the hook comes off on the input thread and the system
        // subscriptions of FR-80 on the watcher thread, and both are joined by `join_all` on
        // the main thread *after* this thread has returned. Whatever those two record while
        // they come down lands in the ring **after** the dump, and reaches no file: the last
        // dump of a session is a picture of the UI thread's shutdown, not of the whole
        // program's. Moving the dump behind `join_all` would put file input-output on the main
        // thread, which section 6.1 does not allow — so the order stays and the sentence says
        // what it means (вариант 2 of Н99, решение 117).
        //
        // Off unless the session's published `[diagnostics] log_enabled` says otherwise —
        // section 7, task T-34-2 — in which case this creates nothing at all.
        Role::Ui => {
            let served = serve_window(role);

            crate::diag::dump_on_shutdown();

            served
        }
        Role::Input => serve_window(role),
    }
}

/// Creates this thread's window and pumps its messages until shutdown.
fn serve_window(role: Role) -> WinResult<()> {
    let instance = module_instance()?;
    let _window = Window::create(role, instance)?;

    // Task T-01-4. Section 6.1 puts the tray on the UI thread and nowhere else, and this is
    // the only place it is installed, so no other thread can reach it: `tray` keeps it in
    // thread-local storage, and on the input and watcher threads that slot stays empty.
    //
    // Declared after `_window` so that it is dropped *before* it: dropping the attachment is
    // the cleanup path of FR-83, and removing the icon needs the window it was added under
    // to still exist.
    let _tray = match role {
        Role::Ui => {
            let attachment = crate::tray::attach(_window.handle, instance)?;

            // **FR-94, task T-08-2, rewritten by task Т-31-1 — the first publication of the
            // interface locale, through the body every later one goes through too.**
            //
            // The tray has just read `config.toml`; this is where `general.language` reaches
            // the module that turns string identifiers into text. Until решение 99 it was the
            // *only* publication in the life of the process, because the note beside the
            // language combo box promised a restart. The note is gone and the promise with it:
            // «Применить» publishes the locale as well, and it does so by calling this very
            // function ([`crate::tray::adopt_ui_language`]) — one body, not two.
            //
            // It is still deliberately *not* inside `publish_configuration` below. That
            // function hands settings to the modules that act on them; the locale is published
            // into a module that renders, and it travels with the tooltip of the icon, which
            // only the tray can touch.
            crate::tray::adopt_ui_language();

            // Section 6.3, "Конфигурация публикуется потоком UI": the tray has just read
            // `config.toml`, and this is the moment the input thread learns what is in it. It
            // cannot read the file itself — NFR-08 gives the hook fifty milliseconds from
            // start-up and a file read is exactly the kind of thing task T-03-1 was told to
            // keep off that path.
            publish_configuration_to_input_thread();

            Some(attachment)
        }
        Role::Input | Role::Watcher => None,
    };

    // Task T-03-1, FR-01: one `WH_KEYBOARD_LL` hook, on the input thread, and on no other.
    // The window above already exists, which is what the hook needs in two ways — the
    // callback posts `hook::WM_APP_HOTKEY` to it (FR-02), and a low-level hook is only
    // called back on a thread that pumps messages, which `pump` below does.
    //
    // Declared after `_window` so that it is dropped *before* it: the callback may post to
    // that window until the moment the hook comes off, so the window has to outlive the hook
    // and not the other way round.
    let _hook = match role {
        Role::Input => {
            // Task T-36-3: at start-up the belief about a held hotkey is fresh — this is the
            // first `install` of the process — so `Forget` is a no-op said out loud. It is the
            // same answer `watchdog::clears_hotkey_state` gives for `Reason::None`, and saying
            // it here keeps the one door of installation from having a default.
            let installed =
                crate::hook::install(_window.handle, instance, crate::hook::HotkeyMemory::Forget)?;

            // NFR-08 is a deadline on exactly this instant, and it is measured here rather
            // than reasoned about: see the `acceptance` module for why the program has to be
            // the one holding the stopwatch.
            #[cfg(feature = "testing")]
            crate::control::note_hook_installed();

            // Task T-03-2a, and deliberately **after** the hook. See `start_input_pipeline`.
            start_input_pipeline();

            Some(installed)
        }
        Role::Ui | Role::Watcher => None,
    };

    // Task T-03-3: the subscriptions of module `watchdog`. Declared after `_window` for the
    // same reason `_tray` and `_hook` are — each of them names that window and each must be
    // undone before the window stops existing — and after `_hook` because NFR-08 measures the
    // distance from start-up to the installed hook and nothing that is not the hook belongs
    // in front of it.
    //
    // Section 6.1 decides which thread gets which and the `match`es below are that table:
    // `RegisterRawInputDevices (RIDEV_INPUTSINK)` is listed under the input thread, and
    // `SetWinEventHook` under the watcher thread. A failure of either ends the thread and so
    // the program: FR-13 and two rows of the FR-10 flush table are not optional extras, and a
    // program that silently stopped flushing the buffer on a click would be worse than one
    // that refuses to start.
    let _raw_input = match role {
        Role::Input => Some(crate::watchdog::register_raw_input(_window.handle)?),
        Role::Ui | Role::Watcher => None,
    };

    // **FR-21, the device half — task T-08-4.** Until that task this was the second entry of the
    // registration above, `HID_USAGE_KEYBOARD` with `RIDEV_DEVNOTIFY`, and it was measured to
    // take keyboard input away from every low-level hook in the session for as long as any window
    // of this process was in front — the settings dialog of FR-92 among them, which is how FR-96
    // came to be unreachable exactly when the user needed it. Module `watchdog` documents the
    // measurement.
    //
    // The same news now arrives as the message FR-21 actually names. It goes to the **input**
    // window because the cache the rebuild touches is a thread-local of that thread (section 6.3)
    // and because a registered device notification reaches a message-only window, where the
    // broadcast kind would not.
    //
    // Declared after `_window` for the reason every guard here is: the registration names that
    // window and must be withdrawn before the window stops existing. A failure ends the thread
    // and so the program, exactly as the raw input registration's does: FR-21 is not an optional
    // extra, and a program that silently stopped noticing new keyboards would be worse than one
    // that refuses to start.
    let _device_notice = match role {
        Role::Input => Some(crate::watchdog::register_device_notice(_window.handle)?),
        Role::Ui | Role::Watcher => None,
    };

    let _win_events = match role {
        Role::Watcher => Some(crate::watchdog::watch()?),
        Role::Input | Role::Ui => None,
    };

    // Task T-06-2, FR-80. The two halves of the watchdog that need a window of their own, and
    // each goes to the only window that can carry it.
    //
    // `WM_WTSSESSION_CHANGE` and `WM_POWERBROADCAST` are delivered to **top-level** windows, and
    // the UI thread's is the only one this program has — the input and watcher windows are
    // `HWND_MESSAGE` children (see `Window::create`). That is the same rake FR-81 stood on with
    // `RegisterWindowMessage("TaskbarCreated")`, and decision R-20 point 2 is why the UI window
    // is a hidden top-level window rather than a message-only one.
    //
    // The liveness timer goes on the **input** window, because section 6.1 writes "таймер
    // сторожа" under the input thread and because a `WH_KEYBOARD_LL` hook is called back on the
    // thread that installed it: the reinstallation the timer causes has to happen there.
    //
    // Declared after `_window` for the reason `_tray`, `_hook` and the two above are — each
    // names that window and each must be undone before the window stops existing. A failure of
    // either ends the thread and so the program: a resident program without a watchdog is one
    // that will one day stop responding to its own hotkey and say nothing about it, which FR-80
    // exists to prevent.
    let _session_notice = match role {
        Role::Ui => Some(crate::watchdog::register_session_notice(_window.handle)?),
        Role::Input | Role::Watcher => None,
    };

    let _liveness = match role {
        Role::Input => Some(crate::watchdog::start_liveness_timer(_window.handle)?),
        Role::Ui | Role::Watcher => None,
    };

    // **FR-63 — task T-07-1.** `AddClipboardFormatListener` posts `WM_CLIPBOARDUPDATE` to the
    // window named in the call rather than broadcasting it, so any of the three would receive
    // it; the choice is made on which thread is allowed to *do* the clipboard work, because
    // registering here is also what publishes that thread to module `selection`.
    //
    // The **UI** thread, and not the other two. The input thread owns the `WH_KEYBOARD_LL` hook
    // (FR-01) and a clipboard access costs up to 180 ms of FR-62 retries plus the 300 ms wait of
    // FR-61 step 3 — three thousand times NFR-01, on the one thread FR-80 removes the hook from
    // when it stops answering. The watcher thread owns the focus probe of FR-71 and FR-72, on
    // whose answer SEC-06 holds the typing buffer switched off, and lengthening that window is
    // paid for in the user's keystrokes. Section 6.1 already gives the UI thread the process's
    // slow, deadline-free work — «Чтение и запись конфигурации» — and nothing on a budget.
    //
    // Declared after `_window` for the reason every guard above it is: the registration names
    // that window and must be withdrawn before the window stops existing.
    //
    // A failure is **not** fatal, unlike the watchdog subscriptions above. FR-65 makes the
    // selection path optional by configuration, so a program that could not register a clipboard
    // listener is a program with one feature fewer, not one that must refuse to start; the
    // journal takes the reason (NFR-13) and the rest of the process carries on.
    let _clipboard = match role {
        Role::Ui => crate::selection::listen(_window.handle)
            .inspect_err(|error| report_non_critical("AddClipboardFormatListener", error))
            .ok(),
        Role::Input | Role::Watcher => None,
    };

    // Re-read the flag after the window has been published, and before the first
    // `GetMessageW`. This closes the only race in the shutdown design: a `request_shutdown`
    // that ran before the window existed found `NO_WINDOW`, posted nothing, and would
    // otherwise leave this thread pumping for ever. The flag is stored before the wake-ups
    // are posted, so either that store is visible here or the post reached the queue.
    //
    // ⭐ **Task Т-22-7: that last sentence is a conclusion, and until this task the code did not
    // support it.** It is the store-buffer litmus — this read against `request_shutdown`'s read
    // of `WAKE_TARGETS`, with the two publications crossing — and Release/Acquire permits the one
    // outcome the sentence rules out. All four operations of the pair now run on `SeqCst`; the
    // reasoning is written out once, in `request_shutdown`.
    if shutdown_requested() {
        return Ok(());
    }

    let pumped = pump();

    // Latched before the frame unwinds: the buffer belongs to this thread and is dropped — and
    // zeroed, SEC-02 — with it, so after this function returns the live mirror of SEC-04a reads
    // a truthful zero and the file sink, which runs later still, would have nothing to report.
    #[cfg(feature = "testing")]
    if matches!(role, Role::Input) {
        crate::control::note_buffer_len_at_exit();
    }

    pumped
}

// ---------------------------------------------------------------------------------------
// The input path — FR-07, FR-20, FR-21, FR-25, FR-11 (task T-03-2a)
// ---------------------------------------------------------------------------------------

/// Everything the input thread owns besides the hook: the typing buffer of FR-07, the layout
/// cache of FR-20 and the active layout of FR-04.
///
/// # Why this runs after the hook and not before it
///
/// NFR-08 gives the program under fifty milliseconds from start-up to an installed hook, and
/// building the cache is not a constant: FR-20 sweeps virtual keys `0x08..=0xFF` against eight
/// modifier combinations **for every layout loaded in the session**, so its cost is thousands
/// of `ToUnicodeEx` calls multiplied by a number the user chooses. Measured values are in the
/// report of task T-03-2a; the argument does not rest on them, because no measurement on one
/// machine can bound a quantity that grows with somebody else's layout list.
///
/// So the order is fixed the way the task specification fixes it: **the hook first**. Without
/// the cache the program still records every stroke — by scan code, which is what section 4.1
/// says the buffer is for — and merely has no characters to put beside them until the sweep
/// finishes. Without the hook it is not a program at all. The price of this order is that the
/// input thread does not reach `GetMessageW` until the sweep is over, so a keystroke made in
/// that window is *delayed* by up to the sweep's duration rather than lost; the alternative
/// order would have lost it outright, and would have missed NFR-08 to do it.
///
/// The buffer goes up between the two, because it is one allocation of eight kilobytes and
/// because a cache published into a buffer that does not exist would be dropped on the floor.
fn start_input_pipeline() {
    // FR-07.
    install_buffer();

    // FR-04, the `hkl` every stroke is stored under. FR-11 rests on it being per stroke.
    publish_active_layout(foreground_layout());

    // FR-20, and FR-25 if it fails.
    rebuild_layout_cache();

    #[cfg(feature = "testing")]
    crate::control::note_cache_ready();

    // FR-07 again. The sweep above takes long enough that the UI thread has almost always
    // published `[buffer] capacity` by now; this applies it without waiting for the message
    // that would otherwise be the only thing that does.
    apply_configured_capacity();
}

/// Installs the typing buffer of FR-07 on the calling thread.
///
/// The capacity is whatever the UI thread has published, and the default of section 7 when it
/// has published nothing yet — [`BUFFER_CAPACITY`] and [`crate::buffer::effective_capacity`]
/// agree that zero means the default, so there is no third state to handle here.
///
/// ⭐ **Point 1 of task T-13-4.** A buffer is born believing `CapsLock` is off — `Held::caps`
/// starts `false` and the tracker only ever learns from presses that reach `Recorder::record`,
/// which the presses made before this moment did not. A user who starts the program with
/// `CapsLock` on, and the autostart of FR-93 does it for them at every logon, would otherwise
/// have every stroke of the session recorded under a cleared `CAPS` bit: the cache of FR-20 is
/// keyed on it, and the replacement of FR-22 goes out through `KEYEVENTF_UNICODE`, which carries
/// the character literally and knows nothing of the real `CapsLock`. «GHBDTN» would come back as
/// «привет» instead of «ПРИВЕТ», silently, until the process was restarted.
///
/// Seeded here rather than inside `buffer::install` so that the Win32 reading stays a decision
/// of the product's start-up path: `buffer::install` is a plain constructor a test may call, and
/// this is the one place the *input thread* creates the buffer it will type into.
fn install_buffer() {
    crate::buffer::install(&settings::Buffer {
        capacity: BUFFER_CAPACITY.load(Ordering::Acquire),
        // FR-15, task T-52-4 — see [`configured_idle_timeout_s`] for the "nothing published
        // yet" case, which is section 7's own default and not a third state.
        idle_timeout_s: configured_idle_timeout_s(),
    });

    crate::buffer::set_caps_lock(crate::hook::caps_lock_on);
}

/// Resizes the buffer to the capacity the UI thread published, if it is not that size already.
///
/// Called from the window procedure on every message, which is what makes the message a
/// wake-up rather than a command (SEC-05), and once more at the end of the start-up pipeline,
/// which is what makes a post that arrived before the window existed harmless.
fn apply_configured_capacity() {
    apply_capacity(BUFFER_CAPACITY.load(Ordering::Acquire));
    apply_idle_timeout(BUFFER_IDLE_TIMEOUT_S.load(Ordering::Acquire));
}

/// `[buffer] idle_timeout_s` as it stands now — the published value, or section 7's default
/// while the UI thread has not read the file yet. **FR-15**, task T-52-4.
fn configured_idle_timeout_s() -> u32 {
    match BUFFER_IDLE_TIMEOUT_S.load(Ordering::Acquire) {
        IDLE_TIMEOUT_UNPUBLISHED => settings::Buffer::default().idle_timeout_s,
        published => published,
    }
}

/// Applies `[buffer] idle_timeout_s` to the buffer of this thread — **FR-15**, task T-52-4.
///
/// The sibling of [`apply_capacity`] and it runs beside it, on every message of the input
/// window, for the same three reasons: the message is a wake-up rather than a command (SEC-05),
/// a post that arrived before the window existed is harmless because the start-up pipeline
/// reads the atomics once more at the end, and re-reading a published value costs one relaxed
/// load and one comparison.
///
/// ⚠ **Unlike a capacity change, this one throws nothing away.** [`Recorder::set_capacity`]
/// rebuilds the ring, so `apply_capacity` has to compare before it acts or it would empty the
/// buffer on every message; the timeout is one number and setting it again is free. The
/// comparison below is therefore for tidiness rather than for safety.
fn apply_idle_timeout(published: u32) {
    if published == IDLE_TIMEOUT_UNPUBLISHED {
        // Nothing published yet. The buffer was installed on section 7's default and that is
        // still the right answer.
        return;
    }

    crate::buffer::with(|recorder| {
        if recorder.idle_timeout_s() != crate::buffer::effective_idle_timeout_s(published) {
            recorder.set_idle_timeout_s(published);
        }
    });
}

/// The half of [`apply_configured_capacity`] that does not read the atomic — see there.
///
/// Separate so that the rule can be driven from a test without a second thread and without
/// touching process-global state.
fn apply_capacity(requested: usize) {
    if requested == 0 {
        // Nothing published yet. Not a reason to touch a buffer that is already the right size
        // by the defaults of section 7.
        return;
    }

    crate::buffer::with(|recorder| {
        // The comparison is against the *effective* capacity and not against the raw number,
        // because the buffer clamps what it is asked for: comparing the raw `100000` against
        // the `4096` the buffer really has would differ for ever, and this runs on every
        // message, so the buffer would be rebuilt — and emptied — again and again.
        if recorder.capacity() != crate::buffer::effective_capacity(requested) {
            recorder.set_capacity(requested);
        }
    });
}

/// Builds the layout cache of FR-20 and publishes it into the buffer.
///
/// ⚠ Never from the hook callback. FR-20 is thousands of `ToUnicodeEx` calls and NFR-01 gives
/// the callback a hundred microseconds; module `layouts` says the same in its own threading
/// contract. Both call sites are on the input thread outside the callback: the start-up
/// pipeline and the window procedure of FR-21.
fn rebuild_layout_cache() {
    // Finding Н7 — task T-39-4: whether a failure has a working cache to spare, asked of the
    // buffer wherever FR-70 keeps it and handed to the decision as an argument. At start-up the
    // buffer holds no cache yet, so the table of FR-25 arrives there exactly as before.
    let keeping = with_recorder_wherever_it_is(|recorder| recorder.has_cache()).unwrap_or(false);
    let built = LayoutCache::build();

    // Finding Н10 — task T-39-6, decision 122г: the layouts the table of FR-25 is filed under,
    // read only when the table is about to be used. A failed enumeration answers an empty list,
    // and the table then carries the two identifiers it always did.
    let session = if built.is_err() && !keeping {
        crate::layouts::enumerate().unwrap_or_default()
    } else {
        Vec::new()
    };

    if let Some(cache) = cache_or_fallback(built, keeping, &session) {
        publish_cache(cache);
    }

    // Instrumentation, feature `testing`. It is what lets the acceptance run tell a rebuild
    // that happened from one that was merely wired up: FR-21 has no visible effect of its own,
    // and FR-11 — that the rebuild left the buffer alone — can only be asserted against a
    // rebuild that is known to have run.
    #[cfg(feature = "testing")]
    crate::control::note_cache_built();
}

/// The cache to use, given what [`LayoutCache::build`] answered and whether the buffer already
/// holds one — FR-25, and **finding Н7, task T-39-4**.
///
/// A failure to build is **not** a reason to end the program, and not a reason to run without a
/// cache either: FR-25 has module `convert` carry a hardwired RU/EN table for exactly this, and
/// point 4 of it is why `LayoutCache` refuses to be empty — "the cache did not build" has to
/// stay distinguishable from "these keys carry no characters".
///
/// ⭐ **But the table is for a buffer that has nothing better, and only then.** Until task T-39-4
/// a failed *rebuild* replaced a working cache with it as well: every layout but the RU/EN pair
/// stopped existing, a cycle of three broke, and nothing said so — not the rule
/// [`LayoutCache::rebuild`] states for itself, that a failed rebuild leaves the previous cache as
/// it was. Now `keeping` — "the buffer already holds a cache" — makes a failure answer `None`:
/// publish nothing, keep what works. At start-up there is nothing to keep, and the table arrives
/// exactly as before. Every failure is counted either way.
///
/// ⭐ **And the table is filed under the session's own layouts — finding Н10, task T-39-6.**
/// `session` is the enumeration of FR-35 (empty when there is none to read), and
/// [`crate::convert::fallback_cache_for`] files the table under its layouts of English and
/// Russian, so that a buffer recording under «США — международная» finds its active layout in
/// the reserve rather than nothing.
///
/// `keeping` and `session` are arguments rather than reads of the buffer and of the system, so
/// that the decision reads no global state and can be driven from a test;
/// [`rebuild_layout_cache`] asks both.
fn cache_or_fallback(
    built: Result<LayoutCache, LayoutError>,
    keeping: bool,
    session: &[LayoutId],
) -> Option<LayoutCache> {
    match built {
        Ok(cache) => Some(cache),

        // SEC-01, SEC-07: the reason is dropped unread rather than formatted. `LayoutError`
        // carries no keystroke, and the rule of this program is still that nothing on this path
        // becomes a string. What is kept is the count — of every failure, cache kept or not.
        Err(_reason) => {
            LAYOUT_CACHE_FAILURES.fetch_add(1, Ordering::Relaxed);

            (!keeping).then(|| crate::convert::fallback_cache_for(session))
        }
    }
}

/// Hands the cache to the buffer of the calling thread — FR-20, FR-21.
///
/// ⚠ **FR-11: this does not flush the buffer.** `Recorder::set_cache` re-resolves the position
/// of the active layout and touches nothing else, which is the whole of what a rebuild owes the
/// buffer. See [`publish_active_layout`] for the other half of the same rule.
///
/// Through [`with_recorder_wherever_it_is`] since task T-10-0f, so that a rebuild the probe of
/// FR-21 asked for during the interval of FR-71 lands in the parked recorder instead of being
/// swept and thrown away — see there for the measurement.
fn publish_cache(cache: LayoutCache) {
    with_recorder_wherever_it_is(|recorder| recorder.set_cache(cache));
}

/// Publishes the layout strokes are recorded under — the `hkl` of FR-04.
///
/// ⚠ **FR-11: "смена раскладки не сбрасывает буфер".** The user switching layout in the middle
/// of a word is a normal thing to do and not a signal that what came before is void; every
/// stroke carries the layout it was typed under precisely so that nothing has to be thrown
/// away. There is no `reset` on this path and there must never be one.
///
/// Through [`with_recorder_wherever_it_is`] since task T-10-0f: the probe of FR-21 arrives
/// during the interval of FR-71, when the recorder is parked, and the layout it carries is
/// exactly the value the parked recorder must wake up with — the stale one is what stamped
/// strokes with a direction that converted «в себя». See there for the measurement.
/// ⚠ **Task T-10-5 — the stamp is mirrored onto the channel of SEC-04a from here.** This is
/// the one place in the program that writes `Recorder::active`, so it is the one place that
/// can say what the stamp holds; FR-26 computes the direction of every conversion from that
/// value, and until this key the channel could only show *counts* of the events that were
/// supposed to refresh it. The mirror is written only when the write really landed in a
/// recorder — `None` means this thread owns neither an installed nor a parked one, and
/// publishing a layout no recorder took would be a reading of nothing. See
/// [`crate::control::note_active_layout`].
fn publish_active_layout(layout: LayoutId) {
    let landed = with_recorder_wherever_it_is(|recorder| recorder.set_active_layout(layout));

    #[cfg(feature = "testing")]
    if landed.is_some() {
        crate::control::note_active_layout(layout.raw());
    }

    let _ = landed;
}

/// **Step 5 of FR-40 has just moved the layout, and the stamp of FR-04 follows it** — task
/// **T-10-5**.
///
/// Called from `inject::System::switch_layout`, on the input thread, once
/// [`crate::switch::stamp_follows`] has said this program's model of the layout may be moved to
/// `layout`. That function is where the *rule* lives and this is where the *effect* does: the
/// whole of what happens here is [`publish_active_layout`], which is the same publication the four
/// probe paths of FR-21 make and which touches nothing but the stamp.
///
/// ⚠ The gate used to be [`crate::switch::confirmed`], and task **Т-14-6** moved it one predicate
/// across on the user's decision of 2026-08-25. The difference is the classic console window,
/// where FR-52's addendum says no verdict can be taken at all: the switch is believed there rather
/// than verified, on the 60 of 60 Т-14-2 measured from outside. `confirmed` still means what it
/// always meant, and `switch::stamp_follows` documents why the two are separate words.
///
/// # Why this exists rather than a re-read of the foreground layout
///
/// Because a re-read is either redundant or impossible, and never useful. For the confirmed
/// outcomes `switch::to` has already done it — decision R-32 makes the verdict of FR-50 a
/// re-reading of FR-52 rather than a return value believed on trust, so the layout has been
/// observed to *be* the target. For the console outcome the re-read is what cannot be had at all,
/// and asking again would answer the same `0` FR-52's addendum is about. Either way a second round
/// of Win32 calls would be spent on the path NFR-09 budgets, and it would open a window in which a
/// layout the user changed in between is mistaken for the one this switch produced.
///
/// # Why it is a function of this module
///
/// `publish_active_layout` reaches the recorder through [`with_recorder_wherever_it_is`], which
/// is a pair of thread-locals of the input thread (FR-70, FR-71) and belongs here. Module
/// `inject` asks for the publication and does not perform it, exactly as it asks module `switch`
/// for the switch and does not perform that either.
///
/// ⚠ **FR-11: nothing here flushes the buffer**, and nothing that does may ever be added. See
/// [`publish_active_layout`].
pub fn note_layout_switched(layout: LayoutId) {
    publish_active_layout(layout);
}

/// Re-reads the layout of the foreground window and the layout list of the session and follows
/// them — the stamp the first, the cache the second — **the delivery of FR-21**, task T-03-3.
///
/// # Why the program has to ask instead of being told
///
/// FR-21 rebuilds the cache "по сообщениям `WM_INPUTLANGCHANGE` и `WM_DEVICECHANGE`", and task
/// T-03-2a established by measurement that neither message reaches this program:
/// `WM_INPUTLANGCHANGE` is sent to the window with the keyboard **focus**, and the windows the
/// user switches layout in are other people's. The handler was written and correct; it was never
/// called, and the cache was therefore built once at start-up and never rebuilt.
///
/// ⚠ Task **T-08-4** corrected the other half of that sentence, which used to read "all three
/// windows of this process are hidden and never focused". They are not: the UI window takes the
/// foreground before the tray menu of FR-91, and the settings dialog of FR-92 is a window of ours
/// that holds it for as long as the user has it open. That mistaken assumption cost this program
/// every keystroke in that state, FR-96 included — module [`crate::watchdog`] documents the
/// measurement. It changes nothing about `WM_INPUTLANGCHANGE`, which still cannot arrive for the
/// window the user is *typing* into, and everything below stands.
///
/// The `WM_DEVICECHANGE` half is no longer in this position at all: since task T-08-4 the real
/// message arrives at the input window, and the branch of [`window_proc`] that
/// `layouts::needs_rebuild` guards is what answers it.
///
/// The layout is a property of the **thread that owns the foreground window**, and that is what
/// makes the question answerable without the message:
/// `GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow()))` is what
/// [`foreground_layout`] already computed for FR-04. Module `watchdog` provides the moments worth
/// asking at — `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS`, the two events the FR-10 table
/// already has this program subscribed to — and both arrive here as
/// [`crate::watchdog::WM_APP_LAYOUT`].
///
/// The known limit of asking at those two moments is written down rather than left implicit: a
/// user who switches layout with `Alt+Shift` **without leaving the window they are typing in**
/// changes no foreground and moves no focus, so the program learns of it at the next window or
/// focus change and not before. The two mechanisms that would close that gap were measured and
/// rejected in the report of task T-03-3 — `RegisterShellHookWindow` does not deliver
/// `HSHELL_LANGUAGE` in its documented shape on this system, and the TSF sink that would is a
/// COM interface needing the `implement` feature of the `windows` crate, which is outside
/// section 3.2 of SPEC and therefore the controller's decision, not this task's.
///
/// # Why the rebuild is conditional, and on what — finding Н11, task T-39-4
///
/// FR-20 is a sweep of virtual keys `0x08..=0xFF` against eight modifier combinations for every
/// layout in the session — thousands of `ToUnicodeEx` calls, five milliseconds on the machine
/// this was measured on. `EVENT_SYSTEM_FOREGROUND` fires on every `Alt+Tab`, and the window the
/// user switched to is nearly always running the same layout as the one they left, so rebuilding
/// unconditionally would pay the whole sweep for nothing many times a minute.
///
/// ⚠ **Until task T-39-4 the condition was the wrong one.** The rebuild was decided by the same
/// comparison as the stamp — «is the window's layout the one the buffer records under?» — which is
/// a question about the *active* layout, while the cache is a function of the *list*. So every
/// `Alt+Shift` paid the full sweep over a list that had not changed, and a layout added to the
/// session without touching the active one was never taken in until the program restarted.
/// Measured in process, before the repair, by the test
/// `the_probe_of_fr21_rebuilds_for_a_changed_list_and_not_for_a_changed_window`: one rebuild
/// wasted and one missing. Now the stamp keeps its comparison — [`layout_refresh_needed`] — and
/// the cache is rebuilt when [`crate::layouts::layout_list_changed`] says the enumeration of
/// FR-35 is no longer the list the cache holds: a layout added, removed or moved. One enumeration
/// per probe, two Win32 calls on this thread's message loop, and the sweep only when it buys
/// something.
///
/// A zero answer — no foreground window at all, which happens while the desktop switches and on
/// the secure desktop — is not a change and is not treated as one: publishing it would tell the
/// buffer to record under a layout no cache contains.
///
/// ⚠ **FR-11: nothing here flushes the buffer**, and nothing that does may ever be added. The
/// buffer *is* flushed on a foreground change, but by the row of the FR-10 table that says so
/// and through [`crate::watchdog::apply_flush`], which is a different event that happens to
/// arrive at the same time.
///
/// ⚠ **Task T-10-0f: the recorder is read and written wherever FR-70 keeps it.** The probe
/// behind `WM_APP_LAYOUT` is posted in the same breath as the `WM_APP_FLUSH` of the very focus
/// change it is about, and answering that flush **parks** the recorder until the verdict of
/// FR-71 comes back — so this function runs precisely inside the interval in which
/// `buffer::with` answers `None`. Reading the recorded layout through `buffer::with` here made
/// every focus-change probe compare against nothing and refresh nothing, measured live as
/// `layout_probes=1` against `window_flushes=12` and a first press that converted a stale
/// direction «в себя»; see [`with_recorder_wherever_it_is`] for the measurement that pinned it.
fn refresh_layout_and_cache() {
    let observed = foreground_layout();

    // FR-04: the stamp follows the window, exactly as it always did.
    if layout_refresh_needed(
        observed,
        with_recorder_wherever_it_is(|recorder| recorder.active_layout()),
    ) {
        publish_active_layout(observed);
    }

    // ⭐ Finding Н11 — task T-39-4: the cache follows the *list*. See the documentation above.
    let Ok(session) = crate::layouts::enumerate() else {
        // Nothing to compare against, and nothing a sweep could do better: `LayoutCache::build`
        // starts with this very enumeration. The next probe asks again.
        return;
    };

    let cached = with_recorder_wherever_it_is(|recorder| {
        recorder.cache().map(|cache| {
            cache
                .maps()
                .iter()
                .map(|map| map.layout())
                .collect::<Vec<_>>()
        })
    });

    match cached {
        // No buffer on this thread: nothing here owns a cache to rebuild.
        None => {}
        // The list the cache was built from is the list the session has.
        Some(Some(cached)) if !crate::layouts::layout_list_changed(&session, &cached) => {}
        // A layout was added, removed or moved — or there is no cache to compare yet.
        Some(_) => rebuild_layout_cache(),
    }
}

/// The device half of FR-21 at one window — `WM_DEVICECHANGE`, and `WM_INPUTLANGCHANGE` beside it
/// in [`crate::layouts::REBUILD_MESSAGES`] — **finding Н25, task T-39-5**.
///
/// `at_the_input_window` is [`is_input_window`] as the window procedure asks it. It is an argument
/// so that the rule can be driven from a test without a window of the input thread; the rebuild
/// itself reaches a buffer parked by FR-70 through [`with_recorder_wherever_it_is`], like every
/// other publication of FR-21.
fn answer_device_change(message: u32, at_the_input_window: bool) {
    if crate::layouts::needs_rebuild(message) && at_the_input_window {
        // Task T-08-4: the count that lets a run *show* FR-21 being delivered rather
        // than assert it. It answers `false` for `WM_INPUTLANGCHANGE`, which is the
        // other message of the list and is not a device change; the result is dropped
        // because the rebuild below happens for both alike. SEC-07 — a count of events,
        // never a device name.
        let _ = crate::watchdog::note_device_change(message);

        publish_active_layout(foreground_layout());
        rebuild_layout_cache();
    }
}

/// The stamp half of [`refresh_layout_and_cache`], as a function of its two inputs — since task
/// T-39-4 the rebuild half is [`crate::layouts::layout_list_changed`] (finding Н11).
///
/// Split out for the same reason [`apply_capacity`] is: the rule can then be driven from a test
/// without a foreground window and, more to the point, without running a real `LayoutCache::build`
/// beside every other test in this binary.
///
/// `recorded` is what [`crate::buffer::with`] answered: `None` on a thread that owns no buffer,
/// which is not a reason to rebuild anything — that thread has no cache to rebuild.
fn layout_refresh_needed(observed: LayoutId, recorded: Option<LayoutId>) -> bool {
    if observed == LayoutId::default() {
        // No foreground window, or a window that vanished between two calls. Not a change.
        return false;
    }

    match recorded {
        None => false,
        Some(recorded) => recorded != observed,
    }
}

/// The keyboard layout of the window the user is typing into — **FR-52**, asked of the one
/// reader there is.
///
/// ⭐ **Task T-10-20 emptied this function of its own Win32 calls, and that is the point of it.**
/// It used to be a second, hand-written copy of the same four calls that live in
/// [`crate::switch::current`], and the two had to be kept in step by hand. FR-52 then changed —
/// the layout is read of the thread that owns the **focus** window, not the foreground one — and
/// a requirement that has to be re-implemented in two places is a requirement that will one day
/// be implemented in one. Four repairs of defect E went past both copies. There is one reader
/// now; this is a caller of it and nothing else.
///
/// Kept as a named function rather than inlined at its three call sites so that the *reason* the
/// program asks — FR-04's stamp and the cache of FR-20/FR-21, not FR-50's verdict — still has a
/// name in this module.
///
/// A zero answer is not an error and is returned as [`LayoutId::default`], which no cache
/// contains: the buffer then records strokes with their scan codes and no characters, which is
/// the same outcome as a key the layouts have nothing on. FR-23 converts such a stroke from its
/// **scan code**, which is what survived — task Т-22-10 corrected the sentence that used to stand
/// here: [`crate::convert::convert_stroke`] guards on the candidate, so the stroke comes through
/// unchanged only when the target layout is silent on that key too, and takes the target's own
/// character otherwise. See the module documentation of [`crate::buffer`].
fn foreground_layout() -> LayoutId {
    crate::switch::current()
}

/// Posts `message` to the window of `role`, if that thread has one right now, and answers whether
/// the message was queued.
///
/// The same shape as the wake-up loop of [`request_shutdown`] and for the same reason:
/// `PostMessageW` queues and returns, so one thread can nudge another without either of them
/// blocking, which is what section 6.3 and NFR-04 require of everything near the hook path.
///
/// ⭐ **The answer — task T-37-1, findings С23 and Н3.** `false` is the two ways a post can fail:
/// the thread has no window (it has not created one yet, or has already destroyed it), or
/// `PostMessageW` refused — a window that went between the load and the call, or a queue that
/// is full. The caller whose work hangs on the message arriving is the one that can do something
/// about it, so the answer goes back to it; a caller for whom a lost message is harmless says so
/// where it drops the answer.
fn post_to(role: Role, message: u32) -> bool {
    let raw = WAKE_TARGETS[role.index()].load(Ordering::Acquire);

    if raw == NO_WINDOW {
        // That thread has not created its window yet, or has already destroyed it. What the
        // first case costs is the caller's to say — the answer below tells it.
        return false;
    }

    // SAFETY: `raw` was published by `Window::create` from the handle CreateWindowExW returned
    // and is cleared by `Window::drop` before the window is destroyed, so the value read here
    // is either a live window of this process or `NO_WINDOW`, which was filtered out above.
    // PostMessageW only queues the message and returns; it never dereferences wparam or lparam,
    // both of which are zero here, and it does not block.
    let posted = unsafe {
        PostMessageW(
            Some(HWND(raw as *mut c_void)),
            message,
            WPARAM(0),
            LPARAM(0),
        )
    };

    match posted {
        Ok(()) => true,
        Err(error) => {
            // NFR-13: examined, not discarded. A failure here means the target destroyed its
            // window between the load above and this call — it is already leaving — or that its
            // queue is full, which is a thread that has stopped taking messages.
            report_non_critical("PostMessageW", &error);
            false
        }
    }
}

/// Posts `message` to the input thread's window, and answers whether it was queued.
///
/// The one thing module `watchdog` needs from this module and the only way it gets it. Its
/// `WinEvent` callback runs on the watcher thread and the buffer it has news for is a
/// thread-local of the input thread (section 6.3), so the news has to travel; the register of
/// which thread owns which window is [`WAKE_TARGETS`], and it stays here rather than being
/// copied into a second module.
///
/// Callable from a callback the system drives, which is the property that matters: it is
/// [`post_to`] and therefore one atomic load and one `PostMessageW`, and `PostMessageW` queues
/// and returns without blocking (NFR-04).
///
/// ⭐ **It answers since task T-37-1, and the answer is what that task is about** (findings С23
/// and Н3). It used to return nothing, and its two neighbours below called the difference a
/// virtue. But the two posts that most need to know — the flush of a focus change, whose three
/// consequences hung on the message arriving, and the request to put the hook back, whose reason
/// stayed armed with nobody to deliver it — are posts **to this thread**. Every caller now either
/// uses the answer or drops it with `let _ =` and a sentence saying why a loss does no harm there.
pub(crate) fn post_to_input_thread(message: u32) -> bool {
    post_to(Role::Input, message)
}

/// Posts `message` to the watcher thread's window, and answers whether that thread had one.
///
/// **The handover of decision R-31**, and the only thing module `switch` needs from this module.
/// Method 3 of FR-50 is `ITfInputProcessorProfileMgr::ActivateProfile`, which is COM, and section
/// 6.1 writes of the input thread "Не выполняет: UI, файловый ввод-вывод, **COM**"; the watcher
/// thread already holds an STA by the same table. So the input thread publishes the target and
/// posts this, which is one atomic load and one `PostMessageW` — it queues and returns, so the
/// thread that holds the hook does not block on COM (NFR-04, FR-80).
///
/// The answer matters: with no watcher window there is nobody to do the work, and the caller has
/// to take its pending request back rather than leave it for a later message to act on — which is
/// what `guard::request_probe` does with its ticket. [`post_to_input_thread`] answers the same
/// question in the same words since task T-37-1; until then this paragraph said it did not, and
/// called that a virtue. Since the same task a refused `PostMessageW` is a `false` here as well,
/// where it used to be answered `true`: a message that was not queued does not arrive either.
pub(crate) fn post_to_watcher_thread(message: u32) -> bool {
    post_to(Role::Watcher, message)
}

/// Posts `message` to the UI thread's window, and answers whether that thread had one.
///
/// **The handover of task T-07-2**, and the third of its kind in this program. The primitives of
/// module `selection` refuse the thread that owns the hook outright — `OpenClipboard` takes a
/// system-wide lock, the retries of FR-62 cost up to 180 ms and step 3 of FR-61 waits up to 300,
/// against the hundred microseconds NFR-01 gives the hook callback and the few seconds FR-80
/// says the system waits before removing the hook silently. So the input thread decides *whether*
/// the press belongs to the selection path and posts this; the work is done on the **UI** thread,
/// which section 6.1 already gives the process's slow, deadline-free work and which is the thread
/// `selection::listen` published as the one allowed to block.
///
/// The answer matters: with no UI window there is no selection path, and module `selection` has
/// to take its pending plan back and let the press go down the typing-buffer path of FR-60
/// instead. [`post_to_input_thread`] answers the same question in the same words since task
/// T-37-1, and a refused `PostMessageW` is a `false` here too since then — see
/// [`post_to_watcher_thread`].
pub(crate) fn post_to_ui_thread(message: u32) -> bool {
    post_to(Role::Ui, message)
}

/// Whether `hwnd` is the watcher thread's own window.
///
/// **Section 6.1 in one line.** The password probe of FR-71 may run on the watcher thread and on
/// no other, and all three threads of this process share one window procedure, so the procedure
/// has to be able to tell which window a message arrived at. `buffer::is_installed` is the same
/// test for the input thread; the watcher owns no thread-local of its own, and the register of
/// which thread owns which window is already here.
///
/// **SEC-05.** This is also what keeps a forged `guard::WM_APP_PROBE` harmless twice over: a
/// message aimed at the input or the UI window is ignored here, and one aimed at the watcher
/// window finds no probe pending and does nothing.
///
/// ⚠ Until task Т-14-4 this had a second caller — the arm that ran method 3 of FR-50 on the
/// watcher thread (decision R-31). Methods 2 and 3 are gone from the requirement, so that arm is
/// gone from the procedure and FR-71's is the one left.
fn is_watcher_window(hwnd: HWND) -> bool {
    let raw = WAKE_TARGETS[Role::Watcher.index()].load(Ordering::Acquire);

    raw != NO_WINDOW && raw == hwnd.0 as usize
}

/// Whether `hwnd` is the input thread's own window — the counterpart of [`is_watcher_window`],
/// task **T-06-1**.
///
/// # Why the buffer can no longer be the test
///
/// Everywhere else in this procedure "this is the input thread" is asked as
/// `buffer::is_installed`, and section 6.3 makes that true: the typing buffer is a thread-local
/// of that thread and of no other. FR-70 is the one requirement that **takes the buffer away** —
/// see [`apply_buffering_gate`] — so during a password field the buffer is absent on the very
/// thread the question is about, and asking it that way would answer "no" precisely when the
/// answer has to be "yes, and switch it back on".
///
/// So the gate asks the register of windows instead, which is the same fact from the other side
/// and is what [`is_watcher_window`] already does for method 3 of FR-50. The existing
/// `buffer::is_installed` tests are left exactly as they are: each of them guards work that
/// needs a buffer to act on, and skipping it while there is none is correct rather than merely
/// harmless.
///
/// **SEC-05.** It is also what keeps a forged [`crate::guard::WM_APP_FIELD`] from reaching the
/// gate on the UI or watcher window, where re-installing a typing buffer would put one on a
/// thread section 6.3 gives none.
fn is_input_window(hwnd: HWND) -> bool {
    let raw = WAKE_TARGETS[Role::Input.index()].load(Ordering::Acquire);

    raw != NO_WINDOW && raw == hwnd.0 as usize
}

/// Whether `hwnd` is the UI thread's own window — **task T-36-4, finding С15**.
///
/// The third of the family, and the last branch of [`window_proc`] to get one. The sound of
/// FR-100 is answered wherever its message lands, and section 6.1 forbids exactly that: the
/// input thread holds the hook, FR-80 takes the hook off a thread that stops answering, and
/// playing a sound there is work that thread has no budget for. A process at the same integrity
/// level can post any of the three sound messages (SEC-05) — to the **input** window as easily
/// as to the UI one — and until this gate the branch could not tell the difference, while four
/// gates of [`is_input_window`] and one of [`is_watcher_window`] stood beside it doing exactly
/// that for their own messages.
///
/// ⛔ Reads the register directly and **not** through [`ui_window_raw`]: that accessor exists
/// only under `panic = "unwind"` (FR-99 has nothing to signal where a panic ends the process),
/// and this gate compiles in both configurations because the branch it guards does.
fn is_ui_window(hwnd: HWND) -> bool {
    let raw = WAKE_TARGETS[Role::Ui.index()].load(Ordering::Acquire);

    raw != NO_WINDOW && raw == hwnd.0 as usize
}

/// Which press one of the three sound messages asks for **at this window** — task T-36-4.
///
/// The predicate of the gate above, as a function of its arguments rather than as an `if` inside
/// the procedure: `at_the_ui_window` is computed there and the decision is made here, which is
/// the shape task T-39-5 gave `answer_device_change` and the only shape a test can drive.
///
/// **SEC-05.** A forged message aimed at the input or the watcher window now buys its sender
/// nothing at all, and one aimed at the UI window buys what it always did — one playback of a
/// click, a sound any process may make for itself, with the setting of FR-100 still obeyed.
const fn press_of_sound_message_at(message: u32, at_the_ui_window: bool) -> Option<Press> {
    if at_the_ui_window {
        press_of_sound_message(message)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------------------
// FR-70 — the typing buffer is switched off in a password field (task T-06-1)
// ---------------------------------------------------------------------------------------

thread_local! {
    /// The typing buffer while module `guard` has buffering switched off — **FR-70**.
    ///
    /// # Why the buffer is parked rather than emptied on every stroke
    ///
    /// FR-70 says «буфер набора **не ведётся**», and the honest reading of that is that the
    /// stroke never reaches the ring at all — not that it reaches it and is removed afterwards.
    /// The only place a stroke could be turned away is [`crate::buffer::record`], which module
    /// `hook` calls from inside the callback, and both of those files are accepted code that
    /// this task may not touch. What *is* reachable is the thing `record` itself consults: it is
    /// `with(…).unwrap_or(Recorded::Ignored)` over a thread-local, so a thread with no buffer
    /// records nothing and suppresses nothing, which is exactly the pair FR-70 asks for.
    ///
    /// That is also the cheapest possible answer for NFR-01: nothing was added to the callback
    /// path. The presence check `record` performs for every stroke on the machine is the same
    /// one it always performed, and in a password field it now takes the shorter arm.
    ///
    /// # Why the recorder is kept instead of dropped
    ///
    /// `Recorder` owns the layout cache of FR-20 — thousands of `ToUnicodeEx` calls, rebuilt
    /// only when the layout moves — and the `hkl` of FR-04. Dropping it on entering a password
    /// field and building a fresh one on leaving would pay the whole sweep of FR-20 for every
    /// password box the user ever focuses, and would leave the buffer without characters until
    /// something happened to rebuild it.
    ///
    /// **SEC-02 is met before the value gets here**: [`park_buffer`] flushes through
    /// `buffer::reset`, which is the one function of module `buffer` that overwrites the ring
    /// with zeroes, and only then takes the recorder out. What is parked is an empty, zeroed
    /// ring with a cache beside it.
    static PARKED_BUFFER: std::cell::RefCell<Option<crate::buffer::Recorder>> =
        const { std::cell::RefCell::new(None) };
}

/// Plays a focus change whose `WM_APP_FLUSH` was refused — what that message's branch of
/// [`window_proc`] does, made at the moment the next message of the input window finds the request
/// marked. **Finding С23, task T-37-1.**
///
/// # The same decisions, through the same doors
///
/// Nothing here decides anything anew, because deciding twice is deciding differently (the note
/// above the fork of FR-14 in [`window_proc`] says so of that very fork):
///
/// * **FR-14** is `watchdog::take_typing_induced_flush`, asked about `WM_APP_FLUSH` — the message
///   the request stands for — and its `true` gets what the exempt half of the branch gives:
///   `guard::note_focus_moved_keeping_buffer`, the probe of FR-72 with the buffer left alone;
/// * everything else gets the ordinary half: the flush of FR-10 resolved by FR-12
///   (`watchdog::apply_flush`), then `guard::note_focus_moved` and [`park_buffer`] — the probe, and
///   the unconditional wipe decision **П-2** puts on a focus change (SPEC §10, record 10).
///
/// Then the gate, as the branch has it. The two are idempotent, so the gate that runs again at the
/// tail of the procedure for most messages decides nothing twice; for the messages that return
/// before the tail — the hotkey, the liveness tick, the rehook, the wipe — this is the only gate
/// there is, and it is the reason the replay is not left to the tail.
///
/// # Counted once
///
/// The request leaves the cell through the one door it has, the swap of
/// `watchdog::take_pending_flush`, whichever of the two functions opens it: `window_flushes_taken`
/// moves once, `flushes_without_request` does not move, and `focus_after_typing` moves for the
/// exempt kind as it would have for the message. A buffer that FR-70 has parked takes nothing out —
/// exactly as it takes nothing out of the message itself — and the request waits for the next flush.
fn replay_lost_flush() {
    if crate::watchdog::take_typing_induced_flush(crate::watchdog::WM_APP_FLUSH) {
        crate::guard::note_focus_moved_keeping_buffer();
    } else {
        crate::watchdog::apply_flush(crate::watchdog::WM_APP_FLUSH, LPARAM(0));
        crate::guard::note_focus_moved();
        park_buffer();
    }

    apply_buffering_gate();
}

/// Applies what module `guard` has published to the typing buffer of this thread — **FR-70,
/// FR-71, FR-73**, task T-06-1.
///
/// Called from [`window_proc`] on every message of the **input** window, which is the same
/// wiring [`apply_configured_capacity`] has and for the same reason: re-reading a published
/// value after every message costs one relaxed load and one comparison, cannot miss a change
/// however the change was made, and needs no channel between the two threads. The
/// [`crate::guard::WM_APP_FIELD`] the watcher thread posts is therefore a **nudge** — it makes a
/// message arrive — and this line is what decides.
///
/// The state is read with one operation, which is what FR-71 asks of the reader of the flag, and
/// it is read here rather than in the callback because here is where the buffer lives.
fn apply_buffering_gate() {
    // ⭐ **FR-84 for the hotkey as well as for the buffer — task T-52-3.** FR-95 says the
    // hotkey disappears while the program is active and points at «список исключений (FR-84)»
    // as the remedy for an application that needs the key for itself; until this task the
    // remedy did not exist in the code. The gate is where it belongs: the same message, the
    // same thread, the same published verdict the line below already reads, and one relaxed
    // store — the callback goes on naming nothing of module `guard`, which is that module's
    // contract in section 6.3.
    crate::hook::set_hotkey_yields(crate::guard::excluded());

    if crate::guard::buffering_allowed() {
        restore_buffer();
    } else {
        park_buffer();
    }
}

/// Takes the typing buffer away and wipes it — the "off" half of [`apply_buffering_gate`].
///
/// ⚠ **The wipe is not incidental.** SEC-02, and the task specification says why: without it,
/// whatever the user typed before moving into the password field would sit in this process's
/// memory for the whole time they are typing the password. `buffer::reset` is the one function
/// that overwrites the ring with zeroes, and it runs **before** the recorder leaves the slot.
///
/// Idempotent: a thread whose buffer is already parked has none installed and returns at the
/// first line. A thread that never had one — the UI and watcher threads, and this thread before
/// [`start_input_pipeline`] — does the same.
fn park_buffer() {
    if !crate::buffer::is_installed() {
        return;
    }

    // SEC-02, and FR-10's "полный сброс" in the one place FR-70 asks for one: everything typed
    // before the focus moved is gone, and the memory it stood in is overwritten.
    crate::buffer::reset();

    // The recorder is *moved* out rather than dropped, so the cache of FR-20 survives. The
    // one-slot recorder left behind is thrown away by the `uninstall` below and exists only
    // because `buffer::with` lends a `&mut` and a value has to be put in its place; it is never
    // recorded into, because the very next line removes it.
    let Some(parked) = crate::buffer::with(|recorder| {
        core::mem::replace(recorder, crate::buffer::Recorder::with_capacity(1))
    }) else {
        return;
    };

    crate::buffer::uninstall();

    PARKED_BUFFER.with(|cell| {
        cell.replace(Some(parked));
    });
}

/// Puts the typing buffer back — the "on" half of [`apply_buffering_gate`].
///
/// Idempotent, and a no-op on every thread that never parked one.
///
/// ⚠ **Nothing is recovered.** What comes back is the ring as [`park_buffer`] left it: empty and
/// zeroed. The strokes made while buffering was off were never stored, so there is nothing for
/// this to restore even in principle — which is the point of the design, not a limitation of it.
///
/// ⭐ **Point 4 of task T-13-4 — the one belief that must not come back as it left.** The ring is
/// deliberately empty, but `Held::caps` is not part of the ring: it is a belief about the
/// *machine*, and while the buffer sat in [`PARKED_BUFFER`] the machine went on being typed at.
/// `record` was never reached for any of it — `buffer::with` answers `None` on a parked thread —
/// so a `CapsLock` pressed in the password field of FR-70 or in the excluded process of FR-84
/// leaves the tracker inverted for the rest of the session. So the machine is asked again here,
/// at the moment the buffer goes back on the thread, and only here: the early return above makes
/// this the transition and not every message.
///
/// SEC-02 is untouched by that. What is seeded is one `bool` about a key, over a ring that is
/// still empty and still zeroed; no stroke is recovered, restored or inferred.
fn restore_buffer() {
    let Some(parked) = PARKED_BUFFER.with(|cell| cell.replace(None)) else {
        return;
    };

    crate::buffer::install_recorder(parked);

    crate::buffer::set_caps_lock(crate::hook::caps_lock_on);
}

/// Applies `f` to the typing buffer of this thread wherever FR-70 currently keeps it —
/// installed on the thread, or parked by [`apply_buffering_gate`] — and answers `None` on a
/// thread that has neither, which is every thread but the input one.
///
/// # Task T-10-0f — why the parked recorder must be reachable
///
/// The layout probe of FR-21 rides `WM_APP_LAYOUT`, posted by the `WinEvent` callback in the
/// same breath as the `WM_APP_FLUSH` of the very focus change it is about — and answering that
/// flush is what *parks* the buffer: FR-71 publishes «ответа ещё нет» and [`park_buffer`]
/// takes the recorder off the thread until the verdict comes back, milliseconds later. The
/// probe is therefore dispatched into the one window of time in which `buffer::with` answers
/// `None`, and a refresh routed only through it died there, silently and deterministically.
///
/// Measured on a live run before the repair: `layout_probes=1` against `window_flushes=12`,
/// and pinned by a paired experiment on the running product — five bare `WM_APP_LAYOUT` all
/// answered, five posted immediately behind a `WM_APP_FLUSH` all lost (report of task
/// T-10-0f). The recorder those probes were for was not gone; it was in [`PARKED_BUFFER`],
/// holding the stale `active` that stamped the next strokes and made the first press of FR-26
/// convert «в себя» — the acceptance session's «первое нажатие моргает». So the publication
/// reaches the recorder wherever it is, and the probe stops caring whether it arrived during
/// the interval of FR-71.
///
/// # Why this is sound
///
/// A parked recorder is a value of this same thread: parking moves it between two
/// thread-locals of the input thread and nothing else, both moves happen inside one message of
/// the same message loop, and this helper runs in that loop too — so the borrow is
/// single-threaded and can never observe the recorder half-moved. `is_installed` first, so a
/// thread with a live buffer never pays the second thread-local access; NFR-01 to NFR-05 are
/// untouched — nothing here runs in the hook callback.
///
/// ⚠ **FR-70 is not weakened.** What reaches a parked recorder through this helper is the
/// layout of FR-04 and the cache of FR-20 — recording stays off, the ring stays empty and
/// zeroed (SEC-02), and no stroke can be added to a recorder that is not installed.
fn with_recorder_wherever_it_is<R>(f: impl FnOnce(&mut crate::buffer::Recorder) -> R) -> Option<R> {
    if crate::buffer::is_installed() {
        return crate::buffer::with(f);
    }

    PARKED_BUFFER.with(|cell| cell.borrow_mut().as_mut().map(f))
}

/// Hands the input thread the settings it reacts to — FR-02 and FR-95 for the hook, FR-07 for
/// the buffer, FR-42 with FR-44 for the replacement of module `inject`, and FR-84 for the
/// exclusion list of module `guard`.
///
/// Runs on the UI thread, right after the tray has been attached, because the tray is where
/// the configuration of section 7 lives: it read the file, it owns `general.enabled`, and it
/// is the thread allowed to touch a file at all (section 6.1). The input thread only ever
/// reads the published atomics.
///
/// `with_tray` answers `None` on any thread that is not the UI thread, so a stray call from
/// elsewhere would publish nothing rather than publish something wrong.
fn publish_configuration_to_input_thread() {
    // Cloned rather than read field by field, because the borrow ends with the closure and the
    // publication below runs outside it — `with_tray` lends the tray for the length of the call
    // only. One clone of a structure of a few scalars and a handful of short strings, once per
    // publication, on the thread section 6.1 already lets read a file.
    let published = crate::tray::with_tray(|tray| tray.config().clone());

    let Some(config) = published else {
        return;
    };

    publish_configuration(&config);
}

/// Hands the input thread the settings of an **arbitrary** configuration — the same publication
/// [`publish_configuration_to_input_thread`] makes, without asking the tray what it holds.
///
/// **This is what the settings dialog of FR-92 reaches for** (task T-08-1). A setting the dialog
/// wrote to the file but did not bring to the module that acts on it is, from where the user
/// stands, a setting that does not work: rule R-52, a requirement closes where it closes. So the
/// apply path of the dialog writes the file *and* calls this, and the change takes effect on the
/// next keystroke rather than on the next start of the program.
///
/// Every store below is into an atomic another module owns, which is section 6.3's rule for
/// state that crosses a thread boundary, and none of them is a lock (NFR-04). Nothing is applied
/// to anything already built except `[buffer] capacity`, which is a live allocation and is
/// therefore the one value that needs the message at the end.
///
/// ⭐ **Task T-13-13: this is also where the three millisecond fields of section 7 meet their
/// ceilings** — see [`effective_ms`] and the block at the top of the body.
pub fn publish_configuration(config: &settings::Config) {
    // ⭐ **The ceilings of the three millisecond fields of section 7 — task T-13-13.**
    //
    // Computed first, before a single store, and computed **here** rather than where the values
    // are slept on. Three reasons, and they are the whole of the decision:
    //
    // * a publication happens once — at start-up and on every «Применить» — while a press happens
    //   as often as the user presses. A check at `inject::sleep_ms` or at
    //   `selection::wait_for_change` would be paid for by every press for ever, and NFR-01 to
    //   NFR-05 are about not adding work to that path;
    // * the published value must not lie. `inject::inter_event_delay_ms`,
    //   `selection::published_timeout` and `selection::published_restore_delay` are read by
    //   `control` for the dump and by the tests, and a clamp applied only at the point of sleeping
    //   would leave all three of them answering a number nothing acts on;
    // * one place for all three. The two `[selection]` fields hang the UI thread and the
    //   `[replacement]` one freezes the input thread, but they are the same defect of the same
    //   kind of field, and a repair split across two modules is a repair that drifts apart.
    //
    // The file is **not** rewritten and must not be: the number in it stays the author's, and only
    // the published effect is bounded. Silently correcting somebody's file is the very fault task
    // T-13-6 was raised to repair, and doing it here would trade one for the other.
    let inter_event_delay_ms = effective_ms(
        config.replacement.inter_event_delay_ms,
        crate::inject::MAX_INTER_EVENT_DELAY_MS,
    );
    let clipboard_timeout_ms = effective_ms(
        config.selection.clipboard_timeout_ms,
        crate::selection::MAX_CLIPBOARD_TIMEOUT_MS,
    );
    let clipboard_restore_delay_ms = effective_ms(
        config.selection.clipboard_restore_delay_ms,
        crate::selection::MAX_CLIPBOARD_RESTORE_DELAY_MS,
    );

    // The **clamped** value compared against the raw one, which is the comparison this function
    // already makes for `[buffer] capacity` — see `apply_capacity`, where the effective capacity
    // and not the requested one is what the buffer is measured against.
    //
    // ⚠ **Once per publication, and that is what this position buys.** There is no latch and there
    // must not be one: a `static` flag would give one entry for the whole session, and this file
    // can be applied again with a different value five minutes later. There is equally no check on
    // the press path, which would give one entry per press — the audit's own arithmetic is
    // twenty-three pauses for a six-letter word. The state that makes it "once" is the call
    // itself: `publish_configuration` runs exactly once per publication (start-up through
    // `publish_configuration_to_input_thread`, and once per «Применить» through
    // `tray::apply_settings`), so an entry raised in its body is one entry per publication in
    // which something was above its ceiling — and none at all in a publication in which nothing
    // was.
    if inter_event_delay_ms != config.replacement.inter_event_delay_ms
        || clipboard_timeout_ms != config.selection.clipboard_timeout_ms
        || clipboard_restore_delay_ms != config.selection.clipboard_restore_delay_ms
    {
        note_field_clamped();
    }

    crate::hook::set_active(config.general.enabled);

    // **Section `[diagnostics]` of section 7 — task T-34-2, finding С47.** The journal used to
    // read the file again at shutdown to learn whether it was on, and an unreadable file meant
    // «off»; now it listens to this publication like every other module, so the answer at
    // shutdown is the one the person set in this session. One store into an atomic `diag`
    // owns, written and read on this thread.
    crate::diag::set_log_enabled(config.diagnostics.log_enabled);

    // **Section `[feedback]` of section 7 — FR-100, task Т-21-5.** One store into an atomic of
    // this module, by the rule every publication above and below it follows: the configuration
    // belongs to the UI thread (section 6.1), and the sound is made on the UI thread, so this is
    // the shortest publication in the function. Nothing is applied to anything already built —
    // the next press reads whatever stands here at that moment.
    set_sound_enabled(config.feedback.sound);

    // A name this program does not know leaves the default of section 7 in place — `Pause`.
    // A resident utility whose hotkey silently ceased to exist because of a typo in a file
    // would be a worse answer than one that keeps answering to the documented default. The
    // settings dialog is where a bad name is reported to the user; that is FR-92, task
    // T-08-1.
    if let Some(vk) = crate::hook::vk_from_name(&config.hotkey.key) {
        crate::hook::set_hotkey_vk(vk);
    }

    // Section `[replacement]` of section 7 — FR-42 and FR-44, task T-04-2. This is the only
    // place either value may be published from: section 6.3 gives the configuration to the UI
    // thread, module `inject` runs on the input thread, and NFR-09 forbids the replacement path
    // from reading a file to find out. Both are plain stores into atomics `inject` owns; there
    // is no message to post, because unlike the buffer capacity of FR-07 neither value is
    // *applied* to anything the input thread has already built — the next hotkey press reads
    // whatever stands here at that moment.
    crate::inject::set_replacement_method(config.replacement.method);
    crate::inject::set_inter_event_delay_ms(inter_event_delay_ms);

    // Section `[layouts]` of section 7 — FR-30, FR-31, task T-05-2. Published for the same
    // reason and by the same rule as `[replacement]` above: the configuration belongs to the UI
    // thread (section 6.1), the choice of the target layout is made on the input thread, and
    // NFR-09 forbids that choice to read a file. The strings of section 7 are parsed here, once
    // per publication, so that the hotkey path reads numbers out of atomics and parses nothing.
    // Nothing is applied to anything already built: the next press reads whatever stands there
    // at that moment, exactly as it does for the replacement method.
    crate::layouts::publish(crate::layouts::Configured::from_settings(&config.layouts));

    // **Section `[selection]` of section 7 — FR-65, task T-07-2.** Published for the reason
    // everything above it is, and for one more that is this section's own: FR-65 says a switched
    // off selection path leaves the hotkey working «только по буферу набора», which means the
    // **input** thread has to be able to decide it without asking anybody. A flag it could only
    // learn by posting a message to the UI thread would make a disabled feature depend on the
    // availability of a thread it must not need, and NFR-09 gives the typing-buffer path thirty
    // milliseconds for the whole of itself. So the switch and the two timings of FR-61 steps 3
    // and 8 go into atomics module `selection` owns, exactly as `[replacement]` and `[layouts]`
    // do one and two lines above.
    //
    // Task T-13-13: what goes down is the configuration's `[selection]` with the two millisecond
    // fields at their ceilings — `enabled` and everything else travel untouched, and the file
    // itself is not written to. The copy is three scalars wide and is made once per publication.
    crate::selection::publish(&settings::Selection {
        clipboard_timeout_ms,
        clipboard_restore_delay_ms,
        ..config.selection.clone()
    });

    // **Section `[exclusions]` of section 7 — FR-84, task T-06-3.** Published for the reason
    // everything above it is: the list is the UI thread's to read out of the file and the watcher
    // thread's to compare against, and section 6.3 says a value that crosses threads is published
    // rather than fetched. `guard::publish_exclusions` puts it into the table of atomics the
    // watcher thread reads and asks for a fresh probe; nothing here waits for that answer, and
    // nothing on the hook path is touched by it.
    //
    // The names are folded and stored once, here, so that the comparison on the watcher thread
    // parses nothing — the same rule `[layouts]` follows one line above, where the strings of
    // section 7 are turned into numbers at publication time.
    crate::guard::publish_exclusions(&config.exclusions.processes);

    publish_buffer_section(&config.buffer);
}

// ---------------------------------------------------------------------------------------
// T-13-13 — the ceilings of the three millisecond fields of section 7
// ---------------------------------------------------------------------------------------

/// A millisecond field of section 7 as it is **published**: what the file asked for, or the
/// ceiling when the file asked for more.
///
/// [`crate::buffer::effective_capacity`] written for the millisecond fields, and written once
/// rather than three times: `[replacement] inter_event_delay_ms`, `[selection]
/// clipboard_timeout_ms` and `[selection] clipboard_restore_delay_ms` differ in which thread they
/// hang and in nothing else, so they get one rule and one call site each. The ceilings themselves
/// are **not** here — each lives in the module that owns the field and states the section 7
/// default beside it: [`crate::inject::MAX_INTER_EVENT_DELAY_MS`],
/// [`crate::selection::MAX_CLIPBOARD_TIMEOUT_MS`] and
/// [`crate::selection::MAX_CLIPBOARD_RESTORE_DELAY_MS`], each with the reason its number is that
/// number.
///
/// No zero case, unlike `effective_capacity`: zero is a **documented value** of all three fields
/// (section 7 gives `inter_event_delay_ms` exactly that default) and means "do not wait", which is
/// a request and not a mistake.
const fn effective_ms(requested: u32, ceiling: u32) -> u32 {
    match requested {
        requested if requested > ceiling => ceiling,
        requested => requested,
    }
}

/// Puts the one journal entry a publication that had to clamp leaves behind — task T-13-13.
///
/// # SEC-01, SEC-07 — what this entry can and cannot say
///
/// «configuration field clamped to its ceiling» is a fact about **a decision of this program**: a
/// publication found a millisecond field of section 7 above its bound and published the bound. The
/// string is chosen at compile time and is a row of the closed table of module `diag`; the call
/// carries **no number at all**, so neither the value in the file, nor the value published, nor
/// which of the three fields it was, nor how many of them there were can be recovered from the
/// ring. A configuration file is hand-edited and may hold anything, which is the same reasoning
/// the five «configuration file …» rows of task T-13-6 rest on. [`crate::diag::OsCode::NONE`] goes
/// with it because no Win32 call failed here.
///
/// # Where this runs — NFR-01…NFR-05
///
/// On the UI thread, inside [`publish_configuration`], which section 6.1 already lets read a file.
/// The hook callback cannot reach it and neither can the replacement path: nothing below
/// `crate::inject::on_hotkey` calls anything in this section, and the hot path gained no branch at
/// all from this task. `diag::record` is in any case allocation-free, lock-free and free of
/// input-output, which is why it is callable from anywhere in section 6.1.
fn note_field_clamped() {
    crate::diag::record(
        crate::diag::Operation::from_name("configuration field clamped to its ceiling"),
        crate::diag::OsCode::NONE,
    );
}

/// Publishes the `[buffer]` section to the input thread and nudges it into applying it — FR-07
/// and, since task T-52-4, **FR-15**.
///
/// Two steps and in this order: the values go into the atomics first, and only then is the
/// message posted, so a thread woken by the message cannot read a stale one.
///
/// The message is a wake-up and not a command — the same design as [`WM_APP_WAKE`], and for
/// the same SEC-05 reason. If it never arrives, because the input thread had not created its
/// window yet, the values are not lost: [`start_input_pipeline`] reads the atomics once more
/// when its sweep is over.
///
/// ⚠ **The timeout is clamped here and the capacity is not**, and the asymmetry is deliberate.
/// A capacity is clamped where it is *used*, because `apply_capacity` has to compare the raw
/// number against the effective one to avoid rebuilding the ring on every message. The timeout
/// has no such comparison to protect, and it does need a value the "nothing published yet"
/// marker cannot collide with — see [`BUFFER_IDLE_TIMEOUT_S`]. Clamping at publication is also
/// what task T-13-13 does with the three millisecond fields of section 7.
fn publish_buffer_section(buffer: &settings::Buffer) {
    BUFFER_CAPACITY.store(buffer.capacity, Ordering::Release);
    BUFFER_IDLE_TIMEOUT_S.store(
        crate::buffer::effective_idle_timeout_s(buffer.idle_timeout_s),
        Ordering::Release,
    );

    // Task T-37-1: the answer is dropped for the reason the paragraph above gives — the message
    // is a wake-up, the values are in the atomics, and the input thread reads them after every
    // message it takes and once more at the end of its start-up.
    let _ = post_to(Role::Input, WM_APP_CONFIGURED);
}

/// The message loop. Returns when [`PostQuitMessage`] has been reached, that is, when this
/// thread decided on its own flag that it is leaving.
fn pump() -> WinResult<()> {
    let mut message = MSG::default();

    loop {
        // SAFETY: `message` is a live, properly aligned `MSG` owned by this frame for the
        // whole call, which is the only buffer the call writes to. `None` for the window
        // filter asks for every message of this thread, which is what a loop that must also
        // see `WM_QUIT` and thread messages needs; the two zero filters mean "no range
        // filter" and are the documented way to ask for everything.
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };

        // NFR-13: GetMessageW has three outcomes, not two, and `-1` is an error that must
        // never be read as "no more messages" — doing so turns a broken queue into a silent,
        // clean-looking exit.
        match result.0 {
            -1 => return Err(WinError::from_thread()),
            0 => return Ok(()),
            _ => {
                // **FR-101, task Т-32-3 — the modeless windows of the letters.** A modal
                // dialog runs its own loop and gets `IsDialogMessageW` for free; the windows
                // of the letters are modeless and run in *this* loop, so without this line
                // Tab would not move between their buttons, Enter would not press the default
                // one and Esc would not close them.
                //
                // It costs one comparison per message on a thread with no letters open, which
                // is the ordinary state of this program: `letters::filter_message` walks a
                // list that is empty then.
                //
                // SAFETY: `message` was filled by the `GetMessageW` above and is read, not
                // written.
                if unsafe { crate::letters::filter_message(&message) } {
                    continue;
                }

                // SAFETY: `message` was filled by the GetMessageW above and is read, not
                // written, by DispatchMessageW.
                //
                // `TranslateMessage` is deliberately absent: it exists to synthesise
                // `WM_CHAR` from key messages, and none of these three windows is ever
                // visible, focusable or a keyboard target, so it would have nothing to
                // translate. Task T-08-1 adds the dialog message filtering the settings
                // window needs, in the UI thread's loop, where it belongs.
                //
                // The `LRESULT` returned is the window procedure's own result forwarded
                // back; it is neither a `BOOL` nor a handle nor a pointer and carries no
                // error information, so NFR-13 has nothing to check here.
                let _ = unsafe { DispatchMessageW(&message) };
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// The window class, the windows and the window procedure
// ---------------------------------------------------------------------------------------

/// Class name of the three hidden windows. Never displayed anywhere.
const WINDOW_CLASS_NAME: PCWSTR = w!("LangSwitcher.Hidden");

/// Handle of this module, the `hInstance` a window class and a window are registered under.
fn module_instance() -> WinResult<HINSTANCE> {
    // SAFETY: `None` asks for the handle of the file used to create the calling process,
    // which is the documented way to name the running executable and cannot refer to a
    // module that could be unloaded under us. The result is a borrowed handle that must not
    // be freed, and nothing here frees it.
    let module = unsafe { GetModuleHandleW(PCWSTR::null())? };

    Ok(HINSTANCE(module.0))
}

/// The window class of the process, unregistered when this value is dropped.
struct WindowClass {
    instance: HINSTANCE,
}

impl WindowClass {
    fn register(instance: HINSTANCE) -> WinResult<Self> {
        let class = WNDCLASSEXW {
            cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: WINDOW_CLASS_NAME,
            // Everything else stays zero: no class styles, no extra bytes, no icon, no
            // cursor, no background brush and no menu. A window that is never painted and
            // never shown needs none of them, and each one left out is one less thing a
            // message from another process could reach.
            ..Default::default()
        };

        // SAFETY: `class` is a fully initialised `WNDCLASSEXW` owned by this frame; its
        // `cbSize` describes it and its two pointer fields — the procedure and the class
        // name — are a `'static` function and a `'static` literal, so both outlive every
        // window ever created from the class. The call only reads through the pointer.
        let atom = unsafe { RegisterClassExW(&class) };

        // NFR-13: RegisterClassExW reports failure with a zero atom.
        if atom == 0 {
            return Err(WinError::from_thread());
        }

        Ok(Self { instance })
    }
}

impl Drop for WindowClass {
    fn drop(&mut self) {
        // SAFETY: the class name is a `'static` literal and the instance is the one the
        // class was registered under, which is what identifies the class to unregister.
        // Every window of this class has been destroyed by now — the threads that owned
        // them have been joined — which is the precondition the call demands.
        if let Err(error) = unsafe { UnregisterClassW(WINDOW_CLASS_NAME, Some(self.instance)) } {
            report_non_critical("UnregisterClassW", &error);
        }
    }
}

/// A thread's hidden window. Destroyed, and unpublished, when this value is dropped.
///
/// The value never leaves the thread that created it, which is what makes the `Drop`
/// correct: `DestroyWindow` only works on a window of the calling thread.
struct Window {
    handle: HWND,
    role: Role,
}

impl Window {
    fn create(role: Role, instance: HINSTANCE) -> WinResult<Self> {
        let (ex_style, style, parent) = match role {
            // Message-only. Out of window enumeration, out of the Z order, out of Alt+Tab,
            // and impossible to show by accident.
            Role::Input | Role::Watcher => {
                (WINDOW_EX_STYLE(0), WINDOW_STYLE(0), Some(HWND_MESSAGE))
            }
            // Decision R-20 point 2: a top-level window that is never shown, because
            // `RegisterWindowMessage("TaskbarCreated")` is broadcast and broadcasts never
            // reach `HWND_MESSAGE` windows, and FR-81 (task T-01-4) needs it. No
            // `WS_VISIBLE`, and `WS_EX_TOOLWINDOW` keeps it out of the taskbar and out of
            // Alt+Tab even if some future code did show it.
            Role::Ui => (WS_EX_TOOLWINDOW, WS_POPUP, None),
        };

        // SAFETY: the class name is a `'static` literal naming a class registered by
        // `WindowClass::register`, which outlives every window because it is dropped only
        // after every thread has been joined. The window name is null, which is the
        // documented way to ask for no title. `instance` is the module handle the class was
        // registered under. No `lpParam` is passed, so the `WM_CREATE` this call delivers
        // to `window_proc` carries no pointer of ours for the procedure to dereference.
        let handle = unsafe {
            CreateWindowExW(
                ex_style,
                WINDOW_CLASS_NAME,
                PCWSTR::null(),
                style,
                0,
                0,
                0,
                0,
                parent,
                None,
                Some(instance),
                None,
            )?
        };

        // Published only after the handle is known good.
        //
        // `SeqCst` since task **Т-22-7**: this is the third operation of the store-buffer litmus
        // written out in [`request_shutdown`], and Release here against Acquire there did **not**
        // forbid the outcome both sides depend on being impossible — `request_shutdown` reading
        // `NO_WINDOW` while `serve_window` reads `false`, and the thread pumping for ever. One
        // store per thread per process, so the ordering costs nothing that could be measured.
        WAKE_TARGETS[role.index()].store(handle.0 as usize, Ordering::SeqCst);

        // ⭐ **Finding Н38, task T-37-1** — the start-up probe of FR-70 is asked for by whichever
        // thread publishes the last of the three windows. See the function.
        request_startup_probe_once_every_window_exists();

        Ok(Self { handle, role })
    }
}

/// Whether the start-up probe of FR-70 has been asked for — [`request_startup_probe_once_every_window_exists`].
static STARTUP_PROBE_ASKED: AtomicBool = AtomicBool::new(false);

/// Asks the watcher thread for the first probe of the password field, once, as soon as all three
/// windows are published — **finding Н38, task T-37-1, variant 1 «заказывать проверку позже»**.
///
/// # What was lost
///
/// The one probe that no focus change asks for is the one `guard::publish_exclusions` asks for,
/// and at start-up it is asked for by the UI thread while the three threads race to publish their
/// windows. A watcher window that is not there yet refuses the post, the ticket is taken back —
/// correctly, that is task Т-22-2 and it is not undone here — and nothing asks again: the field the
/// person starts in is never probed, the verdict stays at the default of FR-73, «буферизовать», and
/// a password typed there before the first focus change goes into the ring (SEC-06).
///
/// # Why here, and why all three
///
/// The register of windows is where "every thread can hear a message" is known, and
/// [`Window::create`] is where it becomes true — so the thread that publishes the **last** window
/// is the one that asks. All three and not only the watcher's: the probe is run on the watcher
/// thread, but its verdict is delivered to the **input** thread (`guard::WM_APP_FIELD`), and a
/// verdict nudging a window that does not exist would wait for whatever message reached that
/// window next.
///
/// A configuration that is published after this — the UI window came last — asks for its own
/// probe through `guard::publish_exclusions`, which then finds the watcher window and is accepted;
/// the start-up probe may therefore run without the exclusion list of FR-84 and be followed by one
/// that has it. Two probes at start-up and never none.
///
/// # The ordering
///
/// `SeqCst`, and for the reason [`request_shutdown`] writes out: the three stores in
/// [`Window::create`] are `SeqCst`, so the three loads here put all six operations in one total
/// order, and the thread whose store comes last in it sees the other two. The swap makes "once"
/// hold when two threads see all three at once.
fn request_startup_probe_once_every_window_exists() {
    let every_window_exists = WAKE_TARGETS
        .iter()
        .all(|target| target.load(Ordering::SeqCst) != NO_WINDOW);

    if every_window_exists && !STARTUP_PROBE_ASKED.swap(true, Ordering::SeqCst) {
        // The answer is `false` when the watcher window went away between the load above and the
        // post — the process is leaving, and a probe for a program that is leaving is not owed —
        // or when its queue is already full, in which case the first focus change asks again.
        let _ = crate::guard::request_startup_probe();
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // Unpublished first, so that no wake-up can be posted to a window that is about to
        // stop existing.
        WAKE_TARGETS[self.role.index()].store(NO_WINDOW, Ordering::Release);

        // SAFETY: `handle` came from a successful CreateWindowExW and has not been
        // destroyed elsewhere — this type is neither `Copy` nor `Clone`. `DestroyWindow`
        // requires the calling thread to be the one that created the window, and it is:
        // the value is created inside `serve_window` and never leaves that frame, so this
        // runs on the creating thread.
        if let Err(error) = unsafe { DestroyWindow(self.handle) } {
            report_non_critical("DestroyWindow", &error);
        }
    }
}

/// The window procedure of all three hidden windows.
///
/// SEC-05 in full: exactly two messages are handled and every other one goes to
/// `DefWindowProcW`. No message initiates a privileged action, no message parameter is
/// dereferenced, and no message can make this process do anything on behalf of the sender.
///
/// # Safety
///
/// Called by the OS with the arguments of a window message. The procedure reads `wparam`
/// and `lparam` as opaque values and never dereferences them, so the only obligation left
/// to the caller is the one the OS always meets: `hwnd` names a live window of this thread.
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_APP_WAKE => {
            // Decision R-20 point 3: the decision to quit is taken on our own flag, never
            // on the arrival of the message. A process at the same integrity level can post
            // WM_APP_WAKE to this window — SEC-05 says to assume it will — and all that
            // achieves is a re-read of a flag that is still false.
            if shutdown_requested() {
                // SAFETY: PostQuitMessage takes a value, touches no memory of ours and can
                // only be called for the calling thread, which is where a window procedure
                // always runs. It posts WM_QUIT to this thread's own queue, which is what
                // the `0` arm of `pump` is waiting for.
                unsafe { PostQuitMessage(0) };
            }

            LRESULT(0)
        }

        // SEC-05: WM_CLOSE is suppressed explicitly. The default handling destroys the
        // window, which would hand any process at the same integrity level a way to stop
        // this one's message loop. The program leaves through `request_shutdown` and
        // through nothing else.
        WM_CLOSE => LRESULT(0),

        // Tasks T-01-4 and T-03-1. Two modules get their look at the message next, because
        // between them they own values that cannot be written as a `match` arm here: one of
        // the tray's is only known at run time (`RegisterWindowMessage`), and the hook's are
        // constants of another module. Each answers `None` for everything it does not
        // handle, and on a thread that owns neither a tray nor a hook they answer `None` to
        // everything.
        //
        // SEC-05 is unaffected: both lists are closed and explicit, and nothing on either of
        // them initiates a privileged action. The menu is tracked with `TPM_RETURNCMD`, so
        // there is no `WM_COMMAND` handler anywhere in this process for a foreign message to
        // aim at.
        _ => {
            // FR-01 and FR-83, "снятие хуков". `Tray::shut_down` carries a marked place for
            // this call — task T-01-4 left it as a `TODO(T-03-1)` — but `src\tray.rs` is
            // outside the file scope of task T-03-1, so the call is made here instead, in
            // the window procedure the message arrives at and before the tray is given it.
            // The rest of FR-83's cleanup stays where T-01-4 put it. `wparam` is `TRUE` only
            // if the session really is ending; a `WM_QUERYENDSESSION` that is later refused
            // must not cost the program its hook.
            if message == WM_ENDSESSION && wparam.0 != 0 {
                crate::hook::uninstall();
            }

            // FR-21, task T-03-2a. `layouts` names the two messages that oblige the owner of
            // the cache to rebuild it — `WM_INPUTLANGCHANGE`, when the user adds, removes or
            // switches a layout, and `WM_DEVICECHANGE`, when a keyboard arrives or leaves with
            // a different scan code map — and offers `needs_rebuild` so that this line does not
            // have to restate the requirement. Until now nobody called it.
            //
            // ⚠ **And until task T-08-4 it fired for neither message.** `WM_INPUTLANGCHANGE`
            // still cannot arrive — it goes to the window with the keyboard focus — but
            // `WM_DEVICECHANGE` now does: `watchdog::register_device_notice` asks for it at this
            // very window, in place of the raw input keyboard entry that used to bring the same
            // news as `WM_INPUT_DEVICE_CHANGE` and took the whole low-level hook chain with it.
            // This branch is the handler task T-03-2a wrote for FR-21's own message, doing at
            // last what it was written to do.
            //
            // This is the message loop of the input thread, with the hook callback long
            // returned, which is where module `layouts` says a rebuild belongs and where NFR-01
            // and NFR-02 confine it: the sweep is thousands of `ToUnicodeEx` calls, three
            // orders of magnitude past the callback's budget.
            //
            // ⭐ **Finding Н25 — task T-39-5: the register of windows says "this is the input
            // window", not the buffer.** The test used to be `buffer::is_installed`, and FR-70
            // takes the buffer off this very thread for the interval of a password field: a
            // keyboard plugged in during it was dropped unread and the cache never rebuilt. The
            // probe branch below had the same trap closed in task T-10-0f. The test is still not
            // decoration — the UI and watcher windows are top-level, `WM_DEVICECHANGE` is
            // broadcast to every top-level window, and without it every device change in the
            // machine would run a full sweep on a thread section 6.1 exists to keep free.
            //
            // ⚠ **FR-11: neither call flushes the buffer.** See `publish_cache` and
            // `publish_active_layout`; there is no `reset` anywhere on this path.
            //
            // SEC-05: a process at the same integrity level can post either message. All that
            // buys it is a rebuild of our own cache out of the system's own layout list — an
            // idempotent operation over memory of ours, which is the same standing the wake-up
            // message has. ⚠ Task T-39-5 does not weaken it: a forged message at the UI or
            // watcher window is refused by the register of our own windows exactly as it was
            // refused by the missing buffer, and one at the input window buys what it always did.
            answer_device_change(message, is_input_window(hwnd));

            // ⭐ **Finding С23, task T-37-1 (idea И-21) — the work of a focus change is bound to
            // the cell, not to the message.** A `WM_APP_FLUSH` whose post was refused used to take
            // three things with it in silence: the flush of FR-10, the wipe of SEC-02 and the new
            // probe of FR-70 — a password field entered that way was taken for the field before
            // it. The watcher thread now marks such a request (`watchdog::PENDING_LOST`), and the
            // first message of the input window that finds the mark plays it, whatever message
            // that is.
            //
            // **Here, and not in the tail beside the gate**, because the tail is not reached by
            // every message: `handle_watchdog_message` returns early for the liveness tick, the
            // rehook and the wipe, and `hook::handle_input_message` for the hotkey — and the hotkey
            // is exactly the message that must not find the previous field's strokes still in the
            // ring. And **before** this message's own flush, because the lost event is the older
            // of the two.
            //
            // ⚠ **An addition, not a replacement** (SEC-05, decision П-2): `WM_APP_FLUSH` is left
            // out and its own branch below is untouched, unconditional park and all. And only a
            // **marked** request is played — see `watchdog::PENDING_LOST` for why a request whose
            // message is still on its way must be left to that message, FR-14 above all.
            //
            // NFR-01: one atomic load while nothing is marked, on every message of every window —
            // the price `apply_configured_capacity` below already pays for the same idea.
            if crate::watchdog::flush_post_lost()
                && message != crate::watchdog::WM_APP_FLUSH
                && is_input_window(hwnd)
                && crate::watchdog::claim_lost_flush()
            {
                replay_lost_flush();
            }

            // FR-10, FR-12, FR-13 — task T-03-3. The two asynchronous flush sources of the
            // FR-10 table arrive here: a mouse button as the `WM_INPUT` of Raw Input, and the
            // `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS` subscriptions as the private
            // `watchdog::WM_APP_FLUSH` the watcher thread posts. `apply_flush` answers `None`
            // for every other message and on every thread that owns no buffer, which is every
            // thread but the input one (section 6.3).
            //
            // The result is deliberately dropped: FR-12 decides how much of the buffer goes,
            // module `watchdog` counts what happened, and there is nothing for the window
            // procedure to do with the answer. SEC-07 — it is a count, never a stroke.
            //
            // ⭐ **FR-14 — task Т-48-2, question 111.1.** The line above it is the whole of the
            // new rule as this function sees it: `take_typing_induced_flush` decides, once, and
            // the answer is carried down to the park below. Deciding twice would be deciding
            // differently — the watcher thread publishes verdicts between these two points —
            // and deciding *there* would be too late, because the flush would already have run.
            //
            // `true` means the focus event was the application's own answer to the user's
            // typing: the address bar of Edge announcing the suggestion row it has just opened,
            // an autocompletion, an Electron echo. The request is already taken by then, so
            // there is nothing left for `apply_flush` to apply; the call is skipped rather than
            // relying on that, so that the intent is on the page and not in the arithmetic.
            let caused_by_typing = crate::watchdog::take_typing_induced_flush(message);

            if !caused_by_typing {
                crate::watchdog::apply_flush(message, lparam);
            }

            // **FR-21, the delivery** — task T-03-3, and the point of that half of the task.
            // `WM_INPUTLANGCHANGE` cannot reach this program: it goes to the window with the
            // keyboard focus, and the windows that do the typing are other people's. `watchdog`
            // stands in for it with the layout probe behind `WM_APP_LAYOUT`, and `rebuild_for`
            // maps that to a rebuild only if the layout really moved.
            //
            // ⚠ The device half no longer comes through here. Task T-08-4 replaced
            // `RIDEV_DEVNOTIFY` and its `WM_INPUT_DEVICE_CHANGE` with a device notification that
            // delivers the real `WM_DEVICECHANGE`, which the `needs_rebuild` branch above
            // answers; the arm for the old message stays in `rebuild_for` and answers the same
            // way it always did, but nothing this program registers can produce it any more.
            //
            // ⚠ **FR-11: not one of these paths flushes the buffer.** A layout change is not a
            // reason to throw away what the user has typed; every stroke carries the layout it
            // was typed under precisely so that it does not have to be.
            //
            // ⚠ **Task T-10-0f: the gate is the window register, not the buffer.** This used
            // to be `buffer::is_installed()`, and that was the loss the acceptance session
            // felt as «первое нажатие моргает»: the probe behind `WM_APP_LAYOUT` follows the
            // `WM_APP_FLUSH` of its own focus change, answering that flush parks the buffer
            // for the interval of FR-71, and `is_installed` then answered «no» for exactly
            // every probe a focus change ever posted — measured as `layout_probes=1` against
            // `window_flushes=12`, and pinned by the paired experiment written up at
            // `with_recorder_wherever_it_is`. `is_input_window` is the same fact from the
            // other side — the register of windows — and it is the test built for precisely
            // this trap: see its documentation, «FR-70 is the one requirement that takes the
            // buffer away». SEC-05 stands as it was: a forged message at the UI or watcher
            // window is refused by the register exactly as it was refused by the missing
            // buffer, and what a forged one at the input window buys — a re-read of the
            // system's own layout list into memory of ours — is unchanged.
            if is_input_window(hwnd) {
                match crate::watchdog::rebuild_for(message) {
                    Some(crate::watchdog::Rebuild::Unconditional) => {
                        publish_active_layout(foreground_layout());
                        rebuild_layout_cache();
                    }
                    Some(crate::watchdog::Rebuild::IfLayoutChanged) => refresh_layout_and_cache(),
                    None => {}
                }

                // ⭐ **Point 5 of task T-13-4 — the session came back, or the machine woke.**
                // `watchdog::WM_APP_LAYOUT` is posted at this thread on exactly the occasions on
                // which this program may have been shown nothing: the two `WinEvent` rows of the
                // FR-10 table, and the return from an absence — an unlocked session, a resumed
                // machine — where the watchdog already asks for a fresh layout stamp because
                // what happened while it was away is unknown. **The `CapsLock` of the machine is
                // unknown for precisely the same reason and over precisely the same interval**,
                // so it is read in the same breath, on the same message, at the same thread.
                //
                // ⚠ **Decision R-20, and nothing is added to `watchdog`.** The message already
                // flies and carries no data; the value is read here, where it means something.
                // `watchdog::reinstall_hook` and the `WM_APP_LAYOUT` posts belong to another
                // task's file and are not touched — this is the receiving side and it lives here.
                //
                // Costs one thread-local read and one `GetKeyState` on a message that already
                // re-reads the keyboard layout and may rebuild the whole cache of FR-20. Nothing
                // of this is in the hook callback (NFR-01 to NFR-05).
                //
                // Unlike the layout stamp above, this does **not** reach for a parked recorder
                // through `with_recorder_wherever_it_is`, and it does not need to: the trap of
                // task T-10-0f is that a probe dispatched during the interval of FR-71 would be
                // *lost*, and this one is not — `restore_buffer` is point 4 of the same task and
                // seeds the recorder at the moment it comes back off the shelf. Two points, one
                // reading each, no state carried between them.
                if message == crate::watchdog::WM_APP_LAYOUT {
                    crate::buffer::set_caps_lock(crate::hook::caps_lock_on);
                }
            }

            // **FR-40, steps 3 to 6 — task T-04-1.** The far end of the handoff of FR-02: the
            // callback recognised the hotkey, suppressed it (step 1), posted
            // `hook::WM_APP_HOTKEY` and returned at once (step 2), and this is where the work
            // is done — on the input thread, in its ordinary message loop, with the callback
            // long returned. Section 6.1 puts `SendInput` exactly here and FR-40 forbids it
            // anywhere else: `SendInput` calls this process's own low-level hook back
            // synchronously, so from inside the callback it would be re-entrancy into a
            // function the system is already running.
            //
            // `buffer::is_installed` is what says "this is the input thread", the same test the
            // two rebuild paths above use, and it is also the whole of SEC-05 here: any process
            // at the same integrity level can post `WM_APP_HOTKEY` to any of these three
            // windows, and on the UI and watcher windows that buys the sender nothing at all,
            // while on the input window it buys a replacement of text the user has already
            // typed, from data the sender can neither see nor influence.
            //
            // Placed before `hook::handle_input_message` because that function answers
            // `Some(LRESULT(0))` for this message and returns; its counter of FR-02 handoffs
            // still counts every one of them, one line further down.
            //
            // The result is dropped: module `inject` counts what FR-45 asks to be counted and
            // there is nothing for a window procedure to do with the answer. SEC-01, SEC-07 —
            // what is dropped is counts and lengths, never a stroke.
            //
            // ⚠ **FR-60 — task T-07-2.** One hotkey, one meaning, two sources of data, and this
            // is where the source is chosen. `selection::wants_selection_path` answers `true`
            // only when it has handed the press to the UI thread; every reason it answers `false`
            // — `[selection] enabled = false` (FR-65), a password field (SEC-06), no mapping
            // cache, no UI window — is settled without a single system call made on the user's
            // behalf, and the line below then runs exactly as it ran before this task existed.
            //
            // `WM_APP_BUFFER_PATH` is the same branch arriving late: the selection path ran,
            // step 3 of FR-61 found the clipboard sequence number unmoved, and "there is no
            // selection" is FR-60's other half. It is a message of its own rather than a re-post
            // of `WM_APP_HOTKEY`, which would be offered the selection path again and loop.
            let hotkey = message == crate::hook::WM_APP_HOTKEY;
            let handed_back = message == crate::selection::WM_APP_BUFFER_PATH;

            if (hotkey || handed_back) && !(hotkey && crate::selection::wants_selection_path()) {
                // ⚠ **FR-100, task Т-21-5 — the answer is not made here.** This is the input
                // thread, whose budget NFR-09 puts at thirty milliseconds for the whole of a
                // replacement, and section 6.1 gives the process's slow work to the UI thread.
                // So the outcome travels: `post_to` is one atomic load and one `PostMessageW`,
                // which queues and returns without blocking (NFR-04). The hook callback is
                // further away still — it returned to the system long before `WM_APP_HOTKEY`
                // reached this loop.
                //
                // ⭐ **Task Т-49-2 — the gate moved and this is the whole of the repair.** The
                // condition on this `if` used to carry `crate::buffer::is_installed()`, and in a
                // password field FR-70 has **parked** the buffer, so the branch did not run and
                // not one message was posted: the press was answered by silence. The buffer is
                // now asked *inside* `press_outcome`, together with `guard::refuses`, and the
                // press that this program deliberately would not act on gets a voice of its own.
                //
                // `is_input_window` replaces what `is_installed` used to do for SEC-05 — a
                // forged `WM_APP_HOTKEY` at the UI window must not reach `inject::on_hotkey`.
                // It is the gate the FR-70 arm below already uses and it names the same thread
                // through the register of windows.
                //
                // `on_hotkey` is called **only** when the buffer is here: with the buffer parked
                // there is nothing for it to convert, and asking it would be a system call made
                // on a press this program has already decided not to answer with a replacement.
                if is_input_window(hwnd) {
                    let buffered = crate::buffer::is_installed();
                    let replaced = buffered && crate::inject::on_hotkey().is_some();

                    if let Some(press) = press_outcome(buffered, crate::guard::refuses(), replaced)
                    {
                        // Task T-37-1: dropped — a refused post costs one click of FR-100 that
                        // is not heard, and nothing of the press itself, which is already done.
                        let _ = post_to(Role::Ui, sound_message_for(press));
                    }
                }
            }

            // ⛔ **There was an arm for `switch::WM_APP_SWITCH` here until task Т-14-4**, and
            // there is deliberately none now. It was the far end of the handoff of decision R-31:
            // method 3 of the old FR-50 was `ITfInputProcessorProfileMgr::ActivateProfile`, which
            // is COM, and section 6.1 writes of the input thread «Не выполняет: UI, файловый
            // ввод-вывод, **COM**» — so the input thread published a target into an atomic, posted
            // that message, and the watcher thread did the work here inside its own STA and posted
            // `watchdog::WM_APP_LAYOUT` back so that the stamp of FR-04 followed (task T-10-5).
            //
            // The user struck methods 2 and 3 out of FR-50 on 2026-08-25 (question 63 of
            // `DECISIONS.md`), on the measurements of T-13-10 and Т-14-2: across 504 executions
            // the two fallbacks gave a verdict **not once**, and method 2 blocked the calling
            // thread without limit against a window that had stopped pumping (60 of 60) — in this
            // program that is the input thread with the low-level hook. With method 3 gone, so is
            // the handover, the atomic channel and this arm; module `switch` keeps the number
            // `WM_APP + 8` reserved and nothing posts it. The one method FR-50 now prescribes is
            // a `PostMessageW` that finishes on the input thread, where
            // `inject::System::switch_layout` already publishes the stamp itself — from
            // `switch::stamp_follows` since task Т-14-6, from `switch::confirmed` before it.

            // **FR-71, the heavy half — task T-06-1.** A handoff to the watcher thread — since
            // task Т-14-4 the only one this procedure still answers — and it exists for the same
            // class of reason the hotkey's handoff does: the three levels of FR-72 end
            // in UI Automation, which is COM and takes tens of milliseconds, and section 6.1
            // writes «Определение поля пароля (UI Automation, COM STA)» under the **watcher**
            // thread — the only one of the three with an apartment. FR-71 calls doing it inside
            // the hook «категорически запрещено», and doing it inside the `WinEvent` callback
            // would hold up the source of every focus event in the session.
            //
            // `is_watcher_window` is what says "this is the watcher thread" — the test the
            // retired arm of method 3 used one comment above, and since task Т-14-4 the only
            // caller of it left — and it is not decoration: without it a forged `WM_APP_PROBE`
            // posted at the input window would run UI Automation on the thread that owns the
            // keyboard hook.
            //
            // SEC-05: the message carries nothing — `wparam` and `lparam` are zero — and whether
            // a probe is wanted travels in an atomic only this process writes, so a forged
            // `WM_APP_PROBE` finds nothing pending and does nothing. The result is dropped:
            // module `guard` counts what happened. SEC-01, SEC-07 — a state out of four, never a
            // stroke.
            if message == crate::guard::WM_APP_PROBE && is_watcher_window(hwnd) {
                let _ = crate::guard::run_pending_probe();
            }

            // **FR-80 — task T-06-2.** Four messages, and each is answered on one window only:
            // `WM_POWERBROADCAST` and `WM_WTSSESSION_CHANGE` on the UI window, which is the only
            // top-level window of this process and therefore the only one they can arrive at,
            // and `WM_TIMER` and `watchdog::WM_APP_REHOOK` on the input window, which is the
            // only thread allowed to own the hook. Everything else answers `None` and falls
            // through unchanged.
            //
            // ⭐ **A fifth message since task Т-13-7, and it is FR-10 rather than FR-80.**
            // `watchdog::WM_APP_WIPE` is the far end of rows 8 and 9 of the flush table — the
            // session lock and the user's pause from the tray, both of which say «полный сброс +
            // обнуление памяти». **This is the whole of the route through this procedure**, and
            // it is a route between two of the three threads:
            //
            //   `WM_WTSSESSION_CHANGE` at the **UI** window   ─┐
            //   `Tray::toggle_state` on the **UI** thread ─────┴─► `watchdog::request_wipe`
            //                                                     └─ PostMessageW ─┐
            //                                                                       ▼
            //                             `WM_APP_WIPE` at the **input** window ─► `buffer::reset`
            //
            // Nothing had to be added here for it: this line already offers every message of
            // every window to `handle_watchdog_message`, and the arm binds itself to the input
            // window exactly as the other four bind themselves. The buffer it resets is the
            // thread-local of this very thread (section 6.3), which is why the arm can do the
            // work rather than pass it on again.
            //
            // SEC-05: a process at the same integrity level can post any of the five. What the
            // first four buy is one reinstallation of this program's own hook — an operation it
            // performs on itself every thirty seconds anyway, over state of its own, granting
            // nothing. The rehook message carries nothing: the reason travels in an atomic of
            // this process, and a forged message that finds it empty does nothing at all. What a
            // forged `WM_APP_WIPE` buys is one reset of our own typing buffer, which is what
            // every `Enter` the user types already does; it is refused outright at the UI and
            // watcher windows, where a ring does not exist in the first place.
            if let Some(result) = crate::watchdog::handle_watchdog_message(hwnd, message, wparam) {
                return result;
            }

            // **FR-63 — task T-07-1.** The far end of the registration made in `serve_window`:
            // `WM_CLIPBOARDUPDATE` arrives here, at the UI window, and `selection` classifies it
            // as this program's own change or the user's by the sequence number FR-61 already
            // names. `handle_clipboard_message` answers `None` for every other message and at
            // every window that is not the registered one, which is every window of the input
            // and watcher threads.
            //
            // The verdict is deliberately dropped: this task builds the primitives and the
            // decision belongs to the selection path of T-07-2. Module `selection` counts what
            // arrived, which is what acceptance points 16 and 27 measure.
            //
            // SEC-05: a process at the same integrity level can post `WM_CLIPBOARDUPDATE` here.
            // What that buys it is one read of a system counter and one comparison — nothing is
            // opened, written or decided. SEC-01, SEC-07 — a sequence number and one of two
            // words, never a byte of the clipboard.
            let _ = crate::selection::handle_clipboard_message(hwnd, message);

            // **FR-61 — task T-07-2.** The far end of the handoff the branch above made: the
            // eight steps run *here*, on the UI thread, because the clipboard primitives refuse
            // the thread that owns the hook and because section 6.1 gives this thread the
            // process's slow work. `handle_selection_message` answers `None` for every other
            // message and at every window that is not the registered one, which is every window
            // of the input and watcher threads.
            //
            // The one thing done with the answer is FR-60's other half: an outcome that is not a
            // conversion — no selection, or a refusal — hands the press back to the input thread,
            // where the typing-buffer path runs it. Nothing is waited for: `post_to` queues and
            // returns.
            //
            // SEC-05: the message carries nothing and what it is about travels in a slot only
            // this process writes, so a forged `WM_APP_SELECTION` finds no plan and is refused
            // before the clipboard is opened. SEC-01, SEC-07 — what crosses here is an outcome
            // out of three and two counts, never a character.
            if let Some(outcome) = crate::selection::handle_selection_message(hwnd, message) {
                if outcome.falls_back() {
                    // Task T-37-1: dropped, and not because nothing is lost — the press is then
                    // answered by no path at all. A post to the input window is refused only when
                    // that window is not there or its queue is full, and an input thread in either
                    // state converts nothing from its buffer anyway; the person presses again.
                    let _ = post_to(Role::Input, crate::selection::WM_APP_BUFFER_PATH);
                } else {
                    // **FR-100, task Т-21-5.** A conversion of the selection path is the one
                    // outcome that does *not* travel on to the typing-buffer path, so it is the
                    // one this branch has to answer itself. Everything that falls back is
                    // answered by the input thread's branch above, after `on_hotkey` has said
                    // what it did — which is why exactly one tone comes out of one press.
                    //
                    // Sounded straight rather than posted: this already **is** the UI thread.
                    sound_press(Press::Replaced);
                }
            }

            // **FR-100, task Т-21-5 — the far end of the input thread's post.** The one place in
            // the program where a sound is made, and it is on the UI thread **because this gate
            // says so** — task T-36-4, finding С15. The three messages of the family
            // (`WM_APP_SOUND_DONE`, `_IDLE`, `_REFUSED`) are posted to the UI window by this
            // program; until the gate, a message that arrived anywhere else was answered just
            // the same, and the window it could arrive at is the **input** one — the thread that
            // holds the hook, whose budget section 6.1 protects and whom FR-80 unhooks if it
            // stops answering.
            //
            // SEC-05: a process at the same integrity level can post any of the three. At the UI
            // window that buys the sender one playback of a click — a sound any process may make
            // for itself — and nothing of this program is read or changed by it; anywhere else it
            // now buys nothing. The setting of FR-100 still holds: with the sound off, a posted
            // message is as silent as a press.
            if let Some(press) = press_of_sound_message_at(message, is_ui_window(hwnd)) {
                sound_press(press);
            }

            if let Some(result) = crate::hook::handle_input_message(message, wparam, lparam) {
                return result;
            }

            let handled = crate::tray::handle_ui_message(message, wparam, lparam);

            // FR-90 and FR-95. `general.enabled` is the tray's, the menu of FR-91 can flip
            // it inside the call above, and the hook has to be told — a program the user has
            // just suspended must stop swallowing the hotkey at once. Re-reading it here,
            // after every message the UI thread sees, is the whole of that wiring: it costs
            // one thread-local read, it cannot miss a change however the change was made,
            // and it needs no notification channel between the two modules. `with_tray`
            // answers `None` on the input and watcher threads, which is why this is correct
            // to run for every window of the process.
            if let Some(active) = crate::tray::with_tray(|tray| tray.enabled()) {
                crate::hook::set_active(active);
            }

            // **FR-70, FR-71, FR-73 — task T-06-1.** Two lines, and both on the input window
            // only, because the buffer they are about is a thread-local of that thread
            // (section 6.3).
            //
            // The first is the race of FR-71. `watchdog::WM_APP_FLUSH` is posted by the
            // `WinEvent` callback for `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_FOCUS` and for
            // nothing else — the two rows of the FR-10 table — so it is exactly "the focus
            // moved", already delivered to this thread by the subscription task T-03-3 made.
            // **No second subscription is raised**: module `guard` contains no
            // `SetWinEventHook`. `note_focus_moved` publishes "no answer yet" and asks the
            // watcher thread for one; the gate below then empties the buffer and switches
            // recording off, so a stroke made between the focus moving and the verdict arriving
            // is not kept — not even retroactively.
            //
            // Placed **after** `watchdog::apply_flush` above, which is the FR-10 flush of the
            // same event resolved by the timestamps of FR-12. The order matters only in that the
            // gate is stricter: FR-12 keeps what is newer than the event, and the gate then
            // removes it as well. That is the trade the task specification prescribes for SEC-06
            // and it is bounded by `guard::PROBE_BUDGET_MS` — **1550 ms** in the worst case, the
            // figure task T-13-12 corrected from 1050 when it counted level 3's two cross-process
            // transactions one by one instead of as one. The doc comment of that constant also
            // names the two calls the number does **not** cover: the first `CoCreateInstance` of
            // the process, and a client that came back without the two timeout properties.
            //
            // The second is the gate itself, on every message and for the reason
            // `apply_configured_capacity` below is: a published value re-read after every message
            // cannot be missed, whichever way it changed.
            if is_input_window(hwnd) {
                if message == crate::watchdog::WM_APP_FLUSH && caused_by_typing {
                    // ⭐ **FR-14 — the exempt half**, task Т-48-2. The probe of FR-72 is asked
                    // for exactly as it is below; what is not done is the two things that would
                    // take the user's word away — `Field::Pending` and the park. The verdict
                    // comes back by the same road, `guard::WM_APP_FIELD` → the gate below, and
                    // a `Password` still empties and overwrites the buffer there (SEC-06,
                    // decision П-4).
                    crate::guard::note_focus_moved_keeping_buffer();
                } else if message == crate::watchdog::WM_APP_FLUSH {
                    crate::guard::note_focus_moved();

                    // ⚠ **The wipe is unconditional and is here rather than in the gate below.**
                    // `note_focus_moved` publishes "no answer yet" and asks the watcher thread
                    // for a verdict, and a verdict can in principle come back before the next
                    // line runs — the two threads are not synchronised, which is the point.
                    // Leaving the wipe to the gate would then mean an `Ordinary` verdict
                    // arriving first and the strokes of the previous field surviving the focus
                    // change. Parking here closes that: the buffer is emptied and its memory
                    // overwritten (SEC-02) on the focus change itself, and the gate below only
                    // decides whether to put it back.
                    park_buffer();
                }

                apply_buffering_gate();
            }

            // FR-07, and exactly the same wiring for exactly the same reason: `[buffer]
            // capacity` is the UI thread's to read and the input thread's to obey, and
            // re-reading the published value after every message costs one atomic load and one
            // comparison, cannot miss a change however the change was made, and needs no
            // channel between the two threads. `WM_APP_CONFIGURED` is therefore only a nudge —
            // it makes a message arrive, and it is this line that decides.
            //
            // Answers nothing on the UI and watcher threads, which own no buffer.
            apply_configured_capacity();

            match handled {
                Some(result) => result,

                // SAFETY: forwarding the message unchanged to the default procedure, which
                // is what every message not named above must get. The arguments are the ones
                // the OS just passed in and are handed on unmodified; nothing is
                // dereferenced on the way.
                None => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// COM apartment of the watcher thread
// ---------------------------------------------------------------------------------------

/// The COM apartment of the thread that created it, left when this value is dropped.
struct ComApartment;

impl ComApartment {
    /// Enters a single-threaded apartment — `COINIT_APARTMENTTHREADED`.
    ///
    /// UI Automation, which task T-06-1 uses to recognise a password field (SEC-06),
    /// requires an STA. An STA also requires the thread to pump messages, which this thread
    /// does anyway: that is why the apartment and the message loop belong on the same
    /// thread.
    fn enter_sta() -> WinResult<Self> {
        // SAFETY: no reserved pointer is passed, as the signature demands. The call
        // initialises the *calling* thread's apartment and nothing else, and it is made
        // before that thread does anything COM-related. `S_FALSE` — the apartment was
        // already entered — is a success and still owes a `CoUninitialize`, which is why
        // the guard is returned in that case too rather than treated as an error.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;

        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        // Task T-37-2: the order the teardown ran in, for the test of `watcher_body`.
        #[cfg(test)]
        note_teardown("apartment");

        // SAFETY: balances exactly one successful CoInitializeEx on this same thread — the
        // value is created and dropped inside `watcher_body` and never leaves it — and it
        // runs after the message loop has returned or unwound, and after the guard declared
        // below it there has released the one COM object of this apartment that outlives a
        // probe (task T-37-2), so none is still in use. The call returns nothing, so NFR-13 has
        // no result to check.
        unsafe { CoUninitialize() };
    }
}

// ---------------------------------------------------------------------------------------
// FR-97 — self-termination of a debug build
// ---------------------------------------------------------------------------------------

/// FR-97: a debug build terminates itself after a deadline measured from start-up.
///
/// The whole mechanism sits behind `cfg(debug_assertions)` — module, deadline, environment
/// variable and the call site in [`run_as_first_instance`] alike. In a Release build it is
/// *absent from the compiled code*, not disabled at run time, which is what FR-97 asks for
/// and what acceptance point 13 verifies by reading this source rather than by searching
/// the binary for a string that `strip = true` would have removed in any case.
///
/// Why a debug build needs it at all is decision R-19: the program is resident, it has no
/// interface yet, and without a deadline there would be nothing to stop it with.
#[cfg(debug_assertions)]
mod debug_timeout {
    use std::sync::OnceLock;
    use std::thread::{self, JoinHandle, Thread};
    use std::time::{Duration, Instant};

    use windows::core::Result as WinResult;

    /// Environment variable that overrides the deadline, in seconds — FR-97.
    const TIMEOUT_ENV_VAR: &str = "LANGSW_DEBUG_TIMEOUT_SEC";

    /// Deadline used when the variable is absent or unusable: ten minutes, fixed by the
    /// user's decision on question 18.
    const DEFAULT_TIMEOUT_SECS: u64 = 600;

    /// The thread parked on the deadline, so that a shutdown requested from elsewhere wakes
    /// it instead of being noticed a poll interval later.
    static MAIN_THREAD: OnceLock<Thread> = OnceLock::new();

    /// How long the deadline path gives the other threads to come down on their own before
    /// it ends the process from here.
    ///
    /// Long enough that an ordinary shutdown — which takes milliseconds — always finishes
    /// inside it and leaves through the clean path, and short enough that acceptance point
    /// 11a's "с точностью в несколько секунд" holds for a process whose UI thread is wedged.
    const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

    /// How often the grace period above is re-examined.
    const GRACE_POLL: Duration = Duration::from_millis(20);

    /// Exit code of a debug build that had to end itself because its threads would not stop.
    ///
    /// Distinct from every other way this process can end — `EXIT_ALREADY_RUNNING` is 2 and
    /// `hook::EXIT_EMERGENCY` is 3 — so that an acceptance run can tell the FR-97 deadline
    /// firing on a wedged process apart from the same deadline firing on a healthy one,
    /// which leaves through `join_all` with a code of zero.
    const EXIT_DEBUG_TIMEOUT: u32 = 4;

    /// Blocks until the deadline passes or shutdown is requested, whichever comes first, and
    /// reports which of the two happened.
    ///
    /// `true` means the deadline expired — the FR-97 case, and the one that must not rely on
    /// any other thread. `false` means somebody else asked the program to stop, and the
    /// ordinary shutdown path is in charge.
    ///
    /// Parks rather than polls: an idle debug build must be as invisible to NFR-10 as an
    /// idle release build, and a parked thread costs exactly nothing until it is woken.
    pub fn wait_for_deadline() -> bool {
        // Registered here rather than before the threads start because it does not need to
        // be earlier: the flag is re-read at the top of every iteration below, so a request
        // that arrives before the registration is seen without any wake-up at all.
        let _ = MAIN_THREAD.set(thread::current());

        let deadline = Instant::now() + configured_timeout();

        loop {
            if super::shutdown_requested() {
                return false;
            }

            let now = Instant::now();
            if now >= deadline {
                return true;
            }

            // `park_timeout` may return early and for no reason; both conditions above are
            // re-checked on every pass, so a spurious wake-up costs one loop iteration.
            thread::park_timeout(deadline - now);
        }
    }

    /// Gives the three threads [`SHUTDOWN_GRACE`] to leave their message loops and ends the
    /// process if they have not — the second half of FR-97.
    ///
    /// Returns normally when every thread has finished, and the caller then leaves through
    /// `join_all` exactly as it always did: the tray icon is removed, the configuration is
    /// saved, the class is unregistered and the mutex is released. That is the ordinary case
    /// and it costs one poll interval.
    ///
    /// It does not return when a thread is wedged. `JoinHandle::is_finished` is what makes
    /// the distinction possible without a second bookkeeping channel — `join` itself has no
    /// timed form and would be the very thing that hangs.
    ///
    /// ⚠ Ending the process here skips the unwinding of whatever the wedged thread owns: the
    /// tray icon can be left in the notification area as a ghost until the shell repaints it,
    /// and the configuration is not saved. That is the trade FR-97 makes deliberately. The
    /// alternative is a debug build that keeps a low-level keyboard hook for as long as a
    /// menu stays open, and §4.11 of SPEC exists to say that this is the worse outcome. The
    /// hook itself is already gone by the time this is called: the caller removes it first.
    pub fn terminate_unless_threads_stop(threads: &[JoinHandle<WinResult<()>>]) {
        let grace_ends = Instant::now() + SHUTDOWN_GRACE;

        loop {
            if threads.iter().all(JoinHandle::is_finished) {
                return;
            }

            if Instant::now() >= grace_ends {
                super::terminate_this_process(EXIT_DEBUG_TIMEOUT);
                return;
            }

            thread::sleep(GRACE_POLL);
        }
    }

    /// Wakes the thread parked in [`wait_for_deadline`], if there is one.
    pub fn wake_main_thread() {
        if let Some(main) = MAIN_THREAD.get() {
            main.unpark();
        }
    }

    /// The deadline this process will use.
    fn configured_timeout() -> Duration {
        parse_timeout(std::env::var(TIMEOUT_ENV_VAR).ok().as_deref())
    }

    /// Whether the FR-97 deadline was armed from the environment for this run — task Т-25-2.
    ///
    /// Presence and not value: an unusable value still gets a deadline (see [`parse_timeout`]),
    /// and what the caller is asking about is not how long the process may live but whether it
    /// was started by a bench. The variable is the mark of one.
    pub fn deadline_is_armed() -> bool {
        std::env::var_os(TIMEOUT_ENV_VAR).is_some()
    }

    /// The name this suppression files itself under in the journal of §6.2.
    ///
    /// A row of the closed table of `diag` — SEC-07: the journal carries an index into that
    /// table, never a string a caller built.
    const SUPPRESSED_OPERATION: &str = "FR-82 notification suppressed";

    /// Withholds the FR-82 notification when the deadline is armed, and says so in the journal.
    ///
    /// The condition arrives as an argument rather than being read here, so that the decision
    /// can be tested without touching the environment of the test process — the same split
    /// [`parse_timeout`] uses, and for the same reason. [`super::already_running_is_silent`] is
    /// the one caller and reads the environment for it.
    ///
    /// The journal line is what replaces the window. It costs one `diag::record`, which is
    /// allocation-free and lock-free, and it carries a name out of the closed table and no code:
    /// nothing failed here, a window was deliberately not shown.
    pub fn notification_is_suppressed(armed: bool) -> bool {
        if !armed {
            return false;
        }

        crate::diag::record(
            crate::diag::Operation::from_name(SUPPRESSED_OPERATION),
            crate::diag::OsCode::NONE,
        );

        true
    }

    /// Reads the deadline out of the raw value of the environment variable.
    ///
    /// Split out from [`configured_timeout`] so it can be tested without touching the
    /// environment of the test process, which is shared by every test in the binary.
    fn parse_timeout(raw: Option<&str>) -> Duration {
        let seconds = raw
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        Duration::from_secs(seconds)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn absent_variable_gives_the_default_of_ten_minutes() {
            assert_eq!(parse_timeout(None), Duration::from_secs(600));
        }

        #[test]
        fn a_plain_number_is_taken_as_seconds() {
            assert_eq!(parse_timeout(Some("5")), Duration::from_secs(5));
        }

        #[test]
        fn surrounding_whitespace_is_ignored() {
            assert_eq!(parse_timeout(Some("  30\t")), Duration::from_secs(30));
        }

        #[test]
        fn an_unusable_value_falls_back_to_the_default() {
            // A debug build must terminate itself whatever the environment says, so an
            // unreadable value can only mean "use the default" — never "no deadline".
            for raw in ["", "abc", "-1", "5.5", "99999999999999999999"] {
                assert_eq!(
                    parse_timeout(Some(raw)),
                    Duration::from_secs(600),
                    "unusable value {raw:?} must fall back to the default"
                );
            }
        }

        /// **Task Т-25-2, the FR-82 half.** The notification is withheld exactly when the
        /// deadline is armed, and the journal keeps the line that stands in for the window.
        ///
        /// Both halves of the decision are asserted in one test on purpose: the ring of §6.2 is
        /// process-wide, and two tests asking about the same row could run at the same time and
        /// read each other's entry. This is the only writer of that row in this binary.
        #[test]
        fn the_fr82_notification_is_withheld_only_when_the_deadline_is_armed() {
            fn suppressions() -> usize {
                crate::diag::snapshot()
                    .iter()
                    .filter(|event| event.operation.name() == SUPPRESSED_OPERATION)
                    .count()
            }

            let before = suppressions();

            // Nobody armed a deadline: this is a person who started the program twice, and
            // FR-82 owes them a window.
            assert!(
                !notification_is_suppressed(false),
                "without the deadline the notification of FR-82 stands"
            );
            assert_eq!(
                suppressions(),
                before,
                "and nothing is written down, because nothing happened"
            );

            // A bench: the window would block a process nobody can close it for.
            assert!(
                notification_is_suppressed(true),
                "with the deadline armed the notification is withheld"
            );
            assert_eq!(
                suppressions(),
                before + 1,
                "and the journal carries the line that replaces it"
            );
        }
    }
}

/// Ends this process immediately, with the given exit code — the last resort of FR-97.
///
/// `TerminateProcess` on the process pseudo-handle and not `std::process::exit`: the point of
/// reaching this line is that another thread is wedged, and `exit` runs the at-exit handlers
/// and can be made to wait by exactly the kind of thread that got us here.
///
/// Debug-only, like the whole of FR-97. The emergency combination of FR-96 has a termination
/// of its own in module `hook`, because that one must be present in both configurations and
/// must be reachable from inside the hook callback.
#[cfg(debug_assertions)]
fn terminate_this_process(code: u32) {
    // SAFETY: `GetCurrentProcess` returns the process pseudo-handle, a constant naming the
    // calling process; it needs no closing and cannot be invalid. `TerminateProcess` on it is
    // the documented way for a process to end itself at once. Both arguments are values and
    // neither call dereferences anything of ours.
    if let Err(error) = unsafe { TerminateProcess(GetCurrentProcess(), code) } {
        // NFR-13: examined. Nothing can be done about it — the caller has already removed
        // the hook, so the machine has its keyboard back either way.
        report_non_critical("TerminateProcess", &error);
    }
}

// ---------------------------------------------------------------------------------------
// Instrumentation of the acceptance bench — SEC-04a, feature `testing`
// ---------------------------------------------------------------------------------------

/// The **file** sink of the acceptance instrumentation: the numbers, once, at shutdown.
///
/// Compiled **only** under the cargo feature `testing`, which SEC-04a defines for this purpose
/// — automating the acceptance checks of §11.5 — and which is absent from the Release
/// configuration; acceptance criterion 8 of §13 of SPEC checks that the shipped binary carries
/// no trace of the feature. The same gate module [`crate::hook`]'s fault injection sits behind,
/// and for the same reason.
///
/// # Why the program has to measure itself
///
/// NFR-08 is a deadline on the program's own start-up — under fifty milliseconds from launch to
/// an installed hook — and no observer outside the process can see the instant the hook went
/// up. Point 19 of task T-03-2a is the same shape: it asks how many strokes are in the typing
/// buffer after a synthetic `ghbdtn`, and SEC-04 has removed every inter-process entry point on
/// purpose, so there is nothing to ask.
///
/// # This module holds nothing — task T-03-4
///
/// It used to own the counters and the start-up marks. They are module [`crate::control`]'s
/// now, and that module is the **single source** both sinks read (decision Р-28): the live
/// channel of SEC-04a, and the file below. What is left here is one function that formats and
/// writes, because a channel needs a live process and a client while a run that merely expired
/// under FR-97 still has to leave its numbers somewhere.
///
/// **The keys and the format of the file are unchanged, to the character.** Tasks are wired to
/// this file by its shape, so growth goes through the channel and not through here.
///
/// # SEC-01, SEC-07
///
/// What leaves this module is two durations and a row of counts. **No key code, no scan code,
/// no character and nothing derived from one**, and nothing that could carry one may ever be
/// added to the report below. The length of the buffer is the one number SEC-04a allows a
/// debug channel to publish, and it is published as a number and as nothing else; `Stroke` has
/// neither `Debug` nor `Display`, so the content is not expressible here even by mistake.
///
/// # Threading
///
/// The file is written by the **main** thread, after every one of the three threads of section
/// 6.1 has been joined: section 6.1 leaves file I/O to the UI thread, NFR-05 forbids it near
/// the callback, and the main thread is neither of those places.
#[cfg(feature = "testing")]
mod acceptance {
    /// Environment variable naming the file the report is written to.
    ///
    /// Absent means silent, which is what every run that did not ask for a report gets.
    const REPORT_ENV_VAR: &str = "LANGSW_TESTING_REPORT";

    /// Writes the numbers to the file named by [`REPORT_ENV_VAR`], if the variable is set.
    pub fn write_report() {
        let Ok(path) = std::env::var(REPORT_ENV_VAR) else {
            return;
        };

        // Decision Р-28: one source, two sinks. Everything above the subscriptions comes from
        // the same snapshot the channel of SEC-04a publishes, so the two can never disagree.
        let state = crate::control::snapshot();

        // Task T-03-3 added the eight counts of module `watchdog`. Every one of them is a
        // count of events — SEC-07 allows those and nothing else, and there is no stroke, key
        // code or character anywhere in this structure to put here even by mistake.
        let subscriptions = crate::watchdog::counters();

        let report = format!(
            "hook_ready_us={}\ncache_ready_us={}\ncache_builds={}\nbuffer_len={}\nlayout_cache_failures={}\nhotkey_handoffs={}\npost_failures={}\nmouse_packets={}\nmouse_flushes={}\nwindow_flushes={}\nwindow_flushes_taken={}\nfull_clears={}\npartial_clears={}\nkept_events={}\nstrokes_removed={}\ndevice_changes={}\nlayout_probes={}\n",
            state.hook_ready_us,
            state.cache_ready_us,
            state.cache_builds,
            // The latched value and not the live mirror: by the time this runs the input
            // thread is gone and its buffer has been dropped and zeroed (SEC-02), so the live
            // number is a truthful zero and a useless one. See `control::note_buffer_len_at_exit`.
            state.buffer_len_at_exit,
            state.layout_cache_failures,
            state.hotkey_handoffs,
            state.post_failures,
            subscriptions.mouse_packets,
            subscriptions.mouse_flushes,
            subscriptions.window_flushes,
            subscriptions.window_flushes_taken,
            subscriptions.full_clears,
            subscriptions.partial_clears,
            subscriptions.kept_events,
            subscriptions.strokes_removed,
            subscriptions.device_changes,
            subscriptions.layout_probes,
        );

        // A failed write is not worth ending on: the process is already leaving, and the run
        // that asked for a report notices an absent file without being told.
        let _ = std::fs::write(path, report);
    }
}

// ---------------------------------------------------------------------------------------
// FR-98 — the panic hook
// ---------------------------------------------------------------------------------------

/// Installs the panic hook FR-98 requires.
///
/// The hook runs before the process is torn down, and it runs even under `panic = "abort"`,
/// which is what the Release profile sets: abort happens after the hook, not instead of it.
///
/// Task **T-03-1** added the `UnhookWindowsHookEx` call this body exists for, and the
/// exception in front of it.
fn install_panic_hook() {
    let previous = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        // FR-99, and the one case in which this hook must do nothing at all. A panic raised
        // inside the callback of the keyboard hook is caught by the callback itself, counted,
        // and answered by letting the stroke through — the program "остаётся запущенной" by
        // the plain words of FR-99. Tearing the process down here would make that impossible,
        // because this hook runs *before* the `catch_unwind` that is about to absorb the
        // panic. Returning early also keeps the callback free of I/O on the panic path
        // (NFR-05): the standard hook below prints, and nothing here prints.
        //
        // In a `panic = "abort"` build — the Release profile — this is always false: nothing
        // unwinds, nothing can be absorbed, and the branch below is the only one taken.
        if crate::hook::panic_is_absorbed() {
            return;
        }

        // FR-98, and first, before anything else in this body. Whatever happens next, the
        // machine gets its keyboard back: a panicking process that still holds a
        // `WH_KEYBOARD_LL` hook is the exact failure §4.11 of SPEC was written against.
        // `hook::uninstall` is callable from any thread, takes the handle out of an atomic
        // with a single swap and blocks on nothing, which is what a panic hook needs — one
        // that waited on a lock could deadlock against the thread that panicked while
        // holding it.
        crate::hook::uninstall();

        // Nothing here blocks either: an atomic store, three non-blocking posts and an
        // unpark.
        request_shutdown();

        // SEC-01 and SEC-07: a panic message must never carry a keystroke, a key code or
        // buffer contents. The previous hook is the standard library's, which prints the
        // payload the program itself wrote, so the requirement is a rule about what this
        // program is allowed to put into a panic message, and it is stated here because
        // this is where it would be violated. Module `hook` holds up its end: there is no
        // `panic!`, `assert!`, `unwrap` or `expect` anywhere on its callback path, and the
        // payload FR-99 catches is dropped unread rather than formatted.
        previous(info);
    }));
}

// ---------------------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------------------

/// Records a Win32 failure that must not take the process down.
///
/// NFR-13 asks two things of the result of every Win32 call: that it is examined, and that
/// the failure of a non-critical operation is journalled rather than escalated. Everything
/// that cannot propagate a `Result` — the `Drop` implementations and the wake-up posts —
/// converges here, so that wiring in the journal is one edit instead of a dozen.
///
/// The in-memory ring journal is module `diag`, task **T-06-4**, and this is the third and
/// last edit that task makes to this file.
///
/// `pub(crate)` rather than private since task T-01-4: `tray` has the same kind of failures
/// to record — a shell that is not up yet, a handle that will not free — and a second copy
/// of this function there would defeat the whole point of having one place to wire the
/// journal into.
///
/// # SEC-01, SEC-07 — what crosses into the journal here
///
/// `operation` is a `&str` and the journal has no field a `&str` could go into. This line is
/// where the two meet, and the meeting is a **narrowing**: `Operation::from_name` maps the
/// name onto the closed table of module `diag` and answers `Operation::UNLISTED` for anything
/// that is not in it, keeping none of the text. So the requirement does not depend on every
/// caller of this function passing something harmless — one of them already passes a
/// `format!` — it depends on the type on the other side of the call, which has no room for a
/// character whatever is passed here.
///
/// `error` contributes its `HRESULT` and nothing else. The message inside a
/// `windows::core::Error` is text and is dropped unread, for the same reason the panic
/// payload is dropped unread in `join_all`.
///
/// # NFR-01 to NFR-05
///
/// Both calls below are allocation-free, lock-free and free of input-output, which is what
/// lets this function keep being called from `Drop` implementations and from code next to the
/// hook callback. Nothing is formatted here; module `diag` formats only when it dumps.
pub(crate) fn report_non_critical(operation: &str, error: &WinError) {
    crate::diag::record(
        crate::diag::Operation::from_name(operation),
        crate::diag::OsCode::of(error),
    );
}

// ---------------------------------------------------------------------------------------
// Tests of the input path — task T-03-2a
// ---------------------------------------------------------------------------------------

/// What can be driven without a window, a hook or a keyboard.
///
/// The three functions tested here are the whole of what this module does *to* the buffer, and
/// they were written as separate functions so that they could be reached from here: the window
/// procedure and the start-up pipeline around them are Win32 and are verified on the running
/// program instead, which is the only honest place for them.
///
/// The buffer is a thread-local (section 6.3) and every test runs on its own thread, so the
/// recorder one test installs is invisible to the next.
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::buffer::{self, Recorder};
    use crate::hook::{Edge, KeyEvent};
    use crate::layouts::LayoutCache;

    /// Scan code and virtual key of `A`. Nothing here depends on which key it is.
    const VK_A: u16 = 0x41;
    const SCAN_A: u16 = 0x1E;

    /// A layout no cache in this file contains — the lookup then answers "no characters",
    /// which is an outcome and not an error (FR-23).
    const SOME_LAYOUT: LayoutId = LayoutId::from_raw(0x0409_0409);

    /// The tests that rebuild the cache of FR-20 take this first — task T-39-5a.
    ///
    /// `cache_builds` is one counter for the whole process and `cargo test` runs the tests of this
    /// binary on parallel threads: a device change answered by one test landed inside the
    /// measurement of the other, and the instrument of finding Н11 read a rebuild it had not
    /// caused. A poisoned gate is still a gate — the test that poisoned it has already reported.
    static CACHE_BUILDS_GATE: Mutex<()> = Mutex::new(());

    fn cache_builds_gate() -> std::sync::MutexGuard<'static, ()> {
        CACHE_BUILDS_GATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// A [`Beeper`] that writes down what it was asked for instead of making a sound —
    /// **FR-100, task Т-21-5**.
    ///
    /// The seam exists for one reason: the sound is a side effect on the machine, and a test
    /// that made real sounds could only assert that it did not crash. Behind the seam the whole
    /// of FR-100 is a function of two booleans, which is what the tests below drive.
    #[derive(Default)]
    struct Bench {
        /// Every tone asked for, in order.
        tones: Vec<Tone>,
    }

    impl Beeper for Bench {
        fn beep(&mut self, tone: Tone) {
            self.tones.push(tone);
        }
    }

    /// **The two sounds are real, complete `.wav` files, and they are not the same sound.**
    ///
    /// The bytes reach the audio service through `SND_MEMORY`, which parses them as a RIFF image
    /// — a truncated or mis-typed file is not a build error and not a panic, it is silence on
    /// the user's machine. So the header is read here, by hand, out of the very slices the
    /// program plays: `RIFF`/`WAVE`, format 1 (PCM), 16 bits, and a `data` chunk with something
    /// in it.
    ///
    /// The last assertion is the one the tuning of stage Э21 is about: «чк-чк» and «чк» must be
    /// two different sounds. Pointing both constants at one file would leave every other test in
    /// this module green.
    #[test]
    fn both_sounds_are_complete_wav_images_and_differ_from_each_other() {
        for (what, wave) in [
            ("replaced", SOUND_REPLACED),
            ("idle", SOUND_IDLE),
            ("refused", SOUND_REFUSED),
        ] {
            assert!(
                wave.len() > 44,
                "{what}: a RIFF header alone is 44 bytes, this is {}",
                wave.len()
            );
            assert!(
                wave.len() < 64 * 1024,
                "{what}: a click is milliseconds long; {} bytes is something else",
                wave.len()
            );
            assert_eq!(&wave[0..4], b"RIFF", "{what}: not a RIFF file");
            assert_eq!(&wave[8..12], b"WAVE", "{what}: not a WAVE file");
            assert_eq!(&wave[12..16], b"fmt ", "{what}: no format chunk first");

            let format = u16::from_le_bytes([wave[20], wave[21]]);
            let bits = u16::from_le_bytes([wave[34], wave[35]]);

            assert_eq!(
                format, 1,
                "{what}: only uncompressed PCM is asked of the mixer"
            );
            assert_eq!(bits, 16, "{what}: 16 bits per sample");
            assert_eq!(
                &wave[36..40],
                b"data",
                "{what}: no data chunk where one is due"
            );

            let data = u32::from_le_bytes([wave[40], wave[41], wave[42], wave[43]]) as usize;

            assert!(data > 0, "{what}: the data chunk is empty");
            assert_eq!(
                44 + data,
                wave.len(),
                "{what}: the data chunk and the file disagree about the length"
            );
        }

        assert_ne!(
            SOUND_REPLACED, SOUND_IDLE,
            "«чк-чк» and «чк» have to be two different sounds — that is the whole feature"
        );
        assert!(
            SOUND_REPLACED.len() > SOUND_IDLE.len(),
            "the answer that says «done» is the longer of the two: one click against two"
        );

        // Task Т-49-2. The third has to differ from **both**, and pointing it at either of them
        // would leave every other test in this module green — the same trap this test was
        // written for when there were two.
        assert_ne!(SOUND_REFUSED, SOUND_REPLACED);
        assert_ne!(
            SOUND_REFUSED, SOUND_IDLE,
            "the refusal is the idle click with a low body under it, not the idle click"
        );
        assert!(
            SOUND_REFUSED.len() > SOUND_IDLE.len(),
            "the body decays for 62 ms after the click, so the refusal is the longer of the two"
        );
    }

    /// Each tone names its own bytes and nobody else's.
    #[test]
    fn each_tone_carries_the_wave_that_belongs_to_it() {
        assert_eq!(Tone::Replaced.wave(), SOUND_REPLACED);
        assert_eq!(Tone::Idle.wave(), SOUND_IDLE);
        assert_eq!(Tone::Refused.wave(), SOUND_REFUSED);
    }

    /// **FR-100 — one press, one tone, and no two of the three are the same tone.**
    #[test]
    fn a_press_answers_with_the_tone_its_outcome_earned() {
        let mut bench = Bench::default();

        answer_press(&mut bench, Press::Replaced, true);
        answer_press(&mut bench, Press::Idle, true);
        answer_press(&mut bench, Press::Refused, true);

        assert_eq!(
            bench.tones,
            vec![Tone::Replaced, Tone::Idle, Tone::Refused],
            "a replacement, an idle press and a refusal are told apart by ear or not at all"
        );
    }

    /// **The switch of `[feedback] sound`, and it is the whole of «off».**
    ///
    /// All three tones together: the user asked for a third sound, not for a third preference,
    /// and a refusal that went on sounding with the setting off would be exactly the surprise
    /// FR-100's switch exists to prevent.
    #[test]
    fn the_switch_of_the_setting_silences_every_tone() {
        let mut bench = Bench::default();

        answer_press(&mut bench, Press::Replaced, false);
        answer_press(&mut bench, Press::Idle, false);
        answer_press(&mut bench, Press::Refused, false);

        assert!(
            bench.tones.is_empty(),
            "with the setting off nothing is sounded at all: {:?}",
            bench.tones
        );
    }

    /// **Task T-42-6, finding Н112** — a window that refuses is answered by the same third
    /// tone, exactly once, and by silence when `[feedback] sound` is off.
    ///
    /// [`sound_refusal`] is the door `src\settings.rs` calls when the ninth tick is refused; it
    /// is a wrapper over [`sound_press`], which is [`answer_press`] with the production beeper.
    /// The production beeper would make a noise in a test, so what is measured here is the pair
    /// underneath it — the outcome the door carries and the setting it obeys — through the
    /// bench. ⛔ The invariant the task rests on is the second half: with the sound off the tone
    /// is `None` **and the tick still does not go in** (`cycle_change_refusal` does not consult
    /// the setting at all — see its test in `tests\settings.rs`).
    #[test]
    fn a_window_that_refuses_earns_one_dull_knock_and_obeys_the_setting() {
        let mut bench = Bench::default();

        answer_press(&mut bench, Press::Refused, true);

        assert_eq!(
            bench.tones,
            vec![Tone::Refused],
            "one refusal of a window, one knock — the very tone a refused press earns"
        );

        let mut silent = Bench::default();

        answer_press(&mut silent, Press::Refused, false);

        assert!(
            silent.tones.is_empty(),
            "with `[feedback] sound` off the refusal is silent — and still a refusal: {:?}",
            silent.tones
        );
    }

    /// The rule apart from the calling of it — the form every other decision of this program
    /// takes, and what lets the two above be about the wiring rather than about the arithmetic.
    #[test]
    fn the_tone_of_a_press_is_a_function_of_the_outcome_and_the_setting() {
        assert_eq!(tone_for(Press::Replaced, true), Some(Tone::Replaced));
        assert_eq!(tone_for(Press::Idle, true), Some(Tone::Idle));
        assert_eq!(tone_for(Press::Refused, true), Some(Tone::Refused));

        for press in [Press::Replaced, Press::Idle, Press::Refused] {
            assert_eq!(tone_for(press, false), None, "{press:?} with the sound off");
        }
    }

    /// **The hole of task Т-49-2, as the three booleans that decide it.**
    ///
    /// The user, 2026-09-09: «в поле пароля когда вводишь, нет никакого звукового сигнала».
    /// The row that was missing is the first one below — the buffer parked by FR-70 **and** a
    /// program that is refusing rather than idling.
    #[test]
    fn a_press_the_program_refuses_earns_a_voice_of_its_own() {
        // The buffer is parked (FR-70, FR-84) and the program is refusing: the arm that did not
        // exist. `replaced` cannot be true here — with the buffer parked nothing is converted —
        // and the rule does not consult it.
        assert_eq!(
            press_outcome(false, true, false),
            Some(Press::Refused),
            "a password field and an excluded process are refusals, not idle presses"
        );

        // The buffer is not on this thread and nothing is being refused — the UI and watcher
        // threads, and the input thread before the pipeline starts. There is nothing to say.
        assert_eq!(press_outcome(false, false, false), None);

        // With the buffer here the rule is exactly what it was before this task.
        assert_eq!(press_outcome(true, false, true), Some(Press::Replaced));
        assert_eq!(press_outcome(true, false, false), Some(Press::Idle));

        // ⚠ **And the refusal never outranks a replacement that really happened.** The state can
        // move between the press and the message — the watcher thread publishes verdicts — so
        // the rule is written to prefer the fact over the flag: text the user can see changed.
        assert_eq!(press_outcome(true, true, true), Some(Press::Replaced));
    }

    /// Each of the three messages carries one outcome, and every other message carries none.
    ///
    /// SEC-05: the family is a closed set of three numbers. A message outside it must fall
    /// through to the rest of the window procedure rather than make a sound.
    #[test]
    fn the_three_sound_messages_map_one_to_one_onto_the_three_outcomes() {
        for press in [Press::Replaced, Press::Idle, Press::Refused] {
            assert_eq!(
                press_of_sound_message(sound_message_for(press)),
                Some(press),
                "{press:?} does not survive the round trip through its message"
            );
        }

        assert_eq!(
            press_of_sound_message(WM_APP_SOUND_REFUSED),
            Some(Press::Refused)
        );
        assert_eq!(press_of_sound_message(crate::hook::WM_APP_HOTKEY), None);
        assert_eq!(press_of_sound_message(WM_APP_SOUND_REFUSED + 1), None);
    }

    /// **A sound message answered only at the UI window — task T-36-4, finding С15.**
    ///
    /// The branch used to answer wherever the message landed, while four gates of
    /// `is_input_window` and one of `is_watcher_window` stood beside it in the same procedure
    /// asking exactly this question for their own messages. The window a forger would aim at is
    /// the **input** one: that thread holds the hook, section 6.1 gives it no budget for this
    /// kind of work, and FR-80 takes the hook off a thread that stops answering.
    ///
    /// All **three** messages of the family are checked, not two: task Т-49-2 added
    /// `WM_APP_SOUND_REFUSED` and widened the surface a forgery has.
    #[test]
    fn a_sound_message_is_answered_at_the_ui_window_and_nowhere_else() {
        for press in [Press::Replaced, Press::Idle, Press::Refused] {
            let message = sound_message_for(press);

            assert_eq!(
                press_of_sound_message_at(message, true),
                Some(press),
                "{press:?} must still be answered at the UI window — this is the program's own \
                 post, including the one `hook::post_hotkey` makes when a press is lost (Н40)"
            );
            assert_eq!(
                press_of_sound_message_at(message, false),
                None,
                "SEC-05: {press:?} posted to the input or the watcher window buys nothing"
            );
        }

        // And the gate does not turn a stranger into a sound at the right window either.
        assert_eq!(
            press_of_sound_message_at(crate::hook::WM_APP_HOTKEY, true),
            None
        );
    }

    /// **The gate of С15 is on the branch, and the ungated call is gone — task T-36-4.**
    ///
    /// The pure function above says what the gate decides; this says that the window procedure
    /// really goes through it. Both halves are needed: a predicate nobody calls is as silent as
    /// no predicate at all.
    ///
    /// ⚠ The needles are identifiers with their opening bracket and never whole call lines —
    /// `rustfmt` reflows a line and a needle written as one stops matching without anything
    /// having changed (the trap of stage Э32). The product half of the file only, for the reason
    /// `the_shutdown_handshake_walks_on_seqcst` states: the needles are written out here, and a
    /// sweep over the whole text would find them in their own argument lists.
    ///
    /// The positive control is the count itself: the gated form must appear **once**, so a zero —
    /// which is what a removed gate leaves behind — fails.
    #[test]
    fn the_sound_branch_of_the_window_procedure_goes_through_the_gate() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("app.rs"),
        )
        .expect("src/app.rs must be readable")
        .replace("\r\n", "\n");

        let product = source
            .split_once("\n#[cfg(test)]\nmod tests {")
            .expect("this file ends in its own test module")
            .0;

        // The procedure itself, not the whole file: the ungated call is legitimate — and
        // necessary — inside the definition of the gate, which is where the family is decoded.
        let procedure = product
            .split_once("unsafe extern \"system\" fn window_proc(")
            .expect("the window procedure must be in this file")
            .1;

        let gated = procedure.matches("press_of_sound_message_at(").count();
        let ungated = procedure.matches("press_of_sound_message(message)").count();

        println!("window_proc: gated x{gated}, ungated x{ungated}");

        assert_eq!(
            gated, 1,
            "С15: the branch calls the gated form, and it is the only call of it here"
        );
        assert_eq!(
            ungated, 0,
            "С15: and the ungated call is gone from the procedure; a sound message aimed at the \
             input window must not be answered on the thread that holds the hook"
        );
        assert_eq!(
            product.matches("fn is_ui_window(").count(),
            1,
            "the third gate of the family exists beside is_input_window and is_watcher_window"
        );
    }

    /// Types one ordinary key into the buffer of this thread.
    fn press() {
        buffer::record(KeyEvent {
            vk: VK_A,
            edge: Edge::Down,
            extra_info: 0,
            scan: SCAN_A,
            flags: 0,
            time: 0,
        });
    }

    /// FR-25 and **finding Н7 — task T-39-4.** A cache that will not build is answered with the
    /// hardwired table of module `convert` **only where there is no cache to keep** — at start-up,
    /// before any cache has been published. A rebuild that fails over a working cache leaves that
    /// cache in place: until this task it was replaced by the RU/EN table, every layout but those
    /// two stopped existing, a cycle of three broke, and nobody was told. Every failure is counted
    /// either way.
    ///
    /// ⚠ **The canon of this test moved with the task, as the task declared it would:** it used to
    /// be `a_failed_build_falls_back_to_the_hardwired_table_of_fr25`, asserting the table for every
    /// failure, a working cache or none.
    ///
    /// Both directions are one test on purpose: [`LAYOUT_CACHE_FAILURES`] is a counter of the
    /// process, `cargo test` runs the tests of a binary in parallel, and two tests asserting on
    /// the same counter would be asserting on each other's timing.
    #[test]
    fn a_failed_build_falls_back_to_fr25_only_where_there_is_no_cache_to_keep() {
        let built = crate::convert::fallback_cache();
        let before = layout_cache_failures();

        // A cache that built is used exactly as it is, whatever the buffer held, and nothing is
        // counted.
        assert_eq!(
            cache_or_fallback(Ok(built.clone()), false, &[]),
            Some(built.clone())
        );
        assert_eq!(
            cache_or_fallback(Ok(built.clone()), true, &[]),
            Some(built.clone())
        );
        assert_eq!(layout_cache_failures(), before);

        for reason in [
            LayoutError::Enumeration,
            LayoutError::NoUsableLayouts,
            LayoutError::Empty,
        ] {
            // Start-up: nothing to keep, so the very cache FR-25 prescribes, and not an empty one —
            // point 4 of FR-25 keeps "the cache did not build" apart from "no characters". With no
            // session list to file it under, it carries the two identifiers it always did.
            let fallen_back = cache_or_fallback(Err(reason), false, &[]);
            assert_eq!(
                fallen_back.as_ref(),
                Some(&built),
                "start-up, failure {reason:?}"
            );
            assert!(
                fallen_back.is_some_and(|cache| !cache.is_empty()),
                "failure {reason:?}"
            );

            // Н7: a rebuild that failed over a working cache hands nothing back to replace it.
            let replacement = cache_or_fallback(Err(reason), true, &[]);
            assert!(
                replacement.is_none(),
                "Н7: a rebuild that failed ({reason:?}) over a working cache handed back FR-25's \
                 two layouts to replace the session's"
            );
        }

        // Н10 — task T-39-6, decision 122г: the table is filed under the layouts the session really
        // has in its two languages, so the buffer finds its active layout in it; a third language
        // gets no table. In this test and not in its own, for the counter above.
        let russian_typewriter = LayoutId::from_raw(0xF008_0419);
        let us_international = LayoutId::from_raw(0xF002_0409);
        let german = LayoutId::from_raw(0x0407_0407);

        let filed = cache_or_fallback(
            Err(LayoutError::Empty),
            false,
            &[russian_typewriter, german, us_international],
        )
        .expect("nothing to keep: the table of FR-25");
        let (russian, us, third) = (
            filed.contains(russian_typewriter),
            filed.contains(us_international),
            filed.contains(german),
        );

        assert!(
            russian && us && !third,
            "Н10: the table of FR-25 over the session — Russian (typewriter) found: {russian}; \
             US-International found: {us}; German given a table: {third}"
        );

        assert_eq!(
            layout_cache_failures(),
            before + 7,
            "every failure is counted"
        );
    }

    /// **Finding Н11 — task T-39-4, and the instrument of premise П5 run in process.**
    ///
    /// The probe of FR-21 decided «rebuild the cache» by asking whether the layout of the window in
    /// front differed from the stamp — a different question from the one it answered. So an
    /// ordinary `Alt+Shift` ran the whole sweep of FR-20 over an unchanged list, and a layout added
    /// to the session without touching the active one was never taken in. Both halves are measured
    /// on `cache_builds`, the counter of completed builds, against the session this test runs in:
    /// a window whose layout differs from the stamp over a cache that holds the whole list — a
    /// rebuild there is wasted — and a stamp equal to the window over a cache that lacks a layout
    /// of the session — a rebuild there is owed.
    ///
    /// It reads the real foreground window and the real layout list, and says so and measures
    /// nothing on a session that cannot stage it.
    #[cfg(feature = "testing")]
    #[test]
    fn the_probe_of_fr21_rebuilds_for_a_changed_list_and_not_for_a_changed_window() {
        let _gate = cache_builds_gate();

        let Ok(session) = crate::layouts::enumerate() else {
            println!("SKIPPED: the layout list of this session could not be read");
            return;
        };
        let window = foreground_layout();

        if window == LayoutId::default() || !session.contains(&window) {
            println!("SKIPPED: no foreground window on a layout of this session");
            return;
        }

        let Some(other) = session.iter().copied().find(|layout| *layout != window) else {
            println!("SKIPPED: this session carries one layout, nothing to stage");
            return;
        };

        let whole = LayoutCache::build().expect("the layouts of this session build a cache");
        let short = LayoutCache::from_maps(
            whole
                .maps()
                .iter()
                .filter(|map| map.layout() != other)
                .cloned()
                .collect(),
        )
        .expect("the cache keeps every layout but one");

        buffer::install_recorder(Recorder::with_capacity(16));

        // The window moved off the stamp; the list is exactly the one the cache holds.
        publish_cache(whole);
        publish_active_layout(other);
        let before = crate::control::snapshot().cache_builds;
        refresh_layout_and_cache();
        let wasted = crate::control::snapshot().cache_builds.wrapping_sub(before);

        // The stamp is the window's; the session has a layout the cache does not.
        publish_cache(short);
        publish_active_layout(window);
        let before = crate::control::snapshot().cache_builds;
        refresh_layout_and_cache();
        let owed = crate::control::snapshot().cache_builds.wrapping_sub(before);

        buffer::uninstall();

        assert!(
            wasted == 0 && owed == 1,
            "Н11: a probe over an unchanged list rebuilt the cache {wasted} time(s), expected 0; a \
             probe over a list with a layout the cache lacks rebuilt it {owed} time(s), expected 1"
        );
    }

    /// **Finding Н25 — task T-39-5: a device change reaches the cache while the buffer is parked.**
    ///
    /// `WM_DEVICECHANGE` — a keyboard plugged in or pulled out — is answered at the input window
    /// with a rebuild of the cache of FR-20. The branch asked `buffer::is_installed` whether this
    /// was the input thread, and FR-70 takes the buffer off that very thread for the interval of a
    /// password field: a keyboard arriving then was thrown away unread and the cache stayed as it
    /// was. The layout probe beside it had the same trap repaired in task T-10-0f, by asking the
    /// register of windows instead.
    ///
    /// Observed on the recorder itself rather than on a counter of the process: a cache that is not
    /// the session's — one map of FR-25 — is published first, and a rebuild is what replaces it.
    #[test]
    fn a_device_change_rebuilds_at_the_input_window_even_with_the_buffer_parked() {
        const WM_DEVICECHANGE: u32 = 0x0219;

        let _gate = cache_builds_gate();

        let marker = || {
            LayoutCache::from_maps(vec![crate::convert::fallback_cache().maps()[0].clone()])
                .expect("one map of FR-25 is a cache")
        };

        // At the input window, with the buffer parked by the gate of FR-70.
        buffer::install_recorder(Recorder::with_capacity(8));
        publish_cache(marker());
        park_buffer();
        answer_device_change(WM_DEVICECHANGE, true);
        let rebuilt_while_parked =
            with_recorder_wherever_it_is(|recorder| recorder.cache().cloned())
                != Some(Some(marker()));
        restore_buffer();
        buffer::uninstall();

        // At a window that is not the input one: the UI and watcher windows hear the broadcast too.
        buffer::install_recorder(Recorder::with_capacity(8));
        publish_cache(marker());
        answer_device_change(WM_DEVICECHANGE, false);
        let rebuilt_elsewhere =
            buffer::with(|recorder| recorder.cache().cloned()) != Some(Some(marker()));
        buffer::uninstall();

        assert!(
            rebuilt_while_parked && !rebuilt_elsewhere,
            "Н25: a device change at the input window with the buffer parked rebuilt the cache: \
             {rebuilt_while_parked}; one at a window that is not the input one rebuilt it: \
             {rebuilt_elsewhere}"
        );
    }

    /// **FR-11.** Neither a new active layout nor a rebuilt cache flushes the buffer — the two
    /// calls this module makes into the buffer on the `WM_INPUTLANGCHANGE` path, driven here
    /// exactly as the window procedure makes them.
    #[test]
    fn neither_a_layout_change_nor_a_rebuilt_cache_flushes_the_buffer() {
        buffer::install_recorder(Recorder::with_capacity(16));

        for _ in 0..3 {
            press();
        }
        assert_eq!(buffer::len(), 3);

        // The user switched layout: FR-21 says rebuild, FR-11 says keep what was typed.
        publish_active_layout(SOME_LAYOUT);
        assert_eq!(buffer::len(), 3, "FR-11: a layout change flushes nothing");

        publish_cache(crate::convert::fallback_cache());
        assert_eq!(buffer::len(), 3, "FR-11: a rebuilt cache flushes nothing");

        // And again, because the messages of FR-21 arrive as often as the user presses the
        // layout switch: a rule that held once and not twice would be no rule.
        publish_active_layout(LayoutId::default());
        publish_cache(crate::convert::fallback_cache());
        assert_eq!(buffer::len(), 3);

        buffer::uninstall();
    }

    /// The cache really does reach the buffer — a rebuild that published nothing would satisfy
    /// the test above by doing no work at all.
    #[test]
    fn the_published_cache_is_the_one_the_buffer_answers_from() {
        buffer::install_recorder(Recorder::with_capacity(4));

        assert_eq!(buffer::with(|recorder| recorder.has_cache()), Some(false));

        publish_cache(
            LayoutCache::from_maps(crate::convert::fallback_cache().maps().to_vec())
                .expect("the hardwired table of FR-25 is never empty"),
        );

        assert_eq!(buffer::with(|recorder| recorder.has_cache()), Some(true));

        buffer::uninstall();
    }

    /// FR-07: the capacity published by the UI thread is applied, and applied **once**.
    #[test]
    fn the_configured_capacity_is_applied_only_when_it_differs() {
        buffer::install_recorder(Recorder::with_capacity(8));
        press();
        assert_eq!(buffer::len(), 1);

        // Nothing published yet: the buffer is left exactly as it is, strokes included.
        apply_capacity(0);
        assert_eq!(buffer::with(|recorder| recorder.capacity()), Some(8));
        assert_eq!(buffer::len(), 1);

        // The configured value is the one already in force — and this is the case that runs on
        // every message of the input thread, so it must not touch the buffer.
        apply_capacity(8);
        assert_eq!(
            buffer::len(),
            1,
            "an unchanged capacity must not empty the buffer"
        );

        // A value the buffer clamps. Applied once, and then recognised as already in force,
        // which is what keeps the ring from being rebuilt on every message for ever.
        apply_capacity(usize::MAX);
        assert_eq!(
            buffer::with(|recorder| recorder.capacity()),
            Some(crate::buffer::MAX_CAPACITY)
        );

        press();
        assert_eq!(buffer::len(), 1);
        apply_capacity(usize::MAX);
        assert_eq!(
            buffer::len(),
            1,
            "the clamped capacity is compared, not the raw one"
        );

        buffer::uninstall();
    }

    /// A thread with no buffer is every thread but the input one, and none of the three calls
    /// above may fail there — the window procedure runs on all of them.
    ///
    /// Since task T-10-0f "no buffer" means neither installed **nor parked**: the publications
    /// look into [`PARKED_BUFFER`] too, and on every thread but the input one both slots are
    /// forever empty, which is what this asserts by running on a thread that never had either.
    #[test]
    fn a_thread_without_a_buffer_is_left_alone() {
        assert!(!buffer::is_installed());

        apply_capacity(512);
        publish_active_layout(SOME_LAYOUT);
        publish_cache(crate::convert::fallback_cache());

        assert!(!buffer::is_installed());
        assert_eq!(
            with_recorder_wherever_it_is(|recorder| recorder.active_layout()),
            None,
            "a thread that never had a recorder has nothing parked either"
        );
    }

    /// **Task T-10-0f: the layout probe's publication reaches a parked recorder.** The
    /// regression behind «первое нажатие моргает»: the probe of FR-21 arrives during the
    /// interval of FR-71 — the recorder is parked — and before this task the publications
    /// went through `buffer::with` alone, so every focus-change probe refreshed nothing and
    /// the recorder woke up with the stale layout that stamped the next strokes (FR-26
    /// converting «в себя»). Driven here exactly as the window procedure drives it: park,
    /// publish, restore, and the fresh layout must be what the restored recorder answers.
    #[test]
    fn the_layout_probe_publication_reaches_a_parked_recorder() {
        let other = LayoutId::from_raw(0x0419_0419);

        buffer::install_recorder(Recorder::with_capacity(8));
        publish_cache(crate::convert::fallback_cache());
        publish_active_layout(SOME_LAYOUT);

        // The interval of FR-71: the focus moved, no verdict yet, the recorder is parked.
        park_buffer();
        assert!(!buffer::is_installed());

        // The read half of `refresh_layout_and_cache` sees the parked recorder's layout —
        // before this task it answered `None` here and the refresh was refused outright.
        assert_eq!(
            with_recorder_wherever_it_is(|recorder| recorder.active_layout()),
            Some(SOME_LAYOUT)
        );

        // The write half, exactly as `refresh_layout_and_cache` makes it when the probe
        // finds the foreground layout moved.
        publish_active_layout(other);
        publish_cache(crate::convert::fallback_cache());

        // The verdict comes back, the recorder is restored — and it wakes up recording
        // under the layout the probe delivered, not under the stale one it parked with.
        restore_buffer();
        assert!(buffer::is_installed());
        assert_eq!(
            buffer::with(|recorder| recorder.active_layout()),
            Some(other),
            "the restored recorder must carry the layout published while it was parked"
        );
        assert_eq!(buffer::with(|recorder| recorder.has_cache()), Some(true));

        buffer::uninstall();
    }

    /// The counterpart rule of the test above: what reaches a parked recorder is the layout
    /// of FR-04 and the cache of FR-20 and nothing else — FR-70 is not weakened. The ring
    /// comes back exactly as [`park_buffer`] left it: empty and zeroed (SEC-02).
    #[test]
    fn a_publication_into_a_parked_recorder_stores_no_strokes() {
        buffer::install_recorder(Recorder::with_capacity(8));
        press();
        assert_eq!(buffer::len(), 1);

        // Parking wipes — that is FR-70's own rule, asserted here as the baseline.
        park_buffer();

        publish_active_layout(SOME_LAYOUT);
        publish_cache(crate::convert::fallback_cache());

        restore_buffer();
        assert_eq!(
            buffer::len(),
            0,
            "publications while parked must not resurrect or add strokes"
        );

        buffer::uninstall();
    }

    /// **Point 4 of task T-13-4: the buffer comes back from the gate of FR-70 believing what the
    /// machine believes, not what it parked with.**
    ///
    /// While the buffer sits in [`PARKED_BUFFER`] — the password field of FR-70, the excluded
    /// process of FR-84 — `buffer::with` answers `None` for every stroke, so a `CapsLock` pressed
    /// during it never reaches `Recorder::record` and the tracker comes back **inverted**. The
    /// ring is deliberately empty on the way back (nothing is recovered, and SEC-02 saw to the
    /// memory), but `Held::caps` is not a stroke: it is a belief about the machine, and the
    /// machine went on being typed at.
    ///
    /// The assertion is against `hook::caps_lock_on()` and not against `true`, for the reason the
    /// twin test in `tests\hook.rs` states: the machine's toggle belongs to whoever is running
    /// `cargo test`, and what this task promises is that the tracker ends up **equal to the
    /// reading the product makes** — which is the same claim on every machine.
    #[test]
    fn a_buffer_restored_from_the_gate_is_seeded_with_the_machines_caps_lock() {
        buffer::install_recorder(Recorder::with_capacity(8));

        let machine = crate::hook::caps_lock_on();

        // The press that never reached `record`. Stated rather than pressed: a press that
        // reaches `record` is exactly the case this defect is not about.
        buffer::with(|recorder| recorder.set_caps_lock(!machine));

        park_buffer();
        assert!(!buffer::is_installed(), "the interval of FR-70");

        restore_buffer();
        assert!(buffer::is_installed());

        assert_eq!(
            buffer::with(|recorder| recorder.held().caps()),
            Some(machine),
            "point 4: the buffer coming back must re-read the machine's CapsLock"
        );
        assert_eq!(
            buffer::len(),
            0,
            "FR-70 is not weakened: the ring comes back empty, seed or no seed"
        );

        buffer::uninstall();
    }

    /// **FR-21 delivery**, task T-03-3: when the layout probe behind `WM_APP_LAYOUT` is worth a
    /// rebuild and when it is not.
    ///
    /// The sweep of FR-20 is thousands of `ToUnicodeEx` calls and `EVENT_SYSTEM_FOREGROUND`
    /// fires on every `Alt+Tab`, so "the foreground changed" and "the layout changed" have to be
    /// different questions. The decision is driven here rather than by calling
    /// `refresh_layout_and_cache`, which would run a real cache build in a test binary whose
    /// other tests assert on the process-wide failure counter of FR-25.
    #[test]
    fn a_rebuild_is_asked_for_only_when_the_layout_really_moved() {
        let other = LayoutId::from_raw(0x0419_0419);

        // The ordinary case: the user switched window and the new one runs the same layout.
        assert!(!layout_refresh_needed(SOME_LAYOUT, Some(SOME_LAYOUT)));

        // The case FR-21 is about.
        assert!(layout_refresh_needed(other, Some(SOME_LAYOUT)));

        // No foreground window at all — while the desktop switches, and on the secure desktop.
        // Publishing that would tell the buffer to record under a layout no cache contains.
        assert!(!layout_refresh_needed(
            LayoutId::default(),
            Some(SOME_LAYOUT)
        ));

        // A thread that owns no buffer owns no cache either, so there is nothing to rebuild.
        assert!(!layout_refresh_needed(other, None));
        assert!(!layout_refresh_needed(LayoutId::default(), None));
    }

    /// **Task T-19-1, finding 3 of the audit of 2026-08-31.** A thread of section 6.1 whose
    /// body returns `Err` asks the process to come down — whichever of the three it is, and
    /// while the other two are still pumping.
    ///
    /// The situation is exactly the production one: the three threads are started through the
    /// very wiring `spawn_threads` uses, and the main thread is parked inside `join_all` on the
    /// input handle, which is where a Release build spends the whole session. The bodies are
    /// substitutes — the real [`thread_body`] wants a window, a COM apartment and a message
    /// queue — and the two that are not failing park, because "still pumping" is the state that
    /// made this a defect.
    ///
    /// ⭐ **Task Т-22-7, finding м7 of the audit of 2026-09-01 — the shutdown handshake walks on
    /// `SeqCst`.**
    ///
    /// # Why this test reads the source
    ///
    /// The property is a *forbidden interleaving*, and no test can show one. The outcome
    /// Release/Acquire permits — `request_shutdown` reading `NO_WINDOW` while `serve_window`
    /// reads `false`, and the thread pumping for ever — needs two stores to sit in two store
    /// buffers at once; it is architecture-, compiler- and scheduler-dependent, it does not
    /// reproduce on x86 at all in practice, and a test that waited for it would be green because
    /// it lost. What can be checked honestly is the memory ordering the four operations are
    /// written with, which is also exactly what a reviewer checks — the same argument
    /// `tests\hook.rs` gives for reading the source of the callback.
    ///
    /// # What is asserted
    ///
    /// The four operations of the pair, and their absence in the ordering the litmus goes
    /// through. The store of `clear_shutdown_request` is not among them and is deliberately left
    /// alone: it is `#[cfg(test)]`, it stores `false`, and it takes part in no handshake.
    ///
    /// ⚠ The line endings are normalised before anything is matched, for the reason
    /// `tests\hook.rs` states at `the_capslock_seed_is_outside_the_callback`: `.gitattributes`
    /// makes the canonical checkout CRLF, and a test about memory ordering must not have a
    /// verdict that depends on how a checkout stored its newlines.
    #[test]
    fn the_shutdown_handshake_walks_on_seqcst() {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("app.rs"),
        )
        .expect("src/app.rs must be readable")
        .replace("\r\n", "\n");

        // ⚠ The product half of the file only. The needles below are written out in this very
        // function, so a sweep over the whole text would find every one of them in its own
        // argument list and answer about itself — the first draft of this test did exactly that
        // and failed on the repaired tree. `clear_shutdown_request` stays on the product side and
        // is meant to: it is `#[cfg(test)]`, it stores `false`, and no needle names it.
        let product = source
            .split_once("\n#[cfg(test)]\nmod tests {")
            .expect("this file ends in its own test module")
            .0;

        for operation in [
            "SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);",
            "let raw = target.load(Ordering::SeqCst);",
            "WAKE_TARGETS[role.index()].store(handle.0 as usize, Ordering::SeqCst);",
            "SHUTDOWN_REQUESTED.load(Ordering::SeqCst)",
        ] {
            assert!(
                product.contains(operation),
                "Т-22-7: the shutdown handshake needs one total order over all four of its \
                 operations, and {operation:?} is not in src\\app.rs"
            );
        }

        for permitted in [
            "SHUTDOWN_REQUESTED.store(true, Ordering::Release)",
            "SHUTDOWN_REQUESTED.load(Ordering::Acquire)",
            "WAKE_TARGETS[role.index()].store(handle.0 as usize, Ordering::Release)",
            "let raw = target.load(Ordering::Acquire);",
        ] {
            assert!(
                !product.contains(permitted),
                "Т-22-7: {permitted:?} leaves the store-buffer outcome permitted, and the \
                 comments beside it promise that it is not"
            );
        }
    }

    /// All three roles are one test on purpose: [`SHUTDOWN_REQUESTED`] is a `static` of the
    /// process, and two tests asserting on it would be asserting on each other's timing. The
    /// input case is the positive control — it was green before the repair as well, because
    /// `join_all` reaches its own request as soon as the handle at index 0 returns.
    #[test]
    fn a_thread_that_ends_with_an_error_asks_the_process_to_come_down() {
        // Collected rather than asserted case by case, so that one run names every role and
        // the positive control is visible in the same output as the failure.
        let mut asked_by = Vec::new();

        for failing in [Role::Watcher, Role::Ui, Role::Input] {
            clear_shutdown_request();

            let gate = Arc::new((Mutex::new(false), Condvar::new()));
            let body_gate = Arc::clone(&gate);

            let body = move |role: Role| -> WinResult<()> {
                if role == failing {
                    // A refused CreateWindowExW, CoInitializeEx or watchdog::watch: an
                    // ordinary `Err`, not a panic. The panic path was covered by FR-98 all
                    // along; this one was not.
                    return Err(WinError::from_thread());
                }

                let (lock, awake) = &*body_gate;
                let mut open = lock.lock().expect("gate");
                while !*open {
                    open = awake.wait(open).expect("gate");
                }
                Ok(())
            };

            let threads = spawn_threads_with(body).expect("three threads start");

            // `join_all` on its own thread because it blocks, exactly as the main thread of a
            // Release build blocks: on the input handle, first in the vector.
            let joiner = thread::spawn(move || join_all(threads));

            let asked = shutdown_requested_within(Duration::from_secs(2));

            // Whatever the verdict, let the parked bodies out and collect every thread, so
            // that a failure of this test leaves nothing running behind it.
            {
                let (lock, awake) = &*gate;
                *lock.lock().expect("gate") = true;
                awake.notify_all();
            }
            let _ = joiner.join();

            asked_by.push((failing, asked));
        }

        // Left as it was found: the rest of this binary must not inherit a requested shutdown.
        clear_shutdown_request();

        let silent: Vec<Role> = asked_by
            .iter()
            .filter(|(_, asked)| !asked)
            .map(|(role, _)| *role)
            .collect();

        assert!(
            silent.is_empty(),
            "threads that ended with Err without asking the process to come down: {silent:?} \
             (all three roles: {asked_by:?})"
        );
    }

    /// Whether shutdown is requested within `within`. Polled, because the request is made by
    /// another thread and there is nothing to park on.
    fn shutdown_requested_within(within: Duration) -> bool {
        let deadline = Instant::now() + within;

        while Instant::now() < deadline {
            if shutdown_requested() {
                return true;
            }
            thread::sleep(Duration::from_millis(5));
        }

        shutdown_requested()
    }

    /// ⭐ **Task T-37-1, finding Н38 — the probe of FR-70 is asked for once every window exists,
    /// and not only when a focus change asks for one.**
    ///
    /// # What was lost
    ///
    /// The one probe that is not tied to a focus change is the one `guard::publish_exclusions`
    /// asks for, and at start-up it is asked for by the UI thread while the three threads race to
    /// publish their windows. When the watcher window is not published yet the post is refused,
    /// the ticket is taken back (task Т-22-2, correctly), and nothing asks again: the field the
    /// person starts in is never probed, and the verdict stays the default of FR-73 — «буферизовать»
    /// — until the first focus change. A password typed there goes into the ring (SEC-06).
    ///
    /// # What is staged, and why it is the start-up of the finding
    ///
    /// The three windows are created by the product's own `Window::create`, under the product's
    /// own class, in the order the finding is about — the watcher window **last** — and nothing
    /// else asks for a probe while they come up: exactly the state a start-up is in after the UI
    /// thread's request was refused. The assertion is the one thing the watcher thread needs in
    /// order to run the probe on its first turn of the loop: a `WM_APP_PROBE` in its queue.
    ///
    /// ⚠ **The message is peeked and never dispatched**, so no probe runs here: running one is
    /// entering the three levels of FR-72 against whatever window the machine has in front, which
    /// is a state of the room and not of the code (the header of `tests\guard.rs` says so). That
    /// the probe itself then runs without a single focus change is the live acceptance of this
    /// task — the dump's `guard.probes` above zero after a restart with the focus left alone.
    ///
    /// # The instrument's control
    ///
    /// A request posted while the watcher window exists **is** seen by the peek, and a second peek
    /// finds the queue drained — otherwise a red below could be an instrument that sees nothing.
    #[test]
    fn the_first_probe_is_asked_for_once_every_window_exists() {
        use windows::Win32::UI::WindowsAndMessaging::{PM_REMOVE, PeekMessageW};

        fn probe_is_queued(window: HWND) -> bool {
            let mut message = MSG::default();

            // SAFETY: `message` is a live local of exactly the type the call fills, and `window`
            // is a window this very thread created and has not destroyed — `PeekMessageW` filters
            // on the calling thread's queue and dereferences nothing else of ours. `PM_REMOVE`
            // takes the message out without dispatching it, so no window procedure runs.
            unsafe {
                PeekMessageW(
                    &raw mut message,
                    Some(window),
                    crate::guard::WM_APP_PROBE,
                    crate::guard::WM_APP_PROBE,
                    PM_REMOVE,
                )
            }
            .as_bool()
        }

        let instance = module_instance().expect("the module handle");
        let _class = WindowClass::register(instance).expect("the window class of the process");

        {
            let watcher = Window::create(Role::Watcher, instance).expect("a watcher window");

            assert!(
                post_to_watcher_thread(crate::guard::WM_APP_PROBE),
                "control: the watcher window is published, so the post is accepted"
            );
            assert!(
                probe_is_queued(watcher.handle),
                "control: the instrument sees a request that was posted"
            );
            assert!(
                !probe_is_queued(watcher.handle),
                "control: and a second look finds the queue drained"
            );
        }

        let input = Window::create(Role::Input, instance).expect("an input window");
        let ui = Window::create(Role::Ui, instance).expect("a UI window");
        let watcher = Window::create(Role::Watcher, instance).expect("a watcher window");

        let asked = probe_is_queued(watcher.handle);
        let asked_again = probe_is_queued(watcher.handle);

        // Unpublished before the verdict, so that a failure here leaves no window of this class
        // standing in the register for another test of this binary to post to.
        drop(watcher);
        drop(ui);
        drop(input);

        assert!(
            asked,
            "Н38: all three windows are published and no probe was asked for — the field the \
             program starts in stays at the default of FR-73 until the first focus change"
        );
        assert!(!asked_again, "one start-up probe, and not one per window");
    }

    /// ⭐ **Task T-37-2, finding Н36 — the watcher thread releases the client of level 3 inside
    /// its apartment on every way out, the panicking one included.**
    ///
    /// # What was wrong
    ///
    /// The apartment was a guard and the release was a line after `serve_window`. A panic on the
    /// watcher thread unwinds past that line: the apartment's `Drop` runs `CoUninitialize`, and the
    /// client left in its thread-local is released later by the thread-local destructor, into an
    /// apartment that no longer exists — undefined behaviour, reachable wherever a panic unwinds
    /// (a debug build and every test binary; the Release profile aborts instead).
    ///
    /// # How it is seen
    ///
    /// The two steps write themselves into a journal of the thread as they run — `ComApartment::drop`
    /// writes «apartment», `guard::release_automation` writes «client» — and the body of the
    /// watcher thread is driven twice on threads of its own: once returning normally, which is the
    /// **positive control** (the journal sees the release when it happens), and once with the
    /// thing it serves panicking. The order is the verdict: «client» before «apartment».
    #[test]
    fn the_watcher_thread_releases_the_client_before_it_leaves_the_apartment_even_in_a_panic() {
        fn journal_of(body: fn() -> WinResult<()>) -> (bool, Vec<&'static str>) {
            thread::spawn(move || {
                let unwound = std::panic::catch_unwind(|| {
                    let _ = watcher_body(body);
                })
                .is_err();

                let journal = TEARDOWN.with(|journal| journal.borrow().clone());

                (unwound, journal)
            })
            .join()
            .expect("the thread hands its journal back")
        }

        let (unwound, normal) = journal_of(|| Ok(()));

        assert!(!unwound, "control: the normal body does not panic");
        assert_eq!(
            normal,
            ["client", "apartment"],
            "control: on the normal way out the journal sees the release, and before the apartment"
        );

        let (unwound, panicking) = journal_of(|| panic!("staged: a panic on the watcher thread"));

        assert!(
            unwound,
            "control: the staged panic unwound through the body"
        );
        assert_eq!(
            panicking,
            ["client", "apartment"],
            "Н36: a panic on the watcher thread must release the client before the apartment is left"
        );
    }
}
