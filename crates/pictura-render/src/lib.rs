//! CPU compositing for Kooka Pictura.
//!
//! M2 scope: composite the document layer stack (opacity, raster masks, groups,
//! 27 blend modes) into an image. Spec: `docs/05-layers/blend-modes.md` and the
//! W3C Compositing and Blending Level 1 model. GPU acceleration is M2.5.
//!
//! Contract owned by task M2-A:
//!
//! ```ignore
//! /// 4-channel (R,G,B,A) planar, straight alpha, 8-bit, at document size.
//! pub fn composite_rgba(doc: &pictura_core::Document) -> pictura_core::PixelBuffer;
//! ```
//!
//! # Groups and pass-through
//!
//! A group whose blend mode is [`BlendMode::PassThrough`] is composited as a
//! true pass-through when **opacity is 255 and it has no mask**: its children
//! are composited directly onto the running canvas, so child blend modes see
//! the backdrop outside the group. Any other blend mode is an **isolated**
//! group: children composite onto a private transparent buffer that is then
//! blended as one layer with the group's mode, opacity, and mask.
//!
//! Approximation: a pass-through group with non-255 opacity or a mask falls
//! back to isolated compositing with `PassThrough` treated as `Normal`. The
//! reference applies the group opacity/mask to the pass-through result while
//! still letting children blend against the parent backdrop; that mixed model
//! is not implemented. Exact pass-through parity is limited to the 255/no-mask
//! case (see the test below).

mod blend;
mod canvas_damage;
pub use canvas_damage::CanvasDamage;
mod composite;
mod composite_clipping;
mod composite_knockout;
mod composite_rows;
mod view_pyramid;
pub(crate) use composite::{channel, mask_alpha, render_smart_source, sample};
pub use composite::{
    composite_rgba, decode_adjustment, encode_brightness_contrast, encode_hue_saturation,
    encode_invert, encode_photo_filter, encode_posterize, encode_solid_color_fill,
    encode_threshold,
};
pub use view_pyramid::{Planes, PyramidLevel, ViewPyramid, SMALLEST_SIDE, TILE};

mod composite_native;
pub use composite_native::{composite_native, refresh_native_composite};
pub use pictura_adjust::{
    replace_color_mask, replace_color_result, replace_color_shift_for, Adjustment, AutoKind,
    ExposureGamma, GradientFillParams, GradientKind, GradientMapParams, GradientStop, OpacityStop,
    PatternFillParams, ReplaceColorParams, ReplaceColorSample,
};

mod color_balance;
mod gradient_map;
pub use color_balance::encode_color_balance;
pub use gradient_map::{encode_gradient_map, gradient_map_dither};

mod channel_mixer;
pub use channel_mixer::encode_channel_mixer;

mod curves;
pub use curves::encode_curves;

mod adjustment_defaults;
pub use adjustment_defaults::{default_adjustment_block, ADJUSTMENT_DIALOG_KINDS};
mod adjustment_params;
pub use adjustment_params::{
    adjustment_editor, curve_points, reset_adjustment, set_adjustment_param, set_curve_points,
    AdjustmentEditor, AdjustmentParam, ParamKind,
};

mod selective_color;
pub use pictura_adjust::{SelectiveColorMethod, SelectiveRange};
pub use selective_color::encode_selective_color;

mod color_lookup;
pub use color_lookup::{
    color_lookup_preset_index, encode_color_lookup, identity_cube, set_color_lookup_preset,
};

mod color_lookup_presets;
pub use color_lookup_presets::{preset_cube, COLOR_LOOKUP_PRESETS};

mod vector_mask;

mod fill;
pub use fill::encode_gradient_fill;

pub mod layer_effects;
pub use layer_effects::{
    decode_bevel_emboss, decode_color_overlay, decode_drop_shadow, decode_gradient_overlay,
    decode_inner_glow, decode_inner_shadow, decode_outer_glow, decode_pattern_overlay,
    decode_satin, decode_stroke, BevelDirection, BevelEmboss, BevelHighlight, BevelShadow,
    BevelStyle, BevelTechnique, ColorOverlay, DropShadow, GlowSource, GlowTechnique,
    GradientOverlay, InnerGlow, InnerShadow, OuterGlow, PatternOverlay, Satin, Stroke, StrokeFill,
    StrokePosition,
};

pub mod gpu;
pub use gpu::{
    composite_active, composite_gpu, composite_gpu_or_cpu, composite_region_active, gpu_available,
    Backend, GpuError,
};

mod filter;
pub use filter::{
    apply_adjustment_region, apply_filter, apply_filter_region, filter_preserves_opacity,
    preview_apron,
};

mod smart_filter;
pub use smart_filter::{apply_smart_filter_chain, decode_smart_filter, SmartFilterOp};

pub mod locks;

mod filter_proxy;
pub use filter_proxy::FilterProxy;

mod gpu_filter;
pub use gpu_filter::{apply_filter_active, filter_gpu_available};

pub mod document_ops;
pub use document_ops::{
    add_gradient_fill, add_group, add_group_full, add_group_in, add_layer, add_layer_full,
    add_layer_in, add_layer_mask, add_layer_mask_converting_background, add_raster_layer_from_rgba,
    add_shape_layer, add_slice, add_smart_filter, add_solid_fill, add_vector_mask, align_layers,
    any_effects_visible, apply_layer_mask, apply_pictura_raw, apply_visibility, arrange_path,
    background_from_layer, can_add_smart_filter, can_align, can_arrange_path, can_convert_depth,
    can_convert_mode, can_convert_smart_object_to_layers, can_convert_to_smart_object,
    can_create_clipping_mask, can_distribute, can_edit_smart_object_contents, can_lift_selection,
    can_merge_scope, can_merge_target, can_move_path_to, can_new_smart_object_via_copy,
    can_rasterize_smart_object, can_release_clipping_mask, can_replace_smart_object_contents,
    can_reset_smart_object_transform, clear_layer, clear_layer_style, clear_smart_filters,
    convert_bit_depth, convert_depth_exposure_gamma, convert_for_smart_filters, convert_mode,
    convert_smart_object_to_layers, convert_to_bitmap, convert_to_indexed, convert_to_smart_object,
    copy_layer, copy_layer_style, copy_merged, copy_path_to_document, copy_shape_attributes,
    coverage_bounds, create_clipping_mask, crop_document, crop_document_grow, defringe,
    delete_cropped_pixels, delete_hidden_layers, delete_layer_mask, delete_paths,
    delete_smart_filter, delete_vector_mask, distribute_layers, document_bit_depth,
    document_color_mode, duplicate_layer, duplicate_paths, extend_background, flatten,
    flatten_rows, flip_document, group_layer, group_paths, has_forced_locks, has_layer_mask,
    has_layer_style, has_vector_mask, identity_mesh, indexed_exact_available, is_background,
    is_fill_content_layer, is_shape_layer, is_visible_in_panel, layer_from_background,
    layer_live_shape, layer_mask_disabled, layer_mask_linked, layer_shape_paths,
    layer_style_effect_names, layer_style_pattern_names, layer_style_value, layer_via_copy,
    layer_via_cut, lift_selection, lock_group_layers, mask_document, merge_lifted, merge_scope,
    move_path, move_path_to, move_selection_content, neutral_color, new_smart_object_via_copy,
    next_layer_name, open_as_smart_object, parent_path, paste_clip, paste_layer_style,
    paste_shape_attributes, perspective_crop, perspective_crop_refusal, perspective_crop_size,
    place_smart_object, rasterize_all_layers, rasterize_fill_content, rasterize_shape,
    rasterize_smart_object, rasterize_vector_mask, release_clipping_mask, remove_matte,
    remove_slice, rename_path, reorder_smart_filters, replace_smart_object_contents,
    reset_smart_object_transform, resize_canvas_document, resize_document, resize_shape,
    resolve_path, resolve_path_mut, resolve_slices, reverse_paths, rotate_document,
    rotate_document_in, rotate_document_in_offset, save_view, scale_layer_effects, select_similar,
    set_all_effects_visible, set_blend_paths, set_color_paths, set_document_layer_style_value,
    set_fill_paths, set_layer_live_shape, set_layer_mask_enabled, set_layer_mask_linked,
    set_layer_shape_paths, set_layer_style_value, set_lock_paths, set_opacity_paths,
    set_shape_fill, set_shape_stroke, set_slice, set_vector_mask_enabled, set_vector_mask_linked,
    set_visible_paths, shape_bounds, shape_coverage, shape_fill, shape_fill_color, shape_stroke,
    smart_object_source_bytes, stamp_scope, style_mesh, transform_layer, transform_layer_quad,
    transform_layer_warp, translate_layer, translate_layer_active, translate_layer_index,
    translate_layer_path, translate_layer_rect, trim_to_content, ungroup_layer, ungroup_paths,
    vector_mask_disabled, vector_mask_linked, write_mask_back, AlignEdge, Arrange, BitmapMethod,
    Clip, LayerMaskKind, LayerStyle, LayerTransform, MaskDocument, MatteBackground, MattingError,
    MergeError, MergeOutcome, MergeScope, NewLayerSpec, PasteMode, ShapeAttributes, ShapeStroke,
    Slice, StampScope, VectorMaskKind, WarpMesh, WarpParams, WarpStyle, MASK_FLAG_LINKED,
    VECTOR_MASK_FLAG_DISABLED, VECTOR_MASK_FLAG_INVERT, VECTOR_MASK_FLAG_NOT_LINKED,
};

mod text_render;
pub use text_render::{materialize_text_rgba, render_text_layer, BundledRasterizer, BundledText};
mod fonts;
pub use fonts::{font_family, font_postscript_name, register_font};
mod type_caret;
pub use type_caret::type_caret_stops;
mod type_layer;
pub use type_layer::{
    add_type_layer, apply_type_style, render_type, replace_type_layer, type_layer_at,
    type_layer_spec, type_mask, type_placement, TypePlacement,
};

pub use pictura_ops::{Anchor, Resample};

#[cfg(test)]
use composite::blend;
#[cfg(test)]
use composite::blend_if_factor;

#[cfg(test)]
mod tests;
