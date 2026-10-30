//! Before/after profile for the per-layer GPU eligibility change.
//!
//! A 4000² document with a Curves adjustment, a drop-shadow layer, and a
//! clipping group: on the baseline every one of those declines the whole
//! document to the CPU; on this branch the same document composites on the GPU
//! (the clipping group is built on the CPU and blended on the GPU).
//!
//! Run on a Vulkan machine:
//! `cargo test -p pictura-render --test per_layer_bench -- --ignored --nocapture`

use std::time::{Duration, Instant};

use pictura_core::{
    AdjustmentData, BitDepth, BlendMode, Channel, ColorMode, Document, Layer, LayerStack, PsdRect,
};
use pictura_render::{composite_active, composite_region_active, composite_rgba, Backend};

const N: u32 = 4000;
const RUNS: usize = 3;

fn rect(w: u32, h: u32, x: i32, y: i32) -> PsdRect {
    PsdRect {
        top: y,
        left: x,
        bottom: y + h as i32,
        right: x + w as i32,
    }
}

fn pixel(name: &str, rect: PsdRect, rgb: (u8, u8, u8), alpha: u8, blend: BlendMode) -> Layer {
    let n = (rect.width() * rect.height()) as usize;
    Layer {
        name: name.into(),
        rect,
        blend,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: [0i16, 1, 2, -1]
            .map(|id| Channel {
                id,
                data: vec![
                    match id {
                        0 => rgb.0,
                        1 => rgb.1,
                        2 => rgb.2,
                        _ => alpha,
                    };
                    n
                ]
                .into(),
            })
            .to_vec(),
        ..Default::default()
    }
}

fn curves() -> AdjustmentData {
    let fixture = pictura_codec::read_psd(include_bytes!(
        "../../pictura-codec/tests/fixtures/curves.psd"
    ))
    .expect("curves fixture parses");
    fixture
        .layers
        .iter()
        .find_map(|layer| layer.adjustment.clone())
        .expect("curves fixture has an adjustment layer")
}

fn adjustment(name: &str, data: AdjustmentData) -> Layer {
    Layer {
        name: name.into(),
        rect: rect(0, 0, 0, 0),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        visible: true,
        adjustment: Some(data),
        channels: Vec::new(),
        ..Default::default()
    }
}

fn scene() -> Document {
    let mut styled = pixel(
        "effect",
        rect(1024, 1024, 200, 200),
        (200, 100, 50),
        255,
        BlendMode::Normal,
    );
    pictura_render::set_layer_style_value(&mut styled, "dropShadow.on", 1.0);

    let clip_base = pixel(
        "clip base",
        rect(512, 512, 100, 100),
        (255, 0, 0),
        255,
        BlendMode::Normal,
    );
    let mut clipped = pixel(
        "clipped",
        rect(512, 512, 100, 100),
        (0, 255, 0),
        200,
        BlendMode::Normal,
    );
    clipped.clipping = true;

    let mut doc = Document::new(N, N, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        pixel(
            "background",
            rect(N, N, 0, 0),
            (255, 255, 255),
            255,
            BlendMode::Normal,
        ),
        adjustment("curves", curves()),
        styled,
        clip_base,
        clipped,
    ]);
    doc
}

fn best(mut run: impl FnMut() -> (Duration, Backend)) -> (f64, Backend) {
    let mut best = Duration::MAX;
    let mut backend = Backend::Cpu;
    for _ in 0..RUNS {
        let (elapsed, b) = run();
        best = best.min(elapsed);
        backend = b;
    }
    (best.as_secs_f64() * 1000.0, backend)
}

fn max_delta(a: &[u8], b: &[u8]) -> i32 {
    a.iter()
        .zip(b)
        .map(|(&x, &y)| (x as i32 - y as i32).abs())
        .max()
        .unwrap_or(0)
}

#[test]
#[ignore = "benchmark; run with --ignored --nocapture on a Vulkan machine"]
fn per_layer_gpu_bench() {
    let doc = scene();
    let cpu = composite_rgba(&doc);
    let _ = composite_active(&doc, true);

    let (active_ms, backend) = best(|| {
        let start = Instant::now();
        let (_buf, backend) = composite_active(&doc, true);
        (start.elapsed(), backend)
    });
    let (cpu_ms, _) = best(|| {
        let start = Instant::now();
        let _ = composite_rgba(&doc);
        (start.elapsed(), Backend::Cpu)
    });
    let (active, _) = composite_active(&doc, true);
    let full_delta = max_delta(&active.data, &cpu.data);

    let region = rect(512, 512, 1200, 1500);
    let _ = composite_region_active(&doc, region, true);
    let (region_ms, rbackend) = best(|| {
        let start = Instant::now();
        let (_buf, backend) = composite_region_active(&doc, region, true);
        (start.elapsed(), backend)
    });
    let (region_cpu_ms, _) = best(|| {
        let start = Instant::now();
        let _ = composite_region_active(&doc, region, false);
        (start.elapsed(), Backend::Cpu)
    });
    let (region_buf, _) = composite_region_active(&doc, region, true);
    let (region_cpu, _) = composite_region_active(&doc, region, false);
    let region_delta = max_delta(&region_buf.data, &region_cpu.data);

    let speedup = |active: f64, cpu: f64| {
        if active > 0.0 {
            cpu / active
        } else {
            f64::INFINITY
        }
    };
    eprintln!("per-layer bench {N}x{N} (curves + drop shadow + clipping group):");
    eprintln!(
        "  full:   backend={backend:?} active={active_ms:.2} ms cpu={cpu_ms:.2} ms speedup={:.2}x max_delta={full_delta}",
        speedup(active_ms, cpu_ms)
    );
    eprintln!(
        "  region: backend={rbackend:?} active={region_ms:.2} ms cpu={region_cpu_ms:.2} ms speedup={:.2}x max_delta={region_delta}",
        speedup(region_ms, region_cpu_ms)
    );
    assert!(full_delta <= 1, "full composite delta {full_delta}");
    assert!(region_delta <= 1, "region composite delta {region_delta}");
}
