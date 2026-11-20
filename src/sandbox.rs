//! **No test reaches the real places of the person** — the guard of the night of 2026-11-20 (stage
//! Э120): a test of the settings window went the live road of applying and wrote the path of its own
//! test executable into the person's `HKCU\…\Run`.
//!
//! While the process is a test binary of cargo — its image stands in a folder named `deps`, which
//! is where `cargo test`, with or without `--features testing`, puts every test executable — a write
//! to the real `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` or into the real
//! `%APPDATA%\Lang_Switcher\` stops the process with a panic **before** it is made: the test fails
//! and names the place, and the place stays as it was. Tests write into places of their own — a
//! folder under `%TEMP%`, a key of their own (`settings::use_run_key_on_this_thread`).
//!
//! The real folder is asked of the shell (`FOLDERID_RoamingAppData`), not of the environment: a test
//! that moved `APPDATA` elsewhere must not move the guard with it. In the program itself — an image
//! anywhere but in `deps` — nothing here ever refuses.

use std::path::{Component, Path, PathBuf};

/// The words a refusal starts with, for the tests that look for it.
pub const REFUSED: &str = "a test reached a real place of the person";

/// Whether `exe` is a test binary of cargo: it stands in a folder named `deps`. Pure.
pub fn is_test_image(exe: &Path) -> bool {
    exe.parent()
        .and_then(Path::file_name)
        .is_some_and(|folder| folder.eq_ignore_ascii_case("deps"))
}

/// Whether this process is a test binary of cargo.
pub fn running_as_test() -> bool {
    std::env::current_exe().is_ok_and(|exe| is_test_image(&exe))
}

/// The real `%APPDATA%\Lang_Switcher\` — the roaming application data the shell knows, whatever the
/// environment says.
pub fn real_folder() -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{
        FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
    };

    // SAFETY: a GUID of the shell's; no token means «this process's user».
    let path =
        unsafe { SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, None) }.ok()?;
    // SAFETY: the call answered a NUL-terminated string it allocated; read once, freed once below.
    let text = unsafe { path.to_string() }.ok();
    // SAFETY: frees exactly the string the call allocated, once.
    unsafe { CoTaskMemFree(Some(path.0.cast())) };

    text.map(|folder| Path::new(&folder).join(crate::CONFIG_DIR_NAME))
}

/// Whether `path` lies in `folder`, compared component by component and without regard to case.
/// Pure.
pub fn lies_in(path: &Path, folder: &Path) -> bool {
    let folded = |path: &Path| -> Vec<String> {
        path.components()
            .filter(|component| !matches!(component, Component::CurDir))
            .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
            .collect()
    };
    let (path, folder) = (folded(path), folded(folder));
    path.len() >= folder.len() && path[..folder.len()] == folder[..]
}

/// Stops a test before it writes `path` in the real `%APPDATA%\Lang_Switcher\`.
pub fn guard_path(path: &Path) {
    let real = in_pretended_folder(path) || real_folder().is_some_and(|real| lies_in(path, &real));
    if real && running_as_test() {
        panic!(
            "{REFUSED}: a write into the real %APPDATA%\\Lang_Switcher ({}) — write into a folder \
             of the test's own",
            path.display()
        );
    }
}

/// Stops a test before it writes the real `HKCU\…\Run`.
pub fn guard_run_key() {
    if running_as_test() {
        panic!(
            "{REFUSED}: a write into the real HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run \
             — set a key of the test's own (settings::use_run_key_on_this_thread)"
        );
    }
}

thread_local! {
    /// A key of a test's own the guard treats as the real `Run` on this thread.
    static PRETEND_RUN_KEY: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    /// A folder of a test's own the guard treats as the real `%APPDATA%\Lang_Switcher` on this
    /// thread.
    static PRETEND_FOLDER: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// **For the tests of the guard itself**: the key under `HKEY_CURRENT_USER` of the test's own that
/// the guard treats as the real `Run` on this thread — so that the guard is tried on a place that
/// may be written, and a guard that failed would write there and nowhere else. `None` — no such key.
#[doc(hidden)]
pub fn pretend_real_run_key_on_this_thread(subkey: Option<&str>) {
    PRETEND_RUN_KEY.with(|cell| *cell.borrow_mut() = subkey.map(str::to_owned));
}

/// **For the tests of the guard itself**: the folder of the test's own that the guard treats as the
/// real `%APPDATA%\Lang_Switcher` on this thread. `None` — no such folder.
#[doc(hidden)]
pub fn pretend_real_folder_on_this_thread(folder: Option<&Path>) {
    PRETEND_FOLDER.with(|cell| *cell.borrow_mut() = folder.map(Path::to_path_buf));
}

/// Whether the key `subkey` of a test's own is the one the guard treats as the real `Run` now.
pub fn pretended_real_run_key(subkey: &str) -> bool {
    PRETEND_RUN_KEY.with(|cell| {
        cell.borrow()
            .as_deref()
            .is_some_and(|pretended| pretended.eq_ignore_ascii_case(subkey))
    })
}

/// Whether `path` lies in the folder the guard treats as the real one on this thread.
fn in_pretended_folder(path: &Path) -> bool {
    PRETEND_FOLDER.with(|cell| {
        cell.borrow()
            .as_deref()
            .is_some_and(|folder| lies_in(path, folder))
    })
}
