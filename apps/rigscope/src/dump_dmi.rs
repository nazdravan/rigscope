//! The privileged side: `rigscope --dump-dmi` (run through pkexec) reads exactly the two DMI
//! files and writes them, framed, to stdout. It parses nothing and takes no other input.

use rigscope_engine::EngineError;
use rigscope_probe::{encode_dmi_dump, paths, DmiDump, Source};

/// Exit code of `--dump-dmi` on failure.
pub const DUMP_FAILED: i32 = 2;

/// Reads the two files from `src` and frames them.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn dump(src: &dyn Source) -> Result<Vec<u8>, String> {
    let read = |path: &str| src.read(path).map_err(|e| format!("{path}: {e:?}"));
    let entry = read(paths::DMI_ENTRY)?;
    let table = read(paths::DMI_TABLE)?;
    Ok(encode_dmi_dump(&DmiDump { entry, table }))
}

/// Interprets the helper's exit code (as seen through pkexec) and stdout.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn interpret(code: Option<i32>, stdout: Vec<u8>, stderr: &str) -> Result<Vec<u8>, EngineError> {
    match code {
        Some(0) => Ok(stdout),
        // pkexec: 126 = the user dismissed the authentication dialog.
        Some(126) => Err(EngineError::Io("cancelled".into())),
        // pkexec: 127 = not authorized, or pkexec itself is missing (shell convention).
        Some(127) => Err(EngineError::Io(
            "not authorized or pkexec not available".into(),
        )),
        None => Err(EngineError::Io("helper was killed".into())),
        Some(code) => {
            let msg = stderr.trim();
            Err(EngineError::Io(if msg.is_empty() {
                format!("helper failed (exit {code})")
            } else {
                msg.to_string()
            }))
        }
    }
}

/// Runs `pkexec <this binary> --dump-dmi`, giving up after [`HELPER_TIMEOUT`].
#[cfg(target_os = "linux")]
pub fn run_helper() -> Result<Vec<u8>, EngineError> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let exe = std::env::current_exe().map_err(|e| EngineError::Io(e.to_string()))?;
    let mut child = match Command::new("pkexec")
        .arg(exe)
        .arg("--dump-dmi")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return interpret(Some(127), Vec::new(), "")
        }
        Err(e) => return Err(EngineError::Io(format!("pkexec: {e}"))),
    };
    // Drain both pipes on threads so a chatty helper cannot block on a full pipe.
    let reader = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.by_ref().take(256 * 1024).read_to_end(&mut buf);
            }
            buf
        })
    };
    let out = reader(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let err = reader(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < HELPER_TIMEOUT => {
                std::thread::sleep(std::time::Duration::from_millis(100))
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(EngineError::Io(
                    "timed out waiting for authentication".into(),
                ));
            }
            Err(e) => return Err(EngineError::Io(e.to_string())),
        }
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned();
    interpret(status.code(), stdout, &stderr)
}

/// How long to wait for the user to authenticate.
#[cfg(target_os = "linux")]
pub const HELPER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

#[cfg(test)]
mod tests {
    use super::*;
    use rigscope_probe::{decode_dmi_dump, MemSource};

    #[test]
    fn dump_frames_both_files() {
        let src = MemSource::default()
            .with(paths::DMI_ENTRY, b"_SM3_x".to_vec())
            .with(paths::DMI_TABLE, vec![7; 10]);
        let d = decode_dmi_dump(&dump(&src).unwrap()).unwrap();
        assert_eq!(d.entry, b"_SM3_x");
        assert_eq!(d.table, vec![7; 10]);
    }

    #[test]
    fn dump_reports_missing_files() {
        let err = dump(&MemSource::default()).unwrap_err();
        assert!(err.contains("smbios_entry_point"), "{err}");
    }

    #[test]
    fn interpret_success_returns_stdout() {
        assert_eq!(interpret(Some(0), vec![1, 2], ""), Ok(vec![1, 2]));
    }

    #[test]
    fn interpret_cancel_and_missing_pkexec() {
        assert_eq!(
            interpret(Some(126), vec![], ""),
            Err(EngineError::Io("cancelled".into()))
        );
        assert_eq!(
            interpret(Some(127), vec![], ""),
            Err(EngineError::Io(
                "not authorized or pkexec not available".into()
            ))
        );
        assert_eq!(
            interpret(None, vec![], ""),
            Err(EngineError::Io("helper was killed".into()))
        );
    }

    #[test]
    fn interpret_helper_failure_shows_stderr() {
        assert_eq!(
            interpret(Some(DUMP_FAILED), vec![], "DMI: permission denied\n"),
            Err(EngineError::Io("DMI: permission denied".into()))
        );
    }
}
