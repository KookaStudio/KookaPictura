//! Document-scope wrappers around the M10 `pictura-ops` image operations
//! (`IMG-001`, `IMG-002`, `IMG-003`). See `docs/dev/m12-document-ops.md`.

mod canvas;
mod crop;
mod depth;
mod layer_ops;
mod matting;
mod mode;
mod native_store;
mod orient;
mod pictura_raw;
mod resize;
mod rotate_in;
mod slices;
mod smart_filters;

pub use canvas::{extend_background, resize_canvas_document};
pub use crop::{
    crop_document, crop_document_grow, delete_cropped_pixels, translate_layer,
    translate_layer_active, translate_layer_index, translate_layer_path, translate_layer_rect,
};
pub use depth::convert_depth_exposure_gamma;
pub(crate) use layer_ops::insert_node;
pub use layer_ops::{
    add_gradient_fill, add_group, add_group_full, add_group_in, add_layer, add_layer_full,
    add_layer_in, add_layer_mask, add_layer_mask_converting_background, add_raster_layer_from_rgba,
    add_shape_layer, add_solid_fill, add_vector_mask, align_layers, any_effects_visible,
    apply_layer_mask, apply_visibility, arrange_path, background_from_layer, can_align,
    can_arrange_path, can_convert_smart_object_to_layers, can_convert_to_smart_object,
    can_create_clipping_mask, can_distribute, can_edit_smart_object_contents, can_lift_selection,
    can_merge_scope, can_merge_target, can_move_path_to, can_new_smart_object_via_copy,
    can_rasterize_smart_object, can_release_clipping_mask, can_replace_smart_object_contents,
    can_reset_smart_object_transform, clear_layer, clear_layer_style, convert_for_smart_filters,
    convert_smart_object_to_layers, convert_to_smart_object, copy_layer, copy_layer_style,
    copy_merged, copy_path_to_document, copy_shape_attributes, coverage_bounds,
    create_clipping_mask, delete_hidden_layers, delete_layer_mask, delete_paths,
    delete_vector_mask, distribute_layers, duplicate_layer, duplicate_paths, flatten, flatten_rows,
    group_layer, group_paths, has_forced_locks, has_layer_mask, has_layer_style, has_vector_mask,
    identity_mesh, is_background, is_fill_content_layer, is_shape_layer, is_visible_in_panel,
    layer_from_background, layer_live_shape, layer_mask_disabled, layer_mask_linked,
    layer_shape_paths, layer_style_effect_names, layer_style_pattern_names, layer_style_value,
    layer_via_copy, layer_via_cut, lift_selection, lock_group_layers, mask_document, merge_lifted,
    merge_scope, move_path, move_path_to, move_selection_content, neutral_color,
    new_smart_object_via_copy, next_layer_name, open_as_smart_object, parent_path, paste_clip,
    paste_layer_style, paste_shape_attributes, perspective_crop, perspective_crop_refusal,
    perspective_crop_size, place_smart_object, rasterize_all_layers, rasterize_fill_content,
    rasterize_shape, rasterize_smart_object, rasterize_vector_mask, release_clipping_mask,
    rename_path, replace_smart_object_contents, reset_smart_object_transform, resize_shape,
    resolve_path, resolve_path_mut, reverse_paths, scale_layer_effects, select_similar,
    set_all_effects_visible, set_blend_paths, set_color_paths, set_document_layer_style_value,
    set_fill_paths, set_layer_live_shape, set_layer_mask_enabled, set_layer_mask_linked,
    set_layer_shape_paths, set_layer_style_value, set_lock_paths, set_opacity_paths,
    set_shape_fill, set_shape_stroke, set_vector_mask_enabled, set_vector_mask_linked,
    set_visible_paths, shape_bounds, shape_coverage, shape_fill, shape_fill_color, shape_stroke,
    smart_object_source_bytes, stamp_scope, style_mesh, sync_vector_mask_block, transform_layer,
    transform_layer_quad, transform_layer_warp, trim_to_content, ungroup_layer, ungroup_paths,
    vector_mask_disabled, vector_mask_linked, write_mask_back, AlignEdge, Arrange, Clip,
    LayerMaskKind, LayerStyle, LayerTransform, MaskDocument, MergeError, MergeOutcome, MergeScope,
    NewLayerSpec, PasteMode, ShapeAttributes, ShapeStroke, StampScope, VectorMaskKind, WarpMesh,
    WarpParams, WarpStyle, MASK_FLAG_LINKED, VECTOR_MASK_FLAG_DISABLED, VECTOR_MASK_FLAG_INVERT,
    VECTOR_MASK_FLAG_NOT_LINKED,
};
pub use matting::{defringe, remove_matte, MatteBackground, MattingError};
pub use mode::{
    can_convert_depth, can_convert_mode, convert_bit_depth, convert_mode, convert_to_bitmap,
    convert_to_indexed, document_bit_depth, document_color_mode, indexed_exact_available,
    save_view, BitmapMethod,
};
pub use orient::{flip_document, rotate_document};
pub use pictura_raw::apply_pictura_raw;
pub use resize::resize_document;
pub use rotate_in::{rotate_document_in, rotate_document_in_offset};
pub use slices::{add_slice, remove_slice, resolve_slices, set_slice, Slice};
pub use smart_filters::{
    add_smart_filter, can_add_smart_filter, clear_smart_filters, delete_smart_filter,
    reorder_smart_filters,
};

/// Reject a zero width or height with `InvalidParams`.
pub(crate) fn valid_size(width: u32, height: u32) -> Result<(), pictura_ops::OpsError> {
    if width == 0 || height == 0 {
        return Err(pictura_ops::OpsError::InvalidParams(
            "width and height must be >= 1".into(),
        ));
    }
    Ok(())
}

/// Rebuild the document composite from its (already transformed) layer tree.
pub(crate) fn recompute(doc: &mut pictura_core::Document) {
    doc.composite = crate::composite_rgba(doc);
}

/// Visit each layer depth-first, groups before their children.
pub(crate) fn for_each_layer(
    layers: &mut [pictura_core::Layer],
    f: &mut impl FnMut(&mut pictura_core::Layer),
) {
    for layer in layers.iter_mut() {
        f(layer);
        for_each_layer(&mut layer.children, f);
    }
}

/// Resample one planar 8-bit channel from `sw×sh` to `dw×dh`.
///
/// The caller validates the sizes, so a rejected `resize` (which would mean a
/// malformed plane) falls back to passing the input through untouched.
pub(crate) fn resample_plane(
    data: &[u8],
    sw: u32,
    sh: u32,
    dw: u32,
    dh: u32,
    resample: pictura_ops::Resample,
) -> Vec<u8> {
    let mut plane = pictura_core::PixelBuffer::new(sw, sh, 1);
    let n = plane.data.len().min(data.len());
    plane.data[..n].copy_from_slice(&data[..n]);
    pictura_ops::resize(&plane, dw, dh, resample)
        .map(|b| b.data.to_vec())
        .unwrap_or_else(|_| data.to_vec())
}
