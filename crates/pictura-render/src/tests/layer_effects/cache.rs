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
    let full = crate::layer_effects::build_layer_passes(layer, d, &effects, None, None);
    let part = crate::layer_effects::build_layer_passes(layer, d, &effects, Some(region), None);
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
    let _guard = crate::layer_source::cache_test_guard();
    crate::purge_source_cache();
    let region = crate::composite::composite_rgba_region(&d, x0, y0, rw, rh);
    assert_eq!(
        crate::layer_source::pass_build_count(&d.layers[1], &d),
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

fn styled_pixel_layer(name: &str, r: PsdRect, rgb: (u8, u8, u8)) -> Layer {
    let mut layer = solid(name, r, rgb, 255, BlendMode::Normal, 255);
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
    ] {
        crate::set_layer_style_value(&mut layer, key, 1.0);
    }
    crate::set_layer_style_value(&mut layer, "dropShadow.distance", 40.0);
    layer
}

fn non_pattern_styled_doc() -> Document {
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
            styled_pixel_layer("Styled", rect(100, 100, 140, 140), (255, 0, 0)),
        ],
    )
}

fn two_styled_doc() -> Document {
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
            styled_pixel_layer("Styled", rect(100, 100, 140, 140), (255, 0, 0)),
            styled_pixel_layer("Unchanged", rect(160, 160, 200, 200), (0, 0, 255)),
        ],
    )
}

fn assert_slice(region: &PixelBuffer, full: &PixelBuffer, x0: u32, y0: u32) {
    for y in 0..region.height {
        for x in 0..region.width {
            assert_eq!(
                px(region, x, y),
                px(full, x0 + x, y0 + y),
                "pixel ({x},{y})"
            );
        }
    }
}

#[test]
fn warm_short_region_reuses_ready_effect_pass() {
    let _guard = crate::layer_source::cache_test_guard();
    crate::purge_source_cache();
    let d = non_pattern_styled_doc();
    let full = composite_rgba(&d);
    let layer = &d.layers[1];
    let effects = crate::layer_source::decoded_effects(layer);
    let before = crate::layer_source::pass_build_count(layer, &d);
    let region = crate::composite::composite_rgba_region(&d, 110, 110, 60, 60);
    assert_eq!(
        crate::layer_source::pass_build_count(layer, &d),
        before,
        "a warm region builds no effect pass"
    );
    let full_pass = crate::layer_source::effect_passes_with(layer, &d, &effects, None, false, None);
    let region_pass = crate::layer_source::effect_passes_with(
        layer,
        &d,
        &effects,
        Some(rect(110, 110, 170, 170)),
        false,
        None,
    );
    assert!(
        std::sync::Arc::ptr_eq(&full_pass, &region_pass),
        "the region shares the retained whole-layer pass"
    );
    assert_slice(&region, &full, 110, 110);
}

#[test]
fn cold_short_region_builds_transient_effect_pass() {
    let _guard = crate::layer_source::cache_test_guard();
    crate::purge_source_cache();
    let d = non_pattern_styled_doc();
    let layer = &d.layers[1];
    let before = crate::layer_source::pass_build_count(layer, &d);
    let region = crate::composite::composite_rgba_region(&d, 110, 110, 60, 60);
    assert_eq!(
        crate::layer_source::pass_build_count(layer, &d),
        before + 1,
        "a cold region builds one transient effect pass"
    );
    let full = composite_rgba(&d);
    assert_slice(&region, &full, 110, 110);
}

#[test]
fn full_composite_reuses_ready_effect_pass() {
    let _guard = crate::layer_source::cache_test_guard();
    crate::purge_source_cache();
    let d = non_pattern_styled_doc();
    let _ = composite_rgba(&d);
    let layer = &d.layers[1];
    let effects = crate::layer_source::decoded_effects(layer);
    let builds = crate::layer_source::pass_build_count(layer, &d);
    let first = crate::layer_source::effect_passes_with(layer, &d, &effects, None, false, None);
    let second = crate::layer_source::effect_passes_with(layer, &d, &effects, None, false, None);
    assert!(
        std::sync::Arc::ptr_eq(&first, &second),
        "a full composite reuses its cached pass"
    );
    assert_eq!(
        crate::layer_source::pass_build_count(layer, &d),
        builds,
        "no effect pass is rebuilt"
    );
}

#[test]
fn region_effect_families_match_cold_and_warm_slices() {
    let _guard = crate::layer_source::cache_test_guard();
    let d = non_pattern_styled_doc();
    let full = composite_rgba(&d);
    crate::purge_source_cache();
    let cold = crate::composite::composite_rgba_region(&d, 95, 95, 70, 70);
    assert_slice(&cold, &full, 95, 95);
    let _ = composite_rgba(&d);
    let warm = crate::composite::composite_rgba_region(&d, 95, 95, 70, 70);
    assert_slice(&warm, &full, 95, 95);
    assert_eq!(
        cold.data.as_ref(),
        warm.data.as_ref(),
        "cold and warm regions agree"
    );
}

#[test]
fn edited_styled_region_rebuilds_after_warm() {
    let _guard = crate::layer_source::cache_test_guard();
    let mut d = two_styled_doc();
    let _ = composite_rgba(&d);
    let unchanged_before = crate::layer_source::pass_build_count(&d.layers[2], &d);
    let w = d.layers[1].rect.width() as usize;
    let alpha = d.layers[1]
        .channels
        .iter_mut()
        .find(|channel| channel.id == -1)
        .expect("alpha plane");
    let data = alpha.data.as_mut_slice();
    for y in 15..25 {
        for x in 15..25 {
            data[y * w + x] = 0;
        }
    }
    let before = crate::layer_source::pass_build_count(&d.layers[1], &d);
    let region = crate::composite::composite_rgba_region(&d, 105, 105, 50, 50);
    assert_eq!(
        crate::layer_source::pass_build_count(&d.layers[1], &d),
        before + 1,
        "the edit rebuilds fresh passes"
    );
    assert_eq!(
        crate::layer_source::pass_build_count(&d.layers[2], &d),
        unchanged_before,
        "an unchanged layer reuses its warmed pass"
    );
    let full = composite_rgba(&d);
    assert_slice(&region, &full, 105, 105);
}

#[test]
fn cold_overlay_region_sources_stay_bounded() {
    let _guard = crate::layer_source::cache_test_guard();
    crate::purge_source_cache();
    let mut layer = solid(
        "Gradient",
        rect(100, 100, 140, 140),
        (255, 0, 0),
        255,
        BlendMode::Normal,
        255,
    );
    crate::set_layer_style_value(&mut layer, "gradientOverlay.on", 1.0);
    let d = doc(
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
    let layer = &d.layers[1];
    let effects = crate::layer_source::decoded_effects(layer);
    let region = rect(110, 110, 170, 170);
    let passes =
        crate::layer_source::effect_passes_with(layer, &d, &effects, Some(region), false, None);
    for pass in passes.below.iter().chain(&passes.above) {
        assert!(
            pass.rect.left >= region.left
                && pass.rect.top >= region.top
                && pass.rect.right <= region.right
                && pass.rect.bottom <= region.bottom,
            "pass {:?} escapes {region:?}",
            pass.rect
        );
    }
    let part = crate::composite::composite_rgba_region(&d, 110, 110, 60, 60);
    let full = composite_rgba(&d);
    assert_slice(&part, &full, 110, 110);
}

#[test]
fn pattern_region_rebuilds_after_pattern_edit() {
    let _guard = crate::layer_source::cache_test_guard();
    crate::purge_source_cache();
    let mut d = doc(
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
            solid(
                "Patterned",
                rect(30, 30, 34, 34),
                (255, 0, 0),
                255,
                BlendMode::Normal,
                255,
            ),
        ],
    );
    assert!(crate::set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.pattern",
        0.0
    ));
    assert!(crate::set_document_layer_style_value(
        &mut d,
        "1",
        "patternOverlay.on",
        1.0
    ));
    let _ = composite_rgba(&d);
    let before = crate::layer_source::pass_build_count(&d.layers[1], &d);
    let first = crate::composite::composite_rgba_region(&d, 30, 30, 4, 4);
    assert_eq!(
        crate::layer_source::pass_build_count(&d.layers[1], &d),
        before + 1,
        "a pattern region rebuilds its pass"
    );

    d.layer_section_extra = Default::default();
    let rgba: Vec<u8> = [0u8, 0, 255, 255].repeat(4);
    let record =
        pictura_codec::encode_rgb_pattern("Edited", "kooka-builtin-pattern-0", 2, 2, &rgba)
            .expect("pattern record");
    assert!(pictura_codec::add_document_pattern(
        &mut d,
        "kooka-builtin-pattern-0",
        &record
    ));
    let before_edit = crate::layer_source::pass_build_count(&d.layers[1], &d);
    let second = crate::composite::composite_rgba_region(&d, 30, 30, 4, 4);
    assert_eq!(
        crate::layer_source::pass_build_count(&d.layers[1], &d),
        before_edit + 1,
        "the pattern edit rebuilds its pass"
    );
    assert_ne!(
        first.data.as_ref(),
        second.data.as_ref(),
        "the pattern edit changes pixels"
    );
    let full = composite_rgba(&d);
    assert_slice(&second, &full, 30, 30);
}
