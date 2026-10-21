//! Image ▸ Image Size / Canvas Size. Free functions over a [`PictureView`]
//! (their own bridge, so the `PictureView` declaration list does not grow).
//! Ported from photorust's `showImageSize` / `showCanvasSize`.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::helpers::{parse_anchor, parse_resample};
use super::super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_codec::{document_resolution, set_document_resolution, Resolution};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The document's resolution in pixels per inch (CS6's 72 when the
        /// file names none).
        fn document_ppi(view: &PictureView) -> f64;

        /// Whether the document shows its resolution in pixels/cm.
        fn document_ppi_per_cm(view: &PictureView) -> bool;

        /// Image ▸ Image Size: resample to `width` × `height` with `resample`
        /// (`"nearest"`, `"bilinear"`, `"bicubic"`; empty keeps the pixels) and
        /// set the resolution to `ppi` (shown per cm when `per_cm`). One
        /// "Image Size" state; false when nothing changes or the size is
        /// refused.
        fn image_size_apply(
            view: Pin<&mut PictureView>,
            resample: &QString,
            width: i32,
            height: i32,
            ppi: f64,
            per_cm: bool,
        ) -> bool;

        /// Image ▸ Canvas Size: place the document on a `width` × `height`
        /// canvas at `anchor` (`"top-left"` … `"bottom-right"`), filling the
        /// Background layer's new area with `fill` (`0xRRGGBB`). One
        /// "Canvas Size" state; false when nothing changes or it is refused.
        fn canvas_size_apply(
            view: Pin<&mut PictureView>,
            anchor: &QString,
            width: i32,
            height: i32,
            fill: u32,
        ) -> bool;
    }
}

const DEFAULT_PPI: f64 = 72.0;

fn resolution(view: &PictureView) -> Option<Resolution> {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(document_resolution)
}

fn document_ppi(view: &PictureView) -> f64 {
    resolution(view).map_or(DEFAULT_PPI, |r| r.ppi)
}

fn document_ppi_per_cm(view: &PictureView) -> bool {
    resolution(view).is_some_and(|r| r.per_cm)
}

fn image_size_apply(
    mut view: Pin<&mut PictureView>,
    resample: &QString,
    width: i32,
    height: i32,
    ppi: f64,
    per_cm: bool,
) -> bool {
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
            rust.session.selection = None;
            Edit::Full
        } else {
            Edit::Meta
        }
    });
    true
}

fn canvas_size_apply(
    mut view: Pin<&mut PictureView>,
    anchor: &QString,
    width: i32,
    height: i32,
    fill: u32,
) -> bool {
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
        rust.session.selection = None;
    }
    view.as_mut().edit("Canvas Size", |_| Edit::Full);
    view.as_mut().changed();
    true
}
