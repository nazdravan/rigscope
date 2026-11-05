//! The live macOS source: runs `system_profiler` and `sysctl` from their fixed system paths, with
//! fixed arguments, a time limit and an output cap. The arguments are checked, never shell-parsed.

use crate::{MacError, MacSource, MemMac, R, mask_fixture};
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const SYSTEM_PROFILER: &str = "/usr/sbin/system_profiler";
const SYSCTL: &str = "/usr/sbin/sysctl";
const PROFILER_TIMEOUT: Duration = Duration::from_secs(30);
const SYSCTL_TIMEOUT: Duration = Duration::from_secs(5);
/// Most output read from a tool.
const MAX_OUTPUT: u64 = 8 * 1024 * 1024;

/// The profiler data types rigscope reads (and captures for a fixture).
pub const DATA_TYPES: &[&str] = &[
    "SPMemoryDataType",
    "SPHardwareDataType",
    "SPSerialATADataType",
    "SPNVMeDataType",
    "SPDisplaysDataType",
    "SPNetworkDataType",
    "SPEthernetDataType",
];

/// The sysctl names rigscope reads (and captures for a fixture).
pub const SYSCTLS: &[&str] = &[
    "hw.memsize",
    "hw.model",
    "hw.physicalcpu",
    "hw.logicalcpu",
    "hw.packages",
    "hw.cpufrequency",
    "machdep.cpu.brand_string",
    "machdep.cpu.vendor",
    "kern.osproductversion",
    "kern.osrelease",
    "kern.hostname",
    "kern.boottime",
];

/// `SPMemoryDataType`-style names only.
fn valid_data_type(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.starts_with("SP")
        && s.chars().all(|c| c.is_ascii_alphanumeric())
}

/// `hw.memsize`-style names only.
fn valid_sysctl(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_')
}

fn run(program: &str, args: &[&str], limit: Duration) -> R<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => MacError::NotAvailable,
            _ => MacError::Failed(format!("{program}: {e}")),
        })?;
    let mut pipe = child.stdout.take().ok_or(MacError::NotAvailable)?;
    // Drain on a thread so a chatty tool cannot block on a full pipe while we wait.
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = pipe.by_ref().take(MAX_OUTPUT).read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() < limit => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(MacError::Timeout);
            }
            Err(e) => return Err(MacError::Failed(format!("{program}: {e}"))),
        }
    };
    let out = reader.join().unwrap_or_default();
    if !status.success() {
        return Err(MacError::NotAvailable);
    }
    String::from_utf8(out).map_err(|_| MacError::Failed(format!("{program}: not UTF-8")))
}

/// Runs the real tools.
#[derive(Clone, Copy, Debug, Default)]
pub struct RealMac;

impl MacSource for RealMac {
    fn profiler(&self, data_type: &str) -> R<String> {
        if !valid_data_type(data_type) {
            return Err(MacError::NotAvailable);
        }
        run(SYSTEM_PROFILER, &[data_type, "-json"], PROFILER_TIMEOUT)
    }

    fn sysctl(&self, name: &str) -> R<String> {
        if !valid_sysctl(name) {
            return Err(MacError::NotAvailable);
        }
        run(SYSCTL, &["-n", name], SYSCTL_TIMEOUT).map(|s| s.trim().to_string())
    }
}

/// Every value rigscope reads, saved as a fixture and masked.
pub fn capture() -> MemMac {
    let real = RealMac;
    let mut m = MemMac::new();
    for t in DATA_TYPES {
        if let Ok(text) = real.profiler(t) {
            if let Ok(v) = serde_json::from_str(&text) {
                m.profiler.insert((*t).to_string(), v);
            }
        }
    }
    for n in SYSCTLS {
        if let Ok(v) = real.sysctl(n) {
            m.sysctl.insert((*n).to_string(), v);
        }
    }
    mask_fixture(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_names_reach_the_tools() {
        assert!(valid_data_type("SPMemoryDataType"));
        assert!(!valid_data_type("-json"));
        assert!(!valid_data_type("SPMemory DataType"));
        assert!(!valid_data_type(""));
        assert!(valid_sysctl("hw.memsize"));
        assert!(valid_sysctl("machdep.cpu.brand_string"));
        assert!(!valid_sysctl("-a"));
        assert!(!valid_sysctl("hw.memsize; rm"));
        assert!(!valid_sysctl("../x"));
    }

    #[test]
    fn every_name_in_the_tables_is_valid() {
        assert!(DATA_TYPES.iter().all(|t| valid_data_type(t)));
        assert!(SYSCTLS.iter().all(|n| valid_sysctl(n)));
    }

    #[test]
    fn a_missing_program_is_not_available() {
        assert_eq!(
            run("/nonexistent/tool", &[], Duration::from_secs(1)),
            Err(MacError::NotAvailable)
        );
    }
}
