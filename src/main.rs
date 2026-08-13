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
// Task T-01-1 asks nothing more of this file than that it build, start and exit with code 0
// without opening a window. The hidden window and the message loop are task T-01-2.

fn main() {}
