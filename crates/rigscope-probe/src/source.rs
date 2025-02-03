use std::collections::BTreeMap;

/// Largest file a source returns; bigger files are an error, not a truncated read.
pub const MAX_FILE_LEN: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    NotFound,
    PermissionDenied,
    TooLarge,
    Io(String),
}

/// Read access to absolute Linux paths (`/proc/cpuinfo`, ...).
pub trait Source {
    fn read(&self, path: &str) -> Result<Vec<u8>, ReadError>;
    /// Entry names (not paths) in a directory.
    fn list(&self, dir: &str) -> Result<Vec<String>, ReadError>;
    /// The path with symlinks resolved (sysfs links reveal the bus: `/usb`, `/ata`, `/virtio`).
    fn canonical(&self, path: &str) -> Result<String, ReadError>;
}

/// In-memory source for tests and the web build.
#[derive(Debug, Clone, Default)]
pub struct MemSource {
    pub files: BTreeMap<String, Vec<u8>>,
    /// Paths that answer `PermissionDenied`.
    pub denied: Vec<String>,
    /// Symlinks: path → canonical target.
    pub links: BTreeMap<String, String>,
}

impl MemSource {
    pub fn with(mut self, path: &str, contents: impl Into<Vec<u8>>) -> Self {
        self.files.insert(path.to_string(), contents.into());
        self
    }
}

impl Source for MemSource {
    fn read(&self, path: &str) -> Result<Vec<u8>, ReadError> {
        if self.denied.iter().any(|d| d == path) {
            return Err(ReadError::PermissionDenied);
        }
        let data = self.files.get(path).ok_or(ReadError::NotFound)?;
        if data.len() > MAX_FILE_LEN {
            return Err(ReadError::TooLarge);
        }
        Ok(data.clone())
    }

    fn list(&self, dir: &str) -> Result<Vec<String>, ReadError> {
        let prefix = format!("{}/", dir.trim_end_matches('/'));
        let mut names: Vec<String> = self
            .files
            .keys()
            .filter_map(|k| k.strip_prefix(&prefix))
            .filter_map(|rest| rest.split('/').next())
            .map(str::to_string)
            .collect();
        names.dedup();
        if names.is_empty() {
            return Err(ReadError::NotFound);
        }
        Ok(names)
    }

    fn canonical(&self, path: &str) -> Result<String, ReadError> {
        if let Some(target) = self.links.get(path) {
            return Ok(target.clone());
        }
        let prefix = format!("{}/", path.trim_end_matches('/'));
        if self.files.contains_key(path) || self.files.keys().any(|k| k.starts_with(&prefix)) {
            Ok(path.to_string())
        } else {
            Err(ReadError::NotFound)
        }
    }
}
