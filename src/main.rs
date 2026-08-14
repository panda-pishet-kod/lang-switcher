#![cfg_attr(not(test), windows_subsystem = "windows")]
#![allow(non_snake_case)]

// Lang_Switcher executable entry point.
//
// The windows_subsystem attribute is conditional on purpose (decision R-12). The program is
// resident and owns no console, but applying the attribute unconditionally takes the console
// away from the test harness as well: `cargo test` still builds and still runs, and its
// output goes nowhere.
//
// non_snake_case is allowed because the [[bin]] target is named LangSwitcher, which is what
// produces LangSwitcher.exe. That name is fixed by decision 6 of DECISIONS.md and is already
// hard-wired into the LangSw-Install scheduled task, so the lint is suppressed rather than
// the target renamed.
//
// This file stays a thin entry point and gains nothing else (decision R-18). The process
// lifecycle — the single instance of FR-82, the three threads of section 6.1, their hidden
// windows, their message loops and shutdown — lives in `lang_switcher::app`, that is, in the
// library rather than in the binary: code in a binary is reachable neither by the
// integration tests under tests\ nor by any later task.

use std::process::ExitCode;

fn main() -> ExitCode {
    lang_switcher::app::run()
}
