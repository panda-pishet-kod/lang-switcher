//! Integration tests for the `theme` module, task T-11-1.
//!
//! The palette checks recompute every expected `COLORREF` from the R,G,B triples of the
//! task with an expression written in this file — never by reading the module's constants
//! or its packing helper back. `COLORREF` is `0x00BBGGRR`, the reverse of the written
//! order, and a module that packed the bytes the written way round would not fail to build
//! or to run: it would silently paint everything in someone else's colours. Independent
//! recomputation is the only test that catches it.
//!
//! [`system_is_light`] is the one machine-dependent function here and is tested for its
//! contract alone: it answers without panicking. Its value mirrors this machine's
//! `AppsUseLightTheme` and is deliberately not compared with anything.

use lang_switcher::theme::{
    Brushes, CheckMark, FOG, GRAPHITE, LABEL_TEXT_FORMAT, PaintBuffer, Palette, Reading,
    ThemeSetting, check_mark_points, combo_chevron_points, dc_is_rtl, draw_check_mark,
    label_format, reading_order, resolve, system_is_light,
};
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    CreateDIBSection, CreateSolidBrush, DIB_RGB_COLORS, DRAW_TEXT_FORMAT, DT_RTLREADING, DeleteDC,
    DeleteObject, FillRect, GetCurrentObject, GetDC, GetObjectW, GetPixel, GetStockObject, HBITMAP,
    HBRUSH, HDC, HGDIOBJ, LAYOUT_RTL, LOGBRUSH, OBJ_FONT, OEM_FIXED_FONT, ReleaseDC, SRCCOPY,
    SYSTEM_FONT, SelectObject, SetLayout, SetPixel,
};
use windows::Win32::System::Threading::{GR_GDIOBJECTS, GetCurrentProcess, GetGuiResources};

// =========================================================================================
// The palettes — criterion 11, the BGR trap
// =========================================================================================

/// The expected `COLORREF` of one written R,G,B triple, spelled out independently of the
/// module: `r | g<<8 | b<<16`, which is the `0x00BBGGRR` layout of the documentation.
const fn expected_colorref(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}

/// Every field of a palette by name, in the order of the task's table.
///
/// Eighteen since task T-12-1: `title_fg` — the ink DWM is handed for the caption text —
/// joined the seventeen the T-12-11 role `cap` had made of the sixteen of T-11-1. `cap` is
/// written out for both palettes even though «Графит» spells the same number as
/// `text_muted`, and that duplication is the point: the equality belongs to the dark palette
/// alone, and the light one puts twelve levels between the two roles.
fn fields_of(palette: &Palette) -> [(&'static str, u32); 19] {
    [
        ("window_bg", palette.window_bg.0),
        ("title_bg", palette.title_bg.0),
        ("title_fg", palette.title_fg.0),
        ("panel_bg", palette.panel_bg.0),
        ("panel_border", palette.panel_border.0),
        ("text", palette.text.0),
        ("text_muted", palette.text_muted.0),
        ("cap", palette.cap.0),
        ("field_bg", palette.field_bg.0),
        ("field_border", palette.field_border.0),
        ("button_bg", palette.button_bg.0),
        ("button_border", palette.button_border.0),
        ("accent_bg", palette.accent_bg.0),
        ("accent_fg", palette.accent_fg.0),
        ("box_border", palette.box_border.0),
        ("sel_bg", palette.sel_bg.0),
        ("sel_fg", palette.sel_fg.0),
        ("hover_bg", palette.hover_bg.0),
        // ⚠ Последним, и это обязательно: сверка идёт `zip`-ом с таблицами триплетов
        // и первым делом сличает имена. Вставка в середину одной стороны без другой
        // роняет проверку на «the field order of the test tables must match `fields_of`».
        ("menu_hover_bg", palette.menu_hover_bg.0),
    ]
}

/// The R,G,B triples of the task's table, column «Графит», copied as written.
const GRAPHITE_TRIPLES: [(&str, (u32, u32, u32)); 19] = [
    ("window_bg", (32, 35, 41)),
    ("title_bg", (26, 29, 34)),
    // `ui.ps1:177` — `TitleFg=(Col 232 234 236)`, four levels above `text` on that same
    // line and read against `title_bg` and not against the ground of the client area.
    ("title_fg", (232, 234, 236)),
    ("panel_bg", (39, 43, 50)),
    ("panel_border", (54, 59, 67)),
    ("text", (228, 231, 234)),
    ("text_muted", (152, 160, 168)),
    // `ui.ps1:179` — `Cap=(Col 152 160 168)`, the same number as `Muted` on that line.
    ("cap", (152, 160, 168)),
    ("field_bg", (25, 28, 33)),
    ("field_border", (58, 64, 72)),
    ("button_bg", (46, 51, 59)),
    ("button_border", (63, 69, 78)),
    ("accent_bg", (228, 231, 234)),
    ("accent_fg", (27, 30, 35)),
    ("box_border", (86, 96, 112)),
    ("sel_bg", (60, 66, 76)),
    ("sel_fg", (240, 242, 244)),
    ("hover_bg", (52, 58, 67)),
    // Тот же номер, что и строкой выше: на тёмной земле меню полоса уже отстояла
    // на 20/23/26, и двигать её было нечего. Совпадение намеренное.
    ("menu_hover_bg", (52, 58, 67)),
];

/// The R,G,B triples of the task's table, column «Туман», copied as written.
const FOG_TRIPLES: [(&str, (u32, u32, u32)); 19] = [
    ("window_bg", (237, 239, 242)),
    ("title_bg", (247, 248, 250)),
    // `ui.ps1:188` — `TitleFg=(Col 35 38 43)`. The number `text` also carries in this
    // palette, written out all the same: the equality belongs to «Туману», not to the roles.
    ("title_fg", (35, 38, 43)),
    ("panel_bg", (255, 255, 255)),
    ("panel_border", (225, 229, 234)),
    ("text", (35, 38, 43)),
    ("text_muted", (110, 118, 127)),
    // `ui.ps1:190` — `Cap=(Col 122 130 139)` against `Muted=(Col 110 118 127)`: this is the
    // palette where the two roles part company, twelve levels in every channel.
    ("cap", (122, 130, 139)),
    ("field_bg", (255, 255, 255)),
    ("field_border", (207, 213, 220)),
    ("button_bg", (255, 255, 255)),
    ("button_border", (207, 213, 220)),
    ("accent_bg", (43, 47, 54)),
    ("accent_fg", (245, 246, 247)),
    ("box_border", (168, 176, 185)),
    ("sel_bg", (228, 232, 237)),
    ("sel_fg", (35, 38, 43)),
    ("hover_bg", (234, 238, 242)),
    // Отличается от строки выше намеренно и ради того, чтобы вид СОВПАЛ: земля меню
    // здесь `window_bg` 237,239,242, а не белая `button_bg`, и общий номер давал
    // 3/1/0 — полосы не было видно. Это число отстоит на 21/17/13, как у кнопки.
    ("menu_hover_bg", (216, 222, 229)),
];

/// Checks one palette against one column of the task's table, field by field.
fn assert_palette_matches(
    palette_name: &str,
    palette: &Palette,
    triples: &[(&str, (u32, u32, u32)); 19],
) {
    for ((field, actual), (expected_field, (r, g, b))) in fields_of(palette).iter().zip(triples) {
        // The rows travel in the order of the task's table on both sides; a mismatch here
        // means the test itself is broken, not the palette.
        assert_eq!(
            field, expected_field,
            "the field order of the test tables must match `fields_of`"
        );

        let expected = expected_colorref(*r, *g, *b);
        assert_eq!(
            *actual, expected,
            "{palette_name}.{field}: expected COLORREF {expected:#010x} of R,G,B \
             ({r},{g},{b}) — a mismatch that looks byte-swapped means the 0x00BBGGRR \
             packing of FR-92а was written the wrong way round"
        );
    }
}

#[test]
fn every_graphite_field_is_the_bgr_packing_of_its_written_triple() {
    assert_palette_matches("GRAPHITE", &GRAPHITE, &GRAPHITE_TRIPLES);
}

#[test]
fn every_fog_field_is_the_bgr_packing_of_its_written_triple() {
    assert_palette_matches("FOG", &FOG, &FOG_TRIPLES);
}

// =========================================================================================
// The resolution — criterion 10, all six rows of FR-92а
// =========================================================================================

#[test]
fn the_resolve_table_is_all_six_combinations_and_each_answers_the_very_palette() {
    // 3 settings × 2 system states — the whole input space of the pure half of FR-92а.
    let table: [(ThemeSetting, bool, &'static Palette, &str); 6] = [
        (ThemeSetting::System, true, &FOG, "FOG"),
        (ThemeSetting::System, false, &GRAPHITE, "GRAPHITE"),
        (ThemeSetting::Light, true, &FOG, "FOG"),
        (ThemeSetting::Light, false, &FOG, "FOG"),
        (ThemeSetting::Dark, true, &GRAPHITE, "GRAPHITE"),
        (ThemeSetting::Dark, false, &GRAPHITE, "GRAPHITE"),
    ];

    for (setting, system_light, expected, expected_name) in table {
        let resolved = resolve(setting, system_light);

        // Identity, not equality: the acceptance instrument of position 25 compares pixels
        // against the module's constants, so `resolve` must answer with those very statics
        // and not with a copy that could drift away from them.
        assert!(
            std::ptr::eq(resolved, expected),
            "resolve({setting:?}, system_light = {system_light}) must be the very static \
             {expected_name}, not a copy"
        );
    }
}

/// **Task T-13-17 — what makes a pixel probe of a palette change mean anything.**
///
/// The acceptance of a window that follows the system theme is a pair of shots and a probe
/// of the same pixels on both of them, against these constants. That probe only says
/// something while the two palettes actually *differ* in the roles it reads: a role the two
/// palettes spelled the same number for would read identically before and after a real
/// change and identically before and after no change at all, and the shot would prove
/// nothing either way.
///
/// The four roles below are the ones the стенд probes — the ground of the client area, the
/// ink on it, and the two colours DWM is handed for the caption — and this test pins that
/// they are four honest discriminators. (`cap` and `text_muted` are deliberately *not* here:
/// «Графит» spells one number for both, which is a property of that palette the module
/// documents at length, and neither role is probed.)
#[test]
fn the_roles_a_theme_change_is_probed_by_differ_between_the_two_palettes() {
    let roles: [(&str, u32, u32); 4] = [
        ("window_bg", GRAPHITE.window_bg.0, FOG.window_bg.0),
        ("text", GRAPHITE.text.0, FOG.text.0),
        ("title_bg", GRAPHITE.title_bg.0, FOG.title_bg.0),
        ("title_fg", GRAPHITE.title_fg.0, FOG.title_fg.0),
    ];

    for (role, graphite, fog) in roles {
        println!("{role}: GRAPHITE 0x{graphite:06X} vs FOG 0x{fog:06X}");

        assert_ne!(
            graphite, fog,
            "«Графит» and «Туман» must not spell one number for `{role}` — a probe of that \
             role could then not tell a palette change from no change at all"
        );
    }
}

// =========================================================================================
// The setting — criterion 12, the literals and the garbage
// =========================================================================================

#[test]
fn the_three_literal_values_read_as_themselves() {
    assert_eq!(
        ThemeSetting::from_config_str("system"),
        ThemeSetting::System
    );
    assert_eq!(ThemeSetting::from_config_str("light"), ThemeSetting::Light);
    assert_eq!(ThemeSetting::from_config_str("dark"), ThemeSetting::Dark);
}

#[test]
fn everything_else_reads_as_system_silently() {
    // The empty string, plain garbage, and the letter case FR-92а's literals do not use:
    // section 7 requires garbage in the configuration to be survived silently, and FR-92а
    // names `system` the default it falls back to.
    for garbage in [
        "", "junk", "DARK", "Dark", "LIGHT", " dark", "dark ", "auto",
    ] {
        assert_eq!(
            ThemeSetting::from_config_str(garbage),
            ThemeSetting::System,
            "{garbage:?} is not one of the three literals and must read as System"
        );
    }
}

#[test]
fn as_config_str_writes_the_literal_that_reads_back_as_the_same_setting() {
    for setting in [
        ThemeSetting::System,
        ThemeSetting::Light,
        ThemeSetting::Dark,
    ] {
        let written = setting.as_config_str();

        assert_eq!(
            ThemeSetting::from_config_str(written),
            setting,
            "{setting:?} must survive the round trip through its configuration string"
        );
    }

    // And the strings themselves are the three literals of section 7, not merely a
    // self-consistent invention.
    assert_eq!(ThemeSetting::System.as_config_str(), "system");
    assert_eq!(ThemeSetting::Light.as_config_str(), "light");
    assert_eq!(ThemeSetting::Dark.as_config_str(), "dark");
}

// =========================================================================================
// The system switch — criterion 13, honest about what it can check
// =========================================================================================

#[test]
fn system_is_light_answers_without_panicking_and_the_value_is_machine_dependent() {
    // The value mirrors this machine's `AppsUseLightTheme` and is therefore deliberately
    // not compared with anything: both answers are legitimate on some machine, and a test
    // that pinned one would fail on the other machine while verifying nothing. What is
    // verified is the whole verifiable contract: the call comes back with a `bool` at all —
    // no panic, whatever this profile's registry holds.
    let answer: bool = system_is_light();

    if answer {
        println!("this machine asks its applications for the light theme");
    } else {
        println!("this machine asks its applications for the dark theme");
    }
}

// =========================================================================================
// The brushes — criterion 14, creation and a single release
// =========================================================================================

#[test]
fn a_brush_set_is_created_with_live_handles_that_carry_the_palette_and_drops_once() {
    for (palette_name, palette) in [("GRAPHITE", &GRAPHITE), ("FOG", &FOG)] {
        let brushes = Brushes::new(palette)
            .unwrap_or_else(|| panic!("the brush set of {palette_name} must be creatable"));

        // Eight since task T-11-5b: `box_border` — the `FrameRect` frame of an unchecked
        // owner-drawn check box or radio button — joined the seven of T-11-5a; same
        // owner, same creation, same release.
        let named: [(&str, HBRUSH, u32); 8] = [
            ("window_bg", brushes.window_bg(), palette.window_bg.0),
            ("panel_bg", brushes.panel_bg(), palette.panel_bg.0),
            ("field_bg", brushes.field_bg(), palette.field_bg.0),
            ("button_bg", brushes.button_bg(), palette.button_bg.0),
            ("sel_bg", brushes.sel_bg(), palette.sel_bg.0),
            (
                "button_border",
                brushes.button_border(),
                palette.button_border.0,
            ),
            ("accent_bg", brushes.accent_bg(), palette.accent_bg.0),
            ("box_border", brushes.box_border(), palette.box_border.0),
        ];

        for (field, handle, expected_color) in named {
            assert!(
                !handle.is_invalid(),
                "{palette_name}: the {field} brush handle must be live"
            );

            let mut log = LOGBRUSH::default();

            // SAFETY: `handle` is the live brush checked on the line above, `log` is a
            // live local of exactly the size the call is told, and `GetObjectW` writes at
            // most that many bytes into it and keeps no pointer once it returns.
            let written = unsafe {
                GetObjectW(
                    handle.into(),
                    size_of::<LOGBRUSH>() as i32,
                    Some((&raw mut log).cast()),
                )
            };

            assert_eq!(
                written as usize,
                size_of::<LOGBRUSH>(),
                "{palette_name}: GetObjectW must describe the {field} brush"
            );
            assert_eq!(
                log.lbColor.0, expected_color,
                "{palette_name}: the {field} brush must be solid {expected_color:#010x} — \
                 the very palette field it was created from"
            );
        }

        // The release under test: `Drop` frees all eight handles here, exactly once. No
        // double release is reachable from safe code — `Brushes` is neither `Copy` nor
        // `Clone`, its fields are private and never reassigned, the getters hand out
        // borrowed copies without ownership, and `drop` runs once per value.
        drop(brushes);
    }
}

// =========================================================================================
// Внеэкранный буфер одного элемента — task T-15-1
// =========================================================================================
//
// The property these tests are about is the one the type exists for and the one a picture of a
// finished button cannot show: a drawing body handed this buffer goes on naming **the coordinates
// of the window** — for what it writes, for what it reads back out again, and for the blit that
// carries the result over. The reading half is not decoration. The corner smoothing of this very
// module blits the ground of each corner **out of** the DC it is drawing into, so a buffer that
// moved only the writes would blend four arcs into the wrong pixels and shift the halftones of
// every rounded figure in the program, with nothing to say so.
//
// The surfaces below stand in for the DC of a window: a memory DC over the **screen** DC and a
// bitmap compatible with the screen, which is the colour depth a window has. A bitmap made
// compatible with a memory DC would be monochrome — the trap the module words twice — and these
// tests would then be measuring one bit per pixel.

/// The ground every surface below starts from, so that «nothing was written here» is an
/// assertion against a known colour and not against the undefined pixels of a fresh bitmap.
const GROUND: u32 = expected_colorref(16, 32, 48);

/// What a drawing body lays down inside its own rectangle.
const INK: u32 = expected_colorref(200, 216, 232);

/// One pixel of a third colour, so that a picture that arrived whole but a pixel askew is told
/// apart from one that arrived where it belongs.
const MARK: u32 = expected_colorref(248, 8, 128);

/// A colour surface of `width` × `height`, standing in for the DC of a window.
struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
}

impl Surface {
    /// Makes the surface and grounds all of it with [`GROUND`].
    fn new(width: i32, height: i32) -> Self {
        // SAFETY: `None` asks for the DC of the whole screen, which every process may have; it
        // is released a few lines below and never kept.
        let screen = unsafe { GetDC(None) };
        assert!(!screen.is_invalid(), "the screen DC must be obtainable");

        // SAFETY: a memory DC over the screen's, deleted in `Drop`.
        let dc = unsafe { CreateCompatibleDC(Some(screen)) };

        // SAFETY: compatible with the *screen* DC — so a colour bitmap and not a monochrome
        // one — and deleted in `Drop`.
        let bitmap = unsafe { CreateCompatibleBitmap(screen, width, height) };

        // SAFETY: `screen` is the handle `GetDC` answered, released exactly once.
        unsafe { ReleaseDC(None, screen) };

        assert!(!dc.is_invalid(), "the memory DC must be creatable");
        assert!(!bitmap.is_invalid(), "the colour bitmap must be creatable");

        // SAFETY: both handles are live and ours; what the DC was born with is put back in
        // `Drop`, before the bitmap is deleted.
        let previous = unsafe { SelectObject(dc, bitmap.into()) };
        assert!(!previous.is_invalid(), "the bitmap must be selectable");

        let surface = Self {
            dc,
            bitmap,
            previous,
        };

        fill(
            surface.dc,
            &RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            },
            GROUND,
        );

        surface
    }

    /// One pixel of the surface, as a `COLORREF` number.
    fn pixel(&self, x: i32, y: i32) -> u32 {
        // SAFETY: `self.dc` is this value's live memory DC; the call reads one pixel of the
        // bitmap selected into it and touches no memory of ours.
        unsafe { GetPixel(self.dc, x, y) }.0
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        // SAFETY: the bitmap the DC was born with goes back first, which frees ours to be
        // deleted; both handles then go exactly once, and `drop` runs once.
        unsafe { SelectObject(self.dc, self.previous) };
        let _ = unsafe { DeleteObject(self.bitmap.into()) };
        let _ = unsafe { DeleteDC(self.dc) };
    }
}

/// Fills `area` of `dc` with one solid colour, brush made and freed on this frame.
fn fill(dc: HDC, area: &RECT, color: u32) {
    // SAFETY: a plain solid brush, deleted a few lines below and handed to nobody else.
    let brush = unsafe { CreateSolidBrush(COLORREF(color)) };
    assert!(!brush.is_invalid(), "the brush must be creatable");

    // SAFETY: `dc` is a live DC of the caller, `area` a live local of this frame, and `brush`
    // the live brush of the line above.
    let painted = unsafe { FillRect(dc, area, brush) };
    assert_ne!(painted, 0, "the fill must succeed");

    // SAFETY: the brush is selected into nothing and is freed exactly once.
    let _ = unsafe { DeleteObject(brush.into()) };
}

/// The rectangle these tests use: **not** at the origin, because a row of a list is not, and a
/// buffer that quietly assumed the client corner would pass every test placed at `(0, 0)`.
const OFFSET_AREA: RECT = RECT {
    left: 7,
    top: 5,
    right: 22,
    bottom: 18,
};

#[test]
fn a_rectangle_with_nothing_in_it_gets_no_buffer() {
    let surface = Surface::new(40, 30);

    let empty = [
        (
            "no width",
            RECT {
                left: 7,
                top: 5,
                right: 7,
                bottom: 18,
            },
        ),
        (
            "no height",
            RECT {
                left: 7,
                top: 5,
                right: 22,
                bottom: 5,
            },
        ),
        (
            "inverted",
            RECT {
                left: 22,
                top: 18,
                right: 7,
                bottom: 5,
            },
        ),
    ];

    for (name, area) in empty {
        // SAFETY: `surface.dc` is a live memory DC of this frame, read and not written.
        let buffer = unsafe { PaintBuffer::for_rect(surface.dc, &area) };

        assert!(
            buffer.is_none(),
            "a rectangle with {name} has no picture to hold and must get no surface"
        );
    }
}

/// **The coordinate arrangement of T-15-1, the writing half.** A body drawn into the buffer
/// names the rectangle it was handed, and the picture arrives on the window at that very
/// rectangle — not at the origin, not a pixel off, and not one pixel outside it.
#[test]
fn a_buffer_takes_the_callers_own_coordinates_and_hands_the_picture_back_unmoved() {
    let surface = Surface::new(40, 30);
    let area = OFFSET_AREA;

    // SAFETY: `surface.dc` is a live memory DC of this frame; the buffer reads it and writes to
    // it only in the blit below.
    let buffer = unsafe { PaintBuffer::for_rect(surface.dc, &area) }
        .expect("a buffer for a non-empty rectangle must be creatable");

    // The body: the caller's own coordinates, written exactly as they would be on the window.
    fill(buffer.dc(), &area, INK);

    // SAFETY: `buffer.dc()` is the live memory DC of the buffer; the call writes one pixel of
    // the bitmap selected into it.
    unsafe { SetPixel(buffer.dc(), area.left, area.top, COLORREF(MARK)) };

    // SAFETY: `surface.dc` is the live DC the buffer was made for.
    assert!(
        unsafe { buffer.blit(surface.dc) },
        "a one-to-one blit between two compatible surfaces must succeed"
    );

    drop(buffer);

    // The corner the mark was put on — the whole test in one pixel: a picture that arrived at
    // the origin, or a row too high, or a column too far left, misses it.
    assert_eq!(
        surface.pixel(area.left, area.top),
        MARK,
        "the marked pixel must land on the very corner of the rectangle the buffer was made for"
    );

    // Every edge of the rectangle, inside and out. `right` and `bottom` are exclusive, so the
    // last painted pixel is one short of each and the first untouched one is exactly on them.
    let inside = [
        (area.left + 1, area.top),
        (area.left, area.top + 1),
        (area.right - 1, area.top),
        (area.left, area.bottom - 1),
        (area.right - 1, area.bottom - 1),
    ];

    for (x, y) in inside {
        assert_eq!(
            surface.pixel(x, y),
            INK,
            "({x}, {y}) is inside the rectangle and must carry what the body drew"
        );
    }

    let outside = [
        (area.left - 1, area.top),
        (area.left, area.top - 1),
        (area.right, area.top),
        (area.left, area.bottom),
        (area.right, area.bottom),
        (0, 0),
    ];

    for (x, y) in outside {
        assert_eq!(
            surface.pixel(x, y),
            GROUND,
            "({x}, {y}) is outside the rectangle and must be untouched — an element may not \
             write past the rectangle the message gave it"
        );
    }
}

/// **The coordinate arrangement of T-15-1, the reading half — and the trap of the whole task.**
///
/// `Supersample::render` blits the ground of a corner **out of** the DC the figure is being drawn
/// into, at the corner's own coordinates on the window. So the mapping has to act on reads as
/// well as on writes: this is the test that says it does, and a buffer that only shifted the
/// writes would move the halftones of every rounded corner in the program while every picture
/// still looked plausible.
#[test]
fn a_read_out_of_a_buffer_goes_through_the_same_coordinates_as_a_write() {
    let surface = Surface::new(40, 30);
    let area = OFFSET_AREA;

    // SAFETY: as in the test above.
    let buffer = unsafe { PaintBuffer::for_rect(surface.dc, &area) }
        .expect("a buffer for a non-empty rectangle must be creatable");

    fill(buffer.dc(), &area, INK);

    let (mark_x, mark_y) = (area.left + 2, area.top + 3);

    // SAFETY: `buffer.dc()` is the live memory DC of the buffer.
    unsafe { SetPixel(buffer.dc(), mark_x, mark_y, COLORREF(MARK)) };

    // A read through the DC, at the coordinate of the window — what `GetPixel` and a blit both
    // put through the mapping.
    // SAFETY: as above.
    assert_eq!(
        unsafe { GetPixel(buffer.dc(), mark_x, mark_y) }.0,
        MARK,
        "a pixel read at the coordinate it was written at must be the pixel that was written"
    );

    // And the read the smoothing actually performs: one pixel blitted **out of** the buffer at
    // the coordinate the corner has on the window.
    let scratch = Surface::new(4, 4);

    // SAFETY: both DCs are live memory DCs of this frame; the source point is a logical point of
    // the buffer, which is what a blit takes.
    unsafe {
        BitBlt(
            scratch.dc,
            0,
            0,
            1,
            1,
            Some(buffer.dc()),
            mark_x,
            mark_y,
            SRCCOPY,
        )
    }
    .expect("the read-back blit must succeed");

    assert_eq!(
        scratch.pixel(0, 0),
        MARK,
        "a blit out of the buffer at the coordinate of the window must read the pixel that is \
         there — this is the ground the corner smoothing blends its arcs into"
    );
}

/// **NFR-13, the degraded path that would otherwise change the picture.** A body whose own face
/// was refused draws in whatever face the DC already carried; a fresh memory DC carries the stock
/// `SYSTEM_FONT`, which is neither the same face nor the same metrics. The buffer therefore
/// arrives wearing the face of the DC it stands in for.
#[test]
fn a_buffer_carries_over_the_face_the_dc_it_stands_in_for_was_wearing() {
    let surface = Surface::new(40, 30);

    // SAFETY: stock objects are owned by the system, are never deleted by a process, and may be
    // selected into any DC.
    let fixed = unsafe { GetStockObject(OEM_FIXED_FONT) };
    // SAFETY: as above — the face a fresh memory DC is born with, kept for the contrast below.
    let stock = unsafe { GetStockObject(SYSTEM_FONT) };

    assert!(
        !fixed.is_invalid() && !stock.is_invalid(),
        "stock faces exist"
    );
    assert_ne!(
        fixed, stock,
        "the two stock faces must be different handles"
    );

    // SAFETY: `surface.dc` is a live memory DC of this frame and `fixed` a stock object; nothing
    // of ours is displaced and nothing is deleted.
    unsafe { SelectObject(surface.dc, fixed) };

    // SAFETY: `surface.dc` is live and is read, not written.
    let buffer = unsafe { PaintBuffer::for_rect(surface.dc, &OFFSET_AREA) }
        .expect("a buffer for a non-empty rectangle must be creatable");

    // SAFETY: `buffer.dc()` is the buffer's live memory DC; the call reads an attribute of it.
    let carried = unsafe { GetCurrentObject(buffer.dc(), OBJ_FONT) };

    assert_eq!(
        carried, fixed,
        "the buffer must wear the face of the DC it stands in for, not the one a fresh memory \
         DC is born with"
    );
    assert_ne!(
        carried, stock,
        "and that face must actually differ from the stock one, or this test proves nothing"
    );
}

/// **§5.6 of the task — a leaked GDI object gives neither an error nor a red test.** So it is
/// counted: two objects per buffer, two hundred buffers made and dropped, and the process must
/// end holding no more of them than it started with.
#[test]
fn buffers_made_and_dropped_leave_no_gdi_object_behind() {
    let surface = Surface::new(40, 30);

    // One buffer first, so that whatever GDI allocates once for this process is already
    // allocated when the baseline is taken.
    // SAFETY: `surface.dc` is a live memory DC of this frame.
    drop(unsafe { PaintBuffer::for_rect(surface.dc, &OFFSET_AREA) });

    let count = || {
        // SAFETY: `GetCurrentProcess` answers a pseudo-handle that needs no closing, and the
        // call reads a counter of this process.
        unsafe { GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS) }
    };

    let before = count();

    for _ in 0..200 {
        // SAFETY: as above.
        let buffer = unsafe { PaintBuffer::for_rect(surface.dc, &OFFSET_AREA) }
            .expect("a buffer for a non-empty rectangle must be creatable");

        fill(buffer.dc(), &OFFSET_AREA, INK);
    }

    let after = count();

    println!("GDI objects of this process: {before} before, {after} after 200 buffers");

    // Two hundred leaked buffers would be four hundred objects; the small slack is for whatever
    // else the process may allocate while the loop runs, and is two orders below the leak.
    assert!(
        after <= before + 8,
        "buffers must free their DC and their bitmap on every path: {before} objects before the \
         loop, {after} after it"
    );
}

// =========================================================================================
// Чип-«клавиша»: чистая арифметика коробки и разбор строки — task Т-26-2, решение 87
// =========================================================================================

/// **The box a chip wants around a key name** — task Т-26-2, решение 85 п. 1.
///
/// Pure arithmetic, so it is a table and not a hope: the air is a share of the chip face's em
/// (the mock-up's `padding: 2px 5px 3px` on an 11 px face), the interior is the character cell
/// the pen will draw into, and the frame is counted twice — once on each side — so it does not
/// eat the interior.
///
/// The numbers are recomputed here from the percentages rather than read back off the module,
/// for the reason the palette checks above recompute their triples: a constant compared with
/// itself proves nothing.
#[test]
fn the_chip_box_is_air_around_the_key_and_the_frame_is_counted_on_both_sides() {
    use lang_switcher::theme::{
        CHIP_PAD_X_PERCENT, CHIP_PAD_Y_PERCENT, CHIP_RADIUS_PERCENT, chip_box,
    };

    // The face of the chip at 96 DPI: `lfHeight` −11, a character cell 13 px tall — the pair
    // measured on the raised window, `scratchpad-Э26` probe.
    let (em, line, thickness) = (11, 13, 1);
    let text = 30;

    let made = chip_box(text, em, line, thickness);

    let pad_x = (em * CHIP_PAD_X_PERCENT) / 100;
    let pad_y = (em * CHIP_PAD_Y_PERCENT) / 100;

    println!("чип: {made:?}, воздух {pad_x} × {pad_y}");

    assert_eq!(
        made.width,
        text + 2 * (pad_x + thickness),
        "the width is the text plus air and frame on both sides"
    );
    assert_eq!(
        made.height,
        line + 2 * (pad_y + thickness),
        "the height is the character cell plus air and frame on both sides"
    );
    assert_eq!(made.radius, (em * CHIP_RADIUS_PERCENT) / 100);
    assert_eq!(
        made.inset_x,
        pad_x + thickness,
        "the first glyph clears the frame"
    );
    assert_eq!(made.inset_y, pad_y + thickness);

    // ⚠ The interior is never eaten by the frame: whatever the thickness, the room left for the
    // text is exactly what was asked for. This is the one property a DPI change could break.
    for thickness in 1..=3 {
        let made = chip_box(text, em, line, thickness);

        assert_eq!(
            made.width - 2 * made.inset_x,
            text,
            "a {thickness} px frame must not eat the text it goes round"
        );
    }

    // The air grows with the face and never with anything else: a chip on a twice larger face
    // is more than twice as tall, because the cell grows too.
    let larger = chip_box(text, em * 2, line * 2, thickness);

    assert!(
        larger.height > made.height && larger.radius > made.radius,
        "the box is a share of its own face: {made:?} against {larger:?}"
    );

    // A key that measures nothing is still a box, not a negative one.
    let empty = chip_box(0, em, line, thickness);

    assert!(
        empty.width > 0 && empty.height > 0,
        "a chip round no text is air and frame, not a hole: {empty:?}"
    );
}

/// **Where the chip stands in a help row** — task Т-26-2, решение 85 п. 1.
///
/// The row is split at the placeholder the string tables have carried since task Т-23-4, which
/// is why решение 85 cost no new localisation string at all. Three cases and no fourth: a row
/// with the placeholder, a row without one (rows 4 and 5 of the help), and a key name that is
/// empty — a box round nothing is not a chip, so the two halves are joined back up.
#[test]
fn a_help_row_splits_at_the_placeholder_and_nowhere_else() {
    use lang_switcher::theme::{KEY_PLACEHOLDER, chip_row};

    // The real row 1 of the Russian table, byte for byte.
    let row = "Набрали слово не в той раскладке — нажмите {0}: слово перекодируется.";
    let split = chip_row(row, "Pause");

    println!("{split:?}");

    assert_eq!(split.key, Some("Pause"));
    assert_eq!(split.prefix, "Набрали слово не в той раскладке — нажмите ");
    assert_eq!(
        split.suffix, ": слово перекодируется.",
        "the colon is glued to the chip and must stay on its side of the split"
    );

    // ⚠ Nothing is lost and nothing is invented: the two halves and the placeholder are the row.
    assert_eq!(
        format!("{}{KEY_PLACEHOLDER}{}", split.prefix, split.suffix),
        row,
        "the split must be reversible"
    );

    // A row with no placeholder — rows 4 and 5 — is all prefix and has no chip.
    let plain = "Пауза, настройки и выход — в значке в трее.";
    let split = chip_row(plain, "Pause");

    assert_eq!(split.key, None, "a row with no placeholder has no chip");
    assert_eq!(split.prefix, plain);
    assert_eq!(split.suffix, "");

    // An empty key name: the row keeps its words and loses only the figure — task T-43-3,
    // finding Н138. ⚠ Until that task this block asserted an **empty suffix**, which is the very
    // defect: the branch kept the first half of the sentence and dropped the second, and the
    // test of the split held the loss in place, so it could not fail.
    let split = chip_row(row, "");

    assert_eq!(split.key, None, "a box round an empty name is not a chip");
    assert_eq!(split.prefix, "Набрали слово не в той раскладке — нажмите ");
    assert_eq!(
        split.suffix, ": слово перекодируется.",
        "an empty name takes the figure away and nothing else — the tail stays in the row"
    );
    assert_eq!(
        format!("{}{KEY_PLACEHOLDER}{}", split.prefix, split.suffix),
        row,
        "the split of an empty name must be reversible too"
    );

    // The example of the mandate, word for word. The two halves carry their own spacing — the
    // rule of the split above — so the tail begins with the space that stood after `{0}`.
    let split = chip_row("до {0} после", "");

    assert_eq!(split.prefix, "до ");
    assert_eq!(split.key, None);
    assert_eq!(split.suffix, " после");
}

/// **Task T-43-3, finding Н138** — an empty key name draws the **whole** sentence, not its first
/// half.
///
/// `theme::chip_row` promised it in so many words — «the two halves are drawn as one sentence» —
/// and did the opposite, and the test above held the opposite in place. The value was not the
/// whole of it, and that was measured before the repair (`scratchpad-E43\premises-p4.log`): the
/// one place a row becomes words, `push_row_atoms`, took **only** `row.prefix` for a row with no
/// key, so mending the split alone would have changed a value and not one pixel.
///
/// So the pen is asked, not the value. A row whose key name is empty is drawn; the same sentence
/// with the placeholder cut out is drawn on a second sheet as a row that never had one (rows 4
/// and 5 of the help); and the two sheets must be the same pixel for pixel, with the measuring
/// twin answering the same lines and the same height for both. ⚠ Not against a literal — the
/// приём of Э33: two ways of asking one question.
#[test]
fn an_empty_key_name_draws_the_whole_sentence_and_not_its_first_half() {
    use lang_switcher::theme::{
        ChipColors, ChipRowMetrics, ChipRowStyle, KEY_PLACEHOLDER, chip_row, measure_chip_row,
        paint_chip_row,
    };
    use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, LOGFONTW};

    let mut logical = LOGFONTW {
        lfHeight: -14,
        lfWeight: 400,
        ..Default::default()
    };

    for (slot, unit) in logical.lfFaceName.iter_mut().zip("Segoe UI".encode_utf16()) {
        *slot = unit;
    }

    let mut chip_logical = logical;
    chip_logical.lfHeight = -12;

    // SAFETY: both structures are live locals of this frame; the handles are freed at the end.
    let (body, chip) = unsafe {
        (
            CreateFontIndirectW(&raw const logical),
            CreateFontIndirectW(&raw const chip_logical),
        )
    };

    // SAFETY: a plain colour in, a handle out, freed at the end.
    let ground = unsafe { CreateSolidBrush(COLORREF(0x00FF_FFFF)) };

    let pitch = 20;

    let metrics = ChipRowMetrics {
        body: Some(body),
        chip_face: Some((chip, chip_logical.lfHeight)),
        pitch,
        dpi: 96,
    };

    let style = ChipRowStyle {
        ground,
        ink: COLORREF(0),
        body: Some(body),
        chip_face: Some((chip, chip_logical.lfHeight)),
        chip: ChipColors {
            outline: COLORREF(0),
            fill: ground,
            ink: COLORREF(0),
        },
        pitch,
        dpi: 96,
    };

    // A tail that wraps, a tail that does not, a colon glued to the placeholder, and a
    // placeholder that opens the sentence — the four places a half can stand.
    let sentences = [
        "Набрали слово не в той раскладке — нажмите {0}: слово перекодируется, раскладка переключится.",
        "Zaznacz tekst i naciśnij {0} — zaznaczenie zostanie przekonwertowane.",
        "Повторное нажатие {0} возвращает всё назад.",
        "{0} — клавиша, которой переключается раскладка.",
    ];

    for width in [140, 260] {
        for sentence in sentences {
            let (prefix, suffix) = sentence
                .split_once(KEY_PLACEHOLDER)
                .expect("every sentence of this test carries the placeholder");
            let joined = format!("{prefix}{suffix}");

            let keyless = chip_row(sentence, "");
            let plain = chip_row(&joined, "Pause");

            assert_eq!(
                plain.key, None,
                "the joined sentence has no placeholder left"
            );

            // SAFETY: a memory DC of the test's own and two faces alive for the whole test.
            let measured = unsafe {
                let sheet = Sheet::new(8, false);

                (
                    measure_chip_row(sheet.dc, width, keyless, metrics),
                    measure_chip_row(sheet.dc, width, plain, metrics),
                )
            };

            println!("width {width}: {measured:?} — «{sentence}»");

            assert_eq!(
                measured.0, measured.1,
                "width {width}: the twin measured the empty-name row as {:?} and the sentence \
                 it stands for as {:?} — the tail is not being measured. «{sentence}»",
                measured.0, measured.1
            );

            let drawn = |row| {
                let sheet = Sheet::new(320, false);
                let area = RECT {
                    left: 4,
                    top: 4,
                    right: 4 + width,
                    bottom: sheet.side,
                };

                // SAFETY: own DC, own rectangle, live brush and faces.
                unsafe { paint_chip_row(sheet.dc, area, row, style) };

                (sheet.inked(), sheet.pixels())
            };

            let (keyless_ink, keyless_pixels) = drawn(keyless);
            let (plain_ink, plain_pixels) = drawn(plain);

            assert!(
                plain_ink > 0,
                "width {width}: the sentence must leave ink at all"
            );
            assert!(
                keyless_pixels == plain_pixels,
                "width {width}: the empty-name row left {keyless_ink} inked pixels and the \
                 sentence it stands for {plain_ink} — the pen is not drawing the tail. \
                 «{sentence}»"
            );
        }
    }

    // SAFETY: every handle was made here, handed to nobody, and is freed exactly once.
    unsafe {
        let _ = DeleteObject(body.into());
        let _ = DeleteObject(chip.into());
        let _ = DeleteObject(ground.into());
    }
}

/// **Task T-42-2, finding С44** — the measuring twin and the pen put a help row on the same
/// number of lines, and they agree **in pixels**.
///
/// `theme::measure_chip_row` is what the about window lays its help panel out by
/// (`settings::fit_about_help`); `theme::paint_chip_row` is what draws it. They share
/// `push_row_atoms` and `wrap_atoms` by construction — but a construction is a promise, and
/// this test is the measurement: the row is **drawn** on a sheet, the runs of ink are counted
/// off the pixels, and the count is held against the number the twin answered.
///
/// ⚠ **Not against a literal** — the приём of Э33. What is compared is two ways of asking the
/// same question, so the test keeps holding when the wrap itself is deliberately changed; a
/// literal would have to be edited then, and an edited literal proves nothing.
///
/// Rows with a chip and rows without are both taken: the chip is the atom that cannot be
/// broken, and it is exactly where the second model of the wrap went wrong before this task —
/// the fitting stand of the контур called a two-line Polish row three lines long.
#[test]
fn the_measuring_twin_and_the_pen_put_a_help_row_on_the_same_number_of_lines() {
    use lang_switcher::theme::{
        ChipColors, ChipRowMetrics, ChipRowStyle, chip_row, measure_chip_row, paint_chip_row,
    };
    use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, LOGFONTW};

    let sheet = Sheet::new(320, false);

    let mut logical = LOGFONTW {
        lfHeight: -14,
        lfWeight: 400,
        ..Default::default()
    };

    for (slot, unit) in logical.lfFaceName.iter_mut().zip("Segoe UI".encode_utf16()) {
        *slot = unit;
    }

    let mut chip_logical = logical;
    chip_logical.lfHeight = -12;

    // SAFETY: both structures are live locals of this frame; the handles are freed at the end.
    let (body, chip) = unsafe {
        (
            CreateFontIndirectW(&raw const logical),
            CreateFontIndirectW(&raw const chip_logical),
        )
    };

    let pitch = 20;

    let metrics = ChipRowMetrics {
        body: Some(body),
        chip_face: Some((chip, chip_logical.lfHeight)),
        pitch,
        dpi: 96,
    };

    // SAFETY: plain colours in, handles out, both freed at the end.
    let (ground, fill) = unsafe {
        (
            CreateSolidBrush(COLORREF(0x00FF_FFFF)),
            CreateSolidBrush(COLORREF(0x00FF_FFFF)),
        )
    };

    // Five sentences that matter: one that wraps far, one that wraps once, one with no chip at
    // all, one whose chip lands mid-row, and one short enough for a single line.
    let sentences = [
        "Набрали слово не в той раскладке — нажмите {0}: слово перекодируется, раскладка переключится.",
        "Zaznacz tekst i naciśnij {0} — zaznaczenie zostanie przekonwertowane.",
        "Пауза, настройки и выход — в значке в трее.",
        "Повторное нажатие {0} возвращает всё назад.",
        "Готово.",
    ];

    for width in [140, 200, 260] {
        for sentence in sentences {
            for key in ["PrintScreen", "Pause", ""] {
                let row = chip_row(sentence, key);

                // SAFETY: the sheet's DC is live and both faces outlive the call.
                let (lines, needed) = unsafe { measure_chip_row(sheet.dc, width, row, metrics) }
                    .expect("the memory DC must answer the metrics of its own face");

                let area = RECT {
                    left: 4,
                    top: 4,
                    right: 4 + width,
                    bottom: sheet.side,
                };

                // SAFETY: own DC, own rectangle, live brushes and faces — the drawing half of
                // the pair, asked the same question.
                unsafe {
                    FillRect(sheet.dc, &area, ground);

                    paint_chip_row(
                        sheet.dc,
                        area,
                        row,
                        ChipRowStyle {
                            ground,
                            ink: COLORREF(0),
                            body: Some(body),
                            chip_face: Some((chip, chip_logical.lfHeight)),
                            chip: ChipColors {
                                outline: COLORREF(0),
                                fill,
                                ink: COLORREF(0),
                            },
                            pitch,
                            dpi: 96,
                        },
                    );
                }

                // Where the ink is. ⚠ Runs of inked rows are **not** the lines: a chip is a
                // figure taller than the text it holds, so it joins two lines into one run —
                // measured, and it is why this test counts **bands** instead. A band is the
                // `pitch` pixels a line of `draw_atoms` is placed on.
                let pixels = sheet.pixels();
                let inked = |y: i32| {
                    y >= 0
                        && y < sheet.side
                        && (area.left..area.right).any(|x| {
                            pixels[(y * sheet.side + x) as usize] & 0x00FF_FFFF != 0x00FF_FFFF
                        })
                };

                // Every band the twin promised carries ink…
                for band in 0..lines {
                    let from = area.top + band * pitch;

                    assert!(
                        (from..from + pitch).any(inked),
                        "width {width}, key «{key}»: the twin says {lines} lines, and line \
                         {band} — pixels {from}..{} — was never drawn. The two models of the \
                         wrap have parted. Sentence: «{sentence}»",
                        from + pitch
                    );
                }

                // …and nothing is drawn past the height it answered. This is the whole of what
                // `fit_about_help` relies on: a row laid out by `needed` is not clipped.
                let below = (area.top + needed..sheet.side).find(|y| inked(*y));

                assert!(
                    below.is_none(),
                    "width {width}, key «{key}»: the twin asked for {needed} px and the pen \
                     drew at {} px — a row laid out by that number would be clipped. Sentence: \
                     «{sentence}»",
                    below.unwrap_or(0) - area.top
                );
            }
        }
    }

    // SAFETY: every handle was made here, handed to nobody, and is freed exactly once.
    unsafe {
        let _ = DeleteObject(body.into());
        let _ = DeleteObject(chip.into());
        let _ = DeleteObject(ground.into());
        let _ = DeleteObject(fill.into());
    }
}

// =========================================================================================
// The straight tick in a mirrored window — вопрос 97 п. 1, task Т-30-3
// =========================================================================================

/// A 32-bit top-down surface with a white ground — the same shape `Supersample` draws into, so
/// that what is compared is what a window would get.
struct Sheet {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u32,
    side: i32,
}

impl Sheet {
    fn new(side: i32, mirrored: bool) -> Self {
        // SAFETY: a memory DC over the screen; deleted in `Drop`.
        let dc = unsafe { CreateCompatibleDC(None) };
        assert!(!dc.is_invalid(), "a memory DC is needed for this test");

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: side,
                biHeight: -side,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();

        // SAFETY: the description is live and the pointer takes the address of the section.
        let bitmap =
            unsafe { CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) }
                .expect("a DIB section is needed for this test");

        // SAFETY: the section was just made and this DC is its own.
        let previous = unsafe { SelectObject(dc, bitmap.into()) };

        let sheet = Self {
            dc,
            bitmap,
            previous,
            bits: bits.cast(),
            side,
        };

        // SAFETY: a plain colour in, a handle out, freed below.
        let brush = unsafe { CreateSolidBrush(COLORREF(0x00FF_FFFF)) };
        let whole = RECT {
            left: 0,
            top: 0,
            right: side,
            bottom: side,
        };
        // SAFETY: own DC, own rectangle, live brush.
        unsafe { FillRect(dc, &whole, brush) };
        // SAFETY: made above, handed to nobody, freed once.
        let _ = unsafe { DeleteObject(brush.into()) };

        if mirrored {
            // What `WS_EX_LAYOUTRTL` does to the context of a window, asked for directly.
            // SAFETY: own DC.
            unsafe { SetLayout(dc, LAYOUT_RTL) };
        }

        sheet
    }

    fn pixels(&self) -> Vec<u32> {
        let count = (self.side * self.side) as usize;
        // SAFETY: the section is live and holds exactly `side * side` words by construction.
        unsafe { std::slice::from_raw_parts(self.bits, count) }.to_vec()
    }

    fn inked(&self) -> usize {
        self.pixels()
            .iter()
            .filter(|&&point| (point & 0x00FF_FFFF) != 0x00FF_FFFF)
            .count()
    }
}

impl Drop for Sheet {
    fn drop(&mut self) {
        // SAFETY: what was in the DC goes back; the section and the DC are freed once each.
        unsafe { SelectObject(self.dc, self.previous) };
        let _ = unsafe { DeleteObject(self.bitmap.into()) };
        let _ = unsafe { DeleteDC(self.dc) };
    }
}

/// The tick of the settings dialog, in the size and the shape the product draws it.
const GLYPH_TICK: CheckMark = CheckMark {
    points_tenths: [(41, 88), (68, 115), (126, 57)],
    pen_tenths: 21,
};

/// **Т-30-3, решение 97.1: the tick drawn into a mirrored context is straight, not reversed.**
///
/// The same figure is drawn twice — once into an ordinary surface, once into a mirrored one —
/// and what is asked of the pair is the thing решение 97.1 decided: that the second is **not the
/// reflection of the first**. GDI mirrors strokes drawn into a mirrored context, measured before
/// any of this was written (`scratchpad-Э30\посылки-п3.log`) and seen by eye on the stand
/// (`scratchpad-Э30\ДО-Т-30-3-галочка-he.png`, where the long stroke ran down to the right), so
/// passing this is only possible if `check_mark_points` reflects the figure first.
///
/// # ⚠ Why this does not demand the two be identical
///
/// It was written that way and it failed, and the failure was **the test's** and not the
/// product's. Two things make an exact match unreachable, and both were measured rather than
/// argued: supersampled antialiasing of a stroke is not mirror-symmetric (a handful of edge
/// pixels differ in every size), and the tick of the cycle list has its stroke bounds clamped by
/// the edge of its own square, which no reflection can undo — that one lands a pixel across
/// (`scratchpad-Э30\посылки-п3.log`). Demanding equality would have meant either a test that can
/// only be satisfied by luck or a product bent to satisfy it.
///
/// So the two claims that **are** exact are made here: the ink is the same amount, and the
/// figure is not its own reflection. The rest of the acceptance of the drawn form is the stand's
/// screenshots and the user's eye, which is where решение 97.1 came from in the first place.
///
/// ⚠ The ink is counted before anything is compared. Two blank surfaces are neither equal nor
/// reflections of each other in any interesting way, and a test that skipped this would pass a
/// `draw_check_mark` that drew nothing at all.
#[test]
fn the_tick_drawn_in_a_mirrored_context_is_straight_and_not_reversed() {
    let side = 17;

    let plain = Sheet::new(side, false);
    let mirrored = Sheet::new(side, true);

    let glyph = RECT {
        left: 0,
        top: 0,
        right: side,
        bottom: side,
    };

    for sheet in [&plain, &mirrored] {
        draw_check_mark(sheet.dc, &glyph, COLORREF(0x0000_0000), GLYPH_TICK, 96);
    }

    assert!(
        plain.inked() > 0,
        "the instrument must be drawing something at all before its silence means anything"
    );
    assert_eq!(
        plain.inked(),
        mirrored.inked(),
        "the same figure carries the same amount of ink either way round"
    );

    // The reflection of the straight tick — what the window used to show, and what решение 97.1
    // says it must not show.
    let reflected: Vec<u32> = {
        let straight = plain.pixels();
        let mut out = vec![0_u32; straight.len()];

        for y in 0..side {
            for x in 0..side {
                out[(y * side + (side - 1 - x)) as usize] = straight[(y * side + x) as usize];
            }
        }

        out
    };

    assert_ne!(
        mirrored.pixels(),
        reflected,
        "решение 97.1: the tick carries a meaning and not a direction — a mirrored window must \
         not show the reversed figure"
    );

    // And it stands where the straight one stands: the columns the ink occupies are the same.
    // This is the claim the reflection formula is chosen to satisfy, and the one the cycle
    // list's clamped glyph misses by a pixel — so it is asserted for the dialog glyph, which is
    // the one the eye meets on every check box of the window.
    let columns = |points: &[u32]| {
        let mut first = side;
        let mut last = -1;

        for x in 0..side {
            let inked =
                (0..side).any(|y| (points[(y * side + x) as usize] & 0x00FF_FFFF) != 0x00FF_FFFF);

            if inked {
                first = first.min(x);
                last = last.max(x);
            }
        }

        (first, last)
    };

    assert_eq!(
        columns(&mirrored.pixels()),
        columns(&plain.pixels()),
        "and it occupies the same columns of its square"
    );
}

/// **Т-30-3: the pure geometry — the mirrored figure is the reflection of the straight one.**
///
/// The half of the rule that needs no window at all, and the half that says *what* reflection:
/// `width - x` about the square, which is the reflection that carries the **edges** of the tile
/// onto the edges of the straight one. The pixel-centre form `width - 1 - x` reads more natural
/// and was measured a pixel out (`scratchpad-Э30\посылки-п3-w-1.log`); `draw_check_mark` says
/// why at length. An off-by-one here would not look like a mirrored tick — it would look like a
/// tick one pixel out of its box, which is the harder kind of wrong to notice.
#[test]
fn the_mirrored_tick_is_the_reflection_of_the_straight_one() {
    let glyph = RECT {
        left: 0,
        top: 0,
        right: 17,
        bottom: 17,
    };
    let width = glyph.right - glyph.left;

    let straight = check_mark_points(&glyph, GLYPH_TICK, 96, false);
    let mirrored = check_mark_points(&glyph, GLYPH_TICK, 96, true);

    assert_ne!(
        straight, mirrored,
        "the tick of this program is not symmetrical, so the two must differ"
    );

    for (at, ((sx, sy), (mx, my))) in straight.iter().zip(&mirrored).enumerate() {
        assert_eq!(*my, *sy, "point {at} does not move vertically");
        assert_eq!(
            *mx,
            glyph.left + width - (sx - glyph.left),
            "point {at} is reflected about the square, by edges and not by pixel centres"
        );
    }

    // And the square is respected: the offset of the rectangle carries through, so a glyph that
    // does not start at zero is not reflected about the wrong axis.
    let moved = RECT {
        left: 100,
        top: 40,
        right: 117,
        bottom: 57,
    };

    let here = check_mark_points(&moved, GLYPH_TICK, 96, true);

    for (at, (x, y)) in here.iter().enumerate() {
        assert_eq!(*x, mirrored[at].0 + 100, "point {at} moved with the square");
        assert_eq!(*y, mirrored[at].1 + 40);
    }
}

/// **Т-30-3: the chevron of a combo box is symmetrical, so the mirror leaves it alone.**
///
/// The other figure this program strokes. It needs no reflection — and «needs none» is a claim
/// about its geometry, so it is checked rather than asserted in prose.
///
/// ⚠ **The claim is about the figure and not about where it sits.** The first edition of this
/// test asked whether the chevron stood at equal distances from the two edges of the area, and
/// was answered `27` and `6`: it does not, and it should not — it is inset against one edge, and
/// under the mirror that *position* moves to the other side, which is exactly what a
/// right-to-left combo box wants. What must not change is the **shape**, and the shape is
/// symmetrical when the two arms are of equal length. That is what is checked.
///
/// If somebody redraws the chevron as an arrow one day, this fails and решение 97.1 has to be
/// extended to it.
#[test]
fn the_combo_chevron_is_symmetrical_and_needs_no_mirror() {
    let area = RECT {
        left: 0,
        top: 0,
        right: 40,
        bottom: 24,
    };

    let points = combo_chevron_points(&area, 96);

    assert_eq!(
        points[1].0 - points[0].0,
        points[2].0 - points[1].0,
        "the two arms of the chevron are of equal length, so reflecting the figure about its \
         own axis is the identity: «{points:?}»"
    );
    assert_eq!(
        points[0].1, points[2].1,
        "and they start at the same height, so the figure has a vertical axis at all"
    );

    // Stated from the other side: reflecting the three points about the figure's own centre
    // gives the same three points back. That is the whole of «needs no mirror».
    let centre = points[1].0;
    let reflected = points.map(|(x, y)| (2 * centre - x, y));
    let mut sorted = reflected;
    sorted.sort_by_key(|(x, _)| *x);
    let mut original = points;
    original.sort_by_key(|(x, _)| *x);

    assert_eq!(
        sorted, original,
        "the chevron is its own reflection, so the mirror has nothing to undo"
    );
}

// =========================================================================================
// Reading order — вопрос 97 п. 2, task Т-30-2
// =========================================================================================

/// **Т-30-2: the reading-order bit is the four rows of the measurement and nothing else.**
///
/// The table this checks is not a preference — it is `scratchpad-Э30\посылки-п2.log`, measured
/// on the probe «a ב» whose three glyphs cannot change so that only their order can:
///
/// | context | flag | order measured |
/// |---|---|---|
/// | ordinary | none | left to right |
/// | ordinary | `DT_RTLREADING` | right to left |
/// | mirrored | none | right to left |
/// | mirrored | `DT_RTLREADING` | left to right |
///
/// So the flag asks for **the opposite of the context**, which is why it is set for an island
/// and only in a mirrored window: an island in an ordinary window already reads left to right
/// and asking again would turn it round.
#[test]
fn the_reading_order_bit_is_asked_for_only_by_an_island_in_a_mirrored_window() {
    let none = DRAW_TEXT_FORMAT(0);

    assert_eq!(
        reading_order(false, false),
        none,
        "ordinary window, native text: the context is already left to right"
    );
    assert_eq!(
        reading_order(false, true),
        none,
        "ordinary window, Latin island: also already left to right — asking would REVERSE it"
    );
    assert_eq!(
        reading_order(true, false),
        none,
        "mirrored window, native text: the context is already right to left"
    );
    assert_eq!(
        reading_order(true, true),
        DT_RTLREADING,
        "mirrored window, Latin island: the one case that needs the opposite of its context"
    );
}

/// **Т-30-2: a mirrored device context is recognised, and an ordinary one is not called one.**
///
/// The predicate is asked of the DC rather than of the interface locale, so it is checked on a
/// DC — one made mirrored by hand with `SetLayout`, which is what `WS_EX_LAYOUTRTL` does to the
/// context of a window. Both directions are asserted: a predicate that answered `true` always
/// would pass a test that only tried the mirrored one.
#[test]
fn a_mirrored_device_context_is_told_from_an_ordinary_one() {
    // SAFETY: a memory DC over the screen, deleted below on both paths.
    let dc = unsafe { CreateCompatibleDC(None) };

    assert!(
        !dc.is_invalid(),
        "a memory DC is needed for this measurement"
    );

    // SAFETY: `dc` is the live DC just made.
    assert!(
        !unsafe { dc_is_rtl(dc) },
        "a fresh memory DC is not mirrored"
    );

    // SAFETY: as above; `SetLayout` takes a plain value.
    unsafe { SetLayout(dc, LAYOUT_RTL) };

    // SAFETY: as above.
    assert!(
        unsafe { dc_is_rtl(dc) },
        "and it is once it has been told to be"
    );

    // And the format built from it carries the bit for an island and not for native text.
    // SAFETY: as above.
    let (native, island) = unsafe {
        (
            label_format(dc, Reading::Native),
            label_format(dc, Reading::LatinIsland),
        )
    };

    assert_eq!(
        native, LABEL_TEXT_FORMAT,
        "native text keeps the plain format"
    );
    assert_eq!(
        island,
        LABEL_TEXT_FORMAT | DT_RTLREADING,
        "an island adds the one bit and nothing else"
    );

    // SAFETY: the DC made above, deleted exactly once.
    let _ = unsafe { DeleteDC(dc) };
}
