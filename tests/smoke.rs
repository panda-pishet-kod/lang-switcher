//! Integration smoke test for task T-01-1.
//!
//! Proves that the library plus binary plus `tests\` wiring is assembled correctly and that
//! `cargo test` really does execute tests. This is not a formality: an empty `cargo test`
//! reports success even with a broken harness, and from stage E2 onwards the whole test set
//! of section 11.1 of SPEC depends on this directory.

use lang_switcher::{APP_NAME, CONFIG_DIR_NAME, EXE_NAME};

#[test]
fn crate_constants_match_the_recorded_decisions() {
    // Decision 7 of DECISIONS.md: a space, not an underscore.
    assert_eq!(APP_NAME, "Lang Switcher");
    // Decision 6 of DECISIONS.md, hard-wired into the LangSw-Install scheduled task.
    assert_eq!(EXE_NAME, "LangSwitcher.exe");
    // Section 7 of SPEC: the configuration lives in %APPDATA%\Lang_Switcher\config.toml.
    assert_eq!(CONFIG_DIR_NAME, "Lang_Switcher");
}
