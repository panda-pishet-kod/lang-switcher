//! The window of the settings on the shell (T-120-20): its pictures for the control point C3.
//!
//! Printed as the product lays the window out — `fit::emergency` at the factor of the table of the
//! language — into the folder `LANGSW_UI_SETTINGS_DIR` names; without it the test prints nothing.
//! The drawing needs the `testing` feature, and so does the whole file.
#![cfg(feature = "testing")]

use lang_switcher::settings::{self, Language};
use lang_switcher::ui::fit;
use lang_switcher::ui::layout::{WindowSize, client_size, window_size};
use lang_switcher::ui::text::{Direction, Text};
use lang_switcher::ui::tokens::Theme;
use lang_switcher::ui::windows::settings as window_of_settings;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_AS_DATAFILE, LoadLibraryExW};
use windows::core::PCWSTR;

/// The languages of C3: two straight, the mirror, and English for the words of the note of a refused
/// write (C6 of the sheet).
const LANGUAGES: [Language; 4] = [Language::Ru, Language::De, Language::He, Language::En];

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

/// Prints every state of the window — five sections and «Несколько раскладок» — × light and dark ×
/// ru, de, he (the mirror) and en × the window of 2/3 of a 1920 × 1040 work area and the least one,
/// 620 × 500, with a list of the layouts beside them.
#[test]
fn print_the_pictures_of_the_window_of_the_settings() {
    use lang_switcher::ui::canvas::{Canvas, Mode};
    use lang_switcher::ui::elements::{self, Look, State};
    use lang_switcher::ui::tokens::Color;
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

    let Some(folder) = std::env::var_os("LANGSW_UI_SETTINGS_DIR") else {
        return;
    };
    // SAFETY: the apartment of this test's thread, for the whole test: `Art` keeps pictures past
    // each canvas in memory.
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let mut art = lang_switcher::ui::art::Art::new();
    let folder = Path::new(&folder);
    std::fs::create_dir_all(folder).expect("the folder of the pictures");
    with_product_strings();

    let sizes: [(&str, WindowSize); 2] = [
        ("two-thirds", window_size(1920.0, 1040.0)),
        ("least", client_size(620.0, 500.0)),
    ];
    let mut lines = Vec::new();
    for language in LANGUAGES {
        let rtl = language.is_rtl();
        let direction = if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let text = Text::new(language.tag(), direction).expect("DirectWrite");
        for (state, window) in window_of_settings::states(language) {
            for theme in [Theme::Light, Theme::Dark] {
                for (name, size) in sizes {
                    let k = fit::k_for(language.tag(), size.class);
                    let layout = fit::emergency(&window, size, k, rtl, &text);
                    let look = Look {
                        theme,
                        rtl,
                        b: layout.b,
                        h: size.h,
                        text: &text,
                    };
                    let mut canvas =
                        Canvas::in_memory(size.w, size.h, 1.0, Mode::Direct2D).expect("a canvas");
                    canvas.frame_with(Some(&mut art), |painter| {
                        painter.clear(Color::Bg.get(theme));
                        for item in &layout.items {
                            elements::draw(painter, &look, &item.part, item.rect, State::NORMAL);
                        }
                    });
                    let shade = if theme == Theme::Dark {
                        "dark"
                    } else {
                        "light"
                    };
                    let file = folder.join(format!(
                        "{}-{}-{shade}-{name}.png",
                        state.replace(' ', "-"),
                        language.tag()
                    ));
                    canvas.save_png(&file).expect("the picture is written");
                    lines.push(format!(
                        "{state}|{}|{shade}|{name}|{:.0}x{:.0}|B {:.2}|k {:.3}|fits {}",
                        language.tag(),
                        size.w,
                        size.h,
                        layout.b,
                        layout.k,
                        layout.shortfall.fits()
                    ));
                }
            }
        }
    }
    std::fs::write(folder.join("settings-layouts.txt"), lines.join("\r\n")).expect("the list");
}
