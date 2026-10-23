//! The installer and the updater agree — task T-104-6 (TZ E104 §0 п. 13–18).
//!
//! The installer (`installer\LangSwitcher.iss`, Pascal) and the updater (`src\update.rs`, Rust)
//! name the same things in two languages: the task of the scheduler, the folder, the kept copy of
//! the installer, the mutex of Setup, the switch of phase two. Nothing but this test ties the two
//! together — a name changed on one side compiles on both and turns the update into one that
//! never finishes. And the task the installer registers is the task §0 п. 14 describes: the
//! system account, once a day and at sign-in, only in idle, a missed start at the first chance,
//! thirty minutes, one instance.

use std::fs;
use std::path::Path;

use lang_switcher::update;

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The value of `#define <name> "<value>"` in the script.
fn define_value(script: &str, name: &str) -> Option<String> {
    script.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("#define")?.trim_start();
        let rest = rest.strip_prefix(name)?;
        if !rest.starts_with([' ', '\t']) {
            return None;
        }
        let value = rest.trim();
        Some(value.strip_prefix('"')?.strip_suffix('"')?.to_owned())
    })
}

/// Everything §0 п. 13–18 asks of the script, as reasons it fails — empty when it holds.
fn reasons_the_updater_is_not_installed(script: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    let mut need = |what: &str, why: &str| {
        if !script.contains(what) {
            reasons.push(format!("{why}: «{what}» is missing"));
        }
    };

    // The file and the folder.
    need(
        "Source: \"{#SourceUpdater}\"; DestDir: \"{app}\"; DestName: \"{#UpdaterExeName}\"",
        "the updater is not copied beside the program",
    );
    need(
        "Name: \"{commonappdata}\\{#UpdateDir}\"",
        "the folder of the updater is not made",
    );
    need(
        "SetupMutex={#SetupMutexName}",
        "Setup holds no mutex for phase two to wait on",
    );

    // The rights of the folder — SYSTEM and Administrators, nobody else (премиса П10).
    need(
        "/setowner *S-1-5-32-544 /T",
        "the owner of the folder is not taken",
    );
    need(
        "/inheritance:r /grant:r *S-1-5-18:(OI)(CI)F *S-1-5-32-544:(OI)(CI)F",
        "the inheritance of %ProgramData% is not cut",
    );
    need(
        "ExpandConstant('{sys}\\icacls.exe')",
        "icacls is not the one of System32",
    );

    // The copy of the installer — what a rollback installs.
    need(
        "ExpandConstant('{srcexe}')",
        "the installer does not keep a copy of itself",
    );
    need(
        "'\\{#InstalledSetup}'",
        "the copy is not under the name the updater reads",
    );

    // The task — §0 п. 14.
    need(
        "<UserId>S-1-5-18</UserId>",
        "the task does not run under the system account",
    );
    need(
        "<DaysInterval>1</DaysInterval>",
        "the task does not run once a day",
    );
    need("<LogonTrigger>", "the task does not run at sign-in");
    need("<Delay>PT5M</Delay>", "the sign-in run is not delayed");
    need(
        "<RunOnlyIfIdle>true</RunOnlyIfIdle>",
        "the task does not wait for idle",
    );
    need("<Duration>PT10M</Duration>", "the idle is not ten minutes");
    need(
        "<StartWhenAvailable>true</StartWhenAvailable>",
        "a missed start is not made up",
    );
    need(
        "<ExecutionTimeLimit>PT30M</ExecutionTimeLimit>",
        "the run is not limited to 30 minutes",
    );
    need(
        "<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>",
        "a second instance is not refused",
    );
    need(
        "<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>",
        "a laptop on battery never updates",
    );
    need(
        "ExpandConstant('{app}\\{#UpdaterExeName}')",
        "the task does not start the updater of {app}",
    );
    need(
        "ExpandConstant('{sys}\\schtasks.exe')",
        "schtasks is not the one of System32",
    );
    need(
        "'/Create /TN \"' + LangSwUpdateTask + '\" /XML \"'",
        "the task is not registered from its XML",
    );

    // Phase two — at the end of a Setup started with /UPDATE, whatever came of it.
    need(
        "<event('DeinitializeSetup')>",
        "phase two is not started at the end of Setup",
    );
    need(
        "if not HasSwitch('/UPDATE') then",
        "phase two is not limited to a Setup of the updater",
    );
    need(
        "Exec(Updater, '{#PhaseTwo}', '', SW_HIDE, ewNoWait, Code)",
        "phase two is not started without waiting",
    );

    // The uninstaller — no remains (SPEC §13 п. 7).
    need(
        "'/Delete /TN \"' + LangSwUpdateTask + '\" /F'",
        "the uninstaller does not delete the task",
    );
    need(
        "Type: filesandordirs; Name: \"{commonappdata}\\{#UpdateDir}\"",
        "the uninstaller does not remove the folder of the updater",
    );

    // ⛔ Round 1 of T-104-9 (Test_VM): `schtasks` refused the XML with «(1,40) unable to switch the
    // encoding» — a declaration of UTF-8 in a file it reads into a wide string — and the task was
    // not registered at all. The XML carries no declaration.
    if script.contains("<?xml version=\"1.0\" encoding=") {
        reasons.push(
            "the task XML declares an encoding: schtasks refuses it («unable to switch the encoding»)"
                .to_owned(),
        );
    }

    reasons
}

/// **The two languages name the same things** — the defines of the script against the constants
/// of `src\update.rs`.
#[test]
fn the_installer_and_the_updater_name_the_same_things() {
    let script = read("installer/LangSwitcher.iss");

    let pairs = [
        ("UpdaterExeName", update::EXE_NAME.to_owned()),
        ("UpdateTask", update::TASK_NAME.to_owned()),
        (
            "UpdateDir",
            format!("{}\\{}", update::DATA_DIR, update::UPDATE_DIR),
        ),
        ("InstalledSetup", update::INSTALLED_SETUP.to_owned()),
        ("SetupMutexName", update::SETUP_MUTEX.to_owned()),
        ("PhaseTwo", update::PHASE_TWO_SWITCH.to_owned()),
    ];

    for (name, expected) in pairs {
        assert_eq!(
            define_value(&script, name).as_deref(),
            Some(expected.as_str()),
            "#define {name} of the script must be what src\\update.rs says"
        );
    }

    assert!(
        update::SETUP_SWITCHES.contains("/UPDATE"),
        "phase one must start Setup with the switch the script waits for"
    );

    // Решение 189.33: the helper of the updater finds the program's window by the class the
    // program registers and the uninstaller posts to — three places, one name.
    assert_eq!(
        define_value(&script, "ProductWindowClass").as_deref(),
        Some(update::PROGRAM_WINDOW_CLASS)
    );
    assert!(
        read("src/app.rs").contains(&format!(
            "const WINDOW_CLASS_NAME: PCWSTR = w!(\"{}\");",
            update::PROGRAM_WINDOW_CLASS
        )),
        "src\\app.rs must register the class the updater looks for"
    );
}

/// **The script installs the updater the way §0 п. 13–18 describes** — the file, the folder and
/// its rights, the copy, the task, phase two, the uninstaller.
#[test]
fn the_installer_installs_and_removes_the_updater() {
    let reasons = reasons_the_updater_is_not_installed(&read("installer/LangSwitcher.iss"));

    assert!(reasons.is_empty(), "{}", reasons.join("\n"));
}

/// **The checker can fail** — the negative control: the real script with one condition of the task
/// taken out, and with the folder's grant widened to every user, must both be refused, each with
/// its own reason.
#[test]
fn a_task_without_its_idle_and_a_folder_for_everybody_are_told_apart() {
    let script = read("installer/LangSwitcher.iss");

    let no_idle = script.replace(
        "<RunOnlyIfIdle>true</RunOnlyIfIdle>",
        "<RunOnlyIfIdle>false</RunOnlyIfIdle>",
    );
    assert_ne!(no_idle, script, "the bait must change the script");
    let reasons = reasons_the_updater_is_not_installed(&no_idle);
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(reasons[0].contains("idle"), "{reasons:?}");

    let open = script.replace(
        "/inheritance:r /grant:r *S-1-5-18:(OI)(CI)F *S-1-5-32-544:(OI)(CI)F",
        "/grant *S-1-5-32-545:(OI)(CI)M",
    );
    assert_ne!(open, script, "the bait must change the script");
    let reasons = reasons_the_updater_is_not_installed(&open);
    assert_eq!(reasons.len(), 1, "{reasons:?}");
    assert!(reasons[0].contains("inheritance"), "{reasons:?}");
}

/// **The updater carries no `uiAccess` and no manifest of the program** — §0 п. 13: its own
/// manifest asks for `asInvoker` and `uiAccess="false"`, and `build.rs` links `app.rc` (the
/// manifest of the program) into the program and the bench only.
#[test]
fn the_updater_has_its_own_manifest_without_ui_access() {
    let manifest = read("updater.manifest");

    assert!(manifest.contains("level=\"asInvoker\""));
    assert!(manifest.contains("uiAccess=\"false\""));
    assert!(!manifest.contains("uiAccess=\"true\""));

    let build = read("build.rs");

    assert!(
        build.contains("compile_for(\"app.rc\", [\"LangSwitcher\", \"langsw-e2e\"]"),
        "app.rc must be linked into the program and the bench only"
    );
    assert!(
        build.contains("compile_for(\"updater.rc\", [\"LangSwitcherUpdate\"]"),
        "updater.rc must be linked into the updater"
    );
    assert!(
        !build.contains("embed_resource::compile(\"app.rc\""),
        "a compile for every binary would give the updater the manifest of the program"
    );
}
