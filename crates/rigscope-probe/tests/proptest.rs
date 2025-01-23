#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use rigscope_probe::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn decode_dmi_dump_never_panics(b in proptest::collection::vec(any::<u8>(), 0..512)) {
        let _ = decode_dmi_dump(&b);
    }

    #[test]
    fn build_snapshot_never_panics_on_garbage_files(
        cpuinfo in ".{0,300}", present in ".{0,20}", freq in ".{0,12}",
        table in proptest::collection::vec(any::<u8>(), 0..300)) {
        let src = MemSource::default()
            .with(paths::CPUINFO, cpuinfo)
            .with(paths::CPU_PRESENT, present)
            .with("/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq", freq)
            .with(paths::DMI_ENTRY, rigscope_smbios::testutil::entry_point_3(300))
            .with(paths::DMI_TABLE, table);
        let _ = build_snapshot(&src, None, 0);
    }

    #[test]
    fn read_storage_never_panics(name in "[a-z]{1,8}[0-9]{0,3}", size in ".{0,24}", rot in ".{0,4}",
                                 model in proptest::collection::vec(any::<u8>(), 0..64), link in ".{0,80}") {
        let mut src = MemSource::default()
            .with(&format!("/sys/block/{name}/size"), size)
            .with(&format!("/sys/block/{name}/queue/rotational"), rot)
            .with(&format!("/sys/block/{name}/device/model"), model);
        src.links.insert(format!("/sys/block/{name}"), link);
        let s = read_storage(&src);
        prop_assert!(s.disks.len() <= 1);
    }
}
