//! M31 region-compositing parity tests.

use std::time::Instant;

use pictura_core::{
    BitDepth, BlendMode, ColorMode, Document, LayerMask, PixelBuffer, PsdRect, TypeTool,
};
use pictura_render::{
    composite_active, composite_region_active, encode_invert, gpu_available, set_text_gamma,
    text_gamma, Backend, DEFAULT_TEXT_GAMMA,
};

use crate::common::{
    adjustment_layer, base_layer, bench_enabled, gamma_guard, group, large_layer, layer,
    require_gpu, scene, SIZE,
};

fn whole_rect() -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: SIZE as i32,
        right: SIZE as i32,
    }
}

fn region_rect(top: i32, left: i32, bottom: i32, right: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn type_tool() -> TypeTool {
    TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "Hi".into(),
        bounds: [0, 0, SIZE as i32, SIZE as i32],
        text_desc: Vec::new(),
        warp_desc: Vec::new(),
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
        path: None,
    }
}

fn alpha_ramp(x: u32, y: u32) -> u8 {
    let i = y * SIZE + x;
    (i * 255 / (SIZE * SIZE - 1)) as u8
}

/// One document per scene kind the region path must preserve: separable,
/// non-separable, adjustment, masked, an isolated group, and a proxied type
/// layer.
fn region_scenes() -> Vec<(&'static str, Document)> {
    let separable = scene(BlendMode::Multiply);
    let nonseparable = scene(BlendMode::Hue);

    let mut adjustment = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    adjustment.layers = pictura_core::LayerStack::from_layers(vec![
        base_layer(),
        adjustment_layer("invert", encode_invert()),
    ]);

    let mut masked_layer = layer("masked", BlendMode::Multiply, |x, y| {
        (x as u8 * 16 + 8, y as u8 * 16 + 8, 255 - x as u8 * 16, 200)
    });
    masked_layer.opacity = 180;
    masked_layer.mask = Some(LayerMask {
        rect: whole_rect(),
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some((0..SIZE * SIZE).map(|i| (i * 37 % 256) as u8).collect()),
        ..Default::default()
    });
    let mut masked = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    masked.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), masked_layer]);

    let mut proxied_type = layer("text", BlendMode::Normal, |x, y| {
        (0, 0, 0, alpha_ramp(x, y))
    });
    proxied_type.type_tool = Some(type_tool());
    let mut typed = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    typed.layers = pictura_core::LayerStack::from_layers(vec![base_layer(), proxied_type]);

    let mut iso = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    iso.layers = pictura_core::LayerStack::from_layers(vec![
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

    vec![
        ("separable_multiply", separable),
        ("nonseparable_hue", nonseparable),
        ("adjustment_invert", adjustment),
        ("masked_layer", masked),
        ("proxied_type", typed),
        ("isolated_group", iso),
    ]
}

/// Assert `composite_region_active(doc, r, true)` equals the `r` slice of the
/// already-composited `full` buffer, byte for byte.
fn assert_region_slice(doc: &Document, full: &PixelBuffer, r: PsdRect, ctx: &str) {
    let (region, backend) = composite_region_active(doc, r, true);
    assert_eq!(backend, Backend::Gpu, "{ctx}: region must run on the GPU");
    let x0 = r.left.max(0) as usize;
    let y0 = r.top.max(0) as usize;
    let x1 = r.right.min(doc.width as i32).max(0) as usize;
    let y1 = r.bottom.min(doc.height as i32).max(0) as usize;
    let rw = x1 - x0;
    let rh = y1 - y0;
    assert_eq!(
        (region.width as usize, region.height as usize),
        (rw, rh),
        "{ctx}: region size"
    );

    let fplane = full.pixel_count();
    let rplane = region.pixel_count();
    let mut max_delta = 0i32;
    let mut samples = 0usize;
    for ry in 0..rh {
        for rx in 0..rw {
            let fi = (y0 + ry) * doc.width as usize + x0 + rx;
            let ri = ry * rw + rx;
            for c in 0..4 {
                let d =
                    (full.data[c * fplane + fi] as i32 - region.data[c * rplane + ri] as i32).abs();
                max_delta = max_delta.max(d);
                samples += 1;
            }
        }
    }
    assert_eq!(
        max_delta, 0,
        "{ctx}: region differs from the full-active slice by {max_delta} LSB over {samples} samples"
    );
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn region_matches_full_slice() {
    let _guard = gamma_guard();
    let old_gamma = text_gamma();
    set_text_gamma(Some(DEFAULT_TEXT_GAMMA));
    require_gpu();
    let rects = [
        region_rect(0, 0, 4, 4),
        region_rect(1, 2, 7, 6),
        region_rect(3, 5, SIZE as i32, SIZE as i32),
        region_rect(4, 0, SIZE as i32, SIZE as i32),
        region_rect(7, 7, SIZE as i32, SIZE as i32),
    ];
    let scenes = region_scenes();
    let count = scenes.len();
    for (name, doc) in scenes {
        let (full, backend) = composite_active(&doc, true);
        assert_eq!(backend, Backend::Gpu, "{name}: full must run on the GPU");
        for &r in &rects {
            assert_region_slice(&doc, &full, r, name);
        }
    }
    eprintln!("region parity: {count} scenes byte-identical to the full-active slice");
    set_text_gamma(old_gamma);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn full_region_equals_full_composite() {
    let _guard = gamma_guard();
    let old_gamma = text_gamma();
    set_text_gamma(Some(DEFAULT_TEXT_GAMMA));
    require_gpu();
    for (name, doc) in region_scenes() {
        let (full, fb) = composite_active(&doc, true);
        let (region, rb) = composite_region_active(&doc, whole_rect(), true);
        assert_eq!(fb, Backend::Gpu, "{name}: full backend");
        assert_eq!(rb, fb, "{name}: region backend");
        assert_eq!((region.width, region.height), (full.width, full.height));
        assert_eq!(
            region.data, full.data,
            "{name}: whole-doc region != full composite"
        );
    }
    eprintln!("full-region parity: every scene byte-identical");
    set_text_gamma(old_gamma);
}

#[test]
fn cpu_region_matches_full_slice() {
    let _guard = gamma_guard();
    let old_gamma = text_gamma();
    set_text_gamma(Some(DEFAULT_TEXT_GAMMA));
    // The CPU fallback must composite only the requested rect yet stay
    // byte-identical to the same slice of the full CPU composite, including
    // partial coverage and every multi-layer scene kind.
    let rects = [
        region_rect(0, 0, 4, 4),
        region_rect(1, 2, 7, 6),
        region_rect(3, 5, SIZE as i32, SIZE as i32),
        region_rect(-5, -5, 3, 3),
        region_rect(6, 6, 999, 999),
    ];
    for (name, doc) in region_scenes() {
        let (full, fb) = composite_active(&doc, false);
        assert_eq!(fb, Backend::Cpu, "{name}: full CPU backend");
        for &r in &rects {
            let (region, rb) = composite_region_active(&doc, r, false);
            assert_eq!(rb, Backend::Cpu, "{name}: region CPU backend");
            let x0 = r.left.max(0) as usize;
            let y0 = r.top.max(0) as usize;
            let x1 = r.right.min(doc.width as i32).max(0) as usize;
            let y1 = r.bottom.min(doc.height as i32).max(0) as usize;
            let rw = x1.saturating_sub(x0);
            let rh = y1.saturating_sub(y0);
            assert_eq!(
                (region.width as usize, region.height as usize),
                (rw, rh),
                "{name}: region size"
            );
            let fplane = full.pixel_count();
            let rplane = region.pixel_count();
            for ry in 0..rh {
                for rx in 0..rw {
                    for c in 0..4 {
                        let f = c * fplane + (y0 + ry) * doc.width as usize + x0 + rx;
                        let rr = c * rplane + ry * rw + rx;
                        assert_eq!(
                            region.data[rr],
                            full.data[f],
                            "{name}: pixel ({}, {}) channel {c}",
                            x0 + rx,
                            y0 + ry
                        );
                    }
                }
            }
        }
    }
    eprintln!("cpu region parity: every scene byte-identical to the full slice");
    set_text_gamma(old_gamma);
}

#[test]
fn empty_and_out_of_bounds_region_are_safe() {
    let doc = scene(BlendMode::Multiply);
    let (full, _) = composite_active(&doc, false);
    for (name, r) in [
        ("zero_area", region_rect(0, 0, 0, 0)),
        ("inverted", region_rect(5, 5, 2, 3)),
        ("fully_outside", region_rect(-10, -10, -5, -5)),
        ("past_bottom_right", region_rect(20, 20, 30, 30)),
    ] {
        let (buf, backend) = composite_region_active(&doc, r, false);
        assert_eq!(backend, Backend::Cpu, "{name}");
        assert_eq!(
            (buf.width, buf.height),
            (0, 0),
            "{name} must be zero-dimension"
        );
        assert!(buf.data.is_empty(), "{name} must have no data");
    }

    // Partial overlap clamps to the intersection and matches the full slice.
    let (buf, _) = composite_region_active(&doc, region_rect(-5, -5, 3, 3), false);
    assert_eq!((buf.width, buf.height), (3, 3));
    let fplane = full.pixel_count();
    let rplane = buf.pixel_count();
    for y in 0..3usize {
        for x in 0..3usize {
            for c in 0..4 {
                assert_eq!(
                    buf.data[c * rplane + y * 3 + x],
                    full.data[c * fplane + y * SIZE as usize + x]
                );
            }
        }
    }

    // Extending past the far edge clamps to what fits.
    let (buf, _) = composite_region_active(&doc, region_rect(6, 6, 20, 20), false);
    assert_eq!((buf.width, buf.height), (2, 2));
    eprintln!("empty/oob region: clamped and zero-dimension without panic");
}

/// Evidence only: records the region-vs-full win at 4000². No ratio is asserted
/// because timings vary by machine and load.
#[test]
fn region_vs_full_timing_4000() {
    if !bench_enabled() {
        eprintln!("skipping region timing: set PICTURA_GPU_BENCH=1 to run");
        return;
    }
    const N: u32 = 4000;
    if !gpu_available() {
        eprintln!("no usable Vulkan GPU; skipping region timing");
        return;
    }
    let mut doc = Document::new(N, N, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        large_layer(N, N, 0, BlendMode::Normal),
        large_layer(N, N, 1, BlendMode::Multiply),
    ]);
    let _ = composite_active(&doc, true); // warm-up: keep device/pipeline setup out of the numbers

    let t = Instant::now();
    let (full, backend) = composite_active(&doc, true);
    let full_ms = t.elapsed().as_millis();
    assert_eq!(backend, Backend::Gpu);

    let dirty = region_rect(1200, 1500, 1712, 2012); // 512×512
    let t = Instant::now();
    let (region, rbackend) = composite_region_active(&doc, dirty, true);
    let region_ms = t.elapsed().as_millis();
    assert_eq!(rbackend, Backend::Gpu);
    assert_eq!((region.width, region.height), (512, 512));
    let ratio = if region_ms > 0 {
        format!("{}x", full_ms / region_ms.max(1))
    } else {
        "n/a".into()
    };
    eprintln!("region timing 4000²: region 512×512 {region_ms} ms vs full {full_ms} ms ({ratio})");
    drop(full);
}
