//! The PICT Save As writer: extended version 2 with one `DirectBitsRect`, at
//! 16 or 32 bits, PackBits or QuickTime JPEG.

use super::{jpeg_quality_percent, over_white, JpegEncode, Raster, SaveOptions};
use crate::error::PsdError;
use crate::pict::packbits;

const VERSION_OP: u16 = 0x0011;
const VERSION_2: u16 = 0x02FF;
const HEADER_OP: u16 = 0x0C00;
const CLIP_OP: u16 = 0x0001;
const DIRECT_BITS_RECT: u16 = 0x009A;
const COMPRESSED_QUICKTIME: u16 = 0x8200;
const END_OF_PICTURE: u16 = 0x00FF;
const SRC_COPY: u16 = 0x0040;
const PIXEL_TYPE_DIRECT: i16 = 16;
const RESOLUTION_72: u32 = 72 << 16;
const MAX_DIMENSION: u32 = 32_767;

pub(super) fn write(
    raster: &Raster,
    options: &SaveOptions,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    let width = raster.width;
    let height = raster.height;
    if width == 0 || height == 0 {
        return Err(PsdError::Invalid("zero image size".into()));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(PsdError::Unsupported("PICT dimensions over 32767".into()));
    }
    let resolution = if options.pict_resolution == 16 {
        16
    } else {
        32
    };
    let compression = options.pict_compression.min(4);
    let mut out = Vec::new();
    write_header(&mut out, width, height);
    if compression > 0 {
        let quality = [0u8, 3, 6, 9, 12][usize::from(compression)];
        let bytes = encode_jpeg(raster, quality, jpeg)?;
        write_jpeg_record(&mut out, width, height, &bytes)?;
    } else if resolution == 16 {
        write_direct16(&mut out, raster)?;
    } else {
        write_direct32(&mut out, raster)?;
    }
    if !out.len().is_multiple_of(2) {
        out.push(0);
    }
    out.extend_from_slice(&END_OF_PICTURE.to_be_bytes());
    Ok(out)
}

fn write_header(out: &mut Vec<u8>, width: u32, height: u32) {
    out.resize(512, 0);
    out.extend_from_slice(&512u16.to_be_bytes());
    write_rect(out, 0, 0, height as i16, width as i16);
    out.extend_from_slice(&VERSION_OP.to_be_bytes());
    out.extend_from_slice(&VERSION_2.to_be_bytes());
    out.extend_from_slice(&HEADER_OP.to_be_bytes());
    out.extend_from_slice(&0xFFFEu16.to_be_bytes());
    out.extend_from_slice(&0x0000u16.to_be_bytes());
    out.extend_from_slice(&RESOLUTION_72.to_be_bytes());
    out.extend_from_slice(&RESOLUTION_72.to_be_bytes());
    write_rect(out, 0, 0, height as i16, width as i16);
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&CLIP_OP.to_be_bytes());
    out.extend_from_slice(&0x000Au16.to_be_bytes());
    write_rect(out, 0, 0, height as i16, width as i16);
}

fn write_rect(out: &mut Vec<u8>, top: i16, left: i16, bottom: i16, right: i16) {
    out.extend_from_slice(&top.to_be_bytes());
    out.extend_from_slice(&left.to_be_bytes());
    out.extend_from_slice(&bottom.to_be_bytes());
    out.extend_from_slice(&right.to_be_bytes());
}

fn write_pixmap(
    out: &mut Vec<u8>,
    pack_type: i16,
    pixel_size: i16,
    components: i16,
    component_size: i16,
) {
    out.extend_from_slice(&0i16.to_be_bytes());
    out.extend_from_slice(&pack_type.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&RESOLUTION_72.to_be_bytes());
    out.extend_from_slice(&RESOLUTION_72.to_be_bytes());
    out.extend_from_slice(&PIXEL_TYPE_DIRECT.to_be_bytes());
    out.extend_from_slice(&pixel_size.to_be_bytes());
    out.extend_from_slice(&components.to_be_bytes());
    out.extend_from_slice(&component_size.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes());
}

fn write_direct32(out: &mut Vec<u8>, raster: &Raster) -> Result<(), PsdError> {
    let width = raster.width as usize;
    let height = raster.height as usize;
    let row_bytes = width
        .checked_mul(4)
        .ok_or_else(|| PsdError::Unsupported("PICT row overflow".into()))?;
    if row_bytes > 0x7FFF {
        return Err(PsdError::Unsupported("PICT 32-bit width over 8191".into()));
    }
    let components = if raster.alpha { 4usize } else { 3 };
    let pack_type = if row_bytes < 8 { 1 } else { 4 };
    out.extend_from_slice(&DIRECT_BITS_RECT.to_be_bytes());
    out.extend_from_slice(&0x0000_00FFu32.to_be_bytes());
    out.extend_from_slice(&((row_bytes as u16) | 0x8000).to_be_bytes());
    write_rect(out, 0, 0, height as i16, width as i16);
    write_pixmap(out, pack_type, 32, components as i16, 8);
    write_rect(out, 0, 0, height as i16, width as i16);
    write_rect(out, 0, 0, height as i16, width as i16);
    out.extend_from_slice(&SRC_COPY.to_be_bytes());
    if pack_type == 1 {
        for y in 0..height {
            for x in 0..width {
                let at = (y * width + x) * 4;
                if components == 4 {
                    out.extend_from_slice(&[
                        raster.rgba[at + 3],
                        raster.rgba[at],
                        raster.rgba[at + 1],
                        raster.rgba[at + 2],
                    ]);
                } else {
                    out.extend_from_slice(&[
                        0,
                        raster.rgba[at],
                        raster.rgba[at + 1],
                        raster.rgba[at + 2],
                    ]);
                }
            }
        }
        return Ok(());
    }
    let mut planes = vec![0u8; components * width];
    for y in 0..height {
        for x in 0..width {
            let at = (y * width + x) * 4;
            if components == 4 {
                planes[x] = raster.rgba[at + 3];
                planes[width + x] = raster.rgba[at];
                planes[2 * width + x] = raster.rgba[at + 1];
                planes[3 * width + x] = raster.rgba[at + 2];
            } else {
                planes[x] = raster.rgba[at];
                planes[width + x] = raster.rgba[at + 1];
                planes[2 * width + x] = raster.rgba[at + 2];
            }
        }
        let packed = packbits::encode(&planes);
        if row_bytes > 250 {
            out.extend_from_slice(&(packed.len() as u16).to_be_bytes());
        } else {
            out.push(packed.len() as u8);
        }
        out.extend_from_slice(&packed);
    }
    Ok(())
}

fn write_direct16(out: &mut Vec<u8>, raster: &Raster) -> Result<(), PsdError> {
    let width = raster.width as usize;
    let height = raster.height as usize;
    let row_bytes = width
        .checked_mul(2)
        .ok_or_else(|| PsdError::Unsupported("PICT row overflow".into()))?;
    if row_bytes > 0x7FFF {
        return Err(PsdError::Unsupported("PICT 16-bit width over 16383".into()));
    }
    out.extend_from_slice(&DIRECT_BITS_RECT.to_be_bytes());
    out.extend_from_slice(&0x0000_00FFu32.to_be_bytes());
    out.extend_from_slice(&((row_bytes as u16) | 0x8000).to_be_bytes());
    write_rect(out, 0, 0, height as i16, width as i16);
    write_pixmap(out, 1, 16, 3, 5);
    write_rect(out, 0, 0, height as i16, width as i16);
    write_rect(out, 0, 0, height as i16, width as i16);
    out.extend_from_slice(&SRC_COPY.to_be_bytes());
    for y in 0..height {
        for x in 0..width {
            let at = (y * width + x) * 4;
            let alpha = raster.rgba[at + 3];
            let red = over_white(raster.rgba[at], alpha) >> 3;
            let green = over_white(raster.rgba[at + 1], alpha) >> 3;
            let blue = over_white(raster.rgba[at + 2], alpha) >> 3;
            let word = (u16::from(red) << 10) | (u16::from(green) << 5) | u16::from(blue);
            out.extend_from_slice(&word.to_be_bytes());
        }
    }
    Ok(())
}

fn encode_jpeg(
    raster: &Raster,
    quality: u8,
    jpeg: Option<&JpegEncode<'_>>,
) -> Result<Vec<u8>, PsdError> {
    let pixels = raster.width as usize * raster.height as usize;
    let mut rgb = Vec::with_capacity(pixels * 3);
    for index in 0..pixels {
        let at = index * 4;
        let alpha = raster.rgba[at + 3];
        rgb.push(over_white(raster.rgba[at], alpha));
        rgb.push(over_white(raster.rgba[at + 1], alpha));
        rgb.push(over_white(raster.rgba[at + 2], alpha));
    }
    let quality = jpeg_quality_percent(quality);
    if let Some(encode) = jpeg {
        encode(&rgb, raster.width, raster.height, 3, quality)
            .ok_or_else(|| PsdError::Invalid("PICT JPEG encode".into()))
    } else {
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
            .encode(
                &rgb,
                raster.width,
                raster.height,
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|_| PsdError::Invalid("PICT JPEG encode".into()))?;
        Ok(out)
    }
}

fn write_jpeg_record(
    out: &mut Vec<u8>,
    width: u32,
    height: u32,
    jpeg: &[u8],
) -> Result<(), PsdError> {
    let length = jpeg
        .len()
        .checked_add(154)
        .and_then(|length| u32::try_from(length).ok())
        .ok_or_else(|| PsdError::Unsupported("PICT JPEG over 4 GiB".into()))?;
    out.extend_from_slice(&COMPRESSED_QUICKTIME.to_be_bytes());
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&quicktime_header(width, height, jpeg.len() as u32));
    out.extend_from_slice(jpeg);
    Ok(())
}

/// The 154-byte QuickTime image description PICT `$8200` carries: a
/// Photo-JPEG codec description with the JPEG's size and the payload length.
fn quicktime_header(width: u32, height: u32, jpeg_length: u32) -> [u8; 154] {
    let mut bytes = Vec::with_capacity(154);
    let word = |bytes: &mut Vec<u8>, value: u16| bytes.extend_from_slice(&value.to_be_bytes());
    let long = |bytes: &mut Vec<u8>, value: u32| bytes.extend_from_slice(&value.to_be_bytes());
    word(&mut bytes, 0);
    long(&mut bytes, 0x0001_0000);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0x0001_0000);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0x4000_0000);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0x0040_0000);
    word(&mut bytes, 0);
    word(&mut bytes, height as u16);
    word(&mut bytes, width as u16);
    word(&mut bytes, 0);
    word(&mut bytes, 768);
    word(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0x0056_6A70);
    long(&mut bytes, 0x6567_0000);
    long(&mut bytes, 0);
    long(&mut bytes, 1);
    long(&mut bytes, 0x0001_6170);
    long(&mut bytes, 0x706C_0000);
    long(&mut bytes, 0);
    word(&mut bytes, 768);
    word(&mut bytes, width as u16);
    word(&mut bytes, height as u16);
    word(&mut bytes, 72);
    word(&mut bytes, 0);
    word(&mut bytes, 72);
    word(&mut bytes, 0);
    long(&mut bytes, jpeg_length);
    word(&mut bytes, 1);
    long(&mut bytes, 0x0B46_6F74);
    long(&mut bytes, 0x6F20_2D20);
    long(&mut bytes, 0x4A50_4547);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0);
    long(&mut bytes, 0x0018_FFFF);
    let mut header = [0u8; 154];
    header.copy_from_slice(&bytes);
    header
}
