use crate::{WinSource, field_err};
use rigscope_model::{Fan, Field, SensorChip, SensorInfo, Temperature};
use rigscope_win::WinError;

pub const NO_CPU_TEMP: &str = "not available on Windows: needs a kernel driver";
/// Below this an ACPI zone is not a real reading (0 K is what absent zones report).
const MIN_TENTHS_K: u32 = 2000;
/// The label of the one chip shown when the ACPI zones cannot be read.
const ACPI_ZONES: &str = "ACPI thermal zones";

fn cpu_chip() -> SensorChip {
    SensorChip {
        hwmon: "cpu".into(),
        name: "CPU".into(),
        temps: vec![Temperature {
            index: 1,
            label: "CPU".into(),
            millicelsius: Field::Unavailable(NO_CPU_TEMP.into()),
            ..Temperature::default()
        }],
        fans: vec![Fan {
            index: 1,
            label: "fans".into(),
            rpm: Field::Unavailable(NO_CPU_TEMP.into()),
        }],
    }
}

fn gpu_temp(src: &mut dyn WinSource, luid: u64) -> Field<i32> {
    match src.gpu_temperature(luid) {
        Ok(Some(dc)) => i32::try_from(dc).map_or(Field::Unknown, |dc| Field::Known(dc * 100)),
        Ok(None) => Field::Unavailable("the graphics driver reports no temperature".into()),
        Err(e) => field_err(&e),
    }
}

pub(crate) fn read_sensors(src: &mut dyn WinSource, gpus: &[(String, u64)]) -> SensorInfo {
    let mut chips = vec![cpu_chip()];
    for (card, luid) in gpus {
        chips.push(SensorChip {
            hwmon: card.clone(),
            name: format!("GPU ({card})"),
            temps: vec![Temperature {
                index: 1,
                label: "GPU".into(),
                millicelsius: gpu_temp(src, *luid),
                ..Temperature::default()
            }],
            fans: Vec::new(),
        });
    }
    let zones = match src.acpi_thermal_zones() {
        Ok(v) => v,
        // No ACPI zones on this machine (a VM): nothing to show.
        Err(WinError::NotAvailable) => Vec::new(),
        // WMI timed out, failed or was refused: one chip that says so (spec 5.4).
        Err(e) => {
            chips.push(SensorChip {
                hwmon: "acpi".into(),
                name: "acpitz".into(),
                temps: vec![Temperature {
                    index: 1,
                    label: ACPI_ZONES.into(),
                    millicelsius: field_err(&e),
                    ..Temperature::default()
                }],
                fans: Vec::new(),
            });
            Vec::new()
        }
    };
    for z in zones {
        if z.tenths_kelvin < MIN_TENTHS_K {
            continue;
        }
        let mc = (i64::from(z.tenths_kelvin) * 100) - 273_150;
        let label = z
            .instance
            .rsplit('\\')
            .next()
            .unwrap_or(&z.instance)
            .to_string();
        chips.push(SensorChip {
            hwmon: format!("acpi-{label}"),
            // "acpitz": chip_kind() already labels it "ACPI thermal zone", never "CPU".
            name: "acpitz".into(),
            temps: vec![Temperature {
                index: 1,
                label,
                millicelsius: Field::from_option(i32::try_from(mc).ok()),
                ..Temperature::default()
            }],
            fans: Vec::new(),
        });
    }
    SensorInfo { chips }
}

/// Cheap tick: only the GPU temperatures (D3DKMT); ACPI zones need WMI and are read in collect.
pub(crate) fn refresh_sensors(src: &mut dyn WinSource, gpus: &[(String, u64)], s: &mut SensorInfo) {
    for (card, luid) in gpus {
        if let Some(chip) = s.chips.iter_mut().find(|c| &c.hwmon == card) {
            if let Some(t) = chip.temps.first_mut() {
                t.millicelsius = gpu_temp(src, *luid);
            }
        }
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
    use rigscope_win::ThermalZoneRaw;

    #[test]
    fn cpu_temp_is_unavailable_with_reason_gpu_and_acpi_shown() {
        let mut w = MemWin::default();
        w.gpu_temps.insert(7, Ok(Some(452)));
        w.thermal = Ok(vec![
            ThermalZoneRaw {
                instance: "ACPI\\ThermalZone\\TZ00_0".into(),
                tenths_kelvin: 3082,
            },
            ThermalZoneRaw {
                instance: "ACPI\\ThermalZone\\TZ01_0".into(),
                tenths_kelvin: 0,
            },
        ]);
        let gpus = vec![("gpu0".to_string(), 7u64)];
        let s = read_sensors(&mut w, &gpus);
        let cpu = s.chips.iter().find(|c| c.hwmon == "cpu").unwrap();
        assert_eq!(
            cpu.temps[0].millicelsius,
            Field::Unavailable(NO_CPU_TEMP.into())
        );
        assert_eq!(cpu.fans[0].rpm, Field::Unavailable(NO_CPU_TEMP.into()));
        let gpu = s.chips.iter().find(|c| c.hwmon == "gpu0").unwrap();
        assert_eq!(gpu.temps[0].millicelsius, Field::Known(45_200));
        let acpi: Vec<_> = s.chips.iter().filter(|c| c.name == "acpitz").collect();
        assert_eq!(acpi.len(), 1); // the 0 K zone is skipped
        assert_eq!(acpi[0].temps[0].millicelsius, Field::Known(35_050));
        assert_eq!(acpi[0].temps[0].label, "TZ00_0");
    }

    #[test]
    fn acpi_zone_errors_show_one_chip_or_stay_silent() {
        let read = |e: WinError| {
            let mut w = MemWin::default();
            w.thermal = Err(e);
            read_sensors(&mut w, &[])
                .chips
                .into_iter()
                .filter(|c| c.name == "acpitz")
                .collect::<Vec<_>>()
        };
        let t = read(WinError::Timeout);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].temps[0].label, ACPI_ZONES);
        assert_eq!(
            t[0].temps[0].millicelsius,
            Field::Unavailable("WMI did not answer".into())
        );
        let f = read(WinError::Failed {
            api: "IWbemServices::ExecQuery".into(),
            code: 0x8004_1003,
        });
        assert!(matches!(f[0].temps[0].millicelsius, Field::Unavailable(_)));
        let d = read(WinError::AccessDenied);
        assert_eq!(d[0].temps[0].millicelsius, Field::NeedsElevation);
        assert!(read(WinError::NotAvailable).is_empty());
    }

    #[test]
    fn refresh_updates_gpu_only() {
        let mut w = MemWin::default();
        w.gpu_temps.insert(7, Ok(Some(452)));
        let gpus = vec![("gpu0".to_string(), 7u64)];
        let mut s = read_sensors(&mut w, &gpus);
        w.gpu_temps.insert(7, Ok(Some(600)));
        refresh_sensors(&mut w, &gpus, &mut s);
        let gpu = s.chips.iter().find(|c| c.hwmon == "gpu0").unwrap();
        assert_eq!(gpu.temps[0].millicelsius, Field::Known(60_000));
    }
}
