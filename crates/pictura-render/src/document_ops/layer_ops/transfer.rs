//! Copy a layer node from one open document into another: the Layers panel
//! drag onto another document's tab or canvas.

use pictura_core::Document;

use super::create::{insert_node, release_background};
use super::paths::resolve_path;
use crate::{document_bit_depth, document_color_mode};

/// Deep-copy the node at `path` in `src` — children, masks, effects, and every
/// attribute — into `dst` by the New Layer insertion rule relative to
/// `selection_path` (inside a selected group, else above the selected node,
/// else on top). The copy keeps its name and its document position; a
/// Background's copy is an ordinary, unlocked layer. Returns the new path in
/// `dst`, or `None` for an unknown path or a document of another color mode or
/// bit depth.
//
// ponytail: CS6 converts the copy into the destination's mode and depth; a
// mismatched pair is refused until a per-layer conversion exists.
pub fn copy_path_to_document(
    src: &Document,
    path: &str,
    dst: &mut Document,
    selection_path: &str,
) -> Option<String> {
    if document_color_mode(src) != document_color_mode(dst)
        || document_bit_depth(src) != document_bit_depth(dst)
    {
        return None;
    }
    let mut copy = resolve_path(src, path)?.clone();
    if !src.variables.definitions().is_empty()
        || !dst.variables.definitions().is_empty()
        || !dst.variables.protected_ids().is_empty()
    {
        super::create::clear_copied_layer_identity(&mut copy);
    }
    if copy.background {
        release_background(&mut copy);
    }
    let created = insert_node(dst, selection_path, copy);
    (!created.is_empty()).then_some(created)
}
