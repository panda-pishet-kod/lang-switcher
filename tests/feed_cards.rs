//! The author's cards in the feed — stage Э121, task T-121-1 (решение 208.244): the record
//! `[[card]]`, a top-level array of its own, and the guard of every released version.
//!
//! # The guard of the released versions
//!
//! Every version out there reads the feed with the parse of 1.8.0 — `RawFeed`, `RawItem`,
//! `RawText` of `src\feed.rs`, copied below **verbatim and frozen**. That parse refuses the WHOLE
//! feed over one unknown field of an `[[item]]` (the finding of the window of the cards, п. 1:
//! `#[serde(flatten)] languages` takes every key it does not know and requires a title and a text
//! of it), and a refused feed is no news and no update for anybody. A top-level `[[card]]` it skips:
//! `RawFeed` has no `deny_unknown_fields`. So the record of a card is written where the old parse
//! does not look, and this test reads the very file the product reads its cards from with the
//! frozen copy: a card put where the copy breaks — a field of an `[[item]]`, a table under it —
//! turns the test red together with the record.
//!
//! The copy is **not** to be brought up to date with `src\feed.rs`: it is the program people have,
//! and they have it whatever this build says.

use lang_switcher::cards::{CardSide, CardView};
use lang_switcher::letters::{Date, feed};

/// The feed with two cards the author would write: a panel at the end of the line and a whole
/// window, next to an update and a news item.
const WITH_CARDS: &str = include_str!("fixtures/feed-with-cards.toml");

/// ⛔ **Frozen: the parse of 1.8.0**, `src\feed.rs` at `v1.8.0`, the three structs and nothing of
/// what reads them past the shape — the choice of language does not decide whether the feed is
/// refused. `Date` is the program's own, as it was.
mod released_1_8_0 {
    use lang_switcher::letters::Date;

    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    pub struct RawFeed {
        pub schema: u32,
        #[serde(default)]
        pub item: Vec<RawItem>,
        #[serde(default)]
        pub download: Option<toml::Value>,
        #[serde(default)]
        pub stop: Option<toml::Value>,
    }

    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    pub struct RawItem {
        pub id: u64,
        #[serde(rename = "type")]
        pub kind: String,
        #[serde(default)]
        pub date: Option<Date>,
        #[serde(default)]
        pub version: Option<String>,
        #[serde(default)]
        pub link: String,
        #[serde(flatten)]
        pub languages: std::collections::BTreeMap<String, RawText>,
    }

    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    pub struct RawText {
        pub title: String,
        pub text: String,
    }
}

fn day(year: i32, month: u32, day: u32) -> Date {
    Date::from_ymd(year, month, day).expect("a real day")
}

/// **The released versions read the feed with cards, and see the same letters** — the guard of
/// T-121-1. The frozen parse of 1.8.0 takes the file without a refusal, its entries are the ones
/// the product reads, and the cards are not among them.
#[test]
fn the_released_parse_reads_a_feed_with_cards_and_sees_no_card() {
    let old: released_1_8_0::RawFeed = toml::from_str(WITH_CARDS)
        .expect("1.8.0 must read a feed with [[card]] — or nobody gets news");
    let new = feed::parse_body(WITH_CARDS, "ru").expect("the product reads it too");

    assert_eq!(old.schema, 1);
    let old_ids: Vec<u64> = old.item.iter().map(|item| item.id).collect();
    let mut new_ids: Vec<u64> = new.news.iter().map(|item| item.id).collect();
    new_ids.extend(new.update.iter().map(|item| item.id));
    new_ids.sort_unstable();
    assert_eq!(
        old_ids, new_ids,
        "the same entries, the cards are none of them"
    );
    assert!(
        old.item
            .iter()
            .all(|item| item.kind == "news" || item.kind == "update"),
        "no entry of 1.8.0 is a card"
    );
    assert_eq!(new.cards.len(), 2, "and the product sees both cards");
}

/// The cards as the product reads them: the term, the view, the side, the picture with its sum and
/// size, the words in the language of the interface — its own, else English, else Russian.
#[test]
fn a_card_is_read_with_its_term_view_side_picture_and_words() {
    let ru = feed::parse_body(WITH_CARDS, "ru").expect("it parses");
    let [panel, full] = ru.cards.as_slice() else {
        panic!("two cards, in the order of their numbers: {:?}", ru.cards);
    };

    assert_eq!(
        (panel.id, panel.from, panel.to),
        (1, day(2026, 12, 31), day(2027, 1, 10))
    );
    assert_eq!(panel.view, CardView::Panel(CardSide::End));
    assert_eq!(panel.title, "С Новым годом!");
    assert_eq!(
        panel.link,
        "https://panda-pishet-kod.dev/langswitcher/?lang=ru"
    );
    let image = panel.image.as_ref().expect("the picture of the panel");
    assert_eq!(
        image.url,
        "https://panda-pishet-kod.dev/langswitcher/cards/2027-panel.jpg"
    );
    assert_eq!(image.size, 524_288);
    assert_eq!(image.sha256[0], 0x00);
    assert_eq!(image.sha256[31], 0xFF, "the digits are read in either case");

    assert_eq!(full.view, CardView::Full);
    assert_eq!(full.link, "", "a card may have no link");
    assert_eq!(full.image.as_ref().map(|image| image.size), Some(1_048_576));

    // The language: its own (ar), else English (de), else Russian.
    let ar = feed::parse_body(WITH_CARDS, "ar").expect("it parses");
    assert_eq!(ar.cards[0].title, "سنة جديدة سعيدة!");
    assert_eq!(
        ar.cards[1].title, "Merry Christmas!",
        "no ar table: English"
    );
    let de = feed::parse_body(WITH_CARDS, "de").expect("it parses");
    assert_eq!(de.cards[0].title, "Happy New Year!");
}

/// **A card of the wrong shape costs itself, never the feed** — the lesson of `[[item]]`: one bad
/// record must not silence the others. A card with no term, a term backwards, no `ru` and no `en`
/// is skipped; the news, the update and the good card stay.
#[test]
fn a_bad_card_is_skipped_and_the_rest_of_the_feed_stays() {
    let bad = [
        // No term at all.
        "[[card]]\nid = 7\nview = \"full\"\n[card.ru]\ntitle = \"а\"\ntext = \"б\"\n",
        // The term backwards.
        "[[card]]\nid = 7\nfrom = 2027-01-10\nto = 2027-01-01\n[card.ru]\ntitle = \"а\"\ntext = \"б\"\n",
        // Neither ru nor en.
        "[[card]]\nid = 7\nfrom = 2027-01-01\nto = 2027-01-10\n[card.de]\ntitle = \"a\"\ntext = \"b\"\n",
        // A number that is not one.
        "[[card]]\nid = \"seven\"\nfrom = 2027-01-01\nto = 2027-01-10\n[card.ru]\ntitle = \"а\"\ntext = \"б\"\n",
    ];

    for card in bad {
        let body = format!("{WITH_CARDS}\n{card}");
        let read = feed::parse_body(&body, "ru").unwrap_or_else(|refusal| {
            panic!("a bad card refused the whole feed ({refusal:?}):\n{card}")
        });

        assert_eq!(read.cards.len(), 2, "only the bad card goes:\n{card}");
        assert_eq!(read.news.len(), 1, "the news stays:\n{card}");
        assert!(read.update.is_some(), "the update stays:\n{card}");
    }
}

/// **A field this build does not know costs nothing** — the trap of `[[item]]` is not built
/// again: a later version may add a key to `[[card]]` and this one reads the card past it. A view
/// it does not know is the plain template (А), a side it does not know is the start.
#[test]
fn an_unknown_field_or_view_of_a_card_is_read_past() {
    let body = "schema = 1\n\n[[card]]\nid = 3\nfrom = 2027-01-01\nto = 2027-01-02\n\
                view = \"spiral\"\nside = \"middle\"\nsparkles = 12\n\
                [card.frame]\ncolour = \"gold\"\n\
                [card.ru]\ntitle = \"а\"\ntext = \"б\"\nextra = 1\n";

    let read = feed::parse_body(body, "ru").expect("it parses");

    assert_eq!(
        read.cards.len(),
        1,
        "the card is read past what it does not know"
    );
    assert_eq!(
        read.cards[0].view,
        CardView::Text,
        "an unknown view is the plain template"
    );
    assert!(read.cards[0].image.is_none());

    let side = "schema = 1\n\n[[card]]\nid = 3\nfrom = 2027-01-01\nto = 2027-01-02\n\
                view = \"panel\"\nside = \"middle\"\n[card.ru]\ntitle = \"а\"\ntext = \"б\"\n";
    let read = feed::parse_body(side, "ru").expect("it parses");
    assert_eq!(read.cards[0].view, CardView::Panel(CardSide::Start));
}

/// **A picture the record does not describe whole is no picture** — the card stays and shows as
/// the plain template (А): an address, a sum of 64 hexadecimal digits and a size from one byte to
/// the cap, or nothing.
#[test]
fn a_picture_described_badly_leaves_the_card_without_one() {
    let card = |fields: &str| {
        format!(
            "schema = 1\n\n[[card]]\nid = 4\nfrom = 2027-01-01\nto = 2027-01-02\nview = \"full\"\n\
             {fields}\n[card.ru]\ntitle = \"а\"\ntext = \"б\"\n"
        )
    };
    let sum = "sha256 = \"00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff\"";
    let url = "image = \"https://panda-pishet-kod.dev/langswitcher/cards/a.png\"";

    let good = card(&format!("{url}\n{sum}\nsize = 100"));
    assert!(
        feed::parse_body(&good, "ru").expect("parses").cards[0]
            .image
            .is_some()
    );

    for fields in [
        format!("{sum}\nsize = 100"),
        format!("{url}\nsize = 100"),
        format!("{url}\n{sum}"),
        format!("{url}\n{sum}\nsize = 0"),
        format!(
            "{url}\n{sum}\nsize = {}",
            lang_switcher::cards::IMAGE_CAP + 1
        ),
        format!("{url}\nsha256 = \"0011\"\nsize = 100"),
        format!("{url}\nsha256 = \"{}\"\nsize = 100", "zz".repeat(32)),
    ] {
        let read = feed::parse_body(&card(&fields), "ru").expect("it parses");
        assert_eq!(read.cards.len(), 1, "the card stays:\n{fields}");
        assert!(read.cards[0].image.is_none(), "with no picture:\n{fields}");
    }
}

/// Only the newest cards are kept, oldest first — as many as [`lang_switcher::cards::CARDS_KEPT`].
#[test]
fn only_the_newest_cards_are_kept() {
    let mut body = String::from("schema = 1\n");
    for id in [5u64, 1, 9, 3, 7] {
        body.push_str(&format!(
            "\n[[card]]\nid = {id}\nfrom = 2027-01-01\nto = 2027-01-02\n[card.en]\ntitle = \"t\"\ntext = \"x\"\n"
        ));
    }

    let read = feed::parse_body(&body, "en").expect("it parses");
    let ids: Vec<u64> = read.cards.iter().map(|card| card.id).collect();

    assert_eq!(lang_switcher::cards::CARDS_KEPT, 3);
    assert_eq!(ids, [5, 7, 9]);
}

/// **The sentry of the stage: a feed without `[[card]]` is read exactly as before** — no card,
/// and the letters of the file the same as the parse of 1.8.0 makes of it.
#[test]
fn a_feed_without_cards_has_none() {
    let body = WITH_CARDS
        .split("\n[[card]]")
        .next()
        .expect("the part before the cards");
    let read = feed::parse_body(body, "ru").expect("it parses");

    assert!(read.cards.is_empty());
    assert_eq!(read.news.len(), 1);
    assert!(read.update.is_some());
}
