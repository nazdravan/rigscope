//! Project automation: `cargo xtask <ci|layers|unsafe|wasm|linux>`.

mod layers;
mod screenshot;
mod unsafe_scope;

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const WASM_CRATES: &[&str] = &[
    "rigscope-model",
    "rigscope-smbios",
    "rigscope-procfs",
    "rigscope-probe",
    "rigscope-engine",
];
const LINUX_IMAGE: &str = "rigscope-linux-test";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or("help");
    let rest = args.get(1..).unwrap_or(&[]);
    match task {
        "ci" => ci(),
        "layers" => layers_check(),
        "unsafe" => unsafe_scope_check(),
        "wasm" => wasm(),
        "linux" => linux(rest),
        "gen-fixtures" => gen_fixtures(),
        "icons" => run(Command::new("python3").args(["-I", "assets/icon.py"])),
        "screenshot" => screenshot::screenshot(rest),
        "screenshot-inner" => screenshot::inner(rest),
        "shots" => screenshot::readme_shots(),
        "banner" => run(Command::new("python3").args(["-I", "assets/banner.py"])),
        _ => {
            eprintln!("usage: cargo xtask <ci|layers|unsafe|wasm|linux [cmd...]|gen-fixtures|icons|banner|shots|screenshot [--fixture D --section S --lang L --out P]>");
            Ok(())
        }
    }
}

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn run(cmd: &mut Command) -> Result<()> {
    eprintln!("$ {cmd:?}");
    let status = cmd
        .current_dir(root())
        .status()
        .with_context(|| format!("{cmd:?}"))?;
    if !status.success() {
        bail!("command failed ({status}): {cmd:?}");
    }
    Ok(())
}

pub(crate) fn cargo() -> Command {
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
}

fn ci() -> Result<()> {
    run(cargo().args(["fmt", "--all", "--check"]))?;
    run(cargo().args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ]))?;
    run(cargo().args(["test", "--workspace", "--quiet"]))?;
    layers_check()?;
    unsafe_scope_check()?;
    wasm()?;
    eprintln!("xtask ci: OK");
    Ok(())
}

fn layers_check() -> Result<()> {
    let out = cargo()
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(root())
        .output()?;
    if !out.status.success() {
        bail!("cargo metadata failed");
    }
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let mut violations = Vec::new();
    for pkg in meta
        .get("packages")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
    {
        let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or_default();
        if name == "xtask" {
            continue;
        }
        let deps: Vec<&str> = pkg
            .get("dependencies")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter(|d| d.get("kind").and_then(|v| v.as_str()) != Some("dev"))
            .filter_map(|d| d.get("name").and_then(|v| v.as_str()))
            .collect();
        violations.extend(layers::check(name, &deps));
    }
    if !violations.is_empty() {
        for v in &violations {
            eprintln!("layer violation: {v}");
        }
        bail!("{} layer violation(s)", violations.len());
    }
    eprintln!("layers: OK");
    Ok(())
}

/// The wasm check needs a toolchain that ships the wasm32 std. A Homebrew `cargo` earlier in
/// PATH does not, so use the rustup-managed binaries (by absolute path) when rustup is installed.
fn wasm() -> Result<()> {
    let mut cmd = match (rustup_which("cargo"), rustup_which("rustc")) {
        (Some(c), Some(r)) => {
            let mut c = Command::new(c);
            c.env("RUSTC", r);
            c
        }
        _ => cargo(),
    };
    cmd.args(["check", "--quiet", "--target", "wasm32-unknown-unknown"]);
    for c in WASM_CRATES {
        cmd.args(["-p", c]);
    }
    run(&mut cmd)?;
    eprintln!("wasm: OK");
    Ok(())
}

/// Builds the Linux test image and runs a command in it (default: the workspace tests).
pub(crate) fn linux(cmd: &[String]) -> Result<()> {
    let root = root();
    run(Command::new("docker").args([
        "build",
        "-q",
        "-t",
        LINUX_IMAGE,
        "-f",
        "docker/linux-test.Dockerfile",
        "docker",
    ]))?;
    let mount = format!("{}:/src", root.display());
    let mut docker = Command::new("docker");
    docker.args([
        "run",
        "--rm",
        "-v",
        &mount,
        "-v",
        "rigscope-target:/target",
        "-v",
        "rigscope-cargo:/usr/local/cargo/registry",
        "-w",
        "/src",
        LINUX_IMAGE,
    ]);
    if cmd.is_empty() {
        // Linux-only code (`cfg(target_os = "linux")`) is invisible to clippy on macOS.
        docker.args([
            "bash",
            "-c",
            "cargo clippy -q --workspace --all-targets -- -D warnings && cargo test --workspace --quiet",
        ]);
    } else {
        docker.args(cmd);
    }
    run(&mut docker)
}

fn rustup_which(tool: &str) -> Option<String> {
    let out = Command::new("rustup").args(["which", tool]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!path.is_empty()).then_some(path)
}

/// Writes the synthetic DMI tables into `fixtures/<name>/sys/firmware/dmi/tables/`.
fn gen_fixtures() -> Result<()> {
    for name in rigscope_smbios::testutil::DMI_FIXTURES {
        let Some((entry, table)) = rigscope_smbios::testutil::fixture_tables(name) else {
            bail!("no tables for fixture {name}");
        };
        let dir = root()
            .join("fixtures")
            .join(name)
            .join("sys/firmware/dmi/tables");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("smbios_entry_point"), entry)?;
        std::fs::write(dir.join("DMI"), table)?;
        eprintln!("wrote {}", dir.display());
    }
    for (name, w) in [
        ("windows-laptop", rigscope_probe_win::samples::laptop()),
        ("windows-vm", rigscope_probe_win::samples::vm()),
    ] {
        let dir = root().join("fixtures").join(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        w.save(&dir)?;
        eprintln!("fixtures/{name}: written");
    }
    Ok(())
}

fn unsafe_scope_check() -> Result<()> {
    let root = root();
    let mut manifests = Vec::new();
    let mut files = Vec::new();
    for dir in ["crates", "apps"] {
        for entry in std::fs::read_dir(root.join(dir))? {
            let krate = entry?.path();
            let manifest = krate.join("Cargo.toml");
            if manifest.exists() {
                let rel = manifest
                    .strip_prefix(&root)?
                    .to_string_lossy()
                    .replace('\\', "/");
                manifests.push((rel, std::fs::read_to_string(&manifest)?));
            }
            collect_rs(&root, &krate.join("src"), &mut files)?;
            collect_rs(&root, &krate.join("tests"), &mut files)?;
        }
    }
    let v = unsafe_scope::check(&manifests, &files);
    if !v.is_empty() {
        for x in &v {
            eprintln!("unsafe scope: {x}");
        }
        bail!("{} unsafe-scope violation(s)", v.len());
    }
    eprintln!("unsafe scope: OK");
    Ok(())
}

fn collect_rs(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> Result<()> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for e in rd {
        let p = e?.path();
        if p.is_dir() {
            collect_rs(root, &p, out)?;
        } else if p.extension().is_some_and(|x| x == "rs") {
            let rel = p.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
            out.push((rel, std::fs::read_to_string(&p)?));
        }
    }
    Ok(())
}
