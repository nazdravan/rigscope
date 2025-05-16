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

/// What a value's wording depends on: the UI language and the OS the snapshot came from.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub lang: Language,
    pub platform: rigscope_model::Platform,
}

impl Ctx {
    pub fn of(app: &rigscope_engine::App) -> Ctx {
        Ctx {
            lang: app.session.language,
            platform: app.session.snapshot.platform,
        }
    }
}

/// Displays a field, spelling out why a value is missing.
pub fn field<T>(cx: Ctx, f: &Field<T>, show: impl Fn(&T) -> String) -> String {
    match f {
        Field::Known(v) => show(v),
        Field::Unknown => tr(cx.lang, Key::Unknown).to_string(),
        Field::NeedsElevation => match cx.platform {
            rigscope_model::Platform::Windows => tr(cx.lang, Key::NeedsAdmin).to_string(),
            _ => tr(cx.lang, Key::NeedsRoot).to_string(),
        },
        Field::Unavailable(reason) => crate::i18n::reason(cx.lang, reason),
    }
}

pub fn text(cx: Ctx, f: &Field<String>) -> String {
    field(cx, f, Clone::clone)
}
