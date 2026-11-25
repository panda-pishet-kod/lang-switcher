//! The views of an author's card — stage Э121, task T-121-5 (решение 208.244 пп. 1–2): Б, the
//! picture in place of the panel at the start or the end of the line; В1, the picture over the whole
//! window and the words and the buttons in a strip under it; the small screen; a language read right
//! to left. The window of a card as the letter describes it, laid out by the layout of the shell.

use lang_switcher::letters::{Action, Button, Letter, LetterPlan};
use lang_switcher::settings::Language;
use lang_switcher::theme::ThemeSetting;
use lang_switcher::ui::host::Content as _;
use lang_switcher::ui::layout::{
    Concession, Layout, Part, PhotoEdge, client_size, lay_out, lay_out_with,
};
use lang_switcher::ui::scene::{Art, Node, Photo, PhotoPlace, Window};
use lang_switcher::ui::text::{Direction, Text};
use lang_switcher::ui::tokens::{PANEL_SHARE, Role};
use lang_switcher::ui::windows::letter::{Id, LetterView};

const KEY: u64 = 0x2027;

/// The plan of a card as `letters::plan_in` makes it, with words of the test's own and `photo`.
fn plan(photo: Option<Photo>, caption: &str) -> LetterPlan {
    LetterPlan {
        caption: "Письмо от автора".to_owned(),
        eyebrow: "Письмо от автора".to_owned(),
        title: "Новость от автора".to_owned(),
        subtitle: "31 декабря".to_owned(),
        panel_title: "С Новым годом!".to_owned(),
        panel_text: caption.to_owned(),
        right: Some(Button {
            label: "Читать на сайте".to_owned(),
            action: Action::OpenLink,
            enabled: true,
        }),
        accent: Some(Button {
            label: "Закрыть".to_owned(),
            action: Action::Close,
            enabled: true,
        }),
        photo,
        ..LetterPlan::default()
    }
}

const CAPTION: &str = "Спасибо, что весь год были с Lang Switcher.";

fn window(photo: Option<Photo>, caption: &str) -> Window<Id> {
    LetterView {
        letter: Letter::Card(1),
        plan: plan(photo, caption),
        key: "Pause".to_owned(),
        tick: lang_switcher::letters::DEMO_STILL,
        language: Language::Ru,
        theme: ThemeSetting::Light,
    }
    .describe()
}

fn photo(place: PhotoPlace) -> Option<Photo> {
    Some(Photo { key: KEY, place })
}

fn laid(window: &Window<Id>, w: f32, h: f32, rtl: bool) -> Layout<Id> {
    let (tag, direction) = if rtl {
        ("he", Direction::RightToLeft)
    } else {
        ("ru", Direction::LeftToRight)
    };
    let text = Text::new(tag, direction).expect("the text of the language");
    lay_out(window, client_size(w, h), 1.0, rtl, &text)
}

/// The rectangles of the parts that are the picture of the card.
fn photos(layout: &Layout<Id>) -> Vec<(Part, (f32, f32, f32, f32))> {
    layout
        .items
        .iter()
        .filter(|item| matches!(item.part, Part::Photo { .. }))
        .map(|item| {
            (
                item.part.clone(),
                (item.rect.x, item.rect.y, item.rect.w, item.rect.h),
            )
        })
        .collect()
}

/// Whether anything of the panel with the panda is in the layout: the dotted panel, the circle,
/// the line of the brand.
fn has_the_panda(layout: &Layout<Id>) -> bool {
    layout.items.iter().any(|item| {
        matches!(
            item.part,
            Part::Panel
                | Part::Art(_)
                | Part::BrandName { .. }
                | Part::Link {
                    role: Role::Brand,
                    ..
                }
        )
    })
}

/// The leftmost x of the text of the layout.
fn text_left(layout: &Layout<Id>) -> f32 {
    layout
        .items
        .iter()
        .filter(|item| matches!(item.part, Part::Text { .. }))
        .map(|item| item.rect.x)
        .fold(f32::MAX, f32::min)
}

// =========================================================================================
// The description
// =========================================================================================

/// **The letter of a card describes its view** — Б: the picture in place of the art, no line of the
/// brand; В1: the heading and the words of the card alone in the strip; no picture: the panda of
/// the news and its brand (А).
#[test]
fn the_letter_of_a_card_describes_its_view() {
    let start = window(photo(PhotoPlace::Start), CAPTION);
    assert_eq!(
        start.art,
        Some(Art::Photo(Photo {
            key: KEY,
            place: PhotoPlace::Start
        }))
    );
    assert!(start.brand.is_none(), "no Panda Koder over the picture");
    assert!(
        matches!(start.body, Node::Column(_)),
        "the text of a news item"
    );

    let whole = window(photo(PhotoPlace::Whole), CAPTION);
    assert!(whole.brand.is_none());
    let Node::Rows(words) = &whole.body else {
        panic!("the strip of В1 holds rows: {:?}", whole.body);
    };
    assert_eq!(
        words,
        &vec![
            Node::Text(Role::Strong, "С Новым годом!".to_owned()),
            Node::Text(Role::Small, CAPTION.to_owned()),
        ]
    );
    assert_eq!(whole.buttons.len(), 2, "«Читать на сайте» and «Закрыть»");

    let plain = window(None, CAPTION);
    assert_eq!(plain.art, Some(Art::News), "А — the panda of the news");
    assert!(plain.brand.is_some());
}

// =========================================================================================
// Б — the picture in place of the panel
// =========================================================================================

/// **Б at the start and at the end of the line** — the panel's place and size, 38 % of the eye of
/// 900 × 540, the text on the other side; nothing of the panda.
#[test]
fn the_picture_of_b_stands_where_the_panel_stood_at_either_side() {
    let panel = 900.0 * PANEL_SHARE.0;

    let start = laid(
        &window(photo(PhotoPlace::Start), CAPTION),
        900.0,
        540.0,
        false,
    );
    assert_eq!(
        photos(&start),
        [(
            Part::Photo {
                key: KEY,
                whole: false,
                edge: PhotoEdge::End
            },
            (0.0, 0.0, panel, 540.0)
        )]
    );
    assert!(!has_the_panda(&start));
    assert!(text_left(&start) > panel, "the text beside the picture");
    assert!(start.shortfall.fits(), "{:?}", start.shortfall);

    let end = laid(
        &window(photo(PhotoPlace::End), CAPTION),
        900.0,
        540.0,
        false,
    );
    assert_eq!(
        photos(&end),
        [(
            Part::Photo {
                key: KEY,
                whole: false,
                edge: PhotoEdge::Start
            },
            (900.0 - panel, 0.0, panel, 540.0)
        )]
    );
    assert!(text_left(&end) < 900.0 - panel && text_left(&end) > 0.0);
}

/// **In a language read right to left the sides change places** — the picture «at the start» is at
/// the right; its rectangle moves, the picture itself is not turned (the canvas draws it as it is).
#[test]
fn right_to_left_the_sides_change_places() {
    let panel = 900.0 * PANEL_SHARE.0;
    let start = laid(
        &window(photo(PhotoPlace::Start), CAPTION),
        900.0,
        540.0,
        true,
    );
    let end = laid(&window(photo(PhotoPlace::End), CAPTION), 900.0, 540.0, true);

    assert_eq!(photos(&start)[0].1, (900.0 - panel, 0.0, panel, 540.0));
    assert_eq!(photos(&end)[0].1, (0.0, 0.0, panel, 540.0));
    assert!(start.mirrored && end.mirrored);
}

/// **The small screen** — no panel: the picture is a band over the text, a third of the height, and
/// the text under it.
#[test]
fn on_a_small_screen_the_picture_of_b_is_a_band_over_the_text() {
    let layout = laid(
        &window(photo(PhotoPlace::Start), CAPTION),
        620.0,
        500.0,
        false,
    );
    let band = (500.0_f32 * 0.34).round();

    assert_eq!(
        photos(&layout),
        [(
            Part::Photo {
                key: KEY,
                whole: false,
                edge: PhotoEdge::Bottom
            },
            (0.0, 0.0, 620.0, band)
        )]
    );
    assert!(
        !has_the_panda(&layout),
        "no circle beside the heading either"
    );
    let first_text = layout
        .items
        .iter()
        .filter(|item| matches!(item.part, Part::Text { .. }))
        .map(|item| item.rect.y)
        .fold(f32::MAX, f32::min);
    assert!(first_text > band, "the text under the band: {first_text}");
}

// =========================================================================================
// В1 — the whole window
// =========================================================================================

/// **В1: the picture over the window, the strip under it** — the line of the theme at the top of
/// the strip, the words at the start of the line, the buttons at its end, both inside the strip;
/// the picture down to the strip.
#[test]
fn the_picture_of_c1_takes_the_window_and_the_strip_its_foot() {
    let layout = laid(
        &window(photo(PhotoPlace::Whole), CAPTION),
        900.0,
        540.0,
        false,
    );
    let pictures = photos(&layout);
    let [(part, (x, y, w, top))] = pictures.as_slice() else {
        panic!("one picture: {pictures:?}");
    };
    assert_eq!(
        part,
        &Part::Photo {
            key: KEY,
            whole: true,
            edge: PhotoEdge::None
        }
    );
    assert_eq!((*x, *y, *w), (0.0, 0.0, 900.0));
    assert!(
        *top > 540.0 * 0.6,
        "the picture is most of the window: {top}"
    );

    let rule = layout
        .items
        .iter()
        .find(|item| item.part == Part::Divider)
        .expect("the line over the strip");
    assert_eq!((rule.rect.x, rule.rect.y, rule.rect.w), (0.0, *top, 900.0));

    let buttons: Vec<_> = layout
        .items
        .iter()
        .filter(|item| matches!(item.part, Part::Button { .. }))
        .collect();
    assert_eq!(buttons.len(), 2);
    for item in layout
        .items
        .iter()
        .filter(|item| matches!(item.part, Part::Button { .. } | Part::Text { .. }))
    {
        assert!(
            item.rect.y > *top && item.rect.bottom() < 540.0,
            "inside the strip: {item:?}"
        );
    }
    let words_right = layout
        .items
        .iter()
        .filter(|item| matches!(item.part, Part::Text { .. }))
        .map(|item| item.rect.right())
        .fold(0.0_f32, f32::max);
    let buttons_left = buttons
        .iter()
        .map(|item| item.rect.x)
        .fold(f32::MAX, f32::min);
    assert!(words_right < buttons_left, "the words, then the buttons");
    assert!(!has_the_panda(&layout));
    assert!(layout.shortfall.fits(), "{:?}", layout.shortfall);
}

/// **The caption of В1 is two lines at most in the eye** (208.244 п. 9) — a longer one is the
/// shortfall of the window, which the instrument of fitting counts.
#[test]
fn a_caption_of_c1_over_two_lines_does_not_fit() {
    let text = Text::new("ru", Direction::LeftToRight).expect("the text");
    let size = client_size(900.0, 540.0);
    let lay = |caption: &str| {
        lay_out_with(
            &window(photo(PhotoPlace::Whole), caption),
            size,
            1.0,
            false,
            &text,
            &[],
        )
    };

    assert!(lay(CAPTION).shortfall.fits());
    let long = CAPTION.repeat(6);
    let over = lay(&long);
    assert!(
        !over.shortfall.fits(),
        "six sentences are over two lines: {:?}",
        over.shortfall
    );
    let _ = Concession::ALL;
}

/// **On a small screen the strip of В1 grows** — the cap of two lines is the eye's (208.197): at
/// 620 × 500 a caption of three or four lines fits, and only a strip over half the window does not.
#[test]
fn on_a_small_screen_the_strip_of_c1_grows_to_half_the_window() {
    let text = Text::new("ru", Direction::LeftToRight).expect("the text");
    let size = client_size(620.0, 500.0);
    let lay = |caption: &str| {
        lay_out_with(
            &window(photo(PhotoPlace::Whole), caption),
            size,
            1.0,
            false,
            &text,
            &[],
        )
    };

    let three = lay(&CAPTION.repeat(3));
    assert!(
        three.shortfall.fits(),
        "three sentences on a small screen: {:?}",
        three.shortfall
    );
    let huge = lay(&CAPTION.repeat(30));
    assert!(
        !huge.shortfall.fits(),
        "a strip over half the window: {:?}",
        huge.shortfall
    );
}
