//! **The resource script as rc.exe reads it** — stage L10N, task T-L10N-2.
//!
//! Until stage L10N the string tables of all fourteen languages stood in `app.rc`, and a test
//! that swept the tables read that one file. Since then `app.rc` carries the templates, the
//! `#define` lines and fourteen `#include "res/lang/<tag>.rc"` lines, and every language is a
//! file of its own. This is the **one** reader that puts them back together: `app.rc` with each
//! `#include` line replaced by the text of the file it names — what rc.exe compiles. A test that
//! sweeps strings reads [`resource_script`], never `app.rc` alone: a sweep of `app.rc` alone finds
//! no string at all and passes for the wrong reason.
//!
//! Included by `#[path]` from each test file that needs it, so every crate of `tests\` that sweeps
//! strings shares this body and none carries a copy of the parsing.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// The root of the crate, where `app.rc` lives and where its `#include` paths start.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The file an `#include` line names, or `None` for any other line.
fn included(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix("#include")?.trim();
    rest.strip_prefix('"')?.strip_suffix('"')
}

/// The text of a file of the crate, line ends folded to `\n`.
fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()))
        .replace("\r\n", "\n")
}

/// The files `app.rc` includes, in the order of its `#include` lines, as paths relative to the
/// root with forward slashes (`res/lang/ru.rc`).
pub fn included_files() -> Vec<String> {
    read(&root().join("app.rc"))
        .lines()
        .filter_map(included)
        .map(str::to_owned)
        .collect()
}

/// The language files: the tag (the file name without `.rc`) and the text of each, in the order
/// of the `#include` lines of `app.rc`.
pub fn language_files() -> Vec<(String, String)> {
    included_files()
        .into_iter()
        .filter(|path| path.starts_with("res/lang/"))
        .map(|path| {
            let tag = path
                .trim_start_matches("res/lang/")
                .trim_end_matches(".rc")
                .to_owned();
            (tag, read(&root().join(&path)))
        })
        .collect()
}

/// `app.rc` with every `#include` line replaced by the text of the file it names, line ends
/// folded to `\n`. The language files include nothing themselves; a nested `#include` would be
/// expanded the same way, down to a depth that no script of this crate needs.
pub fn resource_script() -> String {
    expand(&root(), "app.rc", 0)
}

fn expand(root: &Path, name: &str, depth: usize) -> String {
    assert!(depth < 4, "{name}: #include nested deeper than any script of this crate");
    let mut out = String::new();
    for line in read(&root.join(name)).lines() {
        match included(line) {
            Some(path) => out.push_str(&expand(root, path, depth + 1)),
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}
