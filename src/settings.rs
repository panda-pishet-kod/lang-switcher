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
use windows::Win32::Graphics::Dwm::{
    DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWINDOWATTRIBUTE,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC,
    CreateSolidBrush, DT_CALCRECT, DT_CENTER, DT_END_ELLIPSIS, DT_SINGLELINE, DT_VCENTER, DeleteDC,
    DeleteObject, DrawFocusRect, DrawTextW, EndPaint, FW_BOLD, FillRect, GetDC, GetObjectW,
    GetTextExtentPoint32W, HBITMAP, HBRUSH, HDC, HFONT, HGDIOBJ, InvalidateRect, LOGFONTW,
    PAINTSTRUCT, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RedrawWindow, ReleaseDC, SRCCOPY,
    ScreenToClient, SelectObject, SetBkColor, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceExW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::UI::Controls::{
    CDDS_ITEMPREPAINT, CDDS_PREPAINT, CDIS_SELECTED, CDRF_DODEFAULT, CDRF_NOTIFYITEMDRAW, CLR_NONE,
    DRAWITEMSTRUCT, EM_LIMITTEXT, EM_SETMARGINS, HIMAGELIST, ICC_LISTVIEW_CLASSES, ILC_COLOR32,
    ILC_MASK, INITCOMMONCONTROLSEX, ImageList_AddMasked, ImageList_Create, ImageList_Destroy,
    ImageList_SetBkColor, InitCommonControlsEx, LIST_VIEW_ITEM_STATE_FLAGS, LVCF_WIDTH, LVCOLUMNW,
    LVIF_STATE, LVIF_TEXT, LVIR_BOUNDS, LVIS_FOCUSED, LVIS_SELECTED, LVIS_STATEIMAGEMASK, LVITEMW,
    LVM_DELETEALLITEMS, LVM_GETIMAGELIST, LVM_GETITEMRECT, LVM_GETITEMSTATE, LVM_GETNEXTITEM,
    LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETBKCOLOR, LVM_SETCOLUMNWIDTH,
    LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETIMAGELIST, LVM_SETITEMSTATE, LVM_SETTEXTBKCOLOR,
    LVM_SETTEXTCOLOR, LVN_ITEMCHANGING, LVNI_SELECTED, LVS_EX_CHECKBOXES, LVS_EX_FULLROWSELECT,
    LVSIL_STATE, MEASUREITEMSTRUCT, NM_CUSTOMDRAW, NMCUSTOMDRAW_DRAW_STATE_FLAGS, NMHDR,
    NMLVCUSTOMDRAW, ODS_COMBOBOXEDIT, ODS_DISABLED, ODS_FOCUS, ODS_NOFOCUSRECT, ODS_SELECTED,
    ODT_BUTTON, ODT_COMBOBOX, ODT_LISTBOX, ODT_STATIC, WM_MOUSELEAVE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, GetKeyState, IsWindowEnabled, SetFocus, TME_LEAVE, TRACKMOUSEEVENT,
    TrackMouseEvent, VIRTUAL_KEY, VK_APPS, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END,
    VK_ESCAPE, VK_F1, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_MENU, VK_NEXT, VK_NUMLOCK, VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RIGHT, VK_RMENU, VK_RSHIFT,
    VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SNAPSHOT, VK_UP,
};
use windows::Win32::UI::Shell::{
    DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, BN_DBLCLK, BN_SETFOCUS, CB_ADDSTRING, CB_ERR, CB_GETCURSEL, CB_GETLBTEXT,
    CB_GETLBTEXTLEN, CB_RESETCONTENT, CB_SETCURSEL, CB_SETITEMHEIGHT, CallWindowProcW,
    DLGC_WANTALLKEYS, DM_SETDEFID, DWLP_MSGRESULT, DefWindowProcW, DestroyIcon, DialogBoxParamW,
    EC_LEFTMARGIN, EC_RIGHTMARGIN, EndDialog, GW_CHILD, GW_HWNDNEXT, GWLP_USERDATA, GWLP_WNDPROC,
    GetClientRect, GetDlgCtrlID, GetDlgItem, GetDlgItemTextW, GetParent, GetWindow,
    GetWindowLongPtrW, GetWindowRect, HICON, ICON_BIG, ICON_SMALL, IDCANCEL, IDOK, IMAGE_ICON,
    LB_ADDSTRING, LB_DELETESTRING, LB_GETCOUNT, LB_GETCURSEL, LB_GETTEXT, LB_GETTEXTLEN,
    LB_RESETCONTENT, LR_DEFAULTCOLOR, LR_DEFAULTSIZE, LoadImageW, MapDialogRect, PostMessageW,
    STM_SETICON, SW_SHOWNORMAL, SendDlgItemMessageW, SetDlgItemTextW, SetWindowLongPtrW,
    SetWindowTextW, UISF_HIDEFOCUS, WINDOW_LONG_PTR_INDEX, WM_APP, WM_CHAR, WM_COMMAND,
    WM_CTLCOLORBTN, WM_CTLCOLORDLG, WM_CTLCOLOREDIT, WM_CTLCOLORLISTBOX, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_DRAWITEM, WM_ERASEBKGND, WM_GETDLGCODE, WM_GETFONT, WM_INITDIALOG, WM_KEYDOWN,
    WM_KEYUP, WM_KILLFOCUS, WM_MEASUREITEM, WM_MOUSEMOVE, WM_NCDESTROY, WM_NOTIFY, WM_PAINT,
    WM_QUERYUISTATE, WM_SETFONT, WM_SETICON, WM_SYSCHAR, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDPROC,
};
use windows::core::{Error as WinError, PCWSTR, PWSTR, w};

use crate::CONFIG_DIR_NAME;
use crate::layouts::{self, LayoutId, LayoutSpec};
// ⚠ The names after `ThemeSetting` are the drawing library task T-14-3 moved out of this file
// into its owner (§6.2, finding 24 of the audit of 2026-08-24). They are imported by name — not
// called as `theme::…` — so that the change of address stayed a change of address: every call
// site of this file reads exactly as it read before the move.
use crate::theme::{
    self, ButtonBorderRole, ButtonColors, ButtonFaceRole, ButtonTextRole, CHECK_FRAME_ORDER,
    CORNER_RADIUS, CheckMark, ComboBorderRole, ComboChevronRole, ComboFillRole, CornerColors,
    GlyphFillRole, GlyphFrameRole, GlyphKind, GlyphMarkRole, GlyphTextRole, HotBrush,
    ResolvedButtonColors, StaticColorRole, ThemeSetting, antialiased_logfont, caption_advance,
    check_frame_colors, combo_chevron_points, combo_closed_color_roles, combo_fill_brush,
    combo_item_color_roles, combo_text_ink, create_font, dc_dpi, draw_check_mark,
    draw_combo_chevron, field_frame_air, glyph_color_roles, label_ink, list_frame_air,
    list_frame_box, list_item_color_roles, paint_caption_underline, paint_ellipse, paint_label,
    paint_rounded, paint_rounded_corners, paint_selection_stripe, resolve_button_colors,
    restore_face, scaled, scaled_tenths_offset, select_face, title_bar_is_dark,
};

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
    ///
    /// # A file from a newer schema is recognised even when it will not parse — task T-19-4
    ///
    /// ⭐ **Finding 9 of the audit of 2026-08-31.** Serde ignores fields it has no name for, so
    /// a later schema that only *adds* keys parses here without complaint and reaches
    /// [`Config::migrate`], which stamps it [`ReadOutcome::FromNewerSchema`] and so
    /// [`SavePolicy::Forbidden`]. A later schema that adds one **value** to a field does not:
    /// the three enumerated fields of section 7 are closed on purpose — "a value outside it
    /// must not pass silently" — and the strict parse below refuses the whole document.
    ///
    /// Until this task that refusal came first and `schema_version` was never read at all. The
    /// file became [`ConfigError::Malformed`], `for_read` answered
    /// [`SavePolicy::QuarantineFirst`], and a configuration written by a newer build was moved
    /// into the single `.bad` slot and replaced by the defaults of this schema — the exact
    /// damage `FromNewerSchema` exists to prevent, done to the file the promise names.
    ///
    /// So a refused parse is asked one more question before it is called damage: what schema
    /// does the document claim? The strict parse still runs **first**, and every file of this
    /// schema or older takes exactly the path it always took — a version at or below
    /// [`CURRENT_SCHEMA_VERSION`] is `Malformed` as before, and a newer file that *does* parse
    /// still keeps every field this build understood, which an early return could not have
    /// given it.
    pub fn from_toml_str(text: &str) -> Result<(Self, ReadOutcome), ConfigError> {
        let error = match toml::from_str::<Self>(text) {
            Ok(mut config) => {
                let outcome = config.migrate();
                return Ok((config, outcome));
            }
            Err(error) => error,
        };

        // The parser refused. If the document names a schema this build is too old for, that
        // refusal is the expected answer to a file from the future and not a verdict on it.
        if let Some(version) = claimed_schema_version(text)
            && version > CURRENT_SCHEMA_VERSION
        {
            // The defaults of section 7, carrying the version the file claims. Nothing of the
            // file itself is kept — none of it was understood — and nothing of it is lost
            // either: `FromNewerSchema` forbids the write, so the bytes stay where they are.
            let config = Self {
                schema_version: version,
                ..Self::default()
            };

            return Ok((config, ReadOutcome::FromNewerSchema { version }));
        }

        Err(ConfigError::Malformed {
            at: error.span().map(|span| line_and_column(text, span.start)),
        })
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

/// The `schema_version` a document claims, read without the schema of this build — task
/// **T-19-4**.
///
/// Deliberately weak, and every way it can answer nothing is a way of saying "not a file from
/// the future": a document that is not TOML at all, one with no `schema_version`, one whose
/// stamp is not an integer, one whose stamp will not fit a `u32`. Each of those is a file this
/// build cannot read *and* cannot excuse, so each falls through to [`ConfigError::Malformed`]
/// and its quarantine, which is exactly where it belongs.
///
/// The document is parsed a second time here rather than once into a [`toml::Table`] and then
/// deserialised out of it. This runs on the failure path only — once per session, on a file that
/// is already refused — and the strict parse above stays the plain, obvious call it has always
/// been.
fn claimed_schema_version(text: &str) -> Option<u32> {
    let document = text.parse::<toml::Table>().ok()?;

    u32::try_from(document.get("schema_version")?.as_integer()?).ok()
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

// -----------------------------------------------------------------------------------------
// The decision `read_or_default` exists to be given — task T-13-6
// -----------------------------------------------------------------------------------------

/// Whether the configuration file may be written back, and what has to happen first.
///
/// **This is the delivery of the second half of the [`read_or_default`] pair to the place that
/// decides.** That pair exists so that "overwriting is a decision, not a side effect" can be
/// true of a caller which keeps the configuration in memory and writes it out again later —
/// [`crate::tray::Tray`], the one caller of [`write_to`] in the whole program. A value of this
/// type *is* that decision: taken once, at the moment of the read, and carried for the rest of
/// the session, because every later write is about the same file this read looked at.
///
/// The type is deliberately about the **file**, not about the configuration: the configuration
/// that comes back from [`read_or_default`] is usable in every case, and nothing here changes
/// that. A resident utility runs on the defaults of section 7 whatever the file says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavePolicy {
    /// The file may be written whenever the owner of the configuration asks.
    ///
    /// What every successful read answers, [`ReadOutcome::Migrated`] included: everything the
    /// file held has been understood and is in memory, so writing the memory back loses none
    /// of it. This is the ordinary case and the behaviour that was here before.
    Allowed,
    /// The bytes on disk are somebody's text that this build could not read. They are moved to
    /// [`quarantine_path_for`] before the first write, and if they cannot be moved, **nothing
    /// is written at all**.
    ///
    /// Section 7 leaves the file editable by hand and the dialog of FR-92 has no field for
    /// `[buffer] capacity`, so editing it by hand is the only way to set one. A stray bracket
    /// in it must therefore not cost the rest of it: the exclusions, the cycle order and the
    /// hotkey are still the user's, still on the disk, and the program has no business
    /// replacing them with defaults it merely fell back on.
    QuarantineFirst,
    /// The file must not be written at all, for the whole session.
    ///
    /// ⚠ **FR-83 asks for «сохранение конфигурации» at shutdown, and this refuses to perform
    /// it. The refusal is what performs it.** The reading is the one this module has already
    /// written down for itself at [`ReadOutcome::FromNewerSchema`]: "this build is not required
    /// to understand such a file, but it is required not to damage it". A file from a newer
    /// schema holds fields this build has no name for; [`Config::to_toml_string`] writes the
    /// schema it knows and nothing else, and [`Config::migrate`] deliberately leaves the newer
    /// `schema_version` in place. One write would therefore delete the unknown fields *and*
    /// leave the stamp that tells the newer build there is nothing to migrate — a loss neither
    /// build could afterwards detect. For such a file **not saving is what saving the user's
    /// configuration means**, and that is the sense in which FR-83 is honoured here rather than
    /// broken.
    Forbidden,
}

impl SavePolicy {
    /// The decision the second half of a [`read_or_default`] pair calls for.
    ///
    /// Every failed read maps to [`SavePolicy::QuarantineFirst`] and not only
    /// [`ConfigError::Malformed`]: a file that could not be *opened* is just as much text this
    /// build has not seen, the defaults that came back in its place are just as much not what
    /// is on the disk, and the move to [`quarantine_path_for`] either preserves those bytes or
    /// refuses the write. There is no failure of a read after which overwriting the file is
    /// known to be safe, so there is no arm here that says it is.
    pub fn for_read(outcome: &Result<ReadOutcome, ConfigError>) -> Self {
        match outcome {
            Ok(ReadOutcome::NoFile | ReadOutcome::Current | ReadOutcome::Migrated { .. }) => {
                Self::Allowed
            }
            Ok(ReadOutcome::FromNewerSchema { .. }) => Self::Forbidden,
            Err(_) => Self::QuarantineFirst,
        }
    }
}

/// What is appended to the name of a configuration file that is moved out of the way.
///
/// One name, with nothing unique in it, so that exactly one copy is ever kept: a program that
/// left a numbered trail of unreadable configurations in somebody's `%APPDATA%` would be
/// solving a small problem by making a permanent one.
pub const QUARANTINE_SUFFIX: &str = ".bad";

/// Where [`quarantine`] moves a configuration file this build could not read.
///
/// Beside the original and named after it — `config.toml.bad` next to `config.toml` — so that
/// whoever opens the folder of section 7 finds the two together and can see what happened by
/// looking. Same shape as [`temporary_path_for`], and same reason for the shape: a rename is
/// only atomic within one directory.
pub fn quarantine_path_for(path: &Path) -> PathBuf {
    let mut name = match path.file_name() {
        Some(name) => name.to_os_string(),
        None => OsString::from(CONFIG_FILE_NAME),
    };
    name.push(QUARANTINE_SUFFIX);
    path.with_file_name(name)
}

/// What [`quarantine`] found at the path it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quarantined {
    /// A file was there and is now at [`quarantine_path_for`], byte for byte.
    Moved,
    /// There was nothing to move. Nothing of the user's is at risk, so the caller may write.
    NothingThere,
}

/// Moves a configuration this build could not read out of the way, byte for byte.
///
/// **A rename and never a rewrite.** The point of the copy is the text a person typed, which
/// this build by definition failed to parse — so there is nothing "understood" to write out,
/// and writing out the part that did parse would be the very loss this exists to prevent. The
/// file is moved with its bytes untouched: not re-encoded, not re-serialised, not truncated.
///
/// A previous `.bad` is replaced: `fs::rename` overwrites an existing destination on Windows,
/// which is what keeps the count of copies at one.
///
/// # The two ways it ends
///
/// [`Quarantined::NothingThere`] is answered when the source is gone — the file was deleted
/// between the read and the save. Source and destination share a directory by construction, so
/// a `NotFound` can only be about the source, and a source that does not exist has no bytes to
/// lose: the caller is free to write.
///
/// Any other failure comes back as [`ConfigError::Io`] and is an instruction to the caller,
/// not a detail to log: the bytes on the disk are still the only copy, so nothing may be
/// written over them (NFR-13 — the result is acted on, and the direction it is acted on in is
/// the one that keeps the file).
pub fn quarantine(path: &Path) -> Result<Quarantined, ConfigError> {
    match fs::rename(path, quarantine_path_for(path)) {
        Ok(()) => Ok(Quarantined::Moved),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Quarantined::NothingThere),
        Err(err) => Err(ConfigError::Io(err)),
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
    if dialog_is_open() {
        return Ok(());
    }

    // The list view of the cycle lives in `comctl32`, whose window classes are registered on
    // demand. Without this the whole dialog fails to be created — one missing class takes the
    // template with it — so the result is examined (NFR-13) before anything is shown.
    ensure_list_view_class()?;

    // Raised here and lowered however this function leaves, panic on the way through
    // included — see [`DialogSession`].
    let _open = DialogSession::open();

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
            // FR-92а, task T-12-1: loaded before the window exists, because a resource does
            // not need one; shown on `WM_INITDIALOG`, which is where a window does.
            icon: CaptionIcons::load(),
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

/// Whether a settings dialog of FR-92 is on the screen of **this** thread — task T-13-14.
///
/// The published half of [`DIALOG_OPEN`], and the whole of what leaves this module about it:
/// one `bool`, read, never written. [`show_dialog`] uses it to refuse a second copy of the
/// window, and [`crate::tray`] uses it to refuse the two menu commands that edit the same
/// configuration the open dialog is editing — section 6.3 has one owner of the configuration,
/// and «Применить» replaces it wholesale from the copy the dialog was opened with, so a
/// suspension made from the tray meanwhile would be silently undone.
///
/// **SEC-05, R-20.** Nothing outside this process can move this: the flag is a thread-local of
/// the UI thread raised by [`DialogSession::open`] and by nothing else, so the decision the tray
/// takes is a decision by *published value* and not by anything a message carries. There is no
/// window message, no `wParam` and no pointer anywhere on this road.
pub fn dialog_is_open() -> bool {
    DIALOG_OPEN.with(Cell::get)
}

/// The open dialog of this thread, for as long as this value lives — task T-11-9, T-13-14.
///
/// The guard [`show_dialog`] has always had, lifted out of that function and given a name so
/// that it can also be the *only* way [`DIALOG_OPEN`] is ever raised. There is exactly one such
/// guard in the program and this is it: a second one would be a second answer to «идёт ли
/// правка настроек», and two answers to one question are how they start to differ.
///
/// Both thread-locals are cleared on the way out, however that way is taken — a normal return,
/// an early `?`, or a panic unwinding through the modal call. The program stays up after a
/// panic (FR-98, FR-99), and a flag left raised would mean the settings window could never be
/// opened again *and* that the two tray commands stayed dead for the rest of the session.
///
/// ⚠ Not re-entrant, and does not have to be: [`show_dialog`] refuses to open a second dialog
/// while [`dialog_is_open`] answers true, so at most one of these exists per thread at a time.
pub struct DialogSession;

impl DialogSession {
    /// Raises the flag and hands back the value whose [`Drop`] lowers it.
    pub fn open() -> Self {
        DIALOG_OPEN.with(|open| open.set(true));
        Self
    }
}

impl Drop for DialogSession {
    fn drop(&mut self) {
        DIALOG_OPEN.with(|open| open.set(false));
        // FR-92а, task T-11-9: the record must not outlive the dialog it names — see
        // `DIALOG_WINDOW`. Cleared on the same guard so that no exit path, panic included,
        // can leave a stale window behind.
        DIALOG_WINDOW.with(|window| window.set(0));
    }
}

/// Whether an «О программе» window of FR-92а is on the screen of **this** thread — task
/// T-13-17.
///
/// The published half of [`ABOUT_WINDOW`], and the whole of what leaves this module about
/// that window: one `bool`, read, never written. True from the `WM_INITDIALOG` of
/// [`about_proc`] until the guard of [`show_about_dialog`] clears the record, which is
/// exactly the stretch of time there is a window to tell anything to.
///
/// # One thread-local here and two next door
///
/// The settings dialog needs a flag *beside* its window record because [`dialog_is_open`]
/// has to answer «да» before the window exists — it refuses a second copy of the dialog and
/// locks two tray commands, and both decisions are taken while `DialogBoxParamW` is still
/// on its way in. Nothing asks that of this window: the whole of what anybody wants to know
/// about it is «есть ли кому отдать сообщение», and the record of the live window *is* that
/// answer. A second flag would be a second answer to one question, and [`DialogSession`]
/// says in as many words how two answers to one question start to differ.
///
/// **SEC-05, R-20.** Nothing outside this process can move this: the record is a
/// thread-local of the UI thread, written by the procedure of a window this program created
/// and cleared by the guard of the function that opened it. There is no window message, no
/// `wParam` and no pointer anywhere on this road.
///
/// ⚠ **Task T-13-20 must read this beside [`dialog_is_open`].** That task narrows the
/// reading of the `WM_SETTINGCHANGE` string to the case «есть кому отдать»; after this task
/// «кому отдать» is «диалог открыт **или** about жив», and a gate that asked only the first
/// would stop the about window from following the system theme again.
pub fn about_is_open() -> bool {
    ABOUT_WINDOW.with(Cell::get) != 0
}

/// The open «О программе» window of this thread, for as long as this value lives — FR-92а,
/// task T-13-17.
///
/// [`DialogSession`] in the window next door, and deliberately the same shape: created by
/// [`show_about_dialog`] before the modal call and dropped however that function leaves — a
/// normal return, the `-1` return of a dialog that could not be created, or a panic
/// unwinding through the call. The window itself is closed by four different roads — «ОК»,
/// `Esc`, the cross of the caption, and a failed `debug_assert` on the way out in Debug —
/// and not one of them is a place to clear a record from: a [`Drop`] on the frame that owns
/// the window is the one place that covers all four at once.
///
/// A record left behind would be worse than an empty one. [`on_system_theme_message`] posts
/// [`WM_APP_SYSTEM_THEME`] to whatever the record names, and Windows is free to have given
/// that handle to somebody else's window by then — the very reason the dialog's own record
/// is cleared on a guard rather than in a handler.
///
/// ⚠ Not re-entrant, and does not have to be: the window is modal, and the tray command
/// that opens it cannot run while the modal loop of this very window is up.
pub struct AboutSession;

impl AboutSession {
    /// Hands back the guard whose [`Drop`] clears the record.
    ///
    /// Nothing is recorded here: `DialogBoxParamW` has not been called yet and there is no
    /// window to name — [`AboutSession::record`] does that from `WM_INITDIALOG`. The guard
    /// is taken from *before* the modal call all the same, so that the exit paths which
    /// never reach a window are covered by the same `Drop` as the ones that do.
    pub fn open() -> Self {
        Self
    }

    /// Records the window this session names — called from the `WM_INITDIALOG` of
    /// [`about_proc`] and from nowhere else in the program.
    ///
    /// Public for the reason [`GLYPH_CHECK_CONTROLS`] is public: `tests\settings.rs`
    /// exercises the very guard the product uses, over the very record it writes, without a
    /// live window. It is an in-process Rust call and not a door SEC-05 speaks of — no
    /// window message reaches it, and nothing but this program links this library.
    pub fn record(hwnd: HWND) {
        ABOUT_WINDOW.with(|window| window.set(hwnd.0 as isize));
    }
}

impl Drop for AboutSession {
    fn drop(&mut self) {
        // FR-92а, task T-13-17: the record must not outlive the window it names — see
        // `ABOUT_WINDOW`. Cleared on the guard so that no exit path, panic included, can
        // leave a stale window behind.
        ABOUT_WINDOW.with(|window| window.set(0));
    }
}

thread_local! {
    /// Whether this thread already has a settings dialog on the screen — see [`show_dialog`].
    ///
    /// Raised by [`DialogSession::open`] and lowered by that value's [`Drop`], and by nothing
    /// else in the program; read through [`dialog_is_open`].
    static DIALOG_OPEN: Cell<bool> = const { Cell::new(false) };

    /// The window of that dialog while it is up, zero otherwise — FR-92а, task T-11-9.
    ///
    /// Recorded on `WM_INITDIALOG` and cleared by the guard of [`show_dialog`] however that
    /// function leaves, so a non-zero record always names the live modal dialog of this
    /// thread. Stored as the plain integer the handle is: `HWND` is a pointer type, and a
    /// thread-local wants a value. A thread-local like its neighbour, because the dialog
    /// belongs to the UI thread and to no other (section 6.1).
    static DIALOG_WINDOW: Cell<isize> = const { Cell::new(0) };

    /// The window of the «О программе» dialog while it is up, zero otherwise — FR-92а,
    /// task T-13-17.
    ///
    /// The neighbour of [`DIALOG_WINDOW`], on the same terms and for the same reasons:
    /// written on `WM_INITDIALOG` by [`AboutSession::record`], cleared by the guard of
    /// [`show_about_dialog`] however that function leaves, stored as the plain integer the
    /// handle is, and thread-local because the window belongs to the UI thread and to no
    /// other (section 6.1). Read through [`about_is_open`]; the half that posts to it is
    /// [`on_system_theme_message`], the same function that posts to its neighbour.
    static ABOUT_WINDOW: Cell<isize> = const { Cell::new(0) };
}

// ---------------------------------------------------------------------------------------
// The system theme changing under an open window — FR-92а, tasks T-11-9 and T-13-17
// ---------------------------------------------------------------------------------------

/// The one string of `WM_SETTINGCHANGE` this program reacts to — FR-92а, task T-11-9.
///
/// Windows broadcasts a `WM_SETTINGCHANGE` naming this word when the personalization switch
/// `AppsUseLightTheme` moves. SEC-05 allows exactly the comparison against it and forbids
/// using the message's content any further.
pub const IMMERSIVE_COLOR_SET: &str = "ImmersiveColorSet";

/// The windows' own «системная тема сменилась» nudge — FR-92а, tasks T-11-9 and T-13-17.
///
/// `WM_APP + 14`, the next free number of the program-wide row: `+ 1` is the wake-up of
/// [`crate::app`] and `+ 5` its configuration nudge, `+ 2` the tray callback, `+ 3` and
/// `+ 4` belong to [`crate::hook`], `+ 6`, `+ 7` and `+ 9` to [`crate::watchdog`], `+ 8` is
/// [`crate::switch::WM_APP_SWITCH`], `+ 10` and `+ 11` are [`crate::guard`]'s pair, and
/// `+ 12` and `+ 13` are [`crate::selection`]'s.
///
/// SEC-05: the message carries nothing and decides nothing. Both handlers of it — the one
/// in [`dialog_proc`] and, since task T-13-17, the one in [`about_proc`] — re-resolve the
/// palette out of this program's own setting and its own read of the system switch
/// ([`refresh_palette`], [`refresh_about_palette`]), and an unchanged resolution repaints
/// nothing — so a forged message buys the sender one reading of the personalization switch
/// and one comparison of two pointers, and at worst one repaint of our own window with the
/// palette it already ought to wear.
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
/// tasks T-11-9 and T-13-17.
///
/// `setting_string` is what the message named, already read within the bounds SEC-05
/// prescribes — the reading belongs to the module that owns the receiving window.
///
/// **Two windows of FR-92а can be on the screen, and this function is the one road to both
/// of them.** The settings dialog of FR-92 (task T-11-9) and the «О программе» window
/// (task T-13-17) are told in the same way, in this order, from this function alone: a
/// second delivery mechanism would be a second answer to «пора ли перекраситься», and the
/// audit that produced this task was about a window nobody told at all. Neither window
/// listens for `WM_SETTINGCHANGE` itself — the broadcast reaches the tray's hidden window,
/// which is the only window of this program that reads it (§6.2).
///
/// A window that is not up is the cheap exit and costs one thread-local read: it will
/// resolve the fresh system switch when it opens ([`show_dialog`] and
/// [`show_about_dialog`] both read it at that moment), so there is nothing to tell it now.
/// Otherwise the decision is [`repaint_for_system_theme`] over that window's own state, and
/// a yes is one `PostMessageW` of [`WM_APP_SYSTEM_THEME`] to it — posted, never sent, like
/// everything this program tells its own windows (the implication of FR-72).
///
/// A message arriving while a state is borrowed — a re-entrant dispatch inside a handler —
/// finds `with_state` / `with_about_state` answering `None` and does nothing; Windows sends
/// these in a batch, and a later arrival of the batch finds the borrow free.
pub fn on_system_theme_message(setting_string: Option<&str>) {
    // The settings dialog of FR-92 — task T-11-9.
    if let Some(hwnd) = live_window(&DIALOG_WINDOW) {
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
            post_system_theme(hwnd);
        }
    }

    // The «О программе» window of FR-92а — task T-13-17. The same three steps over the state
    // that window keeps: its own setting — the one the tray copied out of the configuration
    // when it opened the window — its own palette, and the same fresh reading of the system
    // switch. The dialog's own «Применить» has no counterpart here: this window changes
    // nothing (SEC-05), so the setting it was opened under cannot move under it.
    if let Some(hwnd) = live_window(&ABOUT_WINDOW) {
        // SAFETY: a non-zero record names the live modal about window of this thread —
        // written on `WM_INITDIALOG` by `AboutSession::record`, cleared however
        // `show_about_dialog` leaves — so `hwnd` is the window of a dialog created by
        // `show_about_dialog`, which is the contract of `with_about_state`.
        let repaint = unsafe {
            with_about_state(hwnd, |state| {
                repaint_for_system_theme(
                    setting_string,
                    state.setting,
                    state.palette,
                    theme::resolve(ThemeSetting::System, theme::system_is_light()),
                )
            })
        };

        if repaint == Some(true) {
            post_system_theme(hwnd);
        }
    }
}

/// The window a record names, or `None` when the record is empty — FR-92а, task T-13-17.
///
/// The two records of [`on_system_theme_message`] are read by one body rather than by two
/// copies of four lines. Zero is «нет окна» in both, and it is the value both are born with
/// and are returned to by their guards, so a `None` here is «этому окну сейчас нечего
/// сказать» and never «окно есть, но его номер потерян».
fn live_window(record: &'static std::thread::LocalKey<Cell<isize>>) -> Option<HWND> {
    let raw = record.with(Cell::get);

    if raw == 0 {
        return None;
    }

    Some(HWND(raw as *mut std::ffi::c_void))
}

/// Posts the empty [`WM_APP_SYSTEM_THEME`] nudge to one of this program's own windows —
/// FR-92а, task T-13-17: the one posting site both recipients share.
///
/// SEC-05: the message carries two plain zeros. Nothing of the `WM_SETTINGCHANGE` that
/// started this — no string, no pointer, no length — travels with it; the receiving handler
/// asks the system itself what the theme now is.
fn post_system_theme(hwnd: HWND) {
    // SAFETY: `hwnd` came from a live record of this thread — the settings dialog or the
    // about window, both of them windows this program created and neither of them able to
    // outlive its record; the message carries two plain zeros and no pointer, so
    // `PostMessageW` queues them by value and returns.
    if let Err(error) =
        unsafe { PostMessageW(Some(hwnd), WM_APP_SYSTEM_THEME, WPARAM(0), LPARAM(0)) }
    {
        // NFR-13. Not fatal: the palette catches up on the next occasion — the next message
        // of the batch, or a pressed «Применить», or the next opening of the window — and
        // the journal is told.
        crate::app::report_non_critical("PostMessageW", &error);
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
    /// The icon the caption is shown by `WM_SETICON` — task T-12-1.
    ///
    /// Owned here for exactly the reason [`CaptionIcons`] gives: the handles must outlive the
    /// window that displays them, and this value is dropped only after the modal call has
    /// returned. `None` — a refused `LoadImageW` — leaves the caption without an icon, which
    /// is the caption every task before this one had (NFR-13).
    icon: Option<CaptionIcons>,
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

            // FR-92а, task T-12-1: the icon of the caption, taken out of the state as two
            // plain handles and posted only after the borrow has ended — the discipline every
            // handler of this file keeps, whatever the message costs.
            //
            // SAFETY: as above — the pointer was stored just now and the value it names is
            // alive for the whole of this modal call.
            let frames =
                unsafe { with_state(hwnd, |state| state.icon.as_ref().map(CaptionIcons::frames)) };

            if let Some(Some(frames)) = frames {
                // SAFETY: `hwnd` is the live dialog, and the handles belong to the value the
                // state keeps, which is dropped only after this modal call returns.
                unsafe { CaptionIcons::show_on(hwnd, frames) };
            }

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
            // Task T-12-5: the far half of the second subclass pair, on the same terms and for
            // the same reason — the two lists are still live windows here.
            unsubclass_lists(hwnd);
            // Task T-12-8: and the third pair, the nine push buttons, on the very same terms.
            unsubclass_buttons(hwnd, &PUSH_BUTTONS);

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
/// `hwnd` its dialog procedure was called with, or the live window [`ABOUT_WINDOW`] records
/// (task T-13-17) — whose `GWLP_USERDATA` therefore holds either zero or the pointer that
/// function stored.
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

/// The colour roles of one button in one state — FR-92а, task T-11-5a, widened by the
/// response of task T-12-8; the pure half of `WM_DRAWITEM`, closed by a table test:
/// обычная/умолчательная × нормальная/**горячая**/нажатая/запрещённая.
///
/// The default button is recognised by its identifier — `IDOK`, the one control
/// `DM_SETDEFID` names on `WM_INITDIALOG` — not by `ODS_DEFAULT`, which the manager juggles
/// as the focus moves and which would make the accent wander off «ОК».
///
/// The order of the arms is the precedence:
/// - **запрещённость первой**: a disabled button takes no Enter and no click, so it must
///   not advertise itself — the accent face yields to the ordinary one and the ink goes
///   muted, whichever button it is («Выше»/«Ниже» are the ones actually seen grey). A
///   disabled button does **not** grow hot either: the cursor standing on it changes
///   nothing, because nothing is there to be clicked;
/// - **нажатие второй**: a held button shows the selection pair `sel_bg`/`sel_fg` of the
///   palette, default and ordinary alike — the accent yields for the length of the press,
///   and so does the hot face, which is what «нажата > горячая» means: a pressed button is
///   also hot (the cursor is on it while the button is held), and this arm standing above
///   the hot one is the whole of that precedence;
/// - **акцент третьим, и он сам отвечает на курсор** — task T-15-2. «ОК» is the accented
///   button, and the accent is what it wears both at rest and under the pointer; what the
///   pointer moves is *which* accent. At rest the face is `accent_bg`; under the pointer it
///   is `sel_fg`, and the ink stays `accent_fg` in both. Task T-12-8 had left this button
///   deaf on purpose — the palette holds no field called «the accent under the cursor» and
///   inventing a colour is forbidden — and the user reported the result as «только вспышка
///   и никакой подсветки». `sel_fg` is what answers it **out of the fields the palette
///   already has**: it is the one field of both palettes that moves the accent *the way an
///   ordinary button moves on hover* — lighter in «Графите» (228,231,234 → 240,242,244)
///   and darker in «Тумане» (43,47,54 → 35,38,43) — so the accent is strengthened rather
///   than lost, and the caption keeps the contrast `accent_fg` was chosen for. Nothing else
///   moves: the frame stays `button_border`, and the about window's «ОК» rides along by
///   construction through [`about_button_colors`];
/// - **горячая** — the cursor stands on an ordinary, enabled, unpressed button: the face
///   goes to `hover_bg` and **nothing else moves**. The frame stays `button_border` and the
///   caption stays `text`, exactly as at rest — the response is a change of ground, the way
///   the tray menu (task T-11-21) makes it. This arm stands **below** the accent one, which
///   is what keeps `hover_bg` off «ОК»: an almost-white button turning almost the colour of
///   the window is not a highlight, it is a disappearance.
///
/// Focus is deliberately absent here: `ODS_FOCUS` changes no colour — it adds the dotted
/// `DrawFocusRect` frame on top of whatever face this table chose, and that is the drawing
/// half's business. `hot` is not focus and does not pretend to be: the two are independent,
/// and a focused button under the cursor wears the hot face and the dotted frame at once.
pub fn button_color_roles(control: i32, hot: bool, pressed: bool, disabled: bool) -> ButtonColors {
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
            face: if hot {
                ButtonFaceRole::SelFg
            } else {
                ButtonFaceRole::AccentBg
            },
            text: ButtonTextRole::AccentFg,
            border,
        };
    }

    if hot {
        return ButtonColors {
            face: ButtonFaceRole::HoverBg,
            text: ButtonTextRole::Text,
            border,
        };
    }

    ButtonColors {
        face: ButtonFaceRole::ButtonBg,
        text: ButtonTextRole::Text,
        border,
    }
}

/// The colour roles of the one button of the **about** window — finding **A-13**, task T-12-4.
///
/// The table above and one difference: the frame. The mock-up's about window draws its «ОК»
/// with a fill and no outline (`chrome.ps1:200-202` fills a rounded rectangle and never
/// strokes it), while the settings dialog's buttons are drawn framed a few lines earlier in
/// the same generator — so the difference belongs to the *window*, not to the state of the
/// button, and this is a wrapper over [`button_color_roles`] rather than a seventh column in
/// it. Face and ink come from there unchanged, which is what keeps «заливка и чернила кнопки
/// прежние» true by construction: there is one table of them and this function does not touch
/// it.
///
/// Pure, so the table test can close it the way it closes the one it wraps.
pub fn about_button_colors(control: i32, hot: bool, pressed: bool, disabled: bool) -> ButtonColors {
    ButtonColors {
        border: ButtonBorderRole::FaceItself,
        ..button_color_roles(control, hot, pressed, disabled)
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

/// Asks DWM to colour the non-client title bar after the palette — FR-92а, tasks T-11-4 and
/// T-12-1.
///
/// Three attributes, all documented, all four bytes, all set through the one entry point
/// `DwmSetWindowAttribute` this file has always had:
///
/// * `DWMWA_USE_IMMERSIVE_DARK_MODE` — the `BOOL` of task T-11-4, true exactly when the
///   resolved palette is the dark one. It is what makes the *caption buttons* — the cross,
///   the minimise and the maximise — legible, and it is kept for that alone: the two
///   colours below leave those buttons to the system;
/// * `DWMWA_CAPTION_COLOR` := [`theme::Palette::title_bg`] — task T-12-1. Until this task
///   the fill was DWM's own, mixed from the user's accent colour: measured 34,29,40 in one
///   window of this program and 40,26,45 in the other, against the 26,29,34 of the palette;
/// * `DWMWA_TEXT_COLOR` := [`theme::Palette::title_fg`] — task T-12-1. Without it the
///   caption text stays the system's pure white (measured 255,255,255) or pure black over
///   a fill that is now ours, and the mock-ups' 232,234,236 / 35,38,43 never appears.
///
/// Both colours are `COLORREF` — `0x00BBGGRR`, the same byte order `theme` packs — and the
/// documented way back to the system's own choice is the value `DWMWA_COLOR_DEFAULT`
/// (`0xFFFFFFFF`), which this program never has cause to send: every window it opens is
/// opened in a palette.
///
/// ⚠ **Called at every palette change, not only at creation.** The four call sites are
/// `WM_INITDIALOG` of the settings dialog, `WM_INITDIALOG` of the about window,
/// [`refresh_palette`] — the common tail of a pressed «Применить» and of
/// `WM_SETTINGCHANGE` — and [`refresh_about_palette`], which is the same tail for the about
/// window (task T-13-17): one road to the caption for both windows, not a second one for
/// the newcomer. The attributes are properties of the window and survive nothing but
/// another call, so a palette change that skipped this would leave yesterday's caption over
/// today's client area.
///
/// A refusal is survived and left alone on purpose (NFR-13: every result is examined right
/// here and deliberately dropped). `DWMWA_USE_IMMERSIVE_DARK_MODE` took its public number
/// only in Windows 10 20H1 and the two colours only in Windows 11 build 22000, so on an
/// older build the call answers an error for a perfectly live window — a legal state of the
/// machine, not a violation of ownership, which is why there is no `debug_assert` on this
/// path. It is not journaled either: the closed `OPERATIONS` vocabulary of `diag` has no row
/// for it (decision of `reviews\T-11-1.md`), and the only consequence is a system-coloured
/// title bar over a correctly painted client area.
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

    set_caption_colour(hwnd, DWMWA_CAPTION_COLOR, palette.title_bg);
    set_caption_colour(hwnd, DWMWA_TEXT_COLOR, palette.title_fg);
}

/// Hands DWM one `COLORREF` attribute of the caption — task T-12-1, the body the two colour
/// lines of [`apply_title_bar_theme`] share.
///
/// The four bytes are passed as a plain `u32` and not as the `COLORREF` newtype: the
/// attribute is documented as «a `COLORREF` value», the size is the four bytes of the
/// number, and taking the number out of the wrapper here means the call cannot depend on
/// how the crate happens to lay that wrapper out.
fn set_caption_colour(hwnd: HWND, attribute: DWMWINDOWATTRIBUTE, colour: COLORREF) {
    let value: u32 = colour.0;

    // NFR-13: the result is examined here and deliberately dropped — see the caller for why
    // a refusal is a legal state of the machine and not a failure of this program.
    //
    // SAFETY: `hwnd` is the live window. The attribute pointer names `value`, a live local
    // of this frame, and the size passed is exactly its four bytes; the call copies the
    // value and keeps no pointer once it returns.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            attribute,
            (&raw const value).cast(),
            size_of::<u32>() as u32,
        )
    };
}

// =========================================================================================
// FR-92а, task T-12-1 — the icon of the caption
// =========================================================================================

/// The identifier of the program's icon in `app.rc` — line 221, `IDI_APP_ACTIVE ICON`.
///
/// ⚠ Kept equal to the `.rc` by hand, exactly as [`IDD_SETTINGS`] is: a `.rc` file and a
/// Rust file have no vocabulary in common. It is deliberately the same number the private
/// constant of the same name in `crate::tray` carries — one resource, loaded by the
/// notification area for its icon and by this file for the caption of two windows — and
/// `app.rc` is not this task's to change.
const IDI_APP_ACTIVE: u16 = 101;

/// The two icons one window's caption is shown by `WM_SETICON`, owned by this value and
/// destroyed when the window that was shown them is gone — task T-12-1.
///
/// # Who owns the handles
///
/// **This value, and nothing else.** `WM_SETICON` does not transfer ownership: it hands the
/// window a handle to *display*, and Learn is explicit that the caller is the one who
/// destroys it. Neither icon is loaded with `LR_SHARED`, so each is a fresh handle this
/// process owns and has to free — the same choice, and for the same documented reason, that
/// `crate::tray::Icon` makes for the notification-area icon: `LR_SHARED` may only be used
/// for an image loaded at the size the resource already holds, and both sizes here are asked
/// for by number, so the loader scales a frame and the result belongs to the caller.
///
/// # When they are freed
///
/// After the window is destroyed and not before: the value lives in the state on the frame
/// of [`show_dialog`] / [`show_about_dialog`], and `DialogBoxParamW` is modal — it returns
/// only once the window is gone, and the state is dropped after it returns. Destroying an
/// icon a live caption is still displaying is exactly the bug this ordering rules out.
struct CaptionIcons {
    /// The frame the caption itself shows — `ICON_SMALL`, asked for at the system's
    /// small-icon metric.
    small: HICON,
    /// The frame Alt+Tab and the task bar show — `ICON_BIG`, asked for at the system's
    /// ordinary icon metric.
    big: HICON,
}

impl CaptionIcons {
    /// Loads both frames out of the resources the interface strings come from.
    ///
    /// `None` on a refusal of either `LoadImageW` (NFR-13): a window without an icon is the
    /// window this program had before this task, which is a smaller loss than a half-loaded
    /// pair — and the first handle of a half-loaded pair is freed right here rather than
    /// leaked, because nothing else would ever come to own it.
    ///
    /// A refusal is deliberately **not** journaled, for the reason [`theme::Brushes::new`]
    /// gives for its own: the `OPERATIONS` vocabulary of module `diag` is closed
    /// (`reviews\T-11-1.md`), it has no row for loading a picture, and adding one belongs to
    /// the module that owns the table and not to this one. NFR-13 is satisfied where it asks
    /// to be — the answer is examined, the half-built pair is unwound, and the caller sees
    /// `None`.
    fn load() -> Option<Self> {
        let module = resource_module();
        let instance = HINSTANCE(module.0);

        // The caption shows the small frame, and «small» is a system metric, not sixteen:
        // the answer follows the display scale. `crate::tray` asks the same question for the
        // notification area and its function is the one asked here, rather than a second
        // copy of the same `GetSystemMetrics` pair (§6.2).
        let (small_cx, small_cy) = crate::tray::small_icon_size();

        // SAFETY: `instance` is the module handle whose resources carry `IDI_APP_ACTIVE`
        // — the very module the interface strings are read from — and the "name" is an
        // integer identifier in the `MAKEINTRESOURCE` form, so nothing is dereferenced as a
        // string. Without `LR_SHARED` the call makes a handle this value owns; the crate
        // turns a null result into an error, so NFR-13 is satisfied by the `ok()?`.
        let small = HICON(
            unsafe {
                LoadImageW(
                    Some(instance),
                    resource_id(IDI_APP_ACTIVE),
                    IMAGE_ICON,
                    small_cx,
                    small_cy,
                    LR_DEFAULTCOLOR,
                )
            }
            .ok()?
            .0,
        );

        // `LR_DEFAULTSIZE` with a zero width and height is the documented way to ask for the
        // system's icon metric — the same question `SM_CXICON`/`SM_CYICON` answer, asked of
        // the loader instead of being fetched and passed back in.
        //
        // SAFETY: as above.
        let big = unsafe {
            LoadImageW(
                Some(instance),
                resource_id(IDI_APP_ACTIVE),
                IMAGE_ICON,
                0,
                0,
                LR_DEFAULTCOLOR | LR_DEFAULTSIZE,
            )
        };

        // NFR-13: examined. The frame already loaded is freed right here — a half-built pair
        // is nobody's property, so nothing else would ever come to free it.
        let Ok(big) = big else {
            // SAFETY: `small` came from the successful call above, was handed to nobody, and
            // this is the one place that frees it — no `Drop` of `CaptionIcons` will ever run
            // for a value that is never built.
            if let Err(error) = unsafe { DestroyIcon(small) } {
                crate::app::report_non_critical("DestroyIcon", &error);
            }

            return None;
        };

        Some(Self {
            small,
            big: HICON(big.0),
        })
    }

    /// The two frames as plain values — what a handler takes out of the state before it lets
    /// the borrow go.
    ///
    /// The split every handler of this file makes: the borrow of the state ends before any
    /// message is put anywhere, so no message can find the state busy underneath. The handles
    /// outlive the borrow — they belong to the value the state keeps, which is dropped only
    /// after the modal call returns.
    fn frames(&self) -> [(u32, HICON); 2] {
        [(ICON_SMALL, self.small), (ICON_BIG, self.big)]
    }

    /// Shows both frames on `hwnd` — the two `WM_SETICON` messages of task T-12-1.
    ///
    /// ⚠ **Posted, not sent** — FR-72, the same choice and the same reason as the `DM_SETDEFID`
    /// of `WM_INITDIALOG` a few lines above each call site: this program uses no bare
    /// `SendMessage` anywhere, and there is nothing here to wait for. The window picks the two
    /// messages up in the modal loop it is about to enter, `DefDlgProc` hands them on to
    /// `DefWindowProc`, and the caption is drawn with the icon from its first paint on.
    ///
    /// The answer of `WM_SETICON` — the handle the window displayed *before* — is therefore
    /// not seen at all, and nothing is lost by that: it is null in both windows, because
    /// neither template declares an icon and this is the first and only sender. A value that
    /// was never ours to free would not become ours by arriving in a return value.
    ///
    /// # Safety
    ///
    /// `hwnd` is a live window of this thread, and `frames` came from a [`CaptionIcons`]
    /// somebody keeps alive for longer than that window.
    unsafe fn show_on(hwnd: HWND, frames: [(u32, HICON); 2]) {
        for (which, icon) in frames {
            // SAFETY: see the contract above. `wparam` is a plain number and `lparam` carries
            // an icon handle by value — `PostMessageW` queues the two words and returns, and
            // no memory of ours is dereferenced by either end.
            if let Err(error) = unsafe {
                PostMessageW(
                    Some(hwnd),
                    WM_SETICON,
                    WPARAM(which as usize),
                    LPARAM(icon.0 as isize),
                )
            } {
                // NFR-13. Not fatal: the caption keeps the icon it had, which is none, and
                // the journal is told — the same name, and the same handling, the refused
                // `DM_SETDEFID` of this file already carries.
                crate::app::report_non_critical("PostMessageW", &error);
            }
        }
    }
}

impl Drop for CaptionIcons {
    fn drop(&mut self) {
        for icon in [self.small, self.big] {
            // SAFETY: each handle came from a successful `LoadImageW` without `LR_SHARED`,
            // so it is ours to destroy, and it is destroyed exactly once — the type is
            // neither `Copy` nor `Clone`, its fields are private and never reassigned, and
            // `drop` runs once. The window that displayed them is already destroyed: this
            // value lives on the frame of the modal call and is dropped after it returns.
            if let Err(error) = unsafe { DestroyIcon(icon) } {
                crate::app::report_non_critical("DestroyIcon", &error);
            }
        }
    }
}

/// Repaints a window of this module and every child in it — the visible half of a palette
/// change. Called by [`refresh_palette`] for the settings dialog and by
/// [`refresh_about_palette`] for the about window (task T-13-17): one body, two windows.
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

/// Air added to the dialog font's height to make one item of a **dropped-down list**, in the
/// pixels of the mock-ups — the number task T-11-15 wrote, kept here for the list alone.
///
/// ⚠ **This number is not a length of the mock-ups and never was.** It was derived by measuring
/// the picture — the closed part of a combo was read off `ui-02-graphite.png` as a box of its
/// own pixels and the font subtracted from it — and the measurement was wrong: the generator
/// states the whole box as `h = 12` dialog units (`scratchpad\ui.ps1`, lines 103, 116 and 118),
/// which is 22,5 px at 96 DPI, and carries no literal for «air» at all. Task T-12-2 corrected
/// the closed part to that literal in [`COMBO_CLOSED_ITEM_EXTRA`]; the height of the **items of
/// the dropped-down list** was never measured against the mock-ups by anyone, and T-12-2 is
/// forbidden to move what it has not measured — so the list keeps exactly the height it has
/// had since T-11-15, and this constant is what keeps it: 15 + [`scaled`]`(12)` = 15 + 9 =
/// **24 px** at 96 DPI, the very number `CB_GETITEMHEIGHT(0)` answered before that task.
///
/// The rows of the exclusion list are **not** measured by it — they have had
/// [`EXCLUSION_ROW_HEIGHT_DLU`] of their own since task T-11-16.
pub const COMBO_LIST_ITEM_EXTRA: i32 = 12;

/// Air added to the dialog font's height to make the **closed part** of a combo box — the
/// selection field — in the pixels of the mock-ups; task T-12-2, finding F3/R-04 of the
/// Э12 protocol.
///
/// # Where the 2 comes from — the literal of the generator, not the picture
///
/// `scratchpad\ui.ps1` gives every combo box `h = 12` dialog units (lines 103, 116, 118) and
/// its `'combo'` arm draws the box exactly that tall. Twelve vertical dialog units are
/// 12 × 1,875 = **22,5 px at 96 DPI** — the whole closed part of the mock-up, frame included.
///
/// Six of those pixels are not ours to spend. The system lays the selection field inside the
/// control with two pixels of its own edge above and below it (4 px), and the frame this
/// dialog draws takes the top and the bottom row of the client area (2 px) — measured on the
/// stand and not assumed: window 30 px = `CB_GETITEMHEIGHT(-1)` 24 + 6, with 28 px of fill.
/// So the selection field itself must stand 22,5 − 6 = **16,5 px**, and the dialog font is
/// 15 px there: the air is **1,5 px at 96 DPI**, which is 1,5 × 1,4 = **2,1 pixels of the
/// mock-ups**, and 2 is that number written whole.
///
/// Through [`scaled`] the 2 comes back as 1 px at 96 DPI (2 × 5/7 = 1,43 → 1), so the field
/// stands 15 + 1 = 16 px and the closed part 16 + 6 = **22 px** against the mock-up's 22,5 —
/// half a pixel below it, where the whole number above would have put it half a pixel above.
/// The air travels with the DPI of the window because it goes through [`scaled`], which is the
/// only road a mock-up length takes into this file.
///
/// **Why a second constant and not a smaller [`COMBO_LIST_ITEM_EXTRA`].** `WM_MEASUREITEM`
/// arrives **once** for a `CBS_OWNERDRAWFIXED` combo and sets one height for both the closed
/// part and the items of the dropped-down list — measured, not read: before this task
/// `CB_GETITEMHEIGHT(-1)` and `CB_GETITEMHEIGHT(0)` both answered 24 on all four combo boxes.
/// The height of the list items is not the subject of T-12-2, so the two are set apart by the
/// documented lever instead — [`set_combo_closed_height`] sends `CB_SETITEMHEIGHT` with
/// `wParam = -1`, which names the selection field alone.
pub const COMBO_CLOSED_ITEM_EXTRA: i32 = 2;

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
/// The item height is [`combo_row_height`] with [`COMBO_LIST_ITEM_EXTRA`] mock-up pixels of
/// air — the dialog font's height plus that air through [`scaled`] at the DPI of the dialog's
/// own DC. The message arrives while the combo is being
/// created — *before* `WM_INITDIALOG`, so
/// before the dialog's state exists — but after the dialog has taken its `DS_SETFONT`
/// font, which the dialog manager passes to the dialog before creating any control. The
/// font's height is read through the documented `MapDialogRect`: the vertical dialog base
/// unit is defined as the height of the dialog font and one vertical dialog unit as one
/// eighth of it, so a rectangle 8 units tall maps to exactly one font height in pixels —
/// no font handle changes hands and no message is sent.
///
/// ⚠ **What this height is, since task T-12-2, and what it is not.** One `WM_MEASUREITEM`
/// reaches a `CBS_OWNERDRAWFIXED` combo box, and the height it answers is worn by both the
/// closed part and every item of the dropped-down list. T-12-2 brought the *closed* part to
/// the 12 dialog units of the mock-ups and left the *list* where it was, so what is answered
/// here is the height of a **list item**, and [`set_combo_closed_height`] overrides the closed
/// part afterwards with `CB_SETITEMHEIGHT`. Answering the closed part's smaller height here
/// instead would shrink the rows of the dropped-down list with it — the thing that task was
/// told not to touch.
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
    // its `'lbox'` rows `Y 11` — [`EXCLUSION_ROW_HEIGHT_DLU`] vertical dialog units. The
    // generator states a combo box as `h = 12` units, and that is the CLOSED part, which
    // [`set_combo_closed_height`] takes care of since task T-12-2; the dropped-down list it
    // says nothing about, so its items keep the font-plus-air of T-11-15.
    let measured = if item.CtlType.0 == ODT_LISTBOX.0 {
        dialog_units(hwnd, 0, EXCLUSION_ROW_HEIGHT_DLU).map(|(_, row)| row)
    } else {
        // The rows of the dropped-down list, and only they, since task T-12-2 — see the ⚠ of
        // the doc comment above.
        combo_row_height(hwnd, COMBO_LIST_ITEM_EXTRA)
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

/// One row of a combo box: the dialog font's height plus `air` **mock-up** pixels of it, in the
/// pixels of `hwnd`'s own monitor — the arithmetic [`on_measure_item`] and
/// [`set_combo_closed_height`] share, written once (§6.2) rather than twice.
///
/// The font height comes through the documented `MapDialogRect`: eight vertical dialog units
/// are one dialog-font height by the definition of the base units, so a rectangle
/// [`DIALOG_FONT_HEIGHT_DLU`] units tall maps to exactly that height in pixels — no font handle
/// changes hands and no message is sent. The air is a length of the mock-ups and travels
/// through [`scaled`], as every length of the mock-ups in this file does.
///
/// `None` for a refused `MapDialogRect` (NFR-13: examined; both callers then keep the height
/// the control had). A refused `GetDC` is survived one level down: [`dc_dpi`] answers 96 for a
/// DC that will not say, which is the 100 % air and a legal height.
fn combo_row_height(hwnd: HWND, air: i32) -> Option<i32> {
    let (_, font_height) = dialog_units(hwnd, 0, DIALOG_FONT_HEIGHT_DLU)?;

    // SAFETY: `hwnd` is the dialog — a live window either being built (`WM_MEASUREITEM`) or
    // filled in (`WM_INITDIALOG`); the call answers its DC or an invalid handle, and the DC is
    // released below on both paths.
    let dc = unsafe { GetDC(Some(hwnd)) };

    // NFR-13: examined — see the doc comment above.
    let extra = scaled(air, dc_dpi(dc));

    if !dc.is_invalid() {
        // SAFETY: releases exactly the DC taken above, once.
        unsafe { ReleaseDC(Some(hwnd), dc) };
    }

    Some(font_height + extra)
}

/// The `wParam` of `CB_SETITEMHEIGHT` and `CB_GETITEMHEIGHT` that names the **selection
/// field** — the closed part — instead of an item of the list: the documented −1, spelled as
/// the unsigned word a message parameter is, the way `CB_SETCURSEL` is already handed its
/// «nothing chosen» in [`select_layout`].
const CB_SELECTION_FIELD: usize = usize::MAX;

/// Brings the closed part of the four combo boxes to the 12 dialog units of the mock-ups —
/// FR-92, FR-92а, task T-12-2 (findings F3 and R-04 of the Э12 protocol).
///
/// # Why this and not a smaller item height
///
/// `WM_MEASUREITEM` reaches a `CBS_OWNERDRAWFIXED` combo box **once**, and the height it
/// answers is worn by the closed part and by every item of the dropped-down list alike — read
/// off the live stand rather than out of the documentation: before this task
/// `CB_GETITEMHEIGHT(-1)` (the selection field) and `CB_GETITEMHEIGHT(0)` (one list item) both
/// answered **24** on all four combo boxes, and the closed part measured 30 px on the screen,
/// which is that 24 plus the six pixels the system and our own frame take. Shrinking the
/// answer of [`on_measure_item`] would therefore have shrunk the rows of the dropped-down list
/// by the same amount — and their height is not what task T-12-2 measured or was told to
/// change. `CB_SETITEMHEIGHT` with `wParam = -1` is the documented lever that moves the
/// selection field alone, so the two heights are set apart here and the list keeps its own.
///
/// # The height
///
/// [`combo_row_height`] with [`COMBO_CLOSED_ITEM_EXTRA`] — the font plus the air the generator's
/// `h = 12` dialog units leave once the six pixels the system spends are taken off; the
/// derivation is written out at that constant. At 96 DPI: 15 + 1 = 16, and the closed part
/// stands 16 + 6 = 22 px against the mock-up's 22,5.
///
/// # NFR-13
///
/// Two refusals are possible and both are examined here. A refused `MapDialogRect` gives no
/// height at all — nothing is sent, and the four combo boxes keep the height
/// [`on_measure_item`] gave them, which is the look this dialog had before this task: taller
/// than the mock-up, and alive. `CB_SETITEMHEIGHT` answers `CB_ERR` (−1) for a height it will
/// not take; the answer is looked at for every combo box and counted, and a combo box that
/// refused simply keeps its previous, taller closed part — the other three still take theirs.
/// The refusal is deliberately **not** journaled, for the reason [`subclass_combo_boxes`]
/// gives at its own dropped `BOOL`: the operation vocabulary of `diag` is closed
/// (reviews\T-11-1.md), `src\diag.rs` is outside this task, and a combo box that stayed
/// 30 px tall is a visible degradation rather than a silent one.
///
/// Answers how many of the four took the height — for the tests, and for the sentence above to
/// be a measurement and not a hope.
fn set_combo_closed_height(hwnd: HWND) -> usize {
    let Some(height) = combo_row_height(hwnd, COMBO_CLOSED_ITEM_EXTRA) else {
        return 0;
    };

    let mut taken = 0;

    for control in COMBO_BOXES {
        // `wParam` names the selection field; `lParam` is the height it is to stand.
        let answer = send_to(
            hwnd,
            control,
            CB_SETITEMHEIGHT,
            CB_SELECTION_FIELD,
            isize::try_from(height).unwrap_or(0),
        );

        // NFR-13: examined right here — see the doc comment above.
        if answer == CB_ERR as isize {
            continue;
        }

        taken += 1;
    }

    taken
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
/// Out of that structure exactly four fields are read — `dwDrawStage`, `dwItemSpec`,
/// `uItemState`, `hdc` — and three written — `clrText`, `clrTextBk`, `uItemState` — the
/// documented answer protocol of item prepaint; the `hdc` is the DC of the paint the
/// notification is inside of and is painted into, **the rectangle of the message is not read
/// at all** (a list view never fills it — see below), and no pointer of the message is
/// followed. Neither is anything else taken from the message: whether the row is selected,
/// whether its tick is set and where the row lies are all the list's own answers —
/// `LVM_GETITEMSTATE` twice and `LVM_GETITEMRECT` through [`row_bounds`], asked by identifier
/// through [`send_to`], the reading [`read_cycle_checks`] already does for the
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
/// and the paints of this handler lose to it. Clearing the bit in the item-prepaint answer is
/// the documented custom-draw lever for exactly this — the control then draws the row as an
/// ordinary one; the row still *is* selected (`LVIS_SELECTED` is untouched,
/// `LVS_SHOWSELALWAYS` stays a style of the template). The answer is
/// `CDRF_DODEFAULT` — «draw it yourself, with the fields I set»; `CDRF_NEWFONT` is the
/// answer of a handler that swapped a font into the `hdc`, which this one never does.
/// Chosen by the documentation — the product is not launched by the executor; the eye check
/// is the controller's, at the final acceptance.
///
/// # Why the ground stopped being a colour field — task T-11-25
///
/// Only the **ink** of the table reaches the message now (`clrText`). The ground reaches the
/// window instead, through [`draw_cycle_row`], because the mock-ups do not fill the row: they
/// inset a rounded stripe into it, and a rectangle of `clrTextBk` cannot have a corner radius
/// any more than the `FrameRect` of a check-box glyph could. The lever that lets a handler paint
/// under a control's own drawing is the documented transparent text background — `CLR_NONE`,
/// sent to the whole control by [`paint_cycle_list`] and written into `clrTextBk` here as well:
/// with it the control fills no rectangle at all and only lays the letters down, over whatever
/// this handler has painted a moment before.
///
/// The tick travels the same road and for a measured reason. It used to be a picture in the
/// state image list, and the edge of that picture could not be smoothed: `ImageList_AddMasked`
/// makes a hole of a pixel that is **exactly** the key colour, so a half-covered edge pixel — a
/// blend of the key and the fill — would have shown as a coloured fringe on every row. The
/// documented replacement the task proposed, an `ILC_COLOR32` list without `ILC_MASK` carrying
/// per-pixel alpha, was measured on this machine and **does not blend**: the comctl32 this
/// program runs on is 5.82 (no `Microsoft.Windows.Common-Controls` v6 assembly in either
/// manifest), its `ImageList_Draw` copies the image opaquely and the alpha byte is ignored — the
/// air came out black rather than transparent. So the cell of the image list is now nothing but
/// the key colour — a hole edge to edge, kept for the row height and the state-icon column it
/// still measures — and the tick itself is drawn on the row by [`draw_cycle_row`], smoothed like
/// every other figure of this dialog and blended into whichever ground the row actually wears.
///
/// # Where the row is — task T-11-25-2
///
/// Both of those paints need a rectangle, and the rectangle the message appears to offer —
/// `NMCUSTOMDRAW::rc` — is the one thing a list view does **not** fill at item prepaint. It
/// arrives `{0, 0, 0, 0}`, measured on the live control; T-11-25 painted into it and so painted
/// into nothing: no ground, no stripe, and every row's tick on the same nine scan lines at the
/// top-left corner of the list, one glyph over the other. The rectangle is therefore asked of
/// the control, by [`row_bounds`], exactly as the two state readings above are — and for the
/// same reason.
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

    // And whether its tick is set — the state image index of FR-31, read exactly the way
    // [`read_cycle_checks`] reads it. The tick is drawn by this handler since task T-11-25, so
    // the bits that used to pick a picture out of the image list now pick a picture to paint.
    let checked = u32::try_from(send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_GETITEMSTATE,
        draw.nmcd.dwItemSpec,
        isize::try_from(LVIS_STATEIMAGEMASK.0).unwrap_or(0),
    ))
    .unwrap_or(0)
        == CHECKED_IMAGE;

    // And **where** the row is — asked of the list, never taken from the message. See the
    // doc comment: a list view leaves `NMCUSTOMDRAW::rc` empty at item prepaint, so the
    // documented request is [`row_bounds`]. FALSE means the control does not place the row it
    // is asking about, and then there is nowhere honest to paint (NFR-13): the row keeps the
    // `LVM_SETBKCOLOR` erase and the ink of the table, and loses its stripe and its tick for
    // that repaint rather than taking them somewhere invented.
    let bounds = row_bounds(hwnd, draw.nmcd.dwItemSpec);

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

    if let Some((_, ink)) = paint.colours {
        draw.clrText = ink;
    }

    // The ground of the row is **not** written into the structure any more (task T-11-25): the
    // one value that goes there is `CLR_NONE`, the documented transparent text background, and
    // the ground itself is the figure [`draw_cycle_row`] paints below. Written here as well as
    // sent to the whole control by [`paint_cycle_list`], because the structure arrives carrying
    // the control's own colours and this handler is the one place that could put an opaque one
    // back by accident.
    draw.clrTextBk = COLORREF(CLR_NONE as u32);

    // The ground, the selection figure on it and the tick — everything the control no longer
    // draws for itself. Here and not after the answer, so that what the control still draws
    // (the label, and a state image that is now a hole edge to edge) lands on top of it.
    //
    // `draw.nmcd.hdc` is the DC of the paint this notification is inside of; it belongs to the
    // sender for the length of the send, and this procedure is inside that send. The rectangle
    // is `bounds`, the control's own answer — **not** `draw.nmcd.rc`, which a list view never
    // fills. SEC-05: the DC is painted into and no pointer of the message is followed.
    if let Some(row) = bounds {
        draw_cycle_row(
            draw.nmcd.hdc,
            &row,
            mode,
            selected,
            checked,
            palette,
            dc_dpi(draw.nmcd.hdc),
        );
    }

    if paint.strip_selected {
        // The lever of the doc comment: without `CDIS_SELECTED` the control paints the row as
        // ordinary — no system highlight over the selection figure just drawn, and the ink of
        // the table instead of the system's.
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

/// The rectangle of one row of the layout list, in the client pixels its own paint DC is in —
/// task T-11-25-2.
///
/// # Why this is asked and not taken from the message
///
/// Because the message does not carry it. `NMCUSTOMDRAW` has an `rc` field, and for a list
/// view it is **not** filled at the item prepaint stage: measured on the live control through
/// the stand, every row of the layout list arrives with `rc` = `{0, 0, 0, 0}`. Painting into
/// that rectangle is painting into nothing at the top-left corner of the list — an empty
/// `FillRect`, an empty selection stripe, and every row's tick stacked on the same nine
/// scan lines above the first row, which is exactly the regression this task repairs.
///
/// The documented request for the geometry is `LVM_GETITEMRECT`: the **left** field of the
/// rectangle carries the code on the way in, the whole rectangle comes back in client
/// coordinates on the way out, and `LVIR_BOUNDS` is the code for «the entire item» — in a
/// report list view, the row across every column, from the state image to the end of the
/// last one. Client coordinates are the coordinates of the DC the control hands this
/// handler, so the answer needs no mapping.
///
/// `None` is FALSE from the control — an index it does not place. The callers' answer to it
/// is NFR-13 degradation, stated where they use it.
fn row_bounds(hwnd: HWND, item: usize) -> Option<RECT> {
    let mut rect = RECT {
        // The one field that is an argument rather than an answer.
        left: i32::try_from(LVIR_BOUNDS).unwrap_or(0),
        ..Default::default()
    };

    // SAFETY: `rect` is a fully initialised `RECT` owned by this frame; the message writes
    // through the pointer for the length of the send and keeps nothing. `LVM_GETITEMRECT`
    // takes the item index in `wParam` and the rectangle in `lParam`, which is what is passed.
    let answered = send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_GETITEMRECT,
        item,
        std::ptr::from_mut(&mut rect) as isize,
    );

    (answered != 0).then_some(rect)
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

    // FR-92а, task T-12-8: the fourth state, and the one the message does not carry — Windows
    // has no `ODS_HOTLIGHT` for an owner-drawn button. It is this program's own record, kept by
    // [`button_proc`], and the window of the message is **compared** against it and never
    // followed, exactly as the combo and list branches above compare theirs (SEC-05). A forged
    // message naming the control the cursor really is on buys the sender the face that control
    // is already wearing.
    let hot = is_hot(item_window);

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

            // The ground the button stands on, and what the corners the rounding cuts away
            // are erased with — the very rule this dialog answers `WM_CTLCOLORBTN` with
            // (task T-12-6; `draw_combo_closed_part` asks the same question for its own
            // corners). «Задать», «Удалить», «Добавить», «Выше», «Ниже» and «Открыть папку
            // журнала» lie on a panel; the three buttons of the bottom row do not.
            let ground = if state.panel_children.contains(&control) {
                brushes.panel_bg()
            } else {
                brushes.window_bg()
            };

            let (colors, hot_brush) = resolve_button_colors(
                button_color_roles(control, hot, pressed, disabled),
                ground,
                brushes,
                state.palette,
            );

            Some((
                colors,
                hot_brush,
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    // `_hot_brush` is named and not discarded on purpose: it owns the transient brush the hot
    // face is filled with, and a `_` would have freed it before the first pixel was drawn.
    let Some(Some((colors, _hot_brush, face))) = choice else {
        return 0;
    };

    // SAFETY: see the caller — `dc` and `rect` are the values of the message, used only
    // to paint into for the length of this send; `face` is a face the state owns for longer.
    unsafe { paint_push_button(hwnd, control, dc, rect, colors, focused, face) }
}

/// Paints one owner-drawn push button: the ground of the whole rectangle, the rounded face
/// under its single-pixel frame, the caption and the focus cue. The ground is erased first
/// because an owner-drawn control is responsible for the whole of its rectangle and a repaint
/// has to be idempotent — task T-12-6, and see the fill itself for the measurement.
///
/// The drawing half [`on_draw_item`] and [`on_about_draw_item`]
/// share — one body, moved out of `on_draw_item` by task T-11-11 rather than copied
/// (§6.2). Sharing it is why the «ОК» of the about window rounds off with the rest.
///
/// The caption comes from the dialog's own control by identifier — never from the message.
///
/// # Один кадр, а не череда — task T-15-1
///
/// ⚠ The steps below are drawn into an off-screen [`theme::PaintBuffer`] and handed to the
/// window in a single `BitBlt` at the end. They are the same steps in the same order as before
/// that task; what changed is only the DC they write into, and why they had to stop writing
/// into the window's own is the defect the **user** found — «я на мгновенье вижу впавшие
/// прямоугольники по краям кнопок». A body that paints a control call by call is sampled by DWM
/// between the calls: measured on the стенд, an intermediate state reached the screen in **100 %
/// of hover transitions**, the face already laid down and the four corners still bare ground.
///
/// A refused buffer (NFR-13) is not a refused button: `target` is then the DC of the message
/// itself and this body paints exactly as it did before the task, flicker included, which is the
/// degradation [`BackgroundCache`] answers a refusal of GDI with.
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

    let dpi = dc_dpi(dc);

    // ⚠ **Один кадр вместо череды — task T-15-1.** The surface the steps below are drawn onto.
    // Every one of them used to write straight into the DC of the message, and DWM samples the
    // surface of a window sixty times a second: on the стенд an intermediate state reached the
    // screen in **100 % of hover transitions**. Off-screen there is no screen for one to appear
    // on, and the finished button crosses over in the single blit at the foot of this body.
    //
    // `None` — GDI refused the surface (NFR-13). `target` is then the DC of the message itself
    // and every step below paints where it painted before this task, flicker and all: the honest
    // degradation, and the one [`BackgroundCache`] answers a refusal with.
    //
    // ⚠ `dpi` is taken one line above off the DC of the **message** and never off the buffer.
    // `GetDeviceCaps` of a memory surface answers for the memory and not for the window — the
    // very reason the rounded figure is handed its DPI instead of reading it — and a radius
    // scaled off the buffer would be wrong at every scale but 100 % with no test going red.
    //
    // SAFETY: `dc` is the DC of the message the caller is inside of; the buffer reads it for its
    // colour depth and its face, writes to it only in the blit below, and frees its own DC and
    // bitmap when this frame ends.
    let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };
    let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);

    // ⚠ **The ground, before the face — task T-12-6.** The corners the rounding cuts away used
    // to be left standing on whatever `WM_CTLCOLORBTN` had erased with, and the measured answer
    // is that the system erases nothing before it hands an owner-drawn button its drawing: a
    // button repainted **over itself** — the focus arriving on a click is the ordinary case —
    // laid a second antialiased arc over the first and its corners darkened towards the frame
    // ink (12 px of the stand's «Задать», 62,67,76 → 63,69,78) with no state of the button
    // having changed. One `FillRect` of the very brush that message answers makes the drawing
    // idempotent, which is what a repaint has to be; it is the same erase
    // [`draw_combo_closed_part`] does for the closed part of a combo box.
    //
    // ⚠ **And since task T-15-1 the erase carries a second duty: it is what grounds the buffer.**
    // The corner smoothing reads the ground of each corner back **out of** the DC it draws into,
    // and the pixels of a fresh `CreateCompatibleBitmap` are undefined — so this fill, standing
    // exactly where it stood, is what makes the buffer hold under the corners the same colour
    // the screen held there, and the halftones of the arcs come out unmoved. Moving this line
    // after the figure would not merely undo T-12-6, it would change the picture.
    //
    // NFR-13: the answer is dropped with the rest of the paint calls, for the reason the block
    // comment above gives.
    //
    // SAFETY: `target` is the buffer of this frame or, on a refusal, the DC of the message;
    // `rect` is a value of the message, and `colors.ground` is a live brush the window's state
    // owns for longer than this call.
    unsafe { FillRect(target, &rect, colors.ground) };

    // The face and the single-pixel frame in one figure — п. 2.2 of task T-11-13: the
    // buttons of the mock-ups have [`CORNER_RADIUS`] corners — the same radius as everything
    // else the dialog rounds off, since task T-11-16 — and a square `FillRect` under a square
    // `FrameRect` cannot have any. The corners the rounding cuts away show the erase above,
    // which is the ground the button stands on.
    paint_rounded(
        target,
        &rect,
        scaled(CORNER_RADIUS, dpi),
        colors.border,
        colors.face,
        dpi,
    );

    // SAFETY: `target` is a handle passed by value; both calls write an attribute of the DC
    // and touch no memory of this process.
    unsafe { SetBkMode(target, TRANSPARENT) };
    // SAFETY: as above.
    unsafe { SetTextColor(target, colors.ink) };

    if !caption.is_empty() {
        let mut text_rect = rect;

        // Our own face, grey-antialiased — task T-11-17. `None` leaves the manager's own font
        // in the DC, which is what this drawing used before that task (NFR-13) — and since task
        // T-15-1 that font is in the buffer too, carried over from the DC of the message when
        // the buffer was made, so a window whose faces were refused still draws in the face the
        // dialog manager gave the control and not in the stock face of a fresh memory DC.
        //
        // SAFETY: `target` is the buffer or the DC of the message and `face` is a live font the
        // window's state owns for longer than this call; the previous handle is put back below.
        let previous_face = unsafe { select_face(target, face) };

        // SAFETY: `caption` and `text_rect` are live locals of this frame; the format
        // has no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the caption
        // and writes only pixels of the DC.
        unsafe {
            DrawTextW(
                target,
                &mut caption,
                &mut text_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            )
        };

        // SAFETY: `previous_face` is what `select_face` answered for this same DC, and
        // nothing between the two calls selected another font.
        unsafe { restore_face(target, previous_face) };
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
        // SAFETY: `target` is the buffer or the DC of the message and `focus_rect` is a live
        // local of this frame; the call keeps no pointer.
        let _ = unsafe { DrawFocusRect(target, &focus_rect) };
    }

    // ⚠ **The one moment any of the above becomes visible — task T-15-1.** Nothing since the
    // erase has touched the window, so what DWM can sample is the button as it was or the button
    // as it now is, and there is no third state for the eye to catch.
    //
    // NFR-13: the answer is examined in words and dropped, for the reason the block comment at
    // the head of this body gives — and with one addition of its own. A refused blit leaves the
    // button showing the picture the window already had there, which is its previous state and
    // never a hole; there is no slower road worth taking, because painting the whole body a
    // second time into the DC of the message is the very flicker this task removes.
    //
    // SAFETY: `dc` is the DC of the message, painted into for the length of this send; `buffer`
    // is this frame's own, and its DC and bitmap are freed as it goes out of scope on this line.
    if let Some(buffer) = buffer {
        let _ = unsafe { buffer.blit(dc) };
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
                    // Unreachable by construction — `list_item_color_roles` answers the two
                    // above and nothing else — and written out rather than folded into the
                    // first arm, so that the day a row does grow a hot state the compiler
                    // has already named the colour it would take.
                    ComboFillRole::HoverBg => state.palette.hover_bg,
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

    // ⚠ **Один кадр вместо череды — task T-15-1б.** The surface the steps below are drawn onto,
    // and the same cure task T-15-1 gave the push buttons. Measured on the стенд with a self-check
    // raised by a click — the стимул a row's selection is raised by: an intermediate state reached
    // the screen in **13 of 30** transitions in «Графите» and **9 of 30** in «Тумане», the stripe
    // laid down with the label of the row **not yet on it**. ⚠ The instrument in `tools\` reported
    // a zero here and refused to certify it (`THIS ZERO IS UNCONFIRMED`, exit code 4) because its
    // self-check is raised by a press and this state by a click; the unconfirmed zero was hiding a
    // real defect.
    //
    // ⚠ `rect` of a list row is **offset** inside the control — it is not a client rectangle
    // starting at the origin — and the buffer is built for exactly that: it moves its own logical
    // origin onto the corner of `rect`, so every step below names the same rectangles it named
    // before.
    //
    // `None` — GDI refused the surface (NFR-13). `target` is then the DC of the message itself and
    // every step below paints where it painted before this task, flicker and all.
    //
    // SAFETY: `dc` is the DC of the message the caller is inside of; the buffer reads it for its
    // colour depth and its face, writes to it only in the blit at the foot of this body, and frees
    // its own DC and bitmap when this frame ends.
    // ⚠ Named `surface` and not `buffer`, unlike the other bodies of this task: the text of a row
    // is read into a `buffer` of its own a few lines below, and two `buffer`s in one body — one
    // shadowing the other inside a block — is a thing to read twice and get wrong once.
    let surface = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };
    let target = surface.as_ref().map_or(dc, theme::PaintBuffer::dc);

    // ⚠ **Since task T-15-1б this erase also grounds the buffer** — the corner smoothing of the
    // stripe below reads the ground of each corner back **out of** the DC it draws into, and the
    // pixels of a fresh `CreateCompatibleBitmap` are undefined.
    //
    // SAFETY: `target` is the buffer of this frame or, on a refusal, the DC of the message; `rect`
    // is a value of the message; `ground` is a live brush of the dialog's state.
    unsafe { FillRect(target, &rect, ground) };

    // The selection stripe of the mock-ups — the one figure both lists of the dialog wear
    // since task T-11-25, drawn by [`paint_selection_stripe`].
    if selected {
        paint_selection_stripe(target, &rect, fill_ink, fill, dpi);
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
                // SAFETY: `target` is a handle passed by value; both calls write an attribute of
                // that DC and touch no memory of this process.
                unsafe { SetBkMode(target, TRANSPARENT) };
                // SAFETY: as above.
                unsafe { SetTextColor(target, ink) };

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
                // ⚠ Into `target` and never into `dc`: the face has to be selected into the very
                // DC `DrawTextW` writes through. Selecting it into the DC of the message while
                // drawing into the buffer left the buffer wearing the font the CONTROL was
                // created with — and that font is `DEFAULT_QUALITY`, so the row came out in
                // ClearType, with colour fringes on every glyph, against the grey antialiasing of
                // the rest of the dialog. Caught by the 24 frames: `excl-dark` moved 197 levels
                // on one row (task T-15-1б, and the reason that number is not allowed to be
                // waved through as "one level on text").
                //
                // SAFETY: `target` is the buffer or the DC of the message and `face` is a live
                // font the dialog's state owns for longer than this call.
                let previous_face = unsafe { select_face(target, face) };

                // SAFETY: the slice and `text_rect` are live locals of this frame; the format
                // has no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the text
                // and writes only pixels of the DC.
                unsafe { DrawTextW(target, &mut buffer[..copied], &mut text_rect, DT_SINGLELINE) };

                // SAFETY: `previous_face` is what `select_face` answered for this same DC.
                unsafe { restore_face(target, previous_face) };
            }
        }
    }

    if focused {
        // NFR-13: the `BOOL` is examined and deliberately dropped — see above.
        //
        // SAFETY: `target` is the buffer or the DC of the message and `rect` is a live local of
        // this frame; the call keeps no pointer.
        let _ = unsafe { DrawFocusRect(target, &rect) };
    }

    // ⚠ **The one moment any of the above becomes visible — task T-15-1б.** Nothing since the
    // erase has touched the window, so what DWM can sample is the row as it was or the row as it
    // now is, and there is no third state for the eye to catch.
    //
    // NFR-13: the answer is examined in words and dropped, for the reason [`paint_push_button`]
    // gives for its own blit.
    //
    // SAFETY: `dc` is the DC of the message, painted into for the length of this send; `surface`
    // is this frame's own, and its DC and bitmap are freed as it goes out of scope on this line.
    if let Some(surface) = surface {
        let _ = unsafe { surface.blit(dc) };
    }

    // TRUE — the row is drawn.
    1
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
/// The ground of the whole rectangle first — the brush `WM_CTLCOLORBTN` answers for this
/// control — because an owner-drawn control is responsible for the whole of its rectangle and
/// a repaint has to be idempotent (task T-12-6; see the fill itself for the measurement).
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

            // The ground this element stands on — the very rule `on_ctl_color` answers
            // `WM_CTLCOLORBTN` with: the panel brush for a control lying on one of the eight
            // group panels, the window brush elsewhere. All eight glyph elements do lie on a
            // panel, but the rule is asked and not assumed, exactly as `draw_combo_closed_part`
            // asks it. See the fill below for why it is needed at all (task T-12-6).
            let ground = if state.panel_children.contains(&control) {
                brushes.panel_bg()
            } else {
                brushes.window_bg()
            };

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
                ground,
                fill,
                fill_ink,
                frame,
                mark,
                ink,
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    let Some(Some((ground, fill, fill_ink, frame, mark, ink, face))) = choice else {
        return 0;
    };

    let dpi = dc_dpi(dc);

    // ⚠ **Один кадр вместо череды — task T-15-1б.** The surface the steps below are drawn onto,
    // and the same cure task T-15-1 gave the push buttons. Measured on the стенд with a self-check
    // raised by a click, the стимул this element's own state is raised by: an intermediate state
    // reached the screen in **22 of 30** transitions in «Графите» and **18 of 30** in «Тумане» —
    // the ground erased and the caption **not yet drawn**, a line of the dialog standing blank for
    // one frame. ⚠ The instrument in `tools\` reported a zero here and refused to certify it
    // (`THIS ZERO IS UNCONFIRMED`, exit code 4) exactly because its self-check is raised by a
    // press and this state by a click; the unconfirmed zero was hiding a real defect, which is the
    // whole reason that refusal exists.
    //
    // `None` — GDI refused the surface (NFR-13). `target` is then the DC of the message itself and
    // every step below paints where it painted before this task, flicker and all.
    //
    // ⚠ `dpi` is taken one line above off the DC of the **message** and never off the buffer:
    // `GetDeviceCaps` of a memory surface answers for the memory and not for the window, and the
    // glyph's radius, its square and its check mark are all scaled through it.
    //
    // SAFETY: `dc` is the DC of the message the caller is inside of; the buffer reads it for its
    // colour depth and its face, writes to it only in the blit at the foot of this body, and frees
    // its own DC and bitmap when this frame ends.
    let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &rect) };
    let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);

    // ⚠ **The ground, before anything is drawn on it — task T-12-6.**
    //
    // An owner-drawn button is responsible for the whole of its rectangle, exactly as the
    // owner-drawn static of [`paint_label`] is: `WM_CTLCOLORBTN` names the brush, but nothing
    // promises that the system lays it down before handing the drawing over — and the measured
    // answer is that it does not. A check box that is repainted **over itself** without this
    // fill draws its caption a second time on top of the first, and since `SetBkMode` is
    // `TRANSPARENT`, the half-tone pixels of the antialiasing blend with what is already there
    // instead of replacing it: an edge pixel of the caption's halo went 179,182,186 →
    // 225,228,231 on the stand, ≈74 % → ≈98 % of the ink, and the text visibly thickened. The
    // cores never moved, which is what said it was a second pass and not a different drawing.
    //
    // The repaint that does it is an ordinary one: the element loses the focus to whatever the
    // person clicked — pressing «Задать» is the case the defect was found in — and the system
    // button invalidates itself **without** erasing (`bErase = FALSE`), because an owner-drawn
    // control has no background of its own to erase.
    //
    // Filling here is right under either behaviour — a second fill over the same colour is
    // invisible and costs one `FillRect` — and skipping it is right under only one. It is the
    // same erase `draw_combo_closed_part` does for the closed part of a combo box and
    // `paint_label` for a caption.
    //
    // NFR-13: the answer is examined in words and deliberately dropped, for the reason the
    // block comment below the glyph rectangle gives for every other paint call here.
    //
    // ⚠ **And since task T-15-1б the erase carries a second duty: it is what grounds the buffer.**
    // The corner smoothing of the glyph reads the ground of each corner back **out of** the DC it
    // draws into, and the pixels of a fresh `CreateCompatibleBitmap` are undefined — so this fill,
    // standing exactly where it stood, is what makes the buffer hold under the corners the same
    // colour the screen held there.
    //
    // SAFETY: `target` is the buffer of this frame or, on a refusal, the DC of the message; `rect`
    // is a value of the message, and `ground` is a live brush the dialog's state owns for longer
    // than this call.
    unsafe { FillRect(target, &rect, ground) };

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
                target,
                &glyph,
                scaled(GLYPH_CORNER_RADIUS, dpi),
                frame.unwrap_or(fill_ink),
                fill,
                dpi,
            );

            if let Some(GlyphMarkPaint::Check(mark_ink)) = mark {
                draw_check_mark(target, &glyph, mark_ink, GLYPH_CHECK_MARK, dpi);
            }
        }
        GlyphKind::RadioButton => {
            paint_ellipse(target, &glyph, frame, fill, dpi);

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

                paint_ellipse(target, &dot, None, dot_brush, dpi);
            }
        }
    }

    // The caption, from the dialog's own control by identifier — never from the message.
    // Without the trailing NUL: `DrawTextW` takes the length of the slice it is given.
    let mut caption: Vec<u16> = get_text(hwnd, control).encode_utf16().collect();

    if !caption.is_empty() {
        // SAFETY: `target` is a handle passed by value; both calls write an attribute of that
        // DC and touch no memory of this process.
        unsafe { SetBkMode(target, TRANSPARENT) };
        // SAFETY: as above.
        unsafe { SetTextColor(target, ink) };

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
        // SAFETY: `target` is the buffer or the DC of the message and `face` is a live font the
        // dialog's state owns for longer than this call; the previous handle is put back below.
        let previous_face = unsafe { select_face(target, face) };

        // SAFETY: `caption` and `text_rect` are live locals of this frame; the format has
        // no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the caption and
        // writes only pixels of the DC.
        unsafe {
            DrawTextW(
                target,
                &mut caption,
                &mut text_rect,
                DT_VCENTER | DT_SINGLELINE,
            )
        };

        if focused {
            // The dotted frame goes around the caption, not the glyph — as the native
            // control draws it. The width is measured with `DT_CALCRECT` into a scratch
            // rectangle; the height is the control's own, the caption being vertically
            // centred in it. (An element with no caption gets no cue — none of the eight
            // is captionless, and a frame around nothing would be noise.)
            let mut measured = text_rect;

            // SAFETY: as above — `DT_CALCRECT` writes the measured extent into
            // `measured`, a live local of this frame, and draws nothing.
            unsafe {
                DrawTextW(
                    target,
                    &mut caption,
                    &mut measured,
                    DT_CALCRECT | DT_SINGLELINE,
                )
            };

            let focus_rect = RECT {
                left: text_rect.left - FOCUS_CUE_TEXT_INSET,
                top: rect.top,
                right: (measured.right + FOCUS_CUE_INSET).min(rect.right),
                bottom: rect.bottom,
            };

            // NFR-13: the `BOOL` is examined and deliberately dropped — see the block
            // comment above the glyph painting.
            //
            // SAFETY: `target` is the buffer or the DC of the message and `focus_rect` is a
            // live local of this frame; the call keeps no pointer.
            let _ = unsafe { DrawFocusRect(target, &focus_rect) };
        }

        // SAFETY: `previous_face` is what `select_face` answered for this same DC, and
        // nothing between the two calls selected another font.
        unsafe { restore_face(target, previous_face) };
    }

    // ⚠ **The one moment any of the above becomes visible — task T-15-1б.** Nothing since the
    // erase has touched the window, so what DWM can sample is the element as it was or the element
    // as it now is, and there is no third state for the eye to catch — in particular no frame in
    // which the ground is erased and the caption is not yet on it.
    //
    // NFR-13: the answer is examined in words and dropped, for the reason [`paint_push_button`]
    // gives for its own blit. A refused blit leaves the element showing the picture the window
    // already had there — its previous state and never a hole.
    //
    // SAFETY: `dc` is the DC of the message, painted into for the length of this send; `buffer` is
    // this frame's own, and its DC and bitmap are freed as it goes out of scope on this line.
    if let Some(buffer) = buffer {
        let _ = unsafe { buffer.blit(dc) };
    }

    // TRUE — the element is drawn.
    1
}

// =========================================================================================
// The lengths of the mock-ups, and the way this dialog turns its own numbers into pixels —
// FR-92а, tasks T-11-13 and T-11-15. The second way, the DPI scale of the mock-ups, moved to
// module `theme` with the drawing library (task Т-14-3); what stays here is `dialog_units`,
// which asks the dialog itself through `MapDialogRect`.
// =========================================================================================

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

/// What `SysListView32` puts between the **end of the state image cell** and the first ink of
/// the label it draws there, in screen pixels — measured on the live control, task T-12-7.
///
/// # Not a length of the mock-ups, and therefore not [`scaled`]
///
/// Every other number of this list is a literal of `scratchpad-Э11\ui.ps1` put through
/// [`scaled`] or through [`dialog_units`]. This one is neither: it is a property of the
/// control, it appears in no message and in no documented constant, and the generator of the
/// mock-ups knows nothing about it. What it *is* is the reason the row of the live list stood
/// four pixels right of the row of the mock-ups (finding **F4**/**BLIND-5** of the E12
/// protocol, with the correction **VCORR**), and this constant is what takes those four
/// pixels back out — by making the cell of the state image list exactly this much narrower
/// than the row the mock-ups lay out ([`CheckCell::image_width`]).
///
/// # The measurement, in two halves, each named by its instrument
///
/// Stand `dlgstand dark cycle`, the dialog of this module, the layout list holding this
/// module's own 19 × 23 cell, 96 DPI:
///
/// * **2 px — the label rectangle.** `LVM_GETITEMRECT` answers `LVIR_BOUNDS` = `0,2,229,26`
///   and `LVIR_LABEL` = `21,2,229,26` for the same row. The cell is 19 wide
///   (`ImageList_GetIconSize` of the list the control holds), so the control starts the label
///   rectangle **2 px** after the cell ends.
/// * **2 px — the pen inside that rectangle.** The label rectangle begins at client x = 21,
///   which is screen x = 54 on the shot (`stand\t127-before-cycle-dark-plain.png`, list
///   interior from x = 33); the first ink of the row is at screen x = 56 — «А» of «Английская»,
///   two smoothing pixels before the stem, probed column by column. The control lays the pen
///   down **2 px** inside the rectangle it just reported.
///
/// Both halves are arithmetic on numbers an instrument answered — none is read off a picture.
///
/// ⚠ Measured at **96 DPI**, which is the DPI of every machine this program has been measured
/// on. Whether the control scales either half with the DPI of the window is not documented and
/// was not measurable here; on a machine at another scale this number is to be measured again
/// the same way. It is a screen-pixel number for exactly that reason: pretending it is a
/// mock-up length and putting it through [`scaled`] would claim a proportion nobody measured.
pub const LVIEW_LABEL_INDENT: i32 = 4;

/// What `SysListView32` adds to the height of the cell of its state image list to arrive at
/// the height of a row, in screen pixels — measured on the live control, task T-12-7.
///
/// The same kind of number as [`LVIEW_LABEL_INDENT`] and named for the same reason: the row
/// height of a report list view is the cell height and nothing a message can reach
/// ([`install_check_images`]), but it is not *equal* to it. Measured on the stand at 96 DPI:
/// the cell is 23 px tall (`ImageList_GetIconSize` — `dialog_units` of
/// [`LAYOUT_ROW_HEIGHT_DLU`], `MulDiv(12, 15, 8)`), and `LVM_GETITEMRECT` with `LVIR_BOUNDS`
/// answers `0,2,229,26` and `0,26,229,50` — rows **24** px tall, pitch 24. One pixel more than
/// the cell, on every row, which is finding **F5**/**BLIND-6**: 24 px against the 22,5 px of
/// the mock-ups.
///
/// So the cell is handed the row of the mock-ups **less this**, and the control's own pixel
/// puts it back — [`install_check_images`] is the one place that arithmetic lives.
pub const LVIEW_ROW_OVERHEAD: i32 = 1;

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

/// Point size of the name row of the about window, in **tenths of a point** — finding
/// **A-05**, task T-12-4.
///
/// The generator draws that one row in a face of its own: `$FBold = Font('Segoe UI', 15,
/// Bold)` (`scratchpad-Э11\chrome.ps1:109`, used at line 194) against the `$FS = 9 * $DPI`
/// = 12,6 pt of everything else in that window. Both are stated at the mock-ups' 140 %, so
/// the ratio is what carries over — 15 / 12,6 of the dialog font, which at the 9 pt of the
/// template is **10,7 pt**.
///
/// Tenths of a point and a ratio for the same reason [`PANEL_CAPTION_POINTS_TENTHS`] is: the
/// face is never *made* from this number, it is made from the window's own `lfHeight` scaled
/// by it, so the row grows with the DPI of the window like everything else and no point size
/// is written down by hand.
pub const ABOUT_NAME_POINTS_TENTHS: i32 = 107;

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

/// What the dialog's own background draws under one control — FR-92а, task T-11-13, the
/// pure half of the background drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundFigure {
    /// A group panel: the rounded [`theme::Palette::panel_bg`] fill under a single-pixel
    /// [`theme::Palette::panel_border`] frame, over the control's whole rectangle, with the
    /// caption of the (invisible) control at its top edge.
    Panel,
    /// An input field or a list: the rounded [`theme::Palette::field_bg`] fill under a
    /// single-pixel [`theme::Palette::field_border`] frame, drawn outside the control's
    /// rectangle — so the corners the rounding cuts away are the field's own colour and not
    /// the panel's. How far outside is [`field_frame_air`]'s answer for a field and
    /// [`theme::LIST_FIRST_ROW_TOP`] above a list.
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
/// stands [`theme::LIST_FIRST_ROW_TOP`] mock-up pixels above its first row, where the frame of a
/// field is the [`FIELD_BOX_DLU`] box of the generator centred on the control. Both lists are
/// in [`FRAMED_FIELDS`] as well — this is a subset of it and not a second list of controls,
/// and a test holds the containment.
pub const FRAMED_LISTS: [i32; 2] = [IDC_EXCLUSIONS, IDC_CYCLE_LIST];

/// The height of the box the mock-ups draw around an input field, in vertical **dialog
/// units** — the `h = 12` of every `'edit'` row of the generator.
///
/// Read straight off `scratchpad\ui.ps1`: the five fields of the pictures are
/// `@{t='edit'; …; h=12}` (lines 108, 130, 135, 137 and 142), and the `'edit'` arm strokes
/// its frame **inside** that rectangle. Twelve of these units are `MulDiv(12, 15, 8)` = 23
/// pixels of the window at 96 DPI, and grow with the font at every other DPI because
/// `MapDialogRect` is what maps them ([`dialog_units`]) — the number is never a pixel count
/// written down by hand.
///
/// # Why the box is not simply the control's own rectangle
///
/// A single-line `EDIT` lays its text along the **top** of its client area and no documented
/// message moves it down: `EM_SETMARGINS` ([`set_field_margins`]) moves the two sides only,
/// and `EM_SETRECT` is documented for multiline controls. So a control as tall as the box
/// leaves the whole surplus underneath the text — the 3 above and 11 below that task T-12-3
/// was given to close. The lever this file holds is the same one task T-11-16 used for the
/// two lists: **where the frame is drawn**. The template gives the control exactly one
/// [`DIALOG_FONT_HEIGHT_DLU`] — one em box, nothing to spare — and the background draws this
/// box around it, centred, so the air of the picture is above and below the text in equal
/// halves.
pub const FIELD_BOX_DLU: i32 = 12;

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
    /// [`theme::Palette::cap`] — the ink of a panel caption since task T-12-11.
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
///    than the dialog font, letter-spaced, in `cap` since task T-12-11 — read off that same
///    hidden control with `GetDlgItemTextW`, which is what keeps FR-94 working with no
///    second source of truth for the eight strings;
/// 3. every field and list, outside its own rectangle: the rounded fill and frame of
///    [`BackgroundFigure::Field`] — one thickness around a list, and the centred
///    [`FIELD_BOX_DLU`] box of the generator around a field (task T-12-3).
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
                    caption: palette.cap,
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

    // Task T-12-1, п. 4: the line the mock-ups draw under the title bar. Laid straight after
    // the ground and before every figure, so that a panel or a field which ever reached the
    // top of the client area would paint over it rather than lose its own first row. None
    // does today — the topmost figure of this template starts well below — and the
    // acceptance probe of the row says so in numbers.
    paint_caption_underline(dc, area, colors.field_border);

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
    //
    // …except above a list, where the mock-ups leave [`LIST_FIRST_ROW_TOP`] of their own
    // pixels between the frame and the first row (`$ry = $py + 3 + …` in both list arms of the
    // generator). A list box lays its rows out from the top of its client area and no message
    // moves them, so the air of the picture is made by lifting the frame instead — task
    // T-11-16, п. 5.
    //
    // Both numbers come from [`list_frame_air`] since task T-12-5, and not from two `scaled`
    // calls written out here: the same frame is now named a second time from inside each list
    // ([`list_frame_box`]), and a copy of this arithmetic there is exactly how the arc and the
    // end of the interior would drift apart.
    let (list_top, border) = list_frame_air(dpi);

    // …and except for a field, whose frame is not hung on the control at all: the box of the
    // mock-ups is [`FIELD_BOX_DLU`] tall and the template gives the control one font height,
    // so the frame is drawn round the control **centred** and the leftover air falls above and
    // below the text in equal halves — task T-12-3. `None` is a refused `MapDialogRect`, and
    // [`field_frame_air`] answers the old one-thickness frame to it (NFR-13).
    let field_box = dialog_units(hwnd, 0, FIELD_BOX_DLU).map(|(_, height)| height);

    for (control, rect) in &children {
        if background_figure(*control) != Some(BackgroundFigure::Field) {
            continue;
        }

        // A list keeps its own two distances — the air of the picture above, one thickness
        // below — because its rows are laid out by the control from the top of its client
        // area and centring the frame would move the frame away from the first row.
        let (top, bottom) = if FRAMED_LISTS.contains(control) {
            (list_top, border)
        } else {
            let air = field_frame_air(field_box, rect.bottom - rect.top, border);
            (air, air)
        };

        let frame = RECT {
            left: rect.left - border,
            top: rect.top - top,
            right: rect.right + border,
            bottom: rect.bottom + bottom,
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

/// The `LOGFONTW` of the name row of the about window — finding **A-05**, task T-12-4.
///
/// [`caption_logfont`] with one number changed: [`ABOUT_NAME_POINTS_TENTHS`] over
/// [`DIALOG_FONT_POINTS_TENTHS`] instead of the caption's ratio, so the row comes out *larger*
/// than the dialog font where the panel caption comes out smaller. Bold in both cases, and
/// both go through [`antialiased_logfont`], so the quality is named in exactly one place.
///
/// ⚠ **The size travels through the window's own `lfHeight` and never through «10,7 pt»**: the
/// dialog font is what the template asks for and what the manager already created at the DPI
/// of the window, so a face derived from it is right at every scale, and a face created from a
/// point size by hand would be right at 96 DPI only.
///
/// Measured at 96 DPI, where the manager's face is `lfHeight` −12: this answers −14, and the
/// «L» of «Lang Switcher» comes out **10 px tall on a two-pixel stem** — the mock-up's numbers
/// (A-05: 14 mock-up px = 10 screen px, stem 2). Pure, like the two above it.
pub fn about_name_logfont(base: LOGFONTW) -> LOGFONTW {
    let mut logical = antialiased_logfont(base);

    logical.lfHeight = (base.lfHeight * ABOUT_NAME_POINTS_TENTHS) / DIALOG_FONT_POINTS_TENTHS;
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

/// The **three** faces one window sets **its own** text in, owned: created together, freed
/// together in `Drop` — FR-92а, task T-11-17, widened by task T-12-4.
///
/// # Why one type serves both windows although neither uses all three
///
/// The settings dialog draws with `text` and `caption` and has no name row; the about window
/// draws with `text` and `name` and has no panel to caption. That is not new with the third
/// face — the about window has been carrying an unused `caption` since task T-11-11 — and it
/// is the cheaper half of the trade: one unused `CreateFontIndirectW` per window against a
/// second owner with a second `Drop` to keep the discipline of. The set is «the faces a window
/// of this file may set its own text in», and each window uses the ones it has text for.
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
    /// The same face **larger** and bolder, for the one row it belongs to: «Lang Switcher» in
    /// the about window — [`about_name_logfont`], finding A-05, task T-12-4.
    name: HFONT,
}

impl DialogFonts {
    /// All three faces out of the font `control` was given, every handle examined.
    ///
    /// `None` on every refusal — no font on the control, an unreadable `LOGFONTW`, any
    /// `CreateFontIndirectW` declining. The window then draws in the manager's own face, which
    /// is what it drew in before this task: degraded-but-alive, and the whole loss is the
    /// smoothing (NFR-13).
    ///
    /// ⚠ A half-built set is unwound here and not leaked: the handles already created are
    /// freed on the failing path, because a constructor that answers `None` leaves no value
    /// for `Drop` to run on — the same discipline [`CaptionIcons::load`] keeps.
    fn new(hwnd: HWND, control: i32) -> Option<Self> {
        let base = dialog_logfont(hwnd, control)?;

        let text = create_font(antialiased_logfont(base))?;

        let Some(caption) = create_font(caption_logfont(base)) else {
            // SAFETY: `text` was created a line above, handed to nobody, and is freed exactly
            // once here — the failed constructor answers `None` and no `Drop` will run.
            let _ = unsafe { DeleteObject(text.into()) };
            return None;
        };

        let Some(name) = create_font(about_name_logfont(base)) else {
            for face in [text, caption] {
                // SAFETY: both were created above, handed to nobody, and are freed exactly
                // once here — the failed constructor answers `None` and no `Drop` will run.
                let _ = unsafe { DeleteObject(face.into()) };
            }

            return None;
        };

        Some(Self {
            text,
            caption,
            name,
        })
    }

    /// The face of the window's own text, borrowed — the owner frees it, nobody else.
    fn text(&self) -> HFONT {
        self.text
    }

    /// The face of a panel caption, borrowed — the owner frees it, nobody else.
    fn caption(&self) -> HFONT {
        self.caption
    }

    /// The face of the about window's name row, borrowed — the owner frees it, nobody else.
    fn name(&self) -> HFONT {
        self.name
    }
}

impl Drop for DialogFonts {
    fn drop(&mut self) {
        for face in [self.text, self.caption, self.name] {
            // SAFETY: each came from a successful `CreateFontIndirectW` in `new` and is freed
            // exactly once: the type is neither `Copy` nor `Clone`, its fields are private and
            // never reassigned, and `drop` runs once. Every drawing that selected a face put
            // the previous one back before it returned, so neither is in a DC any more. The
            // `BOOL` is dropped for the reason `theme::Brushes` gives for its own cleanup.
            let _ = unsafe { DeleteObject(face.into()) };
        }
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

        // ⚠ Task T-12-8, the response of the cursor — the two arms the same procedure of the
        // buttons has, and for the same reason. Neither *answers* the message: both fall
        // through to `DefSubclassProc` below, so the control's own hot-tracking, its click and
        // its keyboard are untouched — in particular **the list does not drop down**, which is
        // what a swallowed `WM_MOUSEMOVE` would have been the shortest road to breaking. The
        // repaint the flag asks for goes through `WM_PAINT` and the arm above it, and the
        // colour is chosen by `combo_closed_color_roles`.
        WM_MOUSEMOVE => enter_hot(combo),

        WM_MOUSELEAVE => leave_hot(combo),

        // The belt to the braces of the pair — see `subclass_combo_boxes`. Forwarded on
        // afterwards: `WM_NCDESTROY` must reach every procedure of the chain.
        //
        // SAFETY: `combo` is that control, and the procedure and identifier are the pair the
        // subclass was installed with.
        WM_NCDESTROY => {
            // Task T-12-8: the record cannot outlive the window it names — see `HOT_CONTROL`.
            forget_hot(combo);
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

    // Task T-12-8: the cursor standing on the closed part, out of this program's own record —
    // the very question the push buttons ask, asked here about the control that is painting
    // itself rather than about the window of a message.
    let roles = combo_closed_color_roles(is_hot(combo), disabled);

    // The colour choice, split from the painting exactly as everywhere in this file: the
    // borrow of the state ends before the DC is touched, and what leaves the closure is plain
    // values — a brush the state keeps alive until the dialog ends, three inks, and (only while
    // the closed part is hot) the transient brush of the response, which the binding below
    // holds alive until this function returns.
    //
    // SAFETY: `dialog` is the parent of one of this dialog's own controls, which is the
    // window `show_dialog` created — the contract of `with_state`.
    let choice = unsafe {
        with_state(dialog, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13, T-11-4).
            let brushes = state.brushes.as_ref()?;
            let palette = state.palette;

            // The hot fill is the one colour of the table the window's brush set does not hold
            // — see [`HotBrush`]. `None` for every other role, and also for a refused
            // `CreateSolidBrush`, in which case the fill below falls back to the quiet
            // `field_bg` and the closed part simply does not light up (NFR-13).
            let hot = match roles.fill {
                ComboFillRole::HoverBg => HotBrush::new(palette.hover_bg),
                ComboFillRole::FieldBg | ComboFillRole::SelBg => None,
            };

            let fill = hot
                .as_ref()
                .map_or_else(|| combo_fill_brush(roles.fill, brushes), HotBrush::brush);

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
                fill,
                hot,
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

    // `_hot` is named and not discarded on purpose: it owns the transient brush `fill` may name,
    // and a `_` would have freed it before the first pixel was drawn.
    let Some(Some((ground, fill, _hot, border, ink, chevron_ink, face))) = choice else {
        return;
    };

    let dpi = dc_dpi(dc);

    // ⚠ **Один кадр вместо череды — task T-15-1б.** The surface the steps below are drawn onto,
    // and the same cure task T-15-1 gave the push buttons: a string of separate GDI calls
    // straight into the DC of a control is not atomic, and DWM samples the surface of a window
    // sixty times a second. Measured on the стенд against the closed part of `IDC_THEME`, thirty
    // hover transitions in each palette: an intermediate state reached the screen in **30 of 30**,
    // its bounding box exactly this control's rectangle, holding the hot fill, the frame ink and
    // **100 px of the ground of the panel** — the four 5×5 corner squares of raw ground standing
    // where the smoothed arcs were not yet drawn. Off-screen there is no screen for that state to
    // appear on, and the finished field crosses over in the single blit at the foot of this body.
    //
    // `None` — GDI refused the surface (NFR-13). `target` is then the DC of this paint itself and
    // every step below paints where it painted before this task, flicker and all: the honest
    // degradation [`BackgroundCache`] answers a refusal with.
    //
    // ⚠ `dpi` is taken one line above off the DC of **this paint** and never off the buffer.
    // `GetDeviceCaps` of a memory surface answers for the memory and not for the window, and a
    // radius scaled off the buffer would be wrong at every scale but 100 % with no test going red.
    //
    // SAFETY: `dc` is the DC `BeginPaint` answered for this window, owned until `EndPaint`; the
    // buffer reads it for its colour depth and its face, writes to it only in the blit below, and
    // frees its own DC and bitmap when this frame ends.
    let buffer = unsafe { theme::PaintBuffer::for_rect(dc, &area) };
    let target = buffer.as_ref().map_or(dc, theme::PaintBuffer::dc);

    // 1. The ground, then the field on top of it. `WM_ERASEBKGND` deliberately erases
    // nothing (see `combo_box_proc`), so this is the one erase of the closed part — and it
    // is what the four corners the rounding cuts away are filled with. Without it those
    // corners would hold whatever the parent last painted there, which is right only for as
    // long as the dialog carries no `WS_CLIPCHILDREN`.
    //
    // ⚠ **And since task T-15-1б the erase carries a second duty: it is what grounds the buffer.**
    // The corner smoothing reads the ground of each corner back **out of** the DC it draws into,
    // and the pixels of a fresh `CreateCompatibleBitmap` are undefined — so this fill, standing
    // exactly where it stood, is what makes the buffer hold under the corners the same colour the
    // screen held there, and the halftones of the arcs come out unmoved.
    //
    // SAFETY: `target` is the buffer of this frame or, on a refusal, the DC of this paint;
    // `area` is a live local of this frame; `ground` is a live brush of the dialog's state.
    unsafe { FillRect(target, &area, ground) };

    paint_rounded(target, &area, scaled(CORNER_RADIUS, dpi), border, fill, dpi);

    // 2. The chevron, in place of the system button.
    let chevron = combo_chevron_points(&area, dpi);
    draw_combo_chevron(target, chevron, chevron_ink, dpi);

    // 3. The chosen value, from the control by identifier — never from anywhere else.
    let mut value = combo_selected_text(dialog, control);

    if !value.is_empty() {
        // SAFETY: `target` is a handle passed by value; both calls write an attribute of that DC
        // and touch no memory of this process.
        unsafe { SetBkMode(target, TRANSPARENT) };
        // SAFETY: as above.
        unsafe { SetTextColor(target, ink) };

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
            // SAFETY: `target` is the buffer or the DC of this paint, and `font` is either a live
            // face the dialog's state owns for longer than this call or the live font of the
            // control, owned by the manager; the previous handle is put back below.
            Some(unsafe { SelectObject(target, font.into()) })
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
                target,
                &mut value,
                &mut text_rect,
                DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            )
        };

        if let Some(previous) = previous_font {
            // SAFETY: `previous` is the font that was in the DC a moment ago; putting it back
            // ends this function's use of it.
            unsafe { SelectObject(target, previous) };
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
        // SAFETY: `target` is the buffer or the DC of this paint and `focus_rect` is a live local
        // of this frame; the call keeps no pointer.
        let _ = unsafe { DrawFocusRect(target, &focus_rect) };
    }

    // ⚠ **The one moment any of the above becomes visible — task T-15-1б.** Nothing since the
    // erase has touched the window, so what DWM can sample is the closed part as it was or as it
    // now is, and there is no third state for the eye to catch.
    //
    // NFR-13: the answer is examined in words and dropped, for the reason [`paint_push_button`]
    // gives for its own blit. A refused blit leaves the control showing the picture the window
    // already had there — its previous state and never a hole — and painting the body a second
    // time into the DC of the paint is the very flicker this task removes.
    //
    // SAFETY: `dc` is the DC of this paint, owned until `EndPaint`; `buffer` is this frame's own,
    // and its DC and bitmap are freed as it goes out of scope on this line.
    if let Some(buffer) = buffer {
        let _ = unsafe { buffer.blit(dc) };
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

// =========================================================================================
// Отклик наведения: «когда наводишь — должно быть видно» — FR-92а, task T-12-8
// =========================================================================================
//
// ⚠ **What this section is.** Not a repair of a disagreement with the mock-ups: the mock-up of
// this dialog draws no hot state at all (`scratchpad-Э11\ui.ps1` has no 'Hover' in it, and the
// 'btn' branch of `Draw-Control` knows one state), and the protocol of stage Э12 recorded the
// absence as a *match*. It is a new requirement, arrived at in the user's own words while the
// delivery was being looked over: «когда наводишь — должно быть видно, это общепринятая
// практика». The reference for the look is therefore not the mock-up but this program's **own
// tray menu**, where the response was made and accepted in task T-11-21: the entry under the
// cursor carries `hover_bg`, and the same palette field is what lights a button here.
//
// **What the cursor costs.** Nothing but a flag. `WM_MOUSEMOVE` arrives by the dozen per
// second, so the handler below allocates nothing, reads nothing, journals nothing and — the
// point of the whole arrangement — asks for a repaint **only on the edge**: the very first move
// onto a control arms the leave notification and lights it, every move after that finds the
// flag already set and returns without touching the window. One repaint on the way in, one on
// the way out, and not a single timer or thread anywhere near it (NFR-01…NFR-05 hold because
// the keyboard hook is not on this road at all — everything here is the UI thread's).

thread_local! {
    /// The one control the cursor stands on, zero when it stands on none — task T-12-8.
    ///
    /// One place and not a field per window, because the cursor is one: at most one control of
    /// this thread can be hot at any moment, and a second store would only be a way for the two
    /// to disagree. Stored as the plain integer the handle is, exactly as [`DIALOG_WINDOW`]
    /// beside it, and **compared and never followed** — nothing here dereferences a window
    /// handle, so a stale value can at worst make one control paint itself the way it already
    /// looks. It is cleared on `WM_NCDESTROY` all the same, so a handle the system later reuses
    /// cannot inherit somebody else's highlight.
    ///
    /// A thread-local because the windows of this program live on the UI thread and on no other
    /// (section 6.1).
    static HOT_CONTROL: Cell<isize> = const { Cell::new(0) };
}

/// Whether the cursor stands on this control — the question the drawing halves ask.
///
/// The zero check is not redundant: an invalid handle must not match the «nothing is hot»
/// record.
fn is_hot(window: HWND) -> bool {
    let hot = HOT_CONTROL.with(Cell::get);

    hot != 0 && hot == window.0 as isize
}

/// The cursor has arrived on `window`: arm the leave notification, remember the control and ask
/// for the one repaint — task T-12-8.
///
/// **Nothing happens on a move that changes nothing.** The flag is looked at first, and a
/// control that is already the hot one leaves this function without a single call — which is
/// what keeps a window that the cursor is resting on from repainting itself for ever.
///
/// The order is deliberate: the tracking is armed **before** the flag is set, so a refused
/// `TrackMouseEvent` — the one call here that can leave the state stuck — leaves the control
/// cold instead of leaving it lit with no way back. A control that never lights up is the
/// picture every task before this one drew; a control that lights up and never goes out would
/// be a new defect (NFR-13).
fn enter_hot(window: HWND) {
    if is_hot(window) {
        return;
    }

    let mut tracking = TRACKMOUSEEVENT {
        cbSize: u32::try_from(size_of::<TRACKMOUSEEVENT>()).unwrap_or(0),
        dwFlags: TME_LEAVE,
        hwndTrack: window,
        dwHoverTime: 0,
    };

    // SAFETY: `tracking` is a live local of this frame, filled in whole above, and the call
    // reads it and keeps no pointer to it; `window` is the control this procedure is installed
    // on. NFR-13: the answer is examined right here — a refusal means no `WM_MOUSELEAVE` will
    // ever come, so the flag is left alone and this control simply has no response. Nothing is
    // journaled, for the reason `subclass_combo_boxes` writes down at its own dropped answer:
    // the closed operation vocabulary of `diag` has no row for this call, the consequence is
    // cosmetic, and the dialog works either way.
    if unsafe { TrackMouseEvent(&mut tracking) }.is_err() {
        return;
    }

    HOT_CONTROL.with(|hot| hot.set(window.0 as isize));

    invalidate_hot(window);
}

/// The cursor has left `window`: forget it and ask for the repaint back to the quiet face.
///
/// The record is cleared **only when it still names this control**. The two messages are not
/// ordered against each other by anything: a `WM_MOUSELEAVE` for the control the cursor came
/// from can arrive after the `WM_MOUSEMOVE` of the one it went to, and a clear that did not
/// look would then put out the light of the control the cursor is actually on. The repaint is
/// asked for either way — this control has to stop showing the hot face whichever of the two
/// orders happened.
fn leave_hot(window: HWND) {
    if is_hot(window) {
        HOT_CONTROL.with(|hot| hot.set(0));
    }

    invalidate_hot(window);
}

/// The control is being destroyed: drop the record if it names it — task T-12-8.
///
/// Not a repaint and not a message: a window inside `WM_NCDESTROY` has nothing left to paint.
/// This exists so that the record cannot outlive the window it names — see [`HOT_CONTROL`].
fn forget_hot(window: HWND) {
    if is_hot(window) {
        HOT_CONTROL.with(|hot| hot.set(0));
    }
}

/// The one repaint of the response — the whole rectangle of the control, without an erase.
///
/// `false` for the erase because every drawing this repaint reaches grounds its own rectangle
/// before it draws: an owner-drawn push button since task T-12-6, the closed face of a combo
/// box since T-11-14. An erase on top of that would be the two-step repaint that flickers.
fn invalidate_hot(window: HWND) {
    // SAFETY: `window` is the control this procedure is installed on; the call marks a
    // rectangle of it and touches no memory of ours.
    //
    // NFR-13: examined in words and deliberately dropped. A refusal — a window already gone —
    // means the control keeps the face it last drew until something else invalidates it, which
    // is a cosmetic nothing on a window that is being taken down anyway; there is no journal
    // row for it (reviews\T-11-1.md) and no `debug_assert`, because a window closing under the
    // cursor is a legal state and not a violation of ownership.
    let _ = unsafe { InvalidateRect(Some(window), None, false) };
}

/// The nine owner-drawn **push buttons** of the settings dialog — task T-12-8.
///
/// The single place they are listed, and the list [`subclass_buttons`] and
/// [`unsubclass_buttons`] both walk, so every install has its removal by construction — the
/// discipline [`subclass_combo_boxes`] set for the four combo boxes.
///
/// ⚠ **Push buttons and nothing else.** The check boxes, the radio buttons and the eight group
/// panels are `BS_OWNERDRAW` too and are therefore «кнопки» to the window manager, but they are
/// not what this response is about: the colours of a push button are the closed table of
/// [`button_color_roles`], which task T-12-8 widened, while a glyph element answers
/// [`glyph_color_roles`] — a table with no hot state and no palette field waiting to be its
/// ground — and a panel is the *background* of its block since task T-11-13 and takes no
/// clicks at all. Subclassing either would arm a notification for a repaint that changes
/// nothing.
const PUSH_BUTTONS: [i32; 9] = [
    IDC_HOTKEY_CAPTURE,
    IDC_CYCLE_UP,
    IDC_CYCLE_DOWN,
    IDC_EXCLUSION_REMOVE,
    IDC_EXCLUSION_ADD,
    IDC_LOG_OPEN,
    OK_COMMAND,
    CANCEL_COMMAND,
    IDC_APPLY,
];

/// The one owner-drawn push button of the about window — task T-12-8.
///
/// It is «ОК», and [`button_color_roles`] answers the accent for it at rest and
/// [`Palette::sel_fg`] under the cursor — **task T-15-2 gave the accented button its response**,
/// and the day it did, the flag this subclass keeps stopped being a spare and became load
/// bearing. ⚠ Until that task the doc here said the opposite, and said it for a good reason:
/// the palette held no second accent and inventing a colour is forbidden. The way out was not
/// a new colour but an existing field — which is why the wiring was kept uniform all along,
/// and why the response appeared by one arm of the colour table and by nothing else.
/// The repaint the flag asks for costs nothing visible — a push button has drawn itself
/// idempotently since task T-12-6, so a repaint with nothing changed puts back the very pixels
/// that were there.
const ABOUT_BUTTONS: [i32; 1] = [OK_COMMAND];

/// The identifier of the button subclass — one number for all ten windows, because the key of
/// the subclass API is the (window, procedure, identifier) triple and the window is what varies.
/// Distinct from [`COMBO_SUBCLASS_ID`] and [`LIST_SUBCLASS_ID`] for readability alone; the pairs
/// could not collide anyway, being installed on different windows with different procedures.
const BUTTON_SUBCLASS_ID: usize = 3;

/// Puts [`button_proc`] in front of each button of `buttons` — task T-12-8.
///
/// **The pair.** Called exactly once per window — from [`fill_dialog`] for the settings dialog
/// and from the `WM_INITDIALOG` of [`about_proc`] for the about window — and the other half is
/// [`unsubclass_buttons`], called exactly once from the `WM_DESTROY` of the same procedure,
/// while the children are still alive. Both walk the same list with the same procedure and the
/// same [`BUTTON_SUBCLASS_ID`]; [`button_proc`] additionally takes itself off on `WM_NCDESTROY`,
/// which is the belt to that pair's braces. Every sentence of this paragraph is
/// [`subclass_combo_boxes`]'s, because this is deliberately the same arrangement and not a
/// second invention.
///
/// A refused install is survived (NFR-13): that button then behaves as it did before this task
/// — no response to the cursor — and everything else about it is unchanged. Not journaled, for
/// the reason [`subclass_combo_boxes`] writes down at its own dropped answer.
fn subclass_buttons(hwnd: HWND, buttons: &[i32]) {
    for &control in buttons {
        // SAFETY: `hwnd` is the live window; the crate turns a missing control into an error.
        let Ok(button) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
            crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
            continue;
        };

        // SAFETY: `button` is a live control of this window, created by the dialog manager on
        // this thread, and `button_proc` is a function of exactly the signature `SUBCLASSPROC`
        // names. The reference data is zero — this subclass keeps no state of its own; the one
        // thing it remembers lives in `HOT_CONTROL`, which is a thread-local and not a per
        // window field, for the reason written down there.
        let _ = unsafe { SetWindowSubclass(button, Some(button_proc), BUTTON_SUBCLASS_ID, 0) };
    }
}

/// Takes [`button_proc`] back off the buttons of `buttons` — the far half of the pair
/// [`subclass_buttons`] describes.
///
/// The refusal is deliberately **not** journaled, for the reason [`unsubclass_combo_boxes`]
/// gives: on `WM_DESTROY` a `FALSE` means the subclass was already gone — never installed, or
/// removed by the `WM_NCDESTROY` arm of the procedure — and neither is a fault.
fn unsubclass_buttons(hwnd: HWND, buttons: &[i32]) {
    for &control in buttons {
        // SAFETY: `hwnd` is the live window — `WM_DESTROY` reaches it before its children are
        // destroyed — and the crate turns a missing control into an error.
        let Ok(button) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
            continue;
        };

        // SAFETY: `button` is the live control the subclass was installed on, and the procedure
        // and identifier are the very pair `SetWindowSubclass` was given.
        let _ = unsafe { RemoveWindowSubclass(button, Some(button_proc), BUTTON_SUBCLASS_ID) };
    }
}

/// The procedure that stands in front of every owner-drawn push button while its window is up —
/// task T-12-8.
///
/// Three messages are its own and **not one of them is answered here**: each arm updates the
/// flag and then falls through to [`DefSubclassProc`], so a button behaves in every other way
/// exactly as it did before the subclass — it still tracks its own press, still answers the
/// keyboard, still notifies its parent. Nothing about the *drawing* happens here either: the
/// arms ask for a repaint, and the repaint goes the road it always went — the button's own
/// `WM_PAINT` sends `WM_DRAWITEM` to the dialog, which chooses the colours by the table.
///
/// # Safety
///
/// Called by the window manager with the arguments of a window message, on one of the controls
/// [`subclass_buttons`] installed it on.
unsafe extern "system" fn button_proc(
    button: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    _reference_data: usize,
) -> LRESULT {
    match message {
        // ⚠ The message that arrives by the dozen per second: see `enter_hot` for what it costs
        // when nothing has changed, which is the ordinary case.
        WM_MOUSEMOVE => enter_hot(button),

        // The far end of the notification `enter_hot` armed.
        WM_MOUSELEAVE => leave_hot(button),

        // The belt to the braces of the pair — see `subclass_buttons`. Forwarded on afterwards:
        // `WM_NCDESTROY` must reach every procedure of the chain.
        //
        // SAFETY: `button` is that control, and the procedure and identifier are the pair the
        // subclass was installed with.
        WM_NCDESTROY => {
            forget_hot(button);
            let _ = unsafe { RemoveWindowSubclass(button, Some(button_proc), BUTTON_SUBCLASS_ID) };
        }

        _ => {}
    }

    // SAFETY: the four arguments are the ones the window manager passed in, forwarded
    // unchanged; this is what `DefSubclassProc` exists for.
    unsafe { DefSubclassProc(button, message, wparam, lparam) }
}

// =========================================================================================
// Интерьер списка кончается скруглённой заливкой, а не прямым срезом — FR-92а, находка R-06
// протокола Э12, task T-12-5
// =========================================================================================
//
// ⚠ What was wrong, in numbers. The frame of a list is [`CORNER_RADIUS`] rounded and is drawn
// by the dialog's own background *outside* the control ([`paint_background`]); at 96 DPI that
// radius is 4 px and the frame stands 1 px off the control at the sides and the bottom and
// [`LIST_FIRST_ROW_TOP`] — 2 px through [`scaled`] — above it. So three of the four pixels each
// corner arc is made of fall inside the control, and the control fills its client area with a
// flat rectangle: measured on the stand before this task, the bottom-left corner of the
// exclusion list ran `392 = panel, 393 = field_bg` for three rows in a row — a square cut, no
// halftone anywhere. The five input fields do not have the fault: task T-12-3 gave their box
// 4 px of air above and below the control, which is the whole radius, so their arcs never reach
// the control at all. Measured, both halves, and written in the report of this task.
//
// ⚠ Why the repair is painted **last** and not at the erase. Because an erase is not the last
// thing a list paints: the rows come after it, and a row's ground is a flat `FillRect` across
// the full width of the control ([`draw_list_item`], [`draw_cycle_row`]). A list with enough
// rows to reach its own bottom edge would square the two bottom corners off again, which is
// exactly the state the scrolling check of this task puts it in. So the four corner squares go
// down after the control has finished drawing — and only those four squares, so that nothing
// the control drew between them is touched.

/// The **window** rectangle of a control, in that control's own **client** coordinates — task
/// T-12-5.
///
/// The two rectangles are not the same one, and the difference is exactly the non-client
/// furniture: a list showing a vertical scroll bar has a client area 17 px narrower than its
/// window, and a control with a border would have its client origin at a positive offset from
/// the window's. `GetClientRect` answers the first from zero and says nothing about either.
///
/// Worked out the documented way — `GetWindowRect` speaks screen coordinates, `ScreenToClient`
/// carries a screen point into the client coordinates of a named window — rather than assumed to
/// be `(0, 0, client_width, client_height)`, which is what it happens to be for a control with no
/// border and no scroll bar and is what made the scroll-bar case wrong.
///
/// `None` on a refused call (NFR-13): the caller then paints nothing, which leaves the corners as
/// the control drew them rather than putting a figure at a guessed place.
fn window_rect_in_client(control: HWND) -> Option<RECT> {
    let mut window = RECT::default();

    // SAFETY: `control` is the live control and `window` is a live local the call fills.
    unsafe { GetWindowRect(control, &mut window) }.ok()?;

    let mut origin = POINT {
        x: window.left,
        y: window.top,
    };

    // SAFETY: `control` is the live control and `origin` is a live local of this frame that the
    // call rewrites in place.
    if !unsafe { ScreenToClient(control, &mut origin) }.as_bool() {
        return None;
    }

    Some(RECT {
        left: origin.x,
        top: origin.y,
        right: origin.x + (window.right - window.left),
        bottom: origin.y + (window.bottom - window.top),
    })
}

/// Identifier of the subclass this section puts on the two lists — the pair (procedure,
/// identifier) that names it, exactly as [`COMBO_SUBCLASS_ID`] names the combo boxes'.
///
/// A number of its own and not the combos' 1: the key is per window and no window carries both,
/// but two subclasses of one file sharing an identifier is a trap for whoever adds the third.
const LIST_SUBCLASS_ID: usize = 2;

/// Puts [`list_proc`] in front of both lists — FR-92а, task T-12-5.
///
/// **The pair.** Called exactly once, from [`fill_dialog`] on `WM_INITDIALOG`; the other half is
/// [`unsubclass_lists`], called exactly once, from the `WM_DESTROY` branch of [`dialog_proc`],
/// while the children are still alive. Both walk the same [`FRAMED_LISTS`] list with the same
/// procedure and the same [`LIST_SUBCLASS_ID`], so every install has its removal by construction
/// rather than by discipline; [`list_proc`] additionally removes itself on `WM_NCDESTROY`, the
/// belt to that pair's braces. The two removals cannot double-free anything —
/// `RemoveWindowSubclass` on a window that no longer carries the subclass answers `FALSE` and
/// does nothing. Every sentence of this paragraph is [`subclass_combo_boxes`]'s, because this is
/// deliberately the same shape: one more procedure of the same kind, and not a second way of
/// doing it.
///
/// A refused install is survived (NFR-13): that list then keeps the square corners it has had
/// since T-11-13 — visibly imperfect rather than silently broken — and the dialog still opens.
/// Not journaled, for the reason [`subclass_combo_boxes`] writes down at its own dropped answer.
fn subclass_lists(hwnd: HWND) {
    for control in FRAMED_LISTS {
        // SAFETY: `hwnd` is the live dialog; the crate turns a missing control into an error.
        let Ok(list) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
            crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
            continue;
        };

        // SAFETY: `list` is a live control of this dialog, created by the dialog manager on this
        // thread, and `list_proc` is a function of exactly the signature `SUBCLASSPROC` names.
        // The reference data is zero — this subclass keeps no state of its own; everything it
        // needs it reads off the dialog through `with_state`.
        let _ = unsafe { SetWindowSubclass(list, Some(list_proc), LIST_SUBCLASS_ID, 0) };
    }
}

/// Takes [`list_proc`] back off both lists — the far half of the pair [`subclass_lists`]
/// describes.
///
/// The refusal is deliberately **not** journaled: this runs on `WM_DESTROY`, where a `FALSE`
/// means the subclass was already gone (never installed, or removed by the `WM_NCDESTROY` arm of
/// the procedure), and neither is a fault worth a journal row.
fn unsubclass_lists(hwnd: HWND) {
    for control in FRAMED_LISTS {
        // SAFETY: `hwnd` is the live dialog — `WM_DESTROY` reaches it before its children are
        // destroyed — and the crate turns a missing control into an error.
        let Ok(list) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
            continue;
        };

        // SAFETY: `list` is the live control the subclass was installed on, and the procedure
        // and identifier are the very pair `SetWindowSubclass` was given.
        let _ = unsafe { RemoveWindowSubclass(list, Some(list_proc), LIST_SUBCLASS_ID) };
    }
}

/// The procedure that stands in front of each of the two lists while this dialog is up —
/// FR-92а, task T-12-5.
///
/// ⚠ **Nothing is intercepted.** Every message, `WM_PAINT` included, reaches the control's own
/// procedure first and unchanged, and its answer is the answer this procedure gives back. The
/// control still erases its ground, lays out its rows, draws its selection, moves its scroll bar
/// and answers every `LB_*`, `LVM_*` and keyboard message exactly as it did before the subclass —
/// which is what keeps the ⛔ list of this task (the selection stripe of T-11-25, the ticks of
/// T-11-25-2, the scrolling, the row insets and the row height) untouched by construction rather
/// than by inspection. The one thing added is four corner squares painted *after* the control
/// has finished a `WM_PAINT`.
///
/// `WM_NCDESTROY` is answered before the forwarding and forwarded on afterwards: the message
/// must reach every procedure of the chain.
///
/// # Safety
///
/// Called by the window manager with the arguments of a window message, on one of the two
/// controls [`subclass_lists`] installed it on.
unsafe extern "system" fn list_proc(
    list: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    _reference_data: usize,
) -> LRESULT {
    if message == WM_NCDESTROY {
        // The belt to the braces of the pair — see `subclass_lists`.
        //
        // SAFETY: `list` is that control, and the procedure and identifier are the pair the
        // subclass was installed with.
        let _ = unsafe { RemoveWindowSubclass(list, Some(list_proc), LIST_SUBCLASS_ID) };
    }

    // SAFETY: the four arguments are the ones the window manager passed in, forwarded
    // unchanged; this is what `DefSubclassProc` exists for.
    let answer = unsafe { DefSubclassProc(list, message, wparam, lparam) };

    if message == WM_PAINT {
        // SAFETY: `list` is the control this procedure is installed on, and its own `WM_PAINT`
        // has just returned — the update region is validated and the drawing is finished, which
        // is the moment the patch is for.
        unsafe { patch_list_corners(list) };
    }

    answer
}

/// Lays the four rounded corners of the list's frame over the interior the control has just
/// painted flat — the whole of what [`list_proc`] adds, and the whole of task T-12-5.
///
/// # The DC
///
/// `GetDC` and not `BeginPaint`: the control's own `WM_PAINT` has already run and validated the
/// window, and a second `BeginPaint` on a window with nothing left to update hands back an empty
/// clip — the corners would simply not be painted.
///
/// ⚠ Everything that can refuse is settled **before** the DC is taken — the parent, the
/// identifier, the client rectangle, the window rectangle, the colours — so that between `GetDC`
/// and `ReleaseDC` there is exactly one path and no way off it. The one `return` inside that
/// stretch is the refusal of `GetDC` itself, where no DC was taken and there is nothing to
/// release. A leaked DC gives neither an error nor a red test, so the shape is held by a test
/// that reads it out of this source rather than by care.
///
/// # The figure
///
/// [`list_frame_box`] in the control's own client coordinates — the very frame
/// [`paint_background`] draws around this control from the other side, named through the same
/// [`list_frame_air`] — and [`CORNER_RADIUS`] through [`scaled`], the radius every panel, button
/// and field of this dialog wears. Not one number of the figure is invented here.
///
/// Nothing is drawn at all when the state is unreachable or `theme::Brushes::new` was refused at
/// initialisation — the reason [`draw_combo_closed_part`] gives for its own silence (NFR-13).
///
/// # Safety
///
/// Called from [`list_proc`] alone, on one of the two controls [`subclass_lists`] installed it on.
unsafe fn patch_list_corners(list: HWND) {
    // The dialog is the parent of its own control; the colours are read through it, by
    // identifier, exactly as every other drawing of this file reads what it draws.
    //
    // SAFETY: `list` is the live control; a window with no parent answers an error.
    let Ok(dialog) = (unsafe { GetParent(list) }) else {
        return;
    };

    // SAFETY: reading a window's identifier reads a field of that window and no memory of ours.
    let control = unsafe { GetDlgCtrlID(list) };

    // The same gate the owner-draw handlers keep (SEC-05), even though this procedure can only
    // be reached on a window it was installed on: one list of two, checked before any work.
    if !FRAMED_LISTS.contains(&control) {
        return;
    }

    let mut client = RECT::default();

    // SAFETY: `list` is the live control and `client` is a live local the call fills.
    if unsafe { GetClientRect(list, &mut client) }.is_err() {
        return;
    }

    // ⚠ And the **window** rectangle as well, which is not the same rectangle and was measured
    // not to be: a list that grows more rows than it can show puts up a vertical scroll bar, the
    // scroll bar is non-client, and the client area then ends 17 px short of the window's right
    // edge. The frame [`paint_background`] draws is hung on the *window*, so basing the patch on
    // the client alone drew a perfectly good rounded corner 17 px inside the list, hard against
    // the scroll bar — measured on the stand with nine exclusions: an arc at columns 583, 584,
    // 585 where the list is still list. The frame is asked for where it really is instead.
    let Some(window) = window_rect_in_client(list) else {
        return;
    };

    // The colour choice, split from the painting exactly as everywhere in this file: the borrow
    // of the state ends before any DC is taken, and what leaves the closure is plain values —
    // two brushes the state keeps alive until the dialog ends, and one ink.
    //
    // SAFETY: `dialog` is the parent of one of this dialog's own controls, which is the window
    // `show_dialog` created — the contract of `with_state`.
    let choice = unsafe {
        with_state(dialog, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13, T-11-4).
            let brushes = state.brushes.as_ref()?;

            Some(CornerColors {
                // The ground the corners the rounding cuts away are left standing on — the very
                // rule `on_ctl_color` answers `WM_CTLCOLORBTN` with, and the one
                // `draw_combo_closed_part` asks for its own corners. Both lists do lie on a
                // panel, but the rule is asked and not assumed.
                ground: if state.panel_children.contains(&control) {
                    brushes.panel_bg()
                } else {
                    brushes.window_bg()
                },
                outline: state.palette.field_border,
                fill: brushes.field_bg(),
            })
        })
    };

    let Some(Some(colors)) = choice else {
        return;
    };

    // SAFETY: `list` is the live control; the call answers its DC or an invalid handle, and the
    // DC is released below — the single road out of this function from here on.
    let dc = unsafe { GetDC(Some(list)) };

    // NFR-13: examined. Nothing of ours can be drawn without a DC, and there is nothing to
    // release either — the list keeps the square corners of before this task for that repaint.
    if dc.is_invalid() {
        return;
    }

    let dpi = dc_dpi(dc);

    // The figure is the frame of the **window**; `client` is only the licence — what of that
    // figure this DC is allowed to write. A corner that falls wholly in the non-client scroll bar
    // is clamped to nothing and left to the system, which is the honest answer: those pixels are
    // not this program's to paint from a client DC.
    paint_rounded_corners(
        dc,
        &list_frame_box(&window, dpi),
        &client,
        scaled(CORNER_RADIUS, dpi),
        colors,
        dpi,
    );

    // ⚠ The one `ReleaseDC` of this function, and it is reached from every path that took a DC:
    // there is no `?`, no early `return` and no `panic!` between the two calls — only pure
    // arithmetic and drawing that swallows its own refusals. A leaked DC gives neither an error
    // nor a red test, which is why this is written down rather than assumed (NFR-13).
    //
    // SAFETY: releases exactly the DC taken above, once, for the window it was taken for.
    let released = unsafe { ReleaseDC(Some(list), dc) };

    debug_assert_eq!(
        released, 1,
        "ReleaseDC отказал на DC, взятом этой же функцией — это утечка"
    );
}

/// Air between the text of the closed part and the chevron, in mock-up pixels.
pub const COMBO_CHEVRON_TEXT_GAP: i32 = 4;

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
    // FR-92а, task T-12-5, finding R-06. The other half of this pair — `unsubclass_lists` — is
    // the same `WM_DESTROY` branch of `dialog_proc` the combo boxes' removal stands in, and
    // nothing else in the file installs or removes this subclass either.
    subclass_lists(hwnd);
    // Task T-12-8: the response of the cursor on the nine push buttons. The other half of this
    // pair — `unsubclass_buttons` — stands in the same `WM_DESTROY` branch as the two above,
    // walking the same `PUSH_BUTTONS` list.
    subclass_buttons(hwnd, &PUSH_BUTTONS);
    // п. 2 of task T-11-15: the text of the five input fields, off the frame by the inset of
    // the mock-ups. Once, here — a margin is a property of the control, not of a paint.
    set_field_margins(hwnd);
    // Task T-12-2: the closed part of the four combo boxes, brought to the 12 dialog units of
    // the mock-ups apart from the items of their dropped-down lists. Once, here — a height is
    // a property of the control like the margin above, and this is the first moment all four
    // controls exist: `WM_MEASUREITEM` arrived before this window had a `WM_INITDIALOG` at
    // all. Before anything is put into any of them, so no combo box is ever holding a list at
    // one height and about to be measured at another.
    //
    // ⚠ NFR-13: the answer of every send is examined inside the function — a `CB_ERR` there
    // leaves that one combo box at the taller closed part it already had, and the other three
    // still take theirs. The count comes back so that the examination is a number rather than
    // a promise; nothing in this window depends on it, and no `debug_assert` stands on it
    // either, for the reason `subclass_combo_boxes` writes down at its own dropped answer: a
    // machine that refused is a legal, degraded state and not a broken invariant.
    let _taken = set_combo_closed_height(hwnd);

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

    // FR-92а, task T-11-7: the list carries palette state of its own — the three colours,
    // set here for the first showing and again by `apply_now` on every palette change — and a
    // state image list, which since task T-11-25 carries no colour at all and is therefore
    // built once, here. After `prepare_cycle_list`: the extended style must exist before the
    // system pair it creates can be replaced.
    paint_cycle_list(hwnd, state.palette);
    install_check_images(hwnd);

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
        // the three `LVM_SET*COLOR` colours — which no brush recreation reaches,
        // and it is handed the fresh palette here, so the repaint below shows one
        // whole dialog. The state image list is *not* rebuilt with it since task
        // T-11-25: its cells carry no colour any more, only the size of the row,
        // and the tick they used to hold is painted from the palette on every
        // repaint by `draw_cycle_row`.
        paint_cycle_list(hwnd, fresh);

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
/// of the window it is built for — task T-11-16, the pure half of [`check_glyph`].
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
    /// Width of the left edge of the row **as the mock-ups lay it out** — air, tick, air.
    ///
    /// This is where the label of the mock-ups begins, and it is the number the design-token
    /// table of task T-11-16 reads. It is **not** the width the state image list is built at
    /// any more — see [`CheckCell::image_width`].
    pub width: i32,
    /// Width the cell of the state image list is actually made — [`CheckCell::width`] less
    /// [`LVIEW_LABEL_INDENT`], task T-12-7.
    ///
    /// The control puts the label a fixed distance after the cell, not at the cell's own right
    /// edge, so a cell as wide as the mock-up's left edge lands the label that distance too far
    /// right. Taking the distance out of the **cell** is what puts the label back where the
    /// generator draws it, and it moves nothing else: the tick is painted on the row by
    /// [`draw_cycle_row`] at [`CheckCell::glyph_left`], which this does not touch, and the tick
    /// still ends inside the narrowed cell — 5 + 9 = 14 against 15 at 96 DPI — so the square is
    /// still the click target `LVS_EX_CHECKBOXES` toggles the row by.
    ///
    /// Never below one: `ImageList_Create` of a zero-width cell would answer nothing, and the
    /// control would fall back to the system's own 16 × 16 pair (NFR-13).
    pub image_width: i32,
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
    let width = glyph_left + glyph_side + scaled(LIST_CHECK_TEXT_GAP, dpi);

    CheckCell {
        width,
        image_width: (width - LVIEW_LABEL_INDENT).max(1),
        height,
        glyph_left,
        glyph_top: (height - glyph_side) / 2,
        glyph_side,
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
/// Colours of the given palette rather than roles, as [`check_frame_colors`] of `theme` and
/// for the same reason; every answer is a field of `palette` and nothing else, so not a single
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
///
/// # ⚠ The third colour is `CLR_NONE` since task T-11-25
///
/// «To make the text background transparent» is what the documentation of `LVM_SETTEXTBKCOLOR`
/// says `CLR_NONE` is for, and transparency is exactly what this list now needs: the ground of a
/// row is no longer a rectangle the control fills, it is the figure [`draw_cycle_row`] paints in
/// the item prepaint — a rounded stripe inset into the row, which any opaque fill of the control
/// would erase the moment the label went down over it. With `CLR_NONE` the control fills nothing
/// and only writes the letters, so what stays behind them is the drawing this module made.
///
/// The ground *of the control* — `LVM_SETBKCOLOR`, the first row above — is untouched and stays
/// `field_bg`: it is what the list wears below and around the rows, and it is also the erase
/// every repaint starts from, which is the honest ground the stripe is inset into.
fn paint_cycle_list(hwnd: HWND, palette: &theme::Palette) {
    // NFR-13: each message answers a success flag, examined in words and dropped: a refused
    // colour leaves the system one on exactly that surface — the dialog lives degraded, the
    // precedent of the refused brushes of T-11-4 — and the journal has no row for cosmetics
    // (reviews\T-11-1.md).
    for (message, value) in [
        (
            LVM_SETBKCOLOR,
            isize::try_from(palette.field_bg.0).unwrap_or(0),
        ),
        (
            LVM_SETTEXTCOLOR,
            isize::try_from(palette.text.0).unwrap_or(0),
        ),
        (LVM_SETTEXTBKCOLOR, isize::try_from(CLR_NONE).unwrap_or(0)),
    ] {
        send_to(hwnd, IDC_CYCLE_LIST, message, 0, value);
    }

    // `LVM_SETBKCOLOR` above just wrote `field_bg` into the cells of the state image list as
    // well — measured, see [`clear_state_image_ground`] — so the ground comes out again here,
    // and it has to come out **here** and not only where the list is installed: this is the
    // whole of what a palette change sends the list, and after it the cells would otherwise be
    // opaque again.
    clear_state_image_ground(hwnd);
}

/// Takes the ground out of the cells of the state image list of the layout list — FR-92а,
/// task T-11-25-2.
///
/// # What is measured here, and why the tick needs it
///
/// Since task T-11-25 the two cells of that list hold no picture at all: every pixel of both
/// is the key colour, so the mask makes each cell a hole edge to edge and the tick is painted
/// on the row instead ([`draw_cycle_row`]). A hole draws nothing — **provided the image list
/// has no background colour of its own.** `ImageList_Draw` with `ILD_NORMAL` fills the whole
/// cell rectangle with that colour first and blits the picture over it; only `CLR_NONE` means
/// «no fill, use the mask», which is what a freshly created list carries.
///
/// A list view does not leave it that way. Measured on the live control through the stand,
/// with the layout list holding this module's own two cells:
///
/// * straight after `LVM_SETIMAGELIST` the cell colour is `0x00211c19` — `field_bg` of the
///   graphite palette, the very value `LVM_SETBKCOLOR` was given;
/// * sent `LVM_SETBKCOLOR` again with `0x00112233`, the cell colour becomes `0x00112233`.
///
/// So the control writes its own erase colour into the cells of whatever state image list it
/// is holding, on both messages. With it there, the state image is a 19 × 23 opaque rectangle
/// of `field_bg` painted **after** the item prepaint of [`on_notify`] — it punched the tick and
/// a bite of the selection stripe out of every row, which is half of what task T-11-25-2
/// repairs. `CLR_NONE` is the documented way back and the only one this needs.
///
/// The handle is asked of the control (`LVM_GETIMAGELIST`, `LVSIL_STATE`) rather than
/// remembered, so this works the same for the pair `LVS_EX_CHECKBOXES` made and for the pair
/// [`install_check_images`] put in its place, and answers zero when there is no list at all.
fn clear_state_image_ground(hwnd: HWND) {
    let list = send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_GETIMAGELIST,
        usize::try_from(LVSIL_STATE).unwrap_or(0),
        0,
    );

    if list == 0 {
        // No state image list to clear — nothing to do, and nothing degraded either: without
        // one there is no cell to paint over the row (NFR-13).
        return;
    }

    // SAFETY: `list` is the handle the control just answered for its own state image list; the
    // call writes one field of it and follows no pointer of ours. The previous colour is
    // dropped — it is the value measured above, and nothing needs it back.
    let _ = unsafe { ImageList_SetBkColor(HIMAGELIST(list), COLORREF(CLR_NONE as u32)) };
}

/// Replaces the state image list of the layout list with two empty cells — FR-92а, task
/// T-11-7 for the replacement, task T-11-25 for the cells being empty.
///
/// `LVS_EX_CHECKBOXES` draws its ticks with the system pair, which stays light in the dark
/// palette (§10 п.9 called that the accepted price). Task T-11-7 lifted that by painting the
/// two palette frames into the cells; task T-11-25 took the picture out of them again and
/// moved it onto the row itself ([`draw_cycle_row`]), because the edge of a picture inside a
/// masked image list cannot be smoothed and the alpha route that would have smoothed it is not
/// available on the comctl32 this program runs on — the measurement is in the doc comment of
/// [`on_notify`].
///
/// So what the list installed here still does is **measure**, and that is not a leftover: a
/// report list view takes its row height from the cell of its state image list and from nowhere
/// a message can reach, and the width of the cell is the column the label starts after. Both are
/// [`check_cell`], and both would go back to the system's 16 × 16 if this list were dropped.
///
/// Everything else about the check boxes is untouched: the extended style stays on the
/// control, so clicking a square still flips the item between state images 1 and 2, and
/// [`set_row_check`]/[`read_cycle_checks`] keep speaking `LVIS_STATEIMAGEMASK` — the
/// participation mechanism of FR-31 is the same mechanism, and it now picks a picture for
/// [`draw_cycle_row`] to paint instead of a picture for the control to blit.
fn install_check_images(hwnd: HWND) {
    // The height of a row of the layout list — [`LAYOUT_ROW_HEIGHT_DLU`] dialog units of this
    // window (task T-11-16). It travels down the build because the **cell** of the state image
    // list is what a report list view takes its row height from; a refused `MapDialogRect`
    // leaves the cell as tall as the tick, which is the height the control had before this task
    // (NFR-13).
    //
    // Less [`LVIEW_ROW_OVERHEAD`] since task T-12-7: the control does not make the row *equal*
    // to the cell, it makes it one pixel taller — so the twelve dialog units of the mock-ups
    // come out of the control as twelve dialog units only if the cell is handed one less. Never
    // negative: a refused `MapDialogRect` answers zero above, and zero less one would be an
    // `ImageList_Create` of a negative height (NFR-13).
    let row_height = dialog_units(hwnd, 0, LAYOUT_ROW_HEIGHT_DLU)
        .map(|(_, vertical)| (vertical - LVIEW_ROW_OVERHEAD).max(0))
        .unwrap_or(0);

    let Some(list) = build_check_image_list(row_height) else {
        // NFR-13, examined in words: with no cells of our own the system pair simply stays —
        // its own row height, its own column, and its light squares drawn over the tick this
        // dialog paints — which is a coarser list rather than no list. No journal row for
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

    // The first swap answers the pair `LVS_EX_CHECKBOXES` created, and it is ours to free
    // once the control has let go. The list *currently* installed is deliberately never
    // destroyed here: the
    // control destroys the image lists it holds when it is itself destroyed — the template
    // carries no `LVS_SHAREIMAGELISTS`, which is the one style that would keep it from
    // doing so.
    // The control has just painted its own ground into the cells: `LVM_SETIMAGELIST` hands the
    // list the colour the control erases with. Taken straight back out — see
    // [`clear_state_image_ground`], and the measurement in its doc comment.
    clear_state_image_ground(hwnd);

    if previous != 0 {
        // SAFETY: `previous` is the handle the control just answered and no longer holds;
        // it is freed exactly once, here. The `BOOL` is examined in words and dropped —
        // a refusal would mean the handle was not a live image list of this process, which
        // the swap above makes unreachable (NFR-13).
        let _ = unsafe { ImageList_Destroy(Some(HIMAGELIST(previous))) };
    }
}

/// Builds the two-cell state image list of [`install_check_images`] — the outer layer of
/// three: takes and releases the screen DC, which is where the colour depth of the cells
/// comes from.
///
/// Public since task T-11-25 for the reason [`paint_rounded`] is: a test builds the list, draws
/// both cells over a ground of its own in a memory bitmap and reads the pixels back, which is
/// how «весь кадр — дыра» is held closed without a window and without starting the product.
///
/// ⚠ The answer is owned by the caller: it is either handed to a control with `LVM_SETIMAGELIST`
/// — which takes it over, see [`install_check_images`] — or destroyed with `ImageList_Destroy`.
pub fn build_check_image_list(row_height: i32) -> Option<HIMAGELIST> {
    // SAFETY: the screen DC of this process; released below, on every path.
    let screen = unsafe { GetDC(None) };

    if screen.is_invalid() {
        // NFR-13: examined — no DC, no cells; the caller words the degradation.
        return None;
    }

    let list = build_check_frames(screen, row_height);

    // SAFETY: releases exactly the DC taken above, once.
    unsafe { ReleaseDC(None, screen) };

    list
}

/// The middle layer of [`build_check_image_list`]: owns the memory DC the cells are filled
/// through. ⚠ The bitmaps are compatible with the **screen**, not with this DC: a memory DC
/// is born with a monochrome bitmap selected, and a bitmap compatible with *it* would carry
/// one bit per pixel — the classic trap the task's «в память» route walks past.
fn build_check_frames(screen: HDC, row_height: i32) -> Option<HIMAGELIST> {
    // SAFETY: a memory DC over the live screen DC; deleted below, on every path.
    let dc = unsafe { CreateCompatibleDC(Some(screen)) };

    if dc.is_invalid() {
        // NFR-13: examined — as in the caller.
        return None;
    }

    let list = draw_frames_into_list(screen, dc, row_height);

    // SAFETY: deletes exactly the DC created above, once; the cell bitmaps were deselected
    // before their own deletion, so nothing of ours is still selected into it.
    let _ = unsafe { DeleteDC(dc) };

    list
}

/// The colour the whole cell of the state image list is filled with — task T-11-15 for the
/// mechanism, task T-11-25 for the cell being nothing else.
///
/// `ImageList_AddMasked` is the documented way to a mask: it builds one from the bitmap
/// itself, turning every pixel of this colour into a hole the row shows through. Until task
/// T-11-25 the tick was painted over this colour and only the air around it was a hole, which
/// is why the square could not be smoothed: a half-covered edge pixel is a blend of the key and
/// the fill, no longer *exactly* the key, so no longer a hole — a magenta fringe on every row.
/// Now the cell carries this colour and nothing else, the whole of it is a hole, and the tick is
/// painted onto the row by [`draw_cycle_row`], where nothing has to match a colour exactly and
/// the edge blends into the ground the row actually wears.
///
/// The colour is therefore never seen. Magenta is the traditional key, and it stays that:
/// `ImageList_AddMasked` needs *some* colour to build a mask from, and one that no palette owns
/// keeps the cell readable as «пусто» rather than as a colour someone might mistake for a paint.
const CHECK_CELL_KEY: COLORREF = COLORREF(0x00FF_00FF);

/// The inner layer of [`build_check_image_list`]: the image list itself and the two cells,
/// one per entry of [`CHECK_FRAME_ORDER`]. Any refusal destroys the half-built list and
/// answers `None` — a one-cell list would silently shift the meaning of state image 2.
///
/// The two cells are identical and empty; the order constant is still what counts them, because
/// what it says — one cell per state image index, «снята» first — is exactly what has to stay
/// true for the participation bits of FR-31 to keep landing on a cell that exists.
fn draw_frames_into_list(screen: HDC, dc: HDC, row_height: i32) -> Option<HIMAGELIST> {
    // The whole cell, in the pixels of the screen it is made for — the DPI of the
    // screen DC this whole build hangs from (NFR-13: a DC that will not say is answered as 96
    // by `dc_dpi`, which is the 100 % cell).
    let dpi = dc_dpi(screen);
    let cell = check_cell(dpi, row_height);

    // SAFETY: plain numbers in, a handle out, owned by this frame until it is either handed
    // to the caller or destroyed below. `ILC_MASK` beside `ILC_COLOR32` — every pixel of the
    // cell is a hole, which is what [`CHECK_CELL_KEY`] is for.
    //
    // `image_width` and not `width`: the control adds [`LVIEW_LABEL_INDENT`] of its own after
    // the cell before it starts the label, task T-12-7.
    let list =
        unsafe { ImageList_Create(cell.image_width, cell.height, ILC_COLOR32 | ILC_MASK, 2, 0) };

    if list.is_invalid() {
        // NFR-13: examined — as in the callers.
        return None;
    }

    for _ in CHECK_FRAME_ORDER {
        // SAFETY: compatible with the *screen* DC — see the caller's ⚠ — and owned by this
        // frame until the `DeleteObject` below.
        let bitmap = unsafe { CreateCompatibleBitmap(screen, cell.image_width, cell.height) };

        if bitmap.is_invalid() {
            // SAFETY: the half-built list is ours until handed out; freed exactly once.
            let _ = unsafe { ImageList_Destroy(Some(list)) };
            return None;
        }

        // SAFETY: both handles are live and ours; the previous bitmap is kept and put back
        // below — `ImageList_AddMasked` reads the bitmap's bits, and a bitmap still selected
        // into a DC is not readable.
        let previous = unsafe { SelectObject(dc, bitmap.into()) };

        fill_check_cell(dc, cell);

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

/// Fills one cell of the state image list, in the bitmap currently selected into `dc`, with
/// [`CHECK_CELL_KEY`] and nothing else — the whole of what a cell carries since task T-11-25.
///
/// The brush is transient, exactly as the pens of [`draw_check_mark`]: nothing here outlives
/// the paint, so nothing belongs in `theme::Brushes`, whose reason to exist is answers that
/// must outlive it.
fn fill_check_cell(dc: HDC, cell: CheckCell) {
    let whole = RECT {
        left: 0,
        top: 0,
        right: cell.image_width,
        bottom: cell.height,
    };

    // SAFETY: a plain colour in, a handle out, owned by this frame until the `DeleteObject`
    // below.
    let key = unsafe { CreateSolidBrush(CHECK_CELL_KEY) };

    if key.is_invalid() {
        // NFR-13: examined — no brush, no fill; the mask would then come out of whatever the
        // fresh bitmap held, and the cell would blit that over the row instead of being a hole.
        // The honest answer is to leave the cell alone and let the callers' degradation words
        // cover it: a coarse cell over the tick, not a lost tick.
        return;
    }

    // SAFETY: `dc` holds the cell bitmap for exactly this call; `key` is the live brush just
    // made. The answer is dropped for the NFR-13 reason `draw_glyph_element` gives.
    unsafe { FillRect(dc, &whole, key) };

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(key.into()) };
}

/// Everything one row of the layout list wears that the control does not draw itself — the
/// ground, the selection figure on it and the tick — FR-92а, task T-11-25.
///
/// # The three layers, in the order they go down
///
/// 1. **The ground** — the pair an *ordinary* row of this mode wears, out of the very same
///    [`cycle_row_paint`] table the ink comes from: `field_bg` in both modes, and the fallback
///    for a table row that answers nothing is `field_bg` too, because that is what
///    `LVM_SETBKCOLOR` already erased the control with.
/// 2. **The selection** — exactly what the table answers *differently* for a selected row, and
///    nothing where it answers the same. That is the whole rule, and it is the table's own
///    sentence rather than a second copy of it: in the cycle mode a selected row takes `sel_bg`
///    where an ordinary one takes nothing, so the stripe is drawn; in the pair mode a selected
///    row takes precisely what every other row takes — the logical «выключенность» of task
///    T-11-7-2 — so there is no stripe at all, and FR-31's «выделение в нём не появляется»
///    holds by construction instead of by a second condition that could drift from the table.
///    The figure is [`paint_selection_stripe`], the one both lists of this dialog share.
/// 3. **The tick** — [`check_glyph`] for where it goes, [`draw_check_glyph`] for what it is.
///
/// # Why the tick is here and not in the image list any more
///
/// Because the edge of a picture inside a masked image list cannot be smoothed, and the alpha
/// route that would have replaced the mask was measured on this machine and does not work: see
/// [`CHECK_CELL_KEY`] for the first half and the doc comment of [`on_notify`] for the second.
/// Drawn here the square blends into the ground the row actually wears — `field_bg` under an
/// ordinary row, `sel_bg` under the selection — which no baked-in ground could have done either.
///
/// Public for the reason [`paint_rounded`] is: a test draws a row into a memory bitmap and reads
/// the pixels back, which is how both halves of this task are held closed without a window and
/// without starting the product.
pub fn draw_cycle_row(
    dc: HDC,
    row: &RECT,
    mode: LayoutMode,
    selected: bool,
    checked: bool,
    palette: &theme::Palette,
    dpi: i32,
) {
    let plain = cycle_row_paint(mode, false, palette);
    let paint = cycle_row_paint(mode, selected, palette);

    let ground = plain.colours.map_or(palette.field_bg, |(ground, _)| ground);

    // SAFETY: a plain colour in, a handle out, owned by this frame until the `DeleteObject`
    // below.
    let brush = unsafe { CreateSolidBrush(ground) };

    if brush.is_invalid() {
        // NFR-13: examined — no brush, no ground, and nothing on top of a ground that was not
        // laid is honest either: the row keeps the `LVM_SETBKCOLOR` erase, which is `field_bg`,
        // and loses its selection and its tick for one repaint.
        return;
    }

    // SAFETY: `dc` is painted into for the length of the send this call is inside of, and `row`
    // is a live rectangle of the caller's frame; `brush` is the live brush just made. The answer
    // is dropped for the NFR-13 reason `draw_list_item` states for all its paint calls.
    unsafe { FillRect(dc, row, brush) };

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(brush.into()) };

    if paint.colours != plain.colours
        && let Some((selection, _)) = paint.colours
    {
        // SAFETY: as for the ground brush above.
        let fill = unsafe { CreateSolidBrush(selection) };

        if !fill.is_invalid() {
            // NFR-13: a refused brush leaves the ground of the row and no stripe — the row is
            // still readable, and the ink of the selection pair is already on it.
            paint_selection_stripe(dc, row, selection, fill, dpi);

            // SAFETY: created above, handed to nobody, freed exactly once.
            let _ = unsafe { DeleteObject(fill.into()) };
        }
    }

    draw_check_glyph(dc, &check_glyph(row, dpi), checked, palette, dpi);
}

/// The square of the tick inside one row of the layout list — task T-11-25.
///
/// The control draws the state image at the left edge of the row, so the cell of
/// [`check_cell`] *is* the left edge of the row: the air before the tick, the tick, the air
/// after it. Taking the geometry from the same pure function the image list is built from is
/// what keeps the tick this module paints on top of the column the control lays out around —
/// the click that toggles a check box lands anywhere in that cell, and the label starts after
/// it.
///
/// The row's own height is what the tick is centred in, which is the same number
/// [`install_check_images`] hands the cell: a report list view takes its row height from the
/// cell and from nowhere else. Pure.
pub fn check_glyph(row: &RECT, dpi: i32) -> RECT {
    let cell = check_cell(dpi, row.bottom - row.top);

    RECT {
        left: row.left + cell.glyph_left,
        top: row.top + cell.glyph_top,
        right: row.left + cell.glyph_left + cell.glyph_side,
        bottom: row.top + cell.glyph_top + cell.glyph_side,
    }
}

/// One tick of the layout list, in the colours of [`check_frame_colors`] — task T-11-16 for
/// the figure, task T-11-7 for the colours, task T-11-25 for the ground it is drawn on.
///
/// The brush is transient, exactly as the pens of [`draw_check_mark`]: nothing here outlives
/// the paint, so nothing belongs in `theme::Brushes`, whose reason to exist is answers that
/// must outlive it. The square is rounded by [`LIST_CHECK_CORNER_RADIUS`] and the mark is
/// [`draw_check_mark`] with [`LIST_CHECK_MARK`] — the tick of the list has its own figure in
/// the mock-ups, one size smaller than the tick of a dialog check box.
///
/// [`paint_rounded`] and no longer `theme::stroke_rounded`: the square is smoothed like every other
/// figure of this dialog now that it is painted on the row and not into a cell whose mask makes
/// a hole of one exact colour. `FillRectPx … 2` and `StrokeRectPx … 2 1` of the `'lview'` arm
/// are one `RoundRect` inside it, exactly as the glyph of a dialog check box is drawn: a square
/// `FillRect` under a square `FrameRect` cannot have a corner radius at all, and the frameless
/// picture — the checked one, whose accent fill covers it whole — is outlined in its own fill.
///
/// `dpi` is the DPI of the window, passed in rather than read off `dc` for the reason
/// [`paint_rounded`] states.
fn draw_check_glyph(dc: HDC, glyph: &RECT, checked: bool, palette: &theme::Palette, dpi: i32) {
    let colors = check_frame_colors(checked, palette);

    // SAFETY: a plain colour in, a handle out, owned by this frame until the `DeleteObject`
    // below.
    let fill = unsafe { CreateSolidBrush(colors.fill) };

    if fill.is_invalid() {
        // NFR-13: examined — no brush, no square; the row keeps its ground and loses its tick
        // for one repaint, and the callers' degradation words cover it.
        return;
    }

    paint_rounded(
        dc,
        glyph,
        scaled(LIST_CHECK_CORNER_RADIUS, dpi),
        colors.frame.unwrap_or(colors.fill),
        fill,
        dpi,
    );

    // SAFETY: created above, handed to nobody, freed exactly once — see `draw_check_mark`
    // on the dropped `BOOL`.
    let _ = unsafe { DeleteObject(fill.into()) };

    if let Some(ink) = colors.mark {
        draw_check_mark(dc, glyph, ink, LIST_CHECK_MARK, dpi);
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
    let column = LVCOLUMNW {
        mask: LVCF_WIDTH,
        cx: cycle_column_width(hwnd),
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

/// The width the single column of the layout list is to have **right now** — the client width
/// of the control, and nothing taken off it, task T-12-7 (backlog item T-11-26).
///
/// # The twenty pixels that were taken off here, and why they were a second helping
///
/// Until this task the column was created `client_width − 20`, with «room for the vertical
/// scroll bar» for the reason. Measured on the stand, `dlgstand dark cycle`, the layout list of
/// this dialog:
///
/// | state | `GetClientRect` | `GetWindowRect` | `WS_VSCROLL` |
/// |---|---|---|---|
/// | two rows | 249 | 249 | no |
/// | twelve rows | **232** | 249 | yes |
///
/// `GetClientRect` **already** takes the scroll bar off — 17 px of it — because a scroll bar of
/// a control is non-client area. Subtracting a second twenty left the selection stripe ending
/// 21 px short of the frame with no scroll bar there to fill the gap, which is finding
/// **BLIND-3** of the E12 protocol; the mock-ups pull the stripe in by 2 mock-up pixels
/// ([`LIST_SELECTION_INSET`]) and by nothing else.
///
/// So the honest width is the client width itself, and the whole of the scroll-bar question is
/// already answered by the number the control hands over. Measured the same way, with twelve
/// rows and the column set to the client 232: `WS_HSCROLL` stays **off** — a column exactly as
/// wide as the client scrolls nowhere — while a column of 400 puts a horizontal bar up at once
/// and eats 17 px of the client height with it.
///
/// ⚠ **Order.** The client width is not a constant of the dialog: it changes the moment the
/// vertical scroll bar appears, and that happens when the list is **filled**, long after the
/// column was created. That is why [`fill_cycle_list`] asks again — see [`fit_cycle_column`].
///
/// A refused `GetClientRect` answers the 200 px the column was created with before this task
/// (NFR-13), and the floor of 40 stays: a column narrower than that would hide the tick itself.
fn cycle_column_width(hwnd: HWND) -> i32 {
    client_width(hwnd, IDC_CYCLE_LIST).unwrap_or(200).max(40)
}

/// Puts the single column of the layout list back on the client width the control answers
/// **now** — task T-12-7, the trap named in [`cycle_column_width`].
///
/// Called from [`fill_cycle_list`] and therefore after every change of the number of rows: the
/// vertical scroll bar comes and goes with that number, and with it 17 px of client width. Both
/// directions matter — a list that has just lost its scroll bar has 17 px of dead field on the
/// right until the column is widened again, which is the same defect the other way round.
fn fit_cycle_column(hwnd: HWND) {
    // NFR-13: the answer is the flag `LVM_SETCOLUMNWIDTH` returns, examined in words and
    // dropped — a refused width leaves the column as it was, which is a stripe that stops
    // short rather than a list that does not work. No journal row for cosmetics
    // (reviews\T-11-1.md).
    send_to(
        hwnd,
        IDC_CYCLE_LIST,
        LVM_SETCOLUMNWIDTH,
        0,
        isize::try_from(cycle_column_width(hwnd)).unwrap_or(0),
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

    // After the rows and not before them: the vertical scroll bar appears with the row that
    // overflows the control, and it is the client width the column has to match — task T-12-7.
    fit_cycle_column(hwnd);

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
    /// The setting `[general].theme` this window was opened under — FR-92а, task T-13-17.
    ///
    /// Kept because the palette has to be resolvable **again**. The handler of
    /// [`WM_APP_SYSTEM_THEME`] re-resolves out of this setting and a fresh reading of the
    /// system switch ([`refresh_about_palette`]), and a window that remembered only the
    /// answer could not tell «система стала светлой» from «настройка light»: under `light`
    /// and `dark` FR-92а fixes the palette and the system has no say, which is the very
    /// sentence [`repaint_for_system_theme`] decides by.
    ///
    /// Copied out of the configuration by the caller at the moment of opening and never
    /// moved afterwards: this window changes nothing (SEC-05), so there is no «Применить»
    /// here to move it — and a settings dialog cannot be open at the same time, because the
    /// tray command that would open one is a command of a menu this modal window is holding
    /// the thread away from.
    setting: ThemeSetting,
    /// The palette of this window, resolved at the moment it is opened and re-resolved
    /// whenever the system theme moves under it — FR-92а, task T-13-17.
    ///
    /// Until that task this field was resolved once and never again: the window was called
    /// short-lived and nobody posted it [`WM_APP_SYSTEM_THEME`]. The audit of 2026-08-24
    /// read that as a departure from the letter of FR-92а — the requirement names this
    /// window among «всё видимое глазом» and asks for the change to apply to open windows
    /// without a restart, without an exception for short-lived ones, and a person can hold
    /// this window open for as long as they like.
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
    /// The icon the caption is shown by `WM_SETICON` — task T-12-1: the settings dialog's
    /// field, its type and its ownership, in the window that shares its paths (§6.2).
    icon: Option<CaptionIcons>,
    /// The logo the window shows beside its name — finding **A-08**, task T-12-4.
    ///
    /// `None` until `WM_INITDIALOG`, exactly as `fonts` is and for the same kind of reason:
    /// the size is asked of the window, and there is no window when this value is built. See
    /// [`AboutLogo`].
    logo: Option<AboutLogo>,
}

/// The size of the about window's logo in **mock-up pixels** — finding A-08, task T-12-4.
///
/// The generator's literal: `Draw-AppIcon $g ($aX + 28) ($aY + 76) 56` —
/// `scratchpad-Э11\chrome.ps1:193`. Through [`scaled`] that is **40 px** at 96 DPI, which is
/// the number the protocol measured the mock-up at and the number the window used to miss by
/// eight: an `SS_ICON` static loads the 32 px frame of the `.ico` and ignores the units its
/// statement declares, so the template could not ask for 40 however it was written.
pub const ABOUT_LOGO_SIZE: i32 = 56;

/// The logo of the about window, loaded at the size the mock-up draws it and owned until the
/// window is gone — finding **A-08**, task T-12-4.
///
/// # Why a handle of our own and not a bigger rectangle in the template
///
/// Measured, and the measurement is what settled it: an `SS_ICON` static sizes *itself* to the
/// icon it holds and pays no attention to the width and height of its statement — the comment
/// in `app.rc` said so before this task and the protocol confirmed it (a 21 × 20 unit
/// statement showing a 32 × 32 picture). The documented lever is the other way round: hand the
/// control an icon of the size wanted (`STM_SETICON`) and it resizes itself around it. Owner
/// drawing was the alternative and is refused by a test of `tests\settings.rs` — «the about
/// icon must stay `SS_ICON`» — so this is the one road left, and it is a documented one.
///
/// # Who owns the handle
///
/// **This value, and nothing else** — the ownership [`CaptionIcons`] spells out at length, for
/// the same documented reason: `LR_SHARED` may only be used for an image asked for at the size
/// the resource already holds, and 40 px is not a frame of `res\langswitcher-active.ico` (it
/// carries 16, 20, 24, 32, 48, 64 and 256), so the loader scales one and the result belongs to
/// the caller. Without `LR_SHARED` the handle **must** be destroyed, and `Drop` is where.
///
/// `STM_SETICON` does not transfer ownership either: the control is handed a handle to
/// *display*. Its answer — the icon the control held before — is the 32 px frame the dialog
/// manager loaded out of the resource by itself; that one is shared, was never ours, and is
/// deliberately not touched.
///
/// # When it is freed
///
/// After the window is destroyed and not before, exactly as [`CaptionIcons`]: the value lives
/// in the state on the frame of [`show_about_dialog`], and `DialogBoxParamW` is modal — it
/// returns only once the window is gone, and the state is dropped after it returns.
struct AboutLogo {
    /// The frame, scaled to [`ABOUT_LOGO_SIZE`] mock-up pixels of the window's own DPI.
    icon: HICON,
}

impl AboutLogo {
    /// Loads the frame at the size `hwnd`'s DPI makes of [`ABOUT_LOGO_SIZE`].
    ///
    /// ⚠ **The size travels through the DPI of the window**, like every length of the mock-ups
    /// in this file: [`scaled`] over the DPI of the window's own DC. A 40 written down by hand
    /// would be a 40 px logo on a 200 % screen too, beside a name row and a template that had
    /// both doubled.
    ///
    /// `None` for a refused `LoadImageW` (NFR-13): the control then keeps the 32 px frame it
    /// loaded for itself, which is the picture this window had before this task — degraded and
    /// alive. The refusal is deliberately **not** journaled, for the reason
    /// [`CaptionIcons::load`] gives for its own: the `OPERATIONS` vocabulary of module `diag`
    /// is closed and has no row for loading a picture.
    fn load(hwnd: HWND) -> Option<Self> {
        let module = resource_module();
        let instance = HINSTANCE(module.0);

        // SAFETY: `hwnd` is the live dialog, and the call answers its DC or an invalid handle;
        // the DC is released below on both paths — the shape [`combo_row_height`] already uses.
        let dc = unsafe { GetDC(Some(hwnd)) };

        // NFR-13 is one level down: `dc_dpi` answers 96 for a DC that will not say, which is
        // the 100 % size and a legal one.
        let size = scaled(ABOUT_LOGO_SIZE, dc_dpi(dc));

        if !dc.is_invalid() {
            // SAFETY: releases exactly the DC taken above, once.
            unsafe { ReleaseDC(Some(hwnd), dc) };
        }

        // SAFETY: `instance` is the module handle whose resources carry `IDI_APP_ACTIVE` — the
        // very module the interface strings are read from — and the "name" is an integer
        // identifier in the `MAKEINTRESOURCE` form, so nothing is dereferenced as a string.
        // Without `LR_SHARED` the call makes a handle this value owns; the crate turns a null
        // result into an error, so NFR-13 is satisfied by the `ok()?`.
        let icon = HICON(
            unsafe {
                LoadImageW(
                    Some(instance),
                    resource_id(IDI_APP_ACTIVE),
                    IMAGE_ICON,
                    size,
                    size,
                    LR_DEFAULTCOLOR,
                )
            }
            .ok()?
            .0,
        );

        Some(Self { icon })
    }

    /// The frame as a plain value — what the handler takes out of the state before it lets the
    /// borrow go, the split every handler of this file makes.
    fn handle(&self) -> HICON {
        self.icon
    }
}

impl Drop for AboutLogo {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful `LoadImageW` without `LR_SHARED`, so it is
        // ours to destroy, and it is destroyed exactly once — the type is neither `Copy` nor
        // `Clone`, its field is private and never reassigned, and `drop` runs once. The window
        // that displayed it is already destroyed: this value lives in the state on the frame of
        // the modal call and is dropped after it returns.
        if let Err(error) = unsafe { DestroyIcon(self.icon) } {
            // NFR-13. Not fatal, and named in the journal exactly as `CaptionIcons` names its
            // own: a handle that would not be destroyed is a leak of one icon for the life of
            // the process, and the program has no better answer than to say so.
            crate::app::report_non_critical("DestroyIcon", &error);
        }
    }
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
    // FR-92а, task T-13-17. Taken here and released however this function leaves, the `-1`
    // return below and a panic on the way through included — see [`AboutSession`]. The
    // window itself is recorded from `WM_INITDIALOG`, which is the first moment there is
    // one to record.
    let _open = AboutSession::open();

    let palette = theme::resolve(setting, theme::system_is_light());

    let state = RefCell::new(AboutState {
        // FR-92а, task T-13-17: kept for the re-resolution, not for the first one — see the
        // field.
        setting,
        palette,
        brushes: theme::Brushes::new(palette),
        version,
        // `None` until `WM_INITDIALOG` — see the field.
        fonts: None,
        // FR-92а, task T-12-1: loaded before the window exists — see [`CaptionIcons`].
        icon: CaptionIcons::load(),
        // `None` until `WM_INITDIALOG` — see the field and [`AboutLogo::load`]: the size is
        // asked of the window, so this one cannot be loaded before there is one.
        logo: None,
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

            // FR-92а, task T-13-17: the record `on_system_theme_message` finds this window
            // by. Cleared by the guard of `show_about_dialog` however that function leaves,
            // so it never outlives the window it names — the dialog's own record next door
            // is written and cleared on exactly these terms.
            AboutSession::record(hwnd);

            // Task T-12-8: the response of the cursor, wired on this window's one push button
            // exactly as on the nine of the settings dialog. The other half of the pair is the
            // `WM_DESTROY` branch below, walking the same `ABOUT_BUTTONS` list — see that
            // constant for why «ОК» is subclassed although this wave paints it no differently.
            subclass_buttons(hwnd, &ABOUT_BUTTONS);

            // SAFETY: the pointer has just been stored and names the `RefCell` on the
            // frame of `show_about_dialog`, which outlives this modal call.
            unsafe {
                with_about_state(hwnd, |state| {
                    fill_about(hwnd, state.version);

                    // FR-92а, task T-11-17: the face the caption of «ОК» is set in, made out
                    // of the font the manager gave the window — the same call, and the same
                    // ownership, as in the settings dialog. Task T-12-4 added a third face to
                    // the set, and the name row of this very window is what it is for.
                    state.fonts = DialogFonts::new(hwnd, OK_COMMAND);

                    // FR-92а, task T-12-4: the 40 px logo, at the size this window's DPI makes
                    // of the mock-up's — see [`AboutLogo`] for the ownership.
                    state.logo = AboutLogo::load(hwnd);

                    // The non-client title bar follows the resolved palette from the
                    // first showing — the same call the settings dialog makes.
                    apply_title_bar_theme(hwnd, state.palette);
                })
            };

            // FR-92а, task T-12-1: the icon of the caption — the settings dialog's path, and
            // its discipline: the handles leave the borrow before the messages are posted.
            //
            // SAFETY: as above — the pointer was stored just now and the value it names is
            // alive for the whole of this modal call.
            let frames = unsafe {
                with_about_state(hwnd, |state| state.icon.as_ref().map(CaptionIcons::frames))
            };

            if let Some(Some(frames)) = frames {
                // SAFETY: `hwnd` is the live window, and the handles belong to the value the
                // state keeps, which is dropped only after this modal call returns.
                unsafe { CaptionIcons::show_on(hwnd, frames) };
            }

            // FR-92а, task T-12-4: the logo of the window's own body, on the same discipline —
            // the handle leaves the borrow before any message is sent.
            //
            // SAFETY: as above — the pointer was stored just now and the value it names is
            // alive for the whole of this modal call.
            let logo = unsafe {
                with_about_state(hwnd, |state| state.logo.as_ref().map(AboutLogo::handle))
            };

            if let Some(Some(logo)) = logo {
                // `STM_SETICON`, the documented way to give an `SS_ICON` static a picture: the
                // control resizes itself around the handle, which is how a 40 px logo reaches a
                // window whose template can only ask for one and be ignored (see [`AboutLogo`]).
                //
                // ⚠ **Sent and not posted, and that is not the `SendMessage` FR-72 forbids**:
                // this is `SendDlgItemMessageW` through `send_to`, the one road this file
                // already sends its own controls anything by (`WM_GETFONT` in
                // `dialog_logfont`). The answer — the shared 32 px frame the dialog manager
                // loaded for itself — is deliberately dropped: it was never ours to free.
                let _ = send_to(hwnd, IDC_ABOUT_ICON, STM_SETICON, logo.0 as usize, 0);
            }

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

        // FR-92а, task T-12-1, п. 4: the ground of this window and the one-pixel line under
        // its title bar. Until this task the window had no erase handler at all — the dialog
        // manager erased it with the brush `WM_CTLCOLORDLG` answers, which is the very same
        // `window_bg` — so the ground is unchanged and the line is the whole difference.
        WM_ERASEBKGND => {
            // SAFETY: the pointer was stored on `WM_INITDIALOG` and the value it names is
            // alive for the whole of this modal call.
            unsafe { on_about_erase_background(hwnd, wparam) }
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

        // Task T-12-8: the far half of the subclass pair, on the terms `dialog_proc` states at
        // its own `WM_DESTROY` — this message reaches the window while its children are still
        // live, so «ОК» is still there to be handed back its own procedure. The message carries
        // nothing and is not dereferenced (SEC-05).
        WM_DESTROY => {
            unsubclass_buttons(hwnd, &ABOUT_BUTTONS);

            // «Not handled»: the dialog manager still needs its own `WM_DESTROY`.
            0
        }

        // FR-92а, task T-13-17: the system theme moved while this window is up — the far end
        // of the nudge `on_system_theme_message` posted, the same message and the same road
        // the settings dialog is told by. The message carries nothing and decides nothing
        // (SEC-05): `refresh_about_palette` resolves the palette afresh out of this
        // program's own setting and its own reading of the system switch, and leaves
        // everything alone when the resolution did not move — the check that settles the
        // batch Windows sends of these into one repaint. A forged message therefore buys the
        // sender one reading of the personalization switch and one pointer comparison, at
        // worst one repaint of our own window in the palette the system already asks for.
        WM_APP_SYSTEM_THEME => {
            // SAFETY: the pointer was stored on `WM_INITDIALOG` and the value it names is
            // alive for the whole of this modal call.
            unsafe { with_about_state(hwnd, |state| refresh_about_palette(hwnd, state)) };

            0
        }

        _ => 0,
    }
}

/// Re-resolves the palette of the «О программе» window and repaints it if the resolution
/// moved — FR-92а, task T-13-17.
///
/// [`refresh_palette`] over the state this window keeps, and deliberately the same steps in
/// the same order: a fresh [`theme::resolve`] out of the window's own setting and one
/// reading of the system switch; the identity test that is both the correctness check and
/// the debounce of the batch — `resolve` answers `&'static` identity, so pointer equality is
/// the whole of it; and, only when the resolution moved *and* a whole new brush set was
/// made, the new palette, the caption through [`apply_title_bar_theme`], and one
/// invalidation of the window together with its children.
///
/// Two lines of the dialog's version are absent because this window has neither of the
/// things they are for: the layout list's three `LVM_SET*COLOR` colours, and the cached
/// background picture. Everything this window does paint with is read from `state.palette`
/// on the next paint — the shared `WM_CTLCOLOR*` answer, the shared owner-drawn button and
/// the two `FillRect`s of [`on_about_erase_background`] — so the invalidation is the whole
/// of what the client area needs (§6.2).
///
/// A refused [`theme::Brushes::new`] (NFR-13) leaves palette and brushes exactly as they
/// were: the previous consistent palette on the screen is better than half a new one. It is
/// not journaled, for the reason `Brushes::new` gives for its own refusal.
fn refresh_about_palette(hwnd: HWND, state: &mut AboutState) {
    let fresh = theme::resolve(state.setting, theme::system_is_light());

    if !std::ptr::eq(fresh, state.palette)
        && let Some(brushes) = theme::Brushes::new(fresh)
    {
        state.palette = fresh;
        state.brushes = Some(brushes);

        apply_title_bar_theme(hwnd, fresh);

        repaint_after_palette_change(hwnd);
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

/// The version line of the about window: [`IDS_ABOUT_VERSION`] with the version number
/// substituted — FR-94. The number comes out of the `VERSIONINFO` resource of the running
/// executable, read by the caller of [`show_about_dialog`] the same way the old box read
/// it, and is never spelled in the source.
///
/// # Three parts and not four — решение В-2, finding A-12
///
/// The mock-up's line is «версия 0.1.0» (`chrome.ps1:195`), and the fourth part is what made
/// the live line longer than the model's at the same point size. The revision is *read* all the
/// same — the caller hands over whatever the resource holds — and only the last part is left
/// out of the sentence a person reads. Nothing else in the program loses it.
///
/// `None` — a binary without the resource — shows a dash: not a failure, and next to
/// impossible for this window, because the template it was created from lives in the same
/// `app.rc` as the version block.
///
/// Public so the test can call the very function the dialog calls.
pub fn about_version_line(version: Option<(u16, u16, u16, u16)>) -> String {
    let number = version.map_or_else(
        || "—".to_owned(),
        // The revision is deliberately dropped here and nowhere else — see the doc comment.
        |(major, minor, build, _revision)| format!("{major}.{minor}.{build}"),
    );

    format_text(IDS_ABOUT_VERSION, &[&number])
}

/// The whole background of the about window — FR-92а, task T-12-1, п. 4.
///
/// Two `FillRect`s and no picture: this window has no panels, no field frames and no
/// letter-spaced captions, so there is nothing here worth the [`BackgroundCache`] the
/// settings dialog needs — the ground and the line are the whole of it.
///
/// Answers 1 — «erased» — when the ground was laid, and 0 when the state is unreachable (a
/// re-entrant message) or [`theme::Brushes::new`] was refused at initialisation: the dialog
/// manager then erases with the answer of `WM_CTLCOLORDLG`, exactly as it did before this
/// task, and the window is short of its line and of nothing else (NFR-13).
///
/// # Safety
///
/// Called from [`about_proc`] only, with the `wparam` of the message — the DC the manager
/// owns for the length of the send.
unsafe fn on_about_erase_background(hwnd: HWND, wparam: WPARAM) -> isize {
    // The one thing taken out of the message (SEC-05): the DC to paint into. It is written
    // to and never read from, and no pointer of the message is followed.
    let dc = HDC(wparam.0 as *mut std::ffi::c_void);

    let mut client = RECT::default();

    // NFR-13: examined — no rectangle, no paint, and the manager's own erase is the
    // degraded-but-alive answer.
    //
    // SAFETY: `hwnd` is the live window and `client` is a live local the call fills.
    if unsafe { GetClientRect(hwnd, &mut client) }.is_err() {
        return 0;
    }

    // The colour choice, split from the painting exactly as everywhere in this file: the
    // borrow of the state ends before the DC is touched.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_about_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13).
            let brushes = state.brushes.as_ref()?;

            Some((brushes.window_bg(), state.palette.field_border))
        })
    };

    let Some(Some((ground, line))) = choice else {
        return 0;
    };

    // SAFETY: `dc` is the DC of the message, painted into for the length of this send;
    // `ground` is a live brush this window's state owns for longer than the call.
    unsafe { FillRect(dc, &client, ground) };

    paint_caption_underline(dc, &client, line);

    // TRUE — the background is drawn; the manager must not erase over it.
    1
}

/// The colour role of one static of the about window — the closed vocabulary
/// [`static_color_role`] already answers with, reused rather than widened (§6.2): the version
/// line **and the two description lines** are the quiet ones, the icon and the name are
/// ordinary captions. Nothing in this window is a field, so [`StaticColorRole::Field`] never
/// comes out of it.
///
/// # The description went quiet in task T-12-4 — finding A-06
///
/// The mock-up paints the paragraph under the name with `$brMu` — the muted brush — and the
/// name itself with `$brFg` (`chrome.ps1:194,196`), so only two things in that window are
/// drawn in the full-strength ink: the name and the icon. Решение В-2 keeps the live two
/// sentences where the model has one, and says in as many words that they stay «в колонке
/// имени и приглушённым цветом». Until this task both lines answered [`StaticColorRole::Label`]
/// and came out `text`.
///
/// Public for the same reason as [`static_color_role`]: the table test calls the function
/// the dialog calls.
pub fn about_static_color_role(control: i32) -> StaticColorRole {
    match control {
        IDC_ABOUT_VERSION | IDC_ABOUT_LINE_1 | IDC_ABOUT_LINE_2 => StaticColorRole::Muted,
        // Spelled out rather than swallowed by the catch-all, so the mirrored identifiers
        // of the template stay load-bearing in exactly one function.
        IDC_ABOUT_ICON | IDC_ABOUT_NAME => StaticColorRole::Label,
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
                // Finding A-05, task T-12-4: **one** row of this window is set in the second
                // face — the name. Everything else here (the version line and the two
                // description lines) keeps the dialog font, exactly as the mock-up draws them:
                // the generator gives `$FBold` to `'Lang Switcher'` and `$F` to the rest of the
                // window (`chrome.ps1:194-196`).
                state.fonts.as_ref().map(|fonts| {
                    if control == IDC_ABOUT_NAME {
                        fonts.name()
                    } else {
                        fonts.text()
                    }
                }),
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
    let (ctl_type, ctl_id, item_state, dc, rect, item_window) = (
        item.CtlType,
        item.CtlID,
        item.itemState,
        item.hDC,
        item.rcItem,
        item.hwndItem,
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

    // Task T-12-8: the same fourth state as in the settings dialog, read the same way — this
    // program's own record, compared against the window of the message and never followed. The
    // colour table answers the accent for «ОК» at rest and `sel_fg` under the cursor since task
    // T-15-2, so this flag is **load bearing**: the button paints itself differently hot and
    // cold, and dropping the read would take the response away. ⚠ Until that task the comment
    // here said the button painted itself the same either way — true then, false now.
    let hot = is_hot(item_window);

    // The colour choice, split from the painting — the borrow ends before the DC is
    // touched, exactly as everywhere in this file.
    //
    // SAFETY: see the caller.
    let choice = unsafe {
        with_about_state(hwnd, |state| {
            // `None` — the brushes were refused at initialisation (NFR-13).
            let brushes = state.brushes.as_ref()?;

            let (colors, hot_brush) = resolve_button_colors(
                // Task T-12-4, finding A-13: the same face and the same ink as everywhere —
                // and no frame, which is the one thing this window's button does differently
                // from the nine of the settings dialog.
                about_button_colors(control, hot, pressed, disabled),
                // The ground of this window has no panels in it: `on_about_ctl_color`
                // answers `WM_CTLCOLORBTN` with the window brush and nothing else
                // (task T-12-6).
                brushes.window_bg(),
                brushes,
                state.palette,
            );

            Some((
                colors,
                hot_brush,
                state.fonts.as_ref().map(DialogFonts::text),
            ))
        })
    };

    // `_hot_brush` is named and not discarded for the reason `on_draw_item` gives at its own.
    let Some(Some((colors, _hot_brush, face))) = choice else {
        return 0;
    };

    // SAFETY: see the caller — `dc` and `rect` are the values of the message, used only
    // to paint into for the length of this send; `face` is a face the state owns for longer.
    unsafe { paint_push_button(hwnd, control, dc, rect, colors, focused, face) }
}
