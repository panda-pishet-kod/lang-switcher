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
pub const EXCUSED: &[Excuse] = &[
    // AUDIT-180 А1 (208.211): the title of «От автора» is the first sentence of the owner's text and
    // the card says the rest. The wave split the old translation of every language the same way
    // (L10N-180b, without a word lost); these languages still do not fit the window of 900 × 540
    // even at 12 points: the owner's text is not cut (the owner was promised), so the owner decides.
    Excuse {
        window: "author",
        class: Class::Ordinary,
        languages: &["hy", "ka", "ml", "te", "uz", "uz-Cyrl"],
        category: Some(Category::Wave),
        until: "волна переводов 1.8.0: «От автора» не влезает при кегле 12 — решает владелец (L10N-180b)",
        decision: None,
        why: "заголовок (hy, ka, ml) или текст карточки (te, uz, uz-Cyrl) длиннее окна; текст не режется",
    },
];

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
    let languages = if excuse.languages.is_empty() {
        "все языки".to_owned()
    } else {
        excuse.languages.join(", ")
    };
    let category = match excuse.category {
        Some(Category::Wave) => "волна переводов",
        Some(Category::Owner) => "слово владельца",
        None => "БЕЗ КАТЕГОРИИ",
    };
    format!(
        "{}, {:?}, {languages} — {category}, срок: {}{} — {}",
        excuse.window,
        excuse.class,
        excuse.until,
        excuse
            .decision
            .map_or_else(String::new, |decision| format!(", решение {decision}")),
        excuse.why
    )
}

/// The excuses written without a category or a term — refused always (208.175).
pub fn uncategorised(excuses: &[Excuse]) -> Vec<String> {
    excuses
        .iter()
        .filter(|excuse| {
            excuse.category.is_none() || excuse.until.is_empty() || excuse.why.is_empty()
        })
        .map(excuse_line)
        .collect()
}

/// The excuses a release does not let through (208.175, T-120-24): any of the wave, and one of the
/// owner without the number of its decision.
pub fn unsettled(excuses: &[Excuse]) -> Vec<String> {
    excuses
        .iter()
        .filter(|excuse| match excuse.category {
            Some(Category::Wave) | None => true,
            Some(Category::Owner) => excuse.decision.is_none(),
        })
        .map(excuse_line)
        .collect()
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
