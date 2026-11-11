//! The canvas display source. The canvas crops come from the tile pyramid
//! (`cxxqt_object/tiles.rs`) and stay composite-only for every other reader,
//! so Channels, Histogram, Navigator, and the filter/adjustment previews never
//! see the solo mask. The solo mask is served by [`canvas_present_image`],
//! which only the canvas provider reads.

use super::super::helpers_composite::level_display_image;
use crate::cxxqt_object::qobject;
use crate::cxxqt_object::state::PictureViewRust;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QImage;
use pictura_core::{LayerMask, PixelBuffer, PsdRect};
use pictura_render::TileKey;

impl qobject::PictureView {
    /// A premultiplied crop of the **canvas display source** at `level` over
    /// `(x, y, w, h)`: the solo mask while a mask is soloed, else the document
    /// composite. Tiles a large document still lacks are stood in for and
    /// built on a worker. The canvas provider calls this; every other panel
    /// and dialog keeps calling `display_image` so they never see the mask.
    pub fn canvas_display_image(&self, level: i32, x: i32, y: i32, w: i32, h: i32) -> QImage {
        if w <= 0 || h <= 0 {
            return QImage::default();
        }
        let rect = PsdRect {
            top: y,
            left: x,
            bottom: y + h,
            right: x + w,
        };
        let (image, missing) = self.rust().canvas_present_image(level, rect);
        self.request_tiles(missing);
        image
    }
}

impl PictureViewRust {
    /// The canvas present image at `level` over `rect` (document pixels at
    /// that level): Quick Mask's overlay, the solo mask, or the composite,
    /// and the tiles left to a worker. Nothing but the canvas calls this.
    pub(crate) fn canvas_present_image(&self, level: i32, rect: PsdRect) -> (QImage, Vec<TileKey>) {
        if self.session.quick_mask.active {
            return self.quick_mask_crop(level, rect);
        }
        if !self.level_meets(level, rect) {
            return (QImage::default(), Vec::new());
        }
        let (crop, missing) = self.tile_crop(self.canvas_source(), level, rect);
        if crop.width() == 0 || crop.height() == 0 {
            return (QImage::default(), missing);
        }
        (level_display_image(&crop), missing)
    }

    /// A `max_side`-bounded grayscale thumbnail of the active layer's raster mask
    /// coverage, for the Channels panel's mask channel row; `None` without a
    /// document or an active layer carrying a raster mask. A disabled mask still
    /// shows its stored coverage (see [`mask_sample`]). Cost is O(thumbnail), not
    /// O(document): the panel refreshes this after every edit, so a full-pixel
    /// scan here is a per-edit UI stall (#1038).
    pub(crate) fn mask_channel_thumbnail(&self, max_side: u32) -> Option<(Vec<u8>, u32, u32)> {
        let doc = self.session.doc.as_ref()?;
        let path = self.layer_target.anchor()?;
        let mask = pictura_render::resolve_path(doc, path)?.mask.as_ref()?;
        mask_thumbnail_samples(mask, doc.width, doc.height, max_side)
    }
}

/// A `max_side`-bounded, aspect-kept grayscale thumbnail of `mask` over a
/// `width`×`height` document, one nearest-neighbour [`mask_sample`] per output
/// pixel. `None` for an empty document or `max_side`. Sampling the grid
/// directly is O(output) rather than O(document); the earlier build-a-full-plane
/// step took the same nearest-neighbour sample, so the pixels are unchanged.
fn mask_thumbnail_samples(
    mask: &LayerMask,
    width: u32,
    height: u32,
    max_side: u32,
) -> Option<(Vec<u8>, u32, u32)> {
    if width == 0 || height == 0 || max_side == 0 {
        return None;
    }
    let (tw, th) = if width >= height {
        (
            max_side,
            ((height as u64 * max_side as u64) / width as u64).max(1) as u32,
        )
    } else {
        (
            ((width as u64 * max_side as u64) / height as u64).max(1) as u32,
            max_side,
        )
    };
    let mut out = Vec::with_capacity((tw * th) as usize);
    for ty in 0..th {
        let sy = ty as usize * height as usize / th as usize;
        for tx in 0..tw {
            let sx = (tx as usize * width as usize / tw as usize).min(width as usize - 1);
            out.push(mask_sample(mask, sx as i32, sy as i32));
        }
    }
    Some((out, tw, th))
}

#[cfg(test)]
thread_local! {
    /// Test-only: mask samples touched since the last reset. Proves the Channels
    /// thumbnail samples its own grid rather than scanning the whole document.
    static MASK_SAMPLE_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The mask's stored coverage at a document pixel, for display: `data` in-rect,
/// `default_color` off-rect and for an empty rectangle, `255` for a data-less
/// mask. The enabled/disabled bit is deliberately ignored so the solo plane and
/// the Channels mask thumbnail keep showing the mask's raster even while the mask
/// is switched off (the compositor instead reveals where a disabled mask sits).
fn mask_sample(mask: &LayerMask, x: i32, y: i32) -> u8 {
    #[cfg(test)]
    MASK_SAMPLE_CALLS.with(|calls| calls.set(calls.get() + 1));
    let Some(data) = mask.data.as_ref() else {
        return 255;
    };
    let mw = i64::from(mask.rect.right) - i64::from(mask.rect.left);
    let mh = i64::from(mask.rect.bottom) - i64::from(mask.rect.top);
    if mw <= 0 || mh <= 0 {
        return mask.default_color;
    }
    let mx = i64::from(x) - i64::from(mask.rect.left);
    let my = i64::from(y) - i64::from(mask.rect.top);
    if mx < 0 || my < 0 || mx >= mw || my >= mh {
        return mask.default_color;
    }
    data.sample(my as usize * mw as usize + mx as usize)
        .unwrap_or(mask.default_color)
}

/// The document-sized grayscale mask plane: each sample replicated across RGB,
/// opaque. The grayscale value is the stored channel, not density/feather
/// (composite-time parameters). A whole-document restore rebuilds this, so it
/// fills one plane per core and touches only the pages it writes.
pub(in crate::cxxqt_object) fn mask_display_plane(
    mask: &LayerMask,
    width: u32,
    height: u32,
) -> PixelBuffer {
    use rayon::prelude::*;
    let mut buffer = PixelBuffer::zeroed(width, height, 4);
    let plane = buffer.pixel_count();
    if plane == 0 {
        return buffer;
    }
    let stride = width as usize;
    buffer
        .data
        .as_mut_slice()
        .par_chunks_mut(plane)
        .enumerate()
        .for_each(|(channel, out)| {
            if channel == 3 {
                out.fill(255);
                return;
            }
            out.par_chunks_mut(stride).enumerate().for_each(|(y, row)| {
                for (x, sample) in row.iter_mut().enumerate() {
                    *sample = mask_sample(mask, x as i32, y as i32);
                }
            });
        });
    buffer
}

/// The same grayscale plane over `clipped` only, sized and originated at the
/// rectangle: the level-0 source of the solo mask's display tiles, filled one
/// row per task so a large tile footprint is cheap.
pub(in crate::cxxqt_object) fn mask_region_plane(
    mask: &LayerMask,
    clipped: PsdRect,
) -> PixelBuffer {
    use rayon::prelude::*;
    let rw = clipped.width().max(0) as u32;
    let rh = clipped.height().max(0) as u32;
    let mut buffer = PixelBuffer::zeroed(rw, rh, 4);
    let plane = buffer.pixel_count();
    if plane == 0 {
        return buffer;
    }
    let data = buffer.data.as_mut_slice();
    let (gray, rest) = data.split_at_mut(plane);
    gray.par_chunks_mut(rw as usize)
        .enumerate()
        .for_each(|(oy, row)| {
            let y = clipped.top + oy as i32;
            for (ox, sample) in row.iter_mut().enumerate() {
                *sample = mask_sample(mask, clipped.left + ox as i32, y);
            }
        });
    let (green_blue, alpha) = rest.split_at_mut(2 * plane);
    green_blue[..plane].copy_from_slice(gray);
    green_blue[plane..].copy_from_slice(gray);
    alpha.fill(255);
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mask(rect: PsdRect, default_color: u8, disabled: bool, data: Option<Vec<u8>>) -> LayerMask {
        LayerMask {
            rect,
            default_color,
            disabled,
            flags: 0,
            data: data.map(Into::into),
            ..Default::default()
        }
    }

    const RECT: PsdRect = PsdRect {
        top: 0,
        left: 0,
        bottom: 2,
        right: 2,
    };

    #[test]
    fn mask_sample_shows_stored_coverage() {
        let live = mask(RECT, 7, false, Some(vec![0, 128, 255, 64]));
        assert_eq!(mask_sample(&live, 0, 0), 0);
        assert_eq!(mask_sample(&live, 1, 1), 64);
        assert_eq!(mask_sample(&live, 2, 0), 7, "off-rect is default_color");
        let disabled = mask(RECT, 7, true, Some(vec![0, 200, 255, 64]));
        assert_eq!(
            mask_sample(&disabled, 0, 0),
            0,
            "a disabled mask keeps its raster"
        );
        assert_eq!(
            mask_sample(&disabled, 1, 0),
            200,
            "stored coverage, not reveal-all"
        );
        let empty = mask(RECT, 9, false, None);
        assert_eq!(mask_sample(&empty, 0, 0), 255, "no data reveals");
    }

    #[test]
    fn mask_display_plane_is_opaque_grayscale() {
        let m = mask(RECT, 128, false, Some(vec![0, 128, 255, 64]));
        let plane = mask_display_plane(&m, 3, 2);
        let p = plane.pixel_count();
        assert_eq!((plane.width, plane.height, plane.channels), (3, 2, 4));
        let at = |x: usize, y: usize| -> [u8; 4] {
            let i = y * 3 + x;
            [
                plane.data[i],
                plane.data[p + i],
                plane.data[2 * p + i],
                plane.data[3 * p + i],
            ]
        };
        assert_eq!(at(1, 1), [64, 64, 64, 255], "in-rect sample, opaque");
        assert_eq!(at(2, 0), [128, 128, 128, 255], "off-rect default_color");
    }

    #[test]
    fn mask_display_plane_matches_a_serial_build() {
        let (w, h) = (9u32, 7u32);
        let rect = PsdRect {
            top: 2,
            left: 3,
            bottom: 5,
            right: 7,
        };
        let data: Vec<u8> = (0..12).map(|i| (i * 17) as u8).collect();
        let m = mask(rect, 40, false, Some(data));
        let plane = mask_display_plane(&m, w, h);
        let n = (w * h) as usize;
        assert_eq!(plane.data.len(), n * 4);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let v = mask_sample(&m, x, y);
                let i = y as usize * w as usize + x as usize;
                assert_eq!(
                    [
                        plane.data[i],
                        plane.data[n + i],
                        plane.data[2 * n + i],
                        plane.data[3 * n + i],
                    ],
                    [v, v, v, 255],
                    "at {x},{y}"
                );
            }
        }
    }

    #[test]
    fn mask_thumbnail_samples_match_the_full_plane_downsample() {
        // A document larger than the mask rect, so the off-rect default_color
        // shows through in the reference as well.
        let m = mask(RECT, 7, false, Some(vec![0, 128, 255, 64]));
        let (width, height, max_side) = (6u32, 4u32, 3u32);
        let (out, tw, th) = mask_thumbnail_samples(&m, width, height, max_side).expect("thumb");
        assert_eq!((tw, th), (3, 2));

        // The reference: the document-sized plane the old code built, then the
        // same nearest-neighbour sample.
        let mut plane = vec![0u8; (width * height) as usize];
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                plane[y as usize * width as usize + x as usize] = mask_sample(&m, x, y);
            }
        }
        let mut reference = Vec::new();
        for ty in 0..th {
            let sy = ty as usize * height as usize / th as usize;
            for tx in 0..tw {
                let sx = (tx as usize * width as usize / tw as usize).min(width as usize - 1);
                reference.push(plane[sy * width as usize + sx]);
            }
        }
        assert_eq!(out, reference, "the thumbnail is unchanged by the rewrite");
    }

    #[test]
    fn mask_thumbnail_samples_touch_only_the_grid() {
        // A 4096² document would cost 16.7 M samples if the whole document were
        // scanned; the thumbnail grid is 24×24. This is the regression guard for
        // the per-edit Channels-panel stall (#1038).
        let (width, height) = (4096u32, 4096u32);
        let m = mask(RECT, 7, false, Some(vec![0, 128, 255, 64]));
        MASK_SAMPLE_CALLS.with(|calls| calls.set(0));
        let (out, tw, th) = mask_thumbnail_samples(&m, width, height, 24).expect("thumb");
        let calls = MASK_SAMPLE_CALLS.with(std::cell::Cell::get);
        assert_eq!(out.len() as u32, tw * th);
        assert_eq!(calls, u64::from(tw * th), "one sample per thumbnail pixel");
        assert!(
            calls < u64::from(width) * u64::from(height) / 1000,
            "the thumbnail must not scan the document: {calls} samples"
        );
    }

    #[test]
    fn canvas_mask_thumbnail_samples_only_grid() {
        for missing in [false, true] {
            let mut m = mask(
                RECT,
                0,
                false,
                if missing {
                    None
                } else {
                    Some(vec![0, 1, 128, 255])
                },
            );
            let original = m.clone();
            let mut outputs = Vec::new();
            for disabled in [false, true] {
                m.disabled = disabled;
                MASK_SAMPLE_CALLS.with(|calls| calls.set(0));
                let output = pictura_core::assert_no_materialization(|| {
                    mask_thumbnail_samples(&m, 4096, 2048, 24).unwrap()
                });
                assert_eq!((output.1, output.2), (24, 12));
                assert_eq!(MASK_SAMPLE_CALLS.with(std::cell::Cell::get), 24 * 12);
                if missing {
                    assert!(output.0.iter().all(|&v| v == 255));
                }
                outputs.push(output);
            }
            assert_eq!(outputs[0], outputs[1]);
            m.disabled = false;
            assert_eq!(m, original);
        }
    }
}
