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
//! Boundary: FR-83 (`WM_QUERYENDSESSION` / `WM_ENDSESSION`, unhooking, removing the tray
//! icon, wiping the buffer, saving the configuration) is handled in [`crate::tray`] by task
//! T-01-4, which attached itself to [`request_shutdown`] and [`shutdown_requested`]; the
//! "release the mutex" action of FR-83 is the one part of it that lives here, in
//! [`SingleInstance`]'s `Drop`, and the "снятие хуков" action is the other, in
//! [`window_proc`], because `src\tray.rs` was outside the file scope of task T-03-1.
//! FR-01's "one hook on the input thread" is installed here, in [`serve_window`], and
//! implemented in [`crate::hook`].
//! Implemented by backlog tasks: T-01-2 (done), T-01-4 (done), T-03-1 (done).
//!
//! # Thread model — section 6.1
//!
//! Three threads, each with a single responsibility and its own message loop:
//!
//! * **input** — owns a message-only window; task T-03-1 adds `WH_KEYBOARD_LL` to it,
//!   T-02-1's layout cache is rebuilt on its messages, and `SendInput` runs here;
//! * **UI** — owns a hidden top-level window; task T-01-4 attached the tray icon and menu
//!   to it, and they run on this thread and on no other;
//! * **watcher** — owns a message-only window and a COM STA apartment; tasks T-06-1 and
//!   T-06-2 add `SetWinEventHook` and the UI Automation password-field probe.
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
//! Nothing here is a mutex, a lock or a channel. The only cross-thread state is an
//! `AtomicBool` and three published window handles, and the only cross-thread call is
//! `PostMessageW`, which does not block. This is deliberate groundwork: NFR-04 forbids
//! blocking primitives on the hook path and section 6.3 forbids mutexes there outright, so
//! the shutdown interface T-03-1 will call from inside hook-adjacent code must not have
//! any.
//!
//! # SEC-01, SEC-07
//!
//! No keystroke, key code or buffer content passes through this module, and none may be
//! added later: neither [`report_non_critical`] nor the panic hook is given anything but
//! an operation name and an OS error code.

use std::ffi::c_void;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE,
    MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MSG, MessageBoxW, PostMessageW, PostQuitMessage,
    RegisterClassExW, UnregisterClassW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE,
    WM_ENDSESSION, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{Error as WinError, PCWSTR, Result as WinResult, w};

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
    install_panic_hook();

    match SingleInstance::acquire() {
        Ok(Acquisition::Acquired(instance)) => run_as_first_instance(instance),
        Ok(Acquisition::AlreadyRunning) => {
            notify_already_running();
            ExitCode::from(EXIT_ALREADY_RUNNING)
        }
        Err(error) => {
            // FR-82 cannot be honoured if the mutex cannot be created at all, and starting
            // anyway would mean two copies hooking the keyboard. Refusing to start is the
            // only answer that keeps the requirement true.
            report_non_critical("CreateMutexW", &error);
            ExitCode::FAILURE
        }
    }
}

/// Asks every thread of the process to leave its message loop. Idempotent, callable from
/// any thread, and safe to call before the threads exist or after they are gone.
///
/// This is the interface task T-01-4 attaches to for FR-83, and the one the panic hook
/// (FR-98) uses. Decision R-20 point 3: the flag is what decides, the message only wakes.
pub fn request_shutdown() {
    // Release/Acquire pairing with the load in `shutdown_requested`: a thread woken by the
    // message below, or one that re-reads the flag after publishing its window, is
    // guaranteed to observe `true`. Without that ordering a thread could be woken, see a
    // stale `false`, go back to waiting, and keep the process alive for ever.
    SHUTDOWN_REQUESTED.store(true, Ordering::Release);

    for target in &WAKE_TARGETS {
        let raw = target.load(Ordering::Acquire);
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
pub fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::Acquire)
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
/// Compiled only where FR-99 is: under `panic = "abort"`, which is what the Release profile
/// of section 3.2 sets, a panic in the callback ends the process instead of being absorbed,
/// there is no fail-safe transition to signal, and this accessor would be a function nobody
/// calls. The `cfg` is the same one `hook::guarded_decision` is selected on, and keeping the
/// two identical is what leaves the Release build free of unreachable code.
#[cfg(panic = "unwind")]
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
    /// Another instance holds the name already.
    AlreadyRunning,
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
        let instance = Self { handle };

        if last_error == ERROR_ALREADY_EXISTS {
            drop(instance);
            return Ok(Acquisition::AlreadyRunning);
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

/// Starts the three threads of section 6.1.
///
/// On failure the threads already started are brought down before returning, rather than
/// detached: a detached thread would keep pumping and the process would never exit.
fn spawn_threads() -> std::io::Result<Vec<JoinHandle<WinResult<()>>>> {
    let mut threads = Vec::with_capacity(THREAD_COUNT);

    for role in [Role::Input, Role::Ui, Role::Watcher] {
        let spawned = thread::Builder::new()
            .name(role.thread_name().to_owned())
            .spawn(move || thread_body(role));

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

        // Whichever thread came down first, the process is on its way out: ask the rest to
        // stop as well. Without this a thread that ended on its own — a failed window
        // creation, a panic — would leave the others pumping and the process alive.
        request_shutdown();
    }

    clean
}

/// The body of every one of the three threads.
fn thread_body(role: Role) -> WinResult<()> {
    match role {
        // The watcher thread is the only COM apartment in the process, and it is an STA:
        // section 6.1 puts UI Automation here, and UI Automation requires a single-threaded
        // apartment. Task T-06-1 may move this initialisation next to the code that needs
        // it; until then it belongs to whoever creates the thread.
        Role::Watcher => {
            let _apartment = ComApartment::enter_sta()?;
            serve_window(role)
        }
        Role::Input | Role::Ui => serve_window(role),
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

            // Section 6.3, "Конфигурация публикуется потоком UI": the tray has just read
            // `config.toml`, and this is the moment the input thread's hook learns what is
            // in it. It cannot read the file itself — NFR-08 gives the hook fifty
            // milliseconds from start-up and a file read is exactly the kind of thing task
            // T-03-1 was told to keep off that path.
            publish_configuration_to_hook();

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
        Role::Input => Some(crate::hook::install(_window.handle, instance)?),
        Role::Ui | Role::Watcher => None,
    };

    // Re-read the flag after the window has been published, and before the first
    // `GetMessageW`. This closes the only race in the shutdown design: a `request_shutdown`
    // that ran before the window existed found `NO_WINDOW`, posted nothing, and would
    // otherwise leave this thread pumping for ever. The flag is stored before the wake-ups
    // are posted, so either that store is visible here or the post reached the queue.
    if shutdown_requested() {
        return Ok(());
    }

    pump()
}

/// Hands the hook of task T-03-1 the two settings it reacts to — FR-02 and FR-95.
///
/// Runs on the UI thread, right after the tray has been attached, because the tray is where
/// the configuration of section 7 lives: it read the file, it owns `general.enabled`, and it
/// is the thread allowed to touch a file at all (section 6.1). The hook only ever reads the
/// two published atomics.
///
/// `with_tray` answers `None` on any thread that is not the UI thread, so a stray call from
/// elsewhere would publish nothing rather than publish something wrong.
fn publish_configuration_to_hook() {
    let published = crate::tray::with_tray(|tray| {
        (
            tray.enabled(),
            crate::hook::vk_from_name(&tray.config().hotkey.key),
        )
    });

    let Some((active, hotkey)) = published else {
        return;
    };

    crate::hook::set_active(active);

    // A name this program does not know leaves the default of section 7 in place — `Pause`.
    // A resident utility whose hotkey silently ceased to exist because of a typo in a file
    // would be a worse answer than one that keeps answering to the documented default. The
    // settings dialog is where a bad name is reported to the user; that is FR-92, task
    // T-08-1.
    if let Some(vk) = hotkey {
        crate::hook::set_hotkey_vk(vk);
    }
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

        // Published only after the handle is known good. Release pairs with the Acquire
        // load in `request_shutdown`.
        WAKE_TARGETS[role.index()].store(handle.0 as usize, Ordering::Release);

        Ok(Self { handle, role })
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
        // SAFETY: balances exactly one successful CoInitializeEx on this same thread — the
        // value is created and dropped inside `thread_body` and never leaves it — and it
        // runs after the message loop has returned, so no COM object of this apartment is
        // still in use. The call returns nothing, so NFR-13 has no result to check.
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
/// The in-memory ring journal is module `diag`, task **T-06-4**; the body stays empty until
/// it exists. SEC-01 and SEC-07: only an operation name and an OS error code ever reach
/// this function, and nothing that could carry a keystroke, a key code or buffer contents
/// may ever be added to its arguments.
///
/// `pub(crate)` rather than private since task T-01-4: `tray` has the same kind of failures
/// to record — a shell that is not up yet, a handle that will not free — and a second copy
/// of this function there would defeat the whole point of having one place to wire the
/// journal into.
pub(crate) fn report_non_critical(operation: &str, error: &WinError) {
    let _ = (operation, error);
}
