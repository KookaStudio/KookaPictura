use std::sync::Arc;

use pictura_core::PixelBuffer;

use super::*;

/// A `w`×`h` gradient (red across, green down), opaque when `planes` is 4.
fn layer(w: u32, h: u32, planes: u8) -> PixelBuffer {
    let mut buf = PixelBuffer::new(w, h, planes);
    let n = buf.pixel_count();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * w as usize + x;
            buf.data[i] = (x * 255 / w as usize) as u8;
            buf.data[n + i] = (y * 255 / h as usize) as u8;
            buf.data[2 * n + i] = 128;
            if planes == 4 {
                buf.data[3 * n + i] = 255;
            }
        }
    }
    buf
}

fn params(correction: AwaCorrection, focal: f64, crop: f64, scale: f64) -> AwaParams {
    AwaParams {
        correction,
        scale,
        focal_length: focal,
        crop_factor: crop,
        offset: [0.0; 2],
        constraints: Arc::default(),
    }
}

/// A fisheye whose focal length is 120 px on a 300×200 layer.
fn fisheye() -> AwaParams {
    params(
        AwaCorrection::Fisheye,
        120.0 / 300f64.hypot(200.0) * 43.27,
        1.0,
        1.0,
    )
}

/// A constraint through layer points (pixels of a 300×200 layer).
fn constraint(points: &[[f64; 2]], polygon: bool, orientation: AwaOrientation) -> AwaConstraint {
    AwaConstraint {
        points: points
            .iter()
            .map(|p| [p[0] / 300.0, p[1] / 200.0])
            .collect(),
        polygon,
        orientation,
    }
}

fn with(mut p: AwaParams, constraints: Vec<AwaConstraint>) -> AwaParams {
    p.constraints = Arc::new(constraints);
    p
}

/// The farthest a path strays from the chord between its ends.
fn deviation(path: &[[f64; 2]]) -> f64 {
    let (a, b) = (path[0], *path.last().unwrap());
    let len = (b[0] - a[0]).hypot(b[1] - a[1]);
    path.iter()
        .map(|p| edge(a, b, *p).abs() / len)
        .fold(0.0, f64::max)
}

fn alpha_at(buf: &PixelBuffer, x: usize, y: usize) -> u8 {
    buf.data[3 * buf.pixel_count() + y * buf.width as usize + x]
}

#[test]
fn short_fisheye_leaves_a_disc() {
    let mut buf = layer(300, 200, 4);
    let p = params(AwaCorrection::Fisheye, 11.74, 0.20, 0.5);
    let f = 11.74 * 0.20 / 43.27 * 300f64.hypot(200.0);
    // The CS6 capture this reproduces: a disc of about 0.054 diagonals.
    assert!((f / 300f64.hypot(200.0) - 0.0543).abs() < 0.001);
    adaptive_wide_angle(&mut buf, &p).unwrap();
    for y in 0..200 {
        for x in 0..300 {
            let d = (x as f64 + 0.5 - 150.0).hypot(y as f64 + 0.5 - 100.0);
            let a = alpha_at(&buf, x, y);
            if d < f - 2.0 {
                assert_eq!(a, 255, "inside the disc at ({x}, {y})");
            } else if d > f + 2.0 {
                assert_eq!(a, 0, "outside the disc at ({x}, {y})");
            }
        }
    }
}

#[test]
fn perspective_pulls_corners_in() {
    let source = layer(300, 200, 4);
    let mut buf = source.clone();
    let focal = 100.0 / 300f64.hypot(200.0) * 43.27;
    adaptive_wide_angle(
        &mut buf,
        &params(AwaCorrection::Perspective, focal, 1.0, 1.5),
    )
    .unwrap();
    for (x, y) in [(0, 0), (299, 0), (0, 199), (299, 199)] {
        assert_eq!(alpha_at(&buf, x, y), 0, "corner ({x}, {y}) stays covered");
    }
    for (x, y) in [(0, 100), (299, 100), (150, 0), (150, 199)] {
        assert_eq!(
            alpha_at(&buf, x, y),
            255,
            "edge midpoint ({x}, {y}) is clear"
        );
    }
    let centre = 100 * 300 + 150;
    for c in 0..3 {
        let n = buf.pixel_count();
        let (was, now) = (source.data[c * n + centre], buf.data[c * n + centre]);
        assert!(
            was.abs_diff(now) <= 2,
            "plane {c} at the centre: {was} → {now}"
        );
    }
}

#[test]
fn full_spherical_is_identity_without_constraints() {
    let source = layer(64, 32, 4);
    let mut buf = source.clone();
    adaptive_wide_angle(
        &mut buf,
        &params(AwaCorrection::FullSpherical, 10.0, 1.0, 1.0),
    )
    .unwrap();
    assert_eq!(&buf.data[..], &source.data[..]);
}

#[test]
fn scale_shrinks_about_the_centre() {
    let mut buf = layer(64, 32, 4);
    adaptive_wide_angle(
        &mut buf,
        &params(AwaCorrection::FullSpherical, 10.0, 1.0, 0.5),
    )
    .unwrap();
    let opaque = (0..32)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .filter(|&(x, y)| alpha_at(&buf, x, y) > 0)
        .count();
    assert!(
        (448..=576).contains(&opaque),
        "{opaque} of 2048 stay opaque"
    );
    assert_eq!(alpha_at(&buf, 32, 16), 255);
    assert_eq!(alpha_at(&buf, 2, 2), 0);
}

#[test]
fn blank_areas_are_white_without_alpha() {
    let mut buf = layer(64, 32, 3);
    adaptive_wide_angle(
        &mut buf,
        &params(AwaCorrection::FullSpherical, 10.0, 1.0, 0.5),
    )
    .unwrap();
    let n = buf.pixel_count();
    let at = 64 + 1;
    assert!((0..3).all(|c| buf.data[c * n + at] == 255));
    let centre = 16 * 64 + 32;
    assert!((0..3).any(|c| buf.data[c * n + centre] != 255));
}

#[test]
fn constrained_runs_are_deterministic() {
    let p = with(
        fisheye(),
        vec![constraint(
            &[[40.0, 40.0], [260.0, 40.0]],
            false,
            AwaOrientation::Unfixed,
        )],
    );
    let mut a = layer(300, 200, 4);
    let mut b = a.clone();
    adaptive_wide_angle(&mut a, &p).unwrap();
    adaptive_wide_angle(&mut b, &p).unwrap();
    assert_eq!(&a.data[..], &b.data[..]);
}

#[test]
fn a_line_constraint_straightens_its_trace() {
    let (a, b) = ([40.0, 40.0], [260.0, 40.0]);
    let before = AwaWarp::solve(&fisheye(), 300, 200).arc(a, b, true);
    assert!(deviation(&before) > 8.0, "bows {}", deviation(&before));
    let p = with(
        fisheye(),
        vec![constraint(&[a, b], false, AwaOrientation::Unfixed)],
    );
    let warp = AwaWarp::solve(&p, 300, 200);
    assert_eq!(warp.constraint_state(0), Some(AwaOrientation::Unfixed));
    let after = &warp.constraint_paths(0, true)[0];
    assert!(deviation(after) <= 1.0, "still bows {}", deviation(after));
}

#[test]
fn oriented_lines_level_their_trace() {
    for (points, orientation, axis) in [
        (
            [[60.0, 150.0], [240.0, 175.0]],
            AwaOrientation::Horizontal,
            1,
        ),
        ([[70.0, 30.0], [95.0, 170.0]], AwaOrientation::Vertical, 0),
    ] {
        let p = with(fisheye(), vec![constraint(&points, false, orientation)]);
        let warp = AwaWarp::solve(&p, 300, 200);
        let path = &warp.constraint_paths(0, true)[0];
        let spread = path
            .iter()
            .map(|q| (q[axis] - path[0][axis]).abs())
            .fold(0.0, f64::max);
        assert!(spread <= 1.0, "{orientation:?} spreads {spread}");
    }
}

#[test]
fn a_fixed_line_takes_its_angle() {
    let angle = 30f64.to_radians();
    let p = with(
        params(AwaCorrection::Perspective, 15.31, 10.0, 1.0),
        vec![constraint(
            &[[100.0, 80.0], [200.0, 120.0]],
            false,
            AwaOrientation::Fixed(angle),
        )],
    );
    let warp = AwaWarp::solve(&p, 300, 200);
    let path = &warp.constraint_paths(0, true)[0];
    let (a, b) = (path[0], *path.last().unwrap());
    let got = (b[1] - a[1]).atan2(b[0] - a[0]);
    assert!(
        (got - angle).abs() < 0.5f64.to_radians(),
        "{}°",
        got.to_degrees()
    );
}

#[test]
fn a_polygon_straightens_its_edges() {
    let corners = [[60.0, 40.0], [240.0, 40.0], [240.0, 160.0], [60.0, 160.0]];
    let before = AwaWarp::solve(&fisheye(), 300, 200).arc(corners[0], corners[1], true);
    assert!(deviation(&before) > 4.0);
    let p = with(
        fisheye(),
        vec![constraint(&corners, true, AwaOrientation::Unfixed)],
    );
    let warp = AwaWarp::solve(&p, 300, 200);
    let edges = warp.constraint_paths(0, true);
    assert_eq!(edges.len(), 4);
    for path in &edges {
        assert!(deviation(path) <= 1.0, "an edge bows {}", deviation(path));
    }
}

#[test]
fn an_invalid_constraint_changes_nothing() {
    let p = with(
        fisheye(),
        vec![constraint(
            &[[80.0, 80.0], [80.0, 80.0]],
            false,
            AwaOrientation::Unfixed,
        )],
    );
    assert_eq!(AwaWarp::solve(&p, 300, 200).constraint_state(0), None);
    let mut with_it = layer(300, 200, 4);
    let mut without = with_it.clone();
    adaptive_wide_angle(&mut with_it, &p).unwrap();
    adaptive_wide_angle(&mut without, &fisheye()).unwrap();
    assert_eq!(&with_it.data[..], &without.data[..]);
}

#[test]
fn inverse_undoes_forward() {
    let p = with(
        fisheye(),
        vec![constraint(
            &[[40.0, 40.0], [260.0, 40.0]],
            false,
            AwaOrientation::Unfixed,
        )],
    );
    let warp = AwaWarp::solve(&p, 300, 200);
    for s in [[150.0, 100.0], [70.0, 60.0], [220.0, 150.0]] {
        let back = warp.inverse(warp.forward(s)).unwrap();
        assert!(
            (back[0] - s[0]).hypot(back[1] - s[1]) < 0.5,
            "{s:?} → {back:?}"
        );
    }
}
