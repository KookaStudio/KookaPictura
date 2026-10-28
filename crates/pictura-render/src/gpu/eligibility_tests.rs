//! `check_supported` over the content routes, the depth gate, and the
//! smart-filter route. None of these needs a device.

use super::*;
use crate::composite_route::tests::{adjustment, filter, proxy, rect, smart, type_layer};
use pictura_codec::CAMERA_RAW_FILTER_ID;
use pictura_core::LayerBlock;

fn doc_of(layer: Layer) -> Document {
    let mut doc = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
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

fn reason(err: Result<(), GpuError>) -> String {
    match err {
        Ok(()) => "ok".into(),
        Err(e) => format!("{e:?}"),
    }
}

#[test]
fn every_declining_route_names_its_reason() {
    let undecoded_vscg = Layer {
        rect: rect(),
        extra_blocks: vec![LayerBlock {
            key: *b"vscg",
            data: vec![0xFF; 3],
        }],
        ..Default::default()
    };
    let solid_fill = adjustment(*b"SoCo", vec![255, 0, 0, 255]);
    let cases = [
        (adjustment(*b"zzzz", vec![1, 2, 3]), "UnsupportedAdjustment"),
        (undecoded_vscg, "UnsupportedAdjustment"),
        (solid_fill, "UnsupportedAdjustment"),
        (
            smart(proxy(), vec![filter(CAMERA_RAW_FILTER_ID, true)], true),
            "UnsupportedSmartFilter",
        ),
        (
            smart(Vec::new(), Vec::new(), true),
            "UnsupportedSmartObject",
        ),
        (type_layer(Vec::new()), "UnsupportedText"),
    ];
    for (layer, expected) in cases {
        let doc = doc_of(layer);
        assert_eq!(reason(check_supported(&doc)), expected);
    }
    for accepted in [
        adjustment(*b"nvrt", Vec::new()),
        pixels(),
        type_layer(proxy()),
    ] {
        assert_eq!(reason(check_supported(&doc_of(accepted))), "ok");
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
fn proxy_less_type_layer_declines_as_type_source() {
    let doc = doc_of(type_layer(Vec::new()));
    let before = crate::metrics::decline_count("type_source");
    assert!(matches!(
        composite_gpu(&doc),
        Err(GpuError::UnsupportedText)
    ));
    assert!(crate::metrics::decline_count("type_source") > before);
    let (buf, backend) = composite_active(&doc, true);
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(buf, crate::composite_rgba(&doc));
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
fn enabled_smart_filter_declines() {
    let layer = smart(proxy(), vec![filter(CAMERA_RAW_FILTER_ID, true)], true);
    assert!(matches!(
        check_supported(&doc_of(layer)),
        Err(GpuError::UnsupportedSmartFilter)
    ));
}

#[test]
fn disabled_filter_group_still_declines() {
    let layer = smart(proxy(), vec![filter(CAMERA_RAW_FILTER_ID, true)], false);
    assert!(matches!(
        check_supported(&doc_of(layer)),
        Err(GpuError::UnsupportedSmartFilter)
    ));
}

#[test]
fn disabled_smart_filters_stay_eligible() {
    let layer = smart(proxy(), vec![filter(CAMERA_RAW_FILTER_ID, false)], true);
    assert!(check_supported(&doc_of(layer)).is_ok());
    // An enabled filter this engine cannot model leaves the proxy in charge.
    let unknown = smart(proxy(), vec![filter(1, true)], true);
    assert!(check_supported(&doc_of(unknown)).is_ok());
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
#[should_panic(expected = "no usable Vulkan device")]
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
