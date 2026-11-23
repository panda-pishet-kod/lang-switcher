//! **The windows excused from fitting** (208.175) — one list for the instrument of fitting
//! (`tests\ui_fit.rs`) and the tests of behaviour that fit a window themselves
//! (`tests\letters.rs`): each with its category and its term, printed, never skipped in silence.

#![allow(dead_code)]

use lang_switcher::ui::layout::Class;

/// What lifts an excuse, and by when (208.175).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// The wave of translations (T-120-24): a stand-in sample ×1,3, or a word wider than its
    /// column — the translator shortens it, from `l10n\E120-fit-wave.md`.
    Wave,
    /// The word of the owner: lifted by a decision, whose number is written in `decision`.
    Owner,
}

/// A window excused from fitting: printed by the instrument, never skipped in silence.
pub struct Excuse {
    pub window: &'static str,
    pub class: Class,
    /// The languages; empty — every one.
    pub languages: &'static [&'static str],
    /// None is never written: a record without a category is refused (208.175) —
    /// [`uncategorised`].
    pub category: Option<Category>,
    /// By when it is lifted.
    pub until: &'static str,
    /// The decision that settled it, for an excuse of the owner — a release asks it.
    pub decision: Option<&'static str>,
    pub why: &'static str,
}

/// **The windows excused from fitting** (208.175), each with its category and its term.
pub const EXCUSED: &[Excuse] = &[];

/// The excuse of `window` at `class` in the language `tag`, if there is one.
pub fn excused(window: &str, class: Class, tag: &str) -> Option<&'static Excuse> {
    EXCUSED.iter().find(|excuse| {
        excuse.window == window
            && excuse.class == class
            && (excuse.languages.is_empty() || excuse.languages.contains(&tag))
    })
}

/// How an excuse prints.
pub fn excuse_line(excuse: &Excuse) -> String {
    format!(
        "{}, {:?}, {} — {}",
        excuse.window,
        excuse.class,
        languages_words(excuse.languages),
        term_words(excuse.category, excuse.until, excuse.decision, excuse.why)
    )
}

/// The languages of a record as it prints them; empty — every one.
fn languages_words(languages: &[&str]) -> String {
    if languages.is_empty() {
        "все языки".to_owned()
    } else {
        languages.join(", ")
    }
}

/// The category, the term, the decision and the reason of a record as it prints them.
fn term_words(
    category: Option<Category>,
    until: &str,
    decision: Option<&str>,
    why: &str,
) -> String {
    let category = match category {
        Some(Category::Wave) => "волна переводов",
        Some(Category::Owner) => "слово владельца",
        None => "БЕЗ КАТЕГОРИИ",
    };
    format!(
        "{category}, срок: {until}{} — {why}",
        decision.map_or_else(String::new, |decision| format!(", решение {decision}"))
    )
}

/// A record without a category, a term or a reason — refused always (208.175).
fn is_uncategorised(category: Option<Category>, until: &str, why: &str) -> bool {
    category.is_none() || until.is_empty() || why.is_empty()
}

/// A record a release does not let through (208.175, T-120-24): any of the wave, and one of the
/// owner without the number of its decision.
fn is_unsettled(category: Option<Category>, decision: Option<&str>) -> bool {
    match category {
        Some(Category::Wave) | None => true,
        Some(Category::Owner) => decision.is_none(),
    }
}

/// The excuses written without a category or a term — refused always (208.175).
pub fn uncategorised(excuses: &[Excuse]) -> Vec<String> {
    excuses
        .iter()
        .filter(|excuse| is_uncategorised(excuse.category, excuse.until, excuse.why))
        .map(excuse_line)
        .collect()
}

/// The excuses a release does not let through (208.175, T-120-24): any of the wave, and one of the
/// owner without the number of its decision.
pub fn unsettled(excuses: &[Excuse]) -> Vec<String> {
    excuses
        .iter()
        .filter(|excuse| is_unsettled(excuse.category, excuse.decision))
        .map(excuse_line)
        .collect()
}

/// **A string excused from translation** (208.236 п. 6): it still reads as English in `languages`
/// while the wave takes it. The guard «строка ≠ английской» of `tests\l10n.rs` lets it through,
/// printed and never in silence; the gate of a release (`tests\ui_fit.rs`) lets no string of the
/// wave through, as it lets no window of the wave through.
pub struct Untranslated {
    /// The identifiers, `IDS_…`.
    pub ids: &'static [&'static str],
    /// The languages; empty — every one but English.
    pub languages: &'static [&'static str],
    /// None is never written: a record without a category is refused — [`uncategorised_strings`].
    pub category: Option<Category>,
    /// By when it is lifted.
    pub until: &'static str,
    /// The decision that settled it, for a record of the owner — a release asks it.
    pub decision: Option<&'static str>,
    pub why: &'static str,
}

/// **The strings excused from translation** (208.236 п. 6), each with its category and its term.
///
/// A string a stage adds in English to every language waits here for its wave — for example, as
/// the eighteen strings of the second hotkey would (3360…3377, 208.233):
///
/// ```text
/// Untranslated {
///     ids: &["IDS_HOTKEY_MODE_ONE", …],
///     languages: &[],
///     category: Some(Category::Wave),
///     until: "волна 18 строк второй клавиши",
///     decision: Some("208.233"),
///     why: "строки второй клавиши добавлены английскими во все языки до волны",
/// }
/// ```
pub const UNTRANSLATED: &[Untranslated] = &[];

/// The record of `records` that excuses the string `id` of the language `tag`, if there is one.
pub fn untranslated<'a>(
    records: &'a [Untranslated],
    id: &str,
    tag: &str,
) -> Option<&'a Untranslated> {
    records.iter().find(|record| {
        record.ids.contains(&id) && (record.languages.is_empty() || record.languages.contains(&tag))
    })
}

/// How a string excused from translation prints.
pub fn untranslated_line(record: &Untranslated) -> String {
    let ids = match record.ids {
        [] => "НИ ОДНОЙ СТРОКИ".to_owned(),
        [one] => (*one).to_owned(),
        [first, .., last] => format!("{first} … {last} ({})", record.ids.len()),
    };
    format!(
        "{ids}, {} — {}",
        languages_words(record.languages),
        term_words(record.category, record.until, record.decision, record.why)
    )
}

/// The strings excused without a category, a term, a reason or a string — refused always.
pub fn uncategorised_strings(records: &[Untranslated]) -> Vec<String> {
    records
        .iter()
        .filter(|record| {
            record.ids.is_empty() || is_uncategorised(record.category, record.until, record.why)
        })
        .map(untranslated_line)
        .collect()
}

/// The strings excused that a release does not let through — as [`unsettled`].
pub fn unsettled_strings(records: &[Untranslated]) -> Vec<String> {
    records
        .iter()
        .filter(|record| is_unsettled(record.category, record.decision))
        .map(untranslated_line)
        .collect()
}

/// The strings of `records`, and the languages of each, that excused nothing in a pass of the
/// guard which met the English strings `matched` (identifier, language); `tags` — the languages
/// of the pass but English, which a record of every language stands for. A string translated
/// since is taken away from its record, so that the record hides no English string to come.
pub fn stale_strings(
    records: &[Untranslated],
    matched: &[(String, String)],
    tags: &[String],
) -> Vec<String> {
    let mut found = Vec::new();
    for record in records {
        let languages: Vec<&str> = if record.languages.is_empty() {
            tags.iter().map(String::as_str).collect()
        } else {
            record.languages.to_vec()
        };
        for id in record.ids {
            let translated: Vec<&str> = languages
                .iter()
                .copied()
                .filter(|tag| {
                    !matched
                        .iter()
                        .any(|(m_id, m_tag)| m_id == id && m_tag == tag)
                })
                .collect();
            if !translated.is_empty() {
                found.push(format!(
                    "{id} уже не английская в: {} — снять из записи «{}»",
                    translated.join(", "),
                    untranslated_line(record)
                ));
            }
        }
    }
    found
}

/// The excuses — and the languages of each — that excused nothing in a pass of the instrument
/// which met the failures `matched` (window, class, language): an excuse no longer needed is taken
/// away, so that it hides no fault to come.
pub fn stale(matched: &[(String, Class, String)]) -> Vec<String> {
    let hit = |excuse: &Excuse, tag: Option<&str>| {
        matched.iter().any(|(window, class, language)| {
            window == excuse.window
                && *class == excuse.class
                && tag.is_none_or(|tag| tag == language)
        })
    };
    let mut found = Vec::new();
    for excuse in EXCUSED {
        if excuse.languages.is_empty() {
            if !hit(excuse, None) {
                found.push(excuse_line(excuse));
            }
        } else {
            for tag in excuse.languages {
                if !hit(excuse, Some(tag)) {
                    found.push(format!("{tag}: {}", excuse_line(excuse)));
                }
            }
        }
    }
    found
}
