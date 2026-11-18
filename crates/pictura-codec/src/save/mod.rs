//! Deterministic Save As encoders for the formats whose options or containers
//! Qt's image writers cannot express (`codec/raster-save-encoders`).

mod eps;
mod rows;
mod stream;
#[cfg(test)]
mod tests;
mod tiff;
mod tiff_codec;

use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use pictura_core::{ColorMode, Document, Samples};

use crate::error::PsdError;
use crate::write_stream::{io_error, Sink};
use rows::RasterRows;

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
        let mode = doc.source.mode().unwrap_or(doc.mode);
        // Only the mode that needs colour separation pays for it; the RGB
        // writers should not allocate CMYK and Lab planes they never read.
        let cmyk = if mode == ColorMode::Cmyk {
            crate::write_cmyk::cmyk_composite_planes(doc, 8, n)
        } else {
            Vec::new()
        };
        let lab = if mode == ColorMode::Lab {
            doc.source
                .planes()
                .as_ref()
                .map(|s| s.samples.narrow_to_u8())
                .filter(|v| v.len() == n * 3)
                .unwrap_or_else(|| crate::color_mode::rgb_to_lab(&rgb))
        } else {
            Vec::new()
        };
        let indexed = if mode == ColorMode::Indexed {
            match (
                doc.source.palette(),
                doc.source.planes().as_ref().and_then(|s| match &s.samples {
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

/// A final integrity check run after encoding and before the writer replaces
/// any destination. Returning false aborts the save with the target untouched.
pub type BeforeWrite<'a> = dyn Fn() -> bool + 'a;

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
    save_document_with_jpeg_guarded(kind, doc, options, path, jpeg, None)
}

/// As [`save_document_with_jpeg`], with a final integrity check after encoding
/// and before any destination is replaced. A false check leaves the target set
/// untouched.
pub fn save_document_with_jpeg_guarded(
    kind: SaveKind,
    doc: &Document,
    options: &SaveOptions,
    path: &Path,
    jpeg: Option<&JpegEncode<'_>>,
    before_write: Option<&BeforeWrite<'_>>,
) -> Result<(), PsdError> {
    let options = options.clone().with_defaults();
    let check = || before_write.is_none_or(|guard| guard());
    if streams(kind, doc, &options) {
        let rows = RasterRows::from_document(doc)?;
        return write_atomic_with(path, &check, |sink| match kind {
            SaveKind::Png => stream::write_png(sink, &rows, &options),
            SaveKind::Bmp => stream::write_bmp(sink, &rows, &options),
            SaveKind::Tga => stream::write_tga(sink, &rows, &options),
            SaveKind::Raw => stream::write_raw(sink, &rows, &options),
            SaveKind::Cineon => stream::write_cineon(sink, &rows),
            _ => tiff::write_to(sink, doc, &rows, &options, jpeg),
        });
    }
    // ponytail: GIF, EPS and DCS, BMP at 8 bits or fewer (one palette for every
    // pixel), and CMYK or Lab Raw and TIFF build the whole image first.
    let raster = Raster::from_document(doc)?;
    let bytes = match kind {
        SaveKind::Bmp => write_bmp(&raster, &options)?,
        SaveKind::Gif => write_gif(&raster, &options)?,
        SaveKind::Raw => write_raw(&raster, &options)?,
        SaveKind::Tiff => tiff::write(doc, &raster, &options, jpeg)?,
        SaveKind::Eps => eps::write(&raster, &options, doc, jpeg)?,
        SaveKind::Dcs => {
            return eps::write_dcs_guarded(path, &raster, &options, doc, jpeg, before_write)
        }
        SaveKind::Jpeg => return Err(PsdError::Unsupported("JPEG stays on Qt".into())),
        SaveKind::Png | SaveKind::Tga | SaveKind::Cineon => {
            unreachable!("row formats stream")
        }
    };
    if !check() {
        return Err(PsdError::Invalid("save was interrupted".into()));
    }
    write_atomic(path, &bytes)
}

/// Whether `kind` streams `doc` from its composite rows (#1320), rather than
/// building the whole image.
pub fn streams(kind: SaveKind, doc: &Document, options: &SaveOptions) -> bool {
    let separated = matches!(
        doc.source.mode().unwrap_or(doc.mode),
        ColorMode::Cmyk | ColorMode::Lab
    );
    match kind {
        SaveKind::Png | SaveKind::Tga | SaveKind::Cineon => true,
        SaveKind::Bmp => options.bmp_depth > 8,
        SaveKind::Raw | SaveKind::Tiff => !separated,
        SaveKind::Gif | SaveKind::Eps | SaveKind::Dcs | SaveKind::Jpeg => false,
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PsdError> {
    write_atomic_with(path, &|| true, |sink| sink.put(bytes))
}

/// Write `path` through an unpredictable sibling temp file: `write` streams
/// into it, `check` runs after it, then the file is synced and renamed over
/// `path`. Any failure removes the temp file and leaves `path` untouched.
fn write_atomic_with(
    path: &Path,
    check: &dyn Fn() -> bool,
    write: impl FnOnce(&mut Sink<'_>) -> Result<(), PsdError>,
) -> Result<(), PsdError> {
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
    let written = (|| {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(io_error)?;
        let mut out = BufWriter::new(file);
        write(&mut Sink::new(&mut out)?)?;
        out.flush().map_err(io_error)?;
        if !check() {
            return Err(PsdError::Invalid("save was interrupted".into()));
        }
        let file = out.into_inner().map_err(|e| io_error(e.into_error()))?;
        file.sync_all().map_err(io_error)?;
        fs::rename(&tmp, path).map_err(io_error)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
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
            mapped_transparency: false,
            matte: [255, 255, 255],
        },
    );
    indexed.palette
}

/// The BMP file and info headers, masks and palette; the file size (bytes
/// 2..6) is left zero for the writer to fill in.
pub(super) fn bmp_header(w: u32, h: u32, options: &SaveOptions, palette: &[[u8; 4]]) -> Vec<u8> {
    let depth = options.bmp_depth;
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
        for p in palette {
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
        for p in palette {
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
    out
}

/// Validate the BMP options for `options.bmp_depth`.
pub(super) fn bmp_check(options: &SaveOptions) -> Result<(), PsdError> {
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
    Ok(())
}

fn write_bmp(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    bmp_check(options)?;
    let depth = options.bmp_depth;
    let palette = if depth <= 8 {
        bmp_palette(raster, depth)
    } else {
        Vec::new()
    };
    let mut out = bmp_header(raster.width, raster.height, options, &palette);
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
                mapped_transparency: false,
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
            16 | 24 | 32 => bmp_pixels(
                &raster.rgba[y * w * 4..(y + 1) * w * 4],
                depth,
                options.bmp_advanced,
                &mut row,
            ),
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

/// One 16-, 24- or 32-bit BMP row from interleaved RGBA, before padding.
pub(super) fn bmp_pixels(rgba: &[u8], depth: u8, advanced: u8, row: &mut Vec<u8>) {
    for px in rgba.as_chunks::<4>().0 {
        let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
        match depth {
            16 => {
                let (r, g, b) = if advanced == 1 {
                    (r, g, b)
                } else {
                    (over_white(r, a), over_white(g, a), over_white(b, a))
                };
                let v = match advanced {
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
            24 => row.extend_from_slice(&[over_white(b, a), over_white(g, a), over_white(r, a)]),
            _ => row.extend_from_slice(&[b, g, r, a]),
        }
    }
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

#[cfg(test)]
fn write_png(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    crate::write_stream::to_vec(|sink| stream::write_png(sink, raster, options))
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
    !crc32_update(0xFFFF_FFFF, data)
}

/// The running PNG CRC-32 register after `data`; the checksum is its
/// complement.
pub(super) fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
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
    crc
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
                mapped_transparency: false,
                matte: [255, 255, 255],
            },
        )
    };
    Ok(crate::web::encode_gif(&image, options.gif_interlaced))
}

#[cfg(test)]
fn write_tga(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    crate::write_stream::to_vec(|sink| stream::write_tga(sink, raster, options))
}

fn write_raw(raster: &Raster, options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    crate::write_stream::to_vec(|sink| stream::write_raw(sink, raster, options))
}

#[cfg(test)]
fn write_cineon(raster: &Raster, _options: &SaveOptions) -> Result<Vec<u8>, PsdError> {
    crate::write_stream::to_vec(|sink| stream::write_cineon(sink, raster))
}

/// The 1024-byte Cineon header for a `w` × `h` image of `n` pixels.
fn cineon_header(w: u32, h: u32, n: usize) -> Vec<u8> {
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
    out
}
