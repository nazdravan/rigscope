//! Windows access for rigscope. `types` and `layout` are plain Rust (built and tested on every
//! OS); `ffi` calls Win32 and is the only code in the workspace allowed to use `unsafe`.

pub mod layout;
pub mod types;

pub use types::*;
