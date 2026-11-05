#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_engine::testutil::FakeEnv;
use rigscope_engine::*;
use rigscope_model::DmiSource;
use rigscope_model::{DmiDump, encode_dmi_dump};
use rigscope_probe::{MemSource, paths};
use serde_json::json;

#[test]
fn opened_snapshot_blocks_live_until_refresh() {
    let env = FakeEnv::fixture("desktop-ddr4");
    let h = env.handle();
    let mut a = App::new(Box::new(env));
    a.run("snapshot.save", json!({"path": "/s.json", "mask": false}))
        .unwrap();
    let bytes = h.lock().unwrap().files.get("/s.json").cloned().unwrap();
    let env = FakeEnv::fixture("laptop-ddr5");
    let hb = env.handle();
    let mut b = App::new(Box::new(env));
    hb.lock().unwrap().files.insert("/s.json".into(), bytes);
    b.run("snapshot.open", json!({"path": "/s.json"})).unwrap();
    assert!(b.session.opened.is_some());
    assert!(matches!(
        b.run("live.toggle", json!({})),
        Err(EngineError::Disabled(_))
    ));
    let before = b.session.snapshot.cpu.clone();
    b.session.live = true; // even if forced on, a tick must not mix machines
    b.run("live.tick", json!({})).unwrap();
    assert_eq!(b.session.snapshot.cpu, before);
    b.run("snapshot.refresh", json!({})).unwrap();
    assert!(
        b.session.opened.is_none(),
        "refresh returns to this computer"
    );
    assert!(b.run("live.toggle", json!({})).is_ok());
}

#[test]
fn undo_after_elevate_forgets_the_dump() {
    let mut src = MemSource::default().with("/sys/class/dmi/id/board_vendor", "Acme\n");
    src.denied = vec![paths::DMI_ENTRY.into(), paths::DMI_TABLE.into()];
    let env = FakeEnv::mem(src);
    let (entry, table) = rigscope_smbios::testutil::fixture_tables("desktop-ddr4").unwrap();
    env.handle().lock().unwrap().elevate = Some(encode_dmi_dump(&DmiDump {
        entry,
        table,
        smart: Vec::new(),
    }));
    let mut a = App::new(Box::new(env));
    a.run("snapshot.elevate", json!({})).unwrap();
    a.run("snapshot.elevate_poll", json!({})).unwrap();
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::FullTable);
    a.run("snapshot.undo", json!({})).unwrap();
    assert!(a.session.dmi_dump.is_none());
    assert!(a.is_enabled("snapshot.elevate").is_ok());
}

#[test]
fn elevate_is_disabled_when_the_table_is_missing() {
    let m = MemSource::default().with("/sys/class/dmi/id/board_vendor", "Acme\n");
    let a = App::new(Box::new(FakeEnv::mem(m)));
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::NoTable);
    assert!(a.is_enabled("snapshot.elevate").is_err());
}
