//! GPU oracle error assertions moved from the external `gpu_parity` suite so
//! `composite_gpu` can stay `pub(crate)`. Device-free declines run in the
//! default job; the oversized case keeps its `#[ignore]` reason.

use super::*;
use pictura_core::{AdjustmentData, LayerStack};

const SIZE: u32 = 8;

fn layer(name: &str, blend: BlendMode) -> Layer {
    let pixels = (SIZE * SIZE) as usize;
    let channels = vec![
        Channel {
            id: 0,
            data: vec![128u8; pixels].into(),
        },
        Channel {
            id: 1,
            data: vec![96u8; pixels].into(),
        },
        Channel {
            id: 2,
            data: vec![64u8; pixels].into(),
        },
        Channel {
            id: -1,
            data: vec![255u8; pixels].into(),
        },
    ];
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        blend,
        opacity: 255,
        fill: 255,
        visible: true,
        channels,
        ..Default::default()
    }
}

fn scene(blend: BlendMode) -> Document {
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers =
        LayerStack::from_layers(vec![layer("base", BlendMode::Normal), layer("top", blend)]);
    doc
}

fn adjustment_layer(name: &str, adjustment: AdjustmentData) -> Layer {
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        visible: true,
        adjustment: Some(adjustment),
        ..Default::default()
    }
}

#[test]
fn dissolve_still_cpu_only() {
    assert!(matches!(
        composite_gpu(&scene(BlendMode::Dissolve)),
        Err(GpuError::UnsupportedMode(BlendMode::Dissolve))
    ));
}

#[test]
fn cpu_only_modes_error_without_panicking() {
    for &mode in &[BlendMode::Dissolve] {
        match composite_gpu(&scene(mode)) {
            Err(GpuError::UnsupportedMode(m)) => assert_eq!(m, mode),
            other => panic!("{mode:?}: expected UnsupportedMode, got {other:?}"),
        }
    }
}

#[test]
fn unsupported_adjustment_falls_back_or_errors() {
    let cases = [
        AdjustmentData {
            key: *b"clrL",
            data: vec![1, 2, 3].into(),
        },
        AdjustmentData {
            key: *b"selc",
            data: vec![1, 2, 3].into(),
        },
    ];
    for data in cases {
        let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = LayerStack::from_layers(vec![
            layer("base", BlendMode::Normal),
            adjustment_layer("unsupported", data),
        ]);
        assert!(matches!(
            composite_gpu(&doc),
            Err(GpuError::UnsupportedAdjustment)
        ));
        let (buffer, backend) = crate::composite(crate::CompositeRequest::whole(&doc, true));
        assert_eq!(backend, Backend::Cpu);
        assert_eq!(buffer.data, crate::composite_rgba(&doc).data);
    }
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn document_past_2d_product_limit_returns_too_large() {
    require_gpu();
    let doc = Document {
        width: u32::MAX,
        height: u32::MAX,
        mode: ColorMode::Rgb,
        depth: BitDepth::Eight,
        composite: PixelBuffer::new(0, 0, 3),
        layers: LayerStack::default(),
        channels: Vec::new(),
        ..Default::default()
    };
    assert!(matches!(composite_gpu(&doc), Err(GpuError::TooLarge)));
}

/// #1290: the region estimate counts the fixed buffers and each layer's source
/// and mask, and an isolated group's inner canvas.
#[test]
fn the_region_estimate_counts_every_upload() {
    let doc = scene(BlendMode::Normal);
    let plans = plan::plan_stack(&doc, None).expect("plannable");
    assert_eq!(region_host_bytes(&plans, SIZE, SIZE), 64 * (16 + 2 * 5));
    let mut grouped = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    grouped.layers = LayerStack::from_layers(vec![Layer {
        name: "group".into(),
        is_group: true,
        visible: true,
        opacity: 255,
        fill: 255,
        blend: BlendMode::Normal,
        children: LayerStack::from_layers(vec![
            layer("a", BlendMode::Normal),
            layer("b", BlendMode::Normal),
        ]),
        ..Default::default()
    }]);
    let plans = plan::plan_stack(&grouped, None).expect("plannable");
    let per_pixel = region_host_bytes(&plans, 1, 1) - 16;
    assert!(per_pixel >= 2 * 5, "both children counted: {per_pixel}");
}

/// #1290: a region whose host buffers exceed the governor's room declines
/// after the device limits are checked and before any region-sized buffer,
/// so the CPU composites it in bands; nothing is recorded as a refusal.
#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn a_region_over_the_memory_room_declines_to_the_cpu() {
    use pictura_core::memory::{self, HostReading};
    require_gpu();
    let doc = scene(BlendMode::Normal);
    let plans = plan::plan_stack(&doc, None).expect("plannable");
    memory::set_host_for_test(Some(HostReading {
        available: Some(0),
        ..HostReading::default()
    }));
    memory::clear_refusal();
    let result = composite_gpu_region_raw(&doc, &plans, 0, 0, SIZE, SIZE);
    let refusal = memory::last_refusal();
    memory::set_host_for_test(None);
    assert!(matches!(result, Err(GpuError::Memory)), "{result:?}");
    assert_eq!(refusal, None);
}
