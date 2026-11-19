//! The probe of the interface shell (stage Э120, T-120-9): «О программе» on the shell, laid out and
//! drawn in memory with the program's own strings — read from the product binary as a data file,
//! as `tests\letters.rs` does — in the languages of the probe.
//!
//! Sinhala is not a language of the program (`res\lang\languages.toml`): its pictures are not
//! taken, and the report says so.

use lang_switcher::settings::{self, Language};
use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::fit;
use lang_switcher::ui::host::Content;
use lang_switcher::ui::layout::{Layout, WindowSize, client_size};
use lang_switcher::ui::text::{Direction, Text};
use lang_switcher::ui::tokens::{TYPE_FLOOR, Theme};
use lang_switcher::ui::windows::about::{About, Id};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW};
use windows::core::PCWSTR;

/// The languages of the probe the program has.
const LANGUAGES: [Language; 6] = [
    Language::Ru,
    Language::En,
    Language::He,
    Language::Ar,
    Language::Hi,
    Language::Ta,
];

/// The strings of the product binary, built beside the tests.
fn with_product_strings() {
    static PRODUCT: OnceLock<usize> = OnceLock::new();
    let raw = *PRODUCT.get_or_init(|| {
        let exe = std::env::current_exe()
            .expect("the test executable has a path")
            .parent()
            .and_then(Path::parent)
            .expect("the test executable lives in <target>\\debug\\deps")
            .join("LangSwitcher.exe");
        let path: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: a NUL-terminated path owned by this frame; mapped for its resources only.
        let module =
            unsafe { LoadLibraryExW(PCWSTR(path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
                .expect("the product binary opens as a data file");
        module.0 as usize
    });
    settings::set_resource_module(HMODULE(std::ptr::without_provenance_mut(raw)));
}

fn about(language: Language, theme: Theme) -> About {
    let theme = if theme == Theme::Dark {
        ThemeSetting::Dark
    } else {
        ThemeSetting::Light
    };
    About {
        language,
        key: "Pause".to_owned(),
        theme,
    }
}

fn text_for(about: &About) -> (Text, bool) {
    let (locale, rtl) = about.language();
    let direction = if rtl {
        Direction::RightToLeft
    } else {
        Direction::LeftToRight
    };
    (Text::new(&locale, direction).expect("DirectWrite"), rtl)
}

fn lay(about: &About, text: &Text, rtl: bool, size: WindowSize) -> Layout<Id> {
    fit::emergency(&about.describe(), size, 1.0, rtl, text)
}

/// The least window, 620 × 500 points: every language fits without scrolling, no type under 12.
#[test]
fn about_fits_the_least_window_in_every_language_of_the_probe_at_twelve_and_up() {
    with_product_strings();
    let least = client_size(620.0, 500.0);

    for language in LANGUAGES {
        let about = about(language, Theme::Light);
        assert!(
            !about.title().is_empty(),
            "{language:?}: the strings of the product are read"
        );
        let (text, rtl) = text_for(&about);
        let layout = lay(&about, &text, rtl, least);

        assert!(
            layout.shortfall.fits(),
            "{language:?}: {:?} does not fit at B {}",
            layout.shortfall,
            layout.b
        );
        assert!(layout.b >= TYPE_FLOOR.0, "{language:?}: B {}", layout.b);
    }
}

/// Prints the pictures of the probe into `LANGSW_UI_PROBE_DIR`: the six languages × two themes ×
/// 100, 125, 150 and 200 % on a 1920 × 1040 work area, and the least window 620 × 500.
#[cfg(feature = "testing")]
#[test]
fn print_the_pictures_of_the_probe() {
    use lang_switcher::ui::canvas::{Canvas, Mode};
    use lang_switcher::ui::elements::{self, Look, State};
    use lang_switcher::ui::layout::window_size;
    use lang_switcher::ui::tokens::Color;

    let Some(folder) = std::env::var_os("LANGSW_UI_PROBE_DIR") else {
        return;
    };
    let folder = Path::new(&folder);
    std::fs::create_dir_all(folder).expect("the folder of the pictures");
    with_product_strings();

    let mut lines = Vec::new();
    for language in LANGUAGES {
        for theme in [Theme::Light, Theme::Dark] {
            let about = about(language, theme);
            let (text, rtl) = text_for(&about);
            let mut sizes: Vec<(String, f32, WindowSize)> = [1.0_f32, 1.25, 1.5, 2.0]
                .iter()
                .map(|scale| {
                    (
                        format!("{}", (scale * 100.0) as u32),
                        *scale,
                        window_size(1920.0 / scale, 1040.0 / scale),
                    )
                })
                .collect();
            sizes.push(("least".to_owned(), 1.0, client_size(620.0, 500.0)));

            for (name, scale, size) in sizes {
                let layout = lay(&about, &text, rtl, size);
                let look = Look {
                    theme,
                    rtl,
                    b: layout.b,
                    h: size.h,
                    text: &text,
                };
                let mut canvas =
                    Canvas::in_memory(size.w, size.h, scale, Mode::Direct2D).expect("a canvas");
                canvas.frame(|painter| {
                    painter.clear(Color::Bg.get(theme));
                    for item in &layout.items {
                        elements::draw(painter, &look, &item.part, item.rect, State::NORMAL);
                    }
                });
                let file = folder.join(format!(
                    "about-{}-{}-{name}.png",
                    language.tag(),
                    if theme == Theme::Dark {
                        "dark"
                    } else {
                        "light"
                    }
                ));
                canvas.save_png(&file).expect("the picture is written");
                lines.push(format!(
                    "{}|{}|{name}|{:.0}x{:.0}|B {:.2}|k {:.3}|fits {}",
                    language.tag(),
                    if theme == Theme::Dark {
                        "dark"
                    } else {
                        "light"
                    },
                    size.w,
                    size.h,
                    layout.b,
                    layout.k,
                    layout.shortfall.fits()
                ));
            }
        }
    }
    std::fs::write(folder.join("probe-layouts.txt"), lines.join("\r\n")).expect("the list");
}
