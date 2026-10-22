//! Qt-free raster image header probe and allocation budget.
//!
//! Parses just enough of a container header to name the format and its declared
//! dimensions, so an attacker-supplied file can be refused before a decoder is
//! asked to allocate. The declared size is attacker-controlled, so the result is
//! advisory; the app pairs it with a check on the actual decoded allocation.

use thiserror::Error;

/// The containers the probe recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
    Tiff,
    WebP,
}

impl ImageFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            ImageFormat::Png => "PNG",
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Gif => "GIF",
            ImageFormat::Bmp => "BMP",
            ImageFormat::Tiff => "TIFF",
            ImageFormat::WebP => "WebP",
        }
    }
}

/// Tunable refusal thresholds for an import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageBudget {
    /// Maximum declared width or height, in pixels.
    pub max_dimension: u32,
    /// Maximum `width * height * 4` decoded allocation, in bytes.
    pub max_alloc_bytes: u64,
    /// Maximum bytes one document's decoded planes may claim in all. The
    /// default is three quarters of the memory available when it is read,
    /// never less than one `max_alloc_bytes` allocation (#844).
    pub max_document_bytes: u64,
}

impl Default for ImageBudget {
    fn default() -> Self {
        const MAX_ALLOC_BYTES: u64 = 2 * 1024 * 1024 * 1024;
        Self {
            max_dimension: 30_000,
            // 2 GiB admits a 16000²-class RGBA import (the canvas target):
            // 16507×16196×4 ≈ 1020 MiB, over the former 512 MiB bound. The
            // 30 000 px dimension cap remains the outer limit. The Qt decode edge
            // raises QImageReader's separate 256 MB allocation limit to match
            // (see crates/pictura-app/cpp/decode_image.cpp).
            max_alloc_bytes: MAX_ALLOC_BYTES,
            max_document_bytes: crate::budget::document_bytes(
                crate::budget::available_memory(),
                MAX_ALLOC_BYTES,
            ),
        }
    }
}

/// The declared metadata read from a container header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageProbe {
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    /// Bits per channel as declared by the header.
    pub bit_depth: u16,
}

/// Which budget limit a refusal named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    Dimension,
    Allocation,
}

/// A typed probe refusal carrying provenance for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImportError {
    #[error("unrecognized image container")]
    UnknownContainer { bytes_read: usize },
    #[error("truncated {container} header")]
    Truncated {
        container: String,
        bytes_read: usize,
    },
    #[error("{container} declared {width}x{height} over the {limit} px dimension limit")]
    DimensionLimit {
        container: String,
        width: u32,
        height: u32,
        limit: u32,
        bytes_read: usize,
    },
    #[error("{container} declared {width}x{height} needs {bytes} bytes over the {limit} byte allocation limit")]
    AllocationLimit {
        container: String,
        width: u32,
        height: u32,
        bytes: u64,
        limit: u64,
        bytes_read: usize,
    },
}

impl ImportError {
    /// Bytes of input the probe examined.
    pub fn bytes_read(&self) -> usize {
        match self {
            ImportError::UnknownContainer { bytes_read }
            | ImportError::Truncated { bytes_read, .. }
            | ImportError::DimensionLimit { bytes_read, .. }
            | ImportError::AllocationLimit { bytes_read, .. } => *bytes_read,
        }
    }

    /// Which limit was exceeded, if any.
    pub fn limit_exceeded(&self) -> Option<LimitKind> {
        match self {
            ImportError::DimensionLimit { .. } => Some(LimitKind::Dimension),
            ImportError::AllocationLimit { .. } => Some(LimitKind::Allocation),
            _ => None,
        }
    }

    /// Container description, or `"unknown"` when nothing matched.
    pub fn source(&self) -> &str {
        match self {
            ImportError::UnknownContainer { .. } => "unknown",
            ImportError::Truncated { container, .. }
            | ImportError::DimensionLimit { container, .. }
            | ImportError::AllocationLimit { container, .. } => container,
        }
    }
}

/// Sniff a raster image header and refuse anything over `budget`.
///
/// Pure and allocation-free apart from the result; never decodes pixels.
pub fn probe_image(bytes: &[u8], budget: ImageBudget) -> Result<ImageProbe, ImportError> {
    let declared = sniff(bytes)?;
    let source = declared.format.as_str();
    if declared.width > budget.max_dimension || declared.height > budget.max_dimension {
        return Err(ImportError::DimensionLimit {
            container: source.into(),
            width: declared.width,
            height: declared.height,
            limit: budget.max_dimension,
            bytes_read: bytes.len(),
        });
    }
    // `width * height * 4` can exceed u64 for attacker-declared dimensions, so
    // saturate: an overflowing estimate is over budget by construction.
    let bytes_needed = (declared.width as u64)
        .checked_mul(declared.height as u64)
        .and_then(|pixels| pixels.checked_mul(4));
    if bytes_needed.is_none_or(|bytes| bytes > budget.max_alloc_bytes) {
        return Err(ImportError::AllocationLimit {
            container: source.into(),
            width: declared.width,
            height: declared.height,
            bytes: bytes_needed.unwrap_or(u64::MAX),
            limit: budget.max_alloc_bytes,
            bytes_read: bytes.len(),
        });
    }
    Ok(declared)
}

fn trunc(source: &str, bytes: &[u8]) -> ImportError {
    ImportError::Truncated {
        container: source.into(),
        bytes_read: bytes.len(),
    }
}

fn sniff(bytes: &[u8]) -> Result<ImageProbe, ImportError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        parse_png(bytes)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        parse_jpeg(bytes)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        parse_gif(bytes)
    } else if bytes.starts_with(b"BM") {
        parse_bmp(bytes)
    } else if bytes.starts_with(b"II\x2a\x00") || bytes.starts_with(b"MM\x00\x2a") {
        parse_tiff(bytes)
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        parse_webp(bytes)
    } else {
        Err(ImportError::UnknownContainer {
            bytes_read: bytes.len(),
        })
    }
}

fn parse_png(bytes: &[u8]) -> Result<ImageProbe, ImportError> {
    // signature(8) + length(4) + "IHDR"(4) + 13 data bytes = 29.
    if bytes.len() < 29 || &bytes[12..16] != b"IHDR" {
        return Err(trunc("PNG", bytes));
    }
    Ok(ImageProbe {
        format: ImageFormat::Png,
        width: be_u32(bytes, 16),
        height: be_u32(bytes, 20),
        bit_depth: bytes[24] as u16,
    })
}

fn parse_jpeg(bytes: &[u8]) -> Result<ImageProbe, ImportError> {
    let mut pos = 2usize;
    while pos + 2 <= bytes.len() {
        if bytes[pos] != 0xFF {
            return Err(trunc("JPEG", bytes));
        }
        let marker = bytes[pos + 1];
        pos += 2;
        if marker == 0xD8 || marker == 0xD9 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        if pos + 2 > bytes.len() {
            return Err(trunc("JPEG", bytes));
        }
        let seg_len = be_u16(bytes, pos) as usize;
        if seg_len < 2 || pos + seg_len > bytes.len() {
            return Err(trunc("JPEG", bytes));
        }
        let is_sof = (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if is_sof {
            if seg_len < 7 {
                return Err(trunc("JPEG", bytes));
            }
            return Ok(ImageProbe {
                format: ImageFormat::Jpeg,
                width: be_u16(bytes, pos + 5) as u32,
                height: be_u16(bytes, pos + 3) as u32,
                bit_depth: bytes[pos + 2] as u16,
            });
        }
        pos += seg_len;
    }
    Err(trunc("JPEG", bytes))
}

fn parse_gif(bytes: &[u8]) -> Result<ImageProbe, ImportError> {
    if bytes.len() < 10 {
        return Err(trunc("GIF", bytes));
    }
    Ok(ImageProbe {
        format: ImageFormat::Gif,
        width: le_u16(bytes, 6) as u32,
        height: le_u16(bytes, 8) as u32,
        bit_depth: 8,
    })
}

fn parse_bmp(bytes: &[u8]) -> Result<ImageProbe, ImportError> {
    if bytes.len() < 18 {
        return Err(trunc("BMP", bytes));
    }
    // Only the Windows BITMAPINFOHEADER family carries the fields we need.
    if le_u32(bytes, 14) < 40 || bytes.len() < 30 {
        return Err(trunc("BMP", bytes));
    }
    let width = le_i32(bytes, 18).unsigned_abs();
    let height = le_i32(bytes, 22).unsigned_abs();
    Ok(ImageProbe {
        format: ImageFormat::Bmp,
        width,
        height,
        bit_depth: le_u16(bytes, 28),
    })
}

fn parse_tiff(bytes: &[u8]) -> Result<ImageProbe, ImportError> {
    if bytes.len() < 8 {
        return Err(trunc("TIFF", bytes));
    }
    let little = bytes[0] == b'I';
    let read16 = |at: usize| {
        if little {
            le_u16(bytes, at)
        } else {
            be_u16(bytes, at)
        }
    };
    let read32 = |at: usize| {
        if little {
            le_u32(bytes, at)
        } else {
            be_u32(bytes, at)
        }
    };
    let ifd = read32(4) as usize;
    if ifd + 2 > bytes.len() {
        return Err(trunc("TIFF", bytes));
    }
    let count = read16(ifd) as usize;
    let entries = ifd + 2;
    let (mut width, mut height, mut bits) = (None, None, None);
    for i in 0..count {
        let entry = entries + i * 12;
        if entry + 12 > bytes.len() {
            return Err(trunc("TIFF", bytes));
        }
        let tag = read16(entry);
        let kind = read16(entry + 2);
        let n = read32(entry + 4);
        let value = read32(entry + 8);
        let resolved = match (kind, n) {
            (3, 1) => value & 0xFFFF,
            (3, _) => {
                let at = value as usize;
                if at + 2 > bytes.len() {
                    return Err(trunc("TIFF", bytes));
                }
                read16(at) as u32
            }
            (4, _) => value,
            _ => continue,
        };
        match tag {
            256 => width = Some(resolved),
            257 => height = Some(resolved),
            258 => bits = Some(resolved),
            _ => {}
        }
    }
    let (Some(width), Some(height)) = (width, height) else {
        return Err(trunc("TIFF", bytes));
    };
    Ok(ImageProbe {
        format: ImageFormat::Tiff,
        width,
        height,
        bit_depth: bits.unwrap_or(8) as u16,
    })
}

fn parse_webp(bytes: &[u8]) -> Result<ImageProbe, ImportError> {
    if bytes.len() < 20 {
        return Err(trunc("WebP", bytes));
    }
    let fourcc = &bytes[12..16];
    match fourcc {
        b"VP8X" => {
            if bytes.len() < 30 {
                return Err(trunc("WebP", bytes));
            }
            Ok(ImageProbe {
                format: ImageFormat::WebP,
                width: le_u24(bytes, 24) + 1,
                height: le_u24(bytes, 27) + 1,
                bit_depth: 8,
            })
        }
        b"VP8L" => {
            if bytes.len() < 25 || bytes[20] != 0x2F {
                return Err(trunc("WebP", bytes));
            }
            let bits = u32::from_le_bytes([bytes[21], bytes[22], bytes[23], bytes[24]]);
            Ok(ImageProbe {
                format: ImageFormat::WebP,
                width: (bits & 0x3FFF) + 1,
                height: ((bits >> 14) & 0x3FFF) + 1,
                bit_depth: 8,
            })
        }
        b"VP8 " => {
            if bytes.len() < 30 || bytes[23..26] != [0x9D, 0x01, 0x2A] {
                return Err(trunc("WebP", bytes));
            }
            Ok(ImageProbe {
                format: ImageFormat::WebP,
                width: (le_u16(bytes, 26) & 0x3FFF) as u32,
                height: (le_u16(bytes, 28) & 0x3FFF) as u32,
                bit_depth: 8,
            })
        }
        _ => Err(trunc("WebP", bytes)),
    }
}

fn be_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

fn le_u16(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn be_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn le_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn le_i32(bytes: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

fn le_u24(bytes: &[u8], at: usize) -> u32 {
    bytes[at] as u32 | ((bytes[at + 1] as u32) << 8) | ((bytes[at + 2] as u32) << 16)
}
