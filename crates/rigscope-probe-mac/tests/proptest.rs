use proptest::prelude::*;
use rigscope_probe_mac::{
    decode_hex_text, manufacturer, mask_text, parse_boottime, parse_hardware, parse_memory,
    parse_mts, parse_size, parse_type,
};

proptest! {
    #[test]
    fn text_parsers_never_panic(s in ".{0,200}") {
        let _ = parse_size(&s);
        let _ = parse_mts(&s);
        let _ = parse_type(&s);
        let _ = decode_hex_text(&s);
        let _ = manufacturer(&s);
        let _ = mask_text(&s);
        let _ = parse_boottime(&s);
    }

    #[test]
    fn json_parsers_never_panic(s in ".{0,400}") {
        let _ = parse_memory(&s);
        let _ = parse_hardware(&s);
    }

    #[test]
    fn hex_prefixed_text_never_panics(s in "0x[0-9A-Fa-f]{0,300}") {
        let _ = decode_hex_text(&s);
        let _ = manufacturer(&s);
    }

    #[test]
    fn masking_keeps_the_length(s in ".{0,100}") {
        prop_assert_eq!(mask_text(&s).chars().count(), s.chars().count());
    }

    #[test]
    fn slot_count_is_capped(n in 0usize..200) {
        let items: Vec<String> = (0..n)
            .map(|i| format!(r#"{{"_name":"S{i}","dimm_size":"8 GB"}}"#))
            .collect();
        let json = format!(r#"{{"SPMemoryDataType":[{{"_items":[{}]}}]}}"#, items.join(","));
        let p = parse_memory(&json).unwrap();
        prop_assert!(p.dimms.len() <= rigscope_probe_mac::MAX_SLOTS);
    }
}
