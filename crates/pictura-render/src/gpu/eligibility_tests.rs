//! `plan_stack` over the content routes, the depth gate, and the clipping and
//! effect routes. None of these needs a device.

use super::*;
use crate::composite_route::tests::{adjustment, filter, proxy, rect, smart, type_layer};
use crate::tests::layer_effects::{spec_block, ShadowSpec};
use pictura_codec::CAMERA_RAW_FILTER_ID;
use pictura_core::{AdjustmentData, Knockout, LayerBlock};

fn doc_of(layer: Layer) -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

fn doc_layers(layers: Vec<Layer>) -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(layers);
    doc
}

fn doc_sized(layer: Layer, width: u32, height: u32) -> Document {
    let mut doc = Document::new(width, height, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

fn pixels() -> Layer {
    Layer {
        rect: rect(),
        channels: proxy(),
        ..Default::default()
    }
}

fn renderable_type() -> Layer {
    let mut layer = type_layer(Vec::new());
    layer.rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 80,
        right: 200,
    };
    if let Some(type_tool) = layer.type_tool.as_mut() {
        type_tool.bounds = [0, 0, 200, 80];
        type_tool.style = Some(crate::tests::composite::type_style());
    }
    layer
}

fn levels_block() -> AdjustmentData {
    let mut data = vec![0, 2];
    for v in [5u16, 250, 10, 240, 120] {
        data.extend_from_slice(&v.to_be_bytes());
    }
    AdjustmentData {
        key: *b"levl",
        data: data.into(),
    }
}

fn vector_fill_layer() -> Layer {
    let fill = crate::encode_solid_color_fill([1, 2, 3]);
    let mut data = b"SoCo".to_vec();
    data.extend_from_slice(&fill.data);
    Layer {
        rect: rect(),
        extra_blocks: vec![LayerBlock {
            key: *b"vscg",
            data: data.into(),
        }],
        ..Default::default()
    }
}

fn reason(err: Result<(), GpuError>) -> String {
    match err {
        Ok(()) => "ok".into(),
        Err(e) => format!("{e:?}"),
    }
}

fn plan(doc: &Document) -> Result<Vec<plan::LayerPlan<'_>>, GpuError> {
    plan::plan_stack(doc, None)
}

#[test]
fn every_declining_route_names_its_reason() {
    let undecoded_vscg = Layer {
        rect: rect(),
        extra_blocks: vec![LayerBlock {
            key: *b"vscg",
            data: vec![0xFF; 3].into(),
        }],
        ..Default::default()
    };
    let cases = [
        (adjustment(*b"zzzz", vec![1, 2, 3]), "UnsupportedAdjustment"),
        (undecoded_vscg, "UnsupportedAdjustment"),
    ];
    for (layer, expected) in cases {
        let doc = doc_of(layer);
        assert_eq!(reason(check_supported(&doc)), expected);
    }
}

#[test]
fn accepted_routes_do_not_decline() {
    let mut clipped = pixels();
    clipped.clipping = true;
    let cases = vec![
        doc_of(pixels()),
        doc_of(adjustment(*b"nvrt", Vec::new())),
        doc_of(adjustment(*b"levl", levels_block().data.to_vec())),
        doc_of(vector_fill_layer()),
        doc_of(type_layer(proxy())),
        doc_of(renderable_type()),
        doc_of(smart(proxy(), Vec::new(), true)),
        doc_layers(vec![pixels(), clipped]),
    ];
    for doc in cases {
        assert_eq!(reason(check_supported(&doc)), "ok");
    }
}

#[test]
fn proxied_type_layer_with_gamma_is_eligible() {
    assert!(
        crate::text_gamma::text_gamma().is_some(),
        "the default gamma is on"
    );
    assert_eq!(reason(check_supported(&doc_of(type_layer(proxy())))), "ok");
}

#[test]
fn channel_less_type_layer_is_accepted() {
    let doc = doc_of(renderable_type());
    assert_eq!(reason(check_supported(&doc)), "ok");
    let plans = plan(&doc).expect("accepted");
    assert!(
        matches!(&plans[0].body, plan::Body::Raster(_)),
        "a proxy-less type layer takes the cached raster route"
    );
}

#[test]
fn unrenderable_type_source_declines() {
    let doc = doc_of(type_layer(Vec::new()));
    assert!(matches!(
        composite_gpu(&doc),
        Err(GpuError::UnsupportedText)
    ));
}

#[test]
fn unrenderable_smart_source_declines() {
    let doc = doc_of(smart(Vec::new(), Vec::new(), true));
    assert!(matches!(
        check_supported(&doc),
        Err(GpuError::UnsupportedSmartObject)
    ));
}

#[test]
fn decoded_vector_fill_is_accepted() {
    let doc = doc_of(vector_fill_layer());
    assert_eq!(reason(check_supported(&doc)), "ok");
    let plans = plan(&doc).expect("accepted");
    assert!(matches!(&plans[0].body, plan::Body::Fill(_)));
}

#[test]
fn effects_and_clipping_are_accepted() {
    let mut styled = pixels();
    styled.extra_blocks.push(spec_block(&ShadowSpec::default()));
    assert_eq!(reason(check_supported(&doc_of(styled))), "ok");

    let mut clipped = pixels();
    clipped.clipping = true;
    let doc = doc_layers(vec![pixels(), clipped]);
    assert_eq!(reason(check_supported(&doc)), "ok");
    let plans = plan(&doc).expect("accepted");
    assert!(matches!(&plans[0].body, plan::Body::ClipGroup { .. }));
}

#[test]
fn advanced_blending_still_declines() {
    let mut layer = pixels();
    layer.knockout = Knockout::Deep;
    assert!(matches!(
        check_supported(&doc_of(layer)),
        Err(GpuError::UnsupportedAdvancedBlending)
    ));
}

#[test]
fn hidden_layer_is_not_planned() {
    let mut hidden = pixels();
    hidden.visible = false;
    hidden.blend = BlendMode::Dissolve;
    let doc = doc_of(hidden);
    assert_eq!(reason(check_supported(&doc)), "ok");
    assert!(plan(&doc).expect("accepted").is_empty());
}

#[test]
fn hidden_clipping_base_suppresses_run() {
    let mut base = pixels();
    base.visible = false;
    let mut clipped = pixels();
    clipped.clipping = true;
    let doc = doc_layers(vec![base, clipped]);
    assert_eq!(reason(check_supported(&doc)), "ok");
    assert!(plan(&doc).expect("accepted").is_empty());
}

#[test]
fn plan_is_device_independent() {
    let doc = doc_of(pixels());
    let plans = plan(&doc).expect("planning must not require a device");
    assert_eq!(plans.len(), 1);
    assert!(matches!(&plans[0].body, plan::Body::Pixels));
    let before = plans.len();
    let _ = crate::gpu_available();
    assert_eq!(
        plan(&doc).expect("planning after device init").len(),
        before,
        "planning does not depend on a device"
    );
    let declined = doc_of(adjustment(*b"zzzz", vec![1, 2, 3]));
    assert!(matches!(
        plan(&declined),
        Err(GpuError::UnsupportedAdjustment)
    ));
}

#[test]
fn plan_and_dispatch_agree() {
    let accepted = [
        (doc_of(pixels()), "pixels"),
        (doc_of(adjustment(*b"nvrt", Vec::new())), "adjustment"),
        (
            doc_of(adjustment(*b"levl", levels_block().data.to_vec())),
            "lut",
        ),
        (doc_of(vector_fill_layer()), "fill"),
        (doc_of(renderable_type()), "raster"),
    ];
    for (doc, expected) in accepted {
        let plans = plan(&doc).expect("accepted");
        let kind = match &plans[0].body {
            plan::Body::Pixels => "pixels",
            plan::Body::Raster(_) => "raster",
            plan::Body::Fill(_) => "fill",
            plan::Body::Adjustment(plan::AdjustmentPlan::Inline(_)) => "adjustment",
            plan::Body::Adjustment(plan::AdjustmentPlan::Lut(_)) => "lut",
            plan::Body::Group { .. } => "group",
            plan::Body::ClipGroup { .. } => "clip",
            plan::Body::Decline(_) => "decline",
        };
        assert_eq!(kind, expected);
        match composite_gpu(&doc) {
            Ok(_) => {}
            Err(GpuError::Unavailable) => assert!(
                !crate::gpu_available(),
                "{expected}: a device is available but dispatch reported unavailable"
            ),
            Err(e) => panic!("{expected}: plan accepted but dispatch declined: {e}"),
        }
    }
    let declined = doc_of(adjustment(*b"zzzz", vec![1, 2, 3]));
    assert!(matches!(
        composite_gpu(&declined),
        Err(GpuError::UnsupportedAdjustment)
    ));
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[test]
fn cpu_oracle_unchanged_for_every_route() {
    let docs = [
        ("pixels", doc_of(pixels())),
        ("invert", doc_of(adjustment(*b"nvrt", Vec::new()))),
        ("vector_fill", doc_of(vector_fill_layer())),
        ("type", doc_sized(renderable_type(), 200, 80)),
        (
            "smart_filtered",
            doc_of(smart(
                proxy(),
                vec![filter(CAMERA_RAW_FILTER_ID, true)],
                true,
            )),
        ),
    ];
    for (name, doc) in docs {
        let out = crate::composite_rgba(&doc);
        let baseline = match name {
            "pixels" => 7455481069867015845,
            "invert" => 9808874869469701221,
            "vector_fill" => 13231920238882543897,
            "type" => 13681371586364879357,
            "smart_filtered" => 7455481069867015845,
            _ => unreachable!(),
        };
        assert_eq!(
            fnv1a(out.data.as_ref()),
            baseline,
            "{name}: the CPU oracle bytes changed"
        );
        assert_eq!(
            crate::composite_active(&doc, false).0,
            out,
            "{name}: the CPU backend is the oracle"
        );
    }
}

#[test]
fn sixteen_bit_document_declines() {
    let mut doc = doc_of(pixels());
    doc.source_depth = Some(BitDepth::Sixteen);
    assert!(matches!(
        check_supported(&doc),
        Err(GpuError::UnsupportedDepth)
    ));
}

#[test]
fn thirty_two_bit_active_backend_is_cpu() {
    let mut doc = doc_of(pixels());
    doc.source_depth = Some(BitDepth::ThirtyTwo);
    let (buf, backend) = composite_active(&doc, true);
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(buf, crate::composite_rgba(&doc));
}

#[test]
fn eight_bit_document_stays_eligible() {
    let doc = doc_of(pixels());
    assert!(doc.source_depth.is_none());
    assert!(check_supported(&doc).is_ok());
}

#[test]
fn validation_runs_at_most_once() {
    let _ = gpu_available();
    let _ = gpu_available();
    assert!(VALIDATION_RUNS.load(std::sync::atomic::Ordering::Relaxed) <= 1);
}

#[test]
#[cfg(feature = "software-vulkan")]
fn software_vulkan_accepts_a_cpu_device() {
    assert!(backend::accelerates(wgpu::DeviceType::Cpu));
}

#[test]
#[should_panic(expected = "no usable GPU device")]
fn require_gpu_panics_without_a_device() {
    require_device(false);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn gpu_is_available_under_software_vulkan() {
    require_gpu();
    let doc = validation_document();
    let (buf, backend) = composite_active(&doc, true);
    assert_eq!(backend, Backend::Gpu);
    let cpu = crate::composite_rgba(&doc);
    assert!(buf
        .data
        .iter()
        .zip(cpu.data.iter())
        .all(|(g, c)| g.abs_diff(*c) <= 1));
}
