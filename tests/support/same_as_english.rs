//! **The strings that read as English and are right** (208.236 п. 6) — the list of the guard
//! «строка ≠ английской заглушке» of `tests\l10n.rs`.
//!
//! A string of a language that is the very text of `en.rc` is refused by that guard: it is how an
//! untranslated stand-in looks, and before the guard nothing caught one (TR-L10N-180c: English put
//! back into `de.rc` left `l10n` and `ui_fit` green). Some are right all the same — the word is
//! the language's own, or the international one its Windows writes. Each such string is a record
//! here, by identifier and language, with its reason.
//!
//! * A string with no letter — `" → "`, `", "`, `"{0} · {1}"` — is not a word of any language, and
//!   the guard does not ask for it; it needs no record.
//! * A string waiting for its wave is not right, only late: its record goes into
//!   `tests\support\excused.rs` (`UNTRANSLATED`, category «волна переводов»), not here.
//! * A record that no longer matches — the language translated the string, or the string left
//!   `en.rc` — is refused, so that it hides no English string to come.

/// A string of the languages `languages` that reads as the English one and is right.
pub struct Same {
    /// The identifier, `IDS_…`.
    pub id: &'static str,
    /// The languages, by tag — never empty: whether a word is a language's own is a question of
    /// each language.
    pub languages: &'static [&'static str],
    /// Why it is right.
    pub why: &'static str,
}

/// The record of `records` for the string `id` of the language `tag`, if there is one.
pub fn same<'a>(records: &'a [Same], id: &str, tag: &str) -> Option<&'a Same> {
    records
        .iter()
        .find(|record| record.id == id && record.languages.contains(&tag))
}

/// **The strings that read as English and are right**, as of release 1.8.0 (208.236 п. 6): every
/// one the guard met on `release/1.8.0` at `42c4561`, read by the waves of L10N that wrote them.
pub const SAME_AS_ENGLISH: &[Same] = &[
    Same {
        id: "IDS_OK",
        languages: &[
            "az", "bg", "cs", "da", "de", "el", "et", "fi", "fr", "hu", "it", "ka", "lb", "mt",
            "nb", "nl", "nn", "pl", "pt", "ro", "sk", "sq", "sv", "uz", "uz-Cyrl", "vi",
        ],
        why: "«OK» — международная подпись кнопки: Windows на этих языках пишет её латиницей и \
              не переводит",
    },
    Same {
        id: "IDS_WIZARD_PROGRAM",
        languages: &[
            "bs", "cs", "da", "hr", "hu", "nb", "nn", "pl", "ro", "sk", "sl", "sr-Latn", "sv", "tr",
        ],
        why: "«Program» — своё слово этих языков («программа»), пишется так же",
    },
    Same {
        id: "IDS_ABOUT_VERSION",
        languages: &["da", "fr", "sv"],
        why: "«version» — своё слово датского, французского и шведского",
    },
    Same {
        id: "IDS_AUTOSTART_ABSENT",
        languages: &["es", "it"],
        why: "«no» — испанское и итальянское «нет»",
    },
    Same {
        id: "IDS_CORNER_UPDATE",
        languages: &["nl"],
        why: "«update» — нидерландское слово для обновления программы",
    },
    Same {
        id: "IDS_FEEDBACK_PANEL",
        languages: &["da", "mt", "nl", "ro", "sv"],
        why: "«Feedback» — принятое в этих языках заимствование, так пишет и их Windows",
    },
    Same {
        id: "IDS_GROUP_GENERAL",
        languages: &["es", "ro"],
        why: "«General» — испанское и румынское «общие»",
    },
    Same {
        id: "IDS_GROUP_LAYOUTS",
        languages: &["da", "de", "pt"],
        why: "«Layouts» — принятое в этих языках заимствование для раскладок клавиатуры",
    },
    Same {
        id: "IDS_LAYOUT_WORD_SOURCE",
        languages: &["fr"],
        why: "«source» — французское «источник»",
    },
    Same {
        id: "IDS_NEWS_LATER",
        languages: &["nl"],
        why: "«Later» — нидерландское «позже»",
    },
    Same {
        id: "IDS_WIZARD_CONTACT_LINE",
        languages: &["nl", "ro"],
        why: "«Contact:» — нидерландское и румынское «связь»",
    },
    Same {
        id: "IDS_WIZARD_SEND",
        languages: &["da", "nb", "nn"],
        why: "«Send» — датское и норвежское «отправь»",
    },
];
