//! Lookup in the `pci.ids` database (hwdata), read at run time from the system.

/// Names for one PCI function: vendor, device, and the subsystem (board maker's variant).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PciNames {
    pub vendor: Option<String>,
    pub device: Option<String>,
    pub subsystem: Option<String>,
}

/// Looks up `vendor:device` (and the `subsys_vendor subsys_device` line under it).
pub fn lookup(text: &str, vendor: u16, device: u16, subsystem: Option<(u16, u16)>) -> PciNames {
    let mut out = PciNames::default();
    let mut in_vendor = false;
    let mut in_device = false;
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("\t\t") {
            // Subsystem: "ssvv ssdd  Name" under the matched device.
            if let (true, Some((sv, sd))) = (in_device, subsystem) {
                let mut ids = rest.split_whitespace();
                if hex(ids.next()) == Some(sv) && hex(ids.next()) == Some(sd) {
                    out.subsystem = name_after_ids(rest, 2);
                }
            }
        } else if let Some(rest) = line.strip_prefix('\t') {
            if !in_vendor {
                continue;
            }
            in_device = hex(rest.split_whitespace().next()) == Some(device);
            if in_device {
                out.device = name_after_ids(rest, 1);
            }
        } else {
            if in_vendor {
                break; // the next vendor block: our vendor is done
            }
            // The class list ("C 03  Display controller") follows the vendors.
            if line.starts_with("C ") {
                break;
            }
            in_vendor = hex(line.split_whitespace().next()) == Some(vendor);
            in_device = false;
            if in_vendor {
                out.vendor = name_after_ids(line, 1);
            }
        }
    }
    out
}

/// The name of a vendor id alone (a board maker named by a subsystem id the database does not
/// list under the device).
pub fn vendor_name(text: &str, vendor: u16) -> Option<String> {
    for line in text.lines() {
        if line.starts_with('#') || line.starts_with('\t') || line.trim().is_empty() {
            continue;
        }
        if line.starts_with("C ") {
            break;
        }
        if hex(line.split_whitespace().next()) == Some(vendor) {
            return name_after_ids(line, 1);
        }
    }
    None
}

fn hex(s: Option<&str>) -> Option<u16> {
    let s = s?;
    if s.len() != 4 {
        return None;
    }
    u16::from_str_radix(s, 16).ok()
}

/// The text after the first `n` whitespace-separated ids.
fn name_after_ids(line: &str, n: usize) -> Option<String> {
    let mut rest = line.trim_start();
    for _ in 0..n {
        let end = rest.find(char::is_whitespace)?;
        rest = rest.get(end..)?.trim_start();
    }
    let name = rest.trim();
    (!name.is_empty()).then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS: &str = "# comment
1002  Advanced Micro Devices, Inc. [AMD/ATI]
\t15bf  Phoenix1
\t\t17aa 3d12  Radeon 780M
10de  NVIDIA Corporation
\t2484  GA104 [GeForce RTX 3070]
\t\t1462 3909  RTX 3070 GAMING X TRIO
\t\t1043 87b8  TUF RTX 3070
\t2486  GA104 [GeForce RTX 3060 Ti]
8086  Intel Corporation
\t46a6  Alder Lake-P GT2 [Iris Xe Graphics]

# List of known device classes
C 03  Display controller
\t00  VGA compatible controller
";

    #[test]
    fn finds_vendor_device_and_subsystem() {
        let n = lookup(IDS, 0x10de, 0x2484, Some((0x1043, 0x87b8)));
        assert_eq!(n.vendor.as_deref(), Some("NVIDIA Corporation"));
        assert_eq!(n.device.as_deref(), Some("GA104 [GeForce RTX 3070]"));
        assert_eq!(n.subsystem.as_deref(), Some("TUF RTX 3070"));
    }

    #[test]
    fn unknown_device_keeps_vendor() {
        let n = lookup(IDS, 0x8086, 0xffff, None);
        assert_eq!(n.vendor.as_deref(), Some("Intel Corporation"));
        assert_eq!(n.device, None);
    }

    #[test]
    fn device_ids_are_not_matched_under_another_vendor() {
        assert_eq!(lookup(IDS, 0x1002, 0x2484, None).device, None);
    }

    #[test]
    fn vendor_name_alone() {
        assert_eq!(
            vendor_name(IDS, 0x10de).as_deref(),
            Some("NVIDIA Corporation")
        );
        assert_eq!(
            vendor_name(IDS, 0x8086).as_deref(),
            Some("Intel Corporation")
        );
        assert_eq!(
            vendor_name(IDS, 0x15bf),
            None,
            "a device id is not a vendor"
        );
        assert_eq!(vendor_name(IDS, 0x0003), None, "classes are not vendors");
    }

    #[test]
    fn class_section_is_not_a_vendor() {
        assert_eq!(lookup(IDS, 0x0003, 0x0000, None), PciNames::default());
    }
}
