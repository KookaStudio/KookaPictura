use super::{pict, Raster, SaveOptions};
use crate::error::PsdError;
use crate::pict::decode_pict;
use crate::probe::ImageBudget;
use pictura_core::Document;

fn raster(rgba: &[u8], width: u32, height: u32) -> Raster {
    Raster::from_document(&Document::from_rgba("pict", width, height, rgba)).expect("raster")
}

fn budget() -> ImageBudget {
    ImageBudget::default()
}

fn expanded(value: u8) -> u8 {
    let five = value >> 3;
    (five << 3) | (five >> 2)
}

#[test]
fn pict_write_32() {
    let rgba = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 8, 7, 255];
    let bytes = pict::write(&raster(&rgba, 2, 2), &SaveOptions::default(), None).expect("write");
    let image = decode_pict(&bytes, budget()).expect("decode");
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(image.rgba, rgba);
}

#[test]
fn pict_write_16() {
    let rgba = [255, 0, 0, 255, 0, 255, 0, 255];
    let options = SaveOptions {
        pict_resolution: 16,
        ..SaveOptions::default()
    };
    let bytes = pict::write(&raster(&rgba, 2, 1), &options, None).expect("write");
    let image = decode_pict(&bytes, budget()).expect("decode");
    assert_eq!(
        image.rgba,
        vec![
            expanded(255),
            expanded(0),
            expanded(0),
            255,
            expanded(0),
            expanded(255),
            expanded(0),
            255,
        ]
    );
}

#[test]
fn pict_write_alpha() {
    let rgba = [255, 0, 0, 0, 0, 255, 0, 255];
    let bytes = pict::write(&raster(&rgba, 2, 1), &SaveOptions::default(), None).expect("write");
    let image = decode_pict(&bytes, budget()).expect("decode");
    assert_eq!(image.rgba, rgba);
}

#[test]
fn pict_write_deterministic() {
    let rgba = [1u8, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255];
    let r = raster(&rgba, 2, 2);
    let options = SaveOptions::default();
    let first = pict::write(&r, &options, None).expect("first");
    let second = pict::write(&r, &options, None).expect("second");
    assert_eq!(first, second);
}

#[test]
fn pict_write_jpeg() {
    let mut rgba = Vec::new();
    for y in 0..16u32 {
        for x in 0..16u32 {
            rgba.extend_from_slice(&[(x * 16) as u8, (y * 16) as u8, ((x + y) * 8) as u8, 255]);
        }
    }
    let r = raster(&rgba, 16, 16);
    let mut sizes = Vec::new();
    for compression in 1..=4u8 {
        let options = SaveOptions {
            pict_compression: compression,
            ..SaveOptions::default()
        };
        let bytes = pict::write(&r, &options, None).expect("write");
        assert_eq!(bytes.get(564..566), Some(&[0x82, 0x00][..]));
        let image = decode_pict(&bytes, budget()).expect("decode");
        assert_eq!((image.width, image.height), (16, 16));
        assert!(image.rgba[0].abs_diff(rgba[0]) < 48);
        sizes.push(bytes.len());
    }
    assert!(sizes[0] <= sizes[3], "JPEG sizes {sizes:?}");
}

#[test]
fn pict_write_limits() {
    let wide = raster(&vec![0u8; 8192 * 4], 8192, 1);
    assert!(matches!(
        pict::write(&wide, &SaveOptions::default(), None),
        Err(PsdError::Unsupported(_))
    ));

    let wide16 = raster(&vec![0u8; 16_384 * 4], 16_384, 1);
    let options = SaveOptions {
        pict_resolution: 16,
        ..SaveOptions::default()
    };
    assert!(matches!(
        pict::write(&wide16, &options, None),
        Err(PsdError::Unsupported(_))
    ));

    let tall = raster(&[0u8; 4], 1, 32_768);
    assert!(matches!(
        pict::write(&tall, &SaveOptions::default(), None),
        Err(PsdError::Unsupported(_))
    ));
}
