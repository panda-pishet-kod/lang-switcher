//! Lang_Switcher: a resident keyboard layout correction utility for Windows.
//!
//! The user types `ghbdtn` in the English layout, presses the hotkey, and gets `привет`
//! together with a switched layout.
//!
//! This is the crate skeleton produced by task T-01-1. Every module below is a stub that
//! carries only its documented responsibility, taken from the module table in section 6.2
//! of SPEC, and the identifiers of the requirements it will cover. The product logic
//! arrives with the later tasks of stages E1 to E10.
//!
//! The crate builds as two targets: this library and the `LangSwitcher` binary. Integration
//! tests under `tests\` can only link against a library target, which is why the split
//! exists from the very first task (decision R-11).

pub mod app;
pub mod buffer;
pub mod convert;
pub mod diag;
pub mod guard;
pub mod hook;
pub mod inject;
pub mod layouts;
pub mod selection;
pub mod settings;
pub mod switch;
pub mod theme;
pub mod tray;
pub mod watchdog;

/// SEC-04a debug control channel, the single documented exception to SEC-04.
///
/// Compiled only under the `testing` feature, which is absent from the Release
/// configuration; acceptance criterion 8 of section 13 of SPEC verifies that the shipped
/// binary carries no trace of it. Implemented by task T-03-4.
#[cfg(feature = "testing")]
pub mod control;

/// Display name in the tray, in the about box and in the installer.
///
/// Decision 7 of DECISIONS.md: with a space, without an underscore, and identical in both
/// interface locales (FR-94).
pub const APP_NAME: &str = "Lang Switcher";

/// File name of the executable.
///
/// Decision 6 of DECISIONS.md. Hard-wired into the `LangSw-Install` scheduled task, so it
/// is not subject to renaming.
pub const EXE_NAME: &str = "LangSwitcher.exe";

/// Directory name used for the program's own folders.
///
/// Section 7 of SPEC: the configuration lives in `%APPDATA%\Lang_Switcher\config.toml`.
/// Section 8.4 uses the same name for `%ProgramFiles%\Lang_Switcher`. Note that this is
/// the underscored form, unlike [`APP_NAME`].
pub const CONFIG_DIR_NAME: &str = "Lang_Switcher";
