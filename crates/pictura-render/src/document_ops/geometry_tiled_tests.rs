//! Canvas Size, the exact turns and flips, and arbitrary Rotate through the
//! tiled driver (#1294) against the dense paths they replaced
//! (`extend_channel`, `extend_samples`, the dense Background extension,
//! `orient::remap_plane`, `remap_samples`, and the dense inverse map). The
//! fixtures are wider than one block and taller than one strip.

use pictura_core::memory::eviction::{self, run_pass};
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerMask, LayerStack, PixelBuffer, Plane,
    PsdRect, Residency, Samples, SourceChannels, SourcePlanes, TileSample,
};
use pictura_ops::Anchor;

use super::super::canvas::{anchor_offset, extend_background_dense, extend_channel};
use super::super::native_store::{extend_samples, remap_samples, Remap};
use super::super::orient::remap_plane;
use super::super::rotate_in::rotate_plane_dense;
use super::super::{
    extend_background, flip_document, resize_canvas_document, rotate_document, rotate_document_in,
};

const W: i32 = 600;
const H: i32 = 300;

fn rect(left: i32, top: i32, w: i32, h: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom: top + h,
        right: left + w,
    }
}

fn dims(r: PsdRect) -> (usize, usize) {
    (r.width().max(0) as usize, r.height().max(0) as usize)
}

fn seeded(n: usize, seed: usize) -> Vec<u8> {
    (0..n)
        .map(|i| ((i * 37 + seed * 101 + i / 7) % 251) as u8)
        .collect()
}

fn tiled<T: TileSample>(data: Vec<T>, w: usize, h: usize) -> Plane<T> {
    let mut plane = Plane::from(data);
    assert!(plane.tile(w, h));
    plane
}

fn every<T: TileSample>(plane: &Plane<T>) -> Vec<T> {
    let mut out = vec![T::default(); plane.len()];
    plane.read_into(0, &mut out);
    out
}

/// A layer over `r` whose four channels are tiled seeded planes.
fn layer(name: &str, r: PsdRect, seed: usize) -> Layer {
    let (w, h) = dims(r);
    Layer {
        name: name.into(),
        rect: r,
        channels: [0, 1, 2, -1]
            .into_iter()
            .map(|id| Channel {
                id,
                data: tiled(seeded(w * h, seed + id.unsigned_abs() as usize), w, h),
            })
            .collect(),
        visible: true,
        opacity: 255,
        fill: 255,
        ..Default::default()
    }
}

/// A seeded 600 × 300 document: a full-canvas Background, an offset layer
/// with a mask of another rect, and an alpha channel.
fn seeded_doc() -> Document {
    let mut doc = Document::new(W as u32, H as u32, ColorMode::Rgb, BitDepth::Eight);
    let mut background = layer("Background", rect(0, 0, W, H), 1);
    background.background = true;
    let mut offset = layer("offset", rect(30, 20, 400, 230), 7);
    let mask_rect = rect(50, 10, 250, 190);
    let (mw, mh) = dims(mask_rect);
    offset.mask = Some(LayerMask {
        rect: mask_rect,
        default_color: 255,
        data: Some(tiled(seeded(mw * mh, 13), mw, mh)),
        ..Default::default()
    });
    doc.layers = LayerStack::from_layers(vec![background, offset]);
    doc.channels = vec![Channel {
        id: 3,
        data: tiled(seeded((W * H) as usize, 21), W as usize, H as usize),
    }];
    doc
}

fn seeded_samples(depth: BitDepth, n: usize, seed: usize) -> Samples {
    let bytes = seeded(n, seed);
    match depth {
        BitDepth::Sixteen => Samples::U16(
            bytes
                .iter()
                .enumerate()
                .map(|(i, &b)| u16::from(b) * 257 + (i % 199) as u16)
                .collect::<Vec<_>>()
                .into(),
        ),
        _ => Samples::F32(
            bytes
                .iter()
                .enumerate()
                .map(|(i, &b)| f32::from(b) / 255.0 + (i % 13) as f32 * 1e-4)
                .collect::<Vec<_>>()
                .into(),
        ),
    }
}

/// [`seeded_doc`] at `depth`: the offset layer keeps a retained store with a
/// `-2` plane at its mask rect, and the document a three-plane store.
fn deep_doc(depth: BitDepth) -> Document {
    let mut doc = seeded_doc();
    doc.source_depth = Some(depth);
    let offset = &mut doc.layers[1];
    let (lw, lh) = dims(offset.rect);
    let (mw, mh) = dims(offset.mask.as_ref().unwrap().rect);
    offset.source_channels = Some(SourceChannels::new(
        depth,
        offset.rect,
        vec![
            (0, seeded_samples(depth, lw * lh, 3)),
            (-2, seeded_samples(depth, mw * mh, 5)),
        ],
    ));
    doc.source_planes = Some(SourcePlanes {
        depth,
        width: W as u32,
        height: H as u32,
        samples: seeded_samples(depth, 3 * (W * H) as usize, 9),
    });
    doc
}

fn is_tiled(samples: &Samples) -> bool {
    match samples {
        Samples::U8(p) => p.is_tiled(),
        Samples::U16(p) => p.is_tiled(),
        Samples::F32(p) => p.is_tiled(),
    }
}

/// Each stacked `w × h` plane of `samples`, densely.
fn each_plane(samples: &Samples, (w, h): (usize, usize)) -> Vec<Samples> {
    let n = w * h;
    (0..samples.len() / n.max(1))
        .map(|c| samples.slice(c * n..(c + 1) * n).expect("whole planes"))
        .collect()
}

// [ocg_canvas_exact]
#[test]
fn tiled_canvas_size_matches_the_dense_paths() {
    let cases = [
        (900, 700, Anchor::Center),
        (300, 200, Anchor::BottomRight),
        (601, 299, Anchor::TopLeft),
    ];
    for (w, h, anchor) in cases {
        let before = seeded_doc();
        let mut doc = before.clone();
        resize_canvas_document(&mut doc, w, h, anchor).expect("resizes");
        let (dx, dy) = anchor_offset(anchor, W as u32, H as u32, w, h);

        let alpha = &doc.channels[0].data;
        assert!(alpha.is_tiled());
        let source = every(&before.channels[0].data);
        let want = extend_channel(&source, W as u32, H as u32, w, h, dx, dy);
        assert!(every(alpha) == want, "alpha at {w}×{h} {anchor:?}");
        for (old, new) in before.layers[1]
            .channels
            .iter()
            .zip(&doc.layers[1].channels)
        {
            assert!(new.data.shares(&old.data), "a layer plane was copied");
        }

        let mut dense = doc.clone();
        for channel in &mut dense.layers[0].channels {
            channel.data = Plane::from(every(&channel.data));
        }
        assert!(extend_background(&mut doc, [200, 100, 50]).expect("extends"));
        assert!(extend_background_dense(&mut dense, [200, 100, 50]));
        assert_eq!(doc.layers[0].rect, dense.layers[0].rect);
        for (got, want) in doc.layers[0].channels.iter().zip(&dense.layers[0].channels) {
            assert!(got.data.is_tiled());
            assert!(
                every(&got.data) == every(&want.data),
                "Background channel {} at {w}×{h} {anchor:?}",
                got.id
            );
        }
    }
}

// [ocg_canvas_deep]
#[test]
fn the_document_store_shifts_through_tiles() {
    for depth in [BitDepth::Sixteen, BitDepth::ThirtyTwo] {
        let before = deep_doc(depth);
        let mut doc = before.clone();
        resize_canvas_document(&mut doc, 900, 700, Anchor::Center).expect("resizes");
        let (dx, dy) = anchor_offset(Anchor::Center, W as u32, H as u32, 900, 700);
        let store = doc.source_planes.as_ref().unwrap();
        assert_eq!((store.width, store.height), (900, 700));
        assert!(is_tiled(&store.samples));
        let old = &before.source_planes.as_ref().unwrap().samples;
        let want = extend_samples(old, W as usize, H as usize, 900, 700, dx, dy);
        assert_eq!(store.samples, want, "{depth:?}");
    }
}

// [ocg_canvas_uniform]
#[test]
fn a_uniform_extension_holds_no_allocation() {
    let side = 1024u32;
    let rgba = [255u8; 4].repeat((side * side) as usize);
    let mut doc = Document::from_rgba("white", side, side, &rgba);
    doc.layers[0].background = true;
    resize_canvas_document(&mut doc, 2048, 2048, Anchor::Center).expect("resizes");
    assert!(extend_background(&mut doc, [255, 255, 255]).expect("extends"));
    let dense = 2048 * 2048;
    for c in &doc.layers[0].channels {
        assert!(
            c.data.allocated_bytes() * 4 < dense,
            "channel {}: {} of {dense} bytes",
            c.id,
            c.data.allocated_bytes()
        );
    }
}

/// The five exact remaps as (engine call, dense pixel-buffer op, store remap).
type Orient = (
    fn(&mut Document),
    fn(&PixelBuffer) -> PixelBuffer,
    Remap,
    &'static str,
);

const ORIENTS: [Orient; 5] = [
    (
        |d| rotate_document(d, 1).unwrap(),
        pictura_ops::rotate90_cw,
        Remap::Rot90,
        "90 cw",
    ),
    (
        |d| rotate_document(d, 3).unwrap(),
        pictura_ops::rotate90_ccw,
        Remap::Rot270,
        "90 ccw",
    ),
    (
        |d| rotate_document(d, 2).unwrap(),
        pictura_ops::rotate180,
        Remap::Rot180,
        "180",
    ),
    (
        |d| flip_document(d, true).unwrap(),
        pictura_ops::flip_horizontal,
        Remap::FlipH,
        "flip h",
    ),
    (
        |d| flip_document(d, false).unwrap(),
        pictura_ops::flip_vertical,
        Remap::FlipV,
        "flip v",
    ),
];

fn remapped(data: &Plane<u8>, r: PsdRect, op: fn(&PixelBuffer) -> PixelBuffer) -> Vec<u8> {
    let (w, h) = dims(r);
    remap_plane(&every(data), w as u32, h as u32, op)
}

// [ocg_orient_exact]
#[test]
fn tiled_turns_and_flips_match_the_exact_remap() {
    for (apply, op, _, name) in ORIENTS {
        let before = seeded_doc();
        let mut doc = before.clone();
        apply(&mut doc);
        for (old, new) in before.layers.iter().zip(doc.layers.iter()) {
            for (a, b) in old.channels.iter().zip(&new.channels) {
                assert!(b.data.is_tiled(), "{name}: {} {}", new.name, b.id);
                let want = remapped(&a.data, old.rect, op);
                assert!(every(&b.data) == want, "{name}: {} {}", new.name, b.id);
            }
            if let (Some(a), Some(b)) = (&old.mask, &new.mask) {
                let got = b.data.as_ref().unwrap();
                assert!(got.is_tiled());
                let want = remapped(a.data.as_ref().unwrap(), a.rect, op);
                assert!(every(got) == want, "{name}: mask");
            }
        }
        let canvas = rect(0, 0, W, H);
        let want = remapped(&before.channels[0].data, canvas, op);
        assert!(doc.channels[0].data.is_tiled());
        assert!(every(&doc.channels[0].data) == want, "{name}: alpha");
    }
}

// [ocg_orient_deep]
#[test]
fn retained_stores_turn_through_tiles() {
    for depth in [BitDepth::Sixteen, BitDepth::ThirtyTwo] {
        for (apply, _, remap, name) in [ORIENTS[0], ORIENTS[3]] {
            let before = deep_doc(depth);
            let mut doc = before.clone();
            apply(&mut doc);
            let old = &before.layers[1];
            let store = doc.layers[1].source_channels.as_ref().unwrap();
            for ((id, a), (_, b)) in old
                .source_channels
                .as_ref()
                .unwrap()
                .planes
                .iter()
                .zip(&store.planes)
            {
                let r = if *id == -2 {
                    old.mask.as_ref().unwrap().rect
                } else {
                    old.rect
                };
                let (w, h) = dims(r);
                assert!(is_tiled(b), "{depth:?} {name} {id}");
                assert_eq!(b, &remap_samples(a, w, h, remap), "{depth:?} {name} {id}");
            }
            let old = &before.source_planes.as_ref().unwrap().samples;
            let new = &doc.source_planes.as_ref().unwrap().samples;
            assert!(is_tiled(new));
            assert_eq!(new, &remap_samples(old, W as usize, H as usize, remap));
        }
    }
}

/// Rotate a copy of `before` and check every plane against the dense map.
fn check_rotation(before: &Document, angle: f64, pivot: (f64, f64)) {
    let mut doc = before.clone();
    assert!(rotate_document_in(&mut doc, angle, pivot), "{angle}°");
    let canvas = (W as u32, H as u32);
    let dense = |data: &Plane<u8>, from: PsdRect, to: PsdRect| {
        rotate_plane_dense(&every(data), from, to, canvas, angle, pivot)
    };
    for (old, new) in before.layers.iter().zip(doc.layers.iter()) {
        for (a, b) in old.channels.iter().zip(&new.channels) {
            assert!(b.data.is_tiled(), "{angle}°: {} {}", new.name, b.id);
            let want = dense(&a.data, old.rect, new.rect);
            assert!(
                every(&b.data) == want,
                "{angle}° {pivot:?}: {} {}",
                new.name,
                b.id
            );
        }
        if let (Some(a), Some(b)) = (&old.mask, &new.mask) {
            let want = dense(a.data.as_ref().unwrap(), a.rect, b.rect);
            assert!(every(b.data.as_ref().unwrap()) == want, "{angle}°: mask");
        }
    }
    let from = rect(0, 0, W, H);
    let to = rect(0, 0, doc.width as i32, doc.height as i32);
    let want = dense(&before.channels[0].data, from, to);
    assert!(every(&doc.channels[0].data) == want, "{angle}°: alpha");
}

// [ocg_rotate_exact]
#[test]
fn tiled_rotate_matches_the_dense_inverse_map() {
    let before = seeded_doc();
    let centre = (W as f64 / 2.0, H as f64 / 2.0);
    for angle in [15.0, 45.0, -30.0, 123.4, 270.5] {
        check_rotation(&before, angle, centre);
    }
    // Off-centre quarter turns are resampled, and their corners land exactly
    // on integers, the window's tightest case.
    for angle in [33.0, 90.0, 180.0] {
        check_rotation(&before, angle, (100.0, 50.0));
    }
}

// [ocg_rotate_deep]
#[test]
fn retained_stores_rotate_through_tiles() {
    for depth in [BitDepth::Sixteen, BitDepth::ThirtyTwo] {
        let before = deep_doc(depth);
        let mut doc = before.clone();
        let pivot = (W as f64 / 2.0, H as f64 / 2.0);
        assert!(rotate_document_in(&mut doc, 20.0, pivot));
        let canvas = (W as u32, H as u32);
        let old = &before.layers[1];
        let new = &doc.layers[1];
        let store = new.source_channels.as_ref().unwrap();
        for ((id, a), (_, b)) in old
            .source_channels
            .as_ref()
            .unwrap()
            .planes
            .iter()
            .zip(&store.planes)
        {
            let (from, to) = if *id == -2 {
                (
                    old.mask.as_ref().unwrap().rect,
                    new.mask.as_ref().unwrap().rect,
                )
            } else {
                (old.rect, new.rect)
            };
            assert!(is_tiled(b), "{depth:?} {id}");
            let want = match a {
                Samples::U16(p) => Samples::U16(
                    rotate_plane_dense(&every(p), from, to, canvas, 20.0, pivot).into(),
                ),
                Samples::F32(p) => Samples::F32(
                    rotate_plane_dense(&every(p), from, to, canvas, 20.0, pivot).into(),
                ),
                Samples::U8(_) => unreachable!("a deep store"),
            };
            assert_eq!(b, &want, "{depth:?} {id}");
        }
        let old = &before.source_planes.as_ref().unwrap().samples;
        let new_store = doc.source_planes.as_ref().unwrap();
        assert!(is_tiled(&new_store.samples));
        let to = rect(0, 0, doc.width as i32, doc.height as i32);
        let from = rect(0, 0, W, H);
        let got = each_plane(
            &new_store.samples,
            (doc.width as usize, doc.height as usize),
        );
        for (c, plane) in each_plane(old, (W as usize, H as usize)).iter().enumerate() {
            let want = match plane {
                Samples::U16(p) => Samples::U16(
                    rotate_plane_dense(&every(p), from, to, canvas, 20.0, pivot).into(),
                ),
                Samples::F32(p) => Samples::F32(
                    rotate_plane_dense(&every(p), from, to, canvas, 20.0, pivot).into(),
                ),
                Samples::U8(_) => unreachable!("a deep store"),
            };
            assert_eq!(got[c], want, "{depth:?} store plane {c}");
        }
    }
}

// [ocg_output_spills]
#[test]
fn rotated_tiles_spill_and_read_back() {
    let _scratch = crate::test_support::scratch_guard();
    let dir = pictura_core::scratch::default_volume()
        .with_file_name("test-scratch")
        .join(format!("render-rotate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    pictura_core::scratch::set_volumes(std::slice::from_ref(&dir));
    let side = 1024;
    let mut doc = Document::new(side as u32, side as u32, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![layer("seeded", rect(0, 0, side, side), 1)]);
    let before = doc.clone();
    let pivot = (side as f64 / 2.0, side as f64 / 2.0);
    assert!(rotate_document_in(&mut doc, 15.0, pivot));
    let rotated = &doc.layers[0].channels[0].data;
    for _ in 0..6 {
        run_pass(u64::MAX);
    }
    let spilled = (0..rotated.tile_count())
        .filter(|&i| rotated.tile_residency(i) == Some(Residency::Spilled))
        .count();
    assert!(spilled > 0, "no tile spilled: {:?}", eviction::counts());
    let want = rotate_plane_dense(
        &every(&before.layers[0].channels[0].data),
        before.layers[0].rect,
        doc.layers[0].rect,
        (side as u32, side as u32),
        15.0,
        pivot,
    );
    assert!(
        every(rotated) == want,
        "spilled tiles read back differently"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
