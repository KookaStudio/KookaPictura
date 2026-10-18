//! The pass the Gradient and the Paint Bucket share: every pixel of one layer
//! asks a source for its colour and coverage, and the result is laid down in a
//! Brush mode through the selection.

use crate::replace::grow;
use crate::stroke::{blend_pixel, layer_at_mut, parse_layer_path, union_rect};
use crate::{PaintMode, Rgba};
use pictura_core::{
    layer_pixel_locked, layer_transparency_locked, BitDepth, ColorMode, Document, PsdRect,
};
use rayon::prelude::*;

/// Fill the pixel layer at `path`. `source` is asked, per document pixel, for
/// the colour to lay and its coverage (0.0–1.0); `None` leaves the pixel
/// alone. `selection` is a document-sized coverage mask (0–255, row-major);
/// without one the whole layer is filled. A transparency lock keeps every
/// pixel's alpha. Returns the document rectangle changed, or `None` when
/// nothing changed or the layer cannot be filled: a group, an adjustment,
/// locked pixels, or Clear under a transparency lock.
///
/// Rows run on every core: `source` sees each pixel once, in no fixed order,
/// so it must not depend on the pixels it was asked about before.
// ponytail: 8-bit RGB only, within the layer's own rectangle.
pub(crate) fn fill_layer(
    doc: &mut Document,
    path: &str,
    mode: PaintMode,
    selection: Option<&[u8]>,
    source: impl Fn(i32, i32) -> Option<(Rgba, f32)> + Sync,
) -> Option<PsdRect> {
    if doc.depth != BitDepth::Eight || doc.mode != ColorMode::Rgb {
        return None;
    }
    let canvas = (doc.width as i32, doc.height as i32);
    if selection.is_some_and(|s| s.len() < (canvas.0 * canvas.1) as usize) {
        return None;
    }
    let layer = layer_at_mut(doc, &parse_layer_path(path)?)?;
    if layer.is_group || layer.adjustment.is_some() || layer_pixel_locked(layer) {
        return None;
    }
    let keep_alpha = layer_transparency_locked(layer);
    if keep_alpha && mode == PaintMode::Clear {
        return None;
    }
    let rect = layer.rect;
    let w = rect.width().max(0);
    let n = (w * rect.height().max(0)) as usize;
    let (y0, y1) = (rect.top.max(0), rect.bottom.min(canvas.1));
    let (x0, x1) = (rect.left.max(0), rect.right.min(canvas.0));
    if y1 <= y0 || x1 <= x0 {
        return None;
    }
    // The R, G, B and alpha planes, each a whole layer long; a missing colour
    // plane refuses the fill, a missing alpha is an opaque layer.
    let mut planes: [Option<&mut [u8]>; 4] = [None, None, None, None];
    for c in layer.channels.iter_mut() {
        let k = match c.id {
            0 => 0,
            1 => 1,
            2 => 2,
            -1 => 3,
            _ => continue,
        };
        if c.data.len() == n && planes[k].is_none() {
            planes[k] = Some(c.data.as_mut_slice());
        }
    }
    let [Some(r), Some(g), Some(b), alpha] = planes else {
        return None;
    };
    let row = |ly: i32| {
        let a = (ly * w) as usize;
        a..a + w as usize
    };
    // Only the layer rows on the canvas are cut into the row tasks.
    let span = row(y0 - rect.top).start..row(y1 - 1 - rect.top).end;
    let w_us = w as usize;
    let rows = r[span.clone()]
        .chunks_mut(w_us)
        .zip(g[span.clone()].chunks_mut(w_us))
        .zip(b[span.clone()].chunks_mut(w_us))
        .zip(rows_of(alpha.map(|a| &mut a[span]), w_us))
        .enumerate()
        .map(|(k, (((r, g), b), a))| (y0 + k as i32, r, g, b, a))
        .collect::<Vec<_>>();
    rows.into_par_iter()
        .filter_map(|(y, r, g, b, mut a)| {
            let mut dirty = None;
            for x in x0..x1 {
                let selected = selection.map_or(255, |s| s[(y * canvas.0 + x) as usize]);
                if selected == 0 {
                    continue;
                }
                let Some((color, cover)) = source(x, y) else {
                    continue;
                };
                let mut strength = cover.clamp(0.0, 1.0) * selected as f32 / 255.0;
                let mut color = color;
                if mode == PaintMode::Dissolve {
                    // A see-through colour dissolves into fewer pixels, not fainter ones.
                    strength *= color.a as f32 / 255.0;
                    color.a = 255;
                }
                if strength <= 0.0 {
                    continue;
                }
                let i = (x - rect.left) as usize;
                let dst = (r[i], g[i], b[i], a.as_ref().map_or(255, |a| a[i]));
                if keep_alpha && dst.3 == 0 {
                    continue;
                }
                let Some(out) = blend_pixel(mode, dst, &color, strength, pixel_noise(x, y)) else {
                    continue;
                };
                let out_alpha = if keep_alpha || a.is_none() {
                    dst.3
                } else {
                    out.3
                };
                if (out.0, out.1, out.2, out_alpha) == dst {
                    continue;
                }
                r[i] = out.0;
                g[i] = out.1;
                b[i] = out.2;
                if let Some(a) = a.as_deref_mut() {
                    a[i] = out_alpha;
                }
                dirty = Some(grow(dirty, x, y));
            }
            dirty
        })
        .reduce_with(union_rect)
}

/// `plane` cut into rows of `w`, or endless `None`s for an absent plane.
fn rows_of(plane: Option<&mut [u8]>, w: usize) -> impl Iterator<Item = Option<&mut [u8]>> {
    let mut rows = plane.map(|p| p.chunks_mut(w));
    std::iter::from_fn(move || {
        Some(match rows.as_mut() {
            Some(rows) => Some(rows.next()?),
            None => None,
        })
    })
}

/// A fixed value in `0.0..1.0` for a pixel: the same fill must come out the
/// same every time, so its Dissolve and dither cannot use a generator.
pub(crate) fn pixel_noise(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    (h & 0xFFFF) as f32 / 65_536.0
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::stroke::layer_rgba;
    use pictura_core::{Channel, Layer, LockFlags};

    pub(crate) const RED: Rgba = Rgba {
        r: 220,
        g: 0,
        b: 0,
        a: 255,
    };

    /// A `w × h` document with one layer of `rgba` at `(left, top)`; a
    /// Background (no alpha channel) when `background`.
    pub(crate) fn doc_at(
        (w, h): (i32, i32),
        (left, top): (i32, i32),
        rgba: [u8; 4],
        background: bool,
    ) -> Document {
        let mut doc = Document::new(w as u32, h as u32, ColorMode::Rgb, BitDepth::Eight);
        let ids: &[i16] = if background {
            &[0, 1, 2]
        } else {
            &[0, 1, 2, -1]
        };
        doc.layers.push(Layer {
            rect: PsdRect {
                top,
                left,
                bottom: top + h,
                right: left + w,
            },
            channels: ids
                .iter()
                .map(|&id| Channel {
                    id,
                    data: vec![rgba[if id < 0 { 3 } else { id as usize }]; (w * h) as usize].into(),
                })
                .collect(),
            background,
            ..Default::default()
        });
        doc
    }

    pub(crate) fn doc(w: i32, h: i32, rgba: [u8; 4]) -> Document {
        doc_at((w, h), (0, 0), rgba, false)
    }

    /// Layer-local pixel `(x, y)` of the first layer.
    pub(crate) fn pixel(doc: &Document, x: i32, y: i32) -> [u8; 4] {
        layer_rgba(&doc.layers[0]).get(x, y)
    }

    pub(crate) fn lock(doc: &mut Document, flag: u8) {
        doc.layers[0].lock = LockFlags::default().with(flag, true);
    }

    fn flood(doc: &mut Document, mode: PaintMode, selection: Option<&[u8]>) -> Option<PsdRect> {
        fill_layer(doc, "0", mode, selection, |_, _| Some((RED, 1.0)))
    }

    #[test]
    fn a_fill_covers_the_layer_and_reports_what_changed() {
        let mut d = doc(8, 4, [255; 4]);
        let dirty = flood(&mut d, PaintMode::Normal, None).expect("filled");
        assert_eq!(
            (dirty.left, dirty.top, dirty.right, dirty.bottom),
            (0, 0, 8, 4)
        );
        assert_eq!(pixel(&d, 7, 3), [220, 0, 0, 255]);
        assert!(
            flood(&mut d, PaintMode::Normal, None).is_none(),
            "an unchanged layer reported a change"
        );
    }

    #[test]
    fn the_selection_confines_and_scales_the_fill() {
        let mut d = doc(4, 1, [255; 4]);
        let dirty = flood(&mut d, PaintMode::Normal, Some(&[255, 128, 0, 0])).expect("filled");
        assert_eq!((dirty.left, dirty.right), (0, 2));
        assert_eq!(pixel(&d, 0, 0), [220, 0, 0, 255]);
        let half = pixel(&d, 1, 0);
        assert!((half[1] as i32 - 127).abs() <= 1, "green {}", half[1]);
        assert_eq!(pixel(&d, 2, 0), [255; 4]);
        assert!(
            flood(&mut d, PaintMode::Normal, Some(&[255])).is_none(),
            "a short selection was read"
        );
    }

    #[test]
    fn a_layer_is_filled_in_document_space() {
        // The layer sits at (3, 0) and hangs off the 8 px canvas.
        let mut d = doc_at((8, 2), (3, 0), [255; 4], false);
        let asked = std::sync::Mutex::new(Vec::new());
        let dirty = fill_layer(&mut d, "0", PaintMode::Normal, None, |x, y| {
            asked.lock().unwrap().push((x, y));
            (x == 5).then_some((RED, 1.0))
        })
        .expect("filled");
        let asked = asked.into_inner().unwrap();
        assert_eq!((dirty.left, dirty.right), (5, 6));
        assert_eq!(pixel(&d, 2, 1), [220, 0, 0, 255]);
        assert!(asked.iter().all(|&(x, _)| (3..8).contains(&x)));
    }

    #[test]
    fn a_transparency_lock_keeps_alpha_and_refuses_clear() {
        let mut d = doc(4, 1, [10, 10, 10, 0]);
        d.layers[0].channels[3].data[0] = 200;
        lock(&mut d, LockFlags::TRANSPARENCY);
        flood(&mut d, PaintMode::Normal, None).expect("filled");
        assert_eq!(pixel(&d, 0, 0), [220, 0, 0, 200]);
        assert_eq!(
            pixel(&d, 1, 0),
            [10, 10, 10, 0],
            "an empty pixel gained colour"
        );
        assert!(flood(&mut d, PaintMode::Clear, None).is_none());
    }

    #[test]
    fn locked_pixels_a_group_and_other_modes_are_refused() {
        let mut locked = doc(4, 1, [255; 4]);
        lock(&mut locked, LockFlags::PIXELS);
        assert!(flood(&mut locked, PaintMode::Normal, None).is_none());

        let mut group = doc(4, 1, [255; 4]);
        group.layers[0].is_group = true;
        assert!(flood(&mut group, PaintMode::Normal, None).is_none());

        let mut deep = doc(4, 1, [255; 4]);
        deep.depth = BitDepth::Sixteen;
        assert!(flood(&mut deep, PaintMode::Normal, None).is_none());
    }

    #[test]
    fn behind_fills_only_what_is_not_opaque_and_clear_erases() {
        let mut d = doc(2, 1, [255; 4]);
        d.layers[0].channels[3].data[1] = 0;
        flood(&mut d, PaintMode::Behind, None).expect("filled");
        assert_eq!(pixel(&d, 0, 0), [255; 4]);
        assert_eq!(pixel(&d, 1, 0), [220, 0, 0, 255]);

        flood(&mut d, PaintMode::Clear, None).expect("cleared");
        assert_eq!(pixel(&d, 0, 0)[3], 0);

        let mut background = doc_at((2, 1), (0, 0), [255; 4], true);
        assert!(
            flood(&mut background, PaintMode::Clear, None).is_none(),
            "a layer without alpha was cleared"
        );
    }

    #[test]
    fn dissolve_paints_whole_pixels_in_proportion_and_repeats() {
        let run = || {
            let mut d = doc(64, 64, [255; 4]);
            let see_through = Rgba { a: 128, ..RED };
            fill_layer(&mut d, "0", PaintMode::Dissolve, None, |_, _| {
                Some((see_through, 0.5))
            });
            d
        };
        let d = run();
        let painted = layer_rgba(&d.layers[0])
            .data
            .iter()
            .inspect(|px| assert!(**px == [220, 0, 0, 255] || **px == [255; 4]))
            .filter(|px| **px != [255; 4])
            .count();
        // Half strength of a half-opaque colour: about a quarter of 4096.
        assert!((800..1250).contains(&painted), "{painted} pixels");
        assert_eq!(layer_rgba(&run().layers[0]), layer_rgba(&d.layers[0]));
    }
}
