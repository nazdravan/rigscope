//! "What RAM can I buy?" summary derived from the SMBIOS memory arrays.

use crate::{FormFactor, MemType, MemoryInfo, SlotState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatLine {
    /// Common memory type of the installed modules; `None` if mixed or unknown.
    pub mem_type: Option<MemType>,
    pub mixed_types: bool,
    pub form_factor: Option<FormFactor>,
    /// Highest rated speed among installed modules.
    pub max_mts: Option<u32>,
    /// Board max capacity / slots. SMBIOS has no per-slot limit, so this is an estimate.
    pub max_per_slot_bytes_estimated: Option<u64>,
    pub free_slots: Option<u16>,
    pub total_slots: Option<u16>,
}

/// `None` when there is no memory array (no DMI data).
pub fn compat_summary(m: &MemoryInfo) -> Option<CompatLine> {
    if m.arrays.is_empty() {
        return None;
    }
    let mut types: Vec<MemType> = Vec::new();
    let mut form_factors: Vec<FormFactor> = Vec::new();
    let mut max_mts: Option<u32> = None;
    let mut free: u32 = 0;
    let mut total: u32 = 0;
    let mut max_capacity: Option<u64> = Some(0);

    for array in &m.arrays {
        let slots = array
            .slot_count
            .known()
            .map_or(array.slots.len() as u32, |n| u32::from(*n));
        total = total.saturating_add(slots);
        max_capacity = match (max_capacity, array.max_capacity_bytes.known()) {
            (Some(acc), Some(cap)) => Some(acc.saturating_add(*cap)),
            _ => None,
        };
        for slot in &array.slots {
            match &slot.module {
                SlotState::Empty => free = free.saturating_add(1),
                SlotState::Unknown => {}
                SlotState::Installed(module) => {
                    if let Some(t) = module.mem_type.known() {
                        if !types.contains(t) {
                            types.push(*t);
                        }
                    }
                    if let Some(f) = module.form_factor.known() {
                        if !form_factors.contains(f) {
                            form_factors.push(*f);
                        }
                    }
                    if let Some(mts) = module.rated_mts.known() {
                        max_mts = Some(max_mts.map_or(*mts, |cur| cur.max(*mts)));
                    }
                }
            }
        }
    }

    let mixed_types = types.len() > 1;
    Some(CompatLine {
        mem_type: single(&types),
        mixed_types,
        form_factor: single(&form_factors),
        max_mts,
        max_per_slot_bytes_estimated: match (max_capacity, u64::from(total)) {
            (Some(cap), slots) if slots > 0 && cap > 0 => Some(cap / slots),
            _ => None,
        },
        free_slots: u16::try_from(free).ok(),
        total_slots: u16::try_from(total).ok(),
    })
}

impl CompatLine {
    /// Short text to paste into a shop search, e.g. `DDR4 SO-DIMM 3200 MT/s 32GB`.
    pub fn to_search_text(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(t) = self.mem_type {
            parts.push(t.label());
        }
        if let Some(f) = self.form_factor {
            parts.push(f.label());
        }
        if let Some(mts) = self.max_mts {
            parts.push(format!("{mts} MT/s"));
        }
        if let Some(bytes) = self.max_per_slot_bytes_estimated {
            parts.push(size_label(bytes));
        }
        parts.join(" ")
    }
}

fn single<T: Copy>(v: &[T]) -> Option<T> {
    if v.len() == 1 {
        v.first().copied()
    } else {
        None
    }
}

/// `32 GiB` → `32GB` (shops use GB for binary sizes); falls back to MB.
fn size_label(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    const GIB: u64 = 1024 * MIB;
    if bytes >= GIB && bytes % GIB == 0 {
        format!("{}GB", bytes / GIB)
    } else {
        format!("{}MB", bytes / MIB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field, MemoryArray, MemoryModule, MemorySlot};

    const GIB: u64 = 1024 * 1024 * 1024;

    fn module(t: MemType, ff: FormFactor, mts: u32) -> SlotState {
        SlotState::Installed(Box::new(MemoryModule {
            size_bytes: Field::Known(8 * GIB),
            mem_type: Field::Known(t),
            form_factor: Field::Known(ff),
            rated_mts: Field::Known(mts),
            ..Default::default()
        }))
    }

    fn array(max: u64, slots: Vec<SlotState>) -> MemoryArray {
        MemoryArray {
            max_capacity_bytes: Field::Known(max),
            slot_count: Field::Known(slots.len() as u16),
            ecc: Field::Unknown,
            slots: slots
                .into_iter()
                .map(|module| MemorySlot {
                    module,
                    ..Default::default()
                })
                .collect(),
        }
    }

    #[test]
    fn compat_single_ddr4() {
        let m = MemoryInfo {
            os_total_bytes: Field::Unknown,
            arrays: vec![array(
                64 * GIB,
                vec![
                    module(MemType::Ddr4, FormFactor::SoDimm, 3200),
                    SlotState::Empty,
                ],
            )],
        };
        let c = compat_summary(&m).expect("compat");
        assert_eq!(c.mem_type, Some(MemType::Ddr4));
        assert!(!c.mixed_types);
        assert_eq!(c.form_factor, Some(FormFactor::SoDimm));
        assert_eq!(c.max_mts, Some(3200));
        assert_eq!(c.max_per_slot_bytes_estimated, Some(32 * GIB));
        assert_eq!(c.free_slots, Some(1));
        assert_eq!(c.total_slots, Some(2));
        assert_eq!(c.to_search_text(), "DDR4 SO-DIMM 3200 MT/s 32GB");
    }

    #[test]
    fn compat_mixed_modules() {
        let same = MemoryInfo {
            os_total_bytes: Field::Unknown,
            arrays: vec![array(
                32 * GIB,
                vec![
                    module(MemType::Ddr4, FormFactor::Dimm, 2666),
                    module(MemType::Ddr4, FormFactor::Dimm, 3200),
                ],
            )],
        };
        let c = compat_summary(&same).expect("compat");
        assert_eq!(c.max_mts, Some(3200));
        assert!(!c.mixed_types);
        assert_eq!(c.mem_type, Some(MemType::Ddr4));

        let mixed = MemoryInfo {
            os_total_bytes: Field::Unknown,
            arrays: vec![array(
                32 * GIB,
                vec![
                    module(MemType::Ddr4, FormFactor::Dimm, 3200),
                    module(MemType::Ddr5, FormFactor::Dimm, 4800),
                ],
            )],
        };
        let c = compat_summary(&mixed).expect("compat");
        assert!(c.mixed_types);
        assert_eq!(c.mem_type, None);
    }

    #[test]
    fn compat_no_dmi() {
        assert_eq!(compat_summary(&MemoryInfo::default()), None);
    }

    #[test]
    fn compat_zero_slots() {
        let m = MemoryInfo {
            os_total_bytes: Field::Unknown,
            arrays: vec![MemoryArray {
                max_capacity_bytes: Field::Known(16 * GIB),
                slot_count: Field::Known(0),
                ..Default::default()
            }],
        };
        let c = compat_summary(&m).expect("compat");
        assert_eq!(c.max_per_slot_bytes_estimated, None);
    }

    #[test]
    fn compat_multiple_arrays_sum() {
        let m = MemoryInfo {
            os_total_bytes: Field::Unknown,
            arrays: vec![
                array(
                    32 * GIB,
                    vec![
                        module(MemType::Ddr4, FormFactor::Dimm, 3200),
                        SlotState::Empty,
                    ],
                ),
                array(32 * GIB, vec![SlotState::Empty, SlotState::Empty]),
            ],
        };
        let c = compat_summary(&m).expect("compat");
        assert_eq!(c.total_slots, Some(4));
        assert_eq!(c.free_slots, Some(3));
        assert_eq!(c.max_per_slot_bytes_estimated, Some(16 * GIB));
    }

    #[test]
    fn search_text_skips_unknown_parts() {
        let c = CompatLine {
            mem_type: Some(MemType::Ddr5),
            ..Default::default()
        };
        assert_eq!(c.to_search_text(), "DDR5");
    }
}
