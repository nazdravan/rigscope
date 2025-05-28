//! Builds a [`Snapshot`](rigscope_model::Snapshot) on Windows from a [`WinSource`].
//! Every function here is safe Rust over raw data, so it is tested on Linux and macOS too.

mod errors;
mod source;
// The readers are used by the collector added in a later task.
#[allow(dead_code)]
mod system;

pub use errors::{field_err, field_of};
pub use source::{MemWin, WinSource, R};
