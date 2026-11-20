//! **The window in the corner** — task T-119-6 (решения 208.90–208.94, 208.119, 208.141): one window
//! by the notification area for the hint of the flag (T-119-5), «Обновлено» and, in the next stage,
//! the announcement of a letter.
//!
//! # The conditions of stage E120 (TZ-E120 §10)
//!
//! The look moves onto the shell of E120 in T-120-14, so here:
//! 1. the colours and sizes are **one table under the names of the passport** (`PASSPORT.md` §1–§4) —
//!    [`Palette`], [`SIZES`], [`TYPE`] — and the drawing names them, never a number of its own;
//! 2. the drawing is **one function of the state**, [`paint`], apart from when and where the window is
//!    shown;
//! 3. **no mirror** for Hebrew and Arabic — it comes with the shell;
//! 4. the look is the passport's: the main button on the right and wider than the second, the header
//!    not smaller than 12, the font of the system (Segoe UI Variable Text, on Windows 10 Segoe UI;
//!    monospace Cascadia Mono, else Consolas), buttons that do not fit — one under the other;
//! 5. the place is **measured by the notification area** (`TrayNotifyWnd` of the taskbar), not «the
//!    lower right corner»;
//! 6. the behaviour — when to show, not taking the focus, once — and its tests are the full ones.
//!
//! ⚠ The font of the mock-up (Inter) is not used: the passport of E120 says «файлов шрифтов в
//! программе нет» — the system's font, which the shell keeps.

use crate::layouts::LayoutId;
use windows::Win32::Foundation::{COLORREF, HWND, RECT};

// -----------------------------------------------------------------------------------------
// The table of the passport — §1 colours, §2 type, §3 sizes, §4 roundings
// -----------------------------------------------------------------------------------------

/// The colours of the passport (§1) the window in the corner uses, by the passport's names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// `bg` — the ground of the window.
    pub bg: (u8, u8, u8),
    /// `surface` — the second button, the box of the check.
    pub surface: (u8, u8, u8),
    /// `surface2` — the panel of the picture, a hovered second button, a hovered ✕.
    pub surface2: (u8, u8, u8),
    /// `line` — the frame of the window and of the picture.
    pub line: (u8, u8, u8),
    /// `edge` — the frame of everything that is pressed: the second button, the check box.
    pub edge: (u8, u8, u8),
    /// `ink` — the title, the text of the second button.
    pub ink: (u8, u8, u8),
    /// `ink2` — the text, the label of the check box, the ✕.
    pub ink2: (u8, u8, u8),
    /// `accent` — the header, the link, the tick.
    pub accent: (u8, u8, u8),
    /// `button` — the ground of the main button, and nothing else.
    pub button: (u8, u8, u8),
    /// `button_fg` — the text of the main button.
    pub button_fg: (u8, u8, u8),
}

/// The passport's light column.
pub const LIGHT: Palette = Palette {
    bg: (0xFA, 0xF9, 0xF6),
    surface: (0xFD, 0xFC, 0xFA),
    surface2: (0xF1, 0xEF, 0xE9),
    line: (0xE6, 0xE3, 0xDC),
    edge: (0x8C, 0x88, 0x80),
    ink: (0x1C, 0x1B, 0x18),
    ink2: (0x5B, 0x59, 0x55),
    accent: (0x1F, 0x7A, 0x4A),
    button: (0x1C, 0x1B, 0x18),
    button_fg: (0xFF, 0xFF, 0xFF),
};

/// The passport's dark column.
pub const DARK: Palette = Palette {
    bg: (0x23, 0x27, 0x2F),
    surface: (0x2B, 0x30, 0x3A),
    surface2: (0x32, 0x38, 0x45),
    line: (0x36, 0x3C, 0x47),
    edge: (0x7A, 0x82, 0x90),
    ink: (0xE6, 0xE9, 0xEE),
    ink2: (0xA2, 0xA8, 0xB3),
    accent: (0x6C, 0xC8, 0x94),
    button: (0xD6, 0xDA, 0xE0),
    button_fg: (0x23, 0x27, 0x2F),
};

/// The palette of a theme of Windows.
pub fn palette(light: bool) -> Palette {
    if light { LIGHT } else { DARK }
}

/// The sizes of the window in the corner at 100 %, in points (= pixels at 96 DPI) — passport §3
/// («Окно в углу»), §4 (roundings) and the mock-up «Окна в углу» v3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sizes {
    /// The width of the window.
    pub width: i32,
    /// The padding: top, right, bottom, left.
    pub padding: (i32, i32, i32, i32),
    /// Between the parts.
    pub gap: i32,
    /// The height of the header and the side of its ✕.
    pub header: i32,
    /// The icon of the header.
    pub icon: i32,
    /// The circle of the panda.
    pub panda: i32,
    /// The least height of a button.
    pub button: i32,
    /// Between the two buttons.
    pub button_gap: i32,
    /// The height of the picture.
    pub picture: i32,
    /// The side of the check box.
    pub check: i32,
    /// Between the check box and its label (0,6 B at B = 13).
    pub check_gap: i32,
    /// The rounding of the window — large (§4).
    pub radius_window: i32,
    /// The rounding of a button and of the picture — middle (§4).
    pub radius_button: i32,
    /// The rounding of the ✕ — small (§4).
    pub radius_close: i32,
    /// The rounding of the check box (§4).
    pub radius_check: i32,
}

/// [`Sizes`] of the passport.
pub const SIZES: Sizes = Sizes {
    width: 364,
    padding: (10, 12, 16, 16),
    gap: 12,
    header: 28,
    icon: 16,
    panda: 44,
    button: 40,
    button_gap: 8,
    picture: 150,
    check: 16,
    check_gap: 8,
    radius_window: 12,
    radius_button: 8,
    radius_close: 6,
    radius_check: 4,
};

/// The constant type sizes of the window in the corner (passport §2), in tenths of a point at 100 %,
/// with the weight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Type {
    /// The header «LANG SWITCHER · …» — monospace capitals, 600, `accent`.
    pub header: (i32, u32),
    /// The title — 600, `ink`.
    pub title: (i32, u32),
    /// The text — 400, `ink2`.
    pub text: (i32, u32),
    /// The link — 400, `accent`, underlined.
    pub link: (i32, u32),
    /// A button — 500; the main one 600.
    pub button: (i32, u32),
    /// The label of the check box — 400, `ink2`.
    pub check: (i32, u32),
}

/// [`Type`] of the passport.
pub const TYPE: Type = Type {
    header: (120, 600),
    title: (150, 600),
    text: (135, 400),
    link: (130, 400),
    button: (135, 500),
    check: (130, 400),
};

/// A length of the table at a DPI: points at 100 % to pixels, rounded.
pub fn px(points: i32, dpi: i32) -> i32 {
    let dpi = if dpi > 0 { dpi } else { 96 };
    (points * dpi + 48) / 96
}

/// A type size of the table at a DPI: tenths of a point to pixels, rounded.
pub fn type_px(tenths: i32, dpi: i32) -> i32 {
    let dpi = if dpi > 0 { dpi } else { 96 };
    (tenths * dpi + 480) / 960
}

// -----------------------------------------------------------------------------------------
// What the window says
// -----------------------------------------------------------------------------------------

/// What a button of the window does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Closes the window.
    Close,
    /// Opens an address of the program's own pages (through `settings::open_in_the_shell`) and
    /// closes the window.
    Open(&'static str),
}

/// What the window in the corner says — the parts in the order of the mock-up. The words are the
/// caller's, read from the string tables.
#[derive(Clone, Debug, PartialEq)]
pub struct Content {
    /// What the window is about — the end of the header «LANG SWITCHER · …», written in capitals.
    pub about: String,
    /// The title.
    pub title: String,
    /// The text, up to three lines.
    pub text: String,
    /// The link under the text and its address, if any.
    pub link: Option<(String, &'static str)>,
    /// The main button — on the right, filled, two shares of the width.
    pub main: (String, Action),
    /// The second button — on the left, framed, one share; none for a window of one button.
    pub second: Option<(String, Action)>,
    /// The picture under the buttons — the animation of the flag (T-119-6).
    pub picture: bool,
    /// The label of the check box at the bottom, if any.
    pub check: Option<String>,
}

/// How the window ended — what its caller is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct End {
    /// Which way: the main button, the second, the ✕ or `Esc`.
    pub by: EndBy,
    /// Whether the check box was ticked when it ended.
    pub checked: bool,
}

/// The way a window in the corner ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndBy {
    /// The main button.
    Main,
    /// The second button.
    Second,
    /// The ✕ or `Esc`.
    Close,
}

// -----------------------------------------------------------------------------------------
// The layout — pure, once the heights of the texts are measured
// -----------------------------------------------------------------------------------------

/// Where every part of the window stands, in client pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layout {
    /// The size of the window.
    pub width: i32,
    /// The height of the window.
    pub height: i32,
    /// The header line (icon and words).
    pub header: RECT,
    /// The icon of the header.
    pub icon: RECT,
    /// The ✕.
    pub close: RECT,
    /// The circle of the panda.
    pub panda: RECT,
    /// The title.
    pub title: RECT,
    /// The text.
    pub text: RECT,
    /// The link (empty when none).
    pub link: RECT,
    /// The second button (empty when none).
    pub second: RECT,
    /// The main button.
    pub main: RECT,
    /// The picture (empty when none).
    pub picture: RECT,
    /// The check box with its label (empty when none).
    pub check: RECT,
}

/// The heights the texts take at the width they are given — measured by the window, handed in so
/// that the layout itself is pure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Measured {
    /// The title.
    pub title: i32,
    /// The text.
    pub text: i32,
    /// The link, 0 when none.
    pub link: i32,
    /// The label of the check box, 0 when none.
    pub check: i32,
    /// The widths the two button labels need, without the button's padding.
    pub buttons: (i32, i32),
}

/// The width of the column of the title and the text at a DPI — the window less its padding, the
/// panda and the gap.
pub fn text_width(dpi: i32) -> i32 {
    let s = SIZES;
    px(s.width - s.padding.1 - s.padding.3 - s.panda - s.gap, dpi)
}

/// The layout of a content of these parts at a DPI. The buttons stand in a row — the second one
/// share on the left, the main two on the right — unless a label does not fit its share; then one
/// under the other, the main one first (passport §10 (4)). Pure.
pub fn layout(content: &Content, measured: Measured, dpi: i32) -> Layout {
    let s = SIZES;
    let at = |points: i32| px(points, dpi);
    let width = at(s.width);
    let (top, right, bottom, left) = (
        at(s.padding.0),
        at(s.padding.1),
        at(s.padding.2),
        at(s.padding.3),
    );
    let inner_right = width - right;
    let header = RECT {
        left,
        top,
        right: inner_right,
        bottom: top + at(s.header),
    };
    let icon = RECT {
        left,
        top: top + (at(s.header) - at(s.icon)) / 2,
        right: left + at(s.icon),
        bottom: top + (at(s.header) - at(s.icon)) / 2 + at(s.icon),
    };
    let close = RECT {
        left: inner_right - at(s.header),
        top,
        right: inner_right,
        bottom: top + at(s.header),
    };

    let mut y = header.bottom + at(s.gap);
    let panda = RECT {
        left,
        top: y,
        right: left + at(s.panda),
        bottom: y + at(s.panda),
    };
    let column = panda.right + at(s.gap);
    let title = RECT {
        left: column,
        top: y,
        right: inner_right,
        bottom: y + measured.title,
    };
    let text = RECT {
        left: column,
        top: title.bottom + at(4),
        right: inner_right,
        bottom: title.bottom + at(4) + measured.text,
    };
    let link = if content.link.is_some() {
        RECT {
            left: column,
            top: text.bottom + at(6),
            right: inner_right,
            bottom: text.bottom + at(6) + measured.link,
        }
    } else {
        RECT::default()
    };

    y = text.bottom.max(link.bottom).max(panda.bottom) + at(s.gap);

    let row = inner_right - left;
    let share = (row - at(s.button_gap)) / 3;
    let pad = at(14) * 2;
    let fits = content.second.is_none()
        || (measured.buttons.0 + pad <= share * 2 && measured.buttons.1 + pad <= share);
    let height_of_button = at(s.button);
    let (main, second) = match (&content.second, fits) {
        (None, _) => (
            RECT {
                left,
                top: y,
                right: inner_right,
                bottom: y + height_of_button,
            },
            RECT::default(),
        ),
        (Some(_), true) => (
            RECT {
                left: inner_right - share * 2,
                top: y,
                right: inner_right,
                bottom: y + height_of_button,
            },
            RECT {
                left,
                top: y,
                right: left + row - share * 2 - at(s.button_gap),
                bottom: y + height_of_button,
            },
        ),
        (Some(_), false) => (
            RECT {
                left,
                top: y,
                right: inner_right,
                bottom: y + height_of_button,
            },
            RECT {
                left,
                top: y + height_of_button + at(s.button_gap),
                right: inner_right,
                bottom: y + height_of_button * 2 + at(s.button_gap),
            },
        ),
    };

    y = main.bottom.max(second.bottom);

    let picture = if content.picture {
        let p = RECT {
            left,
            top: y + at(s.gap),
            right: inner_right,
            bottom: y + at(s.gap) + at(s.picture),
        };
        y = p.bottom;
        p
    } else {
        RECT::default()
    };

    let check = if content.check.is_some() {
        let tall = measured.check.max(at(s.check));
        let c = RECT {
            left,
            top: y + at(s.gap),
            right: inner_right,
            bottom: y + at(s.gap) + tall,
        };
        y = c.bottom;
        c
    } else {
        RECT::default()
    };

    Layout {
        width,
        height: y + bottom,
        header,
        icon,
        close,
        panda,
        title,
        text,
        link,
        second,
        main,
        picture,
        check,
    }
}

/// Which part a client point falls on — for the mouse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The ✕.
    Close,
    /// The main button.
    Main,
    /// The second button.
    Second,
    /// The link.
    Link,
    /// The check box or its label.
    Check,
    /// Anything else.
    Nothing,
}

/// The part a client point falls on. Pure.
pub fn hit(layout: &Layout, x: i32, y: i32) -> Part {
    let inside = |r: &RECT| x >= r.left && x < r.right && y >= r.top && y < r.bottom;

    if inside(&layout.close) {
        Part::Close
    } else if inside(&layout.main) {
        Part::Main
    } else if inside(&layout.second) {
        Part::Second
    } else if inside(&layout.link) {
        Part::Link
    } else if inside(&layout.check) {
        Part::Check
    } else {
        Part::Nothing
    }
}

/// Where the window stands: by the notification area `area` (the rectangle of `TrayNotifyWnd`),
/// inside the work area, `gap` from it — above it with a taskbar below, below it with one at the
/// top, beside it with one on a side; the right edges together. Pure — the same rule as the list of
/// the flag's own ([`crate::flags::own_place`]).
pub fn place(area: RECT, work: RECT, size: (i32, i32), gap: i32) -> (i32, i32) {
    crate::flags::own_place(area, work, size, gap)
}

// -----------------------------------------------------------------------------------------
// The picture — the animation of the flag, drawn by code with the person's own layouts
// -----------------------------------------------------------------------------------------

/// The length of one round of the animation, ms (the mock-up: 5,6 s).
pub const ROUND_MS: u32 = 5600;

/// What the picture shows at a moment of its round — the mock-up's timing: the flag of the first
/// layout; from 1,0 s the plate «Alt + Shift» and at 1,4 s the flag of the second; from 2,3 s the
/// cursor goes to the flag, from 2,8 s the list is open, from 3,5 s the first row is hovered, at
/// 4,2 s the list closes on the first layout again. Pure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Which of the two layouts the flag shows: 0 or 1.
    pub flag: usize,
    /// The plate of the keys is up.
    pub plate: bool,
    /// The cursor: none, or its share of the way to the flag in thousandths.
    pub cursor: Option<i32>,
    /// The list is open.
    pub list: bool,
    /// The first row of the list is hovered.
    pub hover: bool,
}

/// [`Frame`] at `ms` into the round. Pure.
pub fn frame(ms: u32) -> Frame {
    let t = ms % ROUND_MS;

    Frame {
        flag: usize::from((1400..4200).contains(&t)),
        plate: (1000..1900).contains(&t),
        cursor: (2300..4300).contains(&t).then(|| {
            let travel = i32::try_from((t - 2300).min(500)).unwrap_or(500);
            travel * 2
        }),
        list: (2800..4200).contains(&t),
        hover: (3500..4200).contains(&t),
    }
}

/// The two layouts the picture shows — the one in force first and the next of the session; one
/// layout alone shows itself twice.
pub fn picture_layouts(session: &[LayoutId], current: LayoutId) -> (LayoutId, LayoutId) {
    let first = if session.contains(&current) {
        current
    } else {
        session.first().copied().unwrap_or(current)
    };
    let second = session
        .iter()
        .copied()
        .find(|layout| *layout != first)
        .unwrap_or(first);

    (first, second)
}

// -----------------------------------------------------------------------------------------
// The window
// -----------------------------------------------------------------------------------------

/// The class of the window.
const CLASS: windows::core::PCWSTR = windows::core::w!("LangSwitcher.Corner");

/// The window's own timer of the animation and its step, ms.
const ANIMATION_TIMER: usize = 1;
const ANIMATION_MS: u32 = 40;

/// The fonts of one showing, by role — deleted with it.
struct Fonts {
    header: windows::Win32::Graphics::Gdi::HFONT,
    title: windows::Win32::Graphics::Gdi::HFONT,
    text: windows::Win32::Graphics::Gdi::HFONT,
    link: windows::Win32::Graphics::Gdi::HFONT,
    button: windows::Win32::Graphics::Gdi::HFONT,
    main: windows::Win32::Graphics::Gdi::HFONT,
    check: windows::Win32::Graphics::Gdi::HFONT,
}

impl Drop for Fonts {
    fn drop(&mut self) {
        use windows::Win32::Graphics::Gdi::DeleteObject;

        for font in [
            self.header,
            self.title,
            self.text,
            self.link,
            self.button,
            self.main,
            self.check,
        ] {
            // SAFETY: a font of this showing, no longer selected into any DC.
            let _ = unsafe { DeleteObject(font.into()) };
        }
    }
}

/// The window on the screen — one at a time, on the UI thread.
struct Shown {
    hwnd: HWND,
    content: Content,
    layout: Layout,
    fonts: Fonts,
    hover: Part,
    checked: bool,
    tracking: bool,
    started: std::time::Instant,
    /// The flags of the person's two layouts and their codes, for the picture.
    flags: [(
        Option<windows::Win32::UI::WindowsAndMessaging::HICON>,
        String,
    ); 2],
    icon: Option<windows::Win32::UI::WindowsAndMessaging::HICON>,
    on_end: fn(End),
    ended: bool,
}

impl Drop for Shown {
    fn drop(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

        for icon in [self.flags[0].0, self.flags[1].0, self.icon]
            .into_iter()
            .flatten()
        {
            // SAFETY: icons made for this showing and owned by it.
            let _ = unsafe { DestroyIcon(icon) };
        }
    }
}

thread_local! {
    static SHOWN: std::cell::RefCell<Option<Shown>> = const { std::cell::RefCell::new(None) };
}

/// Whether a window in the corner is on the screen.
pub fn is_shown() -> bool {
    SHOWN.with(|shown| shown.borrow().is_some())
}

/// The face of the system for a role — the first of `faces` Windows has, by the name the DC answers
/// after the font is selected; the last one otherwise.
fn system_face(faces: &[&str]) -> String {
    use windows::Win32::Graphics::Gdi::{
        CreateFontW, DeleteObject, GetDC, GetTextFaceW, ReleaseDC, SelectObject,
    };

    // SAFETY: the screen DC, released below on the same thread.
    let dc = unsafe { GetDC(None) };

    for face in faces {
        let wide: Vec<u16> = face.encode_utf16().chain(Some(0)).collect();

        // SAFETY: a NUL-terminated name of this frame; the font is deleted below.
        let font = unsafe {
            CreateFontW(
                -16,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                windows::Win32::Graphics::Gdi::DEFAULT_CHARSET,
                windows::Win32::Graphics::Gdi::OUT_DEFAULT_PRECIS,
                windows::Win32::Graphics::Gdi::CLIP_DEFAULT_PRECIS,
                windows::Win32::Graphics::Gdi::CLEARTYPE_QUALITY,
                0,
                windows::core::PCWSTR(wide.as_ptr()),
            )
        };
        let mut answer = [0u16; 64];

        // SAFETY: the live screen DC and font; the previous font is selected back.
        let length = unsafe {
            let previous = SelectObject(dc, font.into());
            let length = GetTextFaceW(dc, Some(&mut answer));
            SelectObject(dc, previous);
            let _ = DeleteObject(font.into());
            length
        };
        let length = usize::try_from(length).unwrap_or(0).saturating_sub(1);
        let got = String::from_utf16_lossy(&answer[..length.min(answer.len())]);

        if got.eq_ignore_ascii_case(face) {
            // SAFETY: the DC taken above.
            unsafe { ReleaseDC(None, dc) };
            return (*face).to_owned();
        }
    }

    // SAFETY: the DC taken above.
    unsafe { ReleaseDC(None, dc) };
    faces
        .last()
        .map_or_else(String::new, |face| (*face).to_owned())
}

/// The fonts of the passport at a DPI.
fn make_fonts(dpi: i32) -> Fonts {
    use windows::Win32::Graphics::Gdi::{
        CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
    };

    let main_face = system_face(&["Segoe UI Variable Text", "Segoe UI"]);
    let mono_face = system_face(&["Cascadia Mono", "Consolas"]);
    let make = |face: &str, (tenths, weight): (i32, u32), underline: bool| {
        let wide: Vec<u16> = face.encode_utf16().chain(Some(0)).collect();

        // SAFETY: a NUL-terminated name of this frame; the font is the caller's to delete.
        unsafe {
            CreateFontW(
                -type_px(tenths, dpi),
                0,
                0,
                0,
                i32::try_from(weight).unwrap_or(400),
                0,
                u32::from(underline),
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                0,
                windows::core::PCWSTR(wide.as_ptr()),
            )
        }
    };

    Fonts {
        header: make(&mono_face, TYPE.header, false),
        title: make(&main_face, TYPE.title, false),
        text: make(&main_face, TYPE.text, false),
        link: make(&main_face, TYPE.link, true),
        button: make(&main_face, TYPE.button, false),
        main: make(&main_face, (TYPE.button.0, 600), false),
        check: make(&main_face, TYPE.check, false),
    }
}

/// The height a text takes at a width in a font — `DrawTextW` with `DT_CALCRECT`.
fn measure(
    dc: windows::Win32::Graphics::Gdi::HDC,
    font: windows::Win32::Graphics::Gdi::HFONT,
    text: &str,
    width: i32,
    one_line: bool,
) -> (i32, i32) {
    use windows::Win32::Graphics::Gdi::{
        DT_CALCRECT, DT_NOPREFIX, DT_SINGLELINE, DT_WORDBREAK, DrawTextW, SelectObject,
    };

    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: 0,
    };
    let mode = if one_line {
        DT_SINGLELINE
    } else {
        DT_WORDBREAK
    };

    // SAFETY: a live DC and font; the previous font is selected back.
    unsafe {
        let previous = SelectObject(dc, font.into());
        DrawTextW(dc, &mut wide, &mut rect, DT_CALCRECT | DT_NOPREFIX | mode);
        SelectObject(dc, previous);
    }

    (rect.right - rect.left, rect.bottom - rect.top)
}

/// The rectangle of the notification area (`TrayNotifyWnd` of the taskbar), or of the taskbar.
fn notification_area() -> Option<RECT> {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect};
    use windows::core::w;

    // SAFETY: constant class names; a null answer is checked.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    // SAFETY: as above.
    let area =
        unsafe { FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None) }.unwrap_or(taskbar);
    let mut rect = RECT::default();

    // SAFETY: a window handle by value and a local rectangle.
    unsafe { GetWindowRect(area, &mut rect) }.ok()?;
    Some(rect)
}

/// Shows the window by the notification area **without taking the focus**, fading in over a quarter
/// of a second; `sound` — the notification sound of Windows (the caller asks FR-100's setting).
/// `on_end` is told how it ended. `false` when another window in the corner is up or nothing could
/// be made (NFR-13).
pub fn show(content: Content, sound: bool, on_end: fn(End)) -> bool {
    use windows::Win32::Graphics::Dwm::{DWMWINDOWATTRIBUTE, DwmSetWindowAttribute};
    use windows::Win32::Graphics::Gdi::{
        GetDC, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect, ReleaseDC,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        AW_BLEND, AnimateWindow, CS_DROPSHADOW, CreateWindowExW, HWND_TOPMOST, IDC_ARROW,
        IMAGE_ICON, LR_DEFAULTCOLOR, LoadCursorW, LoadImageW, RegisterClassExW, SWP_NOACTIVATE,
        SetTimer, SetWindowPos, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };

    if is_shown() {
        return false;
    }

    let area = notification_area().unwrap_or_default();

    // SAFETY: `None` names the running executable; the handle is borrowed, not freed.
    let Ok(module) = (unsafe { GetModuleHandleW(None) }) else {
        return false;
    };

    // SAFETY: a stock cursor of the system; a refusal leaves the class without one.
    let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default();
    let class = WNDCLASSEXW {
        cbSize: u32::try_from(size_of::<WNDCLASSEXW>()).unwrap_or(0),
        style: CS_DROPSHADOW,
        lpfnWndProc: Some(corner_proc),
        hInstance: module.into(),
        hCursor: cursor,
        lpszClassName: CLASS,
        ..Default::default()
    };

    // SAFETY: a live structure of this frame; a second registration answers 0 and is no error.
    let _ = unsafe { RegisterClassExW(&class) };

    // SAFETY: the class registered above, a module of this process, no parent.
    let Ok(hwnd) = (unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            CLASS,
            windows::core::PCWSTR::null(),
            WS_POPUP,
            area.left,
            area.top,
            1,
            1,
            None,
            None,
            Some(module.into()),
            None,
        )
    }) else {
        return false;
    };

    // The large rounding of the passport by the system's own attribute (§4), Windows 11; Windows 10
    // has none and keeps the corners square.
    let round = 2u32;

    // SAFETY: the live window just made and a local of the size passed.
    let _ = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(33),
            (&raw const round).cast(),
            u32::try_from(size_of::<u32>()).unwrap_or(4),
        )
    };

    // SAFETY: the live window; the DC is released below on the same thread.
    let dc = unsafe { GetDC(Some(hwnd)) };
    let dpi = crate::theme::dc_dpi(dc);
    let fonts = make_fonts(dpi);
    let column = text_width(dpi);
    let row = px(SIZES.width - SIZES.padding.1 - SIZES.padding.3, dpi);
    let measured = Measured {
        title: measure(dc, fonts.title, &content.title, column, false).1,
        text: measure(dc, fonts.text, &content.text, column, false).1,
        link: content.link.as_ref().map_or(0, |(words, _)| {
            measure(dc, fonts.link, words, column, true).1
        }),
        check: content.check.as_ref().map_or(0, |words| {
            measure(
                dc,
                fonts.check,
                words,
                row - px(SIZES.check + SIZES.check_gap, dpi),
                false,
            )
            .1
        }),
        buttons: (
            measure(dc, fonts.main, &content.main.0, row, true).0,
            content.second.as_ref().map_or(0, |(words, _)| {
                measure(dc, fonts.button, words, row, true).0
            }),
        ),
    };

    // SAFETY: the DC taken above from this window.
    unsafe { ReleaseDC(Some(hwnd), dc) };

    let layout = layout(&content, measured, dpi);

    // SAFETY: a rectangle by reference; the nearest monitor is always answered.
    let monitor = unsafe { MonitorFromRect(&area, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: u32::try_from(size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };

    // SAFETY: a monitor handle by value and a live local whose `cbSize` describes it.
    let _ = unsafe { GetMonitorInfoW(monitor, &mut info) };

    let (x, y) = place(
        area,
        info.rcWork,
        (layout.width, layout.height),
        px(SIZES.gap, dpi),
    );

    let session = crate::layouts::enumerate().unwrap_or_default();
    let (one, two) = picture_layouts(&session, crate::switch::read().layout);
    let look = crate::flags::Look {
        size: u32::try_from(px(20, dpi)).unwrap_or(20),
        paused: false,
        unread: false,
        dark_bar: !crate::theme::system_is_light(),
    };
    let flag_of = |layout: LayoutId| {
        (
            crate::flags::icon(&crate::flags::face_of(layout), look),
            crate::flags::language_code(layout),
        )
    };
    let icon_side = px(SIZES.icon, dpi);

    // SAFETY: an icon resource of this module (FR-90's own); a refusal leaves the header without it.
    let icon = unsafe {
        LoadImageW(
            Some(module.into()),
            windows::core::PCWSTR(101 as *const u16),
            IMAGE_ICON,
            icon_side,
            icon_side,
            LR_DEFAULTCOLOR,
        )
    }
    .ok()
    .map(|handle| windows::Win32::UI::WindowsAndMessaging::HICON(handle.0));

    let picture = content.picture;

    SHOWN.with(|shown| {
        *shown.borrow_mut() = Some(Shown {
            hwnd,
            content,
            layout,
            fonts,
            hover: Part::Nothing,
            checked: false,
            tracking: false,
            started: std::time::Instant::now(),
            flags: [flag_of(one), flag_of(two)],
            icon,
            on_end,
            ended: false,
        });
    });

    // SAFETY: the live window of this thread; placed and sized, not activated, not yet shown.
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            layout.width,
            layout.height,
            SWP_NOACTIVATE,
        )
    };

    if sound {
        play_notification_sound();
    }

    // SAFETY: the live hidden window; `AW_BLEND` without `AW_ACTIVATE` shows it without the focus.
    let _ = unsafe { AnimateWindow(hwnd, 250, AW_BLEND) };

    if picture {
        // SAFETY: the live window and its own timer identifier.
        let _ = unsafe { SetTimer(Some(hwnd), ANIMATION_TIMER, ANIMATION_MS, None) };
    }

    true
}

/// The notification sound of Windows — the sound the person set for notifications.
fn play_notification_sound() {
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};

    // SAFETY: a constant NUL-terminated alias; asynchronous, nothing of this frame is kept.
    let _ = unsafe {
        PlaySoundW(
            windows::core::w!("Notification.Default"),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        )
    };
}

/// Ends the window: it goes, its state goes, and its caller is told — outside the borrow.
fn end(by: EndBy) {
    use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;

    let told = SHOWN.with(|shown| {
        let mut shown = shown.borrow_mut();
        let state = shown.as_mut()?;

        if state.ended {
            return None;
        }

        state.ended = true;
        Some((state.hwnd, state.on_end, state.checked))
    });

    let Some((hwnd, on_end, checked)) = told else {
        return;
    };

    // SAFETY: the live window of this thread, destroyed once (`ended`).
    let _ = unsafe { DestroyWindow(hwnd) };

    SHOWN.with(|shown| *shown.borrow_mut() = None);
    on_end(End { by, checked });
}

/// Does what a button asks.
fn act(action: Action, by: EndBy) {
    if let Action::Open(address) = action {
        let _ = crate::settings::open_in_the_shell(address);
    }

    end(by);
}

/// The window procedure.
unsafe extern "system" fn corner_proc(
    hwnd: HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT};
    use windows::Win32::UI::Controls::WM_MOUSELEAVE;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, MA_NOACTIVATE, WM_KEYDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
        WM_PAINT, WM_TIMER,
    };

    let low = |value: isize| i32::from((value & 0xFFFF) as u16 as i16);
    let high = |value: isize| i32::from(((value >> 16) & 0xFFFF) as u16 as i16);
    let redraw = || {
        // SAFETY: the live window this procedure runs for.
        let _ = unsafe { InvalidateRect(Some(hwnd), None, false) };
    };

    match message {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();

            // SAFETY: the live window this `WM_PAINT` is for; ended below.
            let dc = unsafe { BeginPaint(hwnd, &mut ps) };

            SHOWN.with(|shown| {
                if let Some(state) = shown.borrow().as_ref() {
                    paint(dc, state, palette(crate::theme::system_is_light()));
                }
            });

            // SAFETY: the paint begun above.
            let _ = unsafe { EndPaint(hwnd, &ps) };
            return LRESULT(0);
        }
        // A click does not take the focus from the window the person types in — FR-101's «фокус не
        // отбирают» (T-119-6); the window answers the mouse all the same.
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_TIMER if wparam.0 == ANIMATION_TIMER => {
            redraw();
            return LRESULT(0);
        }
        WM_KEYDOWN if wparam.0 == 0x1B => {
            end(EndBy::Close);
            return LRESULT(0);
        }
        WM_MOUSEMOVE => {
            let first = SHOWN.with(|shown| {
                let mut shown = shown.borrow_mut();
                let state = shown.as_mut()?;
                state.hover = hit(&state.layout, low(lparam.0), high(lparam.0));
                let first = !state.tracking;
                state.tracking = true;
                Some(first)
            });

            if first == Some(true) {
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: u32::try_from(size_of::<TRACKMOUSEEVENT>()).unwrap_or(0),
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };

                // SAFETY: a live local whose `cbSize` describes it, for the live window.
                let _ = unsafe { TrackMouseEvent(&mut tracking) };
            }

            redraw();
            return LRESULT(0);
        }
        WM_MOUSELEAVE => {
            SHOWN.with(|shown| {
                if let Some(state) = shown.borrow_mut().as_mut() {
                    state.hover = Part::Nothing;
                    state.tracking = false;
                }
            });
            redraw();
            return LRESULT(0);
        }
        WM_LBUTTONUP => {
            let what = SHOWN.with(|shown| {
                let shown = shown.borrow();
                let state = shown.as_ref()?;
                Some((
                    hit(&state.layout, low(lparam.0), high(lparam.0)),
                    state.content.main.1,
                    state.content.second.as_ref().map(|(_, action)| *action),
                    state.content.link.as_ref().map(|(_, address)| *address),
                ))
            });

            match what {
                Some((Part::Close, ..)) => end(EndBy::Close),
                Some((Part::Main, main, ..)) => act(main, EndBy::Main),
                Some((Part::Second, _, Some(second), _)) => act(second, EndBy::Second),
                Some((Part::Link, _, _, Some(address))) => {
                    let _ = crate::settings::open_in_the_shell(address);
                }
                Some((Part::Check, ..)) => {
                    SHOWN.with(|shown| {
                        if let Some(state) = shown.borrow_mut().as_mut() {
                            state.checked = !state.checked;
                        }
                    });
                    redraw();
                }
                _ => {}
            }

            return LRESULT(0);
        }
        _ => {}
    }

    // SAFETY: the arguments this procedure was called with, handed on unchanged.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn colour((r, g, b): (u8, u8, u8)) -> COLORREF {
    COLORREF(u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16))
}

/// **The drawing — one function of the state** (TZ-E120 §10 (2)): the ground and the frame, the
/// header with the icon and ✕, the panda, the title, the text, the link, the buttons, the picture,
/// the check box — every colour and size from the table of the passport.
fn paint(dc: windows::Win32::Graphics::Gdi::HDC, state: &Shown, palette: Palette) {
    use windows::Win32::Graphics::Gdi::{
        CreatePen, CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX,
        DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, DeleteObject, DrawTextW, FillRect, FrameRect,
        HFONT, LineTo, MoveToEx, PS_SOLID, SelectObject, SetBkMode, SetTextCharacterExtra,
        SetTextColor, TRANSPARENT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DrawIconEx};

    let dpi = crate::theme::dc_dpi(dc);
    let at = |points: i32| px(points, dpi);
    let l = &state.layout;
    let c = &state.content;
    let whole = RECT {
        left: 0,
        top: 0,
        right: l.width,
        bottom: l.height,
    };

    // SAFETY: colours by value; the brushes are deleted at the end of this paint.
    let (bg, line, surface, surface2, button, accent) = unsafe {
        (
            CreateSolidBrush(colour(palette.bg)),
            CreateSolidBrush(colour(palette.line)),
            CreateSolidBrush(colour(palette.surface)),
            CreateSolidBrush(colour(palette.surface2)),
            CreateSolidBrush(colour(palette.button)),
            CreateSolidBrush(colour(palette.accent)),
        )
    };

    // SAFETY: the live paint DC and brushes of this frame.
    unsafe {
        FillRect(dc, &whole, bg);
        FrameRect(dc, &whole, line);
        SetBkMode(dc, TRANSPARENT);
    }

    let text = |what: &str, mut area: RECT, ink: (u8, u8, u8), font: HFONT, align| {
        let mut wide: Vec<u16> = what.encode_utf16().collect();

        // SAFETY: the live paint DC, a live font of this showing, a local buffer and rectangle;
        // the previous font is selected back.
        unsafe {
            SetTextColor(dc, colour(ink));
            let previous = SelectObject(dc, font.into());
            DrawTextW(dc, &mut wide, &mut area, align | DT_NOPREFIX);
            SelectObject(dc, previous);
        }
    };

    // The header: the icon, «LANG SWITCHER · …» in monospace capitals with the passport's spacing
    // (0,08 em), the ✕.
    if let Some(icon) = state.icon {
        // SAFETY: the live paint DC and an icon of this showing.
        let _ = unsafe {
            DrawIconEx(
                dc,
                l.icon.left,
                l.icon.top,
                icon,
                l.icon.right - l.icon.left,
                l.icon.bottom - l.icon.top,
                0,
                None,
                DI_NORMAL,
            )
        };
    }

    let words = format!(
        "{} · {}",
        crate::APP_NAME.to_uppercase(),
        c.about.to_uppercase()
    );

    // SAFETY: the live paint DC; the spacing is reset below.
    unsafe { SetTextCharacterExtra(dc, (type_px(TYPE.header.0, dpi) * 8 + 50) / 100) };
    text(
        &words,
        RECT {
            left: l.icon.right + at(8),
            right: l.close.left - at(4),
            ..l.header
        },
        palette.accent,
        state.fonts.header,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
    );
    // SAFETY: as above.
    unsafe { SetTextCharacterExtra(dc, 0) };

    if state.hover == Part::Close {
        crate::theme::paint_rounded(
            dc,
            &l.close,
            at(SIZES.radius_close),
            colour(palette.surface2),
            surface2,
            dpi,
        );
    }

    {
        let mid_x = (l.close.left + l.close.right) / 2;
        let mid_y = (l.close.top + l.close.bottom) / 2;
        let arm = at(5);

        // SAFETY: a pen of this frame, selected and deleted below.
        unsafe {
            let pen = CreatePen(PS_SOLID, at(1).max(1), colour(palette.ink2));
            let previous = SelectObject(dc, pen.into());
            let _ = MoveToEx(dc, mid_x - arm, mid_y - arm, None);
            let _ = LineTo(dc, mid_x + arm + 1, mid_y + arm + 1);
            let _ = MoveToEx(dc, mid_x + arm, mid_y - arm, None);
            let _ = LineTo(dc, mid_x - arm - 1, mid_y + arm + 1);
            SelectObject(dc, previous);
            let _ = DeleteObject(pen.into());
        }
    }

    paint_panda(dc, l.panda);

    text(
        &c.title,
        l.title,
        palette.ink,
        state.fonts.title,
        DT_LEFT | DT_WORDBREAK,
    );
    text(
        &c.text,
        l.text,
        palette.ink2,
        state.fonts.text,
        DT_LEFT | DT_WORDBREAK,
    );

    if let Some((words, _)) = &c.link {
        text(
            words,
            l.link,
            palette.accent,
            state.fonts.link,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }

    // The buttons: the main one filled with `button`, the second framed with `edge` on `surface`
    // (`surface2` under the mouse).
    crate::theme::paint_rounded(
        dc,
        &l.main,
        at(SIZES.radius_button),
        colour(palette.button),
        button,
        dpi,
    );
    text(
        &c.main.0,
        l.main,
        palette.button_fg,
        state.fonts.main,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
    );

    if let Some((words, _)) = &c.second {
        let ground = if state.hover == Part::Second {
            (surface2, palette.surface2)
        } else {
            (surface, palette.surface)
        };
        let _ = ground.1;
        crate::theme::paint_rounded(
            dc,
            &l.second,
            at(SIZES.radius_button),
            colour(palette.edge),
            ground.0,
            dpi,
        );
        text(
            words,
            l.second,
            palette.ink,
            state.fonts.button,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }

    if c.picture {
        crate::theme::paint_rounded(
            dc,
            &l.picture,
            at(SIZES.radius_button),
            colour(palette.line),
            surface2,
            dpi,
        );
        let ms =
            u32::try_from(state.started.elapsed().as_millis() % u128::from(ROUND_MS)).unwrap_or(0);
        paint_picture(dc, l.picture, frame(ms), state, palette, dpi);
    }

    if let Some(words) = &c.check {
        let side = at(SIZES.check);
        let tall = l.check.bottom - l.check.top;
        let square = RECT {
            left: l.check.left,
            top: l.check.top + (tall - side) / 2,
            right: l.check.left + side,
            bottom: l.check.top + (tall - side) / 2 + side,
        };
        let (fill, outline) = if state.checked {
            (accent, palette.accent)
        } else {
            (surface, palette.edge)
        };
        crate::theme::paint_rounded(
            dc,
            &square,
            at(SIZES.radius_check),
            colour(outline),
            fill,
            dpi,
        );

        if state.checked {
            // SAFETY: a pen of this frame, selected and deleted below.
            unsafe {
                let pen = CreatePen(PS_SOLID, at(2).max(1), colour(palette.button_fg));
                let previous = SelectObject(dc, pen.into());
                let _ = MoveToEx(dc, square.left + side * 3 / 16, square.top + side / 2, None);
                let _ = LineTo(dc, square.left + side * 7 / 16, square.top + side * 3 / 4);
                let _ = LineTo(dc, square.left + side * 13 / 16, square.top + side / 4);
                SelectObject(dc, previous);
                let _ = DeleteObject(pen.into());
            }
        }

        text(
            words,
            RECT {
                left: square.right + at(SIZES.check_gap),
                ..l.check
            },
            palette.ink2,
            state.fonts.check,
            DT_LEFT | DT_VCENTER | DT_WORDBREAK,
        );
    }

    // SAFETY: objects of this frame, no longer selected into any DC.
    unsafe {
        for brush in [bg, line, surface, surface2, button, accent] {
            let _ = DeleteObject(brush.into());
        }
    }
}

/// A colour of a drawing — `(red, green, blue)`.
type Rgb = (u8, u8, u8);

/// An ellipse of a drawing — `(left, top, right, bottom)`.
type Oval = (i32, i32, i32, i32);

/// The panda in its circle (the mock-up: a circle of `#6CC894`, the head of the panda) — the
/// drawing's own colours, the same in both themes (passport §1: «Рисунки панды несут свои цвета»).
fn paint_panda(dc: windows::Win32::Graphics::Gdi::HDC, area: RECT) {
    use windows::Win32::Graphics::Gdi::{
        CreatePen, CreateSolidBrush, DeleteObject, Ellipse, PS_NULL, SelectObject,
    };

    let w = area.right - area.left;
    let at = |x: i32, y: i32, rx: i32, ry: i32| {
        (
            area.left + x * w / 44 - rx * w / 44,
            area.top + y * w / 44 - ry * w / 44,
            area.left + x * w / 44 + rx * w / 44,
            area.top + y * w / 44 + ry * w / 44,
        )
    };
    let shapes: [(Rgb, Oval); 9] = [
        ((0x6C, 0xC8, 0x94), at(22, 22, 22, 22)),
        ((0x1C, 0x1B, 0x18), at(13, 14, 5, 5)),
        ((0x1C, 0x1B, 0x18), at(31, 14, 5, 5)),
        ((0xFA, 0xF9, 0xF6), at(22, 24, 13, 12)),
        ((0x1C, 0x1B, 0x18), at(17, 23, 4, 5)),
        ((0x1C, 0x1B, 0x18), at(27, 23, 4, 5)),
        ((0xFA, 0xF9, 0xF6), at(17, 22, 1, 1)),
        ((0xFA, 0xF9, 0xF6), at(27, 22, 1, 1)),
        ((0x1C, 0x1B, 0x18), at(22, 29, 2, 1)),
    ];

    // SAFETY: a null pen and brushes of this frame, selected and deleted below.
    unsafe {
        let pen = CreatePen(PS_NULL, 0, colour((0, 0, 0)));
        let previous_pen = SelectObject(dc, pen.into());

        for (rgb, (left, top, right, bottom)) in shapes {
            let brush = CreateSolidBrush(colour(rgb));
            let previous = SelectObject(dc, brush.into());
            let _ = Ellipse(dc, left, top, right, bottom);
            SelectObject(dc, previous);
            let _ = DeleteObject(brush.into());
        }

        SelectObject(dc, previous_pen);
        let _ = DeleteObject(pen.into());
    }
}

/// The picture of one frame: a strip of the taskbar with the person's flag and its code; the plate
/// of the keys «Alt + Shift»; the cursor going to the flag; the list of the two layouts above it.
fn paint_picture(
    dc: windows::Win32::Graphics::Gdi::HDC,
    area: RECT,
    now: Frame,
    state: &Shown,
    palette: Palette,
    dpi: i32,
) {
    use windows::Win32::Graphics::Gdi::{
        CreateSolidBrush, DT_CENTER, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteObject,
        DrawTextW, FillRect, Polygon, SelectObject, SetTextColor,
    };
    use windows::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DrawIconEx};

    let at = |points: i32| px(points, dpi);
    let bar = RECT {
        left: area.left + at(1),
        top: area.bottom - at(36),
        right: area.right - at(1),
        bottom: area.bottom - at(1),
    };
    let flag_side = at(20);
    let flag = RECT {
        left: bar.right - at(64),
        top: bar.top + (bar.bottom - bar.top - flag_side) / 2,
        right: bar.right - at(64) + flag_side,
        bottom: bar.top + (bar.bottom - bar.top - flag_side) / 2 + flag_side,
    };
    let text = |what: &str, mut place: RECT, ink: (u8, u8, u8), centred: bool| {
        let mut wide: Vec<u16> = what.encode_utf16().collect();
        let align = if centred { DT_CENTER } else { DT_LEFT };

        // SAFETY: the live paint DC and a font of this showing; the previous font is selected back.
        unsafe {
            SetTextColor(dc, colour(ink));
            let previous = SelectObject(dc, state.fonts.check.into());
            DrawTextW(
                dc,
                &mut wide,
                &mut place,
                align | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            SelectObject(dc, previous);
        }
    };

    // SAFETY: colours by value; deleted below.
    let (ground, line) = unsafe {
        (
            CreateSolidBrush(colour(palette.surface)),
            CreateSolidBrush(colour(palette.line)),
        )
    };

    // SAFETY: the live paint DC and brushes of this frame.
    unsafe {
        FillRect(dc, &bar, ground);
        FillRect(
            dc,
            &RECT {
                bottom: bar.top + at(1),
                ..bar
            },
            line,
        );
    }

    let (icon, code) = &state.flags[now.flag];

    if let Some(icon) = icon {
        // SAFETY: the live paint DC and an icon of this showing.
        let _ = unsafe {
            DrawIconEx(
                dc, flag.left, flag.top, *icon, flag_side, flag_side, 0, None, DI_NORMAL,
            )
        };
    }

    text(
        code,
        RECT {
            left: flag.right + at(8),
            right: bar.right - at(4),
            ..bar
        },
        palette.ink,
        false,
    );

    if now.plate {
        let plate = RECT {
            left: area.left + at(16),
            top: area.top + at(16),
            right: area.left + at(136),
            bottom: area.top + at(46),
        };
        crate::theme::paint_rounded(
            dc,
            &plate,
            at(SIZES.radius_close),
            colour(palette.edge),
            ground,
            dpi,
        );
        text("Alt + Shift", plate, palette.ink, true);
    }

    if now.list {
        let row = at(30);
        let list = RECT {
            left: flag.left - at(120),
            top: bar.top - at(8) - row * 2 - at(8),
            right: flag.right + at(40),
            bottom: bar.top - at(8),
        };
        crate::theme::paint_rounded(
            dc,
            &list,
            at(SIZES.radius_button),
            colour(palette.line),
            ground,
            dpi,
        );

        for (index, (icon, code)) in state.flags.iter().enumerate() {
            let place = RECT {
                left: list.left + at(4),
                top: list.top + at(4) + row * i32::try_from(index).unwrap_or(0),
                right: list.right - at(4),
                bottom: list.top + at(4) + row * (i32::try_from(index).unwrap_or(0) + 1),
            };

            if now.hover && index == 0 {
                // SAFETY: a brush of this frame, deleted right after.
                unsafe {
                    let hover = CreateSolidBrush(colour(palette.surface2));
                    FillRect(dc, &place, hover);
                    let _ = DeleteObject(hover.into());
                }
            }

            if let Some(icon) = icon {
                // SAFETY: the live paint DC and an icon of this showing.
                let _ = unsafe {
                    DrawIconEx(
                        dc,
                        place.left + at(6),
                        place.top + (row - flag_side) / 2,
                        *icon,
                        flag_side,
                        flag_side,
                        0,
                        None,
                        DI_NORMAL,
                    )
                };
            }

            text(
                code,
                RECT {
                    left: place.left + at(6) + flag_side + at(8),
                    ..place
                },
                palette.ink,
                false,
            );
        }
    }

    if let Some(share) = now.cursor {
        let from = (area.left + (area.right - area.left) / 3, area.top + at(40));
        let to = ((flag.left + flag.right) / 2, (flag.top + flag.bottom) / 2);
        let x = from.0 + (to.0 - from.0) * share / 1000;
        let y = from.1 + (to.1 - from.1) * share / 1000;
        let points = [
            windows::Win32::Foundation::POINT { x, y },
            windows::Win32::Foundation::POINT { x, y: y + at(14) },
            windows::Win32::Foundation::POINT {
                x: x + at(4),
                y: y + at(10),
            },
            windows::Win32::Foundation::POINT {
                x: x + at(10),
                y: y + at(10),
            },
        ];

        // SAFETY: a brush of this frame, selected and deleted below; the points are a local array.
        unsafe {
            let ink = CreateSolidBrush(colour(palette.ink));
            let previous = SelectObject(dc, ink.into());
            let _ = Polygon(dc, &points);
            SelectObject(dc, previous);
            let _ = DeleteObject(ink.into());
        }
    }

    // SAFETY: objects of this frame, no longer selected into any DC.
    unsafe {
        let _ = DeleteObject(ground.into());
        let _ = DeleteObject(line.into());
    }
}

// -----------------------------------------------------------------------------------------
// «Обновлено» — the block of the last important release, and when its window comes
// -----------------------------------------------------------------------------------------

/// The block «the last important release» — **exactly one in the build** (T-119-6): its version and
/// whether it has the picture. Its words are strings under permanent names (`IDS_RELEASE_*`, no
/// version in a name); «Как пользоваться» is the page of the program in the person's language. An
/// important release replaces the block; a minor one leaves it (the order of a release).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Release {
    /// The version of the release.
    pub version: &'static str,
    /// Whether its window has the animation of the flag.
    pub picture: bool,
}

/// The block of this build.
pub const LAST_IMPORTANT: Release = Release {
    version: "1.8.0",
    picture: true,
};

/// Whether the start after an update owes the window «Обновлено» — T-119-6: the version updated
/// from (`previous`, the version last seen) is not empty — not the first install, which has
/// «Привет» — and older than the block's, and the block is not newer than the build. Pure.
pub fn release_is_due(previous: &str, build: &str, block: &str) -> bool {
    !previous.is_empty()
        && crate::letters::version_is_newer(block, previous)
        && !crate::letters::version_is_newer(block, build)
}

/// The UI window's timer that waits for the quiet moment of «Обновлено» — once a second while a
/// window is due, gone the moment it is shown.
pub const QUIET_TIMER: usize = 31;
const QUIET_MS: u32 = 1000;

/// The UI window's timer that watches whether the flag came onto the taskbar while its hint is up.
pub const HINT_TIMER: usize = 32;
const HINT_MS: u32 = 1000;

/// The seconds without a key or the mouse the window «Обновлено» waits for (T-119-6).
pub const QUIET_SECONDS: u32 = 5;

/// At the start of the program (the UI thread, the tray up, before the first tick of the letters,
/// which moves the version seen): the start after an update writes the day of it
/// (`[letters] updated_on`, T-119-7); if the release is important, the window is written as due
/// (`[letters] release_due`); a window due from before — the program stopped before its quiet moment
/// came — stays due; then the UI window waits for the quiet moment, once a second. With «Больше не
/// показывать окна обновлений» no window. Answers the version updated from, when this start is one.
pub fn plan_release_window(owner: HWND) -> Option<String> {
    use windows::Win32::UI::WindowsAndMessaging::SetTimer;

    let state = crate::letters::state_now();
    let build = crate::letters::version_string();
    let updated = crate::letters::was_updated(&state, &build);
    let previous = updated.then(|| state.last_seen_version.clone());

    if updated {
        let today = crate::letters::today();
        crate::letters::update_state(|letters| letters.updated_on = today);
    }

    if !state.release_window {
        return previous;
    }

    let mut owed = state.clone();
    let due = owe_release_window(&mut owed, &build);

    if owed.release_due != state.release_due || owed.last_seen_version != state.last_seen_version {
        crate::letters::update_state(move |letters| {
            letters.release_due = owed.release_due;
            letters.last_seen_version = owed.last_seen_version;
        });
    }

    if due {
        // SAFETY: the live UI window and an identifier of this module; replaced if it exists.
        let _ = unsafe { SetTimer(Some(owner), QUIET_TIMER, QUIET_MS, None) };
    }

    previous
}

/// ⭐ **Whether this start owes the window «Обновлено», and what that writes** — the pure body of
/// [`plan_release_window`]. A start after an important update writes the window as due **and moves
/// the version seen to this build at once**: until the stand of 2026-11-20 the version seen moved only
/// at the first tick of the letters, ninety seconds on, and a second start before it owed the window a
/// second time. A window due from before — the program stopped before its quiet moment — stays due.
pub fn owe_release_window(letters: &mut crate::settings::Letters, build: &str) -> bool {
    if !letters.release_window {
        return false;
    }

    if crate::letters::was_updated(letters, build)
        && release_is_due(&letters.last_seen_version, build, LAST_IMPORTANT.version)
    {
        letters.release_due = letters.last_seen_version.clone();
        letters.last_seen_version = build.to_owned();
        return true;
    }

    !letters.release_due.is_empty()
}

/// [`QUIET_TIMER`]: the window «Обновлено» comes when the moment is quiet — five seconds without a
/// key or the mouse, and Windows takes notifications (not «Не беспокоить», not a full-screen
/// program — `letters::moment_is_quiet_for`). Shown, it is no longer due — once, whatever comes of
/// it. Called from the timer's message: the clock of the quiet is that message's.
pub fn quiet_tick(owner: HWND, sound: bool) {
    use windows::Win32::UI::WindowsAndMessaging::KillTimer;

    let state = crate::letters::state_now();
    let stop = || {
        // SAFETY: the timer of the live UI window.
        let _ = unsafe { KillTimer(Some(owner), QUIET_TIMER) };
    };

    if state.release_due.is_empty() || !state.release_window {
        stop();
        return;
    }

    if is_shown() || !crate::letters::moment_is_quiet_for(QUIET_SECONDS) {
        return;
    }

    if show(release_content(&state.release_due), sound, release_ended) {
        crate::letters::update_state(|letters| letters.release_due.clear());
        stop();
    }
}

/// What the window «Обновлено» says — the block of the release in the language of the interface;
/// `previous` — the version updated from («Было Y»).
pub fn release_content(previous: &str) -> Content {
    use crate::settings::{self, text};

    let language = settings::ui_language();
    let build = crate::letters::version_string();

    Content {
        about: text(settings::IDS_CORNER_UPDATE),
        title: settings::format_text(settings::IDS_RELEASE_TITLE, &[&build]),
        text: settings::format_text(settings::IDS_RELEASE_TEXT, &[previous]),
        link: Some((
            text(settings::IDS_RELEASE_CHANGES),
            crate::letters::links::changelog_url(language),
        )),
        // The main button is the passport's — on the right and wider (§10 (4)): «Как пользоваться»,
        // the action of the window; «Понятно» the second, as in the hint of the flag.
        main: (
            text(settings::IDS_RELEASE_HOWTO),
            Action::Open(crate::letters::links::program_url(language)),
        ),
        second: Some((text(settings::IDS_CORNER_OK), Action::Close)),
        picture: LAST_IMPORTANT.picture,
        check: Some(text(settings::IDS_RELEASE_NEVER)),
    }
}

/// The window «Обновлено» ended: the check box ticked turns the windows of updates off — the same
/// setting as the switch of «От автора» (T-119-10).
fn release_ended(end: End) {
    if end.checked {
        crate::letters::update_state(|letters| letters.release_window = false);
    }
}

// -----------------------------------------------------------------------------------------
// The hint of the flag — task T-119-5 (решения 208.88, 208.89 п. 6, 208.113, 208.141)
// -----------------------------------------------------------------------------------------

/// What the hint says: the flag is on and stands under ^; «Открыть Параметры Windows» opens the
/// page of the taskbar, «Понятно» and ✕ close it. Not a word about dragging the icon — the drag did
/// not move it (208.113 п. 7).
pub fn hint_content() -> Content {
    use crate::settings::{self, text};

    Content {
        about: text(settings::IDS_CORNER_FLAG),
        title: text(settings::IDS_HINT_TITLE),
        text: text(settings::IDS_HINT_TEXT),
        link: None,
        main: (
            text(settings::IDS_HINT_OPEN),
            Action::Open(TASKBAR_SETTINGS),
        ),
        second: Some((text(settings::IDS_CORNER_OK), Action::Close)),
        picture: false,
        check: None,
    }
}

/// The page of the taskbar in the settings of Windows.
pub const TASKBAR_SETTINGS: &str = "ms-settings:taskbar";

/// The flag was turned on: once per turning on, and only while it stands under ^, the hint comes
/// up at once (the person is at the settings window); a timer of the UI window takes it away the
/// moment the flag is on the taskbar.
pub fn show_flag_hint(owner: HWND, icon_id: u32) {
    use windows::Win32::UI::WindowsAndMessaging::SetTimer;

    if flag_on_the_bar(owner, icon_id) {
        return;
    }

    if show(hint_content(), false, |_| {}) {
        // SAFETY: the live UI window and an identifier of this module.
        let _ = unsafe { SetTimer(Some(owner), HINT_TIMER, HINT_MS, None) };
    }
}

/// [`HINT_TIMER`]: the hint goes by itself once the flag is on the taskbar; the timer goes with the
/// hint.
pub fn hint_tick(owner: HWND, icon_id: u32) {
    use windows::Win32::UI::WindowsAndMessaging::KillTimer;

    let hint_up = SHOWN.with(|shown| {
        shown
            .borrow()
            .as_ref()
            .is_some_and(|state| state.content.main.1 == Action::Open(TASKBAR_SETTINGS))
    });

    if !hint_up {
        // SAFETY: the timer of the live UI window.
        let _ = unsafe { KillTimer(Some(owner), HINT_TIMER) };
        return;
    }

    if flag_on_the_bar(owner, icon_id) {
        end(EndBy::Close);
        // SAFETY: as above.
        let _ = unsafe { KillTimer(Some(owner), HINT_TIMER) };
    }
}

/// Whether the flag stands on the taskbar rather than under ^ (T-119-5). Windows 11: the person's
/// choice `HKCU\Control Panel\NotifyIconSettings\<id>\IsPromoted` of the key whose
/// `ExecutablePath` is this program. Windows 10, which has no such keys: the icon's rectangle is
/// not empty and not inside ^ (the `Button` of `TrayNotifyWnd`).
pub fn flag_on_the_bar(owner: HWND, icon_id: u32) -> bool {
    if let Some(promoted) = promoted_in_settings() {
        return promoted;
    }

    let Some(icon) = icon_rect(owner, icon_id) else {
        return false;
    };

    if icon.right <= icon.left || icon.bottom <= icon.top {
        return false;
    }

    chevron_rect_10().is_none_or(|chevron| {
        !(icon.left >= chevron.left
            && icon.right <= chevron.right
            && icon.top >= chevron.top
            && icon.bottom <= chevron.bottom)
    })
}

/// The rectangle of an icon of this program's window (`Shell_NotifyIconGetRect`).
fn icon_rect(owner: HWND, icon_id: u32) -> Option<RECT> {
    use windows::Win32::UI::Shell::{NOTIFYICONIDENTIFIER, Shell_NotifyIconGetRect};

    let icon = NOTIFYICONIDENTIFIER {
        cbSize: u32::try_from(size_of::<NOTIFYICONIDENTIFIER>()).unwrap_or(0),
        hWnd: owner,
        uID: icon_id,
        ..Default::default()
    };

    // SAFETY: a live local whose `cbSize` describes it; the answer is a value.
    unsafe { Shell_NotifyIconGetRect(&icon) }.ok()
}

/// Windows 10: the ^ of the tray — the `Button` child of `TrayNotifyWnd`.
fn chevron_rect_10() -> Option<RECT> {
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, GetWindowRect};
    use windows::core::w;

    // SAFETY: constant class names; null answers are checked.
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }.ok()?;
    // SAFETY: as above.
    let notify = unsafe { FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None) }.ok()?;
    // SAFETY: as above.
    let button = unsafe { FindWindowExW(Some(notify), None, w!("Button"), None) }.ok()?;
    let mut rect = RECT::default();

    // SAFETY: a window handle by value and a local rectangle.
    unsafe { GetWindowRect(button, &mut rect) }.ok()?;
    Some(rect)
}

/// Windows 11: `IsPromoted` of this program's key under `HKCU\Control Panel\NotifyIconSettings`;
/// `None` when the keys are not there (Windows 10) or none is this program's.
fn promoted_in_settings() -> Option<bool> {
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegCloseKey,
        RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
    };
    use windows::core::{PCWSTR, PWSTR, w};

    let me = std::env::current_exe().ok()?;
    let me = me.to_string_lossy().to_lowercase();
    let mut key = HKEY::default();

    // SAFETY: a constant NUL-terminated name and a live local; the key is closed below.
    if unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Control Panel\\NotifyIconSettings"),
            None,
            KEY_READ,
            &mut key,
        )
    }
    .is_err()
    {
        return None;
    }

    let mut found = None;

    for index in 0..512u32 {
        let mut name = [0u16; 64];
        let mut length = u32::try_from(name.len()).unwrap_or(0);

        // SAFETY: the open key; a live local of `length` units.
        let status = unsafe {
            RegEnumKeyExW(
                key,
                index,
                Some(PWSTR(name.as_mut_ptr())),
                &mut length,
                None,
                None,
                None,
                None,
            )
        };

        if status.is_err() {
            break;
        }

        let sub: Vec<u16> = name[..usize::try_from(length).unwrap_or(0)]
            .iter()
            .copied()
            .chain(Some(0))
            .collect();
        let mut path = [0u16; 520];
        let mut bytes = u32::try_from(size_of_val(&path)).unwrap_or(0);

        // SAFETY: the open key, NUL-terminated names, a live buffer of `bytes` bytes the call does
        // not write past.
        let read = unsafe {
            RegGetValueW(
                key,
                PCWSTR(sub.as_ptr()),
                w!("ExecutablePath"),
                RRF_RT_REG_SZ,
                None,
                Some(path.as_mut_ptr().cast()),
                Some(&mut bytes),
            )
        };

        if read.is_err() {
            continue;
        }

        let units = usize::try_from(bytes).unwrap_or(0) / 2;
        let text = String::from_utf16_lossy(&path[..units.min(path.len())]);

        if text.trim_end_matches('\0').to_lowercase() != me {
            continue;
        }

        let mut promoted = 0u32;
        let mut size = u32::try_from(size_of::<u32>()).unwrap_or(4);

        // SAFETY: as above, a DWORD into a local of its size.
        let read = unsafe {
            RegGetValueW(
                key,
                PCWSTR(sub.as_ptr()),
                w!("IsPromoted"),
                RRF_RT_REG_DWORD,
                None,
                Some((&raw mut promoted).cast()),
                Some(&mut size),
            )
        };

        found = Some(read.is_ok() && promoted == 1);
        break;
    }

    // SAFETY: closes exactly the key opened above, once.
    let _ = unsafe { RegCloseKey(key) };

    found
}

// -----------------------------------------------------------------------------------------
// The entry «Что нового в X…» — task T-119-7 (решения 208.93 п. 5, 208.94 п. 5)
// -----------------------------------------------------------------------------------------

/// The days the entry stands after an update.
pub const WHATS_NEW_DAYS: i64 = 7;

/// Whether the entry «Что нового в X…» stands in the menu today — a week from the day of the last
/// update, **any** update, important or not. Pure.
pub fn whats_new_stands(
    updated_on: Option<crate::letters::Date>,
    today: Option<crate::letters::Date>,
) -> bool {
    match (updated_on, today) {
        (Some(updated), Some(today)) => {
            today >= updated && today < updated.plus_days(WHATS_NEW_DAYS)
        }
        _ => false,
    }
}

/// The entry was chosen: the window «Обновлено» once more if it has not been seen — it was due and
/// its quiet moment has not come — else the page of the changes on the site.
pub fn whats_new() {
    let state = crate::letters::state_now();

    if !state.release_due.is_empty()
        && !is_shown()
        && show(release_content(&state.release_due), false, release_ended)
    {
        crate::letters::update_state(|letters| letters.release_due.clear());
        return;
    }

    let _ = crate::settings::open_in_the_shell(crate::letters::links::changelog_url(
        crate::settings::ui_language(),
    ));
}
