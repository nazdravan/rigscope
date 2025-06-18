//! Built-in commands.

mod export;
pub(crate) mod snapshot;
mod view;

use crate::CommandSpec;

pub fn builtin() -> Vec<CommandSpec> {
    let mut v = Vec::new();
    v.extend(snapshot::specs());
    v.extend(view::specs());
    v.extend(export::specs());
    v
}

/// `enabled` for commands that are always available.
pub(crate) fn always(_: &crate::Session) -> Result<(), String> {
    Ok(())
}
