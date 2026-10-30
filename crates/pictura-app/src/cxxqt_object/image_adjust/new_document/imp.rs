use super::*;

use super::super::super::open_jobs::make_background;
use crate::history::History;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::{set_document_resolution, Resolution};
use pictura_core::{BitDepth, ColorMode, Document};
use pictura_render::BitmapMethod;

pub(super) const DEFAULT_PPI: f64 = 72.0;

/// The finishing steps for `mode` at `bits` on an 8-bit base, or `None` when
/// the pair is refused. Bitmap is 1-bit only, and the 1-bit depth is
/// Bitmap's alone.
pub(super) fn plan(
    mode: &str,
    bits: i32,
) -> Option<(&'static str, Option<ColorMode>, Option<BitDepth>)> {
    let depth = match bits {
        1 if mode == "bitmap" => None,
        8 if mode != "bitmap" => None,
        16 if mode != "bitmap" => Some(BitDepth::Sixteen),
        32 if matches!(mode, "rgb" | "grayscale") => Some(BitDepth::ThirtyTwo),
        _ => return None,
    };
    let (base, target) = match mode {
        "rgb" => ("rgb", None),
        "grayscale" => ("grayscale", None),
        "bitmap" => ("grayscale", Some(ColorMode::Bitmap)),
        "cmyk" => ("rgb", Some(ColorMode::Cmyk)),
        "lab" => ("rgb", Some(ColorMode::Lab)),
        _ => return None,
    };
    Some((base, target, depth))
}

/// Fill the base layer's colour planes with `rgb` (its luminance on Gray).
pub(super) fn fill_layer(doc: &mut Document, rgb: [u8; 3]) {
    let gray = doc.mode == ColorMode::Grayscale;
    let luma = pictura_core::rec601_luma(rgb[0], rgb[1], rgb[2]);
    for layer in &mut doc.layers {
        for channel in &mut layer.channels {
            let value = match channel.id {
                0 if gray => luma,
                id @ 0..=2 => rgb[id as usize],
                _ => continue,
            };
            channel.data = vec![value; channel.data.len()].into();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_document(
    mut view: Pin<&mut PictureView>,
    width: i32,
    height: i32,
    mode: &QString,
    bits: i32,
    fill: u32,
    transparent: bool,
    ppi: f64,
    per_cm: bool,
) -> bool {
    let Some((base, target, depth)) = plan(&mode.to_string(), bits) else {
        return false;
    };
    if !ppi.is_finite() || ppi <= 0.0 {
        return false;
    }
    let background = if transparent { "transparent" } else { "white" };
    if !view.as_mut().new_document(
        width,
        height,
        &QString::from(base),
        8,
        &QString::from(background),
    ) {
        return false;
    }
    let rgb = [(fill >> 16) as u8, (fill >> 8) as u8, fill as u8];
    let finished = {
        let mut rust = view.as_mut().rust_mut();
        let profile = pictura_codec::Profile::from_icc(&rust.working_cmyk).ok();
        let options = rust.conversion_options;
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        if !transparent {
            if rgb != [255; 3] {
                fill_layer(doc, rgb);
            }
            // White and Background Color seed the locked Background
            // (open-and-new.md, New-document construction step 2).
            if let Some(layer) = doc.layers.first_mut() {
                make_background(layer);
            }
        }
        let converted = match target {
            Some(ColorMode::Bitmap) => {
                pictura_render::convert_to_bitmap(doc, BitmapMethod::Threshold).is_ok()
            }
            Some(mode) => {
                pictura_render::convert_mode_with_profiles(doc, mode, profile.as_ref(), options)
                    .is_ok()
            }
            None => true,
        } && depth
            .is_none_or(|d| pictura_render::convert_bit_depth(doc, d).is_ok());
        if converted && ((ppi - DEFAULT_PPI).abs() > 1e-6 || per_cm) {
            set_document_resolution(doc, Resolution { ppi, per_cm });
        }
        converted
    };
    if !finished {
        view.as_mut().rust_mut().session.doc = None;
        return false;
    }
    {
        let mut rust = view.as_mut().rust_mut();
        let layers = rust.session.doc.as_ref().map_or(0, |doc| doc.layers.len());
        if layers > 0 {
            rust.select_only_layer(&(layers - 1).to_string());
        } else {
            rust.layer_target.clear();
        }
    }
    view.as_mut().recomposite();
    // The conversions are part of making the document, not edits to undo.
    let mut rust = view.as_mut().rust_mut();
    rust.session.history = History::default();
    let target = rust.layer_target.clone();
    let rust = &mut *rust;
    if let Some(doc) = rust.session.doc.as_mut() {
        rust.session
            .history
            .capture_live(doc, &None, &target, "New");
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_pairs_each_mode_with_the_depths_it_takes() {
        assert_eq!(plan("rgb", 8), Some(("rgb", None, None)));
        assert_eq!(
            plan("cmyk", 16),
            Some(("rgb", Some(ColorMode::Cmyk), Some(BitDepth::Sixteen)))
        );
        assert_eq!(
            plan("bitmap", 1),
            Some(("grayscale", Some(ColorMode::Bitmap), None))
        );
        assert_eq!(plan("bitmap", 8), None);
        assert_eq!(plan("rgb", 1), None);
        assert_eq!(plan("lab", 32), None);
        assert_eq!(
            plan("grayscale", 32),
            Some(("grayscale", None, Some(BitDepth::ThirtyTwo)))
        );
        assert_eq!(plan("indexed", 8), None);
    }

    #[test]
    fn fill_uses_the_luminance_on_gray() {
        let mut doc = Document::new(2, 1, ColorMode::Grayscale, BitDepth::Eight);
        {
            let _ = doc.layers.push(pictura_core::Layer {
                channels: vec![pictura_core::Channel {
                    id: 0,
                    data: vec![255; 2].into(),
                }],
                ..Default::default()
            });
        };
        fill_layer(&mut doc, [255, 0, 0]);
        assert_eq!(&doc.layers[0].channels[0].data[..], &[76, 76]);
    }
}
