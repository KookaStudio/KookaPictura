//! Blend-mode and backend dispatch parity tests.

use pictura_core::BlendMode;
use pictura_render::{composite, composite_rgba, gpu_available, Backend, CompositeRequest};

use crate::common::{require_gpu, scene, GPU_MODES, SIZE};

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn gpu_matches_cpu_within_one_lsb() {
    require_gpu();
    let mut checked = 0;
    let mut max_delta = 0i32;
    for &mode in GPU_MODES {
        let doc = scene(mode);
        let cpu = composite_rgba(&doc);
        let (gpu, backend) = composite(CompositeRequest::whole(&doc, true));
        assert_eq!(backend, Backend::Gpu, "{mode:?} must dispatch to the GPU");
        assert_eq!((gpu.width, gpu.height, gpu.channels), (SIZE, SIZE, 4));
        for (i, (&a, &b)) in cpu.data.iter().zip(&gpu.data).enumerate() {
            let delta = (a as i32 - b as i32).abs();
            max_delta = max_delta.max(delta);
            assert!(delta <= 1, "{mode:?} byte {i}: cpu {a} vs gpu {b}");
        }
        checked += 1;
    }
    eprintln!("gpu parity: {checked} modes ok, max delta {max_delta} LSB");
}

#[test]
fn fallback_matches_cpu_for_cpu_only_mode() {
    let doc = scene(BlendMode::Dissolve);
    let (buffer, backend) = composite(CompositeRequest::whole(&doc, true));
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(buffer.data, composite_rgba(&doc).data);
}

#[test]
#[ignore = "requires a Vulkan device; run with --profile gpu --run-ignored all"]
fn non_separable_modes_match_cpu() {
    require_gpu();
    let mut checked = 0;
    let mut max_delta = 0i32;
    for &mode in &[
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ] {
        let doc = scene(mode);
        let cpu = composite_rgba(&doc);
        let (gpu, backend) = composite(CompositeRequest::whole(&doc, true));
        assert_eq!(backend, Backend::Gpu, "{mode:?} must dispatch to the GPU");
        assert_eq!((gpu.width, gpu.height, gpu.channels), (SIZE, SIZE, 4));
        for (i, (&a, &b)) in cpu.data.iter().zip(&gpu.data).enumerate() {
            let delta = (a as i32 - b as i32).abs();
            max_delta = max_delta.max(delta);
            assert!(delta <= 1, "{mode:?} byte {i}: cpu {a} vs gpu {b}");
        }
        checked += 1;
    }
    eprintln!("non-separable parity: {checked} modes ok, max delta {max_delta} LSB");
}

#[test]
fn disabled_gpu_returns_cpu_backend() {
    let doc = scene(BlendMode::Normal);
    let (buf, backend) = composite(CompositeRequest::whole(&doc, false));
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(buf.data, composite_rgba(&doc).data);
}

#[test]
fn enabled_gpu_matches_availability() {
    let doc = scene(BlendMode::Normal);
    let cpu = composite_rgba(&doc);
    let (buf, backend) = composite(CompositeRequest::whole(&doc, true));
    if gpu_available() {
        assert_eq!(backend, Backend::Gpu);
        assert_eq!((buf.width, buf.height, buf.channels), (SIZE, SIZE, 4));
        for (i, (&a, &b)) in cpu.data.iter().zip(&buf.data).enumerate() {
            assert!(
                (a as i32 - b as i32).abs() <= 1,
                "byte {i}: cpu {a} vs gpu {b}"
            );
        }
    } else {
        assert_eq!(backend, Backend::Cpu);
        assert_eq!(buf.data, cpu.data);
    }
}
