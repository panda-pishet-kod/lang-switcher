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
//! * [`SPAWNED`] — processes that came back from a `Command::spawn` of the bench's own. These
//!   are the only ones requirement C accepts for a protected name.
//! * [`ADOPTED`] — processes proved to descend from a [`SPAWNED`] one by walking the parent
//!   chain. They are "ours" for requirements A, B, D and E, but they do **not** satisfy C.
//!
//! Keeping them apart is what stops a descendant relationship from becoming a loophole: if the
//! bench ever spawned something that turned out to have `explorer.exe` somewhere beneath it,
//! that `explorer.exe` would still be refused.
//!
//! # ⛔ Identity is a **pair**, never a bare number — task T-13-27
//!
//! Windows hands process ids back out. A number that belonged to something the bench launched
//! can, minutes later, belong to a person's editor; a registry keyed on the number alone would
//! then wave that editor through gates D and E — which is the accident above, arrived at by a
//! different road. So both sets hold a [`Proc`]: the id **and** the instant the process started.
//! Two live processes can share an id over time; they cannot share an id *and* a creation
//! `FILETIME`, which is stamped at a hundred-nanosecond resolution at the moment of creation and
//! never rewritten.
//!
//! ⛔ **The direction of refusal is fixed, and it is the point of the whole addition.** Reading a
//! start time can fail: the process has exited, or the rights are not there. Every such failure
//! is answered **«not our process»**. Never «ours», and never «fall back on the id alone, the way
//! it used to work» — falling back would restore, on exactly the unlucky path, the defect this
//! module was paid for. [`creation_time`] answers `Option`, [`identify`] answers `Option`, and
//! every caller in this file reads `None` as a refusal.
//!
//! A record is struck out when the bench sees the process end — [`forget_process`], called from
//! `shell::process_is_alive` the instant it answers «gone» and from `shell::terminate` the
//! instant it succeeds — and records that no longer name a live process are swept away when the
//! registry next grows. Both paths only ever **remove**, so the set of processes the bench is
//! willing to touch can only shrink, never widen.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::process::Command;
use std::sync::Mutex;

/// ⛔ The identity of a process: its id **together with** the instant it started.
///
/// The whole type exists so that a process id cannot be mistaken for an identity. Windows reuses
/// ids; it does not reissue a creation time, so the pair names one process for the whole life of
/// the run and a reused number does not match the record left by its predecessor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Proc {
    /// The process id — the number every Win32 call and every window takes.
    pid: u32,
    /// `lpCreationTime` of `GetProcessTimes`, its two `FILETIME` halves joined into one number.
    started: u64,
}

impl fmt::Display for Proc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{:#018x}", self.pid, self.started)
    }
}

/// Processes the bench started itself.
static SPAWNED: Mutex<BTreeSet<Proc>> = Mutex::new(BTreeSet::new());

/// Processes proved to descend from a [`SPAWNED`] one.
static ADOPTED: Mutex<BTreeSet<Proc>> = Mutex::new(BTreeSet::new());

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
///
/// ⛔ **Keyed by the bare id on purpose, and [`forget_process`] does not touch it** — task
/// T-13-27. With the registries now holding pairs, the soundness argument above holds a fortiori:
/// entering [`SPAWNED`] or [`ADOPTED`] takes *more* than it used to (the start time has to be
/// readable and has to match), so an answer of "not ours" has even fewer ways to become "ours"
/// than when this cache was written. Keying it by the pair instead would be the one change in
/// this file with the outcome «refused before, allowed now»: a process that inherited a refused
/// id would miss the cache and be re-examined. A cache keyed by the number refuses it outright —
/// the strictly narrower answer, and therefore the one kept.
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

// ---------------------------------------------------------------------------------------
// The identity itself — task T-13-27
// ---------------------------------------------------------------------------------------

/// The instant a process started, as the raw `FILETIME` of `GetProcessTimes`.
///
/// # ⛔ Why every failure here means «not our process»
///
/// `OpenProcess` fails when the process has already exited or when this bench has no rights to
/// it; `GetProcessTimes` fails when the handle dies underneath the call. In **every** one of
/// those cases this returns `None`, and every caller reads `None` as «not ours». The two answers
/// that are forbidden, in as many words:
///
/// * «ours» — it would hand a stranger the gates the module exists to close;
/// * «decide on the id alone, as the code did before this task» — which is the same thing on the
///   one path where it matters, because the path where the times cannot be read is exactly the
///   path where the id may already belong to somebody else.
///
/// Refusing a process the bench really did start is the cost, and it is the acceptable
/// direction: a position fails and the run carries on, which is what requirement B does anyway.
///
/// ⚠ Read through `GetProcessTimes` rather than through the `CreationDate` column of the
/// PowerShell process table used by [`process_table`]: the table costs a child process per read,
/// and this question is asked inside `input::send_verified` before **every** send and inside
/// `shell::activate_window`'s polling. `GetProcessTimes` is two Win32 calls with no child
/// process, no new crate and no new `Cargo.toml` feature — `Win32_System_Threading` and
/// `Win32_Foundation` are already in the closed list of §3.2, which SEC-03 forbids extending.
fn creation_time(pid: u32) -> Option<u64> {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    // SAFETY: `OpenProcess` takes an access mask, an inheritance flag and an id, and returns a
    // handle or an error. `PROCESS_QUERY_LIMITED_INFORMATION` is the least right that answers
    // this question and the one granted across integrity levels for the same user, so an
    // elevated copy of the product is readable too. NFR-13: the result is examined, and its
    // failure is answered with `None` — «not ours», per the note above.
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return None;
    };

    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();

    // SAFETY: `handle` was opened immediately above with exactly the access this call documents,
    // and all four out-parameters are live locals of this frame that outlive the call. NFR-13:
    // the `Result` is examined below — a failure never becomes a start time.
    let read =
        unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) };

    // SAFETY: the handle opened above, closed exactly once, on both the failing and the
    // succeeding path.
    let _ = unsafe { CloseHandle(handle) };

    read.ok()?;

    Some((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
}

/// The identity of a live process, or `None` — which every caller reads as «not ours».
fn identify(pid: u32) -> Option<Proc> {
    Some(Proc {
        pid,
        started: creation_time(pid)?,
    })
}

/// Whether the bench may work with **this identity** — the decision behind [`is_ours`], with the
/// reading of the start time already done.
///
/// Split out from [`is_ours`] so that the acceptance of task T-13-27 is a unit test rather than a
/// live run of the bench, which that task is forbidden to make.
fn is_ours_pair(process: Proc) -> bool {
    SPAWNED.lock().is_ok_and(|set| set.contains(&process))
        || ADOPTED.lock().is_ok_and(|set| set.contains(&process))
}

/// Whether **this identity** was spawned by the bench itself — the decision behind
/// [`was_spawned`], and the one requirement C turns on.
fn was_spawned_pair(process: Proc) -> bool {
    SPAWNED.lock().is_ok_and(|set| set.contains(&process))
}

/// Whether an id names a process the bench spawned **in this run** — the pair, not the number.
fn is_root(roots: &BTreeSet<Proc>, pid: u32) -> bool {
    identify(pid).is_some_and(|process| roots.contains(&process))
}

/// Drops every record that no longer names a live process it can identify.
///
/// ⚠ Only ever **removes**. A record whose process has exited is useless — nothing may be done to
/// a dead process — and keeping it is how the registry grew without bound and carried stale
/// numbers from one position into the next, which is the second half of the audit finding of
/// 2026-08-24. A record whose process is momentarily unreadable is dropped too, and that is the
/// same safe direction [`creation_time`] documents: the bench will then refuse a process it did
/// start, rather than accept one it did not.
fn sweep_dead() {
    if let Ok(mut spawned) = SPAWNED.lock() {
        spawned.retain(|process| identify(process.pid) == Some(*process));
    }
    if let Ok(mut adopted) = ADOPTED.lock() {
        adopted.retain(|process| identify(process.pid) == Some(*process));
    }
}

/// ⛔ Strikes every record bearing this process id out of both registries — task T-13-27.
///
/// Called from `shell::process_is_alive` the instant it answers «this process is gone» and from
/// `shell::terminate` the instant a terminate succeeds. That instant is exactly the one at which
/// the number becomes free for Windows to hand to somebody else, so it is the instant the record
/// has to stop meaning anything.
///
/// **By the bare id, deliberately.** By the time this is called the process is dead, so
/// [`identify`] can no longer produce its pair; and removing *every* record with that number is
/// the strictly wider removal, which is the strictly narrower registry.
///
/// ⛔ [`REFUSED`] is **not** touched, and that is a rule and not an omission. That set is the
/// cache of the answer «not ours»; striking an id out of it would let a later question about the
/// same number be answered «ours», which is the one transition this module forbids. Losing a
/// registry record can only cost a refusal — requirement C, for instance, starts refusing a
/// protected name the moment its record goes, which is the safe direction and is accepted.
pub fn forget_process(pid: u32) {
    if let Ok(mut spawned) = SPAWNED.lock() {
        spawned.retain(|process| process.pid != pid);
    }
    if let Ok(mut adopted) = ADOPTED.lock() {
        adopted.retain(|process| process.pid != pid);
    }
}

/// Records a process the bench has just started — requirement A.
///
/// ⚠ **The only way anything enters the registry as a root.** Requirement A says the registry
/// is filled at launch and nowhere else, and this is the only function that writes to
/// [`SPAWNED`].
///
/// ⛔ The pair is read **here**, at the moment of launch, when the process is certainly alive and
/// certainly the one that was just started. A process whose start time cannot be read does not
/// enter the registry at all: the bench will then refuse to touch its own child, which is the
/// safe direction of [`creation_time`] and costs a failed position, not somebody's session.
pub fn register_spawned(pid: u32) {
    let Some(process) = identify(pid) else {
        return;
    };

    // The registry is not allowed to grow without bound. Every growth pays for a sweep of the
    // records that no longer name a live process — see [`sweep_dead`].
    sweep_dead();

    if let Ok(mut spawned) = SPAWNED.lock() {
        spawned.insert(process);
    }
}

/// Whether the bench may work with this process at all — requirements A, B, D and E.
pub fn is_ours(pid: u32) -> bool {
    identify(pid).is_some_and(is_ours_pair)
}

/// Whether this process was started by the bench itself, as opposed to merely descending from
/// something that was — the distinction requirement C turns on.
fn was_spawned(pid: u32) -> bool {
    identify(pid).is_some_and(was_spawned_pair)
}

/// Requirement C as a decision, apart from where the name and the identity came from.
///
/// `true` means «refuse». Split out for the same reason as [`is_ours_pair`]: it makes the
/// requirement checkable by a unit test instead of by a live run of the bench.
fn protected_name_is_refused(name: &str, spawned_here: bool) -> bool {
    is_protected(name) && !spawned_here
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

    // ⛔ The identity, read once for both gates below. `None` is answered «не наш» and never
    // «наш» and never «по одному PID, как раньше» — see `creation_time`.
    let Some(process) = identify(pid) else {
        return Err(format!(
            "⛔ у процесса {pid} ({name}) не читается время старта — тождество не подтверждается, \
             а неподтверждённое тождество означает «не наш»; ни WM_CLOSE, ни принудительное \
             снятие по нему не отправляются"
        ));
    };

    if !is_ours_pair(process) {
        return Err(format!(
            "⛔ процесс {pid} ({name}) НЕ запускался стендом — реестр пункта A его не содержит; \
             ни WM_CLOSE, ни принудительное снятие по нему не отправляются"
        ));
    }

    // Requirement C, the second belt: a protected name needs more than membership by descent.
    if protected_name_is_refused(&name, was_spawned_pair(process)) {
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
/// On success the process is added to [`ADOPTED`] **with its pair**, so the checks that follow it
/// during the scenario are cheap set lookups rather than repeated walks of the process table, and
/// so a number the window's process later gives up cannot carry the adoption with it.
pub fn claim_window_process(pid: u32) -> Result<(), String> {
    // ⛔ Read once, used for every answer below. A window hands out a process id; this turns the
    // id into an identity, and `None` — the process is already gone, or unreadable — is «not
    // ours», never «ours».
    let identified = identify(pid);

    if identified.is_some_and(is_ours_pair) {
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

    // ⛔ No identity, no adoption. Nothing may enter [`ADOPTED`] under a bare number: a record
    // that named only the id would go on answering «ours» after the id changed hands.
    let Some(process) = identified else {
        return refuse(format!(
            "⛔ у процесса {pid} ({name}), которому принадлежит окно, не читается время старта — \
             тождество не подтверждается, и это означает «не наш». Окно НЕ усыновлено, ни \
             WM_CLOSE, ни принудительное снятие по нему не отправлены (пункт B)"
        ));
    };

    if descends_from_ours(pid) {
        if let Ok(mut adopted) = ADOPTED.lock() {
            adopted.insert(process);
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
///
/// ⚠ A link of the chain counts as a root only when the **pair** matches: the process table
/// hands out a parent id, and an id alone is what this module no longer trusts. A parent that has
/// exited, or whose times cannot be read, is not a root — the walk goes on past it exactly as it
/// went on past a non-matching id before.
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
        if is_root(&roots, parent) {
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
    if is_root(&roots, pid) {
        return Some(pid);
    }

    let table = process_table();
    let mut current = pid;

    for _ in 0..MAX_ANCESTRY {
        let entry = table.get(&current)?;
        if entry.parent == 0 || entry.parent == current {
            return None;
        }
        if is_root(&roots, entry.parent) {
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
///
/// Each record is printed as `pid@время-старта`, because the pid alone is no longer what the
/// bench decides by and a report that showed only the number would hide the half that decides.
pub fn describe() -> String {
    format!(
        "запущены стендом: [{}]; усыновлены как потомки: [{}]",
        render(&SPAWNED),
        render(&ADOPTED)
    )
}

/// One registry, rendered for [`describe`].
fn render(registry: &Mutex<BTreeSet<Proc>>) -> String {
    registry
        .lock()
        .map(|set| {
            set.iter()
                .map(Proc::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registries are process-wide statics and the test harness runs tests in parallel, so
    /// every test below takes this first. Poisoning is stepped over deliberately: a panic in one
    /// test must not turn every other one red and hide what actually broke.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn serially() -> std::sync::MutexGuard<'static, ()> {
        ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn plant(registry: &Mutex<BTreeSet<Proc>>, process: Proc) {
        registry.lock().expect("реестр").insert(process);
    }

    /// **Criterion 6 of task T-13-27, on invented numbers so that both outcomes are exact.**
    ///
    /// One id, two start times a single hundred-nanosecond tick apart: the process the bench
    /// launched, and the stranger Windows handed the number to afterwards. The first is ours,
    /// the second is not, and before this task both were.
    #[test]
    fn a_stranger_that_inherited_the_pid_is_not_ours_and_the_original_still_is() {
        let _serial = serially();

        const PID: u32 = 0xE2E0_0001;
        const OURS_STARTED: u64 = 133_996_800_000_000_000;
        const STRANGER_STARTED: u64 = 133_996_800_000_000_001;

        let ours = Proc {
            pid: PID,
            started: OURS_STARTED,
        };
        let stranger = Proc {
            pid: PID,
            started: STRANGER_STARTED,
        };
        plant(&SPAWNED, ours);

        // Printed, not only asserted: the acceptance of task T-13-27 asks for both outcomes
        // **with the numbers**, and `cargo test -- --nocapture` is where they are read from.
        println!(
            "  PID {PID}: старт {OURS_STARTED} → ours={}; старт {STRANGER_STARTED} → ours={}",
            is_ours_pair(ours),
            is_ours_pair(stranger)
        );

        assert!(
            is_ours_pair(ours),
            "наш процесс, тот же PID {PID}, то же время старта {OURS_STARTED} — ours"
        );
        assert!(
            !is_ours_pair(stranger),
            "чужой процесс, тот же PID {PID}, иное время старта {STRANGER_STARTED} — not ours"
        );
        let rendered = ours.to_string();
        assert!(
            rendered.starts_with(&format!("{PID}@")),
            "реестр печатает обе половины тождества: {rendered}"
        );
        assert_ne!(
            rendered,
            stranger.to_string(),
            "две записи с одним номером различимы и в отчёте"
        );

        forget_process(PID);
    }

    /// **Criterion 7 of task T-13-27 — the direction of refusal.**
    ///
    /// No process bears this id, so `OpenProcess` fails and the start time cannot be read. The
    /// answer has to be «not ours» at every gate — not «ours», and not «decide by the id alone».
    #[test]
    fn a_process_whose_start_time_cannot_be_read_is_never_ours() {
        let _serial = serially();

        const UNREADABLE: u32 = 0xE2E0_0002;

        assert_eq!(
            creation_time(UNREADABLE),
            None,
            "время старта несуществующего процесса не читается"
        );
        assert_eq!(identify(UNREADABLE), None, "и тождество не строится");
        assert!(!is_ours(UNREADABLE), "ворота A, B, D, E — «не наш»");
        assert!(
            !was_spawned(UNREADABLE),
            "пункт C — «не запускался стендом»"
        );

        // The registry is deliberately made to hold the number: even so, the answer is «not
        // ours», because a record is matched by the pair and the pair cannot be built.
        plant(
            &SPAWNED,
            Proc {
                pid: UNREADABLE,
                started: 133_996_800_000_000_000,
            },
        );
        assert!(
            !is_ours(UNREADABLE),
            "номер в реестре не делает процесс своим, когда тождество не подтверждается"
        );

        let refusal = may_touch(UNREADABLE).expect_err("ворота C и D обязаны отказать");
        assert!(
            refusal.contains("не читается время старта"),
            "отказ называет причину: {refusal}"
        );

        forget_process(UNREADABLE);
    }

    /// **Criterion 8 of task T-13-27 — requirement C is not weakened anywhere.**
    ///
    /// A protected name passes only for the very pair in [`SPAWNED`]; membership by descent
    /// ([`ADOPTED`]) never satisfies it, and a stranger holding the same number satisfies it
    /// least of all.
    #[test]
    fn a_protected_name_needs_this_very_pair_in_spawned() {
        let _serial = serially();

        const PID: u32 = 0xE2E0_0003;
        let ours = Proc {
            pid: PID,
            started: 133_996_800_000_000_000,
        };
        let stranger = Proc {
            pid: PID,
            started: 133_996_800_000_000_777,
        };

        // Adopted by descent and nothing more — the case requirement C exists for.
        plant(&ADOPTED, ours);
        assert!(is_ours_pair(ours), "потомок — свой для пунктов A, B, D, E");
        for name in ["explorer", "explorer.exe", "claude", "conhost.exe", "Code"] {
            assert!(
                protected_name_is_refused(name, was_spawned_pair(ours)),
                "{name}: усыновления мало для пункта C"
            );
        }

        // Spawned directly: the answer flips, exactly as it did before this task.
        plant(&SPAWNED, ours);
        assert!(
            !protected_name_is_refused("explorer.exe", was_spawned_pair(ours)),
            "запущенный самим стендом explorer.exe пункт C пропускает — поведение не изменилось"
        );

        // The same number, another start time: not in SPAWNED, so C refuses — and A, B, D, E
        // refuse before it.
        assert!(
            protected_name_is_refused("explorer.exe", was_spawned_pair(stranger)),
            "чужой процесс с тем же PID {PID} пункт C не проходит ни при какой паре"
        );
        assert!(!is_ours_pair(stranger), "и своим он тоже не является");

        // A name off the list is not what C is about, with or without a pair.
        assert!(!protected_name_is_refused("notepad.exe", false));

        forget_process(PID);
    }

    /// **Criterion 9 of task T-13-27 — the record does not outlive the position.**
    #[test]
    fn a_record_struck_out_at_the_close_of_a_position_stops_answering_ours() {
        let _serial = serially();

        const PID: u32 = 0xE2E0_0004;
        let spawned = Proc {
            pid: PID,
            started: 133_996_800_000_000_000,
        };
        let adopted = Proc {
            pid: PID,
            started: 133_996_800_000_000_005,
        };

        plant(&SPAWNED, spawned);
        plant(&ADOPTED, adopted);
        assert!(is_ours_pair(spawned), "до закрытия позиции — свой");
        assert!(was_spawned_pair(spawned), "и запущен самим стендом");

        // What `shell::terminate` and `shell::process_is_alive` call.
        forget_process(PID);

        assert!(!is_ours_pair(spawned), "после закрытия позиции — not ours");
        assert!(
            !is_ours_pair(adopted),
            "вычёркивание по номеру убирает и усыновлённую запись"
        );
        assert!(!is_ours(PID), "повторный вопрос о том же PID — not ours");
        assert!(
            !was_spawned_pair(spawned),
            "пункт C тоже перестал пропускать"
        );
    }

    /// **Ловушка 2 задачи** — striking a record out must not empty the cache of refusals.
    ///
    /// [`REFUSED`] is the answer «not ours», and that answer is never allowed to become «ours».
    #[test]
    fn striking_a_record_out_leaves_the_refusal_cache_alone() {
        let _serial = serially();

        const PID: u32 = 0xE2E0_0005;
        REFUSED
            .lock()
            .expect("кэш отказов")
            .insert(PID, "⛔ отказано ранее".to_owned());

        forget_process(PID);

        assert_eq!(
            REFUSED
                .lock()
                .expect("кэш отказов")
                .get(&PID)
                .map(String::as_str),
            Some("⛔ отказано ранее"),
            "ответ «не наш» пережил вычёркивание записи из реестра"
        );

        REFUSED.lock().expect("кэш отказов").remove(&PID);
    }

    /// **Criterion 6 again, this time against Windows itself**, and the sweep beside it.
    ///
    /// The pair of the live bench process is read through `GetProcessTimes`, matched, then
    /// forged one tick out — and the match breaks. Both outcomes on the same real number. The
    /// dead record planted before [`register_spawned`] shows the registry no longer grows
    /// without bound.
    ///
    /// One test rather than two because both halves need the bench's own id, and two tests
    /// wanting the same id would race each other.
    #[test]
    fn the_pair_is_read_from_windows_and_the_registry_drops_the_dead() {
        let _serial = serially();

        let me = std::process::id();
        let started = creation_time(me).expect("время старта собственного процесса читается");
        assert_ne!(started, 0, "FILETIME создания процесса не бывает нулевым");
        assert_eq!(
            identify(me),
            Some(Proc { pid: me, started }),
            "тождество — это пара, и обе половины прочитаны"
        );

        let dead = Proc {
            pid: 0xE2E0_0006,
            started: 133_996_800_000_000_000,
        };
        plant(&SPAWNED, dead);
        plant(&ADOPTED, dead);

        register_spawned(me);

        println!(
            "  живой процесс: PID {me}, GetProcessTimes.lpCreationTime = {started} \
             ({:#018x}) → is_ours={}",
            started,
            is_ours(me)
        );

        assert!(
            is_ours(me),
            "PID {me}, время старта {started} — ours (запись поставлена самим register_spawned)"
        );
        assert!(
            !is_ours_pair(dead),
            "запись мёртвого процесса не пережила пополнения реестра"
        );

        // The same number, the start time of the process that would inherit it next.
        forget_process(me);
        plant(
            &SPAWNED,
            Proc {
                pid: me,
                started: started + 1,
            },
        );
        println!(
            "  тот же PID {me}, в реестре подменённое время старта {} → is_ours={}",
            started + 1,
            is_ours(me)
        );
        assert!(
            !is_ours(me),
            "PID {me}, время старта {} в реестре против {started} у живого процесса — not ours",
            started + 1
        );

        forget_process(me);
        forget_process(dead.pid);
    }
}
