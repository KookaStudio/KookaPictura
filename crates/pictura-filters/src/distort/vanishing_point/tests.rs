use std::sync::Arc;

use pictura_core::PixelBuffer;

use super::geometry::{focal_for, v3, Camera, P2, V3};
use super::planes::Part;
use super::{vanishing_point, DragMode, VpEdit, VpEditor, VpPlanes, VpValidity};
use crate::{apply, Filter};

const W: u32 = 800;
const H: u32 = 600;

fn close(a: P2, b: P2, tol: f64) -> bool {
    (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol
}

fn square_on() -> [P2; 4] {
    [
        (200.0, 100.0),
        (600.0, 100.0),
        (600.0, 500.0),
        (200.0, 500.0),
    ]
}

/// The camera every synthetic view uses: principal point at the document
/// centre, a slightly wide lens.
fn camera() -> Camera {
    Camera {
        centre: (W as f64 / 2.0, H as f64 / 2.0),
        half: (W as f64).hypot(H as f64) / 2.0,
        f: 1.7,
    }
}

/// A 3D rectangle turned 35° about the vertical and tilted 20°, so both its
/// edge pairs converge, and its image through [`camera`].
fn tilted() -> ([P2; 4], V3, V3) {
    let (a, b) = (35f64.to_radians(), 20f64.to_radians());
    let eu = v3(a.cos(), 0.0, a.sin());
    let ev = v3(-a.sin() * b.sin(), b.cos(), a.cos() * b.sin());
    let origin = v3(-0.6, -0.4, 5.0);
    let cam = camera();
    let at = |u: f64, v: f64| {
        cam.project(origin.add(eu.scale(u)).add(ev.scale(v)))
            .unwrap()
    };
    (
        [at(0.0, 0.0), at(1.6, 0.0), at(1.6, 1.1), at(0.0, 1.1)],
        eu,
        ev,
    )
}

/// Where the image line through `a` and `b` passes nearest `p`.
fn line_miss(a: P2, b: P2, p: P2) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs() / dx.hypot(dy)
}

fn planes_with(q: [P2; 4]) -> VpPlanes {
    let mut planes = VpPlanes::new(W, H);
    planes.add(q);
    planes
}

#[test]
fn a_square_on_rectangle_is_valid_with_an_even_grid() {
    let planes = planes_with(square_on());
    assert_eq!(planes.validity(0), VpValidity::Valid);
    let lines = planes.grid_lines(0);
    // A 400 px square at Grid Size 100: three inner lines each way.
    assert_eq!(lines.len(), 6);
    let mut xs: Vec<f64> = lines
        .iter()
        .filter(|[a, b]| (a.0 - b.0).abs() < 1e-6)
        .map(|[a, _]| a.0)
        .collect();
    xs.sort_by(f64::total_cmp);
    for (x, want) in xs.iter().zip([300.0, 400.0, 500.0]) {
        assert!((x - want).abs() < 1e-6, "{x} vs {want}");
    }
}

#[test]
fn crossed_concave_and_degenerate_quads_are_red() {
    let planes = VpPlanes::new(W, H);
    let crossed = [
        (200.0, 100.0),
        (600.0, 500.0),
        (600.0, 100.0),
        (200.0, 500.0),
    ];
    let concave = [
        (200.0, 100.0),
        (600.0, 100.0),
        (300.0, 200.0),
        (200.0, 500.0),
    ];
    let flat = [
        (200.0, 100.0),
        (300.0, 100.0),
        (400.0, 100.0),
        (500.0, 100.0),
    ];
    for q in [crossed, concave, flat] {
        assert_eq!(planes.quad_validity(&q), VpValidity::Aspect, "{q:?}");
    }
}

#[test]
fn a_quad_no_camera_sees_as_a_rectangle_is_yellow() {
    // Both vanishing points lie up and to the right of the centre, so their
    // directions meet at under 90° and f² comes out negative.
    let mut planes = VpPlanes::new(400, 400);
    let q = [
        (160.0, 100.0),
        (330.0, 110.0),
        (300.0, 280.0),
        (100.0, 300.0),
    ];
    assert!(focal_for(&q, (200.0, 200.0), 400f64.hypot(400.0) / 2.0).is_none());
    let i = planes.add(q);
    assert_eq!(planes.validity(i), VpValidity::Vanishing);
    assert!(planes.grid_lines(i).is_empty());
}

#[test]
fn a_projected_rectangle_recovers_its_camera_and_grid_converges() {
    let (q, eu, ev) = tilted();
    let cam = camera();
    let f = focal_for(&q, cam.centre, cam.half).unwrap();
    assert!((f - cam.f).abs() < 1e-6, "focal {f}");
    let mut planes = planes_with(q);
    assert_eq!(planes.validity(0), VpValidity::Valid);
    // The plane is about 270 px across; a finer grid gives several lines.
    planes.set_grid(0, 40.0);
    let vanish = |d: V3| {
        let k = cam.half * cam.f / d.z;
        (cam.centre.0 + d.x * k, cam.centre.1 + d.y * k)
    };
    let (vu, vv) = (vanish(eu), vanish(ev));
    let lines = planes.grid_lines(0);
    assert!(lines.len() >= 4);
    // Every grid line runs toward one of the two vanishing points.
    for [a, b] in lines {
        let miss = line_miss(a, b, vu).min(line_miss(a, b, vv));
        assert!(
            miss < 1e-3,
            "a grid line misses both vanishing points by {miss}"
        );
    }
}

#[test]
fn resizing_an_edge_leaves_the_grid_on_the_surface() {
    let (q, _, _) = tilted();
    let mut planes = planes_with(q);
    let before = planes.grid_lines(0);
    // Drag edge 1 (corner 1 → 2) outward, past its node.
    let node = planes.edge_nodes(0)[1];
    let centre = planes.edge_nodes(0)[3];
    let out = (
        node.0 + (node.0 - centre.0) * 0.4,
        node.1 + (node.1 - centre.1) * 0.4,
    );
    planes.move_edge(0, 1, out);
    assert_eq!(planes.validity(0), VpValidity::Valid);
    let after = planes.grid_lines(0);
    assert!(after.len() > before.len());
    // Lines across the resized axis stay put; lines along it only lengthen.
    for [a, b] in &before {
        let kept = after.iter().any(|[c, d]| {
            (close(*a, *c, 1e-6) && close(*b, *d, 1e-6))
                || (close(*a, *c, 1e-6) && line_miss(*c, *d, *b) < 1e-6)
        });
        assert!(kept, "the line {a:?}–{b:?} moved");
    }
}

fn torn(planes: &mut VpPlanes, from: usize, edge: usize) -> usize {
    let child = planes
        .tear_off(from, edge)
        .expect("a valid plane tears off");
    let node = planes.edge_nodes(from)[edge];
    let other = planes.edge_nodes(from)[(edge + 2) % 4];
    // Drag away from the parent, beyond the edge.
    let p = (
        node.0 + (node.0 - other.0) * 0.5,
        node.1 + (node.1 - other.1) * 0.5 - 40.0,
    );
    planes.extend(child, p);
    child
}

#[test]
fn a_torn_off_plane_is_perpendicular_and_shares_the_edge() {
    let (q, _, _) = tilted();
    let mut planes = planes_with(q);
    let child = torn(&mut planes, 0, 1);
    assert_eq!(planes.validity(child), VpValidity::Valid);
    assert_eq!(planes.parent(child), Some(0));
    assert_eq!(planes.angle(child), Some(90.0));
    let (pn, cn) = (planes.frame(0).unwrap().n, planes.frame(child).unwrap().n);
    assert!(pn.dot(cn).abs() < 1e-9, "normals at {}", pn.dot(cn));
    let (p, c) = (planes.corners(0), planes.corners(child));
    assert!(
        close(c[0], p[1], 1e-6) && close(c[1], p[2], 1e-6),
        "{c:?} vs {p:?}"
    );
    assert!(planes.has_children(0));
}

#[test]
fn the_angle_turns_a_related_plane_about_its_hinge() {
    let (q, _, _) = tilted();
    let mut planes = planes_with(q);
    let child = torn(&mut planes, 0, 2);
    let dot = |planes: &VpPlanes| {
        planes
            .frame(0)
            .unwrap()
            .n
            .dot(planes.frame(child).unwrap().n)
            .abs()
    };
    assert!(planes.set_angle(child, 45.0));
    assert!((dot(&planes) - 45f64.to_radians().cos()).abs() < 1e-9);
    assert!(planes.set_angle(child, 0.0));
    assert!((dot(&planes) - 1.0).abs() < 1e-9);
    // Alt-drag: aim the far edge's node where it sits at 60°.
    assert!(planes.set_angle(child, 60.0));
    let target = planes.edge_nodes(child)[2];
    planes.set_angle(child, 120.0);
    planes.aim(child, target);
    let angle = planes.angle(child).unwrap();
    assert!((angle - 60.0).abs() < 0.1, "aimed at {angle}°");
}

#[test]
fn a_plane_with_related_planes_cannot_turn() {
    let (q, _, _) = tilted();
    let mut planes = planes_with(q);
    let child = torn(&mut planes, 0, 1);
    assert!(planes.angle_editable(child));
    let grandchild = torn(&mut planes, child, 2);
    assert!(!planes.angle_editable(0), "a root has no angle");
    assert!(
        !planes.angle_editable(child),
        "the parent's angle is locked"
    );
    assert!(!planes.set_angle(child, 30.0));
    assert!(planes.angle_editable(grandchild));
}

#[test]
fn related_planes_follow_their_parent() {
    let (q, _, _) = tilted();
    let mut planes = planes_with(q);
    let child = torn(&mut planes, 0, 1);
    let before = planes.corners(child);
    // A small reshape keeps this wide view's camera real.
    let c2 = planes.corners(0)[2];
    planes.move_corner(0, 2, (c2.0 + 3.0, c2.1 + 2.0));
    assert_eq!(planes.validity(0), VpValidity::Valid);
    let (p, c) = (planes.corners(0), planes.corners(child));
    assert_ne!(c, before);
    assert!(close(c[0], p[1], 1e-6) && close(c[1], p[2], 1e-6));
    // A crossed parent turns the whole family red and freezes its outlines.
    let frozen = planes.corners(child);
    planes.move_corner(0, 2, (p[0].0 - 50.0, p[0].1 - 50.0));
    assert_eq!(planes.validity(0), VpValidity::Aspect);
    assert_eq!(planes.validity(child), VpValidity::Aspect);
    assert_eq!(planes.corners(child), frozen);
}

#[test]
fn grid_lines_continue_across_a_flat_hinge() {
    let (q, _, _) = tilted();
    let mut planes = planes_with(q);
    let child = torn(&mut planes, 0, 2);
    assert!(planes.set_angle(child, 0.0));
    // The parent's lines across edge 2 end on it; the child's lines across
    // its hinge start at the same points.
    let parent: Vec<P2> = planes.grid_lines(0).iter().map(|l| l[1]).collect();
    let starts: Vec<P2> = planes.grid_lines(child).iter().map(|l| l[0]).collect();
    let hinge = (planes.corners(0)[2], planes.corners(0)[3]);
    let on_hinge: Vec<&P2> = parent
        .iter()
        .filter(|p| line_miss(hinge.0, hinge.1, **p) < 1e-6)
        .collect();
    assert!(!on_hinge.is_empty());
    for p in on_hinge {
        assert!(
            starts.iter().any(|s| close(*s, *p, 1e-6)),
            "no child line continues {p:?}"
        );
    }
}

#[test]
fn removing_a_plane_removes_its_related_planes() {
    let (q, _, _) = tilted();
    let mut planes = planes_with(q);
    torn(&mut planes, 0, 1);
    let other = planes.add(square_on());
    torn(&mut planes, other, 0);
    assert_eq!(planes.len(), 4);
    planes.remove(0);
    assert_eq!(planes.len(), 2);
    assert_eq!(planes.parent(0), None);
    assert_eq!(planes.parent(1), Some(0));
    assert_eq!(planes.validity(1), VpValidity::Valid);
}

#[test]
fn render_grids_draws_black_lines_into_the_layer() {
    let planes = planes_with(square_on());
    let edit = VpEdit {
        lines: planes.rendered_grid(-150.0, -50.0),
    };
    // A 500×500 layer whose origin is at document (150, 50).
    let (w, h) = (500usize, 500usize);
    let mut opaque = PixelBuffer::new(w as u32, h as u32, 3);
    opaque.data.as_mut_slice().fill(255);
    vanishing_point(&mut opaque, &edit).unwrap();
    let at = |b: &PixelBuffer, plane: usize, x: usize, y: usize| b.data[plane * w * h + y * w + x];
    // Document x = 300 is a grid line on the boundary between layer
    // columns 149 and 150, so each takes half the ink.
    let half = |v: u8| (120..=136).contains(&v);
    assert!(half(at(&opaque, 0, 149, 251)) && half(at(&opaque, 0, 150, 251)));
    assert_eq!(at(&opaque, 0, 100, 101), 255, "inside a cell");
    assert_eq!(at(&opaque, 0, 20, 20), 255, "outside the plane");
    // The outline at document y = 100, between layer rows 49 and 50.
    assert!(half(at(&opaque, 1, 120, 49)) && half(at(&opaque, 1, 120, 50)));

    let mut clear = PixelBuffer::new(w as u32, h as u32, 4);
    vanishing_point(&mut clear, &edit).unwrap();
    assert!(half(at(&clear, 3, 150, 251)));
    assert_eq!(at(&clear, 0, 150, 251), 0);
    assert_eq!(at(&clear, 3, 100, 101), 0);
}

#[test]
fn an_empty_edit_changes_nothing_and_runs_are_deterministic() {
    let mut buf = PixelBuffer::new(32, 32, 4);
    for (i, v) in buf.data.as_mut_slice().iter_mut().enumerate() {
        *v = (i * 7 % 256) as u8;
    }
    let original = buf.clone();
    let empty = Filter::VanishingPoint {
        edit: Arc::new(VpEdit::default()),
    };
    apply(&empty, &mut buf).unwrap();
    assert_eq!(buf, original);
    let edit = Filter::VanishingPoint {
        edit: Arc::new(VpEdit {
            lines: vec![[(1.5, 2.0), (30.0, 27.25)], [(16.0, 0.0), (16.0, 32.0)]],
        }),
    };
    let (mut a, mut b) = (original.clone(), original);
    apply(&edit, &mut a).unwrap();
    apply(&edit, &mut b).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, buf);
}

#[test]
fn the_editor_creates_cycles_deletes_and_undoes() {
    let mut editor = VpEditor::new(VpPlanes::new(W, H));
    let first = editor.create(square_on());
    let second = editor.create([
        (300.0, 200.0),
        (700.0, 200.0),
        (700.0, 550.0),
        (300.0, 550.0),
    ]);
    assert_eq!(editor.active(), Some(second));
    // Both contain (400, 300): Ctrl-click walks through them.
    assert_eq!(editor.cycle((400.0, 300.0)), Some(first));
    assert_eq!(editor.cycle((400.0, 300.0)), Some(second));
    assert!(editor.delete_active());
    assert_eq!(editor.planes().len(), 1);
    assert!(editor.undo());
    assert_eq!(editor.planes().len(), 2);
    assert!(editor.redo());
    assert_eq!(editor.planes().len(), 1);
    assert!(editor.undo() && editor.undo() && editor.undo());
    assert!(editor.planes().is_empty());
    assert!(!editor.undo());
}

#[test]
fn hits_prefer_nodes_then_the_newest_plane() {
    let mut editor = VpEditor::new(VpPlanes::new(W, H));
    editor.create(square_on());
    editor.create([
        (300.0, 200.0),
        (700.0, 200.0),
        (700.0, 550.0),
        (300.0, 550.0),
    ]);
    editor.set_active(Some(0));
    assert_eq!(editor.hit((601.0, 99.0), 4.0), Some((0, Part::Corner(1))));
    assert_eq!(editor.hit((400.0, 101.0), 4.0), Some((0, Part::Edge(0))));
    assert_eq!(editor.hit((400.0, 300.0), 4.0), Some((0, Part::Inside)));
    assert_eq!(editor.hit((650.0, 400.0), 4.0), Some((1, Part::Inside)));
    assert_eq!(editor.hit((50.0, 50.0), 4.0), None);
}

#[test]
fn a_tear_off_drag_is_one_undo_step() {
    let (q, _, _) = tilted();
    let mut editor = VpEditor::new(VpPlanes::new(W, H));
    editor.create(q);
    let node = editor.planes().edge_nodes(0)[1];
    assert!(editor.begin(0, Part::Edge(1), DragMode::TearOff, node));
    assert_eq!(editor.active(), Some(1));
    editor.drag_to((node.0 + 40.0, node.1 - 30.0));
    editor.drag_to((node.0 + 80.0, node.1 - 60.0));
    editor.end();
    assert_eq!(editor.planes().len(), 2);
    assert!(editor.undo());
    assert_eq!(editor.planes().len(), 1);
    // A root's edge cannot be Alt-turned, and an invalid plane cannot tear.
    assert!(!editor.begin(0, Part::Edge(2), DragMode::Angle, node));
    let red = editor.create([(10.0, 10.0), (90.0, 90.0), (90.0, 10.0), (10.0, 90.0)]);
    assert!(!editor.begin(red, Part::Edge(0), DragMode::TearOff, (50.0, 10.0)));
}

#[test]
fn a_slid_related_plane_stays_on_its_hinge() {
    let (q, _, _) = tilted();
    let mut planes = planes_with(q);
    let child = torn(&mut planes, 0, 1);
    let before = planes.corners(child);
    let mid = planes.edge_nodes(child)[0];
    planes.slide(child, mid, (mid.0 + 3.0, mid.1 + 20.0));
    let after = planes.corners(child);
    assert_ne!(before, after);
    let (p1, p2) = (planes.corners(0)[1], planes.corners(0)[2]);
    for c in [after[0], after[1]] {
        assert!(line_miss(p1, p2, c) < 1e-6, "{c:?} left the hinge");
    }
}
