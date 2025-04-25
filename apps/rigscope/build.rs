//! Packs `fixtures/desktop-ddr4` into `$OUT_DIR/demo_fixture.bin`, the demo machine that builds
//! for other systems than Linux carry inside the binary (see `src/demo.rs`).

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
    Ok(())
}
