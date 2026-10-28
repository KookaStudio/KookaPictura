//! Filters on a Smart Object become Smart Filters (#984): the Filter menu, the
//! Filter Gallery, Last Filter, and the control server append a generic
//! Smart Filter record instead of filtering the proxy, and the dialog previews
//! it by pushing the record onto the live chain. Lens Correction keeps its own
//! record, which a re-apply replaces so the dialog can re-edit it.

use core::pin::Pin;

use cxx_qt::CxxQtType;
use pictura_codec::FilterStep;
use pictura_core::{FilterMask, PsdRect, SmartFilter};
use pictura_filters::catalogue;

use super::super::document_session::{FilterPreview, PreviewBaseline};
use super::super::helpers::{active_mask_target, layer_has_effects};
use super::super::qobject::PictureView;
use super::super::state::PictureViewRust;
use crate::cxxqt_object::edit::structural;

/// Whether the edit target is a Smart Object's pixels: one active layer
/// carrying a smart object, with no layer mask targeted. Linked and legacy
/// objects count, so the destructive paths refuse them.
pub(in crate::cxxqt_object) fn on_smart_object(rust: &PictureViewRust) -> bool {
    active_mask_target(rust).is_none()
        && rust
            .session
            .doc
            .as_ref()
            .zip(rust.layer_target.single())
            .and_then(|(doc, path)| pictura_render::resolve_path(doc, path))
            .is_some_and(|layer| layer.smart_object.is_some())
}

/// The path of the embedded Smart Object a Smart Filter can attach to.
pub(in crate::cxxqt_object) fn smart_filter_target(rust: &PictureViewRust) -> Option<String> {
    if active_mask_target(rust).is_some() {
        return None;
    }
    let doc = rust.session.doc.as_ref()?;
    let path = rust.layer_target.single()?.to_string();
    pictura_render::can_replace_smart_object_contents(doc, &path).then_some(path)
}

/// Why `kind` may not apply to the active Smart Object: a kind that cannot be
/// a Smart Filter (CS6 asks to rasterize first). `None` off a Smart Object.
pub(in crate::cxxqt_object) fn smart_refusal(kind: &str, rust: &PictureViewRust) -> Option<String> {
    if !on_smart_object(rust) || catalogue::smart_capable(kind) {
        return None;
    }
    let label = catalogue::descriptor(kind).map_or(kind, |spec| spec.label);
    Some(format!(
        "Rasterize the Smart Object to apply {}",
        label.trim_end_matches('…').trim_end()
    ))
}

/// One step of `kind`: its full-resolution slots, the kind's defaults when
/// `params` is empty (a parameterless entry or the control server).
fn step(kind: &str, params: &[f64]) -> FilterStep {
    let slots = if params.is_empty() {
        catalogue::descriptor(kind).map_or_else(Vec::new, |spec| spec.defaults())
    } else {
        super::super::filter_map::commit_params(kind, params)
    };
    FilterStep {
        kind: kind.to_string(),
        slots,
    }
}

/// A gallery stack's steps: each kind takes the next `arity` slots.
pub(in crate::cxxqt_object) fn stack_steps(
    kinds: &[String],
    params: &[f64],
) -> Option<Vec<FilterStep>> {
    let mut at = 0;
    let mut steps = Vec::with_capacity(kinds.len());
    for kind in kinds {
        let arity = catalogue::arity(kind)?;
        steps.push(step(kind, params.get(at..at + arity)?));
        at += arity;
    }
    (at == params.len() && !steps.is_empty()).then_some(steps)
}

/// The record for `steps`, or `None` with `filter_error` set.
fn record(rust: &mut PictureViewRust, steps: &[FilterStep], gallery: bool) -> Option<SmartFilter> {
    if let Some(reason) = steps.iter().find_map(|s| smart_refusal(&s.kind, rust)) {
        rust.filter_error = Some(reason);
        return None;
    }
    let record = pictura_render::smart_filter_record(steps, gallery);
    if record.is_none() {
        rust.filter_error =
            Some("the filter is not recognised or its parameters are wrong".to_string());
    }
    record
}

/// The selection as a document-sized filter mask.
fn selection_mask(rust: &PictureViewRust) -> Option<FilterMask> {
    let selection = rust.session.selection.as_ref()?;
    Some(FilterMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: selection.height as i32,
            right: selection.width as i32,
        },
        data: selection.data.clone().into(),
    })
}

/// The target path, or `None` with `filter_error` set: a linked or legacy
/// object, a pending transform, or a preview open elsewhere.
fn target(rust: &mut PictureViewRust) -> Option<String> {
    rust.filter_error = None;
    if rust.modal_transform_blocks_edit() {
        rust.filter_error = Some("a transform is pending".to_string());
        return None;
    }
    let Some(path) = smart_filter_target(rust) else {
        rust.filter_error = Some("the smart object is not directly editable".to_string());
        return None;
    };
    if rust
        .session
        .live
        .filter_preview()
        .is_some_and(|p| p.layer_path != path)
        || rust.session.live.mask_filter_preview().is_some()
    {
        rust.filter_error = Some("another layer has an open preview; cancel it first".to_string());
        return None;
    }
    Some(path)
}

/// Put back an open preview on `path`; returns the rect it covered.
fn restore_preview(rust: &mut PictureViewRust, path: &str) -> Option<PsdRect> {
    let preview = rust
        .session
        .live
        .filter_preview()
        .is_some_and(|p| p.layer_path == path)
        .then(|| rust.session.live.take_filter_preview())
        .flatten()?;
    let layer = pictura_render::resolve_path_mut(rust.session.doc.as_mut()?, path)?;
    Some(preview.baseline.restore(layer))
}

/// The index of the Smart Filter being re-edited on `path`, if any.
fn edit_index(rust: &PictureViewRust, path: &str) -> Option<usize> {
    rust.smart_filter_edit
        .as_ref()
        .filter(|(p, _)| p == path)
        .map(|(_, i)| *i)
}

/// How a stored Smart Filter re-edits: 1 Camera Raw, 2 Lens Correction, 3 a
/// generic filter with a dialog, 4 a Filter Gallery stack, 0 none (a filter
/// with no dialog, or one this engine does not model).
pub(in crate::cxxqt_object) fn edit_kind(filter: &SmartFilter) -> i32 {
    match filter.filter_id {
        pictura_codec::CAMERA_RAW_FILTER_ID => 1,
        pictura_codec::LENS_CORRECTION_FILTER_ID => 2,
        pictura_codec::PICTURA_FILTER_ID => match record_steps(filter) {
            Some((_, true)) => 4,
            Some((steps, false))
                if steps.len() == 1
                    && catalogue::descriptor(&steps[0].kind)
                        .is_some_and(|spec| !spec.params.is_empty()) =>
            {
                3
            }
            _ => 0,
        },
        _ => 0,
    }
}

/// A generic record's steps and gallery flag, when it decodes.
pub(in crate::cxxqt_object) fn record_steps(
    filter: &SmartFilter,
) -> Option<(Vec<FilterStep>, bool)> {
    (filter.filter_id == pictura_codec::PICTURA_FILTER_ID)
        .then(|| pictura_codec::decode_pictura_filter(&filter.options))
        .flatten()
}

/// Start re-editing the Smart Filter at `index` on the active Smart Object at
/// `path`: until [`edit_end`], previews show the chain up to that filter (the
/// filters stacked above it are not previewed) and a commit replaces it in
/// place. False when `path` is not the active embedded Smart Object or the
/// filter has no generic dialog.
pub(in crate::cxxqt_object) fn edit_begin(
    rust: &mut PictureViewRust,
    path: &str,
    index: usize,
) -> bool {
    let editable = smart_filter_target(rust).as_deref() == Some(path)
        && rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::resolve_path(doc, path))
            .and_then(|layer| layer.smart_object.as_ref())
            .and_then(|so| so.smart_filters.get(index))
            .is_some_and(|filter| matches!(edit_kind(filter), 3 | 4));
    if editable {
        rust.smart_filter_edit = Some((path.to_string(), index));
    }
    editable
}

/// End a re-edit started by [`edit_begin`].
pub(in crate::cxxqt_object) fn edit_end(rust: &mut PictureViewRust) {
    rust.smart_filter_edit = None;
}

/// Preview `steps` as a Smart Filter on top of the active Smart Object's
/// chain: the layer is kept whole as the baseline, the record (and a mask from
/// the selection) is pushed onto the typed view only, and the compositor's
/// chain key re-renders it. While a filter is re-edited the record stands in
/// for it and the filters above it are left out, as CS6 does. Cancel restores
/// the layer. Returns the region to refresh; `None` with `filter_error` set
/// when refused.
pub(in crate::cxxqt_object) fn smart_preview(
    rust: &mut PictureViewRust,
    steps: &[FilterStep],
    gallery: bool,
) -> Option<Option<PsdRect>> {
    if !rust.slot_allows(super::super::document_session::SessionKind::FilterPreview) {
        rust.filter_error = Some("another session is live".to_string());
        return None;
    }
    let path = target(rust)?;
    let record = record(rust, steps, gallery)?;
    let editing = edit_index(rust, &path);
    let mask = selection_mask(rust).filter(|_| editing.is_none());
    restore_preview(rust, &path);
    let layer = pictura_render::resolve_path_mut(rust.session.doc.as_mut()?, &path)?;
    let baseline = PreviewBaseline::Layer(Box::new(layer.clone()));
    let so = layer.smart_object.as_mut()?;
    if let Some(index) = editing {
        so.smart_filters.truncate(index);
    }
    so.smart_filters.push(record);
    if so.filter_mask.is_none() && mask.is_some() {
        so.filter_mask = mask;
        so.filter_mask_enabled = true;
    }
    let region = (!layer_has_effects(layer)).then_some(layer.rect);
    rust.session.live.set_filter_preview(FilterPreview {
        layer_path: path,
        baseline,
    });
    Some(region)
}

/// Append `steps` as one Smart Filter to the active Smart Object (or, while a
/// filter is re-edited, replace it), putting an open preview back first;
/// `false` with `filter_error` set when refused. Lens Correction attaches its
/// own record, replacing an existing one.
pub(in crate::cxxqt_object) fn smart_commit(
    rust: &mut PictureViewRust,
    steps: &[FilterStep],
    gallery: bool,
) -> bool {
    let Some(path) = target(rust) else {
        return false;
    };
    let lens = !gallery && steps.len() == 1 && steps[0].kind == "lens-correction";
    let record = if lens {
        None
    } else {
        match record(rust, steps, gallery) {
            Some(record) => Some(record),
            None => return false,
        }
    };
    let editing = edit_index(rust, &path);
    let mask = selection_mask(rust);
    restore_preview(rust, &path);
    let Some(doc) = rust.session.doc.as_mut() else {
        return false;
    };
    let applied = match (record, editing) {
        (Some(record), Some(index)) => {
            pictura_render::replace_smart_filter_at(doc, &path, index, record)
        }
        (Some(record), None) => pictura_render::apply_smart_filter(doc, &path, record, mask),
        (None, _) => pictura_render::apply_lens_correction(doc, &path, &steps[0].slots),
    };
    if !applied {
        rust.filter_error = Some("the smart filter could not be applied".to_string());
    }
    applied
}

/// Route a commit of `kind` on a Smart Object: `None` off one (the destructive
/// path runs), else whether it applied, recording `last_filter`.
pub(in crate::cxxqt_object) fn commit_kind(
    rust: &mut PictureViewRust,
    kind: &str,
    params: &[f64],
) -> Option<bool> {
    if !on_smart_object(rust) {
        return None;
    }
    let applied = smart_commit(rust, &[step(kind, params)], false);
    if applied {
        rust.last_filter = Some((kind.to_string(), params.to_vec()));
    }
    Some(applied)
}

/// What the active Smart Object covers on the canvas, for a damage-bounded
/// history state.
fn influence(rust: &PictureViewRust) -> Option<PsdRect> {
    let path = rust.layer_target.single()?;
    pictura_render::damage::influence(rust.session.doc.as_ref()?, path)
}

/// [`commit_kind`] plus one `label` history state.
pub(in crate::cxxqt_object) fn route_commit(
    mut view: Pin<&mut PictureView>,
    kind: &str,
    params: &[f64],
    label: &str,
) -> Option<bool> {
    let before = influence(view.rust());
    let applied = {
        let mut rust = view.as_mut().rust_mut();
        commit_kind(&mut rust, kind, params)?
    };
    if applied {
        let after = influence(view.rust());
        view.as_mut().edit(label, |_| structural(before, after));
    }
    Some(applied)
}

/// A Filter Gallery stack on a Smart Object, as one "Filter Gallery" Smart
/// Filter: `None` off one, else whether the preview or commit applied.
pub(in crate::cxxqt_object) fn route_stack(
    mut view: Pin<&mut PictureView>,
    kinds: &[String],
    params: &[f64],
    commit: bool,
    label: &str,
) -> Option<bool> {
    if !on_smart_object(view.rust()) {
        return None;
    }
    let Some(steps) = stack_steps(kinds, params) else {
        view.as_mut().rust_mut().filter_error =
            Some("the gallery stack has an unknown filter or wrong parameters".to_string());
        return Some(false);
    };
    if commit {
        let before = influence(view.rust());
        let applied = {
            let mut rust = view.as_mut().rust_mut();
            smart_commit(&mut rust, &steps, true)
        };
        if applied {
            let after = influence(view.rust());
            view.as_mut().edit(label, |_| structural(before, after));
        }
        return Some(applied);
    }
    let region = {
        let mut rust = view.as_mut().rust_mut();
        smart_preview(&mut rust, &steps, true)
    };
    match region {
        Some(Some(rect)) => view.as_mut().refresh_region(rect),
        Some(None) => view.as_mut().recomposite(),
        None => return Some(false),
    }
    Some(true)
}

/// The kind's preview on a Smart Object: `None` off one, else as
/// [`smart_preview`].
pub(in crate::cxxqt_object) fn preview_kind(
    rust: &mut PictureViewRust,
    kind: &str,
    params: &[f64],
) -> Option<Option<Option<PsdRect>>> {
    on_smart_object(rust).then(|| smart_preview(rust, &[step(kind, params)], false))
}

#[cfg(test)]
mod tests;
