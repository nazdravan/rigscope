#![allow(unsafe_code)]

use super::{from_error, Device};
use crate::layout::{
    battery_info_query, battery_status_query, parse_battery_info, parse_battery_status, utf16z,
    ven_dev, BATTERY_CAPACITY_RELATIVE, BATTERY_DEVICE_NAME, BATTERY_INFORMATION,
    BATTERY_MANUFACTURE_NAME, BATTERY_SERIAL_NUMBER, IOCTL_BATTERY_QUERY_INFORMATION,
    IOCTL_BATTERY_QUERY_STATUS, IOCTL_BATTERY_QUERY_TAG,
};
use crate::{BatteryRaw, PciDisplayRaw, WinError};
use windows::core::{GUID, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiEnumDeviceInterfaces,
    SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW, SetupDiGetDevicePropertyW,
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, GUID_DEVCLASS_DISPLAY, HDEVINFO,
    SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_DriverVersion, DEVPKEY_Device_HardwareIds, DEVPKEY_Device_LocationInfo,
    DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::Win32::System::Power::GUID_DEVICE_BATTERY;

const MAX_DEVICES: u32 = 64;
/// Bytes for one device property or one interface detail record.
const BUF: usize = 2048;

/// pciprop.h. windows 0.62.2 only exports these under NetworkManagement::WiFi.
const PCI_PROPS: GUID = GUID::from_u128(0x3ab22e31_8264_4b4e_9af5_a8d2d8e33e62);
const DEVPKEY_PCI_CURRENT_LINK_SPEED: DEVPROPKEY = DEVPROPKEY {
    fmtid: PCI_PROPS,
    pid: 9,
};
const DEVPKEY_PCI_CURRENT_LINK_WIDTH: DEVPROPKEY = DEVPROPKEY {
    fmtid: PCI_PROPS,
    pid: 10,
};

/// Owns an HDEVINFO.
struct DevInfo(HDEVINFO);

impl DevInfo {
    fn new(guid: &GUID, interfaces: bool) -> Result<DevInfo, WinError> {
        let flags = if interfaces {
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE
        } else {
            DIGCF_PRESENT
        };
        // SAFETY: `guid` is a live reference for the call; no enumerator string, no window.
        // The returned list is owned by `DevInfo` and destroyed on drop.
        let h = unsafe { SetupDiGetClassDevsW(Some(guid), PCWSTR::null(), None, flags) }
            .map_err(|e| from_error("SetupDiGetClassDevsW", &e))?;
        Ok(DevInfo(h))
    }
}

impl Drop for DevInfo {
    fn drop(&mut self) {
        // SAFETY: the list came from SetupDiGetClassDevsW and is destroyed exactly once.
        let _ = unsafe { SetupDiDestroyDeviceInfoList(self.0) };
    }
}

fn prop(set: &DevInfo, dev: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<Vec<u8>> {
    let mut kind = DEVPROPTYPE::default();
    let mut buf = vec![0u8; BUF];
    let mut need = 0u32;
    // SAFETY: `buf` is passed as a slice, so its length is the size the API sees; `dev` is an
    // element of `set`; `kind`/`need` are local out-parameters.
    unsafe {
        SetupDiGetDevicePropertyW(
            set.0,
            dev,
            key,
            &mut kind,
            Some(&mut buf),
            Some(&raw mut need),
            0,
        )
    }
    .ok()?;
    buf.truncate(need as usize);
    Some(buf)
}

fn u32_prop(set: &DevInfo, dev: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<u32> {
    let b = prop(set, dev, key)?;
    Some(u32::from_le_bytes(b.get(..4)?.try_into().ok()?))
}

pub fn pci_display() -> Result<Vec<PciDisplayRaw>, WinError> {
    let set = DevInfo::new(&GUID_DEVCLASS_DISPLAY, false)?;
    let mut out = Vec::new();
    for i in 0..MAX_DEVICES {
        let mut dev = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: `dev` has cbSize set and is a live out-parameter; an index past the end
        // fails (ERROR_NO_MORE_ITEMS) and ends the loop.
        if unsafe { SetupDiEnumDeviceInfo(set.0, i, &mut dev) }.is_err() {
            break;
        }
        // HardwareIds is a REG_MULTI_SZ; the first string ("PCI\VEN_…&DEV_…&SUBSYS_…&REV_…")
        // carries the ids.
        let ids = prop(&set, &dev, &DEVPKEY_Device_HardwareIds).and_then(|b| utf16z(&b));
        let Some((vendor_id, device_id)) = ids.as_deref().and_then(ven_dev) else {
            continue;
        };
        out.push(PciDisplayRaw {
            vendor_id,
            device_id,
            driver_version: prop(&set, &dev, &DEVPKEY_Device_DriverVersion)
                .and_then(|b| utf16z(&b)),
            link_speed: u32_prop(&set, &dev, &DEVPKEY_PCI_CURRENT_LINK_SPEED),
            link_width: u32_prop(&set, &dev, &DEVPKEY_PCI_CURRENT_LINK_WIDTH),
            location: prop(&set, &dev, &DEVPKEY_Device_LocationInfo).and_then(|b| utf16z(&b)),
        });
    }
    Ok(out)
}

fn interface_paths(guid: &GUID) -> Result<Vec<String>, WinError> {
    let set = DevInfo::new(guid, true)?;
    let mut out = Vec::new();
    for i in 0..MAX_DEVICES {
        let mut itf = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: `itf` has cbSize set and is a live out-parameter; `guid` is live; an index
        // past the end fails and ends the loop.
        if unsafe { SetupDiEnumDeviceInterfaces(set.0, None, guid, i, &mut itf) }.is_err() {
            break;
        }
        // u32 storage: SP_DEVICE_INTERFACE_DETAIL_DATA_W needs 4-byte alignment. Its first
        // field is cbSize, which must be the fixed header size (8 on x64, 6 on x86).
        let mut store = vec![0u32; BUF / 4];
        if let Some(cb) = store.first_mut() {
            *cb = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        }
        let detail = store
            .as_mut_ptr()
            .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        // SAFETY: `detail` points at `store`, BUF bytes, 4-aligned, with cbSize set; BUF is
        // the size passed. `itf` came from this `set`.
        let r = unsafe {
            SetupDiGetDeviceInterfaceDetailW(set.0, &itf, Some(detail), BUF as u32, None, None)
        };
        if r.is_err() {
            continue;
        }
        // DevicePath starts right after cbSize.
        let bytes: Vec<u8> = store.iter().flat_map(|w| w.to_le_bytes()).skip(4).collect();
        if let Some(p) = utf16z(&bytes) {
            out.push(p);
        }
    }
    Ok(out)
}

fn u32_le(b: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(..4)?.try_into().ok()?))
}

pub fn batteries() -> Result<Vec<BatteryRaw>, WinError> {
    let mut out = Vec::new();
    for p in interface_paths(&GUID_DEVICE_BATTERY)?.into_iter().take(8) {
        let Ok(dev) = Device::open(&p, true) else {
            continue;
        };
        // Input: a wait timeout of 0 (answer now). Tag 0 = no battery in this slot.
        let Some(tag) = dev
            .ioctl(IOCTL_BATTERY_QUERY_TAG, &0u32.to_le_bytes(), 4)
            .ok()
            .and_then(|b| u32_le(&b))
            .filter(|t| *t != 0)
        else {
            continue;
        };
        let text = |level| {
            dev.ioctl(
                IOCTL_BATTERY_QUERY_INFORMATION,
                &battery_info_query(tag, level),
                512,
            )
            .ok()
            .and_then(|b| utf16z(&b))
        };
        let info = dev
            .ioctl(
                IOCTL_BATTERY_QUERY_INFORMATION,
                &battery_info_query(tag, BATTERY_INFORMATION),
                64,
            )
            .ok()
            .and_then(|b| parse_battery_info(&b));
        let status = dev
            .ioctl(IOCTL_BATTERY_QUERY_STATUS, &battery_status_query(tag), 16)
            .ok()
            .and_then(|b| parse_battery_status(&b));
        out.push(BatteryRaw {
            name: text(BATTERY_DEVICE_NAME),
            manufacturer: text(BATTERY_MANUFACTURE_NAME),
            serial: text(BATTERY_SERIAL_NUMBER),
            chemistry: info
                .as_ref()
                .map(|i| i.chemistry.clone())
                .filter(|c| !c.is_empty()),
            relative: info
                .as_ref()
                .is_some_and(|i| i.capabilities & BATTERY_CAPACITY_RELATIVE != 0),
            designed: info.as_ref().map(|i| i.designed),
            full: info.as_ref().map(|i| i.full),
            now: status.as_ref().map(|s| s.capacity),
            cycles: info.as_ref().map(|i| i.cycles),
            power_state: status.map(|s| s.power_state),
        });
    }
    Ok(out)
}
