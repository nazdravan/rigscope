use crate::EngineError;
use rigscope_probe::Source;

/// Everything the engine needs from the outside world. The app provides the real one,
/// tests provide `testutil::FakeEnv`.
pub trait Env {
    fn source(&self) -> &dyn Source;
    fn now_unix(&self) -> u64;
    /// Runs the privileged helper and returns its raw dump bytes.
    fn elevate_dump(&self) -> Result<Vec<u8>, EngineError>;
    fn clipboard_set(&mut self, text: &str) -> Result<(), EngineError>;
    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), EngineError>;
    fn read_file(&self, path: &str) -> Result<Vec<u8>, EngineError>;
    /// Whether the elevated helper can work on this platform.
    fn can_elevate(&self) -> bool;
}
