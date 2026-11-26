use core::pin::Pin;
use std::collections::HashSet;
use std::path::Path;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_core::hdr_merge::{
    median_exposure, merge, merge_working_bytes, Exposure, HdrMerge, MergeError, MergeOptions,
    ResponseCurve,
};
use pictura_core::memory::{governor, Reservation};
use pictura_core::{BitDepth, Document, Samples, SourceChannels, SourcePlanes};

use super::PictureView;
use crate::cxxqt_object::helpers::{decode_import, list_of_strings, safe_io};
use crate::cxxqt_object::helpers_composite::rgba_image;
use crate::layer_target::LayerTarget;

const MAX_SOURCES: usize = 32;

struct Prepared {
    sources: Vec<Vec<u8>>,
    ev100: Vec<f64>,
    width: u32,
    height: u32,
    _memory: Reservation<'static>,
}

pub struct HdrBracket(Result<Prepared, String>);

pub struct HdrResult(Result<HdrMerge, String>);

fn prepare(paths: &[String]) -> Result<Prepared, String> {
    if !(3..=MAX_SOURCES).contains(&paths.len()) {
        return Err(format!("Select 3 to {MAX_SOURCES} exposure files."));
    }
    let budget = pictura_codec::ImageBudget::default();
    let mut seen = HashSet::new();
    let mut sources = Vec::with_capacity(paths.len());
    let mut ev100 = Vec::with_capacity(paths.len());
    let mut working = None;
    let mut dimensions = (0, 0);
    for path in paths {
        let canonical = Path::new(path)
            .canonicalize()
            .map_err(|_| format!("Cannot read {path}"))?;
        if !seen.insert(canonical.clone()) {
            return Err(format!("Duplicate exposure file: {path}"));
        }
        let length = canonical
            .metadata()
            .ok()
            .filter(|meta| meta.is_file() && meta.len() <= safe_io::import_file_limit())
            .map(|meta| meta.len())
            .ok_or_else(|| format!("Cannot read {path} within the import limit"))?;
        // A read buffer may grow geometrically before reaching its known size.
        let _read_memory = governor()
            .reserve(length.saturating_mul(2), "read an HDR exposure")
            .map_err(|_| "Insufficient memory to read the exposure files".to_owned())?;
        let bytes = safe_io::read_capped(&canonical, length)
            .ok_or_else(|| format!("Cannot read {path}"))?;
        let header = pictura_codec::probe_image(&bytes, budget)
            .map_err(|error| format!("Cannot import {path}: {error}"))?;
        if matches!(header.format, pictura_codec::ImageFormat::Png | pictura_codec::ImageFormat::Tiff | pictura_codec::ImageFormat::Jpeg) && header.bit_depth > 8 {
            return Err(format!("HDR merge supports at most 8 bits per channel: {path}"));
        }
        let next_size = (header.width, header.height);
        if working.is_none() {
            let guess = merge_working_bytes(header.width, header.height, paths.len())
                .ok_or_else(|| "Exposure bracket dimensions overflow".to_owned())?;
            working = Some(
                governor()
                    .reserve(guess, "merge an HDR bracket")
                    .map_err(|_| "Insufficient memory to merge this exposure bracket".to_owned())?,
            );
            dimensions = next_size;
        } else if dimensions != next_size {
            return Err(format!("Exposure dimensions do not match: {path}"));
        }
        let (rgba, width, height) =
            decode_import(&bytes).ok_or_else(|| format!("Cannot decode {path}"))?;
        if (width, height) != dimensions {
            return Err(format!("Decoded exposure dimensions changed: {path}"));
        }
        sources.push(super::convert_placed_rgba(
            &bytes, rgba, width, height, pictura_codec::Policy::Convert,
        ));
        ev100.push(pictura_codec::raster_ev100(&bytes).unwrap_or(f64::NAN));
    }

    Ok(Prepared {
        sources,
        ev100,
        width: dimensions.0,
        height: dimensions.1,
        _memory: working.expect("there are at least three sources"),
    })
}

pub fn hdr_bracket_open(paths: &QStringList) -> Box<HdrBracket> {
    Box::new(HdrBracket(prepare(&list_of_strings(paths))))
}

pub fn hdr_bracket_error(bracket: &HdrBracket) -> QString {
    bracket.0.as_ref().err().map_or_else(QString::default, |error| QString::from(error.as_str()))
}

pub fn hdr_bracket_exposures(bracket: &HdrBracket) -> Vec<f64> {
    bracket.0.as_ref().map(|input| input.ev100.clone()).unwrap_or_default()
}

pub fn hdr_bracket_base(exposures: &[f64]) -> i32 {
    median_exposure(exposures).map_or(-1, |index| index as i32)
}

pub fn hdr_bracket_merge(
    bracket: &HdrBracket,
    ev100: &[f64],
    align: bool,
    ghost_base: i32,
    curve: &[f64],
) -> Box<HdrResult> {
    Box::new(HdrResult(merge_prepared(bracket, ev100, align, ghost_base, curve)))
}

fn merge_prepared(
    bracket: &HdrBracket,
    ev100: &[f64],
    align: bool,
    ghost_base: i32,
    curve: &[f64],
) -> Result<HdrMerge, String> {
    let prepared = bracket.0.as_ref().map_err(Clone::clone)?;
    if ev100.len() != prepared.sources.len() {
        return Err("Supply an exposure value for every selected source".to_owned());
    }
    let response = if curve.is_empty() {
        None
    } else {
        Some(
            <[f64; 3]>::try_from(curve)
                .ok()
                .and_then(ResponseCurve::new)
                .ok_or_else(|| "Invalid Pictura response curve".to_owned())?,
        )
    };
    let sources: Vec<_> = prepared.sources.iter().zip(ev100).map(|(rgba, &ev100)| Exposure {
        rgba: rgba.as_chunks::<4>().0,
        ev100,
    }).collect();
    let options = MergeOptions {
        align,
        ghost_base: usize::try_from(ghost_base).ok(),
        response,
    };
    merge(&sources, prepared.width, prepared.height, &options).map_err(|error| match error {
        MergeError::InvalidBracket => "Invalid HDR exposure bracket".to_owned(),
        MergeError::InvalidExposure => "Supply distinct, finite EV100 values within 32 stops".to_owned(),
        MergeError::Alignment(index) => format!("Could not align exposure {}", index + 1),
    })
}

pub fn hdr_result_error(result: &HdrResult) -> QString {
    result.0.as_ref().err().map_or_else(QString::default, |error| QString::from(error.as_str()))
}

pub fn hdr_result_curve(result: &HdrResult) -> Vec<f64> {
    result.0.as_ref().map(|merged| merged.response.exponents().to_vec()).unwrap_or_default()
}

pub fn hdr_result_preview(bracket: &HdrBracket, result: &HdrResult, ev: f64) -> QImage {
    let (Ok(input), Ok(merged)) = (&bracket.0, &result.0) else {
        return QImage::default();
    };
    let stride = input.width.max(input.height).div_ceil(640).max(1);
    let width = input.width.div_ceil(stride);
    let height = input.height.div_ceil(stride);
    let scale = ev.clamp(-8.0, 8.0).exp2() as f32;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in (0..input.height).step_by(stride as usize) {
        for x in (0..input.width).step_by(stride as usize) {
            let i = (y * input.width + x) as usize;
            pixels.extend(merged.planes[..3].iter().map(|p| display(p[i] * scale)));
            pixels.push(display(merged.planes[3][i]));
        }
    }
    rgba_image(pixels, width as i32, height as i32)
}

fn display(value: f32) -> u8 {
    (value * 256.0).trunc().clamp(0.0, 255.0) as u8
}

fn merged_document(width: u32, height: u32, merge: HdrMerge) -> Option<Document> {
    let count = width as usize * height as usize;
    let samples = Samples::F32(merge.planes.concat().into());
    let display = samples.narrow_to_u8();
    let mut rgba = Vec::with_capacity(count * 4);
    for i in 0..count {
        rgba.extend((0..4).map(|channel| display[channel * count + i]));
    }
    let mut doc = Document::from_rgba("HDR Merge", width, height, &rgba);
    let layer = doc.layers.last_mut()?;
    let channels = [0, 1, 2, -1]
        .into_iter()
        .zip(merge.planes.map(|plane| Samples::F32(plane.into())))
        .collect();
    layer.source_channels = Some(SourceChannels::new(BitDepth::ThirtyTwo, layer.rect, channels));
    doc.source.set_native(Some(BitDepth::ThirtyTwo), Some(SourcePlanes {
        depth: BitDepth::ThirtyTwo, width, height, samples,
    })).ok()?;
    Some(doc)
}

pub fn hdr_result_install(
    mut view: Pin<&mut PictureView>,
    bracket: &HdrBracket,
    mut result: Box<HdrResult>,
) -> bool {
    let Ok(input) = &bracket.0 else {
        return false;
    };
    let Ok(merge) = std::mem::replace(&mut result.0, Err(String::new())) else {
        return false;
    };
    let Some(mut doc) = merged_document(input.width, input.height, merge) else {
        return false;
    };
    let target = LayerTarget::for_opened(&doc, Some("0".to_owned()));
    let history = super::history_for(&mut doc, &target, "Merge To HDR Pro");
    let mut rust = view.as_mut().rust_mut();
    rust.replace_document(Some(doc), history, target, None);
    rust.session.mark_unsaved();
    rust.path = None;
    rust.source_format = "psd".to_owned();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converted_sixteen_bit_samples_keep_precision() {
        let merged = HdrMerge {
            planes: [vec![0.301], vec![0.125], vec![0.75], vec![1.0]],
            response: ResponseCurve::new([1.0; 3]).unwrap(),
        };
        let mut doc = merged_document(1, 1, merged).unwrap();
        pictura_render::convert_depth_exposure_gamma(
            &mut doc, BitDepth::Sixteen,
            pictura_render::ExposureGamma { exposure_ev: 0.0, gamma: 1.0 },
        ).unwrap();
        let bytes = pictura_codec::write_psd(&doc).unwrap();
        let reopened = pictura_codec::read_psd(&bytes).unwrap();
        let native = reopened.layers.last().unwrap().source_channels.as_ref().unwrap();
        let red = &native.planes.iter().find(|(id, _)| *id == 0).unwrap().1;
        let Samples::U16(red) = red else { panic!("expected 16-bit layer") };
        assert!((i32::from(red[0]) - 19726).abs() <= 1);
    }

    #[test]
    fn merged_native_samples_survive_psd_and_psb() {
        let merged = HdrMerge {
            planes: [vec![2.5], vec![0.301], vec![0.125], vec![1.0]],
            response: ResponseCurve::new([1.0; 3]).unwrap(),
        };
        let doc = merged_document(1, 1, merged).unwrap();
        for bytes in [pictura_codec::write_psd(&doc).unwrap(), pictura_codec::write_psb(&doc).unwrap()] {
            let reopened = pictura_codec::read_psd(&bytes).unwrap();
            let layer = reopened.layers.last().unwrap();
            let native = layer.source_channels.as_ref().unwrap();
            let red = &native.planes.iter().find(|(id, _)| *id == 0).unwrap().1;
            assert_eq!(red, &Samples::F32(vec![2.5].into()));
            let green = &native.planes.iter().find(|(id, _)| *id == 1).unwrap().1;
            assert_eq!(green, &Samples::F32(vec![0.301].into()));
        }
    }
}
