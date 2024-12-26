#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_engine::testutil::FakeEnv;
use rigscope_engine::*;
use rigscope_model::DmiSource;
use rigscope_probe::{encode_dmi_dump, paths, DmiDump, MemSource};
use serde_json::json;

fn denied_env() -> FakeEnv {
    let mut src = MemSource::default()
        .with(paths::CPUINFO, "processor\t: 0\nmodel name\t: Test CPU\n\n")
        .with("/sys/class/dmi/id/board_vendor", "Acme\n");
    src.denied = vec![paths::DMI_ENTRY.into(), paths::DMI_TABLE.into()];
    FakeEnv::mem(src)
}

fn desktop_dump() -> Vec<u8> {
    let (entry, table) = rigscope_smbios::testutil::fixture_tables("desktop-ddr4").unwrap();
    encode_dmi_dump(&DmiDump { entry, table })
}

#[test]
fn elevate_success_fills_slots_and_survives_refresh() {
    let env = denied_env();
    env.handle().lock().unwrap().elevate = Some(desktop_dump());
    let mut a = App::new(Box::new(env));
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::SysfsIdOnly);
    a.run("snapshot.elevate", json!({})).unwrap();
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::FullTable);
    assert_eq!(a.session.snapshot.memory.arrays[0].slots.len(), 2);
    assert_eq!(a.session.history.len(), 1);
    a.run("snapshot.refresh", json!({})).unwrap();
    assert_eq!(
        a.session.snapshot.dmi_source,
        DmiSource::FullTable,
        "refresh keeps elevated data"
    );
}

#[test]
fn elevate_cancelled_keeps_snapshot() {
    let mut a = App::new(Box::new(denied_env()));
    let before = a.session.snapshot.clone();
    assert!(matches!(
        a.run("snapshot.elevate", json!({})),
        Err(EngineError::Io(_))
    ));
    assert_eq!(a.session.snapshot, before);
    assert!(a.session.history.is_empty());
    assert!(a
        .session
        .status
        .as_deref()
        .unwrap_or_default()
        .contains("cancelled"));
}

#[test]
fn elevate_garbage_dump_is_error() {
    let env = denied_env();
    env.handle().lock().unwrap().elevate = Some(b"junk".to_vec());
    let mut a = App::new(Box::new(env));
    let before = a.session.snapshot.clone();
    assert!(a.run("snapshot.elevate", json!({})).is_err());
    assert_eq!(a.session.snapshot, before);
    assert!(a.session.dmi_dump.is_none());
}

#[test]
fn elevate_with_undecodable_table_is_error() {
    let env = denied_env();
    let dump = encode_dmi_dump(&DmiDump {
        entry: b"nope".to_vec(),
        table: vec![1, 2, 3],
    });
    env.handle().lock().unwrap().elevate = Some(dump);
    let mut a = App::new(Box::new(env));
    assert!(a.run("snapshot.elevate", json!({})).is_err());
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::SysfsIdOnly);
}

#[test]
fn elevate_disabled_when_table_already_full() {
    let mut a = App::new(Box::new(FakeEnv::fixture("desktop-ddr4")));
    assert!(matches!(
        a.run("snapshot.elevate", json!({})),
        Err(EngineError::Disabled(_))
    ));
}

#[test]
fn elevate_disabled_when_platform_cannot() {
    let mut env = denied_env();
    env.can_elevate = false;
    let mut a = App::new(Box::new(env));
    assert!(matches!(
        a.run("snapshot.elevate", json!({})),
        Err(EngineError::Disabled(_))
    ));
}
