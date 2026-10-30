use super::*;

use super::super::super::helpers::{parse_anchor, parse_resample};
use crate::cxxqt_object::edit::Edit;
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
    let (resized, changed) = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        let resize = resample.filter(|_| (width as u32, height as u32) != (doc.width, doc.height));
        if let Some(resample) = resize {
            if pictura_render::resize_document(doc, width as u32, height as u32, resample).is_err()
            {
                return false;
            }
        }
        let current = document_resolution(doc);
        let moved = current.is_none_or(|r| (r.ppi - ppi).abs() > 1e-6 || r.per_cm != per_cm);
        // A document with no resolution resource already reads as 72 ppi;
        // writing one only for that would be a no-op state.
        let needed = moved && !(current.is_none() && (ppi - DEFAULT_PPI).abs() < 1e-6 && !per_cm);
        if needed {
            set_document_resolution(doc, wanted);
        }
        (resize.is_some(), resize.is_some() || needed)
    };
    if !changed {
        return false;
    }
    view.as_mut().edit("Image Size", |rust| {
        if resized {
            rust.session.set_selection(None);
            Edit::Full
        } else {
            Edit::Meta
        }
    });
    true
}

pub(super) fn canvas_size_apply(
    mut view: Pin<&mut PictureView>,
    anchor: &QString,
    width: i32,
    height: i32,
    fill: u32,
) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let Some(anchor) = parse_anchor(&anchor.to_string()) else {
        return false;
    };
    if width < 1 || height < 1 {
        return false;
    }
    {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        if (width as u32, height as u32) == (doc.width, doc.height) {
            return false;
        }
        if pictura_render::resize_canvas_document(doc, width as u32, height as u32, anchor).is_err()
        {
            return false;
        }
        pictura_render::extend_background(doc, [(fill >> 16) as u8, (fill >> 8) as u8, fill as u8]);
        rust.session.set_selection(None);
    }
    view.as_mut().edit("Canvas Size", |_| Edit::Full);
    true
}
