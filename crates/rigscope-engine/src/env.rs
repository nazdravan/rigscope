use crate::EngineError;
use rigscope_model::Collector;

/// Everything the engine needs from the outside world. The app provides the real one,
/// tests provide `testutil::FakeEnv`.
pub trait Env {
    fn collector(&mut self) -> &mut dyn Collector;
    fn now_unix(&self) -> u64;
    /// Runs the privileged helper and returns its raw dump bytes.
    /// Starts the privileged helper in the background (it may wait minutes for the user to
    /// authenticate, so it must not block the UI thread).
    fn start_elevate(&mut self) -> Result<(), EngineError>;
    /// The helper's raw dump once it finished; `None` while it is still running.
    fn poll_elevate(&mut self) -> Option<Result<Vec<u8>, EngineError>>;
    fn clipboard_set(&mut self, text: &str) -> Result<(), EngineError>;
    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), EngineError>;
    fn read_file(&self, path: &str) -> Result<Vec<u8>, EngineError>;
    /// Whether the elevated helper can work on this platform.
    fn can_elevate(&self) -> bool;
}
