//! Deterministic Save As encoders for the formats whose options or containers
//! Qt's image writers cannot express (`codec/raster-save-encoders`).

mod eps;
#[cfg(test)]
mod tests;
mod tiff;

use std::fs;
use std::io::Write;
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use pictura_core::{ColorMode, Document, Samples};

use crate::error::PsdError;

/// A Save As format the engine writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveKind {
    Jpeg,
    Png,
    Bmp,
    Tiff,
    Gif,
    Eps,
    Dcs,
    Raw,
    Tga,
    Cineon,
}

impl SaveKind {
    pub fn from_code(code: i32) -> Option<Self> {
        Some(match code {
            1 => Self::Jpeg,
            2 => Self::Png,
            3 => Self::Bmp,
            4 => Self::Tiff,
            5 => Self::Gif,
            6 => Self::Eps,
            7 => Self::Dcs,
            8 => Self::Raw,
            9 => Self::Tga,
            10 => Self::Cineon,
            _ => return None,
        })
    }

    pub fn code(self) -> i32 {
        match self {
            Self::Jpeg => 1,
            Self::Png => 2,
            Self::Bmp => 3,
            Self::Tiff => 4,
            Self::Gif => 5,
            Self::Eps => 6,
            Self::Dcs => 7,
            Self::Raw => 8,
            Self::Tga => 9,
            Self::Cineon => 10,
        }
    }

    pub fn from_suffix(suffix: &str) -> Option<Self> {
        Some(match suffix.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" | "jpe" => Self::Jpeg,
            "png" => Self::Png,
            "bmp" => Self::Bmp,
            "tif" | "tiff" => Self::Tiff,
            "gif" => Self::Gif,
            "eps" => Self::Eps,
            "tga" | "vda" | "icb" | "vst" => Self::Tga,
            "raw" => Self::Raw,
            "cin" => Self::Cineon,
            _ => return None,
        })
    }

    pub fn is_engine_writer(self) -> bool {
        !matches!(self, Self::Jpeg)
    }
}

/// The typed Save As options shared by the C++ dialogs and the writers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveOptions {
    pub png_compression: u8,
    pub png_interlace: bool,
    pub bmp_os2: bool,
    pub bmp_depth: u8,
    pub bmp_rle: bool,
    pub bmp_flip: bool,
    pub bmp_advanced: u8,
    pub tiff_compression: u8,
    pub tiff_jpeg_quality: u8,
    pub tiff_pixel_order: bool,
    pub tiff_byte_order_mac: bool,
    pub tiff_layer_compression: u8,
    pub tiff_pyramid: bool,
    pub tiff_transparency: bool,
    pub gif_interlaced: bool,
    pub tga_depth: u8,
    pub tga_rle: bool,
    pub raw_header: u32,
    pub raw_interleaved: bool,
    pub cineon_bits: u8,
    pub eps_preview: u8,
    pub eps_encoding: u8,
    pub eps_jpeg_quality: u8,
    pub eps_halftone: bool,
    pub eps_transfer: bool,
    pub eps_transparent_whites: bool,
    pub eps_ps_color_management: bool,
    pub eps_vector_data: bool,
    pub eps_interpolation: bool,
    pub dcs_version: u8,
    pub dcs_preview: u8,
    /// The working RGB profile to embed in a TIFF; `None` writes no profile.
    pub icc: Option<Vec<u8>>,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self {
            png_compression: 0,
            png_interlace: false,
            bmp_os2: false,
            bmp_depth: 24,
            bmp_rle: false,
            bmp_flip: false,
            bmp_advanced: 0,
            tiff_compression: 2,
            tiff_jpeg_quality: 10,
            tiff_pixel_order: false,
            tiff_byte_order_mac: false,
            tiff_layer_compression: 0,
            tiff_pyramid: false,
            tiff_transparency: true,
            gif_interlaced: false,
            tga_depth: 24,
            tga_rle: false,
            raw_header: 0,
            raw_interleaved: true,
            cineon_bits: 10,
            eps_preview: 2,
            eps_encoding: 2,
            eps_jpeg_quality: 10,
            eps_halftone: false,
            eps_transfer: false,
            eps_transparent_whites: false,
            eps_ps_color_management: false,
            eps_vector_data: false,
            eps_interpolation: true,
            dcs_version: 2,
            dcs_preview: 2,
            icc: None,
        }
    }
}

impl SaveOptions {
    pub fn with_defaults(mut self) -> Self {
        if self.png_compression > 9 {
            self.png_compression = 9;
        }
        if !matches!(self.bmp_depth, 1 | 4 | 8 | 16 | 24 | 32) {
            self.bmp_depth = 24;
        }
        if self.tiff_compression > 4 {
            self.tiff_compression = 2;
        }
        if self.tga_depth != 16 && self.tga_depth != 32 {
            self.tga_depth = 24;
        }
        if self.cineon_bits == 0 {
            self.cineon_bits = 10;
        }
        if self.dcs_version != 1 {
            self.dcs_version = 2;
        }
        self
    }
}

/// A flattened image in the planes the writers consume.
#[derive(Clone)]
pub(crate) struct Raster {
    pub width: u32,
    pub height: u32,
    pub mode: ColorMode,
    pub rgba: Vec<u8>,
    pub rgb: Vec<u8>,
    pub gray: Vec<u8>,
    pub cmyk: Vec<u8>,
    pub lab: Vec<u8>,
    pub alpha: bool,
    pub palette: Option<[u8; 768]>,
    pub indices: Option<Vec<u8>>,
}

impl Raster {
    pub(crate) fn from_document(doc: &Document) -> Result<Self, PsdError> {
        let (width, height) = (doc.width, doc.height);
        if width == 0 || height == 0 {
            return Err(PsdError::Invalid("zero image size".into()));
        }
        let n = width as usize * height as usize;
        let channels = doc.composite.channels as usize;
        if doc.composite.width != width || doc.composite.height != height {
            return Err(PsdError::Invalid("composite size mismatch".into()));
        }
        let bytes = doc.composite.data.to_vec();
        if channels == 0 || bytes.len() < n * channels {
            return Err(PsdError::Invalid("short composite buffer".into()));
        }
        // `Composite` stores one plane per channel (`c * n + i`), not
        // interleaved samples; reading it as interleaved scrambles every
        // multi-pixel image.
        let mut rgba = vec![255u8; n * 4];
        match channels {
            1 => {
                for i in 0..n {
                    let v = bytes[i];
                    rgba[i * 4] = v;
                    rgba[i * 4 + 1] = v;
                    rgba[i * 4 + 2] = v;
                }
            }
            2 => {
                for i in 0..n {
                    let v = bytes[i];
                    rgba[i * 4] = v;
                    rgba[i * 4 + 1] = v;
                    rgba[i * 4 + 2] = v;
                    rgba[i * 4 + 3] = bytes[n + i];
                }
            }
            3 => {
                for i in 0..n {
                    rgba[i * 4] = bytes[i];
                    rgba[i * 4 + 1] = bytes[n + i];
                    rgba[i * 4 + 2] = bytes[2 * n + i];
                }
            }
            4 => {
                for i in 0..n {
                    rgba[i * 4] = bytes[i];
                    rgba[i * 4 + 1] = bytes[n + i];
                    rgba[i * 4 + 2] = bytes[2 * n + i];
                    rgba[i * 4 + 3] = bytes[3 * n + i];
                }
            }
            _ => return Err(PsdError::Invalid("unsupported composite channels".into())),
        }
        let mut rgb = vec![0u8; n * 3];
        let mut gray = vec![0u8; n];
        for i in 0..n {
            let (r, g, b) = (rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2]);
            rgb[i] = r;
            rgb[n + i] = g;
            rgb[2 * n + i] = b;
            gray[i] = pictura_core::rec601_luma(r, g, b);
        }
        let alpha = channels >= 4 && rgba.as_chunks::<4>().0.iter().any(|p| p[3] != 255);
        let mode = doc.source_mode.unwrap_or(doc.mode);
        // Only the mode that needs colour separation pays for it; the RGB
        // writers should not allocate CMYK and Lab planes they never read.
        let cmyk = if mode == ColorMode::Cmyk {
            crate::write_cmyk::cmyk_composite_planes(doc, 8, n)
        } else {
            Vec::new()
        };
        let lab = if mode == ColorMode::Lab {
            doc.source_planes
                .as_ref()
                .map(|s| s.samples.narrow_to_u8())
                .filter(|v| v.len() == n * 3)
                .unwrap_or_else(|| crate::color_mode::rgb_to_lab(&rgb))
        } else {
            Vec::new()
        };
        let indexed = if mode == ColorMode::Indexed {
            match (
                doc.source_palette,
                doc.source_planes.as_ref().and_then(|s| match &s.samples {
                    Samples::U8(v) if v.len() == n => Some(v.to_vec()),
                    _ => None,
                }),
            ) {
                (Some(palette), Some(indices)) => (Some(palette), Some(indices)),
                _ => (None, None),
            }
        } else {
            (None, None)
        };
        Ok(Self {
            width,
            height,
            mode,
            rgba,
            rgb,
            gray,
            cmyk,
            lab,
            alpha,
            palette: indexed.0,
            indices: indexed.1,
        })
    }

    pub(crate) fn rgb_interleaved(&self) -> Vec<u8> {
        let n = self.width as usize * self.height as usize;
        let mut out = vec![0u8; n * 3];
        for i in 0..n {
            out[i * 3] = self.rgb[i];
            out[i * 3 + 1] = self.rgb[n + i];
            out[i * 3 + 2] = self.rgb[2 * n + i];
        }
        out
    }

    pub(crate) fn cmyk_interleaved(&self) -> Vec<u8> {
        let n = self.width as usize * self.height as usize;
        let mut out = vec![0u8; n * 4];
        for i in 0..n {
            for c in 0..4 {
                out[i * 4 + c] = self.cmyk[c * n + i];
            }
        }
        out
    }
}

/// Convert planar RGB to planar CMYK for a separation file. Unlike
/// `color_mode::rgb_to_cmyk` (the exact PSD round-trip inverse), this is a
/// normal process-ink encoding: 0 means no ink, 255 full ink.
pub(super) fn rgb_to_cmyk_separation(rgb: &[u8]) -> Vec<u8> {
    let plane = rgb.len() / 3;
    let (r, rest) = rgb.split_at(plane);
    let (g, b) = rest.split_at(plane);
    let mut out = vec![0u8; plane * 4];
    for i in 0..plane {
        let k = 255 - r[i].max(g[i]).max(b[i]);
        out[i] = 255 - r[i] - k;
        out[plane + i] = 255 - g[i] - k;
        out[2 * plane + i] = 255 - b[i] - k;
        out[3 * plane + i] = k;
    }
    out
}

fn over_white(value: u8, alpha: u8) -> u8 {
    ((u32::from(value) * u32::from(alpha) + 255 * (255 - u32::from(alpha)) + 127) / 255) as u8
}

/// A JPEG encoder supplied by the app for TIFF strips and EPS/DCS payloads.
pub type JpegEncode<'a> = dyn Fn(&[u8], u32, u32, u8, u8) -> Option<Vec<u8>> + 'a;

pub(super) fn jpeg_quality_percent(quality: u8) -> u8 {
    // Match the JPEG Save As dialog's 0-12 -> libjpeg table.
    const PERCENT: [u8; 13] = [60, 64, 68, 72, 75, 78, 81, 84, 86, 89, 92, 95, 98];
    PERCENT[usize::from(quality.min(12))]
}

/// Write a document in an engine-owned Save As format.
pub fn save_document(
    kind: SaveKind,
    doc: &Document,
    options: &SaveOptions,
    path: &Path,
) -> Result<(), PsdError> {
    save_document_with_jpeg(kind, doc, options, path, None)
}

/// As [`save_document`], with the app's Qt-backed JPEG encoder for TIFF/EPS.
pub fn save_document_with_jpeg(
    kind: SaveKind,
    doc: &Document,
    options: &SaveOptions,
    path: &Path,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<(), PsdError> {
    let options = options.clone().with_defaults();
    let raster = Raster::from_document(doc)?;
    match kind {
        SaveKind::Png => write_atomic(path, &write_png(&raster, &options)?),
        SaveKind::Bmp => write_atomic(path, &write_bmp(&raster, &options)?),
        SaveKind::Gif => write_atomic(path, &write_gif(&raster, &options)?),
        SaveKind::Tga => write_atomic(path, &write_tga(&raster, &options)?),
        SaveKind::Raw => write_atomic(path, &write_raw(&raster, &options)?),
        SaveKind::Cineon => write_atomic(path, &write_cineon(&raster, &options)?),
        SaveKind::Tiff => {
            let bytes = tiff::write(doc, &raster, &options, jpeg)?;
            write_atomic(path, &bytes)
        }
        SaveKind::Eps => write_atomic(path, &eps::write(&raster, &options, doc, jpeg)?),
        SaveKind::Dcs => eps::write_dcs(path, &raster, &options, doc, jpeg),
        SaveKind::Jpeg => Err(PsdError::Unsupported("JPEG stays on Qt".into())),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PsdError> {
    let Some(dir) = path.parent() else {
        return Err(PsdError::Invalid("destination has no directory".into()));
    };
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| PsdError::Invalid("destination has no file name".into()))?;
    let tmp = dir.join(format!(
        ".{name}.{:016x}.tmp",
        u64::from_le_bytes(crate::os_random::<8>())
    ));
    let write = || -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    };
    if let Err(e) = write() {
        let _ = fs::remove_file(&tmp);
        return Err(PsdError::Invalid(format!("write failed: {e}")));
    }
    Ok(())
}

fn bmp_palette(raster: &Raster, depth: u8) -> Vec<[u8; 4]> {
    if let (Some(palette), Some(indices)) = (raster.palette, raster.indices.as_ref()) {
        if depth <= 8 && indices.len() == raster.width as usize * raster.height as usize {
            let entries = 1usize << depth;
            return (0..entries)
                .map(|i| [palette[i], palette[256 + i], palette[512 + i], 255])
                .collect();
        }
    }
    let colors = 1u16 << depth;
    let indexed = crate::web::quantize(
        &raster.rgba,
        raster.width,
        raster.height,
        crate::web::PaletteOptions {
            reduction: crate::web::ColorReduction::Adaptive,
            colors,
            dither: crate::web::Dither::None,
            amount: 0,
            transparency: false,
            matte: [255, 255, 255],
        },
    );
    indexed.palette
}

fn write_bmp(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    let depth = options.bmp_depth;
    if options.bmp_os2
        && (depth == 16 || depth == 32 || options.bmp_flip || options.bmp_advanced != 0)
    {
        return Err(PsdError::Unsupported(
            "OS/2 BMP cannot hold Flip, 16-bit Advanced Modes or 32 bits".into(),
        ));
    }
    if options.bmp_rle && (options.bmp_os2 || (depth != 4 && depth != 8)) {
        return Err(PsdError::Unsupported(
            "RLE needs a 4/8-bit Windows BMP".into(),
        ));
    }
    let (w, h) = (raster.width, raster.height);
    let palette = if depth <= 8 {
        bmp_palette(raster, depth)
    } else {
        Vec::new()
    };
    let palette_entry = if options.bmp_os2 { 3 } else { 4 };
    let header_size = if options.bmp_os2 { 12 } else { 40 };
    // The three 16-bit Advanced Modes layouts need BI_BITFIELDS masks.
    let bits_fields = !options.bmp_os2 && depth == 16 && options.bmp_advanced != 0;
    let mask_bytes = if bits_fields { 12 } else { 0 };
    let offset = 14 + header_size + mask_bytes + palette.len() * palette_entry;
    let mut out = Vec::new();
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    if options.bmp_os2 {
        out.extend_from_slice(&12u32.to_le_bytes());
        out.extend_from_slice(&(w as u16).to_le_bytes());
        out.extend_from_slice(&(h as u16).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&(depth as u16).to_le_bytes());
        for p in &palette {
            out.extend_from_slice(&p[..3]);
        }
    } else {
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(w as i32).to_le_bytes());
        let signed_height = if options.bmp_flip {
            -(h as i32)
        } else {
            h as i32
        };
        out.extend_from_slice(&signed_height.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&(depth as u16).to_le_bytes());
        let compression = if options.bmp_rle {
            if depth == 8 {
                1u32
            } else {
                2u32
            }
        } else if bits_fields {
            3u32 // BI_BITFIELDS
        } else {
            0u32
        };
        out.extend_from_slice(&compression.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&(palette.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        for p in &palette {
            out.extend_from_slice(&[p[2], p[1], p[0], 0]);
        }
        if bits_fields {
            let (r, g, b) = if options.bmp_advanced == 2 {
                (0xF800u32, 0x07E0u32, 0x001Fu32)
            } else {
                (0x7C00u32, 0x03E0u32, 0x001Fu32)
            };
            for mask in [r, g, b] {
                out.extend_from_slice(&mask.to_le_bytes());
            }
        }
    }
    let rows = bmp_rows(raster, options, depth, &palette);
    if options.bmp_rle {
        let encoded = bmp_rle(&rows, depth, raster.width as usize)?;
        out.extend_from_slice(&encoded);
    } else {
        out.extend_from_slice(&rows.concat());
    }
    let size = out.len() as u32;
    out[2..6].copy_from_slice(&size.to_le_bytes());
    Ok(out)
}

fn bmp_rows(
    raster: &Raster,
    options: &SaveOptions,
    depth: u8,
    palette: &[[u8; 4]],
) -> Vec<Vec<u8>> {
    let (w, h) = (raster.width as usize, raster.height as usize);
    let n = w * h;
    let quantized = (depth <= 8 && raster.indices.is_none()).then(|| {
        crate::web::quantize(
            &raster.rgba,
            raster.width,
            raster.height,
            crate::web::PaletteOptions {
                reduction: crate::web::ColorReduction::Adaptive,
                colors: 1u16 << depth,
                dither: crate::web::Dither::None,
                amount: 0,
                transparency: false,
                matte: [255, 255, 255],
            },
        )
    });
    let index_at = |i: usize| -> u8 {
        if let Some(indices) = raster.indices.as_ref() {
            indices[i]
        } else if let Some(indexed) = quantized.as_ref() {
            indexed.indices[i]
        } else {
            0
        }
    };
    let mut rows = Vec::with_capacity(h);
    for y in 0..h {
        let mut row = Vec::new();
        match depth {
            1 => {
                let mut byte = 0u8;
                for x in 0..w {
                    let bit = index_at(y * w + x) & 1;
                    byte |= bit << (7 - (x % 8));
                    if x % 8 == 7 {
                        row.push(byte);
                        byte = 0;
                    }
                }
                if w % 8 != 0 {
                    row.push(byte);
                }
            }
            4 => {
                for x in (0..w).step_by(2) {
                    let hi = index_at(y * w + x) & 0xF;
                    let lo = if x + 1 < w {
                        index_at(y * w + x + 1) & 0xF
                    } else {
                        0
                    };
                    row.push((hi << 4) | lo);
                }
            }
            8 => {
                for x in 0..w {
                    row.push(index_at(y * w + x));
                }
            }
            16 => {
                for x in 0..w {
                    let i = y * w + x;
                    let (r, g, b, a) = (
                        raster.rgba[i * 4],
                        raster.rgba[i * 4 + 1],
                        raster.rgba[i * 4 + 2],
                        raster.rgba[i * 4 + 3],
                    );
                    let (r, g, b) = if options.bmp_advanced == 1 {
                        (r, g, b)
                    } else {
                        (over_white(r, a), over_white(g, a), over_white(b, a))
                    };
                    let v = match options.bmp_advanced {
                        1 => {
                            ((a >> 7) as u16) << 15
                                | ((r as u16) >> 3) << 10
                                | ((g as u16) >> 3) << 5
                                | ((b as u16) >> 3)
                        }
                        2 => ((r as u16) >> 3) << 11 | ((g as u16) >> 2) << 5 | ((b as u16) >> 3),
                        _ => ((r as u16) >> 3) << 10 | ((g as u16) >> 3) << 5 | ((b as u16) >> 3),
                    };
                    row.extend_from_slice(&v.to_le_bytes());
                }
            }
            24 => {
                for x in 0..w {
                    let i = y * w + x;
                    let a = raster.rgba[i * 4 + 3];
                    row.extend_from_slice(&[
                        over_white(raster.rgba[i * 4 + 2], a),
                        over_white(raster.rgba[i * 4 + 1], a),
                        over_white(raster.rgba[i * 4], a),
                    ]);
                }
            }
            32 => {
                for x in 0..w {
                    let i = y * w + x;
                    row.extend_from_slice(&[
                        raster.rgba[i * 4 + 2],
                        raster.rgba[i * 4 + 1],
                        raster.rgba[i * 4],
                        raster.rgba[i * 4 + 3],
                    ]);
                }
            }
            _ => {}
        }
        while row.len() % 4 != 0 {
            row.push(0);
        }
        rows.push(row);
    }
    let _ = palette;
    let _ = n;
    if !options.bmp_flip {
        rows.reverse();
    }
    rows
}

fn bmp_rle(rows: &[Vec<u8>], depth: u8, width: usize) -> Result<Vec<u8>, PsdError> {
    let mut out = Vec::new();
    for row in rows {
        // Absolute-mode byte counts are data bytes, not pixels: 8-bit has one
        // pixel per byte, RLE4 two pixels per byte. Odd counts need a pad byte
        // so the next command starts on a word boundary.
        let data = if depth == 8 {
            &row[..width.min(row.len())]
        } else {
            let pairs = width.div_ceil(2);
            &row[..pairs.min(row.len())]
        };
        for chunk in data.chunks(255) {
            out.push(chunk.len() as u8);
            out.extend_from_slice(chunk);
            if chunk.len() % 2 != 0 {
                out.push(0);
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out.push(0);
    out.push(1);
    Ok(out)
}

fn write_png(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    let (w, h) = (raster.width, raster.height);
    let gray = matches!(
        raster.mode,
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone
    );
    let alpha = raster.alpha;
    let color_type = if gray {
        if alpha {
            4
        } else {
            0
        }
    } else if alpha {
        6
    } else {
        2
    };
    let channels = match color_type {
        0 => 1,
        2 => 3,
        4 => 2,
        _ => 4,
    };
    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    let mut ihdr = [0u8; 13];
    ihdr[..4].copy_from_slice(&w.to_be_bytes());
    ihdr[4..8].copy_from_slice(&h.to_be_bytes());
    ihdr[8] = 8;
    ihdr[9] = color_type;
    ihdr[12] = u8::from(options.png_interlace);
    write_chunk(&mut out, b"IHDR", &ihdr);
    let mut raw = Vec::new();
    if options.png_interlace {
        for (x0, y0, dx, dy) in adam7() {
            let pw = w.saturating_sub(x0).div_ceil(dx);
            let ph = h.saturating_sub(y0).div_ceil(dy);
            if pw == 0 || ph == 0 {
                continue;
            }
            for py in 0..ph {
                let y = y0 + py * dy;
                raw.push(0);
                for px in 0..pw {
                    let x = x0 + px * dx;
                    png_pixel(raster, x, y, gray, alpha, &mut raw);
                }
            }
        }
    } else {
        for y in 0..h {
            raw.push(0);
            for x in 0..w {
                png_pixel(raster, x, y, gray, alpha, &mut raw);
            }
        }
    }
    let _ = channels;
    let mut encoder = ZlibEncoder::new(
        Vec::new(),
        Compression::new(u32::from(options.png_compression.min(9))),
    );
    encoder
        .write_all(&raw)
        .map_err(|_| PsdError::Invalid("PNG deflate".into()))?;
    let compressed = encoder
        .finish()
        .map_err(|_| PsdError::Invalid("PNG deflate".into()))?;
    write_chunk(&mut out, b"IDAT", &compressed);
    write_chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

fn png_pixel(raster: &Raster, x: u32, y: u32, gray: bool, alpha: bool, out: &mut Vec<u8>) {
    let i = (y as usize * raster.width as usize) + x as usize;
    if gray {
        out.push(raster.gray[i]);
        if alpha {
            out.push(raster.rgba[i * 4 + 3]);
        }
    } else {
        out.extend_from_slice(&raster.rgba[i * 4..i * 4 + 3]);
        if alpha {
            out.push(raster.rgba[i * 4 + 3]);
        }
    }
}

fn adam7() -> [(u32, u32, u32, u32); 7] {
    [
        (0, 0, 8, 8),
        (4, 0, 8, 8),
        (0, 4, 4, 8),
        (2, 0, 4, 4),
        (0, 2, 2, 4),
        (1, 0, 2, 2),
        (0, 1, 1, 2),
    ]
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn write_gif(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    let image = if let (Some(palette), Some(indices)) = (raster.palette, raster.indices.as_ref()) {
        let mut entries: Vec<[u8; 4]> = (0..256)
            .map(|i| [palette[i], palette[256 + i], palette[512 + i], 255])
            .collect();
        if raster.alpha {
            // A GIF has one transparent index. Pick the index that carries the
            // most below-half-opacity pixels and mark only that entry.
            let mut counts = [0usize; 256];
            for (i, &index) in indices.iter().take(indices.len()).enumerate() {
                if raster.rgba[i * 4 + 3] < 128 {
                    counts[index as usize] += 1;
                }
            }
            if let Some(index) = (0..256).max_by_key(|&i| counts[i]) {
                if counts[index] > 0 {
                    entries[index][3] = 0;
                }
            }
        }
        crate::web::Indexed {
            width: raster.width,
            height: raster.height,
            palette: entries,
            indices: indices.clone(),
        }
    } else {
        crate::web::quantize(
            &raster.rgba,
            raster.width,
            raster.height,
            crate::web::PaletteOptions {
                reduction: crate::web::ColorReduction::Adaptive,
                colors: 256,
                dither: crate::web::Dither::None,
                amount: 0,
                transparency: raster.alpha,
                matte: [255, 255, 255],
            },
        )
    };
    Ok(crate::web::encode_gif(&image, options.gif_interlaced))
}

fn write_tga(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    let depth = options.tga_depth;
    if depth != 16 && depth != 24 && depth != 32 {
        return Err(PsdError::Unsupported("TGA depth".into()));
    }
    let (w, h) = (raster.width, raster.height);
    let mut out = Vec::with_capacity(18 + (w as usize * h as usize * 4));
    let image_type: u8 = if options.tga_rle { 10 } else { 2 };
    out.extend_from_slice(&[0, 0, image_type]);
    out.extend_from_slice(&[0, 0, 0, 0, 0]);
    out.extend_from_slice(&[0, 0]); // x origin
    out.extend_from_slice(&[0, 0]); // y origin
    out.extend_from_slice(&(w as u16).to_le_bytes());
    out.extend_from_slice(&(h as u16).to_le_bytes());
    out.push(depth);
    // Bit 5: top-left origin. Bits 0-3: attribute (alpha) bits per pixel.
    let attribute_bits = match depth {
        16 => 1,
        32 => 8,
        _ => 0,
    };
    out.push(0x20 | attribute_bits);
    let pixels: Vec<[u8; 4]> = (0..w as usize * h as usize)
        .map(|i| {
            let (r, g, b, a) = (
                raster.rgba[i * 4],
                raster.rgba[i * 4 + 1],
                raster.rgba[i * 4 + 2],
                raster.rgba[i * 4 + 3],
            );
            if depth == 16 {
                let v = ((r as u16 >> 3) << 10)
                    | ((g as u16 >> 3) << 5)
                    | (b as u16 >> 3)
                    | (((a as u16) >> 7) << 15);
                [v as u8, (v >> 8) as u8, 0, 0]
            } else if depth == 24 {
                [over_white(b, a), over_white(g, a), over_white(r, a), 255]
            } else {
                [b, g, r, a]
            }
        })
        .collect();
    if options.tga_rle {
        for chunk in pixels.chunks(128) {
            out.push((chunk.len() - 1) as u8);
            for p in chunk {
                if depth == 16 {
                    out.extend_from_slice(&p[..2]);
                } else if depth == 24 {
                    out.extend_from_slice(&p[..3]);
                } else {
                    out.extend_from_slice(&p[..4]);
                }
            }
        }
    } else {
        for p in &pixels {
            if depth == 16 {
                out.extend_from_slice(&p[..2]);
            } else if depth == 24 {
                out.extend_from_slice(&p[..3]);
            } else {
                out.extend_from_slice(&p[..4]);
            }
        }
    }
    Ok(out)
}

fn write_raw(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    let n = raster.width as usize * raster.height as usize;
    let planes: Vec<&[u8]> = match raster.mode {
        ColorMode::Grayscale | ColorMode::Bitmap | ColorMode::Duotone => vec![&raster.gray],
        ColorMode::Cmyk => vec![
            &raster.cmyk[..n],
            &raster.cmyk[n..2 * n],
            &raster.cmyk[2 * n..3 * n],
            &raster.cmyk[3 * n..],
        ],
        ColorMode::Lab => vec![
            &raster.lab[..n],
            &raster.lab[n..2 * n],
            &raster.lab[2 * n..],
        ],
        // The reader normalizes Multichannel to RGB and keeps no plate store,
        // so Raw cannot truthfully write the mode's channels.
        ColorMode::Multichannel => {
            return Err(PsdError::Unsupported(
                "Raw needs Multichannel planes".into(),
            ))
        }
        _ => vec![
            &raster.rgb[..n],
            &raster.rgb[n..2 * n],
            &raster.rgb[2 * n..],
        ],
    };
    let mut out = vec![0u8; options.raw_header as usize];
    if options.raw_interleaved {
        for i in 0..n {
            for plane in &planes {
                out.push(plane[i]);
            }
        }
    } else {
        for plane in &planes {
            out.extend_from_slice(plane);
        }
    }
    Ok(out)
}

fn write_cineon(raster: &Raster, _options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    let (w, h) = (raster.width, raster.height);
    let n = w as usize * h as usize;
    let mut out = vec![0u8; 1024];
    let put_u32 = |out: &mut [u8], at: usize, value: u32| {
        out[at..at + 4].copy_from_slice(&value.to_be_bytes());
    };
    // File information header (offsets 0..192).
    put_u32(&mut out, 0, 0x802A_5FD7);
    put_u32(&mut out, 4, 1024);
    put_u32(&mut out, 8, 1024); // generic section length
    put_u32(&mut out, 12, 0); // industry section length
    put_u32(&mut out, 16, 0); // user section length
    put_u32(&mut out, 20, 1024 + (n * 4) as u32);
    out[24..32].copy_from_slice(b"V4.5\0\0\0\0");
    // Image information header: orientation, channel count, channel blocks.
    out[192] = 0; // left-to-right, top-to-bottom
    out[193] = 3;
    for (channel, designator) in [1u8, 2, 3].into_iter().enumerate() {
        let at = 196 + channel * 28;
        out[at] = 0; // universal metric
        out[at + 1] = designator;
        out[at + 2] = 10;
        put_u32(&mut out, at + 4, w);
        put_u32(&mut out, at + 8, h);
        out[at + 20..at + 24].copy_from_slice(&1023.0f32.to_be_bytes());
    }
    // Image data format: pixel interleave, longword packing, unsigned.
    out[680] = 0;
    out[681] = 5;
    out[682] = 0;
    out[683] = 0;
    let mut packed = Vec::with_capacity(n * 4);
    for i in 0..n {
        let r = (u32::from(raster.rgba[i * 4]) * 1023) / 255;
        let g = (u32::from(raster.rgba[i * 4 + 1]) * 1023) / 255;
        let b = (u32::from(raster.rgba[i * 4 + 2]) * 1023) / 255;
        let word = (r << 22) | (g << 12) | (b << 2);
        packed.extend_from_slice(&word.to_be_bytes());
    }
    out.extend_from_slice(&packed);
    Ok(out)
}
