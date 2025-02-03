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

    #[test]
    fn read_gpus_never_panics(card in "card[0-9]{1,2}(-[A-Z]{1,4}-[0-9])?", vendor in ".{0,10}", vram in ".{0,24}",
                              speed in ".{0,20}", width in ".{0,4}", ids in ".{0,200}", drv in ".{0,40}") {
        let mut src = MemSource::default()
            .with(&format!("/sys/class/drm/{card}/device/vendor"), vendor)
            .with(&format!("/sys/class/drm/{card}/device/device"), "0x2484")
            .with(&format!("/sys/class/drm/{card}/device/mem_info_vram_total"), vram)
            .with(&format!("/sys/class/drm/{card}/device/current_link_speed"), speed)
            .with(&format!("/sys/class/drm/{card}/device/current_link_width"), width)
            .with("/usr/share/hwdata/pci.ids", ids);
        src.links.insert(format!("/sys/class/drm/{card}/device/driver"), drv);
        prop_assert!(read_gpus(&src).gpus.len() <= 1);
    }
}
