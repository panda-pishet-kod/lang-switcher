//! Integration tests for the configuration half of the `settings` module, task T-01-3.
//!
//! Every test that touches the file system works inside a directory it creates under
//! `%TEMP%` and removes again, including on panic. Nothing here writes into the real
//! `%APPDATA%\Lang_Switcher`: that is the working environment of whoever runs the tests,
//! and the one test that is about the standard path builds the string and stops there.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use lang_switcher::settings::{
    self, CONFIG_FILE_NAME, CURRENT_SCHEMA_VERSION, Config, ConfigError, Language, LayoutMode,
    QUARANTINE_SUFFIX, Quarantined, ReadOutcome, ReplacementMethod, SavePolicy,
};
// Task Т-32-1, FR-101: the section `[letters]` of section 7 lives in `settings` because the
// schema does, and the calendar it is written in and the vocabulary of `thanks` live in their
// owner (§6.2). The tests of the section are here, beside the rest of section 7, and they call
// the owner at its own address.
use lang_switcher::letters;
// Task T-14-3: the drawing library of FR-92а moved out of `settings` into its owner, `theme`
// (§6.2, finding 24 of the audit of 2026-08-24). The tests of it are still here — they are
// tests of the pictures this dialog draws — and they call it at its new address.
use lang_switcher::theme;
use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CLEARTYPE_QUALITY,
    CreateCompatibleDC, CreateDIBSection, CreateFontIndirectW, CreateSolidBrush, DEFAULT_QUALITY,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, FONT_CHARSET, FONT_QUALITY, FW_BOLD, FillRect,
    GetDeviceCaps, GetPixel, GetTextExtentPoint32W, GetTextMetricsW, HBITMAP, HDC, HFONT, LOGFONTW,
    LOGPIXELSY, SelectObject, TEXTMETRICW,
};

/// A directory under `%TEMP%` that removes itself, panic or no panic.
///
/// There is no temporary-directory crate in the dependency list of section 3.2 of SPEC and
/// there will not be one, so the few lines are written out here.
struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "lang_switcher_t013_{}_{}_{label}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(&path).expect("the temporary directory must be creatable");
        Self { path }
    }

    /// Path of `config.toml` inside this directory. The file is not created.
    fn config(&self) -> PathBuf {
        self.path.join(CONFIG_FILE_NAME)
    }

    /// Names of everything currently in the directory, sorted.
    fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.path)
            .expect("the temporary directory must be readable")
            .map(|entry| {
                entry
                    .expect("the directory entry must be readable")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Writes `text` as the configuration file of `dir` and returns its path.
fn write_file(dir: &TestDir, text: &str) -> PathBuf {
    let path = dir.config();
    fs::write(&path, text).expect("the configuration file must be writable");
    path
}

/// A configuration that differs from the defaults in every section.
fn a_thoroughly_customised_config() -> Config {
    let mut config = Config::default();
    config.general.enabled = false;
    config.general.autostart = false;
    config.general.language = Language::En;
    config.general.theme = ThemeSetting::Dark;
    config.hotkey.key = "ScrollLock".to_owned();
    config.layouts.mode = LayoutMode::Cycle;
    config.layouts.pair_source = "0x00000419".to_owned();
    config.layouts.pair_target = "0x00000409".to_owned();
    config.layouts.cycle = vec!["0x00000419".to_owned(), "0x00000409".to_owned()];
    config.replacement.method = ReplacementMethod::Selection;
    config.replacement.inter_event_delay_ms = 7;
    config.selection.enabled = false;
    config.selection.clipboard_timeout_ms = 450;
    config.selection.clipboard_restore_delay_ms = 120;
    config.buffer.capacity = 64;
    config.exclusions.processes = vec!["mstsc.exe".to_owned(), "putty.exe".to_owned()];
    config.diagnostics.log_enabled = true;
    config
}

// Criterion 9. The defaults are the ones printed in section 7 of SPEC, field by field, and
// the `[layouts]` ones are additionally fixed by decision 19 of DECISIONS.md.
#[test]
fn defaults_match_section_7_field_by_field() {
    let config = Config::default();

    // Schema 6 — task Т-32-1, вопрос 101: the section `[letters]` of FR-101 and FR-102. Schema 5
    // — решение 97.3 (Hebrew and Arabic). Schema 4 — task Т-29-1, вопрос 94.1: the twelve locales
    // of решение 93 became a schema of their own, so that a build which knows two of them
    // recognises a file of the twelve by its stamp instead of by failing to parse it. Schema 3 —
    // FR-100 (task Т-21-5) added the section `[feedback]`. Schema 2 was FR-42а moving the default
    // of `[replacement] method` to `auto`; every rung is still on the ladder and every one is
    // asserted by the migration tests below.
    //
    // ⚠ **This is the one place the number is written as a literal**, and it is written twice on
    // purpose: everywhere else in this file a file "of today" is stamped
    // `{CURRENT_SCHEMA_VERSION}`, so that raising the schema costs one edit here and none there.
    assert_eq!(config.schema_version, 6);
    assert_eq!(CURRENT_SCHEMA_VERSION, 6);

    assert!(config.general.enabled);
    assert!(config.general.autostart);
    assert_eq!(config.general.language, Language::Ru);
    assert_eq!(config.general.theme, ThemeSetting::System);

    assert_eq!(config.hotkey.key, "Pause");

    assert_eq!(config.layouts.mode, LayoutMode::Pair);
    assert_eq!(config.layouts.pair_source, "0x00000409");
    assert_eq!(config.layouts.pair_target, "0x00000419");
    assert_eq!(config.layouts.cycle, ["0x00000409", "0x00000419"]);

    assert_eq!(config.replacement.method, ReplacementMethod::Auto);
    assert_eq!(config.replacement.inter_event_delay_ms, 0);

    assert!(config.selection.enabled);
    assert_eq!(config.selection.clipboard_timeout_ms, 300);
    assert_eq!(config.selection.clipboard_restore_delay_ms, 200);

    assert_eq!(config.buffer.capacity, 256);

    assert!(config.exclusions.processes.is_empty());

    assert!(!config.diagnostics.log_enabled);

    // Section `[letters]` — FR-101 and FR-102, task Т-32-1. The feed is **on** (вопрос 101 п. 3)
    // and nothing else has happened yet: no day counted from, no letter shown, no news read.
    assert!(config.letters.feed);
    assert_eq!(config.letters.first_run, None);
    assert!(!config.letters.welcome_shown);
    assert_eq!(config.letters.thanks, letters::Thanks::Pending);
    assert_eq!(config.letters.thanks_due, None);
    assert_eq!(config.letters.last_seen_version, "");
    assert_eq!(config.letters.last_letter, None);
    assert_eq!(config.letters.feed_last_read, None);
    assert_eq!(config.letters.first_feed_letter, None);
    assert_eq!(config.letters.latest_known, "");
    assert!(config.letters.read_ids.is_empty());
    assert!(config.letters.reminders.is_empty());
    assert!(config.letters.first_shown.is_empty());
}

// Criterion 10. Round trip: write, read, get the same thing back.
#[test]
fn round_trip_through_a_file_preserves_every_field() {
    let dir = TestDir::new("round_trip");
    let path = dir.config();
    let written = a_thoroughly_customised_config();

    settings::write_to(&path, &written).expect("writing the configuration must succeed");
    let (read_back, outcome) = settings::read_from(&path).expect("reading it back must succeed");

    assert_eq!(read_back, written);
    assert_eq!(outcome, ReadOutcome::Current);

    // The defaults survive a round trip just as well, sections and all.
    let defaults = Config::default();
    settings::write_to(&path, &defaults).expect("rewriting with the defaults must succeed");
    let (read_defaults, outcome) = settings::read_from(&path).expect("rereading must succeed");
    assert_eq!(read_defaults, defaults);
    assert_eq!(outcome, ReadOutcome::Current);
}

/// The four settings задача Т-23-2 took out of the dialog and left in the file — решение 81
/// п. 2, «Оставить в файле».
///
/// The column is the field's own name as `src\settings.rs` spells it, because that is what
/// the sweep below looks for in the two functions that talk to the window.
const SETTINGS_THE_DIALOG_NO_LONGER_SHOWS: [(&str, &str); 4] = [
    ("replacement.method", "метод замены — FR-42/FR-42а"),
    ("inter_event_delay_ms", "задержка между событиями — FR-44"),
    ("clipboard_timeout_ms", "таймаут буфера обмена — §4.7"),
    (
        "clipboard_restore_delay_ms",
        "задержка восстановления — §4.7",
    ),
];

/// **Т-23-2, решение 81 п. 2** — the four settings that left the dialog survive the dialog.
///
/// The family they joined is `buffer.capacity`, `general.enabled` and `schema_version`: fields
/// FR-92 never showed, which the window must carry through untouched because pressing «ОК»
/// **replaces the file whole**. A field the dialog neither fills nor reads is a field whose
/// value comes out of the file and goes back into it unchanged — and a field the dialog
/// *reads* would come back as whatever an absent control answers, which for a check box is
/// «снят» and for a text field is an empty string.
///
/// Two halves, and both are needed. The value half runs the trip a press of «ОК» makes —
/// hand-written file → `read_from` → `write_to` → `read_from` — over hand-set values none of
/// which is the default, so a field quietly replaced by its default is caught. The shape half
/// is what makes the value half hold in the future: the two functions that talk to the window
/// must not mention these fields at all.
///
/// ⚠ The shape half reads `src\settings.rs` and not this file: a probe that looked for a
/// string in a file that itself contains that string would be measuring itself.
#[test]
fn the_four_settings_the_dialog_no_longer_shows_survive_the_round_trip() {
    let dir = TestDir::new("not_shown_round_trip");

    // Hand-written, the way the user of решение 81 edits it, and not one value is a default:
    // the method is `selection` (default `auto`), the delay 13 (default 0), the timeout 987
    // (default 300), the restore delay 654 (default 200). `[buffer] capacity` and
    // `[general] enabled` ride along as the family this joins.
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [general]\n\
             enabled = false\n\
             \n\
             [replacement]\n\
             method = \"selection\"\n\
             inter_event_delay_ms = 13\n\
             \n\
             [selection]\n\
             clipboard_timeout_ms = 987\n\
             clipboard_restore_delay_ms = 654\n\
             \n\
             [buffer]\n\
             capacity = 64\n"
        ),
    );

    let (config, outcome) = settings::read_from(&path).expect("the hand-written file reads");
    assert_eq!(outcome, ReadOutcome::Current);

    // What «ОК» does: the working configuration — the one the file was read into — is written
    // back whole. Read it again and every hand-set value must still be there.
    settings::write_to(&path, &config).expect("writing the configuration back must succeed");
    let (back, _) = settings::read_from(&path).expect("rereading must succeed");

    assert_eq!(back, config, "the whole configuration survives the press");
    assert_eq!(back.replacement.method, ReplacementMethod::Selection);
    assert_eq!(back.replacement.inter_event_delay_ms, 13);
    assert_eq!(back.selection.clipboard_timeout_ms, 987);
    assert_eq!(back.selection.clipboard_restore_delay_ms, 654);
    // The two older members of the family, so that the trip is the same trip.
    assert!(!back.general.enabled);
    assert_eq!(back.buffer.capacity, 64);

    // The shape half: neither of the two functions that talk to the window names any of the
    // four. `fill_dialog` naming one would put a value on a control that is not there;
    // `read_dialog` naming one would take the answer of a control that is not there and
    // write it into the configuration the press then saves.
    let source = settings_module_source();

    for signature in ["fn fill_dialog(", "fn read_dialog("] {
        let body = function_body(&source, signature);

        for (field, what) in SETTINGS_THE_DIALOG_NO_LONGER_SHOWS {
            assert!(
                !body.contains(field),
                "`{signature}` still names `{field}` — {what}; решение 81 left that setting \
                 in config.toml and took its control off the window"
            );
        }
    }

    // And the switch that stayed is still wired, or the sweep above would pass for a dialog
    // that had lost the selection section altogether.
    assert!(
        function_body(&source, "fn read_dialog(").contains("selection.enabled"),
        "`[selection] enabled` is the one field of its section the dialog still shows"
    );
}

// Criterion 11. An unknown field does not fail the read, and the fields around it are read.
#[test]
fn unknown_fields_are_ignored_and_the_rest_is_read() {
    let dir = TestDir::new("unknown_fields");
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             unknown_top_level = 42\n\
             \n\
             [general]\n\
             enabled = false\n\
             unknown_field = \"whatever this is\"\n\
             \n\
             [buffer]\n\
             capacity = 128\n\
             \n\
             [not_a_section_of_the_schema]\n\
             anything = true\n"
        ),
    );

    let (config, outcome) = settings::read_from(&path).expect("unknown fields must not fail");

    assert_eq!(outcome, ReadOutcome::Current);
    assert!(!config.general.enabled);
    assert_eq!(config.buffer.capacity, 128);
    // Everything the file did not mention is still the default.
    assert!(config.general.autostart);
    assert_eq!(config.general.language, Language::Ru);
    assert_eq!(config.hotkey.key, "Pause");
}

// Criterion 12. A field missing from a section that is present falls back to its default.
#[test]
fn missing_field_falls_back_to_its_default() {
    let dir = TestDir::new("missing_field");
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [selection]\n\
             clipboard_timeout_ms = 500\n"
        ),
    );

    let (config, _) = settings::read_from(&path).expect("a partial section must be readable");

    assert_eq!(config.selection.clipboard_timeout_ms, 500);
    assert!(config.selection.enabled);
    assert_eq!(config.selection.clipboard_restore_delay_ms, 200);
}

// Criterion 13. A section missing entirely is filled with its defaults.
#[test]
fn missing_section_falls_back_to_its_defaults() {
    let dir = TestDir::new("missing_section");
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [general]\n\
             enabled = false\n"
        ),
    );

    let (config, _) = settings::read_from(&path).expect("a file with one section must be readable");

    let defaults = Config::default();
    assert_eq!(config.hotkey, defaults.hotkey);
    assert_eq!(config.layouts, defaults.layouts);
    assert_eq!(config.replacement, defaults.replacement);
    assert_eq!(config.selection, defaults.selection);
    assert_eq!(config.buffer, defaults.buffer);
    assert_eq!(config.exclusions, defaults.exclusions);
    assert_eq!(config.diagnostics, defaults.diagnostics);
    assert!(!config.general.enabled);
}

// Criterion 14. An empty file gives the fully default configuration.
#[test]
fn empty_file_gives_the_default_configuration() {
    let dir = TestDir::new("empty_file");
    let path = write_file(&dir, "");

    let (config, outcome) = settings::read_from(&path).expect("an empty file must be readable");

    // ⚠ **One field of the defaults is not what an empty file gives, since task Т-32-1** —
    // `[letters] welcome_shown`. An empty file is a file, and a file means the program has been
    // here before: the rung `step_5_to_6` marks «Привет» as shown for exactly that reason, so
    // the letter of a first run is not shown to somebody being migrated. The assertion below is
    // therefore «the defaults with that one field moved», and it is written out rather than
    // relaxed, so that any *other* drift still fails here.
    let mut expected = Config::default();
    expected.letters.welcome_shown = true;

    assert_eq!(config, expected);
    // An empty file has no version marker, so it arrives through migration.
    assert_eq!(outcome, ReadOutcome::Migrated { from: 0 });
}

// Criterion 15. A missing file gives the fully default configuration.
//
// ⚠ Task Т-29-3, вопрос 95: one field of those defaults is now chosen rather than fixed —
// `general.language` is the interface language of Windows when this build has it, and `en`
// when it has not. `Config::for_a_first_run` is that configuration, and the criterion is
// asserted against it plus the statement that **only** that one field can differ from
// `Config::default`. What the rule itself does is measured by the staged tests at the foot of
// this file; on this machine the two configurations happen to be identical, and a criterion
// written against `Config::default` would therefore have gone on passing while saying nothing.
#[test]
fn missing_file_gives_the_default_configuration() {
    let dir = TestDir::new("missing_file");
    let path = dir.config();
    assert!(!path.exists(), "the test must start without a file");

    let (config, outcome) =
        settings::read_from(&path).expect("a missing file must not be an error");

    assert_eq!(config, Config::for_a_first_run());

    let mut everything_else = Config::default();
    everything_else.general.language = config.general.language;
    assert_eq!(
        config, everything_else,
        "`general.language` is the one field вопрос 95 may move"
    );

    assert_eq!(outcome, ReadOutcome::NoFile);
    // Reading did not create it either.
    assert!(!path.exists(), "reading must not create the file");
    assert!(dir.entries().is_empty());
}

// Criterion 16. A malformed file does not bring the program down: the caller gets an error
// and a usable configuration.
#[test]
fn malformed_toml_does_not_bring_the_program_down() {
    let dir = TestDir::new("malformed");
    let path = write_file(
        &dir,
        "schema_version = 1\n\
         \n\
         [general\n\
         enabled = = true\n",
    );

    let (config, result) = settings::read_or_default(&path);

    assert_eq!(config, Config::default());
    let error = result.expect_err("a malformed file must be reported");
    assert!(
        matches!(error, ConfigError::Malformed { .. }),
        "expected a malformed-file error, got {error:?}"
    );
    // The file itself is left as it was found.
    assert_eq!(dir.entries(), [CONFIG_FILE_NAME]);
}

// Criterion 17. A value outside an enumerated field's set does not pass silently.
#[test]
fn invalid_enumerated_value_does_not_pass_silently() {
    let dir = TestDir::new("bad_enum");

    for (section, field, value) in [
        ("general", "language", "klingon"),
        ("layouts", "mode", "diagonal"),
        ("replacement", "method", "telepathy"),
    ] {
        let path = write_file(
            &dir,
            &format!("schema_version = 1\n\n[{section}]\n{field} = \"{value}\"\n"),
        );

        let error = settings::read_from(&path)
            .expect_err("an unknown value of an enumerated field must be rejected");
        assert!(
            matches!(error, ConfigError::Malformed { .. }),
            "expected a malformed-file error for {section}.{field}, got {error:?}"
        );

        // And the caller that cannot fail still gets the defaults, not the bad value.
        let (config, result) = settings::read_or_default(&path);
        assert_eq!(config, Config::default());
        assert!(result.is_err());
    }

    // The very same fields with values from the set are accepted, so the rejection above
    // is about the value and not about the field.
    let path = write_file(
        &dir,
        "schema_version = 1\n\
         \n\
         [general]\n\
         language = \"en\"\n\
         \n\
         [layouts]\n\
         mode = \"cycle\"\n\
         \n\
         [replacement]\n\
         method = \"selection\"\n",
    );
    let (config, _) = settings::read_from(&path).expect("values from the set must be accepted");
    assert_eq!(config.general.language, Language::En);
    assert_eq!(config.layouts.mode, LayoutMode::Cycle);
    assert_eq!(config.replacement.method, ReplacementMethod::Selection);
}

// Criterion 18. A file without `schema_version` is the earliest form and is migrated.
#[test]
fn file_without_schema_version_is_migrated_to_the_current_one() {
    let dir = TestDir::new("migrate_up");
    let path = write_file(
        &dir,
        "[general]\n\
         enabled = false\n\
         \n\
         [buffer]\n\
         capacity = 512\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("an unversioned file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 0 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    // Migration carried the settings over rather than resetting them.
    assert!(!config.general.enabled);
    assert_eq!(config.buffer.capacity, 512);
    assert_eq!(config.hotkey.key, "Pause");

    // Reading migrates in memory only; writing back is the caller's decision, and once it
    // is taken the file comes back as a current one.
    settings::write_to(&path, &config).expect("writing the migrated configuration must succeed");
    let (again, outcome) = settings::read_from(&path).expect("rereading must succeed");
    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(again, config);
}

// Criterion 19. A file from a newer schema is recognised as such and is not damaged.
#[test]
fn file_from_a_newer_schema_is_recognised_and_left_intact() {
    let dir = TestDir::new("from_future");
    let original = "schema_version = 99\n\
                    \n\
                    [general]\n\
                    enabled = false\n\
                    \n\
                    [something_added_in_schema_99]\n\
                    setting = \"kept\"\n";
    let path = write_file(&dir, original);

    let (config, outcome) = settings::read_from(&path).expect("a newer file must still be read");

    assert_eq!(outcome, ReadOutcome::FromNewerSchema { version: 99 });
    // The version is not silently rewritten to the current one.
    assert_eq!(config.schema_version, 99);
    assert!(!config.general.enabled);

    // And reading has left the file exactly as it was, byte for byte.
    let on_disk = fs::read_to_string(&path).expect("the file must still be readable");
    assert_eq!(on_disk, original);
    assert_eq!(dir.entries(), [CONFIG_FILE_NAME]);
}

/// **Finding 9 of the audit of 2026-08-31, task T-19-4.** A file from a newer schema is
/// recognised by its stamp, and not by whether the rest of it happens to parse.
///
/// The three enumerated fields of section 7 are closed on purpose — "a value outside it must not
/// pass silently" — so **one new value of an existing field** is all a later schema needs in
/// order to refuse the strict parse of this build. Until this task that refusal came first:
/// `toml::from_str` ran before anybody looked at `schema_version`, the file became
/// [`ConfigError::Malformed`], and [`SavePolicy::for_read`] answered `QuarantineFirst` — a `.bad`
/// copy that keeps exactly one file, and a newer configuration replaced by the defaults of this
/// schema. That is the opposite of what `ReadOutcome::FromNewerSchema` promises in words: "this
/// build is not required to understand such a file, but it is required not to damage it".
///
/// The criterion 19 test above covers added **keys** only, which serde ignores by construction,
/// so it could never have caught this.
///
/// The control below is the other half of the promise, and it must not move: a file of the
/// **current** schema that will not parse is still damaged, and still quarantined.
#[test]
fn a_newer_schema_is_recognised_even_when_this_build_cannot_parse_it() {
    let dir = TestDir::new("from_future_unparsable");

    // A value the current schema has no name for, in a field that is closed.
    let original = "schema_version = 99\n\
                    \n\
                    [general]\n\
                    enabled = false\n\
                    \n\
                    [replacement]\n\
                    method = \"smart\"\n";
    let path = write_file(&dir, original);

    let (config, outcome) = settings::read_or_default(&path);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::FromNewerSchema { version: 99 }),
        "the stamp decides, not the parser"
    );
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::Forbidden);

    // The version is carried, so nothing later can mistake this session for a current one.
    assert_eq!(config.schema_version, 99);

    // Nothing was quarantined and nothing was rewritten: the bytes are exactly as they were.
    assert_eq!(
        fs::read_to_string(&path).expect("the file must still be readable"),
        original
    );
    assert_eq!(dir.entries(), [CONFIG_FILE_NAME]);

    // --- the control: the current schema, and a value it does not admit --------------------
    let current = TestDir::new("current_unparsable");
    let broken = write_file(
        &current,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[replacement]\nmethod = \"smart\"\n"
        ),
    );
    let (_, outcome) = settings::read_or_default(&broken);

    assert!(
        matches!(outcome, Err(ConfigError::Malformed { .. })),
        "a file of this schema that will not parse is damaged, exactly as it was before"
    );
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::QuarantineFirst);
}

// -----------------------------------------------------------------------------------------
// Task T-13-6 — the outcome of the read decides the fate of the file
//
// The pair `read_or_default` answers is only worth returning if somebody acts on it, and the
// finding of the audit of 2026-08-24 was that nobody did. These tests are about the half this
// module owns: the decision a given outcome calls for, and the move that keeps a file this
// build could not read. What the tray then does with the decision is `tests\tray.rs`.
//
// ⚠ Not one test here goes near `%APPDATA%`. Every path is built by `TestDir` under `%TEMP%`
// and handed to the functions explicitly — the same way every test above this line does it —
// and `settings::default_config_path`, the one function in the module that reads `APPDATA` at
// all, is called by no test of this section.
// -----------------------------------------------------------------------------------------

/// Every outcome of a read names the one thing that may be done to the file afterwards.
///
/// The outcomes are taken from real reads of real files rather than built by hand, so that the
/// mapping is checked against what the reader actually answers. The one exception is the
/// input-output failure, which has no portable way to be provoked from a test and is built
/// directly — the arm it exercises is the same one.
#[test]
fn each_read_outcome_names_what_may_be_done_to_the_file() {
    let dir = TestDir::new("save_policy");

    // No file at all — a first run. Nothing on the disk to lose.
    let missing = dir.path.join("not-created.toml");
    let (_, outcome) = settings::read_or_default(&missing);
    assert_eq!(outcome.as_ref().ok(), Some(&ReadOutcome::NoFile));
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::Allowed);

    // A current file: read whole, so writing the memory back loses none of it.
    let current = write_file(
        &dir,
        &format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general]\nenabled = false\n"),
    );
    let (_, outcome) = settings::read_or_default(&current);
    assert_eq!(outcome.as_ref().ok(), Some(&ReadOutcome::Current));
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::Allowed);

    // A migrated file: the ladder understood every rung, so it may be written back — that is
    // what a migration is for, and this is the behaviour the criterion calls "unchanged".
    let old = write_file(&dir, "[general]\nenabled = false\n");
    let (_, outcome) = settings::read_or_default(&old);
    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::Migrated { from: 0 })
    );
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::Allowed);

    // Malformed: the bytes are somebody's text and this build did not understand them.
    let broken = write_file(
        &dir,
        &format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general\nenabled = = true\n"),
    );
    let (_, outcome) = settings::read_or_default(&broken);
    assert!(matches!(outcome, Err(ConfigError::Malformed { .. })));
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::QuarantineFirst);

    // A failure to read at all is treated the same way, and for the same reason: the defaults
    // that came back are not what is on the disk, so the disk must not be overwritten with
    // them until the bytes are safe somewhere.
    let unreadable: Result<ReadOutcome, ConfigError> = Err(ConfigError::Io(std::io::Error::from(
        std::io::ErrorKind::PermissionDenied,
    )));
    assert_eq!(
        SavePolicy::for_read(&unreadable),
        SavePolicy::QuarantineFirst
    );

    // From a newer build: readable, and precisely therefore untouchable.
    let future = write_file(&dir, "schema_version = 99\n\n[general]\nenabled = false\n");
    let (_, outcome) = settings::read_or_default(&future);
    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::FromNewerSchema { version: 99 })
    );
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::Forbidden);
}

/// The kept copy is named after the original and sits in the same folder.
///
/// Same folder because a move is only atomic within one volume, and named after the original
/// because whoever opens the folder of section 7 should be able to see what happened without
/// being told.
#[test]
fn the_kept_copy_is_named_after_the_original_and_sits_beside_it() {
    let dir = TestDir::new("bad_name");
    let path = dir.config();
    let aside = settings::quarantine_path_for(&path);

    assert_eq!(aside.parent(), path.parent());
    assert_eq!(
        aside.file_name().expect("the copy has a name"),
        format!("{CONFIG_FILE_NAME}{QUARANTINE_SUFFIX}").as_str()
    );
    assert_eq!(
        aside.file_name().expect("the copy has a name"),
        "config.toml.bad"
    );
}

/// **The copy is the bytes of the original and not a rendering of the part that parsed.**
///
/// Compared as bytes and never as length: the point of the copy is the text a person typed, so
/// a line ending that changed, an encoding that was normalised or a section that was dropped
/// would all be the loss this exists to prevent, and all three keep the length plausible.
#[test]
fn a_kept_configuration_is_the_original_bytes_and_nothing_else() {
    let dir = TestDir::new("bad_bytes");
    let path = dir.config();

    // CRLF, a comment, non-ASCII in a value, and a section the parser never reaches because
    // of the bracket above it — every one of them a thing a rewrite would lose.
    let original = &format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\r\n\
                    # моя правка\r\n\
                    \r\n\
                    [general\r\n\
                    enabled = = true\r\n\
                    \r\n\
                    [exclusions]\r\n\
                    processes = [\"мой-редактор.exe\"]\r\n"
    );
    fs::write(&path, original).expect("the configuration file must be writable");

    assert_eq!(
        settings::quarantine(&path).expect("the move must succeed"),
        Quarantined::Moved
    );

    let kept = fs::read(settings::quarantine_path_for(&path)).expect("the copy must be readable");

    assert_eq!(
        kept,
        original.as_bytes(),
        "the copy is not byte for byte what the person had"
    );
    assert!(!path.exists(), "a move leaves nothing at the old name");
    assert_eq!(dir.entries(), ["config.toml.bad"]);
}

/// Exactly one copy is kept: a second unreadable file replaces the first.
///
/// A numbered trail of unreadable configurations in somebody's `%APPDATA%` would be a small
/// problem turned into a permanent one.
#[test]
fn only_one_kept_copy_is_ever_left_behind() {
    let dir = TestDir::new("bad_once");
    let path = dir.config();

    fs::write(&path, "first [\n").expect("the file must be writable");
    settings::quarantine(&path).expect("the first move must succeed");

    fs::write(&path, "second [\n").expect("the file must be writable again");
    settings::quarantine(&path).expect("the second move must succeed");

    assert_eq!(dir.entries(), ["config.toml.bad"]);
    assert_eq!(
        fs::read(settings::quarantine_path_for(&path)).expect("the copy must be readable"),
        b"second [\n",
        "the copy must be the file that was there last"
    );
}

/// Nothing to move is not a failure — it is permission to write.
///
/// The file can disappear between the read at start-up and the first save, and there is
/// nothing of anybody's to keep in that case. Source and destination share a directory by
/// construction, so a `NotFound` can only be about the source.
#[test]
fn moving_a_file_that_is_not_there_is_not_a_failure() {
    let dir = TestDir::new("bad_missing");

    assert_eq!(
        settings::quarantine(&dir.config()).expect("a missing file is not an error"),
        Quarantined::NothingThere
    );
    assert!(dir.entries().is_empty(), "and nothing was created");
}

// -----------------------------------------------------------------------------------------
// FR-42а — the migration to schema 2. Task T-10-8.
//
// Up to schema 1 the default of `[replacement] method` was `backspace`, and `write_to` always
// wrote the field out, so `backspace` in an old file is what the absence of a choice looks
// like. `selection` was never a default of any schema, so it is a choice a person made.
// The rung must raise the former and must not touch the latter.
// -----------------------------------------------------------------------------------------

/// A schema 1 file exactly as the previous build wrote it for a user who never chose a
/// method — plus enough customised fields to see that migration is surgery on one field and
/// not a rewrite.
const SCHEMA_1_BACKSPACE_FILE: &str = "schema_version = 1\n\
                                       \n\
                                       [general]\n\
                                       enabled = true\n\
                                       autostart = false\n\
                                       language = \"en\"\n\
                                       \n\
                                       [hotkey]\n\
                                       key = \"ScrollLock\"\n\
                                       \n\
                                       [layouts]\n\
                                       mode = \"cycle\"\n\
                                       cycle = [\"0x00000419\", \"0x00000409\"]\n\
                                       \n\
                                       [replacement]\n\
                                       method = \"backspace\"\n\
                                       inter_event_delay_ms = 7\n\
                                       \n\
                                       [selection]\n\
                                       clipboard_timeout_ms = 450\n\
                                       \n\
                                       [buffer]\n\
                                       capacity = 64\n\
                                       \n\
                                       [exclusions]\n\
                                       processes = [\"mstsc.exe\"]\n\
                                       \n\
                                       [diagnostics]\n\
                                       log_enabled = true\n";

#[test]
fn migration_raises_the_old_default_backspace_to_auto_and_touches_nothing_else() {
    let dir = TestDir::new("migrate_backspace");
    let path = write_file(&dir, SCHEMA_1_BACKSPACE_FILE);

    let (config, outcome) = settings::read_from(&path).expect("a schema 1 file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 1 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);

    // The one field the rung is about: the old default became the new default.
    assert_eq!(config.replacement.method, ReplacementMethod::Auto);

    // Everything else survived, field by field — migration is not a reset.
    assert!(config.general.enabled);
    assert!(!config.general.autostart);
    assert_eq!(config.general.language, Language::En);
    assert_eq!(config.hotkey.key, "ScrollLock");
    assert_eq!(config.layouts.mode, LayoutMode::Cycle);
    assert_eq!(config.layouts.cycle, ["0x00000419", "0x00000409"]);
    assert_eq!(config.replacement.inter_event_delay_ms, 7);
    assert_eq!(config.selection.clipboard_timeout_ms, 450);
    assert_eq!(config.buffer.capacity, 64);
    assert_eq!(config.exclusions.processes, ["mstsc.exe"]);
    assert!(config.diagnostics.log_enabled);

    // Written back — the caller's decision, as always — the file is current and stays put.
    settings::write_to(&path, &config).expect("writing the migrated configuration");
    let (again, outcome) = settings::read_from(&path).expect("rereading");
    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(again, config);
}

#[test]
fn migration_leaves_an_explicit_selection_exactly_as_the_person_chose_it() {
    let dir = TestDir::new("migrate_selection");
    let path = write_file(
        &dir,
        "schema_version = 1\n\
         \n\
         [replacement]\n\
         method = \"selection\"\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("a schema 1 file must be read");

    // The version is raised — the file does travel through the rung — and the choice is not.
    assert_eq!(outcome, ReadOutcome::Migrated { from: 1 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(config.replacement.method, ReplacementMethod::Selection);
}

#[test]
fn an_unversioned_file_with_backspace_climbs_both_rungs_to_auto() {
    let dir = TestDir::new("migrate_unversioned");
    let path = write_file(
        &dir,
        "[replacement]\n\
         method = \"backspace\"\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("an unversioned file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 0 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(config.replacement.method, ReplacementMethod::Auto);
}

#[test]
fn backspace_in_a_current_file_is_an_explicit_override_and_stays() {
    // From schema 2 onwards `backspace` is a manual override, exactly as `selection` always
    // was: only files below the current version travel through the rung, so an override
    // written today is still there tomorrow. Without this property the word would be
    // unusable — every restart would erase it.
    let dir = TestDir::new("current_backspace");
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [replacement]\n\
             method = \"backspace\"\n"
        ),
    );

    let (config, outcome) = settings::read_from(&path).expect("a current file must be read");

    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(config.replacement.method, ReplacementMethod::Backspace);

    // And the round trip keeps it: written back whole, read back the same.
    settings::write_to(&path, &config).expect("writing");
    let (again, outcome) = settings::read_from(&path).expect("rereading");
    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(again.replacement.method, ReplacementMethod::Backspace);
}

#[test]
fn auto_is_the_word_section_7_spells_it() {
    // The value reads and writes as the word of the file format, `auto`, alongside the two
    // that were already there — FR-42а adds a word to the closed set, not a synonym.
    let dir = TestDir::new("auto_word");
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [replacement]\n\
             method = \"auto\"\n"
        ),
    );

    let (config, outcome) = settings::read_from(&path).expect("the word auto must be admitted");
    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(config.replacement.method, ReplacementMethod::Auto);

    let text = config.to_toml_string().expect("serialises");
    assert!(text.contains("method = \"auto\""));
}

// ---------------------------------------------------------------------------------------
// FR-100 — the sound of a press, and the schema 3 it costs. Task Т-21-5, decision 77
// ---------------------------------------------------------------------------------------

/// The user asked for the sound switched **on**, so that is what a fresh configuration says and
/// what the file carries — and the round trip is asserted because a field that reads back as
/// something else is a setting the dialog cannot keep.
#[test]
fn the_sound_of_fr_100_is_on_by_default_and_survives_the_round_trip() {
    assert!(
        Config::default().feedback.sound,
        "FR-100: «по умолчанию включён» — the user asked for it on"
    );

    let dir = TestDir::new("feedback_round_trip");

    for wanted in [false, true] {
        let mut config = Config::default();
        config.feedback.sound = wanted;

        let text = config.to_toml_string().expect("serialises");

        assert!(
            text.contains(&format!("sound = {wanted}")),
            "the file has to carry the field: {text}"
        );

        let path = write_file(&dir, &text);
        let (back, outcome) = settings::read_from(&path).expect("a current file must be read");

        assert_eq!(outcome, ReadOutcome::Current);
        assert_eq!(
            back.feedback.sound, wanted,
            "the round trip kept the switch"
        );
    }
}

/// **The rung of the ladder, and what it deliberately does not do.**
///
/// A schema 2 file was written by a build that had never heard of `[feedback]`, so it carries no
/// section at all — and the serde default of the field is the same `true` a fresh configuration
/// gets. The rung therefore stamps the version and touches nothing: the user asked for the sound
/// on by default, and «on» is what an existing installation gets too.
///
/// The `[general]` line is there to prove the rest of the file survives the rung untouched.
#[test]
fn a_file_of_schema_two_is_raised_to_three_with_the_sound_on() {
    let dir = TestDir::new("feedback_migration");
    let path = write_file(
        &dir,
        "schema_version = 2\n\
         \n\
         [general]\n\
         enabled = false\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("a schema 2 file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 2 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert!(
        config.feedback.sound,
        "an existing installation gets the sound the user asked for"
    );
    assert!(
        !config.general.enabled,
        "and everything the file did carry came through the rung untouched"
    );
}

/// **The rung of task Т-29-1, вопрос 94.1: schema 3 is raised to schema 4, and that is all.**
///
/// A schema 3 file was written by a build whose `Language` held two values and whose section 7
/// was otherwise this one. Nothing in it means anything different under schema 4 — the twelve
/// locales of решение 93 only *added* admissible values to a field that already existed — so the
/// rung stamps the version and touches nothing else. That is the same decision
/// [`step_2_to_3`](../src/settings.rs) took, and for the same reason: a section, or a value, that
/// never existed carries no decision of anybody's to preserve.
///
/// What the raised stamp buys is written down at
/// `a_downgrade_meets_the_new_stamp_and_refuses_to_write_rather_than_quarantining`: from schema 4
/// on, a build that knows two locales recognises a file of the twelve by its **stamp** — before
/// the parser ever refuses it — and so leaves it alone instead of moving it to `.bad`.
///
/// The fields below are deliberately not the defaults, so that damage would show.
#[test]
fn a_file_of_schema_three_is_raised_to_the_current_schema_and_nothing_else_moves() {
    let dir = TestDir::new("locale_schema_migration");
    let path = write_file(
        &dir,
        "schema_version = 3\n\
         \n\
         [general]\n\
         enabled = false\n\
         language = \"en\"\n\
         \n\
         [feedback]\n\
         sound = false\n\
         \n\
         [exclusions]\n\
         processes = [\"мой-редактор.exe\"]\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("a schema 3 file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 3 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    // ⚠ Task Т-30-5 renamed this test: a schema 3 file now climbs **two** rungs, 3 → 4 → 5, and
    // lands on the current schema rather than on four. That the ladder is walked to the top and
    // not one step is the thing worth pinning, and the number the file lands on is asserted by
    // the rung that owns it — `a_file_of_schema_four_is_raised_to_five_and_nothing_else_moves`.
    assert_eq!(
        config.schema_version, 6,
        "and it climbed the whole ladder, not one rung of it"
    );

    assert!(
        !config.general.enabled,
        "the rung carried the switch through"
    );
    assert_eq!(
        config.general.language,
        Language::En,
        "and the locale the person chose, untouched"
    );
    assert!(!config.feedback.sound, "and the sound they turned off");
    assert_eq!(config.exclusions.processes, ["мой-редактор.exe"]);

    // --- the round trip: written back, the file is a current one and reads as one ----------
    let again = write_file(&dir, &config.to_toml_string().expect("serialises"));
    let (back, outcome) = settings::read_from(&again).expect("the raised file must be read");

    assert_eq!(
        outcome,
        ReadOutcome::Current,
        "once written back, the file is of this schema and needs no rung"
    );
    assert_eq!(back, config, "and every field survived the round trip");
}

/// **Т-30-5, решение 97.3: a file of schema four is raised to five and nothing else moves.**
///
/// The same rung as `a_file_of_schema_three_is_raised_to_four_and_nothing_else_moves`, one step
/// up, and for the same reason: вопрос 97 adds **two admissible values** to `general.language`
/// and renames nothing, retypes nothing and changes the meaning of nothing. So the step is a
/// bare stamp, and what has to be shown is that it is *only* a stamp — every field a schema 4
/// file can hold means under schema 5 what it meant before.
///
/// The fields below are deliberately not the defaults, so that damage would show. `el` is one of
/// the ten of решение 93 — a value that schema 4 admits and schema 3 did not — because the rung
/// under test is the one above it and it must carry that value through untouched.
#[test]
fn a_file_of_schema_four_is_raised_to_five_and_nothing_else_moves() {
    let dir = TestDir::new("rtl_schema_migration");
    let path = write_file(
        &dir,
        "schema_version = 4\n\
         \n\
         [general]\n\
         enabled = false\n\
         language = \"el\"\n\
         \n\
         [feedback]\n\
         sound = false\n\
         \n\
         [exclusions]\n\
         processes = [\"мой-редактор.exe\"]\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("a schema 4 file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 4 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    // ⚠ Task Т-32-1: a schema 4 file now climbs **two** rungs, 4 → 5 → 6, and the rung this test
    // owns is still the bare stamp of решение 97.3. What the rung above it does to such a file is
    // asserted by `a_file_of_schema_five_is_raised_to_six_and_the_letters_know_it_is_not_new`.
    assert_eq!(config.schema_version, 6, "and that version is six");

    assert!(
        !config.general.enabled,
        "the rung carried the switch through"
    );
    assert_eq!(
        config.general.language,
        Language::El,
        "and the locale the person chose, untouched"
    );
    assert!(!config.feedback.sound, "and the sound they turned off");
    assert_eq!(config.exclusions.processes, ["мой-редактор.exe"]);

    // --- the round trip: written back, the file is a current one and reads as one ----------
    let again = write_file(&dir, &config.to_toml_string().expect("serialises"));
    let (back, outcome) = settings::read_from(&again).expect("the raised file must be read");

    assert_eq!(
        outcome,
        ReadOutcome::Current,
        "once written back, the file is of this schema and needs no rung"
    );
    assert_eq!(back, config, "and every field survived the round trip");
}

/// **Т-32-1, вопрос 101: a file of schema five is raised to six, and the raised file knows it is
/// not a new installation.**
///
/// The first rung since schema 2 that is **not** a bare stamp, so this test is about what it
/// carries rather than about what it leaves alone — although it checks that too. Three things:
///
/// 1. `welcome_shown` comes out **true**. The file exists, so the program has been here before,
///    so «Привет» — the letter of a first run — must not be shown. This is the whole reason the
///    rung has a body.
/// 2. `last_seen_version` comes out **empty**, which differs from every build's version and is
///    what makes «Что нового» appear exactly once for somebody who has just updated.
/// 3. `first_run` stays **empty**. `Config::migrate` reads no clock, by design; the day is
///    written by `letters::initialise` on the first run after the migration, which is the same
///    day. A test of the ladder must not depend on what day it is run.
///
/// The other fields are deliberately not the defaults, so that damage would show.
#[test]
fn a_file_of_schema_five_is_raised_to_six_and_the_letters_know_it_is_not_new() {
    let dir = TestDir::new("letters_schema_migration");
    let path = write_file(
        &dir,
        "schema_version = 5\n\
         \n\
         [general]\n\
         enabled = false\n\
         language = \"he\"\n\
         \n\
         [feedback]\n\
         sound = false\n\
         \n\
         [exclusions]\n\
         processes = [\"мой-редактор.exe\"]\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("a schema 5 file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 5 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(config.schema_version, 6, "and that version is six");

    // What the rung carries — the three decisions of the rung, in order.
    assert!(
        config.letters.welcome_shown,
        "a machine being updated has been used: «Привет» must not be shown to it"
    );
    assert_eq!(
        config.letters.last_seen_version, "",
        "and «Что нового» must be shown to it exactly once"
    );
    assert_eq!(
        config.letters.first_run, None,
        "the day is written on the first run after the migration, not by the pure ladder"
    );

    // What the rung leaves alone — the rest of `[letters]`, at its defaults, and every field of
    // the file the person actually chose.
    assert!(
        config.letters.feed,
        "the feed is on by default (вопрос 101)"
    );
    assert_eq!(config.letters.thanks, letters::Thanks::Pending);
    assert_eq!(config.letters.thanks_due, None);
    assert!(config.letters.read_ids.is_empty());

    assert!(
        !config.general.enabled,
        "the rung carried the switch through"
    );
    assert_eq!(
        config.general.language,
        Language::He,
        "and the locale the person chose, untouched"
    );
    assert!(!config.feedback.sound, "and the sound they turned off");
    assert_eq!(config.exclusions.processes, ["мой-редактор.exe"]);

    // --- the round trip: written back, the file is a current one and reads as one ----------
    let again = write_file(&dir, &config.to_toml_string().expect("serialises"));
    let (back, outcome) = settings::read_from(&again).expect("the raised file must be read");

    assert_eq!(
        outcome,
        ReadOutcome::Current,
        "once written back, the file is of this schema and needs no rung"
    );
    assert_eq!(back, config, "and every field survived the round trip");
}

/// **Т-32-1: every field of `[letters]` survives a round trip through the file, dates and maps
/// included.**
///
/// The section holds three kinds of value the rest of section 7 does not — a date, a list of
/// numbers and two maps keyed by number — and each of them has a way of not coming back. A date
/// is written as a bare TOML date and has to be read as one; an empty map is left out of the
/// file altogether and has to come back empty rather than missing; a map key is text on the disk
/// and a number in the program.
#[test]
fn every_field_of_the_letters_section_survives_the_file() {
    let dir = TestDir::new("letters_round_trip");
    let path = dir.config();

    let mut written = Config::default();
    written.letters.feed = false;
    written.letters.first_run = letters::Date::from_ymd(2026, 9, 3);
    written.letters.welcome_shown = true;
    written.letters.thanks = letters::Thanks::Snoozed;
    written.letters.thanks_due = letters::Date::from_ymd(2026, 10, 3);
    written.letters.last_seen_version = "0.39.0".to_owned();
    written.letters.last_letter = letters::Date::from_ymd(2026, 9, 30);
    written.letters.feed_last_read = letters::Date::from_ymd(2026, 9, 20);
    written.letters.first_feed_letter = letters::Date::from_ymd(2026, 9, 21);
    written.letters.latest_known = "0.40.0".to_owned();
    written.letters.read_ids = vec![12, 13];
    written.letters.count_reminder(14);
    written
        .letters
        .set_first_shown(14, letters::Date::from_ymd(2026, 9, 25).expect("a date"));

    settings::write_to(&path, &written).expect("writing the configuration must succeed");

    let text = fs::read_to_string(&path).expect("the file must be readable as text");
    println!("--- the file as written ---\n{text}");

    // The dates are **bare** TOML dates, exactly as section 7 prints them — not quoted strings.
    assert!(
        text.contains("first_run = 2026-09-03"),
        "a date must be written as a TOML date:\n{text}"
    );
    assert!(
        !text.contains("\"2026-09-03\""),
        "and never as a quoted string:\n{text}"
    );

    let (read_back, outcome) = settings::read_from(&path).expect("reading it back must succeed");

    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(read_back, written, "every field of `[letters]` came back");
    assert_eq!(read_back.letters.reminders_sent(14), 1);
    assert_eq!(
        read_back.letters.first_shown_on(14),
        letters::Date::from_ymd(2026, 9, 25)
    );
}

/// **Т-32-1: a date written by a person's own hand — quoted — is read rather than refused.**
///
/// `[letters] feed` is a field section 7 tells people to edit for the first ninety days
/// (FR-102), so this section is one people open. A quoted date is what a person writes; refusing
/// the document over it would move the whole configuration to `.bad` and take every other
/// setting with it. The value this program *writes* is still the bare form — the test above
/// pins that — and this is only about what it will **read**.
#[test]
fn a_date_written_by_hand_in_quotes_is_read_as_a_date() {
    let dir = TestDir::new("letters_quoted_date");
    let path = write_file(
        &dir,
        "schema_version = 6\n\
         \n\
         [letters]\n\
         feed = false\n\
         first_run = \"2026-09-01\"\n\
         welcome_shown = true\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("a quoted date must be readable");

    assert_eq!(outcome, ReadOutcome::Current);
    assert!(!config.letters.feed);
    assert_eq!(
        config.letters.first_run,
        letters::Date::from_ymd(2026, 9, 1)
    );
}

/// **Task T-19-4 through the bump: the file of a schema this build does not know is refused.**
///
/// Written against `CURRENT_SCHEMA_VERSION + 1` rather than a fixed number, because that is the
/// case the bump creates in the world: the build that goes out with schema 4 writes files an
/// installed schema 3 build will meet. Its answer must be [`SavePolicy::Forbidden`] — read what
/// can be read, and **never write**, or the fields it does not understand are gone.
///
/// ⚠ Task Т-29-1 added the second half: **the file is still whole afterwards**. A policy that is
/// right in memory and a file that has been moved to `.bad` all the same would satisfy every
/// assertion this test used to make.
#[test]
fn a_file_of_the_next_schema_is_forbidden_to_be_written_back() {
    let dir = TestDir::new("feedback_downgrade");
    let newer = CURRENT_SCHEMA_VERSION + 1;
    let original = format!(
        "schema_version = {newer}\n\
         \n\
         [general]\n\
         enabled = false\n"
    );
    let path = write_file(&dir, &original);

    let (config, outcome) = settings::read_or_default(&path);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::FromNewerSchema { version: newer }),
        "the stamp decides, not the parser"
    );
    assert_eq!(config.schema_version, newer, "the stamp is left as it was");
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::Forbidden);

    // Task Т-29-1: and the bytes are exactly where they were, with no `.bad` beside them.
    assert_eq!(
        fs::read_to_string(&path).expect("the file must still be readable"),
        original,
        "a file from the future is read and never touched"
    );
    assert_eq!(dir.entries(), [CONFIG_FILE_NAME]);
}

// Criterion 20. The write is atomic, and no temporary file is left behind by it.
#[test]
fn successful_write_leaves_no_temporary_file() {
    let dir = TestDir::new("atomic_write");
    let nested = dir.path.join("created").join("on").join("demand");
    let path = nested.join(CONFIG_FILE_NAME);

    // The directory is created if it is not there, which is what happens on a first run.
    assert!(!nested.exists());
    settings::write_to(&path, &Config::default()).expect("the first write must succeed");
    assert!(path.exists());

    let entries: Vec<String> = fs::read_dir(&nested)
        .expect("the directory must be readable")
        .map(|entry| {
            entry
                .expect("the directory entry must be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        entries,
        [CONFIG_FILE_NAME],
        "a successful write must leave the configuration file and nothing else"
    );

    // Overwriting an existing file leaves the same single file behind.
    settings::write_to(&path, &a_thoroughly_customised_config()).expect("rewriting must succeed");
    let entries: Vec<String> = fs::read_dir(&nested)
        .expect("the directory must be readable")
        .map(|entry| {
            entry
                .expect("the directory entry must be readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(entries, [CONFIG_FILE_NAME]);
    let (config, _) = settings::read_from(&path).expect("the rewritten file must be readable");
    assert_eq!(config, a_thoroughly_customised_config());
}

// Criterion 21. The standard path is %APPDATA%\Lang_Switcher\config.toml. Checked by
// building the string: no file and no directory is created anywhere near the real
// %APPDATA%, which belongs to whoever is running the tests.
#[test]
fn default_path_is_appdata_lang_switcher_config_toml() {
    let built = settings::config_path_in(Path::new(r"X:\example\AppData\Roaming"));
    assert_eq!(
        built,
        Path::new(r"X:\example\AppData\Roaming\Lang_Switcher\config.toml")
    );
    assert!(!built.exists(), "the check must not create anything");

    let app_data = std::env::var_os("APPDATA").expect("APPDATA must be set in a user session");
    let expected = Path::new(&app_data)
        .join("Lang_Switcher")
        .join("config.toml");
    assert_eq!(
        settings::default_config_path().expect("APPDATA is set, so a path must be built"),
        expected
    );
}

// Section 6.3. The configuration will be published to the input thread by cloning it whole
// and swapping an atomic pointer, with no mutex in the hook path (NFR-04). This task does
// not implement the publication; it must not make it impossible, and the bounds below are
// what that comes down to.
#[test]
fn configuration_can_be_cloned_whole_and_published_through_a_pointer() {
    fn publishable<T: Clone + Send + Sync + 'static>() {}
    publishable::<Config>();

    let config = a_thoroughly_customised_config();
    let published = Arc::new(config.clone());
    let handle = std::thread::spawn(move || published.buffer.capacity);
    assert_eq!(
        handle.join().expect("the reader thread must not panic"),
        config.buffer.capacity
    );
}

// SEC-01 and SEC-07. A damaged configuration file can contain anything at all, so the text
// it holds must not reach the error the caller gets, in any of the error's forms.
#[test]
fn error_from_a_damaged_file_never_quotes_its_content() {
    let dir = TestDir::new("no_leak");
    let secret = "ghbdtn-this-must-never-be-quoted";
    let path = write_file(
        &dir,
        &format!("schema_version = 1\n\n[general]\nenabled = \"{secret}\"\n"),
    );

    let (config, result) = settings::read_or_default(&path);
    assert_eq!(config, Config::default());
    let error = result.expect_err("a value of the wrong type must be reported");

    let displayed = error.to_string();
    let debugged = format!("{error:?}");
    assert!(
        !displayed.contains(secret),
        "the error text quoted the file: {displayed}"
    );
    assert!(
        !debugged.contains(secret),
        "the error's Debug quoted the file: {debugged}"
    );
    // The position is still reported, which is what makes the error actionable.
    assert!(
        matches!(error, ConfigError::Malformed { at: Some(_) }),
        "expected a positioned malformed-file error, got {error:?}"
    );
}

// A written file carries every section of section 7, so nothing the writer did not touch
// is dropped on the way out.
#[test]
fn written_file_carries_every_section_of_section_7() {
    let text = Config::default()
        .to_toml_string()
        .expect("the defaults must serialise");

    assert!(text.contains(&format!("schema_version = {CURRENT_SCHEMA_VERSION}")));
    assert!(text.contains("method = \"auto\""));
    for section in [
        "[general]",
        "[hotkey]",
        "[layouts]",
        "[replacement]",
        "[selection]",
        "[buffer]",
        "[exclusions]",
        // FR-100, task Т-21-5 — the section schema 3 added.
        "[feedback]",
        "[diagnostics]",
    ] {
        assert!(text.contains(section), "section {section} was not written");
    }

    // SEC-01: the schema has no place to put anything typed, and the written file shows it.
    let (parsed, outcome) = Config::from_toml_str(&text).expect("the written file must parse");
    assert_eq!(parsed, Config::default());
    assert_eq!(outcome, ReadOutcome::Current);
}

// =========================================================================================
// FR-92а — `[general].theme`, the file half of the setting. Task T-11-2.
// =========================================================================================
//
// The three words of the value and the rule about every other word live in
// `theme::ThemeSetting` and are tested with `tests\theme.rs`; what is tested here is that
// the configuration carries the setting through the file — and that the file rules of
// section 7 hold for this key the way they hold for its neighbours, with the one exception
// FR-92а itself makes: an unknown word is not a malformed file but the default.

use lang_switcher::theme::ThemeSetting;

// Criterion 9 of T-11-2. Each of the three words of section 7 reads as its setting.
#[test]
fn each_of_the_three_theme_words_reads_as_its_setting() {
    let dir = TestDir::new("theme_words");

    for (word, setting) in [
        ("system", ThemeSetting::System),
        ("light", ThemeSetting::Light),
        ("dark", ThemeSetting::Dark),
    ] {
        let path = write_file(
            &dir,
            &format!(
                "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general]\ntheme = \"{word}\"\n"
            ),
        );
        let (config, outcome) =
            settings::read_from(&path).expect("a theme word of section 7 must be readable");
        assert_eq!(config.general.theme, setting, "for the word {word:?}");
        assert_eq!(outcome, ReadOutcome::Current);
    }
}

// Criterion 10 of T-11-2. A missing key is the default, and an unknown word is the default
// too — silently, with the file read whole. This is the one asymmetry with `language` and
// its kind (criterion 17 above): FR-92а spells the rule out for this value, and
// `from_config_str` is built so that every string is an answer.
#[test]
fn a_missing_or_unknown_theme_reads_as_system_and_fails_nothing() {
    let dir = TestDir::new("theme_default");

    // No `theme` key at all: the theme is the default, the field beside it is read.
    let path = write_file(
        &dir,
        &format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general]\nenabled = false\n"),
    );
    let (config, outcome) =
        settings::read_from(&path).expect("a file without the key must be readable");
    assert_eq!(config.general.theme, ThemeSetting::System);
    assert!(!config.general.enabled);
    assert_eq!(outcome, ReadOutcome::Current);

    // An unknown word: no error, the value is the default, and the field beside it still
    // arrives — so it was the word that was ignored, not the file.
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\n\
             [general]\nenabled = false\ntheme = \"midnight\"\n"
        ),
    );
    let (config, outcome) =
        settings::read_from(&path).expect("an unknown theme word must not fail the file");
    assert_eq!(config.general.theme, ThemeSetting::System);
    assert!(!config.general.enabled);
    assert_eq!(outcome, ReadOutcome::Current);
}

// Criterion 11 of T-11-2. Write → read brings each of the three settings back.
#[test]
fn every_theme_setting_survives_a_write_and_a_read() {
    let dir = TestDir::new("theme_round_trip");
    let path = dir.config();

    for setting in [
        ThemeSetting::System,
        ThemeSetting::Light,
        ThemeSetting::Dark,
    ] {
        let mut written = Config::default();
        written.general.theme = setting;

        settings::write_to(&path, &written).expect("writing the configuration must succeed");
        let (read_back, outcome) = settings::read_from(&path).expect("reading back must succeed");

        assert_eq!(read_back, written, "for the setting {setting:?}");
        assert_eq!(read_back.general.theme, setting);
        assert_eq!(outcome, ReadOutcome::Current);
    }
}

// Criterion 12 of T-11-2. A file from before the key, once written back, carries
// `theme = "system"` in so many words — and everything else it said is still what it says.
// The «nothing else moved» half is a comparison of parsed structures, not of bytes: byte
// identity is not what section 7 promises (the writer owns the formatting), field identity
// is.
#[test]
fn a_file_without_the_key_gains_an_explicit_system_and_loses_nothing() {
    let dir = TestDir::new("theme_written_back");

    // A current file a person could have today: no `theme` anywhere, and the fields that
    // are present are deliberately not the defaults, so damage would show.
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [general]\n\
             enabled = false\n\
             language = \"en\"\n\
             \n\
             [hotkey]\n\
             key = \"ScrollLock\"\n\
             \n\
             [exclusions]\n\
             processes = [\"mstsc.exe\"]\n"
        ),
    );

    let (read, outcome) = settings::read_from(&path).expect("the file must be readable");
    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(read.general.theme, ThemeSetting::System);

    settings::write_to(&path, &read).expect("writing it back must succeed");

    // The key is now in the file in so many words…
    let text = fs::read_to_string(&path).expect("the written file must be readable");
    assert!(
        text.contains("theme = \"system\""),
        "the written file does not spell the key out: {text}"
    );

    // …and nothing else moved: the structures are equal, and the fields the person had
    // set are named one by one so a failure points at a field and not at a struct.
    let (reread, outcome) = settings::read_from(&path).expect("the rewritten file must parse");
    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(reread, read);
    assert!(!reread.general.enabled);
    assert_eq!(reread.general.language, Language::En);
    assert_eq!(reread.hotkey.key, "ScrollLock");
    assert_eq!(reread.exclusions.processes, ["mstsc.exe"]);
}

// =========================================================================================
// FR-92а — the appearance combo of the dialog, the storage half. Task T-11-3.
// =========================================================================================

// Criterion 11 of T-11-3. The order of the three combo items is the order of the
// `ThemeSetting` values, pinned through the very pair of functions the dialog calls —
// `fill_dialog` adds the items with `theme_combo_index` saying where the current setting
// sits, and `read_dialog` turns the selection back with `theme_from_combo_index`. A copy of
// the mapping here would test the copy; calling the pair tests the dialog.
#[test]
fn the_theme_combo_order_is_the_order_of_the_theme_setting_values() {
    // Index 0 is `System`, 1 is `Light`, 2 is `Dark` — the order the items 3058–3060 go
    // into the combo, and the order the enum declares.
    assert_eq!(settings::theme_combo_index(ThemeSetting::System), 0);
    assert_eq!(settings::theme_combo_index(ThemeSetting::Light), 1);
    assert_eq!(settings::theme_combo_index(ThemeSetting::Dark), 2);

    // The inverse agrees with the forward map for every value, so a reordering of either
    // function alone cannot pass.
    for setting in [
        ThemeSetting::System,
        ThemeSetting::Light,
        ThemeSetting::Dark,
    ] {
        let index = settings::theme_combo_index(setting);
        let back = settings::theme_from_combo_index(
            isize::try_from(index).expect("a combo index fits an isize"),
        );
        assert_eq!(back, setting, "index {index} does not round-trip");
    }

    // What `CB_GETCURSEL` answers when nothing is selected — a state the dropdown list
    // cannot reach from the keyboard or the mouse — reads as the default of FR-92а.
    assert_eq!(settings::theme_from_combo_index(-1), ThemeSetting::System);
}

// =========================================================================================
// FR-94, решение 99.1 — which of the two mechanisms a language change asks for. Task Т-31-2.
// =========================================================================================

/// **Criterion 1 of task Т-31-2.** All four combinations of direction, plus «the language did
/// not change», through the very function the dialog calls.
///
/// The rule the user asked for is not «other language → rebuild the window»: it is «other
/// **direction of writing** → rebuild», because the mirror is set once, when the window is
/// created (task Т-30-2 — a copy of the template with `WS_EX_LAYOUTRTL`, and there is no later
/// moment to put the style on). Words can be changed in place; a direction cannot.
#[test]
fn the_mechanism_of_a_language_change_follows_the_direction_of_writing() {
    use settings::LanguageSwitch::{Relabel, Reopen, Unchanged};

    // Left to right → left to right: the words move, the window does not.
    assert_eq!(
        settings::language_switch(Language::Ru, Language::De),
        Relabel
    );
    // Right to left → right to left: the same, and this is the pair that would be lost by a
    // rule written on «is the new language right-to-left» instead of on the change.
    assert_eq!(
        settings::language_switch(Language::He, Language::Ar),
        Relabel
    );
    // The two crossings — the only cases that cost a window.
    assert_eq!(
        settings::language_switch(Language::Ru, Language::He),
        Reopen
    );
    assert_eq!(
        settings::language_switch(Language::Ar, Language::En),
        Reopen
    );

    // And the language that did not change asks for nothing at all: «Применить» pressed twice
    // must not blink the window the second time.
    for language in Language::ALL {
        assert_eq!(
            settings::language_switch(language, language),
            Unchanged,
            "{language:?} → {language:?} changes nothing"
        );
    }

    // The whole square, so that no pair is decided by an accident of the two above: every
    // ordered pair of the fourteen locales, checked against the predicate that owns the fact.
    let mut relabel = 0;
    let mut reopen = 0;

    for old in Language::ALL {
        for new in Language::ALL {
            let wanted = match (old == new, old.is_rtl() == new.is_rtl()) {
                (true, _) => Unchanged,
                (false, true) => Relabel,
                (false, false) => Reopen,
            };

            assert_eq!(
                settings::language_switch(old, new),
                wanted,
                "{old:?} → {new:?}"
            );

            match wanted {
                Relabel => relabel += 1,
                Reopen => reopen += 1,
                Unchanged => {}
            }
        }
    }

    println!("--- 14 × 14 pairs: relabel {relabel}, reopen {reopen} ---");

    // Twelve left-to-right locales and two right-to-left ones: 12 × 11 + 2 × 1 = 134 pairs
    // keep the direction, and 12 × 2 + 2 × 12 = 48 cross it. A number, so that a mechanism
    // quietly claiming every pair could not pass this test.
    assert_eq!(relabel, 134, "pairs that keep the direction of writing");
    assert_eq!(reopen, 48, "pairs that cross it");
}

/// **Criterion 2 of task Т-31-2, and the guard over a line a measurement paid for.**
///
/// Three claims about the shape, swept over the source, because none of them can be driven from
/// a test without a raised window — the acceptance of the behaviour itself is the stand of §3 of
/// the mandate, which compares the relabelled window with a fresh one pixel by pixel.
///
/// 1. **One body, two occasions.** `relabel_dialog` is called from `fill_dialog` (the window is
///    being built) and from `relabel_in_place` (the language changed under it) — решение 99.1
///    asks for the very same filling, not for a second one that would drift.
/// 2. **The cached picture is thrown away** before the repaint. This is the line the mandate did
///    not ask for and the measurement did: `scratchpad-Э31\красное-до-2-кэш-фона.log` — with the
///    line taken out, the relabelled window differs from a fresh one by **3444 pixels**, in six
///    bands that are exactly the six panel captions, because `on_erase_background` draws them
///    into a picture whose key knows nothing about text. With the line in, the difference is 0.
/// 3. **«ОК» does not relabel and does not rebuild** — решение 99: записать и закрыть.
///
/// ⚠ Э22's trap: a sweep that finds its own needle. This one reads `src\settings.rs` and nothing
/// else, and cuts the product half off at the test module before looking.
#[test]
fn the_two_mechanisms_of_the_language_change_are_written_where_the_measurement_put_them() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("settings.rs"),
    )
    .expect("src\\settings.rs must be readable")
    .replace("\r\n", "\n");

    let product = source
        .split_once("\n#[cfg(test)]\nmod tests {")
        .map_or(source.as_str(), |(before, _)| before);

    let calls = product.matches("relabel_dialog(hwnd, state)").count();

    println!("--- relabel_dialog is called {calls} times ---");

    assert_eq!(
        calls, 2,
        "the text half of the window is filled in by one body on both occasions — when the \
         window is built and when the language changes under it"
    );

    for caller in ["fn fill_dialog(", "unsafe fn relabel_in_place("] {
        let at = product
            .find(caller)
            .unwrap_or_else(|| panic!("{caller} must be in this file"));
        let body = &product[at..];
        let end = body.find("\n}").expect("a function closes with its brace");
        let body = &body[..end];

        println!("--- {caller} ---\n{body}");

        assert!(
            body.contains("relabel_dialog(hwnd, state)"),
            "{caller} must fill the text half through the one body"
        );
    }

    // The line the measurement paid for, in the one function that may hold it: `fill_dialog`
    // must **not** hold it — on `WM_INITDIALOG` there is no picture to throw away.
    let at = product
        .find("unsafe fn relabel_in_place(")
        .expect("relabel_in_place must be in this file");
    let relabel = &product[at..];
    let end = relabel
        .find("\n}")
        .expect("a function closes with its brace");
    let relabel = &relabel[..end];

    assert!(
        relabel.contains("state.background = None;"),
        "the cached picture of the background carries the six panel captions and its key knows \
         nothing about their text: without this line «Применить» leaves the window translated \
         by halves — measured, 3444 pixels in six bands"
    );
    assert!(
        relabel.contains("repaint_whole_window(hwnd)"),
        "…and the window is repainted whole afterwards"
    );

    // «ОК» applies and closes, and does neither of the two mechanisms — решение 99.
    let at = product
        .find("unsafe fn on_command(")
        .expect("on_command must be in this file");
    let commands = &product[at..];
    let end = commands
        .find("\n/// Whether one of the four arrow keys")
        .unwrap_or(commands.len());
    let commands = &commands[..end];

    let ok_at = commands
        .find("OK_COMMAND => {")
        .expect("the «ОК» arm must be in on_command");
    let apply_at = commands
        .find("IDC_APPLY => {")
        .expect("the «Применить» arm must be in on_command");

    let ok_arm = &commands[ok_at..apply_at];

    println!("--- the «ОК» arm ---\n{ok_arm}");

    assert!(
        ok_arm.contains("end_dialog(hwnd,"),
        "«ОК» writes and closes"
    );
    assert!(
        !ok_arm.contains("relabel_in_place") && !ok_arm.contains("reopen_in_place"),
        "…and does neither mechanism: the window has a moment to live, and relabelling or \
         rebuilding it in that moment would be a flash and nothing else"
    );
}

// =========================================================================================
// FR-92а — the painting half of the dialog: colour roles and the title bar. Task T-11-4.
// =========================================================================================
//
// What is testable without a window is exactly the two pure functions the handlers call:
// the mapping «identifier → colour role» and the choice of the DWM flag from the palette.
// The `WM_CTLCOLOR*` handlers themselves need a live dialog and are checked by the
// controller's instrument on the real window at acceptance.

// Task T-14-3 moved the colour-role vocabulary into its owner, `theme` (§6.2): the roles and
// the resolution live there, the mapping «identifier → role» — `settings::static_color_role`
// and its neighbours — stays with the module that owns the window.
use lang_switcher::theme::{
    ButtonBorderRole, ButtonColors, ButtonFaceRole, ButtonTextRole, FOG, GRAPHITE, StaticColorRole,
    resolve,
};

// Criterion 10 of T-11-4. The mapping is closed by a table: every explanatory note of the
// dialog by its actual identifier, the `ES_READONLY` trap, and every other static the
// template carries as a sample of the default. The numbers are written out here rather
// than imported — the same rule the template tests below follow: a test that imported the
// identifiers would agree with any renumbering of them.
#[test]
fn the_static_colour_roles_follow_the_table_of_fr_92a() {
    // The explanatory notes and hints — muted ink over the window background. Six until task
    // Т-31-3 retired 1092, `IDC_LANGUAGE_RESTART`, with its sentence (решение 99.4).
    for (control, name) in [
        (1011, "IDC_HOTKEY_NOTE"),
        (1098, "IDC_CYCLE_HINT"),
        (1105, "IDC_EXCLUSION_HINT"),
        (1027, "IDC_LAYOUT_NOTE"),
        (1062, "IDC_LOG_DIR"),
    ] {
        assert_eq!(
            settings::static_color_role(control),
            StaticColorRole::Muted,
            "{name} ({control}) is an explanatory note and must be muted"
        );
    }

    // ⚠ The `ES_READONLY` trap: IDC_HOTKEY is an edit field to the eye, but a read-only
    // edit is asked about with `WM_CTLCOLORSTATIC` — it must come out a field, not a
    // caption, or the one field of the «Горячая клавиша» group stays window-coloured.
    assert_eq!(
        settings::static_color_role(1010),
        StaticColorRole::Field,
        "IDC_HOTKEY (1010) is a read-only edit and must be painted as a field"
    );

    // Everything else the message asks about is an ordinary caption: group boxes, labels,
    // the text of checkboxes and radios, the state lines.
    for (control, name) in [
        (1090, "IDC_GROUP_GENERAL"),
        (1091, "IDC_LANGUAGE_LABEL"),
        (1109, "IDC_THEME_LABEL"),
        (1094, "IDC_HOTKEY_LABEL"),
        (1096, "IDC_PAIR_SOURCE_LABEL"),
        (1097, "IDC_PAIR_TARGET_LABEL"),
        (1107, "IDC_LOG_DIR_LABEL"),
        (1070, "IDC_STATE_HOOK"),
        (1071, "IDC_STATE_LAYOUTS"),
        (1072, "IDC_STATE_AUTOSTART"),
        (1001, "IDC_AUTOSTART"),
    ] {
        assert_eq!(
            settings::static_color_role(control),
            StaticColorRole::Label,
            "{name} ({control}) is an ordinary caption and must be a label"
        );
    }
}

// Criterion 11 of T-11-4. The DWM flag is a pure function of the palette — dark for
// «Графит», light for «Туман» — checked for both palettes directly and through the six
// rows of `resolve`, the way the dialog actually reaches a palette.
#[test]
fn the_title_bar_is_dark_exactly_for_the_graphite_palette() {
    assert!(
        theme::title_bar_is_dark(&GRAPHITE),
        "the dark palette must ask for a dark title bar"
    );
    assert!(
        !theme::title_bar_is_dark(&FOG),
        "the light palette must not ask for a dark title bar"
    );

    for (setting, system_light, dark_expected) in [
        (ThemeSetting::Light, true, false),
        (ThemeSetting::Light, false, false),
        (ThemeSetting::Dark, true, true),
        (ThemeSetting::Dark, false, true),
        (ThemeSetting::System, true, false),
        (ThemeSetting::System, false, true),
    ] {
        assert_eq!(
            theme::title_bar_is_dark(resolve(setting, system_light)),
            dark_expected,
            "{setting:?} with system_light = {system_light} chose the wrong flag"
        );
    }
}

// =========================================================================================
// FR-92а — the owner-drawn buttons: the closed colour-role table. Task T-11-5a.
// =========================================================================================
//
// Criterion 9: the mapping «(identifier, state) → colour roles (face, text, frame)» is a
// pure function, closed here by the full table — two kinds (ordinary, default «ОК») ×
// the states of the button. Since task T-12-8 there are five of them rather than three:
// наведение joined покой/нажатие/запрет, and «нажата под курсором» is the cell that shows
// the precedence «нажата > горячая». The `WM_DRAWITEM` handler itself needs a live
// dialog and is checked by the controller on the real window at acceptance, along with
// Enter/Esc/Space. The identifiers are literals, not imports — the rule of the template
// tests below: a test that imported the numbers would agree with any renumbering.

#[test]
fn the_button_colour_roles_follow_the_closed_table_of_fr_92a() {
    use ButtonFaceRole as Face;
    use ButtonTextRole as Ink;

    // Every ordinary button of the dialog by its actual identifier — including «Отмена»
    // (2, the manager's own IDCANCEL): the accent belongs to «ОК» alone, so each of these
    // is driven through every state and must never show it.
    let ordinary = [
        (1012, "Задать"),
        (1025, "Выше"),
        (1026, "Ниже"),
        (1053, "Удалить"),
        (1052, "Добавить"),
        (1061, "Открыть папку журнала"),
        (2, "Отмена"),
        (1080, "Применить"),
    ];

    // (hot, pressed, disabled) → the expected face and ink of an ordinary button. The frame is
    // `button_border` in every row of the table — asserted below with the whole struct.
    let ordinary_states = [
        (false, false, false, Face::ButtonBg, Ink::Text),
        // Task T-12-8: the cursor on the button moves the face to `hover_bg` and moves
        // nothing else — the ink stays `text` and the frame stays `button_border`.
        (true, false, false, Face::HoverBg, Ink::Text),
        (false, true, false, Face::SelBg, Ink::SelFg),
        // «Нажата > горячая»: a button held down is under the cursor by definition, and it
        // shows the selection pair and not the hot face.
        (true, true, false, Face::SelBg, Ink::SelFg),
        (false, false, true, Face::ButtonBg, Ink::TextMuted),
        // «Запрет > горячая»: a disabled button takes no click, so the cursor standing on
        // it changes nothing at all — «Выше» and «Ниже» are the ones actually seen grey.
        (true, false, true, Face::ButtonBg, Ink::TextMuted),
    ];

    for (control, name) in ordinary {
        for (hot, pressed, disabled, face, text) in ordinary_states {
            assert_eq!(
                settings::button_color_roles(control, hot, pressed, disabled),
                ButtonColors {
                    face,
                    text,
                    border: ButtonBorderRole::ButtonBorder,
                },
                "«{name}» ({control}), hot = {hot}, pressed = {pressed}, disabled = {disabled}"
            );
        }
    }

    // The default button «ОК» — identifier 1, the dialog manager's own IDOK, the number
    // `DM_SETDEFID` is sent with: the accent pair in its normal state; the selection pair
    // while pressed (the accent yields for the length of the press); muted ink on the
    // ordinary face when disabled — a disabled button takes no Enter and must not
    // advertise itself as the default.
    //
    // ⚠ **That day came — task T-15-2.** Until it, the hot row of this half of the table was
    // the row above it character for character (task T-12-8, п. 4: the palette held no second
    // accent to light «ОК» up with and inventing a colour is forbidden), and the note here
    // said that the day a wave gave the accent a hot face, this assertion was the one that
    // would have to be edited for it. The user reported the consequence — «только вспышка и
    // никакой подсветки» — and the wave arrived: the accented button now answers the pointer
    // with `sel_fg`, **a field the palette already had**, and the ink stays `accent_fg` on
    // both faces. One row moved, and one only.
    //
    // The rest of this half is untouched on purpose, because the precedence is untouched:
    // `disabled` > `pressed` > `hot` > обычное. A pressed «ОК» still shows the selection pair
    // whether or not the cursor is on it, and a disabled one still shows the ordinary face and
    // muted ink — the accent's own answer to the pointer sits **below** both of them.
    //
    // ⚠ Why `sel_fg` and not `hover_bg` — the face every ordinary button takes: `hover_bg` on
    // the accent inverts it («Графит» 228,231,234 → 52,58,67, the ground of the window; «Туман»
    // 43,47,54 → 234,238,242), and an accent that goes out is not a highlight. `sel_fg` moves
    // it the way an ordinary button moves — lighter in the dark palette, darker in the light
    // one. `theme::ButtonFaceRole::SelFg` carries the numbers and the reasoning.
    let default_states = [
        (false, false, false, Face::AccentBg, Ink::AccentFg),
        (true, false, false, Face::SelFg, Ink::AccentFg),
        (false, true, false, Face::SelBg, Ink::SelFg),
        (true, true, false, Face::SelBg, Ink::SelFg),
        (false, false, true, Face::ButtonBg, Ink::TextMuted),
        (true, false, true, Face::ButtonBg, Ink::TextMuted),
    ];

    for (hot, pressed, disabled, face, text) in default_states {
        assert_eq!(
            settings::button_color_roles(1, hot, pressed, disabled),
            ButtonColors {
                face,
                text,
                border: ButtonBorderRole::ButtonBorder,
            },
            "«ОК» (1), hot = {hot}, pressed = {pressed}, disabled = {disabled}"
        );
    }
}

/// **Criterion 13 of T-12-4 — finding A-13**: the «ОК» of the about window is drawn with a
/// fill and **no frame**, and nothing else about it moves.
///
/// The mock-up's generator draws the buttons of the settings dialog framed (`FillR` then
/// `StrokeR`) and the button of the about window filled only (`chrome.ps1:200-202` — one
/// `FillR` and no stroke). The difference belongs to the window, so it lives in a wrapper over
/// the table above and not in a seventh column of it: this test says the wrapper changes the
/// frame **and only** the frame, in every state of the table, which is what «заливка и чернила
/// прежние» means where a table can say it.
#[test]
fn the_about_button_is_the_same_table_without_a_frame() {
    // The whole table of the wrapped function, on both the ordinary identifier and «ОК» —
    // there is only one button in that window, but the wrapper is not told so, and a wrapper
    // that quietly answered something else for a foreign identifier would be a trap.
    for control in [1, 1012] {
        for hot in [false, true] {
            for pressed in [false, true] {
                for disabled in [false, true] {
                    let framed = settings::button_color_roles(control, hot, pressed, disabled);
                    let plain = settings::about_button_colors(control, hot, pressed, disabled);

                    assert_eq!(
                        framed.face, plain.face,
                        "the face must not move ({control}, hot = {hot}, pressed = {pressed}, \
                         disabled = {disabled})"
                    );
                    assert_eq!(
                        framed.text, plain.text,
                        "the ink must not move ({control}, hot = {hot}, pressed = {pressed}, \
                         disabled = {disabled})"
                    );
                    assert_eq!(
                        framed.border,
                        ButtonBorderRole::ButtonBorder,
                        "the settings dialog keeps its frame ({control})"
                    );
                    assert_eq!(
                        plain.border,
                        ButtonBorderRole::FaceItself,
                        "the about window's button has no frame in any state ({control})"
                    );
                }
            }
        }
    }
}

/// **Criterion 9 of T-12-8** — the response of the cursor is a member of the palette and not a
/// colour of its own, in both palettes.
///
/// The role tables above say «`hover_bg`»; this says what that field *is*, and that it is a
/// field of the palette rather than a number invented for this task. The two literals are the
/// ones the mock-up generator carries (`scratchpad-Э11\ui.ps1`: `Hover=(Col 52 58 67)` for
/// «Графит» and `Hover=(Col 234 238 242)` for «Туман»).
///
/// ⚠ **Раньше здесь стояло «и это те же числа, которыми светится меню трея».** Больше это
/// не так: у меню есть своё `menu_hover_bg`, и в «Тумане» оно намеренно другое — 216,222,229.
/// Причина не в цвете, а в земле: кнопка стоит на `button_bg` 255,255,255, запись меню —
/// на `window_bg` 237,239,242, и общий номер отстоял от первой на 21/17/13, а от второй
/// на 3/1/0. Разные числа взяты ради того, чтобы вид совпал. В «Графите» поля держат один
/// и тот же номер, потому что на тёмной земле двигать было нечего.
///
/// The second half is the part that makes the measurement mean anything: `hover_bg` is
/// **distinct from every other field of its palette**, so a shot of a hot button cannot be
/// confused with a shot of a pressed one, a quiet one or a selected row.
#[test]
fn the_hot_face_of_a_button_is_a_field_of_the_palette_and_not_a_colour_of_its_own() {
    for (palette, name, expected) in [
        (&GRAPHITE, "Графит", (52u8, 58u8, 67u8)),
        (&FOG, "Туман", (234, 238, 242)),
    ] {
        let (red, green, blue) = expected;
        let colorref = u32::from(red) | (u32::from(green) << 8) | (u32::from(blue) << 16);

        assert_eq!(
            palette.hover_bg.0, colorref,
            "«{name}» must light a hot element with {red},{green},{blue}"
        );

        // Every other ground and ink of the same palette — the whole struct but this one
        // field. None of them may be equal to it: the response has to be *visible* as itself.
        for (other, field) in [
            (palette.window_bg, "window_bg"),
            (palette.title_bg, "title_bg"),
            (palette.title_fg, "title_fg"),
            (palette.panel_bg, "panel_bg"),
            (palette.panel_border, "panel_border"),
            (palette.text, "text"),
            (palette.text_muted, "text_muted"),
            (palette.cap, "cap"),
            (palette.field_bg, "field_bg"),
            (palette.field_border, "field_border"),
            (palette.button_bg, "button_bg"),
            (palette.button_border, "button_border"),
            (palette.accent_bg, "accent_bg"),
            (palette.accent_fg, "accent_fg"),
            (palette.box_border, "box_border"),
            (palette.sel_bg, "sel_bg"),
            (palette.sel_fg, "sel_fg"),
            // ⛔ `menu_hover_bg` в списке нет намеренно, и это единственное исключение.
            // Это тот же отклик на курсор, только в другом месте и на другой земле;
            // в «Графите» он держит ровно тот же номер, и запрет на совпадение сделал бы
            // красным то, что верно по замыслу. Прочие семнадцать полей — иные роли,
            // и совпадение с ними по-прежнему дефект.
        ] {
            assert_ne!(
                other.0, palette.hover_bg.0,
                "«{name}»: `hover_bg` must not be the same number as `{field}` — \
                 a response nobody can tell from another state is no response"
            );
        }
    }
}

// =========================================================================================
// FR-92а — the owner-drawn check boxes and radio buttons: the closed glyph table.
// Task T-11-5b.
// =========================================================================================
//
// Criterion 9: the mapping «(вид, взведён, запрещён) → роли красок глифа» is a pure
// function, closed here by the full 2×2×2 table — two kinds (check box, radio button) ×
// two check states × two enablements. The drawing half needs a live dialog and is checked
// by the controller on the real window at acceptance, along with clicks, Space and the
// arrow keys.

// T-14-3: the glyph roles moved to their owner, `theme`.
use lang_switcher::theme::{
    GlyphColors, GlyphFillRole, GlyphFrameRole, GlyphKind, GlyphMarkRole, GlyphTextRole,
};

#[test]
fn the_glyph_colour_roles_follow_the_closed_2x2x2_table_of_fr_92a() {
    use GlyphFillRole as Fill;
    use GlyphFrameRole as Frame;
    use GlyphKind as Kind;
    use GlyphMarkRole as Mark;
    use GlyphTextRole as Ink;

    // Every cell of the input space, with the whole expected answer beside it:
    // - the checked, enabled check box is the one cell the accent covers whole — accent
    //   fill edge to edge, accent-ink check mark, no frame;
    // - the checked, enabled radio keeps the field ground and shows the accent as the
    //   dot — an accent-filled circle would hide an accent dot;
    // - disabling quenches the accent (the precedent of the button table): a checked but
    //   disabled glyph drops to the box_border mark on the field ground, and the caption
    //   ink goes muted with it;
    // - every unchecked cell is the quiet ground itself: field fill, box_border frame,
    //   no mark.
    let table: [(Kind, bool, bool, GlyphColors); 8] = [
        (
            Kind::CheckBox,
            true,
            false,
            GlyphColors {
                fill: Fill::AccentBg,
                frame: None,
                mark: Some(Mark::AccentFg),
                text: Ink::Text,
            },
        ),
        (
            Kind::CheckBox,
            false,
            false,
            GlyphColors {
                fill: Fill::FieldBg,
                frame: Some(Frame::BoxBorder),
                mark: None,
                text: Ink::Text,
            },
        ),
        (
            Kind::CheckBox,
            true,
            true,
            GlyphColors {
                fill: Fill::FieldBg,
                frame: Some(Frame::BoxBorder),
                mark: Some(Mark::BoxBorder),
                text: Ink::TextMuted,
            },
        ),
        (
            Kind::CheckBox,
            false,
            true,
            GlyphColors {
                fill: Fill::FieldBg,
                frame: Some(Frame::BoxBorder),
                mark: None,
                text: Ink::TextMuted,
            },
        ),
        (
            Kind::RadioButton,
            true,
            false,
            GlyphColors {
                fill: Fill::FieldBg,
                frame: Some(Frame::BoxBorder),
                mark: Some(Mark::AccentBg),
                text: Ink::Text,
            },
        ),
        (
            Kind::RadioButton,
            false,
            false,
            GlyphColors {
                fill: Fill::FieldBg,
                frame: Some(Frame::BoxBorder),
                mark: None,
                text: Ink::Text,
            },
        ),
        (
            Kind::RadioButton,
            true,
            true,
            GlyphColors {
                fill: Fill::FieldBg,
                frame: Some(Frame::BoxBorder),
                mark: Some(Mark::BoxBorder),
                text: Ink::TextMuted,
            },
        ),
        (
            Kind::RadioButton,
            false,
            true,
            GlyphColors {
                fill: Fill::FieldBg,
                frame: Some(Frame::BoxBorder),
                mark: None,
                text: Ink::TextMuted,
            },
        ),
    ];

    for (kind, checked, disabled, expected) in table {
        assert_eq!(
            theme::glyph_color_roles(kind, checked, disabled),
            expected,
            "{kind:?}, checked = {checked}, disabled = {disabled}"
        );
    }
}

// =========================================================================================
// FR-92а — the check-state store of the six glyph elements. Task T-11-5b-2.
// =========================================================================================
//
// Criterion 9 — the substance of the defect: a `BS_OWNERDRAW` button keeps no check state
// of its own, so the dialog now keeps its own store, and this section closes that store
// without a live window. The configuration goes in the way `fill_dialog` writes it and
// comes back out the way `read_dialog` reads it; a click's flip inverts exactly the
// element clicked; a radio walk quenches the neighbours of its own range and no one else.
// The identifiers are the template's own — the same six the style test below reads out
// of the built binary.
//
// ⚠ **Six since task Т-23-2** (решения 81 и 82): the three method radios of «Замена» left
// the dialog with their group. The one radio run that is left is the layout mode, and the
// four check boxes are the three of «Общие» — autostart, sound, and the selection switch
// that moved here — plus the journal switch of «Диагностика».

use lang_switcher::settings::{GLYPH_CHECK_CONTROLS, GlyphChecks};

/// The six identifiers by name, mirrored from `app.rc` exactly as the style test's list, and
/// **in template order**: «Общие» first, then the mode run of «Раскладки», then
/// «Диагностика».
///
/// Eight until task Т-21-5 added the sound switch of FR-100, nine with it, six since task
/// Т-23-2 removed the «Замена» group.
const GLYPH_AUTOSTART: i32 = 1001;
const GLYPH_SOUND: i32 = 1004;
const GLYPH_SELECTION_ENABLED: i32 = 1040;
const GLYPH_MODE_PAIR: i32 = 1020;
const GLYPH_MODE_CYCLE: i32 = 1021;
const GLYPH_LOG_ENABLED: i32 = 1060;

#[test]
fn the_store_lists_the_six_template_identifiers_and_starts_all_unchecked() {
    assert_eq!(
        GLYPH_CHECK_CONTROLS.as_slice(),
        [
            GLYPH_AUTOSTART,
            GLYPH_SOUND,
            GLYPH_SELECTION_ENABLED,
            GLYPH_MODE_PAIR,
            GLYPH_MODE_CYCLE,
            GLYPH_LOG_ENABLED,
        ]
        .as_slice(),
        "the storage side must list the same six controls the template carries"
    );

    let checks = GlyphChecks::new();

    for control in GLYPH_CHECK_CONTROLS {
        assert!(
            !checks.get(control),
            "before the configuration is written in, {control} must read «снят» — the \
             very answer a WM_DRAWITEM that outruns initialisation must draw (NFR-13)"
        );
    }
}

#[test]
fn what_the_configuration_wrote_into_the_store_is_what_reads_back_out() {
    // Every combination of the five configuration facts the six elements carry: the four
    // check boxes and the one radio group. 2 × 2 × 2 × 2 × 2 = 32 round trips.
    //
    // ⚠ Т-23-2: the method run is gone with its group, and the sound switch of FR-100 —
    // absent from this sweep since Т-21-5 put it in the store — takes the freed dimension.
    for autostart in [false, true] {
        for sound in [false, true] {
            for selection in [false, true] {
                for log in [false, true] {
                    for mode in [LayoutMode::Pair, LayoutMode::Cycle] {
                        let checks = GlyphChecks::new();

                        // The write half, exactly the calls `fill_dialog` makes: four
                        // set calls and one radio walk.
                        checks.set(GLYPH_AUTOSTART, autostart);
                        checks.set(GLYPH_SOUND, sound);
                        checks.set(GLYPH_SELECTION_ENABLED, selection);
                        checks.check_radio(
                            GLYPH_MODE_PAIR,
                            GLYPH_MODE_CYCLE,
                            match mode {
                                LayoutMode::Pair => GLYPH_MODE_PAIR,
                                LayoutMode::Cycle => GLYPH_MODE_CYCLE,
                            },
                        );
                        checks.set(GLYPH_LOG_ENABLED, log);

                        // The read half, exactly the reads `read_dialog` performs.
                        assert_eq!(checks.get(GLYPH_AUTOSTART), autostart);
                        assert_eq!(checks.get(GLYPH_SOUND), sound);
                        assert_eq!(checks.get(GLYPH_SELECTION_ENABLED), selection);
                        assert_eq!(checks.get(GLYPH_LOG_ENABLED), log);

                        let mode_back = if checks.get(GLYPH_MODE_CYCLE) {
                            LayoutMode::Cycle
                        } else {
                            LayoutMode::Pair
                        };
                        assert_eq!(mode_back, mode, "the mode must survive the round trip");
                    }
                }
            }
        }
    }
}

#[test]
fn a_clicks_flip_inverts_exactly_the_element_clicked() {
    let checks = GlyphChecks::new();
    checks.set(GLYPH_AUTOSTART, true);

    // The flip `restore_self_switching` performs on a click: the opposite of the
    // element's own stored state, written back.
    checks.set(GLYPH_LOG_ENABLED, !checks.get(GLYPH_LOG_ENABLED));

    assert!(
        checks.get(GLYPH_LOG_ENABLED),
        "the first click arms an unchecked box"
    );
    assert!(
        checks.get(GLYPH_AUTOSTART),
        "a flip of one box must not move another"
    );

    checks.set(GLYPH_LOG_ENABLED, !checks.get(GLYPH_LOG_ENABLED));

    assert!(
        !checks.get(GLYPH_LOG_ENABLED),
        "the second click quenches it again — a double click must not stick"
    );
}

#[test]
fn a_radio_walk_arms_the_chosen_and_quenches_the_neighbours_of_its_range_alone() {
    // ⚠ Т-23-2: one radio run is left, so «and no one else» is now measured against the four
    // check boxes rather than against a second run. The identifier range of the walk is what
    // bounds it, and the four boxes stand on **both** sides of that range — 1001 and 1004
    // below it, 1040 and 1060 above it — which is what makes the bound measurable at all.
    let checks = GlyphChecks::new();

    checks.set(GLYPH_AUTOSTART, true);
    checks.set(GLYPH_SOUND, true);
    checks.set(GLYPH_SELECTION_ENABLED, true);
    checks.set(GLYPH_LOG_ENABLED, true);
    checks.check_radio(GLYPH_MODE_PAIR, GLYPH_MODE_CYCLE, GLYPH_MODE_PAIR);

    assert!(checks.get(GLYPH_MODE_PAIR), "the chosen radio is armed");
    assert!(
        !checks.get(GLYPH_MODE_CYCLE),
        "its neighbour of the same run is quenched"
    );

    // Switching the mode quenches the neighbour and leaves every check box alone.
    checks.check_radio(GLYPH_MODE_PAIR, GLYPH_MODE_CYCLE, GLYPH_MODE_CYCLE);

    assert!(checks.get(GLYPH_MODE_CYCLE), "the new mode is armed");
    assert!(
        !checks.get(GLYPH_MODE_PAIR),
        "the old mode is quenched on the same walk"
    );

    for (control, name) in [
        (GLYPH_AUTOSTART, "автозапуск"),
        (GLYPH_SOUND, "звуковой отклик"),
        (GLYPH_SELECTION_ENABLED, "конвертировать выделенное"),
        (GLYPH_LOG_ENABLED, "вести журнал"),
    ] {
        assert!(
            checks.get(control),
            "«{name}» ({control}) is a check box and not part of any radio run"
        );
    }
}

#[test]
fn an_identifier_outside_the_six_reads_unchecked_and_stores_nothing() {
    let checks = GlyphChecks::new();

    // 1010 is the hotkey field — a control of the dialog, but not a glyph element. It was
    // 1032, the delay field, until task Т-23-2 took that control out of the template.
    checks.set(1010, true);

    assert!(
        !checks.get(1010),
        "an identifier outside the six must read «снят» (NFR-13)"
    );

    for control in GLYPH_CHECK_CONTROLS {
        assert!(
            !checks.get(control),
            "a dropped write must not land on {control}"
        );
    }
}

// =========================================================================================
// FR-92а — the panel map: containment of rectangles. Task T-11-5c.
// =========================================================================================
//
// Criterion 10: the map «identifier → lies on a panel?» is a pure function of rectangles
// alone, closed here on synthetic ones — inside, outside, and on the boundary. The
// coordinate space is deliberately meaningless small integers: the dialog feeds the
// function `GetWindowRect` screen rectangles, this test feeds it numbers it made up, and
// containment does not care — which is the very property that makes the map testable
// without a live window (the product is not started by any test).

/// A rectangle from its four edges, in the order the `RECT` fields are declared.
fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
    RECT {
        left,
        top,
        right,
        bottom,
    }
}

#[test]
fn the_panel_map_is_rectangle_containment_with_the_boundary_counted_in() {
    // Two panels apart from each other, as the two columns of the dialog are.
    let panels = [rect(10, 10, 110, 60), rect(10, 70, 110, 120)];

    let controls = [
        // Strictly inside the first panel.
        (1, rect(20, 20, 40, 30)),
        // Strictly outside every panel.
        (2, rect(120, 10, 150, 30)),
        // Coincides with the first panel edge for edge — the whole boundary at once.
        (3, rect(10, 10, 110, 60)),
        // Touches the left and bottom edges of the first panel from the inside.
        (4, rect(10, 50, 30, 60)),
        // Straddles the right edge of the first panel — partly off it, so not on it.
        (5, rect(100, 20, 120, 30)),
        // Straddles the gap between the two panels — inside neither.
        (6, rect(20, 55, 40, 80)),
        // Strictly inside the second panel.
        (7, rect(50, 80, 90, 110)),
    ];

    assert_eq!(
        settings::controls_on_panels(&panels, &controls),
        vec![1, 3, 4, 7],
        "containment must count the boundary as inside and an edge crossing as outside"
    );

    // No panels — nothing is on one: the degraded answer of a dialog whose child walk
    // found no group rectangles, and the shape of the map before `WM_INITDIALOG` runs.
    assert_eq!(
        settings::controls_on_panels(&[], &controls),
        Vec::<i32>::new(),
        "with no panels no control can lie on one"
    );
}

// =========================================================================================
// FR-92а — the owner-drawn combo boxes: the closed colour-role table. Task T-11-6.
// =========================================================================================
//
// Criterion 10: the mapping «(закрытая часть, подсвечен) → роли красок пункта» is a pure
// function, closed here by the full 2×2 table — item of the dropped-down list against the
// closed face, ordinary against highlighted. The drawing half needs a live dialog and is
// checked by the controller on the real window at acceptance.

// T-14-3: the combo roles moved to their owner, `theme`.
use lang_switcher::theme::{ComboFillRole, ComboItemColors, ComboTextRole};

#[test]
fn the_combo_item_colour_roles_follow_the_closed_2x2_table_of_fr_92a() {
    use ComboFillRole as Fill;
    use ComboTextRole as Ink;

    let table: [(bool, bool, Fill, Ink); 4] = [
        // An ordinary item of the dropped-down list — the quiet ground of the list, the
        // same colours WM_CTLCOLORLISTBOX erases it with.
        (false, false, Fill::FieldBg, Ink::Text),
        // The highlighted item — the selection pair of the palette.
        (false, true, Fill::SelBg, Ink::SelFg),
        // The closed face, quiet: a field to the eye.
        (true, false, Fill::FieldBg, Ink::Text),
        // The closed face while the manager marks it selected — the combo holding the
        // focus, or the list dropped: still the field pair, or the face would sit on the
        // dialog as a permanently lit stripe; the focus is the dotted rectangle's job.
        (true, true, Fill::FieldBg, Ink::Text),
    ];

    for (closed_part, highlighted, fill, text) in table {
        assert_eq!(
            theme::combo_item_color_roles(closed_part, highlighted),
            ComboItemColors { fill, text },
            "closed_part = {closed_part}, highlighted = {highlighted}"
        );
    }
}

// =========================================================================================
// FR-92а — the check frames of the layout list: the closed colour table and the untouched
// participation bits. Task T-11-7.
// =========================================================================================
//
// Criterion 9: the mapping «(взведена, палитра) → краски кадра» is a pure function, closed
// here by the full 2×2 table — both states against both palettes, every answer a field of
// the palette and nothing else. Criterion 11's other half: the state image bits the
// participation mechanism of FR-31 reads and writes, and the frame order that gives those
// bits their pictures, are held to the values they had before this task.

// Task T-14-5 split this import in two: the colour table of the frame and the order of the
// two frames moved to `theme` with the drawing library, while the state image bits FR-31
// reads and writes stayed with the window that owns them.
use lang_switcher::settings::{CHECKED_IMAGE, CycleRowPaint, UNCHECKED_IMAGE};
use lang_switcher::theme::{CHECK_FRAME_ORDER, CheckFrameColors};

#[test]
fn the_check_frame_colours_follow_the_2x2_table_of_fr_92a() {
    // Both palettes by name, so a swapped pair could not pass: the loop below asserts
    // against the fields of the very palette it hands in.
    for palette in [&GRAPHITE, &FOG] {
        // Снята: the quiet ground of the list under the single-pixel box frame — the same
        // cell the unchecked owner-drawn check box of the dialog paints.
        assert_eq!(
            theme::check_frame_colors(false, palette),
            CheckFrameColors {
                fill: palette.field_bg,
                frame: Some(palette.box_border),
                mark: None,
            },
            "unchecked frame, palette {:?}",
            palette.field_bg
        );

        // Взведена: the accent covers the square whole — no frame — and the check mark is
        // cut from the accent's own foreground, as on the dialog's check boxes.
        assert_eq!(
            theme::check_frame_colors(true, palette),
            CheckFrameColors {
                fill: palette.accent_bg,
                frame: None,
                mark: Some(palette.accent_fg),
            },
            "checked frame, palette {:?}",
            palette.accent_bg
        );
    }
}

#[test]
fn the_state_image_bits_and_the_frame_order_of_fr_31_are_unchanged() {
    // The bits the participation mechanism speaks — written by set_row_check, read by
    // read_cycle_checks — as literals, not as the crate's own constants read back: state
    // image index 1 is the unticked square, index 2 the ticked one, in the form
    // LVIS_STATEIMAGEMASK carries them (index << 12). The values predate this task and a
    // drift here would corrupt every saved cycle.
    assert_eq!(UNCHECKED_IMAGE, 0x1000, "index 1 — the unticked square");
    assert_eq!(CHECKED_IMAGE, 0x2000, "index 2 — the ticked one");

    // And the frames of the custom image list stand in exactly that order: the frame at
    // position i answers state image index i + 1, so «снята» must come first and «взведена»
    // second — the same order the system pair of LVS_EX_CHECKBOXES had, which is what keeps
    // the replacement from moving a single state bit.
    assert_eq!(CHECK_FRAME_ORDER, [false, true]);

    for (position, checked) in CHECK_FRAME_ORDER.into_iter().enumerate() {
        let mask = (u32::try_from(position).expect("two frames") + 1) << 12;

        assert_eq!(
            mask,
            if checked {
                CHECKED_IMAGE
            } else {
                UNCHECKED_IMAGE
            },
            "frame {position} carries the picture of the bits that name it"
        );
    }
}

// -----------------------------------------------------------------------------------------
// FR-92а and FR-31 — task T-11-7-2: the list is never window-disabled (a disabled
// SysListView32 ignores its own colours and paints the system wash — the defect of the
// final sweep), so the pair mode's «выключенность» is logical: muted row paints by the
// pure table below, and a gate that refuses every item change before any action.
// -----------------------------------------------------------------------------------------

#[test]
fn the_row_paints_follow_the_mode_table_of_fr_92a() {
    // Both palettes by name, so a swapped pair could not pass: the loop asserts against
    // the fields of the very palette it hands in — full table, two modes × two roles.
    for palette in [&GRAPHITE, &FOG] {
        // Cycle, ordinary row: nothing is written — the control draws with the colours
        // LVM_SETTEXTCOLOR/LVM_SETTEXTBKCOLOR already gave the whole control.
        assert_eq!(
            settings::cycle_row_paint(LayoutMode::Cycle, false, palette),
            CycleRowPaint {
                colours: None,
                strip_selected: false,
            },
            "cycle mode, ordinary row, palette {:?}",
            palette.field_bg
        );

        // Cycle, selected row: the selection pair of the palette, the system highlight
        // stripped — task T-11-7 exactly as it was.
        assert_eq!(
            settings::cycle_row_paint(LayoutMode::Cycle, true, palette),
            CycleRowPaint {
                colours: Some((palette.sel_bg, palette.sel_fg)),
                strip_selected: true,
            },
            "cycle mode, selected row, palette {:?}",
            palette.sel_bg
        );

        // Pair, both roles: the muted ink on the list's own ground, and the row that still
        // carries LVIS_SELECTED wears the same paints as the rest — selection is not drawn
        // as active in a mode the list has no say in.
        for selected in [false, true] {
            assert_eq!(
                settings::cycle_row_paint(LayoutMode::Pair, selected, palette),
                CycleRowPaint {
                    colours: Some((palette.field_bg, palette.text_muted)),
                    strip_selected: true,
                },
                "pair mode, selected {selected}, palette {:?}",
                palette.text_muted
            );
        }
    }
}

#[test]
fn a_click_in_pair_mode_is_refused_before_any_action_of_fr_31() {
    // Pair: every change the control asks about — tick, selection, focus — is refused at
    // LVN_ITEMCHANGING, before it happens: a click on the list changes nothing.
    assert!(
        settings::cycle_list_change_is_refused(Some(LayoutMode::Pair)),
        "the pair mode refuses every item change"
    );

    // Cycle: the path of FR-31 as it was — changes proceed.
    assert!(
        !settings::cycle_list_change_is_refused(Some(LayoutMode::Cycle)),
        "the cycle mode lets every item change through"
    );

    // No readable state: a programmatic fill holds the borrow, or the dialog is not up
    // yet — the program's own writes pass in either mode, so the list can be filled while
    // the pair mode is on.
    assert!(
        !settings::cycle_list_change_is_refused(None),
        "an unreachable state means a programmatic write, which passes"
    );
}

// =========================================================================================
// FR-92 and FR-93 — the settings dialog and autostart. Task T-08-1.
// =========================================================================================
//
// The dialog is checked the way `tests\tray.rs` checks the menu: from outside, against
// literals written out here, and never against the module's own constants. A test that
// imported the strings from the crate and then asserted that the resource contains them would
// pass whatever the resource said — and the failure mode this task is guarding against is
// precisely a resource whose strings came out wrong (fact 6 of section 9 of STATE.md).
//
// ⚠ The template is read out of the **built `LangSwitcher.exe`**, opened with
// `LOAD_LIBRARY_AS_DATAFILE`, and not out of this test executable. `embed-resource` links
// `app.rc` into the binary targets of the crate only, so the test binaries carry no resource
// section at all: section 4.4 of STATE.md, and the same workaround `tests\tray.rs` applies.

use std::os::windows::ffi::OsStrExt;

use lang_switcher::settings::LayoutRow;
use lang_switcher::{app, guard, hook, inject, layouts, selection};

use windows::Win32::Foundation::{FreeLibrary, HMODULE, HRSRC, RECT, SIZE};
use windows::Win32::System::LibraryLoader::{
    FindResourceExW, FindResourceW, LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW, LoadResource,
    LockResource, SizeofResource,
};
use windows::Win32::UI::WindowsAndMessaging::RT_DIALOG;
use windows::core::PCWSTR;

/// `RT_STRING`, the resource type of a string table.
///
/// Spelled out because the `windows` crate does not export it, exactly as `src\settings.rs`
/// has to. The value is 6 and belongs to the binary interface of the resource loader.
const RT_STRING: PCWSTR = PCWSTR(std::ptr::without_provenance(6));

/// Identifier `app.rc` gives the dialog of FR-92.
const IDD_SETTINGS: u16 = 200;

/// Identifier `app.rc` gives the about dialog of FR-92а — task T-11-11.
const IDD_ABOUT: u16 = 201;

/// The caption of the dialog, written out rather than imported — see the note above.
const DIALOG_CAPTION: &str = "Lang Switcher — настройки";

/// Every section of the table of FR-92, in the order the requirement prints them, plus the
/// state group this task adds. These are the group box captions the template must carry.
/// ⚠ **Six since task Т-23-2** (решения 81 и 82): «Замена» left the dialog whole, and
/// «Выделение» — a group of one check box after its two spin fields went — was dissolved
/// into «Общие». Both settings stay in `config.toml`; what left is the dialog's half.
const FR_92_SECTIONS: [&str; 6] = [
    "Общие",
    "Горячая клавиша",
    "Раскладки",
    "Исключения",
    "Диагностика",
    "Состояние",
];

/// Every visible string of the template, in template order and with the empty ones left out.
///
/// The captions of the sections above are in here too, in their place: this is the whole of
/// what a person reads on that window, and it is what a wrong code page would destroy.
const TEMPLATE_TEXT: [&str; 29] = [
    "Общие",
    "Запускать при входе в систему",
    "Язык интерфейса:",
    // The appearance row of FR-92а, task T-11-3, declared right after the language combo;
    // its own combo carries no text in the template — the items are added by the dialog.
    "Оформление:",
    // «вступит в силу после перезапуска» stood here until task Т-31-3. Решение 99.4 retired it:
    // the language takes effect at once now, and a window that promised a restart would lie.
    // The sound switch of FR-100, task Т-21-5.
    "Звуковой отклик",
    // ⚠ Т-23-2, решение 82.2: the selection switch of FR-61 is the **last row of «Общие»**
    // now. Its group of one was dissolved when the two spin fields beside it left the
    // dialog, and it moved here to its sister check boxes.
    "Конвертировать выделенный текст",
    "Горячая клавиша",
    "Клавиша:",
    // Task T-08-2 replaced «Пока меняется только в файле настроек.» with the button that arms
    // the capture: the sentence was true only for as long as the field could not capture a key.
    "Задать",
    "Раскладки",
    "Пара",
    "Несколько раскладок",
    "Источник:",
    "Цель:",
    "Цикл: галочка — участие, кнопки — порядок",
    "Выше",
    "Ниже",
    "Исключения",
    "Удалить",
    "Добавить",
    "Имя процесса, например game.exe",
    "Диагностика",
    "Вести журнал",
    "Открыть папку журнала",
    "Папка журнала:",
    "Состояние",
    "ОК",
    "Отмена",
    "Применить",
];

/// Identifiers `app.rc` gives the controls, and what each of them is for. One row per element
/// FR-92 names, so that a section losing a control is a failing test and not a smaller window.
///
/// ⚠ **Thirty-eight since task Т-23-2**: eleven controls left the window with the groups
/// «Замена» and «Выделение» — the three method radios of FR-42а, the three millisecond
/// fields of FR-44 and §4.7, and the five statics that captioned them. Their identifiers —
/// 1029, 1030, 1031, 1032, 1041, 1042, 1099, 1100, 1101, 1102, 1103 — are **retired, not
/// freed**: a new control must take a new number, or an installed build and a new one would
/// disagree about what a number means.
///
/// ⚠ And one control **arrived**: 1004, the sound switch task Т-21-5 added to «Общие» and
/// never wrote into this list. A row missing here is a control whose disappearance no test
/// notices, which is the whole point of the list; found while rewriting it for Т-23-2.
///
/// ⚠ **Thirty-seven since task Т-31-3**: 1092, «вступит в силу после перезапуска», left with
/// its sentence (решение 99.4). Its number is retired, not freed, exactly as the eleven above.
const TEMPLATE_CONTROLS: [(u32, &str); 37] = [
    (1001, "Общие: автозапуск"),
    (1002, "Общие: язык интерфейса"),
    (1003, "Общие: оформление — FR-92а"),
    (1004, "Общие: звуковой отклик — FR-100"),
    (1040, "Общие: конвертировать выделенное — FR-61"),
    (1010, "Горячая клавиша: поле клавиши"),
    (1011, "Горячая клавиша: предупреждение"),
    (1012, "Горячая клавиша: кнопка захвата — FR-94"),
    (1020, "Раскладки: режим «Пара»"),
    (1021, "Раскладки: режим «Несколько раскладок»"),
    (1022, "Раскладки: источник пары"),
    (1023, "Раскладки: цель пары"),
    (1024, "Раскладки: список участия в цикле"),
    (1025, "Раскладки: порядок вверх"),
    (1026, "Раскладки: порядок вниз"),
    (1027, "Раскладки: замечание о ненайденной раскладке"),
    (1050, "Исключения: список процессов"),
    (1051, "Исключения: имя процесса"),
    (1052, "Исключения: добавить"),
    (1053, "Исключения: удалить"),
    (1060, "Диагностика: вести журнал"),
    (1061, "Диагностика: открыть папку журнала"),
    (1062, "Диагностика: путь папки журнала"),
    // The static text. It carried -1 until FR-94, and a control identified by -1 is a control
    // whose text can never be replaced — so every one of these numbers is a precondition of the
    // interface having a second language at all.
    (1090, "Общие: заголовок группы"),
    (1091, "Общие: подпись «Язык интерфейса»"),
    // 1092 — «вступит в силу после перезапуска» — снят задачей Т-31-3 (решение 99.4) вместе с
    // предложением, которое перестало быть правдой: язык действует сразу.
    (1093, "Горячая клавиша: заголовок группы"),
    (1094, "Горячая клавиша: подпись «Клавиша»"),
    (1095, "Раскладки: заголовок группы"),
    (1096, "Раскладки: подпись «Источник»"),
    (1097, "Раскладки: подпись «Цель»"),
    (1098, "Раскладки: пояснение к списку цикла"),
    (1104, "Исключения: заголовок группы"),
    (1105, "Исключения: пояснение об имени процесса"),
    (1106, "Диагностика: заголовок группы"),
    (1107, "Диагностика: подпись «Папка журнала»"),
    (1108, "Состояние: заголовок группы"),
    (1109, "Общие: подпись «Оформление» — FR-92а"),
];

// -----------------------------------------------------------------------------------------
// Criterion 10 — the Cyrillic of the dialog survived rc.exe
// -----------------------------------------------------------------------------------------

#[test]
fn the_resource_script_declares_the_utf8_code_page() {
    // Fact 6 of section 9 of STATE.md, in one assertion: without this line rc.exe reads the
    // script in the system ANSI code page and every Cyrillic string in the dialog turns into
    // mojibake, with no error at build time.
    let script = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("app.rc"))
        .expect("app.rc must be readable");

    let pragma = script
        .find("#pragma code_page(65001)")
        .expect("app.rc must declare the UTF-8 code page");
    let dialog = script
        .find("DIALOGEX")
        .expect("app.rc must carry the dialog of FR-92");

    println!("#pragma at byte {pragma}, DIALOGEX at byte {dialog}");

    assert!(
        pragma < dialog,
        "the code page has to be declared before the strings it governs"
    );
}

/// **The user's finding П-1, task T-21-4: neither window had a button on the task bar.**
///
/// «Когда открыто окно настроек или окно о программе, визуально не вижу значка запущенного
/// приложения на панели задач.» The cause is not in these windows but in their owner: the
/// program's UI window is a top-level `WS_POPUP` carrying `WS_EX_TOOLWINDOW` by decision R-20
/// point 2 — deliberately and correctly, because it exists only to receive the broadcast of
/// FR-81 and must never be shown. An owned window inherits that absence: the shell gives no
/// button to a window whose owner has none.
///
/// `WS_EX_APPWINDOW` is the documented answer — «forces a top-level window onto the taskbar
/// when the window is visible» — and it is put in the **template**, not on the live window,
/// because the field is read when the window is created and the shell looks once, when it is
/// first shown. Modality and the owner are unchanged: `DialogBoxParamW` still runs its own loop
/// and the dialog still belongs to the UI window.
///
/// Measured on the bench before it was written, with the owner the product really passes
/// (`scratchpad-Э21\stand21`, a `WS_EX_TOOLWINDOW` owner as in `app.rs`): both windows raised,
/// **zero** new buttons on the task bar. ⚠ And measured with a negative control that caught a
/// broken instrument first — the stand's own console window put a button up on its own, which
/// made the first reading say «the button is there» in a mode that raises no dialog at all.
///
/// The hidden windows of the program are not touched by any of this and must not be: the sweep
/// below is about these two templates only.
#[test]
fn both_windows_of_fr_92_ask_the_task_bar_for_a_button_of_their_own() {
    // `WS_EX_APPWINDOW` and `WS_EX_TOOLWINDOW`, spelled numerically for the reason `app.rc`
    // spells its own constants numerically — the script includes no `windows.h`.
    const WS_EX_APPWINDOW: u32 = 0x0004_0000;
    const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;

    let product = ProductImage::open();

    for (id, what) in [
        (IDD_SETTINGS, "the settings window of FR-92"),
        (IDD_ABOUT, "the «О программе» window of FR-92а"),
    ] {
        let template = DialogTemplate::parse(&product.resource(RT_DIALOG, id));

        println!("{what}: dwExtendedStyle = 0x{:08X}", template.ex_style);

        assert!(
            template.ex_style & WS_EX_APPWINDOW != 0,
            "⚠ {what} asks for no button of its own, so it inherits the absence of its \
             WS_EX_TOOLWINDOW owner and the user sees nothing on the task bar — П-1. \
             dwExtendedStyle is 0x{:08X}",
            template.ex_style
        );
        assert_eq!(
            template.ex_style & WS_EX_TOOLWINDOW,
            0,
            "{what} must not be a tool window itself — that is the owner's role, not this \
             window's, and the two styles together are a contradiction"
        );
    }
}

#[test]
fn the_dialog_template_carries_the_russian_of_fr_92() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    println!("caption: {}", template.caption);
    for text in &template.text {
        println!("  {text}");
    }

    assert_eq!(
        template.caption, DIALOG_CAPTION,
        "the caption came out of rc.exe wrong — check #pragma code_page(65001)"
    );

    assert_eq!(
        template.text, TEMPLATE_TEXT,
        "the visible strings of the dialog differ from what FR-92 is written in"
    );
}

#[test]
fn every_section_of_fr_92_has_its_group_in_the_template() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    for section in FR_92_SECTIONS {
        assert!(
            template.text.iter().any(|text| text == section),
            "section {section} of FR-92 has no group in the dialog"
        );
    }
}

#[test]
fn every_element_of_fr_92_has_its_control_in_the_template() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    for (id, what) in TEMPLATE_CONTROLS {
        assert!(
            template.controls.contains(&id),
            "the dialog has no control {id} — {what}"
        );
    }

    // The three buttons the dialog is driven by. `IDOK` and `IDCANCEL` are the dialog
    // manager's own numbers; «Применить» is ours.
    for id in [1u32, 2, 1080] {
        assert!(
            template.controls.contains(&id),
            "the dialog has no control {id}"
        );
    }
}

/// The eleven identifiers that left the window with «Замена» and «Выделение» — task Т-23-2,
/// решения 81 и 82.
///
/// A list of *absences*, and it is the sharper half of the pair above: `TEMPLATE_CONTROLS`
/// catches a control that disappeared, this one catches a control that came back. The
/// numbers are retired rather than freed — a new control takes a new number — so this list
/// never shrinks and the assertion never weakens.
const RETIRED_CONTROLS: [(u32, &str); 11] = [
    (1029, "Замена: метод «Автоматически» — FR-42а"),
    (1030, "Замена: метод Backspace"),
    (1031, "Замена: метод выделения"),
    (1032, "Замена: задержка между событиями — FR-44"),
    (1041, "Выделение: таймаут буфера обмена — §4.7"),
    (1042, "Выделение: задержка восстановления — §4.7"),
    (1099, "Замена: заголовок группы"),
    (1100, "Замена: подпись задержки"),
    (1101, "Выделение: заголовок группы"),
    (1102, "Выделение: подпись таймаута буфера обмена"),
    (1103, "Выделение: подпись задержки восстановления"),
];

/// **Т-23-2, решения 81 и 82** — the two groups the user asked to be taken out of the dialog
/// are out of the **built** template, control by control.
#[test]
fn the_eleven_controls_of_the_two_removed_groups_are_gone_from_the_template() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    for (id, what) in RETIRED_CONTROLS {
        assert!(
            !template.controls.contains(&id),
            "control {id} — {what} — is back in the dialog; решение 81 took it out and left \
             the setting in config.toml"
        );
    }

    // And their captions are off the window with them: a control removed while its string
    // row stayed would be invisible here but visible to `localise_dialog`.
    for gone in [
        "Замена",
        "Автоматически (рекомендуется)",
        "Backspace",
        "Выделение (совместимость)",
        "Задержка между событиями, мс:",
        "Выделение",
        "Таймаут буфера обмена, мс:",
        "Задержка восстановления, мс:",
    ] {
        assert!(
            !template.text.iter().any(|text| text == gone),
            "«{gone}» is still written on the window"
        );
    }
}

/// **Т-23-2, решение 82.2** — the selection switch of FR-61 stands inside «Общие».
///
/// Containment of rectangles and nothing else: the group panels of this dialog are hidden
/// controls whose rectangles *are* the blocks (task T-11-13), so «which group is this control
/// in» is arithmetic on the template and needs no window. Checked both ways — inside the
/// «Общие» rectangle and inside no other panel — because a control that lies in two panels at
/// once is a template that has drifted, not a control that moved.
#[test]
fn the_selection_switch_stands_in_the_general_panel() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let (sx, sy, scx, scy) = template
        .bounds
        .iter()
        .find(|(id, ..)| *id == 1040)
        .map(|(_, x, y, cx, cy)| (*x, *y, *cx, *cy))
        .expect("the dialog must still carry the selection switch (1040)");

    for (panel, what) in PANELS {
        let (px, py, pcx, pcy) = template
            .bounds
            .iter()
            .find(|(id, ..)| *id == panel)
            .map(|(_, x, y, cx, cy)| (*x, *y, *cx, *cy))
            .unwrap_or_else(|| panic!("the dialog must carry panel {panel} — «{what}»"));

        let inside = sx >= px && sy >= py && sx + scx <= px + pcx && sy + scy <= py + pcy;

        println!("«{what}» ({panel}) {px},{py} {pcx}x{pcy}: selection switch inside = {inside}");

        assert_eq!(
            inside,
            panel == 1090,
            "«Конвертировать выделенный текст» (1040) must lie in «Общие» (1090) and in no \
             other panel — решение 82.2"
        );
    }
}

/// **Т-23-2, решение 83 п. 3** — the geometry the user chose at К-1, in the numbers of the
/// built template.
///
/// Three facts, and every one of them is a number the user looked at on the stand:
/// the window is 420 × 364 dialog units (it was 420 × 385); the two columns end level with
/// one another; and «Состояние» stands **four** units under them — the gap every other pair
/// of groups in this dialog has, and the one the user asked for by name («уменьшить до
/// стандартной величины»).
#[test]
fn the_window_carries_the_geometry_chosen_at_the_control_point() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    println!("window: {:?} dialog units", template.size);

    // ⚠ **420 → 430 by решение 87 п. 3** (task Т-26-3). The width, and only the width, is what
    // moved: the dialog font went from 9 pt to 10, and the vertical base unit grew with it —
    // so the 364 units below are *longer in pixels* than they were and did not have to change.
    // The horizontal unit did **not** grow (Segoe UI has the same 7 px average character width
    // at both sizes, measured), so the two labels that were already tight — «Язык интерфейса:»
    // and «Клавиша:» — needed the ten units this number carries. The user asked for exactly
    // this: «расширить модуль и соответственно окно самой программы».
    assert_eq!(
        template.size,
        (430, 364),
        "the window of решения 83 и 87 is 430 × 364 dialog units"
    );

    let bottom = |panel: u32| -> i32 {
        template
            .bounds
            .iter()
            .find(|(id, ..)| *id == panel)
            .map(|(_, _, y, _, cy)| y + cy)
            .unwrap_or_else(|| panic!("the dialog must carry panel {panel}"))
    };
    let top = |panel: u32| -> i32 {
        template
            .bounds
            .iter()
            .find(|(id, ..)| *id == panel)
            .map(|(_, _, y, ..)| *y)
            .unwrap_or_else(|| panic!("the dialog must carry panel {panel}"))
    };

    let layouts = bottom(1095);
    let diagnostics = bottom(1106);
    let state = top(1108);

    println!(
        "«Раскладки» ends at {layouts}, «Диагностика» at {diagnostics}, «Состояние» \
              starts at {state}"
    );

    assert_eq!(
        layouts, diagnostics,
        "решение 83 п. 1: the two columns end level — that is what «вровень» means"
    );

    // The standard gap of this dialog, measured on a pair the task did not touch: «Общие»
    // and «Горячая клавиша». Read rather than written down, so that a re-cut of the whole
    // layout moves the expectation with it instead of leaving this number behind.
    let standard = top(1093) - bottom(1090);

    println!("standard gap between two groups: {standard} units");

    assert_eq!(
        standard, 4,
        "the groups of this dialog stand four units apart"
    );
    assert_eq!(
        state - layouts,
        standard,
        "решение 83 п. 3: «Состояние» must stand the standard gap under the columns, not \
         the 25 units of the mock-up the user rejected"
    );
}

// Criterion 10 of T-11-5a — read out of the **built** `LangSwitcher.exe`, like everything
// else about the template. ⚠ `BS_OWNERDRAW` (0x0B) is a button *type*, not a flag: types
// live in the low nibble of the style and replace one another, so the check is equality of
// the nibble and not a bit test — `style & 0x0B != 0` would also pass for a plain
// `BS_DEFPUSHBUTTON` (0x01) or a `BS_CHECKBOX` (0x02), which are not owner drawing at all.
#[test]
fn the_nine_buttons_of_the_dialog_are_owner_drawn() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    // The nine push buttons of FR-92, by their actual identifiers — «ОК» (1, the former
    // DEFPUSHBUTTON) and «Отмена» (2) carry the dialog manager's own numbers.
    const OWNER_DRAWN_BUTTONS: [(u32, &str); 9] = [
        (1012, "Задать"),
        (1025, "Выше"),
        (1026, "Ниже"),
        (1053, "Удалить"),
        (1052, "Добавить"),
        (1061, "Открыть папку журнала"),
        (1, "ОК"),
        (2, "Отмена"),
        (1080, "Применить"),
    ];

    for (id, what) in OWNER_DRAWN_BUTTONS {
        let style = template
            .styles
            .iter()
            .find(|(control, _)| *control == id)
            .map(|(_, style)| *style)
            .unwrap_or_else(|| panic!("the dialog has no control {id} — «{what}»"));

        println!("«{what}» ({id}): style {style:#010x}");

        assert_eq!(
            style & 0x0F,
            0x0B,
            "«{what}» ({id}) must carry BS_OWNERDRAW as its button type — task T-11-5a; \
             the style is {style:#010x}"
        );

        // The task changes the button type and nothing else about the template: the tab
        // stop of every button survives, or the keyboard loses the button entirely.
        assert_ne!(
            style & 0x0001_0000,
            0,
            "«{what}» ({id}) must keep WS_TABSTOP; the style is {style:#010x}"
        );
    }
}

// Criterion 10 of T-11-5b — read out of the **built** `LangSwitcher.exe`, exactly as the
// nine buttons above. The same ⚠ applies: `BS_OWNERDRAW` (0x0B) is a button *type* in the
// low nibble, so the check is equality of the nibble and not a bit test — and here the
// former types were `BS_AUTOCHECKBOX` (0x03) and `BS_AUTORADIOBUTTON` (0x09), both of
// which a bit test against 0x0B would also pass.
#[test]
fn the_six_check_boxes_and_radio_buttons_are_owner_drawn_and_notifying() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    // The four check boxes and two radio buttons of FR-92, by their actual identifiers;
    // the third column says which of them open a WS_GROUP run. The list-view ticks of the
    // cycle list are not here — they belong to task T-11-7.
    //
    // ⚠ Six since task Т-23-2: the three method radios of «Замена» left the window. And
    // 1004 — the sound switch of FR-100 — is here for the first time: task Т-21-5 added the
    // control and left this list at eight, so nothing checked its button type at all.
    const OWNER_DRAWN_GLYPHS: [(u32, &str, bool); 6] = [
        (1001, "Запускать при входе в систему", false),
        (1004, "Звуковой отклик", false),
        (1040, "Конвертировать выделенный текст", false),
        (1020, "Пара", true),
        (1021, "Несколько раскладок", false),
        (1060, "Вести журнал", false),
    ];

    for (id, what, opens_group) in OWNER_DRAWN_GLYPHS {
        let style = template
            .styles
            .iter()
            .find(|(control, _)| *control == id)
            .map(|(_, style)| *style)
            .unwrap_or_else(|| panic!("the dialog has no control {id} — «{what}»"));

        println!("«{what}» ({id}): style {style:#010x}");

        assert_eq!(
            style & 0x0F,
            0x0B,
            "«{what}» ({id}) must carry BS_OWNERDRAW as its button type — task T-11-5b; \
             the style is {style:#010x}"
        );

        // BS_NOTIFY (0x4000) is load-bearing: without it a button sends no BN_SETFOCUS,
        // and BN_SETFOCUS is what returns the arrow-key self-checking of the radio
        // groups in src\settings.rs.
        assert_ne!(
            style & 0x4000,
            0,
            "«{what}» ({id}) must carry BS_NOTIFY; the style is {style:#010x}"
        );

        assert_ne!(
            style & 0x0001_0000,
            0,
            "«{what}» ({id}) must keep WS_TABSTOP; the style is {style:#010x}"
        );

        // WS_GROUP (0x00020000) must sit exactly where it sat before the task — on the
        // opener of each radio run and nowhere else among the eight: the ranges
        // CheckRadioButton walks and the arrow-key navigation both end at the *next*
        // control carrying WS_GROUP, so a lost opener merges two groups and an extra one
        // splits a group in half.
        assert_eq!(
            style & 0x0002_0000 != 0,
            opens_group,
            "«{what}» ({id}) must {} WS_GROUP; the style is {style:#010x}",
            if opens_group { "keep" } else { "not gain" }
        );
    }
}

// Criterion 9 of T-11-5c — read out of the **built** `LangSwitcher.exe`, exactly as the
// nine buttons and the six glyphs above. The same ⚠ applies: `BS_OWNERDRAW` (0x0B) is a
// button *type* in the low nibble, so the check is equality of the nibble and not a bit
// test — the former type here was `BS_GROUPBOX` (0x07), which a bit test would also let
// through (0x07 & 0x0B is 0x03, not zero).
#[test]
fn the_six_group_boxes_are_owner_drawn_and_take_no_tab_stop() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    // The six groups of the dialog — the five sections of the FR-92 table plus
    // «Состояние» — by their actual identifiers. Eight until task Т-23-2 removed «Замена»
    // and dissolved «Выделение».
    const OWNER_DRAWN_GROUPS: [(u32, &str); 6] = [
        (1090, "Общие"),
        (1093, "Горячая клавиша"),
        (1095, "Раскладки"),
        (1104, "Исключения"),
        (1106, "Диагностика"),
        (1108, "Состояние"),
    ];

    for (id, what) in OWNER_DRAWN_GROUPS {
        let style = template
            .styles
            .iter()
            .find(|(control, _)| *control == id)
            .map(|(_, style)| *style)
            .unwrap_or_else(|| panic!("the dialog has no control {id} — «{what}»"));

        println!("«{what}» ({id}): style {style:#010x}");

        assert_eq!(
            style & 0x0F,
            0x0B,
            "«{what}» ({id}) must carry BS_OWNERDRAW as its button type — task T-11-5c; \
             the style is {style:#010x}"
        );

        // A group box never had WS_TABSTOP and must not gain one: a panel takes no focus,
        // and a stop on it would add a dead stop to the keyboard loop of the dialog.
        assert_eq!(
            style & 0x0001_0000,
            0,
            "«{what}» ({id}) must not gain WS_TABSTOP; the style is {style:#010x}"
        );
    }
}

// -----------------------------------------------------------------------------------------
// Task T-11-18 — every label of both templates is owner-drawn
// -----------------------------------------------------------------------------------------
//
// ⚠ `SS_OWNERDRAW` (0x0D) is a static *type*, exactly as `BS_OWNERDRAW` (0x0B) is a button
// type: types live in the low nibble of the style and replace one another, so the check is
// equality of the nibble and not a bit test — the type these labels carried before the task
// was `SS_LEFT` (0x00), and `style & 0x0D != 0` would pass for `SS_ICON` (0x03) or
// `SS_BLACKFRAME` (0x07) just as happily.
//
// The list is every `LTEXT` of the settings template — field labels, notes, hints, the three
// rows of the «Состояние» block and the journal path — and every `LTEXT` of the about
// template. The icon of the about window is an `ICON` statement and deliberately not here:
// `SS_ICON` is the type that loads the 32 px frame of the `.ico`, and it draws no text.

/// Every `LTEXT` of the settings template, by identifier and by what it says on the window.
const OWNER_DRAWN_LABELS: [(u32, &str); 14] = [
    (1091, "Общие: подпись «Язык интерфейса»"),
    (1109, "Общие: подпись «Оформление»"),
    // 1092 — «вступит в силу после перезапуска» — снят задачей Т-31-3, решение 99.4.
    (1094, "Горячая клавиша: подпись «Клавиша»"),
    (1011, "Горячая клавиша: предупреждение"),
    (1096, "Раскладки: подпись «Источник»"),
    (1097, "Раскладки: подпись «Цель»"),
    (1098, "Раскладки: пояснение к списку цикла"),
    (1027, "Раскладки: замечание о ненайденной раскладке"),
    (1105, "Исключения: пояснение об имени процесса"),
    (1107, "Диагностика: подпись «Папка журнала»"),
    (1062, "Диагностика: путь папки журнала"),
    (1070, "Состояние: перехват клавиатуры"),
    (1071, "Состояние: раскладки сеанса"),
    (1072, "Состояние: автозапуск в реестре"),
];

/// Every `LTEXT` of the about template. The `ICON` is not one of them — see above; neither is
/// the panel 1125, which is a `Button` exactly as the six panels of the settings dialog are.
///
/// Four until task Т-23-4 (решение 82.5) added the «Как пользоваться» block: five numerals and
/// five rows.
const OWNER_DRAWN_ABOUT_LABELS: [(u32, &str); 14] = [
    (1121, "О программе: имя"),
    (1122, "О программе: версия"),
    (1123, "О программе: первая строка"),
    (1124, "О программе: вторая строка"),
    (1126, "Как пользоваться: номер 1"),
    (1131, "Как пользоваться: строка 1"),
    (1127, "Как пользоваться: номер 2"),
    (1132, "Как пользоваться: строка 2"),
    (1128, "Как пользоваться: номер 3"),
    (1133, "Как пользоваться: строка 3"),
    (1129, "Как пользоваться: номер 4"),
    (1134, "Как пользоваться: строка 4"),
    (1130, "Как пользоваться: номер 5"),
    (1135, "Как пользоваться: строка 5"),
];

/// **Criterion 9 of T-11-18** — read out of the **built** `LangSwitcher.exe`, exactly as the
/// buttons, the glyphs and the panels above are.
#[test]
fn every_label_of_both_templates_is_owner_drawn() {
    let product = ProductImage::open();

    for (resource, labels) in [
        (IDD_SETTINGS, OWNER_DRAWN_LABELS.as_slice()),
        (IDD_ABOUT, OWNER_DRAWN_ABOUT_LABELS.as_slice()),
    ] {
        let template = DialogTemplate::parse(&product.resource(RT_DIALOG, resource));

        for (id, what) in labels {
            let style = template.style_of(*id, what);

            println!("«{what}» ({id}): style {style:#010x}");

            assert_eq!(
                style & 0x0F,
                0x0D,
                "«{what}» ({id}) must carry SS_OWNERDRAW as its static type — task T-11-18; \
                 the style is {style:#010x}"
            );

            // The task changes the static type and nothing else about the template. rc.exe
            // gives an `LTEXT` WS_GROUP whether or not a style expression is written out, and
            // the tab order of the dialog stands on it: a label that lost WS_GROUP would let
            // the arrow keys of the run before it walk on into the next block.
            assert_ne!(
                style & 0x0002_0000,
                0,
                "«{what}» ({id}) must keep WS_GROUP; the style is {style:#010x}"
            );

            // A label never had WS_TABSTOP and must not gain one: static text takes no focus.
            assert_eq!(
                style & 0x0001_0000,
                0,
                "«{what}» ({id}) must not gain WS_TABSTOP; the style is {style:#010x}"
            );
        }
    }
}

/// **Criterion 9 of T-11-18, the closing half** — the two lists above are *every* label of
/// their template and not a convenient subset: no static may stay on the system's own drawing.
///
/// The question the lists cannot answer by themselves is «is that all of them», and it is the
/// one that decides the task: a single label left behind puts a ClearType line next to the
/// grey-antialiased ones and the window still reads as two type faces. So the walk goes the
/// other way round — over every control the built resource carries, picking out the ones whose
/// **class** is `Static`, and every one of those has to be either a listed label or the icon.
///
/// The class and not the style, because the low nibble means a different thing for every
/// class: `LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS` of the cycle list is 0x0D, which is
/// the very number `SS_OWNERDRAW` is, and a style-only test would count that list as a label.
#[test]
fn no_static_of_either_template_was_left_on_the_system_drawing() {
    let product = ProductImage::open();

    /// The ordinal of the predefined `Static` class in a dialog template.
    const STATIC_CLASS: u16 = 0x0082;

    // The one static of either template that is *not* a label: the `ICON` statement of the
    // about window, whose `SS_ICON` (0x03) type is what loads the 32 px frame of the `.ico`.
    const ABOUT_ICON: u32 = 1120;

    for (resource, labels) in [
        (IDD_SETTINGS, OWNER_DRAWN_LABELS.as_slice()),
        (IDD_ABOUT, OWNER_DRAWN_ABOUT_LABELS.as_slice()),
    ] {
        let template = DialogTemplate::parse(&product.resource(RT_DIALOG, resource));

        let statics: Vec<u32> = template
            .classes
            .iter()
            .filter(|(_, class)| *class == Some(STATIC_CLASS))
            .map(|(id, _)| *id)
            .collect();

        println!(
            "{resource}: statics {statics:?}, {} labels listed",
            labels.len()
        );

        for id in &statics {
            if *id == ABOUT_ICON {
                let icon = template.style_of(*id, "О программе: значок");

                println!("  {id}: SS_ICON, style {icon:#010x}");

                assert_eq!(
                    icon & 0x0F,
                    0x03,
                    "the about icon must stay SS_ICON — it draws no text at all, and owner \
                     drawing it would only lose the icon; the style is {icon:#010x}"
                );

                continue;
            }

            assert!(
                labels.iter().any(|(listed, _)| listed == id),
                "template {resource} carries a static {id} that task T-11-18 does not name — \
                 a label added to app.rc without being put on the list above is a label still \
                 drawn by the system, on ClearType, next to the grey-antialiased rest"
            );
        }

        // And the list holds no ghosts: every identifier it names is really a static of this
        // template. Without this half a renumbered label would silently drop out of both.
        for (id, what) in labels {
            assert!(
                statics.contains(id),
                "«{what}» ({id}) is on the T-11-18 list but is not a static of template \
                 {resource}"
            );
        }
    }
}

/// **Criterion 9 of T-11-18, the third side** — the gate the drawing stands on and the
/// template agree, control for control.
///
/// The two lists above are the test's own reading of `app.rc`; `settings::OWNER_DRAWN_LABELS`
/// and `settings::OWNER_DRAWN_ABOUT_LABELS` are the gate `WM_DRAWITEM` refuses foreign
/// identifiers with (SEC-05). A label carrying `SS_OWNERDRAW` but missing from the gate is
/// the worst of the three failures this file can catch: the system stops drawing it, the
/// program refuses to draw it, and the label goes **blank**.
#[test]
fn the_drawing_gate_and_the_templates_name_the_same_labels() {
    let mut settings_gate = settings::OWNER_DRAWN_LABELS.to_vec();
    let mut about_gate = settings::OWNER_DRAWN_ABOUT_LABELS.to_vec();

    let mut settings_template: Vec<i32> = OWNER_DRAWN_LABELS
        .iter()
        .map(|(id, _)| i32::try_from(*id).expect("a control identifier fits an i32"))
        .collect();
    let mut about_template: Vec<i32> = OWNER_DRAWN_ABOUT_LABELS
        .iter()
        .map(|(id, _)| i32::try_from(*id).expect("a control identifier fits an i32"))
        .collect();

    // Sorted before the comparison: the two orders are the template's and the source's, and
    // neither is the contract — the membership is.
    for list in [
        &mut settings_gate,
        &mut about_gate,
        &mut settings_template,
        &mut about_template,
    ] {
        list.sort_unstable();
    }

    println!("settings gate:     {settings_gate:?}");
    println!("settings template: {settings_template:?}");
    println!("about gate:        {about_gate:?}");
    println!("about template:    {about_template:?}");

    assert_eq!(
        settings_gate, settings_template,
        "settings::OWNER_DRAWN_LABELS and the SS_OWNERDRAW statics of the settings template \
         have to name the same controls — a label in one and not the other is a label that \
         nobody draws"
    );
    assert_eq!(
        about_gate, about_template,
        "settings::OWNER_DRAWN_ABOUT_LABELS and the SS_OWNERDRAW statics of the about \
         template have to name the same controls"
    );
}

/// **Criterion 11 of T-11-18** — the word wrap of the two-line labels survived the move to
/// owner drawing.
///
/// The regression this closes is silent and permanent: an `SS_LEFT` static wraps its text, a
/// `DrawTextW` without `DT_WORDBREAK` does not, and the two labels whose text does not fit on
/// one line would come back as one clipped line with the tail simply gone. Nothing would
/// error, and the window would still open.
///
/// Both halves are asserted, because either alone passes for the wrong reason: the format the
/// drawing uses **carries `DT_WORDBREAK`**, and the two labels the flag exists for are really
/// **two lines high in the template** — a `cy` quietly reduced to one line would leave a
/// correct format wrapping into a rectangle with no second line in it.
#[test]
fn the_two_line_labels_keep_their_word_wrap() {
    // The four `DT_*` of the format, spelled out here rather than imported: the point of the
    // test is that the number the product uses is the one an `SS_LEFT` static drew with, and a
    // test that imports the product's own arithmetic proves nothing about the number.
    const DT_LEFT: u32 = 0x0000_0000;
    const DT_TOP: u32 = 0x0000_0000;
    const DT_WORDBREAK: u32 = 0x0000_0010;
    const DT_EXPANDTABS: u32 = 0x0000_0040;

    let format = theme::LABEL_TEXT_FORMAT.0;

    println!("LABEL_TEXT_FORMAT = {format:#010x}");

    assert_ne!(
        format & DT_WORDBREAK,
        0,
        "the labels must be drawn with DT_WORDBREAK — task T-11-18; the format is {format:#010x}"
    );

    // And nothing else crept in: DT_CENTER, DT_RIGHT, DT_VCENTER or DT_SINGLELINE would each
    // move a caption that has not moved since task T-11-16.
    assert_eq!(
        format,
        DT_LEFT | DT_TOP | DT_WORDBREAK | DT_EXPANDTABS,
        "the format has to be exactly what an SS_LEFT static drew with — left, top, wrapping, \
         tabs expanded; it is {format:#010x}"
    );

    // The label the wrap exists for, and the height it needs for a second line. A one-line
    // label of this dialog is 9 dialog units high; this one is 16.
    //
    // Two until task Т-31-3: «вступит в силу после перезапуска» (1092, 18 units) was retired
    // with its sentence by решение 99.4, and the wrap now has exactly one customer.
    const TWO_LINE_LABELS: [(u32, &str, i32); 1] = [(1062, "путь к папке журнала", 16)];

    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    for (id, what, height) in TWO_LINE_LABELS {
        let (_, top, _, bottom) = template.rect_of(id);

        println!("«{what}» ({id}): {} dialog units high", bottom - top);

        assert_eq!(
            bottom - top,
            height,
            "«{what}» ({id}) has to keep room for its second line — the wrap has nowhere to go \
             in a one-line rectangle"
        );
    }
}

// -----------------------------------------------------------------------------------------
// Task T-11-19 — the rhythm of the layout: the window is 16 dialog units taller
// -----------------------------------------------------------------------------------------

/// The height of the window and the eight rectangles task T-11-19 moved, read out of the
/// **built** `LangSwitcher.exe` like every other statement about the template.
///
/// The measurement the task rests on: the appearance row of FR-92а grew 16 dialog units into
/// «Общие» and pushed everything below it down by the same 16, which ate the air in front of
/// «Состояние» — 53 units in the mock-ups, 37 in the product. The cure is not to move the
/// left column back but to give the window the 16 units it is short of, so the rhythm below
/// the last block is the mock-ups' again.
///
/// Two halves, and either alone would pass for the wrong reason. The numbers are asserted so
/// that a rectangle that did not move is a failing test; the four gaps are asserted so that
/// nine numbers changed in step — and not one of them typed a unit out — is what actually
/// happened.
#[test]
fn the_window_carries_the_sixteen_units_the_appearance_row_took() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    println!(
        "IDD_SETTINGS: {} x {} dialog units",
        template.size.0, template.size.1
    );

    // Width untouched — two columns 200 units wide at x = 7 and x = 213.
    //
    // ⚠ **364, and it was 385 from task T-11-19 to delivery e25.** T-11-19 gave the window
    // sixteen units so the air in front of «Состояние» would be the mock-ups' 53 again; that
    // air was spent twice afterwards — fourteen units on the sound row of FR-100 — and решение
    // 83 spent what was left: the user looked at the candidate on the stand and said the gap
    // over «Состояние» was too big, «уменьшить до стандартной величины». The standard is four
    // units, the same gap every other pair of groups keeps, and cutting 25 to 4 took 21 units
    // off the window. What T-11-19 was really about — that the rhythm below the last block is
    // a decided number and not a leftover — is what this test still holds; the number is now
    // 4 and the decision is 83 rather than В-1.
    // ⚠ The **height** is what this test is about, and it did not move: 364 units, decided by
    // решение 83 п. 3 and untouched by Т-26-3. The width went 420 → 430 (решение 87 п. 3) —
    // see the sister test for why that is a horizontal matter and this one is not.
    assert_eq!(
        template.size,
        (430, 364),
        "the settings dialog has to be 430 x 364 dialog units — решения 83 п. 3 и 87 п. 3"
    );

    // The eight rectangles that moved, in full: a `y` alone would say nothing about a width
    // or a height that drifted with it. «Диагностика» moved up with the rest of the right
    // column when «Замена» and «Выделение» left it (task Т-23-2), and its children moved
    // with it — the panel keeps its own 74 units.
    //
    // ⚠ **Every x below is ten units larger since решение 87 п. 3** (task Т-26-3), and every
    // `y`, width and height is exactly what it was. That is the whole shape of the change and
    // the reason it is safe: the right column was **moved**, not re-cut — «Диагностика» keeps
    // its own 200 × 74 — and the two full-width rows grew by the ten units the window grew by.
    const MOVED: [(u32, &str, i32, i32, i32, i32); 8] = [
        (1106, "Диагностика: панель", 223, 215, 200, 74),
        (1108, "Состояние: панель", 7, 293, 416, 44),
        (1070, "Состояние: перехватчик", 14, 305, 402, 9),
        (1071, "Состояние: раскладки", 14, 316, 402, 9),
        (1072, "Состояние: автозапуск", 14, 327, 402, 9),
        (1, "ОК", 263, 343, 50, 14),
        (2, "Отмена", 317, 343, 50, 14),
        (1080, "Применить", 371, 343, 52, 14),
    ];

    for (id, what, x, y, cx, cy) in MOVED {
        let measured = template.rect_of(id);

        println!("«{what}» ({id}): {measured:?}");

        assert_eq!(
            measured,
            (x, y, x + cx, y + cy),
            "«{what}» ({id}) is not where task T-11-19 puts it"
        );
    }

    // The children of «Диагностика» moved by exactly what the panel moved by — 21 units up,
    // task Т-23-2 — and by nothing else: the panel is the same 74 units it always was, and the
    // four rows inside it keep their own steps. Asserted as absolute numbers rather than as a
    // delta, because a delta would pass for a block that had been re-cut and shifted back.
    for (id, what, bottom) in [
        (1060u32, "Вести журнал", 237i32),
        (1061, "Открыть папку журнала", 239),
        (1107, "Папка журнала:", 252),
        (1062, "путь к папке журнала", 268),
    ] {
        let (_, _, _, measured) = template.rect_of(id);

        assert_eq!(
            measured, bottom,
            "«{what}» ({id}) is not where the move of task Т-23-2 puts it — the panel moved \
             as a whole, its children did not drift inside it"
        );
    }

    // **The rhythm.** Four gaps, every one of them arithmetic on the numbers above, and every
    // one of them what the mock-ups measure.
    let (.., layouts_bottom) = template.rect_of(1095);
    let (.., diagnostics_bottom) = template.rect_of(1106);
    let (_, state_top, _, state_bottom) = template.rect_of(1108);
    let (_, buttons_top, _, buttons_bottom) = template.rect_of(1);

    let air_left = state_top - layouts_bottom;
    let air_right = state_top - diagnostics_bottom;
    let air_buttons = buttons_top - state_bottom;
    let margin = template.size.1 - buttons_bottom;

    println!(
        "воздух: под «Раскладками» {air_left}, под «Диагностикой» {air_right}, \
         перед кнопками {air_buttons}, поле снизу {margin}"
    );

    // ⚠ **4, and it was 53 at task T-11-19 and 39 after the sound row of FR-100 took fourteen
    // of them.** Решение 83 п. 3 spent the rest: «расстояние между блоком состояние и верхними
    // уменьшить до стандартной величины». The left column is no longer the odd one — both
    // columns now end at 289 and both stand four units over «Состояние», which is the gap
    // between every other pair of groups in this window. What T-11-19 really fixed is asserted
    // above and below this line and is untouched: the rhythm below the last block is a decided
    // number, the buttons keep their 6 and the margin its 7.
    assert_eq!(
        air_left, 4,
        "the air under «Раскладки» is the standard gap of this dialog — решение 83 п. 3"
    );
    assert_eq!(
        air_right, 4,
        "the air under «Диагностика» has to stay the 4 units of the mock-ups"
    );
    assert_eq!(
        air_buttons, 6,
        "the air between «Состояние» and the buttons has to stay the 6 units it was"
    );
    assert_eq!(
        margin, 7,
        "the margin under the buttons has to stay the 7 units it was"
    );
}

// -----------------------------------------------------------------------------------------
// Task T-11-13 — the defect, and the mock-ups
// -----------------------------------------------------------------------------------------

/// `WS_VISIBLE`. rc.exe ORs it into every control statement of a dialog whatever the style
/// expression says; the only way to take it back off is the `NOT` operator of the
/// expression, which is what `app.rc` now does for the six panels.
const WS_VISIBLE: u32 = 0x1000_0000;

/// `WS_BORDER` — the sunken system rectangle. rc.exe adds this one to `EDITTEXT` and
/// `LISTBOX` on top of `WS_VISIBLE`, and the same `NOT` takes it off.
const WS_BORDER: u32 = 0x0080_0000;

/// `WS_CLIPSIBLINGS` — the style the hypothesis of task T-11-13 named. Not a single control
/// of this template has ever carried it; see the test below.
const WS_CLIPSIBLINGS: u32 = 0x0400_0000;

/// The six group panels, by identifier and caption — the same six
/// `settings::GROUP_BOXES` lists, written out as literals by the rule the other template
/// tests follow. Eight until task Т-23-2.
const PANELS: [(u32, &str); 6] = [
    (1090, "Общие"),
    (1093, "Горячая клавиша"),
    (1095, "Раскладки"),
    (1104, "Исключения"),
    (1106, "Диагностика"),
    (1108, "Состояние"),
];

/// **Criterion 10 of T-11-13, and the two-sided detector of the defect** — read out of the
/// **built** `LangSwitcher.exe`, like every other statement about the template.
///
/// The defect: a click anywhere on the empty ground of a block emptied the whole block. The
/// hypothesis named the cause — the panel is an owner-drawn `Button` covering the whole area
/// of its block, so the click lands on it, it redraws itself, and nothing clips its fill away
/// from its neighbours, which are never invalidated. This test measures every premise of that
/// hypothesis that can be measured without a window, and then asserts the cure: the panel is
/// **not a visible element any more**, so no click can reach it and it paints nothing.
///
/// Before the fix the same test failed on the `WS_VISIBLE` assertion with `0x5000000b`
/// printed for all eight — that is the «ДО» half of the detector, and the «ПОСЛЕ» half is
/// this test passing with `0x4000000b`. The live half — the click that used to empty the
/// block and no longer does — is the controller's, on a debug build of his own.
#[test]
fn the_eight_panels_are_not_elements_that_can_swallow_their_block() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    // Premise the hypothesis names: nothing in this template clips siblings. Measured over
    // *every* control, not only the panels — the style is absent from the whole dialog, so
    // no drawing of any control is clipped away from any other.
    for (id, style) in &template.styles {
        assert_eq!(
            style & WS_CLIPSIBLINGS,
            0,
            "control {id} carries WS_CLIPSIBLINGS ({style:#010x}); the measurement of task \
             T-11-13 recorded that not one control of this dialog does"
        );
    }

    for (id, what) in PANELS {
        let style = template.style_of(id, what);
        let (left, top, right, bottom) = template.rect_of(id);

        // Premise: the panel is an owner-drawn Button — a class that takes clicks and
        // redraws itself on them. Still true after the fix, and harmless now.
        assert_eq!(
            style & 0x0F,
            0x0B,
            "«{what}» ({id}) must keep BS_OWNERDRAW; the style is {style:#010x}"
        );

        // Premise: the panel covers the whole area of its block — it overlaps the controls
        // of the block, and there is ground inside it that belongs to no other control, so
        // a click on the empty part of the block can only land on the panel.
        let mut overlapped = Vec::new();

        for (other, x, y, cx, cy) in &template.bounds {
            if *other == id {
                continue;
            }

            if *x < right && x + cx > left && *y < bottom && y + cy > top {
                overlapped.push(*other);
            }
        }

        let mut bare = 0usize;

        for y in top..bottom {
            for x in left..right {
                let covered = template.bounds.iter().any(|(other, ox, oy, cx, cy)| {
                    *other != id && *ox <= x && x < ox + cx && *oy <= y && y < oy + cy
                });

                if !covered {
                    bare += 1;
                }
            }
        }

        println!(
            "«{what}» ({id}): style {style:#010x}, rect {left},{top}..{right},{bottom}, \
             overlaps {} controls, {bare} dialog units of bare ground",
            overlapped.len()
        );

        assert!(
            !overlapped.is_empty(),
            "«{what}» ({id}) overlaps nothing — the premise of the defect is not what was \
             measured"
        );
        assert!(
            bare > 0,
            "«{what}» ({id}) has no ground of its own — the premise of the defect is not \
             what was measured"
        );

        // **The cure.** The panel is not a visible element: an invisible window takes no
        // click, sends no `WM_DRAWITEM`, paints nothing and covers nobody. The rectangle
        // and the identifier stay — the background drawing reads both off this very
        // control.
        assert_eq!(
            style & WS_VISIBLE,
            0,
            "«{what}» ({id}) is still a visible element — the defect of task T-11-13 is \
             back; the style is {style:#010x}"
        );
    }
}

/// **Criterion 13 of T-11-13** — п. 2.3: no field and no list keeps the system border.
///
/// The system border is a sunken rectangle in the system's colours, which is not what the
/// mock-ups show; the rounded `field_border` frame the dialog draws instead lives one pixel
/// outside each of these rectangles, in the dialog's own background drawing. Read out of the
/// built binary, and everything else about each style has to survive — a lost `WS_VSCROLL`
/// or `WS_TABSTOP` would cost the control its scroll bar or its place in the keyboard loop.
#[test]
fn no_field_or_list_of_the_dialog_carries_the_system_border() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    // The four of `settings::FRAMED_FIELDS`, by their actual identifiers; the third column
    // is the bit that must still be there next to the border that must not. Seven until
    // task Т-23-2 took the three `ES_NUMBER` millisecond fields out of the window.
    const FIELDS: [(u32, &str, u32, &str); 4] = [
        (1010, "поле горячей клавиши", 0x0001_0000, "WS_TABSTOP"),
        (1051, "имя процесса", 0x0080, "ES_AUTOHSCROLL"),
        (1050, "список исключений", 0x0020_0000, "WS_VSCROLL"),
        (1024, "список раскладок", 0x0001, "LVS_REPORT"),
    ];

    for (id, what, kept, kept_name) in FIELDS {
        let style = template.style_of(id, what);

        println!("«{what}» ({id}): style {style:#010x}");

        assert_eq!(
            style & WS_BORDER,
            0,
            "«{what}» ({id}) still carries WS_BORDER — task T-11-13, п. 2.3; the style is \
             {style:#010x}"
        );

        assert_ne!(
            style & kept,
            0,
            "«{what}» ({id}) lost {kept_name} along with the border; the style is \
             {style:#010x}"
        );
    }

    // And the seven are the seven the module names: a field added to the template without
    // being added to `FRAMED_FIELDS` would keep a system border nobody draws a frame for.
    let listed: Vec<i32> = FIELDS.iter().map(|(id, ..)| *id as i32).collect();
    let mut mine = settings::FRAMED_FIELDS.to_vec();
    let mut theirs = listed.clone();
    mine.sort_unstable();
    theirs.sort_unstable();

    assert_eq!(
        mine, theirs,
        "the module's list of framed fields and the template's have parted"
    );
}

/// **Criterion 11 of T-11-13** — п. 2.1: the caption the mock-ups show is the upper case of
/// the control's text, and the space inside a caption survives the trip to the pen.
#[test]
fn a_panel_caption_is_the_upper_case_of_the_control_text() {
    // The six captions of the dialog, in both locales — the strings FR-94 puts on the
    // six controls, and what the background drawing must make of them. Eight until task
    // Т-23-2 took «Замена» and «Выделение» off the window.
    for (source, expected) in [
        ("Общие", "ОБЩИЕ"),
        ("Горячая клавиша", "ГОРЯЧАЯ КЛАВИША"),
        ("Раскладки", "РАСКЛАДКИ"),
        ("Исключения", "ИСКЛЮЧЕНИЯ"),
        ("Диагностика", "ДИАГНОСТИКА"),
        ("Состояние", "СОСТОЯНИЕ"),
        ("General", "GENERAL"),
        ("Hotkey", "HOTKEY"),
        ("Diagnostics", "DIAGNOSTICS"),
        ("", ""),
    ] {
        assert_eq!(
            settings::panel_caption(source),
            expected,
            "«{source}» must be set as «{expected}»"
        );
    }

    // The space, said out loud: the trap of п. 2.1 is a caption drawn character by
    // character whose blank went missing, and «ГОРЯЧАЯКЛАВИША» is what that looks like.
    let two_words = settings::panel_caption("Горячая клавиша");
    assert!(
        two_words.contains(' '),
        "the space inside a caption must survive: «{two_words}»"
    );
    assert_ne!(two_words, "ГОРЯЧАЯКЛАВИША");
}

/// **Criterion 11 of T-11-13, the second half** — the blank that measures as nothing still
/// takes room on the line.
#[test]
fn a_character_the_device_measures_as_nothing_still_takes_room() {
    // A measured character keeps its own width whatever the font height is.
    assert_eq!(theme::caption_advance(9, 12), 9);
    assert_eq!(theme::caption_advance(1, 12), 1);

    // A blank — measured as zero, or as a negative by a refused measurement — takes the
    // explicit width instead, and that width is never zero for a font of any real size.
    for height in [10, 12, 16, 24] {
        let blank = theme::caption_advance(0, height);

        assert!(
            blank > 0,
            "a blank in a {height} px face must take room, not {blank}"
        );
        assert!(
            blank < height,
            "a blank in a {height} px face must be narrower than the face is tall, not \
             {blank}"
        );
        assert_eq!(theme::caption_advance(-1, height), blank);
    }
}

/// **Criterion 10 of T-11-16; criterion 12 of T-11-13** — one radius, one name, five figures.
///
/// The generator of the mock-ups keeps the radius in its style table (`Radius = 6` for both
/// approved styles) and hands that same `$S.Radius` to `'group'`, `'edit'`, `'combo'`, `'btn'`,
/// `'def'`, `'lbox'` and `'lview'` alike. Tasks T-11-13 and T-11-14 split it into «панель 6 /
/// прочее 4» off a reading of the picture; this test is what holds the split from coming back.
#[test]
fn one_radius_rounds_the_panel_the_field_the_combo_the_button_and_both_lists() {
    assert_eq!(
        theme::CORNER_RADIUS,
        6,
        "the `Radius = 6` of the style table of ui.ps1"
    );

    // ⚠ `drawing_source()` and not `settings_module_source()` since task T-14-5: `CORNER_RADIUS`
    // itself moved to `theme` with the drawing library (task T-14-3), so the natural place for a
    // second radius to come back is a file this test used to be blind to. The join restores the
    // reach the check had before the move — both halves of the drawing are searched, and the
    // call-site rows below still read the bodies that stayed in `settings`.
    let source = drawing_source();

    // The three names of T-11-13 are gone, not renamed around: a second radius could only come
    // back as a second constant.
    for gone in [
        "PANEL_CORNER_RADIUS",
        "BUTTON_CORNER_RADIUS",
        "FIELD_CORNER_RADIUS",
    ] {
        assert!(
            !source.contains(gone),
            "`{gone}` must be gone — the mock-ups have one radius, not three"
        );
    }

    // And the five places round off by the one name that is left. Each is named by the function
    // that owns the figure, so a call site that quietly went back to a number of its own takes
    // its row down with it.
    let places = [
        // панель и оба списка — обе половины фона диалога. ⚠ Тело переехало из
        // `on_erase_background` в `paint_background` задачей T-11-17: обработчик сообщения
        // теперь отдаёт готовую картинку, а рисует фон эта функция. Существо строки то же —
        // обе половины фона скругляются одним именем.
        ("unsafe fn paint_background(", 2),
        // кнопка
        ("unsafe fn paint_push_button(", 1),
        // закрытая часть комбобокса, она же поле на вид
        ("unsafe fn draw_combo_closed_part(", 1),
    ];

    for (signature, times) in places {
        let body = function_body(&source, signature);

        assert_eq!(
            body.matches("scaled(CORNER_RADIUS,").count(),
            times,
            "`{signature}` must round off by the one radius, {times} time(s)"
        );
    }

    // The two figures that keep a radius of their own, because the generator gives them one:
    // the square of a dialog check box and the smaller square of a layout-list tick.
    assert_eq!(
        settings::GLYPH_CORNER_RADIUS,
        3,
        "the `… $S.Mark 3` of 'check'"
    );
    assert_eq!(
        settings::LIST_CHECK_CORNER_RADIUS,
        2,
        "the `… $S.Mark 2` of 'lview'"
    );
}

/// **Criterion 12 of T-11-13, the DPI half; criterion 9 of T-11-15** — every length of the
/// mock-ups is a length *of the pictures*, and the window scales it.
///
/// The pictures are drawn at 140 %: `scratchpad\ui.ps1`, line 7, `$DPI = 1.4`. So the scale
/// divides by 96 × 1,4 = 134,4 DPI and not by 96, and a length of the picture comes out
/// **smaller** on a 100 % screen — which is the whole of task T-11-15.
#[test]
fn the_lengths_of_the_mock_ups_scale_with_the_dpi_of_the_window() {
    // The scale itself, in the shape the module writes it: tenths of a DPI, because 134,4 is
    // not whole, and the factor named apart so the source of the number stays visible.
    assert_eq!(theme::SCREEN_DPI, 96, "100 % is 96 DPI");
    assert_eq!(theme::MOCKUP_SCALE_TENTHS, 14, "the `$DPI = 1.4` of ui.ps1");
    assert_eq!(
        theme::MOCKUP_DPI_TENTHS,
        1344,
        "96 × 1,4 = 134,4 DPI, carried in tenths"
    );
    assert_eq!(
        theme::MOCKUP_DPI_TENTHS,
        theme::SCREEN_DPI * theme::MOCKUP_SCALE_TENTHS,
        "the mock-up DPI must stay 96 × 1,4 and not a number of its own"
    );

    // 100 % — the picture divided by 1,4, rounded to nearest.
    assert_eq!(theme::scaled(6, 96), 4);
    assert_eq!(theme::scaled(4, 96), 3);

    // 125 %, 150 %, 200 % — rounded to nearest, so a one-pixel frame never rounds away.
    assert_eq!(theme::scaled(6, 120), 5);
    assert_eq!(theme::scaled(4, 120), 4);
    assert_eq!(theme::scaled(6, 144), 6);
    assert_eq!(theme::scaled(4, 192), 6);
    assert_eq!(theme::scaled(1, 120), 1);
    assert_eq!(theme::scaled(1, 144), 1);

    // The letter spacing is carried in tenths of a pixel, and scales as a tenth does.
    assert_eq!(theme::scaled(11, 96), 8);
    assert_eq!(theme::scaled(11, 192), 16);

    // A device that will not say what its DPI is gets the 100 % look, and — since T-11-15 —
    // that is the *scaled* 100 % look and not the mock-up number handed over unchanged.
    assert_eq!(theme::scaled(6, 0), 4);
    assert_eq!(theme::scaled(6, -1), 4);
    assert_eq!(theme::scaled_tenths(15, 0), theme::scaled_tenths(15, 96));

    // The scale is one division and nothing else, and at 96 DPI it is an exact fraction:
    // 96 / 134,4 = **5 / 7**. Every mock-up length must therefore come out as the nearest
    // whole of five sevenths of itself — written here as arithmetic that does not go anywhere
    // near the module's own formula.
    for pixels in [1, 2, 3, 4, 5, 6, 8, 11, 12, 16, 33] {
        let five_sevenths = (pixels * 10 + 7) / 14;

        assert_eq!(
            theme::scaled(pixels, 96),
            five_sevenths,
            "{pixels} px of the mock-ups must be {five_sevenths} px at 96 DPI"
        );
    }
}

/// One row of `scratchpad\design-tokens.md` — task T-11-16, criterion 9.
struct Token {
    /// Раздел и название величины, как их пишет эталон.
    what: &'static str,
    /// Как эталон записывает величину — для вывода в отчёт теста.
    reference: &'static str,
    /// Пары «величина продукта при 96 DPI в сотых пикселя, эталон в них же». Пустой список —
    /// у строки эталона нет числа (например «текст по центру»).
    pixels: Vec<(i32, i32)>,
    /// Пары «величина продукта, эталон» для того, что от DPI не зависит и переносится как
    /// есть: единицы диалога и отношение кеглей.
    exact: Vec<(i32, i32)>,
    /// Кусок исходника, которым держится строка без числа.
    shape: Option<&'static str>,
}

/// The reference length at 96 DPI, in hundredths of a pixel: the number the generator writes,
/// given in **tenths of a mock-up pixel**, divided by the 1,4 the pictures were drawn at.
fn at_96(mockup_tenths: i32) -> i32 {
    // tenths of a mock-up pixel → hundredths of a screen pixel: × 100 ÷ 10 ÷ 1,4 = × 100 ÷ 14.
    mockup_tenths * 100 / 14
}

/// A whole screen pixel in hundredths, for the left-hand column.
fn px(pixels: i32) -> i32 {
    pixels * 100
}

/// **Criterion 9 of T-11-16 — the table the task is about**: every row of
/// `scratchpad\design-tokens.md` against the constant of the product that answers it.
///
/// The reference is not a measurement of the pictures: `scratchpad\design-tokens.md` was
/// extracted from `scratchpad\ui.ps1`, the generator that *drew* them, where every size is a
/// literal. So each row here is «what the generator writes» against «what this dialog puts on
/// a 96 DPI screen», and the two must agree to within **half a pixel** — the tolerance the
/// task names.
///
/// Rows: 6 + 6 + 3 + 5 + 3 + 6 + 8 = **37**, one for every row of the seven tables of the
/// reference. The count is asserted, so a row of the reference cannot quietly go missing.
#[test]
fn every_row_of_the_reference_meets_the_constant_that_answers_it() {
    // The height of a row of the layout list at 96 DPI: 12 vertical dialog units, which
    // `MapDialogRect` maps as MulDiv(12, 15, 8) for the 15-pixel base unit of Segoe UI 9 pt.
    let layout_row = 12 * 15 / 8;
    let cell = settings::check_cell(96, layout_row);
    let glyph_side = theme::scaled(settings::GLYPH_SIZE, 96);
    let dot_inset = theme::scaled_tenths_offset(settings::GLYPH_DOT_INSET_TENTHS, 96);
    // Task Т-30-3 gave this a rectangle and a mirror flag: the figure is the same one, asked
    // for in a square at the origin and the way round every left-to-right locale draws it.
    let glyph_mark = theme::check_mark_points(
        &RECT {
            left: 0,
            top: 0,
            right: glyph_side,
            bottom: glyph_side,
        },
        settings::GLYPH_CHECK_MARK,
        96,
        false,
    );

    // The radius and the frame are one number for five figures each, so the three rows that
    // ask for them in three different tables are answered by the same two expressions.
    let radius = || vec![(px(theme::scaled(theme::CORNER_RADIUS, 96)), at_96(60))];
    // ⚠ The frame is the one row whose «при 96 DPI» column the reference does not divide: a
    // pen has no fractional width, and `.max(1)` is what keeps the single pixel at every scale.
    let border = || vec![(px(theme::scaled(theme::BORDER_THICKNESS, 96).max(1)), px(1))];

    let token = |what, reference, pixels: Vec<(i32, i32)>| Token {
        what,
        reference,
        pixels,
        exact: Vec::new(),
        shape: None,
    };
    let units = |what, reference, exact: Vec<(i32, i32)>| Token {
        what,
        reference,
        pixels: Vec::new(),
        exact,
        shape: None,
    };

    let table: Vec<Token> = vec![
        // ---------------------------------------------------------------- 1. Панель группы
        token("1. Панель · Радиус", "6 px макета", radius()),
        token("1. Панель · Толщина рамки", "1 px макета", border()),
        units(
            "1. Панель · Отступ заголовка слева",
            "7 DLU",
            vec![(settings::PANEL_CAPTION_INSET_DLU, 7)],
        ),
        // ⚠ 5 → 11 by решение 90: the generator's 5 pressed the caption against the panel's
        // top edge (4 px above it, 13 px below it to the first control). 11 mock-up pixels are
        // 8 screen pixels at 96 DPI and put the heading between the two instead of on the edge.
        // The reference moves with the constant because it *is* the decision, not a reading of
        // the Э11 picture any more — same as the caption's point size in решение 89.
        token(
            "1. Панель · Отступ заголовка сверху",
            "11 px макета",
            vec![(
                px(theme::scaled(settings::PANEL_CAPTION_INSET_Y, 96)),
                at_96(110),
            )],
        ),
        token(
            "1. Панель · Разрядка заголовка",
            "1,1 px макета",
            // The tracking is carried in tenths of a *screen* pixel: ten hundredths each.
            vec![(
                theme::scaled(settings::PANEL_CAPTION_TRACKING_TENTHS, 96) * 10,
                at_96(11),
            )],
        ),
        // ⚠ 76 → 83 by решение 89. The Э11 generator's 7,6 pt was a ratio against a 9 pt body;
        // both windows are set one step larger since решения 85 и 87, and this face was the one
        // thing left behind — measured at `lfHeight` −10 in both while their own faces went to
        // −13 and −12. The reference is now the accepted Э23 mock-up, whose `.helpcap` is
        // 10,5 px against a 12,5 px row: 0,84, and 8,3 / 9 is 0,922 of the **9 pt** base, which
        // lands the caption at −11 — 11 / 13 = 0,846 of the body it actually stands next to.
        units(
            "1. Панель · Кегль заголовка",
            "8,3 pt против 9 pt = 0,922 основного, в промилле",
            vec![(
                settings::PANEL_CAPTION_POINTS_TENTHS * 1000 / settings::DIALOG_FONT_POINTS_TENTHS,
                83 * 1000 / 90,
            )],
        ),
        // ------------------------------------------------- 2. Флажок и переключатель
        token(
            "2. Глиф · Сторона квадрата / диаметр круга",
            "17 px макета",
            vec![(px(glyph_side), at_96(170))],
        ),
        token(
            "2. Глиф · Радиус скругления квадрата",
            "3 px макета",
            vec![(
                px(theme::scaled(settings::GLYPH_CORNER_RADIUS, 96)),
                at_96(30),
            )],
        ),
        token(
            "2. Глиф · Толщина пера галочки",
            "2,1 px макета",
            vec![(
                px(theme::scaled_tenths(
                    settings::GLYPH_CHECK_MARK.pen_tenths,
                    96,
                )),
                at_96(21),
            )],
        ),
        token(
            "2. Глиф · Точки галочки от угла квадрата",
            "(4,5; 8,6) (7,3; 11,8) (12,5; 5,2)",
            vec![
                (px(glyph_mark[0].0), at_96(45)),
                (px(glyph_mark[0].1), at_96(86)),
                (px(glyph_mark[1].0), at_96(73)),
                (px(glyph_mark[1].1), at_96(118)),
                (px(glyph_mark[2].0), at_96(125)),
                (px(glyph_mark[2].1), at_96(52)),
            ],
        ),
        token(
            "2. Глиф · Втяжка точки переключателя и диаметр точки",
            "4,6 / 7,8 px макета",
            vec![
                (px(dot_inset), at_96(46)),
                // The diameter is not a number of its own — it falls out of the circle and the
                // inset, exactly as `($bs-9.2)` falls out of them in the generator.
                (px(glyph_side - 2 * dot_inset), at_96(78)),
            ],
        ),
        units(
            "2. Глиф · Текст от левого края элемента",
            "12 DLU",
            vec![(settings::GLYPH_TEXT_INSET_DLU, 12)],
        ),
        // ---------------------------------------------------------------- 3. Поле ввода
        token("3. Поле · Радиус", "6 px макета", radius()),
        token("3. Поле · Толщина рамки", "1 px макета", border()),
        units(
            "3. Поле · Втяжка текста",
            "3 DLU",
            vec![(settings::FIELD_TEXT_INSET_DLU, 3)],
        ),
        // ----------------------------------------------------------------- 4. Комбобокс
        token(
            "4. Комбобокс · Радиус, рамка",
            "6 / 1 px макета",
            vec![radius()[0], border()[0]],
        ),
        units(
            "4. Комбобокс · Втяжка текста",
            "3 DLU",
            vec![(settings::FIELD_TEXT_INSET_DLU, 3)],
        ),
        token(
            "4. Комбобокс · Центр шеврона от правого края",
            "14 px макета",
            vec![(
                px(theme::scaled(theme::COMBO_CHEVRON_INSET_X, 96)),
                at_96(140),
            )],
        ),
        token(
            "4. Комбобокс · Толщина пера шеврона",
            "1,5 px макета",
            vec![(
                px(theme::scaled_tenths(theme::COMBO_CHEVRON_PEN_TENTHS, 96)),
                at_96(15),
            )],
        ),
        token(
            "4. Комбобокс · Плечо шеврона",
            "±4 по x, ∓2 по y px макета",
            vec![
                (px(theme::scaled(theme::COMBO_CHEVRON_ARM_X, 96)), at_96(40)),
                (px(theme::scaled(theme::COMBO_CHEVRON_ARM_Y, 96)), at_96(20)),
            ],
        ),
        // ------------------------------------------------------------------- 5. Кнопка
        token("5. Кнопка · Радиус", "6 px макета", radius()),
        token("5. Кнопка · Толщина рамки", "1 px макета", border()),
        Token {
            what: "5. Кнопка · Текст",
            reference: "по центру",
            pixels: Vec::new(),
            exact: Vec::new(),
            shape: Some("DT_CENTER | DT_VCENTER | DT_SINGLELINE"),
        },
        // ------------------------------------------------------ 6. Список исключений
        token(
            "6. Список исключений · Радиус рамки списка",
            "6 px макета",
            radius(),
        ),
        units(
            "6. Список исключений · Высота строки",
            "11 DLU",
            vec![(settings::EXCLUSION_ROW_HEIGHT_DLU, 11)],
        ),
        token(
            "6. Список исключений · Первая строка от верха рамки",
            "3 px макета",
            vec![(px(theme::scaled(theme::LIST_FIRST_ROW_TOP, 96)), at_96(30))],
        ),
        token(
            "6. Список исключений · Втяжка прямоугольника выделения и его радиус",
            "2 / 3 px макета",
            vec![
                (
                    px(theme::scaled(theme::LIST_SELECTION_INSET, 96)),
                    at_96(20),
                ),
                (
                    px(theme::scaled(theme::LIST_SELECTION_RADIUS, 96)),
                    at_96(30),
                ),
            ],
        ),
        token(
            "6. Список исключений · Втяжка текста от левого края списка",
            "7 px макета",
            vec![(px(theme::scaled(settings::LIST_TEXT_INSET, 96)), at_96(70))],
        ),
        token(
            "6. Список исключений · Текст ниже верха строки",
            "2 px макета",
            vec![(px(theme::scaled(settings::LIST_TEXT_TOP, 96)), at_96(20))],
        ),
        // -------------------------------------------------------- 7. Список раскладок
        token(
            "7. Список раскладок · Радиус рамки",
            "6 px макета",
            radius(),
        ),
        units(
            "7. Список раскладок · Высота строки",
            "12 DLU",
            vec![(settings::LAYOUT_ROW_HEIGHT_DLU, 12)],
        ),
        token(
            "7. Список раскладок · Сторона галочки",
            "13 px макета",
            vec![(px(cell.glyph_side), at_96(130))],
        ),
        token(
            "7. Список раскладок · Галочка от левого края списка",
            "7 px макета",
            vec![(px(cell.glyph_left), at_96(70))],
        ),
        token(
            "7. Список раскладок · Радиус скругления галочки",
            "2 px макета",
            vec![(
                px(theme::scaled(settings::LIST_CHECK_CORNER_RADIUS, 96)),
                at_96(20),
            )],
        ),
        token(
            "7. Список раскладок · Толщина пера галочки",
            "1,8 px макета",
            vec![(
                px(theme::scaled_tenths(
                    settings::LIST_CHECK_MARK.pen_tenths,
                    96,
                )),
                at_96(18),
            )],
        ),
        token(
            "7. Список раскладок · Текст после галочки",
            "+7 px макета",
            vec![(
                px(cell.width - cell.glyph_left - cell.glyph_side),
                at_96(70),
            )],
        ),
        token(
            "7. Список раскладок · Втяжка выделения и её радиус",
            "2 / 3 px макета",
            vec![
                (
                    px(theme::scaled(theme::LIST_SELECTION_INSET, 96)),
                    at_96(20),
                ),
                (
                    px(theme::scaled(theme::LIST_SELECTION_RADIUS, 96)),
                    at_96(30),
                ),
            ],
        ),
    ];

    // Криterion 9, the count: 6 + 6 + 3 + 5 + 3 + 6 + 8 rows of the seven tables of
    // `scratchpad\design-tokens.md`, and not one of them dropped on the way here.
    assert_eq!(
        table.len(),
        6 + 6 + 3 + 5 + 3 + 6 + 8,
        "the table must hold one row per row of scratchpad\\design-tokens.md"
    );

    // Half a pixel at 96 DPI — the tolerance the task names, in the hundredths this table
    // counts in.
    const TOLERANCE: i32 = 50;

    let source = settings_module_source();

    for row in &table {
        assert!(
            !row.pixels.is_empty() || !row.exact.is_empty() || row.shape.is_some(),
            "{}: a row of the reference must be answered by something",
            row.what
        );

        for (product, reference) in &row.pixels {
            let off = (product - reference).abs();

            println!(
                "{}: эталон {} = {},{:02} px при 96 DPI · продукт {},{:02} px · расхождение \
                 {},{:02}",
                row.what,
                row.reference,
                reference / 100,
                reference % 100,
                product / 100,
                product % 100,
                off / 100,
                off % 100
            );

            assert!(
                off <= TOLERANCE,
                "{}: эталон {},{:02} px при 96 DPI, продукт {},{:02} px — расхождение \
                 {},{:02} px больше половины пикселя",
                row.what,
                reference / 100,
                reference % 100,
                product / 100,
                product % 100,
                off / 100,
                off % 100
            );
        }

        for (product, reference) in &row.exact {
            println!(
                "{}: эталон {} = {reference} · продукт {product}",
                row.what, row.reference
            );

            assert_eq!(
                product, reference,
                "{}: величина не зависит от DPI и обязана совпадать точно",
                row.what
            );
        }

        if let Some(shape) = row.shape {
            println!("{}: эталон {} · продукт `{shape}`", row.what, row.reference);

            assert!(
                source.contains(shape),
                "{}: the module must still draw it as `{shape}`",
                row.what
            );
        }
    }
}

/// **Criterion 11 of T-11-16** — each inset is expressed in the unit the generator states it
/// in, and reaches every place that needs it in that unit.
///
/// Read out of the module's own source, in the manner of the sweeps of `tests\guard.rs`: what
/// is asserted here is not a value but a shape — that no call site went back to a bare number,
/// and that nobody turned a dialog unit into a mock-up pixel on the way.
#[test]
fn every_inset_is_written_in_the_unit_the_generator_states_it_in() {
    // T-14-3: the drawing is two files now — see `drawing_source`.
    let source = drawing_source();

    // Criterion 9 of T-11-15, carried forward: the source of the scale is named where it is.
    let scale = source
        .split_once("pub const MOCKUP_SCALE_TENTHS")
        .expect("the module must still declare the mock-up scale")
        .0;

    for named in ["scratchpad\\ui.ps1", "$DPI = 1.4", "140 %"] {
        assert!(
            scale.contains(named),
            "the comment on the mock-up scale must name `{named}` — the source of the number"
        );
    }

    // Dialog units — `($c.x + 3)` of the `'edit'` and `'combo'` arms, and `($c.x + 12)` of the
    // `'check'` and `'radio'` arms. Each goes through `dialog_units`, never through `scaled`.
    let in_units = [
        ("fn set_field_margins(", "FIELD_TEXT_INSET_DLU"),
        ("unsafe fn draw_combo_closed_part(", "FIELD_TEXT_INSET_DLU"),
        ("unsafe fn draw_combo_item(", "FIELD_TEXT_INSET_DLU"),
        ("unsafe fn draw_glyph_element(", "GLYPH_TEXT_INSET_DLU"),
        ("unsafe fn draw_panel_caption(", "PANEL_CAPTION_INSET_DLU"),
    ];

    for (signature, constant) in in_units {
        let body = function_body(&source, signature);

        assert!(
            body.contains("dialog_units("),
            "`{signature}` must map its inset with MapDialogRect and not with the mock-up scale"
        );
        assert!(
            body.contains(constant),
            "`{signature}` must take its inset from `{constant}`"
        );
        assert!(
            !body.contains(&format!("scaled({constant}")),
            "`{constant}` is a dialog unit — putting it through the mock-up scale would be a \
             second conversion of an already-scaled length"
        );
    }

    // Mock-up pixels — `($px + 7)` and `($ry + 2)` of the `'lbox'` arm, `$bx = $px + 7` and
    // `($bx + $bs + 7)` of the `'lview'` arm. Each goes through `scaled`, never through
    // `dialog_units`.
    let in_pixels = [
        ("unsafe fn draw_list_item(", "scaled(LIST_TEXT_INSET"),
        ("unsafe fn draw_list_item(", "scaled(LIST_TEXT_TOP"),
        ("pub fn check_cell(", "scaled(LIST_TEXT_INSET"),
        ("pub fn check_cell(", "scaled(LIST_CHECK_TEXT_GAP"),
        // ⚠ `list_frame_air` и не `paint_background`: тело переехало задачей T-12-5, ровно как
        // задачей T-11-17 оно переехало из `on_erase_background` в `paint_background`
        // (см. пояснение в `one_radius_rounds_…`). Проверяемое — то же: втяжка идёт через
        // `scaled`, а не литералом. Переехало потому, что с T-12-5 ту же рамку называет второй
        // вызывающий — `list_frame_box`, изнутри самого списка, — и арифметика вынесена в одну
        // чистую функцию, чтобы две копии не разошлись на пиксель.
        ("pub fn list_frame_air(", "scaled(LIST_FIRST_ROW_TOP"),
    ];

    for (signature, call) in in_pixels {
        let body = function_body(&source, signature);

        assert!(
            body.contains(call),
            "`{signature}` must take its inset as a mock-up pixel through `{call}…`"
        );
    }

    // …и цепочка от той функции до фона не порвана: `paint_background` берёт обе величины
    // оттуда и ниоткуда больше. Без этой проверки первая половина осталась бы верной, а фон
    // мог бы вернуться к собственному литералу — то самое расхождение, ради которого
    // арифметику и вынесли (T-12-5).
    let background = function_body(&source, "unsafe fn paint_background(");

    assert!(
        background.contains("list_frame_air(dpi)"),
        "`paint_background` must take the air above a list and the thickness around it from \
         `list_frame_air` — the one place either number is worked out"
    );
    assert!(
        !background.contains("scaled(LIST_FIRST_ROW_TOP"),
        "and must not work the inset out a second time: a copy of that arithmetic is exactly \
         how the frame and the arc the list paints from inside it would drift apart"
    );

    // And the two names the old, measured-off-the-picture inset lived in are gone — a second
    // inset could only come back as a second constant.
    for gone in ["TEXT_INSET_X", "COMBO_TEXT_INSET_X"] {
        assert!(
            !source.contains(gone),
            "`{gone}` must be gone: the mock-ups state the inset of a field in dialog units \
             and the inset of a list row in their own pixels, and neither is 12 px"
        );
    }
}

/// **Criterion 12 of T-11-16, carried from T-11-15** — no drawing carries a bare pixel number.
///
/// The specific offenders this task removed, named one by one so that none of them can come
/// back by accident: the pen and the three points of the check mark, the side of a glyph, the
/// gap after it, the inset of a radio dot, and the side of a layout-list tick.
#[test]
fn no_figure_of_the_dialog_is_drawn_by_a_bare_number() {
    // T-14-3: the drawing is two files now — see `drawing_source`.
    let source = drawing_source();

    for gone in [
        // the 2-pixel pen and the 13×13 coordinates of T-11-5b
        "CreatePen(PS_SOLID, 2, ink)",
        "glyph.left + 3",
        "glyph.left + 5",
        "glyph.left + 10",
        // the bare screen-pixel glyph of T-11-5b and the gap beside it
        "const GLYPH_SIZE: i32 = 13",
        "GLYPH_TEXT_GAP",
        "GLYPH_DOT_INSET:",
        // the bare screen-pixel tick of T-11-7
        "CHECK_FRAME_SIZE",
    ] {
        assert!(
            !source.contains(gone),
            "`{gone}` must be gone — a length of the mock-ups written as a bare screen pixel"
        );
    }

    // Every pen of the dialog is still a length of the mock-ups in tenths, through the scale.
    //
    // ⚠ Named as the length and not as the whole `CreatePen(…)` line since task T-11-17: both
    // strokes are made by the one `stroke_polyline`, which is handed the thickness — the
    // smoothed drawing hands it the same number multiplied by `SUPERSAMPLE`, and a `CreatePen`
    // spelled out at each figure could not do that. The substance of the row is unchanged: the
    // thickness of both pens is a `_TENTHS` constant of the mock-ups through `scaled_tenths`,
    // and the assertion below holds that the one `CreatePen` left takes a name and not a
    // number.
    for pen in [
        "scaled_tenths(mark.pen_tenths, dpi)",
        "scaled_tenths(COMBO_CHEVRON_PEN_TENTHS, dpi)",
    ] {
        assert!(source.contains(pen), "the pen `{pen}` must be the one made");
    }

    assert!(
        source.contains("CreatePen(PS_SOLID, thickness, ink)"),
        "the one pen of the two strokes must take the thickness it is handed"
    );

    // The three points of a check mark are a table of the module and not coordinates written
    // where they are drawn.
    let body = function_body(&source, "fn draw_check_mark(");

    assert!(
        body.contains("check_mark_points(glyph, mark, dpi, mirrored)"),
        "the strokes must come from the pure `check_mark_points`, not from literals"
    );

    // The two marks the dialog draws, each with the literals its own arm of the generator has.
    // The dialog glyph is the row of the reference; the tick of the layout list is the same
    // figure one size down, and the reference gives its pen but not its points — so the points
    // are held here, against the `(PtF ($bx+3.4) ($by+6.6)) …` of the `'lview'` arm.
    assert_eq!(
        settings::GLYPH_CHECK_MARK,
        theme::CheckMark {
            points_tenths: [(45, 86), (73, 118), (125, 52)],
            pen_tenths: 21,
        }
    );
    assert_eq!(
        settings::LIST_CHECK_MARK,
        theme::CheckMark {
            points_tenths: [(34, 66), (56, 90), (96, 40)],
            pen_tenths: 18,
        }
    );

    // Both land inside the square they belong to, at 96 DPI and at 200 %: a check mark that
    // walked out of its own glyph would be a defect no table of constants would catch.
    for (mark, side) in [
        (settings::GLYPH_CHECK_MARK, settings::GLYPH_SIZE),
        (settings::LIST_CHECK_MARK, settings::LIST_CHECK_SIZE),
    ] {
        for dpi in [96, 120, 144, 192] {
            let square = theme::scaled(side, dpi);

            let box_of_the_glyph = RECT {
                left: 0,
                top: 0,
                right: square,
                bottom: square,
            };

            for (x, y) in theme::check_mark_points(&box_of_the_glyph, mark, dpi, false) {
                assert!(
                    (0..=square).contains(&x) && (0..=square).contains(&y),
                    "at {dpi} DPI the point ({x}, {y}) is outside the {square}-pixel square"
                );
            }
        }
    }
}

/// **Criterion 11 of T-11-25, the first half** — the cell of the state image list is the key
/// colour and nothing else, so the whole of it is a hole and no pixel of it is ever seen.
///
/// # What this test used to say, and why it says something else now
///
/// Until T-11-25 the cell held the picture of the tick, drawn *over* the key colour, and this
/// test held the key apart from every colour of both palettes — because a palette that grew a
/// field equal to the key would have punched a hole in its own tick. That premise is gone: the
/// cell carries no palette colour at all any more. The tick moved onto the row itself
/// (`draw_cycle_row`), where the mask cannot reach it and the edge of the square can therefore
/// be smoothed — the whole subject of T-11-25, measured in
/// `the_square_of_the_tick_is_smoothed_and_carries_no_key_colour` below.
///
/// So the sentence this test holds is the new one, and it is held on **real pixels** rather
/// than on the source: both cells of the list the module actually builds are drawn over a
/// ground of their own and must leave every pixel of it exactly as it was.
#[test]
fn the_cell_of_the_state_image_list_is_a_hole_edge_to_edge() {
    use windows::Win32::UI::Controls::{
        ILD_NORMAL, ImageList_Destroy, ImageList_Draw, ImageList_GetImageCount,
    };

    let source = settings_module_source();

    // The cell is filled with the key and nothing is drawn into it afterwards: the whole
    // picture of the tick lives in `draw_check_glyph`, which the row painter calls.
    let body = function_body(&source, "fn fill_check_cell(");

    assert!(
        body.contains("FillRect(dc, &whole, key)"),
        "the cell must be the key colour over the whole of itself"
    );
    assert!(
        !body.contains("check_frame_colors") && !body.contains("draw_check_mark"),
        "nothing of the tick may be drawn into the cell any more — the mask would make a hole \
         of one exact colour and the square could not be smoothed"
    );
    assert!(
        source.contains("ImageList_AddMasked(list, bitmap, CHECK_CELL_KEY)"),
        "the hole must still come from the documented masked add"
    );

    // The row height of the layout list at 96 DPI, near enough: what the cell is measured
    // for is the row, and the test only needs a cell big enough to see.
    let list = settings::build_check_image_list(19).expect("the state image list must build");

    // SAFETY: `list` is the live list just built and owned by this frame.
    assert_eq!(
        unsafe { ImageList_GetImageCount(list) },
        2,
        "one cell per state image index, as `CHECK_FRAME_ORDER` counts them"
    );

    let ground = COLORREF(0x0040_3020);
    let sheet = Sheet::new(96);
    sheet.clear(ground);

    for index in 0..2 {
        // SAFETY: `list` is live and ours, `sheet.dc` holds the sheet's own bitmap, and the
        // cell is far smaller than the sheet.
        let drawn = unsafe { ImageList_Draw(list, index, sheet.dc, 4, 4, ILD_NORMAL) };

        assert!(
            drawn.as_bool(),
            "cell {index} must draw — a refused blit would prove nothing"
        );
    }

    let mut touched = Vec::new();

    for y in 0..96 {
        for x in 0..96 {
            if sheet.rgb(x, y) != (0x20, 0x30, 0x40) {
                touched.push((x, y, sheet.rgb(x, y)));
            }
        }
    }

    println!(
        "оба кадра списка изображений: {} изменённых пикселей из {}",
        touched.len(),
        96 * 96
    );

    // SAFETY: the list was built by us, handed to no control, and is freed exactly once.
    let _ = unsafe { ImageList_Destroy(Some(list)) };

    assert!(
        touched.is_empty(),
        "a cell that is the key colour edge to edge must leave the ground untouched — {:?}",
        &touched[..touched.len().min(8)]
    );
}

/// **Хвост 1 задачи T-12-7** — ячейка списка изображений уже левого края строки ровно на тот
/// отступ, который `SysListView32` кладёт после неё сам, и галочка от этого не двигается.
///
/// Числа, из которых собрана правка, — замер живого контрола стендом `dlgstand dark cycle`
/// (док-комментарий `LVIEW_LABEL_INDENT`): `LVM_GETITEMRECT` даёт `LVIR_BOUNDS` = `0,2,229,26`
/// и `LVIR_LABEL` = `21,2,229,26` при ячейке 19 px — то есть 2 px до прямоугольника ярлыка, и
/// ещё 2 px до пера внутри него. Здесь держится следствие: ширина **картинки** равна ширине
/// строки минус эти четыре, а `glyph_left` и `glyph_side` не трогаются вовсе — иначе галочка
/// уехала бы вместе с текстом, и хвост 1 сломал бы T-11-25-2.
#[test]
fn the_state_image_cell_gives_back_the_indent_the_list_view_adds_after_it() {
    // Клик по квадрату переключает участие (`LVS_EX_CHECKBOXES`), а областью клика служит
    // ячейка state-образа: сузив её, нельзя выронить из неё саму галочку.
    for dpi in [96, 120, 144, 192] {
        let cell = settings::check_cell(dpi, 22);

        assert_eq!(
            cell.image_width,
            (cell.width - settings::LVIEW_LABEL_INDENT).max(1),
            "at {dpi} DPI the cell must be the mock-up's left edge less the control's own indent"
        );

        assert!(
            cell.glyph_left + cell.glyph_side <= cell.image_width,
            "at {dpi} DPI the tick ({}..{}) must still end inside the {}-pixel cell, or the \
             click that toggles the row would land outside it",
            cell.glyph_left,
            cell.glyph_left + cell.glyph_side,
            cell.image_width
        );
    }

    // 96 DPI, где замер и сделан: 5 + 9 + 5 = 19 макетных пикселей строки, 15 — ячейки.
    let cell = settings::check_cell(96, 22);

    assert_eq!(
        (
            cell.width,
            cell.image_width,
            cell.glyph_left,
            cell.glyph_side
        ),
        (19, 15, 5, 9),
        "the measured cell of the stand: a 19-pixel row edge, a 15-pixel state image, the tick \
         5 px in and 9 px across"
    );

    // И лист, который модуль действительно строит, — той же ширины: это то, что видит контрол.
    let cell_height = 22;
    let list =
        settings::build_check_image_list(cell_height).expect("the state image list must build");
    let (mut cx, mut cy) = (0i32, 0i32);

    // SAFETY: `list` is the live list just built and owned by this frame; both pointers are to
    // live locals the call fills.
    let asked = unsafe {
        windows::Win32::UI::Controls::ImageList_GetIconSize(
            list,
            Some(std::ptr::from_mut(&mut cx)),
            Some(std::ptr::from_mut(&mut cy)),
        )
    };

    // SAFETY: the list was built by us, handed to no control, and is freed exactly once.
    let _ = unsafe { windows::Win32::UI::Controls::ImageList_Destroy(Some(list)) };

    assert!(asked.as_bool(), "the list must answer its own cell size");
    assert_eq!(
        (cx, cy),
        (cell.image_width, cell_height),
        "the built list must carry the narrowed cell, because that is the number the control \
         lays the row out around"
    );
}

/// **Хвост 2 задачи T-12-7** — ячейке отдаётся высота строки макета **минус** тот пиксель,
/// который контрол добавляет сам, и арифметика эта живёт ровно в одном месте.
///
/// Замер стенда (док-комментарий `LVIEW_ROW_OVERHEAD`): ячейка 23 px → `LVIR_BOUNDS`
/// `0,2,229,26` и `0,26,229,50`, то есть строка 24 px. Отдать ячейке 22 — получить 23, что и
/// есть `dialog_units(LAYOUT_ROW_HEIGHT_DLU)`. Здесь держится **источник** числа: высота
/// по-прежнему идёт из диалоговых единиц, а не из пикселя, подобранного с картинки.
#[test]
fn the_row_of_the_layout_list_is_asked_in_dialog_units_less_the_pixel_the_control_adds() {
    let source = settings_module_source();
    let body = function_body(&source, "fn install_check_images(");

    assert!(
        body.contains("dialog_units(hwnd, 0, LAYOUT_ROW_HEIGHT_DLU)"),
        "the height of a row must still come from the dialog's own units"
    );

    assert!(
        body.contains("vertical - LVIEW_ROW_OVERHEAD"),
        "the cell must be handed the row of the mock-ups less `LVIEW_ROW_OVERHEAD` — the pixel \
         the control puts back"
    );

    assert!(
        body.contains(".max(0)"),
        "a refused `MapDialogRect` answers zero, and zero less one is a negative cell height"
    );

    assert_eq!(
        settings::LVIEW_ROW_OVERHEAD,
        1,
        "the measurement of the stand: cell 23 → row 24"
    );
}

/// **Хвост 3 задачи T-12-7 (бэклог T-11-26)** — единственная колонка списка раскладок берёт
/// **всю** клиентскую ширину контрола, и запас под полосу прокрутки из неё не вычитается
/// второй раз.
///
/// Замер стенда (док-комментарий `cycle_column_width`): `GetClientRect` контрола отвечает 249
/// без прокрутки и **232** с нею при неизменных 249 наружных — полосу он уже вычел сам.
/// Прежние `client_width − 20` были вторым вычитанием, и справа оставалась мёртвая полоса
/// 21 px (находка **BLIND-3**).
#[test]
fn the_single_column_of_the_layout_list_takes_the_whole_client_width() {
    let source = settings_module_source();
    let width = function_body(&source, "fn cycle_column_width(");

    assert!(
        width.contains("client_width(hwnd, IDC_CYCLE_LIST)"),
        "the width must be the one the control answers"
    );

    assert!(
        !width.contains("- 20"),
        "nothing is taken off the client width any more: `GetClientRect` has already taken the \
         scroll bar off it"
    );

    // Ширина не константа окна: полоса прокрутки появляется вместе с переполняющей строкой,
    // то есть **после** заполнения, — поэтому колонку пересчитывают там, где строки меняются.
    let fill = function_body(&source, "fn fill_cycle_list(");

    assert!(
        fill.contains("fit_cycle_column(hwnd)"),
        "every change of the number of rows must refit the column — the scroll bar comes and \
         goes with that number"
    );

    let fit = function_body(&source, "fn fit_cycle_column(");

    assert!(
        fit.contains("LVM_SETCOLUMNWIDTH") && fit.contains("cycle_column_width(hwnd)"),
        "the refit must be the documented message, and it must ask the same arithmetic the \
         column was created with"
    );

    // И создаётся колонка тем же выражением, а не своей копией числа.
    let prepare = function_body(&source, "fn prepare_cycle_list(");

    assert!(
        prepare.contains("cx: cycle_column_width(hwnd)"),
        "the column must be created from the one function that answers its width"
    );
}

/// **Criterion 10 of T-11-13** — the panels and their captions are drawn by the background,
/// and the branch that used to draw them from `WM_DRAWITEM` is gone.
///
/// The first half is the table of `background_figure`: every group is a panel, every field
/// and list is a frame, and nothing else is either. The second half is read out of the
/// source of the module, in the manner of the sweeps in `tests\guard.rs` and
/// `tests\diag.rs` — the drawing of a panel must not be reachable from a message again.
#[test]
fn the_background_draws_the_panels_and_wm_drawitem_no_longer_knows_them() {
    use lang_switcher::settings::BackgroundFigure;

    for panel in settings::GROUP_BOXES {
        assert_eq!(
            settings::background_figure(panel),
            Some(BackgroundFigure::Panel),
            "control {panel} is a group panel"
        );
    }

    for field in settings::FRAMED_FIELDS {
        assert_eq!(
            settings::background_figure(field),
            Some(BackgroundFigure::Field),
            "control {field} is a field or a list"
        );
    }

    // A push button, a check box, a combo box, a static — the background leaves all of them
    // to their own drawing.
    for other in [1, 2, 1080, 1001, 1002, 1012, 1091, 1070] {
        assert_eq!(
            settings::background_figure(other),
            None,
            "control {other} must be left to its own drawing"
        );
    }

    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("settings.rs"),
    )
    .expect("the module source must be readable");

    assert!(
        !source.contains("draw_group_panel"),
        "`draw_group_panel` is back — a panel drawn from a message is an element again"
    );

    // The one handler that draws a panel is the background one, and it is wired to
    // `WM_ERASEBKGND` and to nothing else.
    assert!(
        source.contains("WM_ERASEBKGND => {"),
        "the dialog must answer WM_ERASEBKGND"
    );
    assert!(
        source.contains("unsafe fn on_erase_background("),
        "the background handler must exist"
    );
}

/// **Criterion 14 of T-11-13** — FR-94: the caption of a panel is the text of the hidden
/// control, so changing the interface language changes the panels too.
///
/// The path is one road with no fork: `SetDlgItemTextW` writes the string of the locale in
/// force onto the control (the `LOCALISED_CONTROLS` table, run by `localise_dialog`), and
/// `GetDlgItemTextW` reads it back off the same control when the background draws
/// (`get_text` inside `draw_panel_caption`). This test walks that road: the eight panels are
/// all in the localisation table, each with a string row of its own, and the drawing takes
/// its text from `get_text` and from no store of its own.
#[test]
fn the_panel_captions_are_read_off_the_controls_fr_94_writes() {
    let product = ProductImage::shared();

    for panel in settings::GROUP_BOXES {
        let row = settings::LOCALISED_CONTROLS
            .iter()
            .find(|(control, _)| *control == panel)
            .map(|(_, string)| *string)
            .unwrap_or_else(|| {
                panic!(
                    "panel {panel} is in no row of the localisation table — FR-94 would \
                        never rewrite its caption"
                )
            });

        let russian = product.string(settings::Language::Ru, row);
        let english = product.string(settings::Language::En, row);

        println!(
            "panel {panel}: ru «{russian}» → «{}», en «{english}» → «{}»",
            settings::panel_caption(&russian),
            settings::panel_caption(&english)
        );

        assert!(!russian.is_empty(), "panel {panel} has no Russian caption");
        assert!(!english.is_empty(), "panel {panel} has no English caption");
        assert_ne!(
            russian, english,
            "panel {panel} would look the same in both locales — the test would prove \
             nothing about the language"
        );
    }

    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("settings.rs"),
    )
    .expect("the module source must be readable");

    // The one place the caption string is obtained, and it is the control's own text.
    assert!(
        source.contains("let caption = panel_caption(&get_text(hwnd, control));"),
        "the caption must come from the control itself (get_text → GetDlgItemTextW)"
    );
    assert!(
        source.contains("fn get_text(hwnd: HWND, control: i32) -> String {"),
        "`get_text` must still be the GetDlgItemTextW wrapper the caption path goes through"
    );
}

/// **T-12-11** — the ink of a panel heading is `cap`, and only the heading is.
///
/// Decision В-5 (`DECISIONS.md`, question 56) gave the heading a palette role of its own. In
/// «Графите» the new role spells the number `text_muted` already spelled, so nothing on a
/// dark screen can tell the two apart — the *only* place the split is visible is the light
/// palette, and the only thing that keeps it from being quietly undone is this pair of
/// checks. They are two, deliberately:
///
/// * the heading takes `palette.cap` — the field it is now painted from;
/// * `text_muted` is still the ink of the explanatory notes — no `caption:` row of the
///   background's colour set may name it again.
///
/// Together they separate «the headings were repainted» from «everything quiet was
/// repainted», which is the same separating probe the live measurement of this task takes on
/// the screen: the note «Цикл: галочка — участие, кнопки — порядок», the note «Имя процесса,
/// например game.exe» and the journal path must stay 110,118,127 in «Тумане» while the eight
/// headings move to 122,130,139.
#[test]
fn the_panel_heading_takes_the_cap_role_and_leaves_the_muted_notes_alone() {
    // The two roles, side by side: equal in the dark, twelve levels apart in the light. A
    // palette that lost the split would make every other check of this test vacuous.
    assert_eq!(
        GRAPHITE.cap, GRAPHITE.text_muted,
        "«Графит» spells one number for both roles (ui.ps1:179) — that is why the dark shot \
         of this task must come out unchanged"
    );
    assert_ne!(
        FOG.cap, FOG.text_muted,
        "«Туман» is the palette that tells the two roles apart (ui.ps1:190) — with them \
         equal there is no heading colour to check for"
    );

    let source = settings_module_source();

    assert!(
        source.contains("caption: palette.cap,"),
        "the ink of a panel heading must be taken from `palette.cap`"
    );
    assert!(
        !source.contains("caption: palette.text_muted"),
        "no heading may be painted with the muted ink again — that is the mistake T-12-11 \
         measured and В-5 answered"
    );

    // And the muted role is still in the file for the notes it was always for: a change that
    // removed every mention of it would pass the two checks above and still be wrong.
    assert!(
        source.contains("palette.text_muted"),
        "`text_muted` must stay the ink of the explanatory notes — T-12-11 added a role, it \
         did not replace one"
    );
}

// =========================================================================================
// FR-92а — the closed part of the combo boxes, the rounded check-box glyph and the exclusion
// list that paints its own selection. Task T-11-14.
// =========================================================================================
//
// What is measurable without a window: the pure halves — the colour table of the closed part,
// the geometry of the chevron, the radius of the glyph, the colour table of a list row, the
// gate that says which control may draw its own items — and the shape of the module's own
// source, where the pairing of the subclass and the fate of `WM_PAINT` live. The look on the
// screen is the controller's, on the real window: the product is not started by any test.

/// The module source, with the line endings normalised — the sweeps below match on it.
fn settings_module_source() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("settings.rs"),
    )
    .expect("the module source must be readable")
    .replace("\r\n", "\n")
}

/// The source of `theme`, the same way — task **T-14-3**.
fn theme_module_source() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("theme.rs"),
    )
    .expect("the theme source must be readable")
    .replace("\r\n", "\n")
}

/// **The text of the drawing this program does — task T-14-3.**
///
/// Until that task the drawing engine of FR-92а lived inside `settings`, and the sweeps of
/// this file swept one file because there was one file to sweep. T-14-3 moved the engine to
/// its owner (§6.2, finding 24 of the audit of 2026-08-24), so «the drawing» is now the two
/// files joined, and the sweeps that are about the drawing read this instead.
///
/// ⚠ The join **strengthens** every absence assertion and merely re-addresses the presence
/// ones: `HALFTONE` may now appear in neither file rather than in one, and
/// `SetStretchBltMode(dc, COLORONCOLOR)` is still required to be written exactly once — in
/// whichever of the two it now lives. What it deliberately does *not* do is replace the sweeps
/// that are about the **shape of `settings` itself** — the subclass pairings, `get_text`, the
/// absence of `draw_group_panel` — and those go on reading [`settings_module_source`].
fn drawing_source() -> String {
    let mut joined = settings_module_source();
    joined.push('\n');
    joined.push_str(&theme_module_source());
    joined
}

/// The body of one function of the module source, from the opening of its signature to the
/// closing brace in the first column — enough to sweep one function without dragging the
/// neighbours in.
fn function_body<'a>(source: &'a str, signature: &str) -> &'a str {
    let after = source
        .split_once(signature)
        .unwrap_or_else(|| panic!("the module must still declare `{signature}`"))
        .1;

    after
        .split_once("\n}\n")
        .unwrap_or_else(|| panic!("`{signature}` must end at a brace in the first column"))
        .0
}

/// **Criterion 10 of T-11-14**, widened by **criterion 12 of T-12-8** — the closed part of a
/// combo box is drawn from a closed table of roles, and the table is a pure function.
///
/// The full 2×2 now: наведение × разрешённость. The frame is the field frame whatever happens —
/// a combo box is a field to the eye, exactly as the five input fields and the two lists of
/// `FRAMED_FIELDS` are since T-11-13 — and the chevron is the muted hint in every cell. What
/// the disabled state moves is the text, the precedent `button_color_roles` and
/// `glyph_color_roles` set: запрещённость гасит. What the cursor moves is the fill, and only
/// the fill: `hover_bg` while it stands on an enabled closed face, and nothing else in the row
/// changes with it.
///
/// The bottom half of the table is the precedence «запрет выше наведения», the very one the
/// buttons keep: a combo box the mode has switched off does not light up under the cursor.
#[test]
fn the_closed_part_of_a_combo_box_follows_its_own_colour_table() {
    // T-14-3: the combo roles moved to their owner, `theme`.
    use lang_switcher::theme::{ComboBorderRole, ComboChevronRole, ComboClosedColors};

    use ComboFillRole as Fill;
    use ComboTextRole as Ink;

    let table: [(bool, bool, Fill, ComboBorderRole, Ink, ComboChevronRole); 4] = [
        // Разрешённый, курсора нет: the value in the ordinary ink on the quiet field.
        (
            false,
            false,
            Fill::FieldBg,
            ComboBorderRole::FieldBorder,
            Ink::Text,
            ComboChevronRole::TextMuted,
        ),
        // Разрешённый, курсор на закрытой части — task T-12-8: the fill goes to `hover_bg`
        // and not one other cell of the row moves.
        (
            true,
            false,
            Fill::HoverBg,
            ComboBorderRole::FieldBorder,
            Ink::Text,
            ComboChevronRole::TextMuted,
        ),
        // Запрещённый — «Источник» and «Цель» under the «Несколько раскладок» mode: the same
        // field, the same chevron, the muted value.
        (
            false,
            true,
            Fill::FieldBg,
            ComboBorderRole::FieldBorder,
            Ink::TextMuted,
            ComboChevronRole::TextMuted,
        ),
        // Запрещённый под курсором: запрет выше наведения — the row is the one above,
        // unchanged, because a combo box that takes no click must not advertise itself.
        (
            true,
            true,
            Fill::FieldBg,
            ComboBorderRole::FieldBorder,
            Ink::TextMuted,
            ComboChevronRole::TextMuted,
        ),
    ];

    for (hot, disabled, fill, border, text, chevron) in table {
        assert_eq!(
            theme::combo_closed_color_roles(hot, disabled),
            ComboClosedColors {
                fill,
                border,
                text,
                chevron
            },
            "hot = {hot}, disabled = {disabled}"
        );
    }
}

/// **Criterion 10 of T-11-14, the geometry half; п. 8 of T-11-16** — the chevron stands where
/// the *generator* puts it, and grows with the DPI of the window.
///
/// The numbers are no longer read off `ui-03-fog.png` with the eye: `scratchpad\ui.ps1`, the
/// `'combo'` arm, draws the figure through `(PtF ($cx-4) ($cy-2)), (PtF $cx ($cy+2)), (PtF
/// ($cx+4) ($cy-2))` around a centre at `$cx = $px + $pw - 14`. So the span is 8 mock-up
/// pixels and the drop **4**, both measured from the centre — not a span halved twice, which
/// is what put the apex a whole pixel low at 96 DPI.
#[test]
fn the_chevron_stands_where_the_mock_up_measured_it() {
    // A closed part 140 × 33 at 100 %, the proportions of the measured picture.
    let area = rect(0, 0, 140, 33);
    let points = theme::combo_chevron_points(&area, 96);

    println!("chevron at 96 dpi: {points:?}");

    let (left, top) = points[0];
    let (apex_x, apex_y) = points[1];
    let (right, right_top) = points[2];

    // Two arms of equal height and one apex below them — a chevron pointing down, and not up.
    assert_eq!(top, right_top, "the two arms must stand at one height");
    assert!(
        apex_y > top,
        "the apex must lie below the arms — «галочка вниз»"
    );

    // The two half-lengths of the generator: ±4 by `x` and ∓2 by `y` of the picture, which
    // since T-11-15 are 3 px and 1 px of a 100 % screen — the picture is drawn at 140 %.
    assert_eq!(apex_x - left, 3, "4 px of the mock-up are 3 px at 100 %");
    assert_eq!(right - apex_x, 3, "and the same on the other side");
    assert_eq!(
        apex_y - top,
        2,
        "±2 by y of the picture, twice, is 2 px at 100 %"
    );
    assert_eq!(apex_x - left, right - apex_x, "the apex is in the middle");

    // Vertically centred in the field it is given.
    assert_eq!(
        (top + apex_y) / 2,
        (area.top + area.bottom) / 2,
        "the chevron must be centred on the middle of the closed part"
    );

    // On the right, well clear of the text, and inside the field.
    assert!(
        apex_x > (area.left + area.right) / 2,
        "the chevron belongs to the right half of the field"
    );
    assert!(
        right < area.right && left > area.left,
        "the chevron must stay inside the field"
    );

    // And it scales, like every other length of the mock-ups since T-11-13.
    let wide = theme::combo_chevron_points(&rect(0, 0, 280, 66), 192);

    println!("chevron at 192 dpi: {wide:?}");
    assert_eq!(wide[2].0 - wide[0].0, 12, "the span doubles at 200 %");
    assert_eq!(wide[1].1 - wide[0].1, 6, "the drop doubles with it");

    // The stroke is the one length of the mock-ups written in tenths of a pixel: 1,5 px of the
    // picture, which is 1,07 px of a 100 % screen — one pixel, and two at 200 % (T-11-15).
    assert_eq!(
        theme::scaled_tenths(15, 96),
        1,
        "1,5 px of a 140 % picture is one pixel at 100 %"
    );
    assert_eq!(theme::scaled_tenths(15, 192), 2, "2 px at 200 %");
    // Never a zero-width pen, which GDI would read as a hairline drawn by other rules.
    assert_eq!(theme::scaled_tenths(1, 96), 1);
    assert_eq!(
        theme::scaled_tenths(15, 0),
        1,
        "the 100 % look when the DPI is refused"
    );
}

/// **Criteria 9 and 11 of T-11-14** — the subclass is installed and removed as one pair, and
/// `WM_PAINT` never reaches the procedure that draws the system button.
///
/// Both facts live in the shape of the module's own source, which is where the sweeps of
/// `tests\guard.rs` and `tests\diag.rs` read their kind of fact from. Criterion 9: exactly one
/// install site and exactly one removal site of the pair, walking the same list with the same
/// procedure and the same identifier, plus the `WM_NCDESTROY` safety net inside the procedure.
/// Criterion 11: the `WM_PAINT` arm of the procedure ends in a `return`, so the fall-through
/// to `DefSubclassProc` — and with it the control's own painting — is unreachable from it.
#[test]
fn the_combo_subclass_is_one_pair_and_wm_paint_never_falls_through() {
    let source = settings_module_source();

    // The pair, by its call sites. The install is matched with its own indentation, because
    // «unsubclass_combo_boxes» contains «subclass_combo_boxes» as a substring.
    assert_eq!(
        source.matches("\n    subclass_combo_boxes(hwnd);").count(),
        1,
        "the subclass must be installed in exactly one place"
    );
    assert_eq!(
        source.matches("unsubclass_combo_boxes(hwnd);").count(),
        1,
        "the subclass must be removed in exactly one place"
    );
    assert!(
        source.contains("WM_DESTROY => {"),
        "the removal hangs on the dialog's WM_DESTROY, where the children are still alive"
    );

    // The same procedure and the same identifier on both sides — one install, two removals
    // (the pair's own, and the WM_NCDESTROY safety net inside the procedure).
    assert_eq!(
        source
            .matches("SetWindowSubclass(combo, Some(combo_box_proc), COMBO_SUBCLASS_ID, 0)")
            .count(),
        1,
        "exactly one SetWindowSubclass, with the procedure and the identifier of the pair"
    );

    // ⚠ **Since task Т-32-8 there are two pairs and not one**, and the canon of «two removals»
    // is raised to three with the reason written down rather than worked around: the wizard of
    // FR-104 has a combo box of its own, on another window, and it wears this dialog's closed
    // face rather than a copy of it (§6.2). Its install carries `COMBO_FOREIGN` instead of `0`,
    // which is what the assertion above still counts as one; its removal is the third match
    // here. The `WM_NCDESTROY` safety net inside the procedure serves both pairs.
    // ⚠ Matched on the **argument** and not on a one-line call: `cargo fmt` broke that install
    // across five lines the moment it was written, and a needle shaped like a single line went
    // on answering «none». The install is the only place in the file that passes this name.
    assert_eq!(
        source.matches("COMBO_FOREIGN,").count(),
        1,
        "exactly one foreign install — the wizard's combo box and no other"
    );
    assert_eq!(
        source
            .matches("RemoveWindowSubclass(combo, Some(combo_box_proc), COMBO_SUBCLASS_ID)")
            .count(),
        3,
        "the removals of the two pairs and the WM_NCDESTROY safety net, and nothing else"
    );

    let body = function_body(&source, "unsafe extern \"system\" fn combo_box_proc(");

    // The whole of criterion 11: the WM_PAINT arm answers and returns.
    let paint_arm = body
        .split_once("WM_PAINT => {")
        .expect("the subclass must answer WM_PAINT")
        .1
        .split_once("\n        }")
        .expect("the WM_PAINT arm must be an arm of the match")
        .0;

    println!("--- the WM_PAINT arm of combo_box_proc ---\n{paint_arm}");

    assert!(
        paint_arm.contains("paint_combo_closed_part(combo, reference_data)"),
        "the WM_PAINT arm must draw the closed part itself — and since task Т-32-8 it hands on \
         the reference datum, which is what tells a combo of another window from the four of \
         this one"
    );
    assert!(
        paint_arm.contains("return LRESULT(0);"),
        "the WM_PAINT arm must return — a fall-through would let the control draw the \
         system arrow button, which is the whole defect of this task"
    );
    assert!(
        !paint_arm.contains("DefSubclassProc"),
        "the WM_PAINT arm must not reach the displaced procedure"
    );

    // One tail call to the displaced procedure in the whole body, and it is the tail.
    assert_eq!(
        body.matches("DefSubclassProc(").count(),
        1,
        "everything that is not answered here goes on, once, at the end"
    );

    // The dialog itself does not answer WM_PAINT: the closed part is the subclass's business
    // and the background is WM_ERASEBKGND's (T-11-13).
    let dialog = function_body(&source, "unsafe extern \"system\" fn dialog_proc(");

    assert!(
        !dialog.contains("WM_PAINT"),
        "the dialog procedure must not have grown a WM_PAINT branch"
    );
}

/// **Criterion 12 of T-11-14** — the check-box square is rounded by a named constant of 3 px,
/// and it is one figure now rather than a fill under a frame.
#[test]
fn the_check_box_glyph_is_rounded_by_the_three_pixels_of_the_mock_ups() {
    assert_eq!(
        settings::GLYPH_CORNER_RADIUS,
        3,
        "the check-box square — 3 px, smaller than the 4 of a field because the figure is"
    );

    // A length of the mock-ups like every other since T-11-13 — and since T-11-15 a length of
    // pictures drawn at 140 %, so the three pixels of the picture are two on a 100 % screen.
    assert_eq!(theme::scaled(settings::GLYPH_CORNER_RADIUS, 96), 2);
    assert_eq!(theme::scaled(settings::GLYPH_CORNER_RADIUS, 192), 4);

    let source = settings_module_source();
    let body = function_body(&source, "unsafe fn draw_glyph_element(");

    let square = body
        .split_once("GlyphKind::CheckBox => {")
        .expect("the glyph drawing must still have its check-box arm")
        .1
        .split_once("GlyphKind::RadioButton => {")
        .expect("the radio arm must still follow it")
        .0;

    println!("--- the check-box arm of draw_glyph_element ---\n{square}");

    assert!(
        square.contains("paint_rounded("),
        "the square must be drawn by the rounded figure"
    );
    assert!(
        square.contains("GLYPH_CORNER_RADIUS"),
        "and by the named radius, not by a number written where it is used"
    );
    assert!(
        !square.contains("FillRect("),
        "a square FillRect cannot have a corner radius"
    );
    assert!(
        !square.contains("FrameRect("),
        "nor can a square FrameRect over it"
    );
}

/// **Критерий 2 T-12-6** — каждое тело, рисующее элемент целиком, **стирает фон своего
/// прямоугольника до того, как что-нибудь на нём нарисует**.
///
/// Owner-draw контрол отвечает за весь свой прямоугольник. `WM_CTLCOLORBTN` называет кисть,
/// но никто не обещает, что система положит её сама перед тем, как отдать рисование, — и
/// замер говорит, что не кладёт: флажок «Запускать при входе в систему», перерисованный
/// **поверх себя** при уходе фокуса (нажатие «Задать» — тот случай, на котором дефект нашли),
/// клал подпись вторым проходом по `SetBkMode(TRANSPARENT)`, и полутона сглаживания
/// складывались — краевой пиксель ореола 179,182,186 → 225,228,231 при **неизменных** ядрах
/// 228,231,234. Перерисовка обязана быть идемпотентной, и заливка — то, что её такой делает.
///
/// Читается из исходника, в манере остальных проверок формы этого файла: утверждается не
/// значение, а **порядок** — заливка стоит раньше первой фигуры и раньше текста. Строка,
/// которой не хватало, была именно отсутствием, и поймать её можно только так.
#[test]
fn every_owner_drawn_element_erases_its_ground_before_it_draws() {
    // T-14-7: the joined text of the drawing, because `paint_label` — the body of the label
    // half — moved to `theme` with the third slice of finding 24. The five other bodies of the
    // loop and both bodies of the second half are still `settings`'s own and are still found.
    let source = drawing_source();

    // Пять тел `WM_DRAWITEM` и шестое — закрытая часть комбобокса, которая рисует себя по
    // `WM_PAINT` подкласса и потому тем более отвечает за весь свой прямоугольник.
    //
    // ⚠ Т-26-2: тело подписи теперь называется `paint_label_at_pitch` — `paint_label` стала
    // однострочной обёрткой над ним с `None` вместо шага строки (§6.2: одно тело, не два), и
    // заливка уехала туда же, где рисование. Седьмое тело — строка справки с чипом: она
    // рисуется не `DrawTextW`, а по словам, и отвечает за свой прямоугольник ровно так же.
    // ⚠ `draw_chip` в списке НЕТ и быть не должно: он рисует фигуру ВНУТРИ строки, землю
    // которой уже положили, — заливка там стёрла бы соседние слова.
    for signature in [
        "unsafe fn paint_push_button(",
        "pub unsafe fn paint_label_at_pitch(",
        "pub unsafe fn paint_chip_row(",
        "unsafe fn draw_glyph_element(",
        "unsafe fn draw_combo_item(",
        "unsafe fn draw_list_item(",
        "unsafe fn draw_combo_closed_part(",
    ] {
        let body = function_body(&source, signature);

        let fill = body.find("FillRect(").unwrap_or_else(|| {
            panic!("`{signature}` must erase the ground of its rectangle with `FillRect`")
        });

        for later in ["paint_rounded(", "paint_ellipse(", "DrawTextW("] {
            let Some(at) = body.find(later) else {
                continue;
            };

            assert!(
                fill < at,
                "`{signature}`: the ground must be erased before `{later}` draws on it — \
                 otherwise a repaint lays the new drawing on top of the old one"
            );
        }
    }

    // И заливается тем же, чем этот диалог отвечает на `WM_CTLCOLORBTN`: панельной кистью для
    // контрола на одной из восьми панелей, оконной вне их. Два места спрашивают карту панелей,
    // а не назначают кисть от себя — иначе фон элемента и фон под ним могли бы разойтись.
    for signature in ["unsafe fn on_draw_item(", "unsafe fn draw_glyph_element("] {
        let body = function_body(&source, signature);

        assert!(
            body.contains("state.panel_children.contains(&control)"),
            "`{signature}` must read the ground off the panel map, exactly as `on_ctl_color` \
             does when it answers WM_CTLCOLORBTN"
        );
    }
}

/// **Criterion 13 of T-11-14** — the exclusion list paints its own rows, so the selected one
/// wears `sel_bg`/`sel_fg` instead of the system's `COLOR_HIGHLIGHT` blue; and everything the
/// population and the reading of FR-84 stand on survived the new style.
#[test]
fn the_exclusion_list_paints_its_own_selection() {
    use windows::Win32::UI::Controls::{ODT_BUTTON, ODT_COMBOBOX, ODT_LISTBOX};

    use ComboFillRole as Fill;
    use ComboTextRole as Ink;

    // The colours: the very table the combo items answer, reused and not copied.
    assert_eq!(
        theme::list_item_color_roles(false),
        ComboItemColors {
            fill: Fill::FieldBg,
            text: Ink::Text
        },
        "an ordinary row is the quiet ground of the list"
    );
    assert_eq!(
        theme::list_item_color_roles(true),
        ComboItemColors {
            fill: Fill::SelBg,
            text: Ink::SelFg
        },
        "the selected row is the selection pair of the palette — the point of the task"
    );

    // The gate SEC-05 asks for: one list box and four combo boxes, and nothing else.
    assert!(settings::owner_drawn_item(ODT_LISTBOX.0, 1050));
    assert!(
        !settings::owner_drawn_item(ODT_LISTBOX.0, 1024),
        "the layout list is a SysListView32 drawn by NM_CUSTOMDRAW — not this road"
    );
    assert!(
        !settings::owner_drawn_item(ODT_COMBOBOX.0, 1050),
        "the right identifier under the wrong type is still refused"
    );
    assert!(!settings::owner_drawn_item(ODT_BUTTON.0, 1050));

    for combo in [1002, 1003, 1022, 1023] {
        assert!(
            settings::owner_drawn_item(ODT_COMBOBOX.0, combo),
            "combo {combo} must keep its owner drawing of T-11-6"
        );
    }

    for stranger in [1, 1080, 1051, 1012] {
        assert!(
            !settings::owner_drawn_item(ODT_LISTBOX.0, stranger),
            "control {stranger} must not be able to ask for a drawing"
        );
    }

    // And the style, out of the built binary.
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let style = template
        .styles
        .iter()
        .find(|(control, _)| *control == 1050)
        .map(|(_, style)| *style)
        .expect("the dialog has no control 1050 — the exclusion list");

    println!("exclusion list (1050): style {style:#010x}");

    // LBS_OWNERDRAWFIXED (0x0010) appeared.
    assert_ne!(
        style & 0x0010,
        0,
        "the list must carry LBS_OWNERDRAWFIXED; the style is {style:#010x}"
    );
    // LBS_OWNERDRAWVARIABLE (0x0020) did not: a variable-height list asks WM_MEASUREITEM per
    // row, a protocol nobody here speaks.
    assert_eq!(
        style & 0x0020,
        0,
        "the list must not carry LBS_OWNERDRAWVARIABLE; the style is {style:#010x}"
    );
    // LBS_HASSTRINGS (0x0040) survived — LB_ADDSTRING and LB_GETTEXT of FR-84 stand on it,
    // and an owner-drawn list without it stores no strings at all.
    assert_ne!(
        style & 0x0040,
        0,
        "the list must keep LBS_HASSTRINGS; the style is {style:#010x}"
    );
    // LBS_NOTIFY (0x0001) — the selection change «Удалить» listens for.
    assert_ne!(
        style & 0x0001,
        0,
        "the list must keep LBS_NOTIFY; the style is {style:#010x}"
    );
    // LBS_NOINTEGRALHEIGHT (0x0100), WS_VSCROLL, WS_TABSTOP — untouched.
    assert_ne!(style & 0x0100, 0, "LBS_NOINTEGRALHEIGHT; {style:#010x}");
    assert_ne!(style & 0x0020_0000, 0, "WS_VSCROLL; {style:#010x}");
    assert_ne!(style & 0x0001_0000, 0, "WS_TABSTOP; {style:#010x}");
    // And the system border stayed off, as task T-11-13 left it.
    assert_eq!(
        style & WS_BORDER,
        0,
        "the list must keep its rounded frame of T-11-13, not the system one; {style:#010x}"
    );
}

/// **Criterion 14 of T-11-14** — the module names no system theme and reads no `itemData`.
///
/// The ban of FR-92а is on the undocumented ordinals of `uxtheme.dll` and on the names of the
/// dark system themes; a documented subclass and a `WM_PAINT` of one's own are not that, and
/// this test is what keeps the difference from eroding. The word `uxtheme` itself does appear
/// once in the module — in the ⚠ that says why the trick is not used — so what is swept for
/// is *use*: the crate module that holds the theme API, the calls, the ordinal wrappers and
/// the theme-name strings.
#[test]
fn the_settings_module_names_no_system_theme_and_reads_no_item_data() {
    let source = settings_module_source();

    for forbidden in [
        // The theme API itself.
        "Uxtheme",
        "SetWindowTheme(",
        "OpenThemeData(",
        "OpenThemeDataForDpi(",
        "DrawThemeBackground(",
        "DrawThemeText(",
        "GetThemeColor(",
        // The undocumented ordinals of the dark mode, by the names they go under.
        "AllowDarkModeForWindow",
        "AllowDarkModeForApp",
        "SetPreferredAppMode",
        "ShouldAppsUseDarkMode",
        "RefreshImmersiveColorPolicyState",
        "FlushMenuThemes",
        // And the names of the dark system themes.
        "DarkMode_Explorer",
        "DarkMode_CFD",
        "DarkMode_ItemsView",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` is in src\\settings.rs — FR-92а forbids exactly this"
        );
    }

    // SEC-05: `itemData` is named in the comments that promise it is never read, and read
    // nowhere. A field access is what a read looks like.
    assert!(
        !source.contains(".itemData"),
        "the module must not touch the itemData of any message struct — SEC-05"
    );
}

// Criterion 9 of T-11-6 — read out of the **built** `LangSwitcher.exe`, exactly as the
// buttons, glyphs and groups above. ⚠ The check differs from theirs in kind:
// `CBS_OWNERDRAWFIXED` (0x0010) is a style *flag* that combines with the rest, unlike the
// button type `BS_OWNERDRAW` — but the combo *type* lives in the low two bits, where
// `CBS_DROPDOWNLIST` is 0x0003, so the type check is equality of that field and the flag
// checks are bit tests. Everything the population and the keyboard relied on before the
// task must survive next to the new flag: `CBS_HASSTRINGS` is what keeps the item strings
// in the combo itself (the drawing reads them back with `CB_GETLBTEXT`), and a lost
// `WS_TABSTOP` would drop the combo out of the keyboard loop.
#[test]
fn the_four_combo_boxes_are_owner_drawn_and_keep_their_old_styles() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    // The four combo boxes of FR-92, by their actual identifiers.
    const OWNER_DRAWN_COMBOS: [(u32, &str); 4] = [
        (1002, "язык интерфейса"),
        (1003, "оформление"),
        (1022, "источник пары"),
        (1023, "цель пары"),
    ];

    for (id, what) in OWNER_DRAWN_COMBOS {
        let style = template
            .styles
            .iter()
            .find(|(control, _)| *control == id)
            .map(|(_, style)| *style)
            .unwrap_or_else(|| panic!("the dialog has no control {id} — «{what}»"));

        println!("«{what}» ({id}): style {style:#010x}");

        // The flag of task T-11-6 appeared.
        assert_ne!(
            style & 0x0010,
            0,
            "«{what}» ({id}) must carry CBS_OWNERDRAWFIXED — task T-11-6; \
             the style is {style:#010x}"
        );

        // And FIXED it is: CBS_OWNERDRAWVARIABLE (0x0020) is the neighbouring flag the
        // task does not ask for — a variable-height combo would ask WM_MEASUREITEM per
        // item, a protocol nobody here speaks.
        assert_eq!(
            style & 0x0020,
            0,
            "«{what}» ({id}) must not carry CBS_OWNERDRAWVARIABLE; the style is {style:#010x}"
        );

        // The combo *type* survived: CBS_DROPDOWNLIST (0x0003) — equality of the type
        // field, not a bit test, or a demoted CBS_DROPDOWN (0x0002) would slip through.
        assert_eq!(
            style & 0x0003,
            0x0003,
            "«{what}» ({id}) must keep CBS_DROPDOWNLIST as its combo type; \
             the style is {style:#010x}"
        );

        // CBS_HASSTRINGS (0x0200) survived — the existing population stands on it.
        assert_ne!(
            style & 0x0200,
            0,
            "«{what}» ({id}) must keep CBS_HASSTRINGS; the style is {style:#010x}"
        );

        // WS_VSCROLL (0x00200000) survived — the dropped-down list keeps its scroll bar.
        assert_ne!(
            style & 0x0020_0000,
            0,
            "«{what}» ({id}) must keep WS_VSCROLL; the style is {style:#010x}"
        );

        // WS_TABSTOP (0x00010000) survived — the combo stays in the keyboard loop.
        assert_ne!(
            style & 0x0001_0000,
            0,
            "«{what}» ({id}) must keep WS_TABSTOP; the style is {style:#010x}"
        );
    }
}

/// **Criteria 17 and 18 of T-12-2** — the air of the **closed** part of a combo box, its
/// derivation, and the wall between it and the items of the dropped-down list.
///
/// Findings F3 and R-04 of the Э12 protocol: the closed part stood 30 px against the mock-up's
/// 22,5, because `COMBO_ITEM_EXTRA = 12` had been derived by *measuring the picture* — its doc
/// comment claimed the mock-ups draw the closed part «33 of their own pixels high». The
/// generator has no such literal: `scratchpad\ui.ps1` states every combo box as `h = 12`
/// dialog units (lines 103, 116, 118), and that is the whole closed part.
///
/// This test holds the corrected number **and its derivation**, so that the next task cannot
/// re-derive the old one from a picture in silence — the arithmetic below is written out in
/// hundredths of a pixel and does not go anywhere near the module's own formula.
#[test]
fn the_closed_part_of_a_combo_box_is_the_twelve_dialog_units_of_the_generator() {
    // ---- the derivation, in hundredths of a pixel at 96 DPI ------------------------------
    //
    // 1. The generator's literal: `h = 12` vertical dialog units, and one vertical unit is an
    //    eighth of the dialog font's height — 15 px at 96 DPI, so 15/8 = 1,875 px.
    let model_box = 12 * 1500 / settings::DIALOG_FONT_HEIGHT_DLU; // 22,50 px
    assert_eq!(model_box, 2250, "12 DLU must be 22,5 px at 96 DPI");

    // 2. Six pixels of that box are not the item's: four of the system's own edges around the
    //    selection field and two of the frame this dialog draws at the edge of the client
    //    area. Measured on the stand, task T-12-2: window 30 px = CB_GETITEMHEIGHT(-1) 24 + 6.
    const SYSTEM_SPEND: i32 = 600;

    // 3. What is left is the selection field, and the dialog font fills 15 px of it.
    let field = model_box - SYSTEM_SPEND; // 16,50 px
    let air_at_96 = field - 1500; // 1,50 px

    assert_eq!(field, 1650);
    assert_eq!(air_at_96, 150);

    // 4. The mock-ups are drawn at 140 %, so the air as a length of *theirs* is 1,4 × that —
    //    2,1 mock-up pixels, which is 2 written whole.
    let air_in_mockup_pixels = air_at_96 * theme::MOCKUP_SCALE_TENTHS / 10; // 2,10 px
    assert_eq!(air_in_mockup_pixels, 210);
    assert_eq!(
        (air_in_mockup_pixels + 50) / 100,
        settings::COMBO_CLOSED_ITEM_EXTRA,
        "the air of the closed part must be the 12 DLU of the generator minus the six pixels \
         the system spends minus the font — 2,1 mock-up pixels — and not a number read off a \
         picture"
    );

    // ---- and what that number puts on the screen ----------------------------------------
    //
    // Through `scaled` — the one road a mock-up length takes into the module — 2 comes back
    // as 1 px at 96 DPI, so the field stands 16 px and the closed part 16 + 6 = 22 against the
    // mock-up's 22,5. The controller's acceptance window is 22..23 px.
    let field_at_96 = 15 + theme::scaled(settings::COMBO_CLOSED_ITEM_EXTRA, 96);

    assert_eq!(
        field_at_96, 16,
        "the selection field must stand 16 px at 96 DPI"
    );

    let closed_part = field_at_96 + 6;

    assert!(
        (22..=23).contains(&closed_part),
        "the closed part must measure 22..23 px at 96 DPI against the model's 22,5 — it is \
         {closed_part}"
    );

    // The defect this task was raised on: 30 px, which is what the list air still makes.
    assert_eq!(
        15 + theme::scaled(settings::COMBO_LIST_ITEM_EXTRA, 96) + 6,
        30,
        "the number that made the 30-pixel closed part must be identified, not forgotten"
    );

    // ---- the wall: the dropped-down list keeps the height it had -------------------------
    //
    // One `WM_MEASUREITEM` reaches a `CBS_OWNERDRAWFIXED` combo box and its answer is worn by
    // the closed part and by every item of the list alike — measured, not assumed: before this
    // task `CB_GETITEMHEIGHT(-1)` and `CB_GETITEMHEIGHT(0)` both answered 24 on all four combo
    // boxes, and after it 16 and 24. The height of the list items was never measured against
    // the mock-ups and T-12-2 was told not to move it, so this number must not drift.
    assert_eq!(
        settings::COMBO_LIST_ITEM_EXTRA,
        12,
        "the air of a dropped-down list item is the number T-11-15 left; T-12-2 does not move it"
    );
    assert_eq!(
        15 + theme::scaled(settings::COMBO_LIST_ITEM_EXTRA, 96),
        24,
        "a list item must still be the 24 px `CB_GETITEMHEIGHT(0)` answered before T-12-2"
    );

    assert!(
        theme::scaled(settings::COMBO_CLOSED_ITEM_EXTRA, 96)
            < theme::scaled(settings::COMBO_LIST_ITEM_EXTRA, 96),
        "the closed part is the shorter of the two on the screen — that is the whole of F3"
    );

    // ---- and the source: which number goes where, and by which lever ---------------------
    let source = settings_module_source();

    // The lie is gone with the name that carried it. «33 mock-up pixels» was never a length of
    // the generator, and the old single name is what the next task would have re-used.
    for gone in [
        "COMBO_ITEM_EXTRA:",
        "scaled(COMBO_ITEM_EXTRA",
        "33 of their own pixels high",
        "33 ÷ 1,4",
    ] {
        assert!(
            !source.contains(gone),
            "`{gone}` must be gone from src\\settings.rs — the closed part is 12 DLU of the \
             generator, and the 33 px it used to be derived from were read off a picture"
        );
    }

    // The derivation lives where the number lives.
    for literal in ["`h = 12` dialog units", "22,5 px at 96 DPI"] {
        assert!(
            source.contains(literal),
            "the doc comment of the closed-part air must derive it from the generator's \
             literal — `{literal}` is missing"
        );
    }

    // `WM_MEASUREITEM` measures the list, and only the list.
    let measured = function_body(&source, "unsafe fn on_measure_item(");

    assert!(
        measured.contains("combo_row_height(hwnd, COMBO_LIST_ITEM_EXTRA)"),
        "`on_measure_item` must answer the height of a *list item*"
    );
    assert!(
        !measured.contains("COMBO_CLOSED_ITEM_EXTRA"),
        "the closed part must not be measured by `WM_MEASUREITEM` — one answer there would \
         shrink the rows of the dropped-down list with it"
    );

    // SEC-05: the gate is still the first thing that message does, before any work.
    let gate = measured
        .find("if !owner_drawn_item(")
        .expect("`on_measure_item` must still gate on type and identifier — SEC-05");
    let work = measured
        .find("combo_row_height(")
        .expect("`on_measure_item` must still measure something");

    assert!(
        gate < work,
        "SEC-05: the type and the identifier are checked before any work is done"
    );

    // The closed part is set apart by the documented lever, on all four combo boxes, and the
    // refusal of the send is examined (NFR-13) rather than dropped.
    let closed = function_body(&source, "fn set_combo_closed_height(");

    for shape in [
        "combo_row_height(hwnd, COMBO_CLOSED_ITEM_EXTRA)",
        "for control in COMBO_BOXES",
        "CB_SETITEMHEIGHT",
        "CB_SELECTION_FIELD",
        "if answer == CB_ERR as isize",
    ] {
        assert!(
            closed.contains(shape),
            "`set_combo_closed_height` must carry `{shape}`"
        );
    }

    // −1 is the documented `wParam` for the selection field, and it is a name here.
    assert!(
        source.contains("const CB_SELECTION_FIELD: usize = usize::MAX;"),
        "the −1 of CB_SETITEMHEIGHT must be a named constant and not a bare cast"
    );

    // And it is sent once, from `WM_INITDIALOG`, where the four controls exist.
    let filled = function_body(&source, "fn fill_dialog(");

    assert!(
        filled.contains("set_combo_closed_height(hwnd)"),
        "`fill_dialog` must set the closed-part height once, on WM_INITDIALOG"
    );
    assert_eq!(
        source.matches("set_combo_closed_height(hwnd)").count(),
        1,
        "the height must be set from exactly one place"
    );

    // The exclusion list is untouched by all of this — it has had its own dialog units since
    // task T-11-16, and T-12-2 was told to leave that branch alone.
    assert!(
        measured.contains("dialog_units(hwnd, 0, EXCLUSION_ROW_HEIGHT_DLU)"),
        "the row of the exclusion list must still be its own 11 dialog units"
    );
}

// -----------------------------------------------------------------------------------------
// The about dialog — FR-92а, task T-11-11, read out of the built binary like everything else
// -----------------------------------------------------------------------------------------

#[test]
fn the_about_dialog_template_carries_its_elements() {
    // Criterion 9 of T-11-11, by the instrument of T-08-1: the template of the window that
    // replaced `MessageBoxW`, read out of the built `LangSwitcher.exe` and compared against
    // literals written out here.
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_ABOUT));

    println!("caption: {}", template.caption);
    for text in &template.text {
        println!("  {text}");
    }

    // Task T-12-4, решение В-2: «О программе» and not «О программе Lang Switcher» — the name of
    // the product stands in the window itself, on the row beside the logo, and the caption of
    // the mock-up carries the two words alone (`chrome.ps1:186`).
    assert_eq!(
        template.caption, "О программе",
        "the caption came out of rc.exe wrong — check #pragma code_page(65001)"
    );

    // Every element the task names: the icon, the name row, the version row, the two
    // description lines, the «Как пользоваться» block of task Т-23-4 and the one button. One
    // row per element, so a lost control is a failing test and not a smaller window.
    const ABOUT_CONTROLS: [(u32, &str); 18] = [
        (1120, "иконка программы"),
        (1121, "имя программы"),
        (1122, "строка версии"),
        (1123, "первая строка описания"),
        (1124, "вторая строка описания"),
        (1125, "Как пользоваться: панель"),
        (1126, "Как пользоваться: номер 1"),
        (1127, "Как пользоваться: номер 2"),
        (1128, "Как пользоваться: номер 3"),
        (1129, "Как пользоваться: номер 4"),
        (1130, "Как пользоваться: номер 5"),
        (1131, "Как пользоваться: строка 1"),
        (1132, "Как пользоваться: строка 2"),
        (1133, "Как пользоваться: строка 3"),
        (1134, "Как пользоваться: строка 4"),
        (1135, "Как пользоваться: строка 5"),
        // Task Т-32-4, FR-103: the one control this window gained — the way into «От автора».
        (1136, "кнопка «От автора…»"),
        (1, "кнопка «ОК»"),
    ];

    for (id, what) in ABOUT_CONTROLS {
        assert!(
            template.controls.contains(&id),
            "the about dialog has no control {id} — {what}"
        );
    }

    assert_eq!(
        template.controls.len(),
        ABOUT_CONTROLS.len(),
        "the about dialog carries exactly its eighteen controls and nothing else"
    );

    // The visible literals, in template order. The version row is empty on purpose — it is
    // composed at run time from the version resource — the five help rows are empty for the
    // same reason (the key name is substituted into them), and the icon control carries an
    // ordinal, not a text, so none of them appears here. The five numerals do: a digit is a
    // template literal with no string row, exactly as «Lang Switcher» is.
    assert_eq!(
        template.text,
        [
            "Lang Switcher",
            "Исправляет текст, набранный в неверной раскладке.",
            "Перекодировка — по нажатию одной клавиши.",
            "Как пользоваться",
            "1",
            "2",
            "3",
            "4",
            "5",
            // Task Т-32-4: the button that leads to «От автора». Its caption is a template
            // literal like every other, and FR-94 replaces it on the live window.
            "От автора…",
            "ОК",
        ],
        "the visible strings of the about dialog came out of rc.exe wrong"
    );
}

/// **Т-23-4, решение 82.5** — the «Как пользоваться» panel, in the numbers of the built
/// template.
///
/// The block is drawn from the rectangle of a hidden control, the way the six panels of the
/// settings dialog are, so «is this label on the panel» is arithmetic on the template and
/// needs no window. Three facts: the window grew to the height the rows really need, the
/// panel is not a visible element, and every one of the ten statics lies inside it.
#[test]
fn the_about_window_carries_the_help_panel_of_the_accepted_mock_up() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_ABOUT));

    println!("IDD_ABOUT: {:?} dialog units", template.size);

    // ⚠ 233 and not the «≈175» of the mock-up: the estimate assumed one line per row, and the
    // rows measured on the raised window need ten lines between them, not five. The width is
    // the 191 it has always been — решение 82.5 says the window grows downwards only.
    //
    // ⚠ **233 → 256 by решение 87** (task Т-26-2): the body of this window is set one step
    // above the dialog font since that decision, and its lines stand 1,4 of the face apart
    // instead of the 1,33 `DrawTextW` gives — every row of text takes more room, and the room
    // is taken downwards. The number moves by a decision of the user and never by a hand that
    // found it inconvenient (правило [[canon]]); the width is untouched, as both decisions say.
    assert_eq!(
        template.size,
        (191, 256),
        "the about window of решения 82.5 и 87 is 191 × 256 dialog units"
    );

    // The panel is a hidden control: `NOT WS_VISIBLE` in the template, `BS_OWNERDRAW` as its
    // button type. The same two facts the six panels of the settings dialog are held to, and
    // for the same reason — an invisible window takes no click and paints nothing.
    let panel_style = template.style_of(1125, "Как пользоваться: панель");

    println!("панель (1125): style {panel_style:#010x}");

    assert_eq!(
        panel_style & 0x0F,
        0x0B,
        "the panel must carry BS_OWNERDRAW as its button type; the style is {panel_style:#010x}"
    );
    assert_eq!(
        panel_style & WS_VISIBLE,
        0,
        "the panel must not be a visible element; the style is {panel_style:#010x}"
    );
    assert_eq!(
        panel_style & 0x0001_0000,
        0,
        "a panel takes no focus and must not gain WS_TABSTOP; the style is {panel_style:#010x}"
    );

    let (pl, pt, pr, pb) = template.rect_of(1125);

    println!("панель (1125): {pl},{pt}..{pr},{pb}");

    // The ten statics of the block, every one inside the panel — and the panel itself inside
    // the window, which is what makes «inside» mean anything.
    assert!(
        pl >= 0 && pt >= 0 && pr <= template.size.0 && pb <= template.size.1,
        "the panel must lie inside the window"
    );

    for (id, what) in OWNER_DRAWN_ABOUT_LABELS.iter().skip(4) {
        let (left, top, right, bottom) = template.rect_of(*id);

        println!("«{what}» ({id}): {left},{top}..{right},{bottom}");

        assert!(
            left >= pl && top >= pt && right <= pr && bottom <= pb,
            "«{what}» ({id}) sticks out of the panel it is drawn on"
        );
    }

    // And the list the drawing reads is the list the template declares: a label on the panel
    // is filled with the panel's colour, and one left out of `ABOUT_LABELS_ON_THE_PANEL` would
    // cut a window-coloured hole in the block.
    let mut declared: Vec<i32> = OWNER_DRAWN_ABOUT_LABELS
        .iter()
        .skip(4)
        .map(|(id, _)| *id as i32)
        .collect();
    let mut listed = settings::ABOUT_LABELS_ON_THE_PANEL.to_vec();
    declared.sort_unstable();
    listed.sort_unstable();

    assert_eq!(
        listed, declared,
        "the module's list of labels on the panel and the template's have parted"
    );

    // Every rectangle of this template is disjoint from every other, and it is load-bearing:
    // an SS_OWNERDRAW static fills its whole rectangle before it writes a word, so two
    // overlapping ones erase each other on every repaint. The panel is excluded — it *is* the
    // ground the ten stand on, and its own drawing runs before theirs.
    for (first, x1, y1, cx1, cy1) in &template.bounds {
        for (second, x2, y2, cx2, cy2) in &template.bounds {
            if first >= second || *first == 1125 || *second == 1125 {
                continue;
            }

            let apart = x1 + cx1 <= *x2 || x2 + cx2 <= *x1 || y1 + cy1 <= *y2 || y2 + cy2 <= *y1;

            assert!(
                apart,
                "controls {first} and {second} overlap — one of them erases the other on \
                 every repaint"
            );
        }
    }
}

// The same ⚠ as for the nine buttons of the settings dialog: `BS_OWNERDRAW` (0x0B) is a
// button *type* in the low nibble, so the check is equality of the nibble and not a bit
// test.
#[test]
fn the_about_ok_button_is_owner_drawn_and_keeps_its_tab_stop() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_ABOUT));

    let style = template
        .styles
        .iter()
        .find(|(control, _)| *control == 1)
        .map(|(_, style)| *style)
        .unwrap_or_else(|| panic!("the about dialog has no control 1 — «ОК»"));

    println!("«ОК» (1): style {style:#010x}");

    assert_eq!(
        style & 0x0F,
        0x0B,
        "«ОК» (1) must carry BS_OWNERDRAW as its button type — FR-92а; \
         the style is {style:#010x}"
    );

    assert_ne!(
        style & 0x0001_0000,
        0,
        "«ОК» (1) must keep WS_TABSTOP; the style is {style:#010x}"
    );
}

#[test]
fn the_about_template_says_what_the_russian_table_says() {
    // Two independent witnesses, exactly as for the settings dialog: the template literals
    // keep the window readable if a string fails to load, the table is what the program
    // actually shows, and a code page accident would have to corrupt both identically.
    let product = ProductImage::shared();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_ABOUT));

    assert_eq!(
        template.caption,
        product.string(settings::Language::Ru, settings::IDS_ABOUT_CAPTION),
        "the caption of the about template and of the Russian table have parted"
    );

    for (text, id) in [
        (
            "Исправляет текст, набранный в неверной раскладке.",
            settings::IDS_ABOUT_LINE_1,
        ),
        (
            "Перекодировка — по нажатию одной клавиши.",
            settings::IDS_ABOUT_LINE_2,
        ),
        ("ОК", settings::IDS_ABOUT_OK),
        // Task Т-23-4: the caption of the «Как пользоваться» panel is a template literal like
        // the rest — the window stays readable if a string fails to load — and FR-94 replaces
        // it on the very control the block is drawn from. The five rows below it are **not**
        // here: they are empty in the template, because the key name is substituted into them.
        ("Как пользоваться", settings::IDS_ABOUT_HELP),
    ] {
        assert_eq!(
            product.string(settings::Language::Ru, id),
            text,
            "row {id} of the Russian table and the about template have parted"
        );
        assert!(
            template.text.iter().any(|title| title == text),
            "the about template does not show «{text}»"
        );
    }
}

#[test]
fn the_about_strings_continue_the_row_of_fr_94_without_holes() {
    // Criterion 10 of T-11-11, the numbering half: the five identifiers continue the row
    // at 3061 contiguously — written as literals, so a renumbering in the crate cannot
    // silently agree with itself.
    assert_eq!(settings::IDS_ABOUT_CAPTION, 3061);
    assert_eq!(settings::IDS_ABOUT_VERSION, 3062);
    assert_eq!(settings::IDS_ABOUT_LINE_1, 3063);
    assert_eq!(settings::IDS_ABOUT_LINE_2, 3064);
    assert_eq!(settings::IDS_ABOUT_OK, 3065);

    // And the row they continue ends right in front of them.
    assert_eq!(settings::IDS_THEME_DARK, 3060);

    // Task Т-23-4 — the six rows of the «Как пользоваться» panel take the rest of the block
    // 3056 opened, contiguously with the five above and stopping right in front of the menu.
    assert_eq!(settings::IDS_ABOUT_HELP, 3066);
    assert_eq!(settings::IDS_ABOUT_HELP_1, 3067);
    assert_eq!(settings::IDS_ABOUT_HELP_2, 3068);
    assert_eq!(settings::IDS_ABOUT_HELP_3, 3069);
    assert_eq!(settings::IDS_ABOUT_HELP_4, 3070);
    assert_eq!(settings::IDS_ABOUT_HELP_5, 3071);
    assert_eq!(
        settings::IDS_MENU_SUSPEND,
        3072,
        "the menu block starts at 3072 and the help must stop in front of it"
    );
}

/// **Т-23-4** — the panel of the about window is drawn where a block is drawn, and its caption
/// comes off its own control.
///
/// The same road the six panels of the settings dialog take, and the same reason for taking
/// it: a block is the *background* of what stands on it, so it is laid by the window's erase
/// and never by a `WM_DRAWITEM` that could arrive after its children have painted. Read off
/// `src\settings.rs` — the shape of the two functions, not their output.
#[test]
fn the_help_panel_is_drawn_by_the_windows_own_erase_and_reads_its_caption_off_its_control() {
    let source = settings_module_source();
    let erase = function_body(&source, "unsafe fn on_about_erase_background(");

    assert!(
        erase.contains("IDC_ABOUT_HELP"),
        "the about window's erase must draw the panel of решение 82.5"
    );
    assert!(
        erase.contains("paint_rounded("),
        "the panel is the same rounded figure the six panels of the settings dialog are"
    );
    assert!(
        erase.contains("draw_panel_caption(hwnd, IDC_ABOUT_HELP"),
        "the caption must be read off the panel's own control, so FR-94 reaches it by the \
         one road it reaches every other piece of text"
    );

    // And no second road: a `WM_DRAWITEM` that painted the block would paint it over the
    // labels standing on it, which is the very defect task T-11-13 closed in the other window.
    let draw = function_body(&source, "unsafe fn on_about_draw_item(");

    assert!(
        !draw.contains("IDC_ABOUT_HELP,"),
        "the panel must not be drawn from a WM_DRAWITEM — see task T-11-13"
    );
}

/// **Т-23-4, решение 82.5** — the help names the key that is really in force.
///
/// `[hotkey] key` is a name in a text file, and a name this build does not know leaves the
/// default of section 7 acting (`app::publish_configuration` says so, and the settings window
/// puts a note under the field). A help panel that repeated the file's word would name a key
/// that does nothing at all — the one thing решение 82.5 is against.
#[test]
fn the_help_of_the_about_window_names_the_key_that_is_really_in_force() {
    // A key this build knows is shown as the file spells it.
    assert_eq!(settings::effective_hotkey_name("Pause"), "Pause");
    assert_eq!(settings::effective_hotkey_name("ScrollLock"), "ScrollLock");
    assert_eq!(settings::effective_hotkey_name("F9"), "F9");

    // A name nobody knows falls back to the default of section 7 — the key that really acts.
    assert_eq!(settings::effective_hotkey_name("Хрюкозябра"), "Pause");
    assert_eq!(settings::effective_hotkey_name(""), "Pause");

    // ⚠ A **text** key is deliberately not replaced: `Q` really is the hotkey when the file
    // says so — FR-95 warns about it in the settings window, and the help telling the truth
    // about the user's own configuration is the point.
    assert_eq!(settings::effective_hotkey_name("Q"), "Q");

    // And the fallback is the default of section 7 itself, not a second copy of the word.
    assert_eq!(
        Config::default().hotkey.key,
        settings::effective_hotkey_name("нет такой клавиши"),
        "the fallback of the help is the default of section 7"
    );
}

/// **Т-23-4** — the five rows of the help substitute the key name, and only the first three
/// have a place for it.
///
/// The `{0}` is filled by `settings::format_text`, the same substitution the version line uses
/// — so this test reads the rows out of the **built** binary and does the substitution the
/// window does, in both locales.
#[test]
fn the_help_rows_substitute_the_key_name_where_the_mock_up_names_a_key() {
    let _guard = with_product_strings();

    for language in [settings::Language::Ru, settings::Language::En] {
        settings::set_ui_language(language);

        let rows: Vec<String> = [
            settings::IDS_ABOUT_HELP_1,
            settings::IDS_ABOUT_HELP_2,
            settings::IDS_ABOUT_HELP_3,
            settings::IDS_ABOUT_HELP_4,
            settings::IDS_ABOUT_HELP_5,
        ]
        .iter()
        .map(|id| settings::format_text(*id, &["ScrollLock"]))
        .collect();

        println!("{language:?}: {rows:#?}");

        for (index, row) in rows.iter().enumerate() {
            assert!(!row.is_empty(), "row {index} of the help is empty");
            assert!(
                !row.contains("{0}"),
                "row {index} kept its placeholder: «{row}»"
            );
            assert_eq!(
                row.contains("ScrollLock"),
                index < 3,
                "row {index} must {} the key name: «{row}»",
                if index < 3 { "carry" } else { "not carry" }
            );
            // The mock-up's own word: no row of the help spells a key of its own, or the
            // window would tell a person about a key their program does not answer to.
            assert!(
                !row.contains("Pause"),
                "row {index} spells a key name of its own: «{row}»"
            );
        }
    }

    settings::set_ui_language(settings::Language::Ru);
}

#[test]
fn the_about_version_line_substitutes_the_number_the_resource_gave() {
    // Criterion 12 of T-11-11: the version is *substituted* into the string of the locale
    // in force — the number itself came out of the `VERSIONINFO` resource by
    // `tray::file_version`, and no version literal exists in the source.
    let _guard = with_product_strings();

    settings::set_ui_language(settings::Language::Ru);
    let russian = settings::about_version_line(Some((0, 1, 0, 0)));
    let russian_missing = settings::about_version_line(None);
    // The fourth part is deliberately not zero here: a revision of 7 must change nothing in
    // the line — see the ⚠ below.
    let russian_revised = settings::about_version_line(Some((0, 1, 0, 7)));

    settings::set_ui_language(settings::Language::En);
    let english = settings::about_version_line(Some((0, 1, 0, 0)));

    println!("ru: {russian} / {russian_missing} / {russian_revised}");
    println!("en: {english}");

    // ⚠ **Three parts and a lower-case letter since task T-12-4** — finding A-12 and решение
    // В-2: the mock-up's line is «версия 0.1.0» (`chrome.ps1:195`), and the fourth part is what
    // made the live line longer than the model's at the same point size. The revision is still
    // *read* — the caller hands over whatever the resource holds — and only the sentence a
    // person reads leaves it out.
    assert_eq!(russian, "версия 0.1.0");
    assert_eq!(
        russian_revised, "версия 0.1.0",
        "the revision is dropped from the line, whatever it is"
    );
    // A binary without the resource shows a dash rather than failing — the reading the old
    // box gave the same case.
    assert_eq!(russian_missing, "версия —");
    assert_eq!(english, "version 0.1.0");

    settings::set_ui_language(settings::Language::Ru);
}

#[test]
fn the_about_static_colour_roles_follow_their_table() {
    // The role function the about dialog's `WM_CTLCOLORSTATIC` answers with — the closed
    // vocabulary of the settings dialog, reused: the version line **and the two description
    // lines** are muted, the icon and the name are ordinary captions, and nothing in that
    // window is a field.
    //
    // ⚠ The two description lines moved from `Label` to `Muted` in task T-12-4 — finding A-06:
    // the mock-up paints the paragraph under the name with the muted brush (`chrome.ps1:196`
    // draws it with `$brMu`, while line 194 draws the name with `$brFg`), and решение В-2 says
    // in as many words that the description stays «в колонке имени и приглушённым цветом».
    //
    // ⚠ Task Т-23-4 added the ten statics of «Как пользоваться», and they split: the numeral
    // of a row is the mock-up's `.key` — muted — and the sentence beside it is the row text,
    // which the mock-up leaves in the full-strength ink.
    let cases = [
        (1120, theme::StaticColorRole::Label, "иконка"),
        (1121, theme::StaticColorRole::Label, "имя"),
        (1122, theme::StaticColorRole::Muted, "строка версии"),
        (1123, theme::StaticColorRole::Muted, "описание, строка 1"),
        (1124, theme::StaticColorRole::Muted, "описание, строка 2"),
        (1126, theme::StaticColorRole::Muted, "справка: номер 1"),
        (1127, theme::StaticColorRole::Muted, "справка: номер 2"),
        (1128, theme::StaticColorRole::Muted, "справка: номер 3"),
        (1129, theme::StaticColorRole::Muted, "справка: номер 4"),
        (1130, theme::StaticColorRole::Muted, "справка: номер 5"),
        (1131, theme::StaticColorRole::Label, "справка: строка 1"),
        (1132, theme::StaticColorRole::Label, "справка: строка 2"),
        (1133, theme::StaticColorRole::Label, "справка: строка 3"),
        (1134, theme::StaticColorRole::Label, "справка: строка 4"),
        (1135, theme::StaticColorRole::Label, "справка: строка 5"),
    ];

    for (control, expected, what) in cases {
        assert_eq!(
            settings::about_static_color_role(control),
            expected,
            "{what} ({control})"
        );
    }
}

// -----------------------------------------------------------------------------------------
// The parts of the dialog that are pure logic
// -----------------------------------------------------------------------------------------

/// Two layouts of different languages, as a session normally has them.
const EN: layouts::LayoutId = layouts::LayoutId::from_raw(0x0409_0409);
/// The Russian one.
const RU: layouts::LayoutId = layouts::LayoutId::from_raw(0x0419_0419);
/// A second English layout, loaded under an explicit layout identifier.
const EN_SECOND: layouts::LayoutId = layouts::LayoutId::from_raw(0xF001_0409);

#[test]
fn the_cycle_list_starts_with_the_configured_order() {
    let mut config = Config::default();
    config.layouts.cycle = vec!["0x00000419".to_owned()];

    let rows = settings::layout_rows(&config.layouts, &[EN, RU]);

    println!("{rows:?}");

    assert_eq!(
        rows,
        vec![
            LayoutRow {
                layout: RU,
                checked: true
            },
            LayoutRow {
                layout: EN,
                checked: false
            },
        ],
        "FR-31: the configured order is the cycle, and it comes first"
    );

    // And back again, which is the round trip the dialog makes on every apply.
    assert_eq!(
        settings::cycle_from_rows(&rows, &[EN, RU]),
        vec!["0x00000419".to_owned()]
    );
}

#[test]
fn a_layout_the_session_does_not_have_is_dropped_from_the_list() {
    let mut config = Config::default();
    config.layouts.cycle = vec!["0x0000040C".to_owned(), "0x00000409".to_owned()];

    let rows = settings::layout_rows(&config.layouts, &[EN, RU]);

    assert_eq!(rows.len(), 2, "the session has two layouts and no more");
    assert_eq!(rows[0].layout, EN);
    assert!(rows[0].checked);
    assert!(!rows[1].checked, "RU is not in the cycle of this file");
}

#[test]
fn one_layout_per_language_is_written_in_the_language_form() {
    // The form section 7 prints, and the one that survives a new session: the handle is
    // assigned by the session, the language identifier is not.
    assert_eq!(settings::spec_text(EN, &[EN, RU]), "0x00000409");
    assert_eq!(settings::spec_text(RU, &[EN, RU]), "0x00000419");

    // Two layouts of one language can only be told apart by the handle, so the handle is what
    // is written. Both forms are read by `layouts::LayoutSpec`, so the schema is not extended.
    assert_eq!(settings::spec_text(EN, &[EN, EN_SECOND, RU]), "0x04090409");
    assert_eq!(
        settings::spec_text(EN_SECOND, &[EN, EN_SECOND, RU]),
        "0xF0010409"
    );
}

// -----------------------------------------------------------------------------------------
// Т-23-3, решение 82.1 — вариант В: имя раскладки чистое, различитель только при коллизии
// -----------------------------------------------------------------------------------------
//
// The user's complaint was the tail: «Русский (Россия) — 0x04190419» in three places of the
// window, where the number means nothing to the person reading it and the name alone is
// what tells the two layouts apart — until it is not. Вариант В shows the name bare and adds
// the identifier back **only to the strings that would otherwise be identical**.
//
// Two halves. The rule itself is pure — it is a count of equal names over a list of pairs and
// nothing else — and is closed here on names written by hand rather than asked of a locale
// service; that no window and no system are needed is the point of the half. The wiring is
// closed on the layout identifiers the other tests of this file already use: `EN` and
// `EN_SECOND` share a language identifier, so the system gives them one and the same display
// name, which is exactly the collision the rule is for.

#[test]
fn a_layout_name_carries_a_discriminator_only_when_it_collides() {
    // No collision: three different names, three bare labels, not a hexadecimal digit in
    // sight — and the discriminators are present and deliberately unused.
    let apart = settings::disambiguated_labels(&[
        (
            Some("English (United States)".to_owned()),
            "0x00000409".to_owned(),
        ),
        (Some("Русский (Россия)".to_owned()), "0x00000419".to_owned()),
        (
            Some("Deutsch (Deutschland)".to_owned()),
            "0x00000407".to_owned(),
        ),
    ]);

    println!("{apart:?}");

    assert_eq!(
        apart,
        vec![
            "English (United States)",
            "Русский (Россия)",
            "Deutsch (Deutschland)"
        ],
        "a name nobody else wears is shown as it is — решение 82.1"
    );

    // A collision of two: **both** get the tail, not just the second. A discriminator on one
    // of a pair tells the reader nothing about which one they are looking at.
    let together = settings::disambiguated_labels(&[
        (
            Some("English (United States)".to_owned()),
            "0x04090409".to_owned(),
        ),
        (Some("Русский (Россия)".to_owned()), "0x00000419".to_owned()),
        (
            Some("English (United States)".to_owned()),
            "0xF0010409".to_owned(),
        ),
    ]);

    println!("{together:?}");

    assert_eq!(
        together,
        vec![
            "English (United States) — 0x04090409",
            "Русский (Россия)",
            "English (United States) — 0xF0010409"
        ],
        "both of the pair carry the identifier, and the third name is left alone"
    );

    // A name the system would not give: the bare identifier, exactly as before Т-23-3. A
    // nameless row is not a collision — it has no name to collide with — and two of them come
    // out different because their identifiers are.
    let nameless = settings::disambiguated_labels(&[
        (None, "0x04090409".to_owned()),
        (None, "0xF0010409".to_owned()),
        (Some("Русский (Россия)".to_owned()), "0x00000419".to_owned()),
    ]);

    println!("{nameless:?}");

    assert_eq!(
        nameless,
        vec!["0x04090409", "0xF0010409", "Русский (Россия)"],
        "the fallback of a nameless layout is the bare identifier it always was"
    );
}

/// **Т-23-3, решение 82.1 — the wiring, on this machine's own locale service.**
///
/// The names come from `LCIDToLocaleName` + `GetLocaleInfoEx`, and what they *say* is not
/// asserted here: this test is about the shape the rule gives them. ⚠ Until task Т-27-1 the
/// reason was stronger than that — a localised name said whatever the Windows of the machine
/// said, so there was nothing stable to write down. Вопрос 92 took that away: a native name
/// is the same on every Windows, and the two tests of Т-27-1 below write both of this
/// machine's out in full. What is asserted here is still only the shape, and it is
/// machine-independent: a language nobody shares is shown without a tail, a
/// language two layouts share is shown with one, and the tail is **character for character
/// what section 7 would store for that layout** — the string `spec_text` produces, so a person
/// who reads the identifier off the window can find it in `config.toml`.
#[test]
fn the_discriminator_appears_on_a_collision_and_is_the_identifier_section_7_stores() {
    // One layout per language: no collision anywhere, so no label carries an identifier.
    let apart = settings::layout_labels(&[EN, RU], &[EN, RU]);

    println!("две раскладки разных языков: {apart:?}");

    for label in &apart {
        assert!(
            !label.is_empty(),
            "a layout with no label at all would be a row the user cannot choose"
        );
        assert!(
            !label.contains("0x"),
            "«{label}» still carries an identifier nobody needs — решение 82.1"
        );
    }

    // Two layouts of one language: the system gives both the same display name, so both — and
    // only they — take the tail.
    let session = [EN, EN_SECOND, RU];
    let together = settings::layout_labels(&session, &session);

    println!("две раскладки одного языка: {together:?}");

    for (index, layout) in session.iter().enumerate() {
        let expected_tail = format!(" — {}", settings::spec_text(*layout, &session));
        let collides = index < 2;

        assert_eq!(
            together[index].ends_with(&expected_tail),
            collides,
            "«{}» must {} the tail «{expected_tail}»",
            together[index],
            if collides { "carry" } else { "not carry" }
        );
    }

    // And the name in front of the tail is the same name the third row wears bare — the tail
    // is added to a label, it does not replace one.
    assert_eq!(
        together[0].trim_end_matches(&format!(" — {}", settings::spec_text(EN, &session))),
        together[1].trim_end_matches(&format!(" — {}", settings::spec_text(EN_SECOND, &session))),
        "the two colliding rows show one and the same name"
    );
    assert_eq!(
        together[2], apart[1],
        "the row that collides with nobody reads the same in both sessions"
    );
}

/// **Т-23-3** — all three places the user sees a layout name go through one function.
///
/// The two combo boxes are filled together in `fill_layouts` and the cycle list in
/// `fill_cycle_list`; a fourth place that built a label of its own would be a place where the
/// rule of решение 82.1 could quietly not hold. Read off `src\settings.rs` — the sweep looks
/// for the batch function and for the absence of the per-layout one it replaced.
#[test]
fn every_place_that_shows_a_layout_name_asks_the_one_function() {
    let source = settings_module_source();

    for signature in ["fn fill_layouts(", "fn fill_cycle_list("] {
        assert!(
            function_body(&source, signature).contains("layout_labels("),
            "`{signature}` must take its labels from `layout_labels`, which is where the \
             collision rule of решение 82.1 lives"
        );
    }

    assert!(
        !source.contains("fn layout_label("),
        "`layout_label` built one label out of one layout and could not see a collision at \
         all — Т-23-3 replaced it with `layout_labels`"
    );
}

// -----------------------------------------------------------------------------------------
// Т-27-1, вопрос 92 — имя раскладки: как язык называет себя сам, с заглавной первой буквы
// -----------------------------------------------------------------------------------------
//
// The user's observation was a window in English showing its layouts in Russian. The cause
// was not a defect but a property of the construction: `LOCALE_SLOCALIZEDDISPLAYNAME` is the
// name the **Windows** interface language uses, and it follows the system, never the
// program. Вопрос 92 moved the names to `LOCALE_SNATIVEDISPLAYNAME` — each language named as
// it names itself — and asked for the first letter to be raised, because that is the one
// thing the native names do not agree on.
//
// ⚠ What the two tests below assert is exactly what the old ones could not: a native name
// does not depend on the Windows the test runs on, so the strings are machine-independent
// and may be written out. They are the live values of this machine, taken with an instrument
// and not from memory (`scratchpad-Э27\прибор-имена.log`):
//
// | язык | `LOCALE_SLOCALIZEDDISPLAYNAME` | `LOCALE_SNATIVEDISPLAYNAME` |
// |---|---|---|
// | `0x0409` | «Английский (США)» | «English (United States)» |
// | `0x0419` | «Русский (Россия)» | «русский (Россия)» — строчная `U+0440` |

/// **Т-27-1 — красное «до», и его носитель.**
///
/// On this machine — a Russian Windows — the English layout read «Английский (США)» before
/// вопрос 92, in a program window switched to English as readily as in a Russian one. This
/// test is the one that was red on `508fee8` and is the honest proof that the constant moved.
#[test]
fn an_english_layout_is_named_in_english_however_the_system_is_localised() {
    let labels = settings::layout_labels(&[EN], &[EN]);

    println!("0x0409: {labels:?}");

    assert_eq!(
        labels,
        vec!["English (United States)"],
        "the English layout must name itself in English — вопрос 92; a localised name would \
         follow the language of Windows and not of the layout"
    );
}

/// **Т-27-1 — регресс-сторож капитализации, и не красное «до».**
///
/// ⚠ This one was green before the change as well, and for a different reason: the localised
/// name of `ru-RU` on a Russian Windows already read «Русский (Россия)» with a capital. The
/// native name does not — the system returns «русский (Россия)», `U+0440`, measured — so the
/// same string now stands only because the capitalisation puts the letter back up. Take the
/// capitalisation away and this test goes red; that is what it is here for.
#[test]
fn a_russian_layout_keeps_its_capital_letter_though_the_native_name_has_none() {
    let labels = settings::layout_labels(&[RU], &[RU]);

    let first: Vec<String> = labels[0]
        .chars()
        .take(1)
        .map(|character| format!("U+{:04X}", u32::from(character)))
        .collect();

    println!("0x0419: {labels:?}, первый знак {first:?}");

    assert_eq!(
        labels,
        vec!["Русский (Россия)"],
        "the native name of Russian starts with a lower-case «р» and a list of layouts is not \
         running prose — вопрос 92 raises the first letter"
    );
}

/// **Т-27-1 — сама капитализация, на голой строке-образце.**
///
/// This machine has two layouts and one of them is English, whose native name is already
/// capital: a capitalisation that did nothing at all would pass the English test above. So
/// the rule is closed here without a locale service, on strings — including the ones no
/// machine of this project will ever be asked about.
#[test]
fn the_first_character_is_raised_whole_and_the_rest_of_the_name_is_left_alone() {
    // The live case of вопрос 92, measured off this machine: `LOCALE_SNATIVEDISPLAYNAME`
    // returns the adjective in lower case, and «Россия» behind it must stay as it is.
    assert_eq!(
        settings::capitalised("русский (Россия)"),
        "Русский (Россия)"
    );

    // A name that is already right comes back untouched — which is exactly why the English
    // layout could not close the capitalisation on its own.
    assert_eq!(
        settings::capitalised("English (United States)"),
        "English (United States)"
    );

    // The same lower-case habit in another language, and inner capitals again left alone.
    assert_eq!(
        settings::capitalised("español (España)"),
        "Español (España)"
    );

    // A script with no case at all: nothing to raise, and nothing lost on the way through.
    assert_eq!(settings::capitalised("日本語 (日本)"), "日本語 (日本)");

    // ⚠ The case the **whole** iterator is for. No language names itself this way, but the
    // form of the code is decided by it all the same: `ß` raises into two characters, and an
    // implementation that took `to_uppercase().next()` returns «S-lein» — half the letter,
    // measured on that very implementation and not imagined. This is the only assertion here
    // that tells the two forms apart.
    assert_eq!(settings::capitalised("ß-lein"), "SS-lein");

    // A name the system gave empty is not a panic and not a space — it is an empty name, and
    // `layout_labels` decides what to do with it.
    assert_eq!(settings::capitalised(""), "");
}

// ⚠ `a_millisecond_field_reads_as_a_number_and_never_as_a_guess` stood here until task
// Т-23-2. It closed `settings::parse_ms`, which turned the text of a millisecond field back
// into a number; решение 81 took all three millisecond fields out of the dialog, so the
// function has no caller and no field to parse and left with them. The **values** did not:
// `replacement.inter_event_delay_ms` and the two timings of §4.7 stay in `config.toml`,
// where serde reads them, and their ceilings are `selection`'s and `inject`'s own — closed
// by the tests of those modules, not by this one.

#[test]
fn the_hotkey_section_warns_about_a_text_key_and_about_a_name_nobody_knows() {
    // The default of section 7 is a key that types nothing, so there is nothing to say.
    assert_eq!(settings::hotkey_note("Pause"), None);
    assert_eq!(settings::hotkey_note("ScrollLock"), None);
    assert_eq!(settings::hotkey_note("F9"), None);

    // FR-92: «предупреждение при выборе текстовой клавиши». Since FR-94 the answer is the
    // identifier of an interface string rather than the string itself, so the text is fetched
    // out of the built binary — in both locales, because a warning that exists in one language
    // only is half a warning.
    let product = ProductImage::shared();

    let letter = settings::hotkey_note("A").expect("a letter must be warned about");
    let letter_ru = product.string(settings::Language::Ru, letter);
    let letter_en = product.string(settings::Language::En, letter);
    println!("A -> {letter} -> ru: {letter_ru}\n         -> en: {letter_en}");
    assert!(letter_ru.contains("текстовая"));
    assert!(letter_en.contains("text key"));

    // And the other silent failure: a name this build does not know leaves `Pause` in force,
    // which `app::publish_configuration` does without telling anybody. This is the telling.
    let unknown = settings::hotkey_note("Ктулху").expect("an unknown name must be reported");
    let unknown_ru = product.string(settings::Language::Ru, unknown);
    let unknown_en = product.string(settings::Language::En, unknown);
    println!("unknown -> {unknown} -> ru: {unknown_ru}\n               -> en: {unknown_en}");
    assert!(unknown_ru.contains("не распознано"));
    assert!(unknown_en.contains("not recognised"));

    // The classification itself, at the two edges that matter.
    assert!(settings::is_text_key(0x41), "A is a text key");
    assert!(settings::is_text_key(0x20), "space is a text key");
    assert!(!settings::is_text_key(0x13), "Pause is not");
    assert!(!settings::is_text_key(0x78), "F9 is not");
}

// -----------------------------------------------------------------------------------------
// Criterion 13 — a changed setting reaches the running modules without a restart
// -----------------------------------------------------------------------------------------

/// Serialises the tests that publish a configuration into the process-wide atomics.
///
/// `app::publish_configuration` writes into atomics of `hook`, `inject`, `layouts`, `selection`
/// and `guard` — one program, one configuration, as it has to be — and `cargo test` runs the tests
/// of one binary on parallel threads. Every test that publishes takes this first, so that one
/// test's publication cannot be read by another test's assertions. The same shape, and for the
/// same reason, as [`LOCALE`] further down this file.
///
/// Added by task T-13-13, which is what made the gate necessary: until then the tests that publish
/// asserted about fields no other publishing test touched, and they now share three.
///
/// ⚠ **Scope: the publication atomics and nothing else.** The tests task T-13-17 added at the end
/// of this file work on the About-window record — `settings::about_is_open`, `AboutSession`,
/// `settings::on_system_theme_message` — and touch no published field, so they neither take this
/// gate nor need it. Checked call by call when the two tasks were brought together on slice
/// `a17beaf`; a future test that publishes must take it.
static PUBLICATION: Mutex<()> = Mutex::new(());

/// Takes that gate.
fn publishing() -> MutexGuard<'static, ()> {
    PUBLICATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// How many entries of the journal carry `name` right now — task T-13-13.
///
/// By name and not by ordinal, because that is all a reader of the ring is given: the entry is a
/// row of the closed vocabulary of module `diag` and carries no value of its own (SEC-01, SEC-07),
/// so counting the rows is the whole of what can be asked.
fn journal_entries_named(name: &str) -> usize {
    lang_switcher::diag::snapshot()
        .iter()
        .filter(|event| event.operation.name() == name)
        .count()
}

/// The name task T-13-13 added to the closed vocabulary of module `diag`.
const CLAMPED: &str = "configuration field clamped to its ceiling";

#[test]
fn what_the_dialog_applies_reaches_the_modules_that_act_on_it() {
    let _publishing = publishing();

    // This is the second half of «Применить»: `tray::apply_settings` writes the file and then
    // calls exactly this, which is the only route by which a setting becomes behaviour.
    // Nothing here is a restart, and nothing here reads a file.
    let config = a_thoroughly_customised_config();

    app::publish_configuration(&config);

    // `[replacement]` — FR-42 and FR-44, module `inject`.
    println!(
        "inject: method={:?} delay={}",
        inject::replacement_method(),
        inject::inter_event_delay_ms()
    );
    assert_eq!(inject::replacement_method(), ReplacementMethod::Selection);
    assert_eq!(inject::inter_event_delay_ms(), 7);

    // `[selection]` — FR-61 and FR-65, module `selection`.
    println!(
        "selection: enabled={} timeout={:?} restore={:?}",
        selection::path_enabled(),
        selection::published_timeout(),
        selection::published_restore_delay()
    );
    assert!(!selection::path_enabled());
    assert_eq!(
        selection::published_timeout(),
        std::time::Duration::from_millis(450)
    );
    assert_eq!(
        selection::published_restore_delay(),
        std::time::Duration::from_millis(120)
    );

    // `[layouts]` — FR-30 and FR-31, module `layouts`.
    println!("layouts: mode={:?}", layouts::published().mode());
    assert_eq!(layouts::published().mode(), LayoutMode::Cycle);

    // `[exclusions]` — FR-84, module `guard`.
    println!("guard: exclusions={}", guard::counters().exclusions);
    assert_eq!(guard::counters().exclusions, 2);
    assert!(guard::is_excluded_name("mstsc.exe"));

    // `[hotkey]` and `general.enabled` — FR-02 and FR-95, module `hook`.
    println!(
        "hook: vk={:#04x} active={}",
        hook::hotkey_vk(),
        hook::is_active()
    );
    assert_eq!(
        hook::hotkey_vk(),
        hook::vk_from_name("ScrollLock").expect("ScrollLock is a name this build knows")
    );
    assert!(!hook::is_active());

    // And the reverse direction, so that the test cannot pass by the values happening to be
    // there already: publishing the defaults puts every one of them back.
    app::publish_configuration(&Config::default());

    assert_eq!(inject::replacement_method(), ReplacementMethod::Auto);
    assert_eq!(inject::inter_event_delay_ms(), 0);
    assert!(selection::path_enabled());
    assert_eq!(layouts::published().mode(), LayoutMode::Pair);
    assert_eq!(guard::counters().exclusions, 0);
    assert!(hook::is_active());
}

// -----------------------------------------------------------------------------------------
// Task T-13-13 — the ceilings of the three millisecond fields of section 7
//
// The audit of 2026-08-24: «`inter_event_delay_ms` не ограничен сверху: значение из файла
// способно усыпить поток ввода — тот самый, где живут LL-хук и сторож». The repair is a clamp on
// the **publication** — `app::publish_configuration` — and the three tests below are its three
// halves: a file that asks for too much, a dialog value that asks for too much, and a value that
// asks for something reasonable and must go through untouched.
//
// ⚠ Not one of them goes near `%APPDATA%`. The file is built by `TestDir` under `%TEMP%` and
// handed to `settings::read_or_default` explicitly, exactly as every test above this line does it.
// -----------------------------------------------------------------------------------------

/// **Criterion 5 of task T-13-13, both halves.** A file with `u32::MAX` in all three millisecond
/// fields publishes the ceilings — and the file is byte-for-byte what it was.
///
/// The second half is the point of the design and is easy to lose: the number in the file stays
/// the author's. Silently correcting somebody's hand-edited configuration is the very fault task
/// T-13-6 was raised to repair — «неизвестные поля уничтожаются при первом сохранении» — and a
/// repair that fixed one by committing the other would be no repair. What is bounded is the
/// **published effect**, and nothing else.
#[test]
fn a_file_asking_for_u32_max_publishes_the_ceilings_and_is_left_byte_for_byte() {
    let _publishing = publishing();

    let dir = TestDir::new("ms_ceilings");
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\n\
             [replacement]\n\
             inter_event_delay_ms = {max}\n\n\
             [selection]\n\
             clipboard_timeout_ms = {max}\n\
             clipboard_restore_delay_ms = {max}\n",
            max = u32::MAX
        ),
    );
    let before = fs::read(&path).expect("the file just written must be readable");

    let (config, outcome) = settings::read_or_default(&path);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::Current),
        "the file is well-formed: an out-of-range value is not a parse error"
    );

    // Nothing is clamped on the way **in**: the schema of section 7 is untouched by this task,
    // and what the file says is what the configuration in memory says.
    assert_eq!(config.replacement.inter_event_delay_ms, u32::MAX);
    assert_eq!(config.selection.clipboard_timeout_ms, u32::MAX);
    assert_eq!(config.selection.clipboard_restore_delay_ms, u32::MAX);

    app::publish_configuration(&config);

    println!(
        "published: delay={} timeout={:?} restore={:?}",
        inject::inter_event_delay_ms(),
        selection::published_timeout(),
        selection::published_restore_delay()
    );

    // The ceilings, as numbers — the ones `reports\ТЗ-Э13-ремонт.md` fixes for this task.
    assert_eq!(inject::MAX_INTER_EVENT_DELAY_MS, 1_000);
    assert_eq!(selection::MAX_CLIPBOARD_TIMEOUT_MS, 5_000);
    assert_eq!(selection::MAX_CLIPBOARD_RESTORE_DELAY_MS, 5_000);

    // ...and as what the three modules now answer.
    assert_eq!(
        inject::inter_event_delay_ms(),
        inject::MAX_INTER_EVENT_DELAY_MS
    );
    assert_eq!(
        selection::published_timeout(),
        std::time::Duration::from_millis(u64::from(selection::MAX_CLIPBOARD_TIMEOUT_MS))
    );
    assert_eq!(
        selection::published_restore_delay(),
        std::time::Duration::from_millis(u64::from(selection::MAX_CLIPBOARD_RESTORE_DELAY_MS))
    );

    // The other half, by bytes and not by fields: nothing on the publication path opened this
    // file for writing, and a comparison of the parsed configuration would not have caught one
    // that had.
    let after = fs::read(&path).expect("the file must still be readable");
    assert_eq!(
        after, before,
        "the publication must not touch the configuration file"
    );
    assert_eq!(
        dir.entries(),
        vec![CONFIG_FILE_NAME.to_owned()],
        "and it must leave nothing beside it either"
    );

    // The defaults back, so that this test leaves the process as it found it.
    app::publish_configuration(&Config::default());
}

/// **Criterion 6 of task T-13-13.** A value above the ceiling is clamped, and the journal says so
/// **once per publication** — not once per session and not once per press.
///
/// Nine digits was what the dialog of FR-92 could produce while the three millisecond fields stood
/// in it. Since task Т-23-2 (решение 81) the fields are gone and the numbers are typed into
/// `config.toml` by hand, where nothing bounds them at all — so 999 999 999 ms, about eleven and a
/// half days, is **more** reachable than before, not less, and this ceiling is the only thing
/// between it and the product.
#[test]
fn a_value_above_the_ceiling_is_clamped_and_journalled_once_for_each_publication() {
    let _publishing = publishing();

    // SEC-07: the entry has a row of its own in the closed vocabulary of module `diag`. Without
    // it the fact would reach the ring as `UNLISTED` — a code with nothing beside it — which is
    // exactly the defect task T-08-2 was raised to repair.
    assert_ne!(
        lang_switcher::diag::Operation::from_name(CLAMPED),
        lang_switcher::diag::Operation::UNLISTED,
        "«{CLAMPED}» must be a row of the table, or the entry carries no name"
    );

    let mut config = Config::default();
    config.replacement.inter_event_delay_ms = 999_999_999;
    config.selection.clipboard_timeout_ms = 999_999_999;
    config.selection.clipboard_restore_delay_ms = 999_999_999;

    let before = journal_entries_named(CLAMPED);

    app::publish_configuration(&config);

    let after_one = journal_entries_named(CLAMPED);
    println!("entries «{CLAMPED}»: {before} -> {after_one}");

    assert_eq!(
        after_one,
        before + 1,
        "one entry for the publication, whichever of the three fields were above their ceilings"
    );

    assert_eq!(
        inject::inter_event_delay_ms(),
        inject::MAX_INTER_EVENT_DELAY_MS
    );
    assert_eq!(
        selection::published_timeout(),
        std::time::Duration::from_millis(u64::from(selection::MAX_CLIPBOARD_TIMEOUT_MS))
    );
    assert_eq!(
        selection::published_restore_delay(),
        std::time::Duration::from_millis(u64::from(selection::MAX_CLIPBOARD_RESTORE_DELAY_MS))
    );

    // **«Once per publication» and not «once per session».** The same configuration applied again
    // is another «Применить», and it says so again — a latch would have gone quiet here, and the
    // second application of a bad value would then be invisible in a dump.
    app::publish_configuration(&config);

    assert_eq!(
        journal_entries_named(CLAMPED),
        before + 2,
        "a second publication is a second entry"
    );

    // And a publication in which nothing is above its ceiling adds nothing at all.
    app::publish_configuration(&Config::default());

    assert_eq!(
        journal_entries_named(CLAMPED),
        before + 2,
        "the defaults of section 7 are inside every ceiling"
    );
}

/// **Criterion 7 of task T-13-13 — the half that is easiest to break.** A value below the ceiling
/// is published as it stands, the ceiling does not touch it, and no entry is made.
///
/// The edge is checked as well, in both directions: a value *equal* to the ceiling is not above
/// it, and zero — the documented default of `inter_event_delay_ms` in section 7 — is a request for
/// no pause and not a mistake to be corrected, which is where this clamp deliberately differs from
/// `buffer::effective_capacity`.
#[test]
fn a_value_below_the_ceiling_is_published_as_it_stands_and_raises_nothing() {
    let _publishing = publishing();

    let before = journal_entries_named(CLAMPED);

    let mut config = Config::default();
    config.replacement.inter_event_delay_ms = 7;
    config.selection.clipboard_timeout_ms = 450;
    config.selection.clipboard_restore_delay_ms = 120;

    app::publish_configuration(&config);

    println!(
        "below the ceilings: delay={} timeout={:?} restore={:?}",
        inject::inter_event_delay_ms(),
        selection::published_timeout(),
        selection::published_restore_delay()
    );

    assert_eq!(inject::inter_event_delay_ms(), 7);
    assert_eq!(
        selection::published_timeout(),
        std::time::Duration::from_millis(450)
    );
    assert_eq!(
        selection::published_restore_delay(),
        std::time::Duration::from_millis(120)
    );
    assert_eq!(
        journal_entries_named(CLAMPED),
        before,
        "nothing was above its ceiling, so nothing may be journalled"
    );

    // The edge: exactly the ceiling is not above the ceiling.
    config.replacement.inter_event_delay_ms = inject::MAX_INTER_EVENT_DELAY_MS;
    config.selection.clipboard_timeout_ms = selection::MAX_CLIPBOARD_TIMEOUT_MS;
    config.selection.clipboard_restore_delay_ms = selection::MAX_CLIPBOARD_RESTORE_DELAY_MS;

    app::publish_configuration(&config);

    assert_eq!(
        inject::inter_event_delay_ms(),
        inject::MAX_INTER_EVENT_DELAY_MS
    );
    assert_eq!(
        journal_entries_named(CLAMPED),
        before,
        "the ceiling itself passes through, and passing through is not clamping"
    );

    // And zero, which section 7 documents as the default of the delay and FR-41 describes as the
    // ordinary case: a floor would have turned it into something else.
    config.replacement.inter_event_delay_ms = 0;
    config.selection.clipboard_timeout_ms = 0;
    config.selection.clipboard_restore_delay_ms = 0;

    app::publish_configuration(&config);

    assert_eq!(inject::inter_event_delay_ms(), 0);
    assert_eq!(
        selection::published_timeout(),
        std::time::Duration::ZERO,
        "zero is a value of the field and not an absence of one"
    );
    assert_eq!(journal_entries_named(CLAMPED), before);

    app::publish_configuration(&Config::default());
}

// -----------------------------------------------------------------------------------------
// Criterion 15 — FR-93, the value under HKCU\...\Run
// -----------------------------------------------------------------------------------------

#[test]
fn the_autostart_value_appears_and_disappears_under_hkcu() {
    // ⚠ This test writes into the registry of whoever is running it — the one value FR-93
    // names, under the current user and nowhere else. If that value is already there it
    // belongs to a real installation of this program, and the test refuses to touch it rather
    // than restore an approximation of it afterwards.
    if let Some(existing) = settings::autostart_value() {
        println!("autostart already registered ({existing}); this test does not touch it");
        return;
    }

    /// Puts the absence back even if an assertion below fails.
    struct Restore;

    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = settings::set_autostart(false);
        }
    }

    let _restore = Restore;

    assert!(!settings::autostart_registered(), "nothing there to start");

    settings::set_autostart(true).expect("FR-93: the value must be writable under HKCU");

    let written = settings::autostart_value().expect("FR-93: the value must be there now");
    println!("HKCU\\...\\Run\\Lang Switcher = {written}");

    assert_eq!(
        Some(written),
        settings::autostart_command(),
        "the value is the running executable, quoted"
    );

    // Writing it twice is one value and not two — the property that lets «Применить» run this
    // on every apply.
    settings::set_autostart(true).expect("a second write must succeed");
    assert!(settings::autostart_registered());

    settings::set_autostart(false).expect("FR-93: the value must be removable");
    assert!(
        !settings::autostart_registered(),
        "FR-93: unticking the box removes the value"
    );

    // And removing what is not there is not a failure, which is what makes the operation safe
    // to repeat.
    settings::set_autostart(false).expect("removing an absent value must succeed");
}

// -----------------------------------------------------------------------------------------
// Harness: the product binary and its dialog template
// -----------------------------------------------------------------------------------------

/// The built `LangSwitcher.exe`, opened as a data file so its resources can be read.
///
/// Section 4.4 of STATE.md: `embed-resource` links `app.rc` into the binary targets of this
/// crate and not into the test executables, so a test that looked for the dialog template in
/// its own image would be told `0x80070714`, "the specified image file does not contain a
/// resource section". The template under test is the one that ships, so the test reads the
/// file that ships.
struct ProductImage {
    module: HMODULE,
    /// Whether dropping this value unmaps the image. The shared one is never unmapped: it is
    /// handed to `settings::set_resource_module`, which keeps no lifetime, and the tests of one
    /// binary run on parallel threads — one of them freeing the mapping another is reading from
    /// would be a use-after-free in the test harness.
    owned: bool,
}

/// The one mapping [`ProductImage::shared`] hands out, as a raw handle.
static SHARED_IMAGE: OnceLock<usize> = OnceLock::new();

impl ProductImage {
    /// The mapping shared by every test that needs the product's strings, loaded once.
    fn shared() -> Self {
        let raw = *SHARED_IMAGE.get_or_init(|| Self::load().0 as usize);

        Self {
            module: HMODULE(std::ptr::without_provenance_mut(raw)),
            owned: false,
        }
    }

    /// One string of one locale, decoded straight out of the resource.
    ///
    /// Written from the documented layout of a string table rather than by calling the
    /// module's own reader: a test that used `settings::text` to check what
    /// `settings::text` reads would pass whatever the resource said. Sixteen strings to a
    /// block, block `id / 16 + 1`, each string a `u16` length followed by that many UTF-16
    /// units, counted and not NUL-terminated.
    fn string(&self, language: settings::Language, id: u16) -> String {
        self.string_of_langid(language.langid(), id)
    }

    /// The same string, addressed by the raw `LANGID` instead of by the variant.
    ///
    /// The sweep of the twelve locales (Т-28-5) needs this form and not the one above: it
    /// walks a table of numbers written out in the test, so that a locale missing from
    /// `Language` altogether is caught by the sweep rather than by the compiler refusing to
    /// name it.
    fn string_of_langid(&self, langid: u16, id: u16) -> String {
        let bytes = self.resource_of_language(RT_STRING, id / 16 + 1, langid);

        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();

        let wanted = usize::from(id % 16);
        let mut at = 0usize;

        for slot in 0..16 {
            let length = usize::from(units[at]);
            at += 1;

            if slot == wanted {
                // Not `from_utf16_lossy`: a replacement character would hide exactly the damage
                // a missing `#pragma code_page(65001)` produces.
                return String::from_utf16(&units[at..at + length])
                    .expect("a string table entry must be valid UTF-16");
            }

            at += length;
        }

        panic!("string {id} is not in block {}", id / 16 + 1);
    }

    fn open() -> Self {
        Self {
            module: Self::load(),
            owned: true,
        }
    }

    fn load() -> HMODULE {
        // `cargo test` puts the test executables in `<target>\debug\deps` and the binary
        // target one level up.
        let exe = std::env::current_exe()
            .expect("the test executable must have a path")
            .parent()
            .and_then(Path::parent)
            .expect("the test executable lives in <target>\\debug\\deps")
            .join("LangSwitcher.exe");

        assert!(
            exe.is_file(),
            "the product binary must be built alongside the tests: {}",
            exe.display()
        );

        let path: Vec<u16> = exe
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: `path` is a NUL-terminated UTF-16 buffer owned by this frame and not moved
        // or dropped until the call returns. `LOAD_LIBRARY_AS_DATAFILE` maps the image for
        // resource reading only: no entry point runs, nothing is relocated and no dependency
        // is loaded. The handle is freed exactly once, in `Drop`.
        unsafe { LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
            .expect("the product binary must be openable as a data file")
    }

    /// A copy of one resource of the product binary, by type and numeric identifier.
    fn resource(&self, kind: PCWSTR, id: u16) -> Vec<u8> {
        let name = PCWSTR(std::ptr::without_provenance(usize::from(id)));

        // SAFETY: `self.module` is loaded for the lifetime of this value. Both "strings" are
        // integer identifiers in the `MAKEINTRESOURCE` form — a value below 65536 carried in
        // the pointer — so nothing is dereferenced as a string.
        let found = unsafe { FindResourceW(Some(self.module), name, kind) };
        assert!(!found.0.is_null(), "resource {id} must be present");

        self.bytes_of(found, id)
    }

    /// The same, for a resource that exists once per language — FR-94's two string tables.
    ///
    /// `FindResourceExW` and not `FindResourceW`: the plain form asks for the language of the
    /// *calling thread*, which would make the test pass or fail depending on the Windows
    /// display language of whoever runs it. The language wanted is named outright.
    fn resource_of_language(&self, kind: PCWSTR, id: u16, language: u16) -> Vec<u8> {
        let name = PCWSTR(std::ptr::without_provenance(usize::from(id)));

        // SAFETY: as in `resource`, with the language identifier passed by value.
        let found = unsafe { FindResourceExW(Some(self.module), kind, name, language) };
        assert!(
            !found.0.is_null(),
            "resource {id} must be present in language 0x{language:04X}"
        );

        self.bytes_of(found, id)
    }

    /// The bytes of a resource already found.
    fn bytes_of(&self, found: HRSRC, id: u16) -> Vec<u8> {
        // SAFETY: `self.module` and `found` are the pair just established.
        let size = unsafe { SizeofResource(Some(self.module), found) };

        // SAFETY: the same pair.
        let block = unsafe { LoadResource(Some(self.module), found) }.expect("resource loads");

        // SAFETY: `block` came from the `LoadResource` directly above.
        let start = unsafe { LockResource(block) };
        assert!(
            !start.is_null() && size > 0,
            "resource {id} must be readable"
        );

        // SAFETY: `start` points at `size` bytes of read-only resource data inside the mapping
        // of `self.module`, which is alive for the whole of this call. The bytes are copied out
        // before the borrow ends, so nothing of the module escapes.
        unsafe {
            std::slice::from_raw_parts(start.cast::<u8>(), usize::try_from(size).unwrap_or(0))
                .to_vec()
        }
    }
}

impl Drop for ProductImage {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }

        // SAFETY: `module` came from a successful `LoadLibraryExW` and is freed exactly once —
        // this type is neither `Copy` nor `Clone`, and every slice taken from it has already
        // been copied into a `Vec`.
        let _ = unsafe { FreeLibrary(self.module) };
    }
}

/// What a `DLGTEMPLATEEX` resource says, read by hand.
///
/// Read by hand and not by creating the dialog, because creating it would prove that *this*
/// process can build the window and would say nothing about the bytes rc.exe produced — which
/// is exactly what is under test. The layout is the one documented for `DLGTEMPLATEEX` and
/// `DLGITEMTEMPLATEEX`: a fixed header with three variable-length fields, then one item each
/// with two, every item starting on a four-byte boundary.
struct DialogTemplate {
    /// The caption of the window.
    caption: String,
    /// The window's own size in dialog units — `(cx, cy)` of the header. Read out by task
    /// T-11-19, whose whole substance is the height: the rhythm of the layout is the air
    /// left between the last block and the bottom edge, and that is arithmetic on this
    /// number and the rectangles below.
    size: (i32, i32),
    /// Every non-empty control caption, in template order.
    text: Vec<String>,
    /// The identifier of every control.
    controls: Vec<u32>,
    /// The style of every control, paired with its identifier, in template order — task
    /// T-11-5a reads the button types of the nine owner-drawn buttons out of this.
    styles: Vec<(u32, u32)>,
    /// The rectangle of every control in dialog units — `(id, x, y, cx, cy)`, in template
    /// order. Read out by task T-11-13, whose defect is one control covering the whole area
    /// of the others: the measurement of that is arithmetic on these four numbers.
    bounds: Vec<(u32, i32, i32, i32, i32)>,
    /// The window class of every control, paired with its identifier, in template order —
    /// the ordinal of a predefined class, or `None` for a registered one named by string
    /// (`SysListView32` is the only such control of this program).
    ///
    /// Read out by task T-11-18, which has to say «every static of the template» and cannot
    /// say it from the style alone: the low nibble is the control *type*, and it means one
    /// thing for a static and another for every other class — the cycle list's
    /// `LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS` is 0x0D, the very number
    /// `SS_OWNERDRAW` is.
    classes: Vec<(u32, Option<u16>)>,
    /// The `DS_SETFONT` declaration of the template, or `None` for a template without one.
    ///
    /// Read out by task T-11-20, which measures the dialog's own face against the face this
    /// module sets its own text in: the numbers have to be the window's own, and the window's
    /// own font is what these five fields say.
    font: Option<TemplateFont>,
    /// `dwExtendedStyle` of the header — task T-21-4, the user's finding П-1.
    ///
    /// The window is owned by the program's hidden UI window, which carries `WS_EX_TOOLWINDOW`
    /// by decision R-20 point 2, and an owned window inherits its owner's absence from the task
    /// bar. `WS_EX_APPWINDOW` in this field is what gives it a button of its own, and it is read
    /// here rather than off the live window because this is the field that decides it: the style
    /// is in place before the window is ever shown, which is the only moment the shell looks.
    ex_style: u32,
}

/// The `DS_SETFONT` declaration of a dialog template — what the dialog manager builds a
/// window's own face out of, exactly as rc.exe wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TemplateFont {
    /// Point size — `9` for both templates of this program.
    points: u16,
    /// `lfWeight`.
    weight: u16,
    /// `lfItalic`.
    italic: u8,
    /// `lfCharSet`.
    charset: u8,
    /// The type face — «Segoe UI».
    face: String,
}

impl DialogTemplate {
    fn parse(bytes: &[u8]) -> Self {
        let mut at = 0usize;

        let version = read_u16(bytes, &mut at);
        let signature = read_u16(bytes, &mut at);
        assert_eq!(
            (version, signature),
            (1, 0xFFFF),
            "app.rc must produce a DIALOGEX template"
        );

        let _help_id = read_u32(bytes, &mut at);
        // `dwExtendedStyle` of the header — read out rather than discarded since task T-21-4,
        // the user's finding П-1: whether the window gets a button of its own on the task bar
        // is decided by `WS_EX_APPWINDOW` in this very field, before the window exists.
        let ex_style = read_u32(bytes, &mut at);
        let style = read_u32(bytes, &mut at);
        let items = read_u16(bytes, &mut at);

        // x, y, cx, cy. The position is the dialog manager's own business — `DS_CENTER`
        // places the window — but the size is the layout's, and task T-11-19 measures it.
        let _x = read_u16(bytes, &mut at);
        let _y = read_u16(bytes, &mut at);
        let size = (
            i32::from(read_u16(bytes, &mut at) as i16),
            i32::from(read_u16(bytes, &mut at) as i16),
        );

        let _menu = read_name(bytes, &mut at);
        let _class = read_name(bytes, &mut at);
        let caption = read_string(bytes, &mut at);

        // `DS_SETFONT`, which the template declares: point size, weight, italic, charset and
        // the type face follow the caption.
        let font = (style & 0x0000_0040 != 0).then(|| {
            let points = read_u16(bytes, &mut at);
            let weight = read_u16(bytes, &mut at);
            let italic = bytes[at];
            let charset = bytes[at + 1];
            at += 2;
            let face = read_string(bytes, &mut at);

            TemplateFont {
                points,
                weight,
                italic,
                charset,
                face,
            }
        });

        let mut text = Vec::new();
        let mut controls = Vec::new();
        let mut styles = Vec::new();
        let mut bounds = Vec::new();
        let mut classes = Vec::new();

        for _ in 0..items {
            at = (at + 3) & !3;

            let _help_id = read_u32(bytes, &mut at);
            let _ex_style = read_u32(bytes, &mut at);
            let style = read_u32(bytes, &mut at);

            // x, y, cx, cy — dialog units, `i16` in the template and widened here so that
            // the containment arithmetic of task T-11-13 has room to add.
            let x = i32::from(read_u16(bytes, &mut at) as i16);
            let y = i32::from(read_u16(bytes, &mut at) as i16);
            let cx = i32::from(read_u16(bytes, &mut at) as i16);
            let cy = i32::from(read_u16(bytes, &mut at) as i16);

            let id = read_u32(bytes, &mut at);

            let class = read_class(bytes, &mut at);
            let title = read_name(bytes, &mut at);

            let extra = usize::from(read_u16(bytes, &mut at));
            at += extra;

            controls.push(id);
            styles.push((id, style));
            bounds.push((id, x, y, cx, cy));
            classes.push((id, class));

            if let Some(title) = title
                && !title.is_empty()
            {
                text.push(title);
            }
        }

        Self {
            caption,
            size,
            text,
            controls,
            styles,
            bounds,
            classes,
            font,
            ex_style,
        }
    }

    /// The style of one control, by identifier.
    fn style_of(&self, id: u32, what: &str) -> u32 {
        self.styles
            .iter()
            .find(|(control, _)| *control == id)
            .map(|(_, style)| *style)
            .unwrap_or_else(|| panic!("the dialog has no control {id} — «{what}»"))
    }

    /// The rectangle of one control, by identifier, as `(left, top, right, bottom)` in
    /// dialog units.
    fn rect_of(&self, id: u32) -> (i32, i32, i32, i32) {
        self.bounds
            .iter()
            .find(|(control, ..)| *control == id)
            .map(|(_, x, y, cx, cy)| (*x, *y, *x + *cx, *y + *cy))
            .unwrap_or_else(|| panic!("the dialog has no control {id}"))
    }
}

/// One little-endian `u16` of the template.
fn read_u16(bytes: &[u8], at: &mut usize) -> u16 {
    let value = u16::from_le_bytes([bytes[*at], bytes[*at + 1]]);
    *at += 2;
    value
}

/// One little-endian `u32` of the template.
fn read_u32(bytes: &[u8], at: &mut usize) -> u32 {
    let value = u32::from_le_bytes([bytes[*at], bytes[*at + 1], bytes[*at + 2], bytes[*at + 3]]);
    *at += 4;
    value
}

/// A NUL-terminated UTF-16 string of the template.
fn read_string(bytes: &[u8], at: &mut usize) -> String {
    let mut units = Vec::new();

    loop {
        let unit = read_u16(bytes, at);

        if unit == 0 {
            break;
        }

        units.push(unit);
    }

    // Not `from_utf16_lossy`: a replacement character would hide exactly the damage this file
    // is here to detect.
    String::from_utf16(&units).expect("the template must hold valid UTF-16")
}

/// The class field of one control of the template: `Some` ordinal for one of the six
/// predefined classes, `None` for a class named by string.
///
/// The same `sz_Or_Ord` shape [`read_name`] reads, but keeping the number instead of throwing
/// it away — task T-11-18 asks «which of these controls is a static», and only the class
/// answers that. The values are the ones `winuser.h` fixes and the dialog manager acts on:
/// 0x0080 `Button`, 0x0081 `Edit`, 0x0082 `Static`, 0x0083 `ListBox`, 0x0084 `ScrollBar`,
/// 0x0085 `ComboBox`.
fn read_class(bytes: &[u8], at: &mut usize) -> Option<u16> {
    if u16::from_le_bytes([bytes[*at], bytes[*at + 1]]) == 0xFFFF {
        *at += 2;
        return Some(read_u16(bytes, at));
    }

    // A registered class named by string — `SysListView32`, the one this program creates —
    // or the empty name. Consumed exactly as `read_name` consumes it, and not reported: this
    // test has nothing to say about a class it did not predefine.
    let _ = read_name(bytes, at);

    None
}

/// The `sz_Or_Ord` field of the template: an empty string, an ordinal, or a string.
fn read_name(bytes: &[u8], at: &mut usize) -> Option<String> {
    match u16::from_le_bytes([bytes[*at], bytes[*at + 1]]) {
        0x0000 => {
            *at += 2;
            Some(String::new())
        }
        // An ordinal — a predefined window class such as `Button`. Not a string, and not
        // something this test has anything to say about.
        0xFFFF => {
            *at += 4;
            None
        }
        _ => Some(read_string(bytes, at)),
    }
}

// -----------------------------------------------------------------------------------------
// FR-94 — the two string tables, read out of the built binary
// -----------------------------------------------------------------------------------------
//
// ⚠ Same rule as the dialog template above, and it is the whole point of this section: every
// string is written out here as a literal and compared against what the **built**
// `LangSwitcher.exe` carries. Importing the strings from the crate and asserting the resource
// contains them would pass whatever the resource said, and the failure being guarded against is
// precisely a resource whose Cyrillic came out wrong — fact 6 of section 9 of STATE.md, which
// costs nothing at build time and leaves the tests green.
//
// The English half is here for the same reason and for one more: FR-94 asks for two languages,
// and a table nobody checks is a table that can be half empty.

/// Every interface string of FR-94: identifier, Russian, English.
///
/// The identifiers come from the crate — they are the contract between `app.rc` and
/// `src\settings.rs`, and checking that contract is the point. The text does not.
///
/// ⚠ **Seventy-three since task Т-31-4** — решение 99.4 authorised the canon of «seventy-two»
/// away: `IDS_LANGUAGE_RESTART` left (71) and the two words of the tray tooltip arrived (73).
const FR_94_STRINGS: [(u16, &str, &str); 203] = [
    (
        settings::IDS_DIALOG_CAPTION,
        "Lang Switcher — настройки",
        "Lang Switcher — Settings",
    ),
    (settings::IDS_GROUP_GENERAL, "Общие", "General"),
    (
        settings::IDS_AUTOSTART,
        "Запускать при входе в систему",
        "Start when I sign in",
    ),
    (
        settings::IDS_LANGUAGE_LABEL,
        "Язык интерфейса:",
        "Interface language:",
    ),
    // 3004 — «вступит в силу после перезапуска» / «takes effect after a restart» — снята
    // задачей Т-31-3 из всех четырнадцати таблиц: решение 99.4, язык действует сразу.
    // Номер оставлен дырой, как 3017..3021, 3023, 3024 и 3056.
    (settings::IDS_GROUP_HOTKEY, "Горячая клавиша", "Hotkey"),
    (settings::IDS_HOTKEY_LABEL, "Клавиша:", "Key:"),
    (settings::IDS_HOTKEY_SET, "Задать", "Set"),
    // ⚠ «Отмена» and not «Отменить» since task Т-23-5: the mock-up of решение 82.6 draws that
    // word on the button, and the user accepted it as drawn.
    (settings::IDS_HOTKEY_STOP, "Отмена", "Cancel"),
    (settings::IDS_GROUP_LAYOUTS, "Раскладки", "Layouts"),
    (settings::IDS_MODE_PAIR, "Пара", "Pair"),
    (
        settings::IDS_MODE_CYCLE,
        "Несколько раскладок",
        "Several layouts",
    ),
    (settings::IDS_PAIR_SOURCE, "Источник:", "Source:"),
    (settings::IDS_PAIR_TARGET, "Цель:", "Target:"),
    (
        settings::IDS_CYCLE_HINT,
        "Цикл: галочка — участие, кнопки — порядок",
        "Cycle: the tick takes part, the buttons set the order",
    ),
    (settings::IDS_CYCLE_UP, "Выше", "Up"),
    (settings::IDS_CYCLE_DOWN, "Ниже", "Down"),
    (
        settings::IDS_SELECTION_ENABLED,
        "Конвертировать выделенный текст",
        "Convert the selected text",
    ),
    (settings::IDS_GROUP_EXCLUSIONS, "Исключения", "Exclusions"),
    (settings::IDS_EXCLUSION_REMOVE, "Удалить", "Remove"),
    (settings::IDS_EXCLUSION_ADD, "Добавить", "Add"),
    (
        settings::IDS_EXCLUSION_HINT,
        "Имя процесса, например game.exe",
        "Process name, for example game.exe",
    ),
    (
        settings::IDS_GROUP_DIAGNOSTICS,
        "Диагностика",
        "Diagnostics",
    ),
    (settings::IDS_LOG_ENABLED, "Вести журнал", "Keep a journal"),
    (
        settings::IDS_LOG_OPEN,
        "Открыть папку журнала",
        "Open the journal folder",
    ),
    (
        settings::IDS_LOG_DIR_LABEL,
        "Папка журнала:",
        "Journal folder:",
    ),
    (settings::IDS_GROUP_STATE, "Состояние", "State"),
    (settings::IDS_OK, "ОК", "OK"),
    (settings::IDS_CANCEL, "Отмена", "Cancel"),
    (settings::IDS_APPLY, "Применить", "Apply"),
    (
        settings::IDS_NOTE_TEXT_KEY,
        "Это текстовая клавиша: пока программа активна, она перестанет вводить символ.",
        "This is a text key: while the program is active it will stop typing its character.",
    ),
    (
        settings::IDS_NOTE_UNKNOWN_KEY,
        "Имя клавиши не распознано — действует клавиша по умолчанию, Pause.",
        "The key name is not recognised — the default key, Pause, is in force.",
    ),
    (
        // ⚠ Task Т-23-5: the invitation moved into the **field** and lost its second half.
        // «Esc — отмена» became a line of its own under the field — `IDS_CAPTURE_HINT`.
        settings::IDS_CAPTURE_PROMPT,
        "Нажмите клавишу…",
        "Press a key…",
    ),
    (
        settings::IDS_CAPTURE_MODIFIER,
        "Модификатор сам по себе горячей клавишей быть не может.",
        "A modifier on its own cannot be a hotkey.",
    ),
    (
        settings::IDS_CAPTURE_COMBINATION,
        "Сочетания с модификаторами не поддерживаются: в файле настроек хранится одна клавиша.",
        "Combinations with modifiers are not supported: the settings file holds a single key.",
    ),
    (
        settings::IDS_CAPTURE_EMERGENCY,
        "Ctrl+Alt+Shift+F12 — аварийный выход, эта комбинация не назначается.",
        "Ctrl+Alt+Shift+F12 is the emergency exit; it cannot be assigned.",
    ),
    (
        settings::IDS_CAPTURE_NAMELESS,
        "У этой клавиши нет имени в файле настроек — выберите другую.",
        "This key has no name in the settings file — choose another one.",
    ),
    (
        settings::IDS_CAPTURE_RESERVED,
        "Эту клавишу занимает система — выберите другую.",
        "The system owns this key — choose another one.",
    ),
    (
        settings::IDS_STATE_HOOK,
        "Перехват клавиатуры: {0} · восстановлений хука: {1} · отказов установки: {2}",
        "Keyboard hook: {0} · hook recoveries: {1} · install failures: {2}",
    ),
    (settings::IDS_HOOK_UP, "установлен", "installed"),
    (settings::IDS_HOOK_DOWN, "НЕ УСТАНОВЛЕН", "NOT INSTALLED"),
    (
        settings::IDS_STATE_LAYOUTS,
        "Раскладок в сеансе: {0} · исключений опубликовано: {1} · записей в журнале: {2}",
        "Layouts in the session: {0} · exclusions published: {1} · journal entries: {2}",
    ),
    (
        settings::IDS_STATE_AUTOSTART,
        "Автозапуск в реестре (HKCU\\…\\Run): {0}",
        "Autostart in the registry (HKCU\\…\\Run): {0}",
    ),
    (settings::IDS_AUTOSTART_PRESENT, "есть, {0}", "yes, {0}"),
    (settings::IDS_AUTOSTART_ABSENT, "нет", "no"),
    (
        settings::IDS_LAYOUT_NOTE,
        "В сеансе нет раскладки, названной в файле ({0}) — выберите заново.",
        "The session has no layout named in the file ({0}) — choose again.",
    ),
    (settings::IDS_LAYOUT_WORD_SOURCE, "источник", "source"),
    (settings::IDS_LAYOUT_WORD_TARGET, "цель", "target"),
    (
        settings::IDS_LOG_DIR_MISSING,
        "%APPDATA% не задан — журнал писать некуда",
        "%APPDATA% is not set — there is nowhere to write the journal",
    ),
    (settings::IDS_THEME_LABEL, "Оформление:", "Appearance:"),
    (settings::IDS_THEME_SYSTEM, "Как в системе", "Match system"),
    (settings::IDS_THEME_LIGHT, "Светлое", "Light"),
    (settings::IDS_THEME_DARK, "Тёмное", "Dark"),
    // Task T-12-4, решение В-2: the caption of the window is «О программе» and nothing more —
    // the product's name is already on the row under the logo — and the version line opens with
    // a lower-case letter, as the mock-up writes it (`chrome.ps1:195` — «версия 0.1.0»). Both
    // locales, which is what FR-94 means by a table per locale.
    (settings::IDS_ABOUT_CAPTION, "О программе", "About"),
    (settings::IDS_ABOUT_VERSION, "версия {0}", "version {0}"),
    (
        settings::IDS_ABOUT_LINE_1,
        "Исправляет текст, набранный в неверной раскладке.",
        "Fixes text typed in the wrong keyboard layout.",
    ),
    (
        settings::IDS_ABOUT_LINE_2,
        "Перекодировка — по нажатию одной клавиши.",
        "Conversion takes a single key press.",
    ),
    (settings::IDS_ABOUT_OK, "ОК", "OK"),
    // The «Как пользоваться» panel of task Т-23-4, решение 82.5 — the caption and the five
    // rows the accepted mock-up writes. The `{0}` of the first three is the hotkey name and
    // is deliberately part of the literal: what a wrong code page would destroy is the text
    // around it, and the placeholder is what the substitution of `format_text` looks for.
    (
        settings::IDS_ABOUT_HELP,
        "Как пользоваться",
        "How to use it",
    ),
    (
        settings::IDS_ABOUT_HELP_1,
        "Набрали слово не в той раскладке — нажмите {0}: слово перекодируется, раскладка переключится.",
        "You typed a word in the wrong layout — press {0}: the word is converted and the layout switches.",
    ),
    (
        settings::IDS_ABOUT_HELP_2,
        "Повторное нажатие {0} возвращает исходный текст.",
        "Pressing {0} again brings the original text back.",
    ),
    (
        settings::IDS_ABOUT_HELP_3,
        "Выделите текст и нажмите {0} — конвертируется выделенное.",
        "Select text and press {0} — the selection is converted.",
    ),
    (
        settings::IDS_ABOUT_HELP_4,
        "Пауза, настройки и выход — в значке в трее.",
        "Suspend, settings and exit are in the tray icon.",
    ),
    (
        settings::IDS_ABOUT_HELP_5,
        "Сменить клавишу: Настройки → Горячая клавиша.",
        "To change the key: Settings → Hotkey.",
    ),
    (settings::IDS_MENU_SUSPEND, "Приостановить", "Suspend"),
    (settings::IDS_MENU_RESUME, "Возобновить", "Resume"),
    (settings::IDS_MENU_SETTINGS, "Настройки…", "Settings…"),
    (
        settings::IDS_MENU_AUTOSTART,
        "Запускать при входе в систему",
        "Start when I sign in",
    ),
    (settings::IDS_MENU_ABOUT, "О программе", "About"),
    (settings::IDS_MENU_EXIT, "Выход", "Exit"),
    // FR-100, task Т-21-5. Last in the list because it is last in `INTERFACE_STRINGS`, and the
    // two orders are asserted equal by
    // `the_two_tables_hold_exactly_the_identifiers_the_crate_publishes`.
    (settings::IDS_SOUND, "Звуковой отклик", "Sound feedback"),
    // FR-94, task Т-23-5, решение 82.6 — the way out of a capture, said under the field while
    // the field itself holds the invitation.
    (
        settings::IDS_CAPTURE_HINT,
        "Esc или клик мимо — отмена",
        "Esc or a click elsewhere cancels",
    ),
    // FR-90, task Т-31-4, решение 99.2 — the state of the program in the tooltip of the tray
    // icon. Two words that were Russian literals in `src\tray.rs` in all fourteen locales until
    // this task, and the gender is the program's: «программа активна», «the program is
    // suspended» — not the hook's, which is what IDS_HOOK_UP above speaks of.
    (settings::IDS_TIP_ACTIVE, "активна", "active"),
    (settings::IDS_TIP_PAUSED, "приостановлена", "suspended"),
    // FR-101, FR-102 and FR-103, task Т-32-2, вопрос 101 — the letters from the author. The
    // heart is «♥» (U+2665) and not the mock-up's «🖤»: measured, `scratchpad-Э32\глиф-сердце.log`
    // — U+2665 is in all three faces this program draws in, and U+1F5A4 is outside the BMP,
    // where the glyph-coverage instrument cannot answer at all.
    (
        settings::IDS_LETTER_CAPTION,
        "Письмо от автора",
        "A letter from the author",
    ),
    (settings::IDS_CLOSE, "Закрыть", "Close"),
    (
        settings::IDS_CHANNEL_OPEN,
        "Открыть канал",
        "Open the channel",
    ),
    (
        settings::IDS_SUPPORT_OPEN,
        "Открыть страницу поддержки",
        "Open the support page",
    ),
    (
        settings::IDS_HELLO_TITLE,
        "Здравствуйте! Меня зовут Панда.",
        "Hello! My name is Panda.",
    ),
    (
        settings::IDS_HELLO_LEAD,
        "Я сделал Lang Switcher и рад, что вы его поставили. Программа делает одну вещь: чинит \
         слово, набранное не в той раскладке. Вот как это выглядит:",
        "I made Lang Switcher and I am glad you installed it. The program does one thing: it \
         fixes a word typed in the wrong layout. Here is what that looks like:",
    ),
    (
        settings::IDS_HELLO_DEMO_CAP,
        "Набрали «ghbdtn», нажали {0}: получили «привет», раскладка переключилась на русскую. \
         Ещё раз — и как было.",
        "You typed «ghbdtn», pressed {0} and got «привет», with the layout switched to Russian. \
         Press it again and it is back as it was.",
    ),
    (
        settings::IDS_HELLO_PANEL,
        "Три вещи на первый день",
        "Three things for the first day",
    ),
    (
        settings::IDS_HELLO_ROW_1,
        "{0} перекодирует последнее набранное слово и переключает раскладку. Работает в любом \
         окне.",
        "{0} converts the last word you typed and switches the layout. It works in any window.",
    ),
    (
        settings::IDS_HELLO_ROW_2,
        "Ещё раз {0}: слово вернётся к исходному, буква в букву.",
        "Press {0} again and the word comes back exactly as it was, letter for letter.",
    ),
    (
        settings::IDS_HELLO_ROW_3,
        "Значок в трее: пауза, настройки и «О программе», где есть раздел «Как пользоваться».",
        "The tray icon: pause, settings and «About», which has a «How to use it» section.",
    ),
    (
        settings::IDS_HELLO_SETTINGS,
        "Открыть настройки",
        "Open settings",
    ),
    (settings::IDS_HELLO_OK, "Понятно", "Got it"),
    (
        settings::IDS_THANKS_TITLE,
        "Прошёл месяц. Спасибо!",
        "A month has passed. Thank you!",
    ),
    (
        settings::IDS_THANKS_LEAD,
        "Здравствуйте, это снова Панда. Месяц назад вы поставили Lang Switcher, и я надеюсь, \
         что вам с ним удобно: я потратил много сил, чтобы программа была лёгкой и просто \
         работала.",
        "Hello, it is Panda again. You installed Lang Switcher a month ago, and I hope it suits \
         you: I put a great deal of work into keeping it light and simply working.",
    ),
    (
        settings::IDS_THANKS_PARA,
        "Если понравилось, расскажите об этом в канале. Если что-то не так, напишите мне: \
         негатив тоже важен, только узнав о проблеме, её можно починить. Мастер обращения есть \
         в меню значка, пункт «Написать автору…».",
        "If you liked it, say so in the channel. If something is wrong, write to me: criticism \
         matters too — a problem can only be fixed once somebody knows about it. The wizard is \
         in the tray menu, under «Write to the author…».",
    ),
    (
        settings::IDS_SUPPORT_PANEL,
        "Поддержать автора",
        "Support the author",
    ),
    (
        settings::IDS_SUPPORT_TEXT,
        "Программа бесплатная и останется такой. Если хотите сказать спасибо делом, на странице \
         поддержки есть несколько способов. Классно, что вы цените чужой труд ♥",
        "The program is free and will stay that way. If you would like to say thank you in \
         deed, the support page lists a few ways. It is good that you value somebody else's \
         work ♥",
    ),
    (
        settings::IDS_THANKS_SNOOZE,
        "Напомнить через неделю",
        "Remind me in a week",
    ),
    (
        settings::IDS_THANKS_FOOT,
        "Это письмо показывается один раз.",
        "This letter is shown once.",
    ),
    (
        settings::IDS_WHATSNEW_TITLE,
        "Что нового в {0}",
        "What is new in {0}",
    ),
    (
        settings::IDS_WHATSNEW_FROM,
        "было {0} · обновлено сегодня",
        "you had {0} · updated today",
    ),
    (
        settings::IDS_WHATSNEW_TODAY,
        "обновлено сегодня",
        "updated today",
    ),
    (
        settings::IDS_WHATSNEW_PANEL,
        "Три изменения",
        "Three changes",
    ),
    (
        settings::IDS_WHATSNEW_1,
        "Мастер «Написать автору»: пять шагов для ошибки и три для пожелания.",
        "The «Write to the author» wizard: five steps for a bug and three for an idea.",
    ),
    (
        settings::IDS_WHATSNEW_2,
        "Он сам соберёт версию, сборку Windows, раскладки и журнал — каждый пункт можно снять.",
        "It collects the version, the Windows build, the layouts and the journal — every item \
         can be taken off.",
    ),
    (
        settings::IDS_WHATSNEW_3,
        "Ничего не отправляется само: текст попадёт в буфер обмена, а вставите его вы.",
        "Nothing is sent by itself: the text goes to the clipboard and you paste it.",
    ),
    (
        settings::IDS_WHATSNEW_FULL,
        "Полный список изменений опубликован в канале.",
        "The full list of changes is published in the channel.",
    ),
    (settings::IDS_AUTHOR_CAPTION, "От автора", "From the author"),
    (
        settings::IDS_AUTHOR_VERSION,
        "версия {0} · Панда, он же Panda_Pishet_Kod",
        "version {0} · Panda, also known as Panda_Pishet_Kod",
    ),
    (settings::IDS_AUTHOR_PANEL, "Автор", "The author"),
    (
        settings::IDS_AUTHOR_TEXT,
        "Программу делает один человек в свободное время. Программа бесплатная и останется \
         такой. Если хотите сказать спасибо делом, на странице поддержки есть несколько \
         способов. Классно, что вы цените чужой труд ♥",
        "The program is made by one person in their spare time. It is free and will stay that \
         way. If you would like to say thank you in deed, the support page lists a few ways. It \
         is good that you value somebody else's work ♥",
    ),
    (
        settings::IDS_NEWS_PANEL,
        "Новости и обновления",
        "News and updates",
    ),
    (
        settings::IDS_NEWS_ABOUT_FEED,
        "Раз в 15 дней программа читает один файл с сайта автора: письма и номер свежей версии. \
         Больше она ничего не отправляет и не скачивает.",
        "Once in fifteen days the program reads one file from the author's site: letters and \
         the number of the latest version. It sends nothing else and downloads nothing.",
    ),
    (
        settings::IDS_NEWS_NEVER_READ,
        "Лента ещё не читалась.",
        "The feed has not been read yet.",
    ),
    (
        settings::IDS_NEWS_READ_ON,
        "Лента прочитана {0}, подпись автора верна. Следующее чтение через {1} дней.",
        "Feed read on {0}, the author's signature checks out. Next reading in {1} days.",
    ),
    (
        settings::IDS_NEWS_LATEST,
        "Установлена версия {0}, это последняя.",
        "Version {0} is installed, and it is the latest.",
    ),
    (
        settings::IDS_NEWS_AVAILABLE,
        "Установлена версия {0}, доступна {1}.",
        "Version {0} is installed, {1} is available.",
    ),
    (
        settings::IDS_NEWS_DOWNLOAD,
        "Открыть страницу загрузки",
        "Open the download page",
    ),
    (
        settings::IDS_NEWS_LETTERS,
        "Последние письма",
        "Recent letters",
    ),
    (
        settings::IDS_NEWS_FILE_ONLY,
        "Отключается в файле настроек: [letters] feed = false.",
        "Turned off in the settings file: [letters] feed = false.",
    ),
    (
        settings::IDS_NEWS_SWITCH,
        "Сообщать об обновлениях и новостях автора",
        "Tell me about the author's updates and news",
    ),
    (
        settings::IDS_NEWS_SWITCH_SUB,
        "Раз в 15 дней программа читает один файл с сайта автора. Больше она ничего не \
         отправляет и не скачивает.",
        "Once in fifteen days the program reads one file from the author's site. It sends \
         nothing else and downloads nothing.",
    ),
    (settings::IDS_FEEDBACK_PANEL, "Обратная связь", "Feedback"),
    (
        settings::IDS_FEEDBACK_TEXT,
        "Нашли ошибку или есть идея? Мастер соберёт обращение за пять шагов. Ничего не \
         отправится само: текст попадёт в буфер обмена, а вставите его вы.",
        "Found a bug or have an idea? The wizard puts a message together in five steps. Nothing \
         is sent by itself: the text goes to the clipboard, and you paste it.",
    ),
    (
        settings::IDS_WRITE_TO_AUTHOR,
        "Написать автору…",
        "Write to the author…",
    ),
    (
        settings::IDS_LETTERS_CAPTION,
        "Последние письма",
        "Recent letters",
    ),
    (
        settings::IDS_LETTERS_FOOT,
        "Хранятся три последние новости. Когда приходит четвёртая, самая старая уходит, даже \
         непрочитанная.",
        "The three latest news items are kept. When a fourth arrives, the oldest goes, read or \
         not.",
    ),
    (
        settings::IDS_LETTERS_OPEN,
        "Открыть письмо",
        "Open the letter",
    ),
    (settings::IDS_NEWS_READ_BUTTON, "Прочитано", "Mark as read"),
    (
        settings::IDS_MENU_UNREAD,
        "Непрочитанное письмо…",
        "An unread letter…",
    ),
    (
        settings::IDS_MENU_UPDATE,
        "Доступна версия {0}…",
        "Version {0} is available…",
    ),
    (
        settings::IDS_TOAST_TITLE,
        "Письмо от автора",
        "A letter from the author",
    ),
    (
        settings::IDS_TOAST_NEWS,
        "У Панды новость для вас. Нажмите, чтобы прочитать, это займёт минуту.",
        "Panda has news for you. Click to read it, it will take a minute.",
    ),
    (
        settings::IDS_TOAST_THANKS,
        "Прошёл месяц с установки. Панда хочет сказать пару слов. Нажмите, чтобы прочитать.",
        "It has been a month since you installed it. Panda would like a word. Click to read it.",
    ),
    (
        settings::IDS_TOAST_UPDATE_TITLE,
        "Вышла версия {0}",
        "Version {0} is out",
    ),
    (
        settings::IDS_TOAST_UPDATE,
        "Три изменения и ссылка на загрузку. Нажмите, чтобы прочитать.",
        "Three changes and a link to the download. Click to read it.",
    ),
    (settings::IDS_ABOUT_AUTHOR, "От автора…", "From the author…"),
    (
        settings::IDS_ENTRY_UPDATE_MARK,
        "{0} · обновление · у вас {1}",
        "{0} · update · you have {1}",
    ),
    (
        settings::IDS_ENTRY_UNREAD_MARK,
        "{0} · не прочитано",
        "{0} · not read",
    ),
    (
        settings::IDS_ENTRY_READ_MARK,
        "{0} · прочитано",
        "{0} · read",
    ),
    (
        settings::IDS_UPDATE_SUB,
        "у вас {0} · выпуск {1}",
        "you have {0} · released {1}",
    ),
    (settings::IDS_UPDATE_HOW, "Как обновиться", "How to update"),
    (
        settings::IDS_UPDATE_STEP_1,
        "Нажмите «Открыть страницу загрузки»: там всегда последняя версия.",
        "Press «Open the download page»: the latest version is always there.",
    ),
    (
        settings::IDS_UPDATE_STEP_2,
        "Скачайте файл LangSwitcher-setup.exe и запустите его.",
        "Download the LangSwitcher-setup.exe file and run it.",
    ),
    (
        settings::IDS_UPDATE_STEP_3,
        "Установщик сам закроет программу, заменит её и предложит запустить заново. Ваши \
         настройки сохранятся.",
        "The installer closes the program itself, replaces it and offers to start it again. \
         Your settings are kept.",
    ),
    (
        settings::IDS_UPDATE_FOOT,
        "Строка «Доступна версия {0}» останется в меню значка, пока вы не обновитесь.",
        "The «Version {0} is available» entry stays in the tray menu until you update.",
    ),
    (
        settings::IDS_NEWS_LETTER_TITLE,
        "Новость от автора",
        "News from the author",
    ),
    (settings::IDS_NEWS_LATER, "Позже", "Later"),
    (
        settings::IDS_NEWS_OPEN_LINK,
        "Открыть ссылку",
        "Open the link",
    ),
    (
        settings::IDS_NEWS_FOOT,
        "«Прочитано» закрывает письмо насовсем. «Позже» или крестик: напомню через неделю и ещё \
         раз через две, потом только точка на значке.",
        "«Mark as read» closes the letter for good. «Later» or the cross: I will remind you in \
         a week and once more in two, after that only the dot on the icon.",
    ),
    (
        settings::IDS_WIZARD_CAPTION,
        "Написать автору",
        "Write to the author",
    ),
    (
        settings::IDS_WIZARD_STEP,
        "Шаг {0} из {1}",
        "Step {0} of {1}",
    ),
    (settings::IDS_WIZARD_CANCEL, "Отмена", "Cancel"),
    (settings::IDS_WIZARD_BACK, "Назад", "Back"),
    (settings::IDS_WIZARD_NEXT, "Далее", "Next"),
    (settings::IDS_WIZARD_DONE, "Готово", "Done"),
    (
        settings::IDS_WIZARD_WHAT_TITLE,
        "Что случилось?",
        "What happened?",
    ),
    (
        settings::IDS_WIZARD_WHAT_NOTE,
        "От ответа зависит, о чём мастер спросит дальше.",
        "The answer decides what the wizard asks about next.",
    ),
    (
        settings::IDS_WIZARD_CARD_WRONG,
        "Программа сделала не то",
        "The program did the wrong thing",
    ),
    (
        settings::IDS_WIZARD_CARD_WRONG_SUB,
        "слово перекодировалось неправильно, пропали буквы, раскладка не та",
        "the word came out converted wrongly, letters went missing, the layout was not the one",
    ),
    (
        settings::IDS_WIZARD_CARD_NOTHING,
        "Ничего не произошло",
        "Nothing happened",
    ),
    (
        settings::IDS_WIZARD_CARD_NOTHING_SUB,
        "нажали клавишу, а текст остался как был",
        "you pressed the key and the text stayed as it was",
    ),
    (
        settings::IDS_WIZARD_CARD_IDEA,
        "Хочу предложить улучшение",
        "I would like to suggest an improvement",
    ),
    (
        settings::IDS_WIZARD_CARD_IDEA_SUB,
        "идея, пожелание, неудобство",
        "an idea, a wish, an inconvenience",
    ),
    (
        settings::IDS_WIZARD_WHERE_TITLE,
        "Где это случилось?",
        "Where did it happen?",
    ),
    (
        settings::IDS_WIZARD_WHERE_NOTE,
        "Поведение зависит от окна: консоль, браузер и редактор получают текст по-разному. \
         Переключитесь в то окно, и мастер запишет имя программы сам.",
        "The behaviour depends on the window: a console, a browser and an editor take text in \
         different ways. Switch to that window and the wizard writes the program's name itself.",
    ),
    (settings::IDS_WIZARD_PROGRAM, "Программа:", "Program:"),
    (
        settings::IDS_WIZARD_CAPTURE,
        "Взять из активного окна",
        "Take from the active window",
    ),
    (
        settings::IDS_WIZARD_CAPTURE_NOTE,
        "Берётся имя процесса и класс окна. Заголовок окна не берётся: в нём бывает имя \
         документа.",
        "The process name and the window class are taken. The window title is not: it often \
         holds the name of a document.",
    ),
    (
        settings::IDS_WIZARD_CAPTURE_COUNT,
        "Переключитесь в нужное окно… {0}",
        "Switch to the window you need… {0}",
    ),
    (
        settings::IDS_WIZARD_WHERE_FIELD,
        "Поле ввода",
        "Input field",
    ),
    (
        settings::IDS_WIZARD_FIELD_NORMAL,
        "Обычное поле ввода",
        "An ordinary input field",
    ),
    (
        settings::IDS_WIZARD_FIELD_PASSWORD,
        "Поле пароля",
        "A password field",
    ),
    (
        settings::IDS_WIZARD_FIELD_PASSWORD_SUB,
        "в полях пароля программа не работает нарочно: нажатия там не запоминаются",
        "in password fields the program does not work on purpose: keystrokes there are not \
         remembered",
    ),
    (
        settings::IDS_WIZARD_FIELD_UNKNOWN,
        "Не знаю",
        "I do not know",
    ),
    (
        settings::IDS_WIZARD_DID_TITLE,
        "Что вы делали?",
        "What were you doing?",
    ),
    (
        settings::IDS_WIZARD_DID_NOTE,
        "Пишите пример вместо настоящего текста. Программа не хранит нажатий, поэтому в \
         обращение попадёт только то, что вы напишете здесь.",
        "Write an example instead of the real text. The program keeps no keystrokes, so the \
         appeal carries only what you write here.",
    ),
    (
        settings::IDS_WIZARD_TYPED_IN,
        "Набрали слово в раскладке",
        "Typed a word in the layout",
    ),
    (settings::IDS_WIZARD_AND_PRESSED, "и нажали", "and pressed"),
    (settings::IDS_WIZARD_EXPECTED, "Ожидали:", "Expected:"),
    (settings::IDS_WIZARD_GOT, "Получили:", "Got:"),
    (
        settings::IDS_WIZARD_REPEAT,
        "Повторяется?",
        "Does it repeat?",
    ),
    (
        settings::IDS_WIZARD_REPEAT_ALWAYS,
        "каждый раз",
        "every time",
    ),
    (settings::IDS_WIZARD_REPEAT_SOMETIMES, "иногда", "sometimes"),
    (settings::IDS_WIZARD_REPEAT_ONCE, "один раз", "once"),
    (
        settings::IDS_WIZARD_IDEA_TITLE,
        "Опишите идею",
        "Describe the idea",
    ),
    (
        settings::IDS_WIZARD_IDEA_NOTE,
        "Два вопроса. Чем конкретнее пример, тем проще понять, что именно сделать.",
        "Two questions. The more concrete the example, the easier it is to see what exactly to \
         do.",
    ),
    (
        settings::IDS_WIZARD_IDEA_WHAT,
        "Что предлагаете?",
        "What do you suggest?",
    ),
    (
        settings::IDS_WIZARD_IDEA_HELPS,
        "Чем это поможет?",
        "How would it help?",
    ),
    (
        settings::IDS_WIZARD_ATTACH_TITLE,
        "Что приложить?",
        "What to attach?",
    ),
    (
        settings::IDS_WIZARD_ATTACH_NOTE,
        "Каждый пункт можно снять. Ниже написано, что именно попадёт в текст.",
        "Every item can be taken off. What each of them adds is written beside it.",
    ),
    (
        settings::IDS_WIZARD_ATTACH_MACHINE,
        "Версия программы и сборка Windows",
        "The program's version and the Windows build",
    ),
    (
        settings::IDS_WIZARD_ATTACH_LAYOUTS,
        "Раскладки в системе",
        "The layouts in the system",
    ),
    (
        settings::IDS_WIZARD_ATTACH_SETTINGS,
        "Настройки",
        "The settings",
    ),
    (
        settings::IDS_WIZARD_ATTACH_JOURNAL,
        "Журнал программы из памяти",
        "The program's journal, out of memory",
    ),
    (
        settings::IDS_WIZARD_ATTACH_JOURNAL_SUB,
        "{0} записей. Только имена операций и коды ошибок, ни одной клавиши: так устроен сам \
         журнал",
        "{0} entries. Only operation names and error codes, not a single keystroke: that is how \
         the journal itself is built",
    ),
    (
        settings::IDS_WIZARD_ATTACH_FOOT,
        "Файл журнала не нужен: записи есть в памяти, пока программа запущена. Если проблема \
         повторяется, включите журнал в настройках, тогда записи переживут перезапуск.",
        "No journal file is needed: the entries are in memory while the program runs. If the \
         problem repeats, turn the journal on in the settings and the entries will survive a \
         restart.",
    ),
    (
        settings::IDS_WIZARD_PREVIEW_TITLE,
        "Проверьте и отправьте",
        "Check it and send",
    ),
    (
        settings::IDS_WIZARD_PREVIEW_NOTE,
        "Это весь текст обращения. Правьте что угодно. Отправляете вы сами: текст попадёт в \
         буфер обмена, а канал откроется в браузере.",
        "This is the whole text of the appeal. Edit anything you like. You send it yourself: \
         the text goes to the clipboard and the channel opens in the browser.",
    ),
    (
        settings::IDS_WIZARD_COPY,
        "Скопировать и открыть канал",
        "Copy and open the channel",
    ),
    (
        settings::IDS_WIZARD_SAVE,
        "Сохранить в папку журнала",
        "Save to the journal folder",
    ),
    (
        settings::IDS_WIZARD_COPIED_ONLY,
        "Скопировано. Адрес канала появится в следующей версии.",
        "Copied. The address of the channel will appear in the next version.",
    ),
    (
        settings::IDS_WIZARD_COPIED,
        "Скопировано, канал открыт.",
        "Copied, the channel is open.",
    ),
    (settings::IDS_WIZARD_SAVED, "Сохранено: {0}", "Saved: {0}"),
    (
        settings::IDS_WIZARD_FAILED,
        "Не получилось. Текст остался в окне — скопируйте его вручную.",
        "It did not work. The text is still in the window — copy it by hand.",
    ),
    (
        settings::IDS_THANKYOU_BUG_TITLE,
        "Спасибо за сообщение об ошибке",
        "Thank you for the bug report",
    ),
    (
        settings::IDS_THANKYOU_IDEA_TITLE,
        "Спасибо за идею",
        "Thank you for the idea",
    ),
    (
        settings::IDS_THANKYOU_BUG_TEXT,
        "Я читаю каждое обращение сам. Ошибки чиню, а о важном для всех рассказываю письмом в \
         программе или в канале. Здорово, что вы нашли на это время 🖤",
        "I read every appeal myself. Bugs I fix, and what matters to everybody I tell about in \
         a letter in the program or in the channel. It is good that you found the time 🖤",
    ),
    (
        settings::IDS_THANKYOU_IDEA_TEXT,
        "Я читаю каждое обращение сам. Идеи складываю в план и лучшие из них делаю, а о важном \
         для всех рассказываю письмом в программе или в канале. Здорово, что вы нашли на это \
         время 🖤",
        "I read every appeal myself. Ideas go into the plan and the best of them get made, and \
         what matters to everybody I tell about in a letter in the program or in the channel. \
         It is good that you found the time 🖤",
    ),
];

/// Serialises the tests that publish an interface locale.
///
/// The locale is process-wide, as it has to be — one program, one interface — and `cargo test`
/// runs the tests of one binary on parallel threads. Every test that moves it takes this first
/// and puts the default of section 7 back when it is done.
static LOCALE: Mutex<()> = Mutex::new(());

/// Takes that gate and points the string loader at the built binary.
///
/// ⚠ The redirection is what makes any of this testable at all: `embed-resource` links `app.rc`
/// into the **binary** targets of the crate, so this test executable carries no resource section
/// and every string would come back empty. Section 4.4 of STATE.md.
fn with_product_strings() -> MutexGuard<'static, ()> {
    let guard = LOCALE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    settings::set_resource_module(ProductImage::shared().module);

    guard
}

#[test]
fn both_string_tables_of_fr_94_are_in_the_built_binary() {
    let product = ProductImage::shared();

    for (id, russian, english) in FR_94_STRINGS {
        let in_binary_ru = product.string(settings::Language::Ru, id);
        let in_binary_en = product.string(settings::Language::En, id);

        println!("{id} ru: {in_binary_ru}");
        println!("{id} en: {in_binary_en}");

        assert_eq!(
            in_binary_ru, russian,
            "string {id} came out of rc.exe wrong in Russian — check #pragma code_page(65001)"
        );
        assert_eq!(
            in_binary_en, english,
            "string {id} came out of rc.exe wrong in English"
        );
    }
}

#[test]
fn the_two_tables_hold_exactly_the_identifiers_the_crate_publishes() {
    // The mirror between `app.rc` and `src\settings.rs` in one assertion: the crate says which
    // identifiers the interface has, this file says what they read, and neither may grow a row
    // the other does not know about.
    let published: Vec<u16> = settings::INTERFACE_STRINGS.to_vec();
    let checked: Vec<u16> = FR_94_STRINGS.iter().map(|(id, _, _)| *id).collect();

    assert_eq!(
        published, checked,
        "every interface string must be checked against both locales"
    );
}

#[test]
fn the_russian_table_says_what_the_dialog_template_says() {
    // Two independent witnesses to the same text, which is what makes either of them worth
    // anything: the template is what ships in the window and the table is what the program puts
    // into it, and a code page accident would have to corrupt both identically to pass here.
    let product = ProductImage::shared();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let russian: Vec<String> = FR_94_STRINGS
        .iter()
        .map(|(id, _, _)| product.string(settings::Language::Ru, *id))
        .collect();

    assert_eq!(
        template.caption,
        product.string(settings::Language::Ru, settings::IDS_DIALOG_CAPTION),
        "the caption of the template and of the Russian table have parted"
    );

    for text in &template.text {
        assert!(
            russian.contains(text),
            "the template shows «{text}», which is in no row of the Russian table"
        );
    }
}

#[test]
fn lang_switcher_is_spelled_the_same_in_both_locales() {
    // Decision on question 7: the display name is not translated. It is the name in the caption
    // of the window, the name of the value under `HKCU\…\Run` and the name in the about box.
    let product = ProductImage::shared();

    let ru = product.string(settings::Language::Ru, settings::IDS_DIALOG_CAPTION);
    let en = product.string(settings::Language::En, settings::IDS_DIALOG_CAPTION);

    println!("caption ru: {ru}");
    println!("caption en: {en}");

    assert!(ru.starts_with("Lang Switcher"), "{ru}");
    assert!(en.starts_with("Lang Switcher"), "{en}");
    assert_ne!(ru, en, "the rest of the caption is translated");
}

#[test]
fn the_interface_speaks_the_language_the_configuration_names() {
    // **Criterion 10.** The note beside the combo box says the choice takes effect after a
    // restart; this is that choice taking effect. `set_ui_language` is what `app` calls once, at
    // start-up, with `general.language` — and from that moment every string of the interface
    // comes out of the other table.
    let _guard = with_product_strings();

    settings::set_ui_language(settings::Language::Ru);
    let russian = settings::text(settings::IDS_GROUP_GENERAL);
    let russian_menu = settings::text(settings::IDS_MENU_EXIT);
    let russian_state = settings::format_text(settings::IDS_STATE_AUTOSTART, &["нет"]);

    settings::set_ui_language(settings::Language::En);
    let english = settings::text(settings::IDS_GROUP_GENERAL);
    let english_menu = settings::text(settings::IDS_MENU_EXIT);
    let english_state = settings::format_text(settings::IDS_STATE_AUTOSTART, &["no"]);

    println!("ru: {russian} / {russian_menu} / {russian_state}");
    println!("en: {english} / {english_menu} / {english_state}");

    assert_eq!(russian, "Общие");
    assert_eq!(english, "General");
    assert_eq!(russian_menu, "Выход");
    assert_eq!(english_menu, "Exit");

    // The placeholder of a composed line is filled in and nothing of it is left over.
    assert_eq!(russian_state, "Автозапуск в реестре (HKCU\\…\\Run): нет");
    assert_eq!(
        english_state,
        "Autostart in the registry (HKCU\\…\\Run): no"
    );

    settings::set_ui_language(settings::Language::Ru);
}

#[test]
fn the_language_identifiers_are_the_ones_the_resource_is_tagged_with() {
    // The joint between `app.rc` and the loader, spelled out on both sides of it.
    assert_eq!(settings::Language::Ru.langid(), 0x0419);
    assert_eq!(settings::Language::En.langid(), 0x0409);
    assert_eq!(settings::Language::Ru.tag(), "ru");
    assert_eq!(settings::Language::En.tag(), "en");
}

// -----------------------------------------------------------------------------------------
// FR-94 — the capture
// -----------------------------------------------------------------------------------------

/// No modifier at all.
const BARE: settings::Modifiers = settings::Modifiers {
    ctrl: false,
    alt: false,
    shift: false,
    win: false,
};

#[test]
fn a_captured_key_is_named_the_way_section_7_reads_it_back() {
    // The round trip that keeps `settings::key_name` and `hook::vk_from_name` from drifting:
    // whatever a capture writes into `[hotkey] key`, the reader of that file has to turn back
    // into the very code the user pressed. Every code Windows has, not a chosen few.
    let mut named = 0;

    for vk in 0u16..=254 {
        let Some(name) = settings::key_name(vk) else {
            continue;
        };

        named += 1;

        assert_eq!(
            hook::vk_from_name(&name),
            Some(vk),
            "capture would write «{name}» for 0x{vk:02X}, which reads back as something else"
        );
    }

    println!("{named} virtual keys have a name section 7 can carry");

    // The three shapes of name, each pinned so that a silent narrowing of the vocabulary shows
    // up here rather than in a user's file.
    assert_eq!(settings::key_name(0x41).as_deref(), Some("A"));
    assert_eq!(settings::key_name(0x30).as_deref(), Some("0"));
    assert_eq!(settings::key_name(0x77).as_deref(), Some("F8"));
    assert_eq!(settings::key_name(0x13).as_deref(), Some("Pause"));
    assert!(
        named >= 62,
        "36 characters, 24 function keys and the named ones"
    );
}

#[test]
fn a_press_with_nothing_held_becomes_the_hotkey() {
    // **Criterion 13, the half that needs no window.** The name that comes out is the name that
    // goes into `[hotkey] key`, and it is built out of the virtual-key code alone: no character
    // is produced and no keyboard layout is consulted (SEC-01).
    assert_eq!(
        settings::capture(0x77, BARE),
        settings::Capture::Taken("F8".to_owned())
    );
    assert_eq!(
        settings::capture(0x13, BARE),
        settings::Capture::Taken("Pause".to_owned())
    );

    // ⭐ **A text key is refused since task Т-23-5, решение 82.6** — and it used to be *taken*
    // and warned about afterwards. The user asked for the other order in as many words:
    // «текстовая клавиша — предупреждение и захват НЕ гаснет». A press that was an accident no
    // longer becomes the hotkey; the warning is the same one FR-92 always asked for.
    assert_eq!(
        settings::capture(0x41, BARE),
        settings::Capture::Refused(settings::Refusal::Text)
    );
    assert_eq!(
        settings::Refusal::Text.string_id(),
        settings::IDS_NOTE_TEXT_KEY,
        "the refusal says the warning FR-92 already has, not a sentence of its own"
    );

    // ⚠ And **the file's side is untouched**: a text key named by hand is still a hotkey and
    // still carries the warning of FR-95. What Т-23-5 closed is the one road that assigned one
    // without asking.
    assert_eq!(
        settings::hotkey_note("A"),
        Some(settings::IDS_NOTE_TEXT_KEY)
    );
    assert_eq!(
        settings::effective_hotkey_name("A"),
        "A",
        "a text key in the file acts, and the help of FR-92а names it"
    );
}

#[test]
fn the_capture_refuses_every_press_that_cannot_be_a_hotkey() {
    // **Criterion 16.** Six refusals since task Т-23-5, each with a sentence of its own in both
    // locales — five until решение 82.6 made a text key a refusal instead of an assignment.
    let product = ProductImage::shared();

    let cases = [
        // A modifier on its own — the user has not finished pressing anything.
        (0x11u16, BARE, settings::Refusal::Modifier, "Ctrl"),
        (0x10, BARE, settings::Refusal::Modifier, "Shift"),
        (0x12, BARE, settings::Refusal::Modifier, "Alt"),
        // The `Windows` key: the shell acts on it whatever anybody else does.
        (0x5B, BARE, settings::Refusal::Reserved, "Win"),
        // Anything with a modifier held: section 7 stores one key name and the callback
        // compares one code, so `Ctrl+K` could only be written down as `K`.
        (
            0x4B,
            settings::Modifiers { ctrl: true, ..BARE },
            settings::Refusal::Combination,
            "Ctrl+K",
        ),
        (
            0x77,
            settings::Modifiers { alt: true, ..BARE },
            settings::Refusal::Combination,
            "Alt+F8",
        ),
        // A key section 7 has no name for: it could not be written to the file.
        (0x08, BARE, settings::Refusal::Nameless, "Backspace"),
        (0x0D, BARE, settings::Refusal::Nameless, "Enter"),
        // ⭐ A key that types a character — task Т-23-5, решение 82.6. Every letter, every
        // digit, the space, the numeric pad and the OEM keys: `is_text_key`'s own list.
        // ⚠ The space and the OEM keys answered `Nameless` until that task — both true, and
        // «эта клавиша участвует в наборе» is the one of the two a person can act on.
        (0x41, BARE, settings::Refusal::Text, "буква A"),
        (0x30, BARE, settings::Refusal::Text, "цифра 0"),
        (0x20, BARE, settings::Refusal::Text, "Space"),
        (0xBA, BARE, settings::Refusal::Text, "OEM 1"),
    ];

    for (vk, modifiers, expected, what) in cases {
        let outcome = settings::capture(vk, modifiers);

        println!(
            "{what:9} -> {outcome:?} -> ru: {}",
            product.string(settings::Language::Ru, expected.string_id())
        );

        assert_eq!(
            outcome,
            settings::Capture::Refused(expected),
            "{what} must be refused as {expected:?}"
        );
    }

    // Every refusal has something to say, in both languages, and none of them is empty.
    for refusal in [
        settings::Refusal::Modifier,
        settings::Refusal::Combination,
        settings::Refusal::Emergency,
        settings::Refusal::Reserved,
        settings::Refusal::Text,
        settings::Refusal::Nameless,
    ] {
        for language in [settings::Language::Ru, settings::Language::En] {
            let sentence = product.string(language, refusal.string_id());

            assert!(
                !sentence.is_empty(),
                "{refusal:?} has nothing to say in {}",
                language.tag()
            );
        }
    }
}

// -----------------------------------------------------------------------------------------
// Т-23-5, решение 82.6 — машина захвата: вход, клавиша, отказ, Esc, клик мимо
// -----------------------------------------------------------------------------------------
//
// The user's complaint, word for word: «выделяется Pause белым цветом на тёмном фоне…
// начинает моргать вертикальная линия… юзабилити сильно страдает». The диагноз is that the
// field pretended to be a text field while the capture waited — a caret and a selection are a
// **false affordance** in a control nothing is typed into — and решение 82.6 accepted the live
// mock-up that takes both away.
//
// What can be closed without a window is the **decision**: `settings::capture_step` is the
// whole machine as a pure function of the armed flag and one event, and the window procedures
// keep exactly two jobs — turning a message into an event, and carrying out the answer. The
// look is the user's half, at контрольная точка К-2.

/// No modifiers held — the ordinary case, written out here like the other capture tests.
const NOTHING_HELD: settings::Modifiers = settings::Modifiers {
    ctrl: false,
    alt: false,
    shift: false,
    win: false,
};

#[test]
fn a_capture_that_is_not_armed_answers_nothing_to_any_event() {
    // The gate, and it is the one that makes every other arm safe to write: with no capture
    // armed, a key, a lost focus and a click on the window's ground are all somebody else's
    // business. `Esc` especially — the dialog manager closes the window with it, and a machine
    // that answered `Cancel` here would swallow that.
    for event in [
        settings::CaptureEvent::KeyDown(0x1B, NOTHING_HELD),
        settings::CaptureEvent::KeyDown(0x13, NOTHING_HELD),
        settings::CaptureEvent::FocusLost {
            to_capture_button: false,
        },
        settings::CaptureEvent::FocusLost {
            to_capture_button: true,
        },
        settings::CaptureEvent::ClickBeside,
    ] {
        assert_eq!(
            settings::capture_step(false, event),
            settings::CaptureStep::Ignore,
            "{event:?} must do nothing while no capture is armed"
        );
    }
}

#[test]
fn an_armed_capture_takes_a_key_and_stays_armed_through_a_refusal() {
    // A key section 7 can name ends the capture with that name — the same answer
    // `settings::capture` gives, carried through unchanged.
    assert_eq!(
        settings::capture_step(true, settings::CaptureEvent::KeyDown(0x13, NOTHING_HELD)),
        settings::CaptureStep::Take("Pause".to_owned()),
        "a named key is taken"
    );
    assert_eq!(
        settings::capture_step(true, settings::CaptureEvent::KeyDown(0x77, NOTHING_HELD)),
        settings::CaptureStep::Take("F8".to_owned()),
        "so is a function key"
    );

    // ⚠ **п. 4 of решение 82.6, and it is the arm that keeps a person from being thrown out
    // of the capture for pressing the wrong thing**: a refusal is «not that one», not an end
    // to the question. Every one of the five refusals answers `Refuse` and none of them
    // answers `Cancel`.
    for (vk, held, expected, what) in [
        (0x41u16, NOTHING_HELD, settings::Refusal::Text, "буква A"),
        (0x20, NOTHING_HELD, settings::Refusal::Text, "пробел"),
        (0x08, NOTHING_HELD, settings::Refusal::Nameless, "Backspace"),
        (
            0x11,
            NOTHING_HELD,
            settings::Refusal::Modifier,
            "модификатор",
        ),
        (
            0x5B,
            NOTHING_HELD,
            settings::Refusal::Reserved,
            "клавиша Win",
        ),
        (
            0x13,
            settings::Modifiers {
                ctrl: true,
                alt: false,
                shift: false,
                win: false,
            },
            settings::Refusal::Combination,
            "Ctrl+Pause",
        ),
    ] {
        let step = settings::capture_step(true, settings::CaptureEvent::KeyDown(vk, held));

        println!("{what}: {step:?}");

        assert_eq!(
            step,
            settings::CaptureStep::Refuse(expected),
            "{what} must be refused and the capture must stay armed"
        );
    }
}

#[test]
fn an_armed_capture_is_cancelled_by_escape_by_a_click_beside_and_by_a_lost_focus() {
    // Escape — the way out, and therefore the one key a capture cannot assign. It stays
    // assignable by hand: `hook::vk_from_name` reads `Escape` and `Esc` out of the file.
    assert_eq!(
        settings::capture_step(true, settings::CaptureEvent::KeyDown(0x1B, NOTHING_HELD)),
        settings::CaptureStep::Cancel,
        "Esc cancels"
    );

    // A press on the window's own ground — the half `WM_KILLFOCUS` cannot see, because the
    // statics of this dialog take no focus and a click on them moves nothing.
    assert_eq!(
        settings::capture_step(true, settings::CaptureEvent::ClickBeside),
        settings::CaptureStep::Cancel,
        "a click beside the field cancels — решение 82.6 п. 3"
    );

    // The focus going anywhere else — another control, or another window taking the whole
    // dialog's activation.
    assert_eq!(
        settings::capture_step(
            true,
            settings::CaptureEvent::FocusLost {
                to_capture_button: false
            }
        ),
        settings::CaptureStep::Cancel,
        "a lost focus cancels"
    );

    // ⚠ …except the capture button itself. It is about to report that it was clicked, and
    // cancelling here would turn that click into a fresh arming — the button would then never
    // cancel anything, which is the whole of what it is for while a capture is on.
    assert_eq!(
        settings::capture_step(
            true,
            settings::CaptureEvent::FocusLost {
                to_capture_button: true
            }
        ),
        settings::CaptureStep::Ignore,
        "the capture button may take the focus without ending the capture"
    );
}

/// **Т-23-5, решение 82.6** — the field does not pretend to be a text field.
///
/// The two artefacts the user named — the caret and the selection — are both made by the edit
/// control's own `WM_SETFOCUS` handler, and the cure is that the message never reaches it
/// while a capture is armed. Read off `src\settings.rs`: what a window procedure forwards is
/// the shape of the file and not something a test can press a key at.
#[test]
fn the_field_shows_no_caret_and_no_selection_while_a_capture_is_armed() {
    let source = settings_module_source();
    let field = function_body(&source, "unsafe extern \"system\" fn hotkey_field_proc(");

    assert!(
        field.contains("WM_SETFOCUS => return LRESULT(0),"),
        "the field's procedure must swallow WM_SETFOCUS while armed — the edit's own handler \
         is what makes the caret and selects the whole text"
    );

    // And nothing anywhere in the module unselects after the fact: a selection removed by a
    // message has already been painted once, and `EM_SETSEL` is the message that would do it.
    //
    // ⚠ **The comment lines are stripped first, and that is the whole reason this is not a
    // bare `contains`.** `src\settings.rs` explains at the arm above why it does *not* send
    // that message, and a probe looking for the name in a file that spells the name in its own
    // prose would be measuring itself and passing for ever.
    let sending: Vec<&str> = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter(|line| line.contains("EM_SETSEL"))
        .collect();

    println!("строки кода с EM_SETSEL: {sending:?}");

    assert!(
        sending.is_empty(),
        "no EM_SETSEL is sent: the selection is never made, so there is nothing to unmake"
    );

    // The control of the probe above, in one line: the prose the strip removes really is
    // there, so a green answer means «no call site» and not «no such word in the file».
    assert!(
        source.contains("EM_SETSEL"),
        "the module must go on saying why it sends no EM_SETSEL — otherwise the sweep above \
         is measuring an absence nobody wrote down"
    );

    // The invitation stands in the field and the way out under it — the two halves of the
    // mock-up, in the one function that arms a capture.
    let arm = function_body(&source, "fn arm_capture(");

    assert!(
        arm.contains("set_text(hwnd, IDC_HOTKEY, &text(IDS_CAPTURE_PROMPT));"),
        "the field must show the invitation while the capture waits"
    );
    assert!(
        arm.contains("show_capture_note(hwnd, None);"),
        "and the note under it must show the way out"
    );
    assert!(
        arm.contains("set_text(hwnd, IDC_HOTKEY_CAPTURE, &text(IDS_HOTKEY_STOP));"),
        "and the button must offer to take it back"
    );
    assert!(
        !arm.contains("focus_control("),
        "the focus is moved by `toggle_capture` **after** the borrow ends — a `SetFocus` from \
         inside it sends WM_SETFOCUS while the state cannot be read, and the field would then \
         hand the message to the edit control"
    );

    // ⭐ **Находка Т-23-5** — the note is invalidated the moment it is written.
    //
    // `SetDlgItemTextW` on an `SS_OWNERDRAW` static moves the text the control stores and not
    // one pixel of the screen: this module draws that control out of a `WM_DRAWITEM`, and
    // nothing asks for one until the control is invalidated. Every note of the capture went
    // through that call alone, so on a **raised** window the five refusals of FR-94 were
    // written and never shown — as old as task T-11-18 and found by looking at the stand.
    //
    // The one place the note is written is `set_note`, and the invalidation lives there beside
    // the write, exactly as it does in `set_check` for an owner-drawn button.
    let note = function_body(&source, "fn set_note(");

    assert!(
        note.contains("set_text(hwnd, IDC_HOTKEY_NOTE, note);"),
        "`set_note` must write the note"
    );
    assert!(
        note.contains("repaint_control(hwnd, IDC_HOTKEY_NOTE);"),
        "…and invalidate it, or the text it wrote stays invisible on a raised window"
    );

    // And nowhere else writes that control: a second `set_text` on it would be a second place
    // for the defect to come back.
    let writers: Vec<&str> = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter(|line| line.contains("IDC_HOTKEY_NOTE,") && line.contains("set_text"))
        .collect();

    println!("места записи заметки: {writers:?}");

    assert_eq!(
        writers.len(),
        1,
        "the note must be written in one place — `set_note` — and repainted there"
    );

    // The frame and the ink, the two colours the capture moves.
    assert!(
        function_body(&source, "unsafe fn on_erase_background(").contains("palette.box_border"),
        "the frame of the field must go to `box_border` while a capture is armed"
    );
    assert!(
        function_body(&source, "unsafe fn on_ctl_color(").contains("palette.text_muted"),
        "the invitation must be written in the quiet ink"
    );
}

#[test]
fn the_emergency_combination_of_fr_96_is_never_captured() {
    // **Criterion 14, the half a test can reach.** The other half — that `Ctrl+Alt+Shift+F12`
    // ends the process *while a capture is armed* — is a property of the hook callback, which
    // handles it before it reads anything this module can influence, and is shown on the running
    // program in the behavioural run.
    let all_three = settings::Modifiers {
        ctrl: true,
        alt: true,
        shift: true,
        win: false,
    };

    assert_eq!(
        settings::capture(hook::EMERGENCY_VK, all_three),
        settings::Capture::Refused(settings::Refusal::Emergency)
    );

    // The `Windows` key held as well changes nothing: `hook::emergency_modifiers_held` asks
    // about three modifiers and this asks the same question.
    assert_eq!(
        settings::capture(
            hook::EMERGENCY_VK,
            settings::Modifiers {
                win: true,
                ..all_three
            }
        ),
        settings::Capture::Refused(settings::Refusal::Emergency)
    );

    // F12 on its own is an ordinary key and is taken — the refusal is about the combination and
    // not about the key.
    assert_eq!(
        settings::capture(hook::EMERGENCY_VK, BARE),
        settings::Capture::Taken("F12".to_owned())
    );
}

#[test]
fn a_capture_stops_the_conversion_and_gives_it_back_when_it_ends() {
    // **Criterion 15.** While a capture is armed the callback is published as inactive, which
    // `hook::classify` answers by passing every stroke through: no conversion is fired, nothing
    // is suppressed, and nothing reaches the typing buffer either. The proof is the callback's
    // own decision function, called with the mode the callback would really see.
    let _guard = with_product_strings();

    hook::set_active(true);
    hook::set_hotkey_vk(hook::DEFAULT_HOTKEY_VK);

    let press = hook::KeyEvent {
        vk: hook::DEFAULT_HOTKEY_VK,
        edge: hook::Edge::Down,
        extra_info: 0,
        scan: 0,
        flags: 0,
        time: 0,
    };

    let before = hook::classify(
        hook::current_mode(),
        &mut hook::HotkeyState::default(),
        press,
    );
    println!("before the capture: {before:?}");
    assert!(before.fire_hotkey, "the hotkey fires when nothing is armed");

    {
        let session = settings::CaptureSession::arm("Pause".to_owned());

        assert!(session.hook_was_active());
        assert_eq!(session.previous_key(), "Pause");
        assert_eq!(session.published_vk(), hook::DEFAULT_HOTKEY_VK);
        assert!(
            !hook::is_active(),
            "arming a capture must suspend the conversion path"
        );
        assert_eq!(
            hook::hotkey_vk(),
            0,
            "and it must publish a code no keyboard can produce"
        );

        let during = hook::classify(
            hook::current_mode(),
            &mut hook::HotkeyState::default(),
            press,
        );
        println!("during the capture: {during:?}");

        assert!(
            !during.fire_hotkey,
            "a press made while choosing a hotkey must not convert anything"
        );

        // ⚠ And the same with the active flag put back to `true` under the capture, which is
        // what `app::window_proc` really does after every message the UI thread sees. The
        // suspension has to survive that, and it does, because it does not rest on that flag.
        hook::set_active(true);

        let clobbered = hook::classify(
            hook::current_mode(),
            &mut hook::HotkeyState::default(),
            press,
        );
        println!("with the active flag re-published under it: {clobbered:?}");

        assert!(
            !clobbered.fire_hotkey,
            "the suspension must not depend on a flag another module re-publishes"
        );
        assert_eq!(
            clobbered.decision,
            hook::Decision::Pass,
            "and the stroke must still reach the window that is capturing it"
        );
    }

    // Dropping the session — which is what cancelling, accepting and closing the dialog all do —
    // publishes the previous state back.
    assert!(hook::is_active(), "the conversion path comes back");
    assert_eq!(
        hook::hotkey_vk(),
        hook::DEFAULT_HOTKEY_VK,
        "and so does the hotkey"
    );

    let after = hook::classify(
        hook::current_mode(),
        &mut hook::HotkeyState::default(),
        press,
    );
    println!("after the capture: {after:?}");
    assert!(after.fire_hotkey);

    // A capture armed while the program was suspended gives back "suspended" and not "active".
    hook::set_active(false);
    drop(settings::CaptureSession::arm("Pause".to_owned()));
    assert!(!hook::is_active(), "a suspended program stays suspended");

    hook::set_active(true);
}

#[test]
fn the_captured_key_reaches_the_hook_through_publish_configuration() {
    // **Criteria 13, 18 and 25.** The dialog publishes nothing itself: it puts the captured name
    // into the configuration and «Применить» hands that configuration to
    // `app::publish_configuration`, which is the one caller of `hook::set_hotkey_vk` there has
    // ever been. `src\hook.rs` is not touched by this task.
    let _guard = with_product_strings();
    let _publishing = publishing();

    let mut config = Config::default();

    let settings::Capture::Taken(name) = settings::capture(0x77, BARE) else {
        panic!("F8 with nothing held must be capturable");
    };

    config.hotkey.key = name.clone();
    app::publish_configuration(&config);

    println!(
        "captured «{name}» -> hook::hotkey_vk() = 0x{:02X}",
        hook::hotkey_vk()
    );

    assert_eq!(name, "F8");
    assert_eq!(hook::hotkey_vk(), 0x77);

    // And back to the default of section 7, so that this test cannot leave the process with a
    // hotkey another test did not ask for.
    config.hotkey.key = "Pause".to_owned();
    app::publish_configuration(&config);
    assert_eq!(hook::hotkey_vk(), hook::DEFAULT_HOTKEY_VK);
}

// ---------------------------------------------------------------------------------------
// FR-92а — the system theme changing under the open dialog. Task T-11-9.
// ---------------------------------------------------------------------------------------

use lang_switcher::theme::Palette;

/// **Criterion 9 of task T-11-9.** «Перекрашивать?» — the full table of the three axes
/// the task names: the string the `WM_SETTINGCHANGE` carried (the exact word / a foreign
/// word / an empty word / no string at all — a null `lParam` reads as `None` long before
/// the decision), the setting in force (`system` / `dark` — under a fixed setting the
/// system has no say), and whether the palette the system now resolves to is the one
/// already on the screen. Sixteen rows, one `true`.
///
/// The rows are written out rather than derived, and the expectations are literals: a table
/// computed the way the code computes it would check nothing.
#[test]
fn the_repaint_decision_of_fr_92a_follows_the_full_table() {
    /// One row: the string, the setting, (палитра сейчас, палитра по системе), the answer.
    type RepaintRow = (
        Option<&'static str>,
        ThemeSetting,
        (&'static Palette, &'static Palette),
        bool,
    );

    let same: (&Palette, &Palette) = (&GRAPHITE, &GRAPHITE);
    let moved: (&Palette, &Palette) = (&GRAPHITE, &FOG);

    #[rustfmt::skip]
    let table: [RepaintRow; 16] = [
        // The one row that repaints: the exact word, the setting `system`, the palette moved.
        (Some("ImmersiveColorSet"),   ThemeSetting::System, moved, true),
        (Some("ImmersiveColorSet"),   ThemeSetting::System, same,  false),
        (Some("ImmersiveColorSet"),   ThemeSetting::Dark,   moved, false),
        (Some("ImmersiveColorSet"),   ThemeSetting::Dark,   same,  false),
        // A foreign word: no, whatever the setting and wherever the palettes stand.
        (Some("WindowsThemeElement"), ThemeSetting::System, moved, false),
        (Some("WindowsThemeElement"), ThemeSetting::System, same,  false),
        (Some("WindowsThemeElement"), ThemeSetting::Dark,   moved, false),
        (Some("WindowsThemeElement"), ThemeSetting::Dark,   same,  false),
        // An empty word — a `WM_SETTINGCHANGE` that named nothing: same answer as foreign.
        (Some(""),                    ThemeSetting::System, moved, false),
        (Some(""),                    ThemeSetting::System, same,  false),
        (Some(""),                    ThemeSetting::Dark,   moved, false),
        (Some(""),                    ThemeSetting::Dark,   same,  false),
        // No string at all — the null `lParam` of SEC-05, read as `None` by the receiver.
        (None,                        ThemeSetting::System, moved, false),
        (None,                        ThemeSetting::System, same,  false),
        (None,                        ThemeSetting::Dark,   moved, false),
        (None,                        ThemeSetting::Dark,   same,  false),
    ];

    for (string, setting, (current, by_system), expected) in table {
        assert_eq!(
            settings::repaint_for_system_theme(string, setting, current, by_system),
            expected,
            "for {string:?} under {setting:?}, palette moved: {}",
            !std::ptr::eq(current, by_system)
        );
    }
}

/// **Criterion 12 of task T-11-9.** The dialog's system-theme message is `WM_APP + 14` — the
/// literal `0x8000 + 14`, not the constant read back — and it collides with none of the
/// public numbers of the occupied row. The three private ones (`+ 1` and `+ 5` of `app`,
/// `+ 2` of the tray) are out of an integration test's reach; they are not `14` by the same
/// literal this test pins.
#[test]
fn the_system_theme_message_is_wm_app_plus_14_and_collides_with_nothing_public() {
    assert_eq!(settings::WM_APP_SYSTEM_THEME, 0x8000 + 14);

    for occupied in [
        lang_switcher::hook::WM_APP_HOTKEY,
        lang_switcher::hook::WM_APP_FAIL_SAFE,
        lang_switcher::watchdog::WM_APP_FLUSH,
        lang_switcher::watchdog::WM_APP_LAYOUT,
        lang_switcher::watchdog::WM_APP_REHOOK,
        lang_switcher::switch::WM_APP_SWITCH,
        lang_switcher::guard::WM_APP_PROBE,
        lang_switcher::guard::WM_APP_FIELD,
        lang_switcher::selection::WM_APP_SELECTION,
        lang_switcher::selection::WM_APP_BUFFER_PATH,
    ] {
        assert_ne!(
            settings::WM_APP_SYSTEM_THEME,
            occupied,
            "two private messages share a number"
        );
    }
}

// ---------------------------------------------------------------------------------------
// FR-92а — the system theme changing under the open «О программе» window. Task T-13-17.
// ---------------------------------------------------------------------------------------
//
// The finding of the audit of 2026-08-24: FR-92а names this window among «всё видимое
// глазом» and asks for a system theme change to reach open windows without a restart, and
// this one resolved its palette once and was told nothing afterwards. What is measurable
// without a live window is the record and the shape — who writes it down, who clears it, and
// whether the reaction to the message is a reading of the system rather than of the message.
// The look on the screen is the controller's, on the real window: the стенд, not a test.

use lang_switcher::settings::AboutSession;
use windows::Win32::Foundation::HWND;

/// A handle that names no window, for the record tests below.
///
/// Deliberately not a live window. What is under test is the *record* — an integer this
/// module writes down and clears — and a test that opened a real window would be testing the
/// dialog manager instead. Nothing is ever sent to this number: the tests below assert that
/// the record is gone before anything could be.
fn a_handle_that_names_no_window() -> HWND {
    HWND(0x1357 as *mut std::ffi::c_void)
}

/// **Criterion 6 of task T-13-17, and ловушка 1 of it.** The window of «О программе» is
/// registered while it lives and unregistered on the way out — whatever the way out is.
///
/// The ordinary road and the panic are both walked here, because the guard exists precisely
/// so that they are one road: the window is closed by «ОК», by `Esc`, by the cross of the
/// caption and — in Debug — by a failed `debug_assert`, and a record cleared in a handler
/// would be cleared on some of those and not on others.
#[test]
fn the_about_window_record_is_cleared_on_every_exit_path() {
    assert!(
        !settings::about_is_open(),
        "no about window has been opened on this thread"
    );

    {
        let _open = AboutSession::open();

        assert!(
            !settings::about_is_open(),
            "the guard alone names no window — there is none until `WM_INITDIALOG`"
        );

        AboutSession::record(a_handle_that_names_no_window());

        assert!(
            settings::about_is_open(),
            "a recorded window is the whole of what «about жив» means"
        );
    }

    assert!(
        !settings::about_is_open(),
        "the record must not outlive the frame that owns the window"
    );

    // The other road. The program stays up after a panic (FR-98, FR-99), and a record left
    // behind would keep `on_system_theme_message` posting to a number Windows is free to
    // have given to somebody else's window by then.
    let unwound = std::panic::catch_unwind(|| {
        let _open = AboutSession::open();
        AboutSession::record(a_handle_that_names_no_window());
        assert!(settings::about_is_open(), "recorded inside the frame");
        panic!("the frame is left by a panic");
    });

    assert!(unwound.is_err(), "the panic must have been the way out");
    assert!(
        !settings::about_is_open(),
        "a panic on the way out clears the record like every other exit path"
    );
}

/// **Criterion 6 of task T-13-17, second half.** A theme message that arrives after the
/// window is gone is delivered to nobody and does nothing.
///
/// The handle recorded and released here names no window at all. Were the record to survive
/// its guard, this call would take that number out again and post `WM_APP_SYSTEM_THEME` to
/// whatever Windows has since made of it; it returns having touched nothing, because an
/// empty record is the first exit of the function — before `with_about_state`, before any
/// reading of the system switch and before any `PostMessageW`.
#[test]
fn a_theme_message_after_the_about_window_closed_reaches_nobody() {
    {
        let _open = AboutSession::open();
        AboutSession::record(a_handle_that_names_no_window());
        assert!(settings::about_is_open());
    }

    assert!(
        !settings::about_is_open(),
        "the window is closed, so there is nobody to tell"
    );

    // All three shapes the tray can hand in: the exact word, a foreign word, and the null
    // `lParam` of SEC-05 read as `None`.
    settings::on_system_theme_message(Some(settings::IMMERSIVE_COLOR_SET));
    settings::on_system_theme_message(Some("WindowsThemeElement"));
    settings::on_system_theme_message(None);

    assert!(
        !settings::about_is_open(),
        "and nothing about the call resurrected the record"
    );
}

/// **Criterion 2 of task T-13-17 — ловушка 2.** One delivery road, two recipients.
///
/// `on_system_theme_message` already decided who to tell; after this task it tells two, out
/// of the two records, through the one posting site. A second mechanism — the about window
/// listening for `WM_SETTINGCHANGE` on its own, or a second `PostMessageW` of this message
/// somewhere else — would be a second answer to «пора ли перекраситься».
#[test]
fn the_theme_nudge_has_one_delivery_road_and_two_recipients() {
    let source = settings_module_source();
    let body = function_body(
        &source,
        "pub fn on_system_theme_message(setting_string: Option<&str>) {",
    );

    for record in ["DIALOG_WINDOW", "ABOUT_WINDOW"] {
        assert!(
            body.contains(&format!("live_window(&{record})")),
            "`on_system_theme_message` must reach {record}"
        );
    }

    assert_eq!(
        body.matches("post_system_theme(hwnd);").count(),
        2,
        "both recipients are posted to, and both through the shared site"
    );

    let posts = product_lines_with("PostMessageW(Some(hwnd), WM_APP_SYSTEM_THEME");
    println!("WM_APP_SYSTEM_THEME posting sites: {posts:?}");
    assert_eq!(
        posts.len(),
        1,
        "the message is posted from one place in the module and no other"
    );

    // And the about window does not listen for the broadcast itself: the tray's hidden
    // window is the one window of this program that reads `WM_SETTINGCHANGE` (§6.2).
    let about = function_body(&source, "unsafe extern \"system\" fn about_proc(");
    assert!(
        !about.contains("WM_SETTINGCHANGE") && !about.contains("WM_THEMECHANGED"),
        "the about window is told by the one road and does not open a second one"
    );
}

/// **Criteria 3 and 4 of task T-13-17.** The about window answers the nudge by asking the
/// system, and its caption goes the road the dialog's caption goes.
///
/// SEC-05 in one test: the message carries nothing, and the palette that comes out of the
/// handler is `theme::resolve` over this program's own setting and this program's own
/// reading of `AppsUseLightTheme`. A forged message cannot impose a palette, because there
/// is no place in the message for one to be named.
#[test]
fn the_about_window_answers_the_nudge_by_re_reading_the_system() {
    let source = settings_module_source();

    let about = function_body(&source, "unsafe extern \"system\" fn about_proc(");
    assert!(
        about.contains("WM_APP_SYSTEM_THEME => {"),
        "the about procedure must handle the nudge at all — it did not before this task"
    );
    assert!(
        about.contains("with_about_state(hwnd, |state| refresh_about_palette(hwnd, state))"),
        "and answer it out of its own state"
    );

    let refresh = function_body(
        &source,
        "fn refresh_about_palette(hwnd: HWND, state: &mut AboutState) {",
    );

    for line in [
        // The reaction is a reading of the system's switch under the window's own setting.
        "let fresh = theme::resolve(state.setting, theme::system_is_light());",
        // The identity test — the correctness check and the debounce of the batch in one.
        "if !std::ptr::eq(fresh, state.palette)",
        // The caption of the window: the one shared road (п. 3), not a second DWM path.
        "apply_title_bar_theme(hwnd, fresh);",
        // And the visible half.
        "repaint_whole_window(hwnd);",
    ] {
        assert!(
            refresh.contains(line),
            "`refresh_about_palette` must carry the line `{line}`"
        );
    }

    // No second road to DWM: every caption of this module goes through the one function.
    let dwm = product_lines_with("DwmSetWindowAttribute(");
    println!("DwmSetWindowAttribute call sites: {dwm:?}");
    assert_eq!(
        dwm.len(),
        2,
        "the two calls of `apply_title_bar_theme` and `set_caption_colour`, and no third"
    );
}

/// **Criterion 1 of task T-13-17.** The record has exactly two writers — the one that names
/// the window and the one that forgets it — and the guard is taken before the modal call.
#[test]
fn the_about_record_is_written_by_the_pair_and_by_nobody_else() {
    let source = settings_module_source();

    let writes = product_lines_with("ABOUT_WINDOW.with(|window| window.set(");
    println!("ABOUT_WINDOW writers: {writes:?}");
    assert_eq!(
        writes.len(),
        2,
        "one writer names the window, one forgets it, and there is no third"
    );
    assert!(
        writes
            .iter()
            .any(|line| line.contains("window.set(hwnd.0 as isize)")),
        "the window is written down as the plain integer the handle is"
    );
    assert!(
        writes.iter().any(|line| line.contains("window.set(0)")),
        "and cleared to the zero that means «нет окна»"
    );

    let dropped = function_body(&source, "impl Drop for AboutSession {");
    assert!(
        dropped.contains("ABOUT_WINDOW.with(|window| window.set(0));"),
        "the clearing belongs to the guard's `Drop` and to nothing else"
    );

    let show = function_body(&source, "pub fn show_about_dialog(");
    let guard = show
        .find("let _open = AboutSession::open();")
        .expect("`show_about_dialog` must take the guard");
    // ⚠ Task Т-30-2 renamed what stands here: the modal call is now `show_modal_dialog`, which
    // is `DialogBoxIndirectParamW` on a copy of the template for a right-to-left locale and
    // `DialogBoxParamW` for the other twelve. The sweep names the new call, because what it is
    // really about is the **order** — the guard before the modal call — and that is unchanged.
    let modal = show
        .find("show_modal_dialog(")
        .expect("`show_about_dialog` must still go through the modal call");

    assert!(
        guard < modal,
        "the guard is taken before the modal call, so the `-1` return is covered too"
    );
}

// =========================================================================================
// FR-92а — своё сглаживание на чистом GDI и серое сглаживание нашего текста. Task T-11-17.
// =========================================================================================
//
// What is measurable without a window: the pure halves — the two `LOGFONTW` builders, the four
// corner tiles of a rounded rectangle, the bounding box of a stroke, the box filter the
// reduction is since task T-11-23 — and the shape of the module's own source, where the
// supersampling factor, the ownership of every GDI object and the caching of the background
// live. The look on the screen is the controller's, on the real window: the product is not
// started by any test.

/// The lines of the module source that are **not** comments and hold `needle`.
///
/// The sweeps below have to separate a call from a sentence about a call: the section of the
/// module these tasks added explains at length what `HALFTONE` did and why it is gone, and a
/// count that swept the prose in with the code would prove nothing — or, since task T-11-23,
/// would prove the opposite of the truth.
fn product_lines_with(needle: &str) -> Vec<String> {
    // T-14-3: the drawing of FR-92а is two files since the engine moved to `theme`; see
    // [`drawing_source`] for why the join keeps every sweep below meaning what it meant.
    drawing_source()
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//") && line.contains(needle))
        .map(str::to_owned)
        .collect()
}

/// **Criterion 13 of T-11-17, as решение 88 left it** — every face this program sets its own
/// text in is asked for the **named** smoothing, and the ask is a pure function of a `LOGFONTW`.
///
/// ⚠ **The mode is ClearType since решение 88 (2026-09-02) and was grey coverage before it.**
/// T-11-17 chose grey against the colour fringe; решение 88 chose ClearType against the
/// «двоение» grey coverage produces when GDI puts a fractional stem on a whole pixel — both
/// modes rendered side by side at 6× and picked by eye. The derivation lives at
/// `theme::smoothed_logfont`.
///
/// What this test is really for has not changed, and it is **not** the value: it is that the
/// program **names** a mode instead of inheriting one. The expected number is written out here
/// as the `CLEARTYPE_QUALITY` of `wingdi.h` — the literal 5 — and not only read back from the
/// crate: a builder that quietly left the manager's `DEFAULT_QUALITY` in place would agree with
/// itself, look identical on this machine, and be wrong on a machine whose smoothing is off.
#[test]
fn our_own_faces_are_asked_for_the_named_smoothing_and_nothing_else_moves() {
    // A face with something recognisable in every field the builders must not touch.
    let mut base = LOGFONTW {
        lfHeight: -18,
        lfWidth: 7,
        lfWeight: 400,
        lfItalic: 1,
        lfUnderline: 1,
        lfStrikeOut: 1,
        // ⚠ Deliberately the mode the builder must **write over**, and deliberately not the
        // one it writes: `DEFAULT_QUALITY` is what the dialog manager's own font carries, and
        // a builder that changed nothing at all would be caught by this starting point.
        lfQuality: DEFAULT_QUALITY,
        ..Default::default()
    };

    // «Segoe UI», the face the template asks for, as UTF-16 into the fixed array.
    for (slot, unit) in base.lfFaceName.iter_mut().zip("Segoe UI".encode_utf16()) {
        *slot = unit;
    }

    let text = theme::smoothed_logfont(base);

    assert_eq!(
        text.lfQuality.0, 5,
        "the quality must be CLEARTYPE_QUALITY — the 5 of wingdi.h (решение 88)"
    );
    assert_eq!(
        text.lfQuality, CLEARTYPE_QUALITY,
        "and it must be the constant the crate names 5 by"
    );
    assert_ne!(
        text.lfQuality, base.lfQuality,
        "the builder must NAME a mode, not pass the manager's own through"
    );
    assert_ne!(
        text.lfQuality, ANTIALIASED_QUALITY,
        "grey coverage is what решение 88 takes off our own text — it is what «двоило»"
    );

    // And not one other field moved: the face, the size, the weight and the rest are the
    // window's own, which is why the builder takes a `LOGFONTW` instead of naming a face.
    assert_eq!(text.lfHeight, base.lfHeight);
    assert_eq!(text.lfWidth, base.lfWidth);
    assert_eq!(text.lfWeight, base.lfWeight);
    assert_eq!(text.lfItalic, base.lfItalic);
    assert_eq!(text.lfUnderline, base.lfUnderline);
    assert_eq!(text.lfStrikeOut, base.lfStrikeOut);
    assert_eq!(text.lfFaceName, base.lfFaceName);

    // The caption face: the same smoothing, and the two changes п. 2.1 of T-11-13 asks for.
    let caption = settings::caption_logfont(base);

    assert_eq!(
        caption.lfQuality.0, 5,
        "a panel caption is our own text as much as a button caption is"
    );
    assert_eq!(
        caption.lfHeight,
        (base.lfHeight * 83) / 90,
        "8,3 pt against 9 pt — решение 89, the caption put back into the proportion the \
         accepted mock-up gives it against the body"
    );
    assert!(
        caption.lfHeight < 0,
        "a height asked for by character height stays negative"
    );
    assert_eq!(caption.lfWeight, 700, "FW_BOLD");
    assert_eq!(
        caption.lfWeight,
        i32::try_from(FW_BOLD.0).expect("FW_BOLD fits in an i32"),
        "and it is the constant the crate names 700 by"
    );
    assert_eq!(
        caption.lfFaceName, base.lfFaceName,
        "the caption is the dialog's own face, shrunk — never a face named in the module"
    );

    // ---- Т-26-2, решение 87: the four faces of the about window ---------------------------
    //
    // Every one of them is the dialog's own `lfHeight` scaled by a ratio, never a point size
    // written by hand — that is what makes them right at every DPI. The ratios are recomputed
    // here from the published constants rather than read back as results.
    let body = settings::about_body_logfont(base);

    assert_eq!(
        body.lfHeight,
        (base.lfHeight * settings::ABOUT_BODY_POINTS_TENTHS) / 90,
        "the body is 10 pt against the dialog's 9"
    );
    assert!(
        body.lfHeight < base.lfHeight,
        "a negative height grows by getting smaller: {} against {}",
        body.lfHeight,
        base.lfHeight
    );
    assert_eq!(
        body.lfWeight, base.lfWeight,
        "⚠ the body is ORDINARY weight — the hypothesis that these rows were drawn bold is \
         refuted, and the repair is size, not weight"
    );
    assert_eq!(
        body.lfFaceName, base.lfFaceName,
        "the body is the dialog's own face, one step larger"
    );

    // The three emphasised roles. `Bold` is the dialog's own family at 700 — the picture of
    // task T-12-4 and the fallback of решение 87 п. 1.
    for (role, logfont) in [
        (
            "имя",
            settings::about_name_logfont(base, settings::Emphasis::Bold),
        ),
        (
            "номер",
            settings::about_number_logfont(base, settings::Emphasis::Bold),
        ),
        (
            "чип",
            settings::about_chip_logfont(base, settings::Emphasis::Bold),
        ),
    ] {
        println!(
            "{role}, откат: lfHeight {}, вес {}",
            logfont.lfHeight, logfont.lfWeight
        );

        assert_eq!(
            logfont.lfWeight, 700,
            "{role}: the fallback of решение 87 is FW_BOLD"
        );
        assert_eq!(
            logfont.lfFaceName, base.lfFaceName,
            "{role}: the fallback stays the dialog's own family"
        );
        assert_eq!(
            logfont.lfQuality.0, 5,
            "{role}: our own text carries the smoothing решение 88 names"
        );
    }

    // `Semibold` is the one exception решение 87 authorises: a family named in the module.
    // ⚠ The weight is left at **zero** — `FW_DONTCARE` — on purpose: the semibold family has
    // one weight, and asking it for 700 on top invites the synthetic bold GDI makes when it
    // cannot find what it was asked for.
    for (role, logfont) in [
        (
            "имя",
            settings::about_name_logfont(base, settings::Emphasis::Semibold),
        ),
        (
            "номер",
            settings::about_number_logfont(base, settings::Emphasis::Semibold),
        ),
        (
            "чип",
            settings::about_chip_logfont(base, settings::Emphasis::Semibold),
        ),
    ] {
        let family: String = String::from_utf16_lossy(&logfont.lfFaceName)
            .trim_end_matches('\0')
            .to_owned();

        println!(
            "{role}, Ш-2: семейство «{family}», вес {}",
            logfont.lfWeight
        );

        assert_eq!(
            family, "Segoe UI Semibold",
            "{role}: the family of решение 87 п. 1"
        );
        assert_eq!(
            logfont.lfWeight, 0,
            "{role}: the weight is the family's own business"
        );
        assert_ne!(
            logfont.lfFaceName, base.lfFaceName,
            "{role}: this is the one place the module names a face, and it must differ"
        );
    }

    // The two emphases differ in the family and **not** in the size: a system without the
    // semibold family must lay the window out identically, or the fallback would move text.
    for (semibold, bold) in [
        (
            settings::about_name_logfont(base, settings::Emphasis::Semibold),
            settings::about_name_logfont(base, settings::Emphasis::Bold),
        ),
        (
            settings::about_chip_logfont(base, settings::Emphasis::Semibold),
            settings::about_chip_logfont(base, settings::Emphasis::Bold),
        ),
    ] {
        assert_eq!(
            semibold.lfHeight, bold.lfHeight,
            "the fallback must not change the size, only the weight"
        );
    }
}

/// **The line pitch of the about window's body is a share of its own face** — task Т-26-2,
/// решение 85, «воздух межстрочья».
///
/// `line-height: 1.4` of the accepted mock-up, and a share rather than a length of a picture,
/// so it grows with the face and with the DPI in one step. The property that makes it *work* is
/// the one asserted last: it must come out **larger** than the natural line of that face, or
/// `theme::label_model_pitch` refuses it and the air never appears.
#[test]
fn the_body_line_pitch_is_a_share_of_the_face_and_larger_than_its_natural_line() {
    // The two faces this program is really drawn at: 96 DPI (`lfHeight` −13 for the body) and
    // 120 DPI (−16). Both measured on the raised window, not predicted.
    for em in [13, 16, 26] {
        let pitch = settings::about_body_line_pitch(em);

        println!("тело {em} px → шаг {pitch} px");

        assert_eq!(pitch, (em * settings::ABOUT_BODY_LINE_PERCENT) / 100);

        // Segoe UI advances a line by about 1,33 em of its own accord (`tmHeight`), and the
        // mock-up asks for 1,4 — so the pitch has something to give at every size.
        let natural = (em * 133) / 100;

        assert!(
            pitch > natural,
            "a pitch of {pitch} against a natural line of {natural} gives no air at all — \
             `label_model_pitch` would refuse it and the window would look as it did before"
        );
    }

    // A negative `lfHeight` is what the manager really hands over, and the pitch is a length:
    // it must come out positive whichever sign it was given.
    assert_eq!(
        settings::about_body_line_pitch(-13),
        settings::about_body_line_pitch(13),
        "the sign of lfHeight is not the sign of a distance"
    );
}

/// **Criterion 10 of T-11-17, as task T-11-23 left it** — the supersampling factor is a
/// constant, the reduction is an average of **ours**, and `HALFTONE` is gone from the module.
///
/// ⚠ This test used to demand the opposite, and was wrong to. Until T-11-23 step 3 of the
/// smoothing was a `StretchBlt` in `HALFTONE` mode, on the belief that the mode averages the
/// block of source pixels behind a destination pixel. It does not: `HALFTONE` *halftones* —
/// MSDN says the average over the destination **block of pixels** approximates the source,
/// which is a dither, right over an area and wrong at a pixel. Measured on this machine, an arc
/// reduced 4 : 1 from `32,35,41` to `228,231,234` came back with 116 of 256 pixels outside the
/// two colours it was made of, the darkest `2,5,12` and the brightest `255,255,255`; the same
/// bits averaged by hand gave none. So the mode is not merely unnecessary here — every line
/// that names it is a defect, which is what the sweeps below say.
#[test]
fn the_smoothing_is_a_named_factor_and_an_average_of_our_own_with_no_halftone_left() {
    // Four samples each way, sixteen per pixel — the factor `tools\make-icons.ps1` draws at.
    assert_eq!(theme::SUPERSAMPLE, 4);

    // And a ceiling on what may be enlarged whole, so that a smoothed detail cannot quietly
    // become a smoothed panel: the largest surface this file can ask for is 256 × 256.
    assert_eq!(theme::SUPERSAMPLE_MAX_SIDE, 64);
    assert_eq!(
        theme::SUPERSAMPLE_MAX_SIDE * theme::SUPERSAMPLE,
        256,
        "the widest enlarged surface the module can ask for"
    );

    // The block one destination pixel is averaged from is the factor squared, and it is a
    // constant rather than a number written at the loop.
    assert_eq!(
        theme::SUPERSAMPLE_BLOCK,
        (theme::SUPERSAMPLE * theme::SUPERSAMPLE) as usize
    );
    assert_eq!(theme::SUPERSAMPLE_BLOCK, 16);

    // T-14-3: the drawing is two files now — see `drawing_source`.
    let source = drawing_source();

    // ⚠ Not one line of code may name the mode again — see the note above this test.
    let halftone = product_lines_with("HALFTONE");
    assert!(
        halftone.is_empty(),
        "HALFTONE dithers instead of averaging and has no place in the reduction: {halftone:?}"
    );

    // And with the mode goes the `SetBrushOrgEx` the documentation paired with it: there is no
    // dither pattern left to align.
    let origin = product_lines_with("SetBrushOrgEx");
    assert!(
        origin.is_empty(),
        "the brush origin was there for HALFTONE and goes with it: {origin:?}"
    );

    // The way **up** is still a blit, and still replicates rather than blends: a ground that
    // arrived already smeared would smear the whole tile.
    let enlarging = product_lines_with("SetStretchBltMode(dc, COLORONCOLOR)");
    assert_eq!(
        enlarging.len(),
        1,
        "the enlargement must replicate, not blend: {enlarging:?}"
    );

    // …and it is the only scaling blit the module makes at all. The way down does not scale.
    let stretches = product_lines_with("StretchBlt(");
    assert_eq!(
        stretches.len(),
        1,
        "the one StretchBlt of the module is the enlargement: {stretches:?}"
    );

    // The way **down** is arithmetic, in one place, through the pure box filter.
    let averaging = product_lines_with("average_of_block(&block)");
    assert_eq!(
        averaging.len(),
        1,
        "the reduction averages in exactly one place: {averaging:?}"
    );

    // ⚠ **The needle is the wrapped signature since task T-14-3, and the wrap is not a change
    // of the signature.** `Supersample` moved to `theme` with the rest of the engine, so
    // `render` had to become `pub(crate)` for the dialog to keep calling it — and those eleven
    // characters push the one-line form past the hundred columns `rustfmt` allows, so it lays
    // the parameters out one to a line. Name, arity, types and answer are the same six tokens
    // they were; what follows is the very text the file now carries.
    let reduction = function_body(
        &source,
        "fn render(\n        &self,\n        dc: HDC,\n        tile: &RECT,\n        \
         thickness: i32,\n        figure: impl FnOnce(Canvas),\n    ) -> bool {",
    );

    assert!(
        reduction.contains("self.reduce(width, height);"),
        "step 3 must be the arithmetic reduction"
    );
    assert_eq!(
        reduction.matches("StretchBlt(").count(),
        1,
        "the only stretching blit inside `render` is the enlargement of step 1"
    );
    assert!(
        reduction.contains("BitBlt("),
        "and the reduced tile goes over one to one, which cannot invent a colour"
    );

    // ⚠ The documented, silent trap: the pixels GDI drew may still be in the batch queue, and
    // a read before `GdiFlush` reads what was there before — with no error and no refusal.
    let flush_at = source
        .find("let _ = unsafe { GdiFlush() };")
        .expect("the reduction must flush the batch before it reads the section's pixels");
    let read_at = source
        .find(".read()")
        .expect("the reduction must read the section's pixels");

    assert!(
        flush_at < read_at,
        "the `GdiFlush` must stand *before* the first read of the DIB section's bits"
    );

    // The pixels are the section's own, kept from `CreateDIBSection` instead of being dropped.
    assert!(
        source.contains("bits: *mut u32,"),
        "the surface must keep the pointer to its own pixels"
    );
    assert!(
        source.contains("if bits.is_null() {"),
        "and a section that answered no pixels must be refused (NFR-13)"
    );

    // No length of an enlarged figure is a multiplication written at the figure: the factor
    // reaches a drawing through `Canvas`, which is the one place it is applied.
    for through_the_canvas in ["(x - self.origin_x) * SUPERSAMPLE", "pixels * SUPERSAMPLE"] {
        assert!(
            source.contains(through_the_canvas),
            "`{through_the_canvas}` — the factor must reach a drawing through `Canvas`"
        );
    }
}

/// **The assertion whose absence let the halo of T-11-23 live a whole stage** — the reduction
/// of a block is the average of that block, and an average never leaves the extremes of what it
/// averages.
///
/// The second half is the one that matters. Every colour this program paints is a palette
/// member or a blend of two, so no pixel of a smoothed figure may be darker than the darkest
/// thing behind it or brighter than the brightest thing on it. `HALFTONE` broke that and
/// nothing said so; a mean cannot break it, and this says so.
#[test]
fn the_reduction_of_a_block_is_its_average_and_never_leaves_its_extremes() {
    /// One 32-bit `BI_RGB` pixel out of three channels, in the order the section stores them.
    fn pixel(red: u32, green: u32, blue: u32) -> u32 {
        blue | (green << 8) | (red << 16)
    }

    /// The three channels of one pixel back out, red first.
    fn channels(pixel: u32) -> (u32, u32, u32) {
        ((pixel >> 16) & 0xFF, (pixel >> 8) & 0xFF, pixel & 0xFF)
    }

    // The two colours of the menu check mark: the ground it stands on and the ink it is drawn
    // in — the pair whose blend produced `0,2,9` and `255,255,255` under `HALFTONE`.
    let ground = pixel(32, 35, 41);
    let ink = pixel(228, 231, 234);

    // A block of nothing but ground comes back as ground, to the last bit: the flat interior of
    // every figure must survive the reduction untouched.
    let flat = [ground; 16];
    assert_eq!(
        theme::average_of_block(&flat),
        ground,
        "a block of one colour must reduce to that colour"
    );
    assert_eq!(theme::average_of_block(&[ink; 16]), ink);

    // Every coverage from none to all, against the average computed here from the definition.
    for covered in 0..=16u32 {
        let mut block = [ground; 16];

        for sample in block.iter_mut().take(covered as usize) {
            *sample = ink;
        }

        let (red, green, blue) = channels(theme::average_of_block(&block));

        for (name, got, low, high) in [
            ("red", red, 32, 228),
            ("green", green, 35, 231),
            ("blue", blue, 41, 234),
        ] {
            // The average, from its definition, rounded to the nearest — the box filter.
            let expected = (low * (16 - covered) + high * covered + 8) / 16;

            assert_eq!(
                got, expected,
                "{covered}/16 covered: the {name} channel must be the mean of the block, \
                 {expected}, and not {got}"
            );

            // ⚠ And the property the whole task turns on: the mean is inside its own terms.
            assert!(
                got >= low && got <= high,
                "{covered}/16 covered: the {name} channel came back {got}, outside the \
                 {low}…{high} the block was made of — that is the halo of T-11-23"
            );
        }
    }

    // The same, on the two extremes GDI can hand over at all: a block of black and white in any
    // proportion must stay black-and-white grey, never overshoot into a colour.
    for covered in 0..=16usize {
        let mut block = [pixel(0, 0, 0); 16];

        for sample in block.iter_mut().take(covered) {
            *sample = pixel(255, 255, 255);
        }

        let (red, green, blue) = channels(theme::average_of_block(&block));
        let expected = (255 * covered as u32 + 8) / 16;

        assert_eq!((red, green, blue), (expected, expected, expected));
        assert!(red <= 255, "no mean of bytes can exceed a byte");
    }

    // A block whose channels differ from each other, so that a filter which averaged the wrong
    // bytes together could not agree with this by accident.
    let mixed = [
        pixel(10, 20, 30),
        pixel(50, 60, 70),
        pixel(90, 100, 110),
        pixel(130, 140, 150),
    ];

    assert_eq!(
        channels(theme::average_of_block(&mixed)),
        (70, 80, 90),
        "each channel is averaged with itself and with no other"
    );

    // An empty block has no mean and answers zero rather than dividing by nothing. The product
    // never hands one over — the block is always [`theme::SUPERSAMPLE_BLOCK`] long — and the
    // function is public, so the degenerate case is answered rather than left to chance.
    assert_eq!(theme::average_of_block(&[]), 0);
}

/// **Criterion 11 of T-11-17** — every GDI object this task added is owned by a value with a
/// `Drop`, and no handle is used before it is examined.
#[test]
fn every_surface_picture_and_face_of_this_task_is_owned_and_freed_in_drop() {
    // T-14-3: `Supersample` moved to `theme`, `BackgroundCache` and `DialogFonts` stayed with
    // the window they belong to — the three owners are still three, across two files.
    let source = drawing_source();

    // The three owners this task added, beside the two the module already had.
    for owner in [
        "impl Drop for Supersample {",
        "impl Drop for BackgroundCache {",
        "impl Drop for DialogFonts {",
    ] {
        assert!(
            source.contains(owner),
            "`{owner}` — the owner must free its own"
        );
    }

    // The one DIB section of the module is the enlarged surface, and it is examined both ways
    // the call can decline — an `Err` and an invalid handle.
    let sections = product_lines_with("CreateDIBSection(");
    assert_eq!(
        sections.len(),
        1,
        "the enlarged surface is the one DIB section of the module: {sections:?}"
    );
    assert!(
        source.contains("let Ok(bitmap) = created else {"),
        "a refused DIB section must be examined (NFR-13)"
    );
    assert!(
        source.contains("if bitmap.is_invalid() {"),
        "and so must an invalid handle from a call that answered `Ok`"
    );

    // Every memory DC and every bitmap of the module is bound to a name the line after it is
    // made, which is where it is examined — never used inline.
    for made in ["CreateCompatibleDC(", "CreateCompatibleBitmap("] {
        for line in product_lines_with(made) {
            assert!(
                line.starts_with("let ") && line.contains("unsafe {"),
                "`{line}` — a handle must be bound and examined, never used inline"
            );
        }
    }
    assert!(
        source.contains("if dc.is_invalid() {"),
        "a refused memory DC must be examined (NFR-13)"
    );

    // The bitmap a memory DC was born with is kept and put back before ours is deleted — a
    // bitmap still selected into a DC cannot be freed.
    //
    // ⚠ Three since task T-15-1, not the two of T-11-17: `theme::PaintBuffer` — the off-screen
    // surface one owner-drawn element is painted into so that no half-finished state of it can
    // reach the screen — is the third owner of a bitmap, and it deselects exactly as the other
    // two do. The count is a census of owners and the number moves when a legitimate owner is
    // added; **the invariant this row states does not move**, and every one of the three has to
    // go on taking its bitmap out of the DC before it frees it.
    assert_eq!(
        product_lines_with("unsafe { SelectObject(self.dc, self.previous) };").len(),
        3,
        "all three owners of a bitmap must deselect before they delete"
    );

    // And the face that used to be made and deleted on every erase is an owned pair now.
    assert!(
        !source.contains("fn caption_font("),
        "`caption_font` made a face on every WM_ERASEBKGND — the faces are owned since T-11-17"
    );
    assert!(
        source.contains("fn new(hwnd: HWND, control: i32) -> Option<Self> {"),
        "`DialogFonts::new` must be the one place the two faces are made"
    );
}

/// **Criterion 12 of T-11-17** — the background is a picture built once, not eight panels on
/// every repaint, and the picture is rebuilt when the palette moves.
#[test]
fn the_background_is_a_cached_picture_rebuilt_on_a_palette_change() {
    let source = settings_module_source();

    // The erase handler no longer draws a figure of its own: it decides whether the picture in
    // hand still fits, hands it over, and paints straight into the DC only when there is none.
    let erase = function_body(&source, "unsafe fn on_erase_background(");

    assert!(
        !erase.contains("paint_rounded("),
        "the erase handler must not round off a figure — that is `paint_background`'s work now"
    );
    assert!(
        erase.contains("picture.show(dc)"),
        "an erase with a picture in hand must cost one blit"
    );
    assert!(
        erase.contains("BackgroundCache::build("),
        "and must build one when there is none"
    );

    // The picture is handed over with one `BitBlt` of the whole client area.
    assert!(
        source.contains("BitBlt("),
        "the picture must be handed over by a blit, not repainted"
    );

    // The three halves of «still the right picture»: the client size, the palette by
    // identity — `theme::resolve` answers `&'static`, and `refresh_palette` is the one place
    // the answer can change — and, since task Т-23-5, the colour the hotkey field's frame was
    // painted with, which is the one colour of this picture an armed capture moves.
    for line in [
        "self.width == width",
        "&& self.height == height",
        "&& std::ptr::eq(self.palette, palette)",
        "&& self.hotkey_frame == hotkey_frame",
    ] {
        assert!(
            source.contains(line),
            "the picture must be compared against `{line}`"
        );
    }

    // Rebuilt whole and never repainted in place: the assignment drops the stale picture.
    assert!(
        source.contains("with_state(hwnd, |state| state.background = built)"),
        "a stale picture must be replaced by the assignment, which drops it and its GDI objects"
    );
}

/// **Criterion 1 of T-11-17, the pure half** — the four corner tiles of a rounded rectangle are
/// a table, and they are what makes the smoothing affordable.
#[test]
fn the_four_corner_tiles_are_the_corners_and_nothing_between_them() {
    let area = RECT {
        left: 10,
        top: 20,
        right: 360,
        bottom: 144,
    };

    // Radius 4 plus a one-pixel frame — the panel of the mock-ups at 96 DPI.
    let tiles = theme::corner_tiles(&area, 5);

    let corners = [
        (10, 20, 15, 25),
        (355, 20, 360, 25),
        (10, 139, 15, 144),
        (355, 139, 360, 144),
    ];

    for (tile, expected) in tiles.iter().zip(corners) {
        assert_eq!(
            (tile.left, tile.top, tile.right, tile.bottom),
            expected,
            "the tiles are the four corners, in the order top-left, top-right, bottom-left, \
             bottom-right"
        );
    }

    // The cost of the decision, in the numbers the report states: four tiles of 5 × 5 enlarged
    // four times each way, against the whole 350 × 124 figure enlarged the same way.
    let enlarged_corners: i32 = tiles
        .iter()
        .map(|tile| {
            (tile.right - tile.left)
                * theme::SUPERSAMPLE
                * ((tile.bottom - tile.top) * theme::SUPERSAMPLE)
        })
        .sum();

    let enlarged_whole = (area.right - area.left)
        * theme::SUPERSAMPLE
        * ((area.bottom - area.top) * theme::SUPERSAMPLE);

    println!(
        "panel {}×{}: corners {enlarged_corners} px enlarged against {enlarged_whole} px whole \
         — {} times less",
        area.right - area.left,
        area.bottom - area.top,
        enlarged_whole / enlarged_corners
    );

    assert!(
        enlarged_whole / enlarged_corners > 100,
        "smoothing the corners alone must be orders of magnitude cheaper than the whole figure"
    );
}

/// **Criterion 1 of T-11-17, the pure half** — the tile a stroke is smoothed in holds the whole
/// stroke, pen and all.
#[test]
fn the_tile_of_a_stroke_holds_the_pen_around_every_point() {
    // The three points of a dialog check mark at 96 DPI, with its two-pixel pen.
    let points = [(3, 6), (5, 8), (9, 4)];
    let tile = theme::stroke_bounds(&points, 2);

    // Half the pen on each side, and one pixel more for the smoothed edge itself.
    assert_eq!(
        (tile.left, tile.top, tile.right, tile.bottom),
        (1, 2, 11, 10)
    );

    for (x, y) in points {
        assert!(
            x > tile.left && x < tile.right && y > tile.top && y < tile.bottom,
            "the point ({x}, {y}) must sit inside the tile with room for the pen"
        );
    }

    // A thicker pen widens the tile by half of itself on each side.
    let thick = theme::stroke_bounds(&points, 8);
    assert_eq!(
        (thick.left, thick.top, thick.right, thick.bottom),
        (-2, -1, 14, 13)
    );

    // No points at all is no tile — and the caller draws nothing either way.
    let nothing = theme::stroke_bounds(&[], 2);
    assert_eq!(
        (nothing.left, nothing.top, nothing.right, nothing.bottom),
        (0, 0, 0, 0)
    );
}

/// **The exception of T-11-17 is over — T-11-25, the source half.** The rounded square of a
/// layout-list tick is smoothed like every other figure of this dialog, because it is no longer
/// drawn into a cell whose mask makes a hole of one exact colour: it is drawn on the row.
///
/// What the deliberate exception was: `ImageList_AddMasked` makes a hole only of a pixel that
/// is *exactly* the key colour, so a smoothed corner — a blend of the key and the fill — would
/// have carried a coloured fringe onto every row, and the square had to keep the staircase
/// `stroke_rounded` leaves. The documented replacement the task proposed, an `ILC_COLOR32` list
/// without `ILC_MASK` carrying per-pixel alpha, was measured on this machine and does not blend
/// at all (the numbers are in the report of T-11-25), so the picture left the image list
/// altogether. The pixels are in
/// `the_square_of_the_tick_is_smoothed_and_carries_no_key_colour`.
#[test]
fn the_tick_of_the_layout_list_is_drawn_on_the_row_and_its_square_is_smoothed() {
    let source = settings_module_source();
    let body = function_body(&source, "fn draw_check_glyph(");

    assert!(
        body.contains("paint_rounded("),
        "the square is smoothed now — it is drawn on the row, where no mask can be harmed"
    );
    assert!(
        !body.contains("stroke_rounded("),
        "the aliased core is the fallback of `paint_rounded` and no longer the drawing itself"
    );
    assert!(
        body.contains("draw_check_mark(dc, glyph, ink, LIST_CHECK_MARK, dpi)"),
        "the mark keeps its own smoothing, stroked inside the fill"
    );

    // And the road the tick takes to the window: the item prepaint of the list draws the row,
    // and the row draws the tick. Nothing of the picture is left in the image list.
    assert!(
        function_body(&source, "unsafe fn on_notify(").contains("draw_cycle_row("),
        "the item prepaint must draw the row itself — ground, selection and tick"
    );
    assert!(
        function_body(&source, "pub fn draw_cycle_row(").contains("draw_check_glyph("),
        "and the row is where the tick is painted"
    );
}

// -----------------------------------------------------------------------------------------
// Настоящие пиксели: сглаженный угол против прямой стороны — задача T-11-17, критерий 1
// -----------------------------------------------------------------------------------------
//
// `paint_rounded` needs a DC and nothing else, so the whole of this runs in a memory bitmap:
// no window is created, no message loop is pumped, and the product is not started.

/// A 32-bit top-down DIB the size of a square, with its memory DC — the ground the figures
/// below are drawn on. Freed by [`Sheet`]'s own `Drop`, exactly as the module's own surfaces
/// are.
struct Sheet {
    dc: HDC,
    bitmap: HBITMAP,
    side: i32,
}

impl Sheet {
    fn new(side: i32) -> Self {
        // SAFETY: a memory DC over the screen, freed in `Drop`.
        let dc = unsafe { CreateCompatibleDC(None) };
        assert!(
            !dc.is_invalid(),
            "a memory DC must be available to the tests"
        );

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: side,
                biHeight: -side,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();

        // SAFETY: `info` lives on this frame and is read by the call; `bits` receives the
        // address of the pixels and is never followed here — the reading goes through `GetPixel`.
        let bitmap = unsafe {
            CreateDIBSection(
                Some(dc),
                &raw const info,
                DIB_RGB_COLORS,
                &raw mut bits,
                None,
                0,
            )
        }
        .expect("the DIB section of the test sheet");

        // SAFETY: both handles are live and ours.
        unsafe { SelectObject(dc, bitmap.into()) };

        Self { dc, bitmap, side }
    }

    /// Fills the whole sheet with one colour.
    fn clear(&self, colour: COLORREF) {
        let whole = RECT {
            left: 0,
            top: 0,
            right: self.side,
            bottom: self.side,
        };

        // SAFETY: the brush is made and freed here; `whole` lives on this frame.
        unsafe {
            let brush = CreateSolidBrush(colour);
            FillRect(self.dc, &whole, brush);
            let _ = DeleteObject(brush.into());
        }
    }

    /// The grey level of one pixel — every colour used here is a grey, so one channel says it.
    fn grey(&self, x: i32, y: i32) -> i32 {
        // SAFETY: `self.dc` holds this sheet's bitmap; the coordinates are inside it.
        (unsafe { GetPixel(self.dc, x, y) }.0 & 0xFF) as i32
    }

    /// All three channels of one pixel, red first — for the tests of task T-11-23, whose subject
    /// is a real palette pair and not a grey, and where a defect in one channel is the point.
    fn rgb(&self, x: i32, y: i32) -> (i32, i32, i32) {
        // SAFETY: `self.dc` holds this sheet's bitmap; the coordinates are inside it.
        let colour = unsafe { GetPixel(self.dc, x, y) }.0;

        (
            (colour & 0xFF) as i32,
            ((colour >> 8) & 0xFF) as i32,
            ((colour >> 16) & 0xFF) as i32,
        )
    }
}

impl Drop for Sheet {
    fn drop(&mut self) {
        // SAFETY: both came from the successful calls in `new` and are freed exactly once.
        unsafe {
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

/// **Criterion 1 of T-11-17, on real pixels** — a smoothed corner meets the aliased straight
/// edge beside it without a seam, and the frame keeps the width and the position it had.
///
/// This is the check the whole `stroke_shift` correction exists for. GDI centres a pen on its
/// path, and an enlarged odd pen straddles the block boundary of the reduction: without the
/// correction the one-pixel frame *inside the four corner tiles* comes back spread over two
/// pixels at roughly half strength, which is a step visible to the naked eye exactly where the
/// tile ends. With it, the frame inside a tile is the same pixel at the same strength as the
/// frame outside it, and only the curve itself is smoothed.
#[test]
fn a_smoothed_corner_meets_the_straight_edge_without_a_seam() {
    let ground = COLORREF(0x0020_2020);
    let fill_colour = COLORREF(0x0080_8080);
    let ink = COLORREF(0x00FF_FFFF);

    let sheet = Sheet::new(40);
    sheet.clear(ground);

    // SAFETY: a brush made and freed by this frame, live for every call that uses it.
    let fill = unsafe { CreateSolidBrush(fill_colour) };

    // The panel of the mock-ups in miniature: a rounded rectangle inset by two pixels, with the
    // corner radius of `CORNER_RADIUS` at 96 DPI and the one-pixel frame of `BORDER_THICKNESS`.
    let area = RECT {
        left: 2,
        top: 2,
        right: 38,
        bottom: 38,
    };

    theme::paint_rounded(sheet.dc, &area, 6, ink, fill, 96);

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(fill.into()) };

    // The tiles are `radius + thickness` a side, so the straight run of the left edge is
    // everything between them.
    let side = 6 + 1;
    let straight = (area.top + side)..(area.bottom - side);

    for y in straight.clone() {
        assert_eq!(
            sheet.grey(area.left, y),
            0xFF,
            "the straight left edge must be the untouched one-pixel frame at y = {y}"
        );
        assert_eq!(
            sheet.grey(area.left + 1, y),
            0x80,
            "and the fill must start the very next pixel at y = {y}"
        );
        assert_eq!(
            sheet.grey(area.left - 1, y),
            0x20,
            "and the ground must be untouched outside it at y = {y}"
        );
    }

    // And the rows the corner tiles own, where the arc has already come down onto the straight
    // edge: the same column, at the same strength. A seam would show here as a frame pixel at
    // half the ink and a second half-lit one beside it.
    for y in [area.top + side - 1, area.bottom - side] {
        let frame = sheet.grey(area.left, y);
        let outside = sheet.grey(area.left - 1, y);

        println!("tile row y = {y}: outside {outside:#04X}, frame {frame:#04X}");

        assert!(
            frame > 0xE0,
            "the frame inside the corner tile must be the same pixel at the same strength as \
             the frame outside it — got {frame:#04X} at y = {y}, which is the seam \
             `stroke_shift` exists to close"
        );
        assert!(
            outside < 0x40,
            "and nothing of the frame may have spilled a pixel outwards — got {outside:#04X} \
             at y = {y}"
        );
    }

    // The curve itself *is* smoothed: somewhere along the arc there are pixels that are neither
    // ground, nor ink, nor fill — the partial coverage that has no place in an aliased figure.
    let mut blended = 0;

    for y in area.top..(area.top + side) {
        for x in area.left..(area.left + side) {
            let value = sheet.grey(x, y);

            if value != 0x20 && value != 0x80 && value != 0xFF {
                blended += 1;
            }
        }
    }

    println!(
        "blended pixels in the top-left corner tile: {blended} of {}",
        side * side
    );

    assert!(
        blended >= 8,
        "a smoothed corner must carry partial coverage — {blended} blended pixels is a \
         staircase, not a curve"
    );
}

/// **Criterion 1 of T-11-17, the pure half of the correction** — the half-pixel an odd pen
/// needs, and nothing for an even one.
#[test]
fn only_an_odd_pen_takes_the_half_pixel_of_the_enlarged_path() {
    // One pixel at 96 DPI, three at 250 % — the frame is odd at most scales, which is why the
    // correction is the difference between a smoothed corner and a seam.
    assert_eq!(theme::stroke_shift(1), theme::SUPERSAMPLE / 2);
    assert_eq!(theme::stroke_shift(3), theme::SUPERSAMPLE / 2);
    assert_eq!(theme::stroke_shift(5), theme::SUPERSAMPLE / 2);

    // An even pen has no centre pixel to lose: enlarged, it lands on the block boundary of its
    // own accord.
    assert_eq!(theme::stroke_shift(0), 0);
    assert_eq!(theme::stroke_shift(2), 0);
    assert_eq!(theme::stroke_shift(4), 0);

    // Half of one pixel of the window, in the pixels of the enlarged surface.
    assert_eq!(theme::SUPERSAMPLE / 2, 2);
}

/// **The defect of T-11-23 on real product pixels** — nothing a smoothed figure paints is
/// darker than the darkest colour it was drawn from or brighter than the brightest.
///
/// The pure half is
/// `the_reduction_of_a_block_is_its_average_and_never_leaves_its_extremes`; this is the same
/// sentence said about the two figures that actually go through [`settings::Supersample`] — a
/// glyph enlarged whole (the check mark) and a rounded rectangle enlarged at its four corners
/// (every panel, field, button and selection stripe of the dialog).
///
/// The numbers this replaces: with the `HALFTONE` reduction of T-11-17 the very same two
/// drawings answered **39** impossible pixels of 576 for the mark, down to `9,12,18` and up to
/// `255,255,255`, and **58** of 1 600 for the rectangle, down to `2,5,12` — which is the halo
/// the user saw, measured without a window and without starting the product.
#[test]
fn no_smoothed_figure_paints_a_colour_it_was_not_drawn_from() {
    // The three colours of «Графит» these two figures are made of, and the band they close.
    let ground = COLORREF(0x0029_2320); // window_bg 32,35,41
    let fill_colour = COLORREF(0x0032_2B27); // panel_bg  39,43,50
    let ink = COLORREF(0x00EA_E7E4); // text      228,231,234

    let darkest = (32, 35, 41);
    let brightest = (228, 231, 234);

    /// The pixels of `sheet` outside the band `darkest … brightest`, and how many inside it are
    /// none of the `solid` colours the figure was painted from — that is, partially covered.
    fn survey(
        sheet: &Sheet,
        side: i32,
        darkest: (i32, i32, i32),
        brightest: (i32, i32, i32),
        solid: &[i32],
    ) -> (Vec<String>, i32) {
        let mut impossible = Vec::new();
        let mut blended = 0;

        for y in 0..side {
            for x in 0..side {
                let (red, green, blue) = sheet.rgb(x, y);

                if red < darkest.0
                    || red > brightest.0
                    || green < darkest.1
                    || green > brightest.1
                    || blue < darkest.2
                    || blue > brightest.2
                {
                    impossible.push(format!("({x},{y}) = {red},{green},{blue}"));
                }

                // Partial coverage — the thing the smoothing exists to produce, and the reason
                // «ни одного невозможного пикселя» cannot be passed by not smoothing at all.
                if !solid.contains(&red) {
                    blended += 1;
                }
            }
        }

        (impossible, blended)
    }

    // 1. A glyph enlarged whole: the check mark of a check box, stroked at four times its size
    //    and averaged back down in one piece.
    {
        let sheet = Sheet::new(24);
        sheet.clear(ground);

        let glyph = RECT {
            left: 4,
            top: 4,
            right: 20,
            bottom: 20,
        };

        theme::draw_check_mark(sheet.dc, &glyph, ink, settings::GLYPH_CHECK_MARK, 96);

        let (impossible, blended) = survey(&sheet, 24, darkest, brightest, &[32, 228]);

        println!(
            "галочка: {blended} смешанных пикселей, {} невозможных",
            impossible.len()
        );

        assert!(
            impossible.is_empty(),
            "a check mark drawn in {brightest:?} on {darkest:?} may paint nothing outside the \
             two: {impossible:?}"
        );
        assert!(
            blended >= 8,
            "and it must still be smoothed — {blended} partially covered pixels is a staircase"
        );
    }

    // 2. A rounded rectangle, smoothed at its four corners only — the panel of the dialog, and
    //    the figure whose accent-button corners the controller measured at `255,255,255`.
    {
        let sheet = Sheet::new(40);
        sheet.clear(ground);

        let area = RECT {
            left: 2,
            top: 2,
            right: 38,
            bottom: 38,
        };

        // SAFETY: the brush is made here, used only by the call below and freed here.
        let fill = unsafe { CreateSolidBrush(fill_colour) };

        theme::paint_rounded(sheet.dc, &area, 6, ink, fill, 96);

        // SAFETY: created above, handed to nobody, freed exactly once.
        let _ = unsafe { DeleteObject(fill.into()) };

        let (impossible, blended) = survey(&sheet, 40, darkest, brightest, &[32, 39, 228]);

        println!(
            "панель: {blended} смешанных пикселей, {} невозможных",
            impossible.len()
        );

        assert!(
            impossible.is_empty(),
            "a rounded rectangle of {fill_colour:?} framed in {brightest:?} on {darkest:?} may \
             paint nothing outside them: {impossible:?}"
        );
        assert!(
            blended >= 8,
            "and its corners must still be smoothed — {blended} partially covered pixels is a \
             staircase"
        );
    }
}

// -----------------------------------------------------------------------------------------
// Настоящие пиксели: отказ сглаживания не оставляет угол незакрашенным — задача T-11-24
// -----------------------------------------------------------------------------------------

/// **Criterion 10 of T-11-24, on real pixels** — a rounded rectangle whose smoothing is refused
/// comes out with four aliased corners and not with four holes.
///
/// # The lever
///
/// Nothing here exhausts GDI, waits for anything or depends on the state of the machine. A
/// corner tile is `radius + thickness` a side, so a radius of [`theme::SUPERSAMPLE_MAX_SIDE`]
/// with the one-pixel frame of 96 DPI asks for a tile of 65 — one past the ceiling
/// `Supersample::for_tile` holds — and the surface is refused before a single call to GDI is
/// made. The figure is large enough that `paint_rounded` offers it the smoothing first: the
/// refusal is of the surface and not of the whole technique.
///
/// # What it holds
///
/// The four corner tiles are held **out of the clip** while the straight part is drawn — that
/// is what lets a smoothed corner blend into the true ground instead of into an aliased corner —
/// so a corner the smoothing does not paint is not a rougher corner, it is bare window
/// background. Until this task the refusal was dropped and the figure came out with the ground
/// showing through all four of them.
#[test]
fn a_refused_smoothing_paints_an_aliased_corner_and_never_leaves_a_hole() {
    let ground = COLORREF(0x0020_2020);
    let fill_colour = COLORREF(0x0080_8080);
    let ink = COLORREF(0x00FF_FFFF);

    // The frame `paint_rounded` makes at 96 DPI, and the corner tile that follows from it.
    let thickness = 1;
    let radius = theme::SUPERSAMPLE_MAX_SIDE;
    let side = radius + thickness;

    let sheet = Sheet::new(200);
    sheet.clear(ground);

    let area = RECT {
        left: 4,
        top: 4,
        right: 196,
        bottom: 196,
    };

    // Both halves of the premise, so that a future ceiling cannot quietly turn this test into a
    // test of the ordinary path: the tile is past the ceiling, and the figure is wide enough to
    // be offered the smoothing that is then refused.
    assert!(
        side > theme::SUPERSAMPLE_MAX_SIDE,
        "the lever of this test is a corner tile past the ceiling — {side} against {}",
        theme::SUPERSAMPLE_MAX_SIDE
    );
    assert!(
        side * 2 <= (area.right - area.left).min(area.bottom - area.top),
        "and the figure must be big enough to hold four tiles, or it is drawn aliased whole"
    );

    // SAFETY: the brush is made here, used only by the call below and freed here.
    let fill = unsafe { CreateSolidBrush(fill_colour) };

    theme::paint_rounded(sheet.dc, &area, radius, ink, fill, 96);

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(fill.into()) };

    // The centre of each corner's arc, in the order `corner_tiles` answers them: top-left,
    // top-right, bottom-left, bottom-right. The figure spans `left … right - 1`, which is why
    // the two far centres are counted off `right - 1` and `bottom - 1`.
    let tiles = theme::corner_tiles(&area, side);
    let centres = [
        (area.left + radius, area.top + radius),
        (area.right - 1 - radius, area.top + radius),
        (area.left + radius, area.bottom - 1 - radius),
        (area.right - 1 - radius, area.bottom - 1 - radius),
    ];

    // Three pixels inside the arc, so that no rounding of the ellipse GDI actually walks can put
    // one of these pixels outside the figure: what is measured here is the hole, not the
    // rasteriser.
    let inside = (radius - 3) * (radius - 3);

    let mut holes = 0;
    let mut first_hole = None;
    let mut blended = 0;
    let mut framed = [0; 4];

    for (corner, (tile, (centre_x, centre_y))) in tiles.iter().zip(centres).enumerate() {
        for y in tile.top..tile.bottom {
            for x in tile.left..tile.right {
                let value = sheet.grey(x, y);

                // An aliased corner is made of the three colours the figure is drawn from and of
                // nothing in between: a partially covered pixel here would mean the tile was
                // smoothed after all, and the refusal this test is about never happened.
                if value != 0x20 && value != 0x80 && value != 0xFF {
                    blended += 1;
                }

                // The arc of the frame, which is as much of the corner as the fill is.
                if value == 0xFF {
                    framed[corner] += 1;
                }

                let (from_x, from_y) = (x - centre_x, y - centre_y);

                // Well inside the arc — the fill of the figure, whatever the smoothing did.
                if from_x * from_x + from_y * from_y <= inside && value == 0x20 {
                    holes += 1;
                    first_hole.get_or_insert((x, y));
                }
            }
        }
    }

    println!(
        "отказ сглаживания: {holes} пикселей земли внутри фигуры, {blended} смешанных, \
         чернил по углам {framed:?}"
    );

    assert_eq!(
        holes, 0,
        "a refused smoothing must leave the aliased corner `RoundRect` would have drawn and not \
         the ground — the first hole of the four corners is at {first_hole:?}"
    );
    assert_eq!(
        blended, 0,
        "and what it leaves must be exactly that staircase — a partially covered pixel means \
         the tile went through the smoothing this test asked to be refused"
    );
    assert!(
        framed.iter().all(|count| *count >= radius),
        "and the arc must run through every one of the four tiles — the frame is as much of the \
         corner as the fill is: {framed:?} pixels of ink against a quarter circle of {radius}"
    );
}

/// **The second path of T-11-24** — the answer of `Supersample::render` decides something, and
/// the two refusals of the smoothing meet in one branch.
///
/// The test above drives the refusal of the **surface**, which is deterministic: a tile past the
/// ceiling. The refusal of a single **tile** is a refused `StretchBlt` or a refused `BitBlt` —
/// GDI out of what a blit takes — and that cannot be arranged without exhausting the machine.
/// So what is held here is that it cannot end anywhere else: the answer is bound to a name, the
/// name decides, and the fallback both refusals reach is the one aliased corner.
#[test]
fn both_refusals_of_the_smoothing_end_in_the_same_aliased_corner() {
    // T-14-3: the drawing is two files now — see `drawing_source`.
    let source = drawing_source();

    // ⚠ `paint_corner_tiles` и не `paint_rounded`: тело переехало задачей T-12-5, которой
    // понадобилось назвать те же четыре угла отдельно от прямой части фигуры
    // (`paint_rounded_corners` — заплатка поверх плоской заливки списка). Проверяемое — то же,
    // и ниже проверено, что `paint_rounded` по-прежнему этим телом и кончается: иначе
    // утверждение стало бы верным о функции, до которой никто не доходит.
    let body = function_body(&source, "fn paint_corner_tiles(");

    assert!(
        body.contains("let painted = surface"),
        "NFR-13: the answer of `Supersample::render` must be bound and not dropped"
    );
    assert!(
        body.contains("if !painted {") && body.contains("stroke_corner_aliased("),
        "and a tile that was not painted must fall back to the aliased corner"
    );
    assert!(
        !body.contains("let Some(surface) = Supersample::for_tile"),
        "a refused surface may no longer leave before the loop: the figure has been drawn with \
         four holes in it by then, and that `return` is what left all four showing"
    );

    // Обе дороги к этому телу — и целая фигура, и заплатка из четырёх углов.
    for (caller, why) in [
        (
            "pub fn paint_rounded(",
            "the whole figure must still end in the four smoothed corners",
        ),
        (
            "pub fn paint_rounded_corners(",
            "and the corner patch of T-12-5 must reach them through the same body, not a copy",
        ),
    ] {
        assert!(
            function_body(&source, caller).contains("paint_corner_tiles("),
            "{why}"
        );
    }

    // And the corner the fallback paints is the aliased core itself — the same figure the whole
    // rectangle is drawn from, named again inside the one tile.
    assert!(
        function_body(&source, "fn stroke_corner_aliased(").contains("stroke_rounded("),
        "the fallback must draw the figure `RoundRect` would have drawn there"
    );
}

// -----------------------------------------------------------------------------------------
// Скруглённая заливка интерьера списков — находка R-06, задача T-12-5
// -----------------------------------------------------------------------------------------

/// **T-12-5, геометрия** — рамка списка, названная изнутри контрола, это та же рамка, что
/// `paint_background` рисует снаружи, и обе стоят на одной арифметике.
#[test]
fn the_frame_of_a_list_is_the_same_rectangle_from_either_side() {
    // При 96 DPI: `scaled(BORDER_THICKNESS = 1)` = 1, `scaled(LIST_FIRST_ROW_TOP = 3)` = 2 —
    // мок-ап даёт 3, но в окно тройка приходит через `scaled`, и это ровно те числа, которыми
    // фон рисует рамку с задачи T-11-16.
    assert_eq!(
        theme::list_frame_air(96),
        (2, 1),
        "the air above a list and the thickness around it, at 96 DPI"
    );

    // И воздух никогда не тоньше рамки: `max(border)` в теле — не украшение.
    for dpi in [96, 120, 144, 192] {
        let (top, border) = theme::list_frame_air(dpi);

        assert!(
            top >= border && border >= 1,
            "at {dpi} DPI the air {top} must be at least the thickness {border}, and the \
             thickness at least one pixel"
        );
    }

    // Коробка — клиентский прямоугольник, раздвинутый ровно на эти величины.
    let client = RECT {
        left: 0,
        top: 0,
        right: 210,
        bottom: 81,
    };
    let box_at_96 = theme::list_frame_box(&client, 96);

    assert_eq!(
        (
            box_at_96.left,
            box_at_96.top,
            box_at_96.right,
            box_at_96.bottom
        ),
        (-1, -2, 211, 82),
        "the frame of a list starts at negative coordinates of its own client area: it stands \
         outside the control on all four sides, and only its corner arcs reach back in"
    );

    for dpi in [96, 120, 144, 192] {
        let (top, border) = theme::list_frame_air(dpi);
        let frame = theme::list_frame_box(&client, dpi);

        assert_eq!(
            (frame.left, frame.top, frame.right, frame.bottom),
            (
                client.left - border,
                client.top - top,
                client.right + border,
                client.bottom + border
            ),
            "at {dpi} DPI"
        );
    }
}

/// **T-12-5, критерий 8** — заплатка скругляет интерьер, который контрол залил прямоугольником,
/// и **между углами не трогает ничего**.
#[test]
fn the_corner_patch_rounds_a_flat_interior_and_leaves_the_middle_alone() {
    let ground = COLORREF(0x0020_2020);
    let fill_colour = COLORREF(0x0080_8080);
    let ink = COLORREF(0x00FF_FFFF);

    let sheet = Sheet::new(40);
    sheet.clear(ground);

    // SAFETY: brushes made here and freed below, live for every call that uses them.
    let fill = unsafe { CreateSolidBrush(fill_colour) };
    // SAFETY: as above.
    let ground_brush = unsafe { CreateSolidBrush(ground) };

    // The client area of a list, and the flat rectangle a list control fills it with — the
    // square cut of R-06 in miniature.
    let client = RECT {
        left: 5,
        top: 5,
        right: 35,
        bottom: 35,
    };

    // SAFETY: `sheet.dc` holds this test's bitmap and `client` lives on this frame.
    unsafe { FillRect(sheet.dc, &client, fill) };

    let corner = client.bottom - 1;

    assert_eq!(
        sheet.grey(client.left, corner),
        0x80,
        "the flat fill must reach the very corner before the patch — otherwise this test is \
         proving nothing"
    );

    let area = theme::list_frame_box(&client, 96);

    theme::paint_rounded_corners(
        sheet.dc,
        &area,
        &client,
        6,
        theme::CornerColors {
            ground: ground_brush,
            outline: ink,
            fill,
        },
        96,
    );

    // SAFETY: created above, handed to nobody, freed exactly once each.
    let _ = unsafe { DeleteObject(fill.into()) };
    // SAFETY: as above.
    let _ = unsafe { DeleteObject(ground_brush.into()) };

    // 1. The corner is cut away: the pixel that carried the square is no longer the fill.
    assert_ne!(
        sheet.grey(client.left, corner),
        0x80,
        "the bottom-left corner of the interior must not be square any more"
    );

    // 2. **Between** the corners nothing moved. The tiles are `radius + thickness` a side, so
    // everything outside them along each edge is the straight run the control drew.
    let side = 6 + 1;

    for y in (client.top + side)..(client.bottom - side) {
        assert_eq!(
            sheet.grey(client.left, y),
            0x80,
            "the straight left edge of the interior at y = {y}"
        );
        assert_eq!(
            sheet.grey(client.right - 1, y),
            0x80,
            "the straight right edge of the interior at y = {y}"
        );
    }

    for x in (client.left + side)..(client.right - side) {
        assert_eq!(
            sheet.grey(x, client.top),
            0x80,
            "the straight top edge of the interior at x = {x}"
        );
        assert_eq!(
            sheet.grey(x, client.bottom - 1),
            0x80,
            "the straight bottom edge of the interior at x = {x}"
        );
    }

    // 3. Every one of the four corners carries at least two halftones — a colour that is
    // neither the ground, nor the fill, nor the ink. That is the whole of criterion 8, and it
    // is what tells a smoothed arc from a staircase.
    for (name, x0, y0) in [
        ("top-left", client.left, client.top),
        ("top-right", client.right - side, client.top),
        ("bottom-left", client.left, client.bottom - side),
        ("bottom-right", client.right - side, client.bottom - side),
    ] {
        let mut halftones = 0;

        for y in y0..(y0 + side) {
            for x in x0..(x0 + side) {
                let grey = sheet.grey(x, y);

                if grey != 0x20 && grey != 0x80 && grey != 0xFF {
                    halftones += 1;
                }
            }
        }

        println!("угол {name}: {halftones} полутонов");

        assert!(
            halftones >= 2,
            "the {name} corner must be smoothed, not cut: {halftones} halftones"
        );
    }
}

/// **T-12-5, случай полосы прокрутки** — угол, целиком выпавший из `bounds`, не рисуется
/// где-то ещё вместо этого.
///
/// Это регрессия, пойманная прибором на стенде и почти пропущенная: список с числом строк
/// больше видимого поднимает **неклиентскую** полосу прокрутки, клиентская область делается
/// на 17 px у́же окна, и заплатка, посчитанная от клиентской области, положила безупречно
/// скруглённый угол на 17 px внутрь списка. `bounds` — это разрешение, а не фигура; а
/// вывернутый прямоугольник, который даёт обрезка выпавшего угла, `FillRect` **нормализует**,
/// а не отвергает — то есть «ничего» превращается в «полосу не там».
#[test]
fn a_corner_outside_the_bounds_paints_nothing_at_all() {
    let ground = COLORREF(0x0020_2020);
    let fill_colour = COLORREF(0x0080_8080);
    let bar = COLORREF(0x00C0_C0C0);
    let ink = COLORREF(0x00FF_FFFF);

    let sheet = Sheet::new(40);
    sheet.clear(ground);

    // SAFETY: brushes made here and freed below.
    let fill = unsafe { CreateSolidBrush(fill_colour) };
    // SAFETY: as above.
    let ground_brush = unsafe { CreateSolidBrush(ground) };
    // SAFETY: as above.
    let bar_brush = unsafe { CreateSolidBrush(bar) };

    // The window of the control, and the client area a vertical scroll bar has left of it.
    let window = RECT {
        left: 5,
        top: 5,
        right: 35,
        bottom: 35,
    };
    let client = RECT {
        right: window.right - 12,
        ..window
    };
    let strip = RECT {
        left: client.right,
        ..window
    };

    // SAFETY: `sheet.dc` holds this test's bitmap; both rectangles live on this frame.
    unsafe { FillRect(sheet.dc, &client, fill) };
    // SAFETY: as above.
    unsafe { FillRect(sheet.dc, &strip, bar_brush) };

    // The figure is the frame of the **window**, which is where `paint_background` hangs it —
    // its right-hand corners therefore fall inside the strip the scroll bar owns.
    let area = theme::list_frame_box(&window, 96);

    theme::paint_rounded_corners(
        sheet.dc,
        &area,
        &client,
        6,
        theme::CornerColors {
            ground: ground_brush,
            outline: ink,
            fill,
        },
        96,
    );

    // SAFETY: created above, handed to nobody, freed exactly once each.
    let _ = unsafe { DeleteObject(fill.into()) };
    // SAFETY: as above.
    let _ = unsafe { DeleteObject(ground_brush.into()) };
    // SAFETY: as above.
    let _ = unsafe { DeleteObject(bar_brush.into()) };

    for y in window.top..window.bottom {
        for x in strip.left..strip.right {
            assert_eq!(
                sheet.grey(x, y),
                0xC0,
                "the strip the scroll bar owns must come out of the patch untouched, and ({x}, \
                 {y}) did not"
            );
        }
    }

    // And the left-hand corners, which are inside `bounds`, were still drawn: a patch that
    // painted nothing anywhere would pass the loop above for the wrong reason.
    assert_ne!(
        sheet.grey(client.left, client.bottom - 1),
        0x80,
        "the corners that are inside the bounds must still be rounded"
    );
}

/// **T-12-5, критерии «не сломать» и «подкласс снимается»** — подкласс списков это одна пара,
/// и его процедура ничего не перехватывает.
///
/// Второе — половина доказательства того, что выделение (T-11-25), галочки (T-11-25-2),
/// прокрутка, втяжки и высота строк не задеты: они не задеты **по построению**, раз каждое
/// сообщение доходит до собственной процедуры контрола неизменным и её ответ и возвращается.
#[test]
fn the_list_subclass_is_one_pair_and_intercepts_nothing() {
    let source = settings_module_source();

    // The pair, by its call sites. The install is matched with its own indentation, because
    // «unsubclass_lists» contains «subclass_lists» as a substring.
    assert_eq!(
        source.matches("\n    subclass_lists(hwnd);").count(),
        1,
        "the subclass must be installed in exactly one place"
    );
    assert_eq!(
        source.matches("unsubclass_lists(hwnd);").count(),
        1,
        "the subclass must be removed in exactly one place"
    );

    // The same procedure and the same identifier on both sides — one install, two removals
    // (the pair's own, and the `WM_NCDESTROY` safety net inside the procedure).
    assert_eq!(
        source
            .matches("SetWindowSubclass(list, Some(list_proc), LIST_SUBCLASS_ID, 0)")
            .count(),
        1,
        "exactly one SetWindowSubclass, with the procedure and the identifier of the pair"
    );
    assert_eq!(
        source
            .matches("RemoveWindowSubclass(list, Some(list_proc), LIST_SUBCLASS_ID)")
            .count(),
        2,
        "the removal of the pair and the WM_NCDESTROY safety net, and nothing else"
    );

    // And the two subclasses of this file do not share an identifier.
    assert!(
        source.contains("const COMBO_SUBCLASS_ID: usize = 1;")
            && source.contains("const LIST_SUBCLASS_ID: usize = 2;"),
        "the combo boxes and the lists must be told apart by their subclass identifiers"
    );

    let body = function_body(&source, "unsafe extern \"system\" fn list_proc(");

    assert!(
        body.contains("let answer = unsafe { DefSubclassProc(list, message, wparam, lparam) };")
            && body.trim_end().ends_with("answer"),
        "every message must reach the control's own procedure unchanged, and its answer must be \
         the answer given back — that is what leaves the selection, the ticks, the scrolling and \
         the row metrics of the two lists untouched by construction"
    );
    assert!(
        !body.contains("return "),
        "and nothing may be answered before it: a `return` here would be an interception"
    );

    // NFR-13: the DC of the patch is released on every path out of the function that took one.
    let patch = function_body(&source, "unsafe fn patch_list_corners(");

    assert_eq!(
        patch.matches("GetDC(Some(list))").count(),
        1,
        "exactly one DC is taken"
    );
    assert_eq!(
        patch.matches("ReleaseDC(Some(list), dc)").count(),
        1,
        "and exactly one is released"
    );

    // Code only: the comments between the two calls talk *about* `return` and `?`, and a sweep
    // that counted those would be reading the explanation instead of the program.
    let held: String = patch
        .split_once("GetDC(Some(list))")
        .expect("the patch must take a DC")
        .1
        .split_once("ReleaseDC(Some(list), dc)")
        .expect("and release it")
        .0
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    // The one `return` allowed between the two is the refusal of `GetDC` itself, where no DC
    // was taken and there is nothing to release. Every other way out — a second `return`, a `?`
    // — would leak, and a leaked DC gives neither an error nor a red test, which is why this is
    // read out of the source and not hoped for.
    assert!(
        held.contains("if dc.is_invalid() {"),
        "the refusal of `GetDC` must be examined (NFR-13)"
    );
    assert_eq!(
        held.matches("return").count(),
        1,
        "and it must be the only way out of the function while a DC is in hand"
    );
    assert!(
        !held.contains('?'),
        "and no `?` either — the same leak wearing a shorter spelling"
    );
}

// -----------------------------------------------------------------------------------------
// Настоящие пиксели: подпись собственной отрисовкой — задача T-11-18, критерии 11 и 12
// -----------------------------------------------------------------------------------------
//
// `theme::paint_label` needs a DC, a rectangle, a brush and an ink — no window, no message
// and no state — so the whole of this section runs in the memory bitmap of [`Sheet`] above:
// no dialog is created, no message loop is pumped, and **the product is not started**.
//
// The face handed in is `None` throughout, which leaves whatever font the DC holds. That is
// deliberate: the subject here is the *shape* of the drawing — what is filled, what is
// written and where the lines fall — and the face is the subject of the T-11-17 tests above.

/// The grey of the ground these labels are painted on, and of their ink. Both are greys
/// because [`Sheet::grey`] reads one channel, and they are far apart so that a partially
/// covered pixel still lands unambiguously on one side of `INKED`.
const LABEL_GROUND: COLORREF = COLORREF(0x0040_4040);
const LABEL_INK: COLORREF = COLORREF(0x00FF_FFFF);
/// The grey of the sheet outside the label's rectangle — a colour the drawing must never
/// reach, and a colour the ground is not, so «the fill stayed inside» is decidable.
const LABEL_OUTSIDE: COLORREF = COLORREF(0x0000_0000);
/// Anything brighter than this is ink and not ground.
const INKED: i32 = 0x80;

impl Sheet {
    /// The lowest row of the sheet that carries any ink, or `None` for a sheet with none.
    fn lowest_inked_row(&self, area: &RECT) -> Option<i32> {
        (area.top..area.bottom).rfind(|y| (area.left..area.right).any(|x| self.grey(x, *y) > INKED))
    }
}

/// Paints one caption into `area` of `sheet` through the product's own [`theme::paint_label`],
/// with a brush this function makes and frees.
fn paint_label_on(sheet: &Sheet, area: RECT, caption: &str) -> isize {
    let mut text: Vec<u16> = caption.encode_utf16().collect();

    // SAFETY: the brush is made here, used only by the call below and freed here.
    let ground = unsafe { CreateSolidBrush(LABEL_GROUND) };

    // SAFETY: `sheet.dc` holds this sheet's bitmap, `text` and `area` are live locals of this
    // frame, and `ground` is live for the whole call. `None` leaves the DC's own font in place.
    // Task Т-30-2 gave `paint_label` the reading order of its run. `Native` is what every label
    // of this program asks for except the journal path, and it is what this measurement is of.
    let answer = unsafe {
        theme::paint_label(
            sheet.dc,
            area,
            &mut text,
            theme::LabelStyle {
                ground,
                ink: LABEL_INK,
                face: None,
                pitch: None,
                reading: theme::Reading::Native,
            },
        )
    };

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(ground.into()) };

    answer
}

/// **Criterion 12 of T-11-18, on real pixels** — an empty label draws its ground and not one
/// pixel more, and it takes the words of the label that stood there before it with it.
///
/// The second half is the one that matters and the one a «returns early on an empty string»
/// implementation fails. `IDC_HOTKEY_NOTE` and `IDC_LAYOUT_NOTE` are **written empty while the
/// window is up** — `show_hotkey` clears the note after a capture, `fill_layouts` clears the
/// layout note the moment both combo boxes name layouts the session has — and an owner-drawn
/// static is answerable for the whole of its rectangle. A paint that returns before the
/// `FillRect` leaves the previous sentence on the screen, unerasable, for as long as the
/// dialog is open.
///
/// So the test paints a sentence first and then empties the label, which is exactly the
/// sequence the two notes go through, and reads the pixels back.
#[test]
fn an_empty_label_erases_the_words_that_stood_there_and_draws_nothing_else() {
    let sheet = Sheet::new(80);
    sheet.clear(LABEL_OUTSIDE);

    let area = RECT {
        left: 8,
        top: 8,
        right: 72,
        bottom: 40,
    };

    // The note as it stands while it has something to say.
    assert_eq!(
        paint_label_on(&sheet, area, "no such layout"),
        1,
        "a label with text answers «drawn»"
    );

    let inked = (area.top..area.bottom)
        .flat_map(|y| (area.left..area.right).map(move |x| (x, y)))
        .filter(|(x, y)| sheet.grey(*x, *y) > INKED)
        .count();

    println!("with text: {inked} inked pixels inside the label");
    assert!(
        inked > 0,
        "the sheet has to carry the sentence before the emptying can be shown to remove it"
    );

    // And the same label after the note was cleared — the very `String::new()` `fill_layouts`
    // writes.
    assert_eq!(
        paint_label_on(&sheet, area, ""),
        1,
        "an empty label answers «drawn» too: the ground is the whole of it, and answering \
         «not drawn» would hand the rectangle back to a system drawing that no longer exists"
    );

    for y in area.top..area.bottom {
        for x in area.left..area.right {
            assert_eq!(
                sheet.grey(x, y),
                0x40,
                "({x}, {y}) inside the emptied label is not the ground — the words of the \
                 previous note are still there"
            );
        }
    }

    // And nothing left the rectangle: the row and the column just outside it are untouched.
    for x in 0..80 {
        assert_eq!(
            sheet.grey(x, area.top - 1),
            0x00,
            "the row above the label was painted into at x = {x}"
        );
        assert_eq!(
            sheet.grey(x, area.bottom),
            0x00,
            "the row below the label was painted into at x = {x}"
        );
    }
    for y in 0..80 {
        assert_eq!(
            sheet.grey(area.left - 1, y),
            0x00,
            "the column left of the label was painted into at y = {y}"
        );
        assert_eq!(
            sheet.grey(area.right, y),
            0x00,
            "the column right of the label was painted into at y = {y}"
        );
    }
}

/// **Criterion 11 of T-11-18, on real pixels** — a caption too long for one line really does
/// wrap onto the next one.
///
/// The flag test above says the format carries `DT_WORDBREAK`; this one says the flag does
/// what the two-line labels need it to do, without knowing anything about the font in the DC:
/// a short caption and a long one are painted into the very same rectangle, and the long one's
/// last inked row has to sit **below** the short one's. One line cannot reach there.
#[test]
fn a_caption_too_long_for_one_line_wraps_onto_the_next() {
    let sheet = Sheet::new(120);

    let area = RECT {
        left: 4,
        top: 4,
        right: 60,
        bottom: 116,
    };

    sheet.clear(LABEL_OUTSIDE);
    paint_label_on(&sheet, area, "one");
    let one_line = sheet
        .lowest_inked_row(&area)
        .expect("the short caption must leave ink on the sheet");

    sheet.clear(LABEL_OUTSIDE);
    paint_label_on(
        &sheet,
        area,
        "one two three four five six seven eight nine ten",
    );
    let wrapped = sheet
        .lowest_inked_row(&area)
        .expect("the long caption must leave ink on the sheet");

    println!("one line ends at row {one_line}, the wrapped caption at row {wrapped}");

    assert!(
        wrapped > one_line,
        "the long caption ended on the same line as the short one ({wrapped} against \
         {one_line}) — DT_WORDBREAK is not reaching the drawing, and the two-line labels of \
         the template lose their tails"
    );
}

// =========================================================================================
// FR-92а — остаток текста: пять полей ввода и строки списка раскладок. Task T-11-20.
// =========================================================================================
//
// The objection this task rests on is one sentence: a `LOGFONTW` that differs from the dialog's
// own face in `lfQuality` **and in nothing else** names the same type face at the same size, the
// same weight and the same character set, so a `WM_SETFONT` with it changes the rasterisation
// and not the metrics.
//
// That is a measurement, not an argument, and it is made here — the numbers below decided
// whether `src\settings.rs` was touched at all. Both faces are created in memory and selected
// into a **32-bit** memory DC: that is the depth ClearType is actually applied at, and a 1-bit
// DC would switch the smoothing off by itself and make the two sides agree for the wrong
// reason.
//
// The base is not a font invented by the test: it is built out of the `DS_SETFONT` declaration
// rc.exe wrote into the product's own resource, which is what the dialog manager builds the
// window's face out of.

/// The face the dialog manager creates for a window, as a `LOGFONTW` with `quality` on it.
///
/// The height is the manager's own arithmetic — a point size becomes a negative character
/// height at the DPI of the device — and the `+ 36` is the rounding `MulDiv` does.
fn manager_logfont(dc: HDC, font: &TemplateFont, quality: FONT_QUALITY) -> LOGFONTW {
    // SAFETY: `dc` is the live memory DC of the caller's sheet.
    let dpi = unsafe { GetDeviceCaps(Some(dc), LOGPIXELSY) };

    let mut logical = LOGFONTW {
        lfHeight: -((i32::from(font.points) * dpi + 36) / 72),
        lfWeight: i32::from(font.weight),
        lfItalic: font.italic,
        lfCharSet: FONT_CHARSET(font.charset),
        lfQuality: quality,
        ..Default::default()
    };

    for (slot, unit) in logical.lfFaceName.iter_mut().zip(font.face.encode_utf16()) {
        *slot = unit;
    }

    logical
}

/// One created face, freed on the way out — no test leaks a GDI object (NFR-13).
struct Face(HFONT);

impl Face {
    fn new(logical: LOGFONTW) -> Self {
        // SAFETY: `logical` is a live local of the caller's frame, read by the call; the handle
        // it answers is owned by this value and freed in `Drop`.
        let handle = unsafe { CreateFontIndirectW(&raw const logical) };
        assert!(
            !handle.is_invalid(),
            "CreateFontIndirectW must answer a face for the dialog's own LOGFONTW"
        );
        Self(handle)
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful `CreateFontIndirectW` above and is freed
        // exactly once — this type is neither `Copy` nor `Clone`, and every measurement that
        // selected it put the previous face back before it returned.
        let _ = unsafe { DeleteObject(self.0.into()) };
    }
}

/// `GetTextMetricsW` for one face on one sheet, with the previous face put back.
fn metrics_of(sheet: &Sheet, face: &Face) -> TEXTMETRICW {
    let mut metrics = TEXTMETRICW::default();

    // SAFETY: both handles are live; the out-pointer addresses a live local of this frame and
    // the call writes exactly one `TEXTMETRICW` through it. The previous face is put back
    // before the borrow ends. NFR-13: the result is examined.
    unsafe {
        let previous = SelectObject(sheet.dc, face.0.into());
        let read = GetTextMetricsW(sheet.dc, &raw mut metrics);
        SelectObject(sheet.dc, previous);
        assert!(
            read.as_bool(),
            "GetTextMetricsW must answer for a live face"
        );
    }

    metrics
}

/// The width `text` takes in one face on one sheet — `GetTextExtentPoint32W`.
fn extent_of(sheet: &Sheet, face: &Face, text: &str) -> SIZE {
    let wide: Vec<u16> = text.encode_utf16().collect();
    let mut size = SIZE::default();

    // SAFETY: `wide` and `size` are live locals of this frame; the call reads the units of the
    // one and writes the other. The previous face is put back. NFR-13: the result is examined.
    unsafe {
        let previous = SelectObject(sheet.dc, face.0.into());
        let measured = GetTextExtentPoint32W(sheet.dc, &wide, &raw mut size);
        SelectObject(sheet.dc, previous);
        assert!(
            measured.as_bool(),
            "GetTextExtentPoint32W must answer for a live face"
        );
    }

    size
}

/// The fields task T-11-20 names, as a printable row.
fn metrics_row(what: &str, metrics: &TEXTMETRICW) -> String {
    format!(
        "{what:<18} height={:<4} ascent={:<4} descent={:<4} internal={:<4} external={:<4} \
         ave={:<4} max={:<4} weight={}",
        metrics.tmHeight,
        metrics.tmAscent,
        metrics.tmDescent,
        metrics.tmInternalLeading,
        metrics.tmExternalLeading,
        metrics.tmAveCharWidth,
        metrics.tmMaxCharWidth,
        metrics.tmWeight,
    )
}

/// The dialog font of the settings template and a sheet to measure it on.
fn template_font_and_sheet() -> (TemplateFont, Sheet) {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let font = template
        .font
        .clone()
        .expect("the settings template declares DS_SETFONT");

    (font, Sheet::new(64))
}

/// The two starting points the manager's own `lfQuality` can be.
///
/// It is not written down anywhere this program can read: `DEFAULT_QUALITY` is what a zeroed
/// `LOGFONTW` carries, and `CLEARTYPE_QUALITY` is what the machine resolves it to. Both are
/// measured, so the answer does not depend on which one the manager actually passed.
const BASE_QUALITIES: [(&str, FONT_QUALITY); 2] = [
    ("DEFAULT_QUALITY", DEFAULT_QUALITY),
    ("CLEARTYPE_QUALITY", CLEARTYPE_QUALITY),
];

/// **Criterion 9 of T-11-20 — the whole of the controller's objection, as a measurement.**
///
/// The dialog's own face and [`theme::smoothed_logfont`] of it are created side by side
/// and measured on the same memory DC. Height, ascent, descent, internal and external leading,
/// average and maximum character width and weight must come back **equal**. Had one field
/// disagreed, the remainder would have been honestly unfixable and nothing would have been
/// handed to the five fields at all.
#[test]
fn our_face_measures_the_same_as_the_dialog_font_in_every_field_the_task_names() {
    let (font, sheet) = template_font_and_sheet();

    println!(
        "template font: {:?} {} pt, weight {}, italic {}, charset {}",
        font.face, font.points, font.weight, font.italic, font.charset
    );

    for (name, quality) in BASE_QUALITIES {
        let base = manager_logfont(sheet.dc, &font, quality);
        let ours = theme::smoothed_logfont(base);

        // The premise first: one field moved and not a byte else. A base that differed
        // somewhere else would make the metrics agree for a reason this task cannot claim.
        assert_eq!(
            ours.lfQuality, CLEARTYPE_QUALITY,
            "our face is the one that NAMES its smoothing — решение 88"
        );

        // ⚠ Since решение 88 the named mode is one of the two starting points below, so for
        // that one the builder moves **nothing** — and that is correct, not a miss. The claim
        // this test makes is «the mode is named», never «the mode differs from the manager's».
        if quality != CLEARTYPE_QUALITY {
            assert_ne!(ours.lfQuality, base.lfQuality, "and the base is not");
        }
        assert_eq!(ours.lfHeight, base.lfHeight);
        assert_eq!(ours.lfWidth, base.lfWidth);
        assert_eq!(ours.lfWeight, base.lfWeight);
        assert_eq!(ours.lfItalic, base.lfItalic);
        assert_eq!(ours.lfUnderline, base.lfUnderline);
        assert_eq!(ours.lfStrikeOut, base.lfStrikeOut);
        assert_eq!(ours.lfCharSet, base.lfCharSet);
        assert_eq!(ours.lfOutPrecision, base.lfOutPrecision);
        assert_eq!(ours.lfClipPrecision, base.lfClipPrecision);
        assert_eq!(ours.lfPitchAndFamily, base.lfPitchAndFamily);
        assert_eq!(ours.lfEscapement, base.lfEscapement);
        assert_eq!(ours.lfOrientation, base.lfOrientation);
        assert_eq!(ours.lfFaceName, base.lfFaceName);

        let dialog_face = Face::new(base);
        let our_face = Face::new(ours);

        let dialog_metrics = metrics_of(&sheet, &dialog_face);
        let our_metrics = metrics_of(&sheet, &our_face);

        println!("--- base {name} ---");
        println!("{}", metrics_row("шрифт диалога", &dialog_metrics));
        println!("{}", metrics_row("наше начертание", &our_metrics));

        for (field, dialog_value, our_value) in [
            (
                "высота (tmHeight)",
                dialog_metrics.tmHeight,
                our_metrics.tmHeight,
            ),
            (
                "подъём (tmAscent)",
                dialog_metrics.tmAscent,
                our_metrics.tmAscent,
            ),
            (
                "спуск (tmDescent)",
                dialog_metrics.tmDescent,
                our_metrics.tmDescent,
            ),
            (
                "внутренний интерлиньяж (tmInternalLeading)",
                dialog_metrics.tmInternalLeading,
                our_metrics.tmInternalLeading,
            ),
            (
                "внешний интерлиньяж (tmExternalLeading)",
                dialog_metrics.tmExternalLeading,
                our_metrics.tmExternalLeading,
            ),
            (
                "средняя ширина знака (tmAveCharWidth)",
                dialog_metrics.tmAveCharWidth,
                our_metrics.tmAveCharWidth,
            ),
            (
                "максимальная ширина знака (tmMaxCharWidth)",
                dialog_metrics.tmMaxCharWidth,
                our_metrics.tmMaxCharWidth,
            ),
            (
                "насыщенность (tmWeight)",
                dialog_metrics.tmWeight,
                our_metrics.tmWeight,
            ),
        ] {
            assert_eq!(
                dialog_value, our_value,
                "base {name}: {field} — шрифт диалога {dialog_value}, наше начертание \
                 {our_value}. Метрики разошлись: WM_SETFONT с этим начертанием сдвинул бы \
                 текст, и остаток честно неисправим"
            );
        }
    }
}

/// **Criterion 11 of T-11-20 — `WM_SETFONT` leaves by the way the FR-72 guard allows.**
///
/// The guard is `tests\guard.rs`, `send_message_is_used_nowhere_and_the_timeout_is_the_fifty_of_fr72`:
/// it sweeps every source of `src\` for the literal `SendMessage` and lets a line through only
/// when it is `SendMessageTimeout`. `SendDlgItemMessageW` does not contain the literal and is
/// what this module already calls «the one way this module sends anything to its own controls»;
/// `send_to` is the wrapper. So the check here is that the hand-over goes through `send_to` and
/// that no second path was opened beside it.
#[test]
fn the_face_is_handed_over_by_the_one_send_this_module_uses_for_its_own_controls() {
    // The message name also stands in the `use` list at the top of the module. A row of that
    // list is names and commas and has no call in it, so a bracket is what separates the
    // declaration of the name from a use of it.
    let sends: Vec<String> = product_lines_with("WM_SETFONT")
        .into_iter()
        .filter(|line| line.contains('('))
        .collect();

    for line in &sends {
        println!("{line}");
    }

    assert_eq!(
        sends.len(),
        1,
        "the face is handed over in one place and not in several: {sends:?}"
    );

    assert!(
        sends[0].contains("send_to("),
        "WM_SETFONT must go through `send_to`, the SendDlgItemMessageW wrapper the FR-72 guard \
         lets past — the line is {:?}",
        sends[0]
    );

    // And the file opened no bare send of its own. The guard says this for the whole of `src\`;
    // it is repeated on this one file because this task is the one that added a send.
    let bare: Vec<String> = product_lines_with("SendMessage")
        .into_iter()
        .filter(|line| !line.contains("SendMessageTimeout"))
        .collect();

    assert!(
        bare.is_empty(),
        "FR-72: a bare SendMessage appeared in src\\settings.rs: {bare:?}"
    );
}

/// **Criterion 10 of T-11-20 — no second face was created for this.**
///
/// The face handed to the six controls is [`settings::CONTROLS_THAT_DRAW_THEIR_OWN_TEXT`]'s
/// share of the one the window already owned: `DialogFonts::text`, made once on
/// `WM_INITDIALOG` and freed in `Drop`. The sweep says so of the source — every face of this
/// module is made by the single `create_font`, which is called by the constructor of
/// `DialogFonts` and by nothing else, and there is no `CreateFontIndirectW` anywhere else.
///
/// ⚠ **Three calls since task T-12-4 and not two**: that task gave the about window's name row
/// a face of its own — `about_name_logfont`, 10,7 pt bold, finding A-05 — and put it in the
/// same set, made by the same function on the same `WM_INITDIALOG` and freed by the same
/// `Drop`. The number below is the count of *faces of the set*, and the point of the assertion
/// is unchanged: nothing outside that constructor asks GDI for a font.
///
/// ⚠ **Task Т-26-2 stopped counting call sites and started counting faces.** The set grew to
/// six (the body, the numerals and the chip of решение 87 joined the text, the caption and the
/// name), and six `else` arms unwinding six half-built sets would have been six copies of the
/// same three lines — so the constructor makes them in a **loop over one array**, and there is
/// exactly one `create_font(` call site for all six. Counting call sites would now say «one»
/// however many faces the set had, which measures nothing; the array is what the number below
/// is taken from. The probe of `resolve_emphasis` is the one face made outside the set, and it
/// is deliberately named here: it is a **question to GDI**, freed in the same function, and
/// never handed to a window.
#[test]
fn the_fields_are_handed_the_face_the_window_already_owned_and_not_a_new_one() {
    let made = product_lines_with("CreateFontIndirectW(");
    for line in &made {
        println!("{line}");
    }
    assert_eq!(
        made.len(),
        1,
        "every face of this module comes out of the one `create_font`: {made:?}"
    );

    let created = product_lines_with("create_font(");
    for line in &created {
        println!("{line}");
    }
    assert_eq!(
        created.len(),
        3,
        "the declaration, the one loop of `DialogFonts::new` and the one probe of \
         `resolve_emphasis` — and nothing else asks for a face: {created:?}"
    );

    // …and the size of the set is the length of the array the loop walks, which is where the
    // number «six faces» is really written down. Read off the source, so a seventh face added
    // without a `Drop` for it cannot slip past.
    let source = settings_module_source();
    let wanted = function_body(
        &source,
        "fn new(hwnd: HWND, control: i32) -> Option<Self> {",
    );

    let faces = wanted.matches("_logfont(base").count();

    println!("лиц в наборе: {faces}");

    assert_eq!(
        faces, 6,
        "the set of решение 87 is six faces: text, caption, name, body, number, chip"
    );

    // Every one of them is freed, and the list `Drop` walks is the list the constructor filled.
    let dropped = function_body(&source, "impl Drop for DialogFonts {");

    for face in [
        "self.text",
        "self.caption",
        "self.name",
        "self.body",
        "self.number",
        "self.chip",
    ] {
        assert!(
            dropped.contains(face),
            "`{face}` is made and never freed — `Drop` must walk the whole set: {dropped}"
        );
    }

    // The hand-over takes a face it was given, and takes it from the owner.
    let handed = product_lines_with("hand_our_face_to_the_controls_that_draw_their_own_text");
    for line in &handed {
        println!("{line}");
    }
    assert_eq!(handed.len(), 2, "declared once and called once: {handed:?}");

    let source = settings_module_source();

    assert!(
        source.contains("state.fonts = DialogFonts::new(hwnd, GROUP_BOXES[0]);\n\n                    fill_dialog(hwnd, state);"),
        "the face has to be made before the filling that hands it over — task T-11-20's ordering"
    );

    assert!(
        source.contains("impl Drop for DialogFonts"),
        "the faces are still freed in Drop"
    );
}

/// **Criterion 10 of T-11-20, from the resource** — the list of controls the face is handed to
/// is exactly the controls of the template that draw their own text.
///
/// Read from the compiled template and not from a list written twice: every control of class
/// `EDIT` — the ordinal `0x0081` — plus the one control named by a registered class string,
/// `SysListView32`, which is the layout list. Everything else of the window is drawn by the
/// module itself and needs no face of its own. A field added to the template later shows up
/// here as a control the hand-over does not name.
#[test]
fn the_face_goes_to_every_control_of_the_template_that_draws_its_own_text_and_to_no_other() {
    const EDIT_CLASS_ORDINAL: u16 = 0x0081;

    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let mut own_text: Vec<u32> = template
        .classes
        .iter()
        .filter(|(_, class)| *class == Some(EDIT_CLASS_ORDINAL) || class.is_none())
        .map(|(id, _)| *id)
        .collect();
    own_text.sort_unstable();

    let mut handed: Vec<u32> = settings::CONTROLS_THAT_DRAW_THEIR_OWN_TEXT
        .iter()
        .map(|id| u32::try_from(*id).expect("a control identifier is positive"))
        .collect();
    handed.sort_unstable();

    println!("template: {own_text:?}");
    println!("handed:   {handed:?}");

    assert_eq!(
        own_text, handed,
        "the controls of the template that draw their own text are {own_text:?}, and the face \
         is handed to {handed:?} — a control in the first list and not in the second stays on \
         the manager's ClearType while the rest of the window is grey-antialiased"
    );

    assert_eq!(
        handed.len(),
        3,
        "the two EDITTEXT fields and the layout list — the remainder task T-11-20 names, three \
         since task Т-23-2 took the three millisecond fields off the window"
    );
}

/// **Criterion 12 of T-11-20 — the width of real text did not move either.**
///
/// A metric is an average; what the five fields and the rows of the layout list actually put on
/// the screen is a string. The strings measured here are the ones those controls hold: the
/// digits of the three numeric fields, the name of a key, a process name of the exclusion list,
/// the names of two layouts, and a line whose letters are the widest and the narrowest there
/// are — a face that rounded advances differently would show it here first.
#[test]
fn real_strings_take_the_same_width_in_our_face_as_in_the_dialog_font() {
    let (font, sheet) = template_font_and_sheet();

    for (name, quality) in BASE_QUALITIES {
        let base = manager_logfont(sheet.dc, &font, quality);

        let dialog_face = Face::new(base);
        let our_face = Face::new(theme::smoothed_logfont(base));

        for text in [
            "0",
            "1500",
            "99999",
            "Pause",
            "notepad.exe",
            "Русский (Россия)",
            "English (United States)",
            "Шшщ ЖЮЯ — jgqy WMil",
        ] {
            let dialog_size = extent_of(&sheet, &dialog_face, text);
            let our_size = extent_of(&sheet, &our_face, text);

            println!(
                "{name}: {text:?} — шрифт диалога {}x{}, наше начертание {}x{}",
                dialog_size.cx, dialog_size.cy, our_size.cx, our_size.cy
            );

            assert_eq!(
                (dialog_size.cx, dialog_size.cy),
                (our_size.cx, our_size.cy),
                "base {name}: «{text}» is {}x{} in the dialog's own face and {}x{} in ours — \
                 the text would move, and criterion 12 forbids it",
                dialog_size.cx,
                dialog_size.cy,
                our_size.cx,
                our_size.cy
            );
        }
    }
}

// -----------------------------------------------------------------------------------------
// Настоящие пиксели: строка списка раскладок — задача T-11-25, критерии 10, 11 и 12
// -----------------------------------------------------------------------------------------
//
// `draw_cycle_row` needs a DC and nothing else, so the whole of this runs in a memory bitmap:
// no window is created, no `SysListView32` is asked for anything, and the product is not
// started. What the control still does with the row — the transparent text background of
// `CLR_NONE`, the label over the drawing below — is the controller's half, on the real window.

/// The colour of the sheet beyond the row: not a colour of either palette, so a drawing that
/// escaped the rectangle it was given would show up as a changed marker pixel.
const BEYOND_THE_ROW: COLORREF = COLORREF(0x0000_80FF);

/// One row of the layout list, drawn into a memory bitmap the way the item prepaint draws it.
struct Row {
    sheet: Sheet,
    area: RECT,
}

impl Row {
    /// A row 112 × 20 pixels — the width of the single column of the list and the height of
    /// `LAYOUT_ROW_HEIGHT_DLU` near enough — inset into a sheet larger than itself.
    fn draw(mode: LayoutMode, selected: bool, checked: bool, palette: &Palette) -> Self {
        let sheet = Sheet::new(128);
        sheet.clear(BEYOND_THE_ROW);

        let area = RECT {
            left: 8,
            top: 8,
            right: 120,
            bottom: 28,
        };

        settings::draw_cycle_row(sheet.dc, &area, mode, selected, checked, palette, 96);

        Self { sheet, area }
    }

    fn rgb(&self, x: i32, y: i32) -> (i32, i32, i32) {
        self.sheet.rgb(x, y)
    }

    /// Every pixel of the sheet, in reading order — for comparing two rows outright.
    fn pixels(&self) -> Vec<(i32, i32, i32)> {
        let mut all = Vec::new();

        for y in 0..128 {
            for x in 0..128 {
                all.push(self.rgb(x, y));
            }
        }

        all
    }
}

/// The three channels of a palette colour, in the order [`Sheet::rgb`] answers them.
fn channels(colour: COLORREF) -> (i32, i32, i32) {
    (
        (colour.0 & 0xFF) as i32,
        ((colour.0 >> 8) & 0xFF) as i32,
        ((colour.0 >> 16) & 0xFF) as i32,
    )
}

/// **Criterion 10 of T-11-25, on real pixels** — the selection of the layout list is a rounded
/// stripe inset into the row and not the row itself.
///
/// Three things at once, and each of them is what the mock-ups say: the band of the inset
/// carries the ground and not the selection; the pixel next to it carries the selection; and
/// each of the four corners of the stripe carries at least one partially covered pixel, which
/// is what a rounded corner is and what a rectangle can never have.
#[test]
fn the_selection_of_the_layout_list_is_a_rounded_stripe_inset_into_the_row() {
    let inset = theme::scaled(theme::LIST_SELECTION_INSET, 96);
    let radius = theme::scaled(theme::LIST_SELECTION_RADIUS, 96);

    // The corner tile of `paint_rounded`: the radius and the frame that runs around it.
    let side = radius + theme::scaled(theme::BORDER_THICKNESS, 96).max(1);

    for palette in [&GRAPHITE, &FOG] {
        let row = Row::draw(LayoutMode::Cycle, true, false, palette);

        let ground = channels(palette.field_bg);
        let selection = channels(palette.sel_bg);
        let middle = (row.area.top + row.area.bottom) / 2;

        // The band of the inset, on both sides: the ground of the row, never the selection.
        for offset in 0..inset {
            for (name, x) in [
                ("left", row.area.left + offset),
                ("right", row.area.right - 1 - offset),
            ] {
                assert_eq!(
                    row.rgb(x, middle),
                    ground,
                    "the {name} inset band must carry the ground at x = {x}, palette {:?}",
                    palette.field_bg
                );
            }
        }

        // And the first pixel inside it, on both sides: the selection.
        for (name, x) in [
            ("left", row.area.left + inset),
            ("right", row.area.right - 1 - inset),
        ] {
            assert_eq!(
                row.rgb(x, middle),
                selection,
                "the selection must start the very next pixel on the {name}, at x = {x}"
            );
        }

        // Nothing of the row reached outside the rectangle it was given.
        for (x, y) in [
            (row.area.left - 1, middle),
            (row.area.right, middle),
            (row.area.left + 20, row.area.top - 1),
            (row.area.left + 20, row.area.bottom),
        ] {
            assert_eq!(
                row.rgb(x, y),
                channels(BEYOND_THE_ROW),
                "the row must not paint outside itself — ({x}, {y})"
            );
        }

        // The four corners of the stripe, each with at least one pixel that is neither the
        // ground nor the selection: partial coverage, which is the curve itself.
        let stripe = RECT {
            left: row.area.left + inset,
            top: row.area.top,
            right: row.area.right - inset,
            bottom: row.area.bottom,
        };

        let tiles = theme::corner_tiles(&stripe, side);
        let mut blended = [0; 4];

        for (corner, tile) in tiles.iter().enumerate() {
            for y in tile.top..tile.bottom {
                for x in tile.left..tile.right {
                    let value = row.rgb(x, y);

                    if value != ground && value != selection {
                        blended[corner] += 1;
                    }
                }
            }
        }

        println!(
            "выделение, палитра {:?}: втяжка {inset} px, радиус {radius} px, полутонов по \
             углам {blended:?} из {} на угол",
            palette.sel_bg,
            side * side
        );

        assert!(
            blended.iter().all(|count| *count >= 1),
            "every one of the four corners of the stripe must be rounded — {blended:?} \
             partially covered pixels, and a corner with none of them is a right angle"
        );
    }
}

/// **Criterion 13 of T-11-25** — one figure, two lists, one pair of constants.
///
/// The exclusion list has drawn this stripe since T-11-16; the layout list draws the same one
/// now. Held as one body called twice rather than as two bodies that agree today: a third pair
/// of constants is exactly what a second copy would eventually grow.
#[test]
fn both_lists_of_the_dialog_draw_the_same_selection_stripe() {
    // ⚠ `drawing_source()` and not `settings_module_source()` since task T-14-5: the shared
    // figure moved to `theme` with the drawing library while both callers stayed with the
    // window that owns them, so the two ends of this criterion now live in two files. The
    // join also **strengthens** the count below — a second `scaled(LIST_SELECTION_…` would
    // now be forbidden in either file rather than in one of them.
    let source = drawing_source();
    let body = function_body(&source, "fn paint_selection_stripe(");

    for call in [
        "scaled(LIST_SELECTION_INSET, dpi)",
        "scaled(LIST_SELECTION_RADIUS, dpi)",
        "paint_rounded(",
    ] {
        assert!(
            body.contains(call),
            "the stripe must take `{call}` — the втяжка and the радиус of the mock-ups \
             through the one scale"
        );
    }

    for (signature, list) in [
        ("unsafe fn draw_list_item(", "the exclusion list"),
        ("pub fn draw_cycle_row(", "the layout list"),
    ] {
        assert!(
            function_body(&source, signature).contains("paint_selection_stripe("),
            "{list} must draw its selection with the one shared figure"
        );
    }

    // And neither list carries the lengths itself: a second `scaled(LIST_SELECTION_…` outside
    // the shared body would be the third pair this criterion forbids.
    for constant in ["LIST_SELECTION_INSET", "LIST_SELECTION_RADIUS"] {
        let used = source.matches(&format!("scaled({constant}, dpi)")).count();

        assert_eq!(
            used, 1,
            "`{constant}` must be scaled in exactly one place — the shared stripe — and it is \
             scaled in {used}"
        );
    }
}

/// **Criterion 11 of T-11-25, on real pixels** — the square of the tick is smoothed against the
/// ground the row actually wears, and carries no key colour nor any admixture of one.
///
/// # The two halves, and why the second one needs its own arithmetic
///
/// The first half is the smoothing: each of the four corners of the square must hold a pixel
/// that is none of the colours the picture is drawn from — partial coverage. The controller
/// measured the opposite on the stand before this task: `28 → 230` with nothing in between,
/// «ни одного полутона по всему периметру».
///
/// The second half is the fringe. A pixel of pure magenta is easy to look for and would not
/// have been the defect anyway: the defect this test rules out is a *blend* of the key with a
/// paint of the palette, which is what a smoothed edge over `CHECK_CELL_KEY` would have
/// produced. `(r + b) / 2 − g` is the measure of it — an affine function of the colour, so a
/// blend of two colours takes the blend of their values, and it is at most 3 for every colour
/// of either palette and 255 for the key itself. Anything above 8 is a magenta admixture of
/// more than two per cent, and there is none.
#[test]
fn the_square_of_the_tick_is_smoothed_and_carries_no_key_colour() {
    // The corner tile of `paint_rounded` at the radius of a layout-list tick.
    let side = theme::scaled(settings::LIST_CHECK_CORNER_RADIUS, 96)
        + theme::scaled(theme::BORDER_THICKNESS, 96).max(1);

    for palette in [&GRAPHITE, &FOG] {
        for selected in [false, true] {
            for checked in [false, true] {
                let row = Row::draw(LayoutMode::Cycle, selected, checked, palette);
                let glyph = settings::check_glyph(&row.area, 96);

                // Everything the picture is drawn from: the ground under the tick, its fill,
                // its frame and its mark. A pixel that is none of these is a blend.
                let colours = [
                    channels(if selected {
                        palette.sel_bg
                    } else {
                        palette.field_bg
                    }),
                    channels(palette.field_bg),
                    channels(palette.accent_bg),
                    channels(palette.accent_fg),
                    channels(palette.box_border),
                ];

                let tiles = theme::corner_tiles(&glyph, side);
                let mut blended = [0; 4];

                for (corner, tile) in tiles.iter().enumerate() {
                    for y in tile.top..tile.bottom {
                        for x in tile.left..tile.right {
                            if !colours.contains(&row.rgb(x, y)) {
                                blended[corner] += 1;
                            }
                        }
                    }
                }

                // The fringe, over the whole row and not only over the tick.
                let mut worst = i32::MIN;
                let mut worst_at = (0, 0);
                let mut key = Vec::new();

                for y in row.area.top..row.area.bottom {
                    for x in row.area.left..row.area.right {
                        let (red, green, blue) = row.rgb(x, y);
                        let magenta = red + blue - 2 * green;

                        if magenta > worst {
                            worst = magenta;
                            worst_at = (x, y);
                        }

                        if (red, green, blue) == (0xFF, 0x00, 0xFF) {
                            key.push((x, y));
                        }
                    }
                }

                println!(
                    "галочка, палитра {:?}, выделена {selected}, взведена {checked}: полутонов \
                     по углам {blended:?} из {} на угол, пурпур не выше {},{} в ({}, {})",
                    palette.field_bg,
                    side * side,
                    worst / 2,
                    (worst % 2) * 5,
                    worst_at.0,
                    worst_at.1
                );

                assert!(
                    blended.iter().all(|count| *count >= 1),
                    "every one of the four corners of the square must carry partial coverage — \
                     {blended:?}, and a corner with none of them is the hard step the stand \
                     measured before this task"
                );

                assert!(
                    key.is_empty(),
                    "no pixel of the row may be the key colour — {:?}",
                    &key[..key.len().min(8)]
                );

                assert!(
                    worst <= 16,
                    "no pixel of the row may carry an admixture of the key either — \
                     (r + b) / 2 − g is {},{} at ({}, {}), and every colour of the palette is \
                     at most 3",
                    worst / 2,
                    (worst % 2) * 5,
                    worst_at.0,
                    worst_at.1
                );
            }
        }
    }
}

/// **Criterion 12 of T-11-25, on real pixels** — the pair mode did not move.
///
/// The table test `the_row_paints_follow_the_mode_table_of_fr_92a` holds the colours; this
/// holds the drawing they turn into. A pair-mode row is the same picture whether it carries
/// `LVIS_SELECTED` or not, and it is the same picture an ordinary cycle-mode row is — the
/// selection of FR-31 «в нём не появляется», said in pixels rather than in a second condition
/// that could drift from the table. The cycle-mode selected row is compared too, so that a
/// change which quietly stopped drawing the stripe at all could not pass this test.
#[test]
fn the_pair_mode_row_shows_no_selection_at_all() {
    for palette in [&GRAPHITE, &FOG] {
        for checked in [false, true] {
            let ordinary = Row::draw(LayoutMode::Cycle, false, checked, palette).pixels();

            for selected in [false, true] {
                assert_eq!(
                    Row::draw(LayoutMode::Pair, selected, checked, palette).pixels(),
                    ordinary,
                    "a pair-mode row with selected = {selected} must be the very picture an \
                     ordinary row is — palette {:?}, checked {checked}",
                    palette.field_bg
                );
            }

            assert_ne!(
                Row::draw(LayoutMode::Cycle, true, checked, palette).pixels(),
                ordinary,
                "and the cycle mode must still draw its selection — palette {:?}",
                palette.sel_bg
            );
        }
    }
}

/// **T-11-25-2, the first half of the repair** — the row painter takes the rectangle of the row
/// from the control and never from the message.
///
/// # What this holds and why it is worth a test of its own
///
/// `NMCUSTOMDRAW` carries an `rc`, and for a list view it is **not filled** at the item prepaint
/// stage: measured on the live control through the stand, every row arrives with
/// `{0, 0, 0, 0}`. T-11-25 painted into that rectangle, which is why the shipped picture had no
/// ground, no selection stripe and one tick for two rows, stacked at the top-left corner of the
/// list nine scan lines above the first row. Nothing in memory could have caught it — the row
/// painter is pure in its rectangle and was measured with a good one — so what is held here is
/// the **road**: the rectangle comes out of `LVM_GETITEMRECT`, and `draw.nmcd.rc` is not read
/// anywhere in the handler.
#[test]
fn the_row_painter_takes_its_rectangle_from_the_control_and_not_from_the_message() {
    let source = settings_module_source();
    let handler = function_body(&source, "unsafe fn on_notify(");

    assert!(
        handler.contains("row_bounds(hwnd, draw.nmcd.dwItemSpec)"),
        "the item prepaint must ask the control where the row is"
    );
    // Comments are allowed to name it — that is where the measurement is written down; code is
    // not.
    let code: String = handler
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();

    assert!(
        !code.contains("draw.nmcd.rc"),
        "and it must not read the rectangle of the message at all — a list view never fills it"
    );

    let asked = function_body(&source, "fn row_bounds(");

    for part in [
        "LVM_GETITEMRECT",
        "left: i32::try_from(LVIR_BOUNDS)",
        "std::ptr::from_mut(&mut rect) as isize",
    ] {
        assert!(
            asked.contains(part),
            "the documented request needs `{part}` — the code travels in `left` and the whole \
             rectangle comes back in the client pixels of the control's own paint"
        );
    }

    // FALSE is an index the control does not place, and then there is nowhere honest to paint.
    assert!(
        asked.contains("(answered != 0).then_some(rect)"),
        "a refused request must answer `None` rather than a rectangle of zeros — that is the \
         very shape of the defect this task repairs"
    );
    assert!(
        handler.contains("if let Some(row) = bounds"),
        "and the handler must paint only when it was given somewhere to paint"
    );
}

/// **T-11-25-2, the second half of the repair** — the cell of the state image list is a hole
/// only while the list carries no ground of its own, and a list view gives it one.
///
/// # The measurement this test is made of
///
/// `the_cell_of_the_state_image_list_is_a_hole_edge_to_edge` above draws the two cells of a
/// freshly built list and finds them harmless. That is true of a list nobody has touched —
/// `ImageList_Draw` with `ILD_NORMAL` uses the mask when the list's background colour is
/// `CLR_NONE`, which is what a new list carries — and it is **not** true of the list once a
/// control is holding it. Measured on the live control through the stand:
///
/// * straight after `LVM_SETIMAGELIST` the cell colour reads `0x00211c19`, `field_bg` of the
///   graphite palette — the value `LVM_SETBKCOLOR` had been given;
/// * sent `LVM_SETBKCOLOR` again with `0x00112233`, the cell colour reads `0x00112233`.
///
/// With that colour in place every cell is an opaque rectangle of the control's own ground,
/// painted **after** the item prepaint — it punched the tick and a bite of the selection stripe
/// out of every row of the shipped T-11-25. Here the same thing is done deliberately and the
/// pixels are counted: with a ground the cell covers, with `CLR_NONE` it covers nothing.
#[test]
fn the_cells_are_a_hole_only_while_the_image_list_keeps_no_ground_of_its_own() {
    use windows::Win32::UI::Controls::{
        CLR_NONE, ILD_NORMAL, ImageList_Destroy, ImageList_Draw, ImageList_SetBkColor,
    };

    let list = settings::build_check_image_list(19).expect("the state image list must build");
    let ground = COLORREF(0x0040_3020);

    let mut counts = Vec::new();

    // First the colour a list view writes into the cells, then the one this module writes back.
    for cell_ground in [GRAPHITE.field_bg, COLORREF(CLR_NONE as u32)] {
        // SAFETY: `list` is the live list built above and owned by this frame; the call writes
        // one field of it and follows no pointer of ours.
        let _ = unsafe { ImageList_SetBkColor(list, cell_ground) };

        let sheet = Sheet::new(96);
        sheet.clear(ground);

        for index in 0..2 {
            // SAFETY: `list` is live and ours, `sheet.dc` holds the sheet's own bitmap, and the
            // cell is far smaller than the sheet.
            let drawn = unsafe { ImageList_Draw(list, index, sheet.dc, 4, 4, ILD_NORMAL) };

            assert!(drawn.as_bool(), "cell {index} must draw");
        }

        let touched = (0..96)
            .flat_map(|y| (0..96).map(move |x| (x, y)))
            .filter(|(x, y)| sheet.rgb(*x, *y) != (0x20, 0x30, 0x40))
            .count();

        println!(
            "ячейка на земле {cell_ground:?}: {touched} изменённых пикселей из {}",
            96 * 96
        );

        counts.push(touched);
    }

    // SAFETY: the list was built by us, handed to no control, and is freed exactly once.
    let _ = unsafe { ImageList_Destroy(Some(list)) };

    assert!(
        counts[0] > 0,
        "with a ground of its own the cell must cover it — otherwise this test proves nothing \
         about the colour the control writes in"
    );
    assert_eq!(
        counts[1], 0,
        "and with `CLR_NONE` the cell must be a hole again — {} pixels covered",
        counts[1]
    );

    // The road: the ground is taken back out after both messages that write it in. The startup
    // order is `paint_cycle_list` and then `install_check_images`; a palette change sends only
    // the first, and a fresh image list is coloured by the second.
    let source = settings_module_source();
    let clearing = function_body(&source, "fn clear_state_image_ground(");

    for part in ["LVM_GETIMAGELIST", "LVSIL_STATE", "ImageList_SetBkColor("] {
        assert!(
            clearing.contains(part),
            "the clearing must ask the control for the list it holds and use `{part}`"
        );
    }
    assert!(
        clearing.contains("COLORREF(CLR_NONE as u32)"),
        "and it must write the documented «no ground, use the mask» value"
    );

    for signature in ["fn paint_cycle_list(", "fn install_check_images("] {
        assert!(
            function_body(&source, signature).contains("clear_state_image_ground(hwnd)"),
            "`{signature}` writes the ground into the cells and must take it out again"
        );
    }
}

// -----------------------------------------------------------------------------------------
// Геометрия диалога: коробка поля, разведённые прямоугольники, список вровень с кнопкой —
// задача T-12-3
// -----------------------------------------------------------------------------------------
//
// Four defects of the Э12 comparison, and every one of them is arithmetic on the template and
// on one pure function — no window is created and the product is not started. What the pixels
// then do is the controller's half, on the stand.

/// The two input fields of `IDD_SETTINGS`, by identifier.
///
/// The same two `EDITTEXT` rows `settings::FRAMED_FIELDS` names minus the two lists — the
/// controls a `12`-unit box is drawn round and whose own rectangle is one font height. Five
/// until task Т-23-2 took the three millisecond fields out of the window.
const INPUT_FIELDS: [(u32, &str); 2] = [
    (1010, "Горячая клавиша: поле клавиши"),
    (1051, "Исключения: имя процесса"),
];

/// `MulDiv(units, base, 8)` — what the dialog manager does to a **vertical** number of a
/// template, with the round-to-nearest `MulDiv` is documented to do.
///
/// `base` is the vertical dialog base unit: the height of the dialog's own font, which is what
/// `MapDialogRect` divides by eight. Measured, never assumed — see the caller.
fn vertical_units(units: i32, base: i32) -> i32 {
    (units * base + 4) / 8
}

/// The vertical dialog base unit of `IDD_SETTINGS` on this machine — the height of the face
/// its `DS_SETFONT` declares, measured on a memory DC.
fn vertical_base_unit() -> i32 {
    let (font, sheet) = template_font_and_sheet();
    let face = Face::new(manager_logfont(sheet.dc, &font, DEFAULT_QUALITY));

    metrics_of(&sheet, &face).tmHeight
}

/// **Defect Г-3 of the Э12 comparison** — the box the mock-ups draw round an input field is
/// twelve dialog units, the control inside it is one font height, and the difference falls
/// above and below the text in equal halves.
///
/// The whole of the cure in one place: the number comes from the generator (`'edit'` rows of
/// `ui.ps1`, `h = 12`), the control comes from the definition of the vertical dialog unit
/// (`DIALOG_FONT_HEIGHT_DLU` = 8 of them are one font height), and the frame is drawn by
/// [`theme::field_frame_air`] round the middle of the control. Read out of the **built**
/// `LangSwitcher.exe`, like every other statement this suite makes about the template.
#[test]
fn the_box_round_an_input_field_is_the_twelve_dialog_units_of_the_generator() {
    assert_eq!(
        settings::FIELD_BOX_DLU,
        12,
        "the box of the mock-ups is the `h = 12` of every `'edit'` row of the generator"
    );

    // The derivation is written down where the number is, and not as a count of pixels read
    // off a picture — `design-tokens.md` §3, the error class that produced T-11-15 and F3.
    let source = settings_module_source();
    let derivation = source
        .split_once("pub const FIELD_BOX_DLU")
        .expect("the module must declare FIELD_BOX_DLU")
        .0;

    for phrase in ["ui.ps1", "h = 12", "MulDiv(12, 15, 8)"] {
        assert!(
            derivation.contains(phrase),
            "the doc comment of FIELD_BOX_DLU must carry «{phrase}» — the derivation, not a \
             pixel count"
        );
    }

    // Every one of the five fields is one font height tall in the template, and not the box.
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    for (id, what) in INPUT_FIELDS {
        let (_, top, _, bottom) = template.rect_of(id);

        println!("«{what}» ({id}): {} dialog units tall", bottom - top);

        assert_eq!(
            bottom - top,
            settings::DIALOG_FONT_HEIGHT_DLU,
            "«{what}» ({id}) has to be one dialog font height — a taller control puts the \
             whole surplus under the text, which is defect Г-3"
        );
    }

    // The arithmetic of the frame, in pixels, at the base unit of this machine.
    let base = vertical_base_unit();
    let field_box = vertical_units(settings::FIELD_BOX_DLU, base);
    let control = vertical_units(settings::DIALOG_FONT_HEIGHT_DLU, base);
    let air = theme::field_frame_air(Some(field_box), control, 1);
    let outer = control + 2 * air;

    println!(
        "base unit {base}: box {field_box} px, control {control} px, air {air} px, outer \
         {outer} px"
    );

    assert_eq!(
        air,
        (field_box - control) / 2,
        "the air is half of what the box has left over"
    );
    assert!(
        outer <= field_box && outer >= field_box - 1,
        "the box round the control is {outer} px against the {field_box} px of the mock-ups — \
         centring may lose the odd pixel of an odd remainder and nothing more"
    );

    // The same table with the numbers of 96 DPI written out, so the derivation is held even on
    // a machine that measures something else: box 23, control 15, air 4, outer 23.
    assert_eq!(theme::field_frame_air(Some(23), 15, 1), 4);
    assert_eq!(15 + 2 * 4, 23);

    // **The rule degenerates into the old one.** A control already as tall as the box, one
    // taller than it, and a refused `MapDialogRect` all keep the one-thickness frame every
    // field wore before this task (NFR-13).
    for (name, box_height, control_height) in [
        ("as tall as the box", Some(23), 23),
        ("taller than the box", Some(23), 30),
        ("MapDialogRect refused", None, 15),
    ] {
        assert_eq!(
            theme::field_frame_air(box_height, control_height, 1),
            1,
            "«{name}» must fall back to one border thickness"
        );
    }

    // A thicker border at a higher DPI is the floor, not the answer.
    assert_eq!(theme::field_frame_air(Some(23), 23, 2), 2);

    // The pass draws it: the same number above and below a field, and the list branch keeps
    // the air of the mock-ups above its first row.
    let pass = function_body(&source, "unsafe fn paint_background(");

    for part in [
        "field_frame_air(field_box, rect.bottom - rect.top, border)",
        "(air, air)",
        "(list_top, border)",
        "FRAMED_LISTS.contains(control)",
        "dialog_units(hwnd, 0, FIELD_BOX_DLU)",
    ] {
        assert!(
            pass.contains(part),
            "the field pass of the background must carry `{part}`"
        );
    }
}

/// **Defect Г-1 of the Э12 comparison, «критично»** — nothing overlaps the «Оформление» combo
/// box, and the place the retired restart hint left beside the language row is empty.
///
/// An `SS_OWNERDRAW` static fills its own rectangle with `panel_bg` before it writes a word, so
/// a static overlapping a combo box does not sit on top of it — it **erases** the top of the
/// frame the background drew.
///
/// ⚠ **Rewritten by task Т-31-3.** The defect had one culprit — the hint 1092 sitting beside the
/// language row — and решение 99.4 retired that control with its sentence. A test written about
/// two rectangles would now be a test about one, so it is written about the property instead:
/// **no** control of the template may overlap the combo, and 1092 is not in the template at all.
/// The guard is stronger than the one it replaces and it costs the same line.
#[test]
fn nothing_overlaps_the_appearance_row_and_the_hint_is_gone() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    assert!(
        !template.controls.contains(&1092),
        "1092 — «вступит в силу после перезапуска» — was retired by решение 99.4; the place \
         right of the language combo stays empty and the layout does not move"
    );

    let (combo_left, combo_top, combo_right, combo_bottom) = template.rect_of(1003);

    println!("«Оформление» (1003): {combo_left},{combo_top}..{combo_right},{combo_bottom}");

    // ⚠ The closed part of the combo is twelve dialog units (task T-12-2); the rectangle the
    // template declares carries the **dropped-down list** below it, and a control standing
    // under the closed part is not the defect. What Г-1 was about is the *top* of the frame,
    // so the band this test guards is the closed part.
    let closed_bottom = combo_top + 12;
    let mut checked = 0;

    // ⚠ **Statics only, and the six panels are deliberately not among them.** A panel is a
    // `Button` of class 0x0080 carrying `NOT WS_VISIBLE`, and it contains every control of its
    // group by construction — «Общие» (1090) is 7,7 210×94 and the combo sits inside it. What
    // Г-1 was about is a *static*: `SS_OWNERDRAW` fills its own rectangle before it writes.
    const STATIC_CLASS: u16 = 0x0082;

    for (id, x, y, cx, cy) in &template.bounds {
        let is_static = template
            .classes
            .iter()
            .any(|(other, class)| other == id && *class == Some(STATIC_CLASS));

        if !is_static {
            continue;
        }

        checked += 1;

        let overlaps =
            *x < combo_right && combo_left < x + cx && *y < closed_bottom && combo_top < y + cy;

        assert!(
            !overlaps,
            "static {id} ({x},{y} {cx}×{cy}) overlaps the closed part of «Оформление» \
             ({combo_left},{combo_top}..{combo_right},{closed_bottom}) — an SS_OWNERDRAW static \
             there erases the top of the combo box's frame, which is defect Г-1"
        );
    }

    println!("{checked} statics checked against the combo");

    assert!(
        checked >= 14,
        "«overlaps: none» out of a handful of controls is the cheapest lie such a test tells: \
         {checked} statics is not this template"
    );

    // And the row still fits the panel «Общие».
    //
    // ⚠ **94 units, and it was 66 until 2026-09-01 and 80 for the length of one delivery.**
    // Decision В-1 (question 55) legitimised the growth the «Оформление» row of FR-92а cost and
    // wrote 66 down as the canon *of that moment*; it did not make 66 a principle. The sound
    // switch of FR-100 is one more row and cost 14 more units, by the user's own word: «место
    // под расширение вниз модуля общие более чем достаточно». Решение 82.2 brings the selection
    // switch of FR-61 here from the group that was dissolved around it — one more row, 14 more
    // units, and the same word covers it. The window IS re-cut this time, but not by this
    // block: решение 83 cut the gap over «Состояние», and the window came out shorter, not
    // taller.
    let (_, panel_top, _, panel_bottom) = template.rect_of(1090);

    assert_eq!(
        (panel_top, panel_bottom),
        (7, 101),
        "«Общие» is 7..101 dialog units — В-1, the sound row of FR-100 and the selection row \
         of решение 82.2"
    );

    let base = vertical_base_unit();

    // The closed part of a combo box is the twelve units of task T-12-2 at most — 22 px of the
    // 22,5 the mock-ups draw — whatever the template declares for the dropped-down list below
    // it. That upper bound is the height that has to fit inside the panel.
    let closed_at_most = vertical_units(settings::FIELD_BOX_DLU, base);
    let row_bottom = vertical_units(combo_top, base) + closed_at_most;

    println!(
        "«Оформление»: {}..{row_bottom} px, панель до {} px",
        vertical_units(combo_top, base),
        vertical_units(panel_bottom, base)
    );

    assert!(
        row_bottom <= vertical_units(panel_bottom, base),
        "the appearance row has to fit inside the panel it lives in"
    );
}

/// **Defect Д-3 of the Э12 comparison** — the frame of the exclusion list and the frame of the
/// «Удалить» button beside it land on the same row of pixels.
///
/// Declared level in the mock-ups (`ui.ps1`: both at y = 160) they came out two pixels apart,
/// because the frame of a list is drawn [`theme::LIST_FIRST_ROW_TOP`] mock-up pixels above
/// the control — the air of the picture before its first row, task T-11-16 — where a button
/// frames its own rectangle. The list is what moves: one unit down, one unit shorter, so the
/// bottom of the box stays on the row it was on.
#[test]
fn the_exclusion_list_and_its_button_wear_their_frames_on_one_row() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let (_, list_top, _, list_bottom) = template.rect_of(1050);
    let (_, button_top, _, _) = template.rect_of(1053);

    let base = vertical_base_unit();

    // `scaled(LIST_FIRST_ROW_TOP, 96)` — the mock-up pixels of T-11-16 in the pixels of a
    // 96 DPI window, which is what the stand measures on.
    let lifted = theme::scaled(theme::LIST_FIRST_ROW_TOP, 96).max(1);

    let list_frame = vertical_units(list_top, base) - lifted;
    let button_frame = vertical_units(button_top, base);

    println!(
        "список (1050) y={list_top} → рамка {list_frame} px; кнопка (1053) y={button_top} → \
         рамка {button_frame} px; поднятие {lifted} px"
    );

    // ⚠ **One pixel of slack since решение 87 п. 3, and it is arithmetic, not a defect.** The
    // cure of Д-3 is «one dialog unit down», and it worked exactly while one vertical unit was
    // 1,875 px — near enough to the {lifted} px lift that the rounding hid the difference. The
    // 10 pt dialog font of Т-26-3 makes the unit **2,125 px**, and no whole number of units is
    // 2 px any more: the achievable positions are multiples of 2,125 and the lift is a fixed
    // count of mock-up pixels. The two frames therefore land at most one pixel apart, and on
    // the raised window at 96 DPI they read as level (`scratchpad-Э26\врез-рамки.png`, 6×).
    //
    // Д-3 itself was **two** pixels and is not back: what is asserted is the distance, not an
    // equality that the unit can no longer deliver at every DPI.
    assert!(
        (list_frame - button_frame).abs() <= 1,
        "the frame of the list is drawn {lifted} px above the control and the button frames \
         its own rectangle: they may differ by the one pixel the 2,125 px unit cannot spend, \
         and no more — list {list_frame} px against button {button_frame} px (defect Д-3)"
    );

    // And the shape of the cure, which is what survived the re-layout of task Т-23-2: the
    // list stands **one unit** below the button and gives that unit back out of its own
    // height, so its bottom is where a list declared level with the button would have ended.
    // The absolute numbers moved with the group — 161/43 became 20/163 when «Исключения» went
    // to the top of the column and grew (решение 83 п. 1) — but the one unit did not, and it
    // is the whole of Д-3.
    assert_eq!(
        list_top - button_top,
        1,
        "the list stands one unit below the button, which is what puts their frames on one row"
    );
    assert_eq!(
        list_bottom,
        button_top + 1 + (list_bottom - list_top),
        "the unit the list gave up at the top it takes back nowhere else"
    );
    assert_eq!(
        (list_top, list_bottom),
        (20, 183),
        "«Исключения» stands at the top of the right column and its list is 163 units — \
         решение 83 п. 1, the user's choice at К-1"
    );
}

/// **Defect Д-4 of the Э12 comparison** — the journal path stands at the generator's own y.
#[test]
fn the_journal_path_stands_where_the_generator_puts_it() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let (left, top, right, bottom) = template.rect_of(1062);
    let (_, caption_top, _, _) = template.rect_of(1107);

    println!("путь журнала (1062): {left},{top}..{right},{bottom}; подпись (1107) y={caption_top}");

    // ⚠ The defect was a **step**, not an address: `IDC_LOG_DIR` stood eleven units under its
    // own caption where the generator writes nine (ui.ps1: y = 264 and y = 273), and the path
    // came out two units low. Task Т-23-2 moved the whole «Диагностика» group 21 units up when
    // the right column was re-cut, so the generator's absolute y is no longer where the group
    // is — the nine units between the caption and the path are, and they are what Д-4 was.
    assert_eq!(
        top - caption_top,
        9,
        "ui.ps1 puts the journal path nine units under its caption (y = 264 → y = 273); the \
         eleven it stood at were defect Д-4"
    );
    // ⚠ **x 220 → 230 by решение 87 п. 3**: the whole right column moved ten units to the
    // right when the left one grew, and this control moved with its column. The width and the
    // two lines of height are the generator's still, and they are what the defect was about.
    assert_eq!(
        (left, right - left, bottom - top),
        (230, 186, 16),
        "and the path keeps the generator's width and its two lines of height, at the x its \
         column was moved to"
    );
}

// =========================================================================================
// Э12 волна 2 — межстрочие многострочных подсказок. Task T-12-12, решение В-6.
// =========================================================================================
//
// The pixels below are drawn on the memory sheet of [`Sheet`] with the settings template's own
// face selected into it: no window is created, no message is pumped and **the product is not
// started**. The face matters here in a way it did not for the T-11-18 tests — the whole
// subject is the distance between two lines, and that distance is measured against the face's
// own `tmHeight`.

use windows::Win32::Graphics::Gdi::{DrawTextW, SetBkMode, SetTextColor, TRANSPARENT};
use windows::Win32::System::Threading::{GR_GDIOBJECTS, GetCurrentProcess, GetGuiResources};

/// The rows of `area` that carry ink, as bands of consecutive rows — the same reading the
/// stand's `inkrows.ps1` makes of a screenshot, done here on the sheet.
fn ink_bands(sheet: &Sheet, area: &RECT) -> Vec<(i32, i32)> {
    let mut bands: Vec<(i32, i32)> = Vec::new();

    for y in area.top..area.bottom {
        let inked = (area.left..area.right).any(|x| sheet.grey(x, y) > INKED);

        match (inked, bands.last_mut()) {
            (true, Some(last)) if last.1 == y - 1 => last.1 = y,
            (true, _) => bands.push((y, y)),
            (false, _) => {}
        }
    }

    bands
}

/// The sheet, the template's own face and the rectangle every test of this section paints in.
///
/// The face is created from the **built** template's `DS_SETFONT` declaration through
/// [`manager_logfont`], so it is the face the dialog manager itself would make, not one the
/// test invented.
fn label_sheet() -> (Sheet, Face, TEXTMETRICW, i32) {
    let (font, sheet) = template_font_and_sheet();
    let face = Face::new(manager_logfont(sheet.dc, &font, DEFAULT_QUALITY));
    let metrics = metrics_of(&sheet, &face);

    // SAFETY: `sheet.dc` is the live memory DC of this frame.
    let dpi = unsafe { GetDeviceCaps(Some(sheet.dc), LOGPIXELSY) };

    (sheet, face, metrics, dpi)
}

/// Paints one caption through the product's own [`theme::paint_label`] with a given face.
fn paint_label_in_face(sheet: &Sheet, area: RECT, caption: &str, face: &Face) -> isize {
    let mut text: Vec<u16> = caption.encode_utf16().collect();

    // SAFETY: the brush is made here, used only by the call below and freed here.
    let ground = unsafe { CreateSolidBrush(LABEL_GROUND) };

    // SAFETY: `sheet.dc` holds this sheet's bitmap, `text` and `area` are live locals of this
    // frame, and both `ground` and the face outlive the call.
    // `Reading::Native`, as in every measurement of a label of this program that is not the
    // journal path — task Т-30-2.
    let answer = unsafe {
        theme::paint_label(
            sheet.dc,
            area,
            &mut text,
            theme::LabelStyle {
                ground,
                ink: LABEL_INK,
                face: Some(face.0),
                pitch: None,
                reading: theme::Reading::Native,
            },
        )
    };

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(ground.into()) };

    answer
}

/// Paints one caption the way the module painted **every** label before task T-12-12: ground,
/// then one `DrawTextW` with [`theme::LABEL_TEXT_FORMAT`]. The reference the separating
/// probe below compares against.
fn paint_label_the_old_way(sheet: &Sheet, area: RECT, caption: &str, face: &Face) {
    let mut text: Vec<u16> = caption.encode_utf16().collect();
    let mut text_rect = area;

    // SAFETY: every handle is live and owned by the caller or made and freed right here; the
    // format carries no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the caption is only read.
    unsafe {
        let ground = CreateSolidBrush(LABEL_GROUND);
        FillRect(sheet.dc, &area, ground);
        let _ = DeleteObject(ground.into());

        SetBkMode(sheet.dc, TRANSPARENT);
        SetTextColor(sheet.dc, LABEL_INK);
        let previous = SelectObject(sheet.dc, face.0.into());
        DrawTextW(
            sheet.dc,
            &mut text,
            &raw mut text_rect,
            theme::LABEL_TEXT_FORMAT,
        );
        SelectObject(sheet.dc, previous);
    }
}

/// Every pixel of `area`, for comparing two drawings pixel for pixel.
fn pixels_of(sheet: &Sheet, area: &RECT) -> Vec<i32> {
    (area.top..area.bottom)
        .flat_map(|y| (area.left..area.right).map(move |x| (x, y)))
        .map(|(x, y)| sheet.grey(x, y))
        .collect()
}

/// **Criterion 12 of T-12-12** — the 23 is the generator's arithmetic and not a count of
/// pixels read off `ui-02-graphite.png`.
///
/// `design-tokens.md` §3 calls a number taken from a picture a forbidden class of error, and
/// it is the class that produced T-11-15 and finding F3: the mock-ups are drawn at 140 %, so a
/// number counted on them is 1,4 × too large as a screen length until [`theme::scaled`]
/// divides it back. The chain that produces this one is written where the constant is, and
/// this test holds the chain there.
#[test]
fn the_line_pitch_of_a_wrapped_label_is_the_line_spacing_of_the_generators_face() {
    assert_eq!(
        theme::LABEL_LINE_PITCH,
        23,
        "the pitch of the mock-ups is 1,33 em of the generator's own face — 22,345 mock-up \
         pixels, whose whole steps are 22 and 23 and whose two lines of this label landed on 23"
    );

    // T-14-7: the source of `theme`, because the constant and its derivation moved there with
    // the third slice of finding 24. Deliberately **not** `drawing_source()`: the assertion is
    // that the chain is written **where the constant is**, and a haystack of both files would
    // let a phrase in the other one answer for it.
    let source = theme_module_source();
    let derivation = source
        .split_once("pub const LABEL_LINE_PITCH")
        .expect("the module must declare LABEL_LINE_PITCH")
        .0;

    // Every link of the chain by name: the file it comes from, the two literals that fix the
    // point size, the GDI+ call that does the laying out, and the two face metrics whose ratio
    // is the line spacing. A doc comment that lost any one of them would be a number with a
    // story instead of a derivation.
    for phrase in [
        "ui.ps1",
        "$DPI = 1.4",
        "$FS = 9 * $DPI",
        "DrawString",
        "GetLineSpacing",
        "2724",
        "2048",
    ] {
        assert!(
            derivation.contains(phrase),
            "the doc comment of LABEL_LINE_PITCH must carry «{phrase}» — the derivation from \
             the generator's literals, not a count of pixels on a picture"
        );
    }

    // 12,6 pt is 16,8 px to the em at 96 DPI, and 2724 / 2048 of that is 22,345 px to the
    // line. The rounding of the mock-up literal does not reach the screen: both neighbours of
    // the fraction come back as the same 16 px, which is what В-6 asked for and what the stand
    // measured.
    let em_tenths = 126 * 96 / 72;
    let pitch_tenths = em_tenths * 2724 / 2048;

    println!(
        "12,6 pt = {} tenths of a mock-up pixel to the em; 2724/2048 of it = {} tenths to the \
         line; scaled(22, 96) = {}, scaled(23, 96) = {}",
        em_tenths,
        pitch_tenths,
        theme::scaled(22, 96),
        theme::scaled(theme::LABEL_LINE_PITCH, 96)
    );

    assert!(
        (223..=224).contains(&pitch_tenths),
        "the generator's line spacing must come out at 22,3 mock-up pixels — it came out \
         {pitch_tenths} tenths, so the chain in the doc comment no longer holds"
    );
    assert_eq!(
        theme::scaled(theme::LABEL_LINE_PITCH, 96),
        16,
        "решение В-6: 23 mock-up pixels are 16 pixels at 96 DPI, against the 15 `DrawTextW` \
         advances a line by on its own"
    );
    assert_eq!(
        theme::scaled(22, 96),
        theme::scaled(theme::LABEL_LINE_PITCH, 96),
        "the whole-pixel rounding of the mock-up literal is not what decides the screen: 22 \
         and 23 mock-up pixels are the same 16 screen pixels, and the fraction between them \
         is the 15,96 the arithmetic asks for"
    );

    // And it scales with the DPI of the window, which is the second half of В-6.
    assert!(
        theme::scaled(theme::LABEL_LINE_PITCH, 144) > theme::scaled(theme::LABEL_LINE_PITCH, 96),
        "the pitch has to grow with the DPI of the window — it goes through `scaled` for that"
    );
}

/// **The separating question of T-12-12, as a table** — which labels take the model pitch and
/// which are left exactly as they were drawn before the task.
///
/// [`theme::label_model_pitch`] is the whole of the decision and it is pure, so it can be
/// closed here rather than photographed. Either «no» means the single plain `DrawTextW` of
/// T-11-18, unchanged to the pixel.
#[test]
fn only_a_label_that_really_wrapped_takes_the_model_pitch() {
    // 96 DPI: `DrawTextW` advances a line by tmHeight = 15 and the model asks for 16.
    assert_eq!(
        theme::label_model_pitch(2, 15, 16),
        Some(16),
        "«вступит в силу после перезапуска» is the label this task exists for: two lines, and \
         a model pitch a pixel wider than the natural one"
    );

    // The seventeen one-line labels. This is the row that keeps them still.
    assert_eq!(
        theme::label_model_pitch(1, 15, 16),
        None,
        "a label that did not wrap has no pitch to set, and seventeen of the eighteen \
         OWNER_DRAWN_LABELS are one line — moving them would be «rewrote the label drawing», \
         not «fixed the line pitch»"
    );
    assert_eq!(
        theme::label_model_pitch(0, 15, 16),
        None,
        "no lines at all is no pitch either"
    );

    // Three lines take it as readily as two: nothing here counts to two.
    assert_eq!(
        theme::label_model_pitch(3, 15, 16),
        Some(16),
        "the rule is «it wrapped», not «it wrapped once»"
    );

    // A model pitch that is not wider has nothing to give: `DrawTextW` already advances by
    // `natural`, and squeezing lines together is not what В-6 asked for.
    assert_eq!(
        theme::label_model_pitch(2, 16, 16),
        None,
        "a model pitch equal to the natural one is the drawing that already happens, and one \
         plain call is both cheaper and exact"
    );
    assert_eq!(
        theme::label_model_pitch(2, 17, 16),
        None,
        "a model pitch narrower than the natural one would squeeze the lines together"
    );

    // ⚠ The height of the control is deliberately **not** an input, and this is the row that
    // says why. `IDC_LOG_DIR` is 16 dialog units — 30 px — and two lines at the model pitch
    // reach 1 × 16 + 15 = 31, so a height question would refuse the model pitch to exactly the
    // label the task names as having to get it. Containment is `paint_label_lines`'s job, and
    // it does it by clamping the clip to the rectangle.
    let (lines, natural, model) = (2, 15, 16);
    let journal_path_height = 16 * 15 / 8;
    let ink = (lines - 1) * model + natural;
    println!(
        "IDC_LOG_DIR: 16 dialog units = {journal_path_height} px, {lines} model lines = {ink} px"
    );
    assert!(
        ink > journal_path_height,
        "the arithmetic this row exists for has changed — check that dropping the height \
         question is still the right call"
    );
    assert_eq!(
        theme::label_model_pitch(2, 15, 16),
        Some(16),
        "a journal path that did wrap has to get the model pitch too, short control or not — \
         the code leads both multi-line labels by one rule"
    );
}

/// **Criterion 7 of T-12-12, on real pixels** — a wrapped label's lines stand
/// `scaled(LABEL_LINE_PITCH, dpi)` apart, and not the face's own `tmHeight`.
///
/// The caption is painted through the product's own [`theme::paint_label`] with the
/// template's own face, and the ink is read back band by band exactly as `inkrows.ps1` reads
/// it off a screenshot of the stand.
#[test]
fn the_lines_of_a_wrapped_label_stand_the_model_pitch_apart() {
    let (sheet, face, metrics, dpi) = label_sheet();
    let pitch = theme::scaled(theme::LABEL_LINE_PITCH, dpi);

    println!(
        "sheet at {dpi} DPI: tmHeight = {}, model pitch = {pitch}",
        metrics.tmHeight
    );

    // Nothing to prove on a machine whose face already advances by the model pitch — and the
    // arithmetic says so rather than the test quietly passing.
    if pitch <= metrics.tmHeight {
        println!("the natural line height is already the model pitch — nothing to move");
        return;
    }

    // Two lines' worth of room, and a caption that wraps into exactly two.
    let area = RECT {
        left: 2,
        top: 2,
        right: 46,
        bottom: 2 + 2 * pitch + metrics.tmHeight,
    };

    sheet.clear(LABEL_OUTSIDE);
    assert_eq!(
        paint_label_in_face(&sheet, area, "wrap me now", &face),
        1,
        "a label with text answers «drawn»"
    );

    let bands = ink_bands(&sheet, &area);
    println!("bands: {bands:?}");

    assert_eq!(
        bands.len(),
        2,
        "the caption has to come back as two bands of ink for the pitch between them to be \
         readable at all — it came back as {bands:?}"
    );

    assert_eq!(
        bands[1].0 - bands[0].0,
        pitch,
        "the tops of the two lines are {} px apart against the {pitch} px of решение В-6 — \
         {} px is the natural tmHeight `DrawTextW` advances by on its own, which is the \
         defect TYPO-2-muted-line-pitch",
        bands[1].0 - bands[0].0,
        metrics.tmHeight
    );

    // And the first line did not move: the shift is `index × (pitch − natural)`, which is
    // zero for the first line, so a label's first row is where it always was.
    sheet.clear(LABEL_OUTSIDE);
    paint_label_the_old_way(&sheet, area, "wrap me now", &face);
    let before = ink_bands(&sheet, &area);
    println!("the same caption drawn the old way: {before:?}");

    assert_eq!(
        before.len(),
        2,
        "the reference drawing has to wrap the same caption into two lines too"
    );
    assert_eq!(
        before[0].0, bands[0].0,
        "the first line of a wrapped label must not move by a pixel — only the lines under it do"
    );
    assert_eq!(
        before[1].0 - before[0].0,
        metrics.tmHeight,
        "the reference drawing is the one plain `DrawTextW`, which advances by tmHeight — if \
         it did not, this test would be comparing the change against itself"
    );
}

/// **Criterion 9 of T-12-12, as a test rather than a screenshot** — a label that fits on one
/// line is drawn *pixel for pixel* the way it was drawn before the task.
///
/// The stand answers this with a diff of two screenshots; this answers it without a window, on
/// every machine that runs the battery, and it is the probe that separates «fixed the line
/// pitch» from «rewrote the label drawing». The same caption is painted twice into the same
/// rectangle — once through [`theme::paint_label`] and once through the single plain
/// `DrawTextW` the module used before — and the two bitmaps must be identical.
#[test]
fn a_one_line_label_is_drawn_exactly_as_it_was_before_the_pitch() {
    let (sheet, face, metrics, dpi) = label_sheet();

    println!(
        "sheet at {dpi} DPI: tmHeight = {}, model pitch = {}",
        metrics.tmHeight,
        theme::scaled(theme::LABEL_LINE_PITCH, dpi)
    );

    let area = RECT {
        left: 2,
        top: 2,
        right: 60,
        bottom: 2 + 4 * metrics.tmHeight,
    };

    // Short enough for one line in a rectangle four lines tall — the shape of every one of the
    // seventeen labels the change must not touch.
    sheet.clear(LABEL_OUTSIDE);
    paint_label_in_face(&sheet, area, "Клавиша:", &face);
    let now = pixels_of(&sheet, &area);

    sheet.clear(LABEL_OUTSIDE);
    paint_label_the_old_way(&sheet, area, "Клавиша:", &face);
    let before = pixels_of(&sheet, &area);

    let moved = now
        .iter()
        .zip(before.iter())
        .filter(|(a, b)| a != b)
        .count();

    println!("{} pixels compared, {moved} of them differ", now.len());

    assert_eq!(
        moved, 0,
        "a one-line label came out different from the plain drawing in {moved} pixels — the \
         change reached the seventeen labels it had no business reaching"
    );

    // And the sheet really carried ink, so the comparison was not two empty rectangles.
    assert!(
        !ink_bands(&sheet, &area).is_empty(),
        "the caption has to leave ink for the comparison above to mean anything"
    );
}

/// **Trap 6 of T-12-12, on the one instrument that can see it** — drawing a wrapped label over
/// and over leaks no GDI object.
///
/// A leaked region gives no error, no wrong pixel and no red test; it shows on the process's
/// object counter and nowhere else. The line by line drawing narrows a clip for every line of
/// every repaint, so a region created and not deleted there would be a handle lost several
/// times a second for as long as the window is open — until the 10 000-object quota takes the
/// whole process down.
///
/// The counter is read before and after two thousand paints of a two-line label. It counts the
/// **whole process**, and the tests of this binary run in parallel threads that hold sheets,
/// faces and brushes of their own, so the bound below is deliberately loose — a few hundred
/// rather than a few. It is still decisive: the smallest leak it has to catch is one object per
/// paint, which is two thousand, and the one actually at stake is one per line per paint, which
/// is four thousand. Ten times the bound either way.
#[test]
fn drawing_a_wrapped_label_over_and_over_leaks_no_gdi_object() {
    let (sheet, face, metrics, dpi) = label_sheet();
    let pitch = theme::scaled(theme::LABEL_LINE_PITCH, dpi);

    let area = RECT {
        left: 2,
        top: 2,
        right: 46,
        bottom: 2 + 2 * pitch + metrics.tmHeight,
    };

    // Warm every allocation the first call makes — a face realised into the DC, a scratch of
    // the text engine — so the reading below is of the loop and not of the first call.
    for _ in 0..8 {
        paint_label_in_face(&sheet, area, "wrap me now", &face);
    }

    let before = gdi_objects();

    const PAINTS: i32 = 2_000;
    for _ in 0..PAINTS {
        paint_label_in_face(&sheet, area, "wrap me now", &face);
    }

    let after = gdi_objects();

    println!(
        "GDI objects: {before} before {PAINTS} paints, {after} after — growth {}",
        after as i64 - before as i64
    );

    assert!(
        after <= before + 200,
        "the process held {before} GDI objects before {PAINTS} paints of a wrapped label and \
         {after} after: the drawing is leaking an object per paint, which is what \
         `SaveDC`/`RestoreDC` exists here to make impossible"
    );
}

/// The count of GDI objects this process holds — the only instrument a leaked region shows on.
fn gdi_objects() -> u32 {
    // SAFETY: `GetCurrentProcess` answers a pseudo-handle that needs no closing, and the call
    // reads a counter of this process and touches no memory of ours. NFR-13: a zero answer
    // means the counter could not be read, and the caller's comparison survives it — zero
    // before and zero after is «no growth», which is the honest reading of «cannot tell».
    unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
}

// =========================================================================================
// FR-94, tasks Т-28-2, Т-28-5 and Т-30-1 — the fourteen locales: tier one of решение 93 plus
// the two right-to-left locales of вопрос 97.
// =========================================================================================

/// Every locale: the value section 7 stores, the `LANGID` `app.rc` tags the string table with,
/// and the name the language combo box shows.
///
/// Written out here rather than imported from the crate on purpose, exactly as the interface
/// strings are: what a test must not borrow from the code under test is the **answer**. The
/// order is the order of the combo box — Russian, English, the ten of решение 93, then Hebrew
/// and Arabic — and it is a contract, not a preference: `Language::index` encodes it and the
/// atomic of `set_ui_language` stores it.
///
/// ⚠ The **length** is taken from `Language::ALL` on purpose, and the number itself is asserted
/// in exactly one place — `the_language_combo_offers_native_names_in_the_order_of_the_array`.
/// A count repeated in seven tests is a count that gets fixed in six of them.
///
/// ⚠ Every `LANGID` here was taken from `LCIDToLocaleName` and not from memory
/// (`scratchpad-Э28\прибор-langid.log` for the twelve, `scratchpad-Э30\прибор-langid.log` for
/// `he` and `ar`, both with a positive control on `0x0FFF`).
const ALL_LOCALES: [(&str, u16, &str); Language::ALL.len()] = [
    ("ru", 0x0419, "Русский"),
    ("en", 0x0409, "English"),
    ("uk", 0x0422, "Українська"),
    ("de", 0x0407, "Deutsch"),
    ("fr", 0x040C, "Français"),
    ("es", 0x040A, "Español"),
    ("pt", 0x0416, "Português (Brasil)"),
    ("it", 0x0410, "Italiano"),
    ("pl", 0x0415, "Polski"),
    ("cs", 0x0405, "Čeština"),
    ("tr", 0x041F, "Türkçe"),
    ("el", 0x0408, "Ελληνικά"),
    ("he", 0x040D, "עברית"),
    ("ar", 0x0401, "العربية"),
];

/// The `Language` a configuration naming `tag` reads back as — through the file, deliberately.
///
/// The parse is the point: `Language` is a **closed** set, and the only honest way to ask
/// whether a value belongs to it is to hand the value to the reader as a file would. A helper
/// that named the variant in Rust would be asking the compiler, not the schema.
fn language_of_tag(tag: &str) -> Language {
    let dir = TestDir::new(&format!("twelve_{tag}"));
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [general]\n\
             language = \"{tag}\"\n"
        ),
    );

    let (config, outcome) = settings::read_or_default(&path);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::Current),
        "`language = \"{tag}\"` must be a value of section 7, not a refusal"
    );

    config.general.language
}

/// **Т-28-2, Т-30-1: all fourteen values of `general.language` survive the file, both ways.**
///
/// Round trip and not just a read: the value has to come back out of `to_toml_string` spelled
/// the same way it went in, or a build that merely *reads* fourteen values would write two.
#[test]
fn every_one_of_the_language_values_survives_a_round_trip() {
    for (tag, _, _) in ALL_LOCALES {
        let language = language_of_tag(tag);

        assert_eq!(
            language.tag(),
            tag,
            "the value read back must be the value written"
        );

        let dir = TestDir::new(&format!("twelve_back_{tag}"));
        let path = dir.config();
        let mut config = Config::default();
        config.general.language = language;

        settings::write_to(&path, &config).expect("the configuration must be writable");

        let text = fs::read_to_string(&path).expect("the written file must be readable");

        assert!(
            text.contains(&format!("language = \"{tag}\"")),
            "the writer must spell `{tag}` back; it wrote:\n{text}"
        );

        let (again, outcome) = settings::read_or_default(&path);

        assert_eq!(outcome.as_ref().ok(), Some(&ReadOutcome::Current));
        assert_eq!(again.general.language, language, "round trip of `{tag}`");
    }
}

/// **Т-28-2: every locale is joined to the `LANGID` its string table is tagged with.**
///
/// The joint between `app.rc` and `settings.rs` is these twelve numbers and nothing else — the
/// two files share no header — so the test states them itself.
#[test]
fn every_locale_carries_the_langid_its_string_table_is_tagged_with() {
    for (tag, langid, _) in ALL_LOCALES {
        assert_eq!(
            language_of_tag(tag).langid(),
            langid,
            "locale `{tag}` must ask the resource loader for 0x{langid:04X}"
        );
    }
}

/// **Т-28-5: not one hole in any of the twelve tables — the completeness sweep.**
///
/// Walks every identifier of [`settings::INTERFACE_STRINGS`] in every one of the twelve
/// locales of the **built** binary. A locale whose table is missing altogether fails inside
/// `resource_of_language`; a row missing from a table that exists comes back empty and fails
/// here. Both are the same defect seen at two distances, and neither is visible at build time:
/// `rc.exe` does not mind a table with holes in it.
#[test]
fn every_locale_carries_every_interface_string() {
    let product = ProductImage::shared();
    let mut empty: Vec<String> = Vec::new();

    for (tag, langid, _) in ALL_LOCALES {
        for id in settings::INTERFACE_STRINGS {
            let string = product.string_of_langid(langid, id);

            if string.trim().is_empty() {
                empty.push(format!("{tag} ({langid:#06X}) has no string {id}"));
            }
        }
    }

    assert!(
        empty.is_empty(),
        "every one of the twelve tables must carry all {} identifiers; holes:\n{}",
        settings::INTERFACE_STRINGS.len(),
        empty.join("\n")
    );
}

/// **Task Т-31-3 — the retired number 3004 is a hole, and a hole costs nothing.**
///
/// Решение 99.4 took `IDS_LANGUAGE_RESTART` out of all fourteen tables and left its number
/// where it was, because a string identifier that moved would change what an **installed** build
/// shows. Three facts, measured on the built binary rather than reasoned about:
///
/// 1. `INTERFACE_STRINGS` no longer names 3004, so the completeness sweep above — the one that
///    would have failed on an empty row — does not walk it. The vocabulary is a list of
///    identifiers *in use*, not a range.
/// 2. The row really is gone from every one of the fourteen tables. A sweep that only checked
///    the crate's list would pass a resource that still carried the string.
/// 3. **The hole does not take its neighbours with it.** Strings live sixteen to a resource and
///    3004 sits in the middle of the block 3000..3015; `settings::string_from` walks that block
///    by index, so a row that stopped short would silently empty everything after it. 3003 and
///    3005 — the two immediate neighbours — and 3015, the last of the block, are read back in
///    all fourteen locales.
#[test]
fn the_retired_restart_string_is_a_hole_and_the_block_reads_across_it() {
    const RETIRED: u16 = 3004;

    assert!(
        !settings::INTERFACE_STRINGS.contains(&RETIRED),
        "the vocabulary of the interface must not name a retired identifier"
    );
    assert_eq!(
        settings::INTERFACE_STRINGS.len(),
        203,
        "two hundred and three identifiers in use — the mandate of Э32 authorised the canon \
         of seventy-three away («канон INTERFACE_STRINGS растёт с 73»), and the growth is the \
         sixty-one strings of the letters from the author (FR-101…FR-103, task Т-32-3), the \
         ten of the two letters out of the feed (Т-32-6) and the fifty-nine of the wizard \
         (FR-104, Т-32-8)"
    );

    let product = ProductImage::shared();
    let mut still_there: Vec<String> = Vec::new();
    let mut empty_neighbours: Vec<String> = Vec::new();
    let mut read = 0;

    for (tag, langid, _) in ALL_LOCALES {
        let retired = product.string_of_langid(langid, RETIRED);

        if !retired.trim().is_empty() {
            still_there.push(format!("{tag} ({langid:#06X}) still carries «{retired}»"));
        }

        // The two neighbours of the hole and the last row of the same sixteen-string block.
        for id in [3003_u16, 3005, 3015] {
            let string = product.string_of_langid(langid, id);
            read += 1;

            if string.trim().is_empty() {
                empty_neighbours.push(format!("{tag} ({langid:#06X}) has no string {id}"));
            }
        }
    }

    println!("--- {read} neighbour rows read across the hole at {RETIRED} ---");

    assert!(
        still_there.is_empty(),
        "the row was retired from every table, not only from the crate's list:\n{}",
        still_there.join("\n")
    );
    assert!(
        empty_neighbours.is_empty(),
        "a hole in the middle of a sixteen-string block must not empty the rows after it:\n{}",
        empty_neighbours.join("\n")
    );
    assert_eq!(
        read,
        ALL_LOCALES.len() * 3,
        "three rows in each of the fourteen locales — «no holes» out of no readings is the \
         cheapest lie such a sweep tells"
    );
}

/// The last schema a build that knew **two** locales could have stamped a file with.
///
/// Task Т-28-2 measured what such a build does when it meets one of the ten locales решение 93
/// added: nothing in the file told it the file was newer, so the strict parse refused the
/// document, `claimed_schema_version` answered the same 3 it carried itself, and the answer was
/// [`SavePolicy::QuarantineFirst`] — the user's configuration moved to `.bad` and replaced by
/// defaults. That was the measured downgrade cost of решение 93, and вопрос **94.1** is the
/// decision to stop paying it.
const LAST_TWO_LOCALE_SCHEMA: u32 = 3;

/// The last schema a build that knew **twelve** locales could have stamped a file with — the one
/// вопрос 94.1 bought, and the one вопрос 97.3 now leaves behind.
///
/// Exactly the same cost, one tier up: `he` and `ar` are two more values of a **closed** enum, so
/// a twelve-locale build meeting `language = "he"` under an unchanged stamp would refuse the
/// document, read its own schema number back, and quarantine the user's configuration. Раising
/// the schema to five is what stops it — that is решение 97.3, and the reason the schema of the
/// letters window of the parallel window moves on to six.
const LAST_TWELVE_LOCALE_SCHEMA: u32 = 4;

/// **Т-30-5, решение 97.3 — the Т-29-1 flip on the numbers of this stage: a twelve-locale build
/// meeting a Hebrew file refuses to write instead of quarantining.**
///
/// The same three steps and the same shape, with the pair that вопрос 97 creates in the world:
/// «a stamp I am too young for» + «`he`, a value I cannot parse». Modelled with
/// `CURRENT_SCHEMA_VERSION + 1` and `zz` for the reason the older test gives — a test may not
/// install an older build — and the **real** pair is asserted where it can be: what this build
/// writes for `he` carries a stamp no twelve-locale build ever wrote.
#[test]
fn a_twelve_locale_build_meeting_a_right_to_left_file_refuses_to_write() {
    // --- 1. what this build writes for one of the two locales вопрос 97 added ---------------
    let mut ours = Config::default();
    ours.general.language = Language::He;

    let text = ours.to_toml_string().expect("the file must serialise");

    assert!(
        text.contains("language = \"he\""),
        "the file carries the locale it was given: {text}"
    );

    // A `const` block for the reason the older test states: both sides are constants, so the day
    // somebody lowers the schema this stops being a red test and becomes a build that does not
    // compile — the right weight for the one number решение 97.3 bought.
    const {
        assert!(
            CURRENT_SCHEMA_VERSION > LAST_TWELVE_LOCALE_SCHEMA,
            "a file naming `he` or `ar` must be stamped with a schema no twelve-locale build \
             had — that is решение 97.3, and without it such a build quarantines the file"
        );
    }
    assert!(
        text.contains(&format!("schema_version = {CURRENT_SCHEMA_VERSION}")),
        "and the stamp is in the file, not merely in memory: {text}"
    );

    // --- 2. the reader that is too old for that stamp ---------------------------------------
    let dir = TestDir::new("rtl_downgrade_meets_the_stamp");
    let newer = CURRENT_SCHEMA_VERSION + 1;
    let original = format!(
        "schema_version = {newer}\n\
         \n\
         [general]\n\
         language = \"zz\"\n"
    );
    let path = write_file(&dir, &original);

    let (config, outcome) = settings::read_or_default(&path);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::FromNewerSchema { version: newer }),
        "the stamp is read before the refused parse is called a verdict"
    );
    assert_eq!(
        SavePolicy::for_read(&outcome),
        SavePolicy::Forbidden,
        "and a file from the future is never written back"
    );
    assert_eq!(
        fs::read_to_string(&path).expect("the file must still be readable"),
        original,
        "the bytes are exactly where the person left them"
    );
    assert_eq!(
        dir.entries(),
        [CONFIG_FILE_NAME],
        "and no `.bad` was made — this is what решение 97.3 bought"
    );
    assert_eq!(
        config.schema_version, newer,
        "the stamp is carried, not reset"
    );

    // --- 3. the control, which must not move: a file of THIS schema naming `he` is current ---
    //
    // The other half of the truth. Raising the schema must not turn the new locales into
    // strangers to the build that has them: `he` under the current stamp is an ordinary file.
    let mine = TestDir::new("rtl_current_schema");
    let good = write_file(
        &mine,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [general]\n\
             language = \"he\"\n"
        ),
    );

    let (config, outcome) = settings::read_or_default(&good);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::Current),
        "a file of this schema naming `he` is a current file and nothing else"
    );
    assert_eq!(config.general.language, Language::He);
    assert_eq!(
        SavePolicy::for_read(&outcome),
        SavePolicy::Allowed,
        "and it may be written back"
    );
}

/// **Т-29-1, вопрос 94.1 — the flip of the Т-28-2 measurement: a downgrade now refuses to write
/// instead of quarantining.**
///
/// Three steps, and they are one contract.
///
/// 1. **This build stamps a twelve-locale file with a schema the two-locale builds never had.**
///    That is the whole of what raising the schema buys, and it is the half that could not be
///    true before this task: on schema 3 a file of the twelve was indistinguishable, by its
///    stamp, from a file of the two.
/// 2. **A reader too old for that stamp answers [`SavePolicy::Forbidden`] and leaves the bytes
///    alone** — no rewrite, and no `.bad` beside them. Modelled with `CURRENT_SCHEMA_VERSION + 1`
///    and a value no build of this line admits, because a test may not install an older build:
///    the pair «a stamp I am too young for» + «a value I cannot parse» is exactly the shape a
///    two-locale build sees in a file of the twelve, and `Config::from_toml_str` decides it by
///    the stamp **before** the refused parse becomes a verdict.
/// 3. **The control, which must not move**: the *same* stamp with a value this build does not
///    admit is still `QuarantineFirst`. The stamp is what tells a file from the future apart
///    from a file that is simply damaged, and raising the schema must not blur that.
#[test]
fn a_downgrade_meets_the_new_stamp_and_refuses_to_write_rather_than_quarantining() {
    // --- 1. what this build writes for one of the ten locales решение 93 added --------------
    let mut ours = Config::default();
    ours.general.language = Language::De;

    let text = ours.to_toml_string().expect("the file must serialise");

    assert!(
        text.contains("language = \"de\""),
        "the file carries the locale it was given: {text}"
    );
    // A `const` block on purpose, and not only because clippy asks: both sides are constants,
    // so the day somebody lowers the schema this stops being a red test and becomes a build
    // that does not compile — which is the right weight for the one number вопрос 94.1 bought.
    const {
        assert!(
            CURRENT_SCHEMA_VERSION > LAST_TWO_LOCALE_SCHEMA,
            "a file of the twelve locales must be stamped with a schema no two-locale build \
             had — that is вопрос 94.1, and without it the downgrade cost of Т-28-2 stands"
        );
    }
    assert!(
        text.contains(&format!("schema_version = {CURRENT_SCHEMA_VERSION}")),
        "and the stamp is in the file, not merely in memory: {text}"
    );

    // --- 2. the reader that is too old for that stamp ---------------------------------------
    let dir = TestDir::new("downgrade_meets_the_stamp");
    let newer = CURRENT_SCHEMA_VERSION + 1;
    let original = format!(
        "schema_version = {newer}\n\
         \n\
         [general]\n\
         language = \"zz\"\n"
    );
    let path = write_file(&dir, &original);

    let (config, outcome) = settings::read_or_default(&path);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::FromNewerSchema { version: newer }),
        "the stamp is read before the refused parse is called a verdict"
    );
    assert_eq!(
        SavePolicy::for_read(&outcome),
        SavePolicy::Forbidden,
        "and a file from the future is never written back"
    );
    assert_eq!(
        config.schema_version, newer,
        "the stamp is carried, not reset"
    );
    assert_eq!(
        fs::read_to_string(&path).expect("the file must still be readable"),
        original,
        "the bytes are exactly where the person left them"
    );
    assert_eq!(
        dir.entries(),
        [CONFIG_FILE_NAME],
        "and no `.bad` was made — this is what 94.1 bought"
    );

    // --- 3. the control: the same stamp, and a value this build does not admit ---------------
    let damaged = TestDir::new("same_stamp_unknown_value");
    let broken = write_file(
        &damaged,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [general]\n\
             language = \"zz\"\n"
        ),
    );

    let (config, outcome) = settings::read_or_default(&broken);

    assert!(
        matches!(outcome, Err(ConfigError::Malformed { .. })),
        "a closed set must refuse a value outside it: {outcome:?}"
    );
    assert_eq!(
        SavePolicy::for_read(&outcome),
        SavePolicy::QuarantineFirst,
        "a damaged file of this schema is still damaged — the stamp is what tells the two apart"
    );
    assert_eq!(
        config.general.language.tag(),
        "ru",
        "what came back in its place is the default of section 7"
    );
}

/// **Т-28-2, Т-30-1: the combo offers the native names, in the order of the array.**
///
/// Three things at once, and they are one contract: the name shown, the position it stands at,
/// and the value that position reads back as. A build that showed «Deutsch» at position three
/// and stored Ukrainian for it would pass any two of the three checks separately.
///
/// ⚠ **The one place the count itself is a literal.** Everything else takes its length from
/// `Language::ALL`; here the number is the thing under test, so it is typed out — twelve of
/// решение 93 plus the two of вопрос 97.
#[test]
fn the_language_combo_offers_native_names_in_the_order_of_the_array() {
    assert_eq!(
        Language::ALL.len(),
        14,
        "решение 93 names twelve locales for tier one, вопрос 97 adds Hebrew and Arabic"
    );

    for (position, (tag, _, native)) in ALL_LOCALES.iter().enumerate() {
        let language = language_of_tag(tag);
        let index = u32::try_from(position).expect("a locale count fits a u32");

        assert_eq!(
            language.native_name(),
            *native,
            "the combo shows `{tag}` under its own name"
        );
        assert_eq!(
            language.index(),
            index,
            "`{tag}` stands at position {position}"
        );
        assert_eq!(
            Language::from_index(index),
            language,
            "and position {position} reads back as `{tag}`"
        );
        assert_eq!(
            Language::ALL[position],
            language,
            "the array itself holds `{tag}` at {position}"
        );
    }

    // Out of range in both directions is the default of section 7, not a panic and not a
    // neighbouring language: this is what a `CB_ERR` of −1 arrives as.
    assert_eq!(Language::from_index(14), Language::Ru);
    assert_eq!(Language::from_index(u32::MAX), Language::Ru);
}

/// **Т-28-2, Т-30-1: the names are distinct, and each starts the way its own script can.**
///
/// Решение 92 established the capital first letter for the *layout* list; решение 93 keeps the
/// rule for the *language* list, where the names are literals and the capital is simply typed.
/// A duplicate would be worse than ugly — two rows of a chooser reading the same word give the
/// user no way to pick.
///
/// ⚠ **Rewritten honestly by task Т-30-1.** Hebrew and Arabic have **no case at all**:
/// `is_uppercase` is `false` for «ע» and «ا» not because the name is mis-typed but because the
/// question does not apply to those scripts. Demanding a capital of them would be a test that
/// can only be satisfied by writing the name wrong. So the rule is split at the only line that
/// is real: a script that *has* case must use it, and a script that has none must still start
/// with a letter **of its own block** — which is the half that catches a name gone to Latin or
/// to question marks, and that is what the capital check was really buying.
#[test]
fn the_native_names_are_distinct_and_start_in_their_own_script() {
    let mut seen: Vec<&str> = Vec::new();

    for language in Language::ALL {
        let name = language.native_name();
        let first = name.chars().next().expect("a name is not empty");

        if language.is_rtl() {
            // The two blocks of вопрос 97: Hebrew U+0590…U+05FF, Arabic U+0600…U+06FF.
            let own_block = matches!(first, '\u{0590}'..='\u{05FF}' | '\u{0600}'..='\u{06FF}');

            assert!(
                own_block,
                "«{name}» must begin with a letter of its own script, not `{first}`"
            );
            assert!(
                !first.is_uppercase() && !first.is_lowercase(),
                "`{first}` is expected to be caseless — if this fails, the name is not the \
                 script it claims to be"
            );
        } else {
            assert!(
                first.is_uppercase(),
                "«{name}» must start with a capital: решение 92, kept by решение 93"
            );
        }

        assert!(
            !seen.contains(&name),
            "«{name}» appears twice in the language chooser"
        );

        seen.push(name);
    }

    assert_eq!(seen.len(), Language::ALL.len());
}

/// **Т-28-2: the dialog fills the combo from the array rather than from a list of its own.**
///
/// A source sweep, like the sweeps this file already keeps over `read_dialog`: the two literals
/// that used to stand here — `"Русский"` and `"English"` — must be gone from the module, or a
/// thirteenth locale would be added to the enum and quietly not appear in the window.
#[test]
fn the_dialog_names_no_language_of_its_own() {
    let source = settings_module_source();

    for line in source.lines().map(str::trim) {
        if line.starts_with("//") || line.starts_with("///") {
            continue;
        }

        assert!(
            !line.contains("combo_add(hwnd, IDC_LANGUAGE, \""),
            "the language combo must be filled from `Language::ALL`, not from a literal: {line}"
        );
    }

    assert!(
        source.contains("for language in Language::ALL {"),
        "and it is filled by walking the array"
    );
}

/// **Т-28-3: the autostart row reads the same in the dialog and in the tray menu.**
///
/// One action reachable from two places, so one wording. The two identifiers are separate on
/// purpose (they belong to different windows), and that is exactly why a test has to hold them
/// together — nothing else would notice them drifting apart in the tenth locale.
#[test]
fn the_autostart_row_reads_the_same_in_the_dialog_and_in_the_menu() {
    let product = ProductImage::shared();

    for (tag, langid, _) in ALL_LOCALES {
        assert_eq!(
            product.string_of_langid(langid, settings::IDS_AUTOSTART),
            product.string_of_langid(langid, settings::IDS_MENU_AUTOSTART),
            "locale `{tag}` words the same action twice"
        );
    }
}

/// **Т-28-3: every locale keeps every placeholder of the row it translates.**
///
/// `format_text` substitutes by number, so a translation is free to move `{0}` and `{1}` about
/// — and is not free to lose one or invent one. A lost `{0}` shows the user a help line that
/// never names their key; an invented `{2}` shows them a literal `{2}`. Neither is visible at
/// build time and neither is visible to the completeness sweep.
#[test]
fn every_locale_keeps_the_placeholders_of_the_row_it_translates() {
    let product = ProductImage::shared();
    let mut wrong: Vec<String> = Vec::new();

    for id in settings::INTERFACE_STRINGS {
        let russian = product.string_of_langid(0x0419, id);
        let wanted = placeholders_of(&russian);

        for (tag, langid, _) in ALL_LOCALES {
            let mine = placeholders_of(&product.string_of_langid(langid, id));

            if mine != wanted {
                wrong.push(format!(
                    "{tag}: string {id} carries {mine:?} where Russian carries {wanted:?}"
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "a translation may move a placeholder and may not lose or invent one:\n{}",
        wrong.join("\n")
    );
}

/// Which of `{0}`, `{1}`, `{2}` a string carries, sorted — the order inside the string is the
/// translator's business, the set is not.
fn placeholders_of(text: &str) -> Vec<&'static str> {
    ["{0}", "{1}", "{2}"]
        .into_iter()
        .filter(|slot| text.contains(slot))
        .collect()
}

/// **Т-28-3, Т-30-1: the product name is not translated in any of the fourteen.**
///
/// The decision on question 7, now with twelve more chances to be broken: «Lang Switcher» is the
/// display name of the product, and it is the same three words under `HKCU\…\Run` and in the
/// caption of every locale.
#[test]
fn the_product_name_is_untranslated_in_every_locale() {
    let product = ProductImage::shared();

    for (tag, langid, _) in ALL_LOCALES {
        let caption = product.string_of_langid(langid, settings::IDS_DIALOG_CAPTION);

        assert!(
            caption.starts_with("Lang Switcher"),
            "locale `{tag}` renamed the product: «{caption}»"
        );
    }
}

/// **Т-30-1: not one translation carries a direction mark — the guard the glossary asks for.**
///
/// Direction is decided by the **code**: the window is mirrored by the extended style its
/// template is created with (Т-30-2), and an island of Latin text is turned back to left-to-
/// right reading by one function (Т-30-4). A `U+200E` or `U+200F` buried in a translation would
/// make an island silently — the string would look perfectly ordinary in the `.rc` and in every
/// other sweep, and the one place it showed would be a window nobody thought to look at.
///
/// The isolates `U+2066…U+2069` and the deprecated embedding controls `U+202A…U+202E` are here
/// for the same reason: they are the other ways to say the same thing invisibly.
///
/// ⚠ This sweep reads the **built binary**, not the `.rc` — a mark that survived the resource
/// compiler is the one that matters, and a mark the compiler ate never reaches a user.
#[test]
fn no_translation_carries_a_direction_mark() {
    let product = ProductImage::shared();
    let mut marked: Vec<String> = Vec::new();

    for (tag, langid, _) in ALL_LOCALES {
        for id in settings::INTERFACE_STRINGS {
            let string = product.string_of_langid(langid, id);

            for (at, unit) in string.char_indices() {
                let bad = matches!(
                    unit,
                    '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
                );

                if bad {
                    marked.push(format!(
                        "{tag}: string {id} carries U+{:04X} at byte {at}",
                        unit as u32
                    ));
                }
            }
        }
    }

    assert!(
        marked.is_empty(),
        "direction belongs to the code, not to the strings; found:\n{}",
        marked.join("\n")
    );

    // The instrument must be able to fail: a mark in a string it *would* look at is caught.
    let planted = "Lang Switcher \u{200F}— הגדרות";

    assert!(
        planted
            .chars()
            .any(|unit| matches!(unit, '\u{200E}' | '\u{200F}')),
        "positive control: the predicate above finds a planted mark"
    );
}

/// **Т-30-2: the mirror is added to the template, and only to a template that can carry it.**
///
/// The lever of вопрос 97: `SetProcessDefaultLayout` was measured first and does not mirror a
/// dialog that has an owner — which both of this program's do — so the window is mirrored by
/// the extended style of a **copy** of its compiled template, run through
/// `DialogBoxIndirectParamW` (`scratchpad-Э30\посылки-п1.log`).
///
/// What is checked here is the patch itself, over a synthetic `DLGTEMPLATEEX` header: the bit
/// goes in, the rest of the extended style is kept, and the whole thing is refused for a buffer
/// that is not the extended form. That last half is the reason the function exists — the old
/// `DLGTEMPLATE` keeps `style` where the extended one keeps `signature`, so a patch applied
/// without looking would corrupt a window style instead of adding one, and there is no template
/// in this program's own resources to catch that with.
#[test]
fn the_mirror_goes_into_the_template_and_only_into_an_extended_one() {
    // `dlgVer` = 1, `signature` = 0xFFFF, `helpID` = 0, `exStyle` = WS_EX_APPWINDOW — the
    // header `rc.exe` produces for both `DIALOGEX` statements of `app.rc`.
    let header = |version: u16, signature: u16, exstyle: u32| {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&signature.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&exstyle.to_le_bytes());
        bytes.extend_from_slice(&[0; 32]);
        bytes
    };

    const WS_EX_LAYOUTRTL: u32 = 0x0040_0000;
    const WS_EX_APPWINDOW: u32 = 0x0004_0000;

    let mut template = header(1, 0xFFFF, WS_EX_APPWINDOW);

    assert_eq!(
        settings::mirror_template(&mut template),
        Some((WS_EX_APPWINDOW, WS_EX_APPWINDOW | WS_EX_LAYOUTRTL)),
        "the bit is added and the style that was there is kept"
    );

    assert_eq!(
        u32::from_le_bytes([template[8], template[9], template[10], template[11]]),
        WS_EX_APPWINDOW | WS_EX_LAYOUTRTL,
        "and it is written back into the buffer, not only reported"
    );

    // Idempotent: a template already carrying the bit is not disturbed by a second pass.
    let before = template.clone();

    settings::mirror_template(&mut template);

    assert_eq!(
        template, before,
        "adding a bit that is already there changes nothing"
    );

    // The refusals, each of which is a corrupted style if it is not refused.
    let mut old_form = header(0xFFFF, 0x0001, WS_EX_APPWINDOW);

    assert_eq!(
        settings::mirror_template(&mut old_form),
        None,
        "a DLGTEMPLATE of the old form is not patched at the extended offset"
    );

    let mut wrong_version = header(2, 0xFFFF, WS_EX_APPWINDOW);

    assert_eq!(
        settings::mirror_template(&mut wrong_version),
        None,
        "a version this code has not read the layout of is left alone"
    );

    let mut truncated = vec![1_u8, 0, 0xFF, 0xFF, 0, 0, 0, 0, 0];

    assert_eq!(
        settings::mirror_template(&mut truncated),
        None,
        "SEC-05: a buffer too short for the field is refused rather than indexed into"
    );

    let mut empty: Vec<u8> = Vec::new();

    assert_eq!(settings::mirror_template(&mut empty), None);
}

/// **Т-30-2: exactly the two right-to-left locales ask for a mirror, and no others.**
///
/// `Language::is_rtl` is the one place that knows it — the dialog template, the tray menu and
/// the reading order of an island all read this and nothing else. A sweep over the whole array
/// rather than two `assert!`s, so that a fifteenth locale added without a thought about
/// direction shows up here rather than in a window.
#[test]
fn only_hebrew_and_arabic_are_read_right_to_left() {
    let mirrored: Vec<&str> = Language::ALL
        .into_iter()
        .filter(|language| language.is_rtl())
        .map(Language::tag)
        .collect();

    assert_eq!(
        mirrored,
        vec!["he", "ar"],
        "вопрос 97 names two right-to-left locales and this build has exactly those"
    );

    // And the other twelve are not mirrored — stated from the other side so that a predicate
    // that answered `true` for everything could not pass the check above by accident.
    assert_eq!(
        Language::ALL
            .into_iter()
            .filter(|language| !language.is_rtl())
            .count(),
        12
    );
}

/// **Т-30-4, решение 97.2 (г): a name reads by its own first strong character.**
///
/// The rule the user gave in so many words, and the one that a blanket «names are islands»
/// would break: a layout is named in its own language and so is a locale in the chooser, so a
/// Hebrew name has to stay right to left in an English window exactly as an English name stays
/// left to right in a Hebrew one.
///
/// The names here are not invented: the two layout names this machine actually produces were
/// read off the live product in Э27 (`Русский (Россия)`, `English (United States)`), and the
/// fourteen locale names are `Language::native_name`.
#[test]
fn a_name_reads_by_its_own_first_strong_character() {
    for name in [
        "English (United States)",
        "Русский (Россия)",
        "Ελληνικά",
        "Deutsch",
        "Português (Brasil)",
        "Čeština",
    ] {
        assert_eq!(
            settings::reading_of_name(name),
            theme::Reading::LatinIsland,
            "«{name}» begins with a strong left-to-right letter and reads that way"
        );
    }

    for name in ["עברית", "العربية", "עברית (ישראל)", "العربية (السعودية)"]
    {
        assert_eq!(
            settings::reading_of_name(name),
            theme::Reading::Native,
            "«{name}» begins with a strong right-to-left letter and keeps its own direction"
        );
    }

    // A leading neutral is skipped, not treated as an answer: what decides is the first
    // **strong** character, and a bracket or a space is neither.
    assert_eq!(
        settings::reading_of_name("  (עברית)"),
        theme::Reading::Native,
        "leading spaces and a bracket are neutral and are skipped"
    );
    assert_eq!(
        settings::reading_of_name("(English)"),
        theme::Reading::LatinIsland
    );

    // A name of no strong characters at all reads the way the window does — there is nothing
    // in it that could look wrong either way, and the window's direction is the better default.
    for name in ["", "   ", "123", "— · —"] {
        assert_eq!(
            settings::reading_of_name(name),
            theme::Reading::Native,
            "«{name}» has no strong character and takes no view"
        );
    }
}

/// **Т-30-4: a panel caption in a cursive script is not cut into letters.**
///
/// ⛔ Found by eye on the stand and by nothing else (`scratchpad-Э30\ШАПКА2-ar.png`). The letter
/// spacing of the panel captions is applied by placing **every character with a `TextOutW` of
/// its own**, which is harmless for an alphabet whose letters stand apart and destroys Arabic:
/// its letters change shape according to what they join to, and a cut run comes out as a row of
/// isolated forms. «الاستثناءات» arrived as eleven separate letters.
///
/// Hebrew is refused the spacing too, for the reason that covers both: neither script has
/// capitals, so «capitals, spaced out» is a device with nothing to apply, and letter-spacing
/// either of them is a typographic error rather than a style.
///
/// The captions here are the real ones — the six group headings of the settings window in the
/// two new locales, and their Russian and English counterparts as the control.
#[test]
fn a_caption_in_a_cursive_script_is_drawn_as_one_run() {
    for caption in [
        "الاستثناءات",
        "التخطيطات",
        "عام",
        "التشخيص",
        "الحالة",
        "مفتاح الاختصار",
        "כללי",
        "פריסות",
        "חריגים",
        "אבחון",
        "מצב",
        "מקש קיצור",
    ] {
        assert!(
            !settings::caption_takes_tracking(caption),
            "«{caption}» is written in a script that must not be cut into characters"
        );
    }

    // The control: the captions that have always been spaced must go on being spaced, or the
    // fix above would have quietly taken the mock-ups' typography away from twelve locales.
    for caption in [
        "ОБЩИЕ",
        "GENERAL",
        "ГОРЯЧАЯ КЛАВИША",
        "DIAGNOSTICS",
        "ΓΕΝΙΚΆ",
        "AUSNAHMEN",
    ] {
        assert!(
            settings::caption_takes_tracking(caption),
            "«{caption}» keeps the letter spacing of the mock-ups"
        );
    }
}

/// **Т-30-4, замер: no direction mark reaches the configuration file, and none is needed.**
///
/// The mandate offered a trailing direction mark in the text of a list item as a way of keeping
/// the closing bracket of `English (United States)` from jumping in a mirrored window, with the
/// warning that such a mark must never reach `[layouts]` of `config.toml`.
///
/// **It was measured first and is not needed at all.** Modern bidi resolves a bracket *pair* to
/// the direction of the text inside it, so the closing bracket does not jump: the same string
/// came out identical under both reading orders (`scratchpad-Э30\посылки-п2.log`) and both
/// layout names came out right on the stand, in the combo boxes and in the cycle list
/// (`scratchpad-Э30\ЖИВОЕ-he-настройки.png`). So no mark is inserted anywhere, and this test is
/// the guard that keeps it that way — the cheapest place for one to be added later «to be safe»
/// is exactly the list item that is written back to the file.
#[test]
fn no_layout_name_carries_a_direction_mark_into_the_configuration() {
    let source = settings_module_source();

    for (at, line) in source.lines().enumerate() {
        let line = line.trim();

        if line.starts_with("//") || line.starts_with("///") {
            continue;
        }

        for needle in ["\\u{200E}", "\\u{200F}", "\\u{2066}", "\\u{2069}"] {
            assert!(
                !line.contains(needle),
                "line {}: a direction mark in the module would reach a layout name and, \
                 through it, `[layouts]` of the configuration file — вопрос 97.2 leaves \
                 direction to the code, and the code decides it with `theme::reading_order`",
                at + 1
            );
        }
    }
}

/// **Т-30-1: the arrow of the help line points the way the script reads.**
///
/// `IDS_ABOUT_HELP_5` names two places in order — «Settings» and then «Hotkey» — and the arrow
/// between them leads the eye from the first to the second. In a right-to-left line the first
/// of them stands on the **right**, so the arrow that leads to the second is «←». The rest keep
/// «→». Nothing else in the tables carries an arrow, so this is the whole of the rule.
#[test]
fn the_help_arrow_follows_the_direction_of_the_script() {
    let product = ProductImage::shared();

    for (tag, langid, _) in ALL_LOCALES {
        let line = product.string_of_langid(langid, settings::IDS_ABOUT_HELP_5);
        let language = language_of_tag(tag);

        let (wanted, unwanted) = if language.is_rtl() {
            ('←', '→')
        } else {
            ('→', '←')
        };

        assert!(
            line.contains(wanted),
            "locale `{tag}` must lead the eye with `{wanted}`: «{line}»"
        );
        assert!(
            !line.contains(unwanted),
            "locale `{tag}` points the wrong way with `{unwanted}`: «{line}»"
        );
    }
}

/// **Т-29-2, сторож: the item the user picked in the language combo is the value that lands in
/// the file — for every one of the twelve, and from every one of the twelve.**
///
/// The whole chain the settings dialog walks, expressed in the three contracts that make it up
/// and driven over the full 12 × 12 matrix of «откуда → куда»:
///
/// 1. what `fill_dialog` selects when the dialog opens on a configuration — `Language::index`;
/// 2. what `read_dialog` stores for the item that is selected when «ОК» is pressed —
///    `Language::from_index` of the very same number;
/// 3. what the file then says, and what a later start reads back out of it.
///
/// ⚠ **This guard was written after the reported defect turned out not to exist.** Три
/// «находки о несохранении языка» (83.3, 94.4, м-Э29-1) were the readings of a broken
/// instrument, not of the program — the stand of Э29 drove the real dialog through all 296
/// cases of this matrix and every one of them wrote the file. There is therefore no red «до»
/// behind this test and none is claimed. What it is here for is the **from**-half: the reports
/// all said the failure depended on which locale the dialog was showing, and nothing in the
/// chain may ever start depending on that. A test that only walked `to` could not say so.
#[test]
fn the_locale_picked_in_the_combo_is_the_locale_the_file_ends_up_carrying() {
    let dir = TestDir::new("combo_to_file");

    for from in Language::ALL {
        for to in Language::ALL {
            // 1. The dialog opens on a configuration carrying `from` and selects its position.
            let mut config = Config::default();
            config.general.language = from;

            let selected = config.general.language.index();
            assert_eq!(
                selected,
                from.index(),
                "the combo opens on `{}`",
                from.tag()
            );

            // 2. «ОК»: the item at the position the user left selected becomes the value.
            let picked = to.index();
            config.general.language = Language::from_index(picked);

            assert_eq!(
                config.general.language,
                to,
                "position {picked} must read back as `{}` (dialog showing `{}`)",
                to.tag(),
                from.tag()
            );

            // 3. The file, and the start that reads it again.
            let path = write_file(&dir, &config.to_toml_string().expect("serialises"));

            assert!(
                fs::read_to_string(&path)
                    .expect("readable")
                    .contains(&format!("language = \"{}\"", to.tag())),
                "the file must name `{}` in so many words (dialog showing `{}`)",
                to.tag(),
                from.tag()
            );

            let (back, outcome) = settings::read_from(&path).expect("the file must be read");

            assert_eq!(outcome, ReadOutcome::Current);
            assert_eq!(
                back.general.language,
                to,
                "`{}` → `{}` did not survive the file",
                from.tag(),
                to.tag()
            );
            assert_eq!(
                back.general.language.index(),
                picked,
                "and the next opening of the dialog selects the same item again"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------
// Т-29-3, вопрос 95 — the locale a first run starts in
// ---------------------------------------------------------------------------------------
//
// The order asked for, in the user's own words: «при установке программы у пользователя в
// будущем, нужно автоматически проверять язык системы и устанавливать по умолчанию языком
// интерфейса, если имеется; если нет в списке локализаций нашей программы — по умолчанию
// ставить английский».
//
// ⚠ **This machine cannot accept it honestly.** Its Windows is Russian, so the rule answers
// `ru` — which is also the old hard-coded default, and a green light that would be green
// whatever the rule did. Every test below therefore drives the rule with **staged** tags, and
// the one live test says only what a live test here honestly can: that the machine answers
// *some* one of the twelve, through an instrument that is not able to answer nothing.

/// **The mapping rule of вопрос 95, staged tag by tag.**
///
/// A Windows UI language is a BCP-47 tag — `de-DE`, `pt-BR`, `es-MX` — and section 7 spells a
/// locale in two letters. The bridge is the **primary subtag** and nothing else, which is what
/// makes `de-AT` German and `pt-PT` Portuguese without a table of regions that would have to be
/// extended for every country Windows ships.
#[test]
fn a_system_tag_maps_onto_a_locale_by_its_primary_subtag() {
    for (tag, wanted) in [
        // The plain case, all twelve, in the shape Windows actually hands out.
        ("ru-RU", Language::Ru),
        ("en-US", Language::En),
        ("uk-UA", Language::Uk),
        ("de-DE", Language::De),
        ("fr-FR", Language::Fr),
        ("es-ES", Language::Es),
        ("pt-BR", Language::Pt),
        ("it-IT", Language::It),
        ("pl-PL", Language::Pl),
        ("cs-CZ", Language::Cs),
        ("tr-TR", Language::Tr),
        ("el-GR", Language::El),
        // Т-30-1, вопрос 97 — the two right-to-left locales, by the same rule as the rest.
        ("he-IL", Language::He),
        ("ar-SA", Language::Ar),
        // Arabic is one table for the standard language, so every Arabic region is `ar`. This
        // is the `pt` decision applied a second time, and it is the reason the mapping goes by
        // the tag and not through `langid`: `Ar` is 0x0401, which is `ar-SA` alone.
        ("ar-EG", Language::Ar),
        ("ar-MA", Language::Ar),
        ("he", Language::He),
        ("ar", Language::Ar),
        // The two the order names by hand: a region this build never enumerated.
        ("de-AT", Language::De),
        ("pt-PT", Language::Pt),
        // And the rest of the same rule, which is the point of having a rule.
        ("en-GB", Language::En),
        ("es-MX", Language::Es),
        ("fr-CA", Language::Fr),
        // A bare primary tag is already the answer.
        ("de", Language::De),
        ("el", Language::El),
        // A script subtag sits between the two and must not be mistaken for the region.
        ("uk-Cyrl-UA", Language::Uk),
        // Windows writes them with a hyphen; a file or a registry value may hold an underscore.
        ("pt_BR", Language::Pt),
        // Case is not part of a language tag's meaning.
        ("DE-de", Language::De),
    ] {
        assert_eq!(
            settings::language_of_ui_tag(tag),
            Some(wanted),
            "`{tag}` must map onto `{}`",
            wanted.tag()
        );
    }

    for tag in [
        "zh-CN",
        "ja-JP",
        "ko-KR",
        "nl-NL",
        "sv-SE",
        // ⚠ Т-30-1, recorded as a **fact and not a defect**: `iw` is the retired ISO 639-1 code
        // for Hebrew, which some systems still hand out. This build does not have it, so a
        // machine that answers `iw` gets English by the rule of вопрос 95 — the same road as a
        // language this build never translated. Whether that is worth an alias is a question
        // for the user, not a thing to fix inside a task about mirroring.
        "iw",
        "iw-IL",
        "z",
        "",
        "-",
        "---",
        "не-тег",
    ] {
        assert_eq!(
            settings::language_of_ui_tag(tag),
            None,
            "`{tag}` is not one of the fourteen and must not be pretended to be"
        );
    }

    // And the consequence of the line above, stated where it is visible: the retired code does
    // not fall to Hebrew, it falls to the end of the list.
    assert_eq!(settings::first_run_language(["iw-IL"]), Language::En);
    assert_eq!(settings::first_run_language(["he-IL"]), Language::He);
    assert_eq!(settings::first_run_language(["ar-EG"]), Language::Ar);
}

/// **The fall-through, and the English at the end of it — вопрос 95.**
///
/// Windows keeps an **ordered list**, not one language: a person may have Japanese first and
/// German second. Walking the list is the whole reason [`GetUserPreferredUILanguages`] was
/// chosen over `GetUserDefaultUILanguage`, which answers one identifier and cannot say what the
/// person would have taken instead.
#[test]
fn the_first_preference_this_build_has_wins_and_english_is_the_end_of_the_list() {
    for (preferred, wanted) in [
        // The ordinary case: the first is one of ours.
        (vec!["de-DE", "en-US"], Language::De),
        // The first is not; the second is. That is a better answer than English.
        (vec!["zh-Hans-CN", "de-DE"], Language::De),
        (vec!["ja-JP", "ko-KR", "el-GR"], Language::El),
        // Nothing in the list is ours.
        (vec!["ja-JP", "ko-KR"], Language::En),
        // Windows answered nothing at all — a machine in a state this build cannot read.
        (vec![], Language::En),
        // And the order is respected, not the alphabet: both are ours, the first one wins.
        (vec!["tr-TR", "de-DE"], Language::Tr),
    ] {
        assert_eq!(
            settings::first_run_language(preferred.iter().copied()),
            wanted,
            "for {preferred:?}"
        );
    }
}

/// **The rule reaches the configuration only where there is no file — вопрос 95.**
///
/// Two halves, and the second is the ⚠ of the order: *«существующий конфиг не трогается
/// никогда»*. A file that exists but says nothing about the language is **not** a first run; it
/// is somebody's file, and the value it gets is the default of section 7, exactly as before.
#[test]
fn only_a_first_run_takes_the_language_of_the_system() {
    // --- no file at all: the rule decides -------------------------------------------------
    let dir = TestDir::new("first_run_language");
    let missing = dir.config();
    assert!(!missing.exists(), "the test must start without a file");

    let (config, outcome) = settings::read_from(&missing).expect("a missing file is not an error");

    assert_eq!(outcome, ReadOutcome::NoFile);
    assert_eq!(
        config.general.language,
        settings::system_language_for_a_first_run(),
        "a first run starts in the language of the system"
    );
    assert_eq!(
        config,
        Config::for_a_first_run(),
        "and in nothing else — the rest of section 7 is untouched by вопрос 95"
    );

    // Every other field is the default of section 7. Written as a whole-structure comparison
    // so that a field added later cannot slip through unasserted.
    let mut as_before = Config::default();
    as_before.general.language = config.general.language;
    assert_eq!(config, as_before);

    // --- a file that exists: the rule is not consulted ------------------------------------
    let existing = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [general]\n\
             enabled = false\n"
        ),
    );

    let (config, outcome) = settings::read_from(&existing).expect("the file must be readable");

    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(
        config.general.language,
        Language::Ru,
        "a file with no `language` line is somebody's file, and its missing values come from \
         section 7 — never from the system"
    );

    // The same for a file of an older schema, which travels the ladder on the way in.
    let old = write_file(&dir, "schema_version = 1\n\n[general]\nenabled = false\n");
    let (config, outcome) = settings::read_from(&old).expect("an old file must be readable");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 1 });
    assert_eq!(
        config.general.language,
        Language::Ru,
        "a migration is not a first run either"
    );

    // And a file this build could not read at all falls back to the defaults of section 7,
    // because those bytes are somebody's configuration and this is not a fresh machine.
    let broken = dir.path.join("broken.toml");
    fs::write(&broken, "[general\nenabled = = true\n").expect("writable");
    let (config, outcome) = settings::read_or_default(&broken);

    assert!(outcome.is_err());
    assert_eq!(
        config.general.language,
        Language::Ru,
        "a damaged file is not a first run"
    );
}

/// **The live half, and the whole of what it can honestly claim on this machine.**
///
/// The Windows here is Russian, so the rule answers `ru` — the same value the hard-coded
/// default answered before вопрос 95, which means a green light here proves nothing about the
/// rule. What it *can* prove is that the two Win32 halves work at all: the list comes back
/// non-empty, every tag in it is a plausible BCP-47 tag rather than rubbish, and the answer is
/// one of the fourteen. An instrument that cannot fail says nothing, so the emptiness of the
/// list is asserted rather than tolerated.
#[test]
fn the_machine_answers_a_real_list_of_tags_and_one_of_the_locales() {
    let preferred = settings::preferred_ui_languages();

    println!("preferred UI languages of this machine: {preferred:?}");

    assert!(
        !preferred.is_empty(),
        "Windows always has at least one preferred UI language; an empty list means the call \
         was not made or was not read"
    );

    for tag in &preferred {
        assert!(
            !tag.is_empty()
                && tag.len() <= 85
                && tag
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "`{tag}` is not a language tag — the multi-string was read wrongly"
        );
    }

    let chosen = settings::system_language_for_a_first_run();

    println!(
        "this machine's first-run locale would be `{}`",
        chosen.tag()
    );

    assert!(
        Language::ALL.contains(&chosen),
        "the answer is always one of the twelve"
    );
}
