use rigscope_win::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub type R<T> = Result<T, WinError>;

/// Raw Windows data. `RealWin` (Windows only) calls the OS; [`MemWin`] replays values.
pub trait WinSource {
    fn smbios_raw(&self) -> R<Vec<u8>>;
    fn os_version(&self) -> R<OsVersionRaw>;
    fn computer_name(&self) -> R<String>;
    fn uptime_ms(&self) -> R<u64>;
    fn memory_status(&self) -> R<MemoryStatusRaw>;
    /// `GetLogicalProcessorInformationEx(RelationAll)` buffer.
    fn processors(&self) -> R<Vec<u8>>;
    fn cpu_registry(&self) -> R<CpuRegistryRaw>;
    fn cpu_features(&self) -> R<Vec<String>>;
    /// `(logical cpu, % processor performance)`; the first call after creation has no data.
    fn cpu_performance(&mut self) -> R<Vec<(u32, f64)>>;
    fn physical_disks_wmi(&self) -> R<Vec<PhysicalDiskRaw>>;
    fn physical_disks_ioctl(&self) -> R<Vec<DiskIoctlRaw>>;
    /// The raw `STORAGE_PROTOCOL_DATA_DESCRIPTOR` response (see `layout::nvme_log_from_response`).
    fn nvme_health_log(&self, disk: u32) -> R<Vec<u8>>;
    fn ata_smart(&self, disk: u32) -> R<AtaSmartRaw>;
    fn adapters_dxgi(&self) -> R<Vec<DxgiAdapterRaw>>;
    fn pci_display(&self) -> R<Vec<PciDisplayRaw>>;
    /// Deci-degrees C (`D3DKMT_ADAPTER_PERFDATA.Temperature`); `None` = the driver reports none.
    fn gpu_temperature(&mut self, luid: u64) -> R<Option<u32>>;
    fn net_adapters(&self) -> R<Vec<NetAdapterRaw>>;
    fn batteries(&self) -> R<Vec<BatteryRaw>>;
    fn ac_online(&self) -> R<Option<bool>>;
    fn acpi_thermal_zones(&self) -> R<Vec<ThermalZoneRaw>>;
    fn is_elevated(&self) -> bool;
}

fn na<T>() -> R<T> {
    Err(WinError::NotAvailable)
}

/// In-memory values: tests, sample machines and fixture directories. Everything defaults to
/// `NotAvailable` (or empty), so a test sets only what it checks.
/// `#[serde(default)]` on the struct fills missing JSON keys from `MemWin::default()`; the
/// binary parts live in their own files (see `save`/`load`) and are skipped in the JSON.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemWin {
    #[serde(skip, default = "na")]
    pub smbios: R<Vec<u8>>,
    pub os_version: R<OsVersionRaw>,
    pub computer_name: R<String>,
    pub uptime_ms: R<u64>,
    pub memory_status: R<MemoryStatusRaw>,
    #[serde(skip, default = "na")]
    pub processors: R<Vec<u8>>,
    pub cpu_registry: R<CpuRegistryRaw>,
    pub cpu_features: R<Vec<String>>,
    /// One entry per `cpu_performance` call; the last one repeats.
    pub perf: Vec<R<Vec<(u32, f64)>>>,
    /// Replay position; 0 in both a fresh sample and a loaded fixture, so equality holds.
    #[serde(skip)]
    pub perf_calls: usize,
    pub disks_wmi: R<Vec<PhysicalDiskRaw>>,
    pub disks_ioctl: R<Vec<DiskIoctlRaw>>,
    #[serde(skip)]
    pub nvme: BTreeMap<u32, R<Vec<u8>>>,
    #[serde(skip)]
    pub ata: BTreeMap<u32, R<AtaSmartRaw>>,
    pub dxgi: R<Vec<DxgiAdapterRaw>>,
    pub pci_display: R<Vec<PciDisplayRaw>>,
    pub gpu_temps: BTreeMap<u64, R<Option<u32>>>,
    pub net: R<Vec<NetAdapterRaw>>,
    pub batteries: R<Vec<BatteryRaw>>,
    pub ac_online: R<Option<bool>>,
    pub thermal: R<Vec<ThermalZoneRaw>>,
    pub elevated: bool,
}

impl Default for MemWin {
    fn default() -> Self {
        MemWin {
            smbios: na(),
            os_version: na(),
            computer_name: na(),
            uptime_ms: na(),
            memory_status: na(),
            processors: na(),
            cpu_registry: na(),
            cpu_features: na(),
            perf: Vec::new(),
            perf_calls: 0,
            disks_wmi: Ok(Vec::new()),
            disks_ioctl: Ok(Vec::new()),
            nvme: BTreeMap::new(),
            ata: BTreeMap::new(),
            dxgi: Ok(Vec::new()),
            pci_display: Ok(Vec::new()),
            gpu_temps: BTreeMap::new(),
            net: Ok(Vec::new()),
            batteries: Ok(Vec::new()),
            ac_online: Ok(None),
            thermal: Ok(Vec::new()),
            elevated: true,
        }
    }
}

const MARKER: &str = "winsource.json";
const FORMAT: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Envelope {
    format: u32,
    data: MemWin,
}

fn io_err(e: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
}

/// A binary file, or the error recorded instead of it (`<name>.err.json`).
fn save_bin(dir: &Path, name: &str, v: &R<Vec<u8>>) -> std::io::Result<()> {
    match v {
        Ok(b) => std::fs::write(dir.join(name), b),
        Err(e) => std::fs::write(
            dir.join(format!("{name}.err.json")),
            serde_json::to_vec(e).map_err(io_err)?,
        ),
    }
}

fn load_bin(dir: &Path, name: &str) -> std::io::Result<R<Vec<u8>>> {
    let err = dir.join(format!("{name}.err.json"));
    if err.exists() {
        let e: WinError = serde_json::from_slice(&std::fs::read(err)?).map_err(io_err)?;
        return Ok(Err(e));
    }
    let p = dir.join(name);
    if !p.exists() {
        return Ok(na());
    }
    let b = std::fs::read(&p)?;
    if b.len() > 4 * 1024 * 1024 {
        return Err(io_err(format!("{name}: larger than 4 MiB")));
    }
    Ok(Ok(b))
}

impl MemWin {
    pub fn is_fixture(dir: &Path) -> bool {
        dir.join(MARKER).is_file()
    }

    /// Removes what an earlier `save` left, so stale `*.err.json` or `ioctl/*` files
    /// cannot win over new data on `load`. Unrelated files are kept.
    fn clear_managed(dir: &Path) -> std::io::Result<()> {
        for name in [
            MARKER,
            "smbios.bin",
            "smbios.bin.err.json",
            "processors.bin",
            "processors.bin.err.json",
        ] {
            match std::fs::remove_file(dir.join(name)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        match std::fs::remove_dir_all(dir.join("ioctl")) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        Self::clear_managed(dir)?;
        std::fs::create_dir_all(dir)?;
        if !self.nvme.is_empty() || !self.ata.is_empty() {
            std::fs::create_dir_all(dir.join("ioctl"))?;
        }
        let env = Envelope {
            format: FORMAT,
            data: self.clone(),
        };
        std::fs::write(
            dir.join(MARKER),
            serde_json::to_vec_pretty(&env).map_err(io_err)?,
        )?;
        save_bin(dir, "smbios.bin", &self.smbios)?;
        save_bin(dir, "processors.bin", &self.processors)?;
        for (n, v) in &self.nvme {
            save_bin(dir, &format!("ioctl/nvme{n}.bin"), v)?;
        }
        for (n, v) in &self.ata {
            let attr = v
                .as_ref()
                .map(|a| a.attributes.clone())
                .map_err(Clone::clone);
            let thr = v
                .as_ref()
                .map(|a| a.thresholds.clone())
                .map_err(Clone::clone);
            save_bin(dir, &format!("ioctl/ata{n}-attr.bin"), &attr)?;
            save_bin(dir, &format!("ioctl/ata{n}-thr.bin"), &thr)?;
        }
        Ok(())
    }

    pub fn load(dir: &Path) -> std::io::Result<MemWin> {
        let env: Envelope =
            serde_json::from_slice(&std::fs::read(dir.join(MARKER))?).map_err(io_err)?;
        if env.format != FORMAT {
            return Err(io_err(format!(
                "fixture format {} (expected {FORMAT})",
                env.format
            )));
        }
        let mut w = env.data;
        w.smbios = load_bin(dir, "smbios.bin")?;
        w.processors = load_bin(dir, "processors.bin")?;
        if let Ok(rd) = std::fs::read_dir(dir.join("ioctl")) {
            for e in rd {
                let name = e?.file_name().to_string_lossy().into_owned();
                let stem = name.trim_end_matches(".err.json").trim_end_matches(".bin");
                if let Some(n) = stem.strip_prefix("nvme").and_then(|n| n.parse().ok()) {
                    w.nvme
                        .insert(n, load_bin(dir, &format!("ioctl/nvme{n}.bin"))?);
                } else if let Some(n) = stem
                    .strip_prefix("ata")
                    .and_then(|r| r.strip_suffix("-attr"))
                    .and_then(|n| n.parse::<u32>().ok())
                {
                    let attr = load_bin(dir, &format!("ioctl/ata{n}-attr.bin"))?;
                    let thr = load_bin(dir, &format!("ioctl/ata{n}-thr.bin"))?;
                    w.ata.insert(
                        n,
                        attr.and_then(|a| {
                            thr.map(|t| AtaSmartRaw {
                                attributes: a,
                                thresholds: t,
                            })
                        }),
                    );
                }
            }
        }
        Ok(w)
    }
}

impl WinSource for MemWin {
    fn smbios_raw(&self) -> R<Vec<u8>> {
        self.smbios.clone()
    }
    fn os_version(&self) -> R<OsVersionRaw> {
        self.os_version.clone()
    }
    fn computer_name(&self) -> R<String> {
        self.computer_name.clone()
    }
    fn uptime_ms(&self) -> R<u64> {
        self.uptime_ms.clone()
    }
    fn memory_status(&self) -> R<MemoryStatusRaw> {
        self.memory_status.clone()
    }
    fn processors(&self) -> R<Vec<u8>> {
        self.processors.clone()
    }
    fn cpu_registry(&self) -> R<CpuRegistryRaw> {
        self.cpu_registry.clone()
    }
    fn cpu_features(&self) -> R<Vec<String>> {
        self.cpu_features.clone()
    }
    fn cpu_performance(&mut self) -> R<Vec<(u32, f64)>> {
        let i = self.perf_calls.min(self.perf.len().saturating_sub(1));
        self.perf_calls += 1;
        self.perf.get(i).cloned().unwrap_or_else(na)
    }
    fn physical_disks_wmi(&self) -> R<Vec<PhysicalDiskRaw>> {
        self.disks_wmi.clone()
    }
    fn physical_disks_ioctl(&self) -> R<Vec<DiskIoctlRaw>> {
        self.disks_ioctl.clone()
    }
    fn nvme_health_log(&self, disk: u32) -> R<Vec<u8>> {
        self.nvme.get(&disk).cloned().unwrap_or_else(na)
    }
    fn ata_smart(&self, disk: u32) -> R<AtaSmartRaw> {
        self.ata.get(&disk).cloned().unwrap_or_else(na)
    }
    fn adapters_dxgi(&self) -> R<Vec<DxgiAdapterRaw>> {
        self.dxgi.clone()
    }
    fn pci_display(&self) -> R<Vec<PciDisplayRaw>> {
        self.pci_display.clone()
    }
    fn gpu_temperature(&mut self, luid: u64) -> R<Option<u32>> {
        self.gpu_temps.get(&luid).cloned().unwrap_or(Ok(None))
    }
    fn net_adapters(&self) -> R<Vec<NetAdapterRaw>> {
        self.net.clone()
    }
    fn batteries(&self) -> R<Vec<BatteryRaw>> {
        self.batteries.clone()
    }
    fn ac_online(&self) -> R<Option<bool>> {
        self.ac_online.clone()
    }
    fn acpi_thermal_zones(&self) -> R<Vec<ThermalZoneRaw>> {
        self.thermal.clone()
    }
    fn is_elevated(&self) -> bool {
        self.elevated
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::field_reassign_with_default)]
    use super::*;

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!("rigscope-memwin-{}", std::process::id()));
        let mut w = MemWin::default();
        w.smbios = Ok(vec![0, 3, 4, 0, 0, 0, 0, 0]);
        w.computer_name = Ok("PC".into());
        w.nvme.insert(0, Ok(vec![7; 512]));
        w.os_version = Err(WinError::Timeout);
        w.save(&dir).unwrap();
        assert!(MemWin::is_fixture(&dir));
        let back = MemWin::load(&dir).unwrap();
        assert_eq!(back, w);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn save_replaces_stale_files() {
        let dir =
            std::env::temp_dir().join(format!("rigscope-memwin-stale-{}", std::process::id()));
        let mut w = MemWin::default();
        w.smbios = Err(WinError::AccessDenied);
        w.nvme.insert(0, Ok(vec![1; 8]));
        w.save(&dir).unwrap();
        std::fs::write(dir.join("keep.txt"), b"x").unwrap();
        w.smbios = Ok(vec![0, 3, 4, 0, 0, 0, 0, 0]);
        w.nvme.clear();
        w.save(&dir).unwrap();
        let back = MemWin::load(&dir).unwrap();
        assert!(back.smbios.is_ok());
        assert!(back.nvme.is_empty());
        assert!(dir.join("keep.txt").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
