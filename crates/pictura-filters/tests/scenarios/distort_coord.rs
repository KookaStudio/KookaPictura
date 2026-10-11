//! Spec scenarios from the replaced `distort/coord.rs` unit tests, run through
//! `apply` against the ported photorust engine.

use super::shims::*;
use pictura_core::PixelBuffer;
use pictura_filters::*;

fn mk<const C: usize>(w: u32, h: u32, px: &[[u8; C]]) -> PixelBuffer {
    let mut b = PixelBuffer::new(w, h, C as u8);
    let n = px.len();
    for (i, p) in px.iter().enumerate() {
        for (c, &v) in p.iter().enumerate() {
            b.data[c * n + i] = v;
        }
    }
    b
}

fn ramp(w: u32, h: u32) -> PixelBuffer {
    let px: Vec<[u8; 3]> = (0..(w * h) as usize)
        .map(|i| [i as u8, (i * 5) as u8, (i * 11) as u8])
        .collect();
    mk(w, h, &px)
}

#[test]
fn polar_remaps_and_kinds_differ() {
    let base = ramp(17, 13);
    let mut r2p = base.clone();
    let mut p2r = base.clone();
    polar_coordinates(&mut r2p, PolarKind::RectangularToPolar).unwrap();
    polar_coordinates(&mut p2r, PolarKind::PolarToRectangular).unwrap();
    assert_ne!(r2p.data, base.data);
    assert_ne!(p2r.data, base.data);
    assert_ne!(r2p.data, p2r.data);
}

#[test]
fn polar_horizontal_line_maps_to_constant_radius_ring() {
    let (w, h) = (65u32, 65u32);
    let mut px = vec![[0u8, 0, 0]; (w * h) as usize];
    let row = 34usize;
    let start = row * w as usize;
    for p in &mut px[start..start + w as usize] {
        *p = [255, 255, 255];
    }
    let mut b = mk(w, h, &px);
    polar_coordinates(&mut b, PolarKind::RectangularToPolar).unwrap();
    let c = 32i64;
    let at = |x: i64, y: i64| b.data[(y * w as i64 + x) as usize];
    let ring = at(c + 24, c);
    assert_eq!(ring, at(c - 24, c));
    assert_eq!(ring, at(c, c - 24));
    assert_eq!(ring, at(c, c + 24));
    assert!(
        ring > 100,
        "ring samples the bright source row (got {ring})"
    );
    assert_eq!(at(c, c), 0, "center maps to the dark top row");
}

#[test]
fn alpha_moves_with_the_warp_and_tiny_images_do_not_panic() {
    let px: Vec<[u8; 4]> = (0..12 * 12)
        .map(|i| [i as u8, (i * 3) as u8, (i * 7) as u8, (i * 11) as u8])
        .collect();
    let base = mk(12, 12, &px);
    let n = base.pixel_count();
    let alpha = base.data[3 * n..].to_vec();
    let mut p = base.clone();
    polar_coordinates(&mut p, PolarKind::RectangularToPolar).unwrap();
    assert_ne!(&p.data[3 * n..], &alpha[..], "polar resamples alpha too");
    let mut s = base.clone();
    shear(&mut s, &[(-1.0, -1.0), (1.0, 1.0)], ShearFill::WrapAround).unwrap();
    assert_ne!(&s.data[3 * n..], &alpha[..], "shear resamples alpha too");

    let one = mk(1, 1, &[[100, 150, 200, 255]]);
    for kind in [PolarKind::RectangularToPolar, PolarKind::PolarToRectangular] {
        let mut b = one.clone();
        polar_coordinates(&mut b, kind).unwrap();
    }
    let mut sh = one.clone();
    shear(
        &mut sh,
        &[(-1.0, 0.0), (1.0, 1.0)],
        ShearFill::RepeatEdgePixels,
    )
    .unwrap();

    for &(w, h) in &[(1u32, 8u32), (8, 1)] {
        let px: Vec<[u8; 3]> = (0..(w * h) as usize)
            .map(|i| [i as u8, 0, 255 - i as u8])
            .collect();
        let base = mk(w, h, &px);
        let mut a = base.clone();
        assert!(polar_coordinates(&mut a, PolarKind::PolarToRectangular).is_ok());
        let mut b = base.clone();
        assert!(shear(
            &mut b,
            &[(-1.0, 0.5), (0.0, -0.5), (1.0, 0.5)],
            ShearFill::WrapAround,
        )
        .is_ok());
    }
}
