use super::*;

use super::super::super::helpers::{parse_anchor, parse_resample};
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::impl_core::{JobBar, Outcome};
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
        return false;
    }
    let resample = resample.to_string();
    let resample = if resample.is_empty() {
        None
    } else {
        match parse_resample(&resample) {
            Some(r) => Some(r),
            None => return false,
        }
    };
    if width < 1 || height < 1 || !ppi.is_finite() || ppi <= 0.0 {
        return false;
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
        return false;
    }
    if resize.is_none() && !needed {
        return false;
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
                pictura_render::resize_document(doc, width as u32, height as u32, resample).ok()?;
            }
            if needed {
                set_document_resolution(doc, wanted);
            }
            Some(Outcome::default())
        },
        move |rust, _| {
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
        return false;
    }
    let Some(anchor) = parse_anchor(&anchor.to_string()) else {
        return false;
    };
    if width < 1 || height < 1 {
        return false;
    }
    let admitted = view.rust().session.doc.as_ref().is_some_and(|doc| {
        (width as u32, height as u32) != (doc.width, doc.height)
            && crate::cxxqt_object::helpers::canvas_size_admits(doc, width, height, anchor)
    });
    if !admitted {
        return false;
    }
    view.as_mut().document_op(
        "Canvas Size",
        JobBar::Busy,
        move |doc| {
            // Both steps run on a copy that shares the planes, so a refused
            // Background extension (#1294) leaves the document as it was.
            let mut next = doc.clone();
            pictura_render::resize_canvas_document(&mut next, width as u32, height as u32, anchor)
                .ok()?;
            pictura_render::extend_background(
                &mut next,
                [(fill >> 16) as u8, (fill >> 8) as u8, fill as u8],
            )
            .ok()?;
            *doc = next;
            Some(Outcome::default())
        },
        |rust, _| {
            rust.session.clear_geometry_selection();
            Edit::Full
        },
    )
}
