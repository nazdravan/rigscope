//! `rigscope --capture-fixture DIR` (Windows): saves this machine as a fixture, masked.

use anyhow::Result;
use std::path::Path;

pub fn run(dir: &Path) -> Result<()> {
    check_target(dir)?;
    write(dir)
}

/// `DIR` must be new, empty, or an earlier rigscope fixture: `save` replaces a fixture's own
/// files, and a folder with anything else in it is refused so nothing of the user's is touched.
fn check_target(dir: &Path) -> Result<()> {
    let shown = dir.display();
    match std::fs::metadata(dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => anyhow::bail!("{shown}: {e}"),
        Ok(m) if !m.is_dir() => anyhow::bail!("{shown}: not a directory"),
        Ok(_) => {}
    }
    if rigscope_probe_win::MemWin::is_fixture(dir) {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(dir).map_err(|e| anyhow::anyhow!("{shown}: {e}"))?;
    if entries.next().is_some() {
        anyhow::bail!(
            "{shown}: the directory is not empty and is not a rigscope fixture; \
             choose a new or empty directory"
        );
    }
    Ok(())
}

#[cfg(windows)]
fn write(dir: &Path) -> Result<()> {
    let mut real = rigscope_probe_win::RealWin::new();
    let w = rigscope_probe_win::capture(&mut real);
    w.save(dir)
        .map_err(|e| anyhow::anyhow!("{}: {e}", dir.display()))?;
    eprintln!("fixture written to {}", dir.display());
    Ok(())
}

#[cfg(not(windows))]
fn write(_dir: &Path) -> Result<()> {
    anyhow::bail!("--capture-fixture works only on Windows")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("rigscope-capture-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn refuses_a_folder_with_other_files_and_leaves_it_alone() {
        let d = temp("busy");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("thesis.docx"), b"mine").unwrap();
        // `check_target`, not `run`: on Windows a passing check would capture this machine.
        let err = check_target(&d).unwrap_err().to_string();
        assert!(err.contains("not a rigscope fixture"), "{err}");
        assert_eq!(std::fs::read(d.join("thesis.docx")).unwrap(), b"mine");
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1);
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn refuses_a_file() {
        let d = temp("file");
        std::fs::write(&d, b"x").unwrap();
        assert!(check_target(&d).is_err());
        std::fs::remove_file(&d).unwrap();
    }

    #[test]
    fn accepts_new_empty_or_fixture_folders() {
        let d = temp("ok");
        assert!(check_target(&d).is_ok(), "does not exist yet");
        std::fs::create_dir_all(&d).unwrap();
        assert!(check_target(&d).is_ok(), "empty");
        rigscope_probe_win::MemWin::default().save(&d).unwrap();
        std::fs::write(d.join("notes.txt"), b"x").unwrap();
        assert!(check_target(&d).is_ok(), "an earlier fixture");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
