//! The privileged read. `pkexec /bin/sh -c ROOT_SCRIPT` runs a fixed script, compiled into this
//! binary, that prints the two DMI files and `smartctl --json=c -a` for each disk as hex. Nothing
//! from rigscope itself runs as root, so it works the same from an AppImage, a `.tar.gz` or a
//! build folder: there is no helper binary to install or to trust. The script takes only the
//! smartctl path (one of [`SMARTCTL_PATHS`]) and kernel disk names, and checks both again; the
//! unprivileged window parses everything.

use rigscope_engine::EngineError;
use rigscope_probe::{encode_dmi_dump, paths, DmiDump, Source, MAX_DMI_PART, MAX_SMART_JSON};

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

/// The disks whose SMART data needs root (physical disks with a valid kernel name).
pub fn disks_to_query(src: &dyn Source) -> Vec<String> {
    rigscope_probe::read_storage(src)
        .disks
        .into_iter()
        .filter(|d| d.health == rigscope_model::Field::NeedsRoot && valid_disk_name(&d.name))
        .map(|d| d.name)
        .collect()
}

/// Shown for a disk when the root side found no smartctl.
pub const NO_SMARTCTL: &str = "smartctl not installed (smartmontools package)";

/// Frames the DMI files and the per-disk smartctl JSON. A missing DMI table is fine as long as
/// there are disks to report on.
pub fn assemble(
    dmi: Result<(Vec<u8>, Vec<u8>), String>,
    smart: Vec<(String, Vec<u8>)>,
) -> Result<Vec<u8>, String> {
    let (entry, table) = match dmi {
        Ok(parts) => parts,
        Err(e) if smart.is_empty() => return Err(e),
        Err(_) => (Vec::new(), Vec::new()),
    };
    Ok(encode_dmi_dump(&DmiDump {
        entry,
        table,
        smart,
    }))
}

/// Fixture mode: the same result from files, with `smart` standing in for smartctl.
pub fn dump(src: &dyn Source, smart: &dyn Fn(&str) -> Vec<u8>) -> Result<Vec<u8>, String> {
    let read = |path: &str| src.read(path).map_err(|e| format!("{path}: {e:?}"));
    let dmi = read(paths::DMI_ENTRY).and_then(|e| Ok((e, read(paths::DMI_TABLE)?)));
    let smart = disks_to_query(src)
        .into_iter()
        .map(|n| {
            let json = smart(&n);
            (n, json)
        })
        .collect();
    assemble(dmi, smart)
}

/// Runs as root under `/bin/sh` (POSIX sh, coreutils `od`; `timeout` when present).
/// Arguments: `$1` = smartctl path or empty, then disk names. Output: `RSPART <label>` lines,
/// each followed by `od` hex of its bytes or `RSMISSING`; `RSCODE <n>` after each smartctl;
/// `RSEND` last, so a cut-off output is recognised.
pub const ROOT_SCRIPT: &str = r#"set -u
smartctl=$1
shift
case $smartctl in
    '' | /usr/sbin/smartctl | /usr/bin/smartctl | /sbin/smartctl) ;;
    *) echo "rigscope: unexpected smartctl path" >&2; exit 3 ;;
esac
for f in smbios_entry_point DMI; do
    printf 'RSPART dmi %s\n' "$f"
    if [ -r "/sys/firmware/dmi/tables/$f" ]; then
        od -An -v -tx1 < "/sys/firmware/dmi/tables/$f"
    else
        printf 'RSMISSING\n'
    fi
done
for d in "$@"; do
    case $d in
        '' | *[!a-z0-9]*) continue ;;
    esac
    printf 'RSPART smart %s\n' "$d"
    if [ -z "$smartctl" ]; then
        printf 'RSMISSING\n'
        continue
    fi
    if command -v timeout > /dev/null 2>&1; then
        json=$(timeout 20 "$smartctl" --json=c -a "/dev/$d" 2> /dev/null)
    else
        json=$("$smartctl" --json=c -a "/dev/$d" 2> /dev/null)
    fi
    code=$?
    printf '%s' "$json" | od -An -v -tx1
    printf 'RSCODE %s\n' "$code"
done
printf 'RSEND\n'
"#;

/// `$0` of the script (shown by `ps` and in some authentication dialogs).
pub const SCRIPT_NAME: &str = "rigscope-read-hardware";

/// The arguments after `pkexec`: `/bin/sh -c ROOT_SCRIPT rigscope-read-hardware SMARTCTL DISKS…`.
pub fn pkexec_args(smartctl: Option<&str>, disks: &[String]) -> Vec<String> {
    let mut args = vec![
        "/bin/sh".to_string(),
        "-c".to_string(),
        ROOT_SCRIPT.to_string(),
        SCRIPT_NAME.to_string(),
        smartctl.unwrap_or_default().to_string(),
    ];
    args.extend(disks.iter().filter(|d| valid_disk_name(d)).cloned());
    args
}

/// Largest script output read (hex triples the size of what it encodes).
pub const MAX_SCRIPT_OUTPUT: u64 = 32 * 1024 * 1024;

/// `timeout` exits with 124 when it had to stop smartctl.
const TIMED_OUT: i32 = 124;

fn error_json(msg: &str) -> Vec<u8> {
    serde_json::json!({ "rigscope_error": msg })
        .to_string()
        .into_bytes()
}

/// One `RSPART` block while parsing.
struct Part {
    label: String,
    bytes: Vec<u8>,
    missing: bool,
    code: Option<i32>,
}

/// Turns the script's output into the dump the engine decodes.
pub fn parse_script_output(out: &[u8], asked: &[String]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(out).map_err(|_| "elevated read: output is not text")?;
    let mut parts: Vec<Part> = Vec::new();
    let mut ended = false;
    for line in text.lines() {
        if ended {
            return Err("elevated read: output after the end marker".into());
        }
        if let Some(label) = line.strip_prefix("RSPART ") {
            parts.push(Part {
                label: label.to_string(),
                bytes: Vec::new(),
                missing: false,
                code: None,
            });
            continue;
        }
        if line == "RSEND" {
            ended = true;
            continue;
        }
        let part = parts
            .last_mut()
            .ok_or("elevated read: data before the first part")?;
        if line == "RSMISSING" {
            part.missing = true;
        } else if let Some(code) = line.strip_prefix("RSCODE ") {
            part.code = code.trim().parse().ok();
        } else {
            for token in line.split_whitespace() {
                let byte = u8::from_str_radix(token, 16)
                    .ok()
                    .filter(|_| token.len() == 2)
                    .ok_or("elevated read: bad hex in the output")?;
                part.bytes.push(byte);
            }
            if part.bytes.len() > MAX_DMI_PART.max(MAX_SMART_JSON) {
                return Err("elevated read: part too large".into());
            }
        }
    }
    if !ended {
        return Err("elevated read was cut off".into());
    }

    let mut dmi_file = |name: &str| -> Result<Vec<u8>, String> {
        let label = format!("dmi {name}");
        match parts.iter_mut().find(|p| p.label == label) {
            Some(p) if !p.missing => Ok(std::mem::take(&mut p.bytes)),
            _ => Err(format!(
                "no DMI table at /sys/firmware/dmi/tables/{name} (virtual machine or container?)"
            )),
        }
    };
    let dmi = dmi_file("smbios_entry_point").and_then(|e| Ok((e, dmi_file("DMI")?)));

    let mut smart = Vec::new();
    for part in parts {
        let Some(name) = part.label.strip_prefix("smart ") else {
            continue;
        };
        // Only disks this side asked about, each once.
        if !asked.iter().any(|a| a == name) || smart.iter().any(|(n, _)| n == name) {
            return Err(format!("elevated read: unexpected disk {name:?}"));
        }
        let json = if part.missing {
            error_json(NO_SMARTCTL)
        } else if part.code == Some(TIMED_OUT)
            && serde_json::from_slice::<serde_json::Value>(&part.bytes).is_err()
        {
            error_json("smartctl timed out")
        } else {
            part.bytes
        };
        smart.push((name.to_string(), json));
    }
    assemble(dmi, smart)
}

/// Interprets pkexec's exit code and the script's output.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn interpret(
    code: Option<i32>,
    stdout: &[u8],
    stderr: &str,
    asked: &[String],
) -> Result<Vec<u8>, EngineError> {
    match code {
        Some(0) => parse_script_output(stdout, asked).map_err(EngineError::Io),
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
        None => Err(EngineError::Io("elevated read was killed".into())),
        Some(code) => {
            let msg = stderr.trim();
            Err(EngineError::Io(if msg.is_empty() {
                format!("elevated read failed (exit {code})")
            } else {
                msg.to_string()
            }))
        }
    }
}

/// Runs the root script through pkexec for `disks`, giving up after [`ELEVATE_TIMEOUT`].
#[cfg(target_os = "linux")]
pub fn run_elevated(disks: &[String]) -> Result<Vec<u8>, EngineError> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let smartctl = find_smartctl(|p| std::path::Path::new(p).is_file());
    let mut child = match Command::new("pkexec")
        .args(pkexec_args(smartctl, disks))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return interpret(Some(127), &[], "", disks)
        }
        Err(e) => return Err(EngineError::Io(format!("pkexec: {e}"))),
    };
    // Drain both pipes on threads so a chatty child cannot block on a full pipe.
    let reader = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.by_ref().take(MAX_SCRIPT_OUTPUT).read_to_end(&mut buf);
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
            Ok(None) if start.elapsed() < ELEVATE_TIMEOUT => {
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
    interpret(status.code(), &stdout, &stderr, disks)
}

/// How long to wait for the user to authenticate (and for smartctl on every disk).
#[cfg(target_os = "linux")]
pub const ELEVATE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

#[cfg(test)]
mod tests {
    use super::*;
    use rigscope_probe::{decode_dmi_dump, MemSource};

    fn no_smart(_: &str) -> Vec<u8> {
        b"{}".to_vec()
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!(" {b:02x}")).collect()
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
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
        assert_eq!(disks_to_query(&src), names(&["nvme0n1", "sda"]));
        let smart = |name: &str| format!("{{\"for\":\"{name}\"}}").into_bytes();
        let d = decode_dmi_dump(&dump(&src, &smart).unwrap()).unwrap();
        let got: Vec<&str> = d.smart.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(got, vec!["nvme0n1", "sda"], "virtio has no SMART");
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
    fn script_accepts_exactly_the_fixed_smartctl_paths() {
        for p in SMARTCTL_PATHS {
            assert!(ROOT_SCRIPT.contains(&format!("| {p}")), "{p} missing");
        }
    }

    #[test]
    fn pkexec_runs_system_sh_and_drops_bad_disk_names() {
        let args = pkexec_args(
            Some("/usr/bin/smartctl"),
            &names(&["sda", "x;y", "nvme0n1"]),
        );
        assert_eq!(args[0], "/bin/sh");
        assert_eq!(args[1], "-c");
        assert_eq!(args[2], ROOT_SCRIPT);
        assert_eq!(
            &args[3..],
            &names(&[SCRIPT_NAME, "/usr/bin/smartctl", "sda", "nvme0n1"])
        );
        assert_eq!(pkexec_args(None, &[])[4], "", "no smartctl = empty");
    }

    #[test]
    fn parses_dmi_and_smart() {
        let out = format!(
            "RSPART dmi smbios_entry_point\n{}\nRSPART dmi DMI\n{}\n{}\n\
             RSPART smart sda\n{}\nRSCODE 0\nRSEND\n",
            hex(b"_SM3_"),
            hex(&[1, 2, 3]),
            hex(&[4]),
            hex(br#"{"ok":1}"#)
        );
        let d = decode_dmi_dump(&parse_script_output(out.as_bytes(), &names(&["sda"])).unwrap())
            .unwrap();
        assert_eq!(d.entry, b"_SM3_");
        assert_eq!(d.table, vec![1, 2, 3, 4]);
        assert_eq!(d.smart, vec![("sda".to_string(), br#"{"ok":1}"#.to_vec())]);
    }

    #[test]
    fn missing_dmi_with_disks_still_reports_smart() {
        let out = "RSPART dmi smbios_entry_point\nRSMISSING\nRSPART dmi DMI\nRSMISSING\n\
             RSPART smart vdb\nRSMISSING\nRSPART smart sda\n\nRSCODE 124\nRSEND\n";
        let d =
            decode_dmi_dump(&parse_script_output(out.as_bytes(), &names(&["vdb", "sda"])).unwrap())
                .unwrap();
        assert!(d.entry.is_empty() && d.table.is_empty());
        assert!(String::from_utf8_lossy(&d.smart[0].1).contains(NO_SMARTCTL));
        assert!(String::from_utf8_lossy(&d.smart[1].1).contains("timed out"));
    }

    #[test]
    fn missing_dmi_without_disks_says_why() {
        let out = "RSPART dmi smbios_entry_point\nRSMISSING\nRSPART dmi DMI\nRSMISSING\nRSEND\n";
        let err = parse_script_output(out.as_bytes(), &[]).unwrap_err();
        assert!(err.contains("no DMI table"), "{err}");
    }

    #[test]
    fn bad_output_is_refused() {
        let ok_head = "RSPART dmi smbios_entry_point\n 00\nRSPART dmi DMI\n 00\n";
        for (out, why) in [
            (ok_head.to_string(), "cut off"),
            (format!("{ok_head} zz\nRSEND\n"), "bad hex"),
            (format!("{ok_head} 123\nRSEND\n"), "bad hex"),
            (format!(" 00\n{ok_head}RSEND\n"), "before the first part"),
            (format!("{ok_head}RSEND\nRSPART x\n"), "after the end"),
            (
                format!("{ok_head}RSPART smart sdz\n 00\nRSEND\n"),
                "unexpected disk",
            ),
        ] {
            let err = parse_script_output(out.as_bytes(), &names(&["sda"])).unwrap_err();
            assert!(err.contains(why), "{why}: {err}");
        }
        assert!(parse_script_output(&[0xff, 0xfe], &[]).is_err());
    }

    #[test]
    fn interpret_cancel_and_missing_pkexec() {
        assert_eq!(
            interpret(Some(126), &[], "", &[]),
            Err(EngineError::Io("cancelled".into()))
        );
        assert_eq!(
            interpret(Some(127), &[], "", &[]),
            Err(EngineError::Io(
                "not authorized or pkexec not available".into()
            ))
        );
        assert_eq!(
            interpret(None, &[], "", &[]),
            Err(EngineError::Io("elevated read was killed".into()))
        );
    }

    #[test]
    fn interpret_127_keeps_pkexec_message() {
        assert_eq!(
            interpret(Some(127), &[], "Error executing command as another user: No authentication agent found.\n", &[]),
            Err(EngineError::Io(
                "not authorized or pkexec not available: Error executing command as another user: No authentication agent found.".into()
            ))
        );
    }

    #[test]
    fn interpret_script_failure_shows_stderr() {
        assert_eq!(
            interpret(Some(3), &[], "rigscope: unexpected smartctl path\n", &[]),
            Err(EngineError::Io("rigscope: unexpected smartctl path".into()))
        );
    }

    /// The real script under the local `/bin/sh`, without root: DMI is unreadable here (or
    /// readable, when the tests run as root), the disk list is empty.
    #[cfg(target_os = "linux")]
    #[test]
    fn root_script_runs_under_sh() {
        let args = pkexec_args(None, &names(&["sda"]));
        let out = std::process::Command::new(&args[0])
            .args(&args[1..])
            .output()
            .unwrap();
        assert!(out.status.success(), "{:?}", out);
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(
            text.starts_with("RSPART dmi smbios_entry_point\n"),
            "{text}"
        );
        assert!(text.contains("RSPART smart sda\nRSMISSING\n"), "{text}");
        assert!(text.ends_with("RSEND\n"), "{text}");
        let dump = parse_script_output(text.as_bytes(), &names(&["sda"])).unwrap();
        assert_eq!(decode_dmi_dump(&dump).unwrap().smart.len(), 1);
    }
}
