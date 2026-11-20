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

/// **The list of «Исключения» takes what the section leaves** — in the least window too, where a
/// list of its natural six rows would push the two ways to add one under the rule of the buttons.
#[test]
fn the_list_of_exclusions_takes_what_the_section_leaves() {
    use lang_switcher::ui::layout::Part;

    with_product_strings();
    for language in [Language::Ru, Language::De, Language::Ml, Language::Ta] {
        let text = Text::new(language.tag(), Direction::LeftToRight).expect("DirectWrite");
        let (_, window) = window_of_settings::states(language)
            .into_iter()
            .find(|(name, _)| name == "settings Exclusions")
            .expect("the state of «Исключения»");
        let size = client_size(620.0, 500.0);
        let layout = fit::emergency(
            &window,
            size,
            fit::k_for(language.tag(), size.class),
            false,
            &text,
        );
        let rows = layout
            .items
            .iter()
            .filter(|item| matches!(item.part, Part::ListRow { .. }))
            .count();
        let rule = layout
            .items
            .iter()
            .find(|item| matches!(item.part, Part::ButtonRule))
            .map(|item| item.rect.y)
            .expect("the rule over the row of buttons");
        // The content is what starts above the rule — the buttons of the sections among it.
        let lowest = layout
            .items
            .iter()
            .filter(|item| item.rect.y < rule && !matches!(item.part, Part::Panel))
            .map(|item| item.rect.y + item.rect.h)
            .fold(0.0_f32, f32::max);
        println!(
            "{language:?}: rows {rows}, rule {rule}, lowest {lowest}, shortfall {:?}, k {}",
            layout.shortfall, layout.k
        );
        assert!(
            (1..=6).contains(&rows),
            "{language:?}: the list shows what the room lets it, {rows} rows"
        );
        // ⚠ Not `layout.shortfall`: the core measures it from the natural height, where a list in a
        // fill has its six rows (reported to the core, T-120-20).
        assert!(lowest <= rule, "{language:?}: nothing under the rule");
    }
}

/// The words of «Диагностика» from its block «Состояние» on: the block is the last of the section.
fn the_block_of_the_state(language: Language, state: &str) -> Option<String> {
    let (_, window) = window_of_settings::states(language)
        .into_iter()
        .find(|(name, _)| name == state)?;
    let words = format!("{:?}", window.body);
    let at = words.find("Status {")?;
    Some(words[at..].to_owned())
}

/// **T-120-21 — the red before.** The block «Состояние» speaks to a person until «Подробности для
/// автора» is pressed: the state, what is fixed, the key, the start with Windows — and not one word of
/// the developer (the registry and its path, the published exclusions).
#[test]
fn the_block_of_the_state_speaks_to_a_person_before_the_details() {
    with_product_strings();
    for (language, words) in [
        (Language::Ru, ["реестр", "hkcu", "опубликован", "\\"]),
        (Language::En, ["registry", "hkcu", "published", "\\"]),
    ] {
        let block = the_block_of_the_state(language, "settings Diagnostics")
            .unwrap_or_else(|| panic!("{language:?}: «Диагностика» has the block «Состояние»"));
        println!("{language:?}: {block}");
        let lower = block.to_lowercase();
        for word in words {
            assert!(
                !lower.contains(word),
                "{language:?}: «{word}» stands in the block before «Подробности для автора»"
            );
        }
        let details = the_block_of_the_state(language, "settings Diagnostics details")
            .expect("the block with the details open");
        assert!(
            details.to_lowercase().contains("hkcu"),
            "{language:?}: «Подробности для автора» opens the lines of the author"
        );
    }
}

/// **Three states, three first lines, three dots — the head of the menu's own** (T-120-21): one
/// source, `tray_menu::status`.
#[test]
fn the_three_states_of_the_program_are_the_menus_three() {
    use lang_switcher::ui::windows::tray_menu;

    with_product_strings();
    for (state, enabled, fail_safe) in [
        ("settings Diagnostics", true, false),
        ("settings Diagnostics paused", false, false),
        ("settings Diagnostics restart", false, true),
    ] {
        let (dot, words) = tray_menu::status(enabled, fail_safe);
        let expected = format!(
            "Status {{ state: {dot:?}, words: {:?} }}",
            settings::text_in(Language::Ru, words)
        );
        let block = the_block_of_the_state(Language::Ru, state).expect("the block");
        assert!(
            block.starts_with(&expected),
            "{state}: {expected} — the block begins {}",
            &block[..block.len().min(80)]
        );
    }
}

/// **«Подробности для автора» shows the four lines as they were, the path of the `Run` key on a line
/// of its own** (the mock-up's `.spath`), in the face of code.
#[test]
fn the_details_show_the_lines_of_the_author_with_the_path_apart() {
    with_product_strings();
    let block =
        the_block_of_the_state(Language::Ru, "settings Diagnostics details").expect("the block");
    let path = r"C:\Program Files\Lang_Switcher\LangSwitcher.exe";
    let code_lines = block.matches("Text(Code,").count();
    assert_eq!(code_lines, 5, "four lines and the path apart: {block}");
    assert!(
        block.contains(&format!("Text(Code, {path:?})")),
        "the path is a line of its own"
    );
    assert!(
        block.contains(&format!(
            "Text(Code, {:?})",
            settings::text_in(Language::Ru, settings::IDS_STATE_AUTOSTART).replace(
                "{0}",
                settings::text_in(Language::Ru, settings::IDS_AUTOSTART_PRESENT)
                    .replace("{0}", "")
                    .trim_end_matches([',', ' '])
            )
        )),
        "the line of the `Run` key keeps its words: {block}"
    );
}
