//! Spec scenarios from the replaced `distort/undulate.rs` unit tests, run through
//! `apply` against the ported photorust engine.

use super::shims::*;
use pictura_core::PixelBuffer;
use pictura_filters::*;

const WL: (f64, f64) = (10.0, 40.0);
const AMP: (f64, f64) = (5.0, 15.0);
const SC: (f64, f64) = (100.0, 50.0);
const NWL: (f64, f64) = (10.0, 11.0);
const NAMP: (f64, f64) = (20.0, 21.0);
const NSC: (f64, f64) = (100.0, 1.0);

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

fn wv(base: &PixelBuffer, g: u32, kind: WaveType, seed: u64, repeat: bool) -> PixelBuffer {
    let mut out = base.clone();
    wave(&mut out, g, WL, AMP, kind, SC, seed, repeat).unwrap();
    out
}

fn narrow(base: &PixelBuffer, g: u32, seed: u64) -> PixelBuffer {
    let mut out = base.clone();
    wave(&mut out, g, NWL, NAMP, WaveType::Sine, NSC, seed, true).unwrap();
    out
}

#[test]
fn ripple_noop_displaces_and_sizes_differ() {
    let base = ramp(24, 24);
    let mut noop = base.clone();
    ripple(&mut noop, 0.0, RippleSize::Medium).unwrap();
    assert_eq!(noop.data, base.data);
    let (mut small, mut large) = (base.clone(), base.clone());
    ripple(&mut small, 100.0, RippleSize::Small).unwrap();
    ripple(&mut large, 100.0, RippleSize::Large).unwrap();
    assert_ne!(small.data, base.data);
    assert_ne!(small.data, large.data);
}

#[test]
fn wave_is_seed_deterministic_and_moves_pixels() {
    let base = ramp(24, 24);
    let a = wv(&base, 5, WaveType::Sine, 7, true);
    assert_eq!(a.data, wv(&base, 5, WaveType::Sine, 7, true).data);
    assert_ne!(a.data, wv(&base, 5, WaveType::Sine, 8, true).data);
    assert_eq!(narrow(&base, 1, 42).data, narrow(&base, 1, 42).data);
    assert_ne!(narrow(&base, 1, 42).data, base.data);
}

#[test]
fn wave_edge_modes_types_and_validation() {
    let base = ramp(32, 8);
    let a = wv(&base, 3, WaveType::Sine, 1, true);
    assert_ne!(a.data, wv(&base, 3, WaveType::Sine, 1, false).data);
    let sine = wv(&base, 2, WaveType::Sine, 3, true);
    assert_ne!(sine.data, wv(&base, 2, WaveType::Triangle, 3, true).data);
    assert_ne!(sine.data, wv(&base, 2, WaveType::Square, 3, true).data);

    let reject = |g: u32, wl: (f64, f64), amp: (f64, f64), sc: (f64, f64)| {
        let mut b = base.clone();
        assert!(wave(&mut b, g, wl, amp, WaveType::Sine, sc, 1, true).is_err());
        assert_eq!(b.data, base.data);
    };
    let (wl, amp, ok) = ((10.0, 20.0), (5.0, 15.0), (100.0, 100.0));
    reject(0, wl, amp, ok);
    reject(1000, wl, amp, ok);
    reject(5, (20.0, 20.5), amp, ok);
    reject(5, wl, (20.0, 20.5), ok);
    reject(5, wl, amp, (0.0, 100.0));
    reject(5, wl, amp, (100.0, 101.0));
}

#[test]
fn alpha_moves_with_the_warp_and_tiny_images_do_not_panic() {
    let px: Vec<[u8; 4]> = (0..24 * 24)
        .map(|i| [i as u8, (i * 3) as u8, (i * 7) as u8, (i * 11) as u8])
        .collect();
    let base = buf4(24, 24, &px);
    let n = base.pixel_count();
    let alpha = base.data[3 * n..].to_vec();
    let mut r = base.clone();
    ripple(&mut r, 400.0, RippleSize::Small).unwrap();
    assert_ne!(&r.data[3 * n..], &alpha[..], "ripple resamples alpha too");
    let w = wv(&base, 4, WaveType::Square, 9, false);
    assert_ne!(&w.data[3 * n..], &alpha[..], "wave resamples alpha too");

    let one = buf4(1, 1, &[[100, 150, 200, 255]]);
    let mut r1 = one.clone();
    ripple(&mut r1, 500.0, RippleSize::Large).unwrap();
    assert_eq!(r1.data, one.data);
    assert_eq!(wv(&one, 999, WaveType::Triangle, 1, false).data, one.data);

    for &(w, h) in &[(1u32, 8u32), (8, 1)] {
        let px: Vec<[u8; 3]> = (0..(w * h) as usize)
            .map(|i| [i as u8, 0, 255 - i as u8])
            .collect();
        let base = buf3(w, h, &px);
        let mut a = base.clone();
        assert!(ripple(&mut a, 250.0, RippleSize::Medium).is_ok());
        wv(&base, 2, WaveType::Sine, 3, true);
    }
}
