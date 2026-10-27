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
