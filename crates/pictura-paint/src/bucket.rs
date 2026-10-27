//! The Paint Bucket's engine (`docs/03-tools/gradient-and-paint-bucket.md`).
//!
//! The Paint Bucket is the Magic Wand with paint instead of a selection:
//! [`flood`] answers "which pixels belong with the one clicked?" as a coverage
//! mask, and [`fill`] lays the foreground colour or a pattern through it.
//!
//! Unlike the Magic Wand's flood, this one counts **alpha** as a channel, so a
//! fill inside a drawn outline on a transparent layer stops at the outline
//! instead of treating empty pixels as black.
//!
//! Ported from photorust's `core/src/bucket.rs` and `core/src/wand.rs`
//! (<https://github.com/perfecto25/photorust>), extended with the pattern
//! fill. Behavioural parity only.

use crate::fill::{fill_layer, Alpha};
use crate::healing::RgbaImage;
use crate::{PaintMode, Rgba};
use pictura_core::mask_ops::{self, Border};
use pictura_core::{Document, PsdRect};

/// What the Paint Bucket fills with: the options bar's Fill menu.
#[derive(Clone, Copy, Debug)]
pub enum BucketPaint<'a> {
    Foreground(Rgba),
    /// A tile repeated from the document origin.
    Pattern(&'a RgbaImage),
}

/// The pixels of `surface` that match the one at `seed` within `tolerance`
/// (0–255 per channel, alpha included, inclusive): those 4-connected to it
/// when `contiguous`, otherwise every one. A row-major 0 / 255 mask the size
/// of `surface`; `None` for a seed outside it.
pub fn flood(
    surface: &RgbaImage,
    seed: (i32, i32),
    tolerance: u8,
    contiguous: bool,
) -> Option<Vec<u8>> {
    let (w, h) = (surface.width, surface.height);
    if seed.0 < 0 || seed.1 < 0 || seed.0 >= w || seed.1 >= h {
        return None;
    }
    let target = surface.get(seed.0, seed.1);
    // The largest difference on any one channel: one badly-off channel is
    // enough to reject a pixel.
    let matches = |p: &[u8; 4]| {
        p.iter()
            .zip(&target)
            .all(|(a, b)| a.abs_diff(*b) <= tolerance)
    };
    if !contiguous {
        return Some(
            surface
                .data
                .iter()
                .map(|p| matches(p) as u8 * 255)
                .collect(),
        );
    }
    let mut seeds = vec![0u8; (w * h) as usize];
    seeds[(seed.1 * w + seed.0) as usize] = 1;
    mask_ops::flood(w as usize, h as usize, &seeds, |_, to| {
        matches(&surface.data[to])
    })
}

/// Fill the pixel layer at `path` with `paint` through a document-sized
/// coverage `mask` (0–255, row-major over `doc.width`) at `opacity` (0.0–1.0)
/// in `mode`, inside `selection` (another such mask) when there is one. The
/// `alpha` policy says what the fill does with the pixels' transparency; a
/// transparency lock overrides it and keeps every pixel's alpha. Returns the
/// document rectangle changed; `None` when nothing changed or the layer cannot
/// be filled.
#[allow(clippy::too_many_arguments)]
pub fn fill(
    doc: &mut Document,
    path: &str,
    mask: &[u8],
    paint: BucketPaint<'_>,
    mode: PaintMode,
    opacity: f32,
    selection: Option<&[u8]>,
    alpha: Alpha,
) -> Option<PsdRect> {
    let opacity = opacity.clamp(0.0, 1.0);
    let width = doc.width as i32;
    let empty_tile = matches!(paint, BucketPaint::Pattern(t) if t.width <= 0 || t.height <= 0);
    if opacity <= 0.0 || empty_tile || mask.len() < (width * doc.height as i32) as usize {
        return None;
    }
    fill_layer(doc, path, mode, selection, alpha, |x, y| {
        let cover = mask[(y * width + x) as usize];
        if cover == 0 {
            return None;
        }
        let color = match paint {
            BucketPaint::Foreground(color) => color,
            BucketPaint::Pattern(tile) => {
                let [r, g, b, a] = tile.get(x.rem_euclid(tile.width), y.rem_euclid(tile.height));
                Rgba { r, g, b, a }
            }
        };
        Some((color, cover as f32 / 255.0 * opacity))
    })
}

/// The stroke band around a selection: a solid `color` laid over the pixels
/// within `width` px of the selection edge, `Inside` / `Outside` / `Center`
/// choosing which side. `selection` is a document-sized 0–255 coverage mask.
/// Returns the document rectangle changed; `None` when nothing changed, the
/// selection is empty, or the layer cannot be filled.
///
/// ponytail: the band is built from integer max/min (dilate/erode) filters, so
/// it is a square-capped approximation of CS6's round stroke; the brush-mode
/// blend rides through `fill_layer` and `opacity` is `0.0`–`1.0`.
#[allow(clippy::too_many_arguments)]
pub fn stroke_selection(
    doc: &mut Document,
    path: &str,
    selection: &[u8],
    color: Rgba,
    width: u32,
    position: StrokeAlign,
    mode: PaintMode,
    opacity: f32,
) -> Option<PsdRect> {
    let (w, h) = (doc.width as i32, doc.height as i32);
    if width == 0 || opacity <= 0.0 || selection.len() < (w * h) as usize {
        return None;
    }
    if !selection.iter().any(|&v| v > 0) {
        return None;
    }
    let selection = &selection[..(w * h) as usize];
    let radius = width.min(u32::from(u16::MAX)) as i32;
    let (out_r, in_r) = match position {
        StrokeAlign::Outside => (radius, 0),
        StrokeAlign::Inside => (0, radius),
        StrokeAlign::Center => ((radius + 1) / 2, radius / 2),
    };
    let dilated = mask_ops::dilate(
        selection,
        w as usize,
        h as usize,
        out_r.max(0) as usize,
        Border::Clamp,
    );
    let eroded = mask_ops::erode(
        selection,
        w as usize,
        h as usize,
        in_r.max(0) as usize,
        Border::Clamp,
    );
    let band: Vec<u8> = selection
        .iter()
        .zip(&dilated)
        .zip(&eroded)
        .map(|((&m, &d), &e)| match position {
            StrokeAlign::Outside => d.saturating_sub(m),
            StrokeAlign::Inside => m.saturating_sub(e),
            StrokeAlign::Center => d.saturating_sub(e),
        })
        .collect();
    fill(
        doc,
        path,
        &band,
        BucketPaint::Foreground(color),
        mode,
        opacity,
        None,
        Alpha::Blend,
    )
}

/// Which side of the selection edge the stroke band covers.
#[derive(Clone, Copy, Debug)]
pub enum StrokeAlign {
    Inside,
    Outside,
    Center,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill::tests::{doc, lock, pixel, RED};
    use crate::stamp::layer_surface;
    use pictura_core::LockFlags;

    const WHITE: [u8; 4] = [255; 4];
    const BLACK: [u8; 4] = [0, 0, 0, 255];

    /// Two white rooms with a 2 px black wall between them.
    fn rooms() -> Document {
        let mut d = doc(40, 20, WHITE);
        for c in &mut d.layers[0].channels[..3] {
            for (i, v) in c.data.iter_mut().enumerate() {
                if (19..21).contains(&(i % 40)) {
                    *v = 0;
                }
            }
        }
        d
    }

    /// Flood `d` from `seed` and fill it red.
    fn bucket(d: &mut Document, seed: (i32, i32), tolerance: u8, contiguous: bool) {
        let mask = flood(&layer_surface(d, "0").unwrap(), seed, tolerance, contiguous).unwrap();
        let red = BucketPaint::Foreground(RED);
        fill(
            d,
            "0",
            &mask,
            red,
            PaintMode::Normal,
            1.0,
            None,
            Alpha::Blend,
        );
    }

    /// A `w × h` document whose selection covers the rectangle at `(left, top)`.
    fn selected(w: i32, h: i32, left: i32, top: i32, sw: i32, sh: i32) -> (Document, Vec<u8>) {
        let d = doc(w, h, WHITE);
        let mut selection = vec![0u8; (w * h) as usize];
        for y in top..top + sh {
            for x in left..left + sw {
                selection[(y * w + x) as usize] = 255;
            }
        }
        (d, selection)
    }

    #[test]
    fn the_outside_band_stops_at_the_selections_edge() {
        // An 8×1 row: the selection covers the middle four pixels.
        let (mut d, selection) = selected(8, 1, 2, 0, 4, 1);
        stroke_selection(
            &mut d,
            "0",
            &selection,
            RED,
            1,
            StrokeAlign::Outside,
            PaintMode::Normal,
            1.0,
        )
        .expect("outside stroke");
        assert_eq!(pixel(&d, 1, 0), [220, 0, 0, 255], "the band's left arm");
        assert_eq!(pixel(&d, 6, 0), [220, 0, 0, 255], "the band's right arm");
        assert_eq!(pixel(&d, 7, 0), WHITE, "the band reached past the edge");
    }

    #[test]
    fn the_inside_band_stops_at_the_selections_edge() {
        let (mut d, selection) = selected(8, 1, 2, 0, 4, 1);
        stroke_selection(
            &mut d,
            "0",
            &selection,
            RED,
            1,
            StrokeAlign::Inside,
            PaintMode::Normal,
            1.0,
        )
        .expect("inside stroke");
        assert_eq!(pixel(&d, 2, 0), [220, 0, 0, 255], "the band's left arm");
        assert_eq!(pixel(&d, 5, 0), [220, 0, 0, 255], "the band's right arm");
        assert_eq!(pixel(&d, 3, 0), WHITE, "the band covered the interior");
        assert_eq!(pixel(&d, 6, 0), WHITE, "the band reached past the edge");
    }

    #[test]
    fn the_band_does_not_reach_the_canvas_edge() {
        // A 10×10 selection with room to its right on a 24×24 canvas.
        let (mut d, selection) = selected(24, 24, 7, 7, 10, 10);
        stroke_selection(
            &mut d,
            "0",
            &selection,
            RED,
            2,
            StrokeAlign::Outside,
            PaintMode::Normal,
            1.0,
        )
        .expect("outside stroke");
        assert_eq!(pixel(&d, 5, 12), [220, 0, 0, 255], "the band's left arm");
        assert_eq!(pixel(&d, 17, 12), [220, 0, 0, 255], "the band's right arm");
        assert_eq!(pixel(&d, 19, 12), WHITE, "the band reached past the edge");
        assert_eq!(pixel(&d, 23, 23), WHITE, "the band reached the corner");
    }

    #[test]
    fn a_contiguous_fill_stops_at_the_wall_and_a_global_one_does_not() {
        let mut d = rooms();
        bucket(&mut d, (5, 10), 32, true);
        assert_eq!(pixel(&d, 5, 10), [220, 0, 0, 255]);
        assert_eq!(pixel(&d, 30, 10), WHITE, "the fill leaked past the wall");
        assert_eq!(pixel(&d, 19, 10), BLACK, "the wall itself was filled");

        let mut d = rooms();
        bucket(&mut d, (5, 10), 32, false);
        assert_eq!(pixel(&d, 30, 10), [220, 0, 0, 255]);
        assert_eq!(pixel(&d, 20, 10), BLACK);
    }

    #[test]
    fn tolerance_is_inclusive_per_channel() {
        let patched = || {
            let mut d = doc(20, 20, [100, 100, 100, 255]);
            // A patch 20 levels off on one channel only.
            for (i, v) in d.layers[0].channels[1].data.iter_mut().enumerate() {
                if i % 20 >= 10 {
                    *v = 120;
                }
            }
            d
        };
        let mut tight = patched();
        bucket(&mut tight, (2, 10), 19, true);
        assert_eq!(pixel(&tight, 15, 10), [100, 120, 100, 255]);
        let mut exact = patched();
        bucket(&mut exact, (2, 10), 20, true);
        assert_eq!(pixel(&exact, 15, 10), [220, 0, 0, 255]);
    }

    #[test]
    fn transparency_is_not_black() {
        // A black outline on an empty layer: the inside fills, the outline and
        // the outside do not.
        let mut d = doc(20, 20, [0; 4]);
        for y in 5..15 {
            for x in 5..15 {
                if x == 5 || x == 14 || y == 5 || y == 14 {
                    d.layers[0].channels[3].data[y * 20 + x] = 255;
                }
            }
        }
        bucket(&mut d, (10, 10), 32, true);
        assert_eq!(pixel(&d, 10, 10), [220, 0, 0, 255]);
        assert_eq!(pixel(&d, 5, 10), BLACK);
        assert_eq!(pixel(&d, 1, 1)[3], 0);
    }

    #[test]
    fn a_seed_off_the_surface_floods_nothing() {
        let surface = layer_surface(&rooms(), "0").unwrap();
        assert!(flood(&surface, (40, 10), 32, true).is_none());
        assert!(flood(&surface, (-1, 10), 32, false).is_none());
    }

    #[test]
    fn opacity_and_the_mask_scale_the_fill() {
        let mut d = doc(4, 1, WHITE);
        let black = BucketPaint::Foreground(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        });
        let dirty = fill(
            &mut d,
            "0",
            &[255, 128, 0, 0],
            black,
            PaintMode::Normal,
            0.5,
            None,
            Alpha::Blend,
        )
        .expect("filled");
        assert_eq!((dirty.left, dirty.right), (0, 2));
        assert!((pixel(&d, 0, 0)[0] as i32 - 128).abs() <= 1);
        assert!((pixel(&d, 1, 0)[0] as i32 - 191).abs() <= 1);
        assert_eq!(pixel(&d, 2, 0), WHITE);
        assert!(fill(
            &mut d,
            "0",
            &[255; 4],
            black,
            PaintMode::Normal,
            0.0,
            None,
            Alpha::Blend
        )
        .is_none());
        assert!(
            fill(
                &mut d,
                "0",
                &[255; 3],
                black,
                PaintMode::Normal,
                1.0,
                None,
                Alpha::Blend
            )
            .is_none(),
            "a short mask was read"
        );
    }

    #[test]
    fn a_transparency_lock_keeps_the_fill_off_empty_pixels() {
        let mut d = doc(20, 20, [255, 255, 255, 0]);
        for a in &mut d.layers[0].channels[3].data[..200] {
            *a = 255;
        }
        lock(&mut d, LockFlags::TRANSPARENCY);
        let red = BucketPaint::Foreground(RED);
        fill(
            &mut d,
            "0",
            &[255; 400],
            red,
            PaintMode::Normal,
            1.0,
            None,
            Alpha::Blend,
        )
        .expect("filled");
        assert_eq!(pixel(&d, 10, 5), [220, 0, 0, 255]);
        assert_eq!(pixel(&d, 10, 15)[3], 0, "an empty pixel gained coverage");
    }

    #[test]
    fn a_pattern_tiles_from_the_document_origin_inside_the_selection() {
        let tile = RgbaImage {
            width: 2,
            height: 1,
            data: vec![[10, 10, 10, 255], [200, 200, 200, 255]],
        };
        let mut d = doc(6, 2, WHITE);
        let selection = [255, 255, 255, 255, 0, 0].repeat(2);
        fill(
            &mut d,
            "0",
            &[255; 12],
            BucketPaint::Pattern(&tile),
            PaintMode::Normal,
            1.0,
            Some(&selection),
            Alpha::Blend,
        )
        .expect("filled");
        assert_eq!(pixel(&d, 0, 0)[0], 10);
        assert_eq!(pixel(&d, 1, 1)[0], 200);
        assert_eq!(pixel(&d, 2, 1)[0], 10);
        assert_eq!(pixel(&d, 4, 0), WHITE, "the fill escaped the selection");

        let empty = RgbaImage {
            width: 0,
            height: 0,
            data: Vec::new(),
        };
        let paint = BucketPaint::Pattern(&empty);
        assert!(fill(
            &mut d,
            "0",
            &[255; 12],
            paint,
            PaintMode::Normal,
            1.0,
            None,
            Alpha::Blend
        )
        .is_none());
    }

    #[test]
    fn a_selection_stroke_bands_the_edge_where_asked() {
        let red = RED;
        // An 8×1 row: selection covers the middle four pixels.
        let selection = [0, 0, 255, 255, 255, 255, 0, 0];
        let mut d = doc(8, 1, WHITE);

        // Outside 1 px: the two pixels just outside the selection take the
        // colour; the selection itself is untouched.
        stroke_selection(
            &mut d,
            "0",
            &selection,
            red,
            1,
            StrokeAlign::Outside,
            PaintMode::Normal,
            1.0,
        )
        .expect("outside stroke");
        assert_eq!(pixel(&d, 1, 0), [220, 0, 0, 255]);
        assert_eq!(pixel(&d, 6, 0), [220, 0, 0, 255]);
        assert_eq!(pixel(&d, 2, 0), WHITE, "the selection was painted");
        assert_eq!(pixel(&d, 0, 0), WHITE, "the band spread too far");

        // Inside 1 px: the selection's border takes the colour; its middle and
        // the outside stay white.
        let mut d = doc(8, 1, WHITE);
        stroke_selection(
            &mut d,
            "0",
            &selection,
            red,
            1,
            StrokeAlign::Inside,
            PaintMode::Normal,
            1.0,
        )
        .expect("inside stroke");
        assert_eq!(pixel(&d, 2, 0), [220, 0, 0, 255]);
        assert_eq!(pixel(&d, 5, 0), [220, 0, 0, 255]);
        assert_eq!(pixel(&d, 1, 0), WHITE);
    }
}
