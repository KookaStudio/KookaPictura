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
    set(layer, "blending.blendInterior", 0.0);
    assert_eq!(layer.blend, BlendMode::Multiply);
    assert_eq!(layer.opacity, 128);
    assert_eq!(layer.knockout, Knockout::Deep);
    assert!(!layer.blend_interior);
    assert_eq!(layer_style_value(layer, "blending.opacity"), Some(50.0));
    assert!(!set_layer_style_value(layer, "blending.mode", 99.0));
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
