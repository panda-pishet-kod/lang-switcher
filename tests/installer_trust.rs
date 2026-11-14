//! **The helper of the installer and the module path it inherits** — `installer\local-trust.ps1`,
//! stage E116, task T-116-5 (BACKLOG Q-208-21, decision 208.45).
//!
//! Setup runs the helper in Windows PowerShell 5.1, and the helper is a child of whatever started
//! Setup. Started from a PowerShell 7 terminal — directly or through `cmd.exe` — it inherits
//! `PSModulePath` with the module folders of PowerShell 7 in front; 5.1 then finds
//! `Microsoft.PowerShell.Security` of PowerShell 7 first, cannot load it, step `check` fails with
//! exit 11 and the **base** image is installed in silence: no `uiAccess`, and no switching in the
//! windows of programs run as administrator (208.45: the owner's Photoshop and the window of
//! Claude). The helper therefore sets the path back to the folders of Windows PowerShell itself
//! before it asks for any cmdlet of a module.
//!
//! A test of the text of the script, because the condition is an environment of another process:
//! the stand measures the run (T-116-9), this keeps the line where it has to stand.

use std::fs;
use std::path::PathBuf;

fn read(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .replace("\r\n", "\n")
}

/// The lines of code of a PowerShell script, numbered from 1: the block comment `<# … #>` and every
/// line comment left out.
fn code(script: &str) -> Vec<(usize, &str)> {
    let mut inside = false;
    let mut lines = Vec::new();

    for (index, line) in script.lines().enumerate() {
        let trimmed = line.trim();

        if inside {
            if trimmed.ends_with("#>") {
                inside = false;
            }
            continue;
        }

        if trimmed.starts_with("<#") {
            inside = !trimmed.ends_with("#>");
            continue;
        }

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        lines.push((index + 1, line));
    }

    lines
}

/// The number of the first line of code that contains `needle`.
fn first(code: &[(usize, &str)], needle: &str) -> usize {
    code.iter()
        .find(|(_, line)| line.contains(needle))
        .map(|(number, _)| *number)
        .unwrap_or_else(|| panic!("`{needle}` is in the code of the helper"))
}

/// ⭐ **Stage E116, task T-116-5 — the path is set, once, to the folders of Windows PowerShell, and
/// before anything asks for a module.** One assignment of `$env:PSModulePath`; it names
/// `$PSHOME\Modules` and does not carry the inherited path on; it stands above the first step, the
/// first signature cmdlet, the first certificate cmdlet and the first use of the `Cert:` drive —
/// every one of which loads `Microsoft.PowerShell.Security` or `PKI`.
#[test]
fn the_helper_sets_the_module_path_of_windows_powershell_before_any_module_is_asked_for() {
    let helper = read("installer/local-trust.ps1");
    let code = code(&helper);

    let resets: Vec<&(usize, &str)> = code
        .iter()
        .filter(|(_, line)| line.trim_start().starts_with("$env:PSModulePath ="))
        .collect();

    assert_eq!(
        resets.len(),
        1,
        "T-116-5: installer\\local-trust.ps1 sets $env:PSModulePath once, before the steps \
         (Q-208-21: started from PowerShell 7, the helper failed at step check, exit 11, and the \
         base image was installed); found {resets:?}"
    );

    let (at, line) = *resets[0];

    assert!(
        line.contains("$PSHOME") && line.contains("'Modules'"),
        "the path names the modules of Windows PowerShell itself ($PSHOME\\Modules): line {at} `{}`",
        line.trim()
    );
    assert!(
        !line.contains("$env:PSModulePath +") && !line.contains("+ $env:PSModulePath"),
        "the inherited path is not carried on: line {at} `{}`",
        line.trim()
    );

    for later in [
        "Enter-Step $step",
        "Get-AuthenticodeSignature",
        "Set-AuthenticodeSignature",
        "New-SelfSignedCertificate",
        "Cert:\\",
    ] {
        let use_at = first(&code, later);

        assert!(
            at < use_at,
            "the path is set (line {at}) before `{later}` (line {use_at}) asks for its module"
        );
    }
}
