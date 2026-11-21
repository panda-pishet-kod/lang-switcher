//! The style table of the interface shell, `src\ui\tokens.rs` (stage Э120, T-120-2): the type size
//! of a window, the floor of 12, the contrast of the colour pairs the passport promises, and the
//! export to the mock-ups.

use lang_switcher::ui::tokens::{
    BUTTON_MIN_HEIGHT, Color, FIXED_SIZES, RADIUS_MEDIUM, Rgb, Role, TYPE_FLOOR, Theme, role_size,
    window_b,
};
use std::path::Path;
use std::process::Command;

/// The relative luminance of a colour, WCAG 2.x.
fn luminance(colour: Rgb) -> f64 {
    let linear = |channel: u8| {
        let c = f64::from(channel) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * linear(colour.r) + 0.7152 * linear(colour.g) + 0.0722 * linear(colour.b)
}

/// The contrast ratio of two colours, WCAG 2.x.
fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Every pair of `fronts` × `backs` in both themes whose contrast is below `least`.
fn below(fronts: &[Color], backs: &[Color], least: f64) -> Vec<String> {
    let mut found = Vec::new();

    for theme in [Theme::Light, Theme::Dark] {
        for front in fronts {
            for back in backs {
                let ratio = contrast(front.get(theme), back.get(theme));
                if ratio < least {
                    found.push(format!(
                        "{theme:?}: {} on {} is {ratio:.2}, below {least}",
                        front.name(),
                        back.name()
                    ));
                }
            }
        }
    }

    found
}

#[test]
fn the_type_size_of_a_window_meets_the_two_checkpoints_of_the_passport() {
    let small = window_b(911.0, 480.0, 1.0);
    let large = window_b(1280.0, 688.0, 1.0);

    assert!((small - 13.98).abs() <= 0.05, "B(911×480) = {small}");
    assert!((large - 15.9).abs() <= 0.05, "B(1280×688) = {large}");
}

#[test]
fn no_role_of_any_window_is_smaller_than_twelve() {
    let mut smallest = (f32::MAX, String::new());

    for w in (620..=1280).step_by(10) {
        for h in (500..=720).step_by(10) {
            for k in [0.80, 0.85, 0.90, 0.95, 1.0] {
                let (w, h) = (w as f32, h as f32);
                let b = window_b(w, h, k);

                for role in Role::ALL {
                    let size = role_size(role, b, h);
                    if size < smallest.0 {
                        smallest = (size, format!("{} at {w}×{h}, k = {k}", role.name()));
                    }
                }
            }
        }
    }

    assert!(
        smallest.0 >= 12.0,
        "{} is {} points",
        smallest.1,
        smallest.0
    );
    assert_eq!(TYPE_FLOOR.0, 12.0, "the floor of the passport is 12");
}

#[test]
fn the_fixed_sizes_of_the_corner_window_and_the_tray_menu_are_not_smaller_than_twelve() {
    let small: Vec<f32> = FIXED_SIZES
        .iter()
        .map(|size| size.0)
        .filter(|size| *size < 12.0)
        .collect();

    assert!(small.is_empty(), "{small:?}");
}

#[test]
fn text_second_text_and_accent_read_on_every_background() {
    let found = below(
        &[Color::Text, Color::Ink2, Color::Accent],
        &[Color::Bg, Color::Surface, Color::Surface2],
        4.5,
    );

    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_edge_of_what_can_be_pressed_stands_out_from_every_background() {
    let found = below(
        &[Color::Edge],
        &[Color::Bg, Color::Surface, Color::Surface2],
        3.0,
    );

    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_knob_of_a_switch_stands_out_from_the_track_on_and_off() {
    let found = below(&[Color::Knob], &[Color::Accent, Color::Edge], 3.0);

    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_text_of_the_main_button_reads_at_seven() {
    let found = below(&[Color::ButtonFg], &[Color::Button], 7.0);

    assert!(found.is_empty(), "{found:#?}");
}

/// `knob` is `bg` by reference (§1), not a number of its own: equal values written twice would
/// part the first time one of them is edited.
#[test]
fn the_knob_is_the_background_by_reference() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("ui")
            .join("tokens.rs"),
    )
    .expect("src\\ui\\tokens.rs must be readable");
    let arm = source
        .lines()
        .find(|line| line.trim_start().starts_with("Color::Knob => ") && !line.contains('"'))
        .expect("the arm of knob in Color::pair");

    assert_eq!(arm.trim(), "Color::Knob => Color::Bg.pair(),");
    assert_eq!(Color::Knob.pair(), Color::Bg.pair());
}

#[test]
fn every_colour_has_a_name_of_its_own() {
    let mut names: Vec<&str> = Color::ALL.iter().map(|colour| colour.name()).collect();
    names.sort_unstable();
    names.dedup();

    assert_eq!(names.len(), Color::ALL.len());
}

/// The block of `css` that `selector` opens, up to its closing brace.
fn block<'a>(css: &'a str, selector: &str) -> &'a str {
    let start = css
        .find(&format!("{selector} {{"))
        .unwrap_or_else(|| panic!("tokens.css has no block {selector}"));
    let rest = &css[start..];
    &rest[..rest.find('}').expect("the block closes")]
}

/// The values of `tokens.css` are the values of `tokens.rs`, byte for byte: three colours and two
/// sizes, printed by `tools\export-tokens.ps1` into a folder of the test's own.
#[test]
fn the_export_for_the_mock_ups_carries_the_values_of_the_table() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = std::env::temp_dir().join(format!("ls-export-tokens-{}", std::process::id()));
    let run = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(root.join("tools").join("export-tokens.ps1"))
        .arg("-OutDir")
        .arg(&out)
        .output()
        .expect("PowerShell starts");
    assert!(
        run.status.success(),
        "export-tokens.ps1 failed: {}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );

    let css = std::fs::read_to_string(out.join("tokens.css")).expect("tokens.css was written");
    let tables = std::fs::read_to_string(out.join("PASSPORT-tables.md"))
        .expect("PASSPORT-tables.md was written");
    let _ = std::fs::remove_dir_all(&out);

    let hex = |colour: Color, theme: Theme| format!("#{:06X}", colour.get(theme).hex());
    let light = block(&css, ":root, :root[data-theme=\"light\"]");
    let dark = block(&css, ":root[data-theme=\"dark\"]");
    let sizes = block(&css, "\n:root");

    for line in [
        (light, format!("--w-bg: {};", hex(Color::Bg, Theme::Light))),
        (
            dark,
            format!("--w-accent: {};", hex(Color::Accent, Theme::Dark)),
        ),
        (
            light,
            format!("--w-button-fg: {};", hex(Color::ButtonFg, Theme::Light)),
        ),
        (sizes, format!("--w-radius-medium: {}px;", RADIUS_MEDIUM.0)),
        (
            sizes,
            format!(
                "--w-button-min-height: calc({} * var(--w-b));",
                BUTTON_MIN_HEIGHT.0
            ),
        ),
    ]
    .iter()
    .map(|(block, line)| (*block, line.as_str()))
    {
        assert!(
            line.0.lines().any(|candidate| candidate.trim() == line.1),
            "tokens.css lacks `{}` in its block:\n{}",
            line.1,
            line.0
        );
    }

    let accent = format!(
        "| `accent` | `{}` | `{}` |",
        hex(Color::Accent, Theme::Light),
        hex(Color::Accent, Theme::Dark)
    );
    assert!(
        tables.contains(&accent),
        "PASSPORT-tables.md lacks {accent}"
    );
}
