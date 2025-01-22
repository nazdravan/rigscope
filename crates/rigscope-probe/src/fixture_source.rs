use crate::fsutil::{list_dir, map_io, read_capped};
use crate::{ReadError, Source};
use std::path::PathBuf;

/// Reads absolute paths below a fixture directory (`<root>/proc/cpuinfo`, ...).
#[derive(Debug, Clone)]
pub struct FixtureSource {
    root: PathBuf,
}

impl FixtureSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        FixtureSource { root: root.into() }
    }
}

impl FixtureSource {
    /// Maps an absolute Linux path into the fixture root; `..` components are refused.
    fn resolve(&self, path: &str) -> Result<PathBuf, ReadError> {
        let mut out = self.root.clone();
        for part in path.split('/').filter(|p| !p.is_empty()) {
            if part == ".." || part == "." {
                return Err(ReadError::NotFound);
            }
            out.push(part);
        }
        Ok(out)
    }
}

impl Source for FixtureSource {
    fn read(&self, path: &str) -> Result<Vec<u8>, ReadError> {
        read_capped(&self.resolve(path)?)
    }

    fn list(&self, dir: &str) -> Result<Vec<String>, ReadError> {
        list_dir(&self.resolve(dir)?)
    }

    fn canonical(&self, path: &str) -> Result<String, ReadError> {
        let root = std::fs::canonicalize(&self.root).map_err(map_io)?;
        let real = std::fs::canonicalize(self.resolve(path)?).map_err(map_io)?;
        // Links that leave the fixture root are treated as missing.
        let rel = real.strip_prefix(&root).map_err(|_| ReadError::NotFound)?;
        Ok(format!("/{}", rel.to_string_lossy()))
    }
}
