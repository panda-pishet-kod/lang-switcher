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
    // The probes of the interface shell (stage Э120), opened alone, in the testing build only: the
    // window «О программе» (T-120-9), the fields and the list (T-120-18), the popup and the window
    // in the corner (T-120-14), the letters (T-120-16).
    // A table behind `black_box`: a comparison with one literal may be folded into numbers, and
    // then the name is not in the file as text — the positive control of criterion 8 reads it there.
    #[cfg(feature = "testing")]
    {
        type Probe = (&'static str, fn() -> ExitCode);
        static PROBES: [Probe; 5] = [
            ("--ui-probe-about", lang_switcher::ui::probe::run_about),
            ("--ui-probe-fields", lang_switcher::ui::probe::run_fields),
            ("--ui-probe-popup", lang_switcher::ui::probe::run_popup),
            ("--ui-probe-corner", lang_switcher::ui::probe::run_corner),
            (
                "--ui-probe-letters",
                lang_switcher::letters::testing::run_probe,
            ),
        ];
        for argument in std::env::args() {
            if let Some((_, run)) = std::hint::black_box(&PROBES)
                .iter()
                .find(|(name, _)| *name == argument)
            {
                return run();
            }
        }
    }

    lang_switcher::app::run()
}
