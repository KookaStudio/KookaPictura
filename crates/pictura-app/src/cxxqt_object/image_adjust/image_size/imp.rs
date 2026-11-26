use super::*;

use super::super::super::helpers::{parse_anchor, parse_resample};
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::impl_core::{document_engine_error, JobBar, Outcome};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::{document_resolution, set_document_resolution, Resolution};

pub(super) const DEFAULT_PPI: f64 = 72.0;

pub(super) fn resolution(view: &PictureView) -> Option<Resolution> {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(document_resolution)
}

pub(super) fn document_ppi(view: &PictureView) -> f64 {
    resolution(view).map_or(DEFAULT_PPI, |r| r.ppi)
}

pub(super) fn document_ppi_per_cm(view: &PictureView) -> bool {
    resolution(view).is_some_and(|r| r.per_cm)
}

fn has_canvas_background(doc: &pictura_core::Document) -> bool {
    doc.layers
        .iter()
        .any(|layer| layer.background && !layer.is_group())
}

pub(super) fn canvas_size_has_background(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(has_canvas_background)
}

/// Whether a resample of the document to `width` × `height` passes the Image
/// Size gate (#1293); a refusal is recorded for the handler's message.
pub(crate) fn image_size_fits_budget(view: &PictureView, width: i32, height: i32) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| crate::cxxqt_object::helpers::image_size_admits(doc, width, height))
}

/// Whether a Canvas Size of the document to `width` × `height` at `anchor`
/// passes its gate (#1294); a refusal is recorded for the handler's message.
pub(super) fn canvas_size_fits_budget(
    view: &PictureView,
    anchor: &QString,
    width: i32,
    height: i32,
) -> bool {
    let Some(anchor) = parse_anchor(&anchor.to_string()) else {
        return false;
    };
    view.rust().session.doc.as_ref().is_some_and(|doc| {
        crate::cxxqt_object::helpers::canvas_size_admits(doc, width, height, anchor)
    })
}

pub(super) fn image_size_apply(
    mut view: Pin<&mut PictureView>,
    resample: &QString,
    width: i32,
    height: i32,
    ppi: f64,
    per_cm: bool,
) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Capability);
    }
    let resample = resample.to_string();
    let resample = if resample.is_empty() {
        None
    } else {
        match parse_resample(&resample) {
            Some(r) => Some(r),
            None => return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg),
        }
    };
    if width < 1 || height < 1 || !ppi.is_finite() || ppi <= 0.0 {
        return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg);
    }
    let wanted = Resolution { ppi, per_cm };
    let (resize, needed) = {
        let Some(doc) = view.rust().session.doc.as_ref() else {
            return false;
        };
        let resize = resample.filter(|_| (width as u32, height as u32) != (doc.width, doc.height));
        let current = document_resolution(doc);
        let moved = current.is_none_or(|r| (r.ppi - ppi).abs() > 1e-6 || r.per_cm != per_cm);
        // A document with no resolution resource already reads as 72 ppi;
        // writing one only for that would be a no-op state.
        let needed = moved && !(current.is_none() && (ppi - DEFAULT_PPI).abs() < 1e-6 && !per_cm);
        (resize, needed)
    };
    if resize.is_some() && !image_size_fits_budget(&view, width, height) {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Host);
    }
    if resize.is_none() && !needed {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Ok);
    }
    let bar = if resize.is_some() {
        JobBar::Counted
    } else {
        JobBar::Busy
    };
    view.as_mut().document_op(
        "Image Size",
        bar,
        move |doc| {
            if let Some(resample) = resize {
                pictura_render::resize_document(doc, width as u32, height as u32, resample)
                    .map_err(document_engine_error)?;
            }
            if needed {
                set_document_resolution(doc, wanted);
            }
            Ok(Some(Outcome::default()))
        },
        move |rust, _| {
            if crate::history_log::detailed() {
                rust.session.history_log_detail = Some(format!(
                    "image.image.size width={width} height={height} resolution={ppi}"
                ));
            }
            if resize.is_some() {
                rust.session.clear_geometry_selection();
                Edit::Full
            } else {
                Edit::Meta
            }
        },
    )
}

pub(super) fn canvas_size_apply(
    mut view: Pin<&mut PictureView>,
    anchor: &QString,
    width: i32,
    height: i32,
    fill: u32,
) -> bool {
    crate::cxxqt_object::helpers::memory_gate::clear();
    if view.rust().modal_transform_blocks_edit() {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Capability);
    }
    let Some(anchor) = parse_anchor(&anchor.to_string()) else {
        return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg);
    };
    if width < 1 || height < 1 {
        return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg);
    }
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return view.document_preflight_refusal(pictura_error::ErrorCode::InvalidArg);
    };
    if (width as u32, height as u32) == (doc.width, doc.height) {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Ok);
    }
    if !crate::cxxqt_object::helpers::canvas_size_admits(doc, width, height, anchor) {
        return view.document_preflight_refusal(pictura_error::ErrorCode::Host);
    }
    view.as_mut().document_op(
        "Canvas Size",
        JobBar::Busy,
        move |doc| {
            // Both steps run on a copy that shares the planes, so a refused
            // Background extension (#1294) leaves the document as it was.
            // A layer materialized from the merged composite during resize
            // preserves the original pixels but cannot authorize fill.
            let had_background = has_canvas_background(doc);
            let mut next = doc.clone();
            if let Some(job) = pictura_core::job::current() {
                job.set_stages(if had_background { 2 } else { 1 });
            }
            pictura_render::resize_canvas_document(&mut next, width as u32, height as u32, anchor)
                .map_err(document_engine_error)?;
            if had_background {
                pictura_render::extend_background(
                    &mut next,
                    [(fill >> 16) as u8, (fill >> 8) as u8, fill as u8],
                )
                .map_err(document_engine_error)?;
            }
            *doc = next;
            Ok(Some(Outcome::default()))
        },
        |rust, _| {
            rust.session.clear_geometry_selection();
            Edit::Full
        },
    )
}

pub(super) fn preview_image(view: &PictureView, max_side: i32) -> cxx_qt_lib::QImage {
    use crate::cxxqt_object::helpers_composite::premultiplied_display_image;
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return cxx_qt_lib::QImage::default();
    };
    let painting = rust.session.live.stroke().is_some();
    let side = u32::try_from(max_side).unwrap_or(0);
    match crate::cxxqt_object::picture::preview_display(doc, painting, side) {
        // SAFETY: a level is exactly width*height premultiplied RGBA8888 bytes.
        Some(level) => unsafe {
            cxx_qt_lib::QImage::from_raw_bytes(
                level.data().to_vec(),
                level.width() as i32,
                level.height() as i32,
                cxx_qt_lib::QImageFormat::Format_RGBA8888_Premultiplied,
            )
        },
        None => {
            let whole = crate::cxxqt_object::picture::whole(doc);
            premultiplied_display_image(&crate::cxxqt_object::tiles::display_region(
                doc, whole, painting,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::has_canvas_background;
    use pictura_core::{
        BitDepth, ColorMode, Document, Layer, LayerContent, LayerParts, LayerStack,
    };

    #[test]
    fn canvas_background_predicate_uses_flag_for_non_pixel_layers() {
        let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        let text = Layer::from_parts(
            LayerParts {
                background: true,
                ..Default::default()
            },
            LayerContent::Text(None),
        );
        doc.layers = LayerStack::from_layers(vec![text]);
        assert!(has_canvas_background(&doc));
        doc.layers[0].background = false;
        assert!(!has_canvas_background(&doc));
        let group = Layer::from_parts(
            LayerParts {
                background: true,
                ..Default::default()
            },
            LayerContent::Group(LayerStack::default()),
        );
        doc.layers = LayerStack::from_layers(vec![group]);
        assert!(!has_canvas_background(&doc));
    }
}
