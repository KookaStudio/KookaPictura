//! Group, mask, fill, and adjustment-layer parity tests.

use pictura_core::{
    AdjustmentData, BitDepth, BlendMode, ColorMode, Document, LayerMask, PsdRect, VectorFillRule,
    VectorMask, VectorSubpath,
};
use pictura_render::{
    composite_gpu, composite_gpu_or_cpu, composite_rgba, encode_invert, GpuError,
};

use crate::common::{
    adjustment_layer, base_layer, check_scene_parity, group, layer, require_gpu, SIZE,
};

/// The data path's mask plane and packed-group source are exercised here; the
/// blend/adjustment tests above never bind either.
#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn groups_and_masks_match_cpu() {
    require_gpu();
    let mut max_delta = 0i32;

    // Masked, reduced-opacity pixel layer over an opaque backdrop.
    let mut masked = layer("masked", BlendMode::Multiply, |x, y| {
        (x as u8 * 16 + 8, y as u8 * 16 + 8, 255 - x as u8 * 16, 200)
    });
    masked.opacity = 180;
    masked.mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some((0..SIZE * SIZE).map(|i| (i * 37 % 256) as u8).collect()),
        ..Default::default()
    });
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), masked]);
    check_scene_parity(&doc, &mut max_delta);

    // Isolated group (packed inner canvas source) over the backdrop.
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        base_layer(),
        group(
            "iso",
            BlendMode::Multiply,
            200,
            vec![layer("child", BlendMode::Normal, |x, y| {
                (255 - x as u8 * 20, 40 + y as u8 * 20, x as u8 * 32, 255)
            })],
        ),
    ]);
    check_scene_parity(&doc, &mut max_delta);

    // Adjustment layer gated by a mask.
    let mut adj = adjustment_layer("invert", encode_invert());
    adj.mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        default_color: 255,
        disabled: false,
        flags: 0,
        data: Some(
            (0..SIZE * SIZE)
                .map(|i| (255 - i * 11 % 256) as u8)
                .collect(),
        ),
        ..Default::default()
    });
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), adj]);
    check_scene_parity(&doc, &mut max_delta);

    // Pixel layer clipped by a document-relative vector mask (per-pixel plane).
    let mut shaped = layer("shaped", BlendMode::Normal, |x, y| {
        (x as u8 * 20, y as u8 * 20, 128, 255)
    });
    let edge = 4 * 256;
    shaped.vector_mask = Some(VectorMask {
        subpaths: vec![VectorSubpath {
            closed: true,
            operation: 1,
            fill_rule: VectorFillRule::EvenOdd,
            points: vec![[0, 0], [edge, 0], [edge, edge], [0, edge]],
        }],
        invert: false,
        disabled: false,
        ..Default::default()
    });
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), shaped]);
    check_scene_parity(&doc, &mut max_delta);

    eprintln!("group/mask parity: max delta {max_delta} LSB");
}

/// A reduced-fill layer must match the CPU oracle within ±1 LSB; over an opaque
/// backdrop the fill factor scales colour only, so the output alpha stays 255.
#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn fill_scene_matches_cpu() {
    require_gpu();
    let mut top = layer("fill", BlendMode::Multiply, |x, y| {
        (x as u8 * 16 + 8, y as u8 * 16 + 8, 255 - x as u8 * 16, 200)
    });
    top.opacity = 180;
    top.fill = 96;
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), top]);

    let mut max_delta = 0i32;
    check_scene_parity(&doc, &mut max_delta);

    let cpu = composite_rgba(&doc);
    let plane = cpu.pixel_count();
    for i in 0..plane {
        assert_eq!(
            cpu.data[3 * plane + i],
            255,
            "fill must not change the alpha over an opaque backdrop at {i}"
        );
    }
    eprintln!("fill parity: max delta {max_delta} LSB, alpha unchanged");
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn adjustment_layers_match_cpu() {
    require_gpu();
    use pictura_render::{
        encode_brightness_contrast, encode_hue_saturation, encode_invert, encode_posterize,
        encode_threshold,
    };
    let cases: &[(&str, AdjustmentData)] = &[
        ("invert", encode_invert()),
        ("posterize", encode_posterize(4)),
        ("threshold", encode_threshold(128)),
        ("brightness_contrast", encode_brightness_contrast(20, 0)),
        ("hue_saturation", encode_hue_saturation(30, 0, 0)),
    ];
    let mut checked = 0;
    let mut max_delta = 0i32;
    for (name, data) in cases {
        let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            base_layer(),
            adjustment_layer(name, data.clone()),
        ]);
        let cpu = composite_rgba(&doc);
        let gpu = match composite_gpu(&doc) {
            Ok(buf) => buf,
            Err(e) => panic!("{name}: {e}"),
        };
        assert_eq!((gpu.width, gpu.height, gpu.channels), (SIZE, SIZE, 4));
        let mut case_delta = 0i32;
        for (i, (&a, &b)) in cpu.data.iter().zip(&gpu.data).enumerate() {
            let delta = (a as i32 - b as i32).abs();
            case_delta = case_delta.max(delta);
            max_delta = max_delta.max(delta);
            assert!(delta <= 1, "{name} byte {i}: cpu {a} vs gpu {b}");
        }
        eprintln!("adjustment {name}: max delta {case_delta} LSB");
        checked += 1;
    }
    eprintln!("adjustment parity: {checked} kinds ok, max delta {max_delta} LSB");
}

#[test]
fn unsupported_adjustment_falls_back_or_errors() {
    // A key the renderer does not decode, and one that decodes but the GPU has
    // no kind for (Levels): both must error and fall back without panicking.
    let mut levels = vec![0, 2];
    for v in [5u16, 250, 10, 240, 120] {
        levels.extend_from_slice(&v.to_be_bytes());
    }
    let cases = [
        AdjustmentData {
            key: *b"clrL",
            data: vec![1, 2, 3].into(),
        },
        AdjustmentData {
            key: *b"levl",
            data: levels.into(),
        },
    ];
    for data in cases {
        let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![
            base_layer(),
            adjustment_layer("unsupported", data),
        ]);
        assert!(
            matches!(composite_gpu(&doc), Err(GpuError::UnsupportedAdjustment)),
            "expected UnsupportedAdjustment"
        );
        assert_eq!(
            composite_gpu_or_cpu(&doc).data,
            composite_rgba(&doc).data,
            "fallback must equal the CPU oracle"
        );
    }
}
