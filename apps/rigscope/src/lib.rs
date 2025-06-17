//! The modules of the `rigscope` binary. They live in this library so their tests run in the
//! library's test executable: on Windows the binary carries a `requireAdministrator` manifest
//! (see `build.rs`), and Cargo links it into the binary's own test harness too, which then
//! could not start without a UAC prompt. The binary target therefore has `test = false`.

pub mod args;
pub mod capture;
pub mod control;
pub mod demo;
pub mod demo_pack;
pub mod dump_dmi;
pub mod env;
