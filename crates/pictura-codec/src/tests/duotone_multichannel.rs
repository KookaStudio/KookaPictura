//! Duotone and Multichannel read/write-back tests, split from `color_modes.rs`.

use super::color_modes::{flat_psd, flat_psd_with_data};
use super::*;

#[test]
fn duotone_opens_as_rgb_and_preserves_color_mode_data() {
    let data = b"Duo!".to_vec();
    let p = flat_psd_with_data(8, 8, 1, 2, 1, &data, &[&[10, 20]]);
    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.mode, ColorMode::Rgb);
    assert_eq!(doc.depth, BitDepth::Eight);
    assert_eq!(doc.source.mode(), Some(ColorMode::Duotone));
    assert_eq!(doc.color_mode_data, data);
    assert_eq!(doc.composite.channels, 3);
    assert_eq!(&doc.composite.data[0..2], &[10, 20]);
    assert_eq!(&doc.composite.data[2..4], &[10, 20]);
    assert_eq!(&doc.composite.data[4..6], &[10, 20]);

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        8,
        "output header color mode is Duotone"
    );
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        1,
        "one color channel"
    );
    let cmd_len = u32::from_be_bytes(out[26..30].try_into().unwrap()) as usize;
    assert_eq!(
        &out[30..30 + cmd_len],
        &data[..],
        "color_mode_data preserved"
    );

    let back = read_psd(&out).unwrap();
    assert_eq!(back.source.mode(), Some(ColorMode::Duotone));
    assert_eq!(back.color_mode_data, data);
    assert_eq!(
        back.source.planes().as_ref().unwrap().samples.to_bytes(),
        vec![10, 20],
        "the plane bytes match the retained source"
    );
    assert_eq!(back.composite, doc.composite);
}

#[test]
fn multichannel_one_channel_opens_as_gray_rgb() {
    let p = flat_psd(8, 7, 1, 2, 1, &[&[40, 80]]);
    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.mode, ColorMode::Rgb);
    assert_eq!(doc.source.mode(), Some(ColorMode::Multichannel));
    assert_eq!(doc.composite.channels, 3);
    assert_eq!(&doc.composite.data[0..2], &[40, 80]);
    assert_eq!(&doc.composite.data[2..4], &[40, 80]);
    assert_eq!(&doc.composite.data[4..6], &[40, 80]);

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        7,
        "output header color mode is Multichannel"
    );
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        1,
        "one channel"
    );
    let back = read_psd(&out).unwrap();
    assert_eq!(back.source.mode(), Some(ColorMode::Multichannel));
    assert_eq!(
        back.source.planes().as_ref().unwrap().samples.to_bytes(),
        vec![40, 80],
        "the plate bytes match the retained source"
    );
    assert_eq!(back.composite, doc.composite);
}

#[test]
fn multichannel_three_channels_maps_cmy_to_rgb() {
    // Known CMY plates: r = 255 - c, g = 255 - m, b = 255 - y per pixel.
    let p = flat_psd(8, 7, 3, 2, 1, &[&[0, 255], &[255, 0], &[128, 128]]);
    let doc = read_psd(&p).unwrap();
    assert_eq!(doc.mode, ColorMode::Rgb);
    assert_eq!(doc.source.mode(), Some(ColorMode::Multichannel));
    assert_eq!(
        doc.composite.data,
        vec![255, 0, 0, 255, 127, 127],
        "pixel0=(255,0,127) pixel1=(0,255,127)"
    );

    let out = write_psd(&doc).unwrap();
    assert_eq!(
        u16::from_be_bytes(out[24..26].try_into().unwrap()),
        7,
        "output header color mode is Multichannel"
    );
    assert_eq!(
        u16::from_be_bytes(out[12..14].try_into().unwrap()),
        3,
        "three channels"
    );
    let back = read_psd(&out).unwrap();
    assert_eq!(
        back.source.planes().as_ref().unwrap().samples.to_bytes(),
        vec![0, 255, 255, 0, 128, 128],
        "the three plate bytes match the retained source"
    );
    assert_eq!(back.composite, doc.composite);
}

#[test]
fn multichannel_two_channels_is_unsupported() {
    let p = flat_psd(8, 7, 2, 1, 1, &[&[0], &[0]]);
    assert!(
        matches!(read_psd(&p), Err(PsdError::Unsupported(_))),
        "Multichannel N=2 stays Unsupported"
    );
}

#[test]
fn edited_duotone_and_multichannel_write_rgb() {
    for (mode, planes) in [
        (8u16, vec![vec![10u8]]),
        (7, vec![vec![40u8]]),
        (7, vec![vec![0u8], vec![255], vec![128]]),
    ] {
        let refs: Vec<&[u8]> = planes.iter().map(|p| p.as_slice()).collect();
        let channels = planes.len() as u16;
        let p = flat_psd(8, mode, channels, 1, 1, &refs);
        let mut doc = read_psd(&p).unwrap();
        doc.composite
            .edit_data(|data| {
                data[0] ^= 1;
            })
            .expect("pixel edit preserves geometry");

        let out = write_psd(&doc).unwrap();
        assert_eq!(
            u16::from_be_bytes(out[24..26].try_into().unwrap()),
            3,
            "edited mode {mode} N={channels} writes RGB"
        );
        let back = read_psd(&out).unwrap();
        assert_eq!(back.mode, ColorMode::Rgb);
        assert_eq!(back.source.mode(), None);
        assert_eq!(back.composite, doc.composite);
    }
}
