//! `cargo xtask licences`: every file in `assets/fonts/` (each font and its licence) is named in
//! `NOTICE`, so a new font never ships without its licence.

use anyhow::{Context, Result, bail};

pub(crate) fn check() -> Result<()> {
    let root = crate::root();
    let notice = std::fs::read_to_string(root.join("NOTICE")).context("NOTICE")?;
    let dir = root.join("assets").join("fonts");
    let names: Vec<String> = std::fs::read_dir(&dir)
        .with_context(|| dir.display().to_string())?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    let missing = not_in_notice(&notice, &names);
    if !missing.is_empty() {
        bail!(
            "assets/fonts files not named in NOTICE (add the font and its licence): {}",
            missing.join(", ")
        );
    }
    eprintln!(
        "licences: {} files in assets/fonts, each named in NOTICE",
        names.len()
    );
    Ok(())
}

/// The names (dot files skipped) that NOTICE does not mention as `assets/fonts/<name>`, sorted.
fn not_in_notice(notice: &str, names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = names
        .iter()
        .filter(|n| !n.starts_with('.') && !notice.contains(&format!("assets/fonts/{n}")))
        .cloned()
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::not_in_notice;

    #[test]
    fn a_font_missing_from_notice_is_reported() {
        let names = ["NotoSans-Regular.ttf", "OFL.txt", "New.ttf", ".DS_Store"].map(String::from);
        let notice = "Noto Sans (assets/fonts/NotoSans-Regular.ttf), see assets/fonts/OFL.txt";
        assert_eq!(not_in_notice(notice, &names), ["New.ttf"]);
    }
}
