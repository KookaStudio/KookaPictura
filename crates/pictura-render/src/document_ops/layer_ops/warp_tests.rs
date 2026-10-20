//! Edit ▸ Transform ▸ Warp's layer op and its resampler (`warp.rs`,
//! `warp_raster.rs`); the resampler cases are ported from photocraft's
//! `algo::warp` tests.

use super::kernel::Interpolation;
use super::paths::resolve_path;
use super::transform::solve_homography;
use super::transform::PlaneMap;
use super::transform_layer_warp;
use super::warp_raster::{triangulate, warp_planes, Planes};
use pictura_core::warp::{BezierMesh, Warp, WarpStyle};
use pictura_core::{
    BitDepth, Channel, ColorMode, Document, Layer, LayerMask, PsdRect, Samples, SourceChannels,
    SourcePlanes,
};

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        top,
        left,
        bottom,
        right,
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

fn plane(w: i32, h: i32, seed: u8) -> Vec<u8> {
    (0..(w * h) as usize)
        .map(|i| (i as u8).wrapping_mul(7).wrapping_add(seed))
        .collect()
}

fn pixel_layer(r: PsdRect) -> Layer {
    let (w, h) = (r.width(), r.height());
    Layer {
        name: "L".into(),
        rect: r,
        channels: vec![
            Channel {
                id: -1,
                data: vec![255; (w * h) as usize].into(),
            },
            Channel {
                id: 0,
                data: plane(w, h, 3).into(),
            },
            Channel {
                id: 1,
                data: plane(w, h, 11).into(),
            },
            Channel {
                id: 2,
                data: plane(w, h, 29).into(),
            },
        ],
        ..Default::default()
    }
}

fn doc_with(layer: Layer) -> Document {
    let mut doc = Document::new(64, 64, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

fn channel(layer: &Layer, id: i16) -> &[u8] {
    &layer.channels.iter().find(|c| c.id == id).unwrap().data
}

/// The custom warp that moves the whole box by `(dx, dy)`.
fn shifted(b: [f64; 4], dx: f64, dy: f64) -> Warp {
    Warp::custom(
        BezierMesh::identity(b, 1, 1).map_points(|p| [p[0] + dx, p[1] + dy]),
        b,
    )
}

#[test]
fn identity_warps_are_refused_and_change_nothing() {
    let r = rect(5, 3, 25, 19);
    let b = bounds(r);
    for warp in [
        Warp::none(b),
        Warp::custom(BezierMesh::identity(b, 1, 1), b),
        Warp::preset(WarpStyle::Arc, 0.0, b),
    ] {
        let mut doc = doc_with(pixel_layer(r));
        let before = doc.clone();
        assert!(!transform_layer_warp(
            &mut doc,
            "0",
            &warp,
            None,
            Interpolation::Bicubic
        ));
        assert_eq!(doc, before);
    }
}

#[test]
fn a_translating_mesh_moves_the_pixels_exactly() {
    let r = rect(5, 3, 25, 19);
    for interp in [
        Interpolation::Nearest,
        Interpolation::Bilinear,
        Interpolation::Bicubic,
    ] {
        let mut doc = doc_with(pixel_layer(r));
        assert!(transform_layer_warp(
            &mut doc,
            "0",
            &shifted(bounds(r), 10.0, -2.0),
            None,
            interp
        ));
        let before = pixel_layer(r);
        let layer = resolve_path(&doc, "0").unwrap();
        assert_eq!(layer.rect, rect(15, 1, 35, 17), "{interp:?}");
        for id in [-1, 0, 1, 2] {
            assert_eq!(channel(layer, id), channel(&before, id), "{interp:?} {id}");
        }
    }
}

#[test]
fn arc_keeps_the_bottom_corners_and_spreads_the_top() {
    let r = rect(10, 20, 50, 44);
    let warp = Warp::preset(WarpStyle::Arc, 50.0, bounds(r));
    let corner = warp.map(10.0, 44.0);
    assert!((corner.0 - 10.0).abs() < 1e-9 && (corner.1 - 44.0).abs() < 1e-9);
    let mut doc = doc_with(pixel_layer(r));
    assert!(transform_layer_warp(
        &mut doc,
        "0",
        &warp,
        None,
        Interpolation::Bilinear
    ));
    let out = resolve_path(&doc, "0").unwrap().rect;
    assert!(
        out.top < r.top - 2 && out.left < r.left && out.right > r.right,
        "the top rises and spreads past the box: {out:?}"
    );
}

#[test]
fn a_thin_layer_stretched_into_a_curve_keeps_the_curve() {
    // A 3 px wide column stretched 100x and bent into an arch: a grid sized in
    // source pixels (one cell across) comes out as a straight band.
    let r = rect(0, 0, 3, 40);
    let src = Planes::new(vec![vec![1.0; 120], vec![1.0; 120]], 3, 40, Some(1));
    let arch = |x: f64, y: f64| {
        let dx = x * 100.0;
        let t = (dx - 150.0) / 150.0;
        (dx, y + 40.0 * (1.0 - t * t))
    };
    let t = triangulate(r, &arch).unwrap();
    let out = warp_planes(&src, r, &t, Interpolation::Bilinear, false);
    let (dl, dt, dr, _) = t.dest;
    let alpha = |x: i32, y: i32| out[1][((y - dt) * (dr - dl) + (x - dl)) as usize];
    // The band follows the arch: 40 px lower in the middle, about 36 px a
    // third of the way in.
    assert!(alpha(150, 60) > 0.99 && alpha(150, 20) < 0.01, "middle");
    assert!(alpha(100, 55) > 0.99 && alpha(100, 20) < 0.01, "a third in");
}

#[test]
fn a_shrinking_warp_is_prefiltered() {
    // A grid of 1 px lines every 4 px warped to ~1/5 size must come out its area
    // average (9/16 white), not point-sampled moiré.
    let (w, h) = (400, 320);
    let r = rect(0, 0, w, h);
    let grid: Vec<f32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| if x % 4 == 0 || y % 4 == 0 { 0.0 } else { 1.0 }))
        .collect();
    let n = grid.len();
    let src = Planes::new(vec![grid, vec![1.0; n]], w as usize, h as usize, Some(1));
    let warp = Warp::preset(WarpStyle::Arc, 15.0, bounds(r));
    let map = |x: f64, y: f64| {
        let (u, v) = warp.map(x, y);
        (0.2 * u + 7.0, 0.2 * v + 5.0)
    };
    let t = triangulate(r, &map).unwrap();
    let out = warp_planes(&src, r, &t, Interpolation::Bicubic, false);
    let opaque: Vec<f32> = out[0]
        .iter()
        .zip(&out[1])
        .filter(|(_, a)| **a >= 0.999)
        .map(|(v, _)| *v)
        .collect();
    assert!(opaque.len() > 2000, "{} opaque pixels", opaque.len());
    let worst = opaque
        .iter()
        .map(|v| (v - 0.5625).abs())
        .fold(0.0f32, f32::max);
    assert!(
        worst < 0.25,
        "worst deviation from the area average {worst}"
    );
}

#[test]
fn a_fold_draws_the_later_cell_on_top() {
    // The left half is mirrored onto the strip the right half covers, so the
    // two cells overlap everywhere: the sampled pixel reads about 1/8 from the
    // earlier (left) cell and 7/8 from the later (right) one.
    let r = rect(0, 0, 8, 4);
    let values: Vec<f32> = (0..32).map(|i| (i % 8) as f32 / 8.0).collect();
    let src = Planes::new(vec![values, vec![1.0; 32]], 8, 4, Some(1));
    let fold = |x: f64, y: f64| (2.0 + (x - 4.0).abs() / 2.0, y);
    let t = triangulate(r, &fold).unwrap();
    let out = warp_planes(&src, r, &t, Interpolation::Nearest, false);
    let (dl, dt, dr, _) = t.dest;
    let v = out[0][((1 - dt) * (dr - dl) + (3 - dl)) as usize];
    assert!(v >= 6.0 / 8.0, "the right-most source wins: {v}");
}

#[test]
fn the_layer_mask_moves_and_keeps_its_default() {
    let r = rect(4, 4, 14, 14);
    let mut layer = pixel_layer(r);
    layer.mask = Some(LayerMask {
        rect: r,
        default_color: 255,
        data: Some(vec![0u8; 100].into()),
        ..Default::default()
    });
    let mut doc = doc_with(layer);
    assert!(transform_layer_warp(
        &mut doc,
        "0",
        &shifted(bounds(r), 5.0, 0.0),
        None,
        Interpolation::Bilinear
    ));
    let mask = resolve_path(&doc, "0").unwrap().mask.clone().unwrap();
    assert_eq!(mask.rect, rect(9, 4, 19, 14));
    assert_eq!(mask.default_color, 255);
    assert!(mask.data.unwrap().iter().all(|v| *v == 0));
}

#[test]
fn depth16_warp_resamples_and_keeps_the_native_store() {
    let r = rect(0, 0, 8, 8);
    let mut layer = pixel_layer(r);
    let native = |seed: u8| {
        Samples::U16(
            plane(8, 8, seed)
                .into_iter()
                .map(|v| (v as u16 * 257).saturating_add(7))
                .collect(),
        )
    };
    layer.source_channels = Some(SourceChannels::new(
        BitDepth::Sixteen,
        r,
        vec![
            (-1, Samples::U16(vec![65535; 64])),
            (0, native(3)),
            (1, native(11)),
            (2, native(29)),
        ],
    ));
    let mut doc = doc_with(layer);
    doc.source_depth = Some(BitDepth::Sixteen);
    doc.source_planes = Some(SourcePlanes {
        depth: BitDepth::Sixteen,
        width: 64,
        height: 64,
        samples: Samples::U16(vec![0; 3 * 64 * 64]),
    });
    let b = bounds(r);
    let mut mesh = BezierMesh::identity(b, 1, 1);
    mesh.points[5] = [1.0, 1.0];
    assert!(transform_layer_warp(
        &mut doc,
        "0",
        &Warp::custom(mesh, b),
        None,
        Interpolation::Bicubic
    ));
    let layer = resolve_path(&doc, "0").unwrap();
    let store = layer
        .source_channels
        .as_ref()
        .expect("a warp keeps a resampled native store");
    assert_eq!(store.rect, layer.rect);
    let area = (layer.rect.width() * layer.rect.height()) as usize;
    assert!(store.planes.iter().all(|(_, s)| s.len() == area));
    let bytes = pictura_codec::write_psd(&doc).expect("a warped high-depth layer saves");
    assert_eq!(u16::from_be_bytes(bytes[22..24].try_into().unwrap()), 16);
}

#[test]
fn a_projective_box_is_kept_exactly() {
    // Entering Warp after Perspective keeps the box's homography after the
    // (identity) mesh: interior pixels sample where the inverse homography
    // says. A Bezier patch through the projected control points would put the
    // source centre 1.6 px left of where the homography does.
    let r = rect(0, 0, 40, 40);
    let mut layer = pixel_layer(r);
    let gradient: Vec<u8> = (0..1600).map(|i| (i % 40) as u8 * 6).collect();
    layer.channels.iter_mut().find(|c| c.id == 0).unwrap().data = gradient.into();
    let quad = [(0.0, 0.0), (40.0, 8.0), (40.0, 32.0), (0.0, 40.0)];
    let b = bounds(r);
    let mut doc = doc_with(layer);
    assert!(transform_layer_warp(
        &mut doc,
        "0",
        &Warp::custom(BezierMesh::identity(b, 1, 1), b),
        Some(quad),
        Interpolation::Bilinear
    ));
    let out = resolve_path(&doc, "0").unwrap();
    let corners = [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)];
    let inverse = solve_homography(quad, corners).unwrap();
    for x in [10, 18, 25, 33] {
        let (u, _) = inverse.forward(f64::from(x) + 0.5, 20.5).unwrap();
        let i = ((20 - out.rect.top) * out.rect.width() + (x - out.rect.left)) as usize;
        let got = f64::from(channel(out, 0)[i]);
        assert!(
            (got - 6.0 * (u - 0.5)).abs() <= 3.0,
            "column {x}: {got} for source x {u}"
        );
    }
}

#[test]
fn a_mask_wider_than_the_layer_moves_with_it() {
    // The mask reaches 6 px right of the layer: the outside part follows the
    // translation instead of collapsing onto the box edge.
    let r = rect(4, 4, 14, 14);
    let mut layer = pixel_layer(r);
    layer.mask = Some(LayerMask {
        rect: rect(4, 4, 20, 14),
        default_color: 255,
        data: Some(vec![0u8; 160].into()),
        ..Default::default()
    });
    let mut doc = doc_with(layer);
    assert!(transform_layer_warp(
        &mut doc,
        "0",
        &shifted(bounds(r), 5.0, 0.0),
        None,
        Interpolation::Bilinear
    ));
    let mask = resolve_path(&doc, "0").unwrap().mask.clone().unwrap();
    assert_eq!(mask.rect, rect(9, 4, 25, 14));
    assert!(mask.data.unwrap().iter().all(|v| *v == 0));
}

#[test]
fn a_mask_that_cannot_follow_refuses_the_warp() {
    // A one-row mask reaching almost to the coordinate limit: the pixels can
    // move 1000 px right, the mask cannot. The op refuses rather than move the
    // pixels and leave the mask behind.
    let r = rect(4, 4, 14, 14);
    let right = 1_048_000;
    let mut layer = pixel_layer(r);
    layer.mask = Some(LayerMask {
        rect: rect(4, 4, right, 5),
        default_color: 255,
        data: Some(vec![0u8; (right - 4) as usize].into()),
        ..Default::default()
    });
    let mut doc = doc_with(layer);
    let before = doc.clone();
    assert!(!transform_layer_warp(
        &mut doc,
        "0",
        &shifted(bounds(r), 1000.0, 0.0),
        None,
        Interpolation::Bilinear
    ));
    assert_eq!(doc, before);
}

#[test]
fn a_warp_past_the_result_area_ceiling_is_refused() {
    // Inside the coordinate limit but billions of pixels in area: refused
    // before any plane is allocated.
    let r = rect(0, 0, 4, 4);
    let b = bounds(r);
    let huge = Warp::custom(
        BezierMesh::identity(b, 1, 1).map_points(|p| [p[0] * 16_384.0, p[1] * 16_384.0]),
        b,
    );
    assert!(triangulate(r, &|x, y| huge.map(x, y)).is_none());
    let mut doc = doc_with(pixel_layer(r));
    let before = doc.clone();
    assert!(!transform_layer_warp(
        &mut doc,
        "0",
        &huge,
        None,
        Interpolation::Bilinear
    ));
    assert_eq!(doc, before);
}
