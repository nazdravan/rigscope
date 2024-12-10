use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EngineError {
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("command not available: {0}")]
    Disabled(String),
    #[error("bad parameters: {0}")]
    BadParams(String),
    #[error("probe failed: {0}")]
    Probe(String),
    #[error("{0}")]
    Io(String),
    #[error("internal error; the previous state was restored")]
    Panicked,
}
