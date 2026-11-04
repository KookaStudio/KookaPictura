//! Request matrix: whole/region x CPU/GPU over a stack with a group, a mask, an
//! adjustment, and a separable blend.

use super::*;
use pictura_core::{LayerMask, LayerStack};

const SIZE: u32 = 8;

fn gradient(name: &str, blend: BlendMode, alpha: u8) -> Layer {
    let pixels = (SIZE * SIZE) as usize;
    let mut r = Vec::with_capacity(pixels);
    let mut g = Vec::with_capacity(pixels);
    let mut b = Vec::with_capacity(pixels);
    for y in 0..SIZE {
        for x in 0..SIZE {
            r.push((x * 32) as u8);
            g.push((y * 32) as u8);
            b.push(((x + y) * 16) as u8);
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
        visible: true,
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
                data: vec![alpha; pixels].into(),
            },
        ],
        ..Default::default()
    }
}

fn fixture() -> Document {
    let mut masked = gradient("masked", BlendMode::Multiply, 200);
    masked.mask = Some(LayerMask {
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        default_color: 0,
        disabled: false,
        flags: 0,
        data: Some((0..SIZE * SIZE).map(|i| (i * 37 % 256) as u8).collect()),
        ..Default::default()
    });

    let child = gradient("child", BlendMode::Normal, 255);
    let group = Layer {
        name: "group".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        blend: BlendMode::Multiply,
        opacity: 200,
        fill: 255,
        visible: true,
        children: LayerStack::from_layers(vec![child]),
        is_group: true,
        ..Default::default()
    };

    let adjustment = Layer {
        name: "invert".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: SIZE as i32,
            right: SIZE as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        visible: true,
        adjustment: Some(crate::encode_invert()),
        ..Default::default()
    };

    let mut doc = Document::new(SIZE, SIZE, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![
        gradient("base", BlendMode::Normal, 255),
        masked,
        group,
        adjustment,
    ]);
    doc
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
    }
}

fn slice(full: &PixelBuffer, doc_width: u32, region: PsdRect) -> Vec<u8> {
    let x0 = region.left.max(0) as usize;
    let y0 = region.top.max(0) as usize;
    let x1 = region.right.max(0) as usize;
    let y1 = region.bottom.max(0) as usize;
    let w = x1.saturating_sub(x0);
    let h = y1.saturating_sub(y0);
    let fplane = full.pixel_count();
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            for c in 0..4 {
                out[c * w * h + y * w + x] =
                    full.data[c * fplane + (y0 + y) * doc_width as usize + x0 + x];
            }
        }
    }
    out
}

fn assert_matches_oracle(got: &PixelBuffer, oracle: &[u8], label: &str) {
    assert_eq!(got.data.len(), oracle.len(), "{label}: length");
    for (i, (&a, &b)) in got.data.iter().zip(oracle).enumerate() {
        assert!(
            a.abs_diff(b) <= 1,
            "{label}: byte {i}: request {a} vs oracle {b}"
        );
    }
}

#[test]
fn request_matrix_matches_the_oracles() {
    let doc = fixture();
    let cpu = crate::composite_rgba(&doc);
    let region = rect(2, 1, 6, 7);
    let cpu_region = slice(&cpu, doc.width, region);

    let (whole, backend) = crate::composite(crate::CompositeRequest::whole(&doc, false));
    assert_eq!(backend, Backend::Cpu, "whole CPU backend");
    assert_eq!(whole.data, cpu.data, "whole CPU bytes");

    let (region_cpu, backend) =
        crate::composite(crate::CompositeRequest::region(&doc, region, false));
    assert_eq!(backend, Backend::Cpu, "region CPU backend");
    assert_eq!(region_cpu.data, cpu_region, "region CPU bytes");

    let gpu_available = crate::gpu::gpu_available();

    let (whole_gpu, backend) = crate::composite(crate::CompositeRequest::whole(&doc, true));
    if gpu_available {
        assert_eq!(backend, Backend::Gpu, "whole GPU backend");
        assert_matches_oracle(&whole_gpu, &cpu.data, "whole GPU");
    } else {
        assert_eq!(backend, Backend::Cpu, "whole GPU fallback backend");
        assert_eq!(whole_gpu.data, cpu.data, "whole GPU fallback bytes");
    }

    let (region_gpu, backend) =
        crate::composite(crate::CompositeRequest::region(&doc, region, true));
    if gpu_available {
        assert_eq!(backend, Backend::Gpu, "region GPU backend");
        assert_matches_oracle(&region_gpu, &cpu_region, "region GPU");
    } else {
        assert_eq!(backend, Backend::Cpu, "region GPU fallback backend");
        assert_eq!(region_gpu.data, cpu_region, "region GPU fallback bytes");
    }

    let mut deep = doc.clone();
    deep.source_depth = Some(BitDepth::ThirtyTwo);
    let (deep_region, backend) =
        crate::composite(crate::CompositeRequest::region(&deep, region, true));
    assert_eq!(backend, Backend::Cpu, "32-bit region backend");
    let deep_cpu = slice(&crate::composite_rgba(&deep), deep.width, region);
    assert_eq!(deep_region.data, deep_cpu, "32-bit region CPU bytes");
}
