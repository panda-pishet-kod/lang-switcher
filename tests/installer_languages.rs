//! The installer speaks the language of Windows, and every language says everything — task
//! **T-91-6**, decisions 155.10…155.12.
//!
//! `installer\LangSwitcher.iss` lists fourteen languages: English first (the fallback), the twelve
//! official Inno Setup translations of the program's other languages, and Brazilian Portuguese
//! with the messages of Portuguese. Greek has no official translation and gets English. The
//! messages that are the installer's own live in `installer\lang\<Language>.isl`, one file per
//! language, read after the official file; `English.isl` is the sample the others translate.
//!
//! This sweep is what keeps that true after the next edit: a message added to the sample and to
//! the script but forgotten in one language would compile — Inno would fall back to an empty
//! string or to English without a word — and only a user of that language would ever see it.

use std::fs;
use std::path::Path;

/// The internal names of [Languages], in order: English first — it is the fallback.
const LANGUAGES: [&str; 14] = [
    "english",
    "russian",
    "ukrainian",
    "german",
    "french",
    "spanish",
    "portuguese",
    "italian",
    "polish",
    "czech",
    "turkish",
    "hebrew",
    "arabic",
    "brazilianportuguese",
];

/// Messages the script may use without defining them: Inno's own custom messages, translated in
/// every official `.isl` (`LaunchProgram` is the caption of the [Run] entry, decision 155.12).
const INNO_OWN: [&str; 1] = ["LaunchProgram"];

/// The languages whose messages must carry letters of their own script — a translation that came
/// out as Latin letters or as question marks is not one. (char range, language file)
const SCRIPTS: [(&str, char, char); 4] = [
    ("Russian.isl", '\u{0400}', '\u{04FF}'),
    ("Ukrainian.isl", '\u{0400}', '\u{04FF}'),
    ("Hebrew.isl", '\u{0590}', '\u{05FF}'),
    ("Arabic.isl", '\u{0600}', '\u{06FF}'),
];

fn installer() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/installer"))
}

/// The lines of one `[Section]` of an Inno file, without comments and blank lines.
fn section<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let header = format!("[{name}]");
    let mut inside = false;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            inside = line.eq_ignore_ascii_case(&header);
            continue;
        }
        if inside && !line.is_empty() && !line.starts_with(';') {
            out.push(line);
        }
    }
    out
}

/// The `[CustomMessages]` of one of our language files, as (name, value), in file order.
fn messages(file: &str) -> Vec<(String, String)> {
    let path = installer().join("lang").join(file);
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("{file}: {error}"));
    assert!(
        !bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
        "{file} starts with a byte order mark; the files of this folder are UTF-8 without one"
    );
    let text = String::from_utf8(bytes).unwrap_or_else(|_| panic!("{file} is not UTF-8"));
    section(&text, "CustomMessages")
        .into_iter()
        .map(|line| {
            let (name, value) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("{file}: not a message: {line}"));
            (name.trim().to_owned(), value.trim().to_owned())
        })
        .collect()
}

/// Every entry of `[Languages]`: its internal name and its files, in order.
fn language_entries(script: &str) -> Vec<(String, Vec<String>)> {
    section(script, "Languages")
        .into_iter()
        .map(|line| {
            let field = |key: &str| -> String {
                let at = line
                    .find(&format!("{key}: \""))
                    .unwrap_or_else(|| panic!("no {key} in: {line}"));
                let rest = &line[at + key.len() + 3..];
                rest[..rest.find('"').expect("a closing quote")].to_owned()
            };
            let files = field("MessagesFile")
                .split(',')
                .map(|file| file.trim().to_owned())
                .collect();
            (field("Name"), files)
        })
        .collect()
}

/// **Every language of the wizard has its own file, and every file says everything English says.**
///
/// Red, it names the language and the message. The checks: the fourteen languages in their order;
/// for each, the official file of its name and `lang\<the same name>`; no file of the folder that
/// no language reads; the same set of names as `English.isl` in every file, every value non-empty;
/// Brazilian Portuguese word for word Portuguese; the four non-Latin languages in their scripts.
#[test]
fn every_language_of_the_installer_has_its_file_and_every_message() {
    let script = fs::read_to_string(installer().join("LangSwitcher.iss"))
        .expect("installer\\LangSwitcher.iss must be readable");
    let entries = language_entries(&script);

    let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names, LANGUAGES,
        "[Languages] is not the fourteen languages of decision 155.10, English first"
    );

    let sample = messages("English.isl");
    assert!(
        sample.len() >= 4,
        "English.isl holds {} messages — fewer than the installer uses",
        sample.len()
    );
    let mut sample_names: Vec<&str> = sample.iter().map(|(name, _)| name.as_str()).collect();
    sample_names.sort_unstable();

    let mut used_files = Vec::new();
    let mut problems = Vec::new();

    for (name, files) in &entries {
        let ours: Vec<&String> = files
            .iter()
            .filter(|file| file.starts_with("lang\\"))
            .collect();
        if ours.len() != 1 {
            problems.push(format!("{name}: not exactly one file of lang\\: {files:?}"));
            continue;
        }
        let file = &ours[0]["lang\\".len()..];
        let official = if name == "english" {
            "compiler:Default.isl".to_owned()
        } else {
            format!("compiler:Languages\\{file}")
        };
        if files.first() != Some(&official) {
            problems.push(format!(
                "{name}: the official file is not first or is not {official}: {files:?}"
            ));
        }
        if !installer().join("lang").join(file).is_file() {
            problems.push(format!("{name}: installer\\lang\\{file} does not exist"));
            continue;
        }
        used_files.push(file.to_owned());

        let own = messages(file);
        let mut own_names: Vec<&str> = own.iter().map(|(name, _)| name.as_str()).collect();
        own_names.sort_unstable();
        for missing in &sample_names {
            if !own_names.contains(missing) {
                problems.push(format!("{file}: the message {missing} is missing"));
            }
        }
        for extra in &own_names {
            if !sample_names.contains(extra) {
                problems.push(format!("{file}: the message {extra} is not in English.isl"));
            }
        }
        for (message, value) in &own {
            if value.is_empty() {
                problems.push(format!("{file}: the message {message} is empty"));
            }
        }
        if let Some((_, low, high)) = SCRIPTS.iter().find(|(script, _, _)| *script == file) {
            for (message, value) in &own {
                if !value.chars().any(|c| (*low..=*high).contains(&c)) {
                    problems.push(format!(
                        "{file}: the message {message} carries no letter of its script"
                    ));
                }
            }
        }
    }

    let mut on_disk: Vec<String> = fs::read_dir(installer().join("lang"))
        .expect("installer\\lang must be readable")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    on_disk.sort();
    for file in &on_disk {
        if !used_files.contains(file) {
            problems.push(format!("installer\\lang\\{file} is read by no language"));
        }
    }

    if used_files.contains(&"Portuguese.isl".to_owned())
        && used_files.contains(&"BrazilianPortuguese.isl".to_owned())
        && messages("Portuguese.isl") != messages("BrazilianPortuguese.isl")
    {
        problems.push(
            "BrazilianPortuguese.isl does not carry the messages of Portuguese.isl (155.12)".into(),
        );
    }

    assert!(
        problems.is_empty(),
        "{} problem(s) with the languages of the installer:\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}

/// **The script asks for no message that the sample does not define.**
///
/// Every `{cm:Name…}` and `CustomMessage('Name')` of `LangSwitcher.iss` is a message of
/// `English.isl` or one of Inno's own; a name that is neither would be shown to the user as an
/// empty string. The counts in front make sure the search found the calls at all.
#[test]
fn the_script_uses_only_the_messages_the_sample_defines() {
    let script = fs::read_to_string(installer().join("LangSwitcher.iss"))
        .expect("installer\\LangSwitcher.iss must be readable");
    let sample: Vec<String> = messages("English.isl")
        .into_iter()
        .map(|(name, _)| name)
        .collect();

    let mut used = Vec::new();
    for (open, close) in [("{cm:", [',', '}']), ("CustomMessage('", ['\'', '\''])] {
        let mut rest = script.as_str();
        while let Some(at) = rest.find(open) {
            rest = &rest[at + open.len()..];
            let end = rest
                .find(|c: char| close.contains(&c))
                .expect("a message name ends");
            used.push(rest[..end].to_owned());
        }
    }

    assert!(
        used.iter().any(|name| name == "LaunchProgram"),
        "the caption of the [Run] entry, {{cm:LaunchProgram,…}}, was not found: {used:?}"
    );
    assert!(
        used.len() >= 5,
        "only {} message uses found in the script: {used:?}",
        used.len()
    );

    let unknown: Vec<&String> = used
        .iter()
        .filter(|name| !sample.contains(name) && !INNO_OWN.contains(&name.as_str()))
        .collect();
    assert!(
        unknown.is_empty(),
        "the script asks for messages English.isl does not define: {unknown:?}"
    );
}
