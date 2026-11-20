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

use std::cell::Cell;
use std::collections::BTreeMap;
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
    BeginPaint, CreateCompatibleDC, CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS, DT_SINGLELINE,
    DT_VCENTER, DeleteDC, DeleteObject, DrawFocusRect, DrawTextW, EndPaint, FONT_WEIGHT, FW_BOLD,
    FillRect, GetDC, GetMonitorInfoW, GetObjectW, GetTextExtentPoint32W, GetTextFaceW,
    GetTextMetricsW, HBRUSH, HDC, HFONT, InvalidateRect, LOGFONTW, MONITOR_DEFAULTTONEAREST,
    MONITORINFO, MonitorFromWindow, PAINTSTRUCT, ReleaseDC, ScreenToClient, SelectObject,
    SetBkColor, SetBkMode, SetTextColor, TEXTMETRICW, TRANSPARENT, TextOutW,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceExW, FindResourceW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::UI::Controls::{LVIS_STATEIMAGEMASK, WM_MOUSELEAVE};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyState, IsWindowEnabled, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    VIRTUAL_KEY, VK_APPS, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_END, VK_ESCAPE, VK_F1, VK_HOME,
    VK_INSERT, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_NUMLOCK, VK_PAUSE,
    VK_PRIOR, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT, VK_SNAPSHOT,
};
use windows::Win32::UI::Shell::{
    DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CB_GETCURSEL, CB_GETLBTEXT, CB_GETLBTEXTLEN, CreateDialogIndirectParamW, CreateDialogParamW,
    DLGPROC, DLGTEMPLATE, DestroyIcon, GW_CHILD, GW_HWNDNEXT, GetClientRect, GetDlgCtrlID,
    GetDlgItem, GetDlgItemTextW, GetParent, GetWindow, GetWindowRect, HICON, ICON_BIG, ICON_SMALL,
    IDOK, IMAGE_ICON, LR_DEFAULTCOLOR, LR_DEFAULTSIZE, LoadImageW, PostMessageW, RT_DIALOG,
    SB_BOTTOM, SB_LINEDOWN, SB_LINEUP, SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK,
    SB_TOP, SW_SHOWNORMAL, SendDlgItemMessageW, SetDlgItemTextW, UISF_HIDEFOCUS, WM_APP,
    WM_ERASEBKGND, WM_GETFONT, WM_MOUSEMOVE, WM_NCDESTROY, WM_PAINT, WM_QUERYUISTATE, WM_SETICON,
    WNDPROC, WS_EX_LAYOUTRTL,
};
use windows::core::{Error as WinError, PCWSTR, PWSTR, w};

use crate::CONFIG_DIR_NAME;
use crate::layouts::{LayoutId, LayoutSpec};
// The state of `[letters]` lives in this file because the schema of section 7 does; the
// calendar it is written in, the vocabulary of `thanks` and every rule that reads the section
// live in `letters`, which owns them (§6.2). The import is by module name for the same reason
// `theme` is: a field of the schema reads as `letters::Date`, and where it came from is part of
// what it says.
use crate::letters;
// ⚠ The names after `ThemeSetting` are the drawing library task T-14-3 moved out of this file
// into its owner (§6.2, finding 24 of the audit of 2026-08-24). They are imported by name — not
// called as `theme::…` — so that the change of address stayed a change of address: every call
// site of this file reads exactly as it read before the move.
use crate::theme::{
    self, ButtonBorderRole, ButtonColors, ButtonFaceRole, ButtonTextRole, CORNER_RADIUS, CheckMark,
    ComboBorderRole, ComboChevronRole, ComboFillRole, HotBrush, ResolvedButtonColors, ThemeSetting,
    caption_advance, check_frame_colors, combo_chevron_points, combo_closed_color_roles,
    combo_fill_brush, combo_text_ink, create_font, dc_dpi, draw_check_mark, draw_combo_chevron,
    drawing_dpi, paint_rounded, paint_selection_stripe, restore_face, scaled, select_face,
    smoothed_logfont, title_bar_is_dark,
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
///
/// Version 3 arrived with FR-100 and task Т-21-5: the section `[feedback]` and its one field.
/// [`step_2_to_3`] carries the files of schema 2 over, and carries nothing but the stamp — see
/// the rung for why that is the whole of it.
///
/// Version 4 arrived with вопрос **94.1** and task Т-29-1: the ten locales решение 93 added to
/// [`Language`] became a schema of their own. Nothing in the *file* changed meaning — that is
/// why [`step_3_to_4`] is a bare stamp — and the whole of what the number buys is in the other
/// direction: a build that knows two locales now recognises a file of the twelve by its stamp,
/// **before** its parser refuses one of the ten values, and so leaves that file alone instead of
/// moving it to `.bad`. See [`Language`], where the cost this pays off is written out.
///
/// Version 5 arrived with **решение 97.3** and task Т-30-5: Hebrew and Arabic. The same coin
/// spent one tier up, and for the same thing — [`step_4_to_5`] is a bare stamp, and what the
/// number buys is that a twelve-locale build meeting `language = "he"` leaves the file whole
/// instead of quarantining it.
///
/// Version 6 arrived with **вопрос 101** and task Т-32-1: the section `[letters]` of FR-101 and
/// FR-102. The first rung since [`step_1_to_2`] that is **not** a bare stamp — it has a
/// decision to carry, and [`step_5_to_6`] spells it out: a machine that is being updated has
/// already been used, so it must not meet «Привет», and it must meet «Что нового» exactly once.
///
/// Version 7 arrived with **вопрос 157** and task T-93-1: `[hotkey] modifiers`, the combinations
/// `Ctrl`/`Alt`/`Shift` + a key. [`step_6_to_7`] is a bare stamp — an absent list is the bare key a
/// schema 6 file meant — and the number buys the other direction once more: a build that has
/// never heard of the list meets a stamp it does not know, reads the file and leaves it alone,
/// instead of dropping the list on its next write.
///
/// Version 8 arrived with **вопрос 159** and task T-95-1: `[hotkey] trigger`, the double press of
/// `Shift`. [`step_7_to_8`] is a bare stamp again — an absent trigger is `press`, which is what a
/// schema 7 hotkey was — and the number is spent on `0.93.0` and the builds before it: they meet a
/// stamp they do not know and leave the file alone instead of dropping the trigger (measured on the
/// base — посылка П4 of stage Э95: `FromNewerSchema`, the hotkey `Pause`, the bytes untouched).
///
/// Version 9 arrived with **stage E108**, task T-108-2: the thirty languages of group A in the
/// closed set of `general.language`. [`step_8_to_9`] is a bare stamp, and the number is spent on
/// 1.2.0 and the builds before it — they meet a stamp they do not know and leave a file that says
/// `language = "hi"` alone instead of quarantining it.
///
/// Version 10 arrived with **stage E109**, task T-109-1: the twenty-two languages of group B in
/// the closed set. [`step_9_to_10`] is a bare stamp, and the number is spent on 1.3.0 and the
/// builds before it, which would otherwise quarantine a file that says `language = "nb"`.
pub const CURRENT_SCHEMA_VERSION: u32 = 10;

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

/// Interface language, `general.language` of section 7. Sixty-six values — the twelve of решение
/// 93, tier one, the two right-to-left locales of вопрос 97, since stage E108 the thirty of group A
/// (решения 198.13, 198.14, 200б.1; Serbian and Uzbek in both of their scripts), and since stage
/// E109 the twenty-two of group B (Latin with letters of its own; Norwegian in both of its forms).
///
/// A closed set rather than a free string: a value outside it must not pass silently.
///
/// ⭐ **The closed set and the languages a build offers are two things since stage E108** (task
/// T-108-2). Every value is here, with its tag, LANGID, name and direction, and a file that names
/// one of them parses; the languages the build **offers** — the combo, the first run, the strings —
/// are [`Self::shipped`]: the entries of `res\lang\languages.toml` that say `state = "shipped"`,
/// read by `build.rs`. A language joins the program with its file `res\lang\<tag>.rc` and that one
/// line of the registry; until then [`Self::in_force`] puts English in its place.
///
/// # ⚠ What a closed set cost on a downgrade, and how вопрос 94.1 stopped paying it
///
/// Решение 93 added ten **values** to this field and left [`CURRENT_SCHEMA_VERSION`] where it
/// was: the values live inside a schema that already existed, and a new build reads every old
/// file without a migration. The price was paid in the other direction, and task Т-28-2
/// measured it. An installed build that knows two values, handed a file that said
/// `language = "de"` under the same stamp `3`, refused the strict parse — that is what «must
/// not pass silently» means — and [`Config::from_toml_str`] then asked the document what schema
/// it claimed. The answer was `3`, which is **not** newer than that build's own, so the file was
/// [`ConfigError::Malformed`] and the policy [`SavePolicy::QuarantineFirst`]: the configuration
/// moved to `.bad` and replaced by defaults.
///
/// **Вопрос 94.1, task Т-29-1, is the decision to stop paying that.** The schema is now `4`, and
/// nothing about the file changed with it — see [`step_3_to_4`], a bare stamp. What changed is
/// the answer an old build gives: the stamp in a file of the twelve is one no two-locale build
/// ever wrote, so [`Config::from_toml_str`] reaches [`ReadOutcome::FromNewerSchema`] →
/// [`SavePolicy::Forbidden`] and the file is **read and never touched** — no rewrite, no `.bad`.
///
/// Both halves are measured and pinned by
/// `a_downgrade_meets_the_new_stamp_and_refuses_to_write_rather_than_quarantining`, whose
/// control keeps the other half true: a file of the **current** stamp that will not parse is
/// still damaged, and still quarantined.
///
/// **Вопрос 97.3 pays the same coin again, one tier up.** `He` and `Ar` are two more values of
/// this closed set, so a *twelve*-locale build meeting `language = "he"` would have refused the
/// document and quarantined it exactly as a two-locale build once did with `de`. The schema is
/// therefore `5`, [`step_4_to_5`] is a bare stamp, and
/// `a_twelve_locale_build_meeting_a_right_to_left_file_refuses_to_write` pins the same three
/// claims on the new numbers. The lesson this enum keeps writing down is one line long: **a new
/// value of a closed set costs a schema**. Stage E108 pays it a third time: the thirty values of
/// group A are schema `9`, [`step_8_to_9`] is a bare stamp.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Russian, the default of section 7.
    #[default]
    Ru,
    /// English.
    En,
    /// Ukrainian.
    Uk,
    /// German.
    De,
    /// French.
    Fr,
    /// Spanish.
    Es,
    /// Portuguese — **Brazilian**, which is what решение 93 names and what [`Self::langid`]
    /// asks for. The tag stays the bare `pt` of the other eleven: section 7 spells locales in
    /// two letters, and there is no second Portuguese here to tell it apart from.
    Pt,
    /// Italian.
    It,
    /// Polish.
    Pl,
    /// Czech.
    Cs,
    /// Turkish.
    Tr,
    /// Greek.
    El,
    /// Hebrew — **read right to left**, вопрос 97. The first locale of this program whose
    /// windows are mirrored; [`Self::is_rtl`] is the one place that knows it.
    He,
    /// Arabic — read right to left, вопрос 97. One table for the standard language, tagged
    /// `ar-SA`, and not one table per country: section 7 spells a locale in two letters, and
    /// this build ships one translation per language exactly as it does for `pt`.
    Ar,
    /// Hindi — group A, stage E108 (решение 202.1: the trial language with Urdu).
    Hi,
    /// Bengali — one language, `bn-IN` (решение 200б.2).
    Bn,
    /// Urdu — **read right to left**, stage E108 (решение 202.1: the trial language with Hindi).
    Ur,
    /// Punjabi — Gurmukhi, `pa-IN` (решение 200б.2).
    Pa,
    /// Marathi.
    Mr,
    /// Telugu.
    Te,
    /// Tamil.
    Ta,
    /// Persian — **read right to left**. Its calendar is the solar hijri; the program prints the
    /// Gregorian month (`CAL_GREGORIAN`) whatever the locale's own calendar is.
    Fa,
    /// Gujarati.
    Gu,
    /// Thai. Its calendar is the Buddhist one; the program prints the Gregorian month.
    Th,
    /// Kannada.
    Kn,
    /// Odia.
    Or,
    /// Malayalam.
    Ml,
    /// Uzbek in the Latin script — the official one and the main Uzbek of this program
    /// (решение 200б.1); `uz-Latn-UZ`.
    Uz,
    /// Uzbek in the Cyrillic script, alive in everyday use — the second Uzbek (решение 200б.1);
    /// `uz-Cyrl-UZ`. The tag of section 7 is `uz-Cyrl`.
    #[serde(rename = "uz-Cyrl")]
    UzCyrl,
    /// Nepali.
    Ne,
    /// Assamese.
    As,
    /// Azerbaijani — Latin, the official script (решение 200б.1).
    Az,
    /// Khmer.
    Km,
    /// Kazakh.
    Kk,
    /// Serbian in the Cyrillic script — the constitutional one and the main Serbian of this
    /// program (решение 200б.1); `sr-Cyrl-RS`.
    Sr,
    /// Serbian in the Latin script — the second Serbian (решение 200б.1); `sr-Latn-RS`. The tag of
    /// section 7 is `sr-Latn`.
    #[serde(rename = "sr-Latn")]
    SrLatn,
    /// Uyghur — **read right to left**, in the Arabic script.
    Ug,
    /// Bulgarian.
    Bg,
    /// Armenian.
    Hy,
    /// Lao.
    Lo,
    /// Belarusian.
    Be,
    /// Tatar.
    Tt,
    /// Georgian.
    Ka,
    /// Macedonian.
    Mk,
    /// Vietnamese — group B, stage E109 (решения 198.13, 198.14): Latin with letters of its own.
    /// Its tones are precomposed letters (NFC), never a letter and a combining mark.
    Vi,
    /// Dutch.
    Nl,
    /// Romanian — `ș` and `ț` with the comma below (U+0219, U+021B), not the cedilla.
    Ro,
    /// Hungarian.
    Hu,
    /// Swedish.
    Sv,
    /// Albanian.
    Sq,
    /// Danish.
    Da,
    /// Finnish.
    Fi,
    /// Croatian.
    Hr,
    /// Slovak.
    Sk,
    /// Norwegian Bokmål — the main Norwegian of this program (решение 200б.1), `nb-NO`; a Windows
    /// that still names the old macrolanguage `no` is answered with it ([`language_of_tag_among`]).
    Nb,
    /// Lithuanian.
    Lt,
    /// Slovenian.
    Sl,
    /// Bosnian — in the Latin script, `bs-Latn-BA`; the only Bosnian of this program.
    Bs,
    /// Latvian.
    Lv,
    /// Estonian.
    Et,
    /// Welsh.
    Cy,
    /// Maltese.
    Mt,
    /// Icelandic.
    Is,
    /// Luxembourgish.
    Lb,
    /// Maori.
    Mi,
    /// Norwegian Nynorsk — the second Norwegian (решение 200б.1), `nn-NO`.
    Nn,
}

/// The tags of the languages this build offers — `build.rs` out of `res\lang\languages.toml`
/// (stage E108, task T-108-2); [`Language::shipped`] is the one reader.
mod shipped_registry {
    include!(concat!(env!("OUT_DIR"), "/shipped_languages.rs"));
}

impl Language {
    /// Every value of the closed set, in the order of the registry `res\lang\languages.toml` —
    /// Russian, English, the ten of решение 93, the two right-to-left locales of вопрос 97, the
    /// thirty of group A (stage E108) and the twenty-two of group B (stage E109).
    ///
    /// [`Self::index`] is a position in this array and the atomic of [`set_ui_language`] stores
    /// that position — in the process only: the file holds the tag (замер L10N). The combo of the
    /// dialog no longer walks this array: since решение 202.7 its order is [`language_order`].
    pub const ALL: [Self; 66] = [
        Self::Ru,
        Self::En,
        Self::Uk,
        Self::De,
        Self::Fr,
        Self::Es,
        Self::Pt,
        Self::It,
        Self::Pl,
        Self::Cs,
        Self::Tr,
        Self::El,
        Self::He,
        Self::Ar,
        Self::Hi,
        Self::Bn,
        Self::Ur,
        Self::Pa,
        Self::Mr,
        Self::Te,
        Self::Ta,
        Self::Fa,
        Self::Gu,
        Self::Th,
        Self::Kn,
        Self::Or,
        Self::Ml,
        Self::Uz,
        Self::UzCyrl,
        Self::Ne,
        Self::As,
        Self::Az,
        Self::Km,
        Self::Kk,
        Self::Sr,
        Self::SrLatn,
        Self::Ug,
        Self::Bg,
        Self::Hy,
        Self::Lo,
        Self::Be,
        Self::Tt,
        Self::Ka,
        Self::Mk,
        Self::Vi,
        Self::Nl,
        Self::Ro,
        Self::Hu,
        Self::Sv,
        Self::Sq,
        Self::Da,
        Self::Fi,
        Self::Hr,
        Self::Sk,
        Self::Nb,
        Self::Lt,
        Self::Sl,
        Self::Bs,
        Self::Lv,
        Self::Et,
        Self::Cy,
        Self::Mt,
        Self::Is,
        Self::Lb,
        Self::Mi,
        Self::Nn,
    ];

    /// **Every value in the order of its name — the part of the combo under the line**, решение
    /// 202.7, вариант 1 of the mock-up `https://claude.ai/artifact/Phkza4d6dMJ5wGfLNHeBTM`.
    ///
    /// Written down and not sorted at run time, for the reason [`Self::native_name`] is written
    /// down: the list must read the same on every machine, and a comparison asked of Windows
    /// depends on the version of its sorting tables. The order is the one the owner chose on the
    /// mock-up, and it was measured, not reasoned: the root collation of ICU (`InvariantCulture`
    /// of PowerShell 7) over the forty-four names, with Georgian moved to the end as the mock-up
    /// has it — 40 of 40 of the mock-up's rows (stage E108, `scratchpad-E108`). NLS
    /// (`CompareStringEx`, the invariant locale) puts Georgian after Armenian and Uyghur after
    /// Persian — 22 of 40 — and was not taken. Latin, Greek, Cyrillic, Armenian, Hebrew, Arabic, the
    /// Indic scripts, Thai, Lao, Khmer, Georgian.
    ///
    /// Stage E109: the twenty-two of group B by the same instrument over the sixty-six names; the
    /// rule gives the forty-four of E108 back in their order, row for row
    /// (`scratchpad-E109\by-name.out.txt`). All twenty-two are Latin and fall among the Latin rows.
    pub const BY_NAME: [Self; 66] = [
        Self::Az,
        Self::Bs,
        Self::Cs,
        Self::Cy,
        Self::Da,
        Self::De,
        Self::Et,
        Self::En,
        Self::Es,
        Self::Fr,
        Self::Hr,
        Self::Is,
        Self::It,
        Self::Lv,
        Self::Lb,
        Self::Lt,
        Self::Hu,
        Self::Mt,
        Self::Mi,
        Self::Nl,
        Self::Nb,
        Self::Nn,
        Self::Uz,
        Self::Pl,
        Self::Pt,
        Self::Ro,
        Self::Sq,
        Self::Sk,
        Self::Sl,
        Self::SrLatn,
        Self::Fi,
        Self::Sv,
        Self::Vi,
        Self::Tr,
        Self::El,
        Self::Be,
        Self::Bg,
        Self::Kk,
        Self::Mk,
        Self::Ru,
        Self::Sr,
        Self::Tt,
        Self::UzCyrl,
        Self::Uk,
        Self::Hy,
        Self::He,
        Self::Ug,
        Self::Ur,
        Self::Ar,
        Self::Fa,
        Self::Ne,
        Self::Mr,
        Self::Hi,
        Self::As,
        Self::Bn,
        Self::Pa,
        Self::Gu,
        Self::Or,
        Self::Ta,
        Self::Te,
        Self::Kn,
        Self::Ml,
        Self::Th,
        Self::Lo,
        Self::Km,
        Self::Ka,
    ];

    /// Whether this build carries the strings of this language — its tag is an entry of
    /// `res\lang\languages.toml` with `state = "shipped"` (stage E108, task T-108-2).
    pub fn is_shipped(self) -> bool {
        shipped_registry::SHIPPED.contains(&self.tag())
    }

    /// The languages this build offers, in the order of [`Self::ALL`] — the values of the closed
    /// set whose strings are in the build ([`Self::is_shipped`]).
    pub fn shipped() -> impl Iterator<Item = Self> {
        Self::ALL
            .into_iter()
            .filter(|language| language.is_shipped())
    }

    /// The language the interface is shown in when the file names this one — itself when the
    /// build carries it, English when it does not.
    ///
    /// Only a build of a stage of translations meets the second case: a file written by a build
    /// that already had a language a later intermediate build does not. Its strings would be
    /// empty, so the program speaks English and leaves the file as it is.
    pub fn in_force(self) -> Self {
        if self.is_shipped() { self } else { Self::En }
    }

    /// The Windows language identifier of this locale — the number `app.rc` tags its string
    /// table with and the number [`text`] asks `FindResourceExW` for.
    ///
    /// A primary language identifier from `winnt.h` with a sublanguage in the high six bits,
    /// which is how a `LANGID` is built: `LANG_RUSSIAN` (0x19) with `SUBLANG_DEFAULT` (0x01) is
    /// 0x0419. Spelled out rather than computed for the same reason `app.rc` spells them out:
    /// the two files have no shared header, and these fourteen numbers are the joint between
    /// them.
    ///
    /// ⚠ Every number below was read back from `LCIDToLocaleName` before it was written here —
    /// `scratchpad-Э28\прибор-langid.log` for the twelve and `scratchpad-Э30\прибор-langid.log`
    /// for `He` (0x040D, `he-IL`) and `Ar` (0x0401, `ar-SA`), positive control on the
    /// impossible `0x0FFF` in both. Two of
    /// them are worth naming: `Pt` is **0x0416**, `pt-BR`, and not the 0x0816 of Portugal; `Es`
    /// is 0x040A, which Windows answers as `es-ES_tradnl`.
    pub const fn langid(self) -> u16 {
        match self {
            Self::Ru => 0x0419,
            Self::En => 0x0409,
            Self::Uk => 0x0422,
            Self::De => 0x0407,
            Self::Fr => 0x040C,
            Self::Es => 0x040A,
            Self::Pt => 0x0416,
            Self::It => 0x0410,
            Self::Pl => 0x0415,
            Self::Cs => 0x0405,
            Self::Tr => 0x041F,
            Self::El => 0x0408,
            Self::He => 0x040D,
            Self::Ar => 0x0401,
            // Group A, stage E108 — the `langid` of `res\lang\languages.toml`, measured there by
            // `LocaleNameToLCID` and back (69 of 69 round trips, stage L10N).
            Self::Hi => 0x0439,
            Self::Bn => 0x0445,
            Self::Ur => 0x0420,
            Self::Pa => 0x0446,
            Self::Mr => 0x044E,
            Self::Te => 0x044A,
            Self::Ta => 0x0449,
            Self::Fa => 0x0429,
            Self::Gu => 0x0447,
            Self::Th => 0x041E,
            Self::Kn => 0x044B,
            Self::Or => 0x0448,
            Self::Ml => 0x044C,
            Self::Uz => 0x0443,
            Self::UzCyrl => 0x0843,
            Self::Ne => 0x0461,
            Self::As => 0x044D,
            Self::Az => 0x042C,
            Self::Km => 0x0453,
            Self::Kk => 0x043F,
            Self::Sr => 0x281A,
            Self::SrLatn => 0x241A,
            Self::Ug => 0x0480,
            Self::Bg => 0x0402,
            Self::Hy => 0x042B,
            Self::Lo => 0x0454,
            Self::Be => 0x0423,
            Self::Tt => 0x0444,
            Self::Ka => 0x0437,
            Self::Mk => 0x042F,
            // Group B, stage E109 — the `langid` of the registry, measured there as group A's.
            Self::Vi => 0x042A,
            Self::Nl => 0x0413,
            Self::Ro => 0x0418,
            Self::Hu => 0x040E,
            Self::Sv => 0x041D,
            Self::Sq => 0x041C,
            Self::Da => 0x0406,
            Self::Fi => 0x040B,
            Self::Hr => 0x041A,
            Self::Sk => 0x041B,
            Self::Nb => 0x0414,
            Self::Lt => 0x0427,
            Self::Sl => 0x0424,
            Self::Bs => 0x141A,
            Self::Lv => 0x0426,
            Self::Et => 0x0425,
            Self::Cy => 0x0452,
            Self::Mt => 0x043A,
            Self::Is => 0x040F,
            Self::Lb => 0x046E,
            Self::Mi => 0x0481,
            Self::Nn => 0x0814,
        }
    }

    /// Whether this locale is read **right to left** — вопрос 97, task Т-30-2.
    ///
    /// The one place that knows it. Everything the mirroring does — the extended style the
    /// dialog template is created with, the `TPM_LAYOUTRTL` of the tray menu, the reading order
    /// of an island of Latin text — asks this and nothing else, so a fifteenth locale that
    /// happens to be Persian is one arm here rather than a hunt through three modules.
    ///
    /// Deliberately **not** asked of Windows through `GetLocaleInfoEx` and
    /// `LOCALE_IREADINGLAYOUT`: the direction of an interface this program ships is a property
    /// of the translation in `app.rc`, which is written once and does not vary by machine.
    pub const fn is_rtl(self) -> bool {
        // Stage E108: Urdu, Persian and Uyghur — the `rtl` of the registry.
        matches!(self, Self::He | Self::Ar | Self::Ur | Self::Fa | Self::Ug)
    }

    /// The name of section 7, for a message and for a test — two letters, and since stage E108
    /// two letters and a script for the second Serbian and the second Uzbek (`sr-Latn`, `uz-Cyrl`).
    ///
    /// The same words serde produces from the variants above — `rename_all = "lowercase"` and the
    /// two `rename`s — written out rather than derived: this is the vocabulary of the configuration
    /// file, and a reader of section 7 should find it spelled here and not inferred.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
            Self::Uk => "uk",
            Self::De => "de",
            Self::Fr => "fr",
            Self::Es => "es",
            Self::Pt => "pt",
            Self::It => "it",
            Self::Pl => "pl",
            Self::Cs => "cs",
            Self::Tr => "tr",
            Self::El => "el",
            Self::He => "he",
            Self::Ar => "ar",
            Self::Hi => "hi",
            Self::Bn => "bn",
            Self::Ur => "ur",
            Self::Pa => "pa",
            Self::Mr => "mr",
            Self::Te => "te",
            Self::Ta => "ta",
            Self::Fa => "fa",
            Self::Gu => "gu",
            Self::Th => "th",
            Self::Kn => "kn",
            Self::Or => "or",
            Self::Ml => "ml",
            Self::Uz => "uz",
            Self::UzCyrl => "uz-Cyrl",
            Self::Ne => "ne",
            Self::As => "as",
            Self::Az => "az",
            Self::Km => "km",
            Self::Kk => "kk",
            Self::Sr => "sr",
            Self::SrLatn => "sr-Latn",
            Self::Ug => "ug",
            Self::Bg => "bg",
            Self::Hy => "hy",
            Self::Lo => "lo",
            Self::Be => "be",
            Self::Tt => "tt",
            Self::Ka => "ka",
            Self::Mk => "mk",
            Self::Vi => "vi",
            Self::Nl => "nl",
            Self::Ro => "ro",
            Self::Hu => "hu",
            Self::Sv => "sv",
            Self::Sq => "sq",
            Self::Da => "da",
            Self::Fi => "fi",
            Self::Hr => "hr",
            Self::Sk => "sk",
            Self::Nb => "nb",
            Self::Lt => "lt",
            Self::Sl => "sl",
            Self::Bs => "bs",
            Self::Lv => "lv",
            Self::Et => "et",
            Self::Cy => "cy",
            Self::Mt => "mt",
            Self::Is => "is",
            Self::Lb => "lb",
            Self::Mi => "mi",
            Self::Nn => "nn",
        }
    }

    /// What the language combo box of FR-92 shows for this locale — **the name of the language
    /// in that language**, first letter capital.
    ///
    /// Hard-coded and not asked of `GetLocaleInfoEx`, unlike the *layout* names of решение 92:
    /// the name of an interface language is a piece of this program's interface, and it must
    /// read the same on every machine whatever Windows itself is set to. The capital first
    /// letter is the rule решение 92 established for the layout list, applied here by writing
    /// the names that way rather than by lifting a letter at run time — these fourteen strings
    /// never change, so there is nothing to lift. ⚠ Hebrew and Arabic have **no case at all**,
    /// so «עברית» and «العربية» carry no capital and cannot: the rule is «as the language writes
    /// its own name», and for these two that is what it says.
    ///
    /// ⚠ **Not localised, on purpose:** the list reads the same in all fourteen locales. A person
    /// who has the program in a language they cannot read has to find their own language in it,
    /// and «Deutsch» is the only spelling that helps them.
    pub const fn native_name(self) -> &'static str {
        match self {
            Self::Ru => "Русский",
            Self::En => "English",
            Self::Uk => "Українська",
            Self::De => "Deutsch",
            Self::Fr => "Français",
            Self::Es => "Español",
            Self::Pt => "Português (Brasil)",
            Self::It => "Italiano",
            Self::Pl => "Polski",
            Self::Cs => "Čeština",
            Self::Tr => "Türkçe",
            Self::El => "Ελληνικά",
            Self::He => "עברית",
            Self::Ar => "العربية",
            // Group A, stage E108 — the `endonym` of the registry: what Windows calls the language
            // in itself, first letter capital where the script has case; the translator of each
            // language confirms it. Scripts without case — all but the Latin and the Cyrillic
            // ones here — carry none, as Hebrew and Arabic above.
            Self::Hi => "हिन्दी",
            Self::Bn => "বাংলা",
            Self::Ur => "اردو",
            Self::Pa => "ਪੰਜਾਬੀ",
            Self::Mr => "मराठी",
            Self::Te => "తెలుగు",
            Self::Ta => "தமிழ்",
            Self::Fa => "فارسی",
            Self::Gu => "ગુજરાતી",
            Self::Th => "ไทย",
            Self::Kn => "ಕನ್ನಡ",
            Self::Or => "ଓଡ଼ିଆ",
            Self::Ml => "മലയാളം",
            Self::Uz => "O‘zbek",
            Self::UzCyrl => "Ўзбекча",
            Self::Ne => "नेपाली",
            Self::As => "অসমীয়া",
            Self::Az => "Azərbaycan",
            Self::Km => "ខ្មែរ",
            Self::Kk => "Қазақ тілі",
            Self::Sr => "Српски",
            Self::SrLatn => "Srpski",
            Self::Ug => "ئۇيغۇرچە",
            Self::Bg => "Български",
            Self::Hy => "Հայերեն",
            Self::Lo => "ລາວ",
            Self::Be => "Беларуская",
            Self::Tt => "Татар",
            Self::Ka => "ქართული",
            Self::Mk => "Македонски",
            // Group B, stage E109 — the `endonym` of the registry, as group A.
            Self::Vi => "Tiếng Việt",
            Self::Nl => "Nederlands",
            Self::Ro => "Română",
            Self::Hu => "Magyar",
            Self::Sv => "Svenska",
            Self::Sq => "Shqip",
            Self::Da => "Dansk",
            Self::Fi => "Suomi",
            Self::Hr => "Hrvatski",
            Self::Sk => "Slovenčina",
            Self::Nb => "Norsk bokmål",
            Self::Lt => "Lietuvių",
            Self::Sl => "Slovenščina",
            Self::Bs => "Bosanski",
            Self::Lv => "Latviešu",
            Self::Et => "Eesti",
            Self::Cy => "Cymraeg",
            Self::Mt => "Malti",
            Self::Is => "Íslenska",
            Self::Lb => "Lëtzebuergesch",
            Self::Mi => "Māori",
            Self::Nn => "Norsk nynorsk",
        }
    }

    /// This locale as its position in [`Self::ALL`] — the number [`UI_LANGUAGE`] stores and the
    /// item index of the language combo box.
    ///
    /// Public since task Т-28-2: with fourteen positions the order is worth a test of its own,
    /// and a test cannot check a contract it is not allowed to read.
    pub fn index(self) -> u32 {
        // A locale is always in `ALL` — the array is the enumeration itself — but NFR-13 asks
        // what happens if it somehow is not, and the honest answer is the default of section 7,
        // exactly as `from_index` gives for a number out of range.
        Self::ALL
            .iter()
            .position(|&language| language == self)
            .and_then(|at| u32::try_from(at).ok())
            .unwrap_or(0)
    }

    /// The locale [`Self::index`] came from. Anything else is the default of section 7.
    pub fn from_index(index: u32) -> Self {
        usize::try_from(index)
            .ok()
            .and_then(|at| Self::ALL.get(at).copied())
            .unwrap_or(Self::Ru)
    }
}

// =========================================================================================
// Вопрос 95, task Т-29-3 — the locale a first run starts in
// =========================================================================================

/// The most UTF-16 units of the preferred-languages multi-string that are ever looked at.
///
/// A BCP-47 tag is at most 85 characters, and Windows keeps a handful of them; 1024 units is
/// that with room to spare, and a list still running at 1024 is one this program was never
/// going to find itself in. The bound is here for the same reason [`SETTING_NAME_CAP`] is in
/// `tray`: the size comes back from a system call, and a read whose length is somebody else's
/// number is a read with no length at all (SEC-05).
///
/// Public since task T-43-10, so that a test can name the cap instead of copying the number.
///
/// [`SETTING_NAME_CAP`]: crate::tray
pub const PREFERRED_LANGUAGES_CAP: u32 = 1024;

/// Whether the size the sizing call of [`preferred_ui_languages`] answered is one this program
/// reads — and the journal entry of the one refusal that is this program's own; task T-43-10,
/// finding Н71.
///
/// Two answers end the read, and until that task they ended it the same silent way:
///
/// * **zero units** — Windows named nothing. The ordinary empty answer, silent as before: an
///   empty list is the road to `en` that вопрос 95 prescribes, and there is nothing to explain;
/// * **more than [`PREFERRED_LANGUAGES_CAP`]** — Windows answered, and this program refused to
///   read so long a list. The person then sees an English program on the first run with no
///   reason anywhere, so this one is journaled — under the row the two calls of
///   [`preferred_ui_languages`] already journal under, `GetUserPreferredUILanguages` (the
///   vocabulary is not widened), with the code Windows itself gives a buffer too small for the
///   answer, `ERROR_INSUFFICIENT_BUFFER`: it is what happened, and it keeps the entry apart from
///   a refusal of the call itself.
///
/// **SEC-01, SEC-07.** The fact and that number, and nothing else: not the count of units, not a
/// tag — the multi-string is never read on this road at all.
///
/// Pure in its answer, and public so that a test can hand it the two sizes a live machine does
/// not give on demand.
pub fn preferred_languages_size_is_readable(units: u32) -> bool {
    if units > PREFERRED_LANGUAGES_CAP {
        crate::app::report_non_critical(
            "GetUserPreferredUILanguages",
            &WinError::from_hresult(
                windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER.to_hresult(),
            ),
        );

        return false;
    }

    units != 0
}

/// The primary subtag of a BCP-47 language tag — `de` of `de-AT`, `uk` of `uk-Cyrl-UA`.
///
/// The primary subtag is the first one by definition, so the rule is «everything before the
/// first separator», and it needs to know nothing about scripts, regions or variants. The
/// underscore is admitted beside the hyphen because a tag copied out of a registry value or an
/// environment variable is often written `pt_BR`, and treating that as one long primary subtag
/// would answer «not one of ours» to a language this build has.
fn primary_subtag(tag: &str) -> &str {
    match tag.find(['-', '_']) {
        Some(at) => &tag[..at],
        None => tag,
    }
}

/// The primary subtag of Norwegian as one macrolanguage — `no`, older than the two tags of its two
/// written forms, `nb` and `nn` (stage E109). [`language_of_tag_among`] answers it as Bokmål.
pub const NORWEGIAN_MACROLANGUAGE: &str = "no";

/// The locale one BCP-47 language tag names, if this build has it — **вопрос 95**.
///
/// The mapping is by [`primary_subtag`] and by nothing else: `de-DE`, `de-AT` and a bare `de`
/// are all German, and `pt-PT` is the same `pt` as `pt-BR`. That is a decision and not a
/// simplification. Section 7 spells a locale in two letters, this build ships one translation
/// per language, and a table of regions would have to name every country Windows can be set to
/// in order to say the same thing.
///
/// ⚠ **Deliberately not matched through [`Language::langid`].** Those numbers carry a
/// *sublanguage*: `Pt` is `0x0416`, which is `pt-BR`, so a machine set to `pt-PT` (`0x0816`)
/// would fail to match and fall to English — a person whose Windows is Portuguese would be
/// handed an English program while the file `pt` was sitting in the build. The tag is the right
/// joint here, and the identifier stays what it is: the number `app.rc` tags a string table
/// with.
///
/// Case-insensitive, because case carries no meaning in a language tag: `DE-de` is the tag
/// `de-DE` written by somebody who did not know the convention.
///
/// ⭐ **Stage E108, task T-108-2, решение 200б.1 — Serbian and Uzbek are two languages each, and
/// the primary subtag cannot tell them apart.** `sr` is Serbian in either script, `uz` Uzbek in
/// either. For these the **script subtag** of the full tag decides: Windows answers
/// `sr-Cyrl-RS` / `sr-Latn-RS` and `uz-Latn-UZ` / `uz-Cyrl-UZ`
/// (`GetUserPreferredUILanguages`, and `LCIDToLocaleName` of a layout). A tag whose script is the
/// one of a language's own tag (`sr-Latn`, `uz-Cyrl`) is that language; any other script, or
/// none — `sr`, `sr-RS`, `uz-UZ` — is the main one, whose tag is the bare subtag (Serbian
/// Cyrillic, Uzbek Latin). Only languages the build offers ([`Language::shipped`]) are answered;
/// if the main one is not among them, the other one of the same language is better than English.
///
/// ⭐ **Stage E109, task T-109-1 — Norwegian is two languages with two primary subtags of their
/// own**, `nb` (Bokmål, the main one) and `nn` (Nynorsk), so `nb-NO` and `nn-NO` need no rule. The
/// one rule is for the old macrolanguage `no` (`no`, `no-NO`), which a Windows or a copied tag may
/// still name: it is Bokmål, the form [`NORWEGIAN_MACROLANGUAGE`] stands for. Bosnian is one
/// language here, in the Latin script: `bs-Latn-BA` and `bs-Cyrl-BA` alike are `bs`.
pub fn language_of_ui_tag(tag: &str) -> Option<Language> {
    language_of_tag_among(tag, Language::shipped())
}

/// [`language_of_ui_tag`] over a given set of languages instead of the ones the build offers —
/// public so that a test drives the rule of the scripts over the whole closed set, Serbian and
/// Uzbek in both scripts, whatever the registry ships today.
pub fn language_of_tag_among(
    tag: &str,
    offered: impl IntoIterator<Item = Language>,
) -> Option<Language> {
    let primary = match primary_subtag(tag) {
        macrolanguage if macrolanguage.eq_ignore_ascii_case(NORWEGIAN_MACROLANGUAGE) => {
            Language::Nb.tag()
        }
        primary => primary,
    };

    // An empty primary subtag is not «no preference», it is a tag that was never a tag —
    // `""`, `"-"`, `"---"`. `eq_ignore_ascii_case` would answer `false` for every language
    // anyway; the early return says so on purpose rather than by luck.
    if primary.is_empty() {
        return None;
    }

    let script = script_subtag(tag);
    let mut main = None;
    let mut other = None;

    for language in offered {
        let own = language.tag();

        if !primary.eq_ignore_ascii_case(primary_subtag(own)) {
            continue;
        }

        match script_subtag(own) {
            None => main = Some(language),
            Some(own_script) if script.is_some_and(|s| s.eq_ignore_ascii_case(own_script)) => {
                return Some(language);
            }
            Some(_) => other = other.or(Some(language)),
        }
    }

    main.or(other)
}

/// The script subtag of a BCP-47 tag, if it has one — `Latn` of `sr-Latn-RS`, `Cyrl` of
/// `uz-Cyrl`: the second subtag when it is four letters (RFC 5646 §2.2.3). `None` for `sr`,
/// `sr-RS`, `de-DE`.
fn script_subtag(tag: &str) -> Option<&str> {
    tag.split(['-', '_'])
        .nth(1)
        .filter(|subtag| subtag.len() == 4 && subtag.bytes().all(|b| b.is_ascii_alphabetic()))
}

/// The locale a **first run** starts in, given the preferred UI languages of Windows in order —
/// **вопрос 95**, and the whole of the rule the user asked for.
///
/// «Автоматически проверять язык системы и устанавливать по умолчанию языком интерфейса, если
/// имеется; если нет в списке локализаций нашей программы — по умолчанию ставить английский».
///
/// The list is walked in the order Windows keeps it and the **first** tag this build has wins.
/// Walking rather than looking only at the head is the reason the ordered list is asked for at
/// all: a person with Japanese first and German second gets the German program, which is a
/// better answer than English for somebody who has said in so many words that they read German.
///
/// English is the end of the list and not a fifteenth case: an empty list — a machine whose
/// answer this build could not read — takes the same road as a list of fourteen languages none
/// of which is ours, and both are «нет в списке локализаций нашей программы».
///
/// Pure, and public for that reason: `tests\settings.rs` drives it with staged tags, which is
/// the only honest way to check it on a machine whose own Windows is Russian.
pub fn first_run_language<'a>(preferred: impl IntoIterator<Item = &'a str>) -> Language {
    preferred
        .into_iter()
        .find_map(language_of_ui_tag)
        .unwrap_or(Language::En)
}

/// The interface languages of this user's Windows, most preferred first, as BCP-47 tags.
///
/// # Why this call and not `GetUserDefaultUILanguage`
///
/// Three reasons, and all three are about not inventing a table this program would then have to
/// maintain:
///
/// * it answers **tags**, which is what [`language_of_ui_tag`] matches on. The older call
///   answers one `LANGID`, and turning a `LANGID` into a two-letter tag means either a second
///   call to the locale service or a hand-written table of primary language identifiers —
///   a table that would have to be right for every language Windows ships, not only for the
///   fourteen this build has;
/// * it answers the **ordered list** the user actually configured. A person may have set
///   Japanese first and German second, and only a list can say so;
/// * it is the call that answers what the interface is *displayed* in, language packs included,
///   which is what вопрос 95 asks about.
///
/// # What a failure means here
///
/// An empty vector, and no more. NFR-13: the refusal is examined and journalled, and the
/// direction it is acted on in is the one that changes nothing the user can lose — an empty
/// list falls to `en` through [`first_run_language`], which is exactly the answer вопрос 95
/// prescribes for «нет в списке локализаций нашей программы». There is no file yet on this
/// road, so nothing whatever is at risk (this is the «файла нет» path and only that).
pub fn preferred_ui_languages() -> Vec<String> {
    let mut count = 0u32;
    let mut units = 0u32;

    // SAFETY: the sizing call of the documented two-call pattern. Both out-parameters are
    // locals of this frame, and `None` for the buffer is what asks for the size: the call
    // writes no characters at all on this road.
    if let Err(error) = unsafe {
        windows::Win32::Globalization::GetUserPreferredUILanguages(
            windows::Win32::Globalization::MUI_LANGUAGE_NAME,
            &raw mut count,
            None,
            &raw mut units,
        )
    } {
        crate::app::report_non_critical("GetUserPreferredUILanguages", &error);
        return Vec::new();
    }

    // Task T-43-10, finding Н71: «Windows named nothing» stays silent, «longer than the cap» is
    // journaled — both inside the one function the test drives.
    if !preferred_languages_size_is_readable(units) {
        return Vec::new();
    }

    let mut buffer = vec![0u16; units as usize];

    // SAFETY: the fetching call. `buffer` is owned by this frame, is not moved or dropped
    // until the call returns, and its length is the very number the sizing call above asked
    // for and `units` still holds. The call writes at most that many units and terminates the
    // multi-string itself.
    if let Err(error) = unsafe {
        windows::Win32::Globalization::GetUserPreferredUILanguages(
            windows::Win32::Globalization::MUI_LANGUAGE_NAME,
            &raw mut count,
            Some(PWSTR(buffer.as_mut_ptr())),
            &raw mut units,
        )
    } {
        crate::app::report_non_critical("GetUserPreferredUILanguages", &error);
        return Vec::new();
    }

    // A double-NUL-terminated multi-string: the runs between the zeros are the tags, and the
    // empty run at the end is the second terminator. `units` is clamped to what was actually
    // allocated — the second call reports the length it wrote, and a number larger than the
    // buffer would be a number this code has no business trusting.
    buffer[..(units as usize).min(buffer.len())]
        .split(|unit| *unit == 0)
        .filter(|run| !run.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

/// The locale a first run starts in **on this machine** — [`preferred_ui_languages`] handed to
/// [`first_run_language`], and nothing else.
///
/// Split from the rule so that the rule can be tested without a machine and the machine can be
/// asked without a rule: `tests\settings.rs` does both, and neither test can hide a mistake in
/// the other half.
pub fn system_language_for_a_first_run() -> Language {
    first_run_language(preferred_ui_languages().iter().map(String::as_str))
}

// =========================================================================================
// Решение 202.7, stage E108, task T-108-2 — the order of the combo «Язык интерфейса»
// =========================================================================================

/// The rows of the combo «Язык интерфейса» in the order they are shown — решение 202.7.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LanguageOrder {
    /// Every language the build offers, once each: the group on top, then the rest.
    pub languages: Vec<Language>,
    /// How many of them the group on top holds — the line lies along the bottom edge of the last
    /// of them, and there is no line when nothing is below.
    pub top: usize,
}

/// **The order of the combo «Язык интерфейса», вариант 1 of решение 202.7** — дословно владельца:
/// «вначале системный, потом английски, языки раскладок пользователя, русский и по далее по
/// самоназванию».
///
/// On top: the language of Windows, English, the languages of the person's layouts in the order of
/// the session, Russian — each once, and only languages the build offers. Below the line: every
/// other language the build offers, in the order of [`Language::BY_NAME`]. The combo stores the
/// language and not the row (`read_dialog`), so the order may change from one opening to the next
/// — a layout added, Windows switched to another language — and nothing stored moves.
///
/// Pure, for the test of the order with a given language of Windows and given layouts;
/// [`language_order_on_this_machine`] asks the machine.
pub fn language_order(windows: Option<Language>, layouts: &[Language]) -> LanguageOrder {
    let mut languages = Vec::new();

    let on_top = windows
        .into_iter()
        .chain([Language::En])
        .chain(layouts.iter().copied())
        .chain([Language::Ru]);

    for language in on_top {
        if language.is_shipped() && !languages.contains(&language) {
            languages.push(language);
        }
    }

    let top = languages.len();

    for language in Language::BY_NAME {
        if language.is_shipped() && !languages.contains(&language) {
            languages.push(language);
        }
    }

    LanguageOrder { languages, top }
}

/// [`language_order`] with the language of Windows and the layouts of `session` — asked once per
/// opening of the settings (решение 202.7: «вычисляется при каждом открытии»).
///
/// The language of Windows is the one the first run would pick ([`first_run_language`] without
/// its English at the end): the first preferred interface language the build offers. A layout
/// speaks the language of its locale (`LCIDToLocaleName` of its language identifier), matched by
/// the same rule as Windows' own tags — so `uz-Cyrl-UZ` is Uzbek in Cyrillic.
/// The layouts the instrument of fitting describes the window of the settings over — three
/// common ones, so that the window is measured with the names a person sees there (T-120-20).
pub fn layouts_for_fitting() -> Vec<LayoutId> {
    vec![
        LayoutId::from_raw(0x0409_0409),
        LayoutId::from_raw(0x0419_0419),
        LayoutId::from_raw(0x0407_0407),
    ]
}

pub fn language_order_on_this_machine(session: &[LayoutId]) -> LanguageOrder {
    let windows = preferred_ui_languages()
        .iter()
        .find_map(|tag| language_of_ui_tag(tag));
    let layouts: Vec<Language> = session
        .iter()
        .filter_map(|layout| language_of_langid(layout.language_id()))
        .collect();

    language_order(windows, &layouts)
}

/// The language of the build a Windows language identifier names, if any — the locale name of
/// `langid` (`LCIDToLocaleName`) read by [`language_of_ui_tag`].
fn language_of_langid(langid: u16) -> Option<Language> {
    let mut locale = [0u16; 85];

    // SAFETY: `locale` is a live buffer of this frame and the call is told its length by the
    // slice, so it cannot write past the end. The identifier is a plain integer. The answer is the
    // number of units written including the terminator, zero on failure, checked below (NFR-13).
    let written = unsafe {
        windows::Win32::Globalization::LCIDToLocaleName(u32::from(langid), Some(&mut locale), 0)
    };

    let units = usize::try_from(written).ok()?.checked_sub(1)?;

    language_of_ui_tag(&String::from_utf16_lossy(locale.get(..units)?))
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
    /// The flag of the layout on the tray icon — task T-119-2, решения 208.88, 208.89, the check
    /// box «Флаг раскладки в трее» of FR-92. Default `false`, and **off means 1.6.1 to the
    /// letter**: no flag, no new subscription, hook or thread (the guard is in `tests\tray.rs`).
    ///
    /// ⚠ `schema_version` does not move for it — the reasoning of `idle_timeout_s` in section 7:
    /// there is no `deny_unknown_fields` here, so an older build reads the file past the key, and
    /// its save drops it, which is the default and a build that has no flag anyway.
    #[serde(default)]
    pub tray_flag: bool,
}

impl Default for General {
    fn default() -> Self {
        Self {
            enabled: true,
            autostart: true,
            language: Language::Ru,
            theme: ThemeSetting::System,
            tray_flag: false,
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
    /// **The modifiers held with the key — task T-93-1, вопрос 157.** Empty — the default and
    /// what every file before schema 7 means — is the bare key.
    ///
    /// Names from the closed set `Ctrl`, `Alt`, `Shift` (`hook::HOTKEY_MODIFIERS`); the program
    /// writes them in that order, and a capture never writes anything else. Kept as strings for
    /// the reason [`Self::key`] is: the file is also written by hand, a name nobody knows must not
    /// cost the whole document, and the reading is `hook`'s — [`Self::binding`] answers `None`
    /// for an unknown name and the **whole** hotkey stays the default `Pause`, exactly as for an
    /// unknown key.
    #[serde(default)]
    pub modifiers: Vec<String>,
    /// **How the key is pressed — task T-95-1, вопрос 159**: [`TRIGGER_PRESS`], the default and
    /// what every file before schema 8 means, or [`TRIGGER_DOUBLE_TAP`] — two taps of `Shift` in a
    /// row (157.17: the double press is `Shift`'s alone). Always written, as `modifiers = []` is.
    ///
    /// A string for the reason [`Self::key`] is one: a word nobody knows must not cost the whole
    /// document. [`Self::binding`] answers `None` for it and the **whole** hotkey stays the default
    /// `Pause` — the fate of an unknown key name; and `double_tap` is lawful with `key = "Shift"`
    /// and no modifiers only, with anything else it is the same `None`.
    #[serde(default = "default_hotkey_trigger")]
    pub trigger: String,
}

impl Default for Hotkey {
    fn default() -> Self {
        Self {
            key: default_hotkey_key(),
            modifiers: Vec::new(),
            trigger: default_hotkey_trigger(),
        }
    }
}

impl Hotkey {
    /// The code and the modifier set this hotkey means — task T-93-1, вопрос 157 — or `None` when
    /// the key or one of the modifiers is a name this build does not know.
    ///
    /// `None` is read by every caller as «the default of section 7 acts»: the start-up publication
    /// leaves `Pause` in place (`app::publish_configuration`), the help names `Pause`
    /// ([`effective_hotkey_name`]) and the settings window says why ([`hotkey_note`]).
    ///
    /// ⭐ **The double press of `Shift` — task T-95-1, вопрос 159**: `trigger = "double_tap"` with
    /// `key = "Shift"` and no modifiers means `VK_SHIFT` with the flag `hook::DOUBLE_TAP` and
    /// nothing else; the trigger with anything else, and a word outside its closed set, is `None`
    /// like an unknown name — the whole hotkey stays `Pause`. A single `Shift` is no hotkey
    /// (157.1): `hook::vk_from_name` knows no modifier, so `key = "Shift"` that is pressed is `None`
    /// too. Names are read the way key names are, in any case and with the blanks around ignored.
    pub fn binding(&self) -> Option<(u16, u16)> {
        let modifiers = crate::hook::modifiers_from_names(&self.modifiers)?;
        let trigger = self.trigger.trim();

        if trigger.eq_ignore_ascii_case(TRIGGER_DOUBLE_TAP) {
            let lawful = modifiers == 0 && self.key.trim().eq_ignore_ascii_case(DOUBLE_TAP_KEY);

            return lawful.then_some((VK_SHIFT.0, crate::hook::DOUBLE_TAP));
        }

        if !trigger.eq_ignore_ascii_case(TRIGGER_PRESS) {
            return None;
        }

        let vk = crate::hook::vk_from_name(&self.key)?;

        Some((vk, modifiers))
    }

    /// The `[hotkey]` that means `vk` with `modifiers` — task T-93-1: the key under the name
    /// section 7 stores ([`key_name`]) and the modifiers **in the canonical order** `Ctrl`, `Alt`,
    /// `Shift` (`hook::HOTKEY_MODIFIERS`), whatever order they were pressed in.
    ///
    /// `None` for a key section 7 has no name for, and for a set with a bit no hotkey carries —
    /// `hook::MOD_WIN` is held, never written.
    ///
    /// The flag `hook::DOUBLE_TAP` reads back to [`Self::double_shift`] — with a `Shift` and
    /// nothing else beside it (task T-95-1); with anything else it is `None`.
    pub fn from_binding(vk: u16, modifiers: u16) -> Option<Self> {
        if modifiers & crate::hook::DOUBLE_TAP != 0 {
            let shift = SHIFT_KEYS.contains(&vk);

            return (shift && modifiers == crate::hook::DOUBLE_TAP).then(Self::double_shift);
        }

        let known = crate::hook::HOTKEY_MODIFIERS
            .iter()
            .fold(0, |set, (_, bit)| set | bit);

        if modifiers & !known != 0 {
            return None;
        }

        Some(Self {
            key: key_name(vk)?,
            modifiers: crate::hook::HOTKEY_MODIFIERS
                .iter()
                .filter(|(_, bit)| modifiers & bit != 0)
                .map(|(name, _)| (*name).to_owned())
                .collect(),
            trigger: default_hotkey_trigger(),
        })
    }

    /// Whether this hotkey is the double press of `Shift` — what [`Self::binding`] reads, so a
    /// `double_tap` that is not lawful is not one (task T-95-2).
    pub fn is_double_tap(&self) -> bool {
        self.binding()
            .is_some_and(|(_, set)| set & crate::hook::DOUBLE_TAP != 0)
    }

    /// The double press of `Shift` as section 7 stores it — `key = "Shift"`, `modifiers = []`,
    /// `trigger = "double_tap"` (task T-95-1, вопрос 159): what a capture of two taps writes.
    pub fn double_shift() -> Self {
        Self {
            key: DOUBLE_TAP_KEY.to_owned(),
            modifiers: Vec::new(),
            trigger: TRIGGER_DOUBLE_TAP.to_owned(),
        }
    }
}

/// `[hotkey] trigger` of a hotkey that is pressed — the default of section 7 (task T-95-1).
pub const TRIGGER_PRESS: &str = "press";

/// `[hotkey] trigger` of the double press of `Shift` — task T-95-1, вопрос 159.
pub const TRIGGER_DOUBLE_TAP: &str = "double_tap";

/// The one key a double press is assigned to — 157.17, «Только Shift»: the name section 7 stores
/// for it, and each half of the name the windows show, «Shift, Shift» (task T-95-1).
pub const DOUBLE_TAP_KEY: &str = "Shift";

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
    /// Pause inserted between synthesised input events, milliseconds. Default `0`. Soft —
    /// решение 120.2, `soft_whole_number`.
    #[serde(default, deserialize_with = "soft_inter_event_delay_ms")]
    pub inter_event_delay_ms: u32,
}

/// Section `[selection]` of section 7.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Whether the selection path is available at all. Default `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// How long to wait for the clipboard to carry the selection, milliseconds.
    /// Default `300`. Soft — решение 120.2, `soft_whole_number`.
    #[serde(
        default = "default_clipboard_timeout_ms",
        deserialize_with = "soft_clipboard_timeout_ms"
    )]
    pub clipboard_timeout_ms: u32,
    /// How long to wait before restoring the previous clipboard, milliseconds.
    /// Default `200`. Soft — решение 120.2, `soft_whole_number`.
    #[serde(
        default = "default_clipboard_restore_delay_ms",
        deserialize_with = "soft_clipboard_restore_delay_ms"
    )]
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
    /// Capacity of the typing buffer in characters. Default `256`. Soft — решение 120.2,
    /// `soft_whole_number`.
    #[serde(
        default = "default_buffer_capacity",
        deserialize_with = "soft_buffer_capacity"
    )]
    pub capacity: usize,
    /// **How long a word may sit in the buffer with nothing happening to it, seconds — FR-15**,
    /// task T-52-4. Default `300`; `0` switches the rule off.
    ///
    /// Published clamped to [`crate::buffer::MIN_IDLE_TIMEOUT_S`] …
    /// [`crate::buffer::MAX_IDLE_TIMEOUT_S`], with `0` kept as `0`. The file may hold anything —
    /// it is edited by hand — and the ceilings are applied where the value crosses to the input
    /// thread, exactly as the three millisecond fields of task T-13-13 are.
    ///
    /// ⚠ **`schema_version` did not move for this key, and 6 is deliberate.** There is no
    /// `deny_unknown_fields` in this module, so a file written by this build is read by the
    /// previous one without complaint — the key is simply ignored there. Raising the schema to
    /// 7 would do the opposite: a user who installed this build and then went back to `e51`
    /// would find their configuration quarantined as «из будущего»
    /// ([`ReadOutcome::FromNewerSchema`]), which is a real cost paid for a key whose absence
    /// already has a defined meaning.
    ///
    /// Soft — решение 120.2, `soft_whole_number`: a value that is not a whole number this field
    /// can hold is its default, and costs nothing else of the file.
    #[serde(
        default = "default_buffer_idle_timeout_s",
        deserialize_with = "soft_buffer_idle_timeout_s"
    )]
    pub idle_timeout_s: u32,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            capacity: default_buffer_capacity(),
            idle_timeout_s: default_buffer_idle_timeout_s(),
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

/// Section `[feedback]` of section 7 — **FR-100**, the sound of a press.
///
/// The user asked for it (question 77, finding №10 of the audit of 2026-08-31): a press that
/// does nothing is indistinguishable from a press that worked, because the program's whole
/// output is text somebody else's window draws. One sound says the replacement happened and
/// another says the press was idle.
///
/// Default **on**, which is not the default of `bool` — hence `default_true`, exactly as
/// `[selection] enabled` does it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feedback {
    /// Whether a press answers with a system sound. Default `true`.
    #[serde(default = "default_true")]
    pub sound: bool,
}

impl Default for Feedback {
    fn default() -> Self {
        Self { sound: true }
    }
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

/// Section `[letters]` of section 7 — **FR-101 and FR-102**, вопрос 101, task Т-32-1.
///
/// The state of the five letters and of the feed: when this installation started counting,
/// which letters it has shown, which news it has read and when it last looked at the feed.
/// Nothing here is a keystroke, a window title or anything a person typed (SEC-01, SEC-07):
/// the section holds dates, two version strings, a handful of flags and the numbers the author
/// gave the entries of the feed.
///
/// # Only one field of this section is meant to be edited by hand
///
/// `feed` is, and section 7 says so out loud: for the first ninety days it is the **only** way
/// to turn the feed off (FR-102), because the switch in the «От автора» window does not appear
/// until the feed has actually produced a letter. The rest is bookkeeping — a person is free to
/// edit it, and the worst that comes of it is a letter shown twice or not at all.
///
/// # The order of the fields is load-bearing
///
/// TOML puts every plain value of a table before its sub-tables, so the two maps go last and
/// this section goes last in [`Config`]. A map written above a scalar would make the file
/// unreadable to the very parser that wrote it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Letters {
    /// Whether the author's feed is read at all — FR-102, and **on** by default (вопрос 101
    /// п. 3). This is the field SEC-03 names as the way to refuse the one network operation
    /// this program has.
    #[serde(default = "default_true")]
    pub feed: bool,

    /// The day this installation was first used, from which every cadence of FR-101 counts.
    ///
    /// Empty on a file that has never been used — a fresh install and a file just raised from
    /// schema 5 alike — and filled by [`crate::letters::initialise`] on the first run, which
    /// is the day the migration happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_run: Option<letters::Date>,

    /// Whether «Привет» has been shown. False only on a machine that had no configuration file
    /// at all (FR-101); the migration of schema 5 → 6 sets it true, because a machine that is
    /// being updated has been used.
    #[serde(default)]
    pub welcome_shown: bool,

    /// The state of the «Спасибо» letter — `pending`, `snoozed` or `done`.
    #[serde(
        default,
        deserialize_with = "thanks_from_toml",
        serialize_with = "thanks_to_toml"
    )]
    pub thanks: letters::Thanks,

    /// The day «Спасибо» becomes due: thirty days from the first run, or seven from a snooze.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thanks_due: Option<letters::Date>,

    /// The version «Что нового» was last shown for. Empty means «show it once», which is what
    /// the migration leaves behind for a person who has just updated.
    #[serde(default)]
    pub last_seen_version: String,

    /// The day the last letter was put on the screen — the one-letter-a-day rule of FR-101.
    ///
    /// ⚠ **Not in the mock-up's печать of section 7, and added by the executor of Э32 with the
    /// reason written down in the report.** Every other letter is stopped from coming back by a
    /// mark of its own — the version it was about, the day it was first shown, `done` — and
    /// none of those stops *another* letter from following it an hour later, across a restart,
    /// on the same day. FR-101 says «не более одного письма в сутки», and a rule that has to
    /// survive a restart needs a day written in the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_letter: Option<letters::Date>,

    /// The day the feed was last read **and its signature verified** — FR-102. A failed read
    /// does not move it; what keeps a broken host from being asked again the same day is
    /// `feed_last_try` below, and since решение 189.2 the next good read is due the next day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed_last_read: Option<letters::Date>,

    /// The day the feed was last **asked**, whatever came of it — FR-102, task T-88-1, решение
    /// 148.8.
    ///
    /// Written by the tick of the letters **before** the read starts. `feed_last_read` alone
    /// cannot say «already asked today»: a failed read does not move it, so until `e88` a host
    /// that did not answer was asked again at every tick, once an hour. A day in the file, like
    /// `last_letter` above, because a rule that has to survive a restart needs one.
    ///
    /// ⚠ **Not in the печать of section 7 of `SPEC.md`**, which stage Э88 does not touch
    /// (решение 148.5). The schema stays 6: an older build ignores the key, as it ignored
    /// `idle_timeout_s`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed_last_try: Option<letters::Date>,

    /// The day the first letter out of the feed was shown — half of the condition the feed
    /// switch of FR-102 appears on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_feed_letter: Option<letters::Date>,

    /// The newest version the «Обновление» letter has already been shown for — FR-101, «один
    /// раз на версию».
    #[serde(default)]
    pub latest_known: String,

    /// The day a read of the feed first named a version newer than this build — TZ E104 §0
    /// п. 20, task T-104-7: since stage Э104 the updater installs a new version by itself, and
    /// the letter «Обновление» with the entry «Доступна версия X…» are its **fallback**, shown
    /// only when the feed has named the newer version for [`letters::UPDATE_FALLBACK_DAYS`] days
    /// and this build is still the old one. Cleared by the first read that names nothing newer.
    ///
    /// ⚠ Not in the печать of section 7 and **no schema of its own**, as `feed_last_try` before
    /// it: there is no `deny_unknown_fields` here, an older build reads the file past this key,
    /// and if it drops the key on a write the cost is three more days of the fallback's wait.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_noticed: Option<letters::Date>,

    /// Whether the window «Обновлено» comes up after an important release — task T-119-6
    /// (решения 208.93, 208.119). The check box «Больше не показывать окна обновлений» of that
    /// window and the switch «Сообщать об обновлениях программы» of «От автора» (T-119-10) are this
    /// one setting; it silences only that window — the entry of the menu and the fallback letter
    /// stay. On by default. No schema of its own, as `update_noticed` before it.
    #[serde(default = "default_true")]
    pub release_window: bool,

    /// While the window «Обновлено» is due and not yet shown, the version the program was updated
    /// **from** («Было Y» of its text) — task T-119-6: written when the start after an important
    /// update finds it due, emptied the moment the window is shown — so it is shown **once**, even
    /// when it was never closed and the computer restarted, and still shown when the program
    /// stopped before its quiet moment came. Empty — nothing is due.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub release_due: String,

    /// The day of the last update — task T-119-7: the entry «Что нового в X…» of the menu stands
    /// for a week from it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_on: Option<letters::Date>,

    /// ⭐ **The mark of the first run — stage E116, task T-116-12, решение 208.59.** `due` only in
    /// a file this run created ([`read_from_via`], no file at all); `sent` after any HTTP answer to
    /// the read of the feed that carried `?first=1`. Absent — a file of an older build, an update —
    /// means never. See [`letters::FirstMark`].
    ///
    /// ⚠ Not in the печать of section 7 and **no schema of its own**, as `feed_last_try` and
    /// `update_noticed` before it: an older build reads past the key, and an unknown word is read
    /// as no mark (never sent), not as damage.
    #[serde(
        default,
        deserialize_with = "first_mark_from_toml",
        serialize_with = "first_mark_to_toml",
        skip_serializing_if = "Option::is_none"
    )]
    pub first_mark: Option<letters::FirstMark>,

    /// The identifiers of the news items that have been read — FR-101, and «Прочитано» is the
    /// only thing that ever puts one here.
    #[serde(default)]
    pub read_ids: Vec<u64>,

    /// How many reminders each unread news item has had — at most
    /// [`crate::letters::REMINDERS_PER_NEWS`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub reminders: letters::NewsMap<u32>,

    /// The day each news item was **first** shown, which is what its two reminders are counted
    /// from.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub first_shown: letters::NewsMap<letters::Date>,
}

impl Default for Letters {
    /// The section as section 7 prints it for a machine that has never run this program: the
    /// feed on, nothing shown, nothing read, no day counted from yet.
    fn default() -> Self {
        Self {
            feed: true,
            first_run: None,
            welcome_shown: false,
            thanks: letters::Thanks::Pending,
            thanks_due: None,
            last_seen_version: String::new(),
            last_letter: None,
            feed_last_read: None,
            feed_last_try: None,
            first_feed_letter: None,
            latest_known: String::new(),
            update_noticed: None,
            release_window: true,
            release_due: String::new(),
            updated_on: None,
            first_mark: None,
            read_ids: Vec::new(),
            reminders: BTreeMap::new(),
            first_shown: BTreeMap::new(),
        }
    }
}

impl Letters {
    /// Whether this news item has been read — FR-101.
    pub fn is_read(&self, id: u64) -> bool {
        self.read_ids.contains(&id)
    }

    /// Marks a news item read. Idempotent: «Прочитано» pressed twice is one entry.
    pub fn mark_read(&mut self, id: u64) {
        if !self.is_read(id) {
            self.read_ids.push(id);
            self.read_ids.sort_unstable();
        }

        self.reminders.remove(&letters::key_of(id));
    }

    /// The day this news item was first shown, if it has been.
    pub fn first_shown_on(&self, id: u64) -> Option<letters::Date> {
        self.first_shown.get(&letters::key_of(id)).copied()
    }

    /// Records the day a news item was first shown.
    pub fn set_first_shown(&mut self, id: u64, day: letters::Date) {
        self.first_shown.insert(letters::key_of(id), day);
    }

    /// How many reminders this news item has already had.
    pub fn reminders_sent(&self, id: u64) -> u32 {
        self.reminders
            .get(&letters::key_of(id))
            .copied()
            .unwrap_or(0)
    }

    /// Counts one more reminder for this news item, and never past
    /// [`crate::letters::REMINDERS_PER_NEWS`] — a saturating count, because the number is read
    /// back out of a file anybody may edit (SEC-05).
    pub fn count_reminder(&mut self, id: u64) {
        let sent = self.reminders_sent(id);

        self.reminders.insert(
            letters::key_of(id),
            sent.saturating_add(1).min(letters::REMINDERS_PER_NEWS),
        );
    }

    /// Records that the feed has produced a letter — the day of the **first** one, which is
    /// half of the condition the switch of FR-102 appears on, and it is never moved again.
    pub fn note_feed_letter(&mut self, day: letters::Date) {
        if self.first_feed_letter.is_none() {
            self.first_feed_letter = Some(day);
        }
    }

    /// Forgets every news item that is not in `kept` — FR-101: the news the feed no longer
    /// carries goes, and its reminders and its read mark go with it.
    ///
    /// Keys that are not numbers at all — a hand-edited file — are dropped by the same sweep:
    /// nothing can ever match them again, and leaving them would grow the file for ever.
    pub fn retain_news(&mut self, kept: &[u64]) {
        self.read_ids.retain(|id| kept.contains(id));
        self.reminders
            .retain(|key, _| letters::id_of(key).is_some_and(|id| kept.contains(&id)));
        self.first_shown
            .retain(|key, _| letters::id_of(key).is_some_and(|id| kept.contains(&id)));
    }
}

// Serde bridge for `letters.thanks`, on the terms `general.theme` is on: the vocabulary of the
// three words belongs to [`letters::Thanks`], and a derive here would mint a second spelling of
// them. Neither function knows a single word of it.

/// Reads `letters.thanks`: the word goes to the owner of the vocabulary, which answers for
/// every string — see [`letters::Thanks`] for why an unknown word is not an error here.
fn thanks_from_toml<'de, D>(deserializer: D) -> Result<letters::Thanks, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    Ok(letters::Thanks::from_config_str(&text))
}

/// Writes `letters.thanks`: the word is the owner's, verbatim.
fn thanks_to_toml<S>(thanks: &letters::Thanks, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(thanks.as_config_str())
}

/// Reads `letters.first_mark` (stage E116, task T-116-12): the word goes to the owner of the
/// vocabulary, [`letters::FirstMark`]; a word it does not know is no mark — never sent — and not
/// damage to the file.
fn first_mark_from_toml<'de, D>(deserializer: D) -> Result<Option<letters::FirstMark>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    Ok(letters::FirstMark::from_config_str(&text))
}

/// Writes `letters.first_mark`: the owner's word, verbatim; no mark is not written at all
/// (`skip_serializing_if`).
fn first_mark_to_toml<S>(
    mark: &Option<letters::FirstMark>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match mark {
        Some(mark) => serializer.serialize_str(mark.as_config_str()),
        None => serializer.serialize_none(),
    }
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
    /// Section `[feedback]` — FR-100.
    #[serde(default)]
    pub feedback: Feedback,
    /// Section `[diagnostics]`.
    #[serde(default)]
    pub diagnostics: Diagnostics,
    /// Section `[letters]` — FR-101 and FR-102.
    ///
    /// ⚠ **Last on purpose.** It is the only section holding maps, TOML writes a table's
    /// sub-tables after its plain values, and a section that follows one would be swallowed by
    /// it. See [`Letters`].
    #[serde(default)]
    pub letters: Letters,
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
            feedback: Feedback::default(),
            diagnostics: Diagnostics::default(),
            letters: Letters::default(),
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

/// Serde default for `hotkey.trigger` — [`TRIGGER_PRESS`]: a file before schema 8 has no such
/// field, and its hotkey is pressed (task T-95-1).
fn default_hotkey_trigger() -> String {
    TRIGGER_PRESS.to_owned()
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

/// Serde default for `buffer.idle_timeout_s` — **FR-15**, task T-52-4.
///
/// Five minutes. Long enough that a user who stopped to read something and came back finds the
/// word they were typing still there, short enough that a machine left alone is not keeping a
/// half-typed line in memory for the rest of the day (SEC-01).
fn default_buffer_idle_timeout_s() -> u32 {
    300
}

thread_local! {
    /// How many soft fields the last [`Config::from_toml_str`] on this thread read as their
    /// default — task T-55-7. Taken by the caller with [`take_softened_fields`].
    static SOFTENED: Cell<u32> = const { Cell::new(0) };
}

/// How many soft fields the last read of a configuration on this thread read as their default,
/// and zero from then on — task T-55-7, решение 120.2.
///
/// A side channel, and deliberately one: this module writes no journal, and the value of a field
/// that fell back to its default is, by construction, not in the configuration any more. The tray
/// reads the configuration on its own thread, takes this count right after the read and puts one
/// line in the journal per field. A read that failed as a whole leaves zero — nothing of it is
/// used.
pub fn take_softened_fields() -> u32 {
    SOFTENED.with(|count| count.replace(0))
}

/// Counts one soft field read as its default.
fn note_softened() {
    SOFTENED.with(|count| count.set(count.get().saturating_add(1)));
}

/// A number of section 7 that is soft — решение 120.2, task T-55-7.
///
/// **Soft means: a value of the document that is not a whole number this field can hold is the
/// default of this field**, and it is counted ([`take_softened_fields`]). A negative number, a
/// fraction, a string, a boolean, a number too large for the field — every one of them costs the
/// field and nothing else. Until that task any of them declared the whole document damaged, and the
/// hotkey, the exclusions, the cycle, the language and the theme went to quarantine with it.
///
/// **Soft is about the type, not the range.** A whole number the field can hold passes through as
/// it is, however large or small: the ceilings of `buffer` and `inject` take it to their bounds
/// where the value is published (finding Н49 — words in section 7, not code here), and softness
/// must not be mistaken for them.
///
/// **Five fields, by name, and no more:** `replacement.inter_event_delay_ms`,
/// `selection.clipboard_timeout_ms`, `selection.clipboard_restore_delay_ms`, `buffer.capacity` and
/// `buffer.idle_timeout_s` — the numbers решение 81 left to be edited by hand. The closed sets of
/// section 7 (`general.language`, `layouts.mode`, `replacement.method`) and `schema_version` stay
/// loud: a value outside a closed set must not pass silently, and a stamp read as a default would
/// blind the recognition of a file from a newer schema. `tests\settings.rs` sweeps this file for
/// exactly these five.
///
/// ⚠ A number with a unit glued to it (`300ms`) is not a value at all but a syntax error of the
/// document, which no field ever sees. That typo is answered by the narrow recovery of решение
/// 120.9, task T-55-7a: [`Config::from_toml_str`] takes the one line out of a copy of the text,
/// counts the field here, and reads again.
fn soft_whole_number<'de, D, T>(deserializer: D, default: T) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: TryFrom<i64>,
{
    let value = toml::Value::deserialize(deserializer)?;

    let whole = match value {
        toml::Value::Integer(number) => T::try_from(number).ok(),
        _ => None,
    };

    Ok(whole.unwrap_or_else(|| {
        note_softened();
        default
    }))
}

/// `replacement.inter_event_delay_ms`, soft — [`soft_whole_number`].
fn soft_inter_event_delay_ms<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, 0)
}

/// `selection.clipboard_timeout_ms`, soft — [`soft_whole_number`].
fn soft_clipboard_timeout_ms<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, default_clipboard_timeout_ms())
}

/// `selection.clipboard_restore_delay_ms`, soft — [`soft_whole_number`].
fn soft_clipboard_restore_delay_ms<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, default_clipboard_restore_delay_ms())
}

/// `buffer.capacity`, soft — [`soft_whole_number`].
fn soft_buffer_capacity<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, default_buffer_capacity())
}

/// `buffer.idle_timeout_s`, soft — [`soft_whole_number`].
fn soft_buffer_idle_timeout_s<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    soft_whole_number(deserializer, default_buffer_idle_timeout_s())
}

/// The five soft keys by their `[section]` and name — решение 120.9, task T-55-7a.
///
/// The fields [`soft_whole_number`] softens, spelled here the way they stand in the file, because
/// the recovery of [`Config::from_toml_str`] reads text and not fields. `tests\settings.rs` drives
/// every one of them with a syntax typo, and drives the closed sets, a soft name in a foreign
/// section, a dotted key and the stamp to show that no other line is ever taken out.
const SOFT_KEYS: [(&str, &str); 5] = [
    ("replacement", "inter_event_delay_ms"),
    ("selection", "clipboard_timeout_ms"),
    ("selection", "clipboard_restore_delay_ms"),
    ("buffer", "capacity"),
    ("buffer", "idle_timeout_s"),
];

/// The byte range of the line of `text` that holds `offset`, without its line break — **if** that
/// line assigns one of [`SOFT_KEYS`] in the canonical form, `key = …`, under its own `[section]`.
///
/// The narrow recovery of решение 120.9 asks this about the position a refused parse stopped at.
/// Everything that is not exactly that answers `None`, and the document is then as damaged as it
/// always was: a closed set written without its quotes, a soft name in another section, a dotted
/// key at the top level, a quoted key, a stamp with a unit glued to it.
fn soft_key_line_at(text: &str, offset: usize) -> Option<std::ops::Range<usize>> {
    let offset = offset.min(text.len());
    let start = text.get(..offset)?.rfind('\n').map_or(0, |at| at + 1);
    let end = text
        .get(offset..)?
        .find('\n')
        .map_or(text.len(), |at| offset + at);

    let (key, _) = text[start..end].split_once('=')?;
    let key = key.trim();

    // The section is the nearest header above the line: `[name]`, and never `[[name]]`.
    let header = text[..start]
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with('['))?;
    let section = header
        .strip_prefix('[')
        .filter(|rest| !rest.starts_with('['))?
        .split(']')
        .next()?
        .trim();

    SOFT_KEYS.contains(&(section, key)).then_some(start..end)
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
    /// The configuration the program starts with when it has **no usable file** — **вопрос 95**,
    /// task Т-29-3, and since task T-55-9 (findings С20 and Н67) every road on which the program
    /// lives by defaults.
    ///
    /// [`Config::default`] with one field chosen instead of fixed: `general.language` is the
    /// interface language of this user's Windows when this build has it, and `en` when it has
    /// not — [`system_language_for_a_first_run`].
    ///
    /// ⚠ **Who calls it, by name — and why that is still вопрос 95.** Until task T-55-9 this doc
    /// said «exactly one caller», the `NotFound` arm of [`read_from`], and every other road —
    /// a file that could not be read or parsed, a file from a newer schema, no `%APPDATA%` —
    /// reached [`Config::default`] and its hard `ru`: somebody who rolled a build back met a
    /// Russian menu, and on the road of a newer file could not even change it. The callers now
    /// are the roads on which **there is no file the program could use**:
    ///
    /// * the `NotFound` arm of [`read_from_via`] — no file at all, вопрос 95 itself;
    /// * the branch of [`Config::from_toml_str_via`] for a file from a newer schema that did not
    ///   parse — none of its fields were understood;
    /// * the `Err` arm of [`read_or_default_via`] — a file that could not be read or parsed;
    /// * `tray::Tray::install_at` with no `%APPDATA%` — nothing was read at all (Н67).
    ///
    /// **No byte of any file is touched on any of them.** «Существующий конфиг не трогается
    /// никогда» is about what the file holds, and it holds what it held; what changes is only the
    /// language the program speaks in memory while it has no usable file. A file that parses —
    /// current or migrated — is somebody's file, and a language missing from it is still the `ru`
    /// of section 7, never the system's.
    ///
    /// It is deliberately **not** `Default::default`. A `Default` that reads the operating
    /// system is a `Default` that answers differently on two machines, and this type is
    /// compared for equality all over the tests; the difference has to be visible at the call
    /// site, which is what a named constructor gives.
    pub fn for_a_first_run() -> Self {
        Self::for_a_first_run_in(system_language_for_a_first_run())
    }

    /// [`Config::for_a_first_run`] with the language handed in — task T-55-9. Pure, so that a test
    /// on a machine whose Windows is Russian can still tell the language of the system from the
    /// `ru` of section 7.
    pub fn for_a_first_run_in(language: Language) -> Self {
        Self {
            general: General {
                language,
                ..General::default()
            },
            ..Self::default()
        }
    }

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
        Self::from_toml_str_via(text, &system_language_for_a_first_run)
    }

    /// [`Config::from_toml_str`] with the language of a first run handed in — task T-55-9, finding
    /// С20.
    ///
    /// The one branch that asks for it is the file from a newer schema that did not parse: none of
    /// its fields were understood, so the program lives by defaults, and since that task their
    /// language is the system's ([`Config::for_a_first_run_in`]) rather than the hard `ru` of
    /// section 7 — a person who rolled a build back no longer meets a Russian menu that, on the
    /// road of a newer file, they could not even change. The argument is lazy and no other road
    /// calls it: a document that parses keeps the language it names, and no byte of the file is
    /// touched on any road. [`Config::from_toml_str`] hands in
    /// [`system_language_for_a_first_run`]; `tests\settings.rs` hands in a staged language, the
    /// only way to tell the two answers apart on a machine whose Windows is Russian.
    pub fn from_toml_str_via(
        text: &str,
        system_language: &dyn Fn() -> Language,
    ) -> Result<(Self, ReadOutcome), ConfigError> {
        // Task T-55-7: the count of softened fields belongs to this read and to nothing before it.
        SOFTENED.with(|count| count.set(0));

        let error = match toml::from_str::<Self>(text) {
            Ok(mut config) => {
                let outcome = config.migrate();
                return Ok((config, outcome));
            }
            Err(error) => error,
        };

        // ⭐ **Решение 120.9, task T-55-7a** — a syntax typo on the line of a soft number costs
        // that number and nothing else: see [`Config::without_broken_soft_lines`].
        if let Some(recovered) = Self::without_broken_soft_lines(text, &error) {
            return Ok(recovered);
        }

        // A read that failed as a whole uses nothing it softened on the way.
        SOFTENED.with(|count| count.set(0));

        // The parser refused. If the document names a schema this build is too old for, that
        // refusal is the expected answer to a file from the future and not a verdict on it.
        if let Some(version) = claimed_schema_version(text)
            && version > CURRENT_SCHEMA_VERSION
        {
            // The configuration of a first run, carrying the version the file claims — task
            // T-55-9, finding С20: its language is the system's. Nothing of the file itself is
            // kept — none of it was understood — and nothing of it is lost either:
            // `FromNewerSchema` forbids the write, so the bytes stay where they are.
            let config = Self {
                schema_version: version,
                ..Self::for_a_first_run_in(system_language())
            };

            return Ok((config, ReadOutcome::FromNewerSchema { version }));
        }

        Err(ConfigError::Malformed {
            at: error.span().map(|span| line_and_column(text, span.start)),
        })
    }

    /// The narrow recovery of **решение 120.9** — variant Б of решение 120.2, task T-55-7a.
    ///
    /// A soft number with a typo that breaks the syntax — `300ms`, `3 00` — is not a value its
    /// field could soften: the parser refuses the document before any field is read. So when the
    /// refused parse stopped on the line of a soft key in its own section ([`soft_key_line_at`]),
    /// that line is taken out of a copy of the text, the field is counted as softened, and the
    /// copy is read again — once per such line, and never more times than there are soft keys.
    /// The file is not touched: this is a copy in memory, and the person's typo stays on the disk
    /// where they can see it (an unchanged configuration is not written back, task T-55-4).
    ///
    /// Anything else — a stop on a line that is not a soft key, a copy that still refuses for a
    /// reason outside those lines — answers `None`, and the caller reports the **original**
    /// refusal, position included, exactly as before.
    fn without_broken_soft_lines(
        text: &str,
        error: &toml::de::Error,
    ) -> Option<(Self, ReadOutcome)> {
        let mut copy = text.to_owned();
        let mut stopped_at = error.span()?.start;

        for _ in 0..SOFT_KEYS.len() {
            let line = soft_key_line_at(&copy, stopped_at)?;

            copy.replace_range(line, "");
            note_softened();

            match toml::from_str::<Self>(&copy) {
                Ok(mut config) => {
                    let outcome = config.migrate();
                    return Some((config, outcome));
                }
                Err(next) => stopped_at = next.span()?.start,
            }
        }

        None
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
        if self.schema_version < 3 {
            step_2_to_3(self);
        }
        if self.schema_version < 4 {
            step_3_to_4(self);
        }
        if self.schema_version < 5 {
            step_4_to_5(self);
        }
        if self.schema_version < 6 {
            step_5_to_6(self);
        }
        if self.schema_version < 7 {
            step_6_to_7(self);
        }
        if self.schema_version < 8 {
            step_7_to_8(self);
        }
        if self.schema_version < 9 {
            step_8_to_9(self);
        }
        if self.schema_version < 10 {
            step_9_to_10(self);
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
/// ⚠ **The other half of that reading — решение 120.3, finding Т9.** A file without
/// `schema_version`, or with one below 2, is taken for one the program wrote, and nothing in it
/// can say otherwise: a `backspace` typed into such a file by hand is raised to `auto` the same
/// way, and the choice it stood for is lost. Section 7 of SPEC.md says so where a person reads
/// about the file.
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

/// Raises a file from schema 2 to schema 3 — **FR-100**, the section `[feedback]`.
///
/// **The stamp and nothing else, and that is the whole decision rather than an omission.** A
/// schema 2 file was written by a build that had never heard of `[feedback]`, so it carries no
/// such section — and the serde default of the one field it would carry is `true`, the same
/// value a fresh configuration of this build gets. There is therefore nothing to raise: the
/// absence of the section already means exactly what the new default means.
///
/// Contrast [`step_1_to_2`], which does have surgery to do: `backspace` in an old file was the
/// value the *program* had put there, so it had to be told apart from a decision a person made.
/// A section that never existed carries no decision of anybody's to preserve.
fn step_2_to_3(config: &mut Config) {
    config.schema_version = 3;
}

/// Raises a file from schema 3 to schema 4 — **вопрос 94.1**, the twelve locales of решение 93.
///
/// **The stamp and nothing else, and — as with [`step_2_to_3`] — that is the whole decision
/// rather than an omission.** Решение 93 added ten admissible **values** to `general.language`;
/// it renamed no field, retyped none and changed the meaning of none. So every value a schema 3
/// file can hold means under schema 4 exactly what it meant before, and there is nothing to
/// raise: `ru` is still `ru`, and a file that says nothing about the language still gets the
/// default of section 7.
///
/// The number is not therefore idle. It is spent entirely in the **other** direction, on a build
/// that has *not* been updated: from here on a file of the twelve locales carries a stamp no
/// two-locale build ever wrote, so such a build recognises it as a file from the future by the
/// stamp — before its parser refuses one of the ten new words — and leaves it whole instead of
/// moving it to `.bad`. That is the cost [`Language`] used to write down as accepted, and this
/// rung is where it stops being paid.
///
/// Contrast [`step_1_to_2`], the one rung that does have surgery to do: `backspace` in an old
/// file was a value the *program* had put there, so it had to be told apart from a decision a
/// person made. A value that was already the person's own carries no such ambiguity.
fn step_3_to_4(config: &mut Config) {
    config.schema_version = 4;
}

/// Raises a file from schema 4 to schema 5 — **решение 97.3**, Hebrew and Arabic.
///
/// **The stamp and nothing else**, for the same reason [`step_3_to_4`] is one rung down: вопрос
/// 97 adds two admissible **values** to `general.language` and renames no field, retypes none
/// and changes the meaning of none. Every value a schema 4 file can hold means under schema 5
/// exactly what it meant before.
///
/// And the number is spent in the same direction — on the build that has *not* been updated.
/// `Language` is a **closed** set, so a twelve-locale build meeting `language = "he"` refuses
/// the whole document; without a stamp it had never seen, it would read its own schema number
/// back out of the file, call it damaged, and move the person's configuration to `.bad`. That is
/// the cost вопрос 94.1 measured at the tier below and paid with schema 4; this rung is where it
/// is paid again, for the same coin, one tier up.
///
/// ⚠ The letters window of the parallel work planned schema 5 for itself and moves on to **6**
/// (вопрос 97.3): two windows cannot spend the same number.
fn step_4_to_5(config: &mut Config) {
    config.schema_version = 5;
}

/// Raises a file from schema 5 to schema 6 — **вопрос 101**, the section `[letters]` of FR-101
/// and FR-102.
///
/// **The first rung since [`step_1_to_2`] that carries a decision rather than a stamp**, and
/// the decision is about who the file belongs to: somebody who has been using this program
/// already. Three things follow from that, and the mandate of Э32 names all three.
///
/// * `welcome_shown = true`. «Привет» is the letter of a first run, and this is not one. The
///   absence of the section would otherwise read as «never shown» — which is exactly what it
///   means on a machine with no file at all, and exactly what it must not mean here.
/// * `last_seen_version` left **empty**. A person who has just updated is the one person «Что
///   нового» is written for, and an empty version differs from every build's, so they meet it
///   once and then never again.
/// * `thanks` and the rest left at their defaults. `first_run` stays empty and is filled on the
///   first run after the migration — [`crate::letters::initialise`] — which is the day of the
///   migration, because this function runs while the program is starting. It is not written
///   here for one reason: [`Config::migrate`] is a pure function of the document, it reads no
///   clock, and every test of the ladder would otherwise answer differently tomorrow.
///
/// ⚠ The `feed` field is left at its default, which is **on** (вопрос 101 п. 3). That is the
/// one thing in this rung that is a change of behaviour for an existing installation, and it is
/// the decision the user took: the feed is announced in the «От автора» window from the first
/// day, refusable in this very file from the first day, and refusable with a switch once it has
/// actually shown a letter.
fn step_5_to_6(config: &mut Config) {
    config.letters.welcome_shown = true;
    config.letters.last_seen_version = String::new();
    config.schema_version = 6;
}

/// Raises a file from schema 6 to schema 7 — **вопрос 157**, task T-93-1: `[hotkey] modifiers`.
///
/// **The stamp and nothing else, and that is the whole decision.** A schema 6 file carries no
/// `modifiers`, and its absence reads as the empty list — the bare key, which is exactly what a
/// schema 6 hotkey was. There is nothing to raise.
///
/// The number is spent in the other direction, as [`step_3_to_4`]'s was. There is no
/// `deny_unknown_fields` in this module, so a build that has never heard of the list would read a
/// file of the combinations as one of its own, answer to the bare key, and — under an unchanged
/// stamp — drop the list the next time it writes the file (on «Применить», on the date of the
/// letters). Under a stamp it has never seen, it reads the file and leaves it alone
/// ([`ReadOutcome::FromNewerSchema`], [`SavePolicy::Forbidden`]).
fn step_6_to_7(config: &mut Config) {
    config.schema_version = 7;
}

/// Raises a file from schema 7 to schema 8 — **вопрос 159**, task T-95-1: `[hotkey] trigger`.
///
/// **The stamp and nothing else**, as [`step_6_to_7`] one rung down: a schema 7 file carries no
/// `trigger`, and its absence reads as `press` — the hotkey a schema 7 file meant. There is nothing
/// to raise.
///
/// The number is spent in the other direction once more. `0.93.0` has no `deny_unknown_fields`
/// either, so under an unchanged stamp it would read a file of the double press, find a key named
/// `Shift` it does not know, answer to `Pause` — and drop the trigger the next time it writes the
/// file. Under a stamp it has never seen, it reads the file and leaves it alone
/// ([`ReadOutcome::FromNewerSchema`], [`SavePolicy::Forbidden`]).
fn step_7_to_8(config: &mut Config) {
    config.schema_version = 8;
}

/// Raises a file from schema 8 to schema 9 — **stage E108**, task T-108-2: the thirty languages
/// of group A in the closed set of `general.language`.
///
/// **A bare stamp**, as [`step_3_to_4`] and [`step_4_to_5`] were for the same reason: nothing in a
/// schema 8 file changes meaning — it names one of the fourteen, and they are all still here. The
/// number buys the other direction (память `closed-enum-downgrade-cost`): 1.2.0 and the builds
/// before it meet `language = "hi"` under a stamp they have never written, read the file and leave
/// it alone ([`ReadOutcome::FromNewerSchema`], [`SavePolicy::Forbidden`]) instead of refusing the
/// word and moving the whole configuration to `.bad`.
fn step_8_to_9(config: &mut Config) {
    config.schema_version = 9;
}

/// Raises a file from schema 9 to schema 10 — **stage E109**, task T-109-1: the twenty-two
/// languages of group B in the closed set of `general.language`.
///
/// **A bare stamp** for the reason of [`step_8_to_9`]: a schema 9 file names one of the forty-four,
/// and they are all still here. 1.3.0 and the builds before it meet `language = "nb"` under a stamp
/// they have never written and leave the file alone instead of moving it to `.bad`.
fn step_9_to_10(config: &mut Config) {
    config.schema_version = 10;
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

/// The name of **the last good feed kept on disk** — task T-119-9, решение 208.98: the signed
/// document exactly as the site gave it, beside the configuration.
pub const KEPT_FEED_FILE_NAME: &str = "news.toml";

/// Where the last good feed is kept inside an arbitrary application data directory —
/// `%APPDATA%\Lang_Switcher\news.toml` for the product ([`kept_feed_path`]).
pub fn kept_feed_path_in(app_data: &Path) -> PathBuf {
    app_data.join(CONFIG_DIR_NAME).join(KEPT_FEED_FILE_NAME)
}

/// The standard place of the kept feed — [`kept_feed_path_in`] over `%APPDATA%`, `None` without it.
pub fn kept_feed_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|app_data| kept_feed_path_in(Path::new(&app_data)))
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
    read_from_via(path, |path| fs::read(path))
}

/// [`read_from`] with the reading of the bytes handed in — task T-55-6, решение 120.1.
///
/// # A file that could not be read is not a damaged file
///
/// Until that task every failure to read went the way of a file that could not be parsed: the
/// session got [`SavePolicy::QuarantineFirst`], and the file was moved to `config.toml.bad` and
/// replaced by defaults at the first save. A file held open by another program for a moment, or
/// closed to this account, is most likely whole and fine — so a read that fails is **tried once
/// more, at once, inside the same attempt** (not a loop, and not «in N seconds»), and a read that
/// fails twice is answered as [`ConfigError::Io`], which [`SavePolicy::for_read`] turns into
/// [`SavePolicy::NotRead`]: nothing written and nothing moved, this session. A file that is gone by
/// the second attempt was there a moment ago — somebody is replacing it — and is answered the same
/// way rather than as a first run.
///
/// # Bytes that are not UTF-8 are damage
///
/// A file an editor saved as UTF-16, or as ANSI with a letter outside ASCII, was read whole: it is
/// the content this build cannot take, and the verdict is [`ConfigError::Malformed`] with no
/// position — the quarantine such a file always went to. Calling it «not read» would leave the
/// program unable to save, for good, until somebody found the encoding.
///
/// # Why the reading is an argument
///
/// The retry is the point, and a test cannot make the file system fail exactly once on cue:
/// `tests\settings.rs` hands in a closure that fails and counts, and [`read_from`] hands in
/// `fs::read`.
pub fn read_from_via(
    path: &Path,
    mut read: impl FnMut(&Path) -> io::Result<Vec<u8>>,
) -> Result<(Config, ReadOutcome), ConfigError> {
    let bytes = match read(path) {
        Ok(bytes) => bytes,
        // ⭐ **Вопрос 95, task Т-29-3 — the one road on which the system is consulted.** There
        // is no file, so there is nothing of anybody's to preserve, and this is a first run:
        // the language of the configuration becomes the language of the user's Windows if this
        // build has it, and `en` if it has not.
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            let mut config = Config::for_a_first_run();

            // ⭐ Stage E116, task T-116-12 (решение 208.59): the mark of the first run is written
            // here and nowhere else — this run creates the file. Every other road of
            // `for_a_first_run` (a damaged file, a file of a newer schema, no `%APPDATA%`) has a
            // file or no place for one, and is not a first run.
            config.letters.first_mark = Some(letters::FirstMark::Due);

            return Ok((config, ReadOutcome::NoFile));
        }
        // Решение 120.1: once more, at once — and whatever the second attempt answers is the
        // answer.
        Err(_) => read(path).map_err(ConfigError::Io)?,
    };

    // Read whole, and not text: damage, not a failed read.
    let Ok(text) = String::from_utf8(bytes) else {
        return Err(ConfigError::Malformed { at: None });
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
    read_or_default_via(path, &system_language_for_a_first_run)
}

/// [`read_or_default`] with the language of a first run handed in — task T-55-9, finding С20.
///
/// A file that could not be read or parsed leaves the program on defaults, and since that task
/// their language is the system's ([`Config::for_a_first_run_in`]) rather than the hard `ru` of
/// section 7: the program has no usable file there, which is exactly the case вопрос 95 answered
/// for a file that is missing. No byte of the file is touched — a damaged file is still kept
/// aside before the first write, a file that could not be read is still left alone — and a file
/// that parses keeps the language it names. The argument is lazy and asked only on the failed
/// road; [`read_or_default`] hands in [`system_language_for_a_first_run`], `tests\settings.rs` a
/// staged language.
pub fn read_or_default_via(
    path: &Path,
    system_language: &dyn Fn() -> Language,
) -> (Config, Result<ReadOutcome, ConfigError>) {
    match read_from(path) {
        Ok((config, outcome)) => (config, Ok(outcome)),
        Err(err) => (Config::for_a_first_run_in(system_language()), Err(err)),
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
    /// The bytes on disk are somebody's text that this build read whole and could not parse —
    /// since task T-55-6 only that; a file that could not be read at all is
    /// [`SavePolicy::NotRead`]. They are moved to [`quarantine_path_for`] before the first write,
    /// and if they cannot be moved, **nothing is written at all**.
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
    /// The file is there and could not be read at all — held open by another program, closed to
    /// this account, a failing disk — **решение 120.1**, task T-55-6.
    ///
    /// Nothing is written and nothing is moved this session. A failure to read says nothing about
    /// what is in the file: most likely it is the person's whole configuration, whole and fine,
    /// and the next start will read it. Until that task this was [`SavePolicy::QuarantineFirst`] —
    /// the file moved to `config.toml.bad` and replaced by defaults at the first save — which turned
    /// a moment of contention into lost settings. The read has already been tried twice by then
    /// ([`read_from`]); the session runs on the defaults, and the file stays exactly where it is.
    ///
    /// **Not [`SavePolicy::Forbidden`]**, although it forbids the same writes: the cause is
    /// different — the file is not from the future, it simply could not be read just now — and
    /// the journal says which of the two it was.
    NotRead,
}

impl SavePolicy {
    /// The decision the second half of a [`read_or_default`] pair calls for.
    ///
    /// **A file that could not be parsed and a file that could not be read are two causes with
    /// two fates — решение 120.1, task T-55-6.** [`ConfigError::Malformed`] is text this build
    /// read whole and did not understand: [`SavePolicy::QuarantineFirst`], the bytes kept aside
    /// before the first write. [`ConfigError::Io`] is a file that is there and could not be read,
    /// twice: [`SavePolicy::NotRead`], nothing written and nothing moved this session.
    ///
    /// ⚠ **This reverses what task T-13-6 decided and section 10 made a rule of.** Every failed
    /// read used to map to `QuarantineFirst`, on the reasoning that a file that could not be
    /// opened is just as much text this build has not seen. That is true, and it is why neither
    /// arm here overwrites the file; but moving the file aside was the wrong half of the answer —
    /// a file held open for a moment is most likely whole and fine, and the quarantine turned that
    /// moment into lost settings. There is still no failure of a read after which overwriting the
    /// file is known to be safe, and still no arm here that says it is.
    pub fn for_read(outcome: &Result<ReadOutcome, ConfigError>) -> Self {
        match outcome {
            Ok(ReadOutcome::NoFile | ReadOutcome::Current | ReadOutcome::Migrated { .. }) => {
                Self::Allowed
            }
            Ok(ReadOutcome::FromNewerSchema { .. }) => Self::Forbidden,
            Err(ConfigError::Io(_)) => Self::NotRead,
            Err(ConfigError::Malformed { .. } | ConfigError::Serialize) => Self::QuarantineFirst,
        }
    }

    /// Whether `HKCU\…\Run` may be made to follow a configuration at `moment` — **the one rule of
    /// решение 120.4 (ж) and task T-55-2**.
    ///
    /// **The rule in one sentence: the `Run` key follows only a configuration a person stands
    /// behind** — at the start, the one read whole out of their file or made by a first run
    /// ([`SavePolicy::Allowed`]); on request, the one they have just chosen, as long as their file
    /// may be written at all (neither [`SavePolicy::Forbidden`] nor [`SavePolicy::NotRead`]).
    ///
    /// The two moments differ on exactly one policy, and on purpose. Under
    /// [`SavePolicy::QuarantineFirst`] the configuration at the start is the defaults standing in
    /// for text this build could not parse: nobody chose them, and the person's own autostart is
    /// somewhere in the bytes that were not understood, so the registry is left alone. The same
    /// policy on request is a person choosing in the dialog, and their choice is what the file will
    /// carry once the damaged bytes are moved aside — so the registry follows. Under
    /// [`SavePolicy::Forbidden`] the file is from a newer build and is not written this session,
    /// and under [`SavePolicy::NotRead`] (task T-55-6) it could not even be read: a registry made
    /// to follow anything then would disagree with that file for good.
    ///
    /// ⚠ A session with no `%APPDATA%` has read nothing and is not a policy; its caller,
    /// [`crate::tray`], answers that case where it asks.
    pub fn lets_the_run_key_follow(self, moment: RunKeyMoment) -> bool {
        match self {
            Self::Allowed => true,
            Self::QuarantineFirst => moment == RunKeyMoment::Request,
            Self::Forbidden | Self::NotRead => false,
        }
    }
}

/// The two moments at which `HKCU\…\Run` could be made to follow a configuration — решение 120.4
/// (ж), tasks T-55-1 and T-55-2. What each may do is [`SavePolicy::lets_the_run_key_follow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKeyMoment {
    /// The start of the program: the configuration is whatever the one read produced.
    Start,
    /// A person asked — «Применить» of FR-92 or the check mark of FR-91 — and the configuration is
    /// what they have just chosen.
    Request,
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
    crate::sandbox::guard_path(path);
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
/// destination on Windows — **unless the destination carries the read-only attribute**, which
/// [`replace_read_only_once`] answers (task T-55-3). A crash, a power cut or a killed process
/// leaves either the old file or the new one, never half of either — the configuration is
/// written by the UI thread while the program runs, and a truncated file would silently reset
/// every setting the user has.
///
/// The temporary file is removed if anything after its creation fails, so a failed write
/// leaves no litter next to the configuration.
pub fn write_to(path: &Path, config: &Config) -> Result<WriteOutcome, ConfigError> {
    let text = config.to_toml_string()?;

    write_bytes_to(path, text.as_bytes())
}

/// **The body of [`write_to`] for any bytes** — task T-119-9: the last good feed is kept beside the
/// configuration ([`kept_feed_path_in`]) the same way, temporary file, flush, one rename, so that a
/// killed process leaves the old copy or the new one and never half of either.
pub fn write_bytes_to(path: &Path, bytes: &[u8]) -> Result<WriteOutcome, ConfigError> {
    crate::sandbox::guard_path(path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let temporary = temporary_path_for(path);
    if let Err(err) = write_and_sync(&temporary, bytes) {
        let _ = fs::remove_file(&temporary);
        return Err(err);
    }
    match replace_read_only_once(&temporary, path) {
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            let _ = fs::remove_file(&temporary);
            Err(ConfigError::Io(err))
        }
    }
}

/// What [`write_to`] did — task T-55-3.
///
/// The ordinary answer is [`WriteOutcome::Written`]. The other one is a write that landed only
/// after the read-only attribute of the target was cleared, and the caller — the tray, the one
/// writer of this file — names it in the journal: a person may have set that attribute by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    /// The file was replaced by the one rename.
    Written,
    /// The rename was refused for the read-only attribute; the attribute was cleared, and the
    /// second rename replaced the file. The attribute stays cleared — the doc of [`write_to`].
    WrittenAfterClearingReadOnly,
}

/// Puts `temporary` over `path` — the one rename of [`write_to`], and **one** retry for a target
/// that carries the read-only attribute — task T-55-3, finding Н26.
///
/// # The finding
///
/// `fs::rename` over a file with `FILE_ATTRIBUTE_READONLY` answers `ERROR_ACCESS_DENIED`, and it
/// answers it every time (premise П5 of the stage, measured: `raw_os_error = 5`, the temporary
/// left behind). The refusal reached only the journal: «Применить» closed the dialog as if nothing
/// had happened, and the change was gone at the next start. This module had no word for the
/// attribute at all.
///
/// # What is done, and what is not
///
/// * **Only after the refusal, and only for that refusal.** The attribute is not looked at before a
///   rename fails, so the ordinary write is the one it always was. A refusal of another kind — or
///   `PermissionDenied` on a file that is *not* read-only, which then has another cause, a handle
///   held open or the permissions of the folder — is handed back as it came.
/// * **Once, not in a loop.** The attribute is cleared, the rename is tried a second time, and
///   whatever the second attempt answers is the answer. Nothing waits and nothing repeats.
/// * **The attribute is left cleared**, whatever the second attempt answered. Putting it back after
///   a write that landed would make the next save fail in exactly the same way, and the one after
///   that — the very defect this repairs; putting it back after a write that did not land would buy
///   nothing, because the write is about to be reported as failed anyway. A person who set the
///   attribute to keep the program away is told: a write that landed is named in the journal by
///   the caller (`configuration read-only attribute cleared`), and one that did not is reported as
///   the failed write it is.
///
/// Atomicity is untouched: the retry is the same single rename of the same finished temporary.
fn replace_read_only_once(temporary: &Path, path: &Path) -> io::Result<WriteOutcome> {
    let refusal = match fs::rename(temporary, path) {
        Ok(()) => return Ok(WriteOutcome::Written),
        Err(refusal) => refusal,
    };

    let Ok(metadata) = fs::metadata(path) else {
        return Err(refusal);
    };
    let mut permissions = metadata.permissions();

    if refusal.kind() != io::ErrorKind::PermissionDenied || !permissions.readonly() {
        return Err(refusal);
    }

    // Windows: this clears `FILE_ATTRIBUTE_READONLY` and touches no mode bits — the Unix concern of
    // the lint does not exist on the one platform this program is built for.
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);

    if fs::set_permissions(path, permissions).is_err() {
        return Err(refusal);
    }

    fs::rename(temporary, path).map(|()| WriteOutcome::WrittenAfterClearingReadOnly)
}

/// Names the temporary file used by [`write_to`], beside the target.
///
/// Beside it and not in `%TEMP%`, because a rename is only atomic within one volume. The
/// process id keeps two instances of the program from colliding; within one process the
/// configuration is written by the UI thread alone (section 6.3), so nothing finer is
/// needed.
///
/// Public since task T-55-5, so that a test can take the name a write leaves from here and hand it
/// to [`remove_abandoned_temporaries`] — the mask of the sweep is never written out a second time.
pub fn temporary_path_for(path: &Path) -> PathBuf {
    let mut name = match path.file_name() {
        Some(name) => name.to_os_string(),
        None => OsString::from(CONFIG_FILE_NAME),
    };
    name.push(format!(".{}{TEMPORARY_SUFFIX}", std::process::id()));
    path.with_file_name(name)
}

/// What [`temporary_path_for`] puts after the process id, and what [`remove_abandoned_temporaries`]
/// looks for — task T-55-5. One constant for both, so that the name a write leaves and the name a
/// start removes cannot drift apart.
const TEMPORARY_SUFFIX: &str = ".tmp";

/// What [`remove_abandoned_temporaries`] did — task T-55-5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Swept {
    /// Temporaries of an interrupted write that were removed.
    pub removed: usize,
    /// Temporaries of an interrupted write that were found and would not go.
    pub refused: usize,
}

/// Removes the temporaries an interrupted [`write_to`] left beside `path` — task T-55-5, finding
/// Т8.
///
/// A process stopped between the write of its temporary and the rename — killed, or cut off by a
/// power failure — leaves `config.toml.<pid>.tmp` behind, and until this task nothing ever removed
/// it: no start looked, and the uninstaller had no `[UninstallDelete]` for it. Litter like that is
/// at odds with section 8.4, which promises an uninstall that leaves nothing of the program behind.
///
/// **What counts** is a file in the folder of `path` named exactly the way [`temporary_path_for`]
/// names one: the name of `path`, a dot, decimal digits and [`TEMPORARY_SUFFIX`]. The kept copy of
/// an unreadable file (`config.toml.bad`, [`QUARANTINE_SUFFIX`]) is not one and is precious, and no
/// other file of the folder is looked at — the journal keeps temporaries of its own there.
///
/// **When:** once, at the start, where the configuration is read — before this process has written
/// anything, so none of the temporaries can be its own. Not on a timer, and not by asking whether
/// the process named in a file is still alive: FR-82 keeps one instance per session, and a name
/// alone cannot prove who is still writing.
///
/// A refusal is not a failure of the program's work — the configuration is read all the same — and
/// it is not silence either: it is counted in [`Swept::refused`], and the caller names it in the
/// journal.
pub fn remove_abandoned_temporaries(path: &Path) -> Swept {
    let mut swept = Swept::default();

    let (Some(folder), Some(name)) = (
        path.parent(),
        path.file_name().and_then(|name| name.to_str()),
    ) else {
        return swept;
    };

    // No folder yet — a first run — has nothing in it to sweep.
    let Ok(entries) = fs::read_dir(folder) else {
        return swept;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();

        let abandoned = file_name
            .to_str()
            .and_then(|candidate| candidate.strip_prefix(name))
            .and_then(|rest| rest.strip_prefix('.'))
            .and_then(|rest| rest.strip_suffix(TEMPORARY_SUFFIX))
            .is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
            });

        if !abandoned || !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }

        match fs::remove_file(entry.path()) {
            Ok(()) => swept.removed += 1,
            Err(_) => swept.refused += 1,
        }
    }

    swept
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

thread_local! {
    /// A key of a test's own in place of [`RUN_KEY_PATH`] on this thread
    /// ([`use_run_key_on_this_thread`]): its name and the same as UTF-16 with the terminator.
    static RUN_KEY_OF_TEST: std::cell::RefCell<Option<(String, Vec<u16>)>> =
        const { std::cell::RefCell::new(None) };
}

/// **For tests**: the key under `HKEY_CURRENT_USER` this thread writes and reads in place of the real
/// `Run` — a key of the test's own, made and taken away by the test; `None` gives the real one back.
/// The program never calls it: a test that writes autostart without it is stopped before the write
/// (`sandbox::guard_run_key`, the night of 2026-11-20).
#[doc(hidden)]
pub fn use_run_key_on_this_thread(subkey: Option<&str>) {
    RUN_KEY_OF_TEST.with(|cell| {
        *cell.borrow_mut() = subkey.map(|subkey| (subkey.to_owned(), wide(subkey)));
    });
}

impl RunKey {
    /// Opens [`RUN_KEY_PATH`] under `HKEY_CURRENT_USER` with the access asked for.
    ///
    /// Opened and never created. The key exists in every user profile Windows makes, so a
    /// failure here means something is wrong with the profile rather than that the key is
    /// merely absent — and creating registry keys is not what FR-93 asks for.
    fn open(access: REG_SAM_FLAGS) -> windows::core::Result<Self> {
        let mut key = HKEY::default();
        let own = RUN_KEY_OF_TEST.with(|cell| cell.borrow().clone());

        // The guard: a test does not write the real key — stopped here, before the key is even
        // opened for writing. A key of the test's own passes, unless the test of the guard treats it
        // as the real one.
        let real = own
            .as_ref()
            .is_none_or(|(name, _)| crate::sandbox::pretended_real_run_key(name));
        if real && access.0 & KEY_SET_VALUE.0 != 0 {
            crate::sandbox::guard_run_key();
        }
        let path = own
            .as_ref()
            .map_or(RUN_KEY_PATH, |(_, units)| PCWSTR(units.as_ptr()));

        // SAFETY: `RUN_KEY_PATH` is a NUL-terminated `'static` UTF-16 literal, so the pointer
        // the call reads through outlives the call. `key` is a live local the call writes
        // exactly one handle into, and it is written only on success — which is checked on the
        // next line, so a failed open cannot leave a bogus handle to be closed. The handle
        // becomes the property of the returned value and is closed exactly once, in `Drop`.
        let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path, None, access, &mut key) };

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

/// Whether the image at `exe` may put itself into [`RUN_KEY_PATH`] — **решение 120.4 (е)**, task
/// T-55-1.
///
/// ⛔⛔ **The guard against litter, and the first thing task T-55-1 put in.** From that task on the
/// start of the program makes the registry agree with `general.autostart` (решение 120.4 (б)), and
/// `general.autostart` defaults to `true`. Without this guard every debug build somebody runs,
/// every test binary that attaches a tray and every bench on a development machine would write its
/// own path into `HKCU\…\Run`. That is not a guess: a debug instance started with autostart on did
/// exactly that in this project, and nothing in the session that ran it could take the value out
/// again.
///
/// **Two conditions, and both are required:**
///
/// 1. **not a debug build** — `debug_build` is the caller's `cfg!(debug_assertions)`;
/// 2. **the image is `%ProgramFiles%\Lang_Switcher\LangSwitcher.exe`** — the one place section 8.4
///    installs the program. The installer fixes it (`DefaultDirName={commonpf64}\Lang_Switcher`,
///    `DisableDirPage=yes`), and section 8.2 makes it the only place a `uiAccess` image starts at
///    all.
///
/// The path alone keeps out every test binary (`<target>\debug\deps`), every bench and every
/// release build in a development folder; the profile is a second and independent reason, for a
/// debug image copied into the folder by hand. **The `testing` feature is deliberately not a
/// condition:** the plain battery runs without it, so it would guard half the runs, and the path
/// guards all of them.
///
/// Compared component by component and without regard to case — the file system does not care, and
/// the environment is free to spell `Program Files` either way. Pure, and public so that both
/// answers can be checked without the image having to be anywhere in particular.
pub fn autostart_may_register(debug_build: bool, exe: &Path, program_files: &Path) -> bool {
    if debug_build {
        return false;
    }

    let installed = program_files.join(CONFIG_DIR_NAME).join(crate::EXE_NAME);

    let folded = |path: &Path| -> Vec<String> {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
            .collect()
    };

    folded(exe) == folded(&installed)
}

/// [`autostart_may_register`] asked about the running image — what the start of the program
/// consults before it asks the registry anything at all.
///
/// `%ProgramFiles%` comes from the environment for the reason [`default_config_path`] gives for
/// `%APPDATA%`: the value is already in the process. An image that cannot say where it is, or an
/// environment without `ProgramFiles`, answers `false` — a build that cannot tell whether it is the
/// installed one does not register itself.
pub fn this_build_may_register_autostart() -> bool {
    let (Ok(exe), Some(program_files)) =
        (std::env::current_exe(), std::env::var_os("ProgramFiles"))
    else {
        return false;
    };

    autostart_may_register(cfg!(debug_assertions), &exe, Path::new(&program_files))
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
/// «Нашли ошибку в переводе?» — the quiet button under the language, task T-120-22 (решение
/// 208.169, С12); it opens the wizard on the page of the idea. Block 3280…3295 (`app.rc`).
pub const IDS_TRANSLATION_REPORT: u16 = 3289;
// ⚠ **3004 is retired, not free** — task Т-31-3, решение 99.4. It was `IDS_LANGUAGE_RESTART`,
// «вступит в силу после перезапуска». Решения 83 и 83а.2 chose that sentence and вопрос 99 took
// its subject away: the language now takes effect at once (task Т-31-1). The number stays a hole
// for the reason 3017..3021, 3023, 3024 and 3056 are holes — a string identifier that moved would
// change what an **installed** build shows — and it costs nothing: [`string_from`] walks a block
// of sixteen by index, an absent row comes back empty, and the sweeps of completeness walk
// [`INTERFACE_STRINGS`], which no longer names it.
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
// ⚠ **3017..3021, 3023, 3024 and 3056 are retired, not free** — task Т-23-2, решения 81 и
// 82. They were the captions of «Замена» and «Выделение» and of everything those two groups
// held. The rows are gone from both tables of `app.rc` and the holes stay: a string
// identifier that moved would change what an **installed** build shows.
/// The tick of FR-65 — a row of «Общие» since task Т-23-2, решение 82.2.
pub const IDS_SELECTION_ENABLED: u16 = 3022;
/// «Исключения» — the fourth group of FR-92 since task Т-23-2.
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
/// What the **field** says while a capture is armed — «Нажмите клавишу или сочетание…» since
/// task T-93-2 (вопрос 157, the mock-up of 157.5), «Нажмите клавишу…» before it.
///
/// It stood in the note under the field until task Т-23-5 and read «Нажмите клавишу. Esc —
/// отмена.» Решение 82.6 moved the invitation into the field itself, where the key name would
/// otherwise sit looking editable, and split the «Esc — отмена» half off into
/// [`IDS_CAPTURE_HINT`].
pub const IDS_CAPTURE_PROMPT: u16 = 3039;
/// [`Refusal::Modifier`] — «Ctrl и Alt — только вместе с клавишей; дважды — только Shift.» since
/// task T-95-2 (the moment 6 of the mock-up of 157.18): a `Shift` on its own is no refusal any more,
/// and the line names the way left to the other two. «Модификатор сам по себе горячей клавишей быть
/// не может.» before it.
pub const IDS_CAPTURE_MODIFIER: u16 = 3040;
// ⚠ 3041 is a hole since task T-93-2: `IDS_CAPTURE_COMBINATION` («Сочетания с модификаторами не
// поддерживаются…») left with `Refusal::Combination` and all fourteen of its translations, when
// вопрос 157 made a combination a hotkey. Identifiers are never renumbered — see [`INTERFACE_STRINGS`].
/// [`Refusal::Emergency`].
pub const IDS_CAPTURE_EMERGENCY: u16 = 3042;
/// [`Refusal::Nameless`].
pub const IDS_CAPTURE_NAMELESS: u16 = 3043;
/// [`Refusal::Reserved`].
pub const IDS_CAPTURE_RESERVED: u16 = 3044;
// 3045…3047 — IDS_STATE_HOOK, IDS_HOOK_UP, IDS_HOOK_DOWN — retired by task T-34-5 and left as
// holes: the first line of «Состояние» is IDS_STATE_WORKING…IDS_STATE_JOINED now.
/// The second line of the «Состояние» group.
pub const IDS_STATE_LAYOUTS: u16 = 3048;
/// The third line of the «Состояние» group.
pub const IDS_STATE_AUTOSTART: u16 = 3049;
/// The registry value of FR-93 is there.
pub const IDS_AUTOSTART_PRESENT: u16 = 3050;
/// It is not.
pub const IDS_AUTOSTART_ABSENT: u16 = 3051;
/// «Работает» — the first line of «Состояние» for a person, task T-120-21 (решение 208.169, С10).
/// The lines above (`IDS_STATE_*`) stay unchanged under [`IDS_STATUS_DETAILS`]. Block 3280…3295.
pub const IDS_STATUS_WORKING: u16 = 3280;
/// «Приостановлена» — the word of the menu «Приостановить» and of the tooltip.
pub const IDS_STATUS_SUSPENDED: u16 = 3281;
/// «Нужен перезапуск» — as the menu says it, «Возобновить — нужен перезапуск».
pub const IDS_STATUS_RESTART: u16 = 3282;
/// «Исправляет: {0}» — {0} the layouts, source → target, with the arrow of the cycle.
pub const IDS_STATUS_FIXES: u16 = 3283;
/// «Раскладки не выбраны» — the second line when there is no pair.
pub const IDS_STATUS_FIXES_NONE: u16 = 3284;
/// «Клавиша: {0}» — the hotkey, named as in the settings.
pub const IDS_STATUS_HOTKEY: u16 = 3285;
/// «Запуск при входе в систему: включён».
pub const IDS_STATUS_AUTOSTART_ON: u16 = 3286;
/// «Запуск при входе в систему: выключен».
pub const IDS_STATUS_AUTOSTART_OFF: u16 = 3287;
/// «Подробности для автора» — the quiet button that opens the lines of the developer.
pub const IDS_STATUS_DETAILS: u16 = 3288;
/// «Копировать» — the window of «Подробности для автора» puts its lines on the clipboard (T-120-21,
/// C6 (а)); 3295, the reserve the controller named.
pub const IDS_STATUS_COPY: u16 = 3295;
/// «Запуск, звук, флаг в трее, язык и вид программы» — the subtitle of «Общие» (T-120-20, C3 of the controller).
pub const IDS_SECTION_GENERAL_SUB: u16 = 3307;
/// «Одна клавиша исправляет набранный текст и переключает раскладку» — the subtitle of «Горячая клавиша» (T-120-20, C3 of the controller).
pub const IDS_SECTION_HOTKEY_SUB: u16 = 3308;
/// «Между какими раскладками переводить текст» — the subtitle of «Раскладки» (T-120-20, C3 of the controller).
pub const IDS_SECTION_LAYOUTS_SUB: u16 = 3309;
/// «Программы, в которых Lang Switcher не вмешивается» — the subtitle of «Исключения» (T-120-20, C3 of the controller).
pub const IDS_SECTION_EXCLUSIONS_SUB: u16 = 3310;
/// «Журнал помогает автору разобраться, если что-то пошло не так» — the subtitle of «Диагностика» (T-120-20, C3 of the controller).
pub const IDS_SECTION_DIAGNOSTICS_SUB: u16 = 3311;
/// «Настройки» — the eyebrow over the title of the window of the settings (T-120-20, C3 of the controller).
pub const IDS_SETTINGS_EYEBROW: u16 = 3312;
/// The note about a layout named in the file that this session does not have.
pub const IDS_LAYOUT_NOTE: u16 = 3052;
/// The word «источник» inside that note.
pub const IDS_LAYOUT_WORD_SOURCE: u16 = 3053;
/// The word «цель» inside that note.
pub const IDS_LAYOUT_WORD_TARGET: u16 = 3054;
/// What the journal folder line says when `%APPDATA%` is not set.
pub const IDS_LOG_DIR_MISSING: u16 = 3055;
// 3056 was `IDS_METHOD_AUTO`, the automatic method of FR-42а — retired by task Т-23-2 with
// its group; see the note at 3017 above. It opened the sixteen-string block the four rows
// below stand in, and they go on opening it, so nothing here moves.
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
/// Caption of the «Как пользоваться» panel — FR-92а, task Т-23-4, решение 82.5.
///
/// The six rows 3066–3071 are the whole of what was left of the sixteen-string block 3056
/// opened, and they take it: a string put after the menu block would open a block of its own
/// for six strings and leave ten empty rows behind it.
pub const IDS_ABOUT_HELP: u16 = 3066;
/// First row of the help: the word typed in the wrong layout. Carries a `{0}` — **the name of
/// the hotkey**, substituted by [`format_text`] out of `[hotkey] key` of the running
/// configuration ([`effective_hotkey_name`]), so the help names the key this user's program
/// actually answers to and not a literal written down twice.
pub const IDS_ABOUT_HELP_1: u16 = 3067;
/// Second row: the second press puts the original text back. Carries the same `{0}`.
pub const IDS_ABOUT_HELP_2: u16 = 3068;
/// Third row: the selection of FR-61. Carries the same `{0}`.
pub const IDS_ABOUT_HELP_3: u16 = 3069;
/// Fourth row: what the tray icon holds — FR-91. No `{0}`: no key is named.
pub const IDS_ABOUT_HELP_4: u16 = 3070;
/// Fifth row: where the key is changed. No `{0}` either — it points at the field, not at a key.
pub const IDS_ABOUT_HELP_5: u16 = 3071;
/// Sixth row: a press over nothing typed and nothing selected switches the layout — stage E114,
/// task T-114-2, решение 208.5 (6а). `{0}` is the name of the hotkey, as in the first three.
///
/// ⚠ Not 3072: that is the menu's, and the block 3056 the help opened is full. 3263 is the last
/// free number of the block 3248..3263 the wizard opened (`app.rc`).
pub const IDS_ABOUT_HELP_6: u16 = 3263;
/// «Флаг раскладки в трее» — the check box of FR-90б in «Общие», task T-119-2 (решения 208.88,
/// 208.89). The first of the block 3264…3279.
pub const IDS_TRAY_FLAG: u16 = 3264;
/// The badge «новое» after the label of a setting that is new — «Флаг раскладки в трее» of the
/// window of the settings (passport §6, T-120-20).
pub const IDS_NEW_BADGE: u16 = 3304;
/// «Поддержать» — the button of the card «Поддержать автора» of «От автора» (T-120-17, слово
/// владельца 208.178). Its own key: «Спасибо» keeps [`IDS_SUPPORT_OPEN`] (T-79-1).
pub const IDS_AUTHOR_SUPPORT: u16 = 3305;
/// «Канал» — the second button of that card. Its own key: the wizard keeps [`IDS_CHANNEL_OPEN`]
/// (FR-104).
pub const IDS_AUTHOR_CHANNEL: u16 = 3306;
/// «Раскладка» — the title of the list of the program's own a click on the flag shows when РУС/ENG
/// is hidden at the clock, task T-119-3, part Б (решение 208.88).
pub const IDS_OWN_LIST_TITLE: u16 = 3265;
/// «Дополнительные параметры клавиатуры» — the link at the bottom of that list.
pub const IDS_OWN_LIST_MORE: u16 = 3266;
/// «обновление» — what the window in the corner is about after an update (its header), T-119-6.
pub const IDS_CORNER_UPDATE: u16 = 3267;
/// «Lang Switcher обновлён до {0}» — the title of the window «Обновлено», T-119-6.
pub const IDS_RELEASE_TITLE: u16 = 3268;
/// «Было {0}. Новое: …» — the short text of the last important release (a permanent name), T-119-6.
pub const IDS_RELEASE_TEXT: u16 = 3269;
/// «Все изменения на сайте» — the link of the window «Обновлено», T-119-6.
pub const IDS_RELEASE_CHANGES: u16 = 3270;
/// «Как пользоваться» — the second button of the window «Обновлено», T-119-6.
pub const IDS_RELEASE_HOWTO: u16 = 3271;
/// «Понятно» — the button that closes a window in the corner, T-119-5 and T-119-6.
pub const IDS_CORNER_OK: u16 = 3272;
/// «Больше не показывать окна обновлений» — the check box of the window «Обновлено», T-119-6.
pub const IDS_RELEASE_NEVER: u16 = 3273;
/// «флаг раскладки» — what the hint of the flag is about (its header), T-119-5.
pub const IDS_CORNER_FLAG: u16 = 3274;
/// «письмо» — what the window in the corner that knocks with a letter is about (its header), T-120-16.
/// Block 3293…3295 of stage E120.
pub const IDS_CORNER_LETTER: u16 = 3293;
/// «Флаг раскладки включён» — the title of the hint, T-119-5.
pub const IDS_HINT_TITLE: u16 = 3275;
/// «Чтобы флаг всегда был виден на панели задач, …» — the text of the hint, T-119-5.
pub const IDS_HINT_TEXT: u16 = 3276;
/// «Открыть Параметры Windows» — the main button of the hint, T-119-5.
pub const IDS_HINT_OPEN: u16 = 3277;
/// «Что нового в {0}…» — the entry of the menu a week after an update, T-119-7.
pub const IDS_MENU_WHATSNEW: u16 = 3278;
/// «Сообщать об обновлениях программы» — the switch of «От автора», T-119-10. The last of the block
/// 3264…3279.
pub const IDS_UPDATES_SWITCH: u16 = 3279;
/// «Новости автора выключены в файле настроек» — the line under the switch of «От автора» while
/// `[letters] feed = false`, task T-119-10. The first of 3296… (3293…3295 are stage E120's).
pub const IDS_NEWS_OFF_IN_FILE: u16 = 3296;
// ⭐ Task T-119-9 (решения 208.95–208.99): the fallback letter «Обновление» says it honestly —
// 3297…3302. It opens only from the menu entry «Доступна версия X…» or from «Последние письма», and
// both come on the seventh day of an update the updater could not put in.
/// «Обновиться автоматически не получилось» — the heading of «Обновление».
pub const IDS_FALLBACK_TITLE: u16 = 3297;
/// «доступна {0} · у вас {1} · выпуск {2}» — the quiet line under it: the version the feed names,
/// the one installed here, the day of the release.
pub const IDS_FALLBACK_SUB: u16 = 3298;
/// «Версия {0} вышла {1}, но программа не смогла поставить её сама…» — the first paragraph: a date
/// and not a count of days (решение 208.99 п. 3).
pub const IDS_FALLBACK_NOTICE: u16 = 3299;
/// «Что нового в {0}» — the label over the author's own words about the release.
pub const IDS_FALLBACK_WHATS_NEW: u16 = 3300;
/// «Если так повторяется, напишите автору — это поможет найти причину.» — the last row of the panel,
/// over the button «Написать автору…» (решение 208.97 п. 2).
pub const IDS_FALLBACK_REPEATS: u16 = 3301;
/// «доступна {0} · у вас {1}» — the line under the heading of the row of «Последние письма».
pub const IDS_ENTRY_FALLBACK_MARK: u16 = 3302;
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
/// The greyed «Возобновить» when processing is off after the panics of FR-99 — task T-34-5,
/// finding Н84: the label is the one carrier of the reason, the tray has no tooltips.
pub const IDS_MENU_RESUME_RESTART: u16 = 3221;
/// The caption of the sound switch of FR-100 — task Т-21-5.
pub const IDS_SOUND: u16 = 3078;
/// The hint under the field while a capture is armed — «Esc или клик мимо — отмена», task
/// Т-23-5, решение 82.6.
///
/// Appended after [`IDS_SOUND`] for the reason that one was appended, and it costs no new
/// block: 3072..3087 is the block the menu opened and 3079 is inside it.
pub const IDS_CAPTURE_HINT: u16 = 3079;
/// The state of the **program** in the tooltip of the tray icon while it is working — FR-90,
/// решение 99.2, task Т-31-4.
///
/// Both words lived as Russian literals in `crate::tray`, past these tables and past every
/// sweep, in all fourteen locales; the fitting stand of Э28 measures windows and the tooltip is
/// not a window. The numbers 3080 and 3081 are appended after the last one in use and cost no
/// new block — 3072..3087 is the one the menu opened.
///
/// ⚠ The gender is the **program**'s, not the keyboard hook's, so these two are allowed to
/// disagree with [`IDS_HOOK_UP`] / [`IDS_HOOK_DOWN`] in the same window: there the subject is
/// «перехват». The choice per language is written down in `scratchpad-Э31\глоссарий.md` §1.
pub const IDS_TIP_ACTIVE: u16 = 3080;
/// The same while FR-91 holds it suspended.
pub const IDS_TIP_PAUSED: u16 = 3081;

// =========================================================================================
// FR-101, FR-102 and FR-103 — the letters from the author. Task Т-32-2, вопрос 101.
// =========================================================================================
//
// Fifty-eight identifiers mirrored from `app.rc` by hand, exactly as every identifier above
// is. A mismatch is not silent: the string comes back empty, `FindResourceExW` is journaled,
// and the test that reads the tables out of the built binary compares them against these.

/// The caption of the letter window — one window serves all five letters (FR-101).
pub const IDS_LETTER_CAPTION: u16 = 3082;
/// «Закрыть» — the button that closes a letter and asks for nothing.
pub const IDS_CLOSE: u16 = 3083;
/// «Открыть канал» — FR-103. Disabled while `letters::links::CHANNEL_URL` is a placeholder.
pub const IDS_CHANNEL_OPEN: u16 = 3084;
/// «Поддержать автора» — FR-103, on the same terms.
///
/// ⚠ **The `OPEN` in the name is history, not the caption** — task T-79-1. Until `e79` the button
/// said «Открыть страницу поддержки», and the user turned that down on the live product: it reads
/// as a *technical* support page, where one asks questions about the program, rather than as
/// supporting the person who made it. Only the fourteen captions moved; the identifier and the
/// number 3085 stayed, because a rename here would be six edits and nothing visible.
pub const IDS_SUPPORT_OPEN: u16 = 3085;
/// «Привет» — the heading.
pub const IDS_HELLO_TITLE: u16 = 3086;
/// «Привет» — the paragraph under the heading.
pub const IDS_HELLO_LEAD: u16 = 3087;
/// «Привет» — the caption under the demonstration; carries the key chip.
pub const IDS_HELLO_DEMO_CAP: u16 = 3088;
/// «Привет» — the caption of the panel.
pub const IDS_HELLO_PANEL: u16 = 3089;
/// «Привет» — the first of the three rows; carries the key chip.
pub const IDS_HELLO_ROW_1: u16 = 3090;
/// «Привет» — the second row; carries the key chip.
pub const IDS_HELLO_ROW_2: u16 = 3091;
/// «Привет» — the third row.
pub const IDS_HELLO_ROW_3: u16 = 3092;
/// «Привет» — the line under the body: pause and settings by a right-click on the tray icon, the
/// flag in the settings (208.106 п. 3, T-120-16). Block 3293…3295 of stage E120.
pub const IDS_HELLO_FOOT: u16 = 3294;
/// «Привет» — the left button, which opens the settings window (FR-92).
pub const IDS_HELLO_SETTINGS: u16 = 3093;
/// «Привет» — the accented button.
pub const IDS_HELLO_OK: u16 = 3094;
/// «Спасибо» — the heading.
pub const IDS_THANKS_TITLE: u16 = 3095;
/// «Спасибо» — the paragraph under the heading.
pub const IDS_THANKS_LEAD: u16 = 3096;
/// «Спасибо» — the second paragraph.
pub const IDS_THANKS_PARA: u16 = 3097;
/// The caption of the «Поддержать автора» panel — FR-103.
pub const IDS_SUPPORT_PANEL: u16 = 3098;
/// The text of that panel, the heart included.
pub const IDS_SUPPORT_TEXT: u16 = 3099;
/// «Напомнить через неделю» — the one snooze of FR-101.
pub const IDS_THANKS_SNOOZE: u16 = 3100;
/// The line under «Спасибо»: it is shown once.
pub const IDS_THANKS_FOOT: u16 = 3101;
// ⛔⛔ **3102…3109 — `IDS_WHATSNEW_*` (восемь строк письма «Что нового») — сняты задачей
// T-119-8, решение 208.94 п. 4.** Письмо было закрыто решением 118.5 и теперь убрано целиком;
// номера не занимать — у старых таблиц переводов они значили другое.
/// The caption of the «От автора» window — FR-103.
pub const IDS_AUTHOR_CAPTION: u16 = 3110;
// ⛔⛔ **3111 — `IDS_AUTHOR_VERSION` — снята задачей T-81-3, решение 142.2 п. 5.** Она несла
// «версия {0} · Панда, он же Panda_Pishet_Kod» под именем программы в «От авторе»; слово
// владельца — «не надо вставлять панда, он же панда». Шапка у двух окон стала одна, и строку
// версии в обоих пишет [`IDS_ABOUT_VERSION`]. Это **первое за проект удаление** строки
// интерфейса: [`INTERFACE_STRINGS`] поехал 218 → 217, и это нормально. Номер не
// переиспользуется — надгробие тут по той же причине, по какой стоит надгробие 3004.
/// The caption of its first panel.
pub const IDS_AUTHOR_PANEL: u16 = 3112;
/// The text of that panel, the heart included.
pub const IDS_AUTHOR_TEXT: u16 = 3113;
/// The caption of the «Новости и обновления» panel — FR-102.
pub const IDS_NEWS_PANEL: u16 = 3114;
/// What the feed is and how often it is read — said out loud from the first day (FR-102).
pub const IDS_NEWS_ABOUT_FEED: u16 = 3115;
/// «Лента ещё не читалась.»
pub const IDS_NEWS_NEVER_READ: u16 = 3116;
/// When the feed was read, that the signature checked out, and when the next reading is.
pub const IDS_NEWS_READ_ON: u16 = 3117;
/// «Установлена версия X, это последняя.»
pub const IDS_NEWS_LATEST: u16 = 3118;
/// «Установлена версия X, доступна Y.»
pub const IDS_NEWS_AVAILABLE: u16 = 3119;
/// «Открыть страницу загрузки» — the link the `update` entry of the feed carries.
pub const IDS_NEWS_DOWNLOAD: u16 = 3120;
/// «Последние письма» — the button that opens the list window.
pub const IDS_NEWS_LETTERS: u16 = 3121;
/// The quiet line the panel carries before the switch of FR-102 appears.
pub const IDS_NEWS_FILE_ONLY: u16 = 3122;
/// The caption of the «Обратная связь» panel.
pub const IDS_FEEDBACK_PANEL: u16 = 3125;
/// The text of that panel.
pub const IDS_FEEDBACK_TEXT: u16 = 3126;
/// «Написать автору…» — the permanent entry of FR-91 and the button of FR-103.
pub const IDS_WRITE_TO_AUTHOR: u16 = 3127;
/// The caption of the «Последние письма» window — FR-101.
pub const IDS_LETTERS_CAPTION: u16 = 3128;
/// The line under it: three news items are kept, the fourth pushes the oldest out.
pub const IDS_LETTERS_FOOT: u16 = 3129;
/// «Открыть письмо» — the button of one entry of that window.
pub const IDS_LETTERS_OPEN: u16 = 3130;
/// «Прочитано» — the only thing that marks a news item read (FR-101).
pub const IDS_NEWS_READ_BUTTON: u16 = 3131;
/// The temporary menu entry of FR-91: there is an unread letter.
pub const IDS_MENU_UNREAD: u16 = 3132;
/// The temporary menu entry of FR-91: a newer version exists.
pub const IDS_MENU_UPDATE: u16 = 3133;
/// The title of the balloon a letter is announced by — `NIF_INFO`, FR-101.
pub const IDS_TOAST_TITLE: u16 = 3134;
/// The balloon's text before a «Новость».
pub const IDS_TOAST_NEWS: u16 = 3135;
/// The balloon's text before «Спасибо».
pub const IDS_TOAST_THANKS: u16 = 3136;
// ⛔⛔ **3137, 3138 — `IDS_TOAST_UPDATE_TITLE`, `IDS_TOAST_UPDATE` (стук письма «Обновление») — сняты
// задачей T-119-9, решение 208.99 п. 1: уведомления Windows о нём нет.** Номера не занимать.

// ⛔⛔ **3212 — `IDS_TOAST_WHATSNEW` (стук письма «Что нового») — снята задачей T-119-8 вместе с
// письмом, решение 208.94 п. 4.** Номер не занимать.
/// «Сохранить журнал» — the button of «Диагностика» that writes the dump of the running
/// session now; task T-34-3, finding С48. No ellipsis since task T-39-11 (решение 122.5): the
/// button opens no dialog.
pub const IDS_LOG_SAVE: u16 = 3213;
/// The one sentence the person sees when that write is refused — task T-34-3.
pub const IDS_LOG_SAVE_FAILED: u16 = 3214;
/// The note at the top of a section of the window of the settings when the file of the
/// configuration refused a change and the change was stepped back — T-120-20.
pub const IDS_SETTINGS_NOT_SAVED: u16 = 3303;
/// «Работает» — the first line of «Состояние», task T-34-5, finding Н80.
pub const IDS_STATE_WORKING: u16 = 3215;
/// «Приостановлена» — FR-90.
pub const IDS_STATE_SUSPENDED: u16 = 3216;
/// «Перехват клавиатуры не установлен» — FR-80.
pub const IDS_STATE_HOOK_ABSENT: u16 = 3217;
/// «Перехват установлен, но обработка отключена — нужен перезапуск» — FR-99, finding Н84.
pub const IDS_STATE_DISABLED: u16 = 3218;
/// The health of the hook after the state, shown only when the journal is on — finding С1:
/// silent removals and install failures, two places to fill in.
pub const IDS_STATE_HEALTH: u16 = 3219;
/// The joiner «{0} · {1}» between the two halves of a state line — the state and the health of
/// the hook (task T-34-5), the acting pair and its refusals (task T-34-6) — in the table so
/// that a locale may change it.
pub const IDS_STATE_JOINED: u16 = 3220;
/// «Действующая пара: {0} → {1}» — the fourth line of «Состояние», task T-34-6, finding Н102.
pub const IDS_STATE_PAIR: u16 = 3222;
/// «Действующий цикл: {0}» — the same line in mode «cycle», the layouts joined by «→».
pub const IDS_STATE_CYCLE: u16 = 3223;
/// «Действующая пара не выбрана» — nothing acts: fewer than two layouts, or a pair that
/// resolves to nothing.
pub const IDS_STATE_PAIR_NONE: u16 = 3224;
/// «отказов выбора не было» — zero said in words, not «отказов: 0».
pub const IDS_STATE_NO_FAILURES: u16 = 3225;
/// «отказов выбора: {0}» — the sum of the four counters of `layouts::selection_failures`.
pub const IDS_STATE_FAILURES: u16 = 3226;
/// «Журнал сохранён» — what the button «Сохранить журнал» says for [`JOURNAL_SAVED_MS`] after a
/// write that succeeded; task T-39-11, решение 122.5. The live acceptance of `e56` found the
/// button silent on success. 3227 continues the block of the state strings, as 3221 did.
pub const IDS_LOG_SAVED: u16 = 3227;
/// [`Refusal::Editing`] — «Эта клавиша нужна при наборе»; task T-36-5, finding Н9, решение 123.3.
///
/// 3228 continues the same block of the state strings (3216…3231), because that is where the free
/// numbers are — it belongs to the capture dialog and not to «Состояние». Identifiers are never
/// renumbered: one that moved would change what an installed build shows.
pub const IDS_CAPTURE_TYPING: u16 = 3228;
/// «Lang Switcher уже работает в этом сеансе» — the notification of FR-82 a second copy shows
/// before it exits; task T-43-9, finding Н23, решение 128.2.
///
/// Until that task the sentence was an English literal in `app` — the one English phrase of the
/// program in every locale — left there by the task that wrote FR-82 before FR-94 existed. 3229
/// continues the block of the state strings (3216…3231) for the reason 3228 does: that is where
/// the free numbers are. The language it is read in is **Windows'**, not the configuration's —
/// the second copy exits before it reads any file (`app::notify_already_running`).
pub const IDS_ALREADY_RUNNING: u16 = 3229;
/// The line under the hotkey field **at rest** — «Клавиша, сочетание с Ctrl, Alt, Shift или
/// двойной Shift.» since task T-95-2 (the mock-up of 157.18), «Клавиша или сочетание с Ctrl, Alt,
/// Shift.» before it; task T-93-2, вопрос 157, устройство А (157.6): there is no switch to choose a
/// way, so the ways stand under the field where they are seen without reading the help. A warning
/// of FR-92 ([`hotkey_note`]) takes its place when there is one to give. 3230 is the next free
/// number of the block of the state strings (3216…3231), as 3228 and 3229 were.
pub const IDS_HOTKEY_HINT: u16 = 3230;
/// The line under the field **once**, after a combination is captured — «{0} без {1} остаётся
/// программам.», `{0}` the key, `{1}` the modifiers as the field shows them (task T-93-2, the
/// mock-up of 157.5). It stands until the window closes or the next capture is armed. 3231 is the
/// last free number of the block.
pub const IDS_CAPTURE_COMBO_NOTE: u16 = 3231;
/// The line under the field while a capture waits for the second `Shift` — «Ещё раз Shift —
/// двойное нажатие; клавиша с Shift — сочетание.» (task T-95-2, вопрос 159, the moments 3 and 4 of
/// the mock-up of 157.18). It stays after the window of the pair has run out, until the next press:
/// 400 ms is too short to read it in. 3232 opens the next block of sixteen — 3216…3231 is full.
pub const IDS_CAPTURE_SHIFT_AGAIN: u16 = 3232;
/// The line under the field **once**, after the double press is captured — «Два нажатия Shift
/// подряд. Обычный набор с Shift не затронут.» (task T-95-2, the moment 5 of the mock-up of
/// 157.18), as [`IDS_CAPTURE_COMBO_NOTE`] after a combination.
pub const IDS_CAPTURE_DOUBLE_NOTE: u16 = 3233;

// The eight strings of the two ways of the panel «Исключения» — task T-100-3, вопрос 172 (the
// mock-up of 172.4, variant В). 3234…3241 continue the block of sixteen 3232…3247 that the double
// press of Shift opened; they were the first free numbers when the task wrote them.
/// «Автоматически» — the word over the way that takes the program out of its window.
pub const IDS_EXCLUSION_AUTO: u16 = 3234;
/// «Вручную» — the word over the way that has always been there: the field and «Добавить».
pub const IDS_EXCLUSION_MANUAL: u16 = 3235;
/// «Выбрать окно программы» — the button of the automatic way, as it stands at rest.
pub const IDS_EXCLUSION_PICK: u16 = 3236;
/// «Нажмите и за 5 секунд переключитесь в программу» — the line under the button at rest. The «5»
/// is `guard::CAPTURE_SECONDS`, and a test holds the sentence and the number together.
pub const IDS_EXCLUSION_PICK_NOTE: u16 = 3237;
/// «Переключитесь в нужную программу… {0}» — the button while it counts, `{0}` the seconds left.
pub const IDS_EXCLUSION_PICK_COUNT: u16 = 3238;
/// «Добавлено: {0}» — the line under the button after the program went into the list.
pub const IDS_EXCLUSION_ADDED: u16 = 3239;
/// «Уже в списке: {0}» — the same line when the list held it already; its row is selected.
pub const IDS_EXCLUSION_ALREADY: u16 = 3240;
/// «Окно не выбрано» — the same line when there was nothing to take: no window in front, a window
/// of this program, or a name that cannot be an exclusion.
pub const IDS_EXCLUSION_NOT_PICKED: u16 = 3241;
// Task T-103-2, решение 181.1: the button of «Спасибо» that sends a person where they can write —
// the first free number when the task wrote it, in the block 3232…3247.
/// «Открыть группу» / «Open GitHub Issues» — the panel of «Спасибо», `letters::links::feedback_url`
/// of the language of the letter. `IDS_CHANNEL_OPEN` stays with «Что нового» and FR-103.
pub const IDS_FEEDBACK_OPEN: u16 = 3242;
// Task T-103-7, решение 181.6: the last step of an appeal about a trouble — a letter to the author;
// since task T-107-3 (решение 199.3) the letter of any appeal the site did not take. 3243…3247
// close the block 3232…3247, 3248 opens the next. The address is `{0}` in every one of them: the
// tables of fourteen languages hold no copy of `letters::links::MAIL_ADDRESS`.
// ⛔ 3243 is a hole since task T-107-3: «… отправьте его автору письмом — не в открытое
// обсуждение» said why a trouble went by letter; every appeal goes to the site now, and the note of
// the way back is [`IDS_WIZARD_FAILED_NOTE`].
/// «Адрес для письма: {0}» — the line of the address.
pub const IDS_WIZARD_MAIL_ADDRESS: u16 = 3244;
/// «Скопировать текст» — the clipboard and nothing else.
pub const IDS_WIZARD_COPY_TEXT: u16 = 3245;
/// «Скопировать адрес».
pub const IDS_WIZARD_COPY_ADDRESS: u16 = 3246;
/// «Текст скопирован. Вставьте его в письмо на {0}.» — the answer to «Скопировать текст».
pub const IDS_WIZARD_TEXT_COPIED: u16 = 3247;
/// «Адрес скопирован: {0}.» — the answer to «Скопировать адрес».
pub const IDS_WIZARD_ADDRESS_COPIED: u16 = 3248;
// ⛔⛔ **3249, 3250 — `IDS_TOAST_UPDATED_TITLE`, `IDS_TOAST_UPDATED` (шар «Lang Switcher обновлён до
// X», T-104-7) — сняты задачей T-119-8, решение 208.94 п. 4:** после обновления говорит окно
// «Обновлено». Номера не занимать.
/// The separator of a list the program builds — «, » in every language until a translation gives
/// its own — stage L10N, task T-L10N-10 (decisions 200.10 and 200.10а). Until 1.1.0 a literal of
/// the code, which a translation could not reach; joins the layouts of [`IDS_LAYOUT_NOTE`].
pub const IDS_LIST_SEPARATOR: u16 = 3251;
/// The arrow between the layouts of a cycle in [`IDS_STATE_CYCLE`] — « → » with its spaces; a
/// right-to-left translation chooses its direction. Task T-L10N-10.
pub const IDS_CYCLE_ARROW: u16 = 3252;
/// The tooltip of the tray icon: `{0}` the name of the program, `{1}` [`IDS_TIP_ACTIVE`] or
/// [`IDS_TIP_PAUSED`] — «{0} — {1}» until a translation gives its own order. Task T-L10N-10.
pub const IDS_TIP_FORMAT: u16 = 3253;
// Stage E107, task T-107-3 (решения 199.3, 199.10, 199.11, 201.11а): the appeal goes to the site
// from the last step of the wizard. 3254…3262, the block 3248…3263 again.
/// «Отправить» — the main button of the last step.
pub const IDS_WIZARD_SEND: u16 = 3254;
/// «Отправляется…» — the same button while the appeal is on its way.
pub const IDS_WIZARD_SENDING: u16 = 3255;
/// «Повторить» — the same button once the site did not take it.
pub const IDS_WIZARD_RETRY: u16 = 3256;
/// «Как с вами связаться — почта или Telegram, по желанию, если понадобится уточнить» — the label
/// of the contact (201.11а: one ink, no hint inside the field).
pub const IDS_WIZARD_CONTACT: u16 = 3257;
/// «Для связи:» — the label of the contact's line in the text of the appeal (199.10).
pub const IDS_WIZARD_CONTACT_LINE: u16 = 3258;
/// «Не удалось отправить» — the heading of the last step when the site did not take the appeal.
pub const IDS_WIZARD_FAILED_TITLE: u16 = 3259;
/// Its note — check the connection and «Повторить», or the letter.
pub const IDS_WIZARD_FAILED_NOTE: u16 = 3260;
/// «Обращение отправлено» — the heading of «Готово».
pub const IDS_WIZARD_SENT_TITLE: u16 = 3261;
/// The owner's words of thanks, решение 199.10 — for a trouble and for an idea alike.
pub const IDS_WIZARD_SENT_TEXT: u16 = 3262;
/// The caption of the button the about window gains — FR-103.
pub const IDS_ABOUT_AUTHOR: u16 = 3139;
// ⛔⛔ **3140 — `IDS_ENTRY_UPDATE_MARK` — снята задачей T-119-9: строка «Последних писем» об
// обновлении говорит словами письма (3297, 3302).** Номер не занимать.
/// The same line for a news item that has not been read.
pub const IDS_ENTRY_UNREAD_MARK: u16 = 3141;
/// And for one that has.
pub const IDS_ENTRY_READ_MARK: u16 = 3142;

// The ten strings the two letters **out of the feed** need — task Т-32-6, ступень Б. Stage А
// left both letters with the entry's own words and nothing around them; FR-102 and the mock-up
// ask for the frame: what version you have, how to update, and what «Позже» actually promises.
// ⛔⛔ **3143 — `IDS_UPDATE_SUB` — снята задачей T-119-9: подпись письма — 3298, с версией из
// ленты.** Номер не занимать.
/// The caption of its panel — the three steps of updating.
pub const IDS_UPDATE_HOW: u16 = 3144;
/// ⚠ The first step names the button below it by name; if that button is ever renamed, this
/// sentence is renamed with it in all fourteen tables.
pub const IDS_UPDATE_STEP_1: u16 = 3145;
/// The second step — see [`IDS_UPDATE_STEP_1`].
pub const IDS_UPDATE_STEP_2: u16 = 3146;
/// The third step — see [`IDS_UPDATE_STEP_1`].
pub const IDS_UPDATE_STEP_3: u16 = 3147;
/// The line under «Обновление»: the menu entry stays until the update happens.
pub const IDS_UPDATE_FOOT: u16 = 3148;
/// «Новость» — the heading of the window itself; the entry's own heading goes inside the panel.
pub const IDS_NEWS_LETTER_TITLE: u16 = 3149;
/// Its left button: **not** «Закрыть», because closing and postponing are the same act here and
/// the letter says so out loud.
pub const IDS_NEWS_LATER: u16 = 3150;
/// The button that opens the entry's link — a news entry points anywhere, not at a download.
pub const IDS_NEWS_OPEN_LINK: u16 = 3151;
/// The line under «Новость», word for word from the mock-up: what «Прочитано» does and what
/// «Позже» and the cross do instead.
pub const IDS_NEWS_FOOT: u16 = 3152;

// =========================================================================================
// The wizard «Написать автору» — FR-104, task Т-32-8, ступень В
// =========================================================================================
//
// Fifty-seven strings and not the «≈45» the mandate estimated; the difference is written down
// in the report with what it bought. Two economies were made and are worth naming here: the
// **report** of FR-104 is labelled with the wizard's own strings rather than a second set of
// its own, and the buttons that already exist («Закрыть», «Открыть канал») are reused.

/// The caption of the wizard window.
pub const IDS_WIZARD_CAPTION: u16 = 3153;
/// «Шаг {0} из {1}» — the line over the heading of every step.
pub const IDS_WIZARD_STEP: u16 = 3154;
/// «Отмена» — the button that closes the wizard without writing anything.
pub const IDS_WIZARD_CANCEL: u16 = 3155;
/// «Назад».
pub const IDS_WIZARD_BACK: u16 = 3156;
/// «Далее».
pub const IDS_WIZARD_NEXT: u16 = 3157;
/// «Готово» — the same button on the last step.
pub const IDS_WIZARD_DONE: u16 = 3158;

/// Step 1: «Что случилось?».
pub const IDS_WIZARD_WHAT_TITLE: u16 = 3159;
/// Its note — why the question is asked first.
pub const IDS_WIZARD_WHAT_NOTE: u16 = 3160;
/// The first card: «Программа сделала не то».
pub const IDS_WIZARD_CARD_WRONG: u16 = 3161;
/// Its second line.
pub const IDS_WIZARD_CARD_WRONG_SUB: u16 = 3162;
/// The second card: «Ничего не произошло».
pub const IDS_WIZARD_CARD_NOTHING: u16 = 3163;
/// Its second line.
pub const IDS_WIZARD_CARD_NOTHING_SUB: u16 = 3164;
/// The third card: «Хочу предложить улучшение» — the short road.
pub const IDS_WIZARD_CARD_IDEA: u16 = 3165;
/// Its second line.
pub const IDS_WIZARD_CARD_IDEA_SUB: u16 = 3166;

/// Step 2: «Где это случилось?».
pub const IDS_WIZARD_WHERE_TITLE: u16 = 3167;
/// Its note — why the window matters.
pub const IDS_WIZARD_WHERE_NOTE: u16 = 3168;
/// «Программа:» — the label of the field the capture fills.
pub const IDS_WIZARD_PROGRAM: u16 = 3169;
/// «Взять из активного окна».
pub const IDS_WIZARD_CAPTURE: u16 = 3170;
/// What the capture takes and what it deliberately does not.
pub const IDS_WIZARD_CAPTURE_NOTE: u16 = 3171;
/// The count-down while it waits — «Переключитесь в нужное окно… {0}».
pub const IDS_WIZARD_CAPTURE_COUNT: u16 = 3172;
/// The label of the report line for the kind of field — the radio group has none on screen.
pub const IDS_WIZARD_WHERE_FIELD: u16 = 3173;
/// «Обычное поле ввода».
pub const IDS_WIZARD_FIELD_NORMAL: u16 = 3174;
/// «Поле пароля».
pub const IDS_WIZARD_FIELD_PASSWORD: u16 = 3175;
/// Why that answer already explains a great deal — SEC-02.
pub const IDS_WIZARD_FIELD_PASSWORD_SUB: u16 = 3176;
/// «Не знаю».
pub const IDS_WIZARD_FIELD_UNKNOWN: u16 = 3177;

/// Step 3: «Что вы делали?».
pub const IDS_WIZARD_DID_TITLE: u16 = 3178;
/// Its note — write an example, the program keeps no keystrokes.
pub const IDS_WIZARD_DID_NOTE: u16 = 3179;
/// «Набрали слово в раскладке».
pub const IDS_WIZARD_TYPED_IN: u16 = 3180;
/// «и нажали» — the words before the key chip.
pub const IDS_WIZARD_AND_PRESSED: u16 = 3181;
/// «Ожидали:».
pub const IDS_WIZARD_EXPECTED: u16 = 3182;
/// «Получили:».
pub const IDS_WIZARD_GOT: u16 = 3183;
/// «Повторяется?».
pub const IDS_WIZARD_REPEAT: u16 = 3184;
/// «каждый раз».
pub const IDS_WIZARD_REPEAT_ALWAYS: u16 = 3185;
/// «иногда».
pub const IDS_WIZARD_REPEAT_SOMETIMES: u16 = 3186;
/// «один раз».
pub const IDS_WIZARD_REPEAT_ONCE: u16 = 3187;

/// The idea road's only step: «Опишите идею».
pub const IDS_WIZARD_IDEA_TITLE: u16 = 3188;
/// Its note.
pub const IDS_WIZARD_IDEA_NOTE: u16 = 3189;
/// «Что предлагаете?».
pub const IDS_WIZARD_IDEA_WHAT: u16 = 3190;
/// «Чем это поможет?».
pub const IDS_WIZARD_IDEA_HELPS: u16 = 3191;
/// «Что написано сейчас?» — the first field of the page of the idea for an appeal about the
/// translation, task T-120-22 (решение 208.169, С12). Block 3280…3295.
pub const IDS_WIZARD_TRANSLATION_NOW: u16 = 3290;
/// «Как правильно?» — the second field.
pub const IDS_WIZARD_TRANSLATION_RIGHT: u16 = 3291;
/// «Ошибка в переводе: {0}» — the first line of such an appeal; {0} the tag of the language.
pub const IDS_WIZARD_TRANSLATION_LINE: u16 = 3292;

/// Step 4: «Что приложить?».
pub const IDS_WIZARD_ATTACH_TITLE: u16 = 3192;
/// Its note — every box can be taken off, and what each adds is written beside it.
pub const IDS_WIZARD_ATTACH_NOTE: u16 = 3193;
/// «Версия программы и сборка Windows».
pub const IDS_WIZARD_ATTACH_MACHINE: u16 = 3194;
/// «Раскладки в системе».
pub const IDS_WIZARD_ATTACH_LAYOUTS: u16 = 3195;
/// «Настройки».
pub const IDS_WIZARD_ATTACH_SETTINGS: u16 = 3196;
/// «Журнал программы из памяти».
pub const IDS_WIZARD_ATTACH_JOURNAL: u16 = 3197;
/// What that journal holds — «{0} записей. Только имена операций и коды ошибок…».
pub const IDS_WIZARD_ATTACH_JOURNAL_SUB: u16 = 3198;
/// The line under the four boxes: why no journal **file** is needed.
pub const IDS_WIZARD_ATTACH_FOOT: u16 = 3199;

/// The last step: «Проверьте и отправьте».
pub const IDS_WIZARD_PREVIEW_TITLE: u16 = 3200;
/// Its note — the text goes to the author only when the person presses «Отправить» (задача
/// T-107-4, решение 199.5; until 1.2.0 — «nothing leaves the machine until the person pastes it»).
pub const IDS_WIZARD_PREVIEW_NOTE: u16 = 3201;
// ⛔ 3202, 3204 and 3205 are holes since task T-107-3 (решение 199.3): «Скопировать и открыть
// группу» / «Copy and open GitHub Issues» and its two answers left with the group and the issues,
// which the wizard no longer opens. Holes are not walked.
/// «Сохранить в папку журнала».
pub const IDS_WIZARD_SAVE: u16 = 3203;
/// «Сохранено: {0}» — the file that was written.
pub const IDS_WIZARD_SAVED: u16 = 3206;
/// The one thing that can go wrong on that step and be worth saying out loud.
pub const IDS_WIZARD_FAILED: u16 = 3207;

// ⛔ 3208…3211 are holes since task T-107-3 (решение 199.10): the two headings and the two
// paragraphs of the thanks — for a trouble and for an idea — gave way to one heading and one
// paragraph of the owner's words for both, [`IDS_WIZARD_SENT_TITLE`] and [`IDS_WIZARD_SENT_TEXT`].

// ⚠ The button that opens the wizard from the «Диагностика» section of the settings window
// has **no string of its own**: it says exactly what the tray entry says, and that is
// [`IDS_WRITE_TO_AUTHOR`]. Two strings obliged to stay identical in fourteen tables are two
// chances to drift apart.

/// Every identifier above, so that a test can walk the whole vocabulary of the interface.
///
/// Exported rather than rebuilt in the test: what the test must not import is the *text*, and
/// it does not — it writes every string out itself. The list of identifiers is the contract
/// between `app.rc` and this file, and a test that walked a list of its own would not be
/// checking that contract at all.
/// ⚠ **Two hundred and three since task Т-32-8**, and the canon of «seventy-three» that
/// stood here is authorised away by the mandate of Э32 («канон `INTERFACE_STRINGS` растёт с
/// 73 — цифру в отчёт»): the sixty-one strings of the letters from the author arrived with
/// Т-32-3 (FR-101, FR-102, FR-103), ten more with the two letters out of the feed
/// (Т-32-6, ступень Б), and fifty-nine with the wizard of FR-104 (Т-32-8, ступень В).
/// Before that it was seventy-three since task Т-31-4, and
/// seventy-two before решение 99.4 retired `IDS_LANGUAGE_RESTART` with the sentence it carried
/// and brought the two words of the tray tooltip. The list is of *identifiers in use*, not of
/// numbers in the range — 3004 is a hole and holes are not walked.
///
/// ⚠ **Two hundred and eighteen since task T-43-9** (решение 128.2 authorised 217 → 218): the
/// notification of FR-82 left its English literal in `app` for [`IDS_ALREADY_RUNNING`], the
/// last row.
///
/// ⚠⚠ **And two hundred and seventeen since task T-81-3** — решение 142.2 п. 5. The canon went
/// **down** for the first time in the life of this program: `IDS_AUTHOR_VERSION` (3111) is gone,
/// with all fourteen of its translations, because the owner asked for the author's signature to
/// leave the head of «От автора» («не надо вставлять панда, он же панда») and the two windows
/// of that head now share one version line, [`IDS_ABOUT_VERSION`]. A canon that only ever grows
/// is a canon nobody may tidy; this one moves by a decision, in either direction, and 3111 joins
/// 3004 as a hole that is not walked.
///
/// ⚠ **Two hundred and eighteen since task T-93-2** — вопрос 157, 157.3: `IDS_CAPTURE_COMBINATION`
/// (3041) left with the refusal it said, and [`IDS_HOTKEY_HINT`] and [`IDS_CAPTURE_COMBO_NOTE`]
/// arrived — 217 − 1 + 2. 3041 is the third hole that is not walked.
///
/// ⚠ **Two hundred and twenty since task T-95-2** — вопрос 159, 157.18: the double press of `Shift`
/// brought [`IDS_CAPTURE_SHIFT_AGAIN`] and [`IDS_CAPTURE_DOUBLE_NOTE`], the first two numbers of a
/// new block (3232…3247).
///
/// ⚠ **Two hundred and twenty-eight since task T-100-3** — вопрос 172: the two ways of «Исключения»
/// brought eight, [`IDS_EXCLUSION_AUTO`] … [`IDS_EXCLUSION_NOT_PICKED`] (3234…3241).
///
/// ⚠ **Two hundred and twenty-nine since task T-103-2** — вопрос 181, решение 181.1: «Спасибо» sends
/// a person where they can write, and its button got a word of its own, [`IDS_FEEDBACK_OPEN`]
/// (3242).
///
/// ⚠ **Two hundred and thirty-five since task T-103-7** — решение 181.6: an appeal about a trouble goes
/// to the author by letter, and its last step got six words of its own, [`IDS_WIZARD_MAIL_NOTE`] …
/// [`IDS_WIZARD_ADDRESS_COPIED`] (3243…3248).
///
/// ⚠ **Two hundred and thirty-seven since task T-104-7** — TZ E104 §0 п. 5 и 12 (решения 189.3,
/// 189.18): the program updates itself, and the balloon after an update got its two words,
/// `IDS_TOAST_UPDATED_TITLE` and `IDS_TOAST_UPDATED` (3249, 3250).
///
/// ⚠ **Two hundred and forty since stage L10N, task T-L10N-10** — решения 200.10 и 200.10а: the
/// three pieces the code glued as literals became strings, [`IDS_LIST_SEPARATOR`],
/// [`IDS_CYCLE_ARROW`] and [`IDS_TIP_FORMAT`] (3251…3253).
///
/// ⚠ **Two hundred and forty-one since stage E107, task T-107-3** — решения 199.3, 199.10, 199.11:
/// the appeal goes to the site from the last step of the wizard. Nine arrived,
/// [`IDS_WIZARD_SEND`] … [`IDS_WIZARD_SENT_TEXT`] (3254…3262); eight left with the group, the
/// issues, the note of the letter of a trouble and the two thanks — 3202, 3204, 3205, 3208…3211 and
/// 3243 are holes that are not walked. 240 − 8 + 9.
///
/// ⚠ **Two hundred and thirty-one since stage E119, task T-119-8** — решение 208.94 п. 4: the letter
/// «Что нового» left with its eight words and its balloon (3102…3109, 3212), and the balloon
/// «обновлён до X» with its two (3249, 3250) — eleven holes that are not walked. 242 − 11.
///
/// ⚠ **Two hundred and thirty-two since task T-119-2** — FR-90б: the check box of the flag of the
/// layout, [`IDS_TRAY_FLAG`] (3264).
///
/// ⚠ **Two hundred and forty-five since stage E120, tasks T-120-21 and T-120-22** — решение
/// 208.169, С10 and С12: «Состояние» for a person, nine (3280…3288), and «Нашли ошибку в
/// переводе?», four (3289…3292). 232 + 13.
///
/// ⚠ **Two hundred and forty-seven since task T-119-3, part Б** — решение 208.88: the title and
/// the link of the list of the program's own, [`IDS_OWN_LIST_TITLE`] and [`IDS_OWN_LIST_MORE`]
/// (3265, 3266). 245 + 2.
///
/// ⚠ **Two hundred and sixty since tasks T-119-5, T-119-6, T-119-7 and T-119-10** — the window in
/// the corner (the hint of the flag, «Обновлено»), the entry «Что нового в X…» and the switch of «От
/// автора»: thirteen, 3267…3279, the rest of the block. 247 + 13.
///
/// ⚠ **Two hundred and sixty-two since task T-120-16** — the header of the window in the corner that
/// knocks with a letter, [`IDS_CORNER_LETTER`] (3293). 261 + 1.
///
/// ⚠ **Two hundred and sixty-three since the texts of «Привет» (T-120-16, 208.106 п. 3)** — the line
/// under its body, [`IDS_HELLO_FOOT`] (3294). 262 + 1.
///
/// ⚠ **Two hundred and sixty-five since task T-120-20** — the window of the settings: the note of a
/// refused write, [`IDS_SETTINGS_NOT_SAVED`] (3303), and the badge «новое», [`IDS_NEW_BADGE`] (3304).
/// 263 + 2.
///
/// ⚠ **Two hundred and sixty-six since task T-120-21** — «Копировать» of the small window of
/// «Подробности для автора», [`IDS_STATUS_COPY`] (3295). 265 + 1.
///
/// ⚠ **Two hundred and seventy-two since the C3 of task T-120-20** — the subtitles of the five
/// sections (3307…3311) and the eyebrow of the window of the settings (3312). 266 + 6.
///
/// ⚠ **Two hundred and sixty-seven since task T-120-17** — the two buttons of the card «Поддержать
/// автора» of «От автора», [`IDS_AUTHOR_SUPPORT`] (3305) and [`IDS_AUTHOR_CHANNEL`] (3306). 272 + 2 = 274 since the merge of the shell after Б.
pub const INTERFACE_STRINGS: [u16; 274] = [
    IDS_DIALOG_CAPTION,
    IDS_GROUP_GENERAL,
    IDS_AUTOSTART,
    IDS_LANGUAGE_LABEL,
    IDS_TRANSLATION_REPORT,
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
    IDS_SELECTION_ENABLED,
    IDS_GROUP_EXCLUSIONS,
    IDS_EXCLUSION_REMOVE,
    IDS_EXCLUSION_ADD,
    IDS_EXCLUSION_HINT,
    IDS_GROUP_DIAGNOSTICS,
    IDS_LOG_ENABLED,
    IDS_LOG_OPEN,
    IDS_LOG_SAVE,
    IDS_LOG_SAVE_FAILED,
    IDS_SETTINGS_NOT_SAVED,
    IDS_LOG_SAVED,
    IDS_LOG_DIR_LABEL,
    IDS_GROUP_STATE,
    IDS_OK,
    IDS_CANCEL,
    IDS_APPLY,
    IDS_NOTE_TEXT_KEY,
    IDS_NOTE_UNKNOWN_KEY,
    IDS_CAPTURE_PROMPT,
    IDS_CAPTURE_MODIFIER,
    IDS_CAPTURE_EMERGENCY,
    IDS_CAPTURE_NAMELESS,
    IDS_CAPTURE_RESERVED,
    // Task T-36-5, finding Н9 — the seventh reason a capture refuses. The number lives in the
    // state block (3228), the string belongs to the capture dialog.
    IDS_CAPTURE_TYPING,
    IDS_STATE_WORKING,
    IDS_STATE_SUSPENDED,
    IDS_STATE_HOOK_ABSENT,
    IDS_STATE_DISABLED,
    IDS_STATE_HEALTH,
    IDS_STATE_JOINED,
    IDS_STATE_PAIR,
    IDS_STATE_CYCLE,
    IDS_STATE_PAIR_NONE,
    IDS_STATE_NO_FAILURES,
    IDS_STATE_FAILURES,
    IDS_STATE_LAYOUTS,
    IDS_STATE_AUTOSTART,
    IDS_AUTOSTART_PRESENT,
    IDS_AUTOSTART_ABSENT,
    IDS_STATUS_WORKING,
    IDS_STATUS_SUSPENDED,
    IDS_STATUS_RESTART,
    IDS_STATUS_FIXES,
    IDS_STATUS_FIXES_NONE,
    IDS_STATUS_HOTKEY,
    IDS_STATUS_AUTOSTART_ON,
    IDS_STATUS_AUTOSTART_OFF,
    IDS_STATUS_DETAILS,
    IDS_STATUS_COPY,
    IDS_SECTION_GENERAL_SUB,
    IDS_SECTION_HOTKEY_SUB,
    IDS_SECTION_LAYOUTS_SUB,
    IDS_SECTION_EXCLUSIONS_SUB,
    IDS_SECTION_DIAGNOSTICS_SUB,
    IDS_SETTINGS_EYEBROW,
    IDS_LAYOUT_NOTE,
    IDS_LAYOUT_WORD_SOURCE,
    IDS_LAYOUT_WORD_TARGET,
    IDS_LOG_DIR_MISSING,
    IDS_THEME_LABEL,
    IDS_THEME_SYSTEM,
    IDS_THEME_LIGHT,
    IDS_THEME_DARK,
    IDS_ABOUT_CAPTION,
    IDS_ABOUT_VERSION,
    IDS_ABOUT_LINE_1,
    IDS_ABOUT_LINE_2,
    IDS_ABOUT_OK,
    IDS_ABOUT_HELP,
    IDS_ABOUT_HELP_1,
    IDS_ABOUT_HELP_2,
    IDS_ABOUT_HELP_3,
    IDS_ABOUT_HELP_4,
    IDS_ABOUT_HELP_5,
    IDS_ABOUT_HELP_6,
    IDS_MENU_SUSPEND,
    IDS_MENU_RESUME,
    IDS_MENU_RESUME_RESTART,
    IDS_MENU_SETTINGS,
    IDS_MENU_AUTOSTART,
    IDS_MENU_ABOUT,
    IDS_MENU_EXIT,
    IDS_SOUND,
    IDS_CAPTURE_HINT,
    IDS_TIP_ACTIVE,
    IDS_TIP_PAUSED,
    IDS_LETTER_CAPTION,
    IDS_CLOSE,
    IDS_CHANNEL_OPEN,
    IDS_SUPPORT_OPEN,
    IDS_HELLO_TITLE,
    IDS_HELLO_LEAD,
    IDS_HELLO_DEMO_CAP,
    IDS_HELLO_PANEL,
    IDS_HELLO_ROW_1,
    IDS_HELLO_ROW_2,
    IDS_HELLO_ROW_3,
    IDS_HELLO_FOOT,
    IDS_HELLO_SETTINGS,
    IDS_HELLO_OK,
    IDS_THANKS_TITLE,
    IDS_THANKS_LEAD,
    IDS_THANKS_PARA,
    IDS_SUPPORT_PANEL,
    IDS_SUPPORT_TEXT,
    IDS_THANKS_SNOOZE,
    IDS_THANKS_FOOT,
    IDS_AUTHOR_CAPTION,
    IDS_AUTHOR_PANEL,
    IDS_AUTHOR_TEXT,
    IDS_NEWS_PANEL,
    IDS_NEWS_ABOUT_FEED,
    IDS_NEWS_NEVER_READ,
    IDS_NEWS_READ_ON,
    IDS_NEWS_LATEST,
    IDS_NEWS_AVAILABLE,
    IDS_NEWS_DOWNLOAD,
    IDS_NEWS_LETTERS,
    IDS_NEWS_FILE_ONLY,
    IDS_FEEDBACK_PANEL,
    IDS_FEEDBACK_TEXT,
    IDS_WRITE_TO_AUTHOR,
    IDS_LETTERS_CAPTION,
    IDS_LETTERS_FOOT,
    IDS_LETTERS_OPEN,
    IDS_NEWS_READ_BUTTON,
    IDS_MENU_UNREAD,
    IDS_MENU_UPDATE,
    IDS_TOAST_TITLE,
    IDS_TOAST_NEWS,
    IDS_TOAST_THANKS,
    IDS_ABOUT_AUTHOR,
    IDS_ENTRY_UNREAD_MARK,
    IDS_ENTRY_READ_MARK,
    IDS_UPDATE_HOW,
    IDS_UPDATE_STEP_1,
    IDS_UPDATE_STEP_2,
    IDS_UPDATE_STEP_3,
    IDS_UPDATE_FOOT,
    IDS_NEWS_LETTER_TITLE,
    IDS_NEWS_LATER,
    IDS_NEWS_OPEN_LINK,
    IDS_NEWS_FOOT,
    IDS_WIZARD_CAPTION,
    IDS_WIZARD_STEP,
    IDS_WIZARD_CANCEL,
    IDS_WIZARD_BACK,
    IDS_WIZARD_NEXT,
    IDS_WIZARD_DONE,
    IDS_WIZARD_WHAT_TITLE,
    IDS_WIZARD_WHAT_NOTE,
    IDS_WIZARD_CARD_WRONG,
    IDS_WIZARD_CARD_WRONG_SUB,
    IDS_WIZARD_CARD_NOTHING,
    IDS_WIZARD_CARD_NOTHING_SUB,
    IDS_WIZARD_CARD_IDEA,
    IDS_WIZARD_CARD_IDEA_SUB,
    IDS_WIZARD_WHERE_TITLE,
    IDS_WIZARD_WHERE_NOTE,
    IDS_WIZARD_PROGRAM,
    IDS_WIZARD_CAPTURE,
    IDS_WIZARD_CAPTURE_NOTE,
    IDS_WIZARD_CAPTURE_COUNT,
    IDS_WIZARD_WHERE_FIELD,
    IDS_WIZARD_FIELD_NORMAL,
    IDS_WIZARD_FIELD_PASSWORD,
    IDS_WIZARD_FIELD_PASSWORD_SUB,
    IDS_WIZARD_FIELD_UNKNOWN,
    IDS_WIZARD_DID_TITLE,
    IDS_WIZARD_DID_NOTE,
    IDS_WIZARD_TYPED_IN,
    IDS_WIZARD_AND_PRESSED,
    IDS_WIZARD_EXPECTED,
    IDS_WIZARD_GOT,
    IDS_WIZARD_REPEAT,
    IDS_WIZARD_REPEAT_ALWAYS,
    IDS_WIZARD_REPEAT_SOMETIMES,
    IDS_WIZARD_REPEAT_ONCE,
    IDS_WIZARD_IDEA_TITLE,
    IDS_WIZARD_IDEA_NOTE,
    IDS_WIZARD_IDEA_WHAT,
    IDS_WIZARD_IDEA_HELPS,
    IDS_WIZARD_TRANSLATION_NOW,
    IDS_WIZARD_TRANSLATION_RIGHT,
    IDS_WIZARD_TRANSLATION_LINE,
    IDS_WIZARD_ATTACH_TITLE,
    IDS_WIZARD_ATTACH_NOTE,
    IDS_WIZARD_ATTACH_MACHINE,
    IDS_WIZARD_ATTACH_LAYOUTS,
    IDS_WIZARD_ATTACH_SETTINGS,
    IDS_WIZARD_ATTACH_JOURNAL,
    IDS_WIZARD_ATTACH_JOURNAL_SUB,
    IDS_WIZARD_ATTACH_FOOT,
    IDS_WIZARD_PREVIEW_TITLE,
    IDS_WIZARD_PREVIEW_NOTE,
    IDS_WIZARD_SAVE,
    IDS_WIZARD_SAVED,
    IDS_WIZARD_FAILED,
    // Task T-43-9, finding Н23 — the notification of FR-82, in the language of Windows.
    IDS_ALREADY_RUNNING,
    // Task T-93-2, вопрос 157 — the line under the hotkey field at rest, and the one after a
    // combination is captured.
    IDS_HOTKEY_HINT,
    IDS_CAPTURE_COMBO_NOTE,
    // Task T-95-2, вопрос 159 — the line while a capture waits for the second `Shift`, and the
    // one after the double press is captured.
    IDS_CAPTURE_SHIFT_AGAIN,
    IDS_CAPTURE_DOUBLE_NOTE,
    // Task T-100-3, вопрос 172 — the two ways of «Исключения».
    IDS_EXCLUSION_AUTO,
    IDS_EXCLUSION_MANUAL,
    IDS_EXCLUSION_PICK,
    IDS_EXCLUSION_PICK_NOTE,
    IDS_EXCLUSION_PICK_COUNT,
    IDS_EXCLUSION_ADDED,
    IDS_EXCLUSION_ALREADY,
    IDS_EXCLUSION_NOT_PICKED,
    IDS_FEEDBACK_OPEN,
    IDS_WIZARD_MAIL_ADDRESS,
    IDS_WIZARD_COPY_TEXT,
    IDS_WIZARD_COPY_ADDRESS,
    IDS_WIZARD_TEXT_COPIED,
    IDS_WIZARD_ADDRESS_COPIED,
    // Task T-104-7, TZ E104 §0 п. 5 и 12 — the balloon after an update.
    // Stage L10N, task T-L10N-10 — the pieces the code glued as literals until 1.1.0.
    IDS_LIST_SEPARATOR,
    IDS_CYCLE_ARROW,
    IDS_TIP_FORMAT,
    // Stage E107, task T-107-3 — the appeal goes to the site from the last step of the wizard.
    IDS_WIZARD_SEND,
    IDS_WIZARD_SENDING,
    IDS_WIZARD_RETRY,
    IDS_WIZARD_CONTACT,
    IDS_WIZARD_CONTACT_LINE,
    IDS_WIZARD_FAILED_TITLE,
    IDS_WIZARD_FAILED_NOTE,
    IDS_WIZARD_SENT_TITLE,
    IDS_WIZARD_SENT_TEXT,
    IDS_TRAY_FLAG,
    IDS_NEW_BADGE,
    IDS_AUTHOR_SUPPORT,
    IDS_AUTHOR_CHANNEL,
    IDS_OWN_LIST_TITLE,
    IDS_OWN_LIST_MORE,
    IDS_CORNER_UPDATE,
    IDS_RELEASE_TITLE,
    IDS_RELEASE_TEXT,
    IDS_RELEASE_CHANGES,
    IDS_RELEASE_HOWTO,
    IDS_CORNER_OK,
    IDS_RELEASE_NEVER,
    IDS_CORNER_FLAG,
    IDS_CORNER_LETTER,
    IDS_HINT_TITLE,
    IDS_HINT_TEXT,
    IDS_HINT_OPEN,
    IDS_MENU_WHATSNEW,
    IDS_UPDATES_SWITCH,
    IDS_NEWS_OFF_IN_FILE,
    IDS_FALLBACK_TITLE,
    IDS_FALLBACK_SUB,
    IDS_FALLBACK_NOTICE,
    IDS_FALLBACK_WHATS_NEW,
    IDS_FALLBACK_REPEATS,
    IDS_ENTRY_FALLBACK_MARK,
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
    text_in(ui_language(), id)
}

/// One interface string in `language`, whatever locale is in force — task T-43-9.
///
/// [`text`] is this function asked in the locale in force, and the two share one body. The one
/// reader that needs another language is the notification of FR-82: a second copy of the program
/// exits before it reads any configuration, so its sentence is read in the language Windows
/// prefers. Publishing that language through [`set_ui_language`] first would be a second
/// publication of «на каком языке программа», and решение 99 gives the process exactly one —
/// `tray::adopt_ui_language` (sweep `the_language_of_the_running_process_is_published_from_exactly_one_place`,
/// which the first draft of this task turned red).
pub fn text_in(language: Language, id: u16) -> String {
    string_from(resource_module(), language, id).unwrap_or_default()
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

    /// The same press as the set `hook` compares — `hook::MOD_CTRL`, `MOD_ALT`, `MOD_SHIFT`, and
    /// `MOD_WIN` for a `Windows` key (task T-93-2, вопрос 157).
    pub const fn set(self) -> u16 {
        use crate::hook::{MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN};

        (if self.ctrl { MOD_CTRL } else { 0 })
            | (if self.alt { MOD_ALT } else { 0 })
            | (if self.shift { MOD_SHIFT } else { 0 })
            | (if self.win { MOD_WIN } else { 0 })
    }

    /// What is held right now, from the message queue.
    ///
    /// `GetKeyState` and not `GetAsyncKeyState`: the answer wanted is the state as of the
    /// keystroke being handled, which is what the queue-synchronised call gives, and which is
    /// what the user actually pressed together.
    pub(crate) fn held_now() -> Self {
        Self {
            ctrl: key_is_down(VK_CONTROL),
            alt: key_is_down(VK_MENU),
            shift: key_is_down(VK_SHIFT),
            win: key_is_down(VK_LWIN) || key_is_down(VK_RWIN),
        }
    }
}

/// Whether one virtual key is down, as the message queue sees it.
///
/// `pub(crate)` с задачи Т-46-2: правило нажатия глифа живёт теперь в `widgets::glyph`, и его
/// `arrow_is_down` спрашивает то же самое тем же телом — второго спрашивающего у этой мерки
/// не заводится.
pub(crate) fn key_is_down(key: VIRTUAL_KEY) -> bool {
    // SAFETY: takes a virtual-key code by value and touches no memory of ours. The high bit of
    // the `i16` it answers is "down", which for a signed value is "negative".
    unsafe { GetKeyState(i32::from(key.0)) < 0 }
}

/// Why a press cannot become the hotkey — FR-94, FR-95, FR-96.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A modifier on its own. `Ctrl` is not a hotkey, it is half of one.
    ///
    /// ⚠ **`Combination` stood after this until task T-93-2** — a key with a modifier held was
    /// refused, because section 7 stored one key name and the callback compared one code. Вопрос
    /// 157 widened both (`[hotkey] modifiers`, schema 7; the set compared in `hook::classify`), so
    /// a combination of `Ctrl`, `Alt` and `Shift` is now taken, and the refusal went with its
    /// fourteen sentences. A modifier on its own is still refused — the single tap is rejected by
    /// 157.1, and the double one is stage 2.
    Modifier,
    /// `Ctrl+Alt+Shift+F12`, the emergency exit of FR-96. Never assignable, in either build.
    Emergency,
    /// A key the system owns and an application never gets as a plain key — and, since task
    /// T-93-2, **any press with a `Windows` key held**: `Win` is not one of the modifiers a
    /// hotkey may carry (вопрос 157), and the shell acts on its release whatever anybody else
    /// does with the press.
    Reserved,
    /// ⭐ **A key typing itself needs** — task T-36-5, finding Н9, решение 123.3.
    ///
    /// `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown` and the four arrows: row 3 of the
    /// FR-10 table, the keys that move the caret or take text away. Two things went wrong when
    /// one of them was accepted, and the second is the one a person pays for:
    ///
    /// * the row of FR-10 quietly stopped working — a press that is the hotkey never reaches the
    ///   buffer, so the boundary it was supposed to be never happened;
    /// * **the key was lost in every application.** FR-95 suppresses the hotkey whenever the
    ///   program is active, so somebody who assigned `Delete` had no `Delete` anywhere, with no
    ///   warning in the file and none in the dialog.
    ///
    /// ⛔ Not [`Self::Reserved`]: «the system owns this key» would be a lie about a key the
    /// system does not own. The list is read from `buffer::is_editing_key` — the table of FR-10
    /// lives there and is not copied here.
    ///
    /// ⚠ **The file is not touched by this.** `[hotkey] key = "Delete"` written by hand is still
    /// read, published and acted on: refusing it there would silently return the hotkey to
    /// `Pause`, which is substituting this program's choice for the person's own.
    ///
    /// ⚠ **With modifiers too — 157.12, task T-93-2.** `Shift` + an arrow selects text, `Ctrl` + an
    /// arrow walks a word, `Ctrl+Home` goes to the top: the combinations of these keys are the
    /// editing typing needs just as much, so the key decides and the set does not.
    Editing,
    /// ⭐ **A key that types a character** — task Т-23-5, решение 82.6.
    ///
    /// Until that task a text key was **taken**: the capture ended, the file got `Q`, and the
    /// note under the field carried the warning of FR-95 afterwards. The user asked for the
    /// other order — «текстовая клавиша — предупреждение и захват НЕ гаснет» — and accepted
    /// the live mock-up that behaves that way: the press is refused, the warning appears, and
    /// the capture goes on waiting for the next key instead of quietly assigning the one that
    /// was pressed by accident.
    ///
    /// ⚠ **A text key remains a legal hotkey** and FR-95 is untouched: `[hotkey] key = "Q"`
    /// written by hand is read, published and warned about exactly as before. What changed is
    /// the one road that used to assign one without asking — the capture.
    ///
    /// It says the FR-92 warning and not a sentence of its own ([`IDS_NOTE_TEXT_KEY`]): the
    /// warning already says the true and useful thing — while the program is active that key
    /// stops typing its character.
    ///
    /// ⚠ **The set decides since task T-93-2 — 157.10, вариант 1 until the owner's word.** A text
    /// key with `Ctrl` or `Alt` types nothing and is taken (`Ctrl+K`); with nothing held, with
    /// `Shift` (a capital, a symbol) or with `Ctrl` and `Alt` together (`AltGr`) it does type, and
    /// is refused — see `types_a_character`.
    Text,
    /// A key section 7 has no name for, so it could not be written to the file and read back.
    Nameless,
}

impl Refusal {
    /// The interface string that says this to the user.
    pub const fn string_id(self) -> u16 {
        match self {
            Self::Modifier => IDS_CAPTURE_MODIFIER,
            Self::Emergency => IDS_CAPTURE_EMERGENCY,
            Self::Reserved => IDS_CAPTURE_RESERVED,
            // Task T-36-5: a sentence of its own — the key is not the system's, it is the
            // typing's, and what the person loses by taking it is different.
            Self::Editing => IDS_CAPTURE_TYPING,
            // Task Т-23-5: the warning FR-92 already asks for, said at the moment of the
            // press instead of after it.
            Self::Text => IDS_NOTE_TEXT_KEY,
            Self::Nameless => IDS_CAPTURE_NAMELESS,
        }
    }
}

/// What one press during a capture amounts to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capture {
    /// The press is the hotkey. The value is the `[hotkey]` section 7 stores — the key's name and,
    /// since task T-93-2, the modifiers held with it in the canonical order — and
    /// [`Hotkey::binding`] reads it back to the very code and set it came from.
    Taken(Hotkey),
    /// **A `Shift` on its own — the beginning of a double press**, task T-95-2, вопрос 159: not a
    /// refusal any more (157.17 — the double press is `Shift`'s). What it becomes is decided by
    /// what follows, and that is the memory of [`CapturePair`]: a second tap — the double press; a
    /// key with the `Shift` held — a combination, as in stage 1; nothing — the invitation again.
    Shift,
    /// The press cannot be the hotkey. The capture stays armed and the reason is shown.
    Refused(Refusal),
}

/// One thing that can happen to an armed capture — task Т-23-5, решение 82.6.
///
/// The events the window procedures of the dialog can see, named apart from the Win32 messages
/// that carry them so that the machine below can be exercised without a window. Task T-95-2
/// added two, for the double press of `Shift`: the release of a key and the end of the window of
/// the pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEvent {
    /// A key went down while the field held the focus — the virtual code and the modifiers
    /// held with it.
    KeyDown(u16, Modifiers),
    /// **A key went up while the field held the focus** — task T-95-2: the release of `Shift`
    /// ends a tap of the double press; the release of any other key changes nothing.
    KeyUp(u16),
    /// **The window of the double press ran out** — task T-95-2: the one-shot timer
    /// [`CAPTURE_PAIR_TIMER`] of the dialog, armed by a press of `Shift`, has fired.
    PairTimeout,
    /// The field lost the keyboard focus. The flag says whether it went to the capture button
    /// itself, which is the one window that may take it without ending the capture: that
    /// button is about to report a click, and ending here would turn the click into a fresh
    /// arming.
    FocusLost { to_capture_button: bool },
    /// A press landed on the dialog's own ground — not on the field, not on the button, and
    /// not on any control that takes the focus (those arrive as [`Self::FocusLost`]).
    ClickBeside,
}

/// What one event does to the capture — task Т-23-5, решение 82.6.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureStep {
    /// Nothing at all: no capture is armed, or the event is one the machine deliberately
    /// lets pass.
    Ignore,
    /// The capture ends and the name that stood in the field before it comes back.
    Cancel,
    /// The press is the hotkey; the value is the `[hotkey]` section 7 stores — the key and,
    /// since task T-93-2, its modifiers.
    Take(Hotkey),
    /// The press cannot be the hotkey. **The capture stays armed** — a refusal is «not that
    /// one», not an end to the question — and the note says which of the six reasons it was.
    Refuse(Refusal),
    /// **A `Shift` went down on its own — the first tap of a double press or its second** (task
    /// T-95-2, the moment 3 of the mock-up of 157.18): the field says «Shift…», the line under it
    /// [`IDS_CAPTURE_SHIFT_AGAIN`], and the one-shot timer of the pair runs for
    /// `hook::DOUBLE_TAP_MS`. The capture stays armed.
    AwaitShift,
    /// **The second `Shift` did not come in time, or a tap was held too long** — the moment 4: the
    /// field says the invitation again, the line under it stays until the next press, and the timer
    /// is dead. The capture stays armed.
    ShiftLapsed,
}

/// **The double press of `Shift` as a capture waits for it** — task T-95-2, вопрос 159, the moments
/// of the mock-up of 157.18.
///
/// The memory the capture gained: [`capture_step`] decides one press at a time and cannot tell a
/// second tap from a first. The rule of times is the hook's own, [`crate::hook::Taps`] — one body
/// for the window that assigns the double press and the callback that answers to it, so the field
/// takes exactly the pair the hook will act on. What only a window can know is here: a `Shift` is
/// down (its auto-repeat is no new press), and what the window shows while a pair is open.
///
/// The times are the times of the window messages (`GetMessageTime`), as the hook's are the times of
/// its events; the modifiers held are the queue's (`GetKeyState`, [`Modifiers::held_now`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapturePair {
    /// A `Shift` is down, as the field saw it go down — what tells its auto-repeat from a new press.
    held: bool,
    /// The taps made so far, and the rule they are counted by.
    taps: crate::hook::Taps,
}

impl CapturePair {
    /// One event of an armed capture, with the memory of the pair — task T-95-2. `time` is the
    /// time of the message the event came in.
    ///
    /// * A press of `Shift` on its own ([`capture_step`] answers [`CaptureStep::AwaitShift`]) is a
    ///   tap begun — the first or the second — unless it is the auto-repeat of the `Shift` already
    ///   down, which is nothing.
    /// * The release of `Shift` ends the tap: the second tap within the window — the double press,
    ///   taken; the first — the pair waits, nothing changes on the screen; a tap held too long —
    ///   [`CaptureStep::ShiftLapsed`].
    /// * The end of the window ([`CaptureEvent::PairTimeout`]) with a pair open —
    ///   [`CaptureStep::ShiftLapsed`]; with none, a stale tick, nothing.
    /// * Any other press ends the pair and is decided as [`capture_step`] decides it: a key with
    ///   `Shift` held is a combination (stage 1), `Ctrl` or `Alt` on their own are refused, `Esc`
    ///   cancels. A focus lost or a click beside cancel the capture, the pair with it.
    pub fn step(&mut self, event: CaptureEvent, time: u32) -> CaptureStep {
        match event {
            CaptureEvent::KeyDown(vk, _) if SHIFT_KEYS.contains(&vk) => {
                let repeat = std::mem::replace(&mut self.held, true);

                match capture_step(true, event) {
                    CaptureStep::AwaitShift if repeat => CaptureStep::Ignore,
                    CaptureStep::AwaitShift => {
                        self.taps.press(time, true);
                        CaptureStep::AwaitShift
                    }
                    // A `Shift` under `Ctrl` or `Alt`: refused, and no tap.
                    other => {
                        self.taps.interrupt();
                        other
                    }
                }
            }

            CaptureEvent::KeyUp(vk) if SHIFT_KEYS.contains(&vk) => {
                if !std::mem::replace(&mut self.held, false) {
                    // The release of a `Shift` this capture never saw go down.
                    return CaptureStep::Ignore;
                }

                let pressing = self.taps.pressing();

                if self.taps.release(time, true) {
                    return CaptureStep::Take(Hotkey::double_shift());
                }

                if pressing && !self.taps.waiting() {
                    // The press was no tap — held longer than the window.
                    return CaptureStep::ShiftLapsed;
                }

                CaptureStep::Ignore
            }

            CaptureEvent::KeyUp(_) => CaptureStep::Ignore,

            CaptureEvent::PairTimeout => {
                if !self.open() {
                    return CaptureStep::Ignore;
                }

                self.taps.interrupt();
                CaptureStep::ShiftLapsed
            }

            CaptureEvent::KeyDown(..)
            | CaptureEvent::FocusLost { .. }
            | CaptureEvent::ClickBeside => {
                if matches!(event, CaptureEvent::KeyDown(..)) {
                    self.taps.interrupt();
                }

                capture_step(true, event)
            }
        }
    }

    /// Whether a pair is open — a tap made, or a press that may be one: the field says «Shift…»
    /// and the timer of the pair runs.
    pub fn open(&self) -> bool {
        self.taps.pressing() || self.taps.waiting()
    }
}

/// The whole decision of the capture, as a function of the state and one event — task Т-23-5.
///
/// Split out of the window procedures for the reason [`capture`] itself is split out of them:
/// what a program does is worth closing without a window, and a machine spread over a
/// `WM_KEYDOWN` arm, a `WM_KILLFOCUS` arm and a `WM_LBUTTONDOWN` arm is a machine no test can
/// see whole. The procedures keep exactly two jobs — turning a message into a
/// [`CaptureEvent`], and carrying out the [`CaptureStep`] this function answers.
///
/// ⚠ **Escape is the way out and is therefore the one key a capture cannot assign.** It stays
/// assignable by hand: `hook::vk_from_name` still reads `Escape` and `Esc` out of the file, and
/// the dialog shows whatever it finds there.
pub fn capture_step(armed: bool, event: CaptureEvent) -> CaptureStep {
    if !armed {
        return CaptureStep::Ignore;
    }

    match event {
        CaptureEvent::KeyDown(vk, _) if vk == VK_ESCAPE.0 => CaptureStep::Cancel,

        CaptureEvent::KeyDown(vk, modifiers) => match capture(vk, modifiers) {
            Capture::Taken(hotkey) => CaptureStep::Take(hotkey),
            // Task T-95-2: one press at a time, a `Shift` on its own is the beginning of a pair —
            // what it becomes, [`CapturePair::step`] decides.
            Capture::Shift => CaptureStep::AwaitShift,
            Capture::Refused(refusal) => CaptureStep::Refuse(refusal),
        },

        // Task T-95-2: a release, and the end of the window of a pair, mean something only to the
        // memory of a pair ([`CapturePair::step`]); one press at a time, they are nothing.
        CaptureEvent::KeyUp(_) | CaptureEvent::PairTimeout => CaptureStep::Ignore,

        CaptureEvent::FocusLost { to_capture_button } => {
            if to_capture_button {
                CaptureStep::Ignore
            } else {
                CaptureStep::Cancel
            }
        }

        CaptureEvent::ClickBeside => CaptureStep::Cancel,
    }
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

/// `Shift` in its three forms — the neutral code the queue reports for both keys and the two sided
/// ones a keyboard can send. The key of the double press (task T-95-2): left and right are one.
const SHIFT_KEYS: [u16; 3] = [VK_SHIFT.0, VK_LSHIFT.0, VK_RSHIFT.0];

/// The whole capture decision, as a function of the key and of what was held with it.
///
/// The order of the tests is not free:
///
/// * **FR-96 first**, exactly as `hook::keyboard_hook_proc` puts it first, and for the same
///   reason: the emergency combination is not a candidate hotkey under any circumstance. In
///   practice the callback has already ended the process by the time this could run — this is
///   the answer for the case where the hook is not installed at all;
/// * then the `Windows` key — pressed, or held under another key — which is neither a modifier
///   of this program's vocabulary nor a key an application ever gets;
/// * then a modifier on its own, which is a press the user has not finished making — except a
///   `Shift` with nothing else held, which since task T-95-2 begins a double press
///   ([`Capture::Shift`]);
/// * then a key that would type a character with what is held ([`Refusal::Text`]) and a key
///   typing itself needs ([`Refusal::Editing`]);
/// * and last the vocabulary of section 7, because a name that cannot be written to the file
///   is a hotkey that would not survive a restart.
///
/// ⭐ **A combination is taken since task T-93-2 — вопрос 157, устройство А.** `Ctrl`, `Alt` and
/// `Shift` held with a key make a hotkey of the key *and* the set (`Ctrl + F12`): the capture
/// understands what was pressed, with no switch to choose a way first (157.6).
///
/// **SEC-01, SEC-07.** The value that comes out is a *name of a key* — `F8`, `Pause`, `A` — and
/// the names of the modifiers held with it, built from the virtual-key code and the set and from
/// nothing else. No character is produced, nothing is translated through a keyboard layout, and
/// nothing here reaches the journal: the caller shows the name in the field and writes it to
/// `[hotkey]`, which is where section 7 keeps it anyway.
pub fn capture(vk: u16, modifiers: Modifiers) -> Capture {
    if vk == crate::hook::EMERGENCY_VK && modifiers.emergency() {
        return Capture::Refused(Refusal::Emergency);
    }

    if SYSTEM_RESERVED.contains(&vk) || modifiers.win {
        return Capture::Refused(Refusal::Reserved);
    }

    if MODIFIER_KEYS.contains(&vk) {
        // ⭐ Task T-95-2, вопрос 159: a `Shift` with nothing but itself held is the beginning of
        // a double press — 157.17 gives the double press to `Shift` alone. `Ctrl` and `Alt` on
        // their own, and a `Shift` pressed under either (`Ctrl+Shift`, `Alt+Shift` — the switches
        // of the layout), stay the refusal they were; its line names the way that is left
        // («дважды — только Shift»).
        if SHIFT_KEYS.contains(&vk) && !modifiers.ctrl && !modifiers.alt {
            return Capture::Shift;
        }

        return Capture::Refused(Refusal::Modifier);
    }

    let set = modifiers.set();

    // ⭐ Task Т-23-5, решение 82.6. Before this line a letter, a digit or an OEM key was taken
    // and the warning of FR-95 followed the assignment; the user asked for the warning to come
    // *instead* of it, so that a stray press cannot quietly become the hotkey. See
    // [`Refusal::Text`] for what did **not** change: a text key written into the file by hand
    // is still a hotkey, and FR-95 still suppresses it. Since task T-93-2 the set decides with
    // the key — `Ctrl+K` types nothing and is taken, `Shift+K` types a capital and is not
    // (157.10, вариант 1 until the owner's word).
    if types_a_character(vk, set) {
        return Capture::Refused(Refusal::Text);
    }

    // ⭐ Task T-36-5, finding Н9, решение 123.3 — the keys typing itself needs. `Delete` and
    // `Home` were taken until this line, and taking one cost the person that key in every
    // application (FR-95) with no warning anywhere. The list is the FR-10 table and is read from
    // where that table lives; see [`Refusal::Editing`].
    //
    // Below `is_text_key` because the two never overlap and the order of the two answers does
    // not matter — and above `key_name`, because six of these keys **do** have names and would
    // otherwise be accepted, while the four arrows would be refused as nameless, which is true
    // and useless.
    if crate::buffer::is_editing_key(vk) {
        return Capture::Refused(Refusal::Editing);
    }

    // The key under its name, and the set in the canonical order whatever order it was pressed
    // in — task T-93-2. No `Win` reaches this line, so the set always has a name.
    match Hotkey::from_binding(vk, set) {
        Some(hotkey) => Capture::Taken(hotkey),
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
/// [`crate::hook::set_hotkey`] — the interface this task is given for the hotkey — and
/// dropping publishes back exactly what stood there before, the key and its modifiers in one
/// store (task T-93-1). No press can equal a code no keyboard produces, so nothing is converted
/// and nothing is suppressed for as long as the capture is armed.
///
/// `hook::set_active(false)` is published as well, and it is deliberately **not** what the
/// suspension rests on.
///
/// ⛔ **What it does not do — task T-36-6, finding Н116, решение 123.1.** This paragraph used to
/// promise a second effect: that the flag «additionally stops the stroke reaching the typing
/// buffer», so nothing typed while a hotkey is being chosen goes into the buffer of FR-10. **The
/// program does not keep that promise, and the knowledge is worth more than the sentence was.**
/// `app::window_proc` re-publishes `tray.enabled()` into `hook::set_active` **after every message
/// the UI thread sees** (FR-90, FR-91 — a program the user has just suspended must stop swallowing
/// the hotkey at once), so the flag is back to `true` within one message of being cleared: moving
/// the mouse over the tray icon is enough. Nothing here can hold it down, and this task does not
/// try — the repair would need a flag of the capture's own inside `hook`, read by the callback,
/// which is a new branch on the path NFR-01…NFR-05 governs. The promise is therefore struck out
/// rather than rewritten; the second effect is in `DEFERRED.md`.
///
/// The hotkey code, by contrast, is published from `app::publish_configuration` alone, which runs
/// on «Применить» and at start-up and from no message, so it stays where a capture puts it. That
/// is why the suspension rests on it.
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
    /// `[hotkey]` as it stood when the capture was armed — what «отмена» puts back: the key and,
    /// since task T-93-1, the modifiers held with it.
    previous: Hotkey,
    /// The double press this capture is waiting for, if it is — task T-95-2. Born empty with the
    /// capture and gone with it, so a pair never outlives the capture it was begun in.
    pair: CapturePair,
    /// The hotkey the callback was comparing against when the capture was armed — the code and
    /// its modifier set, which `hook` publishes as one word (task T-93-1).
    ///
    /// ⚠ The only value this session remembers since task T-36-6. There used to be a second —
    /// `hook::is_active()` as it stood at arming — and it was put back in `Drop`; both halves went
    /// out with the promise above. Remembering it was worse than not: `app::window_proc`
    /// re-publishes `tray.enabled()` after every message, so the saved copy was stale within a
    /// message of being taken, and putting it back at the end of a capture wrote a value older
    /// than the one the program already had. What the flag must equal after a capture is
    /// `tray.enabled()`, and that is published by the procedure itself.
    published: (u16, u16),
}

impl CaptureSession {
    /// Arms a capture and suspends the hotkey for its duration.
    pub fn arm(previous: Hotkey) -> Self {
        let published = crate::hook::hotkey();

        crate::hook::set_hotkey(NO_HOTKEY_VK, 0);

        // ⚠ Task T-36-6 left this call exactly where it was and took away only the belief about
        // it: the flag is cleared, and `app::window_proc` publishes `tray.enabled()` back over it
        // at the next message it sees. Removing the call would be a change of behaviour, and this
        // task is the striking out of a promise, not a repair of one.
        crate::hook::set_active(false);

        Self {
            previous,
            pair: CapturePair::default(),
            published,
        }
    }

    /// The hotkey to go back to if the capture is cancelled — the key and its modifiers.
    pub fn previous(&self) -> &Hotkey {
        &self.previous
    }

    /// One event of this capture, decided with the memory of its pair — [`CapturePair::step`],
    /// task T-95-2.
    pub fn step(&mut self, event: CaptureEvent, time: u32) -> CaptureStep {
        self.pair.step(event, time)
    }

    /// Whether this capture has a pair open — the field says «Shift…» and the timer of the pair
    /// runs (task T-95-2).
    pub fn pair_open(&self) -> bool {
        self.pair.open()
    }

    /// The hotkey the callback will be given back when this capture ends — the code and its
    /// modifier set.
    pub fn published(&self) -> (u16, u16) {
        self.published
    }
}

impl Drop for CaptureSession {
    /// Gives the hotkey back — and **only** it, since task T-36-6: the code and its modifiers, in
    /// one store (task T-93-1).
    ///
    /// `hook::set_active` is not touched here: the value it must hold after a capture is
    /// `tray.enabled()`, which `app::window_proc` publishes after every message the UI thread
    /// sees, and writing back a copy taken at arming would put a staler value than the one the
    /// program already has.
    fn drop(&mut self) {
        let (vk, modifiers) = self.published;

        crate::hook::set_hotkey(vk, modifiers);
    }
}

/// Resource identifier of the dialog template in `app.rc`.
///
/// ⚠ Kept equal to `IDD_SETTINGS` there by hand, because a `.rc` file and a Rust file have no
/// shared header. A mismatch is not silent: `DialogBoxParamW` fails outright and the failure is
/// reported, which is the reason the number is far away from the icon identifiers.
pub const IDD_SETTINGS: u16 = 200;

// =========================================================================================
// The mirror — вопрос 97, task Т-30-2. Both windows of the program go through here.
// =========================================================================================

/// Byte offset of `exStyle` inside a `DLGTEMPLATEEX`: `dlgVer` (2) + `signature` (2) +
/// `helpID` (4).
///
/// `rc.exe` compiles a `DIALOGEX` statement — which both templates of this program are — into
/// the extended form, whose first two fields are the version and the signature `0xFFFF` that
/// tell it apart from the old `DLGTEMPLATE`. Both are checked before this offset is used, so a
/// template that is somehow the old form is left alone rather than patched at the wrong place.
const DLGTEMPLATEEX_EXSTYLE_AT: usize = 8;

/// The **modeless twin** of [`show_modal_dialog`] — task Т-32-3, FR-101.
///
/// Same template, same mirror, same fallbacks; `CreateDialogIndirectParamW` in place of
/// `DialogBoxIndirectParamW`, and the window handle answered instead of a modal result.
///
/// # Why a twin and not a flag
///
/// The two calls differ in what they *are*: one does not return until the window is gone and
/// the other returns at once. Everything a modal caller may do with the frame it is standing on
/// — keeping the state on it, borrowing it from the procedure — is exactly what a modeless
/// caller may not. Making that a boolean parameter would put the difference in an `if` and take
/// it out of the type; here it is in the name of the function, and `letters` cannot reach the
/// modal one by accident.
///
/// **What is shared is the half that matters**: the mirror. Both go through
/// [`compiled_template`] and [`mirror_template`], so a letter is born mirrored in Hebrew and in
/// Arabic by the very code the settings window is (§6.2, and the rule of ИТОГ-Э30 §9.2 — a
/// window that must mirror has to be *born* mirrored, because the children inherit the layout
/// at creation and there is no later moment).
///
/// ⚠ **The caller owns `param` for as long as the window lives**, which is the whole difference
/// from the modal call and the reason it is spelled out here: a state on the stack would be
/// gone before the first `WM_PAINT`. `letters` puts the state in a `Box`, hands over the raw
/// pointer, and frees it on `WM_NCDESTROY`.
///
/// `None` for every refusal (NFR-13) — no window is a poor answer and a wrong window is worse.
///
/// # Safety
///
/// `owner` is a live window of this thread, `proc` is the dialog procedure that reads `param`,
/// and `param` names something that outlives **the window**, not this call.
pub(crate) unsafe fn show_modeless_dialog(
    instance: HINSTANCE,
    template: u16,
    owner: HWND,
    proc: DLGPROC,
    param: LPARAM,
) -> Option<HWND> {
    let mirrored = ui_language().is_rtl();

    if let Some(mut copy) = compiled_template(instance, template)
        && (!mirrored || mirror_template(copy.as_mut_bytes()).is_some())
    {
        // Вопрос 143, задача T-83-3: the step of the font, by the one body both calls share.
        apply_font_step(owner, copy.as_mut_bytes());

        // SAFETY: `copy` is a live buffer of this frame holding a whole compiled template. The
        // dialog manager reads it while it builds the window and keeps no pointer into it —
        // the window exists by the time this returns, which is what makes a frame buffer
        // enough here as well as in the modal call. The other three arguments are the
        // caller's, under the contract above.
        let created = unsafe {
            CreateDialogIndirectParamW(Some(instance), copy.as_ptr(), Some(owner), proc, param)
        };

        if let Ok(window) = created {
            return Some(window);
        }

        crate::app::report_non_critical("CreateDialogIndirectParamW", &WinError::from_thread());
    }

    // The template could not be copied, or is not the extended form, or the manager refused the
    // copy: run the resource itself, unmirrored. An unmirrored window is a poor answer for a
    // Hebrew reader and a far better one than no window at all (NFR-13).
    //
    // SAFETY: as above, with the template named by its integer identifier in the
    // `MAKEINTRESOURCE` form — a value below 65536 carried inside the pointer, never
    // dereferenced as a string.
    let created = unsafe {
        CreateDialogParamW(
            Some(instance),
            resource_id(template),
            Some(owner),
            proc,
            param,
        )
    };

    created
        .inspect_err(|error| crate::app::report_non_critical("CreateDialogParamW", error))
        .ok()
}

/// Adds `WS_EX_LAYOUTRTL` to the `exStyle` of a compiled dialog template, in place — the pure
/// half of [`show_modal_dialog`], task Т-30-2.
///
/// Answers what the extended style was and what it became, or `None` for a buffer that is not a
/// `DLGTEMPLATEEX` — too short to hold the header, or carrying something other than the version
/// and signature that mark the extended form. That refusal is the whole reason this is a
/// function and not four lines at the call site: a `DLGTEMPLATE` of the **old** form keeps
/// `style` where the extended one keeps `signature`, so patching at this offset without looking
/// would quietly corrupt a window style instead of adding one.
///
/// Public so that a test can drive it over a template that is not this program's, which is the
/// only way to check the refusal at all: both templates in `app.rc` are `DIALOGEX`.
pub fn mirror_template(template: &mut [u8]) -> Option<(u32, u32)> {
    let at = DLGTEMPLATEEX_EXSTYLE_AT;

    // SEC-05: a length that came from somebody else is checked before it is indexed with.
    if template.len() < at + 4 {
        return None;
    }

    let version = u16::from_le_bytes([template[0], template[1]]);
    let signature = u16::from_le_bytes([template[2], template[3]]);

    if (version, signature) != (1, 0xFFFF) {
        return None;
    }

    let before = u32::from_le_bytes([
        template[at],
        template[at + 1],
        template[at + 2],
        template[at + 3],
    ]);
    let after = before | WS_EX_LAYOUTRTL.0;

    template[at..at + 4].copy_from_slice(&after.to_le_bytes());

    Some((before, after))
}

/// The compiled bytes of one dialog template of this module's own binary, copied out so that
/// they can be patched — task Т-30-2.
///
/// `None` for every refusal and for a template that is not the extended form: the caller then
/// runs the resource itself, unpatched (NFR-13).
/// A copy of a compiled dialog template in a buffer whose alignment is **declared** — task
/// T-42-8, finding Н31.
///
/// # Why a type of its own
///
/// `DialogBoxIndirectParamW` is handed a pointer to a `DLGTEMPLATE`, and the structure begins
/// with `u32` fields: the documented contract is that the address is aligned for them. A
/// `Vec<u8>` says nothing about its alignment — `align_of::<u8>()` is one — so the guarantee
/// rested on what the system allocator happens to do, which for the sizes involved is an
/// aligned block every time and is nowhere written down. One storey up, for the resource
/// itself, the same guarantee **is** stated plainly: the loader aligns resources. This closes
/// the gap between the two storeys.
///
/// The bytes live in a `Vec<u32>`, whose alignment is four by its element type, and the length
/// in bytes is carried beside it: a template is not a whole number of words, and the tail of
/// the last word is padding the dialog manager never reads (it goes by the template's own
/// field lengths, not by the buffer's).
///
/// ⚠ Nothing else changes: the mirroring lever of Э30 patches the same offset in the same
/// bytes (`mirror_template` takes a byte slice), and the length check of SEC-05 is the same
/// check on the same number.
struct AlignedTemplate {
    /// The bytes, four to a word — the alignment this type exists to declare.
    words: Vec<u32>,
    /// How many of those bytes are the template. The rest of the last word is padding.
    bytes: usize,
}

impl AlignedTemplate {
    /// Copies `bytes` into a four-byte-aligned buffer, padding the last word with zeroes.
    fn new(bytes: &[u8]) -> Self {
        let mut words = vec![0_u32; bytes.len().div_ceil(size_of::<u32>())];

        // SAFETY: the destination is `words.len() * 4` bytes long by construction, which is
        // `bytes.len()` rounded **up**, so the copy fits; `u32` has no padding and no invalid
        // bit pattern, so writing arbitrary bytes over it is defined.
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                words.as_mut_ptr().cast::<u8>(),
                bytes.len(),
            );
        }

        Self {
            words,
            bytes: bytes.len(),
        }
    }

    /// The template as bytes, for the mirroring lever.
    fn as_mut_bytes(&mut self) -> &mut [u8] {
        // SAFETY: the buffer holds `words.len() * 4` initialised bytes and `self.bytes` is not
        // greater than that; `u8` has a weaker alignment than `u32`, so the cast is sound.
        unsafe { std::slice::from_raw_parts_mut(self.words.as_mut_ptr().cast::<u8>(), self.bytes) }
    }

    /// The address `DialogBoxIndirectParamW` is handed — aligned for a `u32` by the type of the
    /// buffer, not by the habits of the allocator.
    fn as_ptr(&self) -> *const DLGTEMPLATE {
        self.words.as_ptr().cast()
    }
}

/// The compiled bytes of one dialog template of `instance`, copied into an aligned buffer so that
/// the mirror of task Т-30-2 can patch them — `None` when the copy cannot be made, and the dialog
/// then opens from the template in the image, unmirrored.
///
/// ⚠ **Task T-43-15, finding Н68: the two refusals that carry a code of Windows are journaled.**
/// `FindResourceW` and `LoadResource` used to answer `None` and nothing else, so a Hebrew or an
/// Arabic interface that opened laid out left to right left no trace of why. The other two ways
/// out — a null lock and a template shorter than its own header — are a reading of bytes, carry
/// no code, and are not NFR-13's business; they stay silent on purpose.
fn compiled_template(instance: HINSTANCE, template: u16) -> Option<AlignedTemplate> {
    // SAFETY: `instance` is this program's module, whose resources carry both templates; the
    // "name" is an integer identifier in the `MAKEINTRESOURCE` form, never dereferenced.
    let found = unsafe { FindResourceW(Some(instance.into()), resource_id(template), RT_DIALOG) };

    if found.is_invalid() {
        // Read before any other call of this thread can overwrite the last error.
        crate::app::report_non_critical("FindResourceW", &WinError::from_thread());
        return None;
    }

    // SAFETY: `found` is a resource of `instance`, just answered by `FindResourceW`.
    let size = unsafe { SizeofResource(Some(instance.into()), found) } as usize;
    // SAFETY: as above. A resource handle is not freed — the loader owns the image. NFR-13: the
    // crate turns a failure into an error, which is journaled before the `?` gives up — the shape
    // `string_from` has had for the same call since task T-06-4.
    let loaded = unsafe { LoadResource(Some(instance.into()), found) }
        .inspect_err(|error| crate::app::report_non_critical("LoadResource", error))
        .ok()?;
    // SAFETY: `loaded` was just answered for a resource of this image.
    let raw = unsafe { LockResource(loaded) };

    // A template shorter than its own header cannot be one, and the offset below would read
    // past it. SEC-05: a length that came from somebody else is checked before it is trusted.
    if raw.is_null() || size < DLGTEMPLATEEX_EXSTYLE_AT + 4 {
        return None;
    }

    // SAFETY: `raw` and `size` are the address and the length `LockResource` and
    // `SizeofResource` answered for this one resource, and the image outlives this frame.
    let bytes = unsafe { std::slice::from_raw_parts(raw.cast::<u8>(), size) };

    // Task T-42-8: into a buffer that **declares** the alignment `DialogBoxIndirectParamW`
    // needs, instead of a `Vec<u8>` that happened to have it.
    Some(AlignedTemplate::new(bytes))
}

// =========================================================================================
// Шрифт по экрану — вопрос 143, этап Э83. The copy of the template the mirror patches carries
// the size of the font as well, and that is the whole of the lever on the text of six windows.
// =========================================================================================

/// The steps of the font a window of this program may open at, in whole points, **largest
/// first** — решения 143.1 и 143.2. Ten is the template's own size (`FONT 10` since Э51) and the
/// ceiling: the rule makes the text smaller where the screen asks for it, never larger. Whole
/// points only, because `pointsize` of a `DLGTEMPLATEEX` is a `WORD`.
pub const FONT_STEPS: [u16; 5] = [10, 9, 8, 7, 6];

/// The air, in pixels, the tallest window keeps above **and** below inside the work area of its
/// monitor — решение 143.3, taken by the owner on the mock-up. The steps are coarse, so the air a
/// step leaves is usually more than this; this is the least the rule accepts.
pub const FONT_STEP_AIR: i32 = 12;

/// The smallest face, in pixels, the rule takes — решение 143.5: one pixel under the 12 px of
/// Windows' own text at 100 %. A candidate whose face comes out smaller is not considered.
pub const FONT_STEP_FLOOR: i32 = 11;

/// The first face of [`TALLEST_WINDOW_BY_FACE`] — the floor of the rule.
const TALLEST_FIRST_FACE: i32 = FONT_STEP_FLOOR;

/// **The tallest window of the program, in dialog units, face by face** — 11 to 40 px, решения
/// 143.1 и 143.11, задача T-83-2; with the feed live, задача T-98-2 (Э83-Б-1, решение 166.2).
///
/// The height of a window **in dialog units is not the same at every face**: the lines of the body
/// are 140 % of their own size, and the cell of a face rounds its own way, so the same words take
/// 390 units at 14 px and 407 at 11 px (greek «От автора»). One number for all faces would be the
/// largest and would push 150 % on a 1080p screen from 7 pt to 6 pt — развилка C4, 143.11 — so the
/// rule asks for the tallest window **at the face it is weighing**.
///
/// Measured, not reasoned: the six windows × fourteen languages at every face, each in every state
/// it is really in — «От автора» by the model of `layout_author` held against the live window, on
/// the first day and with the feed live, with the quiet line or with the switch of FR-102,
/// «Последние письма» in the row and the lines of the reading and of the version at their tallest;
/// «Последние письма» by the model of `layout_list` at full capacity, held against the live window;
/// «О программе» with the growth of its help; the three others by their templates. Since the feed
/// went live the tallest is «От автора» with the quiet line: the switch makes the window shorter,
/// and the reading with its date is what grew it. The test
/// `the_tallest_window_by_face_is_the_table_the_rule_counts_with` measures it again every run and
/// wants this table **exactly** — enough units at every face, and not one more.
///
/// ⭐ **28 px: 421 → 412 with task T-104-3** (решение 189.2): the next reading of the feed is a day
/// away and no longer fifteen, so the line of the reading says «1» and not «15», and in Greek at
/// this one face those two digits were what took the line onto a fourth row. Measured by the test
/// above; no face went up.
///
/// ⭐ **12 px: 399 → 389 with task T-108-0** (решение 202.5): the panel «Обратная связь» no longer
/// tells of five steps and the clipboard, and the shorter words took a row off the window that was
/// the tallest at this one face. Measured by the test above; no face went up.
///
/// ⭐⭐ **Stage E108, решение 202.40 (C6): this is the table of the fourteen languages of 1.0.0**, the
/// same numbers as after T-108-0, byte for byte — the look of the fourteen does not move. Tamil and
/// Malayalam made «От автора» a fifth taller than any window of the fourteen, and one table for all
/// would have taken a step of the font off every screen of every language; so the rule counts with
/// the table **of the language of the interface** ([`tallest_window_units_in`]): the fourteen and
/// every language whose windows fit here use this table, a language whose windows are taller
/// somewhere has its own, measured face by face ([`TALLER_LANGUAGES`]). Within a language the step
/// is one for every window (решение 143.1 stands).
const TALLEST_WINDOW_BY_FACE: [i32; 30] = [
    407, 389, 399, 390, 397, 387, 395, 400, 402, 396, // 11 … 20 px
    386, 399, 403, 404, 386, 396, 401, 412, 404, 392, // 21 … 30 px
    401, 395, 398, 401, 401, 408, 404, 404, 407, 404, // 31 … 40 px
];

/// **The languages whose own tallest window is taller than [`TALLEST_WINDOW_BY_FACE`] at some face**
/// — stage E108, решение 202.40 — each with its own table, measured by
/// `the_tallest_window_by_face_is_the_table_the_rule_counts_with` as the fourteen's is: the least
/// number of units that holds the tallest window of that language at each face, 11 … 40 px.
///
/// Measured with the thirty-two languages of 2026-10-29 (`scratchpad-E108\tallest-per-lang.log`):
/// Tamil and Malayalam a fifth taller («От автора» with the quiet line), Telugu and Kannada a few
/// units, Serbian and Uzbek in Cyrillic at some faces only — each table is that language's own
/// tallest, so at a face where it is shorter than the fourteen's the language may keep a larger step.
const TALLER_LANGUAGES: [(Language, [i32; 30]); 14] = [
    (
        Language::Te,
        [
            417, 399, 408, 381, 406, 380, 404, 409, 412, 405, // 11 … 20 px
            395, 408, 403, 413, 395, 405, 411, 412, 413, 401, // 21 … 30 px
            410, 395, 407, 401, 410, 418, 404, 414, 407, 413, // 31 … 40 px
        ],
    ),
    (
        Language::Ta,
        [
            507, 444, 494, 442, 492, 438, 489, 479, 467, 472, // 11 … 20 px
            446, 493, 456, 500, 454, 490, 481, 499, 483, 463, // 21 … 30 px
            479, 472, 476, 470, 497, 489, 475, 484, 487, 483, // 31 … 40 px
        ],
    ),
    (
        Language::Kn,
        [
            407, 408, 399, 399, 397, 396, 395, 400, 412, 396, // 11 … 20 px
            395, 399, 403, 404, 395, 396, 401, 403, 404, 401, // 21 … 30 px
            401, 395, 398, 401, 401, 408, 404, 404, 407, 404, // 31 … 40 px
        ],
    ),
    (
        Language::Ml,
        [
            472, 473, 460, 460, 458, 456, 456, 462, 476, 456, // 11 … 20 px
            454, 460, 465, 466, 454, 457, 464, 474, 466, 463, // 21 … 30 px
            462, 456, 459, 462, 463, 472, 466, 467, 470, 466, // 31 … 40 px
        ],
    ),
    (
        Language::UzCyrl,
        [
            406, 399, 398, 390, 397, 387, 395, 391, 402, 387, // 11 … 20 px
            386, 398, 394, 403, 386, 396, 392, 402, 395, 392, // 21 … 30 px
            392, 387, 397, 392, 400, 399, 394, 395, 398, 394, // 31 … 40 px
        ],
    ),
    (
        Language::Sr,
        [
            412, 380, 380, 380, 396, 380, 394, 382, 383, 380, // 11 … 20 px
            380, 389, 380, 394, 380, 387, 383, 402, 386, 380, // 21 … 30 px
            391, 380, 388, 380, 391, 390, 380, 386, 380, 393, // 31 … 40 px
        ],
    ),
    // Wave 2, with forty languages (`scratchpad-E108\tallest-40.log`).
    (
        Language::Mk,
        [
            413, 399, 407, 390, 405, 387, 403, 391, 402, 404, // 11 … 20 px
            386, 407, 394, 412, 403, 405, 392, 410, 412, 392, // 21 … 30 px
            409, 387, 406, 392, 409, 416, 394, 412, 398, 411, // 31 … 40 px
        ],
    ),
    // Wave 2, with all forty-four languages (`scratchpad-E108\tallest-44.log`).
    (
        Language::Km,
        [
            397, 399, 390, 390, 389, 387, 387, 391, 402, 387, // 11 … 20 px
            386, 390, 394, 395, 386, 388, 392, 394, 395, 392, // 21 … 30 px
            392, 387, 389, 392, 392, 399, 394, 395, 398, 394, // 31 … 40 px
        ],
    ),
    (
        Language::Hy,
        [
            417, 418, 416, 408, 414, 404, 412, 409, 421, 405, // 11 … 20 px
            403, 408, 412, 421, 403, 405, 411, 429, 413, 410, // 21 … 30 px
            410, 404, 407, 410, 410, 418, 413, 414, 416, 413, // 31 … 40 px
        ],
    ),
    (
        Language::Ka,
        [
            417, 408, 408, 399, 406, 404, 404, 409, 421, 405, // 11 … 20 px
            403, 408, 412, 413, 403, 405, 411, 421, 413, 410, // 21 … 30 px
            410, 404, 407, 410, 410, 418, 413, 414, 416, 413, // 31 … 40 px
        ],
    ),
    // Stage E109, group B, with fifty-eight languages (`scratchpad-E109\t109-3-w5-testing.log`):
    // Dutch at 12 px only, Luxembourgish at most faces — «От автора», the quiet line.
    (
        Language::Nl,
        [
            406, 399, 390, 390, 389, 387, 387, 391, 402, 387, // 11 … 20 px
            386, 390, 394, 395, 386, 388, 392, 394, 395, 392, // 21 … 30 px
            392, 387, 389, 392, 392, 399, 394, 395, 398, 394, // 31 … 40 px
        ],
    ),
    (
        Language::Lb,
        [
            416, 408, 407, 390, 405, 396, 403, 400, 412, 396, // 11 … 20 px
            395, 399, 403, 412, 395, 404, 401, 411, 404, 401, // 21 … 30 px
            401, 395, 398, 401, 409, 408, 404, 404, 407, 404, // 31 … 40 px
        ],
    ),
    // With sixty languages (`scratchpad-E109\t109-3-w6-testing.log`): Welsh at 12 px and at a few
    // faces above.
    (
        Language::Cy,
        [
            407, 408, 399, 399, 397, 396, 395, 400, 412, 396, // 11 … 20 px
            395, 399, 403, 404, 395, 396, 401, 403, 404, 401, // 21 … 30 px
            401, 395, 398, 401, 401, 408, 404, 404, 407, 404, // 31 … 40 px
        ],
    ),
    // With all sixty-six (`scratchpad-E109\t109-3-w7-testing.log`): Maori at most faces.
    (
        Language::Mi,
        [
            432, 408, 407, 407, 405, 396, 420, 408, 412, 404, // 11 … 20 px
            403, 416, 411, 412, 403, 404, 409, 429, 421, 409, // 21 … 30 px
            409, 403, 406, 409, 409, 426, 412, 412, 415, 412, // 31 … 40 px
        ],
    ),
];

/// The table of the tallest window `language` counts with — its own, or the fourteen's.
fn tallest_table_of(language: Language) -> &'static [i32; 30] {
    TALLER_LANGUAGES
        .iter()
        .find(|(of, _)| *of == language)
        .map_or(&TALLEST_WINDOW_BY_FACE, |(_, table)| table)
}

/// Whether `language` counts with a table of its own and not the fourteen's — stage E108.
pub fn has_own_tallest_table(language: Language) -> bool {
    TALLER_LANGUAGES.iter().any(|(of, _)| *of == language)
}

/// The height, in dialog units, of the tallest window of the program **in `language`** at a dialog
/// face of `face` pixels — its table ([`tallest_table_of`]), and the table's own maximum beyond
/// it: a face the table does not know is counted as the tallest the table knows, never shorter.
pub fn tallest_window_units_in(language: Language, face: i32) -> i32 {
    let table = tallest_table_of(language);

    usize::try_from(face - TALLEST_FIRST_FACE)
        .ok()
        .and_then(|index| table.get(index).copied())
        .unwrap_or_else(|| table.iter().copied().max().unwrap_or(0))
}

/// [`tallest_window_units_in`] the language of the interface in force — what the rule of the
/// step counts with when a window opens ([`font_step_for`]).
pub fn tallest_window_units(face: i32) -> i32 {
    tallest_window_units_in(ui_language().in_force(), face)
}

/// **The rule of the step — решение 143.1**, the pure half of задача T-83-2: the step the six
/// windows open at on a monitor whose work area is `work_area` pixels tall, whose DPI is `dpi`,
/// and whose windows of this style cost `frame` pixels besides their client area.
///
/// The candidates go from ten down ([`FONT_STEPS`]); a candidate whose face is smaller than
/// [`FONT_STEP_FLOOR`] is not considered. The answer is the **first** candidate for which the
/// tallest window of the program at that candidate's face, with its frame and [`FONT_STEP_AIR`]
/// above and below, fits the work area:
///
/// `MulDiv(tallest(face), cell(face), 8) + frame + 2 × air ≤ work_area`
///
/// where `tallest` answers the height of that window in **dialog units** and `cell` the `tmHeight`
/// of the face — the vertical base unit of a dialog set in it, asked of GDI. When no candidate
/// fits, the answer is the **floor**, the smallest candidate still allowed, and the settings window
/// scrolls as it has since Т-42-12. A monitor on which even ten points come out under the floor —
/// none that Windows makes — keeps ten: the rule never answers larger than the template.
///
/// **The key is the screen, not the percentage** (143.1): a 2560 × 1440 at 125 % keeps ten, a
/// 1920 × 1080 at 125 % takes eight.
pub fn font_step(
    work_area: i32,
    dpi: i32,
    frame: i32,
    tallest: impl Fn(i32) -> i32,
    cell: impl Fn(i32) -> i32,
) -> u16 {
    let mut floor = None;

    for step in FONT_STEPS {
        let face = theme::face_at(step, dpi);

        if face < FONT_STEP_FLOOR {
            continue;
        }

        floor = Some(step);

        if theme::mul_div_round(tallest(face), cell(face), 8) + frame + 2 * FONT_STEP_AIR
            <= work_area
        {
            return step;
        }
    }

    floor.unwrap_or(FONT_STEPS[0])
}

/// The `DS_SETFONT` bit of a template's `style` — a `DLGTEMPLATEEX` carries its font block only
/// when this bit (or `DS_SHELLFONT`, which contains it) is set.
const DS_SETFONT_BIT: u32 = 0x40;

/// The length of the fixed header of a `DLGTEMPLATEEX`: version, signature, help identifier,
/// extended style, style, the count of items and the four coordinates — 2 + 2 + 4 + 4 + 4 + 2 +
/// 2 × 4 bytes.
const DLGTEMPLATEEX_HEADER: usize = 26;

/// Where the `pointsize` of a compiled `DLGTEMPLATEEX` lies — задача T-83-3, the pure half.
///
/// The font block is not at a fixed offset: it follows the menu, the window class and the title,
/// and the first two are each a `sz_Or_Ord` — a zero word for «none», `0xFFFF` and an ordinal, or
/// a string ended by a zero word — and the title is a string. So the offset is walked, field by
/// field, the way the dialog manager walks it. The six templates of this program answer 82, 54,
/// 64, 64, 50 and 62 (measured, `scratchpad-E83\probe-premises-e83.log`).
///
/// `None` for a buffer that is not the extended form, that ends inside a field, or whose style
/// asks for no font — **a template without `DS_SETFONT` is not patched**, it has no size to patch.
/// SEC-05: every length here came from a resource, and each is checked before it is read.
pub fn template_point_size_at(template: &[u8]) -> Option<usize> {
    let word = |at: usize| -> Option<u16> {
        Some(u16::from_le_bytes([
            *template.get(at)?,
            *template.get(at + 1)?,
        ]))
    };

    // A string of `WCHAR`s ended by a zero word: the offset right after its terminator.
    let after_string = |mut at: usize| -> Option<usize> {
        while word(at)? != 0 {
            at += 2;
        }

        Some(at + 2)
    };

    if (word(0)?, word(2)?) != (1, 0xFFFF) {
        return None;
    }

    let style = u32::from_le_bytes(template.get(12..16)?.try_into().ok()?);

    if style & DS_SETFONT_BIT == 0 {
        return None;
    }

    let mut at = DLGTEMPLATEEX_HEADER;

    // The menu and the window class: none, an ordinal, or a string.
    for _ in 0..2 {
        at = match word(at)? {
            0 => at + 2,
            0xFFFF => at + 4,
            _ => after_string(at)?,
        };
    }

    // The title is always a string, the empty one included.
    at = after_string(at)?;

    // The size itself has to be inside the buffer before its offset is worth anything.
    word(at)?;

    Some(at)
}

/// Sets the `pointsize` of a compiled `DLGTEMPLATEEX` in place and answers what it was —
/// задача T-83-3. `None`, and not a byte changed, for every template
/// [`template_point_size_at`] refuses.
pub fn set_template_point_size(template: &mut [u8], points: u16) -> Option<u16> {
    let at = template_point_size_at(template)?;
    let before = u16::from_le_bytes([template[at], template[at + 1]]);

    template[at..at + 2].copy_from_slice(&points.to_le_bytes());

    Some(before)
}

/// The step the next window of this thread is to open at when it is **forced** — the debug
/// substitution of задача T-83-3, and nothing a Release build carries.
///
/// Two doors, both absent from the shipped binary (criterion 8 of §13):
/// * the `testing` feature's [`testing::with_font_step`], for the measuring tests — a value of the
///   thread, so tests running side by side cannot force each other's windows;
/// * a debug build's `LANGSW_DEBUG_FONT_STEP` (6…15), read once — the FR-97 pattern, for the eye:
///   a debug build at 100 % opens its windows at 8 pt and shows what 125 % will look like.
fn forced_font_step() -> Option<u16> {
    #[cfg(feature = "testing")]
    if let Some(step) = testing::forced() {
        return Some(step);
    }

    #[cfg(debug_assertions)]
    if let Some(step) = debug_font_step::configured() {
        return Some(step);
    }

    None
}

/// The debug substitution of the step — задача T-83-3, compiled into debug builds only, the way
/// FR-97's deadline is (`app::debug_timeout`): absent from the Release build, not disabled in it.
#[cfg(debug_assertions)]
mod debug_font_step {
    use std::sync::OnceLock;

    /// Environment variable that forces the step of every window, in whole points — 6 to 15.
    const FONT_STEP_ENV_VAR: &str = "LANGSW_DEBUG_FONT_STEP";

    /// The forced step, read once; `None` when the variable is absent or not a step.
    pub(super) fn configured() -> Option<u16> {
        static STEP: OnceLock<Option<u16>> = OnceLock::new();

        *STEP.get_or_init(|| {
            std::env::var(FONT_STEP_ENV_VAR)
                .ok()?
                .trim()
                .parse::<u16>()
                .ok()
                .filter(|step| (6..=15).contains(step))
        })
    }
}

/// The measuring doors of stage Э83 on the side of this module — compiled only under the
/// `testing` feature (SEC-04a; criterion 8 finds no trace of it in the Release build).
#[cfg(feature = "testing")]
pub mod testing {
    use std::cell::Cell;

    thread_local! {
        /// The step forced on the windows this thread opens, or zero for none.
        static FORCED: Cell<u16> = const { Cell::new(0) };
    }

    /// The forced step, if one is in force on this thread.
    pub(super) fn forced() -> Option<u16> {
        Some(FORCED.with(Cell::get)).filter(|step| *step != 0)
    }

    /// Runs `body` with every window this thread opens forced to `step` points — the debug
    /// substitution of задача T-83-3, for the measuring tests. The step before is put back
    /// however `body` leaves.
    pub fn with_font_step<R>(step: u16, body: impl FnOnce() -> R) -> R {
        struct Restore(u16);

        impl Drop for Restore {
            fn drop(&mut self) {
                FORCED.with(|forced| forced.set(self.0));
            }
        }

        let _restore = Restore(FORCED.with(|forced| forced.replace(step)));

        body()
    }
}

/// **The step of the font of the next window of `owner`, put into the copy of its template** —
/// задача T-83-3, **the one body** both [`show_modal_dialog`] and [`show_modeless_dialog`] call, so
/// the six windows cannot come to disagree about their size.
fn apply_font_step(owner: HWND, template: &mut [u8]) {
    let step = font_step_for(owner, template);

    put_font_step(template, step);
}

/// Puts `step` into the copy of a template as its `pointsize` — задача T-83-3.
///
/// ⚠ **At the step of the template itself nothing is written**: the copy stays equal to the
/// resource byte for byte, which is what «при 100 % не меняется ни один пиксель» rests on.
///
/// Pure, and public so the test calls the very function the windows call.
pub fn put_font_step(template: &mut [u8], step: u16) {
    let Some(at) = template_point_size_at(template) else {
        return;
    };

    if u16::from_le_bytes([template[at], template[at + 1]]) != step {
        let _ = set_template_point_size(template, step);
    }
}

/// The face a compiled `DLGTEMPLATEEX` asks for — its typeface, weight, italic and charset, as the
/// dialog manager hands them to `CreateFontIndirectW` — with `lfHeight` left for the caller.
/// `None` for every template [`template_point_size_at`] refuses. Pure; SEC-05 as there.
pub fn template_face(template: &[u8]) -> Option<LOGFONTW> {
    let at = template_point_size_at(template)?;
    let weight = u16::from_le_bytes([*template.get(at + 2)?, *template.get(at + 3)?]);
    let italic = *template.get(at + 4)?;
    let charset = *template.get(at + 5)?;

    let mut face = LOGFONTW {
        lfWeight: i32::from(weight),
        lfItalic: italic,
        lfCharSet: windows::Win32::Graphics::Gdi::FONT_CHARSET(charset),
        ..Default::default()
    };

    // The typeface: `WCHAR`s ended by a zero word, no longer than `LF_FACESIZE` less its NUL.
    let mut name = at + 6;
    let mut written = 0;

    loop {
        let unit = u16::from_le_bytes([*template.get(name)?, *template.get(name + 1)?]);

        if unit == 0 {
            break;
        }

        if written + 1 >= face.lfFaceName.len() {
            return None;
        }

        face.lfFaceName[written] = unit;
        written += 1;
        name += 2;
    }

    Some(face)
}

/// The cell — `tmHeight`, the vertical base unit of a dialog set in it — of `face` at `pixels`,
/// asked of GDI on a memory DC. Zero on a refusal (NFR-13), which the rule reads as «fits» and
/// answers the template's own ten with: the look of the program before this stage.
///
/// ⚠ **A question to GDI, like the probe of [`resolve_emphasis`]**: the face is made here, measured
/// and freed here, and never handed to a window. The rule of вопрос 143 needs the real `tmHeight`
/// of each candidate face — Segoe UI rounds its cell its own way (13 px → 17, 14 → 19, 15 → 20) —
/// and nothing but GDI knows it.
fn cell_of_face(face: LOGFONTW, pixels: i32) -> i32 {
    let logical = LOGFONTW {
        lfHeight: -pixels,
        ..face
    };

    // SAFETY: a memory DC of this process; deleted below on every path.
    let dc = unsafe { CreateCompatibleDC(None) };

    if dc.is_invalid() {
        return 0;
    }

    let Some(probe) = create_font(logical) else {
        // SAFETY: deletes exactly the DC created above, once.
        let _ = unsafe { DeleteDC(dc) };
        return 0;
    };

    // SAFETY: `dc` and `probe` are live and ours; the previous face is put back below.
    let previous = unsafe { SelectObject(dc, probe.into()) };
    let mut metrics = TEXTMETRICW::default();
    // SAFETY: `metrics` is a live local the call fills.
    let read = unsafe { GetTextMetricsW(dc, &mut metrics) }.as_bool();

    // SAFETY: puts back exactly what `SelectObject` answered, then frees the probe and the DC, each
    // once — the probe is selected out before it is deleted.
    unsafe {
        SelectObject(dc, previous);
        let _ = DeleteObject(probe.into());
        let _ = DeleteDC(dc);
    }

    if read { metrics.tmHeight } else { 0 }
}

/// What the rule has to know about the monitor a window of `owner` will open on, taken **before**
/// the window exists — задача T-83-3, посылка П5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    /// The height of the work area — the monitor less the task bar — in pixels.
    pub work_area: i32,
    /// The DPI of the monitor.
    pub dpi: i32,
    /// What a window of the template's style costs besides its client area there: the caption and
    /// the two borders, in pixels.
    pub frame: i32,
}

/// **The monitor `DS_CENTER` puts a window of `owner` on — its work area, its DPI and the frame of
/// the template's style there** — задача T-83-3, посылка П5.
///
/// The monitor is the owner's: `DS_CENTER` centres a dialog in the work area of the monitor that
/// contains its owner (measured on the owner shaped like `LangSwitcher.Hidden`,
/// `scratchpad-E83\probe-premises-e83.log`).
///
/// ⛔ **The DPI and the frame are not computed, they are asked of a window.** `GetDpiForMonitor`
/// and `AdjustWindowRectExForDpi` live in `Win32_UI_HiDpi`, which is not in the closed list of
/// features (SEC-03), and scaling the 39 px of 100 % would be wrong: 59 against the 56 of 150 %,
/// three of the four pixels 7 pt has there. So a window of the template's own styles is made
/// **hidden** on that monitor — a top-level window of a `PerMonitorV2` process takes the DPI of the
/// monitor it is made on — its frame is read the way `fit_window_to_work_area` reads the dialog's
/// (`GetWindowRect` less `GetClientRect`; 39 px at 96 DPI, the dialog's own, measured) and the DPI
/// off its DC; then it is destroyed. It is never shown, takes no focus, and lives for a few calls.
///
/// NFR-13: any refusal answers `None`, and the window opens at the template's own ten points.
///
/// Public so the test measures the very function the windows ask.
pub fn screen_of(owner: HWND, template: &[u8]) -> Option<Screen> {
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DS_MODALFRAME, DestroyWindow, WINDOW_EX_STYLE, WINDOW_STYLE,
        WS_EX_DLGMODALFRAME, WS_VISIBLE,
    };

    let style = u32::from_le_bytes(template.get(12..16)?.try_into().ok()?);
    let ex_style = u32::from_le_bytes(template.get(8..12)?.try_into().ok()?);

    // SAFETY: `owner` is a live window of this thread; the call reads a system table.
    let monitor = unsafe { MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST) };

    let mut info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };

    // SAFETY: the handle came from the call above and the buffer is a live local whose `cbSize`
    // the call reads first, as it documents.
    if !unsafe { GetMonitorInfoW(monitor, &raw mut info) }.as_bool() {
        return None;
    }

    let work = info.rcWork;

    // The window part of the template's style — the low word is the dialog's own (`DS_*`) and
    // means something else to a static — never visible; and the frame the dialog manager gives a
    // `DS_MODALFRAME` template.
    let window_style = WINDOW_STYLE(style & 0xFFFF_0000 & !WS_VISIBLE.0);
    let mut window_ex_style = WINDOW_EX_STYLE(ex_style);

    if style & u32::try_from(DS_MODALFRAME).unwrap_or(0) != 0 {
        window_ex_style |= WS_EX_DLGMODALFRAME;
    }

    // SAFETY: a system class, no parent, no menu, no pointer of ours; the rectangle is inside the
    // work area of the monitor, so the window is made on it. Destroyed below on every path.
    //
    // NFR-13, examined in words: a refusal costs the step — the window opens at the template's own
    // ten points, the look of before this stage — and is deliberately not journaled: the
    // vocabulary of `diag` is a closed table, and a cosmetic fallback does not widen it
    // (reviews\T-11-1.md, the reason `Brushes::new` gives).
    let probe = unsafe {
        CreateWindowExW(
            window_ex_style,
            w!("STATIC"),
            PCWSTR::null(),
            window_style,
            (work.left + work.right) / 2 - 50,
            (work.top + work.bottom) / 2 - 50,
            100,
            100,
            None,
            None,
            None,
            None,
        )
    }
    .ok()?;

    let mut window = RECT::default();
    let mut client = RECT::default();

    // SAFETY: `probe` is the live window just made; the buffers are live locals of this frame.
    let read = unsafe { GetWindowRect(probe, &raw mut window) }.is_ok()
        && unsafe { GetClientRect(probe, &raw mut client) }.is_ok();

    // SAFETY: `probe` is live; the DC is released right after the one question asked of it.
    let dpi = unsafe {
        let dc = GetDC(Some(probe));
        let dpi = if dc.is_invalid() { 0 } else { dc_dpi(dc) };
        ReleaseDC(Some(probe), dc);
        dpi
    };

    // SAFETY: the window was made above by this thread and is destroyed once, here.
    let _ = unsafe { DestroyWindow(probe) };

    if !read || dpi <= 0 {
        return None;
    }

    Some(Screen {
        work_area: work.bottom - work.top,
        dpi,
        frame: (window.bottom - window.top) - (client.bottom - client.top),
    })
}

/// **The step of the font the next window of `owner` is to open at** — задача T-83-3, the one
/// function both `show_*` ask, from the same inputs, so the windows of one monitor share one step
/// (решение Э51 «один кегль» stands).
///
/// The forced step of the debug substitution first ([`forced_font_step`], absent from Release);
/// otherwise [`font_step`] over the monitor [`screen_of`] measures, the tallest window
/// [`tallest_window_units`] names at each face, and the cell of the template's own face at it.
/// Any refusal on the way answers the template's own ten points (NFR-13).
fn font_step_for(owner: HWND, template: &[u8]) -> u16 {
    if let Some(step) = forced_font_step() {
        return step;
    }

    let (Some(screen), Some(face)) = (screen_of(owner, template), template_face(template)) else {
        return FONT_STEPS[0];
    };

    font_step(
        screen.work_area,
        screen.dpi,
        screen.frame,
        tallest_window_units,
        |pixels| cell_of_face(face, pixels),
    )
}

const IDC_LANGUAGE: i32 = 1002;
const IDC_THEME: i32 = 1003;

const IDC_HOTKEY: i32 = 1010;
const IDC_PAIR_SOURCE: i32 = 1022;
const IDC_PAIR_TARGET: i32 = 1023;
const IDC_CYCLE_LIST: i32 = 1024;
const IDC_EXCLUSIONS: i32 = 1050;
const IDC_EXCLUSION_NAME: i32 = 1051;

// The static text of the dialog — group boxes and labels. They carried -1 until FR-94 needed
// to replace their text, and a control identified by -1 is a control `GetDlgItem` cannot find.
const IDC_GROUP_GENERAL: i32 = 1090;
// ⚠ **1092 is retired, not free** — task Т-31-3, решение 99.4. It was `IDC_LANGUAGE_RESTART`,
// the two-line static «вступит в силу после перезапуска», and it is gone from the template with
// the sentence it carried. The place right of the language combo stays empty: решение 87
// accepted these coordinates by eye and nothing else moves because a neighbour left.
const IDC_GROUP_HOTKEY: i32 = 1093;
const IDC_GROUP_LAYOUTS: i32 = 1095;
// 1099..1103 — the five statics of «Замена» and «Выделение», retired by task Т-23-2 together
// with the six controls above; see the note there.
const IDC_GROUP_EXCLUSIONS: i32 = 1104;
const IDC_GROUP_DIAGNOSTICS: i32 = 1106;
const IDC_GROUP_STATE: i32 = 1108;

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

/// What a change of the interface language asks of a settings window that is **already open** —
/// решение 99.1, task Т-31-2.
///
/// Три исхода, и они не про язык, а про направление письма: подписи меняются словами, а зеркало
/// ставится один раз, при создании окна (`WS_EX_LAYOUTRTL` в копии шаблона — task Т-30-2,
/// другого момента поставить стиль нет). Отсюда и два механизма, которых просил пользователь.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LanguageSwitch {
    /// Язык не менялся: окну не нужно ничего.
    #[default]
    Unchanged,
    /// Язык другой, направление письма то же — окно **перезаполняется на месте**, тем же телом,
    /// которым заполняется при создании ([`relabel_dialog`]).
    Relabel,
    /// Направление письма или шаг шрифта сменились — окно **пересоздаётся** на том же месте, в своём
    /// размере ([`reopened_window`]).
    Reopen,
}

/// Какой из двух механизмов нужен при переходе `old` → `new` — чистая функция, решение 99.1.
///
/// Публичная, как пара [`theme_combo_index`] / [`theme_from_combo_index`] рядом, и по той же
/// причине: тест обязан звать **ту самую** функцию, которую зовёт диалог, а не её копию.
/// Единственное место в программе, где написано, отчего зависит выбор механизма.
///
/// ⭐ **Stage E108, решение 202.40 (в): the step of the font goes by the language too** — a language
/// with a table of the tallest window of its own opens at the step that table gives
/// ([`tallest_window_units_in`]), and the step is set when a window is created, as the mirror is.
/// So a change between two tables is a change that reopens, whatever the direction: a window left
/// relabelled would stay at the step of the old language.
pub fn language_switch(old: Language, new: Language) -> LanguageSwitch {
    if old == new {
        return LanguageSwitch::Unchanged;
    }

    let same_table = std::ptr::eq(
        tallest_table_of(old.in_force()),
        tallest_table_of(new.in_force()),
    );

    if old.is_rtl() == new.is_rtl() && same_table {
        LanguageSwitch::Relabel
    } else {
        LanguageSwitch::Reopen
    }
}

/// `IDOK` as the number the dialog manager sends in `WM_COMMAND`.
///
/// The crate's metadata types the constant as `MESSAGEBOX_RESULT`, because that is one of the
/// places it appears; what arrives in a `WM_COMMAND` is the plain number, and it is the plain
/// number that is matched on.
pub(crate) const OK_COMMAND: i32 = IDOK.0;

// `MS_FIELD_DIGITS` — nine, the digits a millisecond field accepted — stood here until task
// Т-23-2. Решение 81 took all three millisecond fields off the window, so there is no field
// left to limit; the numbers are typed into `config.toml` by hand and bounded where they
// have always really been bounded, by the ceilings of task T-13-13 in `publish_configuration`.

/// The most bytes of UTF-8 that one **UTF-16 code unit** can become.
///
/// Three, and the number is not the familiar four: four bytes of UTF-8 are a character outside the
/// basic plane, and such a character is **two** UTF-16 units — two bytes per unit. The worst case
/// per unit is a BMP character from `U+0800` to `U+FFFF`, which is one unit and three bytes. See
/// [`EXCLUSION_NAME_CHARS`], which is the one place this matters.
const MAX_UTF8_PER_UTF16_UNIT: usize = 3;

/// Characters the exclusion name field accepts — **UTF-16 units**, which is what `EM_LIMITTEXT`
/// counts.
///
/// ⭐ **Task Т-22-8, finding м9 of the audit of 2026-09-01.** The number used to be sixty-four,
/// chosen by hand, and the comment beside it said "the worst case is two bytes per character".
/// That is the worst case for the characters people usually have in mind and not for the unit the
/// field actually counts: sixty-four units of `U+0800..U+FFFF` are **192 bytes** against the 128
/// of [`crate::guard::MAX_EXCLUSION_NAME_BYTES`]. A name of forty-three such characters therefore
/// passed the field, went into the list, was saved to `config.toml` — and was dropped by
/// [`crate::guard::publish_exclusions`], which refuses a name over its ceiling rather than
/// truncating it. The only trace was a number in `exclusions_refused`. The user saw the name in
/// the list they had just applied and the program went on recording in the process they had
/// excluded.
///
/// The ceiling is now **derived** from the one `guard` publishes, and the derivation is asserted
/// below rather than trusted: whatever a name the field accepts is made of, it cannot come out
/// over `guard`'s ceiling, so `guard` has nothing to refuse silently.
///
/// The price is names of 43 to 64 characters, which the field no longer accepts. That is a
/// deliberate trade of the option `guard` never had: a Windows process name — which is what FR-84
/// matches on, an executable file name after [`crate::guard::fold_process_name`] — has no
/// business being longer than forty-two characters, and a program that quietly ignored the
/// exclusion was the worse of the two answers.
pub const EXCLUSION_NAME_CHARS: usize =
    crate::guard::MAX_EXCLUSION_NAME_BYTES / MAX_UTF8_PER_UTF16_UNIT;

/// **The invariant of task Т-22-8, checked at compile time**: a name the field accepts is never
/// refused by `guard`.
///
/// Written out rather than left to the division above, so that an edit of either ceiling — this
/// one, [`MAX_UTF8_PER_UTF16_UNIT`], or [`crate::guard::MAX_EXCLUSION_NAME_BYTES`] — has to meet
/// the rule instead of quietly re-opening the gap. The same device, for the same reason, as the
/// two `const _: () = assert!` lines in `src\guard.rs`.
const _: () = assert!(
    EXCLUSION_NAME_CHARS * MAX_UTF8_PER_UTF16_UNIT <= crate::guard::MAX_EXCLUSION_NAME_BYTES
);

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

/// What «ОК» and «Применить» write into `[layouts] cycle` — task T-55-8, findings С54 and С55.
///
/// **The string of the file goes back byte for byte unless its ticks were touched.**
///
/// * **С54 — an empty session is not an empty cycle.** The list is built from the layouts the
///   session names right now ([`layout_rows`]); a session that named nothing — the enumeration
///   refused, a layout was being reinstalled — gave an empty list, and «ОК» without a single edit
///   wrote an empty cycle over the file's. With no session there is nothing to tick, so the cycle
///   of the file stands whatever else happens. The section of the layouts already shows that
///   moment: both layouts of the pair are then not in the session either, and the note under them
///   says so — no string of its own was needed for it.
/// * **С55 — a tick is not a rewrite.** `0x00000409` names every layout of the language 0x0409;
///   with two of them in the session the list ticks both, and «ОК» wrote both back in the handle
///   form: the cycle grew, its order of FR-31 changed, and the short form never came back.
///
/// **One mechanism for both, chosen over the other one the task allowed.** The task offered two:
/// tick only the layout the resolution of the write would pick, or keep the string whose ticks
/// nobody touched. The second is taken: it is the one of the two that keeps the file *byte for
/// byte* — the invariant the task names — it leaves the list showing exactly what FR-31 and
/// [`layout_rows`] show today, and it answers С54 with the same comparison.
///
/// **«Touched» is compared, not tracked:** the rows as the person leaves them against the rows as
/// the dialog opened. A tick taken back is no edit; a real edit — a tick, an untick, a move — is
/// written the way [`cycle_from_rows`] always wrote it; and two «ОК» in a row without edits give the
/// same cycle twice.
///
/// Pure, and public for the reason [`layout_rows`] is.
pub fn cycle_after_dialog(
    cycle_at_open: &[String],
    rows_at_open: &[LayoutRow],
    rows: &[LayoutRow],
    session: &[LayoutId],
) -> Vec<String> {
    if session.is_empty() || rows == rows_at_open {
        return cycle_at_open.to_vec();
    }

    cycle_from_rows(rows, session)
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

// `parse_ms` — the reader of a millisecond field, which turned the text of an `ES_NUMBER`
// edit back into a number and kept the old value for anything it refused — stood here until
// task Т-23-2. Решение 81 took the three fields it read off the window, and a parser with no
// field to parse is dead weight; the values are read out of `config.toml` by serde now, and
// bounded by the ceilings of task T-13-13.

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
pub fn hotkey_note(hotkey: &Hotkey) -> Option<u16> {
    match hotkey.binding() {
        None => Some(IDS_NOTE_UNKNOWN_KEY),
        Some((vk, modifiers)) if types_a_character(vk, modifiers) => Some(IDS_NOTE_TEXT_KEY),
        Some(_) => None,
    }
}

/// Whether `vk` pressed with `modifiers` types a character — task T-93-1, **157.10** (вариант 1
/// until the owner's word).
///
/// A text key ([`is_text_key`]) types with nothing held and with `Shift` — a capital, a symbol —
/// and with `Ctrl` and `Alt` together, which is `AltGr`: the program's own FR-10 records both as
/// text (`buffer::Recorder::record`, the command row stands aside for `AltGr`), and on many
/// layouts `AltGr` is where the letters of a language live (`ą`, `ę`, `ł`; `@` and `€`). With
/// `Ctrl` alone, `Alt` alone, or either of them with `Shift`, the key is a command and types
/// nothing — FR-10 throws such a stroke away as one.
fn types_a_character(vk: u16, modifiers: u16) -> bool {
    use crate::hook::{MOD_ALT, MOD_CTRL};

    let command = modifiers & (MOD_CTRL | MOD_ALT);

    is_text_key(vk) && (command == 0 || command == MOD_CTRL | MOD_ALT)
}

/// The name of the key the program **actually answers to** — task Т-23-4, решение 82.5.
///
/// `[hotkey] key` is a name in a text file and a person can write anything in it. A name this
/// build does not know leaves the default of section 7 in force — `app::publish_configuration`
/// says so and the settings dialog puts [`IDS_NOTE_UNKNOWN_KEY`] under the field — so a help
/// panel that repeated the file's word would name a key that does nothing. The same reader
/// [`hotkey_note`] asks is asked here, and the answer decides between the file's name and the
/// default that is really acting.
///
/// A *text* key is a different case and is deliberately **not** substituted: `Q` really is the
/// hotkey when the file says so, FR-95 warns about it in the settings window, and the help
/// telling a person the truth about their own configuration is the point.
///
/// Public so a test can call the very function the window calls.
///
/// ⭐ **One string for a combination — task T-93-1, вопрос 157.** `Ctrl + Alt + Shift + F12`: the
/// modifiers in the canonical order `Ctrl`, `Alt`, `Shift` whatever order the file lists them in,
/// [`HOTKEY_JOINER`] between, and the key under the name the program writes ([`key_name`]) — so a
/// hand-written `f12` or `Break` is shown as `F12` and `Pause`. This is the one function every
/// window shows the hotkey through: the field of the settings window, the help of «О программе»,
/// the letters and the chip of their demonstration (`letters.rs`).
///
/// ⭐ **The double press — task T-95-1, вопрос 159: «Shift, Shift»**, the key twice with
/// [`DOUBLE_TAP_JOINER`] between, in every language alike (the names of keys are not translated):
/// the field after a capture and at rest, «О программе» and the letters show this one string.
pub fn effective_hotkey_name(hotkey: &Hotkey) -> String {
    let Some((vk, modifiers)) = hotkey.binding() else {
        return default_hotkey_key();
    };

    if modifiers & crate::hook::DOUBLE_TAP != 0 {
        return [DOUBLE_TAP_KEY, DOUBLE_TAP_KEY].join(DOUBLE_TAP_JOINER);
    }

    let key = key_name(vk).unwrap_or_else(|| hotkey.key.trim().to_owned());
    let mut name = modifier_label(modifiers);

    if !name.is_empty() {
        name.push_str(HOTKEY_JOINER);
    }

    name.push_str(&key);
    name
}

/// What stands between the parts of a hotkey's name — « + », task T-93-1 (the mock-up of 157.5).
pub const HOTKEY_JOINER: &str = " + ";

/// What stands between the two taps in the name of the double press — «, », task T-95-1 (the
/// mock-up of 157.18: «Shift, Shift»).
pub const DOUBLE_TAP_JOINER: &str = ", ";

/// The modifiers of a set, by name, in the canonical order and joined — `Ctrl + Alt` — or empty
/// for a bare key (task T-93-1). The head of [`effective_hotkey_name`].
pub fn modifier_label(modifiers: u16) -> String {
    crate::hook::HOTKEY_MODIFIERS
        .iter()
        .filter(|(_, bit)| modifiers & bit != 0)
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(HOTKEY_JOINER)
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
/// Why a settings window is being put on the screen — решение 99.1, task Т-31-2.
///
/// The ordinary opening and the one that follows a change of the direction of writing differ in
/// two visible ways, and both are the решение: where the window goes, and where the focus is.
// ⚠ No `Eq`: `RECT` of the crate derives `PartialEq` and not `Eq`, and a rectangle is what the
// second variant carries. Nothing here needs the stronger one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Opening {
    /// A person asked for the settings. `DS_CENTER` places the window and the dialog manager
    /// chooses the focus, exactly as they always have.
    #[default]
    Fresh,
    /// «Применить» changed the direction of writing, so the window is being built again.
    ///
    /// It sits down where the previous one stood, in its own size ([`reopened_window`]) — `Some`,
    /// the rectangle of the previous one and the ordinary case —
    /// and the focus goes on the language combo, where the hand that caused this left it. A
    /// `None` rectangle is the degraded path of [`DialogState::reopen_at`]: the window is
    /// still rebuilt, and `DS_CENTER` decides where.
    Again(Option<RECT>),
}

/// How a showing of the settings window ended — решение 99.1, task Т-31-2.
///
/// A third answer beside «ОК» and «Отмена», and the only one the caller has to act on: the two
/// ways of closing are both «the window is gone», and the loop of [`crate::tray::open_settings`]
/// leaves on either.
// ⚠ No `Eq`, for the reason [`Opening`] above has none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DialogOutcome {
    /// The window was closed — «ОК», «Отмена», `Esc` or the cross.
    Closed,
    /// The direction of writing changed under it: show it again, in this rectangle.
    Reopen(Option<RECT>),
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

    /// Records the window this session names — `ui::windows::settings::open` once the host has
    /// made it (T-120-20), so that a second start of the program brings it forward
    /// ([`open_window`]).
    pub fn record(hwnd: HWND) {
        DIALOG_WINDOW.with(|window| window.set(hwnd.0 as isize));
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

/// Whether the «О программе» window of FR-92а (`ui::windows::about`) is on the screen of
/// **this** thread.
///
/// The published half of [`ABOUT_WINDOW`]: one `bool`, read, never written. True from the moment
/// `about::open` has made the window until the window is destroyed. It answers two questions —
/// may a second copy be made (the door at the top of `about::open`), and which commands of the
/// menu are locked (`tray::dialog_locks_command`). The record alone is enough for both because
/// nothing posted is dispatched between the making of the window and its recording: the menu of
/// FR-91 and the double click on the icon both arrive posted.
///
/// **SEC-05, R-20.** Nothing outside this process can move this: the record is a thread-local of
/// the UI thread, written by the function that made the window and cleared when its content is
/// dropped. There is no window message, no `wParam` and no pointer anywhere on this road.
pub fn about_is_open() -> bool {
    ABOUT_WINDOW.with(Cell::get) != 0
}

/// The window of this thread a second start of the program brings forward — FR-82 since 1.8.0,
/// task T-119-14: the settings dialog of FR-92 if it is up, else the «О программе» window (which
/// keeps the settings from opening, `tray::dialog_locks_command`), else `None`.
pub fn open_window() -> Option<HWND> {
    live_window(&DIALOG_WINDOW).or_else(|| live_window(&ABOUT_WINDOW))
}

/// The open «О программе» window of this thread, for as long as this value lives — FR-92а.
///
/// `about::open` puts it into the content of the window, and the host of the shell drops the
/// content when the window is destroyed: the record goes with the window however it closed —
/// a button, Esc, the cross of the caption, the end of the program — and with a window Windows
/// refused to make. A record left behind would name a handle Windows is free to give to
/// somebody else's window.
pub struct AboutSession;

impl AboutSession {
    /// Hands back the guard whose [`Drop`] clears the record. Nothing is recorded yet: there is
    /// no window to name until the host has made it — [`AboutSession::record`] does that.
    pub fn open() -> Self {
        Self
    }

    /// Records the window this session names — called by `about::open` once the window is made,
    /// and from nowhere else in the program.
    ///
    /// Public so that the tests exercise the very guard the product uses, over the very record
    /// it writes. It is an in-process Rust call and not a door SEC-05 speaks of — no window
    /// message reaches it, and nothing but this program links this library.
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

    /// The window of «О программе» while it is up, zero otherwise — FR-92а.
    ///
    /// Written by [`AboutSession::record`], cleared when that session is dropped with the
    /// window's content; stored as the plain integer the handle is, and thread-local because the
    /// window belongs to the UI thread and to no other (section 6.1). Read through
    /// [`about_is_open`] and [`open_window`]. Nothing is posted to it: the window of the shell
    /// follows the system theme on its own.
    static ABOUT_WINDOW: Cell<isize> = const { Cell::new(0) };
}

/// The windows' own «системная тема сменилась» nudge — `WM_APP + 14`. The windows of the letters
/// (`letters`) answer it by resolving their palette afresh; the windows of the shell follow the
/// system theme on their own and are not sent it.
///
/// SEC-05: the message carries nothing and decides nothing.
pub const WM_APP_SYSTEM_THEME: u32 = WM_APP + 14;

/// The window a record names, or `None` when the record is empty — FR-92а, task T-13-17.
///
/// Both records — [`DIALOG_WINDOW`] and [`ABOUT_WINDOW`] — are read by this one body. Zero is
/// «нет окна» in both, and it is the value both are born with and are returned to by their
/// guards, so a `None` here is «окна нет» and never «окно есть, но его номер потерян».
fn live_window(record: &'static std::thread::LocalKey<Cell<isize>>) -> Option<HWND> {
    let raw = record.with(Cell::get);

    if raw == 0 {
        return None;
    }

    Some(HWND(raw as *mut std::ffi::c_void))
}

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
///   construction — since task T-80-2 it asks this very table, with no wrapper between;
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

// ⛔⛔ **`about_button_colors` стояла здесь и снята задачей T-80-2.**
//
// Это была обёртка над таблицей выше, заменявшая рамку на `FaceItself` для **всех** кнопок окна
// «О программе» и трёх окон `letters`. Основанием была находка A-13: макет рисует кнопку окна «О
// программе» заливкой без обводки. Макет прочитан верно — но макет не показывает, что делает
// безрамочная кнопка на живой машине:
//
// - в «Тумане» горячее лицо `hover_bg` 234,238,242 стоит на земле окна `window_bg` 237,239,242 —
//   **3/1/0**, и без каймы кнопка под курсором просто гаснет;
// - в «От автора» кнопки стоят на блоках, а в «Тумане» `panel_bg` и `button_bg` — одно и то же
//   255,255,255, так что безрамочная кнопка в покое невидима целиком.
//
// Владелец увидел это на `e79` и назвал лечение сам: «Нужно привести все кнопки к
// типизированному механизму, который у нас уже существует, а не придумывать отдельное решение».
// Поэтому рамка стала **столбцом** таблицы [`button_color_roles`], а обёртки нет: четыре окна
// больше не могут спрашивать не то, что спрашивает пятое. Единственная строка без рамки —
// `IDC_ABOUT_NAME`, и она написана там же, в таблице.

/// The six group panels of FR-92 — FR-92а, tasks T-11-5c and T-11-13.
///
/// The single place the six are listed. Since task T-11-13 they are not visible elements
/// at all (`NOT WS_VISIBLE` in `app.rc`) and no `WM_DRAWITEM` can reach them: what reads
/// this list now is [`background_figure`], which tells the dialog's own background drawing
/// that the rectangle of one of these controls is a panel — and [`collect_panel_children`],
/// which reads it to split the dialog's children into the panels and the controls that may
/// lie on one.
///
/// Eight until task Т-23-2: «Замена» left the dialog whole and «Выделение» was dissolved
/// into «Общие» (решения 81 и 82).
const GROUP_BOXES: [i32; 6] = [
    IDC_GROUP_GENERAL,
    IDC_GROUP_HOTKEY,
    IDC_GROUP_LAYOUTS,
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
/// coordinates of the window being erased, so every rectangle has to be carried across —
/// **corner by corner, with `ScreenToClient`**, and the result put back in order.
///
/// # ⚠ Why not the one-vector shortcut, which is what stood here until task Т-30-2
///
/// Until вопрос 97 this asked `ClientToScreen` for the dialog's client origin **once** and
/// subtracted it from all forty-four rectangles — the same answer for a fraction of the calls,
/// and correct for as long as the two coordinate systems differed only by a translation.
///
/// A mirrored window is the case where they do not. Screen coordinates are never mirrored;
/// client coordinates of a window carrying `WS_EX_LAYOUTRTL` run **right to left**. So the
/// mapping is not a shift but a shift *and a reflection*, the subtraction produced negative
/// coordinates mirrored about the client origin, and every panel of the settings window was
/// painted off the left edge of it. Hebrew and Arabic came up with no panel backgrounds and no
/// panel captions at all — found by **eye on the stand** (`scratchpad-Э30\ЖИВОЕ-he-*.png`),
/// which is where this kind of defect keeps being found in this program.
///
/// `ScreenToClient` accounts for the mirroring; the reflection it applies is what swaps the two
/// x's, so the corners come back in the wrong order and are put back in the right one below. In
/// an ordinary window the swap is a no-op and the answer is what it always was.
///
/// NFR-13: a refused conversion drops that one child rather than the whole list — a panel
/// missing is what a missing rectangle has always cost here, and the other five still get their
/// background.
pub(crate) fn child_rects_in_client(hwnd: HWND) -> Vec<(i32, RECT)> {
    child_rects(hwnd)
        .into_iter()
        .filter_map(|(control, rect)| Some((control, screen_rect_in_client(hwnd, &rect)?)))
        .collect()
}

/// One screen rectangle in the client coordinates of `hwnd`, in order — task Т-30-2.
///
/// The one place this conversion is written down, because it is the one that a mirrored window
/// turns from a subtraction into a reflection. See [`child_rects_in_client`] for what that cost
/// when it was written out by hand.
fn screen_rect_in_client(hwnd: HWND, rect: &RECT) -> Option<RECT> {
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
        // SAFETY: `hwnd` is a live window and `corner` is a live local of this frame that the
        // call rewrites in place; nothing else is written.
        if !unsafe { ScreenToClient(hwnd, corner) }.as_bool() {
            return None;
        }
    }

    // ⚠ `min`/`max` and not «first corner, second corner»: in a mirrored window the two have
    // just been swapped, and a `RECT` whose `left` is greater than its `right` is empty to every
    // GDI call that takes one — the figure would silently not be drawn at all.
    Some(RECT {
        left: corners[0].x.min(corners[1].x),
        top: corners[0].y.min(corners[1].y),
        right: corners[0].x.max(corners[1].x),
        bottom: corners[0].y.max(corners[1].y),
    })
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
pub(crate) fn apply_title_bar_theme(hwnd: HWND, palette: &theme::Palette) {
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
pub(crate) struct CaptionIcons {
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
    pub(crate) fn load() -> Option<Self> {
        let module = resource_module();
        let instance = HINSTANCE(module.0);

        // The caption shows the small frame, and «small» is a system metric, not sixteen:
        // the answer follows the display scale. `crate::tray` asks the same question for the
        // notification area and its function is the one asked here, rather than a second
        // copy of the same `GetSystemMetrics` pair (§6.2).
        let (small_cx, small_cy) = crate::tray::small_icon_size(None);

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
    pub(crate) fn frames(&self) -> [(u32, HICON); 2] {
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
    pub(crate) unsafe fn show_on(hwnd: HWND, frames: [(u32, HICON); 2]) {
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

// ⚠ **Полная перерисовка окна переехала в `widgets::repaint::whole` — задача Т-45-2.** Тело не
// менялось; здесь стояла `repaint_whole_window`. Названа она была за то, что делает, а не за
// первый повод, ради которого написана: до появления второго вызывающего это была
// `repaint_after_palette_change`.

/// One `WM_CTLCOLOR*` answer, chosen but not yet applied: the caption ink (`None` — no text
/// on this DC), the opaque text background of a field (`None` — transparent), and the brush
/// the control is erased with.
pub(crate) type CtlColorChoice = (Option<COLORREF>, Option<COLORREF>, HBRUSH);

/// Applies one chosen `WM_CTLCOLOR*` answer to the DC — the drawing half [`on_ctl_color`]
/// and [`on_about_ctl_color`] share (§6.2, task T-11-11: one body, not a copy).
///
/// `None` is the «not handled» of both callers — the system colours — and answers zero.
pub(crate) fn apply_ctl_color(dc: HDC, choice: Option<CtlColorChoice>) -> isize {
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
// ⚠ **Значение уехало в `widgets::combo::LIST_ITEM_EXTRA` — задача Т-45-2.** Здесь осталось
// прежнее имя, чтобы вызовы и тесты этого окна читались как читались; число одно и живёт в слое.
pub use crate::widgets::combo::LIST_ITEM_EXTRA as COMBO_LIST_ITEM_EXTRA;

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
// ⚠ Как и соседняя: значение уехало в `widgets::combo::CLOSED_ITEM_EXTRA` задачей Т-45-2.
pub use crate::widgets::combo::CLOSED_ITEM_EXTRA as COMBO_CLOSED_ITEM_EXTRA;

// ⚠ **`CB_SELECTION_FIELD` уехала в `widgets::combo` — задача Т-45-2.** Документированная −1,
// названная беззнаковым словом, каким и бывает параметр сообщения; шлёт её теперь слой.

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

/// Whether the change `LVN_ITEMCHANGING` is announcing **puts a tick in** — task T-42-6.
///
/// The state image of a list view row lives in the top four bits of the state word, and the
/// program's own tick is [`CHECKED_IMAGE`] there ([`read_cycle_checks`] reads it the same way).
/// A change that moves those bits to the ticked image, from anything else, is a hand ticking a
/// row; everything else — a selection, a focus, a tick coming **out** — is not.
///
/// Pure, so the rule can be read and tested without a list view.
pub fn is_ticking_a_row(old_state: u32, new_state: u32) -> bool {
    let image = |state: u32| state & LVIS_STATEIMAGEMASK.0;

    image(new_state) == CHECKED_IMAGE && image(old_state) != CHECKED_IMAGE
}

/// Whether one more tick would be the **ninth** — finding Н112, решение 124.3.
///
/// `already` is how many rows carry a tick **apart from the one being ticked**. The engine
/// takes the first [`crate::layouts::MAX_CYCLE`] of them and drops the rest silently
/// (`layouts.rs`, the rule of §7), so a window that let the ninth tick go in would be showing
/// a choice the program does not honour. Задача отвергает такое изменение — галочка просто не
/// встаёт, — и это и есть видимый отказ: решение 124.3 сняло прежнее «приглушать строки», это
/// была бы смена вида.
///
/// Pure, and public so the test calls the very function the window calls.
pub fn ninth_tick_is_refused(already: usize) -> bool {
    already >= crate::layouts::MAX_CYCLE
}

/// Why the cycle list refused a change — [`cycle_change_refusal`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CycleRefusal {
    /// The pair mode: the list has no ticks to give and nobody asked the program for anything
    /// — task T-11-7-2, FR-31.
    Silent,
    /// The ceiling of the cycle: the hand asked for a ninth layout and the program takes eight
    /// — finding Н112. **Heard as well as seen** (FR-100, the third tone).
    Aloud,
}

/// The whole rule of the `LVN_ITEMCHANGING` gate, as one pure function — tasks T-11-7-2 and
/// T-42-6.
///
/// Answers `Some` when the change the list view is announcing must be refused, and **why** is
/// the value: a refusal earned by the pair mode is silent, one earned by the ceiling of the
/// cycle is [`CycleRefusal::Aloud`] and the window follows it with the dull knock of FR-100.
///
/// Everything the decision rests on is an argument, so the rule can be read in one place and
/// tested without a list view — the form every other decision of this file takes. What the
/// window adds is the *asking*: the mode out of the state, the three state fields out of the
/// notification, and the count of ticks out of the control itself.
pub fn cycle_change_refusal(
    mode: Option<LayoutMode>,
    state_changed: bool,
    old_state: u32,
    new_state: u32,
    ticked_besides: usize,
) -> Option<CycleRefusal> {
    if cycle_list_change_is_refused(mode) {
        return Some(CycleRefusal::Silent);
    }

    // ⚠ `mode.is_some()` is not a formality — it is the whole difference between a hand and the
    // program. `None` is a state held borrowed by [`fill_cycle_list`] or [`move_cycle_row`]
    // writing the list itself, and the **file** is allowed to name more layouts than
    // [`crate::layouts::MAX_CYCLE`] (finding Н49: the engine drops the surplus silently, and §7
    // of `SPEC.md` says so). A ceiling applied to the program's own fill would quietly erase
    // the ninth line of somebody's file the moment the window opened on it. The ceiling belongs
    // to the hand. ⭐ Caught by the test of this very task, not by the eye.
    if mode.is_some()
        && state_changed
        && is_ticking_a_row(old_state, new_state)
        && ninth_tick_is_refused(ticked_besides)
    {
        return Some(CycleRefusal::Aloud);
    }

    None
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
pub(crate) unsafe fn paint_push_button(
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

    let dpi = drawing_dpi(dc);

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

/// Which way a **name** reads — решение 97.2 (г), task Т-30-4.
///
/// The rule the user gave in so many words: by the **first strong character**. A name is not
/// text of the interface language — a layout is named in its own language and so is a locale in
/// the language chooser — so the direction belongs to the name and not to the window. `English
/// (United States)` and `Русский (Россия)` read left to right in a Hebrew window; `עברית` reads
/// right to left in an English one, and a rule that said «names are islands» would force that
/// one round the wrong way. This is the same rule the bidi algorithm uses to give a paragraph
/// its direction, written down here because this program has to apply it to a **run** inside a
/// window whose own direction is already decided.
///
/// A name of no strong characters at all — digits, punctuation, or nothing — is [`Native`]: it
/// looks the same either way, and the window's own direction is the better default for a run
/// that has no opinion.
///
/// ⚠ The strong right-to-left blocks are Hebrew (U+0590…U+05FF) and Arabic (U+0600…U+06FF, with
/// the supplement and extended blocks up to U+08FF), which are the two вопрос 97 admits and the
/// two `Language::is_rtl` names. A fifteenth locale in another right-to-left script would want
/// this list extended, and it is one list.
///
/// Whether a panel caption may be drawn with the letter spacing of the mock-ups — task Т-30-4.
///
/// **No, for a caption carrying a strong right-to-left letter**, and the reason is not taste.
/// The spacing is applied by drawing every character with a `TextOutW` of its own, and Arabic is
/// a cursive script: a letter's shape depends on what it joins to, so a run cut into characters
/// comes out as isolated forms with gaps — unreadable rather than merely wide. Found by eye on
/// the stand, `scratchpad-Э30\ШАПКА2-ar.png`, where «الاستثناءات» came out as eleven letters.
///
/// Hebrew is not cursive and would survive the cutting, and it is refused all the same: neither
/// script has capitals, so «capitals, spaced out» is a device with nothing to apply it to, and
/// letter-spacing either of them is a typographic error and not a style. One rule for both, and
/// it reads off the text rather than off the locale — a caption is what it is whoever is looking
/// at it.
///
/// ⛔ **Stage E108, the stand (FAIL 1): no, for a caption carrying a mark that joins or stacks**
/// either. A Devanagari vowel sign, a virama, a Thai tone mark, a Khmer subscript is a character
/// of its own, and drawn by a `TextOutW` of its own it loses the letter it belongs to: the stand
/// saw «स◌ाम◌ान◌्य» for «सामान्य» (`reports\shots-E108\README.md`). None of those scripts has
/// capitals either. See [`is_joining_or_stacking`].
pub fn caption_takes_tracking(caption: &str) -> bool {
    !caption
        .chars()
        .any(|character| is_strong_rtl(character) || is_joining_or_stacking(character))
}

/// A character of a script whose marks join or stack onto a letter — the scripts of India from
/// Devanagari to Sinhala, Thai, Lao, Tibetan, Myanmar and Khmer — or a combining mark of any
/// script: drawn alone, it has nothing to sit on.
///
/// By blocks and not by language, as [`is_strong_rtl`] is: the rule reads the text.
const fn is_joining_or_stacking(character: char) -> bool {
    matches!(
        character,
        '\u{0300}'..='\u{036F}'
            | '\u{0900}'..='\u{0FFF}'
            | '\u{1000}'..='\u{109F}'
            | '\u{1780}'..='\u{17FF}'
            | '\u{19E0}'..='\u{19FF}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE20}'..='\u{FE2F}'
    )
}

/// A character of one of the two right-to-left scripts вопрос 97 admits — the Hebrew and Arabic
/// blocks, with the presentation forms a font may substitute into.
///
/// One list, read by [`caption_takes_tracking`] and by [`reading_of_name`]. A fifteenth locale
/// in another right-to-left script would extend it here and nowhere else.
const fn is_strong_rtl(character: char) -> bool {
    matches!(
        character,
        '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}'
    )
}

/// Public because a test cannot check a rule it is not allowed to read, and this one is a rule
/// of the interface rather than an implementation detail: it decides what a person sees in the
/// two lists of layouts and in the language chooser.
///
/// [`Native`]: theme::Reading::Native
pub fn reading_of_name(name: &str) -> theme::Reading {
    for character in name.chars() {
        if is_strong_rtl(character) {
            return theme::Reading::Native;
        }

        if character.is_alphabetic() {
            return theme::Reading::LatinIsland;
        }
    }

    theme::Reading::Native
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
// ⚠ **Тело уехало в `widgets::dialog_units` — задача Т-45-2.** Единица диалога есть мера
// ЭЛЕМЕНТОВ окна, и на ней стоит вся геометрия общего слоя; здесь она осталась только именем,
// внесённым `use` в шапке файла, — чтобы все три десятка вызовов этого файла читались так же,
// как читались.
use crate::widgets::dialog_units;

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
/// of the same call; **11 since решение 90**.
///
/// # ⚠ 5 → 11: воздух над заголовком, и почему это не «подвинуть картинку»
///
/// The generator's 5 put the caption 4 px under the panel's top edge at 96 DPI, and left 13 px
/// between the caption and the first control below it — a block whose heading is pressed
/// against the ceiling and floats over a gap. The user named it after e31 («я бы добавил
/// немного пустого места над заголовками, сейчас как будто немного тесновато»), and the
/// measurement agreed: 4 above, 13 below.
///
/// 11 mock-up pixels are 8 screen pixels at 96 DPI, which leaves 9 below — the caption sits
/// **between** the edge and the content instead of on the edge. ⚠ Nothing moves for this: the
/// caption is drawn inside room the panel already had, so no control, panel or window changes
/// by a unit. That is the whole reason this was a one-constant answer and the title bar of
/// пункт 1 was not.
///
/// The number departs from the Э11 generator, like [`PANEL_CAPTION_POINTS_TENTHS`] did before
/// it — and for the same kind of reason: the generator drew a 9 pt window, and both windows
/// are set larger since решения 85, 87 и 89. The air a heading needs grew with the type.
pub const PANEL_CAPTION_INSET_Y: i32 = 11;

/// Point size of the caption face, in **tenths of a point** — the `$FSCAP = 7.6 * $DPI` of
/// the generator, п. 7 of task T-11-16; re-sized to 8,3 by решение 89.
///
/// Tenths of a point rather than a percentage of the dialog font, because that is how the
/// generator states it: two point sizes, `7.6` and `9`, whose ratio is 0,844. T-11-15 carried
/// the ratio as a rounded «85 %», which is a number the pictures do not have.
///
/// # ⚠ 7,6 → 8,3: the one row Э26 grew everything **around** and not itself
///
/// Both windows were re-set one step larger by решения 85 and 87, and this face was the single
/// thing left behind — measured on the raised windows, not deduced:
///
/// | окно | своё лицо | шапка при 76/90 |
/// |---|---|---|
/// | настройки | −13 (ячейка 17 px) | **−10** (12 px) |
/// | «О программе» | −12, тело −13 | **−10** (12 px) |
///
/// ⚠ In the settings dialog it very nearly *did* grow and was thrown back by arithmetic:
/// `−13 × 76 / 90 = −10,98`, and integer division truncates toward zero. A hundredth of a
/// pixel is what kept the caption at its old size while the window around it grew.
///
/// The user saw it («в шапке шрифт мелковат») before anything here was measured. 8,3 answers
/// **−11 in both windows**, and it is not a number picked to be bigger: the accepted mock-up
/// sets its `.helpcap` at 10,5 px against a 12,5 px row — a ratio of 0,84 — and 11 / 13 is
/// 0,846. The caption goes back to the proportion the picture has.
pub const PANEL_CAPTION_POINTS_TENTHS: i32 = 83;

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
///
/// ⚠ **10,7 until task Т-26-2.** The 15 pt of the Э11 generator is one mock-up; the mock-up the
/// user accepted with his own eye in stage Э23 is another, and решение 85 makes it the model:
/// `.appname { font-size: 15.5px }` there, which is 15,5 × 72 / 96 = **11,6 pt**. The two
/// numbers are read off two different pictures of the same window and the newer one wins,
/// because it is the one a person looked at and said «намного лучше» about.
pub const ABOUT_NAME_POINTS_TENTHS: i32 = 116;

/// Point size of the **body** of the about window, in tenths of a point — task Т-26-2,
/// решение 85.
///
/// The two description lines and the five sentences of the help panel. The accepted mock-up
/// sets the first at `font-size: 13px` (9,75 pt) and the second at `12.5px` (9,4 pt); one
/// number carries both, because a `lfHeight` is whole pixels and the two round together: at
/// 96 DPI the dialog face is −12 and this ratio answers −13 for both, at 120 DPI it is −15 and
/// this answers −16. A separate constant for 9,4 pt would answer **the same face** and cost a
/// second name for one thing.
///
/// ⚠ **This is the about window's number and stays so.** The settings dialog is set one step
/// larger too since решение 87 п. 3 — but it gets there by asking its **template** for a 10 pt
/// font, not by drawing its own face over a 9 pt one. See the `FONT` line of `IDD_SETTINGS`:
/// dialog units are *defined* by the dialog font, so a window that draws in a face its
/// template does not know has broken the one invariant its whole layout rests on. This window
/// can afford it because its body is four kinds of label it owns outright; a dialog of forty
/// controls cannot.
pub const ABOUT_BODY_POINTS_TENTHS: i32 = 100;

/// Point size of the key name **inside a chip**, in tenths of a point — task Т-26-2,
/// решение 85 п. 1.
///
/// `kbd { font: 600 11px }` of the accepted mock-up is 8,25 pt; 8,5 is the nearest tenth that
/// still comes out a whole pixel below the sentence around it at 96 DPI (−11 against −13), and
/// that gap is what makes the box read as a key cap.
pub const ABOUT_CHIP_POINTS_TENTHS: i32 = 85;

/// Distance from one line of the about window's body to the next, as a **percentage of the
/// body face's em** — `line-height: 1.4` of the accepted mock-up, task Т-26-2.
///
/// A percentage of the face and not a length of a picture, for the reason
/// `theme::CHIP_PAD_X_PERCENT` is one: the air between two lines of text belongs to the text.
/// [`about_body_line_pitch`] is the whole of the arithmetic.
pub const ABOUT_BODY_LINE_PERCENT: i32 = 140;

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

/// The two input fields and two lists that lost `WS_BORDER` in task T-11-13 — п. 2.3.
///
/// The system border was a sunken rectangle in the system's colours; these four now carry
/// the rounded [`theme::Palette::field_border`] frame the mock-ups show, drawn by the
/// dialog's background one pixel outside each rectangle. The list is what
/// [`background_figure`] answers `Field` for, and a test reads the same four identifiers
/// out of the built binary to check that not one of them kept `WS_BORDER`.
///
/// Seven until task Т-23-2 took the three `ES_NUMBER` millisecond fields off the window.
const FRAMED_FIELDS: [i32; 4] = [
    IDC_HOTKEY,
    IDC_EXCLUSION_NAME,
    IDC_EXCLUSIONS,
    IDC_CYCLE_LIST,
];

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

// =========================================================================================
// Стоимость фона: одна готовая картинка вместо восьми панелей на каждую перерисовку —
// FR-92а, task T-11-17, criterion 12
// =========================================================================================

// =========================================================================================
// Сглаживание нашего текста — FR-92а, task T-11-17 п. 2, поправлено решением 88
// =========================================================================================
//
// The text **this file draws itself** is set in the smoothing this program names for itself,
// through the one function that names it — `theme::smoothed_logfont`. What that mode is has
// changed once, and the history is the point:
//
// - **T-11-17, 2026-08-22 — grey coverage** (`ANTIALIASED_QUALITY`). ClearType tints the edge
//   of every stroke red and blue; against the graphite ground of FR-92а that fringe is what
//   `zoom-pairs.png` shows as colour around the live captions, and the Э11 mock-ups have none
//   of it because they were drawn with GDI+ grey coverage.
// - **Решение 88, 2026-09-02 — ClearType.** ⚠ The fringe was not the only cost being paid, and
//   the second one was invisible until it was measured: GDI places glyphs on whole pixels, so
//   under grey coverage a stem lands inside a pixel in one place of a word and straddles two in
//   another. The user read the half-lit neighbour as an echo — «шрифт двоится» — and the same
//   effect had already made the body of the about window look bold until Т-26-2 measured its
//   weight at 400. Both modes were rendered side by side at 6× and the choice was made by eye:
//   `scratchpad-Э26\ЛИСТ-*.png`, `ВРЕЗ-*.png`. The fringe is back and is the price.
//
// The derivation lives at `theme::smoothed_logfont`; nothing here names a quality of its own.
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
// face handed over is [`DialogFonts::text`] — [`smoothed_logfont`] of the window's own
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
/// quality — through [`smoothed_logfont`], so there is one place that names the quality and
/// not two. Pure, like it.
pub fn caption_logfont(base: LOGFONTW) -> LOGFONTW {
    scaled_logfont(base, PANEL_CAPTION_POINTS_TENTHS, FW_BOLD)
}

/// The `LOGFONTW` of the name row of the about window — finding **A-05**, task T-12-4.
///
/// [`caption_logfont`] with one number changed: [`ABOUT_NAME_POINTS_TENTHS`] over
/// [`DIALOG_FONT_POINTS_TENTHS`] instead of the caption's ratio, so the row comes out *larger*
/// than the dialog font where the panel caption comes out smaller. Bold in both cases, and
/// both go through [`smoothed_logfont`], so the quality is named in exactly one place.
///
/// ⚠ **The size travels through the window's own `lfHeight` and never through «10,7 pt»**: the
/// dialog font is what the template asks for and what the manager already created at the DPI
/// of the window, so a face derived from it is right at every scale, and a face created from a
/// point size by hand would be right at 96 DPI only.
///
/// ⚠ **Twice re-sized, and the second time by решение 85.** T-12-4 measured 10,7 pt off the
/// generator of the Э11 mock-ups; the mock-up the user accepted with his own eye in stage Э23
/// (`scratchpad-Э23\макет-справка-о-программе.html`) sets the same row at `font-size: 15.5px;
/// font-weight: 650`, which is 15,5 × 72 / 96 = **11,6 pt** — the number
/// [`ABOUT_NAME_POINTS_TENTHS`] carries now. The weight went with it: 650 is nearer semibold
/// than bold, and [`FW_SEMIBOLD`] is what a `LOGFONTW` says that with.
pub fn about_name_logfont(base: LOGFONTW, emphasis: Emphasis) -> LOGFONTW {
    emphasised_logfont(base, ABOUT_NAME_POINTS_TENTHS, emphasis)
}

/// The `LOGFONTW` of the **body** of the about window — task Т-26-2, решение 85.
///
/// One step larger than the dialog font: `font-size: 13px` and `12.5px` of the accepted
/// mock-up, which are 9,75 and 9,4 pt — [`ABOUT_BODY_POINTS_TENTHS`] rounds the pair to one
/// number, because the two land on the same whole pixel at every DPI this program is drawn at
/// and a face is made of whole pixels. Ordinary weight: the mock-up's body is `font-weight`
/// unset, and the hypothesis that the live window drew it **bold** is refuted in
/// `reports\ИТОГ-Э26.md` §1 — the template's own `FONT 9, "Segoe UI", 400` is what those rows
/// have always been drawn in.
pub fn about_body_logfont(base: LOGFONTW) -> LOGFONTW {
    scaled_logfont(base, ABOUT_BODY_POINTS_TENTHS, FONT_WEIGHT(0))
}

/// The `LOGFONTW` of a **numeral** of the help panel — task Т-26-2, решение 85.
///
/// The body face, semibold: `.key { font-weight: 600 }` of the accepted mock-up, against the
/// ordinary weight of the sentence beside it. The column of digits is muted as well — that is
/// [`about_static_color_role`]'s half of the same decision and was already true.
pub fn about_number_logfont(base: LOGFONTW, emphasis: Emphasis) -> LOGFONTW {
    emphasised_logfont(base, ABOUT_BODY_POINTS_TENTHS, emphasis)
}

/// The `LOGFONTW` of the key name **inside a chip** — task Т-26-2, решение 85 п. 1.
///
/// `kbd { font: 600 11px }` of the accepted mock-up: 11 × 72 / 96 = 8,25 pt, and
/// [`ABOUT_CHIP_POINTS_TENTHS`] carries 8,5 — the nearest size that is still a whole pixel
/// smaller than the sentence around it at 96 DPI, which is what makes the box read as a key cap
/// rather than as emphasis.
pub fn about_chip_logfont(base: LOGFONTW, emphasis: Emphasis) -> LOGFONTW {
    emphasised_logfont(base, ABOUT_CHIP_POINTS_TENTHS, emphasis)
}

/// Whether this system really has [`SEMIBOLD_FAMILY`] — asked once, when a window's faces are
/// made, and of GDI itself.
///
/// ⚠ **A created face is not proof of anything.** `CreateFontIndirectW` answers a valid handle
/// for a family nobody has, having quietly mapped the request to whatever it considered nearest;
/// on a system without the semibold family that could be another type face entirely, and the
/// window would come out in it without one call failing. The only honest question is therefore
/// *what came back*, and `GetTextFaceW` of the face selected into a DC is what answers it.
///
/// `false` on every refusal — no DC, no face, an unreadable name (NFR-13) — which spends the
/// window on [`Emphasis::Bold`]: the picture of task T-12-4, and a good one.
pub(crate) fn resolve_emphasis(hwnd: HWND, base: LOGFONTW) -> Emphasis {
    let wanted = emphasised_logfont(base, ABOUT_NAME_POINTS_TENTHS, Emphasis::Semibold);

    let Some(face) = create_font(wanted) else {
        return Emphasis::Bold;
    };

    // SAFETY: `hwnd` is the live dialog; the DC is released below on every path.
    let dc = unsafe { GetDC(Some(hwnd)) };

    let given = if dc.is_invalid() {
        None
    } else {
        // SAFETY: `dc` is the live DC just obtained and `face` was created above; the previous
        // object is put back before the DC is released.
        let previous = unsafe { SelectObject(dc, face.into()) };

        let mut name = [0u16; 64];

        // SAFETY: `name` is a live local of this frame and its length is what bounds the copy;
        // the call answers the number of units written, never more than the buffer holds.
        let copied = unsafe { GetTextFaceW(dc, Some(&mut name)) };

        // SAFETY: `previous` is what `SelectObject` answered for this same DC a moment ago.
        unsafe { SelectObject(dc, previous) };

        // The count includes the terminating NUL, which is not part of the name.
        let copied = usize::try_from(copied).unwrap_or(0).saturating_sub(1);

        Some(String::from_utf16_lossy(&name[..copied.min(name.len())]))
    };

    if !dc.is_invalid() {
        // SAFETY: `dc` came from the `GetDC` above and is released exactly once, here.
        unsafe { ReleaseDC(Some(hwnd), dc) };
    }

    // SAFETY: `face` was created above, handed to nobody, and is freed exactly once here — it
    // was a probe and never belonged to a `DialogFonts`.
    let _ = unsafe { DeleteObject(face.into()) };

    // ⚠ Deliberately **not** written to the journal. The vocabulary of `diag` is a closed table
    // with a test that holds it closed, and what happened here is not a refusal of anything: the
    // system simply has a different set of type faces, and the window is drawn in the weight
    // that set has. A line here would widen a closed table for a cosmetic fallback.
    match given {
        Some(name) if name == SEMIBOLD_FAMILY => Emphasis::Semibold,
        _ => Emphasis::Bold,
    }
}

/// The dialog's own face at `tenths` of a point and at `weight` — the one body the five
/// builders above and below are made of (§6.2: one body, not five copies).
///
/// A `weight` of zero — `FW_DONTCARE` — leaves the base weight alone, which is what the body
/// face of the about window wants: the template already asks for 400 and a number written here
/// would be a second opinion about it.
///
/// ⚠ **The size travels through the window's own `lfHeight` and never through a point size**:
/// the dialog font is what the template asks for and what the manager already created at the
/// DPI of the window, so a face derived from it is right at every scale, and a face created
/// from a point size by hand would be right at 96 DPI only. `lfHeight` is negative for a font
/// asked for by character height, which is how the manager creates a `DS_SETFONT` face; the
/// multiplication keeps whichever sign it has.
///
/// Pure — no DC, no window — so every one of the five is closed by a table test.
fn scaled_logfont(base: LOGFONTW, tenths: i32, weight: FONT_WEIGHT) -> LOGFONTW {
    let mut logical = smoothed_logfont(base);

    logical.lfHeight = (base.lfHeight * tenths) / DIALOG_FONT_POINTS_TENTHS;

    if weight.0 != 0 {
        logical.lfWeight = i32::try_from(weight.0).unwrap_or(base.lfWeight);
    }

    logical
}

/// The family the real 600 of the accepted mock-up lives in — решение 87 п. 1.
///
/// ⚠ **This is the one place in this module that names a type face**, and it is an exception
/// granted by name: everywhere else the face is the dialog's own, because the template is what
/// chooses it and a name written here would be a second opinion about the template. The
/// exception exists because there is no other way to ask for what the mock-up shows —
/// see [`Emphasis`].
const SEMIBOLD_FAMILY: &str = "Segoe UI Semibold";

/// How the three emphasised roles of the about window are set — решение 87 п. 1.
///
/// # Why this is a choice at all, and why it is made at run time
///
/// The accepted mock-up sets the name row, the help numerals and the chip at `font-weight: 600`.
/// GDI cannot be asked for that through `lfWeight`: the family «Segoe UI» carries **400 and 700
/// and nothing between**, and a request for 600 is answered — silently — with 700. Measured on
/// the stand: `просили вес 600, дали tmWeight 700`. Writing `FW_SEMIBOLD` in the source would
/// therefore be a line that says one thing and draws another.
///
/// The real 600 is a **family of its own**, [`SEMIBOLD_FAMILY`], and asking for it by name is
/// what решение 87 authorises. That road has its own trap: `CreateFontIndirectW` never fails on
/// a family the system does not have — it maps to the nearest one it does, which could be a
/// different type face altogether. So the family is not assumed, it is **verified** on the
/// created face ([`resolve_emphasis`]), and a system without it falls back to the dialog's own
/// family at `FW_BOLD` — the picture of task T-12-4, which is a good picture and not a failure
/// (NFR-13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emphasis {
    /// [`SEMIBOLD_FAMILY`] — the real 600 of the mock-up. Verified present before it is used.
    Semibold,
    /// The dialog's own family at `FW_BOLD` — what this program drew in before решение 87, and
    /// what a system without the semibold family still draws in.
    Bold,
}

/// The `LOGFONTW` of one emphasised role — task Т-26-2, решение 87 п. 1.
///
/// [`Emphasis::Bold`] is [`scaled_logfont`] at `FW_BOLD` and nothing else. [`Emphasis::Semibold`]
/// is the same size with the family replaced and **`lfWeight` left at zero** — `FW_DONTCARE`,
/// which is the honest thing to say when the weight is the family's own business: the semibold
/// family has one weight and asking it for 700 on top would invite the synthetic bold GDI makes
/// when it cannot find what it was asked for.
///
/// Pure, like every other builder here, so the pair is closed by a table test.
fn emphasised_logfont(base: LOGFONTW, tenths: i32, emphasis: Emphasis) -> LOGFONTW {
    let mut logical = scaled_logfont(base, tenths, FW_BOLD);

    if emphasis == Emphasis::Bold {
        return logical;
    }

    logical.lfWeight = 0;
    logical.lfFaceName = [0; 32];

    // The name, as UTF-16, into the fixed array — one unit short of its end at most, so the
    // NUL the zeroing left behind is never overwritten.
    for (slot, unit) in logical
        .lfFaceName
        .iter_mut()
        .take(SEMIBOLD_FAMILY.len())
        .zip(SEMIBOLD_FAMILY.encode_utf16())
    {
        *slot = unit;
    }

    logical
}

/// The distance from one line of the about window's body to the next, in pixels of the window
/// — task Т-26-2, решение 85.
///
/// `em` is the absolute value of [`about_body_logfont`]'s `lfHeight`, and the answer is
/// [`ABOUT_BODY_LINE_PERCENT`] of it: `line-height: 1.4` of the accepted mock-up, which is the
/// «воздух межстрочья» решение 85 asks for by name. A share of the face and not a length of a
/// picture, so it grows with the face and with the DPI in one step — and it is *larger* than
/// the natural line of that face, which is what [`theme::label_model_pitch`] requires before it
/// will place lines at all.
///
/// Pure.
pub fn about_body_line_pitch(em: i32) -> i32 {
    (em.abs() * ABOUT_BODY_LINE_PERCENT) / 100
}

/// The `LOGFONTW` of the font the dialog manager gave one of the window's own controls.
///
/// The font is asked of a control (`WM_GETFONT` through `SendDlgItemMessageW`, the one way
/// this module sends anything to its own controls): the dialog manager gives its font to every
/// control it creates, and an invisible control has it like any other.
///
/// `None` on every refusal — no font on the control, an unreadable `LOGFONTW` (NFR-13).
pub(crate) fn dialog_logfont(hwnd: HWND, control: i32) -> Option<LOGFONTW> {
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
pub(crate) unsafe fn draw_panel_caption(
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

    // ⛔ **Задача Т-30-4, найдено глазом на стенде: арабская шапка НЕ РАЗБИВАЕТСЯ НА ЗНАКИ.**
    //
    // The letter spacing below is a typographic device of the mock-ups for capitals — «ОБЩИЕ»,
    // «GENERAL» — and it is applied by placing every character with a `TextOutW` of its own.
    // That is harmless for an alphabet whose letters stand apart. Arabic is **cursive**: its
    // letters change shape according to what they join to, and a run cut into single characters
    // comes out as a row of isolated forms with gaps between them — not ugly, *unreadable*.
    // Seen on the stand (`scratchpad-Э30\ШАПКА2-ar.png`): «الاستثناءات» came out as eleven
    // separate letters.
    //
    // Neither Hebrew nor Arabic has capitals either, so there is no «capitals, spaced out» to
    // render in the first place; letter-spacing them is not a style, it is a mistake. So a
    // caption that carries a strong right-to-left letter is drawn as **one run**, and the whole
    // of the device below is skipped. Everything else is drawn exactly as it was.
    if !caption_takes_tracking(&caption) {
        let units: Vec<u16> = caption.encode_utf16().collect();

        // NFR-13: the `BOOL` is examined and deliberately dropped — see the block comment above.
        //
        // SAFETY: `units` is a live local of this frame and the DC is the caller's.
        let _ = unsafe { TextOutW(dc, panel.left + inset_x, top, &units) };

        // SAFETY: the handle was in the DC a moment ago; putting it back ends this function's
        // use of the DC and leaves the caller free to delete `face`.
        unsafe { SelectObject(dc, previous_font) };

        return;
    }

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

/// The reference datum of a subclass installed on a combo box that is **not** on this window.
///
/// Task Т-32-8, and it is what lets the wizard of FR-104 wear the same closed face as the four
/// combo boxes here without a second copy of the two hundred lines that draw it (§6.2, «шов
/// один»). The whole difference between the two cases is where the colours come from:
///
/// * `0` — the combo is one of [`COMBO_BOXES`] on **this** dialog, and the look is read out of
///   its [`DialogState`] as it always was;
/// * [`COMBO_FOREIGN`] — the combo belongs to another window of this program, and the look is
///   built on the spot out of the palette the configuration resolves to. That is the very
///   palette the other window built itself from (`theme::resolve` of the same setting), so the
///   two cannot disagree.
///
/// ⛔ The datum is **not** a pointer and never becomes one: a subclass reference that outlived
/// the thing it named would be a use-after-free reachable from a window message.
pub(crate) const COMBO_FOREIGN: usize = 1;

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
/// Everything the closed face of a combo box is drawn with — the one place the colour table is
/// turned into handles and inks, so that this dialog's four and the wizard's one cannot come
/// out different (§6.2).
///
/// The second element of the tuple is the fill brush and the third **owns** it when the fill is
/// the transient hover colour: a `_` there would free the brush before the first pixel.
type ComboLook = (
    HBRUSH,
    HBRUSH,
    Option<HotBrush>,
    COLORREF,
    COLORREF,
    COLORREF,
    Option<HFONT>,
);

/// Builds that look out of the roles, the palette, a brush set and the ground the corners stand
/// on.
fn combo_look(
    roles: theme::ComboClosedColors,
    palette: &theme::Palette,
    brushes: &theme::Brushes,
    ground: HBRUSH,
    face: Option<HFONT>,
) -> ComboLook {
    // The hot fill is the one colour of the table a window's brush set does not hold — see
    // [`HotBrush`]. `None` for every other role, and also for a refused `CreateSolidBrush`, in
    // which case the fill falls back to the quiet `field_bg` and the closed part simply does
    // not light up (NFR-13).
    let hot = match roles.fill {
        ComboFillRole::HoverBg => HotBrush::new(palette.hover_bg),
        ComboFillRole::FieldBg | ComboFillRole::SelBg => None,
    };

    let fill = hot
        .as_ref()
        .map_or_else(|| combo_fill_brush(roles.fill, brushes), HotBrush::brush);

    (
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
        face,
    )
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
pub(crate) fn is_hot(window: HWND) -> bool {
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
pub(crate) fn subclass_buttons(hwnd: HWND, buttons: &[i32]) {
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
pub(crate) fn unsubclass_buttons(hwnd: HWND, buttons: &[i32]) {
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

/// Air between the text of the closed part and the chevron, in mock-up pixels.
pub const COMBO_CHEVRON_TEXT_GAP: i32 = 4;

/// What acts when the hotkey is pressed — the subject of the fourth line of «Состояние», task
/// T-34-6, finding Н102.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Acting {
    /// Mode «pair» — the two layouts the hotkey switches between, named as the lists name them.
    Pair(String, String),
    /// Mode «cycle» — the layouts in the order the hotkey walks them.
    Cycle(Vec<String>),
    /// Nothing acts: fewer than two layouts in the session, or a pair that resolves to nothing.
    None,
}

/// What the program is doing now — the subject of the first line of «Состояние», task T-34-5,
/// findings Н80 and Н84.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProgramState {
    /// The hook is in place and processing keystrokes.
    Working,
    /// Paused by the person — FR-90, «Приостановить».
    Suspended,
    /// The hook is not installed, or the watchdog has seen it fall — FR-80.
    HookAbsent,
    /// The hook is installed but processing is switched off after the panics of FR-99; only a
    /// restart brings it back.
    Disabled,
}

impl ProgramState {
    /// The string of the interface that names this state.
    pub const fn string_id(self) -> u16 {
        match self {
            Self::Working => IDS_STATE_WORKING,
            Self::Suspended => IDS_STATE_SUSPENDED,
            Self::HookAbsent => IDS_STATE_HOOK_ABSENT,
            Self::Disabled => IDS_STATE_DISABLED,
        }
    }
}

/// The state of the program out of the four flags the modules publish — pure, so that
/// `tests\settings.rs` walks all sixteen combinations.
///
/// The order is the order of what the person can do about it. Fail-safe comes first: a restart
/// is the one remedy, and it is the person's — the watchdog puts a fallen hook back by itself
/// (FR-80), so «not installed» is usually a passing state, but it is still the second thing to
/// say, because a paused program with no hook is not «paused», it is deaf. The pause of FR-90
/// is third, and only a program that is none of those is «Работает».
pub fn program_state(installed: bool, down: bool, active: bool, fail_safe: bool) -> ProgramState {
    if fail_safe {
        ProgramState::Disabled
    } else if !installed || down {
        ProgramState::HookAbsent
    } else if !active {
        ProgramState::Suspended
    } else {
        ProgramState::Working
    }
}

/// The fourth line of «Состояние»: what acts, and how often the choice was refused — pure, so
/// that `tests\settings.rs` can build every shape of it out of the built string tables.
///
/// `refusals` is the sum of the four counters of `layouts::selection_failures`, the ones the
/// dump prints as `layouts.*`; a person whose key stays silent reads here that the choice was
/// refused and by how much, without switching the journal on. Zero is said in words — «отказов
/// выбора не было» — because «отказов: 0 · пара: —» is a table row, not a sentence.
pub fn acting_line(acting: &Acting, refusals: u32) -> String {
    // Stage E108, the stand (FAIL 2), решение 202.19о: in a right-to-left window every name goes
    // into the line as an embedding of its own, so that the arrows read the way the window does
    // whatever script the names are in — [`theme::embedded_name`]. A left-to-right window gets
    // the names as they are, byte for byte as in 1.2.0.
    let right_to_left = ui_language().in_force().is_rtl();
    let in_line = |name: &str| {
        if right_to_left {
            theme::embedded_name(name, reading_of_name(name) == theme::Reading::Native)
        } else {
            name.to_owned()
        }
    };

    let what = match acting {
        Acting::Pair(source, target) => {
            format_text(IDS_STATE_PAIR, &[&in_line(source), &in_line(target)])
        }
        // The arrow is a string since T-L10N-10: « → » left to right, « ← » in the five
        // right-to-left tables, whose first name stands on the right.
        Acting::Cycle(names) => {
            let names: Vec<String> = names.iter().map(|name| in_line(name)).collect();
            format_text(IDS_STATE_CYCLE, &[&names.join(&text(IDS_CYCLE_ARROW))])
        }
        Acting::None => text(IDS_STATE_PAIR_NONE),
    };

    let refused = if refusals == 0 {
        text(IDS_STATE_NO_FAILURES)
    } else {
        format_text(IDS_STATE_FAILURES, &[&refusals.to_string()])
    };

    format_text(IDS_STATE_JOINED, &[&what, &refused])
}

/// **The lines for the author under «Подробности для автора»** — task T-120-21: the four lines of the
/// «Состояние» group of the old dialog, word for word (`fill_state_lines` until task T-120-20), now
/// shown only when that button is pressed. `log_enabled` adds the health of the hook to the first
/// line, as it always did; `session` are the layouts of this session, named as the lists name them.
///
/// **Every number here comes from another module's public reader** — `watchdog`, `hook`, `layouts`,
/// `guard`, `diag` — and nothing is written back to any of them (section 6.2). SEC-07: counts and a
/// path, never a stroke.
pub fn state_details(log_enabled: bool, session: &[LayoutId]) -> Vec<String> {
    let health = crate::watchdog::health();
    let program = program_state(
        crate::hook::is_installed(),
        crate::watchdog::hook_down(),
        crate::hook::is_active(),
        crate::hook::fail_safe(),
    );
    let first = if log_enabled {
        format_text(
            IDS_STATE_JOINED,
            &[
                &text(program.string_id()),
                &format_text(
                    IDS_STATE_HEALTH,
                    &[
                        &health.silent_removals.to_string(),
                        &health.install_failures.to_string(),
                        &crate::hook::lost_hotkeys().to_string(),
                    ],
                ),
            ],
        )
    } else {
        text(program.string_id())
    };
    let counts = format_text(
        IDS_STATE_LAYOUTS,
        &[
            &session.len().to_string(),
            &crate::guard::counters().exclusions.to_string(),
            &crate::diag::recorded().to_string(),
        ],
    );
    let autostart = format_text(
        IDS_STATE_AUTOSTART,
        &[&match autostart_value() {
            Some(command) => format_text(IDS_AUTOSTART_PRESENT, &[&command]),
            None => text(IDS_AUTOSTART_ABSENT),
        }],
    );

    // Task T-34-6, finding Н102: what acts when the key is pressed — the very selection the hotkey
    // makes, `layouts::cycle_for` over the published configuration and the layouts of this session.
    let available: Vec<LayoutId> = session
        .iter()
        .copied()
        .take(crate::layouts::MAX_SESSION)
        .collect();
    let of_session = crate::layouts::Session::of(available.len(), session.len());
    let acting =
        match crate::layouts::cycle_for(crate::layouts::published(), &available, of_session) {
            Ok(cycle) => {
                let names = layout_labels(cycle.layouts(), session);
                match (crate::layouts::published().mode(), names.as_slice()) {
                    (LayoutMode::Pair, [source, target]) => {
                        Acting::Pair(source.clone(), target.clone())
                    }
                    (_, names) if names.len() >= 2 => Acting::Cycle(names.to_vec()),
                    _ => Acting::None,
                }
            }
            Err(_) => Acting::None,
        };
    let failures = crate::layouts::selection_failures();
    let refusals = failures
        .ime_layout
        .saturating_add(failures.no_layouts)
        .saturating_add(failures.origin_outside)
        .saturating_add(failures.too_many_layouts)
        .saturating_add(failures.pair_variant_missing);

    vec![first, counts, autostart, acting_line(&acting, refusals)]
}

// ⚠ **`arrow_key_is_down` переехал в `widgets::glyph` — задача Т-46-2.** Он был частью правила
// нажатия, а правило теперь одно на все окна: тело стоит рядом с `answer`, которое его зовёт.

/// Where a window rebuilt by [`LanguageSwitch::Reopen`] stands — stage E108, the stand (7.4).
///
/// The **place** is the previous window's top left corner, where the eye left it. The **size** is
/// the new window's own: a change between two tables of the tallest window (решение 202.40) opens
/// at another step of the font, and a window kept in the old rectangle showed ≈ 95 px of empty
/// ground under its buttons on the stand (ru → ta, `reports\shots-E108\s7-live-ru-to-ta.png`). A
/// change of the direction of writing alone keeps the step and so the size: nothing moves, as in
/// 1.2.0. A window that grew past the bottom of the work area is lifted, never above its top.
///
/// Pure, so that `tests\settings.rs` reads the rule the dialog uses.
pub fn reopened_window(previous: RECT, own: RECT, work_area: RECT) -> RECT {
    let width = own.right - own.left;
    let height = own.bottom - own.top;

    let top = if previous.top + height > work_area.bottom {
        (work_area.bottom - height).max(work_area.top)
    } else {
        previous.top
    };

    RECT {
        left: previous.left,
        top,
        right: previous.left + width,
        bottom: top + height,
    }
}

/// What the window does with a name typed into the exclusion field — [`add_exclusion`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclusionVerdict {
    /// The name goes into the list.
    Add,
    /// The list already holds it, under this spelling or another — nothing to do, nothing to
    /// say.
    Duplicate,
    /// The name cannot be an exclusion at all, and the window says so aloud.
    Refuse,
}

/// The rule of [`add_exclusion`], as a pure function — task T-42-7, finding Н29.
///
/// # Why a name can be refused at all
///
/// `guard::publish_exclusions` already counts three refusals — an empty fold, a name too long,
/// and everything past `guard::MAX_EXCLUSIONS` — into `Counters::exclusions_refused`, and the
/// dump prints it. But it counts them **on publication**, that is after «ОК», where nobody is
/// looking: a person types `C:\` into the field, presses «Добавить», sees the name accepted,
/// closes the window, and the exclusion is quietly not there. The window has to answer **before**
/// the publication, which is what this rule is.
///
/// **The empty fold.** `guard::fold_process_name` takes the last component of a path and lower-
/// cases it; `C:\`, `\` and `..\` fold to nothing at all, and a nothing matches no process. Such
/// a name is refused here rather than stored to be dropped later.
///
/// ⚠ The ceiling of `MAX_EXCLUSIONS` is **not** an arm of this rule: it is shown before the hand
/// gets this far — the field and the button are dark at the ceiling ([`show_exclusion_ceiling`]).
/// A name typed while they were still lit and submitted at the ceiling cannot happen through the
/// window at all.
///
/// Pure, and public so the test calls the very function the window calls.
pub fn exclusion_verdict(name: &str, existing: &[String]) -> ExclusionVerdict {
    let folded = crate::guard::fold_process_name(name);

    if folded.is_empty() {
        return ExclusionVerdict::Refuse;
    }

    if existing
        .iter()
        .any(|item| crate::guard::fold_process_name(item) == folded)
    {
        return ExclusionVerdict::Duplicate;
    }

    ExclusionVerdict::Add
}

/// Whether the exclusion list is full — `guard::MAX_EXCLUSIONS` names, finding Н29.
///
/// Pure, and reading the ceiling from `guard` rather than repeating the number: the list the
/// window shows and the list the engine publishes are the same list, and one of them growing a
/// ceiling of its own is the defect this pair of functions exists to prevent.
pub fn exclusions_are_full(count: usize) -> bool {
    count >= crate::guard::MAX_EXCLUSIONS
}

/// What «Выбрать окно программы» does with what it found in front — task T-100-3, вопрос 172.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExclusionPick {
    /// Nothing to add — no window in front, a window of this program, or a Store frame whose
    /// program could not be told (решение 172.20). «Окно не выбрано».
    NotPicked,
    /// The name goes into the list and its row is selected — «Добавлено: …».
    Added(String),
    /// The list holds it already: this row, under its own spelling, is selected —
    /// «Уже в списке: …».
    AlreadyListed(usize, String),
    /// The name cannot be an exclusion — longer than the field takes, or folding to nothing. The
    /// third tone of FR-100 and «Окно не выбрано».
    Refused,
}

/// **The rule of «Выбрать окно программы», as a pure function** — task T-100-3, вопрос 172.
///
/// `captured` is the executable name the one body of the capture found in front
/// (`guard::capture_foreground(guard::Asker::Process)`, task T-100-2), `None` for no window or a
/// window of this program. The answer:
///
/// * nothing found, or a blank name — [`ExclusionPick::NotPicked`];
/// * **the host of the Store frames** — [`ExclusionPick::NotPicked`] as well. The capture names the
///   program inside a frame (решение 172.20); the host's own name comes back only when that program
///   could not be told, and taking it would exclude every Store program at once — the very thing
///   the owner chose to avoid;
/// * a name longer than [`EXCLUSION_NAME_CHARS`] — [`ExclusionPick::Refused`]. The field would not
///   take it either (task Т-22-8), and `guard::publish_exclusions` would drop a longer one on «ОК»
///   without a word;
/// * otherwise **the very verdict of the manual way**, [`exclusion_verdict`]: `Add` puts it in,
///   `Duplicate` points at the row that holds it — found by the same fold — and `Refuse` refuses.
///
/// The ceiling is not an arm: the button is dark at it ([`show_exclusion_ceiling`]), and a count
/// running when the list fills up is stopped there.
///
/// Pure, and public so the test calls the very function the window calls.
pub fn exclusion_pick(captured: Option<&str>, existing: &[String]) -> ExclusionPick {
    let Some(name) = captured.map(str::trim).filter(|name| !name.is_empty()) else {
        return ExclusionPick::NotPicked;
    };

    if crate::guard::is_frame_host(name) {
        return ExclusionPick::NotPicked;
    }

    if name.encode_utf16().count() > EXCLUSION_NAME_CHARS {
        return ExclusionPick::Refused;
    }

    match exclusion_verdict(name, existing) {
        ExclusionVerdict::Add => ExclusionPick::Added(name.to_owned()),
        ExclusionVerdict::Duplicate => {
            let folded = crate::guard::fold_process_name(name);

            existing
                .iter()
                .position(|item| crate::guard::fold_process_name(item) == folded)
                .map_or(ExclusionPick::NotPicked, |row| {
                    ExclusionPick::AlreadyListed(row, existing[row].clone())
                })
        }
        ExclusionVerdict::Refuse => ExclusionPick::Refused,
    }
}

// ---------------------------------------------------------------------------------------
// The one door out of this program — finding С26, task T-41-4
// ---------------------------------------------------------------------------------------

/// Asks the shell to open `target`, and is **the one place in this program that asks anything
/// to be opened at all** — finding С26, task T-41-4.
///
/// # Why one door
///
/// Opening something is the only thing this program does that starts somebody else's code, and
/// the audit's warning about it was not hypothetical: by the time the repair arrived there were
/// **three** such places, and two of them opened the *same* journal folder by different code —
/// the button «Открыть папку журнала» of this dialog and «Сохранить в папку журнала» of the
/// letter wizard (`letters::open_folder`, FR-104). «Вторая дверь через полгода» is the shape of
/// the finding, and it had already happened. A sweep in `tests\settings.rs` keeps the count at
/// one from here on.
///
/// # ⚠ The mechanism inside the door is NOT repaired, and that is an open question, not an
/// oversight
///
/// This process carries `uiAccess`. `ShellExecuteW` asks Windows to open the thing **from
/// here**, and when that means creating a process — the shell is not up, or a third-party file
/// manager is registered for folders — **the new process inherits this program's token**: the
/// raised rights and the right to reach across the window boundary Windows otherwise keeps.
/// The installer has defended against the trap with an explicit flag since stage E9; the
/// product never said a word about it.
///
/// The repair the terms of reference ask for is to hand the job to the **already running**
/// shell, so that what comes up is its child and not ours. Both roads they name were measured
/// and **both are closed in this crate's configuration** (`scratchpad-E41\probe-shell-T-41-4.md`):
/// `SHOpenFolderAndSelectItems` is not generated at all, and neither are `IShellBrowser` and
/// `IShellView`; `IShellDispatch2` exists as a type, but **not one method of the chain does**,
/// because every one of them passes a `VARIANT` — and `VARIANT` is not generated either, with
/// the `Win32_System_Variant` feature added or without it. Adding that feature costs nothing
/// (`cargo tree` is 83 either way) and buys nothing; going further would mean moving the crate
/// version, which SEC-03 closes and only the owner may open.
///
/// So the door is one door, and what it does inside it is what it always did. The question of
/// the mechanism is the owner's — вопрос 125, дополнение **125б**.
pub fn open_in_the_shell(target: &str) -> bool {
    let wide = wide(target);

    // SAFETY: `wide` is a NUL-terminated UTF-16 buffer owned by this frame and neither moved
    // nor dropped until the call returns; the two null pointers are the documented way to pass
    // no arguments and no working directory. No owner window is passed: a door shared by three
    // callers has no one window to own the shell's error box, and the shell puts it on the
    // desktop instead — which is what the two callers in `letters.rs` already did.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    // NFR-13. `ShellExecuteW` reports failure as a value of 32 or below in what is nominally a
    // module handle — the one Win32 return value that is neither a `BOOL` nor an error code.
    if result.0 as usize <= 32 {
        crate::app::report_non_critical("ShellExecuteW", &WinError::from_thread());
        return false;
    }

    true
}

/// How long «Журнал сохранён» stands on the button after a write that succeeded — task T-39-11,
/// решение 122.5: «кнопка на 2 с говорит „Журнал сохранён“».
pub const JOURNAL_SAVED_MS: u32 = 2_000;

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

/// The line under the hotkey field when no capture is armed — task T-93-2, вопрос 157.
///
/// In order of precedence:
///
/// 1. **the warning of FR-92**, when there is one ([`hotkey_note`]) — a name this build does not
///    know, a key that types its character: what the person must not miss;
/// 2. **once, after a combination is captured**, which half stays with the programs —
///    [`IDS_CAPTURE_COMBO_NOTE`], «F12 без Ctrl остаётся программам.», the key and the modifiers
///    as the field shows them; and since task T-95-2, **once after the double press** —
///    [`IDS_CAPTURE_DOUBLE_NOTE`], «Два нажатия Shift подряд. Обычный набор с Shift не затронут.»;
/// 3. otherwise **the line at rest** — [`IDS_HOTKEY_HINT`], «Клавиша, сочетание с Ctrl, Alt,
///    Shift или двойной Shift.»: устройство А has no switch, so the ways stand where they are
///    seen (157.6).
///
/// Public so a test reads the very answer the window shows, in every language.
pub fn hotkey_line(hotkey: &Hotkey, combination_taken: bool) -> String {
    hotkey_line_in(ui_language(), hotkey, combination_taken)
}

/// [`hotkey_line`] in `language` — the window of the settings speaks its own (T-120-20); one body.
pub fn hotkey_line_in(language: Language, hotkey: &Hotkey, combination_taken: bool) -> String {
    if let Some(warning) = hotkey_note(hotkey) {
        return text_in(language, warning);
    }

    match hotkey.binding() {
        // Task T-95-2 — once after the double press is captured (the moment 5 of 157.18).
        Some((_, set)) if combination_taken && set & crate::hook::DOUBLE_TAP != 0 => {
            text_in(language, IDS_CAPTURE_DOUBLE_NOTE)
        }
        Some((vk, modifiers)) if combination_taken && modifiers != 0 => {
            let key = key_name(vk).unwrap_or_else(|| hotkey.key.trim().to_owned());

            text_in(language, IDS_CAPTURE_COMBO_NOTE)
                .replace("{0}", &key)
                .replace("{1}", &modifier_label(modifiers))
        }
        _ => text_in(language, IDS_HOTKEY_HINT),
    }
}

/// What the field says while a pair is open — «Shift…», the name of the key and the ellipsis of the
/// invitation (the moment 3 of the mock-up of 157.18). Not a string of the table: the names of keys
/// are not translated, and the ellipsis is the invitation's own.
pub fn pending_shift_name() -> String {
    format!("{DOUBLE_TAP_KEY}…")
}

/// Presses a hotkey capture refused — task T-71-3, backlog line Э36-Б-3 (finding Н9 of stage Э36).
///
/// Each one is a key a person pressed to make it the hotkey and the program did not take. The
/// reason stood under the field (FR-94), and before this task nothing of it reached a dump. The
/// number means something to a person the way `hook::lost_hotkeys` does: non-zero says a capture
/// was tried and turned a key away. It counts the same presses as the journal entries «hotkey
/// capture refused», which a ring of [`crate::diag::CAPACITY`] entries may already have evicted.
///
/// Counted where the refusal is **carried out** — [`run_capture_step`] — and never where it is
/// decided: [`capture_step`] is pure, and the tests call it thousands of times without refusing
/// anybody. A count of a deliberate act in one field of one dialog, not of typing, and nothing of
/// the key or of the reason (SEC-01, SEC-07).
static CAPTURE_REFUSALS: AtomicU32 = AtomicU32::new(0);

/// Presses a hotkey capture refused since the start — task T-71-3; see [`CAPTURE_REFUSALS`].
pub fn capture_refusals() -> u32 {
    CAPTURE_REFUSALS.load(Ordering::Relaxed)
}

/// Writes down that a hotkey capture refused a press — task T-71-3, decision 132.2.
///
/// One count and one entry of the journal, «hotkey capture refused» with `OsCode::NONE`. **Nothing
/// of the press is kept** — not the key, not the modifiers, not which reason of [`Refusal`] it was:
/// an entry of the journal has no field for any of them (SEC-07), and the person has already read
/// the reason under the field. The shape of `letters::note_link_refused`.
///
/// Public so that `tests\diag.rs` calls the very function the dialog calls. It takes no lock and
/// allocates nothing: `diag::record` is callable from every thread.
pub fn note_capture_refused() {
    CAPTURE_REFUSALS.fetch_add(1, Ordering::Relaxed);

    crate::diag::record(
        crate::diag::Operation::from_name("hotkey capture refused"),
        crate::diag::OsCode::NONE,
    );
}

// -----------------------------------------------------------------------------------------
// The list view of the cycle
// -----------------------------------------------------------------------------------------

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

/// A window that had to be shortened, and how far its contents are scrolled — task T-42-12,
/// finding 124б.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScrollView {
    /// How many pixels of the contents are above the top edge of the client area. Zero is the
    /// top; [`ScrollView::most`] is the bottom.
    pub offset: i32,
    /// The height the contents need — the client height the template asked for, before the
    /// window was shortened.
    pub content: i32,
    /// The client height the window really got: what fits the monitor.
    pub page: i32,
}

impl ScrollView {
    /// The largest offset that still shows contents — the bottom of the scroll.
    pub fn most(self) -> i32 {
        (self.content - self.page).max(0)
    }

    /// `offset` brought inside `0..=most` — every road into the scroll goes through this.
    pub fn clamped(self, offset: i32) -> i32 {
        offset.clamp(0, self.most())
    }
}

/// How tall a window may be on the monitor it opened on, and how tall it wants to be — the
/// arithmetic of task T-42-12, apart from the windows.
///
/// Answers `Some` with the client height the window has to settle for, or `None` when the
/// window fits whole and nothing is to be done — which is every window at 100 % and was the
/// only case before this task.
///
/// `frame` is what the window costs besides its client area: the caption and the two borders.
///
/// Pure, and public so the test calls the very function the window calls.
pub fn shortened_client_height(wanted_client: i32, frame: i32, work_area: i32) -> Option<i32> {
    let wanted_whole = wanted_client + frame;

    if wanted_whole <= work_area {
        return None;
    }

    // At least one pixel of client area, however small the monitor claims to be (NFR-13).
    Some((work_area - frame).max(1))
}

/// How wide the window has to be so that a scroll bar covers none of its controls — task
/// T-42-12a, finding 124б.3.
///
/// `WS_VSCROLL` takes its width out of the **client** area, and every control of this window was
/// laid out against the client width the template declared. So a window given a scroll bar
/// without being widened loses that much of its right-hand side **under the bar** — which is
/// exactly what the owner saw on `e61`: «прокрутка налезает на кнопки».
///
/// Pure, and public so the test calls the very function the window calls.
pub fn widened_for_scroll_bar(window_width: i32, bar: i32) -> i32 {
    window_width + bar.max(0)
}

/// **Where a window that has grown to its content stands** — решение 143.10 (а), задача T-83-3.
///
/// `DS_CENTER` centres a window in the work area **for the height of its template**, before
/// `WM_INITDIALOG`, and the window that then grows to its content grows **downwards** whole
/// (measured: «От автора» at 100 % stood 130 px from the top and 36 from the bottom). The rule of
/// the step (решение 143.1) weighs the tallest window as if it stood in the middle, so a grown
/// window that would not keep [`FONT_STEP_AIR`] above and below where it stands is put **in the
/// middle of the work area** — never above its top. A window that keeps the air stays exactly where
/// it stands: at 100 % on the owner's screen that is every window, and not a pixel moves.
///
/// Pure: all four numbers are pixels of the screen — the window's top and height, the work area's
/// top and bottom.
pub fn grown_window_top(top: i32, height: i32, work_top: i32, work_bottom: i32) -> i32 {
    if top - work_top >= FONT_STEP_AIR && work_bottom - (top + height) >= FONT_STEP_AIR {
        return top;
    }

    (work_top + (work_bottom - work_top - height) / 2).max(work_top)
}

/// How far one line of the scroll goes, in pixels — task T-42-12.
///
/// A row of the layout list is twelve dialog units and the tallest thing in the window that a
/// person steps past; a line of the scroll is deliberately smaller than that, so that the arrow
/// of the scroll bar moves the contents by less than one row and nothing is skipped over.
const SCROLL_LINE: i32 = 16;

/// Where the scroll goes for one `WM_VSCROLL` — the pure half of [`on_vertical_scroll`].
///
/// `command` is the low word of `wparam`, `track` its high word — the position the thumb is
/// being dragged to, which only the two `SB_THUMB*` commands carry.
///
/// Public so the test walks every command the scroll bar can send without a window.
pub fn scrolled_to(view: ScrollView, command: i32, track: i32) -> i32 {
    let offset = match command {
        v if v == SB_LINEUP.0 => view.offset - SCROLL_LINE,
        v if v == SB_LINEDOWN.0 => view.offset + SCROLL_LINE,
        v if v == SB_PAGEUP.0 => view.offset - view.page,
        v if v == SB_PAGEDOWN.0 => view.offset + view.page,
        v if v == SB_THUMBTRACK.0 || v == SB_THUMBPOSITION.0 => track,
        v if v == SB_TOP.0 => 0,
        v if v == SB_BOTTOM.0 => view.most(),
        // Every other command — `SB_ENDSCROLL` among them — leaves the scroll where it is.
        _ => view.offset,
    };

    view.clamped(offset)
}

// -----------------------------------------------------------------------------------------
// Layout names
// -----------------------------------------------------------------------------------------

/// What a whole list of layouts is called in the cycle list and in the two combo boxes.
///
/// A list and not one label at a time, and that is the substance rather than the shape: the
/// rule of решение 82.1 cannot be applied to a layout on its own — whether a name needs an
/// identifier behind it is a property of the **list** the name stands in.
///
/// `session` is what section 7 numbers a layout against, and it is passed apart from
/// `layouts` because the cycle list shows the session in the user's own order: the same set,
/// a different sequence, and the identifier must not change with the sequence.
pub fn layout_labels(layouts: &[LayoutId], session: &[LayoutId]) -> Vec<String> {
    let rows: Vec<(Option<String>, String)> = layouts
        .iter()
        .map(|layout| {
            let name = language_name(*layout);

            // The tail of a **named** layout is what section 7 stores for it, so a person who
            // reads it off the window finds the same characters in `config.toml`. A nameless
            // one keeps the raw handle it has always shown: nothing about that case changed
            // in Т-23-3, and `spec_text` would have printed a different number for it.
            let tail = match name {
                Some(_) => spec_text(*layout, session),
                None => format!("0x{:08X}", layout.raw()),
            };

            (name, tail)
        })
        .collect();

    disambiguated_labels(&rows)
}

/// The rule of решение 82.1 — **вариант В, «различитель по требованию»** — over
/// `(имя, различитель)` pairs, and nothing else.
///
/// A name that no other row of the list wears is shown bare: `Русский (Россия)`. A name two
/// or more rows wear is shown with its discriminator behind it — **on every one of them**,
/// because a tail on one of a pair says nothing about which one the reader is looking at. A
/// row the system would not name at all falls back to the bare identifier, exactly as it did
/// before this task.
///
/// ⚠ **The discriminator is a hexadecimal identifier and not the registry name of the
/// layout, and that is a deliberate choice rather than an omission.** Windows shows
/// `Win+Space` the name it keeps under
/// `HKLM\SYSTEM\CurrentControlSet\Control\Keyboard Layouts\<KLID>\Layout Text`, which would
/// read «Русская (машинопись)» where this shows a number. That road was examined and turned
/// down at вопрос 82: reaching it means mapping an `HKL` to a `KLID` by a convention
/// Microsoft documents nowhere — the low word is a language identifier only for the layouts
/// loaded under their own default, and the `0xF0xx` handles of a second layout of one
/// language are precisely the case this discriminator exists for. The price of the
/// undocumented convention was judged higher than the price of a number the user sees only
/// when two names actually collide, which on most machines is never.
///
/// Pure, and public for that reason: `tests\settings.rs` closes the rule on names no system
/// produced, without a window and without a locale service.
pub fn disambiguated_labels(rows: &[(Option<String>, String)]) -> Vec<String> {
    rows.iter()
        .map(|(name, tail)| match name {
            None => tail.clone(),
            Some(name) => {
                let alike = rows
                    .iter()
                    .filter(|(other, _)| other.as_deref() == Some(name.as_str()))
                    .count();

                if alike > 1 {
                    format!("{name} — {tail}")
                } else {
                    name.clone()
                }
            }
        })
        .collect()
}

/// The display name of the language a layout serves, asked of the system — **the name that
/// language calls itself by**, with its first letter raised.
///
/// `LOCALE_SNATIVEDISPLAYNAME` and not `LOCALE_SLOCALIZEDDISPLAYNAME` — **вопрос 92**. The
/// localised name is the one the interface language of **Windows** would use, and it follows
/// the system rather than the layout or the program: on the Russian machine of this project
/// an English layout read «Английский (США)» even in a program window switched to English,
/// which is what the user saw. The native name follows neither — «English (United States)»
/// and «Русский (Россия)» say the same thing under any Windows and in either locale of FR-94,
/// and a person who cannot read the interface language still finds their own layout in the
/// list. The road not taken — following the program's own language — was turned down at the
/// same вопрос: for a Russian name on a machine without Russian there is no honest source.
///
/// The raised first letter is the other half of вопрос 92, and it is not decoration: a
/// language writes its own name the way it writes its adjectives, and Russian writes them in
/// lower case. Measured, not remembered — `GetLocaleInfoEx` returns «**р**усский (Россия)»,
/// `U+0440`, for `ru-RU`. A list of layouts is not running prose, so [`capitalised`] puts the
/// letter back up. It stands **here**, before the name is a name at all, so everything
/// downstream — the collision rule of решение 82.1 and the discriminator it glues on — counts
/// the names it will actually show.
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
            windows::Win32::Globalization::LOCALE_SNATIVEDISPLAYNAME,
            Some(&mut display),
        )
    };

    if written <= 0 {
        return None;
    }

    let units = usize::try_from(written).ok()?.saturating_sub(1);

    Some(capitalised(&String::from_utf16_lossy(
        &display[..units.min(display.len())],
    )))
}

/// `name` with its first character raised to upper case and everything after it untouched —
/// the capitalisation вопрос 92 asks for.
///
/// The first character goes through Unicode `to_uppercase` **and the whole iterator it
/// returns**: raising a character can produce more than one — `ß` raises into `SS` — and
/// keeping only the first unit would drop the rest of the letter silently. A character of a
/// script that has no case at all comes back as it was, which is what makes this safe to run
/// over every name the locale service can produce rather than over the two this machine
/// happens to have.
///
/// Nothing else is touched: «English (United States)» keeps its inner capitals, «русский
/// (Россия)» keeps the capital of «Россия», and a name the system gave empty stays empty
/// instead of costing a panic.
///
/// Pure, and public for that reason: `tests\settings.rs` closes it on a bare string, without a
/// window and without a locale service.
pub fn capitalised(name: &str) -> String {
    let mut characters = name.chars();

    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

// -----------------------------------------------------------------------------------------
// Control helpers
// -----------------------------------------------------------------------------------------

/// A resource identifier in the `MAKEINTRESOURCE` form the dialog manager expects.
///
/// The identifier goes *into* the pointer and is never dereferenced — the same convention
/// `crate::tray` uses for the icons, and `without_provenance` is what says so in Rust.
pub(crate) fn resource_id(id: u16) -> PCWSTR {
    PCWSTR(std::ptr::without_provenance(usize::from(id)))
}

/// A NUL-terminated UTF-16 copy of `text`, for the Win32 calls that want one.
pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The low 16 bits of a message parameter.
pub(crate) fn low_word(value: usize) -> u16 {
    u16::try_from(value & 0xFFFF).unwrap_or(0)
}

/// The next 16 bits of a message parameter.
///
/// `pub(crate)` с задачи Т-46-2: код уведомления `WM_COMMAND` читают теперь оба окна с глифами
/// — это окно и окна писем, — и читают его одним телом.
pub(crate) fn high_word(value: usize) -> u16 {
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
pub(crate) fn send_to(
    hwnd: HWND,
    control: i32,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    // SAFETY: `hwnd` is the live dialog and `control` is one of the identifiers of its
    // template; a message to a control that does not exist answers zero rather than doing
    // anything. Where `lparam` is a pointer, the value it points at is owned by the caller's
    // frame for the whole call — every such call site says so.
    unsafe { SendDlgItemMessageW(hwnd, control, message, WPARAM(wparam), LPARAM(lparam)) }.0
}

/// Sets the text of one control.
pub(crate) fn set_text(hwnd: HWND, control: i32, text: &str) {
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
/// The text of one control of a window of this program — the reader the stand of Э32 uses to
/// check that the appeal of FR-104 reaches the field whole.
///
/// Public only for that: the module's own callers use [`get_text`], which this is.
pub fn text_of(hwnd: HWND, control: i32) -> String {
    get_text(hwnd, control)
}

pub(crate) fn get_text(hwnd: HWND, control: i32) -> String {
    // ⛔ **The buffer is asked of the control and is not a fixed 512.** It was 512 units until
    // task Т-32-10, which was enough for every field of this dialog and **silently cut the
    // appeal of FR-104 at 511 characters** — the live acceptance found it as a file that ended
    // in the middle of a row of dashes (`scratchpad-Э32\живое-В`, 511 знаков of 2400). A
    // reader that truncates without saying so is the worst kind: everything downstream of it
    // looks correct.
    let length =
        unsafe { windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(hwnd), control) }
            .map(|child| {
                // SAFETY: `child` is the live control just answered for this dialog.
                unsafe { windows::Win32::UI::WindowsAndMessaging::GetWindowTextLengthW(child) }
            })
            .unwrap_or(0);

    // One for the terminator `GetDlgItemTextW` always writes, and a floor of 512 so that a
    // control which answers zero still behaves as it did before.
    let mut buffer = vec![0u16; usize::try_from(length).unwrap_or(0).max(511) + 1];

    // SAFETY: `buffer` is owned by this frame and its length is what bounds the copy; the call
    // is given the slice and cannot write past its end. It answers the number of units written,
    // never more than the buffer holds.
    let copied = unsafe { GetDlgItemTextW(hwnd, control, &mut buffer) };

    let copied = usize::try_from(copied).unwrap_or(0);

    String::from_utf16_lossy(&buffer[..copied.min(buffer.len())])
}

// ⚠ **Точечная перерисовка переехала в `widgets::repaint` — задача Т-45-2.** Тела не менялись;
// здесь стояли `repaint_control` и `repaint_control_range`, теперь это
// `widgets::repaint::control` и `widgets::repaint::range`. Таблица переименований — в
// `reports\ИТОГ-Э45.md`.

// =========================================================================================
// What the old «О программе» window left for «От автора»
// =========================================================================================
//
// «О программе» is a window of the shell now (`ui::windows::about`, FR-92а). The logo below
// and the faces and pitches of the name and the body (`about_*_logfont`,
// `about_body_line_pitch`) are still drawn with by «От автора» (`letters`), whose head is the
// head the old window had; they go when that window moves to the shell (T-120-17).

/// The size of the about window's logo in **mock-up pixels** — finding A-08, task T-12-4.
///
/// The generator's literal: `Draw-AppIcon $g ($aX + 28) ($aY + 76) 56` —
/// `scratchpad-Э11\chrome.ps1:193`. Through [`scaled`] that is **40 px** at 96 DPI, which is
/// the number the protocol measured the mock-up at and the number the window used to miss by
/// eight: an `SS_ICON` static loads the 32 px frame of the `.ico` and ignores the units its
/// statement declares, so the template could not ask for 40 however it was written.
pub const ABOUT_LOGO_SIZE: i32 = 56;

// =========================================================================================
// The window kit `letters` builds its own windows out of — task Т-32-3, §6.2
// =========================================================================================
//
// ⚠ **Twenty of this module's own helpers are `pub(crate)` since task Т-32-3, and each of them
// says so where it is defined.** They are the pen the windows of FR-101 and FR-103 are drawn
// with: the rounded panel, the tracked caption, the push button with its four states, the
// colour answers to `WM_CTLCOLOR*`, the hover record, the title-bar theme, the small helpers
// that write a control's text and repaint it.
//
// SPEC section 6.2 gives those windows to `letters` — «окно «От автора», мастер обращения» —
// and gives the dialog machinery to this module. Between the two there is a seam, and the
// alternative to lending across it was a second copy of all of it: this project has a rule
// about that, and task T-14-3 already spent a whole task moving a drawing library out of this
// file into `theme` rather than let `tray` keep a copy of the pixels. **One body, not a copy.**
//
// ⛔ **`pub(crate)` and never `pub`.** None of it is API of the crate; it is one module lending
// another the tools it already had. A test that wants any of it goes through the window using
// it. (A `pub(crate) use` re-export was tried first and refused by the compiler — E0364/E0365:
// a private item cannot be re-exported at a wider visibility, it has to *be* wider.)

/// Puts [`combo_box_proc`] in front of one combo box of **another** window of this program —
/// task Т-32-8, the wizard of FR-104.
///
/// The far half is [`unsubclass_foreign_combo`], and the procedure additionally takes itself
/// off on `WM_NCDESTROY`, exactly as it does for the four of this dialog.
pub(crate) fn subclass_foreign_combo(hwnd: HWND, control: i32) {
    // SAFETY: `hwnd` is a live window of this thread; the crate turns a missing control into
    // an error.
    let Ok(combo) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
        crate::app::report_non_critical("GetDlgItem", &WinError::from_thread());
        return;
    };

    // SAFETY: `combo` is a live control created by the dialog manager on this thread, and
    // `combo_box_proc` has exactly the signature `SUBCLASSPROC` names. The reference datum is a
    // plain number. ⚠ NFR-13: the `BOOL` is dropped for the reason `subclass_combo_boxes`
    // gives at its own — a refusal leaves that one combo box painted by the system, which is
    // visible rather than silent.
    let _ = unsafe {
        SetWindowSubclass(
            combo,
            Some(combo_box_proc),
            COMBO_SUBCLASS_ID,
            COMBO_FOREIGN,
        )
    };
}

/// Takes [`combo_box_proc`] back off a combo box of another window — the far half of the pair.
pub(crate) fn unsubclass_foreign_combo(hwnd: HWND, control: i32) {
    // SAFETY: as above.
    let Ok(combo) = (unsafe { GetDlgItem(Some(hwnd), control) }) else {
        return;
    };

    // SAFETY: the procedure and the identifier are the pair the subclass was installed with;
    // a window that no longer carries it answers `FALSE` and nothing happens.
    let _ = unsafe { RemoveWindowSubclass(combo, Some(combo_box_proc), COMBO_SUBCLASS_ID) };
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
    reference_data: usize,
) -> LRESULT {
    match message {
        // The whole reason this procedure exists — see the ⚠ above.
        //
        // SAFETY: `combo` is the control this procedure is installed on, and the call is
        // inside the window's own `WM_PAINT`, which is where `BeginPaint` may be used.
        WM_PAINT => {
            if unsafe { paint_combo_closed_part(combo, reference_data) } {
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
unsafe fn paint_combo_closed_part(combo: HWND, reference_data: usize) -> bool {
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
    unsafe { draw_combo_closed_part(combo, dc, reference_data) };

    // SAFETY: the same window and the very `PAINTSTRUCT` `BeginPaint` filled in. The `BOOL`
    // is dropped for the reason the paint calls of this file drop theirs.
    let _ = unsafe { EndPaint(combo, &paint) };

    true
}

unsafe fn draw_combo_closed_part(combo: HWND, dc: HDC, reference_data: usize) {
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

    let foreign = reference_data == COMBO_FOREIGN;

    // The same gate the owner-draw handlers keep (SEC-05), even though this procedure can only
    // be reached on a window it was installed on: one list of four, checked before any work.
    //
    // ⛔ A combo of **another** window of this program is exempt from the list and from
    // nothing else — and the exemption is the reference datum the subclass was installed with,
    // which no message can forge: `SetWindowSubclass` is the only thing that writes it.
    if !foreign && !COMBO_BOXES.contains(&control) {
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

    // Every combo box this draws belongs to a window of the letters — the settings window of FR-92
    // is a window of the shell since T-120-20: its look is built out of the palette the
    // configuration resolves to — the very palette that window built itself from — and the
    // brushes are owned for the length of this paint.
    let setting = crate::tray::with_tray(|tray| tray.config().general.theme)
        .unwrap_or(theme::ThemeSetting::System);
    let palette = theme::resolve(setting, theme::system_is_light());
    let brushes = theme::Brushes::new(palette);
    let choice = brushes
        .as_ref()
        .map(|brushes| combo_look(roles, palette, brushes, brushes.window_bg(), None));

    // `_hot` is named and not discarded on purpose: it owns the transient brush `fill` may name,
    // and a `_` would have freed it before the first pixel was drawn.
    let Some((ground, fill, _hot, border, ink, chevron_ink, face)) = choice else {
        return;
    };

    let dpi = drawing_dpi(dc);

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
