//! `rigscope --capture-fixture DIR` (Windows): saves this machine as a fixture, masked.

use anyhow::Result;
use rigscope_probe_win::CAPTURE_LOG;
use std::path::Path;

/// A release build on Windows has no console, so the outcome also goes to `DIR/capture.log`.
/// A refused DIR gets no log: nothing is written into a folder that is not ours.
pub fn run(dir: &Path) -> Result<()> {
    check_target(dir)?;
    let r = write(dir);
    log(dir, &r);
    r
}

/// Writes the capture outcome to `DIR/capture.log` (and stderr). Best effort: a log that
/// cannot be written must not hide the capture's own result.
fn log(dir: &Path, r: &Result<()>) {
    let text = match r {
        Ok(()) => format!(
            "fixture written to {}
",
            dir.display()
        ),
        Err(e) => format!(
            "capture failed: {e:#}
"
        ),
    };
    eprint!("{text}");
    let _ = std::fs::write(dir.join(CAPTURE_LOG), text);
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
    // A log alone is what a capture that failed before writing the fixture leaves behind.
    let mut entries = std::fs::read_dir(dir)
        .map_err(|e| anyhow::anyhow!("{shown}: {e}"))?
        .filter(|e| !matches!(e, Ok(e) if e.file_name() == CAPTURE_LOG));
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
        .map_err(|e| anyhow::anyhow!("{}: {e}", dir.display()))
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
        std::fs::write(d.join(CAPTURE_LOG), b"fixture written").unwrap();
        assert!(check_target(&d).is_ok(), "an earlier fixture with its log");
        std::fs::remove_dir_all(&d).unwrap();
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(CAPTURE_LOG), b"capture failed").unwrap();
        assert!(check_target(&d).is_ok(), "only the log of a failed capture");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn log_records_success_or_the_error() {
        let d = temp("log");
        std::fs::create_dir_all(&d).unwrap();
        log(&d, &Ok(()));
        let ok = std::fs::read_to_string(d.join(CAPTURE_LOG)).unwrap();
        assert!(ok.starts_with("fixture written to "), "{ok}");
        log(&d, &Err(anyhow::anyhow!("smbios: access denied")));
        let err = std::fs::read_to_string(d.join(CAPTURE_LOG)).unwrap();
        assert_eq!(
            err,
            "capture failed: smbios: access denied
"
        );
        // Not a fixture, but a later capture may still use the folder.
        assert!(!rigscope_probe_win::MemWin::is_fixture(&d));
        assert!(check_target(&d).is_ok());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_log_does_not_break_loading_the_fixture() {
        let d = temp("load");
        let w = rigscope_probe_win::MemWin::default();
        w.save(&d).unwrap();
        log(&d, &Ok(()));
        assert_eq!(rigscope_probe_win::MemWin::load(&d).unwrap(), w);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
