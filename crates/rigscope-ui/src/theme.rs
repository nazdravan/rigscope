//! The app's look, from `assets/theme.toml` (the same file the icon, README banner and landing
//! page use).

/// `assets/theme.toml`, compiled in.
pub const THEME_TOML: &str = include_str!("../../../assets/theme.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub accent: egui::Color32,
    pub accent_press: egui::Color32,
    pub bg_light: egui::Color32,
    pub fg_light: egui::Color32,
    pub bg_dark: egui::Color32,
    pub fg_dark: egui::Color32,
    pub warn: egui::Color32,
}

/// `#1F8FC8` → colour.
pub fn parse_hex(s: &str) -> Option<egui::Color32> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some(egui::Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?))
}

/// Reads the `key = "#RRGGBB"` lines the theme needs; `None` if any is missing or malformed.
pub fn parse_theme(toml: &str) -> Option<Theme> {
    let get = |key: &str| {
        toml.lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                if k.trim() != key {
                    return None;
                }
                let v = v.trim();
                let v = v.strip_prefix('"')?;
                let end = v.find('"')?;
                parse_hex(v.get(..end)?)
            })
    };
    Some(Theme {
        accent: get("accent")?,
        accent_press: get("accent_press")?,
        bg_light: get("bg_light")?,
        fg_light: get("fg_light")?,
        bg_dark: get("bg_dark")?,
        fg_dark: get("fg_dark")?,
        warn: get("warn")?,
    })
}

/// The compiled-in theme (falls back to egui's defaults if the file were ever broken).
pub fn theme() -> Option<Theme> {
    parse_theme(THEME_TOML)
}

pub fn visuals(theme: &Theme, dark: bool) -> egui::Visuals {
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    let (bg, fg) = if dark {
        (theme.bg_dark, theme.fg_dark)
    } else {
        (theme.bg_light, theme.fg_light)
    };
    // Dark: the deeper accent keeps white text readable; light: the brighter one.
    v.selection.bg_fill = if dark {
        theme.accent_press
    } else {
        theme.accent
    };
    v.selection.stroke.color = egui::Color32::WHITE;
    v.hyperlink_color = theme.accent;
    v.warn_fg_color = theme.warn;
    v.panel_fill = bg;
    v.window_fill = bg;
    v.override_text_color = None;
    v.widgets.noninteractive.fg_stroke.color = fg;
    v
}

/// egui's fonts with Noto Sans in front for proportional text.
pub fn font_definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "NotoSans".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/NotoSans-Regular.ttf"
        ))),
    );
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.insert(0, "NotoSans".into());
    }
    fonts
}

/// Applies colours and the Noto Sans font to the context.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
    if let Some(t) = theme() {
        ctx.set_visuals_of(egui::Theme::Dark, visuals(&t, true));
        ctx.set_visuals_of(egui::Theme::Light, visuals(&t, false));
    }
}
