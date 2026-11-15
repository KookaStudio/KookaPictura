//! Native mode and depth rendering through Ghostscript's PSD devices.
//!
//! A PSD device writes the page in the target colour model and depth but
//! composites it onto white paper and has no alpha channel. A second
//! `pngalpha` run supplies the page coverage; the colour samples are then
//! un-composited through it. The codec returns the cropped native planes, the
//! RGB8 display derived from them, and the coverage; the app builds the
//! document because layer creation lives in `pictura-render`.

use std::fs::File;
use std::path::Path;

use pictura_core::{BitDepth, ColorMode, Samples};

use crate::budget::{read_bounded, rgba_within_budget};
use crate::ghostscript::{
    png_args, psd_args, read_png, Ghostscript, Raster, RasterError, ScratchDir, MAX_RASTER_FILE,
};
use crate::pdf::import::PdfRect;
use crate::{read_psd_with_options, OpenOptions, Policy};

/// One native render: the cropped colour samples at `depth`, the RGB8 display
/// derived from them, and the 8-bit coverage.
///
/// Only the process colour planes are carried. Extra/spot planes a
/// `psdcmyk` PSD may hold are dropped; the working RGB display already shows
/// their alternate tint, and spot channels are out of scope for native import.
#[derive(Debug, Clone)]
pub struct NativeRender {
    pub width: u32,
    pub height: u32,
    /// The native colour model: CMYK renders CMYK, every other target renders
    /// RGB. Grayscale/Lab conversion happens after the document is built.
    pub mode: ColorMode,
    pub depth: BitDepth,
    /// Planar colour samples (`mode.color_channels()` planes) at `depth`.
    pub samples: Samples,
    /// Planar RGB8 display planes derived from `samples`.
    pub display: Vec<u8>,
    /// One coverage byte per pixel.
    pub alpha: Vec<u8>,
    pub warnings: Vec<String>,
}

/// Render the view-space `area` of page `number` natively.
#[allow(clippy::too_many_arguments)]
pub fn render_native_area(
    gs: &Ghostscript,
    path: &Path,
    password: Option<&str>,
    number: u32,
    area: PdfRect,
    view_height: f64,
    (width, height): (u32, u32),
    anti_alias: bool,
    mode: ColorMode,
    depth: BitDepth,
) -> Result<NativeRender, RasterError> {
    if width == 0 || height == 0 || area.width() <= 0.0 || area.height() <= 0.0 {
        return Err(RasterError::Failed("empty page area".into()));
    }
    if !rgba_within_budget(width, height) {
        return Err(RasterError::Failed(
            "page size over the image budget".into(),
        ));
    }
    let rx = 72.0 * f64::from(width) / area.width();
    let ry = 72.0 * f64::from(height) / area.height();
    let (native_mode, native_depth, device) = native_target(mode, depth);
    let scratch = ScratchDir::new()?;
    let psd = scratch.path("page.psd");
    let png = scratch.path("page.png");

    let mut args = vec![
        format!("-dFirstPage={number}"),
        format!("-dLastPage={number}"),
    ];
    args.extend(psd_args(device, rx, ry, anti_alias, &psd, path));
    let colour = gs.run(path, password, &args)?;
    let doc = read_native_psd(&psd)?;
    let (coverage, alpha_warnings) =
        render_coverage(gs, path, password, &png, rx, ry, anti_alias, Some(number))?;
    let left = (area.x0 * rx / 72.0).round() as i64;
    let top = ((view_height - area.y1) * ry / 72.0).round() as i64;
    finish_native(
        doc,
        coverage,
        native_mode,
        native_depth,
        left,
        top,
        width,
        height,
        colour.warnings.into_iter().chain(alpha_warnings).collect(),
    )
}

/// Render an EPS's bounding box natively.
pub fn render_native_eps(
    gs: &Ghostscript,
    path: &Path,
    bounds: PdfRect,
    (width, height): (u32, u32),
    anti_alias: bool,
    mode: ColorMode,
    depth: BitDepth,
) -> Result<NativeRender, RasterError> {
    if width == 0 || height == 0 || bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Err(RasterError::Failed("empty image size".into()));
    }
    if !rgba_within_budget(width, height) {
        return Err(RasterError::Failed(
            "image size over the image budget".into(),
        ));
    }
    let rx = 72.0 * f64::from(width) / bounds.width();
    let ry = 72.0 * f64::from(height) / bounds.height();
    let (native_mode, native_depth, device) = native_target(mode, depth);
    let scratch = ScratchDir::new()?;
    let psd = scratch.path("eps.psd");
    let png = scratch.path("eps.png");

    let mut args = vec!["-dEPSCrop".to_string()];
    args.extend(psd_args(device, rx, ry, anti_alias, &psd, path));
    let colour = gs.run(path, None, &args)?;
    let doc = read_native_psd(&psd)?;
    let mut args = vec!["-dEPSCrop".to_string()];
    args.extend(png_args(rx, ry, anti_alias, &png, path));
    let alpha_run = gs.run(path, None, &args)?;
    let coverage = read_png(&png)?;
    let top = i64::from(doc.height.saturating_sub(height));
    finish_native(
        doc,
        coverage,
        native_mode,
        native_depth,
        0,
        top,
        width,
        height,
        colour
            .warnings
            .into_iter()
            .chain(alpha_run.warnings)
            .collect(),
    )
}

/// Render a one-page wrapper (an embedded image) natively at `res` dpi.
pub(crate) fn render_native_wrapper(
    gs: &Ghostscript,
    input: &Path,
    res: f64,
    anti_alias: bool,
    mode: ColorMode,
    depth: BitDepth,
) -> Result<NativeRender, RasterError> {
    let (native_mode, native_depth, device) = native_target(mode, depth);
    let scratch = ScratchDir::new()?;
    let psd = scratch.path("image.psd");
    let png = scratch.path("image.png");
    let colour = gs.run(
        input,
        None,
        &psd_args(device, res, res, anti_alias, &psd, input),
    )?;
    let doc = read_native_psd(&psd)?;
    let alpha_run = gs.run(input, None, &png_args(res, res, anti_alias, &png, input))?;
    let coverage = read_png(&png)?;
    let (width, height) = (doc.width, doc.height);
    finish_native(
        doc,
        coverage,
        native_mode,
        native_depth,
        0,
        0,
        width,
        height,
        colour
            .warnings
            .into_iter()
            .chain(alpha_run.warnings)
            .collect(),
    )
}

fn native_target(mode: ColorMode, depth: BitDepth) -> (ColorMode, BitDepth, &'static str) {
    let mode = if mode == ColorMode::Cmyk {
        ColorMode::Cmyk
    } else {
        ColorMode::Rgb
    };
    let depth = if depth == BitDepth::Sixteen {
        BitDepth::Sixteen
    } else {
        BitDepth::Eight
    };
    let device = match (mode, depth) {
        (ColorMode::Cmyk, BitDepth::Sixteen) => "psdcmyk16",
        (ColorMode::Cmyk, _) => "psdcmyk",
        (_, BitDepth::Sixteen) => "psdrgb16",
        _ => "psdrgb",
    };
    (mode, depth, device)
}

fn read_native_psd(path: &Path) -> Result<pictura_core::Document, RasterError> {
    let file = File::open(path).map_err(|e| RasterError::Failed(e.to_string()))?;
    let bytes = read_bounded(file, MAX_RASTER_FILE)
        .ok_or_else(|| RasterError::Failed("Ghostscript PSD output too large".into()))?;
    read_psd_with_options(
        &bytes,
        &OpenOptions {
            policy: Policy::Off,
            ..OpenOptions::default()
        },
    )
    .map_err(|e| RasterError::Failed(e.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn render_coverage(
    gs: &Ghostscript,
    path: &Path,
    password: Option<&str>,
    out: &Path,
    rx: f64,
    ry: f64,
    anti_alias: bool,
    page: Option<u32>,
) -> Result<(Raster, Vec<String>), RasterError> {
    let mut args = Vec::new();
    if let Some(number) = page {
        args.push(format!("-dFirstPage={number}"));
        args.push(format!("-dLastPage={number}"));
    }
    args.extend(png_args(rx, ry, anti_alias, out, path));
    let run = gs.run(path, password, &args)?;
    Ok((read_png(out)?, run.warnings))
}

#[allow(clippy::too_many_arguments)]
fn finish_native(
    doc: pictura_core::Document,
    coverage: Raster,
    mode: ColorMode,
    depth: BitDepth,
    left: i64,
    top: i64,
    width: u32,
    height: u32,
    mut warnings: Vec<String>,
) -> Result<NativeRender, RasterError> {
    if doc.width != coverage.width || doc.height != coverage.height {
        return Err(RasterError::Failed(
            "the colour and coverage renders differ in size".into(),
        ));
    }
    let full = doc
        .source_planes
        .as_ref()
        .map(|store| store.samples.clone())
        .ok_or_else(|| RasterError::Failed("the native render has no retained planes".into()))?;
    let (full_w, full_h) = (doc.width, doc.height);
    let plane = full_w as usize * full_h as usize;
    let colours = mode.color_channels() as usize;
    if colours == 0 || full.len() < colours * plane {
        return Err(RasterError::Failed("the native render is short".into()));
    }
    let straight = uncomposite(&full, &coverage, colours, plane)
        .ok_or_else(|| RasterError::Failed("unreadable native samples".into()))?;
    let samples = crop_samples(&straight, full_w, full_h, colours, left, top, width, height)
        .ok_or_else(|| RasterError::Failed("native crop failed".into()))?;
    let alpha = crop_alpha(&coverage, full_w, full_h, left, top, width, height);
    let display = derive_display(&samples, mode, width, height)
        .ok_or_else(|| RasterError::Failed("native display failed".into()))?;
    warnings.dedup();
    Ok(NativeRender {
        width,
        height,
        mode,
        depth,
        samples,
        display,
        alpha,
        warnings,
    })
}

/// Un-composite the white-paper samples through the coverage. The store keeps
/// 255 (or 65535) as paper, so a partly covered pixel is recovered as
/// `paper - (paper - observed) * 255 / coverage`, where `coverage` is the
/// 8-bit alpha. The subtracted term is clamped to `paper`: the coverage is
/// 8-bit while the colour is not, so a coverage step can put the term past the
/// paper value at a partially covered pixel.
fn uncomposite(
    samples: &Samples,
    coverage: &Raster,
    colours: usize,
    plane: usize,
) -> Option<Samples> {
    if coverage.rgba.len() < plane * 4 {
        return None;
    }
    let alpha = |i: usize| u32::from(coverage.rgba[i * 4 + 3]);
    match samples {
        Samples::U8(v) if v.len() >= plane * colours => {
            let mut out = v[..plane * colours].to_vec();
            for i in 0..plane {
                let a = alpha(i);
                if a == 0 {
                    for c in 0..colours {
                        out[c * plane + i] = 255;
                    }
                } else if a < 255 {
                    for c in 0..colours {
                        let observed = u32::from(out[c * plane + i]);
                        let term = ((255 - observed) * 255 + a / 2) / a;
                        out[c * plane + i] = 255u32.saturating_sub(term.min(255)) as u8;
                    }
                }
            }
            Some(Samples::U8(out.into()))
        }
        Samples::U16(v) if v.len() >= plane * colours => {
            let mut out = v[..plane * colours].to_vec();
            for i in 0..plane {
                let a = u64::from(alpha(i));
                if a == 0 {
                    for c in 0..colours {
                        out[c * plane + i] = 65535;
                    }
                } else if a < 255 {
                    for c in 0..colours {
                        let observed = u64::from(out[c * plane + i]);
                        let term = ((65535 - observed) * 255 + a / 2) / a;
                        out[c * plane + i] = 65535u64.saturating_sub(term.min(65535)) as u16;
                    }
                }
            }
            Some(Samples::U16(out.into()))
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn crop_samples(
    samples: &Samples,
    sw: u32,
    sh: u32,
    colours: usize,
    left: i64,
    top: i64,
    width: u32,
    height: u32,
) -> Option<Samples> {
    let (sw, sh) = (sw as i64, sh as i64);
    let (tw, th) = (width as usize, height as usize);
    let plane = tw * th;
    match samples {
        Samples::U8(v) if v.len() >= sw as usize * sh as usize * colours => {
            let mut out = vec![255u8; plane * colours];
            for y in 0..th {
                let sy = top + y as i64;
                if sy < 0 || sy >= sh {
                    continue;
                }
                for x in 0..tw {
                    let sx = left + x as i64;
                    if sx < 0 || sx >= sw {
                        continue;
                    }
                    let from = sy as usize * sw as usize + sx as usize;
                    for c in 0..colours {
                        out[c * plane + y * tw + x] = v[c * sw as usize * sh as usize + from];
                    }
                }
            }
            Some(Samples::U8(out.into()))
        }
        Samples::U16(v) if v.len() >= sw as usize * sh as usize * colours => {
            let mut out = vec![65535u16; plane * colours];
            for y in 0..th {
                let sy = top + y as i64;
                if sy < 0 || sy >= sh {
                    continue;
                }
                for x in 0..tw {
                    let sx = left + x as i64;
                    if sx < 0 || sx >= sw {
                        continue;
                    }
                    let from = sy as usize * sw as usize + sx as usize;
                    for c in 0..colours {
                        out[c * plane + y * tw + x] = v[c * sw as usize * sh as usize + from];
                    }
                }
            }
            Some(Samples::U16(out.into()))
        }
        _ => None,
    }
}

fn crop_alpha(
    coverage: &Raster,
    sw: u32,
    sh: u32,
    left: i64,
    top: i64,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let (sw, sh) = (sw as i64, sh as i64);
    let (tw, th) = (width as usize, height as usize);
    let mut out = vec![0u8; tw * th];
    for y in 0..th {
        let sy = top + y as i64;
        if sy < 0 || sy >= sh {
            continue;
        }
        for x in 0..tw {
            let sx = left + x as i64;
            if sx < 0 || sx >= sw {
                continue;
            }
            out[y * tw + x] = coverage.rgba[(sy as usize * sw as usize + sx as usize) * 4 + 3];
        }
    }
    out
}

fn derive_display(samples: &Samples, mode: ColorMode, width: u32, height: u32) -> Option<Vec<u8>> {
    let plane = width as usize * height as usize;
    let narrowed = samples.narrow_to_u8();
    match mode {
        ColorMode::Cmyk if narrowed.len() >= plane * 4 => {
            Some(crate::cmyk_to_rgb(&narrowed[..plane * 4]))
        }
        _ if narrowed.len() >= plane * 3 => Some(narrowed[..plane * 3].to_vec()),
        _ => None,
    }
}
