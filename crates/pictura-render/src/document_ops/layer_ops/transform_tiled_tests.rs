//! The tiled resampler (#1290) against the dense one it replaced.

use pictura_core::memory::eviction::{self, run_pass};
use pictura_core::{Plane, PsdRect, Residency, Sample, TileSample};

use super::kernel::Interpolation;
use super::paths::resolve_path;
use super::transform::{
    integer_bbox, resample_plane, solve_homography, transform_layer_with, LayerTransform, PlaneMap,
    QuadMap,
};
use super::transform_oracle::{dense_resample_plane, dense_resample_typed};
use super::transform_tiled::{resample, Source};

const INTERPOLATIONS: [Interpolation; 6] = [
    Interpolation::Nearest,
    Interpolation::Bilinear,
    Interpolation::Bicubic,
    Interpolation::BicubicSmoother,
    Interpolation::BicubicSharper,
    Interpolation::BicubicAutomatic,
];

fn rect(w: i32, h: i32) -> PsdRect {
    PsdRect {
        top: 7,
        left: -3,
        bottom: 7 + h,
        right: -3 + w,
    }
}

fn corners(r: PsdRect) -> [(f64, f64); 4] {
    let (l, t, rr, b) = (r.left as f64, r.top as f64, r.right as f64, r.bottom as f64);
    [(l, t), (rr, t), (rr, b), (l, b)]
}

/// `r`'s corners scaled by `s` and turned by `degrees` about its centre.
fn similar(r: PsdRect, s: f64, degrees: f64) -> [(f64, f64); 4] {
    let (cx, cy) = (
        f64::from(r.left + r.right) / 2.0,
        f64::from(r.top + r.bottom) / 2.0,
    );
    let (sin, cos) = degrees.to_radians().sin_cos();
    corners(r).map(|(x, y)| {
        let (ux, uy) = ((x - cx) * s, (y - cy) * s);
        (cx + cos * ux - sin * uy, cy + sin * ux + cos * uy)
    })
}

/// The maps the tests drive: a downscale, a turned upscale, an extreme
/// downscale, and a perspective quad.
fn maps(r: PsdRect) -> Vec<(&'static str, QuadMap)> {
    let (l, t, rr, b) = (r.left as f64, r.top as f64, r.right as f64, r.bottom as f64);
    let quads = [
        ("half", similar(r, 0.5, 0.0)),
        ("turned", similar(r, 1.5, 17.0)),
        ("tiny", similar(r, 0.02, 0.0)),
        (
            "perspective",
            [
                (l + 40.0, t),
                (rr - 40.0, t + 25.0),
                (rr + 30.0, b),
                (l - 10.0, b - 5.0),
            ],
        ),
    ];
    quads
        .into_iter()
        .map(|(name, quad)| (name, solve_homography(corners(r), quad).expect("map")))
        .collect()
}

fn dest_of<M: PlaneMap>(map: &M, r: PsdRect) -> (i32, i32, i32, i32) {
    let points = corners(r).map(|(x, y)| map.forward(x, y).expect("finite"));
    integer_bbox(&points).expect("box")
}

fn seeded_u8(w: usize, h: usize) -> Plane<u8> {
    let data: Vec<u8> = (0..w * h).map(|i| (i * 37 % 251) as u8).collect();
    let mut plane = Plane::from(data);
    assert!(plane.tile(w, h));
    plane
}

fn every<T: TileSample>(plane: &Plane<T>) -> Vec<T> {
    let mut out = vec![T::default(); plane.len()];
    plane.read_into(0, &mut out);
    out
}

#[test]
fn tiled_resample_matches_the_dense_resampler() {
    let (w, h) = (600, 450);
    let r = rect(w as i32, h as i32);
    let src = seeded_u8(w, h);
    for (name, map) in maps(r) {
        let dest = dest_of(&map, r);
        for interpolation in INTERPOLATIONS {
            let tiled = resample_plane(&src, w, h, &map, r, dest, interpolation).expect("tiled");
            let dense =
                dense_resample_plane(&src, w, h, &map, r, dest, interpolation).expect("dense");
            assert!(tiled.is_tiled(), "{name} {interpolation:?}");
            assert!(every(&tiled) == dense, "{name} {interpolation:?}");
        }
    }
    // The layer op stores what the resampler builds: tiled channels.
    let rgba: Vec<u8> = (0..300 * 200 * 4).map(|i| (i * 31 % 253) as u8).collect();
    let mut doc = pictura_core::Document::from_rgba("t", 300, 200, &rgba);
    let turn = LayerTransform {
        scale_x: 1.5,
        scale_y: 1.5,
        angle_radians: 17f64.to_radians(),
        dx: 0.0,
        dy: 0.0,
    };
    assert!(transform_layer_with(
        &mut doc,
        "0",
        turn,
        Interpolation::Bicubic
    ));
    let layer = resolve_path(&doc, "0").expect("layer");
    assert!(layer.channels.iter().all(|c| c.data.is_tiled()));
}

fn deep_case<T: Sample + TileSample + std::fmt::Debug>(seed: impl Fn(usize) -> T) {
    let (w, h) = (300, 260);
    let r = rect(w as i32, h as i32);
    let data: Vec<T> = (0..w * h).map(seed).collect();
    let mut plane = Plane::from(data.clone());
    assert!(plane.tile(w, h));
    let source = Source {
        plane: &plane,
        w,
        h,
        pad: 0,
        rect: r,
    };
    for (name, map) in maps(r) {
        let dest = dest_of(&map, r);
        for interpolation in INTERPOLATIONS {
            let tiled = resample(&source, &map, dest, interpolation).expect("tiled");
            let dense =
                dense_resample_typed(&data, w, h, &map, r, dest, interpolation).expect("dense");
            assert!(tiled.is_tiled(), "{name} {interpolation:?}");
            let bits = |v: &[T]| T::bytes(v).to_vec();
            assert_eq!(
                bits(&every(&tiled)),
                bits(&dense),
                "{name} {interpolation:?}"
            );
        }
    }
}

#[test]
fn deep_planes_resample_through_tiles() {
    deep_case(|i| (i * 2731 % 65521) as u16);
    deep_case(|i| (i * 37 % 1013) as f32 / 1013.0);
}

#[test]
fn a_solid_layer_scales_into_uniform_tiles() {
    let side = 2048u32;
    let rgba = [90u8, 160, 30, 255].repeat((side * side) as usize);
    let mut doc = pictura_core::Document::from_rgba("solid", side, side, &rgba);
    let grow = LayerTransform {
        scale_x: 1.5,
        scale_y: 1.5,
        angle_radians: 0.0,
        dx: 0.0,
        dy: 0.0,
    };
    assert!(transform_layer_with(
        &mut doc,
        "0",
        grow,
        Interpolation::Bilinear
    ));
    let layer = resolve_path(&doc, "0").expect("layer");
    let dense = (layer.rect.width() * layer.rect.height()) as usize;
    for c in &layer.channels {
        // Only the edge tiles, where the transparent padding blends in, hold
        // samples; 44 of the 144 tiles of a 3072² plane.
        assert!(
            c.data.allocated_bytes() * 10 < dense * 4,
            "channel {}: {} of {dense} bytes",
            c.id,
            c.data.allocated_bytes()
        );
    }
}

#[test]
fn transformed_tiles_spill_and_read_back() {
    let _scratch = crate::test_support::scratch_guard();
    let dir = pictura_core::scratch::default_volume()
        .with_file_name("test-scratch")
        .join(format!("render-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    pictura_core::scratch::set_volumes(std::slice::from_ref(&dir));
    let (w, h) = (1024, 1024);
    let r = rect(w as i32, h as i32);
    let src = seeded_u8(w, h);
    let map = solve_homography(corners(r), similar(r, 1.0, 10.0)).expect("map");
    let dest = dest_of(&map, r);
    let tiled = resample_plane(&src, w, h, &map, r, dest, Interpolation::Bilinear).expect("tiled");
    for _ in 0..6 {
        run_pass(u64::MAX);
    }
    let spilled = (0..tiled.tile_count())
        .filter(|&i| tiled.tile_residency(i) == Some(Residency::Spilled))
        .count();
    assert!(spilled > 0, "no tile spilled: {:?}", eviction::counts());
    let dense =
        dense_resample_plane(&src, w, h, &map, r, dest, Interpolation::Bilinear).expect("dense");
    assert!(
        every(&tiled) == dense,
        "spilled tiles read back differently"
    );
    let _ = std::fs::remove_dir_all(dir);
}
