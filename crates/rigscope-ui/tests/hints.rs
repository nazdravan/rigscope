use rigscope_engine::Language;
use rigscope_ui::i18n::missing_hint;

#[test]
fn missing_values_get_an_explanation() {
    assert!(missing_hint(Language::En, "Unknown").is_some_and(|h| h.contains("did not report")));
    assert!(missing_hint(Language::En, "Needs root")
        .is_some_and(|h| h.contains("Read with elevated rights")));
    assert!(missing_hint(Language::Ro, "Necesită root")
        .is_some_and(|h| h.contains("Citește cu drepturi ridicate")));
    assert_eq!(missing_hint(Language::En, "8 GiB"), None);
}

#[test]
fn opened_snapshot_status_in_romanian() {
    assert_eq!(
        rigscope_ui::i18n::status(Language::Ro, "command not available: showing /s.json; refresh to return to this computer"),
        "comandă indisponibilă: se afișează /s.json; reîmprospătează ca să revii la acest calculator"
    );
    assert_eq!(
        rigscope_ui::i18n::status(
            Language::Ro,
            "command not available: this machine has no DMI table"
        ),
        "comandă indisponibilă: acest calculator nu are tabelă DMI"
    );
}
