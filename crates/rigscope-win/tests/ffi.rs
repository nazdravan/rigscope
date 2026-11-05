#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use rigscope_win::{WinError, ffi, layout};

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

#[test]
fn disks_via_wmi_and_ioctl_agree_on_count() {
    let wmi = ffi::physical_disks_wmi().unwrap();
    let io = ffi::physical_disks_ioctl().unwrap();
    assert!(!wmi.is_empty());
    assert_eq!(wmi.len(), io.len(), "wmi {wmi:?} vs ioctl {io:?}");
    for d in &io {
        assert!(rigscope_win::layout::parse_device_descriptor(&d.descriptor).is_some());
    }
}

/// `Ok(v)` → `Some(v)`; the expected "can't here" errors (not elevated, not supported on this
/// machine) → `None`. `Failed` and `Timeout` fail the test.
fn ok_or_cant<T: std::fmt::Debug>(what: &str, r: Result<T, WinError>) -> Option<T> {
    match r {
        Ok(v) => Some(v),
        Err(WinError::AccessDenied | WinError::NotAvailable) => None,
        Err(e) => panic!("{what}: unexpected {e:?}"),
    }
}

#[test]
fn smart_reads_succeed_or_report_access_denied_or_not_available() {
    for d in ffi::physical_disks_wmi().unwrap() {
        if d.bus_type == Some(17) {
            let r = ffi::nvme_health_raw(d.number);
            println!(
                "disk {} NVMe health: {:?}",
                d.number,
                r.as_ref().map(Vec::len)
            );
            if let Some(b) = ok_or_cant("nvme_health_raw", r) {
                assert!(
                    layout::nvme_log_from_response(&b).is_some(),
                    "disk {}",
                    d.number
                );
            }
        } else {
            let r = ffi::ata_smart_raw(d.number);
            println!(
                "disk {} ATA SMART: {:?}",
                d.number,
                r.as_ref().map(|a| (a.attributes.len(), a.thresholds.len()))
            );
            if let Some(a) = ok_or_cant("ata_smart_raw", r) {
                assert_eq!(a.attributes.len(), 512);
                assert_eq!(a.thresholds.len(), 512);
            }
        }
    }
    // Out of range for SMART_RCV_DRIVE_DATA's u8 drive number: no fallback to drive 0.
    assert_eq!(ffi::ata_smart_raw(256), Err(WinError::NotAvailable));
}

#[test]
fn gpu_net_battery_thermal() {
    let a = ffi::adapters_dxgi().unwrap();
    assert!(!a.is_empty()); // at least the Basic Render Driver
    for p in ffi::pci_display().unwrap() {
        assert_ne!(p.vendor_id, 0, "{p:?}");
    }
    for x in a.iter().filter(|x| !x.software) {
        let r = ffi::GpuPerf::temperature(x.luid);
        println!("GPU {} temperature: {r:?}", x.description);
        if let Some(t) = ok_or_cant("GpuPerf::temperature", r) {
            assert!(t.is_none_or(|t| t < 1500), "{t:?} deci-°C");
        }
    }
    assert!(!ffi::net_adapters().unwrap().is_empty());
    let b = ffi::batteries().unwrap(); // desktop: an empty list
    println!("batteries: {}", b.len());
    if !b.is_empty() {
        assert!(
            b.iter().any(|x| x.designed.is_some() || x.full.is_some()),
            "{b:?}"
        );
    }
    let r = ffi::acpi_thermal_zones();
    println!("thermal zones: {r:?}");
    if let Some(z) = ok_or_cant("acpi_thermal_zones", r) {
        assert!(z.iter().all(|z| z.tenths_kelvin > 2000), "{z:?}");
    }
}
