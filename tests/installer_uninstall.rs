//! Uninstalling a running program closes it first, never leaves it half removed — decisions
//! 155.23, 155.24 and 155.25.
//!
//! Inno's uninstaller does not close programs: the Restart Manager of `CloseApplications` serves
//! Setup only. A running `LangSwitcher.exe` keeps its file locked, the uninstaller skips the file,
//! and the program goes on living in the tray of a computer it was removed from — seen on a clean
//! machine at the acceptance of e91 (155.23). The repair (155.25, the owner's word after 155.24)
//! closes it the way Windows does at sign-out: before anything is removed, an `InitializeUninstall`
//! handler looks for the mutex the product holds (FR-82), posts `WM_ENDSESSION` to the product's
//! one top-level window — the product answers with its one cleanup path, FR-83 — and waits for the
//! program to be gone. Only a program still there is left to the person, with Inno's own
//! `UninstallAppRunningError`; Cancel, and the default answer of a suppressed message box, abort.
//!
//! The product and the installer name the mutex and the window class separately, in two
//! languages, and nothing but this test ties the names together: a name changed in `src\app.rs`
//! would compile on both sides and turn the closing into one that never happens.

use std::fs;
use std::path::Path;

/// The attribute that marks the handler; the script may hold several `InitializeUninstall`
/// implementations (Inno calls them all and needs every one to return True).
const HANDLER: &str = "<event('InitializeUninstall')>";

/// What the handler must contain, in this order: nothing to do without the mutex; the window of
/// the product; the message of the end of a session posted to it; the wait; and only then the
/// person — Inno's own message, Cancel as the answer of a suppressed box, and the refusal.
const STEPS: [&str; 8] = [
    "if not CheckForMutexes('{#ProductMutex}') then",
    "Wnd := FindWindowByClassName('{#ProductWindowClass}');",
    "PostMessage(Wnd, LangSwEndSession, 1, LangSwCloseApp)",
    "if ProgramIsGone(ExeFile, LangSwCloseWaitMs) then",
    "while CheckForMutexes('{#ProductMutex}') do",
    "SetupMessage(msgUninstallAppRunningError)",
    "MB_OKCANCEL, IDCANCEL",
    "Result := False;",
];

/// The values the handler sends: `WM_ENDSESSION` and `ENDSESSION_CLOSEAPP` of `WinUser.h`.
const CONSTANTS: [&str; 2] = ["LangSwEndSession = $0016;", "LangSwCloseApp = $00000001;"];

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The string literal of `const <name>: PCWSTR = w!(…)` in `src/app.rs`, raw or not.
fn product_literal(name: &str) -> String {
    let source = read("src/app.rs");
    let prefix = format!("const {name}: PCWSTR = w!(");
    let line = source
        .lines()
        .map(str::trim_start)
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("src/app.rs no longer declares `{prefix}…)`"));
    let rest = &line[prefix.len()..];
    let start = rest.find('"').expect("the literal is opened") + 1;
    let end = start + rest[start..].find('"').expect("the literal is closed");
    rest[start..end].to_string()
}

/// Every reason why `script` does not close a running program before uninstalling it, or does
/// not refuse when the program stays; empty when it does both.
fn reasons_it_does_not_close_or_refuse(script: &str, mutex: &str, class: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    for define in [
        format!("#define ProductMutex \"{mutex}\""),
        format!("#define ProductWindowClass \"{class}\""),
    ] {
        if !script.lines().any(|line| line.trim() == define) {
            reasons.push(format!(
                "no line `{define}`: the script names another one or none"
            ));
        }
    }
    for constant in CONSTANTS {
        if !script.lines().any(|line| line.trim().starts_with(constant)) {
            reasons.push(format!("no constant `{constant}`"));
        }
    }
    let lines: Vec<&str> = script.lines().collect();
    match lines.iter().position(|line| line.trim() == HANDLER) {
        None => reasons.push(format!("no routine marked {HANDLER}")),
        Some(start) => {
            let body: Vec<&str> = lines[start..]
                .iter()
                .take_while(|line| line.trim_end() != "end;")
                .map(|line| line.trim())
                .collect();
            let mut from = 0;
            for step in STEPS {
                match body[from..].iter().position(|line| line.contains(step)) {
                    Some(found) => from += found + 1,
                    None => reasons.push(format!(
                        "the handler lacks `{step}` (or has it out of order)"
                    )),
                }
            }
        }
    }
    let in_setup = script
        .lines()
        .skip_while(|line| line.trim() != "[Setup]")
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with('['));
    for line in in_setup {
        if line
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("appmutex")
        {
            reasons.push(format!(
                "[Setup] has `{}`: it would stop Setup too and replace the update path that closes \
                 the product by itself (CloseApplications)",
                line.trim()
            ));
        }
    }
    reasons
}

#[test]
fn the_uninstaller_closes_the_running_program_and_asks_only_if_it_stays() {
    let script = read("installer/LangSwitcher.iss");
    let reasons = reasons_it_does_not_close_or_refuse(
        &script,
        &product_literal("MUTEX_NAME"),
        &product_literal("WINDOW_CLASS_NAME"),
    );
    assert!(
        reasons.is_empty(),
        "installer/LangSwitcher.iss would leave a running program behind (155.23, 155.25):\n  {}",
        reasons.join("\n  ")
    );
}

#[test]
fn a_missing_step_and_a_misspelt_name_are_both_told_apart_from_the_real_script() {
    let script = read("installer/LangSwitcher.iss");
    let mutex = product_literal("MUTEX_NAME");
    let class = product_literal("WINDOW_CLASS_NAME");
    let check = |text: &str, mutex: &str, class: &str| {
        reasons_it_does_not_close_or_refuse(text, mutex, class)
    };
    assert!(
        check(&script, &mutex, &class).is_empty(),
        "the real script must pass before its copies can fail meaningfully"
    );
    assert_eq!(
        check(&script, &format!("{mutex}x"), &class).len(),
        1,
        "a mutex name one letter off must be caught"
    );
    assert_eq!(
        check(&script, &mutex, &format!("{class}x")).len(),
        1,
        "a window class one letter off must be caught"
    );
    let without_handler = script.replace(HANDLER, "{ no handler }");
    assert!(
        !check(&without_handler, &mutex, &class).is_empty(),
        "a script without the handler must be caught"
    );
    let without_closing = script.replace(STEPS[2], "Log('nothing posted')");
    assert!(
        !check(&without_closing, &mutex, &class).is_empty(),
        "a handler that never asks the program to close must be caught"
    );
    let without_refusal = script.replace(STEPS[7], "Result := True;");
    assert!(
        !check(&without_refusal, &mutex, &class).is_empty(),
        "a handler that never refuses must be caught"
    );
    let wrong_message = script.replace(CONSTANTS[0], "LangSwEndSession = $0010;");
    assert!(
        !check(&wrong_message, &mutex, &class).is_empty(),
        "a message other than WM_ENDSESSION must be caught"
    );
    let with_app_mutex = script.replace("[Setup]\r\n", "[Setup]\r\nAppMutex=x\r\n");
    assert_ne!(
        with_app_mutex, script,
        "the script has a [Setup] header line"
    );
    assert!(
        !check(&with_app_mutex, &mutex, &class).is_empty(),
        "an AppMutex directive in [Setup] must be caught"
    );
}

// ---------------------------------------------------------------------------------------------
// What the uninstaller leaves in %APPDATA%\Lang_Switcher — decisions 177.1 and 177.20.1, backlog
// Q-171-1, stage E101.
//
// The journal goes always: it is not a setting, and nobody is asked about it; with it the
// temporaries of its write, `diag.log.<pid>.tmp`, which nothing else sweeps. The question «remove
// the settings file too?» is about the settings alone: on Yes `config.toml`, `config.toml.bad` and
// the appeals saved from the wizard «Написать автору» go; on No and in a silent uninstall they stay.
// Then the folder goes, if nothing is left in it. Until e100 the Yes removed `config.toml` alone,
// and the folder stayed with the journal in it.
//
// The installer and the product name these files separately, in two languages, and nothing but
// this test ties the names together: a file renamed in the product would compile on both sides and
// turn the uninstaller into one that leaves it behind.
// ---------------------------------------------------------------------------------------------

/// The value of `#define <name> "<value>"` in the script.
fn define_value(script: &str, name: &str) -> Option<String> {
    script.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("#define ")?.trim_start();
        let rest = rest.strip_prefix(name)?;
        if !rest.starts_with(char::is_whitespace) {
            return None;
        }
        let value = rest.trim();
        Some(value.strip_prefix('"')?.strip_suffix('"')?.to_string())
    })
}

/// A string written as Pascal character codes, `#$043E#$0431…` — how this ASCII script carries a
/// word in another alphabet.
fn pascal_codes(text: &str) -> Option<String> {
    text.split("#$")
        .skip(1)
        .map(|code| u32::from_str_radix(code, 16).ok().and_then(char::from_u32))
        .collect()
}

/// The stems of the name of a saved appeal, `<stem>-<date>.txt`, as `report::file_name` in
/// `src/letters.rs` writes them.
fn appeal_stems() -> Vec<String> {
    let source = read("src/letters.rs");
    let body: Vec<&str> = source
        .lines()
        .map(str::trim)
        .skip_while(|line| !line.starts_with("pub fn file_name(today: super::Date) -> String {"))
        .take_while(|line| *line != "}")
        .collect();
    assert!(
        body.contains(&"format!(\"{stem}-{today}.txt\")"),
        "src/letters.rs no longer names a saved appeal `<stem>-<date>.txt`: {body:?}"
    );
    let mut stems: Vec<String> = body
        .iter()
        .filter_map(|line| line.split_once("=> \""))
        .filter_map(|(_, rest)| rest.split_once('"'))
        .map(|(stem, _)| stem.to_string())
        .collect();
    stems.sort();
    stems
}

/// The lines of the routine that starts at the first line equal to `head`, up to its `end;`.
fn routine<'a>(lines: &[&'a str], head: impl Fn(&str) -> bool) -> Vec<&'a str> {
    lines
        .iter()
        .copied()
        .skip_while(|line| !head(line))
        .take_while(|line| *line != "end;")
        .collect()
}

/// Every reason why the uninstaller would leave the journal, the settings on Yes, or the folder
/// behind; empty when it leaves none of them.
fn reasons_it_leaves_a_tail(
    script: &str,
    product: &[(&str, &str)],
    stems: &[String],
) -> Vec<String> {
    let mut reasons = Vec::new();
    for (name, value) in product {
        match define_value(script, name) {
            Some(found) if found == *value => {}
            found => reasons.push(format!(
                "`#define {name}` is {found:?}; the product writes \"{value}\""
            )),
        }
    }
    let mut ours: Vec<String> = [
        define_value(script, "AppealStem"),
        define_value(script, "AppealStemRu").and_then(|codes| pascal_codes(&codes)),
    ]
    .into_iter()
    .flatten()
    .collect();
    ours.sort();
    if ours != stems {
        reasons.push(format!(
            "the stems of a saved appeal are {ours:?} here and {stems:?} in the product"
        ));
    }

    let lines: Vec<&str> = script.lines().map(str::trim).collect();
    let journal_head = lines.iter().position(|line| {
        *line == "procedure CurUninstallJournal(CurUninstallStep: TUninstallStep);"
    });
    match journal_head {
        Some(at) if at > 0 && lines[at - 1] == "<event('CurUninstallStepChanged')>" => {
            let head = |line: &str| line.starts_with("procedure CurUninstallJournal(");
            let body = routine(&lines[at..], head);
            for wanted in [
                "if CurUninstallStep <> usPostUninstall then",
                "Folder := ExpandConstant('{userappdata}\\{#DataFolder}');",
                "DeleteDataFile(Folder + '\\{#JournalName}', 'the journal');",
                "DeleteDataFiles(Folder, '{#JournalName}.*.tmp', 'the temporaries of the journal');",
            ] {
                if !body.contains(&wanted) {
                    reasons.push(format!("the journal handler lacks `{wanted}`"));
                }
            }
            for forbidden in ["UninstallSilent", "MsgBox", "IDYES", "IDNO"] {
                if body.iter().any(|line| line.contains(forbidden)) {
                    reasons.push(format!(
                        "the journal handler depends on `{forbidden}`: the journal goes always (177.1)"
                    ));
                }
            }
        }
        _ => reasons.push(
            "no `CurUninstallJournal` marked <event('CurUninstallStepChanged')>: Inno calls such a \
             handler before the main one, so the folder is emptied before it is removed"
                .to_string(),
        ),
    }

    let main = routine(&lines, |line| {
        line == "procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);"
    });
    let at = |wanted: &str| main.iter().position(|line| *line == wanted);
    for wanted in [
        "Folder := ExpandConstant('{userappdata}\\{#DataFolder}');",
        "ConfigFile := Folder + '\\{#ConfigName}';",
        "BadFile := ConfigFile + '{#BadSuffix}';",
        "if not (FileExists(ConfigFile) or FileExists(BadFile)) then",
        "else if UninstallSilent() then",
        "mbConfirmation, MB_YESNO, IDNO) = IDYES then",
    ] {
        if at(wanted).is_none() {
            reasons.push(format!("the main handler lacks `{wanted}`"));
        }
    }
    match at("mbConfirmation, MB_YESNO, IDNO) = IDYES then") {
        Some(yes) => {
            let branch: Vec<&str> = main[yes + 1..]
                .iter()
                .copied()
                .take_while(|line| *line != "end")
                .collect();
            for wanted in [
                "DeleteDataFile(ConfigFile, 'the settings');",
                "DeleteDataFile(BadFile, 'the kept copy of unreadable settings');",
                "DeleteDataFiles(Folder, '{#AppealStem}-*.txt', 'the saved appeals');",
                "DeleteDataFiles(Folder, {#AppealStemRu} + '-*.txt', 'the saved appeals in Russian');",
            ] {
                if !branch.contains(&wanted) {
                    reasons.push(format!("the Yes of the question lacks `{wanted}`"));
                }
            }
            let after = yes + 1 + branch.len();
            if !main[after..]
                .iter()
                .any(|line| line.contains("RemoveDir(Folder)"))
            {
                reasons.push("the folder is not removed after the question".to_string());
            }
        }
        None => reasons.push("no question with No as its default answer".to_string()),
    }
    reasons
}

/// The names of the product, `(define of the script, value in the product)`.
fn product_names() -> [(&'static str, &'static str); 4] {
    [
        ("DataFolder", lang_switcher::CONFIG_DIR_NAME),
        ("ConfigName", lang_switcher::settings::CONFIG_FILE_NAME),
        ("BadSuffix", lang_switcher::settings::QUARANTINE_SUFFIX),
        ("JournalName", lang_switcher::diag::LOG_FILE_NAME),
    ]
}

#[test]
fn the_uninstaller_deletes_the_journal_always_and_the_settings_on_yes() {
    let script = read("installer/LangSwitcher.iss");
    let reasons = reasons_it_leaves_a_tail(&script, &product_names(), &appeal_stems());
    assert!(
        reasons.is_empty(),
        "installer/LangSwitcher.iss would leave a tail in %APPDATA%\\Lang_Switcher (177.1, 177.20.1):\n  {}",
        reasons.join("\n  ")
    );
}

#[test]
fn a_kept_journal_a_kept_copy_and_a_renamed_file_are_all_caught() {
    let script = read("installer/LangSwitcher.iss");
    let names = product_names();
    let stems = appeal_stems();
    assert_eq!(
        stems,
        ["report", "обращение"],
        "the two stems of a saved appeal are read from the product"
    );
    let check = |text: &str| reasons_it_leaves_a_tail(text, &names, &stems);
    assert!(
        check(&script).is_empty(),
        "the real script must pass before its copies can fail meaningfully"
    );
    let mutants = [
        (
            "DeleteDataFile(Folder + '\\{#JournalName}', 'the journal');",
            "Log('the journal is kept');",
            "a journal that is not deleted",
        ),
        (
            "DeleteDataFile(BadFile, 'the kept copy of unreadable settings');",
            "Log('the copy is kept');",
            "a kept copy of the settings left on Yes",
        ),
        (
            "#define JournalName  \"diag.log\"",
            "#define JournalName  \"journal.log\"",
            "a journal of another name",
        ),
        (
            "RemoveDir(Folder)",
            "DirExists(Folder)",
            "a folder that is never removed",
        ),
    ];
    for (from, to, what) in mutants {
        let mutant = script.replace(from, to);
        assert_ne!(mutant, script, "the script has `{from}`");
        assert!(!check(&mutant).is_empty(), "{what} must be caught");
    }
    assert_eq!(
        pascal_codes("#$043E#$0431#$0440#$0430#$0449#$0435#$043D#$0438#$0435").as_deref(),
        Some("обращение"),
        "Pascal character codes are read as the word they spell"
    );
}
