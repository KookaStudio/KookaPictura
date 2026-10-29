//! The page image: the rendered composite turned into samples in the colour
//! space its mode and the Output settings call for (design D5), with the
//! alpha kept as a soft mask or flattened onto white, 16-bit samples where
//! allowed, and downsampling applied.

use pictura_color::{Intent, Profile};
use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer};

use super::options::{ColorConversion, Compatibility, Downsample, PdfOptions, ProfileInclusion};
use super::resample::{downsample, target_size};
use super::text::Fill;
use crate::color_mode::{cmyk_to_rgb, composite_retained_8, lab_to_rgb, rgb_to_cmyk, rgb_to_lab};
use crate::error::PsdError;

/// The image's colour space.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Space {
    Gray,
    Rgb,
    Cmyk,
    Lab,
    /// An indexed RGB palette (768 bytes: R, G, B per entry).
    Indexed(Vec<u8>),
    /// ICC-based with `n` components and the profile bytes.
    Icc(u8, Vec<u8>),
}

impl Space {
    pub(crate) fn components(&self) -> usize {
        match self {
            Space::Gray | Space::Indexed(_) => 1,
            Space::Rgb | Space::Lab => 3,
            Space::Cmyk => 4,
            Space::Icc(n, _) => *n as usize,
        }
    }
}

/// The prepared page image, before compression.
#[derive(Debug, Clone)]
pub(crate) struct Raster {
    pub width: u32,
    pub height: u32,
    pub space: Space,
    /// 1, 8, or 16.
    pub bpc: u8,
    /// Interleaved samples: bytes at 8 bpc, big-endian pairs at 16 bpc, packed
    /// rows (1 = white) at 1 bpc.
    pub samples: Vec<u8>,
    /// At 1 bpc, the pixels (`true` = black) for the CCITT encoder.
    pub bits: Option<Vec<bool>>,
    /// An 8-bit soft mask at the image size.
    pub alpha: Option<Vec<u8>>,
}

impl Raster {
    /// Whether JPEG can carry it (8-bit continuous tone).
    pub(crate) fn jpeg_capable(&self) -> bool {
        self.bpc == 8 && !matches!(self.space, Space::Indexed(_))
    }
}

/// The working profile of the stored colour numbers: the document profile,
/// else sRGB (Gray Gamma 2.2 for gray). ponytail: an untagged gray document is
/// assumed to be gamma 2.2.
fn working_profile(doc: &Document) -> Profile {
    doc.document_icc
        .as_deref()
        .and_then(|icc| Profile::from_icc(icc).ok())
        .filter(|p| p.channels() == Some(doc.mode.color_channels()))
        .unwrap_or_else(|| {
            if doc.mode == ColorMode::Grayscale {
                Profile::gray_gamma22()
            } else {
                Profile::srgb()
            }
        })
}

/// The ICC bytes that tag an unconverted Gray/RGB image.
fn working_icc(doc: &Document) -> Vec<u8> {
    match doc.document_icc.as_ref() {
        Some(icc)
            if Profile::from_icc(icc).is_ok_and(|p| p.channels() == Some(color_of(doc) as u8)) =>
        {
            icc.to_vec()
        }
        _ => working_profile(doc).to_icc(),
    }
}

fn color_of(doc: &Document) -> usize {
    if doc.mode == ColorMode::Grayscale {
        1
    } else {
        3
    }
}

fn planar_to_interleaved(planes: &[u8], channels: usize, plane: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(plane * channels);
    for i in 0..plane {
        for c in 0..channels {
            out.push(u16::from(planes[c * plane + i]));
        }
    }
    out
}

/// Retained source planes (`count` of them, narrowed to 8-bit) when they still
/// decode to the working RGB `current`, else `encode(current)`.
fn retained_or(
    doc: &Document,
    current: &[u8],
    count: usize,
    decode: fn(&[u8]) -> Vec<u8>,
    encode: fn(&[u8]) -> Vec<u8>,
) -> Vec<u8> {
    let depth = crate::depth::output_depth(doc).unwrap_or(8);
    let retained: Option<Vec<u8>> = (0..count)
        .map(|c| composite_retained_8(doc, depth, c))
        .collect::<Option<Vec<_>>>()
        .map(|planes| planes.concat());
    match retained {
        Some(r) if decode(&r).as_slice() == current => r,
        _ => encode(current),
    }
}

/// The 16-bit planes for an unedited 16-bit Gray/RGB source: each retained
/// plane whose narrowing is the current plane, else the current plane widened.
fn sixteen_bit_planes(doc: &Document, current: &[u8], color: usize, plane: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(color * plane);
    for c in 0..color {
        let now = &current[c * plane..(c + 1) * plane];
        match crate::write::composite_retained(doc, 16, c) {
            Some(samples) if samples.narrow_to_u8() == now => out.extend(samples.to_u16()),
            _ => out.extend(now.iter().map(|&v| u16::from(v) * 257)),
        }
    }
    out
}

/// Build the page image from `doc` (the save view) and its rendered
/// `composite` (colour planes, then an optional alpha plane).
pub(crate) fn build_raster(
    doc: &Document,
    composite: &PixelBuffer,
    options: &PdfOptions,
    ppi: f64,
) -> Result<Raster, PsdError> {
    if doc.source_mode == Some(ColorMode::Multichannel) || doc.mode == ColorMode::Multichannel {
        return Err(PsdError::Unsupported(
            "Photoshop PDF does not support Multichannel".into(),
        ));
    }
    let (w, h) = (composite.width as usize, composite.height as usize);
    let plane = w * h;
    let color = color_of(doc);
    if (composite.channels as usize) < color || composite.data.len() < color * plane || plane == 0 {
        return Err(PsdError::Invalid(
            "composite does not match the document".into(),
        ));
    }
    let alpha = (composite.channels as usize > color)
        .then(|| composite.data[color * plane..(color + 1) * plane].to_vec())
        .filter(|a| a.iter().any(|&v| v < 255));

    if crate::write_bitmap::writes_bitmap(doc) {
        let bits: Vec<bool> = composite.data[..plane].iter().map(|&v| v < 128).collect();
        let row = w.div_ceil(8);
        let mut samples = vec![0u8; row * h];
        for (i, &black) in bits.iter().enumerate() {
            if !black {
                samples[(i / w) * row + (i % w) / 8] |= 0x80 >> (i % 8);
            }
        }
        return Ok(Raster {
            width: w as u32,
            height: h as u32,
            space: Space::Gray,
            bpc: 1,
            samples,
            bits: Some(bits),
            alpha: None,
        });
    }

    let original = &composite.data[..color * plane];
    let keep_alpha = alpha.is_some() && options.keeps_transparency();
    let mut current = original.to_vec();
    if let (Some(a), false) = (alpha.as_ref(), keep_alpha) {
        for c in 0..color {
            for (v, &a) in current[c * plane..(c + 1) * plane].iter_mut().zip(a) {
                *v =
                    ((u32::from(*v) * u32::from(a) + 255 * (255 - u32::from(a)) + 127) / 255) as u8;
            }
        }
    }
    let (nw, nh) = target_size(
        w as u32,
        h as u32,
        ppi,
        options.downsample,
        options.downsample_ppi,
        options.downsample_above_ppi,
    );
    let downsampled = (nw as usize, nh as usize) != (w, h);
    let converting = options.color_conversion == ColorConversion::ToDestination;

    let sixteen = !options.convert_16_to_8
        && options.compatibility >= Compatibility::Acrobat6
        && doc.source_depth == Some(BitDepth::Sixteen)
        && doc.source_mode.is_none()
        && !converting;

    let (space, samples, max): (Space, Vec<u16>, u16) = if converting {
        let destination = options.destination;
        let interleaved: Vec<u8> = planar_to_interleaved(&current, color, plane)
            .into_iter()
            .map(|v| v as u8)
            .collect();
        let converted = pictura_color::convert_space(
            &working_profile(doc),
            &destination.profile(),
            &interleaved,
            Intent::RelativeColorimetric,
            true,
        )
        .map_err(|e| PsdError::Unsupported(e.to_string()))?;
        let n = destination.channels();
        let space = match (options.profile_inclusion, n) {
            (ProfileInclusion::IncludeDestination, _) => Space::Icc(n, destination.icc()),
            (_, 1) => Space::Gray,
            (_, 4) => Space::Cmyk,
            _ => Space::Rgb,
        };
        (space, converted.into_iter().map(u16::from).collect(), 255)
    } else if sixteen {
        let planes = sixteen_bit_planes(doc, original, color, plane);
        let mut interleaved = Vec::with_capacity(planes.len());
        for i in 0..plane {
            for c in 0..color {
                let v = u32::from(planes[c * plane + i]);
                interleaved.push(match (&alpha, keep_alpha) {
                    (Some(a), false) => {
                        let a = u32::from(a[i]);
                        ((v * a + 65535 * (255 - a) + 127) / 255) as u16
                    }
                    _ => v as u16,
                });
            }
        }
        (device_or_icc(doc, options), interleaved, 65535)
    } else if doc.source_mode == Some(ColorMode::Cmyk) && color == 3 {
        let psd = retained_or(doc, &current, 4, cmyk_to_rgb, rgb_to_cmyk);
        let inks: Vec<u8> = psd.iter().map(|v| 255 - v).collect();
        (Space::Cmyk, planar_to_interleaved(&inks, 4, plane), 255)
    } else if doc.source_mode == Some(ColorMode::Lab) && color == 3 {
        let lab = retained_or(doc, &current, 3, lab_to_rgb, rgb_to_lab);
        (Space::Lab, planar_to_interleaved(&lab, 3, plane), 255)
    } else if alpha.is_none()
        && !downsampled
        && options.compression != super::options::Compression::Jpeg
        && crate::write_indexed::writes_indexed(doc, 8, plane)
    {
        let palette = doc.source_palette.expect("writes_indexed checked it");
        let index = crate::write::composite_retained(doc, 8, 0)
            .map(|s| s.to_bytes())
            .ok_or_else(|| PsdError::Invalid("indexed plane missing".into()))?;
        // The PSD palette is planar (all R, then G, then B); PDF interleaves.
        let mut lookup = Vec::with_capacity(768);
        for i in 0..256 {
            lookup.extend([palette[i], palette[256 + i], palette[512 + i]]);
        }
        let samples = index.into_iter().map(u16::from).collect();
        (Space::Indexed(lookup), samples, 255)
    } else {
        (
            device_or_icc(doc, options),
            planar_to_interleaved(&current, color, plane),
            255,
        )
    };

    let channels = space.components();
    let samples = downsample(
        &samples,
        channels,
        w,
        h,
        nw as usize,
        nh as usize,
        options.downsample,
        max,
    );
    let alpha = alpha.filter(|_| keep_alpha).map(|a| {
        let wide: Vec<u16> = a.into_iter().map(u16::from).collect();
        downsample(
            &wide,
            1,
            w,
            h,
            nw as usize,
            nh as usize,
            options.downsample,
            255,
        )
        .into_iter()
        .map(|v| v as u8)
        .collect()
    });
    let (bpc, bytes) = if max == 65535 {
        (16, samples.iter().flat_map(|v| v.to_be_bytes()).collect())
    } else {
        (8, samples.into_iter().map(|v| v as u8).collect())
    };
    Ok(Raster {
        width: nw,
        height: nh,
        space,
        bpc,
        samples: bytes,
        bits: None,
        alpha,
    })
}

/// A live-text colour (`color` in the working colour numbers, a gray document's
/// gray repeated) in `raster`'s colour space, by the same path the image's
/// samples took. An indexed image's text is filled in DeviceRGB.
pub(crate) fn text_fill(
    doc: &Document,
    options: &PdfOptions,
    raster: &Raster,
    color: [u8; 3],
) -> Result<Fill, PsdError> {
    let unit = |v: u8| f64::from(v) / 255.0;
    let working = &color[..color_of(doc)];
    if raster.bpc == 1 {
        return Ok(Fill::Gray(if color[0] < 128 { 0.0 } else { 1.0 }));
    }
    if options.color_conversion == ColorConversion::ToDestination {
        let converted = pictura_color::convert_space(
            &working_profile(doc),
            &options.destination.profile(),
            working,
            Intent::RelativeColorimetric,
            true,
        )
        .map_err(|e| PsdError::Unsupported(e.to_string()))?;
        let c: Vec<f64> = converted.into_iter().map(unit).collect();
        return Ok(match (&raster.space, c.len()) {
            (Space::Icc(..), _) => Fill::Space(c),
            (_, 1) => Fill::Gray(c[0]),
            (_, 4) => Fill::Cmyk([c[0], c[1], c[2], c[3]]),
            _ => Fill::Rgb([c[0], c[1], c[2]]),
        });
    }
    Ok(match &raster.space {
        Space::Cmyk => {
            let psd = rgb_to_cmyk(&color);
            let ink = |i: usize| unit(255 - psd[i]);
            Fill::Cmyk([ink(0), ink(1), ink(2), ink(3)])
        }
        Space::Lab => {
            let lab = rgb_to_lab(&color);
            Fill::Space(vec![
                f64::from(lab[0]) * 100.0 / 255.0,
                f64::from(lab[1]) - 128.0,
                f64::from(lab[2]) - 128.0,
            ])
        }
        Space::Gray => Fill::Gray(unit(color[0])),
        Space::Icc(..) => Fill::Space(working.iter().map(|&v| unit(v)).collect()),
        Space::Rgb | Space::Indexed(_) => Fill::Rgb(color.map(unit)),
    })
}

fn device_or_icc(doc: &Document, options: &PdfOptions) -> Space {
    let n = color_of(doc) as u8;
    match (options.icc_profile, n) {
        (true, _) => Space::Icc(n, working_icc(doc)),
        (false, 1) => Space::Gray,
        (false, _) => Space::Rgb,
    }
}

/// The page thumbnail: the working composite flattened onto white, averaged
/// down to at most 106 px on its long side, as gray or RGB 8-bit samples.
pub(crate) fn thumbnail(doc: &Document, composite: &PixelBuffer) -> (u32, u32, bool, Vec<u8>) {
    let (w, h) = (composite.width as usize, composite.height as usize);
    let plane = w * h;
    let color = color_of(doc);
    let mut planes = composite.data[..color * plane].to_vec();
    if composite.channels as usize > color {
        let a = &composite.data[color * plane..(color + 1) * plane];
        for c in 0..color {
            for (v, &a) in planes[c * plane..(c + 1) * plane].iter_mut().zip(a) {
                *v =
                    ((u32::from(*v) * u32::from(a) + 255 * (255 - u32::from(a)) + 127) / 255) as u8;
            }
        }
    }
    let scale = (106.0 / w.max(h) as f64).min(1.0);
    let nw = ((w as f64 * scale).round() as usize).max(1);
    let nh = ((h as f64 * scale).round() as usize).max(1);
    let samples = downsample(
        &planar_to_interleaved(&planes, color, plane),
        color,
        w,
        h,
        nw,
        nh,
        Downsample::Average,
        255,
    );
    (
        nw as u32,
        nh as u32,
        color == 1,
        samples.into_iter().map(|v| v as u8).collect(),
    )
}
