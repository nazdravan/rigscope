#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_engine::testutil::FakeEnv;
use rigscope_engine::*;
use serde_json::json;

fn app() -> App {
    App::new(Box::new(FakeEnv::fixture("desktop-ddr4")))
}

#[test]
fn new_app_has_snapshot_and_builtin_commands() {
    let a = app();
    assert_eq!(a.session.snapshot.memory.arrays.len(), 1);
    let ids: Vec<&str> = a.specs().iter().map(|s| s.id).collect();
    for id in [
        "snapshot.refresh",
        "snapshot.undo",
        "view.section",
        "live.toggle",
        "live.tick",
    ] {
        assert!(ids.contains(&id), "missing {id}");
    }
    let undo = a.specs().iter().find(|s| s.id == "snapshot.undo").unwrap();
    assert_eq!(undo.shortcut, Some("Ctrl+Z"));
}

#[test]
fn refresh_pushes_history() {
    let mut a = app();
    a.run("snapshot.refresh", json!({})).unwrap();
    assert_eq!(a.session.history.len(), 1);
}

#[test]
fn undo_restores_previous() {
    let env = FakeEnv::fixture("desktop-ddr4");
    let h = env.handle();
    let mut a = App::new(Box::new(env));
    let first = a.session.snapshot.clone();
    h.lock().unwrap().now = 99;
    a.run("snapshot.refresh", json!({})).unwrap();
    assert_eq!(a.session.snapshot.taken_at_unix, 99);
    a.run("snapshot.undo", json!({})).unwrap();
    assert_eq!(a.session.snapshot, first);
    assert!(a.session.history.is_empty());
}

#[test]
fn undo_with_empty_history_is_disabled() {
    let mut a = app();
    assert!(matches!(
        a.run("snapshot.undo", json!({})),
        Err(EngineError::Disabled(_))
    ));
    assert_eq!(
        a.session.status.as_deref(),
        Some("command not available: nothing to undo")
    );
}

#[test]
fn history_capped_at_20() {
    let mut a = app();
    for _ in 0..25 {
        a.run("snapshot.refresh", json!({})).unwrap();
    }
    assert_eq!(a.session.history.len(), HISTORY_LIMIT);
}

#[test]
fn unknown_command_errors() {
    let mut a = app();
    assert_eq!(
        a.run("nope", json!({})),
        Err(EngineError::UnknownCommand("nope".into()))
    );
}

fn boom(s: &mut Session, _: &serde_json::Value) -> Result<serde_json::Value, EngineError> {
    s.snapshot.taken_at_unix = 12345;
    s.history.clear();
    let v: Vec<u8> = Vec::new();
    #[allow(clippy::indexing_slicing)]
    let _ = v[3];
    Ok(json!(null))
}

#[test]
fn panicking_command_rolls_back() {
    let mut a = app();
    a.run("snapshot.refresh", json!({})).unwrap();
    let before = a.session.snapshot.clone();
    a.register(CommandSpec {
        id: "test.boom",
        label_key: "boom",
        menu: "",
        shortcut: None,
        params_doc: "",
        enabled: |_| Ok(()),
        run: boom,
    });
    assert_eq!(a.run("test.boom", json!({})), Err(EngineError::Panicked));
    assert_eq!(a.session.snapshot, before);
    assert_eq!(a.session.history.len(), 1);
    assert!(a.session.status.is_some());
}

#[test]
fn view_section_switches_and_validates() {
    let mut a = app();
    a.run("view.section", json!({"id": "cpu"})).unwrap();
    assert_eq!(a.session.section, SectionId::Cpu);
    a.run("view.section", json!({"id": "storage"})).unwrap();
    assert_eq!(a.session.section, SectionId::Storage);
    a.run("view.section", json!({"id": "gpu"})).unwrap();
    assert_eq!(a.session.section, SectionId::Gpu);
    a.run("view.section", json!({"id": "network"})).unwrap();
    assert_eq!(a.session.section, SectionId::Network);
    a.run("view.section", json!({"id": "battery"})).unwrap();
    assert_eq!(a.session.section, SectionId::Battery);
    a.run("view.section", json!({"id": "sensors"})).unwrap();
    assert_eq!(a.session.section, SectionId::Sensors);
    a.run("view.section", json!({"id": "cpu"})).unwrap();
    assert!(matches!(
        a.run("view.section", json!({"id": "toaster"})),
        Err(EngineError::BadParams(_))
    ));
    assert!(matches!(
        a.run("view.section", json!({})),
        Err(EngineError::BadParams(_))
    ));
    assert_eq!(a.session.section, SectionId::Cpu);
}

#[test]
fn live_toggle_and_tick() {
    let mut a = app();
    assert!(a.session.live);
    a.run("live.toggle", json!({})).unwrap();
    assert!(!a.session.live);
    a.run("live.tick", json!({})).unwrap();
    assert!(a.session.history.is_empty(), "tick must not create history");
}

#[test]
fn view_language_switches() {
    let mut a = app();
    assert_eq!(a.session.language, Language::En);
    a.run("view.language", json!({"id": "ro"})).unwrap();
    assert_eq!(a.session.language, Language::Ro);
    assert!(matches!(
        a.run("view.language", json!({"id": "fr"})),
        Err(EngineError::BadParams(_))
    ));
}

#[test]
fn engine_runs_on_the_windows_collector() {
    let env = FakeEnv::with(Box::new(rigscope_probe_win::WinCollector::new(
        rigscope_probe_win::samples::laptop(),
    )));
    let mut a = App::new(Box::new(env));
    assert_eq!(
        a.session.snapshot.platform,
        rigscope_model::Platform::Windows
    );
    a.run("live.tick", json!({})).unwrap();
    assert!(a.session.snapshot.cpu.cores.iter().all(|c| c.estimated));
}
