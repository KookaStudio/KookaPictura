//! The Crop tool group's Crop, Perspective Crop, and Slice commands. Free functions
//! over a [`PictureView`] (their own bridge, so the `PictureView` declaration
//! list does not grow). A Perspective Crop records one "Perspective Crop" state,
//! a new slice one "Slice" state, a committed slice move/resize one "Edit
//! Slice" state, and a deletion one "Delete Slice" state; every refusal records
//! nothing.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Document, PsdRect};

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
        /// Perspective-crop to `quad` (8 values: TL, TR, BR, BL x/y in document pixels): warp every layer so the quad becomes the canvas, drop the selection, recomposite, and record one "Perspective Crop" state. False, changing nothing, without a document, on a refusal, or for a degenerate quad.
        fn perspective_crop_commit(view: Pin<&mut PictureView>, quad: &[f64]) -> bool;

        /// As `perspective_crop_commit`, but a positive `out_w`/`out_h` forces that output dimension (0 = auto from the quad).
        fn perspective_crop_commit_sized(
            view: Pin<&mut PictureView>,
            quad: &[f64],
            out_w: i32,
            out_h: i32,
        ) -> bool;

        /// The Crop tool's commit: crop to the `width`×`height` rect at `(x, y)`, discarding the pixels outside it when `delete_cropped`; drop the selection and record one history state named `label` ("Crop" when empty). A rect that stays inside the canvas is clamped as before; a rect that extends beyond it grows the canvas, filling the added area with the Background layer's colour `fill_rgb` (0xRRGGBB; ignored without a Background layer). False, changing nothing, without a document or for an empty rect.
        fn crop_to(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            delete_cropped: bool,
            fill_rgb: i32,
            label: &QString,
        ) -> bool;

        /// The Crop tool's straighten commit: rotate the document about
        /// `(pivot_x, pivot_y)` by `angle` degrees, then crop to the
        /// `width`×`height` box at `(x, y)` (discarding pixels outside when
        /// `delete_cropped`, growing the canvas for an out-of-bounds box and
        /// filling it with `fill_rgb`); drop the selection and record one
        /// history state named `label` ("Crop" when empty). False, changing nothing, without a document, an invalid angle,
        /// or an empty rect.
        fn straighten_crop(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            angle: f64,
            pivot_x: f64,
            pivot_y: f64,
            delete_cropped: bool,
            fill_rgb: i32,
            label: &QString,
        ) -> bool;

        /// Why the document cannot be perspective-cropped (live type, smart objects, vector masks, 16/32-bit), or empty when it can.
        fn perspective_crop_refusal_reason(view: &PictureView) -> QString;

        /// The union of every layer's rect as "left top right bottom", or empty without a document. Used by the crop tools' snapping.
        fn composite_bounds(view: &PictureView) -> QString;

        /// Number of resolved slices (user and auto); 0 without a document.
        fn slice_count(view: &PictureView) -> i32;

        /// Resolved slice `index` as `[x, y, width, height, number, userIndex]` (`userIndex` -1 for an auto slice); empty when out of range.
        fn slice_at(view: &PictureView, index: i32) -> Vec<i32>;

        /// Add a user slice and record one "Slice" state; its index, or -1 (no state) for an empty rect or without a document.
        fn add_user_slice(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        ) -> i32;

        /// Move or resize user slice `index`. With `commit` false this is a live drag step (no history, no `changed`); with `commit` true it records one "Edit Slice" state. False for an empty rect, a bad index, or no document.
        fn set_user_slice(
            view: Pin<&mut PictureView>,
            index: i32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            commit: bool,
        ) -> bool;

        /// Delete user slice `index` and record one "Delete Slice" state; false for a bad index or no document.
        fn remove_user_slice(view: Pin<&mut PictureView>, index: i32) -> bool;
    }
}

fn perspective_crop_commit(mut view: Pin<&mut PictureView>, quad: &[f64]) -> bool {
    perspective_crop_commit_sized(view.as_mut(), quad, 0, 0)
}

fn perspective_crop_commit_sized(
    mut view: Pin<&mut PictureView>,
    quad: &[f64],
    out_w: i32,
    out_h: i32,
) -> bool {
    let [x0, y0, x1, y1, x2, y2, x3, y3] = *quad else {
        return false;
    };
    let corners = [(x0, y0), (x1, y1), (x2, y2), (x3, y3)];
    let cropped = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        if !pictura_render::perspective_crop_sized(doc, corners, out_w, out_h) {
            return false;
        }
        // Canvas dimensions changed, so the old selection no longer maps.
        rust.session.selection = None;
        true
    };
    if cropped {
        view.as_mut().edit("Perspective Crop", |_| Edit::Full);
    }
    cropped
}

fn crop_to(
    mut view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    delete_cropped: bool,
    fill_rgb: i32,
    label: &QString,
) -> bool {
    if width < 1 || height < 1 {
        return false;
    }
    {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        if !apply_crop(doc, x, y, width, height, delete_cropped, fill_rgb) {
            return false;
        }
        // Canvas dimensions changed, so the old selection no longer maps.
        rust.session.selection = None;
    }
    view.as_mut().edit(&crop_label(label), |_| Edit::Full);
    true
}

/// The history name of a Crop tool commit: the caller's label (the guide
/// overlay, e.g. "Crop: Rule of Thirds"), or plain "Crop".
fn crop_label(label: &QString) -> String {
    if label.is_empty() {
        "Crop".to_owned()
    } else {
        label.to_string()
    }
}

/// Crop `doc` to the `width`×`height` rect at `(x, y)`. Inside the canvas the
/// rect is clamped (the classic path); beyond it the canvas grows and the added
/// area is filled with the Background layer's colour when `fill_rgb` is a
/// 0xRRGGBB value (ignored without a Background layer). Discards the pixels
/// outside the new canvas when `delete_cropped`.
fn apply_crop(
    doc: &mut Document,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    delete_cropped: bool,
    fill_rgb: i32,
) -> bool {
    let grows = x < 0 || y < 0 || x + width > doc.width as i32 || y + height > doc.height as i32;
    if grows {
        if !pictura_render::crop_document_grow(doc, x, y, width as u32, height as u32) {
            return false;
        }
        if fill_rgb >= 0 {
            pictura_render::extend_background(
                doc,
                [
                    (fill_rgb >> 16) as u8,
                    (fill_rgb >> 8) as u8,
                    fill_rgb as u8,
                ],
            );
        }
    } else if !pictura_render::crop_document(doc, x, y, width as u32, height as u32) {
        return false;
    }
    if delete_cropped {
        pictura_render::delete_cropped_pixels(doc);
    }
    true
}

fn straighten_crop(
    mut view: Pin<&mut PictureView>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    angle: f64,
    pivot_x: f64,
    pivot_y: f64,
    delete_cropped: bool,
    fill_rgb: i32,
    label: &QString,
) -> bool {
    if width < 1 || height < 1 {
        return false;
    }
    let cropped = {
        let mut rust = view.as_mut().rust_mut();
        let Some(doc) = rust.session.doc.as_mut() else {
            return false;
        };
        let pivot = (pivot_x, pivot_y);
        let Some((ox, oy)) =
            pictura_render::rotate_document_in_offset(doc.width, doc.height, angle, pivot)
        else {
            return false;
        };
        if !pictura_render::rotate_document_in(doc, angle, pivot) {
            return false;
        }
        let nx = (x as f64 + ox).round() as i32;
        let ny = (y as f64 + oy).round() as i32;
        if !apply_crop(doc, nx, ny, width, height, delete_cropped, fill_rgb) {
            return false;
        }
        // Canvas dimensions changed, so the old selection no longer maps.
        rust.session.selection = None;
        true
    };
    if cropped {
        view.as_mut().edit(&crop_label(label), |_| Edit::Full);
    }
    cropped
}

fn perspective_crop_refusal_reason(view: &PictureView) -> QString {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(pictura_render::perspective_crop_refusal)
        .map(QString::from)
        .unwrap_or_default()
}

/// The union of every layer's rect (recursively) as `"left top right bottom"`,
/// or empty without a document.
fn composite_bounds(view: &PictureView) -> QString {
    fn union(layers: &[pictura_core::Layer], acc: &mut Option<PsdRect>) {
        for layer in layers {
            match acc.as_mut() {
                Some(r) => {
                    r.left = r.left.min(layer.rect.left);
                    r.top = r.top.min(layer.rect.top);
                    r.right = r.right.max(layer.rect.right);
                    r.bottom = r.bottom.max(layer.rect.bottom);
                }
                None => *acc = Some(layer.rect),
            }
            union(&layer.children, acc);
        }
    }
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return QString::default();
    };
    let mut acc: Option<PsdRect> = None;
    union(&doc.layers, &mut acc);
    acc.map_or_else(QString::default, |r| {
        QString::from(format!("{} {} {} {}", r.left, r.top, r.right, r.bottom))
    })
}

fn slices(view: &PictureView) -> Vec<pictura_render::Slice> {
    view.rust()
        .session
        .doc
        .as_ref()
        .map(pictura_render::resolve_slices)
        .unwrap_or_default()
}

fn slice_count(view: &PictureView) -> i32 {
    slices(view).len() as i32
}

fn slice_at(view: &PictureView, index: i32) -> Vec<i32> {
    let slices = slices(view);
    let Some(slice) = usize::try_from(index).ok().and_then(|i| slices.get(i)) else {
        return Vec::new();
    };
    vec![
        slice.rect.left,
        slice.rect.top,
        slice.rect.width(),
        slice.rect.height(),
        slice.number as i32,
        slice.user_index.map_or(-1, |i| i as i32),
    ]
}

fn add_user_slice(mut view: Pin<&mut PictureView>, x: i32, y: i32, width: i32, height: i32) -> i32 {
    let added = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => pictura_render::add_slice(doc, rect_of(x, y, width, height)),
        None => None,
    };
    let Some(index) = added else {
        return -1;
    };
    view.as_mut().edit("Slice", |_| Edit::Meta);
    index as i32
}

fn rect_of(x: i32, y: i32, width: i32, height: i32) -> PsdRect {
    PsdRect {
        top: y,
        left: x,
        bottom: y.saturating_add(height.max(0)),
        right: x.saturating_add(width.max(0)),
    }
}

fn set_user_slice(
    mut view: Pin<&mut PictureView>,
    index: i32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    commit: bool,
) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let set = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => pictura_render::set_slice(doc, index, rect_of(x, y, width, height)),
        None => false,
    };
    if set && commit {
        view.as_mut().edit("Edit Slice", |_| Edit::Meta);
    }
    set
}

fn remove_user_slice(mut view: Pin<&mut PictureView>, index: i32) -> bool {
    let Ok(index) = usize::try_from(index) else {
        return false;
    };
    let removed = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => pictura_render::remove_slice(doc, index),
        None => false,
    };
    if removed {
        view.as_mut().edit("Delete Slice", |_| Edit::Meta);
    }
    removed
}
