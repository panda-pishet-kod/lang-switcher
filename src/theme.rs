//! The theme of everything visible: the two palettes of FR-92а, the system switch that
//! chooses between them, and the brushes the windows will paint with.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC: «Палитры FR-92а,
//! чтение `AppsUseLightTheme`, кисти и отрисовка элементов по `WM_DRAWITEM` /
//! `WM_CTLCOLOR*`». This task (T-11-1) lays the foundation — the setting, the palettes, the
//! reading of the system switch, the resolution of the two into one palette, and the brush
//! owner. The message handlers that *paint* with all of it arrive with tasks T-11-4 and
//! later, inside the modules that own the windows being painted.
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

use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Gdi::{CreateSolidBrush, DeleteObject, HBRUSH};
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
/// `COLORREF` because that is the currency of `WM_CTLCOLOR*`, `SetTextColor` and
/// `SetBkColor`, and converting at every use would be sixteen more places to reverse the
/// bytes wrongly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Background of a dialog.
    pub window_bg: COLORREF,
    /// Background of a title bar. DWM paints the real caption itself; this field is for
    /// the caption-like strip of the «О программе» window (task T-11-11).
    pub title_bg: COLORREF,
    /// Fill of a group box.
    pub panel_bg: COLORREF,
    /// Frame of a group box, and separators.
    pub panel_border: COLORREF,
    /// The main text.
    pub text: COLORREF,
    /// Explanatory captions beside the main text.
    pub text_muted: COLORREF,
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
    /// Highlight of a menu item under the cursor.
    pub hover_bg: COLORREF,
}

/// The dark palette — «Графит» of FR-92а.
pub static GRAPHITE: Palette = Palette {
    window_bg: rgb(32, 35, 41),
    title_bg: rgb(26, 29, 34),
    panel_bg: rgb(39, 43, 50),
    panel_border: rgb(54, 59, 67),
    text: rgb(228, 231, 234),
    text_muted: rgb(152, 160, 168),
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
};

/// The light palette — «Туман» of FR-92а.
pub static FOG: Palette = Palette {
    window_bg: rgb(237, 239, 242),
    title_bg: rgb(247, 248, 250),
    panel_bg: rgb(255, 255, 255),
    panel_border: rgb(225, 229, 234),
    text: rgb(35, 38, 43),
    text_muted: rgb(110, 118, 127),
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
