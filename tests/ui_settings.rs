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
        for (state, window) in lang_switcher::ui::windows::catalogue(language)
            .into_iter()
            .filter(|(name, _)| name.starts_with("settings"))
        {
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
        let details = format!(
            "{:?}",
            window_of_settings::details_states(language)[0].1.body
        );
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
    let block = format!(
        "{:?}",
        window_of_settings::details_states(Language::Ru)[0].1.body
    );
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

/// **«Копировать» puts the four lines on the clipboard word for word; «Закрыть» closes; the window
/// opens at the least size** (T-120-21 Б, C6 (а)).
#[test]
fn the_window_of_the_details_copies_the_lines_and_closes() {
    use lang_switcher::ui::host::{Content, Reply};
    use lang_switcher::ui::scene::Event;
    use lang_switcher::ui::windows::settings::{Details, DetailsId, Program};
    use std::cell::RefCell;
    use std::rc::Rc;

    with_product_strings();
    let program = Program {
        enabled: true,
        fail_safe: false,
        details: vec![
            "line one".to_owned(),
            "line two".to_owned(),
            r"Autostart: yes, C:\Program Files\Lang_Switcher\LangSwitcher.exe".to_owned(),
            "line four".to_owned(),
        ],
        path: Some(r"C:\Program Files\Lang_Switcher\LangSwitcher.exe".to_owned()),
    };
    let copied = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&copied);
    let mut details = Details::new(
        Language::Ru,
        lang_switcher::theme::ThemeSetting::Light,
        program.clone(),
        Box::new(move |words: &str| {
            log.borrow_mut().push(words.to_owned());
            true
        }),
    );

    let stays = details.event(Event::Pressed(DetailsId::Copy));
    let closes = details.event(Event::Pressed(DetailsId::Close));

    assert_eq!(stays, Reply::Stay, "«Копировать» leaves the window open");
    assert_eq!(
        copied.borrow().as_slice(),
        [program.details.join("\r\n")],
        "and puts the four lines on the clipboard word for word"
    );
    assert_eq!(closes, Reply::Close, "«Закрыть» closes");
    assert!(
        details.least(),
        "the window opens at the least size of the passport"
    );
    assert_eq!(
        details.shown_lines()[2..4],
        [
            "Autostart: yes".to_owned(),
            r"C:\Program Files\Lang_Switcher\LangSwitcher.exe".to_owned()
        ],
        "the path of the `Run` key on a line of its own"
    );
}

/// **«Подробности для автора» opens a window of the shell owned by the window of the settings.**
#[test]
fn the_details_open_a_window_owned_by_the_settings() {
    use lang_switcher::ui::windows::settings::{Program, open_details, testing};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, GW_OWNER, GetWindow, WINDOW_EX_STYLE, WS_POPUP,
    };
    use windows::core::w;

    with_product_strings();
    // SAFETY: a hidden window of this thread stands in for the window of the settings.
    let owner = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("owner"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        )
    }
    .expect("a window of this thread");
    let details = testing::hidden_while(|| {
        open_details(
            owner,
            Language::Ru,
            lang_switcher::theme::ThemeSetting::Light,
            Program::default(),
        )
    });
    // SAFETY: windows of this thread.
    let owned_by = details.map(|window| unsafe { GetWindow(window, GW_OWNER) });
    if let Some(window) = details {
        // SAFETY: a window of this thread.
        let _ = unsafe { DestroyWindow(window) };
    }
    // SAFETY: the window made above.
    let _ = unsafe { DestroyWindow(owner) };

    assert!(details.is_some(), "the window of the details opens");
    assert_eq!(
        owned_by.and_then(Result::ok),
        Some(owner),
        "owned by the window of the settings"
    );
}

/// **The C3 of T-120-20 (решение 208.160): every section shows its subtitle under its title, and the
/// window its own eyebrow** — `IDS_SETTINGS_EYEBROW`, not the word of the wizard it borrowed before.
/// English and German tell the two keys apart («Settings» against «The settings», «Einstellungen»);
/// in Russian both say «Настройки».
#[test]
fn every_section_shows_its_subtitle_and_the_eyebrow_of_its_own() {
    with_product_strings();
    let sections = [
        ("settings General", settings::IDS_SECTION_GENERAL_SUB),
        ("settings Hotkey", settings::IDS_SECTION_HOTKEY_SUB),
        ("settings Layouts", settings::IDS_SECTION_LAYOUTS_SUB),
        ("settings Exclusions", settings::IDS_SECTION_EXCLUSIONS_SUB),
        (
            "settings Diagnostics",
            settings::IDS_SECTION_DIAGNOSTICS_SUB,
        ),
    ];
    // French, not German: after the wave of translations both German words are «Einstellungen», and
    // the check could not tell the two keys apart.
    for language in [Language::Ru, Language::En, Language::Fr] {
        let eyebrow = settings::text_in(language, settings::IDS_SETTINGS_EYEBROW);
        if language != Language::Ru {
            // ⚠ Контроль: where the two keys say the same, the check below proves nothing.
            assert_ne!(
                eyebrow,
                settings::text_in(language, settings::IDS_WIZARD_ATTACH_SETTINGS),
                "{language:?}: the eyebrow and the word of the wizard must differ for this check"
            );
        }
        let states = window_of_settings::states(language);
        for (state, subtitle) in sections {
            let (_, window) = states
                .iter()
                .find(|(name, _)| name == state)
                .unwrap_or_else(|| panic!("{state}"));
            let words = format!("{:?}", window.body);
            let subtitle = settings::text_in(language, subtitle);
            assert!(
                words.contains(&format!("eyebrow: Some({eyebrow:?})")),
                "{language:?} {state}: the eyebrow of its own, «{eyebrow}»"
            );
            assert!(
                words.contains(&format!("subtitle: Some({subtitle:?})")),
                "{language:?} {state}: the subtitle under the title, «{subtitle}»"
            );
        }
    }
}

/// **T-120-22 Б: «Нашли ошибку в переводе?» stands under «Язык интерфейса» in every language but
/// Russian** — the language the others are translated from. A quiet button, the very next element
/// after the choice of the language and before the theme.
#[test]
fn the_translation_button_stands_under_the_language_in_every_language_but_russian() {
    with_product_strings();
    for language in LANGUAGES {
        let states = window_of_settings::states(language);
        let (_, window) = states
            .iter()
            .find(|(name, _)| name == "settings General")
            .expect("the window has «Общие»");
        let body = format!("{:?}", window.body);
        let choice = body.find("id: Language").expect("«Язык интерфейса»");
        let theme = body.find("id: Theme").expect("«Тема»");
        let button = body.find("id: Translation");
        println!("{language:?}: language at {choice}, button at {button:?}, theme at {theme}");

        if language == Language::Ru {
            assert_eq!(button, None, "Russian has no «Нашли ошибку в переводе?»");
            continue;
        }
        let at = button.unwrap_or_else(|| panic!("{language:?}: the button is there"));
        let between = &body[at..theme];
        let label = format!(
            "{:?}",
            settings::text_in(language, settings::IDS_TRANSLATION_REPORT)
        );
        assert!(
            choice < at && between.contains("kind: Quiet") && between.contains(&label),
            "{language:?}: the quiet {label} under the language and above the theme: {between}"
        );
    }
}

/// **T-120-22 Б, C6 of the sheet, (г): the note of a refused write takes the place of «Нашли ошибку
/// в переводе?»** — with the note over «Общие», the button is not there; without it, it is. The note
/// is temporary, and the button comes back with the next change.
#[test]
fn the_note_of_a_refused_write_takes_the_place_of_the_translation_button() {
    with_product_strings();
    let states = window_of_settings::states(Language::De);
    let body = |name: &str| {
        let (_, window) = states
            .iter()
            .find(|(state, _)| state == name)
            .unwrap_or_else(|| panic!("the window has the state {name}"));
        format!("{:?}", window.body)
    };
    let (plain, refused) = (body("settings General"), body("settings General refused"));
    let note = format!(
        "{:?}",
        settings::text_in(Language::De, settings::IDS_SETTINGS_NOT_SAVED)
    );

    assert!(
        refused.contains(&note) && !plain.contains(&note),
        "the control: the note stands in the one state and not in the other"
    );
    assert!(
        plain.contains("id: Translation"),
        "without the note, the button is there"
    );
    assert!(
        !refused.contains("id: Translation"),
        "with the note, the button is not"
    );
}

/// **208.209, AUDIT-180 Н3, Н4, Н10 — the switches in a group of rows, with their hints.** In
/// «Общие» the five switches stand in one group of rows (passport §3), the flag with its hint under
/// its label; in «Диагностика» «Вести журнал» stands in a group of its own with the hint of what the
/// journal holds.
#[test]
fn the_switches_of_the_settings_stand_in_groups_of_rows_with_their_hints() {
    with_product_strings();
    let states = window_of_settings::states(Language::Ru);
    let body = |name: &str| {
        let (_, window) = states
            .iter()
            .find(|(state, _)| state == name)
            .unwrap_or_else(|| panic!("the window has the state {name}"));
        format!("{:?}", window.body)
    };
    let quoted = |id| format!("{:?}", settings::text_in(Language::Ru, id));

    let general = body("settings General");
    let group = general.find("Group([").expect("«Общие»: a group of rows");
    let end = general
        .find("Pair(")
        .expect("«Общие»: the pair of the language and the theme");
    let rows = &general[group..end];
    for id in [
        "id: Autostart",
        "id: Sound",
        "id: Selection",
        "id: TrayFlag",
        "id: Updates",
    ] {
        assert!(rows.contains(id), "«Общие»: {id} in the group: {rows}");
    }
    assert!(
        rows.contains(&format!(
            "hint: Some({})",
            quoted(settings::IDS_TRAY_FLAG_HINT)
        )),
        "«Общие»: the hint of the flag: {rows}"
    );

    let diagnostics = body("settings Diagnostics");
    let at = diagnostics
        .find("Group([Toggle { id: Log")
        .expect("«Диагностика»: «Вести журнал» in a group of its own");
    assert!(
        diagnostics[at..].contains(&format!("hint: Some({})", quoted(settings::IDS_LOG_HINT))),
        "«Диагностика»: the hint of the journal"
    );
}
