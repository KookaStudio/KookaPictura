//! Blend Text Colors Using Gamma at its default (on, 1.45). The off case runs
//! in its own test binary (`tests/text_gamma_off.rs`): the setting is process
//! wide, and these tests read the default.

use super::*;
#[cfg(feature = "gpu")]
use crate::gpu::composite_gpu;
use crate::text_gamma::{text_gamma_alpha, DEFAULT_TEXT_GAMMA};
use pictura_core::TypeTool;

fn tool() -> Option<TypeTool> {
    Some(TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "Hi".into(),
        bounds: [0, 0, 1, 1],
        text_desc: Vec::new().into(),
        warp_desc: Vec::new().into(),
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
        path: None,
    })
}

fn white(w: u32, h: u32) -> Layer {
    solid(
        "white",
        full(w, h),
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    )
}

/// A black layer over `full(256, 1)` whose alpha ramps 0…255.
fn ramp(type_tool: Option<TypeTool>) -> Layer {
    let mut layer = solid("ink", full(256, 1), (0, 0, 0), 0, BlendMode::Normal, 255);
    layer.channels_mut()[3].data = (0..=255u8).collect::<Vec<_>>().into();
    pictura_codec::edit_layer(layer.as_mut(), |parts| parts.type_tool = type_tool);
    layer
}

#[test]
fn type_edges_thin_by_default() {
    let _gamma = crate::test_support::gamma_guard();
    let typed = composite_rgba(&doc(256, 1, vec![white(256, 1), ramp(tool())]));
    let plain = composite_rgba(&doc(256, 1, vec![white(256, 1), ramp(None)]));
    let mut lighter = 0;
    for x in 0..256 {
        let (t, p) = (px(&typed, x, 0)[0], px(&plain, x, 0)[0]);
        if x == 0 || x == 255 {
            assert_eq!(t, p, "solid and empty pixels are unchanged at {x}");
        }
        assert!(
            t >= p,
            "type is no darker than the plain blend at {x}: {t} < {p}"
        );
        lighter += usize::from(t > p);
    }
    assert!(lighter > 0, "the curve thins some edge pixel");
}

#[test]
fn opacity_passes_through_the_curve() {
    let _gamma = crate::test_support::gamma_guard();
    let mut ink = solid("ink", full(1, 1), (0, 0, 0), 255, BlendMode::Normal, 128);
    pictura_codec::edit_layer(ink.as_mut(), |parts| parts.type_tool = tool());
    let out = composite_rgba(&doc(1, 1, vec![white(1, 1), ink]));
    let curved = text_gamma_alpha(128.0 / 255.0, DEFAULT_TEXT_GAMMA);
    let expected = (255.0 * (1.0 - curved)).round() as i32;
    let got = i32::from(px(&out, 0, 0)[0]);
    assert!(
        (got - expected).abs() <= 1,
        "got {got}, expected {expected}"
    );
    assert!(got > 128, "a curved 50 % is lighter than the plain blend");
}

#[test]
fn pixel_layers_skip_the_curve() {
    let ink = solid("ink", full(1, 1), (0, 0, 0), 128, BlendMode::Normal, 255);
    let out = composite_rgba(&doc(1, 1, vec![white(1, 1), ink]));
    assert_eq!(px(&out, 0, 0), [127, 127, 127, 255]);
}

#[test]
#[cfg(feature = "gpu")]
fn gpu_accepts_proxied_type_while_gamma_is_on() {
    let _gamma = crate::test_support::gamma_guard();
    assert!(
        crate::text_gamma::text_gamma().is_some(),
        "the default gamma is on"
    );
    let d = doc(256, 1, vec![white(256, 1), ramp(tool())]);
    assert!(matches!(
        composite_gpu(&d),
        Ok(_) | Err(GpuError::Unavailable)
    ));
}

#[test]
fn retained_parsed_type_keeps_its_blending_attribute() {
    let _gamma = crate::test_support::gamma_guard();
    let mut fill = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect: full(1, 1),
        opacity: 128,
        adjustment: Some(crate::encode_solid_color_fill([0; 3])),
        type_tool: tool(),
        ..Default::default()
    });
    assert_eq!(fill.kind(), pictura_core::LayerKind::Fill);
    assert!(fill.type_tool().is_none() && fill.fallback_type_tool().is_some());
    let curved = text_gamma_alpha(128.0 / 255.0, DEFAULT_TEXT_GAMMA);
    let expected = (255.0 * (1.0 - curved)).round() as i32;
    let rendered = composite_rgba(&doc(1, 1, vec![white(1, 1), fill.clone()]));
    assert!((i32::from(px(&rendered, 0, 0)[0]) - expected).abs() <= 1);
    pictura_codec::edit_layer(&mut fill, |parts| {
        parts.type_tool = None;
        parts.extra_blocks.push(pictura_core::LayerBlock {
            key: *b"TySh",
            data: vec![255; 3].into(),
        });
    });
    let opaque = composite_rgba(&doc(1, 1, vec![white(1, 1), fill]));
    assert_eq!(px(&opaque, 0, 0), [127, 127, 127, 255]);

    let mut smart = solid("smart", full(1, 1), (0, 0, 0), 255, BlendMode::Normal, 128);
    pictura_codec::edit_layer(&mut smart, |parts| {
        parts.smart_object = Some(pictura_core::SmartObject::default());
        parts.type_tool = tool();
    });
    assert_eq!(smart.kind(), pictura_core::LayerKind::SmartObject);
    let rendered = composite_rgba(&doc(1, 1, vec![white(1, 1), smart]));
    assert!((i32::from(px(&rendered, 0, 0)[0]) - expected).abs() <= 1);
}
