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
use lang_switcher::switch::{self, Scope};
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
    ];

    for (name, kind) in cases {
        let operation = Operation::from_name(name);

        assert_ne!(operation, Operation::UNLISTED, "{name} was not recognised");
        assert_eq!(operation.name(), name);
        assert_eq!(operation.kind(), kind, "{name} landed in the wrong group");
    }

    // ⭐ **Task T-41-11, finding Н43: the channel's group is a `testing`-only group now**, so the
    // row that used to stand in the table above travels under the same gate as the thing it
    // names. Asserted rather than dropped: in a build that *has* the channel, its names must
    // still resolve — the repair was about the shipped file, not about the diagnostic.
    #[cfg(feature = "testing")]
    {
        let operation = Operation::from_name("ConnectNamedPipe");

        assert_ne!(
            operation,
            Operation::UNLISTED,
            "with the channel compiled in, its own names must still be recognised"
        );
        assert_eq!(operation.name(), "ConnectNamedPipe");
        assert_eq!(operation.kind(), Kind::Channel);
    }

    // And the far side, which is the finding itself: without the feature the name is not in the
    // vocabulary at all, so it narrows to `UNLISTED` and nothing of it is kept.
    #[cfg(not(feature = "testing"))]
    assert_eq!(
        Operation::from_name("ConnectNamedPipe"),
        Operation::UNLISTED,
        "the shipped vocabulary has no row for the channel — finding Н43"
    );
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

/// **The row task T-13-13 added** — the clamp of the three millisecond fields of section 7.
///
/// A publication that finds `[replacement] inter_event_delay_ms`, `[selection]
/// clipboard_timeout_ms` or `[selection] clipboard_restore_delay_ms` above its ceiling publishes
/// the ceiling and records «configuration field clamped to its ceiling». Without a row of its own
/// that fact would arrive as `Operation::UNLISTED` — a code with nothing beside it — which is
/// precisely the silent failure task T-08-2 was raised to repair, so the seam is worth a test of
/// its own even though the path itself is exercised in `tests\settings.rs`.
///
/// The name is a **fact and no value**: it does not say which field, what the file asked for, what
/// was published or what the ceiling is, and there is no argument through which any of those could
/// travel — the recording side passes `OsCode::NONE` and nothing else (SEC-01, SEC-07). The dump
/// is checked too: a row that exists in the table but prints as «(unlisted)» would be the same
/// defect one step further along.
#[test]
fn the_clamped_configuration_field_reports_under_its_own_name() {
    let name = "configuration field clamped to its ceiling";
    let operation = Operation::from_name(name);

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "{name} has no row in the table — the clamp would reach the ring nameless"
    );
    assert_eq!(operation.name(), name);
    assert_eq!(
        operation.kind(),
        Kind::Process,
        "the value came out of the configuration file, which is state of the process"
    );

    let _guard = ring();

    diag::record(operation, OsCode::NONE);

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

/// **Finding Н118 — task T-39-8, decision 122.2: the scope of FR-51 comes alive in the dump.**
///
/// The reading of the setting of FR-51 — how far a layout switch reaches — was written and called
/// by nobody, so its counter of failed reads could never move and the dump never said which scope
/// the switches were made under. It is read now where the dump is rendered, on the thread of the
/// UI, and printed as a row of its own beside that counter: `per_window` — the Windows default, a
/// switch reaches its own window — or `session`.
#[test]
fn the_dump_says_which_scope_of_fr51_was_in_force() {
    let expected = match switch::scope() {
        Scope::PerWindow => "per_window",
        Scope::Session => "session",
    };

    let text = diag::render();
    let row = text
        .lines()
        .find(|line| line.split_whitespace().next() == Some("switch.scope"))
        .map(str::to_owned);

    assert!(
        row.as_deref()
            .is_some_and(|row| row.split_whitespace().nth(1) == Some(expected)),
        "Н118: the dump has no row naming the scope of FR-51 in force, expected «{expected}»: \
         {row:?}"
    );
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
        "switch.post_rejected",
        // ⭐ Task Т-14-4. `switch.attach_activate` and `switch.text_services` stood here until
        // methods 2 and 3 of FR-50 were struck out of the requirement; this is the row that
        // replaced them — the addendum of FR-52, a switch sent where no verdict can be taken.
        "switch.sent_unconfirmed",
        // ⭐ Task T-41-13, finding Н2: `WM_APP_FLUSH` messages that arrived with nothing waiting.
        // The finding is that a **stream** of forged ones keeps the typing buffer empty and left
        // no trace whatever; premise П7 measured that the two counters already kept
        // (`window_flushes` / `window_flushes_taken`) cannot show it, because a forgery moves
        // neither. ⚠ The number is not a count of forgeries — coalesced messages land in it too —
        // and `src\watchdog.rs` says so where it is declared.
        "watchdog.flushes_without_request",
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
///
/// ⭐ **Answered — решение 117.3, task T-34-4 (Э34).** The three carry neutral names now —
/// `password probe: client / interface / timeout`, named by the step and not by the call — so
/// nothing reaches the journal unnamed any more, and this scan asserts exactly that. The sweep of
/// acceptance point 9 is untouched; `the_three_steps_of_the_password_probe_have_neutral_names`
/// below checks that the new names carry none of what it looks for.
#[test]
fn nothing_reaches_the_journal_unnamed_any_more() {
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

    // ⭐⭐ **Task T-41-11, finding Н43 — the one class of name that is *meant* to be unnamed here,
    // and only in this configuration.**
    //
    // This scan reads **source text**, and source text knows nothing of `#[cfg]`. The seven calls
    // of the debug channel are in `src\control.rs` and in one `testing`-only block of `src\app.rs`
    // — code that is **not compiled** into the shipped program at all — so in a build without the
    // feature their names are rightly absent from the vocabulary and rightly narrow to
    // `UNLISTED`. That is the repair, not a hole: the finding was that these seven strings were in
    // the shipped binary, and taking them out is what makes the scan see them here.
    //
    // They are listed **by name**, so a new unnamed report cannot hide behind the exception, and
    // the far side is asserted too: with the feature compiled in, not one of them may be unnamed.
    #[cfg(not(feature = "testing"))]
    {
        const CHANNEL_NAMES: [&str; 7] = [
            "control::start",
            "CloseHandle(token)",
            "ConnectNamedPipe",
            "DisconnectNamedPipe",
            "write(control channel)",
            "FlushFileBuffers",
            "CloseHandle(pipe)",
        ];

        let unexpected: Vec<&String> = unnamed
            .iter()
            .filter(|entry| {
                !CHANNEL_NAMES
                    .iter()
                    .any(|name| entry.ends_with(&format!(": {name}")))
            })
            .collect();

        println!("unnamed that are not the channel's: {unexpected:?}");

        assert!(
            unexpected.is_empty(),
            "these reach the journal as a code with no name: {unexpected:?} — until task T-34-4 \
             three names of the password-field probe stood here on purpose (acceptance point 9 of \
             FR-71); решение 117.3 gave them neutral names, and a new one is a hole, not a policy"
        );

        // The exception must not be vacuous either: without the feature every one of the seven is
        // expected to be unnamed, and if one of them turned up named the repair would be undone.
        for name in CHANNEL_NAMES {
            assert_eq!(
                Operation::from_name(name),
                Operation::UNLISTED,
                "{name} must not be in the shipped vocabulary — finding Н43"
            );
        }
    }

    #[cfg(feature = "testing")]
    assert!(
        unnamed.is_empty(),
        "these reach the journal as a code with no name: {unnamed:?} — until task T-34-4 three \
         names of the password-field probe stood here on purpose (acceptance point 9 of FR-71); \
         решение 117.3 gave them neutral names, and a new one is a hole, not a policy"
    );
}

/// **Task T-34-4, решение 117.3.** The three steps of the password-field probe are rows of the
/// vocabulary under names that say which step refused — and carry none of the symbols the sweep
/// of acceptance point 9 of FR-71 keeps out of every file but `src\guard.rs`. The symbols are
/// matched by their distinctive tails, spelled without the leading letters, so that this file
/// does not itself become a place they occur.
#[test]
fn the_three_steps_of_the_password_probe_have_neutral_names() {
    let steps = [
        "password probe: client",
        "password probe: interface",
        "password probe: timeout",
    ];

    for step in steps {
        let operation = Operation::from_name(step);

        assert_ne!(
            operation,
            Operation::UNLISTED,
            "«{step}» is not in the vocabulary"
        );
        assert_eq!(operation.name(), step);
        assert_eq!(
            operation.kind(),
            Kind::Process,
            "«{step}» belongs to the process"
        );

        for tail in ["IAutomation", "IsPassword", "FocusedElement"] {
            assert!(
                !step.contains(tail),
                "«{step}» spells a symbol acceptance point 9 of FR-71 keeps out of the journal"
            );
        }
    }
}

// -------------------------------------------------------------------------------------
// Task T-13-6 — the fate of the configuration file has names, and a fate is not a file
// -------------------------------------------------------------------------------------

/// The five rows task T-13-6 added, each resolving to itself and landing in its group.
///
/// They are names of program events rather than of Win32 calls — the shape `journal started`
/// and `clipboard snapshot truncated` already established. `Kind::Process` for all five: the
/// configuration is state of the process, read as it starts and written as it ends, and its two
/// readers are `tray` and this module itself, so a `Kind::Tray` would be wrong for half of them.
#[test]
fn the_fate_of_the_configuration_file_has_names_of_its_own() {
    let names = [
        "configuration file unreadable",
        "configuration file from a newer schema",
        "configuration file quarantined",
        "configuration file quarantine refused",
        "configuration save suppressed",
    ];

    let _gate = ring();

    for name in names {
        let operation = Operation::from_name(name);

        assert_ne!(
            operation,
            Operation::UNLISTED,
            "«{name}» still reaches the journal as a code with no name"
        );
        assert_eq!(operation.name(), name);
        assert_eq!(
            operation.kind(),
            Kind::Process,
            "{name} landed in the wrong group"
        );

        diag::record(operation, OsCode::NONE);
    }

    let dump = diag::render();

    for name in names {
        assert!(dump.contains(name), "the dump does not print {name}");
    }

    println!("{dump}");
}

/// **Task T-55-1, решение 120.4 (б) — the two things the start of the program may do to
/// `HKCU\…\Run` have names of their own.**
///
/// Names of program events and not of Win32 calls, the shape of the five rows above. A refused
/// write is not among them because it already has a name — `RegSetValueExW`, the one the other two
/// writers of the value report under. `Kind::Process`, like every configuration row: the value
/// follows the configuration, and the configuration is state of the process.
#[test]
fn the_start_up_reconciliation_of_autostart_has_names_of_its_own() {
    let names = [
        "autostart registered at start",
        "autostart removed at start",
    ];

    let _gate = ring();

    for name in names {
        let operation = Operation::from_name(name);

        assert_ne!(
            operation,
            Operation::UNLISTED,
            "«{name}» still reaches the journal as a code with no name"
        );
        assert_eq!(operation.name(), name);
        assert_eq!(
            operation.kind(),
            Kind::Process,
            "{name} landed in the wrong group"
        );

        diag::record(operation, OsCode::NONE);
    }

    let dump = diag::render();

    for name in names {
        assert!(dump.contains(name), "the dump does not print {name}");
    }
}

/// **Task T-55-2, решение 120.4 (ж) — a change of autostart the program withheld has a name of its
/// own.** A session on a configuration from a newer schema asks the `Run` key nothing, and the
/// check mark of FR-91 and «Применить» say so in the journal instead of in silence. The shape of
/// «configuration save suppressed», whose neighbour it is, and `Kind::Process` like it.
#[test]
fn a_suppressed_change_of_autostart_has_a_name_of_its_own() {
    let name = "autostart change suppressed";

    let _gate = ring();

    let operation = Operation::from_name(name);

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "«{name}» still reaches the journal as a code with no name"
    );
    assert_eq!(operation.name(), name);
    assert_eq!(
        operation.kind(),
        Kind::Process,
        "{name} landed in the wrong group"
    );

    diag::record(operation, OsCode::NONE);

    assert!(
        diag::render().contains(name),
        "the dump does not print {name}"
    );
}

/// **Task T-55-3, finding Н26 — clearing the read-only attribute of `config.toml` has a name of its
/// own.** The program changed an attribute a person may have set by hand, and a fact like that is
/// worth a line; the shape of the configuration rows, and `Kind::Process` like them.
#[test]
fn clearing_the_read_only_attribute_of_the_configuration_has_a_name_of_its_own() {
    let name = "configuration read-only attribute cleared";

    let _gate = ring();

    let operation = Operation::from_name(name);

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "«{name}» still reaches the journal as a code with no name"
    );
    assert_eq!(operation.name(), name);
    assert_eq!(
        operation.kind(),
        Kind::Process,
        "{name} landed in the wrong group"
    );

    diag::record(operation, OsCode::NONE);

    assert!(
        diag::render().contains(name),
        "the dump does not print {name}"
    );
}

/// **Task T-55-5, finding Т8 — a temporary of an interrupted write that the start could not remove
/// has a name of its own.** Not a failure of the program's work, and still not silence; the shape
/// of the configuration rows, and `Kind::Process` like them.
#[test]
fn a_temporary_the_start_could_not_remove_has_a_name_of_its_own() {
    let name = "configuration temporary not removed";

    let _gate = ring();

    let operation = Operation::from_name(name);

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "«{name}» still reaches the journal as a code with no name"
    );
    assert_eq!(operation.name(), name);
    assert_eq!(
        operation.kind(),
        Kind::Process,
        "{name} landed in the wrong group"
    );

    diag::record(operation, OsCode::NONE);

    assert!(
        diag::render().contains(name),
        "the dump does not print {name}"
    );
}

/// **Task T-55-6, решение 120.1 — a configuration file that could not be read has a name of its
/// own, apart from one that could not be parsed.** `configuration file unreadable` stays the name of
/// the damaged file that is quarantined; this is the file that is there, could not be read twice,
/// and is left alone for the session. `Kind::Process` like every configuration row.
#[test]
fn a_file_that_could_not_be_read_has_a_name_apart_from_a_damaged_one() {
    let name = "configuration file not read";

    let _gate = ring();

    let operation = Operation::from_name(name);

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "«{name}» still reaches the journal as a code with no name"
    );
    assert_eq!(operation.name(), name);
    assert_ne!(
        operation,
        Operation::from_name("configuration file unreadable"),
        "and it is not the name of the damaged file"
    );
    assert_eq!(
        operation.kind(),
        Kind::Process,
        "{name} landed in the wrong group"
    );

    diag::record(operation, OsCode::NONE);

    assert!(
        diag::render().contains(name),
        "the dump does not print {name}"
    );
}

/// **Task T-55-7, решение 120.2 — a field read as its default has a name of its own.** One bad
/// number in `config.toml` costs that number and nothing else, and the journal says that it did —
/// not which field, not what stood there. `Kind::Process` like every configuration row.
#[test]
fn a_field_read_as_its_default_has_a_name_of_its_own() {
    let name = "configuration field read as its default";

    let _gate = ring();

    let operation = Operation::from_name(name);

    assert_ne!(
        operation,
        Operation::UNLISTED,
        "«{name}» still reaches the journal as a code with no name"
    );
    assert_eq!(operation.name(), name);
    assert_eq!(
        operation.kind(),
        Kind::Process,
        "{name} landed in the wrong group"
    );

    diag::record(operation, OsCode::NONE);

    assert!(
        diag::render().contains(name),
        "the dump does not print {name}"
    );
}

/// **SEC-01 and SEC-07 for those five rows: they name a fate, and a fate is not a file.**
///
/// A configuration file can be edited by hand and filled with anything at all, which is the
/// reasoning `ConfigError` is already built on. The danger the new rows have to be proof
/// against is therefore a caller — present or future — that builds a name out of what it has
/// just failed to parse. Everything such a name could be made of is fed to the funnel here: a
/// line of the file, a value out of it, the name of a field this build has no name for, the
/// position a parser stopped at, and a real row with a fragment of the file glued to it. Every
/// one narrows to `(unlisted)` and none of them reaches the dump.
#[test]
fn nothing_that_could_have_been_in_the_configuration_reaches_the_journal() {
    let as_if_from_the_file = [
        "processes = [\"мой-редактор.exe\"]",
        "мой-редактор.exe",
        "something_added_in_schema_99",
        "configuration file is malformed at line 4, column 9",
        "schema_version = 99",
        "# не трогать",
        "configuration file unreadable: [general",
    ];

    let _gate = ring();

    for text in as_if_from_the_file {
        let operation = Operation::from_name(text);

        assert_eq!(
            operation,
            Operation::UNLISTED,
            "«{text}» was recognised as a program event"
        );
        assert_eq!(operation.kind(), Kind::Unlisted);

        diag::record(operation, OsCode::NONE);
    }

    let dump = diag::render();

    for text in as_if_from_the_file {
        assert!(!dump.contains(text), "the dump contains «{text}»");
    }

    // The last case is the one worth spelling out: a name that *starts* with a real row and
    // continues into a fragment of the file is not that row. The table is matched whole.
    assert!(
        !dump.contains("[general"),
        "a fragment of the file reached the dump through a name built around a real row"
    );
}

// -------------------------------------------------------------------------------------
// Task T-34-1 — a refused write neither destroys the previous dump nor loses its code
// -------------------------------------------------------------------------------------

/// Plants the dump of a «previous run» in a folder of its own and holds it open with every
/// sharing right withheld — which is what an editor or an antivirus does to a file it is
/// reading, and the one refusal a test can produce at will.
///
/// Answers the folder, the target and the handle; the handle is what keeps the refusal alive,
/// and it has to be dropped before the file can be read back.
fn a_previous_dump_held_open(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, std::fs::File) {
    use std::os::windows::fs::OpenOptionsExt;

    let folder = std::env::temp_dir().join(format!("lang_switcher-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the temporary folder could not be created");

    let target = folder.join(LOG_FILE_NAME);
    std::fs::write(&target, PREVIOUS_DUMP).expect("the previous dump could not be planted");

    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&target)
        .expect("the previous dump could not be held open");

    (folder, target, held)
}

/// What the previous run left on the disk. Bytes chosen so that the test can tell them from
/// anything the current process would render — the header names a run that never happened.
const PREVIOUS_DUMP: &[u8] =
    b"Lang Switcher diagnostic journal\r\n\r\n(the dump of the previous run)\r\n";

/// Plants the dump of a «previous run» and takes an **exclusive byte-range lock** over it while
/// leaving every sharing right open — the one arrangement under which a writer that opens the
/// target itself gets as far as truncating it and is refused only on the write.
///
/// That is the order that destroys a dump: `CREATE_ALWAYS` succeeds and empties the file, the
/// `WriteFile` that follows answers `ERROR_LOCK_VIOLATION`, and what is left on the disk is
/// zero bytes and a refusal. A full disk produces the same order and cannot be ordered by a
/// test; a lock can. Measured before this test was written (probe of 2026-09-10): a naive
/// writer left the file at length 0.
fn a_previous_dump_locked(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, std::fs::File) {
    let folder = std::env::temp_dir().join(format!("lang_switcher-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the temporary folder could not be created");

    let target = folder.join(LOG_FILE_NAME);
    std::fs::write(&target, PREVIOUS_DUMP).expect("the previous dump could not be planted");

    let held = std::fs::File::open(&target).expect("the previous dump could not be opened");
    held.lock().expect("the previous dump could not be locked");

    (folder, target, held)
}

/// **Task T-34-1 (a), finding Н101.** Whatever happens to a write, the file on the disk is never
/// a partial dump: after a refusal it is the previous dump **byte for byte**, after a success it
/// is a whole new one, and in neither case is a temporary file left beside it.
///
/// The arrangement is [`a_previous_dump_locked`]: a writer that opens the target itself — which
/// is what `fs::write` did before this task — truncates it and is then refused, leaving zero
/// bytes; that is the red «before» of this test, and it is a real one. The repaired write goes
/// to a temporary file and renames it over the target, so the target is either untouched or
/// replaced whole. Which of the two happens under the lock depends on the rename Windows
/// performs (a rename with POSIX semantics goes through an open handle that shares deletion; the
/// older one is refused), so both outcomes are accepted and the outcome is printed — what is
/// asserted is the invariant, not the road.
///
/// **The positive control comes first:** the same lock is shown to let a naive writer truncate
/// a second file to nothing, so that «the bytes are intact» below cannot be true for the wrong
/// reason (a lock that refused the open altogether would prove nothing).
#[test]
fn a_refused_write_leaves_the_previous_dump_intact() {
    let _gate = ring();

    // Positive control: under this lock a writer that opens the target itself destroys it.
    let (control_folder, control, control_lock) = a_previous_dump_locked("T-34-1a-control");
    let naive = std::fs::write(&control, b"what a naive writer would put there");
    drop(control_lock);
    let destroyed = std::fs::read(&control).expect("the control file could not be read back");
    assert!(
        naive.is_err() && destroyed.is_empty(),
        "the control did not reproduce the destructive order (refused: {}, {} bytes left)",
        naive.is_err(),
        destroyed.len()
    );
    std::fs::remove_dir_all(&control_folder).expect("the control folder could not be removed");

    let (folder, target, held) = a_previous_dump_locked("T-34-1a");

    let outcome = diag::write_to(&target);

    drop(held);

    let after = std::fs::read(&target).expect("the dump could not be read back");

    match outcome {
        Err(error) => {
            println!("the write was refused ({error}); the previous dump must be untouched");
            assert_eq!(
                after, PREVIOUS_DUMP,
                "a refused write changed the bytes of the previous dump"
            );
        }
        Ok(()) => {
            println!("the rename went through the lock; the dump must be a whole new one");
            assert!(
                after.starts_with(b"Lang Switcher diagnostic journal")
                    && contains(&after, b"--- events ---"),
                "a write that reported success left a partial dump of {} bytes",
                after.len()
            );
        }
    }

    let left_behind: Vec<String> = std::fs::read_dir(&folder)
        .expect("the temporary folder could not be listed")
        .map(|entry| {
            entry
                .expect("a directory entry must be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name != LOG_FILE_NAME)
        .collect();
    assert!(
        left_behind.is_empty(),
        "a refused write left something beside the dump: {left_behind:?}"
    );

    std::fs::remove_dir_all(&folder).expect("the temporary folder could not be removed");
}

/// **Task T-34-1 (b), finding Н101.** A refused write reaches the ring as
/// `journal write refused` **with the code the system gave** — not with `S_OK`, which is what
/// the base recorded and what made every refusal look like nothing at all.
///
/// The code is checked against the `io::Error` the write answered with: the Win32 number it
/// carries, turned into the same `HRESULT` every other failure of this program is journalled
/// under. That is the one road an `io::Error` may take into an `OsCode` (SEC-07): the number the
/// operating system gave, and none of the error's text.
#[test]
fn a_refused_write_is_recorded_with_the_code_the_system_gave() {
    use windows::core::HRESULT;

    let _gate = ring();

    let (folder, target, held) = a_previous_dump_held_open("T-34-1b");

    let first_new_ordinal = diag::recorded();
    let error =
        diag::write_to(&target).expect_err("the write must be refused while the file is held");

    drop(held);
    std::fs::remove_dir_all(&folder).expect("the temporary folder could not be removed");

    let raw = error
        .raw_os_error()
        .expect("a refused open carries the Win32 code of the refusal");
    let expected = HRESULT::from_win32(u32::try_from(raw).expect("a Win32 code is not negative"));

    let refusal = diag::snapshot()
        .into_iter()
        .filter(|event| event.ordinal >= first_new_ordinal)
        .find(|event| event.operation == Operation::JOURNAL_WRITE_REFUSED)
        .expect("the refusal did not reach the journal");

    assert_ne!(
        refusal.code,
        OsCode::NONE,
        "the refusal was recorded with no code at all"
    );
    assert_eq!(
        refusal.code.raw(),
        expected.0,
        "the refusal carries a code other than the one the system gave"
    );
}

/// **Task T-34-1 (c) — the negative control of the bridge.** An `io::Error` reaches an
/// `OsCode` through its Win32 number and through nothing else: one this program built out of
/// text — the text here is what a careless caller might write a scan code into — carries no
/// number and answers `NONE`, and one that came from the system answers the `HRESULT` of its
/// number, the same one `OsCode::of` answers for a `windows::core::Error` of that failure.
///
/// The second half is a sweep: the bridge is called from `src\diag.rs` and from nowhere else,
/// so that an `io::Error` built somewhere out of input never finds a road into the ring. The
/// sweep counts the sites it finds, and one is the least it may find — a sweep over nothing
/// would pass for the wrong reason.
#[test]
fn an_io_error_reaches_the_journal_by_its_number_only_and_only_from_module_diag() {
    let out_of_text = std::io::Error::other("scan 0x1E");
    assert_eq!(out_of_text.raw_os_error(), None);
    assert_eq!(
        OsCode::of_io(&out_of_text),
        OsCode::NONE,
        "an error carrying text and no system number must answer NONE"
    );

    let number = i32::try_from(ERROR_ACCESS_DENIED.0).expect("a Win32 code fits an i32");
    let from_the_system = std::io::Error::from_raw_os_error(number);
    let through_windows = WinError::from_hresult(ERROR_ACCESS_DENIED.to_hresult());

    assert_eq!(
        OsCode::of_io(&from_the_system),
        OsCode::of(&through_windows)
    );
    assert_eq!(
        OsCode::of_io(&from_the_system).raw(),
        ERROR_ACCESS_DENIED.to_hresult().0,
        "the number must arrive as the HRESULT every other failure is journalled under"
    );

    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sites: Vec<String> = Vec::new();

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

        for (index, line) in text.lines().enumerate() {
            let line = line.trim_start();

            // A call and not the definition, and not a sentence about either. The first
            // redaction of this sweep looked for `of_io(&` and found nothing at all, because the
            // one real call passes a reference it already holds — the positive control below is
            // what caught that.
            if line.starts_with("//") || !line.contains("of_io(") || line.contains("fn of_io(") {
                continue;
            }

            sites.push(format!("{file}:{}", index + 1));
        }
    }

    println!("call sites of OsCode::of_io: {sites:?}");

    assert!(
        !sites.is_empty(),
        "the sweep found no call of OsCode::of_io at all — it is measuring nothing"
    );
    assert!(
        sites.iter().all(|site| site.starts_with("diag.rs:")),
        "OsCode::of_io is called outside module diag: {sites:?}"
    );
}

// -------------------------------------------------------------------------------------
// Task T-34-2 — the journal listens to the session, not to the file on the disk
// -------------------------------------------------------------------------------------

/// A folder of its own holding a configuration file, and the path a dump would go to — one
/// level down, so that «nothing is created» can be checked on the folder as well as the file.
fn a_session_folder(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let folder = std::env::temp_dir().join(format!("lang_switcher-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the temporary folder could not be created");

    let config = folder.join("config.toml");
    let target = folder.join("journal").join(LOG_FILE_NAME);

    (folder, config, target)
}

/// Publishes `enabled` for the length of the closure and puts the previous value back.
fn with_journal(enabled: bool, body: impl FnOnce()) {
    let before = diag::log_enabled();
    diag::set_log_enabled(enabled);
    body();
    diag::set_log_enabled(before);
}

/// **Task T-34-2, finding С47, first branch.** The configuration file on the disk is broken and
/// the session has the journal on: the dump is written.
///
/// Before this task the decision was taken by reading the file again at shutdown, and a file
/// that would not read meant «off» — the journal went silent precisely when a person had
/// switched it on and the file had been damaged since. The session's own published setting
/// is the answer; the disk is not asked.
#[test]
fn a_broken_file_on_the_disk_does_not_silence_a_journal_the_session_has_on() {
    let _gate = ring();

    let (folder, config, target) = a_session_folder("T-34-2a");
    std::fs::write(&config, "[general\nthis is not a configuration file\n")
        .expect("the broken configuration could not be planted");

    let mut written = false;
    with_journal(true, || written = diag::dump_on_shutdown_to(&target));

    assert!(
        written && target.is_file(),
        "the session had the journal on and no dump was written (answered {written}, file: {})",
        target.is_file()
    );

    std::fs::remove_dir_all(&folder).expect("the temporary folder could not be removed");
}

/// **Task T-34-2, finding С47, second branch.** The file on the disk says «on» and the session
/// says «off»: nothing is created — no folder, no file, not an empty one.
///
/// The disk can say «on» while the session says «off» whenever a save has failed or a file was
/// edited by hand after the program started; and the reverse — the session on, the disk still
/// saying the old «off» — is the same defect from the other side. Either way the file is not
/// what the person set in this session, and section 7's «no file nobody asked for» is about the
/// person, not the disk.
#[test]
fn a_journal_the_session_has_off_creates_nothing_whatever_the_file_says() {
    let _gate = ring();

    let (folder, config, target) = a_session_folder("T-34-2b");
    let mut on_disk = settings::Config::default();
    on_disk.diagnostics.log_enabled = true;
    settings::write_to(&config, &on_disk).expect("the configuration could not be planted");

    let mut written = true;
    with_journal(false, || written = diag::dump_on_shutdown_to(&target));

    let journal_folder = target.parent().expect("the target has a folder");
    assert!(
        !written && !target.exists() && !journal_folder.exists(),
        "the session had the journal off and something was created (answered {written}, file: \
         {}, folder: {})",
        target.exists(),
        journal_folder.exists()
    );

    std::fs::remove_dir_all(&folder).expect("the temporary folder could not be removed");
}

// -------------------------------------------------------------------------------------
// Task T-34-3 — the header says when the dump was taken and whether the session went on
// -------------------------------------------------------------------------------------

/// The line of the header that starts with `name`, if there is one.
fn header_line<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find(|line| line.starts_with(name))
}

/// **Task T-34-3, finding С48.** A dump written while the program runs and the dump of the
/// previous run used to be indistinguishable: the header carried the uptime and nothing else.
/// Now it carries the wall clock of the moment the dump was taken — local time, the calendar
/// the person lives in — and whether the session was still going on.
#[test]
fn the_header_of_a_dump_carries_the_wall_clock_and_the_state_of_the_session() {
    let _gate = ring();

    let today = lang_switcher::letters::today()
        .expect("the clock of the machine must answer")
        .to_string();

    let text = diag::render();

    let taken_at = header_line(&text, "journal.taken_at")
        .unwrap_or_else(|| panic!("the header has no `journal.taken_at`:\n{text}"));
    assert!(
        taken_at.contains(&today),
        "`journal.taken_at` does not carry today's date {today}: {taken_at}"
    );

    let session = header_line(&text, "journal.session")
        .unwrap_or_else(|| panic!("the header has no `journal.session`:\n{text}"));
    assert!(
        session.ends_with("continues"),
        "a dump rendered while the program runs must say the session continues: {session}"
    );
}

/// **Task T-34-3, finding С48 — the other half.** The file written while the session runs and
/// the file written at shutdown differ in the one word that matters: `continues` against
/// `ended`. Before this task the two were the same text with a different uptime.
#[test]
fn a_dump_of_the_running_session_differs_from_the_dump_of_shutdown() {
    let _gate = ring();

    let (folder, _config, target) = a_session_folder("T-34-3");
    let live = folder.join("live.log");

    diag::write_to(&live).expect("the live dump could not be written");
    with_journal(true, || {
        assert!(
            diag::dump_on_shutdown_to(&target),
            "the shutdown dump could not be written"
        );
    });

    let live_text = std::fs::read_to_string(&live).expect("the live dump could not be read");
    let final_text = std::fs::read_to_string(&target).expect("the shutdown dump could not be read");

    let live_session = header_line(&live_text, "journal.session").unwrap_or("(absent)");
    let final_session = header_line(&final_text, "journal.session").unwrap_or("(absent)");

    assert!(
        live_session.ends_with("continues"),
        "the dump of the running session says: {live_session}"
    );
    assert!(
        final_session.ends_with("ended"),
        "the dump of shutdown says: {final_session}"
    );

    std::fs::remove_dir_all(&folder).expect("the temporary folder could not be removed");
}

// -------------------------------------------------------------------------------------
// Task T-34-7 — the prose at the dump call tells the truth about the order of shutdown
// -------------------------------------------------------------------------------------

/// **Task T-34-7, finding Н99.** The comment above `diag::dump_on_shutdown()` in `src\app.rs`
/// used to promise that the paths which report into the journal at the very end had run before
/// the dump. They had not: the hook comes off on the input thread and the subscriptions of
/// FR-80 on the watcher thread, both joined on the main thread after the UI thread has already
/// written the file. The order is kept — moving the dump behind `join_all` would put file
/// input-output on the main thread, which section 6.1 forbids — and the sentence is what
/// changed. A sweep over `src\app.rs`, so that the promise cannot quietly come back; the file
/// swept is not this one.
#[test]
fn the_prose_at_the_dump_call_no_longer_promises_the_other_threads_were_heard() {
    let app = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("app.rs"),
    )
    .expect("src\\app.rs must be readable");

    for stale in [
        "report into the journal last",
        "leave their failures in a ring nobody reads",
    ] {
        assert!(
            !app.contains(stale),
            "src\\app.rs still says «{stale}» — the promise of T-06-4 that task T-34-7 withdrew"
        );
    }

    assert!(
        app.contains("lands in the ring **after** the dump"),
        "src\\app.rs must say that what the other two threads record at shutdown lands after \
         the dump"
    );
}

// -------------------------------------------------------------------------------------
// Task T-34-4 — the counters of the password-field guard and of the clipboard are in the dump
// -------------------------------------------------------------------------------------

/// The row of the dump whose name is exactly `name` — the name and then whitespace, so that
/// `guard.exclusions` is not satisfied by `guard.exclusions_refused`.
fn row_named<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find(|line| {
        line.starts_with(name)
            && line[name.len()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
    })
}

/// **Task T-34-4, finding Н95.** Every counter of `guard::Counters` and of
/// `selection::Counters` is a row of the dump, under its own name — fourteen and fifteen of
/// them on the day this was written, and the numbers are not the point.
///
/// The list of names is built by taking each structure apart **field by field, without `..`**:
/// a fifteenth field of `guard::Counters`, or a sixteenth of `selection::Counters`, refuses to
/// compile this test until it has been named here — and the assertion that runs then asks the
/// dump for that name. The other direction is asserted too: the dump carries exactly as many
/// `guard.` rows and `selection.` rows as the structures have fields, so a row printed under a
/// misspelt name is caught as well as a row that is absent. The values are printed and not
/// asserted — the structure and the dump are two snapshots, and the machine may count between
/// them.
#[test]
fn every_counter_of_the_guard_and_of_the_clipboard_is_a_row_of_the_dump() {
    let lang_switcher::guard::Counters {
        focus_changes,
        probes,
        stale_verdicts,
        password_verdicts,
        ordinary_verdicts,
        undetermined_verdicts,
        level1_verdicts,
        level2_verdicts,
        level2_timeouts,
        level3_failures,
        excluded_verdicts,
        exclusions,
        exclusions_refused,
        exclusion_read_retries,
        // Task T-37-3, finding С5: the fifteenth — a verdict refused because its control lost the
        // focus while the probe ran. Not `stale_verdicts`, which is the generation's refusal.
        mismatched_verdicts,
    } = lang_switcher::guard::counters();

    let guard_rows = [
        ("guard.focus_changes", focus_changes),
        ("guard.probes", probes),
        ("guard.stale_verdicts", stale_verdicts),
        ("guard.password_verdicts", password_verdicts),
        ("guard.ordinary_verdicts", ordinary_verdicts),
        ("guard.undetermined_verdicts", undetermined_verdicts),
        ("guard.level1_verdicts", level1_verdicts),
        ("guard.level2_verdicts", level2_verdicts),
        ("guard.level2_timeouts", level2_timeouts),
        ("guard.level3_failures", level3_failures),
        ("guard.excluded_verdicts", excluded_verdicts),
        ("guard.exclusions", exclusions),
        ("guard.exclusions_refused", exclusions_refused),
        ("guard.exclusion_read_retries", exclusion_read_retries),
        ("guard.mismatched_verdicts", mismatched_verdicts),
    ];

    let lang_switcher::selection::Counters {
        opens,
        closes,
        close_failures,
        open_retries,
        open_refusals,
        wrong_thread_refusals,
        updates,
        own_updates,
        foreign_updates,
        truncations,
        format_list_truncations,
        snapshots_saved_nothing,
        refused_formats,
        handle_formats,
        listener_remove_failures,
        restore_skips,
        restore_failures,
    } = lang_switcher::selection::counters();

    let selection_rows = [
        ("selection.opens", opens),
        ("selection.closes", closes),
        ("selection.close_failures", close_failures),
        ("selection.open_retries", open_retries),
        ("selection.open_refusals", open_refusals),
        ("selection.wrong_thread_refusals", wrong_thread_refusals),
        ("selection.updates", updates),
        ("selection.own_updates", own_updates),
        ("selection.foreign_updates", foreign_updates),
        ("selection.truncations", truncations),
        ("selection.format_list_truncations", format_list_truncations),
        ("selection.snapshots_saved_nothing", snapshots_saved_nothing),
        ("selection.refused_formats", refused_formats),
        ("selection.handle_formats", handle_formats),
        (
            "selection.listener_remove_failures",
            listener_remove_failures,
        ),
        ("selection.restore_skips", restore_skips),
        ("selection.restore_failures", restore_failures),
    ];

    let text = diag::render();
    let mut missing: Vec<&str> = Vec::new();

    for (name, value) in guard_rows.iter().chain(selection_rows.iter()) {
        match row_named(&text, name) {
            Some(row) => println!("{row}   (the reader answered {value})"),
            None => missing.push(name),
        }
    }

    assert!(
        missing.is_empty(),
        "rows missing from the dump: {missing:?}"
    );

    let guard_in_dump = text.lines().filter(|l| l.starts_with("guard.")).count();
    let selection_in_dump = text.lines().filter(|l| l.starts_with("selection.")).count();

    assert_eq!(
        (guard_in_dump, selection_in_dump),
        (guard_rows.len(), selection_rows.len()),
        "the dump carries a `guard.` or `selection.` row the structures do not have"
    );
}

/// **Task T-37-1, findings С23 and Н3 — every field of `watchdog::Health` is a row of the dump.**
///
/// The instrument of task T-34-4 above, built for the one structure it did not take apart. Task
/// T-37-1 adds two fields to `Health` — the posts to the input thread that reached no window, one
/// for the flush of a focus change and one for the request to put the hook back — and SPEC gives a
/// counter that is not in the dump no reader at all. The structure is taken apart **field by field,
/// without `..`**, so a fifteenth field refuses to compile this test until it is named here, and
/// the assertion that runs then asks the dump for its row.
///
/// `last_reason` is a word and not a number, so it is carried as the word the dump prints. The
/// other direction is not asserted by counting `watchdog.` rows: the dump prints none that `Health`
/// does not have, but the prefix is not reserved to it the way `guard.` is to `guard::Counters`.
#[test]
fn every_field_of_the_health_of_the_watchdog_is_a_row_of_the_dump() {
    let lang_switcher::watchdog::Health {
        recoveries,
        silent_removals,
        absent_at_check,
        install_failures,
        last_reason,
        last_gap_us,
        max_gap_us,
        liveness_ticks,
        desktop_switches,
        session_changes,
        power_resumes,
        flushes_without_request,
        rehook_posts_lost,
        flush_posts_lost,
    } = lang_switcher::watchdog::health();

    let rows = [
        ("watchdog.recoveries", recoveries.to_string()),
        ("watchdog.silent_removals", silent_removals.to_string()),
        ("watchdog.absent_at_check", absent_at_check.to_string()),
        ("watchdog.install_failures", install_failures.to_string()),
        ("watchdog.last_reason", last_reason.name().to_owned()),
        ("watchdog.last_gap_us", last_gap_us.to_string()),
        ("watchdog.max_gap_us", max_gap_us.to_string()),
        ("watchdog.liveness_ticks", liveness_ticks.to_string()),
        ("watchdog.desktop_switches", desktop_switches.to_string()),
        ("watchdog.session_changes", session_changes.to_string()),
        ("watchdog.power_resumes", power_resumes.to_string()),
        (
            "watchdog.flushes_without_request",
            flushes_without_request.to_string(),
        ),
        ("watchdog.rehook_posts_lost", rehook_posts_lost.to_string()),
        ("watchdog.flush_posts_lost", flush_posts_lost.to_string()),
    ];

    let text = diag::render();
    let mut missing: Vec<&str> = Vec::new();

    for (name, value) in &rows {
        match row_named(&text, name) {
            Some(row) => println!("{row}   (the reader answered {value})"),
            None => missing.push(name),
        }
    }

    assert!(
        missing.is_empty(),
        "rows of watchdog::Health missing from the dump: {missing:?}"
    );
}
