use super::helpers_composite::{aspect_thumbnail_extent, rgba_image};
use cxx_qt_lib::QImage;
use pictura_core::{LayerMask, PsdRect, VectorMask};

/// CS6's hidden-area grey in a vector-mask thumbnail. `ponytail:` matched by
/// eye; no oracle pins the value.
const VECTOR_THUMB_HIDDEN: u8 = 0x80;

/// A vector mask's thumbnail in a `size`-bounded extent (long edge `size`,
/// document aspect clamped to 3:1), as CS6 draws it: white where the mask
/// reveals, grey where it hides, and the path outline stroked over both in a
/// 1 px black line. An absent, disabled, or all-open mask has no hidden area,
/// matching the compositor.
pub(super) fn vector_mask_thumbnail_image(
    mask: &VectorMask,
    doc_width: u32,
    doc_height: u32,
    size: u32,
) -> Option<QImage> {
    if doc_width == 0 || doc_height == 0 || size == 0 {
        return None;
    }
    let (tw, th) = aspect_thumbnail_extent(doc_width, doc_height, size);
    let rgba = vector_mask_thumbnail_rgba(mask, doc_width, doc_height, tw, th);
    Some(rgba_image(rgba, tw as i32, th as i32))
}

fn vector_mask_thumbnail_rgba(
    mask: &VectorMask,
    doc_width: u32,
    doc_height: u32,
    tw: u32,
    th: u32,
) -> Vec<u8> {
    let clip = PsdRect {
        top: 0,
        left: 0,
        bottom: doc_height as i32,
        right: doc_width as i32,
    };
    let mut rgba = vec![0u8; (tw * th * 4) as usize];
    let mut put = |tx: u32, ty: u32, value: u8| {
        let o = ((ty * tw + tx) * 4) as usize;
        rgba[o..o + 4].copy_from_slice(&[value, value, value, 255]);
    };
    for ty in 0..th {
        let dy = (ty as u64 * doc_height as u64 / th as u64) as i32;
        for tx in 0..tw {
            let dx = (tx as u64 * doc_width as u64 / tw as u64) as i32;
            let show = !mask.has_fill() || mask.inside(dx, dy, clip) != mask.invert;
            put(tx, ty, if show { 255 } else { VECTOR_THUMB_HIDDEN });
        }
    }
    // Vertices are 1/256 document pixels; scale them onto the thumbnail.
    let sx = tw as f64 / (doc_width as f64 * 256.0);
    let sy = th as f64 / (doc_height as f64 * 256.0);
    for subpath in &mask.subpaths {
        let points: Vec<(f64, f64)> = subpath
            .points
            .iter()
            .map(|[x, y]| (*x as f64 * sx, *y as f64 * sy))
            .collect();
        let closing = subpath.closed.then(|| (points.last(), points.first()));
        let segments = points
            .windows(2)
            .map(|w| (w[0], w[1]))
            .chain(closing.and_then(|(a, b)| Some((*a?, *b?))));
        for (a, b) in segments {
            stroke_segment(a, b, tw, th, &mut put);
        }
    }
    rgba
}

/// Mark every thumbnail pixel the segment `a`–`b` passes through (a DDA step
/// per pixel of its longer axis), clipped to the `tw`×`th` thumbnail.
fn stroke_segment(
    a: (f64, f64),
    b: (f64, f64),
    tw: u32,
    th: u32,
    put: &mut impl FnMut(u32, u32, u8),
) {
    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as u32;
    for i in 0..=steps {
        let t = f64::from(i) / f64::from(steps);
        let x = (a.0 + (b.0 - a.0) * t).floor();
        let y = (a.1 + (b.1 - a.1) * t).floor();
        // The right and bottom canvas edges land one past the last pixel.
        let x = x.min(f64::from(tw) - 1.0);
        let y = y.min(f64::from(th) - 1.0);
        if x >= 0.0 && y >= 0.0 {
            put(x as u32, y as u32, 0);
        }
    }
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

    #[test]
    fn extreme_document_mask_thumbnail_aspect_is_clamped() {
        let layer = mask(
            PsdRect {
                top: 25,
                left: 50,
                bottom: 75,
                right: 100,
            },
            255,
        );
        let image = mask_thumbnail_positioned(&layer, 400, 100, 24).expect("clamped mask");
        assert_eq!((image.width(), image.height()), (24, 8));
        assert_eq!(image.pixel_color(0, 0).red(), 0);
        assert_eq!(image.pixel_color(3, 2).red(), 255);
    }

    #[test]
    fn vector_mask_thumbnail_strokes_the_outline() {
        let corner = |x: f64, y: f64| pictura_core::path::PathPoint {
            anchor: (x, y),
            in_handle: None,
            out_handle: None,
            smooth: false,
        };
        let square = pictura_core::path::Subpath {
            points: vec![
                corner(10.0, 10.0),
                corner(30.0, 10.0),
                corner(30.0, 30.0),
                corner(10.0, 30.0),
            ],
            closed: true,
        };
        let data = pictura_codec::encode_vector_mask(&[square], 40, 40);
        let mask = pictura_codec::decode_vector_mask(&data, 40, 40).expect("mask");
        let rgba = vector_mask_thumbnail_rgba(&mask, 40, 40, 40, 40);
        let red = |x: usize, y: usize| rgba[(y * 40 + x) * 4];
        assert_eq!(red(20, 20), 255, "revealed");
        assert_eq!(red(2, 2), VECTOR_THUMB_HIDDEN, "hidden");
        assert_eq!(red(10, 20), 0, "left edge (the closing segment)");
        assert_eq!(red(20, 30), 0, "bottom edge");
        let open = VectorMask::default();
        assert!(vector_mask_thumbnail_rgba(&open, 40, 40, 40, 40)
            .chunks(4)
            .all(|px| px == [255, 255, 255, 255]));
    }

    #[test]
    fn extreme_document_vector_mask_thumbnail_aspect_is_clamped() {
        let mask = VectorMask::default();
        let wide = vector_mask_thumbnail_image(&mask, 200, 20, 24).expect("wide vector mask");
        assert_eq!((wide.width(), wide.height()), (24, 8));
        let tall = vector_mask_thumbnail_image(&mask, 20, 200, 24).expect("tall vector mask");
        assert_eq!((tall.width(), tall.height()), (8, 24));
        let hd = vector_mask_thumbnail_image(&mask, 1600, 900, 24).expect("16:9 vector mask");
        assert_eq!((hd.width(), hd.height()), (24, 14));
    }
}
