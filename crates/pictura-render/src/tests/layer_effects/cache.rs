//! Cached effect-pass replays: cold vs warm, and region vs full.

use super::*;

fn styled_layer() -> Layer {
    let mut layer = solid(
        "Styled",
        rect(30, 30, 34, 34),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    for key in [
        "dropShadow.on",
        "outerGlow.on",
        "innerShadow.on",
        "innerGlow.on",
        "stroke.on",
        "satin.on",
        "bevel.on",
        "colorOverlay.on",
        "gradientOverlay.on",
        "patternOverlay.on",
    ] {
        crate::set_layer_style_value(&mut layer, key, 1.0);
    }
    layer
}

fn styled_doc() -> Document {
    doc(
        80,
        80,
        vec![
            solid(
                "Backdrop",
                full(80, 80),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            styled_layer(),
        ],
    )
}

#[test]
fn effect_passes_are_byte_identical_cached() {
    let d = styled_doc();
    crate::purge_source_cache();
    let cold = composite_rgba(&d);
    let warm = composite_rgba(&d);
    assert_eq!(
        cold.data.as_ref(),
        warm.data.as_ref(),
        "a warm effect pass differs from a cold one"
    );

    let full = composite_rgba(&d);
    let region = crate::composite::composite_rgba_region(&d, 14, 26, 30, 30);
    for y in 0..30 {
        for x in 0..30 {
            assert_eq!(
                px(&region, x, y),
                px(&full, 14 + x, 26 + y),
                "region pixel ({x},{y})"
            );
        }
    }
}

fn styled_region_doc() -> Document {
    let mut layer = solid(
        "Styled",
        rect(100, 100, 140, 140),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    for key in [
        "dropShadow.on",
        "outerGlow.on",
        "innerShadow.on",
        "innerGlow.on",
        "stroke.on",
        "satin.on",
        "bevel.on",
        "colorOverlay.on",
        "gradientOverlay.on",
        "patternOverlay.on",
    ] {
        crate::set_layer_style_value(&mut layer, key, 1.0);
    }
    crate::set_layer_style_value(&mut layer, "dropShadow.distance", 40.0);
    let mut d = doc(
        300,
        300,
        vec![
            solid(
                "Backdrop",
                full(300, 300),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    );
    crate::set_document_layer_style_value(&mut d, "1", "patternOverlay.pattern", 0.0);
    crate::set_document_layer_style_value(&mut d, "1", "patternOverlay.on", 1.0);
    d
}

fn overlay_doc(gradient_align: bool, pattern_link: bool) -> Document {
    let mut layer = solid(
        "Styled",
        rect(100, 100, 140, 140),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    crate::set_layer_style_value(&mut layer, "gradientOverlay.on", 1.0);
    crate::set_layer_style_value(
        &mut layer,
        "gradientOverlay.align",
        if gradient_align { 1.0 } else { 0.0 },
    );
    let mut d = doc(
        300,
        300,
        vec![
            solid(
                "Backdrop",
                full(300, 300),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    );
    crate::set_document_layer_style_value(&mut d, "1", "patternOverlay.pattern", 0.0);
    crate::set_document_layer_style_value(&mut d, "1", "patternOverlay.on", 1.0);
    crate::set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.link",
        if pattern_link { 1.0 } else { 0.0 },
    );
    d
}

fn banded_styled_doc() -> Document {
    let mut layer = solid(
        "Styled",
        rect(100, 100, 140, 140),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    crate::set_layer_style_value(&mut layer, "dropShadow.on", 1.0);
    crate::set_layer_style_value(&mut layer, "dropShadow.size", 20.0);
    doc(
        300,
        300,
        vec![
            solid(
                "Backdrop",
                full(300, 300),
                (255, 255, 255),
                255,
                BlendMode::Normal,
                255,
            ),
            layer,
        ],
    )
}

fn samples(passes: &[crate::layer_effects::EffectPass], x: i32, y: i32) -> Vec<([f32; 3], f32)> {
    let mut out = Vec::new();
    for pass in passes {
        if x < pass.rect.left || x >= pass.rect.right || y < pass.rect.top || y >= pass.rect.bottom
        {
            continue;
        }
        let w = pass.rect.width().max(0) as usize;
        let i = (y - pass.rect.top) as usize * w + (x - pass.rect.left) as usize;
        let color = match &pass.color {
            crate::layer_effects::EffectColor::Constant(color) => *color,
            crate::layer_effects::EffectColor::PerPixel(buffer) => {
                let plane = buffer.pixel_count();
                [
                    buffer.data[i] as f32 / 255.0,
                    buffer.data[plane + i] as f32 / 255.0,
                    buffer.data[2 * plane + i] as f32 / 255.0,
                ]
            }
        };
        if pass.alpha[i] > 0.0 {
            out.push((color, pass.alpha[i]));
        }
    }
    out
}

fn assert_region_passes_equal(d: &Document, region: PsdRect) {
    let layer = &d.layers[1];
    let effects = crate::layer_source::decoded_effects(layer);
    let full = crate::layer_effects::build_layer_passes(layer, d, &effects, None);
    let part = crate::layer_effects::build_layer_passes(layer, d, &effects, Some(region));
    for y in region.top..region.bottom {
        for x in region.left..region.right {
            assert_eq!(
                samples(&part.below, x, y),
                samples(&full.below, x, y),
                "below ({x},{y})"
            );
            assert_eq!(
                samples(&part.above, x, y),
                samples(&full.above, x, y),
                "above ({x},{y})"
            );
        }
    }
    for pass in part.below.iter().chain(&part.above) {
        assert!(
            pass.rect.left >= region.left
                && pass.rect.top >= region.top
                && pass.rect.right <= region.right
                && pass.rect.bottom <= region.bottom,
            "pass {:?} escapes {region:?}",
            pass.rect
        );
    }
}

#[test]
fn region_pass_equals_full_pass_for_every_family() {
    let d = styled_region_doc();
    assert_region_passes_equal(&d, rect(110, 110, 170, 170));
}

#[test]
fn dab_region_pass_is_bounded_and_equal() {
    let mut d = styled_region_doc();
    let layer = &mut d.layers[1];
    let w = layer.rect.width() as usize;
    let alpha = layer
        .channels
        .iter_mut()
        .find(|c| c.id == -1)
        .expect("alpha plane");
    let data = alpha.data.as_mut_slice();
    for y in 15..25 {
        for x in 15..25 {
            data[y * w + x] = 0;
        }
    }
    assert_region_passes_equal(&d, rect(105, 105, 155, 155));
}

#[test]
fn banded_sub_region_reuses_one_pass() {
    let d = banded_styled_doc();
    let (x0, y0, rw, rh) = (90u32, 80u32, 120u32, 160u32);
    crate::purge_source_cache();
    let region = crate::composite::composite_rgba_region(&d, x0, y0, rw, rh);
    assert_eq!(
        crate::layer_source::pass_cache_len(),
        1,
        "the banded composite builds one whole-layer pass"
    );
    crate::purge_source_cache();
    let whole = composite_rgba(&d);
    for y in 0..rh {
        for x in 0..rw {
            assert_eq!(
                px(&region, x, y),
                px(&whole, x0 + x, y0 + y),
                "banded pixel ({x},{y})"
            );
        }
    }
}

#[test]
fn overlay_alignment_and_link_are_region_safe() {
    for (align, link) in [(true, true), (false, false)] {
        let d = overlay_doc(align, link);
        assert_region_passes_equal(&d, rect(110, 110, 170, 170));
    }
}
