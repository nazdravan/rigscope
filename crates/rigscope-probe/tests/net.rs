#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use rigscope_model::*;
use rigscope_probe::*;
use std::path::PathBuf;

fn fixture(name: &str) -> FixtureSource {
    FixtureSource::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name),
    )
}

#[test]
fn desktop_ethernet_hides_virtual_and_loopback() {
    let s = build_snapshot(&fixture("desktop-ddr4"), None, 0);
    assert_eq!(s.net.interfaces.len(), 1);
    assert_eq!(s.net.hidden_virtual, 1, "docker0; lo is never counted");
    let e = &s.net.interfaces[0];
    assert_eq!(e.name, "enp4s0");
    assert_eq!(e.kind, Field::Known(NetKind::Ethernet));
    assert_eq!(e.mac, Field::Known("a8:a1:59:12:34:56".into()));
    assert_eq!(e.state, Field::Known("up".into()));
    assert_eq!(e.speed_mbps, Field::Known(1000));
    assert_eq!(e.duplex, Field::Known("full".into()));
    assert_eq!(e.mtu, Field::Known(1500));
    assert_eq!(e.driver, Field::Known("igb".into()));
    assert_eq!(e.vendor_name, Field::Known("Intel Corporation".into()));
    assert_eq!(
        e.device_name,
        Field::Known("I211 Gigabit Network Connection".into())
    );
}

#[test]
fn laptop_wifi_has_no_link_speed() {
    let s = build_snapshot(&fixture("laptop-ddr5"), None, 0);
    let w = &s.net.interfaces[0];
    assert_eq!(w.kind, Field::Known(NetKind::WiFi));
    assert_eq!(w.driver, Field::Known("mt7921e".into()));
    assert!(
        matches!(w.speed_mbps, Field::Unavailable(_)),
        "Wi-Fi rate is not in sysfs"
    );
    assert_eq!(
        w.duplex,
        Field::Unavailable(WIFI_NO_DUPLEX.into()),
        "duplex is a wired-link notion"
    );
    assert_eq!(
        w.device_name,
        Field::Known("MT7922 802.11ax PCI Express Wireless Network Adapter".into())
    );
}

#[test]
fn vm_virtio_reports_unknown_speed_not_minus_one() {
    let s = build_snapshot(&fixture("minimal-vm"), None, 0);
    let e = &s.net.interfaces[0];
    assert_eq!(e.driver, Field::Known("virtio_net".into()));
    assert_eq!(e.speed_mbps, Field::Unknown);
    assert_eq!(e.duplex, Field::Unknown);
    assert_eq!(e.vendor_name, Field::Unknown, "not a PCI function");
}

#[test]
fn garbage_values_do_not_panic() {
    let mut m = MemSource::default()
        .with("/sys/class/net/eth9/address", vec![0xff, 0x00])
        .with("/sys/class/net/eth9/speed", "fast\n")
        .with("/sys/class/net/eth9/mtu", "-5\n");
    m.links.insert(
        "/sys/class/net/eth9".into(),
        "/sys/devices/pci0000:00/0000:00:03.0/net/eth9".into(),
    );
    let s = build_snapshot(&m, None, 0);
    assert_eq!(s.net.interfaces[0].speed_mbps, Field::Unknown);
    assert_eq!(s.net.interfaces[0].mtu, Field::Unknown);
}
