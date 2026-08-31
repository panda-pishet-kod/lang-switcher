//! Microsoft Word — position 2, and the five rakes of §2 of `TOOLCHAIN.md`.
//!
//! Position 2 is the hardest of the matrix and the only one that checks a word processor's
//! autocorrect, which is the scenario FR-42 exists for. Every rake below cost task Н-12 a
//! separate round of debugging; each is handled here and each is named where it is handled.
//!
//! | Rake | Where it is handled |
//! |---|---|
//! | 1. Dialogs come in two classes, `#32770` **and** `bosa_sdm_msword` | [`DIALOG_CLASSES`] |
//! | 2. A dialog need not be a top-level window — it appears as a child of `OpusApp` | [`dialogs`] searches both |
//! | 3. `SetForegroundWindow` is ignored, `SetFocus` on the document throws | [`crate::shell::activate`], via `word-activate.ps1` — used for every application, not only Word |
//! | 4. `WM_CLOSE` is ignored; the way out is COM `Quit(0)` | [`quit`], via `word-quit.ps1` |
//! | 5. `Stop-Process` poisons the next launch | never called — see [`quit`] |

use std::path::PathBuf;
use std::process::{Command, Output};

use windows::Win32::UI::Accessibility::UIA_DocumentControlTypeId;

use crate::uia::{Automation, Element};
use crate::wait;

/// Where Word is on this machine — §2 of `TOOLCHAIN.md`.
pub const EXECUTABLE: &str = r"C:\Program Files (x86)\Microsoft Office\root\Office16\WINWORD.EXE";

/// Main window class of Word.
pub const MAIN_CLASS: &str = "OpusApp";

/// Class of the document content element, the one `SendInput` reaches.
pub const CONTENT_CLASS: &str = "_WwG";

/// **Rake 1.** Both dialog classes. A detector that knows only the first sees "a dialog with an
/// empty name" and can neither read it nor close it.
pub const DIALOG_CLASSES: [&str; 2] = ["#32770", "bosa_sdm_msword"];

/// A modal window Word put up, and what it says.
#[derive(Debug, Clone)]
pub struct Dialog {
    pub class: String,
    pub name: String,
    /// The static texts inside it — this is what a report has to quote verbatim when a dialog
    /// that §2 of `TOOLCHAIN.md` does not describe turns up.
    pub texts: Vec<String>,
}

impl Dialog {
    pub fn describe(&self) -> String {
        format!(
            "class={:?} name={:?} тексты: {}",
            self.class,
            self.name,
            if self.texts.is_empty() {
                "—".to_owned()
            } else {
                self.texts.join(" | ")
            }
        )
    }
}

/// Every dialog of either class belonging to `pid`.
///
/// **Rake 2.** Word's message windows are not always top-level: they appear as descendants of
/// `OpusApp`. Looking only at the children of the root element misses them, which is how a run
/// ends up waiting for a document that a modal window is standing in front of. Both levels are
/// searched here.
pub fn dialogs(automation: &Automation, pid: u32) -> Vec<Dialog> {
    let mut found = Vec::new();

    for window in automation.top_level_of(pid) {
        // The top level itself.
        if DIALOG_CLASSES.contains(&window.class().as_str()) {
            found.push(read_dialog(automation, &window));
        }

        // And everything under it — rake 2.
        for child in automation.find_all(&window, &|element: &Element| {
            DIALOG_CLASSES.contains(&element.class().as_str())
        }) {
            found.push(read_dialog(automation, &child));
        }
    }

    found
}

fn read_dialog(automation: &Automation, element: &Element) -> Dialog {
    let texts = automation
        .find_all(element, &|candidate: &Element| {
            let class = candidate.class();
            (class.contains("Static") || class.contains("Text")) && !candidate.name().is_empty()
        })
        .iter()
        .map(Element::name)
        .collect();

    Dialog {
        class: element.class(),
        name: element.name(),
        texts,
    }
}

/// Starts Word with a blank document.
///
/// `/w` is what `probe-word.ps1` proved out: a new session with an empty document, which is
/// what the scenario needs and what leaves nothing of the user's behind.
pub fn launch() -> Result<std::process::Child, String> {
    if !PathBuf::from(EXECUTABLE).exists() {
        return Err(format!("{EXECUTABLE} не найден"));
    }

    Command::new(EXECUTABLE)
        .arg("/w")
        .spawn()
        .map_err(|error| format!("не удалось запустить Word: {error}"))
}

/// Waits for the main window — requirement 1, asked of the tree rather than of a clock.
pub fn await_main(automation: &Automation, pid: u32) -> Option<Element> {
    automation.await_window(pid, wait::WINDOW_TIMEOUT, &|element: &Element| {
        element.class() == MAIN_CLASS
    })
}

/// Waits for the document content element inside the main window.
///
/// A separate wait from [`await_main`] on purpose: `OpusApp` is in the tree well before `_WwG`
/// is, and typing into the gap between them goes nowhere at all.
pub fn await_content(automation: &Automation, main: &Element) -> Option<Element> {
    automation.await_element(main, wait::WINDOW_TIMEOUT, &|element: &Element| {
        element.control_type() == Some(UIA_DocumentControlTypeId)
            || element.class() == CONTENT_CLASS
    })
}

/// **Rakes 4 and 5.** Closes Word through COM `Quit(0)`.
///
/// `WM_CLOSE` is ignored by Word (rake 4), and `Stop-Process` poisons the next launch with a
/// safe-mode prompt and a `Resiliency\DisabledItems` entry (rake 5). Neither is used: this
/// calls `GetActiveObject('Word.Application')` and then `Quit(0)` — `wdDoNotSaveChanges` — and
/// if that fails it says so and leaves the process alone rather than reaching for the thing
/// that breaks the next scenario.
pub fn quit() -> Result<String, String> {
    match crate::shell::run_script("word-quit.ps1", &[])? {
        (true, said) => Ok(said),
        (false, said) => Err(format!("word-quit.ps1: {said}")),
    }
}

/// Was a `Resiliency\DisabledItems` key created — the observable consequence of rake 5.
///
/// Checked after the run and reported. ⚠ The task is explicit that this key must be **not
/// created** rather than cleaned up, so this only reads.
pub fn resiliency_disabled_items() -> String {
    let marker = match resiliency_output() {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_owned(),
        Err(error) => return format!("не удалось проверить: {error}"),
    };

    match marker.as_str() {
        "NO_RESILIENCY" => SAID_NO_RESILIENCY.to_owned(),
        "RESILIENCY_ONLY" => SAID_RESILIENCY_ONLY.to_owned(),
        "DISABLEDITEMS_PRESENT" => SAID_DISABLEDITEMS_PRESENT.to_owned(),
        other => format!("непонятный ответ проверки: {other:?}"),
    }
}

/// Asks the registry the one question rake 5 is about, and hands back everything PowerShell
/// said — task **T-19-6**, finding 12 of the audit of 2026-08-31.
///
/// ⚠ **The marker the script prints is ASCII on purpose.** PowerShell hands its output back in
/// the console code page, not UTF-8, so a Cyrillic answer arrives as mojibake — which is exactly
/// what the first protocol of position 2 recorded, turning the one line that proves rake 5 was
/// avoided into unreadable bytes. The Russian wording is added on the Rust side.
///
/// # Why there is not a `\` in sight
///
/// Until this task every line of the script ended in `\`, as though `\` continued a line. It
/// does not: in PowerShell the continuation is a backtick, and a lone `\` is a command name.
/// Measured on this machine, the script therefore ran with **five**
/// `CommandNotFoundException`s on stderr and left with exit code 1.
///
/// ⭐ The audit expected stdout to be empty and the check to answer "непонятный ответ" every
/// time. It is not, and it did not: PowerShell recovered after each bad token and the `if`
/// blocks still evaluated, so the marker came back correctly — `RESILIENCY_ONLY` — and the
/// protocol's line was right by luck of the parser rather than by construction. What was really
/// lost was the ability to tell a working check from a broken one: with five errors on stderr
/// and a failing status, no reader could say whether the answer meant anything.
///
/// No continuation character is needed at all. A `{` block and a trailing `|` already carry a
/// PowerShell statement onto the next line, and those are the only two places this script breaks.
///
/// The `Output` is returned whole rather than reduced here, so that the check of this module can
/// hold the script to leaving **nothing** on stderr and to a clean exit. Neither the markers nor
/// what the bench's protocol does with them is touched.
fn resiliency_output() -> std::io::Result<Output> {
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r"$p='HKCU:\Software\Microsoft\Office\16.0\Word\Resiliency'
              if (Test-Path $p) {
                $d = Get-ChildItem $p -ErrorAction SilentlyContinue |
                     Where-Object { $_.PSChildName -eq 'DisabledItems' }
                if ($d) { 'DISABLEDITEMS_PRESENT' } else { 'RESILIENCY_ONLY' }
              } else { 'NO_RESILIENCY' }",
        ])
        .output()
}

/// The three lawful answers of [`resiliency_disabled_items`], one per marker.
///
/// Named rather than written inline since task **T-19-6**, so that the check below asserts on
/// the same words the protocol prints and cannot drift away from them. The wording and the
/// markers themselves are untouched — the protocol of the bench reads exactly what it always
/// read.
const SAID_NO_RESILIENCY: &str = "ключа Resiliency нет вовсе — DisabledItems не появился";
const SAID_RESILIENCY_ONLY: &str = "Resiliency есть, DisabledItems НЕТ — грабля 5 не наступила";
const SAID_DISABLEDITEMS_PRESENT: &str =
    "⚠ DisabledItems ЕСТЬ — Word был снят принудительно, грабля 5";

#[cfg(test)]
mod tests {
    use super::*;

    /// **Finding 12 of the audit of 2026-08-31, task T-19-6 — the instrument of rake 5 can
    /// answer at all.**
    ///
    /// The command was handed to `powershell -Command` with a `\` at the end of every line, as
    /// though `\` continued a line. In PowerShell the continuation is a backtick; the parser
    /// answered "Unexpected token", stdout came back empty, and the check returned "непонятный
    /// ответ" every single time. The line "Word был снят принудительно" or its absence — the
    /// evidence that rake 5 was avoided, printed in the protocol of every full run — had in
    /// fact been missing since the check was written.
    ///
    /// Reads the registry and nothing else: no Word, no window, no keystroke, and nothing is
    /// created or removed. Which of the three answers comes back depends on the machine, and
    /// that is why the assertion is "one of the three" — an instrument that cannot fail is not
    /// an instrument, and this one fails on exactly the shape the finding is about.
    #[test]
    fn the_resiliency_check_runs_clean_and_gives_one_of_its_three_lawful_answers() {
        let out = resiliency_output().expect("powershell runs");

        // The half that was red before task T-19-6. Five `CommandNotFoundException`s, one per
        // `\`, and an exit code of 1: the script was answering by accident of the parser, and
        // no reader of the protocol could tell a working check from a broken one.
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            "",
            "the script of rake 5 must say nothing at all on stderr"
        );
        assert!(
            out.status.success(),
            "the script of rake 5 must leave cleanly, and it left with {:?}",
            out.status.code()
        );

        // The half that was green before as well, and is the positive control: whatever else
        // was wrong, the marker itself did come back and the protocol's wording was right.
        let said = resiliency_disabled_items();

        assert!(
            [
                SAID_NO_RESILIENCY,
                SAID_RESILIENCY_ONLY,
                SAID_DISABLEDITEMS_PRESENT
            ]
            .contains(&said.as_str()),
            "the check of Resiliency\\DisabledItems answered {said:?}, which is none of its \
             three lawful answers"
        );
    }
}
