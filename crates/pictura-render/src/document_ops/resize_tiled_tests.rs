//! Image Size through the tiled resampler (#1293) against the dense
//! resamplers it replaced (`resample_plane`, `native_store::resize_samples`).
//! Under `cfg(test)` the window budget is small, so the larger resizes here
//! also split their blocks.

use pictura_core::memory::eviction::{self, run_pass};
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerMask, LayerStack, Plane, PsdRect,
    Residency, Samples, SourceChannels, SourcePlanes, TileSample,
};
use pictura_ops::Resample;

use super::native_store::resize_samples;
use super::{resample_plane, resize_document};

const RESAMPLES: [Resample; 3] = [Resample::Nearest, Resample::Bilinear, Resample::Bicubic];

fn rect(left: i32, top: i32, w: i32, h: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom: top + h,
        right: left + w,
    }
}

fn seeded(n: usize, seed: usize) -> Vec<u8> {
    (0..n)
        .map(|i| ((i * 37 + seed * 101) % 251) as u8)
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

fn dims(r: PsdRect) -> (usize, usize) {
    (r.width() as usize, r.height() as usize)
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
        ..Default::default()
    }
}

/// A 300 × 200 document: a full-canvas layer, a layer hanging off the
/// left edge with a smaller mask inside it, and an alpha channel.
fn document() -> Document {
    let (w, h) = (300, 200);
    let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
    let mut offset = layer("offset", rect(-7, 5, 130, 90), 3);
    let mr = rect(4, 11, 70, 60);
    offset.mask = Some(LayerMask {
        rect: mr,
        default_color: 255,
        data: Some(tiled(seeded(70 * 60, 9), 70, 60)),
        ..Default::default()
    });
    doc.layers = LayerStack::from_layers(vec![layer("full", rect(0, 0, w, h), 1), offset]);
    doc.channels = vec![Channel {
        id: 4,
        data: tiled(seeded((w * h) as usize, 5), w as usize, h as usize),
    }];
    doc
}

/// Assert a resized 8-bit plane is tiled and equals the dense resampler run
/// over its source.
fn assert_dense(
    what: &str,
    old: &Plane<u8>,
    from: (usize, usize),
    new: &Plane<u8>,
    to: (usize, usize),
    r: Resample,
) {
    assert!(new.is_tiled(), "{what}: not tiled");
    let want = resample_plane(
        &every(old),
        from.0 as u32,
        from.1 as u32,
        to.0 as u32,
        to.1 as u32,
        r,
    );
    assert!(every(new) == want, "{what} {r:?} {from:?} -> {to:?}");
}

// [ocr_resize_exact]
#[test]
fn tiled_image_size_matches_the_dense_resampler() {
    for (w, h) in [(450, 300), (150, 100), (21, 62), (699, 200)] {
        for r in RESAMPLES {
            let before = document();
            let mut doc = before.clone();
            pictura_core::assert_no_materialization(|| {
                resize_document(&mut doc, w, h, r).expect("resizes");
            });
            assert_eq!((doc.width, doc.height), (w, h));
            for (new, old) in doc.layers.iter().zip(before.layers.iter()) {
                for (nc, oc) in new.channels.iter().zip(&old.channels) {
                    let what = format!("{} channel {}", new.name, nc.id);
                    assert_dense(&what, &oc.data, dims(old.rect), &nc.data, dims(new.rect), r);
                }
                if let (Some(nm), Some(om)) = (&new.mask, &old.mask) {
                    let (nd, od) = (nm.data.as_ref().unwrap(), om.data.as_ref().unwrap());
                    let what = format!("{} mask", new.name);
                    assert_dense(&what, od, dims(om.rect), nd, dims(nm.rect), r);
                }
            }
            let to = (w as usize, h as usize);
            let from = (before.width as usize, before.height as usize);
            assert_dense(
                "alpha",
                &before.channels[0].data,
                from,
                &doc.channels[0].data,
                to,
                r,
            );
        }
    }
}

/// A 16- or 32-bit sample from a seed, spread across the type's range.
trait Seeded: TileSample {
    fn seeded(i: usize, seed: usize) -> Self;
    fn wrap(plane: Plane<Self>) -> Samples;
}

impl Seeded for u16 {
    fn seeded(i: usize, seed: usize) -> Self {
        ((i * 7919 + seed * 104_729) % 65_536) as u16
    }
    fn wrap(plane: Plane<Self>) -> Samples {
        Samples::U16(plane)
    }
}

impl Seeded for f32 {
    fn seeded(i: usize, seed: usize) -> Self {
        ((i as f32) * 0.37 + seed as f32).sin() * 1.25
    }
    fn wrap(plane: Plane<Self>) -> Samples {
        Samples::F32(plane)
    }
}

fn store<T: Seeded>(w: usize, h: usize, planes: usize, seed: usize) -> Samples {
    let data: Vec<T> = (0..w * h * planes).map(|i| T::seeded(i, seed)).collect();
    let mut plane = Plane::from(data);
    assert!(plane.tile(w, h));
    T::wrap(plane)
}

/// A plane's samples as bytes, so 32-bit planes compare bit for bit.
fn bits(s: &Samples) -> Vec<u8> {
    match s {
        Samples::U8(p) => every(p),
        Samples::U16(p) => every(p).iter().flat_map(|v| v.to_ne_bytes()).collect(),
        Samples::F32(p) => every(p).iter().flat_map(|v| v.to_ne_bytes()).collect(),
    }
}

fn is_tiled(s: &Samples) -> bool {
    match s {
        Samples::U8(p) => p.is_tiled(),
        Samples::U16(p) => p.is_tiled(),
        Samples::F32(p) => p.is_tiled(),
    }
}

fn deep_document<T: Seeded>(depth: BitDepth) -> Document {
    let mut doc = document();
    doc.depth = depth;
    doc.source_depth = Some(depth);
    let (w, h) = (doc.width as usize, doc.height as usize);
    doc.source_planes = Some(SourcePlanes {
        depth,
        width: doc.width,
        height: doc.height,
        samples: store::<T>(w, h, 3, 11),
    });
    let offset = &mut doc.layers[1];
    let (lw, lh) = dims(offset.rect);
    let (mw, mh) = dims(offset.mask.as_ref().unwrap().rect);
    let mut planes: Vec<(i16, Samples)> = [0i16, 1, 2, -1]
        .into_iter()
        .map(|id| (id, store::<T>(lw, lh, 1, 20 + id.unsigned_abs() as usize)))
        .collect();
    planes.push((-2, store::<T>(mw, mh, 1, 30)));
    offset.source_channels = Some(SourceChannels::new(depth, offset.rect, planes));
    doc
}

fn assert_deep<T: Seeded>(depth: BitDepth) {
    let before = deep_document::<T>(depth);
    let mut doc = before.clone();
    resize_document(&mut doc, 450, 300, Resample::Bicubic).expect("resizes");
    let (old, new) = (&before.layers[1], &doc.layers[1]);
    let old_store = old.source_channels.as_ref().unwrap();
    let new_store = new.source_channels.as_ref().unwrap();
    assert_eq!(new_store.rect, new.rect);
    let mask = |l: &Layer| l.mask.as_ref().unwrap().rect;
    for ((id, was), (_, now)) in old_store.planes.iter().zip(&new_store.planes) {
        let (from, to) = if *id == -2 {
            (dims(mask(old)), dims(mask(new)))
        } else {
            (dims(old.rect), dims(new.rect))
        };
        assert!(is_tiled(now), "{depth:?} plane {id} not tiled");
        let want = resize_samples(was, from.0, from.1, to.0, to.1, Resample::Bicubic);
        assert!(bits(now) == bits(&want), "{depth:?} plane {id}");
    }
    let (was, now) = (
        before.source_planes.as_ref().unwrap(),
        doc.source_planes.as_ref().unwrap(),
    );
    assert_eq!((now.width, now.height), (450, 300));
    assert!(is_tiled(&now.samples), "{depth:?} document store not tiled");
    let want = resize_samples(&was.samples, 300, 200, 450, 300, Resample::Bicubic);
    assert!(
        bits(&now.samples) == bits(&want),
        "{depth:?} document store"
    );
}

// [ocr_resize_deep]
#[test]
fn retained_stores_resize_through_tiles() {
    assert_deep::<u16>(BitDepth::Sixteen);
    assert_deep::<f32>(BitDepth::ThirtyTwo);
}

// [ocr_uniform_tiles]
#[test]
fn a_solid_document_resizes_into_uniform_tiles() {
    let side = 2048u32;
    let rgba = [90u8, 160, 30, 255].repeat((side * side) as usize);
    let mut doc = Document::from_rgba("solid", side, side, &rgba);
    resize_document(&mut doc, 3072, 3072, Resample::Bilinear).expect("resizes");
    let dense = 3072 * 3072;
    for c in &doc.layers[0].channels {
        assert!(
            c.data.allocated_bytes() * 4 < dense,
            "channel {}: {} of {dense} bytes",
            c.id,
            c.data.allocated_bytes()
        );
    }
}

// [ocr_output_spills]
#[test]
fn resized_tiles_spill_and_read_back() {
    let dir = pictura_core::scratch::default_volume()
        .with_file_name("test-scratch")
        .join(format!("render-resize-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    pictura_core::scratch::set_volumes(std::slice::from_ref(&dir));
    let side = 1024;
    let mut doc = Document::new(side as u32, side as u32, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = LayerStack::from_layers(vec![layer("seeded", rect(0, 0, side, side), 1)]);
    let source = every(&doc.layers[0].channels[0].data);
    resize_document(&mut doc, 1536, 1536, Resample::Bilinear).expect("resizes");
    let resized = &doc.layers[0].channels[0].data;
    for _ in 0..6 {
        run_pass(u64::MAX);
    }
    let spilled = (0..resized.tile_count())
        .filter(|&i| resized.tile_residency(i) == Some(Residency::Spilled))
        .count();
    assert!(spilled > 0, "no tile spilled: {:?}", eviction::counts());
    let want = resample_plane(&source, 1024, 1024, 1536, 1536, Resample::Bilinear);
    assert!(
        every(resized) == want,
        "spilled tiles read back differently"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
