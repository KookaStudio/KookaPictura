#[derive(Debug, thiserror::Error)]
pub enum PsdError {
    #[error("not a PSD/PSB: bad signature {0:#06x}")]
    BadSignature(u32),
    #[error("unsupported: PSD version {0}")]
    NativeVersion(u16),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("truncated file")]
    Truncated,
    #[error("invalid data: {0}")]
    Invalid(String),
}
