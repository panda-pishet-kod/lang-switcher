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
            hint: None,
        },
        Node::Radio {
            id: 1,
            label: label.clone(),
            on: false,
        },
        Node::ChoiceCard {
            id: 1,
            title: label.clone(),
            words: Some("строка".to_owned()),
            selected: true,
            next: None,
            icon: None,
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

/// **The label of a button goes on to a second line only where the layout made room for it**
/// (§3а, step 5) — the stand of 2026-11-20: «Close», exactly as wide as its main button, came out
/// «Clos / e». A label that fits to the rounding of a measure stays whole on its line; a wider one in
/// a button one line high stays on its line too (cut and counted by the layout, not broken here).
#[test]
fn a_button_label_wraps_only_where_the_layout_made_the_button_taller() {
    use lang_switcher::ui::elements::button_label_wraps;

    let line = 18.0;
    let one_line = 18.0;
    let two_lines = 36.0;
    assert!(
        !button_label_wraps(40.0, 40.0, one_line, line),
        "exactly as wide"
    );
    assert!(
        !button_label_wraps(40.004, 40.0, one_line, line),
        "wider by a rounding"
    );
    assert!(
        !button_label_wraps(40.004, 40.0, two_lines, line),
        "wider by a rounding, even in a taller button"
    );
    assert!(
        !button_label_wraps(60.0, 40.0, one_line, line),
        "a button one line high"
    );
    assert!(
        button_label_wraps(60.0, 40.0, two_lines, line),
        "made taller for it"
    );
}

/// **The drawn value of a field of code reads in the direction of its own text** (T-120-19,
/// `FieldKind::Code { rtl }`): in a window in Hebrew a report in Latin letters drawn with `rtl`
/// false starts at the left of the field — the picture of the window (C3, the instrument of fitting)
/// shows it as the field of Windows does; with the window's direction it stands at the right.
#[test]
fn the_drawn_value_of_a_field_of_code_reads_in_its_own_direction() {
    use lang_switcher::ui::elements::draw as draw_part;

    let text = Text::new("he-IL", Direction::RightToLeft).expect("DirectWrite");
    let look = Look {
        theme: Theme::Light,
        rtl: true,
        b: b(),
        h: 512.0,
        text: &text,
    };
    let rect = Rect::new(MARGIN, MARGIN, 600.0, 120.0);
    let first_ink = |rtl: Option<bool>| {
        let part = Part::Input {
            multiline: true,
            code: true,
            rtl,
            capture: None,
            value: "Lang Switcher 1.8.0".to_owned(),
            hint: String::new(),
        };
        let mut canvas = Canvas::in_memory(
            rect.w + MARGIN * 2.0,
            rect.h + MARGIN * 2.0,
            1.0,
            Mode::Direct2D,
        )
        .expect("a canvas");
        let background = Color::Bg.get(look.theme);
        canvas.frame(|painter| {
            painter.clear(background);
            draw_part(painter, &look, &part, rect, State::default());
        });
        let image = canvas.image().expect("pixels");
        let ink = Color::Ink.get(look.theme);
        let width = image.width as usize;
        image
            .pixels()
            .iter()
            .enumerate()
            .filter(|(_, pixel)| **pixel == ink)
            .map(|(at, _)| at % width)
            .min()
            .expect("the value drawn in ink")
    };
    let own = first_ink(Some(false));
    let window = first_ink(None);
    assert!(
        own + 200 < window,
        "left to right from x {own}; the window's direction from x {window}"
    );
}

/// **Б6 of AUDIT-180 — the value of a choice field ends short of its corner mark.** A name longer
/// than the field («English (United States)» in «Источник») ran into the mark; it is cut with an
/// ellipsis a padding before it: nothing of the text in that gap.
///
/// ⚠ Control: the text is drawn left of the gap.
#[test]
fn a_long_value_of_a_choice_field_stops_short_of_its_corner_mark() {
    use lang_switcher::ui::elements::draw as draw_part;
    use lang_switcher::ui::tokens::shape::CHEVRON;
    use lang_switcher::ui::tokens::{FIELD_PAD, Rgb};

    let text = Text::new("en-US", Direction::LeftToRight).expect("DirectWrite");
    let look = Look {
        theme: Theme::Light,
        rtl: false,
        b: b(),
        h: 512.0,
        text: &text,
    };
    let rect = Rect::new(MARGIN, MARGIN, 160.0, 34.0);
    let mut canvas = Canvas::in_memory(
        rect.w + MARGIN * 2.0,
        rect.h + MARGIN * 2.0,
        1.0,
        Mode::Direct2D,
    )
    .expect("a canvas");
    canvas.frame(|painter| {
        painter.clear(Color::Bg.get(Theme::Light));
        draw_part(
            painter,
            &look,
            &Part::Field {
                value: "English (United States) and some more".to_owned(),
            },
            rect,
            State::NORMAL,
        );
    });
    let image = canvas.image().expect("pixels");
    let (pad, mark) = (FIELD_PAD.h.at(b()), CHEVRON.at(b()));
    let mark_left = rect.x + rect.w - pad - mark;
    let surface = Color::Surface.get(Theme::Light);
    let light = u32::from(surface.r) + u32::from(surface.g) + u32::from(surface.b);
    let dark = |p: Rgb| u32::from(p.r) + u32::from(p.g) + u32::from(p.b) + 150 < light;
    let middle = (rect.y + rect.h / 2.0) as u32;
    let inked = |from: f32, to: f32| -> Vec<u32> {
        (from as u32..to as u32)
            .filter(|x| (middle - 6..middle + 6).any(|y| dark(image.at(*x, y))))
            .collect()
    };

    assert!(
        !inked(rect.x + pad, mark_left - pad).is_empty(),
        "control: the value is drawn"
    );
    let gap = inked(mark_left - pad + 1.0, mark_left - 1.0);
    assert!(
        gap.is_empty(),
        "text in the padding before the corner mark, at x {gap:?} (mark at {mark_left})"
    );
}

// ---- AUDIT-180b (решение 208.227): the look of the parts the owner found wrong ---------------

/// `parts` drawn in `theme` over `ground` on a canvas of `w` × `h` points at 100 %.
fn parts_drawn(
    theme: Theme,
    ground: Color,
    w: f32,
    h: f32,
    parts: &[(Part, Rect, State)],
) -> Image {
    let text = Text::new("ru-RU", Direction::LeftToRight).expect("DirectWrite");
    let look = Look {
        theme,
        rtl: false,
        b: b(),
        h: 512.0,
        text: &text,
    };
    let mut canvas = Canvas::in_memory(w, h, 1.0, Mode::Direct2D).expect("a canvas");
    canvas.frame(|painter| {
        painter.clear(ground.get(theme));
        for (part, rect, state) in parts {
            lang_switcher::ui::elements::draw(painter, &look, part, *rect, *state);
        }
    });
    canvas.image().expect("pixels")
}

/// How far `a` is from `b`: the largest difference of a channel.
fn apart(a: lang_switcher::ui::tokens::Rgb, b: lang_switcher::ui::tokens::Rgb) -> u8 {
    a.r.abs_diff(b.r)
        .max(a.g.abs_diff(b.g))
        .max(a.b.abs_diff(b.b))
}

/// The pixels of `image` within `near` of `colour`, inside `rect` or, with `outside`, out of it.
fn pixels_of(
    image: &Image,
    colour: lang_switcher::ui::tokens::Rgb,
    rect: Rect,
    outside: bool,
) -> usize {
    let mut count = 0;
    for y in 0..image.height {
        for x in 0..image.width {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let inside = fx > rect.x && fx < rect.right() && fy > rect.y && fy < rect.bottom();
            if inside != outside && apart(image.at(x, y), colour) <= 8 {
                count += 1;
            }
        }
    }
    count
}

/// **AUDIT-180b п. 1** — in a list of check boxes the cursor chooses nothing, so the mouse sees no
/// frame on it and the keyboard does; the cursor of a list of a choice is the value and keeps its
/// frame. Red before: the first row of «Исключения» always in a frame of `ink`.
#[test]
fn the_cursor_of_a_list_of_check_boxes_is_framed_for_the_keyboard_alone() {
    let rect = Rect::new(10.0, 10.0, 200.0, 30.0);
    let ink = Color::Ink.get(Theme::Light);
    let framed = |check: Option<bool>, state: State| {
        let row = Part::ListRow {
            selected: true,
            index: 0,
            check,
        };
        let image = parts_drawn(
            Theme::Light,
            Color::Surface,
            220.0,
            50.0,
            &[(row, rect, state)],
        );
        pixels_of(&image, ink, rect.inset(-1.0), false) > 0
    };
    assert!(
        !framed(Some(false), State::NORMAL),
        "the cursor of a list of check boxes is framed for the mouse"
    );
    assert!(
        !framed(Some(true), State::HOVER),
        "the cursor of a list of check boxes is framed under the mouse"
    );
    assert!(
        framed(Some(false), State::FOCUS),
        "the keyboard does not see the cursor of a list of check boxes"
    );
    assert!(
        framed(None, State::NORMAL),
        "the value of a list of a choice lost its frame"
    );
}

/// **AUDIT-180b п. 2** — a group that is one stop of Tab rings its current element alone: the
/// current section, the selected segment, the row of the cursor; not every section, not the track,
/// not the whole list. Red before: five rings on the sections, the ring around the track and the
/// list.
#[test]
fn a_group_rings_its_current_element_alone() {
    let rect = Rect::new(20.0, 20.0, 160.0, 32.0);
    let accent = Color::Accent.get(Theme::Light);
    let section = |current| Part::Section {
        index: 0,
        current,
        label: "Общие".to_owned(),
    };
    let segment = |selected| Part::Segment {
        index: 0,
        selected,
        label: "Светлая".to_owned(),
    };
    let row = |selected| Part::ListRow {
        selected,
        index: 0,
        check: Some(false),
    };
    for (part, ringed) in [
        (section(true), true),
        (section(false), false),
        (segment(true), true),
        (segment(false), false),
        (Part::Track, false),
        (row(true), true),
        (row(false), false),
    ] {
        let image = parts_drawn(
            Theme::Light,
            Color::Surface,
            200.0,
            72.0,
            &[(part.clone(), rect, State::FOCUS)],
        );
        // The ring stands outside the part; the stripe of the current section is inside.
        let ring = pixels_of(&image, accent, rect.inset(-1.0), true);
        assert_eq!(
            ring > 0,
            ringed,
            "{part:?} with the keyboard: {ring} pixels of the ring"
        );
    }
}

/// **AUDIT-180b п. 3** — a segment under the mouse is filled inside its track, whose frame stays
/// whole; a usual button under the mouse in the dark theme goes a step further, to `line`, where
/// `surface2` is hardly told from `surface`. Red before: the fill over the whole height covered
/// the frame of the track; the dark button 2B303A → 323845.
#[test]
fn the_hover_of_a_segment_keeps_its_track_whole_and_a_dark_button_shows_the_mouse() {
    let rect = Rect::new(10.0, 10.0, 120.0, 30.0);
    for theme in [Theme::Light, Theme::Dark] {
        let segment = Part::Segment {
            index: 0,
            selected: false,
            label: String::new(),
        };
        let image = parts_drawn(
            theme,
            Color::Surface,
            140.0,
            50.0,
            &[
                (Part::Track, rect, State::NORMAL),
                (segment, rect, State::HOVER),
            ],
        );
        let fill = Color::Line.get(theme);
        let (mid, top) = ((rect.x + rect.w / 2.0) as u32, rect.y as u32);
        assert!(
            apart(image.at(mid, (rect.y + rect.h / 2.0) as u32), fill) <= 2,
            "{theme:?}: the segment is not filled under the mouse"
        );
        assert!(
            apart(image.at(mid, top), fill) > 2,
            "{theme:?}: the fill of the segment covers the frame of its track"
        );
    }

    let button = Part::Button {
        kind: ButtonKind::Normal,
        label: String::new(),
    };
    for (theme, state, colour) in [
        (Theme::Dark, State::HOVER, Color::Line),
        (Theme::Dark, State::PRESSED, Color::Line2),
        (Theme::Light, State::HOVER, Color::Surface2),
        (Theme::Light, State::PRESSED, Color::Line),
    ] {
        let image = parts_drawn(
            theme,
            Color::Bg,
            140.0,
            50.0,
            &[(button.clone(), rect, state)],
        );
        let at = image.at(
            (rect.x + rect.w / 2.0) as u32,
            (rect.y + rect.h / 2.0) as u32,
        );
        assert!(
            apart(at, colour.get(theme)) <= 1,
            "{theme:?}, {state:?}: the usual button is {at:?}, not {colour:?}"
        );
    }
}

/// **AUDIT-180b п. 4** — a section of the settings under the mouse takes `surface`, the ground of
/// the current one, pressed `line`: a step of the fill from nothing is `surface2`, the colour of
/// the panel it stands on, and did not show (in the mock-up too). Red before: `surface2`.
#[test]
fn a_section_under_the_mouse_shows_on_its_panel() {
    let rect = Rect::new(10.0, 10.0, 160.0, 32.0);
    for theme in [Theme::Light, Theme::Dark] {
        for (state, colour) in [
            (State::HOVER, Color::Surface),
            (State::PRESSED, Color::Line),
        ] {
            let section = Part::Section {
                index: 1,
                current: false,
                label: String::new(),
            };
            let image = parts_drawn(
                theme,
                Color::Surface2,
                180.0,
                52.0,
                &[(section, rect, state)],
            );
            let at = image.at(
                (rect.x + rect.w / 2.0) as u32,
                (rect.y + rect.h / 2.0) as u32,
            );
            assert!(
                apart(at, colour.get(theme)) <= 1,
                "{theme:?}, {state:?}: the section is {at:?}, not {colour:?}"
            );
        }
    }
}

/// **AUDIT-180b п. 5** — the dots of the panel are seen at 100 %: a dot has a pixel at least three
/// quarters of the way from the panel's `surface2` to `line2`, as the mock-up's (its CSS names the
/// radius, 1,2 px: a dot of 2,4 across). Red before: a dot of 1,2 across — its darkest pixel 236
/// on a panel of 241 in the light theme, the mock-up's 211.
#[test]
fn the_dots_of_the_panel_are_seen_at_a_hundred_percent() {
    for theme in [Theme::Light, Theme::Dark] {
        let image = parts_drawn(
            theme,
            Color::Bg,
            72.0,
            72.0,
            &[(Part::Panel, Rect::new(0.0, 0.0, 72.0, 72.0), State::NORMAL)],
        );
        let (ground, dot) = (Color::Surface2.get(theme), Color::Line2.get(theme));
        let darkest = (6..13)
            .flat_map(|y| (6..13).map(move |x| (x, y)))
            .map(|(x, y)| apart(image.at(x, y), ground))
            .max()
            .unwrap_or(0);
        let span = apart(dot, ground);
        assert!(
            u32::from(darkest) * 4 >= u32::from(span) * 3,
            "{theme:?}: the darkest pixel of a dot is {darkest} from the panel of {span}"
        );
    }
}
