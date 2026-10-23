#![cfg_attr(not(test), windows_subsystem = "windows")]
#![allow(non_snake_case)]

// LangSwitcherUpdate.exe — the updater of stage Э104 (вопрос 189; TZ E104 §0 п. 13).
//
// A thin entry point, as src\main.rs is for the program (decision R-18): everything it does lives
// in `lang_switcher::update`, where the integration tests under tests\ can reach it. No window and
// no `uiAccess` — its manifest is updater.manifest, linked by build.rs into this file only.
//
// non_snake_case is allowed for the reason main.rs gives: the [[bin]] target is named
// LangSwitcherUpdate, which is what produces LangSwitcherUpdate.exe, the name the task of the
// scheduler and the installer know it by.

use std::process::ExitCode;

fn main() -> ExitCode {
    lang_switcher::update::run()
}
