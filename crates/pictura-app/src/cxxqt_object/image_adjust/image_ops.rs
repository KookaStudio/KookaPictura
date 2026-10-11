//! Image ▸ Duplicate / Trim. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow). Ported from
//! photorust's `Image` menu handlers.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::helpers_composite::{buffer_to_rgba_bytes, current_buffer};
use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::Document;

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
        /// Copy `source`'s document into `target` as a new tab. `merged` flattens
        /// the layer stack to one layer (Image ▸ Duplicate's "Duplicate Merged
        /// Layers Only"); otherwise the whole stack is copied. One "Duplicate"
        /// state on the target; false without a source document.
        fn duplicate_into(
            source: &PictureView,
            target: Pin<&mut PictureView>,
            name: &QString,
            merged: bool,
        ) -> bool;

        /// `Image ▸ Trim`: crop the canvas to the bounds of the visible content.
        /// One "Trim" state; false when the content already fills the canvas.
        fn trim_image(view: Pin<&mut PictureView>) -> bool;

        /// The name Image ▸ Duplicate suggests: the source file's base name plus
        /// " copy".
        fn duplicate_name(view: &PictureView) -> QString;

        /// `Image ▸ Image Rotation ▸ Arbitrary`: rotate the document about its
        /// centre by `angle_deg` degrees clockwise, recomposite, and record one
        /// "Rotate" state. False for a non-finite or zero angle, an out-of-range
        /// angle, or without a document.
        fn rotate_document_arbitrary(view: Pin<&mut PictureView>, angle_deg: f64) -> bool;
    }
}

fn duplicate_into(
    source: &PictureView,
    target: Pin<&mut PictureView>,
    name: &QString,
    merged: bool,
) -> bool {
    let rust = source.rust();
    let Some(src) = rust.doc.as_ref() else {
        return false;
    };
    let doc = if merged {
        let buffer = current_buffer(src, rust.gpu_compute);
        let rgba = buffer_to_rgba_bytes(&buffer);
        Document::from_rgba(&name.to_string(), src.width, src.height, &rgba)
    } else {
        src.clone()
    };
    finish(target, doc, "Duplicate")
}

/// Install `doc` on `view` as a fresh document with a single history anchor.
fn finish(mut view: Pin<&mut PictureView>, doc: Document, label: &str) -> bool {
    {
        let mut rust = view.as_mut().rust_mut();
        rust.doc = Some(doc);
        rust.reset_edit_state();
        rust.active_layer = Some("0".to_string());
    }
    view.as_mut().recomposite();
    view.as_mut().record(label);
    view.as_mut().changed();
    true
}

fn trim_image(view: Pin<&mut PictureView>) -> bool {
    let Some((rgba, width, height, gpu)) = view.rust().doc.as_ref().map(|doc| {
        let buffer = current_buffer(doc, view.rust().gpu_compute);
        (
            buffer_to_rgba_bytes(&buffer),
            doc.width,
            doc.height,
            view.rust().gpu_compute,
        )
    }) else {
        return false;
    };
    let _ = gpu;
    // The content is wherever the composite has coverage; the layer rects are
    // the whole canvas even when the pixels are transparent.
    let mut left = width as i32;
    let mut top = height as i32;
    let mut right = -1i32;
    let mut bottom = -1i32;
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            if rgba[((y * width as i32 + x) * 4 + 3) as usize] > 0 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
            }
        }
    }
    if right <= left || bottom <= top {
        return false;
    }
    if left == 0 && top == 0 && right == width as i32 && bottom == height as i32 {
        return false;
    }
    let mut cropped = view.rust().doc.as_ref().cloned().unwrap();
    if !pictura_render::crop_document(
        &mut cropped,
        left,
        top,
        (right - left) as u32,
        (bottom - top) as u32,
    ) {
        return false;
    }
    finish(view, cropped, "Trim")
}

fn rotate_document_arbitrary(mut view: Pin<&mut PictureView>, angle_deg: f64) -> bool {
    if !angle_deg.is_finite() || angle_deg == 0.0 || !(-359.99..=359.99).contains(&angle_deg) {
        return false;
    }
    let rotated = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.doc.as_mut() else {
            return false;
        };
        let pivot = (doc.width as f64 / 2.0, doc.height as f64 / 2.0);
        pictura_render::rotate_document_in(doc, angle_deg, pivot)
    };
    if rotated {
        view.as_mut().rust_mut().selection = None;
        view.as_mut().recomposite();
        view.as_mut().record("Rotate");
    }
    rotated
}

fn duplicate_name(view: &PictureView) -> QString {
    let base = view.rust().path.as_deref().map_or_else(
        || "Untitled".to_string(),
        |p| {
            std::path::Path::new(p)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        },
    );
    QString::from(format!("{base} copy"))
}
