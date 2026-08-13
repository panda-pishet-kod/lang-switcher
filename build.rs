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

    embed_resource::compile("app.rc", macros)
        .manifest_required()
        .unwrap();
}
