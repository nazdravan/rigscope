#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use eframe::App as _;
use rigscope_engine::testutil::FakeEnv;
use rigscope_engine::{App, Language};
use rigscope_ui::RigscopeUi;
use serde_json::json;
use std::collections::HashMap;

#[derive(Default)]
struct MemStorage(HashMap<String, String>);

impl eframe::Storage for MemStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
    fn set_string(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
    fn remove_string(&mut self, key: &str) {
        self.0.remove(key);
    }
    fn flush(&mut self) {}
}

#[test]
fn language_survives_restart() {
    let mut storage = MemStorage::default();
    let mut ui = RigscopeUi::new(
        App::new(Box::new(FakeEnv::fixture("minimal-vm"))),
        false,
        None,
    );
    ui.app.run("view.language", json!({"id": "ro"})).unwrap();
    ui.save(&mut storage);

    let again = RigscopeUi::new(
        App::new(Box::new(FakeEnv::fixture("minimal-vm"))),
        false,
        Some(&storage),
    );
    assert_eq!(again.app.session.language, Language::Ro);
}

#[test]
fn garbage_stored_language_is_ignored() {
    let mut storage = MemStorage::default();
    storage.0.insert("language".into(), "klingon".into());
    let ui = RigscopeUi::new(
        App::new(Box::new(FakeEnv::fixture("minimal-vm"))),
        false,
        Some(&storage),
    );
    assert_eq!(ui.app.session.language, Language::En);
}
