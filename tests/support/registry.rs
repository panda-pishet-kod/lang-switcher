//! **The registry of the languages of the interface as the tests read it** — stage E108, task
//! T-108-2.
//!
//! `res/lang/languages.toml` is the one list of languages: since stage E108 `build.rs` reads the
//! entries that say `state = "shipped"` out of it and the program offers exactly those. The tables
//! of the tests that used to carry a copy of the fourteen — tag, LANGID, the name of the chooser —
//! read the registry instead, through this module, so a language that joins the program with its
//! file and its line of the registry joins the tests the same moment. What a test still must not
//! borrow from the code under test is the **answer**: the registry is data apart from the code,
//! and `tests\l10n.rs` holds the code's `Language` to it value by value.
//!
//! Included by `#[path]` from each test file that needs it, as `resource_script.rs` is.

#![allow(dead_code)]

use std::path::PathBuf;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    language: Vec<Entry>,
}

/// One language of the registry; the meaning of every field is written at the top of the file.
#[derive(serde::Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub tag: String,
    pub locale: String,
    pub langid: u16,
    pub name_en: String,
    pub endonym: String,
    pub script: String,
    pub rtl: bool,
    pub inno: String,
    pub inno_official: bool,
    pub group: String,
    pub state: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Every entry of `res/lang/languages.toml`, in its order.
pub fn registry() -> Vec<Entry> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("res/lang/languages.toml");
    let text = std::fs::read_to_string(&path).expect("res/lang/languages.toml must be readable");
    toml::from_str::<Registry>(&text)
        .unwrap_or_else(|error| panic!("languages.toml must parse: {error}"))
        .language
}

/// The values of the closed set of `general.language` — the fourteen of 1.0.0 and group A (stage
/// E108), in the order of the registry, which is the order of `Language::ALL`. Group B is the
/// stage after (E109) and not a value yet.
pub fn closed_set() -> Vec<Entry> {
    registry()
        .into_iter()
        .filter(|entry| entry.group != "B")
        .collect()
}

/// The languages the build offers — `state = "shipped"` — in the order of the registry.
pub fn shipped() -> Vec<Entry> {
    registry()
        .into_iter()
        .filter(|entry| entry.state == "shipped")
        .collect()
}

/// `(tag, LANGID, the name of the chooser)` of each entry — the shape of the old tables.
pub fn locales(entries: &[Entry]) -> Vec<(String, u16, String)> {
    entries
        .iter()
        .map(|entry| (entry.tag.clone(), entry.langid, entry.endonym.clone()))
        .collect()
}
