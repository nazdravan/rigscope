#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use rigscope_engine::testutil::FakeEnv;
use rigscope_engine::*;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn app() -> (App, Arc<Mutex<rigscope_engine::testutil::FakeState>>) {
    let env = FakeEnv::fixture("desktop-ddr4");
    let h = env.handle();
    (App::new(Box::new(env)), h)
}

#[test]
fn copy_text_masks_by_default() {
    let (mut a, h) = app();
    a.run("report.copy_text", json!({})).unwrap();
    let clip = h.lock().unwrap().clipboard.clone().unwrap();
    assert!(clip.contains("KHX2666C16/8G"));
    assert!(clip.contains("****2C3D"));
    assert!(!clip.contains("4A1B2C3D"));
    a.run("report.copy_text", json!({"mask": false})).unwrap();
    assert!(h
        .lock()
        .unwrap()
        .clipboard
        .clone()
        .unwrap()
        .contains("4A1B2C3D"));
    let spec = a
        .specs()
        .iter()
        .find(|s| s.id == "report.copy_text")
        .unwrap();
    assert_eq!(spec.shortcut, Some("Ctrl+Shift+C"));
}

#[test]
fn export_text_writes_file() {
    let (mut a, h) = app();
    a.run("report.export_text", json!({"path": "/out/r.txt"}))
        .unwrap();
    let f = h.lock().unwrap().files.get("/out/r.txt").cloned().unwrap();
    assert!(String::from_utf8(f).unwrap().contains("== Memory =="));
    assert!(matches!(
        a.run("report.export_text", json!({})),
        Err(EngineError::BadParams(_))
    ));
}

#[test]
fn save_open_roundtrip() {
    let (mut a, h) = app();
    a.run("snapshot.save", json!({"path": "/s.json", "mask": false}))
        .unwrap();
    let saved = a.session.snapshot.clone();
    let (mut b, hb) = {
        let env = FakeEnv::fixture("minimal-vm");
        let hb = env.handle();
        (App::new(Box::new(env)), hb)
    };
    let bytes = h.lock().unwrap().files.get("/s.json").cloned().unwrap();
    hb.lock().unwrap().files.insert("/s.json".into(), bytes);
    b.run("snapshot.open", json!({"path": "/s.json"})).unwrap();
    assert_eq!(b.session.snapshot, saved);
    assert_eq!(b.session.history.len(), 1);
    assert!(
        !b.session.live,
        "an opened snapshot is not this machine; live updates stop"
    );
}

#[test]
fn save_masks_by_default() {
    let (mut a, h) = app();
    a.run("snapshot.save", json!({"path": "/s.json"})).unwrap();
    let text = String::from_utf8(h.lock().unwrap().files.get("/s.json").cloned().unwrap()).unwrap();
    assert!(!text.contains("4A1B2C3D"));
}

#[test]
fn open_rejects_wrong_schema_garbage_and_oversize() {
    let (mut a, h) = app();
    let mut snap = a.session.snapshot.clone();
    snap.schema = 99;
    h.lock()
        .unwrap()
        .files
        .insert("/old.json".into(), serde_json::to_vec(&snap).unwrap());
    h.lock()
        .unwrap()
        .files
        .insert("/junk.json".into(), b"{not json".to_vec());
    h.lock()
        .unwrap()
        .files
        .insert("/big.json".into(), vec![b' '; 4 * 1024 * 1024 + 1]);
    let before = a.session.snapshot.clone();
    for p in ["/old.json", "/junk.json", "/big.json"] {
        assert!(
            matches!(
                a.run("snapshot.open", json!({"path": p})),
                Err(EngineError::BadParams(_))
            ),
            "{p}"
        );
    }
    assert!(a
        .run("snapshot.open", json!({"path": "/missing.json"}))
        .is_err());
    assert_eq!(a.session.snapshot, before);
}

#[test]
fn copy_compat_line() {
    let (mut a, h) = app();
    a.run("memory.copy_compat", json!({})).unwrap();
    assert_eq!(
        h.lock().unwrap().clipboard.as_deref(),
        Some("DDR4 DIMM 2666 MT/s 32GB")
    );
}

#[test]
fn copy_compat_disabled_without_dmi() {
    let mut a = App::new(Box::new(FakeEnv::fixture("minimal-vm")));
    assert!(matches!(
        a.run("memory.copy_compat", json!({})),
        Err(EngineError::Disabled(_))
    ));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]
    #[test]
    fn open_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let env = FakeEnv::fixture("minimal-vm");
        env.handle().lock().unwrap().files.insert("/x".into(), bytes);
        let mut a = App::new(Box::new(env));
        let r = a.run("snapshot.open", json!({"path": "/x"}));
        prop_assert!(!matches!(r, Err(EngineError::Panicked)));
    }
}
