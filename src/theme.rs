//! The theme of everything visible: the two palettes of FR-92а, the system switch that
//! chooses between them, and the brushes the windows will paint with.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC: «Палитры FR-92а,
//! чтение `AppsUseLightTheme`, кисти и отрисовка элементов по `WM_DRAWITEM` /
//! `WM_CTLCOLOR*`». Task T-11-1 laid the foundation — the setting, the palettes, the
//! reading of the system switch, the resolution of the two into one palette, and the brush
//! owner.
//!
//! # The drawing library lives here — task T-14-3
//!
//! ⚠ **The sentence this file used to carry — that the handlers which *paint* arrive «inside
//! the modules that own the windows being painted» — is out of date, and it was the whole of
//! finding 24 of the audit of 2026-08-24.** What it produced was a drawing engine grown inside
//! `settings`, and `tray` reaching into `settings` for pixels: the scale of the mock-ups, the
//! supersampling surface, the rounded corner, the check mark, the grey-antialiased face. None
//! of that is about a configuration file, and none of it knows which window it is drawing.
//!
//! Since task T-14-3 this module owns it. The second half of the file — everything below the
//! brushes — is that library: how a length of the mock-ups becomes a pixel of a window at a
//! DPI, how an edge is smoothed, how a rounded figure, an ellipse and a polyline are laid
//! down, and how a role of a control becomes a field of [`Palette`]. What stays in the modules
//! that own the windows is **which** element is being drawn — the identifiers of a template,
//! the state of a dialog, the entries of a menu — and they call in here for the drawing of it.
//!
//! The dependency runs one way and is meant to be read as a rule: **nothing in this file names
//! anything of `settings`, of `tray` or of `app`.** A drawing routine that needed a control
//! identifier would be a routine that belongs on the other side of that line.
//!
//! Task T-14-5 carried the second slice over: the palette question the caption is coloured by,
//! the colour table of a list tick, the selection stripe, the caption underline, the chevron of
//! a closed combo box, and the pure arithmetic of a pitch, an advance and the air around a
//! frame. The rule above is what decided each of them, and it decided them **through the
//! body**, not through the signature: a routine whose arithmetic reads a length that a drawing
//! of the owning module also reads was left where it was, because moving it would have moved
//! that length in here — and that is the same back-reference the rule forbids, written the
//! other way round.
//!
//! Task T-14-7 closed the move with the third slice: the painting of **one caption into one
//! rectangle** — its ground, its face, the wrap `DrawTextW` itself decided and the band each
//! line is laid in, with the two numbers only that painting reads. Both windows of the program
//! share the one body. The three units the second slice named as candidates and could not take
//! stayed where they were, each for a reason of the same rule: `settings::draw_check_glyph`
//! reaches for the tick figure of *its* list (`settings::LIST_CHECK_MARK`, whose twin
//! `tray::MENU_CHECK_MARK` lives with *its* menu — the instances belong to the owners of the
//! windows, and only the figure itself is here); `settings::draw_cycle_row` reads a table over
//! `layouts::LayoutMode` and calls two routines that stayed; and the pair of `LOGFONTW`
//! derivations of the settings module divide by a percentage that a drawing of that module
//! reads as well. What is left in `settings` and in `tray` is what knows whose window it is in.
//!
//! # The single owner of the palettes — FR-92а
//!
//! FR-92а names this module the single owner of the palettes: every colour of the theme is
//! a named field of [`Palette`], written here and nowhere else. No other file may hold a
//! theme colour as a number — a handler that needs one asks the palette for it by name, and
//! the acceptance instrument of position 25 (section 11.3) checks pixels of the running
//! program against these very constants, which only means anything while they have no
//! copies.
//!
//! # No global state
//!
//! The module does not hold a "current theme". [`resolve`] is a pure function of its two
//! arguments; who remembers the setting (the configuration key `[general].theme` of section
//! 7 — module `settings`, task T-11-2) and when to re-read the system switch (the
//! `WM_SETTINGCHANGE` handler of task T-11-9) is the callers' business. A module-level
//! current theme would be a second copy of what the configuration already says, and section
//! 6.3 is a chapter about why such copies go stale.
//!
//! # The registry — read and never written
//!
//! [`system_is_light`] and, since task T-86-2, [`taskbar_is_light`] are the two registry
//! reads of this module, and both go through one body: a single `RegGetValueW` of
//! `AppsUseLightTheme` or of `SystemUsesLightTheme` under the same key. Nothing here writes to
//! the registry under any circumstance — the whole of what this program writes there is the
//! autostart value of FR-93, and that belongs to module `settings`.
//!
//! # SEC-05
//!
//! The last sentence of SEC-05 covers the messages this theme reacts to: they carry no
//! privileged action, and the reaction is limited to re-reading the system setting and
//! repainting. This module is sized to exactly that reaction — it hands a handler a palette
//! to look up and brushes to repaint with, and nothing here takes a pointer, a string or a
//! command out of any message.

use windows::Win32::Foundation::{COLORREF, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CLEARTYPE_QUALITY, COLORONCOLOR,
    CreateCompatibleDC, CreateDIBSection, CreatePen, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    ExcludeClipRect, GdiFlush, GetDeviceCaps, GetLayout, HBITMAP, HBRUSH, HDC, HGDIOBJ,
    IntersectClipRect, LAYOUT_RTL, LOGFONTW, LOGPIXELSY, PS_SOLID, Polyline, RestoreDC, RoundRect,
    SRCCOPY, SaveDC, SelectObject, SetStretchBltMode, StretchBlt,
};
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::core::{PCWSTR, w};

// =========================================================================================
// The setting — the three values of `[general].theme`
// =========================================================================================

/// The three values the configuration key `[general].theme` takes — FR-92а, section 7.
///
/// The key itself — reading it from the file, writing it back, its place in the schema —
/// belongs to module `settings` and arrives with task T-11-2. This type is the vocabulary
/// the two modules will share, defined on the owning side of the palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeSetting {
    /// Follow the system's `AppsUseLightTheme`: the palette is chosen by [`resolve`] from
    /// what [`system_is_light`] reads, and a change of the system setting is applied to
    /// open windows without a restart (task T-11-9). The default of FR-92а.
    System,
    /// The light palette [`FOG`], fixed, independent of the system.
    Light,
    /// The dark palette [`GRAPHITE`], fixed, independent of the system.
    Dark,
}

impl ThemeSetting {
    /// The setting a configuration string names.
    ///
    /// The three literals of section 7 and nothing else. Anything other than `"system"`,
    /// `"light"` and `"dark"` — an empty string, an unknown word, a different letter case —
    /// reads as [`ThemeSetting::System`], the default FR-92а names. Section 7 requires the
    /// program to survive garbage in the configuration silently, so there is no error path
    /// here at all: every string is an answer.
    pub fn from_config_str(text: &str) -> Self {
        match text {
            "light" => Self::Light,
            "dark" => Self::Dark,
            // `"system"` in so many words, and every other string by the default of FR-92а.
            _ => Self::System,
        }
    }

    /// The configuration string of this setting — the inverse of [`from_config_str`].
    pub fn as_config_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

// =========================================================================================
// The palettes — FR-92а, the colours themselves
// =========================================================================================

/// One colour as `COLORREF`, from the R,G,B triple it is written down as.
///
/// ⚠ `COLORREF` is `0x00BBGGRR`: the byte order is the *reverse* of the written triple.
/// Packing it the written way round would not fail to build or to run — it would silently
/// paint everything in someone else's colours. That is why the tests of `tests\theme.rs`
/// recompute every field from the R,G,B triples with an expression of their own instead of
/// calling this function or reading the constants back.
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// Every colour one theme needs — FR-92а, one struct per palette, one field per role.
///
/// The two palettes of the program are the two constants [`GRAPHITE`] and [`FOG`] — Rust
/// `static`s, deliberately: a `const` is inlined at every use and every `&` of it is a
/// fresh address, while [`resolve`] answers `&'static Palette` precisely so that a caller
/// holds *the* palette and not a copy — the acceptance instrument of position 25 compares
/// pixels against these very values, and identity is what keeps that comparison meaning
/// something. There is no third palette: a `Palette` is not built at run time, it is
/// picked by [`resolve`].
/// The fields are the roles the drawing tasks T-11-4…T-11-11 paint by; every one of them is
/// `COLORREF` because that is the currency of `WM_CTLCOLOR*`, `SetTextColor`, `SetBkColor`
/// and the four bytes of a DWM colour attribute, and converting at every use would be
/// eighteen more places to reverse the bytes wrongly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Background of a dialog.
    pub window_bg: COLORREF,
    /// Background of a title bar — the fill DWM is asked for by `DWMWA_CAPTION_COLOR`
    /// (task T-12-1), in both windows of the program.
    pub title_bg: COLORREF,
    /// The ink of the caption text — the colour DWM is asked for by `DWMWA_TEXT_COLOR`
    /// (task T-12-1), in both windows of the program.
    ///
    /// # Why this is not [`Palette::text`]
    ///
    /// The two are near neighbours in «Графите» — 232,234,236 against 228,231,234 — and
    /// far apart in nothing at all in «Тумане», where both are 35,38,43. It would be
    /// tempting to call that one role. It is not one role: `text` is the ink of the client
    /// area, chosen against [`Palette::window_bg`] and [`Palette::panel_bg`], while this
    /// one is chosen against [`Palette::title_bg`] and is handed to a part of the window
    /// this program does not paint — the non-client caption, drawn by DWM out of the four
    /// bytes the attribute carries. A field of its own is what lets the caption move
    /// without dragging every label of the dialog with it.
    ///
    /// # Where the two literals come from
    ///
    /// The mock-up generator `scratchpad-Э11\ui.ps1`, the same source the rest of this
    /// palette was copied from and the one the acceptance instrument compares the running
    /// program against: «Графит» — `ui.ps1:177`, `TitleFg=(Col 232 234 236)`; «Туман» —
    /// `ui.ps1:188`, `TitleFg=(Col 35 38 43)`.
    pub title_fg: COLORREF,
    /// Fill of a group box.
    pub panel_bg: COLORREF,
    /// Frame of a group box, and separators.
    pub panel_border: COLORREF,
    /// The main text.
    pub text: COLORREF,
    /// Explanatory captions beside the main text.
    pub text_muted: COLORREF,
    /// The ink of a panel heading — «ОБЩИЕ», «ГОРЯЧАЯ КЛАВИША» and the six others.
    ///
    /// # Why this is not [`Palette::text_muted`]
    ///
    /// The two roles say different things and only *look* like one role in the dark. A
    /// muted text is an explanation standing beside something else — the note «Цикл:
    /// галочка — участие, кнопки — порядок», the path of the journal folder — and it is
    /// quieter than the main text on purpose, because it is read second. A panel heading is
    /// not read second: it is the name of a block, set in upper case, smaller, bolder and
    /// letter-spaced, and what it needs from a colour is to be *distinguishable from the
    /// block's own text* while still carrying the weight of a title. Those two demands land
    /// on the same number in «Графите» and on different numbers in «Тумане», which is the
    /// whole reason the role has to exist as a field instead of as a coincidence: on a white
    /// panel a heading set in the muted ink reads as one more footnote, and the mock-ups
    /// lift it back up by twelve levels.
    ///
    /// # Where the two literals come from
    ///
    /// Both are read off the mock-up generator `scratchpad-Э11\ui.ps1`, which is the source
    /// the acceptance instrument compares the running program against — the same file the
    /// rest of this palette was copied from:
    ///
    /// * «Графит» — `ui.ps1:179`, `Cap=(Col 152 160 168)`. Equal to `Muted` on that same
    ///   line, and *deliberately written out anyway*: the equality is a property of one
    ///   palette, not of the roles, and a field that borrowed the other's number would hide
    ///   the very difference the light palette makes visible.
    /// * «Туман» — `ui.ps1:190`, `Cap=(Col 122 130 139)`, against `Muted=(Col 110 118 127)`
    ///   on the same line: twelve levels lighter in every channel.
    pub cap: COLORREF,
    /// Background of input fields and lists.
    pub field_bg: COLORREF,
    /// Frame of an input field.
    pub field_border: COLORREF,
    /// Face of a button.
    pub button_bg: COLORREF,
    /// Frame of a button.
    pub button_border: COLORREF,
    /// The accent: face of the default button, mark of a checked box.
    pub accent_bg: COLORREF,
    /// Text or check mark drawn on top of the accent.
    pub accent_fg: COLORREF,
    /// Frame of an unchecked check box or radio button.
    pub box_border: COLORREF,
    /// Background of the selected row of a list.
    pub sel_bg: COLORREF,
    /// Text of the selected row of a list.
    pub sel_fg: COLORREF,
    /// Highlight of a push button and of the closed part of a combo box under the cursor.
    ///
    /// ⚠ **The tray menu no longer lights with this field** — it has [`Palette::menu_hover_bg`]
    /// of its own since the wave below. The name is kept because task T-12-8 chose it and the
    /// dialog's own two users have not changed.
    pub hover_bg: COLORREF,
    /// Highlight of a menu item under the cursor.
    ///
    /// # Почему у меню своё поле, а не общее с кнопкой
    ///
    /// Until this field the menu lit with [`Palette::hover_bg`], the very field the button
    /// lights with — the same number, byte for byte. It looked nothing alike, and the reason is
    /// the ground and not the colour: a button stands on [`Palette::button_bg`] and a menu entry
    /// on [`Palette::window_bg`]. In «Туман» that is 255,255,255 against 237,239,242, so one
    /// and the same 234,238,242 reads as a highlight on the button and vanishes on the menu —
    /// **3/1/0 levels of difference, measured.**
    ///
    /// So the two are given different numbers **in order to look the same**: this field stands
    /// off the ground of the menu by 21/17/13, exactly as far as the face of a hot button
    /// stands off its own rest. In «Графит» the ground is dark, the old value already stood off
    /// by 20/23/26, and nothing had to move — the two fields hold the same number there, the
    /// way [`Palette::panel_bg`], [`Palette::field_bg`] and [`Palette::button_bg`] already do
    /// in «Туман».
    pub menu_hover_bg: COLORREF,
}

/// The dark palette — «Графит» of FR-92а.
pub static GRAPHITE: Palette = Palette {
    window_bg: rgb(32, 35, 41),
    title_bg: rgb(26, 29, 34),
    title_fg: rgb(232, 234, 236),
    panel_bg: rgb(39, 43, 50),
    panel_border: rgb(54, 59, 67),
    text: rgb(228, 231, 234),
    text_muted: rgb(152, 160, 168),
    cap: rgb(152, 160, 168),
    field_bg: rgb(25, 28, 33),
    field_border: rgb(58, 64, 72),
    button_bg: rgb(46, 51, 59),
    button_border: rgb(63, 69, 78),
    accent_bg: rgb(228, 231, 234),
    accent_fg: rgb(27, 30, 35),
    box_border: rgb(86, 96, 112),
    sel_bg: rgb(60, 66, 76),
    sel_fg: rgb(240, 242, 244),
    hover_bg: rgb(52, 58, 67),
    // Тот же номер, что и `hover_bg`, и это не описка: на тёмной земле меню
    // (`window_bg` 32,35,41) он уже отстоит на 20/23/26 — двигать было нечего.
    menu_hover_bg: rgb(52, 58, 67),
};

/// The light palette — «Туман» of FR-92а.
pub static FOG: Palette = Palette {
    window_bg: rgb(237, 239, 242),
    title_bg: rgb(247, 248, 250),
    title_fg: rgb(35, 38, 43),
    panel_bg: rgb(255, 255, 255),
    panel_border: rgb(225, 229, 234),
    text: rgb(35, 38, 43),
    text_muted: rgb(110, 118, 127),
    cap: rgb(122, 130, 139),
    field_bg: rgb(255, 255, 255),
    field_border: rgb(207, 213, 220),
    button_bg: rgb(255, 255, 255),
    button_border: rgb(207, 213, 220),
    accent_bg: rgb(43, 47, 54),
    accent_fg: rgb(245, 246, 247),
    box_border: rgb(168, 176, 185),
    sel_bg: rgb(228, 232, 237),
    sel_fg: rgb(35, 38, 43),
    hover_bg: rgb(234, 238, 242),
    // Отстоит от земли меню (`window_bg` 237,239,242) на 21/17/13 — ровно настолько,
    // насколько лицо горячей кнопки отстоит от её покоя `button_bg` 255,255,255.
    // Прежнее общее значение давало здесь 3/1/0, то есть полосы не было видно вовсе.
    menu_hover_bg: rgb(216, 222, 229),
};

// =========================================================================================
// The system switches — FR-92а, `AppsUseLightTheme`; FR-90, `SystemUsesLightTheme`
// =========================================================================================

/// The subkey of `HKEY_CURRENT_USER` that holds the system's two theme switches — the one of
/// the applications (FR-92а) and, since task T-86-2, the one of the taskbar (FR-90).
///
/// ⚠ **Read and never written.** The one registry access of this module is the single
/// `RegGetValueW` in [`personalize_switch_is_light`], behind [`system_is_light`] and
/// [`taskbar_is_light`]; writing the user's personalization settings is not this program's
/// business under any requirement, and the session's permissions say the same thing.
const PERSONALIZE_KEY_PATH: PCWSTR =
    w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");

/// Name of the value under [`PERSONALIZE_KEY_PATH`] that FR-92а names.
const APPS_USE_LIGHT_THEME_VALUE: PCWSTR = w!("AppsUseLightTheme");

/// Name of the value under the same key that says whether the **taskbar** is light — the switch
/// FR-90 follows since task T-86-2 (решение 145.1). Windows moves it together with the
/// applications' switch under «Параметры → Персонализация → Цвета → Выберите режим», and apart
/// from it under «Выберите режим Windows по умолчанию» — which is exactly why the tray asks this
/// value and not the other: the icon sits on the taskbar, not in a window of ours.
const SYSTEM_USES_LIGHT_THEME_VALUE: PCWSTR = w!("SystemUsesLightTheme");

/// Whether the system asks applications for a light theme — FR-92а, one `REG_DWORD` read.
///
/// Any answer but a readable zero is the light theme: a non-zero value says light in so
/// many words, and a missing value, a missing key or any other refusal to read is the
/// ordinary state of a profile whose user never touched the switch — FR-92а reads it as
/// «отсутствие значения читается как светлая тема». Deliberately *not pure*: this is one of
/// the two functions of the module that look at the machine, kept apart from [`resolve`] so
/// that the latter can be verified by a table and this one only has to answer without
/// falling over.
pub fn system_is_light() -> bool {
    personalize_switch_is_light(APPS_USE_LIGHT_THEME_VALUE)
}

/// Whether the system paints the **taskbar** light — FR-90, task T-86-2 (решение 145.1): one
/// `REG_DWORD` read of `SystemUsesLightTheme`.
///
/// The tray asks this before it hands the shell an icon. On a dark taskbar the plate of the main
/// set melts into the bar (finding 144б: RGB(43, 47, 54) against RGB(45, 45, 45), the outer ring
/// of the 16 px frame half transparent), so there the icon comes out of the darkbar set — the
/// glyph alone, no plate — and on a light one out of the main set, as it always did. The rule
/// of the answer is [`system_is_light`]'s, and so is the reader: a readable zero is dark, and
/// everything else — a non-zero value, a missing value, a missing key, any refusal — is light,
/// which is the set the program has always shown. `[general].theme` and `AppsUseLightTheme`
/// have no say here: they are about the windows of this program, and the icon sits on the
/// taskbar.
pub fn taskbar_is_light() -> bool {
    personalize_switch_is_light(SYSTEM_USES_LIGHT_THEME_VALUE)
}

/// One `REG_DWORD` of [`PERSONALIZE_KEY_PATH`] read as a theme switch — the one body behind
/// [`system_is_light`] and [`taskbar_is_light`], and the one `RegGetValueW` of this module.
fn personalize_switch_is_light(value_name: PCWSTR) -> bool {
    let mut value: u32 = 0;
    let mut size = size_of::<u32>() as u32;

    // SAFETY: both names are NUL-terminated `'static` UTF-16 literals, so the pointers the
    // call reads through outlive it. `value` and `size` are live locals of this frame;
    // `RRF_RT_REG_DWORD` restricts the value to exactly the four bytes `size` promises, so
    // the call cannot write past `value` — and it keeps no pointer once it returns.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PERSONALIZE_KEY_PATH,
            value_name,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast()),
            Some(&raw mut size),
        )
    };

    // NFR-13: the return is examined. A refusal is not recorded in the journal: absence of
    // the value is a normal profile, not a failure of anything.
    if status.is_err() {
        return true;
    }

    value != 0
}

// =========================================================================================
// The resolution — setting × system → palette
// =========================================================================================

/// The palette a setting resolves to, given what the system says — FR-92а in one table.
///
/// Pure on purpose, and split from [`system_is_light`] on purpose: this half is verified
/// exhaustively by a six-row table in `tests\theme.rs`, which is only possible while it
/// touches nothing but its arguments. The caller reads the system once and passes the
/// answer in, so one repaint is one consistent palette even if the user flips the system
/// switch mid-message.
pub fn resolve(setting: ThemeSetting, system_light: bool) -> &'static Palette {
    match setting {
        ThemeSetting::Light => &FOG,
        ThemeSetting::Dark => &GRAPHITE,
        ThemeSetting::System => {
            if system_light {
                &FOG
            } else {
                &GRAPHITE
            }
        }
    }
}

/// Whether the non-client title bar is to be dark for this palette — FR-92а, task T-11-4.
///
/// `resolve` answers one of two `&'static` palettes, so identity with
/// [`GRAPHITE`] *is* «разрешённая палитра тёмная» — no colour arithmetic, no third
/// opinion, and the acceptance instrument of position 25 compares against the same
/// constants. Public for the same reason as `settings::static_color_role`: the test of
/// criterion 11 calls the function the dialog calls.
pub fn title_bar_is_dark(palette: &Palette) -> bool {
    std::ptr::eq(palette, &GRAPHITE)
}

// =========================================================================================
// The brushes — what `WM_CTLCOLOR*` answers with
// =========================================================================================

// =========================================================================================
// The scale of the mock-ups — from a length of the picture to a pixel of a window
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

/// `MulDiv` of Windows for positive numbers: the product over the divisor, **rounded half up** —
/// the way the dialog manager turns points into pixels and dialog units into pixels (measured, 38
/// units at 7 px per 4 → 67, `scratchpad-E83\probe-premises-e83.log`). ⛔ Not `[math]::Round` nor a
/// truncation: both answer a pixel less at every half. A divisor of zero or less answers the number
/// unchanged (NFR-13) rather than dividing by it.
pub fn mul_div_round(number: i32, numerator: i32, denominator: i32) -> i32 {
    if denominator <= 0 {
        return number;
    }

    let product = i64::from(number) * i64::from(numerator);

    i32::try_from((product + i64::from(denominator) / 2) / i64::from(denominator))
        .unwrap_or(i32::MAX)
}

/// The face, in pixels, of a dialog font of `points` on a monitor of `dpi` — `MulDiv(points, dpi,
/// 72)`, exactly what the dialog manager asks `CreateFontIndirectW` for (вопрос 143).
pub fn face_at(points: u16, dpi: i32) -> i32 {
    mul_div_round(i32::from(points), dpi, 72)
}

/// Thickness of the single outline of every rounded figure of the dialog, in the pixels of
/// the mock-ups — every `StrokeRectPx … 1` of `Draw-Control`, task T-11-16.
///
/// ⚠ **One** mock-up pixel, not the two T-11-15 wrote down off the picture: a stroke of
/// GDI+ straddles the path, so a one-pixel pen covers two rows of pixels in the picture and
/// measures as two. Through [`scaled`] the number matters only above 100 %: `.max(1)` keeps
/// the frame a pixel wide wherever the division would round it away, which is what the
/// mock-ups show at every scale.
pub const BORDER_THICKNESS: i32 = 1;

// =========================================================================================
// Кадр целиком: внеэкранный буфер одного элемента — FR-92а, task T-15-1
// =========================================================================================
//
// A drawing made of a string of separate GDI calls **straight into the DC of a control** is not
// atomic, and the defect that costs is the one the **user** found and no instrument of this
// project could: «когда я навожу мышь на кнопку вместо плавной подсветки происходит резкое
// переключение и я на мгновенье вижу впавшие прямоугольники по краям кнопок». DWM samples the
// surface of a window sixty times a second and is free to show it **between any two** of those
// calls, so every intermediate state a drawing body leaves behind is a state that can reach the
// eye.
//
// ⚠ Measured and not reasoned. A burst at 60,0 frames a second over the стенд caught an
// intermediate frame in **100 % of hover transitions**, in both palettes: the face already laid
// down, the four corners still bare ground and the caption not yet drawn. The corner smoothing
// makes the gap wider than a race usually is — it reads its ground back **out of** the DC it
// draws into and calls `GdiFlush` to be allowed to, four forced flushes of the batch queue per
// repaint, each one more room for a half-finished picture to be sampled in.
//
// The cure is the one the settings dialog already uses for its own background: build the picture
// off-screen and hand it over in a single `BitBlt`. What the program had no piece for is the
// same thing around **one element** — and that is this type. The element is drawn into a surface
// of its own, where an intermediate state has no screen to appear on, and the finished element
// crosses to the window in one blit; every state DWM can sample is either the element as it was
// or the element as it now is, and there is no third.
//
// # Почему тела рисования не переписываются под новые координаты
//
// ⚠ A surface starts at its own `(0, 0)` while the element it stands for starts wherever the
// message put it — the client rectangle of a control begins at the origin, a row of a list does
// not, and this type is meant for both. Subtracting that corner in every line of a drawing body
// would be a change to every line of it, and it would not even be enough: the smoothing above
// **reads** through the very same DC, so it would have to be taught the same shift the other way
// round.
//
// So nothing is subtracted anywhere. `SetWindowOrgEx` moves the **logical** origin of the
// surface onto the top-left corner of the element, and from that call on a logical coordinate of
// the surface means exactly what it means on the window — for the calls that write and for the
// calls that read alike, because GDI puts both ends of a blit through the mapping. A body drawn
// into this buffer names the same rectangles and the same points it named before, character for
// character; the only thing that changes is which DC it hands them to.

thread_local! {
    /// The DPI of our drawing for every live [`PaintBuffer`] of this thread, by its memory DC —
    /// задача T-83-6. A thread's own because a DC is, and so is a buffer: it lives for the one
    /// message it paints.
    static BUFFER_DRAWING_DPI: std::cell::RefCell<Vec<(HDC, i32)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The doors of the tests into this module — compiled only under the `testing` feature.
#[cfg(feature = "testing")]
pub mod testing {
    /// How many [`super::PaintBuffer`]s of this thread have their DPI recorded — задача T-83-6. The
    /// record of a buffer has to live exactly as long as the buffer: otherwise the list grows by one
    /// with every element painted, and a handle `DeleteDC` frees and GDI hands to the next memory DC
    /// answers the DPI of a buffer that is gone.
    pub fn buffers_recorded() -> usize {
        super::BUFFER_DRAWING_DPI
            .try_with(|all| all.try_borrow().map_or(0, |all| all.len()))
            .unwrap_or(0)
    }
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
// afterwards (`settings::BackgroundCache`), so even the corner arithmetic above is paid at the first
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
pub(crate) struct Canvas {
    /// The memory DC of the enlarged surface, with its bitmap already selected.
    pub(crate) dc: HDC,
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
    pub(crate) fn point(&self, x: i32, y: i32) -> (i32, i32) {
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
    pub(crate) fn outline(&self, area: &RECT) -> RECT {
        let (left, top) = self.point(area.left, area.top);
        let (right, bottom) = self.point(area.right - 1, area.bottom - 1);

        RECT {
            left,
            top,
            right: right + 1,
            bottom: bottom + 1,
        }
    }

    /// One length of the window on the enlarged surface — a radius, a pen thickness.
    pub(crate) fn length(&self, pixels: i32) -> i32 {
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
pub(crate) struct Supersample {
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
    pub(crate) fn for_tile(width: i32, height: i32) -> Option<Self> {
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
    pub(crate) fn render(
        &self,
        dc: HDC,
        tile: &RECT,
        thickness: i32,
        figure: impl FnOnce(Canvas),
    ) -> bool {
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
/// Public for the reason `settings::check_cell` and [`combo_chevron_points`] are: a test
/// draws it into a memory bitmap and reads the pixels back, which is how «шов между сглаженным
/// углом и прямой стороной» is held closed without a window and without starting the product.
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

    paint_corner_tiles(dc, area, &tiles, radius, thickness, outline, fill);
}

/// The four smoothed corners of the figure `area` names, and nothing between them — the tail
/// [`paint_rounded`] has always ended in, lifted out by task T-12-5 so that
/// [`paint_rounded_corners`] can name **the same body** instead of a copy of it (§6.2).
///
/// `tiles` are the corner squares, [`corner_tiles`] of a `radius + thickness` side; every one of
/// them is expected to have been held out of the clip (or freshly grounded) by the caller, since
/// what lands here is blended into whatever the DC already carries there.
///
/// Each corner is drawn at [`SUPERSAMPLE`] times its size and averaged back down. One surface for
/// the four: the tiles are the same size, and a figure that made four would pay four
/// `CreateDIBSection`s for nothing.
///
/// A refused surface is `None` here and is *not* a reason to leave: it refuses each of the four
/// tiles below, and the loop closes all four the same way it closes one — task T-11-24, whose
/// whole subject is that leaving early left the figure with four holes in it.
fn paint_corner_tiles(
    dc: HDC,
    area: &RECT,
    tiles: &[RECT; 4],
    radius: i32,
    thickness: i32,
    outline: COLORREF,
    fill: HBRUSH,
) {
    let surface = Supersample::for_tile(radius + thickness, radius + thickness);

    for tile in tiles {
        // An empty tile is a corner [`paint_rounded_corners`] clamped away entirely — it lies
        // outside what the caller may write — and there is nothing to paint and no hole to leave.
        // Skipped rather than sent down the aliased road, where an inverted rectangle would be
        // normalised by `IntersectClipRect` into a clip that is *not* the corner (T-12-5).
        // [`paint_rounded`]'s own tiles are never empty and never reach this line.
        if tile.right <= tile.left || tile.bottom <= tile.top {
            continue;
        }

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
        // the clip by the caller, so an unpainted tile is a hole in the figure and not a rougher
        // corner; the staircase is what it falls back to.
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
// Check marks, polylines and ellipses — the small figures every window of the program draws
// =========================================================================================

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

/// The three points of one check mark in the pixels of a window at `dpi`, measured from the
/// top-left corner of the square it is drawn in — the pure half of [`draw_check_mark`], task
/// T-11-16.
///
/// Every offset goes through [`scaled_tenths_offset`], which is [`scaled`] for a length given
/// with one decimal and without the floor of one a pen needs: a point may legitimately land on
/// the corner itself.
///
/// # The mirror — вопрос 97 п. 1, task Т-30-3
///
/// `mirrored` says the figure is going into a **mirrored** device context, and it is the whole
/// of what this program does about the tick in a right-to-left window.
///
/// GDI mirrors everything drawn into such a context, strokes included: measured
/// (`scratchpad-Э30\посылки-п3.log`) the tick came back as the exact reflection of itself, in
/// all three of its sizes, and `LAYOUT_BITMAPORIENTATIONPRESERVED` did **not** undo it — that
/// flag is about blits, and these are `Polyline` strokes. Seen by eye on the stand as well
/// (`scratchpad-Э30\ДО-Т-30-3-галочка-he.png`): the long stroke ran down to the right.
///
/// The user's decision (97.1) is that the tick stays **straight**, as it is in Windows itself:
/// the mark carries a meaning, not a direction. So the points are reflected here, about the
/// vertical axis of the glyph square, and GDI reflects them back — the figure arrives the way
/// it is drawn in the other twelve locales. One place, and it is the pure one: all three ticks
/// of this program (the dialog glyph, the row of the cycle list, the tray menu) are drawn by
/// [`draw_check_mark`], which is the only caller of this.
///
/// # The reflection is `width - dx`, and the one that reads more natural is wrong
///
/// `width - 1 - dx` is what a *pixel* reflection would be — column `x` of a mirrored context is
/// column `width - 1 - x` — and it is the formula this task was written with. It came out one
/// pixel to the right, measured (`scratchpad-Э30\посылки-п3-w-1.log`: the tick of the dialog
/// occupied columns 3..10 where the straight one occupies 2..9).
///
/// The reason is that the figure does not reach the window as pixels. [`draw_check_mark`] draws
/// it enlarged into a tile of memory and `BitBlt`s the tile over, and a **blit** is placed by
/// the *edges* of its rectangle, not by the centres of its pixels: a tile of width `w` at
/// logical `left` lands physically at `width - left - w`. Reflecting the points by `width - dx`
/// is what carries the tile's edges onto the edges the straight tile has. Measured exact for the
/// dialog glyph (columns 2..9 both ways) and for the tray menu (1..10 both ways).
///
/// ⚠ **And it is not exact for the third of the three, by one pixel** — the tick of the cycle
/// list, whose stroke bounds run into the edge of its 13-pixel square and are clamped there;
/// a clamp is not a reflection, and no single constant can undo it. Measured: 2..7 straight,
/// 1..6 mirrored. One pixel inside a thirteen-pixel glyph, with the shape itself straight,
/// which is what решение 97.1 is about. Named here rather than left to be rediscovered.
///
/// The antialiasing does not survive the reflection exactly either — supersampled coverage of a
/// stroke is not mirror-symmetric — so a handful of edge pixels differ in every size. That is
/// why the guard on the *drawn* tick asks whether it is straight and not whether it is
/// identical, and why the exact guard is on the geometry, where exactness means something.
pub fn check_mark_points(
    glyph: &RECT,
    mark: CheckMark,
    dpi: i32,
    mirrored: bool,
) -> [(i32, i32); 3] {
    let width = glyph.right - glyph.left;

    mark.points_tenths.map(|(x, y)| {
        let across = scaled_tenths_offset(x, dpi);
        let across = if mirrored { width - across } else { across };

        (
            glyph.left + across,
            glyph.top + scaled_tenths_offset(y, dpi),
        )
    })
}

/// Two strokes of the check mark, with a transient pen of `ink` — FR-92а, task T-11-5b; the
/// figure and the pen come from `mark` and the window's `dpi` since task T-11-16.
///
/// The pen lives for exactly this call: pens are not part of [`Brushes`] — that owner
/// exists because `WM_CTLCOLOR*` answers must outlive the paint, which nothing here needs.
/// Created, selected, drawn with, deselected, deleted; the refusal of `CreatePen` skips
/// the mark and nothing else (NFR-13: examined; the glyph stays a filled square, the state
/// remains readable by the fill alone until the next repaint).
///
/// `dpi` is passed in rather than read off `dc` for the reason [`paint_rounded`] states: the
/// background of the dialog is built in a memory DC, whose own answer to `GetDeviceCaps` is not
/// the DPI the figure is being drawn for.
pub fn draw_check_mark(dc: HDC, glyph: &RECT, ink: COLORREF, mark: CheckMark, dpi: i32) {
    // The two strokes of the generator: down into the corner, long up and out. The three
    // points are `mark`'s own, measured from the corner of the square through the scale — and
    // reflected within that square when the context is mirrored, so that GDI's own reflection
    // brings the tick back the right way round (вопрос 97 п. 1, task Т-30-3).
    //
    // SAFETY: `dc` is painted into for the length of the send this call is inside of.
    let mirrored = unsafe { dc_is_rtl(dc) };
    let points = check_mark_points(glyph, mark, dpi, mirrored);
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
/// [`draw_check_mark`] and of [`draw_combo_chevron`], which are the same two strokes
/// with different numbers (§6.2: one body, not two copies).
///
/// The pen lives for exactly this call: pens are not part of [`Brushes`] — that owner
/// exists because `WM_CTLCOLOR*` answers must outlive the paint, which nothing here needs.
/// Created, selected, drawn with, deselected, deleted; the refusal of `CreatePen` skips the
/// figure and nothing else (NFR-13: examined; a checked glyph stays a filled square and a combo
/// box stays a field, both readable, until the next repaint).
pub(crate) fn stroke_polyline(dc: HDC, points: &[(i32, i32); 3], ink: COLORREF, thickness: i32) {
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

/// Inset of the selection rectangle of a list row from the **left and right** edges of the
/// row, in mock-up pixels — `FillRectPx $g ($px+2) $ry ($pw-4) $rowH $S.SelBg 3`.
///
/// Left and right only: the generator gives the stripe the whole height of the row (`$ry`,
/// `$rowH`) and takes 2 px off each side of its width (`$px+2`, `$pw-4`).
pub const LIST_SELECTION_INSET: i32 = 2;

/// Corner radius of that selection rectangle, in mock-up pixels — the trailing `3` of the
/// same call.
pub const LIST_SELECTION_RADIUS: i32 = 3;

/// The selection figure of the mock-ups on one row of a list — not the whole row, but a rounded
/// rectangle [`LIST_SELECTION_INSET`] mock-up pixels in from each side of it, corner radius
/// [`LIST_SELECTION_RADIUS`]: `FillRectPx $g ($px+2) $ry ($pw-4) $rowH $S.SelBg 3` of the
/// `'lbox'` arm and «втяжка выделения 2, радиус 3» of the `'lview'` one — task T-11-16 for the
/// figure, task T-11-25 for the second list that wears it.
///
/// Left and right only, exactly as the generator does it: the stripe keeps the full height of
/// the row. `ink` is the outline, and both callers hand it the fill's own colour, because
/// [`paint_rounded`] draws frame and interior in one figure and this one has no frame.
///
/// One body and not two copies (§6.2): the exclusion list (`settings::draw_list_item`,
/// owner-drawn) and the layout list (`settings::draw_cycle_row`, custom-drawn) draw the same
/// figure from the same two constants, and a stripe that drifted between the two lists of one
/// dialog is exactly what a second copy would eventually produce.
pub(crate) fn paint_selection_stripe(dc: HDC, row: &RECT, ink: COLORREF, fill: HBRUSH, dpi: i32) {
    let inset = scaled(LIST_SELECTION_INSET, dpi);

    let stripe = RECT {
        left: row.left + inset,
        top: row.top,
        right: row.right - inset,
        bottom: row.bottom,
    };

    paint_rounded(
        dc,
        &stripe,
        scaled(LIST_SELECTION_RADIUS, dpi),
        ink,
        fill,
        dpi,
    );
}

// =========================================================================================
// Начертания: сглаживание нашего текста и то, как лицо попадает в DC
// =========================================================================================

/// The `LOGFONTW` of the dialog's own face, asked to render with the smoothing this program
/// draws its text in — the pure half of `settings::DialogFonts`.
///
/// One field changed and not a byte else: the face, the size, the weight and the character set
/// are the window's own, because the dialog font is what the template asks for and what the
/// manager already created at the window's DPI. Pure, so criterion 13 of task T-11-17 is a
/// test on a `LOGFONTW` and needs no window.
///
/// # ⚠ Серое → ClearType: решение 88 поправляет T-11-17
///
/// Task T-11-17 (the user's decision of 2026-08-22) asked for **grey** coverage, because
/// ClearType tints the edge of every stroke red and blue and the Э11 mock-ups, drawn with GDI+,
/// had no such fringe. Решение 88 (2026-09-02) reverses that half, and for a reason the earlier
/// decision could not have weighed: **the fringe was not the only cost being paid.**
///
/// GDI places every glyph on a **whole pixel**, and the advance widths of a face are fractional.
/// With grey coverage a stem therefore lands wholly inside a pixel in one place of a word and
/// straddles two in another, coming out as two columns of half ink — measured on the delivered
/// window, one row of «Исправляет текст»: `152 121 | 149 134 | 81 152 116`, where 152 is full
/// ink. The eye reads the half-lit neighbour as an echo, and the user reported it as «шрифт
/// двоится» — the same effect that made the body of the about window look bold until task
/// Т-26-2 measured its weight at 400.
///
/// ClearType has no cure for the *placement* either, but it triples the horizontal resolution
/// it can spend on the edge, so the stem stays one stem. The fringe comes back and is the price;
/// it was shown at 6× on `scratchpad-Э26\ВРЕЗ-*.png` and accepted with open eyes.
///
/// ⚠ This is the **only** place the program names a smoothing mode, and that is what makes the
/// decision one line: `settings::DialogFonts`, the two derived faces of the about window, the
/// hand-over to the fields and the tray menu all come through here.
pub fn smoothed_logfont(base: LOGFONTW) -> LOGFONTW {
    LOGFONTW {
        // Решение 88. The manager's own font asks for `DEFAULT_QUALITY`, which the machine
        // resolves to this same mode — but «what the machine resolves it to» is not a promise,
        // and this file does not draw in a mode it did not name.
        lfQuality: CLEARTYPE_QUALITY,
        ..base
    }
}

// =========================================================================================
// Подпись: земля, лицо и перенос строк — task T-14-7
// =========================================================================================
//
// The third slice of the move of finding 24. Below is the painting of **one caption into one
// rectangle**: the ground filled, the ink set, the face selected, `DrawTextW` asked how it
// wrapped the words, and the lines laid down a band at a time. None of it can be told which
// window the rectangle belongs to — both windows of the program hand it a DC, a rectangle, a
// caption, a brush, an ink and a face, and the *choosing* of those six is what stays on the
// other side of the line: `WM_CTLCOLOR*` names the brush, the identifier names the colour
// role, the control itself holds the text. The two numbers travel with the figure by the rule
// task T-14-5 wrote down — in the whole of `src\` nothing else reads either of them.

// =========================================================================================
// Reading order — вопрос 97 п. 2, task Т-30-2. One place, and it comes out of a measurement.
// =========================================================================================

/// Whether `dc` is a **mirrored** device context.
///
/// Asked of the DC and not of [`crate::settings::ui_language`] on purpose: what decides how a
/// run of text is laid out is the context it is drawn into, and a stand — or a window this
/// program has not written yet — that mirrors itself for its own reasons gets the right answer
/// without having to remember to tell anybody.
///
/// # Safety
///
/// `dc` is a live device context.
pub unsafe fn dc_is_rtl(dc: HDC) -> bool {
    // SAFETY: `dc` is live by the contract above; `GetLayout` reads no memory of ours.
    let layout = unsafe { GetLayout(dc) };

    layout & LAYOUT_RTL.0 != 0
}

/// Whether a run of text is an **island** — read left to right whatever the window around it.
///
/// Named rather than passed as a bare `bool` because the two readings of `true` are opposite at
/// the two ends of [`reading_order`], and a call site reading `reading_order(rtl, true)` says
/// nothing about which of them it meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// Text of the interface language: it reads the way the window does.
    Native,
    /// An island of вопрос 97 п. 2 — a key name, a path, a process name, a layout name whose
    /// first strong character is Latin or Cyrillic. It reads left to right even in a mirrored
    /// window, and it still stands against the window's own edge.
    LatinIsland,
}

/// A name set into a sentence of a **right-to-left** window as an embedding of its own — stage
/// E108, the stand (FAIL 2), решение 202.19о.
///
/// A line that strings names together — «Действующий цикл: A ← B ← C» — has its order decided by
/// the bidi algorithm and not by the string: Latin names and the arrows between them merge into
/// one left-to-right island, and a name in Arabic script turns its neighbours round, so one arrow
/// could not read right for both (`reports\shots-E108\README.md`). Each name is wrapped in an
/// embedding of its own direction (LRE or RLE … PDF), and a right-to-left mark stands on either
/// side of it, so that the arrows and spaces between the names are flanked by strong
/// right-to-left characters and take the direction of the window: the first name stands on the
/// right and every arrow is «←».
///
/// ⛔ Not the isolates of Unicode 6.3 (FSI … PDI), which решение 202.19о asked for first:
/// measured by `ScriptItemize` + `ScriptLayout`, the engine `DrawText` uses, they change nothing —
/// «A ← B ← C» with isolated names came out in the same order as without them, the reverse one
/// (`scratchpad-E108\logs\bidi-order.log`). Embeddings came out right with Latin names, with
/// names in Arabic script and with both.
///
/// `reads_right_to_left` is the name's own direction — [`crate::settings::reading_of_name`].
/// Never applied to a name that goes into the configuration file: only to the line of
/// «Состояние» that is put on the screen.
pub fn embedded_name(name: &str, reads_right_to_left: bool) -> String {
    const RIGHT_TO_LEFT_MARK: char = '\u{200F}';
    const LEFT_TO_RIGHT_EMBEDDING: char = '\u{202A}';
    const RIGHT_TO_LEFT_EMBEDDING: char = '\u{202B}';
    const POP_DIRECTIONAL_FORMATTING: char = '\u{202C}';

    let opening = if reads_right_to_left {
        RIGHT_TO_LEFT_EMBEDDING
    } else {
        LEFT_TO_RIGHT_EMBEDDING
    };

    format!("{RIGHT_TO_LEFT_MARK}{opening}{name}{POP_DIRECTIONAL_FORMATTING}{RIGHT_TO_LEFT_MARK}")
}

// =========================================================================================
// Чип-«клавиша»: имя горячей клавиши внутри строки справки — task Т-26-2, решение 85
// =========================================================================================
//
// The mock-up the user accepted with his own eye draws the key name of every help row inside
// a little box — `kbd { font: 600 11px; background: var(--window-bg); border: 1px solid
// var(--panel-border); border-radius: 4px; padding: 2px 5px 3px }` of
// `scratchpad-Э23\макет-справка-о-программе.html`. Решение 85 п. 1 asks for that box and for
// nothing new in the palette: the frame is `field_border` and the fill is `field_bg`, the two
// fields the input boxes of the settings dialog are already drawn from (правило Э12).
//
// ⚠ **A chip cannot be drawn with `DrawTextW`.** The row it stands in wraps, and the system's
// wrap is one call over one string — there is no place in it to put a figure. So a help row is
// laid out here, word by word: every word is measured in the face it will be drawn in, the
// chip is measured as one unbreakable word of its own, and the line is broken where the width
// runs out. The measuring and the drawing are the **same instrument** (`GetTextExtentPoint32W`
// and `TextOutW` on the same DC with the same face), which is what keeps the two from
// disagreeing — the `&`-prefix trap `paint_label_lines` warns about belongs to `DrawTextW`,
// and neither call below is one.

/// The placeholder the string tables put where the key name goes — FR-94, task Т-23-4.
///
/// The five help rows already carry it (`{0}` of `IDS_ABOUT_HELP_1..3`), which is why решение
/// 85 costs **no new localisation string at all**: the chip is drawn *at the placeholder*, and
/// the substitution that used to happen in `settings::fill_about` happens at the pen instead.
pub const KEY_PLACEHOLDER: &str = "{0}";

// =========================================================================================
// Чистая арифметика рисования: шаг строки, ширина знака, воздух рамки — task T-14-5
// =========================================================================================
//
// The second slice of the move of finding 24. Every item below answers a question of the
// drawing with numbers alone — no DC, no handle, no window — and none of them can be told
// which element the numbers belong to: what comes in is a measurement and what goes out is a
// length. A number that only one of these functions works with travels with it, by the rule
// task T-14-3 already wrote down for the radii and the pens: a constant lives **at its own
// figure**. A number a drawing of the owning module also reads stays there — which is why
// `settings::check_cell` did not travel: `LIST_TEXT_INSET` is read by
// `settings::draw_list_item` as well, and that drawing knows perfectly well whose window it
// is in.

// ⚠ **`field_frame_air` уехала в `widgets::field::air` — задача Т-45-2, решение 107.4.**
//
// Здесь она стояла с задачи T-12-3 и считала воздух вокруг однострочного поля. Уехала не
// потому, что плоха, а потому, что таких арифметик в программе было ДВЕ: эта у окна настроек
// и `letters::field_box_air` у мастера, и различались они одним — куда девать лишний пиксель
// нечётного остатка. Теперь тело одно, а различие названо параметром `OddPixel`.
//
// Место у неё было здесь по праву: `theme` — краски и примитивы. Но у поля есть **устройство**,
// а не только цвет, и устройство элемента — это `widgets`.

// =========================================================================================
// Роли цветов: словарь, которым рисование называет поля палитры, и его разрешение
// =========================================================================================
//
// The half of the drawing that has no DC in it at all: an element in a state answers a set of
// **roles**, and a role is resolved against [`Palette`] and [`Brushes`] in exactly one place.
// That split is what keeps a colour number out of every other module — §6.2 gives every
// palette value to this one — and it is what lets each table be closed by a test with no
// window in sight.
//
// ⚠ **Which element answers which role is not decided here.** The mapping «identifier →
// role» belongs to the module that owns the window: `settings::static_color_role`,
// `settings::glyph_kind`, `settings::button_color_roles` and their neighbours. This section
// holds the vocabulary and the resolution, and nothing that has to know what a control is.

/// The face of one owner-drawn button, named as the palette field it is filled with —
/// FR-92а, task T-11-5a.
///
/// Roles and not colours, exactly as [`StaticColorRole`] before it: the mapping stays a
/// pure function a table test can close, and the `WM_DRAWITEM` handler turns a role into
/// a brush of the resolved palette in one place. Not a single colour number enters the
/// module that owns the window — §6.2 gives every palette value to `theme` alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonFaceRole {
    /// [`Palette::button_bg`] — the ordinary face.
    ButtonBg,
    /// [`Palette::accent_bg`] — the accented face of the default button «ОК».
    AccentBg,
    /// [`Palette::sel_bg`] — the face while the button is held pressed: a step
    /// darker in the light palette and lighter in the dark one, with no new colour in the
    /// palette.
    SelBg,
    /// [`Palette::hover_bg`] — the face while the cursor stands on the button and
    /// nothing is held down (task T-12-8).
    ///
    /// The one member of the palette this dialog had never used: until this task `hover_bg`
    /// was drawn by the tray menu alone (`src\tray.rs`, task T-11-21), and the response of
    /// the menu is the reference this face is copied from — the same field of the same two
    /// palettes, «Графит» 52,58,67 and «Туман» 234,238,242, and no new colour invented for
    /// it.
    HoverBg,
    /// [`Palette::sel_fg`] — the face of the **accented** button while the cursor stands on
    /// it and nothing is held down (task T-15-2).
    ///
    /// # Why the ink of a selected row is a face here, and why it is not a new colour
    ///
    /// The palette has no field called «the accent under the cursor», and inventing one is
    /// forbidden — so the response had to be built out of what is already there. Two demands
    /// decide which field: it must exist in **both** palettes, and it must move the accent in
    /// **the same direction** an ordinary button moves on hover, or the two halves of one
    /// dialog would answer the pointer in opposite ways. Exactly one field does both:
    ///
    /// | | «ОК» at rest (`accent_bg`) | under the cursor (`sel_fg`) | an ordinary button |
    /// |---|---|---|---|
    /// | «Графит» | 228,231,234 | 240,242,244 — lighter by 12/11/10 | lighter by 6/7/8 |
    /// | «Туман» | 43,47,54 | 35,38,43 — darker by 8/9/11 | darker by 21/17/13 |
    ///
    /// The alternative — giving «ОК» the same `hover_bg` as everything else — was measured
    /// and rejected on the picture it makes: in «Графите» an almost-white button turns
    /// 52,58,67, which is the ground of the window, and in «Тумане» the near-black one turns
    /// 234,238,242. That is not a highlight, it is the accent going out.
    ///
    /// ⚠ In «Тумане» this field carries the same number as [`Palette::text`] and
    /// [`Palette::title_fg`] (35,38,43). That is a property of that one palette and not of
    /// the roles — the same coincidence [`Palette::cap`] documents at length — and it is
    /// still **the existing field**, not a new colour.
    ///
    /// The caption over this face stays [`ButtonTextRole::AccentFg`]: 27,30,35 on 240,242,244
    /// and 245,246,247 on 35,38,43, so the pair the accent was designed with keeps its
    /// contrast under the pointer.
    SelFg,
}

/// The ink the button's caption is drawn with, named as the palette field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonTextRole {
    /// [`Palette::text`] — the ordinary caption.
    Text,
    /// [`Palette::accent_fg`] — the caption over the accent face.
    AccentFg,
    /// [`Palette::sel_fg`] — the caption over the pressed face, the pair the
    /// palette designed for exactly this ground.
    SelFg,
    /// [`Palette::text_muted`] — the caption of a disabled button.
    TextMuted,
}

/// The frame of the button — two variants since task T-12-4, and the second one is «none».
///
/// The closed table of `settings::button_color_roles` frames every kind in every state with the
/// same single-pixel [`Palette::button_border`], and this type is that fact written down: a
/// second border **colour** still cannot appear without widening it first. What T-12-4 added is
/// not a colour but its absence — finding **A-13**: the mock-up draws the «ОК» of the about
/// window with a fill and no outline at all (`scratchpad-Э11\chrome.ps1:200-202` — `FillR`,
/// no `StrokeR`), while every button of the settings dialog keeps its frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonBorderRole {
    /// [`Palette::button_border`].
    ButtonBorder,
}

/// Face, caption ink and frame of one owner-drawn button — what
/// `settings::button_color_roles` answers and the whole of what the `WM_DRAWITEM` handler
/// needs to choose colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonColors {
    /// What the button face is filled with.
    pub face: ButtonFaceRole,
    /// What the caption is drawn with.
    pub text: ButtonTextRole,
    /// What the single-pixel frame is drawn with.
    pub border: ButtonBorderRole,
}

/// Fill, frame and mark of one tick of the layout list — what [`check_frame_colors`]
/// answers and the whole of what `settings::draw_check_glyph` needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckFrameColors {
    /// What the whole 13×13 square is filled with.
    pub fill: COLORREF,
    /// The single-pixel frame — `None` for the checked frame the accent fill covers whole.
    pub frame: Option<COLORREF>,
    /// The two-stroke check mark — `None` while unchecked.
    pub mark: Option<COLORREF>,
}

/// The colours of one tick of the layout list in one palette — FR-92а, task T-11-7: the pure
/// half of the tick, closed by a table test over both states and both palettes.
///
/// Colours of the given palette rather than roles, unlike [`glyph_color_roles`] and its kin:
/// the drawing this feeds answers no `WM_CTLCOLOR*` and asks for no brush of the dialog's own,
/// so what it needs is the palette's own values — and the task words the function as
/// «(взведена, палитра) → краски кадра». The table stays closed all the same: every answer is a
/// field of `palette` and nothing else, so not a single colour number enters this module
/// (§6.2).
///
/// The two rows quote the check-box cells of the glyph table of T-11-5b, so the ticks of the
/// list match the ticks of the dialog:
/// - **снята** — `field_bg` fill under the single-pixel `box_border` frame, no mark;
/// - **взведена** — `accent_bg` fill edge to edge, no frame, `accent_fg` check mark.
pub fn check_frame_colors(checked: bool, palette: &Palette) -> CheckFrameColors {
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
