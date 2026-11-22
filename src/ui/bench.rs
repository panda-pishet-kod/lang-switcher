//! The element bench (T-120-6): every element of the passport, alone, in every state of §5 — the
//! catalogue the tests of the elements go over, and, under the `testing` feature, the sheets of
//! pictures the owner approves the look from (`print_sheets`).
//!
//! A specimen is placed by the layout itself wherever the layout has a node for it, so the bench
//! shows what windows will show; the parts no window places yet (a list, a key, the progress of a
//! wizard, the corner window) are placed here from the table of styles.
//!
//! The words are real: the strings of `res\lang` — Russian, and on a mirrored sheet Hebrew.

use crate::ui::canvas::{Painter, Rect};
use crate::ui::elements::{self, Look, State};
use crate::ui::layout::{Class, Part, Placed, Status, place_node};
use crate::ui::scene::{ButtonKind, FieldKind, ListItem, Node, Surface};
use crate::ui::text::{Align, Measure, Style, Wrap};
use crate::ui::tokens::{
    BUTTON_GAP, CONTROL_GAP, CORNER_ART, CORNER_PAD, CORNER_WIDTH, KEY_PAD, MENU_MIN_WIDTH,
    MENU_ROW_MIN_HEIGHT, PANEL_ART, PANEL_SHARE, Role, WIZARD_STEP, WIZARD_STEP_GAP,
};

/// One element on the bench.
pub struct Specimen {
    /// Its name, in Russian, for the caption.
    pub name: &'static str,
    /// The sheet it is printed on, 1 to 5.
    pub sheet: u8,
    /// Whether the states of the mouse and the keyboard apply to it.
    pub interactive: bool,
    pub items: Vec<Placed<u8>>,
    pub w: f32,
    pub h: f32,
}

/// The Russian name of a state, for the captions.
pub fn state_name(state: State) -> &'static str {
    match (state.disabled, state.pressed, state.hover, state.focus) {
        (true, ..) => "недоступно",
        (false, true, ..) => "нажато",
        (false, false, true, _) => "под мышью",
        (false, false, false, true) => "с клавиатуры",
        _ => "обычное",
    }
}

/// Every specimen, its words from `word` (an `IDS_` name to the string), at the type size `b`.
pub fn catalogue(word: &dyn Fn(&str) -> String, b: f32, measure: &dyn Measure) -> Vec<Specimen> {
    let mut all = Vec::new();
    let mut node = |name, sheet, interactive, width: f32, class, node: Node<u8>| {
        let (items, h) = place_node(&node, width, b, class, measure);
        all.push(Specimen {
            name,
            sheet,
            interactive,
            items,
            w: width,
            h,
        });
    };
    let button = |label: String, kind, browser| Node::Button {
        id: 1,
        label,
        kind,
        browser,
    };
    let narrow = 260.0;
    let wide = 380.0;

    // Sheet 1: buttons, rows of buttons, the link, the sign ↗.
    node(
        "Кнопка главная",
        1,
        true,
        narrow,
        Class::Ordinary,
        button(word("IDS_CLOSE"), ButtonKind::Primary, false),
    );
    node(
        "Кнопка обычная",
        1,
        true,
        narrow,
        Class::Ordinary,
        button(word("IDS_CANCEL"), ButtonKind::Normal, false),
    );
    node(
        "Кнопка тихая",
        1,
        true,
        narrow,
        Class::Ordinary,
        button(word("IDS_LOG_SAVE"), ButtonKind::Quiet, false),
    );
    node(
        "Кнопка со знаком ↗",
        1,
        true,
        narrow,
        Class::Ordinary,
        button(word("IDS_ABOUT_CAPTION"), ButtonKind::Normal, true),
    );
    node(
        "Ряд кнопок",
        1,
        false,
        wide,
        Class::Ordinary,
        Node::Buttons(vec![
            button(word("IDS_CANCEL"), ButtonKind::Normal, false),
            button(word("IDS_CLOSE"), ButtonKind::Primary, false),
        ]),
    );
    node(
        "Ряд кнопок, не помещается",
        1,
        false,
        narrow / 2.0 + 40.0,
        Class::Ordinary,
        Node::Buttons(vec![
            button(word("IDS_CANCEL"), ButtonKind::Normal, false),
            button(word("IDS_CLOSE"), ButtonKind::Primary, false),
        ]),
    );
    node(
        "Ссылка",
        1,
        true,
        narrow,
        Class::Ordinary,
        Node::Link {
            id: 1,
            words: word("IDS_ABOUT_CAPTION"),
        },
    );

    // Sheet 2: switches, check boxes, radio buttons, segments, choice cards.
    for (name, on) in [
        ("Выключатель, включён", true),
        ("Выключатель, выключен", false),
    ] {
        node(
            name,
            2,
            true,
            wide,
            Class::Ordinary,
            Node::Toggle {
                id: 1,
                label: word("IDS_AUTOSTART"),
                hint: None,
                on,
                badge: None,
            },
        );
    }
    for (name, on) in [("Флажок, включён", true), ("Флажок, выключен", false)]
    {
        node(
            name,
            2,
            true,
            wide,
            Class::Ordinary,
            Node::Check {
                id: 1,
                label: word("IDS_SOUND"),
                on,
                hint: None,
            },
        );
    }
    for (name, on) in [
        ("Радиокнопка, включена", true),
        ("Радиокнопка, выключена", false),
    ] {
        node(
            name,
            2,
            true,
            wide,
            Class::Ordinary,
            Node::Radio {
                id: 1,
                label: word("IDS_THEME_LIGHT"),
                on,
            },
        );
    }
    node(
        "Сегменты на 2 ячейки",
        2,
        true,
        wide,
        Class::Ordinary,
        Node::Segments {
            id: 1,
            label: word("IDS_THEME_LABEL"),
            options: vec![word("IDS_THEME_LIGHT"), word("IDS_THEME_DARK")],
            selected: 0,
        },
    );
    node(
        "Сегменты на 3 ячейки",
        2,
        true,
        wide + 80.0,
        Class::Ordinary,
        Node::Segments {
            id: 1,
            label: word("IDS_THEME_LABEL"),
            options: vec![
                word("IDS_THEME_LIGHT"),
                word("IDS_THEME_DARK"),
                word("IDS_THEME_SYSTEM"),
            ],
            selected: 2,
        },
    );
    for (name, selected) in [
        ("Карточка выбора, выбрана", true),
        ("Карточка выбора", false),
    ] {
        node(
            name,
            2,
            true,
            wide,
            Class::Ordinary,
            Node::ChoiceCard {
                id: 1,
                title: word("IDS_THEME_SYSTEM"),
                words: word("IDS_THEME_LABEL"),
                selected,
            },
        );
    }

    // Sheet 3: fields, lists, sections, the progress of a wizard.
    node(
        "Поле выбора",
        3,
        true,
        wide,
        Class::Ordinary,
        Node::Choice {
            id: 1,
            label: word("IDS_LANGUAGE_LABEL"),
            options: vec![
                "Русский".to_owned(),
                "English".to_owned(),
                "Deutsch".to_owned(),
            ],
            selected: 0,
        },
    );
    node(
        "Разделы столбцом",
        3,
        true,
        220.0,
        Class::Ordinary,
        Node::Sections {
            id: 1,
            items: vec![
                word("IDS_GROUP_GENERAL"),
                word("IDS_GROUP_HOTKEY"),
                word("IDS_THEME_LABEL"),
            ],
            current: 1,
        },
    );
    node(
        "Разделы ярлыками",
        3,
        true,
        wide + 120.0,
        Class::Narrow,
        Node::Sections {
            id: 1,
            items: vec![
                word("IDS_GROUP_GENERAL"),
                word("IDS_GROUP_HOTKEY"),
                word("IDS_THEME_LABEL"),
            ],
            current: 1,
        },
    );

    // Sheet 4: surfaces, cards, the header, a line of settings.
    let words = |id: &str| Node::Text(Role::Body, word(id));
    node(
        "Блок",
        4,
        false,
        wide,
        Class::Ordinary,
        Node::Surface(Surface::Block, Box::new(words("IDS_AUTOSTART"))),
    );
    node(
        "Вложенная карточка",
        4,
        false,
        wide,
        Class::Ordinary,
        Node::Surface(Surface::Nested, Box::new(words("IDS_AUTOSTART"))),
    );
    node(
        "Замечание",
        4,
        false,
        wide,
        Class::Ordinary,
        Node::Surface(Surface::Note, Box::new(words("IDS_AUTOSTART"))),
    );
    node(
        "Карточка-действие",
        4,
        false,
        wide,
        Class::Ordinary,
        // The card draws the frame of its block itself (208.209, AUDIT-180 О3).
        Node::Action {
            title: word("IDS_GROUP_HOTKEY"),
            words: word("IDS_AUTOSTART"),
            buttons: vec![button(word("IDS_LOG_SAVE"), ButtonKind::Normal, false)],
        },
    );
    node(
        "Карточка письма",
        4,
        false,
        wide,
        Class::Ordinary,
        Node::Surface(
            Surface::Block,
            Box::new(Node::Rows(vec![
                Node::Line(Role::Label, word("IDS_GROUP_GENERAL")),
                Node::Text(Role::Strong, word("IDS_ABOUT_CAPTION")),
                Node::Line(Role::Body, word("IDS_AUTOSTART")),
                Node::Buttons(vec![button(word("IDS_CLOSE"), ButtonKind::Normal, false)]),
            ])),
        ),
    );
    node(
        "Строка настройки",
        4,
        true,
        wide,
        Class::Ordinary,
        Node::Toggle {
            id: 1,
            label: word("IDS_AUTOSTART"),
            hint: Some(word("IDS_SOUND")),
            on: true,
            badge: None,
        },
    );
    node(
        "Шапка окна",
        4,
        false,
        wide,
        Class::Ordinary,
        Node::Header {
            eyebrow: Some("Lang Switcher 1.8.0".to_owned()),
            title: word("IDS_ABOUT_CAPTION"),
            subtitle: Some(word("IDS_AUTOSTART")),
        },
    );

    let field = |label: String, value: &str, hint: String, kind| Node::Field {
        id: 1,
        label,
        value: value.to_owned(),
        hint,
        kind,
    };
    node(
        "Поле",
        3,
        true,
        wide,
        Class::Ordinary,
        field(
            word("IDS_EXCLUSION_MANUAL"),
            "game.exe",
            word("IDS_EXCLUSION_HINT"),
            FieldKind::Line,
        ),
    );
    node(
        "Поле с подсказкой",
        3,
        true,
        wide,
        Class::Ordinary,
        field(
            word("IDS_EXCLUSION_MANUAL"),
            "",
            word("IDS_EXCLUSION_HINT"),
            FieldKind::Line,
        ),
    );
    node(
        "Многострочное поле",
        3,
        true,
        wide,
        Class::Ordinary,
        field(
            word("IDS_WIZARD_IDEA_WHAT"),
            "",
            word("IDS_WIZARD_IDEA_NOTE"),
            FieldKind::Text,
        ),
    );
    let themes = [
        word("IDS_THEME_LIGHT"),
        word("IDS_THEME_DARK"),
        word("IDS_THEME_SYSTEM"),
    ];
    for (name, checks) in [("Список с флажками", true), ("Список без флажков", false)]
    {
        let rows = themes
            .iter()
            .enumerate()
            .map(|(index, label)| ListItem {
                label: label.clone(),
                check: checks.then_some(index != 2),
                mark: false,
            })
            .collect();
        node(
            name,
            3,
            true,
            wide,
            Class::Ordinary,
            Node::List {
                id: 1,
                rows,
                selected: 1,
                first: 0,
            },
        );
    }
    let layouts = [
        "English (United States)",
        "Русская",
        "Українська",
        "Deutsch",
        "Français",
        "Español",
        "Italiano",
        "Polski",
        "Türkçe",
        "Ελληνικά",
        "עברית",
        "العربية",
    ];
    node(
        "Список поля выбора",
        3,
        true,
        wide,
        Class::Ordinary,
        Node::List {
            id: 1,
            rows: layouts
                .iter()
                .enumerate()
                .map(|(index, label)| ListItem {
                    label: (*label).to_owned(),
                    check: None,
                    mark: index == 1,
                })
                .collect(),
            selected: 3,
            first: 0,
        },
    );
    node(
        "Список с прокруткой",
        3,
        true,
        wide,
        Class::Ordinary,
        Node::List {
            id: 1,
            rows: layouts
                .iter()
                .map(|label| ListItem {
                    label: (*label).to_owned(),
                    check: None,
                    mark: false,
                })
                .collect(),
            selected: 5,
            first: 3,
        },
    );

    // The parts no node places yet.
    let measure_line = |role: Role, text: &str| {
        measure.measure(text, Style::of(role, b, 500.0), Wrap::OneLine, f32::MAX)
    };
    let text = |role, text: String, rect| Placed {
        part: Part::Text {
            role,
            text,
            wrap: Wrap::OneLine,
            align: Align::Start,
        },
        rect,
        id: None,
    };
    let mut manual = |name, sheet, interactive, w: f32, h: f32, items: Vec<Placed<u8>>| {
        all.push(Specimen {
            name,
            sheet,
            interactive,
            items,
            w,
            h,
        })
    };
    let body = Style::of(Role::Body, b, 500.0).line_height();

    let (bar, gap) = (WIZARD_STEP.at(b), WIZARD_STEP_GAP.at(b));
    let each = (wide - gap * 4.0) / 5.0;
    manual(
        "Ход мастера, шаг 3 из 5",
        3,
        false,
        wide,
        bar,
        (0..5)
            .map(|step| Placed {
                part: Part::Progress { done: step < 3 },
                rect: Rect::new((each + gap) * step as f32, 0.0, each, bar),
                id: None,
            })
            .collect(),
    );

    let panel_w = 911.0 * PANEL_SHARE.0;
    let panel_h = 512.0;
    let art = PANEL_ART.at(911.0, 512.0);
    manual(
        "Панель с рисунком",
        4,
        false,
        panel_w,
        panel_h,
        vec![
            Placed {
                part: Part::Panel,
                rect: Rect::new(0.0, 0.0, panel_w, panel_h),
                id: None,
            },
            Placed {
                part: Part::Art(crate::ui::scene::Art::About),
                rect: Rect::new((panel_w - art) / 2.0, (panel_h - art) / 2.0, art, art),
                id: None,
            },
        ],
    );

    for (name, status) in [
        ("Строка состояния: работает", Status::Working),
        ("Строка состояния: остановлена", Status::Idle),
        ("Строка состояния: нужен перезапуск", Status::Restart),
    ] {
        let dot = crate::ui::tokens::STATUS_DOT.at(b);
        manual(
            name,
            4,
            false,
            wide,
            body,
            vec![
                Placed {
                    part: Part::Dot(status),
                    rect: Rect::new(0.0, 0.0, dot, body),
                    id: None,
                },
                text(
                    Role::Body,
                    word("IDS_MENU_RESUME"),
                    Rect::new(dot + CONTROL_GAP.at(b), 0.0, wide, body),
                ),
            ],
        );
    }

    let key = "Pause";
    let code = Style::of(Role::Code, b, 500.0).line_height();
    let key_w = measure_line(Role::Code, key).width + 2.0 * KEY_PAD.h.at(b);
    let key_h = code + 2.0 * KEY_PAD.v.at(b) + crate::ui::tokens::KEY_BOTTOM.0;
    manual(
        "Клавиша",
        4,
        false,
        key_w,
        key_h,
        vec![Placed {
            part: Part::Key {
                label: key.to_owned(),
            },
            rect: Rect::new(0.0, 0.0, key_w, key_h),
            id: None,
        }],
    );
    manual(
        "Метка «новое»",
        4,
        false,
        narrow,
        body,
        vec![Placed {
            part: Part::Badge {
                text: "НОВОЕ".to_owned(),
            },
            rect: Rect::new(0.0, 0.0, narrow, body),
            id: None,
        }],
    );

    // Sheet 5: the corner window and the tray menu, whole — a draft of their composition from the
    // elements; their own sizes come with T-120-14 and T-120-15.
    let cw = CORNER_WIDTH.0;
    let (top, right, bottom, left) = (
        CORNER_PAD.top.0,
        CORNER_PAD.right.0,
        CORNER_PAD.bottom.0,
        CORNER_PAD.left.0,
    );
    let inner = cw - left - right;
    let art = CORNER_ART.0;
    let bh = Style::of(Role::Button, b, 500.0).line_height()
        + 2.0 * crate::ui::tokens::BUTTON_PAD.v.at(b);
    let share = (inner - BUTTON_GAP.at(b)) / 3.0;
    let y_buttons = top + art + 2.0 * crate::ui::tokens::CORNER_GAP.0;
    let ch = y_buttons + bh + bottom;
    manual(
        "Окно в углу (набросок)",
        5,
        false,
        cw,
        ch,
        vec![
            Placed {
                part: Part::Surface(Surface::Block),
                rect: Rect::new(0.0, 0.0, cw, ch),
                id: None,
            },
            Placed {
                part: Part::Art(crate::ui::scene::Art::About),
                rect: Rect::new(left, top, art, art),
                id: None,
            },
            text(
                Role::Strong,
                word("IDS_ABOUT_CAPTION"),
                Rect::new(
                    left + art + crate::ui::tokens::CORNER_GAP.0,
                    top,
                    inner - art,
                    body,
                ),
            ),
            text(
                Role::Small,
                word("IDS_AUTOSTART"),
                Rect::new(
                    left + art + crate::ui::tokens::CORNER_GAP.0,
                    top + body,
                    inner - art,
                    body,
                ),
            ),
            Placed {
                part: Part::Button {
                    kind: ButtonKind::Normal,
                    label: word("IDS_CANCEL"),
                },
                rect: Rect::new(left, y_buttons, share, bh),
                id: Some(1),
            },
            Placed {
                part: Part::Button {
                    kind: ButtonKind::Primary,
                    label: word("IDS_CLOSE"),
                },
                rect: Rect::new(left + share + BUTTON_GAP.at(b), y_buttons, share * 2.0, bh),
                id: Some(2),
            },
        ],
    );

    let mw = MENU_MIN_WIDTH.0;
    let row = MENU_ROW_MIN_HEIGHT.0;
    let rows = [
        word("IDS_MENU_SUSPEND"),
        word("IDS_GROUP_GENERAL"),
        word("IDS_ABOUT_CAPTION"),
        word("IDS_CLOSE"),
    ];
    let mut items = vec![Placed {
        part: Part::Surface(Surface::Block),
        rect: Rect::new(0.0, 0.0, mw, row * rows.len() as f32 + 8.0),
        id: None,
    }];
    for (index, label) in rows.into_iter().enumerate() {
        let y = 4.0 + row * index as f32;
        items.push(Placed {
            part: Part::MenuRow,
            rect: Rect::new(4.0, y, mw - 8.0, row),
            id: Some(index as u8),
        });
        items.push(text(
            Role::Body,
            label,
            Rect::new(16.0, y + (row - body) / 2.0, mw - 32.0, body),
        ));
    }
    manual(
        "Меню значка (набросок)",
        5,
        true,
        mw,
        row * 4.0 + 8.0,
        items,
    );

    all
}

/// Draws a specimen at `x`, `y` in `state`, mirrored when the look is.
pub fn draw(painter: &mut Painter, look: &Look, specimen: &Specimen, x: f32, y: f32, state: State) {
    let mut items = specimen.items.clone();
    if look.rtl {
        crate::ui::layout::mirror(&mut items, specimen.w);
    }
    for item in &items {
        let rect = Rect::new(item.rect.x + x, item.rect.y + y, item.rect.w, item.rect.h);
        elements::draw(painter, look, &item.part, rect, state);
    }
}

#[cfg(feature = "testing")]
pub use sheets::print_sheets;

/// The sheets of pictures: five sheets × light and dark × straight and mirrored × 100 % and 150 %.
#[cfg(feature = "testing")]
mod sheets {
    use super::{Specimen, catalogue, draw, state_name};
    use crate::ui::canvas::{Canvas, Mode};
    use crate::ui::elements::{Look, State};
    use crate::ui::text::{Align, Direction, Style, Text, Wrap};
    use crate::ui::tokens::{Color, Role, Theme, window_b};
    use std::path::Path;

    /// The window the sheets are drawn for: the usual one.
    const SHEET_W: f32 = 911.0;
    const SHEET_H: f32 = 512.0;

    /// The words of one language: an `IDS_` name to the string.
    type Words<'a> = &'a dyn Fn(&str) -> String;

    /// Prints every sheet into `folder`; the words of the straight sheets are `russian`'s, of the
    /// mirrored ones `hebrew`'s. Answers the files written.
    pub fn print_sheets(
        folder: &Path,
        russian: &dyn Fn(&str) -> String,
        hebrew: &dyn Fn(&str) -> String,
    ) -> ::windows::core::Result<Vec<std::path::PathBuf>> {
        std::fs::create_dir_all(folder).map_err(|_| ::windows::core::Error::from_thread())?;
        let b = window_b(SHEET_W, SHEET_H, 1.0);
        let mut written = Vec::new();

        for rtl in [false, true] {
            let (locale, direction, words): (&str, Direction, Words) = if rtl {
                ("he-IL", Direction::RightToLeft, hebrew)
            } else {
                ("ru-RU", Direction::LeftToRight, russian)
            };
            let text = Text::new(locale, direction)?;
            let captions = Text::new("ru-RU", Direction::LeftToRight)?;
            let specimens = catalogue(words, b, &text);

            for sheet in 1..=5u8 {
                for theme in [Theme::Light, Theme::Dark] {
                    for scale in [1.0_f32, 1.5] {
                        let path = folder.join(format!(
                            "sheet{sheet}-{}-{}-{}.png",
                            if theme == Theme::Light {
                                "light"
                            } else {
                                "dark"
                            },
                            if rtl { "rtl" } else { "ltr" },
                            (scale * 100.0) as u32
                        ));
                        let look = Look {
                            theme,
                            rtl,
                            b,
                            h: 512.0,
                            text: &text,
                        };
                        let on: Vec<&Specimen> =
                            specimens.iter().filter(|s| s.sheet == sheet).collect();
                        print(&path, &look, &captions, &on, scale)?;
                        written.push(path);
                    }
                }
            }
        }
        Ok(written)
    }

    /// One sheet: a row per specimen, a column per state, a caption under each.
    fn print(
        path: &Path,
        look: &Look,
        captions: &Text,
        specimens: &[&Specimen],
        scale: f32,
    ) -> ::windows::core::Result<()> {
        let (margin, gap) = (24.0, 20.0);
        let caption = Style::of(Role::Small, look.b, look.h);
        let cell = specimens.iter().map(|s| s.w).fold(0.0_f32, f32::max) + gap;
        let columns = State::ALL.len() as f32;
        let width = margin * 2.0 + cell * columns;
        let heights: Vec<f32> = specimens
            .iter()
            .map(|s| s.h + caption.line_height() * 2.0 + gap)
            .collect();
        let height = margin * 2.0 + heights.iter().sum::<f32>();

        let mut canvas = Canvas::in_memory(width, height, scale, Mode::Direct2D)?;
        canvas.frame(|painter| {
            painter.clear(Color::Bg.get(look.theme));
            let mut y = margin;
            for (specimen, h) in specimens.iter().zip(&heights) {
                let states: &[State] = if specimen.interactive {
                    State::ALL
                } else {
                    &[State::NORMAL]
                };
                for (column, state) in states.iter().enumerate() {
                    let x = margin + cell * column as f32;
                    draw(painter, look, specimen, x, y, *state);
                    let label = format!("{} — {}", specimen.name, state_name(*state));
                    if let Ok(laid) =
                        captions.lay(&label, caption, Wrap::Words, Align::Start, cell - gap)
                    {
                        painter.text(
                            &laid,
                            x,
                            y + specimen.h + gap / 3.0,
                            Color::Ink2.get(look.theme),
                        );
                    }
                }
                y += h;
            }
        });
        canvas.save_png(path)?;
        Ok(())
    }
}
