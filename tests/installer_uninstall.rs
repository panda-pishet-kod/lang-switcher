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
