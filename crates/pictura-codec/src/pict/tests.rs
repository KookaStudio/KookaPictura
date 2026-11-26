use super::packbits;
use super::{decode_pict, PictError};
use crate::probe::ImageBudget;

fn budget() -> ImageBudget {
    ImageBudget::default()
}

fn rect(out: &mut Vec<u8>, top: i16, left: i16, bottom: i16, right: i16) {
    for value in [top, left, bottom, right] {
        out.extend_from_slice(&value.to_be_bytes());
    }
}

fn v2_frame(width: u16, height: u16) -> Vec<u8> {
    let mut out = vec![0u8; 512];
    out.extend_from_slice(&512u16.to_be_bytes());
    rect(&mut out, 0, 0, height as i16, width as i16);
    out.extend_from_slice(&[0x00, 0x11, 0x02, 0xFF]);
    out
}

fn pixmap_tail(out: &mut Vec<u8>, pack_type: i16, pixel_size: i16, components: i16, size: i16) {
    out.extend_from_slice(&0i16.to_be_bytes());
    out.extend_from_slice(&pack_type.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&(72u32 << 16).to_be_bytes());
    out.extend_from_slice(&(72u32 << 16).to_be_bytes());
    out.extend_from_slice(&16i16.to_be_bytes());
    out.extend_from_slice(&pixel_size.to_be_bytes());
    out.extend_from_slice(&components.to_be_bytes());
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
}

fn direct32(
    out: &mut Vec<u8>,
    width: u16,
    height: u16,
    rgba: &[u8],
    pack_type: i16,
    alpha: bool,
    dst: (i16, i16),
) {
    let w = usize::from(width);
    let h = usize::from(height);
    let row_bytes = (w * 4) as u16;
    let pack_type = if pack_type == 4 && w * 4 < 8 {
        1
    } else {
        pack_type
    };
    out.extend_from_slice(&0x009Au16.to_be_bytes());
    out.extend_from_slice(&0x0000_00FFu32.to_be_bytes());
    out.extend_from_slice(&(row_bytes | 0x8000).to_be_bytes());
    rect(out, 0, 0, height as i16, width as i16);
    pixmap_tail(out, pack_type, 32, if alpha { 4 } else { 3 }, 8);
    rect(out, 0, 0, height as i16, width as i16);
    rect(
        out,
        dst.0,
        dst.1,
        dst.0 + height as i16,
        dst.1 + width as i16,
    );
    out.extend_from_slice(&0x0040u16.to_be_bytes());
    match pack_type {
        4 => {
            let components = if alpha { 4 } else { 3 };
            let mut planes = vec![0u8; components * w];
            for y in 0..h {
                for x in 0..w {
                    let at = (y * w + x) * 4;
                    if alpha {
                        planes[x] = rgba[at + 3];
                        planes[w + x] = rgba[at];
                        planes[2 * w + x] = rgba[at + 1];
                        planes[3 * w + x] = rgba[at + 2];
                    } else {
                        planes[x] = rgba[at];
                        planes[w + x] = rgba[at + 1];
                        planes[2 * w + x] = rgba[at + 2];
                    }
                }
                let packed = packbits::encode(&planes);
                out.push(packed.len() as u8);
                out.extend_from_slice(&packed);
            }
        }
        1 => {
            for y in 0..h {
                for x in 0..w {
                    let at = (y * w + x) * 4;
                    if alpha {
                        out.extend_from_slice(&[
                            rgba[at + 3],
                            rgba[at],
                            rgba[at + 1],
                            rgba[at + 2],
                        ]);
                    } else {
                        out.extend_from_slice(&[0, rgba[at], rgba[at + 1], rgba[at + 2]]);
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

fn direct16(out: &mut Vec<u8>, width: u16, height: u16, rgba: &[u8]) {
    let w = usize::from(width);
    let h = usize::from(height);
    let row_bytes = (w * 2) as u16;
    out.extend_from_slice(&0x009Au16.to_be_bytes());
    out.extend_from_slice(&0x0000_00FFu32.to_be_bytes());
    out.extend_from_slice(&(row_bytes | 0x8000).to_be_bytes());
    rect(out, 0, 0, height as i16, width as i16);
    pixmap_tail(out, 1, 16, 3, 5);
    rect(out, 0, 0, height as i16, width as i16);
    rect(out, 0, 0, height as i16, width as i16);
    out.extend_from_slice(&0x0040u16.to_be_bytes());
    for y in 0..h {
        for x in 0..w {
            let at = (y * w + x) * 4;
            let red = u16::from(rgba[at] >> 3);
            let green = u16::from(rgba[at + 1] >> 3);
            let blue = u16::from(rgba[at + 2] >> 3);
            let word = (red << 10) | (green << 5) | blue;
            out.extend_from_slice(&word.to_be_bytes());
        }
    }
}

fn indexed8(out: &mut Vec<u8>, width: u16, height: u16, indices: &[u8], palette: &[[u8; 3]]) {
    let w = usize::from(width);
    let h = usize::from(height);
    out.extend_from_slice(&0x0098u16.to_be_bytes());
    out.extend_from_slice(&(width | 0x8000).to_be_bytes());
    rect(out, 0, 0, height as i16, width as i16);
    pixmap_tail(out, 0, 8, 1, 8);
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0x8000u16.to_be_bytes());
    out.extend_from_slice(&((palette.len() as u16) - 1).to_be_bytes());
    for (index, color) in palette.iter().enumerate() {
        out.extend_from_slice(&(index as u16).to_be_bytes());
        for channel in color {
            out.extend_from_slice(&(u16::from(*channel) << 8).to_be_bytes());
        }
    }
    rect(out, 0, 0, height as i16, width as i16);
    rect(out, 0, 0, height as i16, width as i16);
    out.extend_from_slice(&0x0040u16.to_be_bytes());
    for y in 0..h {
        let row = &indices[y * w..(y + 1) * w];
        let packed = packbits::encode(row);
        out.push(packed.len() as u8);
        out.extend_from_slice(&packed);
    }
}

fn bitmap1(out: &mut Vec<u8>, width: u16, height: u16, bits: &[u8], packed: bool) {
    let row_bytes = usize::from(width).div_ceil(8) as u16;
    out.extend_from_slice(&(if packed { 0x0098u16 } else { 0x0090u16 }).to_be_bytes());
    out.extend_from_slice(&row_bytes.to_be_bytes());
    rect(out, 0, 0, height as i16, width as i16);
    rect(out, 0, 0, height as i16, width as i16);
    rect(out, 0, 0, height as i16, width as i16);
    out.extend_from_slice(&0x0040u16.to_be_bytes());
    if packed {
        for y in 0..usize::from(height) {
            let row = &bits[y * usize::from(row_bytes)..(y + 1) * usize::from(row_bytes)];
            let packed = packbits::encode(row);
            out.push(packed.len() as u8);
            out.extend_from_slice(&packed);
        }
    } else {
        out.extend_from_slice(bits);
    }
}

fn jpeg_record(out: &mut Vec<u8>, width: u16, height: u16, jpeg: &[u8]) {
    out.extend_from_slice(&0x8200u16.to_be_bytes());
    out.extend_from_slice(&((jpeg.len() + 154) as u32).to_be_bytes());
    let mut header = [0u8; 154];
    header[24..26].copy_from_slice(&0i16.to_be_bytes());
    header[26..28].copy_from_slice(&0i16.to_be_bytes());
    header[28..30].copy_from_slice(&(height as i16).to_be_bytes());
    header[30..32].copy_from_slice(&(width as i16).to_be_bytes());
    out.extend_from_slice(&header);
    out.extend_from_slice(jpeg);
}

fn jpeg_bytes(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)
        .encode(rgb, width, height, image::ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn finish(mut picture: Vec<u8>) -> Vec<u8> {
    if !picture.len().is_multiple_of(2) {
        picture.push(0);
    }
    picture.extend_from_slice(&0x00FFu16.to_be_bytes());
    picture
}

#[test]
fn pict_read_direct32() {
    let rgba = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 8, 7, 255];
    let mut picture = v2_frame(2, 2);
    direct32(&mut picture, 2, 2, &rgba, 4, false, (0, 0));
    let image = decode_pict(&finish(picture), budget()).expect("decode");
    assert_eq!((image.width, image.height), (2, 2));
    assert_eq!(
        image.rgba,
        vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 8, 7, 255]
    );
}

#[test]
fn pict_read_direct16() {
    let rgba = [255, 0, 0, 255, 0, 255, 0, 255];
    let mut picture = v2_frame(2, 1);
    direct16(&mut picture, 2, 1, &rgba);
    let image = decode_pict(&finish(picture), budget()).expect("decode");
    let expanded = |value: u8| {
        let five = value >> 3;
        (five << 3) | (five >> 2)
    };
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
            255
        ]
    );
}

#[test]
fn pict_read_indexed() {
    let palette = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]];
    let indices = [0u8, 1, 2, 3];
    let mut picture = v2_frame(2, 2);
    indexed8(&mut picture, 2, 2, &indices, &palette);
    let image = decode_pict(&finish(picture), budget()).expect("decode");
    assert_eq!(
        image.rgba,
        vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255]
    );
}

#[test]
fn pict_read_bitmap() {
    let bits = [0b1010_0000, 0b0101_0000];
    let mut picture = v2_frame(4, 2);
    bitmap1(&mut picture, 4, 2, &bits, false);
    let image = decode_pict(&finish(picture), budget()).expect("decode");
    assert_eq!(
        image.rgba,
        vec![
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255,
        ]
    );

    let wide = [0xFFu8; 8];
    let mut packed = v2_frame(64, 1);
    bitmap1(&mut packed, 64, 1, &wide, true);
    let image = decode_pict(&finish(packed), budget()).expect("packed decode");
    assert_eq!(image.rgba[..4], [0, 0, 0, 255]);
    assert_eq!(image.rgba[image.rgba.len() - 4..], [0, 0, 0, 255]);
}

#[test]
fn pict_read_v1() {
    let mut picture = vec![0u8; 512];
    picture.extend_from_slice(&512u16.to_be_bytes());
    rect(&mut picture, 0, 0, 2, 4);
    picture.extend_from_slice(&[0x11, 0x01]);
    picture.push(0x90);
    picture.extend_from_slice(&1u16.to_be_bytes());
    rect(&mut picture, 0, 0, 2, 4);
    rect(&mut picture, 0, 0, 2, 4);
    rect(&mut picture, 0, 0, 2, 4);
    picture.extend_from_slice(&0x0040u16.to_be_bytes());
    picture.extend_from_slice(&[0b1010_0000, 0b0101_0000]);
    picture.push(0xFF);
    let image = decode_pict(&picture, budget()).expect("decode");
    assert_eq!((image.width, image.height), (4, 2));
    assert_eq!(image.rgba[..4], [0, 0, 0, 255]);
    assert_eq!(image.rgba[4..8], [255, 255, 255, 255]);
}

#[test]
fn pict_read_multiple() {
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let mut picture = v2_frame(2, 1);
    direct32(&mut picture, 1, 1, &red, 4, false, (0, 0));
    direct32(&mut picture, 1, 1, &blue, 4, false, (0, 1));
    let image = decode_pict(&finish(picture), budget()).expect("decode");
    assert_eq!(image.rgba, vec![255, 0, 0, 255, 0, 0, 255, 255]);
}

#[test]
fn pict_read_jpeg() {
    let rgb = [0u8, 200, 0, 0, 200, 0, 0, 200, 0, 0, 200, 0];
    let jpeg = jpeg_bytes(2, 2, &rgb);
    let mut picture = v2_frame(2, 2);
    jpeg_record(&mut picture, 2, 2, &jpeg);
    let image = decode_pict(&finish(picture), budget()).expect("decode");
    assert_eq!((image.width, image.height), (2, 2));
    assert!(image.rgba[1] > 150, "green {}", image.rgba[1]);
}

#[test]
fn pict_read_alpha() {
    let rgba = [255, 0, 0, 0, 0, 255, 0, 255];
    let mut picture = v2_frame(2, 1);
    direct32(&mut picture, 2, 1, &rgba, 4, true, (0, 0));
    let image = decode_pict(&finish(picture), budget()).expect("decode");
    assert_eq!(image.rgba, vec![255, 0, 0, 0, 0, 255, 0, 255]);
}

#[test]
fn pict_read_packbits() {
    let rgba = [7, 7, 7, 255, 7, 7, 7, 255, 7, 7, 7, 255, 7, 7, 7, 255];
    let mut picture = v2_frame(4, 1);
    direct32(&mut picture, 4, 1, &rgba, 4, false, (0, 0));
    let image = decode_pict(&finish(picture), budget()).expect("decode");
    assert_eq!(image.rgba, rgba);
}

#[test]
fn pict_refuse_malformed() {
    let short = v2_frame(2, 2);
    assert!(matches!(
        decode_pict(&short[..520], budget()),
        Err(PictError::Malformed(_))
    ));

    let rgba = [1, 2, 3, 255];
    let mut picture = v2_frame(1, 1);
    direct32(&mut picture, 1, 1, &rgba, 4, false, (0, 0));
    let valid = finish(picture);
    let cut = valid.len() - 6;
    assert!(matches!(
        decode_pict(&valid[..cut], budget()),
        Err(PictError::Malformed(_))
    ));
}

#[test]
fn pict_refuse_vector() {
    let mut picture = v2_frame(2, 2);
    picture.extend_from_slice(&0x0034u16.to_be_bytes());
    rect(&mut picture, 0, 0, 2, 2);
    assert!(matches!(
        decode_pict(&finish(picture), budget()),
        Err(PictError::Unsupported(_))
    ));
}

#[test]
fn pict_refuse_unsupported() {
    let mut picture = v2_frame(1, 1);
    picture.extend_from_slice(&0x8201u16.to_be_bytes());
    picture.extend_from_slice(&0u32.to_be_bytes());
    assert!(matches!(
        decode_pict(&finish(picture), budget()),
        Err(PictError::Unsupported(_))
    ));

    let mut pattern = v2_frame(1, 1);
    pattern.extend_from_slice(&0x0012u16.to_be_bytes());
    assert!(matches!(
        decode_pict(&finish(pattern), budget()),
        Err(PictError::Unsupported(_))
    ));

    let mut bad_pixel = v2_frame(1, 1);
    bad_pixel.extend_from_slice(&0x009Au16.to_be_bytes());
    bad_pixel.extend_from_slice(&0x0000_00FFu32.to_be_bytes());
    bad_pixel.extend_from_slice(&0x8004u16.to_be_bytes());
    rect(&mut bad_pixel, 0, 0, 1, 1);
    pixmap_tail(&mut bad_pixel, 1, 3, 3, 8);
    assert!(matches!(
        decode_pict(&finish(bad_pixel), budget()),
        Err(PictError::Unsupported(_))
    ));

    let mut bad_pack = v2_frame(1, 1);
    direct32(&mut bad_pack, 1, 1, &[1, 2, 3, 255], 4, false, (0, 0));
    let pack_type_at = 512 + 2 + 8 + 4 + 2 + 4 + 2 + 8 + 2;
    bad_pack[pack_type_at..pack_type_at + 2].copy_from_slice(&9i16.to_be_bytes());
    assert!(matches!(
        decode_pict(&finish(bad_pack), budget()),
        Err(PictError::Unsupported(_))
    ));
}

#[test]
fn pict_refuse_oversized() {
    let picture = v2_frame(30_001, 1);
    assert!(matches!(
        decode_pict(&picture, budget()),
        Err(PictError::DimensionLimit { .. })
    ));
}

#[test]
fn pict_refuse_jpeg() {
    let mut picture = v2_frame(1, 1);
    picture.extend_from_slice(&0x8200u16.to_be_bytes());
    picture.extend_from_slice(&158u32.to_be_bytes());
    picture.extend_from_slice(&[0u8; 158]);
    assert!(matches!(
        decode_pict(&finish(picture), budget()),
        Err(PictError::Jpeg)
    ));
}

#[test]
fn pict_read_committed_fixture() {
    let bytes = include_bytes!("../../tests/fixtures/pict_rgb.pct");
    let image = decode_pict(bytes, budget()).expect("decode fixture");
    assert_eq!((image.width, image.height), (4, 3));
    let jpeg = include_bytes!("../../tests/fixtures/pict_jpeg.pct");
    let image = decode_pict(jpeg, budget()).expect("decode JPEG fixture");
    assert_eq!((image.width, image.height), (4, 3));
}
