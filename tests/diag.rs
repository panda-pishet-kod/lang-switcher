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
