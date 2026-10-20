//! Shared constants, scenes, and parity helpers for the GPU parity tests.

use pictura_core::{
    AdjustmentData, BitDepth, BlendMode, Channel, ColorLabel, ColorMode, Document, Layer,
    LockFlags, PsdRect,
};
use pictura_render::{composite_gpu, composite_rgba, GpuError};

pub(crate) const SIZE: u32 = 8;

/// Normal + every blend mode the GPU implements, including the whole-RGB
/// DarkerColor/LighterColor comparisons and the four non-separable modes.
pub(crate) const GPU_MODES: &[BlendMode] = &[
    BlendMode::Normal,
    BlendMode::Darken,
    BlendMode::Multiply,
    BlendMode::ColorBurn,
    BlendMode::LinearBurn,
    BlendMode::DarkerColor,
    BlendMode::Lighten,
    BlendMode::Screen,
    BlendMode::ColorDodge,
    BlendMode::LinearDodge,
    BlendMode::LighterColor,
    BlendMode::Overlay,
    BlendMode::SoftLight,
    BlendMode::HardLight,
    BlendMode::VividLight,
    BlendMode::LinearLight,
    BlendMode::PinLight,
    BlendMode::HardMix,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Subtract,
    BlendMode::Divide,
    BlendMode::Hue,
    BlendMode::Saturation,
    BlendMode::Color,
    BlendMode::Luminosity,
];

pub(crate) const CPU_ONLY: &[BlendMode] = &[BlendMode::Dissolve];

/// Heavy hardware evidence tests are opt-in. Set `PICTURA_GPU_BENCH=1` to run the
/// 4000²/2048/1024 timing and large-document tests; the default `cargo test` on a
/// GPU box stays correctness-only.
pub(crate) fn bench_enabled() -> bool {
    std::env::var_os("PICTURA_GPU_BENCH").is_some()
}

pub(crate) fn layer(
    name: &str,
    blend: BlendMode,
    sample: impl Fn(u32, u32) -> (u8, u8, u8, u8),
) -> Layer {
    let pixels = (SIZE * SIZE) as usize;
    let (mut r, mut g, mut b, mut a) = (
        Vec::with_capacity(pixels),
        Vec::with_capacity(pixels),
        Vec::with_capacity(pixels),
        Vec::with_capacity(pixels),
    );
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (rr, gg, bb, aa) = sample(x, y);
            r.push(rr);
            g.push(gg);
            b.push(bb);
            a.push(aa);
        }
    }
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
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: r.into(),
            },
            Channel {
                id: 1,
                data: g.into(),
            },
            Channel {
                id: 2,
                data: b.into(),
            },
            Channel {
                id: -1,
                data: a.into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

/// An empty-rect layer with children, for the group dispatch paths.
pub(crate) fn group(name: &str, blend: BlendMode, opacity: u8, children: Vec<Layer>) -> Layer {
    Layer {
        name: name.into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        },
        blend,
        opacity,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: Vec::new(),
        children: pictura_core::LayerStack::from_layers(children),
        is_group: true,
        background: false,
        ..Default::default()
    }
}

/// Opaque gradient backdrop plus a half-transparent gradient source in `mode`.
pub(crate) fn scene(mode: BlendMode) -> Document {
    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    let base = layer("base", BlendMode::Normal, |x, y| {
        (x as u8 * 32, y as u8 * 32, (x + y) as u8 * 16, 255)
    });
    let top = layer("top", mode, |x, y| {
        (
            x as u8 * 16 + 8,
            y as u8 * 16 + 8,
            255 - x as u8 * 16,
            64 + x as u8 * 16,
        )
    });
    doc.layers = pictura_core::LayerStack::from_layers(vec![base, top]);
    doc
}

/// Opaque gradient base layer for the adjustment scenes.
pub(crate) fn base_layer() -> Layer {
    layer("base", BlendMode::Normal, |x, y| {
        (
            x as u8 * 30 + 5,
            y as u8 * 30 + 5,
            x as u8 * 16 + y as u8 * 8,
            255,
        )
    })
}

/// An adjustment layer owns no pixels: an empty rect and no channels.
pub(crate) fn adjustment_layer(name: &str, adjustment: AdjustmentData) -> Layer {
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
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: Some(adjustment),
        channels: Vec::new(),
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

/// A full-size pixel layer for the timing scene (the `layer` helper is fixed at
/// `SIZE`). Seed 0 is the opaque backdrop; the rest are partially transparent.
pub(crate) fn timing_layer(size: u32, seed: u32, blend: BlendMode) -> Layer {
    large_layer(size, size, seed, blend)
}

/// A full-size `w×h` pixel layer, for the large-document (non-square) scenes.
pub(crate) fn large_layer(w: u32, h: u32, seed: u32, blend: BlendMode) -> Layer {
    let n = (w as usize) * (h as usize);
    let (mut r, mut g, mut b, mut a) = (vec![0u8; n], vec![0u8; n], vec![0u8; n], vec![0u8; n]);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            r[i] = (x * 7 + seed * 31) as u8;
            g[i] = (y * 5 + seed * 17) as u8;
            b[i] = (x ^ y) as u8;
            a[i] = if seed == 0 { 255 } else { 160 };
        }
    }
    Layer {
        name: format!("layer{seed}"),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: h as i32,
            right: w as i32,
        },
        blend,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        color: ColorLabel::None,
        clipping: false,
        visible: true,
        mask: None,
        adjustment: None,
        channels: vec![
            Channel {
                id: 0,
                data: r.into(),
            },
            Channel {
                id: 1,
                data: g.into(),
            },
            Channel {
                id: 2,
                data: b.into(),
            },
            Channel {
                id: -1,
                data: a.into(),
            },
        ],
        children: pictura_core::LayerStack::default(),
        is_group: false,
        background: false,
        ..Default::default()
    }
}

pub(crate) fn is_gpu_gone(err: &GpuError) -> bool {
    matches!(
        err,
        GpuError::Unavailable | GpuError::TooLarge | GpuError::Readback
    )
}

/// Assert one scene's GPU result matches the CPU oracle within ±1 LSB, or skip
/// with a printed note when no adapter is usable.
pub(crate) fn check_scene_parity(doc: &Document, max_delta: &mut i32) {
    let cpu = composite_rgba(doc);
    let gpu = match composite_gpu(doc) {
        Ok(buf) => buf,
        Err(e) if is_gpu_gone(&e) => {
            eprintln!("no usable Vulkan GPU ({e}); skipping scene parity");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    assert_eq!(
        (gpu.width, gpu.height, gpu.channels),
        (doc.width, doc.height, 4)
    );
    for (i, (&a, &b)) in cpu.data.iter().zip(&gpu.data).enumerate() {
        let delta = (a as i32 - b as i32).abs();
        *max_delta = (*max_delta).max(delta);
        assert!(delta <= 1, "byte {i}: cpu {a} vs gpu {b}");
    }
}
