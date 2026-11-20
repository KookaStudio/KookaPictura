//! Image > Calculations (#610): the bridge entry points. Sources arrive as
//! Apply Image snapshots, so the grey read, native depth, and mask geometry are
//! shared; this module only turns them into [`CalculationsParams`], computes
//! the plane, and writes the requested result.

use super::imp::{mode_of, ApplyImageSnapshot};
use crate::cxxqt_object::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{BitDepth, BlendMode, Channel, ColorMode, Document, LayerParts, Plane, PsdRect};
use pictura_render::{calculate, CalculationsParams, CompositeScope, DocumentComposite};
use pictura_select::{CombineMode, Selection};

#[allow(clippy::too_many_arguments)]
fn params(
    source1: &ApplyImageSnapshot,
    invert1: bool,
    source2: &ApplyImageSnapshot,
    invert2: bool,
    mode: &QString,
    scale: f64,
    offset: i32,
    opacity: i32,
    mask: &ApplyImageSnapshot,
    mask_invert: bool,
) -> Option<CalculationsParams> {
    Some(CalculationsParams {
        source1: source1.0.clone()?,
        invert1,
        source2: source2.0.clone()?,
        invert2,
        mode: mode_of(&mode.to_string(), scale, offset)?,
        opacity: opacity.clamp(0, 100) as f32 / 100.0,
        mask: mask.0.clone(),
        mask_invert,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn calculations_preview(
    source1: &ApplyImageSnapshot,
    invert1: bool,
    source2: &ApplyImageSnapshot,
    invert2: bool,
    mode: &QString,
    scale: f64,
    offset: i32,
    opacity: i32,
    mask: &ApplyImageSnapshot,
    mask_invert: bool,
    width: &mut i32,
    height: &mut i32,
) -> Vec<u8> {
    *width = 0;
    *height = 0;
    let Some(p) = params(
        source1,
        invert1,
        source2,
        invert2,
        mode,
        scale,
        offset,
        opacity,
        mask,
        mask_invert,
    ) else {
        return Vec::new();
    };
    let Some(plane) = calculate(&p) else {
        return Vec::new();
    };
    *width = p.source1.width as i32;
    *height = p.source1.height as i32;
    plane.to_vec()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn calculations_apply(
    view: Pin<&mut PictureView>,
    source1: &ApplyImageSnapshot,
    invert1: bool,
    source2: &ApplyImageSnapshot,
    invert2: bool,
    mode: &QString,
    scale: f64,
    offset: i32,
    opacity: i32,
    mask: &ApplyImageSnapshot,
    mask_invert: bool,
    target: &QString,
) -> bool {
    let Some(p) = params(
        source1,
        invert1,
        source2,
        invert2,
        mode,
        scale,
        offset,
        opacity,
        mask,
        mask_invert,
    ) else {
        return false;
    };
    let Some(plane) = calculate(&p) else {
        return false;
    };
    let shape = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        if doc.width as usize != p.source1.width || doc.height as usize != p.source1.height {
            return false;
        }
        Selection {
            width: doc.width,
            height: doc.height,
            data: plane,
        }
    };
    match target.to_string().as_str() {
        "selection" => view.apply_selection_labeled(shape, CombineMode::New, "Calculations"),
        "channel" => view.save_calculations_channel(shape),
        _ => false,
    }
}

/// The 8-bit grayscale image a New Document result opens: one pixel layer
/// holding `plane`, with the composite refreshed from it.
fn gray_document(width: u32, height: u32, plane: Plane<u8>, gpu: bool) -> Document {
    let planar = width as usize * height as usize;
    let mut doc =
        Document::with_uniform_composite(width, height, ColorMode::Grayscale, BitDepth::Eight, 0);
    let _ = doc.layers.push(pictura_codec::layer_from_parts(LayerParts {
        name: "Layer 0".to_string(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: height as i32,
            right: width as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: vec![
            Channel { id: 0, data: plane },
            Channel {
                id: -1,
                data: vec![255u8; planar].into(),
            },
        ],
        ..Default::default()
    }));
    DocumentComposite::new(&mut doc).refresh(CompositeScope::Full, gpu);
    doc
}

#[allow(clippy::too_many_arguments)]
pub(super) fn calculations_document(
    mut view: Pin<&mut PictureView>,
    source1: &ApplyImageSnapshot,
    invert1: bool,
    source2: &ApplyImageSnapshot,
    invert2: bool,
    mode: &QString,
    scale: f64,
    offset: i32,
    opacity: i32,
    mask: &ApplyImageSnapshot,
    mask_invert: bool,
) -> bool {
    let Some(p) = params(
        source1,
        invert1,
        source2,
        invert2,
        mode,
        scale,
        offset,
        opacity,
        mask,
        mask_invert,
    ) else {
        return false;
    };
    let Some(plane) = calculate(&p) else {
        return false;
    };
    let (width, height) = (p.source1.width as u32, p.source1.height as u32);
    if width == 0 || height == 0 {
        return false;
    }
    let gpu_compute = view.rust().gpu_compute;
    let doc = gray_document(width, height, plane, gpu_compute);
    let mut rust = view.as_mut().rust_mut();
    rust.replace_document(
        Some(doc),
        crate::history::History::default(),
        Default::default(),
        None,
    );
    rust.select_only_layer("0");
    let target = rust.layer_target.clone();
    let rust = &mut *rust;
    if let Some(doc) = rust.session.doc.as_mut() {
        rust.session
            .history
            .capture_live(doc, &None, &target, "New");
    }
    rust.path = None;
    rust.session.mark_saved();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculations_document_fills_gray_layer() {
        let doc = gray_document(2, 1, vec![10u8, 200].into(), false);
        let layer = doc.layers.first().unwrap();
        let channel = layer.channels().iter().find(|c| c.id == 0).unwrap();
        assert_eq!(&channel.data[..], &[10, 200]);
        assert_eq!(doc.mode, ColorMode::Grayscale);
        let rgba = pictura_render::composite_rgba(&doc);
        assert_eq!(rgba.channels, 4);
        assert_eq!(&rgba.data[..2], &[10, 200]);
    }
}
