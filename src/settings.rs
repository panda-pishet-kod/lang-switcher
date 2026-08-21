//! The configuration: the schema of section 7 of SPEC, reading, writing, defaults and
//! version migration — and the settings dialog of FR-92 that shows it to a person.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC: «Диалог настроек,
//! схема конфигурации, чтение и запись, миграция версий».
//!
//! Requirements covered here: the configuration schema of section 7 of SPEC, in full
//! (task T-01-3); **FR-92**, the settings dialog with every section of its table, and
//! **FR-93**, autostart through `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! (task T-08-1); **FR-94**, the interface in Russian and English and the hotkey captured by
//! pressing it (task T-08-2). The distribution of requirements over modules follows the
//! backlog, which is its source of truth (decision R-17).
//!
//! # FR-94 — where the interface text comes from
//!
//! Out of the two string tables of `app.rc`, one per locale, chosen by `general.language` of
//! section 7 and by nothing else — not by the user's Windows UI language, which section 7 does
//! not mention. [`text`] reads one string out of the running program's own image with
//! `FindResourceExW`, giving it the language identifier explicitly; [`set_ui_language`] is what
//! `app` publishes at start-up, once, which is what makes «вступит в силу после перезапуска»
//! beside the combo box a true statement rather than a hopeful one.
//!
//! # FR-94 — where the keystrokes of a capture come from
//!
//! From window messages on the UI thread, through a window procedure this module puts in front
//! of the hotkey field's own — see [`subclass_hotkey_field`] for why not from the low-level
//! hook, and [`CaptureSession`] for what happens to the ordinary work of FR-02 while a capture
//! is armed. **FR-96 is untouched by all of it**: the emergency combination is handled at the
//! top of the hook callback, before anything this module can influence is even read.
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
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    HDC, InvalidateRect, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RedrawWindow, SetBkColor,
    SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceExW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
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
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetKeyState, SetFocus, VIRTUAL_KEY, VK_APPS, VK_CAPITAL, VK_CONTROL, VK_DELETE,
    VK_END, VK_ESCAPE, VK_F1, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_MENU, VK_NEXT, VK_NUMLOCK, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN,
    VK_SCROLL, VK_SHIFT, VK_SNAPSHOT,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CB_ADDSTRING, CB_GETCURSEL, CB_RESETCONTENT, CB_SETCURSEL, CallWindowProcW, DLGC_WANTALLKEYS,
    DefWindowProcW, DialogBoxParamW, EndDialog, GWLP_USERDATA, GWLP_WNDPROC, GetClientRect,
    GetDlgCtrlID, GetDlgItem, GetDlgItemTextW, GetParent, GetWindowLongPtrW, IDCANCEL, IDOK,
    LB_ADDSTRING, LB_DELETESTRING, LB_GETCOUNT, LB_GETCURSEL, LB_GETTEXT, LB_GETTEXTLEN,
    LB_RESETCONTENT, SW_SHOWNORMAL, SendDlgItemMessageW, SetDlgItemTextW, SetWindowLongPtrW,
    SetWindowTextW, WM_CHAR, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLOREDIT,
    WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_GETDLGCODE, WM_INITDIALOG, WM_KEYDOWN, WM_KEYUP,
    WM_KILLFOCUS, WM_SYSCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDPROC,
};
use windows::core::{Error as WinError, PCWSTR, PWSTR, w};

use crate::CONFIG_DIR_NAME;
use crate::layouts::{self, LayoutId, LayoutSpec};
use crate::theme::{self, ThemeSetting};

/// File name of the configuration inside the program's application data directory.
///
/// Section 7 of SPEC: `%APPDATA%\Lang_Switcher\config.toml`.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// The schema version this build writes and fully understands.
///
/// Version 2 arrived with FR-42а: the default of `[replacement] method` moved from
/// `backspace` to `auto`, and [`step_1_to_2`] carries the files of the old default over to
/// the new one.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

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

impl Language {
    /// The Windows language identifier of this locale — the number `app.rc` tags its string
    /// table with and the number [`text`] asks `FindResourceExW` for.
    ///
    /// `LANG_RUSSIAN` (0x19) and `LANG_ENGLISH` (0x09) with `SUBLANG_DEFAULT` (0x01) in the high
    /// four bits, which is how a `LANGID` is built. Spelled out rather than computed for the same
    /// reason `app.rc` spells them out: the two files have no shared header, and these two
    /// numbers are the joint between them.
    pub const fn langid(self) -> u16 {
        match self {
            Self::Ru => 0x0419,
            Self::En => 0x0409,
        }
    }

    /// The two-word name of section 7, for a message and for a test.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
        }
    }

    /// This locale as the number [`UI_LANGUAGE`] stores.
    const fn index(self) -> u32 {
        match self {
            Self::Ru => 0,
            Self::En => 1,
        }
    }

    /// The locale [`Self::index`] came from. Anything else is the default of section 7.
    const fn from_index(index: u32) -> Self {
        match index {
            1 => Self::En,
            _ => Self::Ru,
        }
    }
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

/// Text replacement method, `replacement.method` of section 7. Values `auto`, `backspace`
/// and `selection`.
///
/// A closed set rather than a free string: a value outside it must not pass silently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplacementMethod {
    /// Choose between the two real methods by the class of the foreground window at the
    /// moment of the replacement — **FR-42а**, and the default of section 7.
    ///
    /// The choice itself lives in `inject::effective_method`, next to the one
    /// `GetClassNameW` call it is made from; this module only names the value. Schema 2 made
    /// it the default: the measurement of task T-10-7 showed that neither pure method holds
    /// everywhere — `selection` breaks the COOKED input of consoles, `backspace` races the
    /// erasure in ordinary edit controls on repeated presses — so the default became the
    /// choice by window class, and the two words below remain as manual overrides.
    #[default]
    Auto,
    /// Erase the typed run with backspaces and retype it. The default of section 7 up to
    /// schema 1; an explicit manual override since schema 2.
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
    /// Theme of everything visible, `general.theme` of section 7 — **FR-92а**. Default
    /// [`ThemeSetting::System`].
    ///
    /// The vocabulary of the value — the three words and the rule that anything else reads
    /// as the default — belongs to module `theme`, which FR-92а names the single owner.
    /// This field only carries the setting through the file: the serde bridge below hands
    /// the string to [`ThemeSetting::from_config_str`] on the way in and writes
    /// [`ThemeSetting::as_config_str`] on the way out, so no theme word is ever spelled
    /// in this module. Note the asymmetry with the neighbours: `language` outside its set
    /// fails the read loudly, while an unknown theme word reads as `system` — that is not
    /// an inconsistency of this module but the letter of FR-92а, built into
    /// `from_config_str` itself.
    #[serde(
        default = "default_theme",
        serialize_with = "theme_to_toml",
        deserialize_with = "theme_from_toml"
    )]
    pub theme: ThemeSetting,
}

impl Default for General {
    fn default() -> Self {
        Self {
            enabled: true,
            autostart: true,
            language: Language::Ru,
            theme: ThemeSetting::System,
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
/// Both defaults of section 7 — `auto` and `0` — coincide with the defaults of the
/// field types, so the derive is the whole implementation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    /// How the typed run is replaced. Default `auto` — FR-42а.
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

/// Serde default for `general.theme` — the default FR-92а names.
///
/// A function rather than `#[serde(default)]` because [`ThemeSetting`] implements no
/// `Default`: module `theme` keeps the type bare, and giving it one from here would mean
/// editing the owner. The same shape as [`default_true`] and its neighbours.
fn default_theme() -> ThemeSetting {
    ThemeSetting::System
}

// Serde bridge for `general.theme`. [`ThemeSetting`] deliberately carries no serde
// derives: the three words of the value and the rule about every other word belong to
// `ThemeSetting::from_config_str` (FR-92а names module `theme` the single owner), and a
// derive here would mint a second spelling of them. The two functions below only carry
// the string between the file and the owner — neither knows a single theme word.

/// Reads `general.theme` from the file: the string goes to the owner of the vocabulary.
///
/// No error path for the *value*: by the construction of
/// [`ThemeSetting::from_config_str`], every string is an answer — an unknown word is the
/// default, which is how section 7 has garbage pass silently. What does fail is a value
/// that is not a string at all, and that failure is the ordinary type error of the
/// deserializer, same as for every other string field of the schema.
fn theme_from_toml<'de, D>(deserializer: D) -> Result<ThemeSetting, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    Ok(ThemeSetting::from_config_str(&text))
}

/// Writes `general.theme` to the file: the word is the owner's, verbatim.
fn theme_to_toml<S>(setting: &ThemeSetting, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(setting.as_config_str())
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
        if self.schema_version < 2 {
            step_1_to_2(self);
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

/// Raises a file from schema 1 to schema 2 — **FR-42а**, the new default of section 7.
///
/// Up to schema 1 the default of `[replacement] method` was `backspace`, and the file always
/// carried the field, because [`write_to`] writes the whole structure. So `backspace` in a
/// schema ≤ 1 file is what the absence of a choice looks like — the value the program put
/// there itself — and it is raised to the new default, `auto`.
///
/// `selection` is left exactly as it is: it was never a default of any schema, so a file
/// holding it holds a decision a person made (the compatibility mode of FR-42), and a
/// migration that overrode a decision would be damage, not maintenance.
///
/// A **current** file with `backspace` in it never reaches this rung — [`Config::migrate`]
/// runs it for versions below 2 only — which is what keeps `backspace` usable as an explicit
/// manual override from schema 2 onwards.
fn step_1_to_2(config: &mut Config) {
    if config.replacement.method == ReplacementMethod::Backspace {
        config.replacement.method = ReplacementMethod::Auto;
    }
    config.schema_version = 2;
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
// FR-94 — the interface strings, RU and EN
// =========================================================================================

/// Identifiers of the interface strings, mirrored by hand from `app.rc`.
///
/// ⚠ Same rule as the control identifiers below, and the same reason: a `.rc` file and a Rust
/// file share no header. A number that drifts is not silent — the string comes back empty, the
/// refusal is recorded through [`crate::app::report_non_critical`], and
/// `tests\settings.rs` reads both tables out of the **built** `LangSwitcher.exe` and compares
/// them, string by string, against literals written out there.
pub const IDS_DIALOG_CAPTION: u16 = 3000;
/// «Общие» — the first group of FR-92.
pub const IDS_GROUP_GENERAL: u16 = 3001;
/// The autostart tick of FR-93.
pub const IDS_AUTOSTART: u16 = 3002;
/// The label of the language combo box.
pub const IDS_LANGUAGE_LABEL: u16 = 3003;
/// The note that says the language takes effect after a restart — and it now does.
pub const IDS_LANGUAGE_RESTART: u16 = 3004;
/// «Горячая клавиша» — the second group of FR-92.
pub const IDS_GROUP_HOTKEY: u16 = 3005;
/// The label of the hotkey field.
pub const IDS_HOTKEY_LABEL: u16 = 3006;
/// The capture button while no capture is armed.
pub const IDS_HOTKEY_SET: u16 = 3007;
/// The capture button while a capture is armed — pressing it again cancels.
pub const IDS_HOTKEY_STOP: u16 = 3008;
/// «Раскладки» — the third group of FR-92.
pub const IDS_GROUP_LAYOUTS: u16 = 3009;
/// The «Пара» mode of FR-30.
pub const IDS_MODE_PAIR: u16 = 3010;
/// The «Несколько раскладок» mode of FR-31.
pub const IDS_MODE_CYCLE: u16 = 3011;
/// The label of the pair source combo box.
pub const IDS_PAIR_SOURCE: u16 = 3012;
/// The label of the pair target combo box.
pub const IDS_PAIR_TARGET: u16 = 3013;
/// The line explaining what the tick and the buttons of the cycle list do.
pub const IDS_CYCLE_HINT: u16 = 3014;
/// The «up» button of the cycle order.
pub const IDS_CYCLE_UP: u16 = 3015;
/// The «down» button of the cycle order.
pub const IDS_CYCLE_DOWN: u16 = 3016;
/// «Замена» — the fourth group of FR-92.
pub const IDS_GROUP_REPLACEMENT: u16 = 3017;
/// The `Backspace` method of FR-41.
pub const IDS_METHOD_BACKSPACE: u16 = 3018;
/// The selection method of FR-42.
pub const IDS_METHOD_SELECTION: u16 = 3019;
/// The label of the inter-event delay field of FR-44.
pub const IDS_DELAY_LABEL: u16 = 3020;
/// «Выделение» — the fifth group of FR-92.
pub const IDS_GROUP_SELECTION: u16 = 3021;
/// The tick of FR-65.
pub const IDS_SELECTION_ENABLED: u16 = 3022;
/// The label of the clipboard timeout field.
pub const IDS_CLIPBOARD_TIMEOUT: u16 = 3023;
/// The label of the clipboard restore delay field.
pub const IDS_CLIPBOARD_RESTORE: u16 = 3024;
/// «Исключения» — the sixth group of FR-92.
pub const IDS_GROUP_EXCLUSIONS: u16 = 3025;
/// The button that takes a process name out of the list of FR-84.
pub const IDS_EXCLUSION_REMOVE: u16 = 3026;
/// The button that puts one in.
pub const IDS_EXCLUSION_ADD: u16 = 3027;
/// The line saying what a process name looks like.
pub const IDS_EXCLUSION_HINT: u16 = 3028;
/// «Диагностика» — the seventh group of FR-92.
pub const IDS_GROUP_DIAGNOSTICS: u16 = 3029;
/// The journal tick of SEC-07.
pub const IDS_LOG_ENABLED: u16 = 3030;
/// The button that opens the journal folder.
pub const IDS_LOG_OPEN: u16 = 3031;
/// The label in front of the journal folder path.
pub const IDS_LOG_DIR_LABEL: u16 = 3032;
/// «Состояние» — the group of module readers, which is not from the FR-92 table.
pub const IDS_GROUP_STATE: u16 = 3033;
/// The `IDOK` button.
pub const IDS_OK: u16 = 3034;
/// The `IDCANCEL` button.
pub const IDS_CANCEL: u16 = 3035;
/// The «Применить» button.
pub const IDS_APPLY: u16 = 3036;
/// The warning FR-92 asks for when the hotkey is a text key.
pub const IDS_NOTE_TEXT_KEY: u16 = 3037;
/// The warning for a key name this build does not know.
pub const IDS_NOTE_UNKNOWN_KEY: u16 = 3038;
/// What the note says while a capture is armed.
pub const IDS_CAPTURE_PROMPT: u16 = 3039;
/// [`Refusal::Modifier`].
pub const IDS_CAPTURE_MODIFIER: u16 = 3040;
/// [`Refusal::Combination`].
pub const IDS_CAPTURE_COMBINATION: u16 = 3041;
/// [`Refusal::Emergency`].
pub const IDS_CAPTURE_EMERGENCY: u16 = 3042;
/// [`Refusal::Nameless`].
pub const IDS_CAPTURE_NAMELESS: u16 = 3043;
/// [`Refusal::Reserved`].
pub const IDS_CAPTURE_RESERVED: u16 = 3044;
/// The first line of the «Состояние» group, with three places to fill in.
pub const IDS_STATE_HOOK: u16 = 3045;
/// The hook is installed.
pub const IDS_HOOK_UP: u16 = 3046;
/// The hook is not installed — FR-80.
pub const IDS_HOOK_DOWN: u16 = 3047;
/// The second line of the «Состояние» group.
pub const IDS_STATE_LAYOUTS: u16 = 3048;
/// The third line of the «Состояние» group.
pub const IDS_STATE_AUTOSTART: u16 = 3049;
/// The registry value of FR-93 is there.
pub const IDS_AUTOSTART_PRESENT: u16 = 3050;
/// It is not.
pub const IDS_AUTOSTART_ABSENT: u16 = 3051;
/// The note about a layout named in the file that this session does not have.
pub const IDS_LAYOUT_NOTE: u16 = 3052;
/// The word «источник» inside that note.
pub const IDS_LAYOUT_WORD_SOURCE: u16 = 3053;
/// The word «цель» inside that note.
pub const IDS_LAYOUT_WORD_TARGET: u16 = 3054;
/// What the journal folder line says when `%APPDATA%` is not set.
pub const IDS_LOG_DIR_MISSING: u16 = 3055;
/// The automatic method of FR-42а — the third radio button of the «Замена» group, and the
/// recommended one. Appended to the block rather than renumbering the two methods above:
/// every identifier here is mirrored by hand in `app.rc`, and a renumbering would be a
/// silent mismatch waiting to happen.
pub const IDS_METHOD_AUTO: u16 = 3056;
/// The label of the appearance combo box — FR-92а, task T-11-3.
pub const IDS_THEME_LABEL: u16 = 3057;
/// The «Как в системе» item of the appearance combo box. The three items 3058–3060 are a
/// contract of order, not just of text: the dialog adds them to the combo in identifier
/// order, and [`theme_combo_index`] / [`theme_from_combo_index`] are the one place that
/// order is written down in Rust.
pub const IDS_THEME_SYSTEM: u16 = 3058;
/// The «Светлое» item.
pub const IDS_THEME_LIGHT: u16 = 3059;
/// The «Тёмное» item.
pub const IDS_THEME_DARK: u16 = 3060;
/// First item of FR-91 while the program is active.
pub const IDS_MENU_SUSPEND: u16 = 3072;
/// First item of FR-91 while the program is suspended.
pub const IDS_MENU_RESUME: u16 = 3073;
/// Second item of FR-91.
pub const IDS_MENU_SETTINGS: u16 = 3074;
/// Third item of FR-91, the one carrying the check mark of `general.autostart`.
pub const IDS_MENU_AUTOSTART: u16 = 3075;
/// Fourth item of FR-91.
pub const IDS_MENU_ABOUT: u16 = 3076;
/// Fifth item of FR-91.
pub const IDS_MENU_EXIT: u16 = 3077;

/// Every identifier above, so that a test can walk the whole vocabulary of the interface.
///
/// Exported rather than rebuilt in the test: what the test must not import is the *text*, and
/// it does not — it writes every string out itself. The list of identifiers is the contract
/// between `app.rc` and this file, and a test that walked a list of its own would not be
/// checking that contract at all.
pub const INTERFACE_STRINGS: [u16; 67] = [
    IDS_DIALOG_CAPTION,
    IDS_GROUP_GENERAL,
    IDS_AUTOSTART,
    IDS_LANGUAGE_LABEL,
    IDS_LANGUAGE_RESTART,
    IDS_GROUP_HOTKEY,
    IDS_HOTKEY_LABEL,
    IDS_HOTKEY_SET,
    IDS_HOTKEY_STOP,
    IDS_GROUP_LAYOUTS,
    IDS_MODE_PAIR,
    IDS_MODE_CYCLE,
    IDS_PAIR_SOURCE,
    IDS_PAIR_TARGET,
    IDS_CYCLE_HINT,
    IDS_CYCLE_UP,
    IDS_CYCLE_DOWN,
    IDS_GROUP_REPLACEMENT,
    IDS_METHOD_BACKSPACE,
    IDS_METHOD_SELECTION,
    IDS_DELAY_LABEL,
    IDS_GROUP_SELECTION,
    IDS_SELECTION_ENABLED,
    IDS_CLIPBOARD_TIMEOUT,
    IDS_CLIPBOARD_RESTORE,
    IDS_GROUP_EXCLUSIONS,
    IDS_EXCLUSION_REMOVE,
    IDS_EXCLUSION_ADD,
    IDS_EXCLUSION_HINT,
    IDS_GROUP_DIAGNOSTICS,
    IDS_LOG_ENABLED,
    IDS_LOG_OPEN,
    IDS_LOG_DIR_LABEL,
    IDS_GROUP_STATE,
    IDS_OK,
    IDS_CANCEL,
    IDS_APPLY,
    IDS_NOTE_TEXT_KEY,
    IDS_NOTE_UNKNOWN_KEY,
    IDS_CAPTURE_PROMPT,
    IDS_CAPTURE_MODIFIER,
    IDS_CAPTURE_COMBINATION,
    IDS_CAPTURE_EMERGENCY,
    IDS_CAPTURE_NAMELESS,
    IDS_CAPTURE_RESERVED,
    IDS_STATE_HOOK,
    IDS_HOOK_UP,
    IDS_HOOK_DOWN,
    IDS_STATE_LAYOUTS,
    IDS_STATE_AUTOSTART,
    IDS_AUTOSTART_PRESENT,
    IDS_AUTOSTART_ABSENT,
    IDS_LAYOUT_NOTE,
    IDS_LAYOUT_WORD_SOURCE,
    IDS_LAYOUT_WORD_TARGET,
    IDS_LOG_DIR_MISSING,
    IDS_METHOD_AUTO,
    IDS_THEME_LABEL,
    IDS_THEME_SYSTEM,
    IDS_THEME_LIGHT,
    IDS_THEME_DARK,
    IDS_MENU_SUSPEND,
    IDS_MENU_RESUME,
    IDS_MENU_SETTINGS,
    IDS_MENU_AUTOSTART,
    IDS_MENU_ABOUT,
    IDS_MENU_EXIT,
];

/// How many strings one string table resource holds — fixed by the format, not by us.
const STRINGS_PER_BLOCK: u16 = 16;

/// `RT_STRING`, the resource type of a string table.
///
/// Spelled out because the `windows` crate does not export it: `RT_DIALOG`, `RT_VERSION` and
/// the rest of the set are in `Win32::UI::WindowsAndMessaging`, and this one is missing from it.
/// The value is 6 and is part of the binary interface of the resource loader — the same reason
/// `app.rc` spells its constants out numerically.
const RT_STRING: PCWSTR = PCWSTR(std::ptr::without_provenance(6));

/// The image the string tables are read out of, or zero for «the running program's own».
///
/// An override exists for one reason, and it is not a preference: `embed-resource` links
/// `app.rc` into the **binary** targets of this crate only (barrier §4.4 of `STATE.md`), so a
/// test executable carries no resource section at all and would see every string as empty.
/// A test points this at the built `LangSwitcher.exe`, opened with `LOAD_LIBRARY_AS_DATAFILE`,
/// and then checks the strings the *shipping* binary carries rather than a copy of them.
static RESOURCE_MODULE: AtomicUsize = AtomicUsize::new(0);

/// The interface locale in force, as [`Language::index`] — `general.language` of section 7.
///
/// Published once, by `app`, at start-up, and not on every apply. That is what makes the note
/// beside the combo box true: changing the language writes the file and takes effect when the
/// program is started again, and nothing about the running program's language moves under the
/// user's hands in the meantime.
static UI_LANGUAGE: AtomicU32 = AtomicU32::new(0);

/// Publishes the interface locale — FR-94. Called by `app` at start-up.
pub fn set_ui_language(language: Language) {
    UI_LANGUAGE.store(language.index(), Ordering::Relaxed);
}

/// The interface locale in force.
pub fn ui_language() -> Language {
    Language::from_index(UI_LANGUAGE.load(Ordering::Relaxed))
}

/// Points the string loader at a module other than the running program — see
/// [`RESOURCE_MODULE`].
pub fn set_resource_module(module: HMODULE) {
    RESOURCE_MODULE.store(module.0 as usize, Ordering::Relaxed);
}

/// The module [`text`] reads from.
fn resource_module() -> HMODULE {
    let stored = RESOURCE_MODULE.load(Ordering::Relaxed);

    if stored != 0 {
        // A handle is not a pointer we dereference — it goes straight back to Win32 — which is
        // exactly what `without_provenance_mut` says.
        return HMODULE(std::ptr::without_provenance_mut(stored));
    }

    // SAFETY: a null name asks for the module handle of the running program's own image, which
    // is the documented meaning of the argument and needs no memory of ours. The handle is
    // borrowed, not owned: `GetModuleHandleW` does not add a reference and there is nothing to
    // free.
    match unsafe { GetModuleHandleW(PCWSTR::null()) } {
        Ok(module) => module,
        Err(error) => {
            crate::app::report_non_critical("GetModuleHandleW", &error);
            HMODULE::default()
        }
    }
}

/// One interface string in the locale in force — FR-94.
///
/// An empty string when the resource cannot be read, which is the one answer that cannot make
/// things worse: a label that came out blank is visible, and the refusal is in the journal.
pub fn text(id: u16) -> String {
    string_from(resource_module(), ui_language(), id).unwrap_or_default()
}

/// [`text`] with `{0}`, `{1}`… replaced by `arguments`, in order.
///
/// A placeholder and not `format!`, because the sentence is a resource and `format!` needs a
/// literal. **SEC-01, SEC-07:** everything substituted here is a number this program counted, a
/// path, or a word out of the same string table — never a keystroke, a character or anything
/// that came off the clipboard.
pub fn format_text(id: u16, arguments: &[&str]) -> String {
    let mut filled = text(id);

    for (index, argument) in arguments.iter().enumerate() {
        filled = filled.replace(&format!("{{{index}}}"), argument);
    }

    filled
}

/// One string of one string table of one module, decoded by hand.
///
/// The string table format, which is what the walk below follows: strings live sixteen to a
/// resource, the resource identifier is `id / 16 + 1`, and the block is sixteen strings each
/// stored as a `u16` length followed by that many UTF-16 units — counted, never
/// NUL-terminated. `LoadStringW` would do the same walk, but it would choose the *language*
/// itself, from the user's Windows UI language; section 7 says the choice is
/// `general.language`, so the language is passed to `FindResourceExW` explicitly and this
/// function does the rest.
fn string_from(module: HMODULE, language: Language, id: u16) -> Option<String> {
    let block = id / STRINGS_PER_BLOCK + 1;
    let wanted = usize::from(id % STRINGS_PER_BLOCK);

    // SAFETY: `module` is a live module handle — either the running image or one a test loaded
    // with `LOAD_LIBRARY_AS_DATAFILE` and keeps alive. Both "strings" are integer identifiers in
    // the `MAKEINTRESOURCE` form, a value below 65536 carried inside the pointer, so nothing is
    // dereferenced as a string. The call reads no memory of ours.
    let found = unsafe {
        FindResourceExW(
            Some(module),
            RT_STRING,
            resource_id(block),
            language.langid(),
        )
    };

    if found.0.is_null() {
        crate::app::report_non_critical("FindResourceExW", &WinError::from_thread());
        return None;
    }

    // SAFETY: `module` and `found` are the pair just established.
    let size = unsafe { SizeofResource(Some(module), found) };

    // SAFETY: the same pair. NFR-13: the crate turns a failure into an error, which the `?`
    // below examines.
    let block = unsafe { LoadResource(Some(module), found) }
        .inspect_err(|error| crate::app::report_non_critical("LoadResource", error))
        .ok()?;

    // SAFETY: `block` came from the `LoadResource` directly above.
    let start = unsafe { LockResource(block) };

    if start.is_null() || size < u32::try_from(size_of::<u16>()).unwrap_or(u32::MAX) {
        crate::app::report_non_critical("LoadResource", &WinError::from_thread());
        return None;
    }

    let units = usize::try_from(size).ok()? / size_of::<u16>();

    // SAFETY: `start` points at `size` bytes of read-only resource data inside the mapping of
    // `module`, which is alive for the whole of this call; resource data is aligned to four
    // bytes, so reading it as `u16` is aligned. The slice does not outlive the call and every
    // byte taken out of it is copied into the `String` below.
    let data = unsafe { std::slice::from_raw_parts(start.cast::<u16>(), units) };

    let mut at = 0usize;

    for slot in 0..usize::from(STRINGS_PER_BLOCK) {
        let length = usize::from(*data.get(at)?);
        at += 1;

        if slot == wanted {
            // Not `from_utf16_lossy`: a replacement character would hide exactly the damage a
            // missing `#pragma code_page(65001)` produces.
            return String::from_utf16(data.get(at..at.checked_add(length)?)?).ok();
        }

        at = at.checked_add(length)?;
    }

    None
}

// =========================================================================================
// FR-94 — capturing the hotkey by pressing it
// =========================================================================================

/// The modifiers held at the moment a key went down.
///
/// A value rather than a query, so that the decision below is a function of its arguments and
/// can be checked without a keyboard.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    /// Either `Ctrl`.
    pub ctrl: bool,
    /// Either `Alt`.
    pub alt: bool,
    /// Either `Shift`.
    pub shift: bool,
    /// Either `Windows` key.
    pub win: bool,
}

impl Modifiers {
    /// Whether anything at all is held.
    pub const fn any(self) -> bool {
        self.ctrl || self.alt || self.shift || self.win
    }

    /// The three modifiers of FR-96, read exactly as `hook::emergency_modifiers_held` reads
    /// them: all three down, and the `Windows` key is not part of the question.
    pub const fn emergency(self) -> bool {
        self.ctrl && self.alt && self.shift
    }

    /// What is held right now, from the message queue.
    ///
    /// `GetKeyState` and not `GetAsyncKeyState`: the answer wanted is the state as of the
    /// keystroke being handled, which is what the queue-synchronised call gives, and which is
    /// what the user actually pressed together.
    fn held_now() -> Self {
        Self {
            ctrl: key_is_down(VK_CONTROL),
            alt: key_is_down(VK_MENU),
            shift: key_is_down(VK_SHIFT),
            win: key_is_down(VK_LWIN) || key_is_down(VK_RWIN),
        }
    }
}

/// Whether one virtual key is down, as the message queue sees it.
fn key_is_down(key: VIRTUAL_KEY) -> bool {
    // SAFETY: takes a virtual-key code by value and touches no memory of ours. The high bit of
    // the `i16` it answers is "down", which for a signed value is "negative".
    unsafe { GetKeyState(i32::from(key.0)) < 0 }
}

/// Why a press cannot become the hotkey — FR-94, FR-95, FR-96.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A modifier on its own. `Ctrl` is not a hotkey, it is half of one.
    Modifier,
    /// A key with a modifier held. **Section 7 stores one key name and the callback compares
    /// one code** (`hook::classify`, `key.vk != mode.hotkey_vk`), so `Ctrl+K` could only be
    /// written down as `K` — and would then fire on a bare `K`. Refused rather than stored as
    /// something else than what the user pressed. Widening it needs both a schema field and a
    /// change to the callback, and this task is allowed neither.
    Combination,
    /// `Ctrl+Alt+Shift+F12`, the emergency exit of FR-96. Never assignable, in either build.
    Emergency,
    /// A key the system owns and an application never gets as a plain key.
    Reserved,
    /// A key section 7 has no name for, so it could not be written to the file and read back.
    Nameless,
}

impl Refusal {
    /// The interface string that says this to the user.
    pub const fn string_id(self) -> u16 {
        match self {
            Self::Modifier => IDS_CAPTURE_MODIFIER,
            Self::Combination => IDS_CAPTURE_COMBINATION,
            Self::Emergency => IDS_CAPTURE_EMERGENCY,
            Self::Reserved => IDS_CAPTURE_RESERVED,
            Self::Nameless => IDS_CAPTURE_NAMELESS,
        }
    }
}

/// What one press during a capture amounts to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capture {
    /// The press is the hotkey. The value is the name section 7 stores, and
    /// `hook::vk_from_name` reads it back to the same code it came from.
    Taken(String),
    /// The press cannot be the hotkey. The capture stays armed and the reason is shown.
    Refused(Refusal),
}

/// The keys an application never receives as a plain key, because the shell takes them first.
///
/// Only the two `Windows` keys are here, and they are here because the shell acts on their
/// *release* whatever anybody else does with the press. Everything else the system reserves —
/// `Ctrl+Alt+Del`, `Win+L` — is taken below the level any window can see, so it never arrives
/// at the capture at all: nothing to refuse, and the dialog simply goes on waiting.
const SYSTEM_RESERVED: &[u16] = &[VK_LWIN.0, VK_RWIN.0];

/// The three modifiers, in all their forms — the neutral code the queue reports and the two
/// sided ones a keyboard can send.
const MODIFIER_KEYS: &[u16] = &[
    VK_CONTROL.0,
    VK_LCONTROL.0,
    VK_RCONTROL.0,
    VK_MENU.0,
    VK_LMENU.0,
    VK_RMENU.0,
    VK_SHIFT.0,
    VK_LSHIFT.0,
    VK_RSHIFT.0,
];

/// The whole capture decision, as a function of the key and of what was held with it.
///
/// The order of the tests is not free:
///
/// * **FR-96 first**, exactly as `hook::keyboard_hook_proc` puts it first, and for the same
///   reason: the emergency combination is not a candidate hotkey under any circumstance. In
///   practice the callback has already ended the process by the time this could run — this is
///   the answer for the case where the hook is not installed at all;
/// * then the `Windows` key, which is neither a modifier of this program's vocabulary nor a
///   key an application ever gets;
/// * then a modifier on its own, which is a press the user has not finished making;
/// * then anything held with a modifier, for the reason [`Refusal::Combination`] gives;
/// * and last the vocabulary of section 7, because a name that cannot be written to the file
///   is a hotkey that would not survive a restart.
///
/// **SEC-01, SEC-07.** The value that comes out is a *name of a key* — `F8`, `Pause`, `A` —
/// built from the virtual-key code and from nothing else. No character is produced, nothing is
/// translated through a keyboard layout, and nothing here reaches the journal: the caller shows
/// the name in the field and writes it to `[hotkey] key`, which is where section 7 keeps it
/// anyway.
pub fn capture(vk: u16, modifiers: Modifiers) -> Capture {
    if vk == crate::hook::EMERGENCY_VK && modifiers.emergency() {
        return Capture::Refused(Refusal::Emergency);
    }

    if SYSTEM_RESERVED.contains(&vk) {
        return Capture::Refused(Refusal::Reserved);
    }

    if MODIFIER_KEYS.contains(&vk) {
        return Capture::Refused(Refusal::Modifier);
    }

    if modifiers.any() {
        return Capture::Refused(Refusal::Combination);
    }

    match key_name(vk) {
        Some(name) => Capture::Taken(name),
        None => Capture::Refused(Refusal::Nameless),
    }
}

/// The named keys of section 7, in the spelling this program *writes*.
///
/// ⚠ The mirror of `hook::NAMED_KEYS`, narrowed to one spelling per code: that table accepts
/// `Break`, `PgUp` and `Esc` because a person writes them by hand, and this one has to choose
/// which of them a capture puts in the file. Kept honest by a test that runs every code this
/// function names back through `hook::vk_from_name` and demands the same code out — the reverse
/// direction of the one function that reads the file.
const NAMED_KEYS: &[(&str, VIRTUAL_KEY)] = &[
    ("Pause", VK_PAUSE),
    ("Escape", VK_ESCAPE),
    ("Insert", VK_INSERT),
    ("Delete", VK_DELETE),
    ("Home", VK_HOME),
    ("End", VK_END),
    ("PageUp", VK_PRIOR),
    ("PageDown", VK_NEXT),
    ("ScrollLock", VK_SCROLL),
    ("NumLock", VK_NUMLOCK),
    ("CapsLock", VK_CAPITAL),
    ("PrintScreen", VK_SNAPSHOT),
    ("Apps", VK_APPS),
];

/// Highest function key Windows has a virtual-key code for — `hook::HIGHEST_FUNCTION_KEY`.
const HIGHEST_FUNCTION_KEY: u16 = 24;

/// The name section 7 stores for a virtual-key code, or `None` when it has none.
///
/// The inverse of `hook::vk_from_name`, and it has to be written here rather than there:
/// `src\hook.rs` is the accepted code of the hot path and this task does not touch it. What
/// keeps the two from drifting is not discipline but a test — see [`NAMED_KEYS`].
pub fn key_name(vk: u16) -> Option<String> {
    // A single ASCII letter or digit is its own virtual-key code, which is the documented
    // property `hook::vk_from_name` reads in the other direction.
    if let Ok(byte) = u8::try_from(vk)
        && (byte.is_ascii_uppercase() || byte.is_ascii_digit())
    {
        return Some(char::from(byte).to_string());
    }

    if (VK_F1.0..VK_F1.0 + HIGHEST_FUNCTION_KEY).contains(&vk) {
        return Some(format!("F{}", vk - VK_F1.0 + 1));
    }

    NAMED_KEYS
        .iter()
        .find(|(_, code)| code.0 == vk)
        .map(|(name, _)| (*name).to_owned())
}

/// The virtual-key code a capture publishes while it is armed.
///
/// Zero is not a key: `KBDLLHOOKSTRUCT::vkCode` is documented to be in 1..=254, so no stroke can
/// ever equal it and `hook::classify` takes the "ordinary key" branch for every press. That is
/// exactly the state a capture needs — see [`CaptureSession`].
const NO_HOTKEY_VK: u16 = 0;

/// A capture in progress — and, for as long as it lives, the hotkey of FR-02 suspended.
///
/// # Why the ordinary work has to stop
///
/// The keys the user presses to *choose* a hotkey travel through the same global low-level hook
/// as every other key on the machine. Two things would otherwise happen at once: pressing the
/// current hotkey would run a conversion nobody asked for (FR-02), and it would be **suppressed
/// by FR-95 before reaching this window**, so the one key the user is most likely to press
/// first could never be captured at all.
///
/// # What is suspended, and why by two different switches
///
/// **The one that matters is the hotkey code.** Arming publishes [`NO_HOTKEY_VK`] through
/// [`crate::hook::set_hotkey_vk`] — the interface this task is given for the hotkey — and
/// dropping publishes back exactly what stood there before. No press can equal a code no
/// keyboard produces, so nothing is converted and nothing is suppressed for as long as the
/// capture is armed.
///
/// `hook::set_active(false)` is published as well, which additionally stops the stroke reaching
/// the typing buffer, and it is deliberately **not** what the suspension rests on. ⚠ Measured
/// during this task: `app::window_proc` re-publishes `tray.enabled()` into
/// `hook::set_active` **after every message the UI thread sees**, so the active flag can be set
/// back to `true` under a capture by nothing more than the tray icon being hovered over. The
/// hotkey code is published from `app::publish_configuration` alone, which runs on «Применить»
/// and at start-up and not from any message, so it stays where a capture puts it.
///
/// **FR-96 is not affected and cannot be.** The emergency combination is handled at the top of
/// the callback, before either of these values is so much as read — see
/// `hook::keyboard_hook_proc` — so `Ctrl+Alt+Shift+F12` ends the process during a capture
/// exactly as it does at any other moment. That is the condition the decision on question 18
/// puts on running a debug build at all, and nothing in this file weakens it.
///
/// The restoration is in `Drop` and not in a method on purpose: the program stays up after a
/// panic (FR-98, FR-99), and a capture that swallowed a panic would leave the program with no
/// hotkey at all and no way back short of a restart.
pub struct CaptureSession {
    /// `[hotkey] key` as it stood when the capture was armed — what «отмена» puts back.
    previous_key: String,
    /// The code the callback was comparing against when the capture was armed.
    published_vk: u16,
    /// `hook::is_active()` as it stood then.
    hook_was_active: bool,
}

impl CaptureSession {
    /// Arms a capture and suspends the hotkey for its duration.
    pub fn arm(previous_key: String) -> Self {
        let published_vk = crate::hook::hotkey_vk();
        let hook_was_active = crate::hook::is_active();

        crate::hook::set_hotkey_vk(NO_HOTKEY_VK);
        crate::hook::set_active(false);

        Self {
            previous_key,
            published_vk,
            hook_was_active,
        }
    }

    /// The key name to go back to if the capture is cancelled.
    pub fn previous_key(&self) -> &str {
        &self.previous_key
    }

    /// The hotkey code the callback will be given back when this capture ends.
    pub fn published_vk(&self) -> u16 {
        self.published_vk
    }

    /// Whether the callback was doing its ordinary work when this capture was armed.
    pub fn hook_was_active(&self) -> bool {
        self.hook_was_active
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        crate::hook::set_hotkey_vk(self.published_vk);
        crate::hook::set_active(self.hook_was_active);
    }
}

/// Resource identifier of the dialog template in `app.rc`.
///
/// ⚠ Kept equal to `IDD_SETTINGS` there by hand, because a `.rc` file and a Rust file have no
/// shared header. A mismatch is not silent: `DialogBoxParamW` fails outright and the failure is
/// reported, which is the reason the number is far away from the icon identifiers.
pub const IDD_SETTINGS: u16 = 200;

// Control identifiers, mirrored from `app.rc`. Same rule as above.
const IDC_AUTOSTART: i32 = 1001;
const IDC_LANGUAGE: i32 = 1002;
const IDC_THEME: i32 = 1003;
const IDC_HOTKEY: i32 = 1010;
const IDC_HOTKEY_NOTE: i32 = 1011;
const IDC_HOTKEY_CAPTURE: i32 = 1012;
const IDC_MODE_PAIR: i32 = 1020;
const IDC_MODE_CYCLE: i32 = 1021;
const IDC_PAIR_SOURCE: i32 = 1022;
const IDC_PAIR_TARGET: i32 = 1023;
const IDC_CYCLE_LIST: i32 = 1024;
const IDC_CYCLE_UP: i32 = 1025;
const IDC_CYCLE_DOWN: i32 = 1026;
const IDC_LAYOUT_NOTE: i32 = 1027;
// 1029 and not 1033: `CheckRadioButton` walks the identifier range it is given, so the three
// method radios have to be contiguous — and 1032 is already the delay field. `auto` sits in
// front of `backspace` because it is the default of section 7 and the first thing the group
// shows, and the free number in front happened to be there. Mirrored in `app.rc` by hand.
const IDC_METHOD_AUTO: i32 = 1029;
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

// The static text of the dialog — group boxes and labels. They carried -1 until FR-94 needed
// to replace their text, and a control identified by -1 is a control `GetDlgItem` cannot find.
const IDC_GROUP_GENERAL: i32 = 1090;
const IDC_LANGUAGE_LABEL: i32 = 1091;
const IDC_LANGUAGE_RESTART: i32 = 1092;
const IDC_GROUP_HOTKEY: i32 = 1093;
const IDC_HOTKEY_LABEL: i32 = 1094;
const IDC_GROUP_LAYOUTS: i32 = 1095;
const IDC_PAIR_SOURCE_LABEL: i32 = 1096;
const IDC_PAIR_TARGET_LABEL: i32 = 1097;
const IDC_CYCLE_HINT: i32 = 1098;
const IDC_GROUP_REPLACEMENT: i32 = 1099;
const IDC_DELAY_LABEL: i32 = 1100;
const IDC_GROUP_SELECTION: i32 = 1101;
const IDC_CLIP_TIMEOUT_LABEL: i32 = 1102;
const IDC_CLIP_RESTORE_LABEL: i32 = 1103;
const IDC_GROUP_EXCLUSIONS: i32 = 1104;
const IDC_EXCLUSION_HINT: i32 = 1105;
const IDC_GROUP_DIAGNOSTICS: i32 = 1106;
const IDC_LOG_DIR_LABEL: i32 = 1107;
const IDC_GROUP_STATE: i32 = 1108;
const IDC_THEME_LABEL: i32 = 1109;

/// The combo box index of one theme setting — FR-92а, task T-11-3.
///
/// The appearance combo of the dialog is filled with the three strings [`IDS_THEME_SYSTEM`],
/// [`IDS_THEME_LIGHT`] and [`IDS_THEME_DARK`], in that order; this pair of functions is the
/// one place that order is written down. No string of the configuration vocabulary appears
/// here — the words `[general].theme` takes belong to `theme::ThemeSetting` alone (§6.2),
/// and what the dialog trades in is only the position of an item in a list.
///
/// Public so that the test which pins the order of the items to the order of the values can
/// call the very functions the dialog calls, rather than a copy of the mapping.
pub fn theme_combo_index(setting: ThemeSetting) -> usize {
    match setting {
        ThemeSetting::System => 0,
        ThemeSetting::Light => 1,
        ThemeSetting::Dark => 2,
    }
}

/// The theme setting a combo box index names — the inverse of [`theme_combo_index`].
///
/// Takes the answer of `CB_GETCURSEL` as it comes. The user cannot empty a
/// `CBS_DROPDOWNLIST`, so anything outside the three items — `CB_ERR` included — is not a
/// state the dialog can reach; it reads as the default of FR-92а rather than as a panic,
/// the same way `read_dialog` treats the language combo.
pub fn theme_from_combo_index(index: isize) -> ThemeSetting {
    match index {
        1 => ThemeSetting::Light,
        2 => ThemeSetting::Dark,
        _ => ThemeSetting::System,
    }
}

/// Every control of the dialog whose text is a fixed string of the interface, and the string
/// that belongs in it — FR-94.
///
/// A table and not thirty-eight calls, because this is exactly the list a reader wants to see
/// in one place and exactly the list a test has to walk: `tests\settings.rs` checks that every
/// control named here exists in the template of the built binary and that every string named
/// here exists in **both** tables of it.
///
/// The caption of the window is not in the table — it is not a control — and neither are the
/// two entries of the language combo box: a language is named in its own language in a language
/// chooser, so «Русский» and «English» stand as they are in both locales.
const LOCALISED_CONTROLS: &[(i32, u16)] = &[
    (IDC_GROUP_GENERAL, IDS_GROUP_GENERAL),
    (IDC_AUTOSTART, IDS_AUTOSTART),
    (IDC_LANGUAGE_LABEL, IDS_LANGUAGE_LABEL),
    (IDC_THEME_LABEL, IDS_THEME_LABEL),
    (IDC_LANGUAGE_RESTART, IDS_LANGUAGE_RESTART),
    (IDC_GROUP_HOTKEY, IDS_GROUP_HOTKEY),
    (IDC_HOTKEY_LABEL, IDS_HOTKEY_LABEL),
    (IDC_HOTKEY_CAPTURE, IDS_HOTKEY_SET),
    (IDC_GROUP_LAYOUTS, IDS_GROUP_LAYOUTS),
    (IDC_MODE_PAIR, IDS_MODE_PAIR),
    (IDC_MODE_CYCLE, IDS_MODE_CYCLE),
    (IDC_PAIR_SOURCE_LABEL, IDS_PAIR_SOURCE),
    (IDC_PAIR_TARGET_LABEL, IDS_PAIR_TARGET),
    (IDC_CYCLE_HINT, IDS_CYCLE_HINT),
    (IDC_CYCLE_UP, IDS_CYCLE_UP),
    (IDC_CYCLE_DOWN, IDS_CYCLE_DOWN),
    (IDC_GROUP_REPLACEMENT, IDS_GROUP_REPLACEMENT),
    (IDC_METHOD_AUTO, IDS_METHOD_AUTO),
    (IDC_METHOD_BACKSPACE, IDS_METHOD_BACKSPACE),
    (IDC_METHOD_SELECTION, IDS_METHOD_SELECTION),
    (IDC_DELAY_LABEL, IDS_DELAY_LABEL),
    (IDC_GROUP_SELECTION, IDS_GROUP_SELECTION),
    (IDC_SELECTION_ENABLED, IDS_SELECTION_ENABLED),
    (IDC_CLIP_TIMEOUT_LABEL, IDS_CLIPBOARD_TIMEOUT),
    (IDC_CLIP_RESTORE_LABEL, IDS_CLIPBOARD_RESTORE),
    (IDC_GROUP_EXCLUSIONS, IDS_GROUP_EXCLUSIONS),
    (IDC_EXCLUSION_REMOVE, IDS_EXCLUSION_REMOVE),
    (IDC_EXCLUSION_ADD, IDS_EXCLUSION_ADD),
    (IDC_EXCLUSION_HINT, IDS_EXCLUSION_HINT),
    (IDC_GROUP_DIAGNOSTICS, IDS_GROUP_DIAGNOSTICS),
    (IDC_LOG_ENABLED, IDS_LOG_ENABLED),
    (IDC_LOG_OPEN, IDS_LOG_OPEN),
    (IDC_LOG_DIR_LABEL, IDS_LOG_DIR_LABEL),
    (IDC_GROUP_STATE, IDS_GROUP_STATE),
    (OK_COMMAND, IDS_OK),
    (CANCEL_COMMAND, IDS_CANCEL),
    (IDC_APPLY, IDS_APPLY),
];

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
///
/// The answer is the *identifier* of an interface string and not the string itself — FR-94. The
/// decision of what to say is this function's; which language to say it in belongs to
/// [`text`], and the two are separated so that a test can check the decision without a resource
/// and check the resource without the decision.
pub fn hotkey_note(key: &str) -> Option<u16> {
    match crate::hook::vk_from_name(key) {
        None => Some(IDS_NOTE_UNKNOWN_KEY),
        Some(vk) if is_text_key(vk) => Some(IDS_NOTE_TEXT_KEY),
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

    // FR-92а, task T-11-4: the palette of this dialog, resolved once at initialisation —
    // the setting comes from the configuration the dialog was opened with, and the system
    // switch is read exactly once, so the first paint is one consistent palette. Who
    // re-resolves and when is written down in `apply_now` (a pressed «Применить») and in
    // task T-11-9 (`WM_SETTINGCHANGE`), not here.
    let palette = theme::resolve(config.general.theme, theme::system_is_light());

    let state = RefCell::new(DialogState {
        working: config.clone(),
        rows: layout_rows(&config.layouts, &session),
        session,
        apply,
        capture: None,
        palette,
        // `None` — some `CreateSolidBrush` refused — is survived, not escalated: the dialog
        // opens and works with the system colours, because not painting is better than not
        // opening (NFR-13). Nothing is journaled on that path: `Brushes::new` documents
        // why, and the vocabulary of `diag` is closed (reviews\T-11-1.md).
        brushes: theme::Brushes::new(palette),
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
    /// The capture of FR-94, while one is armed. `Some` is the whole of "armed": the value
    /// suspends the conversion path for as long as it lives and restores it when it is dropped.
    capture: Option<CaptureSession>,
    /// The palette every colour answer of this dialog is chosen from — FR-92а, task T-11-4.
    ///
    /// Resolved once when the dialog is created — the setting from the configuration, the
    /// system switch read exactly once — and replaced in exactly one place: a pressed
    /// «Применить» that resolves to the other palette (see [`apply_now`]). `&'static`
    /// because `theme::resolve` answers identity, not a copy, and identity is what
    /// [`title_bar_is_dark`] and the change test in [`apply_now`] compare by.
    palette: &'static theme::Palette,
    /// The brushes of `palette`, owned for as long as the window can be asked to paint.
    ///
    /// `None` when [`theme::Brushes::new`] was refused: every `WM_CTLCOLOR*` then answers
    /// «not handled» and the dialog lives on with the system colours — not painting is
    /// better than not opening (NFR-13). On a palette change the set is recreated whole,
    /// never mutated in place — see the type's own documentation for why.
    brushes: Option<theme::Brushes>,
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
            unsafe {
                with_state(hwnd, |state| {
                    fill_dialog(hwnd, state);

                    // FR-92а, task T-11-4: the non-client title bar follows the resolved
                    // palette from the first showing. The client area needs no call — the
                    // `WM_CTLCOLOR*` answers below paint it as soon as anything paints.
                    apply_title_bar_theme(hwnd, state.palette);
                })
            };

            // TRUE: let the dialog manager choose the focus.
            1
        }

        // FR-92а, task T-11-4: the five colour questions the dialog manager asks while the
        // window and its controls are painted. Answered with the brushes of the resolved
        // palette; zero — the system colours — when there is nothing to answer with.
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX
        | WM_CTLCOLORBTN => {
            // SAFETY: as for `WM_COMMAND` below — the pointer was stored on `WM_INITDIALOG`
            // and the value it names is alive for the whole of this modal call.
            unsafe { on_ctl_color(hwnd, message, wparam, lparam) }
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

/// The colour role a control plays when `WM_CTLCOLORSTATIC` asks about it — FR-92а,
/// task T-11-4.
///
/// Three roles and not sixteen colours: the handler below turns a role into palette fields
/// in one place, and the mapping «identifier → role» stays a pure function a table test can
/// close (criterion 10 of T-11-4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticColorRole {
    /// An ordinary caption — group boxes, labels, the text of checkboxes and radios, the
    /// state lines: `text` over the transparent window background.
    Label,
    /// An explanatory note beside the main text: `text_muted` over the transparent window
    /// background.
    Muted,
    /// A control that is a field to the eye even though the message files it under static:
    /// `text` over `field_bg`, exactly what `WM_CTLCOLOREDIT` would have painted.
    Field,
}

/// The role of one control identifier — the single place the list is written down.
///
/// The muted list is the explanatory notes and hints of the dialog, by their actual
/// identifiers; everything else the message asks about is an ordinary caption. The tasks
/// that follow — T-11-5a/b/c and later — are to reuse this mapping rather than write a
/// second list, which is why it sits here beside the handler and not inside it.
///
/// ⚠ `IDC_HOTKEY` is an `EDITTEXT` with `ES_READONLY`, and a read-only edit is asked about
/// with `WM_CTLCOLORSTATIC`, not `WM_CTLCOLOREDIT` — so it appears in this mapping, and it
/// is painted as the field it looks like, not as a caption.
///
/// Public, like the theme-combo pair above: the table test of T-11-4 calls the very
/// function the handler calls, not a copy of the list.
pub fn static_color_role(control: i32) -> StaticColorRole {
    match control {
        IDC_HOTKEY => StaticColorRole::Field,
        IDC_HOTKEY_NOTE | IDC_LANGUAGE_RESTART | IDC_CYCLE_HINT | IDC_EXCLUSION_HINT
        | IDC_LAYOUT_NOTE | IDC_LOG_DIR => StaticColorRole::Muted,
        _ => StaticColorRole::Label,
    }
}

/// Whether the non-client title bar is to be dark for this palette — FR-92а, task T-11-4.
///
/// `theme::resolve` answers one of two `&'static` palettes, so identity with
/// [`theme::GRAPHITE`] *is* «разрешённая палитра тёмная» — no colour arithmetic, no third
/// opinion, and the acceptance instrument of position 25 compares against the same
/// constants. Public for the same reason as [`static_color_role`]: the test of criterion 11
/// calls the function the dialog calls.
pub fn title_bar_is_dark(palette: &theme::Palette) -> bool {
    std::ptr::eq(palette, &theme::GRAPHITE)
}

/// Asks DWM to colour the non-client title bar after the palette — FR-92а, task T-11-4.
///
/// `DWMWA_USE_IMMERSIVE_DARK_MODE` is the one documented way to a dark caption, named by
/// FR-92а in so many words; the attribute is four bytes of `BOOL`, true exactly when the
/// resolved palette is the dark one.
///
/// A refusal is survived and left alone on purpose (NFR-13: the result is examined right
/// here and deliberately dropped). The attribute took its public number only in Windows 10
/// 20H1, and on older builds the call answers an error for a perfectly live window — a
/// legal state of the machine, not a violation of ownership, which is why there is no
/// `debug_assert` on this path. It is not journaled either: the closed `OPERATIONS`
/// vocabulary of `diag` has no row for it (decision of `reviews\T-11-1.md`), and the only
/// consequence is a system-coloured title bar over a correctly painted client area.
fn apply_title_bar_theme(hwnd: HWND, palette: &theme::Palette) {
    let dark = windows::core::BOOL::from(title_bar_is_dark(palette));

    // SAFETY: `hwnd` is the live dialog. The attribute pointer names `dark`, a live local
    // of this frame, and the size passed is exactly its four bytes; the call copies the
    // value and keeps no pointer once it returns.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&raw const dark).cast(),
            size_of::<windows::core::BOOL>() as u32,
        )
    };
}

/// Repaints the dialog and every child in it — the visible half of a palette change.
fn repaint_after_palette_change(hwnd: HWND) {
    // NFR-13: both answers are examined in words and deliberately dropped. Either call
    // refuses only for a window that is not alive, and `hwnd` is the dialog whose
    // procedure is running; there is nothing to do about a refused invalidation beyond the
    // next `WM_PAINT` arriving anyway, and the journal has no row for it
    // (reviews\T-11-1.md).
    //
    // SAFETY: `hwnd` is the live dialog; a null rectangle means the whole client area, and
    // neither call keeps a pointer.
    let _ = unsafe { InvalidateRect(Some(hwnd), None, true) };

    // `RDW_ALLCHILDREN` is the half `InvalidateRect` does not reach: the controls repaint
    // too, so the person who pressed «Применить» sees one whole dialog in the new palette,
    // not a new background behind stale controls.
    //
    // SAFETY: as above.
    let _ = unsafe {
        RedrawWindow(
            Some(hwnd),
            None,
            None,
            RDW_ERASE | RDW_INVALIDATE | RDW_ALLCHILDREN,
        )
    };
}

/// The answer to one `WM_CTLCOLOR*` question — a colour choice and a brush, nothing else.
///
/// # SEC-05, last sentence, held to the letter
///
/// The whole of what this function takes out of the message is two handles: `wparam` — the
/// DC the control is about to be painted with — and `lparam` — the window handle of the
/// control, which is only asked for its identifier with `GetDlgCtrlID`. Nothing of the
/// message is dereferenced as memory; the DC is only written to (`SetTextColor`,
/// `SetBkColor`, `SetBkMode`), never read from; no privileged action starts here. A forged
/// message from a process of the same integrity level therefore buys its sender a colour
/// lookup and nothing else.
///
/// Zero — «not handled», the system colours — is the answer whenever the state is not
/// reachable (no state yet, or a re-entrant message) or [`theme::Brushes::new`] was refused
/// at initialisation: not painting is better than not opening (NFR-13).
///
/// # Safety
///
/// Called from [`dialog_proc`] only, with the arguments of the message.
unsafe fn on_ctl_color(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    // The two handles of the message — see above; this is all that is ever read out of it.
    let dc = HDC(wparam.0 as *mut std::ffi::c_void);
    let control = HWND(lparam.0 as *mut std::ffi::c_void);

    // SAFETY: `control` is a window handle out of the message; asking for its identifier
    // reads a field of that window and no memory of ours. For `WM_CTLCOLORDLG` the handle
    // is the dialog itself and the identifier goes unused.
    let control_id = unsafe { GetDlgCtrlID(control) };

    // The colour choice, split from the painting: the borrow of the state ends before the
    // DC is touched, and what leaves the closure is plain values — an ink, an optional
    // opaque background, and a brush handle the state keeps alive until the dialog ends.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation, the dialog lives with
            // the system colours (NFR-13).
            let brushes = state.brushes.as_ref()?;
            let palette = state.palette;

            Some(match message {
                // The background of the dialog itself: no text ever lands on this DC, so
                // the brush is the whole answer.
                WM_CTLCOLORDLG => (None, None, brushes.window_bg()),

                // Captions and notes — and the one read-only field the message files under
                // static. The role decides the ink; the caption roles take no opaque
                // background, so the text sits transparently on the window brush instead
                // of in a box of a slightly different colour.
                WM_CTLCOLORSTATIC => match static_color_role(control_id) {
                    StaticColorRole::Label => (Some(palette.text), None, brushes.window_bg()),
                    StaticColorRole::Muted => (Some(palette.text_muted), None, brushes.window_bg()),
                    StaticColorRole::Field => (
                        Some(palette.text),
                        Some(palette.field_bg),
                        brushes.field_bg(),
                    ),
                },

                // Input fields, the exclusion list box — and the dropped-down list of
                // every combo box, whose list window sends this message to the dialog too.
                WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => (
                    Some(palette.text),
                    Some(palette.field_bg),
                    brushes.field_bg(),
                ),

                // The background around the text of checkboxes and radios, until their own
                // drawing arrives with T-11-5a/b/c.
                WM_CTLCOLORBTN => (None, None, brushes.window_bg()),

                // Unreachable: the caller only routes the five messages above here.
                _ => return None,
            })
        })
    };

    let Some((ink, opaque_bg, brush)) = choice.flatten() else {
        return 0;
    };

    // NFR-13, for the three DC calls below: each answers the *previous* value, or
    // `CLR_INVALID` (zero for `SetBkMode`) when the DC is not live. The dialog manager
    // never sends a dead DC — only a forged message could (SEC-05), and the right reaction
    // to a forgery is indifference: the answers are dropped, nothing is read back through
    // the DC, no `debug_assert` hands the forger a crash of a debug build, and the journal
    // has no row for GDI refusals (reviews\T-11-1.md).
    if let Some(colour) = ink {
        // SAFETY: `dc` is a handle passed by value; the call writes an attribute of the DC
        // and touches no memory of this process.
        unsafe { SetTextColor(dc, colour) };

        match opaque_bg {
            // A field: the text background is the field colour, same as the brush.
            Some(colour) => {
                // SAFETY: as above.
                unsafe { SetBkColor(dc, colour) };
            }
            // A caption: transparent over the window brush.
            None => {
                // SAFETY: as above.
                unsafe { SetBkMode(dc, TRANSPARENT) };
            }
        }
    }

    brush.0 as isize
}

/// Puts the interface strings of the locale in force into the window — FR-94.
///
/// Runs before anything is filled in, so that a control is never seen carrying the literal the
/// template shipped with. In the Russian locale the result is the same text the template
/// already had — the table and the template are identical by construction, and a test asserts
/// it — which is what makes this loop safe to run unconditionally rather than only for English.
fn localise_dialog(hwnd: HWND) {
    let caption = wide(&text(IDS_DIALOG_CAPTION));

    // SAFETY: `hwnd` is the live dialog and `caption` is a NUL-terminated UTF-16 buffer owned by
    // this frame, neither moved nor dropped until the call returns; the call copies it.
    if let Err(error) = unsafe { SetWindowTextW(hwnd, PCWSTR(caption.as_ptr())) } {
        crate::app::report_non_critical("SetWindowTextW", &error);
    }

    for (control, string) in LOCALISED_CONTROLS {
        set_text(hwnd, *control, &text(*string));
    }
}

/// Puts the configuration into the controls — the whole of `WM_INITDIALOG`.
fn fill_dialog(hwnd: HWND, state: &mut DialogState<'_>) {
    localise_dialog(hwnd);
    subclass_hotkey_field(hwnd);

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

    // The appearance combo of FR-92а, task T-11-3. The three items go in in the order the
    // pair `theme_combo_index` / `theme_from_combo_index` writes down — `system`, `light`,
    // `dark` — and unlike the language combo the items are localised, so they come out of
    // the string table of the locale in force. This element only stores the choice: what
    // the palette does to the windows is the business of tasks T-11-4 and later.
    send_to(hwnd, IDC_THEME, CB_RESETCONTENT, 0, 0);
    combo_add(hwnd, IDC_THEME, &text(IDS_THEME_SYSTEM));
    combo_add(hwnd, IDC_THEME, &text(IDS_THEME_LIGHT));
    combo_add(hwnd, IDC_THEME, &text(IDS_THEME_DARK));
    send_to(
        hwnd,
        IDC_THEME,
        CB_SETCURSEL,
        theme_combo_index(state.working.general.theme),
        0,
    );

    // Section «Горячая клавиша» of FR-92 and FR-94. The field is read-only because the key is
    // not typed into it: the button beside it arms a capture and the field then shows the name
    // of the key that was pressed. The note under it is the "предупреждение" the requirement
    // asks for, and while a capture is armed it is what says what went wrong.
    show_hotkey(hwnd, &state.working.hotkey.key);

    // Section «Раскладки» of FR-92 — FR-30, FR-31, FR-35.
    fill_layouts(hwnd, state);

    // Section «Замена» of FR-92 — FR-41, FR-42, FR-42а, FR-44.
    let method = match state.working.replacement.method {
        ReplacementMethod::Auto => IDC_METHOD_AUTO,
        ReplacementMethod::Backspace => IDC_METHOD_BACKSPACE,
        ReplacementMethod::Selection => IDC_METHOD_SELECTION,
    };
    check_radio(hwnd, IDC_METHOD_AUTO, IDC_METHOD_SELECTION, method);
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
            || text(IDS_LOG_DIR_MISSING),
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
    let unresolved = [
        (source, IDS_LAYOUT_WORD_SOURCE),
        (target, IDS_LAYOUT_WORD_TARGET),
    ]
    .into_iter()
    .filter(|(spec, _)| spec.resolve(&state.session).is_none())
    .map(|(_, word)| text(word))
    .collect::<Vec<_>>();

    set_text(
        hwnd,
        IDC_LAYOUT_NOTE,
        &if unresolved.is_empty() {
            String::new()
        } else {
            format_text(IDS_LAYOUT_NOTE, &[&unresolved.join(", ")])
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
        &format_text(
            IDS_STATE_HOOK,
            &[
                &text(if crate::watchdog::hook_down() {
                    IDS_HOOK_DOWN
                } else {
                    IDS_HOOK_UP
                }),
                &health.recoveries.to_string(),
                &health.install_failures.to_string(),
            ],
        ),
    );

    set_text(
        hwnd,
        IDC_STATE_LAYOUTS,
        &format_text(
            IDS_STATE_LAYOUTS,
            &[
                &state.session.len().to_string(),
                &crate::guard::counters().exclusions.to_string(),
                &crate::diag::recorded().to_string(),
            ],
        ),
    );

    set_text(
        hwnd,
        IDC_STATE_AUTOSTART,
        &format_text(
            IDS_STATE_AUTOSTART,
            &[&match autostart_value() {
                Some(command) => format_text(IDS_AUTOSTART_PRESENT, &[&command]),
                None => text(IDS_AUTOSTART_ABSENT),
            }],
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
    state.working.general.theme =
        theme_from_combo_index(send_to(hwnd, IDC_THEME, CB_GETCURSEL, 0, 0));

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

    // Three radios, one value. The fallback of the last arm is `Auto` and not `Backspace`,
    // because `auto` is the default of section 7: a dialog where somehow none of the three is
    // ticked reads back as the default rather than as a manual override nobody chose.
    state.working.replacement.method = if is_checked(hwnd, IDC_METHOD_SELECTION) {
        ReplacementMethod::Selection
    } else if is_checked(hwnd, IDC_METHOD_BACKSPACE) {
        ReplacementMethod::Backspace
    } else {
        ReplacementMethod::Auto
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

        // FR-94. The one button of the dialog that is a mode and not an action: pressing it
        // arms the capture, pressing it again cancels it.
        //
        // SAFETY: see the caller.
        IDC_HOTKEY_CAPTURE => unsafe { toggle_capture(hwnd) },

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
    // FR-94. A capture still armed when «Применить» is pressed is a capture the user has
    // abandoned, and it has to end here rather than survive the publication: `apply` publishes
    // `general.enabled` into the callback, and a capture ending afterwards would then publish
    // its own idea of that flag back over it.
    //
    // SAFETY: see the caller.
    unsafe { with_state(hwnd, |state| cancel_capture(hwnd, state)) };

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

            // FR-92а, task T-11-4 — the half of position 25 this dialog owns: the person
            // who chose «Тёмное» and pressed «Применить» sees the change now, not on the
            // next opening. The palette is resolved afresh — the setting may have changed,
            // and under `system` the system switch may have flipped while the dialog was
            // up — and only a *changed* resolution repaints: `resolve` answers `&'static`
            // identity, so pointer equality is the whole test, and an unchanged palette
            // costs nothing here.
            let fresh = theme::resolve(state.working.general.theme, theme::system_is_light());

            // The brushes are recreated from the new palette, never rewritten in place:
            // the new set is built first and the old one is dropped by the assignment, as
            // the documentation of `Brushes` prescribes. A refusal of `Brushes::new`
            // (NFR-13) leaves palette and brushes exactly as they were — the previous
            // consistent palette on screen is better than half a new one — and is not
            // journaled (reviews\T-11-1.md).
            if !std::ptr::eq(fresh, state.palette)
                && let Some(brushes) = theme::Brushes::new(fresh)
            {
                state.palette = fresh;
                state.brushes = Some(brushes);

                apply_title_bar_theme(hwnd, fresh);
                repaint_after_palette_change(hwnd);
            }
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
// FR-94 — the capture, as the dialog runs it
// -----------------------------------------------------------------------------------------

thread_local! {
    /// The window procedure of the hotkey field before this module put its own in front.
    ///
    /// A thread-local because the dialog belongs to the UI thread and to no other (section 6.1),
    /// and because only one dialog can be open at a time — the guard in [`show_dialog`] — so
    /// there is never more than one field to remember. Re-opening the dialog builds a new
    /// control and stores its own procedure over this one.
    static HOTKEY_FIELD_PROC: Cell<WNDPROC> = const { Cell::new(None) };
}

/// Puts the key name and the note that belongs to it into the two controls of the section.
fn show_hotkey(hwnd: HWND, key: &str) {
    set_text(hwnd, IDC_HOTKEY, key);
    set_text(
        hwnd,
        IDC_HOTKEY_NOTE,
        &hotkey_note(key).map_or_else(String::new, text),
    );
}

/// Puts this module's window procedure in front of the hotkey field's own.
///
/// # Where the keystrokes of a capture come from, and why from here
///
/// Not from the low-level hook. That callback is bound by NFR-01 to NFR-05 — no allocation, no
/// blocking, no input-output, under a hundred microseconds — and everything a capture does is
/// exactly what those rules forbid: build a string, set the text of two controls, decide what to
/// show. It also lives in `src\hook.rs`, which this task must not change, and teaching the hot
/// path of every keystroke in the machine about a window that is open once in a blue moon would
/// be the wrong trade whatever the rules said.
///
/// So the keys arrive the ordinary way — as window messages, on the UI thread, to the control
/// that has the focus — and this procedure reads them there. The dialog manager is the only
/// thing in the way, because `IsDialogMessage` keeps Tab, Escape, Enter and the arrows for
/// itself; answering `WM_GETDLGCODE` with `DLGC_WANTALLKEYS` while a capture is armed is what
/// asks it to hand them over.
///
/// A thread-wide `WH_KEYBOARD` hook would have worked too and was rejected for the first reason
/// above: it would make a *hook callback* out of code that sets window text, and NFR-05 forbids
/// that in a hook callback whether the hook is global or not.
fn subclass_hotkey_field(hwnd: HWND) {
    // SAFETY: `hwnd` is the live dialog; the crate turns a missing control into an error.
    let Ok(field) = (unsafe { GetDlgItem(Some(hwnd), IDC_HOTKEY) }) else {
        crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
        return;
    };

    // SAFETY: `field` is the live edit control of this dialog, created by the dialog manager on
    // this thread, and `GWLP_WNDPROC` is the documented way to put a procedure in front of its
    // own. The value stored is a function of exactly the signature the window manager calls.
    // The control is destroyed with the dialog, so there is no window left afterwards for the
    // displaced procedure to be missing from.
    let previous =
        unsafe { SetWindowLongPtrW(field, GWLP_WNDPROC, hotkey_field_proc as *const () as isize) };

    if previous == 0 {
        // NFR-13. Not fatal: the field then behaves as it did before this task, showing the key
        // and refusing to capture one, which is visible rather than silent.
        crate::app::report_non_critical("SetWindowLongPtrW", &WinError::from_thread());
        return;
    }

    // SAFETY: `previous` is what the window manager has just handed back as this control's
    // window procedure. It is a pointer to a function of that exact signature, and it is not
    // null — checked above — which is what makes the `Option` `Some`. The transmute converts
    // between two representations of the same pointer and nothing else.
    let previous = unsafe { std::mem::transmute::<isize, WNDPROC>(previous) };

    HOTKEY_FIELD_PROC.with(|slot| slot.set(previous));
}

/// The window procedure the hotkey field runs while this dialog is up.
///
/// Everything it does is confined to `armed`: with no capture in progress every message goes
/// straight to the procedure it displaced, so the field behaves exactly as an ordinary
/// read-only edit control.
///
/// # Safety
///
/// Called by the window manager with the arguments of a window message, on the control
/// [`subclass_hotkey_field`] installed it on.
unsafe extern "system" fn hotkey_field_proc(
    field: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: `field` is that control, and the parent of a control of a dialog is the dialog.
    // A window with no parent answers an error, which the default below turns into "no dialog"
    // and therefore into "not armed".
    let dialog = unsafe { GetParent(field) }.unwrap_or_default();

    // SAFETY: `dialog` is the dialog `show_dialog` created, whose `GWLP_USERDATA` holds either
    // zero or the pointer that function stored — the contract of `with_state`.
    let armed = unsafe { with_state(dialog, |state| state.capture.is_some()) }.unwrap_or(false);

    if armed {
        match message {
            // The whole reason this procedure exists. Without it `IsDialogMessage` takes Tab,
            // Escape, Enter and the arrows for itself and the capture never sees them.
            WM_GETDLGCODE => return LRESULT(DLGC_WANTALLKEYS as isize),

            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // `wparam` of a key message is the virtual-key code, which is the whole of what
                // the capture reads. **SEC-01:** no character is asked for, no layout is
                // consulted and `lparam` — where the scan code and the repeat count live — is
                // not looked at.
                //
                // SAFETY: as above.
                unsafe { on_capture_key(dialog, wparam.0 as u16) };

                return LRESULT(0);
            }

            // Swallowed rather than forwarded: the release of a key whose press was taken must
            // not reach the control, and `WM_CHAR` is the message that would put a *character*
            // into a field this program promises never to show one in.
            WM_KEYUP | WM_SYSKEYUP | WM_CHAR | WM_SYSCHAR => return LRESULT(0),

            // Clicking somewhere else abandons the capture, because a capture nobody can see is
            // a program with its conversion switched off for no visible reason. The one window
            // that may take the focus without cancelling is the capture button itself: it is
            // about to report that it was clicked, and cancelling here would turn that click
            // into a fresh arming.
            WM_KILLFOCUS => {
                let taking = HWND(std::ptr::without_provenance_mut(wparam.0));

                if !is_capture_button(dialog, taking) {
                    // SAFETY: as above.
                    unsafe { with_state(dialog, |state| cancel_capture(dialog, state)) };
                }
            }

            _ => {}
        }
    }

    match HOTKEY_FIELD_PROC.with(Cell::get) {
        // SAFETY: `previous` is the procedure this one displaced on this very control, and the
        // four arguments are the ones the window manager passed in, forwarded unchanged. This
        // is what `CallWindowProcW` exists for.
        previous @ Some(_) => unsafe { CallWindowProcW(previous, field, message, wparam, lparam) },

        // Unreachable: this procedure is only ever installed together with the slot being
        // filled. Answering the default rather than zero keeps a message from being silently
        // eaten if it ever became reachable.
        //
        // SAFETY: the same four arguments, forwarded unchanged.
        None => unsafe { DefWindowProcW(field, message, wparam, lparam) },
    }
}

/// Whether `window` is the button that arms the capture.
fn is_capture_button(dialog: HWND, window: HWND) -> bool {
    // SAFETY: `dialog` is the live dialog; the crate turns a missing control into an error.
    unsafe { GetDlgItem(Some(dialog), IDC_HOTKEY_CAPTURE) }.is_ok_and(|button| button == window)
}

/// Arms the capture, or cancels the one already armed — the button of the section.
///
/// # Safety
///
/// Called from [`dialog_proc`] only, with the `hwnd` of the dialog it belongs to.
unsafe fn toggle_capture(hwnd: HWND) {
    // SAFETY: see the caller.
    unsafe {
        with_state(hwnd, |state| {
            if state.capture.is_some() {
                cancel_capture(hwnd, state);
            } else {
                arm_capture(hwnd, state);
            }
        })
    };
}

/// Arms a capture: the conversion path stops, the note says what to do, the field takes focus.
fn arm_capture(hwnd: HWND, state: &mut DialogState<'_>) {
    state.capture = Some(CaptureSession::arm(state.working.hotkey.key.clone()));

    set_text(hwnd, IDC_HOTKEY_CAPTURE, &text(IDS_HOTKEY_STOP));
    set_text(hwnd, IDC_HOTKEY_NOTE, &text(IDS_CAPTURE_PROMPT));
    focus_control(hwnd, IDC_HOTKEY);
}

/// Abandons the capture and puts back the key that stood there before it was armed.
///
/// Dropping the session is what restores the callback's `general.enabled`; the assignment below
/// is what restores the *configuration*, and it matters even though nothing else has written to
/// that field: a capture that was accepted and then armed again remembers the accepted name,
/// and cancelling the second capture has to go back to it and not to the file's original.
fn cancel_capture(hwnd: HWND, state: &mut DialogState<'_>) {
    let Some(session) = state.capture.take() else {
        return;
    };

    let previous = session.previous_key().to_owned();

    drop(session);

    state.working.hotkey.key = previous;

    show_hotkey(hwnd, &state.working.hotkey.key);
    set_text(hwnd, IDC_HOTKEY_CAPTURE, &text(IDS_HOTKEY_SET));
}

/// Takes the captured key: the field shows its name and the configuration carries it.
///
/// Nothing is published to the hook from here. The dialog edits a copy and publishes on
/// «Применить» like every other field — through `app::publish_configuration`, which is the one
/// caller of `hook::set_hotkey_vk` there has ever been.
fn accept_capture(hwnd: HWND, state: &mut DialogState<'_>, name: String) {
    // Ends the capture and publishes the callback's previous `general.enabled` back.
    state.capture = None;

    state.working.hotkey.key = name;

    show_hotkey(hwnd, &state.working.hotkey.key);
    set_text(hwnd, IDC_HOTKEY_CAPTURE, &text(IDS_HOTKEY_SET));
}

/// One key pressed while a capture is armed.
///
/// # Safety
///
/// Called from [`hotkey_field_proc`] only, with the `hwnd` of the dialog the field belongs to.
unsafe fn on_capture_key(dialog: HWND, vk: u16) {
    // Escape is the way out of the capture and is therefore the one key a capture cannot
    // assign. It stays assignable by hand: `hook::vk_from_name` still reads `Escape` and `Esc`
    // out of the file, and the dialog shows whatever it finds there.
    if vk == VK_ESCAPE.0 {
        // SAFETY: see the caller.
        unsafe { with_state(dialog, |state| cancel_capture(dialog, state)) };
        return;
    }

    let outcome = capture(vk, Modifiers::held_now());

    // SAFETY: see the caller.
    unsafe {
        with_state(dialog, |state| match outcome {
            Capture::Taken(name) => accept_capture(dialog, state, name),

            // The capture stays armed: a refusal is a "not that one", not an end to the
            // question. The note says which of the five reasons it was.
            Capture::Refused(refusal) => {
                set_text(dialog, IDC_HOTKEY_NOTE, &text(refusal.string_id()));
            }
        })
    };
}

/// Moves the keyboard focus to one control of the dialog.
///
/// `SetFocus` and not the `WM_NEXTDLGCTL` message the dialog manager also understands, for a
/// reason that has nothing to do with focus: **`SendMessage` is forbidden anywhere in `src\`**
/// by acceptance point 17 of FR-72, and `tests\guard.rs` sweeps the whole directory for it. The
/// call this window would make is to a window of its own thread and could not block — but the
/// rule is a rule about the program and not about one call site, and a task that carved an
/// exception into it would be weakening a guard the watcher thread depends on.
///
/// What is given up is small and named: the dialog manager's idea of the default button is not
/// updated. While a capture is armed that costs nothing, because the field answers
/// `DLGC_WANTALLKEYS` and Enter never reaches the manager in the first place.
fn focus_control(hwnd: HWND, control: i32) {
    // SAFETY: `hwnd` is the live dialog; the crate turns a missing control into an error.
    let Ok(window) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
        crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
        return;
    };

    // SAFETY: `window` is the live control just found, on this thread, which is what `SetFocus`
    // requires of its argument. NFR-13: the crate turns "no window had the focus" into an error,
    // and that is not a failure — the dialog has only just been shown in that case — so the
    // result is examined and deliberately not reported.
    let _ = unsafe { SetFocus(Some(window)) };
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
