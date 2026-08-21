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
// three states (normal, pressed, disabled). The `WM_DRAWITEM` handler itself needs a live
// dialog and is checked by the controller on the real window at acceptance, along with
// Enter/Esc/Space. The identifiers are literals, not imports — the rule of the template
// tests below: a test that imported the numbers would agree with any renumbering.

#[test]
fn the_button_colour_roles_follow_the_closed_table_of_fr_92a() {
    use ButtonFaceRole as Face;
    use ButtonTextRole as Ink;

    // Every ordinary button of the dialog by its actual identifier — including «Отмена»
    // (2, the manager's own IDCANCEL): the accent belongs to «ОК» alone, so each of these
    // is driven through all three states and must never show it.
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

    // (pressed, disabled) → the expected face and ink of an ordinary button. The frame is
    // `button_border` in every row of the table — asserted below with the whole struct.
    let ordinary_states = [
        (false, false, Face::ButtonBg, Ink::Text),
        (true, false, Face::SelBg, Ink::SelFg),
        (false, true, Face::ButtonBg, Ink::TextMuted),
    ];

    for (control, name) in ordinary {
        for (pressed, disabled, face, text) in ordinary_states {
            assert_eq!(
                settings::button_color_roles(control, pressed, disabled),
                ButtonColors {
                    face,
                    text,
                    border: ButtonBorderRole::ButtonBorder,
                },
                "«{name}» ({control}), pressed = {pressed}, disabled = {disabled}"
            );
        }
    }

    // The default button «ОК» — identifier 1, the dialog manager's own IDOK, the number
    // `DM_SETDEFID` is sent with: the accent pair in its normal state; the selection pair
    // while pressed (the accent yields for the length of the press); muted ink on the
    // ordinary face when disabled — a disabled button takes no Enter and must not
    // advertise itself as the default.
    let default_states = [
        (false, false, Face::AccentBg, Ink::AccentFg),
        (true, false, Face::SelBg, Ink::SelFg),
        (false, true, Face::ButtonBg, Ink::TextMuted),
    ];

    for (pressed, disabled, face, text) in default_states {
        assert_eq!(
            settings::button_color_roles(1, pressed, disabled),
            ButtonColors {
                face,
                text,
                border: ButtonBorderRole::ButtonBorder,
            },
            "«ОК» (1), pressed = {pressed}, disabled = {disabled}"
        );
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

use windows::Win32::Foundation::{FreeLibrary, HMODULE, HRSRC, RECT};
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
    /// Every non-empty control caption, in template order.
    text: Vec<String>,
    /// The identifier of every control.
    controls: Vec<u32>,
    /// The style of every control, paired with its identifier, in template order — task
    /// T-11-5a reads the button types of the nine owner-drawn buttons out of this.
    styles: Vec<(u32, u32)>,
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

        // x, y, cx, cy.
        for _ in 0..4 {
            let _ = read_u16(bytes, &mut at);
        }

        let _menu = read_name(bytes, &mut at);
        let _class = read_name(bytes, &mut at);
        let caption = read_string(bytes, &mut at);

        // `DS_SETFONT`, which the template declares: point size, weight, italic, charset and
        // the type face follow the caption.
        if style & 0x0000_0040 != 0 {
            let _points = read_u16(bytes, &mut at);
            let _weight = read_u16(bytes, &mut at);
            at += 2;
            let _face = read_string(bytes, &mut at);
        }

        let mut text = Vec::new();
        let mut controls = Vec::new();
        let mut styles = Vec::new();

        for _ in 0..items {
            at = (at + 3) & !3;

            let _help_id = read_u32(bytes, &mut at);
            let _ex_style = read_u32(bytes, &mut at);
            let style = read_u32(bytes, &mut at);

            for _ in 0..4 {
                let _ = read_u16(bytes, &mut at);
            }

            let id = read_u32(bytes, &mut at);

            let _class = read_name(bytes, &mut at);
            let title = read_name(bytes, &mut at);

            let extra = usize::from(read_u16(bytes, &mut at));
            at += extra;

            controls.push(id);
            styles.push((id, style));

            if let Some(title) = title
                && !title.is_empty()
            {
                text.push(title);
            }
        }

        Self {
            caption,
            text,
            controls,
            styles,
        }
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
const FR_94_STRINGS: [(u16, &str, &str); 67] = [
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
