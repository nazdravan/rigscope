//! Byte layouts of the Windows IOCTLs rigscope uses: requests are built and responses are
//! parsed here, in safe code, so the FFI only moves bytes. Every read is bounds-checked.

pub const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;
pub const IOCTL_DISK_GET_LENGTH_INFO: u32 = 0x0007_405C;
pub const SMART_RCV_DRIVE_DATA: u32 = 0x0007_C088;
pub const IOCTL_BATTERY_QUERY_TAG: u32 = 0x0029_4040;
pub const IOCTL_BATTERY_QUERY_INFORMATION: u32 = 0x0029_4044;
pub const IOCTL_BATTERY_QUERY_STATUS: u32 = 0x0029_404C;

/// `BATTERY_QUERY_INFORMATION_LEVEL` values.
pub const BATTERY_INFORMATION: u32 = 0;
pub const BATTERY_DEVICE_NAME: u32 = 4;
pub const BATTERY_MANUFACTURE_NAME: u32 = 6;
pub const BATTERY_SERIAL_NUMBER: u32 = 8;
/// `BATTERY_INFORMATION.Capabilities` bit: capacities are relative, not mWh.
pub const BATTERY_CAPACITY_RELATIVE: u32 = 0x4000_0000;
/// `BATTERY_UNKNOWN_CAPACITY`.
pub const BATTERY_UNKNOWN: u32 = 0xFFFF_FFFF;

pub const ATA_READ_ATTRIBUTES: u8 = 0xD0;
pub const ATA_READ_THRESHOLDS: u8 = 0xD1;
/// `SENDCMDOUTPARAMS`: cBufferSize (4) + DRIVERSTATUS (12) + 512 data bytes.
pub const ATA_SMART_OUT_LEN: usize = 16 + 512;

const NVME_LOG_LEN: u32 = 512;
/// `STORAGE_PROTOCOL_SPECIFIC_DATA` is ten u32.
const PROTOCOL_DATA_LEN: u32 = 40;
/// `STORAGE_PROTOCOL_DATA_DESCRIPTOR`: Version, Size, then the protocol data (40 bytes).
pub const NVME_HEALTH_OUT_LEN: usize = 8 + 40 + 512;

fn put(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    let s = b.get(off..off.checked_add(4)?)?;
    Some(u32::from_le_bytes(s.try_into().ok()?))
}

/// `STORAGE_PROPERTY_QUERY { StorageDeviceProperty (0), PropertyStandardQuery (0) }`.
pub fn device_descriptor_query() -> Vec<u8> {
    let mut v = Vec::with_capacity(12);
    put(&mut v, 0);
    put(&mut v, 0);
    put(&mut v, 0); // AdditionalParameters[1]
    v
}

/// NVMe SMART/health log (0x02) through `StorageDeviceProtocolSpecificProperty`.
pub fn nvme_health_query() -> Vec<u8> {
    let mut v = Vec::with_capacity(NVME_HEALTH_OUT_LEN);
    put(&mut v, 50); // StorageDeviceProtocolSpecificProperty
    put(&mut v, 0); // PropertyStandardQuery
    put(&mut v, 3); // ProtocolTypeNvme
    put(&mut v, 2); // NVMeDataTypeLogPage
    put(&mut v, 2); // ProtocolDataRequestValue: log id
    put(&mut v, 0); // ProtocolDataRequestSubValue
    put(&mut v, PROTOCOL_DATA_LEN); // ProtocolDataOffset (from the protocol data start)
    put(&mut v, NVME_LOG_LEN); // ProtocolDataLength
    for _ in 0..4 {
        put(&mut v, 0); // FixedProtocolReturnData, SubValue2..4
    }
    v.resize(NVME_HEALTH_OUT_LEN, 0);
    v
}

/// The 512-byte log inside a `STORAGE_PROTOCOL_DATA_DESCRIPTOR`.
pub fn nvme_log_from_response(out: &[u8]) -> Option<&[u8]> {
    // ProtocolSpecificData starts at 8; its DataOffset/DataLength are fields 4 and 5.
    let off = usize::try_from(u32_at(out, 8 + 16)?).ok()?;
    let len = usize::try_from(u32_at(out, 8 + 20)?).ok()?;
    // DataOffset counts from ProtocolSpecificData, whose header is 40 bytes; a smaller
    // offset would hand back the descriptor and protocol header as if it were the log.
    if len != NVME_LOG_LEN as usize || off < 40 {
        return None;
    }
    let start = 8usize.checked_add(off)?;
    out.get(start..start.checked_add(len)?)
}

/// `SENDCMDINPARAMS` (packed, without the trailing buffer byte) for SMART READ DATA/THRESHOLDS.
pub fn ata_smart_request(drive: u8, feature: u8) -> Vec<u8> {
    let mut v = Vec::with_capacity(32);
    put(&mut v, 512); // cBufferSize
    v.extend_from_slice(&[
        feature,                   // bFeaturesReg
        1,                         // bSectorCountReg
        1,                         // bSectorNumberReg
        0x4F,                      // bCylLowReg (SMART_CYL_LOW)
        0xC2,                      // bCylHighReg (SMART_CYL_HI)
        0xA0 | ((drive & 1) << 4), // bDriveHeadReg
        0xB0,                      // bCommandReg (SMART_CMD)
        0,                         // bReserved
    ]);
    v.push(drive); // bDriveNumber
    v.resize(32, 0); // bReserved[3] + dwReserved[4]
    v
}

pub fn ata_data_from_response(out: &[u8]) -> Option<&[u8]> {
    out.get(16..16 + 512)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StorageDescriptor {
    pub bus_type: Option<u16>,
    pub vendor: Option<String>,
    pub product: Option<String>,
    pub revision: Option<String>,
    pub serial: Option<String>,
}

/// A NUL-terminated ASCII string at `off` (0 = absent), trimmed.
fn ascii_at(b: &[u8], off: u32) -> Option<String> {
    if off == 0 {
        return None;
    }
    let rest = b.get(usize::try_from(off).ok()?..)?;
    let end = rest.iter().position(|c| *c == 0).unwrap_or(rest.len());
    let s = String::from_utf8_lossy(rest.get(..end)?).trim().to_string();
    (!s.is_empty()).then_some(s)
}

pub fn parse_device_descriptor(b: &[u8]) -> Option<StorageDescriptor> {
    let bus = u32_at(b, 28)?;
    Some(StorageDescriptor {
        bus_type: u16::try_from(bus).ok(),
        vendor: ascii_at(b, u32_at(b, 12)?),
        product: ascii_at(b, u32_at(b, 16)?),
        revision: ascii_at(b, u32_at(b, 20)?),
        serial: ascii_at(b, u32_at(b, 24)?),
    })
}

/// `BATTERY_QUERY_INFORMATION { BatteryTag, InformationLevel, AtRate }`.
pub fn battery_info_query(tag: u32, level: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(12);
    put(&mut v, tag);
    put(&mut v, level);
    put(&mut v, 0);
    v
}

/// `BATTERY_WAIT_STATUS { BatteryTag, Timeout = 0, PowerState, LowCapacity, HighCapacity }`.
pub fn battery_status_query(tag: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(20);
    put(&mut v, tag);
    for _ in 0..4 {
        put(&mut v, 0);
    }
    v
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BatteryInfo {
    pub capabilities: u32,
    pub chemistry: String,
    pub designed: u32,
    pub full: u32,
    pub cycles: u32,
}

/// `BATTERY_INFORMATION` (batclass.h, 36 bytes): Capabilities@0, Technology@4, Reserved[3]@5,
/// Chemistry[4]@8, DesignedCapacity@12, FullChargedCapacity@16, DefaultAlert1@20,
/// DefaultAlert2@24, CriticalBias@28, CycleCount@32.
pub fn parse_battery_info(b: &[u8]) -> Option<BatteryInfo> {
    let chem = b.get(8..12)?;
    Some(BatteryInfo {
        capabilities: u32_at(b, 0)?,
        chemistry: String::from_utf8_lossy(chem)
            .trim_matches(char::from(0))
            .trim()
            .to_string(),
        designed: u32_at(b, 12)?,
        full: u32_at(b, 16)?,
        cycles: u32_at(b, 32)?,
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BatteryStatus {
    pub power_state: u32,
    pub capacity: u32,
}

/// `BATTERY_STATUS { PowerState, Capacity, Voltage, Rate }`.
pub fn parse_battery_status(b: &[u8]) -> Option<BatteryStatus> {
    Some(BatteryStatus {
        power_state: u32_at(b, 0)?,
        capacity: u32_at(b, 4)?,
    })
}

/// A NUL-terminated UTF-16LE string (the battery name IOCTLs return these).
pub fn utf16z(b: &[u8]) -> Option<String> {
    let units: Vec<u16> = b
        .chunks_exact(2)
        .map(|c| {
            u16::from_le_bytes([
                c.first().copied().unwrap_or(0),
                c.get(1).copied().unwrap_or(0),
            ])
        })
        .take_while(|u| *u != 0)
        .collect();
    let s = String::from_utf16_lossy(&units).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// PDH `Processor Information` instance "g,n" → logical CPU g×64+n; totals ("_Total",
/// "0,_Total") → None.
pub fn pdh_cpu(instance: &str) -> Option<u32> {
    let (g, n) = instance.split_once(',')?;
    let g: u32 = g.trim().parse().ok()?;
    let n: u32 = n.trim().parse().ok()?;
    g.checked_mul(64)?.checked_add(n)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]
    use super::*;

    fn u32le(b: &[u8], off: usize) -> u32 {
        u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
    }

    #[test]
    fn nvme_query_layout() {
        let q = nvme_health_query();
        assert_eq!(q.len(), 8 + 40 + 512);
        assert_eq!(u32le(&q, 0), 50); // StorageDeviceProtocolSpecificProperty
        assert_eq!(u32le(&q, 4), 0); // PropertyStandardQuery
        assert_eq!(u32le(&q, 8), 3); // ProtocolTypeNvme
        assert_eq!(u32le(&q, 12), 2); // NVMeDataTypeLogPage
        assert_eq!(u32le(&q, 16), 2); // log 0x02, SMART / health
        assert_eq!(u32le(&q, 24), 40); // ProtocolDataOffset
        assert_eq!(u32le(&q, 28), 512); // ProtocolDataLength
    }

    #[test]
    fn nvme_response_extracts_the_log() {
        let mut out = vec![0u8; NVME_HEALTH_OUT_LEN];
        out[24..28].copy_from_slice(&40u32.to_le_bytes());
        out[28..32].copy_from_slice(&512u32.to_le_bytes());
        out[48] = 0xAB; // first byte of the log (8 + 40)
        let log = nvme_log_from_response(&out).unwrap();
        assert_eq!(log.len(), 512);
        assert_eq!(log[0], 0xAB);
        out[28..32].copy_from_slice(&4096u32.to_le_bytes()); // lying length
        assert_eq!(nvme_log_from_response(&out), None);
        assert_eq!(nvme_log_from_response(&[0; 10]), None);
    }

    #[test]
    fn nvme_log_offset_is_bounded_below_by_the_header() {
        let mut out = vec![0u8; 8 + 48 + 512];
        out[24..28].copy_from_slice(&48u32.to_le_bytes());
        out[28..32].copy_from_slice(&512u32.to_le_bytes());
        out[56] = 0xCD;
        assert_eq!(nvme_log_from_response(&out).unwrap()[0], 0xCD);
        out[24..28].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(nvme_log_from_response(&out), None);
    }

    #[test]
    fn ata_request_layout() {
        let r = ata_smart_request(1, ATA_READ_ATTRIBUTES);
        assert_eq!(r.len(), 32);
        assert_eq!(u32le(&r, 0), 512);
        assert_eq!(
            &r[4..12],
            &[0xD0, 1, 1, 0x4F, 0xC2, 0xA0 | (1 << 4), 0xB0, 0]
        );
        assert_eq!(r[12], 1);
        let mut out = vec![0u8; ATA_SMART_OUT_LEN];
        out[16] = 0x10;
        assert_eq!(ata_data_from_response(&out).unwrap()[0], 0x10);
        assert_eq!(ata_data_from_response(&out[..100]), None);
    }

    #[test]
    fn device_descriptor_strings_and_bus() {
        // STORAGE_DEVICE_DESCRIPTOR: Version, Size, DeviceType, DeviceTypeModifier,
        // RemovableMedia, CommandQueueing, VendorIdOffset@12, ProductIdOffset@16,
        // ProductRevisionOffset@20, SerialNumberOffset@24, BusType@28, RawPropertiesLength@32.
        let mut d = vec![0u8; 36];
        let push = |d: &mut Vec<u8>, at: usize, s: &str| {
            let off = d.len() as u32;
            d[at..at + 4].copy_from_slice(&off.to_le_bytes());
            d.extend_from_slice(s.as_bytes());
            d.push(0);
        };
        push(&mut d, 16, "Samsung SSD 970 EVO Plus 500GB  ");
        push(&mut d, 20, "2B2QEXM7");
        push(&mut d, 24, "  S4EVNX0N123456A");
        d[28..32].copy_from_slice(&17u32.to_le_bytes());
        let p = parse_device_descriptor(&d).unwrap();
        assert_eq!(p.bus_type, Some(17));
        assert_eq!(p.product.as_deref(), Some("Samsung SSD 970 EVO Plus 500GB"));
        assert_eq!(p.serial.as_deref(), Some("S4EVNX0N123456A"));
        assert_eq!(p.revision.as_deref(), Some("2B2QEXM7"));
        assert_eq!(p.vendor, None); // offset 0 = absent
        d[16..20].copy_from_slice(&9999u32.to_le_bytes()); // offset outside the buffer
        assert_eq!(parse_device_descriptor(&d).unwrap().product, None);
    }

    #[test]
    fn battery_layouts() {
        let q = battery_info_query(7, 6);
        assert_eq!((u32le(&q, 0), u32le(&q, 4), q.len()), (7, 6, 12));
        let mut info = vec![0u8; 36];
        info[0..4].copy_from_slice(&0x8000_0000u32.to_le_bytes());
        info[8..12].copy_from_slice(b"LION");
        info[12..16].copy_from_slice(&57_000u32.to_le_bytes());
        info[16..20].copy_from_slice(&52_440u32.to_le_bytes());
        info[28..32].copy_from_slice(&5u32.to_le_bytes()); // CriticalBias, not cycles
        info[32..36].copy_from_slice(&87u32.to_le_bytes());
        let i = parse_battery_info(&info).unwrap();
        assert_eq!(
            (i.chemistry.as_str(), i.designed, i.full, i.cycles),
            ("LION", 57_000, 52_440, 87)
        );
        let mut st = vec![0u8; 16];
        st[0..4].copy_from_slice(&2u32.to_le_bytes());
        st[4..8].copy_from_slice(&43_000u32.to_le_bytes());
        let s = parse_battery_status(&st).unwrap();
        assert_eq!((s.power_state, s.capacity), (2, 43_000));
        assert_eq!(battery_status_query(7).len(), 20);
        assert_eq!(parse_battery_info(&info[..32]), None);
    }

    #[test]
    fn utf16_strings() {
        let b: Vec<u8> = "SMP\0junk"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(utf16z(&b).as_deref(), Some("SMP"));
        assert_eq!(utf16z(&[0, 0]), None);
    }

    #[test]
    fn pdh_instance_names() {
        assert_eq!(pdh_cpu("0,3"), Some(3));
        assert_eq!(pdh_cpu("1,2"), Some(66));
        assert_eq!(pdh_cpu("_Total"), None);
        assert_eq!(pdh_cpu("0,_Total"), None);
    }
}
