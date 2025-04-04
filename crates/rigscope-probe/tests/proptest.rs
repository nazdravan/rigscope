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

    #[test]
    fn read_net_never_panics(name in "[a-z]{2,6}[0-9]{0,2}", speed in ".{0,12}", mtu in ".{0,8}",
                             mac in proptest::collection::vec(any::<u8>(), 0..40), link in ".{0,60}") {
        let mut src = MemSource::default()
            .with(&format!("/sys/class/net/{name}/speed"), speed)
            .with(&format!("/sys/class/net/{name}/mtu"), mtu)
            .with(&format!("/sys/class/net/{name}/address"), mac);
        src.links.insert(format!("/sys/class/net/{name}"), link);
        prop_assert!(read_net(&src).interfaces.len() <= 1);
    }

    #[test]
    fn read_power_never_panics(t in "(Battery|Mains|UPS|.{0,6})", full in ".{0,14}", design in ".{0,14}",
                               volt in ".{0,14}", cap in ".{0,5}", cyc in ".{0,8}") {
        let src = MemSource::default()
            .with("/sys/class/power_supply/BAT0/type", t)
            .with("/sys/class/power_supply/BAT0/charge_full", full)
            .with("/sys/class/power_supply/BAT0/charge_full_design", design)
            .with("/sys/class/power_supply/BAT0/voltage_min_design", volt)
            .with("/sys/class/power_supply/BAT0/capacity", cap)
            .with("/sys/class/power_supply/BAT0/cycle_count", cyc);
        let p = read_power(&src);
        for b in &p.batteries {
            prop_assert!(b.charge_percent.known().is_none_or(|c| *c <= 100));
        }
    }

    #[test]
    fn read_sensors_never_panics(name in ".{0,12}", t in ".{0,14}", f in ".{0,10}", label in ".{0,20}", n in 0u32..200) {
        let src = MemSource::default()
            .with("/sys/class/hwmon/hwmon0/name", name)
            .with(&format!("/sys/class/hwmon/hwmon0/temp{n}_input"), t)
            .with(&format!("/sys/class/hwmon/hwmon0/temp{n}_label"), label)
            .with(&format!("/sys/class/hwmon/hwmon0/fan{n}_input"), f);
        let mut s = read_sensors(&src);
        refresh_sensors(&src, &mut s);
        prop_assert!(s.chips.len() <= 1);
    }

    #[test]
    fn parse_smartctl_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..512),
                                   n in any::<i64>(), id in 0u64..300) {
        let _ = parse_smartctl(&bytes);
        let doc = format!(r#"{{"smartctl":{{"exit_status":{n}}},"temperature":{{"current":{n}}},"ata_smart_attributes":{{"table":[{{"id":{id},"raw":{{"value":{n}}}}}]}},"nvme_smart_health_information_log":{{"data_units_written":{n},"percentage_used":{n}}}}}"#);
        let _ = parse_smartctl(doc.as_bytes());
    }
}
