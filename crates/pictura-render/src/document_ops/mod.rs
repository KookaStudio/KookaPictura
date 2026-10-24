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
pub use depth::{convert_depth, convert_depth_exposure_gamma};
pub use layer_ops::{
    add_gradient_fill, add_group, add_group_full, add_group_in, add_layer, add_layer_full,
    add_layer_in, add_layer_mask, add_layer_mask_converting_background, add_raster_layer_from_rgba,
    add_shape_layer, add_solid_fill, add_vector_mask, align_layers, any_effects_visible,
    apply_layer_mask, apply_visibility, arrange_path, arrange_path_index, background_from_layer,
    can_align, can_arrange_path, can_background_from_layer, can_convert_smart_object_to_layers,
    can_convert_to_smart_object, can_create_clipping_mask, can_delete_paths, can_distribute,
    can_edit_smart_object_contents, can_lift_selection, can_merge_scope, can_merge_target,
    can_move_path, can_move_path_to, can_new_smart_object_via_copy, can_rasterize_smart_object,
    can_release_clipping_mask, can_replace_smart_object_contents, can_reset_smart_object_transform,
    clear_layer, clear_layer_style, content_rect, convert_for_smart_filters,
    convert_smart_object_to_layers, convert_to_smart_object, copy_layer, copy_layer_style,
    copy_merged, copy_path_to_document, copy_shape_attributes, coverage_bounds,
    create_clipping_mask, delete_hidden_layers, delete_layer_mask, delete_paths,
    delete_paths_report, delete_vector_mask, distribute_layers, duplicate_layer, duplicate_paths,
    flatten, flatten_rows, group_layer, group_paths, has_forced_locks, has_layer_mask,
    has_layer_style, has_vector_mask, invert_layer_mask, is_background, is_fill_content_layer,
    is_shape_layer, is_visible_in_panel, layer_content_bounds, layer_from_background,
    layer_live_shape, layer_mask_disabled, layer_mask_linked, layer_mask_refinement,
    layer_shape_paths, layer_style_effect_names, layer_style_pattern_names, layer_style_value,
    layer_via_copy, layer_via_cut, lift_selection, lock_group_layers, mask_document,
    mask_document_coverage, merge_lifted, merge_scope, move_mask_selection_content, move_path,
    move_path_to, move_path_to_result, move_selection_content, neutral_color,
    new_smart_object_via_copy, next_layer_name, open_as_smart_object, parent_path, paste_clip,
    paste_layer_style, paste_shape_attributes, perspective_crop, perspective_crop_refusal,
    perspective_crop_size, perspective_crop_sized, place_smart_object, rasterize_all_layers,
    rasterize_fill_content, rasterize_shape, rasterize_smart_object, rasterize_vector_mask,
    release_clipping_mask, rename_path, replace_smart_object_contents,
    reset_smart_object_transform, resize_shape, resolve_path, resolve_path_mut, reverse_paths,
    reverse_paths_result, scale_layer_effects, select_similar, set_all_effects_visible,
    set_blend_paths, set_color_paths, set_document_layer_style_value, set_fill_paths,
    set_layer_live_shape, set_layer_mask_density, set_layer_mask_enabled, set_layer_mask_feather,
    set_layer_mask_linked, set_layer_shape_paths, set_layer_style_value, set_lock_paths,
    set_opacity_paths, set_shape_fill, set_shape_stroke, set_vector_mask_density,
    set_vector_mask_enabled, set_vector_mask_feather, set_vector_mask_linked, set_visible_paths,
    shape_bounds, shape_coverage, shape_fill, shape_fill_color, shape_layer_rect, shape_stroke,
    smart_object_source_bytes, stamp_scope, sync_vector_mask_block, transform_layer,
    transform_layer_quad, transform_layer_quad_with, transform_layer_warp, transform_layer_with,
    trim_to_content, ungroup_layer, ungroup_paths, untrim_to, vector_mask_disabled,
    vector_mask_linked, vector_mask_refinement, write_mask_back, AlignEdge, Arrange, Clip,
    Interpolation, LayerMaskKind, LayerStyle, LayerTransform, MaskDocument, MergeError,
    MergeOutcome, MergeScope, NewLayerSpec, PasteMode, ShapeAttributes, ShapeStroke, StampScope,
    VectorMaskKind, MASK_FLAG_LINKED, VECTOR_MASK_FLAG_DISABLED, VECTOR_MASK_FLAG_INVERT,
    VECTOR_MASK_FLAG_NOT_LINKED,
};
pub(crate) use layer_ops::{insert_node, map_shape_layer};
pub use matting::{defringe, remove_matte, MatteBackground, MattingError};
pub use mode::{
    can_convert_depth, can_convert_mode, convert_bit_depth, convert_mode, convert_to_bitmap,
    convert_to_indexed, document_bit_depth, document_color_mode, indexed_exact_available,
    save_view, BitmapMethod,
};
pub use orient::{flip_document, rotate_document};
pub use pictura_raw::{
    apply_pictura_raw, apply_pictura_raw_destructive, can_apply_pictura_raw_destructive,
    preview_pictura_raw,
};
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

/// Install a layer-less document's merged composite as a bottom Background.
///
/// A geometry operation calls this before mutating so it transforms real
/// pixels instead of leaving the merged composite stale or blank. Does nothing
/// when the document already has layers, or when the composite does not match
/// the document size (there are then no reliable source pixels).
pub(crate) fn materialize_merged_composite(doc: &mut pictura_core::Document) {
    if !doc.layers.is_empty()
        || !doc.merged_composite_present
        || doc.width == 0
        || doc.height == 0
        || doc.composite.width != doc.width
        || doc.composite.height != doc.height
    {
        return;
    }
    let plane = doc.width as usize * doc.height as usize;
    let colors = doc.mode.color_channels() as usize;
    let planes = doc.composite.channels as usize;
    if plane == 0 || colors == 0 || colors > planes || doc.composite.data.len() < plane * planes {
        return;
    }
    let mut channels: Vec<pictura_core::Channel> = (0..colors)
        .map(|c| pictura_core::Channel {
            id: c as i16,
            data: doc.composite.data[c * plane..(c + 1) * plane]
                .to_vec()
                .into(),
        })
        .collect();
    let alpha = if planes > colors {
        Some(doc.composite.data[colors * plane..(colors + 1) * plane].to_vec())
    } else {
        doc.channels.first().map(|channel| {
            if channel.data.len() >= plane {
                channel.data[..plane].to_vec()
            } else {
                vec![255; plane]
            }
        })
    };
    channels.push(pictura_core::Channel {
        id: -1,
        data: alpha.unwrap_or_else(|| vec![255; plane]).into(),
    });
    doc.layers = pictura_core::LayerStack::from_layers(vec![pictura_core::Layer {
        name: "Background".to_string(),
        rect: pictura_core::PsdRect {
            top: 0,
            left: 0,
            bottom: doc.height as i32,
            right: doc.width as i32,
        },
        channels,
        background: true,
        ..Default::default()
    }]);
}

/// Visit each layer depth-first, groups before their children.
pub(crate) fn for_each_layer(
    layers: &mut pictura_core::LayerStack,
    f: &mut impl FnMut(&mut pictura_core::Layer),
) {
    for layer in layers {
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

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer};

    type GeometryOp = Box<dyn Fn(&mut Document)>;

    fn layered() -> Document {
        let rgba = vec![200u8; 4 * 4 * 4];
        let mut doc = Document::from_rgba("base", 4, 4, &rgba);
        doc.layers[0].background = true;
        doc.composite = crate::composite_rgba(&doc);
        doc
    }

    fn merged_only() -> Document {
        let n = 4 * 4;
        let mut data = vec![0u8; n * 4];
        for i in 0..n {
            data[i] = (i * 11) as u8;
            data[n + i] = (i * 7) as u8;
            data[2 * n + i] = (i * 3) as u8;
            data[3 * n + i] = (40 + i * 13) as u8;
        }
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.composite = PixelBuffer {
            width: 4,
            height: 4,
            channels: 4,
            data: data.into(),
        };
        doc.merged_composite_present = true;
        doc
    }

    #[test]
    fn a_geometry_op_leaves_the_composite_untouched() {
        let quad = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        let ops: Vec<(&str, GeometryOp)> = vec![
            (
                "resize",
                Box::new(|d| {
                    resize_document(d, 8, 8, pictura_ops::Resample::Nearest).unwrap();
                }),
            ),
            (
                "canvas",
                Box::new(|d| {
                    resize_canvas_document(d, 6, 6, pictura_ops::Anchor::Center).unwrap();
                }),
            ),
            ("crop", Box::new(|d| assert!(crop_document(d, 1, 1, 2, 2)))),
            (
                "crop-grow",
                Box::new(|d| assert!(crop_document_grow(d, -1, -1, 6, 6))),
            ),
            (
                "delete-cropped",
                Box::new(|d| {
                    let _ = delete_cropped_pixels(d);
                }),
            ),
            (
                "extend-background",
                Box::new(|d| {
                    resize_canvas_document(d, 6, 6, pictura_ops::Anchor::Center).unwrap();
                    let _ = extend_background(d, [0, 0, 0]);
                }),
            ),
            ("rotate", Box::new(|d| rotate_document(d, 1).unwrap())),
            ("flip", Box::new(|d| flip_document(d, true))),
            ("translate", Box::new(|d| assert!(translate_layer(d, 1, 1)))),
            (
                "rotate-in",
                Box::new(|d| assert!(rotate_document_in(d, 30.0, (2.0, 2.0)))),
            ),
            (
                "perspective",
                Box::new(move |d| assert!(perspective_crop(d, quad))),
            ),
        ];
        for (name, op) in ops {
            let mut doc = layered();
            let before = doc.composite.clone();
            op(&mut doc);
            assert_eq!(doc.composite, before, "{name} must not composite");
        }
    }

    #[test]
    fn materializing_a_loaded_merged_document_uses_its_alpha_channel() {
        let mut doc = merged_only();
        let plane = 16;
        doc.composite = PixelBuffer {
            width: 4,
            height: 4,
            channels: 3,
            data: doc.composite.data[..3 * plane].to_vec().into(),
        };
        doc.channels = vec![pictura_core::Channel {
            id: 0,
            data: vec![7u8; plane].into(),
        }];
        materialize_merged_composite(&mut doc);
        let alpha = doc.layers[0]
            .channels
            .iter()
            .find(|c| c.id == -1)
            .expect("alpha channel");
        assert_eq!(&alpha.data[..], &[7u8; 16]);
    }

    #[test]
    fn a_merged_only_document_keeps_its_pixels_through_geometry() {
        for name in ["resize", "crop", "rotate", "canvas"] {
            let mut doc = merged_only();
            match name {
                "resize" => {
                    resize_document(&mut doc, 8, 8, pictura_ops::Resample::Nearest).unwrap()
                }
                "crop" => assert!(crop_document(&mut doc, 1, 1, 2, 2)),
                "rotate" => rotate_document(&mut doc, 1).unwrap(),
                "canvas" => {
                    resize_canvas_document(&mut doc, 6, 6, pictura_ops::Anchor::Center).unwrap()
                }
                _ => unreachable!(),
            }
            assert_eq!(doc.layers.len(), 1, "{name}: materialized Background");
            let layer = &doc.layers[0];
            assert!(layer.background, "{name}: Background flag");
            let alpha = layer
                .channels
                .iter()
                .find(|c| c.id == -1)
                .unwrap_or_else(|| panic!("{name}: alpha channel"));
            assert!(
                alpha.data.iter().any(|&v| v < 255),
                "{name}: alpha values preserved"
            );
            doc.composite = crate::composite_rgba(&doc);
            assert!(
                doc.composite.data.iter().any(|&v| v != 0),
                "{name}: pixels survive"
            );
        }
    }
}
