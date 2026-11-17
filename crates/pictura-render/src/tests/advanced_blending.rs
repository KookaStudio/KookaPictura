//! Advanced-blending compositor tests: `clbl`, `infx`, `tsly`, the mask-hide
//! switches and `brst` channel restrictions.

use super::*;
use pictura_core::{Knockout, LayerMask, TypeTool, VectorMask, VectorSubpath};

fn set(layer: &mut Layer, key: &str, value: f64) {
    if !crate::set_layer_style_value(layer, key, value) {
        assert_eq!(
            crate::layer_style_value(layer, key),
            Some(value),
            "{key} = {value}"
        );
    }
}

fn color_overlay(layer: &mut Layer, rgb: u32, opacity: f64) {
    set(layer, "colorOverlay.on", 1.0);
    set(layer, "colorOverlay.color", rgb as f64);
    set(layer, "colorOverlay.opacity", opacity);
    set(layer, "colorOverlay.mode", 0.0);
}

fn drop_shadow(layer: &mut Layer, angle: f64, distance: f64) {
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.useGlobalLight", 0.0);
    set(layer, "dropShadow.color", 0.0);
    set(layer, "dropShadow.opacity", 100.0);
    set(layer, "dropShadow.mode", 0.0);
    set(layer, "dropShadow.angle", angle);
    set(layer, "dropShadow.distance", distance);
    set(layer, "dropShadow.spread", 0.0);
    set(layer, "dropShadow.size", 0.0);
}

fn inner_shadow(layer: &mut Layer) {
    set(layer, "innerShadow.on", 1.0);
    set(layer, "innerShadow.color", 0.0);
    set(layer, "innerShadow.opacity", 100.0);
    set(layer, "innerShadow.mode", 0.0);
    set(layer, "innerShadow.angle", 120.0);
    set(layer, "innerShadow.distance", 2.0);
    set(layer, "innerShadow.choke", 0.0);
    set(layer, "innerShadow.size", 2.0);
}

fn add_half_mask(layer: &mut Layer) {
    let mask = half_mask_rect(layer.rect);
    layer.mask = Some(mask);
}

fn half_mask_rect(rect: PsdRect) -> LayerMask {
    let w = rect.width().max(0) as usize;
    let h = rect.height().max(0) as usize;
    let data: Vec<u8> = (0..w * h)
        .map(|i| if (i % w) * 2 < w { 255 } else { 0 })
        .collect();
    LayerMask {
        rect,
        default_color: 0,
        data: Some(data.into()),
        ..Default::default()
    }
}

fn left_half_vector(rect: PsdRect) -> VectorMask {
    let mid = (rect.left + rect.right) / 2;
    let point = |x: i32, y: i32| [x * 256, y * 256];
    VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            points: vec![
                point(rect.left, rect.top),
                point(mid, rect.top),
                point(mid, rect.bottom),
                point(rect.left, rect.bottom),
            ],
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn mode_doc(w: u32, h: u32, mode: ColorMode, layers: Vec<Layer>) -> Document {
    let mut d = Document::new(w, h, mode, BitDepth::Eight);
    d.layers = pictura_core::LayerStack::from_layers(layers);
    d
}

fn type_tool() -> TypeTool {
    TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "Hi".into(),
        bounds: [0, 0, 8, 8],
        text_desc: Vec::new().into(),
        warp_desc: Vec::new().into(),
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
        path: None,
    }
}

fn gray(w: u32, h: u32) -> Layer {
    solid(
        "gray",
        full(w, h),
        (128, 128, 128),
        255,
        BlendMode::Normal,
        255,
    )
}

fn square(name: &str, rgb: (u8, u8, u8), opacity: u8) -> Layer {
    solid(name, rect(4, 4, 8, 8), rgb, 255, BlendMode::Normal, opacity)
}

// ---- Transparency Shapes -------------------------------------------------

#[test]
fn tsly_off_color_overlay_fills_the_canvas() {
    let mut layer = square("square", (255, 0, 0), 255);
    layer.transparency_shapes = false;
    color_overlay(&mut layer, 0x00FF00, 100.0);
    let out = composite_rgba(&doc(12, 12, vec![layer]));
    assert_eq!(px(&out, 0, 0), [0, 255, 0, 255]);
    assert_eq!(px(&out, 11, 11), [0, 255, 0, 255]);
}

#[test]
fn tsly_on_is_byte_identical_to_content_shaping() {
    let mut layer = square("square", (255, 0, 0), 255);
    color_overlay(&mut layer, 0x00FF00, 100.0);
    let out = composite_rgba(&doc(12, 12, vec![layer]));
    assert_eq!(
        px(&out, 0, 0),
        [0, 0, 0, 0],
        "no overlay outside the square"
    );
    assert_eq!(px(&out, 5, 5), [0, 255, 0, 255], "overlay over the square");
}

#[test]
fn tsly_off_knockout_cuts_the_canvas() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (0, 0, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let middle = solid(
        "middle",
        full(12, 12),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let mut top = square("top", (255, 0, 0), 255);
    top.fill = 128;
    top.knockout = Knockout::Deep;
    top.transparency_shapes = false;
    let out = composite_rgba(&doc(12, 12, vec![backdrop.clone(), middle.clone(), top]));
    let mut shaped = square("top", (255, 0, 0), 255);
    shaped.fill = 128;
    shaped.knockout = Knockout::Deep;
    let shaped = composite_rgba(&doc(12, 12, vec![backdrop, middle, shaped]));
    assert_eq!(
        px(&out, 0, 0),
        [0, 0, 255, 255],
        "the whole canvas is punched to the base"
    );
    assert_eq!(
        px(&shaped, 0, 0),
        [0, 255, 0, 255],
        "the shaped knockout leaves the middle layer outside the square"
    );
}

#[test]
fn tsly_off_edge_effects_are_absent() {
    let mut with = square("square", (255, 0, 0), 255);
    with.transparency_shapes = false;
    drop_shadow(&mut with, 180.0, 3.0);
    set(&mut with, "outerGlow.on", 1.0);
    set(&mut with, "innerShadow.on", 1.0);
    set(&mut with, "stroke.on", 1.0);
    set(&mut with, "satin.on", 1.0);
    set(&mut with, "innerGlow.on", 1.0);
    set(&mut with, "bevel.on", 1.0);
    let mut plain = square("square", (255, 0, 0), 255);
    plain.transparency_shapes = false;
    let out = composite_rgba(&doc(12, 12, vec![with]));
    let plain = composite_rgba(&doc(12, 12, vec![plain]));
    assert_eq!(out.data, plain.data, "no edge effect contributes");
}

#[test]
fn tsly_off_overlay_still_follows_the_mask_role() {
    let mut layer = square("square", (255, 0, 0), 255);
    layer.transparency_shapes = false;
    add_half_mask(&mut layer);
    color_overlay(&mut layer, 0x00FF00, 100.0);
    let out = composite_rgba(&doc(12, 12, vec![layer.clone()]));
    assert_eq!(px(&out, 5, 5), [0, 255, 0, 255], "visible half");
    assert_eq!(px(&out, 7, 5), [0, 0, 0, 0], "masked half");
    assert_eq!(px(&out, 0, 0), [0, 0, 0, 0], "outside the mask box");

    layer.layer_mask_hides_effects = true;
    let out = composite_rgba(&doc(12, 12, vec![layer]));
    assert_eq!(px(&out, 5, 5), [0, 255, 0, 255], "output clip visible half");
    assert_eq!(px(&out, 7, 5), [0, 0, 0, 0], "output clip masked half");
}

// ---- Mask hides effects --------------------------------------------------

fn shadowed_layer(mask: bool) -> Layer {
    let mut layer = square("square", (255, 0, 0), 255);
    if mask {
        add_half_mask(&mut layer);
    }
    drop_shadow(&mut layer, 180.0, 3.0);
    layer
}

#[test]
fn lmgm_off_lets_shadow_spill_past_mask() {
    let out = composite_rgba(&doc(12, 12, vec![shadowed_layer(true)]));
    let mut plain = square("square", (255, 0, 0), 255);
    add_half_mask(&mut plain);
    let plain = composite_rgba(&doc(12, 12, vec![plain]));
    assert!(
        px(&out, 7, 5)[3] > px(&plain, 7, 5)[3],
        "the shadow adds alpha in the masked-out half"
    );
}

#[test]
fn lmgm_on_clips_shadow_to_mask() {
    let mut layer = shadowed_layer(true);
    layer.layer_mask_hides_effects = true;
    let out = composite_rgba(&doc(12, 12, vec![layer]));
    let no_shadow = {
        let mut layer = square("square", (255, 0, 0), 255);
        add_half_mask(&mut layer);
        layer
    };
    let plain = composite_rgba(&doc(12, 12, vec![no_shadow]));
    let off = composite_rgba(&doc(12, 12, vec![shadowed_layer(true)]));
    assert!(
        px(&off, 7, 5)[3] > px(&plain, 7, 5)[3],
        "the control shadow is present without the output clip"
    );
    assert_eq!(
        px(&out, 7, 5),
        px(&plain, 7, 5),
        "the mask hides the output"
    );
}

#[test]
fn vmgm_on_clips_effects_independently() {
    let mut layer = square("square", (255, 0, 0), 255);
    add_half_mask(&mut layer);
    layer.vector_mask = Some(left_half_vector(layer.rect));
    drop_shadow(&mut layer, 180.0, 3.0);
    let off = composite_rgba(&doc(12, 12, vec![layer.clone()]));
    layer.vector_mask_hides_effects = true;
    let on = composite_rgba(&doc(12, 12, vec![layer]));
    assert!(
        px(&on, 7, 5)[3] < px(&off, 7, 5)[3],
        "the vector mask clips the shadow output while the raster shapes the source"
    );
}

#[test]
fn disabled_mask_is_inert() {
    let mut masked = shadowed_layer(true);
    masked.mask.as_mut().expect("mask").disabled = true;
    masked.layer_mask_hides_effects = true;
    let plain = shadowed_layer(false);
    let masked = composite_rgba(&doc(12, 12, vec![masked]));
    let plain = composite_rgba(&doc(12, 12, vec![plain]));
    assert_eq!(masked.data, plain.data);
}

#[test]
fn lmgm_on_uses_the_whole_fill_source_box() {
    let mut d = doc(
        12,
        12,
        vec![solid(
            "backdrop",
            full(12, 12),
            (255, 255, 255),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    let path = crate::add_solid_fill(&mut d, "", [255, 0, 0, 128]);
    let layer = crate::resolve_path_mut(&mut d, &path).expect("fill layer");
    layer.fill = 128;
    layer.mask = Some(half_mask_rect(rect(0, 0, 12, 6)));
    layer.layer_mask_hides_effects = true;
    assert!(
        crate::layer_effects::fill_content_box(layer).is_none(),
        "the hiding mask does not narrow the source box"
    );
    layer.layer_mask_hides_effects = false;
    assert!(
        crate::layer_effects::fill_content_box(layer).is_some(),
        "the shaping mask narrows the source box"
    );

    let mut on = d.clone();
    {
        let layer = crate::resolve_path_mut(&mut on, &path).expect("fill layer");
        layer.layer_mask_hides_effects = true;
        drop_shadow(layer, 0.0, 3.0);
    }
    let on = composite_rgba(&on);
    let mut off = d;
    drop_shadow(
        crate::resolve_path_mut(&mut off, &path).expect("fill layer"),
        0.0,
        3.0,
    );
    let off = composite_rgba(&off);
    assert!(
        px(&on, 2, 5)[0] < px(&off, 2, 5)[0],
        "the whole-fill source lets the hidden half's shadow reach the visible half"
    );
}

// ---- Channel Include -----------------------------------------------------

fn restriction_doc(restrictions: Vec<u32>) -> Document {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let mut layer = square("square", (0, 255, 0), 255);
    layer.restricted_channels = restrictions;
    doc(12, 12, vec![backdrop, layer])
}

#[test]
fn excluded_channel_keeps_backdrop() {
    let out = composite_rgba(&restriction_doc(vec![0]));
    assert_eq!(px(&out, 5, 5), [255, 255, 0, 255]);
    assert_eq!(px(&out, 0, 0), [255, 0, 0, 255]);
}

#[test]
fn channel_restrictions_cover_effects() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let mut layer = square("square", (255, 255, 255), 255);
    color_overlay(&mut layer, 0xFF0000, 100.0);
    layer.restricted_channels = vec![0];
    let out = composite_rgba(&doc(12, 12, vec![backdrop.clone(), layer]));
    let plain = composite_rgba(&doc(
        12,
        12,
        vec![backdrop, {
            let mut layer = square("square", (255, 255, 255), 255);
            color_overlay(&mut layer, 0xFF0000, 100.0);
            layer
        }],
    ));
    assert_eq!(px(&out, 5, 5)[0], 0, "the excluded red is the backdrop's");
    assert_eq!(
        px(&plain, 5, 5)[0],
        255,
        "without the restriction red changes"
    );
}

#[test]
fn all_channels_excluded_removes_layer() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (0, 0, 255),
        128,
        BlendMode::Normal,
        255,
    );
    let mut layer = square("square", (255, 0, 0), 255);
    color_overlay(&mut layer, 0x00FF00, 100.0);
    layer.restricted_channels = vec![0, 1, 2];
    let out = composite_rgba(&doc(12, 12, vec![backdrop.clone(), layer]));
    let plain = composite_rgba(&doc(12, 12, vec![backdrop]));
    assert_eq!(out.data, plain.data);
}

#[test]
fn adjustment_leaves_excluded_channel_unadjusted() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let mut adjustment = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        adjustment: Some(AdjustmentData {
            key: *b"nvrt",
            data: Vec::new().into(),
        }),
        restricted_channels: vec![0],
        ..Default::default()
    });
    adjustment.rect = rect(0, 0, 0, 0);
    let out = composite_rgba(&doc(12, 12, vec![backdrop, adjustment]));
    assert_eq!(px(&out, 5, 5), [255, 255, 255, 255]);
}

#[test]
fn channel_restrictions_leave_a_transparent_pixel_unchanged() {
    let layer = {
        let mut layer = square("square", (255, 0, 0), 255);
        layer.restricted_channels = vec![0];
        layer
    };
    let out = composite_rgba(&doc(12, 12, vec![layer]));
    assert_eq!(px(&out, 0, 0), [0, 0, 0, 0]);
    assert_eq!(px(&out, 5, 5)[3], 255);
}

#[test]
fn channel_restrictions_follow_the_document_mode() {
    let gray_out = composite_rgba(&mode_doc(
        12,
        12,
        ColorMode::Grayscale,
        vec![
            solid(
                "backdrop",
                full(12, 12),
                (128, 128, 128),
                255,
                BlendMode::Normal,
                255,
            ),
            {
                let mut layer = square("square", (255, 255, 255), 255);
                layer.restricted_channels = vec![0];
                layer
            },
        ],
    ));
    assert_eq!(px(&gray_out, 5, 5), [128, 128, 128, 255]);

    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let layer = square("square", (255, 0, 0), 255);
    let plain = composite_rgba(&mode_doc(
        12,
        12,
        ColorMode::Cmyk,
        vec![backdrop.clone(), layer.clone()],
    ));
    let mut restricted = layer;
    restricted.restricted_channels = vec![0];
    let restricted = composite_rgba(&mode_doc(
        12,
        12,
        ColorMode::Cmyk,
        vec![backdrop, restricted],
    ));
    assert_eq!(
        plain.data, restricted.data,
        "non-RGB restrictions are inert"
    );

    for mode in [ColorMode::Bitmap, ColorMode::Duotone] {
        let backdrop = solid(
            "backdrop",
            full(12, 12),
            (0, 255, 0),
            255,
            BlendMode::Normal,
            255,
        );
        let layer = square("square", (255, 0, 0), 255);
        let plain = composite_rgba(&mode_doc(
            12,
            12,
            mode,
            vec![backdrop.clone(), layer.clone()],
        ));
        let mut restricted = layer;
        restricted.restricted_channels = vec![0];
        let restricted = composite_rgba(&mode_doc(12, 12, mode, vec![backdrop, restricted]));
        assert_eq!(
            plain.data, restricted.data,
            "{mode:?} restrictions are inert"
        );
    }
}

#[test]
fn channel_restrictions_on_a_clipping_base() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let mut base = solid(
        "base",
        rect(4, 4, 8, 8),
        (255, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    base.restricted_channels = vec![1];
    let mut child = solid(
        "child",
        rect(4, 4, 8, 8),
        (0, 0, 255),
        255,
        BlendMode::Normal,
        255,
    );
    child.clipping = true;
    let out = composite_rgba(&doc(12, 12, vec![backdrop, base, child]));
    assert_eq!(
        px(&out, 5, 5)[1],
        255,
        "the base's excluded green is the backdrop's"
    );
}

#[test]
fn channel_restrictions_on_a_clipped_member() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let base = solid(
        "base",
        rect(4, 4, 8, 8),
        (255, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let mut child = solid(
        "child",
        rect(4, 4, 8, 8),
        (0, 0, 255),
        255,
        BlendMode::Normal,
        255,
    );
    child.clipping = true;
    child.restricted_channels = vec![1];
    let out = composite_rgba(&doc(12, 12, vec![backdrop, base, child]));
    assert_eq!(px(&out, 5, 5)[1], 255, "the member keeps the base's green");
}

#[test]
fn channel_restrictions_apply_to_the_knockout_base() {
    let base = solid(
        "base",
        full(12, 12),
        (0, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let top = {
        let mut top = square("top", (255, 0, 0), 255);
        top.fill = 128;
        top.knockout = Knockout::Deep;
        top
    };
    let plain = composite_rgba(&doc(12, 12, vec![base.clone(), top.clone()]));
    let mut restricted_base = base;
    restricted_base.restricted_channels = vec![1];
    let restricted = composite_rgba(&doc(12, 12, vec![restricted_base, top]));
    assert!(
        px(&restricted, 5, 5)[1] < px(&plain, 5, 5)[1],
        "the base's excluded green is not revealed"
    );
}

// ---- Blend Clipped Layers as Group --------------------------------------

fn clipped_stack(base_flag: bool, member_blend: BlendMode, member_opacity: u8) -> Document {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let mut base = solid(
        "base",
        rect(2, 2, 10, 10),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        128,
    );
    base.blend_clipping = base_flag;
    let mut member = solid(
        "member",
        rect(2, 2, 10, 10),
        (0, 255, 0),
        255,
        member_blend,
        member_opacity,
    );
    member.clipping = true;
    doc(12, 12, vec![backdrop, base, member])
}

#[test]
fn clbl_off_keeps_member_mode_and_opacity() {
    let off = composite_rgba(&clipped_stack(false, BlendMode::Multiply, 102));
    let on = composite_rgba(&clipped_stack(true, BlendMode::Multiply, 102));
    let off_normal = composite_rgba(&clipped_stack(false, BlendMode::Normal, 102));
    assert_ne!(off.data, on.data, "the base opacity is not inherited");
    assert_ne!(off.data, off_normal.data, "the member's own mode is used");
}

#[test]
fn clbl_on_default_is_unchanged() {
    let on = composite_rgba(&clipped_stack(true, BlendMode::Normal, 255));
    let red = px(&on, 5, 5)[0];
    assert!(
        (100..150).contains(&red),
        "the default group fades the 50% base, got {red}"
    );
    assert_eq!(px(&on, 5, 5)[1], 255);
}

#[test]
fn clbl_off_still_bounds_the_member() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let mut base = solid(
        "base",
        rect(2, 2, 6, 6),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    base.blend_clipping = false;
    let mut member = solid(
        "member",
        rect(2, 2, 10, 10),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    member.clipping = true;
    let out = composite_rgba(&doc(12, 12, vec![backdrop, base, member]));
    assert_eq!(px(&out, 8, 8), [255, 255, 255, 255]);
}

// ---- Blend Interior Effects as Group ------------------------------------

fn interior_stack(flag: bool) -> Document {
    let mut layer = solid(
        "square",
        rect(4, 4, 8, 8),
        (255, 0, 0),
        255,
        BlendMode::Multiply,
        255,
    );
    color_overlay(&mut layer, 0x0000FF, 50.0);
    layer.blend_interior = flag;
    doc(12, 12, vec![gray(12, 12), layer])
}

#[test]
fn blend_interior_on_folds_interior_effects() {
    let on = composite_rgba(&interior_stack(true));
    let off = composite_rgba(&interior_stack(false));
    assert!(
        px(&on, 5, 5)[2] < px(&off, 5, 5)[2],
        "the overlay folds before the layer's Multiply mode"
    );
}

#[test]
fn blend_interior_off_uses_effect_modes() {
    let off = composite_rgba(&interior_stack(false));
    assert!(
        px(&off, 5, 5)[2] > px(&composite_rgba(&interior_stack(true)), 5, 5)[2],
        "the overlay's own mode applies after the layer mode"
    );
}

#[test]
fn blend_interior_without_fold_effects_is_byte_identical() {
    let mut layer = square("square", (255, 0, 0), 255);
    drop_shadow(&mut layer, 180.0, 3.0);
    layer.blend_interior = true;
    let on = composite_rgba(&doc(12, 12, vec![layer.clone()]));
    layer.blend_interior = false;
    let off = composite_rgba(&doc(12, 12, vec![layer]));
    assert_eq!(on.data, off.data);
}

#[test]
fn blend_interior_keeps_inner_shadow_after_the_group() {
    let mut base = solid(
        "square",
        rect(4, 4, 8, 8),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    color_overlay(&mut base, 0x0000FF, 50.0);
    base.blend_interior = true;
    let without = composite_rgba(&doc(12, 12, vec![base.clone()]));
    for key in ["innerShadow.on", "stroke.on", "bevel.on"] {
        let mut with = base.clone();
        set(&mut with, key, 1.0);
        let with = composite_rgba(&doc(12, 12, vec![with]));
        assert_ne!(with.data, without.data, "{key} stays after the group blend");
    }
    let mut shadow = base;
    inner_shadow(&mut shadow);
    let shadow = composite_rgba(&doc(12, 12, vec![shadow]));
    assert!(
        px(&shadow, 4, 4)[0] < px(&without, 4, 4)[0],
        "the inner shadow darkens the edge"
    );
}

#[test]
fn blend_interior_folds_a_fill_layer() {
    let mut d = doc(12, 12, vec![gray(12, 12)]);
    let path = crate::add_solid_fill(&mut d, "", [255, 0, 0, 255]);
    let layer = crate::resolve_path_mut(&mut d, &path).expect("fill layer");
    layer.blend = BlendMode::Multiply;
    color_overlay(layer, 0x0000FF, 50.0);
    layer.blend_interior = true;
    let on = composite_rgba(&d);
    let layer = crate::resolve_path_mut(&mut d, &path).expect("fill layer");
    layer.blend_interior = false;
    let off = composite_rgba(&d);
    assert!(
        px(&on, 5, 5)[2] < px(&off, 5, 5)[2],
        "the fill and overlay group before Multiply applies"
    );
}

#[test]
fn blend_interior_is_inert_on_groups_and_adjustments() {
    let mut grouped = group(
        "group",
        BlendMode::Normal,
        255,
        None,
        vec![square("child", (255, 0, 0), 255)],
    );
    grouped.blend_interior = true;
    let on = composite_rgba(&doc(12, 12, vec![grouped.clone()]));
    grouped.blend_interior = false;
    let off = composite_rgba(&doc(12, 12, vec![grouped]));
    assert_eq!(on.data, off.data);

    let mut adjustment = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        adjustment: Some(AdjustmentData {
            key: *b"nvrt",
            data: Vec::new().into(),
        }),
        blend_interior: true,
        ..Default::default()
    });
    let backdrop = gray(12, 12);
    let on = composite_rgba(&doc(12, 12, vec![backdrop.clone(), adjustment.clone()]));
    adjustment.blend_interior = false;
    let off = composite_rgba(&doc(12, 12, vec![backdrop, adjustment]));
    assert_eq!(on.data, off.data);
}

#[test]
fn blend_interior_applies_the_text_curve_once() {
    let mut ink = solid("ink", full(8, 8), (0, 0, 0), 0, BlendMode::Normal, 255);
    ink.channels_mut()[3].data = (0..64).map(|v| (v * 4) as u8).collect::<Vec<_>>().into();
    pictura_codec::edit_layer(ink.as_mut(), |parts| { parts.type_tool = Some(type_tool()) });
    color_overlay(&mut ink, 0x0000FF, 50.0);
    ink.blend_interior = true;
    let on = composite_rgba(&doc(8, 8, vec![ink.clone()]));
    pictura_codec::edit_layer(ink.as_mut(), |parts| { parts.type_tool = None });
    let plain = composite_rgba(&doc(8, 8, vec![ink]));
    assert!(
        on.data.iter().zip(plain.data.iter()).any(|(a, b)| a != b),
        "the grouped path still applies the text curve"
    );
}

// ---- Defaults and route integration -------------------------------------

#[test]
fn advanced_blending_defaults_are_byte_identical() {
    // A hand-computed default fixture; the existing clipping, effect and
    // knockout suites cover the default paths of those features.
    let d = doc(
        12,
        12,
        vec![
            solid(
                "backdrop",
                full(12, 12),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            square("square", (255, 0, 0), 255),
        ],
    );
    let out = composite_rgba(&d);
    assert_eq!(px(&out, 0, 0), [255, 255, 255, 255]);
    assert_eq!(px(&out, 5, 5), [255, 0, 0, 255]);
}

#[test]
fn fold_only_layer_uses_the_region_fast_path() {
    let mut layer = solid(
        "square",
        rect(4, 4, 8, 8),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    color_overlay(&mut layer, 0x00FF00, 100.0);
    let d = doc(
        12,
        200,
        vec![
            solid(
                "backdrop",
                full(12, 200),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    );
    let full = crate::composite::composite_whole(&d);
    let region = crate::composite::composite_rgba_region(&d, 4, 4, 4, 4);
    for y in 4..8u32 {
        for x in 4..8u32 {
            assert_eq!(px(&region, x - 4, y - 4), px(&full, x, y));
        }
    }
}

#[test]
fn layer_paint_bounds_covers_the_canvas_when_tsly_is_off() {
    let mut layer = square("square", (255, 0, 0), 255);
    color_overlay(&mut layer, 0x00FF00, 100.0);
    layer.transparency_shapes = false;
    let d = doc(12, 12, vec![layer]);
    assert_eq!(
        crate::layer_effects::paint_bounds(&d.layers[0], &d),
        Some(rect(0, 0, 12, 12))
    );
}

#[test]
fn group_knockout_paint_bounds_covers_the_canvas() {
    let mut grouped = group(
        "group",
        BlendMode::Normal,
        255,
        None,
        vec![square("child", (255, 0, 0), 255)],
    );
    grouped.knockout = Knockout::Deep;
    grouped.transparency_shapes = false;
    let d = doc(12, 12, vec![grouped]);
    assert_eq!(
        crate::layer_effects::paint_bounds(&d.layers[0], &d),
        Some(rect(0, 0, 12, 12))
    );
}

#[test]
fn hidden_tsly_off_knockout_is_a_no_op() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (0, 0, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let middle = solid(
        "middle",
        full(12, 12),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let mut top = square("top", (255, 0, 0), 255);
    top.fill = 128;
    top.knockout = Knockout::Deep;
    top.transparency_shapes = false;
    top.visible = false;
    let out = composite_rgba(&doc(12, 12, vec![backdrop.clone(), middle.clone(), top]));
    let plain = composite_rgba(&doc(12, 12, vec![backdrop, middle]));
    assert_eq!(out.data, plain.data);
}

#[test]
fn tsly_off_knockout_respects_the_layer_mask() {
    let backdrop = solid(
        "backdrop",
        full(12, 12),
        (0, 0, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let middle = solid(
        "middle",
        full(12, 12),
        (0, 255, 0),
        255,
        BlendMode::Normal,
        255,
    );
    let mut top = square("top", (255, 0, 0), 255);
    top.fill = 128;
    top.knockout = Knockout::Deep;
    top.transparency_shapes = false;
    add_half_mask(&mut top);
    let out = composite_rgba(&doc(12, 12, vec![backdrop, middle, top]));
    assert_eq!(
        px(&out, 7, 5),
        [0, 255, 0, 255],
        "the mask keeps the hidden half out of the knockout cover"
    );
}
