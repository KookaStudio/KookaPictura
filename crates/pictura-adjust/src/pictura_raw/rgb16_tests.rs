//! `render_pictura_raw_rgb16` (`codec/pictura-raw`, #400).

use super::*;
use pictura_core::{CrsSettings, PicturaRawSettings};

fn lcg(seed: u64) -> impl FnMut() -> u64 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state >> 33
    }
}

#[test]
fn rgb16_default_settings_are_noop() {
    let mut next = lcg(11);
    let rgb: Vec<u16> = (0..9 * 7 * 3).map(|_| next() as u16).collect();
    let out = render_pictura_raw_rgb16(9, 7, &rgb, &PicturaRawSettings::default()).unwrap();
    assert_eq!(out, rgb);
}

#[test]
fn rgb16_matches_8bit_on_8bit_values() {
    let (w, h) = (12usize, 8usize);
    let mut next = lcg(5);
    let interleaved: Vec<u8> = (0..w * h * 3).map(|_| next() as u8).collect();
    let n = w * h;
    let mut planar = vec![0u8; n * 3];
    for i in 0..n {
        for c in 0..3 {
            planar[c * n + i] = interleaved[i * 3 + c];
        }
    }
    let input = PixelBuffer {
        width: w as u32,
        height: h as u32,
        channels: 3,
        data: planar.into(),
    };
    let settings = PicturaRawSettings {
        basic: CrsSettings {
            exposure: Some(0.5),
            contrast: Some(20.0),
            saturation: Some(-30.0),
            ..CrsSettings::default()
        },
        ..PicturaRawSettings::default()
    };
    let eight = render_pictura_raw(&input, &settings).unwrap();
    let wide: Vec<u16> = interleaved.iter().map(|&v| v as u16 * 257).collect();
    let sixteen = render_pictura_raw_rgb16(w, h, &wide, &settings).unwrap();
    for i in 0..n {
        for c in 0..3 {
            let a = eight.data[c * n + i] as i32;
            let b = (sixteen[i * 3 + c] as f64 / 257.0).round() as i32;
            assert!(
                (a - b).abs() <= 1,
                "pixel {i} channel {c}: 8-bit {a}, 16-bit {b}"
            );
        }
    }
}

#[test]
fn rgb16_refuses_malformed_buffers() {
    let settings = PicturaRawSettings::default();
    assert!(render_pictura_raw_rgb16(4, 4, &[0; 47], &settings).is_err());
    assert!(render_pictura_raw_rgb16(0, 0, &[], &settings).is_err());
}
