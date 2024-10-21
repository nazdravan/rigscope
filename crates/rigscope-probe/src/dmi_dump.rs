use crate::ProbeError;

/// Largest entry point or table accepted in a dump.
pub const MAX_DMI_PART: usize = 64 * 1024;

/// The two DMI files as read by the privileged helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmiDump {
    pub entry: Vec<u8>,
    pub table: Vec<u8>,
}

const MAGIC: &[u8] = b"RSDMI1";

/// `RSDMI1` + u32le len + entry + u32le len + table.
pub fn encode_dmi_dump(d: &DmiDump) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    for part in [&d.entry, &d.table] {
        out.extend(u32::try_from(part.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(part);
    }
    out
}

pub fn decode_dmi_dump(b: &[u8]) -> Result<DmiDump, ProbeError> {
    let rest = b.strip_prefix(MAGIC).ok_or(ProbeError::BadDump)?;
    let (entry, rest) = take_part(rest)?;
    let (table, rest) = take_part(rest)?;
    if !rest.is_empty() {
        return Err(ProbeError::BadDump);
    }
    Ok(DmiDump {
        entry: entry.to_vec(),
        table: table.to_vec(),
    })
}

fn take_part(b: &[u8]) -> Result<(&[u8], &[u8]), ProbeError> {
    let len_bytes: [u8; 4] = b
        .get(..4)
        .and_then(|s| s.try_into().ok())
        .ok_or(ProbeError::BadDump)?;
    let len =
        usize::try_from(u32::from_le_bytes(len_bytes)).map_err(|_| ProbeError::DumpTooLarge)?;
    if len > MAX_DMI_PART {
        return Err(ProbeError::DumpTooLarge);
    }
    let body = b.get(4..).ok_or(ProbeError::BadDump)?;
    let part = body.get(..len).ok_or(ProbeError::BadDump)?;
    let rest = body.get(len..).ok_or(ProbeError::BadDump)?;
    Ok((part, rest))
}
