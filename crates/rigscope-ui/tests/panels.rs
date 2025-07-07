#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use rigscope_engine::testutil::FakeEnv;
use rigscope_engine::App;
use rigscope_probe::{paths, MemSource};
use serde_json::json;

fn harness(env: FakeEnv, setup: impl FnOnce(&mut App)) -> Harness<'static, App> {
    let mut app = App::new(Box::new(env));
    setup(&mut app);
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(rigscope_ui::draw, app);
    h.run();
    h
}

fn has(h: &Harness<'_, App>, text: &str) -> bool {
    h.query_all_by_label_contains(text).next().is_some()
}

#[test]
fn memory_panel_shows_compat_line() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |_| {});
    assert!(has(&h, "DDR4 DIMM"), "compat line");
    assert!(has(&h, "2666 MT/s"));
    assert!(has(&h, "32 GiB per slot (estimated)"));
    assert!(has(&h, "KHX2666C16/8G"), "part number in slot table");
}

#[test]
fn memory_panel_shows_empty_slot() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |_| {});
    assert!(has(&h, "1 of 2 slots free"));
    assert!(has(&h, "ChannelB-DIMM0"));
    assert!(has(&h, "Empty"));
}

#[test]
fn needs_root_banner_when_partial_dmi() {
    let mut src = MemSource::default()
        .with(paths::CPUINFO, "processor\t: 0\nmodel name\t: Test CPU\n\n")
        .with("/sys/class/dmi/id/board_vendor", "Acme\n");
    src.denied = vec![paths::DMI_ENTRY.into(), paths::DMI_TABLE.into()];
    let h = harness(FakeEnv::mem(src), |_| {});
    assert!(has(&h, "Slot details need root"));
}

#[test]
fn vm_without_dmi_explains_why() {
    let h = harness(FakeEnv::fixture("minimal-vm"), |_| {});
    assert!(has(&h, "No DMI table"));
}

#[test]
fn cpu_panel_lists_cores() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.section", json!({"id": "cpu"})).unwrap();
    });
    assert!(has(&h, "i5-7500"));
    assert!(has(&h, "LGA1151"));
    assert!(has(&h, "cpu3"));
    assert!(has(&h, "3300 MHz"));
    assert!(has(&h, "6 MiB"), "L3 cache size");
}

#[test]
fn system_panel_shows_board_and_os() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.section", json!({"id": "system"})).unwrap();
    });
    assert!(has(&h, "Ubuntu 24.04.1 LTS"));
    assert!(has(&h, "B250M-D3H"));
    assert!(has(&h, "12/04/2018"));
}

#[test]
fn romanian_labels() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.language", json!({"id": "ro"})).unwrap();
    });
    assert!(has(&h, "Memorie"));
    assert!(has(&h, "Procesor"));
    assert!(has(&h, "sloturi libere"));
}

#[test]
fn clicking_sidebar_switches_section() {
    let mut h = harness(FakeEnv::fixture("desktop-ddr4"), |_| {});
    h.get_by_label("Processor").click();
    h.run();
    assert_eq!(h.state().session.section, rigscope_engine::SectionId::Cpu);
}

#[test]
fn card_titles_and_ecc_are_translated() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.language", json!({"id": "ro"})).unwrap();
    });
    assert!(has(&h, "Sloturi"));
    assert!(has(&h, "Fără"), "ECC 'None' in Romanian");
    let vm = harness(FakeEnv::fixture("minimal-vm"), |_| {});
    assert!(has(&vm, "Slot details"));
    assert!(!has(&vm, "Unknown"));
}

#[test]
fn file_menu_lists_export_commands() {
    let mut h = harness(FakeEnv::fixture("desktop-ddr4"), |_| {});
    h.get_by_label("File").click();
    h.run();
    for item in [
        "Copy report",
        "Export report…",
        "Save snapshot…",
        "Open snapshot…",
    ] {
        assert!(has(&h, item), "{item}");
    }
}

#[test]
fn copy_button_copies_compat_line() {
    let env = FakeEnv::fixture("desktop-ddr4");
    let state = env.handle();
    let mut h = harness(env, |_| {});
    h.get_by_label("Copy").click();
    h.run();
    assert_eq!(
        state.lock().unwrap().clipboard.as_deref(),
        Some("DDR4 DIMM 2666 MT/s 32GB")
    );
}

#[test]
fn file_menu_save_uses_picked_path() {
    let env = FakeEnv::fixture("desktop-ddr4");
    let state = env.handle();
    let app = App::new(Box::new(env));
    let picker = |id: &str| (id == "snapshot.save").then(|| json!({"path": "/picked.json"}));
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(
            move |ui, app: &mut App| rigscope_ui::draw_with(ui, app, &picker),
            app,
        );
    h.run();
    h.get_by_label("File").click();
    h.run();
    h.get_by_label_contains("Save snapshot").click();
    h.run();
    assert!(state.lock().unwrap().files.contains_key("/picked.json"));
}

#[test]
fn ui_collects_elevation_result_while_waiting() {
    let mut src = MemSource::default().with("/sys/class/dmi/id/board_vendor", "Acme\n");
    src.denied = vec![paths::DMI_ENTRY.into(), paths::DMI_TABLE.into()];
    let env = FakeEnv::mem(src);
    {
        let (entry, table) = rigscope_smbios::testutil::fixture_tables("desktop-ddr4").unwrap();
        let h = env.handle();
        let mut st = h.lock().unwrap();
        st.elevate = Some(rigscope_probe::encode_dmi_dump(&rigscope_probe::DmiDump {
            entry,
            table,
            smart: Vec::new(),
        }));
        st.pending_polls = 1;
    }
    let mut h = harness(env, |a| {
        a.run("snapshot.elevate", json!({})).unwrap();
    });
    for _ in 0..5 {
        h.step();
    }
    assert!(!h.state().session.elevating);
    assert_eq!(
        h.state().session.snapshot.dmi_source,
        rigscope_model::DmiSource::FullTable
    );
}

#[test]
fn storage_panel_lists_disks() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.section", json!({"id": "storage"})).unwrap();
    });
    assert!(has(&h, "Samsung SSD 970 EVO Plus 500GB"));
    assert!(has(&h, "465.8 GiB (500.1 GB)"));
    assert!(has(&h, "NVMe SSD"));
    assert!(has(&h, "WDC WD10EZEX-08W"));
    assert!(has(&h, "HDD"));
    assert!(has(&h, "not exposed by the kernel without root"));
}

#[test]
fn storage_sidebar_in_romanian() {
    let h = harness(FakeEnv::fixture("laptop-ddr5"), |a| {
        a.run("view.language", json!({"id": "ro"})).unwrap();
        a.run("view.section", json!({"id": "storage"})).unwrap();
    });
    assert!(has(&h, "Stocare"));
    assert!(has(&h, "Detașabil"));
    assert!(has(&h, "nu e expus de kernel fără root"));
    assert!(!has(&h, "not exposed by the kernel"));
}

#[test]
fn gpu_panel_shows_card() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.section", json!({"id": "gpu"})).unwrap();
    });
    assert!(has(&h, "GA104 [GeForce RTX 3070]"));
    assert!(has(&h, "TUF RTX 3070 OC"));
    assert!(has(&h, "10de:2484"));
    assert!(has(&h, "nvidia"));
    assert!(has(&h, "16.0 GT/s PCIe x16"));
    assert!(has(&h, "not exposed by the nvidia driver"));
}

#[test]
fn gpu_panel_romanian_and_vram() {
    let h = harness(FakeEnv::fixture("laptop-ddr5"), |a| {
        a.run("view.language", json!({"id": "ro"})).unwrap();
        a.run("view.section", json!({"id": "gpu"})).unwrap();
    });
    assert!(has(&h, "Placă video"));
    assert!(has(&h, "512 MiB"));
    assert!(has(&h, "da"), "primary display: yes");
}

#[test]
fn vm_gpu_panel_says_none_found() {
    let h = harness(FakeEnv::fixture("minimal-vm"), |a| {
        a.run("view.section", json!({"id": "gpu"})).unwrap();
    });
    assert!(has(&h, "No graphics cards found"));
}

#[test]
fn network_panel_shows_interfaces() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.section", json!({"id": "network"})).unwrap();
    });
    assert!(has(&h, "enp4s0"));
    assert!(has(&h, "I211 Gigabit Network Connection"));
    assert!(has(&h, "1 Gb/s"));
    assert!(has(&h, "igb"));
    assert!(has(&h, "1 virtual interface not listed"));
    assert!(!has(&h, "docker0"));
}

#[test]
fn network_panel_wifi_in_romanian() {
    let h = harness(FakeEnv::fixture("laptop-ddr5"), |a| {
        a.run("view.language", json!({"id": "ro"})).unwrap();
        a.run("view.section", json!({"id": "network"})).unwrap();
    });
    assert!(has(&h, "Rețea"));
    assert!(has(&h, "Wi-Fi"));
    assert!(has(&h, "nu e raportată pentru Wi-Fi"));
}

#[test]
fn battery_panel_shows_health() {
    let h = harness(FakeEnv::fixture("laptop-ddr5"), |a| {
        a.run("view.section", json!({"id": "battery"})).unwrap();
    });
    assert!(has(&h, "BAT0"));
    assert!(has(&h, "92.0 % (derived)"));
    assert!(has(&h, "52.4 Wh"));
    assert!(has(&h, "57.0 Wh"));
    assert!(has(&h, "87"));
    assert!(has(&h, "Discharging"));
}

#[test]
fn desktop_battery_panel_says_none() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.language", json!({"id": "ro"})).unwrap();
        a.run("view.section", json!({"id": "battery"})).unwrap();
    });
    assert!(has(&h, "Baterie"));
    assert!(has(&h, "Nu există baterie"));
}

fn ro(fixture: &str, section: &str) -> Harness<'static, App> {
    harness(FakeEnv::fixture(fixture), |a| {
        a.run("view.language", json!({"id": "ro"})).unwrap();
        a.run("view.section", json!({"id": section})).unwrap();
    })
}

#[test]
fn kernel_values_are_translated_in_romanian() {
    let bat = ro("laptop-ddr5", "battery");
    assert!(has(&bat, "Se descarcă"));
    assert!(!has(&bat, "Discharging"));
    let net = ro("desktop-ddr4", "network");
    assert!(has(&net, "activă"));
    assert!(has(&net, "complet"));
    let cpu = ro("desktop-ddr4", "cpu");
    assert!(has(&cpu, "10 extensii"));
}

#[test]
fn status_messages_are_translated_in_romanian() {
    let mut h = ro("desktop-ddr4", "memory");
    h.state_mut().run("memory.copy_compat", json!({})).unwrap();
    h.run();
    assert!(has(&h, "copiat: DDR4 DIMM 2666 MT/s 32GB"));
    let _ = h.state_mut().run("snapshot.undo", json!({}));
    h.run();
    assert!(has(&h, "comandă indisponibilă: nimic de anulat"));
}

#[test]
fn sensors_panel_shows_temperatures_and_fans() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.section", json!({"id": "sensors"})).unwrap();
    });
    assert!(has(&h, "CPU (Intel)"));
    assert!(has(&h, "Package id 0"));
    assert!(has(&h, "45.0 °C"));
    assert!(has(&h, "max 80.0 °C"));
    assert!(has(&h, "1180 RPM"));
    assert!(has(&h, "NVMe disk"));
}

#[test]
fn sensors_panel_romanian_and_empty() {
    let h = ro("minimal-vm", "sensors");
    assert!(has(&h, "Senzori"));
    assert!(has(&h, "Nu s-au găsit senzori"));
}

#[test]
fn storage_panel_offers_smart_then_shows_it() {
    let env = FakeEnv::fixture("desktop-ddr4");
    {
        let nvme = std::fs::read(format!(
            "{}/../../fixtures/smart/nvme-healthy.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let sda = std::fs::read(format!(
            "{}/../../fixtures/smart/sata-hdd-reallocated.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let (entry, table) = rigscope_smbios::testutil::fixture_tables("desktop-ddr4").unwrap();
        env.handle().lock().unwrap().elevate =
            Some(rigscope_probe::encode_dmi_dump(&rigscope_probe::DmiDump {
                entry,
                table,
                smart: vec![("nvme0n1".into(), nvme), ("sda".into(), sda)],
            }));
    }
    let mut h = harness(env, |a| {
        a.run("view.section", json!({"id": "storage"})).unwrap();
    });
    assert!(has(&h, "Disk health (SMART) needs root"));
    h.get_by_label("Read with elevated rights").click();
    for _ in 0..5 {
        h.step();
    }
    assert!(has(&h, "SMART passed"));
    assert!(has(&h, "4321 h"));
    assert!(has(&h, "10.0 TB"));
    assert!(has(&h, "reallocated sectors"));
    assert!(!has(&h, "Disk health (SMART) needs root"));
}

#[test]
fn smart_summary_in_romanian() {
    use rigscope_model::DiskHealth;
    let h = DiskHealth {
        passed: Some(true),
        power_on_hours: Some(4321),
        reallocated_sectors: Some(8),
        ..Default::default()
    };
    assert_eq!(
        rigscope_ui::i18n::health_line(rigscope_engine::Language::Ro, &h),
        "SMART: OK, 4321 h de funcționare, 8 sectoare realocate"
    );
    assert_eq!(
        rigscope_ui::i18n::warning(rigscope_engine::Language::Ro, "reallocated sectors"),
        "sectoare realocate"
    );
}

#[test]
fn opened_snapshot_banner_stays_until_refresh() {
    let env = FakeEnv::fixture("desktop-ddr4");
    let mut app = App::new(Box::new(env));
    app.run("snapshot.save", json!({"path": "/s.json"}))
        .unwrap();
    app.run("snapshot.open", json!({"path": "/s.json"}))
        .unwrap();
    // Another action replaces the status line; the banner must remain.
    app.run("view.section", json!({"id": "cpu"})).unwrap();
    let mut h = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui_state(rigscope_ui::draw, app);
    h.run();
    assert!(has(&h, "Showing the saved snapshot /s.json"));
    h.state_mut().run("snapshot.refresh", json!({})).unwrap();
    h.run();
    assert!(!has(&h, "Showing the saved snapshot"));
}

#[test]
fn needs_elevation_wording_follows_platform() {
    use rigscope_model::{Field, Platform};
    for (platform, wanted, other) in [
        (Platform::Linux, "Needs root", "Needs administrator"),
        (Platform::Windows, "Needs administrator", "Needs root"),
    ] {
        let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
            a.run("view.section", json!({"id": "cpu"})).unwrap();
            a.session.snapshot.cpu.socket = Field::NeedsElevation;
            a.session.snapshot.platform = platform;
        });
        assert!(has(&h, wanted), "{platform:?} wants {wanted}");
        assert!(!has(&h, other), "{platform:?} must not say {other}");
    }
}

#[test]
fn estimated_frequency_is_marked() {
    let h = harness(FakeEnv::fixture("desktop-ddr4"), |a| {
        a.run("view.section", json!({"id": "cpu"})).unwrap();
        for c in &mut a.session.snapshot.cpu.cores {
            c.estimated = true;
        }
    });
    assert!(has(&h, "estimated: % processor performance"));
}

fn windows_env(w: rigscope_probe_win::MemWin) -> FakeEnv {
    FakeEnv::with(Box::new(rigscope_probe_win::WinCollector::new(w)))
}

#[test]
fn windows_memory_panel_shows_slots_without_no_dmi_card() {
    let h = harness(windows_env(rigscope_probe_win::samples::laptop()), |_| {});
    assert!(
        !has(&h, "No DMI table"),
        "a Windows firmware read is not 'no DMI'"
    );
    assert!(!has(&h, "firmware table lists no memory slots"));
    assert!(has(&h, "DIMM 0"), "slot is shown");
    assert!(has(&h, "M425R4GA3BB0-CWMOL"), "part number in slot table");
}

#[test]
fn windows_firmware_without_slots_says_so() {
    let h = harness(windows_env(rigscope_probe_win::samples::vm()), |_| {});
    assert!(!has(&h, "No DMI table"));
    assert!(has(&h, "The firmware table lists no memory slots"));
}

#[test]
fn windows_smart_without_admin_has_banner_but_no_elevate_button() {
    let mut w = rigscope_probe_win::samples::laptop();
    w.elevated = false;
    for v in w.nvme.values_mut() {
        *v = Err(rigscope_win::WinError::AccessDenied);
    }
    let h = harness(windows_env(w), |a| {
        a.run("view.section", json!({"id": "storage"})).unwrap();
    });
    assert!(
        a_disk_needs_elevation(&h),
        "fixture must exercise NeedsElevation"
    );
    assert!(has(&h, "Disk health (SMART) needs administrator rights"));
    assert!(
        !has(&h, "Read with elevated rights"),
        "no elevate button on Windows"
    );
}

fn a_disk_needs_elevation(h: &Harness<'_, App>) -> bool {
    h.state()
        .session
        .snapshot
        .storage
        .disks
        .iter()
        .any(|d| d.health == rigscope_model::Field::NeedsElevation)
}
