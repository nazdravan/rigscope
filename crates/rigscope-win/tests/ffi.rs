#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use rigscope_win::ffi;

#[test]
fn smbios_has_a_valid_header() {
    let b = ffi::smbios_raw().unwrap();
    assert!(b.len() > 8);
    let len = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
    assert_eq!(8 + len, b.len());
    assert!(b[1] >= 2, "SMBIOS major {}", b[1]);
}

#[test]
fn processors_memory_uptime_name_os() {
    assert!(!ffi::processors_raw().unwrap().is_empty());
    assert!(ffi::memory_status().unwrap().total_phys > 512 << 20);
    assert!(ffi::uptime_ms() > 0);
    assert!(!ffi::computer_name().unwrap().is_empty());
    let os = ffi::os_version().unwrap();
    assert!(os.current_build.unwrap() >= 17763);
    let cpu = ffi::cpu_registry().unwrap();
    assert!(cpu.name.is_some() && cpu.mhz.is_some());
    #[cfg(target_arch = "x86_64")]
    assert!(ffi::cpu_features().iter().any(|f| f == "sse2"));
}

#[test]
fn pdh_second_sample_has_values() {
    let mut p = ffi::Pdh::new().unwrap();
    // No rate yet on the first sample: an empty list, not an error.
    assert!(p.sample().unwrap().is_empty());
    std::thread::sleep(std::time::Duration::from_millis(300));
    let s = p.sample().unwrap();
    assert!(!s.is_empty());
    assert!(s.iter().all(|(_, v)| v.is_finite() && *v >= 0.0));
}

#[test]
fn elevation_is_reported() {
    // GitHub runners run as administrator with an elevated token. On the development machine
    // it depends on the terminal, so nothing is checked there.
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        assert!(ffi::is_elevated());
    }
}
