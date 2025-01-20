//! `FakeEnv`: an in-memory [`Env`] whose state tests can inspect through a shared handle.

use crate::{EngineError, Env};
use rigscope_probe::{FixtureSource, MemSource, Source};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default)]
pub struct FakeState {
    pub clipboard: Option<String>,
    pub files: BTreeMap<String, Vec<u8>>,
    /// What `elevate_dump` returns; `None` = "cancelled".
    pub elevate: Option<Vec<u8>>,
    pub elevate_calls: u32,
    /// Polls that answer "still running" before the result is returned.
    pub pending_polls: u32,
    pub now: u64,
}

pub enum FakeSource {
    Mem(MemSource),
    Fixture(FixtureSource),
}

pub struct FakeEnv {
    pub source: FakeSource,
    pub state: Arc<Mutex<FakeState>>,
    pub can_elevate: bool,
}

impl FakeEnv {
    /// A fake over one of the repository fixtures (`desktop-ddr4`, `laptop-ddr5`, `minimal-vm`).
    pub fn fixture(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name);
        FakeEnv {
            source: FakeSource::Fixture(FixtureSource::new(root)),
            state: Arc::default(),
            can_elevate: true,
        }
    }

    pub fn mem(src: MemSource) -> Self {
        FakeEnv {
            source: FakeSource::Mem(src),
            state: Arc::default(),
            can_elevate: true,
        }
    }

    /// A handle to inspect clipboard/files after the env moved into the app.
    pub fn handle(&self) -> Arc<Mutex<FakeState>> {
        Arc::clone(&self.state)
    }

    fn st(&self) -> std::sync::MutexGuard<'_, FakeState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Env for FakeEnv {
    fn source(&self) -> &dyn Source {
        match &self.source {
            FakeSource::Mem(m) => m,
            FakeSource::Fixture(f) => f,
        }
    }
    fn now_unix(&self) -> u64 {
        self.st().now
    }
    fn start_elevate(&mut self) -> Result<(), EngineError> {
        self.st().elevate_calls += 1;
        Ok(())
    }
    fn poll_elevate(&mut self) -> Option<Result<Vec<u8>, EngineError>> {
        let mut st = self.st();
        if st.pending_polls > 0 {
            st.pending_polls -= 1;
            return None;
        }
        Some(
            st.elevate
                .clone()
                .ok_or_else(|| EngineError::Io("cancelled".into())),
        )
    }
    fn clipboard_set(&mut self, text: &str) -> Result<(), EngineError> {
        self.st().clipboard = Some(text.to_string());
        Ok(())
    }
    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), EngineError> {
        self.st().files.insert(path.to_string(), bytes.to_vec());
        Ok(())
    }
    fn read_file(&self, path: &str) -> Result<Vec<u8>, EngineError> {
        self.st()
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| EngineError::Io(format!("{path}: not found")))
    }
    fn can_elevate(&self) -> bool {
        self.can_elevate
    }
}
