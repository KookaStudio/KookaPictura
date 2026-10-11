//! Spec scenarios from the replaced `distort/ripples.rs` unit tests, run through
//! `apply` against the ported photorust engine.

use super::shims::*;
use pictura_core::PixelBuffer;
use pictura_filters::*;

fn buf3(w: u32, h: u32, px: &[[u8; 3]]) -> PixelBuffer {
    let mut b = PixelBuffer::new(w, h, 3);
    let n = px.len();
    for (i, p) in px.iter().enumerate() {
        for (c, &v) in p.iter().enumerate() {
            b.data[c * n + i] = v;
        }
    }
    b
}

fn buf4(w: u32, h: u32, px: &[[u8; 4]]) -> PixelBuffer {
    let mut b = PixelBuffer::new(w, h, 4);
    let n = px.len();
    for (i, p) in px.iter().enumerate() {
        for (c, &v) in p.iter().enumerate() {
            b.data[c * n + i] = v;
        }
    }
    b
}

/// Deterministic high-frequency pattern so a warp cannot be invariant.
fn ramp(w: u32, h: u32) -> PixelBuffer {
    let px: Vec<[u8; 3]> = (0..(w * h) as usize)
        .map(|i| [i as u8, (i * 5) as u8, (i * 11) as u8])
        .collect();
    buf3(w, h, &px)
}

#[test]
fn zigzag_zero_is_noop_and_nonzero_displaces() {
    let base = ramp(24, 24);
    let mut noop = base.clone();
    zigzag(&mut noop, 0.0, 5, ZigZagStyle::AroundCenter).unwrap();
    assert_eq!(noop.data, base.data, "amount 0 must be bit-exact");

    for style in [
        ZigZagStyle::AroundCenter,
        ZigZagStyle::OutFromCenter,
        ZigZagStyle::PondRipples,
    ] {
        let mut out = base.clone();
        zigzag(&mut out, 80.0, 5, style).unwrap();
        assert_ne!(out.data, base.data, "{style:?} must displace");
    }
}

#[test]
fn zigzag_styles_differ() {
    let base = ramp(24, 24);
    let run = |style| {
        let mut b = base.clone();
        zigzag(&mut b, 80.0, 5, style).unwrap();
        b.data
    };
    let around = run(ZigZagStyle::AroundCenter);
    let out = run(ZigZagStyle::OutFromCenter);
    let pond = run(ZigZagStyle::PondRipples);
    assert_ne!(around, out);
    assert_ne!(around, pond);
    assert_ne!(out, pond);
}

#[test]
fn zigzag_rejects_out_of_range_amount_and_ridges() {
    let base = ramp(8, 8);
    for bad in [-101.0, 101.0, f64::NAN, f64::INFINITY] {
        let mut b = base.clone();
        assert!(zigzag(&mut b, bad, 5, ZigZagStyle::AroundCenter).is_err());
    }
    let mut b = base.clone();
    assert!(zigzag(&mut b, 50.0, 21, ZigZagStyle::AroundCenter).is_err());
}

#[test]
fn alpha_moves_with_the_warp_and_tiny_images_do_not_panic() {
    let px: Vec<[u8; 4]> = (0..24 * 24)
        .map(|i| [i as u8, (i * 3) as u8, (i * 7) as u8, (i * 11) as u8])
        .collect();
    let base = buf4(24, 24, &px);
    let n = base.pixel_count();
    let alpha = base.data[3 * n..].to_vec();

    let mut z = base.clone();
    zigzag(&mut z, 90.0, 7, ZigZagStyle::PondRipples).unwrap();
    assert_ne!(&z.data[3 * n..], &alpha[..], "zigzag resamples alpha too");
    let mut o = base.clone();
    ocean_ripple(&mut o, 9, 15, 3).unwrap();
    assert_ne!(
        &o.data[3 * n..],
        &alpha[..],
        "ocean ripple resamples alpha too"
    );

    for &(w, h) in &[(1u32, 1u32), (1, 8), (8, 1)] {
        let px: Vec<[u8; 3]> = (0..(w * h) as usize)
            .map(|i| [i as u8, (i * 2) as u8, 255 - i as u8])
            .collect();
        let base = buf3(w, h, &px);
        let mut z = base.clone();
        assert!(zigzag(&mut z, 100.0, 20, ZigZagStyle::AroundCenter).is_ok());
        let mut o = base.clone();
        assert!(ocean_ripple(&mut o, 15, 20, 5).is_ok());
    }
}
