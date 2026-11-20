//! **No test reaches the real places of the person** (`src\sandbox.rs`, the night of 2026-11-20): a
//! write of `HKCU\…\Run` or into `%APPDATA%\Lang_Switcher\` from a test process is stopped before it
//! is made. The guard is tried on places of the test's own that it is told to treat as the real ones
//! — a guard that failed would write there, and nowhere else.

#[path = "support/run_key.rs"]
mod run_key;

use lang_switcher::{sandbox, settings};
use run_key::OwnRunKey;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

/// What a call answered, or the words of the panic that stopped it.
fn refusal<T>(call: impl FnOnce() -> T) -> Option<String> {
    catch_unwind(AssertUnwindSafe(call)).err().map(|panic| {
        panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                panic
                    .downcast_ref::<&str>()
                    .map(|words| (*words).to_owned())
            })
            .unwrap_or_default()
    })
}

/// **This battery is a test process, and the program is not**: the image of a test stands in
/// `deps`; the program's anywhere else.
#[test]
fn a_test_image_stands_in_deps_and_the_program_does_not() {
    assert!(
        sandbox::running_as_test(),
        "this very test is a test process"
    );
    for (exe, test) in [
        (r"D:\cache\target\debug\deps\settings-0123abcd.exe", true),
        (r"D:\cache\target\release\deps\ui_fit-89ef.exe", true),
        (r"D:\cache\target\debug\LangSwitcher.exe", false),
        (r"C:\Program Files\Lang_Switcher\LangSwitcher.exe", false),
    ] {
        assert_eq!(sandbox::is_test_image(Path::new(exe)), test, "{exe}");
    }
    assert!(sandbox::lies_in(
        Path::new(r"C:\Users\x\AppData\Roaming\LANG_SWITCHER\config.toml"),
        Path::new(r"C:\Users\x\AppData\Roaming\Lang_Switcher")
    ));
    assert!(!sandbox::lies_in(
        Path::new(r"C:\Users\x\AppData\Roaming\Lang_Switcher-old\config.toml"),
        Path::new(r"C:\Users\x\AppData\Roaming\Lang_Switcher")
    ));
    assert!(
        sandbox::real_folder().is_some_and(|folder| folder.ends_with("Lang_Switcher")),
        "the real folder is asked of the shell"
    );
}

/// **A write of the autostart that would reach the real key is stopped before it is made** — both
/// the write and the removal; the key stays as it was. Tried on a key of the test's own that the
/// guard treats as the real one.
#[test]
fn a_write_of_the_real_run_key_is_stopped_before_it_is_made() {
    let own = OwnRunKey::new("guard");
    sandbox::pretend_real_run_key_on_this_thread(Some(&own.subkey));

    let written = refusal(|| settings::set_autostart(true));
    let removed = refusal(|| settings::set_autostart(false));
    sandbox::pretend_real_run_key_on_this_thread(None);

    for (what, words) in [("the write", written), ("the removal", removed)] {
        let words = words.unwrap_or_else(|| panic!("{what} of the real key was not stopped"));
        assert!(words.starts_with(sandbox::REFUSED), "{what}: {words}");
    }
    assert_eq!(
        settings::autostart_value(),
        None,
        "the write reached the key the guard treats as the real one"
    );

    // A key of the test's own, not treated as the real one, is written as the program writes.
    settings::set_autostart(true).expect("a key of the test's own is written");
    assert_eq!(settings::autostart_value(), settings::autostart_command());
}

/// **A write into the real `%APPDATA%\Lang_Switcher` is stopped before it is made**: the file is not
/// there afterwards. Tried on a folder of the test's own that the guard treats as the real one.
#[test]
fn a_write_into_the_real_folder_is_stopped_before_it_is_made() {
    let folder: PathBuf = std::env::temp_dir().join(format!("langsw-guard-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder of the test's own");
    let file = folder.join("config.toml");
    sandbox::pretend_real_folder_on_this_thread(Some(&folder));

    let written = refusal(|| settings::write_bytes_to(&file, b"[general]\r\n"));
    let quarantined = refusal(|| settings::quarantine(&file));
    sandbox::pretend_real_folder_on_this_thread(None);

    let exists = file.exists();
    let _ = std::fs::remove_dir_all(&folder);
    for (what, words) in [("the write", written), ("the quarantine", quarantined)] {
        let words = words.unwrap_or_else(|| panic!("{what} into the real folder was not stopped"));
        assert!(words.starts_with(sandbox::REFUSED), "{what}: {words}");
    }
    assert!(
        !exists,
        "the write reached the folder the guard treats as the real one"
    );
}
