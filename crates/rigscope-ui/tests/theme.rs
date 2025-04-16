use egui::Color32;
use rigscope_ui::theme::*;

#[test]
fn the_shipped_theme_parses() {
    let t = theme().expect("assets/theme.toml");
    assert_eq!(t.accent, Color32::from_rgb(0x1F, 0x8F, 0xC8));
    assert_eq!(t.accent_press, Color32::from_rgb(0x00, 0x5C, 0x80));
    assert_eq!(t.warn, Color32::from_rgb(0xE8, 0xA3, 0x3D));
}

#[test]
fn hex_parsing() {
    assert_eq!(
        parse_hex("#1B1B1B"),
        Some(Color32::from_rgb(0x1B, 0x1B, 0x1B))
    );
    assert_eq!(parse_hex("1B1B1B"), None, "needs the #");
    assert_eq!(parse_hex("#12345"), None);
    assert_eq!(parse_hex("#GG0000"), None);
    assert_eq!(parse_hex("#é00000"), None);
}

#[test]
fn comment_lines_are_ignored() {
    let t = "# accent = \"#000000\"\naccent = \"#010203\"\naccent_press = \"#000000\"\nbg_light = \"#000000\"\nfg_light = \"#000000\"\nbg_dark = \"#000000\"\nfg_dark = \"#000000\"\nwarn = \"#000000\"\n";
    assert_eq!(
        parse_theme(t).map(|t| t.accent),
        Some(Color32::from_rgb(1, 2, 3))
    );
}

#[test]
fn missing_key_is_none_not_panic() {
    assert_eq!(parse_theme("accent = \"#ffffff\"\n"), None);
    assert_eq!(parse_theme(""), None);
    for junk in [
        "accent = ",
        "= \"#000000\"",
        "accent = \"#000000",
        "\u{0}\u{1}",
    ] {
        let _ = parse_theme(junk);
    }
}

#[test]
fn visuals_use_the_theme_colours() {
    let t = theme().expect("theme");
    let dark = visuals(&t, true);
    assert!(dark.dark_mode);
    assert_eq!(dark.selection.bg_fill, t.accent_press);
    assert_eq!(dark.hyperlink_color, t.accent);
    assert_eq!(dark.panel_fill, t.bg_dark);
    assert_eq!(dark.warn_fg_color, t.warn);
    let light = visuals(&t, false);
    assert!(!light.dark_mode);
    assert_eq!(light.panel_fill, t.bg_light);
    assert_eq!(light.selection.bg_fill, t.accent);
}

#[test]
fn install_sets_noto_sans_first() {
    install(&egui::Context::default());
    let fonts = font_definitions();
    let first = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .and_then(|f| f.first().cloned());
    assert_eq!(first.as_deref(), Some("NotoSans"));
}
