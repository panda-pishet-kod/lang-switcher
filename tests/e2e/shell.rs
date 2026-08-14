//! Bringing a window forward reliably — rake 3 of §2 of `TOOLCHAIN.md`, and the debt of §4.1а
//! of `STATE.md`.
//!
//! # The problem, stated once
//!
//! `SetForegroundWindow` called from a process that is not itself in the foreground is
//! **ignored** by Windows. Not refused with an error — ignored, returning a value that says
//! nothing useful, and only sometimes: whether it works depends on whether the desktop is
//! currently willing to hand focus over, which changes during a session. That is exactly the
//! failure mode recorded in §4.1а of `STATE.md`, where five behavioural tests in
//! `tests\inject.rs` pass early in a session and stop passing later on the same commit.
//!
//! `IUIAutomationElement::SetFocus` is not a way around it either: Word's document answers it
//! with "Target element cannot receive focus".
//!
//! # What works
//!
//! `WScript.Shell.AppActivate` by process id. It goes through the shell's own foreground
//! arbitration rather than around it, and after it the focus lands on the content element and
//! `SendInput` arrives. It needs late binding through `IDispatch`, it is already debugged in
//! `probe-word.ps1`, and decision Р-29 names it as one of exactly two operations the bench may
//! perform by running PowerShell as a child process.
//!
//! ⚠ **This module is used for every application the bench drives, not only for Word.** The
//! rake is not a Word rake — Word is only where it was found. Making one technique serve all
//! positions is what turns the §4.1а debt from an observation into a fixed method; the
//! conclusion the task asks for about those five tests is in the report, not here, because
//! `tests\inject.rs` is not this task's to edit.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::{input, wait};

/// How long to keep asking for a window to come forward.
const ACTIVATE_TIMEOUT: Duration = Duration::from_secs(30);

/// A helper script of the bench, beside the bench's own source.
///
/// ⚠ `<dev>\tools\admin\` is read-only for this task — the scripts there belong to the setup
/// stage. The bench's own live in `tests\e2e\`, as the task requires.
pub fn script(name: &str) -> Result<PathBuf, String> {
    // Resolved from the manifest directory at compile time: the bench runs out of a build
    // directory that contains no scripts, so the path cannot be derived from the executable.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("e2e")
        .join(name);

    if path.exists() {
        Ok(path)
    } else {
        Err(format!("{} не найден", path.display()))
    }
}

/// Runs one of the bench's PowerShell helpers and reports whether it succeeded and what it said.
pub fn run_script(name: &str, args: &[String]) -> Result<(bool, String), String> {
    let path = script(name)?;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&path)
        .args(args)
        .output()
        .map_err(|error| format!("не удалось запустить {name}: {error}"))?;

    let said = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok((output.status.success(), said))
}

/// Brings the windows of `pid` forward, and waits until the system agrees that they are.
///
/// Two conditions, not one. `AppActivate` returning true means the request was made; the
/// foreground window actually belonging to `pid` is the fact the next `SendInput` depends on,
/// and it is what this waits for. The retry loop lives here rather than inside the script so
/// that the bench keeps exactly one sleeping place — see `wait`.
pub fn activate(pid: u32) -> Result<(), String> {
    let asked = wait::until_true(ACTIVATE_TIMEOUT, || {
        // Ask once per poll; the script itself does not loop and does not sleep.
        let asked = run_script(
            "word-activate.ps1",
            &["-ProcessId".to_owned(), pid.to_string()],
        )
        .map(|(ok, _)| ok)
        .unwrap_or(false);

        asked && input::foreground().is_some_and(|(front, _)| front == pid)
    });

    if asked {
        Ok(())
    } else {
        let front = input::foreground().map_or("нет".to_owned(), |(p, _)| p.to_string());
        Err(format!(
            "окно процесса {pid} не удалось вывести вперёд за {} с; переднее окно принадлежит \
             процессу {front}",
            ACTIVATE_TIMEOUT.as_secs()
        ))
    }
}

// ---------------------------------------------------------------------------------------
// Processes the bench opened but did not spawn
// ---------------------------------------------------------------------------------------
//
// ⚠ Several applications of the matrix do not own their window in the process `spawn`
// returned. `notepad.exe` in System32 is a stub that hands over to the packaged Notepad;
// Chrome and VS Code spread over a process tree; `wt.exe` exits at once; `explorer.exe` hands
// the request to the running shell. For all of them the bench identifies the **window** first
// and then works with the window's own process — which is also the process the foreground
// check compares against, so the check keeps meaning what it says.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    TerminateProcess,
};

/// `STILL_ACTIVE` — the exit code of a process that has not exited.
const STILL_ACTIVE: u32 = 259;

/// Whether a process with this id is still running.
pub fn process_is_alive(pid: u32) -> bool {
    // SAFETY: `OpenProcess` takes an access mask, an inheritance flag and an id, and returns a
    // handle or an error. The minimum access that answers the question is asked for. NFR-13:
    // an error means the process is gone or unreachable, and is answered with `false`.
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return false;
    };

    let mut code = 0u32;
    // SAFETY: `handle` was just opened with the access this call needs; `code` is a live local
    // that outlives the call. NFR-13: the result is examined — a failed query is read as
    // "gone", which is the safe direction for a caller that is waiting for it to go.
    let alive = unsafe { GetExitCodeProcess(handle, &mut code) }.is_ok() && code == STILL_ACTIVE;

    // SAFETY: `handle` is the handle opened above, closed exactly once.
    let _ = unsafe { CloseHandle(handle) };

    alive
}

/// ⛔ Ends a process by id — **the only place in the bench that ends anything**.
///
/// # Requirement D lives on the first line of this function
///
/// `own::may_touch` is asked **before** the process handle is even opened, and a refusal
/// returns the reason instead of the force. That is what makes requirement D — "принудительное
/// снятие допустимо исключительно для процессов пункта A" — a property of the code rather than
/// a promise about how the code is called: there is one `TerminateProcess` in `tests\e2e\**`,
/// it is below, and it is unreachable without passing the gate.
///
/// ⚠ **Never used for Word** — rake 5 of §2 of `TOOLCHAIN.md`: a killed Word poisons the next
/// launch with a safe-mode prompt and a `Resiliency\DisabledItems` entry. Word leaves through
/// COM `Quit(0)` in `word::quit` and through nothing else.
pub fn terminate(pid: u32) -> Result<(), String> {
    // ⛔ Requirement D. Nothing below runs for a process the bench did not start.
    crate::own::may_touch(pid)?;

    // SAFETY: `OpenProcess` with the terminate right, on an id that has just passed the registry
    // gate above. NFR-13: examined.
    let handle: HANDLE = unsafe { OpenProcess(PROCESS_TERMINATE, false, pid) }
        .map_err(|error| format!("OpenProcess({pid}): {error}"))?;

    // SAFETY: `handle` was opened with `PROCESS_TERMINATE` immediately above. NFR-13: the
    // result is examined and reported.
    let ended = unsafe { TerminateProcess(handle, 0) };

    // SAFETY: the handle opened above, closed exactly once.
    let _ = unsafe { CloseHandle(handle) };

    ended.map_err(|error| format!("TerminateProcess({pid}): {error}"))
}

// The "is the foreground window ours" question deliberately has **no** helper here. It is
// asked in exactly one place — `input::send_verified` — immediately before every send, and a
// second way of asking it would be a second thing to keep in step. `activate` above waits on
// the same fact through `input::foreground` for the same reason.
