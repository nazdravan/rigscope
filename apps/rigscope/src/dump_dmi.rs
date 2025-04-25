//! The privileged side: `rigscope --dump-dmi` (run through pkexec) reads the two DMI files and
//! runs `smartctl --json -a` (fixed system path, fixed arguments, validated disk names) for each
//! physical disk, then writes everything framed to stdout. It parses nothing from the user and
//! takes no other input; the unprivileged GUI parses the result.

use rigscope_engine::EngineError;
use rigscope_probe::{encode_dmi_dump, paths, DmiDump, Source};

/// Exit code of `--dump-dmi` on failure.
pub const DUMP_FAILED: i32 = 2;

/// Kernel disk names only: `nvme0n1`, `sda`, `mmcblk0`. Anything else is never passed on.
pub fn valid_disk_name(name: &str) -> bool {
    (1..=15).contains(&name.len())
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// smartctl is looked up only in fixed system paths (never through `PATH`).
pub const SMARTCTL_PATHS: &[&str] = &["/usr/sbin/smartctl", "/usr/bin/smartctl", "/sbin/smartctl"];

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn find_smartctl(exists: impl Fn(&str) -> bool) -> Option<&'static str> {
    SMARTCTL_PATHS.iter().copied().find(|p| exists(p))
}

/// Reads the DMI files and asks `smart` for each physical disk's smartctl JSON, then frames
/// everything. A missing DMI table is fine as long as there are disks to report on.
pub fn dump(src: &dyn Source, smart: &dyn Fn(&str) -> Vec<u8>) -> Result<Vec<u8>, String> {
    let read = |path: &str| src.read(path).map_err(|e| format!("{path}: {e:?}"));
    let dmi = read(paths::DMI_ENTRY).and_then(|e| Ok((e, read(paths::DMI_TABLE)?)));
    let disks: Vec<String> = rigscope_probe::read_storage(src)
        .disks
        .into_iter()
        .filter(|d| d.health == rigscope_model::Field::NeedsRoot && valid_disk_name(&d.name))
        .map(|d| d.name)
        .collect();
    let (entry, table) = match dmi {
        Ok(parts) => parts,
        Err(e) if disks.is_empty() => return Err(e),
        Err(_) => (Vec::new(), Vec::new()),
    };
    let smart = disks.iter().map(|n| (n.clone(), smart(n))).collect();
    Ok(encode_dmi_dump(&DmiDump {
        entry,
        table,
        smart,
    }))
}

/// Runs `smartctl --json=c -a /dev/<name>` with a clean environment and a time limit; problems
/// come back as `{"rigscope_error": ...}` so the GUI can show them per disk.
#[cfg(target_os = "linux")]
pub fn smartctl_json(name: &str) -> Vec<u8> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let error = |msg: &str| {
        serde_json::json!({ "rigscope_error": msg })
            .to_string()
            .into_bytes()
    };
    if !valid_disk_name(name) {
        return error("invalid disk name");
    }
    let Some(smartctl) = find_smartctl(|p| std::path::Path::new(p).is_file()) else {
        return error("smartctl not installed (smartmontools package)");
    };
    let Ok(mut child) = Command::new(smartctl)
        .args(["--json=c", "-a", &format!("/dev/{name}")])
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return error("smartctl could not be started");
    };
    let out = child.stdout.take().map(|mut p| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = p
                .by_ref()
                .take(rigscope_probe::MAX_SMART_JSON as u64)
                .read_to_end(&mut buf);
            buf
        })
    });
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < SMARTCTL_TIMEOUT => {
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return error("smartctl timed out");
            }
        }
    }
    out.and_then(|h| h.join().ok()).unwrap_or_default()
}

/// Per-disk limit for smartctl (a sleeping HDD may need a few seconds to answer).
#[cfg(target_os = "linux")]
pub const SMARTCTL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// Interprets the helper's exit code (as seen through pkexec) and stdout.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn interpret(code: Option<i32>, stdout: Vec<u8>, stderr: &str) -> Result<Vec<u8>, EngineError> {
    match code {
        Some(0) => Ok(stdout),
        // pkexec: 126 = the user dismissed the authentication dialog.
        Some(126) => Err(EngineError::Io("cancelled".into())),
        // pkexec: 127 = not authorized, or pkexec itself is missing (shell convention).
        Some(127) => {
            let base = "not authorized or pkexec not available";
            let detail = stderr.trim();
            Err(EngineError::Io(if detail.is_empty() {
                base.into()
            } else {
                format!("{base}: {detail}")
            }))
        }
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

/// Where the packages install the root-owned copy of the binary that polkit authorizes
/// (`packaging/linux/org.rigscope.dump-dmi.policy` names the same path).
pub const INSTALLED_HELPER: &str = "/usr/libexec/rigscope/rigscope-helper";

/// Set to `1` to let pkexec run a binary that is not root-owned (development builds only).
pub const ALLOW_USER_HELPER_ENV: &str = "RIGSCOPE_ALLOW_USER_HELPER";

/// A helper may run as root only if root owns it and nobody else can write to it: otherwise any
/// process of the user could swap the binary after the authentication dialog was approved.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn helper_is_trusted(uid: u32, mode: u32) -> bool {
    uid == 0 && mode & 0o022 == 0
}

/// Picks the program pkexec runs: the installed helper if it is trusted, else the running binary
/// if that one is trusted, else the running binary when `allow_user` (development) is set.
/// `stat` returns `(uid, mode)` of a path, `None` when it is missing.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn choose_helper(
    exe: &str,
    allow_user: bool,
    stat: impl Fn(&str) -> Option<(u32, u32)>,
) -> Result<String, EngineError> {
    let trusted = |p: &str| stat(p).is_some_and(|(uid, mode)| helper_is_trusted(uid, mode));
    if trusted(INSTALLED_HELPER) {
        Ok(INSTALLED_HELPER.into())
    } else if trusted(exe) || allow_user {
        Ok(exe.into())
    } else {
        Err(EngineError::Io(format!(
            "elevated reading needs the installed package: {INSTALLED_HELPER} is missing or not \
             root-owned (for a development build set {ALLOW_USER_HELPER_ENV}=1)"
        )))
    }
}

/// Runs `pkexec <helper> --dump-dmi`, giving up after [`HELPER_TIMEOUT`].
#[cfg(target_os = "linux")]
pub fn run_helper() -> Result<Vec<u8>, EngineError> {
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    use std::process::{Command, Stdio};
    let exe = std::env::current_exe().map_err(|e| EngineError::Io(e.to_string()))?;
    let exe = exe.to_string_lossy().into_owned();
    let allow_user = std::env::var(ALLOW_USER_HELPER_ENV).is_ok_and(|v| v == "1");
    let helper = choose_helper(&exe, allow_user, |p| {
        std::fs::metadata(p).ok().map(|m| (m.uid(), m.mode()))
    })?;
    let mut child = match Command::new("pkexec")
        .arg(helper)
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
                let _ = p.by_ref().take(3 * 1024 * 1024).read_to_end(&mut buf);
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

    fn no_smart(_: &str) -> Vec<u8> {
        b"{}".to_vec()
    }

    #[test]
    fn dump_frames_both_files() {
        let src = MemSource::default()
            .with(paths::DMI_ENTRY, b"_SM3_x".to_vec())
            .with(paths::DMI_TABLE, vec![7; 10]);
        let d = decode_dmi_dump(&dump(&src, &no_smart).unwrap()).unwrap();
        assert_eq!(d.entry, b"_SM3_x");
        assert_eq!(d.table, vec![7; 10]);
    }

    #[test]
    fn dump_runs_smart_for_physical_disks_only() {
        let mut src = MemSource::default()
            .with("/sys/block/nvme0n1/size", "1\n")
            .with("/sys/block/sda/size", "1\n")
            .with("/sys/block/vda/size", "1\n");
        src.links.insert(
            "/sys/block/sda".into(),
            "/sys/devices/pci0000:00/ata1/block/sda".into(),
        );
        src.links.insert(
            "/sys/block/vda".into(),
            "/sys/devices/pci0000:00/virtio1/block/vda".into(),
        );
        let smart = |name: &str| format!("{{\"for\":\"{name}\"}}").into_bytes();
        let d = decode_dmi_dump(&dump(&src, &smart).unwrap()).unwrap();
        let names: Vec<&str> = d.smart.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["nvme0n1", "sda"], "virtio has no SMART");
        assert_eq!(d.smart[1].1, br#"{"for":"sda"}"#);
        assert!(
            d.entry.is_empty(),
            "no DMI table is fine when SMART is there"
        );
    }

    #[test]
    fn dump_fails_when_there_is_nothing_to_read() {
        let err = dump(&MemSource::default(), &no_smart).unwrap_err();
        assert!(err.contains("smbios_entry_point"), "{err}");
    }

    #[test]
    fn disk_names_are_validated() {
        assert!(valid_disk_name("nvme0n1"));
        assert!(valid_disk_name("sda"));
        assert!(!valid_disk_name("../etc"));
        assert!(!valid_disk_name("sda;rm"));
        assert!(!valid_disk_name(""));
        assert!(!valid_disk_name("a-very-long-name-xx"));
    }

    #[test]
    fn smartctl_is_found_only_in_fixed_paths() {
        assert_eq!(
            find_smartctl(|p| p == "/usr/bin/smartctl"),
            Some("/usr/bin/smartctl")
        );
        assert_eq!(find_smartctl(|_| false), None);
    }

    #[test]
    fn helper_trust_needs_root_and_no_foreign_write() {
        assert!(helper_is_trusted(0, 0o100755));
        assert!(!helper_is_trusted(0, 0o100775), "group-writable");
        assert!(!helper_is_trusted(0, 0o100757), "world-writable");
        assert!(!helper_is_trusted(1000, 0o100755), "owned by the user");
    }

    #[test]
    fn installed_helper_wins_over_the_running_binary() {
        let stat = |_: &str| Some((0, 0o100755));
        assert_eq!(
            choose_helper("/opt/x", false, stat).unwrap(),
            INSTALLED_HELPER
        );
    }

    #[test]
    fn running_binary_is_used_when_it_is_root_owned() {
        let stat = |p: &str| (p == "/usr/bin/rigscope").then_some((0, 0o100755));
        assert_eq!(
            choose_helper("/usr/bin/rigscope", false, stat).unwrap(),
            "/usr/bin/rigscope"
        );
    }

    #[test]
    fn user_owned_binary_is_refused_unless_development_flag() {
        let stat = |p: &str| (p == "/home/u/rigscope").then_some((1000, 0o100755));
        let err = choose_helper("/home/u/rigscope", false, stat).unwrap_err();
        assert!(err.to_string().contains(ALLOW_USER_HELPER_ENV), "{err}");
        assert_eq!(
            choose_helper("/home/u/rigscope", true, stat).unwrap(),
            "/home/u/rigscope"
        );
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
    fn interpret_127_keeps_pkexec_message() {
        assert_eq!(
            interpret(Some(127), vec![], "Error executing command as another user: No authentication agent found.\n"),
            Err(EngineError::Io(
                "not authorized or pkexec not available: Error executing command as another user: No authentication agent found.".into()
            ))
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
