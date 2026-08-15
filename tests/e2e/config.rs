//! ⚠ **The user's `config.toml`, borrowed and given back byte for byte.**
//!
//! Position 17 needs the product running in the "Цикл" mode of FR-31 over three layouts, and
//! §7 puts that in a file — `%APPDATA%\Lang_Switcher\config.toml`, which belongs to a living
//! person and holds their settings. The bench read it before this task and never wrote it.
//!
//! # The one rule
//!
//! **The file goes back exactly as it was found, on every path out.** Not "usually", and not
//! "unless something went wrong": a bench that leaves somebody's settings rewritten has done
//! more harm than every position it could have passed is worth.
//!
//! # Three paths out, and what covers each
//!
//! | Path | What puts the file back |
//! |---|---|
//! | the position ends, however it ends | [`Borrowed::give_back`], called by the position |
//! | the bench panics | this type's `Drop`, run by the unwind |
//! | the bench is killed from outside, or panics in a build with `panic = "abort"` | **the stash on disk**, put back by [`recover`] at the start of the next run |
//!
//! The third row is why this module writes anything to disk at all. `Drop` does not run for a
//! process that is killed, and §12 of `Cargo.toml` makes the Release profile `panic = "abort"`,
//! where it does not run for a panic either. So the original is copied **beside nothing** — into
//! the bench's own scratch area under `%TEMP%` — *before* the file is touched, and the next run
//! of the bench finds it there and finishes the job the killed one could not.
//!
//! # Absence is a state
//!
//! A machine with no `config.toml` runs on the defaults of §7, and after a run it must have no
//! `config.toml` again. The stash records absence as the *absence of the copy* beside a marker
//! that names the target, so the two states are told apart rather than confused.
//!
//! # SEC-01, SEC-07
//!
//! Nothing from the file is ever printed. What reaches the report is the length in bytes, the
//! verdict of the byte-for-byte comparison, and the word "отсутствовал" — numbers and facts,
//! never contents.

use std::path::{Path, PathBuf};

/// Where the copy of the original waits for a run that may never come back for it.
///
/// Under `%TEMP%`, and **not** beside `config.toml`: the directory of §7 belongs to the product,
/// and leaving files of the bench's own in it would be its own kind of litter.
fn stash_dir() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("langsw-e2e-config-stash");
    path
}

/// The file naming what was borrowed. Its existence is what says a stash is outstanding.
fn stash_marker() -> PathBuf {
    stash_dir().join("target-path.txt")
}

/// The copy of the original, present only when the original was.
fn stash_copy() -> PathBuf {
    stash_dir().join("original.bin")
}

/// The configuration of §7, borrowed for the length of one position.
pub struct Borrowed {
    path: PathBuf,
    /// The bytes as they were found. `None` means there was no file — a state of its own.
    original: Option<Vec<u8>>,
    given_back: bool,
}

impl Borrowed {
    /// Copies the original aside, then writes `replacement` in its place.
    ///
    /// ⚠ The order is the whole safety property: the stash is complete on disk **before** the
    /// target is touched, so a process killed at any instant leaves the original recoverable.
    pub fn take(path: &Path, replacement: &str) -> Result<Self, String> {
        let original = match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("не удалось прочитать {}: {error}", path.display())),
        };

        // The stash first, and completely.
        std::fs::create_dir_all(stash_dir())
            .map_err(|error| format!("не удалось создать каталог запаса: {error}"))?;
        match &original {
            Some(bytes) => std::fs::write(stash_copy(), bytes)
                .map_err(|error| format!("не удалось сохранить копию конфигурации: {error}"))?,
            None => {
                // Absence is recorded by the copy not being there. A copy left by an earlier
                // borrow would turn "there was no file" into "there was one", so it goes.
                let _ = std::fs::remove_file(stash_copy());
            }
        }
        std::fs::write(
            stash_marker(),
            path.as_os_str().to_string_lossy().as_bytes(),
        )
        .map_err(|error| format!("не удалось записать метку запаса: {error}"))?;

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("не удалось создать каталог конфигурации: {error}"))?;
        }
        std::fs::write(path, replacement.as_bytes())
            .map_err(|error| format!("не удалось записать {}: {error}", path.display()))?;

        Ok(Self {
            path: path.to_path_buf(),
            original,
            given_back: false,
        })
    }

    /// How large the original was, or that there was none — for the report, without contents.
    pub fn describe_original(&self) -> String {
        match &self.original {
            Some(bytes) => format!("исходный config.toml: {} байт", bytes.len()),
            None => "исходного config.toml не было — это тоже состояние".to_owned(),
        }
    }

    /// Puts the file back and **checks** that it went back, byte for byte.
    ///
    /// The sentence it returns is printed by the run beside the clipboard's, which is what
    /// requirement 3 of the task asks for: the restoration is an assertion, not a promise.
    pub fn give_back(&mut self) -> String {
        if self.given_back {
            return "config.toml уже был возвращён".to_owned();
        }
        self.given_back = true;

        let written = match &self.original {
            Some(bytes) => {
                std::fs::write(&self.path, bytes).map_err(|error| format!("запись: {error}"))
            }
            None => match std::fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!("удаление: {error}")),
            },
        };

        if let Err(error) = written {
            return format!("⚠⚠ config.toml НЕ ВОЗВРАЩЁН: {error}");
        }

        // The comparison, made against the disk and not against the intention.
        let now = std::fs::read(&self.path);
        let verdict = match (&self.original, &now) {
            (Some(before), Ok(after)) if before == after => format!(
                "config.toml возвращён побитово: да ({} байт, сверено чтением)",
                after.len()
            ),
            (Some(before), Ok(after)) => format!(
                "⚠⚠ config.toml возвращён, но НЕ СОВПАЛ: было {} байт, стало {} байт",
                before.len(),
                after.len()
            ),
            (Some(_), Err(error)) => {
                format!("⚠⚠ config.toml после возврата не читается: {error}")
            }
            (None, Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                "config.toml отсутствовал до прогона и отсутствует после: да".to_owned()
            }
            (None, Ok(_)) => "⚠⚠ config.toml не было до прогона, но он есть после".to_owned(),
            (None, Err(error)) => format!("⚠⚠ состояние config.toml не читается: {error}"),
        };

        // The stash has done its job the moment the target is right again.
        let _ = std::fs::remove_file(stash_copy());
        let _ = std::fs::remove_file(stash_marker());
        let _ = std::fs::remove_dir(stash_dir());

        verdict
    }
}

impl Drop for Borrowed {
    fn drop(&mut self) {
        if !self.given_back {
            eprintln!("  {}", self.give_back());
        }
    }
}

/// ⚠ **The path no code of ours runs on**: a stash left by a run that was killed.
///
/// Called once, at the start of every run, before anything else touches a configuration.
/// `None` means there was nothing to recover, which is the ordinary case and is not printed.
pub fn recover() -> Option<String> {
    let marker = stash_marker();
    let target = std::fs::read_to_string(&marker).ok()?;
    let target = PathBuf::from(target.trim());
    if target.as_os_str().is_empty() {
        let _ = std::fs::remove_file(&marker);
        return Some("⚠ найдена метка запаса config.toml без пути — метка удалена".to_owned());
    }

    let restored = match std::fs::read(stash_copy()) {
        Ok(bytes) => match std::fs::write(&target, &bytes) {
            Ok(()) => format!(
                "⚠ прежний прогон был прерван: config.toml восстановлен из запаса, {} байт",
                bytes.len()
            ),
            Err(error) => {
                format!("⚠⚠ запас config.toml найден, но восстановить не удалось: {error}")
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // No copy beside the marker: the file did not exist when it was borrowed.
            match std::fs::remove_file(&target) {
                Ok(()) => {
                    "⚠ прежний прогон был прерван: config.toml не существовал до него и удалён"
                        .to_owned()
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    "⚠ прежний прогон был прерван: config.toml и так отсутствует, как и был"
                        .to_owned()
                }
                Err(error) => format!("⚠⚠ config.toml полагается удалить, но не удалось: {error}"),
            }
        }
        Err(error) => format!("⚠⚠ запас config.toml не читается: {error}"),
    };

    let _ = std::fs::remove_file(stash_copy());
    let _ = std::fs::remove_file(&marker);
    let _ = std::fs::remove_dir(stash_dir());

    Some(restored)
}

/// The text position 17 puts in place of the user's file — **the user's own settings with one
/// section changed**.
///
/// Everything outside `[layouts]` is carried over unchanged, which matters for more than
/// politeness: the hotkey the run presses is read from this file at the start of the run, and a
/// configuration built from the defaults of §7 would silently change it for the one position
/// that rewrites it.
///
/// Serialised by the product's own writer, so the file the product then reads is a file of the
/// shape §7 describes rather than one this bench believes in.
pub fn cycle_of_three(path: &Path, third: u32) -> Result<String, String> {
    let (mut config, _) = lang_switcher::settings::read_or_default(path);

    config.layouts.mode = lang_switcher::settings::LayoutMode::Cycle;
    config.layouts.cycle = vec![
        format!("0x{:08X}", crate::layout::US),
        format!("0x{:08X}", crate::layout::RUSSIAN),
        format!("0x{third:08X}"),
    ];

    config
        .to_toml_string()
        .map_err(|error| format!("не удалось построить config.toml режима «Цикл»: {error}"))
}
