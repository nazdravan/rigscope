//! The real environment: filesystem, clock, clipboard, files, privileged helper.

use rigscope_engine::{EngineError, Env};
use rigscope_probe::{FixtureSource, Source};
use std::io::Read;

/// Largest file `read_file` accepts.
pub const MAX_READ: u64 = 4 * 1024 * 1024;

pub enum SystemSource {
    #[cfg(target_os = "linux")]
    Real(rigscope_probe::FsSource),
    Fixture(FixtureSource),
}

pub struct RealEnv {
    pub source: SystemSource,
}

impl RealEnv {
    pub fn fixture(dir: &str) -> Self {
        RealEnv {
            source: SystemSource::Fixture(FixtureSource::new(dir)),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn system() -> Self {
        RealEnv {
            source: SystemSource::Real(rigscope_probe::FsSource),
        }
    }
}

impl Env for RealEnv {
    fn source(&self) -> &dyn Source {
        match &self.source {
            #[cfg(target_os = "linux")]
            SystemSource::Real(s) => s,
            SystemSource::Fixture(s) => s,
        }
    }

    fn now_unix(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn elevate_dump(&self) -> Result<Vec<u8>, EngineError> {
        #[cfg(target_os = "linux")]
        {
            crate::dump_dmi::run_helper()
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(EngineError::Io("elevated reading needs Linux".into()))
        }
    }

    fn clipboard_set(&mut self, _text: &str) -> Result<(), EngineError> {
        Err(EngineError::Io("clipboard not available yet".into()))
    }

    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), EngineError> {
        std::fs::write(path, bytes).map_err(|e| EngineError::Io(format!("{path}: {e}")))
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, EngineError> {
        let file =
            std::fs::File::open(path).map_err(|e| EngineError::Io(format!("{path}: {e}")))?;
        let mut buf = Vec::new();
        file.take(MAX_READ + 1)
            .read_to_end(&mut buf)
            .map_err(|e| EngineError::Io(format!("{path}: {e}")))?;
        if buf.len() as u64 > MAX_READ {
            return Err(EngineError::Io(format!("{path}: file too large")));
        }
        Ok(buf)
    }

    fn can_elevate(&self) -> bool {
        match self.source {
            #[cfg(target_os = "linux")]
            SystemSource::Real(_) => true,
            SystemSource::Fixture(_) => false,
        }
    }
}
