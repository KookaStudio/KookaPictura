//! Large-document, 2-D dispatch limit, and timing tests (opt-in bench).

use std::time::Instant;

use pictura_core::{BitDepth, BlendMode, ColorMode, Document, PixelBuffer};
use pictura_render::{
    composite_active, composite_gpu, composite_rgba, gpu_available, Backend, GpuError,
};

use crate::common::{bench_enabled, is_gpu_gone, large_layer, require_gpu, timing_layer};

/// The M29 regression: 4096×2048 is 8.4 MP ⇒ 131072 one-dimensional
/// workgroups, past the 65535 limit the pre-M29 stack rejected. The 2-D grid
/// must composite it on the GPU within ±1 LSB of the CPU oracle.
#[test]
fn large_document_over_1d_limit_composites_on_gpu() {
    if !bench_enabled() {
        eprintln!("skipping large-document parity: set PICTURA_GPU_BENCH=1 to run");
        return;
    }
    const W: u32 = 4096;
    const H: u32 = 2048;
    let mut doc = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        large_layer(W, H, 0, BlendMode::Normal),
        large_layer(W, H, 1, BlendMode::Multiply),
    ]);
    if !gpu_available() {
        eprintln!("no usable Vulkan GPU; skipping large-document parity");
        return;
    }
    let (gpu, backend) = composite_active(&doc, true);
    assert_eq!(
        backend,
        Backend::Gpu,
        "2-D dispatch must lift the 1-D ceiling"
    );
    assert_eq!((gpu.width, gpu.height, gpu.channels), (W, H, 4));
    let cpu = composite_rgba(&doc);
    let mut max_delta = 0i32;
    for (i, (&a, &b)) in cpu.data.iter().zip(&gpu.data).enumerate() {
        let d = (a as i32 - b as i32).abs();
        max_delta = max_delta.max(d);
        assert!(d <= 1, "byte {i}: cpu {a} vs gpu {b}");
    }
    eprintln!("large {W}x{H} parity: max delta {max_delta} LSB, backend {backend:?}");
}

/// 4000×4000 is 16.7 M px ⇒ 250000 one-dimensional workgroups. Asserts the GPU
/// backend, full ±1 LSB parity against the CPU oracle, and reports the
/// GPU-vs-CPU `composite_active` wall time.
#[test]
fn full_4000_document_composites_on_gpu() {
    if !bench_enabled() {
        eprintln!("skipping 4000² parity: set PICTURA_GPU_BENCH=1 to run");
        return;
    }
    const N: u32 = 4000;
    let mut doc = Document::new(N, N, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![
        large_layer(N, N, 0, BlendMode::Normal),
        large_layer(N, N, 1, BlendMode::Screen),
        large_layer(N, N, 2, BlendMode::Overlay),
    ]);
    if !gpu_available() {
        eprintln!("no usable Vulkan GPU; skipping 4000² parity");
        return;
    }

    // One warm-up call keeps device/pipeline setup out of the GPU number; the
    // timed GPU buffer doubles as the parity buffer.
    let _ = composite_active(&doc, true);
    let t = Instant::now();
    let (gpu, backend) = composite_active(&doc, true);
    let gpu_ms = t.elapsed().as_millis();
    assert_eq!(backend, Backend::Gpu, "4000² must run on the GPU");
    let t = Instant::now();
    let cpu = composite_rgba(&doc);
    let cpu_ms = t.elapsed().as_millis();

    let mut max_delta = 0i32;
    for (i, (&a, &b)) in cpu.data.iter().zip(&gpu.data).enumerate() {
        let d = (a as i32 - b as i32).abs();
        max_delta = max_delta.max(d);
        assert!(d <= 1, "byte {i}: cpu {a} vs gpu {b}");
    }
    eprintln!("4000x4000x3layers: cpu {cpu_ms} ms, gpu {gpu_ms} ms, max delta {max_delta} LSB");
}

/// `limit² × 64` exceeds `u32::MAX`, so no real document reaches the 2-D product
/// guard; a struct-literal document with oversized dimensions exercises the
/// limit path with no large allocation. Must return `TooLarge` and not panic.
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
        layers: pictura_core::LayerStack::default(),
        channels: Vec::new(),
        ..Default::default()
    };
    assert!(matches!(composite_gpu(&doc), Err(GpuError::TooLarge)));
}

/// Evidence only: no ratio is asserted (timings vary by machine and load). One
/// warm-up call keeps one-time device/ pipeline setup out of the GPU number.
#[test]
fn gpu_vs_cpu_timing_1024() {
    if !bench_enabled() {
        println!("skipping GPU 1024 timing: set PICTURA_GPU_BENCH=1 to run");
        return;
    }
    const N: u32 = 1024;
    let modes = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
    ];
    let mut doc = Document::new(N, N, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(
        modes
            .iter()
            .enumerate()
            .map(|(i, &blend)| timing_layer(N, i as u32, blend))
            .collect(),
    );

    let t = Instant::now();
    let _ = composite_rgba(&doc);
    let cpu_ms = t.elapsed().as_millis();

    if !gpu_available() {
        println!("GPU timing {N}x{N}x4layers: cpu {cpu_ms} ms, gpu n/a");
        return;
    }
    let warm = composite_gpu(&doc);
    if let Err(e) = &warm {
        if is_gpu_gone(e) {
            println!("GPU timing {N}x{N}x4layers: cpu {cpu_ms} ms, gpu n/a");
            return;
        }
    }
    drop(warm);

    let t = Instant::now();
    match composite_gpu(&doc) {
        Ok(_) => {
            let gpu_ms = t.elapsed().as_millis();
            println!("GPU timing {N}x{N}x4layers: cpu {cpu_ms} ms, gpu {gpu_ms} ms");
        }
        Err(e) => println!("GPU timing {N}x{N}x4layers: cpu {cpu_ms} ms, gpu n/a ({e})"),
    }
}
