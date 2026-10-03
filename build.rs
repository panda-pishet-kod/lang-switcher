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

    embed_resource::compile("app.rc", macros)
        .manifest_required()
        .unwrap();
}
