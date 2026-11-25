//! The picture of an author's card on the canvas — stage Э121, task T-121-4 (решение 208.244 п. 7):
//! what part of it a box shows (cover, the middle of the whole window kept), and the decoding by
//! WIC — the decoder of the named format, the size of the frame held to the header, brought down
//! to the pixels of the frame. Drawn on canvases in memory and read back pixel by pixel.

#[path = "support/pictures.rs"]
mod pictures;

use lang_switcher::cards::{Format, sniff};
use lang_switcher::ui::canvas::Rect;
use lang_switcher::ui::canvas::{Canvas, Mode};
use lang_switcher::ui::photo::{self, Fit, Held};
use lang_switcher::ui::tokens::Rgb;
use std::rc::Rc;

const RED: [u8; 3] = [220, 30, 30];
const BLUE: [u8; 3] = [30, 30, 220];
const GROUND: Rgb = Rgb {
    r: 250,
    g: 250,
    b: 250,
};

/// Left half red, right half blue.
fn halves(w: u32) -> impl Fn(u32, u32) -> [u8; 3] {
    move |x, _| if x < w / 2 { RED } else { BLUE }
}

fn held(bytes: Vec<u8>) -> Held {
    let (format, w, h) = sniff(&bytes).expect("a picture of the cards");
    Held {
        bytes: Rc::from(bytes),
        format,
        w,
        h,
    }
}

fn near(pixel: Rgb, colour: [u8; 3]) -> bool {
    let close = |a: u8, b: u8| a.abs_diff(b) <= 40;
    close(pixel.r, colour[0]) && close(pixel.g, colour[1]) && close(pixel.b, colour[2])
}

/// Draws the picture held under `key` over `rect` on a canvas of `w` × `h` points at `scale`.
fn draw(
    key: u64,
    w: f32,
    h: f32,
    scale: f32,
    rect: Rect,
    keep: f32,
) -> lang_switcher::ui::canvas::Image {
    let mut canvas = Canvas::in_memory(w, h, scale, Mode::Direct2D).expect("a canvas");
    canvas.frame(|painter| {
        painter.clear(GROUND);
        painter.photo(rect, key, keep);
    });
    canvas.image().expect("pixels")
}

// =========================================================================================
// What a box shows
// =========================================================================================

#[test]
fn a_box_is_covered_and_the_rest_cut_evenly() {
    // A box wider than the picture: the whole width, the middle band of the height.
    assert_eq!(
        photo::fit(1000, 1440, 500.0, 500.0, 0.0),
        Some(Fit {
            source: (0, 220, 1000, 1000),
            place: (0.0, 0.0, 500.0, 500.0)
        })
    );
    // A box narrower than the picture: the whole height, the middle band of the width.
    assert_eq!(
        photo::fit(2560, 1280, 900.0, 600.0, 0.0),
        Some(Fit {
            source: (320, 0, 1920, 1280),
            place: (0.0, 0.0, 900.0, 600.0)
        })
    );
    // The panel of the eye of 208.197, 342 × 540 points of 1000 × 1440: the sides cut a little.
    let panel = photo::fit(1000, 1440, 342.0, 540.0, 0.0).expect("a fit");
    assert_eq!(panel.source.1, 0);
    assert_eq!(panel.source.3, 1440);
    assert!(panel.source.2 < 1000 && panel.source.0 > 0);
    assert_eq!(photo::fit(0, 10, 10.0, 10.0, 0.0), None);
    assert_eq!(photo::fit(10, 10, 0.0, 10.0, 0.0), None);
}

/// **The middle of the whole window is never cut** — 1780 of 2560: a box wider than the middle's
/// proportions cuts the sides only; a narrower one shows the whole middle, fitted to its width.
#[test]
fn the_middle_of_the_whole_window_is_never_cut() {
    let keep = 1780.0 / 2560.0;

    // 620 × 420: wider than 1780 : 1280 — the sides go, the middle stays whole.
    let small = photo::fit(2560, 1280, 620.0, 420.0, keep).expect("a fit");
    assert!(small.source.2 >= 1780, "{small:?}");
    assert_eq!(small.place, (0.0, 0.0, 620.0, 420.0));

    // 400 × 400: narrower than the middle — the middle whole, the colour of the window over and under.
    let narrow = photo::fit(2560, 1280, 400.0, 400.0, keep).expect("a fit");
    assert_eq!(narrow.source, (390, 0, 1780, 1280));
    let (_, y, w, h) = narrow.place;
    assert_eq!(w, 400.0);
    assert!((h - 400.0 * 1280.0 / 1780.0).abs() < 0.01 && (y - (400.0 - h) / 2.0).abs() < 0.01);

    // The panel keeps nothing: it is always covered.
    assert_eq!(
        photo::fit(1000, 1440, 400.0, 10.0, 0.0)
            .expect("a fit")
            .place,
        (0.0, 0.0, 400.0, 10.0)
    );
}

// =========================================================================================
// Decoded by WIC
// =========================================================================================

/// **A PNG and a JPEG are decoded, cut and drawn** — a picture red on the left and blue on the
/// right, 40 × 20, in a box of 20 × 20: the middle 20 × 20 shows, red then blue, at 100 % and 200 %.
#[test]
fn a_png_and_a_jpeg_are_cut_to_the_box_and_drawn() {
    for bytes in [pictures::png(40, 20, halves(40)), pictures::jpeg_halves()] {
        let picture = held(bytes);
        let key = photo::hold(photo::key_of(&[picture.w as u8; 32]), picture.clone());

        for scale in [1.0_f32, 2.0] {
            let image = draw(
                key,
                40.0,
                40.0,
                scale,
                Rect::new(10.0, 10.0, 20.0, 20.0),
                0.0,
            );
            let px = |points: f32| (points * scale) as u32;
            assert!(
                near(image.at(px(13.0), px(20.0)), RED),
                "{:?} red at {scale}: {:?}",
                picture.format,
                image.at(px(13.0), px(20.0))
            );
            assert!(
                near(image.at(px(27.0), px(20.0)), BLUE),
                "{:?} blue at {scale}",
                picture.format
            );
            assert_eq!(
                image.at(px(5.0), px(20.0)),
                GROUND,
                "nothing outside the box"
            );
        }
    }
}

/// **The middle kept** — the box narrower than the middle: the picture at the width of the box,
/// centred, the ground above and below.
#[test]
fn a_narrow_box_shows_the_middle_with_the_ground_above_and_below() {
    let picture = held(pictures::png(40, 20, halves(40)));
    let key = photo::hold(1, picture);
    // keep = 0.9: the middle is 36 × 20; a box of 20 × 20 shows it 20 × 11.1, from y ≈ 4.4.
    let image = draw(key, 40.0, 40.0, 1.0, Rect::new(10.0, 10.0, 20.0, 20.0), 0.9);

    assert_eq!(image.at(20, 11), GROUND, "above the picture");
    assert!(near(image.at(12, 20), RED) && near(image.at(28, 20), BLUE));
    assert_eq!(image.at(20, 28), GROUND, "below the picture");
}

/// **The decoder is the one of the format the header named, never guessed** — the bytes of a JPEG
/// held as a PNG are refused by the decoder of PNG, and nothing is drawn.
#[test]
fn the_decoder_is_the_one_the_header_named() {
    let mut picture = held(pictures::jpeg_halves());
    picture.format = Format::Png;
    let key = photo::hold(2, picture);

    let image = draw(key, 40.0, 40.0, 1.0, Rect::new(0.0, 0.0, 40.0, 40.0), 0.0);
    assert!(
        image.pixels().iter().all(|pixel| *pixel == GROUND),
        "nothing drawn"
    );
}

/// **The frame is the size the header gave** — a picture whose decoded frame is another size than
/// the one held for it is not drawn: the size in pixels checked by the header is the size decoded.
#[test]
fn a_frame_of_another_size_than_the_header_is_not_drawn() {
    let mut picture = held(pictures::png(40, 20, halves(40)));
    picture.w = 20;
    let key = photo::hold(3, picture);

    let image = draw(key, 40.0, 40.0, 1.0, Rect::new(0.0, 0.0, 40.0, 40.0), 0.0);
    assert!(
        image.pixels().iter().all(|pixel| *pixel == GROUND),
        "nothing drawn"
    );
}

/// A picture not held is nothing drawn, and no failure.
#[test]
fn a_picture_not_held_draws_nothing() {
    let image = draw(
        0xDEAD_BEEF,
        20.0,
        20.0,
        1.0,
        Rect::new(0.0, 0.0, 20.0, 20.0),
        0.0,
    );
    assert!(image.pixels().iter().all(|pixel| *pixel == GROUND));
}

/// The newest two pictures are held, an older one is let go.
#[test]
fn a_thread_holds_the_two_newest_pictures() {
    let picture = held(pictures::png(4, 4, |_, _| RED));
    for key in [10, 11, 12] {
        photo::hold(key, picture.clone());
    }
    assert!(photo::held(10).is_none());
    assert!(photo::held(11).is_some() && photo::held(12).is_some());
}

/// **A card shows its picture only when it is on the disk and whole** — `of_card` places it by the
/// view of the card (Б at the start or the end, В1 the whole window) and holds the bytes read back;
/// a card of the plain template, a picture not there or changed on the disk — none (А).
#[test]
fn a_card_shows_its_picture_only_when_it_is_whole_on_the_disk() {
    use lang_switcher::cards::{self, CardImage, CardSide, CardView, FeedCard};
    use lang_switcher::letters::Date;
    use lang_switcher::ui::scene::PhotoPlace;

    let place = std::env::temp_dir()
        .join("langsw-e121-photo")
        .join(std::process::id().to_string());
    let _ = std::fs::remove_dir_all(&place);
    let bytes = pictures::png(40, 20, halves(40));
    let mut hasher = lang_switcher::letters::feed::Sha256::new().expect("CNG");
    assert!(hasher.update(&bytes));
    let sha256 = hasher.finish().expect("a sum");
    let day = |d| Date::from_ymd(2027, 1, d).expect("a day");
    let card = |view| FeedCard {
        id: 1,
        from: day(1),
        to: day(10),
        view,
        image: Some(CardImage {
            url: format!("{}1.png", lang_switcher::links::CARDS_URL),
            sha256,
            size: bytes.len() as u64,
        }),
        title: "t".to_owned(),
        text: "x".to_owned(),
        link: String::new(),
    };

    let full = card(CardView::Full);
    assert_eq!(photo::of_card(&full, &place), None, "not downloaded yet");
    cards::fetch_pictures(
        std::slice::from_ref(&full),
        day(1),
        &place,
        lang_switcher::links::CARDS_URL,
        &mut |_, _| Some(bytes.clone()),
    );

    for (view, expected) in [
        (CardView::Panel(CardSide::Start), PhotoPlace::Start),
        (CardView::Panel(CardSide::End), PhotoPlace::End),
        (CardView::Full, PhotoPlace::Whole),
    ] {
        let shown = photo::of_card(&card(view), &place).expect("the picture is whole");
        assert_eq!(shown.place, expected);
        assert_eq!(shown.key, photo::key_of(&sha256));
        let held = photo::held(shown.key).expect("held");
        assert_eq!((held.format, held.w, held.h), (Format::Png, 40, 20));
    }
    assert_eq!(
        photo::of_card(&card(CardView::Text), &place),
        None,
        "А has no picture"
    );

    let file = cards::path_in(&place, full.image.as_ref().expect("a picture"));
    let mut changed = bytes.clone();
    changed[50] ^= 0xFF;
    std::fs::write(&file, &changed).expect("the test's own file");
    assert_eq!(
        photo::of_card(&full, &place),
        None,
        "changed on the disk — А"
    );

    let _ = std::fs::remove_dir_all(&place);
}
