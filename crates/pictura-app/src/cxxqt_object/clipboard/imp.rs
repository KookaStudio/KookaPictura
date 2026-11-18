use super::*;

use super::super::picture::picture_region;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::Coverage;
use pictura_render::{Clip, PasteMode};
use std::sync::Mutex;

use ffi::PasteKind;

pub(super) static CLIPBOARD: Mutex<Option<Clip>> = Mutex::new(None);

pub(super) fn store(clip: Clip) {
    *CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner()) = Some(clip);
}

pub(super) fn stored() -> Option<Clip> {
    CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The clipboard's pixels as straight RGBA (selection folded into alpha):
/// `(width, height, rgba)`.
pub(in crate::cxxqt_object) fn clipboard_pixels() -> Option<(u32, u32, Vec<u8>)> {
    stored().map(|clip| (clip.width(), clip.height(), clip.masked_rgba()))
}

pub(super) fn clipboard_has_contents() -> bool {
    CLIPBOARD
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

pub(super) fn clipboard_export(width: &mut i32, height: &mut i32) -> Vec<u8> {
    let Some(clip) = stored() else {
        (*width, *height) = (0, 0);
        return Vec::new();
    };
    (*width, *height) = (clip.width() as i32, clip.height() as i32);
    clip.masked_rgba()
}

pub(super) fn clipboard_import(width: i32, height: i32, rgba: &[u8]) -> bool {
    if width <= 0 || height <= 0 {
        return false;
    }
    match Clip::from_rgba(width as u32, height as u32, rgba.to_vec()) {
        Some(clip) => {
            store(clip);
            true
        }
        None => false,
    }
}

pub(super) fn clipboard_purge() {
    *CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

pub(super) fn copy_layer_from_document(
    mut view: Pin<&mut PictureView>,
    source: &PictureView,
    path: &QString,
    selection: &QString,
) -> QString {
    let created = {
        let source = source.rust();
        let mut rust = view.as_mut().rust_mut();
        match (source.session.doc.as_ref(), rust.session.doc.as_mut()) {
            (Some(src), Some(dst)) => pictura_render::copy_path_to_document(
                src,
                &path.to_string(),
                dst,
                &selection.to_string(),
            ),
            _ => None,
        }
    };
    let Some(created) = created else {
        return QString::default();
    };
    view.as_mut().rust_mut().select_only_layer(&created);
    view.as_mut()
        .edit("Duplicate Layer", |_| Edit::Structural { damage: None });
    QString::from(created.as_str())
}

/// The active selection's coverage plane, when it matches the document: a
/// tile-sharing clone, never a contiguous copy (#1268).
pub(in crate::cxxqt_object) fn clip_selection(rust: &PictureViewRust) -> Option<Coverage> {
    let doc = rust.session.doc.as_ref()?;
    rust.session
        .selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .map(pictura_select::Selection::coverage)
}

/// The clip `Copy`/`Copy Merged` would take, without storing it.
pub(in crate::cxxqt_object) fn copy_of(
    rust: &PictureViewRust,
    path: &str,
    merged: bool,
) -> Option<Clip> {
    let selection = clip_selection(rust);
    let doc = rust.session.doc.as_ref()?;
    if merged {
        pictura_render::copy_merged_region(doc, selection.as_ref(), |region| {
            picture_region(doc, rust, region)
        })
    } else {
        pictura_render::copy_layer(doc, path, selection.as_ref())
    }
}

pub(super) fn clipboard_copy(view: Pin<&mut PictureView>, path: &QString, merged: bool) -> bool {
    match copy_of(view.rust(), &path.to_string(), merged) {
        Some(clip) => {
            store(clip);
            true
        }
        None => false,
    }
}

pub(super) fn clipboard_cut(view: Pin<&mut PictureView>, path: &QString) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    let path = path.to_string();
    let Some(clip) = copy_of(view.rust(), &path, false) else {
        return false;
    };
    if !clear(view, &path, "Cut") {
        return false;
    }
    store(clip);
    true
}

pub(super) fn clipboard_clear(view: Pin<&mut PictureView>, path: &QString) -> bool {
    if view.rust().modal_transform_blocks_edit() {
        return false;
    }
    clear(view, &path.to_string(), "Clear")
}

pub(super) fn clear(mut view: Pin<&mut PictureView>, path: &str, label: &str) -> bool {
    let selection = clip_selection(view.rust());
    let cleared = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => pictura_render::clear_layer(doc, path, selection.as_ref()),
        None => false,
    };
    if cleared {
        let region = view
            .rust()
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, path));
        let outcome = match region {
            Some(rect) => Edit::regions(vec![rect], false),
            None => Edit::Full,
        };
        view.as_mut().edit(label, |_| outcome);
    }
    cleared
}

pub(super) fn clipboard_paste(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    kind: PasteKind,
    centre_x: f64,
    centre_y: f64,
) -> QString {
    if view.rust().modal_transform_blocks_edit() {
        return QString::default();
    }
    let (mode, label) = match kind {
        PasteKind::Into => (PasteMode::Into, "Paste Into"),
        PasteKind::Outside => (PasteMode::Outside, "Paste Outside"),
        _ => (PasteMode::Plain, "Paste"),
    };
    let Some(clip) = stored() else {
        return QString::default();
    };
    let selection = clip_selection(view.rust());
    let Some((width, height)) = view
        .rust()
        .session
        .doc
        .as_ref()
        .map(|d| (d.width, d.height))
    else {
        return QString::default();
    };
    let into_bounds = selection
        .as_ref()
        .filter(|_| mode == PasteMode::Into)
        .and_then(|s| {
            s.covered_bounds(pictura_core::PsdRect {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            })
        });
    let origin = match kind {
        // Plain paste and Paste in Place keep the copied pixels' position.
        PasteKind::Plain | PasteKind::InPlace => (clip.rect.left, clip.rect.top),
        _ => {
            let (cx, cy) = match into_bounds {
                Some(b) => (
                    (b.left + b.right) as f64 / 2.0,
                    (b.top + b.bottom) as f64 / 2.0,
                ),
                None => (centre_x, centre_y),
            };
            (
                (cx - clip.width() as f64 / 2.0).round() as i32,
                (cy - clip.height() as f64 / 2.0).round() as i32,
            )
        }
    };
    let created = match view.as_mut().rust_mut().session.doc.as_mut() {
        Some(doc) => pictura_render::paste_clip(
            doc,
            &path.to_string(),
            &clip,
            origin,
            mode,
            selection.as_ref(),
        ),
        None => String::new(),
    };
    if created.is_empty() {
        return QString::default();
    }
    // CS6 drops the marquee after any paste; Reselect restores it. Into and
    // Outside have already folded it into the layer mask.
    let mut rust = view.as_mut().rust_mut();
    if let Some(selection) = rust.session.take_selection() {
        rust.session.deselected_selection = Some(selection);
    }
    view.as_mut().rust_mut().select_only_layer(&created);
    view.as_mut().clear_link_sets();
    // The paste only adds a layer, so its bounded rect bounds the composite change.
    let region = view
        .rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::damage::influence(doc, &created));
    let outcome = match region {
        Some(rect) => Edit::regions(vec![rect], false),
        None => Edit::Full,
    };
    view.as_mut().edit(label, |_| outcome);
    QString::from(created.as_str())
}

#[cfg(test)]
mod coverage_tests;
