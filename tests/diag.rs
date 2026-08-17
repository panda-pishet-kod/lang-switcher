//! Integration tests for task T-06-4 — the ring journal of module `diag`.
//!
//! # What is tested here, and what is a property of the types instead
//!
//! The centre of this module is SEC-01, SEC-07 and NFR-12: no character, no key code and no
//! clipboard content may reach the journal, at any level of detail. **Most of that is not
//! testable, and deliberately so** — it is carried by the types. `diag::record` takes an
//! `Operation` and an `OsCode`, neither of which has a constructor that accepts a character or
//! an arbitrary integer, so a test that "tried to put a symbol in" would not compile, and a
//! test that does not compile is not a test. What is checked here is the one seam where text
//! does arrive — `Operation::from_name`, the narrowing that `app::report_non_critical` calls —
//! and the end product, the rendered dump.
//!
//! # Why the ring is guarded by a mutex in this file
//!
//! The journal is process-wide, as a journal has to be, and `cargo test` runs the tests of one
//! binary on parallel threads. The tests that assert *exact* facts about the ring take
//! [`ring`] first; the ones that only read a value or a path do not need it.
//!
//! The gate is a test harness, not part of the program: `diag::record` itself takes no lock
//! and may not (NFR-04).
//!
//! # `diag::init` is never called from here
//!
//! On purpose, and it is a check in itself. Nothing in this file initialises the journal, and
//! every test below still records and reads entries — which is what "the memory is there
//! before anything asks for it" looks like from the outside. A ring allocated lazily on the
//! first entry would have to allocate on the write path, and the write path may not allocate.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::thread;

use lang_switcher::diag::{self, CAPACITY, Kind, LOG_FILE_NAME, Operation, OsCode};
use lang_switcher::settings;
use windows::Win32::Foundation::ERROR_ACCESS_DENIED;
use windows::core::Error as WinError;

/// Serialises the tests that assert exact facts about the process-wide ring.
static RING: Mutex<()> = Mutex::new(());

/// Takes the gate, ignoring poisoning: a test that failed while holding it has already
/// reported, and the next test's assertions are about the ring, not about that test.
fn ring() -> MutexGuard<'static, ()> {
    RING.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// What a keystroke would look like if one ever reached the funnel — the strings of point 24 of
/// the acceptance criterion, plus a scan code written every way a careless caller might write
/// one.
const AS_IF_TYPED: [&str; 8] = [
    "ghbdtn",
    "привет",
    "ы",
    "scan 0x1E",
    "vk=0x41",
    "Stroke { chars: [1087] }",
    "SendInput(ghbdtn)",
    "buffer: привет",
];

// ---------------------------------------------------------------------------------------
// SEC-01, SEC-07, NFR-12 — the narrowing
// ---------------------------------------------------------------------------------------

/// **Point 10.** A name outside the closed table becomes one value that carries none of it.
///
/// This is the whole mechanism by which the journal is proof against its own callers. Note
/// what is *not* required for it to hold: no caller has to be careful, no setting has to be
/// off, and no build configuration has to be chosen. There is simply nowhere for the text to
/// go once `from_name` has answered.
#[test]
fn a_name_outside_the_table_keeps_none_of_its_text() {
    for typed in AS_IF_TYPED {
        let operation = Operation::from_name(typed);

        assert_eq!(
            operation,
            Operation::UNLISTED,
            "{typed} was recognised as a program event"
        );
        assert_eq!(operation.name(), "(unlisted)");
        assert_eq!(operation.kind(), Kind::Unlisted);
        assert!(
            !operation.name().contains(typed),
            "the name of the entry kept part of {typed}"
        );
    }
}

/// The narrowing must still recognise the names the program really reports, or the journal
/// would be safe and useless at the same time.
///
/// One name from each group, taken from the calls of `app::report_non_critical` that exist in
/// the tree today.
#[test]
fn the_names_the_program_really_reports_are_recognised() {
    let cases = [
        ("CreateMutexW", Kind::Process),
        ("PostMessageW", Kind::Window),
        ("SetWindowsHookExW (watchdog)", Kind::Hook),
        ("ActivateKeyboardLayout", Kind::Layout),
        ("Shell_NotifyIconW(NIM_ADD)", Kind::Tray),
        ("ConnectNamedPipe", Kind::Channel),
    ];

    for (name, kind) in cases {
        let operation = Operation::from_name(name);

        assert_ne!(operation, Operation::UNLISTED, "{name} was not recognised");
        assert_eq!(operation.name(), name);
        assert_eq!(operation.kind(), kind, "{name} landed in the wrong group");
    }
}

/// Every name the program reports, taken out of the sources rather than out of a list.
///
/// The debt task T-08-1 recorded and task T-08-2 paid: the settings dialog reported failures
/// under names that were not in the table, so they landed in the ring as
/// [`Operation::UNLISTED`] — a code with nothing beside it. A list of names in a test would have
/// gone stale the same way, so this reads the sources and checks what it finds there.
///
/// ⚠ **Asserted for `settings.rs`, `tray.rs` and `watchdog.rs`, and that is deliberate.** Task
/// T-08-2 was allowed to add rows for the operations of the settings code and for nothing else,
/// so a gap in another module is *printed* here and left standing rather than quietly filled by a
/// task that has no mandate for it. The scan finds every such gap, which is what makes it worth
/// having. `watchdog.rs` was added by task T-09-1 on the strength of a measurement rather than a
/// hope: the run of this test after the row for `UnregisterDeviceNotification` went into the
/// vocabulary printed eight names for that file and none of them unnamed, so the module can carry
/// the assertion, and from here on a new unnamed report there fails the build instead of being
/// printed and forgotten.
///
/// `guard.rs` is **not** asserted and must not be: its three remaining names each contain a UI
/// Automation symbol, and point 9 of FR-71 forbids such a symbol anywhere under `src\` outside
/// `src\guard.rs` — the rows cannot be added without defeating that sweep. See the note in
/// `src\diag.rs` and the report of task T-08-3.
///
/// Only the literal calls are found, which is the whole of them but one: `app` builds one name
/// with `format!` (`thread::spawn: {error}`), and that one is *meant* to narrow to
/// `UNLISTED` — it carries text that was not chosen at compile time, which is exactly what
/// SEC-07 says must not reach the journal.
#[test]
fn every_operation_name_of_the_settings_code_is_in_the_vocabulary() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    let mut elsewhere = Vec::new();

    for entry in std::fs::read_dir(&sources).expect("the source directory must be readable") {
        let path = entry.expect("a directory entry must be readable").path();

        if path.extension().is_none_or(|kind| kind != "rs") {
            continue;
        }

        let file = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let owned_by_this_task =
            file == "settings.rs" || file == "tray.rs" || file == "watchdog.rs";
        let text = std::fs::read_to_string(&path).expect("a source file must be readable");

        for occurrence in text.split("report_non_critical(\"").skip(1) {
            let Some(name) = occurrence.split('"').next() else {
                continue;
            };

            let operation = Operation::from_name(name);
            checked += 1;

            println!(
                "{file}: {name} -> {} ({})",
                operation.name(),
                operation.kind().name()
            );

            if operation == Operation::UNLISTED {
                assert!(
                    !owned_by_this_task,
                    "{file}: «{name}» reaches the journal without a name of its own"
                );

                elsewhere.push(format!("{file}: {name}"));
                continue;
            }

            assert_eq!(operation.name(), name);
        }
    }

    println!("{checked} operation names found in the sources");

    if elsewhere.is_empty() {
        println!("every one of them has a row in the table");
    } else {
        println!("still unnamed, in modules this task may not touch: {elsewhere:?}");
    }

    assert!(checked > 40, "the scan found suspiciously few call sites");
}

/// **The positive control of the row task T-09-1 added** — the debt task T-08-4 recorded.
///
/// The failure of `UnregisterDeviceNotification` in `watchdog::Drop for DeviceNotice` counted
/// itself and could not name itself, because `src\diag.rs` was closed to the task that wrote it.
/// It now reaches `app::report_non_critical` like every neighbour, and this is what makes that
/// worth anything: **the changed path is a failure path that a healthy run never executes**, so
/// a green suite says nothing at all about it. What can be checked without provoking the failure
/// is the seam the failure would go through — the narrowing of `Operation::from_name`, which is
/// the one place the name could be lost, and the way it is lost is silent. That exact silent
/// failure is the one task T-08-2 caught at the settings dialog.
///
/// The dump is checked too: a row that exists in the table but prints as «(unlisted)» would be
/// the same defect one step further along.
#[test]
fn the_device_notification_withdrawal_reports_under_its_own_name() {
    let name = "UnregisterDeviceNotification";
    let operation = Operation::from_name(name);

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "{name} still has no row in the table — the failure would reach the ring as a bare code"
    );
    assert_eq!(operation.name(), name);
    assert_eq!(
        operation.kind(),
        Kind::Hook,
        "{name} landed in the wrong group"
    );

    let _guard = ring();

    diag::record(operation, OsCode::of(&WinError::from(ERROR_ACCESS_DENIED)));

    let dump = diag::render();

    assert!(dump.contains(name), "the dump does not print {name}");
}

/// The names the settings dialog, autostart and the string tables report under — the rows task
/// T-08-2 added, each in the group it belongs to.
#[test]
fn the_settings_dialog_reports_under_its_own_names() {
    let cases = [
        ("SetDlgItemTextW", Kind::Window),
        ("SetWindowTextW", Kind::Window),
        ("CheckDlgButton", Kind::Window),
        ("CheckRadioButton", Kind::Window),
        ("GetDlgItem", Kind::Window),
        ("EndDialog", Kind::Window),
        ("DialogBoxParamW", Kind::Window),
        ("SetWindowLongPtrW", Kind::Window),
        ("ShellExecuteW", Kind::Window),
        ("RegSetValueExW", Kind::Process),
        ("RegCloseKey", Kind::Process),
        ("FindResourceExW", Kind::Process),
        ("LoadResource", Kind::Process),
    ];

    let _guard = ring();

    for (name, kind) in cases {
        let operation = Operation::from_name(name);

        assert_ne!(
            operation,
            Operation::UNLISTED,
            "{name} still has no row in the table"
        );
        assert_eq!(operation.name(), name);
        assert_eq!(operation.kind(), kind, "{name} landed in the wrong group");

        // And it comes back out of a dump under that name rather than as «(unlisted)».
        diag::record(operation, OsCode::of(&WinError::from(ERROR_ACCESS_DENIED)));
    }

    let dump = diag::render();

    for (name, _) in cases {
        assert!(dump.contains(name), "the dump does not print {name}");
    }

    println!("{dump}");
}

/// **Point 9, the half a test can reach.** An error code comes from an operating system error
/// and from nothing else.
///
/// The other half is that there is no `OsCode::from(u32)` and no public field — which is a
/// property of the source, not of a run, and is shown in the report by the definition itself.
#[test]
fn an_error_code_can_only_come_from_an_operating_system_error() {
    assert_eq!(OsCode::NONE.raw(), 0);
    assert_eq!(OsCode::default(), OsCode::NONE);

    let hresult = ERROR_ACCESS_DENIED.to_hresult();
    let error = WinError::from_hresult(hresult);

    assert_eq!(OsCode::of(&error).raw(), hresult.0);
}

// ---------------------------------------------------------------------------------------
// The ring — section 6.2, section 6.3, NFR-06
// ---------------------------------------------------------------------------------------

/// **Point 14.** Every thread of section 6.1 may record, and no entry is lost when they do it
/// at once.
///
/// The equality is the interesting part. Eight threads take four thousand tickets between them
/// and the counter has moved by exactly four thousand: the `fetch_add` that hands out ordinals
/// gives each caller its own, which is what a ring without a lock has to get right.
#[test]
fn every_thread_may_record_and_no_ticket_is_lost() {
    let _gate = ring();

    const THREADS: usize = 8;
    const EACH: usize = 500;

    let before = diag::recorded();

    thread::scope(|scope| {
        for _ in 0..THREADS {
            scope.spawn(|| {
                for _ in 0..EACH {
                    diag::record(Operation::from_name("PostMessageW"), OsCode::NONE);
                }
            });
        }
    });

    assert_eq!(
        diag::recorded() - before,
        (THREADS * EACH) as u64,
        "an entry was lost between threads"
    );
}

/// **Point 25.** More events than the ring holds: the oldest go, the program does not, and the
/// memory does not move.
///
/// Four rounds of a full ring apiece. After each one the journal holds exactly its capacity —
/// no more, which is the bound, and no fewer, which is that eviction reuses the slot rather
/// than dropping it — every surviving ordinal is inside the last `CAPACITY` issued, and the
/// snapshot is still in order.
#[test]
fn the_oldest_entries_are_evicted_and_the_journal_never_grows() {
    let _gate = ring();

    for round in 1..=4 {
        for _ in 0..CAPACITY {
            diag::record(Operation::from_name("KillTimer"), OsCode::NONE);
        }

        let mark = diag::recorded();
        let events = diag::snapshot();

        assert_eq!(
            events.len(),
            CAPACITY,
            "round {round}: the ring holds {} entries, not its capacity",
            events.len()
        );

        let oldest_possible = mark.saturating_sub(CAPACITY as u64);

        assert!(
            events.iter().all(|event| event.ordinal >= oldest_possible),
            "round {round}: an entry older than the capacity of the ring survived"
        );

        assert!(
            events
                .windows(2)
                .all(|pair| pair[0].ordinal < pair[1].ordinal),
            "round {round}: the snapshot is not in order"
        );
    }
}

/// The ring is there before anything initialises it — points 13 and 12 together.
///
/// `diag::init` is never called in this binary (see the module documentation), yet an entry
/// recorded here is found afterwards. The memory therefore cannot be being allocated on
/// demand, which is the property that lets the write path promise NFR-03.
#[test]
fn the_ring_exists_without_being_initialised() {
    let _gate = ring();

    let before = diag::recorded();

    diag::record(Operation::from_name("DestroyIcon"), OsCode::NONE);

    assert_eq!(diag::recorded(), before + 1);
    assert!(
        diag::snapshot()
            .iter()
            .any(|event| event.ordinal == before && event.operation.name() == "DestroyIcon"),
        "the entry just recorded is not in the journal"
    );
}

/// **NFR-06.** The journal's share of the memory ceiling, in numbers.
#[test]
fn the_journal_costs_a_fixed_and_small_amount_of_memory() {
    assert!(CAPACITY.is_power_of_two(), "the index must be a mask");
    assert_eq!(diag::footprint_bytes(), CAPACITY * 24);

    // Section 5 puts the working set at rest under 8 MB. A tenth of one percent of it.
    assert!(diag::footprint_bytes() < 8 * 1024 * 1024 / 100);
}

// ---------------------------------------------------------------------------------------
// The dump — section 7, section 6.1
// ---------------------------------------------------------------------------------------

/// **Points 16 and 17.** Off by default, and beside the configuration rather than under
/// `%ProgramFiles%`.
#[test]
fn the_journal_is_off_by_default_and_sits_beside_the_configuration() {
    assert!(
        !settings::Config::default().diagnostics.log_enabled,
        "section 7 has the journal off by default"
    );

    let app_data = Path::new(r"C:\Users\Someone\AppData\Roaming");
    let folder = diag::log_dir_in(app_data);
    let file = diag::log_path_in(app_data);

    assert_eq!(folder, app_data.join("Lang_Switcher"));
    assert_eq!(file, folder.join(LOG_FILE_NAME));

    // The same folder section 7 puts `config.toml` in, and reached the same way.
    assert_eq!(
        settings::config_path_in(app_data).parent(),
        Some(folder.as_path()),
        "the journal must live where the configuration lives"
    );

    assert!(
        !file.to_string_lossy().contains("Program Files"),
        "section 7: %ProgramFiles% is not writable by an ordinary user"
    );
}

/// **Point 18.** The public path function the settings dialog of task T-08-1 attaches to.
///
/// It answers a folder, and the folder it answers is the one the file is written into.
#[test]
fn the_folder_the_settings_dialog_will_open_is_the_folder_the_file_goes_to() {
    let Some(folder) = diag::log_dir() else {
        // `APPDATA` is unset, which does not happen in an interactive session. Nothing to
        // check rather than a failure invented out of the environment.
        return;
    };

    let file = diag::log_path().expect("a folder was answered but no file");

    assert_eq!(file.parent(), Some(folder.as_path()));
    assert_eq!(file.file_name(), Some(LOG_FILE_NAME.as_ref()));
    assert!(folder.ends_with("Lang_Switcher"));
}

/// **Points 10 and 24 at the level of a test.** Everything a keystroke could look like is fed
/// to the funnel, and neither the rendered text nor the bytes of the written file contain any
/// of it.
///
/// The file is written into the temporary folder and removed. It is not written into
/// `%APPDATA%` — that is a real person's folder, and a test that wrote there would be doing
/// the thing this module exists to prevent.
#[test]
fn nothing_that_could_have_been_typed_reaches_the_dump() {
    let _gate = ring();

    for typed in AS_IF_TYPED {
        diag::record(Operation::from_name(typed), OsCode::NONE);
    }

    let text = diag::render();

    for typed in AS_IF_TYPED {
        assert!(!text.contains(typed), "the dump contains {typed}");
    }

    // The counters of the earlier tasks are there, which is what makes the dump worth writing.
    for expected in [
        "hook.post_failures",
        "hook.unhook_failures",
        "watchdog.recoveries",
        "layouts.cache_failures",
        "inject.send_mismatches",
        "switch.post_message",
        "switch.attach_activate",
        "switch.text_services",
    ] {
        assert!(text.contains(expected), "the dump is missing {expected}");
    }

    let path = std::env::temp_dir().join("lang_switcher-T-06-4-diag.log");

    diag::write_to(&path).expect("the dump could not be written");

    let written = std::fs::read(&path).expect("the dump could not be read back");

    for typed in AS_IF_TYPED {
        assert!(
            !contains(&written, typed.as_bytes()),
            "the written file contains the UTF-8 bytes of {typed}"
        );

        let utf16: Vec<u8> = typed
            .encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();

        assert!(
            !contains(&written, &utf16),
            "the written file contains the UTF-16 bytes of {typed}"
        );
    }

    std::fs::remove_file(&path).expect("the dump could not be removed");
}

/// A byte-level substring search, because the file is searched as bytes and not as text.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

// -------------------------------------------------------------------------------------
// Task T-08-3 — the seven names task T-08-2 found and had no mandate to add
// -------------------------------------------------------------------------------------

/// Four of the seven rows of the debt, each resolving to itself and landing in its group.
///
/// Task T-08-2 swept the sources, found seven operations reaching the journal as
/// [`Operation::UNLISTED`], printed them and left them standing, because its mandate covered the
/// settings code and nothing else — its report, section 18, problem 2.
///
/// The kinds are the ones that already existed. **No new `Kind` was added**: the task opens
/// `diag.rs` for names, and a variant of an enumeration is not a name.
///
/// ⚠ **Three of the seven are not here**, and the sibling test below is where that is asserted
/// rather than merely stated: the names the password-field probe reports under each contain a UI
/// Automation symbol, and acceptance point 9 of FR-71 forbids such a symbol anywhere under `src\`
/// outside `src\guard.rs`.
#[test]
fn the_names_of_the_t_08_2_debt_that_could_be_added_are_in_the_vocabulary() {
    let cases = [
        ("AddClipboardFormatListener", Kind::Selection),
        ("RemoveClipboardFormatListener", Kind::Selection),
        ("CloseClipboard", Kind::Selection),
        ("CloseHandle(process)", Kind::Process),
    ];

    for (name, kind) in cases {
        let operation = Operation::from_name(name);

        println!(
            "{name} -> {} ({})",
            operation.name(),
            operation.kind().name()
        );

        assert_ne!(
            operation,
            Operation::UNLISTED,
            "«{name}» still reaches the journal as a code with no name"
        );
        assert_eq!(operation.name(), name);
        assert_eq!(operation.kind(), kind, "{name} landed in the wrong group");
    }

    // The two halves of one registration must read as one subject in a dump: the `Add` lives in
    // `app` and the `Remove` in `selection`, and a reader diffing a dump should not have to know
    // that to see they belong together.
    assert_eq!(
        Operation::from_name("AddClipboardFormatListener").kind(),
        Operation::from_name("RemoveClipboardFormatListener").kind()
    );
}

/// **What is still unnamed is exactly three names, and each of them is blocked by FR-71.**
///
/// The sibling test `every_operation_name_of_the_settings_code_is_in_the_vocabulary` prints the
/// gap and asserts it only for the two files task T-08-2 was allowed to touch, which was correct
/// for a task with no mandate elsewhere. This one asserts the whole of `src`, and it is the check
/// that stops the debt coming back: a new `report_non_critical("…")` with no row in the table
/// fails here, in the task that introduces it, instead of being discovered two tasks later.
///
/// ⚠ **Why the expected set is three and not none.** Every one of the three is a name the
/// password-field probe of FR-71 reports under, and every one of them *contains* a UI Automation
/// symbol. Acceptance point 9 of FR-71 — `tests\guard.rs`,
/// `no_ui_automation_name_occurs_anywhere_near_the_hook` — requires that no such symbol occur
/// anywhere under `src\` outside `src\guard.rs`, in code or in prose, so that an edit putting UI
/// Automation on the hook's path has to write one into `hook.rs` first. A row in `diag.rs` would
/// put one there; spelling it in fragments to slip past the sweep would defeat the guard instead
/// of satisfying it. Task T-08-3 therefore left them and asked the question in its report.
///
/// They are matched by shape rather than written out here, for that same reason: this file is
/// under `tests\` and not `src\`, so the sweep does not read it, but repeating the symbols would
/// make the next reader think they are allowed somewhere.
///
/// One further name is absent by design and never reaches this scan: `app` builds a single name
/// with `format!` (`thread::spawn: {error}`), which is *meant* to narrow to
/// [`Operation::UNLISTED`] because it carries text not chosen at compile time — exactly what
/// SEC-07 keeps out of the journal. This is a literal-only scan, so that name is never seen here.
#[test]
fn what_still_reaches_the_journal_unnamed_is_only_what_fr_71_blocks() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0;
    let mut unnamed = Vec::new();

    for entry in std::fs::read_dir(&sources).expect("the source directory must be readable") {
        let path = entry.expect("a directory entry must be readable").path();

        if path.extension().is_none_or(|kind| kind != "rs") {
            continue;
        }

        let file = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let text = std::fs::read_to_string(&path).expect("a source file must be readable");

        for occurrence in text.split("report_non_critical(\"").skip(1) {
            let Some(name) = occurrence.split('"').next() else {
                continue;
            };

            checked += 1;

            if Operation::from_name(name) == Operation::UNLISTED {
                unnamed.push(format!("{file}: {name}"));
            }
        }
    }

    println!("{checked} operation names found in the sources");
    println!("still unnamed: {unnamed:?}");

    assert!(checked > 40, "the scan found suspiciously few call sites");

    // The symbols acceptance point 9 of FR-71 keeps out of every file but `src\guard.rs`, spelled
    // here without their leading letter so that this file does not itself become a place they
    // occur — the sweep reads `src\` only, but a reader should not have to check that.
    let blocked_by_fr_71 = |name: &String| {
        ["UIAutomation", "UIAutomationCore"]
            .iter()
            .any(|symbol| name.contains(symbol))
    };

    let (blocked, rest): (Vec<String>, Vec<String>) =
        unnamed.into_iter().partition(blocked_by_fr_71);

    println!("of those, blocked by acceptance point 9 of FR-71: {blocked:?}");

    assert!(
        rest.is_empty(),
        "these reach the journal as a code with no name and nothing prevents naming them: {rest:?}"
    );
    assert_eq!(
        blocked.len(),
        3,
        "the three names FR-71 blocks are {blocked:?}; if this number changed, the question task \
         T-08-3 put to the controller has been answered one way or the other and this test has to \
         be brought into line with the answer"
    );
}
