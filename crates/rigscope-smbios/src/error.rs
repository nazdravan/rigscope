use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SmbiosError {
    #[error("not an SMBIOS entry point")]
    BadEntryPoint,
    #[error("SMBIOS table larger than {max} bytes", max = crate::MAX_TABLE_LEN)]
    TooLarge,
    #[error("SMBIOS table is truncated")]
    Truncated,
    #[error("bad RawSMBIOSData header")]
    BadRsmb,
}
