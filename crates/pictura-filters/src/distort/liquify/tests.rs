use super::*;
use pictura_core::PixelBuffer;

fn brush(tool: LiquifyTool, size: f64) -> LiquifyBrush {
    LiquifyBrush::new(tool, size)
}

/// A 3-channel buffer whose red plane is `x`, green `y`, blue a checker.
fn gradient(w: u32, h: u32) -> PixelBuffer {
    let mut buf = PixelBuffer::new(w, h, 3);
    let n = (w * h) as usize;
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * w as usize + x;
            buf.data[i] = (x * 255 / (w as usize - 1)) as u8;
            buf.data[n + i] = (y * 255 / (h as usize - 1)) as u8;
            buf.data[2 * n + i] = if (x / 4 + y / 4) % 2 == 0 { 40 } else { 220 };
        }
    }
    buf
}

#[test]
fn an_identity_mesh_leaves_the_layer_bit_identical() {
    let mut buf = gradient(40, 30);
    let before = buf.data.to_vec();
    liquify(&mut buf, &LiquifyMesh::new(40, 30)).unwrap();
    assert_eq!(buf.data.to_vec(), before);
}

#[test]
fn a_uniform_offset_shifts_the_picture_exactly() {
    let mut mesh = LiquifyMesh::new(40, 30);
    mesh.d.fill([3.0, 0.0]);
    let mut buf = gradient(40, 30);
    let before = buf.clone();
    liquify(&mut buf, &mesh).unwrap();
    let n = 40 * 30;
    for c in 0..3 {
        for y in 0..30 {
            for x in 0..37 {
                let i = y * 40 + x;
                assert_eq!(
                    buf.data[c * n + i],
                    before.data[c * n + i + 3],
                    "c{c} ({x},{y})"
                );
            }
        }
    }
}

#[test]
fn forward_warp_moves_pixels_along_the_drag_and_nowhere_else() {
    let mut mesh = LiquifyMesh::new(120, 80);
    let b = brush(LiquifyTool::ForwardWarp, 30.0);
    let dirty = mesh
        .stroke_to(&b, [40.0, 40.0, 1.0], [52.0, 40.0, 1.0])
        .expect("the drag changes the mesh");
    // Pixels moved right, so the output reads from the left.
    let centre = mesh.displacement(52.0, 40.0);
    assert!(centre[0] < -5.0 && centre[1].abs() < 0.5, "{centre:?}");
    // Strongest at the centre, lighter toward the edge.
    let edge = mesh.displacement(52.0, 52.0);
    assert!(edge[0] > centre[0] && edge[0] <= 0.0, "{edge:?}");
    // Nothing beyond the brush footprint, which the dirty rect bounds.
    assert_eq!(mesh.displacement(100.0, 40.0), [0.0, 0.0]);
    assert_eq!(mesh.displacement(52.0, 5.0), [0.0, 0.0]);
    assert!(dirty.left <= 25 && dirty.right >= 64 && dirty.right <= 120);
}

#[test]
fn twirl_turns_further_with_hold_time_and_alt_reverses_it() {
    let turn = |reverse: bool, holds: usize| {
        let mut mesh = LiquifyMesh::new(100, 100);
        let mut b = brush(LiquifyTool::Twirl, 60.0);
        b.reverse = reverse;
        for _ in 0..holds {
            mesh.hold(&b, [50.0, 50.0, 1.0]);
        }
        // Signed angle between the node offset and its source offset.
        let p = [65.0, 50.0];
        let d = mesh.displacement(p[0], p[1]);
        let (ox, oy) = (p[0] - 50.0, p[1] - 50.0);
        let (sx, sy) = (ox + d[0], oy + d[1]);
        (ox * sy - oy * sx).atan2(ox * sx + oy * sy)
    };
    let (one, five) = (turn(false, 1), turn(false, 5));
    // Clockwise on screen (y down) moves content clockwise, so the source of
    // a point lies counterclockwise of it.
    assert!(one < 0.0 && five < one, "{one} {five}");
    let back = turn(true, 5);
    assert!((back + five).abs() < 1e-3, "{back} {five}");
}

#[test]
fn pucker_pulls_toward_the_centre_bloat_pushes_away_and_alt_swaps() {
    let radial = |tool: LiquifyTool, reverse: bool| {
        let mut mesh = LiquifyMesh::new(100, 100);
        let mut b = brush(tool, 60.0);
        b.reverse = reverse;
        for _ in 0..4 {
            mesh.hold(&b, [50.0, 50.0, 1.0]);
        }
        let d = mesh.displacement(62.0, 50.0);
        // Positive: the output reads from farther out, so content moved in.
        d[0]
    };
    assert!(radial(LiquifyTool::Pucker, false) > 0.1);
    assert!(radial(LiquifyTool::Bloat, false) < -0.1);
    assert!(radial(LiquifyTool::Pucker, true) < -0.1);
    assert!(radial(LiquifyTool::Bloat, true) > 0.1);
}

#[test]
fn push_left_moves_left_on_an_upward_drag_and_right_on_a_downward_one() {
    let push = |from: f64, to: f64, reverse: bool| {
        let mut mesh = LiquifyMesh::new(100, 100);
        let mut b = brush(LiquifyTool::PushLeft, 40.0);
        b.reverse = reverse;
        mesh.stroke_to(&b, [50.0, from, 1.0], [50.0, to, 1.0]);
        mesh.displacement(50.0, 50.0)[0]
    };
    // Content moving left means the output reads from the right.
    assert!(push(60.0, 40.0, false) > 1.0);
    assert!(push(40.0, 60.0, false) < -1.0);
    assert!(push(60.0, 40.0, true) < -1.0);
}

#[test]
fn mirror_reflects_the_far_side_onto_the_left_of_the_stroke() {
    let mut mesh = LiquifyMesh::new(100, 100);
    let mut b = brush(LiquifyTool::Mirror, 40.0);
    b.density = 0.0;
    mesh.stroke_to(&b, [50.0, 80.0, 1.0], [50.0, 20.0, 1.0]);
    // Dragging up, left is -x: (44, 50) shows (56, 50).
    let d = mesh.displacement(44.0, 50.0);
    assert!(
        (44.0 + d[0] - 56.0).abs() < 0.6 && d[1].abs() < 0.6,
        "{d:?}"
    );
    // The right of the stroke is left alone.
    assert_eq!(mesh.displacement(58.0, 50.0), [0.0, 0.0]);
}

#[test]
fn turbulence_scrambles_deterministically() {
    let run = || {
        let mut mesh = LiquifyMesh::new(100, 100);
        let b = brush(LiquifyTool::Turbulence, 60.0);
        for _ in 0..6 {
            mesh.hold(&b, [50.0, 50.0, 1.0]);
        }
        mesh
    };
    let (a, b) = (run(), run());
    assert_eq!(a, b);
    assert!(!a.is_identity());
    let moved = (0..10).filter(|i| {
        let d = a.displacement(35.0 + 3.0 * f64::from(*i), 50.0);
        d[0].hypot(d[1]) > 0.05
    });
    assert!(moved.count() >= 5);
}

#[test]
fn frozen_areas_take_no_distortion_and_invert_swaps_them() {
    let warp = brush(LiquifyTool::ForwardWarp, 40.0);
    let mut mesh = LiquifyMesh::new(100, 100);
    mesh.freeze_all();
    assert!(mesh
        .stroke_to(&warp, [50.0, 50.0, 1.0], [60.0, 50.0, 1.0])
        .is_some());
    assert!(mesh.is_identity());
    mesh.invert_freeze();
    mesh.stroke_to(&warp, [50.0, 50.0, 1.0], [60.0, 50.0, 1.0]);
    assert!(!mesh.is_identity());

    // A painted freeze protects only where it was painted.
    let mut mesh = LiquifyMesh::new(100, 100);
    let mut freeze = brush(LiquifyTool::Freeze, 30.0);
    freeze.density = 0.0;
    mesh.stroke_to(&freeze, [20.0, 50.0, 1.0], [40.0, 50.0, 1.0]);
    assert_eq!(mesh.freeze_at(30.0, 50.0), 1.0);
    let big = brush(LiquifyTool::ForwardWarp, 120.0);
    mesh.stroke_to(&big, [50.0, 50.0, 1.0], [50.0, 60.0, 1.0]);
    assert_eq!(mesh.displacement(30.0, 50.0), [0.0, 0.0]);
    assert_ne!(mesh.displacement(70.0, 50.0), [0.0, 0.0]);
    let mut thaw = brush(LiquifyTool::Thaw, 30.0);
    thaw.density = 0.0;
    mesh.stroke_to(&thaw, [20.0, 50.0, 1.0], [40.0, 50.0, 1.0]);
    assert_eq!(mesh.freeze_at(30.0, 50.0), 0.0);
}

#[test]
fn restore_all_and_full_reconstruct_return_the_original_exactly() {
    let warp = brush(LiquifyTool::ForwardWarp, 40.0);
    let original = gradient(80, 60);
    let mut mesh = LiquifyMesh::new(80, 60);
    mesh.stroke_to(&warp, [30.0, 30.0, 1.0], [45.0, 35.0, 1.0]);
    let warped = mesh.clone();

    mesh.reconstruct(0.0);
    assert_eq!(mesh, warped, "amount 0 is a no-op");
    mesh.reconstruct(50.0);
    assert!(!mesh.is_identity());
    mesh.reconstruct(100.0);
    assert!(mesh.is_identity());

    let mut restored = warped.clone();
    restored.freeze_all();
    restored.reconstruct(100.0);
    assert_eq!(restored.d, warped.d, "Reconstruct spares frozen areas");
    restored.restore_all();
    assert!(restored.is_identity(), "Restore All does not");

    let mut buf = original.clone();
    liquify(&mut buf, &restored).unwrap();
    assert_eq!(buf.data.to_vec(), original.data.to_vec());
}

#[test]
fn reconstruct_tool_eases_a_warp_back() {
    let mut mesh = LiquifyMesh::new(100, 100);
    mesh.stroke_to(
        &brush(LiquifyTool::ForwardWarp, 50.0),
        [40.0, 50.0, 1.0],
        [60.0, 50.0, 1.0],
    );
    let before = mesh.displacement(60.0, 50.0)[0].abs();
    mesh.hold(&brush(LiquifyTool::Reconstruct, 50.0), [60.0, 50.0, 1.0]);
    let after = mesh.displacement(60.0, 50.0)[0].abs();
    assert!(after < before && after > 0.0, "{before} {after}");
}

#[test]
fn a_mesh_scaled_to_a_larger_layer_scales_its_offsets() {
    let mut mesh = LiquifyMesh::new(50, 40);
    mesh.stroke_to(
        &brush(LiquifyTool::ForwardWarp, 30.0),
        [20.0, 20.0, 1.0],
        [30.0, 20.0, 1.0],
    );
    assert_eq!(mesh.scaled_to(50, 40), mesh);
    let big = mesh.scaled_to(100, 80);
    let (a, b) = (mesh.displacement(30.0, 20.0), big.displacement(60.0, 40.0));
    assert!((b[0] - 2.0 * a[0]).abs() < 0.5, "{a:?} {b:?}");
}

#[test]
fn a_stroke_to_a_far_off_point_returns() {
    let mut mesh = LiquifyMesh::new(24, 16);
    let b = brush(LiquifyTool::ForwardWarp, 30.0);
    assert!(mesh
        .stroke_to(&b, [0.0, 0.0, 1.0], [1e15, 0.0, 1.0])
        .is_some());
    mesh.stroke_to(&b, [-1e308, 5.0, 1.0], [1e308, 5.0, 1.0]);
    mesh.stroke_to(&b, [500.0, 500.0, 1.0], [900.0, 900.0, 1.0]);
    assert!(mesh
        .stroke_to(&b, [f64::NAN, 0.0, 1.0], [1.0, 1.0, 1.0])
        .is_none());
}

#[test]
fn the_proxy_reduces_and_renders_through_the_mesh() {
    let (w, h) = (64u32, 32u32);
    let n = (w * h) as usize;
    let r: Vec<u8> = (0..n).map(|i| ((i % 64) * 4) as u8).collect();
    let g = vec![100u8; n];
    let b = vec![200u8; n];
    let a = vec![128u8; n];
    let proxy = LiquifyProxy::new(w, h, [&r, &g, &b], Some(&a), 32);
    assert_eq!((proxy.width(), proxy.height(), proxy.scale()), (32, 16, 2));
    let mut mesh = LiquifyMesh::new(w, h);
    let mut out = vec![0u8; 32 * 16 * 4];
    let all = PsdRect {
        left: 0,
        top: 0,
        right: 32,
        bottom: 16,
    };
    proxy.render(&mesh, all, &mut out, None);
    // Premultiplied: 100 at alpha 128 is 50.
    assert_eq!(&out[4..8], &[(10 * 128 / 255) as u8, 50, 100, 128]);

    mesh.freeze_all();
    proxy.render(&mesh, all, &mut out, Some([255, 0, 0]));
    assert_eq!(out[7], 192, "a half-strength opaque tint over half alpha");
    assert!(out[4] > 120 && out[5] < 30);

    let pr = proxy.proxy_rect(PsdRect {
        left: 10,
        top: 4,
        right: 20,
        bottom: 9,
    });
    assert_eq!((pr.left, pr.top, pr.right, pr.bottom), (4, 1, 11, 6));
}

#[test]
#[ignore = "profiling: run with --release --ignored --nocapture"]
fn profile_commit_on_a_24_mp_layer() {
    let (w, h) = (6000u32, 4000u32);
    let mut buf = PixelBuffer::new(w, h, 3);
    let mut mesh = LiquifyMesh::new(w, h);
    let started = std::time::Instant::now();
    let warp = brush(LiquifyTool::ForwardWarp, 800.0);
    mesh.stroke_to(&warp, [2000.0, 2000.0, 1.0], [3500.0, 2200.0, 1.0]);
    let strokes = started.elapsed();
    let started = std::time::Instant::now();
    liquify(&mut buf, &mesh).unwrap();
    eprintln!(
        "24 MP: 1500 px stroke of an 800 px brush {strokes:?}, commit {:?}",
        started.elapsed()
    );
}
