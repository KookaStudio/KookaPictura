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
use crate::cxxqt_object::edit::Edit;

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

/// Preview `steps` as a Smart Filter on top of the active Smart Object's
/// chain: the layer is kept whole as the baseline, the record (and a mask from
/// the selection) is pushed onto the typed view only, and the compositor's
/// chain key re-renders it. Cancel restores the layer. Returns the region to
/// refresh; `None` with `filter_error` set when refused.
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
    let mask = selection_mask(rust);
    restore_preview(rust, &path);
    let layer = pictura_render::resolve_path_mut(rust.session.doc.as_mut()?, &path)?;
    let baseline = PreviewBaseline::Layer(Box::new(layer.clone()));
    let so = layer.smart_object.as_mut()?;
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

/// Append `steps` as one Smart Filter to the active Smart Object, putting an
/// open preview back first; `false` with `filter_error` set when refused. Lens
/// Correction attaches its own record, replacing an existing one.
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
    let mask = selection_mask(rust);
    restore_preview(rust, &path);
    let Some(doc) = rust.session.doc.as_mut() else {
        return false;
    };
    let applied = match record {
        Some(record) => pictura_render::apply_smart_filter(doc, &path, record, mask),
        None => pictura_render::apply_lens_correction(doc, &path, &steps[0].slots),
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

/// [`commit_kind`] plus one `label` history state.
pub(in crate::cxxqt_object) fn route_commit(
    mut view: Pin<&mut PictureView>,
    kind: &str,
    params: &[f64],
    label: &str,
) -> Option<bool> {
    let applied = {
        let mut rust = view.as_mut().rust_mut();
        commit_kind(&mut rust, kind, params)?
    };
    if applied {
        view.as_mut().edit(label, |_| Edit::Structural);
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
        let applied = {
            let mut rust = view.as_mut().rust_mut();
            smart_commit(&mut rust, &steps, true)
        };
        if applied {
            view.as_mut().edit(label, |_| Edit::Structural);
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
