use super::*;
use crate::composite_plan::counters;
use pictura_core::{BitDepth, Document, Layer, LayerMask, TypeSpec};

const W: u32 = 8;
const H: u32 = 160;

fn curves_layer() -> Layer {
    Layer {
        name: "curves".into(),
        rect: full(W, H),
        adjustment: Some(crate::encode_curves(
            &[(0, 0), (128, 140), (255, 255)],
            None,
            None,
            None,
        )),
        ..Default::default()
    }
}

fn lookup_layer() -> Layer {
    let cube = b"LUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n";
    Layer {
        name: "lookup".into(),
        rect: full(W, H),
        adjustment: Some(crate::encode_color_lookup(cube, "identity")),
        ..Default::default()
    }
}

fn smart_filtered_layer() -> Layer {
    let mut smart = pictura_core::SmartObject::default();
    smart.kind = pictura_core::SmartObjectKind::Embedded;
    smart.smart_filters = vec![pictura_core::SmartFilter {
        filter_id: pictura_codec::CAMERA_RAW_FILTER_ID,
        name: "f".into(),
        enabled: true,
        options: Vec::new().into(),
    }];
    smart.smart_filters_enabled = true;
    Layer {
        name: "smart".into(),
        rect: full(W, H),
        smart_object: Some(smart),
        ..Default::default()
    }
}

fn fill_with_effect_layer() -> Layer {
    let mut layer = Layer {
        name: "fill-effect".into(),
        rect: full(W, H),
        adjustment: Some(crate::encode_solid_color_fill([200, 100, 50])),
        ..Default::default()
    };
    crate::set_layer_style_value(&mut layer, "dropShadow.on", 1.0);
    layer
}

fn styled_layer() -> Layer {
    let mut layer = solid(
        "styled",
        full(W, H),
        (200, 100, 50),
        255,
        BlendMode::Multiply,
        255,
    );
    crate::set_layer_style_value(&mut layer, "dropShadow.on", 1.0);
    layer
}

fn base_layer() -> Layer {
    solid(
        "base",
        full(W, H),
        (10, 20, 30),
        255,
        BlendMode::Normal,
        255,
    )
}

fn reset(doc: &Document) {
    for layer in &doc.layers {
        counters::reset_plan(layer);
        counters::reset_mask_gate(layer);
        counters::reset_native_plane(layer);
        counters::reset_source_lookup(layer);
    }
}

fn mixed_doc() -> Document {
    let mut d = doc(
        W,
        H,
        vec![
            base_layer(),
            curves_layer(),
            lookup_layer(),
            fill_with_effect_layer(),
            smart_filtered_layer(),
            styled_layer(),
        ],
    );
    crate::add_type_layer(&mut d, "", &TypeSpec::new("Hi", "Liberation Sans", 24.0));
    let top = d.layers.len() - 1;
    d.layers[top].channels.clear();
    d
}

#[test]
fn banded_composite_resolves_each_layer_once() {
    let d = mixed_doc();
    reset(&d);
    let _ = crate::composite_rgba(&d);
    for layer in &d.layers {
        assert_eq!(counters::plan_count(layer), 1, "{} plan", layer.name);
        assert_eq!(
            counters::mask_gate_count(layer),
            1,
            "{} mask gate",
            layer.name
        );
        assert_eq!(
            counters::native_plane_count(layer),
            1,
            "{} native planes",
            layer.name
        );
    }
}

#[test]
fn adjustment_decodes_once_per_composite() {
    let d = mixed_doc();
    let _ = crate::composite_rgba(&d);
    for layer in &d.layers {
        if let Some(data) = layer.adjustment.as_ref() {
            assert_eq!(
                counters::adjustment_count(data),
                1,
                "{} decodes once",
                layer.name
            );
        }
    }
}

#[test]
fn sources_resolve_once_per_composite() {
    let _guard = crate::layer_source::cache_test_guard();
    let mut d = doc(
        W,
        H,
        vec![base_layer(), smart_filtered_layer(), styled_layer()],
    );
    crate::add_type_layer(&mut d, "", &TypeSpec::new("Hi", "Liberation Sans", 24.0));
    let top = d.layers.len() - 1;
    d.layers[top].channels.clear();
    reset(&d);
    crate::purge_source_cache();
    let _ = crate::composite_rgba(&d);
    for layer in &d.layers {
        if layer.type_tool.is_some() {
            assert_eq!(
                counters::source_lookup_count(layer),
                1,
                "the type raster is fetched once"
            );
        }
        if layer.smart_object.is_some() && crate::composite_route::smart_filters_apply(layer) {
            assert_eq!(
                counters::source_lookup_count(layer),
                1,
                "the smart-filtered source is fetched once"
            );
        }
        if layer.extra_block(b"lfx2").is_some() || layer.extra_block(b"lrFX").is_some() {
            assert_eq!(
                crate::layer_source::pass_build_count(layer, &d),
                1,
                "the effect pass is built once"
            );
        }
    }
}

#[test]
fn hidden_layer_resolves_nothing() {
    let visible = doc(W, H, vec![base_layer()]);
    let mut hidden = curves_layer();
    hidden.visible = false;
    let mut with_hidden = doc(W, H, vec![base_layer(), hidden]);
    crate::add_type_layer(
        &mut with_hidden,
        "",
        &TypeSpec::new("Hi", "Liberation Sans", 24.0),
    );
    let top = with_hidden.layers.len() - 1;
    with_hidden.layers[top].channels.clear();
    with_hidden.layers[top].visible = false;

    reset(&with_hidden);
    let a = crate::composite_rgba(&with_hidden);
    let b = crate::composite_rgba(&visible);
    assert_eq!(a.data, b.data, "a hidden layer changes nothing");
    assert_eq!(counters::plan_count(&with_hidden.layers[1]), 0);
    assert_eq!(counters::plan_count(&with_hidden.layers[top]), 0);
    assert_eq!(counters::plan_count(&with_hidden.layers[0]), 1);
}

#[test]
fn banded_reuses_resolved_mask_and_native_state() {
    let mut masked = solid(
        "masked",
        full(W, H),
        (0, 0, 255),
        255,
        BlendMode::Multiply,
        255,
    );
    let mut mask = LayerMask {
        rect: full(W, H),
        default_color: 0,
        data: Some(vec![128u8; (W * H) as usize].into()),
        ..Default::default()
    };
    mask.set_feather(4.0);
    masked.mask = Some(mask);
    let mut d = doc(W, H, vec![base_layer(), masked]);
    d.source_depth = Some(BitDepth::Sixteen);
    reset(&d);
    let banded = crate::composite_rgba(&d);
    assert_eq!(counters::mask_gate_count(&d.layers[1]), 1);
    assert_eq!(counters::native_plane_count(&d.layers[1]), 1);
    let whole = crate::composite::composite_whole(&d);
    assert_eq!(banded.data, whole.data);
}

#[test]
fn hidden_pattern_layer_does_not_disable_banding() {
    let pattern = || Layer {
        name: "pattern".into(),
        rect: full(W, H),
        adjustment: Some(pictura_core::AdjustmentData {
            key: *b"PtFl",
            data: Vec::new().into(),
        }),
        ..Default::default()
    };
    let visible = doc(W, H, vec![base_layer(), pattern()]);
    assert!(!crate::composite_region::banded(&visible, H));
    let mut hidden = pattern();
    hidden.visible = false;
    let hidden_doc = doc(W, H, vec![base_layer(), hidden]);
    assert!(crate::composite_region::banded(&hidden_doc, H));
}

#[test]
fn banded_composite_does_not_materialize_a_fractured_plane() {
    let mut d = doc(W, H, vec![base_layer(), styled_layer()]);
    let red = d.layers[1]
        .channels
        .iter_mut()
        .find(|channel| channel.id == 0)
        .expect("styled layer has a red channel");
    red.data.fracture(W as usize);
    red.data.write_band(0).expect("band 0 exists");

    pictura_core::reset_materializations();
    let _ = crate::composite_rgba(&d);
    assert_eq!(
        pictura_core::materializations(),
        0,
        "the band path reads no plane bytes"
    );
}

fn assert_edit_invalidates(d: &mut Document, index: usize, edit: impl FnOnce(&mut Layer)) {
    let before_key = crate::layer_source::key::content_key(&d.layers[index], d);
    let before = crate::composite_rgba(d);
    edit(&mut d.layers[index]);
    let after_key = crate::layer_source::key::content_key(&d.layers[index], d);
    assert_ne!(before_key, after_key, "the edit changes the content key");

    crate::purge_source_cache();
    let after = crate::composite_rgba(d);
    crate::purge_source_cache();
    let fresh = crate::composite_rgba(d);
    assert_ne!(before.data, after.data, "the edit changes the composite");
    assert_eq!(after.data, fresh.data, "warm equals purge-then-composite");
}

#[test]
fn content_key_invalidates_on_edit() {
    let _guard = crate::layer_source::cache_test_guard();

    let mut adjustment = doc(W, H, vec![base_layer(), curves_layer()]);
    assert_edit_invalidates(&mut adjustment, 1, |layer| {
        layer.adjustment.as_mut().expect("curves block").data =
            crate::encode_curves(&[(0, 0), (128, 90), (255, 255)], None, None, None).data;
    });

    let mut channel = doc(W, H, vec![base_layer()]);
    assert_edit_invalidates(&mut channel, 0, |layer| {
        let alpha = layer
            .channels
            .iter_mut()
            .find(|channel| channel.id == -1)
            .expect("alpha channel");
        alpha.data.fracture(W as usize);
        let band = alpha.data.write_band(0).expect("band 0 exists");
        band[0] = 0;
    });

    let mut mask = doc(W, H, vec![base_layer()]);
    assert_edit_invalidates(&mut mask, 0, |layer| {
        layer.mask = Some(LayerMask {
            rect: full(W, H),
            default_color: 0,
            data: Some(vec![64u8; (W * H) as usize].into()),
            ..Default::default()
        });
    });
}

#[test]
fn pass_through_group_base_clips_as_isolated() {
    let child = solid(
        "child",
        full(W, H),
        (50, 50, 50),
        255,
        BlendMode::Normal,
        255,
    );
    let blend_if = pictura_core::BlendIf {
        composite_source: (0, 30000),
        composite_dest: (0, 65535),
        channel_ranges: Vec::new(),
    };
    let mut pass = super::group(
        "base",
        BlendMode::PassThrough,
        255,
        None,
        vec![child.clone()],
    );
    pass.blend_if = Some(blend_if.clone());
    let mut normal = super::group("base", BlendMode::Normal, 255, None, vec![child]);
    normal.blend_if = Some(blend_if);
    let mut clipped = solid(
        "clipped",
        full(W, H),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    clipped.clipping = true;
    let pass_doc = doc(W, H, vec![base_layer(), pass, clipped.clone()]);
    let normal_doc = doc(W, H, vec![base_layer(), normal, clipped]);
    assert_eq!(
        crate::composite_rgba(&pass_doc).data,
        crate::composite_rgba(&normal_doc).data,
        "the clipped base composites as an isolated group"
    );
}

#[test]
fn clipping_group_reuses_base_content() {
    let mut clipped = solid(
        "clipped",
        full(W, H),
        (200, 100, 50),
        255,
        BlendMode::Normal,
        255,
    );
    clipped.clipping = true;
    let base = Layer {
        name: "fill".into(),
        rect: full(W, H),
        adjustment: Some(crate::encode_solid_color_fill([200, 100, 50])),
        ..Default::default()
    };
    let d = doc(W, H, vec![base_layer(), base, clipped]);
    reset(&d);
    let out = crate::composite_rgba(&d);
    assert_eq!(
        counters::adjustment_count(d.layers[1].adjustment.as_ref().expect("curves")),
        1,
        "the clipping base decodes once"
    );
    assert_eq!(counters::plan_count(&d.layers[1]), 1);
    crate::purge_source_cache();
    let again = crate::composite_rgba(&d);
    assert_eq!(out.data, again.data);

    let _serial = crate::composite_rows::serial_test_lock();
    crate::composite_blend::set_legacy_blend(true);
    let legacy = crate::composite_rgba(&d);
    crate::composite_blend::set_legacy_blend(false);
    assert_eq!(out.data, legacy.data, "the legacy kernel agrees");
}
