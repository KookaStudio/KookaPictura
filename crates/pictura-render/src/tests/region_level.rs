//! `composite_region` and `composite_region_level` against the whole composite
//! and a whole `ViewPyramid` rebuild (#1216, R5: whole references at small
//! sizes only).

use super::*;
use crate::tests::{doc, solid};
use crate::{composite_rgba, Planes, ViewPyramid};
use pictura_core::{BlendMode, Channel, Layer, LayerMask, PsdRect};
use std::sync::atomic::{AtomicU64, Ordering};

/// Odd on both sides, so every level folds a short last row and column, and
/// four levels deep (2101 → 1051 → 526 → 263).
pub(crate) const W: u32 = 2101;
pub(crate) const H: u32 = 1301;

/// A small deterministic generator.
pub(crate) struct Lcg(u64);

impl Lcg {
    pub(crate) fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n.max(1)
    }
}

fn patterned(name: &str, rect: PsdRect, blend: BlendMode, rng: &mut Lcg) -> Layer {
    let mut layer = solid(name, rect, (0, 0, 0), 255, blend, 200 + rng.below(56) as u8);
    let (w, h) = (rect.width() as usize, rect.height() as usize);
    let (fx, fy, k) = (rng.below(7) + 1, rng.below(5) + 1, rng.below(251));
    for (c, channel) in layer.channels.iter_mut().enumerate() {
        let data: Vec<u8> = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as u32, (i / w) as u32);
                let v = x * fx + y * fy + k + c as u32 * 61;
                if c == 3 {
                    (40 + (v ^ (x / 9)) % 216) as u8
                } else {
                    (v % 256) as u8
                }
            })
            .collect();
        *channel = Channel {
            id: channel.id,
            data: data.into(),
        };
    }
    layer
}

/// An opaque background under one patterned layer per blend mode the oracle
/// covers, at seeded offsets, with one layer masked and one clipped.
pub(crate) fn seeded_doc(seed: u64) -> pictura_core::Document {
    let mut rng = Lcg(seed);
    let mut layers = vec![solid(
        "Background",
        PsdRect {
            top: 0,
            left: 0,
            bottom: H as i32,
            right: W as i32,
        },
        (90, 140, 200),
        255,
        BlendMode::Normal,
        255,
    )];
    let modes = [
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
    for (i, mode) in modes.into_iter().enumerate() {
        let left = rng.below(W - 120) as i32 - 30;
        let top = rng.below(H - 90) as i32 - 20;
        let rect = PsdRect {
            top,
            left,
            bottom: top + 80 + rng.below(300) as i32,
            right: left + 100 + rng.below(400) as i32,
        };
        let mut layer = patterned(&format!("L{i}"), rect, mode, &mut rng);
        if i == 3 {
            let (w, h) = (rect.width() as usize, rect.height() as usize);
            layer.mask = Some(LayerMask {
                rect,
                default_color: 0,
                data: Some(
                    (0..w * h)
                        .map(|p| ((p % w) * 255 / w.max(1)) as u8)
                        .collect::<Vec<u8>>()
                        .into(),
                ),
                ..Default::default()
            });
        }
        if i == 9 {
            layer.clipping = true;
        }
        layers.push(layer);
    }
    doc(W, H, layers)
}

fn identity(buffer: PixelBuffer) -> PixelBuffer {
    buffer
}

fn crop_planar(full: &PixelBuffer, rect: PsdRect) -> Vec<[u8; 4]> {
    let plane = (full.width * full.height) as usize;
    let mut out = Vec::new();
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            let inside = x >= 0 && y >= 0 && (x as u32) < full.width && (y as u32) < full.height;
            out.push(if inside {
                let i = y as usize * full.width as usize + x as usize;
                [
                    full.data[i],
                    full.data[plane + i],
                    full.data[2 * plane + i],
                    full.data[3 * plane + i],
                ]
            } else {
                [0; 4]
            });
        }
    }
    out
}

fn planar_pixels(buffer: &PixelBuffer) -> Vec<[u8; 4]> {
    crop_planar(
        buffer,
        PsdRect {
            top: 0,
            left: 0,
            bottom: buffer.height as i32,
            right: buffer.width as i32,
        },
    )
}

fn random_rect(rng: &mut Lcg, lw: u32, lh: u32) -> PsdRect {
    let left = rng.below(lw) as i32 - 8;
    let top = rng.below(lh) as i32 - 8;
    PsdRect {
        top,
        left,
        bottom: top + 1 + rng.below(lh / 2 + 16) as i32,
        right: left + 1 + rng.below(lw / 2 + 16) as i32,
    }
}

fn reference(full: &PixelBuffer) -> ViewPyramid {
    ViewPyramid::rebuild(Planes::planar(full.width, full.height, &full.data).expect("planes"))
}

fn assert_level(
    d: &pictura_core::Document,
    full: &PixelBuffer,
    pyramid: &ViewPyramid,
    level: usize,
    rect: PsdRect,
) {
    let planes = Planes::planar(full.width, full.height, &full.data).expect("planes");
    let expected = pyramid.crop(planes, level, rect);
    let got = composite_region_level(d, rect, level, &identity);
    assert_eq!(
        got.data(),
        expected.data(),
        "level {level} rect {rect:?} differs from the whole rebuild"
    );
}

#[test]
fn composite_region_matches_full_crop() {
    for seed in [1u64, 7, 42] {
        let d = seeded_doc(seed);
        let full = composite_rgba(&d);
        let pyramid = reference(&full);
        let mut rng = Lcg(seed ^ 0x5eed);
        for _ in 0..6 {
            let rect = random_rect(&mut rng, W, H);
            let region = composite_region(&d, rect);
            assert_eq!(
                (region.width, region.height),
                (rect.width() as u32, rect.height() as u32)
            );
            assert_eq!(
                planar_pixels(&region),
                crop_planar(&full, rect),
                "seed {seed} level 0 rect {rect:?}"
            );
        }
        for level in 0..=3usize {
            let (lw, lh) = level_size(W, H, level);
            for _ in 0..4 {
                assert_level(&d, &full, &pyramid, level, random_rect(&mut rng, lw, lh));
            }
        }
    }
}

#[test]
fn composite_region_folds_odd_edges() {
    let d = seeded_doc(3);
    let full = composite_rgba(&d);
    let pyramid = reference(&full);
    assert_eq!(pyramid.level_count(), 4, "the fixture is four levels deep");
    for level in 1..=3usize {
        let (lw, lh) = level_size(W, H, level);
        let (lw, lh) = (lw as i32, lh as i32);
        for rect in [
            PsdRect {
                top: lh - 5,
                left: lw - 7,
                bottom: lh,
                right: lw,
            },
            PsdRect {
                top: 3,
                left: lw - 1,
                bottom: lh,
                right: lw,
            },
            PsdRect {
                top: 0,
                left: 0,
                bottom: lh,
                right: lw,
            },
        ] {
            assert_level(&d, &full, &pyramid, level, rect);
        }
    }
}

#[test]
fn composite_region_outside_is_transparent() {
    let d = seeded_doc(5);
    let full = composite_rgba(&d);
    let rect = PsdRect {
        top: H as i32 - 10,
        left: W as i32 - 10,
        bottom: H as i32 + 10,
        right: W as i32 + 10,
    };
    let region = composite_region(&d, rect);
    let pixels = planar_pixels(&region);
    assert_eq!(pixels, crop_planar(&full, rect));
    assert_eq!(pixels[19 * 20 + 19], [0; 4], "outside the document");
    assert_ne!(pixels[0], [0; 4], "inside the document");
    let pyramid = reference(&full);
    let (lw, lh) = level_size(W, H, 2);
    assert_level(
        &d,
        &full,
        &pyramid,
        2,
        PsdRect {
            top: lh as i32 - 4,
            left: -4,
            bottom: lh as i32 + 4,
            right: 4,
        },
    );
    let _ = lw;
}

#[test]
fn streamed_region_equals_unbanded() {
    let d = seeded_doc(11);
    let largest = AtomicU64::new(0);
    let source = |r: PsdRect| {
        largest.fetch_max((r.width() as u64) * (r.height() as u64), Ordering::Relaxed);
        composite_region(&d, r)
    };
    let (lw, lh) = level_size(W, H, 2);
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: lh as i32,
        right: lw as i32,
    };
    let footprint = u64::from(W) * u64::from(H);
    let streamed = region_level_chunked(&source, (W, H), whole, 2, footprint / 8);
    assert!(
        largest.load(Ordering::Relaxed) <= footprint / 8,
        "no chunk is larger than the band budget"
    );
    let unbanded = region_level_chunked(&source, (W, H), whole, 2, u64::MAX);
    assert_eq!(streamed.data(), unbanded.data());
}

#[test]
fn flatten_composites_by_band() {
    let mut d = seeded_doc(13);
    let mut reference = d.clone();
    let white = solid(
        "white",
        PsdRect {
            top: 0,
            left: 0,
            bottom: H as i32,
            right: W as i32,
        },
        (255, 255, 255),
        255,
        BlendMode::Normal,
        255,
    );
    let mut layers = vec![white];
    layers.extend(reference.layers.iter().cloned());
    reference.layers = pictura_core::LayerStack::from_layers(layers);
    let expected = composite_rgba(&reference);

    crate::flatten(&mut d).expect("flatten");
    let layer = &d.layers[0];
    let n = (W * H) as usize;
    for (k, id) in [0i16, 1, 2, -1].into_iter().enumerate() {
        let channel = layer
            .channels
            .iter()
            .find(|c| c.id == id)
            .expect("a channel");
        assert!(channel.data.is_tiled(), "the flattened channel is tiled");
        let mut got = vec![0u8; n];
        channel.data.read_into(0, &mut got);
        assert_eq!(
            got,
            expected.data.as_ref()[k * n..(k + 1) * n],
            "channel {id}"
        );
    }

    // A document that is mostly one colour flattens to mostly uniform tiles:
    // no canvas-sized buffer is kept beside the new layer.
    let mut flat = doc(
        2048,
        1024,
        vec![solid(
            "dot",
            PsdRect {
                top: 10,
                left: 10,
                bottom: 20,
                right: 20,
            },
            (200, 10, 10),
            255,
            BlendMode::Normal,
            255,
        )],
    );
    crate::flatten(&mut flat).expect("flatten");
    let red = &flat.layers[0].channels[0].data;
    assert!(
        red.allocated_bytes() <= 256 * 256,
        "only the tile holding the dot is dense: {} bytes",
        red.allocated_bytes()
    );
}
