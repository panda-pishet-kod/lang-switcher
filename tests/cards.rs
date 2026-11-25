//! The picture of an author's card — stage Э121, task T-121-3 (решение 208.244 п. 8): where it
//! may come from, how much of it, in what order it is checked, where it lies and when it goes.
//!
//! Not a byte goes to the network here: the request is handed to [`cards::fetch_pictures`] as a
//! function, and the tests hand it their own, which counts the calls and answers what the test
//! wants. The folder is a folder of the test's own under `%TEMP%`.

use lang_switcher::cards::{
    self, CardImage, CardSide, CardView, FeedCard, Format, IMAGE_CAP, Refused,
};
use lang_switcher::letters::{Date, feed};
use std::path::PathBuf;

fn day(year: i32, month: u32, day: u32) -> Date {
    Date::from_ymd(year, month, day).expect("a real day")
}

/// The SHA-256 of `bytes`, by the program's own body of CNG.
fn sum(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = feed::Sha256::new().expect("CNG");
    assert!(hasher.update(bytes));
    hasher.finish().expect("a sum")
}

/// The head of a PNG of `w` × `h`: the signature, `IHDR`, and some bytes after — enough for the
/// header and nothing a decoder could make a picture of. The checks of this task never decode.
fn png(w: u32, h: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&w.to_be_bytes());
    bytes.extend_from_slice(&h.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0xDE, 0xAD, 0xBE, 0xEF]);
    bytes.extend(std::iter::repeat_n(0x5A, 64));
    bytes
}

/// The head of a JPEG of `w` × `h`: SOI, an APP0 segment, a DQT segment, then the frame `marker`.
fn jpeg(marker: u8, w: u16, h: u16) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8];
    bytes.extend_from_slice(&[
        0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0,
    ]);
    bytes.extend_from_slice(&[0xFF, 0xDB, 0, 4, 0, 0]);
    bytes.extend_from_slice(&[0xFF, marker, 0, 17, 8]);
    bytes.extend_from_slice(&h.to_be_bytes());
    bytes.extend_from_slice(&w.to_be_bytes());
    bytes.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    bytes.extend_from_slice(&[0xFF, 0xD9]);
    bytes
}

/// A card of the whole window in its term, its picture these bytes at the author's folder.
fn card_of(id: u64, bytes: &[u8], view: CardView) -> FeedCard {
    FeedCard {
        id,
        from: day(2026, 12, 31),
        to: day(2027, 1, 10),
        view,
        image: Some(CardImage {
            url: format!("{}{id}.png", lang_switcher::links::CARDS_URL),
            sha256: sum(bytes),
            size: bytes.len() as u64,
        }),
        title: "t".to_owned(),
        text: "x".to_owned(),
        link: String::new(),
    }
}

/// A folder of this test's own, empty and not yet made.
fn folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir()
        .join("langsw-e121-cards")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

// =========================================================================================
// The header — the format and the size in pixels, before the picture is decoded
// =========================================================================================

#[test]
fn the_header_names_the_format_and_the_size_and_nothing_else_passes() {
    assert_eq!(
        cards::sniff(&png(1000, 1440)),
        Some((Format::Png, 1000, 1440))
    );
    assert_eq!(
        cards::sniff(&jpeg(0xC0, 2560, 1280)),
        Some((Format::Jpeg, 2560, 1280))
    );
    assert_eq!(
        cards::sniff(&jpeg(0xC2, 640, 320)),
        Some((Format::Jpeg, 640, 320)),
        "progressive"
    );

    assert_eq!(cards::sniff(&png(0, 10)), None, "no width");
    assert_eq!(cards::sniff(&jpeg(0xC0, 10, 0)), None, "no height");
    assert_eq!(
        cards::sniff(&png(10, 10)[..20]),
        None,
        "the header cut short"
    );
    assert_eq!(
        cards::sniff(&jpeg(0xC0, 10, 10)[..24]),
        None,
        "the frame cut short"
    );
    assert_eq!(
        cards::sniff(b"GIF89a\x10\x00\x10\x00"),
        None,
        "not a format of the cards"
    );
    assert_eq!(
        cards::sniff(&[0xFF, 0xD8, 0xFF, 0xD9]),
        None,
        "a JPEG with no frame"
    );
    assert_eq!(
        cards::sniff(&[0xFF, 0xD8, 0xFF, 0xC4, 0, 1]),
        None,
        "a length under two"
    );
    assert_eq!(cards::sniff(&[]), None);
}

#[test]
fn a_picture_is_no_larger_than_its_view_is_made_at() {
    assert!(cards::size_fits(
        CardView::Panel(CardSide::Start),
        1000,
        1440
    ));
    assert!(
        cards::size_fits(CardView::Panel(CardSide::End), 500, 720),
        "smaller is stretched"
    );
    assert!(!cards::size_fits(
        CardView::Panel(CardSide::Start),
        1001,
        1440
    ));
    assert!(!cards::size_fits(
        CardView::Panel(CardSide::Start),
        1000,
        1441
    ));
    assert!(cards::size_fits(CardView::Full, 2560, 1280));
    assert!(!cards::size_fits(CardView::Full, 2561, 1280));
    assert!(
        !cards::size_fits(CardView::Text, 10, 10),
        "the plain template has no picture"
    );
}

// =========================================================================================
// The checks, in their order
// =========================================================================================

/// **The size, then the sum, then the header** — a picture of the wrong size is refused before
/// its sum is taken, and bytes that are not the author's are refused before a byte of them is
/// parsed: no header of a file the signature does not stand behind is read.
#[test]
fn the_sum_is_checked_before_the_header_is_read() {
    let good = png(1000, 1440);
    let card = card_of(1, &good, CardView::Panel(CardSide::Start));
    let image = card.image.as_ref().expect("a picture");

    assert_eq!(
        cards::check(&good, image, card.view),
        Ok((Format::Png, 1000, 1440))
    );

    let mut longer = good.clone();
    longer.push(0);
    assert_eq!(cards::check(&longer, image, card.view), Err(Refused::Size));

    // The same length, another byte: refused on the sum — and these bytes are no PNG at all, so a
    // header read first would have said «format».
    let mut foreign = good.clone();
    foreign[0] = b'G';
    assert_eq!(cards::check(&foreign, image, card.view), Err(Refused::Sum));

    // The author's bytes, a format the cards do not take.
    let gif = b"GIF89a-not-a-card".to_vec();
    let gif_image = CardImage {
        url: image.url.clone(),
        sha256: sum(&gif),
        size: gif.len() as u64,
    };
    assert_eq!(
        cards::check(&gif, &gif_image, card.view),
        Err(Refused::Format)
    );

    // The author's bytes, larger than the view.
    let wide = png(2560, 1280);
    let wide_image = CardImage {
        url: image.url.clone(),
        sha256: sum(&wide),
        size: wide.len() as u64,
    };
    assert_eq!(
        cards::check(&wide, &wide_image, card.view),
        Err(Refused::Pixels)
    );
    assert!(cards::check(&wide, &wide_image, CardView::Full).is_ok());
}

// =========================================================================================
// The download — the address, the cap, the term, once
// =========================================================================================

/// **The sentry of the stage: no card, no request and no file** — the read of a feed without
/// `[[card]]` asks the network nothing more and leaves the disk as it was.
#[test]
fn a_feed_without_cards_asks_for_nothing_and_writes_nothing() {
    let place = folder("none");
    let mut asked = 0;

    let outcomes = cards::fetch_pictures(
        &[],
        day(2027, 1, 1),
        &place,
        lang_switcher::links::CARDS_URL,
        &mut |_, _| {
            asked += 1;
            None
        },
    );

    assert!(outcomes.is_empty());
    assert_eq!(asked, 0, "not one request");
    assert!(!place.exists(), "not one folder");
}

/// **One request for a picture that is due, no more than its size, and the file after it** — then
/// none while the file is whole; the request asks for the address of the record and caps the
/// answer at its size.
#[test]
fn a_due_picture_is_asked_for_once_capped_at_its_size_and_kept() {
    let place = folder("once");
    let bytes = png(1000, 1440);
    let card = card_of(1, &bytes, CardView::Panel(CardSide::End));
    let asked = std::cell::RefCell::new(Vec::new());

    let mut get = |url: &str, cap: u64| {
        asked.borrow_mut().push((url.to_owned(), cap));
        Some(bytes.clone())
    };
    let outcomes = cards::fetch_pictures(
        std::slice::from_ref(&card),
        day(2026, 12, 24),
        &place,
        lang_switcher::links::CARDS_URL,
        &mut get,
    );

    assert_eq!(outcomes, [(1, Ok(()))]);
    assert_eq!(
        *asked.borrow(),
        [(card.image.as_ref().unwrap().url.clone(), bytes.len() as u64)]
    );
    assert_eq!(
        cards::kept(&card, &place),
        Some(bytes.clone()),
        "the file is the picture"
    );
    let file = cards::path_in(&place, card.image.as_ref().unwrap());
    assert!(
        file.file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".card")
    );

    // Whole on the disk: not asked again.
    let again = cards::fetch_pictures(
        std::slice::from_ref(&card),
        day(2026, 12, 31),
        &place,
        lang_switcher::links::CARDS_URL,
        &mut get,
    );
    assert!(again.is_empty());
    assert_eq!(asked.borrow().len(), 1, "one request in all");

    let _ = std::fs::remove_dir_all(&place);
}

/// **Not sooner than seven days before the term, not after it** — and the plain template, or a
/// card with no picture, asks for nothing.
#[test]
fn a_picture_is_asked_for_only_from_a_week_before_the_term_to_its_end() {
    let bytes = png(10, 10);
    let card = card_of(1, &bytes, CardView::Full);
    let mut asked = 0;

    for (today, due) in [
        (day(2026, 12, 23), false),
        (day(2026, 12, 24), true),
        (day(2027, 1, 10), true),
        (day(2027, 1, 11), false),
    ] {
        assert_eq!(card.picture_is_due(today), due, "{today}");
        let place = folder("term");
        cards::fetch_pictures(
            std::slice::from_ref(&card),
            today,
            &place,
            lang_switcher::links::CARDS_URL,
            &mut |_, _| {
                asked += 1;
                None
            },
        );
        let _ = std::fs::remove_dir_all(&place);
    }
    assert_eq!(asked, 2);

    let mut text = card.clone();
    text.view = CardView::Text;
    assert!(!text.picture_is_due(day(2027, 1, 1)));
    let mut bare = card.clone();
    bare.image = None;
    assert!(!bare.picture_is_due(day(2027, 1, 1)));
}

/// **Only the author's folder** — an address elsewhere is refused before any request: another
/// host, another folder, a path that climbs out, a query.
#[test]
fn a_picture_comes_only_from_the_folder_of_the_cards() {
    let bytes = png(10, 10);
    let base = card_of(1, &bytes, CardView::Full);

    for url in [
        "https://example.com/langswitcher/cards/1.png",
        "http://panda-pishet-kod.dev/langswitcher/cards/1.png",
        "https://panda-pishet-kod.dev/langswitcher/files/1.png",
        "https://panda-pishet-kod.dev/langswitcher/cards/../files/1.png",
        "https://panda-pishet-kod.dev/langswitcher/cards/1.png?x=1",
        "https://panda-pishet-kod.dev/langswitcher/cards/",
    ] {
        let mut card = base.clone();
        card.image.as_mut().unwrap().url = url.to_owned();
        let place = folder("foreign");
        let mut asked = 0;
        let outcomes = cards::fetch_pictures(
            &[card],
            day(2027, 1, 1),
            &place,
            lang_switcher::links::CARDS_URL,
            &mut |_, _| {
                asked += 1;
                Some(bytes.clone())
            },
        );
        assert_eq!(outcomes, [(1, Err(Refused::Address))], "{url}");
        assert_eq!(asked, 0, "no request for {url}");
        assert!(!place.exists(), "no file for {url}");
    }
    assert_eq!(
        lang_switcher::links::CARDS_URL,
        "https://panda-pishet-kod.dev/langswitcher/cards/"
    );
}

/// **What did not come, or is not the author's, leaves no file** — the card shows the plain
/// template, and the picture is asked for again at the next read.
#[test]
fn a_picture_that_fails_leaves_no_file() {
    let bytes = png(10, 10);
    let card = card_of(1, &bytes, CardView::Full);

    let cases: [(Option<Vec<u8>>, Refused); 3] = [
        (None, Refused::Network),
        (Some(bytes[..bytes.len() - 1].to_vec()), Refused::Size),
        (
            Some({
                let mut other = bytes.clone();
                other[30] ^= 1;
                other
            }),
            Refused::Sum,
        ),
    ];
    for (answer, refused) in cases {
        let place = folder("fails");
        let outcomes = cards::fetch_pictures(
            std::slice::from_ref(&card),
            day(2027, 1, 1),
            &place,
            lang_switcher::links::CARDS_URL,
            &mut |_, _| answer.clone(),
        );
        assert_eq!(outcomes, [(1, Err(refused))]);
        assert_eq!(cards::kept(&card, &place), None, "{refused:?}");
        assert!(!place.exists(), "no folder left for {refused:?}");
    }
}

/// **A file changed on the disk is no picture** — read back and checked again every time.
#[test]
fn a_picture_changed_on_the_disk_is_no_picture() {
    let place = folder("changed");
    let bytes = png(10, 10);
    let card = card_of(1, &bytes, CardView::Full);
    cards::fetch_pictures(
        std::slice::from_ref(&card),
        day(2027, 1, 1),
        &place,
        lang_switcher::links::CARDS_URL,
        &mut |_, _| Some(bytes.clone()),
    );
    assert!(cards::kept(&card, &place).is_some());

    let file = cards::path_in(&place, card.image.as_ref().unwrap());
    let mut changed = bytes.clone();
    changed[40] ^= 0xFF;
    std::fs::write(&file, &changed).expect("the test's own file");
    assert_eq!(cards::kept(&card, &place), None);

    let _ = std::fs::remove_dir_all(&place);
}

/// **After the term the picture is deleted** — and every file no card needs, and the folder when
/// it is empty; a picture still in its term stays.
#[test]
fn after_the_term_the_picture_is_deleted() {
    let place = folder("sweep");
    let one = png(10, 10);
    let two = png(20, 10);
    let first = card_of(1, &one, CardView::Full);
    let mut second = card_of(2, &two, CardView::Full);
    second.to = day(2027, 1, 20);
    let mut get = |url: &str, _| {
        Some(if url.contains("/1.") {
            one.clone()
        } else {
            two.clone()
        })
    };
    cards::fetch_pictures(
        &[first.clone(), second.clone()],
        day(2027, 1, 1),
        &place,
        lang_switcher::links::CARDS_URL,
        &mut get,
    );
    std::fs::write(place.join("stray.tmp"), b"x").expect("the test's own file");

    cards::sweep(&[first.clone(), second.clone()], day(2027, 1, 11), &place);
    assert_eq!(cards::kept(&first, &place), None, "past its term");
    assert!(cards::kept(&second, &place).is_some(), "still in its term");
    assert!(
        !place.join("stray.tmp").exists(),
        "nothing a card does not need"
    );

    cards::sweep(&[first, second], day(2027, 1, 21), &place);
    assert!(!place.exists(), "the folder goes when it is empty");
}

/// The cap is one megabyte, and a record over it has no picture (`tests\feed_cards.rs`).
#[test]
fn the_cap_of_a_picture_is_one_megabyte() {
    assert_eq!(IMAGE_CAP, 1024 * 1024);
    assert_eq!(cards::DOWNLOAD_AHEAD_DAYS, 7);
}
