//! Blend Text Colors Using Gamma at its default (on, 1.45). The off case runs
//! in its own test binary (`tests/text_gamma_off.rs`): the setting is process
//! wide, and these tests read the default.

use super::*;
use crate::text_gamma::{text_gamma_alpha, DEFAULT_TEXT_GAMMA};
use pictura_core::TypeTool;

fn tool() -> Option<TypeTool> {
    Some(TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "Hi".into(),
        bounds: [0, 0, 1, 1],
        text_desc: Vec::new(),
        warp_desc: Vec::new(),
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
    layer.channels[3].data = (0..=255u8).collect::<Vec<_>>().into();
    layer.type_tool = type_tool;
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
    ink.type_tool = tool();
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
