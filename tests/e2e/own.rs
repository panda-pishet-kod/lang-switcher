//! ⛔ The registry of processes this bench is allowed to touch — requirements A to E.
//!
//! # Why this module exists
//!
//! An earlier version of the bench searched for windows **across every process on the
//! machine** and, having found one matching a predicate, adopted its process and closed it.
//! Twice that predicate matched a window belonging to the controller's own Claude Code session,
//! and twice the bench closed it. Nothing appeared in the event log, because `WM_CLOSE` to a
//! well-behaved application is not a fault — it is a polite request, honoured.
//!
//! The lesson is not "write a better predicate". A predicate is a guess about identity, and a
//! guess must never be what stands between an automated tool and somebody else's work. This
//! module replaces the guess with a fact: **the bench may touch a process only if the bench
//! started it.**
//!
//! # The five requirements, and where each one lives
//!
//! | | Requirement | Here |
//! |---|---|---|
//! | **A** | a registry of processes the bench started, and their descendants; filled only at launch | [`register_spawned`], [`descends_from_ours`] |
//! | **B** | a window whose process is not in the registry is **not adopted** — the position fails and the run continues; no `WM_CLOSE` and no terminate is ever sent to it | [`claim_window_process`] |
//! | **C** | a second belt: a process **named** `claude`, `powershell`, `pwsh`, `conhost`, `WindowsTerminal`, `explorer`, `Code` or `cmd` is refused unless this very instance was spawned by the bench in this run | [`PROTECTED_NAMES`], [`may_touch`] |
//! | **D** | terminate only by registry | [`may_touch`], enforced at the single call site in `shell::terminate` |
//! | **E** | the foreground window must belong to a registry process before any input is sent | [`is_ours`], called from `input::send_verified` |
//!
//! # Two sets, not one, and the difference matters
//!
//! * [`SPAWNED`] — process ids that came back from a `Command::spawn` of the bench's own. These
//!   are the only ones requirement C accepts for a protected name.
//! * [`ADOPTED`] — process ids proved to descend from a `SPAWNED` one by walking the parent
//!   chain. They are "ours" for requirements A, B, D and E, but they do **not** satisfy C.
//!
//! Keeping them apart is what stops a descendant relationship from becoming a loophole: if the
//! bench ever spawned something that turned out to have `explorer.exe` somewhere beneath it,
//! that `explorer.exe` would still be refused.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use std::sync::Mutex;

/// Processes the bench started itself.
static SPAWNED: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// Processes proved to descend from a [`SPAWNED`] one.
static ADOPTED: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// Processes already found not to be ours.
///
/// Purely a cache, and a sound one: the answer cannot change from "not ours" to "ours". A
/// process only enters [`SPAWNED`] at the instant the bench launches it — before any window
/// search can see it — and [`ADOPTED`] only through [`claim_window_process`], which consults
/// this set first. Without the cache, `adopt_window` would read the process table through a
/// PowerShell child process on **every candidate of every poll**, tens of times a second, for
/// as long as it waits for a window that may never come.
/// Keyed by process id, holding the **full** reason. Requirement B asks the report to name the
/// process a window really belonged to, and a cache that answered "refused earlier" would take
/// that name away from every refusal after the first.
static REFUSED: Mutex<BTreeMap<u32, String>> = Mutex::new(BTreeMap::new());

/// How far up a parent chain to walk before giving up.
///
/// A window's process is a child or a grandchild of what was launched, never more; the bound
/// exists so that a cycle in a stale process table cannot spin here.
const MAX_ANCESTRY: usize = 8;

/// ⛔ Names that are refused unless this very process was spawned by the bench in this run.
///
/// The second belt of requirement C. `claude` is on it because of what happened; the rest are
/// there because they are the processes that host a person's work on this machine — a shell, a
/// terminal, the desktop itself, the editor. Losing any of them costs somebody their session.
///
/// ⚠ This list is matched against the executable name **with and without** the `.exe` suffix,
/// case-insensitively.
pub const PROTECTED_NAMES: [&str; 8] = [
    "claude",
    "powershell",
    "pwsh",
    "conhost",
    "WindowsTerminal",
    "explorer",
    "Code",
    "cmd",
];

/// Records a process the bench has just started — requirement A.
///
/// ⚠ **The only way anything enters the registry as a root.** Requirement A says the registry
/// is filled at launch and nowhere else, and this is the only function that writes to
/// [`SPAWNED`].
pub fn register_spawned(pid: u32) {
    if let Ok(mut spawned) = SPAWNED.lock() {
        spawned.insert(pid);
    }
}

/// Whether the bench may work with this process at all — requirements A, B, D and E.
pub fn is_ours(pid: u32) -> bool {
    SPAWNED.lock().is_ok_and(|set| set.contains(&pid))
        || ADOPTED.lock().is_ok_and(|set| set.contains(&pid))
}

/// Whether this process was started by the bench itself, as opposed to merely descending from
/// something that was — the distinction requirement C turns on.
fn was_spawned(pid: u32) -> bool {
    SPAWNED.lock().is_ok_and(|set| set.contains(&pid))
}

/// The gate every destructive action passes through — requirements C and D.
///
/// `Ok(())` means the bench started this process and may close or end it. `Err` carries a
/// sentence fit to go straight into a report: a refusal is a finding, not an error.
pub fn may_touch(pid: u32) -> Result<(), String> {
    if pid == std::process::id() {
        return Err(format!("процесс {pid} — сам стенд, трогать нельзя"));
    }

    let name = process_name(pid).unwrap_or_else(|| "<имя неизвестно>".to_owned());

    if !is_ours(pid) {
        return Err(format!(
            "⛔ процесс {pid} ({name}) НЕ запускался стендом — реестр пункта A его не содержит; \
             ни WM_CLOSE, ни принудительное снятие по нему не отправляются"
        ));
    }

    // Requirement C, the second belt: a protected name needs more than membership by descent.
    if is_protected(&name) && !was_spawned(pid) {
        return Err(format!(
            "⛔ процесс {pid} ({name}) носит имя из запретного списка пункта C и не был запущен \
             стендом в этом прогоне — отказ, даже несмотря на попадание в реестр"
        ));
    }

    Ok(())
}

/// Whether a process name is on the protected list.
pub fn is_protected(name: &str) -> bool {
    let bare = name.strip_suffix(".exe").unwrap_or(name);
    PROTECTED_NAMES
        .iter()
        .any(|protected| bare.eq_ignore_ascii_case(protected))
}

/// ⛔ **Requirement B.** Decides whether a window found by a predicate may be adopted.
///
/// A predicate matches on class and title, which are things anybody's window can have. This
/// asks the question that actually matters — did the bench start the process behind it — and
/// on `Err` the caller must record a `fail` and move on **without touching the window at all**.
///
/// On success the process is added to [`ADOPTED`], so the checks that follow it during the
/// scenario are cheap set lookups rather than repeated walks of the process table.
pub fn claim_window_process(pid: u32) -> Result<(), String> {
    if is_ours(pid) {
        return Ok(());
    }

    // The cache of previous refusals — see `REFUSED`. The stored reason is returned in full, so
    // a polling caller gets the same sentence every time without re-reading the process table.
    if let Ok(refused) = REFUSED.lock()
        && let Some(reason) = refused.get(&pid)
    {
        return Err(reason.clone());
    }

    let name = process_name(pid).unwrap_or_else(|| "<имя неизвестно>".to_owned());
    let refuse = |reason: String| -> Result<(), String> {
        if let Ok(mut refused) = REFUSED.lock() {
            refused.insert(pid, reason.clone());
        }
        Err(reason)
    };

    // Even before the ancestry walk: a protected name is never adopted by descent.
    if is_protected(&name) {
        return refuse(format!(
            "⛔ окно принадлежит процессу {pid} ({name}) — имя из запретного списка пункта C; \
             окно не усыновлено, ничего ему не отправлено"
        ));
    }

    if descends_from_ours(pid) {
        if let Ok(mut adopted) = ADOPTED.lock() {
            adopted.insert(pid);
        }
        return Ok(());
    }

    refuse(format!(
        "⛔ окно принадлежит постороннему процессу {pid} ({name}): стенд его не запускал и он не \
         происходит ни от одного запущенного стендом. Окно НЕ усыновлено, ни WM_CLOSE, ни \
         принудительное снятие по нему не отправлены (пункт B)"
    ))
}

/// Walks the parent chain looking for a process the bench spawned — requirement A's
/// "and their descendants".
fn descends_from_ours(pid: u32) -> bool {
    let Ok(roots) = SPAWNED.lock().map(|set| set.clone()) else {
        return false;
    };
    if roots.is_empty() {
        return false;
    }

    let table = process_table();
    let mut current = pid;

    for _ in 0..MAX_ANCESTRY {
        let Some(entry) = table.get(&current) else {
            return false;
        };
        let parent = entry.parent;

        // A parent of zero is the top of the tree; a self-parent is a stale table.
        if parent == 0 || parent == current {
            return false;
        }
        if roots.contains(&parent) {
            return true;
        }
        current = parent;
    }

    false
}

/// One row of the process table.
struct Entry {
    parent: u32,
    name: String,
}

/// The process table: id, parent id, executable name.
///
/// Read through PowerShell rather than `CreateToolhelp32Snapshot` because the feature
/// `Win32_System_Diagnostics_ToolHelp` is outside the closed dependency list of §3.2 of SPEC,
/// and the task allows exactly one new section in `Cargo.toml`. This is a **read-only**
/// query — it is the check that decides whether anything may be touched, never an action.
fn process_table() -> BTreeMap<u32, Entry> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_Process | ForEach-Object { \
             '{0};{1};{2}' -f $_.ProcessId, $_.ParentProcessId, $_.Name }",
        ])
        .output();

    let Ok(output) = output else {
        return BTreeMap::new();
    };

    let mut table = BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.trim().split(';');
        let (Some(pid), Some(parent), Some(name)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let (Ok(pid), Ok(parent)) = (pid.trim().parse::<u32>(), parent.trim().parse::<u32>())
        else {
            continue;
        };
        table.insert(
            pid,
            Entry {
                parent,
                name: name.trim().to_owned(),
            },
        );
    }

    table
}

/// The executable name of a process, for refusals and for requirement C.
pub fn process_table_name(pid: u32) -> Option<String> {
    process_table().get(&pid).map(|entry| entry.name.clone())
}

/// [`process_table_name`], kept short at the call sites.
fn process_name(pid: u32) -> Option<String> {
    process_table_name(pid)
}

/// Whether the registry is empty — nothing launched, nothing adopted.
///
/// With an empty registry [`descends_from_ours`] returns `false` before it even reads the
/// process table, so [`is_ours`] is `false` for every process on the machine and [`may_touch`]
/// refuses all of them. The demonstration of requirement 15г asserts this before it calls
/// `shell::terminate` on a live foreign process id, so that the demonstration cannot itself
/// become the accident it exists to rule out.
pub fn registry_is_empty() -> bool {
    SPAWNED.lock().is_ok_and(|set| set.is_empty()) && ADOPTED.lock().is_ok_and(|set| set.is_empty())
}

/// Forgets the refusal cache — called between positions.
///
/// Not for correctness: a refused process stays refused. It keeps the cache from growing over a
/// long run and keeps the refusal a position reports fresh rather than inherited from an earlier
/// one, which matters when the report is read position by position.
pub fn forget_refusals() {
    if let Ok(mut refused) = REFUSED.lock() {
        refused.clear();
    }
}

/// Every refusal recorded during the current position, for the report.
pub fn refusals() -> Vec<String> {
    REFUSED
        .lock()
        .map(|refused| refused.values().cloned().collect())
        .unwrap_or_default()
}

/// The parent chain of a process, innermost first — the **measurement** behind requirement A.
///
/// Criterion 29 asks each of six positions to be classified by measuring whether the window's
/// process is kin to something the bench started, rather than by assuming it from how the
/// application is known to behave. This is that measurement, printed rather than inferred:
/// every step of the chain with its process name, so the reader can see where it leaves the
/// bench's own subtree — or that it never entered it.
/// ⚠ A parent that has already exited is still **named**, as `<завершился>`, rather than
/// silently ending the chain. The distinction matters for the measurement: the System32
/// `notepad.exe` is a stub that hands over to the packaged application and exits at once, so a
/// chain that simply stopped at the packaged process would read as "no kin" when the truth is
/// "kin, and the parent is already gone".
pub fn ancestry(pid: u32) -> Vec<(u32, String)> {
    let table = process_table();
    let mut chain = Vec::new();
    let mut current = pid;

    for _ in 0..MAX_ANCESTRY {
        let Some(entry) = table.get(&current) else {
            chain.push((current, "<завершился>".to_owned()));
            break;
        };
        chain.push((current, entry.name.clone()));

        if entry.parent == 0 || entry.parent == current {
            break;
        }
        current = entry.parent;
    }

    chain
}

/// The measurement requirement 29 asks for, on its own: does this process descend from one the
/// bench started?
///
/// Separate from [`claim_window_process`] because that one refuses a protected name **before**
/// it walks the parent chain — correct for safety, useless for classification, since it would
/// report every `WindowsTerminal.exe` as "not ours" without ever having looked.
pub fn kinship(pid: u32) -> Option<u32> {
    let roots = SPAWNED.lock().ok().map(|set| set.clone())?;
    if roots.is_empty() {
        return None;
    }
    if roots.contains(&pid) {
        return Some(pid);
    }

    let table = process_table();
    let mut current = pid;

    for _ in 0..MAX_ANCESTRY {
        let entry = table.get(&current)?;
        if entry.parent == 0 || entry.parent == current {
            return None;
        }
        if roots.contains(&entry.parent) {
            return Some(entry.parent);
        }
        current = entry.parent;
    }

    None
}

/// Whether this process id is one the bench spawned itself — for the classification report.
pub fn is_spawned_root(pid: u32) -> bool {
    was_spawned(pid)
}

/// Process ids currently running under this executable name — read-only, for measurement.
pub fn pids_named(name: &str) -> Vec<u32> {
    let wanted = name.trim();
    process_table()
        .into_iter()
        .filter(|(_, entry)| entry.name.eq_ignore_ascii_case(wanted))
        .map(|(pid, _)| pid)
        .collect()
}

/// Everything the registry holds, for the report.
pub fn describe() -> String {
    let spawned = SPAWNED
        .lock()
        .map(|set| {
            set.iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let adopted = ADOPTED
        .lock()
        .map(|set| {
            set.iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    format!("запущены стендом: [{spawned}]; усыновлены как потомки: [{adopted}]")
}
