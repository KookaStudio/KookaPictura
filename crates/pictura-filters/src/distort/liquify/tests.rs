use super::*;
use pictura_core::{PixelBuffer, PsdRect};

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

fn segments(lines: &[[f64; 2]]) -> Vec<([f64; 2], [f64; 2])> {
    lines
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&[a, b, _]| (a, b))
        .collect()
}

#[test]
fn the_grid_is_drawn_where_the_distortion_puts_it() {
    let mut mesh = LiquifyMesh::new(64, 48);
    let lines = mesh.grid_lines([0.0, 0.0, 64.0, 48.0], 16.0, 4.0);
    assert_eq!(lines.len() % 3, 0);
    // Undistorted: every segment lies on a multiple of 16, the layer's far
    // edges included.
    let on_grid = |v: f64| (v / 16.0 - (v / 16.0).round()).abs() < 1e-9;
    let straight = segments(&lines);
    assert!(!straight.is_empty());
    assert!(straight
        .iter()
        .all(|(a, b)| (on_grid(a[0]) && on_grid(b[0])) || (on_grid(a[1]) && on_grid(b[1]))));
    // Output p shows source p + 3, so the source line x = 32 shows at 29.
    mesh.d.fill([3.0, 0.0]);
    let shifted = segments(&mesh.grid_lines([0.0, 0.0, 64.0, 48.0], 16.0, 4.0));
    assert!(shifted
        .iter()
        .any(|(a, b)| (a[0] - 29.0).abs() < 1e-6 && (b[0] - 29.0).abs() < 1e-6));
    // A twirl bends the lines but keeps them short and joined: no segment is
    // longer than a lattice cell's diagonal.
    let mut twirled = LiquifyMesh::new(64, 48);
    for _ in 0..30 {
        twirled.hold(&brush(LiquifyTool::Twirl, 40.0), [32.0, 24.0, 1.0]);
    }
    let bent = segments(&twirled.grid_lines([0.0, 0.0, 64.0, 48.0], 16.0, 2.0));
    assert!(bent
        .iter()
        .all(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]) <= 2.0 * std::f64::consts::SQRT_2 + 1e-9));
    assert!(bent.len() > straight.len());
    // Only the asked rectangle is traced.
    let few = mesh.grid_lines([0.0, 0.0, 10.0, 10.0], 16.0, 4.0);
    assert!(few.len() < 3 * shifted.len());
}

#[test]
fn the_brush_fades_to_nothing_at_its_rim() {
    for density in [0.0, 0.5, 1.0] {
        assert!(falloff(0.0, density) > 0.99, "{density}");
        let near_rim = falloff(0.999, density);
        assert!(near_rim < 0.01, "density {density}: {near_rim}");
        let mut last = 1.0;
        for i in 0..100 {
            let w = falloff(f64::from(i) / 100.0, density);
            assert!(w <= last + 1e-12, "monotonic at density {density}");
            last = w;
        }
    }
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
fn a_mesh_scaled_to_a_larger_layer_scales_its_clip() {
    let mut mesh = LiquifyMesh::new(50, 40);
    mesh.set_clip(Some(PsdRect {
        left: 10,
        top: 8,
        right: 30,
        bottom: 24,
    }));
    let big = mesh.scaled_to(100, 80);
    assert_eq!(
        big.clip(),
        Some(PsdRect {
            left: 20,
            top: 16,
            right: 60,
            bottom: 48,
        })
    );
    assert_eq!(big.displacement(5.0, 5.0), [0.0, 0.0]);
    assert_eq!(mesh.scaled_to(50, 40).clip(), mesh.clip());
}

#[test]
fn a_dab_outside_the_clip_moves_nothing() {
    let mut mesh = LiquifyMesh::new(40, 30);
    mesh.set_clip(Some(PsdRect {
        left: 0,
        top: 0,
        right: 20,
        bottom: 30,
    }));
    let b = brush(LiquifyTool::ForwardWarp, 10.0);
    assert!(mesh
        .stroke_to(&b, [30.0, 15.0, 1.0], [38.0, 15.0, 1.0])
        .is_none());
    assert!(mesh.is_identity());
    assert_eq!(mesh.displacement(30.0, 15.0), [0.0, 0.0]);
}

#[test]
fn a_clip_leaves_the_picture_outside_it_untouched() {
    let clip = PsdRect {
        left: 10,
        top: 8,
        right: 30,
        bottom: 22,
    };
    let mut mesh = LiquifyMesh::new(40, 30);
    mesh.set_clip(Some(clip));
    let b = brush(LiquifyTool::ForwardWarp, 14.0);
    let dirty = mesh
        .stroke_to(&b, [16.0, 15.0, 1.0], [26.0, 15.0, 1.0])
        .expect("the drag is inside the clip");
    assert!(
        dirty.left >= clip.left
            && dirty.top >= clip.top
            && dirty.right <= clip.right
            && dirty.bottom <= clip.bottom,
        "{dirty:?}"
    );
    let mut buf = gradient(40, 30);
    let before = buf.clone();
    liquify(&mut buf, &mesh).unwrap();
    let n = 40 * 30;
    let mut changed = false;
    for y in 0..30 {
        for x in 0..40 {
            let inside = x >= clip.left && x < clip.right && y >= clip.top && y < clip.bottom;
            for c in 0..3 {
                let i = c * n + y as usize * 40 + x as usize;
                if inside {
                    changed |= buf.data[i] != before.data[i];
                } else {
                    assert_eq!(buf.data[i], before.data[i], "({x},{y}) c{c}");
                }
            }
        }
    }
    assert!(changed, "the drag changes pixels inside the clip");
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

fn source(w: u32, h: u32, alpha: u8) -> (LiquifySource, PixelBuffer) {
    let buf = gradient(w, h);
    let n = (w * h) as usize;
    let plane = |c: usize| pictura_core::Plane::from(buf.data[c * n..(c + 1) * n].to_vec());
    let alpha = pictura_core::Plane::from(vec![alpha; n]);
    (
        LiquifySource::new(w, h, [plane(0), plane(1), plane(2)], Some(alpha)),
        buf,
    )
}

fn render(
    src: &mut LiquifySource,
    mesh: &LiquifyMesh,
    origin: [f64; 2],
    zoom: f64,
    size: (usize, usize),
) -> Vec<u8> {
    let mut out = vec![0u8; size.0 * size.1 * 4];
    src.render(mesh, origin, zoom, size, &mut out, None);
    out
}

#[test]
fn the_view_at_100_percent_shows_the_layer_exactly() {
    let (mut src, buf) = source(40, 30, 255);
    let out = render(
        &mut src,
        &LiquifyMesh::new(40, 30),
        [0.0, 0.0],
        1.0,
        (40, 30),
    );
    let n = 40 * 30;
    for i in 0..n {
        for c in 0..3 {
            assert_eq!(out[i * 4 + c], buf.data[c * n + i], "pixel {i} channel {c}");
        }
        assert_eq!(out[i * 4 + 3], 255);
    }
}

#[test]
fn a_clipped_view_is_clear_outside_its_bounds() {
    let (mut src, buf) = source(40, 30, 255);
    let clip = PsdRect {
        left: 10,
        top: 5,
        right: 30,
        bottom: 25,
    };
    let mut mesh = LiquifyMesh::new(40, 30);
    mesh.set_clip(Some(clip));
    let out = render(&mut src, &mesh, [0.0, 0.0], 1.0, (40, 30));
    let n = 40 * 30;
    for y in 0..30 {
        for x in 0..40 {
            let i = y as usize * 40 + x as usize;
            let inside = x >= clip.left && x < clip.right && y >= clip.top && y < clip.bottom;
            if inside {
                for c in 0..3 {
                    assert_eq!(out[i * 4 + c], buf.data[c * n + i], "({x},{y}) c{c}");
                }
                assert_eq!(out[i * 4 + 3], 255, "({x},{y})");
            } else {
                assert_eq!(&out[i * 4..i * 4 + 4], &[0, 0, 0, 0], "({x},{y})");
            }
        }
    }
}

#[test]
fn a_magnified_view_interpolates_instead_of_repeating_pixels() {
    let (mut src, _) = source(40, 30, 255);
    // Four screen pixels per layer pixel across the red ramp.
    let out = render(
        &mut src,
        &LiquifyMesh::new(40, 30),
        [10.0, 10.0],
        4.0,
        (16, 1),
    );
    let reds: Vec<u8> = (0..16).map(|i| out[i * 4]).collect();
    assert!(reds.windows(2).all(|w| w[1] >= w[0]), "{reds:?}");
    let distinct = reds.windows(2).filter(|w| w[1] != w[0]).count();
    assert!(distinct >= 10, "a smooth ramp, not 4-px blocks: {reds:?}");
}

#[test]
fn the_view_matches_the_commit_at_100_percent() {
    let (mut src, mut buf) = source(80, 60, 255);
    let mut mesh = LiquifyMesh::new(80, 60);
    mesh.stroke_to(
        &brush(LiquifyTool::ForwardWarp, 40.0),
        [30.0, 30.0, 1.0],
        [45.0, 34.0, 1.0],
    );
    let out = render(&mut src, &mesh, [0.0, 0.0], 1.0, (80, 60));
    liquify(&mut buf, &mesh).unwrap();
    let n = 80 * 60;
    for i in 0..n {
        for c in 0..3 {
            let (a, b) = (i32::from(out[i * 4 + c]), i32::from(buf.data[c * n + i]));
            assert!((a - b).abs() <= 1, "pixel {i} channel {c}: {a} vs {b}");
        }
    }
}

#[test]
fn a_minified_view_averages_and_off_layer_pixels_are_clear() {
    let (mut src, _) = source(64, 32, 128);
    let mut mesh = LiquifyMesh::new(64, 32);
    // An eighth-size view of the 4 px checker averages to its middle grey.
    let out = render(&mut src, &mesh, [0.0, 0.0], 0.125, (8, 4));
    let blue = out[(2 * 8 + 3) * 4 + 2];
    assert!(
        (60..=70).contains(&blue),
        "premultiplied (40+220)/2 at half alpha: {blue}"
    );
    assert_eq!(out[(2 * 8 + 3) * 4 + 3], 128);
    // Off the layer's right edge: clear.
    let out = render(&mut src, &mesh, [60.0, 0.0], 1.0, (8, 1));
    assert_eq!(&out[4 * 4..4 * 4 + 4], &[0, 0, 0, 0]);
    assert_eq!(out[3 * 4 + 3], 128);

    mesh.freeze_all();
    let mut tinted = vec![0u8; 4 * 4];
    src.render(
        &mesh,
        [0.0, 0.0],
        1.0,
        (4, 1),
        &mut tinted,
        Some([255, 0, 0]),
    );
    assert_eq!(
        tinted[3], 192,
        "a half-strength opaque tint over half alpha"
    );
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
