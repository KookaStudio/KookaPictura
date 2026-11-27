//! The PICT reader: header, opcode loop, bitmap/pixmap records, colour and the
//! QuickTime JPEG record.

use crate::probe::ImageBudget;

use super::packbits;

/// A decoded PICT picture.
#[derive(Debug, Clone, PartialEq)]
pub struct PictImage {
    pub width: u32,
    pub height: u32,
    /// Tightly packed RGBA8888, top row first.
    pub rgba: Vec<u8>,
    /// The resolution from the extended version 2 header, if present.
    pub ppi: Option<(f64, f64)>,
}

/// A typed PICT refusal.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PictError {
    #[error("malformed PICT: {0}")]
    Malformed(&'static str),
    #[error("unsupported PICT: {0}")]
    Unsupported(&'static str),
    #[error("PICT declared {width}x{height} over the {limit} px dimension limit")]
    DimensionLimit { width: u32, height: u32, limit: u32 },
    #[error("PICT declared {width}x{height} needing {bytes} bytes over the {limit} byte allocation limit")]
    AllocationLimit {
        width: u32,
        height: u32,
        bytes: u64,
        limit: u64,
    },
    #[error("PICT embedded JPEG is malformed")]
    Jpeg,
}

/// Decode a PICT file or headerless picture to packed RGBA8888.
pub fn decode_pict(bytes: &[u8], budget: ImageBudget) -> Result<PictImage, PictError> {
    let (offset, version) = header_offset(bytes)?;
    let mut reader = Reader::new(bytes, offset);
    let _pic_size = reader.u16()?;
    let frame = reader.read_rect()?;
    let (width, height) = rect_size(frame).ok_or(PictError::Malformed("empty picture frame"))?;
    let mut canvas = Canvas::new(width, height, budget)?;

    reader.set_pos(offset + version.opcode_offset());
    let mut foreground = [0u8, 0u8, 0u8];
    let mut background = [255u8, 255u8, 255u8];
    let mut ppi = None;
    let mut saw_image = false;
    let mut records = 0usize;

    loop {
        if reader.at_end() {
            return Err(PictError::Malformed("missing end of picture"));
        }
        let code = match version {
            Version::One => u16::from(reader.u8()?),
            Version::Two => reader.u16()?,
        };
        if (version == Version::One && code == 0x00FF)
            || (version == Version::Two && code == 0x00FF)
        {
            break;
        }
        if version == Version::Two && code == 0x0C00 {
            ppi = read_header_op(&mut reader)?;
            reader.align_even()?;
            continue;
        }
        match code {
            0x0000 => {}
            0x0001 => skip_region_size(&mut reader)?,
            0x0002 | 0x0009 | 0x000A => reader.skip(8)?,
            0x0003 | 0x0005 | 0x0008 | 0x000D | 0x0015 | 0x0016 => reader.skip(2)?,
            0x0004 => reader.skip(1)?,
            0x0006 | 0x0007 | 0x000B | 0x000C | 0x000E | 0x000F => {
                if code == 0x000E {
                    foreground = read_quickdraw_color(&mut reader)?;
                } else if code == 0x000F {
                    background = read_quickdraw_color(&mut reader)?;
                } else {
                    reader.skip(4)?;
                }
            }
            0x0010 => reader.skip(8)?,
            0x0011 => reader.skip(1)?,
            0x0012..=0x0014 => return Err(PictError::Unsupported("PICT pattern records")),
            0x0017..=0x0019 => return Err(PictError::Unsupported("reserved PICT opcode")),
            0x001A => foreground = read_rgb(&mut reader)?,
            0x001B => background = read_rgb(&mut reader)?,
            0x001C | 0x001E => {}
            0x001D | 0x001F => reader.skip(6)?,
            0x0020 => reader.skip(8)?,
            0x0021 => reader.skip(4)?,
            0x0022 => reader.skip(6)?,
            0x0023 => reader.skip(2)?,
            0x0024..=0x0027 | 0x002F => skip_length_word(&mut reader)?,
            0x0028 => {
                reader.skip(4)?;
                let count = reader.u8()?;
                reader.skip(usize::from(count))?;
            }
            0x0029 | 0x002A => {
                reader.skip(1)?;
                let count = reader.u8()?;
                reader.skip(usize::from(count))?;
            }
            0x002B => {
                reader.skip(2)?;
                let count = reader.u8()?;
                reader.skip(usize::from(count))?;
            }
            0x002C => skip_length_word(&mut reader)?,
            0x002D => reader.skip(10)?,
            0x002E => reader.skip(8)?,
            0x0030..=0x0034
            | 0x0035..=0x0037
            | 0x0040..=0x0044
            | 0x0045..=0x0047
            | 0x0050..=0x0054
            | 0x0055..=0x0057 => reader.skip(8)?,
            0x0038..=0x003C
            | 0x003D..=0x003F
            | 0x0048..=0x004C
            | 0x004D..=0x004F
            | 0x0058..=0x005C
            | 0x005D..=0x005F
            | 0x0078..=0x007C
            | 0x007D..=0x007F
            | 0x0088..=0x008C
            | 0x008D..=0x008F => {}
            0x0060..=0x0067 => reader.skip(12)?,
            0x0068..=0x006F => reader.skip(4)?,
            0x0070..=0x0077 | 0x0080..=0x0087 => skip_region_size(&mut reader)?,
            0x0090 | 0x0091 | 0x0098 | 0x0099 | 0x009A | 0x009B => {
                records += 1;
                if records > 4096 {
                    return Err(PictError::Unsupported("more than 4096 PICT image records"));
                }
                decode_bits_record(
                    &mut reader,
                    code,
                    &mut canvas,
                    foreground,
                    background,
                    budget,
                )?;
                saw_image = true;
            }
            0x0092..=0x0097 | 0x009C..=0x009F | 0x00A2..=0x00AF => skip_length_word(&mut reader)?,
            0x00A0 => reader.skip(2)?,
            0x00A1 => {
                reader.skip(2)?;
                let length = reader.u16()?;
                reader.skip(usize::from(length))?;
            }
            0x00B0..=0x00CF => {}
            0x00D0..=0x00FE => {
                let length = reader.u32()?;
                reader.skip(length as usize)?;
            }
            0x0100..=0x7FFF => {
                let length = 2 * usize::from(code >> 8);
                reader.skip(length)?;
            }
            0x8000..=0x80FF => {}
            0x8100..=0x81FF => {
                let length = reader.u32()?;
                reader.skip(length as usize)?;
            }
            0x8200 => {
                records += 1;
                if records > 4096 {
                    return Err(PictError::Unsupported("more than 4096 PICT image records"));
                }
                saw_image |= decode_quicktime_jpeg(&mut reader, &mut canvas, budget)?;
            }
            0x8201 => return Err(PictError::Unsupported("uncompressed QuickTime PICT record")),
            0x8300..=0xFFFE => return Err(PictError::Unsupported("reserved PICT opcode")),
            0xFFFF => {
                let length = reader.u32()?;
                reader.skip(length as usize)?;
            }
            _ => return Err(PictError::Unsupported("unknown PICT opcode")),
        }
        if version == Version::Two {
            reader.align_even()?;
        }
    }

    if !saw_image {
        return Err(PictError::Unsupported("PICT has no bitmap image records"));
    }
    Ok(PictImage {
        width,
        height,
        rgba: canvas.rgba,
        ppi,
    })
}

/// The `Option` form the application's decode edge uses, mirroring
/// [`decode_webp`](crate::decode_webp).
pub fn decode_pict_rgba(bytes: &[u8], budget: ImageBudget) -> Option<(Vec<u8>, u32, u32)> {
    let image = decode_pict(bytes, budget).ok()?;
    Some((image.rgba, image.width, image.height))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Version {
    One,
    Two,
}

impl Version {
    fn opcode_offset(self) -> usize {
        match self {
            Self::One => 12,
            Self::Two => 14,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    top: i16,
    left: i16,
    bottom: i16,
    right: i16,
}

impl Rect {
    fn size(self) -> Option<(u32, u32)> {
        let width = i32::from(self.right) - i32::from(self.left);
        let height = i32::from(self.bottom) - i32::from(self.top);
        if width <= 0 || height <= 0 {
            None
        } else {
            Some((width as u32, height as u32))
        }
    }
}

fn rect_size(rect: Rect) -> Option<(u32, u32)> {
    rect.size()
}

fn header_offset(bytes: &[u8]) -> Result<(usize, Version), PictError> {
    if bytes.len() >= 524 && bytes[522..524] == [0x11, 0x01] {
        return Ok((512, Version::One));
    }
    if bytes.len() >= 526
        && matches!(
            &bytes[522..526],
            [0x00, 0x11, 0x02, 0xFF] | [0x00, 0x11, 0x00, 0x02]
        )
    {
        return Ok((512, Version::Two));
    }
    if bytes.len() >= 12 && bytes[10..12] == [0x11, 0x01] {
        return Ok((0, Version::One));
    }
    if bytes.len() >= 14
        && matches!(
            &bytes[10..14],
            [0x00, 0x11, 0x02, 0xFF] | [0x00, 0x11, 0x00, 0x02]
        )
    {
        return Ok((0, Version::Two));
    }
    Err(PictError::Malformed("missing PICT version opcode"))
}

fn read_header_op(reader: &mut Reader<'_>) -> Result<Option<(f64, f64)>, PictError> {
    let _version = reader.u16()?;
    let _reserved = reader.u16()?;
    let h_res = reader.u32()?;
    let v_res = reader.u32()?;
    let _frame = reader.read_rect()?;
    let _reserved = reader.u32()?;
    if h_res > 0 && v_res > 0 {
        Ok(Some((
            f64::from(h_res) / 65536.0,
            f64::from(v_res) / 65536.0,
        )))
    } else {
        Ok(None)
    }
}

fn read_quickdraw_color(reader: &mut Reader<'_>) -> Result<[u8; 3], PictError> {
    let color = reader.u32()?;
    Ok([
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
    ])
}

fn read_rgb(reader: &mut Reader<'_>) -> Result<[u8; 3], PictError> {
    let red = reader.u16()?;
    let green = reader.u16()?;
    let blue = reader.u16()?;
    Ok([(red >> 8) as u8, (green >> 8) as u8, (blue >> 8) as u8])
}

fn skip_length_word(reader: &mut Reader<'_>) -> Result<(), PictError> {
    let length = reader.u16()?;
    reader.skip(usize::from(length))?;
    Ok(())
}

fn skip_region_size(reader: &mut Reader<'_>) -> Result<(), PictError> {
    let length = reader.u16()?;
    let data = length
        .checked_sub(2)
        .ok_or(PictError::Malformed("short PICT region"))?;
    reader.skip(usize::from(data))?;
    Ok(())
}

fn check_budget(width: u32, height: u32, budget: ImageBudget) -> Result<(), PictError> {
    if width == 0 || height == 0 {
        return Err(PictError::Malformed("zero image size"));
    }
    if width > budget.max_dimension || height > budget.max_dimension {
        return Err(PictError::DimensionLimit {
            width,
            height,
            limit: budget.max_dimension,
        });
    }
    let bytes = (u64::from(width))
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4));
    match bytes {
        Some(bytes) if bytes <= budget.max_alloc_bytes => Ok(()),
        Some(bytes) => Err(PictError::AllocationLimit {
            width,
            height,
            bytes,
            limit: budget.max_alloc_bytes,
        }),
        None => Err(PictError::AllocationLimit {
            width,
            height,
            bytes: u64::MAX,
            limit: budget.max_alloc_bytes,
        }),
    }
}

struct Canvas {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32, budget: ImageBudget) -> Result<Self, PictError> {
        check_budget(width, height, budget)?;
        let len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(PictError::Malformed("PICT size overflow"))?;
        Ok(Self {
            width: width as usize,
            height: height as usize,
            rgba: vec![255; len],
        })
    }

    fn composite(&mut self, source: &[u8], source_width: usize, source_height: usize, dst: Rect) {
        let Some((dst_width, dst_height)) = dst.size() else {
            return;
        };
        if source_width == 0 || source_height == 0 {
            return;
        }
        let left = i64::from(dst.left);
        let top = i64::from(dst.top);
        let x0 = left.max(0);
        let y0 = top.max(0);
        let x1 = (left + i64::from(dst_width)).min(self.width as i64);
        let y1 = (top + i64::from(dst_height)).min(self.height as i64);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let dst_width = dst_width as usize;
        let dst_height = dst_height as usize;
        for y in y0..y1 {
            let dy = (y - top) as usize;
            let sy = if dst_height == source_height {
                dy
            } else {
                dy * source_height / dst_height
            };
            for x in x0..x1 {
                let dx = (x - left) as usize;
                let sx = if dst_width == source_width {
                    dx
                } else {
                    dx * source_width / dst_width
                };
                let source_at = (sy * source_width + sx) * 4;
                let target_at = (y as usize * self.width + x as usize) * 4;
                if let Some(pixel) = source.get(source_at..source_at + 4) {
                    self.rgba[target_at..target_at + 4].copy_from_slice(pixel);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PixMap {
    row_bytes: usize,
    pack_type: i16,
    pixel_size: i16,
    component_count: i16,
}

fn read_pixmap_tail(reader: &mut Reader<'_>, row_bytes: u16) -> Result<PixMap, PictError> {
    let row_bytes = usize::from(row_bytes & 0x7FFF);
    if row_bytes == 0 {
        return Err(PictError::Malformed("zero PICT row bytes"));
    }
    let _version = reader.i16()?;
    let pack_type = reader.i16()?;
    let _pack_size = reader.u32()?;
    let _h_res = reader.u32()?;
    let _v_res = reader.u32()?;
    let _pixel_type = reader.i16()?;
    let pixel_size = reader.i16()?;
    let component_count = reader.i16()?;
    let component_size = reader.i16()?;
    let _plane_bytes = reader.u32()?;
    let _pm_table = reader.u32()?;
    let _pm_reserved = reader.u32()?;
    if !matches!(pixel_size, 1 | 2 | 4 | 8 | 16 | 32) {
        return Err(PictError::Unsupported("PICT pixel size"));
    }
    if !(1..=4).contains(&component_count) || !(1..=32).contains(&component_size) {
        return Err(PictError::Unsupported("PICT component layout"));
    }
    Ok(PixMap {
        row_bytes,
        pack_type,
        pixel_size,
        component_count,
    })
}

fn read_color_table(reader: &mut Reader<'_>) -> Result<Vec<[u8; 3]>, PictError> {
    let _seed = reader.u32()?;
    let flags = reader.u16()?;
    let count = usize::from(reader.u16()?) + 1;
    if count > 256 {
        return Err(PictError::Unsupported("PICT colour table over 256 entries"));
    }
    let mut colors = vec![[0u8, 0u8, 0u8]; count];
    for index in 0..count {
        let value = usize::from(reader.u16()?);
        let red = reader.u16()?;
        let green = reader.u16()?;
        let blue = reader.u16()?;
        let target = if flags & 0x8000 != 0 {
            index
        } else {
            value % count
        };
        colors[target] = [(red >> 8) as u8, (green >> 8) as u8, (blue >> 8) as u8];
    }
    Ok(colors)
}

fn read_region(reader: &mut Reader<'_>) -> Result<(), PictError> {
    let length = reader.u16()?;
    let data = length
        .checked_sub(2)
        .ok_or(PictError::Malformed("short PICT region"))?;
    reader.skip(usize::from(data))?;
    Ok(())
}

fn decode_bits_record(
    reader: &mut Reader<'_>,
    code: u16,
    canvas: &mut Canvas,
    foreground: [u8; 3],
    background: [u8; 3],
    budget: ImageBudget,
) -> Result<(), PictError> {
    let direct = matches!(code, 0x009A | 0x009B);
    let (row_bytes, bounds, pixmap) = if direct {
        let _base_address = reader.u32()?;
        let row_bytes = reader.u16()?;
        let bounds = reader.read_rect()?;
        let pixmap = read_pixmap_tail(reader, row_bytes)?;
        (row_bytes, bounds, Some(pixmap))
    } else {
        let row_bytes = reader.u16()?;
        let bounds = reader.read_rect()?;
        let pixmap = if row_bytes & 0x8000 != 0 {
            Some(read_pixmap_tail(reader, row_bytes)?)
        } else {
            None
        };
        (row_bytes, bounds, pixmap)
    };
    let (width, height) = bounds
        .size()
        .ok_or(PictError::Malformed("empty PICT image record"))?;
    check_budget(width, height, budget)?;
    let colors = if pixmap.is_some() && !direct {
        Some(read_color_table(reader)?)
    } else {
        None
    };
    let _source = reader.read_rect()?;
    let destination = reader.read_rect()?;
    let _mode = reader.u16()?;
    if matches!(code, 0x0091 | 0x0099 | 0x009B) {
        read_region(reader)?;
    }
    let rgba = decode_pixels(
        reader,
        code,
        row_bytes,
        pixmap,
        width as usize,
        height as usize,
        colors.as_deref(),
        foreground,
        background,
    )?;
    canvas.composite(&rgba, width as usize, height as usize, destination);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn decode_pixels(
    reader: &mut Reader<'_>,
    code: u16,
    row_bytes_field: u16,
    pixmap: Option<PixMap>,
    width: usize,
    height: usize,
    colors: Option<&[[u8; 3]]>,
    foreground: [u8; 3],
    background: [u8; 3],
) -> Result<Vec<u8>, PictError> {
    let mut rgba = vec![0u8; width * height * 4];
    let row_bytes = pixmap.map_or(usize::from(row_bytes_field & 0x7FFF), |p| p.row_bytes);
    let pixel_size = pixmap.map_or(1, |p| p.pixel_size as usize);
    let component_count = pixmap.map_or(1, |p| p.component_count as usize);
    let is_bitmap = pixmap.is_none();
    let packed = match code {
        0x0090 | 0x0091 => false,
        0x0098 | 0x0099 => true,
        0x009A | 0x009B => {
            let pack_type = pixmap.map_or(1, |p| p.pack_type);
            let pack_type = if pack_type == 0 {
                if pixel_size == 16 {
                    3
                } else if pixel_size == 32 {
                    4
                } else {
                    1
                }
            } else {
                pack_type
            };
            match pack_type {
                1 => false,
                2 => {
                    if pixel_size != 32 {
                        return Err(PictError::Unsupported("PICT drop-pad packing"));
                    }
                    decode_drop_pad(reader, width, height, &mut rgba)?;
                    return Ok(rgba);
                }
                3 => {
                    if pixel_size != 16 {
                        return Err(PictError::Unsupported("PICT pixel RLE packing"));
                    }
                    true
                }
                4 => {
                    if pixel_size != 32 {
                        return Err(PictError::Unsupported("PICT component RLE packing"));
                    }
                    true
                }
                _ => return Err(PictError::Unsupported("PICT packing type")),
            }
        }
        _ => unreachable!("bitmap opcode"),
    };
    let packed = packed && !(matches!(code, 0x009A | 0x009B) && row_bytes < 8);
    if packed {
        let prefix = if row_bytes > 250 { 2 } else { 1 };
        let expected = if pixel_size == 32 && component_count > 1 {
            component_count * width
        } else {
            row_bytes
        };
        let mut decoded = Vec::with_capacity(expected);
        for y in 0..height {
            let length = if prefix == 2 {
                usize::from(reader.u16()?)
            } else {
                usize::from(reader.u8()?)
            };
            decoded.clear();
            if length == 0 {
                decoded.resize(expected, 0);
            } else {
                let compressed = reader.take(length)?;
                packbits::decode_exact(compressed, &mut decoded, expected)
                    .map_err(|()| PictError::Malformed("short PICT PackBits row"))?;
            }
            if pixel_size == 32 && component_count > 1 {
                unpack_component_planes(&decoded, width, component_count, &mut rgba, y * width)?;
            } else {
                unpack_row(
                    &decoded,
                    width,
                    pixel_size,
                    component_count,
                    is_bitmap,
                    colors,
                    foreground,
                    background,
                    &mut rgba,
                    y * width,
                )?;
            }
        }
        return Ok(rgba);
    }
    for y in 0..height {
        let row = reader.take(row_bytes)?;
        if pixel_size == 32 && component_count > 1 {
            unpack_unpacked_direct(row, width, component_count, &mut rgba, y * width)?;
        } else {
            unpack_row(
                row,
                width,
                pixel_size,
                component_count,
                is_bitmap,
                colors,
                foreground,
                background,
                &mut rgba,
                y * width,
            )?;
        }
    }
    Ok(rgba)
}

fn decode_drop_pad(
    reader: &mut Reader<'_>,
    width: usize,
    height: usize,
    rgba: &mut [u8],
) -> Result<(), PictError> {
    for y in 0..height {
        let row = reader.take(width * 3)?;
        for x in 0..width {
            let at = (y * width + x) * 4;
            rgba[at] = row[x * 3];
            rgba[at + 1] = row[x * 3 + 1];
            rgba[at + 2] = row[x * 3 + 2];
            rgba[at + 3] = 255;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn unpack_row(
    row: &[u8],
    width: usize,
    pixel_size: usize,
    component_count: usize,
    is_bitmap: bool,
    colors: Option<&[[u8; 3]]>,
    foreground: [u8; 3],
    background: [u8; 3],
    rgba: &mut [u8],
    pixel_base: usize,
) -> Result<(), PictError> {
    if pixel_size == 16 {
        if row.len() < width * 2 {
            return Err(PictError::Malformed("short PICT 16-bit row"));
        }
        for x in 0..width {
            let word = u16::from_be_bytes([row[x * 2], row[x * 2 + 1]]);
            let red = ((word >> 10) & 0x1F) as u8;
            let green = ((word >> 5) & 0x1F) as u8;
            let blue = (word & 0x1F) as u8;
            let at = (pixel_base + x) * 4;
            rgba[at] = (red << 3) | (red >> 2);
            rgba[at + 1] = (green << 3) | (green >> 2);
            rgba[at + 2] = (blue << 3) | (blue >> 2);
            rgba[at + 3] = 255;
        }
        return Ok(());
    }
    if pixel_size == 32 {
        if row.len() < width * 4 {
            return Err(PictError::Malformed("short PICT 32-bit row"));
        }
        for x in 0..width {
            let source = &row[x * 4..x * 4 + 4];
            let at = (pixel_base + x) * 4;
            if component_count == 4 {
                rgba[at] = source[1];
                rgba[at + 1] = source[2];
                rgba[at + 2] = source[3];
                rgba[at + 3] = source[0];
            } else {
                rgba[at] = source[1];
                rgba[at + 1] = source[2];
                rgba[at + 2] = source[3];
                rgba[at + 3] = 255;
            }
        }
        return Ok(());
    }
    let sample = |x: usize| -> Result<usize, PictError> {
        match pixel_size {
            1 => {
                let byte = *row
                    .get(x / 8)
                    .ok_or(PictError::Malformed("short PICT row"))?;
                Ok(usize::from((byte >> (7 - x % 8)) & 1))
            }
            2 => {
                let byte = *row
                    .get(x / 4)
                    .ok_or(PictError::Malformed("short PICT row"))?;
                Ok(usize::from((byte >> (6 - 2 * (x % 4))) & 0x03))
            }
            4 => {
                let byte = *row
                    .get(x / 2)
                    .ok_or(PictError::Malformed("short PICT row"))?;
                Ok(usize::from((byte >> (4 - 4 * (x % 2))) & 0x0F))
            }
            8 => Ok(usize::from(
                *row.get(x).ok_or(PictError::Malformed("short PICT row"))?,
            )),
            _ => Err(PictError::Unsupported("PICT indexed pixel size")),
        }
    };
    for x in 0..width {
        let index = sample(x)?;
        let color = match colors {
            Some(colors) => colors.get(index).copied().unwrap_or([0, 0, 0]),
            None if is_bitmap => {
                if index == 0 {
                    background
                } else {
                    foreground
                }
            }
            None => [0, 0, 0],
        };
        let at = (pixel_base + x) * 4;
        rgba[at] = color[0];
        rgba[at + 1] = color[1];
        rgba[at + 2] = color[2];
        rgba[at + 3] = 255;
    }
    Ok(())
}

fn unpack_component_planes(
    planes: &[u8],
    width: usize,
    component_count: usize,
    rgba: &mut [u8],
    pixel_base: usize,
) -> Result<(), PictError> {
    let needed = width * component_count;
    if planes.len() < needed {
        return Err(PictError::Malformed("short PICT component planes"));
    }
    for x in 0..width {
        let at = (pixel_base + x) * 4;
        if component_count == 4 {
            rgba[at] = planes[width + x];
            rgba[at + 1] = planes[2 * width + x];
            rgba[at + 2] = planes[3 * width + x];
            rgba[at + 3] = planes[x];
        } else {
            rgba[at] = planes[x];
            rgba[at + 1] = planes[width + x];
            rgba[at + 2] = planes[2 * width + x];
            rgba[at + 3] = 255;
        }
    }
    Ok(())
}

fn unpack_unpacked_direct(
    row: &[u8],
    width: usize,
    component_count: usize,
    rgba: &mut [u8],
    pixel_base: usize,
) -> Result<(), PictError> {
    if row.len() < width * 4 {
        return Err(PictError::Malformed("short PICT direct row"));
    }
    for x in 0..width {
        let at = (pixel_base + x) * 4;
        let source = &row[x * 4..x * 4 + 4];
        if component_count == 4 {
            rgba[at] = source[1];
            rgba[at + 1] = source[2];
            rgba[at + 2] = source[3];
            rgba[at + 3] = source[0];
        } else {
            rgba[at] = source[1];
            rgba[at + 1] = source[2];
            rgba[at + 2] = source[3];
            rgba[at + 3] = 255;
        }
    }
    Ok(())
}

fn decode_quicktime_jpeg(
    reader: &mut Reader<'_>,
    canvas: &mut Canvas,
    budget: ImageBudget,
) -> Result<bool, PictError> {
    let length = reader.u32()? as usize;
    if length == 0 {
        return Ok(false);
    }
    if length < 154 {
        return Err(PictError::Malformed("short QuickTime JPEG record"));
    }
    let header = reader.take(154)?;
    let rect = Rect {
        top: i16::from_be_bytes([header[24], header[25]]),
        left: i16::from_be_bytes([header[26], header[27]]),
        bottom: i16::from_be_bytes([header[28], header[29]]),
        right: i16::from_be_bytes([header[30], header[31]]),
    };
    let payload = reader.take(length - 154)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(budget.max_dimension);
    limits.max_image_height = Some(budget.max_dimension);
    limits.max_alloc = Some(budget.max_alloc_bytes);
    let mut jpeg =
        image::ImageReader::with_format(std::io::Cursor::new(payload), image::ImageFormat::Jpeg);
    jpeg.limits(limits);
    let image = jpeg.decode().map_err(|_| PictError::Jpeg)?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    check_budget(width, height, budget)?;
    let destination = if rect.size().is_some() {
        rect
    } else {
        Rect {
            top: 0,
            left: 0,
            bottom: height as i16,
            right: width as i16,
        }
    };
    canvas.composite(rgba.as_raw(), width as usize, height as usize, destination);
    Ok(true)
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], pos: usize) -> Self {
        Self { bytes, pos }
    }

    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn u8(&mut self) -> Result<u8, PictError> {
        let byte = *self
            .bytes
            .get(self.pos)
            .ok_or(PictError::Malformed("truncated PICT"))?;
        self.pos += 1;
        Ok(byte)
    }

    fn u16(&mut self) -> Result<u16, PictError> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn i16(&mut self) -> Result<i16, PictError> {
        Ok(self.u16()? as i16)
    }

    fn u32(&mut self) -> Result<u32, PictError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], PictError> {
        let end = self
            .pos
            .checked_add(length)
            .ok_or(PictError::Malformed("PICT length overflow"))?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or(PictError::Malformed("truncated PICT"))?;
        self.pos = end;
        Ok(slice)
    }

    fn skip(&mut self, length: usize) -> Result<(), PictError> {
        self.take(length).map(|_| ())
    }

    fn align_even(&mut self) -> Result<(), PictError> {
        if !self.pos.is_multiple_of(2) {
            self.skip(1)?;
        }
        Ok(())
    }

    fn read_rect(&mut self) -> Result<Rect, PictError> {
        Ok(Rect {
            top: self.i16()?,
            left: self.i16()?,
            bottom: self.i16()?,
            right: self.i16()?,
        })
    }
}
