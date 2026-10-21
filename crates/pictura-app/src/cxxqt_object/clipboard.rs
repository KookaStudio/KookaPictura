//! The Edit clipboard bridge: Cut / Copy / Copy Merged / Paste / Paste in
//! Place / Paste Into / Paste Outside / Clear and `Purge > Clipboard`. Free
//! functions over a [`PictureView`] (its own bridge, so the `PictureView`
//! declaration list does not grow). One clipboard serves every open document,
//! so a copy in one tab pastes into another; the shell mirrors it to the system
//! clipboard through `clipboard_export` / `clipboard_import`. Copy and Purge
//! record no history; Cut, Clear, and each paste record exactly one state, and
//! every refusal records nothing.
//!
//! [`PictureView`]: super::qobject::PictureView

use super::helpers::layer_visibility_region;
use super::helpers_composite::{current_buffer, rgba_frame};
use super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_render::{Clip, PasteMode};
use std::sync::Mutex;

#[cxx_qt::bridge]
pub mod ffi {
    /// Which Edit paste command runs.
    #[namespace = "pictura"]
    enum PasteKind {
        Plain,
        InPlace,
        Into,
        Outside,
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// `Copy` (`merged == false`) / `Copy Merged`: replace the clipboard with the selected pixels of the layer at `path` or of the visible composite; the whole layer/canvas without a selection. No history; false (clipboard kept) when nothing is copied.
        fn clipboard_copy(view: Pin<&mut PictureView>, path: &QString, merged: bool) -> bool;

        /// `Cut`: `Copy` then `Clear` as one "Cut" state; false without mutating on either refusal.
        fn clipboard_cut(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// `Clear`: erase the selection (the whole layer without one) from the layer at `path`; one "Clear" state; false on a lock or when nothing changes.
        fn clipboard_clear(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Paste the clipboard as a new layer above `path`: Plain and InPlace at the clip's own document rect, Into centred on the selection bounds, Outside centred on document point `(centre_x, centre_y)`. Every paste deselects into the reselect store; Into/Outside also mask by the selection. One state; returns the new path or empty.
        fn clipboard_paste(
            view: Pin<&mut PictureView>,
            path: &QString,
            kind: PasteKind,
            centre_x: f64,
            centre_y: f64,
        ) -> QString;

        /// Whether the clipboard holds a copy.
        fn clipboard_has_contents() -> bool;

        /// The copy as packed straight RGBA (selection folded into alpha) for the system clipboard; empty with zero dimensions when there is none.
        fn clipboard_export(width: &mut i32, height: &mut i32) -> Vec<u8>;

        /// Replace the copy with another application's packed straight RGBA image, placed at the canvas origin; false (copy kept) for a zero dimension or a short buffer.
        fn clipboard_import(width: i32, height: i32, rgba: &[u8]) -> bool;

        /// `Purge > Clipboard`: drop the copy. Not undoable.
        fn clipboard_purge();

        /// Deep-copy the layer node at `path` in `source` into `view` by the New Layer insertion rule relative to `selection` — the Layers panel drag onto another document. `source` and `view` must be different documents. One "Duplicate Layer" state; returns the new path, or empty for an unknown path or a different color mode or bit depth.
        fn copy_layer_from_document(
            view: Pin<&mut PictureView>,
            source: &PictureView,
            path: &QString,
            selection: &QString,
        ) -> QString;
    }
}

use ffi::PasteKind;

static CLIPBOARD: Mutex<Option<Clip>> = Mutex::new(None);

fn store(clip: Clip) {
    *CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner()) = Some(clip);
}

fn stored() -> Option<Clip> {
    CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn clipboard_has_contents() -> bool {
    CLIPBOARD
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

fn clipboard_export(width: &mut i32, height: &mut i32) -> Vec<u8> {
    let Some(clip) = stored() else {
        (*width, *height) = (0, 0);
        return Vec::new();
    };
    (*width, *height) = (clip.width() as i32, clip.height() as i32);
    clip.masked_rgba()
}

fn clipboard_import(width: i32, height: i32, rgba: &[u8]) -> bool {
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

fn clipboard_purge() {
    *CLIPBOARD.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn copy_layer_from_document(
    mut view: Pin<&mut PictureView>,
    source: &PictureView,
    path: &QString,
    selection: &QString,
) -> QString {
    let created = {
        let source = source.rust();
        let mut rust = view.as_mut().rust_mut();
        match (source.doc.as_ref(), rust.doc.as_mut()) {
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
    view.as_mut().clear_link_sets();
    view.as_mut().recomposite();
    view.as_mut().record("Duplicate Layer");
    QString::from(created.as_str())
}

/// The active selection's coverage plane, when it matches the document.
fn coverage(view: &PictureView) -> Option<Vec<u8>> {
    let rust = view.rust();
    let doc = rust.doc.as_ref()?;
    rust.selection
        .as_ref()
        .filter(|s| s.width == doc.width && s.height == doc.height)
        .map(|s| s.data.clone())
}

/// The clip `Copy`/`Copy Merged` would take, without storing it.
fn copy_of(view: &PictureView, path: &str, merged: bool) -> Option<Clip> {
    let selection = coverage(view);
    let rust = view.rust();
    let doc = rust.doc.as_ref()?;
    if merged {
        let composite = rgba_frame(&current_buffer(doc, rust.gpu_compute));
        pictura_render::copy_merged(doc, &composite, selection.as_deref())
    } else {
        pictura_render::copy_layer(doc, path, selection.as_deref())
    }
}

fn clipboard_copy(view: Pin<&mut PictureView>, path: &QString, merged: bool) -> bool {
    match copy_of(&view, &path.to_string(), merged) {
        Some(clip) => {
            store(clip);
            true
        }
        None => false,
    }
}

fn clipboard_cut(view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let Some(clip) = copy_of(&view, &path, false) else {
        return false;
    };
    if !clear(view, &path, "Cut") {
        return false;
    }
    store(clip);
    true
}

fn clipboard_clear(view: Pin<&mut PictureView>, path: &QString) -> bool {
    clear(view, &path.to_string(), "Clear")
}

fn clear(mut view: Pin<&mut PictureView>, path: &str, label: &str) -> bool {
    let selection = coverage(&view);
    let cleared = match view.as_mut().rust_mut().doc.as_mut() {
        Some(doc) => pictura_render::clear_layer(doc, path, selection.as_deref()),
        None => false,
    };
    if cleared {
        let region = view
            .rust()
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, path))
            .and_then(layer_visibility_region);
        match region {
            Some(rect) => view.as_mut().refresh_region(rect),
            None => view.as_mut().recomposite(),
        }
        view.as_mut().record(label);
    }
    cleared
}

fn clipboard_paste(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    kind: PasteKind,
    centre_x: f64,
    centre_y: f64,
) -> QString {
    let (mode, label) = match kind {
        PasteKind::Into => (PasteMode::Into, "Paste Into"),
        PasteKind::Outside => (PasteMode::Outside, "Paste Outside"),
        _ => (PasteMode::Plain, "Paste"),
    };
    let Some(clip) = stored() else {
        return QString::default();
    };
    let selection = coverage(&view);
    let Some((width, height)) = view.rust().doc.as_ref().map(|d| (d.width, d.height)) else {
        return QString::default();
    };
    let into_bounds = selection
        .as_deref()
        .filter(|_| mode == PasteMode::Into)
        .and_then(|s| pictura_render::coverage_bounds(s, width, height));
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
    let created = match view.as_mut().rust_mut().doc.as_mut() {
        Some(doc) => pictura_render::paste_clip(
            doc,
            &path.to_string(),
            &clip,
            origin,
            mode,
            selection.as_deref(),
        ),
        None => String::new(),
    };
    if created.is_empty() {
        return QString::default();
    }
    // CS6 drops the marquee after any paste; Reselect restores it. Into and
    // Outside have already folded it into the layer mask.
    let mut rust = view.as_mut().rust_mut();
    if let Some(selection) = rust.selection.take() {
        rust.deselected_selection = Some(selection);
    }
    view.as_mut().rust_mut().select_only_layer(&created);
    view.as_mut().clear_link_sets();
    // The paste only adds a layer, so its bounded rect bounds the composite change.
    let region = view
        .rust()
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::resolve_path(doc, &created))
        .and_then(layer_visibility_region);
    match region {
        Some(rect) => view.as_mut().refresh_region(rect),
        None => view.as_mut().recomposite(),
    }
    view.as_mut().record(label);
    QString::from(created.as_str())
}
