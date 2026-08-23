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
    ReadOutcome, ReplacementMethod,
};
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

    // Schema 2 — FR-42а moved the default of `[replacement] method` to `auto`.
    assert_eq!(config.schema_version, 2);
    assert_eq!(CURRENT_SCHEMA_VERSION, 2);

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

// Criterion 11. An unknown field does not fail the read, and the fields around it are read.
#[test]
fn unknown_fields_are_ignored_and_the_rest_is_read() {
    let dir = TestDir::new("unknown_fields");
    let path = write_file(
        &dir,
        "schema_version = 2\n\
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
         anything = true\n",
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
        "schema_version = 2\n\
         \n\
         [selection]\n\
         clipboard_timeout_ms = 500\n",
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
        "schema_version = 2\n\
         \n\
         [general]\n\
         enabled = false\n",
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

    assert_eq!(config, Config::default());
    // An empty file has no version marker, so it arrives through migration.
    assert_eq!(outcome, ReadOutcome::Migrated { from: 0 });
}

// Criterion 15. A missing file gives the fully default configuration.
#[test]
fn missing_file_gives_the_default_configuration() {
    let dir = TestDir::new("missing_file");
    let path = dir.config();
    assert!(!path.exists(), "the test must start without a file");

    let (config, outcome) =
        settings::read_from(&path).expect("a missing file must not be an error");

    assert_eq!(config, Config::default());
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
        "schema_version = 2\n\
         \n\
         [replacement]\n\
         method = \"backspace\"\n",
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
        "schema_version = 2\n\
         \n\
         [replacement]\n\
         method = \"auto\"\n",
    );

    let (config, outcome) = settings::read_from(&path).expect("the word auto must be admitted");
    assert_eq!(outcome, ReadOutcome::Current);
    assert_eq!(config.replacement.method, ReplacementMethod::Auto);

    let text = config.to_toml_string().expect("serialises");
    assert!(text.contains("method = \"auto\""));
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

    assert!(text.contains("schema_version = 2"));
    assert!(text.contains("method = \"auto\""));
    for section in [
        "[general]",
        "[hotkey]",
        "[layouts]",
        "[replacement]",
        "[selection]",
        "[buffer]",
        "[exclusions]",
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
            &format!("schema_version = 2\n\n[general]\ntheme = \"{word}\"\n"),
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
    let path = write_file(&dir, "schema_version = 2\n\n[general]\nenabled = false\n");
    let (config, outcome) =
        settings::read_from(&path).expect("a file without the key must be readable");
    assert_eq!(config.general.theme, ThemeSetting::System);
    assert!(!config.general.enabled);
    assert_eq!(outcome, ReadOutcome::Current);

    // An unknown word: no error, the value is the default, and the field beside it still
    // arrives — so it was the word that was ignored, not the file.
    let path = write_file(
        &dir,
        "schema_version = 2\n\n[general]\nenabled = false\ntheme = \"midnight\"\n",
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
        "schema_version = 2\n\
         \n\
         [general]\n\
         enabled = false\n\
         language = \"en\"\n\
         \n\
         [hotkey]\n\
         key = \"ScrollLock\"\n\
         \n\
         [exclusions]\n\
         processes = [\"mstsc.exe\"]\n",
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
// FR-92а — the painting half of the dialog: colour roles and the title bar. Task T-11-4.
// =========================================================================================
//
// What is testable without a window is exactly the two pure functions the handlers call:
// the mapping «identifier → colour role» and the choice of the DWM flag from the palette.
// The `WM_CTLCOLOR*` handlers themselves need a live dialog and are checked by the
// controller's instrument on the real window at acceptance.

use lang_switcher::settings::{
    ButtonBorderRole, ButtonColors, ButtonFaceRole, ButtonTextRole, StaticColorRole,
};
use lang_switcher::theme::{FOG, GRAPHITE, resolve};

// Criterion 10 of T-11-4. The mapping is closed by a table: every explanatory note of the
// dialog by its actual identifier, the `ES_READONLY` trap, and every other static the
// template carries as a sample of the default. The numbers are written out here rather
// than imported — the same rule the template tests below follow: a test that imported the
// identifiers would agree with any renumbering of them.
#[test]
fn the_static_colour_roles_follow_the_table_of_fr_92a() {
    // The six explanatory notes and hints — muted ink over the window background.
    for (control, name) in [
        (1011, "IDC_HOTKEY_NOTE"),
        (1092, "IDC_LANGUAGE_RESTART"),
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
        (1100, "IDC_DELAY_LABEL"),
        (1102, "IDC_CLIP_TIMEOUT_LABEL"),
        (1103, "IDC_CLIP_RESTORE_LABEL"),
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
        settings::title_bar_is_dark(&GRAPHITE),
        "the dark palette must ask for a dark title bar"
    );
    assert!(
        !settings::title_bar_is_dark(&FOG),
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
            settings::title_bar_is_dark(resolve(setting, system_light)),
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
    // ⚠ And the accent **does not answer the cursor** in this wave (task T-12-8, п. 4): the
    // palette holds no second accent to light «ОК» up with, inventing a colour is forbidden,
    // and so the hot row of this half of the table is the row above it, character for
    // character. The rows are here rather than absent on purpose — the day a wave gives the
    // accent a hot face, this is the assertion that has to be edited for it.
    let default_states = [
        (false, false, false, Face::AccentBg, Ink::AccentFg),
        (true, false, false, Face::AccentBg, Ink::AccentFg),
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
/// «Графит» and `Hover=(Col 234 238 242)` for «Туман»), and they are also the very numbers the
/// tray menu has been drawing its own response with since task T-11-21 — the reference this
/// wave copies.
///
/// The second half is the part that makes the measurement mean anything: `hover_bg` is
/// **distinct from every other field of its palette**, so a shot of a hot button cannot be
/// confused with a shot of a pressed one, a quiet one or a selected row.
#[test]
fn the_hot_face_is_the_palette_field_the_tray_menu_already_lights_with() {
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

use lang_switcher::settings::{
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
            settings::glyph_color_roles(kind, checked, disabled),
            expected,
            "{kind:?}, checked = {checked}, disabled = {disabled}"
        );
    }
}

// =========================================================================================
// FR-92а — the check-state store of the eight glyph elements. Task T-11-5b-2.
// =========================================================================================
//
// Criterion 9 — the substance of the defect: a `BS_OWNERDRAW` button keeps no check state
// of its own, so the dialog now keeps its own store, and this section closes that store
// without a live window. The configuration goes in the way `fill_dialog` writes it and
// comes back out the way `read_dialog` reads it; a click's flip inverts exactly the
// element clicked; a radio walk quenches the neighbours of its own range and no one else.
// The identifiers are the template's own — the same eight the style test below reads out
// of the built binary.

use lang_switcher::settings::{GLYPH_CHECK_CONTROLS, GlyphChecks};

/// The eight identifiers by name, mirrored from `app.rc` exactly as the style test's list.
const GLYPH_AUTOSTART: i32 = 1001;
const GLYPH_MODE_PAIR: i32 = 1020;
const GLYPH_MODE_CYCLE: i32 = 1021;
const GLYPH_METHOD_AUTO: i32 = 1029;
const GLYPH_METHOD_BACKSPACE: i32 = 1030;
const GLYPH_METHOD_SELECTION: i32 = 1031;
const GLYPH_SELECTION_ENABLED: i32 = 1040;
const GLYPH_LOG_ENABLED: i32 = 1060;

#[test]
fn the_store_lists_the_eight_template_identifiers_and_starts_all_unchecked() {
    assert_eq!(
        GLYPH_CHECK_CONTROLS,
        [
            GLYPH_AUTOSTART,
            GLYPH_MODE_PAIR,
            GLYPH_MODE_CYCLE,
            GLYPH_METHOD_AUTO,
            GLYPH_METHOD_BACKSPACE,
            GLYPH_METHOD_SELECTION,
            GLYPH_SELECTION_ENABLED,
            GLYPH_LOG_ENABLED,
        ],
        "the storage side must list the same eight controls the template carries"
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
    // Every combination of the five configuration facts the eight elements carry: the
    // three check boxes and the two radio groups. 2 × 2 × 2 × 2 × 3 = 48 round trips.
    for autostart in [false, true] {
        for selection in [false, true] {
            for log in [false, true] {
                for mode in [LayoutMode::Pair, LayoutMode::Cycle] {
                    for method in [
                        ReplacementMethod::Auto,
                        ReplacementMethod::Backspace,
                        ReplacementMethod::Selection,
                    ] {
                        let checks = GlyphChecks::new();

                        // The write half, exactly the calls `fill_dialog` makes: three
                        // set calls and two radio walks.
                        checks.set(GLYPH_AUTOSTART, autostart);
                        checks.check_radio(
                            GLYPH_MODE_PAIR,
                            GLYPH_MODE_CYCLE,
                            match mode {
                                LayoutMode::Pair => GLYPH_MODE_PAIR,
                                LayoutMode::Cycle => GLYPH_MODE_CYCLE,
                            },
                        );
                        checks.check_radio(
                            GLYPH_METHOD_AUTO,
                            GLYPH_METHOD_SELECTION,
                            match method {
                                ReplacementMethod::Auto => GLYPH_METHOD_AUTO,
                                ReplacementMethod::Backspace => GLYPH_METHOD_BACKSPACE,
                                ReplacementMethod::Selection => GLYPH_METHOD_SELECTION,
                            },
                        );
                        checks.set(GLYPH_SELECTION_ENABLED, selection);
                        checks.set(GLYPH_LOG_ENABLED, log);

                        // The read half, exactly the reads `read_dialog` performs.
                        assert_eq!(checks.get(GLYPH_AUTOSTART), autostart);
                        assert_eq!(checks.get(GLYPH_SELECTION_ENABLED), selection);
                        assert_eq!(checks.get(GLYPH_LOG_ENABLED), log);

                        let mode_back = if checks.get(GLYPH_MODE_CYCLE) {
                            LayoutMode::Cycle
                        } else {
                            LayoutMode::Pair
                        };
                        assert_eq!(mode_back, mode, "the mode must survive the round trip");

                        let method_back = if checks.get(GLYPH_METHOD_SELECTION) {
                            ReplacementMethod::Selection
                        } else if checks.get(GLYPH_METHOD_BACKSPACE) {
                            ReplacementMethod::Backspace
                        } else {
                            ReplacementMethod::Auto
                        };
                        assert_eq!(
                            method_back, method,
                            "the replacement method must survive the round trip"
                        );
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
    let checks = GlyphChecks::new();

    checks.set(GLYPH_AUTOSTART, true);
    checks.check_radio(GLYPH_MODE_PAIR, GLYPH_MODE_CYCLE, GLYPH_MODE_PAIR);
    checks.check_radio(GLYPH_METHOD_AUTO, GLYPH_METHOD_SELECTION, GLYPH_METHOD_AUTO);

    // Choosing Backspace quenches Auto — and leaves the mode run and the check boxes
    // alone: the walk is bounded by its own identifier range.
    checks.check_radio(
        GLYPH_METHOD_AUTO,
        GLYPH_METHOD_SELECTION,
        GLYPH_METHOD_BACKSPACE,
    );

    assert!(
        checks.get(GLYPH_METHOD_BACKSPACE),
        "the chosen radio is armed"
    );
    assert!(
        !checks.get(GLYPH_METHOD_AUTO),
        "its former neighbour is quenched"
    );
    assert!(
        !checks.get(GLYPH_METHOD_SELECTION),
        "the third of the run stays quenched"
    );
    assert!(
        checks.get(GLYPH_MODE_PAIR),
        "the other radio run must not move"
    );
    assert!(
        checks.get(GLYPH_AUTOSTART),
        "a check box is not part of any radio run"
    );

    // And the other way round: switching the mode leaves the method run alone.
    checks.check_radio(GLYPH_MODE_PAIR, GLYPH_MODE_CYCLE, GLYPH_MODE_CYCLE);

    assert!(checks.get(GLYPH_MODE_CYCLE), "the new mode is armed");
    assert!(
        !checks.get(GLYPH_MODE_PAIR),
        "the old mode is quenched on the same walk"
    );
    assert!(
        checks.get(GLYPH_METHOD_BACKSPACE),
        "the method run must not move when the mode run walks"
    );
}

#[test]
fn an_identifier_outside_the_eight_reads_unchecked_and_stores_nothing() {
    let checks = GlyphChecks::new();

    // 1032 is the delay field — a control of the dialog, but not a glyph element.
    checks.set(1032, true);

    assert!(
        !checks.get(1032),
        "an identifier outside the eight must read «снят» (NFR-13)"
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

use lang_switcher::settings::{ComboFillRole, ComboItemColors, ComboTextRole};

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
            settings::combo_item_color_roles(closed_part, highlighted),
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

use lang_switcher::settings::{
    CHECK_FRAME_ORDER, CHECKED_IMAGE, CheckFrameColors, CycleRowPaint, UNCHECKED_IMAGE,
};

#[test]
fn the_check_frame_colours_follow_the_2x2_table_of_fr_92a() {
    // Both palettes by name, so a swapped pair could not pass: the loop below asserts
    // against the fields of the very palette it hands in.
    for palette in [&GRAPHITE, &FOG] {
        // Снята: the quiet ground of the list under the single-pixel box frame — the same
        // cell the unchecked owner-drawn check box of the dialog paints.
        assert_eq!(
            settings::check_frame_colors(false, palette),
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
            settings::check_frame_colors(true, palette),
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
const FR_92_SECTIONS: [&str; 8] = [
    "Общие",
    "Горячая клавиша",
    "Раскладки",
    "Замена",
    "Выделение",
    "Исключения",
    "Диагностика",
    "Состояние",
];

/// Every visible string of the template, in template order and with the empty ones left out.
///
/// The captions of the sections above are in here too, in their place: this is the whole of
/// what a person reads on that window, and it is what a wrong code page would destroy.
const TEMPLATE_TEXT: [&str; 37] = [
    "Общие",
    "Запускать при входе в систему",
    "Язык интерфейса:",
    // The appearance row of FR-92а, task T-11-3, declared right after the language combo;
    // its own combo carries no text in the template — the items are added by the dialog.
    "Оформление:",
    "вступит в силу после перезапуска",
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
    "Замена",
    // The third method of FR-42а stands first in its group: it is the default of section 7.
    "Автоматически (рекомендуется)",
    "Backspace",
    "Выделение (совместимость)",
    "Задержка между событиями, мс:",
    "Выделение",
    "Конвертировать выделенный текст",
    "Таймаут буфера обмена, мс:",
    "Задержка восстановления, мс:",
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
const TEMPLATE_CONTROLS: [(u32, &str); 48] = [
    (1001, "Общие: автозапуск"),
    (1002, "Общие: язык интерфейса"),
    (1003, "Общие: оформление — FR-92а"),
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
    (1029, "Замена: метод «Автоматически» — FR-42а"),
    (1030, "Замена: метод Backspace"),
    (1031, "Замена: метод выделения"),
    (1032, "Замена: задержка между событиями"),
    (1040, "Выделение: конвертировать выделенное"),
    (1041, "Выделение: таймаут буфера обмена"),
    (1042, "Выделение: задержка восстановления"),
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
    (1092, "Общие: «вступит в силу после перезапуска»"),
    (1093, "Горячая клавиша: заголовок группы"),
    (1094, "Горячая клавиша: подпись «Клавиша»"),
    (1095, "Раскладки: заголовок группы"),
    (1096, "Раскладки: подпись «Источник»"),
    (1097, "Раскладки: подпись «Цель»"),
    (1098, "Раскладки: пояснение к списку цикла"),
    (1099, "Замена: заголовок группы"),
    (1100, "Замена: подпись задержки"),
    (1101, "Выделение: заголовок группы"),
    (1102, "Выделение: подпись таймаута буфера обмена"),
    (1103, "Выделение: подпись задержки восстановления"),
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
fn the_eight_check_boxes_and_radio_buttons_are_owner_drawn_and_notifying() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    // The three check boxes and five radio buttons of FR-92, by their actual identifiers;
    // the third column says which of them open a WS_GROUP run. The list-view ticks of the
    // cycle list are not here — they belong to task T-11-7.
    const OWNER_DRAWN_GLYPHS: [(u32, &str, bool); 8] = [
        (1001, "Запускать при входе в систему", false),
        (1020, "Пара", true),
        (1021, "Несколько раскладок", false),
        (1029, "Автоматически (рекомендуется)", true),
        (1030, "Backspace", false),
        (1031, "Выделение (совместимость)", false),
        (1040, "Конвертировать выделенный текст", false),
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
// nine buttons and the eight glyphs above. The same ⚠ applies: `BS_OWNERDRAW` (0x0B) is a
// button *type* in the low nibble, so the check is equality of the nibble and not a bit
// test — the former type here was `BS_GROUPBOX` (0x07), which a bit test would also let
// through (0x07 & 0x0B is 0x03, not zero).
#[test]
fn the_eight_group_boxes_are_owner_drawn_and_take_no_tab_stop() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    // The eight groups of the dialog — the seven sections of the FR-92 table plus
    // «Состояние» — by their actual identifiers.
    const OWNER_DRAWN_GROUPS: [(u32, &str); 8] = [
        (1090, "Общие"),
        (1093, "Горячая клавиша"),
        (1095, "Раскладки"),
        (1099, "Замена"),
        (1101, "Выделение"),
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
const OWNER_DRAWN_LABELS: [(u32, &str); 18] = [
    (1091, "Общие: подпись «Язык интерфейса»"),
    (1109, "Общие: подпись «Оформление»"),
    (1092, "Общие: «вступит в силу после перезапуска»"),
    (1094, "Горячая клавиша: подпись «Клавиша»"),
    (1011, "Горячая клавиша: предупреждение"),
    (1096, "Раскладки: подпись «Источник»"),
    (1097, "Раскладки: подпись «Цель»"),
    (1098, "Раскладки: пояснение к списку цикла"),
    (1027, "Раскладки: замечание о ненайденной раскладке"),
    (1100, "Замена: подпись задержки"),
    (1102, "Выделение: подпись таймаута буфера обмена"),
    (1103, "Выделение: подпись задержки восстановления"),
    (1105, "Исключения: пояснение об имени процесса"),
    (1107, "Диагностика: подпись «Папка журнала»"),
    (1062, "Диагностика: путь папки журнала"),
    (1070, "Состояние: перехват клавиатуры"),
    (1071, "Состояние: раскладки сеанса"),
    (1072, "Состояние: автозапуск в реестре"),
];

/// Every `LTEXT` of the about template. The `ICON` is not one of them — see above.
const OWNER_DRAWN_ABOUT_LABELS: [(u32, &str); 4] = [
    (1121, "О программе: имя"),
    (1122, "О программе: версия"),
    (1123, "О программе: первая строка"),
    (1124, "О программе: вторая строка"),
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

    let format = settings::LABEL_TEXT_FORMAT.0;

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

    // The two labels the wrap exists for, and the height they need for a second line. A
    // one-line label of this dialog is 9 dialog units high; these two are 18 and 16.
    const TWO_LINE_LABELS: [(u32, &str, i32); 2] = [
        (1092, "вступит в силу после перезапуска", 18),
        (1062, "путь к папке журнала", 16),
    ];

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

    // Width untouched — two columns 200 units wide at x = 7 and x = 213 — and the height is
    // the 369 of before plus the 16 the appearance row cost.
    assert_eq!(
        template.size,
        (420, 385),
        "the settings dialog has to be 420 x 385 dialog units — task T-11-19"
    );

    // The eight rectangles that moved, in full: a `y` alone would say nothing about a width
    // or a height that drifted with it. «Диагностика» is the one that grew rather than
    // moved — the 16 units go on at its bottom edge and its children stay where they were.
    const MOVED: [(u32, &str, i32, i32, i32, i32); 8] = [
        (1106, "Диагностика: панель", 213, 236, 200, 74),
        (1108, "Состояние: панель", 7, 314, 406, 44),
        (1070, "Состояние: перехватчик", 14, 326, 392, 9),
        (1071, "Состояние: раскладки", 14, 337, 392, 9),
        (1072, "Состояние: автозапуск", 14, 348, 392, 9),
        (1, "ОК", 253, 364, 50, 14),
        (2, "Отмена", 307, 364, 50, 14),
        (1080, "Применить", 361, 364, 52, 14),
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

    // The children of «Диагностика» did **not** move by the 16 units: the panel grew
    // downwards. Their own rectangles are the witness — the growth is at the bottom edge or it
    // is not this task.
    //
    // ⚠ «путь к папке журнала» is 289 and not the 291 this test held until task T-12-3. It
    // moved, and *upwards by two units*, which is the opposite direction from the sixteen this
    // test is about: `IDC_LOG_DIR` stood at y = 275 where the generator of the mock-ups writes
    // 273 (defect Д-4 of the Э12 comparison), and T-12-3 gave it the generator's number back.
    // The statement of T-11-19 is untouched by that — the number is asserted, not relaxed, and
    // the three rows beside it are the ones that prove the 16 units went on below.
    for (id, what, bottom) in [
        (1060u32, "Вести журнал", 258i32),
        (1061, "Открыть папку журнала", 260),
        (1107, "Папка журнала:", 273),
        (1062, "путь к папке журнала", 289),
    ] {
        let (_, _, _, measured) = template.rect_of(id);

        assert_eq!(
            measured, bottom,
            "«{what}» ({id}) moved; the 16 units of task T-11-19 go on below the children of \
             «Диагностика», not through them"
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

    assert_eq!(
        air_left, 53,
        "the air under «Раскладки» has to be the 53 units of the mock-ups; it is the whole \
         point of task T-11-19"
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
/// expression, which is what `app.rc` now does for the eight panels.
const WS_VISIBLE: u32 = 0x1000_0000;

/// `WS_BORDER` — the sunken system rectangle. rc.exe adds this one to `EDITTEXT` and
/// `LISTBOX` on top of `WS_VISIBLE`, and the same `NOT` takes it off.
const WS_BORDER: u32 = 0x0080_0000;

/// `WS_CLIPSIBLINGS` — the style the hypothesis of task T-11-13 named. Not a single control
/// of this template has ever carried it; see the test below.
const WS_CLIPSIBLINGS: u32 = 0x0400_0000;

/// The eight group panels, by identifier and caption — the same eight
/// `settings::GROUP_BOXES` lists, written out as literals by the rule the other template
/// tests follow.
const PANELS: [(u32, &str); 8] = [
    (1090, "Общие"),
    (1093, "Горячая клавиша"),
    (1095, "Раскладки"),
    (1099, "Замена"),
    (1101, "Выделение"),
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

    // The seven of `settings::FRAMED_FIELDS`, by their actual identifiers; the third column
    // is the bit that must still be there next to the border that must not.
    const FIELDS: [(u32, &str, u32, &str); 7] = [
        (1010, "поле горячей клавиши", 0x0001_0000, "WS_TABSTOP"),
        (1032, "задержка между событиями", 0x2000, "ES_NUMBER"),
        (1041, "таймаут буфера обмена", 0x2000, "ES_NUMBER"),
        (1042, "задержка восстановления", 0x2000, "ES_NUMBER"),
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
    // The eight captions of the dialog, in both locales — the strings FR-94 puts on the
    // eight controls, and what the background drawing must make of them.
    for (source, expected) in [
        ("Общие", "ОБЩИЕ"),
        ("Горячая клавиша", "ГОРЯЧАЯ КЛАВИША"),
        ("Раскладки", "РАСКЛАДКИ"),
        ("Замена", "ЗАМЕНА"),
        ("Выделение", "ВЫДЕЛЕНИЕ"),
        ("Исключения", "ИСКЛЮЧЕНИЯ"),
        ("Диагностика", "ДИАГНОСТИКА"),
        ("Состояние", "СОСТОЯНИЕ"),
        ("General", "GENERAL"),
        ("Hotkey", "HOTKEY"),
        ("Replacement", "REPLACEMENT"),
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
    assert_eq!(settings::caption_advance(9, 12), 9);
    assert_eq!(settings::caption_advance(1, 12), 1);

    // A blank — measured as zero, or as a negative by a refused measurement — takes the
    // explicit width instead, and that width is never zero for a font of any real size.
    for height in [10, 12, 16, 24] {
        let blank = settings::caption_advance(0, height);

        assert!(
            blank > 0,
            "a blank in a {height} px face must take room, not {blank}"
        );
        assert!(
            blank < height,
            "a blank in a {height} px face must be narrower than the face is tall, not \
             {blank}"
        );
        assert_eq!(settings::caption_advance(-1, height), blank);
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
        settings::CORNER_RADIUS,
        6,
        "the `Radius = 6` of the style table of ui.ps1"
    );

    let source = settings_module_source();

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
    assert_eq!(settings::SCREEN_DPI, 96, "100 % is 96 DPI");
    assert_eq!(
        settings::MOCKUP_SCALE_TENTHS,
        14,
        "the `$DPI = 1.4` of ui.ps1"
    );
    assert_eq!(
        settings::MOCKUP_DPI_TENTHS,
        1344,
        "96 × 1,4 = 134,4 DPI, carried in tenths"
    );
    assert_eq!(
        settings::MOCKUP_DPI_TENTHS,
        settings::SCREEN_DPI * settings::MOCKUP_SCALE_TENTHS,
        "the mock-up DPI must stay 96 × 1,4 and not a number of its own"
    );

    // 100 % — the picture divided by 1,4, rounded to nearest.
    assert_eq!(settings::scaled(6, 96), 4);
    assert_eq!(settings::scaled(4, 96), 3);

    // 125 %, 150 %, 200 % — rounded to nearest, so a one-pixel frame never rounds away.
    assert_eq!(settings::scaled(6, 120), 5);
    assert_eq!(settings::scaled(4, 120), 4);
    assert_eq!(settings::scaled(6, 144), 6);
    assert_eq!(settings::scaled(4, 192), 6);
    assert_eq!(settings::scaled(1, 120), 1);
    assert_eq!(settings::scaled(1, 144), 1);

    // The letter spacing is carried in tenths of a pixel, and scales as a tenth does.
    assert_eq!(settings::scaled(11, 96), 8);
    assert_eq!(settings::scaled(11, 192), 16);

    // A device that will not say what its DPI is gets the 100 % look, and — since T-11-15 —
    // that is the *scaled* 100 % look and not the mock-up number handed over unchanged.
    assert_eq!(settings::scaled(6, 0), 4);
    assert_eq!(settings::scaled(6, -1), 4);
    assert_eq!(
        settings::scaled_tenths(15, 0),
        settings::scaled_tenths(15, 96)
    );

    // The scale is one division and nothing else, and at 96 DPI it is an exact fraction:
    // 96 / 134,4 = **5 / 7**. Every mock-up length must therefore come out as the nearest
    // whole of five sevenths of itself — written here as arithmetic that does not go anywhere
    // near the module's own formula.
    for pixels in [1, 2, 3, 4, 5, 6, 8, 11, 12, 16, 33] {
        let five_sevenths = (pixels * 10 + 7) / 14;

        assert_eq!(
            settings::scaled(pixels, 96),
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
    let glyph_side = settings::scaled(settings::GLYPH_SIZE, 96);
    let dot_inset = settings::scaled_tenths_offset(settings::GLYPH_DOT_INSET_TENTHS, 96);
    let glyph_mark = settings::check_mark_points((0, 0), settings::GLYPH_CHECK_MARK, 96);

    // The radius and the frame are one number for five figures each, so the three rows that
    // ask for them in three different tables are answered by the same two expressions.
    let radius = || vec![(px(settings::scaled(settings::CORNER_RADIUS, 96)), at_96(60))];
    // ⚠ The frame is the one row whose «при 96 DPI» column the reference does not divide: a
    // pen has no fractional width, and `.max(1)` is what keeps the single pixel at every scale.
    let border = || {
        vec![(
            px(settings::scaled(settings::BORDER_THICKNESS, 96).max(1)),
            px(1),
        )]
    };

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
        token(
            "1. Панель · Отступ заголовка сверху",
            "5 px макета",
            vec![(
                px(settings::scaled(settings::PANEL_CAPTION_INSET_Y, 96)),
                at_96(50),
            )],
        ),
        token(
            "1. Панель · Разрядка заголовка",
            "1,1 px макета",
            // The tracking is carried in tenths of a *screen* pixel: ten hundredths each.
            vec![(
                settings::scaled(settings::PANEL_CAPTION_TRACKING_TENTHS, 96) * 10,
                at_96(11),
            )],
        ),
        units(
            "1. Панель · Кегль заголовка",
            "7,6 pt против 9 pt = 0,844 основного, в промилле",
            vec![(
                settings::PANEL_CAPTION_POINTS_TENTHS * 1000 / settings::DIALOG_FONT_POINTS_TENTHS,
                76 * 1000 / 90,
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
                px(settings::scaled(settings::GLYPH_CORNER_RADIUS, 96)),
                at_96(30),
            )],
        ),
        token(
            "2. Глиф · Толщина пера галочки",
            "2,1 px макета",
            vec![(
                px(settings::scaled_tenths(
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
                px(settings::scaled(settings::COMBO_CHEVRON_INSET_X, 96)),
                at_96(140),
            )],
        ),
        token(
            "4. Комбобокс · Толщина пера шеврона",
            "1,5 px макета",
            vec![(
                px(settings::scaled_tenths(
                    settings::COMBO_CHEVRON_PEN_TENTHS,
                    96,
                )),
                at_96(15),
            )],
        ),
        token(
            "4. Комбобокс · Плечо шеврона",
            "±4 по x, ∓2 по y px макета",
            vec![
                (
                    px(settings::scaled(settings::COMBO_CHEVRON_ARM_X, 96)),
                    at_96(40),
                ),
                (
                    px(settings::scaled(settings::COMBO_CHEVRON_ARM_Y, 96)),
                    at_96(20),
                ),
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
            vec![(
                px(settings::scaled(settings::LIST_FIRST_ROW_TOP, 96)),
                at_96(30),
            )],
        ),
        token(
            "6. Список исключений · Втяжка прямоугольника выделения и его радиус",
            "2 / 3 px макета",
            vec![
                (
                    px(settings::scaled(settings::LIST_SELECTION_INSET, 96)),
                    at_96(20),
                ),
                (
                    px(settings::scaled(settings::LIST_SELECTION_RADIUS, 96)),
                    at_96(30),
                ),
            ],
        ),
        token(
            "6. Список исключений · Втяжка текста от левого края списка",
            "7 px макета",
            vec![(
                px(settings::scaled(settings::LIST_TEXT_INSET, 96)),
                at_96(70),
            )],
        ),
        token(
            "6. Список исключений · Текст ниже верха строки",
            "2 px макета",
            vec![(px(settings::scaled(settings::LIST_TEXT_TOP, 96)), at_96(20))],
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
                px(settings::scaled(settings::LIST_CHECK_CORNER_RADIUS, 96)),
                at_96(20),
            )],
        ),
        token(
            "7. Список раскладок · Толщина пера галочки",
            "1,8 px макета",
            vec![(
                px(settings::scaled_tenths(
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
                    px(settings::scaled(settings::LIST_SELECTION_INSET, 96)),
                    at_96(20),
                ),
                (
                    px(settings::scaled(settings::LIST_SELECTION_RADIUS, 96)),
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
    let source = settings_module_source();

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
    let source = settings_module_source();

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
        body.contains("check_mark_points((glyph.left, glyph.top), mark, dpi)"),
        "the strokes must come from the pure `check_mark_points`, not from literals"
    );

    // The two marks the dialog draws, each with the literals its own arm of the generator has.
    // The dialog glyph is the row of the reference; the tick of the layout list is the same
    // figure one size down, and the reference gives its pen but not its points — so the points
    // are held here, against the `(PtF ($bx+3.4) ($by+6.6)) …` of the `'lview'` arm.
    assert_eq!(
        settings::GLYPH_CHECK_MARK,
        settings::CheckMark {
            points_tenths: [(45, 86), (73, 118), (125, 52)],
            pen_tenths: 21,
        }
    );
    assert_eq!(
        settings::LIST_CHECK_MARK,
        settings::CheckMark {
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
            let square = settings::scaled(side, dpi);

            for (x, y) in settings::check_mark_points((0, 0), mark, dpi) {
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
    use lang_switcher::settings::{ComboBorderRole, ComboChevronRole, ComboClosedColors};

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
            settings::combo_closed_color_roles(hot, disabled),
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
    let points = settings::combo_chevron_points(&area, 96);

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
    let wide = settings::combo_chevron_points(&rect(0, 0, 280, 66), 192);

    println!("chevron at 192 dpi: {wide:?}");
    assert_eq!(wide[2].0 - wide[0].0, 12, "the span doubles at 200 %");
    assert_eq!(wide[1].1 - wide[0].1, 6, "the drop doubles with it");

    // The stroke is the one length of the mock-ups written in tenths of a pixel: 1,5 px of the
    // picture, which is 1,07 px of a 100 % screen — one pixel, and two at 200 % (T-11-15).
    assert_eq!(
        settings::scaled_tenths(15, 96),
        1,
        "1,5 px of a 140 % picture is one pixel at 100 %"
    );
    assert_eq!(settings::scaled_tenths(15, 192), 2, "2 px at 200 %");
    // Never a zero-width pen, which GDI would read as a hairline drawn by other rules.
    assert_eq!(settings::scaled_tenths(1, 96), 1);
    assert_eq!(
        settings::scaled_tenths(15, 0),
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
    assert_eq!(
        source
            .matches("RemoveWindowSubclass(combo, Some(combo_box_proc), COMBO_SUBCLASS_ID)")
            .count(),
        2,
        "the removal of the pair and the WM_NCDESTROY safety net, and nothing else"
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
        paint_arm.contains("paint_combo_closed_part(combo)"),
        "the WM_PAINT arm must draw the closed part itself"
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
    assert_eq!(settings::scaled(settings::GLYPH_CORNER_RADIUS, 96), 2);
    assert_eq!(settings::scaled(settings::GLYPH_CORNER_RADIUS, 192), 4);

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
    let source = settings_module_source();

    // Пять тел `WM_DRAWITEM` и шестое — закрытая часть комбобокса, которая рисует себя по
    // `WM_PAINT` подкласса и потому тем более отвечает за весь свой прямоугольник.
    for signature in [
        "unsafe fn paint_push_button(",
        "pub unsafe fn paint_label(",
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
        settings::list_item_color_roles(false),
        ComboItemColors {
            fill: Fill::FieldBg,
            text: Ink::Text
        },
        "an ordinary row is the quiet ground of the list"
    );
    assert_eq!(
        settings::list_item_color_roles(true),
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
    let air_in_mockup_pixels = air_at_96 * settings::MOCKUP_SCALE_TENTHS / 10; // 2,10 px
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
    let field_at_96 = 15 + settings::scaled(settings::COMBO_CLOSED_ITEM_EXTRA, 96);

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
        15 + settings::scaled(settings::COMBO_LIST_ITEM_EXTRA, 96) + 6,
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
        15 + settings::scaled(settings::COMBO_LIST_ITEM_EXTRA, 96),
        24,
        "a list item must still be the 24 px `CB_GETITEMHEIGHT(0)` answered before T-12-2"
    );

    assert!(
        settings::scaled(settings::COMBO_CLOSED_ITEM_EXTRA, 96)
            < settings::scaled(settings::COMBO_LIST_ITEM_EXTRA, 96),
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
    // description lines and the one button. One row per element, so a lost control is a
    // failing test and not a smaller window.
    const ABOUT_CONTROLS: [(u32, &str); 6] = [
        (1120, "иконка программы"),
        (1121, "имя программы"),
        (1122, "строка версии"),
        (1123, "первая строка описания"),
        (1124, "вторая строка описания"),
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
        "the about dialog carries exactly its six controls and nothing else"
    );

    // The visible literals, in template order. The version row is empty on purpose — it is
    // composed at run time from the version resource — and the icon control carries an
    // ordinal, not a text, so neither appears here.
    assert_eq!(
        template.text,
        [
            "Lang Switcher",
            "Исправляет текст, набранный в неверной раскладке.",
            "Перекодировка — по нажатию одной клавиши.",
            "ОК",
        ],
        "the visible strings of the about dialog came out of rc.exe wrong"
    );
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
    let cases = [
        (1120, settings::StaticColorRole::Label, "иконка"),
        (1121, settings::StaticColorRole::Label, "имя"),
        (1122, settings::StaticColorRole::Muted, "строка версии"),
        (1123, settings::StaticColorRole::Muted, "описание, строка 1"),
        (1124, settings::StaticColorRole::Muted, "описание, строка 2"),
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

#[test]
fn a_millisecond_field_reads_as_a_number_and_never_as_a_guess() {
    assert_eq!(settings::parse_ms("300", 7), 300);
    assert_eq!(settings::parse_ms("  42 ", 7), 42);
    // An empty field is zero, which section 7 allows for the delay of FR-44.
    assert_eq!(settings::parse_ms("", 7), 0);
    // Nothing the dialog can produce, because the field takes digits only and is limited in
    // length — and if it ever did, the value that was there before is kept.
    assert_eq!(settings::parse_ms("99999999999999", 7), 7);
    assert_eq!(settings::parse_ms("nonsense", 7), 7);
}

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

#[test]
fn what_the_dialog_applies_reaches_the_modules_that_act_on_it() {
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
        let bytes = self.resource_of_language(RT_STRING, id / 16 + 1, language.langid());

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
        let _ex_style = read_u32(bytes, &mut at);
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
const FR_94_STRINGS: [(u16, &str, &str); 72] = [
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
    (
        settings::IDS_LANGUAGE_RESTART,
        "вступит в силу после перезапуска",
        "takes effect after a restart",
    ),
    (settings::IDS_GROUP_HOTKEY, "Горячая клавиша", "Hotkey"),
    (settings::IDS_HOTKEY_LABEL, "Клавиша:", "Key:"),
    (settings::IDS_HOTKEY_SET, "Задать", "Set"),
    (settings::IDS_HOTKEY_STOP, "Отменить", "Cancel"),
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
    (settings::IDS_GROUP_REPLACEMENT, "Замена", "Replacement"),
    (settings::IDS_METHOD_BACKSPACE, "Backspace", "Backspace"),
    (
        settings::IDS_METHOD_SELECTION,
        "Выделение (совместимость)",
        "Selection (compatibility)",
    ),
    (
        settings::IDS_DELAY_LABEL,
        "Задержка между событиями, мс:",
        "Delay between events, ms:",
    ),
    (settings::IDS_GROUP_SELECTION, "Выделение", "Selection"),
    (
        settings::IDS_SELECTION_ENABLED,
        "Конвертировать выделенный текст",
        "Convert the selected text",
    ),
    (
        settings::IDS_CLIPBOARD_TIMEOUT,
        "Таймаут буфера обмена, мс:",
        "Clipboard timeout, ms:",
    ),
    (
        settings::IDS_CLIPBOARD_RESTORE,
        "Задержка восстановления, мс:",
        "Restore delay, ms:",
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
        settings::IDS_CAPTURE_PROMPT,
        "Нажмите клавишу. Esc — отмена.",
        "Press a key. Esc cancels.",
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
    (
        settings::IDS_METHOD_AUTO,
        "Автоматически (рекомендуется)",
        "Automatic (recommended)",
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

    // A text key is taken and *warned about* — FR-92 asks for the warning, not for a refusal.
    assert_eq!(
        settings::capture(0x41, BARE),
        settings::Capture::Taken("A".to_owned())
    );
    assert_eq!(
        settings::hotkey_note("A"),
        Some(settings::IDS_NOTE_TEXT_KEY)
    );
}

#[test]
fn the_capture_refuses_every_press_that_cannot_be_a_hotkey() {
    // **Criterion 16.** Five refusals, each with a sentence of its own in both locales.
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
        (0x20, BARE, settings::Refusal::Nameless, "Space"),
        (0xBA, BARE, settings::Refusal::Nameless, "OEM 1"),
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
    settings_module_source()
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//") && line.contains(needle))
        .map(str::to_owned)
        .collect()
}

/// **Criterion 13 of T-11-17** — every face this program sets its own text in is asked for grey
/// antialiasing, and the ask is a pure function of a `LOGFONTW`.
///
/// The expected value is written out here as the `ANTIALIASED_QUALITY` of `wingdi.h` — the
/// literal 4 — and not only read back from the crate: a builder that quietly left the manager's
/// ClearType in place would agree with itself and disagree with the decision of 2026-08-22.
#[test]
fn our_own_faces_are_asked_for_grey_antialiasing_and_nothing_else_moves() {
    // A face with something recognisable in every field the builders must not touch.
    let mut base = LOGFONTW {
        lfHeight: -18,
        lfWidth: 7,
        lfWeight: 400,
        lfItalic: 1,
        lfUnderline: 1,
        lfStrikeOut: 1,
        // The manager's own face renders with ClearType — the starting point this task moves
        // our own text away from.
        lfQuality: CLEARTYPE_QUALITY,
        ..Default::default()
    };

    // «Segoe UI», the face the template asks for, as UTF-16 into the fixed array.
    for (slot, unit) in base.lfFaceName.iter_mut().zip("Segoe UI".encode_utf16()) {
        *slot = unit;
    }

    let text = settings::antialiased_logfont(base);

    assert_eq!(
        text.lfQuality.0, 4,
        "the quality must be ANTIALIASED_QUALITY — the 4 of wingdi.h"
    );
    assert_eq!(
        text.lfQuality, ANTIALIASED_QUALITY,
        "and it must be the constant the crate names 4 by"
    );
    assert_ne!(
        text.lfQuality, CLEARTYPE_QUALITY,
        "ClearType is what this task takes off our own text"
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

    // The caption face: the same antialiasing, and the two changes п. 2.1 of T-11-13 asks for.
    let caption = settings::caption_logfont(base);

    assert_eq!(
        caption.lfQuality.0, 4,
        "a panel caption is our own text as much as a button caption is"
    );
    assert_eq!(
        caption.lfHeight,
        (base.lfHeight * 76) / 90,
        "7,6 pt against 9 pt — the two sizes the mock-ups were drawn with"
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
    assert_eq!(settings::SUPERSAMPLE, 4);

    // And a ceiling on what may be enlarged whole, so that a smoothed detail cannot quietly
    // become a smoothed panel: the largest surface this file can ask for is 256 × 256.
    assert_eq!(settings::SUPERSAMPLE_MAX_SIDE, 64);
    assert_eq!(
        settings::SUPERSAMPLE_MAX_SIDE * settings::SUPERSAMPLE,
        256,
        "the widest enlarged surface the module can ask for"
    );

    // The block one destination pixel is averaged from is the factor squared, and it is a
    // constant rather than a number written at the loop.
    assert_eq!(
        settings::SUPERSAMPLE_BLOCK,
        (settings::SUPERSAMPLE * settings::SUPERSAMPLE) as usize
    );
    assert_eq!(settings::SUPERSAMPLE_BLOCK, 16);

    let source = settings_module_source();

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

    let reduction = function_body(
        &source,
        "fn render(&self, dc: HDC, tile: &RECT, thickness: i32, figure: impl FnOnce(Canvas)) \
         -> bool {",
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
        settings::average_of_block(&flat),
        ground,
        "a block of one colour must reduce to that colour"
    );
    assert_eq!(settings::average_of_block(&[ink; 16]), ink);

    // Every coverage from none to all, against the average computed here from the definition.
    for covered in 0..=16u32 {
        let mut block = [ground; 16];

        for sample in block.iter_mut().take(covered as usize) {
            *sample = ink;
        }

        let (red, green, blue) = channels(settings::average_of_block(&block));

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

        let (red, green, blue) = channels(settings::average_of_block(&block));
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
        channels(settings::average_of_block(&mixed)),
        (70, 80, 90),
        "each channel is averaged with itself and with no other"
    );

    // An empty block has no mean and answers zero rather than dividing by nothing. The product
    // never hands one over — the block is always [`settings::SUPERSAMPLE_BLOCK`] long — and the
    // function is public, so the degenerate case is answered rather than left to chance.
    assert_eq!(settings::average_of_block(&[]), 0);
}

/// **Criterion 11 of T-11-17** — every GDI object this task added is owned by a value with a
/// `Drop`, and no handle is used before it is examined.
#[test]
fn every_surface_picture_and_face_of_this_task_is_owned_and_freed_in_drop() {
    let source = settings_module_source();

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
    assert_eq!(
        product_lines_with("unsafe { SelectObject(self.dc, self.previous) };").len(),
        2,
        "both owners of a bitmap must deselect before they delete"
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

    // The two halves of «still the right picture»: the client size, and the palette by
    // identity — `theme::resolve` answers `&'static`, and `refresh_palette` is the one place
    // the answer can change.
    assert!(
        source.contains(
            "self.width == width && self.height == height && std::ptr::eq(self.palette, palette)"
        ),
        "the picture must be compared against both the size and the palette it was painted in"
    );

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
    let tiles = settings::corner_tiles(&area, 5);

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
                * settings::SUPERSAMPLE
                * ((tile.bottom - tile.top) * settings::SUPERSAMPLE)
        })
        .sum();

    let enlarged_whole = (area.right - area.left)
        * settings::SUPERSAMPLE
        * ((area.bottom - area.top) * settings::SUPERSAMPLE);

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
    let tile = settings::stroke_bounds(&points, 2);

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
    let thick = settings::stroke_bounds(&points, 8);
    assert_eq!(
        (thick.left, thick.top, thick.right, thick.bottom),
        (-2, -1, 14, 13)
    );

    // No points at all is no tile — and the caller draws nothing either way.
    let nothing = settings::stroke_bounds(&[], 2);
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

    settings::paint_rounded(sheet.dc, &area, 6, ink, fill, 96);

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
    assert_eq!(settings::stroke_shift(1), settings::SUPERSAMPLE / 2);
    assert_eq!(settings::stroke_shift(3), settings::SUPERSAMPLE / 2);
    assert_eq!(settings::stroke_shift(5), settings::SUPERSAMPLE / 2);

    // An even pen has no centre pixel to lose: enlarged, it lands on the block boundary of its
    // own accord.
    assert_eq!(settings::stroke_shift(0), 0);
    assert_eq!(settings::stroke_shift(2), 0);
    assert_eq!(settings::stroke_shift(4), 0);

    // Half of one pixel of the window, in the pixels of the enlarged surface.
    assert_eq!(settings::SUPERSAMPLE / 2, 2);
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

        settings::draw_check_mark(sheet.dc, &glyph, ink, settings::GLYPH_CHECK_MARK, 96);

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

        settings::paint_rounded(sheet.dc, &area, 6, ink, fill, 96);

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
/// corner tile is `radius + thickness` a side, so a radius of [`settings::SUPERSAMPLE_MAX_SIDE`]
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
    let radius = settings::SUPERSAMPLE_MAX_SIDE;
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
        side > settings::SUPERSAMPLE_MAX_SIDE,
        "the lever of this test is a corner tile past the ceiling — {side} against {}",
        settings::SUPERSAMPLE_MAX_SIDE
    );
    assert!(
        side * 2 <= (area.right - area.left).min(area.bottom - area.top),
        "and the figure must be big enough to hold four tiles, or it is drawn aliased whole"
    );

    // SAFETY: the brush is made here, used only by the call below and freed here.
    let fill = unsafe { CreateSolidBrush(fill_colour) };

    settings::paint_rounded(sheet.dc, &area, radius, ink, fill, 96);

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(fill.into()) };

    // The centre of each corner's arc, in the order `corner_tiles` answers them: top-left,
    // top-right, bottom-left, bottom-right. The figure spans `left … right - 1`, which is why
    // the two far centres are counted off `right - 1` and `bottom - 1`.
    let tiles = settings::corner_tiles(&area, side);
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
    let source = settings_module_source();

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
        settings::list_frame_air(96),
        (2, 1),
        "the air above a list and the thickness around it, at 96 DPI"
    );

    // И воздух никогда не тоньше рамки: `max(border)` в теле — не украшение.
    for dpi in [96, 120, 144, 192] {
        let (top, border) = settings::list_frame_air(dpi);

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
    let box_at_96 = settings::list_frame_box(&client, 96);

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
        let (top, border) = settings::list_frame_air(dpi);
        let frame = settings::list_frame_box(&client, dpi);

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

    let area = settings::list_frame_box(&client, 96);

    settings::paint_rounded_corners(
        sheet.dc,
        &area,
        &client,
        6,
        settings::CornerColors {
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
    let area = settings::list_frame_box(&window, 96);

    settings::paint_rounded_corners(
        sheet.dc,
        &area,
        &client,
        6,
        settings::CornerColors {
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
// `settings::paint_label` needs a DC, a rectangle, a brush and an ink — no window, no message
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

/// Paints one caption into `area` of `sheet` through the product's own [`settings::paint_label`],
/// with a brush this function makes and frees.
fn paint_label_on(sheet: &Sheet, area: RECT, caption: &str) -> isize {
    let mut text: Vec<u16> = caption.encode_utf16().collect();

    // SAFETY: the brush is made here, used only by the call below and freed here.
    let ground = unsafe { CreateSolidBrush(LABEL_GROUND) };

    // SAFETY: `sheet.dc` holds this sheet's bitmap, `text` and `area` are live locals of this
    // frame, and `ground` is live for the whole call. `None` leaves the DC's own font in place.
    let answer =
        unsafe { settings::paint_label(sheet.dc, area, &mut text, ground, LABEL_INK, None) };

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
/// The dialog's own face and [`settings::antialiased_logfont`] of it are created side by side
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
        let ours = settings::antialiased_logfont(base);

        // The premise first: one field moved and not a byte else. A base that differed
        // somewhere else would make the metrics agree for a reason this task cannot claim.
        assert_eq!(
            ours.lfQuality, ANTIALIASED_QUALITY,
            "our face is the one asked for grey coverage"
        );
        assert_ne!(ours.lfQuality, base.lfQuality, "and the base is not");
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
        4,
        "the declaration and the three faces of `DialogFonts::new` — and nothing else asks for \
         a face: {created:?}"
    );

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
        6,
        "the five EDITTEXT fields and the layout list — the remainder task T-11-20 names"
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
        let our_face = Face::new(settings::antialiased_logfont(base));

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
    let inset = settings::scaled(settings::LIST_SELECTION_INSET, 96);
    let radius = settings::scaled(settings::LIST_SELECTION_RADIUS, 96);

    // The corner tile of `paint_rounded`: the radius and the frame that runs around it.
    let side = radius + settings::scaled(settings::BORDER_THICKNESS, 96).max(1);

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

        let tiles = settings::corner_tiles(&stripe, side);
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
    let source = settings_module_source();
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
    let side = settings::scaled(settings::LIST_CHECK_CORNER_RADIUS, 96)
        + settings::scaled(settings::BORDER_THICKNESS, 96).max(1);

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

                let tiles = settings::corner_tiles(&glyph, side);
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

/// The five input fields of `IDD_SETTINGS`, by identifier.
///
/// The same five `EDITTEXT` rows `settings::FRAMED_FIELDS` names minus the two lists — the
/// controls a `12`-unit box is drawn round and whose own rectangle is one font height.
const INPUT_FIELDS: [(u32, &str); 5] = [
    (1010, "Горячая клавиша: поле клавиши"),
    (1032, "Замена: задержка между событиями"),
    (1041, "Выделение: таймаут буфера обмена"),
    (1042, "Выделение: задержка восстановления"),
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
/// [`settings::field_frame_air`] round the middle of the control. Read out of the **built**
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
    let air = settings::field_frame_air(Some(field_box), control, 1);
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
    assert_eq!(settings::field_frame_air(Some(23), 15, 1), 4);
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
            settings::field_frame_air(box_height, control_height, 1),
            1,
            "«{name}» must fall back to one border thickness"
        );
    }

    // A thicker border at a higher DPI is the floor, not the answer.
    assert_eq!(settings::field_frame_air(Some(23), 23, 2), 2);

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

/// **Defect Г-1 of the Э12 comparison, «критично»** — the hint of the language row and the
/// «Оформление» combo box are disjoint rectangles.
///
/// An `SS_OWNERDRAW` static fills its own rectangle with `panel_bg` before it writes a word,
/// so a static overlapping a combo box does not sit on top of it — it **erases** the top of
/// the frame the background drew. The template is where the overlap was and the template is
/// where it is answered: two rectangles, disjoint by arithmetic, with air between the rows.
#[test]
fn the_restart_hint_and_the_appearance_row_do_not_overlap() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let (hint_left, hint_top, hint_right, hint_bottom) = template.rect_of(1092);
    let (combo_left, combo_top, combo_right, _) = template.rect_of(1003);

    println!(
        "хинт (1092): {hint_left},{hint_top}..{hint_right},{hint_bottom}; \
         «Оформление» (1003): {combo_left},{combo_top}..{combo_right},…"
    );

    // Where the generator puts the hint: beside the language row it belongs to.
    assert_eq!(
        (hint_left, hint_top),
        (132, 32),
        "the hint stands at the generator's own place — ui.ps1: x = 132, y = 32"
    );

    // Disjoint, and by the vertical: the two share the whole of their width.
    assert!(
        hint_left < combo_right && combo_left < hint_right,
        "the two rectangles overlap horizontally, which is why the vertical is what has to \
         part them"
    );
    assert!(
        hint_bottom <= combo_top,
        "the hint ends at {hint_bottom} and «Оформление» starts at {combo_top} — the static \
         erases the top of the combo box's frame, which is defect Г-1"
    );
    assert!(
        combo_top - hint_bottom >= 2,
        "there has to be air between the two rows: {} units",
        combo_top - hint_bottom
    );

    // And the row still fits the panel «Общие», whose 66 units are a decision of the user
    // (В-1) and not a number this task may move.
    let (_, panel_top, _, panel_bottom) = template.rect_of(1090);

    assert_eq!(
        (panel_top, panel_bottom),
        (7, 73),
        "«Общие» is 7..73 dialog units — decision В-1, canon"
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
/// because the frame of a list is drawn [`settings::LIST_FIRST_ROW_TOP`] mock-up pixels above
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
    let lifted = settings::scaled(settings::LIST_FIRST_ROW_TOP, 96).max(1);

    let list_frame = vertical_units(list_top, base) - lifted;
    let button_frame = vertical_units(button_top, base);

    println!(
        "список (1050) y={list_top} → рамка {list_frame} px; кнопка (1053) y={button_top} → \
         рамка {button_frame} px; поднятие {lifted} px"
    );

    assert_eq!(
        list_frame, button_frame,
        "the frame of the list is drawn {lifted} px above the control and the button frames \
         its own rectangle — declared level they come out apart, which is defect Д-3"
    );

    // And the bottom of the list did not move: 44 units at y = 160 and 43 at y = 161 end on
    // the same row.
    assert_eq!(
        vertical_units(list_top, base) + vertical_units(list_bottom - list_top, base),
        vertical_units(160, base) + vertical_units(44, base),
        "the bottom of the list has to stay where it was — only its top moved"
    );
}

/// **Defect Д-4 of the Э12 comparison** — the journal path stands at the generator's own y.
#[test]
fn the_journal_path_stands_where_the_generator_puts_it() {
    let product = ProductImage::open();
    let template = DialogTemplate::parse(&product.resource(RT_DIALOG, IDD_SETTINGS));

    let (left, top, right, bottom) = template.rect_of(1062);

    println!("путь журнала (1062): {left},{top}..{right},{bottom}");

    assert_eq!(
        (left, top, right - left, bottom - top),
        (220, 273, 186, 16),
        "ui.ps1 writes the journal path at x = 220, y = 273, w = 186, h = 16; the 275 it stood \
         at was two units low"
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

/// Paints one caption through the product's own [`settings::paint_label`] with a given face.
fn paint_label_in_face(sheet: &Sheet, area: RECT, caption: &str, face: &Face) -> isize {
    let mut text: Vec<u16> = caption.encode_utf16().collect();

    // SAFETY: the brush is made here, used only by the call below and freed here.
    let ground = unsafe { CreateSolidBrush(LABEL_GROUND) };

    // SAFETY: `sheet.dc` holds this sheet's bitmap, `text` and `area` are live locals of this
    // frame, and both `ground` and the face outlive the call.
    let answer = unsafe {
        settings::paint_label(sheet.dc, area, &mut text, ground, LABEL_INK, Some(face.0))
    };

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(ground.into()) };

    answer
}

/// Paints one caption the way the module painted **every** label before task T-12-12: ground,
/// then one `DrawTextW` with [`settings::LABEL_TEXT_FORMAT`]. The reference the separating
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
            settings::LABEL_TEXT_FORMAT,
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
/// number counted on them is 1,4 × too large as a screen length until [`settings::scaled`]
/// divides it back. The chain that produces this one is written where the constant is, and
/// this test holds the chain there.
#[test]
fn the_line_pitch_of_a_wrapped_label_is_the_line_spacing_of_the_generators_face() {
    assert_eq!(
        settings::LABEL_LINE_PITCH,
        23,
        "the pitch of the mock-ups is 1,33 em of the generator's own face — 22,345 mock-up \
         pixels, whose whole steps are 22 and 23 and whose two lines of this label landed on 23"
    );

    let source = settings_module_source();
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
        settings::scaled(22, 96),
        settings::scaled(settings::LABEL_LINE_PITCH, 96)
    );

    assert!(
        (223..=224).contains(&pitch_tenths),
        "the generator's line spacing must come out at 22,3 mock-up pixels — it came out \
         {pitch_tenths} tenths, so the chain in the doc comment no longer holds"
    );
    assert_eq!(
        settings::scaled(settings::LABEL_LINE_PITCH, 96),
        16,
        "решение В-6: 23 mock-up pixels are 16 pixels at 96 DPI, against the 15 `DrawTextW` \
         advances a line by on its own"
    );
    assert_eq!(
        settings::scaled(22, 96),
        settings::scaled(settings::LABEL_LINE_PITCH, 96),
        "the whole-pixel rounding of the mock-up literal is not what decides the screen: 22 \
         and 23 mock-up pixels are the same 16 screen pixels, and the fraction between them \
         is the 15,96 the arithmetic asks for"
    );

    // And it scales with the DPI of the window, which is the second half of В-6.
    assert!(
        settings::scaled(settings::LABEL_LINE_PITCH, 144)
            > settings::scaled(settings::LABEL_LINE_PITCH, 96),
        "the pitch has to grow with the DPI of the window — it goes through `scaled` for that"
    );
}

/// **The separating question of T-12-12, as a table** — which labels take the model pitch and
/// which are left exactly as they were drawn before the task.
///
/// [`settings::label_model_pitch`] is the whole of the decision and it is pure, so it can be
/// closed here rather than photographed. Either «no» means the single plain `DrawTextW` of
/// T-11-18, unchanged to the pixel.
#[test]
fn only_a_label_that_really_wrapped_takes_the_model_pitch() {
    // 96 DPI: `DrawTextW` advances a line by tmHeight = 15 and the model asks for 16.
    assert_eq!(
        settings::label_model_pitch(2, 15, 16),
        Some(16),
        "«вступит в силу после перезапуска» is the label this task exists for: two lines, and \
         a model pitch a pixel wider than the natural one"
    );

    // The seventeen one-line labels. This is the row that keeps them still.
    assert_eq!(
        settings::label_model_pitch(1, 15, 16),
        None,
        "a label that did not wrap has no pitch to set, and seventeen of the eighteen \
         OWNER_DRAWN_LABELS are one line — moving them would be «rewrote the label drawing», \
         not «fixed the line pitch»"
    );
    assert_eq!(
        settings::label_model_pitch(0, 15, 16),
        None,
        "no lines at all is no pitch either"
    );

    // Three lines take it as readily as two: nothing here counts to two.
    assert_eq!(
        settings::label_model_pitch(3, 15, 16),
        Some(16),
        "the rule is «it wrapped», not «it wrapped once»"
    );

    // A model pitch that is not wider has nothing to give: `DrawTextW` already advances by
    // `natural`, and squeezing lines together is not what В-6 asked for.
    assert_eq!(
        settings::label_model_pitch(2, 16, 16),
        None,
        "a model pitch equal to the natural one is the drawing that already happens, and one \
         plain call is both cheaper and exact"
    );
    assert_eq!(
        settings::label_model_pitch(2, 17, 16),
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
        settings::label_model_pitch(2, 15, 16),
        Some(16),
        "a journal path that did wrap has to get the model pitch too, short control or not — \
         the code leads both multi-line labels by one rule"
    );
}

/// **Criterion 7 of T-12-12, on real pixels** — a wrapped label's lines stand
/// `scaled(LABEL_LINE_PITCH, dpi)` apart, and not the face's own `tmHeight`.
///
/// The caption is painted through the product's own [`settings::paint_label`] with the
/// template's own face, and the ink is read back band by band exactly as `inkrows.ps1` reads
/// it off a screenshot of the stand.
#[test]
fn the_lines_of_a_wrapped_label_stand_the_model_pitch_apart() {
    let (sheet, face, metrics, dpi) = label_sheet();
    let pitch = settings::scaled(settings::LABEL_LINE_PITCH, dpi);

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
/// rectangle — once through [`settings::paint_label`] and once through the single plain
/// `DrawTextW` the module used before — and the two bitmaps must be identical.
#[test]
fn a_one_line_label_is_drawn_exactly_as_it_was_before_the_pitch() {
    let (sheet, face, metrics, dpi) = label_sheet();

    println!(
        "sheet at {dpi} DPI: tmHeight = {}, model pitch = {}",
        metrics.tmHeight,
        settings::scaled(settings::LABEL_LINE_PITCH, dpi)
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
    let pitch = settings::scaled(settings::LABEL_LINE_PITCH, dpi);

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
