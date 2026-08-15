//! The configuration: the schema of section 7 of SPEC, reading, writing, defaults and
//! version migration — and the settings dialog of FR-92 that shows it to a person.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC: «Диалог настроек,
//! схема конфигурации, чтение и запись, миграция версий».
//!
//! Requirements covered here: the configuration schema of section 7 of SPEC, in full
//! (task T-01-3); **FR-92**, the settings dialog with every section of its table, and
//! **FR-93**, autostart through `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! (task T-08-1). Still to come: FR-94, hotkey capture with RU and EN string tables
//! (task T-08-2) — the hotkey field of the dialog therefore *shows* the configured key
//! and does not capture one. The distribution of requirements over modules follows the
//! backlog, which is its source of truth (decision R-17).
//!
//! SEC-01 and SEC-07. This is the only module of the program that writes to disk at all.
//! Nothing derived from a keystroke, from a converted character or from the clipboard is
//! part of the schema, and none of it may reach an error message either: a damaged
//! configuration file can hold anything whatsoever, so a parse failure is reported by
//! position alone and never quotes what it read. The dialog obeys the same rule and it is
//! the sharper case: it is the one window this program shows, and the only text it can
//! display is the configuration, the names of the session's keyboard layouts, the names of
//! processes the user typed into the exclusion list themselves, and counters. **No stroke,
//! no character and no clipboard content reaches a control, an error path or a panic.**
//!
//! # The dialog is a reader of the other modules
//!
//! Section 6.2 gives every other module its own responsibility, and the dialog does not take
//! any of it over: it calls the public *readers* of `layouts`, `guard`, `diag` and `watchdog`
//! to show what they hold, and it changes what they do by exactly one route — writing the
//! configuration and letting the caller publish it (section 6.3). Nothing here reaches into
//! another module's state.
//!
//! # Where it runs — section 6.1
//!
//! On the UI thread and nowhere else. [`show_dialog`] is modal: `DialogBoxParamW` runs a
//! message loop of its own until the dialog ends, which is exactly why the tray must not be
//! borrowed across the call (see [`crate::tray`]). The hook is on another thread, so a dialog
//! open for an hour costs the program nothing — that separation is what section 6.1 exists
//! for.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::UI::Controls::{
    BST_CHECKED, BST_UNCHECKED, CheckDlgButton, CheckRadioButton, EM_LIMITTEXT,
    ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, IsDlgButtonChecked,
    LIST_VIEW_ITEM_STATE_FLAGS, LVCF_WIDTH, LVCOLUMNW, LVIF_STATE, LVIF_TEXT, LVIS_FOCUSED,
    LVIS_SELECTED, LVIS_STATEIMAGEMASK, LVITEMW, LVM_DELETEALLITEMS, LVM_GETITEMSTATE,
    LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETEXTENDEDLISTVIEWSTYLE,
    LVM_SETITEMSTATE, LVNI_SELECTED, LVS_EX_CHECKBOXES, LVS_EX_FULLROWSELECT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CB_ADDSTRING, CB_GETCURSEL, CB_RESETCONTENT, CB_SETCURSEL, DialogBoxParamW, EndDialog,
    GWLP_USERDATA, GetClientRect, GetDlgItem, GetDlgItemTextW, GetWindowLongPtrW, IDCANCEL, IDOK,
    LB_ADDSTRING, LB_DELETESTRING, LB_GETCOUNT, LB_GETCURSEL, LB_GETTEXT, LB_GETTEXTLEN,
    LB_RESETCONTENT, SW_SHOWNORMAL, SendDlgItemMessageW, SetDlgItemTextW, SetWindowLongPtrW,
    WM_COMMAND, WM_INITDIALOG,
};
use windows::core::{Error as WinError, PCWSTR, PWSTR, w};

use crate::CONFIG_DIR_NAME;
use crate::layouts::{self, LayoutId, LayoutSpec};

/// File name of the configuration inside the program's application data directory.
///
/// Section 7 of SPEC: `%APPDATA%\Lang_Switcher\config.toml`.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// The schema version this build writes and fully understands.
///
/// Section 7 of SPEC pins it at 1.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// The version this build assigns to a file that carries no `schema_version` field.
///
/// Nobody ever wrote this number into a file: it is the label given to the earliest form
/// of the configuration so that the migration ladder in [`Config::migrate`] has a rung to
/// start from.
const PRE_VERSION_SCHEMA: u32 = 0;

/// Serde default for [`Config::schema_version`], see [`PRE_VERSION_SCHEMA`].
///
/// Deliberately *not* the current version: a file that omits the field comes from the
/// earliest schema and has to travel through migration like any other old file.
fn pre_version_schema() -> u32 {
    PRE_VERSION_SCHEMA
}

/// Interface language, `general.language` of section 7. Values `ru` and `en`.
///
/// A closed set rather than a free string: a value outside it must not pass silently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Russian, the default of section 7.
    #[default]
    Ru,
    /// English.
    En,
}

/// Layout switching mode, `layouts.mode` of section 7. Values `pair` and `cycle`.
///
/// A closed set rather than a free string: a value outside it must not pass silently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// Switch between [`Layouts::pair_source`] and [`Layouts::pair_target`]. The default
    /// of section 7, confirmed by decision 19: there are exactly two layouts.
    #[default]
    Pair,
    /// Walk the [`Layouts::cycle`] list.
    Cycle,
}

/// Text replacement method, `replacement.method` of section 7. Values `backspace` and
/// `selection`.
///
/// A closed set rather than a free string: a value outside it must not pass silently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplacementMethod {
    /// Erase the typed run with backspaces and retype it. The default of section 7.
    #[default]
    Backspace,
    /// Select the run just typed with `Shift+Left` and let the insertion replace it — the
    /// compatibility mode of **FR-42**.
    ///
    /// ⚠ **The clipboard takes no part in this.** What is selected is the program's own
    /// freshly typed run, character for character, and the replacement is still delivered by
    /// `KEYEVENTF_UNICODE` (module `inject`). The clipboard route is a different requirement
    /// altogether — the selection *path* of FR-60 to FR-65, driven by `[selection] enabled`
    /// and implemented in module `selection` — and confusing the two is what the earlier
    /// wording of this comment did. Section 4.3 of `STATE.md` carried it as a documentation
    /// debt; task T-08-1 is where it is paid.
    Selection,
}

/// Section `[general]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct General {
    /// Whether correction is armed at all. Default `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether the program starts with the session. Default `true`.
    #[serde(default = "default_true")]
    pub autostart: bool,
    /// Interface language. Default `ru`.
    #[serde(default)]
    pub language: Language,
}

impl Default for General {
    fn default() -> Self {
        Self {
            enabled: true,
            autostart: true,
            language: Language::Ru,
        }
    }
}

/// Section `[hotkey]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hotkey {
    /// Name of the virtual key. Default `Pause`.
    ///
    /// Kept as a string here on purpose: turning a key name into a virtual key code and
    /// back is the business of the hotkey capture UI (FR-94, task T-08-2), and this
    /// module must not grow a key table that would then exist in two places.
    #[serde(default = "default_hotkey_key")]
    pub key: String,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            key: default_hotkey_key(),
        }
    }
}

/// Section `[layouts]` of section 7, with the defaults confirmed by decision 19.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layouts {
    /// Switching mode. Default `pair`.
    #[serde(default)]
    pub mode: LayoutMode,
    /// Source layout identifier of the pair. Default `0x00000409`, en-US.
    #[serde(default = "default_pair_source")]
    pub pair_source: String,
    /// Target layout identifier of the pair. Default `0x00000419`, ru-RU.
    #[serde(default = "default_pair_target")]
    pub pair_target: String,
    /// Layout identifiers walked in `cycle` mode. Default the same two, in this order.
    #[serde(default = "default_cycle")]
    pub cycle: Vec<String>,
}

impl Default for Layouts {
    fn default() -> Self {
        Self {
            mode: LayoutMode::Pair,
            pair_source: default_pair_source(),
            pair_target: default_pair_target(),
            cycle: default_cycle(),
        }
    }
}

/// Section `[replacement]` of section 7.
///
/// Both defaults of section 7 — `backspace` and `0` — coincide with the defaults of the
/// field types, so the derive is the whole implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    /// How the typed run is replaced. Default `backspace`.
    #[serde(default)]
    pub method: ReplacementMethod,
    /// Pause inserted between synthesised input events, milliseconds. Default `0`.
    #[serde(default)]
    pub inter_event_delay_ms: u32,
}

/// Section `[selection]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Whether the selection path is available at all. Default `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// How long to wait for the clipboard to carry the selection, milliseconds.
    /// Default `300`.
    #[serde(default = "default_clipboard_timeout_ms")]
    pub clipboard_timeout_ms: u32,
    /// How long to wait before restoring the previous clipboard, milliseconds.
    /// Default `200`.
    #[serde(default = "default_clipboard_restore_delay_ms")]
    pub clipboard_restore_delay_ms: u32,
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            enabled: true,
            clipboard_timeout_ms: default_clipboard_timeout_ms(),
            clipboard_restore_delay_ms: default_clipboard_restore_delay_ms(),
        }
    }
}

/// Section `[buffer]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Buffer {
    /// Capacity of the typing buffer in characters. Default `256`.
    #[serde(default = "default_buffer_capacity")]
    pub capacity: usize,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            capacity: default_buffer_capacity(),
        }
    }
}

/// Section `[exclusions]` of section 7.
///
/// The default of section 7 — an empty list — coincides with the default of the field
/// type, so the derive is the whole implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusions {
    /// Process names in which the program stays silent. Default an empty list.
    #[serde(default)]
    pub processes: Vec<String>,
}

/// Section `[diagnostics]` of section 7.
///
/// The default of section 7 — logging off — coincides with the default of the field type,
/// so the derive is the whole implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostics {
    /// Whether the diagnostic log is written to a file. Default `false`.
    ///
    /// SEC-07 constrains what the log may contain; it does not belong to this module.
    #[serde(default)]
    pub log_enabled: bool,
}

/// The whole configuration file of section 7.
///
/// Every field carries a serde default, which is the machine-checkable form of the rule of
/// section 7: *unknown fields are ignored, missing ones are filled from the defaults*. The
/// ignoring half comes for free — no `deny_unknown_fields` anywhere in this module, and
/// two tests hold that in place.
///
/// Section 6.3. The type is plain owned data: `Clone`, `Send`, `Sync` and `'static`, with
/// no interior references and no locks. That is what lets the UI thread clone a whole
/// configuration and publish it to the input thread through an atomic pointer, with no
/// mutex anywhere near the hook path (NFR-04). Publishing itself is not this task's work;
/// not making it impossible is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// Schema version of the file. Current value [`CURRENT_SCHEMA_VERSION`].
    ///
    /// A file without the field is treated as [`PRE_VERSION_SCHEMA`] and migrated; see
    /// [`Config::migrate`].
    #[serde(default = "pre_version_schema")]
    pub schema_version: u32,
    /// Section `[general]`.
    #[serde(default)]
    pub general: General,
    /// Section `[hotkey]`.
    #[serde(default)]
    pub hotkey: Hotkey,
    /// Section `[layouts]`.
    #[serde(default)]
    pub layouts: Layouts,
    /// Section `[replacement]`.
    #[serde(default)]
    pub replacement: Replacement,
    /// Section `[selection]`.
    #[serde(default)]
    pub selection: Selection,
    /// Section `[buffer]`.
    #[serde(default)]
    pub buffer: Buffer,
    /// Section `[exclusions]`.
    #[serde(default)]
    pub exclusions: Exclusions,
    /// Section `[diagnostics]`.
    #[serde(default)]
    pub diagnostics: Diagnostics,
}

impl Default for Config {
    /// The configuration of section 7 exactly as printed there, `schema_version` included.
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            general: General::default(),
            hotkey: Hotkey::default(),
            layouts: Layouts::default(),
            replacement: Replacement::default(),
            selection: Selection::default(),
            buffer: Buffer::default(),
            exclusions: Exclusions::default(),
            diagnostics: Diagnostics::default(),
        }
    }
}

// Serde default helpers. They exist because `#[serde(default)]` on a field reaches for the
// default of the field's *type*, which for `bool` is `false` and for `String` is empty —
// neither of which is what section 7 says.

/// Serde default for the three boolean fields of section 7 that start out on.
fn default_true() -> bool {
    true
}

/// Serde default for `hotkey.key`.
fn default_hotkey_key() -> String {
    "Pause".to_owned()
}

/// Serde default for `layouts.pair_source`, en-US (decision 19).
fn default_pair_source() -> String {
    "0x00000409".to_owned()
}

/// Serde default for `layouts.pair_target`, ru-RU (decision 19).
fn default_pair_target() -> String {
    "0x00000419".to_owned()
}

/// Serde default for `layouts.cycle`, the same two layouts in this order (decision 19).
fn default_cycle() -> Vec<String> {
    vec![default_pair_source(), default_pair_target()]
}

/// Serde default for `selection.clipboard_timeout_ms`.
fn default_clipboard_timeout_ms() -> u32 {
    300
}

/// Serde default for `selection.clipboard_restore_delay_ms`.
fn default_clipboard_restore_delay_ms() -> u32 {
    200
}

/// Serde default for `buffer.capacity`.
fn default_buffer_capacity() -> usize {
    256
}

/// What reading a configuration ended up being.
///
/// Returned next to the configuration rather than folded into it, because the caller acts
/// on it: a missing file asks to be created, a migrated file asks to be written back, and
/// a file from a newer schema asks to be left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOutcome {
    /// There was no file. The defaults are current by construction; nothing was migrated.
    NoFile,
    /// The file already carried [`CURRENT_SCHEMA_VERSION`].
    Current,
    /// The file carried an older schema and was raised to the current one in memory. The
    /// file on disk is untouched until somebody calls [`write_to`].
    Migrated {
        /// The version the file came in with.
        from: u32,
    },
    /// The file was written by a newer build. It was parsed as far as this schema allows
    /// and its `schema_version` was left as it was: this build is not required to
    /// understand such a file, but it is required not to damage it.
    FromNewerSchema {
        /// The version found in the file.
        version: u32,
    },
}

/// Everything that can go wrong reading or writing the configuration.
///
/// SEC-01 and SEC-07. No variant carries text taken from the file. A configuration file
/// can be edited by hand, pasted into or otherwise filled with anything at all, so a
/// parse failure is described by its position and nothing else.
#[derive(Debug)]
pub enum ConfigError {
    /// The file, or the directory holding it, could not be read, written or replaced.
    Io(io::Error),
    /// The file is not valid TOML, or a field holds a value the schema does not admit —
    /// an unknown value of an enumerated field, or a value of the wrong type.
    ///
    /// `at` is the one-based line and column the parser stopped at, absent when it could
    /// not name a position.
    Malformed {
        /// One-based line and column.
        at: Option<(usize, usize)>,
    },
    /// The configuration could not be turned into TOML.
    ///
    /// Carries no detail on purpose: the serializer's own message can quote the offending
    /// value, and a value can have come from a file. Serialising [`Config`] cannot in
    /// practice fail — every field is a plain scalar, string or array of strings — so the
    /// variant exists to keep the error path total rather than to be acted upon.
    Serialize,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "configuration file could not be accessed: {err}"),
            Self::Malformed {
                at: Some((line, column)),
            } => write!(
                f,
                "configuration file is malformed at line {line}, column {column}"
            ),
            Self::Malformed { at: None } => write!(f, "configuration file is malformed"),
            Self::Serialize => write!(f, "configuration could not be serialised to TOML"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Malformed { .. } | Self::Serialize => None,
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Turns a byte offset into the document into a one-based line and column.
///
/// Only the offset reported by the parser is used; the text is walked, never copied and
/// never handed on, which is what keeps SEC-01 intact on the error path. Continuation
/// bytes of multi-byte characters are skipped so that the column counts characters.
fn line_and_column(text: &str, offset: usize) -> (usize, usize) {
    let stop = offset.min(text.len());
    let mut line = 1usize;
    let mut column = 1usize;
    for byte in &text.as_bytes()[..stop] {
        if *byte == b'\n' {
            line += 1;
            column = 1;
        } else if byte & 0b1100_0000 != 0b1000_0000 {
            column += 1;
        }
    }
    (line, column)
}

impl Config {
    /// Parses a TOML document and brings it to the current schema version.
    ///
    /// Unknown fields are ignored and missing ones are filled from the defaults, both by
    /// construction rather than by extra code. An empty document therefore parses into
    /// exactly [`Config::default`].
    pub fn from_toml_str(text: &str) -> Result<(Self, ReadOutcome), ConfigError> {
        let mut config: Self = toml::from_str(text).map_err(|err| ConfigError::Malformed {
            at: err.span().map(|span| line_and_column(text, span.start)),
        })?;
        let outcome = config.migrate();
        Ok((config, outcome))
    }

    /// Renders the configuration as a TOML document holding every section of section 7.
    ///
    /// The whole structure is written every time, so a field the caller never touched
    /// survives the write. A field that is not part of the schema does not: section 7 has
    /// it ignored on the way in, and there is nothing left to write it back from.
    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        toml::to_string(self).map_err(|_| ConfigError::Serialize)
    }

    /// Raises the configuration to [`CURRENT_SCHEMA_VERSION`], one version at a time.
    ///
    /// The ladder below is the whole mechanism, and it is meant to be extended by adding
    /// rungs, never by rewriting: schema 2 costs one `step_1_to_2` function and one `if`
    /// underneath the existing one. Files from the future are recognised and returned
    /// untouched — this build cannot know what a later schema means, and guessing would
    /// destroy the very fields it does not understand.
    fn migrate(&mut self) -> ReadOutcome {
        let from = self.schema_version;
        if from > CURRENT_SCHEMA_VERSION {
            return ReadOutcome::FromNewerSchema { version: from };
        }
        if from == CURRENT_SCHEMA_VERSION {
            return ReadOutcome::Current;
        }
        if self.schema_version < 1 {
            step_0_to_1(self);
        }
        ReadOutcome::Migrated { from }
    }
}

/// Raises a file from the pre-versioning form to schema 1.
///
/// Schema 0 is the configuration as it was before the version marker existed: the same
/// sections under the same names, only unstamped. No field changed name, type or meaning,
/// so everything a schema 0 file holds has already been read by the current parser by the
/// time this runs, and raising the file means stamping the version onto it. Later steps
/// are where field surgery goes; the shape of the ladder is what this one establishes.
fn step_0_to_1(config: &mut Config) {
    config.schema_version = 1;
}

/// Builds the configuration path inside an arbitrary application data directory.
///
/// Split out from [`default_config_path`] so that the layout of the path can be asserted
/// without an environment and without touching the real `%APPDATA%`.
pub fn config_path_in(app_data: &Path) -> PathBuf {
    app_data.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME)
}

/// The standard configuration path, `%APPDATA%\Lang_Switcher\config.toml` (section 7).
///
/// `None` when `APPDATA` is not set in the environment, which for an interactive Windows
/// session does not happen; the caller decides whether that is fatal. Read from the
/// environment rather than through `SHGetKnownFolderPath`: the value is already in the
/// process, and a Win32 call would mean an `unsafe` block (NFR-14) and a return value to
/// check (NFR-13) for nothing gained.
pub fn default_config_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|app_data| config_path_in(Path::new(&app_data)))
}

/// Reads the configuration from `path`.
///
/// A missing file is not a failure: on a first run there is none, and section 7 has
/// missing values come from the defaults, so [`ReadOutcome::NoFile`] is returned together
/// with [`Config::default`]. A file that cannot be parsed *is* a failure and is reported
/// as one — the caller that must not fail because of it is [`read_or_default`].
///
/// Reading never writes anything back, migration included: what to do with a file that
/// was migrated, or with one from a newer schema, is the caller's decision.
pub fn read_from(path: &Path) -> Result<(Config, ReadOutcome), ConfigError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok((Config::default(), ReadOutcome::NoFile));
        }
        Err(err) => return Err(ConfigError::Io(err)),
    };
    Config::from_toml_str(&text)
}

/// Reads the configuration from `path` and falls back to the defaults on any failure.
///
/// This is the entry point of a resident utility at startup. A configuration file with a
/// stray bracket in it must not keep the program from running, so the configuration comes
/// back usable in every case; the second half of the pair says what actually happened, so
/// that the caller can report it (`diag`) and decide whether the file may be overwritten.
/// A failed read never leads to the file being rewritten from here: overwriting is a
/// decision, not a side effect.
pub fn read_or_default(path: &Path) -> (Config, Result<ReadOutcome, ConfigError>) {
    match read_from(path) {
        Ok((config, outcome)) => (config, Ok(outcome)),
        Err(err) => (Config::default(), Err(err)),
    }
}

/// Writes the configuration to `path`, atomically, creating the directory if needed.
///
/// The write goes to a temporary file beside the target, is flushed to the device, and
/// only then replaces the target with a single rename. The rename is same-directory and
/// therefore same-volume, which is what makes it atomic; `fs::rename` replaces an existing
/// destination on Windows. A crash, a power cut or a killed process leaves either the old
/// file or the new one, never half of either — the configuration is written by the UI
/// thread while the program runs, and a truncated file would silently reset every setting
/// the user has.
///
/// The temporary file is removed if anything after its creation fails, so a failed write
/// leaves no litter next to the configuration.
pub fn write_to(path: &Path, config: &Config) -> Result<(), ConfigError> {
    let text = config.to_toml_string()?;

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let temporary = temporary_path_for(path);
    if let Err(err) = write_and_sync(&temporary, text.as_bytes()) {
        let _ = fs::remove_file(&temporary);
        return Err(err);
    }
    if let Err(err) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(ConfigError::Io(err));
    }
    Ok(())
}

/// Names the temporary file used by [`write_to`], beside the target.
///
/// Beside it and not in `%TEMP%`, because a rename is only atomic within one volume. The
/// process id keeps two instances of the program from colliding; within one process the
/// configuration is written by the UI thread alone (section 6.3), so nothing finer is
/// needed.
fn temporary_path_for(path: &Path) -> PathBuf {
    let mut name = match path.file_name() {
        Some(name) => name.to_os_string(),
        None => OsString::from(CONFIG_FILE_NAME),
    };
    name.push(format!(".{}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// Writes `bytes` to `path` and flushes them to the device before returning.
///
/// The flush is the point: without it the rename can reach the disk ahead of the contents
/// and a power cut leaves an empty configuration where a whole one was promised.
fn write_and_sync(path: &Path, bytes: &[u8]) -> Result<(), ConfigError> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

// =========================================================================================
// FR-93 — autostart through the registry
// =========================================================================================

/// The subkey of `HKEY_CURRENT_USER` FR-93 names.
///
/// ⚠ **`HKEY_CURRENT_USER` and this one subkey are the whole of what this program does to the
/// registry**, and the whole of what it may ever do. The machine-wide hive is not written to
/// under any circumstance: a per-user utility that installs itself for every account is a
/// program doing something its user did not ask for, and writing there needs rights section
/// 8.1 deliberately does not take (SEC-05).
const RUN_KEY_PATH: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");

/// Name of the one value this program owns under [`RUN_KEY_PATH`].
///
/// The display name of decision 7, because that is what the user sees in the autostart list of
/// the Task Manager, and it is the same in both interface locales (FR-94).
const AUTOSTART_VALUE_NAME: PCWSTR = w!("Lang Switcher");

/// The `Run` key of the current user, closed when this value is dropped.
struct RunKey(HKEY);

impl RunKey {
    /// Opens [`RUN_KEY_PATH`] under `HKEY_CURRENT_USER` with the access asked for.
    ///
    /// Opened and never created. The key exists in every user profile Windows makes, so a
    /// failure here means something is wrong with the profile rather than that the key is
    /// merely absent — and creating registry keys is not what FR-93 asks for.
    fn open(access: REG_SAM_FLAGS) -> windows::core::Result<Self> {
        let mut key = HKEY::default();

        // SAFETY: `RUN_KEY_PATH` is a NUL-terminated `'static` UTF-16 literal, so the pointer
        // the call reads through outlives the call. `key` is a live local the call writes
        // exactly one handle into, and it is written only on success — which is checked on the
        // next line, so a failed open cannot leave a bogus handle to be closed. The handle
        // becomes the property of the returned value and is closed exactly once, in `Drop`.
        let status =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY_PATH, None, access, &mut key) };

        status.ok()?;

        Ok(Self(key))
    }
}

impl Drop for RunKey {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from a successful `RegOpenKeyExW` and is closed exactly once —
        // this type is neither `Copy` nor `Clone` and hands the handle out to nobody.
        let status = unsafe { RegCloseKey(self.0) };

        if let Err(error) = status.ok() {
            // NFR-13: the result is examined. Nothing can be done about a handle that refuses
            // to close, and the process is not brought down over it.
            crate::app::report_non_critical("RegCloseKey", &error);
        }
    }
}

/// The command line FR-93 puts in the registry: the running executable, quoted.
///
/// Quoted because a path with a space in it — `C:\Program Files\Lang_Switcher\…`, which is
/// exactly where section 8.4 installs the program — is otherwise read as a command plus
/// arguments. `None` when the path of the running image cannot be determined, which on Windows
/// does not happen; the caller then leaves the registry alone rather than writing something
/// made up.
pub fn autostart_command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;

    Some(format!("\"{}\"", exe.display()))
}

/// The value under [`RUN_KEY_PATH`], or `None` when this program is not registered there.
///
/// Reads and never writes: this is what the dialog shows and what a check of FR-93 reads back.
pub fn autostart_value() -> Option<String> {
    let key = RunKey::open(KEY_QUERY_VALUE).ok()?;

    let mut kind = REG_VALUE_TYPE::default();
    let mut bytes: u32 = 0;

    // SAFETY: the two-call pattern the registry API is built on. Both out parameters are live
    // locals of this frame, the data pointer is `None`, which is what asks for the size alone,
    // and the value name is a `'static` NUL-terminated literal. Nothing is written except the
    // two locals, and only on success — checked immediately.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            AUTOSTART_VALUE_NAME,
            None,
            Some(&mut kind),
            None,
            Some(&mut bytes),
        )
    };

    // NFR-13. `ERROR_FILE_NOT_FOUND` here is the ordinary answer "autostart is off" and is not
    // reported as a failure; every other refusal simply leaves the answer unknown.
    if status.is_err() || kind != REG_SZ || bytes == 0 {
        return None;
    }

    let units = usize::try_from(bytes).ok()? / size_of::<u16>();
    let mut buffer = vec![0u16; units + 1];
    let mut capacity = u32::try_from(buffer.len() * size_of::<u16>()).ok()?;

    // SAFETY: `buffer` is a live allocation of `capacity` bytes owned by this frame, and
    // `capacity` describes it exactly, so the call cannot write past its end — it writes at
    // most `capacity` bytes and reports how many it wrote back through the same variable. The
    // pointer is taken from the buffer and not retained by the call.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            AUTOSTART_VALUE_NAME,
            None,
            None,
            Some(buffer.as_mut_ptr().cast::<u8>()),
            Some(&mut capacity),
        )
    };

    if status.is_err() {
        return None;
    }

    let written = usize::try_from(capacity).ok()? / size_of::<u16>();
    let text = &buffer[..written.min(buffer.len())];
    let end = text
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(text.len());

    Some(String::from_utf16_lossy(&text[..end]))
}

/// Whether this program is registered to start with the session — FR-93, read from the truth.
///
/// The check mark in the tray and in the dialog shows `general.autostart` of section 7, which
/// is what the file says; this is what the *system* says. They are kept equal by
/// [`set_autostart`], and the dialog shows both so that a disagreement is visible rather than
/// hidden.
pub fn autostart_registered() -> bool {
    autostart_value().is_some()
}

/// Makes the registry agree with `enabled` — the whole of FR-93.
///
/// Idempotent in both directions: writing the value twice is one value, and deleting one that
/// is not there is not a failure. That is what lets the caller run this on every apply without
/// having to remember what the previous state was.
pub fn set_autostart(enabled: bool) -> windows::core::Result<()> {
    let key = RunKey::open(KEY_SET_VALUE | KEY_QUERY_VALUE)?;

    if !enabled {
        // SAFETY: `key.0` is a live key opened for setting values and owned by this frame;
        // the value name is a `'static` NUL-terminated literal. The call reads through the
        // name and writes nothing of ours.
        let status = unsafe { RegDeleteValueW(key.0, AUTOSTART_VALUE_NAME) };

        // A value that was not there is the state that was asked for.
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }

        return status.ok();
    }

    let Some(command) = autostart_command() else {
        // Nothing truthful to write. Leaving the registry untouched is the only answer that
        // cannot make matters worse.
        return Err(WinError::from_thread());
    };

    let units = wide(&command);

    // The registry stores `REG_SZ` as bytes, terminator included, and `wide` supplies the
    // terminator.
    //
    // SAFETY: `units` is a live `Vec<u16>` of this frame; the slice covers exactly the same
    // allocation reinterpreted as bytes, which is always valid — every bit pattern of `u8` is
    // an initialised value and the alignment of `u8` is one, so no requirement of the source
    // type is violated. The view is read-only and does not outlive `units`.
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(units.as_ptr().cast::<u8>(), units.len() * size_of::<u16>())
    };

    // SAFETY: as above for the key and the name. `bytes` is a live read-only view of this
    // frame's buffer, and its length is what bounds the copy the registry makes; the call
    // copies the data and keeps no pointer.
    let status = unsafe { RegSetValueExW(key.0, AUTOSTART_VALUE_NAME, None, REG_SZ, Some(bytes)) };

    status.ok()
}

// =========================================================================================
// FR-92 — the settings dialog
// =========================================================================================

/// Resource identifier of the dialog template in `app.rc`.
///
/// ⚠ Kept equal to `IDD_SETTINGS` there by hand, because a `.rc` file and a Rust file have no
/// shared header. A mismatch is not silent: `DialogBoxParamW` fails outright and the failure is
/// reported, which is the reason the number is far away from the icon identifiers.
pub const IDD_SETTINGS: u16 = 200;

// Control identifiers, mirrored from `app.rc`. Same rule as above.
const IDC_AUTOSTART: i32 = 1001;
const IDC_LANGUAGE: i32 = 1002;
const IDC_HOTKEY: i32 = 1010;
const IDC_HOTKEY_NOTE: i32 = 1011;
const IDC_MODE_PAIR: i32 = 1020;
const IDC_MODE_CYCLE: i32 = 1021;
const IDC_PAIR_SOURCE: i32 = 1022;
const IDC_PAIR_TARGET: i32 = 1023;
const IDC_CYCLE_LIST: i32 = 1024;
const IDC_CYCLE_UP: i32 = 1025;
const IDC_CYCLE_DOWN: i32 = 1026;
const IDC_LAYOUT_NOTE: i32 = 1027;
const IDC_METHOD_BACKSPACE: i32 = 1030;
const IDC_METHOD_SELECTION: i32 = 1031;
const IDC_DELAY: i32 = 1032;
const IDC_SELECTION_ENABLED: i32 = 1040;
const IDC_CLIPBOARD_TIMEOUT: i32 = 1041;
const IDC_CLIPBOARD_RESTORE: i32 = 1042;
const IDC_EXCLUSIONS: i32 = 1050;
const IDC_EXCLUSION_NAME: i32 = 1051;
const IDC_EXCLUSION_ADD: i32 = 1052;
const IDC_EXCLUSION_REMOVE: i32 = 1053;
const IDC_LOG_ENABLED: i32 = 1060;
const IDC_LOG_OPEN: i32 = 1061;
const IDC_LOG_DIR: i32 = 1062;
const IDC_STATE_HOOK: i32 = 1070;
const IDC_STATE_LAYOUTS: i32 = 1071;
const IDC_STATE_AUTOSTART: i32 = 1072;
const IDC_APPLY: i32 = 1080;

/// `IDOK` as the number the dialog manager sends in `WM_COMMAND`.
///
/// The crate's metadata types the constant as `MESSAGEBOX_RESULT`, because that is one of the
/// places it appears; what arrives in a `WM_COMMAND` is the plain number, and it is the plain
/// number that is matched on.
const OK_COMMAND: i32 = IDOK.0;

/// `IDCANCEL`, for the reason above. This is also what the dialog manager sends when the user
/// presses `Esc` or closes the window, which is why «Отмена» needs no separate handling.
const CANCEL_COMMAND: i32 = IDCANCEL.0;

/// Digits a millisecond field accepts, so that [`parse_ms`] cannot meet a number that overflows.
const MS_FIELD_DIGITS: usize = 9;

/// Characters the exclusion name field accepts.
///
/// Sixty-four, against the hundred and twenty-eight **bytes** `guard::MAX_EXCLUSION_NAME_BYTES`
/// publishes: the worst case is two bytes per character, and a name longer than this is refused
/// by `guard` rather than truncated, so the field stops it where the user can see it happening.
const EXCLUSION_NAME_CHARS: usize = 64;

/// State image index of a ticked checkbox in a list view, in the form the item state carries it.
const CHECKED_IMAGE: u32 = 0x2000;

/// State image index of an unticked one.
const UNCHECKED_IMAGE: u32 = 0x1000;

/// One line of the layout list of FR-31: a layout of this session and whether it takes part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutRow {
    /// The layout itself, as `layouts` enumerated it.
    pub layout: LayoutId,
    /// Whether `[layouts] cycle` names it.
    pub checked: bool,
}

/// Orders the session's layouts the way the cycle list of FR-31 is to be shown.
///
/// The ones `[layouts] cycle` names come first, **in the order the file gives them**, because
/// that order is the cycle (FR-31); everything else follows in system order, unticked. A name
/// in the file that no layout of this session answers to is dropped rather than shown: FR-35
/// keeps IMEs out of the list altogether, and a layout that was uninstalled since the file was
/// written is not something the user can be asked about.
///
/// Pure, and public so that the ordering can be checked without a window — it is the one part
/// of the layouts section that can be wrong without any Win32 call failing.
pub fn layout_rows(layouts: &Layouts, session: &[LayoutId]) -> Vec<LayoutRow> {
    let mut rows: Vec<LayoutRow> = Vec::with_capacity(session.len());

    for text in &layouts.cycle {
        let spec = LayoutSpec::parse(text);

        for layout in session {
            if spec.matches(*layout) && !rows.iter().any(|row| row.layout == *layout) {
                rows.push(LayoutRow {
                    layout: *layout,
                    checked: true,
                });
            }
        }
    }

    for layout in session {
        if !rows.iter().any(|row| row.layout == *layout) {
            rows.push(LayoutRow {
                layout: *layout,
                checked: false,
            });
        }
    }

    rows
}

/// Renders the ticked rows back into `[layouts] cycle` — the inverse of [`layout_rows`].
///
/// Pure, and public for the same reason.
pub fn cycle_from_rows(rows: &[LayoutRow], session: &[LayoutId]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.checked)
        .map(|row| spec_text(row.layout, session))
        .collect()
}

/// Writes one layout the way section 7 writes one — and picks which of the two forms to use.
///
/// A session with one layout per language gets the **language** form, `0x00000409`, which is
/// what section 7 prints and what survives a re-login: the handle of a layout is assigned by
/// the session and the language identifier is not. A session that has the same language loaded
/// twice gets the **handle** form, `0x04090409`, because the language form cannot tell the two
/// apart and picking either one silently would be a guess.
///
/// Both forms are what `layouts::LayoutSpec` already reads, so nothing is added to the schema.
pub fn spec_text(layout: LayoutId, session: &[LayoutId]) -> String {
    let same_language = session
        .iter()
        .filter(|other| other.language_id() == layout.language_id())
        .count();

    if same_language > 1 {
        format!("0x{:08X}", layout.raw())
    } else {
        format!("0x{:08X}", u32::from(layout.language_id()))
    }
}

/// Reads a millisecond field.
///
/// An empty field is zero, which is the documented default of `inter_event_delay_ms` and a
/// value the user may legitimately want. Anything the parser refuses keeps `fallback`, the
/// value that was there before — the field is limited to [`MS_FIELD_DIGITS`] digits and
/// `ES_NUMBER` admits nothing but digits, so that branch is unreachable through the dialog and
/// exists to keep the function total.
pub fn parse_ms(text: &str, fallback: u32) -> u32 {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return 0;
    }

    trimmed.parse::<u32>().unwrap_or(fallback)
}

/// Whether a virtual key produces text, which FR-92 asks to warn about.
///
/// The letters, the digits, the space, the numeric pad and the OEM keys — everything that puts
/// a character into the field the user is typing in. Binding the hotkey to one of them means
/// the key stops typing that character, because FR-95 suppresses the hotkey whenever the
/// program is active.
pub fn is_text_key(vk: u16) -> bool {
    matches!(vk,
        0x20                    // VK_SPACE
        | 0x30..=0x39           // the digits
        | 0x41..=0x5A           // the letters
        | 0x60..=0x6F           // the numeric pad, its operators included
        | 0xBA..=0xC0           // VK_OEM_1 .. VK_OEM_3
        | 0xDB..=0xDF           // VK_OEM_4 .. VK_OEM_8
        | 0xE2                  // VK_OEM_102
    )
}

/// The note the hotkey section shows under the key name, if it has anything to say.
///
/// Two things can be wrong with `[hotkey] key`, and both of them are silent everywhere else in
/// the program: a name this build does not know leaves the default of section 7 in force
/// (`app::publish_configuration` says so in as many words), and a text key disappears from
/// typing for as long as the program is active (FR-95). The dialog is where FR-92 puts the
/// telling.
pub fn hotkey_note(key: &str) -> Option<&'static str> {
    match crate::hook::vk_from_name(key) {
        None => Some("Имя клавиши не распознано — действует клавиша по умолчанию, Pause."),
        Some(vk) if is_text_key(vk) => {
            Some("Это текстовая клавиша: пока программа активна, она перестанет вводить символ.")
        }
        Some(_) => None,
    }
}

/// Shows the settings dialog of FR-92, modally, and returns when it closes.
///
/// `apply` is called with the configuration the user has assembled every time they press
/// «Применить» or «ОК», and never otherwise — «Отмена» leaves the file and the running program
/// exactly as they were. The caller decides what applying means: this module writes no file of
/// somebody else's and publishes nothing to another thread, because section 6.3 gives the
/// publication of the configuration to the owner of it, and section 6.2 gives the dialog to
/// this module.
///
/// ⚠ **Modal.** `DialogBoxParamW` runs a message loop until the dialog ends, so the caller must
/// hold no borrow of anything the window procedure of this thread can reach — the same rule the
/// tray states for `TrackPopupMenuEx` and `MessageBoxW`.
pub fn show_dialog(
    owner: HWND,
    instance: HINSTANCE,
    config: &Config,
    apply: &mut dyn FnMut(&Config),
) -> windows::core::Result<()> {
    // ⚠ One dialog at a time. A modal dialog runs a message loop that keeps dispatching to the
    // *other* windows of this thread, so the tray icon can be clicked while this window is up
    // and would otherwise open a second copy of it on top of the first — two windows editing
    // two copies of one configuration, of which the last one applied would win. The guard is a
    // thread-local because the dialog belongs to the UI thread and to no other (section 6.1).
    if DIALOG_OPEN.with(Cell::get) {
        return Ok(());
    }

    // The list view of the cycle lives in `comctl32`, whose window classes are registered on
    // demand. Without this the whole dialog fails to be created — one missing class takes the
    // template with it — so the result is examined (NFR-13) before anything is shown.
    ensure_list_view_class()?;

    DIALOG_OPEN.with(|open| open.set(true));

    // Cleared however this function leaves, including on a panic on the way through — the
    // program stays up after one (FR-98, FR-99), and a flag left set would mean the settings
    // window could never be opened again.
    struct OpenGuard;

    impl Drop for OpenGuard {
        fn drop(&mut self) {
            DIALOG_OPEN.with(|open| open.set(false));
        }
    }

    let _open = OpenGuard;

    let session = layouts::enumerate().unwrap_or_default();

    let state = RefCell::new(DialogState {
        working: config.clone(),
        rows: layout_rows(&config.layouts, &session),
        session,
        apply,
    });

    // SAFETY: `instance` is a module handle whose resources carry `IDD_SETTINGS`, and the
    // "name" is an integer identifier in the `MAKEINTRESOURCE` form — a value below 65536
    // carried inside the pointer, never dereferenced as a string. `owner` is a live window of
    // this thread. The parameter is a pointer to `state`, which lives on this frame: the call
    // is modal and does not return until `EndDialog`, so the pointer cannot outlive the value
    // it names. `dialog_proc` is the only reader of it and reads it through a `RefCell`, so no
    // two borrows can overlap however the dialog manager re-enters.
    let result = unsafe {
        DialogBoxParamW(
            Some(instance),
            resource_id(IDD_SETTINGS),
            Some(owner),
            Some(dialog_proc),
            LPARAM(std::ptr::from_ref(&state) as isize),
        )
    };

    // NFR-13. `DialogBoxParamW` answers -1 when the dialog could not be created at all — a
    // missing template, a control class that is not registered — and that is the one outcome
    // that is a failure. Everything else is what `EndDialog` was called with.
    if result == -1 {
        return Err(WinError::from_thread());
    }

    Ok(())
}

thread_local! {
    /// Whether this thread already has a settings dialog on the screen — see [`show_dialog`].
    static DIALOG_OPEN: Cell<bool> = const { Cell::new(false) };
}

/// Everything the dialog procedure needs, for as long as the dialog is up.
///
/// Lives on the frame of [`show_dialog`] and is reached through a `RefCell` behind a raw
/// pointer, which is the standard shape of a Win32 dialog: the manager gives the procedure one
/// `LPARAM` and nothing else. The `RefCell` is not decoration — a message dispatched while
/// another message is being handled would otherwise alias a `&mut`, and here it is refused at
/// run time instead.
struct DialogState<'a> {
    /// The configuration being edited. Starts as a copy of the caller's and is never anything
    /// but a copy: nothing outside this dialog sees it until `apply` is called.
    working: Config,
    /// The layout list of FR-31, in display order.
    rows: Vec<LayoutRow>,
    /// The layouts of this session, as `layouts::enumerate` gave them — FR-35 has already
    /// removed the IMEs.
    session: Vec<LayoutId>,
    /// What to do with a configuration the user asked to apply.
    apply: &'a mut dyn FnMut(&Config),
}

/// The dialog procedure of FR-92.
///
/// # Safety
///
/// Called by the dialog manager with the arguments of a window message. `hwnd` names the live
/// dialog, and on `WM_INITDIALOG` `lparam` is the pointer [`show_dialog`] passed and nothing
/// else — the manager forwards it unchanged and no other sender can reach this procedure,
/// because it is not registered as a window class anybody can create (SEC-05).
unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match message {
        WM_INITDIALOG => {
            // SAFETY: `hwnd` is the live dialog and `GWLP_USERDATA` is a field every window
            // has, which the dialog manager does not use for itself — its own storage is the
            // `DWLP_*` range. The value stored is the pointer the manager forwarded from
            // `DialogBoxParamW`; it is only ever read back by `with_state`, below.
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0) };

            // SAFETY: the pointer has just been stored and names the `RefCell` on the frame of
            // `show_dialog`, which outlives this modal call.
            unsafe { with_state(hwnd, |state| fill_dialog(hwnd, state)) };

            // TRUE: let the dialog manager choose the focus.
            1
        }

        WM_COMMAND => {
            let control = i32::from(low_word(wparam.0));
            let notification = high_word(wparam.0);

            // SAFETY: as above — the pointer was stored on `WM_INITDIALOG` and the value it
            // names is alive for the whole of this modal call.
            unsafe { on_command(hwnd, control, notification) };

            0
        }

        _ => 0,
    }
}

/// Runs `f` against the dialog's state.
///
/// `None` when there is no state yet — messages do arrive before `WM_INITDIALOG` — or when the
/// state is already borrowed, which is what makes a re-entrant message harmless instead of
/// undefined.
///
/// # Safety
///
/// May only be called from the dialog procedure of a dialog created by [`show_dialog`], whose
/// `GWLP_USERDATA` therefore holds either zero or the pointer that function stored.
unsafe fn with_state<R>(hwnd: HWND, f: impl FnOnce(&mut DialogState<'_>) -> R) -> Option<R> {
    // SAFETY: `hwnd` is the live dialog; reading a window field is a plain read.
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

    if raw == 0 {
        return None;
    }

    // SAFETY: by the contract above, `raw` is the pointer `show_dialog` stored, which names a
    // `RefCell<DialogState>` on that function's frame. The call is modal, so the frame is alive
    // for as long as any message of this dialog can be handled. A shared reference is all that
    // is taken, and the `RefCell` is what governs the mutable access below.
    let cell = unsafe { &*(raw as *const RefCell<DialogState<'_>>) };

    let mut state = cell.try_borrow_mut().ok()?;

    Some(f(&mut state))
}

/// Puts the configuration into the controls — the whole of `WM_INITDIALOG`.
fn fill_dialog(hwnd: HWND, state: &mut DialogState<'_>) {
    // Section «Общие» of FR-92.
    set_check(hwnd, IDC_AUTOSTART, state.working.general.autostart);
    send_to(hwnd, IDC_LANGUAGE, CB_RESETCONTENT, 0, 0);
    combo_add(hwnd, IDC_LANGUAGE, "Русский");
    combo_add(hwnd, IDC_LANGUAGE, "English");
    send_to(
        hwnd,
        IDC_LANGUAGE,
        CB_SETCURSEL,
        match state.working.general.language {
            Language::Ru => 0,
            Language::En => 1,
        },
        0,
    );

    // Section «Горячая клавиша» of FR-92. The field shows the key and does not capture one —
    // capture is FR-94's neighbour in task T-08-2 — so it is read-only, and the note under it
    // is the "предупреждение" the requirement asks for.
    set_text(hwnd, IDC_HOTKEY, &state.working.hotkey.key);
    set_text(
        hwnd,
        IDC_HOTKEY_NOTE,
        hotkey_note(&state.working.hotkey.key).unwrap_or(""),
    );

    // Section «Раскладки» of FR-92 — FR-30, FR-31, FR-35.
    fill_layouts(hwnd, state);

    // Section «Замена» of FR-92 — FR-41, FR-42, FR-44.
    let method = match state.working.replacement.method {
        ReplacementMethod::Backspace => IDC_METHOD_BACKSPACE,
        ReplacementMethod::Selection => IDC_METHOD_SELECTION,
    };
    check_radio(hwnd, IDC_METHOD_BACKSPACE, IDC_METHOD_SELECTION, method);
    limit_text(hwnd, IDC_DELAY, MS_FIELD_DIGITS);
    set_text(
        hwnd,
        IDC_DELAY,
        &state.working.replacement.inter_event_delay_ms.to_string(),
    );

    // Section «Выделение» of FR-92 — FR-61 and FR-65.
    set_check(hwnd, IDC_SELECTION_ENABLED, state.working.selection.enabled);
    limit_text(hwnd, IDC_CLIPBOARD_TIMEOUT, MS_FIELD_DIGITS);
    limit_text(hwnd, IDC_CLIPBOARD_RESTORE, MS_FIELD_DIGITS);
    set_text(
        hwnd,
        IDC_CLIPBOARD_TIMEOUT,
        &state.working.selection.clipboard_timeout_ms.to_string(),
    );
    set_text(
        hwnd,
        IDC_CLIPBOARD_RESTORE,
        &state
            .working
            .selection
            .clipboard_restore_delay_ms
            .to_string(),
    );

    // Section «Исключения» of FR-92 — FR-84.
    send_to(hwnd, IDC_EXCLUSIONS, LB_RESETCONTENT, 0, 0);
    for name in &state.working.exclusions.processes {
        list_add(hwnd, IDC_EXCLUSIONS, name);
    }
    limit_text(hwnd, IDC_EXCLUSION_NAME, EXCLUSION_NAME_CHARS);

    // Section «Диагностика» of FR-92 — SEC-07.
    set_check(hwnd, IDC_LOG_ENABLED, state.working.diagnostics.log_enabled);
    set_text(
        hwnd,
        IDC_LOG_DIR,
        &crate::diag::log_dir().map_or_else(
            || "%APPDATA% не задан — журнал писать некуда".to_owned(),
            |dir| dir.display().to_string(),
        ),
    );

    fill_state_lines(hwnd, state);
}

/// Fills the layouts section and the list view of the cycle.
fn fill_layouts(hwnd: HWND, state: &mut DialogState<'_>) {
    let mode = match state.working.layouts.mode {
        LayoutMode::Pair => IDC_MODE_PAIR,
        LayoutMode::Cycle => IDC_MODE_CYCLE,
    };
    check_radio(hwnd, IDC_MODE_PAIR, IDC_MODE_CYCLE, mode);

    for control in [IDC_PAIR_SOURCE, IDC_PAIR_TARGET] {
        send_to(hwnd, control, CB_RESETCONTENT, 0, 0);

        for layout in &state.session {
            combo_add(hwnd, control, &layout_label(*layout));
        }
    }

    let source = LayoutSpec::parse(&state.working.layouts.pair_source);
    let target = LayoutSpec::parse(&state.working.layouts.pair_target);

    select_layout(hwnd, IDC_PAIR_SOURCE, source, &state.session);
    select_layout(hwnd, IDC_PAIR_TARGET, target, &state.session);

    // The note of FR-92 for this section: a field of section 7 that names no layout of this
    // session is otherwise silent — `layouts::LayoutSpec` says as much and points here.
    let unresolved = [(source, "источник"), (target, "цель")]
        .into_iter()
        .filter(|(spec, _)| spec.resolve(&state.session).is_none())
        .map(|(_, name)| name)
        .collect::<Vec<_>>();

    set_text(
        hwnd,
        IDC_LAYOUT_NOTE,
        &if unresolved.is_empty() {
            String::new()
        } else {
            format!(
                "В сеансе нет раскладки, названной в файле ({}) — выберите заново.",
                unresolved.join(", ")
            )
        },
    );

    prepare_cycle_list(hwnd);
    fill_cycle_list(hwnd, &state.rows, 0);
    enable_by_mode(hwnd, state.working.layouts.mode);
}

/// The three read-only lines of the «Состояние» group.
///
/// **Every number here comes from another module's public reader** — `watchdog`, `layouts`,
/// `guard`, `diag` — and nothing is written back to any of them. Section 6.2: the dialog shows
/// their state, it does not own it. SEC-07: counts and a folder path, never a stroke.
fn fill_state_lines(hwnd: HWND, state: &DialogState<'_>) {
    let health = crate::watchdog::health();

    set_text(
        hwnd,
        IDC_STATE_HOOK,
        &format!(
            "Перехват клавиатуры: {} · восстановлений хука: {} · отказов установки: {}",
            if crate::watchdog::hook_down() {
                "НЕ УСТАНОВЛЕН"
            } else {
                "установлен"
            },
            health.recoveries,
            health.install_failures
        ),
    );

    set_text(
        hwnd,
        IDC_STATE_LAYOUTS,
        &format!(
            "Раскладок в сеансе: {} · исключений опубликовано: {} · записей в журнале: {}",
            state.session.len(),
            crate::guard::counters().exclusions,
            crate::diag::recorded()
        ),
    );

    set_text(
        hwnd,
        IDC_STATE_AUTOSTART,
        &format!(
            "Автозапуск в реестре (HKCU\\…\\Run): {}",
            match autostart_value() {
                Some(command) => format!("есть, {command}"),
                None => "нет".to_owned(),
            }
        ),
    );
}

/// Reads every control back into the working configuration.
///
/// The configuration it writes into is the one the dialog was opened with, so a field FR-92
/// does not show — `general.enabled`, `[buffer] capacity`, `schema_version` — survives the
/// round trip untouched. That is not a detail: writing the file replaces it whole.
fn read_dialog(hwnd: HWND, state: &mut DialogState<'_>) {
    state.working.general.autostart = is_checked(hwnd, IDC_AUTOSTART);
    state.working.general.language = match send_to(hwnd, IDC_LANGUAGE, CB_GETCURSEL, 0, 0) {
        1 => Language::En,
        _ => Language::Ru,
    };

    state.working.layouts.mode = if is_checked(hwnd, IDC_MODE_CYCLE) {
        LayoutMode::Cycle
    } else {
        LayoutMode::Pair
    };

    // A combo box with nothing selected leaves the field as the file had it. The user cannot
    // empty a `CBS_DROPDOWNLIST`, so this is the case of a file naming a layout the session
    // does not have: the note in the dialog says so, and the value is kept rather than
    // silently replaced by whatever happens to be first in the list.
    if let Some(layout) = selected_layout(hwnd, IDC_PAIR_SOURCE, &state.session) {
        state.working.layouts.pair_source = spec_text(layout, &state.session);
    }
    if let Some(layout) = selected_layout(hwnd, IDC_PAIR_TARGET, &state.session) {
        state.working.layouts.pair_target = spec_text(layout, &state.session);
    }

    read_cycle_checks(hwnd, &mut state.rows);
    state.working.layouts.cycle = cycle_from_rows(&state.rows, &state.session);

    state.working.replacement.method = if is_checked(hwnd, IDC_METHOD_SELECTION) {
        ReplacementMethod::Selection
    } else {
        ReplacementMethod::Backspace
    };
    state.working.replacement.inter_event_delay_ms = parse_ms(
        &get_text(hwnd, IDC_DELAY),
        state.working.replacement.inter_event_delay_ms,
    );

    state.working.selection.enabled = is_checked(hwnd, IDC_SELECTION_ENABLED);
    state.working.selection.clipboard_timeout_ms = parse_ms(
        &get_text(hwnd, IDC_CLIPBOARD_TIMEOUT),
        state.working.selection.clipboard_timeout_ms,
    );
    state.working.selection.clipboard_restore_delay_ms = parse_ms(
        &get_text(hwnd, IDC_CLIPBOARD_RESTORE),
        state.working.selection.clipboard_restore_delay_ms,
    );

    state.working.exclusions.processes = list_items(hwnd, IDC_EXCLUSIONS);

    state.working.diagnostics.log_enabled = is_checked(hwnd, IDC_LOG_ENABLED);
}

/// One command from the dialog.
///
/// # Safety
///
/// Called from [`dialog_proc`] only, with the `hwnd` of the dialog it belongs to.
unsafe fn on_command(hwnd: HWND, control: i32, _notification: u16) {
    match control {
        // Applying and leaving are one operation followed by the other, which is why «ОК»
        // makes the same call «Применить» makes and then ends the dialog.
        OK_COMMAND => {
            // SAFETY: see the caller.
            unsafe { apply_now(hwnd) };
            end_dialog(hwnd, isize::try_from(OK_COMMAND).unwrap_or(0));
        }

        IDC_APPLY => {
            // SAFETY: see the caller.
            unsafe { apply_now(hwnd) };
        }

        // Nothing was written to the file and nothing was published, so there is nothing to
        // undo: the working configuration was a copy from the first line of `show_dialog`.
        CANCEL_COMMAND => end_dialog(hwnd, isize::try_from(CANCEL_COMMAND).unwrap_or(0)),

        IDC_MODE_PAIR | IDC_MODE_CYCLE => {
            let mode = if is_checked(hwnd, IDC_MODE_CYCLE) {
                LayoutMode::Cycle
            } else {
                LayoutMode::Pair
            };

            enable_by_mode(hwnd, mode);
        }

        IDC_EXCLUSION_ADD => add_exclusion(hwnd),
        IDC_EXCLUSION_REMOVE => remove_exclusion(hwnd),

        // SAFETY: see the caller.
        IDC_CYCLE_UP => unsafe { move_cycle_row(hwnd, -1) },
        // SAFETY: see the caller.
        IDC_CYCLE_DOWN => unsafe { move_cycle_row(hwnd, 1) },

        IDC_LOG_OPEN => open_log_folder(hwnd),

        _ => {}
    }
}

/// Reads the controls and hands the result to the caller of [`show_dialog`].
///
/// # Safety
///
/// Called from [`dialog_proc`] only.
unsafe fn apply_now(hwnd: HWND) {
    // The callback runs inside the borrow because it *is* part of the state — see
    // `DialogState`. It writes a file, stores into atomics and touches the registry; it does
    // not pump messages, and a re-entrant message would in any case be refused by the
    // `RefCell` rather than alias anything.
    //
    // SAFETY: see the caller.
    unsafe {
        with_state(hwnd, |state| {
            read_dialog(hwnd, state);

            let updated = state.working.clone();
            (state.apply)(&updated);
        })
    };

    // Outside the borrow above: the state lines show what the modules answer *after* the
    // publication, and reading them needs no state of the dialog beyond the session list.
    //
    // SAFETY: see the caller.
    unsafe { with_state(hwnd, |state| fill_state_lines(hwnd, state)) };
}

/// Adds the name in the edit field to the exclusion list — FR-84.
///
/// A blank field and a name already in the list are both "nothing to do": a list with the same
/// process twice would publish the same name twice and cost one of the thirty-two rows
/// `guard::MAX_EXCLUSIONS` allows.
fn add_exclusion(hwnd: HWND) {
    let name = get_text(hwnd, IDC_EXCLUSION_NAME).trim().to_owned();

    if name.is_empty() {
        return;
    }

    let existing = list_items(hwnd, IDC_EXCLUSIONS);

    if !existing
        .iter()
        .any(|item| crate::guard::fold_process_name(item) == crate::guard::fold_process_name(&name))
    {
        list_add(hwnd, IDC_EXCLUSIONS, &name);
    }

    set_text(hwnd, IDC_EXCLUSION_NAME, "");
}

/// Removes the selected name from the exclusion list.
fn remove_exclusion(hwnd: HWND) {
    let selected = send_to(hwnd, IDC_EXCLUSIONS, LB_GETCURSEL, 0, 0);

    // `LB_ERR` is -1 and means nothing is selected.
    if selected < 0 {
        return;
    }

    send_to(
        hwnd,
        IDC_EXCLUSIONS,
        LB_DELETESTRING,
        usize::try_from(selected).unwrap_or(0),
        0,
    );
}

/// Opens the journal folder in the shell — the button of the «Диагностика» section.
///
/// The folder is not created if it is absent: an empty folder appearing because somebody
/// pressed a button is a side effect nobody asked for, and the folder exists as soon as
/// anything has been written to it.
fn open_log_folder(hwnd: HWND) {
    let Some(dir) = crate::diag::log_dir() else {
        return;
    };

    if !dir.is_dir() {
        return;
    }

    let path = wide(&dir.display().to_string());

    // SAFETY: `path` is a NUL-terminated UTF-16 buffer owned by this frame and neither moved
    // nor dropped until the call returns; the two null pointers are the documented way to pass
    // no arguments and no working directory. `hwnd` is the live dialog and becomes the owner of
    // any error box the shell decides to show.
    let result = unsafe {
        ShellExecuteW(
            Some(hwnd),
            w!("open"),
            PCWSTR(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    // NFR-13. `ShellExecuteW` reports failure as a value of 32 or below in what is nominally a
    // module handle — the one Win32 return value that is neither a `BOOL` nor an error code.
    if result.0 as usize <= 32 {
        crate::app::report_non_critical("ShellExecuteW", &WinError::from_thread());
    }
}

/// Moves the selected row of the cycle list by `step` and keeps it selected — FR-31's «задание
/// порядка».
///
/// # Safety
///
/// Called from [`dialog_proc`] only.
unsafe fn move_cycle_row(hwnd: HWND, step: i32) {
    // SAFETY: see the caller.
    unsafe {
        with_state(hwnd, |state| {
            // The ticks live in the control and the order lives in `state.rows`, so the ticks
            // are taken back before the order is changed. Otherwise a tick made since the list
            // was filled would be lost by the refill below.
            read_cycle_checks(hwnd, &mut state.rows);

            let Some(from) = selected_row(hwnd) else {
                return;
            };

            let Some(to) = from.checked_add_signed(step as isize) else {
                return;
            };

            if to >= state.rows.len() {
                return;
            }

            state.rows.swap(from, to);
            fill_cycle_list(hwnd, &state.rows, to);
        })
    };
}

// -----------------------------------------------------------------------------------------
// The list view of the cycle
// -----------------------------------------------------------------------------------------

/// Registers the window classes of `comctl32` the template needs.
fn ensure_list_view_class() -> windows::core::Result<()> {
    let request = INITCOMMONCONTROLSEX {
        dwSize: u32::try_from(size_of::<INITCOMMONCONTROLSEX>()).unwrap_or(0),
        dwICC: ICC_LISTVIEW_CLASSES,
    };

    // SAFETY: `request` is a fully initialised structure owned by this frame, and its `dwSize`
    // describes it, which is how the call knows what it was given. The call reads through the
    // pointer and keeps nothing.
    let registered = unsafe { InitCommonControlsEx(&request) };

    if registered.as_bool() {
        Ok(())
    } else {
        // NFR-13: without the class the dialog cannot be created at all, so this is fatal to
        // the dialog and is reported as such rather than discovered later as a bare -1.
        Err(WinError::from_thread())
    }
}

/// Gives the list view its one column and its check boxes.
fn prepare_cycle_list(hwnd: HWND) {
    // FR-31 asks for «галочки участия», which in a list view is an extended style rather than a
    // control style, so it cannot be declared in the template and is set here.
    send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_SETEXTENDEDLISTVIEWSTYLE,
        0,
        isize::try_from(LVS_EX_CHECKBOXES | LVS_EX_FULLROWSELECT).unwrap_or(0),
    );

    // A report-mode list view shows nothing at all without a column, however many items it
    // holds. The width is taken from the control so that the single column fills it.
    let width = client_width(hwnd, IDC_CYCLE_LIST).unwrap_or(200);

    let column = LVCOLUMNW {
        mask: LVCF_WIDTH,
        // Room for the vertical scroll bar, which appears as soon as the session has more
        // layouts than the control can show.
        cx: (width - 20).max(40),
        ..Default::default()
    };

    // SAFETY: `column` is a fully initialised `LVCOLUMNW` owned by this frame; the message
    // reads through the pointer for the length of the call and keeps nothing. `LVM_INSERTCOLUMNW`
    // takes the index in `wParam` and the structure in `lParam`, which is what is passed.
    send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_INSERTCOLUMNW,
        0,
        std::ptr::from_ref(&column) as isize,
    );
}

/// Fills the list view from the rows and selects `focus`.
fn fill_cycle_list(hwnd: HWND, rows: &[LayoutRow], focus: usize) {
    send_to(hwnd, IDC_CYCLE_LIST, LVM_DELETEALLITEMS, 0, 0);

    for (index, row) in rows.iter().enumerate() {
        let mut label = wide(&layout_label(row.layout));

        let item = LVITEMW {
            mask: LVIF_TEXT,
            iItem: i32::try_from(index).unwrap_or(0),
            pszText: PWSTR(label.as_mut_ptr()),
            ..Default::default()
        };

        // SAFETY: `item` and the buffer its `pszText` points at are both owned by this frame
        // and outlive the call, which copies the text into the control and keeps no pointer.
        send_to(
            hwnd,
            IDC_CYCLE_LIST,
            LVM_INSERTITEMW,
            0,
            std::ptr::from_ref(&item) as isize,
        );

        set_row_check(hwnd, index, row.checked);
    }

    if focus < rows.len() {
        select_row(hwnd, focus);
    }
}

/// Ticks or unticks one row.
fn set_row_check(hwnd: HWND, index: usize, checked: bool) {
    let state = LVITEMW {
        mask: LVIF_STATE,
        state: LIST_VIEW_ITEM_STATE_FLAGS(if checked {
            CHECKED_IMAGE
        } else {
            UNCHECKED_IMAGE
        }),
        stateMask: LVIS_STATEIMAGEMASK,
        ..Default::default()
    };

    // SAFETY: `state` is a fully initialised `LVITEMW` owned by this frame; the message reads
    // through the pointer and keeps nothing. Only the state image bits are touched, which is
    // what `stateMask` says.
    send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_SETITEMSTATE,
        index,
        std::ptr::from_ref(&state) as isize,
    );
}

/// Selects and focuses one row.
fn select_row(hwnd: HWND, index: usize) {
    // The two flags are one bit each and the crate's newtype carries no `BitOr`, so they are
    // combined as the numbers they are.
    let selected = LIST_VIEW_ITEM_STATE_FLAGS(LVIS_SELECTED.0 | LVIS_FOCUSED.0);

    let state = LVITEMW {
        mask: LVIF_STATE,
        state: selected,
        stateMask: selected,
        ..Default::default()
    };

    // SAFETY: as in `set_row_check`.
    send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_SETITEMSTATE,
        index,
        std::ptr::from_ref(&state) as isize,
    );
}

/// The selected row of the list view, if any.
fn selected_row(hwnd: HWND) -> Option<usize> {
    let found = send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_GETNEXTITEM,
        usize::MAX,
        isize::try_from(LVNI_SELECTED).unwrap_or(0),
    );

    usize::try_from(found).ok()
}

/// Copies the ticks out of the control and into the rows.
fn read_cycle_checks(hwnd: HWND, rows: &mut [LayoutRow]) {
    for (index, row) in rows.iter_mut().enumerate() {
        let state = send_to(
            hwnd,
            IDC_CYCLE_LIST,
            LVM_GETITEMSTATE,
            index,
            isize::try_from(LVIS_STATEIMAGEMASK.0).unwrap_or(0),
        );

        row.checked = u32::try_from(state).unwrap_or(0) == CHECKED_IMAGE;
    }
}

/// Enables the half of the layouts section the mode of FR-30 and FR-31 is about.
///
/// FR-92 says the list is «активен в режиме „Несколько"», and the pair is the other way round:
/// a control that cannot affect anything is disabled rather than left to be clicked.
fn enable_by_mode(hwnd: HWND, mode: LayoutMode) {
    let cycle = matches!(mode, LayoutMode::Cycle);

    for control in [IDC_PAIR_SOURCE, IDC_PAIR_TARGET] {
        enable(hwnd, control, !cycle);
    }

    for control in [IDC_CYCLE_LIST, IDC_CYCLE_UP, IDC_CYCLE_DOWN] {
        enable(hwnd, control, cycle);
    }
}

// -----------------------------------------------------------------------------------------
// Layout names
// -----------------------------------------------------------------------------------------

/// What one layout is called in the list and in the two combo boxes.
///
/// The localised name of the language, from the system, with the identifier after it: two
/// layouts of the same language are otherwise indistinguishable, and the identifier is the
/// thing section 7 stores. A system that will not name the language leaves the identifier
/// alone, which is still an answer the user can act on.
fn layout_label(layout: LayoutId) -> String {
    match language_name(layout) {
        Some(name) => format!("{name} — 0x{:08X}", layout.raw()),
        None => format!("0x{:08X}", layout.raw()),
    }
}

/// The localised display name of the language a layout serves, asked of the system.
fn language_name(layout: LayoutId) -> Option<String> {
    let mut locale = [0u16; 85];

    // SAFETY: `locale` is a live buffer of this frame and the call is told its length by the
    // slice, so it cannot write past the end. The language identifier is a plain integer. The
    // return value is the number of units written, zero on failure, and is checked below
    // (NFR-13).
    let written = unsafe {
        windows::Win32::Globalization::LCIDToLocaleName(
            u32::from(layout.language_id()),
            Some(&mut locale),
            0,
        )
    };

    if written <= 0 {
        return None;
    }

    let mut display = [0u16; 128];

    // SAFETY: `locale` holds a NUL-terminated locale name the call above wrote, and `display`
    // is a live buffer of this frame whose length bounds what is written into it. Both pointers
    // are borrowed for the length of the call.
    let written = unsafe {
        windows::Win32::Globalization::GetLocaleInfoEx(
            PCWSTR(locale.as_ptr()),
            windows::Win32::Globalization::LOCALE_SLOCALIZEDDISPLAYNAME,
            Some(&mut display),
        )
    };

    if written <= 0 {
        return None;
    }

    let units = usize::try_from(written).ok()?.saturating_sub(1);

    Some(String::from_utf16_lossy(
        &display[..units.min(display.len())],
    ))
}

/// Selects in `control` the session layout `spec` names, if the session has one.
fn select_layout(hwnd: HWND, control: i32, spec: LayoutSpec, session: &[LayoutId]) {
    let index = session.iter().position(|layout| spec.matches(*layout));

    match index {
        Some(index) => {
            send_to(hwnd, control, CB_SETCURSEL, index, 0);
        }
        None => {
            // `CB_ERR`, which is what "no selection" is spelled as.
            send_to(hwnd, control, CB_SETCURSEL, usize::MAX, 0);
        }
    }
}

/// The session layout selected in `control`, if any.
fn selected_layout(hwnd: HWND, control: i32, session: &[LayoutId]) -> Option<LayoutId> {
    let index = send_to(hwnd, control, CB_GETCURSEL, 0, 0);

    session.get(usize::try_from(index).ok()?).copied()
}

// -----------------------------------------------------------------------------------------
// Control helpers
// -----------------------------------------------------------------------------------------

/// A resource identifier in the `MAKEINTRESOURCE` form the dialog manager expects.
///
/// The identifier goes *into* the pointer and is never dereferenced — the same convention
/// `crate::tray` uses for the icons, and `without_provenance` is what says so in Rust.
fn resource_id(id: u16) -> PCWSTR {
    PCWSTR(std::ptr::without_provenance(usize::from(id)))
}

/// A NUL-terminated UTF-16 copy of `text`, for the Win32 calls that want one.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The low 16 bits of a message parameter.
fn low_word(value: usize) -> u16 {
    u16::try_from(value & 0xFFFF).unwrap_or(0)
}

/// The next 16 bits of a message parameter.
fn high_word(value: usize) -> u16 {
    u16::try_from((value >> 16) & 0xFFFF).unwrap_or(0)
}

/// Sends one message to one control of the dialog.
///
/// NFR-13 asks that the result of a Win32 call be examined, and what comes back here is an
/// `LRESULT` — not a `BOOL`, a handle or a pointer, and for most messages not a success flag at
/// all. Where it *is* one it is examined at the call site: [`selected_row`] and
/// [`selected_layout`] both treat the "nothing is selected" answer as such. The insertions —
/// `CB_ADDSTRING`, `LB_ADDSTRING`, `LVM_INSERTITEMW` — answer an index or an error, and an error
/// there shows up as a row missing from a list the user is looking at, which is visible in a way
/// no journal entry would improve on.
fn send_to(hwnd: HWND, control: i32, message: u32, wparam: usize, lparam: isize) -> isize {
    // SAFETY: `hwnd` is the live dialog and `control` is one of the identifiers of its
    // template; a message to a control that does not exist answers zero rather than doing
    // anything. Where `lparam` is a pointer, the value it points at is owned by the caller's
    // frame for the whole call — every such call site says so.
    unsafe { SendDlgItemMessageW(hwnd, control, message, WPARAM(wparam), LPARAM(lparam)) }.0
}

/// Sets the text of one control.
fn set_text(hwnd: HWND, control: i32, text: &str) {
    let text = wide(text);

    // SAFETY: `text` is a NUL-terminated UTF-16 buffer owned by this frame and not moved or
    // dropped until the call returns; the call copies it.
    if let Err(error) = unsafe { SetDlgItemTextW(hwnd, control, PCWSTR(text.as_ptr())) } {
        // NFR-13. Not fatal: a label that would not take its text is a cosmetic defect, and
        // the program has no better answer than to say so in the journal.
        crate::app::report_non_critical("SetDlgItemTextW", &error);
    }
}

/// Reads the text of one control.
fn get_text(hwnd: HWND, control: i32) -> String {
    let mut buffer = [0u16; 512];

    // SAFETY: `buffer` is owned by this frame and its length is what bounds the copy; the call
    // is given the slice and cannot write past its end. It answers the number of units written,
    // never more than the buffer holds.
    let copied = unsafe { GetDlgItemTextW(hwnd, control, &mut buffer) };

    let copied = usize::try_from(copied).unwrap_or(0);

    String::from_utf16_lossy(&buffer[..copied.min(buffer.len())])
}

/// Ticks or unticks one check box.
fn set_check(hwnd: HWND, control: i32, checked: bool) {
    // SAFETY: `hwnd` is the live dialog and `control` names a button of its template.
    if let Err(error) = unsafe {
        CheckDlgButton(
            hwnd,
            control,
            if checked { BST_CHECKED } else { BST_UNCHECKED },
        )
    } {
        crate::app::report_non_critical("CheckDlgButton", &error);
    }
}

/// Whether one check box or radio button is ticked.
fn is_checked(hwnd: HWND, control: i32) -> bool {
    // SAFETY: as above. The call answers the state of the button and touches no memory of ours.
    unsafe { IsDlgButtonChecked(hwnd, control) == BST_CHECKED.0 }
}

/// Ticks exactly one of a group of radio buttons.
fn check_radio(hwnd: HWND, first: i32, last: i32, chosen: i32) {
    // SAFETY: `hwnd` is the live dialog; the three identifiers name buttons of its template and
    // the range is the one the template declares as a group.
    if let Err(error) = unsafe { CheckRadioButton(hwnd, first, last, chosen) } {
        crate::app::report_non_critical("CheckRadioButton", &error);
    }
}

/// Enables or disables one control.
fn enable(hwnd: HWND, control: i32, enabled: bool) {
    // SAFETY: `hwnd` is the live dialog and `control` names a control of its template; the
    // crate turns a missing control into an error, which is the `Ok` guard below.
    let Ok(window) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
        crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
        return;
    };

    // SAFETY: `window` is the live control just found. The `BOOL` returned is the *previous*
    // state and not a success flag, so NFR-13 has nothing to check here.
    let _ = unsafe { EnableWindow(window, enabled) };
}

/// Limits how much text one edit control accepts.
fn limit_text(hwnd: HWND, control: i32, characters: usize) {
    send_to(hwnd, control, EM_LIMITTEXT, characters, 0);
}

/// The client width of one control, in pixels.
fn client_width(hwnd: HWND, control: i32) -> Option<i32> {
    // SAFETY: `hwnd` is the live dialog; the crate turns a missing control into an error.
    let window = unsafe { GetDlgItem(Some(hwnd), control) }.ok()?;

    let mut rect = windows::Win32::Foundation::RECT::default();

    // SAFETY: `window` is the live control and `rect` is a live local the call fills; nothing
    // else is written.
    unsafe { GetClientRect(window, &mut rect) }.ok()?;

    Some(rect.right - rect.left)
}

/// Appends one string to a combo box.
fn combo_add(hwnd: HWND, control: i32, text: &str) {
    let text = wide(text);

    // SAFETY: `text` is a NUL-terminated UTF-16 buffer owned by this frame and outliving the
    // call, which copies it into the control.
    send_to(hwnd, control, CB_ADDSTRING, 0, text.as_ptr() as isize);
}

/// Appends one string to a list box.
fn list_add(hwnd: HWND, control: i32, text: &str) {
    let text = wide(text);

    // SAFETY: as in `combo_add`.
    send_to(hwnd, control, LB_ADDSTRING, 0, text.as_ptr() as isize);
}

/// Every string of a list box, in order.
fn list_items(hwnd: HWND, control: i32) -> Vec<String> {
    let count = send_to(hwnd, control, LB_GETCOUNT, 0, 0).max(0);
    let mut items = Vec::with_capacity(usize::try_from(count).unwrap_or(0));

    for index in 0..usize::try_from(count).unwrap_or(0) {
        let length = send_to(hwnd, control, LB_GETTEXTLEN, index, 0);

        let Ok(length) = usize::try_from(length) else {
            continue;
        };

        // One for the terminator the control writes and never counts.
        let mut buffer = vec![0u16; length + 1];

        // SAFETY: `buffer` is owned by this frame and holds the length the control has just
        // reported plus the terminator, which is exactly what `LB_GETTEXT` writes. The pointer
        // is not retained by the call.
        let copied = send_to(
            hwnd,
            control,
            LB_GETTEXT,
            index,
            buffer.as_mut_ptr() as isize,
        );

        let copied = usize::try_from(copied).unwrap_or(0).min(buffer.len());

        items.push(String::from_utf16_lossy(&buffer[..copied]));
    }

    items
}

/// Ends the dialog with a result.
fn end_dialog(hwnd: HWND, result: isize) {
    // SAFETY: `hwnd` is the live dialog created by `DialogBoxParamW`, and this is the one call
    // that ends it. It does not destroy the window immediately; the manager unwinds its own
    // loop first, which is why nothing after this call touches the dialog.
    if let Err(error) = unsafe { EndDialog(hwnd, result) } {
        crate::app::report_non_critical("EndDialog", &error);
    }
}
