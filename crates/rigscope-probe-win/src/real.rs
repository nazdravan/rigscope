//! The live Windows source: forwards to `rigscope_win::ffi`.

use crate::{MemWin, R, WinSource, mask_fixture};
use rigscope_win::{ffi, *};

pub struct RealWin {
    /// `None` when the PDH query could not be opened; `cpu_performance` then reports it.
    pdh: Option<ffi::Pdh>,
}

impl RealWin {
    pub fn new() -> RealWin {
        RealWin {
            pdh: ffi::Pdh::new().ok(),
        }
    }
}

impl Default for RealWin {
    fn default() -> Self {
        Self::new()
    }
}

impl WinSource for RealWin {
    fn smbios_raw(&self) -> R<Vec<u8>> {
        ffi::smbios_raw()
    }
    fn os_version(&self) -> R<OsVersionRaw> {
        ffi::os_version()
    }
    fn computer_name(&self) -> R<String> {
        ffi::computer_name()
    }
    fn uptime_ms(&self) -> R<u64> {
        Ok(ffi::uptime_ms())
    }
    fn memory_status(&self) -> R<MemoryStatusRaw> {
        ffi::memory_status()
    }
    fn processors(&self) -> R<Vec<u8>> {
        ffi::processors_raw()
    }
    fn cpu_registry(&self) -> R<CpuRegistryRaw> {
        ffi::cpu_registry()
    }
    fn cpu_features(&self) -> R<Vec<String>> {
        Ok(ffi::cpu_features())
    }
    fn cpu_performance(&mut self) -> R<Vec<(u32, f64)>> {
        match self.pdh.as_mut() {
            Some(p) => p.sample(),
            None => Err(WinError::NotAvailable),
        }
    }
    fn physical_disks_wmi(&self) -> R<Vec<PhysicalDiskRaw>> {
        ffi::physical_disks_wmi()
    }
    fn physical_disks_ioctl(&self) -> R<Vec<DiskIoctlRaw>> {
        ffi::physical_disks_ioctl()
    }
    fn nvme_health_log(&self, disk: u32) -> R<Vec<u8>> {
        ffi::nvme_health_raw(disk)
    }
    fn ata_smart(&self, disk: u32) -> R<AtaSmartRaw> {
        ffi::ata_smart_raw(disk)
    }
    fn adapters_dxgi(&self) -> R<Vec<DxgiAdapterRaw>> {
        ffi::adapters_dxgi()
    }
    fn pci_display(&self) -> R<Vec<PciDisplayRaw>> {
        ffi::pci_display()
    }
    fn gpu_temperature(&mut self, luid: u64) -> R<Option<u32>> {
        ffi::GpuPerf::temperature(luid)
    }
    fn net_adapters(&self) -> R<Vec<NetAdapterRaw>> {
        ffi::net_adapters()
    }
    fn batteries(&self) -> R<Vec<BatteryRaw>> {
        ffi::batteries()
    }
    fn ac_online(&self) -> R<Option<bool>> {
        ffi::ac_online()
    }
    fn acpi_thermal_zones(&self) -> R<Vec<ThermalZoneRaw>> {
        ffi::acpi_thermal_zones()
    }
    fn is_elevated(&self) -> bool {
        ffi::is_elevated()
    }
}

/// Every value `RealWin` returns, as a `MemWin` (for `--capture-fixture`), masked by
/// [`mask_fixture`].
pub fn capture(real: &mut RealWin) -> MemWin {
    let mut w = MemWin {
        smbios: real.smbios_raw(),
        os_version: real.os_version(),
        computer_name: real.computer_name(),
        uptime_ms: real.uptime_ms(),
        memory_status: real.memory_status(),
        processors: real.processors(),
        cpu_registry: real.cpu_registry(),
        cpu_features: real.cpu_features(),
        // The first PDH sample has no data; the second, half a second later, has.
        perf: vec![real.cpu_performance(), {
            std::thread::sleep(std::time::Duration::from_millis(500));
            real.cpu_performance()
        }],
        disks_wmi: real.physical_disks_wmi(),
        disks_ioctl: real.physical_disks_ioctl(),
        dxgi: real.adapters_dxgi(),
        pci_display: real.pci_display(),
        net: real.net_adapters(),
        batteries: real.batteries(),
        ac_online: real.ac_online(),
        thermal: real.acpi_thermal_zones(),
        elevated: real.is_elevated(),
        ..MemWin::default()
    };
    // The disks the collector will ask SMART for: WMI's list, or the IOCTL one when WMI fails.
    let disks: Vec<(u32, Option<u16>)> = match (&w.disks_wmi, &w.disks_ioctl) {
        (Ok(v), _) => v.iter().map(|d| (d.number, d.bus_type)).collect(),
        (Err(_), Ok(v)) => v
            .iter()
            .map(|d| {
                let bus = rigscope_win::layout::parse_device_descriptor(&d.descriptor)
                    .and_then(|p| p.bus_type);
                (d.number, bus)
            })
            .collect(),
        (Err(_), Err(_)) => Vec::new(),
    };
    for (n, bus) in disks {
        // BusType 17 = NVMe.
        if bus == Some(17) {
            w.nvme.insert(n, real.nvme_health_log(n));
        } else {
            w.ata.insert(n, real.ata_smart(n));
        }
    }
    let luids: Vec<u64> = w
        .dxgi
        .as_ref()
        .map(|v| v.iter().filter(|a| !a.software).map(|a| a.luid).collect())
        .unwrap_or_default();
    for l in luids {
        w.gpu_temps.insert(l, real.gpu_temperature(l));
    }
    mask_fixture(w)
}
