use super::super::paths::resolve_path;
use super::*;
use crate::layer_effects::{
    decode_bevel_emboss, decode_color_overlay, decode_drop_shadow, decode_gradient_overlay,
    decode_inner_glow, decode_inner_shadow, decode_outer_glow, decode_satin, decode_stroke,
    BevelStyle, GlowSource, StrokeFill, StrokePosition,
};
use pictura_core::{BitDepth, Channel, ColorMode, PsdRect};

/// A 4x4 red square at `(4, 4)` over a white 12x12 backdrop; path `"1"`.
fn doc() -> Document {
    let layer = |name: &str, rect: PsdRect, rgb: [u8; 3]| {
        let n = (rect.width() * rect.height()) as usize;
        let mut channels: Vec<Channel> = (0..3)
            .map(|i| Channel {
                id: i,
                data: vec![rgb[i as usize]; n].into(),
            })
            .collect();
        channels.push(Channel {
            id: -1,
            data: vec![255; n].into(),
        });
        Layer {
            name: name.into(),
            rect,
            opacity: 255,
            fill: 255,
            visible: true,
            channels,
            ..Default::default()
        }
    };
    let rect = |top, left, bottom, right| PsdRect {
        top,
        left,
        bottom,
        right,
    };
    let mut d = Document::new(12, 12, ColorMode::Rgb, BitDepth::Eight);
    d.layers = pictura_core::LayerStack::from_layers(vec![
        layer("Backdrop", rect(0, 0, 12, 12), [255, 255, 255]),
        layer("Square", rect(4, 4, 8, 8), [255, 0, 0]),
    ]);
    d
}

fn square(doc: &mut Document) -> &mut Layer {
    resolve_path_mut(doc, "1").unwrap()
}

fn rgb(doc: &Document, x: usize, y: usize) -> [u8; 3] {
    let out = crate::composite_rgba(doc);
    let plane = (out.width * out.height) as usize;
    let i = y * out.width as usize + x;
    [out.data[i], out.data[plane + i], out.data[2 * plane + i]]
}

fn set(layer: &mut Layer, key: &str, value: f64) {
    assert!(set_layer_style_value(layer, key, value), "{key} = {value}");
}

#[test]
fn an_absent_effect_reads_cs6_defaults_and_is_off() {
    let mut d = doc();
    let layer = square(&mut d);
    assert_eq!(layer_style_value(layer, "dropShadow.on"), Some(0.0));
    assert_eq!(layer_style_value(layer, "dropShadow.opacity"), Some(75.0));
    assert_eq!(layer_style_value(layer, "dropShadow.mode"), Some(3.0));
    assert_eq!(
        layer_style_value(layer, "outerGlow.color"),
        Some(0xFFFFBE as f64)
    );
    assert_eq!(layer_style_value(layer, "fx.visible"), Some(1.0));
    assert_eq!(layer_style_value(layer, "dropShadow.nonsense"), None);
    assert_eq!(layer_style_value(layer, "nonsense.on"), None);
    assert!(!has_layer_style(layer));
}

#[test]
fn drop_shadow_authors_a_decodable_effect_that_renders() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.opacity", 100.0);
    set(layer, "dropShadow.useGlobalLight", 0.0);
    set(layer, "dropShadow.angle", 90.0);
    set(layer, "dropShadow.distance", 3.0);
    set(layer, "dropShadow.size", 0.0);
    set(layer, "dropShadow.color", 0x0000FF as f64);
    assert!(
        !set_layer_style_value(layer, "dropShadow.size", 0.0),
        "unchanged"
    );
    let shadow = decode_drop_shadow(layer).unwrap();
    assert!(shadow.enabled && shadow.present);
    assert_eq!(shadow.blend_mode, BlendMode::Multiply);
    assert_eq!(shadow.color, [0, 0, 255]);
    assert_eq!((shadow.opacity, shadow.angle_deg), (100.0, 90.0));
    assert_eq!((shadow.distance, shadow.size), (3.0, 0.0));
    // Light from 90° casts the shadow straight down: a hard blue band under
    // the square, and white above it.
    assert_eq!(rgb(&d, 5, 9), [0, 0, 255]);
    assert_eq!(rgb(&d, 5, 2), [255, 255, 255]);
    assert_eq!(rgb(&d, 5, 5), [255, 0, 0], "the content stays on top");
}

#[test]
fn every_effect_round_trips_through_its_decoder() {
    let mut d = doc();
    let layer = square(&mut d);
    for name in layer_style_effect_names() {
        if name != "patternOverlay" {
            set(layer, &format!("{name}.on"), 1.0);
        }
    }
    set(layer, "innerShadow.choke", 40.0);
    set(layer, "innerGlow.source", 0.0);
    set(layer, "outerGlow.technique", 1.0);
    set(layer, "bevel.style", 2.0);
    set(layer, "bevel.depth", 250.0);
    set(layer, "bevel.highlightMode", 0.0);
    set(layer, "satin.invert", 1.0);
    set(layer, "colorOverlay.color", 0x00FF00 as f64);
    set(layer, "gradientOverlay.to", 0xFF0000 as f64);
    set(layer, "gradientOverlay.style", 1.0);
    set(layer, "stroke.size", 7.0);
    set(layer, "stroke.position", 2.0);
    set(layer, "stroke.mode", 21.0);

    assert_eq!(decode_inner_shadow(layer).unwrap().choke, 40.0);
    assert_eq!(decode_inner_glow(layer).unwrap().source, GlowSource::Center);
    assert!(decode_outer_glow(layer).unwrap().enabled);
    let bevel = decode_bevel_emboss(layer).unwrap();
    assert_eq!((bevel.style, bevel.depth), (BevelStyle::Emboss, 250.0));
    assert_eq!(bevel.highlight.mode, BlendMode::Normal);
    assert!(decode_satin(layer).unwrap().invert);
    assert_eq!(decode_color_overlay(layer).unwrap().color, [0, 255, 0]);
    let gradient = decode_gradient_overlay(layer).unwrap();
    assert_eq!(gradient.stops.first().unwrap().color, [0, 0, 0]);
    assert_eq!(gradient.stops.last().unwrap().color, [255, 0, 0]);
    let stroke = decode_stroke(layer).unwrap();
    assert_eq!(stroke.size, 7);
    assert_eq!(stroke.position, StrokePosition::Center);
    assert_eq!(stroke.blend_mode, BlendMode::Subtract);
    assert_eq!(stroke.fill, StrokeFill::Solid([0, 0, 0]));
    assert_eq!(layer_style_value(layer, "stroke.mode"), Some(21.0));

    // Every newer blend mode reads back as itself through the string ids.
    for (i, mode) in BlendMode::LAYER_MODES.iter().enumerate() {
        set_layer_style_value(layer, "satin.mode", i as f64);
        assert_eq!(decode_satin(layer).unwrap().blend_mode, *mode);
    }
}

#[test]
fn a_pattern_overlay_needs_the_document_to_be_created() {
    let mut d = doc();
    let layer = square(&mut d);
    assert!(!set_layer_style_value(layer, "patternOverlay.on", 1.0));
    assert!(!has_layer_style(layer));
    assert_eq!(layer_style_value(layer, "patternOverlay.exists"), Some(0.0));
    set(layer, "satin.on", 1.0);
    set(layer, "satin.on", 0.0);
    assert_eq!(
        layer_style_value(layer, "satin.exists"),
        Some(1.0),
        "off, not gone"
    );
    assert!(
        !set_layer_style_value(layer, "satin.exists", 0.0),
        "read-only"
    );
}

#[test]
fn switching_the_last_effect_off_keeps_it_and_clear_removes_it() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "colorOverlay.on", 1.0);
    assert_eq!(rgb(&d, 5, 5), [255, 0, 0], "overlay defaults to red");
    let layer = square(&mut d);
    set(layer, "colorOverlay.color", 0x0000FF as f64);
    assert_eq!(rgb(&d, 5, 5), [0, 0, 255]);
    let layer = square(&mut d);
    set(layer, "colorOverlay.on", 0.0);
    assert!(has_layer_style(square(&mut d)));
    assert_eq!(rgb(&d, 5, 5), [255, 0, 0]);
    assert_eq!(clear_layer_style(&mut d, &["1"]), 1);
    assert!(!has_layer_style(square(&mut d)));
    assert_eq!(
        clear_layer_style(&mut d, &["1"]),
        0,
        "nothing left to clear"
    );
}

#[test]
fn hide_all_effects_turns_off_the_master_switch() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "colorOverlay.on", 1.0);
    set(layer, "colorOverlay.color", 0x0000FF as f64);
    assert!(any_effects_visible(&d, true));
    assert!(!any_effects_visible(&d, false));
    assert_eq!(set_all_effects_visible(&mut d, false), 1);
    assert_eq!(rgb(&d, 5, 5), [255, 0, 0], "the overlay is hidden");
    assert_eq!(
        layer_style_value(square(&mut d), "colorOverlay.on"),
        Some(1.0)
    );
    assert!(any_effects_visible(&d, false));
    assert_eq!(set_all_effects_visible(&mut d, false), 0);
    assert_eq!(set_all_effects_visible(&mut d, true), 1);
    assert_eq!(rgb(&d, 5, 5), [0, 0, 255], "and back");
}

#[test]
fn copy_paste_replaces_the_target_style_and_blending() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "stroke.on", 1.0);
    set(layer, "blending.fillOpacity", 0.0);
    let style = copy_layer_style(layer).unwrap();
    assert_eq!(paste_layer_style(&mut d, &["0"], &style), 1);
    let backdrop = resolve_path_mut(&mut d, "0").unwrap();
    assert_eq!(layer_style_value(backdrop, "stroke.on"), Some(1.0));
    assert_eq!(
        layer_style_value(backdrop, "blending.fillOpacity"),
        Some(0.0)
    );
    assert_eq!(
        paste_layer_style(&mut d, &["0"], &style),
        0,
        "already the same"
    );
}

#[test]
fn the_background_and_groups_carry_no_style() {
    let mut d = doc();
    resolve_path_mut(&mut d, "0").unwrap().background = true;
    let background = resolve_path_mut(&mut d, "0").unwrap();
    assert!(!set_layer_style_value(background, "dropShadow.on", 1.0));
    assert!(copy_layer_style(background).is_none());
    let style = copy_layer_style(square(&mut d)).unwrap();
    assert_eq!(paste_layer_style(&mut d, &["0"], &style), 0);
}

#[test]
fn scale_effects_scales_pixel_sizes_but_not_percentages() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.distance", 10.0);
    set(layer, "dropShadow.spread", 20.0);
    set(layer, "stroke.on", 1.0);
    set(layer, "stroke.size", 5.0);
    set(layer, "gradientOverlay.on", 1.0);
    assert_eq!(scale_layer_effects(&mut d, &["1"], 50.0), 1);
    let layer = square(&mut d);
    assert_eq!(layer_style_value(layer, "dropShadow.distance"), Some(5.0));
    assert_eq!(layer_style_value(layer, "dropShadow.size"), Some(2.5));
    assert_eq!(layer_style_value(layer, "dropShadow.spread"), Some(20.0));
    assert_eq!(layer_style_value(layer, "dropShadow.opacity"), Some(75.0));
    assert_eq!(
        layer_style_value(layer, "stroke.size"),
        Some(3.0),
        "whole pixels"
    );
    assert_eq!(
        layer_style_value(layer, "gradientOverlay.scale"),
        Some(50.0)
    );
    assert_eq!(scale_layer_effects(&mut d, &["1"], 0.0), 0);
}

#[test]
fn blending_options_write_the_layer_record() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "blending.mode", 3.0);
    set(layer, "blending.opacity", 50.0);
    set(layer, "blending.knockout", 2.0);
    set(layer, "blending.blendInterior", 1.0);
    assert_eq!(layer.blend, BlendMode::Multiply);
    assert_eq!(layer.opacity, 128);
    assert_eq!(layer.knockout, Knockout::Deep);
    assert!(layer.blend_interior);
    assert_eq!(layer_style_value(layer, "blending.opacity"), Some(50.0));
    assert!(!set_layer_style_value(layer, "blending.mode", 99.0));
}

#[test]
fn layer_style_copy_paste_preserves_advanced_blending() {
    let mut d = doc();
    let source = square(&mut d);
    source.transparency_shapes = false;
    source.layer_mask_hides_effects = true;
    source.vector_mask_hides_effects = true;
    source.restricted_channels = vec![0, 2];
    let style = copy_layer_style(source).expect("a pixel layer is styleable");
    assert_eq!(paste_layer_style(&mut d, &["0"], &style), 1);
    let target = resolve_path(&d, "0").unwrap();
    assert!(!target.transparency_shapes);
    assert!(target.layer_mask_hides_effects);
    assert!(target.vector_mask_hides_effects);
    assert_eq!(target.restricted_channels, vec![0, 2]);
}

#[test]
fn an_authored_style_survives_save_and_reopen() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "innerGlow.on", 1.0);
    set(layer, "innerGlow.size", 9.0);
    set(layer, "gradientOverlay.on", 1.0);
    set(layer, "gradientOverlay.from", 0x123456 as f64);
    set(layer, "gradientOverlay.mode", 25.0);
    let before = crate::composite_rgba(&d);
    let bytes = pictura_codec::write_psd(&d).unwrap();
    let reopened = pictura_codec::read_psd(&bytes).unwrap();
    let layer = resolve_path(&reopened, "1").unwrap();
    assert_eq!(layer_style_value(layer, "innerGlow.size"), Some(9.0));
    assert_eq!(
        layer_style_value(layer, "gradientOverlay.from"),
        Some(0x123456 as f64)
    );
    assert_eq!(layer_style_value(layer, "gradientOverlay.mode"), Some(25.0));
    assert_eq!(crate::composite_rgba(&reopened).data, before.data);
}

#[test]
fn interior_effects_stack_over_an_opaque_color_overlay() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "colorOverlay.on", 1.0);
    set(layer, "colorOverlay.color", 0x0000FF as f64);
    let overlay_only = crate::composite_rgba(&d);
    let layer = square(&mut d);
    set(layer, "satin.on", 1.0);
    set(layer, "satin.opacity", 100.0);
    set(layer, "satin.distance", 2.0);
    set(layer, "satin.size", 1.0);
    set(layer, "bevel.on", 1.0);
    let stacked = crate::composite_rgba(&d);
    // CS6 draws Satin and Bevel above the overlays, so they still show.
    assert_ne!(stacked.data, overlay_only.data);
    let layer = square(&mut d);
    set(layer, "satin.on", 0.0);
    set(layer, "bevel.on", 0.0);
    assert_eq!(crate::composite_rgba(&d).data, overlay_only.data);
}

#[test]
fn noise_breaks_a_drop_shadow_into_a_fixed_grain() {
    let shadowed = |noise: f64| {
        let mut d = doc();
        let layer = square(&mut d);
        set(layer, "dropShadow.on", 1.0);
        set(layer, "dropShadow.opacity", 100.0);
        set(layer, "dropShadow.spread", 100.0);
        set(layer, "dropShadow.distance", 3.0);
        if noise > 0.0 {
            set(layer, "dropShadow.noise", noise);
        }
        (0..12)
            .flat_map(|y| (0..12).map(move |x| (x, y)))
            .map(|(x, y)| rgb(&d, x, y))
            .collect::<Vec<_>>()
    };
    let clean = shadowed(0.0);
    let grainy = shadowed(82.0);
    assert_ne!(clean, grainy, "the grain shows");
    assert_eq!(grainy, shadowed(82.0), "and is the same every time");
    // The shadow band under the square: some pixels lighten toward white.
    let band: Vec<[u8; 3]> = (4..8).map(|x| grainy[9 * 12 + x]).collect();
    assert!(band.iter().any(|p| p[0] > clean[9 * 12 + 5][0]), "{band:?}");
}

#[test]
fn a_pattern_overlay_embeds_its_built_in_pattern() {
    let mut d = doc();
    let plain = crate::composite_rgba(&d);
    assert_eq!(layer_style_pattern_names().len(), 8);
    assert!(set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.on",
        1.0
    ));
    let layer = square(&mut d);
    assert_eq!(layer_style_value(layer, "patternOverlay.on"), Some(1.0));
    assert_eq!(
        layer_style_value(layer, "patternOverlay.pattern"),
        Some(0.0)
    );
    assert_eq!(pictura_codec::decode_patterns(&d).len(), 1);
    assert_ne!(
        crate::composite_rgba(&d).data,
        plain.data,
        "the checkerboard shows"
    );

    assert!(set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.pattern",
        3.0
    ));
    assert!(!set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.pattern",
        3.0
    ));
    assert!(!set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.pattern",
        99.0
    ));
    assert_eq!(pictura_codec::decode_patterns(&d).len(), 2);
    let before = crate::composite_rgba(&d);
    let reopened = pictura_codec::read_psd(&pictura_codec::write_psd(&d).unwrap()).unwrap();
    let layer = resolve_path(&reopened, "1").unwrap();
    assert_eq!(
        layer_style_value(layer, "patternOverlay.pattern"),
        Some(3.0)
    );
    assert_eq!(crate::composite_rgba(&reopened).data, before.data);
}

#[test]
fn global_light_setter_normalises_and_clamps() {
    let mut d = doc();
    assert!(set_document_global_light(&mut d, 370, 95));
    assert_eq!(
        d.global_light,
        pictura_core::GlobalLight {
            angle: 10,
            altitude: 90
        }
    );
    assert!(!set_document_global_light(&mut d, 10, 90), "unchanged");
    assert!(set_document_global_light(&mut d, -10, -5));
    assert_eq!(
        d.global_light,
        pictura_core::GlobalLight {
            angle: 350,
            altitude: 0
        }
    );
}

#[test]
fn global_light_setter_ignores_unchanged() {
    let mut d = doc();
    assert!(!set_document_global_light(&mut d, 30, 30));
    assert_eq!(d.global_light, pictura_core::GlobalLight::default());
}

#[test]
fn rasterize_layer_style_bakes_and_clears() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.useGlobalLight", 0.0);
    set(layer, "dropShadow.angle", 90.0);
    set(layer, "dropShadow.distance", 3.0);
    set(layer, "dropShadow.size", 0.0);
    layer.source_channels = Some(pictura_core::SourceChannels {
        depth: BitDepth::Sixteen,
        rect: layer.rect,
        planes: Vec::new(),
        working_idents: [0; 3],
    });
    let before = crate::composite_rgba(&d);
    assert!(crate::rasterize_layer_style(&mut d, "1"));
    assert!(!crate::has_layer_style(resolve_path(&d, "1").unwrap()));
    assert!(resolve_path(&d, "1").unwrap().source_channels.is_none());
    let after = crate::composite_rgba(&d);
    for (a, b) in before.data.iter().zip(after.data.iter()) {
        assert!(a.abs_diff(*b) <= 2, "bake pixel {a} vs {b}");
    }
    assert_ne!(rgb(&d, 5, 10), [255, 255, 255], "the shadow is baked");
}

#[test]
fn rasterize_layer_style_fill_cases() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.useGlobalLight", 0.0);
    set(layer, "dropShadow.angle", 90.0);
    set(layer, "dropShadow.distance", 3.0);
    set(layer, "dropShadow.size", 0.0);
    set(layer, "blending.fillOpacity", 50.0);
    let before = crate::composite_rgba(&d);
    assert!(crate::rasterize_layer_style(&mut d, "1"));
    let after = crate::composite_rgba(&d);
    for (a, b) in before.data.iter().zip(after.data.iter()) {
        assert!(a.abs_diff(*b) <= 2, "fill bake pixel {a} vs {b}");
    }
    assert_eq!(resolve_path(&d, "1").unwrap().fill, 255);
}

#[test]
fn rasterize_layer_style_global_light() {
    let mut d = doc();
    d.global_light.angle = 90;
    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.angle", 0.0);
    set(layer, "dropShadow.distance", 3.0);
    set(layer, "dropShadow.size", 0.0);
    let before = crate::composite_rgba(&d);
    assert!(crate::rasterize_layer_style(&mut d, "1"));
    let after = crate::composite_rgba(&d);
    for (a, b) in before.data.iter().zip(after.data.iter()) {
        assert!(a.abs_diff(*b) <= 2, "global-light bake pixel {a} vs {b}");
    }
}

#[test]
fn rasterize_layer_style_pattern_overlay() {
    let mut d = doc();
    assert!(set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.pattern",
        0.0
    ));
    set(square(&mut d), "patternOverlay.on", 1.0);
    let before = crate::composite_rgba(&d);
    assert!(crate::rasterize_layer_style(&mut d, "1"));
    let after = crate::composite_rgba(&d);
    for (a, b) in before.data.iter().zip(after.data.iter()) {
        assert!(a.abs_diff(*b) <= 2, "pattern bake pixel {a} vs {b}");
    }
}

#[test]
fn rasterize_layer_style_keeps_vector_mask() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "colorOverlay.on", 1.0);
    let p = |x: i32, y: i32| [x * 256, y * 256];
    layer.vector_mask = Some(pictura_core::VectorMask {
        subpaths: vec![pictura_core::VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: pictura_core::VectorFillRule::EvenOdd,
            points: vec![p(4, 4), p(8, 4), p(4, 8)],
        }],
        ..Default::default()
    });
    let before = crate::composite_rgba(&d);
    assert!(crate::rasterize_layer_style(&mut d, "1"));
    assert!(resolve_path(&d, "1").unwrap().vector_mask.is_some());
    let after = crate::composite_rgba(&d);
    let plane = before.width as usize * before.height as usize;
    let interior = 5 * before.width as usize + 5;
    let exterior = 7 * before.width as usize + 7;
    for offset in [0, plane, 2 * plane, 3 * plane] {
        assert_eq!(
            before.data[interior + offset],
            after.data[interior + offset],
            "vector-mask interior"
        );
        assert_eq!(
            before.data[exterior + offset],
            after.data[exterior + offset],
            "vector-mask exterior"
        );
    }
    assert!(
        after.data[plane + interior] < 128,
        "the baked overlay stays inside the vector mask"
    );
    assert_eq!(after.data[plane + exterior], 255);
}

#[test]
fn rasterize_layer_style_bakes_hidden_layers_without_destroying_them() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "colorOverlay.on", 1.0);
    layer.visible = false;
    assert!(crate::rasterize_layer_style(&mut d, "1"));
    let layer = resolve_path(&d, "1").unwrap();
    assert!(!layer.visible, "the layer stays hidden");
    assert!(!crate::has_layer_style(layer));
    assert!(
        crate::channel_plane(layer, -1).is_some_and(|alpha| alpha.iter().any(|&v| v > 0)),
        "the hidden layer's pixels survive"
    );
}

#[test]
fn rasterize_layer_style_refuses() {
    let mut d = doc();
    assert!(!crate::can_rasterize_layer_style(&d, "1"), "unstyled");

    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.on", 0.0);
    assert!(
        !crate::can_rasterize_layer_style(&d, "1"),
        "only disabled effects"
    );
    set(square(&mut d), "dropShadow.on", 1.0);
    assert!(crate::can_rasterize_layer_style(&d, "1"));

    let fill = crate::add_solid_fill(&mut d, "1", [0, 255, 0, 255]);
    assert!(!fill.is_empty());
    assert!(!crate::can_rasterize_layer_style(&d, &fill), "fill layer");
    let group = crate::add_group_in(&mut d, "1", "Group");
    assert!(!group.is_empty());
    assert!(!crate::can_rasterize_layer_style(&d, &group), "group");

    {
        let layer = crate::resolve_path_mut(&mut d, "1").unwrap();
        layer.background = true;
    }
    assert!(!crate::can_rasterize_layer_style(&d, "1"), "background");
    {
        let layer = crate::resolve_path_mut(&mut d, "1").unwrap();
        layer.background = false;
        layer.blending_ranges = vec![1u8].into();
    }
    assert!(
        !crate::can_rasterize_layer_style(&d, "1"),
        "customized Blend If"
    );
    {
        let layer = crate::resolve_path_mut(&mut d, "1").unwrap();
        layer.blending_ranges = pictura_core::Plane::default();
        layer.knockout = pictura_core::Knockout::Shallow;
    }
    assert!(!crate::can_rasterize_layer_style(&d, "1"), "knockout");
    crate::resolve_path_mut(&mut d, "1").unwrap().knockout = pictura_core::Knockout::None;

    let mut clipped = doc();
    set(square(&mut clipped), "dropShadow.on", 1.0);
    clipped
        .layers
        .push(pictura_core::Layer {
            clipping: true,
            ..Default::default()
        })
        .unwrap();
    assert!(
        !crate::can_rasterize_layer_style(&clipped, "1"),
        "clipped run above"
    );
}

#[test]
fn create_style_layers_recomposes_identically() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.useGlobalLight", 0.0);
    set(layer, "dropShadow.angle", 90.0);
    set(layer, "dropShadow.distance", 3.0);
    set(layer, "bevel.on", 1.0);
    set(layer, "colorOverlay.on", 1.0);
    set(layer, "innerGlow.on", 1.0);
    set(layer, "stroke.on", 1.0);
    let before = crate::composite_rgba(&d);
    let (created, base_path) = crate::create_style_layers_with_base(&mut d, "1");
    assert_eq!(
        created.len(),
        6,
        "one below (drop shadow) and five above (bevel two, color, inner glow, stroke)"
    );
    assert_eq!(base_path, "2");
    let base = resolve_path(&d, &base_path).unwrap();
    assert!(!crate::has_layer_style(base));
    let after = crate::composite_rgba(&d);
    for (a, b) in before.data.iter().zip(after.data.iter()) {
        assert!(a.abs_diff(*b) <= 4, "create-layers pixel {a} vs {b}");
    }
}

#[test]
fn create_style_layers_inherits_base_state() {
    let mut d = doc();
    let layer = square(&mut d);
    layer.opacity = 128;
    layer.visible = false;
    layer.mask = Some(pictura_core::LayerMask {
        rect: layer.rect,
        data: Some(vec![255u8; 16].into()),
        ..Default::default()
    });
    set(layer, "colorOverlay.on", 1.0);
    let created = crate::create_style_layers(&mut d, "1");
    assert_eq!(created.len(), 1);
    let produced = resolve_path(&d, &created[0]).unwrap();
    assert_eq!(produced.opacity, 128);
    assert_eq!(produced.fill, 255);
    assert!(!produced.visible, "a hidden base yields hidden layers");
    assert!(produced.mask.is_none());
    let base = resolve_path(&d, "1").unwrap();
    assert!(!base.visible);
    assert!(base.mask.is_some());
    assert_eq!(base.fill, 255);
}

#[test]
fn create_style_layers_above_order() {
    let mut d = doc();
    assert!(set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.pattern",
        0.0
    ));
    let layer = square(&mut d);
    set(layer, "patternOverlay.on", 1.0);
    set(layer, "colorOverlay.on", 1.0);
    set(layer, "innerGlow.on", 1.0);
    set(layer, "stroke.on", 1.0);
    let created = crate::create_style_layers(&mut d, "1");
    assert_eq!(created.len(), 4);
    for (index, name) in ["Pattern Overlay", "Color Overlay", "Inner Glow", "Stroke"]
        .iter()
        .enumerate()
    {
        assert_eq!(resolve_path(&d, &created[index]).unwrap().name, *name);
    }
}

#[test]
fn create_style_layers_global_light() {
    let mut d = doc();
    d.global_light.angle = 90;
    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.angle", 0.0);
    set(layer, "dropShadow.distance", 3.0);
    set(layer, "dropShadow.size", 0.0);
    let before = crate::composite_rgba(&d);
    let created = crate::create_style_layers(&mut d, "1");
    assert_eq!(created.len(), 1);
    let after = crate::composite_rgba(&d);
    for (a, b) in before.data.iter().zip(after.data.iter()) {
        assert!(a.abs_diff(*b) <= 4, "global-light create pixel {a} vs {b}");
    }
}

#[test]
fn create_style_layers_names_and_modes() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "outerGlow.on", 1.0);
    set(layer, "innerGlow.on", 1.0);
    let created = crate::create_style_layers(&mut d, "1");
    assert_eq!(created.len(), 3);
    let mut names: Vec<String> = created
        .iter()
        .map(|path| resolve_path(&d, path).unwrap().name.clone())
        .collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), created.len(), "names are unique");
    assert_eq!(resolve_path(&d, &created[0]).unwrap().name, "Drop Shadow");
    assert_eq!(resolve_path(&d, &created[1]).unwrap().name, "Outer Glow");
    assert_eq!(
        resolve_path(&d, &created[0]).unwrap().blend,
        pictura_core::BlendMode::Multiply
    );
    assert_eq!(
        resolve_path(&d, &created[1]).unwrap().blend,
        pictura_core::BlendMode::Screen
    );
}

#[test]
fn create_style_layers_pillow_four_passes() {
    let mut inner = doc();
    let layer = square(&mut inner);
    set(layer, "bevel.on", 1.0);
    assert_eq!(
        crate::create_style_layers(&mut inner, "1").len(),
        2,
        "an Inner bevel has two passes"
    );

    let mut pillow = doc();
    let layer = square(&mut pillow);
    set(layer, "bevel.on", 1.0);
    set(layer, "bevel.style", 3.0);
    assert_eq!(
        crate::create_style_layers(&mut pillow, "1").len(),
        4,
        "a Pillow bevel has four passes"
    );
}

#[test]
fn create_style_layers_refuses() {
    let mut d = doc();
    assert!(crate::create_style_layers(&mut d, "1").is_empty());

    let layer = square(&mut d);
    set(layer, "dropShadow.on", 1.0);
    set(layer, "dropShadow.on", 0.0);
    assert!(crate::create_style_layers(&mut d, "1").is_empty());
    set(square(&mut d), "dropShadow.on", 1.0);
    assert!(crate::can_create_style_layers(&d, "1"));

    let fill = crate::add_solid_fill(&mut d, "1", [0, 255, 0, 255]);
    assert!(!crate::can_create_style_layers(&d, &fill), "fill layer");
    let group = crate::add_group_in(&mut d, "1", "Group");
    assert!(!crate::can_create_style_layers(&d, &group), "group");

    let mut background = doc();
    set(square(&mut background), "dropShadow.on", 1.0);
    background.layers[1].background = true;
    assert!(!crate::can_create_style_layers(&background, "1"));

    let mut clipped = doc();
    set(square(&mut clipped), "dropShadow.on", 1.0);
    clipped.layers[1].clipping = true;
    assert!(!crate::can_create_style_layers(&clipped, "1"));

    let mut smart = doc();
    assert!(crate::convert_to_smart_object(&mut smart, "1"));
    let layer = crate::resolve_path_mut(&mut smart, "1").unwrap();
    set(layer, "dropShadow.on", 1.0);
    assert!(
        !crate::can_rasterize_layer_style(&smart, "1"),
        "smart object"
    );

    let mut run = doc();
    set(square(&mut run), "dropShadow.on", 1.0);
    run.layers
        .push(pictura_core::Layer {
            clipping: true,
            ..Default::default()
        })
        .unwrap();
    assert!(!crate::can_create_style_layers(&run, "1"));
}

fn contour_text_round_trips() {
    let mut d = doc();
    let layer = square(&mut d);
    assert!(set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 64,128 255,255"
    ));
    assert_eq!(
        layer_style_contour(layer, "bevel.glossContour").as_deref(),
        Some("0,0 64,128 255,255")
    );
    let bevel = decode_bevel_emboss(layer).expect("decodes");
    assert_eq!(
        bevel.gloss_contour.points(),
        &[(0.0f32, 0.0), (64.0, 128.0), (255.0, 255.0)]
    );
}

#[test]
fn absent_contour_reads_linear() {
    let mut d = doc();
    let layer = square(&mut d);
    assert_eq!(
        layer_style_contour(layer, "bevel.edgeContour").as_deref(),
        Some("0,0 255,255")
    );
}

#[test]
fn bad_contour_text_is_refused() {
    let mut d = doc();
    let layer = square(&mut d);
    for text in ["0,0", "0,0 0,255", "0,0 256,10", "0,0 nope 255,255"] {
        assert!(
            !set_layer_style_contour(layer, "bevel.glossContour", text),
            "{text}"
        );
    }
    layer.is_group = true;
    assert!(!set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 255,255"
    ));
    layer.is_group = false;
    layer.background = true;
    assert!(!set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 255,255"
    ));
}

#[test]
fn contour_edit_preserves_point_items() {
    use pictura_codec::DescValue;

    let mut d = doc();
    let layer = square(&mut d);
    assert!(set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 128,128 255,255"
    ));
    let mut items = lfx2_items(layer);
    let ebbl = items
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"ebbl")
        .map(|(_, value)| value)
        .expect("ebbl");
    let DescValue::Object { items: fields, .. } = ebbl else {
        panic!("effect object");
    };
    let trns = fields
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"TrnS")
        .map(|(_, value)| value)
        .expect("TrnS");
    let DescValue::Object {
        items: trns_items, ..
    } = trns
    else {
        panic!("contour object");
    };
    let crv = trns_items
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"Crv ")
        .map(|(_, value)| value)
        .expect("Crv");
    let DescValue::List(points) = crv else {
        panic!("point list");
    };
    let DescValue::Object { items: point, .. } = &mut points[1] else {
        panic!("point");
    };
    point.push((b"Smoo".to_vec(), DescValue::Bool(true)));
    write_lfx2(layer, items);

    assert!(set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 128,200 255,255"
    ));
    let items = lfx2_items(layer);
    let ebbl = items
        .iter()
        .find(|(key, _)| key.as_slice() == b"ebbl")
        .map(|(_, value)| value)
        .expect("ebbl");
    let DescValue::Object { items: fields, .. } = ebbl else {
        panic!("effect object");
    };
    let trns = fields
        .iter()
        .find(|(key, _)| key.as_slice() == b"TrnS")
        .map(|(_, value)| value)
        .expect("TrnS");
    let DescValue::Object {
        items: trns_items, ..
    } = trns
    else {
        panic!("contour object");
    };
    let crv = trns_items
        .iter()
        .find(|(key, _)| key.as_slice() == b"Crv ")
        .map(|(_, value)| value)
        .expect("Crv");
    let DescValue::List(points) = crv else {
        panic!("point list");
    };
    let DescValue::Object { items: point, .. } = &points[1] else {
        panic!("point");
    };
    assert!(point
        .iter()
        .any(|(key, value)| { key.as_slice() == b"Smoo" && *value == DescValue::Bool(true) }));
    assert!(point
        .iter()
        .any(|(key, value)| { key.as_slice() == b"Vrtc" && *value == DescValue::Double(200.0) }));
}

#[test]
fn bevel_sub_panel_fields_round_trip() {
    let mut d = doc();
    let layer = square(&mut d);
    set(layer, "bevel.edgeContourOn", 1.0);
    set(layer, "bevel.edgeRange", 30.0);
    set(layer, "bevel.edgeAntiAlias", 1.0);
    assert_eq!(layer_style_value(layer, "bevel.edgeContourOn"), Some(1.0));
    assert_eq!(layer_style_value(layer, "bevel.edgeRange"), Some(30.0));
    assert_eq!(layer_style_value(layer, "bevel.edgeAntiAlias"), Some(1.0));
    let bevel = decode_bevel_emboss(layer).expect("decodes");
    assert!(bevel.edge_contour_on);
    assert_eq!(bevel.edge_range, 30.0);
    assert!(bevel.edge_anti_alias);
}

#[test]
fn bevel_gloss_antialias_defaults_on() {
    use pictura_codec::DescValue;

    let mut d = doc();
    let layer = square(&mut d);
    assert!(set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 255,255"
    ));
    assert_eq!(layer_style_value(layer, "bevel.antiAlias"), Some(1.0));
    let items = lfx2_items(layer);
    let ebbl = items
        .iter()
        .find(|(key, _)| key.as_slice() == b"ebbl")
        .map(|(_, value)| value)
        .expect("ebbl");
    let DescValue::Object { items: fields, .. } = ebbl else {
        panic!("effect object");
    };
    assert!(fields.iter().any(|(key, value)| {
        key.as_slice() == b"antialiasGloss" && *value == DescValue::Bool(true)
    }));
}

#[test]
fn gradient_text_round_trips() {
    let mut d = doc();
    let layer = square(&mut d);
    assert!(set_layer_style_gradient(
        layer,
        "gradientOverlay.gradient",
        "0:000000 2048:00ff00 4096:ffffff"
    ));
    assert_eq!(
        layer_style_gradient(layer, "gradientOverlay.gradient").as_deref(),
        Some("0:000000 2048:00ff00 4096:ffffff")
    );
    let overlay = decode_gradient_overlay(layer).expect("decodes");
    assert_eq!(
        overlay
            .stops
            .iter()
            .map(|stop| (stop.location, stop.color))
            .collect::<Vec<_>>(),
        vec![(0, [0, 0, 0]), (2048, [0, 255, 0]), (4096, [255, 255, 255])]
    );
}

#[test]
fn gradient_edit_preserves_opacity_stops() {
    use pictura_codec::DescValue;

    let mut d = doc();
    let custom = {
        let layer = square(&mut d);
        assert!(set_layer_style_gradient(
            layer,
            "gradientOverlay.gradient",
            "0:000000 2048:000000 4096:000000"
        ));
        set(layer, "gradientOverlay.on", 1.0);
        let opacity = |location: i32, value: f64| DescValue::Object {
            name: String::new(),
            class_id: b"TrnS".to_vec(),
            items: vec![
                (
                    b"Opct".to_vec(),
                    DescValue::UnitFloat {
                        unit: *b"#Prc",
                        value,
                    },
                ),
                (b"Lctn".to_vec(), DescValue::Long(location)),
                (b"Mdpn".to_vec(), DescValue::Long(50)),
            ],
        };
        let custom = vec![opacity(0, 40.0), opacity(4096, 90.0)];
        let mut items = lfx2_items(layer);
        let grfl = items
            .iter_mut()
            .find(|(key, _)| key.as_slice() == b"GrFl")
            .map(|(_, value)| value)
            .expect("GrFl");
        let DescValue::Object { items: fields, .. } = grfl else {
            panic!("effect object");
        };
        let grad = fields
            .iter_mut()
            .find(|(key, _)| key.as_slice() == b"Grad")
            .map(|(_, value)| value)
            .expect("Grad");
        let DescValue::Object {
            items: grad_items, ..
        } = grad
        else {
            panic!("gradient object");
        };
        let trns = grad_items
            .iter_mut()
            .find(|(key, _)| key.as_slice() == b"Trns")
            .map(|(_, value)| value)
            .expect("Trns");
        *trns = DescValue::List(custom.clone());
        let clrs = grad_items
            .iter_mut()
            .find(|(key, _)| key.as_slice() == b"Clrs")
            .map(|(_, value)| value)
            .expect("Clrs");
        let DescValue::List(stops) = clrs else {
            panic!("stops");
        };
        let DescValue::Object { items: middle, .. } = &mut stops[1] else {
            panic!("stop");
        };
        set_item(middle, b"Mdpn", DescValue::Long(25));
        write_lfx2(layer, items);
        custom
    };

    let before = crate::composite_rgba(&d);
    let layer = square(&mut d);
    assert!(set_layer_style_gradient(
        layer,
        "gradientOverlay.gradient",
        "0:ff0000 2048:00ff00 4096:0000ff"
    ));
    let items = lfx2_items(layer);
    let grfl = items
        .iter()
        .find(|(key, _)| key.as_slice() == b"GrFl")
        .map(|(_, value)| value)
        .expect("GrFl");
    let DescValue::Object { items: fields, .. } = grfl else {
        panic!("effect object");
    };
    let grad = fields
        .iter()
        .find(|(key, _)| key.as_slice() == b"Grad")
        .map(|(_, value)| value)
        .expect("Grad");
    let DescValue::Object {
        items: grad_items, ..
    } = grad
    else {
        panic!("gradient object");
    };
    let trns = grad_items
        .iter()
        .find(|(key, _)| key.as_slice() == b"Trns")
        .map(|(_, value)| value)
        .expect("Trns");
    assert_eq!(*trns, DescValue::List(custom));
    let clrs = grad_items
        .iter()
        .find(|(key, _)| key.as_slice() == b"Clrs")
        .map(|(_, value)| value)
        .expect("Clrs");
    let DescValue::List(stops) = clrs else {
        panic!("stops");
    };
    let DescValue::Object { items: middle, .. } = &stops[1] else {
        panic!("stop");
    };
    assert!(middle
        .iter()
        .any(|(key, value)| { key.as_slice() == b"Mdpn" && *value == DescValue::Long(25) }));
    let after = crate::composite_rgba(&d);
    let plane = (before.width * before.height) as usize;
    assert_eq!(before.data[3 * plane..], after.data[3 * plane..]);
    assert_ne!(before.data[..3 * plane], after.data[..3 * plane]);
}

#[test]
fn bad_gradient_text_is_refused() {
    let mut d = doc();
    let layer = square(&mut d);
    for text in [
        "0:000000",
        "0:000000 4097:ffffff",
        "0:000000 0:ffffff",
        "0:zzzzzz 4096:ffffff",
        "0:000000ff 4096:ffffff",
    ] {
        assert!(
            !set_layer_style_gradient(layer, "gradientOverlay.gradient", text),
            "{text}"
        );
    }
    layer.is_group = true;
    assert!(!set_layer_style_gradient(
        layer,
        "gradientOverlay.gradient",
        "0:000000 4096:ffffff"
    ));
    layer.is_group = false;
    layer.background = true;
    assert!(!set_layer_style_gradient(
        layer,
        "gradientOverlay.gradient",
        "0:000000 4096:ffffff"
    ));
}

#[test]
fn authored_contour_and_gradient_survive_save() {
    let mut d = doc();
    let layer = square(&mut d);
    assert!(set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 64,128 255,255"
    ));
    assert!(set_layer_style_gradient(
        layer,
        "gradientOverlay.gradient",
        "0:000000 2048:00ff00 4096:ffffff"
    ));
    let before = crate::composite_rgba(&d);
    let bytes = pictura_codec::write_psd(&d).unwrap();
    let reopened = pictura_codec::read_psd(&bytes).unwrap();
    let layer = resolve_path(&reopened, "1").unwrap();
    assert_eq!(
        layer_style_contour(layer, "bevel.glossContour").as_deref(),
        Some("0,0 64,128 255,255")
    );
    assert_eq!(
        layer_style_gradient(layer, "gradientOverlay.gradient").as_deref(),
        Some("0:000000 2048:00ff00 4096:ffffff")
    );
    let bevel = decode_bevel_emboss(layer).expect("decodes");
    assert_eq!(
        bevel.gloss_contour.points(),
        &[(0.0f32, 0.0), (64.0, 128.0), (255.0, 255.0)]
    );
    assert_eq!(crate::composite_rgba(&reopened).data, before.data);
}

#[test]
fn channel_include_toggles_the_restriction() {
    let mut d = doc();
    let layer = square(&mut d);
    assert!(set_layer_style_value(layer, "blending.include1", 0.0));
    assert_eq!(layer.restricted_channels, vec![1]);
    assert_eq!(layer_style_value(layer, "blending.include1"), Some(0.0));
    assert!(set_layer_style_value(layer, "blending.include1", 1.0));
    assert!(layer.restricted_channels.is_empty());
    assert!(!set_layer_style_value(layer, "blending.include1", 1.0));
}

#[test]
fn channels_are_included_by_default() {
    let mut d = doc();
    let layer = square(&mut d);
    for key in [
        "blending.include0",
        "blending.include1",
        "blending.include2",
    ] {
        assert_eq!(layer_style_value(layer, key), Some(1.0), "{key}");
    }
    assert_eq!(layer_style_value(layer, "blending.include3"), None);
}

#[test]
fn group_accepts_channel_include() {
    let mut d = doc();
    let layer = square(&mut d);
    layer.is_group = true;
    assert!(set_layer_style_value(layer, "blending.include2", 0.0));
    assert_eq!(layer.restricted_channels, vec![2]);
    assert!(!set_layer_style_value(layer, "dropShadow.on", 1.0));
}

#[test]
fn restricted_channels_survives_copy_and_paste() {
    let mut d = doc();
    let style = {
        let layer = square(&mut d);
        layer.restricted_channels = vec![1];
        copy_layer_style(layer).expect("style")
    };
    let backdrop = resolve_path(&d, "0").unwrap().clone();
    let source = resolve_path(&d, "1").unwrap().clone();
    let mut target = source.clone();
    target.name = "Target".into();
    target.restricted_channels.clear();
    target.rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 4,
        right: 4,
    };
    d.layers = pictura_core::LayerStack::from_layers(vec![backdrop, target, source]);
    assert_eq!(paste_layer_style(&mut d, &["1"], &style), 1);
    assert_eq!(resolve_path(&d, "1").unwrap().restricted_channels, vec![1]);
    assert_eq!(rgb(&d, 1, 1), [255, 255, 0]);
    assert_eq!(rgb(&d, 5, 5), [255, 255, 0]);
}

#[test]
fn default_gradient_text_creates_an_absent_effect() {
    let mut d = doc();
    let layer = square(&mut d);
    assert!(set_layer_style_gradient(
        layer,
        "gradientOverlay.gradient",
        "0:000000 4096:ffffff"
    ));
    assert_eq!(
        layer_style_value(layer, "gradientOverlay.exists"),
        Some(1.0)
    );
    assert_eq!(
        layer_style_gradient(layer, "gradientOverlay.gradient").as_deref(),
        Some("0:000000 4096:ffffff")
    );
}

#[test]
fn malformed_stored_contour_and_gradient_are_replaced() {
    use pictura_codec::DescValue;

    let mut d = doc();
    let layer = square(&mut d);
    assert!(set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 128,128 255,255"
    ));
    let mut items = lfx2_items(layer);
    let ebbl = items
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"ebbl")
        .map(|(_, value)| value)
        .expect("ebbl");
    let DescValue::Object { items: fields, .. } = ebbl else {
        panic!("effect object");
    };
    let trns = fields
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"TrnS")
        .map(|(_, value)| value)
        .expect("TrnS");
    *trns = DescValue::Bool(true);
    write_lfx2(layer, items);
    assert!(set_layer_style_contour(
        layer,
        "bevel.glossContour",
        "0,0 64,64 255,255"
    ));
    assert_eq!(
        layer_style_contour(layer, "bevel.glossContour").as_deref(),
        Some("0,0 64,64 255,255")
    );

    assert!(set_layer_style_gradient(
        layer,
        "gradientOverlay.gradient",
        "0:000000 2048:000000 4096:ffffff"
    ));
    let mut items = lfx2_items(layer);
    let grfl = items
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"GrFl")
        .map(|(_, value)| value)
        .expect("GrFl");
    let DescValue::Object { items: fields, .. } = grfl else {
        panic!("effect object");
    };
    let grad = fields
        .iter_mut()
        .find(|(key, _)| key.as_slice() == b"Grad")
        .map(|(_, value)| value)
        .expect("Grad");
    *grad = DescValue::Bool(true);
    write_lfx2(layer, items);
    assert!(set_layer_style_gradient(
        layer,
        "gradientOverlay.gradient",
        "0:ff0000 2048:00ff00 4096:0000ff"
    ));
    assert_eq!(
        layer_style_gradient(layer, "gradientOverlay.gradient").as_deref(),
        Some("0:ff0000 2048:00ff00 4096:0000ff")
    );
}
