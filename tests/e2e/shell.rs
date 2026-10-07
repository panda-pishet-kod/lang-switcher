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
/// ⚠ The setup stage's own scripts live outside the repository, in the author's tools folder, and
/// are read-only for this task. The bench's own live in `tests\e2e\`, as the task requires.
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
/// and it is what this waits for. The retry loop lives here rather than inside the script so that
/// the waiting is a condition asked through `wait::until` and not a delay this file invents: the
/// module comment of `wait` carries the inventory of all **nine** `std::thread::sleep` call sites
/// of the bench and says what each of them is, and none of the nine is here — task **T-13-30**,
/// which brought this sentence in line with what T-13-26 had already measured.
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
/// The script does not loop and does not sleep: the repetition belongs to `wait::until`, whose
/// poll interval is the only sleep in the bench a verdict ever waits behind. It is not the only
/// sleep in the bench — there are **eleven**, listed one by one at the top of `wait` and re-derived
/// from the source on every run by `the_bench_sleeps_only_in_the_eleven_places_this_module_lists`.
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
///
/// ⛔ **The instant this answers «no» is the instant the registry record has to go — task
/// T-13-27.** A dead process gives its id back to Windows, which hands it out again; a record
/// that outlived the process would let the next holder of the number through gates D and E, which
/// is the audit finding of 2026-08-24 in its plainest form.
///
/// This is where `scenarios::App::close` learns that a position's application has gone, and it
/// learns it the same way for all three ways it closes them — COM `Quit(0)`, `WM_CLOSE` and
/// terminate alike — so striking the record here covers the close of **every** position without a
/// line of `scenarios.rs`, which task T-13-27 is forbidden to edit.
///
/// ⚠ The strike only ever **removes**, so it can only narrow what the bench is willing to touch.
/// A process wrongly reported dead — the handle could not be opened for a moment — is a process
/// the bench then refuses; that is the safe direction, and it is the same one
/// [`own::creation_time`](crate::own) documents.
pub fn process_is_alive(pid: u32) -> bool {
    let alive = still_running(pid);

    if !alive {
        crate::own::forget_process(pid);
    }

    alive
}

/// The question itself, apart from what [`process_is_alive`] does with the answer.
///
/// A separate function because the answer «gone» arrives by two roads — the handle would not open
/// at all, or it opened and the exit code is not `STILL_ACTIVE` — and the record has to be struck
/// on both.
fn still_running(pid: u32) -> bool {
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
///
/// # ⛔ The record goes with the process — task T-13-27
///
/// A successful `TerminateProcess` dooms the process, and its id is then Windows's to hand out
/// again. `own::forget_process` on the last line is what stops the registry from carrying the
/// number into the next position. It matters most for the launches the bench keeps **no handle
/// of** — `conhost.exe` through `ShellExecuteEx` — which is the exposure the audit of 2026-08-24
/// named: an open `Child` handle blocks reuse, and those launches have none.
///
/// ⚠ The consequence, stated so that nobody reads it as a bug: a **second** `terminate` of the
/// same id, after the first succeeded, is now refused by `own::may_touch` rather than performed.
/// That is the safe direction — the bench declines to end a process it can no longer prove is
/// its own — and it is the direction task T-13-27 requires of every change it makes.
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

    ended.map_err(|error| format!("TerminateProcess({pid}): {error}"))?;

    // ⛔ Task T-13-27, see the note above: the id is now free for Windows to reissue, so the
    // registry stops vouching for it here. A failed terminate does **not** reach this line — the
    // process is still alive and still the bench's own.
    crate::own::forget_process(pid);

    Ok(())
}

// The "is the foreground window ours" question deliberately has **no** helper here. It is
// asked in exactly one place — `input::send_verified` — immediately before every send, and a
// second way of asking it would be a second thing to keep in step. `activate` above waits on
// the same fact through `input::foreground` for the same reason.

// ---------------------------------------------------------------------------------------
// Task T-10-18 — reaching the product's window procedure with a message
// ---------------------------------------------------------------------------------------

use std::cell::RefCell;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowThreadProcessId, PostMessageW,
};

/// The class name of every window the product creates — `src\app.rs`, `WINDOW_CLASS_NAME`.
///
/// Two of the product's three windows are `HWND_MESSAGE` and therefore take no part in
/// top-level enumeration at all (its own module documentation says so). The **UI** window
/// cannot be one of them — decision Р-20 point 2 needs the broadcast of `TaskbarCreated` —
/// so it is a top-level window that is never shown, and it is the one this finds.
pub const PRODUCT_WINDOW_CLASS: &str = "LangSwitcher.Hidden";

thread_local! {
    /// Where [`enum_windows_proc`] accumulates; a thread-local rather than a `static mut`, and
    /// borrowed only inside the callback, which `EnumWindows` runs on this same thread before
    /// it returns.
    static FOUND: RefCell<Vec<(isize, u32)>> = const { RefCell::new(Vec::new()) };
}

/// `EnumWindows` callback — records every top-level window with its owning process id.
///
/// # Safety
///
/// Called by Windows with a live window handle. It reads nothing through a pointer of ours:
/// `lparam` is ignored deliberately, so a stray callback cannot dereference anything.
unsafe extern "system" fn enum_windows_proc(window: HWND, _lparam: LPARAM) -> windows::core::BOOL {
    let mut pid = 0u32;
    // SAFETY: `window` is the live handle Windows just handed this callback, and `pid` is a
    // live local of this frame. NFR-13: a zero return means the window died between the
    // enumeration and this call, and the entry is then dropped rather than recorded.
    let thread = unsafe { GetWindowThreadProcessId(window, Some(&raw mut pid)) };

    if thread != 0 && pid != 0 {
        FOUND.with(|found| found.borrow_mut().push((window.0 as isize, pid)));
    }

    // TRUE: keep enumerating. There is no early exit — the whole list is wanted.
    windows::core::BOOL(1)
}

/// The class name of a window, or `<класс не читается>`.
fn class_of(window: HWND) -> String {
    let mut buffer = [0u16; 257];
    // SAFETY: `buffer` is a live local array handed over as a slice, so the binding derives the
    // bound from the array itself and cannot write past it. NFR-13: a non-positive return is the
    // documented failure and is examined.
    let length = unsafe { GetClassNameW(window, &mut buffer) };

    if length <= 0 {
        return "<класс не читается>".to_owned();
    }

    String::from_utf16_lossy(&buffer[..length as usize])
}

/// ⭐ **Task T-10-18** — the product's **top-level** window, found by enumeration.
///
/// Both halves of the filter matter and neither is enough on its own: the process id says the
/// window belongs to the copy this run started, and the class says it is the product's rather
/// than something else the same process might own. `watchdog::is_ui_window` compares the
/// handle against the one `register_session_notice` published, so a message aimed anywhere
/// else falls through to `DefWindowProcW` — which is why the answer has to be exact and why a
/// run that finds none or finds several says so instead of guessing.
pub fn product_ui_window(pid: u32) -> Result<HWND, String> {
    FOUND.with(|found| found.borrow_mut().clear());

    // SAFETY: `enum_windows_proc` is a real `extern "system"` function of this binary and the
    // `lparam` it is handed is zero and never dereferenced. `EnumWindows` runs the callback on
    // this thread and returns after the last one, so the thread-local below is complete and
    // is not being borrowed anywhere else. NFR-13: the result is examined — the documented
    // failure is that the callback stopped the enumeration, which this one never does.
    let walked = unsafe { EnumWindows(Some(enum_windows_proc), LPARAM(0)) };

    let mut mine: Vec<HWND> = FOUND.with(|found| {
        found
            .borrow()
            .iter()
            .filter(|(_, owner)| *owner == pid)
            .map(|(raw, _)| HWND(*raw as *mut core::ffi::c_void))
            .collect()
    });

    if let Err(error) = walked {
        return Err(format!("EnumWindows: {error}"));
    }

    mine.retain(|window| class_of(*window) == PRODUCT_WINDOW_CLASS);

    match mine.len() {
        0 => Err(format!(
            "у процесса {pid} нет верхнеуровневого окна класса {PRODUCT_WINDOW_CLASS}"
        )),
        1 => Ok(mine[0]),
        several => Err(format!(
            "у процесса {pid} {several} верхнеуровневых окон класса {PRODUCT_WINDOW_CLASS} — \
             какое из них UI-окно, перечисление не говорит"
        )),
    }
}

/// Posts one message into a window of the product and **examines the Win32 return** — NFR-13.
///
/// ⚠ The examination is the whole point of the helper. Task T-10-13 measured what happens
/// without it: `PostMessageW` from a medium-integrity process into the **installed** copy, which
/// runs elevated, is refused by UIPI with `ERROR_ACCESS_DENIED`, and an experiment that did not
/// look would have recorded «пробуждение ничего не обновляет» on a message that never arrived.
/// The copy this task drives is a plain child of the bench and at the same integrity level, so
/// the post is expected to succeed — expected, and therefore checked.
///
/// Every message this is used for is one the product's own SEC-05 threat model already
/// enumerates: a forged `WM_POWERBROADCAST` or `WM_WTSSESSION_CHANGE` buys the sender a hook
/// reinstallation and nothing else.
pub fn post_to_window(window: HWND, message: u32, wparam: usize) -> Result<(), String> {
    // SAFETY: `window` came from `product_ui_window` on this same thread moments ago;
    // `PostMessageW` only queues a copy of the message and dereferences neither `wparam` nor
    // `lparam`, both of which are plain integers here. NFR-13: the result is examined and the
    // failure is returned rather than swallowed.
    unsafe { PostMessageW(Some(window), message, WPARAM(wparam), LPARAM(0)) }
        .map_err(|error| format!("PostMessage({message:#x}, wparam={wparam:#x}): {error}"))
}
