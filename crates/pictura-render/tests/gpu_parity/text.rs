//! Proxied type layers through the text-gamma curve.

use pictura_core::{BitDepth, BlendMode, ColorMode, Document, LayerMask, PsdRect, TypeTool};
use pictura_render::{
    composite_active, metrics, set_text_gamma, text_gamma, Backend, DEFAULT_TEXT_GAMMA,
};

use crate::common::{base_layer, check_scene_parity, gamma_guard, group, layer, require_gpu, SIZE};

fn type_tool() -> TypeTool {
    TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "Hi".into(),
        bounds: [0, 0, SIZE as i32, SIZE as i32],
        text_desc: Vec::new().into(),
        warp_desc: Vec::new().into(),
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
        path: None,
    }
}

fn whole_rect() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: SIZE as i32,
        right: SIZE as i32,
    }
}

fn alpha_ramp(x: u32, y: u32) -> u8 {
    let i = y * SIZE + x;
    (i * 255 / (SIZE * SIZE - 1)) as u8
}

/// An opaque backdrop under a proxied black type layer whose alpha ramps 0…255
/// across the layer, so the curve is exercised across its input range.
fn type_scene() -> Document {
    blend_type_scene(BlendMode::Normal)
}

/// The same scene with the type layer's own blend mode set to `blend`.
fn blend_type_scene(blend: BlendMode) -> Document {
    let mut text = layer("text", blend, |x, y| (0, 0, 0, alpha_ramp(x, y)));
    text.type_tool = Some(type_tool());
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), text]);
    doc
}

fn gated_scene() -> Document {
    let mut doc = type_scene();
    let text = &mut doc.layers[1];
    text.opacity = 180;
    text.fill = 96;
    text.mask = Some(LayerMask {
        rect: whole_rect(),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some((0..SIZE * SIZE).map(|i| (i * 37 % 256) as u8).collect()),
        ..Default::default()
    });
    doc
}

/// A proxied type child inside a pass-through or isolated group over a backdrop.
fn group_type_scene(pass_through: bool) -> Document {
    let mut text = layer("text", BlendMode::Normal, |x, y| {
        (0, 0, 0, alpha_ramp(x, y))
    });
    text.type_tool = Some(type_tool());
    let (blend, opacity) = if pass_through {
        (BlendMode::PassThrough, 255)
    } else {
        (BlendMode::Multiply, 200)
    };
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        base_layer(),
        group("type group", blend, opacity, vec![text]),
    ]);
    doc
}

fn assert_parity(doc: &Document, max_delta: &mut i32) {
    check_scene_parity(doc, max_delta);
    assert_eq!(
        composite_active(doc, true).1,
        Backend::Gpu,
        "a proxied type layer must stay on the GPU"
    );
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn proxied_type_layer_at_default_gamma_matches_cpu() {
    let _guard = gamma_guard();
    require_gpu();
    let old = text_gamma();
    set_text_gamma(Some(DEFAULT_TEXT_GAMMA));
    let doc = type_scene();
    let before = metrics::decline_count("type_source");
    let mut max_delta = 0;
    assert_parity(&doc, &mut max_delta);
    assert_eq!(
        metrics::decline_count("type_source"),
        before,
        "a proxied type layer must not count a type-source decline"
    );
    set_text_gamma(old);
    eprintln!("proxied type default-gamma parity: max delta {max_delta} LSB");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn proxied_type_gamma_values_and_gates_match_cpu() {
    let _guard = gamma_guard();
    require_gpu();
    let old = text_gamma();
    let mut max_delta = 0;
    for gamma in [None, Some(1.0), Some(DEFAULT_TEXT_GAMMA), Some(2.2)] {
        set_text_gamma(gamma);
        for doc in [type_scene(), gated_scene()] {
            assert_parity(&doc, &mut max_delta);
        }
    }
    set_text_gamma(old);
    eprintln!("proxied type gamma/gate parity: max delta {max_delta} LSB");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn proxied_type_in_groups_matches_cpu() {
    let _guard = gamma_guard();
    require_gpu();
    let old = text_gamma();
    let mut max_delta = 0;
    for gamma in [None, Some(1.0), Some(DEFAULT_TEXT_GAMMA), Some(2.2)] {
        set_text_gamma(gamma);
        for doc in [group_type_scene(false), group_type_scene(true)] {
            assert_parity(&doc, &mut max_delta);
        }
    }
    set_text_gamma(old);
    eprintln!("proxied type group parity: max delta {max_delta} LSB");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn proxied_type_blend_modes_match_cpu() {
    let _guard = gamma_guard();
    require_gpu();
    let old = text_gamma();
    set_text_gamma(Some(DEFAULT_TEXT_GAMMA));
    let mut max_delta = 0;
    for blend in [BlendMode::Multiply, BlendMode::Hue] {
        assert_parity(&blend_type_scene(blend), &mut max_delta);
    }
    set_text_gamma(old);
    eprintln!("proxied type blend-mode parity: max delta {max_delta} LSB");
}
