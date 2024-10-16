#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use rigscope_procfs::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn parsers_never_panic(s in ".{0,512}") {
        let _ = parse_cpuinfo(&s);
        let _ = parse_khz(&s);
        let _ = parse_cache_size(&s);
        let _ = parse_meminfo_total(&s);
        let _ = parse_os_release(&s);
        let _ = parse_uptime(&s);
        let _ = clean_dmi_value(&s);
        prop_assert!(parse_cpu_list(&s).len() <= MAX_CPUS);
    }

    #[test]
    fn cpu_list_ranges_are_capped(a in any::<u32>(), b in any::<u32>()) {
        let list = format!("{a}-{b}");
        prop_assert!(parse_cpu_list(&list).len() <= MAX_CPUS);
    }

    #[test]
    fn cpuinfo_structured_never_panics(lines in proptest::collection::vec(
        ("(processor|model name|cpu MHz|physical id|core id|flags|vendor_id|microcode)", ".{0,40}"), 0..64)) {
        let text: String = lines.iter().map(|(k, v)| format!("{k}\t: {v}\n")).collect();
        let _ = parse_cpuinfo(&text);
    }
}
