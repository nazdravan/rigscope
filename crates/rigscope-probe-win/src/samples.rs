//! Synthetic Windows machines for tests and the `windows-laptop` / `windows-vm` fixtures.
//! Values are documented examples (a 2024 laptop, a Hyper-V guest), not captured data.

use crate::MemWin;
use rigscope_smbios::testutil::{memory_array, memory_device, table, Dimm, StructBuilder};
use rigscope_win::*;

fn rsmb(t: &[u8]) -> Vec<u8> {
    let mut b = vec![0, 3, 4, 0];
    b.extend(u32::try_from(t.len()).unwrap_or(0).to_le_bytes());
    b.extend_from_slice(t);
    b
}

/// `n` cores × 2 threads, one package, L1d/L1i/L2 per core and one L3.
fn processors(n: u64) -> Vec<u8> {
    let rec = |rel: u32, size: u32| {
        let mut r = vec![0u8; size as usize];
        if let Some(s) = r.get_mut(0..4) {
            s.copy_from_slice(&rel.to_le_bytes());
        }
        if let Some(s) = r.get_mut(4..8) {
            s.copy_from_slice(&size.to_le_bytes());
        }
        r
    };
    let set = |r: &mut Vec<u8>, at: usize, v: &[u8]| {
        if let Some(s) = r.get_mut(at..at + v.len()) {
            s.copy_from_slice(v);
        }
    };
    let mut b = Vec::new();
    let all: u64 = if n >= 32 {
        u64::MAX
    } else {
        (1u64 << (2 * n)) - 1
    };
    for c in 0..n {
        let m = 0b11u64 << (2 * c);
        let mut core = rec(0, 48);
        set(&mut core, 8, &[1]);
        set(&mut core, 30, &1u16.to_le_bytes());
        set(&mut core, 32, &m.to_le_bytes());
        b.extend(core);
        for (level, kind, size) in [
            (1u8, 2u32, 48u32 << 10),
            (1, 1, 32 << 10),
            (2, 0, 1280 << 10),
        ] {
            let mut c = rec(2, 56);
            set(&mut c, 8, &[level]);
            set(&mut c, 12, &size.to_le_bytes());
            set(&mut c, 16, &kind.to_le_bytes());
            set(&mut c, 38, &1u16.to_le_bytes());
            set(&mut c, 40, &m.to_le_bytes());
            b.extend(c);
        }
    }
    let mut l3 = rec(2, 56);
    set(&mut l3, 8, &[3]);
    set(&mut l3, 12, &(12u32 << 20).to_le_bytes());
    set(&mut l3, 38, &1u16.to_le_bytes());
    set(&mut l3, 40, &all.to_le_bytes());
    b.extend(l3);
    let mut pkg = rec(3, 48);
    set(&mut pkg, 30, &1u16.to_le_bytes());
    set(&mut pkg, 32, &all.to_le_bytes());
    b.extend(pkg);
    b
}

fn nvme_response(temp_k: u16) -> Vec<u8> {
    let mut out = vec![0u8; layout::NVME_HEALTH_OUT_LEN];
    let mut set = |at: usize, v: &[u8]| {
        if let Some(s) = out.get_mut(at..at + v.len()) {
            s.copy_from_slice(v);
        }
    };
    set(24, &40u32.to_le_bytes());
    set(28, &512u32.to_le_bytes());
    set(48 + 1, &temp_k.to_le_bytes());
    set(48 + 3, &[100]);
    set(48 + 5, &[3]);
    set(48 + 128, &1234u64.to_le_bytes());
    out
}

pub fn laptop() -> MemWin {
    let board = StructBuilder::new(2, 0x02, 0x08)
        .string(0x04, "LENOVO")
        .string(0x05, "21K5")
        .build();
    let bios = StructBuilder::new(0, 0x00, 0x12)
        .string(0x04, "LENOVO")
        .string(0x05, "R2AET45W (1.20)")
        .string(0x08, "05/14/2024")
        .build();
    let mut w = MemWin {
        smbios: Ok(rsmb(&table(&[
            bios,
            board,
            memory_array(0x10, 64 << 30, 2),
            memory_device(
                0x11,
                0x10,
                &Dimm {
                    size_mib: 32768,
                    form_factor: 0x0D,
                    mem_type: 0x22,
                    speed: 5600,
                    configured: 5600,
                    locator: "DIMM 0",
                    bank: "BANK 0",
                    manufacturer: "Samsung",
                    serial: "00000001",
                    part: "M425R4GA3BB0-CWMOL",
                    ranks: 1,
                    voltage_mv: 1100,
                },
            ),
            memory_device(
                0x12,
                0x10,
                &Dimm {
                    size_mib: 32768,
                    form_factor: 0x0D,
                    mem_type: 0x22,
                    speed: 5600,
                    configured: 5600,
                    locator: "DIMM 1",
                    bank: "BANK 1",
                    manufacturer: "Samsung",
                    serial: "00000001",
                    part: "M425R4GA3BB0-CWMOL",
                    ranks: 1,
                    voltage_mv: 1100,
                },
            ),
        ]))),
        os_version: Ok(OsVersionRaw {
            product_name: Some("Windows 10 Pro".into()),
            display_version: Some("24H2".into()),
            current_build: Some(26100),
            ubr: Some(2314),
            edition_id: Some("Professional".into()),
        }),
        computer_name: Ok("LAPTOP-SAMPLE".into()),
        uptime_ms: Ok(3_600_000),
        memory_status: Ok(MemoryStatusRaw {
            total_phys: 32 << 30,
            avail_phys: 20 << 30,
        }),
        processors: Ok(processors(8)),
        cpu_registry: Ok(CpuRegistryRaw {
            name: Some("AMD Ryzen 7 PRO 7840U w/ Radeon 780M Graphics".into()),
            vendor: Some("AuthenticAMD".into()),
            mhz: Some(3301),
            update_revision: Some(vec![0x09, 0x10, 0x40, 0x0A, 0, 0, 0, 0]),
        }),
        cpu_features: Ok(vec!["sse4.2".into(), "avx2".into(), "avx512f".into()]),
        perf: vec![Ok(Vec::new()), Ok((0..16).map(|c| (c, 120.0)).collect())],
        disks_wmi: Ok(vec![PhysicalDiskRaw {
            number: 0,
            friendly_name: Some("SAMSUNG MZVL21T0HCLR-00BL7".into()),
            serial: Some("S641NX0T000000".into()),
            firmware: Some("6L2QGXD7".into()),
            size: Some(1_024_209_543_168),
            bus_type: Some(17),
            media_type: Some(4),
        }]),
        dxgi: Ok(vec![DxgiAdapterRaw {
            description: "AMD Radeon(TM) 780M".into(),
            vendor_id: 0x1002,
            device_id: 0x15bf,
            dedicated_video_memory: 512 << 20,
            shared_system_memory: 16 << 30,
            luid: 0x1_0000,
            software: false,
        }]),
        pci_display: Ok(vec![PciDisplayRaw {
            vendor_id: 0x1002,
            device_id: 0x15bf,
            driver_version: Some("31.0.24033.1003".into()),
            link_speed: Some(4),
            link_width: Some(16),
            location: Some("PCI bus 100, device 0, function 0".into()),
        }]),
        net: Ok(vec![NetAdapterRaw {
            name: "Wi-Fi".into(),
            description: Some("MEDIATEK Wi-Fi 6E MT7922".into()),
            if_type: 71,
            up: true,
            speed_bps: Some(1_201_000_000),
            mac: Some("10:20:30:40:50:60".into()),
            mtu: Some(1500),
            hardware: true,
            filter: false,
        }]),
        batteries: Ok(vec![BatteryRaw {
            name: Some("5B10W51867".into()),
            manufacturer: Some("SMP".into()),
            serial: Some("1234".into()),
            chemistry: Some("LiP".into()),
            relative: false,
            designed: Some(52_500),
            full: Some(49_870),
            now: Some(30_000),
            cycles: Some(112),
            power_state: Some(2),
        }]),
        ac_online: Ok(Some(false)),
        thermal: Ok(vec![ThermalZoneRaw {
            instance: "ACPI\\ThermalZone\\THM0_0".into(),
            tenths_kelvin: 3182,
        }]),
        elevated: true,
        ..MemWin::default()
    };
    w.nvme.insert(0, Ok(nvme_response(318)));
    w.gpu_temps.insert(0x1_0000, Ok(Some(480)));
    w
}

pub fn vm() -> MemWin {
    MemWin {
        smbios: Ok(rsmb(&table(&[StructBuilder::new(2, 0x02, 0x08)
            .string(0x04, "Microsoft Corporation")
            .string(0x05, "Virtual Machine")
            .build()]))),
        os_version: Ok(OsVersionRaw {
            product_name: Some("Windows Server 2025 Datacenter".into()),
            display_version: Some("24H2".into()),
            current_build: Some(26100),
            ubr: Some(1742),
            edition_id: Some("ServerDatacenter".into()),
        }),
        computer_name: Ok("WIN-VM".into()),
        uptime_ms: Ok(120_000),
        memory_status: Ok(MemoryStatusRaw {
            total_phys: 8 << 30,
            avail_phys: 6 << 30,
        }),
        processors: Ok(processors(2)),
        cpu_registry: Ok(CpuRegistryRaw {
            name: Some("Intel(R) Xeon(R) Platinum 8370C CPU @ 2.80GHz".into()),
            vendor: Some("GenuineIntel".into()),
            mhz: Some(2794),
            update_revision: Some(vec![0; 8]),
        }),
        cpu_features: Ok(vec!["sse4.2".into(), "avx2".into()]),
        perf: vec![Ok(Vec::new())],
        disks_wmi: Ok(vec![PhysicalDiskRaw {
            number: 0,
            friendly_name: Some("Msft Virtual Disk".into()),
            serial: None,
            firmware: Some("1.0".into()),
            size: Some(136_365_211_648),
            bus_type: Some(14),
            media_type: Some(0),
        }]),
        dxgi: Ok(vec![DxgiAdapterRaw {
            description: "Microsoft Basic Render Driver".into(),
            vendor_id: 0x1414,
            device_id: 0x8c,
            dedicated_video_memory: 0,
            shared_system_memory: 4 << 30,
            luid: 1,
            software: true,
        }]),
        net: Ok(vec![NetAdapterRaw {
            name: "Ethernet".into(),
            description: Some("Microsoft Hyper-V Network Adapter".into()),
            if_type: 6,
            up: true,
            speed_bps: Some(10_000_000_000),
            mac: Some("00:15:5d:00:00:01".into()),
            mtu: Some(1500),
            hardware: true,
            filter: false,
        }]),
        batteries: Ok(Vec::new()),
        ac_online: Ok(Some(true)),
        thermal: Err(WinError::NotAvailable),
        elevated: true,
        ..MemWin::default()
    }
}
