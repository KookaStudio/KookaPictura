//! Selection-border transform tests: the mask resampler and its warp.

use pictura_core::warp::{BezierMesh, Warp, WarpStyle};
use pictura_core::{Plane, PsdRect};
use pictura_select::Selection;

use super::kernel::Interpolation;
use super::selection_transform::{
    selection_transform_point, transform_selection, warp_dest_rect, warp_selection,
};

fn solid(width: u32, height: u32, x: i32, y: i32, w: i32, h: i32) -> Selection {
    Selection::rect(width, height, x, y, w, h)
}

fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PsdRect {
    PsdRect {
        left,
        top,
        right,
        bottom,
    }
}

fn corners(rect: PsdRect) -> [(f64, f64); 4] {
    [
        (f64::from(rect.left), f64::from(rect.top)),
        (f64::from(rect.right), f64::from(rect.top)),
        (f64::from(rect.right), f64::from(rect.bottom)),
        (f64::from(rect.left), f64::from(rect.bottom)),
    ]
}

fn bounds(selection: &Selection) -> Option<(i32, i32, i32, i32)> {
    let (w, h) = (selection.width as i32, selection.height as i32);
    let mut out: Option<(i32, i32, i32, i32)> = None;
    for y in 0..h {
        for x in 0..w {
            if selection.data.sample((y * w + x) as usize).unwrap_or(0) > 0 {
                out = Some(match out {
                    Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x + 1), b.max(y + 1)),
                    None => (x, y, x + 1, y + 1),
                });
            }
        }
    }
    out
}

fn warp_bounds(rect: PsdRect) -> [f64; 4] {
    [
        f64::from(rect.left),
        f64::from(rect.top),
        f64::from(rect.right),
        f64::from(rect.bottom),
    ]
}

#[test]
fn transform_selection_identity() {
    let selection = solid(10, 10, 2, 2, 4, 4);
    let boxed = rect(2, 2, 6, 6);
    let out = transform_selection(&selection, boxed, corners(boxed), Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid map");
    assert_eq!(out, selection);
}

#[test]
fn transform_selection_scale_doubles() {
    let selection = solid(16, 16, 4, 4, 4, 4);
    let boxed = rect(4, 4, 8, 8);
    // 2x about the box centre (6, 6).
    let quad = [(2.0, 2.0), (10.0, 2.0), (10.0, 10.0), (2.0, 10.0)];
    let out = transform_selection(&selection, boxed, quad, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid map");
    let (left, top, right, bottom) = bounds(&out).expect("coverage");
    assert!((right - left - 8).abs() <= 1, "{left}..{right}");
    assert!((bottom - top - 8).abs() <= 1, "{top}..{bottom}");
}

#[test]
fn transform_selection_rotation_about_pivot() {
    let selection = solid(20, 20, 8, 9, 4, 2);
    let boxed = rect(8, 9, 12, 11);
    // 90° clockwise about the box centre (10, 10): (x, y) -> (20 - y, x).
    let quad = [(11.0, 8.0), (11.0, 12.0), (9.0, 12.0), (9.0, 8.0)];
    let out = transform_selection(&selection, boxed, quad, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid map");
    let (left, top, right, bottom) = bounds(&out).expect("coverage");
    assert!((right - left - 2).abs() <= 1, "width {}", right - left);
    assert!((bottom - top - 4).abs() <= 1, "height {}", bottom - top);
}

#[test]
fn transform_selection_clips_to_canvas() {
    let selection = solid(12, 12, 2, 2, 4, 4);
    let boxed = rect(2, 2, 6, 6);
    let quad = [
        (100.0, 100.0),
        (104.0, 100.0),
        (104.0, 104.0),
        (100.0, 104.0),
    ];
    let out = transform_selection(&selection, boxed, quad, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid map");
    assert_eq!(bounds(&out), None, "a fully off-canvas border is empty");
    assert_eq!(out.width, 12);
    assert_eq!(out.height, 12);
}

#[test]
fn transform_selection_keeps_soft_coverage() {
    let data: Vec<u8> = (0..16 * 16)
        .map(|i| ((i % 16) * 17).min(255) as u8)
        .collect();
    let selection = Selection {
        width: 16,
        height: 16,
        data: Plane::from(data),
    };
    let boxed = rect(0, 0, 16, 16);
    let quad = [(1.0, 1.0), (15.0, 1.0), (15.0, 15.0), (1.0, 15.0)];
    let out = transform_selection(&selection, boxed, quad, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid map");
    let soft = out.data.iter().filter(|&&v| v > 0 && v < 255).count();
    assert!(soft > 0, "the ramp stays soft");
}

#[test]
fn transform_selection_keeps_the_source_unchanged() {
    let selection = solid(10, 10, 2, 2, 4, 4);
    let before = selection.clone();
    let boxed = rect(2, 2, 6, 6);
    let quad = [(1.0, 1.0), (7.0, 1.0), (7.0, 7.0), (1.0, 7.0)];
    let _ = transform_selection(&selection, boxed, quad, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid map");
    assert_eq!(selection, before);
}

#[test]
fn transform_selection_matches_the_dense_oracle() {
    let selection = solid(32, 32, 4, 6, 12, 10);
    let boxed = rect(4, 6, 16, 16);
    let quad = [(5.0, 5.0), (19.0, 7.0), (17.0, 22.0), (3.0, 19.0)];
    let out = transform_selection(&selection, boxed, quad, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid map");

    let map =
        super::transform::solve_homography(corners(boxed), quad).expect("the test quad is valid");
    let dest = super::transform::integer_bbox(&quad).expect("a bbox");
    let canvas = rect(0, 0, 32, 32);
    let dense = super::transform_oracle::dense_resample_plane(
        &selection.data,
        None,
        32,
        32,
        &map,
        canvas,
        dest,
        Interpolation::Bilinear,
    )
    .expect("the dense oracle fits");
    let (dl, dt, dr, _db) = dest;
    let dw = (dr - dl) as usize;
    for y in dt..dest.3 {
        for x in dl..dr {
            let got = out.data.sample((y * 32 + x) as usize).unwrap_or(0);
            let want = dense[(y - dt) as usize * dw + (x - dl) as usize];
            assert_eq!(got, want, "pixel ({x}, {y})");
        }
    }
}

#[test]
fn warp_selection_identity() {
    let selection = solid(12, 12, 3, 3, 5, 5);
    let boxed = rect(3, 3, 8, 8);
    let warp = Warp::none(warp_bounds(boxed));
    let out = warp_selection(&selection, boxed, &warp, None, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid warp");
    assert_eq!(out, selection);
}

#[test]
fn warp_selection_moves_coverage() {
    let selection = solid(32, 32, 4, 4, 24, 24);
    let boxed = rect(4, 4, 28, 28);
    let warp = Warp::preset(WarpStyle::Arc, 50.0, warp_bounds(boxed));
    let out = warp_selection(&selection, boxed, &warp, None, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid warp");
    assert!(bounds(&out).is_some(), "the arc keeps the border");
    assert_ne!(out, selection, "the arc changes the coverage");
}

#[test]
fn warp_selection_custom_mesh_moves_coverage() {
    let selection = solid(32, 32, 6, 6, 20, 20);
    let boxed = rect(6, 6, 26, 26);
    let mut mesh = BezierMesh::identity(warp_bounds(boxed), 1, 1);
    mesh.points[5][0] += 6.0;
    mesh.points[5][1] -= 3.0;
    let warp = Warp::custom(mesh, warp_bounds(boxed));
    let out = warp_selection(&selection, boxed, &warp, None, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid warp");
    assert!(bounds(&out).is_some());
    assert_ne!(out, selection);
}

#[test]
fn warp_selection_empty_stays_empty() {
    let selection = Selection::none(12, 12);
    let boxed = rect(2, 2, 8, 8);
    let warp = Warp::preset(WarpStyle::Arc, 50.0, warp_bounds(boxed));
    let out = warp_selection(&selection, boxed, &warp, None, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid warp");
    assert_eq!(bounds(&out), None);
}

#[test]
fn warp_selection_clips_to_canvas() {
    let selection = solid(12, 12, 2, 2, 4, 4);
    let boxed = rect(2, 2, 6, 6);
    let warp = Warp::none(warp_bounds(boxed));
    let quad = [
        (100.0, 100.0),
        (104.0, 100.0),
        (104.0, 104.0),
        (100.0, 104.0),
    ];
    let out = warp_selection(
        &selection,
        boxed,
        &warp,
        Some(quad),
        Interpolation::Bilinear,
    )
    .expect("no memory refusal")
    .expect("a valid warp");
    assert_eq!(bounds(&out), None);
}

#[test]
fn warp_dest_rect_matches_the_mask() {
    let selection = solid(48, 48, 8, 8, 28, 28);
    let boxed = rect(8, 8, 36, 36);
    let warp = Warp::preset(WarpStyle::Arc, 40.0, warp_bounds(boxed));
    let dest = warp_dest_rect(boxed, &warp, None).expect("a valid warp");
    // The mask is clipped to the canvas; the session box may extend past it.
    let dest = PsdRect {
        left: dest.left.max(0),
        top: dest.top.max(0),
        right: dest.right.min(48),
        bottom: dest.bottom.min(48),
    };
    let out = warp_selection(&selection, boxed, &warp, None, Interpolation::Bilinear)
        .expect("no memory refusal")
        .expect("a valid warp");
    let (left, top, right, bottom) = bounds(&out).expect("coverage");
    assert!(
        (left - dest.left).abs() <= 2,
        "left {left} vs {}",
        dest.left
    );
    assert!((top - dest.top).abs() <= 2, "top {top} vs {}", dest.top);
    assert!(
        (right - dest.right).abs() <= 2,
        "right {right} vs {}",
        dest.right
    );
    assert!(
        (bottom - dest.bottom).abs() <= 2,
        "bottom {bottom} vs {}",
        dest.bottom
    );
}

#[test]
fn selection_transform_point_uses_the_crossed_quad_map() {
    let boxed = rect(0, 0, 10, 10);
    // A bowtie: the side edges cross at (5, 5), where a homography would run
    // through the horizon but the hourglass map stays finite.
    let crossed = [(10.0, 0.0), (0.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    let point = selection_transform_point(boxed, crossed, 4.0, 4.0)
        .expect("the crossed map keeps an interior point finite");
    assert!(point.0.is_finite() && point.1.is_finite());
    let degenerate = [(0.0, 0.0), (0.0, 0.0), (10.0, 10.0), (10.0, 10.0)];
    assert!(selection_transform_point(boxed, degenerate, 4.0, 4.0).is_none());
}
