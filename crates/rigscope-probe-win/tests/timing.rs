#![cfg(windows)]
use rigscope_model::*;
use rigscope_probe_win::{RealWin, WinCollector};
use std::time::Instant;

/// Run by hand, non-elevated: `cargo test -p rigscope-probe-win --test timing -- --ignored --nocapture`.
#[test]
#[ignore]
fn real_collect_duration() {
    let t = Instant::now();
    let mut c = WinCollector::new(RealWin::new());
    let s = c.collect(None, 0);
    println!("collect took {:?}", t.elapsed());
    let _ = s;
}
