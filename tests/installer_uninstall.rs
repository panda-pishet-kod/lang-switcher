//! Uninstalling a running program is refused, never half done — decisions 155.23 and 155.24.
//!
//! Inno's uninstaller does not close programs: the Restart Manager of `CloseApplications` serves
//! Setup only. A running `LangSwitcher.exe` keeps its file locked, the uninstaller skips the file,
//! and the program goes on living in the tray of a computer it was removed from — seen on a clean
//! machine at the acceptance of e91 (155.23). The repair asks the person to exit the program:
//! before anything is removed, an `InitializeUninstall` handler looks for the mutex the product
//! creates on every start (FR-82) and, while it exists, shows Inno's own `UninstallAppRunningError`
//! with OK and Cancel. Cancel — and the default answer of a suppressed message box — aborts the
//! uninstall.
//!
//! The product and the installer name that mutex separately, in two languages, and nothing but
//! this test ties the two names together: a mutex renamed in `src\app.rs` would compile on both
//! sides and turn the check into one that never fires.

use std::fs;
use std::path::Path;

/// The attribute that marks the handler; the script may hold several `InitializeUninstall`
/// implementations (Inno calls them all and needs every one to return True).
const HANDLER: &str = "<event('InitializeUninstall')>";

/// What the handler must contain, in this order: the loop over the product's mutex, Inno's own
/// message, Cancel as the answer of a suppressed box, and the refusal.
const STEPS: [&str; 4] = [
    "while CheckForMutexes('{#ProductMutex}') do",
    "SetupMessage(msgUninstallAppRunningError)",
    "MB_OKCANCEL, IDCANCEL",
    "Result := False;",
];

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The name the product gives its single-instance mutex: the literal of `MUTEX_NAME`.
fn product_mutex() -> String {
    let source = read("src/app.rs");
    let prefix = "const MUTEX_NAME: PCWSTR = w!(r\"";
    let line = source
        .lines()
        .map(str::trim_start)
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("src/app.rs no longer declares `{prefix}…\")`"));
    let rest = &line[prefix.len()..];
    rest[..rest.find('"').expect("the literal of MUTEX_NAME is closed")].to_string()
}

/// Every reason why `script` does not refuse to uninstall while the program named by `mutex`
/// runs; empty when it does.
fn reasons_it_does_not_refuse(script: &str, mutex: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    let define = format!("#define ProductMutex \"{mutex}\"");
    if !script.lines().any(|line| line.trim() == define) {
        reasons.push(format!(
            "no line `{define}`: the script names another mutex or none"
        ));
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
fn the_uninstaller_refuses_while_the_program_runs_and_looks_for_its_very_mutex() {
    let script = read("installer/LangSwitcher.iss");
    let reasons = reasons_it_does_not_refuse(&script, &product_mutex());
    assert!(
        reasons.is_empty(),
        "installer/LangSwitcher.iss would uninstall a running program (155.23):\n  {}",
        reasons.join("\n  ")
    );
}

#[test]
fn a_missing_check_and_a_misspelt_mutex_are_both_told_apart_from_the_real_script() {
    let script = read("installer/LangSwitcher.iss");
    let mutex = product_mutex();
    assert!(
        reasons_it_does_not_refuse(&script, &mutex).is_empty(),
        "the real script must pass before its copies can fail meaningfully"
    );
    let misspelt = format!("{mutex}x");
    assert_eq!(
        reasons_it_does_not_refuse(&script, &misspelt).len(),
        1,
        "a mutex name one letter off must be caught"
    );
    let without_handler = script.replace(HANDLER, "{ no handler }");
    assert!(
        !reasons_it_does_not_refuse(&without_handler, &mutex).is_empty(),
        "a script without the handler must be caught"
    );
    let without_refusal = script.replace(STEPS[3], "Result := True;");
    assert!(
        !reasons_it_does_not_refuse(&without_refusal, &mutex).is_empty(),
        "a handler that never refuses must be caught"
    );
    let with_app_mutex = script.replace("[Setup]\r\n", "[Setup]\r\nAppMutex=x\r\n");
    assert_ne!(
        with_app_mutex, script,
        "the script has a [Setup] header line"
    );
    assert!(
        !reasons_it_does_not_refuse(&with_app_mutex, &mutex).is_empty(),
        "an AppMutex directive in [Setup] must be caught"
    );
}
