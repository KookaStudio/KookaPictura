//! The tiled warp rasterizer (#1292) against the dense one it replaced
//! (`warp_oracle.rs`). Under `cfg(test)` the window budget is small, so every
//! warp here also splits its blocks.

use pictura_core::job::{self, Cancelled, JobProgress};
use pictura_core::memory::eviction::{self, run_pass};
use pictura_core::warp::{BezierMesh, Warp, WarpStyle};
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerMask, Plane, PsdRect, Residency, Sample,
    Samples, SourceChannels, SourcePlanes, TileSample,
};

use super::kernel::Interpolation;
use super::paths::resolve_path;
use super::transform_layer_warp;
use super::warp::warp_gray;
use super::warp_oracle::{dense_warp_planes, Planes};
use super::warp_raster::{triangulate, warp_tiled, Kind, Source, SrcPlane, Triangles};

const INTERPOLATIONS: [Interpolation; 6] = [
    Interpolation::Nearest,
    Interpolation::Bilinear,
    Interpolation::Bicubic,
    Interpolation::BicubicSmoother,
    Interpolation::BicubicSharper,
    Interpolation::BicubicAutomatic,
];

type Map = Box<dyn Fn(f64, f64) -> (f64, f64) + Sync>;

/// A `w × h` rect off the origin, so local and document coordinates differ.
fn rect(w: i32, h: i32) -> PsdRect {
    PsdRect {
        top: 7,
        left: -3,
        bottom: 7 + h,
        right: -3 + w,
    }
}

fn bounds(r: PsdRect) -> [f64; 4] {
    [
        f64::from(r.left),
        f64::from(r.top),
        f64::from(r.right),
        f64::from(r.bottom),
    ]
}

fn seeded(w: usize, h: usize, seed: usize) -> Vec<u8> {
    (0..w * h)
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

fn is_tiled(s: &Samples) -> bool {
    match s {
        Samples::U8(p) => p.is_tiled(),
        Samples::U16(p) => p.is_tiled(),
        Samples::F32(p) => p.is_tiled(),
    }
}

/// A plane's samples as bytes, so 32-bit planes compare bit for bit.
fn bits(s: &Samples) -> Vec<u8> {
    match s {
        Samples::U8(p) => every(p),
        Samples::U16(p) => u16::bytes(&every(p)).to_vec(),
        Samples::F32(p) => f32::bytes(&every(p)).to_vec(),
    }
}

/// The maps the tests drive: an arc, a twist, a fold (the later cell wins), a
/// strong shrink, and a twisted enlargement.
fn maps(r: PsdRect) -> Vec<(&'static str, Map)> {
    let b = bounds(r);
    let arc = Warp::preset(WarpStyle::Arc, 50.0, b);
    let twist = Warp::preset(WarpStyle::Twist, 60.0, b);
    let shrink = Warp::preset(WarpStyle::Arc, 30.0, b);
    let grow = Warp::preset(WarpStyle::Twist, -40.0, b);
    let cx = f64::from(r.left + r.right) / 2.0;
    vec![
        ("arc", Box::new(move |x, y| arc.map(x, y))),
        ("twist", Box::new(move |x, y| twist.map(x, y))),
        (
            "fold",
            Box::new(move |x, y| (cx + (x - cx).abs() / 2.0, y + 0.1 * x)),
        ),
        (
            "shrink",
            Box::new(move |x, y| {
                let (u, v) = shrink.map(x, y);
                (0.05 * u + 3.0, 0.05 * v + 2.0)
            }),
        ),
        (
            "grow",
            Box::new(move |x, y| {
                let (u, v) = grow.map(x, y);
                (2.5 * u - 11.0, 2.5 * v + 4.0)
            }),
        ),
    ]
}

/// The dense rasterizer over unit planes, with `alpha` premultiplying the rest.
fn dense(
    planes: Vec<Vec<f32>>,
    r: PsdRect,
    alpha: Option<usize>,
    t: &Triangles,
    interp: Interpolation,
    hdr: bool,
) -> Vec<Vec<f32>> {
    let (w, h) = (r.width() as usize, r.height() as usize);
    dense_warp_planes(&Planes::new(planes, w, h, alpha), r, t, interp, hdr)
}

fn unit_u8(v: &[u8]) -> Vec<f32> {
    v.iter().map(|x| x.to_unit() as f32).collect()
}

fn to_u8(v: &[f32]) -> Vec<u8> {
    v.iter().map(|x| u8::from_unit(f64::from(*x))).collect()
}

/// The dense mask warp as `warp_gray` did it before #1292.
fn dense_gray(
    gray: Vec<f32>,
    r: PsdRect,
    default: f32,
    t: &Triangles,
    interp: Interpolation,
) -> Vec<f32> {
    let ones = vec![1.0f32; gray.len()];
    let mut out = dense(vec![gray, ones], r, Some(1), t, interp, false);
    let alpha = out.pop().unwrap();
    let mut gray = out.pop().unwrap();
    gray.iter_mut()
        .zip(&alpha)
        .for_each(|(g, a)| *g = *g * a + default * (1.0 - a));
    gray
}

// [ocw_warp_exact]
#[test]
fn tiled_warp_matches_the_dense_rasterizer() {
    let (w, h) = (300usize, 220usize);
    let r = rect(w as i32, h as i32);
    let data: Vec<Vec<u8>> = (0..4).map(|k| seeded(w, h, k)).collect();
    let planes: Vec<Plane<u8>> = data.iter().map(|d| tiled(d.clone(), w, h)).collect();
    let mask = tiled(seeded(w, h, 9), w, h);
    for (name, map) in maps(r) {
        let t = triangulate(r, &*map).expect("triangulates");
        for interp in INTERPOLATIONS {
            let src = Source {
                planes: planes.iter().map(SrcPlane::U8).collect(),
                rect: r,
                alpha: Some(0),
            };
            let out = warp_tiled(&src, &t, interp, false, &[Kind::U8; 4], &|_| {});
            let oracle = dense(
                data.iter().map(|d| unit_u8(d)).collect(),
                r,
                Some(0),
                &t,
                interp,
                false,
            );
            for (k, (got, want)) in out.iter().zip(&oracle).enumerate() {
                assert!(is_tiled(got), "{name} {interp:?} plane {k}");
                assert!(bits(got) == to_u8(want), "{name} {interp:?} plane {k}");
            }
            let gray = warp_gray(SrcPlane::U8(&mask), Kind::U8, r, 0.25, &t, interp);
            let want = to_u8(&dense_gray(unit_u8(&seeded(w, h, 9)), r, 0.25, &t, interp));
            assert!(is_tiled(&gray), "{name} {interp:?} mask");
            assert!(bits(&gray) == want, "{name} {interp:?} mask");
        }
    }
    // The layer op stores what the rasterizer builds: tiled channels and mask.
    let layer = Layer {
        name: "L".into(),
        rect: r,
        channels: (0..4)
            .map(|k| Channel {
                id: k as i16 - 1,
                data: data[k].clone().into(),
            })
            .collect(),
        mask: Some(LayerMask {
            rect: r,
            default_color: 255,
            data: Some(seeded(w, h, 9).into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut doc = Document::new(400, 300, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    let warp = Warp::preset(WarpStyle::Arc, 50.0, bounds(r));
    assert!(transform_layer_warp(
        &mut doc,
        "0",
        &warp,
        None,
        Interpolation::Bilinear
    ));
    let layer = resolve_path(&doc, "0").expect("layer");
    let t = triangulate(r, &|x, y| warp.map(x, y)).expect("triangulates");
    let oracle = dense(
        data.iter().map(|d| unit_u8(d)).collect(),
        r,
        Some(0),
        &t,
        Interpolation::Bilinear,
        false,
    );
    for (c, want) in layer.channels.iter().zip(&oracle) {
        assert!(c.data.is_tiled(), "channel {}", c.id);
        assert!(every(&c.data) == to_u8(want), "channel {}", c.id);
    }
    let mask = layer
        .mask
        .as_ref()
        .and_then(|m| m.data.as_ref())
        .expect("mask");
    assert!(mask.is_tiled());
    let want = dense_gray(
        unit_u8(&seeded(w, h, 9)),
        r,
        1.0,
        &t,
        Interpolation::Bilinear,
    );
    assert!(every(mask) == to_u8(&want));
}

/// Warp a layer whose retained store is `store(seed)` per plane through an
/// arc, and check the store against the dense rasterizer.
fn deep_case(
    depth: BitDepth,
    store: impl Fn(usize) -> Samples,
    unit: impl Fn(&Samples) -> Vec<f32>,
) {
    let (w, h) = (180usize, 140usize);
    let r = rect(w as i32, h as i32);
    let ids = [-2i16, -1, 0, 1, 2];
    let planes: Vec<(i16, Samples)> = ids
        .iter()
        .enumerate()
        .map(|(k, id)| (*id, store(k)))
        .collect();
    let layer = Layer {
        name: "L".into(),
        rect: r,
        channels: ids[1..]
            .iter()
            .enumerate()
            .map(|(k, id)| Channel {
                id: *id,
                data: seeded(w, h, k).into(),
            })
            .collect(),
        mask: Some(LayerMask {
            rect: r,
            default_color: 0,
            data: Some(seeded(w, h, 7).into()),
            ..Default::default()
        }),
        source_channels: Some(SourceChannels::new(depth, r, planes.clone())),
        ..Default::default()
    };
    let mut doc = Document::new(300, 200, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc.source_depth = Some(depth);
    doc.source_planes = Some(SourcePlanes {
        depth,
        width: 300,
        height: 200,
        samples: match depth {
            BitDepth::ThirtyTwo => Samples::F32(vec![0.0; 3 * 300 * 200].into()),
            _ => Samples::U16(vec![0; 3 * 300 * 200].into()),
        },
    });
    let warp = Warp::preset(WarpStyle::Arc, 50.0, bounds(r));
    let interp = Interpolation::Bicubic;
    assert!(transform_layer_warp(&mut doc, "0", &warp, None, interp));
    let t = triangulate(r, &|x, y| warp.map(x, y)).expect("triangulates");
    let hdr = depth == BitDepth::ThirtyTwo;
    let layer_planes: Vec<Vec<f32>> = planes[1..].iter().map(|(_, s)| unit(s)).collect();
    let oracle = dense(layer_planes, r, Some(0), &t, interp, hdr);
    let to_kind = |like: &Samples, v: &[f32]| match like {
        Samples::U16(_) => {
            let out: Vec<u16> = v.iter().map(|x| u16::from_unit(f64::from(*x))).collect();
            u16::bytes(&out).to_vec()
        }
        _ => f32::bytes(v).to_vec(),
    };
    let got = resolve_path(&doc, "0")
        .and_then(|l| l.source_channels.clone())
        .expect("a warp keeps a resampled store");
    let mask_want = dense_gray(unit(&planes[0].1), r, 0.0, &t, interp);
    for (id, samples) in &got.planes {
        assert!(is_tiled(samples), "{depth:?} plane {id}");
        let want = if *id == -2 {
            to_kind(samples, &mask_want)
        } else {
            to_kind(samples, &oracle[(*id + 1) as usize])
        };
        assert!(bits(samples) == want, "{depth:?} plane {id}");
    }
}

// [ocw_warp_deep]
#[test]
fn deep_stores_warp_through_tiles() {
    let (w, h) = (180usize, 140usize);
    deep_case(
        BitDepth::Sixteen,
        |k| {
            Samples::U16(tiled(
                (0..w * h)
                    .map(|i| ((i * 2731 + k * 977) % 65521) as u16)
                    .collect(),
                w,
                h,
            ))
        },
        |s| match s {
            Samples::U16(p) => every(p).iter().map(|v| v.to_unit() as f32).collect(),
            _ => unreachable!(),
        },
    );
    deep_case(
        BitDepth::ThirtyTwo,
        // 32-bit samples run past 1 (HDR) and include NaN, read as zero.
        |k| {
            Samples::F32(tiled(
                (0..w * h)
                    .map(|i| {
                        if i % 97 == 5 {
                            f32::NAN
                        } else {
                            ((i * 37 + k * 11) % 1013) as f32 / 700.0
                        }
                    })
                    .collect(),
                w,
                h,
            ))
        },
        |s| match s {
            Samples::F32(p) => every(p)
                .iter()
                .map(|v| if v.is_nan() { 0.0 } else { *v })
                .collect(),
            _ => unreachable!(),
        },
    );
}

// [ocw_uniform_tiles]
#[test]
fn a_solid_layer_warps_into_uniform_tiles() {
    let side = 2048u32;
    let rgba = [90u8, 160, 30, 255].repeat((side * side) as usize);
    let mut doc = Document::from_rgba("solid", side, side, &rgba);
    let b = [0.0, 0.0, f64::from(side), f64::from(side)];
    let warp = Warp::preset(WarpStyle::Flag, 10.0, b);
    assert!(transform_layer_warp(
        &mut doc,
        "0",
        &warp,
        None,
        Interpolation::Bilinear
    ));
    let layer = resolve_path(&doc, "0").expect("layer");
    let dense = (layer.rect.width() * layer.rect.height()) as usize;
    for c in &layer.channels {
        // Only tiles the wavy edge crosses hold samples.
        assert!(
            c.data.allocated_bytes() * 4 < dense,
            "channel {}: {} of {dense} bytes",
            c.id,
            c.data.allocated_bytes()
        );
    }
}

// [ocw_output_spills]
#[test]
fn warped_tiles_spill_and_read_back() {
    let dir = pictura_core::scratch::default_volume()
        .with_file_name("test-scratch")
        .join(format!("render-warp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    pictura_core::scratch::set_volumes(std::slice::from_ref(&dir));
    let (w, h) = (1024usize, 1024usize);
    let r = rect(w as i32, h as i32);
    let data = seeded(w, h, 1);
    let plane = tiled(data.clone(), w, h);
    let warp = Warp::preset(WarpStyle::Arc, 40.0, bounds(r));
    let t = triangulate(r, &|x, y| warp.map(x, y)).expect("triangulates");
    let src = Source {
        planes: vec![SrcPlane::U8(&plane)],
        rect: r,
        alpha: None,
    };
    let out = warp_tiled(
        &src,
        &t,
        Interpolation::Bilinear,
        false,
        &[Kind::U8],
        &|_| {},
    );
    let Samples::U8(warped) = &out[0] else {
        panic!("an 8-bit plane");
    };
    for _ in 0..6 {
        run_pass(u64::MAX);
    }
    let spilled = (0..warped.tile_count())
        .filter(|&i| warped.tile_residency(i) == Some(Residency::Spilled))
        .count();
    assert!(spilled > 0, "no tile spilled: {:?}", eviction::counts());
    let want = dense(
        vec![unit_u8(&data)],
        r,
        None,
        &t,
        Interpolation::Bilinear,
        false,
    );
    assert!(
        every(warped) == to_u8(&want[0]),
        "spilled tiles read back differently"
    );
    let _ = std::fs::remove_dir_all(dir);
}

// [ocw_cancel]
#[test]
fn a_cancelled_warp_changes_nothing() {
    let (w, h) = (300u32, 200u32);
    let rgba: Vec<u8> = (0..w * h * 4).map(|i| (i * 31 % 253) as u8).collect();
    let original = Document::from_rgba("t", w, h, &rgba);
    let mut doc = original.clone();
    let b = [0.0, 0.0, f64::from(w), f64::from(h)];
    let mut mesh = BezierMesh::identity(b, 1, 1);
    mesh.points[5] = [120.0, 90.0];
    let warp = Warp::custom(mesh, b);
    let job = JobProgress::new();
    job.cancel();
    assert_eq!(
        job::run(&job, || transform_layer_warp(
            &mut doc,
            "0",
            &warp,
            None,
            Interpolation::Bicubic
        )),
        Err(Cancelled)
    );
    let (before, after) = (
        resolve_path(&original, "0").expect("layer"),
        resolve_path(&doc, "0").expect("layer"),
    );
    assert_eq!(before.rect, after.rect);
    for (a, b) in before.channels.iter().zip(&after.channels) {
        assert!(a.data == b.data, "channel {}", a.id);
    }
}
