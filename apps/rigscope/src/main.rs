//! rigscope — detailed hardware configuration viewer for Linux.

mod args;
mod env;

use anyhow::Result;
use rigscope_engine::App;
use serde_json::json;

/// Sample machine shown when not running on Linux and no `--fixture` is given.
const DEMO_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/desktop-ddr4");

fn main() -> Result<()> {
    let args = args::Args::parse(std::env::args().skip(1))?;

    let (env, demo) = match &args.fixture {
        Some(dir) => (env::RealEnv::fixture(dir), true),
        None => default_env(),
    };
    let mut app = App::new(Box::new(env));
    if let Some(lang) = &args.lang {
        app.run("view.language", json!({ "id": lang }))
            .map_err(|e| anyhow::anyhow!("--lang: {e}"))?;
    }
    if let Some(section) = &args.section {
        app.run("view.section", json!({ "id": section }))
            .map_err(|e| anyhow::anyhow!("--section: {e}"))?;
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_title("rigscope"),
        ..Default::default()
    };
    eframe::run_native(
        "rigscope",
        options,
        Box::new(move |_cc| Ok(Box::new(rigscope_ui::RigscopeUi::new(app, demo)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

#[cfg(target_os = "linux")]
fn default_env() -> (env::RealEnv, bool) {
    (env::RealEnv::system(), false)
}

#[cfg(not(target_os = "linux"))]
fn default_env() -> (env::RealEnv, bool) {
    (env::RealEnv::fixture(DEMO_FIXTURE), true)
}
