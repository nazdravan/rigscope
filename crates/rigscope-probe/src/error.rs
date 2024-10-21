use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProbeError {
    #[error("not a rigscope DMI dump")]
    BadDump,
    #[error("DMI dump part larger than {max} bytes", max = crate::MAX_DMI_PART)]
    DumpTooLarge,
}
