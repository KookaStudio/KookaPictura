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

#[test]
fn multichannel_channel_owned_and_legacy_read() {
    let mut metadata = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    metadata.channels = (0..4)
        .map(|id| Channel {
            id,
            data: vec![0].into(),
        })
        .collect();
    let names = ["Cyan", "Magenta", "Yellow", "Black"].map(str::to_owned);
    crate::set_alpha_names(&mut metadata, &names);
    let named_resources = metadata.image_resources.to_vec();
    crate::set_alpha_names(&mut metadata, &[]);
    let display = [crate::ChannelDisplayInfo {
        mode: crate::SPOT_CHANNEL_MODE,
        opacity: 100,
        ..Default::default()
    }; 4];
    crate::set_display_info(&mut metadata, &display);
    let display_resources = metadata.image_resources.to_vec();

    for version in [1, 2] {
        for depth in [8, 16] {
            let bytes: Vec<_> = if depth == 8 {
                vec![0x12, 0x56, 0x9a, 0x0d]
            } else {
                vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0x0d, 0xef]
            };
            for (resources, expected_names, expected_display) in [
                (named_resources.as_slice(), names.as_slice(), &[][..]),
                (display_resources.as_slice(), &[][..], display.as_slice()),
            ] {
                let mut fixture = header_depth(version, 4, 1, 1, depth, 7);
                fixture.extend_from_slice(&0u32.to_be_bytes());
                fixture.extend_from_slice(&(resources.len() as u32).to_be_bytes());
                fixture.extend_from_slice(resources);
                fixture.extend_from_slice(&[0; 8][..if version == 1 { 4 } else { 8 }]);
                fixture.extend_from_slice(&COMPRESSION_RAW.to_be_bytes());
                fixture.extend_from_slice(&bytes);

                let doc = read_psd(&fixture).unwrap();
                assert!(doc.source.multichannel_plates());
                assert!(doc.layers.is_empty());
                assert_eq!(
                    doc.channels
                        .iter()
                        .map(|channel| channel.data[0])
                        .collect::<Vec<_>>(),
                    [0x12, 0x56, 0x9a, 0x0d]
                );
                assert_eq!(
                    doc.source.planes().as_ref().unwrap().samples.to_bytes(),
                    bytes
                );
                assert_eq!(crate::alpha_names(&doc), expected_names);
                if expected_display.is_empty() {
                    let info = crate::display_info(&doc);
                    assert_eq!(info.len(), 4);
                    assert!(info
                        .iter()
                        .all(|entry| entry.mode == crate::SPOT_CHANNEL_MODE));
                    assert_ne!(doc.composite.data, [255, 255, 255]);
                } else {
                    assert_eq!(crate::display_info(&doc), expected_display);
                }
                let reopened = read_psd(&write_psd(&doc).unwrap()).unwrap();
                assert_eq!(reopened.channels, doc.channels);
                assert_eq!(reopened.source.planes(), doc.source.planes());
            }
        }
    }
    for (depth, channels) in [(8, 2), (16, 1), (16, 3), (32, 3)] {
        assert!(matches!(
            read_psd(&flat_psd(depth, 7, channels, 1, 1, &[])),
            Err(PsdError::Unsupported(_))
        ));
    }
    for (bytes, preview) in [
        (flat_psd(8, 7, 1, 1, 1, &[&[40]]), [40, 40, 40]),
        (
            flat_psd(8, 7, 3, 1, 1, &[&[0], &[255], &[128]]),
            [255, 0, 127],
        ),
    ] {
        let doc = read_psd(&bytes).unwrap();
        assert!(!doc.source.multichannel_plates());
        assert!(doc.channels.is_empty());
        assert_eq!(doc.composite.data, preview);
        assert_eq!(&write_psd(&doc).unwrap()[24..26], &[0, 7]);
    }
}

#[test]
fn psd_header_zero_dimension_is_invalid() {
    for (width, height) in [(0, 1), (1, 0)] {
        assert!(matches!(
            read_psd(&header(1, 3, width, height, 3)),
            Err(PsdError::Invalid(_))
        ));
    }
}

#[test]
fn psd_header_channel_count_is_invalid() {
    for channels in [0, 57] {
        assert!(matches!(
            read_psd(&header(1, channels, 1, 1, 3)),
            Err(PsdError::Invalid(_))
        ));
    }
}

#[test]
fn named_multichannel_keeps_fallback_colors_after_channel_removal() {
    let mut doc = Document::new(1, 1, ColorMode::Rgb, BitDepth::Eight);
    doc.channels = vec![
        Channel {
            id: 0,
            data: vec![0].into(),
        },
        Channel {
            id: 1,
            data: vec![0].into(),
        },
    ];
    crate::set_alpha_names(&mut doc, &["Cyan".into(), "Yellow".into()]);
    crate::multichannel::complete_plate_display(&mut doc);
    assert_eq!(crate::multichannel_preview(&doc).data, [0, 255, 0]);
    doc.channels.remove(0);
    crate::remove_channel_display_info(&mut doc, 0);
    crate::set_alpha_names(&mut doc, &["Yellow".into()]);
    assert_eq!(crate::multichannel_preview(&doc).data, [255, 255, 0]);
    crate::set_alpha_names(&mut doc, &[]);
    assert_eq!(crate::multichannel_preview(&doc).data, [255, 255, 0]);
}
