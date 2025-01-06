//! Value formatting for display.

use crate::i18n::{tr, Key};
use rigscope_engine::Language;
use rigscope_model::Field;

/// Binary size: `8 GiB`, `1.5 GiB`, `512 KiB`.
pub fn bytes(n: u64) -> String {
    rigscope_model::format_bytes(n)
}

pub fn mhz(v: u32) -> String {
    format!("{v} MHz")
}

pub fn mts(v: u32) -> String {
    format!("{v} MT/s")
}

pub fn uptime(secs: u64) -> String {
    let (d, h, m) = (secs / 86_400, secs % 86_400 / 3600, secs % 3600 / 60);
    if d > 0 {
        format!("{d} d {h} h {m} min")
    } else {
        format!("{h} h {m} min")
    }
}

/// Displays a field, spelling out why a value is missing.
pub fn field<T>(lang: Language, f: &Field<T>, show: impl Fn(&T) -> String) -> String {
    match f {
        Field::Known(v) => show(v),
        Field::Unknown => tr(lang, Key::Unknown).to_string(),
        Field::NeedsRoot => tr(lang, Key::NeedsRoot).to_string(),
        Field::Unavailable(reason) => reason.clone(),
    }
}

pub fn text(lang: Language, f: &Field<String>) -> String {
    field(lang, f, Clone::clone)
}
