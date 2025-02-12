//! Small helpers shared by the sysfs readers.

use crate::Source;

/// File contents, trimmed; `None` when unreadable.
pub fn text(src: &dyn Source, path: &str) -> Option<String> {
    src.read(path)
        .ok()
        .map(|b| String::from_utf8_lossy(&b).trim().to_string())
}

/// `0x10de` → `0x10de_u16`.
pub fn hex_id(s: Option<String>) -> Option<u16> {
    u16::from_str_radix(s?.trim().trim_start_matches("0x"), 16).ok()
}

pub fn basename(path: &str) -> Option<&str> {
    path.rsplit('/').next().filter(|s| !s.is_empty())
}

/// Where distributions install the PCI id database (hwdata / pciutils).
const PCI_IDS_PATHS: &[&str] = &[
    "/usr/share/hwdata/pci.ids",
    "/usr/share/misc/pci.ids",
    "/usr/share/pci.ids",
];

pub fn pci_ids(src: &dyn Source) -> Option<String> {
    PCI_IDS_PATHS.iter().find_map(|p| text(src, p))
}

/// Name of the driver bound to a device directory (the `driver` symlink's target).
pub fn driver(src: &dyn Source, device_dir: &str) -> Option<String> {
    let path = src.canonical(&format!("{device_dir}/driver")).ok()?;
    basename(&path).map(str::to_string)
}
