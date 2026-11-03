use super::helpers_composite::{aspect_thumbnail_extent, rgba_image};
use cxx_qt_lib::QImage;
use pictura_core::{LayerMask, PsdRect, VectorMask};

/// Downsample a layer mask's grayscale data into a `size`-bounded square (long
/// edge `size`, aspect kept), replicating the value across RGB. Null without
/// mask data.
pub(super) fn mask_thumbnail_image(mask: &LayerMask, size: u32) -> Option<QImage> {
    let data = mask.data.as_ref()?;
    let width = mask.rect.width();
    let height = mask.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);
    let plane = (width * height) as usize;
    if data.len() != plane {
        return None;
    }
    let (tw, th) = if width >= height {
        (
            size,
            ((height as u64 * size as u64) / width as u64).max(1) as u32,
        )
    } else {
        (
            ((width as u64 * size as u64) / height as u64).max(1) as u32,
            size,
        )
    };

    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        let sy = ty as usize * height as usize / th as usize;
        for tx in 0..tw {
            let sx = (tx as usize * width as usize / tw as usize).min(width as usize - 1);
            let v = data.sample(sy * width as usize + sx)?;
            let o = ((ty * tw + tx) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[v, v, v, 255]);
        }
    }
    Some(rgba_image(rgba, tw as i32, th as i32))
}

/// Downsample a vector mask's coverage into a `size`-bounded square (long edge
/// `size`, aspect kept), replicating the value across RGB with the same
/// white-shows / black-hides convention as [`mask_thumbnail_image`]. An absent,
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

/// Panel Options "Entire Document": place a layer mask over the document's
/// aspect ratio at its document position, clipped to the canvas, so a mask
/// smaller than the canvas shows transparent margins and a mask wider than the
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
    let data = mask.data.as_ref()?;
    let width = mask.rect.width();
    let height = mask.rect.height();
    if width <= 0 || height <= 0 {
        return None;
    }
    let (width, height) = (width as u32, height as u32);
    if data.len() != (width as usize * height as usize) {
        return None;
    }
    let (tw, th) = aspect_thumbnail_extent(doc_width, doc_height, size);
    let scale_x = tw as f64 / doc_width as f64;
    let scale_y = th as f64 / doc_height as f64;
    let dest_left = (mask.rect.left as f64 * scale_x).round() as i64;
    let dest_top = (mask.rect.top as f64 * scale_y).round() as i64;
    let dest_w = ((width as f64 * scale_x).round() as i64).max(1);
    let dest_h = ((height as f64 * scale_y).round() as i64).max(1);

    let mut rgba = vec![0u8; (tw as u64 * th as u64 * 4) as usize];
    for py in 0..th as i64 {
        for px in 0..tw as i64 {
            if px < dest_left
                || py < dest_top
                || px >= dest_left + dest_w
                || py >= dest_top + dest_h
            {
                continue;
            }
            let sx = (((px - dest_left) as u64 * width as u64) / dest_w as u64)
                .min(width as u64 - 1) as u32;
            let sy = (((py - dest_top) as u64 * height as u64) / dest_h as u64)
                .min(height as u64 - 1) as u32;
            let v = data.sample(sy as usize * width as usize + sx as usize)?;
            let o = ((py as u32 * tw + px as u32) * 4) as usize;
            rgba[o..o + 4].copy_from_slice(&[v, v, v, 255]);
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
        assert_eq!(image.pixel_color(0, 0).alpha(), 0, "transparent margin");

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

        // A zero-size rect and a mask without data refuse rather than distort.
        let empty = mask(
            PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            0,
        );
        assert!(mask_thumbnail_positioned(&empty, 20, 10, 24).is_none());
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
        assert!(mask_thumbnail_positioned(&missing, 20, 10, 24).is_none());
    }
}
