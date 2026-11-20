//! Integration tests for the `theme` module, task T-11-1.
//!
//! The palette checks recompute every expected `COLORREF` from the R,G,B triples of the
//! task with an expression written in this file — never by reading the module's constants
//! or its packing helper back. `COLORREF` is `0x00BBGGRR`, the reverse of the written
//! order, and a module that packed the bytes the written way round would not fail to build
//! or to run: it would silently paint everything in someone else's colours. Independent
//! recomputation is the only test that catches it.
//!
//! [`system_is_light`] and, since task T-86-2, [`taskbar_is_light`] are the two
//! machine-dependent functions here. The first is tested for its contract alone: it answers
//! without panicking, and its value mirrors this machine's `AppsUseLightTheme` and is
//! deliberately not compared with anything. The second is held to a second reading of the
//! same value by another road of the registry API, and to a sweep of its source.

use lang_switcher::theme::{
    CheckMark, FOG, GRAPHITE, Palette, ThemeSetting, check_mark_points, draw_check_mark, resolve,
    system_is_light, taskbar_is_light,
};
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, CreateSolidBrush,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, FillRect, HBITMAP, HDC, HGDIOBJ, LAYOUT_RTL,
    SelectObject, SetLayout,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, REG_DWORD, REG_VALUE_TYPE, RegCloseKey, RegOpenKeyExW,
    RegQueryValueExW,
};
use windows::core::w;

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
// The taskbar switch — FR-90, task T-86-2
// =========================================================================================

/// **Task T-86-2, FR-90: the taskbar switch, read by the product and read again by another
/// road.**
///
/// The product's [`taskbar_is_light`] is one `RegGetValueW` of `SystemUsesLightTheme`. This
/// test reads the same value through `RegOpenKeyExW` + `RegQueryValueExW` — a second reader
/// that shares no line with the first — and applies the rule of FR-92а by hand: a readable
/// `REG_DWORD` zero is dark, anything else is light. The two must agree on this machine,
/// whatever it says; the value itself is machine-dependent and is not pinned.
///
/// ⚠ What this cannot catch, said plainly: a product that read `AppsUseLightTheme` instead
/// would agree with this reader on every machine where the two switches move together —
/// which is what «Выберите режим» does, and what the owner's machine shows (0 and 0). The
/// name of the value is therefore held separately, by the sweep below.
#[test]
fn taskbar_is_light_agrees_with_a_second_reading_of_system_uses_light_theme() {
    let answer = taskbar_is_light();

    let mut key = HKEY::default();

    // SAFETY: the subkey is a NUL-terminated `'static` literal; `key` is a live local the
    // call writes once, and it is closed below on the same thread.
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            None,
            KEY_READ,
            &raw mut key,
        )
    };

    let second = if opened.is_ok() {
        let mut kind = REG_VALUE_TYPE::default();
        let mut data = [0u8; 4];
        let mut size = u32::try_from(data.len()).expect("four fits");

        // SAFETY: `key` was opened a line above and is closed right after; every out-pointer
        // names a live local of at least the size `size` promises.
        let read = unsafe {
            RegQueryValueExW(
                key,
                w!("SystemUsesLightTheme"),
                None,
                Some(&raw mut kind),
                Some(data.as_mut_ptr()),
                Some(&raw mut size),
            )
        };

        // SAFETY: closing the key opened above, exactly once.
        let _ = unsafe { RegCloseKey(key) };

        if read.is_ok() && kind == REG_DWORD && size == 4 {
            let value = u32::from_le_bytes(data);
            println!("SystemUsesLightTheme = {value} (REG_DWORD)");
            value != 0
        } else {
            println!(
                "SystemUsesLightTheme: absent or not a DWORD ({read:?}, {kind:?}, {size} bytes) \
                 — light by the rule of FR-92а"
            );
            true
        }
    } else {
        println!("Personalize key: {opened:?} — light by the rule of FR-92а");
        true
    };

    println!("taskbar_is_light() = {answer}; the second reading says light = {second}");

    assert_eq!(
        answer, second,
        "the product and the second reader must agree on this machine"
    );
}

/// **Task T-86-2, guard (4), the `theme.rs` half: `SystemUsesLightTheme` is read in one place,
/// and the module writes nothing into the registry.**
///
/// Swept over `src\theme.rs` as a whole — the file keeps no unit-test module at its foot, so
/// there is no suite text to cut away. Three claims: the value name is spelled exactly once
/// (the `w!` literal behind the constant); `RegGetValueW` is called exactly once (the one
/// body behind both switches); and no writing call of the registry API is named at all.
///
/// The negative control is measured, not assumed: the first claim over the base `bc7c456`
/// answers 0 and the sweep fails there — `scratchpad-E86\sweep-theme-negative-control-T-86-2.log`.
#[test]
fn the_taskbar_switch_is_read_once_and_theme_never_writes_the_registry() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("theme.rs"),
    )
    .expect("src\\theme.rs must be readable")
    .replace("\r\n", "\n");

    let count = |needle: &str| source.matches(needle).count();

    let name = count("\"SystemUsesLightTheme\"");
    let reads = count("RegGetValueW(");

    println!("«\"SystemUsesLightTheme\"»: {name}; «RegGetValueW(»: {reads}");

    assert_eq!(
        name, 1,
        "the value name is spelled in one place — the constant behind taskbar_is_light"
    );
    assert_eq!(
        reads, 1,
        "one RegGetValueW — the one body behind both switches"
    );

    for writer in [
        "RegSetValue",
        "RegSetKeyValue",
        "RegCreateKey",
        "RegDeleteValue",
        "RegDeleteKey",
        "RegDeleteTree",
    ] {
        let found = count(writer);

        println!("«{writer}»: {found}");

        assert_eq!(
            found, 0,
            "theme.rs never writes the registry — «{writer}» must not be named"
        );
    }
}

// =========================================================================================
// The brushes — criterion 14, creation and a single release
// =========================================================================================

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

// =========================================================================================
// Чип-«клавиша»: чистая арифметика коробки и разбор строки — task Т-26-2, решение 87
// =========================================================================================

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

// =========================================================================================
// Reading order — вопрос 97 п. 2, task Т-30-2
// =========================================================================================
