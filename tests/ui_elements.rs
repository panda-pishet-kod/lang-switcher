//! The elements of the interface shell, `src\ui\elements.rs` (stage Э120, T-120-6), over the
//! catalogue of the element bench: every state draws a picture of its own, an unavailable element
//! answers no press, a switch, a check box and a radio button answer on their whole line and a
//! choice card on the whole card, and the dark fill of `button` is the main button's alone.
//!
//! The sheets of pictures themselves are printed by `print_the_element_bench`, under the `testing`
//! feature, into the folder `LANGSW_UI_BENCH_DIR` names.

use lang_switcher::ui::bench::{Specimen, catalogue, draw, state_name};
use lang_switcher::ui::canvas::{Canvas, Image, Mode, Rect};
use lang_switcher::ui::elements::{Look, State, hit_area};
use lang_switcher::ui::layout::{Class, Part, place_node};
use lang_switcher::ui::scene::{ButtonKind, Node};
use lang_switcher::ui::text::{Direction, Text};
use lang_switcher::ui::tokens::{Color, Theme, window_b};
use std::path::Path;

/// Room around a specimen for the keyboard ring.
const MARGIN: f32 = 8.0;

fn rc(tag: &str) -> impl Fn(&str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("res/lang/{tag}.rc"));
    let text = std::fs::read_to_string(path).expect("the strings of the language");
    move |id: &str| {
        text.lines()
            .find(|line| line.split_whitespace().next() == Some(id))
            .and_then(|line| Some(line[line.find('"')? + 1..line.rfind('"')?].to_owned()))
            .unwrap_or_else(|| id.to_owned())
    }
}

fn b() -> f32 {
    window_b(911.0, 512.0, 1.0)
}

fn specimens(text: &Text) -> Vec<Specimen> {
    catalogue(&rc("ru"), b(), text)
}

fn picture(specimen: &Specimen, look: &Look, state: State) -> Image {
    let mut canvas = Canvas::in_memory(
        specimen.w + MARGIN * 2.0,
        specimen.h + MARGIN * 2.0,
        1.0,
        Mode::Direct2D,
    )
    .expect("a canvas");
    canvas.frame(|painter| {
        painter.clear(Color::Bg.get(look.theme));
        draw(painter, look, specimen, MARGIN, MARGIN, state);
    });
    canvas.image().expect("pixels")
}

#[test]
fn every_state_of_every_element_draws_a_picture_of_its_own() {
    let text = Text::new("ru-RU", Direction::LeftToRight).expect("DirectWrite");
    let mut faults = Vec::new();

    for theme in [Theme::Light, Theme::Dark] {
        let look = Look {
            theme,
            rtl: false,
            b: b(),
            h: 512.0,
            text: &text,
        };
        for specimen in specimens(&text).iter().filter(|s| s.interactive) {
            let shots: Vec<Image> = State::ALL
                .iter()
                .map(|state| picture(specimen, &look, *state))
                .collect();
            let [normal, hover, pressed, focus, disabled] = &shots[..] else {
                panic!("five states");
            };
            for (a, b, pair) in [
                (normal, hover, "обычное и под мышью"),
                (hover, pressed, "под мышью и нажато"),
                (normal, focus, "обычное и с клавиатуры"),
                (normal, disabled, "обычное и недоступно"),
            ] {
                if a == b {
                    faults.push(format!("{theme:?}: «{}» — {pair} совпадают", specimen.name));
                }
            }
        }
    }

    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

#[test]
fn an_unavailable_element_answers_no_press() {
    let text = Text::new("ru-RU", Direction::LeftToRight).expect("DirectWrite");

    for specimen in specimens(&text) {
        for item in &specimen.items {
            assert_eq!(
                hit_area(&item.part, item.rect, State::DISABLED),
                None,
                "«{}»: {}",
                specimen.name,
                item.part.name()
            );
        }
        if specimen.interactive {
            assert!(
                specimen
                    .items
                    .iter()
                    .any(|item| hit_area(&item.part, item.rect, State::NORMAL).is_some()),
                "«{}» answers a press somewhere",
                specimen.name
            );
        }
    }
}

/// A switch, a check box and a radio button answer on their whole line; a choice card on the whole
/// card (lesson of stage E81).
#[test]
fn the_controls_answer_on_their_whole_line_and_a_card_on_the_whole_card() {
    let text = Text::new("ru-RU", Direction::LeftToRight).expect("DirectWrite");
    let width = 380.0;
    let label = "Запускать при входе в систему".to_owned();

    for node in [
        Node::Toggle {
            id: 1u8,
            label: label.clone(),
            hint: Some("подсказка".to_owned()),
            on: true,
            badge: None,
        },
        Node::Check {
            id: 1,
            label: label.clone(),
            on: true,
        },
        Node::Radio {
            id: 1,
            label: label.clone(),
            on: false,
        },
        Node::ChoiceCard {
            id: 1,
            title: label.clone(),
            words: "строка".to_owned(),
            selected: true,
        },
    ] {
        let (items, height) = place_node(&node, width, b(), Class::Ordinary, &text);
        let areas: Vec<Rect> = items
            .iter()
            .filter(|item| item.id == Some(1))
            .filter_map(|item| hit_area(&item.part, item.rect, State::NORMAL))
            .collect();

        assert_eq!(areas, vec![Rect::new(0.0, 0.0, width, height)], "{node:?}");
    }
}

/// The squares of a picture, 5 × 5 pixels, filled wholly with `colour`, by their left top corner:
/// a fill, not the stems of text drawn in the same colour.
fn solid(image: &Image, colour: lang_switcher::ui::tokens::Rgb) -> Vec<(u32, u32)> {
    let side = 5;
    let mut found = Vec::new();
    for y in 0..image.height.saturating_sub(side) {
        for x in 0..image.width.saturating_sub(side) {
            if (0..side).all(|dy| (0..side).all(|dx| image.at(x + dx, y + dy) == colour)) {
                found.push((x, y));
            }
        }
    }
    found
}

/// The dark fill of `button` is the main button's and nobody else's: on every specimen, in both
/// themes and every state, a fill of that colour stands only inside a main button.
#[test]
fn the_dark_fill_belongs_to_the_main_button_alone() {
    let text = Text::new("ru-RU", Direction::LeftToRight).expect("DirectWrite");
    let mut faults = Vec::new();

    for theme in [Theme::Light, Theme::Dark] {
        let look = Look {
            theme,
            rtl: false,
            b: b(),
            h: 512.0,
            text: &text,
        };
        let dark = Color::Button.get(theme);
        for specimen in specimens(&text) {
            let mains: Vec<Rect> = specimen
                .items
                .iter()
                .filter(|item| {
                    matches!(
                        item.part,
                        Part::Button {
                            kind: ButtonKind::Primary,
                            ..
                        }
                    )
                })
                .map(|item| {
                    Rect::new(
                        item.rect.x + MARGIN,
                        item.rect.y + MARGIN,
                        item.rect.w,
                        item.rect.h,
                    )
                })
                .collect();
            let states: &[State] = if specimen.interactive {
                State::ALL
            } else {
                &[State::NORMAL]
            };
            for state in states {
                let image = picture(&specimen, &look, *state);
                let outside = solid(&image, dark).into_iter().find(|(x, y)| {
                    !mains.iter().any(|m| {
                        (*x as f32) >= m.x.floor()
                            && (*y as f32) >= m.y.floor()
                            && (*x as f32) < m.right()
                            && (*y as f32) < m.bottom()
                    })
                });
                if let Some((x, y)) = outside {
                    faults.push(format!(
                        "{theme:?}: «{}» ({}) — заливка button в {x}, {y}",
                        specimen.name,
                        state_name(*state)
                    ));
                }
            }
        }
    }

    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

/// Prints the sheets of the bench into `LANGSW_UI_BENCH_DIR`: 5 sheets × light and dark × straight
/// and mirrored × 100 % and 150 %.
#[cfg(feature = "testing")]
#[test]
fn print_the_element_bench() {
    let Some(folder) = std::env::var_os("LANGSW_UI_BENCH_DIR") else {
        return;
    };
    let written = lang_switcher::ui::bench::print_sheets(Path::new(&folder), &rc("ru"), &rc("he"))
        .expect("the sheets are printed");
    assert_eq!(written.len(), 40);
}
