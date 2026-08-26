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
    Brushes, FOG, GRAPHITE, PaintBuffer, Palette, ThemeSetting, resolve, system_is_light,
};
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject,
    FillRect, GetCurrentObject, GetDC, GetObjectW, GetPixel, GetStockObject, HBITMAP, HBRUSH, HDC,
    HGDIOBJ, LOGBRUSH, OBJ_FONT, OEM_FIXED_FONT, ReleaseDC, SRCCOPY, SYSTEM_FONT, SelectObject,
    SetPixel,
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
