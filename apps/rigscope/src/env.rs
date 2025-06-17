//! The real environment: filesystem, clock, clipboard, files, the elevated read.

use rigscope_engine::{EngineError, Env};
use rigscope_model::Collector;
use rigscope_probe::{FixtureSource, LinuxCollector, Source};
use std::io::Read;

/// In a fixture directory, the smartctl JSON the simulated elevated read returns per disk.
pub const FIXTURE_SMART_DIR: &str = "/root/smart";

/// Largest file `read_file` accepts.
pub const MAX_READ: u64 = 4 * 1024 * 1024;

/// How the elevated read works in this environment.
pub enum Elevation {
    /// No elevated read (Windows runs as administrator; demo of a Windows fixture).
    None,
    /// Fixture mode: `root/smart/<disk>.json` simulates it.
    Fixture(FixtureSource),
    /// Real Linux: pkexec + the dump helper.
    #[cfg(target_os = "linux")]
    Linux(rigscope_probe::FsSource),
}

pub struct RealEnv {
    pub collector: Box<dyn Collector>,
    pub elevation_kind: Elevation,
    /// Opened on first use; kept alive so X11/Wayland keep serving the copied text.
    clipboard: Option<arboard::Clipboard>,
    /// Result channel of the elevated read while it runs.
    elevation: Option<std::sync::mpsc::Receiver<Result<Vec<u8>, EngineError>>>,
}

impl RealEnv {
    pub fn new(collector: Box<dyn Collector>, elevation_kind: Elevation) -> Self {
        RealEnv {
            collector,
            elevation_kind,
            clipboard: None,
            elevation: None,
        }
    }

    /// A Linux-layout fixture directory.
    pub fn fixture(dir: &str) -> Self {
        Self::new(
            Box::new(LinuxCollector::new(FixtureSource::new(dir))),
            Elevation::Fixture(FixtureSource::new(dir)),
        )
    }

    #[cfg(target_os = "linux")]
    pub fn system() -> Self {
        Self::new(
            Box::new(LinuxCollector::new(rigscope_probe::FsSource)),
            Elevation::Linux(rigscope_probe::FsSource),
        )
    }
}

impl Env for RealEnv {
    fn collector(&mut self) -> &mut dyn Collector {
        self.collector.as_mut()
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
        match &self.elevation_kind {
            Elevation::None => {
                return Err(EngineError::Disabled(
                    "no elevated read on this platform".into(),
                ));
            }
            // Fixture mode simulates the elevated read with the fixture's `root/smart/<disk>.json`.
            Elevation::Fixture(f) => {
                let smart = |name: &str| {
                    f.read(&format!("{FIXTURE_SMART_DIR}/{name}.json"))
                        .unwrap_or_default()
                };
                let result = crate::dump_dmi::dump(f, &smart).map_err(EngineError::Io);
                let _ = tx.send(result);
            }
            #[cfg(target_os = "linux")]
            Elevation::Linux(s) => {
                // The disk list is read here, as the user; the root side only checks it.
                let disks = crate::dump_dmi::disks_to_query(s);
                std::thread::spawn(move || {
                    let _ = tx.send(crate::dump_dmi::run_elevated(&disks));
                });
            }
        }
        self.elevation = Some(rx);
        Ok(())
    }

    fn poll_elevate(&mut self) -> Option<Result<Vec<u8>, EngineError>> {
        let rx = self.elevation.as_ref()?;
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(EngineError::Io(
                "elevated read ended without a result".into(),
            )),
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
        match &self.elevation_kind {
            Elevation::None => false,
            #[cfg(target_os = "linux")]
            Elevation::Linux(_) => true,
            Elevation::Fixture(f) => f.list(FIXTURE_SMART_DIR).is_ok(),
        }
    }
}
