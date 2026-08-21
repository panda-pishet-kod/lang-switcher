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
    Brushes, FOG, GRAPHITE, Palette, ThemeSetting, resolve, system_is_light,
};
use windows::Win32::Graphics::Gdi::{GetObjectW, HBRUSH, LOGBRUSH};

// =========================================================================================
// The palettes — criterion 11, the BGR trap
// =========================================================================================

/// The expected `COLORREF` of one written R,G,B triple, spelled out independently of the
/// module: `r | g<<8 | b<<16`, which is the `0x00BBGGRR` layout of the documentation.
const fn expected_colorref(r: u32, g: u32, b: u32) -> u32 {
    r | g << 8 | b << 16
}

/// Every field of a palette by name, in the order of the task's table.
fn fields_of(palette: &Palette) -> [(&'static str, u32); 16] {
    [
        ("window_bg", palette.window_bg.0),
        ("title_bg", palette.title_bg.0),
        ("panel_bg", palette.panel_bg.0),
        ("panel_border", palette.panel_border.0),
        ("text", palette.text.0),
        ("text_muted", palette.text_muted.0),
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
    ]
}

/// The R,G,B triples of the task's table, column «Графит», copied as written.
const GRAPHITE_TRIPLES: [(&str, (u32, u32, u32)); 16] = [
    ("window_bg", (32, 35, 41)),
    ("title_bg", (26, 29, 34)),
    ("panel_bg", (39, 43, 50)),
    ("panel_border", (54, 59, 67)),
    ("text", (228, 231, 234)),
    ("text_muted", (152, 160, 168)),
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
];

/// The R,G,B triples of the task's table, column «Туман», copied as written.
const FOG_TRIPLES: [(&str, (u32, u32, u32)); 16] = [
    ("window_bg", (237, 239, 242)),
    ("title_bg", (247, 248, 250)),
    ("panel_bg", (255, 255, 255)),
    ("panel_border", (225, 229, 234)),
    ("text", (35, 38, 43)),
    ("text_muted", (110, 118, 127)),
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
];

/// Checks one palette against one column of the task's table, field by field.
fn assert_palette_matches(
    palette_name: &str,
    palette: &Palette,
    triples: &[(&str, (u32, u32, u32)); 16],
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
