// Compiles app.rc into the binary: icons, VERSIONINFO and a manifest.
//
// The manifest is selected by build profile, and that is load-bearing rather than tidy.
// A binary carrying uiAccess="true" cannot be launched unless it is signed AND located
// in %ProgramFiles%: CreateProcess fails with error 740 instead of degrading. Since the
// test harness is built from this same bin target, embedding the uiAccess manifest
// unconditionally makes `cargo test` impossible in principle -- the tests fail before the
// first one runs. Section 8.2 of SPEC already mandates the Debug/Release manifest split;
// this is the build-side half of it. Decision R-01, TOOLCHAIN.md section 8.2.

fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=app.manifest");
    println!("cargo:rerun-if-changed=app-dev.manifest");
    println!("cargo:rerun-if-changed=res/langswitcher-active.ico");
    println!("cargo:rerun-if-changed=res/langswitcher-paused.ico");

    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    let macros: &[&str] = if release { &["LANGSW_UIACCESS=1"] } else { &[] };

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
