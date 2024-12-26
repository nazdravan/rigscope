//! rigscope — detailed hardware configuration viewer for Linux.

mod args;
mod control;
mod dump_dmi;
mod env;

use anyhow::Result;
use rigscope_engine::App;
use serde_json::json;

/// Sample machine shown when not running on Linux and no `--fixture` is given.
#[cfg(not(target_os = "linux"))]
const DEMO_FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/desktop-ddr4");

fn main() -> Result<()> {
    let args = args::Args::parse(std::env::args().skip(1))?;
    if args.dump_dmi {
        return dump_dmi_main();
    }

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
    let control = args.control;
    eframe::run_native(
        "rigscope",
        options,
        Box::new(move |cc| {
            let ui = rigscope_ui::RigscopeUi::new(app, demo);
            let boxed: Box<dyn eframe::App> = match control {
                Some(port) => Box::new(control::Controlled::new(
                    ui,
                    control::spawn(port, cc.egui_ctx.clone())?,
                )),
                None => Box::new(ui),
            };
            Ok(boxed)
        }),
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

/// `--dump-dmi`: the only code that runs as root.
fn dump_dmi_main() -> Result<()> {
    #[cfg(target_os = "linux")]
    let result = dump_dmi::dump(&rigscope_probe::FsSource);
    #[cfg(not(target_os = "linux"))]
    let result: std::result::Result<Vec<u8>, String> = Err("--dump-dmi works only on Linux".into());
    match result {
        Ok(bytes) => {
            use std::io::Write;
            let mut out = std::io::stdout().lock();
            out.write_all(&bytes)?;
            out.flush()?;
            Ok(())
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(dump_dmi::DUMP_FAILED);
        }
    }
}
