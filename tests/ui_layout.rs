//! The layout of the interface shell, `src\ui\layout.rs` (stage Э120, T-120-5), on the stand-in
//! measure of text: every container and every rule of wrapping, the size and class of a window, the
//! row of buttons in one place on every step, the floor of the type size, and the mirror — an exact
//! reflection for every window description.

use lang_switcher::diag;
use lang_switcher::ui::canvas::Rect;
use lang_switcher::ui::fit;
use lang_switcher::ui::layout::{
    Class, Layout, Part, Placed, WindowSize, columns_fit, lay_out, window_size,
};
use lang_switcher::ui::scene::{Art, Brand, ButtonKind, Node, Surface, Window, WindowKind};
use lang_switcher::ui::text::{FixedAdvance, Measure, Style};
use lang_switcher::ui::tokens::{
    BLOCK_GAP, BLOCK_PAD, BLOCK_ROW_GAP, BUTTON_GAP, CONTENT_PAD, PAIR_MIN_HALF, Role, TYPE_FLOOR,
};

/// Every character is half the type size wide.
const MEASURE: FixedAdvance = FixedAdvance(0.5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Id {
    Close,
    Site,
    Copy,
    Autostart,
    Sound,
    Theme,
    Language,
    Sections,
    Back,
    Next,
    Cancel,
}

fn text(role: Role, s: &str) -> Node<Id> {
    Node::Text(role, s.to_owned())
}

fn button(id: Id, label: &str, kind: ButtonKind) -> Node<Id> {
    Node::Button {
        id,
        label: label.to_owned(),
        kind,
        browser: false,
    }
}

/// A window of `body` with no picture and no buttons.
fn bare(body: Node<Id>) -> Window<Id> {
    Window {
        kind: WindowKind::Ordinary,
        art: None,
        brand: None,
        disabled: Vec::new(),
        body,
        buttons: Vec::new(),
    }
}

/// The usual window, 911 × 512, ordinary.
fn usual() -> WindowSize {
    WindowSize {
        w: 911.0,
        h: 512.0,
        class: Class::Ordinary,
        scrolls: false,
    }
}

fn lay(window: &Window<Id>, size: WindowSize) -> Layout<Id> {
    lay_out(window, size, 1.0, false, &MEASURE)
}

fn rects_of(layout: &Layout<Id>, wanted: fn(&Part) -> bool) -> Vec<Rect> {
    layout
        .items
        .iter()
        .filter(|item| wanted(&item.part))
        .map(|item| item.rect)
        .collect()
}

fn texts(layout: &Layout<Id>) -> Vec<Rect> {
    rects_of(layout, |part| matches!(part, Part::Text { .. }))
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

// ---- The size of a window -------------------------------------------------------------------

#[test]
fn the_size_of_a_window_is_two_thirds_of_the_work_area_within_its_limits() {
    let cases = [
        (
            (1366.0, 728.0),
            (1366.0 * 2.0 / 3.0, 540.0),
            Class::Ordinary,
            false,
        ),
        (
            (1920.0, 1040.0),
            (1280.0, 1040.0 * 2.0 / 3.0),
            Class::Large,
            false,
        ),
        ((700.0, 560.0), (700.0, 540.0), Class::Narrow, false),
        ((2560.0, 1400.0), (1280.0, 720.0), Class::Large, false),
        ((3840.0, 2120.0), (1280.0, 720.0), Class::Large, false),
        ((640.0, 360.0), (640.0, 360.0), Class::Narrow, true),
        ((930.0, 750.0), (900.0, 540.0), Class::Ordinary, false),
    ];

    for ((aw, ah), (w, h), class, scrolls) in cases {
        let size = window_size(aw, ah);
        assert!(
            close(size.w, w) && close(size.h, h) && size.class == class && size.scrolls == scrolls,
            "work area {aw}×{ah}: {size:?}, expected {w}×{h} {class:?} scrolls {scrolls}"
        );
        assert!(
            size.w <= 1280.0 && size.h <= 720.0,
            "{aw}×{ah}: above 1280×720"
        );
    }
}

// ---- Containers -----------------------------------------------------------------------------

#[test]
fn a_column_stands_its_blocks_a_block_gap_apart_and_rows_a_line_gap_apart() {
    for (node, gap) in [
        (
            Node::Column(vec![text(Role::Body, "one"), text(Role::Body, "two")]),
            BLOCK_GAP,
        ),
        (
            Node::Rows(vec![text(Role::Body, "one"), text(Role::Body, "two")]),
            BLOCK_ROW_GAP,
        ),
    ] {
        let layout = lay(&bare(node), usual());
        let [first, second] = texts(&layout)[..] else {
            panic!("two texts")
        };
        assert!(
            close(second.y, first.bottom() + gap.at(layout.b)),
            "{first:?} {second:?}"
        );
        assert!(close(second.x, first.x));
    }
}

#[test]
fn a_row_wraps_like_the_words_of_a_line() {
    let link = |id, s: &str| Node::Link {
        id,
        text: s.to_owned(),
    };
    // Each link is 20 characters, 10 B at 0.5 B a character; the column is 40 B wide at most.
    let long = "a".repeat(30);
    let layout = lay(
        &bare(Node::Row(vec![
            link(Id::Site, &long),
            link(Id::Copy, &long),
            link(Id::Close, &long),
        ])),
        usual(),
    );
    let links = rects_of(&layout, |part| matches!(part, Part::Link { .. }));
    let b = layout.b;

    assert!(close(links[1].y, links[0].y), "two fit on the first line");
    assert!(close(links[1].x, links[0].right() + BUTTON_GAP.at(b)));
    assert!(links[2].y > links[0].bottom(), "the third wraps: {links:?}");
    assert!(close(links[2].x, links[0].x));
}

/// The rule of the pair: side by side while each half is 20 B, one under another at 19,9 B; the
/// same for three columns.
#[test]
fn equal_columns_stand_side_by_side_down_to_twenty_b_each() {
    assert_eq!(PAIR_MIN_HALF.0, 15.0);
    let b = 12.0;
    let gap = BLOCK_GAP.at(b);

    for (each, beside) in [(15.0, true), (14.9, false)] {
        assert_eq!(
            columns_fit(2, 2.0 * each * b + gap, b),
            beside,
            "a pair at {each} B a half"
        );
        assert_eq!(
            columns_fit(3, 3.0 * each * b + 2.0 * gap, b),
            beside,
            "three at {each} B each"
        );
    }
}

/// In the usual window (911 × 512, a column of 35 B) a pair stands side by side, 17 B a half, and a
/// triple — under 15 B each — falls to one column a block's gap apart, at the full width (208.168).
#[test]
fn in_the_usual_window_a_pair_stands_side_by_side_and_a_triple_falls_to_a_column() {
    let cell = |s: &str| Node::Surface(Surface::Nested, Box::new(text(Role::Body, s)));
    let pair = lay(
        &bare(Node::Pair(Box::new(cell("a")), Box::new(cell("b")))),
        usual(),
    );
    let halves = rects_of(&pair, |part| matches!(part, Part::Surface(_)));
    assert!(
        close(halves[0].y, halves[1].y) && halves[1].x > halves[0].right(),
        "{halves:?}"
    );
    assert!(
        close(halves[0].h, halves[1].h),
        "the halves share one height"
    );

    let triple = Node::Triple(
        Box::new(cell("a")),
        Box::new(cell("b")),
        Box::new(cell("c")),
    );
    let layout = lay(&bare(triple), usual());
    let cells = rects_of(&layout, |part| matches!(part, Part::Surface(_)));
    let gap = BLOCK_GAP.at(layout.b);

    assert!(
        cells
            .windows(2)
            .all(|pair| close(pair[1].y, pair[0].bottom() + gap))
    );
    assert!(
        cells
            .iter()
            .all(|cell| close(cell.w, cells[0].w) && close(cell.x, cells[0].x))
    );
}

#[test]
fn a_fill_takes_the_rest_of_its_column() {
    let body = Node::Column(vec![
        text(Role::Body, "top"),
        Node::Fill(Box::new(Node::Surface(
            Surface::Block,
            Box::new(text(Role::Body, "middle")),
        ))),
        text(Role::Body, "bottom"),
    ]);
    let layout = lay(&bare(body), usual());
    let block = rects_of(&layout, |part| matches!(part, Part::Surface(_)))[0];
    let bottom = *texts(&layout).last().expect("the last text");
    let pad = CONTENT_PAD.v.at(layout.b);

    assert!(
        close(bottom.bottom(), usual().h - pad),
        "the column reaches its floor: {bottom:?}"
    );
    assert!(
        close(bottom.y, block.bottom() + BLOCK_GAP.at(layout.b)),
        "{block:?}"
    );
}

#[test]
fn a_surface_pads_what_it_holds() {
    let layout = lay(
        &bare(Node::Surface(
            Surface::Block,
            Box::new(text(Role::Body, "inside")),
        )),
        usual(),
    );
    let block = rects_of(&layout, |part| matches!(part, Part::Surface(_)))[0];
    let inside = texts(&layout)[0];
    let (v, h) = (BLOCK_PAD.v.at(layout.b), BLOCK_PAD.h.at(layout.b));

    assert!(close(inside.x, block.x + h) && close(inside.y, block.y + v));
    assert!(close(inside.w, block.w - 2.0 * h) && close(block.h, inside.h + 2.0 * v));
}

/// The steps of a wizard take the height of the tallest, and the row of buttons stands in the same
/// place on every step.
#[test]
fn the_steps_of_a_wizard_are_as_tall_as_the_tallest_and_the_buttons_do_not_move() {
    let steps = |current| Window {
        kind: WindowKind::Wizard,
        art: None,
        brand: None,
        disabled: Vec::new(),
        body: Node::Column(vec![
            Node::Steps(
                current,
                vec![
                    text(Role::Body, "short"),
                    Node::Rows(vec![
                        text(Role::Body, "a"),
                        text(Role::Body, "b"),
                        text(Role::Body, "c"),
                    ]),
                ],
            ),
            text(Role::Small, "under the steps"),
        ]),
        buttons: vec![
            button(Id::Cancel, "Cancel", ButtonKind::Normal),
            button(Id::Back, "Back", ButtonKind::Normal),
            button(Id::Next, "Next", ButtonKind::Primary),
        ],
    };

    let first = lay(&steps(0), usual());
    let second = lay(&steps(1), usual());
    let under = |layout: &Layout<Id>| *texts(layout).last().expect("the line under the steps");
    let buttons =
        |layout: &Layout<Id>| rects_of(layout, |part| matches!(part, Part::Button { .. }));

    assert!(
        close(under(&first).y, under(&second).y),
        "the line under the steps stays put"
    );
    assert_eq!(buttons(&first), buttons(&second), "the buttons stay put");
}

// ---- The row of buttons ---------------------------------------------------------------------

#[test]
fn the_buttons_press_to_the_right_with_the_main_one_rightmost_and_stack_main_first() {
    let row = |label_length: usize| {
        let label = "x".repeat(label_length);
        bare(Node::Buttons(vec![
            button(Id::Close, &label, ButtonKind::Primary),
            button(Id::Copy, &label, ButtonKind::Normal),
        ]))
    };
    let order = |layout: &Layout<Id>| {
        layout
            .items
            .iter()
            .filter(|item| matches!(item.part, Part::Button { .. }))
            .map(|item| (item.id.expect("an id"), item.rect))
            .collect::<Vec<_>>()
    };

    let fits = lay(&row(4), usual());
    let fits = order(&fits);
    assert_eq!(fits[1].0, Id::Close, "the main one is the rightmost");
    assert!(fits[1].1.x > fits[0].1.x && close(fits[0].1.y, fits[1].1.y));

    let stacked = lay(&row(60), usual());
    let b = stacked.b;
    let stacked = order(&stacked);
    assert_eq!(stacked[0].0, Id::Close, "the main one on top");
    assert!(stacked[1].1.y > stacked[0].1.bottom(), "one under another");
    assert!(close(stacked[0].1.w, stacked[1].1.w), "across the width");
    let _ = b;
}

/// The second step of the rule of the pair: when the content does not fit the height, the buttons
/// of an action card move beside its text.
#[test]
fn the_buttons_of_an_action_card_move_beside_its_text_when_the_window_is_tight() {
    let card = |lines: usize| {
        let mut column = vec![Node::Action {
            title: "Title".to_owned(),
            text: "Text".to_owned(),
            buttons: vec![button(Id::Copy, "Copy", ButtonKind::Normal)],
        }];
        column.extend((0..lines).map(|_| text(Role::Body, "filler")));
        bare(Node::Column(column))
    };
    let button_and_text = |layout: &Layout<Id>| {
        let button = rects_of(layout, |part| matches!(part, Part::Button { .. }))[0];
        let text = texts(layout)[1];
        (button, text)
    };

    let roomy = lay(&card(2), usual());
    let (b, t) = button_and_text(&roomy);
    assert!(b.y >= t.bottom(), "roomy: the button under the text");

    let tight = lay(&card(12), usual());
    let (b, t) = button_and_text(&tight);
    assert!(
        b.x >= t.right() && b.y < t.bottom(),
        "tight: the button beside the text, {b:?} {t:?}"
    );
}

// ---- Controls -------------------------------------------------------------------------------

#[test]
fn a_switch_stands_at_the_end_of_its_line_and_a_check_box_before_its_label() {
    let body = Node::Rows(vec![
        Node::Toggle {
            id: Id::Autostart,
            label: "Start with Windows".to_owned(),
            hint: Some("hint".to_owned()),
            on: true,
        },
        Node::Check {
            id: Id::Sound,
            label: "Sound".to_owned(),
            on: false,
        },
    ]);
    let layout = lay(&bare(body), usual());
    let find = |wanted: fn(&Part) -> bool| rects_of(&layout, wanted);
    let (toggle, check) = (
        find(|p| matches!(p, Part::Toggle { .. }))[0],
        find(|p| matches!(p, Part::Check { .. }))[0],
    );
    let hits = find(|p| matches!(p, Part::Hit));
    let labels = texts(&layout);

    assert!(
        close(toggle.right(), hits[0].right()),
        "the switch at the end of its line"
    );
    assert!(labels[0].right() <= toggle.x, "the label before the switch");
    assert!(
        close(check.x, hits[1].x) && check.right() <= labels[2].x,
        "the box before its label"
    );
    assert!(
        hits.iter().all(|hit| close(hit.w, hits[0].w)),
        "the whole line is pressed"
    );
    assert!(
        hits[0].h >= labels[0].h + labels[1].h,
        "the label and the hint are inside the line"
    );
}

/// A segmented switch stands beside its label, on a line of its own when it does not fit beside
/// it, and becomes a choice field when it does not fit even there.
#[test]
fn a_segmented_switch_moves_to_its_own_line_and_then_becomes_a_choice_field() {
    let segments = |option_length: usize| {
        bare(Node::Segments {
            id: Id::Theme,
            label: "Theme of the windows".to_owned(),
            options: (0..3)
                .map(|n| format!("{n}{}", "o".repeat(option_length)))
                .collect(),
            selected: 1,
        })
    };
    let kinds = |layout: &Layout<Id>| {
        (
            rects_of(layout, |p| matches!(p, Part::Track)),
            rects_of(layout, |p| matches!(p, Part::Field { .. })),
            texts(layout)[0],
        )
    };

    let (track, field, label) = kinds(&lay(&segments(2), usual()));
    assert!(
        field.is_empty()
            && close(track[0].y + track[0].h / 2.0, label.y + label.h / 2.0)
            && track[0].x >= label.right()
    );

    let (track, field, label) = kinds(&lay(&segments(20), usual()));
    assert!(
        field.is_empty() && track[0].y > label.bottom(),
        "on a line of its own: {track:?}"
    );

    let (track, field, _) = kinds(&lay(&segments(40), usual()));
    assert!(track.is_empty() && field.len() == 1, "a choice field");
}

#[test]
fn the_sections_are_a_column_and_in_a_narrow_window_a_row_of_tabs() {
    let sections = bare(Node::Sections {
        id: Id::Sections,
        items: [
            "General",
            "Hotkeys",
            "Layouts",
            "From the author",
            "Diagnostics",
        ]
        .map(str::to_owned)
        .to_vec(),
        current: 0,
    });
    let narrow = WindowSize {
        w: 640.0,
        h: 500.0,
        class: Class::Narrow,
        scrolls: false,
    };
    let tabs = |layout: &Layout<Id>| rects_of(layout, |p| matches!(p, Part::Section { .. }));

    let wide = tabs(&lay(&sections, usual()));
    assert!(
        wide.windows(2)
            .all(|pair| close(pair[0].x, pair[1].x) && pair[1].y > pair[0].y)
    );

    let row = tabs(&lay(&sections, narrow));
    assert!(
        close(row[0].y, row[1].y) && row[1].x > row[0].right(),
        "a row of tabs: {row:?}"
    );
}

#[test]
fn a_narrow_window_puts_the_circle_beside_the_header_and_the_settings_do_not() {
    let header = Node::Header {
        eyebrow: Some("Lang Switcher 1.8.0".to_owned()),
        title: "About".to_owned(),
        subtitle: None,
    };
    let window = |kind| Window {
        kind,
        art: Some(Art::About),
        brand: Some(Brand {
            name: "Panda Koder".to_owned(),
            address: "panda-pishet-kod.dev".to_owned(),
            link: Id::Site,
        }),
        disabled: Vec::new(),
        body: header.clone(),
        buttons: Vec::new(),
    };
    let narrow = WindowSize {
        w: 640.0,
        h: 500.0,
        class: Class::Narrow,
        scrolls: false,
    };
    let art = |layout: &Layout<Id>| rects_of(layout, |p| matches!(p, Part::Art(_)));

    let wide = lay(&window(WindowKind::Ordinary), usual());
    assert_eq!(
        rects_of(&wide, |p| matches!(p, Part::Panel)).len(),
        1,
        "the panel"
    );

    let about = lay(&window(WindowKind::Ordinary), narrow);
    assert!(
        rects_of(&about, |p| matches!(p, Part::Panel)).is_empty(),
        "no panel"
    );
    let circle = art(&about)[0];
    assert!(
        texts(&about).iter().all(|text| text.x >= circle.right()),
        "the circle beside the header"
    );

    let settings = lay(&window(WindowKind::Settings), narrow);
    assert!(art(&settings).is_empty(), "no circle in the settings");
}

// ---- Fitting --------------------------------------------------------------------------------

#[test]
fn the_shortfall_says_how_much_does_not_fit_and_the_factor_goes_down_to_the_floor() {
    let lines = |count: usize| {
        bare(Node::Column(
            (0..count).map(|_| text(Role::Body, "line")).collect(),
        ))
    };
    let fits = lay(&lines(3), usual());
    assert!(fits.shortfall.fits(), "{:?}", fits.shortfall);

    let tall = lay(&lines(30), usual());
    assert!(tall.shortfall.h > 0.0, "{:?}", tall.shortfall);

    let lowered = diag::Operation::from_name(fit::TYPE_SIZE_LOWERED);
    assert_ne!(
        lowered,
        diag::Operation::UNLISTED,
        "the name is in the vocabulary"
    );
    let count = || {
        diag::snapshot()
            .iter()
            .filter(|event| event.operation == lowered)
            .count()
    };
    let before = count();

    let some = fit::emergency(&lines(14), usual(), 1.0, false, &MEASURE);
    assert!(
        some.k < 1.0 && some.shortfall.fits(),
        "lowered until it fits: k {}",
        some.k
    );
    assert!(count() > before, "the journal says so");

    let floor = fit::emergency(&lines(200), usual(), 1.0, false, &MEASURE);
    assert!(
        close(floor.b, TYPE_FLOOR.0),
        "no lower than 12: B {}",
        floor.b
    );
    let style = Style::of(Role::Small, floor.b, usual().h);
    assert!(style.size >= 12.0);
    let _ = MEASURE.measure("x", style, lang_switcher::ui::text::Wrap::OneLine, 10.0);
}

// ---- The mirror -----------------------------------------------------------------------------

/// The descriptions the mirror test goes over: the probe below, and — as they appear — every window
/// of `src\ui\windows\`.
fn every_window() -> Vec<Window<Id>> {
    let probe = Window {
        kind: WindowKind::Ordinary,
        art: Some(Art::About),
        brand: Some(Brand {
            name: "Panda Koder".to_owned(),
            address: "panda-pishet-kod.dev".to_owned(),
            link: Id::Site,
        }),
        disabled: Vec::new(),
        body: Node::Column(vec![
            Node::Header {
                eyebrow: Some("Lang Switcher 1.8.0".to_owned()),
                title: "Settings".to_owned(),
                subtitle: Some("A line under the title".to_owned()),
            },
            Node::Sections {
                id: Id::Sections,
                items: ["General", "Hotkeys", "Diagnostics"]
                    .map(str::to_owned)
                    .to_vec(),
                current: 1,
            },
            Node::Pair(
                Box::new(Node::Surface(
                    Surface::Block,
                    Box::new(Node::Rows(vec![
                        Node::Toggle {
                            id: Id::Autostart,
                            label: "Start with Windows".to_owned(),
                            hint: None,
                            on: true,
                        },
                        Node::Check {
                            id: Id::Sound,
                            label: "Sound".to_owned(),
                            on: true,
                        },
                    ])),
                )),
                Box::new(Node::Surface(
                    Surface::Nested,
                    Box::new(Node::Rows(vec![
                        Node::Segments {
                            id: Id::Theme,
                            label: "Theme".to_owned(),
                            options: ["Light", "Dark", "System"].map(str::to_owned).to_vec(),
                            selected: 2,
                        },
                        Node::Choice {
                            id: Id::Language,
                            label: "Language".to_owned(),
                            options: vec!["English".to_owned(), "Deutsch".to_owned()],
                            selected: 0,
                        },
                    ])),
                )),
            ),
            Node::Row(vec![Node::Link {
                id: Id::Site,
                text: "The site".to_owned(),
            }]),
        ]),
        buttons: vec![
            button(Id::Copy, "Copy", ButtonKind::Quiet),
            button(Id::Close, "Close", ButtonKind::Primary),
        ],
    };

    vec![probe]
}

/// **The mirror is an exact reflection**: every rectangle of the right-to-left layout equals the
/// left-to-right one reflected in the vertical axis of the window — for every description, at a
/// wide and a narrow size — and the order of Tab does not change.
#[test]
fn the_mirror_reflects_every_rectangle_and_keeps_the_order_of_tab() {
    let narrow = WindowSize {
        w: 640.0,
        h: 500.0,
        class: Class::Narrow,
        scrolls: false,
    };
    let mut faults = Vec::new();

    for window in every_window() {
        for size in [usual(), narrow] {
            let straight = lay_out(&window, size, 1.0, false, &MEASURE);
            let mirrored = lay_out(&window, size, 1.0, true, &MEASURE);
            assert_eq!(straight.items.len(), mirrored.items.len());
            assert_eq!(
                straight.focus_order(),
                mirrored.focus_order(),
                "the order of Tab"
            );

            for (left, right) in straight.items.iter().zip(&mirrored.items) {
                let Placed { part, rect, .. } = left;
                let reflected = Rect::new(size.w - rect.x - rect.w, rect.y, rect.w, rect.h);
                if right.part != *part
                    || !(close(right.rect.x, reflected.x)
                        && close(right.rect.y, reflected.y)
                        && close(right.rect.w, reflected.w)
                        && close(right.rect.h, reflected.h))
                {
                    faults.push(format!(
                        "{} at {}×{}: {:?}, the reflection is {:?}",
                        part.name(),
                        size.w,
                        size.h,
                        right.rect,
                        reflected
                    ));
                }
            }
        }
    }

    assert!(faults.is_empty(), "not mirrored:\n{}", faults.join("\n"));
}

/// The mirror test over the windows of `src\ui\windows\`: «О программе» in Hebrew and Arabic, at the
/// usual and the narrow size (the strings themselves may be empty in a test process — the
/// rectangles are what is compared).
#[test]
fn the_mirror_reflects_the_window_descriptions_of_the_shell() {
    use lang_switcher::settings::Language;
    use lang_switcher::theme::ThemeSetting;
    use lang_switcher::ui::host::Content;
    use lang_switcher::ui::windows::about::About;

    let narrow = WindowSize {
        w: 640.0,
        h: 500.0,
        class: Class::Narrow,
        scrolls: false,
    };
    for language in [Language::He, Language::Ar] {
        let window = About {
            language,
            key: "Pause".to_owned(),
            theme: ThemeSetting::System,
        }
        .describe();
        for size in [usual(), narrow] {
            let straight = lay_out(&window, size, 1.0, false, &MEASURE);
            let mirrored = lay_out(&window, size, 1.0, true, &MEASURE);
            assert_eq!(straight.focus_order(), mirrored.focus_order());
            for (left, right) in straight.items.iter().zip(&mirrored.items) {
                let reflected = size.w - left.rect.x - left.rect.w;
                assert!(
                    close(right.rect.x, reflected) && close(right.rect.y, left.rect.y),
                    "{language:?} {} at {}",
                    left.part.name(),
                    size.w
                );
            }
        }
    }
}

/// ◆ Content that does not fill the room stands in the middle of the free height above the row of
/// buttons, as the mock-ups centre it; a window with a fill starts at the top.
#[test]
fn content_that_does_not_fill_the_room_stands_in_its_middle() {
    let window = Window {
        buttons: vec![button(Id::Close, "Close", ButtonKind::Primary)],
        ..bare(text(Role::Body, "A short window"))
    };
    let layout = lay(&window, usual());
    let line = texts(&layout)[0];
    let rule = rects_of(&layout, |part| matches!(part, Part::ButtonRule))[0];
    let top = CONTENT_PAD.v.at(layout.b);
    let floor = rule.y - BLOCK_GAP.at(layout.b);

    assert!(
        close(line.y - top, floor - line.bottom()),
        "as much room above as below: {line:?}, floor {floor}"
    );
}

/// The line under the picture: the name, then the address beside it on the same line; mirrored, the
/// name stands at the right.
#[test]
fn the_brand_line_is_the_name_and_the_address_beside_it() {
    let window = Window {
        art: Some(Art::About),
        brand: Some(Brand {
            name: "Panda Koder".to_owned(),
            address: "panda-pishet-kod.dev".to_owned(),
            link: Id::Site,
        }),
        // A button, so that Tab has another stop than the address.
        buttons: vec![Node::Button {
            id: Id::Close,
            label: "Close".to_owned(),
            kind: ButtonKind::Primary,
            browser: false,
        }],
        ..bare(text(Role::Body, "Body"))
    };
    for rtl in [false, true] {
        let layout = lay_out(&window, usual(), 1.0, rtl, &MEASURE);
        let name = rects_of(&layout, |part| matches!(part, Part::BrandName { .. }))[0];
        let address = layout
            .items
            .iter()
            .find(|item| {
                matches!(
                    &item.part,
                    Part::Link {
                        role: Role::Brand,
                        ..
                    }
                ) && item.id == Some(Id::Site)
            })
            .expect("the address is a link to the page of the program")
            .rect;
        assert!(close(name.y, address.y), "one line");
        assert_eq!(
            layout.focus_order().last(),
            Some(&Id::Site),
            "the address is not the last stop of Tab"
        );
        if rtl {
            assert!(
                address.right() <= name.x,
                "mirrored: the address left of the name"
            );
        } else {
            assert!(address.x >= name.right(), "the address after the name");
        }
    }
}
