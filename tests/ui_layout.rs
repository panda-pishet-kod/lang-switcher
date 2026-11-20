//! The layout of the interface shell, `src\ui\layout.rs` (stage Э120, T-120-5), on the stand-in
//! measure of text: every container and every rule of wrapping, the size and class of a window, the
//! row of buttons in one place on every step, the floor of the type size, and the mirror — an exact
//! reflection for every window description.

use lang_switcher::diag;
use lang_switcher::ui::canvas::Rect;
use lang_switcher::ui::fit;
use lang_switcher::ui::layout::{
    Class, Concession, Layout, Part, Placed, WindowSize, columns_fit, lay_out, window_size,
};
use lang_switcher::ui::scene::{
    Art, Brand, ButtonKind, FieldKind, Node, Surface, Window, WindowKind,
};
use lang_switcher::ui::text::{FixedAdvance, Measure, Style, Wrap};
use lang_switcher::ui::tokens::{
    BADGE_GAP, BLOCK_GAP, BLOCK_PAD, BLOCK_ROW_GAP, BUTTON_GAP, CONTENT_PAD, PAIR_MIN_HALF, Role,
    TYPE_FLOOR,
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
        words: s.to_owned(),
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
fn the_buttons_press_to_the_right_with_the_main_one_rightmost_and_give_way_by_step_1() {
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
    assert_eq!(fits.step, None, "a row that fits gives way in nothing");
    let fits = order(&fits);
    assert_eq!(fits[1].0, Id::Close, "the main one is the rightmost");
    assert!(fits[1].1.x > fits[0].1.x && close(fits[0].1.y, fits[1].1.y));

    // §3а step 1: each fits the width, both do not — two rows, the main one in the lower at its
    // edge.
    let two = lay(&row(40), usual());
    assert_eq!(two.step, Some(Concession::Buttons));
    let two = order(&two);
    let (copy, main) = (two[0], two[1]);
    assert_eq!((copy.0, main.0), (Id::Copy, Id::Close));
    assert!(
        main.1.y > copy.1.bottom(),
        "the main one in the lower row: {two:?}"
    );
    assert!(
        close(main.1.right(), copy.1.right()),
        "both at the edge: {two:?}"
    );

    // Wider than the width each — one under another across it, the main one on top.
    let stacked = lay(&row(90), usual());
    let stacked = order(&stacked);
    assert_eq!(stacked[0].0, Id::Close, "the main one on top");
    assert!(stacked[1].1.y > stacked[0].1.bottom(), "one under another");
    assert!(close(stacked[0].1.w, stacked[1].1.w), "across the width");
}

/// §3а step 3 (the second step of the rule of the pair before the order): in a pair standing side
/// by side, when the content does not fit the height, the buttons of an action card move beside its
/// text; a card alone keeps them under its text.
#[test]
fn the_buttons_of_a_card_in_a_pair_move_beside_its_text_by_step_3() {
    let card = |id| Node::Action {
        title: "Title".to_owned(),
        words: "Text".to_owned(),
        buttons: vec![button(id, "Copy", ButtonKind::Normal)],
    };
    let window = |lines: usize, pair: bool| {
        let mut column = vec![if pair {
            Node::Pair(Box::new(card(Id::Copy)), Box::new(card(Id::Close)))
        } else {
            card(Id::Copy)
        }];
        column.extend((0..lines).map(|_| text(Role::Body, "filler")));
        bare(Node::Column(column))
    };
    let button_and_text = |layout: &Layout<Id>| {
        let button = rects_of(layout, |part| matches!(part, Part::Button { .. }))[0];
        let text = texts(layout)[1];
        (button, text)
    };

    let roomy = lay(&window(2, true), usual());
    let (b, t) = button_and_text(&roomy);
    assert!(b.y >= t.bottom(), "roomy: the button under the text");

    let tight = lay(&window(11, true), usual());
    assert_eq!(tight.step, Some(Concession::Card));
    let (b, t) = button_and_text(&tight);
    assert!(
        b.x >= t.right() && b.y < t.bottom(),
        "tight: the button beside the text, {b:?} {t:?}"
    );

    let alone = lay(&window(11, false), usual());
    let (b, t) = button_and_text(&alone);
    assert!(b.y >= t.bottom(), "a card alone: the button under the text");
}

/// §3а step 2: a pair stands in a column when a half is narrower than its widest unbreakable
/// content — here a button — although the half is not narrower than its least, 15 B.
#[test]
fn a_pair_whose_half_is_narrower_than_its_button_stands_in_a_column_by_step_2() {
    let half = |id, label: &str| Node::Action {
        title: "Title".to_owned(),
        words: "Text".to_owned(),
        buttons: vec![button(id, label, ButtonKind::Normal)],
    };
    let window = |label: &str| {
        bare(Node::Pair(
            Box::new(half(Id::Copy, label)),
            Box::new(half(Id::Close, "Close")),
        ))
    };
    let titles = |layout: &Layout<Id>| {
        texts(layout)
            .into_iter()
            .filter(|rect| rect.h > 0.0)
            .collect::<Vec<_>>()
    };

    let side = lay(&window("Copy"), usual());
    assert!(columns_fit(
        2,
        usual().w - 2.0 * CONTENT_PAD.h.at(side.b),
        side.b
    ));
    let t = titles(&side);
    assert!(close(t[0].y, t[2].y), "side by side: {t:?}");

    let wide = lay(&window(&"w".repeat(50)), usual());
    assert_eq!(wide.step, Some(Concession::Pair));
    assert!(wide.shortfall.fits(), "{:?}", wide.shortfall);
    let t = titles(&wide);
    assert!(t[2].y > t[0].bottom(), "in a column: {t:?}");
}

/// §3а step 4: in a narrow window that does not fit, the circle with the panda leaves the header.
#[test]
fn a_narrow_window_loses_the_circle_by_step_4() {
    let narrow = WindowSize {
        w: 640.0,
        h: 500.0,
        class: Class::Narrow,
        scrolls: false,
    };
    let window = |title: &str| Window {
        art: Some(Art::About),
        ..bare(Node::Header {
            eyebrow: None,
            title: title.to_owned(),
            subtitle: None,
        })
    };
    let pictures = |layout: &Layout<Id>| rects_of(layout, |part| matches!(part, Part::Art(_)));

    let roomy = lay(&window("About"), narrow);
    assert_eq!(roomy.step, None);
    assert_eq!(pictures(&roomy).len(), 1, "the circle beside the header");

    // A word that fits the column, but not beside the circle: the room of the title with the
    // circle and without it, read from the roomy window.
    let (circle, title) = (pictures(&roomy)[0], texts(&roomy)[0]);
    let (beside, room) = (title.w, title.right() - circle.x);
    let advance = MEASURE.0 * Role::Title.style().size.at(roomy.b);
    let word = "w".repeat(((beside + room) / 2.0 / advance) as usize);
    let tight = lay(&window(&word), narrow);
    assert_eq!(tight.step, Some(Concession::Circle));
    assert!(tight.shortfall.fits(), "{:?}", tight.shortfall);
    assert!(pictures(&tight).is_empty(), "the circle left");
}

/// §3а step 5: the label of a button wider than its place goes on to a second line, and the button
/// grows — the last step; a word wider than the place is beyond it.
#[test]
fn a_label_wider_than_its_button_goes_on_to_a_second_line_by_step_5() {
    let label = "word ".repeat(30);
    let one = lay(&bare(button(Id::Copy, "Copy", ButtonKind::Normal)), usual());
    let one_line = rects_of(&one, |part| matches!(part, Part::Button { .. }))[0].h;

    let long = lay(
        &bare(button(Id::Copy, label.trim(), ButtonKind::Normal)),
        usual(),
    );
    assert_eq!(long.step, Some(Concession::Wrap));
    assert!(long.shortfall.fits(), "{:?}", long.shortfall);
    let rect = rects_of(&long, |part| matches!(part, Part::Button { .. }))[0];
    assert!(
        rect.h > one_line,
        "the button grew: {rect:?}, one line {one_line}"
    );

    let word = lay(
        &bare(button(Id::Copy, &"w".repeat(200), ButtonKind::Normal)),
        usual(),
    );
    assert_eq!(word.step, Some(Concession::Wrap));
    assert!(
        !word.shortfall.fits(),
        "an unbreakable word does not fit: {:?}",
        word.shortfall
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
            badge: None,
        },
        Node::Check {
            id: Id::Sound,
            label: "Sound".to_owned(),
            on: false,
            hint: None,
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
                            badge: Some("new".to_owned()),
                        },
                        Node::Check {
                            id: Id::Sound,
                            label: "Sound".to_owned(),
                            on: true,
                            hint: None,
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
                words: "The site".to_owned(),
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
/// left-to-right one reflected in the vertical axis of the window (what lies over the picture of the
/// panel moves with the picture instead, T-120-25) — for every description, at a wide and a narrow
/// size — and the order of Tab does not change.
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
                let Placed { part, .. } = left;
                let reflected = mirrored.reflection_of(&straight, left);
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
                let reflected = mirrored.reflection_of(&straight, left).x;
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

/// The example with a key: the word typed, the key, the arrow, the word fixed, in this order from
/// the start; in Hebrew the row mirrors and the arrow points left.
#[test]
fn the_example_with_a_key_reads_the_way_the_language_reads() {
    let window = bare(Node::Example {
        typed: "ghbdtn".to_owned(),
        key: "Pause".to_owned(),
        fixed: "привет".to_owned(),
    });
    for rtl in [false, true] {
        let layout = lay_out(&window, usual(), 1.0, rtl, &MEASURE);
        let key = rects_of(&layout, |part| matches!(part, Part::Key { .. }))[0];
        let word = |wanted: &str| {
            layout
                .items
                .iter()
                .find(|item| matches!(&item.part, Part::Text { text, .. } if text == wanted))
                .map(|item| item.rect)
                .unwrap_or_else(|| panic!("no «{wanted}»"))
        };
        let (typed, fixed) = (word("ghbdtn"), word("привет"));
        let arrow = word(if rtl { "←" } else { "→" });
        if rtl {
            assert!(typed.x > key.right() && key.x > arrow.right() && arrow.x > fixed.right());
        } else {
            assert!(typed.right() < key.x && key.right() < arrow.x && arrow.right() < fixed.x);
        }
    }
}

/// The card of a letter: its buttons at the end in the middle of its height, its text one line cut
/// with a meant ellipsis.
#[test]
fn the_card_of_a_letter_has_its_buttons_at_the_end() {
    let window = bare(Node::LetterCard {
        label: "Письмо".to_owned(),
        title: "Обновление 1.8.0".to_owned(),
        words: "Очень длинный текст письма, который не помещается в одну строку карточки письма \
               и обрезается многоточием"
            .to_owned(),
        buttons: vec![Node::Button {
            id: Id::Site,
            label: "Открыть".to_owned(),
            kind: ButtonKind::Normal,
            browser: false,
        }],
    });
    let layout = lay_out(&window, usual(), 1.0, false, &MEASURE);
    let card = rects_of(&layout, |part| matches!(part, Part::Surface(_)))[0];
    let button = rects_of(&layout, |part| matches!(part, Part::Button { .. }))[0];
    assert!(
        button.right() <= card.right() && button.x > card.x + card.w / 2.0,
        "the button is not at the end"
    );
    assert!(
        layout.items.iter().any(|item| matches!(
            &item.part,
            Part::Text {
                wrap: Wrap::Ellipsis,
                ..
            }
        )),
        "the text of the card is not one line cut with a meant ellipsis"
    );
}

/// Every picture a window may name is a drawing of `res\art`.
#[test]
fn the_pictures_of_the_windows_are_drawings_of_res_art() {
    let names: Vec<&str> = lang_switcher::ui::art::names().collect();
    for art in [
        Art::About,
        Art::Avatar,
        Art::Hello,
        Art::Thanks,
        Art::News,
        Art::Update,
        Art::Wizard,
    ] {
        assert!(
            names.contains(&art.name()),
            "{art:?}: no «{}» among the drawings",
            art.name()
        );
    }
}

/// **The label «new» stands after the words of its switch** (passport §6, T-120-20): on their line,
/// a gap after them, while it fits there; under them when it does not — never over them.
#[test]
fn the_label_new_stands_after_the_words_of_its_switch() {
    let toggle = |label: &str| {
        bare(Node::Toggle {
            id: Id::Autostart,
            label: label.to_owned(),
            hint: None,
            on: false,
            badge: Some("new".to_owned()),
        })
    };
    let badge_and_words = |layout: &Layout<Id>| {
        let badge = rects_of(layout, |part| matches!(part, Part::Badge { .. }));
        let words = texts(layout);
        assert_eq!(badge.len(), 1, "one label «new»");
        (badge[0], words)
    };

    let layout = lay(&toggle("Flag in the tray"), usual());
    let (badge, words) = badge_and_words(&layout);
    assert_eq!(words.len(), 1);
    assert!(
        close(badge.x, words[0].right() + BADGE_GAP.at(layout.b)) && close(badge.y, words[0].y),
        "beside the words: {badge:?}, the words {:?}",
        words[0]
    );

    let long = "Show the flag of the layout in the notification area ".repeat(4);
    let layout = lay(&toggle(&long), usual());
    let (badge, words) = badge_and_words(&layout);
    let bottom = words.iter().map(|rect| rect.bottom()).fold(0.0, f32::max);
    assert!(
        close(badge.y, bottom),
        "under the words: {badge:?}, they end at {bottom}"
    );
}

/// **The sections of the settings stand in the panel, under its picture** (T-120-20, the mock-up's
/// `.panel.setp`): in a wide window the picture at the top of the panel, the column of sections
/// under it inside the panel, taking no room in the flow; in a narrow one — the row of tabs in the
/// flow, and no circle.
#[test]
fn the_sections_of_the_settings_stand_in_the_panel_under_the_picture() {
    let sections = Node::Sections {
        id: Id::Sections,
        items: ["General", "Hotkey", "Layouts", "Exceptions", "Diagnostics"]
            .map(str::to_owned)
            .to_vec(),
        current: 1,
    };
    let header = Node::Header {
        eyebrow: Some("Settings".to_owned()),
        title: "Hotkey".to_owned(),
        subtitle: None,
    };
    let settings = |with_sections: bool| {
        let mut column = vec![header.clone()];
        if with_sections {
            column.push(sections.clone());
        }
        column.push(text(Role::Body, "The key that switches the layout"));
        Window {
            kind: WindowKind::Settings,
            art: Some(Art::About),
            ..bare(Node::Column(column))
        }
    };
    let section_rects =
        |layout: &Layout<Id>| rects_of(layout, |p| matches!(p, Part::Section { .. }));

    let wide = lay(&settings(true), usual());
    let panel = rects_of(&wide, |p| matches!(p, Part::Panel))[0];
    let picture = rects_of(&wide, |p| matches!(p, Part::Art(_)))[0];
    let rows = section_rects(&wide);
    assert_eq!(rows.len(), 5);
    assert!(
        picture.y < panel.h / 2.0 - picture.h / 2.0,
        "the picture at the top: {picture:?}"
    );
    assert!(
        rows.iter().all(|row| row.x >= panel.x
            && row.right() <= panel.right()
            && row.y >= picture.bottom()),
        "the sections inside the panel under the picture: {rows:?}, the panel {panel:?}"
    );
    assert!(
        rows.windows(2)
            .all(|pair| pair[1].y > pair[0].y && close(pair[1].x, pair[0].x)),
        "a column: {rows:?}"
    );
    assert_eq!(
        texts(&wide),
        texts(&lay(&settings(false), usual())),
        "the sections took room in the flow"
    );

    let narrow = WindowSize {
        w: 640.0,
        h: 500.0,
        class: Class::Narrow,
        scrolls: false,
    };
    let layout = lay(&settings(true), narrow);
    let tabs = section_rects(&layout);
    assert!(rects_of(&layout, |p| matches!(p, Part::Panel)).is_empty());
    assert!(
        rects_of(&layout, |p| matches!(p, Part::Art(_))).is_empty(),
        "no circle"
    );
    assert!(close(tabs[1].y, tabs[0].y), "a row of tabs: {tabs:?}");
    let title = texts(&layout)[1];
    assert!(
        tabs[0].y >= title.bottom(),
        "the tabs in the flow under the header"
    );
}

/// **A line of status** (passport §6, T-120-17): its dot first, in the middle of the first line, the
/// words after it and wrapping under themselves — never under the dot; the dot by the state.
#[test]
fn a_line_of_status_is_its_dot_and_its_words() {
    use lang_switcher::ui::layout::Status;
    use lang_switcher::ui::scene::MenuState;

    let line = |state, words: &str| {
        bare(Node::Status {
            state,
            words: words.to_owned(),
        })
    };
    let layout = lay(&line(MenuState::Working, "Updates are told"), usual());
    let dot = rects_of(&layout, |part| matches!(part, Part::Dot(Status::Working)))[0];
    let words = texts(&layout)[0];
    assert!(
        words.x > dot.right(),
        "the words after the dot: {dot:?} {words:?}"
    );
    assert!(close(dot.y, words.y), "the dot on the first line");

    let long = lay(
        &line(MenuState::Restart, &"wrapping words ".repeat(20)),
        usual(),
    );
    let dot = rects_of(&long, |part| matches!(part, Part::Dot(Status::Restart)))[0];
    let words = texts(&long)[0];
    assert!(words.h > dot.h * 1.5, "the words wrap: {words:?}");
    assert!(words.x > dot.right(), "and stay after the dot: {words:?}");
    assert_eq!(
        rects_of(
            &lay(&line(MenuState::Paused, "Off"), usual()),
            |part| matches!(part, Part::Dot(Status::Idle))
        )
        .len(),
        1,
        "paused — the dot of ink3"
    );
}

/// **What Rows hand on, and a field in a row** (T-120-20, the mock-up's `.grow` and `.inrow`): a fill
/// inside Rows takes the room left, as in a Column — a list there shows more rows than its least; a
/// field in a row takes what its button leaves, the button beside it at the bottom of the row; in a
/// width too small for the field the button goes under it.
#[test]
fn rows_hand_on_their_room_and_a_field_shares_its_row_with_its_button() {
    use lang_switcher::ui::scene::{FieldKind, ListItem};

    let rows: Vec<ListItem> = (0..30)
        .map(|row| ListItem {
            label: format!("program{row}.exe"),
            check: None,
            mark: false,
        })
        .collect();
    let window = bare(Node::Column(vec![
        text(Role::Title, "Exceptions"),
        Node::Fill(Box::new(Node::Rows(vec![
            text(Role::Body, "The programs Lang Switcher leaves alone"),
            Node::Fill(Box::new(Node::List {
                id: Id::Language,
                rows,
                selected: 0,
                first: 0,
            })),
        ]))),
    ]));
    let layout = lay(&window, usual());
    let shown = rects_of(&layout, |part| matches!(part, Part::ListRow { .. })).len();
    assert!(shown > 6, "the list took the room left: {shown} rows");

    let row = bare(Node::Row(vec![
        Node::Field {
            id: Id::Sound,
            label: String::new(),
            value: "Pause".to_owned(),
            hint: String::new(),
            kind: FieldKind::Key { capturing: false },
        },
        button(Id::Copy, "Set", ButtonKind::Normal),
    ]));
    let layout = lay(&row, usual());
    let field = rects_of(&layout, |part| matches!(part, Part::Input { .. }))[0];
    let set = rects_of(&layout, |part| matches!(part, Part::Button { .. }))[0];
    assert!(
        close(set.x, field.right() + BUTTON_GAP.at(layout.b)),
        "beside: {field:?} {set:?}"
    );
    assert!(
        close(set.bottom(), field.bottom()),
        "at the bottom of the row"
    );
    let content = usual().w - 2.0 * CONTENT_PAD.h.at(layout.b);
    assert!(
        field.w > content / 2.0,
        "the field takes what the button leaves: {field:?}"
    );

    let (parts, _) =
        lang_switcher::ui::layout::place_node(&row.body, 120.0, 14.0, Class::Ordinary, &MEASURE);
    let narrow = |wanted: fn(&Part) -> bool| {
        parts
            .iter()
            .find(|item| wanted(&item.part))
            .expect("a part")
            .rect
    };
    let (field, set) = (
        narrow(|part| matches!(part, Part::Input { .. })),
        narrow(|part| matches!(part, Part::Button { .. })),
    );
    assert!(
        set.y >= field.bottom(),
        "too narrow for the field: the button under it, {field:?} {set:?}"
    );
}

/// **A list that takes the room left is no shortfall of its own** (T-120-20, stand of `e120-win-b`:
/// «Исключения», 620×500, ml — four rows shown, everything above the rule, yet a shortfall of 28,9):
/// what does not fit is measured with the list at its least, one row; the room left it shows.
#[test]
fn a_list_that_takes_the_room_left_is_no_shortfall_of_its_own() {
    use lang_switcher::ui::scene::ListItem;

    let rows: Vec<ListItem> = (0..30)
        .map(|row| ListItem {
            label: format!("program{row}.exe"),
            check: None,
            mark: false,
        })
        .collect();
    let window = |lines: usize| {
        let mut column: Vec<Node<Id>> = (0..lines).map(|_| text(Role::Body, "a line")).collect();
        column.push(Node::Fill(Box::new(Node::List {
            id: Id::Language,
            rows: rows.clone(),
            selected: 0,
            first: 0,
        })));
        bare(Node::Column(column))
    };
    // Enough lines that the list's natural six rows would not fit, and its least one row would.
    let layout = lay(&window(11), usual());
    let shown = rects_of(&layout, |part| matches!(part, Part::ListRow { .. })).len();
    assert!(
        (1..6).contains(&shown),
        "the list shows what the room holds: {shown} rows"
    );
    assert!(
        layout.shortfall.fits(),
        "no shortfall for the rows it does not need: {:?}",
        layout.shortfall
    );
    // And a column that does not fit even with one row of the list says so.
    let tall = lay(&window(40), usual());
    assert!(!tall.shortfall.fits(), "{:?}", tall.shortfall);
}

/// **A list given room fills it with its frame** (T-120-20, the mock-up's `.grow`): three layouts of
/// the cycle in a fill draw a frame down to the end of the room, the rows from its top — not a frame
/// of three rows with emptiness under it.
#[test]
fn a_list_given_room_fills_it_with_its_frame() {
    use lang_switcher::ui::scene::ListItem;

    let rows: Vec<ListItem> = ["English", "Русский", "Deutsch"]
        .iter()
        .map(|label| ListItem {
            label: (*label).to_owned(),
            check: Some(true),
            mark: false,
        })
        .collect();
    let window = bare(Node::Column(vec![
        text(Role::Title, "Layouts"),
        Node::Fill(Box::new(Node::List {
            id: Id::Language,
            rows,
            selected: 0,
            first: 0,
        })),
    ]));
    let layout = lay(&window, usual());
    let frame = rects_of(&layout, |part| matches!(part, Part::List(_)))[0];
    let rows = rects_of(&layout, |part| matches!(part, Part::ListRow { .. }));
    let last = rows.last().expect("rows");
    assert_eq!(rows.len(), 3);
    assert!(
        frame.bottom() > last.bottom() + last.h,
        "the frame reaches down the room, not three rows: {frame:?}, the last row {last:?}"
    );
    assert!(
        rows[0].y < frame.y + last.h,
        "the rows from the top of the frame: {rows:?}"
    );
}

/// **A small window opens at the least size of the passport** (T-120-21 Б: the details for the
/// author — five lines and two buttons): 620 × 500 while the work area holds it; a work area smaller
/// still gives it all of itself, as to any window.
#[test]
fn a_small_window_opens_at_the_least_size() {
    use lang_switcher::ui::layout::least_window_size;

    let small = least_window_size(1920.0, 1040.0);
    assert_eq!(
        (small.w, small.h, small.class, small.scrolls),
        (620.0, 500.0, Class::Narrow, false)
    );
    assert_eq!(
        least_window_size(600.0, 400.0),
        window_size(600.0, 400.0),
        "a tiny work area"
    );
}

/// **The wizard** (T-120-19, passport §6): its progress — bars of 0,3 B with 0,35 B between them
/// across the width, the passed ones and the current `accent`; its row of buttons — «Отмена» at the
/// left edge, «Назад» and the main one at the right; a check box with a line under its label.
#[test]
fn the_wizard_has_its_progress_its_row_and_its_check_boxes_with_a_line() {
    use lang_switcher::ui::tokens::{WIZARD_STEP, WIZARD_STEP_GAP};

    let window = Window {
        kind: WindowKind::Wizard,
        buttons: vec![
            button(Id::Cancel, "Cancel", ButtonKind::Quiet),
            button(Id::Back, "Back", ButtonKind::Normal),
            button(Id::Next, "Next", ButtonKind::Primary),
        ],
        ..bare(Node::Column(vec![
            Node::Progress {
                total: 4,
                current: 1,
            },
            Node::Check {
                id: Id::Sound,
                label: "The journal".to_owned(),
                on: true,
                hint: Some("The last 500 lines".to_owned()),
            },
        ]))
    };
    let layout = lay(&window, usual());
    let b = layout.b;

    let bars: Vec<(bool, Rect)> = layout
        .items
        .iter()
        .filter_map(|item| match item.part {
            Part::Progress { done } => Some((done, item.rect)),
            _ => None,
        })
        .collect();
    assert_eq!(
        bars.iter().map(|(done, _)| *done).collect::<Vec<_>>(),
        vec![true, true, false, false],
        "passed and current — accent"
    );
    assert!(
        bars.iter()
            .all(|(_, rect)| close(rect.h, WIZARD_STEP.at(b)))
    );
    assert!(close(
        bars[1].1.x - bars[0].1.right(),
        WIZARD_STEP_GAP.at(b)
    ));
    assert!(close(bars[0].1.w, bars[3].1.w), "equal bars");

    let at = |id: Id| {
        layout
            .items
            .iter()
            .find(|item| item.id == Some(id) && matches!(item.part, Part::Button { .. }))
            .expect("the button")
            .rect
    };
    let (cancel, back, next) = (at(Id::Cancel), at(Id::Back), at(Id::Next));
    let content_x = bars[0].1.x;
    assert!(
        close(cancel.x, content_x),
        "«Отмена» at the left edge: {cancel:?}"
    );
    assert!(
        next.x > back.x && back.x > cancel.right() + BUTTON_GAP.at(b),
        "the others at the right"
    );
    assert!(
        close(next.right(), bars[3].1.right()),
        "the main one the rightmost"
    );

    let texts = texts(&layout);
    assert_eq!(texts.len(), 2, "the label and its line");
    assert!(
        texts[1].y >= texts[0].bottom(),
        "the line under the label: {texts:?}"
    );

    // Mirrored with the window: «Отмена» at the right edge, the first bar the rightmost.
    let mirrored = lay_out(&window, usual(), 1.0, true, &MEASURE);
    let cancel_m = mirrored
        .items
        .iter()
        .find(|item| item.id == Some(Id::Cancel))
        .expect("the button")
        .rect;
    assert!(
        close(cancel_m.x, usual().w - cancel.right()),
        "mirrored: {cancel_m:?}"
    );

    // The line is a text of the window too: the instrument of fitting lengthens it.
    let mut window = window;
    let mut small = Vec::new();
    window.texts_mut(&mut |role, s| {
        if matches!(role, Role::Small) {
            small.push(s.clone());
        }
    });
    assert_eq!(small, vec!["The last 500 lines".to_owned()]);
}

/// **A field of code** (T-120-19): `FIELD_TEXT_LINES` lines of the role `code` — the monospace
/// face — in the paddings of a field of text; a field of text keeps the lines of `body`.
#[test]
fn a_field_of_code_is_lines_of_the_role_code() {
    use lang_switcher::ui::tokens::{FIELD_TEXT_LINES, MULTILINE_PAD};

    let field = |kind| Node::Field {
        id: Id::Site,
        label: String::new(),
        value: "a = 1".to_owned(),
        hint: String::new(),
        kind,
    };
    let window = bare(Node::Column(vec![
        field(FieldKind::Code),
        field(FieldKind::Text),
    ]));
    let layout = lay(&window, usual());
    let inputs: Vec<(bool, bool, Rect)> = layout
        .items
        .iter()
        .filter_map(|item| match &item.part {
            Part::Input {
                multiline, code, ..
            } => Some((*multiline, *code, item.rect)),
            _ => None,
        })
        .collect();
    assert_eq!(
        inputs
            .iter()
            .map(|(many, code, _)| (*many, *code))
            .collect::<Vec<_>>(),
        vec![(true, true), (true, false)],
        "both of many lines, the first of code"
    );
    let high = |role| {
        Style::of(role, layout.b, layout.size.h).line_height() * FIELD_TEXT_LINES.0
            + MULTILINE_PAD.v.at(layout.b) * 2.0
    };
    assert!(close(inputs[0].2.h, high(Role::Code)), "{:?}", inputs[0].2);
    assert!(close(inputs[1].2.h, high(Role::Body)), "{:?}", inputs[1].2);

    // Its value and its hint are texts of the role `code` for the instrument of fitting.
    let mut window = window;
    let mut roles = Vec::new();
    window.texts_mut(&mut |role, s| roles.push((role, s.clone())));
    assert!(
        roles.contains(&(Role::Code, "a = 1".to_owned())),
        "{roles:?}"
    );
    assert!(
        roles.contains(&(Role::Body, "a = 1".to_owned())),
        "{roles:?}"
    );
}
