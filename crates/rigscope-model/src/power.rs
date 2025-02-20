use crate::Field;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PowerInfo {
    pub batteries: Vec<Battery>,
    /// Mains adapter connected; `Unknown` when the machine reports none (desktops).
    pub ac_online: Field<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Battery {
    /// `BAT0`.
    pub name: String,
    pub manufacturer: Field<String>,
    pub model: Field<String>,
    pub serial: Field<String>,
    /// `Li-ion`, `Li-poly`.
    pub technology: Field<String>,
    /// `Charging`, `Discharging`, `Full`, `Not charging`.
    pub status: Field<String>,
    pub charge_percent: Field<u8>,
    pub design_mwh: Field<u64>,
    pub full_mwh: Field<u64>,
    pub now_mwh: Field<u64>,
    pub cycles: Field<u32>,
}

impl Battery {
    /// Full charge capacity as a share of the design capacity (derived), e.g. `92.0`.
    pub fn health_percent(&self) -> Option<f64> {
        let (design, full) = (*self.design_mwh.known()?, *self.full_mwh.known()?);
        if design == 0 {
            return None;
        }
        Some((full as f64 / design as f64 * 1000.0).round() / 10.0)
    }
}

/// `57000` mWh → `57.0 Wh`.
pub fn format_mwh(mwh: u64) -> String {
    format!("{:.1} Wh", mwh as f64 / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_is_full_over_design() {
        let b = Battery {
            design_mwh: Field::Known(57_000),
            full_mwh: Field::Known(52_440),
            ..Default::default()
        };
        assert_eq!(b.health_percent(), Some(92.0));
        assert_eq!(
            Battery {
                design_mwh: Field::Known(0),
                full_mwh: Field::Known(1),
                ..Default::default()
            }
            .health_percent(),
            None
        );
        assert_eq!(Battery::default().health_percent(), None);
    }

    #[test]
    fn energy_format() {
        assert_eq!(format_mwh(57_000), "57.0 Wh");
        assert_eq!(format_mwh(45_600), "45.6 Wh");
    }
}
