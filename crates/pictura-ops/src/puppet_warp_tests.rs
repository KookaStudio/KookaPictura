use super::*;

fn pin(rest: [f64; 2], target: [f64; 2], angle: Option<f64>, ordinal: u32) -> Pin {
    Pin {
        rest,
        target,
        angle,
        ordinal,
    }
}

#[test]
fn pins_constrain_vertices() {
    let pins = [
        pin([0.0, 0.0], [0.0, 0.0], Some(0.0), 0),
        pin([100.0, 100.0], [110.0, 100.0], Some(0.0), 1),
    ];
    let mesh = Mesh::solve(100, 100, &pins, 1, 1, 0).unwrap();
    assert_eq!(mesh.source_at([0.0, 0.0]), Some([0.0, 0.0]));
    assert_eq!(mesh.source_at([110.0, 100.0]), Some([100.0, 100.0]));
    assert!(!mesh.is_identity());
}

#[test]
fn stationary_anchor_neighborhood() {
    let pins = [
        pin([0.0, 0.0], [0.0, 0.0], Some(0.0), 0),
        pin([100.0, 0.0], [110.0, 0.0], Some(0.0), 1),
    ];
    let mesh = Mesh::solve(100, 100, &pins, 1, 1, 0).unwrap();
    let source = mesh.source_at([0.5, 0.5]).unwrap();
    assert!((source[0] - 0.5).hypot(source[1] - 0.5) < 1.0);
}

#[test]
fn pin_rotation_modes() {
    let one = [pin([0.0, 0.0], [5.0, 7.0], None, 0)];
    let mesh = Mesh::solve(10, 10, &one, 1, 1, 0).unwrap();
    assert_eq!(
        mesh.bounds(),
        PsdRect {
            left: 5,
            top: 7,
            right: 15,
            bottom: 17
        }
    );
    let rotated = [pin(
        [0.0, 0.0],
        [0.0, 0.0],
        Some(std::f64::consts::FRAC_PI_2),
        0,
    )];
    let mesh = Mesh::solve(10, 10, &rotated, 1, 1, 0).unwrap();
    let p = mesh.source_at([-10.0, 10.0]).unwrap();
    assert!((p[0] - 10.0).abs() < 1e-7 && (p[1] - 10.0).abs() < 1e-7);
    let pair = [
        pin([0.0, 0.0], [0.0, 0.0], None, 0),
        pin([10.0, 0.0], [0.0, 10.0], None, 1),
    ];
    let mesh = Mesh::solve(10, 10, &pair, 1, 1, 0).unwrap();
    let p = mesh.source_at([-10.0, 10.0]).unwrap();
    assert!((p[0] - 10.0).abs() < 1e-7 && (p[1] - 10.0).abs() < 1e-7);
    assert!(Mesh::solve(10, 10, &[one[0], one[0]], 1, 1, 0).is_none());
}

#[test]
fn fold_depth_order() {
    let a = pin([0.0, 0.0], [10.0, 0.0], Some(0.0), 0);
    let b = pin([10.0, 0.0], [0.0, 0.0], Some(0.0), 1);
    let back = Mesh::solve(10, 10, &[a, b], 1, 1, 0).unwrap();
    let front = Mesh::solve(10, 10, &[b, a], 1, 1, 0).unwrap();
    let x = back.source_at([5.0, 1.0]);
    assert_eq!(x, back.source_at([5.0, 1.0]));
    assert!(x.is_some() && front.source_at([5.0, 1.0]).is_some());
}

#[test]
fn warp_sampling_and_bounds() {
    let mesh = Mesh::solve(2, 1, &[pin([0.0, 0.0], [0.5, 0.0], Some(0.0), 0)], 1, 1, 0).unwrap();
    let mut src = PixelBuffer::new(2, 1, 4);
    src.data.copy_from_slice(&[255, 0, 0, 0, 0, 255, 255, 0]);
    let out = mesh.raster(&src, None, [0.0, 0.0], 1.0, 3, 1).unwrap();
    assert_eq!(out.data[1], 255);
    assert_eq!(out.data[5], 0);
    assert!((out.data[9] as i32 - 128).abs() <= 1);
    let mask = PixelBuffer::new(2, 1, 1);
    let padded = Mesh::solve(2, 1, &[], 1, 1, 1).unwrap();
    let out = padded
        .raster(&mask, Some(255), [-1.0, -1.0], 1.0, 4, 3)
        .unwrap();
    assert_eq!(out.data[0], 255);
    assert_eq!(out.data[5], 0);
}

#[test]
fn oversized_mesh_is_refused() {
    assert!(Mesh::solve(4_194_305, 1, &[], 1, 1, 0).is_none());
    assert!(Mesh::solve(10, 10, &[], 1, 1, i32::MAX).is_none());
    let mesh = Mesh::solve(10, 10, &[], 1, 1, 0).unwrap();
    assert!(!mesh.can_raster([0.0, 0.0], 1.0, 4_194_305, 1));
    assert!(!mesh.can_raster([0.0, 0.0], f64::NAN, 10, 10));
}
