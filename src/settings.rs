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

use windows::Win32::Foundation::{COLORREF, ERROR_FILE_NOT_FOUND, HMODULE, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FONT_WEIGHT, FW_BOLD, FillRect, GetDC, GetMonitorInfoW, HDC,
    LOGFONTW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow, ReleaseDC,
};
use windows::Win32::System::LibraryLoader::{
    FindResourceExW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::UI::Controls::LVIS_STATEIMAGEMASK;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VIRTUAL_KEY, VK_APPS, VK_CAPITAL, VK_CONTROL, VK_DELETE, VK_END, VK_ESCAPE, VK_F1,
    VK_HOME, VK_INSERT, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_NUMLOCK,
    VK_PAUSE, VK_PRIOR, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SCROLL, VK_SHIFT,
    VK_SNAPSHOT,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetDlgItemTextW, GetWindowRect, IDOK, SB_BOTTOM, SB_LINEDOWN, SB_LINEUP,
    SB_PAGEDOWN, SB_PAGEUP, SB_THUMBPOSITION, SB_THUMBTRACK, SB_TOP, SW_SHOWNORMAL, WM_APP,
    WNDPROC,
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
    self, ButtonBorderRole, ButtonColors, ButtonFaceRole, ButtonTextRole, CheckMark, ThemeSetting,
    check_frame_colors, dc_dpi, draw_check_mark, paint_rounded, paint_selection_stripe, scaled,
    smoothed_logfont,
};

// The file of the settings — its schema, its model, its reading and writing: `config.rs` since task
// T-120-23.
#[path = "config.rs"]
mod config;
pub use config::*;

/// Serde default for `hotkey.key`.
fn default_hotkey_key() -> String {
    "Pause".to_owned()
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
/// «Нашли ошибку в переводе?» — the button under the language, task T-120-22 (решение
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
/// «Подробности для автора» — the button that opens the lines of the developer.
pub const IDS_STATUS_DETAILS: u16 = 3288;
/// «Копировать» — the window of «Подробности для автора» puts its lines on the clipboard (T-120-21,
/// C6 (а)); 3295, the reserve the controller named.
pub const IDS_STATUS_COPY: u16 = 3295;
/// «Запуск, звук, флаг в трее, язык и вид программы» — the subtitle of «Общие» (T-120-20, C3 of the controller).
pub const IDS_SECTION_GENERAL_SUB: u16 = 3307;
/// «Одна клавиша исправляет набранный текст и переключает раскладку» — the subtitle of «Горячая клавиша» (T-120-20, C3 of the controller).
pub const IDS_SECTION_HOTKEY_SUB: u16 = 3308;
/// «Между какими раскладками исправлять текст» — the subtitle of «Раскладки» (T-120-20, C3 of the controller).
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
/// «Получено сайтом автора» — the line of the state of the page «Обращение отправлено» of the
/// wizard (T-120-19); shown only when the site answered with success.
pub const IDS_WIZARD_RECEIVED: u16 = 3313;
/// The hint under «Флаг раскладки в трее» in «Общие» — AUDIT-180 Н4 (решение 208.209), the line of
/// the mock-up of the window.
pub const IDS_TRAY_FLAG_HINT: u16 = 3314;
/// The hint under «Вести журнал» in «Диагностика» — AUDIT-180 Н10 (решение 208.209), the line of the
/// mock-up: what the journal holds.
pub const IDS_LOG_HINT: u16 = 3315;
/// The third line of the first card of the wizard — where it leads (AUDIT-180 М2, решение 208.132):
/// «Дальше: где, что делали, что приложить, проверка».
pub const IDS_WIZARD_CARD_WRONG_NEXT: u16 = 3316;
/// The same of the card of the idea: «Дальше: идея и проверка».
pub const IDS_WIZARD_CARD_IDEA_NEXT: u16 = 3317;
/// The state in the head of the menu of the icon, in small letters as the mock-up of the menu has
/// it (AUDIT-180 Мн1): «работает», «приостановлена», «нужен перезапуск». The block «Состояние» of
/// the settings keeps [`IDS_STATUS_WORKING`] and its two, which begin a line.
pub const IDS_MENU_WORKING: u16 = 3318;
pub const IDS_MENU_SUSPENDED: u16 = 3319;
pub const IDS_MENU_RESTART: u16 = 3320;
/// The title of «От автора» — the first sentence of the owner's text, without its full stop
/// (AUDIT-180 А1, решение 208.211: «разбить без потерь»); the rest is [`IDS_AUTHOR_TEXT`], word for
/// word.
pub const IDS_AUTHOR_TITLE: u16 = 3321;
/// The sentence at the end of the lead of «Спасибо» that carries its link (AUDIT-180 Пс2, решение
/// 208.217): the first sentence of the old [`IDS_THANKS_PARA`]; the words of the link between
/// `<a>` and `</a>` (`ui::scene::linked`) — the group or GitHub Issues,
/// `letters::links::feedback_url` of the language.
pub const IDS_THANKS_SAY: u16 = 3322;
/// «Письма от автора» — the title of «Последние письма» under its eyebrow, which is the caption
/// [`IDS_LETTERS_CAPTION`] (AUDIT-180b Н4, решение 208.227: the head of the mock-up).
pub const IDS_LETTERS_TITLE: u16 = 3323;
/// The subtitle of «Последние письма»: what the list holds (AUDIT-180b Н4, решение 208.227).
pub const IDS_LETTERS_SUB: u16 = 3324;
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
/// The text of the hint on Windows 10 — Z11 of AUDIT-180 (решение 208.209): the path to «Выберите
/// значки, отображаемые в панели задач»; [`IDS_HINT_TEXT`] holds Windows 11's.
pub const IDS_HINT_TEXT_10: u16 = 3340;
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
/// «Через неделю» — the one snooze of FR-101: the button of the letter «Спасибо» and the second of
/// its knock (решение 208.112 п. 1, Z13 of AUDIT-180: the two name it alike).
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
// 3242 — «Открыть группу» / «Open GitHub Issues», the link under «Спасибо» until AUDIT-180 Пс2
// (решение 208.217): the link went into the lead, [`IDS_THANKS_SAY`]. A hole, never to be taken.
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
// ⚠ **3163 and 3164 are retired, not free** — AUDIT-180 Z8, решение 208.131: the card «Ничего не
// произошло» and its line went into the first one («…или ничего не произошло»). The numbers stay
// holes, as 3004 does: a key of an old translation must not come back meaning something else.
/// The second card: «Хочу предложить улучшение» — the short road.
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
/// a person where they can write, and its button got a word of its own, `IDS_FEEDBACK_OPEN`
/// (3242; gone with AUDIT-180 Пс2).
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
///
/// ⚠ **Two hundred and seventy-five since task T-120-19** — the line of the state of the
/// thanks of the wizard, [`IDS_WIZARD_RECEIVED`] (3313). 274 + 1.
///
/// ⚠ **Two hundred and seventy-seven since AUDIT-180 Z6** — the hints of the flag and of the
/// journal, [`IDS_TRAY_FLAG_HINT`] (3314) and [`IDS_LOG_HINT`] (3315). 275 + 2.
///
/// ⚠ **Two hundred and seventy-seven again since AUDIT-180 Z8** — the card «Ничего не произошло» and
/// its line (3163, 3164) went into the first card (208.131), and the two cards took the line of
/// where they lead, [`IDS_WIZARD_CARD_WRONG_NEXT`] (3316) and [`IDS_WIZARD_CARD_IDEA_NEXT`] (3317)
/// (208.132). 277 − 2 + 2.
///
/// ⚠ **Two hundred and eighty since AUDIT-180 Z14** — the state in the head of the menu in small
/// letters, [`IDS_MENU_WORKING`] and its two (3318…3320). 277 + 3.
///
/// ⚠ **Two hundred and eighty-one since AUDIT-180 Z4 (А1)** — the title of «От автора»,
/// [`IDS_AUTHOR_TITLE`] (3321), the first sentence of the owner's text (208.211). 280 + 1.
///
/// ⚠ **Two hundred and eighty-one again since AUDIT-180 Пс2** (решение 208.217) — the link of
/// «Спасибо» went into its lead: `IDS_FEEDBACK_OPEN` (3242) left, [`IDS_THANKS_SAY`] (3322)
/// came. 281 − 1 + 1.
///
/// ⚠ **Two hundred and eighty-two since the merge of the fixes of AUDIT-180** — Z11 (решение
/// 208.209), made beside them: the text of the hint of the flag on Windows 10,
/// [`IDS_HINT_TEXT_10`] (3340). 281 + 1.
///
/// ⚠ The knock of «Спасибо» names its second button with the letter's own [`IDS_THANKS_SNOOZE`]
/// (решение 208.112 п. 1: the knock and the letter name their buttons alike) — the 3341 of Z13
/// left before any release, a hole.
///
/// ⚠ **Two hundred and eighty-four since AUDIT-180b Н4** (решение 208.227) — the head of «Последние
/// письма» as the mock-up has it: [`IDS_LETTERS_TITLE`] (3323) and [`IDS_LETTERS_SUB`] (3324).
/// 282 + 2.
pub const INTERFACE_STRINGS: [u16; 284] = [
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
    IDS_LETTERS_TITLE,
    IDS_LETTERS_SUB,
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
    IDS_WIZARD_RECEIVED,
    IDS_TRAY_FLAG_HINT,
    IDS_LOG_HINT,
    IDS_WIZARD_CARD_WRONG_NEXT,
    IDS_WIZARD_CARD_IDEA_NEXT,
    IDS_MENU_WORKING,
    IDS_MENU_SUSPENDED,
    IDS_MENU_RESTART,
    IDS_AUTHOR_TITLE,
    IDS_THANKS_SAY,
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
    IDS_HINT_TEXT_10,
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

/// The module [`text`] reads from — and the icon on the caption of the windows of the shell
/// (`ui::host`), which lives in the same resources.
pub(crate) fn resource_module() -> HMODULE {
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

// FR-94 — capturing the hotkey by pressing it: `hotkey_capture.rs` since task T-120-23.
#[path = "hotkey_capture.rs"]
mod hotkey_capture;
pub use hotkey_capture::*;

/// `Shift` in its three forms — the neutral code the queue reports for both keys and the two sided
/// ones a keyboard can send. The key of the double press (task T-95-2): left and right are one.
const SHIFT_KEYS: [u16; 3] = [VK_SHIFT.0, VK_LSHIFT.0, VK_RSHIFT.0];

// =========================================================================================
// The mirror — вопрос 97, task Т-30-2. Both windows of the program go through here.
// =========================================================================================

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

// =========================================================================================
// FR-92а, task T-12-1 — the icon of the caption
// =========================================================================================

// ⚠ **Полная перерисовка окна переехала в `widgets::repaint::whole` — задача Т-45-2.** Тело не
// менялось; здесь стояла `repaint_whole_window`. Названа она была за то, что делает, а не за
// первый повод, ради которого написана: до появления второго вызывающего это была
// `repaint_after_palette_change`.

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
