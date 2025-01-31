use crate::Field;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GpuInfo {
    pub gpus: Vec<Gpu>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gpu {
    /// DRM card: `card0`.
    pub card: String,
    /// PCI address: `0000:01:00.0`.
    pub pci_address: Field<String>,
    pub vendor_id: Field<u16>,
    pub device_id: Field<u16>,
    /// From pci.ids, e.g. `NVIDIA Corporation`.
    pub vendor_name: Field<String>,
    /// From pci.ids, e.g. `GA104 [GeForce RTX 3070]`.
    pub device_name: Field<String>,
    /// Board maker's variant from pci.ids, e.g. `TUF RTX 3070`.
    pub subsystem_name: Field<String>,
    /// Kernel driver: `nvidia`, `amdgpu`, `i915`.
    pub driver: Field<String>,
    pub vram_bytes: Field<u64>,
    /// Current PCIe link, e.g. `16.0 GT/s PCIe x16`.
    pub pcie_link: Field<String>,
    /// The firmware used this card at boot (the primary display).
    pub boot_vga: bool,
}

impl Gpu {
    /// `10de:2484`, shown when pci.ids has no name.
    pub fn pci_id(&self) -> Option<String> {
        Some(format!(
            "{:04x}:{:04x}",
            self.vendor_id.known()?,
            self.device_id.known()?
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pci_id_formatting() {
        let g = Gpu {
            vendor_id: Field::Known(0x10de),
            device_id: Field::Known(0x2484),
            ..Default::default()
        };
        assert_eq!(g.pci_id().as_deref(), Some("10de:2484"));
        assert_eq!(Gpu::default().pci_id(), None);
    }
}
