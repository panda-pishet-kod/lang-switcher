// Compiles app.rc into the binary: icons, VERSIONINFO and a manifest.
//
// The manifest is selected here, and that is load-bearing rather than tidy. Windows runs an
// image whose manifest asks for uiAccess="true" only when the machine trusts its signature
// AND it lies in a secure folder such as %ProgramFiles%. Otherwise it refuses, in one of three
// ways -- each measured, and none of them a quiet start as an ordinary program:
//
//   CreateProcess, any folder, any signature      error 740, nothing starts
//                                                 (T-09-2; the Setup window of 155в)
//   ShellExecute, a signature the machine does    error 8235 "A referral was returned from
//   not trust, or none                            the server", nothing starts (T-03-1, 155в)
//   ShellExecute, a trusted signature, outside    starts, with UIAccess = 0 (T-09-2)
//   %ProgramFiles%
//
// Three images come out of this one script:
//
//   Debug -- cargo test, cargo run    app-dev.manifest. The test harness is built from this
//                                     same bin target, so the uiAccess manifest would make
//                                     `cargo test` fail before its first test runs.
//   Release                           app.manifest, uiAccess -- the FULL image, LangSwitcher.exe.
//   Release, LANGSW_NO_UIACCESS=1     app-dev.manifest -- the BASE image, LangSwitcher-base.exe:
//                                     the same body without uiAccess (stage E91, question 155).
//                                     It starts on any machine; the hotkey does not reach the
//                                     windows of programs run as administrator.
//
// tools\release.ps1 builds both Release images, and tools\verify-one-body.ps1 proves they are
// one body: they differ in the manifest resource and in the fields derived from the content
// hash, and nowhere else. Decision R-01, TOOLCHAIN.md section 8.2, SPEC.md section 8.2.

fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    // Stage L10N, task T-L10N-2: the string tables live in res/lang/<tag>.rc, included by app.rc.
    // rc.exe follows the #include, cargo does not: without this line an edited translation would
    // leave the old strings in the binary. A folder is watched whole, any file below it.
    println!("cargo:rerun-if-changed=res/lang");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app-dev.manifest");
    println!("cargo:rerun-if-changed=res/langswitcher-active.ico");
    println!("cargo:rerun-if-changed=res/langswitcher-paused.ico");
    println!("cargo:rerun-if-changed=res/langswitcher-active-unread.ico");
    println!("cargo:rerun-if-changed=res/langswitcher-paused-unread.ico");
    println!("cargo:rerun-if-env-changed=LANGSW_NO_UIACCESS");

    let release = std::env::var("PROFILE").as_deref() == Ok("release");

    // `1` asks for the base image; unset or empty, for the full one. Any other value stops the
    // build instead of guessing: a `0` that one reader takes for "off" and another for "set" is
    // how a full image would ship under the base image's name, or the other way round.
    let base = match std::env::var("LANGSW_NO_UIACCESS") {
        Err(std::env::VarError::NotPresent) => false,
        Ok(value) if value.is_empty() => false,
        Ok(value) if value == "1" => true,
        other => panic!("LANGSW_NO_UIACCESS must be 1 or unset, not {other:?}"),
    };
    let macros: &[&str] = if release && !base {
        &["LANGSW_UIACCESS=1"]
    } else {
        &[]
    };

    // SEC-08, "the build is reproducible" -- and this line is what makes it true. Measured by
    // task T-09-1, not assumed: two Release builds of identical sources from the cleanest
    // state available differed in exactly 20 bytes out of 668672, in five runs, and every one
    // of those runs is something link.exe writes out of the state of the machine rather than
    // out of the sources.
    //
    //   0x000000F8   IMAGE_FILE_HEADER.TimeDateStamp -- wall clock of the link
    //   0x00096CD4   |
    //   0x00096CF0   |- TimeDateStamp of the three IMAGE_DEBUG_DIRECTORY entries
    //   0x00096D0C   |
    //   0x00096E78   the 16-byte GUID of the CodeView RSDS record, freshly generated per link
    //
    // /Brepro is the linker's deterministic-output switch, and what it actually does here was
    // read out of the built file rather than taken from the documentation. It appends a fourth
    // debug directory entry, IMAGE_DEBUG_TYPE_REPRO, holding a 32-byte hash of the content;
    // every TimeDateStamp above then becomes one value derived from that hash instead of the
    // clock, and the CodeView GUID becomes the first 16 bytes of it. Both builds of the
    // repeated experiment came out with SHA-256 64A19E56...75CF61BE.
    //
    // Note what this is NOT: a timestamp switch. The 16-byte run above is a GUID, and a fix
    // aimed only at the clock would have left it moving. The file does not grow, because the
    // extra entry fits in the slack already present in .rdata.
    //
    // Release only, deliberately. SEC-08 is about the artifact that ships, /Brepro and
    // /INCREMENTAL are mutually exclusive, and a linker warning in the Debug build would cost
    // acceptance point 1 for nothing the requirement asked for.
    if release {
        println!("cargo:rustc-link-arg-bins=/Brepro");
    }

    // ⭐ Stage E104 (TZ E104 p. 13): two resource scripts, each for its own binaries. Until E104
    // `compile` linked app.rc into EVERY binary of the crate -- the program and the bench were
    // the only ones -- and the updater would have carried the manifest of the program, uiAccess
    // and all. The updater has no window and no uiAccess: updater.rc, with updater.manifest and a
    // version block whose numbers come from [package] version (no twelfth place of the version).
    println!("cargo:rerun-if-changed=updater.rc");
    println!("cargo:rerun-if-changed=updater.manifest");

    embed_resource::compile_for("app.rc", ["LangSwitcher", "langsw-e2e"], macros)
        .manifest_required()
        .unwrap();

    shipped_languages();

    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo sets the package version");
    let mut numbers = version.split('.').map(|part| {
        part.parse::<u16>()
            .unwrap_or_else(|_| panic!("the package version {version} is not three numbers"))
    });
    let version_macros = [
        format!("LANGSW_VER_MAJOR={}", numbers.next().unwrap_or(0)),
        format!("LANGSW_VER_MINOR={}", numbers.next().unwrap_or(0)),
        format!("LANGSW_VER_PATCH={}", numbers.next().unwrap_or(0)),
    ];

    embed_resource::compile_for("updater.rc", ["LangSwitcherUpdate"], &version_macros)
        .manifest_required()
        .unwrap();
}

// Stage E108, task T-108-2: the languages this build offers are the entries of the registry
// res/lang/languages.toml that say `state = "shipped"` -- so a language joins the program with its
// file res/lang/<tag>.rc and one line of the registry, and src/settings.rs holds every value of the
// closed set without a second list to keep in step. Written to OUT_DIR/shipped_languages.rs as
// one array of tags, in the order of the registry; tests/l10n.rs reads the registry with the toml
// crate and holds this list to it.
//
// The registry is read line by line and not with the toml crate: a build dependency would be a
// crate added to the closed list of SPEC section 3.2 (criterion 4 of section 13 counts
// `cargo tree`). The shape it reads is the one the file has: `[[language]]` starts an entry,
// `tag = "..."` and `state = "..."` are two of its lines. Anything else it does not understand
// about those two keys stops the build rather than leaving a language out in silence.
fn shipped_languages() {
    let path = "res/lang/languages.toml";
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"));

    let quoted = |line: &str, key: &str| -> Option<String> {
        let rest = line
            .strip_prefix(key)?
            .trim_start()
            .strip_prefix('=')?
            .trim();
        let value = rest.strip_prefix('"')?.strip_suffix('"')?;
        Some(value.to_owned())
    };

    let mut entries: Vec<(Option<String>, Option<String>)> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line == "[[language]]" {
            entries.push((None, None));
        } else if let Some(entry) = entries.last_mut() {
            if line.starts_with("tag ") || line.starts_with("tag=") {
                entry.0 = Some(quoted(line, "tag").unwrap_or_else(|| panic!("{path}: {line}")));
            } else if line.starts_with("state ") || line.starts_with("state=") {
                entry.1 = Some(quoted(line, "state").unwrap_or_else(|| panic!("{path}: {line}")));
            }
        }
    }

    let mut shipped = Vec::new();
    for (tag, state) in entries {
        let tag = tag.unwrap_or_else(|| panic!("{path}: an entry without a tag"));
        match state.as_deref() {
            Some("shipped") => shipped.push(tag),
            Some("planned") => {}
            other => panic!("{path}: «{tag}» has the state {other:?}, not shipped or planned"),
        }
    }

    let list: Vec<String> = shipped.iter().map(|tag| format!("{tag:?}")).collect();
    let code = format!(
        "/// The tags of `res/lang/languages.toml` with `state = \"shipped\"`, in its order — build.rs.\n\
         pub(crate) const SHIPPED: [&str; {}] = [{}];\n",
        shipped.len(),
        list.join(", ")
    );

    let out = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
    std::fs::write(
        std::path::Path::new(&out).join("shipped_languages.rs"),
        code,
    )
    .expect("OUT_DIR/shipped_languages.rs must be writable");
}
