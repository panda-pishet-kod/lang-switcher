//! **The languages of the interface — stage L10N** (TZ L10N; decisions 198.14, 198.16).
//!
//! The strings live in `res/lang/<tag>.rc`, one file per language, and the translations of the
//! stages after this one are written by windows that read little and see no screen. This file
//! is what such a window runs to check its own work (`cargo test --test l10n`), and every
//! refusal names the file, the string and the reason in words a translator acts on:
//! «hi.rc: IDS_X — нет подстановки {1}».
//!
//! * premise П4 — a feed of all 64 languages is read by the reader of today;
//! * the registry `res/lang/languages.toml` (T-L10N-4) against `Language::ALL` and against
//!   Windows itself;
//! * the guard of the files (T-L10N-3) against `en.rc`, with its mutants;
//! * the context `res/lang/context.tsv` (T-L10N-5): one row for every string and only for them.

#[path = "support/resource_script.rs"]
mod resource_script;

use lang_switcher::letters::feed;

/// The tags of the registry, in its order — the 14 the program ships, then groups A and B.
const ALL_TAGS: [&str; 64] = [
    "ru", "en", "uk", "de", "fr", "es", "pt", "it", "pl", "cs", "tr", "el", "he", "ar", // shipped
    "hi", "bn", "ur", "pa", "mr", "te", "ta", "fa", "gu", "th", "kn", "or", "ml", "uz", "ne",
    "as", "az", "km", "kk", "sr", "ug", "bg", "hy", "lo", "be", "tt", "ka", "mk", // A
    "vi", "nl", "ro", "hu", "sv", "sq", "da", "fi", "hr", "sk", "nb", "lt", "sl", "bs", "lv",
    "et", "cy", "mt", "is", "lb", "mi", "nn", // B
];

/// A feed whose two entries — an update and a news — carry `[item.<tag>]` for every tag given,
/// each section about as long as a section of the live feed (9 674 bytes for 14 languages on
/// 2026-10-27, ≈ 690 bytes per language).
fn feed_of(tags: &[&str]) -> String {
    let filler = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(5);
    let mut body = format!("schema = {}\n", feed::FEED_SCHEMA);
    for (id, kind) in [(1, "update"), (2, "news")] {
        body.push_str(&format!("\n[[item]]\nid = {id}\ntype = \"{kind}\"\nlink = \"\"\n"));
        if kind == "update" {
            body.push_str("version = \"1.1.0\"\n");
        }
        for tag in tags {
            body.push_str(&format!(
                "[item.{tag}]\ntitle = \"{kind} {tag}\"\ntext = \"{tag}: {filler}\"\n"
            ));
        }
    }
    body
}

/// **Premise П4 of TZ L10N.** A feed with sections for all 64 languages stays under the ceiling
/// of the response, and the reader of today picks the interface language out of it for every
/// one of them; a language the feed does not carry falls back to `en`, and without `en` to `ru`
/// — the rule `[language, "en", "ru"]` of `letters::feed`. Nothing of the reader changes on a new
/// language: the sections are a map from the tag.
///
/// ⚠ The reader of 0.104.0 (tag `e104`) is the same code by reading — `RawFeed` without
/// `deny_unknown_fields`, the sections a flattened map — and is **not measured** here.
#[test]
fn a_feed_of_all_sixty_four_languages_is_read_in_each_of_them() {
    let body = feed_of(&ALL_TAGS);
    println!(
        "feed of {} languages: {} bytes, ceiling {} bytes",
        ALL_TAGS.len(),
        body.len(),
        feed::RESPONSE_CAP
    );
    assert!(
        body.len() < feed::RESPONSE_CAP / 4,
        "64 languages must leave the ceiling of the response far away: {} bytes",
        body.len()
    );

    for tag in ALL_TAGS {
        let read = feed::parse_body(&body, tag)
            .unwrap_or_else(|refusal| panic!("{tag}: the feed must parse, refused {refusal:?}"));
        assert_eq!(read.news.len(), 1, "{tag}: the news entry");
        assert_eq!(read.news[0].title, format!("news {tag}"), "{tag}: its own section");
        let update = read.update.unwrap_or_else(|| panic!("{tag}: the update entry"));
        assert_eq!(update.title, format!("update {tag}"), "{tag}: its own section of the update");
    }

    // The fallbacks: a language the feed does not carry reads English, and without English
    // Russian. The control of the first loop: a reader that ignored the language would answer
    // the same title for every tag, and these two answers would not both hold.
    let read = feed::parse_body(&body, "xx").expect("the feed parses for any language");
    assert_eq!(read.news[0].title, "news en", "a language the feed lacks falls back to en");
    let without_english: Vec<&str> = ALL_TAGS.iter().copied().filter(|tag| *tag != "en").collect();
    let read = feed::parse_body(&feed_of(&without_english), "xx").expect("it parses");
    assert_eq!(read.news[0].title, "news ru", "and without en to ru");
}
