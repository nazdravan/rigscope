//! Raw macOS data: the JSON of `system_profiler <type> -json` and `sysctl -n <name>`.
//! `RealMac` (macOS only) runs the tools; [`MemMac`] replays saved values (tests, fixtures).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Why a macOS read failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MacError {
    NotAvailable,
    Timeout,
    Failed(String),
}

pub type R<T> = Result<T, MacError>;

pub trait MacSource {
    /// The JSON text of `system_profiler <data_type> -json`, e.g. `SPMemoryDataType`.
    fn profiler(&self, data_type: &str) -> R<String>;
    /// The trimmed output of `sysctl -n <name>`.
    fn sysctl(&self, name: &str) -> R<String>;
}

/// File name of a macOS fixture inside its directory.
pub const FIXTURE_FILE: &str = "macsource.json";

/// Largest fixture file `load` reads.
pub const MAX_FIXTURE_LEN: u64 = 8 * 1024 * 1024;

/// Saved values. The profiler output is kept as parsed JSON so a fixture is readable and
/// diffable; a missing entry reads as `NotAvailable`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MemMac {
    pub format: u32,
    pub profiler: BTreeMap<String, serde_json::Value>,
    pub sysctl: BTreeMap<String, String>,
}

impl MemMac {
    pub fn new() -> Self {
        MemMac {
            format: 1,
            ..MemMac::default()
        }
    }

    pub fn with_profiler(mut self, data_type: &str, json: &str) -> Self {
        if let Ok(v) = serde_json::from_str(json) {
            self.profiler.insert(data_type.to_string(), v);
        }
        self
    }

    pub fn with_sysctl(mut self, name: &str, value: &str) -> Self {
        self.sysctl.insert(name.to_string(), value.to_string());
        self
    }

    pub fn is_fixture(dir: &Path) -> bool {
        dir.join(FIXTURE_FILE).is_file()
    }

    pub fn load(dir: &Path) -> Result<Self, String> {
        let path = dir.join(FIXTURE_FILE);
        let len = std::fs::metadata(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .len();
        if len > MAX_FIXTURE_LEN {
            return Err(format!("{}: file too large", path.display()));
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let m: MemMac =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if m.format != 1 {
            return Err(format!("{}: unknown format {}", path.display(), m.format));
        }
        Ok(m)
    }

    /// Writes the fixture into `dir` (created if needed).
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let path = dir.join(FIXTURE_FILE);
        std::fs::write(&path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))
    }
}

impl MacSource for MemMac {
    fn profiler(&self, data_type: &str) -> R<String> {
        self.profiler
            .get(data_type)
            .map(|v| v.to_string())
            .ok_or(MacError::NotAvailable)
    }

    fn sysctl(&self, name: &str) -> R<String> {
        self.sysctl.get(name).cloned().ok_or(MacError::NotAvailable)
    }
}
