use crate::{field_err, WinSource};
use rigscope_model::{clean_text, Field, Gpu, GpuInfo};
use rigscope_win::DxgiAdapterRaw;

pub const MAX_GPUS: usize = 16;

/// GT/s per `DEVPKEY_PciDevice_CurrentLinkSpeed` code.
fn gts(code: u32) -> Option<f64> {
    Some(match code {
        1 => 2.5,
        2 => 5.0,
        3 => 8.0,
        4 => 16.0,
        5 => 32.0,
        6 => 64.0,
        _ => return None,
    })
}

/// Same text as the Linux probe (`{speed} x{width}`, speed like `16.0 GT/s PCIe`).
pub(crate) fn link_text(gts: f64, width: u32) -> String {
    format!("{gts:.1} GT/s PCIe x{width}") // ajustează după Step 1 dacă Linux scrie altfel
}

fn hardware(src: &dyn WinSource) -> Vec<DxgiAdapterRaw> {
    let mut out: Vec<DxgiAdapterRaw> = Vec::new();
    for a in src.adapters_dxgi().unwrap_or_default() {
        if a.software || out.iter().any(|b| b.luid == a.luid) {
            continue;
        }
        out.push(a);
        if out.len() == MAX_GPUS {
            break;
        }
    }
    out
}

pub(crate) fn gpu_luids(src: &dyn WinSource) -> Vec<(String, u64)> {
    hardware(src)
        .iter()
        .enumerate()
        .map(|(i, a)| (format!("gpu{i}"), a.luid))
        .collect()
}

pub(crate) fn read_gpus(src: &dyn WinSource) -> GpuInfo {
    let pci = src.pci_display();
    let gpus = hardware(src)
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let vendor = u16::try_from(a.vendor_id).ok();
            let device = u16::try_from(a.device_id).ok();
            let dev = pci.as_ref().ok().and_then(|p| {
                p.iter()
                    .find(|d| Some(d.vendor_id) == vendor && Some(d.device_id) == device)
            });
            Gpu {
                card: format!("gpu{i}"),
                pci_address: Field::from_option(dev.and_then(|d| d.location.clone())),
                vendor_id: Field::from_option(vendor),
                device_id: Field::from_option(device),
                vendor_name: Field::Unknown,
                device_name: Field::from_option(clean_text(&a.description)),
                subsystem_name: Field::Unknown,
                driver: match (&pci, dev) {
                    (_, Some(d)) => Field::from_option(d.driver_version.clone()),
                    (Err(e), None) => field_err(e),
                    (Ok(_), None) => Field::Unknown,
                },
                vram_bytes: Field::Known(a.dedicated_video_memory),
                pcie_link: Field::from_option(
                    dev.and_then(|d| Some(link_text(gts(d.link_speed?)?, d.link_width?))),
                ),
                // DXGI enumerates the adapter driving the primary output first.
                boot_vga: i == 0,
            }
        })
        .collect();
    GpuInfo { gpus }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::field_reassign_with_default
    )]
    use super::*;
    use crate::MemWin;
    use rigscope_win::{DxgiAdapterRaw, PciDisplayRaw};

    pub(crate) fn rtx() -> DxgiAdapterRaw {
        DxgiAdapterRaw {
            description: "NVIDIA GeForce RTX 3060".into(),
            vendor_id: 0x10de,
            device_id: 0x2504,
            dedicated_video_memory: 12 << 30,
            shared_system_memory: 8 << 30,
            luid: 0x0000_0000_0001_2345,
            software: false,
        }
    }

    #[test]
    fn hardware_adapters_with_exact_vram_and_link() {
        let mut w = MemWin::default();
        let basic = DxgiAdapterRaw {
            description: "Microsoft Basic Render Driver".into(),
            vendor_id: 0x1414,
            device_id: 0x8c,
            software: true,
            luid: 9,
            ..Default::default()
        };
        w.dxgi = Ok(vec![rtx(), basic, rtx()]); // rtx de două ori: același LUID, deci deduplicat
        w.pci_display = Ok(vec![PciDisplayRaw {
            vendor_id: 0x10de,
            device_id: 0x2504,
            driver_version: Some("32.0.15.6094".into()),
            link_speed: Some(4),
            link_width: Some(16),
            location: Some("PCI bus 1, device 0, function 0".into()),
        }]);
        let g = read_gpus(&w);
        assert_eq!(g.gpus.len(), 1);
        let x = &g.gpus[0];
        assert_eq!(x.card, "gpu0");
        assert_eq!(
            x.device_name,
            Field::Known("NVIDIA GeForce RTX 3060".into())
        );
        assert_eq!(x.vram_bytes, Field::Known(12 << 30)); // peste 4 GiB, exact
        assert_eq!(x.pci_id().as_deref(), Some("10de:2504"));
        assert_eq!(x.driver, Field::Known("32.0.15.6094".into()));
        assert_eq!(x.pcie_link, Field::Known(link_text(16.0, 16)));
        assert_eq!(x.pcie_link, Field::Known("16.0 GT/s PCIe x16".into())); // literal: Linux format
        assert!(x.boot_vga);
    }

    #[test]
    fn luids_follow_gpu_order() {
        let mut w = MemWin::default();
        let basic = DxgiAdapterRaw {
            software: true,
            luid: 9,
            ..Default::default()
        };
        w.dxgi = Ok(vec![basic, rtx()]);
        assert_eq!(gpu_luids(&w), vec![("gpu0".to_string(), 0x12345)]);
    }

    #[test]
    fn integrated_gpu_has_shared_memory_note() {
        let mut w = MemWin::default();
        w.dxgi = Ok(vec![DxgiAdapterRaw {
            description: "Intel(R) UHD Graphics 730".into(),
            vendor_id: 0x8086,
            device_id: 0x4692,
            dedicated_video_memory: 128 << 20,
            shared_system_memory: 8 << 30,
            luid: 1,
            software: false,
        }]);
        let g = read_gpus(&w);
        assert_eq!(g.gpus[0].vram_bytes, Field::Known(128 << 20));
        assert_eq!(g.gpus[0].pcie_link, Field::Unknown);
    }
}
