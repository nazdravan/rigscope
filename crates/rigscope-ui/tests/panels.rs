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
