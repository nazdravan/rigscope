//! rigscope engine: the [`Session`] (current snapshot + history), the command registry and the
//! panic guard. Every user-visible action is a command run through [`App::run`].

mod command;
mod commands;
mod env;
mod error;
mod session;
#[cfg(any(test, feature = "testutil"))]
pub mod testutil;

pub use command::{App, CommandSpec};
pub use env::Env;
pub use error::EngineError;
pub use session::{Language, SectionId, Session, HISTORY_LIMIT};
