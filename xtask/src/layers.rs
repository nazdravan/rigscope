//! Layer rule check (forge-rust-app Rule 3).

/// Rank of a workspace crate. Lower may not depend on higher or equal (except same-rank parsers
/// may be used by `probe`, which has a slightly higher rank).
pub fn rank(name: &str) -> Option<u32> {
    match name {
        "rigscope-model" => Some(10),
        "rigscope-smbios" | "rigscope-procfs" => Some(30),
        "rigscope-probe" => Some(31),
        "rigscope-engine" => Some(50),
        "rigscope-win" => Some(20),
        "rigscope-probe-win" => Some(31),
        "rigscope-ui" => Some(60),
        "rigscope" => Some(70),
        _ => None,
    }
}

/// Crates that must never appear in dependencies of crates below the UI layer.
pub const UI_ONLY: &[&str] = &["egui", "eframe", "winit", "rfd", "arboard", "egui_kittest"];

/// Returns violations for one crate given its direct dependency names.
pub fn check(krate: &str, deps: &[&str]) -> Vec<String> {
    let Some(own) = rank(krate) else {
        return vec![format!(
            "{krate}: not in the layer table (xtask/src/layers.rs)"
        )];
    };
    let mut out = Vec::new();
    for dep in deps {
        if let Some(r) = rank(dep) {
            if r >= own {
                out.push(format!("{krate} (rank {own}) depends on {dep} (rank {r})"));
            }
        } else if own < 60 && UI_ONLY.contains(dep) {
            out.push(format!(
                "{krate} is below the UI layer but depends on {dep}"
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_may_not_depend_on_engine() {
        assert_eq!(check("rigscope-model", &["rigscope-engine"]).len(), 1);
    }

    #[test]
    fn probe_may_depend_on_parsers() {
        assert!(check(
            "rigscope-probe",
            &["rigscope-smbios", "rigscope-procfs", "rigscope-model"]
        )
        .is_empty());
    }

    #[test]
    fn parser_may_not_depend_on_probe() {
        assert_eq!(check("rigscope-smbios", &["rigscope-probe"]).len(), 1);
    }

    #[test]
    fn engine_may_not_use_egui() {
        assert_eq!(check("rigscope-engine", &["egui"]).len(), 1);
    }

    #[test]
    fn ui_may_use_egui_and_engine() {
        assert!(check("rigscope-ui", &["egui", "rigscope-engine"]).is_empty());
    }

    #[test]
    fn unknown_workspace_crate_is_violation() {
        assert_eq!(check("rigscope-mystery", &[]).len(), 1);
    }
}
