use super::*;

use super::super::picture::picture_region;
use crate::cxxqt_object::edit::Edit;
use crate::cxxqt_object::state::PictureViewRust;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
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

pub(super) fn copy_layers_from_document(
    mut view: Pin<&mut PictureView>,
    source: &PictureView,
    paths: &QStringList,
    dest_anchor: &QString,
    source_anchor: &QString,
) -> QStringList {
    let dest_anchor = dest_anchor.to_string();
    let source_anchor = source_anchor.to_string();
    // Refuse before `edit`, so a drop that cannot copy does not settle a pending
    // composite or cancel a vector place on the destination.
    let resolved = {
        let Some(src) = source.rust().session.doc.as_ref() else {
            return QStringList::default();
        };
        if view.rust().session.read_only() {
            return QStringList::default();
        }
        let Some(dst) = view.rust().session.doc.as_ref() else {
            return QStringList::default();
        };
        if pictura_render::document_color_mode(src) != pictura_render::document_color_mode(dst)
            || pictura_render::document_bit_depth(src) != pictura_render::document_bit_depth(dst)
        {
            return QStringList::default();
        }
        let mut resolved: Vec<String> = Vec::new();
        for path in paths.iter().map(|path| path.to_string()) {
            if resolved.contains(&path) {
                continue;
            }
            if pictura_render::resolve_path(src, &path).is_some() {
                resolved.push(path);
            }
        }
        if resolved.is_empty() {
            return QStringList::default();
        }
        // The document's own order is the contract; the target may have been
        // written by a command, not the panel.
        let order: std::collections::HashMap<String, usize> = pictura_render::flatten_rows(src)
            .into_iter()
            .enumerate()
            .map(|(index, (path, _))| (path, index))
            .collect();
        resolved.sort_by_key(|path| order.get(path).copied().unwrap_or(usize::MAX));
        resolved
    };
    let mut created: Vec<String> = Vec::new();
    view.as_mut().edit("Duplicate Layer", |rust| {
        let Some(src) = source.rust().session.doc.as_ref() else {
            return Edit::None;
        };
        let Some(dst) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        // A stale anchor takes `insert_node`'s root-append fallback, so compute
        // the insertion point for that case too.
        let anchor = if dest_anchor.is_empty()
            || pictura_render::resolve_path(dst, &dest_anchor).is_some()
        {
            dest_anchor.as_str()
        } else {
            ""
        };
        // A group or an empty target takes an append, which lands on top; a
        // sibling takes an insert above it. The insertion point is fixed for the
        // whole batch, so the source order is recovered by processing the
        // append case bottom-to-top and the sibling case top-to-bottom.
        let append = anchor.is_empty()
            || pictura_render::resolve_path(dst, anchor).is_some_and(|layer| layer.is_group());
        let (container, base) = if anchor.is_empty() {
            (String::new(), dst.layers.len())
        } else if append {
            let base =
                pictura_render::resolve_path(dst, anchor).map_or(0, |group| group.children().len());
            (anchor.to_string(), base)
        } else if let Some((parent, index)) = anchor.rsplit_once('/') {
            (parent.to_string(), index.parse::<usize>().unwrap_or(0) + 1)
        } else {
            (String::new(), anchor.parse::<usize>().unwrap_or(0) + 1)
        };
        let ordered: Vec<&String> = if append {
            resolved.iter().rev().collect()
        } else {
            resolved.iter().collect()
        };
        let mut inserted: Vec<(String, String)> = Vec::new();
        for path in ordered {
            if let Some(new_path) = pictura_render::copy_path_to_document(src, path, dst, anchor) {
                inserted.push((path.clone(), new_path));
            }
        }
        if inserted.is_empty() {
            return Edit::None;
        }
        // Every copy lands at the same point, so the final index of the i-th
        // insertion is base + i for an append and base + (n - 1 - i) above a
        // sibling (later inserts push the earlier block up).
        let count = inserted.len();
        let mut pairs: Vec<(String, String)> = inserted
            .into_iter()
            .enumerate()
            .map(|(i, (source, _))| {
                let index = if append {
                    base + i
                } else {
                    base + (count - 1 - i)
                };
                let path = if container.is_empty() {
                    index.to_string()
                } else {
                    format!("{container}/{index}")
                };
                (source, path)
            })
            .collect();
        if append {
            pairs.reverse();
        }
        let finals: Vec<String> = pairs.iter().map(|(_, path)| path.clone()).collect();
        let active = pairs
            .iter()
            .find(|(source, _)| *source == source_anchor)
            .map(|(_, path)| path.clone())
            .unwrap_or_else(|| finals.first().cloned().unwrap_or_default());
        let mut target = rust.layer_target.clone();
        target.select(finals.clone(), Some(active), rust.session.doc.as_ref());
        rust.set_layer_target(target);
        created = finals;
        Edit::Structural { damage: None }
    });
    created.into_iter().map(QString::from).collect()
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
