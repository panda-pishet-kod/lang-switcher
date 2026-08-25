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

            let served = serve_window(role);

            // Task **T-06-1**. The UI Automation client of level 3 of FR-72 lives in a
            // thread-local of this thread, and a COM interface must be released inside the
            // apartment it was created in. A thread-local destructor runs at thread exit, which
            // is **after** the `CoUninitialize` in `ComApartment::drop` — that is, after the
            // apartment it belongs to has gone. This line is the release, at the one instant
            // that is both after the last probe and before the apartment is left.
            crate::guard::release_automation();

            served
        }
        // Second of the three edits of task T-06-4. Section 6.1 gives file input-output to
        // the UI thread and forbids it to the input thread, so the one place the journal may
        // reach a file is here, on this thread, once, after `serve_window` has returned.
        //
        // After and not inside: the tray attachment, the window and the watchdog
        // subscriptions are undone by `Drop` implementations *inside* that call, and those
        // are precisely the paths that report into the journal last. Dumping before they ran
        // would leave their failures in a ring nobody reads.
        //
        // Off unless `[diagnostics] log_enabled` says otherwise — section 7 — in which case
        // this creates nothing at all.
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

            // **FR-94, task T-08-2 — the one publication that happens exactly once.**
            //
            // The tray has just read `config.toml`; this is where `general.language` reaches
            // the module that turns string identifiers into text. It is deliberately *not* in
            // `publish_configuration` below, which runs again on every «Применить»: the note
            // beside the language combo box says the choice takes effect after a restart, and
            // it is only true if nothing re-publishes the locale under the user's hands. A menu
            // that changed language halfway through a session while the window that was open
            // did not would be a worse answer than the honest one.
            crate::tray::with_tray(|tray| {
                crate::settings::set_ui_language(tray.config().general.language);
            });

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
            let installed = crate::hook::install(_window.handle, instance)?;

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
    publish_cache(cache_or_fallback(LayoutCache::build()));

    // Instrumentation, feature `testing`. It is what lets the acceptance run tell a rebuild
    // that happened from one that was merely wired up: FR-21 has no visible effect of its own,
    // and FR-11 — that the rebuild left the buffer alone — can only be asserted against a
    // rebuild that is known to have run.
    #[cfg(feature = "testing")]
    crate::control::note_cache_built();
}

/// The cache to use, given what [`LayoutCache::build`] answered — FR-25.
///
/// A failure to build is **not** a reason to end the program, and not a reason to run without a
/// cache either: FR-25 has module `convert` carry a hardwired RU/EN table for exactly this, and
/// point 4 of it is why `LayoutCache` refuses to be empty — "the cache did not build" has to
/// stay distinguishable from "these keys carry no characters".
fn cache_or_fallback(built: Result<LayoutCache, LayoutError>) -> LayoutCache {
    match built {
        Ok(cache) => cache,

        // SEC-01, SEC-07: the reason is dropped unread rather than formatted. `LayoutError`
        // carries no keystroke, and the rule of this program is still that nothing on this path
        // becomes a string. What is kept is the count.
        Err(_reason) => {
            LAYOUT_CACHE_FAILURES.fetch_add(1, Ordering::Relaxed);
            crate::convert::fallback_cache()
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

/// Re-reads the layout of the foreground window and rebuilds the cache if it really changed —
/// **the delivery of FR-21**, task T-03-3.
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
/// # Why the rebuild is conditional
///
/// FR-20 is a sweep of virtual keys `0x08..=0xFF` against eight modifier combinations for every
/// layout in the session — thousands of `ToUnicodeEx` calls, five milliseconds on the machine
/// this was measured on. `EVENT_SYSTEM_FOREGROUND` fires on every `Alt+Tab`, and the window the
/// user switched to is nearly always running the same layout as the one they left, so rebuilding
/// unconditionally would pay the whole sweep for nothing many times a minute. The comparison is
/// against the layout the buffer is recording under, which is the value the rebuild would change
/// anyway.
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

    if !layout_refresh_needed(
        observed,
        with_recorder_wherever_it_is(|recorder| recorder.active_layout()),
    ) {
        return;
    }

    publish_active_layout(observed);
    rebuild_layout_cache();
}

/// The decision of [`refresh_layout_and_cache`], as a function of its two inputs.
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
/// the same outcome as a key the layouts have nothing on, and FR-23 carries such a stroke
/// through conversion unchanged.
fn foreground_layout() -> LayoutId {
    crate::switch::current()
}

/// Posts `message` to the window of `role`, if that thread has one right now.
///
/// The same shape as the wake-up loop of [`request_shutdown`] and for the same reason:
/// `PostMessageW` queues and returns, so one thread can nudge another without either of them
/// blocking, which is what section 6.3 and NFR-04 require of everything near the hook path.
fn post_to(role: Role, message: u32) {
    let raw = WAKE_TARGETS[role.index()].load(Ordering::Acquire);

    if raw == NO_WINDOW {
        // That thread has not created its window yet, or has already destroyed it. The first
        // case is covered by the target re-reading what was published once it has one.
        return;
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

    if let Err(error) = posted {
        // NFR-13: examined, not discarded. A failure here means the target destroyed its window
        // between the load above and this call, that is, it is already leaving.
        report_non_critical("PostMessageW", &error);
    }
}

/// Posts `message` to the input thread's window, if that thread has one right now.
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
pub(crate) fn post_to_input_thread(message: u32) {
    post_to(Role::Input, message);
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
/// The answer matters, which is why this returns something and [`post_to_input_thread`] does not:
/// with no watcher window there is no method 3, and module `switch` has to take its pending
/// target back rather than leave it for a later message to act on.
pub(crate) fn post_to_watcher_thread(message: u32) -> bool {
    if WAKE_TARGETS[Role::Watcher.index()].load(Ordering::Acquire) == NO_WINDOW {
        return false;
    }

    post_to(Role::Watcher, message);
    true
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
/// The answer matters, which is why this returns something and [`post_to_input_thread`] does not:
/// with no UI window there is no selection path, and module `selection` has to take its pending
/// plan back and let the press go down the typing-buffer path of FR-60 instead.
pub(crate) fn post_to_ui_thread(message: u32) -> bool {
    if WAKE_TARGETS[Role::Ui.index()].load(Ordering::Acquire) == NO_WINDOW {
        return false;
    }

    post_to(Role::Ui, message);
    true
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

    publish_buffer_capacity(config.buffer.capacity);
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

/// Publishes `[buffer] capacity` to the input thread and nudges it into applying it — FR-07.
///
/// Two steps and in this order: the value goes into the atomic first, and only then is the
/// message posted, so a thread woken by the message cannot read a stale capacity.
///
/// The message is a wake-up and not a command — the same design as [`WM_APP_WAKE`], and for
/// the same SEC-05 reason. If it never arrives, because the input thread had not created its
/// window yet, the value is not lost: [`start_input_pipeline`] reads the atomic once more when
/// its sweep is over.
fn publish_buffer_capacity(capacity: usize) {
    BUFFER_CAPACITY.store(capacity, Ordering::Release);

    post_to(Role::Input, WM_APP_CONFIGURED);
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
            // `buffer::is_installed` is what says "this is the input thread": section 6.3 gives
            // the buffer to that thread and to no other. The test is not decoration — the UI
            // window is top-level, `WM_DEVICECHANGE` is broadcast to top-level windows, and
            // without it every device change in the machine would run a full sweep on the
            // thread section 6.1 exists to keep free.
            //
            // ⚠ **FR-11: neither call flushes the buffer.** See `publish_cache` and
            // `publish_active_layout`; there is no `reset` anywhere on this path.
            //
            // SEC-05: a process at the same integrity level can post either message. All that
            // buys it is a rebuild of our own cache out of the system's own layout list — an
            // idempotent operation over memory of ours, which is the same standing the wake-up
            // message has.
            if crate::layouts::needs_rebuild(message) && crate::buffer::is_installed() {
                // Task T-08-4: the count that lets a run *show* FR-21 being delivered rather
                // than assert it. It answers `false` for `WM_INPUTLANGCHANGE`, which is the
                // other message of the list and is not a device change; the result is dropped
                // because the rebuild below happens for both alike. SEC-07 — a count of events,
                // never a device name.
                let _ = crate::watchdog::note_device_change(message);

                publish_active_layout(foreground_layout());
                rebuild_layout_cache();
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
            crate::watchdog::apply_flush(message, lparam);

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

            if (hotkey || handed_back)
                && crate::buffer::is_installed()
                && !(hotkey && crate::selection::wants_selection_path())
            {
                let _ = crate::inject::on_hotkey();
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
            // every `Space` the user types already does; it is refused outright at the UI and
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
            if let Some(outcome) = crate::selection::handle_selection_message(hwnd, message)
                && outcome.falls_back()
            {
                post_to(Role::Input, crate::selection::WM_APP_BUFFER_PATH);
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
                if message == crate::watchdog::WM_APP_FLUSH {
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

    /// FR-25. A cache that will not build is answered with the hardwired table of module
    /// `convert`, the failure is counted, and the program carries on with a usable cache.
    ///
    /// Both directions are one test on purpose: [`LAYOUT_CACHE_FAILURES`] is a counter of the
    /// process, `cargo test` runs the tests of a binary in parallel, and two tests asserting on
    /// the same counter would be asserting on each other's timing.
    #[test]
    fn a_failed_build_falls_back_to_the_hardwired_table_of_fr25() {
        let built = crate::convert::fallback_cache();
        let before = layout_cache_failures();

        // A cache that built is used exactly as it is, and nothing is counted.
        assert_eq!(cache_or_fallback(Ok(built.clone())), built);
        assert_eq!(layout_cache_failures(), before);

        // Every way the build can fail ends on the table of FR-25 rather than on a panic, an
        // empty cache or a dead program.
        for reason in [
            LayoutError::Enumeration,
            LayoutError::NoUsableLayouts,
            LayoutError::Empty,
        ] {
            let fallen_back = cache_or_fallback(Err(reason));

            // The very cache FR-25 prescribes, and not an empty one: point 4 of FR-25 is that
            // "the cache did not build" stays distinguishable from "these keys carry no
            // characters".
            assert_eq!(fallen_back, built, "failure {reason:?}");
            assert!(!fallen_back.is_empty(), "failure {reason:?}");
        }

        assert_eq!(layout_cache_failures(), before + 3);
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
}
