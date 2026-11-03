//! Embedded smart-object source alpha: the stored merged transparency plane.

use super::smart_object::{embedded, payload_of};
use super::*;

/// A 2x2 RGB payload whose merged transparency plane is `alpha` (row-major).
pub(super) fn payload_with_alpha(colors: [[u8; 3]; 4], alpha: [u8; 4]) -> Vec<u8> {
    let mut d = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    let plane = 4;
    for (i, c) in colors.iter().enumerate() {
        d.composite.data[i] = c[0];
        d.composite.data[plane + i] = c[1];
        d.composite.data[2 * plane + i] = c[2];
    }
    d.channels = vec![Channel {
        id: -1,
        data: alpha.to_vec().into(),
    }];
    pictura_codec::write_psd(&d).expect("payload writes")
}

fn gray_payload_with_alpha(gray: [u8; 4], alpha: [u8; 4]) -> Vec<u8> {
    let mut d = Document::new(2, 2, ColorMode::Grayscale, BitDepth::Eight);
    d.composite.data.copy_from_slice(&gray);
    d.channels = vec![Channel {
        id: -1,
        data: alpha.to_vec().into(),
    }];
    pictura_codec::write_psd(&d).expect("payload writes")
}

fn payload_with_two_extras(first: [u8; 4], second: [u8; 4]) -> Vec<u8> {
    let mut d = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    for i in 0..4 {
        d.composite.data[i] = 10;
        d.composite.data[4 + i] = 20;
        d.composite.data[8 + i] = 30;
    }
    d.channels = vec![
        Channel {
            id: -1,
            data: first.to_vec().into(),
        },
        Channel {
            id: -2,
            data: second.to_vec().into(),
        },
    ];
    pictura_codec::write_psd(&d).expect("payload writes")
}

fn alpha_of(buf: &PixelBuffer) -> Vec<u8> {
    assert_eq!(buf.channels, 4, "the source is placed as RGBA");
    let plane = (buf.width * buf.height) as usize;
    buf.data[3 * plane..4 * plane].to_vec()
}

#[test]
fn source_preserves_merged_alpha() {
    let alpha = [255, 0, 255, 0];
    let so = embedded(payload_with_alpha([[10, 20, 30]; 4], alpha));
    let out = render_smart_source(&so, full(2, 2), full(2, 2)).expect("source renders");
    assert_eq!(alpha_of(&out), alpha);
    let plane = 4;
    assert_eq!(&out.data[..plane], &[10, 10, 10, 10]);
    assert_eq!(&out.data[plane..2 * plane], &[20, 20, 20, 20]);
    assert_eq!(&out.data[2 * plane..3 * plane], &[30, 30, 30, 30]);
}

#[test]
fn source_preserves_grayscale_merged_alpha() {
    let alpha = [0, 255, 0, 255];
    let so = embedded(gray_payload_with_alpha([128; 4], alpha));
    let out = render_smart_source(&so, full(2, 2), full(2, 2)).expect("source renders");
    assert_eq!(alpha_of(&out), alpha);
    let plane = 4;
    for c in 0..3 {
        assert_eq!(&out.data[c * plane..(c + 1) * plane], &[128; 4]);
    }
}

#[test]
fn source_without_extra_plane_is_opaque() {
    let so = embedded(payload_of([7, 8, 9]));
    let out = render_smart_source(&so, full(2, 2), full(2, 2)).expect("source renders");
    assert_eq!(alpha_of(&out), vec![255; 4]);
}

#[test]
fn source_uses_first_extra_plane_as_alpha() {
    let first = [0, 255, 0, 255];
    let second = [255, 0, 255, 0];
    let so = embedded(payload_with_two_extras(first, second));
    let out = render_smart_source(&so, full(2, 2), full(2, 2)).expect("source renders");
    assert_eq!(alpha_of(&out), first);
}
