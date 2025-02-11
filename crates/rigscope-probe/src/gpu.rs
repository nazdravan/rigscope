//! Graphics cards from `/sys/class/drm` (no root), named through the system's pci.ids.

use crate::Source;
use rigscope_model::{clean_text, Field, Gpu, GpuInfo};
use rigscope_procfs::pci_ids;

pub const DRM_DIR: &str = "/sys/class/drm";
/// Most cards listed.
pub const MAX_GPUS: usize = 32;

pub const NO_PCI_IDS: &str = "pci.ids not installed (hwdata package)";
pub const NVIDIA_NO_VRAM: &str = "not exposed by the nvidia driver";
pub const SHARED_MEMORY: &str = "uses system memory";

use crate::sysfs::{basename, hex_id, text};

/// `0000:01:00.0`
fn is_pci_address(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 12
        && b.get(4) == Some(&b':')
        && b.get(7) == Some(&b':')
        && b.get(10) == Some(&b'.')
        && s.chars()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7 | 10) || c.is_ascii_hexdigit())
}

/// `card0`, not `card0-HDMI-A-1` (an output) or `renderD128`.
fn is_card(name: &str) -> bool {
    name.strip_prefix("card")
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

pub fn read_gpus(src: &dyn Source) -> GpuInfo {
    let Ok(entries) = src.list(DRM_DIR) else {
        return GpuInfo::default();
    };
    let ids = crate::sysfs::pci_ids(src);
    let gpus = entries
        .iter()
        .filter(|n| is_card(n))
        .take(MAX_GPUS)
        .map(|card| read_gpu(src, card, ids.as_deref()))
        .collect();
    GpuInfo { gpus }
}

fn read_gpu(src: &dyn Source, card: &str, ids: Option<&str>) -> Gpu {
    let dev = format!("{DRM_DIR}/{card}/device");
    let read = |f: &str| text(src, &format!("{dev}/{f}"));
    let vendor_id = hex_id(read("vendor"));
    let device_id = hex_id(read("device"));
    let subsystem = hex_id(read("subsystem_vendor")).zip(hex_id(read("subsystem_device")));
    let driver = crate::sysfs::driver(src, &dev);
    let pci_address = src.canonical(&dev).ok().and_then(|p| {
        basename(&p)
            .filter(|b| is_pci_address(b))
            .map(str::to_string)
    });

    let (vendor_name, device_name, subsystem_name) = match (ids, vendor_id, device_id) {
        (None, _, _) => {
            let u = || Field::Unavailable(NO_PCI_IDS.to_string());
            (u(), u(), u())
        }
        (Some(text), Some(v), Some(d)) => {
            let n = pci_ids::lookup(text, v, d, subsystem);
            (
                Field::from_option(n.vendor),
                Field::from_option(n.device),
                Field::from_option(n.subsystem),
            )
        }
        _ => (Field::Unknown, Field::Unknown, Field::Unknown),
    };

    let vram = match read("mem_info_vram_total").and_then(|v| v.parse::<u64>().ok()) {
        Some(bytes) => Field::Known(bytes),
        None => match driver.as_deref() {
            Some("nvidia") => Field::Unavailable(NVIDIA_NO_VRAM.to_string()),
            Some("i915" | "xe") => Field::Unavailable(SHARED_MEMORY.to_string()),
            _ => Field::Unknown,
        },
    };

    let pcie_link = match (
        read("current_link_speed").as_deref().and_then(clean_text),
        read("current_link_width"),
    ) {
        (Some(speed), Some(width)) if width.parse::<u8>().is_ok() => {
            Field::Known(format!("{speed} x{width}"))
        }
        _ => Field::Unknown,
    };

    Gpu {
        card: card.to_string(),
        pci_address: Field::from_option(pci_address),
        vendor_id: Field::from_option(vendor_id),
        device_id: Field::from_option(device_id),
        vendor_name,
        device_name,
        subsystem_name,
        driver: Field::from_option(driver),
        vram_bytes: vram,
        pcie_link,
        boot_vga: read("boot_vga").as_deref() == Some("1"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_names() {
        assert!(is_card("card0"));
        assert!(is_card("card12"));
        assert!(!is_card("card0-HDMI-A-1"));
        assert!(!is_card("renderD128"));
        assert!(!is_card("card"));
    }

    #[test]
    fn pci_addresses() {
        assert!(is_pci_address("0000:01:00.0"));
        assert!(!is_pci_address("device"));
        assert!(!is_pci_address("0000:01:00:0"));
    }
}
