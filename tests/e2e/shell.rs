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
/// [`activate_window`] with the window handle, when the caller has it.
///
/// ⚠ **Why a handle helps, and why using it is allowed.** `AppActivate` addresses a *process*
/// and lets the shell pick which of its windows to raise; with a handle the bench can also ask
/// Win32 directly — `ShowWindow(SW_RESTORE)` for a window that came up minimised, and
/// `SetForegroundWindow` on that exact window. Both act on a window whose process has already
/// passed `own::claim_window_process`, so it is in registry A and requirements A–E permit it:
/// the rule is "nothing the bench did not start", not "no Win32".
///
/// The order matters. The cheap Win32 pair is tried first on every poll; `AppActivate` costs a
/// PowerShell process and is the fallback that goes through the shell's own arbitration when
/// Win32 is ignored — rake 3.
pub fn activate_window(
    pid: u32,
    hwnd: Option<windows::Win32::Foundation::HWND>,
) -> Result<(), String> {
    // ⛔ The gate, before anything is done to any window.
    if !crate::own::is_ours(pid) {
        return Err(format!(
            "⛔ процесс {pid} не в реестре A — стенд не выводит вперёд чужие окна"
        ));
    }

    let asked = wait::until_true(ACTIVATE_TIMEOUT, || {
        if let Some(handle) = hwnd {
            raise_own_window(handle);

            if input::foreground().is_some_and(|(front, _)| front == pid) {
                return true;
            }
        }

        ask_and_check(pid)
    });

    if asked {
        return Ok(());
    }

    // ⚠ Losing the race for the foreground is an **ordinary state of a desktop**, not a
    // breakage, and the bench has to survive it: the position fails, naming who holds the
    // foreground, and the run carries on. It must never stop to ask a person for a keystroke —
    // a position that needs one is **П** by the definition of §11.6 and belongs to the
    // acceptance session, not to an automatic run.
    let holder = match input::foreground() {
        Some((front, _)) => {
            let name =
                crate::own::process_table_name(front).unwrap_or_else(|| "<имя неизвестно>".into());
            format!("процессу {front} ({name})")
        }
        None => "никому — переднего окна нет".to_owned(),
    };

    Err(format!(
        "окно процесса {pid} не удалось вывести вперёд за {} с; передний план принадлежит {holder}",
        ACTIVATE_TIMEOUT.as_secs()
    ))
}

/// Raises a window that belongs to a process in registry A, past the foreground lock.
///
/// ⚠ **`SetForegroundWindow` alone is ignored when the caller is not itself in the foreground**
/// — rake 3, and the bench is always a background console process. The documented way past it
/// is to attach this thread's input queue to the queue of whichever thread owns the foreground:
/// while attached the two share foreground rights, so the call stops being ignored, and the
/// attachment is undone at once.
///
/// This is the same technique the five `#[ignore]` tests of `tests\inject.rs` were moved onto in
/// part 2, and for the same reason: it addresses **our own** window, where `AppActivate` — which
/// looks a window up by process or title — has nothing reliable to go on. `AppActivate` stays as
/// the fallback in the caller, for the applications where it does work.
///
/// The caller re-reads `GetForegroundWindow` afterwards; nothing here is trusted on its return.
fn raise_own_window(handle: windows::Win32::Foundation::HWND) {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, SW_RESTORE,
        SetForegroundWindow, ShowWindow,
    };

    // SAFETY: no arguments; returns a handle by value, possibly null.
    let front = unsafe { GetForegroundWindow() };
    // SAFETY: a null `front` yields zero, which is examined below; `None` asks only for the
    // thread id and writes nothing of ours.
    let owner = unsafe { GetWindowThreadProcessId(front, None) };
    // SAFETY: no arguments; returns this thread's id.
    let ours = unsafe { GetCurrentThreadId() };

    // Attaching a thread to itself is an error; a zero owner means nothing holds the foreground.
    let attached = owner != 0 && owner != ours;

    if attached {
        // SAFETY: both ids name live threads — ours by construction, the other taken from the
        // current foreground window a moment ago. NFR-13: a failed attach is not fatal, it only
        // means the calls below run unprivileged, and the caller re-reads the fact regardless.
        unsafe {
            let _ = AttachThreadInput(ours, owner, true);
        }
    }

    // SAFETY: `handle` belongs to a process that passed the registry gate in `activate_window`.
    // All three calls take it by value and dereference nothing of ours; their `BOOL`s are
    // deliberately not fatal for the same reason.
    unsafe {
        if IsIconic(handle).as_bool() {
            let _ = ShowWindow(handle, SW_RESTORE);
        }
        let _ = BringWindowToTop(handle);
        let _ = SetForegroundWindow(handle);
    }

    if attached {
        // SAFETY: undoes exactly the attachment made above, with the same two ids. Leaving input
        // queues attached would tie this thread's fate to another one's.
        unsafe {
            let _ = AttachThreadInput(ours, owner, false);
        }
    }
}

/// One `AppActivate` and one re-read of the fact — rake 3, the shell-arbitrated way.
///
/// The script does not loop and does not sleep: the repetition belongs to `wait::until`, which
/// is the bench's single sleeping place.
fn ask_and_check(pid: u32) -> bool {
    let asked = run_script(
        "word-activate.ps1",
        &["-ProcessId".to_owned(), pid.to_string()],
    )
    .map(|(ok, _)| ok)
    .unwrap_or(false);

    asked && input::foreground().is_some_and(|(front, _)| front == pid)
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
