//! Opt-in benchmark: a proxied type layer with the text gamma on.
//!
//! Before this branch `check_supported` declined the whole document while the
//! gamma was on, so the CPU path is the old behavior. This measures the new GPU
//! path against it on the same document, plus the curve's GPU overhead.

use std::time::Instant;

use pictura_core::{BitDepth, BlendMode, Channel, ColorMode, Document, Layer, PsdRect, TypeTool};
use pictura_render::{
    composite_active, gpu_available, set_text_gamma, text_gamma, Backend, DEFAULT_TEXT_GAMMA,
};

fn channel(id: i16, data: Vec<u8>) -> Channel {
    Channel {
        id,
        data: data.into(),
    }
}

fn whole(n: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: n as i32,
        right: n as i32,
    }
}

fn type_tool(n: u32) -> TypeTool {
    TypeTool {
        transform: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        text: "Hi".into(),
        bounds: [0, 0, n as i32, n as i32],
        text_desc: Vec::new().into(),
        warp_desc: Vec::new().into(),
        fonts: Vec::new(),
        style: None,
        vertical: false,
        box_bounds: None,
        path: None,
    }
}

/// An opaque gradient backdrop under a full-canvas proxied type layer whose
/// alpha sweeps 0…255, so the curve runs on nearly every source pixel.
fn scene(n: u32) -> Document {
    let px = (n * n) as usize;
    let mut br = vec![0u8; px];
    let mut bg = vec![0u8; px];
    let mut bb = vec![0u8; px];
    let ba = vec![255u8; px];
    let tr = vec![0u8; px];
    let tg = vec![0u8; px];
    let tb = vec![0u8; px];
    let mut ta = vec![0u8; px];
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) as usize;
            br[i] = (x * 255 / (n - 1)) as u8;
            bg[i] = (y * 255 / (n - 1)) as u8;
            bb[i] = 128;
            ta[i] = ((x * 7 + y * 13) % 256) as u8;
        }
    }
    let base = Layer {
        name: "base".into(),
        rect: whole(n),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: vec![
            channel(0, br),
            channel(1, bg),
            channel(2, bb),
            channel(-1, ba),
        ],
        ..Default::default()
    };
    let text = Layer {
        name: "text".into(),
        rect: whole(n),
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        visible: true,
        channels: vec![
            channel(0, tr),
            channel(1, tg),
            channel(2, tb),
            channel(-1, ta),
        ],
        type_tool: Some(type_tool(n)),
        ..Default::default()
    };
    let mut doc = Document::new(n, n, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![base, text]);
    doc
}

fn median(mut times: Vec<f64>) -> f64 {
    times.sort_by(f64::total_cmp);
    times[times.len() / 2]
}

fn bench(doc: &Document, gpu: bool, runs: usize) -> Vec<f64> {
    let mut times = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = Instant::now();
        let (_, backend) = composite_active(doc, gpu);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(
            backend,
            if gpu { Backend::Gpu } else { Backend::Cpu },
            "the benchmark must measure the requested backend"
        );
        times.push(ms);
    }
    times
}

#[test]
#[ignore = "benchmark; run with --ignored --nocapture on a Vulkan machine"]
fn text_gamma_gpu_vs_cpu() {
    if !gpu_available() {
        println!("text-gamma bench: no usable GPU; skipped");
        return;
    }
    let old = text_gamma();
    for n in [1024, 4000] {
        let doc = scene(n);
        set_text_gamma(Some(DEFAULT_TEXT_GAMMA));
        let _ = composite_active(&doc, true);
        let _ = composite_active(&doc, false);

        let cpu_on = median(bench(&doc, false, 5));
        let gpu_on = median(bench(&doc, true, 5));
        set_text_gamma(None);
        let gpu_off = median(bench(&doc, true, 5));
        set_text_gamma(Some(DEFAULT_TEXT_GAMMA));

        println!(
            "text-gamma bench {n}x{n} (median of 5, unstamped source): \
             cpu_on={cpu_on:.1} ms gpu_on={gpu_on:.1} ms gpu_off={gpu_off:.1} ms \
             speedup={:.2}x curve_overhead={:.1} ms",
            cpu_on / gpu_on,
            gpu_on - gpu_off
        );
    }
    set_text_gamma(old);
}
