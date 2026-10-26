//! GPU filter parity for the M27 accelerated kernel set.
//!
//! `pictura_filters::apply` is the oracle. Every accelerated filter must match
//! it within ±1 LSB per colour channel and keep alpha bit-for-bit on 4-channel
//! buffers. A filter with no GPU kernel, or the whole path with GPU compute
//! disabled, must return `Backend::Cpu` with bytes identical to the oracle.
//!
//! The parity test needs a device: it is `#[ignore]`d and fails when run
//! without one. The timing evidence stays opt-in through `PICTURA_GPU_BENCH`.

use pictura_core::PixelBuffer;
use pictura_filters::{apply, Filter, FilterKind, Quality, RadialMethod};
use pictura_render::{apply_filter_active, filter_gpu_available, Backend};

const SIZE: u32 = 32;

/// Heavy large-buffer/timing evidence tests are opt-in. Set `PICTURA_GPU_BENCH=1`
/// to run them; the default `cargo test` on a GPU box stays correctness-only.
fn bench_enabled() -> bool {
    std::env::var_os("PICTURA_GPU_BENCH").is_some()
}

/// Accelerated filters with representative parameters.
fn accelerated_filters() -> Vec<Filter> {
    vec![
        Filter::GaussianBlur { radius: 3.0 },
        Filter::BoxBlur { radius: 2 },
        Filter::MotionBlur {
            angle: 0.0,
            distance: 5,
        },
        Filter::Blur,
        Filter::BlurMore,
        Filter::Sharpen,
        Filter::SharpenMore,
        Filter::UnsharpMask {
            amount: 150.0,
            radius: 3.0,
            threshold: 0,
        },
        Filter::HighPass { radius: 3.0 },
        Filter::RadialBlur {
            method: RadialMethod::Zoom,
            amount: 70.0,
            center: (0.5, 0.5),
            quality: Quality::Best,
        },
        Filter::RadialBlur {
            method: RadialMethod::Zoom,
            amount: 70.0,
            center: (0.25, 0.75),
            quality: Quality::Good,
        },
        Filter::SurfaceBlur {
            radius: 2,
            threshold: 20,
        },
        Filter::SurfaceBlur {
            radius: 12,
            threshold: 40,
        },
        Filter::Maximum { radius: 5 },
        Filter::Maximum { radius: 11 },
        Filter::Minimum { radius: 5 },
        Filter::Minimum { radius: 11 },
        Filter::Median { radius: 1 },
        Filter::Median { radius: 4 },
        Filter::OilPaint {
            stylization: 4.0,
            cleanliness: 5.0,
            scale: 0.0,
            bristle_detail: 4.0,
            angular_direction: 135.0,
            shine: 2.0,
        },
        Filter::OilPaint {
            stylization: 8.0,
            cleanliness: 5.0,
            scale: 10.0,
            bristle_detail: 10.0,
            angular_direction: 85.0,
            shine: 8.0,
        },
        custom_filter(),
    ]
}

/// Non-trivial 5×5 kernel: a centre-weighted blur with `scale != 1` and a
/// non-zero `offset`, so the KERNEL path's normalize + offset + rounding are
/// all exercised.
fn custom_filter() -> Filter {
    let mut kernel = [[1.0f64; 5]; 5];
    for row in &mut kernel {
        row[1] = 2.0;
        row[2] = 4.0;
        row[3] = 2.0;
    }
    Filter::Custom {
        kernel,
        scale: 50.0,
        offset: 5.0,
    }
}

/// The catalogue's GPU-supported kinds and the parity fixture must name the
/// same kinds in both directions.
#[test]
fn accelerated_parity_fixture_matches_the_catalogue() {
    let fixture: std::collections::BTreeSet<&str> = accelerated_filters()
        .iter()
        .map(|filter| filter.kind().name())
        .collect();
    let expected: std::collections::BTreeSet<&str> = FilterKind::ALL
        .iter()
        .filter(|kind| kind.spec().gpu.is_supported())
        .map(|kind| kind.name())
        .collect();
    assert_eq!(
        fixture, expected,
        "GPU parity fixture drifted from the catalogue"
    );
}

/// A kernel-less filter (the painterly family stays CPU-only).
fn unsupported_filter() -> Filter {
    Filter::ColoredPencil {
        pencil_width: 6,
        stroke_pressure: 8,
        paper_brightness: 50,
        background: [255, 255, 255],
        seed: 1,
    }
}

/// 3-channel RGB gradient.
fn gradient_rgb() -> PixelBuffer {
    let n = (SIZE * SIZE) as usize;
    let mut buf = PixelBuffer::new(SIZE, SIZE, 3);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = (y * SIZE + x) as usize;
            buf.data[i] = (x * 8) as u8;
            buf.data[n + i] = (y * 8) as u8;
            buf.data[2 * n + i] = ((x + y) * 4) as u8;
        }
    }
    buf
}

/// 4-channel gradient with a non-trivial alpha plane.
fn gradient_rgba() -> PixelBuffer {
    let n = (SIZE * SIZE) as usize;
    let mut buf = PixelBuffer::new(SIZE, SIZE, 4);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = (y * SIZE + x) as usize;
            buf.data[i] = (x * 8) as u8;
            buf.data[n + i] = (y * 8) as u8;
            buf.data[2 * n + i] = ((x + y) * 4) as u8;
            buf.data[3 * n + i] = ((x * 7 + y * 13) % 256) as u8;
        }
    }
    buf
}

/// Hard black/white/colour edges: stresses tie-rounding in the separable and
/// 3x3 kernels.
fn edge_rgb() -> PixelBuffer {
    let n = (SIZE * SIZE) as usize;
    let mut buf = PixelBuffer::new(SIZE, SIZE, 3);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = (y * SIZE + x) as usize;
            let v = if (x + y) % 4 < 2 { 0 } else { 255 };
            buf.data[i] = v;
            buf.data[n + i] = u8::MAX - v;
            buf.data[2 * n + i] = if x < SIZE / 2 { 0 } else { 255 };
        }
    }
    buf
}

/// A single flat colour: kernels must be a near no-op.
fn flat_rgb() -> PixelBuffer {
    let n = (SIZE * SIZE) as usize;
    let mut buf = PixelBuffer::new(SIZE, SIZE, 3);
    for i in 0..n {
        buf.data[i] = 37;
        buf.data[n + i] = 149;
        buf.data[2 * n + i] = 211;
    }
    buf
}

/// A large `w×h` RGB gradient (the fixture helpers are fixed at `SIZE`).
fn large_gradient_rgb(w: u32, h: u32) -> PixelBuffer {
    let n = (w as usize) * (h as usize);
    let mut buf = PixelBuffer::new(w, h, 3);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            buf.data[i] = (x % 256) as u8;
            buf.data[n + i] = (y % 256) as u8;
            buf.data[2 * n + i] = ((x + y) % 256) as u8;
        }
    }
    buf
}

fn oracle(filter: &Filter, base: &PixelBuffer) -> PixelBuffer {
    let mut buf = base.clone();
    apply(filter, &mut buf).expect("oracle parameters are valid");
    buf
}

/// Colour-plane deltas and an alpha bit-for-bit check. Returns the max colour
/// delta.
fn assert_parity(got: &PixelBuffer, want: &PixelBuffer, label: &str) -> i32 {
    let n = want.pixel_count();
    let mut max_delta = 0i32;
    for c in 0..3 {
        for i in 0..n {
            let d = (want.data[c * n + i] as i32 - got.data[c * n + i] as i32).abs();
            max_delta = max_delta.max(d);
            assert!(
                d <= 1,
                "{label} channel {c} sample {i}: cpu {} vs gpu {}",
                want.data[c * n + i],
                got.data[c * n + i]
            );
        }
    }
    if want.channels == 4 {
        assert_eq!(
            &want.data[3 * n..4 * n],
            &got.data[3 * n..4 * n],
            "{label}: alpha must be untouched"
        );
    }
    max_delta
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn accelerated_filters_match_cpu_within_one_lsb() {
    assert!(
        filter_gpu_available(),
        "no usable Vulkan device; this test is #[ignore]d and needs one (software-vulkan in CI)"
    );
    for (label, base) in [
        ("rgb", gradient_rgb()),
        ("rgba", gradient_rgba()),
        ("edge", edge_rgb()),
        ("flat", flat_rgb()),
    ] {
        for filter in accelerated_filters() {
            let want = oracle(&filter, &base);
            let mut got = base.clone();
            let backend = apply_filter_active(&filter, &mut got, true).expect("valid parameters");
            assert_eq!(backend, Backend::Gpu, "{label}/{filter:?} fell back to CPU");
            let delta = assert_parity(&got, &want, &format!("{label}/{filter:?}"));
            println!("{label}/{filter:?}: max delta {delta} LSB");
        }
    }
}

/// M29: a buffer past the old 1-D filter ceiling. 3072×2048 = 6.29 MP ⇒
/// `3n/4 = 4,718,592` words ⇒ 73728 one-dimensional workgroups, over 65535. The
/// 2-D grid must run Surface Blur and Median on the GPU within ±1 LSB.
#[test]
fn large_buffer_over_1d_limit_runs_on_gpu() {
    if !bench_enabled() {
        println!("skipping large-buffer filter parity: set PICTURA_GPU_BENCH=1 to run");
        return;
    }
    if !filter_gpu_available() {
        println!("no usable Vulkan GPU; skipping large-buffer filter parity");
        return;
    }
    const W: u32 = 3072;
    const H: u32 = 2048;
    let base = large_gradient_rgb(W, H);
    let filters = [
        Filter::SurfaceBlur {
            radius: 1,
            threshold: 20,
        },
        Filter::Median { radius: 1 },
    ];
    for filter in filters {
        let want = oracle(&filter, &base);
        let mut got = base.clone();
        let backend = apply_filter_active(&filter, &mut got, true).expect("valid parameters");
        assert_eq!(
            backend,
            Backend::Gpu,
            "{filter:?} at {W}x{H} fell back to CPU"
        );
        let delta = assert_parity(&got, &want, &format!("large/{filter:?}"));
        println!("large {W}x{H}/{filter:?}: max delta {delta} LSB, backend {backend:?}");
    }
}

#[test]
fn unsupported_filter_falls_back_byte_identical() {
    let filter = unsupported_filter();
    let base = gradient_rgba();
    let want = oracle(&filter, &base);
    let mut got = base.clone();
    let backend = apply_filter_active(&filter, &mut got, true).expect("valid parameters");
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(got.data, want.data, "fallback must be byte-identical");
}

/// Radial Blur's Spin scatters its samples along arcs; the CPU's tiles keep
/// them in cache, so it stays on the CPU while Zoom runs on the GPU.
#[test]
fn radial_spin_runs_on_the_cpu() {
    let filter = Filter::RadialBlur {
        method: RadialMethod::Spin,
        amount: 40.0,
        center: (0.5, 0.5),
        quality: Quality::Good,
    };
    let base = gradient_rgba();
    let want = oracle(&filter, &base);
    let mut got = base.clone();
    let backend = apply_filter_active(&filter, &mut got, true).expect("valid parameters");
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(got.data, want.data, "fallback must be byte-identical");
}

/// Past the GPU's Surface Blur radius cap the CPU histogram runs instead.
#[test]
fn wide_surface_blur_runs_on_the_cpu() {
    let filter = Filter::SurfaceBlur {
        radius: 30,
        threshold: 40,
    };
    let base = gradient_rgba();
    let want = oracle(&filter, &base);
    let mut got = base.clone();
    let backend = apply_filter_active(&filter, &mut got, true).expect("valid parameters");
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(got.data, want.data, "fallback must be byte-identical");
}

#[test]
fn disabled_gpu_is_byte_identical() {
    let mut filters = accelerated_filters();
    filters.push(unsupported_filter());
    for base in [gradient_rgb(), gradient_rgba()] {
        for filter in &filters {
            let want = oracle(filter, &base);
            let mut got = base.clone();
            let backend = apply_filter_active(filter, &mut got, false).expect("valid parameters");
            assert_eq!(backend, Backend::Cpu, "{filter:?} with gpu disabled");
            assert_eq!(got.data, want.data, "{filter:?} disabled must match oracle");
        }
    }
}

/// Surface Blur's GPU speedup at a small radius. The CPU slides a histogram
/// (~40 ms at 1024², radius 4), so only small radii still pay for the GPU.
#[test]
fn surface_blur_1024_gpu_beats_cpu() {
    if !bench_enabled() {
        println!("skipping Surface Blur speedup check: set PICTURA_GPU_BENCH=1 to run");
        return;
    }
    if !filter_gpu_available() {
        println!("no usable Vulkan GPU; skipping Surface Blur speedup check");
        return;
    }
    const BIG: u32 = 1024;
    let n = (BIG * BIG) as usize;
    let mut base = PixelBuffer::new(BIG, BIG, 3);
    for y in 0..BIG {
        for x in 0..BIG {
            let i = (y * BIG + x) as usize;
            base.data[i] = (x % 256) as u8;
            base.data[n + i] = (y % 256) as u8;
            base.data[2 * n + i] = ((x + y) % 256) as u8;
        }
    }
    // A small radius: past 16 the GPU plan defers to the faster CPU histogram.
    let filter = Filter::SurfaceBlur {
        radius: 4,
        threshold: 20,
    };

    let mut cpu = base.clone();
    let t = std::time::Instant::now();
    apply(&filter, &mut cpu).expect("valid parameters");
    let cpu_ms = t.elapsed().as_secs_f64() * 1e3;

    let mut gpu = base.clone();
    let t = std::time::Instant::now();
    let backend = apply_filter_active(&filter, &mut gpu, true).expect("valid parameters");
    let gpu_ms = t.elapsed().as_secs_f64() * 1e3;

    assert_eq!(backend, Backend::Gpu);
    assert_parity(&gpu, &cpu, "surface-1024");
    println!(
        "SurfaceBlur 1024x1024 r=4 t=20: cpu {cpu_ms:.1} ms, gpu {gpu_ms:.1} ms, {:.1}x speedup",
        cpu_ms / gpu_ms
    );
    assert!(gpu_ms < cpu_ms, "GPU should beat CPU for Surface Blur");
}

/// Print-only CPU-vs-GPU timing for Median at 1024×1024 (the CPU baseline is
/// ~1.4 s). No speed assertion: the shader's selection cost is dominated by
/// eight clamped window scans per byte, which may or may not beat the CPU sort.
#[test]
fn median_1024_parity_and_timing() {
    if !bench_enabled() {
        println!("skipping Median 1024 timing: set PICTURA_GPU_BENCH=1 to run");
        return;
    }
    if !filter_gpu_available() {
        println!("no usable Vulkan GPU; skipping Median 1024 timing");
        return;
    }
    const BIG: u32 = 1024;
    let n = (BIG * BIG) as usize;
    let mut base = PixelBuffer::new(BIG, BIG, 3);
    for y in 0..BIG {
        for x in 0..BIG {
            let i = (y * BIG + x) as usize;
            base.data[i] = (x % 256) as u8;
            base.data[n + i] = (y % 256) as u8;
            base.data[2 * n + i] = ((x * 3 + y) % 256) as u8;
        }
    }
    let filter = Filter::Median { radius: 1 };

    let mut cpu = base.clone();
    let t = std::time::Instant::now();
    apply(&filter, &mut cpu).expect("valid parameters");
    let cpu_ms = t.elapsed().as_secs_f64() * 1e3;

    let mut gpu = base.clone();
    let t = std::time::Instant::now();
    let backend = apply_filter_active(&filter, &mut gpu, true).expect("valid parameters");
    let gpu_ms = t.elapsed().as_secs_f64() * 1e3;

    assert_eq!(backend, Backend::Gpu);
    assert_parity(&gpu, &cpu, "median-1024");
    println!("Median 1024x1024 r=1: cpu {cpu_ms:.1} ms, gpu {gpu_ms:.1} ms");
}

/// Print-only CPU-vs-GPU timing for Oil Paint at 1024×1024 (the CPU baseline
/// is ~1.4 s). No speed assertion: the multipass GPU path is reported, not
/// gated on beating the CPU.
#[test]
fn oil_paint_1024_parity_and_timing() {
    if !bench_enabled() {
        println!("skipping Oil Paint 1024 timing: set PICTURA_GPU_BENCH=1 to run");
        return;
    }
    if !filter_gpu_available() {
        println!("no usable Vulkan GPU; skipping Oil Paint 1024 timing");
        return;
    }
    const BIG: u32 = 1024;
    let n = (BIG * BIG) as usize;
    let mut base = PixelBuffer::new(BIG, BIG, 3);
    for y in 0..BIG {
        for x in 0..BIG {
            let i = (y * BIG + x) as usize;
            base.data[i] = (x % 256) as u8;
            base.data[n + i] = (y % 256) as u8;
            base.data[2 * n + i] = ((x * 3 + y) % 256) as u8;
        }
    }
    let filter = Filter::OilPaint {
        stylization: 8.0,
        cleanliness: 5.0,
        scale: 8.0,
        bristle_detail: 5.0,
        angular_direction: 85.0,
        shine: 5.0,
    };

    let mut cpu = base.clone();
    let t = std::time::Instant::now();
    apply(&filter, &mut cpu).expect("valid parameters");
    let cpu_ms = t.elapsed().as_secs_f64() * 1e3;

    let mut gpu = base.clone();
    let t = std::time::Instant::now();
    let backend = apply_filter_active(&filter, &mut gpu, true).expect("valid parameters");
    let gpu_ms = t.elapsed().as_secs_f64() * 1e3;

    assert_eq!(backend, Backend::Gpu);
    assert_parity(&gpu, &cpu, "oil-paint-1024");
    println!(
        "OilPaint 1024x1024 s=8 c=5 sc=8 b=5 a=85 sh=5: cpu {cpu_ms:.1} ms, gpu {gpu_ms:.1} ms"
    );
}
