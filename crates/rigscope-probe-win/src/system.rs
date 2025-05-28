use crate::{field_err, field_of, WinSource};
use rigscope_model::{clean_text, BoardInfo, DmiSource, Field, MemoryInfo, SystemInfo};
use rigscope_smbios::Decoded;

const NO_TABLE: &str = "the firmware has no SMBIOS table (virtual machine?)";
/// First Windows 11 build. `ProductName` keeps saying "Windows 10" on Windows 11.
const WIN11_BUILD: u32 = 22000;

pub(crate) fn read_system(src: &dyn WinSource) -> SystemInfo {
    let os = src.os_version();
    SystemInfo {
        os_name: field_of(&os, |v| {
            let Some(build) = v.current_build else {
                return Field::Unknown;
            };
            let major = if build >= WIN11_BUILD { "11" } else { "10" };
            // "Windows 10 Pro" -> the words after the version number: "Pro".
            let edition = v
                .product_name
                .as_deref()
                .map(|p| p.split_whitespace().skip(2).collect::<Vec<_>>().join(" "))
                .filter(|e| !e.is_empty());
            let mut name = format!("Windows {major}");
            if let Some(e) = edition {
                name = format!("{name} {e}");
            }
            if let Some(d) = v.display_version.as_deref().and_then(clean_text) {
                name = format!("{name} {d}");
            }
            Field::Known(name)
        }),
        kernel: field_of(&os, |v| match (v.current_build, v.ubr) {
            (Some(b), Some(u)) => Field::Known(format!("10.0.{b}.{u}")),
            (Some(b), None) => Field::Known(format!("10.0.{b}")),
            _ => Field::Unknown,
        }),
        hostname: field_of(&src.computer_name(), |n| Field::from_option(clean_text(n))),
        uptime_s: field_of(&src.uptime_ms(), |ms| Field::Known(ms / 1000)),
    }
}

pub(crate) struct Firmware {
    pub decoded: Option<Decoded>,
    pub source: DmiSource,
    /// Why the board/BIOS fields are missing when `decoded` is `None`.
    pub missing: Field<String>,
}

pub(crate) fn read_firmware(src: &dyn WinSource) -> Firmware {
    match src.smbios_raw() {
        Ok(buf) => match rigscope_smbios::decode_rsmb(&buf) {
            Ok(d) => Firmware {
                decoded: Some(d),
                source: DmiSource::Firmware,
                missing: Field::Unknown,
            },
            Err(e) => Firmware {
                decoded: None,
                source: DmiSource::NoTable,
                missing: Field::Unavailable(format!("SMBIOS table unreadable: {e}")),
            },
        },
        Err(rigscope_win::WinError::NotAvailable) => Firmware {
            decoded: None,
            source: DmiSource::None,
            missing: Field::Unavailable(NO_TABLE.into()),
        },
        Err(e) => Firmware {
            decoded: None,
            source: DmiSource::None,
            missing: field_err(&e),
        },
    }
}

pub(crate) fn read_board(fw: &Firmware) -> BoardInfo {
    match &fw.decoded {
        Some(d) => d.board.clone(),
        None => {
            let m = || fw.missing.clone();
            BoardInfo {
                vendor: m(),
                product: m(),
                version: m(),
                bios_vendor: m(),
                bios_version: m(),
                bios_date: m(),
            }
        }
    }
}

pub(crate) fn read_memory(src: &dyn WinSource, fw: &Firmware) -> MemoryInfo {
    MemoryInfo {
        os_total_bytes: field_of(&src.memory_status(), |m| Field::Known(m.total_phys)),
        arrays: fw
            .decoded
            .as_ref()
            .map(|d| d.memory.arrays.clone())
            .unwrap_or_default(),
    }
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
    use rigscope_smbios::testutil::{memory_array, memory_device, table, Dimm};
    use rigscope_win::{MemoryStatusRaw, OsVersionRaw, WinError};

    fn rsmb(t: &[u8]) -> Vec<u8> {
        let mut b = vec![0, 3, 4, 0];
        b.extend(u32::try_from(t.len()).unwrap().to_le_bytes());
        b.extend_from_slice(t);
        b
    }

    fn os(build: u32, product: &str) -> OsVersionRaw {
        OsVersionRaw {
            product_name: Some(product.into()),
            display_version: Some("23H2".into()),
            current_build: Some(build),
            ubr: Some(4317),
            edition_id: Some("Professional".into()),
        }
    }

    fn dimm() -> Dimm<'static> {
        Dimm {
            size_mib: 8192,
            form_factor: 0x09,
            mem_type: 0x1A,
            speed: 3200,
            configured: 3200,
            locator: "DIMM A1",
            bank: "BANK 0",
            manufacturer: "Acme",
            serial: "1234",
            part: "PART",
            ranks: 2,
            voltage_mv: 1200,
        }
    }

    #[test]
    fn windows_11_from_build_not_product_name() {
        let mut w = MemWin::default();
        w.os_version = Ok(os(22631, "Windows 10 Pro"));
        let s = read_system(&w);
        assert_eq!(s.os_name, Field::Known("Windows 11 Pro 23H2".into()));
        assert_eq!(s.kernel, Field::Known("10.0.22631.4317".into()));
        w.os_version = Ok(os(19045, "Windows 10 Pro"));
        assert_eq!(
            read_system(&w).os_name,
            Field::Known("Windows 10 Pro 23H2".into())
        );
    }

    #[test]
    fn system_fields_and_errors() {
        let mut w = MemWin::default();
        w.computer_name = Ok("NICU-PC".into());
        w.uptime_ms = Ok(90_061_000);
        w.os_version = Err(WinError::AccessDenied);
        let s = read_system(&w);
        assert_eq!(s.hostname, Field::Known("NICU-PC".into()));
        assert_eq!(s.uptime_s, Field::Known(90_061));
        assert_eq!(s.os_name, Field::NeedsElevation);
    }

    #[test]
    fn memory_from_rsmb_and_status() {
        let mut w = MemWin::default();
        w.smbios = Ok(rsmb(&table(&[
            memory_array(0x10, 64 << 30, 2),
            memory_device(0x11, 0x10, &dimm()),
        ])));
        w.memory_status = Ok(MemoryStatusRaw {
            total_phys: 16 << 30,
            avail_phys: 8 << 30,
        });
        let fw = read_firmware(&w);
        assert_eq!(fw.source, DmiSource::Firmware);
        let m = read_memory(&w, &fw);
        assert_eq!(m.os_total_bytes, Field::Known(16 << 30));
        assert_eq!(m.arrays.len(), 1);
    }

    #[test]
    fn no_firmware_table() {
        let mut w = MemWin::default();
        w.smbios = Err(WinError::NotAvailable);
        let fw = read_firmware(&w);
        assert_eq!(fw.source, DmiSource::None);
        assert!(read_memory(&w, &fw).arrays.is_empty());
        assert!(matches!(read_board(&fw).vendor, Field::Unavailable(_)));
        w.smbios = Ok(vec![1, 2, 3]); // broken header
        assert_eq!(read_firmware(&w).source, DmiSource::NoTable);
    }
}
