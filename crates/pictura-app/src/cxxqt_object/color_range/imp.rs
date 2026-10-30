use super::*;

use super::super::picture::picture;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::{BitDepth, PixelBuffer};
use pictura_select::{color_range, ColorRangeSelect, CombineMode};

pub(super) fn color_range_available(view: &PictureView) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| doc.source_depth.unwrap_or(doc.depth) != BitDepth::ThirtyTwo)
}

pub(super) fn target(rgb: u32) -> [u8; 3] {
    [(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8]
}

/// Nearest-neighbour shrink of a planar buffer so its longer side is at most
/// `max_side`; the preview needs the look, not every pixel.
pub(super) fn shrink(buffer: &PixelBuffer, max_side: u32) -> PixelBuffer {
    let longer = buffer.width.max(buffer.height);
    if max_side == 0 || longer <= max_side {
        return buffer.clone();
    }
    let scale = max_side as f64 / longer as f64;
    let w = ((buffer.width as f64 * scale).round() as u32).max(1);
    let h = ((buffer.height as f64 * scale).round() as u32).max(1);
    let (sw, sh) = (buffer.width as usize, buffer.height as usize);
    let plane = sw * sh;
    let mut data = vec![0u8; w as usize * h as usize * buffer.channels as usize];
    for c in 0..buffer.channels as usize {
        for y in 0..h as usize {
            let sy = (y * sh / h as usize).min(sh - 1);
            for x in 0..w as usize {
                let sx = (x * sw / w as usize).min(sw - 1);
                data[c * w as usize * h as usize + y * w as usize + x] =
                    buffer.data[c * plane + sy * sw + sx];
            }
        }
    }
    PixelBuffer {
        width: w,
        height: h,
        channels: buffer.channels,
        data: data.into(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn color_range_preview(
    view: &PictureView,
    select: i32,
    rgb: u32,
    fuzziness: i32,
    invert: bool,
    max_side: i32,
    width: &mut i32,
    height: &mut i32,
) -> Vec<u8> {
    *width = 0;
    *height = 0;
    let (Some(select), true) = (
        ColorRangeSelect::from_index(select),
        color_range_available(view),
    ) else {
        return Vec::new();
    };
    let rust = view.rust();
    let Some(doc) = rust.session.doc.as_ref() else {
        return Vec::new();
    };
    let buffer = shrink(&picture(doc, rust), max_side.max(0) as u32);
    let mask = color_range(
        &buffer,
        select,
        target(rgb),
        fuzziness.max(0) as u32,
        invert,
    );
    *width = mask.width as i32;
    *height = mask.height as i32;
    mask.data.to_vec()
}

pub(super) fn color_range_apply(
    view: Pin<&mut PictureView>,
    select: i32,
    rgb: u32,
    fuzziness: i32,
    invert: bool,
) -> bool {
    let (Some(select), true) = (
        ColorRangeSelect::from_index(select),
        color_range_available(&view),
    ) else {
        return false;
    };
    let (shape, refine) = {
        let rust = view.rust();
        let Some(doc) = rust.session.doc.as_ref() else {
            return false;
        };
        let buffer = picture(doc, rust);
        let shape = color_range(
            &buffer,
            select,
            target(rgb),
            fuzziness.max(0) as u32,
            invert,
        );
        (shape, rust.session.selection.is_some())
    };
    // Run on a live selection, Color Range narrows it to a subset (CS6).
    let mode = if refine {
        CombineMode::Intersect
    } else {
        CombineMode::New
    };
    view.apply_selection_labeled(shape, mode, "Color Range")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shrink_keeps_aspect_and_samples_nearest() {
        let (w, h) = (8u32, 4u32);
        let n = (w * h) as usize;
        let mut data = vec![0u8; 3 * n];
        for (i, v) in data[..n].iter_mut().enumerate() {
            *v = (i % w as usize) as u8 * 10;
        }
        let buffer = PixelBuffer {
            width: w,
            height: h,
            channels: 3,
            data: data.into(),
        };
        let small = shrink(&buffer, 4);
        assert_eq!((small.width, small.height), (4, 2));
        assert_eq!(&small.data[..4], &[0, 20, 40, 60]);
        assert_eq!(shrink(&buffer, 100).width, 8, "no upscaling");
        assert_eq!(target(0x12_34_56), [0x12, 0x34, 0x56]);
    }
}
