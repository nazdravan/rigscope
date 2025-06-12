//! Packs `fixtures/desktop-ddr4` into `$OUT_DIR/demo_fixture.bin`, the demo machine that builds
//! for other systems than Linux carry inside the binary (see `src/demo.rs`). For a Windows
//! target it also embeds `windows/rigscope.rc` (administrator manifest, icon, version) into
//! the `rigscope` binary.

#[allow(dead_code)]
#[path = "src/demo_pack.rs"]
mod demo_pack;

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let fixture = manifest.join("../../fixtures/desktop-ddr4");
    println!("cargo:rerun-if-changed={}", fixture.display());
    let out = PathBuf::from(std::env::var("OUT_DIR")?).join("demo_fixture.bin");
    std::fs::write(out, demo_pack::pack(&fixture)?)?;
    windows_resources()?;
    Ok(())
}

/// The manifest (`requireAdministrator`), icon and version resource, for a Windows target only.
fn windows_resources() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=windows/rigscope.rc");
    println!("cargo:rerun-if-changed=windows/rigscope.manifest");
    println!("cargo:rerun-if-changed=windows/rigscope.ico");
    // Target, not host: only Windows executables get the manifest.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    let v = env!("CARGO_PKG_VERSION");
    let core = v.split(['-', '+']).next().unwrap_or(v);
    let mut parts: Vec<&str> = core.split('.').collect();
    parts.resize(4, "0");
    let comma = parts.join(",");
    // `compile_for` links the resource into the named binaries only
    // (`cargo:rustc-link-arg-bin=rigscope=...`): the test executables must not ask for UAC.
    embed_resource::compile_for(
        "windows/rigscope.rc",
        ["rigscope"],
        [
            format!("RIGSCOPE_VERSION_COMMA={comma}"),
            format!("RIGSCOPE_VERSION_STR=\"{v}\""),
        ],
    )
    .manifest_required()
    .map_err(|e| format!("Windows resources: {e}"))?;
    Ok(())
}
