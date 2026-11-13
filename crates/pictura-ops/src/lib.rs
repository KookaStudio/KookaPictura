//! Image operations for Kooka Pictura.
//!
//! M10 scope: the destructive `Image > Image Size / Canvas Size / Image Rotation`
//! math as pure functions over a planar 8-bit [`PixelBuffer`] (channels 1..=4).
//! Specs live in `docs/04-image-ops/` (`IMG-001`, `IMG-002`, `IMG-003`).
//!
//! The closed resample kernels are approximated where the specs say so; each
//! approximation is marked inline. Bad input returns [`OpsError`] instead of
//! panicking. Every operation returns a new buffer; the input is untouched.

use pictura_core::PixelBuffer;

pub mod canvas;
pub mod orient;
pub mod resize;

pub use canvas::{resize_canvas, Anchor};
pub use orient::*;
pub use resize::{resize, Resample};

#[derive(Debug, thiserror::Error)]
pub enum OpsError {
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("invalid parameters: {0}")]
    InvalidParams(String),
    #[error(transparent)]
    Memory(#[from] pictura_core::memory::MemoryError),
}

pub(crate) fn validate(buf: &PixelBuffer) -> Result<usize, OpsError> {
    if buf.channels == 0 || buf.channels > 4 {
        return Err(OpsError::Unsupported(format!(
            "channel count {} is not supported (expected 1..=4)",
            buf.channels
        )));
    }
    let n = buf.pixel_count();
    if n == 0 {
        return Err(OpsError::InvalidParams("empty buffer".into()));
    }
    if buf.data.len() != n * buf.channels as usize {
        return Err(OpsError::InvalidParams(
            "buffer length does not match width*height*channels".into(),
        ));
    }
    Ok(n)
}
