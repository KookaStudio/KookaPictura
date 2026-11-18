//! Apply and clear a smart object's image-stack mode.

use pictura_core::{Document, SmartObjectKind, StackMode};

use super::paths::{resolve_path, resolve_path_mut};

/// The active stack mode of the smart object at `path`, or `None` when the
/// path is not a smart object or the object renders normally.
pub fn smart_object_stack_mode(doc: &Document, path: &str) -> Option<StackMode> {
    resolve_path(doc, path)
        .and_then(|layer| layer.smart_object())
        .and_then(|so| so.stack_mode)
}

/// Whether the smart object at `path` can take `mode` from the menu: a mode
/// needs an embedded payload with at least two visible frames and must not be
/// the active one; `None` is offered only when a mode is active.
pub fn can_set_smart_object_stack_mode(
    doc: &Document,
    path: &str,
    mode: Option<StackMode>,
) -> bool {
    let Some(layer) = resolve_path(doc, path) else {
        return false;
    };
    if layer.is_group() || layer.has_adjustment_content() {
        return false;
    }
    let Some(so) = layer.smart_object() else {
        return false;
    };
    match mode {
        None => so.stack_mode.is_some(),
        Some(mode) => {
            so.kind == SmartObjectKind::Embedded
                && so.stack_mode != Some(mode)
                && crate::layer_source::stack_frame_count(so).is_some_and(|count| count >= 2)
        }
    }
}

/// Set or clear the stack mode of the smart object at `path`.
///
/// `Some(mode)` requires an embedded payload with at least two visible frames;
/// `None` is allowed on any resolved smart-object layer. Setting the active
/// mode is a successful no-op. Returns `false` without mutating for an
/// ineligible target.
pub fn set_smart_object_stack_mode(
    doc: &mut Document,
    path: &str,
    mode: Option<StackMode>,
) -> bool {
    let Some(layer) = resolve_path(doc, path) else {
        return false;
    };
    if layer.is_group() || layer.has_adjustment_content() {
        return false;
    }
    let Some(so) = layer.smart_object() else {
        return false;
    };
    if so.stack_mode == mode {
        return true;
    }
    if mode.is_some()
        && (so.kind != SmartObjectKind::Embedded
            || !crate::layer_source::stack_frame_count(so).is_some_and(|count| count >= 2))
    {
        return false;
    }
    let Some(layer) = resolve_path_mut(doc, path) else {
        return false;
    };
    pictura_codec::edit_layer(layer, |parts| {
        parts.smart_object.as_mut().expect("checked").stack_mode = mode
    });
    true
}
