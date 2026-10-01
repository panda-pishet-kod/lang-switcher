//! The tree names no folder of the author's machine — task **T-89-1**, decision 152.
//!
//! The repository goes public with its history (decision 152.1), and before it does, every path
//! of the author's own disk layout leaves the files of the tree: the defaults of the delivery
//! scripts, the source of the installer, the paths the tests read, the comments and the README.
//! Not because those paths are secret — they are not — but because a stranger cannot build the
//! delivery with them, and because a folder of one machine written into the code is a promise
//! that the code works on that machine only. Since stage E89 the scripts take their folders from
//! the `LANGSW_*` environment variables, set by hand or in the untracked `tools\local.ps1`, and
//! fall back to `<repository>\dist\`; the tests that read something outside the repository skip
//! with a printed line when their variable is not set.
//!
//! # The sweep, and why it cannot measure itself
//!
//! The folder is the author's development folder: drive `D`, folder `dev`. This file never spells
//! the path whole — not in code and not in prose: the matcher below looks for the drive and for
//! the folder separately, and the samples of its control are assembled at run time. A sweep whose
//! own text carries the needle is red on a clean tree (lesson of stage E22), and a sweep that
//! skips its own file to stay green has stopped checking one file of its list. This file is in
//! the list, and it is read like every other.
//!
//! The path is matched in every spelling the tree has carried or could carry: either case of the
//! drive letter, a backslash or a forward slash, a doubled backslash — the escaped form a Rust
//! string literal and an attribute reason write — and the Git Bash spelling with the drive letter
//! as a folder. [`the_matcher_finds_every_spelling_of_the_folder_and_nothing_else`] keeps that
//! true, so a later edit of the matcher cannot quietly narrow it down to nothing.
//!
//! # What is read
//!
//! Every file under [`FOLDERS`], recursively, and every file of the root, except the binary kinds
//! of [`BINARY_EXTENSIONS`] and the two files of [`NOT_OF_THE_TREE`], which exist on a machine
//! but never in the repository. The red answer of this sweep on the base of stage E89 (`752c170`)
//! named the sixty-two lines decision 152 counted, and one more the count had not seen: the reason
//! of an `#[ignore]` attribute in `tests\feed.rs`, where the path was written with doubled
//! backslashes.

use std::fs;
use std::path::{Path, PathBuf};

/// The folders read whole: every file below them, at any depth.
const FOLDERS: [&str; 5] = ["src", "tests", "tools", "installer", ".github"];

/// The files of the root this sweep insists on finding — a list that no longer matches the root
/// is a list that has stopped describing what it reads. Every other file of the root is read as
/// well; these are only the ones whose absence fails the test.
const ROOT_FILES: [&str; 14] = [
    ".gitattributes",
    ".gitignore",
    "app-dev.manifest",
    "app.manifest",
    "app.rc",
    "build.rs",
    "CLA.md",
    "Cargo.lock",
    "Cargo.toml",
    "CONTRIBUTING.md",
    "LICENSE-APACHE",
    "LICENSE-MIT",
    "README.md",
    "SPEC.md",
];

/// Files that exist on a machine and never in the repository, by their path from the root.
///
/// * `.git` — in a linked worktree git writes a one-line pointer file here, and it names the
///   main checkout by its absolute path. In the main checkout `.git` is a folder and is not read.
/// * `tools/local.ps1` — the machine's own values of the `LANGSW_*` variables (stage E89). It is
///   ignored by `.gitignore` and exists precisely to hold the paths this sweep keeps out of the
///   tree.
const NOT_OF_THE_TREE: [&str; 2] = [".git", "tools/local.ps1"];

/// File kinds that are not text: the icons and sounds of `res\`, and whatever a build or an
/// installer compile may leave under a folder of [`FOLDERS`].
const BINARY_EXTENSIONS: [&str; 10] = [
    "ico", "png", "bmp", "wav", "exe", "dll", "pdb", "lib", "exp", "res",
];

/// Files of the tree that the walk must have read, or the walk is not the walk this test claims.
const MUST_BE_READ: [&str; 7] = [
    "README.md",
    "SPEC.md",
    "installer/LangSwitcher.iss",
    "tools/release.ps1",
    "src/lib.rs",
    "tests/portability.rs",
    ".github/PULL_REQUEST_TEMPLATE.md",
];

/// A byte that continues a word — the boundary rule of [`names_the_development_folder`].
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Whether `line` names the author's development folder, in any spelling.
///
/// The line is folded to lower case, and a match is the drive — `d` and a colon, or the Git Bash
/// form, a slash and `d` — at the start of a word, then one or more separators, backslashes or
/// slashes in any mix, then the folder name, which must end there: a longer folder name that only
/// begins the same way is another folder.
fn names_the_development_folder(line: &str) -> bool {
    let folded = line.to_ascii_lowercase();
    let text = folded.as_bytes();

    (0..text.len()).any(|at| {
        let at_a_boundary = at == 0 || !is_word(text[at - 1]);
        let drive = text[at..].starts_with(b"d:") || text[at..].starts_with(b"/d");

        if !(at_a_boundary && drive) {
            return false;
        }

        let rest = &text[at + 2..];
        let separators = rest
            .iter()
            .take_while(|byte| matches!(byte, b'\\' | b'/'))
            .count();
        let rest = &rest[separators..];

        separators > 0 && rest.starts_with(b"dev") && rest.get(3).is_none_or(|byte| !is_word(*byte))
    })
}

/// The path of `path` from the root of the crate, with forward slashes — the spelling of
/// [`NOT_OF_THE_TREE`], [`MUST_BE_READ`] and of the report.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Whether the file at `path` is one this sweep reads: not binary, not a file of the machine.
fn is_read(root: &Path, path: &Path) -> bool {
    let binary = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            BINARY_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
        });

    !binary && !NOT_OF_THE_TREE.contains(&relative(root, path).as_str())
}

/// Every file below `folder`, at any depth, in a stable order.
fn files_below(folder: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(folder)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", folder.display()))
        .map(|entry| entry.expect("a readable directory entry").path())
        .collect();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            files_below(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// **The sweep of decision 152: not one line of the tree names the author's development folder.**
///
/// Red, it names every line it found, by file and number, with the line itself — the list a
/// person fixes from. The checks in front of it are what make a green answer mean something: the
/// root files it insists on are there, the files it must have read were read, and the count of
/// files read is not a count of a walk that went nowhere.
#[test]
fn no_file_of_the_tree_names_the_authors_development_folder() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    for name in ROOT_FILES {
        assert!(
            root.join(name).is_file(),
            "{name} is in the list of the root and not in the root: the list is out of date"
        );
    }

    let mut paths = Vec::new();

    let mut root_files: Vec<PathBuf> = fs::read_dir(root)
        .expect("the root of the crate must be readable")
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|path| path.is_file())
        .collect();
    root_files.sort();
    paths.extend(root_files);

    for folder in FOLDERS {
        files_below(&root.join(folder), &mut paths);
    }

    let mut read = Vec::new();
    let mut offenders = Vec::new();

    for path in paths.into_iter().filter(|path| is_read(root, path)) {
        let name = relative(root, &path);
        let bytes =
            fs::read(&path).unwrap_or_else(|error| panic!("{name} must be readable: {error}"));
        let text = String::from_utf8_lossy(&bytes);

        for (index, line) in text.split('\n').enumerate() {
            let line = line.trim_end_matches('\r');
            if names_the_development_folder(line) {
                offenders.push(format!("  {name}:{}: {}", index + 1, line.trim()));
            }
        }

        read.push(name);
    }

    for name in MUST_BE_READ {
        assert!(
            read.iter().any(|path| path == name),
            "{name} was not read: the walk does not cover what this test claims it covers"
        );
    }
    // The base of stage E89 holds 84 tracked text files beside its 11 icons and sounds; a walk
    // that skipped a folder of [`FOLDERS`] would fall far below this floor.
    assert!(
        read.len() >= 80,
        "{} files read — fewer than the tree holds; the walk went somewhere else",
        read.len()
    );

    assert!(
        offenders.is_empty(),
        "{} line(s) of the tree name the author's development folder ({} files read):\n{}",
        offenders.len(),
        read.len(),
        offenders.join("\n")
    );
}

/// The path `drive:` + `separator` + `folder` + `tail`, assembled at run time — so that this
/// file, which the sweep reads, never carries a sample whole.
fn spelt(drive: char, separator: &str, folder: &str, tail: &str) -> String {
    format!("{drive}:{separator}{folder}{tail}")
}

/// **The matcher catches every spelling of the folder, and nothing that only resembles it.**
///
/// The positive half is what keeps the sweep able to turn red: each spelling the tree has carried
/// — the backslash path of a script default, the forward slashes of a shell, the doubled
/// backslashes of a Rust string, the Git Bash form — must be found. The negative half is what
/// keeps it usable: another drive, a longer folder name, a word that merely ends in `d`, the
/// device folder of Unix and plain prose are not the folder.
#[test]
fn the_matcher_finds_every_spelling_of_the_folder_and_nothing_else() {
    let found = [
        spelt('D', "\\", "dev", "\\cache\\target"),
        spelt('d', "/", "dev", "/artifacts"),
        spelt('D', "\\\\", "dev", "\\\\artifacts, which exists only here"),
        format!("under `{}`", spelt('D', "\\", "dev", "")),
        format!(
            "const SITE: &str = r\"{}\";",
            spelt('D', "\\", "dev", "\\artifacts")
        ),
        format!(
            "$Artifact = '{}'",
            spelt('D', "\\", "dev", "\\artifacts\\LangSwitcher.exe")
        ),
        format!("cd /{}/{}/prog/Lang_Switcher", "d", "dev"),
    ];

    for line in &found {
        assert!(
            names_the_development_folder(line),
            "the matcher must find the folder in: {line}"
        );
    }

    let not_found = [
        spelt('D', "\\", "devices", ""),
        spelt('C', "\\", "dev", "\\cache"),
        spelt('D', "", "dev", ""),
        format!("cm{}", spelt('d', "\\", "dev", "")),
        "/dev/null".to_owned(),
        "the dev folder of drive D".to_owned(),
    ];

    for line in &not_found {
        assert!(
            !names_the_development_folder(line),
            "the matcher must not take this for the folder: {line}"
        );
    }
}
