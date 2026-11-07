use super::helpers_composite::{aspect_thumbnail_extent, rgba_image};
use cxx_qt_lib::QImage;
use pictura_core::{LayerMask, PsdRect, VectorMask};

/// Downsample a vector mask's coverage into a `size`-bounded square (long edge
/// `size`, aspect kept), replicating the value across RGB with the
/// white-shows / black-hides convention. An absent,
/// disabled, or all-open mask is fully white (permissive), matching the
/// compositor. `ponytail:` CS6 strokes the path outline; this fills coverage.
pub(super) fn vector_mask_thumbnail_image(
    mask: &VectorMask,
    doc_width: u32,
    doc_height: u32,
    size: u32,
) -> Option<QImage> {
    if doc_width == 0 || doc_height == 0 || size == 0 {
        return None;
    }
    let (tw, th) = if doc_width >= doc_height {
        (
            size,
            ((doc_height as u64 * size as u64) / doc_width as u64).max(1) as u32,
        )
    } else {
        (
            ((doc_width as u64 * size as u64) / doc_height as u64).max(1) as u32,
            size,
        )
    };
    let clip = PsdRect {
        top: 0,
        left: 0,
        bottom: doc_height as i32,
        right: doc_width as i32,
    };
    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let dy = (ty as u64 * doc_height as u64 / th as u64) as i32;
        for tx in 0..tw {
            let dx = (tx as u64 * doc_width as u64 / tw as u64) as i32;
            let value = if !mask.has_fill() {
                255
            } else {
                let inside = mask.inside(dx, dy, clip);
                let show = if mask.invert { !inside } else { inside };
                if show {
                    255
                } else {
                    0
                }
            };
            let o = ((ty * tw + tx) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    Some(rgba_image(rgba, tw as i32, th as i32))
}

/// Place a layer mask over the document's
/// aspect ratio at its document position, clipped to the canvas, so a mask
/// smaller than the canvas shows its exterior coverage and a mask wider than the
/// canvas is cropped rather than squashed.
pub(super) fn mask_thumbnail_positioned(
    mask: &LayerMask,
    doc_width: u32,
    doc_height: u32,
    size: u32,
) -> Option<QImage> {
    if doc_width == 0 || doc_height == 0 || size == 0 {
        return None;
    }
    let width = i64::from(mask.rect.right) - i64::from(mask.rect.left);
    let height = i64::from(mask.rect.bottom) - i64::from(mask.rect.top);
    if width < 0 || height < 0 {
        return None;
    }
    if let Some(data) = &mask.data {
        if u64::try_from(data.len()).ok()? != (width as u64).checked_mul(height as u64)? {
            return None;
        }
    }
    let (tw, th) = aspect_thumbnail_extent(doc_width, doc_height, size);
    let left = (mask.rect.left as f64 * tw as f64 / doc_width as f64).round() as i64;
    let top = (mask.rect.top as f64 * th as f64 / doc_height as f64).round() as i64;
    let dw = (width as f64 * tw as f64 / doc_width as f64)
        .round()
        .max(1.0) as i64;
    let dh = (height as f64 * th as f64 / doc_height as f64)
        .round()
        .max(1.0) as i64;
    let outside = if mask.data.is_none() {
        255
    } else {
        mask.default_color
    };
    let mut rgba = vec![0; tw as usize * th as usize * 4];
    for y in 0..th {
        for x in 0..tw {
            let value = if width > 0
                && height > 0
                && i64::from(x) >= left
                && i64::from(x) < left + dw
                && i64::from(y) >= top
                && i64::from(y) < top + dh
            {
                let sx = (((i64::from(x) - left) as u64 * width as u64) / dw as u64)
                    .min(width as u64 - 1);
                let sy = (((i64::from(y) - top) as u64 * height as u64) / dh as u64)
                    .min(height as u64 - 1);
                mask.data
                    .as_ref()
                    .and_then(|data| data.sample((sy * width as u64 + sx) as usize))
                    .unwrap_or(outside)
            } else {
                outside
            };
            let i = (y as usize * tw as usize + x as usize) * 4;
            rgba[i..i + 4].copy_from_slice(&[value, value, value, 255]);
        }
    }
    Some(rgba_image(rgba, tw as i32, th as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(rect: PsdRect, value: u8) -> LayerMask {
        let n = (rect.width() * rect.height()) as usize;
        LayerMask {
            rect,
            data: Some(vec![value; n].into()),
            ..Default::default()
        }
    }

    #[test]
    fn positioned_mask_thumbnail_follows_document_aspect() {
        // A 20x10 document and a 4x4 mask at (6,2): 24x12 image, block at (7,2).
        let small = mask(
            PsdRect {
                top: 2,
                left: 6,
                bottom: 6,
                right: 10,
            },
            255,
        );
        let image = mask_thumbnail_positioned(&small, 20, 10, 24).expect("positioned");
        assert_eq!((image.width(), image.height()), (24, 12));
        assert_eq!(image.pixel_color(7, 2).red(), 255);
        assert_eq!(image.pixel_color(7, 2).alpha(), 255);
        assert_eq!(image.pixel_color(0, 0).alpha(), 255);
        assert_eq!(image.pixel_color(0, 0).red(), small.default_color as i32);

        // Wider than the canvas and offset past its left edge: clipped, no panic.
        let wide = mask(
            PsdRect {
                top: 0,
                left: -2,
                bottom: 10,
                right: 30,
            },
            128,
        );
        let clipped = mask_thumbnail_positioned(&wide, 20, 10, 24).expect("clipped");
        assert_eq!((clipped.width(), clipped.height()), (24, 12));
        assert_eq!(clipped.pixel_color(23, 5).alpha(), 255);
        assert_eq!(clipped.pixel_color(23, 5).red(), 128);

        // A far negative offset must not overflow the source index.
        let far = mask(
            PsdRect {
                top: 0,
                left: -70_000,
                bottom: 1,
                right: 1,
            },
            128,
        );
        let sampled = mask_thumbnail_positioned(&far, 1, 1, 24).expect("far offset");
        assert_eq!(sampled.pixel_color(0, 0).red(), 128);

        // Empty storage uses its default; missing decoded data remains visibly white.
        let empty = mask(
            PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            0,
        );
        assert_eq!(
            mask_thumbnail_positioned(&empty, 20, 10, 24)
                .unwrap()
                .pixel_color(0, 0)
                .red(),
            empty.default_color as i32
        );
        let mut missing = mask(
            PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            0,
        );
        missing.data = None;
        assert_eq!(
            mask_thumbnail_positioned(&missing, 20, 10, 24)
                .unwrap()
                .pixel_color(0, 0)
                .red(),
            255
        );
    }
}
