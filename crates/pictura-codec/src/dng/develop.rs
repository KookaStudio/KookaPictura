//! The raw open's develop (#400): decode a raw file once, then develop its
//! camera RGB to cropped, oriented 16-bit sRGB at any white balance.
//!
//! The app's working RGB is sRGB, so the develop ends there; Workflow Options
//! (another space or depth) are #633.

use pictura_color::{Intent, Profile};
use rayon::prelude::*;

use crate::error::PsdError;
use crate::probe::ImageBudget;

use super::linear::is_linear_dng;
use super::tiff::Tiff;
use super::{
    as_shot_transform, demosaic, read_linear_dng, read_raw, to_pro_photo, white_balance_transform,
    CameraRgb, DemosaicMethod, DngColor, DngCrop, WhiteBalance,
};

const XMP: u16 = 700;
/// Rows per lcms2 band, so the ProPhoto-to-sRGB conversion runs in parallel.
const CONVERT_BAND_ROWS: usize = 128;

/// What the develop needs besides the pixels and the white balance.
#[derive(Debug, Clone, PartialEq)]
pub struct RawLook {
    pub color: DngColor,
    /// Relative to the camera RGB the look is applied to.
    pub crop: DngCrop,
    /// TIFF `Orientation`, 1–8.
    pub orientation: u16,
}

impl RawLook {
    /// The look for a copy of the image scaled by `sx` × `sy` (the preview).
    pub fn scaled(&self, sx: f64, sy: f64) -> RawLook {
        RawLook {
            crop: DngCrop {
                x: self.crop.x * sx,
                y: self.crop.y * sy,
                width: self.crop.width * sx,
                height: self.crop.height * sy,
            },
            ..self.clone()
        }
    }
}

/// A decoded raw file: demosaiced camera RGB and what the develop needs.
#[derive(Debug, Clone)]
pub struct RawSource {
    pub camera: CameraRgb,
    pub look: RawLook,
    pub make: Option<String>,
    pub model: Option<String>,
    pub as_shot: WhiteBalance,
}

/// Interleaved 16-bit sRGB, `width × height × 3`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopedRgb {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u16>,
}

/// Reads any supported raw container, maps its colour before the (costly)
/// demosaic so an unsupported camera fails early, then demosaics with AHD.
/// A linear DNG is already camera RGB and skips the demosaic.
pub fn decode_raw_source(bytes: &[u8], budget: &ImageBudget) -> Result<RawSource, PsdError> {
    if is_linear_dng(bytes) {
        let linear = read_linear_dng(bytes, budget)?;
        let as_shot = as_shot_transform(&linear.color)
            .map_err(|error| colour_refusal(&linear.make, &linear.model, error))?
            .white_balance;
        return Ok(RawSource {
            camera: linear.camera,
            look: RawLook {
                color: linear.color,
                crop: linear.default_crop,
                orientation: linear.orientation,
            },
            make: linear.make,
            model: linear.model,
            as_shot,
        });
    }
    let image = read_raw(bytes, budget)?;
    let as_shot = as_shot_transform(&image.color)
        .map_err(|error| colour_refusal(&image.make, &image.model, error))?
        .white_balance;
    let camera = demosaic(&image, DemosaicMethod::Ahd, budget)?;
    Ok(RawSource {
        camera,
        look: RawLook {
            color: image.color,
            crop: image.default_crop,
            orientation: image.orientation,
        },
        make: image.make,
        model: image.model,
        as_shot,
    })
}

fn colour_refusal(make: &Option<String>, model: &Option<String>, error: PsdError) -> PsdError {
    let reason = match error {
        PsdError::Unsupported(text) | PsdError::Invalid(text) => text,
        other => other.to_string(),
    };
    let make = make.as_deref().map(str::trim).unwrap_or("");
    let model = model.as_deref().map(str::trim).unwrap_or("");
    let camera = if model.starts_with(make) {
        model.to_string()
    } else {
        format!("{make} {model}").trim().to_string()
    };
    PsdError::Unsupported(if camera.is_empty() {
        format!("camera colour is not supported yet ({reason})")
    } else {
        format!("camera colour for {camera} is not supported yet ({reason})")
    })
}

/// Develops `camera` at `white_balance`: white balance and matrix to linear
/// ProPhoto, `BaselineExposure`, default crop, orientation, then 16-bit sRGB.
pub fn develop_raw(
    camera: CameraRgb,
    look: &RawLook,
    white_balance: WhiteBalance,
) -> Result<DevelopedRgb, PsdError> {
    let transform = white_balance_transform(&look.color, white_balance)?;
    let mut linear = to_pro_photo(camera, &transform);
    apply_baseline_exposure(&mut linear.rgb, look.color.baseline_exposure);
    let cropped = crop(linear.width, linear.height, &linear.rgb, &look.crop);
    let oriented = orient(cropped, look.orientation);
    to_srgb16(oriented)
}

fn apply_baseline_exposure(rgb: &mut [u16], stops: f64) {
    if stops == 0.0 || !stops.is_finite() {
        return;
    }
    let gain = 2f64.powf(stops) as f32;
    rgb.par_iter_mut()
        .for_each(|v| *v = (*v as f32 * gain).round().min(65535.0) as u16);
}

fn crop(width: usize, height: usize, rgb: &[u16], crop: &DngCrop) -> DevelopedRgb {
    let whole = |v: f64| {
        if v.is_finite() {
            v.round().max(0.0) as usize
        } else {
            0
        }
    };
    let x0 = whole(crop.x).min(width.saturating_sub(1));
    let y0 = whole(crop.y).min(height.saturating_sub(1));
    let w = whole(crop.width).clamp(1, width - x0);
    let h = whole(crop.height).clamp(1, height - y0);
    if (x0, y0, w, h) == (0, 0, width, height) {
        return DevelopedRgb {
            width,
            height,
            rgb: rgb.to_vec(),
        };
    }
    let mut out = Vec::with_capacity(w * h * 3);
    for y in y0..y0 + h {
        out.extend_from_slice(&rgb[(y * width + x0) * 3..(y * width + x0 + w) * 3]);
    }
    DevelopedRgb {
        width: w,
        height: h,
        rgb: out,
    }
}

/// The stored pixel shown at display `(x, y)` under TIFF `orientation`, for a
/// stored image `w × h`.
fn stored_at(orientation: u16, x: usize, y: usize, w: usize, h: usize) -> (usize, usize) {
    match orientation {
        2 => (w - 1 - x, y),
        3 => (w - 1 - x, h - 1 - y),
        4 => (x, h - 1 - y),
        5 => (y, x),
        6 => (y, h - 1 - x),
        7 => (w - 1 - y, h - 1 - x),
        8 => (w - 1 - y, x),
        _ => (x, y),
    }
}

fn orient(image: DevelopedRgb, orientation: u16) -> DevelopedRgb {
    if !(2..=8).contains(&orientation) {
        return image;
    }
    let (w, h) = (image.width, image.height);
    let (ow, oh) = if orientation >= 5 { (h, w) } else { (w, h) };
    let mut out = vec![0u16; ow * oh * 3];
    out.par_chunks_mut(ow * 3).enumerate().for_each(|(y, row)| {
        for (x, px) in row.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            let (sx, sy) = stored_at(orientation, x, y, w, h);
            px.copy_from_slice(&image.rgb[(sy * w + sx) * 3..][..3]);
        }
    });
    DevelopedRgb {
        width: ow,
        height: oh,
        rgb: out,
    }
}

fn to_srgb16(image: DevelopedRgb) -> Result<DevelopedRgb, PsdError> {
    let (width, height) = (image.width, image.height);
    let band = width * 3 * CONVERT_BAND_ROWS;
    let rgb: Vec<Vec<u16>> = image
        .rgb
        .par_chunks(band.max(1))
        .map(|chunk| {
            let rows = chunk.len() / (width * 3);
            let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_ne_bytes()).collect();
            // lcms2 profiles are not `Sync`; each band builds its own.
            let out = pictura_color::convert(
                &Profile::linear_pro_photo(),
                &Profile::srgb(),
                &bytes,
                width as u32,
                rows as u32,
                3,
                16,
                Intent::RelativeColorimetric,
                true,
            )
            .map_err(|error| PsdError::Unsupported(format!("raw colour conversion: {error}")))?;
            Ok(out
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_ne_bytes(*b))
                .collect())
        })
        .collect::<Result<_, PsdError>>()?;
    Ok(DevelopedRgb {
        width,
        height,
        rgb: rgb.concat(),
    })
}

/// A box-averaged copy whose long side is at most `max_dim`; an image already
/// within it comes back unchanged.
pub fn downscale_camera(image: &CameraRgb, max_dim: usize) -> CameraRgb {
    let (w, h) = (image.width, image.height);
    let long = w.max(h);
    if long <= max_dim || max_dim == 0 {
        return image.clone();
    }
    let ow = (w * max_dim / long).max(1);
    let oh = (h * max_dim / long).max(1);
    let mut rgb = vec![0u16; ow * oh * 3];
    rgb.par_chunks_mut(ow * 3)
        .enumerate()
        .for_each(|(oy, row)| {
            let (y0, y1) = (oy * h / oh, ((oy + 1) * h / oh).max(oy * h / oh + 1));
            for (ox, px) in row.as_chunks_mut::<3>().0.iter_mut().enumerate() {
                let (x0, x1) = (ox * w / ow, ((ox + 1) * w / ow).max(ox * w / ow + 1));
                let mut sum = [0u64; 3];
                for y in y0..y1 {
                    for x in x0..x1 {
                        let s = &image.rgb[(y * w + x) * 3..][..3];
                        sum.iter_mut().zip(s).for_each(|(a, &v)| *a += v as u64);
                    }
                }
                let n = ((y1 - y0) * (x1 - x0)) as u64;
                px.iter_mut()
                    .zip(sum)
                    .for_each(|(p, s)| *p = ((s + n / 2) / n) as u16);
            }
        });
    CameraRgb {
        width: ow,
        height: oh,
        rgb,
    }
}

/// The XMP packet in a TIFF-based raw file's IFD0 (tag 700).
pub fn raw_xmp(bytes: &[u8]) -> Option<Vec<u8>> {
    let (tiff, first) = Tiff::parse(bytes).ok()?;
    let ifd0 = tiff.ifd(first).ok()?;
    let packet = tiff.raw(ifd0.get(XMP)?);
    (!packet.is_empty()).then(|| packet.to_vec())
}
