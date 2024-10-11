//! Cleaning of firmware strings shared by the SMBIOS and sysfs parsers.

/// Values vendors put in DMI fields that mean "nothing here".
const PLACEHOLDERS: &[&str] = &[
    "to be filled by o.e.m.",
    "to be filled by oem",
    "not specified",
    "not available",
    "unknown",
    "default string",
    "system product name",
    "system manufacturer",
    "none",
    "n/a",
    "no dimm",
    "empty",
    "undefined",
    "0x0000",
    "00000000",
    "0000000000000000",
];

/// Trims whitespace/NULs; returns `None` for empty or placeholder values.
pub fn clean_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if PLACEHOLDERS.contains(&lower.as_str()) {
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::clean_text;

    #[test]
    fn keeps_real_value_trimmed() {
        assert_eq!(clean_text("  Kingston \0").as_deref(), Some("Kingston"));
    }

    #[test]
    fn placeholders_are_none_case_insensitive() {
        assert_eq!(clean_text("To Be Filled By O.E.M."), None);
        assert_eq!(clean_text("Default string"), None);
        assert_eq!(clean_text("NOT SPECIFIED"), None);
        assert_eq!(clean_text("   "), None);
    }
}
