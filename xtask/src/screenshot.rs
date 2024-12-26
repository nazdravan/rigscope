//! `cargo xtask screenshot`: runs rigscope under Xvfb in the Linux image, drives it through the
//! control channel and saves a PNG into the repository (default `target/shots/`).

use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::Command;
use std::time::{Duration, Instant};

const PORT: u16 = 7878;

#[derive(Debug, Default)]
pub struct ShotArgs {
    pub fixture: String,
    pub section: String,
    pub lang: String,
    /// Repository-relative output path.
    pub out: String,
}

impl ShotArgs {
    pub fn parse(args: &[String]) -> Result<Self> {
        let mut a = ShotArgs {
            fixture: "fixtures/desktop-ddr4".into(),
            section: "memory".into(),
            lang: "en".into(),
            out: String::new(),
        };
        let mut it = args.iter();
        while let Some(flag) = it.next() {
            let v = it
                .next()
                .with_context(|| format!("{flag} needs a value"))?
                .clone();
            match flag.as_str() {
                "--fixture" => a.fixture = v,
                "--section" => a.section = v,
                "--lang" => a.lang = v,
                "--out" => a.out = v,
                other => bail!("unknown flag {other}"),
            }
        }
        if a.out.is_empty() {
            a.out = format!("target/shots/{}-{}.png", a.section, a.lang);
        }
        Ok(a)
    }

    fn to_args(&self) -> Vec<String> {
        [
            "--fixture",
            &self.fixture,
            "--section",
            &self.section,
            "--lang",
            &self.lang,
            "--out",
            &self.out,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
}

/// Host side: run the inner step inside the Linux image.
pub fn screenshot(args: &[String]) -> Result<()> {
    let a = ShotArgs::parse(args)?;
    let mut inner = vec![
        "cargo".to_string(),
        "run".into(),
        "-q".into(),
        "-p".into(),
        "xtask".into(),
        "--".into(),
    ];
    inner.push("screenshot-inner".into());
    inner.extend(a.to_args());
    crate::linux(&inner)?;
    eprintln!("screenshot: {}", a.out);
    Ok(())
}

/// Container side: build, start under Xvfb, drive, save.
pub fn inner(args: &[String]) -> Result<()> {
    let a = ShotArgs::parse(args)?;
    if let Some(dir) = std::path::Path::new(&a.out).parent() {
        std::fs::create_dir_all(crate::root().join(dir))?;
    }
    crate::run(crate::cargo().args(["build", "-q", "-p", "rigscope"]))?;
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".into());
    let bin = format!("{target}/debug/rigscope");
    let mut child = Command::new("xvfb-run")
        .args([
            "-a",
            "-s",
            "-screen 0 1280x900x24",
            &bin,
            "--fixture",
            &a.fixture,
            "--control",
            &PORT.to_string(),
        ])
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .current_dir(crate::root())
        .spawn()
        .context("xvfb-run")?;

    let result = drive(&a);
    if result.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}

fn drive(a: &ShotArgs) -> Result<()> {
    let start = Instant::now();
    let stream = loop {
        match TcpStream::connect(("127.0.0.1", PORT)) {
            Ok(s) => break s,
            Err(_) if start.elapsed() < Duration::from_secs(60) => {
                std::thread::sleep(Duration::from_millis(250))
            }
            Err(e) => bail!("control channel did not come up: {e}"),
        }
    };
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    let mut send = |req: serde_json::Value| -> Result<serde_json::Value> {
        writeln!(writer, "{req}")?;
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let v: serde_json::Value = serde_json::from_str(&line)?;
        if let Some(e) = v.get("error") {
            bail!("{req} → {e}");
        }
        Ok(v)
    };
    let path = crate::root().join(&a.out);
    send(
        serde_json::json!({"method": "run", "command": "view.language", "params": {"id": a.lang}}),
    )?;
    send(
        serde_json::json!({"method": "run", "command": "view.section", "params": {"id": a.section}}),
    )?;
    let shot =
        send(serde_json::json!({"method": "ui.screenshot", "path": path.display().to_string()}))?;
    eprintln!("{shot}");
    send(serde_json::json!({"method": "quit"}))?;
    Ok(())
}
