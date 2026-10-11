//! Layer creation and grouping at document scope (M37).
//!
//! Pure functions over `&mut Document`, following the bottom-first convention of
//! `Document::layers` (index 0 is the bottom of the stack).

mod align;
mod clipboard;
mod clipping;
mod create;
mod layer_masks;
mod layer_style;
mod mask_edit;
mod merge;
#[cfg(test)]
mod merge_tests;
mod move_content;
mod paths;
mod perspective_crop;
mod properties;
mod rasterize;
mod shape_layer;
mod shape_style;
mod smart_object;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_clipboard;
#[cfg(test)]
mod tests_perspective_crop;
#[cfg(test)]
mod tests_transfer;
#[cfg(test)]
mod tests_via;
mod transfer;
mod transform;
mod transform_native;
#[cfg(test)]
mod transform_tests;
mod vector_masks;
mod via;
mod warp;
mod warp_styles;
#[cfg(test)]
mod warp_styles_tests;

pub use align::{align_layers, can_align, can_distribute, distribute_layers, AlignEdge};
pub use clipboard::{
    clear_layer, copy_layer, copy_merged, coverage_bounds, paste_clip, Clip, PasteMode,
};
pub use clipping::{
    can_create_clipping_mask, can_release_clipping_mask, create_clipping_mask,
    release_clipping_mask,
};
pub(crate) use create::insert_node;
pub use create::{
    add_gradient_fill, add_group, add_group_full, add_group_in, add_layer, add_layer_full,
    add_layer_in, add_raster_layer_from_rgba, add_solid_fill, background_from_layer,
    duplicate_layer, group_layer, layer_from_background, neutral_color, next_layer_name,
    ungroup_layer, NewLayerSpec,
};
pub use layer_masks::{
    add_layer_mask, add_layer_mask_converting_background, apply_layer_mask, delete_layer_mask,
    has_layer_mask, layer_mask_disabled, layer_mask_linked, set_layer_mask_enabled,
    set_layer_mask_linked, LayerMaskKind, MASK_FLAG_LINKED,
};
pub use layer_style::{
    any_effects_visible, clear_layer_style, copy_layer_style, has_layer_style,
    layer_style_effect_names, layer_style_pattern_names, layer_style_value, paste_layer_style,
    scale_layer_effects, set_all_effects_visible, set_document_layer_style_value,
    set_layer_style_value, LayerStyle,
};
pub use mask_edit::{mask_document, write_mask_back, MaskDocument};
pub use merge::{
    can_merge_scope, can_merge_target, flatten, is_visible_in_panel, merge_scope, stamp_scope,
    MergeError, MergeOutcome, MergeScope, StampScope,
};
pub use move_content::{
    can_lift_selection, lift_selection, merge_lifted, move_selection_content, trim_to_content,
};
pub use paths::{flatten_rows, is_background, parent_path, resolve_path, resolve_path_mut};
pub use perspective_crop::{perspective_crop, perspective_crop_refusal, perspective_crop_size};
pub use properties::{
    apply_visibility, arrange_path, can_arrange_path, can_move_path_to, delete_hidden_layers,
    delete_paths, duplicate_paths, group_paths, lock_group_layers, move_path, move_path_to,
    rename_path, reverse_paths, select_similar, set_blend_paths, set_color_paths, set_fill_paths,
    set_lock_paths, set_opacity_paths, set_visible_paths, ungroup_paths, Arrange,
};
pub use rasterize::{
    is_fill_content_layer, rasterize_all_layers, rasterize_fill_content, rasterize_shape,
};
pub use shape_layer::{
    add_shape_layer, has_forced_locks, is_shape_layer, layer_live_shape, layer_shape_paths,
    set_layer_live_shape, set_layer_shape_paths, shape_coverage, shape_fill_color,
};
pub use shape_style::{
    copy_shape_attributes, paste_shape_attributes, resize_shape, set_shape_fill, set_shape_stroke,
    shape_bounds, shape_fill, shape_stroke, ShapeAttributes, ShapeStroke,
};
pub use smart_object::{
    can_convert_smart_object_to_layers, can_convert_to_smart_object,
    can_edit_smart_object_contents, can_new_smart_object_via_copy, can_rasterize_smart_object,
    can_replace_smart_object_contents, can_reset_smart_object_transform, convert_for_smart_filters,
    convert_smart_object_to_layers, convert_to_smart_object, new_smart_object_via_copy,
    open_as_smart_object, place_smart_object, rasterize_smart_object,
    replace_smart_object_contents, reset_smart_object_transform, smart_object_source_bytes,
};
pub use transfer::copy_path_to_document;
pub use transform::{transform_layer, transform_layer_quad, LayerTransform};
pub use vector_masks::{
    add_vector_mask, delete_vector_mask, has_vector_mask, rasterize_vector_mask,
    set_vector_mask_enabled, set_vector_mask_linked, sync_vector_mask_block, vector_mask_disabled,
    vector_mask_linked, VectorMaskKind, VECTOR_MASK_FLAG_DISABLED, VECTOR_MASK_FLAG_INVERT,
    VECTOR_MASK_FLAG_NOT_LINKED,
};
pub use via::{layer_via_copy, layer_via_cut};
pub use warp::{identity_mesh, transform_layer_warp, WarpMesh, WarpParams};
pub use warp_styles::{style_mesh, WarpStyle};
