//! Apple QuickDraw PICT read and write (bitmap-only).
//!
//! The reader accepts version 1 and extended version 2 pictures, with or
//! without the 512-byte file header. It decodes the bitmap and pixmap records
//! Photoshop opens (`BitsRect`/`BitsRgn`, `PackBitsRect`/`PackBitsRgn`,
//! `DirectBitsRect`/`DirectBitsRgn`) at 1, 2, 4, 8, 16 and 32 bits, and the
//! `$8200` QuickTime JPEG record. QuickDraw vector drawing records are skipped;
//! a picture with no bitmap record is refused.
//!
//! Format facts: *Inside Macintosh: Imaging With QuickDraw*, Appendix A
//! (picture opcodes), and *Inside Macintosh: QuickTime* for the QuickTime image
//! description carried by `CompressedQuickTime`.

pub(crate) mod packbits;
mod read;
#[cfg(test)]
mod tests;

pub use read::{decode_pict, decode_pict_rgba, PictError, PictImage};
