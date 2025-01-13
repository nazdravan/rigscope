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
    /// Opened on first use; kept alive so X11/Wayland keep serving the copied text.
    clipboard: Option<arboard::Clipboard>,
    /// Result channel of the helper thread while it runs.
    elevation: Option<std::sync::mpsc::Receiver<Result<Vec<u8>, EngineError>>>,
}

impl RealEnv {
    pub fn fixture(dir: &str) -> Self {
        RealEnv {
            source: SystemSource::Fixture(FixtureSource::new(dir)),
            clipboard: None,
            elevation: None,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn system() -> Self {
        RealEnv {
            source: SystemSource::Real(rigscope_probe::FsSource),
            clipboard: None,
            elevation: None,
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

    fn start_elevate(&mut self) -> Result<(), EngineError> {
        if self.elevation.is_some() {
            return Err(EngineError::Disabled(
                "already waiting for authentication".into(),
            ));
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            #[cfg(target_os = "linux")]
            let result = crate::dump_dmi::run_helper();
            #[cfg(not(target_os = "linux"))]
            let result = Err(EngineError::Io("elevated reading needs Linux".into()));
            let _ = tx.send(result);
        });
        self.elevation = Some(rx);
        Ok(())
    }

    fn poll_elevate(&mut self) -> Option<Result<Vec<u8>, EngineError>> {
        let rx = self.elevation.as_ref()?;
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err(EngineError::Io("helper thread ended".into()))
            }
        };
        self.elevation = None;
        Some(result)
    }

    fn clipboard_set(&mut self, text: &str) -> Result<(), EngineError> {
        let err = |e: arboard::Error| EngineError::Io(format!("clipboard: {e}"));
        if self.clipboard.is_none() {
            self.clipboard = Some(arboard::Clipboard::new().map_err(err)?);
        }
        match self.clipboard.as_mut() {
            Some(c) => c.set_text(text.to_string()).map_err(err),
            None => Err(EngineError::Io("clipboard unavailable".into())),
        }
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
