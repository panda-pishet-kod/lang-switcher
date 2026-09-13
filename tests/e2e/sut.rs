//! The program under test: starting it, knowing it is ready, and — requirement 7 of §11.5 —
//! being certain its global hook is gone however this bench ends.
//!
//! # Requirement 7 is about two different accidents
//!
//! "При аварийном завершении в середине прогона гарантированно снимает хук" has to hold when
//! the bench panics *and* when the bench is killed from outside. Those need different
//! mechanisms, because the second one runs no code of ours at all:
//!
//! | Accident | What removes the hook | Why it works |
//! |---|---|---|
//! | Panic | the panic hook installed by [`install_safety_net`] | unwinding runs it before anything else, and it sends FR-96 and then terminates the job |
//! | Killed from outside (`Stop-Process`, taskkill, a debugger detaching badly) | **the job object** | `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: when this process dies the kernel closes its handles, and closing the last handle to the job kills everything in it. No code of the bench runs, and none needs to. |
//! | Bench wedged, neither of the above | the product's own FR-97 deadline | every launch sets `LANGSW_DEBUG_TIMEOUT_SEC`, so the program ends itself within a minute regardless |
//!
//! Three layers, and only the middle one survives `TerminateProcess` on the bench. That is why
//! the job object is here and why it is not optional.
//!
//! # Why raw FFI for the job object
//!
//! The typed job-object bindings of the `windows` crate live behind the feature
//! `Win32_System_JobObjects`, which is **not** in the closed dependency list of §3.2 of SPEC.
//! The task permits exactly one new section in `Cargo.toml` and says to stop and report if
//! anything else is needed. Nothing else *is* needed: three `extern "system"` declarations
//! inside `tests\e2e\` add no crate, no feature and no entry to `cargo tree` — kernel32 is
//! already linked by the product itself. The declarations are checked against the SDK headers
//! and every call examines its return, as NFR-13 requires.
//!
//! Task **T-10-10** adds three more by the same argument and for [`Installed`]:
//! `ShellExecuteExW` out of shell32 — which `Win32_UI_Shell` already links — plus
//! `GetProcessId` and `WaitForSingleObject` out of kernel32. Same reasoning, same treatment:
//! no crate, no feature, no line of `cargo tree`, a `// SAFETY:` on every call and every
//! return examined. `cargo tree` was re-measured after the change and is still 83 lines.

use std::os::windows::io::AsRawHandle;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, HANDLE};

use crate::{channel, input, wait};

/// Deadline handed to every launch of the product, in seconds — FR-97.
///
/// The "Окружение и безопасность" section of the task fixes the band at 30 to 60 seconds:
/// long enough for the longest scenario, short enough that a program forgotten by a wedged
/// bench is gone before anybody notices it. This is the third safety layer of the table above.
pub const DEBUG_TIMEOUT_SECS: u32 = 45;

/// Exit code of a second instance — `app::EXIT_ALREADY_RUNNING`, position 24.
pub const EXIT_ALREADY_RUNNING: i32 = 2;

/// Exit code left by the emergency combination — `hook::EXIT_EMERGENCY`.
pub const EXIT_EMERGENCY: i32 = 3;

// ---------------------------------------------------------------------------------------
// Job object — the layer that survives the bench being killed
// ---------------------------------------------------------------------------------------

/// `JobObjectExtendedLimitInformation`.
const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: i32 = 9;

/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct JobObjectBasicLimitInformation {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: u32,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: u32,
    affinity: usize,
    priority_class: u32,
    scheduling_class: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct IoCounters {
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct JobObjectExtendedLimitInformation {
    basic_limit_information: JobObjectBasicLimitInformation,
    io_info: IoCounters,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateJobObjectW(attributes: *const core::ffi::c_void, name: *const u16) -> isize;
    fn SetInformationJobObject(
        job: isize,
        class: i32,
        information: *const core::ffi::c_void,
        length: u32,
    ) -> i32;
    fn AssignProcessToJobObject(job: isize, process: isize) -> i32;
    fn TerminateJobObject(job: isize, exit_code: u32) -> i32;
}

/// The one job every launched product joins.
///
/// A raw handle in a global rather than an owned value, because the panic hook has to reach it
/// and a panic hook cannot borrow. Zero means "not created yet".
static JOB: AtomicIsize = AtomicIsize::new(0);

/// Creates the job and arms the kill-on-close limit. Called once, from `main`.
pub fn install_safety_net() -> Result<(), String> {
    // SAFETY: a null attributes pointer asks for default security, and a null name for an
    // unnamed job — both documented. NFR-13: a zero return is a failure and is examined here
    // rather than stored.
    let job = unsafe { CreateJobObjectW(core::ptr::null(), core::ptr::null()) };
    if job == 0 {
        return Err(format!(
            "CreateJobObjectW failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut limits = JobObjectExtendedLimitInformation::default();
    limits.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

    // SAFETY: `job` is the handle just created and checked. `limits` is a live, correctly
    // shaped `#[repr(C)]` value and the length passed is its own size, so the kernel reads
    // exactly what exists. NFR-13: a zero return is a failure and is examined.
    let ok = unsafe {
        SetInformationJobObject(
            job,
            JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
            core::ptr::from_ref(&limits).cast(),
            core::mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
        )
    };
    if ok == 0 {
        let error = std::io::Error::last_os_error();
        // SAFETY: `job` is a live handle this function owns and has not closed.
        let _ = unsafe { CloseHandle(HANDLE(job as *mut core::ffi::c_void)) };
        return Err(format!("SetInformationJobObject failed: {error}"));
    }

    JOB.store(job, Ordering::SeqCst);

    // The panic half of requirement 7. `take_hook` keeps the default reporting — a panic must
    // still say what and where — and this wraps it rather than replacing it.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        eprintln!("\n=== СТЕНД УПАЛ С ПАНИКОЙ — снимаю хук тестируемой программы (§11.5 п. 7) ===");
        emergency_teardown();
    }));

    Ok(())
}

/// FR-96 first, then the job. Called from the panic hook and from the ordinary exit path.
///
/// The order matters: FR-96 is the product's own documented way out and leaves it a chance to
/// end cleanly; terminating the job is the guarantee that it ends at all.
pub fn emergency_teardown() {
    match input::emergency_combination() {
        Ok(()) => eprintln!("  FR-96 (Ctrl+Alt+Shift+F12) отправлена"),
        Err(error) => eprintln!("  FR-96 отправить не удалось: {error}"),
    }

    let job = JOB.swap(0, Ordering::SeqCst);
    if job == 0 {
        return;
    }

    // SAFETY: `job` was created by `install_safety_net` and taken out of the global exactly
    // once by the swap above, so this runs at most once. NFR-13: both returns are examined.
    unsafe {
        if TerminateJobObject(job, EXIT_EMERGENCY as u32) == 0 {
            eprintln!(
                "  TerminateJobObject не сработала: {}",
                std::io::Error::last_os_error()
            );
        }
        if let Err(error) = CloseHandle(HANDLE(job as *mut core::ffi::c_void)) {
            eprintln!("  CloseHandle(job) не сработала: {error}");
        }
    }
}

// ---------------------------------------------------------------------------------------
// Launching the product
// ---------------------------------------------------------------------------------------

/// A running copy of the program under test.
pub struct Sut {
    child: Child,
    pub pid: u32,
}

/// Where the product's executable is.
///
/// Derived from the bench's own path rather than from `CARGO_TARGET_DIR`: both binaries are
/// written to the same directory by the same `cargo build`, so the sibling is the copy that
/// belongs to this build and cannot be a stale one from elsewhere.
pub fn executable() -> Result<std::path::PathBuf, String> {
    let mut path = std::env::current_exe()
        .map_err(|error| format!("не удалось определить путь стенда: {error}"))?;
    path.pop();
    path.push(lang_switcher::EXE_NAME);

    if !path.exists() {
        return Err(format!("{} не найден рядом со стендом", path.display()));
    }

    Ok(path)
}

impl Sut {
    /// Starts the product and joins it to the job before doing anything else with it.
    pub fn launch() -> Result<Self, String> {
        Self::launch_with(DEBUG_TIMEOUT_SECS)
    }

    /// [`Sut::launch`] with a deadline of the caller's choosing — **task T-10-9**.
    ///
    /// Every position of the matrix wants [`DEBUG_TIMEOUT_SECS`] and gets it through
    /// [`Sut::launch`]; nothing about the positions changes. What needs another number is the
    /// experiment of that task: its scenario is «работает → `Win+E` → снова набрать», and a
    /// human-tempo round of typing plus the shell opening a window plus ten repetitions does
    /// not fit into forty-five seconds. The deadline is still FR-97's and is still handed over
    /// in `LANGSW_DEBUG_TIMEOUT_SEC` — the same variable, no new one (Р-53).
    ///
    /// ⚠ It is a **net and not a stopping mechanism**: the caller ends its own product by pid,
    /// and this only bounds how long a forgotten one can live.
    pub fn launch_with(deadline_secs: u32) -> Result<Self, String> {
        let exe = executable()?;

        let child = Command::new(&exe)
            .env("LANGSW_DEBUG_TIMEOUT_SEC", deadline_secs.to_string())
            .spawn()
            .map_err(|error| format!("не удалось запустить {}: {error}", exe.display()))?;

        let pid = child.id();

        // ⛔ Requirement A: the product is a process the bench started, and it enters the
        // registry here, at the moment of launch. Without this the FR-96 shutdown below would
        // be refused by the very gate that protects everybody else's processes.
        crate::own::register_spawned(pid);

        let job = JOB.load(Ordering::SeqCst);
        if job != 0 {
            // SAFETY: `job` is the live job handle from `install_safety_net`, and
            // `as_raw_handle` returns the process handle `Child` owns and keeps alive for as
            // long as the `Child` exists — which is longer than this call. NFR-13: examined.
            //
            // ⚠ There is a window between `spawn` and this line in which the product is not
            // yet in the job. It is microseconds wide and the product spends its first
            // milliseconds reading a configuration file; the third layer, the FR-97 deadline
            // already in its environment, covers it. Closing the window entirely would need
            // `CREATE_SUSPENDED`, which means abandoning `std::process::Command` for a hand-
            // rolled `CreateProcessW` — a large amount of unsafe code for a gap that another
            // mechanism already covers.
            let ok = unsafe { AssignProcessToJobObject(job, child.as_raw_handle() as isize) };
            if ok == 0 {
                eprintln!(
                    "warning: AssignProcessToJobObject не сработала: {}",
                    std::io::Error::last_os_error()
                );
            }
        }

        // ⭐ **Finding Н42, task T-41-9.** From here on every reading of the SEC-04a channel is
        // checked against this identifier: a server on that name raised by somebody else —
        // before this product existed, so its own `CreateNamedPipeW` failed and it ran on — would
        // otherwise answer the bench and be believed. The check is inside `channel::read`, so no
        // call site can forget it.
        channel::expect_server(pid);

        Ok(Self { child, pid })
    }

    /// Waits until the product reports its hook installed, through the SEC-04a channel.
    pub fn await_ready(&self, timeout: Duration) -> Option<channel::Snapshot> {
        channel::await_hook(timeout)
    }

    /// Whether the process is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Ends the product through FR-96 and confirms it went.
    ///
    /// Returns the exit code observed. `EXIT_EMERGENCY` is the expected one and is what proves
    /// the emergency path ran rather than the deadline.
    pub fn stop(&mut self) -> Result<i32, String> {
        if !self.is_alive() {
            return self.exit_code();
        }

        input::emergency_combination().map_err(|error| format!("FR-96: {error}"))?;

        let gone = wait::until_true(Duration::from_secs(10), || {
            matches!(self.child.try_wait(), Ok(Some(_)))
        });

        if !gone {
            // The product ignored its own emergency exit. That is a finding, and the machine
            // still has to get its keyboard back.
            //
            // ⛔ Requirement D holds by construction: `Child::kill` acts on the handle this
            // bench's own `Command::spawn` returned for `LangSwitcher.exe`, never on a process
            // id derived from a window.
            let _ = self.child.kill();
            let _ = self.child.wait();
            return Err("FR-96 не завершила программу за 10 с; процесс снят принудительно".into());
        }

        self.exit_code()
    }

    fn exit_code(&mut self) -> Result<i32, String> {
        match self.child.try_wait() {
            Ok(Some(status)) => Ok(status.code().unwrap_or(-1)),
            Ok(None) => Err("процесс ещё жив".into()),
            Err(error) => Err(format!("try_wait: {error}")),
        }
    }
}

impl Drop for Sut {
    fn drop(&mut self) {
        if self.is_alive() {
            let _ = self.stop();
        }
    }
}

/// Whether any copy of the product is running right now — used before and after the run.
pub fn any_running() -> Vec<u32> {
    // Reading the process list through the shell rather than `CreateToolhelp32Snapshot`: the
    // feature `Win32_System_Diagnostics_ToolHelp` is outside the closed list of §3.2, and this
    // is a diagnostic on the way out, not part of a scenario's verdict.
    let stem = lang_switcher::EXE_NAME.trim_end_matches(".exe");
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!("(Get-Process -Name {stem} -ErrorAction SilentlyContinue).Id -join ','"),
        ])
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .split(',')
            .filter_map(|id| id.trim().parse().ok())
            .collect(),
        Err(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------------------
// The installed, signed, uiAccess copy — task T-10-10
// ---------------------------------------------------------------------------------------

/// `SEE_MASK_NOCLOSEPROCESS` — ask `ShellExecuteEx` to hand back a process handle.
const SEE_MASK_NOCLOSEPROCESS: u32 = 0x0000_0040;

/// `SEE_MASK_NOASYNC` — do not return until the invocation is complete.
const SEE_MASK_NOASYNC: u32 = 0x0000_0100;

/// `SW_SHOWNORMAL`.
const SW_SHOWNORMAL: i32 = 1;

/// `WAIT_TIMEOUT` — the object is not signalled, so the process is still running.
const WAIT_TIMEOUT: u32 = 0x0000_0102;

/// `SHELLEXECUTEINFOW`, x64 layout, checked against `shellapi.h` field by field.
#[repr(C)]
struct ShellExecuteInfoW {
    cb_size: u32,
    mask: u32,
    hwnd: isize,
    verb: *const u16,
    file: *const u16,
    parameters: *const u16,
    directory: *const u16,
    show: i32,
    inst_app: isize,
    id_list: *mut core::ffi::c_void,
    class: *const u16,
    key_class: isize,
    hot_key: u32,
    icon_or_monitor: isize,
    process: isize,
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteExW(info: *mut ShellExecuteInfoW) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetProcessId(process: isize) -> u32;
    fn WaitForSingleObject(handle: isize, milliseconds: u32) -> u32;
}

/// Starts `file` with `parameters` through `ShellExecuteEx` and returns its process id —
/// task **T-10-11**.
///
/// # ⚠ Why this exists beside [`Installed::launch`] rather than inside it
///
/// The two want different things of the handle. [`Installed`] **keeps** it, because that handle is
/// how it answers `is_alive` without going near the process table — the point of T-10-10 being that
/// «продукт прожил весь опыт» has to be shown rather than asserted. This one wants only the id, and
/// closes the handle before returning, because its caller tracks liveness by pid through `App`.
/// Merging them would mean giving a working, reviewed launch path a second lifetime rule.
///
/// # ⚠ Why `ShellExecuteEx` and not `Command::spawn`
///
/// Measured, in three passes, while building the console item of `--experiment-sequence`:
///
/// | How `conhost.exe <program>` was started | What happened |
/// |---|---|
/// | `Command::spawn` | conhost took over the **bench's own** console — its initialisation escape sequences came out of the bench's stdout — and then exited. No window |
/// | `Command::spawn` + `CREATE_NEW_CONSOLE` | the same. No window |
/// | `Command::spawn` + `DETACHED_PROCESS` + three null handles | conhost exited at once. No window |
/// | `ShellExecuteEx` | a real `ConsoleWindowClass` window, hosting the program, alive |
///
/// The cause is that `std`'s `Command` always passes explicit standard handles
/// (`STARTF_USESTDHANDLES`), and a console **host** handed somebody else's standard streams does
/// not go on to create the console it exists to create. `ShellExecuteEx` sets no such flag.
///
/// ⛔ Requirement A holds exactly as it does for [`Installed::launch`]: the id comes from the
/// handle of the launch itself and from nothing else, and it is the caller — `launched` — that
/// enters it in the registry, at that instant.
pub fn shell_execute(file: &std::path::Path, parameters: &str) -> Result<u32, String> {
    use std::os::windows::ffi::OsStrExt;

    let path: Vec<u16> = file
        .as_os_str()
        .encode_wide()
        .chain(core::iter::once(0))
        .collect();
    let verb: Vec<u16> = "open".encode_utf16().chain(core::iter::once(0)).collect();
    let args: Vec<u16> = parameters
        .encode_utf16()
        .chain(core::iter::once(0))
        .collect();

    let mut info = ShellExecuteInfoW {
        cb_size: core::mem::size_of::<ShellExecuteInfoW>() as u32,
        mask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        hwnd: 0,
        verb: verb.as_ptr(),
        file: path.as_ptr(),
        parameters: args.as_ptr(),
        directory: core::ptr::null(),
        show: SW_SHOWNORMAL,
        inst_app: 0,
        id_list: core::ptr::null_mut(),
        class: core::ptr::null(),
        key_class: 0,
        hot_key: 0,
        icon_or_monitor: 0,
        process: 0,
    };

    // SAFETY: `info` is a live local whose `cb_size` is its own size; `verb`, `path` and `args`
    // are alive across the call and NUL-terminated by construction, and every other pointer field
    // is null, which the documentation permits. NFR-13: a zero return is the documented failure
    // and is examined rather than discarded.
    let ok = unsafe { ShellExecuteExW(&raw mut info) };

    if ok == 0 {
        return Err(format!(
            "ShellExecuteEx не запустила {}: {}",
            file.display(),
            std::io::Error::last_os_error()
        ));
    }

    if info.process == 0 {
        return Err("ShellExecuteEx не вернула описатель процесса".to_owned());
    }

    // SAFETY: `info.process` is the handle `ShellExecuteEx` has just returned, and
    // `SEE_MASK_NOCLOSEPROCESS` makes this side its owner. NFR-13: a zero return is examined.
    let pid = unsafe { GetProcessId(info.process) };
    let failure = std::io::Error::last_os_error();

    // SAFETY: closing the handle this call owns, exactly once, on every path out.
    unsafe { CloseHandle(HANDLE(info.process as *mut core::ffi::c_void)) }.ok();

    if pid == 0 {
        return Err(format!(
            "не удалось прочитать pid запущенного процесса: {failure}"
        ));
    }

    Ok(pid)
}

/// The installed, signed copy in `%ProgramFiles%` carrying `uiAccess="true"` — task
/// **T-10-10**, and the one configuration the defect under investigation was ever seen in.
///
/// # Why this is a separate type and not another constructor of [`Sut`]
///
/// [`Sut`] is built around [`Child`], and a `uiAccess` process cannot be a child of this bench.
/// Three measured facts, each of which alone rules the ordinary path out:
///
/// | Measured | Consequence |
/// |---|---|
/// | `CreateProcess` on a `uiAccess` binary returns `ERROR_ELEVATION_REQUIRED`, 740 — T-10-9 §9.1 | `Command::spawn` cannot start it at all: the `uiAccess` flag is placed in the token by the AppInfo service, and only `ShellExecuteEx` reaches that service |
/// | `Stop-Process` on the running copy answers **«Access is denied»** | `TerminateProcess` is refused — a `uiAccess` token sits above a plain medium-integrity one, so the bench cannot kill what it started |
/// | joining a job needs `PROCESS_SET_QUOTA` on that same handle | the job object, the safety layer that survives the bench being killed, is not available here |
///
/// So the stopping mechanism is **FR-96 and only FR-96**. That is not a weakening: FR-96 is what
/// the task's environment section names as the way to end this product, and it was measured
/// against exactly this copy before the experiment was built.
///
/// # ⛔ Requirement A is honoured exactly, not relaxed
///
/// The pid is taken from the process handle `ShellExecuteEx` hands back **at the moment of
/// launch** — `GetProcessId` on that handle and nothing else — and goes straight into registry
/// A. This is the narrow exception the task grants and it stays narrow: no search for "any
/// `LangSwitcher` in the session", no window-to-pid derivation, no path by which a copy this run
/// did not start could enter the registry. [`any_running`] stays what it always was, a read-only
/// diagnostic on the way in and out, and is never a source of pids to act on.
pub struct Installed {
    handle: isize,
    pub pid: u32,
}

impl Installed {
    /// Starts the installed copy through `ShellExecuteEx` and registers it under requirement A.
    ///
    /// `LANGSW_DEBUG_TIMEOUT_SEC` is **not** set, and could not help if it were: the FR-97
    /// deadline lives behind `#[cfg(debug_assertions)]`, so a Release build carries none. The
    /// product therefore lives until FR-96 ends it — which is the point of this experiment,
    /// whose whole subject is a **long session** that is not restarted between rounds.
    pub fn launch() -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;

        let exe = std::path::PathBuf::from(r"C:\Program Files\Lang_Switcher")
            .join(lang_switcher::EXE_NAME);

        if !exe.exists() {
            return Err(format!("{} не найден", exe.display()));
        }

        let file: Vec<u16> = exe
            .as_os_str()
            .encode_wide()
            .chain(core::iter::once(0))
            .collect();
        let verb: Vec<u16> = "open".encode_utf16().chain(core::iter::once(0)).collect();

        let mut info = ShellExecuteInfoW {
            cb_size: core::mem::size_of::<ShellExecuteInfoW>() as u32,
            mask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
            hwnd: 0,
            verb: verb.as_ptr(),
            file: file.as_ptr(),
            parameters: core::ptr::null(),
            directory: core::ptr::null(),
            show: SW_SHOWNORMAL,
            inst_app: 0,
            id_list: core::ptr::null_mut(),
            class: core::ptr::null(),
            key_class: 0,
            hot_key: 0,
            icon_or_monitor: 0,
            process: 0,
        };

        // SAFETY: `info` is a live local whose `cb_size` is its own size, and the two string
        // pointers address `file` and `verb` — both alive across the call and both
        // NUL-terminated by construction. Every other pointer field is null, which the
        // documentation permits. NFR-13: a zero return is the documented failure and is
        // examined below rather than discarded.
        let ok = unsafe { ShellExecuteExW(&raw mut info) };

        if ok == 0 {
            return Err(format!(
                "ShellExecuteEx не запустила {}: {}",
                exe.display(),
                std::io::Error::last_os_error()
            ));
        }

        if info.process == 0 {
            return Err("ShellExecuteEx не вернула описатель процесса".to_owned());
        }

        // SAFETY: `info.process` is the process handle `ShellExecuteEx` has just returned, and
        // `SEE_MASK_NOCLOSEPROCESS` makes this side its owner. NFR-13: a zero return means the
        // id could not be read and is examined.
        let pid = unsafe { GetProcessId(info.process) };

        if pid == 0 {
            let error = std::io::Error::last_os_error();
            // SAFETY: closing the handle this call owns, on the one path that abandons it.
            unsafe { CloseHandle(HANDLE(info.process as *mut core::ffi::c_void)) }.ok();
            return Err(format!(
                "не удалось прочитать pid запущенного процесса: {error}"
            ));
        }

        // ⛔ Requirement A: the pid comes from the handle of the launch itself, and it enters
        // the registry here, at that instant — before any window exists to be found by, and
        // before anything in this run has looked at the process table.
        crate::own::register_spawned(pid);

        Ok(Self {
            handle: info.process,
            pid,
        })
    }

    /// Waits until the product reports its hook installed, through the SEC-04a channel.
    ///
    /// ⭐ This is what task T-10-10 bought. The installed copy is now built
    /// `--release --features testing`, so it carries the `uiAccess` manifest **and** the channel
    /// at once — the manifest is chosen by `PROFILE` in `build.rs`, the channel by the feature,
    /// and the two are independent. T-10-9 had to fall back on «процесс жив» here.
    pub fn await_ready(&self, timeout: Duration) -> Option<channel::Snapshot> {
        channel::await_hook(timeout)
    }

    /// Whether the process is still running — asked of the handle, never of the process table.
    pub fn is_alive(&self) -> bool {
        // SAFETY: `handle` is the live process handle this type owns, and a zero timeout makes
        // the call a poll. NFR-13: the return is compared against `WAIT_TIMEOUT` rather than
        // ignored — any other value means the object is signalled or the wait failed, and both
        // of those mean the copy is no longer running.
        unsafe { WaitForSingleObject(self.handle, 0) == WAIT_TIMEOUT }
    }

    /// Ends the product through FR-96 and confirms it went.
    ///
    /// ⚠ **The synthetic FR-96 takes down *any* copy in the session**, so the confirmation here
    /// is not a formality: it is what says the copy that went is the one this handle names. The
    /// caller checks there is no second copy before the run and none after it.
    ///
    /// There is deliberately no forcible fallback. `TerminateProcess` on this process answers
    /// «Access is denied» — measured — so a fallback would be a line that cannot run, and the
    /// job object that [`Sut::stop`] falls back to could not be joined in the first place. If
    /// FR-96 does not end it, that is a finding and it is reported as one.
    pub fn stop(&mut self) -> Result<(), String> {
        if !self.is_alive() {
            return Ok(());
        }

        input::emergency_combination().map_err(|error| format!("FR-96: {error}"))?;

        if wait::until_true(Duration::from_secs(10), || !self.is_alive()) {
            Ok(())
        } else {
            Err(format!(
                "FR-96 не завершила установленный экземпляр {} за 10 с; снять его \
                 принудительно нельзя — TerminateProcess на uiAccess-процессе отвечает \
                 «Access is denied»",
                self.pid
            ))
        }
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        if self.is_alive() {
            let _ = self.stop();
        }

        if self.handle != 0 {
            // SAFETY: the handle this type owns, closed once, on the way out.
            unsafe { CloseHandle(HANDLE(self.handle as *mut core::ffi::c_void)) }.ok();
        }
    }
}
