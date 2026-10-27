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
use lang_switcher::settings::Language;

use std::collections::{BTreeMap, BTreeSet};

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

// -----------------------------------------------------------------------------------------------
// The registry — res/lang/languages.toml, task T-L10N-4
// -----------------------------------------------------------------------------------------------

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    language: Vec<Entry>,
}

/// One language of the registry; the meaning of every field is written at the top of the file.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    tag: String,
    locale: String,
    langid: u16,
    name_en: String,
    endonym: String,
    script: String,
    rtl: bool,
    inno: String,
    inno_official: bool,
    group: String,
    state: String,
    #[serde(default)]
    note: Option<String>,
}

fn registry() -> Vec<Entry> {
    let path = resource_script::root().join("res/lang/languages.toml");
    let text = std::fs::read_to_string(&path).expect("res/lang/languages.toml must be readable");
    toml::from_str::<Registry>(&text)
        .unwrap_or_else(|error| panic!("languages.toml must parse: {error}"))
        .language
}

/// **The registry holds the 64 languages of decision 198.14, and its 14 shipped entries are the
/// program's own `Language::ALL` — tag, LANGID, direction and the name of the chooser.**
///
/// Groups and states are closed sets; tags, LANGIDs and installer names are unique; 12 of the
/// 50 planned languages have a translation of their own in Inno Setup 6.7.3 (premise П5); every
/// shipped language with an official Inno translation has the installer's own file
/// `installer/lang/<inno>.isl`, Greek — the one without — has none (155.11).
#[test]
fn the_registry_holds_sixty_four_languages_and_the_shipped_ones_are_the_programs() {
    let registry = registry();
    let tags: Vec<&str> = registry.iter().map(|entry| entry.tag.as_str()).collect();
    assert_eq!(tags, ALL_TAGS, "the registry: 14 shipped, then A and B, in this order");

    let shipped: Vec<&Entry> = registry.iter().filter(|e| e.group == "shipped").collect();
    assert_eq!(shipped.len(), Language::ALL.len(), "one shipped entry per language of the program");
    for (entry, language) in shipped.iter().zip(Language::ALL) {
        assert_eq!(entry.tag, language.tag(), "the order of Language::ALL");
        assert_eq!(entry.langid, language.langid(), "{}: LANGID", entry.tag);
        assert_eq!(entry.rtl, language.is_rtl(), "{}: direction", entry.tag);
        assert_eq!(entry.endonym, language.native_name(), "{}: the name of the chooser", entry.tag);
    }

    let mut groups = BTreeMap::new();
    for entry in &registry {
        *groups.entry(entry.group.as_str()).or_insert(0) += 1;
        let expected_state = if entry.group == "shipped" { "shipped" } else { "planned" };
        assert_eq!(entry.state, expected_state, "{}: state follows the group", entry.tag);
        assert!(
            entry.script.len() == 4
                && entry.script.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                && entry.script.chars().skip(1).all(|c| c.is_ascii_lowercase()),
            "{}: script {:?} is not an ISO 15924 code",
            entry.tag,
            entry.script
        );
        assert!(!entry.name_en.is_empty() && !entry.endonym.is_empty(), "{}: names", entry.tag);
        assert!(entry.note.as_deref() != Some(""), "{}: an empty note", entry.tag);
    }
    assert_eq!(
        groups,
        BTreeMap::from([("A", 28), ("B", 22), ("shipped", 14)]),
        "the groups of 198.13"
    );

    for (what, values) in [
        ("tag", registry.iter().map(|e| e.tag.clone()).collect::<Vec<_>>()),
        ("langid", registry.iter().map(|e| format!("{:04X}", e.langid)).collect()),
        ("inno", registry.iter().map(|e| e.inno.clone()).collect()),
    ] {
        let unique: BTreeSet<&String> = values.iter().collect();
        assert_eq!(unique.len(), values.len(), "every {what} of the registry is unique");
    }

    let official_planned = registry
        .iter()
        .filter(|e| e.state == "planned" && e.inno_official)
        .count();
    assert_eq!(official_planned, 12, "premise П5: 12 of the 50 have an Inno translation");

    for entry in &shipped {
        let own = resource_script::root().join(format!("installer/lang/{}.isl", entry.inno));
        assert_eq!(
            own.is_file(),
            entry.inno_official,
            "{}: installer/lang/{}.isl exists exactly when Inno has the language",
            entry.tag,
            entry.inno
        );
    }
}

/// The UTF-16 of a string with its terminating zero.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// **Every LANGID of the registry is the one Windows gives its locale, both ways, and the
/// direction is the one Windows reads it in** — the measurement of T-L10N-4 repeated on every
/// run (`LocaleNameToLCID`, `LCIDToLocaleName`, `LOCALE_IREADINGLAYOUT`).
///
/// ⚠ The negative control is part of the test, because it is not the obvious one:
/// `LocaleNameToLCID` does not answer 0 to a name it does not know — it answers 0x1000,
/// `LOCALE_CUSTOM_UNSPECIFIED` — and `LCIDToLocaleName(0x1000)` then answers **the made-up name
/// itself** (measured on 2026-10-27 with «qq-QQ»: the first version of this test expected no way
/// back and was red). A round trip alone does not tell a real locale from a fake; the number
/// does, and every entry is held to its own number of the registry, which is never 0x1000.
#[test]
fn the_langids_of_the_registry_are_the_ones_windows_gives() {
    use windows::Win32::Globalization::{
        GetLocaleInfoEx, LCIDToLocaleName, LOCALE_IREADINGLAYOUT, LocaleNameToLCID,
    };
    use windows::core::PCWSTR;

    let back = |lcid: u32| -> String {
        let mut name = [0u16; 128];
        // SAFETY: the buffer is a live array of this frame and its length goes in with the slice.
        let written = unsafe { LCIDToLocaleName(lcid, Some(&mut name), 0) };
        let written = usize::try_from(written).unwrap_or(0);
        String::from_utf16_lossy(&name[..written.saturating_sub(1)])
    };
    let lcid = |locale: &str| -> u32 {
        let name = wide(locale);
        // SAFETY: `name` is a NUL-terminated UTF-16 string that outlives the call.
        unsafe { LocaleNameToLCID(PCWSTR(name.as_ptr()), 0) }
    };

    assert_eq!(lcid("ru-RU"), 0x0419, "the positive control");
    assert_eq!(back(0x0419), "ru-RU", "and back");
    assert_eq!(back(0x0FFF), "", "the negative control: no locale has the LANGID 0x0FFF");
    const LOCALE_CUSTOM_UNSPECIFIED: u32 = 0x1000;
    assert_eq!(lcid("qq-QQ"), LOCALE_CUSTOM_UNSPECIFIED, "a made-up name gets the custom number");
    assert!(
        registry().iter().all(|entry| u32::from(entry.langid) != LOCALE_CUSTOM_UNSPECIFIED),
        "no entry of the registry may stand on the number a made-up name gets"
    );

    let mut wrong = Vec::new();
    for entry in registry() {
        let measured = lcid(&entry.locale);
        if measured != u32::from(entry.langid) || back(measured) != entry.locale {
            wrong.push(format!(
                "  {}: {} gives 0x{measured:04X} and back {:?}, the registry says 0x{:04X}",
                entry.tag,
                entry.locale,
                back(measured),
                entry.langid
            ));
            continue;
        }
        let name = wide(&entry.locale);
        let mut layout = [0u16; 8];
        // SAFETY: as above — a NUL-terminated name and a live buffer with its length.
        let written =
            unsafe { GetLocaleInfoEx(PCWSTR(name.as_ptr()), LOCALE_IREADINGLAYOUT, Some(&mut layout)) };
        let written = usize::try_from(written).unwrap_or(0);
        let rtl = String::from_utf16_lossy(&layout[..written.saturating_sub(1)]) == "1";
        if rtl != entry.rtl {
            wrong.push(format!("  {}: Windows reads it rtl = {rtl}", entry.tag));
        }
    }
    assert!(wrong.is_empty(), "the registry disagrees with Windows:\n{}", wrong.join("\n"));
}
