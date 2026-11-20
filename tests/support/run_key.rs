//! **A key of the test's own in place of `HKCU\…\Run`** — the night of 2026-11-20: no test writes
//! the person's autostart. The key is made under `HKCU\Software\Lang_Switcher-tests\`, this thread
//! writes and reads it in place of the real `Run` (`settings::use_run_key_on_this_thread`), and it
//! is taken away with everything in it when the value of this type goes.

#![allow(dead_code)]

use lang_switcher::settings;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_ALL_ACCESS, REG_OPTION_VOLATILE, RegCloseKey, RegCreateKeyExW,
    RegDeleteKeyW, RegDeleteTreeW,
};
use windows::core::{HSTRING, PCWSTR};

/// The folder of the tests' own keys under `HKEY_CURRENT_USER`.
const TESTS: &str = r"Software\Lang_Switcher-tests";

/// A key of the test's own, used by this thread in place of the real `Run` while the value lives.
pub struct OwnRunKey {
    pub subkey: String,
}

impl OwnRunKey {
    /// Makes `HKCU\Software\Lang_Switcher-tests\<name>-<process>` — volatile, so that even a test
    /// killed half-way leaves nothing after the session — and turns this thread to it.
    pub fn new(name: &str) -> Self {
        let subkey = format!(r"{TESTS}\{name}-{}", std::process::id());
        let mut key = HKEY::default();
        // SAFETY: a key under the current user named by a live string; the handle is closed below.
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(subkey.as_str()),
                None,
                PCWSTR::null(),
                REG_OPTION_VOLATILE,
                KEY_ALL_ACCESS,
                None,
                &mut key,
                None,
            )
        };
        status.ok().expect("a key of the test's own under HKCU");
        // SAFETY: the handle made just above, closed once.
        let _ = unsafe { RegCloseKey(key) };
        settings::use_run_key_on_this_thread(Some(&subkey));
        Self { subkey }
    }
}

impl Drop for OwnRunKey {
    fn drop(&mut self) {
        settings::use_run_key_on_this_thread(None);
        // SAFETY: the key this value made, with everything in it; then the folder of the tests'
        // keys, which goes only when nothing else is in it.
        unsafe {
            let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(self.subkey.as_str()));
            let _ = RegDeleteKeyW(HKEY_CURRENT_USER, &HSTRING::from(self.subkey.as_str()));
            let _ = RegDeleteKeyW(HKEY_CURRENT_USER, &HSTRING::from(TESTS));
        }
    }
}
