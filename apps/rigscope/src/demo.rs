//! The built-in demo machine, shown when rigscope runs where it cannot read the machine and no
//! `--fixture` was given (macOS; Linux and Windows read themselves). The blob is made by
//! `build.rs`; it is unpacked into a private temporary directory that is removed on exit.

use crate::demo_pack;
use std::path::{Path, PathBuf};

#[cfg_attr(any(target_os = "linux", windows), allow(dead_code))]
static BLOB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/demo_fixture.bin"));

/// A temporary directory holding the unpacked demo fixture; removed on drop.
#[cfg_attr(any(target_os = "linux", windows), allow(dead_code))]
pub struct DemoDir(PathBuf);

#[cfg_attr(any(target_os = "linux", windows), allow(dead_code))]
impl DemoDir {
    pub fn unpack() -> std::io::Result<Self> {
        let dir = std::env::temp_dir().join(format!("rigscope-demo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let me = DemoDir(dir);
        demo_pack::unpack(BLOB, &me.0)?;
        Ok(me)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for DemoDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rigscope_probe::{FixtureSource, Source};

    #[test]
    fn the_embedded_demo_reads_like_the_fixture() {
        let dir = DemoDir::unpack().unwrap();
        let src = FixtureSource::new(dir.path());
        assert!(src.read("/proc/cpuinfo").is_ok());
        // Directory symlinks must still resolve.
        assert!(src.canonical("/sys/block/sda").is_ok());
        let path = dir.path().to_path_buf();
        drop(dir);
        assert!(!path.exists(), "the temporary directory is removed");
    }
}
