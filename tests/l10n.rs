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
    "ru", "en", "uk", "de", "fr", "es", "pt", "it", "pl", "cs", "tr", "el", "he",
    "ar", // shipped
    "hi", "bn", "ur", "pa", "mr", "te", "ta", "fa", "gu", "th", "kn", "or", "ml", "uz", "ne", "as",
    "az", "km", "kk", "sr", "ug", "bg", "hy", "lo", "be", "tt", "ka", "mk", // A
    "vi", "nl", "ro", "hu", "sv", "sq", "da", "fi", "hr", "sk", "nb", "lt", "sl", "bs", "lv", "et",
    "cy", "mt", "is", "lb", "mi", "nn", // B
];

/// A feed whose two entries — an update and a news — carry `[item.<tag>]` for every tag given,
/// each section about as long as a section of the live feed (9 674 bytes for 14 languages on
/// 2026-10-27, ≈ 690 bytes per language).
fn feed_of(tags: &[&str]) -> String {
    let filler = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(5);
    let mut body = format!("schema = {}\n", feed::FEED_SCHEMA);
    for (id, kind) in [(1, "update"), (2, "news")] {
        body.push_str(&format!(
            "\n[[item]]\nid = {id}\ntype = \"{kind}\"\nlink = \"\"\n"
        ));
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
        assert_eq!(
            read.news[0].title,
            format!("news {tag}"),
            "{tag}: its own section"
        );
        let update = read
            .update
            .unwrap_or_else(|| panic!("{tag}: the update entry"));
        assert_eq!(
            update.title,
            format!("update {tag}"),
            "{tag}: its own section of the update"
        );
    }

    // The fallbacks: a language the feed does not carry reads English, and without English
    // Russian. The control of the first loop: a reader that ignored the language would answer
    // the same title for every tag, and these two answers would not both hold.
    let read = feed::parse_body(&body, "xx").expect("the feed parses for any language");
    assert_eq!(
        read.news[0].title, "news en",
        "a language the feed lacks falls back to en"
    );
    let without_english: Vec<&str> = ALL_TAGS
        .iter()
        .copied()
        .filter(|tag| *tag != "en")
        .collect();
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
    assert_eq!(
        tags, ALL_TAGS,
        "the registry: 14 shipped, then A and B, in this order"
    );

    let shipped: Vec<&Entry> = registry.iter().filter(|e| e.group == "shipped").collect();
    assert_eq!(
        shipped.len(),
        Language::ALL.len(),
        "one shipped entry per language of the program"
    );
    for (entry, language) in shipped.iter().zip(Language::ALL) {
        assert_eq!(entry.tag, language.tag(), "the order of Language::ALL");
        assert_eq!(entry.langid, language.langid(), "{}: LANGID", entry.tag);
        assert_eq!(entry.rtl, language.is_rtl(), "{}: direction", entry.tag);
        assert_eq!(
            entry.endonym,
            language.native_name(),
            "{}: the name of the chooser",
            entry.tag
        );
    }

    let mut groups = BTreeMap::new();
    for entry in &registry {
        *groups.entry(entry.group.as_str()).or_insert(0) += 1;
        let expected_state = if entry.group == "shipped" {
            "shipped"
        } else {
            "planned"
        };
        assert_eq!(
            entry.state, expected_state,
            "{}: state follows the group",
            entry.tag
        );
        assert!(
            entry.script.len() == 4
                && entry
                    .script
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase())
                && entry.script.chars().skip(1).all(|c| c.is_ascii_lowercase()),
            "{}: script {:?} is not an ISO 15924 code",
            entry.tag,
            entry.script
        );
        assert!(
            !entry.name_en.is_empty() && !entry.endonym.is_empty(),
            "{}: names",
            entry.tag
        );
        assert!(
            entry.note.as_deref() != Some(""),
            "{}: an empty note",
            entry.tag
        );
    }
    assert_eq!(
        groups,
        BTreeMap::from([("A", 28), ("B", 22), ("shipped", 14)]),
        "the groups of 198.13"
    );

    for (what, values) in [
        (
            "tag",
            registry.iter().map(|e| e.tag.clone()).collect::<Vec<_>>(),
        ),
        (
            "langid",
            registry
                .iter()
                .map(|e| format!("{:04X}", e.langid))
                .collect(),
        ),
        ("inno", registry.iter().map(|e| e.inno.clone()).collect()),
    ] {
        let unique: BTreeSet<&String> = values.iter().collect();
        assert_eq!(
            unique.len(),
            values.len(),
            "every {what} of the registry is unique"
        );
    }

    let official_planned = registry
        .iter()
        .filter(|e| e.state == "planned" && e.inno_official)
        .count();
    assert_eq!(
        official_planned, 12,
        "premise П5: 12 of the 50 have an Inno translation"
    );

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
    assert_eq!(
        back(0x0FFF),
        "",
        "the negative control: no locale has the LANGID 0x0FFF"
    );
    const LOCALE_CUSTOM_UNSPECIFIED: u32 = 0x1000;
    assert_eq!(
        lcid("qq-QQ"),
        LOCALE_CUSTOM_UNSPECIFIED,
        "a made-up name gets the custom number"
    );
    assert!(
        registry()
            .iter()
            .all(|entry| u32::from(entry.langid) != LOCALE_CUSTOM_UNSPECIFIED),
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
        let written = unsafe {
            GetLocaleInfoEx(
                PCWSTR(name.as_ptr()),
                LOCALE_IREADINGLAYOUT,
                Some(&mut layout),
            )
        };
        let written = usize::try_from(written).unwrap_or(0);
        let rtl = String::from_utf16_lossy(&layout[..written.saturating_sub(1)]) == "1";
        if rtl != entry.rtl {
            wrong.push(format!("  {}: Windows reads it rtl = {rtl}", entry.tag));
        }
    }
    assert!(
        wrong.is_empty(),
        "the registry disagrees with Windows:\n{}",
        wrong.join("\n")
    );
}

// -----------------------------------------------------------------------------------------------
// The guard of the language files — task T-L10N-3
// -----------------------------------------------------------------------------------------------

/// The first line of every language file. Measured in stage L10N (200.3): rc.exe carries the code
/// page of app.rc into an included file, so for today's build the line is not load-bearing; it is
/// required anyway, so that a file read or compiled on its own says what it is.
const CODE_PAGE: &str = "#pragma code_page(65001)";

/// One language file as the guard reads it.
struct LanguageFile {
    /// The LANGIDs of its `LANGUAGE` statements, in order.
    langids: Vec<u16>,
    /// Its strings by identifier: the text between the quotes as written, escapes not undone.
    strings: BTreeMap<String, String>,
    /// What is wrong with the file by itself, each in the words of the refusal.
    faults: Vec<String>,
}

/// One string line of a `STRINGTABLE` block — `IDS_NAME   "text"` — as its identifier and the
/// text between the quotes. The escapes of an .rc string are the reason this is a parser and not
/// a split: `""` is a quote inside the string, a backslash starts an escape (`\\`, `\n`, `\t` —
/// the files use `\\` alone, for `HKCU\\…\\Run`), and anything but a comment after the closing
/// quote means a quote inside the text was not doubled.
fn string_line(line: &str) -> Result<(String, String), String> {
    let id: String = line
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if !id.starts_with("IDS_") {
        return Err(format!("строка блока не начинается с IDS_: {line}"));
    }
    let rest = line[id.len()..].trim_start();
    let Some(body) = rest.strip_prefix('"') else {
        return Err(format!("{id} — нет открывающей кавычки"));
    };
    let mut text = String::new();
    let mut chars = body.chars().peekable();
    loop {
        match chars.next() {
            None => return Err(format!("{id} — строка не закрыта кавычкой")),
            Some('"') if chars.peek() == Some(&'"') => {
                chars.next();
                text.push_str("\"\"");
            }
            Some('"') => break,
            Some('\\') => match chars.next() {
                Some(next @ ('\\' | 'n' | 't')) => {
                    text.push('\\');
                    text.push(next);
                }
                _ => {
                    return Err(format!(
                        "{id} — обратная черта без пары: одна черта в строке пишется двумя (\\\\)"
                    ));
                }
            },
            Some(other) => text.push(other),
        }
    }
    let tail: String = chars.collect();
    let tail = tail.trim();
    if !tail.is_empty() && !tail.starts_with("//") {
        return Err(format!(
            "{id} — после закрывающей кавычки лишнее «{tail}»: кавычка внутри строки пишется \
             двумя (\"\")"
        ));
    }
    Ok((id, text))
}

/// A `LANGUAGE 0x19, 0x01` statement as its LANGID, `MAKELANGID(primary, sub)`.
fn language_statement(line: &str) -> Option<u16> {
    let (primary, sub) = line.strip_prefix("LANGUAGE")?.split_once(',')?;
    let number = |text: &str| u16::from_str_radix(text.trim().strip_prefix("0x")?, 16).ok();
    Some((number(sub)? << 10) | number(primary)?)
}

/// Reads a language file: its code page line, its `LANGUAGE` statements and the strings of its
/// `STRINGTABLE` blocks; whatever does not fit the form is a fault named by file and line.
fn read_language_file(file: &str, text: &str) -> LanguageFile {
    let mut read = LanguageFile {
        langids: Vec::new(),
        strings: BTreeMap::new(),
        faults: Vec::new(),
    };
    if text.lines().next() != Some(CODE_PAGE) {
        read.faults.push(format!(
            "{file}: первая строка — не {CODE_PAGE}: файл должен сам назвать свою кодировку"
        ));
    }
    #[derive(PartialEq)]
    enum Place {
        Outside,
        AfterStringtable,
        InBlock,
    }
    let mut place = Place::Outside;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        match place {
            Place::Outside if line.starts_with("LANGUAGE") => match language_statement(line) {
                Some(langid) => read.langids.push(langid),
                None => read
                    .faults
                    .push(format!("{file}:{number}: не разобрана строка {line}")),
            },
            Place::Outside if line == "STRINGTABLE" => place = Place::AfterStringtable,
            Place::Outside if line == CODE_PAGE => {}
            Place::AfterStringtable if line == "BEGIN" => place = Place::InBlock,
            Place::InBlock if line == "END" => place = Place::Outside,
            Place::InBlock => match string_line(line) {
                Ok((id, string)) => {
                    if read.strings.insert(id.clone(), string).is_some() {
                        read.faults
                            .push(format!("{file}: {id} — встречается дважды"));
                    }
                }
                Err(reason) => read.faults.push(format!("{file}: {reason}")),
            },
            _ => read.faults.push(format!(
                "{file}:{number}: строка вне формы файла языка: {line}"
            )),
        }
    }
    if place != Place::Outside {
        read.faults
            .push(format!("{file}: блок STRINGTABLE не закрыт строкой END"));
    }
    read
}

/// The substitutions `{0}`, `{1}`… a string carries, as a set: a translation may move them, it
/// may not lose one or invent one.
fn placeholders(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() && after[digits.len()..].starts_with('}') {
            found.insert(format!("{{{digits}}}"));
        }
        rest = after;
    }
    found
}

/// Everything wrong with one language file against `en.rc` and the registry — each item one line
/// a translator fixes from. Empty means the file is good.
fn refusals(
    tag: &str,
    text: &str,
    english: &LanguageFile,
    langids: &BTreeMap<String, u16>,
) -> Vec<String> {
    let file = format!("{tag}.rc");
    let read = read_language_file(&file, text);
    let mut out = read.faults;

    match langids.get(tag) {
        None => out.push(format!(
            "{file}: языка «{tag}» нет в res/lang/languages.toml"
        )),
        Some(&expected) => {
            if read.langids.is_empty() {
                out.push(format!("{file}: нет строки LANGUAGE"));
            }
            for langid in &read.langids {
                if *langid != expected {
                    out.push(format!(
                        "{file}: LANGUAGE даёт 0x{langid:04X}, а в languages.toml у «{tag}» — \
                         0x{expected:04X}"
                    ));
                }
            }
        }
    }

    for id in english.strings.keys() {
        if !read.strings.contains_key(id) {
            out.push(format!("{file}: нет {id}"));
        }
    }
    for (id, string) in &read.strings {
        let Some(original) = english.strings.get(id) else {
            out.push(format!("{file}: лишний {id} — такого нет в en.rc"));
            continue;
        };
        if string.is_empty() {
            out.push(format!("{file}: {id} — пустая строка"));
        }
        let (want, have) = (placeholders(original), placeholders(string));
        for missing in want.difference(&have) {
            out.push(format!("{file}: {id} — нет подстановки {missing}"));
        }
        for extra in have.difference(&want) {
            out.push(format!(
                "{file}: {id} — лишняя подстановка {extra}: в en.rc её нет"
            ));
        }
    }
    out
}

/// The LANGIDs of the registry by tag.
fn registry_langids() -> BTreeMap<String, u16> {
    registry()
        .into_iter()
        .map(|entry| (entry.tag, entry.langid))
        .collect()
}

/// The English file, read by the same reader — the reference every file is held to.
fn english() -> LanguageFile {
    let files = resource_script::language_files();
    let (_, text) = files
        .iter()
        .find(|(tag, _)| tag == "en")
        .expect("app.rc must include res/lang/en.rc");
    let english = read_language_file("en.rc", text);
    assert!(
        english.faults.is_empty(),
        "en.rc itself:\n{}",
        english.faults.join("\n")
    );
    english
}

/// **Every language file has exactly the strings of `en.rc`, the same substitutions in each, no
/// empty string, well-formed escapes, its code page line and the LANGUAGE of the registry.**
///
/// The strings that read the same as English are listed, not refused: «OK», «Lang Switcher», the
/// names of keys and of `%APPDATA%` are the same in many languages, and only a reader can tell an
/// untranslated string from a legitimate one.
#[test]
fn every_language_file_holds_the_strings_of_english_and_nothing_else() {
    let english = english();
    let langids = registry_langids();
    let files = resource_script::language_files();
    assert_eq!(
        files.len(),
        Language::ALL.len(),
        "one included file per shipped language"
    );
    assert_eq!(
        english.strings.len(),
        237,
        "the strings of 1.0.0 (premise П1)"
    );

    let mut refused = Vec::new();
    for (tag, text) in &files {
        refused.extend(refusals(tag, text, &english, &langids));
        let read = read_language_file(&format!("{tag}.rc"), text);
        let same: Vec<&String> = read
            .strings
            .iter()
            .filter(|(id, string)| *tag != "en" && english.strings.get(*id) == Some(*string))
            .map(|(id, _)| id)
            .collect();
        if !same.is_empty() {
            println!(
                "{tag}.rc: {} совпали с английскими — {:?}",
                same.len(),
                same
            );
        }
    }
    assert!(
        refused.is_empty(),
        "{} отказов:\n{}",
        refused.len(),
        refused.join("\n")
    );
}

/// **The files on disk are the files app.rc includes, every shipped language has one, and every
/// file is a language of the registry.** A translation written and not included would build into
/// nothing; an include without its file would fail rc.exe with a message about a path.
#[test]
fn the_files_of_res_lang_are_the_files_app_rc_includes() {
    let folder = resource_script::root().join("res/lang");
    let on_disk: BTreeSet<String> = std::fs::read_dir(&folder)
        .expect("res/lang must be readable")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".rc"))
        .map(|name| format!("res/lang/{name}"))
        .collect();
    let included: BTreeSet<String> = resource_script::included_files().into_iter().collect();
    let mut wrong: Vec<String> = on_disk
        .difference(&included)
        .map(|path| format!("{path}: файл есть, но app.rc его не подключает (#include)"))
        .collect();
    wrong.extend(
        included
            .difference(&on_disk)
            .map(|path| format!("{path}: app.rc его подключает, а файла нет")),
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));

    let langids = registry_langids();
    for language in Language::ALL {
        let path = format!("res/lang/{}.rc", language.tag());
        assert!(
            included.contains(&path),
            "{}: the shipped language has no {path}",
            language.tag()
        );
    }
    for (tag, _) in resource_script::language_files() {
        assert!(
            langids.contains_key(&tag),
            "res/lang/{tag}.rc: «{tag}» is not in the registry"
        );
    }
}

/// **The guard's own control — one mutant per refusal, each red with its own reason.** Made in
/// memory out of the real `uk.rc`, so the control follows the file: each mutant must change the
/// text it is made of, the real file must pass, and the refusal must name the file, the string
/// and the reason.
#[test]
fn the_guard_refuses_each_kind_of_broken_file_with_its_reason() {
    let english = english();
    let langids = registry_langids();
    let files = resource_script::language_files();
    let (_, uk) = files
        .iter()
        .find(|(tag, _)| tag == "uk")
        .expect("uk.rc is included");
    assert!(
        refusals("uk", uk, &english, &langids).is_empty(),
        "the real uk.rc passes"
    );

    let line_of = |id: &str| -> String {
        uk.lines()
            .find(|line| line.trim_start().starts_with(&format!("{id} ")))
            .unwrap_or_else(|| panic!("uk.rc has a line for {id}"))
            .to_owned()
    };
    let state_pair = line_of("IDS_STATE_PAIR");
    let autostart = line_of("IDS_STATE_AUTOSTART");
    let panel = line_of("IDS_WHATSNEW_PANEL");
    assert!(
        state_pair.contains("{1}"),
        "the mutant of a substitution needs a {{1}}"
    );
    assert!(
        autostart.contains("\\\\"),
        "the mutant of an escape needs a \\\\"
    );

    let mutants: [(&str, String, &[&str]); 9] = [
        (
            "an identifier taken out",
            uk.replacen(&format!("{panel}\n"), "", 1),
            &["uk.rc: нет IDS_WHATSNEW_PANEL"],
        ),
        (
            "{1} turned into {2}",
            uk.replacen(&state_pair, &state_pair.replace("{1}", "{2}"), 1),
            &[
                "uk.rc: IDS_STATE_PAIR — нет подстановки {1}",
                "uk.rc: IDS_STATE_PAIR — лишняя подстановка {2}: в en.rc её нет",
            ],
        ),
        (
            "an empty string",
            uk.replacen(&panel, "    IDS_WHATSNEW_PANEL      \"\"", 1),
            &["uk.rc: IDS_WHATSNEW_PANEL — пустая строка"],
        ),
        (
            "a single backslash",
            uk.replacen(&autostart, &autostart.replace("\\\\", "\\"), 1),
            &["uk.rc: IDS_STATE_AUTOSTART — обратная черта без пары"],
        ),
        (
            "an identifier English does not have",
            uk.replacen(&panel, &format!("{panel}\n    IDS_L10N_EXTRA \"x\""), 1),
            &["uk.rc: лишний IDS_L10N_EXTRA — такого нет в en.rc"],
        ),
        (
            "a quote not doubled",
            uk.replacen(
                &panel,
                "    IDS_WHATSNEW_PANEL      \"Три \"нові\" зміни\"",
                1,
            ),
            &["uk.rc: IDS_WHATSNEW_PANEL — после закрывающей кавычки лишнее"],
        ),
        (
            "the code page line taken out",
            uk.replacen(&format!("{CODE_PAGE}\n"), "", 1),
            &["uk.rc: первая строка — не #pragma code_page(65001)"],
        ),
        (
            "the LANGUAGE of another language",
            uk.replacen("LANGUAGE 0x22, 0x01", "LANGUAGE 0x19, 0x01", 1),
            &["uk.rc: LANGUAGE даёт 0x0419, а в languages.toml у «uk» — 0x0422"],
        ),
        (
            "a block left open",
            uk.trim_end().strip_suffix("END").unwrap_or(uk).to_owned(),
            &["uk.rc: блок STRINGTABLE не закрыт строкой END"],
        ),
    ];

    for (what, mutant, expected) in mutants {
        assert_ne!(
            &mutant, uk,
            "{what}: the mutant must change the text it is made of"
        );
        let refused = refusals("uk", &mutant, &english, &langids);
        for reason in expected {
            assert!(
                refused.iter().any(|line| line.starts_with(reason)),
                "{what}: expected «{reason}…», the guard said:\n{}",
                refused.join("\n")
            );
        }
        println!("{what}: {}", refused.join(" | "));
    }
}

// -----------------------------------------------------------------------------------------------
// The context of the strings — res/lang/context.tsv, task T-L10N-5
// -----------------------------------------------------------------------------------------------

/// The rows of `context.tsv`: id, window, control, room_px, placeholders, note.
fn context() -> Vec<Vec<String>> {
    let path = resource_script::root().join("res/lang/context.tsv");
    let text = std::fs::read_to_string(&path).expect("res/lang/context.tsv must be readable");
    let mut lines = text
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty());
    assert_eq!(
        lines.next(),
        Some("id\twindow\tcontrol\troom_px\tplaceholders\tnote"),
        "the heading of context.tsv"
    );
    lines
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect()
}

/// **`context.tsv` has one row for every string of `en.rc` and for nothing else**, six cells in
/// each, a window and a room in every row, every substitution of the English string named in
/// its placeholders, and every control it names standing in the template of its window — so a
/// translator reading a row reads a place that exists.
#[test]
fn the_context_has_one_row_for_every_string_and_names_real_places() {
    let english = english();
    let rows = context();
    let script = resource_script::resource_script();

    // The controls of every template of app.rc, by dialog.
    let mut templates: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut dialog: Option<String> = None;
    for line in script.lines() {
        if let Some((name, _)) = line.split_once(" DIALOGEX") {
            dialog = Some(name.trim().to_owned());
            continue;
        }
        if line.trim() == "END" {
            dialog = None;
        }
        if let Some(dialog) = &dialog {
            for word in line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                if word.starts_with("IDC_") || word == "IDOK" || word == "IDCANCEL" {
                    templates
                        .entry(dialog.clone())
                        .or_default()
                        .insert(word.to_owned());
                }
            }
        }
    }

    let mut wrong = Vec::new();
    let mut seen = BTreeSet::new();
    for row in &rows {
        let id = row[0].as_str();
        if row.len() != 6 {
            wrong.push(format!("{id}: {} cells, not 6", row.len()));
            continue;
        }
        if !seen.insert(id.to_owned()) {
            wrong.push(format!("{id}: two rows"));
        }
        let Some(original) = english.strings.get(id) else {
            wrong.push(format!("{id}: a row for a string en.rc does not have"));
            continue;
        };
        for (cell, name) in [
            (1, "window"),
            (3, "room_px"),
            (4, "placeholders"),
            (5, "note"),
        ] {
            if row[cell].trim().is_empty() {
                wrong.push(format!("{id}: an empty {name}"));
            }
        }
        for placeholder in placeholders(original) {
            if !row[4].contains(&placeholder) {
                wrong.push(format!(
                    "{id}: placeholders do not say what {placeholder} carries"
                ));
            }
        }
        let windows: Vec<&str> = row[1].split(',').collect();
        for control in row[2]
            .split(',')
            .filter(|c| c.starts_with("IDC_") || c.starts_with("ID"))
        {
            let known = windows.iter().any(|window| {
                templates
                    .get(*window)
                    .is_some_and(|set| set.contains(control))
            });
            if !known {
                wrong.push(format!("{id}: {control} is in no template of {}", row[1]));
            }
        }
    }
    for id in english.strings.keys() {
        if !seen.contains(id) {
            wrong.push(format!("{id}: no row in context.tsv"));
        }
    }
    println!(
        "context.tsv: {} rows, {} templates",
        rows.len(),
        templates.len()
    );
    assert!(wrong.is_empty(), "context.tsv:\n{}", wrong.join("\n"));
}
