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

/// The registry of the languages — the languages of groups A and B join the installer by it (stages
/// E108, E109).
#[path = "support/registry.rs"]
mod registry;

/// The script of a sign — the one judge of `tests\l10n.rs` too (stage E108, решение 202.8).
#[path = "support/scripts.rs"]
mod scripts;

/// The messages of Inno's own `Default.isl` 6.7.3 in `[Messages]` — counted in the file; a language
/// without an official translation carries every one of them in its own file (decision 202.2).
const INNO_MESSAGES: usize = 281;

/// The internal names of [Languages] of 1.2.0, in order: English first — it is the fallback.
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

/// The `[CustomMessages]` of Inno's own `Default.isl` (6.7.3) — a language without an official
/// translation carries them in its own file, translated with the rest of Inno's messages (stage
/// E108, decision 202.2); an official file carries them in its own half, so ours does not.
const INNO_DEFAULT_CUSTOM: [&str; 12] = [
    "NameAndVersion",
    "AdditionalIcons",
    "CreateDesktopIcon",
    "CreateQuickLaunchIcon",
    "ProgramOnTheWeb",
    "UninstallProgram",
    "LaunchProgram",
    "AssocFileExtension",
    "AssocingFileExtension",
    "AutoStartProgramGroupDescription",
    "AutoStartProgram",
    "AddonHostProgramNotFound",
];

/// The languages whose messages must carry letters of their own script — a translation that came
/// out as Latin letters or as question marks is not one. (char range, language file)
const SCRIPTS: [(&str, char, char); 4] = [
    ("Russian.isl", '\u{0400}', '\u{04FF}'),
    ("Ukrainian.isl", '\u{0400}', '\u{04FF}'),
    ("Hebrew.isl", '\u{0590}', '\u{05FF}'),
    ("Arabic.isl", '\u{0600}', '\u{06FF}'),
];

/// Stage E108, decision 202.2: the languages of group A the build ships — and since stage E109 those
/// of group B — after the fourteen, in the
/// order of the registry — `(internal name, Inno has its own translation, the file of lang\)`. The
/// name is the `inno` of the entry in lower case, the file is `<inno>.isl`.
fn groups_a_and_b_of_the_installer() -> Vec<(String, bool, String)> {
    registry::shipped()
        .into_iter()
        .filter(|entry| matches!(entry.group.as_str(), "A" | "B"))
        .map(|entry| {
            (
                entry.inno.to_lowercase(),
                entry.inno_official,
                format!("{}.isl", entry.inno),
            )
        })
        .collect()
}

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

/// Whether a line of an Inno file names something in straight apostrophes — `'Telepítés'` — and
/// not in the quotation marks of its language (RULES п. 16; the stand of 1.4.0, stage E109).
///
/// An opening apostrophe stands after a sign that is not a letter and before a letter; its closing one
/// after a letter and before a sign that is not one — so the elision of «d'Layout» or «a'r» is no name.
fn names_in_straight_apostrophes(line: &str) -> bool {
    let value = line.split_once('=').map_or(line, |(_, value)| value);
    let signs: Vec<char> = value.chars().collect();
    let letter = |at: usize| signs.get(at).is_some_and(|sign| sign.is_alphanumeric());

    (0..signs.len()).any(|open| {
        signs[open] == '\''
            && (open == 0 || !letter(open - 1))
            && letter(open + 1)
            && (open + 1..signs.len())
                .find(|&close| signs[close] == '\'')
                .is_some_and(|close| letter(close - 1) && !letter(close + 1))
    })
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
    let added = groups_a_and_b_of_the_installer();
    let wanted: Vec<&str> = LANGUAGES
        .iter()
        .copied()
        .chain(added.iter().map(|(name, _, _)| name.as_str()))
        .collect();
    assert_eq!(
        names, wanted,
        "[Languages] is not the fourteen languages of decision 155.10, English first, and the \
         shipped languages of groups A and B after them (202.2)"
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
        // Stage E108: a language of group A or B without a translation of Inno's own starts from
        // Default.isl, and its own file carries all of Inno's messages (202.2).
        let unofficial = added
            .iter()
            .any(|(of, official, _)| of == name && !official);
        let official = if name == "english" || unofficial {
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
            let inno_default = unofficial && INNO_DEFAULT_CUSTOM.contains(extra);
            if !sample_names.contains(extra) && !inno_default {
                problems.push(format!("{file}: the message {extra} is not in English.isl"));
            }
        }
        // Stage E108: without an official file, Inno's own custom messages come from ours alone —
        // every one of them, or the wizard shows them in English.
        if unofficial {
            for inno in INNO_DEFAULT_CUSTOM {
                if !own_names.contains(&inno) {
                    problems.push(format!("{file}: Inno's own message {inno} is missing"));
                }
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

/// The tokens of Pascal code, the way the compiler of `[Code]` reads it: identifiers and keywords
/// in lower case, a string `'…'` as one token with its quotes, `:=` as one, every other sign
/// alone — and **no comments**: `{ … }`, `(* … *)` and `// …` are skipped, so a statement that is
/// only written in a comment is not there.
fn pascal_tokens(code: &str) -> Vec<String> {
    let chars: Vec<char> = code.chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;

    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();

        if c.is_whitespace() {
            at += 1;
        } else if c == '{' {
            at = chars[at..]
                .iter()
                .position(|&d| d == '}')
                .map_or(chars.len(), |end| at + end + 1);
        } else if c == '(' && next == Some('*') {
            at = (at + 2..chars.len().saturating_sub(1))
                .find(|&i| chars[i] == '*' && chars[i + 1] == ')')
                .map_or(chars.len(), |end| end + 2);
        } else if c == '/' && next == Some('/') {
            at = chars[at..]
                .iter()
                .position(|&d| d == '\n')
                .map_or(chars.len(), |end| at + end);
        } else if c == '\'' {
            let end = chars[at + 1..]
                .iter()
                .position(|&d| d == '\'')
                .map_or(chars.len(), |end| at + 1 + end + 1);
            tokens.push(chars[at..end].iter().collect::<String>().to_lowercase());
            at = end;
        } else if c.is_alphanumeric() || c == '_' {
            let end = chars[at..]
                .iter()
                .position(|&d| !(d.is_alphanumeric() || d == '_'))
                .map_or(chars.len(), |end| at + end);
            tokens.push(chars[at..end].iter().collect::<String>().to_lowercase());
            at = end;
        } else if c == ':' && next == Some('=') {
            tokens.push(":=".to_owned());
            at += 2;
        } else {
            tokens.push(c.to_string());
            at += 1;
        }
    }

    tokens
}

/// The event of the wizard the routine holding `statement` is attached to — `InitializeWizard` or
/// `CurPageChanged`, by an `<event('…')>` attribute in front of its header or by its own name — or
/// why there is none. `statement` is Pascal, tokenised the same way as `code`.
fn wizard_event_holding(code: &[String], statement: &str) -> Result<String, String> {
    const EVENTS: [&str; 2] = ["initializewizard", "curpagechanged"];

    let wanted = pascal_tokens(statement);
    let at = code
        .windows(wanted.len())
        .position(|window| window == wanted.as_slice())
        .ok_or_else(|| format!("«{statement}» is not in the code"))?;

    let header = code[..at]
        .iter()
        .rposition(|token| token == "procedure" || token == "function")
        .ok_or_else(|| format!("«{statement}» stands in no routine"))?;
    // What stands between the end of the routine before and this header: the attribute, if any.
    let start = code[..header]
        .windows(2)
        .rposition(|pair| pair[0] == "end" && pair[1] == ";")
        .map_or(0, |end| end + 2);
    let front = &code[start..header];
    let name = code.get(header + 1).map_or("", String::as_str);

    EVENTS
        .iter()
        .find(|event| {
            let attribute = ["<", "event", "(", &format!("'{event}'"), ")", ">"];
            name == **event
                || front
                    .windows(attribute.len())
                    .any(|window| window.iter().zip(&attribute).all(|(a, b)| a == b))
        })
        .map(|event| (*event).to_owned())
        .ok_or_else(|| {
            format!("«{statement}» stands in «{name}», which is not an event of the wizard")
        })
}

/// The two statements that make the field of the Ready page wrap — task T-98-3, decision 166.3.
const READY_MEMO_WRAPS: [&str; 2] = [
    "WizardForm.ReadyMemo.WordWrap := True;",
    "WizardForm.ReadyMemo.ScrollBars := ssVertical;",
];

/// **The field of the Ready page wraps its lines — task T-98-3, finding 165.3 of the owner.**
///
/// «Готово к установке» is the first page the wizard shows — the pages of the folder and of the
/// group are off — and its field carries the sentence about the certificate, `LocalCertReady`:
/// one line, longer than the field in every one of the fourteen languages. The field of Inno Setup
/// does not wrap unless the script tells it to, and the owner saw the sentence run off the right
/// edge of the window. The script turns on the wrap by words and the vertical scroll bar of
/// `WizardForm.ReadyMemo` in an event of the wizard; a statement in a comment of any of the three
/// kinds, or in a string, does not count.
///
/// ⚠ Отрицательные контроли below, on scripts written here: the statements in comments and in a
/// string are not found, and in a routine that is no event of the wizard they are refused.
#[test]
fn the_field_of_the_ready_page_wraps_the_sentence_about_the_certificate() {
    let script = fs::read_to_string(installer().join("LangSwitcher.iss"))
        .expect("installer\\LangSwitcher.iss must be readable");
    let code = pascal_tokens(&section(&script, "Code").join("\n"));

    assert!(
        code.windows(2)
            .any(|pair| pair[0] == "function" && pair[1] == "updatereadymemo"),
        "the [Code] of the script was not found: no UpdateReadyMemo in {} tokens",
        code.len()
    );

    let problems: Vec<String> = READY_MEMO_WRAPS
        .iter()
        .filter_map(|statement| wizard_event_holding(&code, statement).err())
        .collect();

    assert!(
        problems.is_empty(),
        "the field of the Ready page does not wrap:\n  {}",
        problems.join("\n  ")
    );

    // ⚠ Отрицательные контроли: the instrument must be able to say «no».
    let commented = pascal_tokens(concat!(
        "<event('InitializeWizard')>\n",
        "procedure Wrap();\n",
        "begin\n",
        "  { WizardForm.ReadyMemo.WordWrap := True; }\n",
        "  (* WizardForm.ReadyMemo.WordWrap := True; *)\n",
        "  // WizardForm.ReadyMemo.WordWrap := True;\n",
        "  Log('WizardForm.ReadyMemo.WordWrap := True;');\n",
        "end;\n",
    ));
    assert!(
        wizard_event_holding(&commented, READY_MEMO_WRAPS[0]).is_err(),
        "a statement in a comment or in a string must not count"
    );

    let elsewhere = pascal_tokens(concat!(
        "<event('InitializeWizard')>\n",
        "procedure Other();\n",
        "begin\n",
        "end;\n",
        "\n",
        "procedure Wrap();\n",
        "begin\n",
        "  WizardForm.ReadyMemo.WordWrap := True;\n",
        "end;\n",
    ));
    assert!(
        wizard_event_holding(&elsewhere, READY_MEMO_WRAPS[0]).is_err(),
        "a statement in a routine that is no event of the wizard must not count"
    );

    let right = pascal_tokens(concat!(
        "procedure Other();\n",
        "begin\n",
        "end;\n",
        "\n",
        "<event('InitializeWizard')>\n",
        "procedure Wrap();\n",
        "begin\n",
        "  WizardForm.ReadyMemo.WordWrap := True;\n",
        "end;\n",
    ));
    assert_eq!(
        wizard_event_holding(&right, READY_MEMO_WRAPS[0]),
        Ok("initializewizard".to_owned()),
        "the control: the statement in an event of the wizard must be found"
    );
}

/// The address the signature of the author opens — task T-99-3, decision 167.1.
const AUTHOR_SITE: &str = "https://panda-pishet-kod.dev/";

/// The scales of the pictures, in the order of the table of the Inno help — `tests\installer_art.rs`
/// holds the files to their sizes.
const ART_SCALES: [&str; 5] = ["100", "125", "150", "175", "200"];

/// How many times the code opens [`AUTHOR_SITE`] — every one of them as
/// `ShellExecAsOriginalUser('open', '<address>', …`, the browser of the person and not the rights of
/// Setup — or why not. `code` is tokenised by [`pascal_tokens`]: an address in a comment is not
/// there.
fn site_opens_as_the_user(code: &[String]) -> Result<usize, String> {
    let address = format!("'{AUTHOR_SITE}'");
    let call = ["shellexecasoriginaluser", "(", "'open'", ","];
    let mut found = 0;
    for (at, token) in code.iter().enumerate() {
        if *token != address {
            continue;
        }
        if at < call.len() || code[at - call.len()..at] != call {
            return Err(format!(
                "{address} is opened otherwise: «{}»",
                code[at.saturating_sub(call.len())..at].join(" ")
            ));
        }
        found += 1;
    }
    if found == 0 {
        return Err(format!("{address} is not in the code"));
    }
    Ok(found)
}

/// **The wizard wears the look of the site the owner chose, and its signature opens the site as the
/// person — task T-99-3, decision 167.1 (variant B with the signature B1 of question 167).**
///
/// `[Setup]`: `WizardStyle` is exactly `modern dynamic windows11 excludelightcontrols hidebevels`;
/// the background is `#FAF9F6` and in the dark theme `#23272F`; the four pictures of Inno's own are
/// empty — the two `…DynamicDark` as well, or the dark theme puts Inno's picture back (decision
/// 167.20); the background pictures are the bamboo of `installer\art` for the five scales, light and
/// dark, in order, and every one of them is on disk. `[Files]`: the ten logos, each on disk.
/// `[Code]`, without comments: the address of the site is opened only by
/// `ShellExecAsOriginalUser('open', …)` and never by `ShellExec`; the slogan is set with the wizard
/// (`InitializeWizard`), and the line without «Back» over the field of the Ready page when the page
/// is shown (`CurPageChanged`).
///
/// ⚠ Отрицательные контроли below: the address in a comment is not found, and opened by `ShellExec`
/// it is refused.
#[test]
fn the_wizard_wears_the_look_of_the_site_and_its_signature_opens_the_site_as_the_person() {
    let script = fs::read_to_string(installer().join("LangSwitcher.iss"))
        .expect("installer\\LangSwitcher.iss must be readable");
    let mut problems = Vec::new();

    let setup = section(&script, "Setup");
    let directive = |key: &str| -> Option<String> {
        let found: Vec<&str> = setup
            .iter()
            .filter_map(|line| line.split_once('='))
            .filter(|(name, _)| name.trim().eq_ignore_ascii_case(key))
            .map(|(_, value)| value.trim())
            .collect();
        match found.as_slice() {
            [one] => Some((*one).to_owned()),
            _ => None,
        }
    };
    let check = |key: &str, want: &str| match directive(key) {
        Some(value) if value.eq_ignore_ascii_case(want) => None,
        Some(value) => Some(format!("{key}={value} — wanted «{want}»")),
        None => Some(format!("{key} is not in [Setup] exactly once")),
    };

    let style: Vec<String> = directive("WizardStyle")
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_lowercase)
        .collect();
    let mut sorted = style.clone();
    sorted.sort();
    let mut wanted = [
        "dynamic",
        "excludelightcontrols",
        "hidebevels",
        "modern",
        "windows11",
    ];
    wanted.sort_unstable();
    if sorted != wanted {
        problems.extend(check(
            "WizardStyle",
            "modern dynamic windows11 excludelightcontrols hidebevels",
        ));
    }
    problems.extend(check("WizardBackColor", "#FAF9F6"));
    problems.extend(check("WizardBackColorDynamicDark", "#23272F"));
    for empty in [
        "WizardImageFile",
        "WizardImageFileDynamicDark",
        "WizardSmallImageFile",
        "WizardSmallImageFileDynamicDark",
    ] {
        problems.extend(check(empty, ""));
    }
    for (key, theme) in [
        ("WizardBackImageFile", "light"),
        ("WizardBackImageFileDynamicDark", "dark"),
    ] {
        let list: Vec<String> = ART_SCALES
            .iter()
            .map(|scale| format!("art\\back-{theme}-{scale}.png"))
            .collect();
        problems.extend(check(key, &list.join(",")));
        for picture in &list {
            if !installer().join(picture).is_file() {
                problems.push(format!("{key}: installer\\{picture} does not exist"));
            }
        }
    }

    let files = section(&script, "Files");
    for theme in ["light", "dark"] {
        for scale in ART_SCALES {
            let logo = format!("art\\logo-{theme}-{scale}.png");
            let entry = format!("Source: \"{logo}\"; Flags: dontcopy");
            if !files.contains(&entry.as_str()) {
                problems.push(format!("[Files] has no «{entry}»"));
            }
            if !installer().join(&logo).is_file() {
                problems.push(format!("[Files]: installer\\{logo} does not exist"));
            }
        }
    }

    let code = pascal_tokens(&section(&script, "Code").join("\n"));
    assert!(
        code.windows(2)
            .any(|pair| pair[0] == "function" && pair[1] == "updatereadymemo"),
        "the [Code] of the script was not found: no UpdateReadyMemo in {} tokens",
        code.len()
    );
    if let Err(why) = site_opens_as_the_user(&code) {
        problems.push(why);
    }
    // The signature is made once, with the wizard. The line over the field is set when the page
    // is shown: Inno sets its own line again then, and one set with the wizard came back with
    // «Back» on the live window (decision 167.21).
    for (statement, wanted) in [
        ("CustomMessage('AuthorSlogan')", "initializewizard"),
        (
            "WizardForm.ReadyLabel.Caption := SetupMessage(msgReadyLabel2b);",
            "curpagechanged",
        ),
    ] {
        match wizard_event_holding(&code, statement) {
            Ok(event) if event == wanted => {}
            Ok(event) => problems.push(format!("«{statement}» stands in {event}, not {wanted}")),
            Err(why) => problems.push(why),
        }
    }

    assert!(
        problems.is_empty(),
        "{} problem(s) with the look of the wizard:\n  {}",
        problems.len(),
        problems.join("\n  ")
    );

    // ⚠ Отрицательные контроли: the instrument must be able to say «no».
    let commented = pascal_tokens(concat!(
        "{ ShellExecAsOriginalUser('open', 'https://panda-pishet-kod.dev/', '', '', SW_SHOWNORMAL, ewNoWait, E); }\n",
        "// ShellExecAsOriginalUser('open', 'https://panda-pishet-kod.dev/', '', '', SW_SHOWNORMAL, ewNoWait, E);\n",
    ));
    assert!(
        site_opens_as_the_user(&commented).is_err(),
        "an address in a comment must not count"
    );
    let elevated = pascal_tokens(concat!(
        "ShellExecAsOriginalUser('open', 'https://panda-pishet-kod.dev/', '', '', SW_SHOWNORMAL, ewNoWait, E);\n",
        "ShellExec('open', 'https://panda-pishet-kod.dev/', '', '', SW_SHOWNORMAL, ewNoWait, E);\n",
    ));
    assert!(
        site_opens_as_the_user(&elevated).is_err(),
        "the address opened by ShellExec, with the rights of Setup, must be refused"
    );
    assert_eq!(
        site_opens_as_the_user(&pascal_tokens(
            "ShellExecAsOriginalUser('open', 'https://panda-pishet-kod.dev/', '', '', SW_SHOWNORMAL, ewNoWait, E);"
        )),
        Ok(1),
        "the control: the address opened as the person must be found"
    );
}

/// **Stage E108, decision 202.2 — every language of group A speaks its own language in the
/// installer, and the installer knows which language it is.**
///
/// Stage E109: the languages of group B as well — all Latin, so the sweep of the script stands
/// aside for them, as it does for every Latin language; the options and the messages of Inno do not.
///
/// For a language without an official translation of Inno, our file is the whole translation:
/// `[LangOptions]` names the language by the LANGID of the registry (Inno picks the language of
/// the wizard by it), reads right to left exactly when the registry says so, and `[Messages]`
/// carries every one of the 281 messages of `Default.isl` — the same names in every such file.
/// For a language Inno translates (ta, th, bg, hy), our file is only our messages, like the
/// fourteen. In both, every message of `[CustomMessages]` carries letters of the language's own
/// script (the registry's `script`), and none of another non-Latin script — the lesson of the
/// trial of Hindi (решение 202.8).
///
/// ⚠ Control: a file whose LANGID is another language's, one without `RightToLeft` for a language
/// read right to left, and a message in Latin letters for a non-Latin language are each caught.
#[test]
fn every_language_of_group_a_speaks_its_own_language_in_the_installer() {
    fn options_of(text: &str) -> Vec<(String, String)> {
        section(text, "LangOptions")
            .into_iter()
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
            .collect()
    }

    fn message_names(text: &str) -> Vec<String> {
        let mut names: Vec<String> = section(text, "Messages")
            .into_iter()
            .filter_map(|line| line.split_once('='))
            .map(|(key, _)| key.trim().to_owned())
            .collect();
        names.sort();
        names
    }

    /// What is wrong with one file of group A, as lines.
    fn problems_of(entry: &registry::Entry, text: &str, own: &[(String, String)]) -> Vec<String> {
        let file = format!("{}.isl", entry.inno);
        let mut out = Vec::new();
        let options = options_of(text);
        let option = |key: &str| {
            options
                .iter()
                .find(|(of, _)| of.eq_ignore_ascii_case(key))
                .map(|(_, value)| value.as_str())
        };

        if entry.inno_official {
            if !options.is_empty() {
                out.push(format!(
                    "{file}: Inno translates {} itself — our file carries only our messages",
                    entry.tag
                ));
            }
            // Stage E109 (the stand of 1.4.0): a message of Inno's own translation may be given again
            // here only to put the quotation marks of the language round a name the official file
            // writes in straight apostrophes (RULES п. 16) — so no message here may keep them.
            for line in section(text, "Messages") {
                if names_in_straight_apostrophes(line) {
                    out.push(format!(
                        "{file}: [Messages] gives a message of Inno again and keeps the straight \
                         apostrophes — «{line}»"
                    ));
                }
            }
        } else {
            let langid = format!("${:04X}", entry.langid);
            if option("LanguageID").map(str::to_uppercase) != Some(langid.clone()) {
                out.push(format!(
                    "{file}: LanguageID {:?}, the registry says {langid}",
                    option("LanguageID")
                ));
            }
            if option("LanguageName").is_none_or(str::is_empty) {
                out.push(format!("{file}: no LanguageName"));
            }
            let rtl = option("RightToLeft").is_some_and(|value| value.eq_ignore_ascii_case("yes"));
            if rtl != entry.rtl {
                out.push(format!(
                    "{file}: RightToLeft is {rtl}, the registry reads {} right to left: {}",
                    entry.tag, entry.rtl
                ));
            }
            let count = message_names(text).len();
            if count != INNO_MESSAGES {
                out.push(format!(
                    "{file}: {count} messages of Inno, Default.isl has {INNO_MESSAGES}"
                ));
            }
        }

        if entry.script != "Latn" {
            for (message, value) in own {
                let scripts: Vec<&str> =
                    value.chars().filter_map(scripts::script_of_char).collect();
                if !scripts.contains(&entry.script.as_str()) {
                    out.push(format!(
                        "{file}: {message} carries no letter of {} — «{value}»",
                        entry.script
                    ));
                }
                if let Some(other) = scripts.iter().find(|script| **script != entry.script) {
                    out.push(format!(
                        "{file}: {message} carries a sign of {other} — «{value}»"
                    ));
                }
            }
        }

        out
    }

    let added: Vec<registry::Entry> = registry::shipped()
        .into_iter()
        .filter(|entry| matches!(entry.group.as_str(), "A" | "B"))
        .collect();

    let mut problems = Vec::new();
    let mut names_of_unofficial: Option<(String, Vec<String>)> = None;

    for entry in &added {
        let file = format!("{}.isl", entry.inno);
        let text = fs::read_to_string(installer().join("lang").join(&file))
            .unwrap_or_else(|error| panic!("{file}: {error}"));
        problems.extend(problems_of(entry, &text, &messages(&file)));

        if !entry.inno_official {
            let names = message_names(&text);
            match &names_of_unofficial {
                None => names_of_unofficial = Some((file.clone(), names)),
                Some((first, theirs)) if *theirs != names => problems.push(format!(
                    "{file}: the names of [Messages] are not those of {first}"
                )),
                Some(_) => {}
            }
        }
    }

    // The controls, on the first language of group A read right to left and not in Latin.
    if let Some(entry) = added
        .iter()
        .find(|entry| entry.rtl && !entry.inno_official && entry.script != "Latn")
    {
        let file = format!("{}.isl", entry.inno);
        let text = fs::read_to_string(installer().join("lang").join(&file)).expect("read above");
        let own = messages(&file);
        let langid = format!("${:04X}", entry.langid);

        // The keys as lines of the file: the header of a file may name them in a comment.
        let other_langid =
            text.replacen(&format!("\nLanguageID={langid}"), "\nLanguageID=$0409", 1);
        let no_rtl = text.replacen("\nRightToLeft=yes", "\nRightToLeft=no", 1);
        let mut latin = own.clone();
        latin[0].1 = "Latin letters only".to_owned();

        for (what, mutant, own) in [
            ("another LANGID", &other_langid, &own),
            ("no RightToLeft", &no_rtl, &own),
            ("a message in Latin letters", &text, &latin),
        ] {
            assert!(
                *mutant != text || own != &messages(&file),
                "{what}: the control must change what it is made of"
            );
            assert!(
                !problems_of(entry, mutant, own).is_empty(),
                "the control: {what} in {file} must be caught"
            );
        }
    }

    assert!(
        problems.is_empty(),
        "{} problem(s) with the languages of groups A and B in the installer:\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}

/// **Stage E109, the stand of 1.4.0 — the installer in Hungarian names its buttons in Hungarian
/// quotation marks.** Inno's own Hungarian translation (`compiler:Languages\Hungarian.isl`, Inno Setup
/// 6.7.3) writes nineteen of its messages with a button or a file in straight apostrophes — «Kattintson
/// a 'Telepítés'-re» on the page «ready to install» (FAIL of the stand, `reports\shots-E109\README.md`).
/// Our `Hungarian.isl` gives exactly those nineteen again, with „…” instead of the apostrophes and not a
/// word otherwise changed (the lines were made out of the official file by replacing the marks alone).
///
/// ⚠ Control: the official line itself is caught by the rule; an elision of another language is not.
#[test]
fn the_official_hungarian_messages_name_their_buttons_in_hungarian_quotation_marks() {
    /// The messages of the official Hungarian translation of Inno Setup 6.7.3 that name something in
    /// straight apostrophes — measured on the file, stage E109.
    const STRAIGHT_IN_THE_OFFICIAL_FILE: [&str; 19] = [
        "SetupAppRunningError",
        "UninstallAppRunningError",
        "ClickNext",
        "BrowseDialogLabel",
        "PasswordLabel3",
        "InfoBeforeClickLabel",
        "InfoAfterClickLabel",
        "SelectDirBrowseLabel",
        "SelectComponentsLabel2",
        "SelectTasksLabel2",
        "SelectStartMenuFolderBrowseLabel",
        "ReadyLabel2a",
        "ReadyLabel2b",
        "CannotContinue",
        "ClickFinish",
        "ShowReadmeCheck",
        "SelectDiskLabel2",
        "ErrorOpeningReadme",
        "ConfirmDeleteSharedFile2",
    ];

    let text = fs::read_to_string(installer().join("lang").join("Hungarian.isl"))
        .expect("Hungarian.isl is read");
    let given: Vec<(String, String)> = section(&text, "Messages")
        .into_iter()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let names: Vec<&str> = given.iter().map(|(name, _)| name.as_str()).collect();

    assert_eq!(
        names, STRAIGHT_IN_THE_OFFICIAL_FILE,
        "Hungarian.isl gives again exactly the messages the official file writes with straight \
         apostrophes"
    );
    for (name, value) in &given {
        assert!(
            value.contains('„') && value.contains('”') && !names_in_straight_apostrophes(value),
            "{name}: the name stands in „…”, not in straight apostrophes — «{value}»"
        );
    }

    assert!(
        names_in_straight_apostrophes("ReadyLabel2b=Kattintson a 'Telepítés'-re a folytatáshoz."),
        "the control: the line of the official file must be caught"
    );
    assert!(
        !names_in_straight_apostrophes("d'Layout gouf op Lëtzebuergesch ëmgeschalt, a'r cynllun"),
        "the control: an elision is not a name in apostrophes"
    );
}
