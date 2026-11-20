//! The window in the corner on the shell (stage Э120, T-120-14 пп. 2 и 4): its description from
//! the words of `corner::Content` and its layout — the sizes of the passport, the buttons in
//! shares 1 : 2 or one under another, the mirror for Hebrew.

use lang_switcher::corner::{Action, Content};
use lang_switcher::ui::canvas::Rect;
use lang_switcher::ui::layout::{Layout, Part, client_size, lay_out};
use lang_switcher::ui::text::{FixedAdvance, Style};
use lang_switcher::ui::tokens::{CORNER_PAD, CORNER_TEXT, CORNER_TITLE, CORNER_WIDTH, Role};
use lang_switcher::ui::windows::corner::{Id, describe};

const MEASURE: FixedAdvance = FixedAdvance(0.5);

fn content(main: &str, second: &str) -> Content {
    Content {
        about: "Обновлено".to_owned(),
        title: "Lang Switcher обновлён до 1.8.0".to_owned(),
        text: "Флаг раскладки теперь у часов.".to_owned(),
        link: None,
        main: (main.to_owned(), Action::Close),
        second: Some((second.to_owned(), Action::Close)),
        picture: false,
        check: Some("Больше не показывать окна обновлений".to_owned()),
    }
}

/// The window laid out at its width and the height it needs.
fn laid(content: &Content, rtl: bool) -> Layout<Id> {
    let window = describe(content, false, None);
    let first = lay_out(
        &window,
        client_size(CORNER_WIDTH.0, 400.0),
        1.0,
        rtl,
        &MEASURE,
    );
    let h = 400.0 + first.shortfall.h;
    lay_out(&window, client_size(CORNER_WIDTH.0, h), 1.0, rtl, &MEASURE)
}

fn rect_of(layout: &Layout<Id>, id: Id) -> Rect {
    layout
        .items
        .iter()
        .find(|item| item.id == Some(id) && matches!(item.part, Part::Button { .. }))
        .map(|item| item.rect)
        .unwrap_or_else(|| panic!("no button {id:?}"))
}

fn first_of(layout: &Layout<Id>, wanted: fn(&Part) -> bool) -> Rect {
    layout
        .items
        .iter()
        .find(|item| wanted(&item.part))
        .map(|item| item.rect)
        .expect("the part")
}

/// The red one before the task: on the view of E119 the window has no mirror. For Hebrew the main
/// button stands at the left, the second at the right; the icon of the header at the right and the
/// ✕ at the left.
#[test]
fn in_hebrew_the_main_button_stands_at_the_left() {
    let layout = laid(&content("Что нового", "Закрыть"), true);
    let (main, second) = (rect_of(&layout, Id::Main), rect_of(&layout, Id::Second));
    assert!(
        main.right() <= second.x,
        "the main button {main:?} is not left of the second {second:?}"
    );
    let icon = first_of(&layout, |part| matches!(part, Part::AppIcon));
    let cross = first_of(&layout, |part| matches!(part, Part::Cross));
    assert!(
        cross.right() <= icon.x,
        "the ✕ is not at the left in Hebrew"
    );
}

/// The main button on the right, twice the second; the parts inside the paddings of the passport.
#[test]
fn the_main_button_is_at_the_end_and_takes_two_shares() {
    let layout = laid(&content("Что нового", "Закрыть"), false);
    let (main, second) = (rect_of(&layout, Id::Main), rect_of(&layout, Id::Second));
    assert!(second.right() <= main.x, "the second is not at the start");
    assert!(
        (main.w - second.w * 2.0).abs() < 1.0,
        "the shares are not 1 : 2: {second:?} {main:?}"
    );
    assert!((second.y - main.y).abs() < 0.5, "not one row");
    let left = CORNER_PAD.left.0;
    let right = CORNER_WIDTH.0 - CORNER_PAD.right.0;
    for item in &layout.items {
        assert!(
            item.rect.x >= left - 0.5 && item.rect.right() <= right + 0.5,
            "{} {:?} outside the paddings",
            item.part.name(),
            item.rect
        );
    }
}

/// Labels that do not fit their shares: one under another, the main one first, at the full width.
#[test]
fn long_labels_put_the_buttons_one_under_another_main_first() {
    let layout = laid(
        &content(
            "Was ist neu in dieser Version",
            "Schließen und nicht mehr fragen",
        ),
        false,
    );
    let (main, second) = (rect_of(&layout, Id::Main), rect_of(&layout, Id::Second));
    assert!(
        main.bottom() <= second.y,
        "the main is not above the second"
    );
    assert!((main.w - second.w).abs() < 0.5, "not the same width");
}

/// The sizes of the window in the corner are the passport's whatever the screen: the title 15,
/// the text 13,5.
#[test]
fn the_window_in_the_corner_keeps_its_type_sizes() {
    for b in [13.0, 15.9] {
        assert_eq!(Style::of(Role::CornerTitle, b, 500.0).size, CORNER_TITLE.0);
        assert_eq!(Style::of(Role::CornerText, b, 500.0).size, CORNER_TEXT.0);
    }
    let window = describe(&content("A", "B"), true, None);
    let check = window_has_check(&window);
    assert!(check, "the check box of the content is not in the window");
}

fn window_has_check(window: &lang_switcher::ui::scene::Window<Id>) -> bool {
    match &window.body {
        lang_switcher::ui::scene::Node::Column(parts) => parts.iter().any(|part| {
            matches!(
                part,
                lang_switcher::ui::scene::Node::Check {
                    id: Id::Check,
                    on: true,
                    ..
                }
            )
        }),
        _ => false,
    }
}

/// Prints the windows in the corner into `LANGSW_UI_CORNER_DIR` for C3: three contents — the hint
/// of the flag, «Обновлено», the notice of a letter — in Russian, German (long buttons) and Hebrew
/// (mirrored), both themes, 100 and 150 %.
#[cfg(feature = "testing")]
#[test]
fn print_the_windows_in_the_corner() {
    use lang_switcher::ui::canvas::{Canvas, Mode};
    use lang_switcher::ui::elements::{self, Look, State};
    use lang_switcher::ui::text::{Direction, Text};
    use lang_switcher::ui::tokens::{Color, Theme};
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

    let Some(folder) = std::env::var_os("LANGSW_UI_CORNER_DIR") else {
        return;
    };
    // SAFETY: the apartment of this test's thread, for the whole test (`Art` outlives each canvas).
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    let mut art = lang_switcher::ui::art::Art::new();
    let folder = std::path::Path::new(&folder);
    std::fs::create_dir_all(folder).expect("the folder of the pictures");

    let mut updated_ru = content("Что нового", "Закрыть");
    updated_ru.picture = true;
    let windows: [(&str, &str, Content); 9] = [
        (
            "hint",
            "ru",
            hint("Флаг раскладки — у часов", "Показать", "Понятно"),
        ),
        ("updated", "ru", updated_ru),
        (
            "letter",
            "ru",
            letter("Письмо от автора", "Прочитать", "Позже"),
        ),
        (
            "hint",
            "de",
            hint("Die Flagge steht jetzt bei der Uhr", "Zeigen", "Verstanden"),
        ),
        (
            "updated",
            "de",
            content("Was ist neu in dieser Version", "Schließen"),
        ),
        (
            "letter",
            "de",
            letter("Ein Brief vom Autor", "Jetzt lesen", "Später erinnern"),
        ),
        ("hint", "he", hint("הדגל עכשיו ליד השעון", "הצג", "הבנתי")),
        ("updated", "he", with_picture(content("מה חדש", "סגור"))),
        ("letter", "he", letter("מכתב מהמחבר", "קרא", "אחר כך")),
    ];
    for (name, language, content) in &windows {
        let rtl = *language == "he";
        let direction = if rtl {
            Direction::RightToLeft
        } else {
            Direction::LeftToRight
        };
        let text = Text::new(language, direction).expect("the text of the language");
        for theme in [Theme::Light, Theme::Dark] {
            for scale in [1.0_f32, 1.5] {
                let picture = (*name == "updated").then(|| taskbar(rtl));
                let window = describe(content, false, picture);
                let first = lay_out(&window, client_size(CORNER_WIDTH.0, 400.0), 1.0, rtl, &text);
                let size = client_size(CORNER_WIDTH.0, 400.0 + first.shortfall.h);
                let layout = lay_out(&window, size, 1.0, rtl, &text);
                let look = Look {
                    theme,
                    rtl,
                    b: layout.b,
                    h: size.h,
                    text: &text,
                };
                let mut canvas =
                    Canvas::in_memory(size.w, size.h, scale, Mode::Direct2D).expect("a canvas");
                canvas.frame_with(Some(&mut art), |painter| {
                    painter.clear(Color::Bg.get(theme));
                    for item in &layout.items {
                        elements::draw(painter, &look, &item.part, item.rect, State::NORMAL);
                    }
                });
                let file = folder.join(format!(
                    "corner-{name}-{language}-{}-{}.png",
                    if theme == Theme::Dark {
                        "dark"
                    } else {
                        "light"
                    },
                    (scale * 100.0) as u32
                ));
                canvas.save_png(&file).expect("the picture is written");
            }
        }
    }
}

#[cfg(feature = "testing")]
fn hint(title: &str, main: &str, second: &str) -> Content {
    Content {
        about: "Флаг".to_owned(),
        title: title.to_owned(),
        text: "Щёлкните по флагу, чтобы выбрать раскладку из списка.".to_owned(),
        link: None,
        main: (main.to_owned(), Action::Close),
        second: Some((second.to_owned(), Action::Close)),
        picture: false,
        check: None,
    }
}

#[cfg(feature = "testing")]
fn letter(title: &str, main: &str, second: &str) -> Content {
    Content {
        about: "Письмо".to_owned(),
        title: title.to_owned(),
        text: "Новости Lang Switcher и ответы на вопросы.".to_owned(),
        link: Some(("Все письма".to_owned(), "letters")),
        main: (main.to_owned(), Action::Close),
        second: Some((second.to_owned(), Action::Close)),
        picture: false,
        check: None,
    }
}

fn taskbar(icons_left: bool) -> lang_switcher::ui::scene::Taskbar {
    use lang_switcher::layouts::LayoutId;
    lang_switcher::ui::scene::Taskbar {
        layouts: [
            LayoutId::from_raw(0x0409_0409),
            LayoutId::from_raw(0x0419_0419),
        ],
        codes: ["EN".to_owned(), "RU".to_owned()],
        shown: 0,
        plate: true,
        cursor: Some(0.5),
        list: true,
        hover: true,
        icons_left,
    }
}

/// «Обновлено» carries the picture of the taskbar between the message and the buttons, at its
/// height; a content without a picture has none even when a moment is given.
#[test]
fn the_picture_of_the_taskbar_stands_between_the_message_and_the_buttons() {
    use lang_switcher::ui::tokens::CORNER_PICTURE;
    let mut with = content("Что нового", "Закрыть");
    with.picture = true;
    let window = describe(&with, false, Some(taskbar(false)));
    let first = lay_out(
        &window,
        client_size(CORNER_WIDTH.0, 400.0),
        1.0,
        false,
        &MEASURE,
    );
    let layout = lay_out(
        &window,
        client_size(CORNER_WIDTH.0, 400.0 + first.shortfall.h),
        1.0,
        false,
        &MEASURE,
    );
    let picture = first_of(&layout, |part| matches!(part, Part::Taskbar(_)));
    assert_eq!(picture.h, CORNER_PICTURE.0, "the height of the picture");
    let main = rect_of(&layout, Id::Main);
    assert!(
        picture.bottom() <= main.y,
        "the picture is not above the buttons"
    );

    let without = describe(&content("A", "B"), false, Some(taskbar(false)));
    let layout = lay_out(
        &without,
        client_size(CORNER_WIDTH.0, 400.0),
        1.0,
        false,
        &MEASURE,
    );
    assert!(
        !layout
            .items
            .iter()
            .any(|item| matches!(item.part, Part::Taskbar(_))),
        "a content without a picture shows one"
    );
}

#[cfg(feature = "testing")]
fn with_picture(mut content: Content) -> Content {
    content.picture = true;
    content
}
