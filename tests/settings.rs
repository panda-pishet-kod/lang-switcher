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

/// The resource script with its language files — the one reader of the string tables (T-L10N-2).
#[path = "support/resource_script.rs"]
mod resource_script;

/// The registry of the languages — the tables of the tests read it (stage E108, T-108-2).
#[path = "support/registry.rs"]
mod registry;

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

    // Schema 7 — task T-93-1, вопрос 157: `[hotkey] modifiers`, a bare stamp that keeps an older
    // build from dropping the list. Schema 6 — task Т-32-1, вопрос 101: the section `[letters]` of
    // FR-101 and FR-102. Schema 5
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
    // Schema 8 — task T-95-1, вопрос 159: `[hotkey] trigger`, the double press of `Shift`.
    // Schema 9 — stage E108, task T-108-2: the thirty languages of group A in `general.language`.
    // Schema 10 — stage E109, task T-109-1: the twenty-two languages of group B.
    assert_eq!(config.schema_version, 10);
    assert_eq!(CURRENT_SCHEMA_VERSION, 10);

    assert!(config.general.enabled);
    assert!(config.general.autostart);
    assert_eq!(config.general.language, Language::Ru);
    assert_eq!(config.general.theme, ThemeSetting::System);

    assert_eq!(config.hotkey.key, "Pause");
    assert_eq!(config.hotkey.trigger, "press");

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

    // The shape half: the window of the settings (on the shell since T-120-20) changes a copy of
    // the configuration in force field by field and puts the copy in force whole, so a field it
    // never names is carried through untouched. It must not name any of the four.
    let window = window_of_settings_source();

    for (field, what) in SETTINGS_THE_DIALOG_NO_LONGER_SHOWS {
        assert!(
            !window.contains(field),
            "the window of the settings still names `{field}` — {what}; решение 81 left that \
             setting in config.toml and took its control off the window"
        );
    }

    // And the switch that stayed is still wired, or the sweep above would pass for a window
    // that had lost the selection switch altogether.
    assert!(
        window.contains("config.selection.enabled = on"),
        "`[selection] enabled` is the one field of its section the window still shows"
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

    // ⭐ Stage E116, task T-116-12 (решение 208.59): and the mark of the first run — this run
    // creates the file.
    let mut first_run = Config::for_a_first_run();
    first_run.letters.first_mark = Some(letters::FirstMark::Due);
    assert_eq!(config, first_run);

    let mut everything_else = Config::default();
    everything_else.general.language = config.general.language;
    everything_else.letters.first_mark = Some(letters::FirstMark::Due);
    assert_eq!(
        config, everything_else,
        "`general.language` is the one field вопрос 95 may move, and `letters.first_mark` the one \
         T-116-12 writes"
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

    // A failure to read at all — **turned round by решение 120.1, task T-55-6**, the precedent of
    // Э29: a pinner reversed with its reason written beside it. Until that task a file that could
    // not be *read* was treated like one that could not be *parsed*, and moved aside before the
    // first write. A failure to read says nothing about what is in the file — held open by another
    // program it is most likely whole and fine — so nothing is written and nothing is moved this
    // session. The damaged file above is still quarantined: that half does not move.
    let unreadable: Result<ReadOutcome, ConfigError> = Err(ConfigError::Io(std::io::Error::from(
        std::io::ErrorKind::PermissionDenied,
    )));
    assert_eq!(SavePolicy::for_read(&unreadable), SavePolicy::NotRead);

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
    // Task T-93-1 put a fifth rung on top (6 → 7), and the file climbs that one too — and task
    // T-95-1 a sixth (7 → 8, the stamp of `[hotkey] trigger`), and stage E108 a seventh (8 → 9, the
    // stamp of the thirty languages of group A), and stage E109 an eighth (9 → 10, the stamp of the
    // twenty-two of group B).
    assert_eq!(
        config.schema_version, 10,
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
    // ⚠ Task T-93-1: three rungs now, 4 → 5 → 6 → 7 — the stamp of `[hotkey] modifiers` on top;
    // task T-95-1: four, the stamp of `[hotkey] trigger` (7 → 8) above it.
    assert_eq!(config.schema_version, 10, "and that version is ten");

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
    // ⚠ Task T-93-1: the file climbs on past six to the stamp of `[hotkey] modifiers`, which
    // carries nothing of the letters — what this rung decided arrives at seven untouched; and on
    // past seven to the stamp of `[hotkey] trigger` (task T-95-1), which carries nothing either.
    assert_eq!(config.schema_version, 10, "and that version is ten");

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
    // Task T-88-1, решение 148.8: the day the feed was last asked is a field of the section too.
    written.letters.feed_last_try = letters::Date::from_ymd(2026, 9, 22);
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

/// **`feed_last_try` stays out of the file until the feed has been asked** — task T-88-1,
/// решение 148.8.
///
/// The field is new in `e88` and the schema stays 6, so the file of a machine the feed has never
/// asked comes out of this build as it came out of `e87`: no new line until there is a day to put
/// in it. A day, once there is one, is a bare TOML date and is read back as the same day.
#[test]
fn the_day_of_the_last_feed_attempt_is_written_only_once_there_is_one() {
    let dir = TestDir::new("letters_feed_last_try");
    let path = dir.config();

    let mut config = Config::default();
    settings::write_to(&path, &config).expect("writing the configuration must succeed");
    let text = fs::read_to_string(&path).expect("the file must be readable as text");

    assert!(
        !text.contains("feed_last_try"),
        "a feed never asked leaves no line in the file:\n{text}"
    );

    config.letters.feed_last_try = letters::Date::from_ymd(2026, 9, 25);
    settings::write_to(&path, &config).expect("writing the configuration must succeed");
    let text = fs::read_to_string(&path).expect("the file must be readable as text");

    assert!(
        text.contains("feed_last_try = 2026-09-25"),
        "the day is a bare TOML date, like every other day of the section:\n{text}"
    );

    let (read_back, outcome) = settings::read_from(&path).expect("reading it back must succeed");

    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(
        read_back.letters.feed_last_try,
        letters::Date::from_ymd(2026, 9, 25)
    );
}

/// ⭐ **Stage E116, task T-116-12 (решение 208.59) — the file this run creates carries the mark of
/// the first run, and a file of an older build never does.**
///
/// * no file at all — the road of вопрос 95 — comes back `first_mark = due`, and the file written
///   from it says so and is read back the same;
/// * a file without the key (an update: the file of 1.5.1) has no mark — never sent;
/// * a word this build does not know is no mark either, and the file is still read, not refused;
/// * a file with no mark writes no line.
#[test]
fn only_the_file_this_run_creates_carries_the_mark_of_the_first_run() {
    let dir = TestDir::new("letters_first_mark");
    let path = dir.config();

    let (fresh, outcome) = settings::read_from(&path).expect("no file is not an error");

    assert_eq!(outcome, ReadOutcome::NoFile);
    assert_eq!(
        fresh.letters.first_mark,
        Some(letters::FirstMark::Due),
        "T-116-12: the run that creates the file is the first run"
    );

    settings::write_to(&path, &fresh).expect("writing the configuration must succeed");
    let text = fs::read_to_string(&path).expect("the file must be readable as text");

    assert!(
        text.contains("first_mark = \"due\""),
        "the mark is a word of the section:\n{text}"
    );

    let (read_back, outcome) = settings::read_from(&path).expect("reading it back must succeed");

    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(read_back.letters.first_mark, Some(letters::FirstMark::Due));

    // The file of an older build: everything but the new key.
    let older = text
        .replace("first_mark = \"due\"\r\n", "")
        .replace("first_mark = \"due\"\n", "");
    assert!(!older.contains("first_mark"));
    fs::write(&path, &older).expect("the file of an older build is written");

    let (updated, _) = settings::read_from(&path).expect("the file of an older build is read");

    assert_eq!(
        updated.letters.first_mark, None,
        "T-116-12: a file without the key is never a first run"
    );
    assert!(!letters::first_query(updated.letters.first_mark));

    // An unknown word: no mark, and the file is read rather than refused.
    fs::write(
        &path,
        text.replace("first_mark = \"due\"", "first_mark = \"maybe\""),
    )
    .expect("the file is written");

    let (odd, _) = settings::read_from(&path).expect("an unknown word is not damage");

    assert_eq!(odd.letters.first_mark, None);

    // No mark, no line.
    settings::write_to(&path, &Config::default()).expect("writing the configuration must succeed");
    assert!(
        !fs::read_to_string(&path)
            .expect("the file must be readable as text")
            .contains("first_mark")
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
    // A file of today — `{CURRENT_SCHEMA_VERSION}` since task T-93-1 raised the schema past the 6
    // this was written with; what is measured is the date, and the stamp only has to be current.
    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [letters]\n\
             feed = false\n\
             first_run = \"2026-09-01\"\n\
             welcome_shown = true\n"
        ),
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

/// Clears the read-only attribute of a file on the way out of a test, whatever the test did, so
/// that [`TestDir`] can remove its directory — task T-55-3.
struct ReadOnlyCleared<'a>(&'a Path);

impl Drop for ReadOnlyCleared<'_> {
    fn drop(&mut self) {
        if let Ok(metadata) = fs::metadata(self.0) {
            let mut permissions = metadata.permissions();
            // Windows: this clears `FILE_ATTRIBUTE_READONLY` and touches no mode bits.
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = fs::set_permissions(self.0, permissions);
        }
    }
}

/// Puts the read-only attribute on `path` — task T-55-3.
fn set_read_only(path: &Path) {
    let mut permissions = fs::metadata(path)
        .expect("the file must be there")
        .permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions).expect("the attribute must be settable");
}

/// **Task T-55-3, finding Н26 — a read-only `config.toml` takes the write after its attribute is
/// cleared once.** Before this task the rename over a file with `+R` answered «доступ запрещён»
/// every time: the dialog closed as if nothing had happened, and the change was gone at the next
/// start. Now the attribute is cleared, the rename is tried once more, and the file carries the new
/// configuration — with the attribute left cleared, which the doc of `write_to` decides.
#[test]
fn a_read_only_configuration_is_written_after_the_attribute_is_cleared_once() {
    let dir = TestDir::new("read_only");
    let path = dir.config();
    let _cleared = ReadOnlyCleared(&path);

    settings::write_to(&path, &Config::default()).expect("the first write must succeed");
    set_read_only(&path);

    let changed = a_thoroughly_customised_config();
    let outcome = settings::write_to(&path, &changed);

    println!("a write over a read-only file: {outcome:?}");

    assert!(
        matches!(
            outcome,
            Ok(settings::WriteOutcome::WrittenAfterClearingReadOnly)
        ),
        "Н26: a read-only configuration must take the write after its attribute is cleared once, \
         and the write must say that it was: {outcome:?}"
    );

    let (read, _) = settings::read_from(&path).expect("the written file must be readable");

    assert_eq!(read, changed, "and the file carries the new configuration");
    assert!(
        !fs::metadata(&path)
            .expect("the file must be there")
            .permissions()
            .readonly(),
        "the attribute is left cleared — the decision the doc of `write_to` writes down"
    );
    assert_eq!(
        dir.entries(),
        [CONFIG_FILE_NAME],
        "and no temporary is left beside it"
    );
}

/// **The other half of task T-55-3: a refusal the attribute does not explain is still a refusal.**
/// The file is read-only **and** held open by a handle that shares nothing but reading, so no
/// rename can replace it whatever happens to the attribute. The write answers `Err`, the bytes on
/// the disk are the old ones, and the temporary is removed in this branch as in every other. The
/// visible trace is the caller's journal — `configuration write failed` of `Tray::save_config`.
#[test]
fn a_refusal_the_read_only_attribute_does_not_explain_is_still_a_refusal() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = TestDir::new("read_only_and_held");
    let path = dir.config();
    let _cleared = ReadOnlyCleared(&path);

    settings::write_to(&path, &Config::default()).expect("the first write must succeed");
    let before = fs::read(&path).expect("the file must be readable");
    set_read_only(&path);

    // FILE_SHARE_READ and nothing else: no writer and no rename may come near the file while this
    // handle is open.
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .expect("the file must open for reading");

    let outcome = settings::write_to(&path, &a_thoroughly_customised_config());

    println!("a write over a read-only file that is held open: {outcome:?}");

    drop(held);

    assert!(
        matches!(outcome, Err(ConfigError::Io(_))),
        "a refusal no attribute explains is returned as the refusal it is: {outcome:?}"
    );
    assert_eq!(
        fs::read(&path).expect("the file must still be there"),
        before,
        "and the bytes on the disk are the old ones"
    );
    assert_eq!(
        dir.entries(),
        [CONFIG_FILE_NAME],
        "and the temporary is removed in this branch too"
    );
}

/// **Task T-55-5, finding Т8 — a start sweeps the temporaries an interrupted write left, and
/// nothing else.** A process killed between the write of `config.toml.<pid>.tmp` and the rename
/// left that file behind for good: nothing removed it at the next start. The folder here holds one
/// such temporary among neighbours that only look like one — a name with no process id, a process
/// id with a letter in it, somebody else's file, the journal's own temporary — and the kept copy of
/// an unreadable configuration, which is precious. Exactly one file goes.
#[test]
fn a_start_sweeps_the_temporaries_of_an_interrupted_write_and_nothing_else() {
    let dir = TestDir::new("sweep");
    let path = dir.config();

    fs::write(&path, "schema_version = 6\n").expect("the configuration must be writable");
    let configuration = fs::read(&path).expect("the configuration must be readable");

    for (name, text) in [
        ("config.toml.12345.tmp", "half of a write"),
        ("config.toml.bad", "the kept copy of an unreadable file"),
        ("config.toml.tmp", "no process id"),
        ("config.toml.12x45.tmp", "not a process id"),
        ("notes.12345.tmp", "somebody else's"),
        ("lang_switcher.log.777.tmp", "the journal's own temporary"),
    ] {
        fs::write(dir.path.join(name), text).expect("a file of the folder must be writable");
    }

    let swept = settings::remove_abandoned_temporaries(&path);

    println!("swept: {swept:?}, left: {:?}", dir.entries());

    assert_eq!(
        swept,
        settings::Swept {
            removed: 1,
            refused: 0
        },
        "Т8: the one temporary of an interrupted write is removed"
    );
    assert_eq!(
        dir.entries(),
        [
            "config.toml",
            "config.toml.12x45.tmp",
            "config.toml.bad",
            "config.toml.tmp",
            "lang_switcher.log.777.tmp",
            "notes.12345.tmp",
        ],
        "and nothing else: the kept copy and every neighbour stay"
    );
    assert_eq!(
        fs::read(&path).expect("the configuration must still be there"),
        configuration,
        "and the configuration itself is untouched"
    );
}

/// **The name a write leaves is the name the sweep removes** — task T-55-5. The mask is not written
/// out a second time: the test takes the name from `temporary_path_for` itself, so a change to one
/// that the other did not follow is a red here and not a stray file on somebody's disk.
#[test]
fn the_sweep_recognises_exactly_the_name_a_write_leaves() {
    let dir = TestDir::new("sweep_name");
    let path = dir.config();
    let temporary = settings::temporary_path_for(&path);

    fs::write(&temporary, "half of a write").expect("the temporary must be writable");

    let swept = settings::remove_abandoned_temporaries(&path);

    assert_eq!(
        swept.removed,
        1,
        "the sweep must recognise {}",
        temporary.display()
    );
    assert!(!temporary.exists(), "and the temporary is gone");
}

/// **A temporary that cannot be removed is counted, not hidden** — task T-55-5. Held open with no
/// sharing at all it survives the sweep, and the count says so; the caller names it in the journal.
#[test]
fn a_temporary_that_cannot_be_removed_is_counted() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = TestDir::new("sweep_refused");
    let path = dir.config();
    let temporary = dir.path.join("config.toml.4242.tmp");

    fs::write(&temporary, "half of a write").expect("the temporary must be writable");

    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&temporary)
        .expect("the temporary must open");

    let swept = settings::remove_abandoned_temporaries(&path);

    drop(held);

    assert_eq!(
        swept,
        settings::Swept {
            removed: 0,
            refused: 1
        },
        "a temporary that would not go is a count, not silence"
    );
    assert!(temporary.exists(), "and it is still there");
}

/// **Task T-55-6, решение 120.1 — a file that is there and cannot be read is left alone, not
/// quarantined.** Held open by another program with no sharing at all, `config.toml` cannot be
/// read: the answer is an input-output failure, the policy is [`SavePolicy::NotRead`], and the
/// file is exactly where and what it was, with no `.bad` beside it. Until this task the same file
/// went to quarantine at the first save, and the settings in it were replaced by defaults.
#[test]
fn a_file_that_cannot_be_read_is_left_where_it_is_and_not_quarantined() {
    use std::os::windows::fs::OpenOptionsExt;

    let dir = TestDir::new("not_read");
    let path = dir.config();

    fs::write(
        &path,
        format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[hotkey]\nkey = \"F9\"\n"),
    )
    .expect("the configuration must be writable");
    let before = fs::read(&path).expect("the configuration must be readable");

    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .expect("the configuration must open");

    let (_, outcome) = settings::read_or_default(&path);
    let policy = SavePolicy::for_read(&outcome);

    drop(held);

    println!("a file held open: {outcome:?} -> {policy:?}");

    assert!(
        matches!(outcome, Err(ConfigError::Io(_))),
        "the read failed as a read: {outcome:?}"
    );
    assert_eq!(
        policy,
        SavePolicy::NotRead,
        "решение 120.1: a file that could not be read is not damage"
    );
    assert_eq!(
        fs::read(&path).expect("the configuration must still be there"),
        before,
        "and it is exactly what it was"
    );
    assert_eq!(
        dir.entries(),
        [CONFIG_FILE_NAME],
        "with no `.bad` beside it"
    );
}

/// **The read is tried once more, and only once** — task T-55-6. Through the seam of `read_from`:
/// a reader that fails the first time and answers the second is read as if nothing had happened;
/// a reader that always fails is asked exactly twice — once and once more, not a loop.
#[test]
fn a_failed_read_is_tried_exactly_once_more() {
    let dir = TestDir::new("read_twice");
    let path = dir.config();
    let text = format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[hotkey]\nkey = \"F9\"\n");

    // ERROR_SHARING_VIOLATION — what a file held open by another program answers.
    const SHARING_VIOLATION: i32 = 32;

    let calls = std::cell::Cell::new(0u32);
    let outcome = settings::read_from_via(&path, |_| {
        calls.set(calls.get() + 1);

        if calls.get() == 1 {
            Err(std::io::Error::from_raw_os_error(SHARING_VIOLATION))
        } else {
            Ok(text.clone().into_bytes())
        }
    });

    println!(
        "fails once, then answers: {outcome:?} after {} calls",
        calls.get()
    );

    assert_eq!(calls.get(), 2, "a failed read is tried once more");

    let (config, read) = outcome.expect("the second attempt answered");

    assert_eq!(read, ReadOutcome::Current);
    assert_eq!(config.hotkey.key, "F9", "and what it read is the file");

    let calls = std::cell::Cell::new(0u32);
    let outcome = settings::read_from_via(&path, |_| {
        calls.set(calls.get() + 1);
        Err(std::io::Error::from_raw_os_error(SHARING_VIOLATION))
    });

    assert_eq!(calls.get(), 2, "and only once more: not a loop");
    assert!(
        matches!(outcome, Err(ConfigError::Io(_))),
        "a read that failed twice is a failed read: {outcome:?}"
    );
}

/// **Text that is not UTF-8 is damage, not a failed read** — task T-55-6. An editor that saves the
/// file as UTF-16, or as ANSI with a letter outside ASCII, leaves bytes that were read whole and
/// that this build cannot take as text. The verdict is [`ConfigError::Malformed`], and the policy
/// stays the quarantine it has always been: a file read whole and not understood is exactly what
/// the quarantine is for, and calling it «not read» would leave the program unable to save, for
/// good, until somebody found the encoding.
#[test]
fn text_that_is_not_utf8_is_damage_and_not_a_failed_read() {
    let dir = TestDir::new("utf16");
    let path = dir.config();

    // UTF-16LE with its byte order mark, the way an editor saves «Unicode».
    let mut bytes = vec![0xFF, 0xFE];

    for unit in "[general]\r\nenabled = true\r\n".encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }

    fs::write(&path, &bytes).expect("the configuration must be writable");

    let (_, outcome) = settings::read_or_default(&path);

    println!("a UTF-16 file: {outcome:?}");

    assert!(
        matches!(outcome, Err(ConfigError::Malformed { at: None })),
        "bytes read whole that are not text are damage: {outcome:?}"
    );
    assert_eq!(
        SavePolicy::for_read(&outcome),
        SavePolicy::QuarantineFirst,
        "and damage is quarantined, as it always was"
    );
}

/// A document of section 7 whose `[general]`, `[hotkey]` and `[layouts]` carry values a person
/// chose, with one more `line` in a `[section]` of its own — task T-55-7.
fn living_document_with(section: &str, line: &str) -> String {
    format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\n\n\
         [general]\nlanguage = \"de\"\ntheme = \"dark\"\n\n\
         [hotkey]\nkey = \"F9\"\n\n\
         [layouts]\ncycle = [\"0x00000419\", \"0x00000409\"]\n\n\
         [{section}]\n{line}\n"
    )
}

/// **Task T-55-7, решение 120.2 — one bad number loses only itself.**
///
/// Решение 81 took the numbers of section 7 off the dialog, so they are edited by hand — and until
/// this task a value of the wrong type in any one of them (`-1`, `1.5`, `"300"`) declared the whole
/// document damaged: the file went to `.bad`, and the hotkey, the exclusions, the cycle, the
/// language and the theme were replaced by defaults with it (premise П8 of the stage, 15 of 15).
/// Now such a value is the default **of its own field**, everything else is read, and the read
/// reports one softened field.
///
/// **The soft fields, by name:** `replacement.inter_event_delay_ms`,
/// `selection.clipboard_timeout_ms`, `selection.clipboard_restore_delay_ms`, `buffer.capacity`,
/// `buffer.idle_timeout_s` — five, where the decision named four (поправка 120.8).
#[test]
fn one_bad_number_loses_only_itself() {
    type ValueOf = fn(&Config) -> u64;

    let fields: [(&str, &str, ValueOf, u64); 5] = [
        (
            "replacement",
            "inter_event_delay_ms",
            |config| u64::from(config.replacement.inter_event_delay_ms),
            0,
        ),
        (
            "selection",
            "clipboard_timeout_ms",
            |config| u64::from(config.selection.clipboard_timeout_ms),
            300,
        ),
        (
            "selection",
            "clipboard_restore_delay_ms",
            |config| u64::from(config.selection.clipboard_restore_delay_ms),
            200,
        ),
        (
            "buffer",
            "capacity",
            |config| u64::try_from(config.buffer.capacity).expect("a capacity fits in u64"),
            256,
        ),
        (
            "buffer",
            "idle_timeout_s",
            |config| u64::from(config.buffer.idle_timeout_s),
            300,
        ),
    ];

    let mut wrong = Vec::new();

    for (section, field, value_of, default) in fields {
        for bad in ["-1", "1.5", "\"300\""] {
            let _ = settings::take_softened_fields();

            let text = living_document_with(section, &format!("{field} = {bad}"));
            let outcome = Config::from_toml_str(&text);
            let softened = settings::take_softened_fields();

            match &outcome {
                Ok((config, ReadOutcome::Current))
                    if value_of(config) == default
                        && config.general.language == Language::De
                        && config.hotkey.key == "F9"
                        && config.layouts.cycle == ["0x00000419", "0x00000409"]
                        && softened == 1 => {}
                _ => wrong.push(format!(
                    "[{section}] {field} = {bad}: softened {softened}, {:?}",
                    outcome
                        .as_ref()
                        .map(|(config, read)| (read, value_of(config)))
                )),
            }
        }
    }

    assert!(wrong.is_empty(), "решение 120.2: {wrong:#?}");
}

/// **The closed sets and the stamp stay loud** — task T-55-7, the insurance of решение 94. A soft
/// field written generically would soften everything, and `language = "xx"` passing silently is
/// exactly what section 7 forbids for the three enumerated fields. And a read that fails as a
/// whole reports no softened field, even when a bad number stood beside the loud failure.
#[test]
fn the_closed_sets_and_the_stamp_stay_loud_while_the_numbers_are_soft() {
    for (section, line) in [
        ("general", "language = \"xx\""),
        ("layouts", "mode = \"spiral\""),
        ("replacement", "method = \"teleport\""),
    ] {
        let _ = settings::take_softened_fields();

        let text = format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[{section}]\n{line}\n");
        let outcome = Config::from_toml_str(&text);

        assert!(
            matches!(outcome, Err(ConfigError::Malformed { .. })),
            "решение 94: `{line}` must fail the read, loudly: {outcome:?}"
        );
        assert_eq!(
            settings::take_softened_fields(),
            0,
            "and nothing was softened on the way"
        );
    }

    let outcome = Config::from_toml_str("schema_version = \"six\"\n\n[general]\nenabled = true\n");

    assert!(
        matches!(outcome, Err(ConfigError::Malformed { .. })),
        "a stamp that is not a number is not softened into the current one: {outcome:?}"
    );

    let text = format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general]\nlanguage = \"xx\"\n\n[buffer]\ncapacity = -1\n"
    );
    let _ = settings::take_softened_fields();
    let outcome = Config::from_toml_str(&text);

    assert!(matches!(outcome, Err(ConfigError::Malformed { .. })));
    assert_eq!(
        settings::take_softened_fields(),
        0,
        "a read that failed as a whole reports no softened field"
    );
}

/// **Решение 120.9 (variant Б of решение 120.2), task T-55-7a — a typo that breaks the syntax of
/// one soft number costs that number and nothing else.**
///
/// `300ms` is not a value at all but a syntax error of the document, and the parser stops before
/// any field sees it (premise П8: the column is the `m`, not the start of the value). Under variant
/// А — the one task T-55-7 shipped first — such a typo still sent the whole file to quarantine. The
/// owner chose variant Б: the one line the parser stopped on is taken out of the text **in memory**
/// — only a `key = …` line of one of the five soft keys, standing in its own `[section]` — and the
/// document is read again. Turned round from the pin of variant А that stood here.
#[test]
fn a_syntax_typo_in_one_soft_number_costs_that_number_and_nothing_else() {
    type ValueOf = fn(&Config) -> u64;

    let fields: [(&str, &str, ValueOf, u64); 5] = [
        (
            "replacement",
            "inter_event_delay_ms",
            |config| u64::from(config.replacement.inter_event_delay_ms),
            0,
        ),
        (
            "selection",
            "clipboard_timeout_ms",
            |config| u64::from(config.selection.clipboard_timeout_ms),
            300,
        ),
        (
            "selection",
            "clipboard_restore_delay_ms",
            |config| u64::from(config.selection.clipboard_restore_delay_ms),
            200,
        ),
        (
            "buffer",
            "capacity",
            |config| u64::try_from(config.buffer.capacity).expect("a capacity fits in u64"),
            256,
        ),
        (
            "buffer",
            "idle_timeout_s",
            |config| u64::from(config.buffer.idle_timeout_s),
            300,
        ),
    ];

    let mut wrong = Vec::new();

    for (section, field, value_of, default) in fields {
        for typo in ["300ms", "3 00", "5 min"] {
            let _ = settings::take_softened_fields();

            let text = living_document_with(section, &format!("{field} = {typo}"));
            let outcome = Config::from_toml_str(&text);
            let softened = settings::take_softened_fields();

            match &outcome {
                Ok((config, ReadOutcome::Current))
                    if value_of(config) == default
                        && config.general.language == Language::De
                        && config.hotkey.key == "F9"
                        && config.layouts.cycle == ["0x00000419", "0x00000409"]
                        && softened == 1 => {}
                _ => wrong.push(format!(
                    "[{section}] {field} = {typo}: softened {softened}, {:?}",
                    outcome
                        .as_ref()
                        .map(|(config, read)| (read, value_of(config)))
                )),
            }
        }
    }

    assert!(wrong.is_empty(), "решение 120.9: {wrong:#?}");
}

/// **Two syntax typos in two soft numbers cost two numbers** — task T-55-7a. Both lines are taken
/// out, one read after the other, and the read counts two softened fields.
#[test]
fn two_syntax_typos_in_two_soft_numbers_cost_two_numbers() {
    let text = format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[hotkey]\nkey = \"F9\"\n\n[buffer]\ncapacity = 300 chars\nidle_timeout_s = 5min\n"
    );

    let _ = settings::take_softened_fields();
    let outcome = Config::from_toml_str(&text);
    let softened = settings::take_softened_fields();

    println!("two typos: softened {softened}, {outcome:?}");

    let (config, read) = outcome.expect("two soft typos are two soft fields, not a damaged file");

    assert_eq!(read, ReadOutcome::Current);
    assert_eq!(
        (config.buffer.capacity, config.buffer.idle_timeout_s),
        (256, 300),
        "each typo is the default of its own field"
    );
    assert_eq!(config.hotkey.key, "F9", "and the rest of the file is read");
    assert_eq!(softened, 2, "two fields, two lines of the journal");
}

/// **The recovery of решение 120.9 is narrow, and everything outside it stays loud** — task
/// T-55-7a. A closed set written without its quotes, a soft name standing in a section where it is
/// not soft, a soft key written as a dotted key at the top level, a stamp with a unit glued to it:
/// every one is a syntax error on a line that is not a soft key in its own section, and every one
/// still fails the read with nothing softened. So does a soft typo beside a loud failure — the read
/// fails as a whole.
#[test]
fn the_syntax_recovery_takes_only_the_line_of_a_soft_key_in_its_own_section() {
    let loud = [
        format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general]\nlanguage = de\n"),
        format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[layouts]\nmode = spiral\n"),
        format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[replacement]\nmethod = auto mode\n"),
        format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general]\ncapacity = 300ms\n"),
        format!("schema_version = {CURRENT_SCHEMA_VERSION}\nbuffer.capacity = 300ms\n"),
        "schema_version = 6x\n\n[buffer]\ncapacity = 256\n".to_owned(),
        format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general]\nlanguage = \"xx\"\n\n[buffer]\ncapacity = 300ms\n"
        ),
    ];

    for text in loud {
        let _ = settings::take_softened_fields();
        let outcome = Config::from_toml_str(&text);

        assert!(
            matches!(outcome, Err(ConfigError::Malformed { .. })),
            "outside the five soft lines a broken document stays broken: {text:?} -> {outcome:?}"
        );
        assert_eq!(
            settings::take_softened_fields(),
            0,
            "and a read that failed as a whole reports nothing softened: {text:?}"
        );
    }
}

/// **The soft fields are five, by name, and no more** — task T-55-7. Swept over the source, because
/// softness spreads by copying an attribute: a sixth `deserialize_with = "soft_…"` on a closed set
/// would be the silent pass section 7 forbids, and no behavioural test of the five would notice.
/// **Task T-36-6, finding Н116, решение 123.1 — the promise is struck out, and the field with
/// it.**
///
/// The documentation of `CaptureSession` promised two effects of arming: the hotkey stops firing
/// (true — the code is replaced by one no keyboard produces) and the strokes stop reaching the
/// typing buffer (**false** — `app::window_proc` re-publishes `tray.enabled()` into
/// `hook::set_active` after every message the UI thread sees, so hovering over the tray icon puts
/// the flag back). The owner chose to strike the promise out rather than build the second effect,
/// which would need a flag of the capture's own inside `hook`, read by the callback — a new branch
/// on the path NFR-01…NFR-05 governs.
///
/// Three things are measured here, and the third is why this is a sweep and not a comment:
///
/// 1. the sentence is gone from the file;
/// 2. the dead pair is gone with it — no field, no accessor, no restoration in `Drop`;
/// 3. **the explanation stayed.** The sentence cost a measurement to disprove, and a repair that
///    deleted the reason along with the claim would invite the next reader to make the promise
///    again.
#[test]
fn the_capture_no_longer_promises_to_stop_the_typing_buffer() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("settings.rs"),
    )
    .expect("src\\settings.rs must be readable")
    .replace("\r\n", "\n");

    // Контроль прибора: он умеет найти то, что ищет. Без этой строки «ничего не нашлось» могло
    // бы значить «искали не в том файле».
    assert!(
        source.contains("pub struct CaptureSession {"),
        "the sweep is reading the wrong file — CaptureSession is not in it"
    );

    assert!(
        !source.contains("additionally stops the stroke reaching the typing buffer"),
        "Н116: the promise about the typing buffer is struck out — the program does not keep it"
    );
    assert!(
        !source.contains("hook_was_active"),
        "Н116: and the dead pair goes with it — a field nothing can restore correctly"
    );

    // The knowledge the sentence was bought with, in the words that say why the promise was
    // impossible rather than merely absent.
    //
    // ⚠ Searched in a **flattened** copy: a doc comment wraps where `rustfmt` and the eye put it,
    // and a needle of more than a few words crosses a line break and stops matching without
    // anything having changed — the trap of stage Э32, met again by the first draft of this very
    // test («after every message the UI thread sees» is split across two `///` lines).
    let flat = source
        .replace("///", " ")
        .replace("//", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    for kept in [
        "re-publishes `tray.enabled()` into `hook::set_active`",
        "after every message the UI thread sees",
    ] {
        assert!(
            flat.contains(kept),
            "the reason the promise could not be kept must stay in the file: {kept:?}"
        );
    }

    // And the suspension itself is untouched: it rests on the hotkey code, which is what task
    // T-36-6 was told not to change. ⚠ Task T-93-1 moved the needles and not the property: the
    // session remembers the whole `[hotkey]` since the modifiers joined it, and the code goes out
    // with its modifier set in one store (`hook::set_hotkey`, вопрос 157).
    let arm = function_body(&source, "pub fn arm(previous: Hotkey) -> Self {");

    assert!(
        arm.contains("set_hotkey(NO_HOTKEY_VK, 0)"),
        "arming still publishes a code no keyboard produces"
    );
    assert!(
        arm.contains("set_active(false)"),
        "and still clears the flag — removing that call would be a change of behaviour, which \
         this task is not"
    );
}

#[test]
fn exactly_the_five_numbers_of_section_7_are_soft() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("settings.rs"),
    )
    .expect("src\\settings.rs must be readable")
    .replace("\r\n", "\n");

    let needle = "deserialize_with = \"soft_";
    let soft: Vec<&str> = source
        .match_indices(needle)
        .map(|(at, _)| {
            let rest = &source[at + needle.len() - "soft_".len()..];
            &rest[..rest.find('"').expect("an attribute closes its string")]
        })
        .collect();

    println!("soft fields in src\\settings.rs: {soft:?}");

    assert_eq!(
        soft,
        [
            "soft_inter_event_delay_ms",
            "soft_clipboard_timeout_ms",
            "soft_clipboard_restore_delay_ms",
            "soft_buffer_capacity",
            "soft_buffer_idle_timeout_s",
        ],
        "решение 120.2: exactly the five numbers of section 7 are soft, in this order"
    );
}

/// ⭐ **Task T-75-2 — section 7 of `SPEC.md` and the default in the code are one number.**
///
/// `impl Default for Config` carries the promise «The configuration of section 7 exactly as
/// printed there», and until this task **nothing checked it**: the document and the function
/// could drift apart in silence, and the only reader who would have noticed was a person holding
/// the two side by side. The drift is not hypothetical — the task that wrote this guard moved
/// `[selection] clipboard_timeout_ms` from 300 to 900 (решение 135.2), and task T-77-1 moved it
/// back to 300 when that turned out to have been the wrong number to move (решение 137.2). Twice
/// in two deliveries the document and the function had to move together, which is exactly the
/// moment such a promise is usually half-kept.
///
/// One field and not all five: this is the field the task moves, and a guard written for a field
/// nobody is touching would be a guard nobody has ever seen fail. The document is read the way
/// [`exactly_the_five_numbers_of_section_7_are_soft`] reads `src\settings.rs` — out of
/// `CARGO_MANIFEST_DIR`, with the line endings flattened — so the check is against the file that
/// ships and not against a copy of the number kept here.
#[test]
fn the_clipboard_timeout_of_section_7_and_the_default_of_the_code_are_one_number() {
    let spec = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("SPEC.md"))
        .expect("SPEC.md must be readable")
        .replace("\r\n", "\n");

    let printed: Vec<&str> = spec
        .lines()
        .filter(|line| line.starts_with("clipboard_timeout_ms"))
        .collect();

    assert_eq!(
        printed.len(),
        1,
        "section 7 prints the field once and this reads that one line: {printed:?}"
    );

    let value: u32 = printed[0]
        .split_once('=')
        .expect("the line of section 7 is `key = value`")
        .1
        .split('#')
        .next()
        .expect("a split answers at least once")
        .trim()
        .parse()
        .expect("the value of section 7 is a whole number of milliseconds");

    println!("SPEC.md §7: {:?}  ->  {value}", printed[0]);

    let default = Config::default().selection.clipboard_timeout_ms;

    assert_eq!(
        value, default,
        "`impl Default for Config` promises the configuration of section 7 exactly as printed \
         there: SPEC.md says {value}, the code says {default}"
    );
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
    for language in Language::shipped() {
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

    // Stage E108, решение 202.40 (в): a language with a table of the tallest window of its own
    // opens at its own step, so a change to or from it reopens even in one direction of writing.
    let own = |language: Language| settings::has_own_tallest_table(language.in_force());

    if settings::has_own_tallest_table(Language::Ta) && Language::Ta.is_shipped() {
        assert_eq!(
            settings::language_switch(Language::Ru, Language::Ta),
            Reopen,
            "Russian → Tamil: one direction, two tables — the window takes the step of Tamil"
        );
        assert_eq!(
            settings::language_switch(Language::Ta, Language::Ml),
            Reopen,
            "Tamil → Malayalam: two tables of their own"
        );
    }
    assert_eq!(
        settings::language_switch(Language::Ru, Language::De),
        Relabel,
        "Russian → German: one direction, the table of the fourteen"
    );

    for old in Language::ALL {
        for new in Language::ALL {
            let same_table = !own(old) && !own(new);
            let wanted = match (old == new, old.is_rtl() == new.is_rtl() && same_table) {
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

    println!(
        "--- {n} × {n} pairs: relabel {relabel}, reopen {reopen} ---",
        n = Language::ALL.len()
    );

    // Since stage E108 the whole closed set — since E109 sixty-one left-to-right values and five
    // right-to-left ones (Hebrew, Arabic, Urdu, Persian, Uyghur). Only the languages that share the
    // table of the fourteen relabel among themselves (решение 202.40): l of them left to right and
    // r right to left give l × (l − 1) + r × (r − 1) pairs, every other pair of different languages
    // reopens. The two counts, so that a mechanism quietly claiming every pair could not pass.
    let shared_ltr = Language::ALL
        .iter()
        .filter(|l| !l.is_rtl() && !own(**l))
        .count();
    let shared_rtl = Language::ALL
        .iter()
        .filter(|l| l.is_rtl() && !own(**l))
        .count();
    let expected_relabel = shared_ltr * (shared_ltr - 1) + shared_rtl * (shared_rtl - 1);
    println!("--- shared table: {shared_ltr} left to right, {shared_rtl} right to left ---");
    assert!(
        shared_rtl == 5 && shared_ltr < 61,
        "the five right-to-left languages share the table, and some left-to-right ones have their own"
    );
    assert_eq!(
        relabel, expected_relabel,
        "pairs that keep the direction and the table"
    );
    assert_eq!(
        reopen,
        Language::ALL.len() * (Language::ALL.len() - 1) - expected_relabel,
        "pairs that cross one or the other"
    );
}

/// **Стенд Э108, 7.4: a window rebuilt in another step of the font keeps its place, not the size
/// of the old one.**
///
/// The stand switched an open settings window from Russian to Tamil: the new window came up at
/// the step of Tamil but in the rectangle of the Russian one, 769 × 847 against 769 × 752 of a
/// fresh Tamil window, ≈ 95 px of empty ground under the buttons (`s7-live-ru-to-ta`). The sizes
/// below are the stand's.
#[test]
fn a_reopened_window_stands_where_the_old_one_stood_in_its_own_size() {
    // 1920 × 1080 at 100 %, the work area of the stand.
    let work_area = rect(0, 0, 1920, 1032);
    let russian = rect(250, 40, 250 + 769, 40 + 847);

    // Russian → Tamil: the place stays, the size is Tamil's.
    let tamil = rect(255, 0, 255 + 769, 752);
    assert_eq!(
        settings::reopened_window(russian, tamil, work_area),
        rect(250, 40, 250 + 769, 40 + 752)
    );

    // A change of direction alone keeps the step and the size: the old rectangle, as in 1.2.0.
    let same_size = rect(255, 0, 255 + 769, 847);
    assert_eq!(
        settings::reopened_window(russian, same_size, work_area),
        russian
    );

    // Tamil → Russian low on the screen: the taller window is lifted so that its bottom is on
    // the work area, and never above the top of it.
    let tamil_low = rect(250, 260, 250 + 769, 260 + 752);
    assert_eq!(
        settings::reopened_window(tamil_low, rect(0, 0, 769, 847), work_area),
        rect(250, 185, 250 + 769, 1032)
    );
    assert_eq!(
        settings::reopened_window(tamil_low, rect(0, 0, 769, 1100), work_area),
        rect(250, 0, 250 + 769, 1100)
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
    ButtonBorderRole, ButtonColors, ButtonFaceRole, ButtonTextRole, FOG, GRAPHITE, resolve,
};

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
        // Task T-100-3 — the button of the automatic way is an ordinary button of the one table
        // (Э80): no frame of its own, no colour of its own.
        (1054, "Выбрать окно программы"),
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
///
/// ## ⛔⛔ Отменено задачей T-80-2 — словом владельца, а не вкусом исполнителя
///
/// Finding A-13 read the mock-up right: its about window draws a fill and no outline. What a
/// mock-up cannot show is what a frameless button does **on a live machine, in the light
/// palette, under the pointer**. Measured on `e79`: the hot face is `hover_bg` 234,238,242 and
/// the ground of that window is `window_bg` 237,239,242 — **3/1/0**, and with no frame the
/// button simply goes out. In «От автора» it is worse: those buttons stand on blocks, and in
/// «Туман» `panel_bg` and `button_bg` are the same 255,255,255, so a frameless button at rest is
/// invisible **except** for the corners the rounding cut away.
///
/// Word of the user at the live acceptance of `e79`: «кнопки в разделе от автора сливаются с
/// фоном не имея каймы… Нужно привести все кнопки к типизированному механизму, который у нас уже
/// существует». So there is one table now — [`settings::button_color_roles`] — and the frame is
/// a column of it rather than a wrapper around it.
///
/// **The one control that keeps `FaceItself` is the name «Lang Switcher»:** it is a name that
/// opens a page, not a button that looks like one, and its two states were accepted by the same
/// eye in the same sitting.
#[test]
fn one_table_answers_every_button_and_the_frame_is_a_column_of_it() {
    /// «Lang Switcher» — the one frameless control, `IDC_AUTHOR_NAME` of «От автора» (FR-92а:
    /// «О программе», which carried it too, is a window of the shell now).
    const NAME: i32 = 1261;

    // Every ordinary button of every window, «ОК» and «Отмена» included, in every state.
    for control in [1, 2, 1012, 1080, 1265, 1266, 1271, 1278, 1279] {
        for hot in [false, true] {
            for pressed in [false, true] {
                for disabled in [false, true] {
                    assert_eq!(
                        settings::button_color_roles(control, hot, pressed, disabled).border,
                        ButtonBorderRole::ButtonBorder,
                        "button {control} wears the frame of the one table (hot = {hot}, \
                         pressed = {pressed}, disabled = {disabled})"
                    );
                }
            }
        }
    }

    // And the name, in both of the two states it has.
    for hot in [false, true] {
        assert_eq!(
            settings::button_color_roles(NAME, hot, false, false).border,
            ButtonBorderRole::FaceItself,
            "the name «Lang Switcher» is the one control drawn without a frame (hot = {hot})"
        );
    }

    // ⭐ And there is **no second table**: the frameless wrapper `about_button_colors` is gone
    // from the whole of `src\`, so no window can ask a different question than its neighbour.
    // That is the defect of `e79` written as a guard — the about window and the three windows of
    // `letters` asked the wrapper while the settings dialog asked the table.
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found: Vec<&str> = Vec::new();

    for module in ["settings.rs", "letters.rs", "theme.rs", "app.rs", "tray.rs"] {
        let Ok(text) = std::fs::read_to_string(sources.join(module)) else {
            continue;
        };

        // ⚠ The needle carries the parenthesis: a call and a definition both have one, and the
        // prose that records why the wrapper was removed does not. A needle without it would
        // find the tombstone and call it a body.
        if text.contains("about_button_colors(") {
            found.push(module);
        }
    }

    assert!(
        found.is_empty(),
        "the frameless wrapper must be gone — still called in {found:?}"
    );

    // ⚠ Отрицательный контроль: the needle must find a call when there **is** one, and must not
    // find one in the prose that explains the removal — otherwise the assertion above is green
    // for the reason that it looked at nothing.
    assert!(
        "let plain = about_button_colors(control, hot, pressed, disabled);"
            .contains("about_button_colors("),
        "the needle must match a call"
    );
    assert!(
        !"`about_button_colors` стояла здесь и снята задачей T-80-2"
            .contains("about_button_colors("),
        "and must not match the sentence that says it is gone"
    );
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

/// **Task T-42-6, finding Н112, решение 124.3** — the ninth tick of the cycle list does not go
/// in, and the refusal is heard as well as seen.
///
/// The window let a person tick as many layouts as the list has rows, and the engine takes the
/// first `layouts::MAX_CYCLE` of them and drops the rest **silently** — so the ninth tick was a
/// choice the program did not honour and did not say it would not honour. Решение 124.3: the
/// ceiling of eight stays, the rows are **not** muted (that would be a change of a frozen look),
/// and the ninth tick is refused through the very gate that already refuses every change of the
/// pair mode.
///
/// The whole rule is `settings::cycle_change_refusal`, and this test is that rule; what the
/// window adds is the asking — the mode out of its state, the state words out of the
/// notification, the count of ticks out of the control itself.
#[test]
fn the_ninth_tick_of_the_cycle_list_is_refused_and_the_eight_stay() {
    let ticking = (
        settings::UNCHECKED_IMAGE | 0x0001,
        settings::CHECKED_IMAGE | 0x0001,
    );
    let unticking = (settings::CHECKED_IMAGE, settings::UNCHECKED_IMAGE);
    let selecting = (settings::CHECKED_IMAGE, settings::CHECKED_IMAGE | 0x0002);

    // 1. What counts as putting a tick in: the state image, and only it.
    assert!(
        settings::is_ticking_a_row(ticking.0, ticking.1),
        "the image going to CHECKED_IMAGE from anything else is a hand ticking a row"
    );
    assert!(
        !settings::is_ticking_a_row(unticking.0, unticking.1),
        "a tick coming out is not a tick going in — it is always allowed"
    );
    assert!(
        !settings::is_ticking_a_row(selecting.0, selecting.1),
        "a selection changing while the image stays is not a tick at all"
    );

    // 2. The ceiling itself, read from the engine's own constant rather than from an eight
    //    written here: the number belongs to `layouts::MAX_CYCLE` (решение 124.3 — 8 stays).
    for already in 0..layouts::MAX_CYCLE {
        assert!(
            !settings::ninth_tick_is_refused(already),
            "with {already} ticked the next tick is number {} and goes in",
            already + 1
        );
    }

    assert!(
        settings::ninth_tick_is_refused(layouts::MAX_CYCLE),
        "with {} ticked the next tick would be the ninth and is refused",
        layouts::MAX_CYCLE
    );

    // 3. The whole gate. Eight ticked: the ninth is refused **aloud** — FR-100, the third tone.
    assert_eq!(
        settings::cycle_change_refusal(
            Some(LayoutMode::Cycle),
            true,
            ticking.0,
            ticking.1,
            layouts::MAX_CYCLE
        ),
        Some(settings::CycleRefusal::Aloud),
        "the ninth tick is refused, and a person hears why"
    );

    // Seven ticked: the eighth goes in and nothing is said.
    assert_eq!(
        settings::cycle_change_refusal(
            Some(LayoutMode::Cycle),
            true,
            ticking.0,
            ticking.1,
            layouts::MAX_CYCLE - 1
        ),
        None,
        "the eighth tick is the last one the engine honours, and it goes in"
    );

    // Taking a tick **out** at the ceiling: always allowed — otherwise a full list could never
    // be changed at all.
    assert_eq!(
        settings::cycle_change_refusal(
            Some(LayoutMode::Cycle),
            true,
            unticking.0,
            unticking.1,
            layouts::MAX_CYCLE
        ),
        None,
        "a tick can always come out, ceiling or no ceiling"
    );

    // Selecting a row at the ceiling is not a tick; three ticks in a list of twenty refuse
    // nothing either (ТЗ приёмка T-42-6).
    assert_eq!(
        settings::cycle_change_refusal(
            Some(LayoutMode::Cycle),
            true,
            selecting.0,
            selecting.1,
            layouts::MAX_CYCLE
        ),
        None,
        "moving the selection is not ticking a row"
    );
    assert_eq!(
        settings::cycle_change_refusal(Some(LayoutMode::Cycle), true, ticking.0, ticking.1, 3),
        None,
        "three ticks in a list of twenty: the fourth goes in"
    );

    // A notification that touches no state at all — `uChanged` without `LVIF_STATE`.
    assert_eq!(
        settings::cycle_change_refusal(
            Some(LayoutMode::Cycle),
            false,
            ticking.0,
            ticking.1,
            layouts::MAX_CYCLE
        ),
        None,
        "a change that touches no state is not a tick"
    );

    // 4. The pair mode still refuses everything, and **silently**: nobody asked the program for
    //    anything there — FR-31, task T-11-7-2. The two refusals are told apart on purpose.
    assert_eq!(
        settings::cycle_change_refusal(Some(LayoutMode::Pair), true, ticking.0, ticking.1, 0),
        Some(settings::CycleRefusal::Silent),
        "the pair mode refuses every change and says nothing about it"
    );

    // 5. The programmatic fill — `None` as the mode — passes at any number of ticks: that is
    //    the program writing its own list, and `fill_cycle_list` ticks more than eight rows
    //    only if the file already holds them.
    assert_eq!(
        settings::cycle_change_refusal(None, true, ticking.0, ticking.1, layouts::MAX_CYCLE),
        None,
        "the program's own write under the held borrow is not a hand at the ceiling"
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

// -----------------------------------------------------------------------------------------
// Task T-11-19 — the rhythm of the layout: the window is 16 dialog units taller
// -----------------------------------------------------------------------------------------

// -----------------------------------------------------------------------------------------
// Task T-11-13 — the defect, and the mock-ups
// -----------------------------------------------------------------------------------------

/// **The разбор of every face role into a colour, closed by the full table** — task T-78-3.
///
/// The role table above says *which* role the name wears; this says what each role **is**, and
/// the two together are the whole of the decision. A role named right and resolved wrong paints
/// the same wrong pixel as a role named wrong.
///
/// Every role of the vocabulary is here, so a new one cannot be added without this table being
/// edited on purpose — the same closure `the_button_colour_roles_follow_the_closed_table_of_fr_92a`
/// keeps over the identifiers.
#[test]
fn every_button_face_role_resolves_to_the_palette_field_it_is_named_after() {
    for (what, palette) in [("Туман", &FOG), ("Графит", &GRAPHITE)] {
        for (role, expected, field) in [
            (ButtonFaceRole::ButtonBg, palette.button_bg, "button_bg"),
            (ButtonFaceRole::AccentBg, palette.accent_bg, "accent_bg"),
            (ButtonFaceRole::SelBg, palette.sel_bg, "sel_bg"),
            (ButtonFaceRole::HoverBg, palette.hover_bg, "hover_bg"),
            (ButtonFaceRole::SelFg, palette.sel_fg, "sel_fg"),
            (ButtonFaceRole::WindowBg, palette.window_bg, "window_bg"),
            (
                ButtonFaceRole::MenuHoverBg,
                palette.menu_hover_bg,
                "menu_hover_bg",
            ),
        ] {
            assert_eq!(
                theme::button_face_color(role, palette),
                expected,
                "{what}: {role:?} is named after {field} and must resolve to it"
            );
        }
    }
}

/// `MulDiv` of Windows for the positive numbers a dialog is made of: the product over the
/// divisor, **rounded half up** — the way the dialog manager maps a template into pixels
/// (measured, `scratchpad-E83\probe-premises-e83.log`: 38 units at 7 px per 4 → **67**, 206 →
/// **361**, 12 units at 17 px per 8 → **26**; truncation would say 66, 360 and 25).
fn mul_div(number: i32, numerator: i32, denominator: i32) -> i32 {
    let product = i64::from(number) * i64::from(numerator);

    i32::try_from((product + i64::from(denominator) / 2) / i64::from(denominator))
        .expect("a dialog is small")
}

/// The base units of the dialog face at `face` pixels, and the box of the backing round «Lang
/// Switcher» in the name's face derived from it — the numbers [`settings::fit_program_name`] and
/// `paint_program_name` both ask `name_backing_box` for, built here the way the window builds them.
fn name_backing_at(face: i32) -> (i32, i32, theme::ChipBox) {
    let wide = |text: &str| -> Vec<u16> { text.encode_utf16().collect() };

    let mut base = LOGFONTW {
        lfHeight: -face,
        lfWeight: 400,
        ..Default::default()
    };
    for (at, ch) in wide("Segoe UI").into_iter().enumerate() {
        base.lfFaceName[at] = ch;
    }

    let name_face = settings::about_name_logfont(base, settings::Emphasis::Semibold);

    // SAFETY: every handle made below is selected out and deleted on the one path out of this
    // block; the DC is a memory DC of this process.
    unsafe {
        let dc = CreateCompatibleDC(None);

        let dialog = CreateFontIndirectW(&base);
        let previous = SelectObject(dc, dialog.into());
        let mut metrics = TEXTMETRICW::default();
        let _ = GetTextMetricsW(dc, &mut metrics);
        let alphabet = wide("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ");
        let mut size = Default::default();
        let _ = GetTextExtentPoint32W(dc, &alphabet, &mut size);
        let unit_x = (size.cx / 26 + 1) / 2;
        let unit_y = metrics.tmHeight;
        SelectObject(dc, previous);
        let _ = DeleteObject(dialog.into());

        let name = CreateFontIndirectW(&name_face);
        let previous = SelectObject(dc, name.into());
        let mut metrics = TEXTMETRICW::default();
        let _ = GetTextMetricsW(dc, &mut metrics);
        let mut size = Default::default();
        let _ = GetTextExtentPoint32W(dc, &wide("Lang Switcher"), &mut size);
        SelectObject(dc, previous);
        let _ = DeleteObject(name.into());
        let _ = DeleteDC(dc);

        (
            unit_x,
            unit_y,
            theme::chip_box(size.cx, name_face.lfHeight.abs(), metrics.tmHeight, 0),
        )
    }
}

/// The rectangle `paint_program_name` fills with the backing inside a button of `rect`: from
/// the left edge, the width of the chip, never past the right edge — the arithmetic of the drawing,
/// written out so that the assertion below compares the button with **what is drawn**.
fn drawn_backing(rect: windows::Win32::Foundation::RECT, chip: &theme::ChipBox) -> (i32, i32) {
    (rect.left, (rect.left + chip.width).min(rect.right))
}

/// Whether the button is exactly its backing — the one statement of задача T-83-1.
///
/// ⚠ **Both halves, and the first draft had only the second**: the drawing clips the backing to
/// the button, so a button one pixel **narrower** than the chip is also «wholly covered by what is
/// drawn» — and the backing is then cut. The control «−1 px» caught exactly that draft. The button
/// is its backing when it is as wide as the chip **and** the drawn figure fills it.
fn button_is_its_backing(rect: windows::Win32::Foundation::RECT, chip: &theme::ChipBox) -> bool {
    rect.right - rect.left == chip.width && drawn_backing(rect, chip) == (rect.left, rect.right)
}

/// **⛔⛔ The button of the name IS its backing — задача T-83-1, решение 143.6, долг Э81-Б-2.**
///
/// The statement held here is **«the rectangle of the name's button equals the backing»**, in
/// pixels, on the faces of the template — and not the neighbouring «the backing is narrower than
/// the column» the guard of Э81 held, which was green on the defect: the button was then 361 px
/// wide over a backing of 116, and the mouse woke the hand over the empty two thirds of the line.
///
/// The rectangle is the one the dialog manager makes of the template (`MulDiv`, measured), the
/// backing is the chip the drawing measures, and the narrowing is `settings::name_rect_of_backing`
/// — the pure half of the very body both windows call. Both templates, one statement.
///
/// ⚠ Controls: the rectangle the template gives — the whole column — is caught (the shape of
/// `e82`); a rectangle narrowed by one pixel too many or too few is caught too.
#[test]
fn the_button_of_the_name_is_exactly_its_backing() {
    /// «Lang Switcher» — the number `app.rc` gives it in «От автора» («О программе» carried it
    /// too until it became a window of the shell, FR-92а).
    const NAMES: [(u16, u32); 1] = [(letters::IDD_AUTHOR, 1261)];
    /// The width of the backing at 13 px, 10 pt at 96 DPI — решение 143.6 names it: **116**.
    const BACKING_AT_13: i32 = 116;

    let (unit_x, unit_y, chip) = name_backing_at(13);

    assert_eq!(
        chip.width, BACKING_AT_13,
        "the backing at the face of the template is the 116 px решение 143.6 was taken on"
    );

    let product = ProductImage::open();

    for (dialog, name) in NAMES {
        let template = DialogTemplate::parse(&product.resource(RT_DIALOG, dialog));
        let (left, top, right, bottom) = template.rect_of(name);

        // The rectangle the dialog manager makes of it — `x` and `cx` mapped each on its own.
        let x = mul_div(left, unit_x, 4);
        let y = mul_div(top, unit_y, 8);
        let rect = windows::Win32::Foundation::RECT {
            left: x,
            top: y,
            right: x + mul_div(right - left, unit_x, 4),
            bottom: y + mul_div(bottom - top, unit_y, 8),
        };

        let fitted = settings::name_rect_of_backing(rect, chip.width);

        println!(
            "dialog {dialog}: template rectangle {}x{} at {},{}; backing {}; the button now \
             {}x{} at {},{}",
            rect.right - rect.left,
            rect.bottom - rect.top,
            rect.left,
            rect.top,
            chip.width,
            fitted.right - fitted.left,
            fitted.bottom - fitted.top,
            fitted.left,
            fitted.top
        );

        assert!(
            button_is_its_backing(fitted, &chip),
            "⛔⛔ Э81-Б-2: dialog {dialog} — the button of the name is {} px wide over a backing \
             of {}: the mouse is caught where nothing is drawn",
            fitted.right - fitted.left,
            chip.width
        );
        assert_eq!(
            (fitted.left, fitted.top, fitted.bottom),
            (rect.left, rect.top, rect.bottom),
            "dialog {dialog}: the name must not move — only the right edge comes in (C5 of Э79)"
        );

        // ⚠ Отрицательный контроль: the rectangle of the template itself — the whole column, the
        // shape of `e82` — must be caught, and so must a button one pixel off the backing either
        // way.
        assert!(
            !button_is_its_backing(rect, &chip),
            "the control must be caught: the column-wide button of e82"
        );
        for off_by_one in [-1, 1] {
            let off = windows::Win32::Foundation::RECT {
                right: fitted.right + off_by_one,
                ..fitted
            };
            assert!(
                !button_is_its_backing(off, &chip),
                "the control must be caught: a button {off_by_one:+} px off its backing"
            );
        }
    }
}

/// **«От автора» narrows the name by the ONE body — задача T-83-1.**
///
/// `settings::fit_program_name` is the measuring and the moving; «От автора» calls it from its
/// `WM_INITDIALOG`. The count is the guard, the way `paint_program_name` is guarded: exactly one
/// call in `letters.rs`, exactly one occurrence in `settings.rs` (the definition — «О программе»,
/// which called it too, is a window of the shell since FR-92а moved it there), and the body
/// measures by the drawing's own `name_backing_box` and narrows by the pure
/// `name_rect_of_backing`.
///
/// ⚠ Control: `letters.rs` with its call taken out — the mutant of the task, «не ужимать» — is
/// caught.
#[test]
fn both_windows_narrow_the_name_by_the_one_body() {
    let source = settings_module_source();
    let letters = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("letters.rs"),
    )
    .expect("src\\letters.rs must be readable")
    .replace("\r\n", "\n");

    let code_of = |text: &str| -> String {
        text.lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let calls_in_letters = |text: &str| {
        code_of(text)
            .matches("settings::fit_program_name(hwnd, IDC_AUTHOR_NAME,")
            .count()
    };
    let occurrences_in_settings = |text: &str| code_of(text).matches("fit_program_name(").count();

    assert_eq!(
        calls_in_letters(&letters),
        1,
        "«От автора» must narrow its name by the one body, exactly once"
    );
    assert_eq!(
        occurrences_in_settings(&source),
        1,
        "settings.rs must hold the body and no call of its own"
    );

    let body = function_body(&source, "pub(crate) unsafe fn fit_program_name(");
    assert!(
        body.contains("name_backing_box(dc, face, &caption, rect)")
            && body.contains("name_rect_of_backing(rect, chip.width)")
            && body.contains("child_rect_in_client(hwnd, control)")
            && body.contains("move_child(hwnd, control, narrowed.top, None, &narrowed)"),
        "the body must measure by the drawing's own box and move by the mirror-safe road"
    );

    // ⚠ Контроль: the mutant of the task — the window left unnarrowed.
    let letters_without = letters.replace(
        "settings::fit_program_name(hwnd, IDC_AUTHOR_NAME, name_face)",
        "()",
    );
    assert_ne!(
        letters_without, letters,
        "the control must change letters.rs"
    );
    assert_eq!(
        calls_in_letters(&letters_without),
        0,
        "the letters control must be caught"
    );
}

/// **The live «От автора» catches the mouse on the backing only — задача T-83-1.**
///
/// The window itself, not its template: built hidden through the measuring door of the tests
/// (`letters::testing`, the product's own `open_window` on the product's own templates), laid out
/// by the product's `WM_INITDIALOG`, and its name's button read back — in a left-to-right locale and
/// in both mirrored ones, where the rectangle comes back through the mirror (Т-30-2).
///
/// ⚠ «О программе» is modal and cannot be built beside a test; it is held by the pure statement and
/// by the sweep above.
#[cfg(feature = "testing")]
#[test]
fn the_live_author_window_catches_the_mouse_on_the_backing_only() {
    let _guard = with_product_strings();
    let (_, _, chip) = name_backing_at(13);

    for language in [Language::Ru, Language::El, Language::He, Language::Ar] {
        settings::set_ui_language(language);

        let window = HiddenWindow::open("0.83.0");
        let rect = window.control_rect(1261);

        println!(
            "{language:?}: the name's button {}x{} at {},{}; the client {} px tall",
            rect.right - rect.left,
            rect.bottom - rect.top,
            rect.left,
            rect.top,
            window.client_height()
        );

        assert!(
            button_is_its_backing(rect, &chip),
            "⛔⛔ Э81-Б-2, {language:?}: the live name's button is {} px wide over a backing of {}",
            rect.right - rect.left,
            chip.width
        );
    }

    settings::set_ui_language(Language::Ru);
}

/// **⛔ C5 — the mirror holds at a reduced step — задачи T-83-3 и T-83-4.**
///
/// The step and the mirror patch the same copy of the template, two fields apart: `pointsize` in
/// the font block and `WS_EX_LAYOUTRTL` in `exStyle`. In Hebrew and in Arabic, at ten points and at
/// eight, «От автора» must come out mirrored, at the face of its step, and with its name's button
/// exactly its backing at that face — the one body of T-83-1 measures in whatever face the window
/// got.
#[cfg(feature = "testing")]
#[test]
fn the_mirror_holds_at_a_reduced_step() {
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, WS_EX_LAYOUTRTL,
    };

    let _guard = with_product_strings();

    for language in [Language::He, Language::Ar] {
        settings::set_ui_language(language);

        for step in [10_u16, 8] {
            let face = theme::face_at(step, 96);
            let (mirrored, font, name) = settings::testing::with_font_step(step, || {
                let window = HiddenWindow::open("0.83.0");
                // SAFETY: the window is live for this block; the style is only read.
                let styles = unsafe { GetWindowLongPtrW(window.window, GWL_EXSTYLE) };
                (
                    styles & isize::try_from(WS_EX_LAYOUTRTL.0).expect("a bit") != 0,
                    window.dialog_font_height(),
                    window.control_rect(1261),
                )
            });
            let (_, _, chip) = name_backing_at(face);

            println!(
                "{language:?} {step} pt: mirrored {mirrored}, lfHeight {font}, the name's button \
                 {}x{} over a backing of {}",
                name.right - name.left,
                name.bottom - name.top,
                chip.width
            );

            assert!(
                mirrored,
                "⛔ C5, {language:?} {step} pt: the window must be mirrored"
            );
            assert_eq!(font, -face, "{language:?} {step} pt: the face of the step");
            assert!(
                button_is_its_backing(name, &chip),
                "{language:?} {step} pt: the name's button must be its backing at this face"
            );
        }
    }

    settings::set_ui_language(Language::Ru);
}

/// A window of `letters`, built hidden through a measuring door and destroyed on drop — «От автора»
/// (задача T-83-1) and, since задача T-98-1, «Последние письма».
#[cfg(feature = "testing")]
struct HiddenWindow {
    owner: windows::Win32::Foundation::HWND,
    window: windows::Win32::Foundation::HWND,
}

#[cfg(feature = "testing")]
impl HiddenWindow {
    /// «От автора» as the product opens it — the words out of the configuration and the feed of a
    /// test binary, which has neither: the window of a first day.
    fn open(version: &str) -> Self {
        Self::on_owner(|owner| {
            letters::testing::open_author_hidden(owner, ProductImage::shared().module, version)
        })
    }

    /// «От автора» with the words of `view` — the door of задача T-98-1.
    fn author(view: letters::AuthorView) -> Self {
        Self::on_owner(|owner| {
            letters::testing::open_author_hidden_with(owner, ProductImage::shared().module, view)
        })
    }

    /// An owner shaped like `LangSwitcher.Hidden` — a popup of no size at the origin, never shown —
    /// and the window `open` makes on it.
    fn on_owner(
        open: impl FnOnce(windows::Win32::Foundation::HWND) -> Option<windows::Win32::Foundation::HWND>,
    ) -> Self {
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, WS_EX_TOOLWINDOW, WS_POPUP,
        };
        use windows::core::{PCWSTR, w};

        // SAFETY: a system class, no parent, no pointer of ours; destroyed in `Drop`.
        let owner = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .expect("a hidden owner must be creatable");

        let window = open(owner).expect("the window must open out of the product's templates");

        Self { owner, window }
    }

    /// One control's rectangle in the client coordinates of the window, corners in order.
    fn control_rect(&self, control: i32) -> windows::Win32::Foundation::RECT {
        use windows::Win32::Foundation::{POINT, RECT};
        use windows::Win32::Graphics::Gdi::ScreenToClient;
        use windows::Win32::UI::WindowsAndMessaging::{GetDlgItem, GetWindowRect};

        // SAFETY: the window is live until `Drop`; the buffers are locals of this frame.
        unsafe {
            let child = GetDlgItem(Some(self.window), control).expect("the control must exist");
            let mut rect = RECT::default();
            GetWindowRect(child, &mut rect).expect("its rectangle must be readable");
            let mut corners = [
                POINT {
                    x: rect.left,
                    y: rect.top,
                },
                POINT {
                    x: rect.right,
                    y: rect.bottom,
                },
            ];
            for corner in &mut corners {
                let _ = ScreenToClient(self.window, corner);
            }
            RECT {
                left: corners[0].x.min(corners[1].x),
                top: corners[0].y.min(corners[1].y),
                right: corners[0].x.max(corners[1].x),
                bottom: corners[0].y.max(corners[1].y),
            }
        }
    }

    /// The `lfHeight` of the font the dialog manager gave the window — what the template's
    /// `pointsize` became (C7).
    fn dialog_font_height(&self) -> i32 {
        use windows::Win32::Graphics::Gdi::{GetObjectW, HGDIOBJ};
        use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_GETFONT};

        // SAFETY: the window is live until `Drop`; the font is the manager's and only read.
        unsafe {
            let font = SendMessageW(self.window, WM_GETFONT, None, None);
            let mut logical = LOGFONTW::default();
            let _ = GetObjectW(
                HGDIOBJ(font.0 as *mut _),
                i32::try_from(size_of::<LOGFONTW>()).expect("small"),
                Some(std::ptr::from_mut(&mut logical).cast()),
            );
            logical.lfHeight
        }
    }

    /// The height of the client area — what `layout_author` made the window.
    fn client_height(&self) -> i32 {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

        let mut client = RECT::default();
        // SAFETY: the window is live until `Drop`.
        unsafe { GetClientRect(self.window, &mut client) }.expect("the client rectangle");
        client.bottom - client.top
    }
}

/// **Задача T-103-2, сторож (4), решение 181.1 — a change of language on the fly moves where a
/// person is sent to write.**
///
/// The wizard of FR-104, opened hidden out of the templates of the built product, and asked by the
/// door of the task where its address would take them. Built in Russian, the group; the language
/// moved and [`letters::language_changed`] told the window, GitHub Issues; Ukrainian, the group
/// again. «Спасибо», the other window of the rule, is a window of the shell since T-120-16 — its
/// half of the guard is in `tests\letters.rs`.
///
/// ⚠ Control: the language moved and **nobody told the window** — the address stays with the words
/// the window shows. Without it a test that read the language in force at the click would pass for
/// one that follows the window.
#[cfg(feature = "testing")]
#[test]
fn a_change_of_language_on_the_fly_moves_where_a_person_is_sent_to_write() {
    let _guard = with_product_strings();
    let module = ProductImage::shared().module;

    settings::set_ui_language(Language::Ru);

    let wizard =
        HiddenWindow::on_owner(|owner| letters::testing::open_wizard_hidden(owner, module));
    let address = |window: &HiddenWindow| {
        letters::testing::feedback_url(window.window).expect("a window of `letters`")
    };

    let mut seen = Vec::new();

    seen.push(("built in Ru", address(&wizard)));

    // The control: the language moved and nobody told the window.
    settings::set_ui_language(Language::En);
    seen.push(("En, not told", address(&wizard)));

    // The road of FR-94: the language changed while the window was open.
    letters::language_changed(wizard.owner);
    seen.push(("En, told", address(&wizard)));

    settings::set_ui_language(Language::Uk);
    letters::language_changed(wizard.owner);
    seen.push(("Uk, told", address(&wizard)));

    settings::set_ui_language(Language::Ru);

    println!("{seen:#?}");

    let group = letters::links::GROUP_URL;
    let issues = letters::links::ISSUES_URL;

    assert_eq!(
        seen,
        [
            ("built in Ru", group),
            ("En, not told", group),
            ("En, told", issues),
            ("Uk, told", group),
        ],
        "181.1: the address follows the language of the window, and moves when it moves"
    );
}

/// **Задача T-103-9, сторож (2), решение 181.33 — a change of language on the fly moves the two
/// pages of the site too.** «От автора» opened hidden out of the templates of the built product and
/// asked by the door of the task which pages «Открыть страницу загрузки» and the name of the program
/// would open — the very calls `perform` reads them by. Built in Russian, the Russian pages; the
/// language moved to English and [`letters::language_changed`] told the window, the pages under
/// `/en/`; Ukrainian, its own under `/uk/` since stage E112 (Q-206-1); Kazakh, the Russian ones
/// again (181.33 п. 2, 202.6).
///
/// ⚠ Control: the language moved and **nobody told the window** — the pages stay with the words
/// the window shows.
#[cfg(feature = "testing")]
#[test]
fn a_change_of_language_on_the_fly_moves_the_pages_of_the_site_too() {
    let _guard = with_product_strings();

    settings::set_ui_language(Language::Ru);

    let author = HiddenWindow::open("0.83.0");
    let pages = |window: &HiddenWindow| {
        letters::testing::site_pages(window.window).expect("a window of `letters`")
    };

    let mut seen = Vec::new();

    seen.push(("built in Ru", pages(&author)));

    // The control: the language moved and nobody told the window.
    settings::set_ui_language(Language::En);
    seen.push(("En, not told", pages(&author)));

    letters::language_changed(author.owner);
    seen.push(("En, told", pages(&author)));

    settings::set_ui_language(Language::Uk);
    letters::language_changed(author.owner);
    seen.push(("Uk, told", pages(&author)));

    settings::set_ui_language(Language::Kk);
    letters::language_changed(author.owner);
    seen.push(("Kk, told", pages(&author)));

    settings::set_ui_language(Language::Ru);

    println!("{seen:#?}");

    let russian = (letters::links::DOWNLOAD_URL, letters::links::PROGRAM_URL);
    let english = (
        letters::links::DOWNLOAD_URL_EN,
        letters::links::PROGRAM_URL_EN,
    );
    let ukrainian = (
        "https://panda-pishet-kod.dev/uk/langswitcher/download/?lang=uk",
        "https://panda-pishet-kod.dev/uk/langswitcher/?lang=uk",
    );

    assert_eq!(
        seen,
        [
            ("built in Ru", russian),
            ("En, not told", russian),
            ("En, told", english),
            ("Uk, told", ukrainian),
            ("Kk, told", russian),
        ],
        "181.33: the pages follow the language of the window, and move when it moves"
    );
}

#[cfg(feature = "testing")]
impl Drop for HiddenWindow {
    fn drop(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

        // SAFETY: both are windows of this thread made by `open`, destroyed once, window first.
        unsafe {
            let _ = DestroyWindow(self.window);
            let _ = DestroyWindow(self.owner);
        }
    }
}

/// What «От автора» says in the locale in force — the words `layout_author` measures, out of the
/// product's own `author_view` and string table (задачи T-83-2 и T-98-2).
///
/// A view is one window. Two of its lines change with the day and the feed — when the feed was
/// read, and what version is installed — and the tallest window has each of them at its tallest
/// (решение 166.2: for every line that changes, the longest the program can show).
/// `feed_states` and `version_states` are the sentences counted there, and the model takes the
/// tallest of each; the words of one window hold that window's own sentence only.
struct AuthorWords {
    view: letters::AuthorView,
    /// The sentences the line of the feed's reading is counted with.
    feed_states: Vec<String>,
    /// The sentences the line of the version is counted with.
    version_states: Vec<String>,
    /// The captions of the five push buttons the three panels may carry.
    support: String,
    channel: String,
    download: String,
    letters: String,
    write: String,
}

impl AuthorWords {
    /// The words of one window — `view`, and nothing besides.
    fn of(view: letters::AuthorView) -> Self {
        Self {
            feed_states: vec![view.feed_state.clone()],
            version_states: vec![view.version_state.clone()],
            view,
            support: settings::text(settings::IDS_SUPPORT_OPEN),
            channel: settings::text(settings::IDS_CHANNEL_OPEN),
            download: settings::text(settings::IDS_NEWS_DOWNLOAD),
            letters: settings::text(settings::IDS_NEWS_LETTERS),
            write: settings::text(settings::IDS_WRITE_TO_AUTHOR),
        }
    }

    /// The window of a first day, for `version`: a machine that has never read the feed — no
    /// switch of FR-102 and no «Последние письма».
    fn now(version: &str) -> Self {
        Self::of(letters::author_view(
            &settings::Letters::default(),
            letters::Date::from_ymd(2026, 9, 23).expect("a day"),
            version,
            letters::FeedView::EMPTY,
        ))
    }

    /// **The window of a machine whose feed is live, at its tallest — решение 166.2, задача
    /// T-98-2.** The published feed at full capacity, so «Последние письма» stands in the row beside
    /// «Открыть страницу загрузки»; the switch of FR-102 and the sentence under it (`switch`), or
    /// the line about the feed and the quiet line about the file.
    ///
    /// The line of the reading is counted on the 28th of **every** month — which name of a month is
    /// the widest is the language's own business — with the next reading a day away (fifteen until
    /// решение 189.2, and the two digits of «15» were what grew the greek window at 28 px); the
    /// line of the version **both** ways, «доступна» on a build the published update is ahead of
    /// and «это последняя» on one past it: in Greek and in English the second has more letters.
    fn fed(switch: bool, feed: &PublishedFeed) -> Self {
        let mut words = Self::of(fed_view(switch, 10, BEHIND_THE_FEED, feed));

        assert!(
            words.view.switch.is_some(),
            "since task T-119-10 the switch of «От автора» is always there"
        );
        assert_eq!(
            !words.view.switch_note.is_empty(),
            switch,
            "the line under the switch (the feed off in the file) must be what was asked for"
        );
        assert!(
            words.view.letters,
            "«Последние письма» must stand in the row: the feed has news"
        );

        words.feed_states = (1..=12)
            .map(|month| fed_view(switch, month, BEHIND_THE_FEED, feed).feed_state)
            .collect();
        words.version_states = [BEHIND_THE_FEED, PAST_THE_FEED]
            .iter()
            .map(|version| fed_view(switch, 10, version, feed).version_state)
            .collect();

        assert_ne!(
            words.version_states[0], words.version_states[1],
            "the two builds must say two things about the version"
        );

        words
    }
}

/// A build the published update is ahead of — «От автора» says «доступна 0.97.0».
const BEHIND_THE_FEED: &str = "0.96.0";
/// A build past the published update — «От автора» says it is the latest.
const PAST_THE_FEED: &str = "0.98.0";

/// **The feed as it stands published** at the base of stage Э98 — `news.toml` of 2026-10-08
/// (SHA-256 `01c8855e…aa91`): the update 0.97.0 (id 5) and the news item «Лента заработала» (id 2),
/// in the fourteen languages of the program; `(language, title and text of the news, title and
/// text of the update)`. Literals and not a read of the file: a test does not read the author's
/// machine (решение 166.2).
///
/// ⭐ **Stage E108: a language of group A joins with the texts its translator wrote for the feed**
/// (`l10n\feed\<тег>.txt` of the control folder, copied here as literals) — the news «Программа
/// теперь обновляется сама» (id 12) and the entry of the update, its version written as the
/// update of this table, 0.97.0: nothing of `e98` was ever published in them, and these are the
/// words the windows will hold. A slice, so that a language is one row.
const PUBLISHED_FEED: &[(Language, &str, &str, &str, &str)] = &[
    (
        Language::Ru,
        "Лента заработала",
        concat!(
            "Это первое письмо из ленты. Программа прочитала один файл с сайта автора,\n",
            "проверила подпись и показала вам это окно. Больше она ничего не отправляет\n",
            "и не скачивает: обновления вы ставите сами, ссылки открываются по щелчку.\n",
            "Отключить чтение можно в файле настроек, [letters] feed = false, а через\n",
            "90 дней — выключателем в окне «От автора».\n",
        ),
        "Вышла версия 0.97.0",
        "Новое: двойной Shift отвечает быстрее. Подробности — на странице загрузки.\n",
    ),
    (
        Language::En,
        "The feed is working",
        concat!(
            "This is the first letter from the feed. The program read one file from the\n",
            "author's site, checked the signature and showed you this window. It sends\n",
            "nothing else and downloads nothing: you install updates yourself and links\n",
            "open on a click. Reading can be turned off in the settings file,\n",
            "[letters] feed = false, and after ninety days with a switch in the\n",
            "«From the author» window.\n",
        ),
        "Version 0.97.0 is out",
        "New: the double Shift answers faster. Details are on the download page.\n",
    ),
    (
        Language::Uk,
        "Стрічка запрацювала",
        concat!(
            "Це перший лист зі стрічки. Програма прочитала один файл із сайту автора,\n",
            "перевірила підпис і показала вам це вікно. Більше вона нічого не надсилає\n",
            "і не завантажує.\n",
        ),
        "Вийшла версія 0.97.0",
        "Подробиці — на сторінці завантаження.\n",
    ),
    (
        Language::De,
        "Der Feed läuft",
        concat!(
            "Das ist der erste Brief aus dem Feed. Das Programm hat eine Datei von der\n",
            "Seite des Autors gelesen, die Signatur geprüft und Ihnen dieses Fenster\n",
            "gezeigt. Sonst sendet es nichts und lädt nichts herunter.\n",
        ),
        "Version 0.97.0 ist da",
        "Einzelheiten auf der Download-Seite.\n",
    ),
    (
        Language::Fr,
        "Le flux fonctionne",
        concat!(
            "Voici la première lettre du flux. Le programme a lu un fichier sur le site\n",
            "de l'auteur, vérifié la signature et vous a montré cette fenêtre. Il\n",
            "n'envoie rien d'autre et ne télécharge rien.\n",
        ),
        "La version 0.97.0 est sortie",
        "Les détails sont sur la page de téléchargement.\n",
    ),
    (
        Language::Es,
        "El canal funciona",
        concat!(
            "Esta es la primera carta del canal. El programa leyó un archivo del sitio\n",
            "del autor, comprobó la firma y le mostró esta ventana. No envía nada más\n",
            "ni descarga nada.\n",
        ),
        "Ha salido la versión 0.97.0",
        "Los detalles están en la página de descarga.\n",
    ),
    (
        Language::Pt,
        "O feed está funcionando",
        concat!(
            "Esta é a primeira carta do feed. O programa leu um arquivo do site do autor,\n",
            "verificou a assinatura e mostrou esta janela. Ele não envia mais nada e não\n",
            "baixa nada.\n",
        ),
        "Saiu a versão 0.97.0",
        "Os detalhes estão na página de download.\n",
    ),
    (
        Language::It,
        "Il flusso funziona",
        concat!(
            "Questa è la prima lettera dal flusso. Il programma ha letto un file dal sito\n",
            "dell'autore, ha verificato la firma e ti ha mostrato questa finestra. Non\n",
            "invia altro e non scarica nulla.\n",
        ),
        "È uscita la versione 0.97.0",
        "I dettagli sono nella pagina di download.\n",
    ),
    (
        Language::Pl,
        "Kanał działa",
        concat!(
            "To pierwszy list z kanału. Program przeczytał jeden plik ze strony autora,\n",
            "sprawdził podpis i pokazał ci to okno. Poza tym nic nie wysyła i nic nie\n",
            "pobiera.\n",
        ),
        "Wyszła wersja 0.97.0",
        "Szczegóły na stronie pobierania.\n",
    ),
    (
        Language::Cs,
        "Kanál funguje",
        concat!(
            "Toto je první dopis z kanálu. Program přečetl jeden soubor ze stránek autora,\n",
            "ověřil podpis a ukázal vám toto okno. Nic jiného neodesílá a nic nestahuje.\n",
        ),
        "Vyšla verze 0.97.0",
        "Podrobnosti najdete na stránce ke stažení.\n",
    ),
    (
        Language::Tr,
        "Akış çalışıyor",
        concat!(
            "Bu, akıştan gelen ilk mektup. Program yazarın sitesinden tek bir dosya okudu,\n",
            "imzayı doğruladı ve size bu pencereyi gösterdi. Bunun dışında hiçbir şey\n",
            "göndermez ve indirmez.\n",
        ),
        "0.97.0 sürümü çıktı",
        "Ayrıntılar indirme sayfasında.\n",
    ),
    (
        Language::El,
        "Η ροή λειτουργεί",
        concat!(
            "Αυτό είναι το πρώτο γράμμα από τη ροή. Το πρόγραμμα διάβασε ένα αρχείο από\n",
            "τον ιστότοπο του δημιουργού, έλεγξε την υπογραφή και σας έδειξε αυτό το\n",
            "παράθυρο. Τίποτε άλλο δεν στέλνει και τίποτε δεν κατεβάζει.\n",
        ),
        "Κυκλοφόρησε η έκδοση 0.97.0",
        "Λεπτομέρειες στη σελίδα λήψης.\n",
    ),
    (
        Language::He,
        "ההזנה עובדת",
        concat!(
            "זהו המכתב הראשון מההזנה. התוכנה קראה קובץ אחד מאתר המפתח, בדקה את החתימה\n",
            "והציגה לכם את החלון הזה. יותר מזה היא לא שולחת דבר ולא מורידה דבר.\n",
        ),
        "יצאה גרסה 0.97.0",
        "הפרטים בעמוד ההורדה.\n",
    ),
    (
        Language::Ar,
        "التغذية تعمل",
        concat!(
            "هذه أول رسالة من التغذية. قرأ البرنامج ملفًا واحدًا من موقع المؤلف، وتحقق من\n",
            "التوقيع، وعرض لك هذه النافذة. وما عدا ذلك لا يرسل شيئًا ولا ينزّل شيئًا.\n",
        ),
        "صدر الإصدار 0.97.0",
        "التفاصيل في صفحة التنزيل.\n",
    ),
    (
        Language::Hi,
        "प्रोग्राम अब अपने-आप अपडेट होता है",
        concat!(
            "संस्करण 0.104.0 से Lang Switcher नए संस्करण खुद डाउनलोड और इंस्टॉल करता है।\n",
            "फ़ाइलें लेखक की साइट से आती हैं, और इंस्टॉल से पहले हस्ताक्षर जाँचा जाता है।\n",
            "नया संस्करण एक बार इंस्टॉल कर लें — आगे के अपडेट अपने-आप आ जाएँगे।\n",
            "इस संस्करण में बिना Visual C++ पैकेज वाले Windows पर शुरू होना भी ठीक किया\n",
            "गया है: संस्करण 0.90.0–0.103.0 वहाँ शुरू नहीं होते थे।\n",
        ),
        "संस्करण 0.97.0 आ गया है",
        "विवरण डाउनलोड पृष्ठ पर हैं।\n",
    ),
    (
        Language::Ur,
        "پروگرام اب خود کو اپ ڈیٹ کرتا ہے",
        // ⚠ In lines, as the feed is written: «Последние письма» shows the first line of a text
        // as its excerpt, and the first version of the translator's file, one paragraph, made the
        // excerpt the whole text — the list grew from 719 to 928 px at 13 px (stage E108, 202.23).
        concat!(
            "ورژن 0.104.0 سے Lang Switcher نئے ورژن خود ڈاؤن لوڈ اور انسٹال کرتا ہے۔\n",
            "فائلیں ڈویلپر کی سائٹ سے آتی ہیں، اور انسٹال سے پہلے دستخط کی جانچ ہوتی ہے۔\n",
            "نیا ورژن ایک بار انسٹال کر لیں — اس کے بعد اپ ڈیٹس خود آ جائیں گی۔\n",
            "اس ورژن میں Visual C++ پیکیج کے بغیر Windows پر شروع ہونے کی خرابی بھی\n",
            "دور کر دی گئی ہے: ورژن 0.90.0 سے 0.103.0 وہاں شروع نہیں ہوتے تھے۔\n",
        ),
        "ورژن 0.97.0 جاری ہو گیا",
        "تفصیلات ڈاؤن لوڈ کے صفحے پر ہیں۔\n",
    ),
    (
        Language::Th,
        "โปรแกรมอัปเดตตัวเองได้แล้ว",
        concat!(
            "ตั้งแต่เวอร์ชัน 0.104.0 Lang Switcher ดาวน์โหลดเวอร์ชันใหม่เองได้แล้ว\n",
            "จากเว็บไซต์ของผู้พัฒนา แล้วติดตั้งให้เองหลังตรวจสอบลายเซ็น\n",
            "ติดตั้งเวอร์ชันใหม่เพียงครั้งเดียว แล้วการอัปเดตต่อไปจะมาเอง\n",
            "เวอร์ชันนี้ยังแก้การเริ่มทำงานบน Windows ที่ไม่มีแพ็กเกจ Visual C++ ด้วย\n",
            "เวอร์ชัน 0.90.0–0.103.0 เริ่มทำงานบนเครื่องแบบนั้นไม่ได้\n",
        ),
        "เวอร์ชัน 0.97.0 ออกแล้ว",
        "ดูรายละเอียดได้ที่หน้าดาวน์โหลด\n",
    ),
    (
        Language::Lo,
        "ໂປຣແກຣມອັບເດດຕົວເອງໄດ້ແລ້ວ",
        concat!(
            "ຕັ້ງແຕ່ເວີຊັນ 0.104.0 Lang Switcher ດາວໂຫຼດເວີຊັນໃໝ່ເອງໄດ້ແລ້ວ\n",
            "ຈາກເວັບໄຊຂອງຜູ້ພັດທະນາ ແລ້ວຕິດຕັ້ງໃຫ້ເອງຫຼັງກວດສອບລາຍເຊັນ\n",
            "ຕິດຕັ້ງເວີຊັນໃໝ່ພຽງເທື່ອດຽວ ແລ້ວການອັບເດດຕໍ່ໄປຈະມາເອງ\n",
            "ເວີຊັນນີ້ຍັງແກ້ການເລີ່ມເຮັດວຽກໃນ Windows ທີ່ບໍ່ມີແພັກເກດ Visual C++ ນຳ\n",
            "ເວີຊັນ 0.90.0–0.103.0 ເລີ່ມເຮັດວຽກໃນເຄື່ອງແບບນັ້ນບໍ່ໄດ້\n",
        ),
        "ເວີຊັນ 0.97.0 ອອກແລ້ວ",
        "ເບິ່ງລາຍລະອຽດໄດ້ທີ່ໜ້າດາວໂຫຼດ\n",
    ),
    (
        Language::Bn,
        "প্রোগ্রাম এখন নিজেই আপডেট হয়",
        concat!(
            "সংস্করণ 0.104.0 থেকে Lang Switcher নতুন সংস্করণ নিজেই ডাউনলোড ও ইনস্টল করে।\n",
            "ফাইল আসে নির্মাতার সাইট থেকে, আর ইনস্টলের আগে স্বাক্ষর যাচাই করা হয়।\n",
            "নতুন সংস্করণ একবার ইনস্টল করে নিন — পরের আপডেটগুলো নিজে থেকেই আসবে।\n",
            "এই সংস্করণে Visual C++ প্যাকেজ ছাড়া Windows-এ চালু হওয়ার সমস্যাও ঠিক\n",
            "করা হয়েছে: সংস্করণ 0.90.0–0.103.0 সেখানে চালু হত না।\n",
        ),
        "সংস্করণ 0.97.0 এসেছে",
        "বিস্তারিত ডাউনলোড পৃষ্ঠায় আছে।\n",
    ),
    (
        Language::Mr,
        "प्रोग्राम आता स्वतःच अद्यतनित होतो",
        concat!(
            "आवृत्ती 0.104.0 पासून Lang Switcher नवीन आवृत्त्या स्वतःच इन्स्टॉल करतो.\n",
            "त्या लेखकाच्या साइटवरून डाउनलोड होतात आणि स्वाक्षरी तपासल्यानंतरच\n",
            "इन्स्टॉल होतात. नवीन आवृत्ती एकदा इन्स्टॉल करा, पुढची अद्यतने आपोआप\n",
            "येतील. या आवृत्तीत Visual C++ पॅकेज नसलेल्या Windows वर प्रोग्राम सुरू\n",
            "न होण्याचीही दुरुस्ती केली आहे: आवृत्त्या 0.90.0 ते 0.103.0 तिथे सुरू\n",
            "होत नव्हत्या.\n",
        ),
        "आवृत्ती 0.97.0 आली आहे",
        "तपशील डाउनलोड पृष्ठावर आहेत.\n",
    ),
    (
        Language::Ne,
        "प्रोग्राम अब आफैँ अद्यावधिक हुन्छ",
        concat!(
            "संस्करण 0.104.0 देखि Lang Switcher नयाँ संस्करण आफैँ स्थापना गर्छ।\n",
            "ती लेखकको साइटबाट डाउनलोड हुन्छन् र हस्ताक्षर जाँचेपछि मात्र स्थापना\n",
            "हुन्छन्। नयाँ संस्करण एक पटक स्थापना गर्नुहोस्, उप्रान्तका अद्यावधिक\n",
            "आफैँ आउनेछन्। यो संस्करणमा Visual C++ प्याकेज नभएको Windows मा पनि\n",
            "प्रोग्राम सुरु हुने गरी सच्याइएको छ: संस्करण 0.90.0 देखि 0.103.0 सम्म\n",
            "त्यहाँ सुरु हुँदैनथे।\n",
        ),
        "संस्करण 0.97.0 आएको छ",
        "विवरण डाउनलोड पृष्ठमा छन्।\n",
    ),
    (
        Language::As,
        "প্ৰগ্ৰামটো এতিয়া নিজেই আপডেট হয়",
        concat!(
            "0.104.0-ৰ পৰা Lang Switcher-এ নতুন সংস্কৰণ নিজেই ডাউনলোড আৰু ইনষ্টল কৰে।\n",
            "ফাইল নিৰ্মাতাৰ ছাইটৰ পৰা আহে, আৰু ইনষ্টলৰ আগতে স্বাক্ষৰ পৰীক্ষা কৰা হয়।\n",
            "নতুন সংস্কৰণ এবাৰ ইনষ্টল কৰি লওক — পিছৰ আপডেটবোৰ নিজেই আহিব।\n",
            "এই সংস্কৰণত Visual C++ পেকেজ নথকা Windows-ত আৰম্ভ হোৱাৰ সমস্যাও শুধৰোৱা\n",
            "হৈছে: সংস্কৰণ 0.90.0–0.103.0 তাত আৰম্ভ হোৱা নাছিল।\n",
        ),
        "সংস্কৰণ 0.97.0 ওলাল",
        "বিৱৰণ ডাউনলোড পৃষ্ঠাত আছে।\n",
    ),
    (
        Language::Sr,
        "Програм се сада ажурира сам",
        concat!(
            "Од верзије 0.104.0 Lang Switcher сам преузима нове верзије са сајта аутора\n",
            "и инсталира их пошто провери потпис. Инсталирајте нову верзију једном —\n",
            "убудуће ће ажурирања стизати сама. У овој верзији је исправљено и\n",
            "покретање на систему Windows без пакета Visual C++: верзије 0.90.0–0.103.0\n",
            "тамо нису могле да се покрену.\n",
        ),
        "Изашла је верзија 0.97.0",
        "Детаљи су на страници за преузимање.\n",
    ),
    (
        Language::SrLatn,
        "Program se sada ažurira sam",
        concat!(
            "Od verzije 0.104.0 Lang Switcher sam preuzima nove verzije sa sajta autora\n",
            "i instalira ih pošto proveri potpis. Instalirajte novu verziju jednom —\n",
            "ubuduće će ažuriranja stizati sama. U ovoj verziji je ispravljeno i\n",
            "pokretanje na sistemu Windows bez paketa Visual C++: verzije 0.90.0–0.103.0\n",
            "tamo nisu mogle da se pokrenu.\n",
        ),
        "Izašla je verzija 0.97.0",
        "Detalji su na stranici za preuzimanje.\n",
    ),
    (
        Language::Fa,
        "برنامه اکنون خودش به‌روزرسانی می‌شود",
        concat!(
            "از نسخه‌ی 0.104.0، Lang Switcher نسخه‌های جدید را خودش بارگیری و نصب می‌کند.\n",
            "نسخه‌ها از سایت سازنده می‌آیند و پیش از نصب، امضا بررسی می‌شود.\n",
            "نسخه‌ی جدید را فقط یک بار نصب کنید — از آن پس به‌روزرسانی‌ها خودشان می‌رسند.\n",
            "در این نسخه، اجرا روی Windows بدون بسته‌ی Visual C++ هم درست شده است:\n",
            "نسخه‌های 0.90.0 تا 0.103.0 آنجا اجرا نمی‌شدند.\n",
        ),
        "نسخه‌ی 0.97.0 منتشر شد",
        "جزئیات در صفحه‌ی بارگیری است.\n",
    ),
    (
        Language::Ug,
        "پروگرامما ئەمدى ئۆزى يېڭىلىنىدۇ",
        concat!(
            "0.104.0 نەشرىدىن باشلاپ Lang Switcher يېڭى نەشرلەرنى ئۆزى چۈشۈرۈپ ئورنىتىدۇ.\n",
            "ئۇلار ئاپتورنىڭ تور بېكىتىدىن كېلىدۇ، ئورنىتىشتىن بۇرۇن ئىمزا تەكشۈرۈلىدۇ.\n",
            "يېڭى نەشرىنى بىر قېتىم ئورنىتىڭ — ئۇنىڭدىن كېيىن يېڭىلانمىلار ئۆزى كېلىدۇ.\n",
            "بۇ نەشرىدە Visual C++ بوغچىسى يوق Windows سىستېمىسىدا ئىجرا بولماسلىق\n",
            "مەسىلىسىمۇ تۈزىتىلدى: 0.90.0 – 0.103.0 نەشرلىرى ئۇ يەردە ئىجرا بولمايتتى.\n",
        ),
        "يېڭى نەشرى چىقتى: 0.97.0",
        "تەپسىلاتى چۈشۈرۈش بېتىدە.\n",
    ),
    (
        Language::Ta,
        "நிரல் இப்போது தானே புதுப்பித்துக்கொள்ளும்",
        concat!(
            "0.104.0 முதல் Lang Switcher புதிய பதிப்புகளைத் தானே நிறுவுகிறது.\n",
            "கோப்புகள் உருவாக்குநரின் தளத்திலிருந்து வருகின்றன, நிறுவும் முன் கையொப்பம்\n",
            "சரிபார்க்கப்படும். புதிய பதிப்பை ஒருமுறை நிறுவுங்கள் — இனி புதுப்பிப்புகள்\n",
            "தாமாகவே வரும். இந்தப் பதிப்பில் Visual C++ தொகுப்பு இல்லாத Windows\n",
            "கணினியிலும் நிரல் தொடங்கும்படி சரிசெய்யப்பட்டுள்ளது: 0.90.0–0.103.0\n",
            "பதிப்புகள் அங்கு தொடங்கவில்லை.\n",
        ),
        "பதிப்பு 0.97.0 வெளியானது",
        "விவரங்கள் பதிவிறக்கப் பக்கத்தில் உள்ளன.\n",
    ),
    (
        Language::Ml,
        "പ്രോഗ്രാം ഇനി സ്വയം അപ്ഡേറ്റ് ചെയ്യും",
        concat!(
            "0.104.0 മുതൽ Lang Switcher പുതിയ പതിപ്പുകൾ സ്വയം ഇൻസ്റ്റാൾ ചെയ്യുന്നു.\n",
            "ഫയലുകൾ നിർമ്മാതാവിന്റെ സൈറ്റിൽ നിന്ന് വരുന്നു, ഇൻസ്റ്റാൾ ചെയ്യുന്നതിന്\n",
            "മുമ്പ് ഒപ്പ് പരിശോധിക്കും. പുതിയ പതിപ്പ് ഒരിക്കൽ ഇൻസ്റ്റാൾ ചെയ്യുക —\n",
            "പിന്നീടുള്ള അപ്ഡേറ്റുകൾ സ്വയം വരും. ഈ പതിപ്പിൽ Visual C++ പാക്കേജ്\n",
            "ഇല്ലാത്ത Windows കമ്പ്യൂട്ടറിലും പ്രോഗ്രാം ആരംഭിക്കുന്ന വിധം ശരിയാക്കി:\n",
            "0.90.0–0.103.0 പതിപ്പുകൾ അവിടെ ആരംഭിച്ചിരുന്നില്ല.\n",
        ),
        "പതിപ്പ് 0.97.0 പുറത്തിറങ്ങി",
        "വിശദാംശങ്ങൾ ഡൗൺലോഡ് പേജിലുണ്ട്.\n",
    ),
    (
        Language::Uz,
        "Dastur endi oʻzi yangilanadi",
        concat!(
            "0.104.0 versiyasidan boshlab Lang Switcher yangilanishlarni oʻzi oʻrnatadi.\n",
            "Yangi versiyalar muallif saytidan yuklab olinadi, oʻrnatishdan oldin esa\n",
            "imzo tekshiriladi. Yangi versiyani bir marta oʻrnating — keyingi\n",
            "yangilanishlar oʻzi keladi. Bu versiyada Visual C++ paketi yoʻq Windows\n",
            "tizimida ishga tushish ham tuzatildi: 0.90.0–0.103.0 versiyalari u yerda\n",
            "ishga tushmasdi.\n",
        ),
        "0.97.0 versiyasi chiqdi",
        "Tafsilotlar yuklab olish sahifasida.\n",
    ),
    (
        Language::UzCyrl,
        "Дастур энди ўзи янгиланади",
        concat!(
            "0.104.0 версиясидан бошлаб Lang Switcher янгиланишларни ўзи ўрнатади.\n",
            "Янги версиялар муаллиф сайтидан юклаб олинади, ўрнатишдан олдин эса\n",
            "имзо текширилади. Янги версияни бир марта ўрнатинг — кейинги\n",
            "янгиланишлар ўзи келади. Бу версияда Visual C++ пакети йўқ Windows\n",
            "тизимида ишга тушиш ҳам тузатилди: 0.90.0–0.103.0 версиялари у ерда\n",
            "ишга тушмасди.\n",
        ),
        "0.97.0 версияси чиқди",
        "Тафсилотлар юклаб олиш саҳифасида.\n",
    ),
    (
        Language::Te,
        "ప్రోగ్రామ్ ఇప్పుడు తనంతట తానే నవీకరించుకుంటుంది",
        concat!(
            "0.104.0 నుండి Lang Switcher కొత్త సంస్కరణలను తానే ఇన్‌స్టాల్ చేసుకుంటుంది.\n",
            "ఫైళ్లు రచయిత సైట్ నుండి వస్తాయి; ఇన్‌స్టాల్‌కు ముందు సంతకం తనిఖీ అవుతుంది.\n",
            "కొత్త సంస్కరణను ఒక్కసారి ఇన్‌స్టాల్ చేయండి — తర్వాతి నవీకరణలు\n",
            "వాటంతట అవే వస్తాయి. ఈ సంస్కరణలో Visual C++ ప్యాకేజీ లేని Windows లో\n",
            "ప్రారంభం కావడం కూడా సరిచేయబడింది: సంస్కరణలు 0.90.0–0.103.0 అక్కడ\n",
            "ప్రారంభం కాలేదు.\n",
        ),
        "సంస్కరణ 0.97.0 విడుదలైంది",
        "వివరాలు డౌన్‌లోడ్ పేజీలో ఉన్నాయి.\n",
    ),
    (
        Language::Kn,
        "ಪ್ರೋಗ್ರಾಂ ಈಗ ತಾನೇ ನವೀಕರಿಸಿಕೊಳ್ಳುತ್ತದೆ",
        concat!(
            "0.104.0 ರಿಂದ Lang Switcher ಹೊಸ ಆವೃತ್ತಿಗಳನ್ನು ತಾನೇ ಸ್ಥಾಪಿಸಿಕೊಳ್ಳುತ್ತದೆ.\n",
            "ಫೈಲ್‌ಗಳು ಲೇಖಕರ ಸೈಟ್‌ನಿಂದ ಬರುತ್ತವೆ; ಸ್ಥಾಪನೆಗೆ ಮುನ್ನ ಸಹಿ ಪರಿಶೀಲನೆಯಾಗುತ್ತದೆ.\n",
            "ಹೊಸ ಆವೃತ್ತಿಯನ್ನು ಒಮ್ಮೆ ಸ್ಥಾಪಿಸಿ — ಮುಂದಿನ ನವೀಕರಣಗಳು ತಾನಾಗಿಯೇ\n",
            "ಬರುತ್ತವೆ. ಈ ಆವೃತ್ತಿಯಲ್ಲಿ Visual C++ ಪ್ಯಾಕೇಜ್ ಇಲ್ಲದ Windows ನಲ್ಲಿ\n",
            "ಪ್ರಾರಂಭವಾಗದಿರುವುದೂ ಸರಿಪಡಿಸಲಾಗಿದೆ: ಆವೃತ್ತಿಗಳು 0.90.0–0.103.0 ಅಲ್ಲಿ\n",
            "ಪ್ರಾರಂಭವಾಗುತ್ತಿರಲಿಲ್ಲ.\n",
        ),
        "ಆವೃತ್ತಿ 0.97.0 ಬಿಡುಗಡೆಯಾಗಿದೆ",
        "ವಿವರಗಳು ಡೌನ್‌ಲೋಡ್ ಪುಟದಲ್ಲಿವೆ.\n",
    ),
    (
        Language::Pa,
        "ਪ੍ਰੋਗਰਾਮ ਹੁਣ ਆਪਣੇ-ਆਪ ਅੱਪਡੇਟ ਹੁੰਦਾ ਹੈ",
        concat!(
            "ਵਰਜਨ 0.104.0 ਤੋਂ Lang Switcher ਨਵੇਂ ਵਰਜਨ ਆਪ ਡਾਊਨਲੋਡ ਅਤੇ ਇੰਸਟਾਲ ਕਰਦਾ ਹੈ।\n",
            "ਫ਼ਾਈਲਾਂ ਲੇਖਕ ਦੀ ਸਾਈਟ ਤੋਂ ਆਉਂਦੀਆਂ ਹਨ, ਇੰਸਟਾਲ ਤੋਂ ਪਹਿਲਾਂ ਦਸਤਖ਼ਤ ਜਾਂਚੇ ਜਾਂਦੇ ਹਨ।\n",
            "ਨਵਾਂ ਵਰਜਨ ਇੱਕ ਵਾਰ ਇੰਸਟਾਲ ਕਰ ਲਓ — ਅੱਗੋਂ ਅੱਪਡੇਟ ਆਪਣੇ-ਆਪ ਆ ਜਾਣਗੇ।\n",
            "ਇਸ ਵਰਜਨ ਵਿੱਚ Visual C++ ਪੈਕੇਜ ਤੋਂ ਬਿਨਾਂ Windows ਉੱਤੇ ਸ਼ੁਰੂ ਹੋਣਾ ਵੀ ਠੀਕ\n",
            "ਕੀਤਾ ਗਿਆ ਹੈ: ਵਰਜਨ 0.90.0–0.103.0 ਉੱਥੇ ਸ਼ੁਰੂ ਨਹੀਂ ਹੁੰਦੇ ਸਨ।\n",
        ),
        "ਵਰਜਨ 0.97.0 ਆ ਗਿਆ ਹੈ",
        "ਵੇਰਵੇ ਡਾਊਨਲੋਡ ਪੰਨੇ ਤੇ ਹਨ।\n",
    ),
    (
        Language::Gu,
        "પ્રોગ્રામ હવે જાતે અપડેટ થાય છે",
        concat!(
            "વર્ઝન 0.104.0 થી Lang Switcher નવાં વર્ઝન જાતે ડાઉનલોડ અને ઇન્સ્ટોલ કરે છે.\n",
            "ફાઇલો લેખકની સાઇટ પરથી આવે છે, અને ઇન્સ્ટોલ પહેલાં હસ્તાક્ષર ચકાસાય છે.\n",
            "નવું વર્ઝન એક વાર ઇન્સ્ટોલ કરી લો — આગળના અપડેટ જાતે આવી જશે.\n",
            "આ વર્ઝનમાં Visual C++ પૅકેજ વગરના Windows પર શરૂ થવાનું પણ ઠીક કર્યું છે:\n",
            "વર્ઝન 0.90.0–0.103.0 ત્યાં શરૂ થતાં નહોતાં.\n",
        ),
        "વર્ઝન 0.97.0 આવી ગયું છે",
        "વિગતો ડાઉનલોડ પૃષ્ઠ પર છે.\n",
    ),
    (
        Language::Az,
        "Proqram artıq özünü yeniləyir",
        concat!(
            "Lang Switcher 0.104.0 versiyasından yeni versiyaları özü yükləyir\n",
            "və quraşdırır: faylları müəllifin saytından götürür, imzanı yoxlayır.\n",
            "Yeni versiyanı bir dəfə quraşdırın — sonra yeniləmələr özü gələcək.\n",
            "Bu versiyada Visual C++ paketi olmayan Windows sistemində proqramın\n",
            "işə düşməməsi də düzəldilib: 0.90.0–0.103.0 versiyaları orada işə\n",
            "düşmürdü.\n",
        ),
        "0.97.0 versiyası çıxdı",
        "Ətraflı məlumat yükləmə səhifəsindədir.\n",
    ),
    (
        Language::Be,
        "Праграма цяпер абнаўляецца сама",
        concat!(
            "Пачынаючы з версіі 0.104.0, Lang Switcher сам спампоўвае новыя версіі\n",
            "з сайта аўтара і ўсталёўвае іх, праверыўшы подпіс. Усталюйце новую\n",
            "версію адзін раз — далей абнаўленні прыйдуць самі. У гэтай версіі\n",
            "выпраўлены і запуск на Windows без пакета Visual C++: версіі\n",
            "0.90.0–0.103.0 там не запускаліся.\n",
        ),
        "Выйшла версія 0.97.0",
        "Падрабязнасці — на старонцы загрузкі.\n",
    ),
    (
        Language::Kk,
        "Бағдарлама енді өзі жаңарады",
        concat!(
            "0.104.0 нұсқасынан бастап Lang Switcher өзі жаңарады.\n",
            "Жаңа нұсқаларды автордың сайтынан жүктеп алып, қолтаңбаны тексергеннен\n",
            "кейін өзі орнатады. Жаңа нұсқаны бір рет орнатыңыз — одан әрі жаңартулар\n",
            "өздігінен келіп тұрады. Бұл нұсқада Visual C++ пакеті жоқ Windows\n",
            "жүйесінде іске қосылу да түзетілді: 0.90.0–0.103.0 нұсқалары онда іске\n",
            "қосылмайтын еді.\n",
        ),
        "0.97.0 нұсқасы шықты",
        "Толығырақ — жүктеп алу бетінде.\n",
    ),
    (
        Language::Tt,
        "Программа хәзер үз-үзен яңарта",
        concat!(
            "0.104.0 версиясеннән башлап Lang Switcher үз-үзен яңарта.\n",
            "Яңа версияләрне ул автор сайтыннан йөкләп ала һәм имзаны тикшергәч\n",
            "урнаштыра. Яңа версияне бер тапкыр урнаштырыгыз — алга таба яңартулар\n",
            "үзеннән-үзе киләчәк. Бу версиядә Visual C++ пакеты булмаган Windows\n",
            "системасында эшләтеп җибәрү дә төзәтелде: 0.90.0–0.103.0 версияләре\n",
            "анда эшләми иде.\n",
        ),
        "0.97.0 версиясе чыкты",
        "Тулырак мәгълүмат — йөкләп алу битендә.\n",
    ),
    (
        Language::Bg,
        "Програмата вече се актуализира сама",
        concat!(
            "Lang Switcher вече сам изтегля и инсталира новите си версии.\n",
            "Считано от 0.104.0, програмата взема версиите от сайта на автора и ги\n",
            "инсталира, след като провери подписа. Инсталирайте новата версия\n",
            "веднъж — оттук нататък актуализациите идват сами. В тази версия е\n",
            "поправено и стартирането на Windows без пакета Visual C++: версии\n",
            "0.90.0–0.103.0 там не се стартираха.\n",
        ),
        "Излезе версия 0.97.0",
        "Подробности — на страницата за изтегляне.\n",
    ),
    (
        Language::Mk,
        "Програмата сега се ажурира сама",
        concat!(
            "Lang Switcher сега сам ги презема и инсталира новите верзии.\n",
            "Од верзија 0.104.0 програмата ги зема верзиите од сајтот на авторот и\n",
            "ги инсталира откако ќе го провери потписот. Инсталирајте ја новата\n",
            "верзија еднаш — натаму ажурирањата доаѓаат сами. Во оваа верзија е\n",
            "поправено и стартувањето на Windows без пакетот Visual C++: верзиите\n",
            "0.90.0–0.103.0 таму не се стартуваа.\n",
        ),
        "Излезе верзија 0.97.0",
        "Подробностите се на страницата за преземање.\n",
    ),
    (
        Language::Or,
        "ପ୍ରୋଗ୍ରାମ ଏବେ ନିଜେ ଅପଡେଟ୍ ହୁଏ",
        concat!(
            "ସଂସ୍କରଣ 0.104.0 ଠାରୁ Lang Switcher ନିଜେ ଅପଡେଟ୍ ହୁଏ: ଲେଖକଙ୍କ ସାଇଟରୁ\n",
            "ନୂଆ ସଂସ୍କରଣ ଡାଉନଲୋଡ୍ କରେ ଏବଂ ସ୍ୱାକ୍ଷର ଯାଞ୍ଚ କରି ଇନଷ୍ଟଲ୍ କରେ। ନୂଆ ସଂସ୍କରଣ\n",
            "ଥରେ ଇନଷ୍ଟଲ୍ କରନ୍ତୁ — ତା ପରେ ଅପଡେଟ୍ ନିଜେ ଆସିବ। ଏହି ସଂସ୍କରଣରେ Visual C++\n",
            "ପ୍ୟାକେଜ୍ ନଥିବା Windows ରେ ଆରମ୍ଭ ହେବାର ସମସ୍ୟା ମଧ୍ୟ ଠିକ୍ ହୋଇଛି:\n",
            "0.90.0–0.103.0 ସଂସ୍କରଣ ସେଠାରେ ଚାଲୁନଥିଲା।\n",
        ),
        "ସଂସ୍କରଣ 0.97.0 ଆସିଗଲା",
        "ବିବରଣୀ ଡାଉନଲୋଡ୍ ପୃଷ୍ଠାରେ ଅଛି।\n",
    ),
    (
        Language::Km,
        "កម្មវិធីអាចធ្វើបច្ចុប្បន្នភាពខ្លួនឯងបានហើយ",
        concat!(
            "ចាប់ពីកំណែ 0.104.0 Lang Switcher ទាញយកកំណែថ្មីពីគេហទំព័រអ្នកបង្កើត\n",
            "ដោយខ្លួនឯង ហើយដំឡើងវា បន្ទាប់ពីពិនិត្យហត្ថលេខា។ គ្រាន់តែដំឡើងកំណែថ្មី\n",
            "ម្ដង បន្ទាប់ពីនោះបច្ចុប្បន្នភាពនឹងមកដោយខ្លួនឯង។ កំណែនេះក៏បានដោះស្រាយ\n",
            "ការចាប់ផ្ដើមលើ Windows ដែលគ្មានកញ្ចប់ Visual C++ ផងដែរ: កំណែ\n",
            "0.90.0–0.103.0 មិនអាចចាប់ផ្ដើមនៅទីនោះបានទេ។\n",
        ),
        "កំណែ 0.97.0 បានចេញហើយ",
        "សូមមើលព័ត៌មានលម្អិតនៅទំព័រទាញយក។\n",
    ),
    (
        Language::Ka,
        "პროგრამა ახლა თავად განახლდება",
        concat!(
            "ვერსია 0.104.0-დან Lang Switcher ახალ ვერსიებს თავად ჩამოტვირთავს\n",
            "ავტორის საიტიდან და ხელმოწერის შემოწმების შემდეგ თავადვე დააინსტალირებს.\n",
            "დააინსტალირეთ ახალი ვერსია ერთხელ — შემდეგ განახლებები თავისით მოვა.\n",
            "ამ ვერსიაში გასწორებულია გაშვებაც Windows-ზე Visual C++ პაკეტის გარეშე:\n",
            "ვერსიები 0.90.0–0.103.0 იქ არ ეშვებოდა.\n",
        ),
        "გამოვიდა ვერსია 0.97.0",
        "დეტალები ჩამოტვირთვის გვერდზეა.\n",
    ),
    (
        Language::Hy,
        "Ծրագիրը հիմա ինքն է թարմացվում",
        concat!(
            "0.104.0 տարբերակից Lang Switcher-ը նոր տարբերակներն ինքն է ներբեռնում\n",
            "հեղինակի կայքից և տեղադրում՝ նախապես ստուգելով ստորագրությունը։\n",
            "Նոր տարբերակը տեղադրեք մեկ անգամ, հետո թարմացումները կգան ինքնուրույն։\n",
            "Այս տարբերակում ուղղված է նաև Visual C++ փաթեթ չունեցող Windows-ում\n",
            "գործարկումը. 0.90.0–0.103.0 տարբերակներն այնտեղ չէին գործարկվում։\n",
        ),
        "Թողարկվեց 0.97.0 տարբերակը",
        "Մանրամասները՝ ներբեռնման էջում։\n",
    ),
    (
        Language::Hr,
        "Program se sada ažurira sam",
        concat!(
            "Lang Switcher se od verzije 0.104.0 ažurira sam.\n",
            "Nove verzije preuzima s autorova web-mjesta i instalira ih nakon provjere\n",
            "potpisa. Instalirajte novu verziju jednom, a dalje će ažuriranja stizati\n",
            "sama. U ovoj je verziji popravljeno i pokretanje na sustavu Windows bez\n",
            "paketa Visual C++: verzije 0.90.0–0.103.0 ondje se nisu pokretale.\n",
        ),
        "Izašla je verzija 0.97.0",
        "Pojedinosti su na stranici za preuzimanje.\n",
    ),
    (
        Language::Sk,
        "Program sa teraz aktualizuje sám",
        concat!(
            "Od verzie 0.104.0 sa Lang Switcher aktualizuje sám.\n",
            "Nové verzie sťahuje zo stránky autora a po overení podpisu ich\n",
            "nainštaluje. Novú verziu nainštalujte raz — ďalšie aktualizácie prídu\n",
            "samy. Táto verzia opravuje aj spúšťanie v systéme Windows bez balíka\n",
            "Visual C++: verzie 0.90.0–0.103.0 sa tam nespúšťali.\n",
        ),
        "Vyšla verzia 0.97.0",
        "Podrobnosti nájdete na stránke na stiahnutie.\n",
    ),
    (
        Language::Sl,
        "Program se zdaj posodablja sam",
        concat!(
            "Od različice 0.104.0 se Lang Switcher posodablja sam.\n",
            "Nove različice prenese s spletnega mesta avtorja in jih po preverjanju\n",
            "podpisa namesti. Novo različico namestite enkrat — nato bodo posodobitve\n",
            "prišle same. Ta različica popravlja tudi zagon v sistemu Windows brez\n",
            "paketa Visual C++: različice 0.90.0–0.103.0 se tam niso zagnale.\n",
        ),
        "Izšla je različica 0.97.0",
        "Podrobnosti so na strani za prenos.\n",
    ),
    (
        Language::Bs,
        "Program se sada ažurira sam",
        concat!(
            "Lang Switcher se od verzije 0.104.0 ažurira sam.\n",
            "Nove verzije preuzima sa sajta autora i instalira ih nakon provjere potpisa.\n",
            "Instalirajte novu verziju jednom, a dalje će ažuriranja stizati sama.\n",
            "U ovoj verziji popravljeno je i pokretanje na sistemu Windows bez paketa\n",
            "Visual C++: verzije 0.90.0–0.103.0 tamo se nisu pokretale.\n",
        ),
        "Izašla je verzija 0.97.0",
        "Detalji su na stranici za preuzimanje.\n",
    ),
    (
        Language::Lt,
        "Programa dabar atsinaujina pati",
        concat!(
            "Lang Switcher dabar pats atsisiunčia ir įdiegia naujas versijas.\n",
            "Nuo versijos 0.104.0 programa jas ima iš autoriaus svetainės ir diegia\n",
            "pati, patikrinusi skaitmeninį parašą. Įdiekite naują versiją vieną\n",
            "kartą — toliau atnaujinimai ateis patys. Šioje versijoje taip pat\n",
            "ištaisytas paleidimas sistemoje Windows be Visual C++ paketo: versijos\n",
            "0.90.0–0.103.0 ten nepasileisdavo.\n",
        ),
        "Išleista versija 0.97.0",
        "Išsamiau — atsisiuntimo puslapyje.\n",
    ),
    (
        Language::Lv,
        "Programma tagad atjaunināsies pati",
        concat!(
            "Lang Switcher tagad pats lejupielādē un instalē jaunās versijas.\n",
            "Sākot ar versiju 0.104.0, programma ņem tās no autora vietnes un instalē\n",
            "pati, pārbaudījusi digitālo parakstu. Instalējiet jauno versiju vienu\n",
            "reizi — turpmāk atjauninājumi parādīsies paši. Šajā versijā labota arī\n",
            "palaišana sistēmā Windows bez Visual C++ pakotnes: versijas\n",
            "0.90.0–0.103.0 tur nepalaidās.\n",
        ),
        "Iznākusi versija 0.97.0",
        "Sīkāka informācija — lejupielādes lapā.\n",
    ),
    (
        Language::Sv,
        "Programmet uppdaterar nu sig självt",
        concat!(
            "Från och med 0.104.0 uppdaterar Lang Switcher sig självt.\n",
            "Programmet hämtar nya versioner från utvecklarens webbplats och installerar\n",
            "dem efter att ha kontrollerat signaturen. Installera den nya versionen\n",
            "en gång, så kommer uppdateringarna av sig själva. Den här versionen\n",
            "åtgärdar också starten i Windows utan Visual C++-paketet: versionerna\n",
            "0.90.0–0.103.0 startade inte där.\n",
        ),
        "Version 0.97.0 har släppts",
        "Detaljer finns på nedladdningssidan.\n",
    ),
    (
        Language::Da,
        "Programmet opdaterer nu sig selv",
        concat!(
            "Fra og med 0.104.0 opdaterer Lang Switcher sig selv.\n",
            "Programmet henter nye versioner fra udviklerens websted og installerer\n",
            "dem, efter at have kontrolleret signaturen. Installer den nye version én\n",
            "gang, så kommer opdateringerne af sig selv. Denne version retter også\n",
            "opstart på Windows uden Visual C++-pakken: Versionerne 0.90.0–0.103.0\n",
            "startede ikke dér.\n",
        ),
        "Version 0.97.0 er udgivet",
        "Detaljer findes på downloadsiden.\n",
    ),
    (
        Language::Fi,
        "Ohjelma päivittyy nyt itsestään",
        concat!(
            "Lang Switcher lataa ja asentaa nyt uudet versiot itse.\n",
            "Versiosta 0.104.0 alkaen se hakee ne tekijän sivustolta ja asentaa\n",
            "ne tarkistettuaan digitaalisen allekirjoituksen. Asenna uusi versio\n",
            "kerran, niin päivitykset tulevat jatkossa itsestään. Tämä versio\n",
            "korjaa myös käynnistymisen Windowsissa, jossa ei ole Visual C++ -pakettia:\n",
            "versiot 0.90.0–0.103.0 eivät käynnistyneet siellä.\n",
        ),
        "Versio 0.97.0 on julkaistu",
        "Lisätietoja on lataussivulla.\n",
    ),
    (
        Language::Nb,
        "Programmet oppdaterer seg nå selv",
        concat!(
            "Fra og med 0.104.0 oppdaterer Lang Switcher seg selv.\n",
            "Programmet laster ned nye versjoner fra utviklerens nettsted og\n",
            "installerer dem etter å ha kontrollert signaturen. Installer den nye\n",
            "versjonen én gang, så kommer oppdateringene av seg selv. Denne\n",
            "versjonen retter også oppstart på Windows uten Visual C++-pakken:\n",
            "versjonene 0.90.0–0.103.0 startet ikke der.\n",
        ),
        "Versjon 0.97.0 er ute",
        "Detaljer finnes på nedlastingssiden.\n",
    ),
    (
        Language::Et,
        "Programm värskendab nüüd ennast ise",
        concat!(
            "Lang Switcher laadib ja paigaldab nüüd uued versioonid ise.\n",
            "Alates versioonist 0.104.0 võtab programm need autori saidilt ja\n",
            "paigaldab pärast digiallkirja kontrollimist. Paigaldage uus versioon\n",
            "ühe korra, edaspidi tulevad värskendused ise. See versioon parandab ka\n",
            "käivitumise Windowsis, kus puudub Visual C++ pakett: versioonid\n",
            "0.90.0–0.103.0 seal ei käivitunud.\n",
        ),
        "Versioon 0.97.0 on välja antud",
        "Üksikasjad on allalaadimislehel.\n",
    ),
    (
        Language::Nn,
        "Programmet oppdaterer seg no sjølv",
        concat!(
            "Frå og med 0.104.0 oppdaterer Lang Switcher seg sjølv.\n",
            "Programmet lastar ned nye versjonar frå nettstaden til utviklaren og\n",
            "installerer dei etter at signaturen er kontrollert. Installer den nye\n",
            "versjonen éin gong, så kjem oppdateringane av seg sjølv. Denne\n",
            "versjonen rettar òg oppstart på Windows utan Visual C++-pakken:\n",
            "versjonane 0.90.0–0.103.0 starta ikkje der.\n",
        ),
        "Versjon 0.97.0 er ute",
        "Detaljar finst på nedlastingssida.\n",
    ),
    (
        Language::Nl,
        "Het programma werkt zichzelf nu bij",
        concat!(
            "Sinds 0.104.0 werkt Lang Switcher zichzelf bij.\n",
            "Nieuwe versies downloadt het van de site van de auteur en installeert het\n",
            "zelf, na controle van de handtekening. Installeer de nieuwe versie één\n",
            "keer, daarna komen de updates vanzelf. Deze versie lost ook het starten op\n",
            "Windows zonder het Visual C++-pakket op: de versies 0.90.0 tot en met\n",
            "0.103.0 startten daar niet.\n",
        ),
        "Versie 0.97.0 is verschenen",
        "Details staan op de downloadpagina.\n",
    ),
    (
        Language::Lb,
        "De Programm aktualiséiert sech elo selwer",
        concat!(
            "Zënter 0.104.0 aktualiséiert Lang Switcher sech selwer.\n",
            "Nei Versioune luet hie vum Site vum Auteur erof an installéiert se selwer,\n",
            "nodeems d'Ënnerschrëft kontrolléiert ass. Installéiert déi nei Versioun\n",
            "eemol, duerno kommen d'Updates vun eleng. An dëser Versioun ass och de\n",
            "Start op Windows ouni de Visual-C++-Package korrigéiert: d'Versioune\n",
            "0.90.0 bis 0.103.0 sinn do net gestart.\n",
        ),
        "D'Versioun 0.97.0 ass do",
        "Détailer stinn op der Downloadsäit.\n",
    ),
    (
        Language::Cy,
        "Mae'r rhaglen bellach yn diweddaru ei hun",
        concat!(
            "Ers 0.104.0, mae Lang Switcher yn diweddaru ei hun.\n",
            "Mae'n lawrlwytho fersiynau newydd o wefan yr awdur ac yn eu gosod,\n",
            "ar ôl gwirio'r llofnod. Gosodwch y fersiwn newydd unwaith, a daw'r\n",
            "diweddariadau ohonynt eu hunain. Mae'r fersiwn hon hefyd yn trwsio\n",
            "cychwyn ar Windows heb y pecyn Visual C++: ni chychwynnodd fersiynau\n",
            "0.90.0 i 0.103.0 yno.\n",
        ),
        "Rhyddhawyd fersiwn 0.97.0",
        "Mae'r manylion ar y dudalen lawrlwytho.\n",
    ),
    (
        Language::Is,
        "Forritið uppfærir sig nú sjálft",
        concat!(
            "Frá og með 0.104.0 uppfærir Lang Switcher sig sjálft.\n",
            "Forritið sækir nýjar útgáfur af vef höfundar og setur þær upp sjálft,\n",
            "eftir að hafa athugað undirskriftina. Settu nýju útgáfuna upp einu sinni\n",
            "og uppfærslurnar koma af sjálfu sér. Þessi útgáfa lagar líka ræsingu á\n",
            "Windows án Visual C++ pakkans: útgáfur 0.90.0 til 0.103.0 ræstust ekki þar.\n",
        ),
        "Útgáfa 0.97.0 er komin út",
        "Nánari upplýsingar eru á niðurhalssíðunni.\n",
    ),
    (
        Language::Vi,
        "Chương trình giờ đây tự cập nhật",
        concat!(
            "Từ phiên bản 0.104.0, Lang Switcher tự cập nhật.\n",
            "Chương trình tự tải các phiên bản mới từ trang web của tác giả và\n",
            "cài đặt chúng sau khi kiểm tra chữ ký. Chỉ cần cài phiên bản mới một lần —\n",
            "các bản cập nhật sau đó sẽ tự đến. Phiên bản này cũng sửa lỗi khởi động\n",
            "trên Windows không có gói Visual C++: các phiên bản 0.90.0–0.103.0 không\n",
            "chạy được ở đó.\n",
        ),
        "Đã ra mắt phiên bản 0.97.0",
        "Chi tiết có trên trang tải xuống.\n",
    ),
    (
        Language::Ro,
        "Programul se actualizează acum singur",
        concat!(
            "Lang Switcher descarcă și instalează singur versiunile noi.\n",
            "Începând cu versiunea 0.104.0, le descarcă de pe site-ul autorului și\n",
            "le instalează după ce verifică semnătura digitală. Instalați noua\n",
            "versiune o singură dată, iar actualizările vor veni de la sine. Această\n",
            "versiune repară și pornirea pe un Windows fără pachetul Visual C++:\n",
            "versiunile 0.90.0–0.103.0 nu porneau acolo.\n",
        ),
        "A apărut versiunea 0.97.0",
        "Detaliile se găsesc pe pagina de descărcare.\n",
    ),
    (
        Language::Hu,
        "A program mostantól magától frissül",
        concat!(
            "A Lang Switcher mostantól maga tölti le és telepíti az új verziókat.\n",
            "A 0.104.0 verziótól kezdve a szerző webhelyéről tölti le őket, és az\n",
            "aláírás ellenőrzése után telepíti. Telepítse az új verziót egyszer,\n",
            "és a frissítések attól kezdve maguktól jönnek. Ez a verzió azt is\n",
            "javítja, hogy a program elinduljon olyan Windowson, amelyen nincs\n",
            "Visual C++ csomag: a 0.90.0–0.103.0 verziók ott nem indultak el.\n",
        ),
        "Megjelent a(z) 0.97.0 verzió",
        "A részletek a letöltési oldalon találhatók.\n",
    ),
    (
        Language::Sq,
        "Programi tani përditësohet vetë",
        concat!(
            "Që nga 0.104.0, Lang Switcher i shkarkon vetë versionet e reja.\n",
            "I merr nga faqja e autorit dhe i instalon, pasi kontrollon nënshkrimin.\n",
            "Instaloni versionin e ri një herë dhe përditësimet do të vijnë vetë.\n",
            "Ky version rregullon edhe nisjen në Windows pa paketën Visual C++:\n",
            "versionet 0.90.0–0.103.0 nuk niseshin atje.\n",
        ),
        "Doli versioni 0.97.0",
        "Detajet janë në faqen e shkarkimit.\n",
    ),
    (
        Language::Mt,
        "Il-programm issa jaġġorna lilu nnifsu",
        concat!(
            "Mill-verżjoni 0.104.0 Lang Switcher jniżżel il-verżjonijiet il-ġodda waħdu.\n",
            "Jieħdhom mis-sit tal-awtur u jinstallahom, wara li jivverifika l-firma.\n",
            "Installa l-verżjoni l-ġdida darba waħda u l-aġġornamenti jaslu waħedhom.\n",
            "Din il-verżjoni ssewwi wkoll il-bidu fuq Windows mingħajr il-pakkett\n",
            "Visual C++: il-verżjonijiet 0.90.0–0.103.0 ma kinux jibdew hemm.\n",
        ),
        "Ħarġet il-verżjoni 0.97.0",
        "Id-dettalji jinsabu fil-paġna tat-tniżżil.\n",
    ),
    (
        Language::Mi,
        "Ka whakahou aunoa te hōtaka ināianei",
        concat!(
            "Mai i te putanga 0.104.0, ka whakahou aunoa a Lang Switcher.\n",
            "Ka tikiake te hōtaka i ngā putanga hou mai i te pae tukutuku a te kaituhi,\n",
            "ā, ka whakauru i aua putanga mā ia anō i muri i te arotake i te waitohu.\n",
            "Kotahi noa iho te whakauru i te putanga hou, ā, ka tae mai ngā whakahou\n",
            "o muri mā rātou anō. Kua whakatikaina hoki e tēnei putanga te tīmatanga\n",
            "i runga i a Windows kāore he mōkī Visual C++: kāore ngā putanga\n",
            "0.90.0–0.103.0 i tīmata ki reira.\n",
        ),
        "Kua tae mai te putanga 0.97.0",
        "Kei te whārangi tikiake ngā taipitopito.\n",
    ),
];

/// The published feed in the locale in force, **at full capacity** — the update and
/// [`letters::NEWS_KEPT`] news items: the one item published stands for all three, each under an
/// id of its own (решение 166.2).
struct PublishedFeed {
    update: letters::FeedItem,
    news: Vec<letters::FeedItem>,
}

impl PublishedFeed {
    fn now() -> Self {
        let language = settings::ui_language();
        let (_, title, text, update_title, update_text) = PUBLISHED_FEED
            .iter()
            .find(|row| row.0 == language)
            .expect("every language of the program must be in the feed");

        let news = (2_u64..)
            .take(letters::NEWS_KEPT)
            .map(|id| letters::FeedItem {
                id,
                date: letters::Date::from_ymd(2026, 9, 25),
                title: (*title).to_owned(),
                text: (*text).to_owned(),
                link: "https://panda-pishet-kod.dev/langswitcher/".to_owned(),
                version: None,
            })
            .collect();

        Self {
            update: letters::FeedItem {
                id: 5,
                date: letters::Date::from_ymd(2026, 10, 8),
                title: (*update_title).to_owned(),
                text: (*update_text).to_owned(),
                link: "https://panda-pishet-kod.dev/langswitcher/download/".to_owned(),
                version: Some("0.97.0".to_owned()),
            },
            news,
        }
    }

    fn view(&self) -> letters::FeedView<'_> {
        letters::FeedView {
            update: Some(&self.update),
            news: &self.news,
        }
    }
}

/// The `[letters]` of a machine whose feed is live and was read on `read` — FR-102: a letter out of
/// it shown, the program on the machine for ninety days when `switch` or since `read`. Nothing
/// marked read. ⭐ Since task T-119-10 the switch of «От автора» is always there (the windows of
/// updates), and `switch` asks for **the line under it**: the feed turned off in the file.
fn fed_letters(switch: bool, read: letters::Date) -> settings::Letters {
    settings::Letters {
        feed: !switch,
        first_run: Some(if switch {
            read.plus_days(-letters::SWITCH_AFTER_DAYS)
        } else {
            read
        }),
        first_feed_letter: Some(read),
        feed_last_read: Some(read),
        ..settings::Letters::default()
    }
}

/// «От автора» of a machine whose feed is live, for a build of `version`, on the 28th of `month`
/// — the day it read the feed: a day of two digits, and the next reading a day away (решение 189.2).
fn fed_view(switch: bool, month: u32, version: &str, feed: &PublishedFeed) -> letters::AuthorView {
    let day = letters::Date::from_ymd(2026, month, 28).expect("a day");

    letters::author_view(&fed_letters(switch, day), day, version, feed.view())
}

/// «Последние письма» of a machine whose feed is live, **at full capacity** — the update (the build
/// is behind it) and the three news items, none of them read, each with its «Прочитано»
/// (решение 166.2).
fn full_list(feed: &PublishedFeed) -> Vec<letters::Entry> {
    // Task T-119-9: the row of the update stands from the seventh day the feed has named the newer
    // version — the update was noticed a week before the day the list is opened on.
    let noticed = letters::Date::from_ymd(2026, 9, 1);
    let state = settings::Letters {
        update_noticed: noticed,
        ..settings::Letters::default()
    };
    let today = noticed.map(|day| day.plus_days(letters::UPDATE_FALLBACK_DAYS));
    let entries = letters::list_entries(&state, today, BEHIND_THE_FEED, feed.view());

    assert_eq!(
        entries.len(),
        letters::NEWS_KEPT + 1,
        "the update and every news item kept"
    );
    assert!(
        entries
            .iter()
            .filter(|entry| entry.news.is_some())
            .all(|entry| entry.unread),
        "every news item unread"
    );

    entries
}

/// The dialog's own face at `face` pixels, as the dialog manager makes it of `FONT 10, "Segoe
/// UI", 400, 0, 0x1` — weight 400, `DEFAULT_CHARSET`, the family named.
fn dialog_face_at(face: i32) -> LOGFONTW {
    let mut base = LOGFONTW {
        lfHeight: -face,
        lfWeight: 400,
        lfCharSet: FONT_CHARSET(1),
        ..Default::default()
    };

    for (at, ch) in "Segoe UI".encode_utf16().enumerate() {
        base.lfFaceName[at] = ch;
    }

    base
}

/// **The model of `letters::layout_author`** — the client height of «От автора» at a dialog face
/// of `face` pixels whose drawing goes at `drawing_dpi`, on a memory DC (задача T-83-2, the road
/// the prибор of Э81 took).
///
/// Every step is the window's own, in the window's order and with the window's integer
/// arithmetic: the base units the documented way, a dialog unit to pixels by **truncation** (the
/// `Metrics` of `letters`, `dlu × base / 4|8`), the template's head by `MulDiv` (the dialog manager,
/// measured), `theme::measure_label` for every paragraph in the face the window measures it in,
/// and the wrapping row of buttons of `place_panel_buttons`. The head's bottom is the lowest of its
/// five controls; the logo is `theme::scaled(56)` at the drawing DPI. «Новости и обновления» in
/// both states of FR-102 — the line about the feed, or the switch (`y(11)`) and its sentence beside
/// the glyph — with «Последние письма» in the row when the view has it (задача T-98-2).
///
/// ⚠ A model is a second opinion until it is held against the window: the live «От автора» of the
/// measuring door is its known answer, locale by locale.
fn author_client_by_model(face: i32, drawing_dpi: i32, words: &AuthorWords) -> i32 {
    /// The head of the template, `(y, cy)` in dialog units: name, version, the two lines.
    const HEAD: [(i32, i32); 4] = [(11, 12), (24, 8), (37, 10), (47, 10)];
    /// The icon's `y` in dialog units; its height is the logo's.
    const ICON_Y: i32 = 13;

    let model = WindowModel::at(face, drawing_dpi);
    let (x, y) = (|units| model.x(units), |units| model.y(units));
    let inner_width = model.inner_width();
    let tight = y(3);
    let gap = y(8);
    let inset = x(7);
    let button = y(14);

    let logo = theme::scaled(56, drawing_dpi);
    let head = HEAD
        .iter()
        .map(|(top, height)| mul_div(*top, model.unit_y, 8) + mul_div(*height, model.unit_y, 8))
        .chain(std::iter::once(mul_div(ICON_Y, model.unit_y, 8) + logo))
        .max()
        .expect("a head");

    let view = &words.view;
    let body = |width: i32, caption: &str| model.measure(model.body, width, caption, model.pitch);
    // A line counted with every sentence it may say: the tallest of them stands.
    let tallest = |captions: &[String]| -> i32 {
        captions
            .iter()
            .map(|caption| body(inner_width, caption))
            .max()
            .unwrap_or(0)
    };

    // «Автор»
    let mut top = head + gap;
    let mut at = top + y(20);
    at += body(inner_width, &view.author_text) + tight;
    at = model.buttons(at, &[&words.support, &words.channel]);
    top = at + inset + gap;

    // «Новости и обновления» — FR-102: the line about the feed, or the switch and its sentence
    // indented by `ROW_TEXT` (задача T-98-2).
    at = top + y(20);
    match view.switch {
        None => at += body(inner_width, &view.feed_about) + tight,
        Some(_) => {
            at += y(11);
            at += body(inner_width - x(11), &view.switch_note) + tight;
        }
    }
    at += tallest(&words.feed_states) + tight;
    at += tallest(&words.version_states) + tight;
    at = if view.letters {
        model.buttons(at, &[&words.download, &words.letters])
    } else {
        model.buttons(at, &[&words.download])
    };
    if !view.file_only.is_empty() {
        at += model.measure(model.text, inner_width, &view.file_only, model.label_pitch) + tight;
    }
    top = at + inset + gap;

    // «Обратная связь»
    at = top + y(20);
    at += body(inner_width, &view.feedback_text) + tight;
    at = model.buttons(at, &[&words.write]);
    top = at + inset + gap;

    top + button + y(12)
}

/// **The model of `letters::layout_list`** — the client height of «Последние письма» at a dialog
/// face of `face` pixels with `entries` in it (задача T-98-2). The window grows by what it holds
/// since задача T-81-4, so its template's 296 units stand for nothing but the window before
/// `WM_INITDIALOG`.
///
/// The window's order: the panel from the top with the inset of the block, every entry as its
/// title (the face of numbers), its line of date and state (the face of text) and its first line
/// (the body), each `TIGHT` apart, its row of buttons — «Открыть» and, while unread, «Прочитано» —
/// and `GAP` after it; the panel as tall as that; then the quiet line under the panel at the full
/// width, the closing button and the window's own pad.
///
/// ⚠ A model is a second opinion until it is held against the window: the live «Последние письма»
/// of the measuring door is its known answer, locale by locale.
fn list_client_by_model(face: i32, drawing_dpi: i32, entries: &[letters::Entry]) -> i32 {
    let model = WindowModel::at(face, drawing_dpi);
    let (x, y) = (|units| model.x(units), |units| model.y(units));
    let inner_width = model.inner_width();
    let full_width = inner_width + x(7) * 2;
    let tight = y(3);
    let gap = y(8);
    let inset = x(7);

    let open = settings::text(settings::IDS_LETTERS_OPEN);
    let read = settings::text(settings::IDS_NEWS_READ_BUTTON);
    let foot = settings::text(settings::IDS_LETTERS_FOOT);

    let top = y(12);
    let mut at = top + inset;

    for entry in entries {
        at += model.measure(model.number, inner_width, &entry.title, model.label_pitch) + tight;
        at += model.measure(model.text, inner_width, &entry.meta, model.label_pitch) + tight;
        at += model.measure(model.body, inner_width, &entry.excerpt, model.pitch) + tight;
        at = if entry.unread {
            model.buttons(at, &[&open, &read])
        } else {
            model.buttons(at, &[&open])
        };
        at += gap;
    }

    let panel_bottom = if entries.is_empty() {
        at + inset
    } else {
        at - gap + inset
    };

    let mut below = panel_bottom + gap;
    below += model.measure(model.text, full_width, &foot, model.label_pitch) + tight;

    below + y(14) + y(12)
}

/// The memory DC, the base units and the faces the models of the windows of `letters` measure with
/// at a dialog face of `face` pixels drawn at `drawing_dpi` — what `Metrics::of` and `Faces::new`
/// give the live window, made the same way (задачи T-83-2 и T-98-2). Everything made is deleted on
/// drop.
///
/// ⚠ A label the window measures without a pitch of its own is measured here **with that pitch
/// named** ([`WindowModel::label_pitch`]): `theme::measure_label` without one takes
/// `LABEL_LINE_PITCH` at the drawing DPI of its DC, and a memory DC has no window — it would answer
/// the screen's 96 where the window of eight points draws at 76. Found by the quiet line of
/// «Последние письма», which wraps (задача T-98-1: 29 px against the window's 26 at 11 px) — the
/// lesson of 143б again, on the model's side.
struct WindowModel {
    dc: HDC,
    /// The base units the documented way: the average width of the alphabet, and `tmHeight`.
    unit_x: i32,
    unit_y: i32,
    body: HFONT,
    text: HFONT,
    /// The face of the titles of «Последние письма».
    number: HFONT,
    /// The pitch of the body — `Faces::body_pitch`.
    pitch: Option<i32>,
    /// The pitch a label measured without one of its own is given — `LABEL_LINE_PITCH` at the
    /// drawing DPI.
    label_pitch: Option<i32>,
}

impl WindowModel {
    fn at(face: i32, drawing_dpi: i32) -> Self {
        let base = dialog_face_at(face);
        let body_face = settings::about_body_logfont(base);

        // SAFETY: a memory DC of this process; the dialog's face is selected out and deleted
        // before the call returns, the three others are owned by the value and deleted on drop.
        unsafe {
            let dc = CreateCompatibleDC(None);

            let dialog = CreateFontIndirectW(&base);
            let previous = SelectObject(dc, dialog.into());
            let mut metrics = TEXTMETRICW::default();
            let _ = GetTextMetricsW(dc, &mut metrics);
            let alphabet: Vec<u16> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
                .encode_utf16()
                .collect();
            let mut size = Default::default();
            let _ = GetTextExtentPoint32W(dc, &alphabet, &mut size);
            SelectObject(dc, previous);
            let _ = DeleteObject(dialog.into());

            Self {
                dc,
                unit_x: (size.cx / 26 + 1) / 2,
                unit_y: metrics.tmHeight,
                body: CreateFontIndirectW(&body_face),
                text: CreateFontIndirectW(&theme::smoothed_logfont(base)),
                number: CreateFontIndirectW(&settings::about_number_logfont(
                    base,
                    settings::Emphasis::Semibold,
                )),
                pitch: Some(settings::about_body_line_pitch(body_face.lfHeight)),
                label_pitch: Some(theme::scaled(theme::LABEL_LINE_PITCH, drawing_dpi)),
            }
        }
    }

    /// Horizontal dialog units in pixels — the `Metrics` of `letters`, by **truncation**.
    fn x(&self, units: i32) -> i32 {
        units * self.unit_x / 4
    }

    /// Vertical dialog units in pixels, the same way.
    fn y(&self, units: i32) -> i32 {
        units * self.unit_y / 8
    }

    /// The block inside a panel: the client of 256 units the manager's way (`MulDiv`), less the
    /// pad of the window and the inset of the panel on both sides.
    fn inner_width(&self) -> i32 {
        mul_div(256, self.unit_x, 4) - self.x(12) * 2 - self.x(7) * 2
    }

    /// `measure` of `letters` — `theme::measure_label` in `face`, nothing for no words.
    fn measure(&self, face: HFONT, width: i32, caption: &str, pitch: Option<i32>) -> i32 {
        if caption.is_empty() {
            return 0;
        }

        let mut wide: Vec<u16> = caption.encode_utf16().collect();

        // SAFETY: the DC and the face are live while `self` is; the previous face is put back.
        unsafe {
            let previous = SelectObject(self.dc, face.into());
            let height = theme::measure_label(self.dc, width, &mut wide, pitch);
            SelectObject(self.dc, previous);
            height
        }
    }

    /// `button_width` of `letters` — the caption on one line in the face of text, the pad on both
    /// sides, never narrower than forty units.
    fn button_width(&self, caption: &str) -> i32 {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::Graphics::Gdi::{DT_CALCRECT, DT_SINGLELINE, DrawTextW};

        let mut wide: Vec<u16> = caption.encode_utf16().collect();
        let mut rect = RECT::default();

        // SAFETY: as in `measure`; `DT_CALCRECT` paints nothing.
        unsafe {
            let previous = SelectObject(self.dc, self.text.into());
            let _ = DrawTextW(self.dc, &mut wide, &mut rect, DT_CALCRECT | DT_SINGLELINE);
            SelectObject(self.dc, previous);
        }

        (rect.right - rect.left + self.x(10) * 2).max(self.x(40))
    }

    /// `place_panel_buttons`: left to right, a new line when the next would run off the block;
    /// answers where the panel's content now ends.
    fn buttons(&self, top: i32, captions: &[&str]) -> i32 {
        let height = self.y(14);
        let between = self.x(4);
        let inner_width = self.inner_width();
        let mut left = 0;
        let mut row = top;

        for caption in captions {
            let width = self.button_width(caption);
            if left > 0 && left + width > inner_width {
                left = 0;
                row += height + self.y(3);
            }
            left += width + between;
        }

        if captions.is_empty() {
            top
        } else {
            row + height + self.y(3)
        }
    }
}

impl Drop for WindowModel {
    fn drop(&mut self) {
        // SAFETY: each was made by `at`, is selected into nothing and is deleted once here.
        unsafe {
            for face in [self.body, self.text, self.number] {
                let _ = DeleteObject(face.into());
            }
            let _ = DeleteDC(self.dc);
        }
    }
}

/// The dialog units a client of `pixels` needs at a face whose cell is `cell` pixels tall: the
/// least `u` for which the dialog manager's `MulDiv(u, cell, 8)` reaches it.
fn units_of_client(pixels: i32, cell: i32) -> i32 {
    let mut units = pixels * 8 / cell;

    while mul_div(units, cell, 8) < pixels {
        units += 1;
    }
    while units > 0 && mul_div(units - 1, cell, 8) >= pixels {
        units -= 1;
    }

    units
}

/// **Every face, with the DPI of its drawing, the rule of вопрос 143 may open the windows at** on
/// the scales Windows offers — 100, 125, 150, 175, 200, 225, 250 and 300 % × the steps 10…6 pt, the
/// face not under the floor (задача T-83-4, развилка C3). Sorted by face; a face two scales make
/// with two drawing DPIs is there twice.
///
/// The fitting stands of П9 measure at every one of these, not at the 13 px of 100 % alone: at 15
/// and 17 px a text is wider against the dialog unit than at 13 (13 / 7 = 1,857; 15 / 8 = 1,875;
/// 17 / 9 = 1,889), and a caption that fits at 100 % may not fit there.
fn rule_configurations() -> Vec<(i32, i32)> {
    const DPIS: [i32; 8] = [96, 120, 144, 168, 192, 216, 240, 288];

    let mut all = Vec::new();

    for dpi in DPIS {
        for step in settings::FONT_STEPS {
            let face = theme::face_at(step, dpi);

            if face >= settings::FONT_STEP_FLOOR {
                all.push((face, theme::drawing_dpi_at(dpi, step)));
            }
        }
    }

    all.sort_unstable();
    all.dedup();
    all
}

/// The faces of [`rule_configurations`], each once — for the stands whose measure does not depend
/// on the DPI of the drawing (a caption against a slot of dialog units).
fn rule_faces() -> Vec<i32> {
    let mut faces: Vec<i32> = rule_configurations()
        .into_iter()
        .map(|(face, _)| face)
        .collect();

    faces.dedup();
    faces
}

/// The dialog's own face at `face` pixels — the manager's `LOGFONTW` of `font` with the height the
/// step makes.
fn manager_face_at(sheet: &Sheet, font: &TemplateFont, face: i32) -> LOGFONTW {
    LOGFONTW {
        lfHeight: -face,
        ..manager_logfont(sheet.dc, font, CLEARTYPE_QUALITY)
    }
}

/// The cell of Segoe UI at `face` pixels — `tmHeight`, the vertical base unit, asked of GDI.
fn cell_of(face: i32) -> i32 {
    let base = dialog_face_at(face);

    // SAFETY: the font is selected out and deleted before the DC.
    unsafe {
        let dc = CreateCompatibleDC(None);
        let font = CreateFontIndirectW(&base);
        let previous = SelectObject(dc, font.into());
        let mut metrics = TEXTMETRICW::default();
        let _ = GetTextMetricsW(dc, &mut metrics);
        SelectObject(dc, previous);
        let _ = DeleteObject(font.into());
        let _ = DeleteDC(dc);
        metrics.tmHeight
    }
}

/// **The model of «От автора» IS the live window — задача T-83-2, the known answer.**
///
/// The model that counts the tallest window at every face has to be held against the window it
/// stands for before anything is believed of it (the lesson of Э81: the one instrument worth
/// trusting at once was the one held against a known answer). The known answer is the live
/// «От автора» itself — built hidden through the measuring door, laid out by the product's own
/// `WM_INITDIALOG` — in all fourteen languages at the template's face.
///
/// ⚠ The numbers of П6 in the ТЗ (789 / 827 / 846) are those of Э81's model **before** T-81-6 made
/// the head the template's; on `e82` the window is two pixels taller: 791 / 829 (measured here).
///
/// ⭐ Since задача T-98-2 the window is held in **every state FR-102 gives it** (Э83-Б-1, решение
/// 166.2): the first day, as the product opens it; and a machine whose feed is live — the quiet
/// line about the file with «доступна» in the line of the version, and the switch with its sentence
/// with «это последняя» — «Последние письма» in the row both times.
#[cfg(feature = "testing")]
#[test]
fn the_model_of_the_author_window_is_the_live_window() {
    let _guard = with_product_strings();
    let mut differences = Vec::new();

    for language in Language::shipped() {
        settings::set_ui_language(language);

        // ⚠ Stage E108, решение 202.40: the step of a window goes by its language now, and on the
        // screen of this machine Tamil and Malayalam open below ten points — the model counts at
        // the template's face, so the live window is held at ten points too.
        let words = AuthorWords::now("0.83.0");
        let live =
            settings::testing::with_font_step(10, || HiddenWindow::open("0.83.0").client_height());
        let model = author_client_by_model(13, 96, &words);

        println!("{language:?}: live {live} px, model {model} px");

        if live != model {
            differences.push(format!("{language:?}: live {live}, model {model}"));
        }

        let feed = PublishedFeed::now();

        for (switch, version) in [(false, BEHIND_THE_FEED), (true, PAST_THE_FEED)] {
            let view = fed_view(switch, 9, version, &feed);
            let model = author_client_by_model(13, 96, &AuthorWords::of(view.clone()));
            let live = settings::testing::with_font_step(10, || {
                HiddenWindow::author(view).client_height()
            });

            println!("{language:?}, feed live, switch {switch}: live {live} px, model {model} px");

            if live != model {
                differences.push(format!(
                    "{language:?}, switch {switch}: live {live}, model {model}"
                ));
            }
        }
    }

    settings::set_ui_language(Language::Ru);

    assert!(
        differences.is_empty(),
        "the model of layout_author is not the window: {differences:?}"
    );
}

/// **The model of «От автора» IS the live window at every face a test can reach — задача T-83-3.**
///
/// The debug substitution of the step opens the window at 8…15 pt, and at 96 DPI those are the
/// faces 11, 12, 13, 15, 16, 17, 19 and 20 px — every face the rule may answer with except 14 and
/// 18, which a 96-DPI screen cannot make and the model alone gives. Fourteen languages × eight
/// faces: the model must be the window at all of them, and the window must have been given the face
/// the step asks for (C7 through the product's own road — `put_font_step` in `show_modeless_dialog`).
///
/// ⚠ «Красное до»: without the substitution every one of these windows opens at 13 px.
///
/// ⭐ Since задача T-98-2 in the three states of [`the_model_of_the_author_window_is_the_live_window`]
/// — the first day and both states of FR-102 with the feed live (Э83-Б-1).
#[cfg(feature = "testing")]
#[test]
fn the_model_of_the_author_window_is_the_live_window_at_every_reachable_face() {
    let _guard = with_product_strings();
    let mut differences = Vec::new();
    let mut measured = 0;

    for language in Language::shipped() {
        settings::set_ui_language(language);

        let feed = PublishedFeed::now();
        let fed = [(false, BEHIND_THE_FEED), (true, PAST_THE_FEED)]
            .map(|(switch, version)| fed_view(switch, 9, version, &feed));
        // `(what, the view the window is opened with — `None` for the product's own road)`.
        let states = [
            ("first day", None),
            ("quiet line", Some(fed[0].clone())),
            ("switch", Some(fed[1].clone())),
        ];

        for (what, view) in states {
            let words = view
                .clone()
                .map_or_else(|| AuthorWords::now("0.83.0"), AuthorWords::of);

            for step in 8..=15_u16 {
                let face = theme::face_at(step, 96);
                let (live, font) = settings::testing::with_font_step(step, || {
                    let window = view
                        .clone()
                        .map_or_else(|| HiddenWindow::open("0.83.0"), HiddenWindow::author);
                    (window.client_height(), window.dialog_font_height())
                });
                let model = author_client_by_model(face, theme::drawing_dpi_at(96, step), &words);
                measured += 1;

                if font != -face {
                    differences.push(format!(
                        "{language:?} {what} {step} pt: the window got lfHeight {font}, not \
                         -{face} (C7)"
                    ));
                } else if live != model {
                    differences.push(format!(
                        "{language:?} {what} {step} pt ({face} px): live {live}, model {model}"
                    ));
                }
            }
        }
    }

    settings::set_ui_language(Language::Ru);

    println!("{measured} windows measured, differences: {differences:?}");

    assert_eq!(
        measured,
        Language::shipped().count() * 3 * 8,
        "every language the build offers, three states, eight faces"
    );
    assert!(
        differences.is_empty(),
        "the model of layout_author is not the window at every face: {differences:?}"
    );
}

/// The faces the rule may answer with, at the least — every face from the floor up to the 40 px
/// of ten points at 300 % (задача T-83-2; the ТЗ names 11…20 «как минимум»). Custom scaling makes
/// any DPI, so every whole face in between is one some screen can ask for.
const RULE_FACES: std::ops::RangeInclusive<i32> = 11..=40;

/// **The tallest window of the program, face by face, is the table the rule counts with —
/// задача T-83-2, решения 143.1 и 143.11; with the feed live, задача T-98-2 (Э83-Б-1, решение
/// 166.2).**
///
/// For every face the rule may answer with: the five windows of the old kind × fourteen
/// languages, each in dialog units, **in every state the window is really in** («О программе» is a
/// window of the shell since FR-92а moved it there, and the shell fits its own) —
/// * «От автора» by the model held against the live window above: the first day (no feed), and a
///   machine whose feed is live — the quiet line about the file or the switch of FR-102 with its
///   sentence, «Последние письма» in the row, the line of the reading and the line of the version
///   each at the tallest sentence the program can put there ([`AuthorWords::fed`]);
/// * «Последние письма» by the model held against the live window, at full capacity — the update
///   and three unread news items of the published feed ([`full_list`]);
/// * the three windows whose height the template decides: настройки 380; «Письмо» and the wizard
///   276, which their layouts never resize.
///
/// The statement is **equality** at every face: the table is the least number of units that holds
/// the tallest window there — enough (`MulDiv(table, cell, 8)` reaches the window) and no more
/// (one unit less does not). Beyond the table, the table's own maximum.
///
/// ⚠ Отрицательный контроль: the table one unit short at any face is caught — the tallest window at
/// that face does not fit it.
#[test]
fn the_tallest_window_by_face_is_the_table_the_rule_counts_with() {
    /// The templates whose height the template decides, in dialog units.
    const FIXED: [(&str, i32); 3] = [("настройки", 380), ("«Письмо»", 276), ("мастер", 276)];

    let _guard = with_product_strings();

    // The words of every language, made once: `(language, «От автора» in its three states, the
    // full «Последние письма»)`.
    let words: Vec<_> = Language::shipped()
        .map(|language| {
            settings::set_ui_language(language);
            let feed = PublishedFeed::now();

            (
                language,
                [
                    ("first day", AuthorWords::now("0.83.0")),
                    ("quiet line", AuthorWords::fed(false, &feed)),
                    ("switch", AuthorWords::fed(true, &feed)),
                ],
                full_list(&feed),
            )
        })
        .collect();

    // ⭐ Stage E108, решение 202.40: the tallest window **of each language**, face by face — the
    // table of the fourteen of 1.0.0 is their tallest, and a language taller than it somewhere has a
    // table of its own (`settings::TALLER_LANGUAGES`).
    let fourteen: Vec<Language> = registry::registry()
        .iter()
        .filter(|entry| entry.group == "shipped")
        .map(|entry| language_of_tag(&entry.tag))
        .collect();
    let mut measured = Vec::new();
    let mut by_language: Vec<(Language, Vec<i32>)> = words
        .iter()
        .map(|(language, ..)| (*language, Vec::new()))
        .collect();

    for face in RULE_FACES {
        let cell = cell_of(face);
        // The DPI our drawing goes at for a face — `dpi × step / 10` against `dpi × step / 72`:
        // whatever the pair, it is 7,2 faces.
        let drawing = mul_div(face, 72, 10);
        let mut tallest = (0, String::new());
        let fixed = FIXED.iter().map(|(_, units)| *units).max().unwrap_or(0);

        for (window, units) in FIXED {
            if units > tallest.0 {
                tallest = (units, window.to_owned());
            }
        }

        for ((language, states, list), (_, own)) in words.iter().zip(by_language.iter_mut()) {
            settings::set_ui_language(*language);
            let mut own_tallest = fixed;
            let mut note = |units: i32, what: String, tallest: &mut (i32, String)| {
                own_tallest = own_tallest.max(units);
                if fourteen.contains(language) && units > tallest.0 {
                    *tallest = (units, what);
                }
            };

            for (state, words) in states {
                let author = units_of_client(author_client_by_model(face, drawing, words), cell);
                note(
                    author,
                    format!("«От автора» {language:?}, {state}"),
                    &mut tallest,
                );
            }

            let list = units_of_client(list_client_by_model(face, drawing, list), cell);
            note(
                list,
                format!("«Последние письма» {language:?}"),
                &mut tallest,
            );

            own.push(own_tallest);
        }

        println!(
            "face {face} px (cell {cell}): tallest of the fourteen {} units — {}; the table says {}",
            tallest.0,
            tallest.1,
            settings::tallest_window_units_in(Language::Ru, face)
        );

        measured.push((face, tallest.0, cell));
    }

    settings::set_ui_language(Language::Ru);

    // The languages of group A: a language with its own table must have exactly its measurement,
    // one without must fit the table of the fourteen at every face.
    let mut group_a_wrong = Vec::new();
    for (language, own) in &by_language {
        let faces = RULE_FACES.zip(own.iter().copied());

        if fourteen.contains(language) {
            assert!(
                !settings::has_own_tallest_table(*language),
                "{language:?}: a language of 1.0.0 counts with the table of the fourteen"
            );
        } else if settings::has_own_tallest_table(*language) {
            for (face, units) in faces {
                let table = settings::tallest_window_units_in(*language, face);
                // Task T-119-10, until T-120-24 — delete with the tables: the table holds the window with no
                // more than the slack of 64 units to spare (see `SLACK` below).
                if table < units || table > units + 64 {
                    group_a_wrong.push(format!(
                        "{language:?} face {face}: measured {units}, its table {table}"
                    ));
                }
            }
        } else if faces
            .clone()
            .any(|(face, units)| units > settings::tallest_window_units_in(Language::Ru, face))
        {
            group_a_wrong.push(format!(
                "{language:?} is taller than the table of the fourteen and needs its own: {own:?}"
            ));
        }
    }
    assert!(
        group_a_wrong.is_empty(),
        "⛔⛔ the tables of the languages of group A — перемерить:\n{}",
        group_a_wrong.join("\n")
    );

    // Where a table and the measurement part: a face at which the table does not hold the tallest
    // window, or holds it with more than `SLACK` units to spare.
    //
    // ⭐ **Task T-119-10, решение контролёра (2026-11-20, вариант б): until T-120-24 — delete with the
    // table.** The switch of the feed and its long line left «От автора», and the tallest window is
    // now «настройки» (380 units at every face) while the table keeps the 407 / 389 / 399 / 390 of
    // the old «От автора». The old windows go onto the shell of E120 in 1.8.0, and the table of the
    // rule and this test go with them (T-120-24), so the table is not moved for a few hours of life
    // — it may stand above the tallest window, by no more than `SLACK`.
    const SLACK: i32 = 64;
    let disagreements = |table: &dyn Fn(i32) -> i32| -> Vec<String> {
        measured
            .iter()
            .filter(|(face, units, _)| table(*face) < *units || table(*face) > *units + SLACK)
            .map(|(face, units, _)| {
                format!("face {face}: measured {units}, table {}", table(*face))
            })
            .collect()
    };

    let wrong = disagreements(&|face: i32| settings::tallest_window_units_in(Language::Ru, face));

    assert!(
        wrong.is_empty(),
        "⛔⛔ the table of the tallest window is not what the windows need — перемерить: {wrong:?}"
    );

    let most = measured
        .iter()
        .map(|(_, units, _)| *units)
        .max()
        .expect("faces");

    let beyond = settings::tallest_window_units_in(Language::Ru, RULE_FACES.end() + 1);
    assert!(
        beyond >= most && beyond <= most + SLACK,
        "beyond the table the rule must count with a maximum that holds the tallest window ({most}) \
         with no more than {SLACK} to spare: {beyond}"
    );

    // ⚠ Отрицательный контроль: a table one unit short of the window — at every face at once, and
    // at the one face the 150 % of a 1080p screen turns on — and a table past the window by more
    // than the slack must be caught, and one unit short must really be a smaller window there.
    let units_at = |face: i32| {
        measured
            .iter()
            .find(|(at, _, _)| *at == face)
            .map_or(0, |(_, units, _)| *units)
    };
    let short = |face: i32| units_at(face) - 1;
    let short_at_14 = |face: i32| {
        if face == 14 {
            units_at(face) - 1
        } else {
            settings::tallest_window_units_in(Language::Ru, face)
        }
    };
    let past = |face: i32| units_at(face) + SLACK + 1;

    assert_eq!(
        disagreements(&past).len(),
        measured.len(),
        "the control past the slack must be caught at every face"
    );

    assert_eq!(
        disagreements(&short).len(),
        measured.len(),
        "the control must be caught at every face"
    );
    let caught = disagreements(&short_at_14);
    assert!(
        caught.len() == 1 && caught[0].starts_with("face 14:"),
        "the control must be caught at the one face it moved, and nowhere else: {caught:?}"
    );
    for (face, units, cell) in &measured {
        assert!(
            mul_div(units - 1, *cell, 8) < mul_div(*units, *cell, 8),
            "face {face}: one unit less must be a smaller window"
        );
    }
}

/// **The rule answers, on the six screens of the decision, what the owner took on the mock-up —
/// развилка C4 of задача T-83-2, решения 143.1, 143.3 и 143.5.**
///
/// The screens: 1920 × 1080 at 100, 125 and 150 %, 1366 × 768 at 100 %, 2560 × 1440 at 125 % and
/// 3840 × 2160 at 150 %; their work areas — the screen less a taskbar of 48 points — 1032, 1020,
/// 1008, 720, 1380 and 2088 px; the frame of these windows at the DPI, measured by the живое окно
/// and `AdjustWindowRectExForDpi` — 39, 47 and 56 px; the cell of every face asked of GDI itself.
///
/// ⚠ The threshold and the floor are **the numbers of the decision, written here apart from the
/// code** (the lesson of Э53а: a guard that takes its expectation from the constant it checks cannot
/// fail) — the mutant «порог 12 → 0» turns the row 125 % to nine points.
#[test]
fn the_rule_answers_what_the_owner_took_on_the_six_screens() {
    /// Решение 143.3.
    const AIR: i32 = 12;
    /// Решение 143.5.
    const FLOOR: i32 = 11;
    /// The six screens: `(what, work area, dpi, frame, the step the owner took)`.
    const SCREENS: [(&str, i32, i32, i32, u16); 6] = [
        ("1920 × 1080, 100 %", 1032, 96, 39, 10),
        ("1920 × 1080, 125 %", 1020, 120, 47, 8),
        ("1920 × 1080, 150 %", 1008, 144, 56, 7),
        ("1366 × 768, 100 %", 720, 96, 39, 8),
        ("2560 × 1440, 125 %", 1380, 120, 47, 10),
        ("3840 × 2160, 150 %", 2088, 144, 56, 10),
    ];

    assert_eq!(
        (settings::FONT_STEP_AIR, settings::FONT_STEP_FLOOR),
        (AIR, FLOOR),
        "the threshold and the floor are the owner's numbers"
    );

    let mut wrong = Vec::new();

    for (screen, work, dpi, frame, owner) in SCREENS {
        let step = settings::font_step(
            work,
            dpi,
            frame,
            |face| settings::tallest_window_units_in(Language::Ru, face),
            cell_of,
        );
        let face = theme::face_at(step, dpi);
        let window = mul_div(
            settings::tallest_window_units_in(Language::Ru, face),
            cell_of(face),
            8,
        ) + frame;

        println!(
            "{screen}: {step} pt, face {face} px, the tallest window {window} px in {work} — {} px \
             of air above and below",
            (work - window) / 2
        );

        if step != owner {
            wrong.push(format!(
                "{screen}: {step} pt against the owner's {owner} pt"
            ));
        }
    }

    assert!(
        wrong.is_empty(),
        "⛔⛔ C4: the rule does not answer what the owner took — {wrong:?}"
    );
}

/// **The rule keeps its floor and its ceiling — задача T-83-2**: never larger than ten points,
/// never a face under the floor while a step above it is allowed, the floor when nothing fits, and
/// ten on a monitor where even ten points come out under the floor.
#[test]
fn the_rule_keeps_its_floor_and_its_ceiling() {
    // A work area nothing fits in: the floor — the smallest step whose face is not under 11 px.
    assert_eq!(
        settings::font_step(
            100,
            96,
            39,
            |face| settings::tallest_window_units_in(Language::Ru, face),
            cell_of
        ),
        8
    );
    assert_eq!(
        settings::font_step(
            100,
            120,
            47,
            |face| settings::tallest_window_units_in(Language::Ru, face),
            cell_of
        ),
        7
    );
    assert_eq!(
        settings::font_step(
            100,
            144,
            56,
            |face| settings::tallest_window_units_in(Language::Ru, face),
            cell_of
        ),
        6
    );

    // A work area everything fits in: ten, and never more.
    assert_eq!(
        settings::font_step(
            100_000,
            144,
            56,
            |face| settings::tallest_window_units_in(Language::Ru, face),
            cell_of
        ),
        10
    );

    // A monitor on which ten points come out under the floor — none that Windows makes: ten.
    assert_eq!(
        settings::font_step(
            100,
            72,
            39,
            |face| settings::tallest_window_units_in(Language::Ru, face),
            cell_of
        ),
        10
    );

    // The face of a step is the dialog manager's `MulDiv`, rounded half up.
    assert_eq!(
        [96, 120, 144].map(|dpi| settings::FONT_STEPS.map(|step| theme::face_at(step, dpi))),
        [
            [13, 12, 11, 9, 8],
            [17, 15, 13, 12, 10],
            [20, 18, 16, 14, 12]
        ]
    );
}

/// The dialog templates of the program — `IDD_LETTER` … `IDD_WIZARD` — with the offset of
/// their `pointsize`, measured by the probe of the stage (`scratchpad-E83\probe-premises-e83.log`).
/// Six then; `IDD_ABOUT` (201) went when «О программе» moved to the shell (FR-92а), `IDD_LETTER` and
/// `IDD_LETTERS_LIST` (202, 203) when the letters did (T-120-16), `IDD_SETTINGS` (200) when the
/// settings did (T-120-20).
const TEMPLATES: [(u16, usize); 2] = [(204, 50), (205, 62)];

/// **The font block of the six templates is found where the dialog manager reads it — задача
/// T-83-3.**
///
/// `settings::template_point_size_at` walks the menu, the class and the title of a `DLGTEMPLATEEX`
/// to its font block; on the six real templates of the product image it must land on `pointsize`
/// **10** and the typeface **«Segoe UI»** (the known answers of `app.rc`), at the offsets the probe
/// measured, and agree with this file's own parser of templates. `settings::template_face` must
/// read the face the manager is asked for: 400, not italic, `DEFAULT_CHARSET`.
///
/// ⚠ Controls: the old form of the template, a buffer that ends before its size, and a template
/// without `DS_SETFONT` are refused — nothing to patch is nothing patched.
#[test]
fn the_font_block_of_the_six_templates_is_found_where_the_manager_reads_it() {
    let product = ProductImage::open();

    for (id, offset) in TEMPLATES {
        let bytes = product.resource(RT_DIALOG, id);
        let at = settings::template_point_size_at(&bytes)
            .unwrap_or_else(|| panic!("template {id}: the font block must be found"));
        let points = u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let face = settings::template_face(&bytes)
            .unwrap_or_else(|| panic!("template {id}: the face must be read"));
        let name = String::from_utf16_lossy(
            &face.lfFaceName[..face
                .lfFaceName
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(face.lfFaceName.len())],
        );

        println!(
            "template {id}: pointsize at {at} = {points}, «{name}», weight {}, italic {}, charset {}",
            face.lfWeight, face.lfItalic, face.lfCharSet.0
        );

        assert_eq!(at, offset, "template {id}: the offset the probe measured");
        assert_eq!(points, 10, "template {id}: FONT 10 of app.rc");
        assert_eq!(name, "Segoe UI", "template {id}: the typeface of app.rc");
        assert_eq!(
            (face.lfWeight, face.lfItalic, face.lfCharSet.0),
            (400, 0, 1),
            "template {id}: 400, not italic, DEFAULT_CHARSET"
        );

        let parsed = DialogTemplate::parse(&bytes)
            .font
            .expect("every template of the program declares DS_SETFONT");
        assert_eq!(
            (parsed.points, parsed.face.as_str()),
            (points, name.as_str()),
            "template {id}: this file's parser and the product's must agree"
        );

        // ⚠ Контроли.
        let mut old_form = bytes.clone();
        old_form[2] = 0;
        old_form[3] = 0;
        assert_eq!(
            settings::template_point_size_at(&old_form),
            None,
            "template {id}: the old DLGTEMPLATE must be refused"
        );
        assert_eq!(
            settings::template_point_size_at(&bytes[..at + 1]),
            None,
            "template {id}: a buffer that ends inside the size must be refused"
        );
        let mut no_font = bytes.clone();
        no_font[12] &= !0x40;
        assert_eq!(
            settings::template_point_size_at(&no_font),
            None,
            "template {id}: a template without DS_SETFONT has no size to patch"
        );
    }
}

/// **At the template's own step the copy is the resource byte for byte — задача T-83-3, the guard
/// «100 % без перемен».**
///
/// `settings::put_font_step` is what both `show_*` put the step in with. At ten points — the step
/// of every window on a screen that takes the whole of it — it must not write a single byte: the
/// copy the dialog manager is handed is then the very template it was always handed. At eight it
/// must write the `pointsize` and nothing else.
#[test]
fn at_the_template_step_the_copy_is_the_resource_byte_for_byte() {
    let product = ProductImage::open();

    for (id, _) in TEMPLATES {
        let resource = product.resource(RT_DIALOG, id);

        let mut copy = resource.clone();
        settings::put_font_step(&mut copy, 10);
        assert_eq!(
            copy, resource,
            "template {id}: at ten points not a byte may change"
        );

        let mut smaller = resource.clone();
        settings::put_font_step(&mut smaller, 8);
        let at = settings::template_point_size_at(&resource).expect("the font block");
        let changed: Vec<usize> = (0..resource.len())
            .filter(|index| smaller[*index] != resource[*index])
            .collect();

        assert_eq!(
            changed,
            vec![at],
            "template {id}: eight points change the low byte of `pointsize` and nothing else"
        );
        assert_eq!(u16::from_le_bytes([smaller[at], smaller[at + 1]]), 8);
    }
}

/// The dialog procedure of a template opened bare, for the measuring below: it lets the manager
/// choose the focus and answers nothing else.
unsafe extern "system" fn bare_dialog_proc(
    _hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    _wparam: windows::Win32::Foundation::WPARAM,
    _lparam: windows::Win32::Foundation::LPARAM,
) -> isize {
    isize::from(message == windows::Win32::UI::WindowsAndMessaging::WM_INITDIALOG)
}

/// A hidden owner shaped like `LangSwitcher.Hidden` — a popup of no size at the origin, never
/// shown. Destroyed on drop.
struct HiddenOwner(windows::Win32::Foundation::HWND);

impl HiddenOwner {
    fn new() -> Self {
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, WS_EX_TOOLWINDOW, WS_POPUP,
        };
        use windows::core::{PCWSTR, w};

        // SAFETY: a system class, no parent, no pointer of ours; destroyed in `Drop`.
        let owner = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )
        }
        .expect("a hidden owner must be creatable");

        Self(owner)
    }
}

impl Drop for HiddenOwner {
    fn drop(&mut self) {
        // SAFETY: made by `new` on this thread, destroyed once.
        let _ = unsafe { windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.0) };
    }
}

/// One template of the product image opened bare and hidden on `owner`, and what the manager made
/// of it: the `lfHeight` of its font, and its window and client rectangles. The window is destroyed
/// before the answer.
fn open_template_bare(
    owner: &HiddenOwner,
    module: windows::Win32::Foundation::HMODULE,
    template: &[u8],
) -> (
    i32,
    windows::Win32::Foundation::RECT,
    windows::Win32::Foundation::RECT,
) {
    use windows::Win32::Foundation::{HINSTANCE, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{GetObjectW, HGDIOBJ};
    use windows::Win32::UI::Controls::{
        ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateDialogIndirectParamW, DLGTEMPLATE, DestroyWindow, GetClientRect, GetWindowRect,
        SendMessageW, WM_GETFONT,
    };

    // The settings template carries a list view, whose class the product registers on demand.
    let request = INITCOMMONCONTROLSEX {
        dwSize: u32::try_from(size_of::<INITCOMMONCONTROLSEX>()).expect("small"),
        dwICC: ICC_LISTVIEW_CLASSES,
    };
    // SAFETY: the structure is a live local the call reads.
    let _ = unsafe { InitCommonControlsEx(&request) };

    // Aligned for the `u32` fields of the header, as `AlignedTemplate` holds it in the product.
    let mut words = vec![0_u32; template.len().div_ceil(4)];
    // SAFETY: the destination is at least `template.len()` bytes long.
    unsafe {
        std::ptr::copy_nonoverlapping(
            template.as_ptr(),
            words.as_mut_ptr().cast::<u8>(),
            template.len(),
        );
    }

    // SAFETY: a whole template in a live aligned buffer, a live owner of this thread, a procedure
    // that dereferences nothing; the window is destroyed below.
    let window = unsafe {
        CreateDialogIndirectParamW(
            Some(HINSTANCE(module.0)),
            words.as_ptr().cast::<DLGTEMPLATE>(),
            Some(owner.0),
            Some(bare_dialog_proc),
            LPARAM(0),
        )
    }
    .expect("the template must open");

    let mut outer = RECT::default();
    let mut client = RECT::default();
    let mut logical = LOGFONTW::default();

    // SAFETY: the window is live until the `DestroyWindow` below; the buffers are locals.
    unsafe {
        let _ = GetWindowRect(window, &mut outer);
        let _ = GetClientRect(window, &mut client);
        let font = SendMessageW(window, WM_GETFONT, None, None);
        let _ = GetObjectW(
            HGDIOBJ(font.0 as *mut _),
            i32::try_from(size_of::<LOGFONTW>()).expect("small"),
            Some(std::ptr::from_mut(&mut logical).cast()),
        );
        let _ = DestroyWindow(window);
    }

    (logical.lfHeight, outer, client)
}

/// **⛔ C7 — the dialog manager obeys the patched `pointsize`, in every one of the six templates —
/// задача T-83-3.**
///
/// Each template of the product image, its copy put at eight points by the very function the
/// windows use, opened hidden: the font the manager gives the window must be **11 px** (`lfHeight =
/// −11`, `MulDiv(8, 96, 72)`), and at the template's own ten **13 px**. A manager that ignored the
/// patch would answer −13 for both — развилка C7.
#[test]
fn every_template_put_at_eight_points_opens_at_eleven_pixels() {
    let product = ProductImage::open();
    let owner = HiddenOwner::new();

    for (id, _) in TEMPLATES {
        let resource = product.resource(RT_DIALOG, id);
        let mut eight = resource.clone();
        settings::put_font_step(&mut eight, 8);

        let (at_ten, ..) = open_template_bare(&owner, product.module, &resource);
        let (at_eight, ..) = open_template_bare(&owner, product.module, &eight);

        println!("template {id}: lfHeight {at_ten} at ten points, {at_eight} at eight");

        assert_eq!(at_ten, -13, "template {id}: ten points at 96 DPI are 13 px");
        assert_eq!(
            at_eight, -11,
            "⛔ C7, template {id}: the manager must obey the patched pointsize"
        );
    }
}

/// **`show_modeless_dialog` puts the step by the ONE body — задача T-83-3.**
///
/// The modeless call opens the four windows of the letters (the modal one, which opened
/// «Настройки» and «О программе», went with those windows to the shell): it has to call
/// `apply_font_step` on its copy exactly once, and that body has to ask the one `font_step_for`
/// and put the answer by `put_font_step`.
///
/// ⚠ Отрицательный контроль: the text of `e82` — the call without it — finds nothing.
#[test]
fn both_show_calls_put_the_step_by_the_one_body() {
    const CALL: &str = "apply_font_step(owner, copy.as_mut_bytes());";

    let source = settings_module_source();
    let code_of = |text: &str| -> String {
        text.lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let calls = |text: &str| -> usize {
        code_of(function_body(
            text,
            "pub(crate) unsafe fn show_modeless_dialog(",
        ))
        .matches(CALL)
        .count()
    };

    assert_eq!(
        calls(&source),
        1,
        "show_modeless_dialog must put the step of the font by the one body, once"
    );

    let body = code_of(function_body(&source, "fn apply_font_step("));
    assert!(
        body.contains("let step = font_step_for(owner, template);")
            && body.contains("put_font_step(template, step);"),
        "the one body must ask the one function of the step and put its answer"
    );

    // ⚠ Контроль: the shape of `e82`, where the call put no step at all.
    let of_e82 = source.replace(CALL, "");
    assert_ne!(of_e82, source, "the control must change the text");
    assert_eq!(calls(&of_e82), 0, "the control must be caught");
}

/// The code of the four files of the windows, comment lines dropped — what the sweep of задача
/// T-83-4 reads. ⛔ Prose beside code names the construction it replaced (the lesson of Э81), so a
/// comment is never counted as a call.
fn window_code() -> Vec<(&'static str, String)> {
    ["letters.rs", "settings.rs", "theme.rs", "widgets.rs"]
        .into_iter()
        .map(|file| {
            let text =
                fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(file))
                    .unwrap_or_else(|_| panic!("src\\{file} must be readable"))
                    .replace("\r\n", "\n");
            (file, text)
        })
        .collect()
}

/// Where the screen's own DPI is asked in the code of the windows: every `dc_dpi(` outside the two
/// places allowed to ask it — the one function of our drawing, which multiplies it by the step, and
/// the measuring of the monitor before a window exists, which is not drawing (задача T-83-3).
fn screen_dpi_askers(files: &[(&'static str, String)]) -> Vec<String> {
    let allowed = |body_of: &str, text: &str| -> usize {
        if !text.contains(body_of) {
            return 0;
        }

        function_body(text, body_of)
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .map(|line| line.matches("dc_dpi(").count())
            .sum()
    };

    let mut found = Vec::new();

    for (file, text) in files {
        let calls: usize = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .map(|line| line.matches("dc_dpi(").count())
            .sum();

        let definitions =
            usize::from(*file == "theme.rs" && text.contains("pub fn dc_dpi(dc: HDC)"));
        let permitted = allowed("pub fn drawing_dpi(dc: HDC) -> i32 {", text)
            + allowed(
                "pub fn screen_of(owner: HWND, template: &[u8]) -> Option<Screen> {",
                text,
            );

        let extra = calls.saturating_sub(definitions + permitted);

        if extra > 0 {
            found.push(format!("{file}: {extra}"));
        }
    }

    found
}

/// **⛔⛔ All our drawing goes by the step — задача T-83-4, решение 143.4.**
///
/// The 28 places of the windows that scaled a frame, a radius, an inset, the pitch of a line, the
/// logo or a glyph by the DPI of the **screen** go through `theme::drawing_dpi` now — the screen ×
/// the step of the window's font / 10. The sweep reads the four files of the windows and finds no
/// `dc_dpi(` outside that function and the measuring of the monitor.
///
/// ⚠ Отрицательный контроль: the text of `e82` — every `drawing_dpi(` put back to `dc_dpi(` — is
/// caught at the 28 places, and the mutant of the task — one place put back — at the one.
#[test]
fn all_our_drawing_goes_by_the_step() {
    let files = window_code();

    let askers = screen_dpi_askers(&files);
    assert!(
        askers.is_empty(),
        "⛔⛔ drawing by the screen's DPI, not by the step: {askers:?}"
    );

    let calls: usize = files
        .iter()
        .map(|(_, text)| {
            text.lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .map(|line| line.matches("drawing_dpi(").count())
                .sum::<usize>()
        })
        .sum();

    // The 28 places, the definition, and — since задача T-83-6, находка 143б — the one carry of the
    // paint buffer: `PaintBuffer::for_rect` asks it of the DC of the message for the surface it
    // makes, so that a drawing asking it of the surface gets the window's step. Its use inside the
    // function itself is none — the body asks `dc_dpi`.
    //
    // FR-92а: four of the 28 were the drawing of the old «О программе», which went to the shell;
    // T-120-16: two more were the drawing of the old letters; T-120-20: the old settings dialog went
    // with its places, the box of T-118-1 (`repaint_capture`) and the pictures of its check boxes
    // among them. 16 are left (counted on the merge of stage/e120-shell into stage/e120-win-b).
    assert_eq!(
        calls,
        16 + 1 + 1,
        "the 16 places of П4 left, the definition and the buffer's carry"
    );

    // ⚠ Контроль «текст e82»: every place put back — the 28 of П4 and the buffer's carry.
    let of_e82: Vec<(&'static str, String)> = files
        .iter()
        .map(|(file, text)| {
            (
                *file,
                text.replace(
                    "pub fn drawing_dpi(dc: HDC)",
                    "pub fn drawing_dpi_KEPT(dc: HDC)",
                )
                .replace("drawing_dpi(", "dc_dpi(")
                .replace(
                    "pub fn drawing_dpi_KEPT(dc: HDC)",
                    "pub fn drawing_dpi(dc: HDC)",
                ),
            )
        })
        .collect();
    let caught: usize = screen_dpi_askers(&of_e82)
        .iter()
        .map(|line| {
            line.rsplit(": ")
                .next()
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(0)
        })
        .sum();
    assert_eq!(
        caught,
        16 + 1,
        "the text of e82 must be caught at all 16 places left and at the buffer's carry"
    );

    // ⚠ Контроль «мутант задачи»: one place of `letters.rs` put back.
    let mutant: Vec<(&'static str, String)> = files
        .iter()
        .map(|(file, text)| {
            let changed = if *file == "letters.rs" {
                text.replacen("theme::drawing_dpi(dc)", "theme::dc_dpi(dc)", 1)
            } else {
                text.clone()
            };
            (*file, changed)
        })
        .collect();
    assert_eq!(
        screen_dpi_askers(&mutant),
        vec!["letters.rs: 1".to_owned()],
        "one place put back must be caught"
    );
}

/// **The DPI of our drawing is the screen's at ten points and the step's below — задача T-83-4.**
///
/// The pure halves: the face turned back into its step, and the step into the DPI of the drawing.
#[test]
fn the_drawing_dpi_follows_the_step_of_the_font() {
    // A step makes one face, and the face gives the step back.
    for dpi in [96, 120, 144, 168, 192] {
        for step in 6..=15_u16 {
            let face = theme::face_at(step, dpi);
            assert_eq!(
                theme::step_of_face(face, dpi),
                step,
                "{dpi} DPI: {step} pt is {face} px, and {face} px must read {step} pt back"
            );
        }
    }

    // A face no step makes reads the template's own ten.
    assert_eq!(theme::step_of_face(10, 96), 10);

    // At ten points the drawing is the screen's to the unit; at eight on 96 DPI it is 76.
    for dpi in [96, 120, 144, 168, 192] {
        assert_eq!(theme::drawing_dpi_at(dpi, 10), dpi);
    }
    assert_eq!(theme::drawing_dpi_at(96, 8), 96 * 8 / 10);
    assert_eq!(
        theme::drawing_dpi_at(120, 8),
        96,
        "125 % at 8 pt draws as 100 %"
    );
    assert_eq!(theme::drawing_dpi_at(144, 7), 100);
}

/// **The live window draws by its step — задача T-83-4, the check of the function on a window.**
///
/// «От автора», hidden, at ten points and at eight on this 96-DPI machine: at ten its drawing DPI
/// is the screen's own; at eight it is 96 × 8 / 10, and the corner radius, the frame, the pitch of
/// a line and the logo are `scaled(x, 76)` — the logo is read off the window itself, the size of its
/// icon control after `STM_SETICON`.
///
/// ⚠ «Красное до»: the drawing of `e82` ignores the step — 96 and a 40 px logo at eight points.
#[cfg(feature = "testing")]
#[test]
fn the_live_window_draws_by_its_step() {
    use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};

    let _guard = with_product_strings();
    settings::set_ui_language(Language::Ru);

    let at = |step: u16| -> (i32, i32, i32) {
        settings::testing::with_font_step(step, || {
            let window = HiddenWindow::open("0.83.0");
            // SAFETY: the window is live for this block; the DC is released at once.
            let (screen, drawing) = unsafe {
                let dc = GetDC(Some(window.window));
                let answer = (theme::dc_dpi(dc), theme::drawing_dpi(dc));
                ReleaseDC(Some(window.window), dc);
                answer
            };
            let logo = window.control_rect(1260);
            (screen, drawing, logo.right - logo.left)
        })
    };

    let (screen, at_ten, logo_at_ten) = at(10);
    let (_, at_eight, logo_at_eight) = at(8);

    println!(
        "screen {screen} DPI; drawing at 10 pt {at_ten}, logo {logo_at_ten} px; at 8 pt {at_eight}, \
         logo {logo_at_eight} px"
    );

    assert_eq!(
        at_ten, screen,
        "at ten points our drawing is the screen's own DPI"
    );
    assert_eq!(
        at_eight,
        screen * 8 / 10,
        "at eight points it is the screen's × 8 / 10"
    );
    assert_eq!(
        logo_at_eight,
        theme::scaled(56, screen * 8 / 10),
        "the logo is scaled by the step"
    );
    assert_eq!(logo_at_ten, theme::scaled(56, screen));

    for (what, length) in [
        ("the corner radius", 6),
        ("the frame", 1),
        ("a line's pitch", 23),
    ] {
        assert_eq!(
            theme::scaled(length, at_eight),
            theme::scaled(length, screen * 8 / 10),
            "{what} at eight points"
        );
    }
}

/// **⛔⛔ The paint buffer draws by the step of the window it stands for — задача T-83-6, находка
/// 143б живой приёмки `e83`.**
///
/// The labels of «От автора» and of the wizard are drawn through `theme::PaintBuffer` (Т-33-8,
/// against the flicker), and a memory DC belongs to no window: `drawing_dpi` found no font behind
/// it and answered the DPI of the screen. At 150 % the lines of a paragraph came out 25 px apart
/// where the layout had allowed 21, and the last line went under the buttons — on the owner's
/// screens the bands of pixels are 25 px apart at 150 % and 29 at 175 %, `scaled(23, 144)` and
/// `scaled(23, 168)`. The buffer carries the drawing DPI of the DC it is made for over, as it
/// carries the face.
///
/// A hidden window given the face of each step on this screen: a buffer made for its DC answers
/// what the DC answers — the screen's own at ten points, screen × step / 10 below — and so does a
/// buffer made for that buffer; a memory DC that is no buffer of ours answers the screen, as before.
///
/// ⚠ «Красное до» (`e83`): the buffer answers the screen's 96 at every step.
#[test]
fn the_paint_buffer_draws_by_the_step_of_the_window_it_stands_for() {
    use windows::Win32::Foundation::{LPARAM, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateFontIndirectW, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, SendMessageW, WM_SETFONT, WS_EX_TOOLWINDOW, WS_POPUP,
    };
    use windows::core::{PCWSTR, w};

    let area = RECT {
        left: 0,
        top: 0,
        right: 120,
        bottom: 40,
    };
    let mut wrong = Vec::new();

    for step in [10_u16, 9, 8, 7, 6] {
        // SAFETY: a system class, no parent, never shown; destroyed at the end of the pass.
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                400,
                200,
                None,
                None,
                None,
                None,
            )
        }
        .expect("a hidden window must be creatable");

        // SAFETY: the window is live; the DC is released at the end of the pass.
        let dc = unsafe { GetDC(Some(window)) };
        let screen = theme::dc_dpi(dc);
        let face = theme::face_at(step, screen);

        // SAFETY: a plain font of this pass, deleted after the window that carries it.
        let font = unsafe {
            CreateFontIndirectW(&LOGFONTW {
                lfHeight: -face,
                lfWeight: 400,
                ..Default::default()
            })
        };

        // SAFETY: our own window of this thread; the font outlives it.
        unsafe {
            SendMessageW(
                window,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(0)),
            )
        };

        let of_window = theme::drawing_dpi(dc);

        // SAFETY: `dc` is live for the calls; each buffer frees its own DC and bitmap on drop,
        // the inner one first.
        let (of_buffer, of_nested) = unsafe {
            let buffer = theme::PaintBuffer::for_rect(dc, &area).expect("a buffer");
            let nested =
                theme::PaintBuffer::for_rect(buffer.dc(), &area).expect("a buffer of a buffer");
            (
                theme::drawing_dpi(buffer.dc()),
                theme::drawing_dpi(nested.dc()),
            )
        };

        // A memory DC that is no buffer of ours belongs to no window, as before this task.
        //
        // SAFETY: made and deleted here, never selected into.
        let of_bare = unsafe {
            let bare = CreateCompatibleDC(Some(dc));
            let answer = theme::drawing_dpi(bare);
            let _ = DeleteDC(bare);
            answer
        };

        println!(
            "{step} pt, face {face} px: the window's DC {of_window}, its buffer {of_buffer}, a \
             buffer of the buffer {of_nested}, a bare memory DC {of_bare}"
        );

        let expected = theme::drawing_dpi_at(screen, step);

        if (of_window, of_buffer, of_nested, of_bare) != (expected, expected, expected, screen) {
            wrong.push(format!(
                "{step} pt: window {of_window}, buffer {of_buffer}, nested {of_nested}, bare \
                 {of_bare} — expected {expected}, {expected}, {expected}, {screen}"
            ));
        }

        // SAFETY: releases the DC taken above and destroys our window, then deletes its font.
        unsafe {
            ReleaseDC(Some(window), dc);
            let _ = DestroyWindow(window);
            let _ = DeleteObject(font.into());
        }
    }

    assert!(
        wrong.is_empty(),
        "⛔⛔ 143б: a paint buffer does not draw by the step of its window — {wrong:?}"
    );
}

/// **The record of a paint buffer lives exactly as long as the buffer — задача T-83-6.** Otherwise
/// the list grows by one with every element painted, and a handle `DeleteDC` frees and GDI hands to
/// the next memory DC answers the DPI of a buffer that is gone.
///
/// ⚠ The mutant «`Drop` forgets nothing» passes
/// `the_paint_buffer_draws_by_the_step_of_the_window_it_stands_for` — a freed handle is not handed
/// out again reliably enough for its bare memory DC to meet it (`mutant-T-83-6-no-forget.log`) —
/// and is caught here.
#[cfg(feature = "testing")]
#[test]
fn the_record_of_a_paint_buffer_lives_as_long_as_the_buffer() {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{GetDC, ReleaseDC};

    let area = RECT {
        left: 0,
        top: 0,
        right: 64,
        bottom: 24,
    };
    let before = theme::testing::buffers_recorded();

    // SAFETY: the screen DC of this process, released at the end; both buffers are dropped before
    // it, the inner one first.
    unsafe {
        let screen = GetDC(None);

        {
            let buffer = theme::PaintBuffer::for_rect(screen, &area).expect("a buffer");
            assert_eq!(
                theme::testing::buffers_recorded(),
                before + 1,
                "one buffer, one record"
            );

            {
                let _nested =
                    theme::PaintBuffer::for_rect(buffer.dc(), &area).expect("a buffer of a buffer");
                assert_eq!(theme::testing::buffers_recorded(), before + 2);
            }

            assert_eq!(
                theme::testing::buffers_recorded(),
                before + 1,
                "the inner record goes with the inner buffer"
            );
        }

        assert_eq!(
            theme::testing::buffers_recorded(),
            before,
            "every record goes with its buffer"
        );

        ReleaseDC(None, screen);
    }
}

/// **⛔⛔ The paragraphs of «От автора» are drawn inside their rectangles at every step — задача
/// T-83-6, находка 143б.**
///
/// The layout gives a paragraph the height of its lines at the pitch of the body face; the drawing
/// sets it in the window's text face at the model pitch of a label, `scaled(23, drawing DPI)`, which
/// the model refuses whenever it is not larger than the face's own line (В-6). Drawn through the
/// buffer at the screen's DPI, the model pitch came out larger than the body's pitch at the steps
/// of 150 % and 175 %, and the last line was cut: at 150 %, 94 px of drawing in 83 px of room.
///
/// The live window, hidden, in the fourteen locales at the five steps on this screen; each
/// paragraph drawn in the text face with the label's own pitch is measured **the way its drawing
/// measures it** — through a `PaintBuffer` made for the control's own DC, by `theme::measure_label`,
/// the documented twin of the painter — and must not ask for more than the rectangle the layout gave
/// it.
///
/// ⚠ At 96 DPI the steps 7 and 6 are faces of 9 and 8 px, which the rule never shows on this screen
/// — but the error is a ratio, screen / drawing = 10 / step, and 96 at step 7 is the ratio of 144 at
/// step 7: the lab stands for 150 %. «Красное до» (`e83`): cut at 7 and 6 points.
#[cfg(feature = "testing")]
#[test]
fn the_paragraphs_of_the_author_window_are_drawn_inside_their_rectangles() {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        CreateFontIndirectW, DeleteObject, GetDC, GetObjectW, HGDIOBJ, ReleaseDC, SelectObject,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetDlgItem, GetWindowLongW, GetWindowTextW, SendMessageW, WM_GETFONT, WS_VISIBLE,
    };

    /// What «От автора» draws in the text face at the label's own pitch — `letters::label_face`
    /// answers `(faces.text, None)` for each of them.
    const PARAGRAPHS: [(&str, i32); 6] = [
        ("IDC_AUTHOR_TEXT", 1264),
        ("IDC_NEWS_ABOUT", 1268),
        ("IDC_NEWS_STATE", 1269),
        ("IDC_NEWS_VERSION", 1270),
        ("IDC_NEWS_FILE_ONLY", 1273),
        ("IDC_FEEDBACK_TEXT", 1277),
    ];

    let _guard = with_product_strings();
    let mut cut = Vec::new();
    let mut measured = 0_usize;

    for step in [10_u16, 9, 8, 7, 6] {
        for language in Language::shipped() {
            settings::set_ui_language(language);

            settings::testing::with_font_step(step, || {
                let window = HiddenWindow::open("0.83.0");

                // SAFETY: the window and its children are live for this block; the face made here
                // is selected out of every DC before it is deleted, and every DC is released.
                unsafe {
                    let font = SendMessageW(window.window, WM_GETFONT, None, None);
                    let mut base = LOGFONTW::default();
                    let _ = GetObjectW(
                        HGDIOBJ(font.0 as *mut _),
                        i32::try_from(size_of::<LOGFONTW>()).expect("small"),
                        Some(std::ptr::from_mut(&mut base).cast()),
                    );
                    let text_face = CreateFontIndirectW(&theme::smoothed_logfont(base));

                    for (name, control) in PARAGRAPHS {
                        let Ok(child) = GetDlgItem(Some(window.window), control) else {
                            continue;
                        };

                        if GetWindowLongW(child, GWL_STYLE) as u32 & WS_VISIBLE.0 == 0 {
                            continue;
                        }

                        let rect = window.control_rect(control);
                        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
                        let mut text = [0_u16; 1024];
                        let copied = usize::try_from(GetWindowTextW(child, &mut text)).unwrap_or(0);

                        if copied == 0 || width <= 0 || height <= 0 {
                            continue;
                        }

                        let mut caption = text[..copied].to_vec();
                        let dc = GetDC(Some(child));
                        let area = RECT {
                            left: 0,
                            top: 0,
                            right: width,
                            bottom: height,
                        };
                        let asked = theme::PaintBuffer::for_rect(dc, &area)
                            .map(|buffer| {
                                let previous = SelectObject(buffer.dc(), text_face.into());
                                let asked =
                                    theme::measure_label(buffer.dc(), width, &mut caption, None);
                                SelectObject(buffer.dc(), previous);
                                asked
                            })
                            .expect("a buffer for the paragraph");
                        ReleaseDC(Some(child), dc);
                        measured += 1;

                        if asked > height {
                            cut.push(format!(
                                "{step} pt, {language:?}, {name}: the drawing asks {asked} px in \
                                 {height}"
                            ));
                        }
                    }

                    let _ = DeleteObject(text_face.into());
                }
            });
        }
    }

    settings::set_ui_language(Language::Ru);

    println!("{measured} paragraphs measured; cut: {cut:?}");

    assert!(
        measured >= 5 * 14 * 4,
        "every step, every locale, at least the four paragraphs that always show: {measured}"
    );
    assert!(
        cut.is_empty(),
        "⛔⛔ 143б: a paragraph of «От автора» is drawn past its rectangle — {cut:?}"
    );
}

/// **The pitch the drawing takes fits the room the layout gives, on every screen of the rule —
/// задача T-83-6, находка 143б.** The model of the two halves, at the real faces.
///
/// For every scale Windows offers × the steps 10…6 pt (the face not under the floor) and for a
/// paragraph of two to six lines: the layout's height at the body face and its pitch
/// (`settings::about_body_line_pitch`, the face one step larger — `ABOUT_BODY_POINTS_TENTHS` over
/// `DIALOG_FONT_POINTS_TENTHS`), against the drawing's in the text face at `scaled(23, DPI)` or its
/// own line when the model refuses (`theme::label_model_pitch`). Drawn **at the DPI of the
/// drawing** — what the buffer answers since this task — it never asks for more than it was given.
///
/// ⚠ Контроль «буфер e83»: the same drawing at the DPI of the **screen** is caught — at 150 %,
/// 7 pt, four lines: 94 px in 83, the owner's screen.
#[test]
fn the_pitch_of_the_drawing_fits_the_room_of_the_layout_on_every_screen_of_the_rule() {
    const DPIS: [i32; 8] = [96, 120, 144, 168, 192, 216, 240, 288];

    let height_at = |natural: i32, pitch: i32, lines: i32| -> i32 {
        match theme::label_model_pitch(lines, natural, pitch) {
            Some(pitch) => (lines - 1) * pitch + natural,
            None => lines * natural,
        }
    };

    let mut cut = Vec::new();
    let mut caught = Vec::new();

    for dpi in DPIS {
        for step in settings::FONT_STEPS {
            let face = theme::face_at(step, dpi);

            if face < settings::FONT_STEP_FLOOR {
                continue;
            }

            let body =
                face * settings::ABOUT_BODY_POINTS_TENTHS / settings::DIALOG_FONT_POINTS_TENTHS;
            let body_pitch = settings::about_body_line_pitch(-body);
            let drawing = theme::drawing_dpi_at(dpi, step);

            for lines in 2..=6 {
                let room = height_at(cell_of(body), body_pitch, lines);
                let asked = height_at(
                    cell_of(face),
                    theme::scaled(theme::LABEL_LINE_PITCH, drawing),
                    lines,
                );
                let asked_at_the_screen = height_at(
                    cell_of(face),
                    theme::scaled(theme::LABEL_LINE_PITCH, dpi),
                    lines,
                );

                if asked > room {
                    cut.push(format!(
                        "{dpi} DPI, {step} pt, {lines} lines: {asked} px in {room}"
                    ));
                }

                if asked_at_the_screen > room {
                    caught.push((dpi, step, lines, asked_at_the_screen, room));
                }
            }
        }
    }

    assert!(
        cut.is_empty(),
        "⛔⛔ 143б: the drawing asks for more than the layout gives — {cut:?}"
    );
    assert!(
        caught.contains(&(144, 7, 4, 94, 83)),
        "the control — the drawing of e83 at the screen's DPI — must be caught at 150 %, 7 pt, four \
         lines, 94 px in 83: {caught:?}"
    );
}

/// **⛔ A grown window moves only when it would leave the air of the rule — решение 143.10 (а),
/// задача T-83-3.**
///
/// `DS_CENTER` centres «От автора» for its template of 345 units, and the window then grows
/// downwards to its content. On the two screens the owner will look at:
///
/// * **100 %, 1080p, 10 pt** — the greek window, the tallest, stands at 130 from the top and keeps
///   34 px at the bottom: it **stays** — not a pixel moves at 100 %;
/// * **150 %, 1080p, 7 pt** — the template window stands at 66 and the grown greek one would end
///   at 1027 in a work area of 1008, **under the task bar** (the shape of `e82`): it is put in the
///   middle, 23 px above and 24 below.
///
/// The heights are the model's, held against the live window above; the frames 39 and 56 are the
/// measured ones. A window taller than the work area goes to its top.
///
/// ⚠ «Красное до»: the placement of `e82` keeps the top, and the 150 % window ends 19 px past the air.
#[test]
fn a_grown_window_moves_only_when_it_would_leave_the_air() {
    /// Решение 143.3 — written here apart from the code.
    const AIR: i32 = 12;

    let _guard = with_product_strings();
    settings::set_ui_language(Language::El);
    let words = AuthorWords::now("0.83.0");
    settings::set_ui_language(Language::Ru);

    // `(what, work area, frame, face, drawing DPI, where it must stand, and whether it moves)`.
    let screens = [
        ("100 %, 10 pt", 1032, 39, 13, 96),
        ("150 %, 7 pt", 1008, 56, 14, 100),
    ];

    for (what, work, frame, face, drawing) in screens {
        let cell = cell_of(face);
        let template = mul_div(345, cell, 8) + frame;
        let centred_for_the_template = (work - template) / 2;
        let grown = author_client_by_model(face, drawing, &words) + frame;

        let top = settings::grown_window_top(centred_for_the_template, grown, 0, work);
        let keeps_the_air = |top: i32| top >= AIR && work - (top + grown) >= AIR;

        println!(
            "{what}: the template window {template} px stands at {centred_for_the_template}; grown \
             to {grown} px it would end at {}; it stands at {top} — {} px above, {} px below",
            centred_for_the_template + grown,
            top,
            work - (top + grown)
        );

        assert!(
            keeps_the_air(top),
            "{what}: the grown window must keep {AIR} px above and below"
        );

        if keeps_the_air(centred_for_the_template) {
            assert_eq!(
                top, centred_for_the_template,
                "{what}: a window that keeps the air must not move by a pixel"
            );
        } else {
            assert_eq!(
                top,
                (work - grown) / 2,
                "{what}: a window that would leave the air goes to the middle"
            );
        }
    }

    // The two screens are the two cases: at 100 % the greek window keeps the air where it stands,
    // at 150 % it would not — the shape of `e82`, where the top was kept and the bottom went under
    // the task bar.
    //
    // ⭐ **Task T-119-10, until T-120-24 — delete with the table:** «От автора» lost the switch of
    // the feed and its long line, and the greek window at 150 % (878 px) no longer leaves the air.
    // The rule is held on a window of the height of `e82` itself — 961 px from 66, ending at 1027
    // in a work area of 1008 — so the control still proves something.
    let at_150 = ((1008 - (mul_div(345, cell_of(14), 8) + 56)) / 2, 961);
    assert!(
        1008 - (at_150.0 + at_150.1) < AIR,
        "the control: at 150 % the window of e82 must leave the air — else this test proves nothing"
    );
    assert_eq!(
        settings::grown_window_top(at_150.0, at_150.1, 0, 1008),
        (1008 - at_150.1) / 2,
        "the window of e82 goes to the middle"
    );

    // A window taller than the work area goes to its top, never above it.
    assert_eq!(settings::grown_window_top(-40, 1200, 0, 1008), 0);
}

/// **The live «От автора» stands where решение 143.10 puts it — задача T-83-3.**
///
/// At the template's ten points the grown window stays where the manager centred it for its
/// template — exactly; at fifteen points on this 96-DPI machine (the debug substitution) the greek
/// window no longer keeps the air where it stands and is put in the middle of the work area.
#[cfg(feature = "testing")]
#[test]
fn the_live_author_window_stands_where_the_rule_expects() {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let _guard = with_product_strings();
    settings::set_ui_language(Language::El);

    for step in [10_u16, 11, 15] {
        let face = theme::face_at(step, 96);
        let (outer, client, work) = settings::testing::with_font_step(step, || {
            let window = HiddenWindow::open("0.83.0");
            let mut outer = RECT::default();
            // SAFETY: the window is live for this block; the buffers are locals.
            let work = unsafe {
                let _ = GetWindowRect(window.window, &mut outer);
                let monitor = MonitorFromWindow(window.window, MONITOR_DEFAULTTONEAREST);
                let mut info = MONITORINFO {
                    cbSize: u32::try_from(size_of::<MONITORINFO>()).expect("small"),
                    ..Default::default()
                };
                let _ = GetMonitorInfoW(monitor, &mut info);
                info.rcWork
            };
            (outer, window.client_height(), work)
        });

        let frame = (outer.bottom - outer.top) - client;
        let template = mul_div(345, cell_of(face), 8) + frame;
        let centred = work.top + ((work.bottom - work.top) - template) / 2;
        let expected =
            settings::grown_window_top(centred, outer.bottom - outer.top, work.top, work.bottom);

        println!(
            "El {step} pt ({face} px): the window {} px stands at {} (the template's centre {centred}, \
             the rule's place {expected}); {} px below",
            outer.bottom - outer.top,
            outer.top,
            work.bottom - outer.bottom
        );

        assert_eq!(
            outer.top, expected,
            "El {step} pt: the place of решение 143.10"
        );

        if step == 10 {
            assert_eq!(
                outer.top, centred,
                "at the template's ten points the window must stand where it always stood"
            );
        }
    }

    settings::set_ui_language(Language::Ru);
}

/// **The monitor is measured before the window exists — задача T-83-3, посылка П5.**
///
/// `settings::screen_of` asks, for an owner shaped like `LangSwitcher.Hidden`, the work area of the
/// monitor `DS_CENTER` will use, its DPI, and the frame the template's style costs there. Held here
/// against what the dialog manager itself does with the same template on the same owner: the frame
/// of the real window — at 96 DPI the **39 px** of решение 143 — and the monitor it lands on.
#[test]
fn the_monitor_is_measured_before_the_window_exists() {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    };

    let product = ProductImage::open();
    let owner = HiddenOwner::new();

    for (id, _) in TEMPLATES {
        let template = product.resource(RT_DIALOG, id);
        let screen = settings::screen_of(owner.0, &template)
            .unwrap_or_else(|| panic!("template {id}: the monitor must be measurable"));

        // SAFETY: a live owner; the buffer is a live local whose `cbSize` the call reads first.
        let work_area = unsafe {
            let monitor = MonitorFromWindow(owner.0, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: u32::try_from(size_of::<MONITORINFO>()).expect("small"),
                ..Default::default()
            };
            assert!(GetMonitorInfoW(monitor, &mut info).as_bool());
            info.rcWork.bottom - info.rcWork.top
        };

        let (_, outer, client) = open_template_bare(&owner, product.module, &template);
        let real_frame = (outer.bottom - outer.top) - (client.bottom - client.top);

        println!(
            "template {id}: work area {} px, DPI {}, frame {} px (the real window's {real_frame}); \
             the rule answers {} pt",
            screen.work_area,
            screen.dpi,
            screen.frame,
            settings::font_step(
                screen.work_area,
                screen.dpi,
                screen.frame,
                |face| settings::tallest_window_units_in(Language::Ru, face),
                cell_of
            )
        );

        assert_eq!(
            screen.work_area, work_area,
            "template {id}: the owner's work area"
        );
        assert_eq!(
            screen.frame, real_frame,
            "template {id}: the frame measured before the window must be the window's own"
        );
        if screen.dpi == 96 {
            assert_eq!(
                screen.frame, 39,
                "template {id}: 39 px at 96 DPI — решение 143"
            );
        }
    }
}

/// **Task T-42-12, finding 124б.1, решение 124б.1** — a window taller than its monitor is
/// shortened to it and scrolls; a window that fits is not touched at all.
///
/// # The defect, in the numbers the owner met
///
/// `IDD_SETTINGS` is 375 dialog units tall, and a dialog unit follows the display scale — 17 px
/// at 100 %, 23 px at 125 %. The window is therefore ~826 px on one scale and **~1113 px** on
/// the other, against a work area of **1032 px** on a 1080p screen with a task bar. At 125 %
/// the bottom row of buttons stood off the edge of the screen and could not be clicked at all:
/// «нижняя часть окна приложения не вписывается в экран и у меня нет возможности нажать
/// кнопки».
///
/// ⚠ **Shrinking the layout is not a cure, and that is why the scroll was chosen** (решение
/// 124б.1): 28 units would have to go for 125 %, but at 150 % the window wants ~1330 px and at
/// 200 % ~1770 px — no layout fits those on a 1080p screen.
///
/// The test is of the arithmetic the window lays itself out by, so it runs at every scale on
/// any machine: `shortened_client_height` decides whether to shorten and by how much, and
/// `scrolled_to` answers every command a scroll bar can send.
#[test]
fn a_window_taller_than_its_monitor_is_shortened_and_scrolls() {
    use windows::Win32::UI::WindowsAndMessaging::{
        SB_BOTTOM, SB_ENDSCROLL, SB_LINEDOWN, SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION,
        SB_THUMBTRACK, SB_TOP,
    };

    // 1. The decision to shorten at all. The frame — caption plus borders — is what the window
    //    costs besides its client area: 29 + 6 at 125 %, measured through GetSystemMetricsForDpi.
    let frame = 35;
    let work_area = 1032;

    // 100 %: 375 units at 17 px per 8 units is 797 px of client, 832 with the frame — it fits,
    // and NOTHING is done. The look of решение 124.1 is untouched at the scale it was accepted
    // at, and that is the whole of the promise.
    assert_eq!(
        settings::shortened_client_height(797, frame, work_area),
        None,
        "a window that fits its monitor is not shortened, and gets no scroll bar"
    );

    // 125 %: 375 units at 23 px is 1078 px of client, 1113 with the frame — 81 px too tall.
    assert_eq!(
        settings::shortened_client_height(1078, frame, work_area),
        Some(work_area - frame),
        "a window taller than the work area is cut to exactly what fits"
    );

    // 150 %, 175 %, 200 %: taller still, and each is cut to the same fitting height — the
    // contents differ, the window does not.
    for wanted in [1313, 1453, 1734] {
        assert_eq!(
            settings::shortened_client_height(wanted, frame, work_area),
            Some(work_area - frame),
            "every scale above the fitting one is cut to the work area"
        );
    }

    // A monitor so short that even the frame does not fit leaves one pixel of client rather
    // than a negative height (NFR-13).
    assert_eq!(
        settings::shortened_client_height(1078, 200, 100),
        Some(1),
        "a client area is never asked to be zero or negative"
    );

    // Exactly the work area is not «too tall»: the boundary belongs to the window.
    assert_eq!(
        settings::shortened_client_height(work_area - frame, frame, work_area),
        None,
        "a window exactly as tall as the work area fits it"
    );

    // 2. The scroll itself. The view of the 125 % case: 1078 px of contents in 997 px of window.
    let view = settings::ScrollView {
        offset: 0,
        content: 1078,
        page: work_area - frame,
    };

    println!(
        "125 %: содержимое {} px, окно {} px, прокрутка до {} px",
        view.content,
        view.page,
        view.most()
    );

    assert_eq!(
        view.most(),
        1078 - 997,
        "the scroll ends where the contents do"
    );

    // A line down and a line up come back to where they started — the arrows of the bar.
    let down = settings::scrolled_to(view, SB_LINEDOWN.0, 0);
    assert!(down > 0, "a line down moves the contents");
    assert_eq!(
        settings::scrolled_to(
            settings::ScrollView {
                offset: down,
                ..view
            },
            SB_LINEUP.0,
            0
        ),
        0,
        "a line up undoes a line down"
    );

    // The page commands move by the height of the window, and neither runs past an end.
    assert_eq!(
        settings::scrolled_to(view, SB_PAGEUP.0, 0),
        0,
        "a page up at the top stays at the top"
    );
    assert_eq!(
        settings::scrolled_to(view, SB_PAGEDOWN.0, 0),
        view.most(),
        "a page down from the top lands at the bottom — the contents are less than two pages"
    );

    // The thumb goes where it is dragged, and not past the ends.
    assert_eq!(settings::scrolled_to(view, SB_THUMBTRACK.0, 40), 40);
    assert_eq!(settings::scrolled_to(view, SB_THUMBTRACK.0, -5), 0);
    assert_eq!(
        settings::scrolled_to(view, SB_THUMBPOSITION.0, 9999),
        view.most(),
        "a thumb dragged past the end stops at the end"
    );

    // The ends themselves, and the command that means «the drag is over»: it moves nothing.
    assert_eq!(settings::scrolled_to(view, SB_BOTTOM.0, 0), view.most());
    assert_eq!(
        settings::scrolled_to(settings::ScrollView { offset: 40, ..view }, SB_TOP.0, 0),
        0
    );
    assert_eq!(
        settings::scrolled_to(
            settings::ScrollView { offset: 40, ..view },
            SB_ENDSCROLL.0,
            0
        ),
        40,
        "SB_ENDSCROLL leaves the scroll exactly where the drag left it"
    );

    // 3. A window that fits has nothing to scroll: every command answers zero, so a stray
    //    `WM_VSCROLL` cannot move a window that was never shortened.
    let whole = settings::ScrollView {
        offset: 0,
        content: 797,
        page: 797,
    };

    assert_eq!(whole.most(), 0, "contents that fit have nowhere to scroll");

    for command in [SB_LINEDOWN.0, SB_PAGEDOWN.0, SB_BOTTOM.0, SB_THUMBTRACK.0] {
        assert_eq!(
            settings::scrolled_to(whole, command, 500),
            0,
            "a window that fits stays at the top whatever the scroll bar says"
        );
    }
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

/// **Закрытая часть комбобокса одинакова во ВСЕХ окнах программы** — решение 110.3,
/// задача Т-47-1.
///
/// # Находка, ради которой этот тест заведён
///
/// Слова пользователя 2026-09-09 со снимком шага 3 мастера: «высота поля выбора раскладки
/// отличается от типового принятого в общем окне». Замер до правки
/// (`scratchpad-Э47\красное-высота-комбо-e46.log`):
///
/// ```text
/// мастер, шаг 3: окно контрола 22 px, CB_GETITEMHEIGHT(-1) 16 px
/// настройки:     окно контрола 24 px, CB_GETITEMHEIGHT(-1) 18 px
/// ```
///
/// Прежняя мерка — «высота шрифта диалога + `CLOSED_ITEM_EXTRA`» — **зависела от шрифта окна**:
/// 9 pt мастера дают высоту 15 px, 10 pt окна настроек — 17 px, отсюда 16 и 18. Формула была
/// одна, а результат разный.
///
/// # Что проверяется, и почему этого нельзя проверить прежним тестом
///
/// [`the_closed_part_of_a_combo_box_is_the_twelve_dialog_units_of_the_generator`] выводит
/// `CLOSED_ITEM_EXTRA` из макета и проверяет **арифметику константы**. Он остался зелёным и
/// при дефекте, и после ремонта: он ничего не знает о том, что окон в программе два и шрифты у
/// них разные. Здесь проверяется само правило — **число, которое ставится контролу, одно на
/// все окна**, — на двух модельных высотах шрифта, измеренных на этой машине.
///
/// ⚠ Оконного дескриптора у теста нет, поэтому тело `widgets::combo::closed_height` повторено
/// здесь **арифметикой**, а то, что окно в самом деле ходит этой дорогой, проверяется чтением
/// исходника ниже — тем же приёмом, каким проверены другие правила этого файла.
/// **Все окна программы несут ОДИН кегль** — решение **116.6**, задача **T-51-1**.
///
/// # Почему этого сторожа не было и почему он нужен
///
/// До этой задачи в программе жили два кегля: `IDD_SETTINGS` просил 10 pt (решение 87 п. 3,
/// принятое глазом по снимкам стенда), остальные пять окон — 9 pt. Пользователь 2026-09-09:
/// «шрифт тоже нужно привести к общему типовому варианту», и выбрал 10 pt.
///
/// ⚠ **Правка кегля прошла всю батарею молча — 848/0 до и после.** Ни один тест не смотрел на
/// `FONT` шаблонов, хотя от этой строки зависит **единица диалога**, а значит вся вёрстка
/// каждого окна. Правило, которое ничем не закреплено, держится до первой правки шаблона.
///
/// Проверяется само правило, а не число в одном месте: **все** строки `FONT` файла ресурсов
/// обязаны совпадать между собой, и совпадать с кеглем окна настроек — эталона, который решение
/// 87 п. 3 поставило и с которого этот этап никого не двигает.
#[test]
fn every_dialog_of_the_program_asks_for_the_same_font() {
    let source =
        std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("app.rc"))
            .expect("app.rc must be readable")
            .replace("\r\n", "\n");

    let fonts: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("FONT "))
        .collect();

    println!("строк FONT в app.rc: {}", fonts.len());
    for line in &fonts {
        println!("  {line}");
    }

    assert_eq!(
        fonts.len(),
        2,
        "два окна — «От автора» и мастер («О программе» ушло на оболочку с FR-92а, письма и \
         «Последние письма» — с T-120-16, настройки — с T-120-20); если шаблонов стало больше, \
         об этом первым"
    );

    let first = fonts[0];

    for line in &fonts {
        assert_eq!(
            line, &first,
            "все окна обязаны просить один и тот же шрифт: единица диалога определяется им, и \
             два кегля — это две разные вёрстки (решение 116.6)"
        );
    }

    assert_eq!(
        first, "FONT 10, \"Segoe UI\", 400, 0, 0x1",
        "кегль эталона — окна настроек, поставленный решением 87 п. 3; этап Т-51 поднял к нему \
         остальные пять окон, а не опустил эталон"
    );
}

/// **Строка РАСКРЫТОГО списка — одна высота на все окна программы**, решение **115.2**, задача
/// **T-50-1**.
///
/// Родной брат теста ниже и написан по его образцу. Замер Э46 (`109а.6`, потом подтверждён
/// решением 110.4):
///
/// ```text
/// мастер, шаг 3: строка раскрытого списка 24 px
/// настройки:     строка раскрытого списка 26 px
/// ```
///
/// Прежняя мерка — «высота шрифта диалога + `LIST_ITEM_EXTRA`» — **зависела от шрифта окна**
/// ровно так же, как зависела мерка закрытой части до Э47: 9 pt мастера дают 15 px, 10 pt окна
/// настроек — 17, воздух `scaled(12, 96) = 9` одинаков, отсюда 24 и 26. Э46 привёл к общему виду
/// **тело** строки и положение слова в ней, Э47 — закрытую часть; сама высота строки не
/// трогалась ни разу.
///
/// ⚠ Оконного дескриптора у теста нет, поэтому тело `widgets::combo::list_row_height` повторено
/// здесь **арифметикой**, а то, что окна в самом деле ходят этой дорогой, проверяется чтением
/// исходников ниже — обоих, потому что `WM_MEASUREITEM` у мастера свой.
#[test]
fn a_row_of_the_dropped_down_list_is_the_same_height_in_every_window() {
    // Те же две модельные высоты, что и в тесте закрытой части: они измерены стендом на этой
    // машине и обе поставлены там же.
    const WIZARD_FONT: i32 = 15;
    const SETTINGS_FONT: i32 = 17;

    let air = theme::scaled(lang_switcher::widgets::combo::LIST_ITEM_EXTRA, 96);
    assert_eq!(air, 9, "воздух строки при 96 DPI — замер Э46");

    // ⭐ **Дефект, числами, прямо здесь.** Прежняя мерка — «шрифт окна + воздух» — и есть эти
    // две строчки: она даёт 24 и 26, что и намерил Э46. Оставлено в теле теста, чтобы «до» и
    // «после» читались рядом, а не в отчёте.
    assert_eq!(
        WIZARD_FONT + air,
        24,
        "мерка до T-50-2 давала мастеру 24 px"
    );
    assert_eq!(SETTINGS_FONT + air, 26, "…и окну настроек 26 px");

    // Тело `widgets::combo::list_row_height`, повторённое арифметикой.
    let row = |font: i32| {
        let by_font = font + air;
        let by_mockup = theme::scaled(lang_switcher::widgets::combo::LIST_BOX, 96);
        by_font.max(by_mockup)
    };

    let wizard = row(WIZARD_FONT);
    let settings = row(SETTINGS_FONT);

    println!("строка списка при 96 DPI: мастер {wizard} px, окно настроек {settings} px");

    assert_eq!(
        wizard, settings,
        "строка раскрытого списка обязана быть одной и той же во всех окнах программы, а вышло \
         {wizard} против {settings} — мерка снова зависит от шрифта окна (решение 115.2)"
    );

    // ⭐ И это ровно то число, которое окно настроек имело всегда: эталон не двигается, мастер
    // встаёт на него. Тот же принцип, что решение 110.3 применило к закрытой части.
    assert_eq!(
        settings,
        SETTINGS_FONT + air,
        "окно настроек обязано остаться при своей прежней строке — это эталон, а не предмет \
         правки"
    );
    assert_eq!(settings, 26, "и она равна 26 px при 96 DPI — замер Э46");

    // Пол по шрифту держит слово внутри строки, какой бы крупный шрифт окно ни несло.
    let huge = row(40);
    assert!(
        huge > theme::scaled(lang_switcher::widgets::combo::LIST_BOX, 96),
        "при крупном шрифте строка обязана подниматься выше макетной мерки — вышло {huge}"
    );
    assert_eq!(huge, 40 + air);

    // ⚠ **И строка списка обязана остаться ВЫШЕ закрытой части.** Одно `WM_MEASUREITEM` красит
    // обе высоты, закрытую двигает `CB_SETITEMHEIGHT(−1)`; макетное число, взятое меньше
    // закрытого, дало бы список с рядами ниже собственного поля.
    assert!(
        theme::scaled(lang_switcher::widgets::combo::LIST_BOX, 96)
            > theme::scaled(lang_switcher::widgets::combo::CLOSED_BOX, 96),
        "макетная строка списка обязана быть выше макетной закрытой части"
    );

    // ---- и что окна в самом деле ходят этой дорогой --------------------------------------
    {
        let (module, what) = ("letters.rs", "мастер и письма");
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join(module),
        )
        .unwrap_or_else(|error| panic!("src\\{module} must be readable: {error}"));

        assert!(
            source.contains("list_row_height(hwnd)"),
            "{what}: `WM_MEASUREITEM` обязан спрашивать высоту у общего слоя, иначе правило \
             живёт в одном окне из двух"
        );
    }
}

/// **Правило 118.1: высота раскрытого списка** — задача T-53a-1.
///
/// Тело чистое, окна не нужно: `dropped_height` — арифметика и ничего кроме.
///
/// Числа взяты из замера, а не из головы: строка списка **26**, закрытая коробка **24** —
/// `scratchpad-E53a\mechanic-probe.log`, все пять комбобоксов на 96 DPI.
#[test]
fn the_dropped_list_shows_at_most_the_ceiling_of_rows() {
    use lang_switcher::widgets::combo::{LIST_CEILING, dropped_height};

    const ROW: i32 = 26;
    const CLOSED_BOX: i32 = 24;
    const FRAME: i32 = 2;

    let height = |rows: i32| dropped_height(rows, ROW, CLOSED_BOX);

    // Ровно формула решения 118.1, на числах ТЗ: 0, 1, 2, 8, 9, 14.
    for rows in [0, 1, 2, 8, 9, 14] {
        assert_eq!(
            height(rows),
            rows.min(LIST_CEILING) * ROW + CLOSED_BOX + FRAME,
            "правило 118.1 обязано отвечать min(строк, потолок) × строку + коробку + рамку \
             (строк {rows})"
        );
    }

    // **Контроль 1 — потолок обязан быть потолком.** Выше него ответ не растёт.
    assert_eq!(
        height(9),
        height(LIST_CEILING),
        "девять строк обязаны дать ту же высоту, что восемь, — иначе потолок не потолок"
    );
    assert_eq!(
        height(14),
        height(LIST_CEILING),
        "четырнадцать строк — тоже: язык листается прокруткой, а не растит окно"
    );

    // **Контроль 2 — правило не обязано вырождаться в константу.** Ниже потолка ответ растёт.
    assert!(
        height(2) < height(LIST_CEILING),
        "две строки обязаны дать высоту СТРОГО меньшую, чем восемь, — иначе правило есть \
         константа, а список мастера снова не по числу раскладок"
    );
    assert!(
        height(0) < height(1) && height(1) < height(2),
        "каждая строка ниже потолка обязана прибавлять высоту"
    );

    // **Контроль 3 — ноль строк даёт одну рамку поверх закрытой коробки.** Это ровно то
    // состояние, которое замерено как красное «до» у мастера: список 203 × 2 px.
    assert_eq!(
        height(0),
        CLOSED_BOX + FRAME,
        "на пустом списке высота обязана быть коробкой и рамкой, и ничем больше"
    );

    // ⛔ **Отрицательное число строк не должно давать высоту меньше пустого списка.**
    // `CB_GETCOUNT` отвечает `CB_ERR` (−1) на контроле, которого нет, и такой ответ обязан
    // выродиться в пустой список, а не в отрицательную высоту.
    assert_eq!(
        height(-1),
        height(0),
        "отказ CB_GETCOUNT обязан читаться как пустой список, а не как минус строка"
    );
}

/// **Сторож потолка — по смыслу, а не по букве строки кода** (⚠ урок Э50: сторож на
/// буквальную строку ломается от перестановки рядом).
///
/// Он не читает исходник и не знает, как потолок записан. Он спрашивает у самого правила, на
/// каком числе строк ответ перестаёт расти, и требует, чтобы это число совпало с
/// `LIST_CEILING`. Подвинут потолок — сторож краснеет; переставлены строки, переименована
/// константа, переписано тело — сторож молчит, потому что смысл цел.
#[test]
fn the_ceiling_of_the_dropped_list_is_where_the_rule_stops_growing() {
    use lang_switcher::widgets::combo::{LIST_CEILING, dropped_height};

    const ROW: i32 = 26;
    const CLOSED_BOX: i32 = 24;

    let height = |rows: i32| dropped_height(rows, ROW, CLOSED_BOX);

    // ⚠⚠ **Число владельца, записанное ЗДЕСЬ, а не прочитанное из кода.** Первая редакция
    // этого сторожа выводила ожидание из самой `LIST_CEILING` — и потому молчала, когда
    // мутация двигала потолок 8 → 9 (замер T-53a-1). Сторож, который спрашивает у подсудимого,
    // что считать правдой, не сторож. Восьмёрку назвал владелец решением **118.1** от
    // 2026-09-10; двигать её можно **только его словом** и правкой обоих чисел разом —
    // память `canon-numbers-are-not-principles`.
    const OWNERS_CEILING: i32 = 8;

    assert_eq!(
        LIST_CEILING, OWNERS_CEILING,
        "потолок раскрытого списка — число владельца (решение 118.1). Разошлось с кодом: \
         подвинуть его можно только словом владельца, и тогда правятся ОБА числа"
    );

    // Ищем перегиб сами, ничего о его положении не предполагая: первое число строк, после
    // которого высота больше не растёт. Верхняя граница поиска заведомо выше любого списка
    // программы — четырнадцать языков.
    let found = (1..=64).find(|rows| height(*rows) == height(rows + 1));

    assert_eq!(
        found,
        Some(OWNERS_CEILING),
        "правило обязано переставать расти ровно на восьмой строке; замерено {found:?}"
    );

    // **И следствие, которого владелец ждёт глазом** (приёмка T-53a-6 п. 2): список из
    // четырнадцати языков показывает восемь строк и листается прокруткой.
    assert_eq!(
        (height(14) - CLOSED_BOX - 2) / ROW,
        OWNERS_CEILING,
        "четырнадцать языков обязаны показать ровно восемь строк — это то, что владелец \
         принимает глазом"
    );

    // И потолок обязан быть осмысленным: не ниже самого короткого списка программы (тема — 3
    // строки, иначе потолок резал бы даже её) и не выше самого длинного (14 языков, иначе он
    // не потолок вовсе).
    assert!(
        (3..14).contains(&OWNERS_CEILING),
        "потолок обязан лежать между самым коротким списком программы и самым длинным — \
         вышло {OWNERS_CEILING}"
    );
}

/// **И что окна в самом деле ходят этой дорогой** — правило 118.1 живёт в общем слое, но
/// пользы от него нет, пока его никто не зовёт.
///
/// ⚠ Сторож смотрит на **имя вызова**, а не на строку целиком: имя — это и есть договор между
/// окном и слоем, и переставленные вокруг него строки его не ломают.
#[test]
fn every_window_asks_the_shared_layer_for_the_dropped_height() {
    {
        let (module, what) = ("letters.rs", "мастер FR-104 — поле раскладки шага 3");
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join(module),
        )
        .unwrap_or_else(|error| panic!("src\\{module} must be readable: {error}"));

        assert!(
            source.contains("combo::set_dropped_height("),
            "{what}: обязано спрашивать раскрытую высоту у общего слоя, иначе правило 118.1 \
             живёт в одном окне из двух"
        );
    }

    // ⛔ **И ни одно окно не заводит своей арифметики** — запрет ТЗ Э53а: весь расчёт в
    // `widgets::combo`. Потолок назван там, и больше нигде.
    {
        let module = "letters.rs";
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join(module),
        )
        .unwrap_or_else(|error| panic!("src\\{module} must be readable: {error}"));

        assert!(
            !source.contains("LIST_CEILING"),
            "src\\{module}: потолок раскрытого списка обязан быть известен только общему слою"
        );
    }
}

#[test]
fn the_closed_part_of_a_combo_box_is_the_same_height_in_every_window() {
    // Высоты шрифта диалога, измеренные стендом на этой машине: мастер несёт 9 pt, окно
    // настроек — 10 pt, и одна кегельная строка выходит 15 и 17 пикселей соответственно
    // (прибор печатает их как «окно контрола» однострочного поля).
    const WIZARD_FONT: i32 = 15;
    const SETTINGS_FONT: i32 = 17;

    // Тело `widgets::combo::closed_height`, повторённое арифметикой.
    let closed = |font: i32| {
        let by_font = font + theme::scaled(lang_switcher::widgets::combo::CLOSED_ITEM_EXTRA, 96);
        let by_mockup = theme::scaled(lang_switcher::widgets::combo::CLOSED_BOX, 96);
        by_font.max(by_mockup)
    };

    let wizard = closed(WIZARD_FONT);
    let settings = closed(SETTINGS_FONT);

    println!("закрытая часть при 96 DPI: мастер {wizard} px, окно настроек {settings} px");

    assert_eq!(
        wizard, settings,
        "закрытая часть комбобокса обязана быть одной и той же во всех окнах программы, а \
         вышло {wizard} против {settings} — мерка снова зависит от шрифта окна (решение 110.3)"
    );

    // ⭐ И это ровно то число, которое окно настроек имело всегда: эталон механики (решение
    // 109.1) не двигается, мастер встаёт на него.
    assert_eq!(
        settings,
        SETTINGS_FONT + theme::scaled(lang_switcher::widgets::combo::CLOSED_ITEM_EXTRA, 96),
        "окно настроек обязано остаться при своей прежней закрытой части — это эталон, а не \
         предмет правки"
    );
    assert_eq!(settings, 18, "и она равна 18 px при 96 DPI — замер стенда");

    // Пол по шрифту не декорация: он держит слово внутри закрытой части, какой бы крупный
    // шрифт окно ни несло. Макетная мерка одна такой гарантии не даёт.
    let huge = closed(40);
    assert!(
        huge > theme::scaled(lang_switcher::widgets::combo::CLOSED_BOX, 96),
        "при крупном шрифте закрытая часть обязана подниматься выше макетной мерки, иначе \
         слово в неё не влезет — вышло {huge}"
    );
    assert_eq!(
        huge,
        40 + theme::scaled(lang_switcher::widgets::combo::CLOSED_ITEM_EXTRA, 96)
    );

    // ---- и что окно в самом деле ходит этой дорогой --------------------------------------
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("widgets.rs"),
    )
    .expect("src/widgets.rs must be readable")
    .replace("\r\n", "\n");

    // ⚠ **Задача T-50-2 вынесла общий хвост в `mockup_pixels`** — до неё блок `GetDC`/`ReleaseDC`
    // стоял тут же двумя строками, и когда строке списка понадобилась та же макетная мерка, он
    // стал бы вторым таким же блоком с собственным `SAFETY`. Правило, о котором этот сторож,
    // не изменилось: бо́льшая из шрифтовой мерки и макетной.
    for part in [
        "pub const CLOSED_BOX: i32 = 25;",
        "let Some(height) = closed_height(hwnd) else {",
        "let by_font = row_height(hwnd, CLOSED_ITEM_EXTRA)?;",
        "Some(by_font.max(mockup_pixels(hwnd, CLOSED_BOX)))",
    ] {
        assert!(
            source.contains(part),
            "слой обязан нести `{part}` — иначе правило 110.3 держится только этим тестом"
        );
    }
}

// -----------------------------------------------------------------------------------------
// The about dialog — FR-92а, task T-11-11, read out of the built binary like everything else
// -----------------------------------------------------------------------------------------

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
    // Stage E114, task T-114-2: the sixth row could not join them — 3072 is the menu's — and takes
    // the last free number of the block the wizard opened, 3248..3263.
    assert_eq!(settings::IDS_ABOUT_HELP_6, 3263);
    assert_eq!(
        settings::IDS_MENU_SUSPEND,
        3072,
        "the menu block starts at 3072 and the help must stop in front of it"
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
    // A key this build knows is shown by its name.
    assert_eq!(settings::effective_hotkey_name(&bare("Pause")), "Pause");
    assert_eq!(
        settings::effective_hotkey_name(&bare("ScrollLock")),
        "ScrollLock"
    );
    assert_eq!(settings::effective_hotkey_name(&bare("F9")), "F9");

    // A name nobody knows falls back to the default of section 7 — the key that really acts.
    assert_eq!(
        settings::effective_hotkey_name(&bare("Хрюкозябра")),
        "Pause"
    );
    assert_eq!(settings::effective_hotkey_name(&bare("")), "Pause");

    // ⚠ A **text** key is deliberately not replaced: `Q` really is the hotkey when the file
    // says so — FR-95 warns about it in the settings window, and the help telling the truth
    // about the user's own configuration is the point.
    assert_eq!(settings::effective_hotkey_name(&bare("Q")), "Q");

    // And the fallback is the default of section 7 itself, not a second copy of the word.
    assert_eq!(
        Config::default().hotkey.key,
        settings::effective_hotkey_name(&bare("нет такой клавиши")),
        "the fallback of the help is the default of section 7"
    );
}

/// A hotkey with no modifiers — `[hotkey]` as every file before schema 7 has it (task T-93-1).
fn bare(key: &str) -> settings::Hotkey {
    settings::Hotkey {
        key: key.to_owned(),
        modifiers: Vec::new(),
        trigger: settings::TRIGGER_PRESS.to_owned(),
    }
}

/// A hotkey with modifiers, named the way `[hotkey] modifiers` names them (task T-93-1).
fn combination(modifiers: &[&str], key: &str) -> settings::Hotkey {
    settings::Hotkey {
        key: key.to_owned(),
        modifiers: modifiers.iter().map(|name| (*name).to_owned()).collect(),
        trigger: settings::TRIGGER_PRESS.to_owned(),
    }
}

// -----------------------------------------------------------------------------------------
// Task T-93-1, вопрос 157 — `[hotkey] modifiers`: the file, the name, the schema
// -----------------------------------------------------------------------------------------

/// **The name of a combination is one string, in the canonical order** — task T-93-1, вопрос
/// 157: `Ctrl + Alt + Shift + F12`, the modifiers `Ctrl`, `Alt`, `Shift` whatever order the file
/// lists them in, « + » between, and the key under the name the program writes. The field of the
/// settings window, the help of «О программе» and the letters all show this one string.
#[test]
fn the_name_of_a_combination_is_one_string_in_the_canonical_order() {
    assert_eq!(
        settings::effective_hotkey_name(&combination(&["Ctrl"], "F12")),
        "Ctrl + F12"
    );
    assert_eq!(
        settings::effective_hotkey_name(&combination(&["Shift", "Alt", "Ctrl"], "F12")),
        "Ctrl + Alt + Shift + F12",
        "the order is the program's, not the file's"
    );
    assert_eq!(
        settings::effective_hotkey_name(&combination(&["shift", " ctrl "], "f9")),
        "Ctrl + Shift + F9",
        "a hand-written file is shown in the spelling the program writes"
    );
    assert_eq!(
        settings::effective_hotkey_name(&combination(&["Alt"], "Break")),
        "Alt + Pause",
        "the key is named by `key_name`, the writer's spelling of its code"
    );
    assert_eq!(
        settings::effective_hotkey_name(&bare("F12")),
        "F12",
        "a bare key is its name alone"
    );
}

/// **A modifier nobody knows leaves the whole hotkey at the default** — task T-93-1: the fate of
/// an unknown key, `Pause`, and the same note under the field. A combination with one of its
/// modifiers dropped would be another key than the one written.
#[test]
fn an_unknown_modifier_leaves_the_whole_hotkey_at_the_default() {
    for unknown in [
        combination(&["Win"], "F12"),
        combination(&["Ctrl", "Strg"], "F12"),
        combination(&[""], "F12"),
    ] {
        assert_eq!(unknown.binding(), None, "{unknown:?}");
        assert_eq!(
            settings::effective_hotkey_name(&unknown),
            "Pause",
            "{unknown:?}: the help names the key that really acts"
        );
        assert_eq!(
            settings::hotkey_note(&unknown),
            Some(settings::IDS_NOTE_UNKNOWN_KEY),
            "{unknown:?}: and the settings window says why"
        );
    }

    assert_eq!(
        combination(&["Ctrl"], "F12").binding(),
        Some((0x7B, hook::MOD_CTRL)),
        "the control: a known pair is read"
    );
    assert_eq!(bare("F12").binding(), Some((0x7B, 0)));
}

/// **A letter with a modifier is not warned about as a text key** — task T-93-1: the warning of
/// FR-92 says the key will stop typing its character, and with `Ctrl` held it types none; the
/// bare `K` stays with the programs.
#[test]
fn a_letter_with_a_command_modifier_is_not_a_text_key_warning() {
    assert_eq!(
        settings::hotkey_note(&bare("K")),
        Some(settings::IDS_NOTE_TEXT_KEY)
    );
    assert_eq!(settings::hotkey_note(&combination(&["Ctrl"], "K")), None);
}

/// **`[hotkey] modifiers` is written in the canonical order and read back to the same set** —
/// task T-93-1. What the program writes for a set is `Hotkey::from_binding` — the names of
/// `hook::HOTKEY_MODIFIERS` in their order, whatever order the bits came in — and the file reads
/// back to the very code and set it came from. An empty list is the bare key, and a file that
/// does not mention `modifiers` at all — every file before schema 7 — reads as one.
#[test]
fn the_modifiers_are_written_in_the_canonical_order_and_read_back() {
    let all = hook::MOD_SHIFT | hook::MOD_CTRL | hook::MOD_ALT;
    let written = settings::Hotkey::from_binding(0x7B, all).expect("F12 has a name");

    assert_eq!(written, combination(&["Ctrl", "Alt", "Shift"], "F12"));

    let config = Config {
        hotkey: written.clone(),
        ..Config::default()
    };

    let text = config.to_toml_string().expect("the file must serialise");
    println!("{text}");

    assert!(
        text.contains("modifiers = [\"Ctrl\", \"Alt\", \"Shift\"]"),
        "the list is in the file in the canonical order: {text}"
    );

    let (back, outcome) = Config::from_toml_str(&text).expect("the written file must parse");

    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(back.hotkey, written);
    assert_eq!(back.hotkey.binding(), Some((0x7B, all)));

    // The bare key: an empty list in the file, and the absent field of an older one.
    let defaults = Config::default()
        .to_toml_string()
        .expect("the defaults must serialise");
    assert!(
        defaults.contains("modifiers = []"),
        "the default writes the empty list: {defaults}"
    );

    let older = Config::from_toml_str(&format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[hotkey]\nkey = \"F9\"\n"
    ))
    .expect("a file without the list must parse")
    .0;
    assert_eq!(older.hotkey, bare("F9"));
    assert_eq!(older.hotkey.binding(), Some((0x78, 0)));

    // A set with a bit no hotkey carries is not written at all.
    assert_eq!(
        settings::Hotkey::from_binding(0x7B, hook::MOD_CTRL | hook::MOD_WIN),
        None
    );
    assert_eq!(
        settings::Hotkey::from_binding(0x08, 0),
        None,
        "Backspace has no name"
    );
}

/// The last schema a build that had never heard of `[hotkey] modifiers` could have stamped — task
/// T-93-1, вопрос 157.
///
/// `deny_unknown_fields` is in no struct of this module (the rule of section 7: unknown fields
/// are ignored), so such a build reads a file of the combinations without a word and drops the
/// list. Under the same stamp it would also **write** that file back — on «Применить», on the
/// letters' date — and the person's `Ctrl+F12` would be a bare `F12` after the next update. The
/// stamp it has never seen makes it read the file and leave it alone instead.
const LAST_SCHEMA_WITHOUT_MODIFIERS: u32 = 6;

/// **Task T-93-1 — the stamp of the modifiers: a build too old for them reads the file and never
/// writes it back.** The shape of `a_downgrade_meets_the_new_stamp_and_refuses_to_write_rather_than_quarantining`
/// (Э29), for a new **field** rather than a new value.
///
/// 1. What this build writes for a combination carries the list and a stamp no build before the
///    modifiers had — and the const block makes lowering the schema a build that does not compile.
/// 2. A reader too old for the stamp, modelled by this build meeting `CURRENT + 1` and a field it
///    does not know in `[hotkey]` (a test may not install an older build): the file is read,
///    `Forbidden` to be written, and its bytes stay exactly where they were.
/// 3. The control — what the stamp prevents: the same unknown field under the stamp this build
///    *does* know is read as current, and the next write drops it. That is what a build of schema
///    6 would do to `modifiers` if the schema had stayed 6.
#[test]
fn a_build_too_old_for_the_modifiers_reads_the_file_and_never_writes_it_back() {
    // --- 1. what this build writes for a combination ------------------------------------------
    let ours = Config {
        hotkey: combination(&["Ctrl"], "F12"),
        ..Config::default()
    };

    let text = ours.to_toml_string().expect("the file must serialise");

    assert!(
        text.contains("modifiers = [\"Ctrl\"]"),
        "the file carries the list it was given: {text}"
    );
    const {
        assert!(
            CURRENT_SCHEMA_VERSION > LAST_SCHEMA_WITHOUT_MODIFIERS,
            "a file of the combinations must be stamped with a schema no build before them had — \
             without it such a build reads the file as its own and drops the list on its next write"
        );
    }
    assert!(
        text.contains(&format!("schema_version = {CURRENT_SCHEMA_VERSION}")),
        "and the stamp is in the file, not merely in memory: {text}"
    );

    // --- 2. the reader too old for the stamp --------------------------------------------------
    let dir = TestDir::new("modifiers_downgrade_meets_the_stamp");
    let newer = CURRENT_SCHEMA_VERSION + 1;
    // ⚠ Task T-95-1: the field this build does not know was `trigger` until schema 8 made it
    // known (вопрос 159); any field outside section 7 serves, and the example is another one.
    let original = format!(
        "schema_version = {newer}\n\
         \n\
         [hotkey]\n\
         key = \"F12\"\n\
         repeat = \"never\"\n"
    );
    let path = write_file(&dir, &original);

    let (config, outcome) = settings::read_or_default(&path);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::FromNewerSchema { version: newer }),
        "the stamp decides"
    );
    assert_eq!(
        SavePolicy::for_read(&outcome),
        SavePolicy::Forbidden,
        "and a file from the future is never written back"
    );
    assert_eq!(
        fs::read_to_string(&path).expect("the file must still be readable"),
        original,
        "the bytes — the field this build does not know among them — stay where they were"
    );
    assert_eq!(
        dir.entries(),
        [CONFIG_FILE_NAME],
        "and nothing was made beside them"
    );
    assert_eq!(config.hotkey.key, "F12", "what can be read is read");

    // --- 3. the control: the same field under a stamp this build knows is lost on the write ---
    let (current, outcome) = Config::from_toml_str(&format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[hotkey]\nkey = \"F12\"\nrepeat = \"never\"\n"
    ))
    .expect("an unknown field is ignored, not refused");

    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(
        SavePolicy::for_read(&Ok::<_, ConfigError>(outcome)),
        SavePolicy::Allowed
    );

    let rewritten = current.to_toml_string().expect("the file must serialise");
    assert!(
        !rewritten.contains("repeat"),
        "a field the schema does not know does not survive a write — which is what the stamp \
         keeps an older build from doing to `modifiers`: {rewritten}"
    );
}

/// **The rung of the modifiers — task T-93-1: a schema 6 file climbs to 7, and its hotkey is the
/// bare key it always was.** `step_6_to_7` is a stamp: an absent `modifiers` already means the
/// empty list, which is what a file of schema 6 meant by its hotkey. The other fields are
/// deliberately not the defaults, so that damage would show.
#[test]
fn a_file_of_schema_six_is_raised_to_seven_and_its_hotkey_is_the_bare_key() {
    let dir = TestDir::new("modifiers_schema_migration");
    let path = write_file(
        &dir,
        "schema_version = 6\n\
         \n\
         [general]\n\
         enabled = false\n\
         language = \"he\"\n\
         \n\
         [hotkey]\n\
         key = \"F9\"\n\
         \n\
         [letters]\n\
         welcome_shown = true\n\
         last_seen_version = \"0.91.0\"\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("a schema 6 file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 6 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    // ⚠ Task T-95-1: the file climbs on past seven to the stamp of `[hotkey] trigger`, which
    // carries nothing of the hotkey but its default `press` — what this rung decided arrives at
    // eight untouched.
    assert_eq!(config.schema_version, 10, "and that version is ten");

    assert_eq!(
        config.hotkey,
        bare("F9"),
        "the hotkey is the bare key it was"
    );
    assert!(
        !config.general.enabled,
        "the rung carried the switch through"
    );
    assert_eq!(config.general.language, Language::He);
    assert!(config.letters.welcome_shown, "and the letters' memory");
    assert_eq!(config.letters.last_seen_version, "0.91.0");
}

// -----------------------------------------------------------------------------------------
// Task T-95-1, вопрос 159 — `[hotkey] trigger`: the double press of `Shift` in the file
// -----------------------------------------------------------------------------------------

/// A `[hotkey]` written out whole — the key, the modifiers and the trigger (task T-95-1).
fn hotkey_of(key: &str, modifiers: &[&str], trigger: &str) -> settings::Hotkey {
    settings::Hotkey {
        key: key.to_owned(),
        modifiers: modifiers.iter().map(|name| (*name).to_owned()).collect(),
        trigger: trigger.to_owned(),
    }
}

/// **The double press of `Shift` is written, read back, published as `Shift` with the flag, and
/// named «Shift, Shift»** — task T-95-1, §0 п. 4–5 of the TZ. ⭐ A red «before» of the task: on the
/// base the build knows no name `Shift`, and the file means `Pause`.
#[test]
fn the_double_press_is_written_read_back_and_published_as_shift_with_the_flag() {
    let double = settings::Hotkey::double_shift();
    assert_eq!(double, hotkey_of("Shift", &[], "double_tap"));

    let config = Config {
        hotkey: double.clone(),
        ..Config::default()
    };
    let text = config.to_toml_string().expect("the file must serialise");
    println!("{text}");

    for line in [
        "key = \"Shift\"",
        "modifiers = []",
        "trigger = \"double_tap\"",
    ] {
        assert!(text.contains(line), "the file carries `{line}`: {text}");
    }

    let (back, outcome) = Config::from_toml_str(&text).expect("the written file must parse");

    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(back.hotkey, double);
    assert_eq!(
        back.hotkey.binding(),
        Some((0x10, hook::DOUBLE_TAP)),
        "the start-up publishes VK_SHIFT with the flag (app::publish_configuration)"
    );
    assert_eq!(
        settings::Hotkey::from_binding(0x10, hook::DOUBLE_TAP),
        Some(double.clone()),
        "and the flag reads back to the same section"
    );
    assert_eq!(settings::effective_hotkey_name(&double), "Shift, Shift");
    assert_eq!(
        settings::hotkey_note(&double),
        None,
        "nothing to warn about"
    );

    // Written by hand, in any case and with blanks — the rule `hook::vk_from_name` reads names by.
    let by_hand = hotkey_of(" shift ", &[], "Double_Tap");
    assert_eq!(by_hand.binding(), Some((0x10, hook::DOUBLE_TAP)));
    assert_eq!(settings::effective_hotkey_name(&by_hand), "Shift, Shift");
}

/// **`trigger` is `press` by default and always written** — task T-95-1: every file before schema
/// 8 has no such field, and its hotkey is pressed; a capture of a key or a combination writes
/// `press` too.
#[test]
fn the_trigger_is_press_by_default_and_always_written() {
    let defaults = Config::default()
        .to_toml_string()
        .expect("the defaults must serialise");
    assert!(
        defaults.contains("trigger = \"press\""),
        "the default writes the trigger: {defaults}"
    );
    assert_eq!(Config::default().hotkey.trigger, settings::TRIGGER_PRESS);

    let older = Config::from_toml_str(&format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[hotkey]\nkey = \"F9\"\n"
    ))
    .expect("a file without the trigger must parse")
    .0;
    assert_eq!(older.hotkey, bare("F9"));
    assert_eq!(older.hotkey.binding(), Some((0x78, 0)));

    assert_eq!(
        settings::Hotkey::from_binding(0x7B, hook::MOD_CTRL).map(|hotkey| hotkey.trigger),
        Some(settings::TRIGGER_PRESS.to_owned()),
        "a combination is pressed"
    );
}

/// **The double press is lawful with `Shift` alone; anything else leaves the default `Pause`** —
/// task T-95-1, §0 п. 4: the fate of an unknown name (stage 1). `Ctrl` and `Alt` twice are not
/// assigned (157.17), a single `Shift` is no hotkey (157.1), and a word outside the closed set of
/// `trigger` is no trigger.
#[test]
fn a_double_press_of_anything_but_shift_alone_leaves_the_default() {
    for unlawful in [
        hotkey_of("F12", &[], "double_tap"),
        hotkey_of("Ctrl", &[], "double_tap"),
        hotkey_of("Alt", &[], "double_tap"),
        hotkey_of("Shift", &["Ctrl"], "double_tap"),
        hotkey_of("Shift", &[], "press"),
        hotkey_of("Shift", &[], "triple_tap"),
        hotkey_of("Pause", &[], "hold"),
        hotkey_of("Pause", &[], ""),
    ] {
        assert_eq!(unlawful.binding(), None, "{unlawful:?}");
        assert_eq!(
            settings::effective_hotkey_name(&unlawful),
            "Pause",
            "{unlawful:?}: the help names the key that really acts"
        );
        assert_eq!(
            settings::hotkey_note(&unlawful),
            Some(settings::IDS_NOTE_UNKNOWN_KEY),
            "{unlawful:?}: and the settings window says why"
        );
    }

    // The controls: the lawful two.
    assert_eq!(hotkey_of("Pause", &[], "press").binding(), Some((0x13, 0)));
    assert_eq!(
        hotkey_of("Shift", &[], "double_tap").binding(),
        Some((0x10, hook::DOUBLE_TAP))
    );

    // And nothing but `Shift` alone reads back out of the flag.
    assert_eq!(
        settings::Hotkey::from_binding(0x10, hook::DOUBLE_TAP | hook::MOD_CTRL),
        None
    );
    assert_eq!(settings::Hotkey::from_binding(0x7B, hook::DOUBLE_TAP), None);
}

/// The last schema a build that had never heard of `[hotkey] trigger` could have stamped — task
/// T-95-1, вопрос 159: `0.93.0` and every build before it.
const LAST_SCHEMA_WITHOUT_TRIGGER: u32 = 7;

/// **Task T-95-1 — the stamp of the trigger: a build too old for it reads the file and never writes
/// it back** — the shape of `a_downgrade_meets_the_new_stamp_and_refuses_to_write_rather_than_quarantining`
/// (Э29), for a new field as at schema 7.
///
/// 1. What this build writes for the double press carries the trigger and a stamp no build before
///    it had — and the const block makes lowering the schema a build that does not compile.
/// 2. A reader too old for the stamp, modelled by this build meeting `CURRENT + 1` and a field it
///    does not know: the file is read, `Forbidden` to be written, and its bytes stay put. (The real
///    `0.93.0` was measured on the base by a probe — посылка П4: `FromNewerSchema { version: 8 }`,
///    the hotkey `Pause`, the bytes untouched — `scratchpad-E95\premises-base-618ef27.log`.)
/// 3. The control — what the stamp prevents: the same unknown field under the stamp this build
///    knows is read as current, and the next write drops it. That is what `0.93.0` would do to
///    `trigger` if the schema had stayed 7.
#[test]
fn a_downgrade_meets_the_stamp_of_the_trigger_and_refuses_to_write_rather_than_dropping_it() {
    // --- 1. what this build writes for the double press ---------------------------------------
    let ours = Config {
        hotkey: settings::Hotkey::double_shift(),
        ..Config::default()
    };
    let text = ours.to_toml_string().expect("the file must serialise");

    assert!(text.contains("trigger = \"double_tap\""), "{text}");
    const {
        assert!(
            CURRENT_SCHEMA_VERSION > LAST_SCHEMA_WITHOUT_TRIGGER,
            "a file of the double press must be stamped with a schema no build before it had — \
             without it such a build reads the file as its own and drops the trigger on its next write"
        );
    }
    assert!(
        text.contains(&format!("schema_version = {CURRENT_SCHEMA_VERSION}")),
        "and the stamp is in the file: {text}"
    );

    // --- 2. the reader too old for the stamp --------------------------------------------------
    let dir = TestDir::new("trigger_downgrade_meets_the_stamp");
    let newer = CURRENT_SCHEMA_VERSION + 1;
    let original = format!(
        "schema_version = {newer}\n\
         \n\
         [hotkey]\n\
         key = \"Shift\"\n\
         modifiers = []\n\
         trigger = \"double_tap\"\n\
         repeat = \"never\"\n"
    );
    let path = write_file(&dir, &original);

    let (config, outcome) = settings::read_or_default(&path);

    assert_eq!(
        outcome.as_ref().ok(),
        Some(&ReadOutcome::FromNewerSchema { version: newer })
    );
    assert_eq!(SavePolicy::for_read(&outcome), SavePolicy::Forbidden);
    assert_eq!(
        fs::read_to_string(&path).expect("the file must still be readable"),
        original,
        "the bytes stay where they were"
    );
    assert_eq!(dir.entries(), [CONFIG_FILE_NAME]);
    assert_eq!(config.hotkey, settings::Hotkey::double_shift());

    // --- 3. the control: an unknown field under the stamp this build knows is lost on the write --
    let (current, outcome) = Config::from_toml_str(&format!(
        "schema_version = {CURRENT_SCHEMA_VERSION}\n\n[hotkey]\nkey = \"Shift\"\nrepeat = \"never\"\n"
    ))
    .expect("an unknown field is ignored, not refused");

    assert_eq!(outcome, ReadOutcome::Current);
    let rewritten = current.to_toml_string().expect("the file must serialise");
    assert!(
        !rewritten.contains("repeat"),
        "a field the schema does not know does not survive a write — which is what the stamp \
         keeps `0.93.0` from doing to `trigger`: {rewritten}"
    );
}

/// **The rung of the trigger — task T-95-1: a schema 7 file climbs to 8, and its hotkey is
/// pressed.** `step_7_to_8` is a stamp: an absent `trigger` already means `press`, which is what a
/// file of schema 7 meant by its hotkey. The other fields are deliberately not the defaults, so
/// that damage would show.
#[test]
fn a_file_of_schema_seven_is_raised_to_eight_and_its_hotkey_is_pressed() {
    let dir = TestDir::new("trigger_schema_migration");
    let path = write_file(
        &dir,
        "schema_version = 7\n\
         \n\
         [general]\n\
         enabled = false\n\
         language = \"he\"\n\
         \n\
         [hotkey]\n\
         key = \"F12\"\n\
         modifiers = [\"Ctrl\"]\n\
         \n\
         [letters]\n\
         welcome_shown = true\n\
         last_seen_version = \"0.93.0\"\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("a schema 7 file must be read");

    assert_eq!(outcome, ReadOutcome::Migrated { from: 7 });
    assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(config.schema_version, 10, "and that version is ten");

    assert_eq!(config.hotkey, combination(&["Ctrl"], "F12"));
    assert_eq!(config.hotkey.trigger, settings::TRIGGER_PRESS);
    assert!(
        !config.general.enabled,
        "the rung carried the switch through"
    );
    assert_eq!(config.general.language, Language::He);
    assert!(config.letters.welcome_shown, "and the letters' memory");
    assert_eq!(config.letters.last_seen_version, "0.93.0");
}

/// **Задача Т-33а-4** — пределы оболочки для шара уведомления, по всем четырнадцати языкам.
///
/// `szInfoTitle` держит 63 единицы UTF-16, `szInfo` — 255, и обрезание у оболочки **молчаливое**.
/// Этот проект на молчаливом обрезании уже обжигался — текст контрола на 511-м знаке, Э32, — и
/// мерить два языка из четырнадцати значит не мерить ничего.
///
/// ⚠ Замок `with_product_strings` обязателен: язык интерфейса — величина процесса, и перебор
/// четырнадцати языков без него сбивал бы соседние тесты, читающие слова.
#[test]
fn every_balloon_string_fits_the_shell_in_all_fourteen_languages() {
    use lang_switcher::letters::{self, Letter};

    const TITLE_LIMIT: usize = 63;
    const BODY_LIMIT: usize = 255;

    let _guard = with_product_strings();

    let mut worst_title = 0;
    let mut worst_body = 0;

    for language in Language::shipped() {
        settings::set_ui_language(language);

        for letter in [Letter::Update, Letter::News(1), Letter::Thanks] {
            let Some((title_id, body_id)) = letters::toast_strings(letter) else {
                continue;
            };

            // Заголовок с подставленной версией — самое длинное, что туда попадает.
            let title = settings::format_text(title_id, &["10.10.10"]);
            let body = settings::text(body_id);

            let title_units = title.encode_utf16().count();
            let body_units = body.encode_utf16().count();

            worst_title = worst_title.max(title_units);
            worst_body = worst_body.max(body_units);

            assert!(
                title_units <= TITLE_LIMIT,
                "{language:?} {letter:?}: заголовок {title_units} единиц при пределе \
                 {TITLE_LIMIT} — оболочка обрежет молча: «{title}»"
            );
            assert!(
                body_units <= BODY_LIMIT,
                "{language:?} {letter:?}: тело {body_units} единиц при пределе {BODY_LIMIT} — \
                 оболочка обрежет молча: «{body}»"
            );
        }
    }

    println!(
        "самый длинный заголовок {worst_title} из {TITLE_LIMIT}, тело {worst_body} из {BODY_LIMIT}"
    );

    // Контроль прибора: измеренные строки не пусты. Ноль означал бы, что ресурс не открылся, и
    // «влезает» тогда значило бы «мерить было нечего».
    assert!(
        worst_title > 0 && worst_body > 0,
        "строки не загрузились: прибор мерил пустоту"
    );

    settings::set_ui_language(Language::Ru);
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
            settings::IDS_ABOUT_HELP_6,
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
            // Stage E114, task T-114-2: the sixth row names the key again — «нажмите {0}».
            let carries = index < 3 || index == 5;

            assert_eq!(
                row.contains("ScrollLock"),
                carries,
                "row {index} must {} the key name: «{row}»",
                if carries { "carry" } else { "not carry" }
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

/// **FR-92а** — the line over the title of «О программе» is the name of the program and its
/// version: the only window that shows the number (passport §2), the same in every language. The
/// number is the one the file that ships declares in `VERSIONINFO`, in three parts; the title and
/// the line under it are the two lines of the description.
#[test]
fn the_line_over_the_title_of_about_is_the_name_and_the_version_of_the_file() {
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::scene::Node;
    use lang_switcher::ui::windows::about::About;
    use windows::Win32::UI::WindowsAndMessaging::RT_VERSION;

    let _guard = with_product_strings();
    let product = ProductImage::open();
    let (major, minor, build, _) =
        lang_switcher::tray::parse_fixed_file_version(&product.resource(RT_VERSION, 1))
            .expect("the product declares its version");
    let wanted = format!("Lang Switcher {major}.{minor}.{build}");

    for language in [Language::Ru, Language::En, Language::He] {
        let about = About {
            language,
            key: "Pause".to_owned(),
            theme: ThemeSetting::Light,
        };
        let window = about.describe();
        let Node::Column(nodes) = &window.body else {
            panic!("the body of «О программе» is a column");
        };
        let Some(Node::Header {
            eyebrow,
            title,
            subtitle,
        }) = nodes.first()
        else {
            panic!("the column opens with the head of the window");
        };

        println!("{language:?}: «{eyebrow:?}» / «{title}» / «{subtitle:?}»");

        assert_eq!(eyebrow.as_deref(), Some(wanted.as_str()), "{language:?}");
        assert_eq!(
            title,
            &settings::text_in(language, settings::IDS_ABOUT_LINE_1),
            "{language:?}: the title is the first line of the description"
        );
        assert_eq!(
            subtitle.as_deref(),
            Some(settings::text_in(language, settings::IDS_ABOUT_LINE_2).as_str()),
            "{language:?}: the line under it is the second"
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
// Task T-55-8 — findings С54 and С55: an «ОК» without edits does not rewrite the cycle
// -----------------------------------------------------------------------------------------

/// **Finding С54 — an «ОК» on a session that named no layouts keeps the cycle of the file.** Until
/// task T-55-8 the list was built from what the session named right then, and «ОК» wrote back
/// exactly what was ticked: a session that named nothing — the enumeration refused, the layouts
/// were being reinstalled — erased the cycle without a single edit (premise П9(а): two entries in,
/// none out).
#[test]
fn an_ok_on_a_session_that_named_no_layouts_keeps_the_cycle_of_the_file() {
    let file = vec!["0x00000419".to_owned(), "0x00000409".to_owned()];
    let mut layouts = Config::default().layouts;
    layouts.cycle = file.clone();

    let at_open = settings::layout_rows(&layouts, &[]);
    let written = settings::cycle_after_dialog(&file, &at_open, &at_open, &[]);

    assert_eq!(
        written, file,
        "С54: the cycle of the file is written back as it was, entry for entry"
    );
}

/// **Finding С55 — an entry in the language form is not lengthened by an «ОК» without edits.**
/// `0x00000409` names every layout of the language 0x0409; with two of them in the session the
/// list ticks both, and until task T-55-8 «ОК» wrote both back in the handle form — the cycle grew
/// and its order changed (premise П9(б): one entry in, two out). The file's string whose ticks
/// nobody touched goes back byte for byte.
#[test]
fn an_entry_in_the_language_form_is_not_lengthened_by_an_ok_without_edits() {
    let session = [EN, EN_SECOND, RU];
    let file = vec!["0x00000409".to_owned()];
    let mut layouts = Config::default().layouts;
    layouts.cycle = file.clone();

    let at_open = settings::layout_rows(&layouts, &session);
    let written = settings::cycle_after_dialog(&file, &at_open, &at_open, &session);

    println!("rows at open: {at_open:?}, written: {written:?}");

    assert_eq!(
        written, file,
        "С55: one entry in, one entry out, in its own form"
    );
}

/// **Task T-42-6, п. 5** — a tick the gate refused is **not an edit**: the cycle line of the
/// file goes back byte for byte, which is exactly what T-55-8 bought.
///
/// The refusal of Н112 happens at `LVN_ITEMCHANGING`, before the change goes in, so the rows the
/// dialog closes with are the rows it opened with and `cycle_after_dialog` keeps the file's own
/// string. Said here as a measurement rather than as a sentence in a comment: a future gate that
/// refused the tick *after* letting it into the rows would be caught right here.
#[test]
fn a_refused_tick_leaves_the_cycle_line_of_the_file_untouched() {
    let session = [EN, RU];
    let file = vec!["0x00000409".to_owned()];
    let mut layouts = Config::default().layouts;
    layouts.cycle = file.clone();

    let at_open = settings::layout_rows(&layouts, &session);

    // The rows after a refused tick are the rows at open — the tick never went in.
    let written = settings::cycle_after_dialog(&file, &at_open, &at_open, &session);

    println!("rows at open: {at_open:?}, written: {written:?}");

    assert_eq!(
        written, file,
        "a refused tick is no edit: the file's own line stands byte for byte (T-55-8)"
    );
}

/// **Task T-42-7, finding Н29** — the exclusion list says when it is full, and refuses a name
/// that could never match a process.
///
/// # What the window did before
///
/// It took the thirty-third name and the name `C:\` alike, cleared the field, and showed the
/// name in the list. `guard::publish_exclusions` then dropped both on «ОК» — counting them into
/// `Counters::exclusions_refused`, which the dump prints and nobody reads at that moment — and
/// the exclusion the person asked for was quietly not there. The counter counts **on
/// publication**; the window has to answer **before** it.
///
/// # What it does now
///
/// The ceiling is shown rather than enforced after the fact: at `guard::MAX_EXCLUSIONS` names
/// the field and «Добавить» go dark (вариант 1 «заглушить ввод на потолке»), and removing a name
/// lights them again. A name whose fold is empty — `C:\`, `\`, `..\`, a run of spaces — is
/// refused **aloud**, by the same door T-42-6 opened for the ninth tick.
#[test]
fn the_exclusion_list_shows_its_ceiling_and_refuses_a_name_that_folds_to_nothing() {
    // 1. The ceiling, читаемый из `guard`, а не переписанный сюда числом.
    for count in 0..lang_switcher::guard::MAX_EXCLUSIONS {
        assert!(
            !settings::exclusions_are_full(count),
            "with {count} names the list still takes one more"
        );
    }

    assert!(
        settings::exclusions_are_full(lang_switcher::guard::MAX_EXCLUSIONS),
        "at {} names the list is full and the field goes dark",
        lang_switcher::guard::MAX_EXCLUSIONS
    );

    // 2. The verdict on one name.
    let existing = vec!["game.exe".to_owned(), "Setup.EXE".to_owned()];

    assert_eq!(
        settings::exclusion_verdict("notepad.exe", &existing),
        settings::ExclusionVerdict::Add,
        "a new name goes in"
    );

    // The fold is the last component, lower-cased — so these three are the same exclusion as
    // one already in the list, and none of them is a refusal: the list is already as asked.
    for duplicate in [r"C:\Games\game.exe", "GAME.EXE", "  setup.exe  "] {
        assert_eq!(
            settings::exclusion_verdict(duplicate, &existing),
            settings::ExclusionVerdict::Duplicate,
            "«{duplicate}» folds onto a name the list already holds"
        );
    }

    // ⛔ The empty fold — the case `guard` counted and the window swallowed. A path with no
    // last component matches no process at all, and the program would have dropped it on «ОК».
    for nothing in [r"C:\", "\\", r"..\", "   ", "/"] {
        assert_eq!(
            settings::exclusion_verdict(nothing, &existing),
            settings::ExclusionVerdict::Refuse,
            "«{nothing}» folds to nothing and can never match a process — refused aloud"
        );

        assert!(
            lang_switcher::guard::fold_process_name(nothing).is_empty(),
            "the premise of the refusal: «{nothing}» really does fold to nothing"
        );
    }
}

/// **The controls of task T-55-8: an edit of the ticks still reaches the cycle, a tick taken back
/// is no edit, and «ОК» twice in a row without edits gives the same cycle twice** — the one the
/// file had, on the very session that lengthened it before.
#[test]
fn an_edit_of_the_ticks_reaches_the_cycle_and_ok_twice_changes_nothing() {
    let session = [EN, RU];
    let file = vec!["0x00000409".to_owned(), "0x00000419".to_owned()];
    let mut layouts = Config::default().layouts;
    layouts.cycle = file.clone();

    let at_open = settings::layout_rows(&layouts, &session);

    // Unticking RU is an edit, and the cycle follows it.
    let mut edited = at_open.clone();
    for row in &mut edited {
        if row.layout == RU {
            row.checked = false;
        }
    }

    let written = settings::cycle_after_dialog(&file, &at_open, &edited, &session);
    assert_eq!(
        written,
        ["0x00000409"],
        "an edit of the ticks reaches the cycle"
    );

    // Ticking it back is no edit at all: the file's string goes back as it was.
    let written = settings::cycle_after_dialog(&file, &at_open, &at_open, &session);
    assert_eq!(written, file, "a tick taken back is no change");

    // «ОК» twice without edits, on the session that lengthened before.
    let session = [EN, EN_SECOND, RU];
    layouts.cycle = file.clone();

    let at_open = settings::layout_rows(&layouts, &session);
    let first = settings::cycle_after_dialog(&file, &at_open, &at_open, &session);
    let second = settings::cycle_after_dialog(&file, &at_open, &at_open, &session);

    assert_eq!(first, second, "two «ОК» in a row give the same cycle");
    assert_eq!(
        first, file,
        "and it is the order and the length of the file, not a longer one"
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
    // Since T-120-20 the layouts are shown by the window of the settings on the shell: the pair
    // and the cycle take their labels from `layout_labels`, where the collision rule of
    // решение 82.1 lives.
    let window: String = window_of_settings_source()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    for call in [
        "settings::layout_labels(&self.session,&self.session)",
        "settings::layout_labels(&names,&self.session)",
    ] {
        assert!(
            window.contains(call),
            "the window of the settings must take its labels from `layout_labels`: `{call}`"
        );
    }

    let source = settings_module_source();
    assert!(
        !source.contains("fn layout_label("),
        "`layout_label` built one label out of one layout and could not see a collision at \
         all — Т-23-3 replaced it with `layout_labels`"
    );
}

/// The source of the window of the settings on the shell (T-120-20).
fn window_of_settings_source() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("ui")
            .join("windows")
            .join("settings.rs"),
    )
    .expect("the window of the settings, src/ui/windows/settings.rs, must be readable")
    .replace("\r\n", "\n")
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
    assert_eq!(settings::hotkey_note(&bare("Pause")), None);
    assert_eq!(settings::hotkey_note(&bare("ScrollLock")), None);
    assert_eq!(settings::hotkey_note(&bare("F9")), None);

    // FR-92: «предупреждение при выборе текстовой клавиши». Since FR-94 the answer is the
    // identifier of an interface string rather than the string itself, so the text is fetched
    // out of the built binary — in both locales, because a warning that exists in one language
    // only is half a warning.
    let product = ProductImage::shared();

    let letter = settings::hotkey_note(&bare("A")).expect("a letter must be warned about");
    let letter_ru = product.string(settings::Language::Ru, letter);
    let letter_en = product.string(settings::Language::En, letter);
    println!("A -> {letter} -> ru: {letter_ru}\n         -> en: {letter_en}");
    assert!(letter_ru.contains("текстовая"));
    assert!(letter_en.contains("text key"));

    // And the other silent failure: a name this build does not know leaves `Pause` in force,
    // which `app::publish_configuration` does without telling anybody. This is the telling.
    let unknown = settings::hotkey_note(&bare("Ктулху")).expect("an unknown name must be reported");
    let unknown_ru = product.string(settings::Language::Ru, unknown);
    let unknown_en = product.string(settings::Language::En, unknown);
    println!("unknown -> {unknown} -> ru: {unknown_ru}\n               -> en: {unknown_en}");
    assert!(unknown_ru.contains("не распознано"));
    // Q-176-2, задача T-102-3: the English table is in American spelling.
    assert!(unknown_en.contains("not recognized"));

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

    // The ceilings, as numbers — the ones `reports\ТЗ-Э13-ремонт.md` fixes for this task; the
    // pause of FR-44 at a quarter of a second since решение 180.1 (task T-103-4), a thousand until
    // then.
    assert_eq!(inject::MAX_INTER_EVENT_DELAY_MS, 250);
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

/// **Решение 120.4 (е), task T-55-1 — the guard against litter.** Only the installed product may
/// put itself into `HKCU\…\Run`: a release image at `%ProgramFiles%\Lang_Switcher\LangSwitcher.exe`.
///
/// Checked on paths written out here, so that both answers are reached whatever machine and profile
/// run the test. Every case is collected before anything is asserted, so a red names all of them.
#[test]
fn only_the_installed_release_image_may_register_itself() {
    let program_files = Path::new(r"C:\Program Files");
    let installed = Path::new(r"C:\Program Files\Lang_Switcher\LangSwitcher.exe");

    let cases: [(&str, bool, &Path, &Path, bool); 9] = [
        (
            "the installed release image — the control",
            false,
            installed,
            program_files,
            true,
        ),
        (
            "the same path spelt in another case",
            false,
            Path::new(r"c:\PROGRAM FILES\lang_switcher\langswitcher.EXE"),
            Path::new(r"C:\program files"),
            true,
        ),
        (
            "the installed path, but a debug build",
            true,
            installed,
            program_files,
            false,
        ),
        (
            "a release build where cargo leaves it",
            false,
            Path::new(r"C:\Builds\LangSwitcher\target\release\LangSwitcher.exe"),
            program_files,
            false,
        ),
        (
            "a test binary",
            false,
            Path::new(r"C:\Builds\LangSwitcher\target\debug\deps\tray-0123456789abcdef.exe"),
            program_files,
            false,
        ),
        (
            "another image in the installed folder",
            false,
            Path::new(r"C:\Program Files\Lang_Switcher\unins000.exe"),
            program_files,
            false,
        ),
        (
            "a folder that only begins with the name",
            false,
            Path::new(r"C:\Program Files\Lang_Switcher2\LangSwitcher.exe"),
            program_files,
            false,
        ),
        (
            "one folder deeper",
            false,
            Path::new(r"C:\Program Files\Lang_Switcher\old\LangSwitcher.exe"),
            program_files,
            false,
        ),
        (
            "the 32-bit Program Files",
            false,
            installed,
            Path::new(r"C:\Program Files (x86)"),
            false,
        ),
    ];

    let mut wrong = Vec::new();

    for (name, debug_build, exe, program_files, expected) in cases {
        let answered = settings::autostart_may_register(debug_build, exe, program_files);

        println!(
            "{name}: debug={debug_build} exe={} program_files={} -> {answered}",
            exe.display(),
            program_files.display()
        );

        if answered != expected {
            wrong.push(format!("{name}: expected {expected}, answered {answered}"));
        }
    }

    assert!(wrong.is_empty(), "решение 120.4 (е): {wrong:?}");
}

/// **The same guard asked about the image running this test** — the answer that stands between a
/// battery of `cargo test` and the `Run` key of whoever runs it. Every tray the tests attach goes
/// through the start-up reconciliation of решение 120.4 (б), and this is what keeps it from writing.
#[test]
fn no_test_binary_may_register_itself() {
    println!(
        "current_exe = {:?}, debug_assertions = {}, ProgramFiles = {:?}",
        std::env::current_exe(),
        cfg!(debug_assertions),
        std::env::var_os("ProgramFiles")
    );

    assert!(
        !settings::this_build_may_register_autostart(),
        "решение 120.4 (е): a test binary answered that it may put itself into HKCU\\…\\Run"
    );
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
        let _ex_style = read_u32(bytes, &mut at);
        let style = read_u32(bytes, &mut at);
        let items = read_u16(bytes, &mut at);

        // x, y, cx, cy. The position is the dialog manager's own business — `DS_CENTER`
        // places the window — but the size is the layout's, and task T-11-19 measures it.
        let _x = read_u16(bytes, &mut at);
        let _y = read_u16(bytes, &mut at);
        let _size = (
            i32::from(read_u16(bytes, &mut at) as i16),
            i32::from(read_u16(bytes, &mut at) as i16),
        );

        let _menu = read_name(bytes, &mut at);
        let _class = read_name(bytes, &mut at);
        let _caption = read_string(bytes, &mut at);

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
            styles,
            bounds,
            classes,
            font,
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
/// The count has followed `settings::INTERFACE_STRINGS` since, row for row; **218 since task
/// T-43-9** (решение 128.2), whose notification of FR-82 is the last row.
///
/// ⚠⚠ **217 since task T-81-3** — решение 142.2 п. 5, and the first time this number has gone
/// **down**. `IDS_AUTHOR_VERSION` carried the author's signature under the name of the program
/// in «От авторе» and the owner asked for it to leave; the head of that window is the head of
/// «О программе» now, version line included, so the string had no second reader and went with
/// all fourteen of its translations.
///
/// ⚠ **218 since task T-93-2** — вопрос 157: `IDS_CAPTURE_COMBINATION` left with the refusal it
/// said, and the two lines under the hotkey field arrived (the one at rest and the one after a
/// combination) — 217 − 1 + 2, row for row with `settings::INTERFACE_STRINGS`.
///
/// ⚠ **220 since task T-95-2** — вопрос 159: the line while the capture waits for the second
/// `Shift` and the line after the double press — 218 + 2, row for row.
///
/// ⚠ **229 since task T-103-2** — вопрос 181: the button of «Спасибо» that sends a person where they
/// can write, `IDS_FEEDBACK_OPEN` — 228 + 1, row for row.
///
/// ⚠ **235 since task T-103-7** — решение 181.6: the six words of the last step of a trouble, which
/// goes to the author by letter — 229 + 6, row for row.
///
/// ⚠ **237 since task T-104-7** — TZ E104 §0 п. 5 и 12: the two words of the balloon after an
/// update — 235 + 2.
///
/// ⚠ **240 since stage L10N, task T-L10N-10** — решения 200.10 и 200.10а: the three pieces the
/// code glued as literals — the separator of a list, the arrow of a cycle, the form of the
/// tooltip — 237 + 3, each the literal it replaced.
///
/// ⚠ **241 since stage E107, task T-107-3** — решения 199.3, 199.10, 199.11: the appeal goes to the
/// site from the last step of the wizard. Nine arrived and eight left (the group, the issues, the
/// note of the letter of a trouble, the two thanks) — 240 − 8 + 9, row for row with
/// `settings::INTERFACE_STRINGS`.
///
/// ⚠ **231 since stage E119, task T-119-8** — решение 208.94 п. 4: the letter «Что нового» with its
/// eight words and its balloon, and the balloon «обновлён до X» with its two — 242 − 11.
///
/// ⚠ **232 since task T-119-2** — FR-90б: the check box of the flag of the layout — 231 + 1.
///
/// ⚠ **245 since stage E120, tasks T-120-21 and T-120-22** — решение 208.169, С10 and С12:
/// «Состояние» for a person (3280…3288) and «Нашли ошибку в переводе?» (3289…3292) — 232 + 13.
///
/// ⚠ **247 since task T-119-3, part Б** — решение 208.88: the title and the link of the list of the
/// program's own (3265, 3266) — 245 + 2.
const FR_94_STRINGS: [(u16, &str, &str); 265] = [
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
    // Task T-120-22 (решение 208.169, С12): the button under the language.
    (
        settings::IDS_TRANSLATION_REPORT,
        "Нашли ошибку в переводе?",
        "Found a mistake in the translation?",
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
        "Исправлять выделенный текст",
        "Fix the selected text",
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
    // Task T-39-11, решение 122.5: no ellipsis — the button opens no dialog.
    (
        settings::IDS_LOG_SAVE,
        "Сохранить журнал",
        "Save the journal",
    ),
    (
        settings::IDS_LOG_SAVE_FAILED,
        "Не удалось сохранить журнал.",
        "The journal could not be saved.",
    ),
    (
        settings::IDS_SETTINGS_NOT_SAVED,
        "Не удалось сохранить настройки. Изменение не применено.",
        "The settings could not be saved. The change was not applied.",
    ),
    (settings::IDS_LOG_SAVED, "Журнал сохранён", "Journal saved"),
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
        "The key name is not recognized — the default key, Pause, is in force.",
    ),
    (
        // ⚠ Task Т-23-5: the invitation moved into the **field** and lost its second half.
        // «Esc — отмена» became a line of its own under the field — `IDS_CAPTURE_HINT`. Task
        // T-93-2 (вопрос 157): it invites a combination as well — in the words that fit the
        // field (157.13: «Нажмите клавишу или сочетание…» of the mock-up is 207 px of 172).
        settings::IDS_CAPTURE_PROMPT,
        "Клавиша или сочетание…",
        "A key or a combination…",
    ),
    (
        // Task T-95-2, вопрос 159 (the mock-up of 157.18): a `Shift` on its own is no refusal any
        // more, and the line names the way left — «Модификатор сам по себе горячей клавишей быть
        // не может.» before it.
        settings::IDS_CAPTURE_MODIFIER,
        "Ctrl и Alt — только вместе с клавишей; дважды — только Shift.",
        "Ctrl and Alt go only with a key; only Shift can be pressed twice.",
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
    // ⭐ Task T-36-5, finding Н9, решение 123.3 — the seventh reason. 3228 by number, the capture
    // dialog by meaning: the number continues the block of the state strings because that is
    // where the free ones are.
    (
        settings::IDS_CAPTURE_TYPING,
        "Эта клавиша нужна при наборе — выберите другую.",
        "Typing needs this key — choose another one.",
    ),
    // Task T-34-5 (Н80, Н84, С1): the first line of «Состояние» speaks of the program. 3045…3047
    // — the developer's «Перехват клавиатуры: {0} · восстановлений хука: {1} · …» and its two
    // words — are retired holes now.
    (settings::IDS_STATE_WORKING, "Работает", "Working"),
    (settings::IDS_STATE_SUSPENDED, "Приостановлена", "Suspended"),
    (
        settings::IDS_STATE_HOOK_ABSENT,
        "Перехват клавиатуры не установлен",
        "The keyboard hook is not installed",
    ),
    (
        settings::IDS_STATE_DISABLED,
        "Перехват установлен, но обработка отключена — нужен перезапуск",
        "The hook is installed, but processing is off — restart the program",
    ),
    // ⚠ Task T-36-4 (Н40) gave this line a **third** number: presses that reached nobody
    // (`hook::lost_hotkeys`). A row of its own was the other way to show it, and the appearance
    // is frozen by the owner's word of 2026-09-10 — so the number joined the line instead, and
    // that made this a changed string in all fourteen tables rather than a new one. The count of
    // `INTERFACE_STRINGS` is therefore unchanged.
    (
        settings::IDS_STATE_HEALTH,
        "тихих снятий перехвата: {0} · отказов установки: {1} · потерянных нажатий: {2}",
        "silent hook removals: {0} · install failures: {1} · lost presses: {2}",
    ),
    (settings::IDS_STATE_JOINED, "{0} · {1}", "{0} · {1}"),
    // Task T-34-6 (Н102): the fourth line — what acts, and the refusals of selection.
    (
        settings::IDS_STATE_PAIR,
        "Действующая пара: {0} → {1}",
        "Acting pair: {0} → {1}",
    ),
    (
        settings::IDS_STATE_CYCLE,
        "Действующий цикл: {0}",
        "Acting cycle: {0}",
    ),
    (
        settings::IDS_STATE_PAIR_NONE,
        "Действующая пара не выбрана",
        "No acting pair",
    ),
    (
        settings::IDS_STATE_NO_FAILURES,
        "отказов выбора не было",
        "no selection failures",
    ),
    (
        settings::IDS_STATE_FAILURES,
        "отказов выбора: {0}",
        "selection failures: {0}",
    ),
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
    // Task T-120-21 (решение 208.169, С10): «Состояние» for a person, 3280…3288.
    (settings::IDS_STATUS_WORKING, "Работает", "Working"),
    (
        settings::IDS_STATUS_SUSPENDED,
        "Приостановлена",
        "Suspended",
    ),
    (
        settings::IDS_STATUS_RESTART,
        "Нужен перезапуск",
        "Restart needed",
    ),
    (settings::IDS_STATUS_FIXES, "Исправляет: {0}", "Fixes: {0}"),
    (
        settings::IDS_STATUS_FIXES_NONE,
        "Раскладки не выбраны",
        "No layouts chosen",
    ),
    (settings::IDS_STATUS_HOTKEY, "Клавиша: {0}", "Key: {0}"),
    (
        settings::IDS_STATUS_AUTOSTART_ON,
        "Запуск при входе в систему: включён",
        "Start when I sign in: on",
    ),
    (
        settings::IDS_STATUS_AUTOSTART_OFF,
        "Запуск при входе в систему: выключен",
        "Start when I sign in: off",
    ),
    (
        settings::IDS_STATUS_DETAILS,
        "Подробности для автора",
        "Details for the author",
    ),
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
        "Исправляет текст, набранный не в той раскладке.",
        "Fixes text typed in the wrong keyboard layout.",
    ),
    (
        settings::IDS_ABOUT_LINE_2,
        "Исправление — по нажатию одной клавиши.",
        "Fixing takes a single key press.",
    ),
    (settings::IDS_ABOUT_OK, "Закрыть", "Close"),
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
        "Набрали слово не в той раскладке — нажмите {0}: слово исправится, раскладка переключится.",
        "You typed a word in the wrong layout — press {0}: the word is fixed and the layout switches.",
    ),
    (
        settings::IDS_ABOUT_HELP_2,
        "Повторное нажатие {0} возвращает исходный текст.",
        "Pressing {0} again brings the original text back.",
    ),
    (
        settings::IDS_ABOUT_HELP_3,
        "Выделите текст и нажмите {0} — исправится выделенное (если это включено и работает Ctrl+C).",
        "Select text and press {0} — the selection is fixed (if enabled and Ctrl+C works).",
    ),
    (
        settings::IDS_ABOUT_HELP_4,
        // Task T-43-8, finding Н86: the help names the menu entry by the menu's own word —
        // «Приостановка» for «Приостановить», not «Пауза».
        "Приостановка, настройки и выход — в значке в трее.",
        "Suspend, settings and exit are in the tray icon.",
    ),
    (
        settings::IDS_ABOUT_HELP_5,
        "Сменить клавишу: Настройки → Горячая клавиша.",
        "To change the key: Settings → Hotkey.",
    ),
    // Stage E114, task T-114-2, решение 208.5 (6а): the press over nothing typed and nothing
    // selected switches the layout, and the help says so in its sixth row.
    (
        settings::IDS_ABOUT_HELP_6,
        "Ничего не набрано и не выделено — нажмите {0}: раскладка переключится.",
        "Nothing typed or selected — press {0} to switch the layout.",
    ),
    (settings::IDS_MENU_SUSPEND, "Приостановить", "Suspend"),
    (settings::IDS_MENU_RESUME, "Возобновить", "Resume"),
    (
        settings::IDS_MENU_RESUME_RESTART,
        "Возобновить — нужен перезапуск",
        "Resume — restart needed",
    ),
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
    // suspended» — not the hook's, which is what the retired IDS_HOOK_UP spoke of until T-34-5.
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
    // ⚠ Task T-79-1: the caption used to be «Открыть страницу поддержки» / «Open the support
    // page», and the user turned it down on the live product — it reads as a *technical* support
    // page, where one asks questions about the program, and not as supporting its author. His
    // words: «давай переименуем в «Поддержать автора»». The identifier keeps its name and its
    // number; only the fourteen captions moved.
    (
        settings::IDS_SUPPORT_OPEN,
        "Поддержать автора",
        "Support the author",
    ),
    (
        settings::IDS_HELLO_TITLE,
        "Привет! Меня зовут Панда.",
        "Hello! My name is Panda.",
    ),
    (
        settings::IDS_HELLO_LEAD,
        "Я сделал Lang Switcher и рад, что вы его поставили. Главное, что он делает, — \
         исправляет текст, набранный не в той раскладке.",
        "I made Lang Switcher and I am glad you installed it. Its main job is to fix text typed \
         in the wrong layout.",
    ),
    (
        settings::IDS_HELLO_DEMO_CAP,
        "Набрали «ghbdtn», нажали {0}: получили «привет», раскладка переключилась на русскую. \
         Ещё раз — и как было.",
        "You typed “руддщ”, pressed {0} and got “hello”, with the layout switched to English. \
         Press it again and it is back as it was.",
    ),
    (
        settings::IDS_HELLO_PANEL,
        "Три вещи на первый день",
        "Three things for the first day",
    ),
    (
        settings::IDS_HELLO_ROW_1,
        "Набрали слово не в той раскладке — нажмите {0}: слово исправится, раскладка \
         переключится.",
        "Typed a word in the wrong layout — press {0}: the word is fixed and the layout switches.",
    ),
    (
        settings::IDS_HELLO_ROW_2,
        "Выделили текст — {0} исправит выделенное. Ничего не набрано — просто переключит \
         раскладку.",
        "Selected some text — {0} fixes the selection. Nothing typed — it just switches the \
         layout.",
    ),
    (
        settings::IDS_HELLO_ROW_3,
        "Ещё раз {0} — всё вернётся как было, буква в букву.",
        "Press {0} again and everything comes back as it was, letter for letter.",
    ),
    // T-120-16 (208.106 п. 3): the line under the body of «Привет».
    (
        settings::IDS_HELLO_FOOT,
        "Пауза и настройки — правым щелчком по значку в трее. В настройках можно включить флаг \
         раскладки в трее.",
        "Pause and settings are a right-click on the tray icon. In the settings you can turn on \
         the layout flag in the tray.",
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
        "Привет, это снова Панда. Месяц назад вы поставили Lang Switcher, и я надеюсь, \
         что вам с ним удобно: я потратил много сил, чтобы программа была лёгкой и просто \
         работала.",
        "Hello, it is Panda again. You installed Lang Switcher a month ago, and I hope it suits \
         you: I put a great deal of work into keeping it light and simply working.",
    ),
    (
        settings::IDS_THANKS_PARA,
        "Если понравилось, расскажите об этом в группе. Если что-то не так, напишите мне: \
         негатив тоже важен, только узнав о проблеме, её можно починить. Мастер обращения есть \
         в меню значка, пункт «Написать автору…».",
        "If you liked it, say so in GitHub Issues. If something is wrong, write to me: criticism \
         matters too — a problem can only be fixed once somebody knows about it. The wizard is \
         in the tray menu, under “Write to the author…”.",
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
        "Напомнить позже",
        "Remind me later",
    ),
    (
        settings::IDS_THANKS_FOOT,
        "Это письмо приходит один раз. Отложить его можно только раз — на неделю.",
        "This letter comes once. You can put it off only once, for a week.",
    ),
    (settings::IDS_AUTHOR_CAPTION, "От автора", "From the author"),
    // ⛔⛔ `IDS_AUTHOR_VERSION` (3111) стояла здесь и **снята задачей T-81-3**, решение 142.2
    // п. 5: «версия {0} · Панда, он же Panda_Pishet_Kod». Шапка двух окон стала одной, и строку
    // версии в обоих пишет `IDS_ABOUT_VERSION`. Надгробие стоит, чтобы снятие читалось как
    // решение, а не как пропущенная строка.
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
        // Решение 189.31: the short form of 189.12 for the window — the full text made «От автора»
        // 35–40 units taller and the windows a step smaller on six screens of fifteen (C9, 189.30);
        // README and SPEC carry 189.12 word for word.
        settings::IDS_NEWS_ABOUT_FEED,
        "Раз в сутки программа читает файл новостей с сайта автора и ничего о вас не передаёт. \
         Новые версии она скачивает оттуда же и ставит сама, проверив подпись.",
        "Once a day the program reads a news file from the author's site and sends nothing about \
         you. It downloads new versions from there and installs them itself after checking the \
         signature.",
    ),
    (
        settings::IDS_NEWS_NEVER_READ,
        "Лента ещё не читалась.",
        "The feed has not been read yet.",
    ),
    (
        // Решение 189.31: the feed is read once a day, and «следующее чтение через 1 дней» said
        // nothing a person needed — the second sentence went in all fourteen tables.
        settings::IDS_NEWS_READ_ON,
        "Лента прочитана {0}, подпись автора верна.",
        "Feed read on {0}, the author's signature checks out.",
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
        "Can be turned off in the settings file: [letters] feed = false.",
    ),
    // ⛔ Task T-119-10 (решение 208.119): the switch of the feed and the line under it (3123, 3124)
    // left «От автора»; the switch there is «Сообщать об обновлениях программы» (3279).
    (settings::IDS_FEEDBACK_PANEL, "Обратная связь", "Feedback"),
    (
        settings::IDS_FEEDBACK_TEXT,
        // Stage E108, task T-108-0 (решение 202.5): until 1.3.0 the panel promised five steps and
        // the clipboard — the path the wizard of 1.2.0 left (решение 199.3).
        "Нашли ошибку или есть идея? Мастер соберёт обращение и отправит его автору, только когда \
         вы нажмёте «Отправить».",
        "Found a bug or have an idea? The wizard puts an appeal together and sends it to the \
         author only when you press “Send”.",
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
    // ⛔ Task T-119-9 (решение 208.99 п. 1): the balloon of «Обновление» (3137, 3138) is gone.
    (settings::IDS_ABOUT_AUTHOR, "От автора…", "From the author…"),
    // ⛔ Task T-119-9: the row of the update in «Последние письма» speaks the letter's words — 3140
    // left, 3297 and 3302 below.
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
    // ⛔ Task T-119-9: the quiet line of «Обновление» names the version of the feed — 3143 left,
    // 3298 below.
    (settings::IDS_UPDATE_HOW, "Как обновиться", "How to update"),
    (
        settings::IDS_UPDATE_STEP_1,
        "Нажмите «Открыть страницу загрузки»: там всегда последняя версия.",
        "Press “Open the download page”: the latest version is always there.",
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
        "The “Version {0} is available” entry stays in the tray menu until you update.",
    ),
    (
        settings::IDS_NEWS_LETTER_TITLE,
        "Новость от автора",
        "News from the author",
    ),
    (
        settings::IDS_NEWS_LATER,
        "Напомнить позже",
        "Remind me later",
    ),
    (
        settings::IDS_NEWS_OPEN_LINK,
        "Читать на сайте",
        "Read on the site",
    ),
    (
        settings::IDS_NEWS_FOOT,
        "«Прочитано» закрывает письмо насовсем. «Напомнить позже» или крестик: напомню через неделю и ещё \
         раз через две, потом только точка на значке.",
        "“Mark as read” closes the letter for good. “Remind me later” or the cross: I will remind you in \
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
        "слово исправилось не так, пропали буквы, раскладка не та",
        "the fix came out wrong, letters went missing, the layout was not the one",
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
        "The behavior depends on the window: a console, a browser and an editor take text in \
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
    // Task T-120-22 (решение 208.169, С12): the appeal about the translation, 3290…3292.
    (
        settings::IDS_WIZARD_TRANSLATION_NOW,
        "Что написано сейчас?",
        "What does it say now?",
    ),
    (
        settings::IDS_WIZARD_TRANSLATION_RIGHT,
        "Как правильно?",
        "What should it say?",
    ),
    (
        settings::IDS_WIZARD_TRANSLATION_LINE,
        "Ошибка в переводе: {0}",
        "Translation mistake: {0}",
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
        // «подпись: число» since T-L10N-10 (решение 200.10а): the number stood before its noun.
        "Записей в журнале: {0}. Только имена операций и коды ошибок, ни одной клавиши: так \
         устроен сам журнал",
        "Journal entries: {0}. Only operation names and error codes, not a single keystroke: that \
         is how the journal itself is built",
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
        // Stage E107, task T-107-4 (решение 199.5): the appeal goes when the person presses
        // «Отправить», and not by the clipboard and a page any more.
        "Это весь текст обращения. Правьте что угодно. Он уйдёт автору, только когда вы нажмёте \
         «Отправить».",
        "This is the whole text of the appeal. Edit anything you like. It goes to the author \
         only when you press “Send”.",
    ),
    // ⛔ IDS_WIZARD_COPY, IDS_WIZARD_COPIED_ONLY and IDS_WIZARD_COPIED left with the group and the
    // issues (stage E107, task T-107-3, решение 199.3).
    (
        settings::IDS_WIZARD_SAVE,
        "Сохранить в папку журнала",
        "Save to the journal folder",
    ),
    (settings::IDS_WIZARD_SAVED, "Сохранено: {0}", "Saved: {0}"),
    (
        settings::IDS_WIZARD_FAILED,
        "Не получилось. Текст остался в окне — скопируйте его вручную.",
        "It did not work. The text is still in the window — copy it by hand.",
    ),
    // ⛔ The four IDS_THANKYOU_* gave way to one thanks for both kinds, IDS_WIZARD_SENT_TITLE and
    // IDS_WIZARD_SENT_TEXT at the end of this table (stage E107, решение 199.10).
    // FR-82, task T-43-9, finding Н23 — what a second copy says before it exits. The English row
    // is the literal `src\app.rs` carried until that task, word for word.
    (
        settings::IDS_ALREADY_RUNNING,
        "Lang Switcher уже работает в этом сеансе.",
        "Lang Switcher is already running in this session.",
    ),
    // Task T-93-2, вопрос 157 — the two lines under the hotkey field. Task T-95-2 (вопрос 159, the
    // mock-up of 157.18): the line at rest names the double press too.
    (
        settings::IDS_HOTKEY_HINT,
        "Клавиша, сочетание с Ctrl, Alt, Shift или двойной Shift.",
        "A key, a combination with Ctrl, Alt, Shift, or a double Shift.",
    ),
    (
        settings::IDS_CAPTURE_COMBO_NOTE,
        "{0} без {1} остаётся программам.",
        "{0} without {1} stays with your programs.",
    ),
    // Task T-95-2, вопрос 159 — the line while the capture waits for the second `Shift`, and the
    // one after the double press is captured.
    (
        settings::IDS_CAPTURE_SHIFT_AGAIN,
        "Ещё раз Shift — двойное нажатие; клавиша с Shift — сочетание.",
        "Shift again for a double press; a key with Shift for a combination.",
    ),
    (
        settings::IDS_CAPTURE_DOUBLE_NOTE,
        "Два нажатия Shift подряд. Обычный набор с Shift не затронут.",
        "Two presses of Shift in a row. Ordinary typing with Shift is not affected.",
    ),
    // Task T-100-3, вопрос 172 (вариант В) — the two ways of «Исключения»: the two words, the
    // button at rest and while it counts, the line under it at rest and after the capture. The
    // Russian is the mock-up's, word for word; «Уже в списке» and «Окно не выбрано» are the two
    // the mock-up did not show (172.5).
    (
        settings::IDS_EXCLUSION_AUTO,
        "Автоматически",
        "Automatically",
    ),
    (settings::IDS_EXCLUSION_MANUAL, "Вручную", "Manually"),
    (
        settings::IDS_EXCLUSION_PICK,
        "Выбрать окно программы",
        "Pick a program window",
    ),
    (
        settings::IDS_EXCLUSION_PICK_NOTE,
        "Нажмите и за 5 секунд переключитесь в программу",
        "Click, then switch to the program within 5 seconds",
    ),
    (
        settings::IDS_EXCLUSION_PICK_COUNT,
        "Переключитесь в нужную программу… {0}",
        "Switch to the program you need… {0}",
    ),
    (
        settings::IDS_EXCLUSION_ADDED,
        "Добавлено: {0}",
        "Added: {0}",
    ),
    (
        settings::IDS_EXCLUSION_ALREADY,
        "Уже в списке: {0}",
        "Already in the list: {0}",
    ),
    (
        settings::IDS_EXCLUSION_NOT_PICKED,
        "Окно не выбрано",
        "No window picked",
    ),
    // Task T-103-2, решение 181.1: where «Спасибо» sends a person to write — the group, GitHub
    // Issues. `IDS_CHANNEL_OPEN` above stays with «Что нового» and FR-103.
    (
        settings::IDS_FEEDBACK_OPEN,
        "Открыть группу",
        "Open GitHub Issues",
    ),
    // Task T-103-7, решение 181.6: an appeal about a trouble goes to the author by letter — the
    // table of the addendum to TZ-E103, word for word; the address is `{0}`. Since stage E107
    // (решение 199.3) the letter of any appeal the site did not take; IDS_WIZARD_MAIL_NOTE left.
    (
        settings::IDS_WIZARD_MAIL_ADDRESS,
        "Адрес для письма: {0}",
        "Email address: {0}",
    ),
    (
        settings::IDS_WIZARD_COPY_TEXT,
        "Скопировать текст",
        "Copy the text",
    ),
    (
        settings::IDS_WIZARD_COPY_ADDRESS,
        "Скопировать адрес",
        "Copy the address",
    ),
    (
        settings::IDS_WIZARD_TEXT_COPIED,
        "Текст скопирован. Вставьте его в письмо на {0}.",
        "Text copied. Paste it into an email to {0}.",
    ),
    (
        settings::IDS_WIZARD_ADDRESS_COPIED,
        "Адрес скопирован: {0}.",
        "Address copied: {0}.",
    ),
    (settings::IDS_LIST_SEPARATOR, ", ", ", "),
    (settings::IDS_CYCLE_ARROW, " → ", " → "),
    (settings::IDS_TIP_FORMAT, "{0} — {1}", "{0} — {1}"),
    // Stage E107, task T-107-3 — the appeal goes to the site from the last step of the wizard: the
    // words of the mock-up «Шаг «Отправить» мастера» (version 2, variant Б), of решение 199.10
    // («Готово», the owner's words) and of 201.11а (the label of the contact), word for word.
    (settings::IDS_WIZARD_SEND, "Отправить", "Send"),
    (settings::IDS_WIZARD_SENDING, "Отправляется…", "Sending…"),
    (settings::IDS_WIZARD_RETRY, "Повторить", "Retry"),
    (
        settings::IDS_WIZARD_CONTACT,
        "Как с вами связаться — почта или Telegram, по желанию, если понадобится уточнить",
        "How to reach you — email or Telegram, optional, in case something needs clarifying",
    ),
    (settings::IDS_WIZARD_CONTACT_LINE, "Для связи:", "Contact:"),
    (
        settings::IDS_WIZARD_FAILED_TITLE,
        "Не удалось отправить",
        "Could not send",
    ),
    (
        settings::IDS_WIZARD_FAILED_NOTE,
        "Сайт автора не ответил. Проверьте связь и нажмите «Повторить» или отправьте письмом: \
         скопируйте текст и адрес.",
        "The author's site did not answer. Check your connection and press “Retry”, or send it \
         by email: copy the text and the address.",
    ),
    (
        settings::IDS_WIZARD_SENT_TITLE,
        "Обращение отправлено",
        "Appeal sent",
    ),
    (
        settings::IDS_WIZARD_SENT_TEXT,
        "Спасибо, что отправили обращение! Я рад вашей активности в жизни проекта и обязательно \
         всё прочитаю в свободное время.",
        "Thank you for sending the appeal! I am glad you take part in the life of the project, \
         and I will read it all in my free time.",
    ),
    // Task T-119-2, FR-90б: the check box of the flag of the layout.
    (
        settings::IDS_TRAY_FLAG,
        "Флаг раскладки в трее",
        "Layout flag in the tray",
    ),
    (settings::IDS_NEW_BADGE, "новое", "new"),
    // Task T-119-3, part Б: the title and the link of the list of the program's own.
    (settings::IDS_OWN_LIST_TITLE, "Раскладка", "Keyboard layout"),
    (
        settings::IDS_OWN_LIST_MORE,
        "Дополнительные параметры клавиатуры",
        "More keyboard settings",
    ),
    // Tasks T-119-5, T-119-6, T-119-7, T-119-10: the window in the corner, the menu, «От автора».
    (settings::IDS_CORNER_UPDATE, "обновление", "update"),
    (
        settings::IDS_RELEASE_TITLE,
        "Lang Switcher обновлён до {0}",
        "Lang Switcher has been updated to {0}",
    ),
    (
        settings::IDS_RELEASE_TEXT,
        "Было {0}. Новое: значок программы может показывать флаг текущей раскладки. Щелчок по \
         флагу — список раскладок Windows. Включается галочкой в настройках.",
        "It was {0}. New: the icon of the program can show the flag of the current layout. A \
         click on the flag opens the list of layouts of Windows. Turned on with a tick in the \
         settings.",
    ),
    (
        settings::IDS_RELEASE_CHANGES,
        "Все изменения на сайте",
        "All the changes on the site",
    ),
    (
        settings::IDS_RELEASE_HOWTO,
        "Как пользоваться",
        "How to use it",
    ),
    (settings::IDS_CORNER_OK, "Понятно", "Got it"),
    (
        settings::IDS_RELEASE_NEVER,
        "Больше не показывать окна обновлений",
        "Do not show update windows again",
    ),
    (settings::IDS_CORNER_FLAG, "флаг раскладки", "layout flag"),
    // Task T-120-16: the header of the window in the corner that knocks with a letter.
    (settings::IDS_CORNER_LETTER, "письмо", "letter"),
    (
        settings::IDS_HINT_TITLE,
        "Флаг раскладки включён",
        "The layout flag is on",
    ),
    (
        settings::IDS_HINT_TEXT,
        "Чтобы флаг всегда был виден на панели задач, включите Lang Switcher в Параметрах: Панель \
         задач → Другие значки на панели задач. Пока он под стрелкой ^.",
        "To keep the flag in sight on the taskbar, turn Lang Switcher on in Settings: Taskbar → \
         Other system tray icons. For now it is under the arrow ^.",
    ),
    (
        settings::IDS_HINT_OPEN,
        "Открыть Параметры Windows",
        "Open Windows Settings",
    ),
    (
        settings::IDS_MENU_WHATSNEW,
        "Что нового в {0}…",
        "What’s new in {0}…",
    ),
    (
        settings::IDS_UPDATES_SWITCH,
        "Сообщать об обновлениях программы",
        "Tell me about updates of the program",
    ),
    // Task T-119-10: under the switch of «От автора» while the feed is off in the file.
    (
        settings::IDS_NEWS_OFF_IN_FILE,
        "Новости автора выключены в файле настроек",
        "News from the author is off in the settings file",
    ),
    // Task T-119-9 (решения 208.95–208.99): the honest letter «Обновление», by the mock-up.
    (
        settings::IDS_FALLBACK_TITLE,
        "Обновиться автоматически не получилось",
        "Could not update automatically",
    ),
    (
        settings::IDS_FALLBACK_SUB,
        "доступна {0} · у вас {1} · выпуск {2}",
        "available {0} · you have {1} · released {2}",
    ),
    (
        settings::IDS_FALLBACK_NOTICE,
        "Версия {0} вышла {1}, но программа не смогла поставить её сама. Обновите вручную — это \
         займёт минуту.",
        "Version {0} came out on {1}, but the program could not install it by itself. Update by \
         hand — it takes a minute.",
    ),
    (
        settings::IDS_FALLBACK_WHATS_NEW,
        "Что нового в {0}",
        "What’s new in {0}",
    ),
    (
        settings::IDS_FALLBACK_REPEATS,
        "Если так повторяется, напишите автору — это поможет найти причину.",
        "If this keeps happening, write to the author — it helps to find the cause.",
    ),
    (
        settings::IDS_ENTRY_FALLBACK_MARK,
        "доступна {0} · у вас {1}",
        "available {0} · you have {1}",
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

/// **Задача T-102-2, сторож (4), решение 178.6 — the plan of «Привет» carries the pair of the
/// language it is built in, and a plan built again after the language changed carries the new
/// one.** On a change of the language on the fly (решение 99) `refresh_plan` builds the plan of an
/// open window again through `plan_for`, and `draw_demo` draws the pair of the plan — so the pair
/// arrives together with the caption, exactly as in a window built anew (вопрос 178, П3).
///
/// Under the gate of [`LOCALE`]: the language is the process's (see [`with_product_strings`]).
///
/// ⚠ Control: the pairs of Russian and English differ — a plan that kept the pair of the language
/// it was first built in is caught.
#[test]
fn the_plan_of_hello_carries_the_pair_of_the_language_it_is_built_in() {
    let _guard = with_product_strings();

    let context = letters::PlanContext {
        version: "0.102.0".to_owned(),
        hotkey: "Pause".to_owned(),
        snooze_offered: true,
        item: None,
    };

    for language in Language::shipped() {
        settings::set_ui_language(language);
        let plan = letters::plan_for(letters::Letter::Welcome, &context);

        assert!(plan.demo, "{language:?}: «Привет» shows the demonstration");
        assert_eq!(
            plan.demo_pair,
            letters::demo_pair(language),
            "{language:?}: the plan carries the pair of its own language"
        );
    }

    // The window refilled on the fly: its plan built in Russian, then built again in English.
    settings::set_ui_language(Language::Ru);
    let russian = letters::plan_for(letters::Letter::Welcome, &context);
    settings::set_ui_language(Language::En);
    let english = letters::plan_for(letters::Letter::Welcome, &context);
    settings::set_ui_language(Language::Ru);

    assert_ne!(
        russian.demo_pair, english.demo_pair,
        "178.6: a plan built again in another language carries another pair"
    );
    assert_ne!(
        russian.demo_caption, english.demo_caption,
        "and another caption with it"
    );

    // A letter without a demonstration carries no pair to draw. («Что нового» stood here beside
    // «Спасибо» until task T-119-8 took it away.)
    let plan = letters::plan_for(letters::Letter::Thanks, &context);

    assert!(!plan.demo, "«Спасибо» has no demonstration");
    assert_eq!(
        plan.demo_pair,
        letters::DemoPair::default(),
        "«Спасибо»: and no pair"
    );
}

/// **Задача T-102-3, сторож (2), решение 178.6 — the caption under the demonstration names the pair
/// of its own language.** Each of the fourteen `IDS_HELLO_DEMO_CAP` carries, in quotation marks,
/// the word typed and the word it becomes of [`letters::demo_pair`] of its language — the pair the
/// demonstration above it draws — and «ghbdtn» and «привет» are the Russian caption's alone.
///
/// A word counts inside the guillemets the captions write it in, with a space on the inside or
/// without one: the French caption writes « sqlut ». Under the gate of [`LOCALE`].
///
/// ⚠ Control: the caption of one language held against the pair of another is caught.
#[test]
fn the_caption_of_hello_names_the_pair_of_its_own_language() {
    let _guard = with_product_strings();

    // The quotation marks of the language (RULES п. 3): « » of the fourteen (French with its
    // spaces), and since stage E108 “ ” of Hindi and „ “ of the languages that write them. Stage
    // E109, group B: „ ” (Croatian, Bosnian, Romanian, Hungarian), » « (Slovenian, Danish), ” ” and
    // » » (Finnish, Swedish), ‘ ’ (Welsh, Dutch).
    let quoted = |caption: &str, word: &str| {
        [
            ("«", "»"),
            ("« ", " »"),
            ("“", "”"),
            ("„", "“"),
            ("„", "”"),
            ("»", "«"),
            ("”", "”"),
            ("»", "»"),
            ("‘", "’"),
        ]
        .iter()
        .any(|(open, close)| caption.contains(&format!("{open}{word}{close}")))
    };
    let russian = letters::demo_pair(Language::Ru);
    let mut captions = Vec::new();

    for language in Language::shipped() {
        settings::set_ui_language(language);
        let caption = settings::text(settings::IDS_HELLO_DEMO_CAP);
        let pair = letters::demo_pair(language);

        assert!(
            quoted(&caption, pair.typed) && quoted(&caption, pair.fixed),
            "{language:?}: the caption names «{}» and «{}» of its own pair: {caption}",
            pair.typed,
            pair.fixed
        );

        if language != Language::Ru {
            assert!(
                !caption.contains(russian.typed) && !caption.contains(russian.fixed),
                "{language:?}: «{}» and «{}» are the Russian caption's alone: {caption}",
                russian.typed,
                russian.fixed
            );
        }

        captions.push((language, caption));
    }

    settings::set_ui_language(Language::Ru);

    let german = captions
        .iter()
        .find(|(language, _)| *language == Language::De)
        .map(|(_, caption)| caption.as_str())
        .expect("the German caption was read");
    let french = letters::demo_pair(Language::Fr);

    assert!(
        !(quoted(german, french.typed) && quoted(german, french.fixed)),
        "the control: a caption held against the pair of another language must be caught"
    );
}

/// Whether `text` holds the address as one slot — exactly one `{0}` and no address of its own.
fn holds_the_address_as_a_slot(text: &str) -> bool {
    text.matches("{0}").count() == 1 && !text.contains('@')
}

/// **Задача T-103-7, сторож (4), решение 181.6 — the words of the letter are in every language,
/// and the address in them is a slot.** The line of the address and the two answers carry `{0}`
/// once and no `@` of their own; the note and the two buttons carry neither; the note of the letter
/// is not the note of the step that sends. Ukrainian is word for word the table of the addendum
/// (Russian and English are, through `FR_94_STRINGS`). Under the gate of [`LOCALE`].
///
/// ⭐ Since stage E107 (решение 199.3) the letter is the way back of any appeal the site did not
/// take, and its note is `IDS_WIZARD_FAILED_NOTE` — `IDS_WIZARD_MAIL_NOTE` left with the rule
/// «a trouble by letter».
///
/// ⚠ Control: a button, which has no slot, is not taken for a line that holds one.
#[test]
fn the_words_of_the_letter_are_in_every_language_with_the_address_as_a_slot() {
    const WITH_SLOT: [u16; 3] = [
        settings::IDS_WIZARD_MAIL_ADDRESS,
        settings::IDS_WIZARD_TEXT_COPIED,
        settings::IDS_WIZARD_ADDRESS_COPIED,
    ];
    const WITHOUT: [u16; 3] = [
        settings::IDS_WIZARD_FAILED_NOTE,
        settings::IDS_WIZARD_COPY_TEXT,
        settings::IDS_WIZARD_COPY_ADDRESS,
    ];
    const UKRAINIAN: [(u16, &str); 5] = [
        (settings::IDS_WIZARD_MAIL_ADDRESS, "Адреса для листа: {0}"),
        (settings::IDS_WIZARD_COPY_TEXT, "Скопіювати текст"),
        (settings::IDS_WIZARD_COPY_ADDRESS, "Скопіювати адресу"),
        (
            settings::IDS_WIZARD_TEXT_COPIED,
            "Текст скопійовано. Вставте його в лист на {0}.",
        ),
        (
            settings::IDS_WIZARD_ADDRESS_COPIED,
            "Адресу скопійовано: {0}.",
        ),
    ];

    let _guard = with_product_strings();
    let mut wrong = Vec::new();

    for language in Language::shipped() {
        settings::set_ui_language(language);

        for id in WITH_SLOT {
            let text = settings::text(id);
            if !holds_the_address_as_a_slot(&text) {
                wrong.push(format!("{language:?} {id}: no single slot — «{text}»"));
            }
        }

        for id in WITHOUT {
            let text = settings::text(id);
            if text.trim().is_empty() || text.contains("{0}") || text.contains('@') {
                wrong.push(format!("{language:?} {id}: «{text}»"));
            }
        }

        if settings::text(settings::IDS_WIZARD_FAILED_NOTE)
            == settings::text(settings::IDS_WIZARD_PREVIEW_NOTE)
        {
            wrong.push(format!(
                "{language:?}: the note of the letter is the note of the step that sends"
            ));
        }

        if language == Language::Uk {
            for (id, want) in UKRAINIAN {
                let text = settings::text(id);
                if text != want {
                    wrong.push(format!("Uk {id}: «{text}», the addendum says «{want}»"));
                }
            }
        }
    }

    settings::set_ui_language(Language::En);
    let button = settings::text(settings::IDS_WIZARD_COPY_TEXT);
    settings::set_ui_language(Language::Ru);

    assert!(
        !holds_the_address_as_a_slot(&button),
        "the control: «{button}» has no slot and must not pass for a line that holds one"
    );
    assert!(
        wrong.is_empty(),
        "181.6: {} words of the letter are not as they must be:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The strings that say where a person is sent to write — решение 181.1, §0 п. 4 of TZ-E103. Six
/// until stage E107; four of them were the wizard's, and решение 199.3 took the group and the
/// issues out of the wizard — the two of «Спасибо» stay.
const STRINGS_OF_THE_PLACE: [(u16, &str); 2] = [
    (settings::IDS_THANKS_PARA, "IDS_THANKS_PARA"),
    (settings::IDS_FEEDBACK_OPEN, "IDS_FEEDBACK_OPEN"),
];

/// The words of the last step of the wizard and of its thanks — stage E107, решение 199.3: not one
/// of them may send a person to the group or to GitHub Issues any more.
const STRINGS_OF_THE_SENDING: [(u16, &str); 8] = [
    (settings::IDS_WIZARD_PREVIEW_NOTE, "IDS_WIZARD_PREVIEW_NOTE"),
    (settings::IDS_WIZARD_SEND, "IDS_WIZARD_SEND"),
    (settings::IDS_WIZARD_SENDING, "IDS_WIZARD_SENDING"),
    (settings::IDS_WIZARD_RETRY, "IDS_WIZARD_RETRY"),
    (settings::IDS_WIZARD_FAILED_TITLE, "IDS_WIZARD_FAILED_TITLE"),
    (settings::IDS_WIZARD_FAILED_NOTE, "IDS_WIZARD_FAILED_NOTE"),
    (settings::IDS_WIZARD_SENT_TITLE, "IDS_WIZARD_SENT_TITLE"),
    (settings::IDS_WIZARD_SENT_TEXT, "IDS_WIZARD_SENT_TEXT"),
];

/// Whether `text` names the group or GitHub Issues — the places решение 199.3 took out of the
/// wizard.
fn names_the_group_or_the_issues(text: &str) -> bool {
    let lower = text.to_lowercase();

    text.contains("GitHub") || lower.contains("груп")
}

/// The word each language called the channel by until `e102` — what none of the six may say now.
/// A table of the fourteen that said it and not a `match` (stage E108): a language that came after
/// `e102` never named the channel there.
const THE_CHANNEL: [(Language, &[&str]); 14] = [
    (Language::Ru, &["канал"]),
    (Language::Uk, &["канал"]),
    (Language::En, &["channel"]),
    (Language::De, &["kanal"]),
    (Language::Fr, &["chaîne"]),
    (Language::Es, &["canal"]),
    (Language::Pt, &["canal"]),
    (Language::It, &["canal"]),
    (Language::Pl, &["kanał", "kanal"]),
    (Language::Cs, &["kanál"]),
    (Language::Tr, &["kanal"]),
    (Language::El, &["κανάλ", "καναλ"]),
    (Language::He, &["ערוץ"]),
    (Language::Ar, &["قناة"]),
];

/// The word `language` called the channel by until `e102`, if it existed then.
fn the_channel_in(language: Language) -> &'static [&'static str] {
    THE_CHANNEL
        .iter()
        .find(|(of, _)| *of == language)
        .map_or(&[], |(_, words)| *words)
}

/// The languages that read the Russian side of the author's places — решение 181.1 (Russian,
/// Ukrainian) and решение 202.6 (Kazakh, Belarusian, Tatar, Uzbek in Cyrillic) — written here apart
/// from `letters::links`, whose answer this is.
const THE_RUSSIAN_SIDE: [Language; 6] = [
    Language::Ru,
    Language::Uk,
    Language::Kk,
    Language::Be,
    Language::Tt,
    Language::UzCyrl,
];

/// Whether `text` in `language` names the place of решение 181.1 and not the channel: on the
/// Russian side the group and not GitHub — «групп…» in Russian, «груп…» in Ukrainian, and in the
/// four of 202.6 whatever their word for it is, so no GitHub —, on the English side «GitHub Issues»,
/// untranslated.
fn names_its_place(language: Language, text: &str) -> bool {
    let lower = text.to_lowercase();
    let place = if THE_RUSSIAN_SIDE.contains(&language) {
        !text.contains("GitHub")
            && (!matches!(language, Language::Ru | Language::Uk) || lower.contains("груп"))
    } else {
        text.contains("GitHub Issues")
    };

    place
        && the_channel_in(language)
            .iter()
            .all(|word| !lower.contains(word))
}

/// **Задача T-103-3, решение 181.1 — each string of the place names the place of its own language.**
/// The paragraph of «Спасибо» and its button: in Russian and Ukrainian the group and not a word of
/// the channel, in the other twelve «GitHub Issues» — the name is not translated — and not their
/// word for the channel either. Under the gate of [`LOCALE`].
///
/// ⭐ **And, since stage E107 (решение 199.3), no word of the wizard's last step and of its thanks
/// names the group or the issues** — the appeal goes to the site, and the way back is the letter.
///
/// ⚠ Controls: «Открыть канал» of «Что нового», which stays, names the channel in every language
/// and is caught in every one of them; «Открыть группу» / «Open GitHub Issues» of «Спасибо» names
/// the place in every language and is caught by the second check in every one of them.
#[test]
fn each_string_of_the_place_names_the_place_of_its_own_language() {
    let _guard = with_product_strings();
    let mut wrong = Vec::new();
    let mut control_caught = 0;
    let mut place_caught = 0;

    for language in Language::shipped() {
        settings::set_ui_language(language);

        for (id, name) in STRINGS_OF_THE_PLACE {
            let text = settings::text(id);

            if !names_its_place(language, &text) {
                wrong.push(format!("{language:?} {name}: «{text}»"));
            }
        }

        for (id, name) in STRINGS_OF_THE_SENDING {
            let text = settings::text(id);

            if names_the_group_or_the_issues(&text) {
                wrong.push(format!(
                    "{language:?} {name} still sends to a place: «{text}»"
                ));
            }
        }

        // The controls are read in the fourteen whose words for the channel and the group this
        // test knows (stage E108: a language of group A brings neither).
        if the_channel_in(language).is_empty() {
            continue;
        }

        if !names_its_place(language, &settings::text(settings::IDS_CHANNEL_OPEN)) {
            control_caught += 1;
        }

        if names_the_group_or_the_issues(&settings::text(settings::IDS_FEEDBACK_OPEN)) {
            place_caught += 1;
        }
    }

    settings::set_ui_language(Language::Ru);

    assert_eq!(
        control_caught,
        THE_CHANNEL.len(),
        "the control: «Открыть канал» must be caught in every language of 1.0.0"
    );
    assert_eq!(
        place_caught,
        THE_CHANNEL.len(),
        "the control: «Открыть группу» / «Open GitHub Issues» must be caught in every language of \
         1.0.0"
    );
    assert!(
        wrong.is_empty(),
        "181.1: {} strings do not name the place of their language:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The words of the path the wizard walked until 1.2.0 — the clipboard and the five steps — in the
/// fourteen languages that said them in `IDS_FEEDBACK_TEXT` (stage E108, task T-108-0). A table and
/// not a `match`: a language that came after 1.2.0 never said them.
const THE_OLD_PATH_OF_THE_FEEDBACK: [(Language, &[&str]); 14] = [
    (Language::Ru, &["буфер", "пять"]),
    (Language::En, &["clipboard", "five"]),
    (Language::De, &["Zwischenablage", "fünf"]),
    (Language::Fr, &["presse-papiers", "cinq"]),
    (Language::Es, &["portapapeles", "cinco"]),
    (Language::It, &["appunti", "cinque"]),
    (Language::Pt, &["área de transferência", "cinco"]),
    (Language::Pl, &["schowk", "pięciu"]),
    (Language::Uk, &["буфер", "п'ять"]),
    (Language::Tr, &["pano", "beş"]),
    (Language::Cs, &["schránk", "pěti"]),
    (Language::El, &["πρόχειρο", "πέντε"]),
    (Language::He, &["לוח", "חמישה"]),
    (Language::Ar, &["الحافظة", "خمس"]),
];

/// Whether `text` names the button `label` in quotation marks — the mark right before it (a space
/// between is allowed, as French writes « Envoyer ») opens a quotation in one of the conventions
/// the languages of the program use.
fn quotes_the_button(text: &str, label: &str) -> bool {
    text.match_indices(label).any(|(at, _)| {
        text[..at]
            .trim_end_matches([' ', '\u{a0}', '\u{202f}'])
            // ” opens a quotation too in Swedish and Finnish (stage E109).
            .ends_with(['«', '»', '“', '„', '"', '‹', '「', '‘', '”'])
    })
}

/// **Stage E108, task T-108-0, решение 202.5 — the panel «Обратная связь» of «От автора» says how
/// the wizard of 1.2.0 sends, in every language.** The wizard sends the appeal itself when the person
/// presses «Отправить» (решение 199.3); until 1.3.0 the panel said «за пять шагов … текст попадёт в
/// буфер обмена, а вставите его вы» in all fourteen. Now each language names its own button
/// `IDS_WIZARD_SEND` in quotation marks, says not a word of the clipboard or of the five steps, and
/// sends nobody to the group or the issues. Under the gate of [`LOCALE`].
///
/// ⚠ Control: the panel as it stood in `e107`, written down here in Russian and English, is caught
/// by both checks — they can fail.
#[test]
fn the_feedback_panel_names_the_button_that_sends_in_every_language() {
    const E107_RU: &str = "Нашли ошибку или есть идея? Мастер соберёт обращение за пять шагов. \
                           Ничего не отправится само: текст попадёт в буфер обмена, а вставите \
                           его вы.";
    const E107_EN: &str = "Found a bug or have an idea? The wizard puts a message together in \
                           five steps. Nothing is sent by itself: the text goes to the clipboard, \
                           and you paste it.";

    fn says_the_old_path(language: Language, text: &str) -> bool {
        let lower = text.to_lowercase();

        THE_OLD_PATH_OF_THE_FEEDBACK
            .iter()
            .filter(|(of, _)| *of == language)
            .flat_map(|(_, words)| words.iter())
            .any(|word| lower.contains(&word.to_lowercase()))
    }

    let _guard = with_product_strings();
    let mut wrong = Vec::new();
    let mut control = Vec::new();

    for language in Language::shipped() {
        settings::set_ui_language(language);

        let text = settings::text(settings::IDS_FEEDBACK_TEXT);
        let send = settings::text(settings::IDS_WIZARD_SEND);

        if !quotes_the_button(&text, &send) {
            wrong.push(format!("{language:?}: «{send}» is not named — «{text}»"));
        }
        if says_the_old_path(language, &text) {
            wrong.push(format!(
                "{language:?}: the clipboard or five steps — «{text}»"
            ));
        }
        if names_the_group_or_the_issues(&text) {
            wrong.push(format!("{language:?}: sends to a place — «{text}»"));
        }

        match language {
            Language::Ru => control.push((language, E107_RU, send)),
            Language::En => control.push((language, E107_EN, send)),
            _ => {}
        }
    }

    settings::set_ui_language(Language::Ru);

    assert_eq!(control.len(), 2, "the control reads Russian and English");
    for (language, old, send) in control {
        assert!(
            !quotes_the_button(old, &send) && says_the_old_path(language, old),
            "the control: the panel of e107 in {language:?} must be caught by both checks"
        );
    }
    assert!(
        wrong.is_empty(),
        "202.5: {} languages describe a path the wizard does not walk:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// **Stage E107, task T-107-3, решение 199.10 — the thanks of the wizard carries no number of the
/// appeal and no invitation to ask about it, in any language.** The owner's words: «просто спасибо
/// что отправили обращения … я обязательно все прочитаю в свободное время» — no `{0}`, no digit, and
/// not the old promise of the four thanks («рассказываю письмом…»). Under the gate of [`LOCALE`].
///
/// ⚠ Control: the paragraph of the old thanks of a trouble, read as it stood, is caught — the
/// check of «not the old promise» can fail. The words are out of the built binary of `e106`'s
/// tables, written down here.
#[test]
fn the_thanks_of_the_wizard_carries_no_number_of_the_appeal() {
    fn wrong_in(text: &str) -> bool {
        text.trim().is_empty()
            || text.contains("{0}")
            || text.chars().any(|c| c.is_ascii_digit())
            || text.contains("рассказываю")
            || text.contains("I tell about")
    }

    let _guard = with_product_strings();
    let mut wrong = Vec::new();

    for language in Language::shipped() {
        settings::set_ui_language(language);

        for id in [
            settings::IDS_WIZARD_SENT_TITLE,
            settings::IDS_WIZARD_SENT_TEXT,
        ] {
            let text = settings::text(id);

            if wrong_in(&text) {
                wrong.push(format!("{language:?} {id}: «{text}»"));
            }
        }
    }

    settings::set_ui_language(Language::Ru);

    assert!(
        wrong_in(
            "Я читаю каждое обращение сам. Ошибки чиню, а о важном для всех рассказываю письмом в \
             программе или в канале."
        ),
        "the control: the old promise must be caught"
    );
    assert!(
        wrong.is_empty(),
        "199.10: {} words of the thanks are not as they must be:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// **Задача T-103-8, решение 183.1 — the line about the file says where the feed can be turned
/// off, not that it is off.** `IDS_NEWS_FILE_ONLY` stands in «От автора» while the feed is **on**;
/// the English «Turned off in the settings file» and the Hebrew «מכובה בקובץ ההגדרות» read as a
/// state — «off». The words of the owner, letter for letter, out of the built binary.
///
/// ⚠ Control: the Russian line, which stays, is read out of the same binary as it is — the reading
/// is live.
#[test]
fn the_line_about_the_file_says_the_feed_can_be_turned_off_and_not_that_it_is_off() {
    const WORDS_OF_183_1: [(Language, &str); 2] = [
        (
            Language::En,
            "Can be turned off in the settings file: [letters] feed = false.",
        ),
        (
            Language::He,
            "ניתן לכבות בקובץ ההגדרות: [letters] feed = false.",
        ),
    ];

    let product = ProductImage::shared();

    assert_eq!(
        product.string(Language::Ru, settings::IDS_NEWS_FILE_ONLY),
        "Отключается в файле настроек: [letters] feed = false.",
        "the control: the Russian line stays and is read as it is"
    );

    let wrong: Vec<String> = WORDS_OF_183_1
        .iter()
        .filter_map(|&(language, words)| {
            let text = product.string(language, settings::IDS_NEWS_FILE_ONLY);

            (text != words).then(|| format!("{language:?}: «{text}», not «{words}»"))
        })
        .collect();

    assert!(wrong.is_empty(), "183.1: {}", wrong.join("; "));
}

/// **Task T-34-5, findings Н80, Н84 and С1 — the red «before».** The first line of «Состояние»
/// spoke of the hook to a developer: «восстановлений хука» counted the ticks of the FR-80 timer
/// (finding С1 — `RECOVERIES` moves on every successful reinstallation, the thirty-second one
/// included), and nothing in the interface said «Приостановлена» or «Работает» (Н80), or that
/// processing had been switched off after failures (Н84). The vocabulary of the built binary
/// is asked directly: no string may count hook recoveries any more, and the states of the
/// program must be there as words.
#[test]
fn the_state_panel_speaks_of_the_program_and_not_of_hook_recoveries() {
    let product = ProductImage::shared();
    let strings: Vec<(u16, String, String)> = settings::INTERFACE_STRINGS
        .iter()
        .map(|&id| {
            (
                id,
                product.string(settings::Language::Ru, id),
                product.string(settings::Language::En, id),
            )
        })
        .collect();

    let offending: Vec<u16> = strings
        .iter()
        .filter(|(_, ru, en)| ru.contains("восстановлений хука") || en.contains("hook recoveries"))
        .map(|(id, ..)| *id)
        .collect();
    assert!(
        offending.is_empty(),
        "these strings still count hook recoveries, which the FR-80 timer moves every thirty \
         seconds: {offending:?}"
    );

    for (ru, en) in [("Работает", "Working"), ("Приостановлена", "Suspended")]
    {
        assert!(
            strings.iter().any(|(_, r, e)| r == ru && e == en),
            "the interface has no state «{ru}» / «{en}»"
        );
    }

    assert!(
        strings
            .iter()
            .any(|(_, ru, _)| ru.contains("нужен перезапуск")),
        "no string tells the person that processing is off after failures and a restart is \
         needed (FR-99, finding Н84)"
    );
}

/// **Task T-34-6, finding Н102 — the red «before».** The four counters of the refusals of
/// selection (`layouts::selection_failures`) were visible only in the dump, which is off by
/// default, and the pair that acts was shown nowhere — a person whose key stays silent had
/// nothing to read. The vocabulary must name the acting pair and say «no refusals» in words
/// when there were none, rather than «отказов: 0 · пара: —».
#[test]
fn the_state_panel_names_the_acting_pair_and_the_refusals_of_selection() {
    let product = ProductImage::shared();
    let strings: Vec<(u16, String, String)> = settings::INTERFACE_STRINGS
        .iter()
        .map(|&id| {
            (
                id,
                product.string(settings::Language::Ru, id),
                product.string(settings::Language::En, id),
            )
        })
        .collect();

    for (ru, en) in [
        ("Действующая пара: {0} → {1}", "Acting pair: {0} → {1}"),
        ("отказов выбора не было", "no selection failures"),
        ("отказов выбора: {0}", "selection failures: {0}"),
    ] {
        assert!(
            strings.iter().any(|(_, r, e)| r == ru && e == en),
            "the interface has no string «{ru}» / «{en}»"
        );
    }
}

/// **Task T-34-6 — the pure half: every shape of the fourth line, out of the built tables.**
/// A pair, a cycle and nothing, each with and without refusals; zero is words, a number is a
/// number, and the two halves are joined the way the table joins them.
#[test]
fn the_acting_line_is_built_from_the_pair_and_the_four_counters_and_says_zero_in_words() {
    use settings::{Acting, acting_line};

    let _gate = with_product_strings();

    let pair = Acting::Pair("Русский".to_owned(), "English".to_owned());
    let cycle = Acting::Cycle(vec![
        "Русский".to_owned(),
        "English".to_owned(),
        "Deutsch".to_owned(),
    ]);

    let cases = [
        (
            &pair,
            0,
            "Действующая пара: Русский → English · отказов выбора не было",
        ),
        (
            &pair,
            7,
            "Действующая пара: Русский → English · отказов выбора: 7",
        ),
        (
            &cycle,
            0,
            "Действующий цикл: Русский → English → Deutsch · отказов выбора не было",
        ),
        (
            &cycle,
            1,
            "Действующий цикл: Русский → English → Deutsch · отказов выбора: 1",
        ),
        (
            &Acting::None,
            0,
            "Действующая пара не выбрана · отказов выбора не было",
        ),
        (
            &Acting::None,
            3,
            "Действующая пара не выбрана · отказов выбора: 3",
        ),
    ];

    for (acting, refusals, expected) in cases {
        let line = acting_line(acting, refusals);
        println!("{line}");
        assert_eq!(line, expected);
        assert!(
            !line.contains(": 0"),
            "zero must be said in words, never as «: 0»: {line}"
        );
    }
}

/// **Стенд Э108, FAIL 2, решение 202.19о: in a right-to-left window every name of the acting line
/// is an embedding of its own; in a left-to-right one the line is what it was.**
///
/// The stand read «Действующий цикл» backwards in all five right-to-left languages, each for one
/// kind of names: Urdu with «←» and Latin names, the four others with «→» and names in Arabic
/// script (`reports\shots-E108\README.md`). Held for all 44 languages with a Latin name, a name in
/// Arabic script and both: right to left, each name stands between two right-to-left marks in an
/// embedding of its own direction; left to right, no direction control at all — the forty-four
/// minus five, the twelve left-to-right ones of 1.2.0 among them, get the plain substitution.
#[test]
fn the_names_of_the_acting_line_are_embedded_in_a_right_to_left_window_only() {
    use settings::{Acting, acting_line};

    const RLM: char = '\u{200F}';
    const LRE: char = '\u{202A}';
    const RLE: char = '\u{202B}';
    const PDF: char = '\u{202C}';
    const CONTROLS: [char; 13] = [
        '\u{200E}', '\u{200F}', '\u{061C}', '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}',
        '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{FEFF}',
    ];

    let latin = "English (United States)";
    let arabic = "اردو (پاکستان)";
    let pair = Acting::Pair(latin.to_owned(), arabic.to_owned());
    let cycle = Acting::Cycle(vec![
        latin.to_owned(),
        arabic.to_owned(),
        "Deutsch".to_owned(),
    ]);

    let product = ProductImage::shared();
    let _gate = with_product_strings();
    let mut wrong = Vec::new();

    for language in Language::ALL {
        settings::set_ui_language(language);

        let (latin_in, arabic_in, german_in) = if language.is_rtl() {
            (
                format!("{RLM}{LRE}{latin}{PDF}{RLM}"),
                format!("{RLM}{RLE}{arabic}{PDF}{RLM}"),
                format!("{RLM}{LRE}Deutsch{PDF}{RLM}"),
            )
        } else {
            (latin.to_owned(), arabic.to_owned(), "Deutsch".to_owned())
        };

        let pair_wanted = product
            .string(language, settings::IDS_STATE_PAIR)
            .replace("{0}", &latin_in)
            .replace("{1}", &arabic_in);
        let arrow = product.string(language, settings::IDS_CYCLE_ARROW);
        let cycle_wanted = product
            .string(language, settings::IDS_STATE_CYCLE)
            .replace("{0}", &[latin_in, arabic_in, german_in].join(&arrow));

        for (line, wanted) in [
            (acting_line(&pair, 0), pair_wanted),
            (acting_line(&cycle, 0), cycle_wanted),
        ] {
            // The line goes on with the refusals through `IDS_STATE_JOINED` of the language.
            if !line.contains(&wanted) {
                wrong.push(format!("{}: «{line:?}» lacks «{wanted:?}»", language.tag()));
            }

            if !language.is_rtl() && line.chars().any(|c| CONTROLS.contains(&c)) {
                wrong.push(format!(
                    "{}: a direction control in «{line:?}»",
                    language.tag()
                ));
            }
        }
    }

    settings::set_ui_language(Language::Ru);

    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// **Стенд Э108, FAIL 2, решение 202.19о: the arrows of the acting line point the way the script
/// reads, and the comma of a list in Arabic script is the Arabic one.**
///
/// The rule of [`the_help_arrow_follows_the_direction_of_the_script`] for the two strings of
/// «Состояние» that carry an arrow: with the names embedded, the first of them stands on the
/// right in a right-to-left line, and the arrow that leads to the next is «←». Arabic had «, »
/// since 1.2.0 where Urdu, Persian and Uyghur have «، ».
#[test]
fn the_arrows_of_the_acting_line_follow_the_direction_of_the_script() {
    let product = ProductImage::shared();
    let mut wrong = Vec::new();

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
        let language = language_of_tag(tag);
        let (wanted, unwanted) = if language.is_rtl() {
            ('←', '→')
        } else {
            ('→', '←')
        };

        for id in [settings::IDS_CYCLE_ARROW, settings::IDS_STATE_PAIR] {
            let line = product.string_of_langid(langid, id);

            if !line.contains(wanted) || line.contains(unwanted) {
                wrong.push(format!("{tag} {id}: «{line}» wants «{wanted}»"));
            }
        }

        if registry::script_of(tag).as_deref() == Some("Arab") {
            let separator = product.string_of_langid(langid, settings::IDS_LIST_SEPARATOR);

            if separator != "، " {
                wrong.push(format!("{tag}: list separator «{separator}», wanted «، »"));
            }
        }
    }

    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// **Task T-34-5 — the pure half, all sixteen combinations.** `program_state` is a function of
/// the four flags the modules publish and of nothing else, and every combination has exactly one
/// answer, in the order the doc comment of the function gives: fail-safe first, then the hook
/// being absent, then the pause, then «Работает».
#[test]
fn the_state_of_the_program_is_decided_the_same_way_for_all_sixteen_combinations() {
    use settings::ProgramState;

    let mut seen = std::collections::BTreeMap::new();

    for bits in 0u8..16 {
        let installed = bits & 1 != 0;
        let down = bits & 2 != 0;
        let active = bits & 4 != 0;
        let fail_safe = bits & 8 != 0;

        let state = settings::program_state(installed, down, active, fail_safe);
        let expected = if fail_safe {
            ProgramState::Disabled
        } else if !installed || down {
            ProgramState::HookAbsent
        } else if !active {
            ProgramState::Suspended
        } else {
            ProgramState::Working
        };

        println!(
            "installed={installed} down={down} active={active} fail_safe={fail_safe} → {state:?}"
        );
        assert_eq!(state, expected, "combination {bits:#06b}");
        *seen.entry(state).or_insert(0) += 1;
    }

    // Every state is reachable, and «Работает» by exactly one road: installed, not down,
    // active, no fail-safe.
    assert_eq!(seen.get(&ProgramState::Working), Some(&1));
    assert_eq!(seen.get(&ProgramState::Suspended), Some(&1));
    assert_eq!(seen.get(&ProgramState::HookAbsent), Some(&6));
    assert_eq!(seen.get(&ProgramState::Disabled), Some(&8));

    // The strings the states name are the ones of the table, and each state names its own.
    let ids: std::collections::BTreeSet<u16> = [
        ProgramState::Working,
        ProgramState::Suspended,
        ProgramState::HookAbsent,
        ProgramState::Disabled,
    ]
    .into_iter()
    .map(ProgramState::string_id)
    .collect();
    assert_eq!(
        ids,
        [
            settings::IDS_STATE_WORKING,
            settings::IDS_STATE_SUSPENDED,
            settings::IDS_STATE_HOOK_ABSENT,
            settings::IDS_STATE_DISABLED,
        ]
        .into_iter()
        .collect()
    );
}

/// **Task T-34-5 — the subject is back in every locale.** The audit found the German and the
/// Hebrew line abbreviated into nonsense («Wiederherstellungen: 3 · Fehlschläge: 0» — of what?).
/// The word for the keyboard hook of each locale — the one the retired first line already used
/// as its subject — must stand in the line of the hook's health and in the line that says the
/// hook is absent, in all fourteen built tables.
#[test]
fn the_state_lines_carry_their_subject_in_every_locale() {
    let product = ProductImage::shared();

    // The subject per locale, as the panel already spelled it before this task (glossary of the
    // stage, `scratchpad-E34\glossary.md`); lower-case stems, matched case-insensitively.
    // ⭐ Stage E114, T-114-7 (б): Greek and Hebrew took the words of the site — «πιάσιμο» (in the
    // line of the health in the genitive, «πιασίματος», so a locale may name more than one form)
    // and «לכידת ההקשות», instead of «παρακολούθηση» and «מעקב», which read as «surveillance»
    // beside the promise that there is none.
    let subject: [(&str, &[&str]); 14] = [
        ("ru", &["перехват"]),
        ("en", &["hook"]),
        ("uk", &["перехоплення"]),
        ("de", &["hook"]),
        ("fr", &["interception"]),
        ("es", &["interceptación"]),
        ("pt", &["interceptação"]),
        ("it", &["intercettazione"]),
        ("pl", &["przechwytywani"]),
        ("cs", &["zachytávání"]),
        ("tr", &["yakalama"]),
        ("el", &["πιάσιμο", "πιασίματος"]),
        ("he", &["לכידת ההקשות"]),
        ("ar", &["مراقبة"]),
    ];

    let mut missing: Vec<String> = Vec::new();

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();

        // Stage E108: a language of group A has no stem written here — its word for the hook is
        // in the glossary of its translator, outside the crate. What is held for it is the same
        // fact without the word: the two lines share a run of three letters or more, the subject.
        // ⚠ A run and not a word: Thai, Lao and Khmer put no space between words (wave 1 of
        // stage E108 — «การดักจับ» stood in both Thai lines inside longer runs); and three, not
        // four: the Urdu word for the hook is «ہُک», three code points with its damma.
        let Some(forms) = subject.iter().find(|(t, _)| *t == tag).map(|(_, s)| *s) else {
            let letters = |id| -> Vec<char> {
                product
                    .string_of_langid(langid, id)
                    .to_lowercase()
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || is_a_mark(c) {
                            c
                        } else {
                            ' '
                        }
                    })
                    .collect()
            };
            let health = letters(settings::IDS_STATE_HEALTH);
            let absent: String = letters(settings::IDS_STATE_HOOK_ABSENT)
                .into_iter()
                .collect();
            let shared = health
                .windows(3)
                .filter(|run| run.iter().all(|c| *c != ' '))
                .map(|run| run.iter().collect::<String>())
                .find(|run| absent.contains(run.as_str()));
            println!("{tag}: shared {shared:?}");

            if shared.is_none() {
                missing.push(format!(
                    "{tag}: the two lines share no run of three letters — «{}» / «{absent}»",
                    health.iter().collect::<String>()
                ));
            }
            continue;
        };

        for id in [settings::IDS_STATE_HEALTH, settings::IDS_STATE_HOOK_ABSENT] {
            let line = product.string_of_langid(langid, id).to_lowercase();
            println!("{tag} {id}: {line}");

            if !forms.iter().any(|form| line.contains(form)) {
                missing.push(format!("{tag} {id}: «{line}» lacks any of {forms:?}"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "the subject is missing from these state lines:\n{}",
        missing.join("\n")
    );
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

    // ⭐ Task T-93-2 — **and the modifiers**: every one of the eight sets of `Ctrl`, `Alt` and
    // `Shift`, written the way a capture writes it (`Hotkey::from_binding`) and read the way the
    // start-up reads it (`Hotkey::binding`), is the set that was pressed, with the key it was
    // pressed with.
    let mut sets = 0;

    for ctrl in [false, true] {
        for alt in [false, true] {
            for shift in [false, true] {
                let set = (if ctrl { hook::MOD_CTRL } else { 0 })
                    | (if alt { hook::MOD_ALT } else { 0 })
                    | (if shift { hook::MOD_SHIFT } else { 0 });

                for vk in [0x77_u16, 0x13, 0x4B] {
                    let written = settings::Hotkey::from_binding(vk, set)
                        .expect("a named key with a set of the three is written");

                    assert_eq!(
                        written.binding(),
                        Some((vk, set)),
                        "{written:?} reads back as something else than 0x{vk:02X} with {set:#05b}"
                    );
                }

                sets += 1;
            }
        }
    }

    assert_eq!(sets, 8, "the eight sets of three modifiers");
}

#[test]
fn a_press_with_nothing_held_becomes_the_hotkey() {
    // **Criterion 13, the half that needs no window.** The name that comes out is the name that
    // goes into `[hotkey] key`, and it is built out of the virtual-key code alone: no character
    // is produced and no keyboard layout is consulted (SEC-01).
    assert_eq!(
        settings::capture(0x77, BARE),
        settings::Capture::Taken(bare("F8"))
    );
    assert_eq!(
        settings::capture(0x13, BARE),
        settings::Capture::Taken(bare("Pause"))
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
        settings::hotkey_note(&bare("A")),
        Some(settings::IDS_NOTE_TEXT_KEY)
    );
    assert_eq!(
        settings::effective_hotkey_name(&bare("A")),
        "A",
        "a text key in the file acts, and the help of FR-92а names it"
    );
}

#[test]
fn the_capture_refuses_every_press_that_cannot_be_a_hotkey() {
    // **Criterion 16.** Six refusals since task Т-23-5, each with a sentence of its own in both
    // locales — five until решение 82.6 made a text key a refusal instead of an assignment. Task
    // T-93-2 (вопрос 157) took the combination out of the list — it is taken now, see
    // `the_capture_takes_a_combination_as_the_key_and_its_set` — and it is six again.
    let product = ProductImage::shared();

    let ctrl = settings::Modifiers { ctrl: true, ..BARE };
    let shift = settings::Modifiers {
        shift: true,
        ..BARE
    };
    let altgr = settings::Modifiers {
        ctrl: true,
        alt: true,
        ..BARE
    };
    let win = settings::Modifiers { win: true, ..BARE };

    // ⭐ Task T-95-2, вопрос 159 (157.17): `Shift` on its own is no refusal any more but the
    // beginning of a double press — on its own and with itself held, as the queue reports its own
    // press. `Ctrl` and `Alt` on their own, and `Shift` pressed under either, stay refused.
    for (vk, held, what) in [
        (0x10_u16, BARE, "Shift"),
        (0x10, shift, "Shift, held by itself"),
        (0xA0, shift, "left Shift"),
        (0xA1, shift, "right Shift"),
    ] {
        assert_eq!(
            settings::capture(vk, held),
            settings::Capture::Shift,
            "{what} begins a double press"
        );
    }

    let cases = [
        // A modifier on its own — the user has not finished pressing anything.
        (0x11u16, BARE, settings::Refusal::Modifier, "Ctrl"),
        (0x12, BARE, settings::Refusal::Modifier, "Alt"),
        (
            0x10,
            ctrl,
            settings::Refusal::Modifier,
            "Shift при зажатом Ctrl",
        ),
        (
            0x10,
            settings::Modifiers { alt: true, ..shift },
            settings::Refusal::Modifier,
            "Shift при зажатом Alt",
        ),
        // The `Windows` key: the shell acts on it whatever anybody else does — and since task
        // T-93-2 any press with it held: `Win` is not a modifier a hotkey may carry (вопрос 157).
        (0x5B, BARE, settings::Refusal::Reserved, "Win"),
        (0x7B, win, settings::Refusal::Reserved, "Win+F12"),
        (
            0x7B,
            settings::Modifiers { win: true, ..ctrl },
            settings::Refusal::Reserved,
            "Win+Ctrl+F12",
        ),
        // ⭐ Task T-93-2, 157.10 (вариант 1 до слова владельца): a text key with a set that types
        // a character — `Shift` gives a capital, `Ctrl+Alt` is `AltGr`.
        (0x4B, shift, settings::Refusal::Text, "Shift+K"),
        (0x31, shift, settings::Refusal::Text, "Shift+1"),
        (0x41, altgr, settings::Refusal::Text, "AltGr+A"),
        (
            0x41,
            settings::Modifiers {
                shift: true,
                ..altgr
            },
            settings::Refusal::Text,
            "AltGr+Shift+A",
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
        // ⭐ Task T-36-5, finding Н9, решение 123.3 — row 3 of the FR-10 table, the keys typing
        // itself needs. Six of them were **taken** until this task: assigning `Delete` cost the
        // person that key in every application, because FR-95 suppresses the hotkey whenever the
        // program is active. The four arrows were refused as nameless, which was true and useless.
        (0x2E, BARE, settings::Refusal::Editing, "Delete"),
        (0x24, BARE, settings::Refusal::Editing, "Home"),
        (0x23, BARE, settings::Refusal::Editing, "End"),
        (0x2D, BARE, settings::Refusal::Editing, "Insert"),
        (0x21, BARE, settings::Refusal::Editing, "PageUp"),
        (0x22, BARE, settings::Refusal::Editing, "PageDown"),
        (0x25, BARE, settings::Refusal::Editing, "Left"),
        (0x26, BARE, settings::Refusal::Editing, "Up"),
        (0x27, BARE, settings::Refusal::Editing, "Right"),
        (0x28, BARE, settings::Refusal::Editing, "Down"),
        // ⭐ Task T-93-2, 157.12: and with modifiers — `Shift` + an arrow selects text, `Ctrl` + an
        // arrow walks a word, `Ctrl+Home` goes to the top. The key decides, not the set.
        (0x25, shift, settings::Refusal::Editing, "Shift+Left"),
        (0x27, ctrl, settings::Refusal::Editing, "Ctrl+Right"),
        (0x24, ctrl, settings::Refusal::Editing, "Ctrl+Home"),
        (0x2E, ctrl, settings::Refusal::Editing, "Ctrl+Delete"),
        // And a key with no name has none with a modifier either.
        (0x0D, ctrl, settings::Refusal::Nameless, "Ctrl+Enter"),
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
        settings::Refusal::Emergency,
        settings::Refusal::Reserved,
        settings::Refusal::Editing,
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

/// **Task T-93-2, вопрос 157, устройство А — the capture takes a combination as the key and its
/// set.** `Ctrl`, `Alt` and `Shift` held with a key make the hotkey `Ctrl + F12`: no switch to
/// choose the way first, the field understands what was pressed (157.6). The set is written in the
/// canonical order whatever order the modifiers went down in, and the capture ends on it.
///
/// ⭐ **The red «before» of the task**: on the base `61b23eb` the capture answered
/// `Refused(Combination)` to `Ctrl+F12` (`scratchpad-E93\t93-2-red-before-base.log`).
///
/// ⚠ **A letter with `Ctrl` or `Alt` is taken** — the controller's decision in the TZ, accepted by
/// the owner: `Ctrl+K` types nothing, so taking it costs the typing nothing, and the bare `K`
/// stays with the programs (157.10 holds the sets that *do* type — `Shift`, `AltGr`).
#[test]
fn the_capture_takes_a_combination_as_the_key_and_its_set() {
    let ctrl = settings::Modifiers { ctrl: true, ..BARE };
    let alt = settings::Modifiers { alt: true, ..BARE };
    let shift = settings::Modifiers {
        shift: true,
        ..BARE
    };

    for (vk, held, expected, what) in [
        (0x7B_u16, ctrl, combination(&["Ctrl"], "F12"), "Ctrl+F12"),
        (0x77, alt, combination(&["Alt"], "F8"), "Alt+F8"),
        (0x7B, shift, combination(&["Shift"], "F12"), "Shift+F12"),
        (
            0x13,
            ctrl,
            combination(&["Ctrl"], "Pause"),
            "Ctrl+Pause (the code a test can give)",
        ),
        (
            0x4B,
            ctrl,
            combination(&["Ctrl"], "K"),
            "Ctrl+K — a letter with Ctrl",
        ),
        (
            0x4B,
            settings::Modifiers {
                shift: true,
                ..ctrl
            },
            combination(&["Ctrl", "Shift"], "K"),
            "Ctrl+Shift+K",
        ),
        (
            0x31,
            settings::Modifiers { shift: true, ..alt },
            combination(&["Alt", "Shift"], "1"),
            "Alt+Shift+1",
        ),
        (
            0x7A,
            settings::Modifiers {
                ctrl: true,
                alt: true,
                ..BARE
            },
            combination(&["Ctrl", "Alt"], "F11"),
            "Ctrl+Alt+F11",
        ),
        (
            0x7A,
            settings::Modifiers {
                ctrl: true,
                alt: true,
                shift: true,
                win: false,
            },
            combination(&["Ctrl", "Alt", "Shift"], "F11"),
            "Ctrl+Alt+Shift+F11 — all three, and not the emergency key",
        ),
    ] {
        let taken = settings::capture(vk, held);
        let step = settings::capture_step(true, settings::CaptureEvent::KeyDown(vk, held));

        println!("{what}: {taken:?}");

        assert_eq!(
            taken,
            settings::Capture::Taken(expected.clone()),
            "{what} must be taken as the key and its set"
        );
        assert_eq!(
            step,
            settings::CaptureStep::Take(expected),
            "{what}: and the capture ends on it"
        );
    }

    assert_eq!(
        settings::effective_hotkey_name(&combination(&["Ctrl", "Shift"], "K")),
        "Ctrl + Shift + K",
        "the field shows what was taken in the canonical order"
    );
}

/// **Task T-93-2 — the line under the field at rest, after a combination, and when there is a
/// warning** ([`settings::hotkey_line`]), read in Russian and English out of the built binary.
///
/// 1. At rest the line says what the field takes — «Клавиша или сочетание с Ctrl, Alt, Shift.»
///    (устройство А: the ways stand where they are seen, 157.6).
/// 2. Once after a combination is captured, which half stays with the programs — «F12 без Ctrl
///    остаётся программам.», with the key and the modifiers as the field shows them.
/// 3. A warning of FR-92 wins over both: a name nobody knows is said, not hidden by the hint.
///
/// ⚠ **Task T-95-2 (вопрос 159, the mock-up of 157.18)**: the line at rest names the double press
/// too — «Клавиша, сочетание с Ctrl, Alt, Shift или двойной Shift.» — and after the double press
/// is captured the line says once «Два нажатия Shift подряд. Обычный набор с Shift не затронут.».
#[test]
fn the_line_under_the_field_says_the_ways_at_rest_and_the_bare_key_after_a_combination() {
    let _guard = with_product_strings();

    for (language, rest, after, after_three, after_double) in [
        (
            Language::Ru,
            "Клавиша, сочетание с Ctrl, Alt, Shift или двойной Shift.",
            "F12 без Ctrl остаётся программам.",
            "F11 без Ctrl + Alt + Shift остаётся программам.",
            "Два нажатия Shift подряд. Обычный набор с Shift не затронут.",
        ),
        (
            Language::En,
            "A key, a combination with Ctrl, Alt, Shift, or a double Shift.",
            "F12 without Ctrl stays with your programs.",
            "F11 without Ctrl + Alt + Shift stays with your programs.",
            "Two presses of Shift in a row. Ordinary typing with Shift is not affected.",
        ),
    ] {
        settings::set_ui_language(language);

        assert_eq!(
            settings::hotkey_line(&bare("Pause"), false),
            rest,
            "{language:?}"
        );
        assert_eq!(
            settings::hotkey_line(&combination(&["Ctrl"], "F12"), false),
            rest,
            "{language:?}: a combination that was not just captured shows the line at rest"
        );
        assert_eq!(
            settings::hotkey_line(&combination(&["Ctrl"], "F12"), true),
            after,
            "{language:?}"
        );
        assert_eq!(
            settings::hotkey_line(&combination(&["Shift", "Alt", "Ctrl"], "F11"), true),
            after_three,
            "{language:?}: the modifiers in the canonical order"
        );
        assert_eq!(
            settings::hotkey_line(&bare("F12"), true),
            rest,
            "{language:?}: a bare key has no half to give back"
        );
        assert_eq!(
            settings::hotkey_line(&combination(&["Win"], "F12"), true),
            settings::text(settings::IDS_NOTE_UNKNOWN_KEY),
            "{language:?}: the warning of FR-92 wins"
        );
        assert_eq!(
            settings::hotkey_line(&settings::Hotkey::double_shift(), true),
            after_double,
            "{language:?}: once after the double press is captured"
        );
        assert_eq!(
            settings::hotkey_line(&settings::Hotkey::double_shift(), false),
            rest,
            "{language:?}: and the line at rest after it"
        );
    }

    settings::set_ui_language(Language::Ru);
}

/// **Task T-95-2 — the four lines of the mock-up of 157.18, word for word, in Russian and English,
/// out of the built binary**: the line at rest, the line while the capture waits for the second
/// `Shift`, the line after the double press, and the refusal of `Ctrl` or `Alt` on its own; and
/// «Shift…» in the field, «Shift, Shift» once the pair is taken.
#[test]
fn the_lines_of_the_double_press_are_the_words_of_the_mock_up() {
    let _guard = with_product_strings();

    for (language, hint, again, note, modifier) in [
        (
            Language::Ru,
            "Клавиша, сочетание с Ctrl, Alt, Shift или двойной Shift.",
            "Ещё раз Shift — двойное нажатие; клавиша с Shift — сочетание.",
            "Два нажатия Shift подряд. Обычный набор с Shift не затронут.",
            "Ctrl и Alt — только вместе с клавишей; дважды — только Shift.",
        ),
        (
            Language::En,
            "A key, a combination with Ctrl, Alt, Shift, or a double Shift.",
            "Shift again for a double press; a key with Shift for a combination.",
            "Two presses of Shift in a row. Ordinary typing with Shift is not affected.",
            "Ctrl and Alt go only with a key; only Shift can be pressed twice.",
        ),
    ] {
        settings::set_ui_language(language);

        assert_eq!(settings::text(settings::IDS_HOTKEY_HINT), hint);
        assert_eq!(settings::text(settings::IDS_CAPTURE_SHIFT_AGAIN), again);
        assert_eq!(settings::text(settings::IDS_CAPTURE_DOUBLE_NOTE), note);
        assert_eq!(
            settings::text(settings::Refusal::Modifier.string_id()),
            modifier
        );
    }

    settings::set_ui_language(Language::Ru);

    assert_eq!(settings::pending_shift_name(), "Shift…");
    assert_eq!(
        settings::effective_hotkey_name(&settings::Hotkey::double_shift()),
        "Shift, Shift"
    );
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
        // Task T-95-2: the release of `Shift` and the end of the window of a pair as well.
        settings::CaptureEvent::KeyDown(0x10, NOTHING_HELD),
        settings::CaptureEvent::KeyUp(0x10),
        settings::CaptureEvent::PairTimeout,
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
        settings::CaptureStep::Take(bare("Pause")),
        "a named key is taken"
    );
    assert_eq!(
        settings::capture_step(true, settings::CaptureEvent::KeyDown(0x77, NOTHING_HELD)),
        settings::CaptureStep::Take(bare("F8")),
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
        // ⚠ Task T-93-2: «Ctrl+Pause» stood here as `Combination` — a combination is taken now
        // (`the_capture_takes_a_combination_as_the_key_and_its_set`); a capital is refused in its
        // place (157.10, вариант 1).
        (
            0x4B,
            settings::Modifiers {
                ctrl: false,
                alt: false,
                shift: true,
                win: false,
            },
            settings::Refusal::Text,
            "Shift+K",
        ),
        // ⭐ Task T-36-5, finding Н9, решение 123.3. `Delete` and `Home` were **taken** here
        // until this task — the capture ended, the file got the name, and the person lost that
        // key in every application (FR-95). Now they are refused like any other unsuitable key,
        // and the refusal does not put the capture out (примечание к FR-94): the next press is
        // still waited for.
        (0x2E, NOTHING_HELD, settings::Refusal::Editing, "Delete"),
        (0x24, NOTHING_HELD, settings::Refusal::Editing, "Home"),
        (0x22, NOTHING_HELD, settings::Refusal::Editing, "PageDown"),
        (
            0x25,
            NOTHING_HELD,
            settings::Refusal::Editing,
            "стрелка влево",
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

// -----------------------------------------------------------------------------------------
// Task T-95-2, вопрос 159 — the capture of the double press of `Shift` (the mock-up of 157.18)
// -----------------------------------------------------------------------------------------
//
// The moments of one field, in order: a `Shift` on its own → «Shift…» and «Ещё раз Shift — …»;
// a second within the window → «Shift, Shift», assigned; no second for 400 ms → the invitation
// again, the line staying; a key with `Shift` held → a combination, as in stage 1; `Ctrl` or
// `Alt` on its own → refused with the new line, the capture armed. The machine is
// `settings::CapturePair::step` — pure, with the time of each message written out.

/// The press of `Shift` itself, as the queue reports it: `Shift` held — by itself.
const SHIFT_ITSELF: settings::Modifiers = settings::Modifiers {
    ctrl: false,
    alt: false,
    shift: true,
    win: false,
};

/// `VK_SHIFT`, the code a window message carries for either `Shift`.
const VK_SHIFT_KEY: u16 = 0x10;

/// Drives one armed capture through `events` and answers every step.
fn capture_steps(events: &[(settings::CaptureEvent, u32)]) -> Vec<settings::CaptureStep> {
    let mut pair = settings::CapturePair::default();

    events
        .iter()
        .map(|&(event, time)| pair.step(event, time))
        .collect()
}

/// **The red «before» of task T-95-2 — two taps of `Shift` in a capture assign the double press**:
/// «Shift…» on the first press, the pair waiting after the first release, «Shift…» again on the
/// second press, and «Shift, Shift» taken on the second release. On the base the first press was
/// refused as a modifier on its own.
#[test]
fn two_taps_of_shift_in_a_capture_assign_the_double_press() {
    use settings::CaptureEvent::{KeyDown, KeyUp};
    use settings::CaptureStep::{AwaitShift, Ignore, Take};

    let steps = capture_steps(&[
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
        (KeyUp(VK_SHIFT_KEY), 1_080),
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_200),
        (KeyUp(VK_SHIFT_KEY), 1_280),
    ]);

    assert_eq!(
        steps,
        [
            AwaitShift,
            Ignore,
            AwaitShift,
            Take(settings::Hotkey::double_shift())
        ],
        "«Shift…», waiting, «Shift…», «Shift, Shift»"
    );

    // The window is the hook's own, to the millisecond: a second press 400 ms after the first.
    let edge = capture_steps(&[
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
        (KeyUp(VK_SHIFT_KEY), 1_050),
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_400),
        (KeyUp(VK_SHIFT_KEY), 1_450),
    ]);
    assert_eq!(edge[3], Take(settings::Hotkey::double_shift()));

    // And what is taken is what the file stores, `trigger = "double_tap"`.
    assert_eq!(
        settings::Hotkey::double_shift().binding(),
        Some((0x10, hook::DOUBLE_TAP))
    );
}

/// **The moment 4 — no second `Shift` in time: the invitation again, the line staying, the capture
/// armed** — by the timer of the pair, by the time of a late press, and for a tap held too long.
#[test]
fn a_second_shift_that_does_not_come_in_time_lapses_and_the_capture_stays_armed() {
    use settings::CaptureEvent::{KeyDown, KeyUp, PairTimeout};
    use settings::CaptureStep::{AwaitShift, Ignore, ShiftLapsed, Take};

    // The timer of the pair fires: the field goes back to the invitation.
    let mut pair = settings::CapturePair::default();
    assert_eq!(
        pair.step(KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
        AwaitShift
    );
    assert_eq!(pair.step(KeyUp(VK_SHIFT_KEY), 1_080), Ignore);
    assert!(pair.open(), "the pair waits for its second tap");
    assert_eq!(pair.step(PairTimeout, 1_400), ShiftLapsed);
    assert!(!pair.open(), "and is over");
    assert_eq!(
        pair.step(PairTimeout, 1_800),
        Ignore,
        "a stale tick finds nothing"
    );

    // The capture is armed: the next taps are a pair of their own.
    assert_eq!(
        pair.step(KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 2_000),
        AwaitShift
    );
    assert_eq!(pair.step(KeyUp(VK_SHIFT_KEY), 2_050), Ignore);
    assert_eq!(
        pair.step(KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 2_150),
        AwaitShift
    );
    assert_eq!(
        pair.step(KeyUp(VK_SHIFT_KEY), 2_200),
        Take(settings::Hotkey::double_shift())
    );

    // A second press later than the window, before the tick has come: the first of a new pair.
    let late = capture_steps(&[
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
        (KeyUp(VK_SHIFT_KEY), 1_050),
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_401),
        (KeyUp(VK_SHIFT_KEY), 1_450),
    ]);
    assert_eq!(
        late,
        [AwaitShift, Ignore, AwaitShift, Ignore],
        "401 ms: no pair"
    );

    // A tap held too long — the tick while it is still held, or the release past the window.
    let held = capture_steps(&[
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
        (PairTimeout, 1_400),
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_430),
        (KeyUp(VK_SHIFT_KEY), 1_500),
    ]);
    assert_eq!(
        held,
        [AwaitShift, ShiftLapsed, Ignore, Ignore],
        "held past the window: the invitation again, the repeat and the release are nothing"
    );

    let slow_release = capture_steps(&[
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
        (KeyUp(VK_SHIFT_KEY), 1_401),
    ]);
    assert_eq!(slow_release, [AwaitShift, ShiftLapsed]);
}

/// **A key with `Shift` held is a combination, as in stage 1** — the pair gives way to it: `Shift +
/// F12` is taken, `Shift + K` (a capital) is refused as before, and the pair is over either way.
#[test]
fn a_key_with_shift_held_is_a_combination_as_in_stage_one() {
    use settings::CaptureEvent::{KeyDown, KeyUp};
    use settings::CaptureStep::{AwaitShift, Ignore, Refuse, Take};

    assert_eq!(
        capture_steps(&[
            (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
            (KeyDown(0x7B, SHIFT_ITSELF), 1_100),
        ]),
        [AwaitShift, Take(combination(&["Shift"], "F12"))]
    );

    let mut pair = settings::CapturePair::default();
    assert_eq!(
        pair.step(KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
        AwaitShift
    );
    assert_eq!(
        pair.step(KeyDown(0x4B, SHIFT_ITSELF), 1_050),
        Refuse(settings::Refusal::Text)
    );
    assert!(!pair.open(), "a refused key ends the pair");
    assert_eq!(pair.step(KeyUp(VK_SHIFT_KEY), 1_100), Ignore);
    assert_eq!(
        pair.step(KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_200),
        AwaitShift,
        "the next Shift begins a pair of its own"
    );
}

/// **The moment 6 — `Ctrl` or `Alt` on its own is refused, and the capture stays armed**; so is a
/// `Shift` pressed under them (`Ctrl+Shift`, `Alt+Shift` switch layouts), and a `Ctrl` in the
/// middle of an open pair ends the pair.
#[test]
fn ctrl_or_alt_on_its_own_is_refused_and_a_pair_under_them_is_none() {
    use settings::CaptureEvent::{KeyDown, KeyUp};
    use settings::CaptureStep::{AwaitShift, Ignore, Refuse};

    let ctrl = settings::Modifiers {
        ctrl: true,
        ..NOTHING_HELD
    };
    let alt = settings::Modifiers {
        alt: true,
        ..NOTHING_HELD
    };

    for (vk, held, what) in [
        (0x11_u16, ctrl, "Ctrl"),
        (0xA2, ctrl, "left Ctrl"),
        (0x12, alt, "Alt"),
        (0xA5, alt, "right Alt"),
        (
            VK_SHIFT_KEY,
            settings::Modifiers {
                shift: true,
                ..ctrl
            },
            "Shift under Ctrl",
        ),
        (
            VK_SHIFT_KEY,
            settings::Modifiers { shift: true, ..alt },
            "Shift under Alt",
        ),
    ] {
        assert_eq!(
            capture_steps(&[(KeyDown(vk, held), 1_000)]),
            [Refuse(settings::Refusal::Modifier)],
            "{what}"
        );
    }

    let ctrl_in_the_middle = capture_steps(&[
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
        (KeyUp(VK_SHIFT_KEY), 1_050),
        (KeyDown(0x11, ctrl), 1_100),
        (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_200),
        (KeyUp(VK_SHIFT_KEY), 1_250),
    ]);
    assert_eq!(
        ctrl_in_the_middle,
        [
            AwaitShift,
            Ignore,
            Refuse(settings::Refusal::Modifier),
            AwaitShift,
            Ignore
        ],
        "a Ctrl between the taps: refused, and the next Shift is a first tap"
    );
}

/// **The auto-repeat of a held `Shift` is no second tap** — the queue repeats a held key as a run
/// of presses — and the left and the right `Shift` are one key (the queue names both `VK_SHIFT`).
#[test]
fn the_auto_repeat_of_a_held_shift_is_no_second_tap() {
    use settings::CaptureEvent::{KeyDown, KeyUp};
    use settings::CaptureStep::{AwaitShift, Ignore, Take};

    assert_eq!(
        capture_steps(&[
            (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
            (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_030),
            (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_060),
            (KeyUp(VK_SHIFT_KEY), 1_090),
            (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_200),
            (KeyUp(VK_SHIFT_KEY), 1_250),
        ]),
        [
            AwaitShift,
            Ignore,
            Ignore,
            Ignore,
            AwaitShift,
            Take(settings::Hotkey::double_shift())
        ]
    );

    // A release of another key changes nothing; a release of `Shift` never seen going down either.
    assert_eq!(
        capture_steps(&[
            (KeyUp(0x41), 1_000),
            (KeyUp(VK_SHIFT_KEY), 1_010),
            (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_100),
            (KeyUp(0x41), 1_120),
            (KeyUp(VK_SHIFT_KEY), 1_150),
        ]),
        [Ignore, Ignore, AwaitShift, Ignore, Ignore]
    );
}

/// **Escape, a click beside and a lost focus end an open pair with the capture** — the way out is
/// the way out whatever the field says.
#[test]
fn escape_a_click_and_a_lost_focus_end_an_open_pair_with_the_capture() {
    use settings::CaptureEvent::{ClickBeside, FocusLost, KeyDown, KeyUp};
    use settings::CaptureStep::{AwaitShift, Cancel, Ignore};

    for way_out in [
        KeyDown(0x1B, NOTHING_HELD),
        ClickBeside,
        FocusLost {
            to_capture_button: false,
        },
    ] {
        assert_eq!(
            capture_steps(&[
                (KeyDown(VK_SHIFT_KEY, SHIFT_ITSELF), 1_000),
                (KeyUp(VK_SHIFT_KEY), 1_050),
                (way_out, 1_100),
            ]),
            [AwaitShift, Ignore, Cancel],
            "{way_out:?}"
        );
    }
}

/// **Task T-95-2, посылка П9 — a capture takes the double press off the hook with the key.** While
/// a capture is armed the word of the hotkey is the code no keyboard makes, and the flag of the
/// double press went down with it: two taps of `Shift` made while choosing a hotkey convert
/// nothing. Dropping the session puts the whole word back.
#[test]
fn a_capture_takes_the_double_press_off_the_hook_and_gives_it_back() {
    let _guard = with_product_strings();

    let taps = |mode: hook::Mode| {
        let mut state = hook::HotkeyState::default();

        [
            (0xA0_u16, hook::Edge::Down, 1_000_u32),
            (0xA0, hook::Edge::Up, 1_080),
            (0xA0, hook::Edge::Down, 1_200),
            (0xA0, hook::Edge::Up, 1_280),
        ]
        .into_iter()
        .filter(|&(vk, edge, time)| {
            hook::classify(
                mode,
                &mut state,
                hook::KeyEvent {
                    vk,
                    edge,
                    extra_info: 0,
                    scan: 0,
                    flags: 0,
                    time,
                },
                || 0,
                || 0,
            )
            .fire_hotkey
        })
        .count()
    };

    let before_word = hook::hotkey();
    hook::set_hotkey(0x10, hook::DOUBLE_TAP);
    hook::set_active(true);

    let published = hook::current_mode();
    assert!(published.hotkey_double_tap);
    assert_eq!(
        taps(published),
        1,
        "the control: the published double press fires"
    );

    let session = settings::CaptureSession::arm(settings::Hotkey::double_shift());
    let during = hook::current_mode();

    assert_eq!(
        hook::hotkey(),
        (0, 0),
        "the whole word is the code no keyboard makes"
    );
    assert!(!during.hotkey_double_tap, "the flag went down with the key");
    assert_eq!(
        taps(hook::Mode {
            active: true,
            ..during
        }),
        0,
        "two taps made while choosing a hotkey convert nothing"
    );

    drop(session);

    assert_eq!(
        hook::hotkey(),
        (0x10, hook::DOUBLE_TAP),
        "the whole word comes back"
    );
    assert!(hook::current_mode().hotkey_double_tap);

    hook::set_hotkey(before_word.0, before_word.1);
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
        settings::Capture::Taken(bare("F12"))
    );

    // ⭐ Task T-93-2: and since combinations are taken, every set **short of** all three is a
    // lawful hotkey on the same key — only the exact emergency set is refused.
    for (held, expected) in [
        (
            settings::Modifiers {
                ctrl: true,
                alt: true,
                shift: false,
                win: false,
            },
            combination(&["Ctrl", "Alt"], "F12"),
        ),
        (
            settings::Modifiers {
                ctrl: true,
                alt: false,
                shift: true,
                win: false,
            },
            combination(&["Ctrl", "Shift"], "F12"),
        ),
        (
            settings::Modifiers {
                ctrl: false,
                alt: true,
                shift: true,
                win: false,
            },
            combination(&["Alt", "Shift"], "F12"),
        ),
    ] {
        assert_eq!(
            settings::capture(hook::EMERGENCY_VK, held),
            settings::Capture::Taken(expected)
        );
    }
}

#[test]
fn a_capture_stops_the_conversion_and_gives_it_back_when_it_ends() {
    // **Criterion 15.** While a capture is armed the callback is published as inactive, which
    // `hook::classify` answers by passing every stroke through: no conversion is fired, nothing
    // is suppressed, and nothing reaches the typing buffer either. The proof is the callback's
    // own decision function, called with the mode the callback would really see.
    let _guard = with_product_strings();

    hook::set_active(true);
    hook::set_hotkey(hook::DEFAULT_HOTKEY_VK, 0);

    let press = hook::KeyEvent {
        vk: hook::DEFAULT_HOTKEY_VK,
        edge: hook::Edge::Down,
        extra_info: 0,
        scan: 0,
        flags: 0,
        time: 0,
    };

    // Task T-93-1: the decision takes the held modifiers as well; this press holds none.
    let before = hook::classify(
        hook::current_mode(),
        &mut hook::HotkeyState::default(),
        press,
        || 0,
        || 0,
    );
    println!("before the capture: {before:?}");
    assert!(before.fire_hotkey, "the hotkey fires when nothing is armed");

    {
        let session = settings::CaptureSession::arm(bare("Pause"));

        assert_eq!(session.previous(), &bare("Pause"));
        assert_eq!(session.published(), (hook::DEFAULT_HOTKEY_VK, 0));
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
            || 0,
            || 0,
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
            || 0,
            || 0,
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
    // publishes the hotkey code back. ⚠ **And only it, since task T-36-6:** the active flag is
    // whatever was published last, which here is the `true` set under the capture just above.
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
        || 0,
        || 0,
    );
    println!("after the capture: {after:?}");
    assert!(after.fire_hotkey);

    // A capture armed while the program was suspended gives back "suspended" and not "active".
    hook::set_active(false);
    drop(settings::CaptureSession::arm(bare("Pause")));
    assert!(!hook::is_active(), "a suspended program stays suspended");

    // ⭐ **Task T-36-6, finding Н116, решение 123.1 — a capture does not put back a stale flag.**
    // The session used to remember `hook::is_active()` at arming and write it back in `Drop`. That
    // copy went stale immediately: `app::window_proc` re-publishes `tray.enabled()` after every
    // message the UI thread sees, so a user who suspended the program *during* a capture — through
    // the tray menu of FR-91, which is what the UI thread is doing all the while — had the capture
    // switch the program back on under them when it ended. Here the state moves the other way
    // round while the session is alive, and what must survive the `Drop` is the **new** value.
    hook::set_active(true);

    let session = settings::CaptureSession::arm(bare("Pause"));

    hook::set_active(false);
    drop(session);

    assert!(
        !hook::is_active(),
        "Н116: the flag published while the capture was alive is the one that stands after it — \
         the session gives back the hotkey code and nothing else"
    );
    assert_eq!(
        hook::hotkey_vk(),
        hook::DEFAULT_HOTKEY_VK,
        "and the hotkey code is still given back, which is what the suspension really rests on"
    );

    hook::set_active(true);
}

#[test]
fn the_captured_key_reaches_the_hook_through_publish_configuration() {
    // **Criteria 13, 18 and 25.** The dialog publishes nothing itself: it puts the captured name
    // into the configuration and «Применить» hands that configuration to
    // `app::publish_configuration`, which is the one caller of `hook::set_hotkey` there has ever
    // been (named `set_hotkey_vk` until task T-93-1 put the modifiers in the same store).
    let _guard = with_product_strings();
    let _publishing = publishing();

    let mut config = Config::default();

    let settings::Capture::Taken(captured) = settings::capture(0x77, BARE) else {
        panic!("F8 with nothing held must be capturable");
    };

    config.hotkey = captured.clone();
    app::publish_configuration(&config);

    println!(
        "captured {captured:?} -> hook::hotkey() = {:?}",
        hook::hotkey()
    );

    assert_eq!(captured, bare("F8"));
    assert_eq!(hook::hotkey(), (0x77, 0));

    // ⭐ Task T-93-2 — a combination travels the same road, the set with the key in one
    // publication.
    let settings::Capture::Taken(captured) =
        settings::capture(0x77, settings::Modifiers { ctrl: true, ..BARE })
    else {
        panic!("Ctrl+F8 must be capturable");
    };

    config.hotkey = captured.clone();
    app::publish_configuration(&config);

    println!(
        "captured {captured:?} -> hook::hotkey() = {:?}",
        hook::hotkey()
    );

    assert_eq!(hook::hotkey(), (0x77, hook::MOD_CTRL));

    // And back to the default of section 7, so that this test cannot leave the process with a
    // hotkey another test did not ask for.
    config.hotkey = bare("Pause");
    app::publish_configuration(&config);
    assert_eq!(hook::hotkey(), (hook::DEFAULT_HOTKEY_VK, 0));
}

// ---------------------------------------------------------------------------------------
// FR-92а — the system theme changing under the open dialog. Task T-11-9.
// ---------------------------------------------------------------------------------------

use lang_switcher::theme::Palette;

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
// FR-92а — the record of the open «О программе» window.
// ---------------------------------------------------------------------------------------
//
// «О программе» is a window of the shell (`ui::windows::about`). The record of it locks the
// menu and keeps a second copy out; what is measurable without a live window is the record and
// the shape — who writes it down, who clears it, and that the road of the system theme does not
// reach it. The window itself is opened, hidden, by `tests\tray.rs`.

use lang_switcher::settings::AboutSession;
use windows::Win32::Foundation::HWND;

/// A handle that names no window, for the record tests below.
///
/// Deliberately not a live window: what is under test is the *record* — an integer this module
/// writes down and clears. Nothing is ever sent to this number.
fn a_handle_that_names_no_window() -> HWND {
    HWND(0x1357 as *mut std::ffi::c_void)
}

/// **Criterion 6 of task T-13-17, and ловушка 1 of it.** The window of «О программе» is
/// registered while it lives and unregistered on the way out — whatever the way out is.
///
/// The ordinary road and the panic are both walked here, because the guard exists precisely so
/// that they are one road: the window is closed by a button, by `Esc`, by the cross of the
/// caption, by the end of the program, and the host drops its content — the guard with it — on
/// every one of them.
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
            "the guard alone names no window — there is none until the host has made it"
        );

        AboutSession::record(a_handle_that_names_no_window());

        assert!(
            settings::about_is_open(),
            "a recorded window is the whole of what «about жив» means"
        );
    }

    assert!(
        !settings::about_is_open(),
        "the record must not outlive the content that owns the guard"
    );

    // The other road. The program stays up after a panic (FR-98, FR-99), and a record left
    // behind would lock the menu for ever and name a number Windows is free to give to
    // somebody else's window.
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

/// **Criterion 1 of task T-13-17.** The record has exactly two writers — the one that names the
/// window and the one that forgets it — and `about::open` records the window only once the host
/// has made it, with the guard already in the content the host owns.
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

    let about = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("ui")
            .join("windows")
            .join("about.rs"),
    )
    .expect("src\\ui\\windows\\about.rs must be readable")
    .replace("\r\n", "\n");
    let open = function_body(&about, "pub fn open(");
    let guard = open
        .find("_session: AboutSession::open(),")
        .expect("`about::open` must put the guard into the content");
    let made = open
        .find("host::open(product, Some(owner))")
        .expect("`about::open` must make the window by the host");
    let recorded = open
        .find("AboutSession::record(window);")
        .expect("`about::open` must record the window it made");

    assert!(
        guard < made && made < recorded,
        "the guard goes into the content before the window is made, and the window is recorded \
         once it is"
    );
}

/// **Task T-43-4, finding С21 — the door at the top of `about::open`.** While «О программе» or
/// the settings dialog is up, a second «О программе» turns round at the door: nothing is made,
/// and the record is left as it was. The menu greys and refuses the entry for the same reason
/// (`tray::dialog_locks_command`); this is the half behind it.
///
/// The positive control — with neither window up the very same road makes the window — is
/// `tests\tray.rs`, which opens it, hidden, by the command of the menu.
#[test]
fn a_second_about_window_turns_round_at_the_door_before_anything_is_made() {
    use lang_switcher::theme::ThemeSetting;
    use lang_switcher::ui::windows::about;

    let owner = a_handle_that_names_no_window();

    assert!(
        !settings::about_is_open() && !settings::dialog_is_open(),
        "this thread has neither window"
    );

    // An «О программе» window is up.
    {
        let _open = AboutSession::open();
        AboutSession::record(a_handle_that_names_no_window());

        let second = about::open(owner, ThemeSetting::Dark, &bare("Pause"));

        println!("over an about window: {second:?}");

        assert_eq!(
            second, None,
            "a second «О программе» over the first must turn round at the door"
        );
        assert_eq!(
            settings::open_window(),
            Some(a_handle_that_names_no_window()),
            "and the record still names the first"
        );
    }

    // The settings dialog is up.
    {
        let _dialog = settings::DialogSession::open();

        let over = about::open(owner, ThemeSetting::Dark, &bare("Pause"));

        println!("over the settings dialog: {over:?}");

        assert_eq!(
            over, None,
            "«О программе» over the settings dialog must turn round at the door as well"
        );
        assert!(!settings::about_is_open(), "and nothing was recorded");
    }

    assert!(
        !settings::about_is_open() && !settings::dialog_is_open(),
        "and both guards are gone again"
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

/// The dialog font of the settings template and a sheet to measure it on.
fn template_font_and_sheet() -> (TemplateFont, Sheet) {
    let product = ProductImage::open();
    // Every template of the program declares the one face, FONT 10 «Segoe UI»; «От автора»'s is read.
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, letters::IDD_AUTHOR));

    let font = template
        .font
        .clone()
        .expect("the template of «От автора» declares DS_SETFONT");

    (font, Sheet::new(64))
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

// -----------------------------------------------------------------------------------------
// Геометрия диалога: коробка поля, разведённые прямоугольники, список вровень с кнопкой —
// задача T-12-3
// -----------------------------------------------------------------------------------------
//
// Four defects of the Э12 comparison, and every one of them is arithmetic on the template and
// on one pure function — no window is created and the product is not started. What the pixels
// then do is the controller's half, on the stand.

// =========================================================================================
// Э12 волна 2 — межстрочие многострочных подсказок. Task T-12-12, решение В-6.
// =========================================================================================
//
// The pixels below are drawn on the memory sheet of [`Sheet`] with the settings template's own
// face selected into it: no window is created, no message is pumped and **the product is not
// started**. The face matters here in a way it did not for the T-11-18 tests — the whole
// subject is the distance between two lines, and that distance is measured against the face's
// own `tmHeight`.

use windows::Win32::System::Threading::{GR_GDIOBJECTS, GetCurrentProcess, GetGuiResources};

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

/// Every value of the closed set: the word section 7 stores, the `LANGID` `app.rc` tags the
/// string table with, and the name the language combo box shows — the fourteen of 1.0.0 and the
/// thirty of group A.
///
/// ⭐ **Read out of the registry `res\lang\languages.toml` since stage E108** (task T-108-2, «таблицы
/// тестов — на реестр»): until then the fourteen rows were written out here. What a test must not
/// borrow from the code under test is still the **answer**, and the registry is data apart from
/// the code — `tests\l10n.rs` holds `Language` to it value by value, and its `LANGID`s were
/// measured there (`LocaleNameToLCID` and back, 69 of 69). The order is the order of the registry
/// and of `Language::ALL`.
fn all_locales() -> Vec<(String, u16, String)> {
    registry::locales(&registry::closed_set())
}

/// The locales the build offers — the entries of the registry with `state = "shipped"`: the ones
/// whose string tables are in the built binary. A sweep of the strings walks these.
fn shipped_locales() -> Vec<(String, u16, String)> {
    registry::locales(&registry::shipped())
}

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

/// **Т-28-2, Т-30-1: all values of `general.language` survive the file, both ways** — the
/// forty-four since stage E108, `sr-Latn` and `uz-Cyrl` with their script.
///
/// Round trip and not just a read: the value has to come back out of `to_toml_string` spelled
/// the same way it went in, or a build that merely *reads* fourteen values would write two.
#[test]
fn every_one_of_the_language_values_survives_a_round_trip() {
    let locales = all_locales();
    assert_eq!(
        locales.len(),
        Language::ALL.len(),
        "the registry names every value of the closed set"
    );

    for (tag, _, _) in locales {
        let tag = tag.as_str();
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
    for (tag, langid, _) in all_locales() {
        assert_eq!(
            language_of_tag(&tag).langid(),
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

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
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
    // Stage E119, task T-119-8 (решение 208.94 п. 4): «Что нового» and «обновлён до X»; task
    // T-119-10 (решение 208.119): the switch of the feed and the line under it (3123, 3124); task
    // T-119-9 (решение 208.99): the balloon of «Обновление» and its two old words (3137, 3138, 3140,
    // 3143).
    const RETIRED_E119: [u16; 17] = [
        3102, 3103, 3104, 3105, 3106, 3107, 3108, 3109, 3212, 3249, 3250, 3123, 3124, 3137, 3138,
        3140, 3143,
    ];

    assert!(
        !settings::INTERFACE_STRINGS.contains(&RETIRED),
        "the vocabulary of the interface must not name a retired identifier"
    );
    // ⛔⛔ **3111 is a second hole since task T-81-3** — решение 142.2 п. 5, and the first time
    // this canon has gone **down**. `IDS_AUTHOR_VERSION` carried «версия {0} · Панда, он же
    // Panda_Pishet_Kod» under the name of the program in «От авторе»; the owner asked for the
    // signature to leave, the head of that window became the head of «О программе» — version
    // line included — and the string went with all fourteen of its translations. The hole is
    // held by the same three checks 3004 is held by, a few lines down.
    assert!(
        !settings::INTERFACE_STRINGS.contains(&3111),
        "3111 left the vocabulary with task T-81-3 and must not come back into it"
    );
    // ⚠ **3041 is the third hole since task T-93-2** — вопрос 157: `IDS_CAPTURE_COMBINATION`
    // («Сочетания с модификаторами не поддерживаются…») left with the refusal it said when a
    // combination became a hotkey. Held by the same three checks.
    assert!(
        !settings::INTERFACE_STRINGS.contains(&3041),
        "3041 left the vocabulary with task T-93-2 and must not come back into it"
    );
    // ⚠ **Eight more holes since stage E107, task T-107-3** — решения 199.3 и 199.10: the button
    // that opened the group or the issues and its two answers (3202, 3204, 3205), the two thanks
    // (3208…3211) and the note of the letter of a trouble (3243). Held by the same three checks.
    for hole in [3202, 3204, 3205, 3208, 3209, 3210, 3211, 3243] {
        assert!(
            !settings::INTERFACE_STRINGS.contains(&hole),
            "{hole} left the vocabulary with task T-107-3 and must not come back into it"
        );
    }
    // ⛔ **Eleven more holes since stage E119, task T-119-8** — решение 208.94 п. 4: the letter
    // «Что нового» with its eight words (3102…3109) and its balloon (3212), and the balloon
    // «обновлён до X» with its two (3249, 3250). Two more since task T-119-10 (решение 208.119):
    // the switch of the feed in «От автора» and the line under it (3123, 3124). Four more since
    // task T-119-9 (решение 208.99): the balloon of «Обновление» and two of its old words (3137,
    // 3138, 3140, 3143). Held by the same three checks.
    for hole in RETIRED_E119 {
        assert!(
            !settings::INTERFACE_STRINGS.contains(&hole),
            "{hole} left the vocabulary with task T-119-8, T-119-9 or T-119-10 and must not come back into it"
        );
    }
    assert_eq!(
        settings::INTERFACE_STRINGS.len(),
        265,
        "two hundred and SIXTY-FIVE identifiers in use since task T-120-20: the note of a refused \
         write of the settings (3303) and the badge «новое» (3304). Two hundred and sixty-three since the texts of «Привет» (T-120-16): the line under its \
         body (3294). Two hundred and sixty-two since the header of the window in the corner \
         that knocks with a letter (3293). Two hundred and sixty-one since task T-119-9 (решения 208.95–208.99): \
         the balloon of «Обновление» left (3137, 3138), and two of its old words whose meaning \
         changed (3140, 3143); the honest letter came with six (3297…3302). Two hundred and \
         fifty-nine since task T-119-10 (решение 208.119): the \
         switch of the news in «От автора» left with its two strings (3123, 3124), the note «Новости \
         автора выключены в файле настроек» came (3296). Two hundred and sixty since tasks \
         T-119-5, -6, -7: the window in the corner, the entry «Что нового в X…» and the switch of \
         «От автора» (3267…3279). Two hundred and forty-seven since task T-119-3, part Б (решение \
         208.88): the title and the link of the list of the program's own (3265, 3266). Two \
         hundred and forty-five since stage E120, tasks T-120-21 and \
         T-120-22 (решение 208.169): «Состояние» for a person, nine (3280…3288), and «Нашли \
         ошибку в переводе?», four (3289…3292). Two hundred and thirty-two since task T-119-2 \
         (FR-90б): the check box \
         of the flag of the layout (3264). Two hundred and thirty-one since stage E119, task T-119-8 (решение \
         208.94 п. 4): the letter «Что нового» and the balloon «обновлён до X» left with eleven \
         strings. Two hundred and forty-two since stage E114, task T-114-2 (решение \
         208.5, 6а): the sixth row of the help, 3263. Two hundred and forty-one since stage \
         E107, task T-107-3 (решения \
         199.3, 199.10, 199.11): the appeal goes to the site — nine arrived (3254…3262), eight \
         left. Two hundred and forty since stage L10N, task T-L10N-10 (решения \
         200.10, 200.10а): the separator of a list, the arrow of a cycle and the form of the \
         tooltip left the code for the tables (3251…3253). Two hundred and thirty-seven since \
         task T-104-7 (TZ E104 §0 п. 5 и \
         12): the program updates itself, and the balloon after an update got its two words \
         (3249, 3250). Two hundred and thirty-five since task T-103-7 (решение 181.6): an \
         appeal about a trouble goes to the author by letter, and its last step got six words of \
         its own (3243…3248). Two hundred and twenty-nine since task T-103-2 (вопрос 181, решение \
         181.1): «Спасибо» sends a person where they can write, and its button got a word of its \
         own (3242). Two hundred and twenty-eight since task T-100-3 (вопрос 172, \
         вариант В): the eight strings of the two ways of «Исключения» arrived (3234…3241) — \
         the two words, the button at rest and counting, the line under it at rest, «Добавлено», \
         «Уже в списке», «Окно не выбрано». Two hundred and twenty since task T-95-2 (вопрос \
         159): the line while a capture waits for the second Shift and the line after the double \
         press arrived (3232, 3233). Two hundred and eighteen since task T-93-2 (вопрос 157): \
         the refusal of combinations (3041) left and the two lines under the hotkey field \
         arrived (3230, 3231). Two hundred and seventeen before it; two hundred and eighteen \
         until task \
         T-81-3 (решение 142.2 п. 5) took `IDS_AUTHOR_VERSION` away, which is the first time \
         this canon has moved downwards. Before that: the mandate of Э32 authorised the canon \
         of seventy-three away («канон INTERFACE_STRINGS растёт с 73»), and the growth is the \
         sixty-one strings of the letters from the author (FR-101…FR-103, task Т-32-3), the \
         ten of the two letters out of the feed (Т-32-6), the fifty-nine of the wizard \
         (FR-104, Т-32-8), the ONE the balloon of «Что нового» gained by задача Т-33а-4 — \
         its own body, because it was knocking with the words of the update — the TWO of \
         task T-34-3 (the button «Сохранить журнал» and the sentence of its refusal), and \
         the SEVEN of task T-34-5 less the THREE it retired (3045…3047): the four states of \
         the program, the health of the hook, their joiner and the greyed «Возобновить» — and \
         the FIVE of task T-34-6: the acting pair, the acting cycle, no pair, no refusals and \
         the refusals counted — and the ONE of task T-39-11, решение 122.5: «Журнал сохранён» \
         on the button after a write that succeeded — and the ONE of task T-36-5, решение \
         123.3: «Эта клавиша нужна при наборе», the seventh reason a capture refuses (Н9) — \
         and the ONE of task T-43-9, решение 128.2: «Lang Switcher уже работает в этом \
         сеансе», the notification of FR-82 that was an English literal in app.rs until then \
         and is now read in the language of Windows (Н23)"
    );

    let product = ProductImage::shared();
    let mut still_there: Vec<String> = Vec::new();
    let mut empty_neighbours: Vec<String> = Vec::new();
    let mut read = 0;

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
        // ⚠ **Both holes, since task T-81-3.** 3111 is checked on exactly the terms 3004 is: it
        // has to be gone from the resource and not only from the crate's list, and its block has
        // to read across it. 3111 sits in the block 3104..3119, so 3110 and 3112 are its
        // neighbours and 3119 is the last row of that block.
        for retired_id in [RETIRED, 3111, 3041].into_iter().chain(RETIRED_E119) {
            let retired = product.string_of_langid(langid, retired_id);

            if !retired.trim().is_empty() {
                still_there.push(format!(
                    "{tag} ({langid:#06X}) still carries {retired_id} «{retired}»"
                ));
            }
        }

        // The two neighbours of each hole and the last row of each sixteen-string block — 3041
        // (task T-93-2) sits in the block 3040..3055.
        // Since task T-119-8: 3101 before the run 3102…3109, 3213 and 3215 round 3212, 3248, 3251
        // and 3263 round 3249…3250 (3110 after the run is read above). Since task T-119-10: 3122
        // and 3125 round 3123…3124 and 3135, the last row of the block 3120…3135. Since task
        // T-119-9: 3136 and 3139 round 3137…3138, 3141 after 3140, 3142 and 3144 round 3143, and
        // 3151, the last row of the block 3136…3151.
        for id in [
            3003_u16, 3005, 3015, 3110, 3112, 3119, 3040, 3042, 3055, 3101, 3213, 3215, 3248, 3251,
            3263, 3122, 3125, 3135, 3136, 3139, 3141, 3142, 3144, 3151,
        ] {
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
        shipped_locales().len() * 24,
        "twenty-four rows in each locale — six round the holes of task T-119-9, three round the holes of task T-119-10, six round the holes of task T-119-8, and nine, three round each of the first three holes (the \
         third, 3041, since task T-93-2); «no holes» out of no readings is the cheapest lie such \
         a sweep tells"
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

/// The last schema a build that knew **fourteen** locales stamped a file with — 1.2.0 and every
/// build since 0.95.0 (`[hotkey] trigger`, вопрос 159).
const LAST_FOURTEEN_LOCALE_SCHEMA: u32 = 8;

/// **Stage E108, task T-108-2 — the coin of вопрос 94.1 spent a third time: a file naming a
/// language of group A carries a stamp no fourteen-locale build wrote.**
///
/// The mechanism is the one the two tests above hold (a stamp too new → read, never written, no
/// `.bad`); what this stage adds is the number, and this test pins it on the real values: every
/// one of the thirty, written by this build, carries `schema_version = 9` — above the 8 of 1.2.0 —
/// and reads back as current. ⚠ The `const` block makes lowering the schema a build that does not
/// compile.
#[test]
fn a_file_of_group_a_carries_a_stamp_no_fourteen_locale_build_wrote() {
    const {
        assert!(
            CURRENT_SCHEMA_VERSION > LAST_FOURTEEN_LOCALE_SCHEMA,
            "a file naming a language of group A must be stamped with a schema no build of \
             fourteen had — without it 1.2.0 quarantines the file"
        );
    }

    let dir = TestDir::new("group_a_stamp");
    let group_a: Vec<Language> = Language::ALL.into_iter().skip(14).take(30).collect();
    assert_eq!(group_a.len(), 30, "the thirty of group A");

    for language in group_a {
        let mut ours = Config::default();
        ours.general.language = language;

        let text = ours.to_toml_string().expect("the file must serialise");

        assert!(
            text.contains(&format!("language = \"{}\"", language.tag()))
                && text.contains(&format!("schema_version = {CURRENT_SCHEMA_VERSION}")),
            "{language:?}: the word and the stamp are in the file: {text}"
        );

        let path = write_file(&dir, &text);
        let (back, outcome) = settings::read_or_default(&path);

        assert_eq!(outcome.as_ref().ok(), Some(&ReadOutcome::Current));
        assert_eq!(back.general.language, language, "{language:?} reads back");
    }
}

/// The last schema a build that knew **forty-four** locales stamped a file with — 1.3.0 (stage
/// E108, the thirty languages of group A).
const LAST_FORTY_FOUR_LOCALE_SCHEMA: u32 = 9;

/// **Stage E109, task T-109-1 — the coin of вопрос 94.1 spent a fourth time: a file naming a
/// language of group B carries a stamp no build of forty-four wrote.**
///
/// The test of group A above, on the twenty-two: every one of them, written by this build, carries
/// `schema_version = 10` — above the 9 of 1.3.0 — and reads back as current. ⚠ The `const` block
/// makes lowering the schema a build that does not compile.
#[test]
fn a_file_of_group_b_carries_a_stamp_no_forty_four_locale_build_wrote() {
    const {
        assert!(
            CURRENT_SCHEMA_VERSION > LAST_FORTY_FOUR_LOCALE_SCHEMA,
            "a file naming a language of group B must be stamped with a schema no build of \
             forty-four had — without it 1.3.0 quarantines the file"
        );
    }

    let dir = TestDir::new("group_b_stamp");
    let group_b: Vec<Language> = Language::ALL.into_iter().skip(44).collect();
    assert_eq!(group_b.len(), 22, "the twenty-two of group B");

    for language in group_b {
        let mut ours = Config::default();
        ours.general.language = language;

        let text = ours.to_toml_string().expect("the file must serialise");

        assert!(
            text.contains(&format!("language = \"{}\"", language.tag()))
                && text.contains(&format!("schema_version = {CURRENT_SCHEMA_VERSION}")),
            "{language:?}: the word and the stamp are in the file: {text}"
        );

        let path = write_file(&dir, &text);
        let (back, outcome) = settings::read_or_default(&path);

        assert_eq!(outcome.as_ref().ok(), Some(&ReadOutcome::Current));
        assert_eq!(back.general.language, language, "{language:?} reads back");
    }
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
    // Task T-55-9, finding С20: the defaults that came back in its place speak the language of the
    // system now, not the hard `ru` of section 7. ⚠ On a Russian Windows the two are the same word.
    assert_eq!(
        config.general.language,
        settings::system_language_for_a_first_run(),
        "what came back in its place is the configuration of a first run"
    );
}

/// **Т-28-2, Т-30-1: every value of the closed set has its native name and its position.**
///
/// Three things at once, and they are one contract: the name shown, the position it stands at,
/// and the value that position reads back as. A build that showed «Deutsch» at position three
/// and stored Ukrainian for it would pass any two of the three checks separately. Since stage
/// E108 (решение 202.7) the combo no longer shows the array in its order — that is
/// `the_language_combo_puts_the_group_of_the_person_on_top_and_the_rest_by_name` — and the
/// position is the number of the atomic of `set_ui_language` alone; the file holds the tag.
///
/// ⚠ **The one place the count itself is a literal.** Everything else takes its length from
/// `Language::ALL`; here the number is the thing under test, so it is typed out — twelve of
/// решение 93, the two of вопрос 97 and the thirty of group A (stage E108), the twenty-two of group B
/// (stage E109).
#[test]
fn every_value_of_the_closed_set_has_its_name_and_its_position() {
    assert_eq!(
        Language::ALL.len(),
        66,
        "решение 93 names twelve locales for tier one, вопрос 97 adds Hebrew and Arabic, stage \
         E108 the thirty of group A, stage E109 the twenty-two of group B"
    );

    for (position, (tag, _, native)) in all_locales().iter().enumerate() {
        let language = language_of_tag(tag);
        let index = u32::try_from(position).expect("a locale count fits a u32");

        assert_eq!(
            language.native_name(),
            native.as_str(),
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
    // neighbouring language.
    assert_eq!(Language::from_index(66), Language::Ru);
    assert_eq!(Language::from_index(u32::MAX), Language::Ru);
}

/// **Stage E108, task T-108-2, решение 202.7 — the combo «Язык интерфейса» puts the language of
/// Windows, English, the languages of the layouts and Russian on top, and the rest by name.**
///
/// The owner's words: «вначале системный, потом английски, языки раскладок пользователя, русский и
/// по далее по самоназванию»; вариант 1 of the mock-up. Driven with a given language of Windows
/// and given layouts — the machine of the mock-up (Windows in Russian; Russian, English, Ukrainian
/// and Kazakh layouts) and two others — and only over the languages the build offers: a language
/// the registry does not ship yet is not a row.
///
/// The order under the line is held to the mock-up itself — its forty rows, written out here from
/// the page (`LANGS` of the artifact) — and not to `Language::BY_NAME`, the answer under test.
/// Stage E109: the twenty-two of group B, which the mock-up does not have, stand where the
/// instrument of the mock-up's order puts them (the root collation of ICU over the sixty-six
/// names, `scratchpad-E109\by-name.out.txt`); the forty rows of the mock-up keep their order.
///
/// ⚠ Controls: a top group in the wrong order (Russian before English) and a rest in the order of
/// `Language::ALL` are both told apart from the answer.
#[test]
fn the_language_combo_puts_the_group_of_the_person_on_top_and_the_rest_by_name() {
    // The mock-up of решение 202.7, below its line: «Письма идут в порядке владельца, внутри
    // письма — по алфавиту самоназвания»; the four on top of it are ru, en, uk, kk. Group B
    // (stage E109) among the Latin rows, by the measure.
    const MOCK_UP: [&str; 62] = [
        "az", "bs", "cs", "cy", "da", "de", "et", "es", "fr", "hr", "is", "it", "lv", "lb", "lt",
        "hu", "mt", "mi", "nl", "nb", "nn", "uz", "pl", "pt", "ro", "sq", "sk", "sl", "sr-Latn",
        "fi", "sv", "vi", "tr", "el", "be", "bg", "mk", "sr", "tt", "uz-Cyrl", "hy", "he", "ug",
        "ur", "ar", "fa", "ne", "mr", "hi", "as", "bn", "pa", "gu", "or", "ta", "te", "kn", "ml",
        "th", "lo", "km", "ka",
    ];

    let shipped = |tags: &[&str]| -> Vec<String> {
        tags.iter()
            .filter(|tag| language_of_tag(tag).is_shipped())
            .map(|tag| (*tag).to_owned())
            .collect()
    };
    let tags = |languages: &[Language]| -> Vec<String> {
        languages.iter().map(|l| l.tag().to_owned()).collect()
    };

    // The machine of the mock-up.
    let order = settings::language_order(
        Some(Language::Ru),
        &[Language::Ru, Language::En, Language::Uk, Language::Kk],
    );
    assert_eq!(
        tags(&order.languages[..order.top]),
        shipped(&["ru", "en", "uk", "kk"]),
        "the group on top"
    );
    assert_eq!(
        tags(&order.languages[order.top..]),
        shipped(&MOCK_UP),
        "under the line: the rest in the order of the mock-up"
    );

    // Stage E109: the order of the whole closed set, shipped or not — a language that is not a row
    // yet must already stand in its place when its translation comes.
    let by_name: Vec<&str> = Language::BY_NAME
        .iter()
        .map(|language| language.tag())
        .filter(|tag| !["ru", "en", "uk", "kk"].contains(tag))
        .collect();
    assert_eq!(
        by_name, MOCK_UP,
        "Language::BY_NAME over the closed set: the mock-up and the measure of group B"
    );

    // Every language the build offers is a row, once.
    let mut every = tags(&order.languages);
    every.sort();
    let mut offered: Vec<String> = Language::shipped().map(|l| l.tag().to_owned()).collect();
    offered.sort();
    assert_eq!(every, offered, "every offered language once");

    // Windows in German, an English and a Ukrainian layout: German, English, Ukrainian, Russian —
    // each once, though English is both the second rule and a layout.
    let german = settings::language_order(Some(Language::De), &[Language::En, Language::Uk]);
    assert_eq!(
        tags(&german.languages[..german.top]),
        shipped(&["de", "en", "uk", "ru"]),
        "Windows first, then English, the layouts, Russian"
    );

    // Windows in a language the build does not have, no layouts of ours: English and Russian.
    let none = settings::language_order(None, &[]);
    assert_eq!(tags(&none.languages[..none.top]), ["en", "ru"]);
    assert_eq!(
        none.languages.len(),
        Language::shipped().count(),
        "nothing is lost under the line"
    );

    // The controls.
    assert_ne!(
        tags(&none.languages[..none.top]),
        ["ru", "en"],
        "the control: Russian before English is not the order"
    );
    let in_the_array: Vec<String> = Language::shipped()
        .filter(|l| !order.languages[..order.top].contains(l))
        .map(|l| l.tag().to_owned())
        .collect();
    assert_ne!(
        in_the_array,
        tags(&order.languages[order.top..]),
        "the control: the order of Language::ALL is not the order by name"
    );
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
///
/// ⭐ **Stage E108: all forty-four values, and the block is the one of the registry's script.**
/// Group A brings fourteen scripts without case (the Indic ones, Thai, Lao, Khmer, Georgian,
/// Armenian has case); each name must begin with a letter of the block of the `script` its entry
/// of `res\lang\languages.toml` names, and a script with case must use it.
#[test]
fn the_native_names_are_distinct_and_start_in_their_own_script() {
    // The Unicode block of each script of the closed set (ISO 15924 → the first letter's range).
    fn block_of(script: &str) -> std::ops::RangeInclusive<char> {
        match script {
            "Latn" => 'A'..='\u{024F}',
            "Grek" => '\u{0370}'..='\u{03FF}',
            "Cyrl" => '\u{0400}'..='\u{04FF}',
            "Armn" => '\u{0530}'..='\u{058F}',
            "Hebr" => '\u{0590}'..='\u{05FF}',
            "Arab" => '\u{0600}'..='\u{06FF}',
            "Deva" => '\u{0900}'..='\u{097F}',
            "Beng" => '\u{0980}'..='\u{09FF}',
            "Guru" => '\u{0A00}'..='\u{0A7F}',
            "Gujr" => '\u{0A80}'..='\u{0AFF}',
            "Orya" => '\u{0B00}'..='\u{0B7F}',
            "Taml" => '\u{0B80}'..='\u{0BFF}',
            "Telu" => '\u{0C00}'..='\u{0C7F}',
            "Knda" => '\u{0C80}'..='\u{0CFF}',
            "Mlym" => '\u{0D00}'..='\u{0D7F}',
            "Thai" => '\u{0E00}'..='\u{0E7F}',
            "Laoo" => '\u{0E80}'..='\u{0EFF}',
            "Geor" => '\u{10A0}'..='\u{10FF}',
            "Khmr" => '\u{1780}'..='\u{17FF}',
            other => panic!("the script {other} has no block in this test"),
        }
    }

    let mut seen: Vec<&str> = Vec::new();
    let entries = registry::closed_set();

    assert_eq!(entries.len(), Language::ALL.len(), "one entry per value");

    for (entry, language) in entries.iter().zip(Language::ALL) {
        assert_eq!(entry.tag, language.tag(), "the order of the registry");

        let name = language.native_name();
        let first = name.chars().next().expect("a name is not empty");

        assert!(
            block_of(&entry.script).contains(&first) && first.is_alphabetic(),
            "«{name}» must begin with a letter of its own script {}, not `{first}`",
            entry.script
        );

        // ⚠ Georgian has capitals in Unicode since 11.0 (Mtavruli, U+1C90…), but the language does
        // not capitalise the first letter of a word with them — «ქართული», as Windows writes it.
        let has_case = first.to_uppercase().ne(first.to_lowercase()) && entry.script != "Geor";

        if has_case {
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
///
/// Since stage E108 (решение 202.7) the combo walks `state.languages` — the order
/// `settings::language_order` works out over the languages the build offers — and not the array.
#[test]
fn the_dialog_names_no_language_of_its_own() {
    // The choice of the language of the window of the settings (T-120-20) is filled by walking
    // the order of решение 202.7, each language by the name it gives itself — no literal.
    let window: String = window_of_settings_source()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    assert!(
        window.contains(
            "options:self.languages.languages.iter().map(|language|language.native_name().to_owned())"
        ),
        "the language choice is filled by walking the order of решение 202.7"
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

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
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

        for (tag, langid, _) in shipped_locales() {
            let tag = tag.as_str();
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

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
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

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
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

/// **Т-30-2: exactly the right-to-left languages ask for a mirror, and no others** — Hebrew and
/// Arabic of вопрос 97, and since stage E108 Urdu, Persian and Uyghur of group A.
///
/// `Language::is_rtl` is the one place that knows it — the dialog template, the tray menu and
/// the reading order of an island all read this and nothing else. A sweep over the whole array
/// rather than five `assert!`s, so that a language added without a thought about direction shows
/// up here rather than in a window; and the registry's `rtl` (measured there:
/// `LOCALE_IREADINGLAYOUT`) says the same, value by value.
#[test]
fn only_the_five_right_to_left_languages_are_read_right_to_left() {
    let mirrored: Vec<&str> = Language::ALL
        .into_iter()
        .filter(|language| language.is_rtl())
        .map(Language::tag)
        .collect();

    assert_eq!(
        mirrored,
        vec!["he", "ar", "ur", "fa", "ug"],
        "вопрос 97 names two right-to-left locales, group A three more"
    );

    for (entry, language) in registry::closed_set().iter().zip(Language::ALL) {
        assert_eq!(
            entry.rtl,
            language.is_rtl(),
            "{}: the direction of the registry",
            entry.tag
        );
    }

    // And the other sixty-one (since stage E109) are not mirrored — stated from the other side so
    // that a predicate that answered `true` for everything could not pass the check above by
    // accident.
    assert_eq!(
        Language::ALL
            .into_iter()
            .filter(|language| !language.is_rtl())
            .count(),
        61
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

/// **Стенд Э108, FAIL 1: a panel caption in a script of joining or stacking marks is not cut
/// into characters either.**
///
/// The spacing places every character with a `TextOutW` of its own, and a Devanagari vowel sign,
/// a virama or a Thai tone mark is a character of its own: drawn alone it loses the consonant it
/// belongs to. The stand saw «स◌ाम◌ान◌्य» for «सामान्य» and «ท◌ั◌่วไป» for «ทั่วไป» — the dotted
/// circle is the font's sign of a mark with nothing to sit on (`reports\shots-E108\README.md`,
/// `s2-hi-settings`). The rule of Т-30-4 refused the spacing to Hebrew and Arabic only.
///
/// Held on the six group headings of the **built** binary in every shipped language, by the
/// script the registry gives the language — not by the code's own list of blocks, which is what
/// is under test. A language written in a script of joining or stacking marks is drawn as one
/// run; every other language is spaced exactly as before, the fourteen of 1.2.0 included.
#[test]
fn a_caption_in_a_script_of_joining_or_stacking_marks_is_drawn_as_one_run() {
    const ONE_RUN: [&str; 14] = [
        "Arab", "Hebr", "Deva", "Beng", "Guru", "Gujr", "Orya", "Taml", "Telu", "Knda", "Mlym",
        "Thai", "Laoo", "Khmr",
    ];

    const GROUPS: [u16; 6] = [
        settings::IDS_GROUP_GENERAL,
        settings::IDS_GROUP_HOTKEY,
        settings::IDS_GROUP_LAYOUTS,
        settings::IDS_GROUP_EXCLUSIONS,
        settings::IDS_GROUP_DIAGNOSTICS,
        settings::IDS_GROUP_STATE,
    ];

    let product = ProductImage::shared();
    let mut wrong = Vec::new();

    for (tag, langid, _) in shipped_locales() {
        let script = registry::script_of(&tag).expect("every shipped language names its script");
        let one_run = ONE_RUN.contains(&script.as_str());

        for id in GROUPS {
            let caption = product.string_of_langid(langid, id);

            if settings::caption_takes_tracking(&caption) == one_run {
                wrong.push(format!(
                    "{tag} ({script}) «{caption}»: spaced {}, wanted {}",
                    one_run, !one_run
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "a caption is cut into characters where its script cannot bear it, or the reverse:\n{}",
        wrong.join("\n")
    );
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

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
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

/// **Task T-43-8, finding Н86 — the help names the menu entry by the menu's own word.**
///
/// The fourth line of «Как пользоваться» lists what lives under the tray icon, and in five
/// locales its first word was not the word of the entry it points at: «Пауза» against
/// «Приостановить», and so on. In French it was worse twice over — «Pause» is also the **name of a
/// key**, the very name the same panel puts in a chip two lines above. The words are the ones the
/// mandate gives, the noun of the menu's verb (French keeps the verb itself).
///
/// Held on the **built** binary: the five lines open with their new word and a comma, the French
/// one no longer opens with the key name, each new word shares its stem with the menu entry of
/// the same locale — and the nine other locales, which the mandate leaves alone, carry the line of
/// `e64` byte for byte.
#[test]
fn the_fourth_help_line_opens_with_the_word_of_the_menu_entry() {
    const OPENING: [(&str, &str, &str); 5] = [
        ("ru", "Приостановка", "Приостанов"),
        ("uk", "Призупинення", "Призупин"),
        ("fr", "Suspendre", "Suspend"),
        ("it", "Sospensione", "Sospen"),
        ("pl", "Wstrzymanie", "Wstrzyma"),
    ];

    const UNTOUCHED: [(&str, &str); 9] = [
        ("en", "Suspend, settings and exit are in the tray icon."),
        (
            "de",
            "Anhalten, Einstellungen und Beenden sind im Infobereichssymbol.",
        ),
        (
            "es",
            "Pausa, configuración y salida están en el icono del área de notificación.",
        ),
        (
            "pt",
            "Pausa, configurações e saída ficam no ícone da área de notificação.",
        ),
        (
            "cs",
            "Pozastavení, nastavení a ukončení jsou v ikoně v oznamovací oblasti.",
        ),
        (
            "tr",
            "Duraklatma, ayarlar ve çıkış bildirim alanı simgesindedir.",
        ),
        (
            "el",
            "Παύση, ρυθμίσεις και έξοδος είναι στο εικονίδιο ειδοποιήσεων.",
        ),
        ("he", "השהיה, הגדרות ויציאה נמצאים בסמל אזור ההודעות."),
        (
            "ar",
            "الإيقاف المؤقت والإعدادات والخروج في أيقونة منطقة الإشعارات.",
        ),
    ];

    let product = ProductImage::shared();
    let mut defects: Vec<String> = Vec::new();

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
        let line = product.string_of_langid(langid, settings::IDS_ABOUT_HELP_4);
        let menu = product.string_of_langid(langid, settings::IDS_MENU_SUSPEND);

        println!("{tag}: menu «{menu}» — help «{line}»");

        if let Some((_, word, stem)) = OPENING.iter().find(|(locale, ..)| *locale == tag) {
            if !line.starts_with(&format!("{word},")) {
                defects.push(format!(
                    "{tag}: the line must open with «{word},» — «{line}»"
                ));
            }

            if !menu.starts_with(stem) || !word.starts_with(stem) {
                defects.push(format!(
                    "{tag}: «{word}» and the menu entry «{menu}» must share the stem «{stem}»"
                ));
            }
        } else if let Some((_, before)) = UNTOUCHED.iter().find(|(locale, _)| *locale == tag) {
            if line != *before {
                defects.push(format!(
                    "{tag}: the mandate leaves this locale alone — «{line}» is not the line of \
                     e64 «{before}»"
                ));
            }
        } else if registry::after_the_fourteen(tag) && !line.trim().is_empty() {
            // Stage E108: a language of group A (and since E109 of group B) came after the mandate of T-43-8 and is in
            // neither table — its line is the translator's, read line by line by the reviewer
            // (решение 202.4); here it is held to be there.
        } else {
            defects.push(format!(
                "{tag}: the locale is in neither table of this test"
            ));
        }
    }

    let french = product.string_of_langid(0x040C, settings::IDS_ABOUT_HELP_4);

    if french.starts_with("Pause") {
        defects.push(format!(
            "fr: the line still opens with «Pause», the name of a key — «{french}»"
        ));
    }

    assert!(
        defects.is_empty(),
        "the fourth line of the help must name the menu entry by its own word:\n{}",
        defects.join("\n")
    );
}

/// Whether `c` is a mark that belongs inside a word of a script with vowel signs — a virama, a
/// nukta, a vowel sign of the Indic scripts, Thai, Lao, Khmer, a haraka of the Arabic script —
/// which `char::is_alphanumeric` does not always answer `true` for. Their punctuation (the danda,
/// the Khmer full stops) is not a mark.
fn is_a_mark(c: char) -> bool {
    let in_a_block = matches!(
        c,
        '\u{0300}'..='\u{036F}'
            | '\u{0610}'..='\u{061A}'
            | '\u{064B}'..='\u{065F}'
            | '\u{0670}'
            | '\u{06D6}'..='\u{06ED}'
            | '\u{0900}'..='\u{0DFF}'
            | '\u{0E00}'..='\u{0EFF}'
            | '\u{1780}'..='\u{17FF}'
    );

    in_a_block && !matches!(c, '\u{0964}' | '\u{0965}' | '\u{17D4}'..='\u{17DA}')
}

/// Whether `letter` belongs to the script the string table of `tag` is written in.
///
/// Four scripts besides Latin, by their Unicode blocks: Cyrillic for `ru` and `uk`, Greek (with
/// its extended block of tonos forms) for `el`, Hebrew (with its presentation forms) for `he`,
/// Arabic (with its supplement, extended block and presentation forms, where the harakat of `ًا`
/// live) for `ar`. Every other locale of the program writes in Latin — ASCII letters and the
/// Latin-1 supplement and Extended-A/B blocks that carry `ü`, `ç`, `ł`, `ř`, `ş`, `ı`.
///
/// ⭐ **Stage E108: the script is the one the registry names for `tag`** (`res\lang\languages.toml`,
/// field `script`), and the blocks are the scripts', not the languages': Urdu, Persian and Uyghur
/// write in the Arabic blocks, Kazakh in the Cyrillic one, Azerbaijani in Latin with `ə` of the IPA
/// block, Hindi in Devanagari with its vowel signs — the fourteen answer exactly as before.
fn letter_of_the_script_of(tag: &str, letter: char) -> bool {
    let script =
        registry::script_of(tag).unwrap_or_else(|| panic!("«{tag}» is not in the registry"));

    match script.as_str() {
        "Cyrl" => matches!(letter, '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}'),
        "Grek" => matches!(letter, '\u{0370}'..='\u{03FF}' | '\u{1F00}'..='\u{1FFF}'),
        "Hebr" => matches!(letter, '\u{0590}'..='\u{05FF}' | '\u{FB1D}'..='\u{FB4F}'),
        "Arab" => matches!(
            letter,
            '\u{0600}'..='\u{06FF}'
                | '\u{0750}'..='\u{077F}'
                | '\u{08A0}'..='\u{08FF}'
                | '\u{FB50}'..='\u{FDFF}'
                | '\u{FE70}'..='\u{FEFF}'
        ),
        // ʻ (U+02BB) and ʼ (U+02BC) are letters of Uzbek in Latin — oʻ, gʻ — and stand in the block
        // of the spacing modifiers (stage E108). Stage E109: Vietnamese writes its letters with two
        // marks — ợ, ở, ạ — out of Latin Extended Additional.
        "Latn" => {
            letter.is_ascii_alphabetic()
                || matches!(
                    letter,
                    '\u{00C0}'..='\u{02AF}' | '\u{02BB}' | '\u{02BC}' | '\u{1E00}'..='\u{1EFF}'
                )
        }
        "Armn" => matches!(letter, '\u{0530}'..='\u{058F}'),
        "Deva" => matches!(letter, '\u{0900}'..='\u{097F}' | '\u{A8E0}'..='\u{A8FF}'),
        "Beng" => matches!(letter, '\u{0980}'..='\u{09FF}'),
        "Guru" => matches!(letter, '\u{0A00}'..='\u{0A7F}'),
        "Gujr" => matches!(letter, '\u{0A80}'..='\u{0AFF}'),
        "Orya" => matches!(letter, '\u{0B00}'..='\u{0B7F}'),
        "Taml" => matches!(letter, '\u{0B80}'..='\u{0BFF}'),
        "Telu" => matches!(letter, '\u{0C00}'..='\u{0C7F}'),
        "Knda" => matches!(letter, '\u{0C80}'..='\u{0CFF}'),
        "Mlym" => matches!(letter, '\u{0D00}'..='\u{0D7F}'),
        "Thai" => matches!(letter, '\u{0E00}'..='\u{0E7F}'),
        "Laoo" => matches!(letter, '\u{0E80}'..='\u{0EFF}'),
        "Geor" => matches!(letter, '\u{10A0}'..='\u{10FF}' | '\u{1C90}'..='\u{1CBF}'),
        "Khmr" => matches!(letter, '\u{1780}'..='\u{17FF}' | '\u{19E0}'..='\u{19FF}'),
        other => panic!("the script {other} of «{tag}» has no blocks in this test"),
    }
}

/// **Task T-43-7, finding Н83, решение 128.1 — the third help line carries its caveat.**
///
/// «Выделите текст и нажмите {0} — конвертируется выделенное» promised a conversion that is a
/// switch of its own («Конвертировать выделенный текст», `IDS_SELECTION_ENABLED`) and that does
/// not happen where `Ctrl+C` copies nothing — the selection path takes its text with a real
/// `Ctrl+C`, FR-61 step 2. Решение 128.1: the line is not greyed; the caveat is written into it,
/// in all fourteen tables, by the procedure of Э28 (`scratchpad-E43\glossary.md` and the table of
/// back-translations beside it).
///
/// Held against the **built** binary, locale by locale: the line is longer than the line of `e64`
/// it replaced (the table below is that text byte for byte — the red «before» of this test); it
/// keeps its one `{0}`; it names `Ctrl+C`, which no locale translates; and every letter it carries
/// outside `Ctrl+C` belongs to the locale's own script, with more of them than before. A caveat
/// that went to English in the Arabic table, or to question marks in any table, fails on the last
/// two — the mojibake of Э28 is caught by reversibility elsewhere, not by a list of suspicious
/// letters here.
///
/// ⚠ The caveat made the line one line of text longer than its slot, and решение 128.4 grew the
/// window for it — that half is held by
/// `every_help_sentence_fits_its_template_slot_in_all_fourteen_languages`.
#[test]
fn the_selection_help_line_carries_its_caveat_in_every_locale() {
    const BEFORE: [(&str, &str); 14] = [
        (
            "ru",
            "Выделите текст и нажмите {0} — конвертируется выделенное.",
        ),
        (
            "en",
            "Select text and press {0} — the selection is converted.",
        ),
        (
            "uk",
            "Виділіть текст і натисніть {0} — конвертується виділене.",
        ),
        (
            "de",
            "Text markieren und {0} drücken — die Markierung wird umgewandelt.",
        ),
        (
            "fr",
            "Sélectionnez du texte et appuyez sur {0} — la sélection est convertie.",
        ),
        (
            "es",
            "Seleccione texto y pulse {0} — se convierte lo seleccionado.",
        ),
        (
            "pt",
            "Selecione o texto e pressione {0} — a seleção é convertida.",
        ),
        (
            "it",
            "Seleziona il testo e premi {0} — la selezione viene convertita.",
        ),
        (
            "pl",
            "Zaznacz tekst i naciśnij {0} — zaznaczenie zostanie przekonwertowane.",
        ),
        ("cs", "Vyberte text a stiskněte {0} — převede se výběr."),
        ("tr", "Metni seçip {0} tuşuna basın — seçim dönüştürülür."),
        (
            "el",
            "Επιλέξτε κείμενο και πατήστε {0} — μετατρέπεται η επιλογή.",
        ),
        ("he", "סמנו טקסט והקישו {0} — המסומן מומר."),
        ("ar", "حدد نصًا واضغط {0} — يتحول المحدد."),
    ];

    let product = ProductImage::shared();
    let mut defects: Vec<String> = Vec::new();

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
        // Stage E108: a language of group A (since E109, of group B too) had no line in `e64` — it is written with the caveat
        // from the start, and the comparison with the old line is the fourteen's alone; every
        // other check below holds it as it holds them.
        let before = BEFORE
            .iter()
            .find(|(locale, _)| *locale == tag)
            .map_or("", |(_, line)| *line);
        if before.is_empty() && !registry::after_the_fourteen(tag) {
            defects.push(format!(
                "{tag}: a language of 1.0.0 without its line of e64"
            ));
        }
        let now = product.string_of_langid(langid, settings::IDS_ABOUT_HELP_3);

        let letters = |line: &str| {
            let words = line.replace("Ctrl+C", "");
            let own = words
                .chars()
                .filter(|letter| letter.is_alphabetic() && letter_of_the_script_of(tag, *letter))
                .count();
            let foreign: String = words
                .chars()
                .filter(|letter| letter.is_alphabetic() && !letter_of_the_script_of(tag, *letter))
                .collect();

            (own, foreign)
        };

        let (own_before, _) = letters(before);
        let (own_now, foreign_now) = letters(&now);
        let (units_before, units_now) = (before.encode_utf16().count(), now.encode_utf16().count());

        println!(
            "{tag}: UTF-16 {units_before} → {units_now}, letters of its script {own_before} → \
             {own_now} — «{now}»"
        );

        if units_now <= units_before {
            defects.push(format!(
                "{tag}: the line is no longer than the line of e64 ({units_now} ≤ {units_before} \
                 UTF-16 units) — «{now}»"
            ));
        }

        if now.matches("{0}").count() != 1 {
            defects.push(format!("{tag}: the key name must stand once — «{now}»"));
        }

        if !now.contains("Ctrl+C") {
            defects.push(format!(
                "{tag}: the caveat must name Ctrl+C, which no locale translates — «{now}»"
            ));
        }

        if !foreign_now.is_empty() {
            defects.push(format!(
                "{tag}: letters outside the locale's script: «{foreign_now}» — «{now}»"
            ));
        }

        if own_now <= own_before {
            defects.push(format!(
                "{tag}: the caveat added no letters of the locale's own script ({own_before} → \
                 {own_now}) — «{now}»"
            ));
        }
    }

    assert!(
        defects.is_empty(),
        "the caveat of решение 128.1 must stand in all fourteen tables:\n{}",
        defects.join("\n")
    );
}

/// **Task T-43-9, finding Н23, решение 128.2 — «уже запущена» in the language of Windows.**
///
/// A second copy of the program showed «Lang Switcher is already running in this session.» — an
/// English literal of `src\app.rs`, the one English phrase of the program in every locale, left
/// there because the task that wrote FR-82 came before FR-94. The sentence is a row of all
/// fourteen tables now (`settings::IDS_ALREADY_RUNNING`), read through `settings::text` in the
/// language Windows prefers — the second copy exits before any configuration is read.
///
/// Four things are held, and none of them puts a box on the screen:
/// 1. every table carries the sentence, and it keeps the product's name untranslated (решение 7)
///    and says the rest in the letters of its own script — the UTF-16 lengths go to the log,
///    because the box is the system's and there is no clip to measure;
/// 2. the English row is the old literal word for word, so the twelve locales that are not
///    Russian lose nothing they had;
/// 3. the style of the box is a table: the two right-to-left languages read right to left
///    (`MB_RTLREADING | MB_RIGHT`), the other twelve keep the style the box always had;
/// 4. the shape — the literal is gone from `src\app.rs`, and `notify_already_running` asks the
///    string table, in the language of Windows, with that style.
#[test]
fn the_second_copy_says_it_is_already_running_in_the_language_of_windows() {
    use windows::Win32::UI::WindowsAndMessaging::{
        MB_ICONINFORMATION, MB_OK, MB_RIGHT, MB_RTLREADING, MB_SETFOREGROUND,
    };

    const OLD_LITERAL: &str = "Lang Switcher is already running in this session.";

    let product = ProductImage::shared();
    let mut defects: Vec<String> = Vec::new();

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
        let sentence = product.string_of_langid(langid, settings::IDS_ALREADY_RUNNING);
        let rest = sentence.replace("Lang Switcher", "");
        let own = rest
            .chars()
            .filter(|letter| letter.is_alphabetic() && letter_of_the_script_of(tag, *letter))
            .count();
        let foreign: String = rest
            .chars()
            .filter(|letter| letter.is_alphabetic() && !letter_of_the_script_of(tag, *letter))
            .collect();

        println!(
            "{tag}: UTF-16 {} — «{sentence}»",
            sentence.encode_utf16().count()
        );

        if !sentence.contains("Lang Switcher") {
            defects.push(format!(
                "{tag}: the name of the product is not translated (решение 7) — «{sentence}»"
            ));
        }

        if own == 0 || !foreign.is_empty() {
            defects.push(format!(
                "{tag}: the sentence must speak in its own script ({own} letters of it, foreign \
                 «{foreign}») — «{sentence}»"
            ));
        }

        if tag == "en" && sentence != OLD_LITERAL {
            defects.push(format!(
                "en: the English row must be the literal it replaced — «{sentence}»"
            ));
        }
    }

    let ordinary = MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND;

    for language in Language::shipped() {
        let style = lang_switcher::app::already_running_style(language);
        let reads_right_to_left = style.0 & MB_RTLREADING.0 != 0 && style.0 & MB_RIGHT.0 != 0;

        if style.0 & ordinary.0 != ordinary.0 {
            defects.push(format!(
                "{language:?}: the box lost its ordinary style — {:#x}",
                style.0
            ));
        }

        if reads_right_to_left != language.is_rtl() {
            defects.push(format!(
                "{language:?}: MB_RTLREADING | MB_RIGHT is {reads_right_to_left}, the language \
                 reads right to left: {}",
                language.is_rtl()
            ));
        }
    }

    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("app.rs"),
    )
    .expect("src\\app.rs must be readable")
    .replace("\r\n", "\n");
    let product_half = source
        .split("\n#[cfg(test)]\nmod tests {")
        .next()
        .expect("a split always has a first part");

    if product_half.contains(OLD_LITERAL) {
        defects.push("src\\app.rs still carries the English literal".to_owned());
    }

    let notify = function_body(product_half, "fn notify_already_running() {");

    for needle in [
        "settings::system_language_for_a_first_run()",
        "settings::text_in(language, settings::IDS_ALREADY_RUNNING)",
        "already_running_style(language)",
    ] {
        if !notify.contains(needle) {
            defects.push(format!("notify_already_running does not ask `{needle}`"));
        }
    }

    // ⚠ And it publishes no locale on its way out: решение 99 gives the process one publication
    // of «на каком языке программа» (`tray::adopt_ui_language`), and the first draft of this
    // task broke that with two calls of `set_ui_language` here.
    if notify.contains("set_ui_language(") {
        defects.push(
            "notify_already_running publishes a locale — a second answer to «на каком языке \
             программа» (решение 99)"
                .to_owned(),
        );
    }

    assert!(
        defects.is_empty(),
        "the notification of FR-82 must come out of the string tables, in the language of \
         Windows:\n{}",
        defects.join("\n")
    );
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

    // Stage E108, решение 202.7: the rows are the order of the languages, worked out per opening —
    // here as a machine whose Windows speaks the language the dialog shows and whose layouts are
    // Russian and English; the position of a row is a position in that order, not in the array.
    for from in Language::shipped() {
        let order = settings::language_order(Some(from), &[Language::Ru, Language::En]);
        let row_of = |language: Language| {
            order
                .languages
                .iter()
                .position(|row| *row == language)
                .expect("every offered language is a row")
        };

        for to in Language::shipped() {
            // 1. The dialog opens on a configuration carrying `from` and selects its row.
            let mut config = Config::default();
            config.general.language = from;

            let selected = row_of(config.general.language.in_force());
            assert_eq!(
                order.languages[selected],
                from,
                "the combo opens on `{}`",
                from.tag()
            );

            // 2. «ОК»: the language of the row the user left selected becomes the value.
            let picked = row_of(to);
            config.general.language = order.languages[picked];

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
                row_of(back.general.language.in_force()),
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
        // Stage E109: `nl-NL` and `sv-SE` stood here until Dutch and Swedish joined group B;
        // Catalan and Basque are in no group.
        "ca-ES",
        "eu-ES",
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

/// **Stage E108, task T-108-2, решение 200б.1 — Serbian and Uzbek are chosen by the script of the
/// full tag of Windows, both pairs.**
///
/// The primary subtag `sr` / `uz` does not tell the scripts apart; Windows does, in the full tag
/// `GetUserPreferredUILanguages` answers: `sr-Cyrl-RS` / `sr-Latn-RS`, `uz-Latn-UZ` /
/// `uz-Cyrl-UZ`. The main language of each — Serbian in Cyrillic, Uzbek in Latin — takes the bare
/// tag and any tag without the other script. Driven over the whole closed set
/// (`language_of_tag_among`), so the rule is held whatever the registry ships today; and over the
/// languages the build offers, the answer is always one of them.
///
/// ⚠ Control: the rule of the primary subtag alone — the first language whose tag begins with
/// `sr` — answers Cyrillic Serbian for `sr-Latn-RS` and is caught.
#[test]
fn serbian_and_uzbek_are_chosen_by_the_script_of_the_full_tag() {
    let among_all = |tag: &str| settings::language_of_tag_among(tag, Language::ALL);

    for (tag, wanted) in [
        ("sr-Cyrl-RS", Language::Sr),
        ("sr-Latn-RS", Language::SrLatn),
        ("sr-Latn-ME", Language::SrLatn),
        ("sr-Cyrl-BA", Language::Sr),
        ("sr", Language::Sr),
        ("sr-RS", Language::Sr),
        ("SR-latn-rs", Language::SrLatn),
        ("uz-Latn-UZ", Language::Uz),
        ("uz-Cyrl-UZ", Language::UzCyrl),
        ("uz", Language::Uz),
        ("uz-UZ", Language::Uz),
        ("uz_Cyrl", Language::UzCyrl),
        // The other languages of group A by their primary subtag, as the fourteen.
        ("az-Latn-AZ", Language::Az),
        ("pa-IN", Language::Pa),
        ("bn-BD", Language::Bn),
        ("kk-KZ", Language::Kk),
        ("ug-CN", Language::Ug),
    ] {
        assert_eq!(
            among_all(tag),
            Some(wanted),
            "`{tag}` must be `{}`",
            wanted.tag()
        );
    }

    // The rule of the primary subtag alone, as it stood until stage E108 — the control.
    let primary_only = |tag: &str| {
        Language::ALL
            .into_iter()
            .find(|language| language.tag().starts_with(&tag[..2]))
    };
    assert_ne!(
        primary_only("sr-Latn-RS"),
        Some(Language::SrLatn),
        "the control: the primary subtag alone does not see the script"
    );

    // Over the languages the build offers: always one of them, or none.
    for tag in [
        "sr-Cyrl-RS",
        "sr-Latn-RS",
        "uz-Latn-UZ",
        "uz-Cyrl-UZ",
        "hi-IN",
        "ur-PK",
    ] {
        if let Some(language) = settings::language_of_ui_tag(tag) {
            assert!(language.is_shipped(), "`{tag}` → {language:?}, not offered");
        }
    }
}

/// **Stage E109, task T-109-1 — Norwegian is Bokmål and Nynorsk, the old macrolanguage `no` is
/// Bokmål, and Bosnian is one language in the Latin script.**
///
/// `nb-NO` and `nn-NO` carry primary subtags of their own; a Windows of an older day, or a tag
/// copied out of somewhere, may still name the macrolanguage `no` — the program answers it with
/// Bokmål, the main Norwegian (решение 200б.1, TZ E109 §0 п. 2). Bosnian in either script is the
/// Latin `bs`. Driven by tag over the whole closed set, so the rule is held whatever the registry
/// ships today.
///
/// ⚠ Control: the rule of the primary subtag alone answers nothing for `no-NO` and is caught.
#[test]
fn norwegian_is_bokmal_and_nynorsk_and_the_old_no_is_bokmal() {
    let among_all =
        |tag: &str| settings::language_of_tag_among(tag, Language::ALL).map(Language::tag);

    for (tag, wanted) in [
        ("nb-NO", "nb"),
        ("nb", "nb"),
        ("nn-NO", "nn"),
        ("nn", "nn"),
        ("no", "nb"),
        ("no-NO", "nb"),
        ("NO_no", "nb"),
        ("bs-Latn-BA", "bs"),
        ("bs-Cyrl-BA", "bs"),
        ("bs", "bs"),
        // The other languages of group B by their primary subtag, as the fourteen.
        ("hr-HR", "hr"),
        ("sl-SI", "sl"),
        ("vi-VN", "vi"),
        ("mi-NZ", "mi"),
        ("lb-LU", "lb"),
        ("is-IS", "is"),
    ] {
        assert_eq!(among_all(tag), Some(wanted), "`{tag}` must be `{wanted}`");
    }

    assert_eq!(among_all("nob"), None, "`nob` is not the subtag `no`");

    // The rule of the primary subtag alone — the control.
    let primary_only = |tag: &str| {
        let primary = tag.split(['-', '_']).next().unwrap_or_default();
        Language::ALL
            .into_iter()
            .find(|language| language.tag().eq_ignore_ascii_case(primary))
            .map(Language::tag)
    };
    assert_eq!(
        primary_only("no-NO"),
        None,
        "the control: the primary subtag alone does not know `no`"
    );
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

/// **Task T-43-10, finding Н71 — a list of languages longer than the cap leaves a trace, and an
/// empty one does not.**
///
/// The sizing call of `preferred_ui_languages` can end the read two ways: Windows named nothing,
/// or Windows named more than `PREFERRED_LANGUAGES_CAP` units and this program refused to read so
/// much. Both used to be the same silent `return Vec::new()`, and the second one turns a first run
/// English with nothing in the journal to say why. A live machine gives neither size on demand, so
/// the size is handed to the one function the read goes through, and the journal is read back:
/// the entry is counted **by name** among the entries recorded after the mark, the way
/// `tests\diag.rs` reads its own.
#[test]
fn a_language_list_longer_than_the_cap_is_journaled_and_an_empty_one_is_not() {
    use lang_switcher::diag;
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;

    let entries_since = |mark: u64| {
        diag::snapshot()
            .into_iter()
            .filter(|event| {
                event.ordinal >= mark && event.operation.name() == "GetUserPreferredUILanguages"
            })
            .collect::<Vec<_>>()
    };

    let cap = settings::PREFERRED_LANGUAGES_CAP;

    // Windows named nothing: nothing to read, nothing to say.
    let mark = diag::recorded();
    assert!(!settings::preferred_languages_size_is_readable(0));
    let after_empty = entries_since(mark);

    // A list this program reads, up to the cap itself: read, and silent.
    let mark = diag::recorded();
    assert!(settings::preferred_languages_size_is_readable(1));
    assert!(settings::preferred_languages_size_is_readable(cap));
    let after_readable = entries_since(mark);

    // One unit past the cap: refused, and journaled.
    let mark = diag::recorded();
    assert!(!settings::preferred_languages_size_is_readable(cap + 1));
    let after_too_long = entries_since(mark);

    println!(
        "entries: empty list {}, readable {}, longer than {cap} units {} — codes {:?}",
        after_empty.len(),
        after_readable.len(),
        after_too_long.len(),
        after_too_long
            .iter()
            .map(|event| format!("{:#010X}", event.code.raw()))
            .collect::<Vec<_>>()
    );

    assert!(
        after_empty.is_empty(),
        "an empty answer is the ordinary road to English and must stay silent"
    );
    assert!(
        after_readable.is_empty(),
        "a list that is read is not a refusal"
    );
    assert_eq!(
        after_too_long.len(),
        1,
        "a list longer than the cap must leave exactly one entry under GetUserPreferredUILanguages"
    );
    assert_eq!(
        after_too_long[0].code.raw(),
        ERROR_INSUFFICIENT_BUFFER.to_hresult().0,
        "and carry the code Windows gives a buffer too small for the answer — the fact and that \
         number, nothing of the list (SEC-01, SEC-07)"
    );
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
    let mut first_run = Config::for_a_first_run();
    // Stage E116, task T-116-12: the mark of the first run is written on this road only.
    first_run.letters.first_mark = Some(letters::FirstMark::Due);
    assert_eq!(
        config, first_run,
        "and in nothing else — the rest of section 7 is untouched by вопрос 95"
    );

    // Every other field is the default of section 7. Written as a whole-structure comparison
    // so that a field added later cannot slip through unasserted.
    let mut as_before = Config::default();
    as_before.general.language = config.general.language;
    as_before.letters.first_mark = Some(letters::FirstMark::Due);
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

    // A file this build could not use — **turned round by task T-55-9** (findings С20 and Н67).
    // Until that task a damaged file fell back to the `ru` of section 7, on the reasoning that its
    // bytes are somebody's configuration. They still are, and they are still not touched; but the
    // program lives by defaults there, not by that configuration, and the language of those
    // defaults is now the system's. ⚠ On a machine whose Windows is Russian this line cannot fail —
    // the system answers `ru` — and the staged test below is the one that can.
    let broken = dir.path.join("broken.toml");
    fs::write(&broken, "[general\nenabled = = true\n").expect("writable");
    let (config, outcome) = settings::read_or_default(&broken);

    assert!(outcome.is_err());
    assert_eq!(
        config.general.language,
        settings::system_language_for_a_first_run(),
        "a file this build could not use speaks the language of the system"
    );
}

/// **Task T-55-9, finding С20 — a file this build could not use lives in the language of the
/// system, and a file that parses does not.**
///
/// The language of the system is staged as German, so that the answer can be told from the `ru`
/// of section 7 on a machine whose Windows is Russian. (а) A damaged file, (а′) a file held open
/// that could not be read, (б) a file from a newer schema that did not parse — all three leave the
/// program on defaults, and those defaults speak the system's language; nothing else of them
/// moves. The controls: a newer file that **did** parse keeps its own language, and (г) — the
/// second half of вопрос 95 — a sound file without a `language` line is still `ru`.
#[test]
fn a_file_this_build_could_not_use_lives_in_the_language_of_the_system() {
    use std::os::windows::fs::OpenOptionsExt;

    let german = || Language::De;
    let dir = TestDir::new("language_of_an_unusable_file");

    // (а) A damaged file.
    let broken = dir.path.join("broken.toml");
    fs::write(&broken, "[general\nenabled = = true\n").expect("writable");

    let (config, outcome) = settings::read_or_default_via(&broken, &german);

    assert!(matches!(outcome, Err(ConfigError::Malformed { .. })));
    assert_eq!(
        config,
        Config::for_a_first_run_in(Language::De),
        "С20: the defaults a damaged file leaves the program on speak the system's language, and \
         nothing else of them moves"
    );

    // (а′) A file held open by another program, which could not be read — task T-55-6.
    let held_path = dir.path.join("held.toml");
    fs::write(&held_path, "schema_version = 6\n").expect("writable");
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&held_path)
        .expect("the file must open");

    let (config, outcome) = settings::read_or_default_via(&held_path, &german);

    drop(held);

    assert!(matches!(outcome, Err(ConfigError::Io(_))));
    assert_eq!(
        config.general.language,
        Language::De,
        "a file that could not be read leaves the program on the same defaults"
    );

    // (б) A file from a newer schema that did not parse.
    let (config, outcome) = Config::from_toml_str_via(
        "schema_version = 99\n\n[general]\nlanguage = \"zz\"\n",
        &german,
    )
    .expect("a newer file is read, not refused");

    assert_eq!(outcome, ReadOutcome::FromNewerSchema { version: 99 });
    assert_eq!(
        config.general.language,
        Language::De,
        "С20: a newer file this build did not understand speaks the system's language"
    );
    assert_eq!(config.schema_version, 99, "and carries its stamp as before");

    // The control of (б): a newer file that did parse is somebody's file, and keeps its language.
    let (config, outcome) = Config::from_toml_str_via(
        "schema_version = 99\n\n[general]\nlanguage = \"fr\"\n",
        &german,
    )
    .expect("a newer file that parses is read");

    assert_eq!(outcome, ReadOutcome::FromNewerSchema { version: 99 });
    assert_eq!(
        config.general.language,
        Language::Fr,
        "a newer file that parsed keeps the language it names"
    );

    // (г) The control of вопрос 95: a sound file without a `language` line.
    let sound = dir.path.join("sound.toml");
    fs::write(
        &sound,
        format!("schema_version = {CURRENT_SCHEMA_VERSION}\n\n[general]\nenabled = false\n"),
    )
    .expect("writable");

    let (config, outcome) = settings::read_or_default_via(&sound, &german);

    assert_eq!(outcome.ok(), Some(ReadOutcome::Current));
    assert_eq!(
        config.general.language,
        Language::Ru,
        "вопрос 95: a file that exists and says nothing about the language is somebody's file — \
         its language is the `ru` of section 7, never the system's"
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
        chosen.is_shipped(),
        "the answer is always a language the build offers"
    );
}

// ---------------------------------------------------------------------------------------
// FR-15 — the idle timeout of `[buffer]`, and the schema it deliberately did not cost.
// Task T-52-4
// ---------------------------------------------------------------------------------------

/// **Section 7's default and its ceilings** — FR-15, task T-52-4.
///
/// Five minutes: long enough that a user who stopped to read something and came back finds the
/// word they were typing still there, short enough that a machine left alone is not keeping a
/// half-typed line in memory for the rest of the day (SEC-01).
#[test]
fn the_idle_timeout_of_fr15_defaults_to_five_minutes_and_survives_the_round_trip() {
    assert_eq!(
        Config::default().buffer.idle_timeout_s,
        300,
        "FR-15: раздел 7 обещает 300 секунд"
    );

    let dir = TestDir::new("idle_timeout_round_trip");

    for wanted in [0, 30, 300, 86_400] {
        let mut config = Config::default();
        config.buffer.idle_timeout_s = wanted;

        let text = config.to_toml_string().expect("serialises");

        assert!(
            text.contains(&format!("idle_timeout_s = {wanted}")),
            "the file has to carry the field: {text}"
        );

        let path = write_file(&dir, &text);
        let (back, outcome) = settings::read_from(&path).expect("a current file must be read");

        assert_eq!(outcome, ReadOutcome::Current);
        assert_eq!(back.buffer.idle_timeout_s, wanted);
    }
}

/// **The ceilings are applied where the value is published, and the file may hold anything.**
///
/// The same rule task T-13-13 gives the three millisecond fields of section 7: a hand-edited
/// file is not validated and not rewritten, and what the program *does* with an impossible
/// number is clamp it. The floor is the thirty-second liveness tick of FR-80 — the rule is
/// checked there and nowhere else, so a shorter timeout could not be honoured and a user who
/// wrote `5` would get thirty seconds while believing they had five.
///
/// ⚠ **Zero is not clamped**, and that is the difference from `capacity`: `0` is how section 7
/// says «выключено», which is a request rather than a slip.
#[test]
fn an_impossible_idle_timeout_is_clamped_at_publication_and_left_in_the_file() {
    let dir = TestDir::new("idle_timeout_clamped");

    for (in_file, published) in [
        (0u32, 0u32),
        (1, 30),
        (29, 30),
        (30, 30),
        (300, 300),
        (86_400, 86_400),
        (86_401, 86_400),
        (u32::MAX, 86_400),
    ] {
        let path = write_file(
            &dir,
            &format!(
                "schema_version = {CURRENT_SCHEMA_VERSION}\n\
                 \n\
                 [buffer]\n\
                 idle_timeout_s = {in_file}\n"
            ),
        );

        let (config, outcome) = settings::read_from(&path).expect("the file must be read");

        assert_eq!(outcome, ReadOutcome::Current);
        assert_eq!(
            config.buffer.idle_timeout_s, in_file,
            "the file is not rewritten and not validated"
        );
        assert_eq!(
            lang_switcher::buffer::effective_idle_timeout_s(config.buffer.idle_timeout_s),
            published,
            "idle_timeout_s = {in_file} публикуется как {published}"
        );
    }

    assert_eq!(lang_switcher::buffer::MIN_IDLE_TIMEOUT_S, 30);
    assert_eq!(lang_switcher::buffer::MAX_IDLE_TIMEOUT_S, 86_400);
}

/// **⭐ The schema did not move for FR-15, and this is the reasoning as a measurement.**
///
/// A file this build has never seen the key in — a configuration written by `e51`, stamped
/// schema 6 — is read as **current**, with the section 7 default filled in by serde. It is not
/// migrated, because there is nothing to migrate: an absent key already has a defined meaning.
///
/// Raising the schema to 7 would have bought nothing and cost something real. There is no
/// `deny_unknown_fields` in this module, so a file written by *this* build is read by the
/// previous one without complaint — the key is simply ignored there. A schema of 7, by
/// contrast, would send that same file into quarantine as «из будущего»
/// (`ReadOutcome::FromNewerSchema`) the moment a user went back to `e51`, which is the one
/// outcome the version field exists to cause and the one nobody wanted here.
#[test]
fn a_schema_6_file_without_the_idle_timeout_is_current_and_not_migrated() {
    let dir = TestDir::new("idle_timeout_absent");

    let path = write_file(
        &dir,
        &format!(
            "schema_version = {CURRENT_SCHEMA_VERSION}\n\
             \n\
             [buffer]\n\
             capacity = 256\n"
        ),
    );

    let (config, outcome) = settings::read_from(&path).expect("an e51 file must still be read");

    assert_eq!(
        outcome,
        ReadOutcome::Current,
        "FR-15: файл схемы 6 без ключа — текущий, а не мигрированный"
    );
    assert_eq!(
        config.buffer.idle_timeout_s, 300,
        "отсутствующий ключ читается умолчанием раздела 7"
    );
    assert_eq!(config.buffer.capacity, 256, "и остальное на месте");

    // The schema itself did not move for FR-15 — the whole point of the paragraph above. ⚠ Task
    // T-93-1 moved it, for a reason of its own: the one rung above the 6 FR-15 shipped on is the
    // stamp of `[hotkey] modifiers` (вопрос 157), and the number here says so rather than six.
    // ⚠ Task T-95-1 moved it once more: the stamp of `[hotkey] trigger` (вопрос 159) is the second.
    // ⚠ Stage E108 a third time: the stamp of the thirty languages of group A.
    // ⚠ Stage E109 a fourth time: the stamp of the twenty-two languages of group B.
    assert_eq!(
        CURRENT_SCHEMA_VERSION, 10,
        "FR-15 не поднимал схему (была 6); 7 — штамп `[hotkey] modifiers` Э93, 8 — штамп \
         `[hotkey] trigger` Э95, 9 — штамп группы А Э108, 10 — штамп группы Б Э109, и только они"
    );

    // And the far side of that promise: a file stamped above the current schema *would* be read
    // as one from the future, which is what makes the choice above a choice and not a
    // coincidence.
    let ahead = CURRENT_SCHEMA_VERSION + 1;
    let newer = write_file(&dir, &format!("schema_version = {ahead}\n"));
    let (_, outcome) = settings::read_from(&newer).expect("a newer file is read, not refused");

    assert_eq!(outcome, ReadOutcome::FromNewerSchema { version: ahead });
}

// =========================================================================================
// The one door out of this program — finding С26, task T-41-4
// =========================================================================================

/// **Finding С26, task T-41-4 — this program asks for something to be opened in exactly one
/// place.**
///
/// Opening something is the only thing this program does that starts somebody else's code, and
/// the audit's warning about a second door was not hypothetical: by the time the repair arrived
/// there were **three** such places, and two of them opened the *same* journal folder by
/// different code — the button of the «Диагностика» section and «Сохранить в папку журнала» of
/// the letter wizard (FR-104).
///
/// # What this test is, and what it is not
///
/// It counts **calls**, not mentions: the lines of every `src\*.rs` that are comments are
/// thrown away first, which is what keeps the prose of the repair — and the row
/// `("ShellExecuteW", Kind::Window)` of the journal's vocabulary — out of the count. Э22's
/// lesson in one line: before believing a sweep, ask whether it can hit itself.
///
/// ⚠ **The instrument's positive control is measured, not assumed.** The very same cut and
/// needle over `7455732` — the commit this stage began at — answer **3**, in the two files the
/// finding names. The journal is `scratchpad-E41\red-T-41-4-sweep.log`, and the assertion below
/// that the count is at least one keeps a mistyped needle from passing on an empty answer.
///
/// ⚠ **What this test does NOT say** is that the one door is *safe*. The mechanism inside it is
/// still `ShellExecuteW`, and under `uiAccess` that is the finding's other half; both roads the
/// terms of reference named for repairing it are closed in this crate's configuration
/// (`scratchpad-E41\probe-shell-T-41-4.md`), and the question went to the owner as дополнение
/// 125б. One door is what a second door costs nothing to become, and that is why it is worth
/// having on its own.
#[test]
fn the_program_asks_for_something_to_be_opened_in_exactly_one_place() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut doors = Vec::new();
    let mut files = 0;

    for entry in fs::read_dir(&sources).expect("the source directory must be readable") {
        let path = entry.expect("a directory entry must be readable").path();

        if path.extension().is_none_or(|kind| kind != "rs") {
            continue;
        }

        files += 1;

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let text = fs::read_to_string(&path).expect("a source file must be readable");

        for (number, line) in text.lines().enumerate() {
            // Comments first: the repair explains itself at length, and an explanation is not
            // a door.
            if line.trim_start().starts_with("//") {
                continue;
            }

            if line.contains("ShellExecuteW(") {
                doors.push(format!("{name}:{}", number + 1));
            }
        }
    }

    println!("files read: {files}; doors found: {doors:?}");

    assert!(
        files >= 15,
        "the sweep must have read the whole of src\\, not {files} files"
    );
    assert_eq!(
        doors.len(),
        1,
        "this program may ask for something to be opened in exactly one place — found {doors:?}"
    );

    // And it is the shared door, not one of the three the finding names.
    let settings = fs::read_to_string(sources.join("settings.rs"))
        .expect("src\\settings.rs must be readable")
        .replace("\r\n", "\n");

    let at = settings
        .find("pub fn open_in_the_shell(target: &str) -> bool {")
        .expect("the one door is `settings::open_in_the_shell`");
    let body = &settings[at..];
    let end = body
        .find("\n}")
        .expect("a function closes with a brace of its own");

    assert!(
        body[..end].contains("ShellExecuteW("),
        "and the one call the sweep found stands inside it"
    );

    // The three callers, by name, so that a fourth has to be added here on purpose.
    let letters = fs::read_to_string(sources.join("letters.rs"))
        .expect("src\\letters.rs must be readable")
        .replace("\r\n", "\n");

    assert_eq!(
        letters.matches("settings::open_in_the_shell(").count(),
        2,
        "«Читать на сайте» and «Сохранить в папку журнала» both go through the door"
    );
    // «Открыть папку журнала» of the window of the settings on the shell (T-120-20).
    assert_eq!(
        window_of_settings_source()
            .matches("settings::open_in_the_shell(&folder.display().to_string())")
            .count(),
        1,
        "and so does «Открыть папку журнала»"
    );
}

/// The windows of the old kind that became one family in stage Э81 — решение 142.2 п. 2 и п. 3.
/// Five then; «О программе» has left the templates for the shell (FR-92а), the letters and «Последние
/// письма» too (T-120-16).
///
/// The settings dialog is deliberately absent: it is 430 units wide by the owner's word and its
/// controls keep their own columns. Its width is held below all the same, so that «the four came
/// to 256» cannot quietly become «all six did».
const FAMILY_OF_E81: [&str; 2] = ["IDD_AUTHOR", "IDD_WIZARD"];

/// One `DIALOGEX` of `app.rc`: its name, its `(cx, cy)`, and every control statement inside it
/// as `(statement, x, y, cx, cy)`.
struct RcDialog {
    name: String,
    size: (i32, i32),
    controls: Vec<(String, i32, i32, i32, i32)>,
}

/// Reads every `DIALOGEX` out of the **text** of `app.rc`.
///
/// ⚠ The text and not the compiled template, and that is the point: the sweep has to be runnable
/// against the `app.rc` of another wave, which is what makes the negative control possible at all.
///
/// Quoted strings are blanked before a line is split on commas — a caption of this file carries
/// commas of its own («Исправляет текст, набранный не в той раскладке.»), and a parser that
/// counted them would read the wrong four numbers. The four numbers of a statement are the **last
/// four integers** it names, whatever stands before them: `ICON` names a resource first, `CONTROL`
/// names a class, `EDITTEXT` names nothing at all.
fn dialogs_of_the_resource(text: &str) -> Vec<RcDialog> {
    /// Every statement that puts a control into a template — the whole vocabulary this file uses.
    const STATEMENTS: [&str; 15] = [
        "ICON",
        "LTEXT",
        "RTEXT",
        "CTEXT",
        "PUSHBUTTON",
        "DEFPUSHBUTTON",
        "CONTROL",
        "EDITTEXT",
        "COMBOBOX",
        "GROUPBOX",
        "LISTBOX",
        "SCROLLBAR",
        "AUTOCHECKBOX",
        "CHECKBOX",
        "AUTORADIOBUTTON",
    ];

    let blanked = |line: &str| -> String {
        let mut out = String::with_capacity(line.len());
        let mut inside = false;

        for character in line.chars() {
            if character == '"' {
                inside = !inside;
                continue;
            }

            out.push(if inside { ' ' } else { character });
        }

        out
    };

    let mut dialogs: Vec<RcDialog> = Vec::new();
    let mut open: Option<RcDialog> = None;

    for line in text.replace("\r\n", "\n").lines() {
        let trimmed = line.trim();

        if let Some((name, tail)) = trimmed.split_once(" DIALOGEX ")
            && !name.contains(' ')
            && !name.starts_with("//")
        {
            let numbers: Vec<i32> = tail
                .split(',')
                .filter_map(|field| field.trim().parse::<i32>().ok())
                .collect();

            assert_eq!(
                numbers.len(),
                4,
                "the DIALOGEX line of {name} must name four numbers: {trimmed}"
            );

            open = Some(RcDialog {
                name: name.to_owned(),
                size: (numbers[2], numbers[3]),
                controls: Vec::new(),
            });

            continue;
        }

        if trimmed == "END"
            && let Some(dialog) = open.take()
        {
            dialogs.push(dialog);
            continue;
        }

        let Some(dialog) = open.as_mut() else {
            continue;
        };

        let statement = trimmed.split_whitespace().next().unwrap_or_default();

        if !STATEMENTS.contains(&statement) {
            continue;
        }

        let numbers: Vec<i32> = blanked(trimmed)
            .split(',')
            .filter_map(|field| field.trim().parse::<i32>().ok())
            .collect();

        assert!(
            numbers.len() >= 4,
            "a control statement of {} must name at least four numbers: {trimmed}",
            dialog.name
        );

        let tail = &numbers[numbers.len() - 4..];

        dialog
            .controls
            .push((trimmed.to_owned(), tail[0], tail[1], tail[2], tail[3]));
    }

    assert!(open.is_none(), "a DIALOGEX of app.rc was never closed");

    dialogs
}

/// **The sweep of task T-81-1** — answers what is wrong with an `app.rc`, or nothing at all.
///
/// Separate from the test so that it can be turned on a text that is **not** the tree's: that is
/// the negative control, and without it a sweep is a thing that cannot fail.
fn what_breaks_the_family_of_e81(text: &str) -> Vec<String> {
    /// The outer margin of the family — решение 142.2 п. 3.
    const MARGIN: i32 = 12;
    /// The width of every window of the family — решение 142.2 п. 2.
    const WIDTH: i32 = 256;

    let dialogs = dialogs_of_the_resource(text);
    let mut broken = Vec::new();

    // Six in stage Э81; five since «О программе» moved to the shell (FR-92а); three since the letters
    // and «Последние письма» did (T-120-16); two since the settings did (T-120-20).
    if dialogs.len() != 2 {
        broken.push(format!(
            "app.rc declares {} dialogs, not two",
            dialogs.len()
        ));
    }

    for name in FAMILY_OF_E81 {
        let Some(dialog) = dialogs.iter().find(|dialog| dialog.name == name) else {
            broken.push(format!("{name} is missing from app.rc"));
            continue;
        };

        if dialog.size.0 != WIDTH {
            broken.push(format!(
                "{name} is {} units wide and the family is {WIDTH}",
                dialog.size.0
            ));
        }

        for (statement, x, _, cx, _) in &dialog.controls {
            let head = statement
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");

            if *x < MARGIN {
                broken.push(format!(
                    "{name}: «{head}…» starts at {x}, left of the margin of {MARGIN}"
                ));
            }

            if x + cx > dialog.size.0 - MARGIN {
                broken.push(format!(
                    "{name}: «{head}…» ends at {}, past the margin of {MARGIN} at {}",
                    x + cx,
                    dialog.size.0 - MARGIN
                ));
            }
        }

        // ⭐ **«Не уже поля» is not «the margin is twelve».** The wizard of `e80` was laid out on
        // a margin of sixteen and broke no rule above: nothing of it stood left of twelve and
        // nothing ran past the edge. The margin is a number the window **reaches**, on both
        // sides, and решение 142.2 п. 3 says which number.
        let left = dialog.controls.iter().map(|(_, x, ..)| *x).min();
        let right = dialog.controls.iter().map(|(_, x, _, cx, _)| x + cx).max();

        if left != Some(MARGIN) {
            broken.push(format!(
                "{name}: the leftmost control stands at {left:?} and the margin is {MARGIN}"
            ));
        }

        if right != Some(dialog.size.0 - MARGIN) {
            broken.push(format!(
                "{name}: the rightmost control ends at {right:?} and the margin is {MARGIN} at {}",
                dialog.size.0 - MARGIN
            ));
        }
    }

    broken
}

/// **Task T-81-1, решение 142.2 п. 2 и п. 3** — the four windows come to 256 units, the wizard
/// comes to the same outer margin, and the settings dialog stays where the owner left it.
///
/// ⚠ **The red before.** On `e80` this sweep names twenty-nine faults at once: three windows 191
/// units wide, one 246, and every control of the wizard standing on the margin of 16 it was laid
/// out with. That red is the whole of what the task closes.
///
/// ⛔ A sweep that cannot fail is not a check, so the two controls below turn it on texts that
/// must be rejected: the resource of `e80` (the negative control) and the resource of this tree
/// with one window put back to 191 (the mutant).
#[test]
fn the_five_windows_of_the_family_share_one_width_and_one_margin() {
    let resource = Path::new(env!("CARGO_MANIFEST_DIR")).join("app.rc");
    let text = fs::read_to_string(&resource).expect("app.rc must be readable");

    let broken = what_breaks_the_family_of_e81(&text);

    for fault in &broken {
        println!("СВИП: {fault}");
    }

    assert!(
        broken.is_empty(),
        "the family of решение 142.2 is broken in {} places: {broken:#?}",
        broken.len()
    );

    // Контроль прибора, первый — **отрицательный**: the resource as `e80` had it. Three windows
    // were 191 units wide and one 246 — «От автора» stands for them here, the about template
    // being gone; a sweep that passes this text is measuring nothing.
    let of_e80 = text.replace(
        "IDD_AUTHOR DIALOGEX 0, 0, 256,",
        "IDD_AUTHOR DIALOGEX 0, 0, 246,",
    );

    assert_ne!(
        of_e80, text,
        "the control must change the text it is made of"
    );
    assert!(
        !what_breaks_the_family_of_e81(&of_e80).is_empty(),
        "the sweep does not see the resource of e80 — it cannot fail"
    );

    // Контроль прибора, второй — **мутант**: one window put back and the sweep must name it.
    let mutant = text.replace(
        "IDD_WIZARD DIALOGEX 0, 0, 256,",
        "IDD_WIZARD DIALOGEX 0, 0, 191,",
    );

    assert_ne!(
        mutant, text,
        "the mutant must change the text it is made of"
    );

    let names = what_breaks_the_family_of_e81(&mutant);

    assert!(
        names
            .iter()
            .any(|fault| fault.starts_with("IDD_WIZARD is 191")),
        "the sweep must name the window that was put back: {names:#?}"
    );

    // And the parser really did read the templates, rather than answering «nothing is wrong»
    // because it found nothing at all.
    let dialogs = dialogs_of_the_resource(&text);

    for dialog in &dialogs {
        println!(
            "{}: {:?} units, {} controls",
            dialog.name,
            dialog.size,
            dialog.controls.len()
        );
    }

    assert!(
        dialogs.iter().all(|dialog| !dialog.controls.is_empty()),
        "every template of app.rc must carry controls"
    );
}

/// Where the bottom row of one window stands, in dialog units — the pure half of the guard of
/// task T-81-2.
#[derive(Debug, PartialEq, Eq)]
struct BottomRow {
    /// The height every button of the row shares, or the heights that differ.
    heights: Vec<i32>,
    /// The air under the row — the window's height less the bottom of the row.
    under: i32,
    /// The air to the right of the row — the window's width less the right edge of the row.
    right: i32,
    /// The gaps inside the right-hand run, left to right.
    gaps: Vec<i32>,
    /// Where each button standing outside the right-hand run starts, left to right.
    ///
    /// ⛔ A `Vec` and not an `Option`, and that is the hole the first draft of this guard had.
    /// A run broken in the middle leaves **two** buttons outside it, and a guard that only asked
    /// «where does the leftmost one stand» would have called the wizard's row правильным with
    /// «Назад» knocked five units out of line: the run would be «Далее» alone, the gaps empty,
    /// and the leftmost button still at twelve. Everything outside the run is counted, and more
    /// than one thing outside it is itself the fault.
    apart: Vec<i32>,
}

/// **Where the bottom row of a template stands — задача T-81-2, решение 142.2 п. 7.**
///
/// The row is every control sharing the lowest `y` of the template: after this task nothing of
/// any of the six windows stands under its buttons, and that is itself part of the rule — the
/// quiet line of «Письмо» and «Последние письма» moved **above** the row, where the accepted
/// mock-up puts it.
///
/// ⚠ The right-hand run is walked from the right edge inwards while the gap is exactly the air
/// of the rule; what is left of it is the button of the other kind («От автора…», «Отмена»),
/// which stands in the left margin. That is the shape решение 142.2 п. 7 names, and the settings
/// dialog — three buttons in one run and nothing apart — is the same shape with an empty
/// remainder.
fn bottom_row_of(dialog: &RcDialog, gap: i32) -> BottomRow {
    let lowest = dialog
        .controls
        .iter()
        .map(|(_, _, y, ..)| *y)
        .max()
        .expect("a template must carry controls");

    let mut row: Vec<(i32, i32, i32)> = dialog
        .controls
        .iter()
        .filter(|(_, _, y, ..)| *y == lowest)
        .map(|(_, x, _, cx, cy)| (*x, *x + *cx, *cy))
        .collect();

    row.sort_unstable();

    let heights = row.iter().map(|(_, _, cy)| *cy).collect();
    let bottom = lowest + row.iter().map(|(_, _, cy)| *cy).max().unwrap_or(0);
    let right = dialog.size.0 - row.last().map_or(0, |(_, edge, _)| *edge);

    // The run, from the right edge inwards, while the gap is the one of the rule.
    let mut first_of_run = row.len() - 1;

    while first_of_run > 0 && row[first_of_run].0 - row[first_of_run - 1].1 == gap {
        first_of_run -= 1;
    }

    let gaps = row[first_of_run..]
        .windows(2)
        .map(|pair| pair[1].0 - pair[0].1)
        .collect();

    let apart = row[..first_of_run].iter().map(|(x, ..)| *x).collect();

    BottomRow {
        heights,
        under: dialog.size.1 - bottom,
        right,
        gaps,
        apart,
    }
}

/// **Task T-81-2, решение 142.2 п. 7** — one bottom row in all six windows of the program.
///
/// The rule, in the owner's own composition: buttons **14** units tall, the row pinned to the
/// bottom with **12** under it, the right edge of the last button **12** from the right edge of
/// the window, **4** between neighbours, and the button of another kind — «От автора…»,
/// «Отмена» — standing apart in the left margin at **12**.
///
/// ⚠ **The red before.** On the tree this task starts from the rule holds nowhere: «О программе»
/// and the settings dialog leave **7** under their rows and 8 and 7 to the right of them;
/// «Письмо» and «Последние письма» keep **26** under theirs, because the quiet line stands below
/// the buttons instead of above them; «От автора» puts its button **past** the bottom edge of its
/// template altogether; and the wizard leaves 26. Six windows, six faults.
///
/// ⛔ The settings dialog is in this test although its width is the owner's frozen 430: the row
/// was promised «во всех окнах», and 141а names that dialog as the very model the other five were
/// to come to.
#[test]
fn every_window_of_the_program_carries_the_same_bottom_row() {
    /// The air of решение 142.2 п. 7: under the row, right of it, and between two buttons.
    const MARGIN: i32 = 12;
    const GAP: i32 = 4;
    /// The height of a push button — `letters::air::BUTTON`, written out as a literal here for
    /// the reason every template test writes its numbers out.
    const BUTTON: i32 = 14;

    let resource = Path::new(env!("CARGO_MANIFEST_DIR")).join("app.rc");
    let text = fs::read_to_string(&resource).expect("app.rc must be readable");
    let dialogs = dialogs_of_the_resource(&text);

    let mut broken = Vec::new();

    // «О программе» has left the templates for the shell (FR-92а), and the letters and «Последние
    // письма» too (T-120-16), and the settings (T-120-20): their row of buttons is the passport's.
    for name in ["IDD_AUTHOR", "IDD_WIZARD"] {
        let dialog = dialogs
            .iter()
            .find(|dialog| dialog.name == name)
            .unwrap_or_else(|| panic!("app.rc must declare {name}"));

        let row = bottom_row_of(dialog, GAP);

        println!("{name}: {row:?}");

        if row.heights.iter().any(|height| *height != BUTTON) {
            broken.push(format!("{name}: the row is {:?} units tall", row.heights));
        }

        // ⭐ **Исключения нет ни у одного окна, включая настройки — решение 142.5.** Под их
        // рядом было семь единиц, и пяти до двенадцати в высоте 375 не было: «Состояние»
        // кончается на 348, а правило просит под панелью 6 + 14 + 12 = 32 из бывших 27. Владелец
        // выбрал дать окну эти пять (375 → 380) вместо того, чтобы двигать его внутренность или
        // жать воздух над рядом. Шесть окон — одно правило, без оговорок.
        if row.under != MARGIN {
            broken.push(format!("{name}: {} units under the row", row.under));
        }

        if row.right != MARGIN {
            broken.push(format!("{name}: {} units right of the row", row.right));
        }

        if row.gaps.iter().any(|air| *air != GAP) {
            broken.push(format!("{name}: the gaps of the run are {:?}", row.gaps));
        }

        // At most one button stands outside the run, and it stands in the left margin. Two of
        // them mean the run itself is broken — see the note on the field.
        match row.apart.as_slice() {
            [] => {}
            [apart] if *apart == MARGIN => {}
            [apart] => broken.push(format!("{name}: the button apart starts at {apart}")),
            many => broken.push(format!(
                "{name}: {} buttons stand outside the run, at {many:?}",
                many.len()
            )),
        }
    }

    assert!(
        broken.is_empty(),
        "the bottom row of решение 142.2 п. 7 is broken in {} places: {broken:#?}",
        broken.len()
    );

    // Контроль прибора — **мутант**: the row of one window pushed one unit off the bottom, which
    // is the smallest thing this rule forbids. A guard that passes it is measuring nothing.
    let author = dialogs_of_the_resource(&text)
        .into_iter()
        .find(|dialog| dialog.name == "IDD_AUTHOR")
        .expect("app.rc must declare IDD_AUTHOR");

    let moved = RcDialog {
        name: author.name.clone(),
        size: (author.size.0, author.size.1 + 1),
        controls: author.controls.clone(),
    };

    assert_ne!(
        bottom_row_of(&moved, GAP).under,
        MARGIN,
        "the guard does not see a row one unit off the bottom — it cannot fail"
    );

    // And a second mutant, sideways: «Назад» knocked one unit out of the run. ⛔ This is the case
    // the first draft of the guard let through — the run shrinks to «Далее» alone, the gaps go
    // empty and the leftmost button is still at twelve. It is caught by counting what stands
    // outside the run rather than by looking only at the leftmost thing.
    let knocked = RcDialog {
        name: "IDD_WIZARD".to_owned(),
        size: (256, 268),
        controls: vec![
            ("PUSHBUTTON IDC_WZ_CANCEL".to_owned(), 12, 242, 56, 14),
            ("PUSHBUTTON IDC_WZ_BACK".to_owned(), 142, 242, 46, 14),
            ("PUSHBUTTON IDC_WZ_NEXT".to_owned(), 193, 242, 51, 14),
        ],
    };

    let row = bottom_row_of(&knocked, GAP);

    println!("мутант «Назад» на единицу в сторону: {row:?}");

    assert_eq!(
        row.apart,
        vec![12, 142],
        "a button knocked out of the run must be counted as standing outside it: {row:?}"
    );
}

/// **Task T-81-2, развилка C3** — every bottom row this stage touched still fits its window in
/// all fourteen languages.
///
/// Two kinds of row are measured, because the program has two kinds:
///
/// * a row whose widths are **frozen in `app.rc`** — «О программе» and the settings dialog. The
///   caption has to fit the slot the template gives it, and there is nowhere for it to go if it
///   does not.
/// * a row the code lays out — «Письмо», «Последние письма», «От автора» and the wizard. There
///   `letters::button_width` gives every button the width of its own caption plus the air, so a
///   caption cannot be clipped; what **can** happen is that the row as a whole runs off the
///   window, and that is what is measured here.
///
/// ⚠ The air of the second kind is read out of `src\letters.rs` rather than written down twice:
/// the literals below are held against that file, so a change to `air::BUTTON_GAP` or
/// `air::BUTTON_PAD` cannot leave this stand measuring the rule of a previous wave.
///
/// ⚠ Замок `with_product_strings` обязателен: язык интерфейса — величина процесса.
#[test]
fn every_bottom_row_fits_its_window_in_all_fourteen_languages() {
    /// The window of the family, in dialog units — решение 142.2 п. 2.
    const FAMILY_WIDTH: i32 = 256;

    let _guard = with_product_strings();

    // The air, read out of the module that uses it.
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("letters.rs"),
    )
    .expect("src\\letters.rs must be readable")
    .replace("\r\n", "\n");

    for (name, value) in [
        ("PAD", 12),
        ("BUTTON", 14),
        ("BUTTON_PAD", 10),
        ("BUTTON_MIN", 40),
        ("BUTTON_GAP", 4),
    ] {
        assert!(
            source.contains(&format!("const {name}: i32 = {value};")),
            "air::{name} is not the {value} this stand measures — the rule moved and the \
             instrument did not"
        );
    }

    let margin = 12;
    let gap = 4;
    let button_pad = 10;
    let button_min = 40;

    let product = ProductImage::open();
    let letter = DialogTemplate::parse(&product.resource(RT_DIALOG, letters::IDD_AUTHOR));

    // Every template of the program declares the one face, FONT 10 «Segoe UI».
    let font = letter
        .font
        .clone()
        .expect("the template of «От автора» declares DS_SETFONT");

    let sheet = Sheet::new(64);

    // Контроль прибора, at the template's own face: a caption three times over must not fit the
    // slot it fits once. Without it this stand is a thing that cannot fail.
    {
        let face = Face::new(manager_logfont(sheet.dc, &font, CLEARTYPE_QUALITY));
        let alphabet = extent_of(
            &sheet,
            &face,
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        )
        .cx;
        let unit_x = (alphabet / 26 + 1) / 2;
        let tripled = extent_of(&sheet, &face, &"Применить".repeat(3)).cx;
        assert!(
            tripled > (63 * unit_x + 2) / 4,
            "the instrument cannot see a clip at all: {tripled} px in {} px",
            (63 * unit_x + 2) / 4
        );
    }

    // The bottom row of every letter the program can show, **pair by pair as `letters.rs` builds
    // them** — `left` and `accent`; the middle slot is never given a button.
    //
    // ⛔ The first draft of this stand took the **two widest captions of the whole list**, on the
    // grounds that it was the stricter measure. It is stricter than the program: it put
    // «Відкрити налаштування» beside a caption from another letter and reported развилка C3 at
    // 257 units of 256 for a row that no window ever shows. A measure stricter than the thing
    // measured is not a safety margin, it is a false red — and a false red spends the owner's
    // word on a question he does not have.
    let letters_of_the_program: [(&str, u16, u16); 4] = [
        (
            "«Здравствуйте»",
            settings::IDS_HELLO_SETTINGS,
            settings::IDS_HELLO_OK,
        ),
        ("«Спасибо»", settings::IDS_CHANNEL_OPEN, settings::IDS_CLOSE),
        (
            "«Доступна версия»",
            settings::IDS_CLOSE,
            settings::IDS_NEWS_DOWNLOAD,
        ),
        (
            "«Новость»",
            settings::IDS_NEWS_LATER,
            settings::IDS_NEWS_READ_BUTTON,
        ),
    ];

    let mut clips = Vec::new();

    // Задача T-83-4, развилка C3: at every face the rule may open the windows at.
    for pixels in rule_faces() {
        let face = Face::new(manager_face_at(&sheet, &font, pixels));
        let alphabet = extent_of(
            &sheet,
            &face,
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
        )
        .cx;
        let unit_x = (alphabet / 26 + 1) / 2;
        let mut tightest = (i32::MAX, String::new());

        for language in Language::shipped() {
            settings::set_ui_language(language);

            // A button the code lays out is as wide as its caption plus the air, never narrower
            // than the floor — the arithmetic of `letters::button_width`, in units.
            let wanted = |string: u16| -> i32 {
                let caption = settings::text(string);
                let text_units = (extent_of(&sheet, &face, &caption).cx * 4 + unit_x - 1) / unit_x;
                (text_units + button_pad * 2).max(button_min)
            };

            let mut rows: Vec<(String, Vec<i32>)> = letters_of_the_program
                .iter()
                .map(|(what, left, accent)| {
                    (
                        format!("«Письмо» {what}"),
                        vec![wanted(*left), wanted(*accent)],
                    )
                })
                .collect();

            rows.push((
                "«Последние письма»".to_owned(),
                vec![wanted(settings::IDS_CLOSE)],
            ));
            rows.push(("«От автора»".to_owned(), vec![wanted(settings::IDS_CLOSE)]));
            rows.push((
                "мастер".to_owned(),
                vec![
                    wanted(settings::IDS_WIZARD_CANCEL),
                    wanted(settings::IDS_WIZARD_BACK),
                    wanted(settings::IDS_WIZARD_NEXT),
                ],
            ));

            for (what, widths) in rows {
                let count = i32::try_from(widths.len()).unwrap_or(0);
                let taken: i32 = widths.iter().sum::<i32>() + gap * (count - 1) + margin * 2;
                let spare = FAMILY_WIDTH - taken;

                if spare < tightest.0 {
                    tightest = (spare, format!("{language:?} {what} (units)"));
                }

                if taken > FAMILY_WIDTH {
                    clips.push(format!(
                        "{pixels} px, {language:?} {what}: the row wants {taken} units of the \
                         {FAMILY_WIDTH} the window has"
                    ));
                }
            }
        }

        println!(
            "{pixels} px (unit {unit_x}/4): the tightest {} to spare — {}",
            tightest.0, tightest.1
        );
    }

    settings::set_ui_language(Language::Ru);

    assert!(
        clips.is_empty(),
        "развилка C3: {} rows do not fit: {clips:#?}",
        clips.len()
    );
}

/// **Task T-81-3, решение 142.2 п. 4** — the head of «От автора» is its template's.
///
/// The head was the head of the old «О программе», rectangle for rectangle, and this test held
/// the two templates equal; that template went with FR-92а, when «О программе» moved to the
/// shell. What stays is the name — a button that takes no focus — and the layout that leaves the
/// head where the template puts it.
#[test]
fn the_head_of_both_windows_is_the_same_head() {
    /// The ordinal of the predefined `Button` class in a dialog template.
    const BUTTON_CLASS: u16 = 0x0080;
    /// The type of a button is the low nibble of its style.
    const BS_OWNERDRAW: u32 = 0x0B;
    /// `WS_TABSTOP`, which a `PUSHBUTTON` statement carries unless the template says otherwise.
    const TABSTOP: u32 = 0x0001_0000;

    let product = ProductImage::open();
    let author = DialogTemplate::parse(&product.resource(RT_DIALOG, 204));

    // The name of «От автора» is a button, and takes no focus — решение 142.2 п. 6.
    let class = author
        .classes
        .iter()
        .find(|(control, _)| *control == 1261)
        .map(|(_, class)| *class)
        .expect("the author template must carry the name");

    assert_eq!(
        class,
        Some(BUTTON_CLASS),
        "the name must be a button of the predefined class"
    );

    let style = author.style_of(1261, "имя «От авторе»");

    println!("имя «От авторе» (1261): style {style:#010x}");

    assert_eq!(
        style & 0x0F,
        BS_OWNERDRAW,
        "and an owner-drawn one, or this program does not paint it at all"
    );
    assert_eq!(
        style & TABSTOP,
        0,
        "and not a tab stop: it stands first in the template"
    );

    // ⛔⛔⛔ **И «От авторе» обязано этот шаблон УВАЖАТЬ — находка 142б.1, задача T-81-6.**
    //
    // Всё, что выше, было зелено и на `e81`, а живое окно выглядело иначе: `layout_author`
    // выбрасывал прямоугольники шапки и клал её заново своей арифметикой. Имя уезжало вправо на
    // инсет подложки, низ подложки срезался, между версией и описанием появлялся лишний воздух.
    // **Совпадение шаблонов — не совпадение окон, если окно кладёт себя само.**
    //
    // Поэтому здесь держится механизм: тело раскладки не должно ставить **ни один** контрол
    // шапки. Оно спрашивает у них, где шапка кончилась, и начинает под ней.
    let letters = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("letters.rs"),
    )
    .expect("src\\letters.rs must be readable")
    .replace("\r\n", "\n");

    let at = letters
        .find("unsafe fn layout_author(")
        .expect("src\\letters.rs must hold the layout of «От автора»");
    let rest = &letters[at..];
    let end = rest
        .find("\n}")
        .expect("a function closes with a brace of its own");
    let body = &rest[..end];

    // ⚠ The body is read with its whitespace squeezed out: `cargo fmt` breaks a call over five
    // lines or one as the arguments happen to fit, and a needle that spelled the indentation
    // would have been blind to the very code this guard was written against. The first draft of
    // it **was** — it spelled twelve spaces where `e81` had eight.
    let squeezed: String = body.chars().filter(|c| !c.is_whitespace()).collect();

    let places =
        |text: &str, control: &str| -> bool { text.contains(&format!("place(hwnd,{control},")) };

    for control in [
        "IDC_AUTHOR_ICON",
        "IDC_AUTHOR_NAME",
        "IDC_AUTHOR_VERSION",
        "IDC_AUTHOR_LINE_1",
        "IDC_AUTHOR_LINE_2",
    ] {
        assert!(
            !places(&squeezed, control),
            "«От автора» must leave {control} where the template put it — the head of решение \
             142.2 п. 4 is the template's"
        );
    }

    assert!(
        squeezed.contains("settings::child_rect_in_client(hwnd,control)"),
        "and it must ask the head's own controls where it ended, rather than re-deriving it"
    );

    // ⛔ **Отрицательный контроль: тело, которое кладёт шапку рукой** — ровно то, чем `e81` и
    // отличался. Форма контроля — та же, что у сторожей T-78-3 и T-80-2: тело с возвращённым
    // дефектом. ⚠ Проверено ещё и рукой, вне теста: в `layout_author` коммита `df29bf1`
    // `place(hwnd, IDC_AUTHOR_NAME, …)` стоял, и эта игла его находит.
    let by_hand = squeezed.replace(
        "lethead=[",
        "place(hwnd,IDC_AUTHOR_NAME,metrics.x(air::TEXT_X),top,0,0);lethead=[",
    );

    assert_ne!(
        by_hand, squeezed,
        "the control must change the body it is made of"
    );
    assert!(
        places(&by_hand, "IDC_AUTHOR_NAME"),
        "the sweep does not see a head laid out by hand — it cannot fail"
    );
}

/// **Task T-81-3, решение 142.2 п. 5** — `IDS_AUTHOR_VERSION` is gone from the resource, with all
/// fourteen of its translations.
///
/// ⚠ The tombstone in `app.rc` names it in prose, and prose is not a string table: the sweep
/// looks for the **statement** — the identifier followed by a quoted string, and the `#define`
/// that gave it a number — so a comment about the removal does not read as the removal undone.
#[test]
fn the_authors_signature_left_the_resource_with_all_fourteen_translations() {
    // Since stage L10N the tables are res/lang/<tag>.rc: app.rc alone would answer zero for the
    // wrong reason, and the mutant below is what told (red before T-L10N-2 switched to this).
    let text = resource_script::resource_script();

    let statements = |body: &str| -> usize {
        body.lines()
            .filter(|line| {
                let line = line.trim_start();
                line.starts_with("IDS_AUTHOR_VERSION") && line.contains('"')
            })
            .count()
    };

    let defines = |body: &str| -> usize {
        body.lines()
            .filter(|line| line.trim_start().starts_with("#define IDS_AUTHOR_VERSION"))
            .count()
    };

    println!(
        "IDS_AUTHOR_VERSION: {} statements, {} defines",
        statements(&text),
        defines(&text)
    );

    assert_eq!(
        statements(&text),
        0,
        "all fourteen translations of the author's signature must be gone from app.rc and its \
         language files"
    );
    assert_eq!(defines(&text), 0, "and so must the number that named it");

    // Контроль прибора — **мутант**: put one translation back and the sweep must see it. Without
    // this the two zeroes above would also be what a sweep looking at the wrong file answers.
    let mutant = text.replace(
        "    IDS_AUTHOR_PANEL        \"Автор\"",
        "    IDS_AUTHOR_VERSION      \"версия {0} · Панда, он же Panda_Pishet_Kod\"\n    \
         IDS_AUTHOR_PANEL        \"Автор\"",
    );

    assert_ne!(
        mutant, text,
        "the mutant must change the text it is made of"
    );
    assert!(
        statements(&mutant) > 0,
        "the sweep does not see a translation put back — it cannot fail"
    );

    // And the tombstone really is prose: the sweep must be blind to it, or it would be reading
    // comments for statements.
    assert!(
        text.contains("`IDS_AUTHOR_VERSION` — УДАЛЕНА задачей T-81-3"),
        "the tombstone must stand, so that the removal reads as a decision"
    );
}

/// **Task T-81-4, решение 142.2 п. 8** — three windows of one height, and two of their own.
///
/// ⭐ **276 единиц, и вот почему именно оно.** The typical height is the tallest of the three
/// windows that share it, measured **after** the widening — and the tallest is «Письмо» with the
/// Greek «Здравствуйте»: 276 units (`scratchpad-E81\probe-letter-height.log`). «О программе»
/// needs 242 of its own and takes the other 34 into its help panel rather than into air under it;
/// the wizard's 268 was measured in an earlier wave for the Greek step «Что приложить?» at a
/// **narrower** slot (224 units against today's 232), so a wider slot cannot want more.
///
/// ⚠ The control that makes the 276 believable: the same instrument, run at the **old** width of
/// 191, answers 320 for a window that lived at 308 — it reads high, never low, because it
/// measures a row that may carry a key chip with `theme::measure_chip_row` where the window uses
/// the narrower `chip_overhang`. An instrument that errs towards «taller» is the safe one for
/// choosing a height.
///
/// ⛔ **«Два других — нет» is an assertion and not forgetfulness.** «От автора» and «Последние
/// письма» compute their own height from their content (решение 142.2 п. 8), so a template height
/// equal to the typical one would mean somebody had quietly made them typical too.
#[test]
fn three_windows_share_one_height_and_two_keep_their_own() {
    /// The height of решение 142.2 п. 8 — a literal, as everywhere in these template tests.
    const TYPICAL: i32 = 276;

    let resource = Path::new(env!("CARGO_MANIFEST_DIR")).join("app.rc");
    let text = fs::read_to_string(&resource).expect("app.rc must be readable");
    let dialogs = dialogs_of_the_resource(&text);

    let height = |name: &str| -> i32 {
        dialogs
            .iter()
            .find(|dialog| dialog.name == name)
            .unwrap_or_else(|| panic!("app.rc must declare {name}"))
            .size
            .1
    };

    // «О программе» shared it until FR-92а moved it to the shell, «Письмо» until T-120-16 did.
    {
        let name = "IDD_WIZARD";
        println!("{name}: {} units tall", height(name));

        assert_eq!(
            height(name),
            TYPICAL,
            "{name} must carry the typical height of решение 142.2 п. 8"
        );
    }

    // «Последние письма» counted its own until T-120-16 moved it to the shell.
    {
        let name = "IDD_AUTHOR";
        println!("{name}: {} units tall — by its content", height(name));

        assert_ne!(
            height(name),
            TYPICAL,
            "{name} counts its own height from what it says, and a template height equal to the \
             typical one would mean somebody had made it typical after all"
        );
    }

    // ⭐⭐ **And it really does count it** — решение 142.3. The count is the guard: one caller,
    // «От автора», and no second (the second, `layout_list`, went with its window in T-120-16).
    let letters = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("letters.rs"),
    )
    .expect("src\\letters.rs must be readable")
    .replace("\r\n", "\n");

    let body_of = |needle: &str| -> String {
        let at = letters
            .find(needle)
            .unwrap_or_else(|| panic!("src\\letters.rs must hold «{needle}»"));
        let rest = &letters[at..];
        let end = rest
            .find("\n}")
            .expect("a function closes with a brace of its own");
        rest[..end].to_owned()
    };

    {
        let what = "unsafe fn layout_author(";
        assert!(
            body_of(what).contains("resize_client("),
            "«{what}» must give its window the height its content asks for"
        );
    }

    assert!(
        !body_of("unsafe fn layout_wizard(").contains("resize_client("),
        "the wizard must not resize itself — a window that grew under the mouse as «Далее» was \
         pressed is the very thing its template forbids"
    );

    // Контроль прибора: the reading must refuse a body that lost the call, or the three
    // assertions above hold of nothing.
    let without = body_of("unsafe fn layout_author(").replace("resize_client(", "let _ = (");

    assert!(
        !without.contains("resize_client("),
        "the control must differ from the body in exactly the way the assertions look at"
    );

    // ⚠ **Мутант:** the typical one moved by a single unit, which is the smallest thing this rule
    // forbids. A guard that passes it is measuring nothing.
    let mutant = text.replace(
        "IDD_WIZARD DIALOGEX 0, 0, 256, 276",
        "IDD_WIZARD DIALOGEX 0, 0, 256, 277",
    );

    assert_ne!(
        mutant, text,
        "the mutant must change the text it is made of"
    );

    let moved = dialogs_of_the_resource(&mutant)
        .into_iter()
        .find(|dialog| dialog.name == "IDD_WIZARD")
        .expect("app.rc must declare IDD_WIZARD");

    assert_ne!(
        moved.size.1, TYPICAL,
        "the reading does not see a height one unit off the typical — it cannot fail"
    );
}

// =========================================================================================
// Task T-100-3 — «Исключения»: автоматически и вручную (вопрос 172, вариант В макета 172.4)
// =========================================================================================
//
// Two ways of adding a program, each under its own word, inside the group they belong to: under
// «Автоматически» the button «Выбрать окно программы» the width of the row of the manual way, with
// the line under it; under «Вручную» the field, «Добавить» and the hint that were always there.
// The capture itself is `guard::capture_foreground` (task T-100-2), and the window adds nothing to
// it but the rule of [`settings::exclusion_pick`] and the way the answer is shown.

/// **Task T-100-3 (2) — the rule of «Выбрать окно программы», as a pure function.**
///
/// What the capture found (`guard::capture_foreground(guard::Asker::Process)`, task T-100-2) and
/// the list as it stands decide one of four answers:
///
/// * nothing found — no window in front, or a window of this program — «Окно не выбрано»;
/// * the host of the Store frames when the program inside could not be told — «Окно не выбрано»,
///   rather than an exclusion of every Store program (решение 172.20);
/// * a new name — `Added`: into the list, and its row selected;
/// * a name the list holds under any spelling — `AlreadyListed` with **the row's own spelling**,
///   which is what the line under the button says and the row that is selected;
/// * a name that cannot be an exclusion — one longer than the field takes, or one that folds to
///   nothing — `Refused`, the third tone of FR-100 and «Окно не выбрано».
///
/// ⚠ «Красное до» (приём Э20): the signature of step one took nothing.
#[test]
fn the_pick_of_the_exclusions_follows_its_rule() {
    use settings::ExclusionPick as Pick;

    let existing = vec!["game.exe".to_owned(), "Setup.EXE".to_owned()];

    assert_eq!(
        settings::exclusion_pick(Some("Telegram.exe"), &existing),
        Pick::Added("Telegram.exe".to_owned()),
        "a new program goes into the list"
    );
    assert_eq!(
        settings::exclusion_pick(Some("CalculatorApp.exe"), &existing),
        Pick::Added("CalculatorApp.exe".to_owned()),
        "172.20: the program inside a Store frame is a program like any other"
    );

    for (spelling, row, own) in [("GAME.EXE", 0, "game.exe"), ("setup.exe", 1, "Setup.EXE")] {
        assert_eq!(
            settings::exclusion_pick(Some(spelling), &existing),
            Pick::AlreadyListed(row, own.to_owned()),
            "«{spelling}» folds onto row {row}, which is selected and named as it stands"
        );
    }

    for nothing in [None, Some(""), Some("   ")] {
        assert_eq!(
            settings::exclusion_pick(nothing, &existing),
            Pick::NotPicked,
            "{nothing:?}: nothing was found — «Окно не выбрано»"
        );
    }

    for host in [
        "ApplicationFrameHost.exe",
        r"C:\Windows\System32\ApplicationFrameHost.exe",
    ] {
        assert_eq!(
            settings::exclusion_pick(Some(host), &existing),
            Pick::NotPicked,
            "172.20: «{host}» would exclude every Store program — not taken"
        );
    }

    let longest = format!("{}.exe", "a".repeat(settings::EXCLUSION_NAME_CHARS - 4));
    let longer = format!("{}.exe", "a".repeat(settings::EXCLUSION_NAME_CHARS - 3));
    assert_eq!(
        settings::exclusion_pick(Some(&longest), &existing),
        Pick::Added(longest.clone()),
        "a name as long as the field takes goes in"
    );
    assert_eq!(
        settings::exclusion_pick(Some(&longer), &existing),
        Pick::Refused,
        "T-22-8: a name the field would not take is refused, not dropped on «ОК»"
    );
    assert_eq!(
        settings::exclusion_pick(Some(r"C:\"), &existing),
        Pick::Refused,
        "a name that folds to nothing is refused, as the manual way refuses it"
    );
}

/// **Task T-100-3 (3) — all fourteen tables carry the eight strings of the two ways**, each in its
/// own script, and the «5» of the line under the button is the five seconds of the capture.
///
/// ⚠ «Красное до»: `e99` carries none of them.
#[test]
fn every_locale_carries_the_strings_of_the_two_ways() {
    let product = ProductImage::shared();
    let ids = [
        settings::IDS_EXCLUSION_AUTO,
        settings::IDS_EXCLUSION_MANUAL,
        settings::IDS_EXCLUSION_PICK,
        settings::IDS_EXCLUSION_PICK_NOTE,
        settings::IDS_EXCLUSION_PICK_COUNT,
        settings::IDS_EXCLUSION_ADDED,
        settings::IDS_EXCLUSION_ALREADY,
        settings::IDS_EXCLUSION_NOT_PICKED,
    ];
    let mut wrong = Vec::new();

    for id in ids {
        assert!(
            settings::INTERFACE_STRINGS.contains(&id),
            "string {id} must be in the vocabulary of the interface"
        );
    }

    for (tag, langid, _) in shipped_locales() {
        let tag = tag.as_str();
        for id in ids {
            let string = product.string_of_langid(langid, id);

            if string.trim().is_empty() {
                wrong.push(format!("{tag}: string {id} is empty"));
            }
        }

        let note = product.string_of_langid(langid, settings::IDS_EXCLUSION_PICK_NOTE);
        if !note.contains(&lang_switcher::guard::CAPTURE_SECONDS.to_string()) {
            wrong.push(format!(
                "{tag}: «{note}» does not name the {} seconds of the capture",
                lang_switcher::guard::CAPTURE_SECONDS
            ));
        }
    }

    assert!(
        wrong.is_empty(),
        "the two ways in fourteen languages:\n{}",
        wrong.join("\n")
    );
}

// =========================================================================================
// Задача T-118-1 — Q-208-23 п. 1, обращение SKHCQ3SA: «При нажатии кнопки Задать весь интерфейс
// приложения мигает, словно оно делает перерисовку».
// =========================================================================================

/// **T-118-1 — the four bands of `widgets::repaint::ring` cover the frame and nothing of the
/// field**, pixel by pixel, at the thicknesses the settings window draws at.
///
/// A band reaching into the field would make the background picture paint over a child that is
/// not told to repaint (the settings window has no `WS_CLIPCHILDREN`); a pixel of the frame left
/// out would keep the old colour of the frame after a capture.
#[test]
fn the_bands_of_a_ring_cover_the_frame_and_nothing_of_the_field() {
    use lang_switcher::widgets::repaint::ring_bands;
    use windows::Win32::Foundation::RECT;

    let rect = |left, top, right, bottom| RECT {
        left,
        top,
        right,
        bottom,
    };

    let cases = [
        // 100 %: one pixel at the sides, the 12-unit box round an 8-unit field.
        (rect(55, 196, 211, 222), rect(56, 201, 210, 217)),
        // 200 %: two pixels at the sides.
        (rect(110, 392, 422, 444), rect(112, 402, 420, 434)),
        // `MapDialogRect` refused (NFR-13): one thickness all round.
        (rect(5, 5, 15, 15), rect(6, 6, 14, 14)),
        // Nothing round the field at all: four empty bands.
        (rect(5, 5, 15, 15), rect(5, 5, 15, 15)),
    ];

    for (outer, inner) in cases {
        let bands = ring_bands(&outer, &inner);
        let mut wrong = 0;

        for y in outer.top - 3..outer.bottom + 3 {
            for x in outer.left - 3..outer.right + 3 {
                let inside = |r: &RECT| x >= r.left && x < r.right && y >= r.top && y < r.bottom;
                let covered = bands.iter().filter(|band| inside(band)).count();
                let frame = inside(&outer) && !inside(&inner);

                if covered != usize::from(frame) {
                    wrong += 1;
                }
            }
        }

        println!(
            "outer {:?} inner {:?}: {wrong} pixels covered wrongly",
            (outer.left, outer.top, outer.right, outer.bottom),
            (inner.left, inner.top, inner.right, inner.bottom)
        );

        assert_eq!(
            wrong, 0,
            "every pixel of the frame exactly once, none of the field and none outside"
        );
    }
}

// =========================================================================================
// Задача T-118-2 — Q-208-23 п. 3, обращение SKHCQ3SA: «В текстовых полях Написать автору не
// везде работает сочетание Ctrl+A, также не везде возможен переход по текстовым полям нажатием
// Tab».
// =========================================================================================

/// **T-118-2 — the table of the fields of the wizard: every field is a tab stop of the right kind,
/// and the tab order of every step is the order of the screen.**
///
/// Measured on the stand with 1.6.0 (`reports\ITOG-E118.md`): opened from the tray, Tab and
/// Shift+Tab walk all seven fields in the order of the screen. The order and the styles were right;
/// what failed was the loop the keys came through when the wizard was opened from the settings
/// window, and the key Ctrl+A, which the field does not know (the sentries below). So this table
/// has no red «before»: it is the guard that the order and the kinds stay what the stand saw. Read
/// off the built template — the dialog manager walks the tab stops in template order.
#[test]
fn the_fields_of_the_wizard_are_tab_stops_in_the_order_of_the_screen() {
    use lang_switcher::letters::Step;

    const WS_TABSTOP: u32 = 0x0001_0000;
    const ES_MULTILINE: u32 = 0x0004;
    const ES_READONLY: u32 = 0x0800;
    const ES_WANTRETURN: u32 = 0x1000;

    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, letters::IDD_WIZARD));
    let style = |id: u32| {
        template
            .styles
            .iter()
            .find(|(control, _)| *control == id)
            .map(|(_, style)| *style)
            .unwrap_or_else(|| panic!("IDD_WIZARD must hold control {id}"))
    };

    // The seven fields and whether each is the multi-line kind.
    for (id, multi) in [
        (1314, false),
        (1326, false),
        (1328, false),
        (1334, true),
        (1336, true),
        (1346, true),
        (1358, false),
    ] {
        let s = style(id);
        println!("field {id}: style {s:#010x}");

        assert!(s & WS_TABSTOP != 0, "field {id} must be a tab stop");
        assert!(s & ES_READONLY == 0, "field {id} is typed into");
        assert_eq!(
            s & ES_MULTILINE != 0,
            multi,
            "field {id}: the kind the stand measured"
        );
        if multi {
            assert!(
                s & ES_WANTRETURN != 0,
                "Enter is a new line in field {id}, and not «Далее»"
            );
        }
    }

    // Every step ends with the row of buttons, «Отмена», «Назад», «Далее».
    let row = [1350_u32, 1351, 1352];

    for (step, expected) in [
        (Step::Where, &[1314_u32, 1315, 1317][..]),
        (Step::Did, &[1322, 1326, 1328, 1330]),
        (Step::Idea, &[1334, 1336]),
        (Step::Preview, &[1346, 1358, 1347, 1348, 1356]),
    ] {
        let shown: Vec<u32> = letters::controls_of(step)
            .iter()
            .map(|control| *control as u32)
            .chain(row)
            .collect();
        let stops: Vec<u32> = template
            .styles
            .iter()
            .filter(|(id, style)| shown.contains(id) && style & WS_TABSTOP != 0)
            .map(|(id, _)| *id)
            .collect();
        let want: Vec<u32> = expected.iter().copied().chain(row).collect();

        println!("{step:?}: tab stops in template order {stops:?}");

        assert_eq!(
            stops, want,
            "{step:?}: the tab order must be the order of the screen the stand measured"
        );
    }
}

// =========================================================================================
// Задача T-118-5 — Q-208-23 п. 2, обращение SKHCQ3SA: «кнопка Применить подсвечивается
// (остается активной) даже если в настройках программы не было сделано никаких изменений».
// Слово владельца 208.77: «А — серая без изменений».
// =========================================================================================

/// **T-118-2 — the sentry of Ctrl+A: the key selects the whole text of a field of the wizard, by
/// the road of the thread's own pump and by the road of a modal loop.**
///
/// The wizard is the product's own, opened hidden through `letters::testing::open_wizard_hidden`;
/// the keys held are the keyboard state of this thread (`SetKeyboardState`, put back afterwards),
/// which is what `GetKeyState` answers from. A single-line field and a multi-line one.
///
/// ⚠ Controls: AltGr (Ctrl+Alt — «ą» on the Polish layout), Ctrl+Shift+A and A alone select nothing;
/// a field of another thread and a list box of this one are left alone (since T-118-6 the body is
/// common to every window of the program — `widgets::keys`); and the road of a modal loop without
/// the filter — the 1.6.0 road — selects nothing, so the selection seen with it is the filter's.
///
/// **T-118-6 (208.80)** — the second test of the module: «Вручную» of the settings window, by the
/// road of that window's modal loop, and the controls of that window that are not fields.
#[cfg(feature = "testing")]
mod ctrl_a_in_the_fields_of_the_wizard {
    use super::{HiddenWindow, ProductImage, with_product_strings};
    use lang_switcher::letters;
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::Controls::{EM_GETSEL, EM_SETSEL};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardState, SetKeyboardState, VIRTUAL_KEY, VK_A, VK_CONTROL, VK_LCONTROL, VK_LSHIFT,
        VK_MENU, VK_RCONTROL, VK_RMENU, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GetDlgItem, HMENU, IsDialogMessageW, MSG, SendMessageW,
        SetWindowTextW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_KEYDOWN, WS_CHILD,
    };
    use windows::core::{HSTRING, w};

    const CTRL: &[VIRTUAL_KEY] = &[VK_CONTROL, VK_LCONTROL];
    const ALTGR: &[VIRTUAL_KEY] = &[VK_CONTROL, VK_LCONTROL, VK_MENU, VK_RMENU];
    const CTRL_SHIFT: &[VIRTUAL_KEY] = &[VK_CONTROL, VK_RCONTROL, VK_SHIFT, VK_LSHIFT];
    const NOTHING: &[VIRTUAL_KEY] = &[];

    fn selection(field: HWND) -> (usize, usize) {
        // SAFETY: a live edit control of this thread; no pointer goes with the message.
        let packed = unsafe { SendMessageW(field, EM_GETSEL, None, None) }.0 as usize;
        (packed & 0xFFFF, (packed >> 16) & 0xFFFF)
    }

    /// The text, and the caret at its end with nothing selected.
    fn put(field: HWND, text: &str) -> usize {
        let length = text.encode_utf16().count();
        // SAFETY: a live edit control of this thread; the text is copied by the call.
        unsafe {
            SetWindowTextW(field, &HSTRING::from(text)).expect("the field takes the text");
            SendMessageW(
                field,
                EM_SETSEL,
                Some(WPARAM(length)),
                Some(LPARAM(length as isize)),
            );
        }
        length
    }

    /// Runs `body` with exactly `held` down in the keyboard state of this thread.
    fn with_keys<R>(held: &[VIRTUAL_KEY], body: impl FnOnce() -> R) -> R {
        let mut saved = [0_u8; 256];
        // SAFETY: a live local array of the documented size.
        unsafe { GetKeyboardState(&mut saved) }.expect("the keyboard state of the thread");
        let mut state = [0_u8; 256];
        for key in held {
            state[usize::from(key.0)] = 0x80;
        }
        // SAFETY: as above.
        unsafe { SetKeyboardState(&state) }.expect("the keyboard state of the thread is set");
        let answer = body();
        // SAFETY: as above.
        let _ = unsafe { SetKeyboardState(&saved) };
        answer
    }

    fn ctrl_a(field: HWND) -> MSG {
        MSG {
            hwnd: field,
            message: WM_KEYDOWN,
            wParam: WPARAM(usize::from(VK_A.0)),
            lParam: LPARAM(0x001E_0001),
            ..Default::default()
        }
    }

    #[test]
    fn ctrl_a_selects_the_whole_text_of_a_field_of_the_wizard() {
        let _guard = with_product_strings();
        let wizard = HiddenWindow::on_owner(|owner| {
            letters::testing::open_wizard_hidden(owner, ProductImage::shared().module)
        });

        for (id, text) in [(1314, "alpha beta"), (1334, "alpha beta\r\ngamma delta")] {
            // SAFETY: the wizard is live until the end of the test.
            let field = unsafe { GetDlgItem(Some(wizard.window), id) }.expect("the field");

            // The pump of the thread.
            let length = put(field, text);
            // SAFETY: a message of a window of this thread, read only.
            let taken = with_keys(CTRL, || unsafe { letters::filter_message(&ctrl_a(field)) });
            println!(
                "{id}, the pump: taken {taken}, selection {:?}",
                selection(field)
            );
            assert!(taken, "{id}: Ctrl+A is the filter's");
            assert_eq!(
                selection(field),
                (0, length),
                "SKHCQ3SA: Ctrl+A must select the whole text of field {id}"
            );

            for (what, keys) in [
                ("AltGr", ALTGR),
                ("Ctrl+Shift", CTRL_SHIFT),
                ("A alone", NOTHING),
            ] {
                put(field, text);
                // SAFETY: as above.
                with_keys(keys, || unsafe { letters::filter_message(&ctrl_a(field)) });
                assert_eq!(
                    selection(field),
                    (length, length),
                    "{id}: {what} + A must not select the text"
                );
            }

            // The road of a modal loop: the loop of another window asks the dialog manager, and
            // the manager asks the filter first. Without the filter — the road of 1.6.0.
            let other = wizard.owner;

            put(field, text);
            // SAFETY: as above; `other` is a live window of this thread.
            let without = with_keys(CTRL, || {
                unsafe { IsDialogMessageW(other, &ctrl_a(field)) }.as_bool()
            });
            println!(
                "{id}, a modal loop without the filter: taken {without}, selection {:?}",
                selection(field)
            );
            assert!(
                !without && selection(field) == (length, length),
                "the positive control: without the filter the loop of another window does not \
                 reach the wizard"
            );

            put(field, text);
            let filter = letters::ModalLoopFilter::install();
            // SAFETY: as above.
            let with = with_keys(CTRL, || {
                unsafe { IsDialogMessageW(other, &ctrl_a(field)) }.as_bool()
            });
            drop(filter);
            println!(
                "{id}, a modal loop with the filter: taken {with}, selection {:?}",
                selection(field)
            );
            assert!(
                with && selection(field) == (0, length),
                "T-118-2: inside a modal loop the filter must hand Ctrl+A to the wizard"
            );
        }

        // ⚠ Задача T-118-6: the body is common to every window of the program — a field of *this
        // thread* is taken whoever its window is. The two things it must leave alone:
        //
        // 1. a field of **another thread** — a window that is not the program's;
        let (made, handed) = std::sync::mpsc::channel::<isize>();
        let (done, finish) = std::sync::mpsc::channel::<()>();
        let other = std::thread::spawn(move || {
            // SAFETY: a system class, no parent; destroyed on this thread before it ends.
            let edit = unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("EDIT"),
                    w!("alpha beta"),
                    WINDOW_STYLE(0),
                    0,
                    0,
                    100,
                    20,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .expect("an edit of another thread");
            let _ = made.send(edit.0 as isize);
            // ⚠ The thread pumps while it waits, so that its field **answers** a send — a field of a
            // thread that does not pump answers nothing, and the control would then pass for that
            // reason and not for the rule of the thread (the first take of the break Q2 of T-118-6).
            let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
            while finish.try_recv().is_err() {
                // SAFETY: the ordinary pump of this thread, over its own messages.
                unsafe {
                    while windows::Win32::UI::WindowsAndMessaging::PeekMessageW(
                        &mut msg,
                        None,
                        0,
                        0,
                        windows::Win32::UI::WindowsAndMessaging::PM_REMOVE,
                    )
                    .as_bool()
                    {
                        windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            // SAFETY: made above on this thread.
            let _ = unsafe { DestroyWindow(edit) };
        });
        let foreign = HWND(handed.recv().expect("the other thread made its field") as *mut _);
        let taken = with_keys(CTRL, || {
            lang_switcher::widgets::keys::select_all_on_ctrl_a(&ctrl_a(foreign))
        });
        let _ = done.send(());
        other.join().expect("the other thread ends");
        assert!(
            !taken,
            "a field of another thread is not the program's and is left to its own window"
        );

        // 2. a window of this thread that is **not a field** — a list box answers no
        //    `DLGC_HASSETSEL`.
        // SAFETY: a system class, a live parent of this thread; destroyed below.
        let list = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("LISTBOX"),
                None,
                WINDOW_STYLE(WS_CHILD.0),
                0,
                0,
                100,
                60,
                Some(wizard.owner),
                Some(HMENU(std::ptr::without_provenance_mut(1050))),
                None,
                None,
            )
        }
        .expect("a list box of this thread");
        let taken = with_keys(CTRL, || {
            lang_switcher::widgets::keys::select_all_on_ctrl_a(&ctrl_a(list))
        });
        assert!(!taken, "a list box is not a field: Ctrl+A stays its own");
        // SAFETY: made above on this thread.
        let _ = unsafe { DestroyWindow(list) };
    }
}

/// **Task T-81-3, решение 142.2 п. 6** — one colour rule for the name of the program.
///
/// The name «Lang Switcher» is drawn without a frame, and the table answers that for the one
/// window of the old kind that still carries it, «От автора». «О программе» carried it too until
/// FR-92а moved that window to the shell: its old number must not be a name any more, or a
/// control of some later template that took it would come out frameless. The negative control
/// shows that this table answers different things to different controls at all.
#[test]
fn both_names_of_the_program_get_one_and_the_same_pair_of_colours() {
    /// The number the old «О программе» gave its name in `app.rc`.
    const OLD_ABOUT_NAME: i32 = 1121;

    for hot in [false, true] {
        let name = settings::button_color_roles(letters::IDC_AUTHOR_NAME, hot, false, false);

        println!("hot={hot}: «От автора» {name:?}");

        assert_eq!(
            name.border,
            theme::ButtonBorderRole::FaceItself,
            "the name is the frameless row of the table — a name that opens a page is not a \
             button that looks like one"
        );
    }

    // And the predicate answers for the name of «От автора», and for nothing else.
    assert!(settings::is_the_program_name(letters::IDC_AUTHOR_NAME));
    assert!(!settings::is_the_program_name(OLD_ABOUT_NAME));
    assert!(!settings::is_the_program_name(1));

    // Контроль прибора: an ordinary button must **not** come out of that table the same way, or
    // the answer above would hold of everything and mean nothing.
    let ordinary = settings::button_color_roles(1, false, false, false);

    assert_ne!(
        ordinary,
        settings::button_color_roles(letters::IDC_AUTHOR_NAME, false, false, false),
        "«ОК» and the name must not be given the same colours — the table would be answering \
         one thing to everybody"
    );
    assert_eq!(
        ordinary.border,
        theme::ButtonBorderRole::ButtonBorder,
        "every other button of this program has a frame — the finding of the mock-up of Э81"
    );
}
