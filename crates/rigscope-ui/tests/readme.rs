//! The README shortcut tables come from the command registry. Run with
//! `RIGSCOPE_UPDATE_README=1 cargo test -p rigscope-ui --test readme` to rewrite them.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_engine::Language;

const START: &str = "<!-- shortcuts:start -->\n";
const END: &str = "<!-- shortcuts:end -->";

fn check(file: &str, lang: Language) {
    let path = format!("{}/../../{file}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let (Some(a), Some(b)) = (text.find(START), text.find(END)) else {
        panic!("{file}: shortcut markers missing");
    };
    let want = rigscope_ui::shortcut_table(lang);
    let have = text.get(a + START.len()..b).unwrap_or_default();
    if have == want {
        return;
    }
    if std::env::var("RIGSCOPE_UPDATE_README").is_ok() {
        let new = format!(
            "{}{START}{want}{}",
            text.get(..a).unwrap_or_default(),
            text.get(b..).unwrap_or_default()
        );
        std::fs::write(&path, new).unwrap_or_else(|e| panic!("{file}: {e}"));
        return;
    }
    panic!(
        "{file}: shortcut table is stale; run RIGSCOPE_UPDATE_README=1 cargo test -p rigscope-ui --test readme"
    );
}

#[test]
fn english_readme_shortcuts_are_current() {
    check("README.md", Language::En);
}

#[test]
fn romanian_readme_shortcuts_are_current() {
    check("README.ro.md", Language::Ro);
}
