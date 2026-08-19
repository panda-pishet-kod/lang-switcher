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

        Ok(Self { child, pid })
    }

    /// ⚠ **The installed, signed Release copy in `%ProgramFiles%`** — task **T-10-9**, and the
    /// one configuration the defect under investigation was ever seen in.
    ///
    /// It is a different program from the one every other mode of this bench runs, in the two
    /// ways that could matter to a defect nobody has reproduced on the debug build: it is built
    /// **without** the `testing` feature, so SEC-04a is not merely quiet but absent, and it
    /// carries `uiAccess="true"` (§8.2), so it runs with the foreground and injection rights that
    /// manifest grants. Neither can be had from the build tree — a debug binary embeds
    /// `app-dev.manifest` and the manifests are outside this task's boundaries.
    ///
    /// ⛔ It is registered in registry A like every other process the bench starts, it is joined
    /// to the job object like every other one, and it is taken down by the synthetic FR-96 of
    /// [`Sut::stop`] — which is what the task's environment section names as the way to end it.
    /// `LANGSW_DEBUG_TIMEOUT_SEC` is set all the same and is expected to do nothing: FR-97 is a
    /// debug-only deadline. The job object is the net here.
    pub fn launch_installed() -> Result<Self, String> {
        let exe = std::path::PathBuf::from(r"C:\Program Files\Lang_Switcher")
            .join(lang_switcher::EXE_NAME);

        if !exe.exists() {
            return Err(format!("{} не найден", exe.display()));
        }

        let child = Command::new(&exe)
            .env("LANGSW_DEBUG_TIMEOUT_SEC", DEBUG_TIMEOUT_SECS.to_string())
            .spawn()
            .map_err(|error| format!("не удалось запустить {}: {error}", exe.display()))?;

        Ok(Self::adopt(child))
    }

    /// Registers a freshly spawned product and joins it to the job — shared by both launchers.
    fn adopt(child: Child) -> Self {
        let pid = child.id();

        // ⛔ Requirement A: the product is a process the bench started, and it enters the
        // registry here, at the moment of launch.
        crate::own::register_spawned(pid);

        let job = JOB.load(Ordering::SeqCst);
        if job != 0 {
            // SAFETY: identical to the call in `launch_with` — a live job handle and the process
            // handle the `Child` owns. NFR-13: examined.
            let ok = unsafe { AssignProcessToJobObject(job, child.as_raw_handle() as isize) };
            if ok == 0 {
                eprintln!(
                    "warning: AssignProcessToJobObject не сработала: {}",
                    std::io::Error::last_os_error()
                );
            }
        }

        Self { child, pid }
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
