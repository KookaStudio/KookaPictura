//! Spec scenarios from the replaced `distort/radial.rs` unit tests, run through
//! `apply` against the ported photorust engine.

use super::shims::*;
use pictura_core::PixelBuffer;
use pictura_filters::*;

fn buf<const C: usize>(w: u32, h: u32, px: &[[u8; C]]) -> PixelBuffer {
    let n = (w * h) as usize;
    assert_eq!(px.len(), n);
    let mut data = vec![0u8; n * C];
    for (i, p) in px.iter().enumerate() {
        for (c, &v) in p.iter().enumerate() {
            data[c * n + i] = v;
        }
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: C as u8,
        data: data.into(),
    }
}

use SpherizeMode::{HorizontalOnly, Normal, VerticalOnly};
const MODES: [SpherizeMode; 3] = [Normal, HorizontalOnly, VerticalOnly];

#[test]
fn warps_change_patterns_and_zero_is_bit_exact_noop() {
    let px: Vec<[u8; 3]> = (0..81)
        .map(|i| {
            let (x, y) = ((i % 9) as u8, (i / 9) as u8);
            [
                x.wrapping_mul(20),
                y.wrapping_mul(30),
                x.wrapping_add(y).wrapping_mul(10),
            ]
        })
        .collect();
    let base = buf(9, 9, &px);
    let solid = buf(9, 9, &vec![[40u8, 90, 160]; 81]);
    let check_solid = |b: &PixelBuffer| {
        for (g, w) in b.data.iter().zip(&solid.data) {
            assert!((*g as i32 - *w as i32).abs() <= 1, "solid drifted");
        }
    };

    let mut t0 = base.clone();
    twirl(&mut t0, 0.0).unwrap();
    assert_eq!(t0.data, base.data);
    let mut p0 = base.clone();
    pinch(&mut p0, 0.0).unwrap();
    assert_eq!(p0.data, base.data);
    for m in MODES {
        let mut s0 = base.clone();
        spherize(&mut s0, 0.0, m).unwrap();
        assert_eq!(s0.data, base.data, "{m:?} no-op at 0");
    }

    let mut p = base.clone();
    pinch(&mut p, 60.0).unwrap();
    assert_ne!(p.data, base.data, "pinch changes pattern");
    let mut n = base.clone();
    spherize(&mut n, 60.0, SpherizeMode::Normal).unwrap();
    assert_ne!(n.data, base.data, "spherize changes pattern");
    let mut h = base.clone();
    spherize(&mut h, 70.0, SpherizeMode::HorizontalOnly).unwrap();
    assert_ne!(n.data, h.data, "HorizontalOnly differs from Normal");

    let mut ps = solid.clone();
    pinch(&mut ps, -80.0).unwrap();
    check_solid(&ps);
    for m in MODES {
        let mut ss = solid.clone();
        spherize(&mut ss, 80.0, m).unwrap();
        check_solid(&ss);
    }
}

#[test]
fn twirl_rotates_pattern_and_pins_center() {
    let mut px = vec![[0u8; 3]; 81];
    px[2 * 9 + 2] = [255; 3];
    px[4 * 9 + 4] = [200; 3];
    let base = buf(9, 9, &px);
    let mut t = base.clone();
    twirl(&mut t, 90.0).unwrap();
    assert_ne!(t.data, base.data, "off-center pixel moves");
    let (n, i) = (t.pixel_count(), 4 * 9 + 4);
    assert_eq!([t.data[i], t.data[n + i], t.data[2 * n + i]], [200; 3]);
}

#[test]
fn params_are_validated() {
    let mut b = PixelBuffer::new(4, 4, 3);
    assert!(twirl(&mut b, f64::NAN).is_err());
    assert!(twirl(&mut b, 1000.0).is_err());
    assert!(pinch(&mut b, f64::INFINITY).is_err());
    assert!(pinch(&mut b, 101.0).is_err());
    assert!(spherize(&mut b, f64::NAN, SpherizeMode::Normal).is_err());
    assert!(spherize(&mut b, 100.1, SpherizeMode::Normal).is_err());
    assert!(twirl(&mut b, 999.0).is_ok());
    assert!(pinch(&mut b, -100.0).is_ok());
    assert!(spherize(&mut b, 100.0, SpherizeMode::VerticalOnly).is_ok());
}

#[test]
fn every_radial_warp_resamples_alpha_too() {
    let alpha: Vec<u8> = (0..49).map(|i| (i * 5) as u8).collect();
    let px: Vec<[u8; 4]> = alpha
        .iter()
        .enumerate()
        .map(|(i, &a)| [(i * 3) as u8, (i * 4) as u8, (i * 5) as u8, a])
        .collect();
    let base = buf(7, 7, &px);
    let check = |b: &PixelBuffer| {
        let n = b.pixel_count();
        assert_ne!(
            &b.data[3 * n..4 * n],
            &alpha[..],
            "warp left alpha untouched"
        );
    };
    let mut t = base.clone();
    twirl(&mut t, 45.0).unwrap();
    check(&t);
    let mut p = base.clone();
    pinch(&mut p, -40.0).unwrap();
    check(&p);
    for m in MODES {
        let mut s = base.clone();
        spherize(&mut s, 40.0, m).unwrap();
        check(&s);
    }
}

#[test]
fn tiny_and_thin_images_do_not_panic() {
    for (w, h) in [(1u32, 1u32), (1, 7), (7, 1)] {
        let n = (w * h) as usize;
        let px: Vec<[u8; 3]> = (0..n)
            .map(|i| [(i * 3) as u8, (i * 5) as u8, (i * 7) as u8])
            .collect();
        let mut b = buf(w, h, &px);
        twirl(&mut b, 33.0).unwrap();
        pinch(&mut b, 80.0).unwrap();
        for m in MODES {
            spherize(&mut b, 80.0, m).unwrap();
        }
    }
}
