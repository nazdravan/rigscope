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

fn denied_env() -> FakeEnv {
    let mut src = MemSource::default()
        .with(paths::CPUINFO, "processor\t: 0\nmodel name\t: Test CPU\n\n")
        .with("/sys/class/dmi/id/board_vendor", "Acme\n");
    src.denied = vec![paths::DMI_ENTRY.into(), paths::DMI_TABLE.into()];
    FakeEnv::mem(src)
}

/// Starts elevation and polls until it settles (FakeEnv answers after `pending_polls` polls).
fn elevate(a: &mut App) -> Result<serde_json::Value, EngineError> {
    a.run("snapshot.elevate", json!({}))?;
    for _ in 0..10 {
        let r = a.run("snapshot.elevate_poll", json!({}))?;
        if !a.session.elevating {
            return Ok(r);
        }
    }
    panic!("elevation never finished");
}

fn desktop_dump() -> Vec<u8> {
    let (entry, table) = rigscope_smbios::testutil::fixture_tables("desktop-ddr4").unwrap();
    encode_dmi_dump(&DmiDump {
        entry,
        table,
        smart: Vec::new(),
    })
}

#[test]
fn elevate_success_fills_slots_and_survives_refresh() {
    let env = denied_env();
    env.handle().lock().unwrap().elevate = Some(desktop_dump());
    let mut a = App::new(Box::new(env));
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::SysfsIdOnly);
    elevate(&mut a).unwrap();
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
    assert!(matches!(elevate(&mut a), Err(EngineError::Io(_))));
    assert_eq!(a.session.snapshot, before);
    assert!(a.session.history.is_empty());
    assert!(
        a.session
            .status
            .as_deref()
            .unwrap_or_default()
            .contains("cancelled")
    );
}

#[test]
fn elevate_garbage_dump_is_error() {
    let env = denied_env();
    env.handle().lock().unwrap().elevate = Some(b"junk".to_vec());
    let mut a = App::new(Box::new(env));
    let before = a.session.snapshot.clone();
    assert!(elevate(&mut a).is_err());
    assert_eq!(a.session.snapshot, before);
    assert!(a.session.dmi_dump.is_none());
}

#[test]
fn elevate_with_undecodable_table_is_error() {
    let env = denied_env();
    let dump = encode_dmi_dump(&DmiDump {
        entry: b"nope".to_vec(),
        table: vec![1, 2, 3],
        smart: Vec::new(),
    });
    env.handle().lock().unwrap().elevate = Some(dump);
    let mut a = App::new(Box::new(env));
    assert!(elevate(&mut a).is_err());
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::SysfsIdOnly);
}

#[test]
fn elevate_disabled_when_table_full_and_no_disk_needs_root() {
    let (entry, table) = rigscope_smbios::testutil::fixture_tables("desktop-ddr4").unwrap();
    let m = MemSource::default()
        .with(paths::DMI_ENTRY, entry)
        .with(paths::DMI_TABLE, table);
    let mut a = App::new(Box::new(FakeEnv::mem(m)));
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::FullTable);
    assert!(matches!(
        a.run("snapshot.elevate", json!({})),
        Err(EngineError::Disabled(_))
    ));
}

#[test]
fn elevate_reads_smart_even_when_dmi_is_full() {
    let env = FakeEnv::fixture("desktop-ddr4");
    let nvme = std::fs::read(format!(
        "{}/../../fixtures/smart/nvme-healthy.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let (entry, table) = rigscope_smbios::testutil::fixture_tables("desktop-ddr4").unwrap();
    env.handle().lock().unwrap().elevate = Some(encode_dmi_dump(&DmiDump {
        entry,
        table,
        smart: vec![("nvme0n1".into(), nvme)],
    }));
    let mut a = App::new(Box::new(env));
    assert!(
        a.is_enabled("snapshot.elevate").is_ok(),
        "disks still need root for SMART"
    );
    elevate(&mut a).unwrap();
    let d = a
        .session
        .snapshot
        .storage
        .disks
        .iter()
        .find(|d| d.name == "nvme0n1")
        .unwrap();
    assert_eq!(d.health.known().and_then(|h| h.power_on_hours), Some(4321));
}

#[test]
fn elevate_with_smart_only_succeeds_without_a_dmi_table() {
    let mut src = MemSource::default()
        .with("/sys/block/nvme0n1/size", "1000\n")
        .with("/sys/class/dmi/id/board_vendor", "Acme\n");
    src.denied = vec![paths::DMI_ENTRY.into(), paths::DMI_TABLE.into()];
    let env = FakeEnv::mem(src);
    let nvme = std::fs::read(format!(
        "{}/../../fixtures/smart/nvme-healthy.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    env.handle().lock().unwrap().elevate = Some(encode_dmi_dump(&DmiDump {
        entry: vec![],
        table: vec![],
        smart: vec![("nvme0n1".into(), nvme)],
    }));
    let mut a = App::new(Box::new(env));
    elevate(&mut a).unwrap();
    assert!(a.session.snapshot.storage.disks[0].health.known().is_some());
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

#[test]
fn elevate_runs_in_background_until_helper_finishes() {
    let env = denied_env();
    {
        let h = env.handle();
        let mut st = h.lock().unwrap();
        st.elevate = Some(desktop_dump());
        st.pending_polls = 2;
    }
    let mut a = App::new(Box::new(env));
    a.run("snapshot.elevate", json!({})).unwrap();
    assert!(a.session.elevating);
    assert!(
        matches!(
            a.run("snapshot.elevate", json!({})),
            Err(EngineError::Disabled(_))
        ),
        "no second helper"
    );
    a.run("snapshot.elevate_poll", json!({})).unwrap();
    a.run("snapshot.elevate_poll", json!({})).unwrap();
    assert!(a.session.elevating, "still waiting for authentication");
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::SysfsIdOnly);
    a.run("snapshot.elevate_poll", json!({})).unwrap();
    assert!(!a.session.elevating);
    assert_eq!(a.session.snapshot.dmi_source, DmiSource::FullTable);
}

#[test]
fn elevate_poll_without_job_is_noop() {
    let mut a = App::new(Box::new(denied_env()));
    a.run("snapshot.elevate_poll", json!({})).unwrap();
    assert!(a.session.history.is_empty());
}
