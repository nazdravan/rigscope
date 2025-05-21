/// The dump bytes are not a usable rigscope DMI dump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumpError {
    /// Wrong magic, truncated, or inconsistent.
    BadDump,
    /// A part is larger than [`MAX_DMI_PART`].
    DumpTooLarge,
}

impl std::fmt::Display for DumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DumpError::BadDump => f.write_str("not a rigscope DMI dump"),
            DumpError::DumpTooLarge => {
                write!(f, "DMI dump part larger than {MAX_DMI_PART} bytes")
            }
        }
    }
}

impl std::error::Error for DumpError {}

/// Most disks a dump may carry SMART data for (the probe reads no more than this many).
pub const MAX_DUMP_DISKS: usize = 256;

/// Largest entry point or table accepted in a dump.
pub const MAX_DMI_PART: usize = 1024 * 1024;

/// The two DMI files as read by the privileged helper.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DmiDump {
    pub entry: Vec<u8>,
    pub table: Vec<u8>,
    /// `(disk name, smartctl --json output)` per disk (format v2).
    pub smart: Vec<(String, Vec<u8>)>,
}

const MAGIC_V1: &[u8] = b"RSDMI1";
const MAGIC_V2: &[u8] = b"RSDMI2";

/// v2: `RSDMI2` + part(entry) + part(table) + u32le count + count × (part(name) + part(json)),
/// where part = u32le length + bytes.
pub fn encode_dmi_dump(d: &DmiDump) -> Vec<u8> {
    let mut out = MAGIC_V2.to_vec();
    let mut part = |bytes: &[u8]| {
        out.extend(u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
        out.extend_from_slice(bytes);
    };
    part(&d.entry);
    part(&d.table);
    let count = u32::try_from(d.smart.len())
        .unwrap_or(u32::MAX)
        .to_le_bytes();
    out.extend(count);
    for (name, json) in &d.smart {
        let mut part = |bytes: &[u8]| {
            out.extend(u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
            out.extend_from_slice(bytes);
        };
        part(name.as_bytes());
        part(json);
    }
    out
}

/// Decodes v2, and v1 (no SMART) from older helpers.
pub fn decode_dmi_dump(b: &[u8]) -> Result<DmiDump, DumpError> {
    let (v2, rest) = match (b.strip_prefix(MAGIC_V2), b.strip_prefix(MAGIC_V1)) {
        (Some(rest), _) => (true, rest),
        (None, Some(rest)) => (false, rest),
        (None, None) => return Err(DumpError::BadDump),
    };
    let (entry, rest) = take_part(rest)?;
    let (table, mut rest) = take_part(rest)?;
    let mut smart = Vec::new();
    if v2 {
        let count: [u8; 4] = rest
            .get(..4)
            .and_then(|s| s.try_into().ok())
            .ok_or(DumpError::BadDump)?;
        let count = u32::from_le_bytes(count);
        if usize::try_from(count).map_or(true, |c| c > MAX_DUMP_DISKS) {
            return Err(DumpError::BadDump);
        }
        rest = rest.get(4..).ok_or(DumpError::BadDump)?;
        for _ in 0..count {
            let (name, r) = take_part(rest)?;
            let (json, r) = take_part(r)?;
            let name = std::str::from_utf8(name)
                .map_err(|_| DumpError::BadDump)?
                .to_string();
            smart.push((name, json.to_vec()));
            rest = r;
        }
    }
    if !rest.is_empty() {
        return Err(DumpError::BadDump);
    }
    Ok(DmiDump {
        entry: entry.to_vec(),
        table: table.to_vec(),
        smart,
    })
}

fn take_part(b: &[u8]) -> Result<(&[u8], &[u8]), DumpError> {
    let len_bytes: [u8; 4] = b
        .get(..4)
        .and_then(|s| s.try_into().ok())
        .ok_or(DumpError::BadDump)?;
    let len =
        usize::try_from(u32::from_le_bytes(len_bytes)).map_err(|_| DumpError::DumpTooLarge)?;
    if len > MAX_DMI_PART {
        return Err(DumpError::DumpTooLarge);
    }
    let body = b.get(4..).ok_or(DumpError::BadDump)?;
    let part = body.get(..len).ok_or(DumpError::BadDump)?;
    let rest = body.get(len..).ok_or(DumpError::BadDump)?;
    Ok((part, rest))
}
