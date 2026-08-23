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
    COLORREF, ERROR_FILE_NOT_FOUND, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE,
    WPARAM,
};
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, COLORONCOLOR,
    ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection,
    CreateFontIndirectW, CreatePen, CreateSolidBrush, DIB_RGB_COLORS, DRAW_TEXT_FORMAT,
    DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_EXPANDTABS, DT_LEFT, DT_SINGLELINE, DT_TOP,
    DT_VCENTER, DT_WORDBREAK, DeleteDC, DeleteObject, DrawFocusRect, DrawTextW, Ellipse, EndPaint,
    ExcludeClipRect, FW_BOLD, FillRect, GdiFlush, GetDC, GetDeviceCaps, GetObjectW, GetStockObject,
    GetTextExtentPoint32W, HBITMAP, HBRUSH, HDC, HFONT, HGDIOBJ, IntersectClipRect, InvalidateRect,
    LOGFONTW, LOGPIXELSY, NULL_PEN, PAINTSTRUCT, PS_SOLID, Polyline, RDW_ALLCHILDREN, RDW_ERASE,
    RDW_INVALIDATE, RedrawWindow, ReleaseDC, RestoreDC, RoundRect, SRCCOPY, SaveDC, SelectObject,
    SetBkColor, SetBkMode, SetStretchBltMode, SetTextColor, StretchBlt, TRANSPARENT, TextOutW,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceExW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::UI::Controls::{
    CDDS_ITEMPREPAINT, CDDS_PREPAINT, CDIS_SELECTED, CDRF_DODEFAULT, CDRF_NOTIFYITEMDRAW,
    DRAWITEMSTRUCT, EM_LIMITTEXT, EM_SETMARGINS, HIMAGELIST, ICC_LISTVIEW_CLASSES, ILC_COLOR32,
    ILC_MASK, INITCOMMONCONTROLSEX, ImageList_AddMasked, ImageList_Create, ImageList_Destroy,
    InitCommonControlsEx, LIST_VIEW_ITEM_STATE_FLAGS, LVCF_WIDTH, LVCOLUMNW, LVIF_STATE, LVIF_TEXT,
    LVIS_FOCUSED, LVIS_SELECTED, LVIS_STATEIMAGEMASK, LVITEMW, LVM_DELETEALLITEMS,
    LVM_GETITEMSTATE, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETBKCOLOR,
    LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETIMAGELIST, LVM_SETITEMSTATE, LVM_SETTEXTBKCOLOR,
    LVM_SETTEXTCOLOR, LVN_ITEMCHANGING, LVNI_SELECTED, LVS_EX_CHECKBOXES, LVS_EX_FULLROWSELECT,
    LVSIL_STATE, MEASUREITEMSTRUCT, NM_CUSTOMDRAW, NMCUSTOMDRAW_DRAW_STATE_FLAGS, NMHDR,
    NMLVCUSTOMDRAW, ODS_COMBOBOXEDIT, ODS_DISABLED, ODS_FOCUS, ODS_NOFOCUSRECT, ODS_SELECTED,
    ODT_BUTTON, ODT_COMBOBOX, ODT_LISTBOX, ODT_STATIC,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, IsWindowEnabled, SetFocus, VIRTUAL_KEY, VK_APPS,
    VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_HOME, VK_INSERT,
    VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_NUMLOCK, VK_PAUSE,
    VK_PRIOR, VK_RCONTROL, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT,
    VK_SNAPSHOT, VK_UP,
};
use windows::Win32::UI::Shell::{
    DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, BN_DBLCLK, BN_SETFOCUS, CB_ADDSTRING, CB_GETCURSEL, CB_GETLBTEXT, CB_GETLBTEXTLEN,
    CB_RESETCONTENT, CB_SETCURSEL, CallWindowProcW, DLGC_WANTALLKEYS, DM_SETDEFID, DWLP_MSGRESULT,
    DefWindowProcW, DialogBoxParamW, EC_LEFTMARGIN, EC_RIGHTMARGIN, EndDialog, GW_CHILD,
    GW_HWNDNEXT, GWLP_USERDATA, GWLP_WNDPROC, GetClientRect, GetDlgCtrlID, GetDlgItem,
    GetDlgItemTextW, GetParent, GetWindow, GetWindowLongPtrW, GetWindowRect, IDCANCEL, IDOK,
    LB_ADDSTRING, LB_DELETESTRING, LB_GETCOUNT, LB_GETCURSEL, LB_GETTEXT, LB_GETTEXTLEN,
    LB_RESETCONTENT, MapDialogRect, PostMessageW, SW_SHOWNORMAL, SendDlgItemMessageW,
    SetDlgItemTextW, SetWindowLongPtrW, SetWindowTextW, UISF_HIDEFOCUS, WINDOW_LONG_PTR_INDEX,
    WM_APP, WM_CHAR, WM_COMMAND, WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLOREDIT,
    WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC, WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND, WM_GETDLGCODE,
    WM_GETFONT, WM_INITDIALOG, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_MEASUREITEM, WM_NCDESTROY,
    WM_NOTIFY, WM_PAINT, WM_QUERYUISTATE, WM_SETFONT, WM_SYSCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WNDPROC,
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
/// Caption of the about dialog — FR-92а, task T-11-11. The five rows 3061–3065 continue
/// the block 3056 opened, contiguously — no gap, so no empty block between.
pub const IDS_ABOUT_CAPTION: u16 = 3061;
/// The version line of the about dialog. Carries a `{0}`: the number is substituted by
/// [`format_text`] from the `VERSIONINFO` resource of the running executable
/// ([`about_version_line`]), never spelled in the resources twice.
pub const IDS_ABOUT_VERSION: u16 = 3062;
/// First description line of the about dialog.
pub const IDS_ABOUT_LINE_1: u16 = 3063;
/// Second description line of the about dialog.
pub const IDS_ABOUT_LINE_2: u16 = 3064;
/// The «ОК» of the about dialog. A row of its own rather than a borrow of [`IDS_OK`]: that
/// row belongs to the settings dialog, and a shared row would make a rewording of one
/// window silently reword the other.
pub const IDS_ABOUT_OK: u16 = 3065;
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
pub const INTERFACE_STRINGS: [u16; 72] = [
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
    IDS_ABOUT_CAPTION,
    IDS_ABOUT_VERSION,
    IDS_ABOUT_LINE_1,
    IDS_ABOUT_LINE_2,
    IDS_ABOUT_OK,
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

/// Resource identifier of the about dialog template in `app.rc` — FR-92а, task T-11-11.
/// Kept equal by hand, exactly as [`IDD_SETTINGS`] above.
pub const IDD_ABOUT: u16 = 201;

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
// 1029 and not 1033: `check_radio` walks the identifier range it is given, so the three
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

// The controls of the about dialog `IDD_ABOUT` — FR-92а, task T-11-11. A block of their own
// from 1120, contiguous and clear of everything above; its «ОК» is `IDOK` and is not here.
const IDC_ABOUT_ICON: i32 = 1120;
const IDC_ABOUT_NAME: i32 = 1121;
const IDC_ABOUT_VERSION: i32 = 1122;
const IDC_ABOUT_LINE_1: i32 = 1123;
const IDC_ABOUT_LINE_2: i32 = 1124;

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
pub const LOCALISED_CONTROLS: &[(i32, u16)] = &[
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
///
/// One-based index 2 in the `LVIS_STATEIMAGEMASK` bits — the *second* frame of whatever state
/// image list the control holds, the system's `LVS_EX_CHECKBOXES` pair and the palette frames
/// of [`CHECK_FRAME_ORDER`] alike. Public since task T-11-7 so a test can hold the bits and the
/// frame order together; the participation mechanism of FR-31 reads and writes exactly these
/// bits ([`set_row_check`], [`read_cycle_checks`]) and is not allowed to drift.
pub const CHECKED_IMAGE: u32 = 0x2000;

/// State image index of an unticked one — one-based index 1, the *first* frame.
pub const UNCHECKED_IMAGE: u32 = 0x1000;

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
            // FR-92а, task T-11-9: the record must not outlive the dialog it names — see
            // `DIALOG_WINDOW`. Cleared on the same guard so that no exit path, panic
            // included, can leave a stale window behind.
            DIALOG_WINDOW.with(|window| window.set(0));
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

    let state = DialogCell {
        // Task T-11-5b-2: every glyph element starts «снят» here; `fill_dialog` writes the
        // configuration in on `WM_INITDIALOG`, before the dialog is first painted.
        checks: GlyphChecks::new(),
        state: RefCell::new(DialogState {
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
            // Empty until `WM_INITDIALOG`: the map is geometry of live windows, and there are
            // no windows yet. Filled exactly once — see the field's own documentation.
            panel_children: Vec::new(),
            // `None` until `WM_INITDIALOG` for the same reason: the faces are made out of the
            // font the dialog manager gives the window, and there is no window yet.
            fonts: None,
            // `None` until the first `WM_ERASEBKGND`, which is where the picture is built.
            background: None,
        }),
    };

    // SAFETY: `instance` is a module handle whose resources carry `IDD_SETTINGS`, and the
    // "name" is an integer identifier in the `MAKEINTRESOURCE` form — a value below 65536
    // carried inside the pointer, never dereferenced as a string. `owner` is a live window of
    // this thread. The parameter is a pointer to `state`, which lives on this frame: the call
    // is modal and does not return until `EndDialog`, so the pointer cannot outlive the value
    // it names. `dialog_proc` is the only reader of it; the guarded half is read through the
    // `RefCell` of the `state` field, so no two borrows can overlap however the dialog manager
    // re-enters, and the check store beside it is `Cell`-based — mutation through a shared
    // reference, no borrow to collide with (task T-11-5b-2).
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

    /// The window of that dialog while it is up, zero otherwise — FR-92а, task T-11-9.
    ///
    /// Recorded on `WM_INITDIALOG` and cleared by the guard of [`show_dialog`] however that
    /// function leaves, so a non-zero record always names the live modal dialog of this
    /// thread. Stored as the plain integer the handle is: `HWND` is a pointer type, and a
    /// thread-local wants a value. A thread-local like its neighbour, because the dialog
    /// belongs to the UI thread and to no other (section 6.1).
    static DIALOG_WINDOW: Cell<isize> = const { Cell::new(0) };
}

// ---------------------------------------------------------------------------------------
// The system theme changing under the open dialog — FR-92а, task T-11-9
// ---------------------------------------------------------------------------------------

/// The one string of `WM_SETTINGCHANGE` this program reacts to — FR-92а, task T-11-9.
///
/// Windows broadcasts a `WM_SETTINGCHANGE` naming this word when the personalization switch
/// `AppsUseLightTheme` moves. SEC-05 allows exactly the comparison against it and forbids
/// using the message's content any further.
pub const IMMERSIVE_COLOR_SET: &str = "ImmersiveColorSet";

/// The dialog's own «системная тема сменилась» nudge — FR-92а, task T-11-9.
///
/// `WM_APP + 14`, the next free number of the program-wide row: `+ 1` is the wake-up of
/// [`crate::app`] and `+ 5` its configuration nudge, `+ 2` the tray callback, `+ 3` and
/// `+ 4` belong to [`crate::hook`], `+ 6`, `+ 7` and `+ 9` to [`crate::watchdog`], `+ 8` is
/// [`crate::switch::WM_APP_SWITCH`], `+ 10` and `+ 11` are [`crate::guard`]'s pair, and
/// `+ 12` and `+ 13` are [`crate::selection`]'s.
///
/// SEC-05: the message carries nothing and decides nothing. The handler in [`dialog_proc`]
/// re-resolves the palette out of this program's own setting and the system switch
/// ([`refresh_palette`]), and an unchanged resolution repaints nothing — so a forged message
/// buys the sender one comparison of two pointers, and at worst one repaint of our own
/// dialog with the palette it already ought to wear.
pub const WM_APP_SYSTEM_THEME: u32 = WM_APP + 14;

/// «Перекрашивать?» — the pure decision of task T-11-9, closed by a table in
/// `tests\settings.rs`.
///
/// Yes exactly when three facts line up: the message named the theme switch — the string of
/// the `WM_SETTINGCHANGE` is exactly [`IMMERSIVE_COLOR_SET`], a null `lParam` having read as
/// `None` long before this point; the setting in force is `system` — under `light` and
/// `dark` FR-92а fixes the palette and the system has no say; and the palette the system now
/// resolves to differs from the one on the screen. Identity is the comparison because
/// [`theme::resolve`] answers `&'static` identity — the same test [`refresh_palette`] runs
/// again before repainting, which is what settles the batch Windows sends of these into one
/// repaint.
pub fn repaint_for_system_theme(
    setting_string: Option<&str>,
    setting: ThemeSetting,
    current: &theme::Palette,
    by_system: &theme::Palette,
) -> bool {
    setting_string == Some(IMMERSIVE_COLOR_SET)
        && setting == ThemeSetting::System
        && !std::ptr::eq(current, by_system)
}

/// The far end of the tray's `WM_SETTINGCHANGE` and `WM_THEMECHANGED` arms — FR-92а,
/// task T-11-9.
///
/// `setting_string` is what the message named, already read within the bounds SEC-05
/// prescribes — the reading belongs to the module that owns the receiving window. A closed
/// dialog is the first exit and costs one thread-local read: it will resolve the fresh
/// system switch when it opens ([`show_dialog`] reads it at initialisation), so there is
/// nothing to tell it now. Otherwise the decision is [`repaint_for_system_theme`] over the
/// dialog's own state, and a yes is one `PostMessageW` of [`WM_APP_SYSTEM_THEME`] to the
/// dialog — posted, never sent, like everything this program tells its own windows (the
/// implication of FR-72).
///
/// A message arriving while the state is borrowed — a re-entrant dispatch inside a handler —
/// finds `with_state` answering `None` and does nothing; Windows sends these in a batch, and
/// a later arrival of the batch finds the borrow free.
pub fn on_system_theme_message(setting_string: Option<&str>) {
    let raw = DIALOG_WINDOW.with(Cell::get);

    if raw == 0 {
        return;
    }

    let hwnd = HWND(raw as *mut std::ffi::c_void);

    // SAFETY: a non-zero record names the live modal dialog of this thread — recorded on
    // `WM_INITDIALOG`, cleared however `show_dialog` leaves — so `hwnd` is the window of a
    // dialog created by `show_dialog`, which is the contract of `with_state`.
    let repaint = unsafe {
        with_state(hwnd, |state| {
            repaint_for_system_theme(
                setting_string,
                state.working.general.theme,
                state.palette,
                theme::resolve(ThemeSetting::System, theme::system_is_light()),
            )
        })
    };

    if repaint == Some(true) {
        // SAFETY: `hwnd` is the live dialog; the message carries two plain zeros and no
        // pointer — `PostMessageW` queues them by value and returns.
        if let Err(error) =
            unsafe { PostMessageW(Some(hwnd), WM_APP_SYSTEM_THEME, WPARAM(0), LPARAM(0)) }
        {
            // NFR-13. Not fatal: the palette catches up on the next occasion — the next
            // message of the batch, or a pressed «Применить» — and the journal is told.
            crate::app::report_non_critical("PostMessageW", &error);
        }
    }
}

/// The eight owner-drawn check boxes and radio buttons whose check state the dialog keeps
/// itself — FR-92а, task T-11-5b-2. The storage-side list, beside the drawing-side list of
/// [`glyph_kind`]: both name the same eight controls of the template.
///
/// Public for the same reason the colour tables are: `tests\settings.rs` exercises the
/// store over these very identifiers, without a live window.
pub const GLYPH_CHECK_CONTROLS: [i32; 8] = [
    IDC_AUTOSTART,
    IDC_MODE_PAIR,
    IDC_MODE_CYCLE,
    IDC_METHOD_AUTO,
    IDC_METHOD_BACKSPACE,
    IDC_METHOD_SELECTION,
    IDC_SELECTION_ENABLED,
    IDC_LOG_ENABLED,
];

/// The check state of the eight owner-drawn check boxes and radio buttons — FR-92а,
/// task T-11-5b-2.
///
/// A button of type `BS_OWNERDRAW` keeps no check state of its own: the button-message
/// pair that stores and answers it for the automatic types ignores the write and answers
/// «снят» for an owner-drawn one — which the final sweep of the live acceptance saw as
/// all eight glyphs drawn unchecked whatever the configuration said. This store is that
/// state, kept by the dialog itself: a fixed array of «идентификатор → взведён» pairs —
/// the elements are eight and known, so no map — and it is the *only* truth about the
/// eight: nothing asks the controls, so nothing can quietly disagree with it.
///
/// The flag sits in a [`Cell`] so that a shared reference reads and writes it, and that
/// interior mutability is load-bearing, not convenience: `fill_dialog` writes the store
/// from inside the `WM_INITDIALOG` borrow of [`DialogState`] and `read_dialog` reads it
/// from inside the «Применить» borrow, where a nested `RefCell` borrow would be refused —
/// the store must answer at every depth, which is what [`DialogCell`] and
/// [`with_glyph_checks`] arrange.
///
/// A write to an identifier the store does not carry is dropped and a read answers «снят»
/// (NFR-13: a wrong identifier misdraws one glyph, it does not fall). The answer for a
/// store that does not exist *yet* is the caller's business: [`with_glyph_checks`] says
/// `None`, and [`is_checked`] turns that into «снят».
pub struct GlyphChecks {
    /// The pairs, in template order. The identifier column never changes after [`Self::new`].
    entries: [(i32, Cell<bool>); 8],
}

impl GlyphChecks {
    /// The eight known identifiers, every one «снят» — the state of the dialog before
    /// `fill_dialog` writes the configuration in.
    pub fn new() -> Self {
        Self {
            entries: GLYPH_CHECK_CONTROLS.map(|control| (control, Cell::new(false))),
        }
    }

    /// Writes one element's state. An identifier outside the eight is dropped — see the
    /// type's own documentation.
    pub fn set(&self, control: i32, on: bool) {
        if let Some((_, cell)) = self.entries.iter().find(|(id, _)| *id == control) {
            cell.set(on);
        }
    }

    /// Reads one element's state; «снят» for an identifier outside the eight.
    pub fn get(&self, control: i32) -> bool {
        self.entries
            .iter()
            .find(|(id, _)| *id == control)
            .is_some_and(|(_, cell)| cell.get())
    }

    /// Arms `chosen` and quenches every other identifier of `first..=last` — the store
    /// half of what the automatic radio types did for themselves: the walk is the same
    /// contiguous identifier range, the template's `WS_GROUP` runs.
    pub fn check_radio(&self, first: i32, last: i32, chosen: i32) {
        for (id, cell) in &self.entries {
            if (first..=last).contains(id) {
                cell.set(*id == chosen);
            }
        }
    }
}

impl Default for GlyphChecks {
    fn default() -> Self {
        Self::new()
    }
}

/// What `GWLP_USERDATA` of the settings dialog points at: the check store beside the
/// guarded state, both on the frame of [`show_dialog`] — task T-11-5b-2.
///
/// The store sits *beside* the `RefCell`, not inside it, deliberately. The writers and
/// readers of check state run at every depth — `fill_dialog` inside the `WM_INITDIALOG`
/// borrow, `read_dialog` inside the «Применить» borrow, `restore_self_switching` and
/// `draw_glyph_element` outside any — and a store behind the `RefCell` would answer the
/// nested callers with a refusal: «Применить» would then read every element «снят» and
/// erase the very configuration it was asked to keep. `Cell`-based state behind a shared
/// reference has no borrow to refuse.
struct DialogCell<'a> {
    /// The check state of the eight glyph elements — [`with_glyph_checks`] is the road in.
    checks: GlyphChecks,
    /// Everything else the dialog procedure needs — [`with_state`] is the road in, and
    /// its `RefCell` is what refuses re-entrant mutation.
    state: RefCell<DialogState<'a>>,
}

/// Everything the dialog procedure needs, for as long as the dialog is up.
///
/// Lives on the frame of [`show_dialog`], inside a [`DialogCell`], and is reached through
/// that cell's `RefCell` behind a raw pointer, which is the standard shape of a Win32
/// dialog: the manager gives the procedure one `LPARAM` and nothing else. The `RefCell` is
/// not decoration — a message dispatched while another message is being handled would
/// otherwise alias a `&mut`, and here it is refused at run time instead.
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
    /// Identifiers of the controls that lie on one of the eight group panels — FR-92а,
    /// task T-11-5c. The `WM_CTLCOLORSTATIC` and `WM_CTLCOLORBTN` answers for these
    /// controls take the `panel_bg` brush instead of the window one, so their erased
    /// background matches the panel they sit on.
    ///
    /// Built exactly once, on `WM_INITDIALOG`, by [`collect_panel_children`] — geometry
    /// of the template's rectangles and nothing else. Deliberately *not* rebuilt on a
    /// palette change: rectangles do not move when colours change, which is why
    /// [`apply_now`] recreates the brushes and leaves this map alone.
    panel_children: Vec<i32>,
    /// The two faces this dialog sets **its own** text in — FR-92а, task T-11-17.
    ///
    /// Owned for as long as the dialog is up and freed in `Drop`, exactly as the brushes are
    /// and for the same reason: a font selected into a DC must outlive the paint. `None` —
    /// the dialog font could not be read or the faces could not be made — leaves every
    /// drawing to the manager's own font, which is what every task before this one drew in
    /// (NFR-13).
    fonts: Option<DialogFonts>,
    /// The whole background of the window, drawn once and kept as a picture — task T-11-17.
    ///
    /// `WM_ERASEBKGND` then costs one `BitBlt` instead of eight rounded panels with their
    /// letter-spaced captions and seven rounded field frames. Rebuilt — not repainted in
    /// place — when the picture no longer fits the window: another client size, or another
    /// palette after [`refresh_palette`]. `None` before the first paint and after a refused
    /// build, which [`on_erase_background`] answers by painting straight into the message's
    /// DC as it did before this task.
    background: Option<BackgroundCache>,
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
            // `DialogBoxParamW`; it is only ever read back by `with_state` and
            // `with_glyph_checks`, below.
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0) };

            // FR-92а, task T-11-9: the record `on_system_theme_message` finds the dialog
            // by. Cleared by the guard of `show_dialog` however that function leaves, so
            // it never outlives the window it names.
            DIALOG_WINDOW.with(|window| window.set(hwnd.0 as isize));

            // SAFETY: the pointer has just been stored and names the `DialogCell` on the frame
            // of `show_dialog`, which outlives this modal call.
            unsafe {
                with_state(hwnd, |state| {
                    // FR-92а, task T-11-17: the two faces this dialog sets its own text in,
                    // made out of the font the manager gave the window and asked to render
                    // with grey antialiasing instead of ClearType. Once, here — a face is a
                    // property of the window, not of a paint — and freed with the state.
                    //
                    // ⚠ **Before `fill_dialog`, and that ordering is task T-11-20's.** Since
                    // that task the text face is not only selected into DCs by our own
                    // drawing: it is handed to the five input fields and to the layout list,
                    // which draw their own text and have to be holding it before anything is
                    // put into them. `fill_dialog` does the handing over and then the
                    // filling; it can only do the first of those if the face already exists.
                    state.fonts = DialogFonts::new(hwnd, GROUP_BOXES[0]);

                    fill_dialog(hwnd, state);

                    // FR-92а, task T-11-5c: the panel map — which controls lie on one of
                    // the eight group panels — is geometry of the just-created windows,
                    // read here once and never again: rectangles do not move when the
                    // palette changes, so «Применить» has nothing to rebuild.
                    state.panel_children = collect_panel_children(hwnd);

                    // FR-92а, task T-11-4: the non-client title bar follows the resolved
                    // palette from the first showing. The client area needs no call — the
                    // `WM_CTLCOLOR*` answers below paint it as soon as anything paints.
                    apply_title_bar_theme(hwnd, state.palette);
                })
            };

            // FR-92а, task T-11-5a: «ОК» lost `BS_DEFPUSHBUTTON` to owner drawing — a
            // button *type* occupies the low nibble the default style lived in — so the
            // default identifier is handed to the dialog manager by the documented
            // replacement, `DM_SETDEFID`. On Enter the manager asks `DM_GETDEFID` and
            // clicks the button this message named, which is what keeps Enter landing on
            // «ОК».
            //
            // *Posted*, not sent: everything this program tells its own windows goes
            // through `PostMessageW` — a bare `SendMessage` is banned program-wide by the
            // implication of FR-72, and a test sweeps `src\` for it. Nothing is lost by
            // the queue: posted messages are retrieved ahead of input, this one is queued
            // before the dialog is even shown, so the default is in force before the
            // first keystroke that could ask for it.
            //
            // SAFETY: `hwnd` is the live dialog; the message carries two plain integers
            // and no pointer — `PostMessageW` queues them by value and returns.
            if let Err(error) = unsafe {
                PostMessageW(
                    Some(hwnd),
                    DM_SETDEFID,
                    WPARAM(usize::try_from(OK_COMMAND).unwrap_or(0)),
                    LPARAM(0),
                )
            } {
                // NFR-13. Not fatal: the one consequence would be Enter answered by the
                // fallback of the dialog manager rather than by the named default, and
                // the journal is told.
                crate::app::report_non_critical("PostMessageW", &error);
            }

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

        // FR-92а, task T-11-13: the whole background of the window — the ground, the eight
        // panels with their captions, and the frames of the fields and lists. The one thing
        // taken out of the message is the DC to paint into (SEC-05), and the window of time
        // is the same as for `WM_DRAWITEM` below.
        WM_ERASEBKGND => {
            // SAFETY: as for `WM_COMMAND` below — the pointer was stored on `WM_INITDIALOG`
            // and the value it names is alive for the whole of this modal call.
            unsafe { on_erase_background(hwnd, wparam) }
        }

        // FR-92а, task T-11-5a: the nine owner-drawn buttons. Handled here and nowhere
        // else — the dialog window exists exactly while the program itself holds the
        // dialog on the screen, which is the gate the last sentence of SEC-05 names.
        WM_DRAWITEM => {
            // SAFETY: as for `WM_COMMAND` below — the pointer was stored on `WM_INITDIALOG`
            // and the value it names is alive for the whole of this modal call.
            unsafe { on_draw_item(hwnd, lparam) }
        }

        // FR-92а, task T-11-6: the item height of the four owner-drawn combo boxes. No gate
        // beyond the window itself is needed — the message arrives at the procedure of the
        // live dialog, and the dialog exists exactly while the program holds it on the
        // screen, which is what SEC-05 asks; a foreign `CtlType` or identifier is answered
        // with «not handled», which sends the message down the dialog manager's default
        // path.
        WM_MEASUREITEM => {
            // SAFETY: the sender owns the struct `lparam` names for the length of the send,
            // and this procedure is inside that send.
            unsafe { on_measure_item(hwnd, lparam) }
        }

        // FR-92а, task T-11-7: the custom-draw questions of the layout list. The gate SEC-05
        // asks for is the handler's first act — `hwndFrom` and `code` are compared before
        // any work — and the window of time is the same as for `WM_DRAWITEM` above.
        WM_NOTIFY => {
            // SAFETY: the sender owns the struct `lparam` names for the length of the send,
            // and this procedure is inside that send.
            unsafe { on_notify(hwnd, lparam) }
        }

        WM_COMMAND => {
            let control = i32::from(low_word(wparam.0));
            let notification = high_word(wparam.0);

            // SAFETY: as above — the pointer was stored on `WM_INITDIALOG` and the value it
            // names is alive for the whole of this modal call.
            unsafe { on_command(hwnd, control, notification) };

            0
        }

        // FR-92а, task T-11-14: the far half of the subclass pair. `WM_DESTROY` reaches the
        // dialog while its children are still live windows — the manager destroys them only
        // after this returns — so the four combo boxes are still there to be handed back
        // their own procedure. The message carries nothing and is not dereferenced (SEC-05);
        // a forged one would remove a subclass that is either already ours to remove or not
        // installed at all, and `RemoveWindowSubclass` simply answers FALSE for the latter.
        WM_DESTROY => {
            unsubclass_combo_boxes(hwnd);

            // «Not handled»: the dialog manager still needs its own `WM_DESTROY`.
            0
        }

        // FR-92а, task T-11-9: the system theme moved while this dialog is up — the far end
        // of the nudge `on_system_theme_message` posted. The message carries nothing and
        // decides nothing (SEC-05): `refresh_palette` resolves the palette afresh out of
        // this program's own setting and the system switch, and leaves everything alone
        // when the resolution did not move — the check that settles the batch Windows sends
        // of these into one repaint. A forged message therefore buys the sender one pointer
        // comparison, at worst one repaint of our own dialog in its own palette.
        WM_APP_SYSTEM_THEME => {
            // SAFETY: as above — the pointer was stored on `WM_INITDIALOG` and the value it
            // names is alive for the whole of this modal call.
            unsafe { with_state(hwnd, |state| refresh_palette(hwnd, state)) };

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
/// May only be called with the window of a dialog created by [`show_dialog`] — the `hwnd` its
/// dialog procedure was called with, or the live window [`DIALOG_WINDOW`] records (task
/// T-11-9) — whose `GWLP_USERDATA` therefore holds either zero or the pointer that function
/// stored.
unsafe fn with_state<R>(hwnd: HWND, f: impl FnOnce(&mut DialogState<'_>) -> R) -> Option<R> {
    // SAFETY: `hwnd` is the live dialog; reading a window field is a plain read.
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

    if raw == 0 {
        return None;
    }

    // SAFETY: by this function's own contract, `raw` is the pointer `show_dialog` stored,
    // which names the `DialogCell` on that function's frame — the call is modal, so the
    // frame is alive for as long as any message of this dialog can be handled. A shared
    // reference is all that is taken, and the `RefCell` of the `state` field is what
    // governs the mutable access below.
    let cell = unsafe { &*(raw as *const DialogCell<'_>) };

    let mut state = cell.state.try_borrow_mut().ok()?;

    Some(f(&mut state))
}

/// Runs `f` against the check store of the eight glyph elements — FR-92а, task T-11-5b-2.
///
/// The same road [`with_state`] takes — the `GWLP_USERDATA` pointer `WM_INITDIALOG`
/// stored — but without the `RefCell` borrow at the end of it: the store is `Cell`-based,
/// so a shared reference reads and writes it even while [`DialogState`] is mutably
/// borrowed. That is the whole point — `fill_dialog` and `read_dialog` run inside such a
/// borrow, and the check state must answer there too (see [`DialogCell`]).
///
/// `None` exactly when there is no state yet — messages, `WM_DRAWITEM` among them, do
/// arrive before `WM_INITDIALOG` stores the pointer. The callers then degrade per
/// NFR-13: [`is_checked`] answers «снят», [`set_check`] and [`check_radio`] drop the
/// write, and the glyph is drawn unchecked rather than not drawn at all.
///
/// # Safety
///
/// Same contract as [`with_state`]: may only be called with the window of a dialog
/// created by [`show_dialog`], whose `GWLP_USERDATA` therefore holds either zero or the
/// pointer that function stored.
unsafe fn with_glyph_checks<R>(hwnd: HWND, f: impl FnOnce(&GlyphChecks) -> R) -> Option<R> {
    // SAFETY: `hwnd` is the live dialog; reading a window field is a plain read.
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

    if raw == 0 {
        return None;
    }

    // SAFETY: by the contract above, `raw` is the pointer `show_dialog` stored, which
    // names the `DialogCell` on that function's frame — alive for as long as any message
    // of this dialog can be handled, because the call is modal. A shared reference is all
    // that is taken, and every mutation behind it goes through the `Cell`s of the store —
    // no borrow that could collide with `with_state`'s, however deep inside one of its
    // closures this runs.
    let cell = unsafe { &*(raw as *const DialogCell<'_>) };

    Some(f(&cell.checks))
}

/// Runs `f` against the [`AboutState`] of the about dialog — FR-92а, task T-11-11.
///
/// `None` on the same three occasions as [`with_state`].
///
/// # Safety
///
/// May only be called with the window of a dialog created by [`show_about_dialog`] — the
/// `hwnd` its dialog procedure was called with — whose `GWLP_USERDATA` therefore holds
/// either zero or the pointer that function stored.
unsafe fn with_about_state<R>(hwnd: HWND, f: impl FnOnce(&mut AboutState) -> R) -> Option<R> {
    // SAFETY: by this function's own contract, `GWLP_USERDATA` of `hwnd` holds either zero
    // or the pointer `show_about_dialog` stored, which names a `RefCell<AboutState>` alive
    // for the whole of that modal call.
    unsafe { with_window_state(hwnd, f) }
}

/// The window-message plumbing behind [`with_about_state`] — one body, made generic by
/// task T-11-11 instead of copied (§6.2). [`with_state`] walked the same road until task
/// T-11-5b-2 put the check store beside the settings dialog's `RefCell`: its pointer now
/// names a [`DialogCell`], not a bare cell, so it reads the field itself.
///
/// # Safety
///
/// May only be called with a window whose `GWLP_USERDATA` holds either zero or a pointer to
/// a `RefCell<S>` that outlives the modal call — which is what the wrapper above
/// guarantees through its own contract: it names the one show function whose state type
/// is `S`, and no other window can carry that pointer, because each show function
/// stores its own frame's cell into the window it alone creates.
unsafe fn with_window_state<S, R>(hwnd: HWND, f: impl FnOnce(&mut S) -> R) -> Option<R> {
    // SAFETY: `hwnd` is the live dialog; reading a window field is a plain read.
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };

    if raw == 0 {
        return None;
    }

    // SAFETY: by the contract above, `raw` is the pointer the show function stored, which
    // names a `RefCell<S>` on that function's frame. The call is modal, so the frame is alive
    // for as long as any message of this dialog can be handled. A shared reference is all that
    // is taken, and the `RefCell` is what governs the mutable access below.
    let cell = unsafe { &*(raw as *const RefCell<S>) };

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

/// One [`StaticColorRole`] resolved against the palette — the single place a role of a
/// **label** becomes an ink, shared by the labels of both windows (§6.2: one body, not two).
///
/// The identifier never appears here: which role a control plays is the business of
/// [`static_color_role`] and [`about_static_color_role`], and this function only turns the
/// answer into a colour. That split is what keeps task T-11-18 from growing a second
/// «identifier → colour» list beside the one task T-11-4 wrote.
///
/// `Field` is spelled out and not swallowed by a catch-all so the match stays closed over the
/// shared vocabulary: no label of either template is a field — the one control that answers
/// `Field` is `IDC_HOTKEY`, an `EDITTEXT`, which draws its own text and is not owner-drawn at
/// all — and if one ever became one, `text` on `field_bg` is what `WM_CTLCOLORSTATIC` already
/// answers for it.
fn label_ink(role: StaticColorRole, palette: &theme::Palette) -> COLORREF {
    match role {
        StaticColorRole::Label | StaticColorRole::Field => palette.text,
        StaticColorRole::Muted => palette.text_muted,
    }
}

/// The `SS_OWNERDRAW` labels of the settings template — FR-92а, task T-11-18.
///
/// The single place the eighteen are listed on the drawing side, and the identifier gate the
/// last sentence of SEC-05 asks for: a `WM_DRAWITEM` naming `ODT_STATIC` and anything else is
/// refused before a DC is touched. Mirrored by hand from the `LTEXT` rows of `app.rc`, exactly
/// as [`GROUP_BOXES`] and [`COMBO_BOXES`] are, and a test compares this list against the
/// statics of the **built** template so the two cannot drift apart.
///
/// ⚠ This is a list of *controls*, not of colours: the colour role of every one of them comes
/// from [`static_color_role`], which task T-11-4 wrote and this task reuses unchanged.
pub const OWNER_DRAWN_LABELS: [i32; 18] = [
    IDC_LANGUAGE_LABEL,
    IDC_THEME_LABEL,
    IDC_LANGUAGE_RESTART,
    IDC_HOTKEY_LABEL,
    IDC_HOTKEY_NOTE,
    IDC_PAIR_SOURCE_LABEL,
    IDC_PAIR_TARGET_LABEL,
    IDC_CYCLE_HINT,
    IDC_LAYOUT_NOTE,
    IDC_DELAY_LABEL,
    IDC_CLIP_TIMEOUT_LABEL,
    IDC_CLIP_RESTORE_LABEL,
    IDC_EXCLUSION_HINT,
    IDC_LOG_DIR_LABEL,
    IDC_LOG_DIR,
    IDC_STATE_HOOK,
    IDC_STATE_LAYOUTS,
    IDC_STATE_AUTOSTART,
];

/// The `SS_OWNERDRAW` labels of the about template — the same gate for the other window.
///
/// `IDC_ABOUT_ICON` is deliberately absent: it is an `SS_ICON` static, the type that loads the
/// 32 px frame of the `.ico`, it draws no text at all, and owner drawing it would only lose
/// the icon.
pub const OWNER_DRAWN_ABOUT_LABELS: [i32; 4] = [
    IDC_ABOUT_NAME,
    IDC_ABOUT_VERSION,
    IDC_ABOUT_LINE_1,
    IDC_ABOUT_LINE_2,
];

/// The `DrawTextW` format of every owner-drawn label — FR-92а, task T-11-18.
///
/// Byte for byte what an `SS_LEFT` static drew these labels with before the task, which is the
/// whole requirement: the smoothing changes and nothing else does.
///
/// - `DT_LEFT | DT_TOP` — the alignment of `SS_LEFT`: against the left edge, against the top,
///   never centred. Both are zero, and both are written out because a format built out of
///   silence is a format nobody can read.
/// - **`DT_WORDBREAK`** — the one flag that is load-bearing rather than cosmetic. Two labels of
///   the settings template are two lines high and their text does not fit on one: the note
///   «вступит в силу после перезапуска» (`IDC_LANGUAGE_RESTART`, 70 × 18 dialog units) and the
///   journal path (`IDC_LOG_DIR`, 186 × 16). A static wraps them; without this flag they would
///   come back as one clipped line, and the wrap is a regression the task names by name.
/// - `DT_EXPANDTABS` — also what the static did. No string of either locale carries a tab
///   today, so it changes no pixel today; it is here so that one arriving tomorrow lands the
///   same way it would have before the task.
///
/// No `DT_NOPREFIX`: `LTEXT` carries no `SS_NOPREFIX`, so an `&` in a label was an underscore
/// before this task and stays one after it — the journal path is the one label a stray `&`
/// could ever reach, and changing what it does there would be a change of behaviour smuggled
/// in under a change of smoothing.
pub const LABEL_TEXT_FORMAT: DRAW_TEXT_FORMAT =
    DRAW_TEXT_FORMAT(DT_LEFT.0 | DT_TOP.0 | DT_WORDBREAK.0 | DT_EXPANDTABS.0);

/// The face of one owner-drawn button, named as the palette field it is filled with —
/// FR-92а, task T-11-5a.
///
/// Roles and not colours, exactly as [`StaticColorRole`] before it: the mapping stays a
/// pure function a table test can close, and the `WM_DRAWITEM` handler turns a role into
/// a brush of the resolved palette in one place. Not a single colour number enters this
/// module — §6.2 gives every palette value to `theme` alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonFaceRole {
    /// [`theme::Palette::button_bg`] — the ordinary face.
    ButtonBg,
    /// [`theme::Palette::accent_bg`] — the accented face of the default button «ОК».
    AccentBg,
    /// [`theme::Palette::sel_bg`] — the face while the button is held pressed: a step
    /// darker in the light palette and lighter in the dark one, with no new colour in the
    /// palette.
    SelBg,
}

/// The ink the button's caption is drawn with, named as the palette field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonTextRole {
    /// [`theme::Palette::text`] — the ordinary caption.
    Text,
    /// [`theme::Palette::accent_fg`] — the caption over the accent face.
    AccentFg,
    /// [`theme::Palette::sel_fg`] — the caption over the pressed face, the pair the
    /// palette designed for exactly this ground.
    SelFg,
    /// [`theme::Palette::text_muted`] — the caption of a disabled button.
    TextMuted,
}

/// The frame of the button. One variant on purpose: the closed table of
/// [`button_color_roles`] frames every kind in every state with the same single-pixel
/// [`theme::Palette::button_border`], and a one-variant type is that fact written down —
/// a second border colour cannot appear without widening this enum first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonBorderRole {
    /// [`theme::Palette::button_border`].
    ButtonBorder,
}

/// Face, caption ink and frame of one owner-drawn button — what [`button_color_roles`]
/// answers and the whole of what the `WM_DRAWITEM` handler needs to choose colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonColors {
    /// What the button face is filled with.
    pub face: ButtonFaceRole,
    /// What the caption is drawn with.
    pub text: ButtonTextRole,
    /// What the single-pixel frame is drawn with.
    pub border: ButtonBorderRole,
}

/// The colour roles of one button in one state — FR-92а, task T-11-5a, the pure half of
/// `WM_DRAWITEM`, closed by a table test: обычная/умолчательная × нормальная/нажатая/
/// запрещённая.
///
/// The default button is recognised by its identifier — `IDOK`, the one control
/// `DM_SETDEFID` names on `WM_INITDIALOG` — not by `ODS_DEFAULT`, which the manager juggles
/// as the focus moves and which would make the accent wander off «ОК».
///
/// The order of the arms is the precedence:
/// - **запрещённость первой**: a disabled button takes no Enter and no click, so it must
///   not advertise itself — the accent face yields to the ordinary one and the ink goes
///   muted, whichever button it is («Выше»/«Ниже» are the ones actually seen grey);
/// - **нажатие второй**: a held button shows the selection pair `sel_bg`/`sel_fg` of the
///   palette, default and ordinary alike — the accent yields for the length of the press;
/// - the accent itself is the *normal* state of «ОК» and nothing else.
///
/// Focus is deliberately absent here: `ODS_FOCUS` changes no colour — it adds the dotted
/// `DrawFocusRect` frame on top of whatever face this table chose, and that is the drawing
/// half's business.
pub fn button_color_roles(control: i32, pressed: bool, disabled: bool) -> ButtonColors {
    let border = ButtonBorderRole::ButtonBorder;

    if disabled {
        return ButtonColors {
            face: ButtonFaceRole::ButtonBg,
            text: ButtonTextRole::TextMuted,
            border,
        };
    }

    if pressed {
        return ButtonColors {
            face: ButtonFaceRole::SelBg,
            text: ButtonTextRole::SelFg,
            border,
        };
    }

    if control == OK_COMMAND {
        return ButtonColors {
            face: ButtonFaceRole::AccentBg,
            text: ButtonTextRole::AccentFg,
            border,
        };
    }

    ButtonColors {
        face: ButtonFaceRole::ButtonBg,
        text: ButtonTextRole::Text,
        border,
    }
}

/// The colours of one owner-drawn push button, resolved out of the palette: the face brush,
/// the caption ink and the frame brush. What [`resolve_button_colors`] answers and the
/// whole of what [`paint_push_button`] needs.
#[derive(Clone, Copy)]
struct ResolvedButtonColors {
    /// What the button face is filled with.
    face: HBRUSH,
    /// What the caption is drawn with.
    ink: COLORREF,
    /// What the single-pixel frame is drawn with. An ink and not a brush since task
    /// T-11-13: the frame and the face are one rounded `RoundRect` now, and `RoundRect`
    /// frames with the selected *pen*.
    border: COLORREF,
}

/// The single place a button role becomes a brush or a colour of the resolved palette —
/// the drawing never sees a role. One body shared by the settings dialog and the about
/// dialog (§6.2, task T-11-11), moved out of `on_draw_item` rather than copied.
fn resolve_button_colors(
    colors: ButtonColors,
    brushes: &theme::Brushes,
    palette: &theme::Palette,
) -> ResolvedButtonColors {
    let face = match colors.face {
        ButtonFaceRole::ButtonBg => brushes.button_bg(),
        ButtonFaceRole::AccentBg => brushes.accent_bg(),
        ButtonFaceRole::SelBg => brushes.sel_bg(),
    };

    let ink = match colors.text {
        ButtonTextRole::Text => palette.text,
        ButtonTextRole::AccentFg => palette.accent_fg,
        ButtonTextRole::SelFg => palette.sel_fg,
        ButtonTextRole::TextMuted => palette.text_muted,
    };

    let border = match colors.border {
        ButtonBorderRole::ButtonBorder => palette.button_border,
    };

    ResolvedButtonColors { face, ink, border }
}

/// The two kinds of owner-drawn glyph element — FR-92а, task T-11-5b.
///
/// The kind decides the shape of the glyph: a check box is the 13×13 square with the
/// two-stroke check mark, a radio button is the circle with the dot. It is also the first
/// axis of the closed 2×2×2 table of [`glyph_color_roles`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphKind {
    /// The three check boxes of the dialog.
    CheckBox,
    /// The five radio buttons — the mode pair and the three replacement methods.
    RadioButton,
}

/// What the box or circle of the glyph is filled with, named as the palette field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphFillRole {
    /// [`theme::Palette::field_bg`] — the quiet ground of every glyph but one.
    FieldBg,
    /// [`theme::Palette::accent_bg`] — the checked, enabled check box, filled whole.
    AccentBg,
}

/// The frame of the glyph. One variant on purpose, exactly as [`ButtonBorderRole`]: the
/// closed table frames every framed cell with the same [`theme::Palette::box_border`], and
/// a second frame colour cannot appear without widening this enum first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphFrameRole {
    /// [`theme::Palette::box_border`] — the field the palette names for exactly this.
    BoxBorder,
}

/// The mark inside a checked glyph — the check strokes of a box, the dot of a radio —
/// named as the palette field it is cut from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphMarkRole {
    /// [`theme::Palette::accent_fg`] — the check mark over the accent-filled box.
    AccentFg,
    /// [`theme::Palette::accent_bg`] — the dot of a checked, enabled radio button.
    AccentBg,
    /// [`theme::Palette::box_border`] — the muted mark of a checked but disabled glyph.
    BoxBorder,
}

/// The ink of the caption to the right of the glyph, named as the palette field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphTextRole {
    /// [`theme::Palette::text`] — the ordinary caption.
    Text,
    /// [`theme::Palette::text_muted`] — the caption of a disabled element (the method
    /// radios are the ones actually seen grey, under the «Пара» mode).
    TextMuted,
}

/// Fill, frame, mark and caption ink of one owner-drawn glyph element — what
/// [`glyph_color_roles`] answers and the whole of what the drawing half needs to choose
/// colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphColors {
    /// What the box or circle is filled with.
    pub fill: GlyphFillRole,
    /// The single-pixel frame — `None` for the one cell the accent fill covers whole.
    pub frame: Option<GlyphFrameRole>,
    /// The mark of a checked glyph — `None` while unchecked.
    pub mark: Option<GlyphMarkRole>,
    /// What the caption is drawn with.
    pub text: GlyphTextRole,
}

/// The colour roles of one glyph in one state — FR-92а, task T-11-5b, the pure half of the
/// glyph drawing, closed by a 2×2×2 table test: флажок/переключатель × взведён/снят ×
/// разрешён/запрещён.
///
/// Roles and not colours, exactly as [`button_color_roles`] before it: not a single colour
/// number enters this module — §6.2 gives every palette value to `theme` alone.
///
/// The shape of the table:
/// - **the checked, enabled check box** is the one cell the accent covers whole: `accent_bg`
///   fill edge to edge, `accent_fg` check mark, and no frame — the task words it so;
/// - **the checked, enabled radio** keeps the quiet ground and shows the accent as the dot:
///   an accent-filled circle would hide an accent dot, so the accent moves inside;
/// - **запрещённость гасит акцент**, the precedent [`button_color_roles`] set: a disabled
///   element must not advertise itself, so a checked-but-disabled glyph drops to the
///   `box_border` mark on the ordinary ground — still readable as checked, no longer loud;
/// - every other cell is the quiet ground itself: `field_bg` fill, `box_border` frame,
///   no mark.
///
/// Focus is deliberately absent here, as it is in [`button_color_roles`]: `ODS_FOCUS`
/// changes no colour — it adds the dotted `DrawFocusRect` frame around the caption, and
/// that is the drawing half's business. The pressed state is absent too: the task's table
/// is (вид, взведён, запрещён), and the glyph the eye needs to trust is the check state,
/// not the length of a button press.
pub fn glyph_color_roles(kind: GlyphKind, checked: bool, disabled: bool) -> GlyphColors {
    let text = if disabled {
        GlyphTextRole::TextMuted
    } else {
        GlyphTextRole::Text
    };

    if checked && !disabled {
        return match kind {
            GlyphKind::CheckBox => GlyphColors {
                fill: GlyphFillRole::AccentBg,
                frame: None,
                mark: Some(GlyphMarkRole::AccentFg),
                text,
            },
            GlyphKind::RadioButton => GlyphColors {
                fill: GlyphFillRole::FieldBg,
                frame: Some(GlyphFrameRole::BoxBorder),
                mark: Some(GlyphMarkRole::AccentBg),
                text,
            },
        };
    }

    GlyphColors {
        fill: GlyphFillRole::FieldBg,
        frame: Some(GlyphFrameRole::BoxBorder),
        // A checked glyph keeps its mark when disabled — the state stays readable — but
        // the mark goes muted with everything else.
        mark: if checked {
            Some(GlyphMarkRole::BoxBorder)
        } else {
            None
        },
        text,
    }
}

/// The glyph kind of one control identifier, `None` for everything that is not one of the
/// eight owner-drawn check boxes and radio buttons — FR-92а, task T-11-5b.
///
/// The single place the eight are listed on the drawing side; the `WM_DRAWITEM` handler
/// branches on this before its push-button path.
fn glyph_kind(control: i32) -> Option<GlyphKind> {
    match control {
        IDC_AUTOSTART | IDC_SELECTION_ENABLED | IDC_LOG_ENABLED => Some(GlyphKind::CheckBox),
        IDC_MODE_PAIR | IDC_MODE_CYCLE | IDC_METHOD_AUTO | IDC_METHOD_BACKSPACE
        | IDC_METHOD_SELECTION => Some(GlyphKind::RadioButton),
        _ => None,
    }
}

/// The eight group panels of FR-92 — FR-92а, tasks T-11-5c and T-11-13.
///
/// The single place the eight are listed. Since task T-11-13 they are not visible elements
/// at all (`NOT WS_VISIBLE` in `app.rc`) and no `WM_DRAWITEM` can reach them: what reads
/// this list now is [`background_figure`], which tells the dialog's own background drawing
/// that the rectangle of one of these controls is a panel — and [`collect_panel_children`],
/// which reads it to split the dialog's children into the panels and the controls that may
/// lie on one.
pub const GROUP_BOXES: [i32; 8] = [
    IDC_GROUP_GENERAL,
    IDC_GROUP_HOTKEY,
    IDC_GROUP_LAYOUTS,
    IDC_GROUP_REPLACEMENT,
    IDC_GROUP_SELECTION,
    IDC_GROUP_EXCLUSIONS,
    IDC_GROUP_DIAGNOSTICS,
    IDC_GROUP_STATE,
];

/// The identifiers of the controls whose rectangle lies on one of the panels — FR-92а,
/// task T-11-5c, the pure half of the panel map.
///
/// A control is «on a panel» when its rectangle is contained in a panel's rectangle,
/// **boundary included**: an element flush with the panel's edge sits on the panel to the
/// eye, so touching an edge from the inside counts as inside. A rectangle that crosses an
/// edge — partly on the panel, partly off it — is not on the panel: no control of the
/// template does that, and a half-on element painted with the panel brush would carry the
/// panel colour outside the panel.
///
/// Pure on purpose, and the coordinate space is deliberately nobody's business: the
/// containment answer is the same in screen pixels, client pixels or synthetic numbers,
/// which is what lets the table test of criterion 10 close this function on rectangles it
/// makes up. The caller passes the eight panel rectangles and the rectangles of every
/// *other* child — the panels themselves are not candidates: a panel's own erase must stay
/// the window brush, or its rounded corners would sit on panel colour instead of the
/// window they are cut from.
pub fn controls_on_panels(panels: &[RECT], controls: &[(i32, RECT)]) -> Vec<i32> {
    controls
        .iter()
        .filter(|(_, control)| {
            panels.iter().any(|panel| {
                panel.left <= control.left
                    && control.right <= panel.right
                    && panel.top <= control.top
                    && control.bottom <= panel.bottom
            })
        })
        .map(|(id, _)| *id)
        .collect()
}

/// Walks the dialog's children once and answers the panel map — the impure shell around
/// [`controls_on_panels`], run on `WM_INITDIALOG` and never again.
///
/// The rectangles are `GetWindowRect` screen rectangles for panels and candidates alike —
/// one coordinate space, which is all containment needs. Every child of the dialog is a
/// candidate except the eight panels themselves (see [`controls_on_panels`] for why).
///
/// NFR-13: both calls are examined. `GetWindow` answering an error is the documented end
/// of the walk — a window with no more children is not a failure of anything. A refused
/// `GetWindowRect` skips that one control: it then keeps the window-coloured background it
/// had before this task, which is the same degraded-but-alive answer the refused-brushes
/// dialog gives (T-11-4), and the journal has no row for GDI refusals (reviews\T-11-1.md).
fn collect_panel_children(hwnd: HWND) -> Vec<i32> {
    let mut panels: Vec<RECT> = Vec::with_capacity(GROUP_BOXES.len());
    let mut candidates: Vec<(i32, RECT)> = Vec::new();

    for (control, rect) in child_rects(hwnd) {
        if GROUP_BOXES.contains(&control) {
            panels.push(rect);
        } else {
            candidates.push((control, rect));
        }
    }

    controls_on_panels(&panels, &candidates)
}

/// Every child of the dialog, with its identifier and its **screen** rectangle, in Z order.
///
/// The one walk of the dialog's children (§6.2): [`collect_panel_children`] splits its
/// answer into panels and candidates, and [`child_rects_in_client`] moves it into the
/// dialog's client coordinates for the background drawing. Invisible children — the eight
/// panels since task T-11-13 — are in the answer like any other: a window that is not shown
/// still has the rectangle the template gave it, which is exactly what both callers want of
/// the panels.
///
/// NFR-13: both calls are examined. `GetWindow` answering an error is the documented end of
/// the walk — a window with no more children is not a failure of anything. A refused
/// `GetWindowRect` leaves that one control out: it then keeps the window-coloured ground it
/// had before task T-11-5c and no frame of its own, which is the degraded-but-alive answer
/// this file gives everywhere, and the journal has no row for GDI refusals
/// (reviews\T-11-1.md).
fn child_rects(hwnd: HWND) -> Vec<(i32, RECT)> {
    let mut children: Vec<(i32, RECT)> = Vec::new();

    // SAFETY: `hwnd` is the live dialog; the call reads a window field and no memory of
    // ours, and answers a handle or an error.
    let mut child = unsafe { GetWindow(hwnd, GW_CHILD) }.ok();

    while let Some(window) = child {
        // SAFETY: `window` is the live child the walk just answered; asking for its
        // identifier reads a field of that window.
        let control = unsafe { GetDlgCtrlID(window) };

        let mut rect = RECT::default();

        // SAFETY: `window` is the live child and `rect` is a live local the call fills;
        // nothing else is written.
        if unsafe { GetWindowRect(window, &mut rect) }.is_ok() {
            children.push((control, rect));
        }

        // SAFETY: as for the first step of the walk.
        child = unsafe { GetWindow(window, GW_HWNDNEXT) }.ok();
    }

    children
}

/// Every child of the dialog, with its identifier and its rectangle **in the client
/// coordinates of the dialog** — what the background drawing of task T-11-13 paints in.
///
/// `GetWindowRect` speaks screen coordinates and a `WM_ERASEBKGND` DC speaks the client
/// coordinates of the window being erased, so the whole set is shifted by one vector: the
/// screen position of the dialog's own client origin, asked for once with `ClientToScreen`
/// instead of once per child.
///
/// NFR-13: a refused `ClientToScreen` answers an empty list rather than a list in the wrong
/// coordinate space — a background drawn at an offset would be worse than none at all, and
/// the caller then leaves the erase to the dialog manager.
fn child_rects_in_client(hwnd: HWND) -> Vec<(i32, RECT)> {
    let mut origin = POINT { x: 0, y: 0 };

    // SAFETY: `hwnd` is the live dialog and `origin` is a live local the call rewrites in
    // place; nothing else is written.
    if !unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
        return Vec::new();
    }

    child_rects(hwnd)
        .into_iter()
        .map(|(control, rect)| {
            (
                control,
                RECT {
                    left: rect.left - origin.x,
                    top: rect.top - origin.y,
                    right: rect.right - origin.x,
                    bottom: rect.bottom - origin.y,
                },
            )
        })
        .collect()
}

/// The four owner-drawn combo boxes of FR-92 — FR-92а, task T-11-6.
///
/// The single place the four are listed: both handlers of the owner-draw pair —
/// [`on_measure_item`] and the combo branch of [`on_draw_item`] — check the incoming
/// `CtlID` against this list **before any work**, and answer «not handled» for anything
/// else. SEC-05 words the allowance around the identifier, and this list is that
/// identifier check written down once.
const COMBO_BOXES: [i32; 4] = [IDC_LANGUAGE, IDC_THEME, IDC_PAIR_SOURCE, IDC_PAIR_TARGET];

/// What the ground of one combo-box item is filled with, named as the palette field —
/// FR-92а, task T-11-6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboFillRole {
    /// [`theme::Palette::field_bg`] — the quiet ground of an ordinary list item and of the
    /// closed face: a combo is a field to the eye, wherever it sits.
    FieldBg,
    /// [`theme::Palette::sel_bg`] — the highlighted item of the dropped-down list.
    SelBg,
}

/// The ink the item's text is drawn with, named as the palette field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboTextRole {
    /// [`theme::Palette::text`] — the ordinary item and the closed face.
    Text,
    /// [`theme::Palette::sel_fg`] — the pair the palette designed for the `sel_bg` ground.
    SelFg,
    /// [`theme::Palette::text_muted`] — the value shown by a **disabled** closed face
    /// (task T-11-14). Never answered for an item of a dropped-down list: a list that can be
    /// dropped down at all belongs to a combo that is not disabled.
    TextMuted,
}

/// Ground and ink of one combo-box item — what [`combo_item_color_roles`] answers and the
/// whole of what the combo branch of `WM_DRAWITEM` needs to choose colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComboItemColors {
    /// What the item's rectangle is filled with.
    pub fill: ComboFillRole,
    /// What the item's text is drawn with.
    pub text: ComboTextRole,
}

/// The colour roles of one combo-box item in one state — FR-92а, task T-11-6, the pure
/// half of the combo drawing, closed by a 2×2 table test: пункт списка/закрытая часть ×
/// обычный/подсвеченный.
///
/// Roles and not colours, exactly as [`button_color_roles`] and [`glyph_color_roles`]
/// before it: not a single colour number enters this module — §6.2 gives every palette
/// value to `theme` alone.
///
/// The shape of the table:
/// - **закрытая часть первой**: the closed face of the combo is a field to the eye, so it
///   keeps `field_bg`/`text` whatever `ODS_SELECTED` says — the manager marks the face
///   selected whenever the combo holds the focus, and a face that flipped to the selection
///   pair would sit on the dialog as a permanently lit stripe. Focus is shown by the
///   dotted `DrawFocusRect` alone, which is the drawing half's business — the same
///   decision [`button_color_roles`] wrote down for `ODS_FOCUS`;
/// - the **highlighted item** of the dropped-down list shows the selection pair
///   `sel_bg`/`sel_fg` of the palette — the same pair every owner-drawn element of this
///   dialog highlights with;
/// - the ordinary item is the quiet ground of the list: `field_bg`/`text` — the very
///   colours `WM_CTLCOLORLISTBOX` has erased the dropped list with since T-11-4, so an
///   item and the list around it are one surface.
pub fn combo_item_color_roles(closed_part: bool, highlighted: bool) -> ComboItemColors {
    if !closed_part && highlighted {
        return ComboItemColors {
            fill: ComboFillRole::SelBg,
            text: ComboTextRole::SelFg,
        };
    }

    ComboItemColors {
        fill: ComboFillRole::FieldBg,
        text: ComboTextRole::Text,
    }
}

/// The colours of one row of the exclusion list — FR-92а, task T-11-14.
///
/// The very table the combo items answer, **reused and not copied** (§6.2): a row of a list
/// is the quiet `field_bg`/`text` ground, and the selected row is the selection pair
/// `sel_bg`/`sel_fg` — the same pair every owner-drawn element of this dialog highlights
/// with, and the whole point of taking the rows away from the system, whose selection stripe
/// is `COLOR_HIGHLIGHT` blue in both palettes. `closed_part` is `false` because a list box
/// has no closed part; that argument exists for the combo alone.
pub fn list_item_color_roles(selected: bool) -> ComboItemColors {
    combo_item_color_roles(false, selected)
}

/// The frame around the closed part of a combo box, named as the palette field — FR-92а,
/// task T-11-14.
///
/// One variant on purpose, exactly as [`ButtonBorderRole`] and [`GlyphFrameRole`] before it:
/// the closed table frames the closed face with the same [`theme::Palette::field_border`]
/// every other field of the dialog is framed with, and a second frame colour cannot appear
/// without widening this enum first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboBorderRole {
    /// [`theme::Palette::field_border`] — the frame the mock-ups draw around every field.
    FieldBorder,
}

/// The ink of the chevron at the right edge of a closed combo box — FR-92а, task T-11-14.
///
/// One variant, for the reason [`ComboBorderRole`] gives: the mock-ups draw the chevron in
/// the muted ink in both palettes and in every state, disabled included — it is a hint of
/// what the control does, never an advertisement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboChevronRole {
    /// [`theme::Palette::text_muted`] — the quiet ink of the mock-ups' chevron.
    TextMuted,
}

/// Ground, frame, ink and chevron of the **closed part** of one combo box — what
/// [`combo_closed_color_roles`] answers and the whole of what the subclass painting of
/// task T-11-14 needs to choose colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComboClosedColors {
    /// What the rounded rectangle of the closed face is filled with.
    pub fill: ComboFillRole,
    /// The single-pixel frame around that rectangle.
    pub border: ComboBorderRole,
    /// What the text of the chosen value is drawn with.
    pub text: ComboTextRole,
    /// What the chevron is stroked with.
    pub chevron: ComboChevronRole,
}

/// The colour roles of the closed part of a combo box in one state — FR-92а, task T-11-14,
/// the pure half of the subclass drawing, closed by a table test: разрешён / запрещён.
///
/// Roles and not colours, exactly as [`button_color_roles`], [`glyph_color_roles`] and
/// [`combo_item_color_roles`] before it: not a single colour number enters this module —
/// §6.2 gives every palette value to `theme` alone.
///
/// The shape of the table:
/// - the closed face **is a field to the eye**, so it keeps `field_bg` under the
///   `field_border` frame whatever else is true — the same surface the five input fields and
///   the two lists of [`FRAMED_FIELDS`] wear since task T-11-13;
/// - **запрещённость гасит текст**, the precedent [`button_color_roles`] and
///   [`glyph_color_roles`] set: a combo the mode has switched off (the pair «Источник» and
///   «Цель» under «Несколько раскладок») shows its value in `text_muted`;
/// - the chevron and the frame do not move with the state: a muted hint stays muted, and a
///   field that lost its frame when disabled would stop reading as a field at all.
///
/// Focus is deliberately absent here, exactly as it is in [`button_color_roles`]: `ODS_FOCUS`
/// changes no colour — it adds the dotted `DrawFocusRect`, and that is the drawing half's
/// business (the decision of task T-11-5a, kept uniform here).
pub fn combo_closed_color_roles(disabled: bool) -> ComboClosedColors {
    ComboClosedColors {
        fill: ComboFillRole::FieldBg,
        border: ComboBorderRole::FieldBorder,
        text: if disabled {
            ComboTextRole::TextMuted
        } else {
            ComboTextRole::Text
        },
        chevron: ComboChevronRole::TextMuted,
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

            // FR-92а, task T-11-5c: the ground of this control — the panel brush for a
            // control lying on one of the eight group panels, the window brush everywhere
            // else. Geometry decided the list once, on `WM_INITDIALOG`; only the ground
            // changes with it — the text roles and the field answers below do not.
            let ground = if state.panel_children.contains(&control_id) {
                brushes.panel_bg()
            } else {
                brushes.window_bg()
            };

            Some(match message {
                // The background of the dialog itself: no text ever lands on this DC, so
                // the brush is the whole answer — and it is always the window brush; the
                // dialog is the ground the panels themselves sit on.
                WM_CTLCOLORDLG => (None, None, brushes.window_bg()),

                // Captions and notes — and the one read-only field the message files under
                // static. The role decides the ink; the caption roles take no opaque
                // background, so the text sits transparently on the ground brush instead
                // of in a box of a slightly different colour.
                //
                // ⚠ Still answered for the eighteen labels since task T-11-18, even though
                // they are `SS_OWNERDRAW` and paint themselves: a static asks this question
                // before it hands the drawing over, the answer costs a table lookup, and the
                // ground and the ink it names are the very ones `draw_label` paints with —
                // `static_color_role` is asked by both, so the two cannot disagree. What the
                // message is still load-bearing *for* is `IDC_HOTKEY`: a read-only `EDIT` is
                // asked about with this message and has no owner drawing to take over from it.
                WM_CTLCOLORSTATIC => match static_color_role(control_id) {
                    StaticColorRole::Label => (Some(palette.text), None, ground),
                    StaticColorRole::Muted => (Some(palette.text_muted), None, ground),
                    StaticColorRole::Field => (
                        Some(palette.text),
                        Some(palette.field_bg),
                        brushes.field_bg(),
                    ),
                },

                // Input fields, the exclusion list box — and the dropped-down list of
                // every combo box, whose list window sends this message to the dialog too.
                // Deliberately not panel-aware: a field is `field_bg` wherever it sits.
                WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => (
                    Some(palette.text),
                    Some(palette.field_bg),
                    brushes.field_bg(),
                ),

                // The erase under every owner-drawn button — the ground the glyphs of
                // T-11-5b and the buttons of T-11-5a are painted over, and what the rounded
                // corners of a button are cut from (task T-11-13). The eight panels no
                // longer ask this question at all: an invisible window is never erased.
                WM_CTLCOLORBTN => (None, None, ground),

                // Unreachable: the caller only routes the five messages above here.
                _ => return None,
            })
        })
    };

    apply_ctl_color(dc, choice.flatten())
}

/// One `WM_CTLCOLOR*` answer, chosen but not yet applied: the caption ink (`None` — no text
/// on this DC), the opaque text background of a field (`None` — transparent), and the brush
/// the control is erased with.
type CtlColorChoice = (Option<COLORREF>, Option<COLORREF>, HBRUSH);

/// Applies one chosen `WM_CTLCOLOR*` answer to the DC — the drawing half [`on_ctl_color`]
/// and [`on_about_ctl_color`] share (§6.2, task T-11-11: one body, not a copy).
///
/// `None` is the «not handled» of both callers — the system colours — and answers zero.
fn apply_ctl_color(dc: HDC, choice: Option<CtlColorChoice>) -> isize {
    let Some((ink, opaque_bg, brush)) = choice else {
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

/// Air added to the dialog font's height to make one owner-drawn item's height, in the pixels
/// of the mock-ups — п. 4 of task T-11-15, in place of the bare «+ 4» of task T-11-6.
///
/// The mock-ups draw the closed part of a combo box **33 of their own pixels high**, and the
/// dialog font is 21 of their pixels tall there (15 px at 96 DPI × 1,4) — so the air of the
/// picture is 33 − 21 = **12 mock-up pixels**, six above the text and six below. Through
/// [`scaled`] that is 9 px at 96 DPI, and the item stands 15 + 9 = **24 px** against the
/// picture's 33 ÷ 1,4 = 23,6. The bare 4 it replaces made a 19-pixel item — a fifth shorter
/// than the mock-up, which is the «расходится существенно» п. 4 asks to be brought in line.
///
/// One number for three controls, as since task T-11-14: the rows of the exclusion list and
/// the items of a dropped-down list are measured by the same message, so a row of a list and
/// an item of a combo stay the same height to the eye.
pub const COMBO_ITEM_EXTRA: i32 = 12;

/// Whether this `CtlType`/`CtlID` pair is one of the dialog's own owner-drawn item holders —
/// the identifier gate SEC-05 asks for, written down once and shared by `WM_MEASUREITEM` and
/// the item branches of `WM_DRAWITEM`.
///
/// `ctl_type` is the plain number of the message, so the gate is a pure function a table test
/// can drive: [`ODT_COMBOBOX`] for the four combo boxes of [`COMBO_BOXES`] (task T-11-6),
/// [`ODT_LISTBOX`] for the single exclusion list (task T-11-14), and nothing else — a foreign
/// type or a foreign identifier is «not handled» before any work is done.
pub fn owner_drawn_item(ctl_type: u32, control: i32) -> bool {
    if ctl_type == ODT_COMBOBOX.0 {
        return COMBO_BOXES.contains(&control);
    }

    if ctl_type == ODT_LISTBOX.0 {
        return control == IDC_EXCLUSIONS;
    }

    false
}

/// Answers `WM_MEASUREITEM` for the four owner-drawn combo boxes — FR-92а, task T-11-6.
///
/// # SEC-05, held to the letter
///
/// `lparam` points at the `MEASUREITEMSTRUCT` of the message. Two fields are read out of
/// it — `CtlType` and `CtlID`, the checks that this is one of our four combo boxes,
/// **both made before any work** — and exactly one is written: `itemHeight`, which is the
/// documented protocol of the message. `itemData` is never read; no pointer of the message
/// is followed anywhere. The window of time is the gate, as the last sentence of SEC-05
/// words it: this message is handled by the dialog procedure alone, and the dialog window
/// exists exactly while the program itself holds the dialog on the screen. A forged
/// message therefore buys its sender a height written back into the sender's own struct,
/// and nothing else.
///
/// # The height
///
/// The item height is the dialog font's height plus [`COMBO_ITEM_EXTRA`] **mock-up** pixels
/// of air, taken through [`scaled`] at the DPI of the dialog's own DC (task T-11-15: the air
/// is a length of the picture like every other, and until this task it was a bare 4 that
/// neither followed the picture nor the DPI). The message arrives while the combo is being
/// created — *before* `WM_INITDIALOG`, so
/// before the dialog's state exists — but after the dialog has taken its `DS_SETFONT`
/// font, which the dialog manager passes to the dialog before creating any control. The
/// font's height is read through the documented `MapDialogRect`: the vertical dialog base
/// unit is defined as the height of the dialog font and one vertical dialog unit as one
/// eighth of it, so a rectangle 8 units tall maps to exactly one font height in pixels —
/// no font handle changes hands and no message is sent.
///
/// Answers 1 — «measured» — for one of the four combos. 0 — «not handled», the dialog
/// manager's default path — for a missing struct, a foreign `CtlType` or a foreign
/// identifier, and for a refused `MapDialogRect` (NFR-13: examined in words and answered
/// by falling back — the control's default item height is a legal, degraded-but-alive
/// answer, and the journal has no row for GDI refusals, reviews\T-11-1.md).
///
/// # Safety
///
/// Called from [`dialog_proc`] only, with the `lparam` of the message: the sender owns
/// the struct for the length of the send, and this procedure is inside that send.
unsafe fn on_measure_item(hwnd: HWND, lparam: LPARAM) -> isize {
    if lparam.0 == 0 {
        return 0;
    }

    // The one dereference of the message: two fields read, `itemHeight` written at the
    // end. `itemData` — never.
    //
    // SAFETY: see above.
    let item = unsafe { &mut *(lparam.0 as *mut MEASUREITEMSTRUCT) };

    // SEC-05: the type and the identifier are checked before any work — the four combo boxes
    // of task T-11-6 and, since task T-11-14, the one owner-drawn list box.
    if !owner_drawn_item(item.CtlType.0, i32::try_from(item.CtlID).unwrap_or(-1)) {
        return 0;
    }

    // The exclusion list has a row height of its own since task T-11-16: the generator gives
    // its `'lbox'` rows `Y 11` — [`EXCLUSION_ROW_HEIGHT_DLU`] vertical dialog units — where a
    // combo box has no height in the mock-ups at all and keeps the font-plus-air of T-11-15.
    let measured = if item.CtlType.0 == ODT_LISTBOX.0 {
        dialog_units(hwnd, 0, EXCLUSION_ROW_HEIGHT_DLU).map(|(_, row)| row)
    } else {
        // Eight vertical dialog units are one dialog-font height by the definition of the
        // base units — see the doc comment above.
        dialog_units(hwnd, 0, DIALOG_FONT_HEIGHT_DLU).map(|(_, font_height)| {
            // The air of the mock-ups, in this window's pixels — п. 4 of task T-11-15.
            //
            // SAFETY: `hwnd` is the window being built; the call answers its DC or an invalid
            // handle, and the DC is released below on both paths.
            let dc = unsafe { GetDC(Some(hwnd)) };

            // NFR-13: examined — `dc_dpi` answers 96 for a DC that will not say, which is the
            // 100 % air and a legal item height.
            let extra = scaled(COMBO_ITEM_EXTRA, dc_dpi(dc));

            if !dc.is_invalid() {
                // SAFETY: releases exactly the DC taken above, once.
                unsafe { ReleaseDC(Some(hwnd), dc) };
            }

            font_height + extra
        })
    };

    let Some(measured) = measured else {
        return 0;
    };

    // A negative or overflowing height cannot come out of a font measurement; if a broken
    // one did, keeping the control's own default is the degraded-but-alive answer again.
    let Ok(height) = u32::try_from(measured) else {
        return 0;
    };

    item.itemHeight = height;

    // TRUE — measured.
    1
}

/// Whether one item change of the layout list is refused — FR-31, task T-11-7-2: the pure
/// half of the gate of [`on_notify`].
///
/// Since task T-11-7-2 the list is never `EnableWindow`-disabled — a disabled
/// `SysListView32` ignores its own colours and paints the system wash, the defect the final
/// sweep found — so the «выключенность» of the pair mode is this decision instead: in the
/// pair mode every change the control asks about — a tick toggled by `LVS_EX_CHECKBOXES`
/// with mouse or space bar, a selection or focus moved by mouse or arrow key — is refused
/// before it happens, and a click on the list changes nothing at all. In the cycle mode
/// every change passes, which is the path of FR-31 exactly as it was.
///
/// `None` is the mode of a moment with no readable state: before `WM_INITDIALOG` stores
/// the pointer, or — the case that matters — while a programmatic fill holds the state
/// borrowed ([`fill_layouts`] and [`move_cycle_row`] write the list from inside
/// [`with_state`], so the re-entrant `LVN_ITEMCHANGING` they raise finds the `RefCell`
/// busy). Those writes are the program's own and are allowed in either mode, which is what
/// keeps [`fill_cycle_list`] filling a pair-mode dialog.
pub fn cycle_list_change_is_refused(mode: Option<LayoutMode>) -> bool {
    mode == Some(LayoutMode::Pair)
}

/// Answers the two notifications of the layout list — FR-92а: `NM_CUSTOMDRAW` (task T-11-7,
/// the row paints of the palette; task T-11-7-2, the muted rows of the pair mode) and
/// `LVN_ITEMCHANGING` (task T-11-7-2, the gate of FR-31 — in the pair mode a click changes
/// nothing).
///
/// # SEC-05 — the checks come before any work
///
/// The first dereference of the message takes the three fields of `NMHDR` and nothing else:
/// `hwndFrom`, `idFrom`, `code`. All three are then compared — `code` against
/// `NM_CUSTOMDRAW` and `LVN_ITEMCHANGING`, `idFrom` against `IDC_CYCLE_LIST`, and `hwndFrom`
/// against the dialog's own `GetDlgItem` answer for that identifier — and only a message
/// that passes every one goes any further. `LVN_ITEMCHANGING` is then decided on the header
/// alone: not a byte of its payload is read. Only `NM_CUSTOMDRAW` reads on, as the
/// `NMLVCUSTOMDRAW` a list view's `NM_CUSTOMDRAW` documents.
/// Out of that structure exactly three fields are read — `dwDrawStage`, `dwItemSpec`,
/// `uItemState` — and three written — `clrText`, `clrTextBk`, `uItemState` — the documented
/// answer protocol of item prepaint; the `hdc` and the rectangle of the message are not
/// touched, and no pointer of the message is followed. Whether the row is selected is not
/// taken from the message either: it is the list's own `LVM_GETITEMSTATE` answer, asked by
/// identifier through [`send_to`] — the reading [`read_cycle_checks`] already does for the
/// ticks. A forged message can therefore recolour one repaint of its own list, or refuse
/// one change of it, and nothing else — no privileged action starts here, and the window of
/// time is the same gate as for `WM_DRAWITEM`: outside the modal call there is no procedure
/// to arrive at.
///
/// # The gate of the pair mode — task T-11-7-2
///
/// Since task T-11-7-2 the list is never `EnableWindow`-disabled (see [`enable_by_mode`]),
/// so the control answers clicks and keys in every mode — and `LVN_ITEMCHANGING` is the one
/// point where it *asks before it moves*: a TRUE answer refuses the change before it
/// happens. That is the gate: with the mode read through [`with_state`] — the same road the
/// palette takes — [`cycle_list_change_is_refused`] refuses every change in the pair mode,
/// which closes ticks (mouse and space bar) and selection (mouse and arrow keys) in one
/// move, before any action. An unreachable state — `None` — allows: that is a programmatic
/// fill running under the held borrow ([`fill_layouts`], [`move_cycle_row`] write the list
/// from inside `with_state`), and the program's own writes pass in either mode. The cycle
/// mode is untouched: the answer there is «not handled», as it always was.
///
/// # The manoeuvre for the row paints, and the two answers
///
/// `CDDS_PREPAINT` is answered with `CDRF_NOTIFYITEMDRAW` — «ask me again for each item» —
/// the documented door to item prepaint. There the row's paints are the answer of the pure
/// [`cycle_row_paint`] table: in the cycle mode a selected row takes `sel_bg`/`sel_fg` and
/// an ordinary one is left to the control's own colours (task T-11-7 as it was); in the
/// pair mode every row takes `field_bg`/`text_muted` — the logical «выключенность» of task
/// T-11-7-2, a selected row wearing the same muted paints as the rest. Wherever the table
/// says so, the `CDIS_SELECTED` bit is *removed* from `uItemState`: while that bit stands
/// the control paints the selection ground itself (the system highlight, or the theme's),
/// and the two colour fields lose to it. Clearing the bit in the item-prepaint answer is
/// the documented custom-draw lever for exactly this — the control then draws the row as an
/// ordinary one, with the colours just set; the row still *is* selected (`LVIS_SELECTED` is
/// untouched, `LVS_SHOWSELALWAYS` stays a style of the template). The answer is
/// `CDRF_DODEFAULT` — «draw it yourself, with the fields I set»; `CDRF_NEWFONT` is the
/// answer of a handler that swapped a font into the `hdc`, which this one never does.
/// Chosen by the documentation — the product is not launched by the executor; the eye check
/// is the controller's, at the final acceptance.
///
/// Answers through [`answer_notify`] — a dialog procedure hands a `WM_NOTIFY` result back
/// in two moves, not by return value. 0 — «not handled» — for every stage this handler has
/// no word for, for a message that fails the gate, and for the state being unreachable.
///
/// # Safety
///
/// Called from [`dialog_proc`] only, with the `lparam` of the message: the sender owns the
/// structure for the length of the send, and this procedure is inside that send.
unsafe fn on_notify(hwnd: HWND, lparam: LPARAM) -> isize {
    if lparam.0 == 0 {
        return 0;
    }

    // The first dereference: the three header fields, copied out as plain values.
    //
    // SAFETY: see above.
    let header = unsafe { &*(lparam.0 as *const NMHDR) };
    let (from_window, from_id, code) = (header.hwndFrom, header.idFrom, header.code);

    // SEC-05: the three comparisons, before any work.
    if (code != NM_CUSTOMDRAW && code != LVN_ITEMCHANGING)
        || from_id != usize::try_from(IDC_CYCLE_LIST).unwrap_or(usize::MAX)
    {
        return 0;
    }

    // SAFETY: `hwnd` is the live dialog; the call reads a window field and no memory of
    // ours, and answers a handle or an error.
    if unsafe { GetDlgItem(Some(hwnd), IDC_CYCLE_LIST) }.ok() != Some(from_window) {
        return 0;
    }

    // Task T-11-7-2, the gate of FR-31 — decided on the header alone, before any action
    // and without reading a byte of the payload (see the doc comment).
    if code == LVN_ITEMCHANGING {
        // SAFETY: see the caller.
        let mode = unsafe { with_state(hwnd, |state| state.working.layouts.mode) };

        if cycle_list_change_is_refused(mode) {
            // TRUE — the change is refused; the list stays exactly as drawn.
            return answer_notify(hwnd, 1);
        }

        // «Not handled» — the change proceeds: the cycle mode as it always was, and the
        // program's own writes under the held borrow.
        return 0;
    }

    // Only now the payload. The mutable reference is the answer protocol of the message:
    // the control reads the colour fields back when the send returns.
    //
    // SAFETY: see above — the header just checked is the head of this very structure.
    let draw = unsafe { &mut *(lparam.0 as *mut NMLVCUSTOMDRAW) };

    if draw.nmcd.dwDrawStage == CDDS_PREPAINT {
        return answer_notify(hwnd, isize::try_from(CDRF_NOTIFYITEMDRAW).unwrap_or(0));
    }

    if draw.nmcd.dwDrawStage != CDDS_ITEMPREPAINT {
        return 0;
    }

    // The list's own answer, not the message's: whether the row `dwItemSpec` names is
    // selected. An out-of-range index answers 0 — not selected — and costs nothing.
    let selected = u32::try_from(send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_GETITEMSTATE,
        draw.nmcd.dwItemSpec,
        isize::try_from(LVIS_SELECTED.0).unwrap_or(0),
    ))
    .unwrap_or(0)
        & LVIS_SELECTED.0
        != 0;

    // The choice, split from the write-back as everywhere in this file: the borrow of the
    // state ends before the message structure is touched. The palette is a `&'static`, so
    // copying the reference out of the borrow is sound.
    //
    // SAFETY: see the caller.
    let read = unsafe { with_state(hwnd, |state| (state.working.layouts.mode, state.palette)) };

    let Some((mode, palette)) = read else {
        // No state to choose from — «not handled», the control draws with its own colours
        // (NFR-13).
        return 0;
    };

    let paint = cycle_row_paint(mode, selected, palette);

    if let Some((ground, ink)) = paint.colours {
        draw.clrTextBk = ground;
        draw.clrText = ink;
    }

    if paint.strip_selected {
        // The lever of the doc comment: without `CDIS_SELECTED` the control paints the row
        // as ordinary — with the pair just set instead of the system highlight.
        draw.nmcd.uItemState =
            NMCUSTOMDRAW_DRAW_STATE_FLAGS(draw.nmcd.uItemState.0 & !CDIS_SELECTED.0);
    }

    answer_notify(hwnd, isize::try_from(CDRF_DODEFAULT).unwrap_or(0))
}

/// Hands one `WM_NOTIFY` answer to the dialog manager — task T-11-7.
///
/// A dialog procedure cannot answer `WM_NOTIFY` by return value alone: the documented
/// result protocol of a DLGPROC is two moves — the answer goes into the `DWLP_MSGRESULT`
/// field of the dialog window, and the return of TRUE says «handled, the result is there».
/// `DWLP_MSGRESULT` is a `DWLP_*` field — the dialog manager's own storage, the very range
/// the `GWLP_USERDATA` comment of `WM_INITDIALOG` sets this file's pointer apart from.
fn answer_notify(hwnd: HWND, result: isize) -> isize {
    // SAFETY: `hwnd` is the live dialog and `DWLP_MSGRESULT` is the field every dialog
    // reserves for exactly this; the previous value is dropped — the field is an answer
    // slot, not state of ours.
    unsafe {
        SetWindowLongPtrW(
            hwnd,
            WINDOW_LONG_PTR_INDEX(i32::try_from(DWLP_MSGRESULT).unwrap_or(0)),
            result,
        )
    };

    // TRUE — handled; the manager reads the answer back from the field.
    1
}

/// Draws one owner-drawn button of the dialog — FR-92а, task T-11-5a; since task T-11-5b
/// the eight check boxes and radio buttons arrive here too, recognised by identifier and
/// handed to [`draw_glyph_element`] before the push-button path below, and since task
/// T-11-6 the items of the four combo boxes, recognised by `CtlType` and handed to
/// [`draw_combo_item`] first of all.
///
/// # SEC-05, last sentence, held to the letter
///
/// `lparam` points at the `DRAWITEMSTRUCT` of the message. Exactly seven fields are read
/// out of it, all in one place at the top of this function: `CtlType` (button or combo),
/// `CtlID` (the identifier), `itemID` (which item of a combo), `itemState` (pressed,
/// disabled, focused, closed part), `hDC` (the DC to paint into), `rcItem` (the rectangle
/// to paint) and `hwndItem` (the window the message claims to speak for — only ever
/// *compared* with the dialog's own control, never followed; see [`draw_combo_item`]) —
/// the identifier and the drawing rectangle SEC-05 words the allowance around, plus the
/// plain values that say what to paint. **The pointer-sized `itemData` is never read**:
/// no pointer of the incoming message is dereferenced beyond that one struct read, and
/// nothing in it is followed further. The caption is *not* taken from the message either —
/// it is read from the dialog's own control by identifier (`get_text` →
/// `GetDlgItemTextW`; a combo item's text by `CB_GETLBTEXT` from the dialog's own combo).
/// The window of time is the gate: this message is handled by the
/// dialog procedure alone, and the dialog window exists exactly while the program itself
/// holds the dialog on the screen — a forged message outside that window has no procedure
/// to arrive at.
///
/// # What is drawn
///
/// Face by `FillRect`, single-pixel frame by `FrameRect`, caption by `DrawTextW` centred
/// both ways in the dialog's own font (already selected into the DC the manager hands
/// over); the colour choice is the closed table of [`button_color_roles`]. `ODS_FOCUS`
/// adds the dotted `DrawFocusRect` frame inside the border, over any face — unless the
/// manager says the focus cue is hidden (`ODS_NOFOCUSRECT`, the keyboard-cues setting),
/// which is how a native button behaves.
///
/// Answers 1 — «drawn» — for a button, and 0 for anything else. 0 is also the answer when
/// the state is unreachable or [`theme::Brushes::new`] was refused at initialisation: an
/// owner-drawn control has no system drawing to fall back to, but painting would need
/// colour numbers this module is not allowed to hold (§6.2 — the palette lives in `theme`
/// alone), and the refused-brushes dialog is already living degraded by the decision of
/// T-11-4.
///
/// # Safety
///
/// Called from [`dialog_proc`] only, with the `lparam` of the message: the dialog manager
/// sends `WM_DRAWITEM` with a pointer to a `DRAWITEMSTRUCT` that lives for the length of
/// the send.
unsafe fn on_draw_item(hwnd: HWND, lparam: LPARAM) -> isize {
    if lparam.0 == 0 {
        return 0;
    }

    // The one dereference of the message, and the whole of what is taken out of it: seven
    // fields, copied out as plain values before anything else runs. `itemData` — never.
    //
    // SAFETY: see above — the dialog manager owns the struct for the length of the send,
    // and this procedure is inside that send.
    let item = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
    let (ctl_type, ctl_id, item_id, item_state, dc, rect, item_window) = (
        item.CtlType,
        item.CtlID,
        item.itemID,
        item.itemState,
        item.hDC,
        item.rcItem,
        item.hwndItem,
    );

    let control = i32::try_from(ctl_id).unwrap_or(-1);

    // FR-92а, task T-11-6: the items of the four combo boxes, by control type — a combo
    // item is not a button, so this branch comes before the button check. SEC-05: the
    // identifier is checked against the four before any work.
    if ctl_type == ODT_COMBOBOX {
        if !owner_drawn_item(ctl_type.0, control) {
            return 0;
        }

        // SAFETY: see the caller — `dc` and `rect` are the values of the message, used
        // only to paint into for the length of this send; `item_window` is compared and
        // never followed.
        return unsafe {
            draw_combo_item(hwnd, control, item_id, item_state.0, dc, rect, item_window)
        };
    }

    // FR-92а, task T-11-14: the rows of the exclusion list, by control type — the same gate,
    // the same seven fields, and the same «not handled» for anything the gate refuses.
    if ctl_type == ODT_LISTBOX {
        if !owner_drawn_item(ctl_type.0, control) {
            return 0;
        }

        // SAFETY: as for the combo branch above.
        return unsafe {
            draw_list_item(hwnd, control, item_id, item_state.0, dc, rect, item_window)
        };
    }

    // FR-92а, task T-11-18: the labels of the template, by control type — a static is not a
    // button, so this branch comes before the button check, exactly as the two above do. SEC-05:
    // the identifier is checked against [`OWNER_DRAWN_LABELS`] before any work, and the state of
    // the message is never even looked at — a label takes no focus and none of them is ever
    // disabled, so `itemState` says nothing about how a label is painted.
    if ctl_type == ODT_STATIC {
        if !OWNER_DRAWN_LABELS.contains(&control) {
            return 0;
        }

        // SAFETY: as for the combo branch above — `dc` and `rect` are the values of the
        // message, used only to paint into for the length of this send.
        return unsafe { draw_label(hwnd, control, dc, rect) };
    }

    if ctl_type != ODT_BUTTON {
        return 0;
    }

    let pressed = item_state.0 & ODS_SELECTED.0 != 0;
    let disabled = item_state.0 & ODS_DISABLED.0 != 0;
    let focused = item_state.0 & ODS_FOCUS.0 != 0 && item_state.0 & ODS_NOFOCUSRECT.0 == 0;

    // FR-92а, task T-11-5b: the check boxes and radio buttons, by identifier, ahead of the
    // push-button path. Everything they take out of the message is already on this frame —
    // the same seven fields, nothing else; the check state is deliberately *not* a field of
    // the message at all, it is read from the dialog's own store (see the function).
    if let Some(kind) = glyph_kind(control) {
        // SAFETY: see the caller — `dc` and `rect` are the values of the message, used
        // only to paint into for the length of this send.
        return unsafe { draw_glyph_element(hwnd, kind, control, dc, rect, disabled, focused) };
    }

    // ⚠ There is deliberately **no branch for the eight group panels here** — task T-11-13.
    // A panel used to be drawn from this message, and being drawable from a message meant
    // being an element: it covered the whole area of its block, took every click that landed
    // on the block's empty ground, redrew itself on the click, and laid its own fill over
    // its neighbours, which were never invalidated and did not come back. The panels are the
    // *background* of their blocks now — `on_erase_background` draws them, and the eight
    // controls are invisible in the template, so no `WM_DRAWITEM` names them any more. A
    // forged message that named one would fall through to the push-button path below and
    // paint a button face inside the rectangle it named, which is the same nothing every
    // other forged identifier buys (SEC-05).
    //
    // The colour choice, split from the painting exactly as `on_ctl_color` splits it: the
    // borrow of the state ends before the DC is touched, and what leaves the closure is
    // plain values — a brush handle the state keeps alive until the dialog ends, and two
    // inks.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13, T-11-4).
            let brushes = state.brushes.as_ref()?;

            Some((
                resolve_button_colors(
                    button_color_roles(control, pressed, disabled),
                    brushes,
                    state.palette,
                ),
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((colors, face))) = choice else {
        return 0;
    };

    // SAFETY: see the caller — `dc` and `rect` are the values of the message, used only
    // to paint into for the length of this send; `face` is a face the state owns for longer.
    unsafe { paint_push_button(hwnd, control, dc, rect, colors, focused, face) }
}

/// Paints one owner-drawn push button: the rounded face under its single-pixel frame, the
/// caption and the focus cue. The drawing half [`on_draw_item`] and [`on_about_draw_item`]
/// share — one body, moved out of `on_draw_item` by task T-11-11 rather than copied
/// (§6.2). Sharing it is why the «ОК» of the about window rounds off with the rest.
///
/// The caption comes from the dialog's own control by identifier — never from the message.
///
/// # Safety
///
/// Called with values copied out of the `WM_DRAWITEM` message the caller is inside of:
/// `dc` and `rect` are owned by the sender for the length of the send, and `hwnd` is the
/// live dialog whose state keeps the brushes of `colors` alive.
unsafe fn paint_push_button(
    hwnd: HWND,
    control: i32,
    dc: HDC,
    rect: RECT,
    colors: ResolvedButtonColors,
    focused: bool,
    face: Option<HFONT>,
) -> isize {
    // Without the trailing NUL: `DrawTextW` takes the length of the slice it is given.
    let mut caption: Vec<u16> = get_text(hwnd, control).encode_utf16().collect();

    // NFR-13, for the paint calls below: each answers a success flag or a previous value,
    // and every answer is deliberately dropped for the same reason `apply_ctl_color` drops
    // its own — the manager never hands a dead DC, only a forged message could (SEC-05),
    // and the right reaction to a forgery is indifference: no read-back, no
    // `debug_assert` handing the forger a crash of a debug build, and no journal row for
    // GDI refusals (reviews\T-11-1.md).

    // The face and the single-pixel frame in one figure — п. 2.2 of task T-11-13: the
    // buttons of the mock-ups have [`CORNER_RADIUS`] corners — the same radius as everything
    // else the dialog rounds off, since task T-11-16 — and a square `FillRect` under a square
    // `FrameRect` cannot have any. The corners the rounding cuts away keep the erase of
    // `WM_CTLCOLORBTN`, which is the ground the button stands on.
    let dpi = dc_dpi(dc);

    paint_rounded(
        dc,
        &rect,
        scaled(CORNER_RADIUS, dpi),
        colors.border,
        colors.face,
        dpi,
    );

    // SAFETY: `dc` is a handle passed by value; both calls write an attribute of the DC
    // and touch no memory of this process.
    unsafe { SetBkMode(dc, TRANSPARENT) };
    // SAFETY: as above.
    unsafe { SetTextColor(dc, colors.ink) };

    if !caption.is_empty() {
        let mut text_rect = rect;

        // Our own face, grey-antialiased — task T-11-17. `None` leaves the manager's own font
        // in the DC, which is what this drawing used before that task (NFR-13).
        //
        // SAFETY: `dc` is the DC of the message and `face` is a live font the window's state
        // owns for longer than this call; the previous handle is put back below.
        let previous_face = unsafe { select_face(dc, face) };

        // SAFETY: `caption` and `text_rect` are live locals of this frame; the format
        // has no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the caption
        // and writes only pixels of the DC.
        unsafe {
            DrawTextW(
                dc,
                &mut caption,
                &mut text_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            )
        };

        // SAFETY: `previous_face` is what `select_face` answered for this same DC, and
        // nothing between the two calls selected another font.
        unsafe { restore_face(dc, previous_face) };
    }

    if focused {
        // Inside the single-pixel border, over whatever face was chosen above.
        let focus_rect = RECT {
            left: rect.left + FOCUS_CUE_INSET,
            top: rect.top + FOCUS_CUE_INSET,
            right: rect.right - FOCUS_CUE_INSET,
            bottom: rect.bottom - FOCUS_CUE_INSET,
        };

        // NFR-13: the `BOOL` is examined and deliberately dropped — see the block
        // comment above the paint calls.
        //
        // SAFETY: `dc` is the DC of the message and `focus_rect` is a live local of
        // this frame; the call keeps no pointer.
        let _ = unsafe { DrawFocusRect(dc, &focus_rect) };
    }

    // TRUE — the button is drawn.
    1
}

/// Draws one owner-drawn label of the settings dialog — FR-92а, task T-11-18.
///
/// The choosing half: the ground the label stands on, the ink its role asks for and the face
/// this window sets its own text in. The painting half is the shared [`paint_label`], which
/// the about window uses too (§6.2: one body, not a copy).
///
/// # SEC-05, and FR-94
///
/// Nothing here dereferences the message: [`on_draw_item`] copied the allowed fields out as
/// plain values and passed two of them in — `dc` to paint into and `rect` to paint within —
/// and the identifier was checked against [`OWNER_DRAWN_LABELS`] before this function was
/// called at all. **The text is not in the message either**: it is read from the dialog's own
/// control by identifier (`get_text` → `GetDlgItemTextW`), which is the whole of why the
/// language switch of FR-94 keeps working — `SetDlgItemTextW` writes the string of the locale
/// into the control, the control invalidates itself, and the very next `WM_DRAWITEM` reads
/// back what was written. There is no second source of the caption to fall out of step with it.
///
/// Answers 1 — «drawn» — and 0 when the state is unreachable or [`theme::Brushes::new`] was
/// refused at initialisation, for the reason [`on_draw_item`] gives.
///
/// # Safety
///
/// Called from [`on_draw_item`] only, with values copied out of the `WM_DRAWITEM` message it
/// is inside of: the sender owns `dc` for the length of the send.
unsafe fn draw_label(hwnd: HWND, control: i32, dc: HDC, rect: RECT) -> isize {
    // The colour choice, split from the painting exactly as `on_ctl_color` splits it: the
    // borrow of the state ends before the DC is touched, and what leaves the closure is plain
    // values — a brush and a face the state keeps alive until the dialog ends, and an ink.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13). The label then
            // keeps whatever the system painted, which is the degraded-but-alive answer of
            // T-11-4.
            let brushes = state.brushes.as_ref()?;

            // The very ground `WM_CTLCOLORSTATIC` answers for this control: the panel brush
            // for a label lying on one of the eight panels, the window brush everywhere else.
            // The map is `panel_children`, built once on `WM_INITDIALOG` — not a second
            // opinion about the geometry.
            let ground = if state.panel_children.contains(&control) {
                brushes.panel_bg()
            } else {
                brushes.window_bg()
            };

            Some((
                ground,
                label_ink(static_color_role(control), state.palette),
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((ground, ink, face))) = choice else {
        return 0;
    };

    // Read after the borrow ends: `get_text` sends a message to a control, and a message sent
    // while the state is borrowed can come back into the procedure.
    //
    // Without the trailing NUL: `DrawTextW` takes the length of the slice it is given.
    let mut caption: Vec<u16> = get_text(hwnd, control).encode_utf16().collect();

    // SAFETY: see the caller — `dc` and `rect` are the values of the message; `ground` and
    // `face` are objects the dialog's state owns for longer than this call.
    unsafe { paint_label(dc, rect, &mut caption, ground, ink, face) }
}

/// Paints one owner-drawn label: the ground it stands on, then its text in our own
/// grey-antialiased face — FR-92а, task T-11-18. The drawing half [`draw_label`] and
/// [`draw_about_label`] share, exactly as [`paint_push_button`] is shared by the two windows'
/// button paths (§6.2: one body, not a copy).
///
/// # Why the ground is filled here and not left to the system
///
/// ⚠ **An owner-drawn static is responsible for the whole of its rectangle.** A plain
/// `SS_LEFT` static filled its client area with the brush `WM_CTLCOLORSTATIC` answered and
/// wrote the text over it; `SS_OWNERDRAW` replaces that drawing outright, and whether the
/// system still lays the brush down first is not something the documentation promises. Filling
/// here is right under either behaviour — a second `FillRect` over the same colour is
/// invisible — and skipping it is right under only one.
///
/// It is not a theoretical worry: the text of these labels **changes while the window is up**.
/// `fill_layouts` writes `String::new()` into `IDC_LAYOUT_NOTE` the moment the two combo boxes
/// name layouts the session has again, `show_hotkey` does the same to `IDC_HOTKEY_NOTE` after
/// a capture, and every state line is rewritten with a shorter or longer sentence. Without the
/// fill the old words would stay under the new ones, and a note that was cleared would never
/// go away.
///
/// # The empty label
///
/// A caption of no characters is drawn as the ground and nothing else: no font is selected, no
/// text colour is set, `DrawTextW` is not called. `IDC_HOTKEY_NOTE` and `IDC_LAYOUT_NOTE` are
/// empty most of the time a person has the window open, and the three rows of the «Состояние»
/// block and the journal path are empty in the template until `WM_INITDIALOG` fills them —
/// so this is the ordinary case, not the corner one (NFR-13).
///
/// # Safety
///
/// Called with values copied out of the `WM_DRAWITEM` message the caller is inside of: `dc` is
/// owned by the sender for the length of the send, and `ground` and `face` are objects
/// somebody else owns for longer than the drawing.
pub unsafe fn paint_label(
    dc: HDC,
    rect: RECT,
    caption: &mut [u16],
    ground: HBRUSH,
    ink: COLORREF,
    face: Option<HFONT>,
) -> isize {
    // NFR-13, for the paint calls below: each answers a success flag or a previous value, and
    // every answer is deliberately dropped for the reason `paint_push_button` gives for its
    // own — the manager never hands a dead DC, only a forged message could (SEC-05), and the
    // right reaction to a forgery is indifference.
    //
    // SAFETY: `dc` is the DC of the message and `rect` is a live local of the caller's frame;
    // `ground` is a live brush somebody else owns.
    unsafe { FillRect(dc, &rect, ground) };

    if caption.is_empty() {
        // TRUE — the label is drawn, and the ground is the whole of it.
        return 1;
    }

    // SAFETY: `dc` is a handle passed by value; both calls write an attribute of the DC and
    // touch no memory of this process.
    unsafe { SetBkMode(dc, TRANSPARENT) };
    // SAFETY: as above.
    unsafe { SetTextColor(dc, ink) };

    // Our own face, grey-antialiased — task T-11-17, and the whole point of this one: the
    // labels used to be the last text in the window still drawn on the manager's ClearType.
    // `None` leaves the manager's own font in the DC, which is what a window whose faces could
    // not be made draws in (NFR-13).
    //
    // SAFETY: `dc` is the DC of the message and `face` is a live font the window's state owns
    // for longer than this call; the previous handle is put back below.
    let previous_face = unsafe { select_face(dc, face) };

    // The whole rectangle of the control, with no inset of its own — the very rectangle an
    // `SS_LEFT` static drew into, so no caption moves by a pixel (T-11-16 and T-11-17 settled
    // the values and this task changes none of them).
    let mut text_rect = rect;

    // SAFETY: `caption` and `text_rect` are live locals of this frame and the caller's; the
    // format has no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the caption and
    // writes only pixels of the DC.
    unsafe { DrawTextW(dc, caption, &mut text_rect, LABEL_TEXT_FORMAT) };

    // SAFETY: `previous_face` is what `select_face` answered for this same DC, and nothing
    // between the two calls selected another font.
    unsafe { restore_face(dc, previous_face) };

    // TRUE — the label is drawn.
    1
}

/// One [`ComboFillRole`] resolved against the brushes of the dialog — the single place these
/// two roles become a brush, shared by the item drawing, the row drawing of the exclusion
/// list and the closed-face drawing of the subclass (§6.2: one body, not three copies).
fn combo_fill_brush(role: ComboFillRole, brushes: &theme::Brushes) -> HBRUSH {
    match role {
        ComboFillRole::FieldBg => brushes.field_bg(),
        ComboFillRole::SelBg => brushes.sel_bg(),
    }
}

/// One [`ComboTextRole`] resolved against the palette — the single place these three roles
/// become an ink, shared by the same three drawings [`combo_fill_brush`] serves.
fn combo_text_ink(role: ComboTextRole, palette: &theme::Palette) -> COLORREF {
    match role {
        ComboTextRole::Text => palette.text,
        ComboTextRole::SelFg => palette.sel_fg,
        ComboTextRole::TextMuted => palette.text_muted,
    }
}

/// Draws one item of an owner-drawn combo box — a row of the dropped-down list or the
/// closed face with the chosen value — FR-92а, task T-11-6.
///
/// # SEC-05, last sentence, held to the letter
///
/// Nothing here dereferences the message: [`on_draw_item`] copied the seven allowed
/// fields of `DRAWITEMSTRUCT` out as plain values and passed five of them in — the item
/// index and state, `dc` to paint into, `rect` to paint within, and `item_window`. The
/// last one is only ever *compared*: the window the message claims to speak for must be
/// the dialog's own combo, the one `GetDlgItem` answers for the already-checked
/// identifier, or nothing is painted at all — a forged message that names our identifier
/// but somebody else's window buys its sender nothing. The item's text is then read from
/// the dialog's own combo by identifier (`CB_GETLBTEXTLEN`/`CB_GETLBTEXT` through
/// `SendDlgItemMessageW`) — never through the handle of the message, which would also be
/// the bare `SendMessage` FR-72 bans program-wide. `itemData` is never read: the four
/// combos keep their strings in the control itself (`CBS_HASSTRINGS`), and nothing was
/// ever stored in their item data.
///
/// # What is drawn
///
/// The ground by `FillRect`, chosen by the closed table of [`combo_item_color_roles`]:
/// `field_bg` for an ordinary item and for the closed face, the selection pair
/// `sel_bg`/`sel_fg` for the highlighted item of the dropped-down list. An empty combo
/// asks for its closed face with no item to name — `itemID` is −1 — and the filled
/// ground is the whole of that answer. Otherwise the item's own text follows, in the
/// dialog's font the DC already holds, vertically centred behind [`FIELD_TEXT_INSET_DLU`]
/// pixels of air;
/// and the dotted `DrawFocusRect` over the closed part while it holds the focus — unless
/// the manager says the focus cue is hidden (`ODS_NOFOCUSRECT`), exactly as the buttons
/// of T-11-5a behave.
///
/// Answers 1 — «drawn». 0 when the identifier and the window of the message disagree,
/// when the state is unreachable, or when `theme::Brushes::new` was refused at
/// initialisation, for the reason [`on_draw_item`] gives: painting would need colour
/// numbers this module is not allowed to hold (§6.2), and the refused-brushes dialog
/// already lives degraded by the decision of T-11-4.
///
/// # Safety
///
/// Called from [`on_draw_item`] only, with values copied out of the message it is inside
/// of: `dc` and `rect` are owned by the sender for the length of the send.
unsafe fn draw_combo_item(
    hwnd: HWND,
    control: i32,
    item_id: u32,
    item_state: u32,
    dc: HDC,
    rect: RECT,
    item_window: HWND,
) -> isize {
    // The comparison SEC-05 allows and nothing more: the window the message claims to
    // speak for must be the dialog's own combo. `GetDlgItem` refusing — no such control —
    // fails the same way a mismatch does.
    //
    // SAFETY: `hwnd` is the live dialog; the call reads a window field and no memory of
    // ours, and answers a handle or an error.
    if unsafe { GetDlgItem(Some(hwnd), control) }.ok() != Some(item_window) {
        return 0;
    }

    let closed_part = item_state & ODS_COMBOBOXEDIT.0 != 0;
    let highlighted = item_state & ODS_SELECTED.0 != 0;
    let focused = item_state & ODS_FOCUS.0 != 0 && item_state & ODS_NOFOCUSRECT.0 == 0;

    // The colour choice, split from the painting exactly as everywhere in this file: the
    // borrow of the state ends before the DC is touched, and what leaves the closure is
    // plain values — a brush the state keeps alive until the dialog ends, and an ink.
    //
    // SAFETY: see the caller of `on_draw_item`.
    let choice = unsafe {
        with_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13, T-11-4).
            let brushes = state.brushes.as_ref()?;
            let palette = state.palette;

            let colors = combo_item_color_roles(closed_part, highlighted);

            Some((
                combo_fill_brush(colors.fill, brushes),
                combo_text_ink(colors.text, palette),
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((fill, ink, face))) = choice else {
        return 0;
    };

    // NFR-13, for the paint calls below: each answers a success flag or a previous value,
    // and every answer is deliberately dropped for the same reason `on_ctl_color` drops
    // its own — the manager never hands a dead DC, only a forged message could (SEC-05),
    // and the right reaction to a forgery is indifference: no read-back, no
    // `debug_assert` handing the forger a crash of a debug build, and no journal row for
    // GDI refusals (reviews\T-11-1.md).

    // SAFETY: `dc` and `rect` are the values of the message, used only to paint into for
    // the length of this send; `fill` is a live brush of the dialog's state.
    unsafe { FillRect(dc, &rect, fill) };

    // −1 — an empty combo asking for its closed face with no item to name: the filled
    // ground above is the whole of the answer.
    if item_id == u32::MAX {
        return 1;
    }

    // The item's text, from the dialog's own combo by identifier — never through the
    // handle of the message. `CB_GETLBTEXTLEN` sizes the buffer; either message answering
    // the error value skips the text and keeps the filled ground (NFR-13: a row without
    // its word is degraded but alive, and the journal has no row for it —
    // reviews\T-11-1.md).
    let index = usize::try_from(item_id).unwrap_or(usize::MAX);
    let length = send_to(hwnd, control, CB_GETLBTEXTLEN, index, 0);

    if let Ok(length) = usize::try_from(length) {
        // One for the terminator the control writes and never counts.
        let mut buffer = vec![0u16; length + 1];

        // SAFETY: `buffer` is owned by this frame and holds the length the combo has just
        // reported plus the terminator, which is exactly what `CB_GETLBTEXT` writes; the
        // pointer is not retained by the call.
        let copied = send_to(
            hwnd,
            control,
            CB_GETLBTEXT,
            index,
            buffer.as_mut_ptr() as isize,
        );

        let copied = usize::try_from(copied).unwrap_or(0).min(length);

        if copied > 0 {
            // SAFETY: `dc` is a handle passed by value; both calls write an attribute of
            // the DC and touch no memory of this process.
            unsafe { SetBkMode(dc, TRANSPARENT) };
            // SAFETY: as above.
            unsafe { SetTextColor(dc, ink) };

            // The same three dialog units the closed face above it uses — an item of the
            // dropped-down list stands directly under the closed part and shares its left
            // edge, so a second inset would make the word jump on opening (task T-11-16).
            let inset = dialog_units(hwnd, FIELD_TEXT_INSET_DLU, 0)
                .map(|(horizontal, _)| horizontal)
                .unwrap_or(FIELD_TEXT_INSET_DLU);

            let mut text_rect = RECT {
                left: rect.left + inset,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };

            // Our own face, grey-antialiased — task T-11-17.
            //
            // SAFETY: `dc` is the DC of the message and `face` is a live font the dialog's
            // state owns for longer than this call; the previous handle is put back below.
            let previous_face = unsafe { select_face(dc, face) };

            // SAFETY: the slice and `text_rect` are live locals of this frame; the format
            // has no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the text
            // and writes only pixels of the DC.
            unsafe {
                DrawTextW(
                    dc,
                    &mut buffer[..copied],
                    &mut text_rect,
                    DT_SINGLELINE | DT_VCENTER,
                )
            };

            // SAFETY: `previous_face` is what `select_face` answered for this same DC.
            unsafe { restore_face(dc, previous_face) };
        }
    }

    if closed_part && focused {
        // Over the whole closed face, which is where a native combo puts its cue.
        //
        // NFR-13: the `BOOL` is examined and deliberately dropped — the block comment
        // above the paint calls says why.
        //
        // SAFETY: `dc` is the DC of the message and `rect` is a live local of this frame;
        // the call keeps no pointer.
        let _ = unsafe { DrawFocusRect(dc, &rect) };
    }

    // TRUE — the item is drawn.
    1
}

/// Draws one row of the exclusion list — FR-92а, task T-11-14.
///
/// # Why the list draws its own rows at all
///
/// `WM_CTLCOLORLISTBOX` has coloured the ground of this list since task T-11-4, but a list
/// box paints its **selection stripe** itself, out of the system `COLOR_HIGHLIGHT` — the blue
/// bar the mock-ups do not have in either palette. There is no message that recolours it:
/// the documented way to own the stripe is to own the row, which is what `LBS_OWNERDRAWFIXED`
/// in `app.rc` and this function are. `LBS_HASSTRINGS` stays beside it, so the population
/// (`LB_ADDSTRING`) and the reading (`LB_GETTEXT` in [`list_items`]) of FR-84 are untouched —
/// the strings still live in the control.
///
/// # SEC-05, last sentence, held to the letter
///
/// Nothing here dereferences the message, exactly as in [`draw_combo_item`]: the seven
/// allowed fields were copied out as plain values by [`on_draw_item`], `item_window` is only
/// ever *compared* against the dialog's own control, and the row's text is then read from the
/// dialog's own list by identifier through `SendDlgItemMessageW`. `itemData` is never read:
/// the list keeps its strings in the control itself and nothing was ever stored in its item
/// data.
///
/// # What is drawn
///
/// The ground by `FillRect`, chosen by [`list_item_color_roles`] — `field_bg`/`text` for an
/// ordinary row, the palette's own `sel_bg`/`sel_fg` for the selected one — then the row's
/// text behind [`LIST_TEXT_INSET`] mock-up pixels of air, and the dotted `DrawFocusRect` when
/// the manager says
/// this row carries the focus and the focus cues are not hidden, exactly as everywhere else
/// in this dialog. An empty list asks for the cue with no row to name (`itemID` is −1); the
/// filled ground and the cue are then the whole of the answer.
///
/// Answers 1 — «drawn» — or 0 for the reasons [`draw_combo_item`] answers 0.
///
/// # Safety
///
/// Called from [`on_draw_item`] only, with values copied out of the message it is inside of:
/// `dc` and `rect` are owned by the sender for the length of the send.
unsafe fn draw_list_item(
    hwnd: HWND,
    control: i32,
    item_id: u32,
    item_state: u32,
    dc: HDC,
    rect: RECT,
    item_window: HWND,
) -> isize {
    // The comparison SEC-05 allows and nothing more — see `draw_combo_item`.
    //
    // SAFETY: `hwnd` is the live dialog; the call reads a window field and no memory of
    // ours, and answers a handle or an error.
    if unsafe { GetDlgItem(Some(hwnd), control) }.ok() != Some(item_window) {
        return 0;
    }

    let selected = item_state & ODS_SELECTED.0 != 0;
    let focused = item_state & ODS_FOCUS.0 != 0 && item_state & ODS_NOFOCUSRECT.0 == 0;

    // The colour choice, split from the painting exactly as everywhere in this file.
    //
    // SAFETY: see the caller of `on_draw_item`.
    let choice = unsafe {
        with_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13, T-11-4).
            let brushes = state.brushes.as_ref()?;
            let colors = list_item_color_roles(selected);

            Some((
                // The ground of the row itself — the field the list is, whatever the row is
                // doing. The selection is a figure *on* it since task T-11-16.
                brushes.field_bg(),
                combo_fill_brush(colors.fill, brushes),
                match colors.fill {
                    ComboFillRole::FieldBg => state.palette.field_bg,
                    ComboFillRole::SelBg => state.palette.sel_bg,
                },
                combo_text_ink(colors.text, state.palette),
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((ground, fill, fill_ink, ink, face))) = choice else {
        return 0;
    };

    let dpi = dc_dpi(dc);

    // NFR-13, for the paint calls below: every answer is deliberately dropped, for the reason
    // `draw_combo_item` states for its own.

    // SAFETY: `dc` and `rect` are the values of the message, used only to paint into for the
    // length of this send; `ground` is a live brush of the dialog's state.
    unsafe { FillRect(dc, &rect, ground) };

    // The selection stripe of the mock-ups: not the whole row, but a rounded rectangle
    // [`LIST_SELECTION_INSET`] mock-up pixels in from each side of it —
    // `FillRectPx $g ($px+2) $ry ($pw-4) $rowH $S.SelBg 3` of the `'lbox'` arm, task T-11-16.
    // Left and right only, exactly as there: the stripe keeps the full height of the row.
    // Outlined in its own fill, because `paint_rounded` draws frame and interior in one figure
    // and this one has no frame.
    if selected {
        let inset = scaled(LIST_SELECTION_INSET, dpi);

        let stripe = RECT {
            left: rect.left + inset,
            top: rect.top,
            right: rect.right - inset,
            bottom: rect.bottom,
        };

        paint_rounded(
            dc,
            &stripe,
            scaled(LIST_SELECTION_RADIUS, dpi),
            fill_ink,
            fill,
            dpi,
        );
    }

    // −1 — an empty list asking for the focus cue with no row to name.
    if item_id != u32::MAX {
        let index = usize::try_from(item_id).unwrap_or(usize::MAX);
        let length = send_to(hwnd, control, LB_GETTEXTLEN, index, 0);

        // `LB_ERR` is negative and fails the conversion, which skips the text and keeps the
        // filled ground (NFR-13, as in `draw_combo_item`).
        if let Ok(length) = usize::try_from(length) {
            // One for the terminator the control writes and never counts.
            let mut buffer = vec![0u16; length + 1];

            // SAFETY: `buffer` is owned by this frame and holds the length the list has just
            // reported plus the terminator, which is exactly what `LB_GETTEXT` writes; the
            // pointer is not retained by the call.
            let copied = send_to(
                hwnd,
                control,
                LB_GETTEXT,
                index,
                buffer.as_mut_ptr() as isize,
            );

            let copied = usize::try_from(copied).unwrap_or(0).min(length);

            if copied > 0 {
                // SAFETY: `dc` is a handle passed by value; both calls write an attribute of
                // the DC and touch no memory of this process.
                unsafe { SetBkMode(dc, TRANSPARENT) };
                // SAFETY: as above.
                unsafe { SetTextColor(dc, ink) };

                // `TxtPx $g $it $F $brFg ($px + 7) ($ry + 2)` of the `'lbox'` arm: seven
                // mock-up pixels in from the left edge of the list, two below the top of the
                // row — task T-11-16, п. 3. Measured in the pixels of the picture and not in
                // dialog units, because that is the unit the generator states them in; and
                // top-aligned rather than `DT_VCENTER`, because the picture places the line
                // and does not centre it.
                let mut text_rect = RECT {
                    left: rect.left + scaled(LIST_TEXT_INSET, dpi),
                    top: rect.top + scaled(LIST_TEXT_TOP, dpi),
                    right: rect.right,
                    bottom: rect.bottom,
                };

                // Our own face, grey-antialiased — task T-11-17.
                //
                // SAFETY: `dc` is the DC of the message and `face` is a live font the
                // dialog's state owns for longer than this call.
                let previous_face = unsafe { select_face(dc, face) };

                // SAFETY: the slice and `text_rect` are live locals of this frame; the format
                // has no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the text
                // and writes only pixels of the DC.
                unsafe { DrawTextW(dc, &mut buffer[..copied], &mut text_rect, DT_SINGLELINE) };

                // SAFETY: `previous_face` is what `select_face` answered for this same DC.
                unsafe { restore_face(dc, previous_face) };
            }
        }
    }

    if focused {
        // NFR-13: the `BOOL` is examined and deliberately dropped — see above.
        //
        // SAFETY: `dc` is the DC of the message and `rect` is a live local of this frame;
        // the call keeps no pointer.
        let _ = unsafe { DrawFocusRect(dc, &rect) };
    }

    // TRUE — the row is drawn.
    1
}

/// One check mark of the mock-ups: the three points of its polyline and the pen it is drawn
/// with — task T-11-16.
///
/// A type and not six loose constants because the dialog draws **two** check marks of
/// different sizes, and the generator gives each its own literals: the tick of a dialog check
/// box lives in a 17-pixel square, the tick of a layout-list row in a 13-pixel one, and their
/// pens are 2,1 and 1,8 mock-up pixels. Both are drawn by [`draw_check_mark`], which is handed
/// one of these instead of holding either set.
///
/// Every number is in **tenths of a mock-up pixel**: the generator writes them with one
/// decimal, and rounding them to whole pixels before the scale is applied is exactly the kind
/// of loss task T-11-16 exists to undo. The points are offsets from the top-left corner of the
/// square, in the order the polyline visits them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckMark {
    /// The three points, in tenths of a mock-up pixel from the square's top-left corner.
    pub points_tenths: [(i32, i32); 3],
    /// The thickness of the pen, in tenths of a mock-up pixel.
    pub pen_tenths: i32,
}

/// The check mark of a dialog check box — `(PtF ($px+4.5) ($by+8.6))`, `(PtF ($px+7.3)
/// ($by+11.8))`, `(PtF ($px+12.5) ($by+5.2))` and `[single]2.1` of the `'check'` arm.
pub const GLYPH_CHECK_MARK: CheckMark = CheckMark {
    points_tenths: [(45, 86), (73, 118), (125, 52)],
    pen_tenths: 21,
};

/// The check mark of a layout-list tick — `(PtF ($bx+3.4) ($by+6.6))`, `(PtF ($bx+5.6)
/// ($by+9.0))`, `(PtF ($bx+9.6) ($by+4.0))` and `[single]1.8` of the `'lview'` arm.
pub const LIST_CHECK_MARK: CheckMark = CheckMark {
    points_tenths: [(34, 66), (56, 90), (96, 40)],
    pen_tenths: LIST_CHECK_PEN_TENTHS,
};

/// The three points of one check mark in the pixels of a window at `dpi`, measured from the
/// top-left corner of the square it is drawn in — the pure half of [`draw_check_mark`], task
/// T-11-16.
///
/// Every offset goes through [`scaled_tenths_offset`], which is [`scaled`] for a length given
/// with one decimal and without the floor of one a pen needs: a point may legitimately land on
/// the corner itself.
pub fn check_mark_points(corner: (i32, i32), mark: CheckMark, dpi: i32) -> [(i32, i32); 3] {
    mark.points_tenths.map(|(x, y)| {
        (
            corner.0 + scaled_tenths_offset(x, dpi),
            corner.1 + scaled_tenths_offset(y, dpi),
        )
    })
}

/// One glyph mark, resolved to the currency its drawing needs: the check strokes take an
/// ink for a transient pen, the dot takes a live brush of the dialog's state.
enum GlyphMarkPaint {
    /// The two strokes of a check mark, in this ink.
    Check(COLORREF),
    /// The dot of a radio button, filled with this brush.
    Dot(HBRUSH),
}

/// Draws one owner-drawn check box or radio button — FR-92а, task T-11-5b.
///
/// # SEC-05, last sentence, held to the letter
///
/// Nothing here reads the message: [`on_draw_item`] copied the seven allowed fields of
/// `DRAWITEMSTRUCT` out as plain values and passed two of them in — `dc` to paint into and
/// `rect` to paint within. The check state is deliberately *not* taken from the message
/// either — `itemState` carries no check bit worth trusting and `itemData` is never read —
/// it is the dialog's own answer, read from its own store by the identifier
/// ([`is_checked`] → [`GlyphChecks`], task T-11-5b-2: an owner-drawn control keeps no
/// check state, so the control cannot be asked), and the caption is the control's own text
/// ([`get_text`] → `GetDlgItemTextW`). A forged message can therefore misdraw nothing but
/// the rectangle it names.
///
/// # What is drawn
///
/// The glyph is a [`GLYPH_SIZE`]-sided square (check box) or circle (radio button) at the
/// left edge of the rectangle, centred vertically; the colour choice is the closed table
/// of [`glyph_color_roles`]. A checked box is filled whole with the accent and crossed by
/// the two pen strokes of the check mark; an unchecked one is the field fill under the
/// single-pixel `box_border` frame — `FrameRect` takes a brush, which is what the eighth
/// brush of `theme::Brushes` exists for. The circle of a radio button is one `Ellipse` —
/// outline of the frame ink, interior of the fill brush — and the checked dot is an inner
/// `Ellipse` of the mark brush. The caption sits to the right in the dialog's own font
/// (already selected into the DC), and focus adds the dotted `DrawFocusRect` frame around
/// the caption — the caller already folded `ODS_NOFOCUSRECT` into `focused`, so the cue
/// behaves as the native one does under the keyboard-cues setting.
///
/// Answers 1 — «drawn». 0 when the state is unreachable or `theme::Brushes::new` was
/// refused at initialisation, for the reason [`on_draw_item`] gives: painting would need
/// colour numbers this module is not allowed to hold (§6.2), and the refused-brushes
/// dialog already lives degraded by the decision of T-11-4.
///
/// # Safety
///
/// Called from [`on_draw_item`] only, with the `hDC` and `rcItem` of the message it is
/// inside of: both are owned by the dialog manager for the length of the send.
unsafe fn draw_glyph_element(
    hwnd: HWND,
    kind: GlyphKind,
    control: i32,
    dc: HDC,
    rect: RECT,
    disabled: bool,
    focused: bool,
) -> isize {
    // The dialog's own answer, not the message's: the store of task T-11-5b-2, read by
    // the identifier the message names — an owner-drawn control keeps no check state of
    // its own, so the control cannot be asked. Before the state exists — `WM_DRAWITEM`
    // does arrive ahead of full initialisation — the answer is «снят» and the element is
    // drawn unchecked rather than not at all (NFR-13).
    let checked = is_checked(hwnd, control);

    // The colour choice, split from the painting as everywhere in this file: the borrow of
    // the state ends before the DC is touched, and what leaves the closure is plain
    // values — brushes the state keeps alive until the dialog ends, and inks.
    //
    // SAFETY: see the caller of `on_draw_item`.
    let choice = unsafe {
        with_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13, T-11-4).
            let brushes = state.brushes.as_ref()?;
            let palette = state.palette;

            let colors = glyph_color_roles(kind, checked, disabled);

            // The single place the glyph roles become brushes and colours of the resolved
            // palette — the drawing below never sees a role.
            let fill = match colors.fill {
                GlyphFillRole::FieldBg => brushes.field_bg(),
                GlyphFillRole::AccentBg => brushes.accent_bg(),
            };

            // The same fill as a colour. Task T-11-14 rounded the square off, and `RoundRect`
            // draws fill and frame in one figure: the cell that has no frame — the checked,
            // enabled check box — is outlined in its own fill instead, so nothing shows.
            let fill_ink = match colors.fill {
                GlyphFillRole::FieldBg => palette.field_bg,
                GlyphFillRole::AccentBg => palette.accent_bg,
            };

            // The frame role as an ink: both figures are now drawn with a pen — the rounded
            // square of task T-11-14 and the circle of the radio button.
            let frame = colors
                .frame
                .map(|GlyphFrameRole::BoxBorder| palette.box_border);

            let mark = colors.mark.map(|role| match kind {
                GlyphKind::CheckBox => GlyphMarkPaint::Check(match role {
                    GlyphMarkRole::AccentFg => palette.accent_fg,
                    GlyphMarkRole::AccentBg => palette.accent_bg,
                    GlyphMarkRole::BoxBorder => palette.box_border,
                }),
                GlyphKind::RadioButton => GlyphMarkPaint::Dot(match role {
                    GlyphMarkRole::AccentBg => brushes.accent_bg(),
                    GlyphMarkRole::BoxBorder => brushes.box_border(),
                    // The closed table never marks a radio with the check-mark ink — the
                    // 2×2×2 table test is what holds this arm unreachable. Answered with
                    // the accent brush rather than a panic: a wrong dot colour would be a
                    // cosmetic defect, a crash of the UI thread would not (NFR-13).
                    GlyphMarkRole::AccentFg => brushes.accent_bg(),
                }),
            });

            let ink = match colors.text {
                GlyphTextRole::Text => palette.text,
                GlyphTextRole::TextMuted => palette.text_muted,
            };

            Some((
                fill,
                fill_ink,
                frame,
                mark,
                ink,
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((fill, fill_ink, frame, mark, ink, face))) = choice else {
        return 0;
    };

    let dpi = dc_dpi(dc);

    // The glyph: at the left edge, centred vertically, [`GLYPH_SIZE`] mock-up pixels a side
    // through the scale — the `$bs = 17` of the generator, and `$by = $py + [int](($ph -
    // $bs)/2)` for the centring (task T-11-16).
    let side = scaled(GLYPH_SIZE, dpi);
    let glyph_top = rect.top + (rect.bottom - rect.top - side) / 2;
    let glyph = RECT {
        left: rect.left,
        top: glyph_top,
        right: rect.left + side,
        bottom: glyph_top + side,
    };

    // NFR-13, for every paint call below: each answers a success flag or a previous
    // value, and every answer is deliberately dropped for the reason `on_draw_item` gives
    // for its own — the manager never hands a dead DC, only a forged message could
    // (SEC-05), and the right reaction to a forgery is indifference: no read-back, no
    // `debug_assert` handing the forger a crash of a debug build, and no journal row for
    // GDI refusals (reviews\T-11-1.md).
    match kind {
        GlyphKind::CheckBox => {
            // п. 2 of task T-11-14: the square of the mock-ups is rounded off by
            // [`GLYPH_CORNER_RADIUS`], and a square `FillRect` under a square `FrameRect`
            // cannot have a corner radius at all. One `RoundRect` draws both — the frame with
            // the pen, the interior with the brush — so the frameless cell of the closed
            // table is outlined in its own fill.
            paint_rounded(
                dc,
                &glyph,
                scaled(GLYPH_CORNER_RADIUS, dpi),
                frame.unwrap_or(fill_ink),
                fill,
                dpi,
            );

            if let Some(GlyphMarkPaint::Check(mark_ink)) = mark {
                draw_check_mark(dc, &glyph, mark_ink, GLYPH_CHECK_MARK, dpi);
            }
        }
        GlyphKind::RadioButton => {
            paint_ellipse(dc, &glyph, frame, fill, dpi);

            if let Some(GlyphMarkPaint::Dot(dot_brush)) = mark {
                // The 4,6 mock-up pixels of the generator; the 7,8 of the dot's own diameter
                // then falls out of the circle around it, as it does there.
                let dot_inset = scaled_tenths_offset(GLYPH_DOT_INSET_TENTHS, dpi);

                let dot = RECT {
                    left: glyph.left + dot_inset,
                    top: glyph.top + dot_inset,
                    right: glyph.right - dot_inset,
                    bottom: glyph.bottom - dot_inset,
                };

                paint_ellipse(dc, &dot, None, dot_brush, dpi);
            }
        }
    }

    // The caption, from the dialog's own control by identifier — never from the message.
    // Without the trailing NUL: `DrawTextW` takes the length of the slice it is given.
    let mut caption: Vec<u16> = get_text(hwnd, control).encode_utf16().collect();

    if !caption.is_empty() {
        // SAFETY: `dc` is a handle passed by value; both calls write an attribute of the
        // DC and touch no memory of this process.
        unsafe { SetBkMode(dc, TRANSPARENT) };
        // SAFETY: as above.
        unsafe { SetTextColor(dc, ink) };

        // Twelve dialog units from the **left edge of the element** — the `($c.x + 12)` of both
        // the `'check'` and the `'radio'` arm of the generator, task T-11-16. A refused
        // `MapDialogRect` keeps the bare unit count, which is the fallback
        // [`draw_panel_caption`] takes for its own inset (NFR-13).
        let caption_inset = dialog_units(hwnd, GLYPH_TEXT_INSET_DLU, 0)
            .map(|(horizontal, _)| horizontal)
            .unwrap_or(GLYPH_TEXT_INSET_DLU);

        let mut text_rect = RECT {
            left: rect.left + caption_inset,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        };

        // Our own face, grey-antialiased — task T-11-17. Selected before the measurement as
        // well as before the drawing: `DT_CALCRECT` below measures with whatever font the DC
        // holds, and a cue measured in one face around a caption drawn in another would sit
        // wrong.
        //
        // SAFETY: `dc` is the DC of the message and `face` is a live font the dialog's state
        // owns for longer than this call; the previous handle is put back below.
        let previous_face = unsafe { select_face(dc, face) };

        // SAFETY: `caption` and `text_rect` are live locals of this frame; the format has
        // no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the caption and
        // writes only pixels of the DC.
        unsafe { DrawTextW(dc, &mut caption, &mut text_rect, DT_VCENTER | DT_SINGLELINE) };

        if focused {
            // The dotted frame goes around the caption, not the glyph — as the native
            // control draws it. The width is measured with `DT_CALCRECT` into a scratch
            // rectangle; the height is the control's own, the caption being vertically
            // centred in it. (An element with no caption gets no cue — none of the eight
            // is captionless, and a frame around nothing would be noise.)
            let mut measured = text_rect;

            // SAFETY: as above — `DT_CALCRECT` writes the measured extent into
            // `measured`, a live local of this frame, and draws nothing.
            unsafe { DrawTextW(dc, &mut caption, &mut measured, DT_CALCRECT | DT_SINGLELINE) };

            let focus_rect = RECT {
                left: text_rect.left - FOCUS_CUE_TEXT_INSET,
                top: rect.top,
                right: (measured.right + FOCUS_CUE_INSET).min(rect.right),
                bottom: rect.bottom,
            };

            // NFR-13: the `BOOL` is examined and deliberately dropped — see the block
            // comment above the glyph painting.
            //
            // SAFETY: `dc` is the DC of the message and `focus_rect` is a live local of
            // this frame; the call keeps no pointer.
            let _ = unsafe { DrawFocusRect(dc, &focus_rect) };
        }

        // SAFETY: `previous_face` is what `select_face` answered for this same DC, and
        // nothing between the two calls selected another font.
        unsafe { restore_face(dc, previous_face) };
    }

    // TRUE — the element is drawn.
    1
}

/// Two strokes of the check mark, with a transient pen of `ink` — FR-92а, task T-11-5b; the
/// figure and the pen come from `mark` and the window's `dpi` since task T-11-16.
///
/// The pen lives for exactly this call: pens are not part of `theme::Brushes` — that owner
/// exists because `WM_CTLCOLOR*` answers must outlive the paint, which nothing here needs.
/// Created, selected, drawn with, deselected, deleted; the refusal of `CreatePen` skips
/// the mark and nothing else (NFR-13: examined; the glyph stays a filled square, the state
/// remains readable by the fill alone until the next repaint).
///
/// `dpi` is passed in rather than read off `dc`: one of the two callers paints into a memory
/// DC of the state image list, whose own answer to `GetDeviceCaps` is not the DPI the frames
/// are being built for.
pub fn draw_check_mark(dc: HDC, glyph: &RECT, ink: COLORREF, mark: CheckMark, dpi: i32) {
    // The two strokes of the generator: down into the corner, long up and out. The three
    // points are `mark`'s own, measured from the corner of the square through the scale.
    let points = check_mark_points((glyph.left, glyph.top), mark, dpi);
    let thickness = scaled_tenths(mark.pen_tenths, dpi);

    // The tile is the mark and nothing else — never the whole glyph. Two reasons, and both
    // matter: the strokes then blend into the flat interior the square has just been filled
    // with, and the smoothed corners of that square are not dragged through a second reduction
    // that would blur what [`paint_rounded`] has already got right. Both marks of the mock-ups
    // sit well inside their square, so the clamp below never actually cuts anything.
    let tile = clamped_to(glyph, &stroke_bounds(&points, thickness));

    let smoothed = Supersample::for_tile(tile.right - tile.left, tile.bottom - tile.top)
        .is_some_and(|surface| {
            surface.render(dc, &tile, thickness, |canvas| {
                let enlarged = points.map(|(x, y)| canvas.point(x, y));

                stroke_polyline(canvas.dc, &enlarged, ink, canvas.length(thickness));
            })
        });

    if !smoothed {
        stroke_polyline(dc, &points, ink, thickness);
    }
}

/// Strokes a polyline through `points` with a transient pen of `ink` — the aliased core of
/// [`draw_check_mark`] and of [`draw_combo_chevron`], which are the same two strokes with
/// different numbers (§6.2: one body, not two copies).
///
/// The pen lives for exactly this call: pens are not part of `theme::Brushes` — that owner
/// exists because `WM_CTLCOLOR*` answers must outlive the paint, which nothing here needs.
/// Created, selected, drawn with, deselected, deleted; the refusal of `CreatePen` skips the
/// figure and nothing else (NFR-13: examined; a checked glyph stays a filled square and a combo
/// box stays a field, both readable, until the next repaint).
fn stroke_polyline(dc: HDC, points: &[(i32, i32); 3], ink: COLORREF, thickness: i32) {
    // SAFETY: takes plain values, reads no memory of ours, answers a handle owned by this
    // frame until the `DeleteObject` below.
    let pen = unsafe { CreatePen(PS_SOLID, thickness, ink) };

    if pen.is_invalid() {
        return;
    }

    // SAFETY: `dc` is painted into for the length of the send this call is inside of;
    // `pen` is the live pen just made. The previous pen is kept and restored below.
    let previous = unsafe { SelectObject(dc, pen.into()) };

    let corners = points.map(|(x, y)| POINT { x, y });

    // SAFETY: `corners` is a live local of this frame, read by the call and not retained. The
    // answer is dropped for the NFR-13 reason the callers state for all their paint calls.
    let _ = unsafe { Polyline(dc, &corners) };

    // SAFETY: `previous` is the pen that was in the DC a moment ago; putting it back ends
    // this function's use of the DC.
    unsafe { SelectObject(dc, previous) };

    // SAFETY: `pen` was created above, handed to nobody — deselected the line before —
    // and freed exactly once, here. The `BOOL` is dropped: a refusal would mean the
    // handle was not a live GDI object of this process, which the ownership above makes
    // unreachable, and the paint path deliberately carries no `debug_assert` (SEC-05).
    let _ = unsafe { DeleteObject(pen.into()) };
}

/// One ellipse with an explicit outline and interior, smoothed — FR-92а, task T-11-5b,
/// smoothed whole by task T-11-17.
///
/// `outline` is the ink of a transient pen for the circle of a radio button; `None`
/// selects the stock `NULL_PEN` — no outline, interior only, which is how the dot is
/// painted. `fill` is a live brush of the dialog's state. `dpi` is the DPI of the window, for
/// the reason [`paint_rounded`] states.
///
/// A circle has no straight part at all, so — unlike a rounded rectangle — it is enlarged
/// **whole**: both ellipses this dialog draws are a glyph under 24 px a side, which is the
/// cheap half of the cost rule of this section. A refused surface or a refused blit falls
/// straight through to [`stroke_ellipse`], the aliased drawing of every task before this one
/// (NFR-13).
fn paint_ellipse(dc: HDC, area: &RECT, outline: Option<COLORREF>, fill: HBRUSH, dpi: i32) {
    // The same frame every other figure of the dialog is outlined with — one pixel at
    // 96 DPI through [`BORDER_THICKNESS`], and it grows with the DPI like the rest of the
    // mock-up (п. 3 of task T-11-15; the circle of a radio button is framed exactly as the
    // square of a check box beside it).
    let thickness = scaled(BORDER_THICKNESS, dpi).max(1);

    // A figure with no pen — the dot of a radio button — is all fill and takes neither the
    // path correction nor the far-edge one; a stroked circle takes both. See [`Canvas::area`].
    let pen = if outline.is_some() { thickness } else { 0 };

    let smoothed = Supersample::for_tile(area.right - area.left, area.bottom - area.top)
        .is_some_and(|surface| {
            surface.render(dc, area, pen, |canvas| {
                let enlarged = if outline.is_some() {
                    canvas.outline(area)
                } else {
                    canvas.area(area)
                };

                stroke_ellipse(
                    canvas.dc,
                    &enlarged,
                    outline,
                    fill,
                    canvas.length(thickness),
                );
            })
        });

    if !smoothed {
        stroke_ellipse(dc, area, outline, fill, thickness);
    }
}

/// The aliased core of [`paint_ellipse`] — one `Ellipse` with a transient pen of `outline`
/// and the interior of `fill`; the body that function had before task T-11-17, with the pen
/// thickness handed in for the reason [`stroke_rounded`] states.
///
/// The transient pen is owned for exactly this call, as in [`draw_check_mark`]; a refused
/// `CreatePen` skips the ellipse (NFR-13: examined — better no circle for one paint than a
/// circle in whatever pen the DC happens to hold).
fn stroke_ellipse(dc: HDC, area: &RECT, outline: Option<COLORREF>, fill: HBRUSH, thickness: i32) {
    let pen = match outline {
        Some(ink) => {
            // SAFETY: takes plain values, reads no memory of ours, answers a handle owned
            // by this frame until the `DeleteObject` below.
            let pen = unsafe { CreatePen(PS_SOLID, thickness, ink) };

            if pen.is_invalid() {
                return;
            }

            Some(pen)
        }
        None => None,
    };

    // SAFETY: `dc` is painted into for the length of the send this call is inside of; the
    // handle selected is either the live pen just made or a stock object, which is owned
    // by the system and never freed by anybody. The previous pen is restored below.
    let previous_pen = match pen {
        Some(pen) => unsafe { SelectObject(dc, pen.into()) },
        None => unsafe { SelectObject(dc, GetStockObject(NULL_PEN)) },
    };

    // SAFETY: as above; `fill` is a live brush of the dialog's state.
    let previous_brush = unsafe { SelectObject(dc, fill.into()) };

    // SAFETY: plain coordinates into a live DC. The answer is dropped for the NFR-13
    // reason the caller states for all its paint calls.
    let _ = unsafe { Ellipse(dc, area.left, area.top, area.right, area.bottom) };

    // SAFETY: both handles were in the DC a moment ago; putting them back ends this
    // function's use of the DC.
    unsafe { SelectObject(dc, previous_brush) };
    unsafe { SelectObject(dc, previous_pen) };

    if let Some(pen) = pen {
        // SAFETY: `pen` was created above, deselected the line before, and freed exactly
        // once, here. The `BOOL` is dropped — see `draw_check_mark`.
        let _ = unsafe { DeleteObject(pen.into()) };
    }
}

// =========================================================================================
// The lengths of the mock-ups, and the two ways this dialog turns its own numbers into
// pixels — FR-92а, tasks T-11-13 and T-11-15
// =========================================================================================

/// The DPI of a screen at 100 % — the ground everything below is measured against, and the
/// answer this file falls back to whenever the device will not say what its DPI is.
pub const SCREEN_DPI: i32 = 96;

/// The scale the mock-ups were **actually** drawn at, in tenths — task T-11-15.
///
/// ⚠ The approved mock-ups `ui-02-graphite.png` and `ui-03-fog.png` are not pictures of a
/// dialog at 100 %. Their generator — `scratchpad\ui.ps1`, line 7 — carries `$DPI = 1.4`,
/// so the pictures were drawn at **140 %** and every length in them is
/// **1,4 × the length at 100 %**: one horizontal dialog unit
/// measures 2,45 px there against 1,75 px at 96 DPI, and the panel of 200 units that is
/// 350 px wide at 100 % is 488 px wide in the picture.
///
/// This number is therefore not decoration: tasks T-11-13 and T-11-14 handed out mock-up
/// pixels believing the pictures were drawn at 100 %, so every `_RADIUS`, `_INSET`,
/// `_WIDTH`, `_EXTRA` and `_TENTHS` constant of this file is 1,4 × too large *as a screen
/// length* — and is right again the moment it goes through [`scaled`], which divides by the
/// DPI of the pictures rather than by 96.
pub const MOCKUP_SCALE_TENTHS: i32 = 14;

/// The DPI of the mock-ups, in **tenths** of a DPI — 96 × 1,4 = **134,4**, task T-11-15.
///
/// Tenths because the number is not whole and rounding it would put the error back: 134 DPI
/// and 135 DPI each miss the picture by about half a percent, and a 33-pixel field of the
/// mock-ups would land a pixel off. Tenths cost one multiplication by ten in each of the two
/// functions below and nothing else.
pub const MOCKUP_DPI_TENTHS: i32 = SCREEN_DPI * MOCKUP_SCALE_TENTHS;

/// One length of the mock-ups in the pixels of a window at `dpi` — the pixel half of «числа
/// масштабируются по DPI окна», and **the one place a mock-up pixel becomes a screen pixel**.
///
/// Pure, and rounded to nearest rather than truncated: a 1 px frame that truncates to zero
/// at 125 % would simply disappear.
///
/// A `dpi` of zero or less — a refused `GetDeviceCaps` — is answered as 96, the 100 % look on
/// a machine that would not say what its DPI is (NFR-13). ⚠ Until task T-11-15 this branch
/// answered the mock-up number *unchanged*, which was the same thing only because the mock-up
/// DPI was believed to be 96; with the true 134,4 it would have handed the refusing machine
/// the 140 % look. The two are separate numbers now and the fallback names its own.
pub fn scaled(pixels: i32, dpi: i32) -> i32 {
    let dpi = if dpi > 0 { dpi } else { SCREEN_DPI };

    (pixels * dpi * 10 + MOCKUP_DPI_TENTHS / 2) / MOCKUP_DPI_TENTHS
}

/// One length of the mock-ups given in **tenths** of a mock-up pixel, in whole pixels of a
/// window at `dpi` — task T-11-14, for the lengths of the mock-ups that are not whole
/// numbers: the 1,5 px stroke of the combo chevron.
///
/// Pure, rounded to nearest like [`scaled`], and never less than one: GDI has no fractional
/// pen, and a pen of zero width is not «thin» but a hairline of one pixel drawn by different
/// rules — asking for one pixel outright is the honest answer. A `dpi` of zero or less is
/// the 100 % look, as in [`scaled`].
pub fn scaled_tenths(tenths: i32, dpi: i32) -> i32 {
    scaled_tenths_offset(tenths, dpi).max(1)
}

/// The same fractional length as [`scaled_tenths`] and **without** its floor of one — task
/// T-11-16, for the fractional lengths that are *positions* rather than pens.
///
/// The generator states the strokes of both check marks and the dot of a radio button as
/// offsets with one decimal — `(PtF ($px+4.5) ($by+8.6))`, `($px+4.6)` — and an offset of zero
/// is a perfectly good offset: a floor of one would push a point off the corner it belongs on.
/// [`scaled_tenths`] is this function with the floor put back, so there is one piece of
/// arithmetic and two names for it (§6.2) rather than two copies drifting apart.
///
/// Pure; a `dpi` of zero or less is the 100 % look, as in [`scaled`].
pub fn scaled_tenths_offset(tenths: i32, dpi: i32) -> i32 {
    let dpi = if dpi > 0 { dpi } else { SCREEN_DPI };

    (tenths * dpi * 10 + MOCKUP_DPI_TENTHS * 5) / (MOCKUP_DPI_TENTHS * 10)
}

/// The DPI of the device a DC paints on — [`SCREEN_DPI`] when the device will not say.
///
/// The manifest of this program declares `PerMonitorV2`, so the DC of a window answers the
/// DPI of *that window's* monitor, which is what «по DPI окна» means on a machine with two
/// screens at different scales.
pub fn dc_dpi(dc: HDC) -> i32 {
    // NFR-13: examined right here. `GetDeviceCaps` answers zero for a DC that is not live,
    // and a zero would turn every scaled length into zero — the fallback is the 100 % look.
    //
    // SAFETY: `dc` is a handle passed by value; the call reads a property of the device and
    // touches no memory of this process.
    let dpi = unsafe { GetDeviceCaps(Some(dc), LOGPIXELSY) };

    if dpi > 0 { dpi } else { SCREEN_DPI }
}

/// A rectangle given in the dialog's own units, in pixels — the *other* half of «числа
/// масштабируются по DPI окна», and the way this dialog has scaled since task T-11-6.
///
/// `MapDialogRect` is the documented conversion, and it is already the dialog's own: the
/// vertical dialog base unit is the height of the dialog font, and the dialog manager
/// creates that font at the DPI of the window — so a dialog unit is a DPI-scaled length by
/// construction, with no arithmetic of ours in the middle. [`on_measure_item`] has measured
/// the combo item height this way since T-11-6; the body was extracted here rather than
/// copied (§6.2), and that function now calls this one.
///
/// `None` for a refused call (NFR-13: examined; every caller then keeps the default it had).
fn dialog_units(hwnd: HWND, horizontal: i32, vertical: i32) -> Option<(i32, i32)> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: horizontal,
        bottom: vertical,
    };

    // SAFETY: `hwnd` is the live dialog and `rect` is a live local the call rewrites in
    // place; nothing else is written.
    unsafe { MapDialogRect(hwnd, &mut rect) }.ok()?;

    Some((rect.right, rect.bottom))
}

/// Corner radius of **everything this dialog rounds off** — group panel, input field, closed
/// part of a combo box, push button and both lists — in the pixels of the mock-ups, п. 1 of
/// task T-11-16.
///
/// ⚠ One constant because the generator of the mock-ups has one number. `scratchpad\ui.ps1`
/// keeps the radius in the style table — `Radius = 6` for «02 Графит» and «03 Туман» — and
/// hands that same `$S.Radius` to every figure it draws: `'group'`, `'edit'`, `'combo'`,
/// `'btn'`, `'def'`, `'lbox'` and `'lview'`. Tasks T-11-13 and T-11-14 read the pictures with
/// the eye and split the number into «панель 6 / прочее 4»; the pictures never carried the
/// split, and the three constants T-11-13 wrote it into are one name again.
///
/// A radius, not the ellipse diameter `RoundRect` takes: [`paint_rounded`] doubles it in the
/// one place that speaks to GDI, so this constant reads as the generator writes it.
pub const CORNER_RADIUS: i32 = 6;

/// Corner radius of the check-box square, in the pixels of the mock-ups — `FillRectPx $g $px
/// $by $bs $bs $S.Mark 3` of the `'check'` arm.
///
/// Its own number and not [`CORNER_RADIUS`] because the generator gives it its own: the square
/// is a small figure, and a 17-pixel square rounded by 6 would read as a lozenge. The circle
/// of a radio button has no radius to set — it is an `Ellipse`.
pub const GLYPH_CORNER_RADIUS: i32 = 3;

/// Corner radius of the tick of the layout list, in mock-up pixels — `FillRectPx $g $bx $by
/// $bs $bs $S.Mark 2` of the `'lview'` arm. Smaller again, because the figure is smaller
/// again: 13 mock-up pixels a side against the 17 of a dialog glyph.
pub const LIST_CHECK_CORNER_RADIUS: i32 = 2;

/// Thickness of the single outline of every rounded figure of the dialog, in the pixels of
/// the mock-ups — every `StrokeRectPx … 1` of `Draw-Control`, task T-11-16.
///
/// ⚠ **One** mock-up pixel, not the two T-11-15 wrote down off the picture: a stroke of
/// GDI+ straddles the path, so a one-pixel pen covers two rows of pixels in the picture and
/// measures as two. Through [`scaled`] the number matters only above 100 %: `.max(1)` keeps
/// the frame a pixel wide wherever the division would round it away, which is what the
/// mock-ups show at every scale.
pub const BORDER_THICKNESS: i32 = 1;

/// Inset of the text of an input field and of the closed part of a combo box from its own
/// left edge, in **dialog units** — п. 2 of task T-11-16.
///
/// ⚠ Dialog units and not pixels, because that is the unit the generator states it in:
/// `Txt … ($c.x + 3) …` in the `'edit'` and `'combo'` arms, where `$c.x` is a coordinate of
/// the dialog grid. Three units are ≈ 5,25 px at 96 DPI. T-11-15 read the same inset off the
/// picture as «12 px макета» — 9 px at 96 DPI, almost twice too much, and the single most
/// visible disagreement with the mock-ups after the radii.
///
/// The three places it reaches, all three through [`dialog_units`]:
///
/// 1. the five input fields — `EM_SETMARGINS`, the documented message ([`set_field_margins`]);
/// 2. the closed part of a combo box ([`draw_combo_closed_part`]);
/// 3. the items of a dropped-down list ([`draw_combo_item`]).
pub const FIELD_TEXT_INSET_DLU: i32 = 3;

/// Inset of the text of one row of the exclusion list from the **left edge of the list**, in
/// mock-up pixels — `TxtPx $g $it $F $brFg ($px + 7) …` of the `'lbox'` arm, п. 3 of T-11-16.
///
/// Mock-up pixels and not dialog units, again because that is what the generator writes: the
/// row of a list is measured from the frame around it, in the pixels of the picture, while
/// the text of a field is measured on the dialog grid. The same 7 is the air before the tick
/// of the layout list ([`check_cell`]).
pub const LIST_TEXT_INSET: i32 = 7;

/// Top of the text of one row of the exclusion list below the top of the row, in mock-up
/// pixels — the `($ry + 2)` of the `'lbox'` arm.
pub const LIST_TEXT_TOP: i32 = 2;

/// Top of the **first** row of a list below the inner edge of its frame, in mock-up pixels —
/// the `$ry = $py + 3 + $i * $rowH` both list arms of the generator start from.
///
/// A list box positions its own rows, starting at the top of its client area, and no message
/// moves them; the one lever this file holds over the distance between the frame and the first
/// row is therefore **where the frame is drawn** — the frame is the dialog's own background
/// ([`on_erase_background`]), and for the two lists it is lifted this far above the control
/// instead of the [`BORDER_THICKNESS`] every field gets.
pub const LIST_FIRST_ROW_TOP: i32 = 3;

/// Inset of the selection rectangle of a list row from the **left and right** edges of the
/// row, in mock-up pixels — `FillRectPx $g ($px+2) $ry ($pw-4) $rowH $S.SelBg 3`.
///
/// Left and right only: the generator gives the stripe the whole height of the row (`$ry`,
/// `$rowH`) and takes 2 px off each side of its width (`$px+2`, `$pw-4`).
pub const LIST_SELECTION_INSET: i32 = 2;

/// Corner radius of that selection rectangle, in mock-up pixels — the trailing `3` of the
/// same call.
pub const LIST_SELECTION_RADIUS: i32 = 3;

/// Height of one row of the exclusion list, in **dialog units** — the `$rowH = Y 11` of the
/// `'lbox'` arm, п. 5 of task T-11-16.
///
/// Dialog units, so the row follows the dialog font and the DPI of the window with no
/// arithmetic of ours in the middle — the same road [`on_measure_item`] already takes for the
/// font height itself.
pub const EXCLUSION_ROW_HEIGHT_DLU: i32 = 11;

/// Height of one row of the layout list, in **dialog units** — the `$rowH = Y 12` of the
/// `'lview'` arm, п. 5 of task T-11-16.
///
/// One unit taller than a row of the exclusion list, exactly as the generator has it. A
/// report-mode `SysListView32` takes no `WM_MEASUREITEM` and has no height message; the
/// documented lever is the **state image list**, whose cell height the control takes as the
/// row height — which is why [`build_check_image_list`] is given this number.
pub const LAYOUT_ROW_HEIGHT_DLU: i32 = 12;

/// Side of the tick of the layout list, in mock-up pixels — the `$bs = 13` of the `'lview'`
/// arm, п. 4 of task T-11-16.
pub const LIST_CHECK_SIZE: i32 = 13;

/// Air between the tick of the layout list and the label after it, in mock-up pixels — the
/// `($bx + $bs + 7)` the generator places the label at.
pub const LIST_CHECK_TEXT_GAP: i32 = 7;

/// Thickness of the pen of the tick of the layout list, in **tenths** of a mock-up pixel —
/// the `[single]1.8` of the `'lview'` arm.
pub const LIST_CHECK_PEN_TENTHS: i32 = 18;

/// Side of the check-box square and diameter of the radio circle, in mock-up pixels — the
/// `$bs = 17` both the `'check'` and the `'radio'` arm of the generator open with, п. 6 of
/// task T-11-16.
///
/// ⚠ Mock-up pixels since this task. It used to be a bare 13 screen pixels — «the size the
/// native glyph draws at 96 DPI» — which is neither the mock-up's figure nor a length that
/// grows with the DPI of the window.
pub const GLYPH_SIZE: i32 = 17;

/// Inset of the caption of a check box or radio button from the **left edge of the element**,
/// in dialog units — `Txt $g $c.s $F $brFg ($c.x + 12) …` in both arms, п. 6 of T-11-16.
///
/// ⚠ Measured from the element, not from the glyph: the glyph stands at the left edge and the
/// text starts twelve units to the right of that same edge, so the air between them is
/// whatever is left over. It used to be a bare «glyph + 5 px» gap, which put the text 18 px
/// from the edge against the mock-up's 21.
pub const GLYPH_TEXT_INSET_DLU: i32 = 12;

/// Inset of the dot of a radio button from the circle around it, in **tenths** of a mock-up
/// pixel — `$p2.AddEllipse(($px+4.6),($by+4.6),($bs-9.2),($bs-9.2))`.
///
/// The diameter of the dot is not a number of its own: 17 − 2 × 4,6 = 7,8 mock-up pixels
/// falls out of the inset and the circle, exactly as it does in the generator.
pub const GLYPH_DOT_INSET_TENTHS: i32 = 46;

/// Inset of the panel caption from the panel's left edge, in **dialog units** — п. 2.1 of
/// T-11-13, `($px + (X 7))` of the `'group'` arm.
///
/// Dialog units and not pixels on purpose: this is a horizontal position on a grid whose
/// every other number — the 7 the panels start at, the 14 their children start at — is in
/// the same units, and [`dialog_units`] is what turns it into pixels.
pub const PANEL_CAPTION_INSET_DLU: i32 = 7;

/// Inset of the panel caption from the panel's top edge, in mock-up pixels — the `($py + 5)`
/// of the same call.
pub const PANEL_CAPTION_INSET_Y: i32 = 5;

/// Point size of the caption face, in **tenths of a point** — the `$FSCAP = 7.6 * $DPI` of
/// the generator, п. 7 of task T-11-16.
///
/// Tenths of a point rather than a percentage of the dialog font, because that is how the
/// generator states it: two point sizes, `7.6` and `9`, whose ratio is 0,844. T-11-15 carried
/// the ratio as a rounded «85 %», which is a number the pictures do not have.
pub const PANEL_CAPTION_POINTS_TENTHS: i32 = 76;

/// Point size of the dialog font itself, in **tenths of a point** — the `$FS = 9 * $DPI` of
/// the generator, and the denominator [`PANEL_CAPTION_POINTS_TENTHS`] is a numerator of.
///
/// Never used to *make* a font: the dialog font comes from the template and the manager, at
/// the DPI of the window. It is here so the ratio the caption is shrunk by is the ratio of
/// the two sizes the pictures were drawn with, and not a percentage written down by hand.
pub const DIALOG_FONT_POINTS_TENTHS: i32 = 90;

/// The height of the dialog font, in vertical **dialog units** — eight of them, by the
/// definition of the vertical dialog base unit.
///
/// Not a length of the mock-ups and not a choice: `MapDialogRect` is documented to map one
/// vertical unit to an eighth of the dialog font's height, so a rectangle this tall maps to
/// exactly one font height in the pixels of the window, whatever face and DPI the manager
/// gave the dialog. It is the one way this file learns the size of a font it never creates.
pub const DIALOG_FONT_HEIGHT_DLU: i32 = 8;

/// Air between the dotted focus cue and the figure it goes around, in **screen pixels** —
/// tasks T-11-5a and T-11-5b, written down as a name by task T-11-16.
///
/// ⚠ Not a length of the mock-ups, and the only length of this file that is not: the pictures
/// draw no focused control at all, so there is nothing in them to take it from. Deliberately
/// **not** scaled either — `DrawFocusRect` paints a hairline of alternating pixels that stays
/// one pixel wide at every DPI, and an inset that grew with the window would pull the cue away
/// from the frame it is supposed to sit just inside of.
const FOCUS_CUE_INSET: i32 = 2;

/// The same air on the left of a cue that goes around a **line of text** rather than a figure —
/// one pixel, because there is no frame on that side for the cue to clear.
const FOCUS_CUE_TEXT_INSET: i32 = 1;

/// Letter spacing of a panel caption, in **tenths** of a mock-up pixel — п. 2.1, «≈ 1,1 px».
///
/// Tenths because the number is not whole and the difference shows: rounding 1,1 px up to
/// 2 px per character stretches «АВТОЗАПУСК» by nine pixels, rounding it down to 1 px loses
/// the spacing the mock-ups have. The pen position is therefore carried in tenths of a pixel
/// through the whole caption and divided only at the moment a character is placed.
pub const PANEL_CAPTION_TRACKING_TENTHS: i32 = 11;

/// Width of a character the DC measures as nothing, as a percentage of the caption font's
/// height — п. 2.1, the explicit space width.
const PANEL_CAPTION_BLANK_PERCENT: i32 = 28;

/// The caption of one group panel, as the mock-ups set it — п. 2.1 of task T-11-13, the
/// pure «строка → выводимая строка» of criterion 11.
///
/// Upper case by the locale of the string itself (`str::to_uppercase` is the full Unicode
/// mapping, so «Горячая клавиша» becomes «ГОРЯЧАЯ КЛАВИША» and «General» becomes
/// «GENERAL»), and **nothing else**: the space inside a caption is a character of the
/// answer like any other. That is the whole reason this is a function with a test rather
/// than a `to_uppercase()` at the call site — the drawing places the characters one by one
/// (the letter spacing of the mock-ups is not a whole number of pixels), and a caption
/// whose spaces went missing on the way to the pen reads «ГОРЯЧАЯКЛАВИША».
pub fn panel_caption(text: &str) -> String {
    text.to_uppercase()
}

/// The advance of one caption character: what the DC measured, or an explicit width when it
/// measured nothing — п. 2.1 of task T-11-13.
///
/// A character-by-character caption asks the DC for the width of one character at a time,
/// and a layout that measures a blank as zero would put the next word on top of the previous
/// one. Nothing is trusted to be non-zero: any measurement that comes back as nothing gets
/// [`PANEL_CAPTION_BLANK_PERCENT`] of the caption font's height instead, which is the width
/// a space has in a face of that size. Pure, so the rule is a table and not a hope.
pub fn caption_advance(measured: i32, font_height: i32) -> i32 {
    if measured > 0 {
        return measured;
    }

    (font_height.abs() * PANEL_CAPTION_BLANK_PERCENT) / 100
}

/// What the dialog's own background draws under one control — FR-92а, task T-11-13, the
/// pure half of the background drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundFigure {
    /// A group panel: the rounded [`theme::Palette::panel_bg`] fill under a single-pixel
    /// [`theme::Palette::panel_border`] frame, over the control's whole rectangle, with the
    /// caption of the (invisible) control at its top edge.
    Panel,
    /// An input field or a list: the rounded [`theme::Palette::field_bg`] fill under a
    /// single-pixel [`theme::Palette::field_border`] frame, one pixel outside the control's
    /// rectangle on every side — so the corners the rounding cuts away are the field's own
    /// colour and not the panel's.
    Field,
}

/// What the background draws under the control with this identifier — `None` for every
/// control the background leaves alone.
///
/// The single place the two lists meet, and the reason the background drawing needs no
/// geometry of its own: it walks the dialog's children, asks this function about each
/// identifier, and paints what it answers inside that child's own rectangle.
pub fn background_figure(control: i32) -> Option<BackgroundFigure> {
    if GROUP_BOXES.contains(&control) {
        return Some(BackgroundFigure::Panel);
    }

    if FRAMED_FIELDS.contains(&control) {
        return Some(BackgroundFigure::Field);
    }

    None
}

/// The five input fields and two lists that lost `WS_BORDER` in task T-11-13 — п. 2.3.
///
/// The system border was a sunken rectangle in the system's colours; these seven now carry
/// the rounded [`theme::Palette::field_border`] frame the mock-ups show, drawn by the
/// dialog's background one pixel outside each rectangle. The list is what
/// [`background_figure`] answers `Field` for, and a test reads the same seven identifiers
/// out of the built binary to check that not one of them kept `WS_BORDER`.
pub const FRAMED_FIELDS: [i32; 7] = [
    IDC_HOTKEY,
    IDC_DELAY,
    IDC_CLIPBOARD_TIMEOUT,
    IDC_CLIPBOARD_RESTORE,
    IDC_EXCLUSION_NAME,
    IDC_EXCLUSIONS,
    IDC_CYCLE_LIST,
];

/// The two of [`FRAMED_FIELDS`] that hold rows rather than a line of text — task T-11-16.
///
/// Named apart because the mock-ups treat them apart in one respect: the frame of a list
/// stands [`LIST_FIRST_ROW_TOP`] mock-up pixels above its first row, where the frame of a
/// field stands one [`BORDER_THICKNESS`] above its text. Both lists are in
/// [`FRAMED_FIELDS`] as well — this is a subset of it and not a second list of controls, and
/// a test holds the containment.
pub const FRAMED_LISTS: [i32; 2] = [IDC_EXCLUSIONS, IDC_CYCLE_LIST];

/// The colours one whole background pass needs, taken out of the state in one borrow.
///
/// The same split every drawing handler of this file makes: the borrow ends before the DC
/// is touched, and what leaves the closure is plain values — three brushes the state keeps
/// alive until the dialog ends, and three inks.
#[derive(Clone, Copy)]
struct BackgroundColors {
    /// [`theme::Palette::window_bg`] — the ground the whole client area starts as.
    window: HBRUSH,
    /// [`theme::Palette::panel_bg`] — the fill of a panel.
    panel: HBRUSH,
    /// [`theme::Palette::field_bg`] — the fill under a field or a list.
    field: HBRUSH,
    /// [`theme::Palette::panel_border`] — the single-pixel frame of a panel.
    panel_border: COLORREF,
    /// [`theme::Palette::field_border`] — the single-pixel frame of a field.
    field_border: COLORREF,
    /// [`theme::Palette::text_muted`] — the ink of a panel caption.
    caption: COLORREF,
}

/// Draws the whole background of the settings dialog — FR-92а, task T-11-13.
///
/// # Why the panels live here and not in `WM_DRAWITEM`
///
/// Until this task each of the eight panels was an owner-drawn `Button` covering the whole
/// area of its block. That made the panel an *element*: it took every click that landed on
/// the empty ground of the block, redrew itself on the click, and — no control of a dialog
/// carries `WS_CLIPSIBLINGS` — laid its own fill straight over its neighbours, which were
/// never told to repaint. One click emptied a whole block. A panel is not an element; a
/// panel **is** the background of its block, and this is where a background is drawn. The
/// eight controls stay in the template, invisible (`NOT WS_VISIBLE` in `app.rc`): an
/// invisible window takes no click, paints nothing and covers nobody, while it keeps the
/// identifier FR-94 sets the caption on and the rectangle that says where the panel goes.
///
/// # What is drawn, in this order
///
/// 1. the whole client area in `window_bg` — the ground the dialog stands on;
/// 2. every panel, in the rectangle of its own hidden control: the rounded fill and frame
///    of [`BackgroundFigure::Panel`], then the caption — upper case, smaller and bolder
///    than the dialog font, letter-spaced, in `text_muted` — read off that same hidden
///    control with `GetDlgItemTextW`, which is what keeps FR-94 working with no second
///    source of truth for the eight strings;
/// 3. every field and list, one pixel outside its own rectangle: the rounded fill and frame
///    of [`BackgroundFigure::Field`].
///
/// Panels before fields, in two passes over one walk of the children: a field can lie on a
/// panel and a panel never lies on a field, so the order is the answer whatever the Z order
/// of the two happens to be. The children themselves paint after this message, on top of
/// all of it — which is what a background is for.
///
/// Answers 1 — «erased» — when the background was drawn, and 0 when the state is
/// unreachable (no state yet, or a re-entrant message) or `theme::Brushes::new` was refused
/// at initialisation: the dialog manager then erases with the answer of `WM_CTLCOLORDLG`,
/// exactly as it did before this task (NFR-13, T-11-4).
///
/// # Safety
///
/// Called from [`dialog_proc`] only, with the `wparam` of the message — the DC the manager
/// owns for the length of the send.
unsafe fn on_erase_background(hwnd: HWND, wparam: WPARAM) -> isize {
    // The one thing taken out of the message (SEC-05): the DC to paint into. It is written
    // to and never read from, and no pointer of the message is followed.
    let dc = HDC(wparam.0 as *mut std::ffi::c_void);

    let mut client = RECT::default();

    // NFR-13: examined — a refused `GetClientRect` means no rectangle to paint in, and the
    // manager's own erase is the degraded-but-alive answer.
    //
    // SAFETY: `hwnd` is the live dialog and `client` is a live local the call fills.
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() {
        return 0;
    }

    let width = client.right - client.left;
    let height = client.bottom - client.top;

    // SAFETY: see the caller — the pointer was stored on `WM_INITDIALOG` and the value it
    // names is alive for the whole of this modal call.
    let choice = unsafe {
        with_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13, T-11-4).
            let brushes = state.brushes.as_ref()?;
            let palette = state.palette;

            // Task T-11-17, criterion 12: is the picture in hand still a picture of *this*
            // window in *this* palette? Both halves are compared before anything is drawn —
            // the size, because the window may have moved to a screen at another scale, and
            // the palette by identity, because `theme::resolve` answers `&'static` and
            // [`refresh_palette`] is the one place it can change.
            let ready = state
                .background
                .as_ref()
                .is_some_and(|picture| picture.shows(width, height, palette));

            Some((
                BackgroundColors {
                    window: brushes.window_bg(),
                    panel: brushes.panel_bg(),
                    field: brushes.field_bg(),
                    panel_border: palette.panel_border,
                    field_border: palette.field_border,
                    caption: palette.text_muted,
                },
                ready,
                palette,
                state.fonts.as_ref().map(DialogFonts::caption),
            ))
        })
    };

    let Some(Some((colors, ready, palette, caption_face))) = choice else {
        return 0;
    };

    let dpi = dc_dpi(dc);

    if !ready {
        // Built **outside** every borrow of the state, exactly as the drawing of every other
        // handler of this file happens outside its own: painting a panel asks the panel for
        // its caption, and a `GetDlgItemTextW` is a send.
        //
        // SAFETY: `dc` is the DC of the message, owned by the manager for the length of this
        // send and read here only for its colour depth; `hwnd` is the live dialog, and
        // `caption_face` a face the state owns for longer than this call.
        let built = unsafe {
            BackgroundCache::build(hwnd, dc, &client, palette, colors, caption_face, dpi)
        };

        // The stale picture is dropped by the assignment; a refused build stores `None`, which
        // the direct painting below answers (NFR-13).
        //
        // SAFETY: as for the borrow above.
        unsafe { with_state(hwnd, |state| state.background = built) };
    }

    // SAFETY: as above. The blit reads the picture's own memory DC and writes the message's.
    let shown = unsafe {
        with_state(hwnd, |state| {
            state
                .background
                .as_ref()
                .is_some_and(|picture| picture.show(dc))
        })
    };

    if shown != Some(true) {
        // No picture — straight into the DC of the message, which is what this handler did
        // before task T-11-17 and is still the right answer when GDI will not give a surface.
        //
        // SAFETY: `dc` is the DC of the message, painted into for the length of this send,
        // and `caption_face` is a face the state owns for longer than this call.
        unsafe { paint_background(hwnd, dc, &client, colors, caption_face, dpi) };
    }

    // TRUE — the background is drawn; the manager must not erase over it.
    1
}

/// The whole background of the dialog, drawn into `dc` — the body [`on_erase_background`] had
/// before task T-11-17, called now against the picture of [`BackgroundCache`] and, when there
/// is no picture, against the DC of the message itself.
///
/// `area` is the rectangle to fill, in the coordinates the children are measured in:
/// `GetClientRect` answers from zero, and [`child_rects_in_client`] answers in the same
/// system, so the picture and the window use one set of numbers.
///
/// # Safety
///
/// `dc` is painted into for the length of the call and is owned by the caller — the DC of the
/// message, or the memory DC of a picture the caller keeps alive; `caption_face` is a face
/// somebody else owns for longer than this call.
unsafe fn paint_background(
    hwnd: HWND,
    dc: HDC,
    area: &RECT,
    colors: BackgroundColors,
    caption_face: Option<HFONT>,
    dpi: i32,
) {
    // SAFETY: `dc` is painted into for the length of this call; `colors.window` is a live
    // brush of the dialog's state.
    unsafe { FillRect(dc, area, colors.window) };

    let children = child_rects_in_client(hwnd);

    // Pass one — the panels. See the doc comment of `on_erase_background` for why they go
    // first.
    for (control, rect) in &children {
        if background_figure(*control) != Some(BackgroundFigure::Panel) {
            continue;
        }

        paint_rounded(
            dc,
            rect,
            scaled(CORNER_RADIUS, dpi),
            colors.panel_border,
            colors.panel,
            dpi,
        );

        if let Some(face) = caption_face {
            // SAFETY: `dc` is the caller's; `face` is a live font the dialog's state owns for
            // longer than this call, and `draw_panel_caption` puts the previous one back.
            unsafe { draw_panel_caption(hwnd, *control, dc, *rect, colors.caption, dpi, face) };
        }
    }

    // Pass two — the fields and lists, one frame's thickness outside each rectangle (п. 2.3
    // of T-11-13; the thickness is [`BORDER_THICKNESS`] through the scale since task
    // T-11-15, so the frame stands outside the control at every DPI and not only at 96,
    // where it is the single pixel it always was).
    let border = scaled(BORDER_THICKNESS, dpi).max(1);

    // …except above a list, where the mock-ups leave [`LIST_FIRST_ROW_TOP`] of their own
    // pixels between the frame and the first row (`$ry = $py + 3 + …` in both list arms of the
    // generator). A list box lays its rows out from the top of its client area and no message
    // moves them, so the air of the picture is made by lifting the frame instead — task
    // T-11-16, п. 5.
    let list_top = scaled(LIST_FIRST_ROW_TOP, dpi).max(border);

    for (control, rect) in &children {
        if background_figure(*control) != Some(BackgroundFigure::Field) {
            continue;
        }

        let top = if FRAMED_LISTS.contains(control) {
            list_top
        } else {
            border
        };

        let frame = RECT {
            left: rect.left - border,
            top: rect.top - top,
            right: rect.right + border,
            bottom: rect.bottom + border,
        };

        paint_rounded(
            dc,
            &frame,
            scaled(CORNER_RADIUS, dpi),
            colors.field_border,
            colors.field,
            dpi,
        );
    }
}

// =========================================================================================
// Стоимость фона: одна готовая картинка вместо восьми панелей на каждую перерисовку —
// FR-92а, task T-11-17, criterion 12
// =========================================================================================

/// The background of the dialog as a finished picture, owned: memory DC and bitmap created
/// together, freed together in `Drop` — NFR-13.
///
/// # Why there is one at all
///
/// `WM_ERASEBKGND` arrives on every repaint, and the background of this dialog is not cheap:
/// eight rounded panels, eight captions set character by character with fractional letter
/// spacing, seven rounded field frames — and since task T-11-17 every one of those figures
/// smooths four corners through an off-screen surface. Doing that work again for a repaint
/// that changed nothing would be paying it for nothing. The picture is built once, and every
/// later erase is one `BitBlt` of the whole client area.
///
/// # When it is rebuilt
///
/// Never repainted in place — rebuilt whole, exactly as `theme::Brushes` is, and for the same
/// reason: a half-updated picture is worse than an old one. [`BackgroundCache::shows`] is the
/// whole test, and it has two halves:
///
/// * the **client size**, because a window that moved to a screen at another scale has other
///   lengths in it and a picture of the old size would be stretched or clipped;
/// * the **palette**, by identity — `theme::resolve` answers `&'static`, and
///   [`refresh_palette`] is the one place in this file the answer can change. A pressed
///   «Применить» and the system switch flipping under `theme = "system"` both go through it,
///   both end in [`repaint_after_palette_change`], and the erase that follows finds the
///   picture painted in the previous palette and builds a new one.
///
/// The interface language is deliberately **not** part of the test: the captions come from the
/// hidden panels ([`draw_panel_caption`] → [`get_text`]), those are written exactly once by
/// [`localise_dialog`] inside `WM_INITDIALOG`, and FR-94 makes a language change take effect
/// on the next start — so no caption of an open dialog can change under the picture.
struct BackgroundCache {
    /// The memory DC the picture is held in, with `bitmap` selected and ready to blit.
    dc: HDC,
    /// The picture itself — compatible with the **window's** DC, not with `dc`: a bitmap
    /// compatible with a memory DC would be monochrome (the trap `build_check_frames` words).
    bitmap: HBITMAP,
    /// The bitmap the fresh memory DC was born with, put back in `Drop` before `bitmap` is
    /// deleted — a bitmap still selected into a DC cannot be freed.
    previous: HGDIOBJ,
    /// Width of the picture, in pixels of the window.
    width: i32,
    /// Height of the picture, in pixels of the window.
    height: i32,
    /// The palette the picture was painted in — compared by identity, see the type's own
    /// documentation.
    palette: &'static theme::Palette,
}

impl BackgroundCache {
    /// Makes the surface and paints the background onto it. `None` on every refusal of GDI
    /// (NFR-13): the caller then paints straight into the DC of the message.
    ///
    /// # Safety
    ///
    /// `target` is the DC of the window the picture is for — read for its colour depth and
    /// never written to; `hwnd` is the live dialog whose children are measured, and
    /// `caption_face` a face the dialog's state owns for longer than this call.
    unsafe fn build(
        hwnd: HWND,
        target: HDC,
        client: &RECT,
        palette: &'static theme::Palette,
        colors: BackgroundColors,
        caption_face: Option<HFONT>,
        dpi: i32,
    ) -> Option<Self> {
        let width = client.right - client.left;
        let height = client.bottom - client.top;

        if width <= 0 || height <= 0 {
            return None;
        }

        // SAFETY: a memory DC over the window's DC; deleted in `Drop` and on every failing
        // path below.
        let dc = unsafe { CreateCompatibleDC(Some(target)) };

        // NFR-13: examined — no DC, no picture.
        if dc.is_invalid() {
            return None;
        }

        // SAFETY: compatible with the *window's* DC — see the field's ⚠ — and owned by this
        // value until `Drop`.
        let bitmap = unsafe { CreateCompatibleBitmap(target, width, height) };

        // NFR-13: examined.
        if bitmap.is_invalid() {
            // SAFETY: deletes exactly the DC made above, once; nothing of ours is in it.
            let _ = unsafe { DeleteDC(dc) };
            return None;
        }

        // SAFETY: both handles are live and ours; the bitmap the memory DC was born with is
        // kept and put back in `Drop`.
        let previous = unsafe { SelectObject(dc, bitmap.into()) };

        // NFR-13: examined — a refused selection would leave the drawing going nowhere.
        if previous.is_invalid() {
            // SAFETY: the bitmap is selected into nothing, so it is free to delete; each
            // handle is freed exactly once.
            let _ = unsafe { DeleteObject(bitmap.into()) };
            let _ = unsafe { DeleteDC(dc) };
            return None;
        }

        let picture = Self {
            dc,
            bitmap,
            previous,
            width,
            height,
            palette,
        };

        let area = RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };

        // SAFETY: `picture.dc` holds the bitmap just selected and is alive for as long as
        // `picture` is; the caller's contract carries `hwnd` and `caption_face`.
        unsafe { paint_background(hwnd, picture.dc, &area, colors, caption_face, dpi) };

        Some(picture)
    }

    /// Whether this picture is still a picture of a window this size in this palette.
    fn shows(&self, width: i32, height: i32, palette: &theme::Palette) -> bool {
        self.width == width && self.height == height && std::ptr::eq(self.palette, palette)
    }

    /// Hands the picture to `dc` — the whole of what a repaint costs once the picture exists.
    ///
    /// Answers whether it was handed over; `false` (NFR-13) sends the caller to the direct
    /// painting, so a refused blit is a slow erase and never an unpainted window.
    fn show(&self, dc: HDC) -> bool {
        // SAFETY: `dc` is the caller's, painted into for the length of the send it is inside
        // of; `self.dc` holds this value's own bitmap. Neither call touches memory of ours.
        unsafe {
            BitBlt(
                dc,
                0,
                0,
                self.width,
                self.height,
                Some(self.dc),
                0,
                0,
                SRCCOPY,
            )
        }
        .is_ok()
    }
}

impl Drop for BackgroundCache {
    fn drop(&mut self) {
        // SAFETY: `self.previous` is the bitmap this DC was born with, kept since `build`;
        // putting it back frees `self.bitmap` to be deleted. The answers are dropped for the
        // reason `theme::Brushes` gives for its own cleanup — there is no journal row for GDI.
        unsafe { SelectObject(self.dc, self.previous) };

        // SAFETY: both came from the successful calls in `build`, were handed to nobody, and
        // are freed exactly once — the type is neither `Copy` nor `Clone`, its fields are
        // private and never reassigned, and `drop` runs once.
        let _ = unsafe { DeleteObject(self.bitmap.into()) };
        let _ = unsafe { DeleteDC(self.dc) };
    }
}

// =========================================================================================
// Серое сглаживание нашего текста — FR-92а, task T-11-17, пункт 2
// =========================================================================================
//
// The second half of the user's decision of 2026-08-22: the text **this file draws itself** is
// set with `ANTIALIASED_QUALITY` — grey antialiasing — instead of the ClearType the dialog
// manager's own font asks for. ClearType tints the edge of every stroke red and blue; against
// the graphite ground of FR-92а that fringe is what `zoom-pairs.png` shows as colour around
// the live captions, and the mock-ups have none of it because they were drawn with grey
// coverage.
//
// ⚠ **What is drawn with this face.** The face covers the text this file draws itself: the
// captions of the nine owner-drawn buttons, the eight panel captions, the rows of both lists,
// the items and the closed face of the four combo boxes, the captions of the eight glyph
// elements — and, since task T-11-18, **every label of both templates**, the twenty-two
// `SS_OWNERDRAW` statics of [`OWNER_DRAWN_LABELS`] and [`OWNER_DRAWN_ABOUT_LABELS`].
//
// Two kinds of text are **not** drawn by this file, and since task T-11-20 they are set in this
// same face all the same — see [`hand_our_face_to_the_controls_that_draw_their_own_text`]:
//
// - the text **inside the five `EDITTEXT` fields** of [`TEXT_FIELDS`] — the three numeric
//   fields, the exclusion name and the read-only hotkey field. An `EDIT` has no owner-drawn
//   type at all: the low nibble of an edit style is not a type, the `ES_*` are flags, and no
//   message hands the parent the drawing of a field's own text;
// - the **row text of the cycle list** `IDC_CYCLE_LIST`. Its rows are coloured through
//   `NM_CUSTOMDRAW`, whose item prepaint sets `clrText`/`clrTextBk` and then answers
//   `CDRF_DODEFAULT` — «draw it yourself, with the fields I set». The control draws the words,
//   in the font it was given.
//
// ⚠ **Task T-11-18 wrote here that owning either «would mean pushing a face of our own onto the
// control with `WM_SETFONT`, which is a change of the dialog's metrics and not of its
// smoothing». That sentence was wrong, and task T-11-20 replaced it with a measurement.** The
// face handed over is [`DialogFonts::text`] — [`antialiased_logfont`] of the window's own
// `LOGFONTW`, one field changed and not a byte else. Same type face, same character height,
// same weight, same character set: `GetTextMetricsW` answers the identical height, ascent,
// descent, internal and external leading, average and maximum character width and weight for
// both, and real strings take the identical number of pixels in both. See the two tests of
// `tests\settings.rs` that measure it. Nothing undocumented is reached for, so the decision
// recorded in `theme-own-draw-decision.md` is untouched: `WM_SETFONT` is the documented way to
// tell a control which face to draw in, and the only thing this one changes is the rasteriser.

/// The `LOGFONTW` of the dialog's own face, asked to render with grey antialiasing — the pure
/// half of [`DialogFonts`], and the whole of «наш текст — серое сглаживание».
///
/// One field changed and not a byte else: the face, the size, the weight and the character set
/// are the window's own, because the dialog font is what the template asks for and what the
/// manager already created at the window's DPI. Pure, so criterion 13 of task T-11-17 is a
/// test on a `LOGFONTW` and needs no window.
pub fn antialiased_logfont(base: LOGFONTW) -> LOGFONTW {
    LOGFONTW {
        // Grey coverage instead of the manager's ClearType — the ⚠ of this section.
        lfQuality: ANTIALIASED_QUALITY,
        ..base
    }
}

/// The `LOGFONTW` of a panel caption — п. 2.1 of task T-11-13, with the antialiasing of
/// task T-11-17 on top.
///
/// Three fields changed against the dialog's own face: the height to
/// [`PANEL_CAPTION_POINTS_TENTHS`] over [`DIALOG_FONT_POINTS_TENTHS`] of what it was (7,6 pt
/// against 9 pt, the two sizes the mock-ups were drawn with), the weight to bold, and the
/// quality — through [`antialiased_logfont`], so there is one place that names the quality and
/// not two. Pure, like it.
pub fn caption_logfont(base: LOGFONTW) -> LOGFONTW {
    let mut logical = antialiased_logfont(base);

    // `lfHeight` is negative for a font asked for by character height, which is how the
    // manager creates a `DS_SETFONT` face; the multiplication keeps whichever sign it has.
    logical.lfHeight = (base.lfHeight * PANEL_CAPTION_POINTS_TENTHS) / DIALOG_FONT_POINTS_TENTHS;
    logical.lfWeight = i32::try_from(FW_BOLD.0).unwrap_or(base.lfWeight);

    logical
}

/// The `LOGFONTW` of the font the dialog manager gave one of the window's own controls.
///
/// The font is asked of a control (`WM_GETFONT` through `SendDlgItemMessageW`, the one way
/// this module sends anything to its own controls): the dialog manager gives its font to every
/// control it creates, and an invisible control has it like any other.
///
/// `None` on every refusal — no font on the control, an unreadable `LOGFONTW` (NFR-13).
fn dialog_logfont(hwnd: HWND, control: i32) -> Option<LOGFONTW> {
    let font = HFONT(send_to(hwnd, control, WM_GETFONT, 0, 0) as *mut std::ffi::c_void);

    // NFR-13: examined. A control with no font of its own answers zero.
    if font.is_invalid() {
        return None;
    }

    let mut logical = LOGFONTW::default();

    // NFR-13: examined — zero is «this is not a font», and there is nothing to copy.
    //
    // SAFETY: `font` is the live font just read off the control; the size argument is the
    // size of `logical`, a live local of this frame, so the call cannot write past it.
    let copied = unsafe {
        GetObjectW(
            font.into(),
            i32::try_from(size_of::<LOGFONTW>()).unwrap_or(0),
            Some((&raw mut logical).cast()),
        )
    };

    if copied == 0 {
        return None;
    }

    Some(logical)
}

/// One `HFONT` from a `LOGFONTW`, examined — `None` for a refused `CreateFontIndirectW`.
fn create_font(logical: LOGFONTW) -> Option<HFONT> {
    // SAFETY: `logical` is a live local of this frame, read by the call; the handle it answers
    // is owned by the caller.
    let created = unsafe { CreateFontIndirectW(&raw const logical) };

    // NFR-13: examined — a refused font is the manager's own face, not a face nobody looked at.
    if created.is_invalid() {
        return None;
    }

    Some(created)
}

/// The two faces one window sets **its own** text in, owned: created together, freed together
/// in `Drop` — FR-92а, task T-11-17.
///
/// Owned rather than made per paint for the reason `theme::Brushes` gives for the brushes: a
/// face selected into a DC has to outlive the drawing, and a paint that makes and deletes a
/// font on every message pays for a face lookup on every message as well. The captions used to
/// be exactly that — [`on_erase_background`] made and deleted one on every erase — and since
/// this task both faces are made once, on `WM_INITDIALOG`, and die with the window's state.
///
/// # Where it lives — section 6.1
///
/// On the UI thread and nowhere else, exactly as `theme::Brushes`: the faces are selected into
/// DCs by window procedures, the windows of this program live on the UI thread, and GDI objects
/// are not shared across threads. Nothing enforces the thread beyond the fact that no other
/// thread ever sees the value.
struct DialogFonts {
    /// The dialog's own face with grey antialiasing — button captions, list rows, combo text,
    /// the captions of the glyph elements.
    text: HFONT,
    /// The same face smaller and bolder, for the eight panel captions.
    caption: HFONT,
}

impl DialogFonts {
    /// Both faces out of the font `control` was given, every handle examined.
    ///
    /// `None` on every refusal — no font on the control, an unreadable `LOGFONTW`, either
    /// `CreateFontIndirectW` declining. The window then draws in the manager's own face, which
    /// is what it drew in before this task: degraded-but-alive, and the whole loss is the
    /// smoothing (NFR-13).
    fn new(hwnd: HWND, control: i32) -> Option<Self> {
        let base = dialog_logfont(hwnd, control)?;

        let text = create_font(antialiased_logfont(base))?;

        let Some(caption) = create_font(caption_logfont(base)) else {
            // SAFETY: `text` was created a line above, handed to nobody, and is freed exactly
            // once here — the failed constructor answers `None` and no `Drop` will run.
            let _ = unsafe { DeleteObject(text.into()) };
            return None;
        };

        Some(Self { text, caption })
    }

    /// The face of the window's own text, borrowed — the owner frees it, nobody else.
    fn text(&self) -> HFONT {
        self.text
    }

    /// The face of a panel caption, borrowed — the owner frees it, nobody else.
    fn caption(&self) -> HFONT {
        self.caption
    }
}

impl Drop for DialogFonts {
    fn drop(&mut self) {
        for face in [self.text, self.caption] {
            // SAFETY: each came from a successful `CreateFontIndirectW` in `new` and is freed
            // exactly once: the type is neither `Copy` nor `Clone`, its fields are private and
            // never reassigned, and `drop` runs once. Every drawing that selected a face put
            // the previous one back before it returned, so neither is in a DC any more. The
            // `BOOL` is dropped for the reason `theme::Brushes` gives for its own cleanup.
            let _ = unsafe { DeleteObject(face.into()) };
        }
    }
}

/// Selects a face of ours into `dc` for the length of one piece of drawing — task T-11-17.
///
/// Answers what was in the DC before, for [`restore_face`] to put back. `None` in, `None`
/// out: a window whose faces could not be made draws in the manager's own font, which is what
/// every task before this one drew in (NFR-13).
///
/// # Safety
///
/// `dc` is painted into for the length of the send the caller is inside of, and `face` is a
/// live font somebody else owns for longer than the drawing.
unsafe fn select_face(dc: HDC, face: Option<HFONT>) -> Option<HGDIOBJ> {
    let face = face?;

    // SAFETY: see the contract above; the previous handle is answered to the caller, which
    // hands it to `restore_face`.
    Some(unsafe { SelectObject(dc, face.into()) })
}

/// Puts back what [`select_face`] took out. `None` — nothing was selected — does nothing.
///
/// # Safety
///
/// `previous` is the handle [`select_face`] answered for this same DC, and no other selection
/// happened in between.
unsafe fn restore_face(dc: HDC, previous: Option<HGDIOBJ>) {
    if let Some(previous) = previous {
        // SAFETY: see the contract above.
        unsafe { SelectObject(dc, previous) };
    }
}

/// Draws the caption of one panel, character by character — п. 2.1 of task T-11-13.
///
/// The text is the **hidden control's own**, read by identifier ([`get_text`] →
/// `GetDlgItemTextW`) and put through [`panel_caption`]; FR-94 rewrites those controls when
/// the interface language changes, so the captions follow the language with no second store
/// of the eight strings anywhere.
///
/// Character by character because the letter spacing of the mock-ups is 1,1 px and GDI has
/// no fractional advance: the pen position is carried in tenths of a pixel across the whole
/// caption and divided down only where a character is placed, so the error never
/// accumulates. Every character's own width comes from the DC, through [`caption_advance`],
/// which is where a character the DC measures as nothing gets an explicit width instead —
/// without it the words of a caption run together.
///
/// # Safety
///
/// `dc` is the DC of the message the caller is inside of, and `face` is a live font the
/// caller owns for longer than this call.
unsafe fn draw_panel_caption(
    hwnd: HWND,
    control: i32,
    dc: HDC,
    panel: RECT,
    ink: COLORREF,
    dpi: i32,
    face: HFONT,
) {
    let caption = panel_caption(&get_text(hwnd, control));

    if caption.is_empty() {
        return;
    }

    // SAFETY: `dc` is painted into for the length of the send this call is inside of, and
    // `face` is the live font of the caller. The previous font is put back below.
    let previous_font = unsafe { SelectObject(dc, face.into()) };

    // NFR-13, for the calls below: each answers a previous value or a success flag, and
    // every answer is deliberately dropped for the reason [`on_draw_item`] gives for its
    // own paint calls.
    //
    // SAFETY: `dc` is a handle passed by value; both calls write an attribute of the DC.
    unsafe { SetBkMode(dc, TRANSPARENT) };
    // SAFETY: as above.
    unsafe { SetTextColor(dc, ink) };

    // The caption font's cell height, for the explicit width of a character the DC measures
    // as nothing: the dialog font's height ([`dialog_units`], the dialog's own scaling)
    // taken down to the same percentage the face was.
    let font_height = dialog_units(hwnd, 0, DIALOG_FONT_HEIGHT_DLU)
        .map(|(_, height)| (height * PANEL_CAPTION_POINTS_TENTHS) / DIALOG_FONT_POINTS_TENTHS)
        .unwrap_or(0);

    // 7 dialog units from the left edge of the panel, 5 mock-up pixels from the top.
    let inset_x = dialog_units(hwnd, PANEL_CAPTION_INSET_DLU, 0)
        .map(|(horizontal, _)| horizontal)
        .unwrap_or(PANEL_CAPTION_INSET_DLU);

    let top = panel.top + scaled(PANEL_CAPTION_INSET_Y, dpi);
    let tracking = scaled(PANEL_CAPTION_TRACKING_TENTHS, dpi);

    // Tenths of a pixel — see the doc comment.
    let mut pen_tenths = (panel.left + inset_x) * 10;

    for character in caption.chars() {
        let mut units = [0u16; 2];
        let encoded = character.encode_utf16(&mut units);

        let mut extent = SIZE::default();

        // NFR-13: examined — a refused measurement is «nothing», which
        // [`caption_advance`] turns into the explicit width.
        //
        // SAFETY: `encoded` and `extent` are live locals of this frame; the call reads the
        // one character and writes only `extent`.
        let measured = if unsafe { GetTextExtentPoint32W(dc, encoded, &mut extent) }.as_bool() {
            extent.cx
        } else {
            0
        };

        // NFR-13: the `BOOL` is examined and deliberately dropped — see the block comment
        // above.
        //
        // SAFETY: `encoded` is a live local of this frame and the DC is the caller's.
        let _ = unsafe { TextOutW(dc, pen_tenths / 10, top, encoded) };

        pen_tenths += caption_advance(measured, font_height) * 10 + tracking;
    }

    // SAFETY: the handle was in the DC a moment ago; putting it back ends this function's
    // use of the DC and leaves the caller free to delete `face`.
    unsafe { SelectObject(dc, previous_font) };
}

// =========================================================================================
// Своё сглаживание краёв: сверхдискретизация и уменьшение — FR-92а, task T-11-17
// =========================================================================================
//
// GDI does not smooth an edge. A circle, a check mark, a chevron and the corner of a rounded
// rectangle therefore leave `Ellipse`, `Polyline` and `RoundRect` as a staircase, while the
// mock-ups of FR-92а were drawn by an engine that smooths every one of them. The decision of
// the user (2026-08-22) is to smooth them **with our own hands, on plain GDI** — no second
// drawing library and no new feature, «программа остаётся самодостаточной» — by the one
// technique plain GDI has for it, and the same one `tools\make-icons.ps1` already uses: draw
// the figure enlarged, then average it back down.
//
// One piece of a picture is smoothed in three steps, which are [`Supersample::render`]:
//
// 1. the ground the figure will stand on is enlarged [`SUPERSAMPLE`] times each way into an
//    off-screen surface — a plain replication, because enlarging blends nothing;
// 2. the figure is drawn into that surface at [`SUPERSAMPLE`] times its size, so one pixel of
//    the window is `SUPERSAMPLE × SUPERSAMPLE` pixels there;
// 3. the surface is averaged back down **by arithmetic of ours** — [`Supersample::reduce`] adds
//    the `SUPERSAMPLE × SUPERSAMPLE` samples behind every destination pixel and divides — and
//    the reduced tile is handed to the caller's DC by a one-to-one `BitBlt`. That average **is**
//    the smoothing: a pixel the figure covers by a third comes back as a third of the figure's
//    colour over two thirds of the ground.
//
// ⚠ Step 3 was a `StretchBlt` in `HALFTONE` mode until task T-11-23, on the belief that the mode
// averages a block. **It does not, and the belief was measured false.** `HALFTONE` *halftones*:
// MSDN words it as «the average color over the destination **block of pixels** approximates the
// color of the source pixels», which is a dither — the area comes out right and the single pixel
// does not. Measured on this machine, an arc reduced 4 : 1 from ground `32,35,41` to ink
// `228,231,234` came back with **116 of 256 pixels outside the two colours it was made of**, the
// darkest `2,5,12` and the brightest `255,255,255` — an overshoot of −30 and +27 past the ends.
// The same bits averaged by hand gave **none**: a mean of numbers cannot leave the range of the
// numbers it is a mean of, which is exactly why the arithmetic below is the fix and why the test
// `the_reduction_of_a_block_is_its_average_and_never_leaves_its_extremes` states it. (Steps 1
// and 2 were measured innocent in the same pass: `COLORONCOLOR` replicated a flat tile to the
// pixel, and `HALFTONE` itself left a flat tile alone — only an *edge* rang.)
//
// ⚠ The documented, silent trap of reading a DIB section back: the pixels GDI drew are not
// necessarily in memory yet — part of the drawing can still sit in the batch queue — until
// `GdiFlush` has been called. Without it there is no error and no refusal, just an occasionally
// stale picture. [`Supersample::reduce`] flushes before it reads the first sample.
//
// # Стоимость — что смягчается целиком и что только по углам
//
// Sixteen times the pixels is sixteen times the work, so **what** is enlarged decides whether
// this is affordable at all. Two rules, and no third:
//
// * a **small glyph** — the circle of a radio button, its dot, either check mark, the chevron
//   of a combo box — is enlarged **whole**. The largest of them is under 24 px a side even at
//   150 %, so the surface is under 96 × 96 = 9 216 pixels, and [`SUPERSAMPLE_MAX_SIDE`] is the
//   ceiling that keeps it that way: a piece larger than that is drawn the old aliased way
//   rather than dragged through a buffer.
// * a **rounded rectangle** — panel, field, list frame, button, closed combo box, selection
//   stripe — is enlarged **at its four corners only**. Enlarging a 350 × 124 panel whole would
//   ask for 1 400 × 496 = 694 400 pixels, and eight panels on every repaint of the background
//   is not a cost this dialog may pay. The corner square is `radius + thickness` a side — 5 px
//   at 96 DPI, 7 at 140 % — so the four of them together are ~100 px enlarged to ~1 600, which
//   is **four hundred times less** than the whole figure. The straight edges have no staircase
//   to remove: they are drawn by the same `RoundRect` as before, with the four corner squares
//   held out of the clip so the smoothed corner is blended into the true ground and not into
//   an aliased corner drawn a moment earlier.
//
// The background of the dialog — the ground, the eight panels with their captions and the
// seven field frames — is built **once** into a picture and handed over with one `BitBlt`
// afterwards ([`BackgroundCache`]), so even the corner arithmetic above is paid at the first
// paint and at a palette change, and not on every `WM_ERASEBKGND`.

/// How many times each way a figure is enlarged before it is averaged back down — the
/// «кратность сверхдискретизации» of task T-11-17.
///
/// Four, not two and not eight: two gives three levels of coverage per pixel and still reads as
/// a staircase on a shallow curve, eight costs four times as much as four for a difference the
/// eye does not find on a 12-pixel glyph. Sixteen samples per pixel is the classic choice of
/// this technique and the one `tools\make-icons.ps1` already draws icons at.
pub const SUPERSAMPLE: i32 = 4;

/// The largest side, in pixels of the window, a piece may have and still be enlarged whole.
///
/// A ceiling and not a preference: `SUPERSAMPLE_MAX_SIDE` squared times `SUPERSAMPLE` squared
/// is the largest surface this file will ever ask for — 256 × 256 pixels, a quarter of a
/// megabyte at four bytes each. Everything the dialog smooths whole is far below it (the
/// tallest glyph is under 24 px at 150 %), so the ceiling never bites in practice; it is there
/// so that a future caller cannot quietly turn a smoothed detail into a smoothed panel.
pub const SUPERSAMPLE_MAX_SIDE: i32 = 64;

/// How many samples of the enlarged surface stand behind one pixel of the window — the block
/// [`average_of_block`] averages, and the length of the array [`Supersample::reduce`] gathers
/// each one into.
pub const SUPERSAMPLE_BLOCK: usize = (SUPERSAMPLE * SUPERSAMPLE) as usize;

/// The mean of one block of samples — the box filter that step 3 of the smoothing **is**, and
/// the whole of task T-11-23.
///
/// Each sample is a whole 32-bit `BI_RGB` pixel of the enlarged surface, which stores blue,
/// green and red in the low three bytes in that order; the answer is one pixel of the same
/// shape. The three bytes are averaged where they lie, so this function needs no opinion about
/// which channel is which, and the fourth byte — not a colour — comes back zero rather than
/// averaged.
///
/// # The property this exists for
///
/// **A mean cannot leave the range of the numbers it is a mean of.** That is the one sentence
/// task T-11-23 turns on: until it, step 3 was a `StretchBlt` in `HALFTONE` mode, which
/// *dithers* — it makes the average over an area right by making the individual pixel wrong,
/// and the wrong pixels were the halo the user saw. Measured, the same arc came back with 116
/// of 256 pixels outside the two colours it was drawn from, overshooting by −30 and +27; this
/// function cannot produce one, and
/// `the_reduction_of_a_block_is_its_average_and_never_leaves_its_extremes` says so.
///
/// Rounding is to the nearest and not down: a box filter that truncated would darken every
/// smoothed edge by half a level, which over a whole dialog is a visible tint.
///
/// Pure, and public for the reason [`corner_tiles`] and [`stroke_bounds`] are: a test hands it
/// a block whose average is known on paper and compares, with no window and no DC in sight.
/// An empty block has no mean and answers zero — the caller never passes one, and a division
/// by zero is not a thing this file will risk on the paint path.
pub fn average_of_block(samples: &[u32]) -> u32 {
    let count = samples.len() as u64;

    if count == 0 {
        return 0;
    }

    // One accumulator per colour byte, low byte first. `u64` so that the sum of an arbitrarily
    // long block cannot wrap — the product only ever hands over [`SUPERSAMPLE_BLOCK`] samples,
    // but a public function may be handed anything.
    let mut sums = [0u64; 3];

    for pixel in samples {
        for (channel, sum) in sums.iter_mut().enumerate() {
            *sum += u64::from((pixel >> (8 * channel)) & 0xFF);
        }
    }

    let mean = |sum: u64| ((sum + count / 2) / count) as u32;

    mean(sums[0]) | (mean(sums[1]) << 8) | (mean(sums[2]) << 16)
}

/// How far the **path** of an enlarged stroke moves so that the stroke lands back on the pixels
/// the unenlarged one would have covered — the half-pixel of an odd pen, in enlarged pixels.
///
/// ⚠ Not decoration, and the one piece of arithmetic without which this whole technique
/// produces a *seam*. GDI centres a pen on its path, and a pen of even width has no centre
/// pixel: measured on this machine, a pen `w` wide on a path at `p` covers
/// `p − w/2 … p + w/2 − 1`. Enlarge a figure by [`SUPERSAMPLE`] and its pen with it, and the
/// band the enlarged pen covers is no longer the band the plain one covered multiplied by four —
/// for an odd thickness it straddles the block boundary, and the reduction turns a crisp
/// one-pixel frame into two half-lit ones. That is invisible while a whole figure is smoothed
/// and *very* visible where a smoothed corner meets the straight edge [`paint_rounded`] leaves
/// aliased.
///
/// The correction is `SUPERSAMPLE / 2` for an odd thickness and nothing for an even one:
///
/// * `t = 1`, path `p` → covers `[p, p]`; enlarged `4p + 2` with pen 4 → covers
///   `[4p, 4p + 3]`, which is exactly block `p`;
/// * `t = 2`, path `p` → covers `[p − 1, p]`; enlarged `4p` with pen 8 → covers
///   `[4p − 4, 4p + 3]`, exactly blocks `p − 1` and `p`.
///
/// Pure, and closed by a test that draws both and compares the pixels.
pub fn stroke_shift(thickness: i32) -> i32 {
    if thickness % 2 == 0 {
        0
    } else {
        SUPERSAMPLE / 2
    }
}

/// The enlarged surface a figure is drawn onto, with the arithmetic that takes a length of the
/// window into a length of that surface.
///
/// Handed to the closure of [`Supersample::render`] so that the drawing inside it never has to
/// know where the tile sits on the screen: it names the same rectangles and the same points it
/// would have named on the window, and every one of them goes through this type.
#[derive(Clone, Copy)]
struct Canvas {
    /// The memory DC of the enlarged surface, with its bitmap already selected.
    dc: HDC,
    /// Left edge of the tile on the window — the point that is `0` on the surface.
    origin_x: i32,
    /// Top edge of the tile on the window — the point that is `0` on the surface.
    origin_y: i32,
    /// [`stroke_shift`] of the pen the figure is stroked with — zero for a figure with no pen.
    shift: i32,
}

impl Canvas {
    /// One **path** point of the window on the enlarged surface — the line a pen runs along,
    /// with the correction of [`stroke_shift`].
    fn point(&self, x: i32, y: i32) -> (i32, i32) {
        (
            (x - self.origin_x) * SUPERSAMPLE + self.shift,
            (y - self.origin_y) * SUPERSAMPLE + self.shift,
        )
    }

    /// One rectangle of a **stroked** figure — `RoundRect` and `Ellipse` — on the enlarged
    /// surface.
    ///
    /// ⚠ `right` and `bottom` are exclusive on both sides of the conversion, but the path they
    /// name is one short of them: a figure given `(L, T, R, B)` is bounded by
    /// `L … R − 1`. So the far edge is converted as the path it is and turned back into an
    /// exclusive coordinate afterwards — multiplying `R` outright would put the far edge a
    /// quarter of a pixel wrong and re-open the seam [`stroke_shift`] exists to close.
    ///
    /// The rectangle may well fall outside the surface — a corner tile is drawn by naming the
    /// **whole** figure and letting GDI clip it to the 20-odd pixels the tile holds, which is
    /// the whole reason a corner comes out of the same `RoundRect` as the figure it belongs to.
    fn outline(&self, area: &RECT) -> RECT {
        let (left, top) = self.point(area.left, area.top);
        let (right, bottom) = self.point(area.right - 1, area.bottom - 1);

        RECT {
            left,
            top,
            right: right + 1,
            bottom: bottom + 1,
        }
    }

    /// One rectangle of a figure with **no pen** — the dot of a radio button, whose whole edge
    /// is the fill. A fill has no path to centre anything on, so the plain multiplication is
    /// the right conversion here and the corrections above would shrink the figure.
    fn area(&self, area: &RECT) -> RECT {
        RECT {
            left: (area.left - self.origin_x) * SUPERSAMPLE,
            top: (area.top - self.origin_y) * SUPERSAMPLE,
            right: (area.right - self.origin_x) * SUPERSAMPLE,
            bottom: (area.bottom - self.origin_y) * SUPERSAMPLE,
        }
    }

    /// One length of the window on the enlarged surface — a radius, a pen thickness.
    fn length(&self, pixels: i32) -> i32 {
        pixels * SUPERSAMPLE
    }
}

/// An off-screen surface [`SUPERSAMPLE`] times the size of the piece being smoothed, owned:
/// memory DC and DIB section created together, freed together in `Drop` — NFR-13.
///
/// One value is made per figure and used for each of its pieces in turn: a rounded rectangle
/// has four corners and makes one surface, not four. Nothing here is cached between figures —
/// a `CreateCompatibleDC` and a `CreateDIBSection` of a few kilobytes are cheap beside the
/// enlarged drawing itself, and a surface kept alive between paints would be a GDI object held
/// for as long as the dialog is up in exchange for nothing.
struct Supersample {
    /// The memory DC the enlarged figure is drawn through.
    dc: HDC,
    /// The 32-bit top-down DIB section selected into `dc`.
    bitmap: HBITMAP,
    /// The pixels of `bitmap`, as `CreateDIBSection` handed them over — the whole reason the
    /// reduction of task T-11-23 can be arithmetic instead of a `HALFTONE` blit.
    ///
    /// One 32-bit pixel per element, `width` of them per row, `height` rows, top row first: a
    /// 32-bit `BI_RGB` section has no padding to skip, because a row of it is a whole number of
    /// `DWORD`s by construction. The memory belongs to the section and is freed with it in
    /// `Drop`; nothing outside [`Supersample::reduce`] ever follows this pointer.
    bits: *mut u32,
    /// The bitmap the fresh memory DC was born with — put back before `bitmap` is deleted,
    /// because a bitmap still selected into a DC cannot be freed.
    previous: HGDIOBJ,
    /// Width of the surface, in its own enlarged pixels.
    width: i32,
    /// Height of the surface, in its own enlarged pixels.
    height: i32,
}

impl Supersample {
    /// The surface for a tile `width` × `height` pixels **of the window**.
    ///
    /// `None` for a tile of nothing, for a tile past [`SUPERSAMPLE_MAX_SIDE`] — the cost
    /// ceiling — and for every refusal of GDI (NFR-13): the caller then draws the figure the
    /// aliased way it drew it before this task, which is degraded-but-alive in the exact sense
    /// of the requirement.
    fn for_tile(width: i32, height: i32) -> Option<Self> {
        if width <= 0
            || height <= 0
            || width > SUPERSAMPLE_MAX_SIDE
            || height > SUPERSAMPLE_MAX_SIDE
        {
            return None;
        }

        Self::new(width * SUPERSAMPLE, height * SUPERSAMPLE)
    }

    /// The surface itself, `width` × `height` of its **own** pixels. Every handle examined.
    fn new(width: i32, height: i32) -> Option<Self> {
        // SAFETY: a memory DC over the screen — no reference DC of ours is needed, and one
        // taken from a window would tie this surface to a window it does not belong to. It is
        // deleted in `Drop`, and on every failing path below.
        let dc = unsafe { CreateCompatibleDC(None) };

        // NFR-13: examined — no DC, no surface.
        if dc.is_invalid() {
            return None;
        }

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                // Negative — the rows run top down, so a coordinate on this surface means what
                // it means on the window and the figure is not drawn upside down.
                biHeight: -height,
                biPlanes: 1,
                // Thirty-two bits: full colour for the averaging to average, one pixel per
                // `u32` for [`Supersample::reduce`] to read, and no row padding to skip.
                // ⚠ A bitmap compatible with a *memory* DC would be monochrome — the classic
                // trap `build_check_frames` words as well.
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();

        // SAFETY: `info` is a fully initialised header living on this frame and read by the
        // call; `bits` receives the address of the pixels the section owns, which stays valid
        // for as long as the section does — that is, until the `DeleteObject` in `Drop` — and
        // is followed by [`Supersample::reduce`] and by nothing else. `DIB_RGB_COLORS` needs no
        // palette, so no colour table is read past the header.
        let created = unsafe {
            CreateDIBSection(
                Some(dc),
                &raw const info,
                DIB_RGB_COLORS,
                &raw mut bits,
                None,
                0,
            )
        };

        // NFR-13: examined, both ways the call can decline.
        let Ok(bitmap) = created else {
            // SAFETY: deletes exactly the DC created above, once; nothing of ours is selected
            // into it.
            let _ = unsafe { DeleteDC(dc) };
            return None;
        };

        if bitmap.is_invalid() {
            // SAFETY: as above.
            let _ = unsafe { DeleteDC(dc) };
            return None;
        }

        // NFR-13: examined too. A section that answered a handle and no pixels is not one this
        // type can reduce, and a null read a pixel at a time is the one failure that would not
        // announce itself — better no smoothing at all (the caller's aliased fallback).
        if bits.is_null() {
            // SAFETY: the bitmap was selected into nothing, so it is free to delete; the DC is
            // deleted after it, exactly once each.
            let _ = unsafe { DeleteObject(bitmap.into()) };
            let _ = unsafe { DeleteDC(dc) };
            return None;
        }

        // SAFETY: both handles are live and ours; the bitmap the memory DC was born with is
        // kept and put back in `Drop`.
        let previous = unsafe { SelectObject(dc, bitmap.into()) };

        // NFR-13: examined — a refused selection means an empty DC, and drawing into one
        // would silently produce nothing.
        if previous.is_invalid() {
            // SAFETY: the bitmap was selected into nothing, so it is free to delete; the DC is
            // deleted after it, exactly once each.
            let _ = unsafe { DeleteObject(bitmap.into()) };
            let _ = unsafe { DeleteDC(dc) };
            return None;
        }

        // Enlarging *into* this surface must replicate and not blend — the ground is carried
        // up so the figure can be blended into it on the way down, and a ground that arrived
        // already smeared would smear the whole tile. NFR-13: the previous mode is answered and
        // deliberately dropped — this DC is one line old and nobody else can hold its mode.
        unsafe { SetStretchBltMode(dc, COLORONCOLOR) };

        Some(Self {
            dc,
            bitmap,
            bits: bits.cast::<u32>(),
            previous,
            width,
            height,
        })
    }

    /// Draws `figure` into `tile` of `dc`, smoothed — the three steps of this section.
    ///
    /// `thickness` is the pen the figure is stroked with, in pixels of the **window**: it is
    /// what [`stroke_shift`] needs, and handing it in here is what keeps every drawing closure
    /// from having to remember the correction. Zero for a figure with no pen at all.
    ///
    /// Answers whether the tile was painted. `false` — a refused blit, a tile larger than the
    /// surface — leaves the tile exactly as it was, and the caller falls back to the aliased
    /// drawing (NFR-13).
    fn render(&self, dc: HDC, tile: &RECT, thickness: i32, figure: impl FnOnce(Canvas)) -> bool {
        let width = tile.right - tile.left;
        let height = tile.bottom - tile.top;

        if width <= 0 || height <= 0 {
            return false;
        }

        let enlarged_width = width * SUPERSAMPLE;
        let enlarged_height = height * SUPERSAMPLE;

        if enlarged_width > self.width || enlarged_height > self.height {
            return false;
        }

        // 1. The ground, enlarged. `COLORONCOLOR` was set on this DC when it was made, so the
        // pixels are replicated and not blended.
        //
        // SAFETY: both DCs are live — `dc` is the caller's, painted into for the length of the
        // send it is inside of, and `self.dc` holds our own bitmap. Neither call touches memory
        // of this process.
        let enlarged = unsafe {
            StretchBlt(
                self.dc,
                0,
                0,
                enlarged_width,
                enlarged_height,
                Some(dc),
                tile.left,
                tile.top,
                width,
                height,
                SRCCOPY,
            )
        };

        // NFR-13: examined — without the ground there is nothing to blend into, and the
        // aliased fallback of the caller is the honest answer.
        if !enlarged.as_bool() {
            return false;
        }

        // 2. The figure, at `SUPERSAMPLE` times its size, on a canvas that already carries the
        // path correction its pen needs — see [`stroke_shift`].
        figure(Canvas {
            dc: self.dc,
            origin_x: tile.left,
            origin_y: tile.top,
            shift: stroke_shift(thickness),
        });

        // 3. Back down, averaging — arithmetic of ours over the section's own pixels, never a
        // `HALFTONE` blit. See the two ⚠ of this section for what that mode actually did and
        // for the `GdiFlush` this step cannot be read without. Nothing here touches an
        // attribute of the caller's DC, so there is no state to put back either.
        self.reduce(width, height);

        // The reduced tile now sits in the top-left `width × height` corner of the surface and
        // goes over one to one — a `BitBlt` and not a `StretchBlt`, because there is nothing
        // left to scale and a one-to-one copy cannot invent a colour.
        //
        // SAFETY: both DCs are live — `dc` is the caller's, painted into for the length of the
        // send it is inside of, and `self.dc` holds our own bitmap. Neither call touches memory
        // of this process.
        //
        // NFR-13: examined — the caller repaints the tile the aliased way if this refused.
        unsafe {
            BitBlt(
                dc,
                tile.left,
                tile.top,
                width,
                height,
                Some(self.dc),
                0,
                0,
                SRCCOPY,
            )
        }
        .is_ok()
    }

    /// Averages every `SUPERSAMPLE × SUPERSAMPLE` block of the enlarged surface into one pixel,
    /// leaving the `width × height` result in the surface's own top-left corner — the reduction
    /// of task T-11-23, and the whole of what step 3 is now.
    ///
    /// # Why the average is done here and not by GDI
    ///
    /// Because GDI has no call that does it. `HALFTONE` was believed to and was measured not to
    /// — see the ⚠ of this section — and there is no other reducing mode: `COLORONCOLOR` and
    /// `BLACKONWHITE` throw whole rows away. An average of sixteen numbers, on the other hand,
    /// **cannot** leave the range of those sixteen by construction, which is the property the
    /// halo of T-11-23 was the absence of.
    ///
    /// # Why in place, and why that is not an aliasing bug
    ///
    /// The result is written back into the same surface, which costs neither a second DIB
    /// section nor a heap buffer. It is safe by the order of the walk, and the argument is
    /// worth writing down because it is the one thing that could quietly rot here:
    ///
    /// * destination pixel `(x, y)` is written to row `y`, column `x`, and reads rows
    ///   `SUPERSAMPLE·y … SUPERSAMPLE·y + SUPERSAMPLE − 1`;
    /// * for `y ≥ 1` those rows are all past row `y`, and row `y` was consumed by block row
    ///   `y / SUPERSAMPLE`, which is strictly earlier than `y` — so the write lands on a row
    ///   already used up;
    /// * for `y = 0` the write and the reads share the row, and there the columns separate them:
    ///   `(x, 0)` writes column `x` and every read still to come in that row is at column
    ///   `SUPERSAMPLE·(x + 1)` or further, which is past `x` for every `x ≥ 0`.
    ///
    /// # Cost
    ///
    /// `SUPERSAMPLE²` = 16 reads and three additions each per destination pixel. The ceiling is
    /// [`SUPERSAMPLE_MAX_SIDE`]²·`SUPERSAMPLE`² = 65 536 samples for one tile, and the tiles
    /// this dialog actually reduces are the four 5 × 5 corners of a rounded rectangle (400
    /// samples each) and glyphs under 24 px a side (under 9 216). Measured against the
    /// `HALFTONE` blit it replaces, in `tests\settings.rs`.
    ///
    /// Answers nothing: there is no way for arithmetic over memory the section owns to fail, and
    /// the failure that *would* matter — a section that handed over no pixels — is refused in
    /// [`Supersample::new`], so a live `Supersample` always has somewhere to read.
    fn reduce(&self, width: i32, height: i32) {
        // ⚠ The silent trap: the drawing above may still be in the batch queue, and reading the
        // pixels before it has been flushed reads whatever was there before. No error, no
        // refusal — just an occasionally stale tile.
        //
        // SAFETY: takes nothing and touches no memory of ours. NFR-13: the `BOOL` is examined
        // in words and dropped — `GdiFlush` answers `FALSE` only for a batch it could not
        // play back, and the next thing this function does is read what did get through; there
        // is no better answer available than to average what is there.
        let _ = unsafe { GdiFlush() };

        let stride = self.width as usize;
        let mut block = [0u32; SUPERSAMPLE_BLOCK];

        for y in 0..height {
            for x in 0..width {
                for sample_y in 0..SUPERSAMPLE {
                    let row = (y * SUPERSAMPLE + sample_y) as usize * stride;

                    for sample_x in 0..SUPERSAMPLE {
                        // SAFETY: `self.bits` is the non-null pointer `CreateDIBSection`
                        // answered, valid until the `DeleteObject` of `Drop`, and it addresses
                        // `self.width * self.height` pixels. The index is inside them:
                        // `render` has already refused a tile whose enlargement is wider or
                        // taller than the surface, so `y * SUPERSAMPLE + sample_y < self.height`
                        // and `x * SUPERSAMPLE + sample_x < self.width`.
                        block[(sample_y * SUPERSAMPLE + sample_x) as usize] = unsafe {
                            self.bits
                                .add(row + (x * SUPERSAMPLE + sample_x) as usize)
                                .read()
                        };
                    }
                }

                let averaged = average_of_block(&block);

                // SAFETY: as for the read above — the destination is row `y`, column `x`, and
                // `y < height ≤ self.height` and `x < width ≤ self.width` hold for the same
                // reason. Why writing into the surface being read is sound is argued in the
                // doc comment of this function.
                unsafe {
                    self.bits
                        .add(y as usize * stride + x as usize)
                        .write(averaged)
                };
            }
        }
    }
}

impl Drop for Supersample {
    fn drop(&mut self) {
        // SAFETY: `self.previous` is the bitmap this DC was born with, kept since `new`;
        // putting it back frees `self.bitmap` to be deleted. The answer is dropped — there is
        // nothing to put back if the DC is already gone, and this path carries no journal row
        // for the reason `theme::Brushes` gives for its own cleanup.
        unsafe { SelectObject(self.dc, self.previous) };

        // SAFETY: both came from the successful calls in `new`, were handed to nobody, and are
        // freed exactly once — the type is neither `Copy` nor `Clone`, its fields are private
        // and never reassigned, and `drop` runs once.
        let _ = unsafe { DeleteObject(self.bitmap.into()) };
        let _ = unsafe { DeleteDC(self.dc) };
    }
}

/// The four corner squares of a rounded rectangle, `side` pixels each — the tiles
/// [`paint_rounded`] smooths, and the pure half of its cost decision.
///
/// In the order top-left, top-right, bottom-left, bottom-right. Pure, so the geometry of the
/// four is one table a test can read without a window.
pub fn corner_tiles(area: &RECT, side: i32) -> [RECT; 4] {
    [
        RECT {
            left: area.left,
            top: area.top,
            right: area.left + side,
            bottom: area.top + side,
        },
        RECT {
            left: area.right - side,
            top: area.top,
            right: area.right,
            bottom: area.top + side,
        },
        RECT {
            left: area.left,
            top: area.bottom - side,
            right: area.left + side,
            bottom: area.bottom,
        },
        RECT {
            left: area.right - side,
            top: area.bottom - side,
            right: area.right,
            bottom: area.bottom,
        },
    ]
}

/// The rectangle a polyline through `points` covers when it is stroked with a pen `thickness`
/// pixels wide — the tile a check mark and a chevron are smoothed in.
///
/// Half the pen reaches past a point on each side, and one pixel more is added for the smoothed
/// edge itself: a stroke that ends exactly on the tile's edge would have nothing to fade into
/// there. Pure, and answered as an empty rectangle for no points at all — a figure with no
/// points has no tile, and the caller draws nothing either way.
pub fn stroke_bounds(points: &[(i32, i32)], thickness: i32) -> RECT {
    let Some((first_x, first_y)) = points.first().copied() else {
        return RECT::default();
    };

    let margin = thickness / 2 + 1;

    let mut bounds = RECT {
        left: first_x,
        top: first_y,
        right: first_x,
        bottom: first_y,
    };

    for (x, y) in points {
        bounds.left = bounds.left.min(*x);
        bounds.top = bounds.top.min(*y);
        bounds.right = bounds.right.max(*x);
        bounds.bottom = bounds.bottom.max(*y);
    }

    RECT {
        left: bounds.left - margin,
        top: bounds.top - margin,
        right: bounds.right + margin,
        bottom: bounds.bottom + margin,
    }
}

/// One rectangle held inside another — the tile of a mark, kept inside the glyph it belongs to.
fn clamped_to(area: &RECT, bounds: &RECT) -> RECT {
    RECT {
        left: area.left.max(bounds.left),
        top: area.top.max(bounds.top),
        right: area.right.min(bounds.right),
        bottom: area.bottom.min(bounds.bottom),
    }
}

/// One rounded rectangle with an outline and an interior, its four corners smoothed —
/// FR-92а, tasks T-11-5c and T-11-13, smoothed by task T-11-17.
///
/// `radius` is the corner radius in pixels of the window, already scaled by
/// [`scaled`]; `RoundRect` takes the *diameter* of the corner ellipse, and doubling it is
/// [`stroke_rounded`]'s business so that [`CORNER_RADIUS`] reads as the mock-ups describe it.
///
/// `outline` is the ink of a transient pen for the single-pixel frame; `fill` is a live
/// brush of the dialog's state. `dpi` is the DPI of the **window**, passed in rather than read
/// off the DC: since task T-11-17 the background of the dialog is built in a memory DC, whose
/// own answer to `GetDeviceCaps` is not the DPI the figure is being drawn for.
///
/// # What is smoothed, and what is not
///
/// The four corners only — the cost rule of this section. The straight edges have no staircase
/// to remove and keep the one `RoundRect` they always had; the corner squares are held out of
/// its clip while it runs, so each smoothed corner is blended into the true ground under the
/// figure and not into an aliased corner drawn a moment before. A rectangle too small to hold
/// four corner squares is drawn whole and aliased before any of that begins.
///
/// # What a refusal leaves behind — task T-11-24
///
/// ⚠ Holding the corners out of the clip is what makes a refusal *cost* something: the corner
/// the smoothing declines to paint is not a rougher corner, it is bare **ground** — a hole,
/// four of them when it is the surface that was refused rather than one tile. So every refusal
/// on the smoothing path — a refused surface (a tile past [`SUPERSAMPLE_MAX_SIDE`], or GDI out
/// of what a surface takes) and a refused tile alike — ends in [`stroke_corner_aliased`], which
/// fills that corner with the staircase `RoundRect` would have put there. The picture of a
/// refusal is the picture this function drew before task T-11-17, and never a hole (NFR-13).
///
/// Public for the reason [`check_cell`] and [`combo_chevron_points`] are: a test draws it into
/// a memory bitmap and reads the pixels back, which is how «шов между сглаженным углом и прямой
/// стороной» is held closed without a window and without starting the product.
pub fn paint_rounded(dc: HDC, area: &RECT, radius: i32, outline: COLORREF, fill: HBRUSH, dpi: i32) {
    // The frame of the mock-ups is [`BORDER_THICKNESS`] of their own pixels — one pixel
    // at 96 DPI, which is the pen this call has always made (п. 3 of task T-11-15), and two
    // at 125 % rather than the lonely hairline a bare `1` would have kept drawing.
    let thickness = scaled(BORDER_THICKNESS, dpi).max(1);

    // The corner square holds the whole curve and the frame that runs around it.
    let side = radius + thickness;

    // A figure too small to hold four of them is drawn whole and aliased: two overlapping
    // tiles would smooth one corner into another.
    let smoothed = radius > 0 && side * 2 <= (area.right - area.left).min(area.bottom - area.top);

    if !smoothed {
        stroke_rounded(dc, area, radius, thickness, outline, fill);
        return;
    }

    let tiles = corner_tiles(area, side);

    // The straight part, with the four corners held back. `SaveDC` is what puts the clip
    // region back afterwards — the caller's DC leaves as it came.
    //
    // SAFETY: `dc` is painted into for the length of the send this call is inside of; the call
    // takes no memory of ours.
    let saved = unsafe { SaveDC(dc) };

    // NFR-13: examined. Zero is «the state could not be saved», and the honest answer is to
    // draw the whole figure and smooth the corners over it — a corner blended into an aliased
    // corner instead of into the ground, which is still nearer the mock-up than no smoothing.
    if saved != 0 {
        for tile in &tiles {
            // NFR-13: the region type is answered and deliberately dropped — every outcome,
            // including an empty region, leaves a clip this drawing is correct under.
            //
            // SAFETY: `dc` is the live DC and the four numbers are plain values.
            let _ = unsafe { ExcludeClipRect(dc, tile.left, tile.top, tile.right, tile.bottom) };
        }
    }

    stroke_rounded(dc, area, radius, thickness, outline, fill);

    if saved != 0 {
        // SAFETY: `saved` is the state this function pushed a few lines above, and nothing
        // between the two calls pushed another.
        let _ = unsafe { RestoreDC(dc, saved) };
    }

    // The four corners, each drawn at `SUPERSAMPLE` times its size and averaged back down. One
    // surface for the four: the tiles are the same size, and a figure that made four would pay
    // four `CreateDIBSection`s for nothing.
    //
    // A refused surface is `None` here and is *not* a reason to leave: it refuses each of the
    // four tiles below, and the loop closes all four the same way it closes one — task T-11-24,
    // whose whole subject is that leaving early left the figure with four holes in it.
    let surface = Supersample::for_tile(side, side);

    for tile in &tiles {
        // The **whole** figure is named inside the tile and GDI clips it: a corner drawn by the
        // same `RoundRect` as the figure it belongs to cannot disagree with it by a pixel.
        let painted = surface.as_ref().is_some_and(|surface| {
            surface.render(dc, tile, thickness, |canvas| {
                stroke_rounded(
                    canvas.dc,
                    &canvas.outline(area),
                    canvas.length(radius),
                    canvas.length(thickness),
                    outline,
                    fill,
                );
            })
        });

        // NFR-13: the answer is examined and this is what it decides. The tile was held out of
        // the clip a few lines above, so an unpainted tile is a hole in the figure and not a
        // rougher corner; the staircase is what it falls back to.
        if !painted {
            stroke_corner_aliased(dc, area, tile, radius, thickness, outline, fill);
        }
    }
}

/// One corner of `area` drawn the way it would have been drawn before task T-11-17 — the
/// fallback of every refusal on the smoothing path, and the whole of task T-11-24.
///
/// The clip is narrowed to the one `tile` and the **whole** figure is named again, exactly as
/// [`Supersample::render`] names it on the enlarged surface: what reaches the DC is then the
/// pixels one plain `RoundRect` over the whole figure would have put inside that tile — a
/// staircase, but painted. Nothing else of the figure is touched, so the corners the smoothing
/// did manage keep their curve.
///
/// # Every refusal of the repair still paints
///
/// `SaveDC` answering zero means the clip could not be put back afterwards, so it is not
/// narrowed at all; `IntersectClipRect` answering `RGN_ERROR` means it was not narrowed either.
/// Both leave the caller's own clip in force, under which the very same call paints the figure
/// **whole** — the picture of before T-11-17 entire, at the price of aliasing whatever corners
/// were smoothed already. That is why there is one `stroke_rounded` here and not one per branch:
/// no path through this function ends without the corner painted, which is the one thing it
/// promises (NFR-13).
fn stroke_corner_aliased(
    dc: HDC,
    area: &RECT,
    tile: &RECT,
    radius: i32,
    thickness: i32,
    outline: COLORREF,
    fill: HBRUSH,
) {
    // SAFETY: `dc` is painted into for the length of the send this call is inside of; the call
    // takes no memory of ours.
    let saved = unsafe { SaveDC(dc) };

    // NFR-13: examined — zero is «the state could not be saved», and a clip narrowed with no way
    // to widen it again would truncate every drawing the caller has left to do.
    if saved != 0 {
        // SAFETY: `dc` is the live DC and the four numbers are plain values.
        //
        // NFR-13: the region type is answered and deliberately dropped, and here the reason is
        // that no answer asks for a different drawing — see the doc comment. `RGN_ERROR` leaves
        // the clip as it was, and the figure named next paints this corner along with the rest
        // of itself; an empty region means the tile is outside what the caller is painting at
        // all, where there is no corner to draw and no hole to leave.
        let _ = unsafe { IntersectClipRect(dc, tile.left, tile.top, tile.right, tile.bottom) };
    }

    stroke_rounded(dc, area, radius, thickness, outline, fill);

    if saved != 0 {
        // SAFETY: `saved` is the state this function pushed a few lines above, and nothing
        // between the two calls pushed another.
        let _ = unsafe { RestoreDC(dc, saved) };
    }
}

/// The aliased core of [`paint_rounded`]: one `RoundRect` with a transient pen of `outline`
/// and the interior of `fill` — the body this function had before task T-11-17, with the pen
/// thickness handed in.
///
/// `RoundRect` draws both at once — frame with the selected pen, interior with the selected
/// brush — which is why this looks like [`stroke_ellipse`] with a different figure. The pen is
/// owned for exactly this call, as there; a refused `CreatePen` skips the figure (NFR-13:
/// examined — better no panel for one paint than a panel framed in whatever pen the DC happens
/// to hold).
///
/// Called twice per smoothed figure with two different scales: once on the window, once on the
/// enlarged surface of [`Supersample`], where every length has been multiplied by
/// [`SUPERSAMPLE`]. That is the whole reason `thickness` is a parameter and not a `scaled`
/// call inside.
fn stroke_rounded(
    dc: HDC,
    area: &RECT,
    radius: i32,
    thickness: i32,
    outline: COLORREF,
    fill: HBRUSH,
) {
    // SAFETY: takes plain values, reads no memory of ours, answers a handle owned by this
    // frame until the `DeleteObject` below.
    let pen = unsafe { CreatePen(PS_SOLID, thickness, outline) };

    if pen.is_invalid() {
        return;
    }

    // SAFETY: `dc` is painted into for the length of the send this call is inside of;
    // `pen` is the live pen just made. The previous pen is restored below.
    let previous_pen = unsafe { SelectObject(dc, pen.into()) };

    // SAFETY: as above; `fill` is a live brush of the dialog's state.
    let previous_brush = unsafe { SelectObject(dc, fill.into()) };

    // SAFETY: plain coordinates into a live DC. The answer is dropped for the NFR-13
    // reason the caller states for all its paint calls.
    let _ = unsafe {
        RoundRect(
            dc,
            area.left,
            area.top,
            area.right,
            area.bottom,
            radius * 2,
            radius * 2,
        )
    };

    // SAFETY: both handles were in the DC a moment ago; putting them back ends this
    // function's use of the DC.
    unsafe { SelectObject(dc, previous_brush) };
    unsafe { SelectObject(dc, previous_pen) };

    // SAFETY: `pen` was created above, deselected the line before, and freed exactly
    // once, here. The `BOOL` is dropped — see `draw_check_mark`.
    let _ = unsafe { DeleteObject(pen.into()) };
}

// =========================================================================================
// The closed part of the four combo boxes — FR-92а, task T-11-14
// =========================================================================================
//
// ⚠ What this section is and is not. `CBS_OWNERDRAWFIXED` (task T-11-6) hands the program the
// *items* of a combo box — the rows of the dropped-down list and the text of the closed face.
// It hands over neither the frame of the closed part nor the button with the arrow beside it:
// those the control paints for itself, out of the system's own colours, and the result is the
// pale square that broke both palettes and that the user named first. The only way to that
// square is to be the one who answers the control's `WM_PAINT`, and the documented way to
// answer another window's message is a subclass.
//
// ⚠ This is **not** the forbidden trick of FR-92а. The ban of that requirement is on the
// undocumented ordinals of `uxtheme.dll` and on the names of the dark system themes — see
// `theme-own-draw-decision`. `SetWindowSubclass` is a documented, exported, headline function
// of `comctl32.dll` with its own MSDN page, and painting one's own control is what
// `WM_PAINT` is for. Nothing here reads a private ordinal, a theme name or a theme handle.

/// Which of the two documented ways of putting a procedure in front of a control's own was
/// chosen, and why — the reasoning behind [`subclass_combo_boxes`], written where the code is.
///
/// `SetWindowLongPtrW(GWLP_WNDPROC)` is already used in this file, by
/// [`subclass_hotkey_field`], and would have worked. `SetWindowSubclass` was chosen for the
/// four combo boxes for three reasons, all of which the hotkey field does not have:
///
/// 1. **Four windows, one procedure.** The `GWLP_WNDPROC` road has to store the displaced
///    procedure of each window somewhere; with one window that is one thread-local `Cell`
///    ([`HOTKEY_FIELD_PROC`]), with four it is a table keyed by window handle. The subclass
///    API keeps that association itself, per window, and hands it back through
///    [`DefSubclassProc`].
/// 2. **The removal is documented and exact.** `RemoveWindowSubclass` takes the window, the
///    procedure and the identifier and unhooks *that* subclass wherever it sits in the chain.
///    Putting a `GWLP_WNDPROC` back is only correct while nothing else has subclassed the
///    control after us — and `comctl32` does subclass its own controls.
/// 3. **The pair is visible.** One call installs, one call removes, and the two are named for
///    each other, which is what criterion 9 of the task asks to be shown.
///
/// The identifier is a constant of this file and not a handle or an address: the pair
/// (procedure, identifier) is what names our subclass, and one number for all four windows is
/// enough because the key is per window.
const COMBO_SUBCLASS_ID: usize = 1;

/// Puts [`combo_box_proc`] in front of each of the four combo boxes — FR-92а, task T-11-14.
///
/// **The pair.** Called exactly once, from [`fill_dialog`] on `WM_INITDIALOG`; the other half
/// is [`unsubclass_combo_boxes`], called exactly once, from the `WM_DESTROY` branch of
/// [`dialog_proc`], while the children are still alive. Both walk the same [`COMBO_BOXES`]
/// list with the same procedure and the same [`COMBO_SUBCLASS_ID`], so every install has its
/// removal by construction rather than by discipline. [`combo_box_proc`] additionally removes
/// itself on `WM_NCDESTROY`, which is the belt to that pair's braces: a control destroyed by
/// any road other than the dialog's own `WM_DESTROY` still lets go of the procedure. The two
/// removals cannot double-free anything — `RemoveWindowSubclass` on a window that no longer
/// carries the subclass answers `FALSE` and does nothing.
///
/// A refused install is survived (NFR-13): that combo then paints itself the way it did before
/// this task — the system's own frame and arrow — which is visibly wrong rather than silently
/// broken, and the dialog still opens. Why the refusal is not journaled is written at the call.
fn subclass_combo_boxes(hwnd: HWND) {
    for control in COMBO_BOXES {
        // SAFETY: `hwnd` is the live dialog; the crate turns a missing control into an error.
        let Ok(combo) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
            crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
            continue;
        };

        // SAFETY: `combo` is a live control of this dialog, created by the dialog manager on
        // this thread, and `combo_box_proc` is a function of exactly the signature
        // `SUBCLASSPROC` names. The reference data is zero — this subclass keeps no state of
        // its own; everything it needs it reads off the dialog through `with_state`.
        // ⚠ NFR-13: the `BOOL` is examined right here, in words, and deliberately dropped —
        // and the reason it is not journaled is not indifference. The operation vocabulary of
        // `diag` is closed (reviews\T-11-1.md) and has no row for this call; adding one would
        // mean editing `src\diag.rs`, which is outside the bounds of this task, and a test of
        // `tests\diag.rs` guards that closure. The consequence of a refusal is visible rather
        // than silent — that one combo box then paints itself the way it did before this task,
        // with the system's own frame and arrow — and the dialog opens and works either way.
        // No `debug_assert` either: a machine that refused is a legal state, not a violation
        // of ownership, and crashing a debug build over a cosmetic degradation is what NFR-13
        // exists to prevent.
        let _ = unsafe { SetWindowSubclass(combo, Some(combo_box_proc), COMBO_SUBCLASS_ID, 0) };
    }
}

/// Takes [`combo_box_proc`] back off the four combo boxes — the far half of the pair
/// [`subclass_combo_boxes`] describes.
///
/// The refusal is deliberately **not** journaled: this runs on `WM_DESTROY`, where a `FALSE`
/// means the subclass was already gone (never installed, or removed by the `WM_NCDESTROY` arm
/// of the procedure), and neither is a fault worth a journal row.
fn unsubclass_combo_boxes(hwnd: HWND) {
    for control in COMBO_BOXES {
        // SAFETY: `hwnd` is the live dialog — `WM_DESTROY` reaches it before its children are
        // destroyed — and the crate turns a missing control into an error.
        let Ok(combo) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
            continue;
        };

        // SAFETY: `combo` is the live control the subclass was installed on, and the
        // procedure and identifier are the very pair `SetWindowSubclass` was given.
        let _ = unsafe { RemoveWindowSubclass(combo, Some(combo_box_proc), COMBO_SUBCLASS_ID) };
    }
}

/// The procedure that stands in front of each combo box while this dialog is up — FR-92а,
/// task T-11-14.
///
/// Three messages are its own; everything else goes straight on to [`DefSubclassProc`], so a
/// combo box behaves in every other way exactly as it did before the subclass — the list still
/// drops down, the keyboard still chooses, `CB_*` still works, and the dropped-down list is
/// still coloured by `WM_CTLCOLORLISTBOX` and drawn item by item by `WM_DRAWITEM`.
///
/// ⚠ **`WM_PAINT` does not reach the original procedure.** The arm ends in `return`, not in a
/// fall-through: the control's own painting — the frame and the pale system button with the
/// arrow — never runs. The one road back to it is a refused `BeginPaint`, and then nothing of
/// ours could have been drawn either; letting the message through is what keeps the window
/// validated instead of asking for a paint for ever (NFR-13).
///
/// # Safety
///
/// Called by the window manager with the arguments of a window message, on one of the four
/// controls [`subclass_combo_boxes`] installed it on.
unsafe extern "system" fn combo_box_proc(
    combo: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    _reference_data: usize,
) -> LRESULT {
    match message {
        // The whole reason this procedure exists — see the ⚠ above.
        //
        // SAFETY: `combo` is the control this procedure is installed on, and the call is
        // inside the window's own `WM_PAINT`, which is where `BeginPaint` may be used.
        WM_PAINT => {
            if unsafe { paint_combo_closed_part(combo) } {
                return LRESULT(0);
            }
        }

        // «Erased» without erasing anything: the closed face is repainted whole by the arm
        // above, and an erase before it is exactly the two-step repaint that flickers.
        WM_ERASEBKGND => return LRESULT(1),

        // The belt to the braces of the pair — see `subclass_combo_boxes`. Forwarded on
        // afterwards: `WM_NCDESTROY` must reach every procedure of the chain.
        //
        // SAFETY: `combo` is that control, and the procedure and identifier are the pair the
        // subclass was installed with.
        WM_NCDESTROY => {
            let _ = unsafe { RemoveWindowSubclass(combo, Some(combo_box_proc), COMBO_SUBCLASS_ID) };
        }

        _ => {}
    }

    // SAFETY: the four arguments are the ones the window manager passed in, forwarded
    // unchanged; this is what `DefSubclassProc` exists for.
    unsafe { DefSubclassProc(combo, message, wparam, lparam) }
}

/// Answers one `WM_PAINT` of a combo box; `false` when `BeginPaint` refused and the message
/// has to go on to the control's own procedure — see [`combo_box_proc`].
///
/// # Safety
///
/// Called from [`combo_box_proc`] alone, inside the `WM_PAINT` of the window it names.
unsafe fn paint_combo_closed_part(combo: HWND) -> bool {
    let mut paint = PAINTSTRUCT::default();

    // SAFETY: `combo` is the live control inside its own `WM_PAINT`, and `paint` is a live
    // local of this frame that the call fills in.
    let dc = unsafe { BeginPaint(combo, &mut paint) };

    if dc.is_invalid() {
        // NFR-13: examined. Nothing of ours can be drawn without a DC, and the window still
        // has to be validated — which the control's own procedure will do.
        return false;
    }

    // SAFETY: `dc` is the DC `BeginPaint` has just answered, owned until `EndPaint` below.
    unsafe { draw_combo_closed_part(combo, dc) };

    // SAFETY: the same window and the very `PAINTSTRUCT` `BeginPaint` filled in. The `BOOL`
    // is dropped for the reason the paint calls of this file drop theirs.
    let _ = unsafe { EndPaint(combo, &paint) };

    true
}

/// Draws the closed part of one combo box the way the mock-ups draw it — FR-92а, task T-11-14.
///
/// # What is drawn
///
/// 1. the client area as a rounded rectangle of [`CORNER_RADIUS`]: `field_bg` under a
///    single-pixel `field_border` frame — the same figure the five input fields and the two
///    lists wear since task T-11-13, so a combo box is a field to the eye like the rest;
/// 2. the chevron at the right edge, [`combo_chevron_points`] wide and stroked in
///    `text_muted` — the flat «⌄» of the mock-ups, in place of the system button;
/// 3. the text of the chosen item, read from the control by identifier, behind
///    [`FIELD_TEXT_INSET_DLU`] dialog units of air and clipped short of the chevron; `text`
///    normally, `text_muted`
///    when the mode has disabled this combo ([`combo_closed_color_roles`]);
/// 4. the dotted `DrawFocusRect` inside the frame while the control holds the focus and the
///    keyboard cues are not hidden — the same cue, the same inset of two pixels, as the
///    owner-drawn push buttons of task T-11-5a.
///
/// # The font
///
/// Unlike a `WM_DRAWITEM`, a `WM_PAINT` hands over a DC with the *stock* font in it — the
/// dialog manager selects a control's font only into the DC it makes for an owner-draw
/// message. So the control's own font is asked for (`WM_GETFONT`), selected for the length of
/// the drawing and put back. No face name and no point size appear in this file; the font is
/// the one the manager already created at the window's DPI.
///
/// Nothing is drawn at all when the state is unreachable or `theme::Brushes::new` was refused
/// at initialisation — the reason [`on_draw_item`] gives for its own zeroes.
///
/// # Safety
///
/// Called from [`paint_combo_closed_part`] alone, with the DC of the paint it is inside of.
unsafe fn draw_combo_closed_part(combo: HWND, dc: HDC) {
    // The dialog is the parent of its own control; everything below is read through it, by
    // identifier, exactly as every other drawing of this file reads what it draws.
    //
    // SAFETY: `combo` is the live control; a window with no parent answers an error.
    let Ok(dialog) = (unsafe { GetParent(combo) }) else {
        return;
    };

    // SAFETY: reading a window's identifier reads a field of that window and no memory of
    // ours.
    let control = unsafe { GetDlgCtrlID(combo) };

    // The same gate the owner-draw handlers keep (SEC-05), even though this procedure can only
    // be reached on a window it was installed on: one list of four, checked before any work.
    if !COMBO_BOXES.contains(&control) {
        return;
    }

    let mut area = RECT::default();

    // SAFETY: `combo` is the live control and `area` is a live local the call fills.
    if unsafe { GetClientRect(combo, &mut area) }.is_err() {
        return;
    }

    // SAFETY: `combo` is the live control; the call reads a window style and answers a flag.
    let disabled = !unsafe { IsWindowEnabled(combo) }.as_bool();

    // The focus cue on the same terms as the buttons of task T-11-5a: shown while the control
    // holds the focus, hidden while the window manager says the keyboard cues are hidden —
    // which is what `ODS_NOFOCUSRECT` carries into an owner-draw message and what
    // `WM_QUERYUISTATE` answers outside one.
    //
    // SAFETY: the call reads the focus of this thread and touches no memory of ours.
    let has_focus = unsafe { GetFocus() } == combo;
    let cues_hidden =
        send_to(dialog, control, WM_QUERYUISTATE, 0, 0) & (UISF_HIDEFOCUS as isize) != 0;
    let focused = has_focus && !cues_hidden;

    let roles = combo_closed_color_roles(disabled);

    // The colour choice, split from the painting exactly as everywhere in this file: the
    // borrow of the state ends before the DC is touched, and what leaves the closure is plain
    // values — a brush the state keeps alive until the dialog ends, and three inks.
    //
    // SAFETY: `dialog` is the parent of one of this dialog's own controls, which is the
    // window `show_dialog` created — the contract of `with_state`.
    let choice = unsafe {
        with_state(dialog, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13, T-11-4).
            let brushes = state.brushes.as_ref()?;
            let palette = state.palette;

            // The ground the corners the rounding cuts away are left standing on — the very
            // rule `on_ctl_color` answers `WM_CTLCOLORBTN` with: the panel brush for a
            // control lying on one of the eight group panels, the window brush elsewhere.
            // All four combo boxes do lie on a panel, but the rule is asked and not assumed.
            let ground = if state.panel_children.contains(&control) {
                brushes.panel_bg()
            } else {
                brushes.window_bg()
            };

            Some((
                ground,
                combo_fill_brush(roles.fill, brushes),
                match roles.border {
                    ComboBorderRole::FieldBorder => palette.field_border,
                },
                combo_text_ink(roles.text, palette),
                match roles.chevron {
                    ComboChevronRole::TextMuted => palette.text_muted,
                },
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((ground, fill, border, ink, chevron_ink, face))) = choice else {
        return;
    };

    let dpi = dc_dpi(dc);

    // 1. The ground, then the field on top of it. `WM_ERASEBKGND` deliberately erases
    // nothing (see `combo_box_proc`), so this is the one erase of the closed part — and it
    // is what the four corners the rounding cuts away are filled with. Without it those
    // corners would hold whatever the parent last painted there, which is right only for as
    // long as the dialog carries no `WS_CLIPCHILDREN`.
    //
    // SAFETY: `dc` is the DC of this paint and `area` is a live local of this frame;
    // `ground` is a live brush of the dialog's state.
    unsafe { FillRect(dc, &area, ground) };

    paint_rounded(dc, &area, scaled(CORNER_RADIUS, dpi), border, fill, dpi);

    // 2. The chevron, in place of the system button.
    let chevron = combo_chevron_points(&area, dpi);
    draw_combo_chevron(dc, chevron, chevron_ink, dpi);

    // 3. The chosen value, from the control by identifier — never from anywhere else.
    let mut value = combo_selected_text(dialog, control);

    if !value.is_empty() {
        // SAFETY: `dc` is the DC of this paint; both calls write an attribute of the DC and
        // touch no memory of this process.
        unsafe { SetBkMode(dc, TRANSPARENT) };
        // SAFETY: as above.
        unsafe { SetTextColor(dc, ink) };

        // The face for the length of the drawing — see the doc comment. Since task T-11-17 it
        // is the dialog's own grey-antialiased face; the control's own font, which the manager
        // created and renders with ClearType, is the fallback for a window whose faces could
        // not be made (NFR-13).
        let font = match face {
            Some(face) => face,
            None => HFONT(send_to(dialog, control, WM_GETFONT, 0, 0) as *mut std::ffi::c_void),
        };

        // NFR-13: examined. A control with no font of its own answers zero, and the stock
        // font of the DC is then the degraded-but-alive answer.
        let previous_font = if font.is_invalid() {
            None
        } else {
            // SAFETY: `dc` is the DC of this paint and `font` is either a live face the
            // dialog's state owns for longer than this call or the live font of the control,
            // owned by the manager; the previous handle is put back below.
            Some(unsafe { SelectObject(dc, font.into()) })
        };

        // Three dialog units from the left edge — the `($c.x + 3)` of the `'combo'` arm of the
        // generator (task T-11-16). The dialog is the window the units belong to; a refused
        // `MapDialogRect` keeps the bare unit count, as everywhere else (NFR-13).
        let inset = dialog_units(dialog, FIELD_TEXT_INSET_DLU, 0)
            .map(|(horizontal, _)| horizontal)
            .unwrap_or(FIELD_TEXT_INSET_DLU);

        let mut text_rect = RECT {
            left: area.left + inset,
            top: area.top,
            right: (chevron[0].0 - scaled(COMBO_CHEVRON_TEXT_GAP, dpi)).max(area.left),
            bottom: area.bottom,
        };

        // SAFETY: `value` and `text_rect` are live locals of this frame; the format has no
        // `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the text and writes only
        // pixels of the DC.
        unsafe {
            DrawTextW(
                dc,
                &mut value,
                &mut text_rect,
                DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            )
        };

        if let Some(previous) = previous_font {
            // SAFETY: `previous` is the font that was in the DC a moment ago; putting it back
            // ends this function's use of it.
            unsafe { SelectObject(dc, previous) };
        }
    }

    // 4. The focus cue, on the terms of task T-11-5a.
    if focused {
        let focus_rect = RECT {
            left: area.left + FOCUS_CUE_INSET,
            top: area.top + FOCUS_CUE_INSET,
            right: area.right - FOCUS_CUE_INSET,
            bottom: area.bottom - FOCUS_CUE_INSET,
        };

        // NFR-13: the `BOOL` is examined and deliberately dropped, as everywhere in this file.
        //
        // SAFETY: `dc` is the DC of this paint and `focus_rect` is a live local of this frame;
        // the call keeps no pointer.
        let _ = unsafe { DrawFocusRect(dc, &focus_rect) };
    }
}

/// The text of the item a combo box currently shows, as UTF-16 without the terminator — read
/// from the dialog's own control by identifier, the one road this file reads a control's
/// content by.
///
/// Empty for a combo with nothing chosen (`CB_ERR`), for a refused length, and for a refused
/// copy — the closed face then shows its ground and its chevron and no word, which is the
/// degraded-but-alive answer of NFR-13.
fn combo_selected_text(dialog: HWND, control: i32) -> Vec<u16> {
    let Ok(index) = usize::try_from(send_to(dialog, control, CB_GETCURSEL, 0, 0)) else {
        return Vec::new();
    };

    let Ok(length) = usize::try_from(send_to(dialog, control, CB_GETLBTEXTLEN, index, 0)) else {
        return Vec::new();
    };

    // One for the terminator the control writes and never counts.
    let mut buffer = vec![0u16; length + 1];

    // SAFETY: `buffer` is owned by this frame and holds the length the combo has just
    // reported plus the terminator, which is exactly what `CB_GETLBTEXT` writes; the pointer
    // is not retained by the call.
    let copied = send_to(
        dialog,
        control,
        CB_GETLBTEXT,
        index,
        buffer.as_mut_ptr() as isize,
    );

    let copied = usize::try_from(copied).unwrap_or(0).min(length);

    buffer.truncate(copied);
    buffer
}

/// Half-width of the chevron of a closed combo box, in mock-up pixels — the `±4` by `x` of
/// `(PtF ($cx-4) …), (PtF $cx …), (PtF ($cx+4) …)` in the `'combo'` arm, п. 8 of T-11-16.
///
/// An arm and not the whole span since this task: the generator states the figure as three
/// points around a centre, and building it out of a span and a drop of half the span put the
/// apex 0,57 px below where the picture has it — the one place the old arithmetic fell outside
/// the half-pixel the task allows.
pub const COMBO_CHEVRON_ARM_X: i32 = 4;

/// Half-height of the chevron, in mock-up pixels — the `∓2` by `y` of the same three points:
/// the two arms stand this far above the middle of the field and the apex this far below it.
pub const COMBO_CHEVRON_ARM_Y: i32 = 2;

/// Distance from the right edge of the closed part to the **centre** of the chevron, in
/// mock-up pixels — the `$cx = $px + $pw - 14` of the `'combo'` arm, п. 8 of T-11-16. It used
/// to be a 16 read off `ui-03-fog.png` with the eye.
pub const COMBO_CHEVRON_INSET_X: i32 = 14;

/// Thickness of the chevron's stroke, in **tenths** of a mock-up pixel — the `[single]1.5` the
/// generator makes its pen with. [`scaled_tenths`] turns it into the whole pixels GDI draws
/// with.
pub const COMBO_CHEVRON_PEN_TENTHS: i32 = 15;

/// Air between the text of the closed part and the chevron, in mock-up pixels.
pub const COMBO_CHEVRON_TEXT_GAP: i32 = 4;

/// The three points of the chevron of a closed combo box — FR-92а, task T-11-14, the pure
/// half of its drawing, closed by a table test.
///
/// A chevron is one polyline through three points and therefore two strokes, which is exactly
/// what the mock-ups show: two arms meeting at an apex below them. Since task T-11-16 the
/// three points are built the way the generator builds them — one centre and two half-lengths,
/// `±`[`COMBO_CHEVRON_ARM_X`] by `x` and `∓`[`COMBO_CHEVRON_ARM_Y`] by `y` — instead of a span
/// halved twice, which rounded the apex a whole pixel low at 96 DPI. The centre sits on the
/// vertical middle of the field it is given, so the figure follows the height of the control
/// instead of a number written here.
///
/// Every length is a mock-up length put through [`scaled`], so the chevron grows with the DPI
/// of the window like the radii and insets of task T-11-13.
pub fn combo_chevron_points(area: &RECT, dpi: i32) -> [(i32, i32); 3] {
    let arm_x = scaled(COMBO_CHEVRON_ARM_X, dpi).max(1);
    let arm_y = scaled(COMBO_CHEVRON_ARM_Y, dpi).max(1);

    let centre_x = area.right - scaled(COMBO_CHEVRON_INSET_X, dpi);
    let centre_y = (area.top + area.bottom) / 2;

    [
        (centre_x - arm_x, centre_y - arm_y),
        (centre_x, centre_y + arm_y),
        (centre_x + arm_x, centre_y - arm_y),
    ]
}

/// Strokes the chevron with a transient pen of `ink`, smoothed — the drawing half of
/// [`combo_chevron_points`], smoothed whole by task T-11-17.
///
/// Two shallow diagonals are exactly the figure GDI draws worst: without smoothing the arms of
/// the mock-up's chevron come out notched, which is the defect `zoom-pairs.png` shows on the
/// `combo-source` row. The figure is a dozen pixels across, so it is enlarged whole — the
/// cheap half of the cost rule; a refused surface or blit falls through to the aliased stroke
/// of every task before this one (NFR-13: the field is still drawn, still opens on a click).
fn draw_combo_chevron(dc: HDC, points: [(i32, i32); 3], ink: COLORREF, dpi: i32) {
    let thickness = scaled_tenths(COMBO_CHEVRON_PEN_TENTHS, dpi);
    let tile = stroke_bounds(&points, thickness);

    let smoothed = Supersample::for_tile(tile.right - tile.left, tile.bottom - tile.top)
        .is_some_and(|surface| {
            surface.render(dc, &tile, thickness, |canvas| {
                let enlarged = points.map(|(x, y)| canvas.point(x, y));

                stroke_polyline(canvas.dc, &enlarged, ink, canvas.length(thickness));
            })
        });

    if !smoothed {
        stroke_polyline(dc, &points, ink, thickness);
    }
}

/// The five input fields of the dialog — the ones [`set_field_margins`] gives the text inset
/// of the mock-ups to.
///
/// All five are `EDITTEXT` of the template and all five are drawn by the dialog's own
/// background as a rounded field (task T-11-13), so all five put their text where this file
/// says and not where a native edit would.
const TEXT_FIELDS: [i32; 5] = [
    IDC_HOTKEY,
    IDC_DELAY,
    IDC_CLIPBOARD_TIMEOUT,
    IDC_CLIPBOARD_RESTORE,
    IDC_EXCLUSION_NAME,
];

/// Puts [`FIELD_TEXT_INSET_DLU`] dialog units of air on both sides of the text of every input
/// field — п. 2 of task T-11-16, the first of the three places that inset lives.
///
/// `EM_SETMARGINS` is the documented message for exactly this and the only one there is: an
/// edit control positions its own text, and no `WM_CTLCOLOREDIT` or owner-draw of ours can
/// move it (an `EDITTEXT` has no owner-draw at all). `EC_LEFTMARGIN | EC_RIGHTMARGIN` in the
/// `wparam` names both margins; the `lparam` carries the left one in the low word and the
/// right one in the high word, which is what the shift by 16 is.
///
/// Both margins get the same number: the mock-ups inset the text of a field from both edges,
/// and a right margin also keeps the caret of a full field off the rounded frame.
///
/// The unit is the dialog's own: [`dialog_units`] maps three units through `MapDialogRect`,
/// whose base units come from the dialog font the manager created at the window's DPI — so a
/// field on a 150 % monitor gets the inset that monitor's pixels ask for, with no arithmetic
/// of ours in the middle. A refused `MapDialogRect` keeps the bare unit count (NFR-13): a
/// field looking slightly tight and nothing worse.
fn set_field_margins(hwnd: HWND) {
    let inset = dialog_units(hwnd, FIELD_TEXT_INSET_DLU, 0)
        .map(|(horizontal, _)| horizontal)
        .unwrap_or(FIELD_TEXT_INSET_DLU);

    // The two margins in one `lparam`, low word left and high word right — the documented
    // shape of the message. Both fit in a word: the inset is a dozen pixels even at 400 %.
    let margins =
        isize::try_from((inset.max(0) as u32) | ((inset.max(0) as u32) << 16)).unwrap_or(0);

    for control in TEXT_FIELDS {
        send_to(
            hwnd,
            control,
            EM_SETMARGINS,
            usize::try_from(EC_LEFTMARGIN | EC_RIGHTMARGIN).unwrap_or(0),
            margins,
        );
    }
}

/// The six controls that draw their **own** text and are therefore the only ones a face has to
/// be handed to — the five input fields of [`TEXT_FIELDS`] and the layout list.
///
/// Everything else in this window is drawn by this file, which selects the face into the DC of
/// the message; these six never see that DC. See the ⚠ of the antialiasing section for why the
/// two kinds cannot be treated the same way.
///
/// Public for the reason [`FRAMED_FIELDS`] is: `tests\settings.rs` reads the classes of the
/// template and asserts that this list is **exactly** the controls of the window that draw
/// their own text — every `EDIT` of the template and the one `SysListView32` — so a control
/// added to the template later cannot quietly stay behind on the manager's ClearType.
pub const CONTROLS_THAT_DRAW_THEIR_OWN_TEXT: [i32; TEXT_FIELDS.len() + 1] = [
    IDC_HOTKEY,
    IDC_DELAY,
    IDC_CLIPBOARD_TIMEOUT,
    IDC_CLIPBOARD_RESTORE,
    IDC_EXCLUSION_NAME,
    IDC_CYCLE_LIST,
];

/// Hands [`DialogFonts::text`] to the six controls of [`CONTROLS_THAT_DRAW_THEIR_OWN_TEXT`] —
/// FR-92а, task T-11-20, the last text of the window still set on the manager's ClearType.
///
/// # Why this is not a change of metrics — the measurement, not the argument
///
/// The face is the dialog's own `LOGFONTW` with `lfQuality` moved to `ANTIALIASED_QUALITY` and
/// **nothing else touched** ([`antialiased_logfont`]): same type face, same character height,
/// same weight, same character set, same escapement. `GetTextMetricsW` answers the same height,
/// ascent, descent, internal and external leading, average and maximum character width and
/// weight for it as for the manager's own face, and real strings measure the same number of
/// pixels — so no field's text moves by a pixel and no row of the list changes height. Both
/// halves are tests in `tests\settings.rs`; the change was measured before it was made.
///
/// # Why `send_to` and not a bare send — FR-72
///
/// `send_to` is `SendDlgItemMessageW`, «the one way this module sends anything to its own
/// controls», and a bare `SendMessage` is banned program-wide by the implication of FR-72: to a
/// window whose thread is not pumping it blocks the caller for ever. `tests\guard.rs` sweeps the
/// whole of `src\` for the name and this file adds none.
///
/// # The `lparam`
///
/// `TRUE` — redraw. The controls are handed the face on `WM_INITDIALOG`, before the window is
/// on the screen, so the repaint it asks for costs nothing; asking for it is what keeps the
/// call correct if it is ever made again while the window is visible.
///
/// `None` — the faces could not be made — hands over nothing and leaves all six on the
/// manager's own face, which is what they were on before this task (NFR-13).
fn hand_our_face_to_the_controls_that_draw_their_own_text(hwnd: HWND, face: Option<HFONT>) {
    let Some(face) = face else {
        return;
    };

    for control in CONTROLS_THAT_DRAW_THEIR_OWN_TEXT {
        // `WM_SETFONT` answers nothing — it is documented to return zero — so there is no
        // result to examine here beyond the one `send_to` describes.
        send_to(hwnd, control, WM_SETFONT, face.0 as usize, 1);
    }
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
    // FR-92а, task T-11-14. The other half of this pair — `unsubclass_combo_boxes` — is the
    // `WM_DESTROY` branch of `dialog_proc`, and nothing else in the file installs or removes
    // this subclass. See `subclass_combo_boxes` for why the pairing is written that way.
    subclass_combo_boxes(hwnd);
    // п. 2 of task T-11-15: the text of the five input fields, off the frame by the inset of
    // the mock-ups. Once, here — a margin is a property of the control, not of a paint.
    set_field_margins(hwnd);

    // FR-92а, task T-11-20: the six controls that draw their own text get the face this
    // window draws all its other text in, so the whole window is on one smoothing. Once,
    // here — a face is a property of the control, exactly as the margin above is — and
    // **before anything is put into any of them**, so no control is ever holding text it was
    // given in one face and is about to redraw in another.
    hand_our_face_to_the_controls_that_draw_their_own_text(
        hwnd,
        state.fonts.as_ref().map(DialogFonts::text),
    );

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

    // FR-92а, task T-11-7: the list carries palette state of its own — the three colours
    // and the state image list of the ticks — set here for the first showing and again by
    // `apply_now` on every palette change. After `prepare_cycle_list`: the extended style
    // must exist before the system pair it creates can be replaced.
    paint_cycle_list(hwnd, state.palette);
    install_check_images(hwnd, state.palette);

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

/// The self-switching `BS_OWNERDRAW` displaced, returned by hand — FR-92а, task T-11-5b.
///
/// `BS_OWNERDRAW` is a button *type*: it displaced `BS_AUTOCHECKBOX` and
/// `BS_AUTORADIOBUTTON` in the template, and the flip on a click and on the arrow keys
/// died with them. This function is that behaviour, spelled out again:
///
/// - **a check box flips on a click**: [`set_check`] of the opposite of its own stored
///   state — the write lands in the dialog's [`GlyphChecks`] store and the repaint rides
///   in `set_check` itself (task T-11-5b-2). `BN_DBLCLK` counts as
///   the click it is: an owner-drawn button folds the second press of a double click into
///   `BN_DBLCLK` instead of a second `BN_CLICKED`, and the automatic type answered both
///   presses with a toggle — dropping one would make a fast double click flip the box and
///   leave it flipped;
/// - **a radio button checks itself and unchecks its group**: [`check_radio`] over the
///   very ranges the rest of the dialog already uses — `IDC_MODE_PAIR..IDC_MODE_CYCLE`
///   and `IDC_METHOD_AUTO..IDC_METHOD_SELECTION`, the template's `WS_GROUP` runs — which
///   quenches the run in the store, arms the chosen one and repaints the whole range: the
///   neighbour that just lost the dot must lose it on the same message;
/// - **`BN_SETFOCUS` is the arrow keys**: the dialog manager answers an arrow key inside
///   a `WS_GROUP` run by moving the focus, and an automatic radio button then checked
///   itself on arrival — `BS_NOTIFY` in the template is what makes the arrival audible
///   here. Answered **only while an arrow key is actually down**: focus also arrives by
///   Tab, and Tab enters the group at its first tab stop, not at the checked element —
///   nothing juggles `WS_TABSTOP` for a non-automatic type the way the automatic types
///   did for themselves — so checking on every arrival would silently move the user's
///   choice to the first radio of the group on a mere walk through the dialog.
///
/// The check state lives in the dialog's own [`GlyphChecks`] store — task T-11-5b-2. An
/// owner-drawn button keeps none of its own: the button-message pair that serves the
/// automatic types ignores the write and answers «снят» for a `BS_OWNERDRAW` one, which
/// the final sweep of the live acceptance saw as eight glyphs drawn unchecked. So
/// [`set_check`], [`is_checked`] and [`check_radio`] — their signatures untouched — write
/// and read the store, and everything that reads the dialog keeps reading the one truth.
fn restore_self_switching(hwnd: HWND, control: i32, notification: u16) {
    let notification = u32::from(notification);
    let clicked = notification == BN_CLICKED || notification == BN_DBLCLK;

    // A radio answers a click, and it answers the focus arrival an arrow key caused — the
    // two ways the automatic type checked itself. `key_is_down` is queue-synchronised, so
    // the answer is the state as of the keystroke whose focus move is being handled.
    let radio_checks = clicked || (notification == BN_SETFOCUS && arrow_key_is_down());

    match control {
        IDC_AUTOSTART | IDC_SELECTION_ENABLED | IDC_LOG_ENABLED if clicked => {
            set_check(hwnd, control, !is_checked(hwnd, control));
        }

        IDC_MODE_PAIR | IDC_MODE_CYCLE if radio_checks => {
            check_radio(hwnd, IDC_MODE_PAIR, IDC_MODE_CYCLE, control);
        }

        IDC_METHOD_AUTO | IDC_METHOD_BACKSPACE | IDC_METHOD_SELECTION if radio_checks => {
            check_radio(hwnd, IDC_METHOD_AUTO, IDC_METHOD_SELECTION, control);
        }

        _ => {}
    }
}

/// Whether one of the four arrow keys is down right now — the keys the dialog manager
/// walks a `WS_GROUP` run with. The same queue-synchronised [`key_is_down`] the capture
/// uses, for the same reason: the state wanted is the one of the keystroke being handled.
fn arrow_key_is_down() -> bool {
    [VK_LEFT, VK_UP, VK_RIGHT, VK_DOWN]
        .into_iter()
        .any(key_is_down)
}

/// One command from the dialog.
///
/// # Safety
///
/// Called from [`dialog_proc`] only, with the `hwnd` of the dialog it belongs to.
unsafe fn on_command(hwnd: HWND, control: i32, notification: u16) {
    // FR-92а, task T-11-5b. ⚠ First, before the match below reads anything: the flip of
    // the self-switching must land before any logic that reads `is_checked` — the mode
    // branch below reads the state of `IDC_MODE_CYCLE`, and it must see the state as this
    // very click has just made it, not as it was before.
    restore_self_switching(hwnd, control, notification);

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

            // FR-92а, task T-11-7-2: the mode the gate and the row paints of the layout
            // list read through `with_state` — kept current on the very click, the same
            // road the palette takes. Nothing is decided early: `read_dialog` still reads
            // the radio buttons afresh on «Применить», and «Отмена» still drops the whole
            // working copy.
            //
            // SAFETY: see the caller.
            unsafe { with_state(hwnd, |state| state.working.layouts.mode = mode) };

            enable_by_mode(hwnd, mode);

            // The muted or full row paints are custom-draw state, not window state: with
            // `EnableWindow` gone from the list (task T-11-7-2) nothing else would repaint
            // it on a mode flip.
            repaint_control(hwnd, IDC_CYCLE_LIST);
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

/// Re-resolves the palette and repaints the dialog if the resolution moved — FR-92а, the
/// common tail of a pressed «Применить» (task T-11-4) and of the system theme changing under
/// the open dialog (task T-11-9).
///
/// The palette is resolved afresh — the setting may have changed, and under `system` the
/// system switch may have flipped while the dialog was up — and only a *changed* resolution
/// repaints: `resolve` answers `&'static` identity, so pointer equality is the whole test,
/// and an unchanged palette costs nothing here. That same comparison is the debounce of
/// T-11-9: Windows sends `WM_SETTINGCHANGE` in a batch, the first arrival repaints, and
/// every later one finds the palette already current and leaves.
fn refresh_palette(hwnd: HWND, state: &mut DialogState<'_>) {
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

        // FR-92а, task T-11-7: the list view holds palette state of its own —
        // the three `LVM_SET*COLOR` colours and the state image list of the
        // ticks — which no brush recreation reaches; both are handed the fresh
        // palette here, so the repaint below shows one whole dialog.
        paint_cycle_list(hwnd, fresh);
        install_check_images(hwnd, fresh);

        repaint_after_palette_change(hwnd);
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
            // next opening. Task T-11-9 runs the same refresh from its own message, which
            // is why the body lives in `refresh_palette` rather than here.
            refresh_palette(hwnd, state);
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

/// The whole geometry of one cell of the state image list of the layout list, in the pixels
/// of the window it is built for — task T-11-16, the pure half of [`draw_check_frame`].
///
/// The cell is what the row is laid out around: the control draws the state image at the left
/// edge of the row and the label immediately after it, so the cell carries the air before the
/// tick, the tick, and the air after it — [`LIST_TEXT_INSET`], [`LIST_CHECK_SIZE`] and
/// [`LIST_CHECK_TEXT_GAP`] mock-up pixels, in that order. Its **height** is the height of the
/// row ([`LAYOUT_ROW_HEIGHT_DLU`], in the pixels the caller mapped it to), because a report
/// list view takes the row height from the state image list and from nowhere a message can
/// reach; the tick is centred in it, as `$by = $ry + [int](($rowH - $bs)/2)` centres it in the
/// generator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckCell {
    /// Width of the whole cell — air, tick, air.
    pub width: i32,
    /// Height of the whole cell, which is the height of the row.
    pub height: i32,
    /// Left edge of the tick inside the cell.
    pub glyph_left: i32,
    /// Top edge of the tick inside the cell.
    pub glyph_top: i32,
    /// Side of the tick.
    pub glyph_side: i32,
}

/// The cell of the layout list at `dpi`, for a row `row_height` pixels tall — task T-11-16.
///
/// Pure, so the whole layout of a row is one table a test can read: the air of the mock-ups
/// before the tick, the tick itself, the air after it, and the tick centred on the row.
pub fn check_cell(dpi: i32, row_height: i32) -> CheckCell {
    let glyph_side = scaled(LIST_CHECK_SIZE, dpi);
    let glyph_left = scaled(LIST_TEXT_INSET, dpi);
    let height = row_height.max(glyph_side);

    CheckCell {
        width: glyph_left + glyph_side + scaled(LIST_CHECK_TEXT_GAP, dpi),
        height,
        glyph_left,
        glyph_top: (height - glyph_side) / 2,
        glyph_side,
    }
}

/// The order the two frames enter the state image list of [`build_check_image_list`]:
/// frame 0 — снята, frame 1 — взведена.
///
/// This array is what couples the drawing to the participation bits of FR-31: a state image
/// index is **one-based** — index 1 names frame 0 — so the frame at position `i` here answers
/// the mask `(i + 1) << 12`, which is [`UNCHECKED_IMAGE`] for the first frame and
/// [`CHECKED_IMAGE`] for the second, exactly the values [`set_row_check`] writes and
/// [`read_cycle_checks`] reads. The system pair of `LVS_EX_CHECKBOXES` sits in the same
/// order, which is why replacing the image list moves not a single state bit. A test holds
/// the coupling.
pub const CHECK_FRAME_ORDER: [bool; 2] = [false, true];

/// Fill, frame and mark of one check frame of the layout list — what [`check_frame_colors`]
/// answers and the whole of what [`draw_check_frame`] needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckFrameColors {
    /// What the whole 13×13 square is filled with.
    pub fill: COLORREF,
    /// The single-pixel frame — `None` for the checked frame the accent fill covers whole.
    pub frame: Option<COLORREF>,
    /// The two-stroke check mark — `None` while unchecked.
    pub mark: Option<COLORREF>,
}

/// The colours of one check frame in one palette — FR-92а, task T-11-7: the pure half of the
/// custom state image list, closed by a table test over both states and both palettes.
///
/// Colours of the given palette rather than roles, unlike [`glyph_color_roles`] and its kin:
/// the frames are painted into memory bitmaps outside any `WM_*` answer, so what the drawing
/// needs is the palette's own values — and the task words the function as «(взведена,
/// палитра) → краски кадра». The table stays closed all the same: every answer is a field of
/// `palette` and nothing else, so not a single colour number enters this module (§6.2).
///
/// The two rows quote the check-box cells of the glyph table of T-11-5b, so the ticks of the
/// list match the ticks of the dialog:
/// - **снята** — `field_bg` fill under the single-pixel `box_border` frame, no mark;
/// - **взведена** — `accent_bg` fill edge to edge, no frame, `accent_fg` check mark.
pub fn check_frame_colors(checked: bool, palette: &theme::Palette) -> CheckFrameColors {
    if checked {
        CheckFrameColors {
            fill: palette.accent_bg,
            frame: None,
            mark: Some(palette.accent_fg),
        }
    } else {
        CheckFrameColors {
            fill: palette.field_bg,
            frame: Some(palette.box_border),
            mark: None,
        }
    }
}

/// Ground, ink and selection treatment of one row of the layout list — what
/// [`cycle_row_paint`] answers and the whole of what the item prepaint of [`on_notify`]
/// writes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CycleRowPaint {
    /// The `(clrTextBk, clrText)` pair to write — `None` writes nothing, and the row keeps
    /// the colours [`paint_cycle_list`] gave the whole control.
    pub colours: Option<(COLORREF, COLORREF)>,
    /// Whether `CDIS_SELECTED` is removed from the draw state, so the control does not
    /// paint the system selection ground over the pair above.
    pub strip_selected: bool,
}

/// The paints of one row of the layout list, by mode and role — FR-92а, task T-11-7-2: the
/// pure half of the item prepaint, closed by a table test over both modes, both roles and
/// both palettes.
///
/// The table:
/// - **cycle, ordinary row** — nothing: the control draws with the `text` on `field_bg`
///   the three messages of [`paint_cycle_list`] already set;
/// - **cycle, selected row** — `sel_bg`/`sel_fg`, `CDIS_SELECTED` stripped: the selection
///   pair of the palette instead of the system highlight (task T-11-7 as it was);
/// - **pair, any row** — `field_bg`/`text_muted`, `CDIS_SELECTED` stripped: the logical
///   «выключенность» of task T-11-7-2. The window stays enabled — a disabled
///   `SysListView32` ignores its colours and paints the system wash, the defect this task
///   heals — so the disabled look is painted here: every row muted, and a row that still
///   carries `LVIS_SELECTED` wears the same muted paints as the rest rather than an active
///   selection.
///
/// Colours of the given palette rather than roles, as [`check_frame_colors`] above and for
/// the same reason; every answer is a field of `palette` and nothing else, so not a single
/// colour number enters this module (§6.2).
pub fn cycle_row_paint(
    mode: LayoutMode,
    selected: bool,
    palette: &theme::Palette,
) -> CycleRowPaint {
    match (mode, selected) {
        (LayoutMode::Pair, _) => CycleRowPaint {
            colours: Some((palette.field_bg, palette.text_muted)),
            strip_selected: true,
        },
        (LayoutMode::Cycle, true) => CycleRowPaint {
            colours: Some((palette.sel_bg, palette.sel_fg)),
            strip_selected: true,
        },
        (LayoutMode::Cycle, false) => CycleRowPaint {
            colours: None,
            strip_selected: false,
        },
    }
}

/// Hands the layout list the three colours of the resolved palette — FR-92а, task T-11-7.
///
/// `LVM_SETBKCOLOR` is the ground of the control below and around the rows, `LVM_SETTEXTCOLOR`
/// the ink of every label, `LVM_SETTEXTBKCOLOR` the ground directly behind the text — all
/// three the documented colour messages of a list view, named by FR-92а («`LVM_SETBKCOLOR` и
/// родственные»). They exist because a `SysListView32` asks its parent no `WM_CTLCOLOR*`
/// question: the brushes of T-11-4 cannot reach it. Sent with [`send_to`] — the way this file
/// already talks to its own list — on initialisation and again on every palette change.
fn paint_cycle_list(hwnd: HWND, palette: &theme::Palette) {
    // NFR-13: each message answers a success flag, examined in words and dropped: a refused
    // colour leaves the system one on exactly that surface — the dialog lives degraded, the
    // precedent of the refused brushes of T-11-4 — and the journal has no row for cosmetics
    // (reviews\T-11-1.md).
    for (message, color) in [
        (LVM_SETBKCOLOR, palette.field_bg),
        (LVM_SETTEXTCOLOR, palette.text),
        (LVM_SETTEXTBKCOLOR, palette.field_bg),
    ] {
        send_to(
            hwnd,
            IDC_CYCLE_LIST,
            message,
            0,
            isize::try_from(color.0).unwrap_or(0),
        );
    }
}

/// Replaces the state image list of the layout list with the two palette frames — FR-92а,
/// task T-11-7: `LVS_EX_CHECKBOXES` draws its ticks with the system pair, which stays light
/// in the dark palette (§10 п.9 called that the accepted price; the documented state image
/// list is what lifts it).
///
/// Everything else about the check boxes is untouched: the extended style stays on the
/// control, so clicking a square still flips the item between state images 1 and 2, and
/// [`set_row_check`]/[`read_cycle_checks`] keep speaking `LVIS_STATEIMAGEMASK` — the
/// participation mechanism of FR-31 is the same mechanism with different pictures.
fn install_check_images(hwnd: HWND, palette: &theme::Palette) {
    // The height of a row of the layout list — [`LAYOUT_ROW_HEIGHT_DLU`] dialog units of this
    // window (task T-11-16). It travels down the build because the **cell** of the state image
    // list is what a report list view takes its row height from; a refused `MapDialogRect`
    // leaves the cell as tall as the tick, which is the height the control had before this task
    // (NFR-13).
    let row_height = dialog_units(hwnd, 0, LAYOUT_ROW_HEIGHT_DLU)
        .map(|(_, vertical)| vertical)
        .unwrap_or(0);

    let Some(list) = build_check_image_list(palette, row_height) else {
        // NFR-13, examined in words: with no frames of our own the system pair simply
        // stays — light squares in the dark palette, exactly the price §10 п.9 already
        // words — which is better than ticks nobody can see at all. No journal row for
        // cosmetics (reviews\T-11-1.md).
        return;
    };

    // `LVM_SETIMAGELIST` with `LVSIL_STATE` — the documented replacement. The answer is the
    // handle of the list previously associated with the control, handed back precisely so
    // the caller can dispose of it: after the swap the control holds no reference to it.
    let previous = send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_SETIMAGELIST,
        usize::try_from(LVSIL_STATE).unwrap_or(0),
        list.0,
    );

    // The first swap answers the pair `LVS_EX_CHECKBOXES` created, every later one — the
    // frames of the previous palette; both are equally ours to free once the control has
    // let go. The list *currently* installed is deliberately never destroyed here: the
    // control destroys the image lists it holds when it is itself destroyed — the template
    // carries no `LVS_SHAREIMAGELISTS`, which is the one style that would keep it from
    // doing so.
    if previous != 0 {
        // SAFETY: `previous` is the handle the control just answered and no longer holds;
        // it is freed exactly once, here. The `BOOL` is examined in words and dropped —
        // a refusal would mean the handle was not a live image list of this process, which
        // the swap above makes unreachable (NFR-13).
        let _ = unsafe { ImageList_Destroy(Some(HIMAGELIST(previous))) };
    }
}

/// Builds the two-frame state image list of [`install_check_images`] — the outer layer of
/// three: takes and releases the screen DC, which is where the colour depth of the frames
/// comes from.
fn build_check_image_list(palette: &theme::Palette, row_height: i32) -> Option<HIMAGELIST> {
    // SAFETY: the screen DC of this process; released below, on every path.
    let screen = unsafe { GetDC(None) };

    if screen.is_invalid() {
        // NFR-13: examined — no DC, no frames; the caller words the degradation.
        return None;
    }

    let list = build_check_frames(screen, palette, row_height);

    // SAFETY: releases exactly the DC taken above, once.
    unsafe { ReleaseDC(None, screen) };

    list
}

/// The middle layer of [`build_check_image_list`]: owns the memory DC the frames are drawn
/// through. ⚠ The bitmaps are compatible with the **screen**, not with this DC: a memory DC
/// is born with a monochrome bitmap selected, and a bitmap compatible with *it* would carry
/// one bit per pixel — the classic trap the task's «в память» route walks past.
fn build_check_frames(
    screen: HDC,
    palette: &theme::Palette,
    row_height: i32,
) -> Option<HIMAGELIST> {
    // SAFETY: a memory DC over the live screen DC; deleted below, on every path.
    let dc = unsafe { CreateCompatibleDC(Some(screen)) };

    if dc.is_invalid() {
        // NFR-13: examined — as in the caller.
        return None;
    }

    let list = draw_frames_into_list(screen, dc, palette, row_height);

    // SAFETY: deletes exactly the DC created above, once; the frame bitmaps were deselected
    // before their own deletion, so nothing of ours is still selected into it.
    let _ = unsafe { DeleteDC(dc) };

    list
}

/// The colour of the air beside the tick, made transparent by the mask of the image list —
/// task T-11-15.
///
/// `ImageList_AddMasked` is the documented way to a mask: it builds one from the bitmap
/// itself, turning every pixel of this colour into a hole the row shows through. The colour
/// is therefore never seen — it only has to be a colour the frames themselves never use, and
/// magenta is the traditional key for exactly that reason. A test holds it apart from every
/// colour of both palettes, so a palette can never grow a field that would punch a hole in
/// its own tick.
const CHECK_CELL_KEY: COLORREF = COLORREF(0x00FF_00FF);

/// The inner layer of [`build_check_image_list`]: the image list itself and the two frames,
/// in the order of [`CHECK_FRAME_ORDER`]. Any refusal destroys the half-built list and
/// answers `None` — a one-frame list would silently shift the meaning of state image 2.
fn draw_frames_into_list(
    screen: HDC,
    dc: HDC,
    palette: &theme::Palette,
    row_height: i32,
) -> Option<HIMAGELIST> {
    // The whole cell, in the pixels of the screen the frames are made for — the DPI of the
    // screen DC this whole build hangs from (NFR-13: a DC that will not say is answered as 96
    // by `dc_dpi`, which is the 100 % cell).
    let dpi = dc_dpi(screen);
    let cell = check_cell(dpi, row_height);

    // SAFETY: plain numbers in, a handle out, owned by this frame until it is either handed
    // to the caller or destroyed below. `ILC_MASK` beside `ILC_COLOR32` — the tick is opaque
    // and the air beside it is a hole, which is what [`CHECK_CELL_KEY`] is for.
    let list = unsafe { ImageList_Create(cell.width, cell.height, ILC_COLOR32 | ILC_MASK, 2, 0) };

    if list.is_invalid() {
        // NFR-13: examined — as in the callers.
        return None;
    }

    for checked in CHECK_FRAME_ORDER {
        // SAFETY: compatible with the *screen* DC — see the caller's ⚠ — and owned by this
        // frame until the `DeleteObject` below.
        let bitmap = unsafe { CreateCompatibleBitmap(screen, cell.width, cell.height) };

        if bitmap.is_invalid() {
            // SAFETY: the half-built list is ours until handed out; freed exactly once.
            let _ = unsafe { ImageList_Destroy(Some(list)) };
            return None;
        }

        // SAFETY: both handles are live and ours; the previous bitmap is kept and put back
        // below — `ImageList_AddMasked` reads the bitmap's bits, and a bitmap still selected
        // into a DC is not readable.
        let previous = unsafe { SelectObject(dc, bitmap.into()) };

        draw_check_frame(dc, checked, palette, cell, dpi);

        // SAFETY: restores the bitmap that was in the DC a moment ago.
        unsafe { SelectObject(dc, previous) };

        // SAFETY: `list` and `bitmap` are live and ours; the call copies the bits, builds the
        // mask from [`CHECK_CELL_KEY`] and keeps no handle.
        let added = unsafe { ImageList_AddMasked(list, bitmap, CHECK_CELL_KEY) };

        // SAFETY: deselected above, copied into the list, freed exactly once. The `BOOL`
        // is dropped for the reason `draw_check_mark` gives for its pen.
        let _ = unsafe { DeleteObject(bitmap.into()) };

        if added < 0 {
            // NFR-13: examined — `ImageList_AddMasked` answers the index or -1.
            //
            // SAFETY: as for the refused bitmap above.
            let _ = unsafe { ImageList_Destroy(Some(list)) };
            return None;
        }
    }

    Some(list)
}

/// Paints one cell of the state image list into the bitmap currently selected into `dc`:
/// [`CHECK_CELL_KEY`] air over the whole of `cell`, then the square of the tick where
/// [`check_cell`] puts it, in the colours of [`check_frame_colors`] — task T-11-16 for the
/// geometry, task T-11-7 for the colours.
///
/// The brushes are transient, exactly as the pens of [`draw_check_mark`]: nothing here
/// outlives the paint, so nothing belongs in `theme::Brushes`, whose reason to exist is
/// answers that must outlive it. The square is rounded by [`LIST_CHECK_CORNER_RADIUS`] and the
/// mark is [`draw_check_mark`] with [`LIST_CHECK_MARK`] — the tick of the list has its own
/// figure in the mock-ups, one size smaller than the tick of a dialog check box.
///
/// `dpi` is the DPI the frames are being built for and is passed in rather than read off `dc`:
/// this is a memory DC, whose own answer is not it.
fn draw_check_frame(dc: HDC, checked: bool, palette: &theme::Palette, cell: CheckCell, dpi: i32) {
    let whole = RECT {
        left: 0,
        top: 0,
        right: cell.width,
        bottom: cell.height,
    };

    let frame = RECT {
        left: cell.glyph_left,
        top: cell.glyph_top,
        right: cell.glyph_left + cell.glyph_side,
        bottom: cell.glyph_top + cell.glyph_side,
    };

    // The air first, over the whole cell: the tick is painted on top of it, and what stays
    // uncovered becomes the hole of the mask.
    //
    // SAFETY: a plain colour in, a handle out, owned by this frame until the `DeleteObject`
    // below.
    let key = unsafe { CreateSolidBrush(CHECK_CELL_KEY) };

    if key.is_invalid() {
        // NFR-13: examined — no brush, no air; without a key colour the mask would come out
        // of whatever the fresh bitmap held, so the honest answer is to leave the cell alone
        // and let the callers' degradation words cover it.
        return;
    }

    // SAFETY: `dc` holds the cell bitmap for exactly this call; `key` is the live brush just
    // made. The answer is dropped for the NFR-13 reason `draw_glyph_element` gives.
    unsafe { FillRect(dc, &whole, key) };

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(key.into()) };

    let colors = check_frame_colors(checked, palette);

    // SAFETY: a plain colour in, a handle out, owned by this frame until the `DeleteObject`
    // below.
    let fill = unsafe { CreateSolidBrush(colors.fill) };

    if fill.is_invalid() {
        // NFR-13: examined — no brush, no fill; the frame stays whatever the fresh bitmap
        // held, and the callers' degradation words cover it.
        return;
    }

    // The square, rounded by [`LIST_CHECK_CORNER_RADIUS`] — `FillRectPx … 2` and
    // `StrokeRectPx … 2 1` of the `'lview'` arm (task T-11-16). One `RoundRect` for fill and
    // frame, exactly as the glyph of a dialog check box is drawn: a square `FillRect` under a
    // square `FrameRect` cannot have a corner radius at all, and the frameless cell — the
    // checked one, whose accent fill covers it whole — is outlined in its own fill.
    //
    // ⚠ [`stroke_rounded`] and **not** [`paint_rounded`]: this cell is drawn over
    // [`CHECK_CELL_KEY`], and `ImageList_AddMasked` makes a hole of every pixel that is
    // *exactly* that colour. A smoothed corner is a blend of the key and the fill — no longer
    // the key, so no longer a hole — and the tick would carry a magenta fringe on every row.
    // The corner it costs is [`LIST_CHECK_CORNER_RADIUS`] — one pixel at 96 DPI, two at 140 % —
    // while the figure the eye actually reads here, the tick itself, **is** smoothed below by
    // [`draw_check_mark`], which strokes it well inside the fill where no mask can be harmed.
    // Written down in the report of task T-11-17 as the one deliberate exception.
    stroke_rounded(
        dc,
        &frame,
        scaled(LIST_CHECK_CORNER_RADIUS, dpi),
        scaled(BORDER_THICKNESS, dpi).max(1),
        colors.frame.unwrap_or(colors.fill),
        fill,
    );

    // SAFETY: created above, handed to nobody, freed exactly once — see `draw_check_mark`
    // on the dropped `BOOL`.
    let _ = unsafe { DeleteObject(fill.into()) };

    if let Some(ink) = colors.mark {
        draw_check_mark(dc, &frame, ink, LIST_CHECK_MARK, dpi);
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
///
/// **Except the list itself — task T-11-7-2.** A `SysListView32` under `EnableWindow(FALSE)`
/// ignores its own `LVM_SET*COLOR` colours and paints the system wash — the defect the final
/// sweep found: a light body in both palettes, an unreadable second row in the dark one. So
/// the list stays enabled at the window level always, and its pair-mode «выключенность» is
/// logical instead: every change the control would make is refused before any action by the
/// gate of [`on_notify`] ([`cycle_list_change_is_refused`]), and the rows wear the muted
/// paints of [`cycle_row_paint`]. The two arrow buttons keep the honest disable — owner
/// drawing paints a disabled button grey with the palette's own colours, so the defect never
/// touched them.
fn enable_by_mode(hwnd: HWND, mode: LayoutMode) {
    let cycle = matches!(mode, LayoutMode::Cycle);

    for control in [IDC_PAIR_SOURCE, IDC_PAIR_TARGET] {
        enable(hwnd, control, !cycle);
    }

    for control in [IDC_CYCLE_UP, IDC_CYCLE_DOWN] {
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

/// Ticks or unticks one check box — in the dialog's own store, task T-11-5b-2.
///
/// An owner-drawn button keeps no check state of its own — the button-message pair that
/// stores it for the automatic types ignores the write for a `BS_OWNERDRAW` one — so the
/// state goes into [`GlyphChecks`], and the element is repainted here, by the writer,
/// because writing a store changes no pixels by itself. A dialog with no state yet has no
/// store to write and nothing on screen to go stale: the write is dropped and the repaint
/// skipped (NFR-13; in practice every caller runs after `WM_INITDIALOG` stored the
/// pointer).
fn set_check(hwnd: HWND, control: i32, checked: bool) {
    // SAFETY: `hwnd` is the live dialog of `show_dialog` — every caller of this helper
    // holds exactly that window.
    let stored = unsafe { with_glyph_checks(hwnd, |checks| checks.set(control, checked)) };

    if stored.is_some() {
        repaint_control(hwnd, control);
    }
}

/// Whether one check box or radio button is ticked — the dialog's own store answers,
/// task T-11-5b-2.
///
/// «Снят» when the state is unreachable — before `WM_INITDIALOG` stores the pointer, or
/// for an identifier the store does not carry: the degraded answer NFR-13 asks for, and
/// the one [`draw_glyph_element`] draws when a `WM_DRAWITEM` outruns initialisation.
fn is_checked(hwnd: HWND, control: i32) -> bool {
    // SAFETY: as in `set_check` — `hwnd` is the live dialog of `show_dialog`.
    unsafe { with_glyph_checks(hwnd, |checks| checks.get(control)) }.unwrap_or(false)
}

/// Ticks exactly one of a group of radio buttons — in the dialog's own store,
/// task T-11-5b-2: every identifier of `first..=last` is quenched, `chosen` alone is
/// armed, and the whole range is repainted — the neighbour that just lost the dot must
/// lose it on the same occasion its winner gains it. The no-state path is `set_check`'s:
/// write dropped, repaint skipped (NFR-13).
fn check_radio(hwnd: HWND, first: i32, last: i32, chosen: i32) {
    // SAFETY: as in `set_check` — `hwnd` is the live dialog of `show_dialog`.
    let stored =
        unsafe { with_glyph_checks(hwnd, |checks| checks.check_radio(first, last, chosen)) };

    if stored.is_some() {
        repaint_control_range(hwnd, first, last);
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

/// Repaints one control now — task T-11-5b: writing check state changes no pixels of an
/// owner-drawn button, so [`set_check`] asks for the repaint the moment it writes the
/// store (task T-11-5b-2).
fn repaint_control(hwnd: HWND, control: i32) {
    // SAFETY: `hwnd` is the live dialog and `control` names a control of its template;
    // the crate turns a missing control into an error, which is the `Ok` guard below.
    let Ok(window) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
        crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
        return;
    };

    // NFR-13: examined in words and deliberately dropped — the call refuses only for a
    // window that is not alive, this one was found the line above, and the journal has no
    // row for GDI refusals (reviews\T-11-1.md).
    //
    // SAFETY: `window` is the live control just found; a null rectangle means its whole
    // client area, and the call keeps no pointer.
    let _ = unsafe { InvalidateRect(Some(window), None, true) };
}

/// Repaints every control of one contiguous identifier range — the radio ranges of task
/// T-11-5b, the same runs [`check_radio`] walks in the store.
fn repaint_control_range(hwnd: HWND, first: i32, last: i32) {
    for control in first..=last {
        repaint_control(hwnd, control);
    }
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

// =========================================================================================
// FR-92а, task T-11-11 — the about dialog
// =========================================================================================
//
// The window that replaces the tray's `MessageBoxW`, which no documented means can repaint.
// It lives here with the settings dialog because §6.2 makes this module the owner of the
// dialogs, and because everything it paints with — the `WM_CTLCOLOR*` answers, the
// owner-drawn button path, the DWM title bar — already lives here and is *shared*, not
// copied: `apply_ctl_color`, `button_color_roles` + `resolve_button_colors`,
// `paint_push_button`, `apply_title_bar_theme`, `with_window_state`.

/// Everything the about dialog needs, for as long as it is up.
///
/// Lives on the frame of [`show_about_dialog`] and is reached the way [`DialogState`] is:
/// through a `RefCell` behind the pointer `GWLP_USERDATA` carries. Deliberately this small.
/// SEC-05 says of the box this window replaces: a window that changes nothing — and a state
/// holding no configuration and no callback is how the new window stays exactly that: there
/// is nothing here a handler could change.
struct AboutState {
    /// The palette of this window, resolved once at the moment it is opened. The window is
    /// short-lived and is not repainted on the fly: nobody posts it
    /// [`WM_APP_SYSTEM_THEME`], and the next opening resolves afresh.
    palette: &'static theme::Palette,
    /// The brushes of `palette`, created for this window alone and dropped however
    /// [`show_about_dialog`] leaves. `None` — `CreateSolidBrush` refused — is survived
    /// exactly as the settings dialog survives it: every colour answer says «not handled»
    /// and the window lives with the system colours (NFR-13).
    brushes: Option<theme::Brushes>,
    /// The four-part file version out of the `VERSIONINFO` resource of the running
    /// executable, read by the caller the same way the old box read it
    /// (`tray::file_version`). `None` — no resource — shows as a dash, not as an error.
    version: Option<(u16, u16, u16, u16)>,
    /// The face this window sets the caption of its own «ОК» in — FR-92а, task T-11-17.
    ///
    /// The same owner the settings dialog keeps, for the same reason and with the same
    /// degradation: `None` draws in the manager's own font. `None` until `WM_INITDIALOG` —
    /// the face is made out of the font the manager gives the window, and there is no window
    /// when this value is built.
    fonts: Option<DialogFonts>,
}

/// Shows the modal «О программе» window of FR-92а — what `tray` calls in place of the
/// `MessageBoxW` it used to show, from the same place in the same order.
///
/// Same shape as [`show_dialog`]: the state lives on this frame, `DialogBoxParamW` runs
/// the modal loop, and the palette is resolved here, at the moment of opening — from the
/// setting the caller copied out of the configuration and one read of the system switch —
/// so the first paint is one consistent palette; the brushes die with this frame.
///
/// SEC-05: the window changes nothing, exactly as the box it replaces — no configuration,
/// no file, no registry; every command it answers ends it.
pub fn show_about_dialog(
    owner: HWND,
    instance: HINSTANCE,
    setting: ThemeSetting,
    version: Option<(u16, u16, u16, u16)>,
) -> windows::core::Result<()> {
    let palette = theme::resolve(setting, theme::system_is_light());

    let state = RefCell::new(AboutState {
        palette,
        brushes: theme::Brushes::new(palette),
        version,
        // `None` until `WM_INITDIALOG` — see the field.
        fonts: None,
    });

    // SAFETY: `instance` is a module handle whose resources carry `IDD_ABOUT`, and the
    // "name" is an integer identifier in the `MAKEINTRESOURCE` form — a value below 65536
    // carried inside the pointer, never dereferenced as a string. `owner` is a live window
    // of this thread. The parameter is a pointer to `state`, which lives on this frame:
    // the call is modal and does not return until `EndDialog`, so the pointer cannot
    // outlive the value it names. `about_proc` is the only reader of it and reads it
    // through the `RefCell`, so no two borrows can overlap however the manager re-enters.
    let result = unsafe {
        DialogBoxParamW(
            Some(instance),
            resource_id(IDD_ABOUT),
            Some(owner),
            Some(about_proc),
            LPARAM(std::ptr::from_ref(&state) as isize),
        )
    };

    // NFR-13. `DialogBoxParamW` answers -1 when the dialog could not be created at all,
    // and that is the one outcome that is a failure — exactly as in `show_dialog`.
    if result == -1 {
        return Err(WinError::from_thread());
    }

    Ok(())
}

/// The dialog procedure of the about window.
///
/// # Safety
///
/// Called by the dialog manager with the arguments of a window message. `hwnd` names the
/// live dialog, and on `WM_INITDIALOG` `lparam` is the pointer [`show_about_dialog`]
/// passed and nothing else — the manager forwards it unchanged and no other sender can
/// reach this procedure (SEC-05), exactly as for [`dialog_proc`].
unsafe extern "system" fn about_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match message {
        WM_INITDIALOG => {
            // SAFETY: `hwnd` is the live dialog and `GWLP_USERDATA` is a field every
            // window has, which the dialog manager does not use for itself. The value
            // stored is the pointer the manager forwarded from `DialogBoxParamW`; it is
            // only ever read back by `with_about_state`.
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0) };

            // SAFETY: the pointer has just been stored and names the `RefCell` on the
            // frame of `show_about_dialog`, which outlives this modal call.
            unsafe {
                with_about_state(hwnd, |state| {
                    fill_about(hwnd, state.version);

                    // FR-92а, task T-11-17: the face the caption of «ОК» is set in, made out
                    // of the font the manager gave the window — the same call, and the same
                    // ownership, as in the settings dialog.
                    state.fonts = DialogFonts::new(hwnd, OK_COMMAND);

                    // The non-client title bar follows the resolved palette from the
                    // first showing — the same call the settings dialog makes.
                    apply_title_bar_theme(hwnd, state.palette);
                })
            };

            // «ОК» is BS_OWNERDRAW, so the default identifier is handed to the dialog
            // manager by the documented replacement, `DM_SETDEFID` — *posted*, not sent,
            // for the reasons written down at the same message of `dialog_proc` (FR-72).
            // It is what keeps Enter landing on «ОК».
            //
            // SAFETY: `hwnd` is the live dialog; the message carries two plain integers
            // and no pointer — `PostMessageW` queues them by value and returns.
            if let Err(error) = unsafe {
                PostMessageW(
                    Some(hwnd),
                    DM_SETDEFID,
                    WPARAM(usize::try_from(OK_COMMAND).unwrap_or(0)),
                    LPARAM(0),
                )
            } {
                // NFR-13. Not fatal: Enter would be answered by the dialog manager's
                // fallback instead of the named default, and the journal is told.
                crate::app::report_non_critical("PostMessageW", &error);
            }

            // TRUE: let the dialog manager choose the focus — «ОК» is the one tab stop.
            1
        }

        // The colour questions of this window, answered from its own palette; the applying
        // half is the shared `apply_ctl_color` — task T-11-11 reuses the paths of the
        // settings dialog rather than copying them (§6.2). `WM_CTLCOLOREDIT` and
        // `WM_CTLCOLORLISTBOX` are deliberately not routed: this window has no field and
        // no list.
        WM_CTLCOLORDLG | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            // SAFETY: as above — the pointer was stored on `WM_INITDIALOG` and the value
            // it names is alive for the whole of this modal call.
            unsafe { on_about_ctl_color(hwnd, message, wparam, lparam) }
        }

        // The one owner-drawn button of this window. The gate the last sentence of SEC-05
        // names is the same as everywhere: the message is handled only while the program
        // itself holds this dialog on the screen, and nothing of it is dereferenced
        // beyond the checked fields and the drawing rectangle.
        WM_DRAWITEM => {
            // SAFETY: the sender owns the struct `lparam` names for the length of the
            // send, and this procedure is inside that send.
            unsafe { on_about_draw_item(hwnd, lparam) }
        }

        WM_COMMAND => {
            let control = i32::from(low_word(wparam.0));

            // «ОК» and Esc — `IDCANCEL`, which the dialog manager sends whether or not
            // the window has the button — both simply end the window: it changes nothing,
            // so there is nothing to apply and nothing to undo (SEC-05).
            if control == OK_COMMAND || control == CANCEL_COMMAND {
                end_dialog(hwnd, isize::try_from(control).unwrap_or(0));
            }

            0
        }

        _ => 0,
    }
}

/// Puts the strings of the locale in force into the about window — FR-94 — and composes
/// the version line out of the `VERSIONINFO` the caller read.
///
/// The name row is deliberately not set: «Lang Switcher» is not translated — decision on
/// question 7 — and the template literal already is the name.
fn fill_about(hwnd: HWND, version: Option<(u16, u16, u16, u16)>) {
    let caption = wide(&text(IDS_ABOUT_CAPTION));

    // SAFETY: `hwnd` is the live dialog and `caption` is a NUL-terminated UTF-16 buffer
    // owned by this frame, neither moved nor dropped until the call returns; the call
    // copies it.
    if let Err(error) = unsafe { SetWindowTextW(hwnd, PCWSTR(caption.as_ptr())) } {
        crate::app::report_non_critical("SetWindowTextW", &error);
    }

    set_text(hwnd, IDC_ABOUT_VERSION, &about_version_line(version));
    set_text(hwnd, IDC_ABOUT_LINE_1, &text(IDS_ABOUT_LINE_1));
    set_text(hwnd, IDC_ABOUT_LINE_2, &text(IDS_ABOUT_LINE_2));
    set_text(hwnd, OK_COMMAND, &text(IDS_ABOUT_OK));
}

/// The version line of the about window: [`IDS_ABOUT_VERSION`] with the four-part number
/// substituted — FR-94. The number comes out of the `VERSIONINFO` resource of the running
/// executable, read by the caller of [`show_about_dialog`] the same way the old box read
/// it, and is never spelled in the source.
///
/// `None` — a binary without the resource — shows a dash: not a failure, and next to
/// impossible for this window, because the template it was created from lives in the same
/// `app.rc` as the version block.
///
/// Public so the test can call the very function the dialog calls.
pub fn about_version_line(version: Option<(u16, u16, u16, u16)>) -> String {
    let number = version.map_or_else(
        || "—".to_owned(),
        |(major, minor, build, revision)| format!("{major}.{minor}.{build}.{revision}"),
    );

    format_text(IDS_ABOUT_VERSION, &[&number])
}

/// The colour role of one static of the about window — the closed vocabulary
/// [`static_color_role`] already answers with, reused rather than widened (§6.2): the
/// version line is the quiet one, everything else — the icon, the name and the two
/// description lines — is an ordinary caption. Nothing in this window is a field, so
/// [`StaticColorRole::Field`] never comes out of it.
///
/// Public for the same reason as [`static_color_role`]: the table test calls the function
/// the dialog calls.
pub fn about_static_color_role(control: i32) -> StaticColorRole {
    match control {
        IDC_ABOUT_VERSION => StaticColorRole::Muted,
        // Spelled out rather than swallowed by the catch-all, so the mirrored identifiers
        // of the template stay load-bearing in exactly one function.
        IDC_ABOUT_ICON | IDC_ABOUT_NAME | IDC_ABOUT_LINE_1 | IDC_ABOUT_LINE_2 => {
            StaticColorRole::Label
        }
        _ => StaticColorRole::Label,
    }
}

/// The `WM_CTLCOLOR*` answers of the about window — the choosing half; the applying half
/// is the shared [`apply_ctl_color`] (§6.2, task T-11-11: one body, not a copy).
///
/// SEC-05 is held exactly as in [`on_ctl_color`]: the whole of what is taken out of the
/// message is two handles, the control is only asked for its identifier, the DC is only
/// written to. Zero — «not handled», the system colours — whenever the state is not
/// reachable or [`theme::Brushes::new`] was refused at initialisation (NFR-13).
///
/// # Safety
///
/// Called from [`about_proc`] only, with the arguments of the message.
unsafe fn on_about_ctl_color(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    // The two handles of the message — see above; this is all that is ever read out of it.
    let dc = HDC(wparam.0 as *mut std::ffi::c_void);
    let control = HWND(lparam.0 as *mut std::ffi::c_void);

    // SAFETY: `control` is a window handle out of the message; asking for its identifier
    // reads a field of that window and no memory of ours. For `WM_CTLCOLORDLG` the handle
    // is the dialog itself and the identifier goes unused.
    let control_id = unsafe { GetDlgCtrlID(control) };

    // The colour choice, split from the painting exactly as `on_ctl_color` splits it: the
    // borrow of the state ends before the DC is touched.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_about_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13).
            let brushes = state.brushes.as_ref()?;
            let palette = state.palette;

            Some(match message {
                // The window itself and the erase under the owner-drawn «ОК»: the window
                // brush is the whole answer — this dialog has no panels.
                WM_CTLCOLORDLG | WM_CTLCOLORBTN => (None, None, brushes.window_bg()),

                // The statics: the role decides the ink, the ground is always the window
                // brush. The `Field` arm keeps the match closed over the shared
                // vocabulary; `about_static_color_role` never answers it.
                WM_CTLCOLORSTATIC => match about_static_color_role(control_id) {
                    StaticColorRole::Label => (Some(palette.text), None, brushes.window_bg()),
                    StaticColorRole::Muted => (Some(palette.text_muted), None, brushes.window_bg()),
                    StaticColorRole::Field => (
                        Some(palette.text),
                        Some(palette.field_bg),
                        brushes.field_bg(),
                    ),
                },

                // Unreachable: the caller only routes the three messages above here.
                _ => return None,
            })
        })
    };

    apply_ctl_color(dc, choice.flatten())
}

/// Draws one owner-drawn label of the about window — FR-92а, task T-11-18.
///
/// The choosing half; the painting half is the shared [`paint_label`] (§6.2: the settings
/// dialog's path, not a copy of it). Three differences from [`draw_label`] and no more: the
/// state is this window's, the ground is always the window brush — this dialog has no panels —
/// and the identifier's role comes from [`about_static_color_role`], the mapping task T-11-11
/// already wrote for exactly these five controls.
///
/// FR-94 holds here as it does there: the caption is read back off the control `fill_about`
/// wrote it into, and the version line composed from the `VERSIONINFO` resource arrives by the
/// same road.
///
/// # Safety
///
/// Called from [`on_about_draw_item`] only, with values copied out of the `WM_DRAWITEM`
/// message it is inside of.
unsafe fn draw_about_label(hwnd: HWND, control: i32, dc: HDC, rect: RECT) -> isize {
    // The colour choice, split from the painting — the borrow ends before the DC is touched,
    // exactly as everywhere in this file.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_about_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13).
            let brushes = state.brushes.as_ref()?;

            Some((
                brushes.window_bg(),
                label_ink(about_static_color_role(control), state.palette),
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((ground, ink, face))) = choice else {
        return 0;
    };

    // Read after the borrow ends, for the reason `draw_label` gives. Without the trailing NUL:
    // `DrawTextW` takes the length of the slice it is given.
    let mut caption: Vec<u16> = get_text(hwnd, control).encode_utf16().collect();

    // SAFETY: see the caller — `dc` and `rect` are the values of the message; `ground` and
    // `face` are objects this window's state owns for longer than this call.
    unsafe { paint_label(dc, rect, &mut caption, ground, ink, face) }
}

/// The `WM_DRAWITEM` of the about window — the four labels of task T-11-18 and one
/// owner-drawn button, «ОК», painted by the shared [`paint_push_button`] with the colours the
/// shared [`button_color_roles`] table chose, on which `IDOK` is the accent (§6.2, task
/// T-11-11: the paths of the settings dialog, not copies of them).
///
/// SEC-05, held as in [`on_draw_item`]: the control type and the identifier are checked
/// before any work, the same copied fields and nothing else are taken out of the message,
/// `itemData` — never.
///
/// # Safety
///
/// Called from [`about_proc`] only, with the `lparam` of the message: the sender owns the
/// struct it names for the length of the send, and this procedure is inside that send.
unsafe fn on_about_draw_item(hwnd: HWND, lparam: LPARAM) -> isize {
    if lparam.0 == 0 {
        return 0;
    }

    // The one dereference of the message — the same copied fields as `on_draw_item`,
    // taken out as plain values before anything else runs.
    //
    // SAFETY: see above — the dialog manager owns the struct for the length of the send,
    // and this procedure is inside that send.
    let item = unsafe { &*(lparam.0 as *const DRAWITEMSTRUCT) };
    let (ctl_type, ctl_id, item_state, dc, rect) = (
        item.CtlType,
        item.CtlID,
        item.itemState,
        item.hDC,
        item.rcItem,
    );

    let control = i32::try_from(ctl_id).unwrap_or(-1);

    // FR-92а, task T-11-18: the four labels of this window, by control type and then by
    // identifier — the same gate and the same order the settings dialog uses. `itemState` is
    // deliberately not consulted: a label takes no focus and none of these is ever disabled.
    if ctl_type == ODT_STATIC {
        if !OWNER_DRAWN_ABOUT_LABELS.contains(&control) {
            return 0;
        }

        // SAFETY: see the caller — `dc` and `rect` are the values of the message, used only
        // to paint into for the length of this send.
        return unsafe { draw_about_label(hwnd, control, dc, rect) };
    }

    // The only owner-drawn *button* of this window is its «ОК».
    if ctl_type != ODT_BUTTON || control != OK_COMMAND {
        return 0;
    }

    let pressed = item_state.0 & ODS_SELECTED.0 != 0;
    let disabled = item_state.0 & ODS_DISABLED.0 != 0;
    let focused = item_state.0 & ODS_FOCUS.0 != 0 && item_state.0 & ODS_NOFOCUSRECT.0 == 0;

    // The colour choice, split from the painting — the borrow ends before the DC is
    // touched, exactly as everywhere in this file.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_about_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13).
            let brushes = state.brushes.as_ref()?;

            Some((
                resolve_button_colors(
                    button_color_roles(control, pressed, disabled),
                    brushes,
                    state.palette,
                ),
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((colors, face))) = choice else {
        return 0;
    };

    // SAFETY: see the caller — `dc` and `rect` are the values of the message, used only
    // to paint into for the length of this send; `face` is a face the state owns for longer.
    unsafe { paint_push_button(hwnd, control, dc, rect, colors, focused, face) }
}
