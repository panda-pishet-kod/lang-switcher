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
//! [`system_is_light`] performs the one registry access of this module: a single
//! `RegGetValueW` of `AppsUseLightTheme`. Nothing here writes to the registry under any
//! circumstance — the whole of what this program writes there is the autostart value of
//! FR-93, and that belongs to module `settings`.
//!
//! # SEC-05
//!
//! The last sentence of SEC-05 covers the messages this theme reacts to: they carry no
//! privileged action, and the reaction is limited to re-reading the system setting and
//! repainting. This module is sized to exactly that reaction — it hands a handler a palette
//! to look up and brushes to repaint with, and nothing here takes a pointer, a string or a
//! command out of any message.

use windows::Win32::Foundation::{COLORREF, POINT, RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, COLORONCOLOR,
    CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, CreateFontIndirectW, CreatePen,
    CreateSolidBrush, DIB_RGB_COLORS, DRAW_TEXT_FORMAT, DT_CALCRECT, DT_EXPANDTABS, DT_LEFT,
    DT_SINGLELINE, DT_TOP, DT_WORDBREAK, DeleteDC, DeleteObject, DrawTextW, Ellipse,
    ExcludeClipRect, FillRect, GdiFlush, GetCurrentObject, GetDeviceCaps, GetStockObject,
    GetTextExtentPoint32W, GetTextMetricsW, HBITMAP, HBRUSH, HDC, HFONT, HGDIOBJ,
    IntersectClipRect, LOGFONTW, LOGPIXELSY, NULL_PEN, NULLREGION, OBJ_FONT, PS_SOLID, Polyline,
    RGN_ERROR, RestoreDC, RoundRect, SRCCOPY, SaveDC, SelectObject, SetBkMode, SetStretchBltMode,
    SetTextColor, SetWindowOrgEx, StretchBlt, TEXTMETRICW, TRANSPARENT, TextOutW,
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
// The system switch — FR-92а, `AppsUseLightTheme`
// =========================================================================================

/// The subkey of `HKEY_CURRENT_USER` that holds the system's app-theme switch (FR-92а).
///
/// ⚠ **Read and never written.** The one registry access of this module is the single
/// `RegGetValueW` in [`system_is_light`]; writing the user's personalization settings is
/// not this program's business under any requirement, and the session's permissions say the
/// same thing.
const PERSONALIZE_KEY_PATH: PCWSTR =
    w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");

/// Name of the value under [`PERSONALIZE_KEY_PATH`] that FR-92а names.
const APPS_USE_LIGHT_THEME_VALUE: PCWSTR = w!("AppsUseLightTheme");

/// Whether the system asks applications for a light theme — FR-92а, one `REG_DWORD` read.
///
/// Any answer but a readable zero is the light theme: a non-zero value says light in so
/// many words, and a missing value, a missing key or any other refusal to read is the
/// ordinary state of a profile whose user never touched the switch — FR-92а reads it as
/// «отсутствие значения читается как светлая тема». Deliberately *not pure*: this is the
/// one function of the module that looks at the machine, kept apart from [`resolve`] so
/// that the latter can be verified by a table and this one only has to answer without
/// falling over.
pub fn system_is_light() -> bool {
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
            APPS_USE_LIGHT_THEME_VALUE,
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

/// The GDI brushes of one palette, owned: created together, freed together in `Drop`.
///
/// A `WM_CTLCOLOR*` handler answers with an `HBRUSH`, and that brush must outlive every
/// paint that uses it — a handler cannot create one per message and delete it on the way
/// out. Somebody has to own the brushes for as long as a palette is on screen, and this
/// type is that somebody: eight brushes for the palette fields the handlers of tasks
/// T-11-4…T-11-10 fill areas with.
///
/// # Where it lives — section 6.1
///
/// **On the UI thread and nowhere else.** The brushes are handed out by window procedures,
/// the windows of this program live on the UI thread (section 6.1), and GDI objects are not
/// shared across threads. The type is neither `Sync` nor sent anywhere; nothing enforces
/// the thread beyond the fact that no other thread ever sees the value — which is exactly
/// how section 6.1 words it.
///
/// # A palette change recreates the set
///
/// When the resolved palette changes — the user picked another setting, or the system
/// switch flipped under `theme = "system"` (task T-11-9) — the set is **recreated from the
/// new palette, not repainted in place**: the old value is dropped, a new one is built with
/// [`Brushes::new`]. That is why there is no setter here and the fields are private — a
/// brush handed to the window manager yesterday must stay valid until the drop, and a field
/// overwritten in place would either leak its old brush or free one still in use.
pub struct Brushes {
    /// [`Palette::window_bg`] — the answer of `WM_CTLCOLORDLG` and `WM_CTLCOLORSTATIC`.
    window_bg: HBRUSH,
    /// [`Palette::panel_bg`] — fill of group boxes.
    panel_bg: HBRUSH,
    /// [`Palette::field_bg`] — the answer of `WM_CTLCOLOREDIT` and `WM_CTLCOLORLISTBOX`.
    field_bg: HBRUSH,
    /// [`Palette::button_bg`] — face of an owner-drawn button.
    button_bg: HBRUSH,
    /// [`Palette::sel_bg`] — the selected row of an owner-drawn list.
    sel_bg: HBRUSH,
    /// [`Palette::button_border`] — the one-pixel `FrameRect` frame of an owner-drawn
    /// button (task T-11-5a).
    button_border: HBRUSH,
    /// [`Palette::accent_bg`] — face of the accented default button (task T-11-5a).
    accent_bg: HBRUSH,
    /// [`Palette::box_border`] — frame of an unchecked owner-drawn check box or radio
    /// button (task T-11-5b): `FrameRect` takes a brush, not a colour.
    box_border: HBRUSH,
}

impl Brushes {
    /// Creates the brush set of one palette, every handle checked.
    ///
    /// `None` when any `CreateSolidBrush` refuses — GDI objects are a finite per-session
    /// resource, and NFR-13 does not allow painting with a handle nobody looked at. The
    /// brushes already created by a partially failed attempt are freed right here: the
    /// constructor owns what it made until it hands it out. Nothing is recorded in the
    /// journal on that path — `CreateSolidBrush` does not promise a last-error code, and
    /// inventing one would put a made-up number into the diagnostics; the caller sees
    /// `None` and keeps its previous set.
    pub fn new(palette: &Palette) -> Option<Self> {
        // SAFETY: `CreateSolidBrush` takes one colour by value, reads no memory of ours
        // and returns a handle. Every handle is examined below; the eight become the
        // property of the returned value and are freed exactly once, in `Drop`.
        let handles = unsafe {
            [
                CreateSolidBrush(palette.window_bg),
                CreateSolidBrush(palette.panel_bg),
                CreateSolidBrush(palette.field_bg),
                CreateSolidBrush(palette.button_bg),
                CreateSolidBrush(palette.sel_bg),
                CreateSolidBrush(palette.button_border),
                CreateSolidBrush(palette.accent_bg),
                CreateSolidBrush(palette.box_border),
            ]
        };

        // NFR-13: every handle is examined before anybody paints with it.
        if handles.iter().any(HBRUSH::is_invalid) {
            for handle in handles.into_iter().filter(|handle| !handle.is_invalid()) {
                // SAFETY: `handle` came from the successful `CreateSolidBrush` above, was
                // handed out to nobody, and this loop is the only place that frees it —
                // the failed constructor returns `None` and no `Drop` will run.
                let deleted = unsafe { DeleteObject(handle.into()) };

                // NFR-13: examined — loudly where the tests run, in words here. A refusal
                // would mean the handle was not a live GDI object of this process, a state
                // the ownership above makes unreachable; nothing can be done about it, the
                // process is not brought down over cleanup, and it is deliberately not
                // recorded: the journal's vocabulary — the closed `OPERATIONS` table of
                // module `diag` — has no row for GDI cleanup, and adding one belongs to
                // the module that owns the table.
                debug_assert!(deleted.as_bool(), "DeleteObject refused a brush just made");
            }

            return None;
        }

        let [
            window_bg,
            panel_bg,
            field_bg,
            button_bg,
            sel_bg,
            button_border,
            accent_bg,
            box_border,
        ] = handles;

        Some(Self {
            window_bg,
            panel_bg,
            field_bg,
            button_bg,
            sel_bg,
            button_border,
            accent_bg,
            box_border,
        })
    }

    /// The brush of [`Palette::window_bg`], borrowed — the owner frees it, nobody else.
    pub fn window_bg(&self) -> HBRUSH {
        self.window_bg
    }

    /// The brush of [`Palette::panel_bg`], borrowed — the owner frees it, nobody else.
    pub fn panel_bg(&self) -> HBRUSH {
        self.panel_bg
    }

    /// The brush of [`Palette::field_bg`], borrowed — the owner frees it, nobody else.
    pub fn field_bg(&self) -> HBRUSH {
        self.field_bg
    }

    /// The brush of [`Palette::button_bg`], borrowed — the owner frees it, nobody else.
    pub fn button_bg(&self) -> HBRUSH {
        self.button_bg
    }

    /// The brush of [`Palette::sel_bg`], borrowed — the owner frees it, nobody else.
    pub fn sel_bg(&self) -> HBRUSH {
        self.sel_bg
    }

    /// The brush of [`Palette::button_border`], borrowed — the owner frees it, nobody else.
    pub fn button_border(&self) -> HBRUSH {
        self.button_border
    }

    /// The brush of [`Palette::accent_bg`], borrowed — the owner frees it, nobody else.
    pub fn accent_bg(&self) -> HBRUSH {
        self.accent_bg
    }

    /// The brush of [`Palette::box_border`], borrowed — the owner frees it, nobody else.
    pub fn box_border(&self) -> HBRUSH {
        self.box_border
    }
}

impl Drop for Brushes {
    fn drop(&mut self) {
        for handle in [
            self.window_bg,
            self.panel_bg,
            self.field_bg,
            self.button_bg,
            self.sel_bg,
            self.button_border,
            self.accent_bg,
            self.box_border,
        ] {
            // SAFETY: every field came from a successful `CreateSolidBrush` in `new` and
            // is freed exactly once: the type is neither `Copy` nor `Clone`, the fields
            // are private and never reassigned, and `drop` runs once.
            let deleted = unsafe { DeleteObject(handle.into()) };

            // NFR-13: examined — loudly where the tests run, in words here. A refusal
            // would mean a handle that is not a live GDI object of this process, which the
            // ownership above makes unreachable; nothing can be done about it, the process
            // is not brought down over cleanup, and it is deliberately not recorded: the
            // journal's vocabulary — the closed `OPERATIONS` table of module `diag` — has
            // no row for GDI cleanup, and adding one belongs to the module that owns the
            // table, not to a `Drop` in this one.
            debug_assert!(deleted.as_bool(), "DeleteObject refused an owned brush");
        }
    }
}

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

/// An off-screen surface exactly the size of one element, owned: memory DC and bitmap made
/// together, freed together in `Drop` — NFR-13, the ownership pattern of [`Supersample`].
///
/// # Как этим пользуются
///
/// Three lines around a drawing body that is otherwise left exactly as it was:
///
/// ```text
/// let dpi = dc_dpi(dc);                                     // ⚠ off the DC of the message
/// let buffer = PaintBuffer::for_rect(dc, &rect);            // None on a refusal of GDI
/// let target = buffer.as_ref().map_or(dc, PaintBuffer::dc);
/// … the body exactly as it was, writing into `target` …
/// if let Some(buffer) = buffer { buffer.blit(dc); }         // one BitBlt, and the only one
/// ```
///
/// `None` on every refusal of GDI (NFR-13): `map_or` then hands the body the DC of the message,
/// and the element is painted the way this program painted it before this task — flicker and
/// all, which is degraded and alive rather than unpainted.
///
/// # ⚠ Два места, где буфер отвечает не про окно
///
/// **The DPI.** `GetDeviceCaps` of a memory surface answers for the memory and not for the
/// window, which is the very reason [`paint_rounded`] takes its `dpi` as a parameter instead of
/// reading it off the DC it paints on. A radius scaled off this buffer would be wrong at every
/// scale that is not 100 %, and nothing would say so — no red test and no warning, just another
/// figure. The DPI is read off the DC of the message, before the substitution.
///
/// **The ground.** [`Supersample::render`] reads the ground of a corner back **out of** the DC
/// it is drawing into, so whatever lies under the four corners is what the smoothing blends the
/// arc into. The contents of a fresh `CreateCompatibleBitmap` are undefined, so a body drawn
/// into a buffer has to lay the ground of its whole rectangle down **first** — which is what an
/// owner-drawn body does anyway, and for its own reason (task T-12-6). Get that wrong and the
/// halftones of all four corners move: the figure is still drawn, and it is drawn differently.
pub struct PaintBuffer {
    /// The memory DC the element is drawn through, with `bitmap` selected and the logical
    /// origin already moved onto `area` — see the section above.
    dc: HDC,
    /// The picture itself — compatible with the DC handed to [`PaintBuffer::for_rect`] and
    /// never with `dc`: a bitmap compatible with a *memory* DC would be monochrome, the same
    /// trap [`Supersample`] words for its own section.
    bitmap: HBITMAP,
    /// The bitmap the fresh memory DC was born with — put back in `Drop` before `bitmap` is
    /// deleted, because a bitmap still selected into a DC cannot be freed.
    previous: HGDIOBJ,
    /// The rectangle of the window this surface stands for, in the coordinates the caller was
    /// handed it in. Kept rather than asked for a second time, so that [`PaintBuffer::blit`]
    /// cannot be given a rectangle other than the one that was drawn.
    area: RECT,
}

impl PaintBuffer {
    /// The surface for the element `area` covers, for the device `target` paints on.
    ///
    /// `target` is read — for its colour depth and for the face it carries — and never written
    /// to: the picture crosses to it in [`PaintBuffer::blit`] and nowhere else.
    ///
    /// `None` for an empty rectangle and for every refusal of GDI (NFR-13), which the type's own
    /// documentation says what the caller does about.
    ///
    /// # Safety
    ///
    /// `target` is a live DC — the one of the message the caller is inside of — read for the
    /// length of this call and not kept.
    pub unsafe fn for_rect(target: HDC, area: &RECT) -> Option<Self> {
        let width = area.right - area.left;
        let height = area.bottom - area.top;

        if width <= 0 || height <= 0 {
            return None;
        }

        // SAFETY: a memory DC over the caller's; deleted in `Drop` and on every failing path
        // below.
        let dc = unsafe { CreateCompatibleDC(Some(target)) };

        // NFR-13: examined — no DC, no buffer.
        if dc.is_invalid() {
            return None;
        }

        // SAFETY: compatible with the *caller's* DC — see the field's own ⚠ — and owned by this
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
            // SAFETY: the bitmap is selected into nothing, so it is free to delete; each handle
            // is freed exactly once.
            let _ = unsafe { DeleteObject(bitmap.into()) };
            let _ = unsafe { DeleteDC(dc) };
            return None;
        }

        // From here on both handles belong to a live value, and every way out of this function
        // runs its `Drop` — there is no second cleanup path to keep in step with the first.
        let buffer = Self {
            dc,
            bitmap,
            previous,
            area: *area,
        };

        // The whole of the coordinate arrangement, and the reason the drawing bodies are left
        // alone — see the section above this type.
        //
        // SAFETY: `buffer.dc` is the memory DC made a few lines above; the call writes an
        // attribute of that DC, and the `None` means it is asked to write no previous origin
        // anywhere.
        let moved = unsafe { SetWindowOrgEx(buffer.dc, area.left, area.top, None) };

        // NFR-13: examined, and this is the one refusal that must not be shrugged at. An origin
        // that did not move leaves a surface whose `(0, 0)` is the corner of the *window*: an
        // element at `(545, 714)` would be drawn past the edge of a bitmap that is 87 px wide,
        // and the blit would hand the window a rectangle of undefined pixels. Refusing the
        // buffer sends the caller back to painting straight into the DC of the message.
        if !moved.as_bool() {
            return None;
        }

        // The face the drawing would have found had it stayed in the DC of the message. A body
        // with a face of its own selects it and puts it back ([`select_face`]), but one whose
        // faces were refused at initialisation draws in whatever the DC already carried (NFR-13)
        // — and a fresh memory DC carries the stock `SYSTEM_FONT`, which is neither the same
        // face nor the same metrics. Carrying it over is what makes this a stand-in for the DC
        // and not merely a rectangle of pixels.
        //
        // SAFETY: `target` is the caller's live DC; `GetCurrentObject` answers a handle owned by
        // whoever selected it there, and selecting that handle into a second DC neither copies
        // nor frees it.
        let face = unsafe { GetCurrentObject(target, OBJ_FONT) };

        // NFR-13: examined — a DC that would not name its face leaves the memory DC with the one
        // it was born with, the same degraded-but-alive answer [`select_face`] gives a window
        // whose fonts were refused.
        if !face.is_invalid() {
            // SAFETY: as above. The answer names the stock font this one-line-old DC was born
            // with and is deliberately dropped: nothing of ours is being displaced, and
            // `DeleteDC` frees no object that is merely selected into the DC it destroys.
            unsafe { SelectObject(buffer.dc, face) };
        }

        Some(buffer)
    }

    /// The DC to draw into — the one thing a drawing body has to be told.
    pub fn dc(&self) -> HDC {
        self.dc
    }

    /// Hands the finished element over to `target` in one `BitBlt` — the whole point of the
    /// type, and the single moment at which any of it becomes visible.
    ///
    /// ⚠ The source point is the top-left corner of the element and not `(0, 0)`, which is not a
    /// slip: it is a **logical** point of a surface whose origin was moved onto that very
    /// corner, so it names device `(0, 0)` of the bitmap. A literal `0, 0` here would read from
    /// `area.left, area.top` pixels *into* the picture and hand the window a shifted element.
    ///
    /// Answers whether it went over. `false` (NFR-13) leaves the element showing what the window
    /// already had there — its previous picture and never a hole — and the next repaint of it
    /// tries again.
    ///
    /// # Safety
    ///
    /// `target` is a live DC, painted into for the length of the send the caller is inside of.
    pub unsafe fn blit(&self, target: HDC) -> bool {
        // SAFETY: both DCs are live — `target` is the caller's and `self.dc` holds this value's
        // own bitmap. Neither end of the copy touches memory of this process.
        unsafe {
            BitBlt(
                target,
                self.area.left,
                self.area.top,
                self.area.right - self.area.left,
                self.area.bottom - self.area.top,
                Some(self.dc),
                self.area.left,
                self.area.top,
                SRCCOPY,
            )
        }
        .is_ok()
    }
}

impl Drop for PaintBuffer {
    fn drop(&mut self) {
        // SAFETY: `self.previous` is the bitmap this DC was born with, kept since `for_rect`;
        // putting it back frees `self.bitmap` to be deleted. The answer is dropped — there is
        // nothing to put back if the DC is already gone, and this path carries no journal row
        // for the reason [`Brushes`] gives for its own cleanup.
        unsafe { SelectObject(self.dc, self.previous) };

        // SAFETY: both came from the successful calls in `for_rect`, were handed to nobody, and
        // are freed exactly once — the type is neither `Copy` nor `Clone`, its fields are private
        // and never reassigned, and `drop` runs once.
        let _ = unsafe { DeleteObject(self.bitmap.into()) };
        let _ = unsafe { DeleteDC(self.dc) };
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

    /// One rectangle of a figure with **no pen** — the dot of a radio button, whose whole edge
    /// is the fill. A fill has no path to centre anything on, so the plain multiplication is
    /// the right conversion here and the corrections above would shrink the figure.
    pub(crate) fn area(&self, area: &RECT) -> RECT {
        RECT {
            left: (area.left - self.origin_x) * SUPERSAMPLE,
            top: (area.top - self.origin_y) * SUPERSAMPLE,
            right: (area.right - self.origin_x) * SUPERSAMPLE,
            bottom: (area.bottom - self.origin_y) * SUPERSAMPLE,
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

/// The three colours one corner patch of [`paint_rounded_corners`] is cut out of — task T-12-5.
///
/// A struct and not three parameters because the function would otherwise carry eight of them,
/// which is one past what this crate's lint settings allow.
#[derive(Clone, Copy)]
pub struct CornerColors {
    /// The ground the corner the rounding cuts away stands on — what the *outside* of the arc
    /// is filled with before the figure goes over it, and therefore what the smoothing blends
    /// the curve into. For a list this is the panel the control lies on.
    pub ground: HBRUSH,
    /// The ink of the single-pixel frame around the figure — `field_border`.
    pub outline: COLORREF,
    /// The interior of the figure — `field_bg`.
    pub fill: HBRUSH,
}

/// The four rounded corners of `area`, laid **over** a drawing that is already there —
/// FR-92а, task T-12-5, finding R-06.
///
/// # What this is for, and why it is not [`paint_rounded`]
///
/// A list control paints its own interior, and it paints it as a rectangle: the ground below the
/// rows (`WM_CTLCOLORLISTBOX` for the exclusion list, `LVM_SETBKCOLOR` for the layout list) and
/// the ground of every row (`settings::draw_list_item`, `settings::draw_cycle_row`) are flat
/// fills edge to edge. The frame around the list is [`CORNER_RADIUS`] rounded and is drawn by
/// the dialog's own background one thickness outside the control (`settings::paint_background`),
/// so three of the four pixels each corner arc is made of fall **inside** the control, where that
/// flat fill lands on top of them. Painting the whole figure again would not help: the rows are
/// painted after the erase, so whatever an erase draws in a corner a row can still square off.
/// This is therefore the *last* thing painted, after the control has finished — the four corner
/// squares, and not one pixel more, so nothing the control drew between them is touched.
///
/// `bounds` is the rectangle the patch may write in — the client area of the control. The tiles
/// are held inside it before anything is drawn, which is not decoration: [`Supersample::render`]
/// reads its ground back out of `dc`, and a tile hanging off the edge of the surface would have
/// it blending the curve into whatever a clipped `StretchBlt` left behind.
///
/// `radius` is in pixels of the window, as [`paint_rounded`] takes it; `dpi` is the DPI of the
/// **window** and not of the DC, for the reason that function states.
///
/// # Every refusal still paints — T-11-24
///
/// A figure too small to hold four corner squares is drawn by [`stroke_corner_aliased`] in each
/// of them — the staircase, never a hole — and so is any tile the smoothing surface refuses,
/// inside [`paint_corner_tiles`]. A radius of zero is the one case that draws nothing at all,
/// and there is nothing to draw: a square figure has no arc to put back.
pub fn paint_rounded_corners(
    dc: HDC,
    area: &RECT,
    bounds: &RECT,
    radius: i32,
    colors: CornerColors,
    dpi: i32,
) {
    if radius <= 0 {
        return;
    }

    // The same pen the figure this is a piece of was stroked with — see [`paint_rounded`].
    let thickness = scaled(BORDER_THICKNESS, dpi).max(1);
    let side = radius + thickness;

    // The same test [`paint_rounded`] makes, on the same figure: two overlapping smoothing tiles
    // would blend one corner into another.
    let smoothed = side * 2 <= (area.right - area.left).min(area.bottom - area.top);

    let mut tiles = corner_tiles(area, side);

    for tile in &mut tiles {
        // Held inside what may be written. See the doc comment: this is what keeps the smoothing
        // from reading its ground off the edge of the surface.
        *tile = clamped_to(tile, bounds);

        // ⚠ **The emptiness is examined here and not left to GDI.** `clamped_to` of a corner that
        // lies wholly outside `bounds` answers an *inverted* rectangle — right below left — and
        // `FillRect` **normalises** what it is given rather than refusing it, so an inverted tile
        // is not «nothing» but a filled band somewhere else entirely. Measured on the stand: with
        // nine exclusions the list puts up a non-client scroll bar, the right-hand tiles clamp to
        // an inverted rectangle, and 13 px of the scroll bar came out `panel_bg`. Nor does the DC
        // save it — a `GetDC` DC of a list writes into that strip, which was measured too.
        if tile.right <= tile.left || tile.bottom <= tile.top {
            continue;
        }

        // The ground the arc is cut from. Laid first and over the whole tile, exactly as
        // `settings::draw_combo_closed_part` lays it before its own rounded figure: the interior
        // of the tile is painted back by the figure a moment later, and what has to be left
        // standing outside the arc is the panel and not the control's flat fill.
        //
        // SAFETY: `dc` is painted into for the length of the call this is inside of, `tile` is a
        // live local of this frame, and `colors.ground` is a live brush of the dialog's state.
        // NFR-13: the answer is dropped for the reason the paint calls of this file drop theirs.
        unsafe { FillRect(dc, tile, colors.ground) };
    }

    if smoothed {
        // The smoothed road, through the very body [`paint_rounded`] uses — not a copy of it.
        // A clamped tile is only ever *smaller* than the surface `for_tile` makes, and `render`
        // refuses a tile larger than that alone.
        paint_corner_tiles(
            dc,
            area,
            &tiles,
            radius,
            thickness,
            colors.outline,
            colors.fill,
        );
        return;
    }

    // T-11-24: the refusal is a staircase and never a hole.
    for tile in &tiles {
        // Empty — clamped away entirely; see the same guard in [`paint_corner_tiles`].
        if tile.right <= tile.left || tile.bottom <= tile.top {
            continue;
        }

        stroke_corner_aliased(
            dc,
            area,
            tile,
            radius,
            thickness,
            colors.outline,
            colors.fill,
        );
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
pub fn check_mark_points(corner: (i32, i32), mark: CheckMark, dpi: i32) -> [(i32, i32); 3] {
    mark.points_tenths.map(|(x, y)| {
        (
            corner.0 + scaled_tenths_offset(x, dpi),
            corner.1 + scaled_tenths_offset(y, dpi),
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
pub(crate) fn paint_ellipse(
    dc: HDC,
    area: &RECT,
    outline: Option<COLORREF>,
    fill: HBRUSH,
    dpi: i32,
) {
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

/// Lays the one-pixel line the mock-ups draw under the title bar along the top row of
/// `area` — FR-92а, task T-12-1, п. 4.
///
/// # Why it is drawn here at all
///
/// The line belongs to the non-client frame: `ui.ps1:431-433` draws it with a `WinBorder`
/// pen along the bottom edge of the caption, and `chrome.ps1` does the same for the about
/// window. This program does not paint its non-client area and — by the verdict of goal 4 of
/// the E12 reconnaissance — is not going to: DWM owns those thirty-one rows, and there is no
/// documented attribute that puts a line under them. So the line is imitated by the first
/// row of the *client* area, which is ours, touches the caption with no gap, and costs one
/// `FillRect`.
///
/// The colour is [`Palette::field_border`] — 58,64,72 against the mock-ups' `WinBorder`
/// 58,63,71, one level apart in two channels, a tolerance the stage's ТЗ accepted rather than
/// let an eighteenth palette field be invented for a difference nobody can see.
pub(crate) fn paint_caption_underline(dc: HDC, area: &RECT, colour: COLORREF) {
    // SAFETY: a plain colour in, a handle out, owned by this frame until the `DeleteObject`
    // below.
    let brush = unsafe { CreateSolidBrush(colour) };

    // NFR-13: examined — no brush, no line, and the window is short of one row of colour
    // rather than short of a background.
    if brush.is_invalid() {
        return;
    }

    let line = RECT {
        left: area.left,
        top: area.top,
        right: area.right,
        bottom: area.top + 1,
    };

    // SAFETY: `dc` is painted into for the length of the call this is inside of, `line` is a
    // live rectangle of this frame, and `brush` is the live brush just made. The answer is
    // dropped for the NFR-13 reason the drawing paths of this file all state: a refused fill
    // costs one row of colour and nothing else.
    unsafe { FillRect(dc, &line, brush) };

    // SAFETY: created above, handed to nobody, freed exactly once.
    let _ = unsafe { DeleteObject(brush.into()) };
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
pub(crate) fn draw_combo_chevron(dc: HDC, points: [(i32, i32); 3], ink: COLORREF, dpi: i32) {
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

// =========================================================================================
// Начертания: серое сглаживание нашего текста и то, как лицо попадает в DC
// =========================================================================================

/// The `LOGFONTW` of the dialog's own face, asked to render with grey antialiasing — the pure
/// half of `settings::DialogFonts`, and the whole of «наш текст — серое сглаживание».
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

/// One `HFONT` from a `LOGFONTW`, examined — `None` for a refused `CreateFontIndirectW`.
pub(crate) fn create_font(logical: LOGFONTW) -> Option<HFONT> {
    // SAFETY: `logical` is a live local of this frame, read by the call; the handle it answers
    // is owned by the caller.
    let created = unsafe { CreateFontIndirectW(&raw const logical) };

    // NFR-13: examined — a refused font is the manager's own face, not a face nobody looked at.
    if created.is_invalid() {
        return None;
    }

    Some(created)
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
pub(crate) unsafe fn select_face(dc: HDC, face: Option<HFONT>) -> Option<HGDIOBJ> {
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
pub(crate) unsafe fn restore_face(dc: HDC, previous: Option<HGDIOBJ>) {
    if let Some(previous) = previous {
        // SAFETY: see the contract above.
        unsafe { SelectObject(dc, previous) };
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

/// The step from one line of a **wrapped** owner-drawn label to the next, in the pixels of the
/// mock-ups — решение **В-6** (`DECISIONS.md`, вопрос 56), task T-12-12.
///
/// # Where the 23 comes from: the generator's literals, not a picture
///
/// `DrawTextW` has no lever for line spacing at all — it advances a line by the natural
/// `tmHeight` of the face, 15 px for Segoe UI 9 pt at 96 DPI. The mock-ups were not drawn with
/// it. Their generator lays text with GDI+ `DrawString` **into a layout rectangle**, and GDI+
/// advances a line by `Font.GetHeight()`, which is the face's own `GetLineSpacing` over its
/// `GetEmHeight` — a different number from `tmHeight` and a larger one. Every value below is a
/// literal of `scratchpad\ui.ps1` or a metric of the face that file names:
///
/// - line 7 — `$DPI = 1.4`: the mock-ups are drawn at 140 %, the same 1,4
///   [`MOCKUP_SCALE_TENTHS`] already carries;
/// - line 10 — `$FS = 9 * $DPI`: the dialog face at that scale is **12,6 pt**;
/// - line 448 — `New-Object System.Drawing.Font('Segoe UI', $FS, [FontStyle]::Regular)`;
/// - line 78 — `$g.DrawString($text, $font, $brush, $r, $sf)` with `$r` a **rectangle**, which
///   is the overload that wraps and advances lines by itself;
/// - Segoe UI regular: `GetEmHeight` = 2048, `GetLineSpacing` = 2724, so a line is
///   2724 / 2048 = **1,330078125 em**.
///
/// 12,6 pt × 96 / 72 = **16,8 mock-up px** to the em, and 16,8 × 1,330078125 = **22,345
/// mock-up px** to the line. A raster can put a line only on a whole row, so the whole steps
/// come out 22 and 23 in turn — and the two lines of this very label landed on **23** in
/// `ui-02-graphite.png` (x-height tops on rows 190 and 213). The picture *confirms* the
/// number; the arithmetic above *produces* it.
///
/// ⚠ The rounding of the mock-up literal is not what decides the screen: [`scaled`] divides by
/// the DPI of the mock-ups and **both** `scaled(22, 96)` and `scaled(23, 96)` are **16** — the
/// 15,96 px the fraction asks for, against the 15 px `DrawTextW` gives on its own. В-6 sealed
/// 23, and the pitch scales with the DPI of the window because it goes through [`scaled`].
pub const LABEL_LINE_PITCH: i32 = 23;

/// Paints one owner-drawn label: the ground it stands on, then its text in our own
/// grey-antialiased face — FR-92а, task T-11-18. The drawing half `settings::draw_label` and
/// `settings::draw_about_label` share, exactly as `settings::paint_push_button` is shared by
/// the two windows' button paths (§6.2: one body, not a copy).
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
    // SAFETY: the whole contract of this function, forwarded unchanged — see the doc comment.
    unsafe { paint_label_at_pitch(dc, rect, caption, ground, ink, face, None) }
}

/// [`paint_label`] with the line pitch **named by the caller** — task Т-26-2, решение 85.
///
/// The one difference: `pitch` in pixels of the window, or `None` for [`LABEL_LINE_PITCH`]
/// through [`scaled`], which is what every label of the settings dialog has always been drawn
/// at. The about window's two description lines ask for their own, because решение 85 sets
/// them in a **larger** face than the dialog's, and a pitch worked out for the 9 pt face is
/// no longer air under a 10 pt one — it is smaller than the line itself, and
/// [`label_model_pitch`] would refuse it. The number the caller hands over is a ratio of the
/// face it draws in (`settings::about_body_line_pitch`), so it grows with the face and with
/// the DPI together.
///
/// `paint_label` is this function with `None`, and there is one body rather than two (§6.2).
///
/// # Safety
///
/// As [`paint_label`]: called with values copied out of the `WM_DRAWITEM` message the caller
/// is inside of.
pub unsafe fn paint_label_at_pitch(
    dc: HDC,
    rect: RECT,
    caption: &mut [u16],
    ground: HBRUSH,
    ink: COLORREF,
    face: Option<HFONT>,
    pitch: Option<i32>,
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

    // How many lines the **system** made of this caption, and how tall one of them is — asked
    // of `DrawTextW` itself, so the wrap cannot be a second opinion (task T-12-12).
    //
    // SAFETY: the face is selected into `dc` above, so the measuring reads the very face the
    // drawing below will use; `caption` and the scratch rectangles are live locals.
    let measured = unsafe { measure_label_lines(dc, rect, caption) };

    // The model pitch of В-6 — or the caller's own, task Т-26-2 — or `None`, «one plain call»,
    // which is the drawing of T-11-18 byte for byte. [`label_model_pitch`] is the whole of the
    // decision and is pure.
    let asked = pitch.unwrap_or_else(|| scaled(LABEL_LINE_PITCH, dc_dpi(dc)));

    let model = measured.and_then(|(lines, natural)| {
        label_model_pitch(lines, natural, asked).map(|pitch| (lines, natural, pitch))
    });

    // The line by line drawing, and how many lines it managed. A caption of one line, or a
    // model pitch with nothing to give, is zero lines drawn without a call being made at all.
    let painted = match model {
        // SAFETY: see the caller — `dc` is the DC of the message and `rect` a live local of
        // the caller's frame; `caption` is a live local of `draw_label`'s.
        Some((lines, natural, pitch)) => unsafe {
            paint_label_lines(dc, rect, caption, lines, natural, pitch)
        },
        None => 0,
    };

    if painted == 0 {
        // The whole rectangle of the control, with no inset of its own — the very rectangle an
        // `SS_LEFT` static drew into, so no caption moves by a pixel (T-11-16 and T-11-17
        // settled the values and this task changes none of them).
        let mut text_rect = rect;

        // SAFETY: `caption` and `text_rect` are live locals of this frame and the caller's;
        // the format has no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call reads the
        // caption and writes only pixels of the DC.
        unsafe { DrawTextW(dc, caption, &mut text_rect, LABEL_TEXT_FORMAT) };
    }

    // SAFETY: `previous_face` is what `select_face` answered for this same DC, and nothing
    // between the two calls selected another font.
    unsafe { restore_face(dc, previous_face) };

    // TRUE — the label is drawn.
    1
}

/// How many lines `DrawTextW` makes of `caption` inside `rect`, and how tall one line is —
/// task T-12-12, and **the reason the wrap of this file cannot drift**.
///
/// Both numbers come out of `DrawTextW` with `DT_CALCRECT` and [`LABEL_TEXT_FORMAT`]: the
/// first with the format whole, which wraps and answers the height of everything; the second
/// with `DT_SINGLELINE` added, which answers the height of one line of the face in the DC.
/// Their quotient is the number of lines. Nothing here decides *where* a line breaks — the
/// system does, exactly as it did before the task, and [`paint_label_lines`] never asks either.
///
/// # NFR-13
///
/// `DrawTextW` answers zero when it refuses, and a height that is not a whole number of lines
/// is an answer this arithmetic cannot read. Both are `None`, which the caller spends on the
/// single plain call — the drawing of T-11-18 untouched.
///
/// # Safety
///
/// `dc` is a live DC with the face of the drawing already selected into it, and `caption` is a
/// live slice. `DT_CALCRECT` writes the measured extent into the scratch rectangles of this
/// frame and nowhere else; without `DT_MODIFYSTRING` the caption itself is only read.
unsafe fn measure_label_lines(dc: HDC, rect: RECT, caption: &mut [u16]) -> Option<(i32, i32)> {
    let mut wrapped = rect;

    // SAFETY: see the contract above.
    let whole = unsafe { DrawTextW(dc, caption, &mut wrapped, LABEL_TEXT_FORMAT | DT_CALCRECT) };

    let mut single = rect;

    // SAFETY: as above.
    let one = unsafe {
        DrawTextW(
            dc,
            caption,
            &mut single,
            LABEL_TEXT_FORMAT | DT_CALCRECT | DT_SINGLELINE,
        )
    };

    if whole <= 0 || one <= 0 || whole % one != 0 {
        return None;
    }

    Some((whole / one, one))
}

/// Draws a wrapped label one line at a time, each line in the band the model pitch puts it in
/// — task T-12-12. Answers how many lines it painted.
///
/// # Why every pass draws the whole caption
///
/// The wrap has to stay the system's own: the captions come from the string tables of both
/// locales (FR-94) and, in the case of the journal path, from whatever folder the person
/// happens to have, so a hand-written line breaker would have to agree with `DT_WORDBREAK` on
/// text that cannot be enumerated. Measured, it would not: `GetTextExtentPoint32W` makes
/// `D:\папка с пробелом\Lang&Switcher\журнал` **250 px** wide and `DrawTextW` makes it **240**,
/// because `DrawTextW` without `DT_NOPREFIX` eats the `&` as a prefix marker and the extent
/// call measures it as a glyph — ten pixels, and the journal path is precisely the label a
/// stray `&` can reach ([`LABEL_TEXT_FORMAT`] says so in its last paragraph).
///
/// So every pass calls the **same** `DrawTextW` with the **same** [`LABEL_TEXT_FORMAT`] on the
/// **same** rectangle width, only shifted down the page, and a clip lets exactly one line of
/// it through. Line `j` of pass `i` lands `(j − i) × natural` from the top of the band, so the
/// band of `natural` pixels holds line `i` and no other — the arithmetic, not a promise.
///
/// # No GDI object is created, so none can leak
///
/// The clip is narrowed with `IntersectClipRect` between `SaveDC` and `RestoreDC`, which is
/// the idiom [`paint_rounded`] already uses. There is no region handle to forget on an exit
/// path because there is no region handle at all.
///
/// # NFR-13, and what each refusal costs
///
/// - `SaveDC` answering zero means the clip could not be put back, so it is not narrowed:
///   the loop stops where it is rather than draw with a clip it cannot undo.
/// - `IntersectClipRect` answering `RGN_ERROR` means the clip was not narrowed either, and a
///   pass drawn under the caller's own clip would put every line of the caption on the page at
///   once. The loop stops there too.
/// - `NULLREGION` is not a refusal but the ordinary case of a partial repaint: the band is
///   outside what the caller is redrawing, there is nothing to paint in it, and the line counts
///   as done.
///
/// A stop at the very first line answers zero, and [`paint_label`] spends that on the single
/// plain call — the picture of before this task, whole.
///
/// # Safety
///
/// Called from [`paint_label`] with values copied out of the `WM_DRAWITEM` message: `dc` is
/// owned by the sender for the length of the send, and `caption` and `rect` are live locals.
unsafe fn paint_label_lines(
    dc: HDC,
    rect: RECT,
    caption: &mut [u16],
    lines: i32,
    natural: i32,
    pitch: i32,
) -> i32 {
    let mut painted = 0;

    for index in 0..lines {
        // The band this line is to end up in, **clamped to the rectangle of the control**: an
        // owner-drawn static answers for its own rectangle and for not one pixel outside it,
        // and the pass below draws into a rectangle shifted down the page, so this clamp is
        // what keeps the ink at home. A band that has fallen off the bottom entirely comes out
        // empty here and `IntersectClipRect` answers `NULLREGION` for it — see below.
        let top = (rect.top + index * pitch).min(rect.bottom);
        let bottom = (top + natural).min(rect.bottom);

        // SAFETY: `dc` is a handle passed by value; the call pushes the state of the device
        // context onto its own stack and touches no memory of this process.
        let saved = unsafe { SaveDC(dc) };

        // NFR-13: examined — see the doc comment.
        if saved == 0 {
            break;
        }

        // SAFETY: `dc` is the live DC and the four numbers are plain values.
        let narrowed = unsafe { IntersectClipRect(dc, rect.left, top, rect.right, bottom) };

        // NFR-13: the region type is examined and each of the three outcomes is answered —
        // see the doc comment. The `RestoreDC` below runs on all of them.
        let drawable = narrowed != RGN_ERROR && narrowed != NULLREGION;

        if drawable {
            // The shift that puts line `index` at the top of its band: its natural place is
            // `index × natural` from the top of the rectangle, its band is `index × pitch`.
            let shift = index * (pitch - natural);

            let mut text_rect = rect;
            text_rect.top += shift;
            text_rect.bottom += shift;

            // SAFETY: `caption` and `text_rect` are live locals of this frame and the
            // caller's; the format has no `DT_MODIFYSTRING` and no `DT_CALCRECT`, so the call
            // reads the caption and writes only pixels of the DC — and only the pixels of the
            // band, because of the clip narrowed just above.
            unsafe { DrawTextW(dc, caption, &mut text_rect, LABEL_TEXT_FORMAT) };
        }

        // SAFETY: `saved` is the state this loop pushed a few lines above, and nothing between
        // the two calls pushed another.
        let _ = unsafe { RestoreDC(dc, saved) };

        if narrowed == RGN_ERROR {
            break;
        }

        painted += 1;
    }

    painted
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

/// Air on each side of the key name inside its chip, in **hundredths of the chip face's em**.
///
/// The mock-up's `padding: 2px 5px 3px` on an 11 px face: 5 / 11 = 45 %. Hundredths of the em
/// rather than mock-up pixels through [`scaled`], for the reason
/// [`PANEL_CAPTION_BLANK_PERCENT`] is a percentage of its own font's height — the figure is a
/// box around **text**, so it has to grow with the text and not with the window, and the em is
/// the one number that is the text's own at every DPI.
pub const CHIP_PAD_X_PERCENT: i32 = 45;

/// The same air above and below, in hundredths of the em — the mock-up's 2 px over and 3 px
/// under an 11 px face, halved into one number because GDI has no asymmetric box here.
pub const CHIP_PAD_Y_PERCENT: i32 = 14;

/// Corner radius of a chip, in hundredths of the em — the mock-up's `border-radius: 4px`.
pub const CHIP_RADIUS_PERCENT: i32 = 36;

/// The box of one chip: the width and height of the whole figure, its radius, and the inset
/// its text stands at — the pure arithmetic of the chip, closed by a table test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChipBox {
    /// Whole width of the figure, frame included.
    pub width: i32,
    /// Whole height of the figure, frame included.
    pub height: i32,
    /// Corner radius, in the pixels of the window.
    pub radius: i32,
    /// Distance from the left edge of the figure to the first glyph.
    pub inset_x: i32,
    /// Distance from the top edge of the figure to the top of the character cell.
    pub inset_y: i32,
}

/// The box a key name of `text_width` pixels, set in a face of `em` pixels and `line` pixels
/// tall, wants around it — task Т-26-2.
///
/// `em` is the absolute value of the chip face's `lfHeight` and `line` is its `tmHeight`: the
/// air is a share of the first (see the three percentages) and the interior is the second,
/// because what has to fit inside the box is the character cell the pen will draw.
///
/// `thickness` is the frame [`paint_rounded`] will stroke — it is counted twice, once on each
/// side, so the interior is not eaten by it.
///
/// Pure: no DC, no handle, no window.
pub fn chip_box(text_width: i32, em: i32, line: i32, thickness: i32) -> ChipBox {
    let em = em.abs();

    let pad_x = (em * CHIP_PAD_X_PERCENT) / 100;
    let pad_y = (em * CHIP_PAD_Y_PERCENT) / 100;

    ChipBox {
        width: text_width + 2 * (pad_x + thickness),
        height: line + 2 * (pad_y + thickness),
        radius: (em * CHIP_RADIUS_PERCENT) / 100,
        inset_x: pad_x + thickness,
        inset_y: pad_y + thickness,
    }
}

/// The two colours and the brush one chip is drawn in — the fields of the palette решение 85
/// names, carried together so the drawing takes a value instead of three arguments.
#[derive(Clone, Copy)]
pub struct ChipColors {
    /// `field_border` — the frame.
    pub outline: COLORREF,
    /// `field_bg` — the fill.
    pub fill: HBRUSH,
    /// `text` — the key name itself.
    pub ink: COLORREF,
}

/// One help row split at its placeholder: what stands before the key, the key, and what
/// stands after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChipRow<'a> {
    /// Everything before [`KEY_PLACEHOLDER`].
    pub prefix: &'a str,
    /// The key name, or `None` for a row with no placeholder — rows 4 and 5 of the help.
    pub key: Option<&'a str>,
    /// Everything after the placeholder; empty when there is none.
    pub suffix: &'a str,
}

/// Splits one help row at its placeholder — pure, and the whole of «где стоит чип».
///
/// A template with no [`KEY_PLACEHOLDER`] is all prefix and no key, which is rows 4 and 5 of
/// the help word for word. An **empty** key name is treated the same way and the two halves
/// are drawn as one sentence: an empty chip would be a box around nothing.
pub fn chip_row<'a>(template: &'a str, key: &'a str) -> ChipRow<'a> {
    match template.split_once(KEY_PLACEHOLDER) {
        Some((prefix, suffix)) if !key.is_empty() => ChipRow {
            prefix,
            key: Some(key),
            suffix,
        },
        Some((prefix, _)) => ChipRow {
            prefix,
            key: None,
            suffix: "",
        },
        None => ChipRow {
            prefix: template,
            key: None,
            suffix: "",
        },
    }
}

/// Everything about a chip row that is not the words: the ground, the ink, the two faces, the
/// chip's own colours and the geometry.
///
/// A value rather than eight arguments — `clippy::too_many_arguments` is a denied warning in
/// this crate, and a row of this window needs every one of these.
#[derive(Clone, Copy)]
pub struct ChipRowStyle {
    /// The ground of the whole rectangle, laid before a word is written.
    pub ground: HBRUSH,
    /// The ink of the sentence — `text`.
    pub ink: COLORREF,
    /// The face of the sentence.
    pub body: Option<HFONT>,
    /// The face of the key name inside the chip, and that face's `lfHeight`.
    pub chip_face: Option<(HFONT, i32)>,
    /// The colours of the chip.
    pub chip: ChipColors,
    /// The distance from one line of the row to the next, in pixels of the window.
    pub pitch: i32,
    /// The DPI of the window, for [`paint_rounded`] and the frame thickness.
    pub dpi: i32,
}

/// One word of a laid-out row: a run of the sentence, or the chip.
#[derive(Clone, Copy)]
enum Atom<'a> {
    /// A word of the sentence, drawn in the body face.
    Word(&'a str),
    /// The chip, drawn as a figure with the key name inside it.
    Chip(&'a str),
}

/// One atom with its measured width and whether a space stands before it.
struct Placed<'a> {
    atom: Atom<'a>,
    width: i32,
    space_before: bool,
}

/// Paints one help row of the about window — the sentence, wrapped, with the chip of решение
/// 85 standing where the placeholder was.
///
/// Answers 1 always: the ground is laid first and unconditionally, so the rectangle is drawn
/// however little of the text could be. A row whose faces could not be made (`None` in the
/// style) is drawn in the manager's own face — degraded-but-alive, exactly as [`paint_label`]
/// treats the same refusal (NFR-13).
///
/// ⚠ **The clip is narrowed to the rectangle of the control for the whole of the drawing.** An
/// owner-drawn static answers for its own rectangle and for not one pixel outside it, and this
/// function places lines by arithmetic rather than by `DrawTextW` — a sentence one line taller
/// than the template allowed would otherwise write on its neighbour. `SaveDC`/`RestoreDC` put
/// the caller's clip back, and no region handle is created, so none can leak — the idiom
/// [`paint_label_lines`] and [`paint_rounded`] already keep.
///
/// # Safety
///
/// Called with values copied out of the `WM_DRAWITEM` message the caller is inside of: `dc` is
/// owned by the sender for the length of the send, and every handle in `style` is an object
/// the window's state owns for longer than the drawing.
pub unsafe fn paint_chip_row(dc: HDC, rect: RECT, row: ChipRow<'_>, style: ChipRowStyle) -> isize {
    // The ground, first and always — п. 2 of criterion 2 of T-12-6, the rule every body that
    // draws a whole element keeps.
    //
    // SAFETY: `dc` is the DC of the message, `rect` a live local, `ground` a live brush.
    unsafe { FillRect(dc, &rect, style.ground) };

    // SAFETY: `dc` is a handle passed by value; the call writes an attribute of the DC.
    unsafe { SetBkMode(dc, TRANSPARENT) };

    // SAFETY: `dc` is a handle passed by value; the state is put back below on every path.
    let saved = unsafe { SaveDC(dc) };

    // NFR-13: zero is «the state could not be saved», and a clip that cannot be put back is
    // not narrowed. The row is then not drawn at all rather than drawn over its neighbours.
    if saved == 0 {
        return 1;
    }

    // SAFETY: `dc` is the live DC and the four numbers are plain values.
    let narrowed = unsafe { IntersectClipRect(dc, rect.left, rect.top, rect.right, rect.bottom) };

    if narrowed != RGN_ERROR && narrowed != NULLREGION {
        // SAFETY: see the caller — the clip is narrowed to the control's own rectangle and the
        // handles of `style` are alive for longer than this call.
        unsafe { lay_chip_row(dc, rect, row, style) };
    }

    // SAFETY: `saved` is the state this function pushed above, and nothing between the two
    // calls pushed another.
    let _ = unsafe { RestoreDC(dc, saved) };

    // TRUE — the row is drawn.
    1
}

/// The body of [`paint_chip_row`], inside the narrowed clip: measure, wrap, draw.
///
/// # Safety
///
/// As [`paint_chip_row`], and with the clip already narrowed to `rect`.
unsafe fn lay_chip_row(dc: HDC, rect: RECT, row: ChipRow<'_>, style: ChipRowStyle) {
    let thickness = scaled(BORDER_THICKNESS, style.dpi).max(1);

    // The body face goes in first: every width below is measured in the face the word will be
    // drawn in, and the chip's own face is selected only for the chip's own two calls.
    //
    // SAFETY: `dc` is the DC of the message and the face is the window's; put back at the end.
    let previous_face = unsafe { select_face(dc, style.body) };

    // The two metrics the placing needs: the ascent the sentence sits on, and the height of
    // one line of it. NFR-13 — a DC that will not answer leaves the row unlaid, which is the
    // ground and nothing else.
    //
    // SAFETY: the face is selected above; the buffer is a live local this call fills.
    let Some(body_metrics) = (unsafe { face_metrics(dc) }) else {
        // SAFETY: `previous_face` is what `select_face` answered for this same DC.
        unsafe { restore_face(dc, previous_face) };
        return;
    };

    // SAFETY: as above — the width of one space in the face now in the DC.
    let space = unsafe { text_width(dc, " ") };

    // The chip's own metrics, in its own face, or `None` for a row with no chip and for a
    // window whose chip face could not be made.
    let chip = match (row.key, style.chip_face) {
        (Some(key), Some((face, em))) => {
            // SAFETY: `dc` is the DC of the message and `face` is the window's own; the body
            // face is put back immediately after the two measurements.
            let previous = unsafe { select_face(dc, Some(face)) };

            // SAFETY: the chip face is in the DC for exactly these two calls.
            let measured = unsafe { face_metrics(dc).map(|m| (m, text_width(dc, key))) };

            // SAFETY: `previous` is what `select_face` answered a moment ago.
            unsafe { restore_face(dc, previous) };

            measured.map(|(metrics, width)| {
                (
                    face,
                    chip_box(width, em, metrics.height, thickness),
                    metrics,
                )
            })
        }
        _ => None,
    };

    // A row whose chip could not be made or measured keeps its key name all the same: the two
    // halves of the template are folded back into one sentence and the row is drawn as it was
    // drawn before this task — degraded-but-alive, and the whole loss is the figure (NFR-13).
    // Declared before `atoms`, because the words below borrow from it.
    let folded = match (&chip, row.key) {
        (None, Some(key)) => Some(format!("{}{key}{}", row.prefix, row.suffix)),
        _ => None,
    };

    // The words, in reading order, each with its own width.
    let mut atoms: Vec<Placed<'_>> = Vec::new();

    match (&chip, row.key) {
        (Some((_, box_of, _)), Some(key)) => {
            // SAFETY: the body face is in the DC; every width below is measured in it.
            unsafe { push_words(dc, &mut atoms, row.prefix, false) };

            atoms.push(Placed {
                atom: Atom::Chip(key),
                width: box_of.width,
                space_before: row.prefix.ends_with(char::is_whitespace),
            });

            let space_after = row.suffix.starts_with(char::is_whitespace);

            // SAFETY: the body face is in the DC — the chip's face was put back above.
            unsafe { push_words(dc, &mut atoms, row.suffix, space_after) };
        }
        _ => {
            let plain = folded.as_deref().unwrap_or(row.prefix);

            // SAFETY: as above.
            unsafe { push_words(dc, &mut atoms, plain, false) };
        }
    }

    // SAFETY: see the caller — the clip is narrowed and every handle is alive.
    unsafe { draw_atoms(dc, rect, &atoms, space, &style, chip, body_metrics) };

    // SAFETY: `previous_face` is what `select_face` answered for this same DC.
    unsafe { restore_face(dc, previous_face) };
}

/// Appends the words of `text` to `atoms`, each measured in the face now in `dc`.
///
/// `leading_space` says whether a space stands before the **first** of them — the two halves of
/// a template carry their own spacing, and a chip glued to a colon must stay glued to it.
///
/// # Safety
///
/// `dc` is live and carries the face the words will be drawn in.
unsafe fn push_words<'a>(dc: HDC, atoms: &mut Vec<Placed<'a>>, text: &'a str, leading_space: bool) {
    let mut space_before = leading_space;

    for word in text.split_whitespace() {
        // SAFETY: see the contract — the width is measured in the face now in the DC.
        let width = unsafe { text_width(dc, word) };

        atoms.push(Placed {
            atom: Atom::Word(word),
            width,
            space_before,
        });

        space_before = true;
    }
}

/// Draws the laid-out row: greedy wrapping by clusters, one line every `style.pitch` pixels.
///
/// A «cluster» is one word that may start a line together with everything glued to it — the
/// atoms whose `space_before` is false. Breaking inside one would put a colon at the start of
/// a line, or take the chip away from the punctuation that follows it.
///
/// # Safety
///
/// `dc` is live, carries the body face, and is clipped to `rect`; every handle of `style` is
/// alive for longer than this call.
unsafe fn draw_atoms(
    dc: HDC,
    rect: RECT,
    atoms: &[Placed<'_>],
    space: i32,
    style: &ChipRowStyle,
    chip: Option<(HFONT, ChipBox, FaceMetrics)>,
    body: FaceMetrics,
) {
    let limit = rect.right - rect.left;

    let mut x = 0;
    let mut line = 0;

    // SAFETY: `dc` is a handle passed by value; the call writes an attribute of the DC.
    unsafe { SetTextColor(dc, style.ink) };

    let mut index = 0;

    while index < atoms.len() {
        // The cluster: this atom and everything glued to it.
        let start = index;
        let mut width = atoms[index].width;

        index += 1;

        while index < atoms.len() && !atoms[index].space_before {
            width += atoms[index].width;
            index += 1;
        }

        let lead = if x == 0 {
            0
        } else if atoms[start].space_before {
            space
        } else {
            0
        };

        // The break: a cluster that does not fit and is not the first on its line starts the
        // next one. A cluster wider than the whole rectangle stands where it is and is cut off
        // by the clip, which is what a word too long for its label has always cost here.
        if x > 0 && x + lead + width > limit {
            line += 1;
            x = 0;
        } else {
            x += lead;
        }

        let top = rect.top + line * style.pitch;

        for placed in &atoms[start..index] {
            match placed.atom {
                Atom::Word(word) => {
                    // SAFETY: `dc` carries the body face and the ink set above; the buffer is a
                    // live local of this frame.
                    unsafe { draw_run(dc, rect.left + x, top, word) };
                }
                Atom::Chip(key) => {
                    if let Some(chip) = chip {
                        // SAFETY: see the caller — every handle is alive and the DC is clipped.
                        unsafe { draw_chip(dc, rect.left + x, top, key, chip, style, body) };

                        // The ink the sentence is drawn in, back after the figure.
                        //
                        // SAFETY: `dc` is a handle passed by value.
                        unsafe { SetTextColor(dc, style.ink) };
                    }
                }
            }

            x += placed.width;
        }
    }
}

/// Draws one chip: the figure of [`paint_rounded`], then the key name inside it.
///
/// # Safety
///
/// `dc` is live and clipped to the label's rectangle; `face` and the brush of `style.chip` are
/// objects the window's state owns for longer than this call. The body face is put back by the
/// caller, which is why this one restores whatever it found.
unsafe fn draw_chip(
    dc: HDC,
    x: i32,
    top: i32,
    key: &str,
    chip: (HFONT, ChipBox, FaceMetrics),
    style: &ChipRowStyle,
    body: FaceMetrics,
) {
    let (face, box_of, metrics) = chip;

    // The baseline of the chip's own text is put on the baseline of the sentence, and the
    // figure is hung around it: that is what makes the box sit *on the line* rather than beside
    // it. `top` is the top of the sentence's character cell, so the chip's cell starts as much
    // lower as its ascent is shorter.
    let cell = top + (body.ascent - metrics.ascent);

    let area = RECT {
        left: x,
        top: cell - box_of.inset_y,
        right: x + box_of.width,
        bottom: cell - box_of.inset_y + box_of.height,
    };

    paint_rounded(
        dc,
        &area,
        box_of.radius,
        style.chip.outline,
        style.chip.fill,
        style.dpi,
    );

    // SAFETY: `dc` is the DC of the message and `face` is the window's own; put back below.
    let previous = unsafe { select_face(dc, Some(face)) };

    // SAFETY: `dc` is a handle passed by value.
    unsafe { SetTextColor(dc, style.chip.ink) };

    // SAFETY: `dc` carries the chip face and the ink set above.
    unsafe { draw_run(dc, x + box_of.inset_x, cell, key) };

    // SAFETY: `previous` is what `select_face` answered for this same DC a moment ago.
    unsafe { restore_face(dc, previous) };
}

/// Writes one run of text at the top-left corner of its character cell.
///
/// `TextOutW` and not `DrawTextW`: the run is placed by this file's own arithmetic, and
/// `DrawTextW` would bring its own alignment, its own tab expansion and its own `&`-prefix
/// rule — the very disagreement between measuring and drawing that [`paint_label_lines`]
/// warns about. `GetTextExtentPoint32W` and `TextOutW` are one instrument.
///
/// # Safety
///
/// `dc` is live and carries the face and the ink the run is to be drawn in.
unsafe fn draw_run(dc: HDC, x: i32, y: i32, text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();

    if units.is_empty() {
        return;
    }

    // NFR-13: the flag is answered and deliberately dropped, for the reason every paint call
    // of this file drops its own.
    //
    // SAFETY: `dc` is the live DC and `units` is a live local of this frame, read by the call.
    let _ = unsafe { TextOutW(dc, x, y, &units) };
}

/// The width of one run in the face now in `dc`, in pixels.
///
/// # Safety
///
/// `dc` is live and carries the face the run would be drawn in.
unsafe fn text_width(dc: HDC, text: &str) -> i32 {
    let units: Vec<u16> = text.encode_utf16().collect();

    if units.is_empty() {
        return 0;
    }

    let mut size = SIZE::default();

    // SAFETY: `units` and `size` are live locals of this frame; the call reads the first and
    // writes the second.
    if unsafe { GetTextExtentPoint32W(dc, &units, &raw mut size) }.as_bool() {
        size.cx
    } else {
        // NFR-13: a refused measurement is nothing rather than a wrong number — the run then
        // overlaps its neighbour instead of standing at a random distance from it.
        0
    }
}

/// The two numbers a laid-out line needs of a face: where its baseline is and how tall one
/// line of it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceMetrics {
    /// `tmAscent` — from the top of the character cell down to the baseline.
    pub ascent: i32,
    /// `tmHeight` — the whole character cell, ascent and descent together.
    pub height: i32,
}

/// The metrics of the face now in `dc`, or `None` for a DC that would not answer (NFR-13).
///
/// # Safety
///
/// `dc` is live and carries the face the numbers are wanted of.
unsafe fn face_metrics(dc: HDC) -> Option<FaceMetrics> {
    let mut metrics = TEXTMETRICW::default();

    // SAFETY: `metrics` is a live local of this frame, which the call fills.
    if !unsafe { GetTextMetricsW(dc, &raw mut metrics) }.as_bool() {
        return None;
    }

    Some(FaceMetrics {
        ascent: metrics.tmAscent,
        height: metrics.tmHeight,
    })
}

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

/// Top of the **first** row of a list below the inner edge of its frame, in mock-up pixels —
/// the `$ry = $py + 3 + $i * $rowH` both list arms of the generator start from.
///
/// A list box positions its own rows, starting at the top of its client area, and no message
/// moves them; the one lever this program holds over the distance between the frame and the
/// first row is therefore **where the frame is drawn** — the frame is the dialog's own
/// background (`settings::on_erase_background`), and for the two lists it is lifted this far
/// above the control instead of the [`BORDER_THICKNESS`] every field gets.
pub const LIST_FIRST_ROW_TOP: i32 = 3;

/// Width of a character the DC measures as nothing, as a percentage of the caption font's
/// height — п. 2.1, the explicit space width.
const PANEL_CAPTION_BLANK_PERCENT: i32 = 28;

/// The pitch a wrapped label is to be drawn at, or `None` for «leave it to the one plain
/// call» — the whole decision of task T-12-12 as a pure function a table test can close.
///
/// `lines` and `natural` are what `measure_label_lines` answered and `model` is
/// [`LABEL_LINE_PITCH`] through [`scaled`]. Two questions, and either «no» is the
/// drawing of before this task:
///
/// 1. **Does the caption wrap at all?** One line has no pitch, and seventeen of the eighteen
///    `settings::OWNER_DRAWN_LABELS` are one line — this is the question that keeps them
///    still, and the one that makes the change «fixed the line pitch» rather than «rewrote
///    the label drawing».
/// 2. **Has the model pitch anything to give?** `DrawTextW` already advances a line by
///    `natural`, so a model pitch that is not larger is either the same drawing or a squeeze,
///    and neither is what В-6 asked for. At 96 DPI the two are 16 against 15.
///
/// # Why the height of the control is not a third question
///
/// It was, in the first draft of this task, and it was wrong. `IDC_LOG_DIR` is 16 dialog units
/// — **30 px** — and two lines at the model pitch reach 1 × 16 + 15 = **31**, so a height
/// question would have refused the model pitch to precisely the label the task names as having
/// to get it («подпись, которая перенеслась бы, обязана получить модельный шаг»).
///
/// Nothing is risked by leaving it out: an owner-drawn static may not paint outside its own
/// rectangle, and that is enforced where it belongs — `paint_label_lines` clamps
/// every band of its clip to the rectangle, so a line the control is too short for is cut off
/// by the very same edge that cuts it off today, and no ink can reach a neighbour.
pub fn label_model_pitch(lines: i32, natural: i32, model: i32) -> Option<i32> {
    if lines < 2 || model <= natural {
        return None;
    }

    Some(model)
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

/// The two distances the frame of a list stands off the rectangle of its control, in pixels of
/// a window at `dpi`: the air **above** it and the thickness at the other three sides.
///
/// Pure, and the one place either number is worked out — `settings::paint_background` draws
/// that frame from the outside and [`list_frame_box`] names the very same figure from inside
/// the control, so the two could disagree by a pixel and put a corner arc where the interior
/// does not end. They cannot now: both ask this.
///
/// ⚠ The air is [`LIST_FIRST_ROW_TOP`] **through [`scaled`]**, which is 2 px at 96 DPI and not
/// the 3 of the mock-ups — the number this program has always drawn with, written down here
/// where it can be read. `max(border)` because a frame thinner than its own thickness is not a
/// frame.
pub fn list_frame_air(dpi: i32) -> (i32, i32) {
    let border = scaled(BORDER_THICKNESS, dpi).max(1);

    (scaled(LIST_FIRST_ROW_TOP, dpi).max(border), border)
}

/// The frame of a list in the **control's own client coordinates** — the same figure
/// `settings::paint_background` paints in the coordinates of the dialog, seen from inside the
/// window it surrounds — task T-12-5.
///
/// `client` is what `GetClientRect` answers for the list, so the rectangle this returns starts
/// at negative numbers: the frame stands outside the control on all four sides, and only the
/// four corner arcs of it reach back in over the interior. That is the whole point — the arc
/// has to be drawn by whoever owns those pixels, and inside the control that is the control.
///
/// Pure, so the geometry is a table a test can read without a window.
pub fn list_frame_box(client: &RECT, dpi: i32) -> RECT {
    let (top, border) = list_frame_air(dpi);

    RECT {
        left: client.left - border,
        top: client.top - top,
        right: client.right + border,
        bottom: client.bottom + border,
    }
}

/// Half of what the `settings::FIELD_BOX_DLU` box has left over after the control inside it —
/// the distance the frame of a field is drawn above its rectangle, and the same below it.
///
/// `box_height` is that box in the pixels of the window (`None` when `MapDialogRect` was
/// refused — NFR-13), `control_height` the height the dialog manager gave the control, and
/// `border` the thickness of the frame itself, which is the floor: a control already as tall
/// as the box, or taller, keeps exactly the one-thickness-outside frame it wore before task
/// T-12-3, and so does a refused measurement. That is what makes the rule **degenerate into
/// the old one** rather than replace it.
///
/// Pure, so the arithmetic is a table in `tests\settings.rs` and not a picture to be read.
pub fn field_frame_air(box_height: Option<i32>, control_height: i32, border: i32) -> i32 {
    let Some(box_height) = box_height else {
        return border;
    };

    ((box_height - control_height) / 2).max(border)
}

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

/// The colour role a control plays when `WM_CTLCOLORSTATIC` asks about it — FR-92а,
/// task T-11-4.
///
/// Three roles and not eighteen colours: the handler below turns a role into palette fields
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

/// One [`StaticColorRole`] resolved against the palette — the single place a role of a
/// **label** becomes an ink, shared by the labels of both windows (§6.2: one body, not two).
///
/// The identifier never appears here: which role a control plays is the business of
/// `settings::static_color_role` and `settings::about_static_color_role`, and this function
/// only turns the answer into a colour. That split is what keeps task T-11-18 from growing a
/// second «identifier → colour» list beside the one task T-11-4 wrote.
///
/// `Field` is spelled out and not swallowed by a catch-all so the match stays closed over the
/// shared vocabulary: no label of either template is a field — the one control that answers
/// `Field` is `IDC_HOTKEY`, an `EDITTEXT`, which draws its own text and is not owner-drawn at
/// all — and if one ever became one, `text` on `field_bg` is what `WM_CTLCOLORSTATIC` already
/// answers for it.
pub(crate) fn label_ink(role: StaticColorRole, palette: &Palette) -> COLORREF {
    match role {
        StaticColorRole::Label | StaticColorRole::Field => palette.text,
        StaticColorRole::Muted => palette.text_muted,
    }
}

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
    /// **No frame**: the ink of the outline is the colour the face is filled with, so the one
    /// `RoundRect` of [`paint_rounded`] draws a fill and nothing else — the mock-up's button.
    ///
    /// Deliberately not a colour of its own and not a new palette field: the pen simply takes
    /// whichever colour [`ButtonFaceRole`] already chose, which is why «no frame» invents
    /// nothing. [`resolve_button_colors`] is where the two meet.
    FaceItself,
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

/// One solid brush of a colour the drawing needs **only while the cursor stands still on one
/// control**, made for the length of a single drawing and freed with the value — FR-92а,
/// task T-12-8 for [`Palette::hover_bg`] and task T-15-2 for [`Palette::sel_fg`], the face
/// the accented «ОК» answers the pointer with. Neither field is in [`Brushes`], and the
/// paragraph below is why neither belongs there.
///
/// # Why this is not a brush of [`Brushes`]
///
/// That set owns the brushes a `WM_CTLCOLOR*` answer **hands to the window manager**, and a
/// handed-out brush has to outlive every paint the manager may use it for — which is why the
/// set is created whole, kept for as long as a palette is on screen and recreated rather than
/// mutated. `hover_bg` is not handed to anybody: it is selected into the DC of one paint by
/// [`paint_rounded`] and is gone before that paint returns. That is the shape the outline pen
/// of [`stroke_rounded`] already has in this file — made, used inside one drawing, deleted —
/// and it is the honest ownership for a colour that is on the screen only while the cursor
/// stands still on one control.
///
/// The value is created inside the colour choice and travels with the resolved colours, so it
/// is dropped exactly where the drawing that uses it ends.
pub(crate) struct HotBrush(HBRUSH);

impl HotBrush {
    /// The brush of one colour, or `None` when GDI refused (NFR-13).
    ///
    /// A refusal is survived by the caller filling with the quiet face it already holds: the
    /// button or the combo box then simply does not light up, which is the picture every task
    /// before T-12-8 drew. Nothing is journaled, for the reason [`Brushes::new`] gives
    /// at its own refusal — `CreateSolidBrush` promises no last-error code, and the closed
    /// operation vocabulary of `diag` has no row for GDI.
    pub(crate) fn new(color: COLORREF) -> Option<Self> {
        // SAFETY: takes one colour by value, reads no memory of ours and answers a handle
        // which becomes the property of this value and is freed exactly once, in `Drop`.
        let brush = unsafe { CreateSolidBrush(color) };

        if brush.is_invalid() {
            return None;
        }

        Some(Self(brush))
    }

    /// The handle, borrowed — this value frees it, nobody else.
    pub(crate) fn brush(&self) -> HBRUSH {
        self.0
    }
}

impl Drop for HotBrush {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from the successful `CreateSolidBrush` of `new`, was selected
        // out of every DC it was selected into by the drawing that used it, and is freed
        // exactly once, here. The `BOOL` is dropped for the reason `stroke_rounded` drops its
        // own: nothing can be done about a refused cleanup and the journal has no row for GDI.
        let _ = unsafe { DeleteObject(self.0.into()) };
    }
}

/// The colours of one owner-drawn push button, resolved out of the palette: the ground it
/// stands on, the face brush, the caption ink and the frame brush. What
/// [`resolve_button_colors`] answers and the whole of what `settings::paint_push_button` needs.
///
/// ⚠ The `face` of the hot state is a brush **this value does not own** — it is the transient
/// [`HotBrush`] the resolver answers beside these colours, and the caller holds that one alive
/// for the length of the drawing. See [`resolve_button_colors`].
#[derive(Clone, Copy)]
pub(crate) struct ResolvedButtonColors {
    /// What the whole rectangle is erased with before the face goes on it — the brush
    /// `WM_CTLCOLORBTN` answers for this control. Not a role but a fact of geometry: the
    /// caller reads it off the panel map, exactly as `on_ctl_color` does (task T-12-6).
    pub(crate) ground: HBRUSH,
    /// What the button face is filled with.
    pub(crate) face: HBRUSH,
    /// What the caption is drawn with.
    pub(crate) ink: COLORREF,
    /// What the single-pixel frame is drawn with. An ink and not a brush since task
    /// T-11-13: the frame and the face are one rounded `RoundRect` now, and `RoundRect`
    /// frames with the selected *pen*.
    pub(crate) border: COLORREF,
}

/// The single place a button role becomes a brush or a colour of the resolved palette —
/// the drawing never sees a role. One body shared by the settings dialog and the about
/// dialog (§6.2, task T-11-11), moved out of `on_draw_item` rather than copied.
///
/// ⚠ **Two values leave here since task T-12-8**, and the second one is not decoration: the hot
/// face is filled with a brush made for this one drawing (see [`HotBrush`]), and the colours
/// only *name* it. The caller must keep the answered [`HotBrush`] alive until the painting is
/// over — every caller binds it and lets it drop when its own frame ends, which is the length
/// of the paint. `None` for every other face, and for a refused `CreateSolidBrush`.
pub(crate) fn resolve_button_colors(
    colors: ButtonColors,
    ground: HBRUSH,
    brushes: &Brushes,
    palette: &Palette,
) -> (ResolvedButtonColors, Option<HotBrush>) {
    // The two faces of this table the window's brush set does not hold — `hover_bg` since task
    // T-12-8 and `sel_fg` since task T-15-2; see [`HotBrush`] for why they are made here
    // instead of being owned there. A refused `CreateSolidBrush` leaves `None`, and the face
    // below falls back to the quiet `button_bg`: the button then looks exactly as it did
    // before those tasks (NFR-13).
    let hot = match colors.face {
        ButtonFaceRole::HoverBg => HotBrush::new(palette.hover_bg),
        ButtonFaceRole::SelFg => HotBrush::new(palette.sel_fg),
        ButtonFaceRole::ButtonBg | ButtonFaceRole::AccentBg | ButtonFaceRole::SelBg => None,
    };

    let face = match colors.face {
        ButtonFaceRole::ButtonBg => brushes.button_bg(),
        ButtonFaceRole::AccentBg => brushes.accent_bg(),
        ButtonFaceRole::SelBg => brushes.sel_bg(),
        ButtonFaceRole::HoverBg | ButtonFaceRole::SelFg => hot
            .as_ref()
            .map_or_else(|| brushes.button_bg(), HotBrush::brush),
    };

    let ink = match colors.text {
        ButtonTextRole::Text => palette.text,
        ButtonTextRole::AccentFg => palette.accent_fg,
        ButtonTextRole::SelFg => palette.sel_fg,
        ButtonTextRole::TextMuted => palette.text_muted,
    };

    // Task T-12-4: «no frame» is the pen taking the colour of the fill, so the single
    // `RoundRect` of the drawing leaves a face and no outline. The face's own colour and not a
    // new one — the whole point of [`ButtonBorderRole::FaceItself`].
    let border = match colors.border {
        ButtonBorderRole::ButtonBorder => palette.button_border,
        ButtonBorderRole::FaceItself => match colors.face {
            ButtonFaceRole::ButtonBg => palette.button_bg,
            ButtonFaceRole::AccentBg => palette.accent_bg,
            ButtonFaceRole::SelBg => palette.sel_bg,
            // ⚠ **Это сочетание сегодня недостижимо, и ветвь всё равно написана верно.**
            // `FaceItself` ставится единственным местом — `about_button_colors`, то есть
            // только для кнопок окна «О программе»; а `HoverBg` рождается в
            // `button_color_roles` ниже ворот `if control == OK_COMMAND`, куда «ОК» этого
            // окна не доходит. Пара «безрамочная кнопка под курсором» не возникает.
            //
            // ⛔ Не удалять и не сливать с соседом: соседнее плечо `SelFg` было ровно таким
            // же мёртвым до задачи T-15-2 и ожило от одной правки таблицы ролей. День, когда
            // безрамочной кнопке дадут отклик на наведение, оживит и это — а слитая или
            // выброшенная ветвь молча покрасит рамку не тем цветом. `unreachable!()` тоже
            // нельзя: паника внутри `WM_DRAWITEM` — прямое нарушение NFR-13.
            ButtonFaceRole::HoverBg => palette.hover_bg,
            ButtonFaceRole::SelFg => palette.sel_fg,
        },
    };

    (
        ResolvedButtonColors {
            ground,
            face,
            ink,
            border,
        },
        hot,
    )
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
    /// [`Palette::field_bg`] — the quiet ground of every glyph but one.
    FieldBg,
    /// [`Palette::accent_bg`] — the checked, enabled check box, filled whole.
    AccentBg,
}

/// The frame of the glyph. One variant on purpose, exactly as [`ButtonBorderRole`]: the
/// closed table frames every framed cell with the same [`Palette::box_border`], and
/// a second frame colour cannot appear without widening this enum first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphFrameRole {
    /// [`Palette::box_border`] — the field the palette names for exactly this.
    BoxBorder,
}

/// The mark inside a checked glyph — the check strokes of a box, the dot of a radio —
/// named as the palette field it is cut from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphMarkRole {
    /// [`Palette::accent_fg`] — the check mark over the accent-filled box.
    AccentFg,
    /// [`Palette::accent_bg`] — the dot of a checked, enabled radio button.
    AccentBg,
    /// [`Palette::box_border`] — the muted mark of a checked but disabled glyph.
    BoxBorder,
}

/// The ink of the caption to the right of the glyph, named as the palette field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphTextRole {
    /// [`Palette::text`] — the ordinary caption.
    Text,
    /// [`Palette::text_muted`] — the caption of a disabled element (the method
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
/// Roles and not colours, exactly as `settings::button_color_roles` before it: not a single
/// colour number enters the module that owns the window — §6.2 gives every palette value to
/// `theme` alone.
///
/// The shape of the table:
/// - **the checked, enabled check box** is the one cell the accent covers whole: `accent_bg`
///   fill edge to edge, `accent_fg` check mark, and no frame — the task words it so;
/// - **the checked, enabled radio** keeps the quiet ground and shows the accent as the dot:
///   an accent-filled circle would hide an accent dot, so the accent moves inside;
/// - **запрещённость гасит акцент**, the precedent `settings::button_color_roles` set: a
///   disabled element must not advertise itself, so a checked-but-disabled glyph drops to the
///   `box_border` mark on the ordinary ground — still readable as checked, no longer loud;
/// - every other cell is the quiet ground itself: `field_bg` fill, `box_border` frame,
///   no mark.
///
/// Focus is deliberately absent here, as it is in `settings::button_color_roles`: `ODS_FOCUS`
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

/// What the ground of one combo-box item is filled with, named as the palette field —
/// FR-92а, task T-11-6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboFillRole {
    /// [`Palette::field_bg`] — the quiet ground of an ordinary list item and of the
    /// closed face: a combo is a field to the eye, wherever it sits.
    FieldBg,
    /// [`Palette::sel_bg`] — the highlighted item of the dropped-down list.
    SelBg,
    /// [`Palette::hover_bg`] — the **closed part** while the cursor stands on it
    /// (task T-12-8), and nothing else: an item of a dropped-down list is never answered
    /// this role, because the list already answers the cursor with the selection pair the
    /// keyboard moves through, and a second highlight over it would say two things at once.
    HoverBg,
}

/// The ink the item's text is drawn with, named as the palette field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboTextRole {
    /// [`Palette::text`] — the ordinary item and the closed face.
    Text,
    /// [`Palette::sel_fg`] — the pair the palette designed for the `sel_bg` ground.
    SelFg,
    /// [`Palette::text_muted`] — the value shown by a **disabled** closed face
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
/// Roles and not colours, exactly as `settings::button_color_roles` and [`glyph_color_roles`]
/// before it: not a single colour number enters the module that owns the window — §6.2 gives
/// every palette value to `theme` alone.
///
/// The shape of the table:
/// - **закрытая часть первой**: the closed face of the combo is a field to the eye, so it
///   keeps `field_bg`/`text` whatever `ODS_SELECTED` says — the manager marks the face
///   selected whenever the combo holds the focus, and a face that flipped to the selection
///   pair would sit on the dialog as a permanently lit stripe. Focus is shown by the
///   dotted `DrawFocusRect` alone, which is the drawing half's business — the same
///   decision `settings::button_color_roles` wrote down for `ODS_FOCUS`;
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
/// the closed table frames the closed face with the same [`Palette::field_border`]
/// every other field of the dialog is framed with, and a second frame colour cannot appear
/// without widening this enum first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboBorderRole {
    /// [`Palette::field_border`] — the frame the mock-ups draw around every field.
    FieldBorder,
}

/// The ink of the chevron at the right edge of a closed combo box — FR-92а, task T-11-14.
///
/// One variant, for the reason [`ComboBorderRole`] gives: the mock-ups draw the chevron in
/// the muted ink in both palettes and in every state, disabled included — it is a hint of
/// what the control does, never an advertisement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboChevronRole {
    /// [`Palette::text_muted`] — the quiet ink of the mock-ups' chevron.
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
/// Roles and not colours, exactly as `settings::button_color_roles`, [`glyph_color_roles`] and
/// [`combo_item_color_roles`] before it: not a single colour number enters the module that owns
/// the window — §6.2 gives every palette value to `theme` alone.
///
/// The shape of the table:
/// - the closed face **is a field to the eye**, so it keeps `field_bg` under the
///   `field_border` frame whatever else is true — the same surface the five input fields and
///   the two lists of `settings::FRAMED_FIELDS` wear since task T-11-13;
/// - **запрещённость гасит текст**, the precedent `settings::button_color_roles` and
///   [`glyph_color_roles`] set: a combo the mode has switched off (the pair «Источник» and
///   «Цель» under «Несколько раскладок») shows its value in `text_muted`;
/// - **запрещённая часть не «горячеет»** — the very precedence `settings::button_color_roles`
///   keeps: a combo box the mode has switched off takes no click, so the cursor standing on it
///   changes nothing at all;
/// - **горячая закрытая часть** — the cursor stands on an enabled closed face: the fill goes
///   to `hover_bg` (task T-12-8) and **nothing else moves**. The frame stays `field_border`,
///   the value keeps its ink and the chevron keeps its muted one, exactly as at rest: the
///   response is a change of ground, the way the tray menu (task T-11-21) makes it. The
///   dropped-down list is not concerned: it is another window with another paint, and the
///   items of it go on answering [`combo_item_color_roles`];
/// - the chevron and the frame do not move with the state: a muted hint stays muted, and a
///   field that lost its frame when disabled would stop reading as a field at all.
///
/// Focus is deliberately absent here, exactly as it is in `settings::button_color_roles`:
/// `ODS_FOCUS` changes no colour — it adds the dotted `DrawFocusRect`, and that is the drawing
/// half's business (the decision of task T-11-5a, kept uniform here).
pub fn combo_closed_color_roles(hot: bool, disabled: bool) -> ComboClosedColors {
    ComboClosedColors {
        fill: if hot && !disabled {
            ComboFillRole::HoverBg
        } else {
            ComboFillRole::FieldBg
        },
        border: ComboBorderRole::FieldBorder,
        text: if disabled {
            ComboTextRole::TextMuted
        } else {
            ComboTextRole::Text
        },
        chevron: ComboChevronRole::TextMuted,
    }
}

/// One [`ComboFillRole`] resolved against the brushes of the dialog — the single place these
/// roles become a brush, shared by the item drawing, the row drawing of the exclusion
/// list and the closed-face drawing of the subclass (§6.2: one body, not three copies).
///
/// ⚠ [`ComboFillRole::HoverBg`] answers the **quiet** `field_bg` here, and that is not a
/// mistake: the window's brush set has no `hover_bg` brush to hand out (see [`HotBrush`] for
/// why it is not one of the eight), and the one drawing that can be answered that role —
/// `settings::draw_combo_closed_part` — makes the transient brush itself and never reaches this
/// line. A caller that did reach it fills with the ground the closed face wears at rest, which
/// is the picture every task before T-12-8 drew (NFR-13).
pub(crate) fn combo_fill_brush(role: ComboFillRole, brushes: &Brushes) -> HBRUSH {
    match role {
        ComboFillRole::FieldBg | ComboFillRole::HoverBg => brushes.field_bg(),
        ComboFillRole::SelBg => brushes.sel_bg(),
    }
}

/// One [`ComboTextRole`] resolved against the palette — the single place these three roles
/// become an ink, shared by the same three drawings [`combo_fill_brush`] serves.
pub(crate) fn combo_text_ink(role: ComboTextRole, palette: &Palette) -> COLORREF {
    match role {
        ComboTextRole::Text => palette.text,
        ComboTextRole::SelFg => palette.sel_fg,
        ComboTextRole::TextMuted => palette.text_muted,
    }
}

/// The order the two frames of the state image list of `settings::build_check_image_list`
/// stand in: frame 0 — снята, frame 1 — взведена.
///
/// This array is what couples the drawing to the participation bits of FR-31: a state image
/// index is **one-based** — index 1 names frame 0 — so the frame at position `i` here answers
/// the mask `(i + 1) << 12`, which is `settings::UNCHECKED_IMAGE` for the first frame and
/// `settings::CHECKED_IMAGE` for the second, exactly the values `settings::set_row_check`
/// writes and `settings::read_cycle_checks` reads. The system pair of `LVS_EX_CHECKBOXES`
/// sits in the same order, which is why replacing the image list moves not a single state
/// bit. A test holds the coupling.
///
/// Since task T-11-25 the two cells hold no picture — both are a hole edge to edge, and what
/// the order names is the *meaning* of the two indices rather than two drawings. It is still
/// this coupling the tick hangs from: `settings::on_notify` reads the same bits and hands
/// `settings::draw_cycle_row` the `checked` the picture is chosen by.
pub const CHECK_FRAME_ORDER: [bool; 2] = [false, true];

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
