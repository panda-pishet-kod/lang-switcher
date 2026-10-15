//! The name is «Lang Switcher», without the platform — decision 173.1 of the owner (stage E101,
//! question 177). The program has carried the short name all along; from e90 to e100 the installer,
//! the local certificate, the description of the signature, `README.md` and `SPEC.md` added «for
//! Windows» to it (decision 153.4, cancelled by 173.1). Three things are held here:
//!
//! 1. the old name is gone from `installer\`, `tools\`, `README.md` and `SPEC.md`, except the
//!    constants that keep it to clean up after e90…e100 — their names end in [`MARK`], so the mark
//!    stands on the very line of the old name, and a comment or a string that slips the old name
//!    back in has no mark;
//! 2. the installer is one name: every directive that named the program with the platform names it
//!    with `AppName`, the Start menu folder is `AppName`, and `[InstallDelete]` removes the folder of
//!    the old name. Inno processes `[InstallDelete]` before `[Files]` and `[Icons]` (the help of Inno
//!    Setup 6.7, «Installation Order»), so an update from any version comes to one folder with two
//!    shortcuts;
//! 3. the helper of the installer cleans up a local certificate of either name: an update over
//!    0.91.0…0.100.0 finds the certificate of the old name even when its `.cer` is gone.
//!
//! What stays: «for Windows 10 and 11», «утилита для Windows» — facts of the requirements, not the
//! name — and turns of speech such as «for Windows PowerShell 5.1».

use std::fs;
use std::path::PathBuf;

/// The name with the platform, in the two languages it was written in.
const OLD_NAMES: [&str; 2] = ["Lang Switcher for Windows", "Lang Switcher для Windows"];

/// The ending of the name of every constant that keeps an old name for the cleanup.
const MARK: &str = "UntilE100";

/// The part of the tree the old name has to leave.
const SWEPT: [&str; 4] = ["installer", "tools", "README.md", "SPEC.md"];

/// Not of the tree: the machine's own values (stage E89, untracked) and the output of the compiler
/// of the installer.
const NOT_OF_THE_TREE: [&str; 2] = ["tools/local.ps1", "installer/Output"];

/// The constants of the old name, by file and by the beginning of their line — the only lines the
/// old name may stand on.
const KEPT: [(&str, &str); 3] = [
    ("installer/LangSwitcher.iss", "#define GroupNameUntilE100 "),
    ("installer/local-trust.ps1", "$LocalSubjectUntilE100 = "),
    ("tools/accept-clean-machine.ps1", "$NameUntilE100 = "),
];

/// A line of the tree that carries an old name: the file, its number, its text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Found {
    file: String,
    line: usize,
    text: String,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn files_of(relative: &str, into: &mut Vec<String>) {
    if NOT_OF_THE_TREE.contains(&relative) {
        return;
    }
    let path = root().join(relative);
    if path.is_dir() {
        let mut names: Vec<String> = fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
            .map(|entry| {
                entry
                    .expect("an entry of the folder must be readable")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        for name in names {
            files_of(&format!("{relative}/{name}"), into);
        }
    } else {
        into.push(relative.to_string());
    }
}

/// Every line of `text` that carries an old name.
fn old_names_in(file: &str, text: &str) -> Vec<Found> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| OLD_NAMES.iter().any(|name| line.contains(name)))
        .map(|(index, line)| Found {
            file: file.to_string(),
            line: index + 1,
            text: line.trim().to_string(),
        })
        .collect()
}

/// Every line of the swept part of the tree that carries an old name, and the number of text files
/// read — the pictures of the installer are not text and are passed over.
fn sweep() -> (Vec<Found>, usize) {
    let mut files = Vec::new();
    for part in SWEPT {
        files_of(part, &mut files);
    }
    let mut found = Vec::new();
    let mut read = 0;
    for file in &files {
        let Ok(text) = fs::read_to_string(root().join(file)) else {
            continue;
        };
        read += 1;
        found.extend(old_names_in(file, &text));
    }
    (found, read)
}

/// The lines that keep an old name without being one of the constants of [`KEPT`].
fn unmarked(found: &[Found]) -> Vec<&Found> {
    found
        .iter()
        .filter(|found| {
            !KEPT
                .iter()
                .any(|(file, start)| found.file == *file && found.text.starts_with(start))
                || !found.text.contains(MARK)
        })
        .collect()
}

#[test]
fn the_old_name_is_gone_except_the_constants_that_clean_up_after_it() {
    let (found, read) = sweep();
    for found in &found {
        println!("{}:{}: {}", found.file, found.line, found.text);
    }
    println!("text files read: {read}");

    let left: Vec<String> = unmarked(&found)
        .iter()
        .map(|found| format!("{}:{}: {}", found.file, found.line, found.text))
        .collect();
    assert!(
        left.is_empty(),
        "the name with the platform is still here (decision 173.1); only a constant whose name \
         ends in {MARK} may keep it, for the cleanup after e90…e100:\n  {}",
        left.join("\n  ")
    );
    for (file, start) in KEPT {
        let lines = found
            .iter()
            .filter(|found| found.file == file && found.text.starts_with(start))
            .count();
        assert_eq!(
            lines, 1,
            "{file}: the constant `{start}…` of the old name, which the cleanup needs, must stand \
             there exactly once"
        );
    }
}

#[test]
fn the_sweep_tells_a_constant_from_a_name_that_slipped_back() {
    let constant = r#"#define GroupNameUntilE100 "Lang Switcher for Windows""#;
    let slipped = r#"#define AppTitle       "Lang Switcher for Windows""#;
    let comment = "; since e90 it is \"Lang Switcher for Windows\" -- GroupNameUntilE100";
    let russian = "**Lang Switcher для Windows** исправляет текст";
    let fact = "for Windows 10 and 11, free";
    let text = [constant, slipped, comment, russian, fact].join("\r\n");

    let found = old_names_in("installer/LangSwitcher.iss", &text);
    assert_eq!(
        found.len(),
        4,
        "both spellings of the old name are found, the fact is not: {found:?}"
    );
    let left: Vec<usize> = unmarked(&found).iter().map(|found| found.line).collect();
    assert_eq!(
        left,
        [2, 3, 4],
        "the constant passes; the name slipped back, a comment that only mentions the mark and the \
         Russian spelling do not"
    );
    let elsewhere = old_names_in("installer/local-trust.ps1", constant);
    assert_eq!(
        unmarked(&elsewhere).len(),
        1,
        "a constant of one file does not pass in another"
    );
}

/// Every reason why `script` names the program with the platform, or leaves the Start menu folder
/// of the old name behind; empty when it does neither.
fn reasons_the_installer_is_not_one_name(script: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    let lines: Vec<&str> = script.lines().map(str::trim).collect();
    let has = |wanted: &str| lines.contains(&wanted);

    for define in [
        r#"#define AppName        "Lang Switcher""#,
        r#"#define GroupNameUntilE100 "Lang Switcher for Windows""#,
    ] {
        if !has(define) {
            reasons.push(format!("no line `{define}`"));
        }
    }
    if script.contains("AppTitle") {
        reasons.push("`AppTitle` is still there: one name, AppName (173.1)".to_string());
    }

    let section = |name: &str| -> Vec<&str> {
        lines
            .iter()
            .copied()
            .skip_while(|line| *line != name)
            .skip(1)
            .take_while(|line| !line.starts_with('['))
            .collect()
    };
    let setup = section("[Setup]");
    for directive in [
        "AppName={#AppName}",
        "AppVerName={#AppName} {#AppVersion}",
        "VersionInfoProductName={#AppName}",
        "VersionInfoDescription={#AppName} Setup",
        "DefaultGroupName={#AppName}",
        "UsePreviousGroup=no",
        "UninstallDisplayName={#AppName}",
    ] {
        if !setup.contains(&directive) {
            reasons.push(format!("[Setup] lacks `{directive}`"));
        }
    }
    let install_delete = section("[InstallDelete]");
    for entry in [
        r#"Type: filesandordirs; Name: "{autoprograms}\{#GroupNameUntilE100}""#,
        r#"Type: filesandordirs; Name: "{autoprograms}\{#AppName}""#,
    ] {
        if !install_delete.contains(&entry) {
            reasons.push(format!("[InstallDelete] lacks `{entry}`"));
        }
    }
    let icons = section("[Icons]");
    if !icons
        .iter()
        .any(|line| line.starts_with(r#"Name: "{group}\{#AppName}"; Filename: "{app}\"#))
    {
        reasons.push("[Icons] does not put the shortcut of AppName into the group".to_string());
    }
    if !script.contains("FmtMessage(SetupMessage(msgUninstallAppRunningError), ['{#AppName}'])") {
        reasons.push(
            "the uninstaller's request to exit the program does not name it by AppName".to_string(),
        );
    }
    reasons
}

#[test]
fn the_installer_names_the_program_once_and_removes_the_start_menu_folder_of_the_old_name() {
    let script = read("installer/LangSwitcher.iss");
    let reasons = reasons_the_installer_is_not_one_name(&script);
    assert!(
        reasons.is_empty(),
        "installer/LangSwitcher.iss is not one name (173.1) or keeps the old Start menu folder:\n  {}",
        reasons.join("\n  ")
    );
}

#[test]
fn a_title_with_the_platform_and_a_forgotten_folder_are_both_caught() {
    let script = read("installer/LangSwitcher.iss");
    assert!(
        reasons_the_installer_is_not_one_name(&script).is_empty(),
        "the real script must pass before its copies can fail meaningfully"
    );
    let with_title = script.replace(
        "AppName={#AppName}\r\n",
        "AppName={#AppTitle}\r\n#define AppTitle \"Lang Switcher for Windows\"\r\n",
    );
    assert_ne!(
        with_title, script,
        "the script has the line `AppName={{#AppName}}`"
    );
    assert!(
        !reasons_the_installer_is_not_one_name(&with_title).is_empty(),
        "a title with the platform must be caught"
    );
    let without_cleanup = script.replace(
        "Type: filesandordirs; Name: \"{autoprograms}\\{#GroupNameUntilE100}\"\r\n",
        "",
    );
    assert_ne!(without_cleanup, script, "the script has the cleanup line");
    assert!(
        !reasons_the_installer_is_not_one_name(&without_cleanup).is_empty(),
        "an update that leaves the folder of the old name must be caught"
    );
}

/// Every reason why `helper` would leave a local certificate of the old name behind, or make one
/// with it; empty when it does neither.
fn reasons_the_cleanup_misses_a_name(helper: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    let lines: Vec<&str> = helper.lines().map(str::trim).collect();
    for wanted in [
        "$LocalSubject = 'CN=Lang Switcher (local)'",
        "$LocalSubjectUntilE100 = 'CN=Lang Switcher for Windows (local)'",
        "$LocalSubjects = @($LocalSubject, $LocalSubjectUntilE100)",
    ] {
        if !lines.contains(&wanted) {
            reasons.push(format!("no line `{wanted}`"));
        }
    }
    let finder: Vec<&str> = lines
        .iter()
        .skip_while(|line| !line.starts_with("function Find-InStore("))
        .take_while(|line| **line != "}")
        .copied()
        .collect();
    if !finder
        .iter()
        .any(|line| line.contains("$LocalSubjects -contains $c.Subject"))
    {
        reasons
            .push("Find-InStore does not match a certificate by either local subject".to_string());
    }
    let create = lines
        .iter()
        .find(|line| line.contains("New-SelfSignedCertificate -Type Custom"));
    match create {
        Some(line)
            if line.contains("-Subject $LocalSubject ")
                && line.contains("-FriendlyName 'Lang Switcher (local)'") => {}
        _ => reasons
            .push("the certificate is not made with the new subject and friendly name".to_string()),
    }
    reasons
}

#[test]
fn the_helper_cleans_up_a_local_certificate_of_either_name() {
    let helper = read("installer/local-trust.ps1");
    let reasons = reasons_the_cleanup_misses_a_name(&helper);
    assert!(
        reasons.is_empty(),
        "installer/local-trust.ps1 would leave a certificate of the old name behind:\n  {}",
        reasons.join("\n  ")
    );
    let one_name = helper.replace(
        "$LocalSubjects = @($LocalSubject, $LocalSubjectUntilE100)",
        "$LocalSubjects = @($LocalSubject)",
    );
    assert_ne!(
        one_name, helper,
        "the helper names both subjects in one list"
    );
    assert!(
        !reasons_the_cleanup_misses_a_name(&one_name).is_empty(),
        "a cleanup by the new name alone must be caught"
    );
}
