//! `plan_stack` over the content routes, the depth gate, and the clipping and
//! effect routes. None of these needs a device.

use super::*;
use crate::composite_route::tests::{adjustment, filter, proxy, rect, smart, type_layer};
use crate::tests::layer_effects::{global_shadow_block, spec_block, ShadowSpec};
use pictura_codec::CAMERA_RAW_FILTER_ID;
use pictura_core::{AdjustmentData, Knockout, LayerBlock};

#[test]
fn gpu_eligibility_consumes_the_resolved_content_route() {
    every_declining_route_names_its_reason();
    accepted_routes_do_not_decline();
    proxied_type_layer_with_gamma_is_eligible();
    channel_less_type_layer_is_accepted();
    unrenderable_smart_source_declines();
    decoded_vector_fill_is_accepted();
    adjustment_without_a_gpu_kind_declines_at_any_rect();
}

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
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect: rect(),
        channels: proxy(),
        ..Default::default()
    })
}

fn red_pixels() -> Layer {
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect: rect(),
        channels: vec![
            Channel {
                id: 0,
                data: vec![255, 0, 255, 0].into(),
            },
            Channel {
                id: 1,
                data: vec![0; 4].into(),
            },
            Channel {
                id: 2,
                data: vec![0; 4].into(),
            },
            Channel {
                id: -1,
                data: vec![255; 4].into(),
            },
        ],
        ..Default::default()
    })
}

fn renderable_type() -> Layer {
    let mut layer = type_layer(Vec::new());
    layer.rect = PsdRect {
        top: 0,
        left: 0,
        bottom: 80,
        right: 200,
    };
    pictura_codec::edit_layer(layer.as_mut(), |parts| {
        if let Some(type_tool) = parts.type_tool.as_mut() {
            type_tool.bounds = [0, 0, 200, 80];
            type_tool.style = Some(crate::tests::composite::type_style());
        }
    });
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
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect: rect(),
        extra_blocks: vec![LayerBlock {
            key: *b"vscg",
            data: data.into(),
        }],
        ..Default::default()
    })
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
    let undecoded_vscg = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect: rect(),
        extra_blocks: vec![LayerBlock {
            key: *b"vscg",
            data: vec![0xFF; 3].into(),
        }],
        ..Default::default()
    });
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
fn globally_lit_effects_are_accepted() {
    let mut styled = pixels();
    pictura_codec::edit_layer(&mut styled, |parts| {
        parts.extra_blocks.push(global_shadow_block())
    });
    let mut doc = doc_of(styled);
    doc.global_light = pictura_core::GlobalLight {
        angle: 90,
        altitude: 45,
    };
    assert_eq!(reason(check_supported(&doc)), "ok");
}

#[test]
fn effects_and_clipping_are_accepted() {
    let mut styled = pixels();
    pictura_codec::edit_layer(styled.as_mut(), |parts| {
        parts.extra_blocks.push(spec_block(&ShadowSpec::default()))
    });
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
fn gpu_declines_each_advanced_blending_toggle() {
    let mut cases: Vec<Layer> = Vec::new();
    let mut layer = pixels();
    layer.blend_clipping = false;
    cases.push(layer);
    let mut layer = pixels();
    layer.blend_interior = true;
    cases.push(layer);
    let mut layer = pixels();
    layer.transparency_shapes = false;
    cases.push(layer);
    let mut layer = pixels();
    layer.layer_mask_hides_effects = true;
    cases.push(layer);
    let mut layer = pixels();
    layer.vector_mask_hides_effects = true;
    cases.push(layer);
    let mut layer = pixels();
    layer.restricted_channels = vec![0];
    cases.push(layer);
    for layer in cases {
        assert!(
            matches!(
                check_supported(&doc_of(layer)),
                Err(GpuError::UnsupportedAdvancedBlending)
            ),
            "a non-default advanced-blending flag must decline"
        );
    }

    let mut child = pixels();
    child.transparency_shapes = false;
    let group = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect: rect(),
        is_group: true,
        blend: BlendMode::Normal,
        children: pictura_core::LayerStack::from_layers(vec![child]),
        ..Default::default()
    });
    assert!(matches!(
        check_supported(&doc_of(group)),
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

fn adjustment_of_kind(kind: &str, rect: PsdRect) -> Layer {
    let kind = crate::AdjustmentKind::from_name(kind).expect("a catalogue kind");
    pictura_codec::layer_from_parts(pictura_core::LayerParts {
        rect,
        adjustment: crate::dialog_default(kind, crate::Defaults::rgb()),
        ..Default::default()
    })
}

#[test]
fn adjustment_without_a_gpu_kind_declines_at_any_rect() {
    for kind in [
        "photo-filter",
        "vibrance",
        "color-balance",
        "black-white",
        "channel-mixer",
        "selective-color",
        "shadows-highlights",
        "color-lookup",
    ] {
        for area in [Layer::default().rect, rect()] {
            let doc = doc_layers(vec![pixels(), adjustment_of_kind(kind, area)]);
            assert_eq!(
                reason(check_supported(&doc)),
                "UnsupportedAdjustment",
                "{kind} at {area:?}"
            );
        }
    }
}

/// A kind whose row declares a GPU route still declines when the parameter
/// shape is one the shader does not model — Hue/Saturation with a colour range
/// has no shader form. The row says "this kind has a GPU path", not "every
/// parameter set uses it".
#[test]
fn gpu_route_falls_back_for_unmodelled_parameters() {
    // The Master-only shape is routed on the GPU.
    let modelled = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        adjustment: Some(crate::encode_hue_saturation(10, 0, 0)),
        ..Default::default()
    });
    let doc = doc_layers(vec![pixels(), modelled]);
    assert_eq!(reason(check_supported(&doc)), "ok", "Master only is routed");

    // The same kind carrying a colour-range edit is not.
    let ranged = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        adjustment: Some(hue_saturation_with_a_range()),
        ..Default::default()
    });
    let baseline = doc_layers(vec![red_pixels()]);
    let baseline_cpu = crate::composite_rgba(&baseline);
    let doc = doc_layers(vec![red_pixels(), ranged]);
    assert_eq!(
        reason(check_supported(&doc)),
        "UnsupportedAdjustment",
        "colour ranges are CPU-only"
    );
    let adjusted_cpu = crate::composite_rgba(&doc);
    assert_ne!(
        adjusted_cpu.data, baseline_cpu.data,
        "the CPU fallback applies the ranged edit"
    );
}

/// A `hue2` block whose Reds band carries a Hue edit, so the decoded adjustment
/// has a non-empty `ranges` list.
fn hue_saturation_with_a_range() -> AdjustmentData {
    let mut data = vec![0u8, 2, 1, 0];
    data.extend_from_slice(&[0u8; 6]);
    for v in [0i16, 0, 0] {
        data.extend_from_slice(&v.to_be_bytes());
    }
    for (band, edit) in [(0usize, (10i16, 0i16, 0i16)), (1, (0, 0, 0))]
        .into_iter()
        .chain((2..6).map(|k| (k, (0, 0, 0))))
    {
        let strip: [i16; 4] = match band {
            0 => [315, 345, 15, 45],
            _ => [0; 4],
        };
        for v in strip.into_iter().chain([edit.0, edit.1, edit.2]) {
            data.extend_from_slice(&v.to_be_bytes());
        }
    }
    AdjustmentData {
        key: *b"hue2",
        data: data.into(),
    }
}

#[test]
fn empty_rect_keeps_gpu_adjustments_and_fills_on_the_gpu() {
    for kind in ["brightness-contrast", "levels", "posterize", "threshold"] {
        let doc = doc_layers(vec![
            pixels(),
            adjustment_of_kind(kind, Layer::default().rect),
        ]);
        assert_eq!(reason(check_supported(&doc)), "ok", "{kind}");
    }
    let fill = pictura_codec::layer_from_parts(pictura_core::LayerParts {
        adjustment: Some(crate::encode_solid_color_fill([1, 2, 3])),
        ..Default::default()
    });
    let doc = doc_of(fill);
    let plans = plan(&doc).expect("a fill with no area stays on the GPU");
    assert!(matches!(&plans[0].body, plan::Body::Fill(_)));
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
            crate::composite_request::composite(crate::composite_request::CompositeRequest::whole(
                &doc, false
            ))
            .0,
            out,
            "{name}: the CPU backend is the oracle"
        );
    }
}

#[test]
fn sixteen_bit_document_declines() {
    let mut doc = doc_of(pixels());
    doc.source
        .set_native(Some(BitDepth::Sixteen), doc.source.planes().clone())
        .expect("valid source depth");
    assert!(matches!(
        check_supported(&doc),
        Err(GpuError::UnsupportedDepth)
    ));
}

#[test]
fn thirty_two_bit_active_backend_is_cpu() {
    let mut doc = doc_of(pixels());
    doc.source
        .set_native(Some(BitDepth::ThirtyTwo), doc.source.planes().clone())
        .expect("valid source depth");
    let (buf, backend) = crate::composite_request::composite(
        crate::composite_request::CompositeRequest::whole(&doc, true),
    );
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(buf, crate::composite_rgba(&doc));
}

#[test]
fn eight_bit_document_stays_eligible() {
    let doc = doc_of(pixels());
    assert!(doc.source.depth().is_none());
    assert!(check_supported(&doc).is_ok());
}

#[test]
fn validation_runs_at_most_once() {
    let _ = gpu_available();
    let _ = gpu_available();
    assert!(VALIDATION_RUNS.load(std::sync::atomic::Ordering::Relaxed) <= 1);
}

#[test]
#[cfg(feature = "software-gpu")]
fn software_gpu_accepts_a_cpu_device() {
    assert!(backend::accelerates(wgpu::DeviceType::Cpu));
}

#[test]
#[should_panic(expected = "no usable GPU device")]
fn require_gpu_panics_without_a_device() {
    require_device(false);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn gpu_is_available_under_a_software_device() {
    require_gpu();
    let doc = validation_document();
    let (buf, backend) = crate::composite_request::composite(
        crate::composite_request::CompositeRequest::whole(&doc, true),
    );
    assert_eq!(backend, Backend::Gpu);
    let cpu = crate::composite_rgba(&doc);
    assert!(buf
        .data
        .iter()
        .zip(cpu.data.iter())
        .all(|(g, c)| g.abs_diff(*c) <= 1));
}
