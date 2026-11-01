//! The canvas display source. The composite pyramid crops live here (moved out
//! of `impl_core.rs` to keep it under its size cap) and stay composite-only, so
//! Channels, Histogram, Navigator, and the filter/adjustment previews never see
//! the solo mask. The solo mask is served by [`canvas_present_image`], which
//! only the canvas provider and the present/region paths read.

use super::super::helpers_composite::{level_display_image, patch_buffer_region, planes_of};
use crate::cxxqt_object::qobject;
use crate::cxxqt_object::state::PictureViewRust;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{c_void, QImage, QImageFormat};
use pictura_core::{LayerMask, PixelBuffer, PsdRect};
use pictura_render::{LevelView, ViewPyramid};

impl qobject::PictureView {
    /// A premultiplied crop of the **canvas display source** at `level` over
    /// `(x, y, w, h)`: the solo mask plane when a mask is soloed, else the
    /// document composite. The canvas provider calls this; every other panel and
    /// dialog keeps calling `display_image` so they never see the mask.
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
        self.rust().canvas_present_image(level, rect)
    }

    /// A zero-copy display image of stored pyramid `level` (premultiplied
    /// RGBA8888), or an empty image for level 0 or past the last level. The
    /// handle behind the image is dropped with the image.
    pub fn canvas_level_image(&self, level: i32) -> QImage {
        if level <= 0 {
            return QImage::default();
        }
        let rust = self.rust();
        let handle = if rust.mask_solo_active() {
            rust.mask_pyramid.level_view(level as usize)
        } else {
            rust.pyramid.level_view(level as usize)
        };
        handle.map_or_else(QImage::default, level_view_image)
    }
}

/// Wrap a shared level handle as a `QImage`; the cleanup drops the handle.
fn level_view_image(view: LevelView) -> QImage {
    let width = view.width() as i32;
    let height = view.height() as i32;
    let expected = width as usize * height as usize * 4;
    debug_assert_eq!(view.data().len(), expected, "a stored level is packed");
    debug_assert_eq!(
        view.data().as_ptr() as usize % 4,
        0,
        "an external QImage needs 4-byte alignment"
    );
    let data = view.data().as_ptr();
    let info = Box::into_raw(Box::new(view)).cast::<c_void>();
    // SAFETY: the handle is alive until `release_level_view` drops it, and the
    // level's storage is packed premultiplied RGBA8888.
    unsafe {
        QImage::from_raw_parts(
            data,
            width,
            height,
            QImageFormat::Format_RGBA8888_Premultiplied,
            release_level_view,
            info,
        )
    }
}

extern "C" fn release_level_view(info: *mut c_void) {
    // SAFETY: `info` is the `Box<LevelView>` made in `level_view_image`.
    unsafe { drop(Box::from_raw(info.cast::<LevelView>())) }
}

impl PictureViewRust {
    /// A premultiplied crop of `pyramid` `level` at `rect` from `level0`, or an
    /// empty image for an out-of-range level, a non-intersecting rectangle, or
    /// an absent source. A partial overlap is transparent where it falls
    /// outside.
    fn crop_source(
        pyramid: &ViewPyramid,
        level0: Option<&PixelBuffer>,
        level: i32,
        rect: PsdRect,
    ) -> QImage {
        if level < 0 {
            return QImage::default();
        }
        let level = level as usize;
        if level >= pyramid.level_count() {
            return QImage::default();
        }
        let (lw, lh) = pyramid.level_size(level);
        if rect.right <= 0 || rect.bottom <= 0 || rect.left >= lw as i32 || rect.top >= lh as i32 {
            return QImage::default();
        }
        let Some(planes) = level0.and_then(planes_of) else {
            return QImage::default();
        };
        let crop = pyramid.crop(planes, level, rect);
        if crop.width() == 0 || crop.height() == 0 {
            return QImage::default();
        }
        level_display_image(&crop)
    }

    pub(crate) fn display_crop(&self, level: i32, rect: PsdRect) -> QImage {
        Self::crop_source(&self.pyramid, self.level0.as_ref(), level, rect)
    }

    pub(crate) fn display_level_count(&self) -> i32 {
        self.pyramid.level_count() as i32
    }

    pub(crate) fn display_level_size(&self, level: i32) -> Option<(u32, u32)> {
        if level < 0 || level as usize >= self.pyramid.level_count() {
            return None;
        }
        Some(self.pyramid.level_size(level as usize))
    }

    /// Whether the canvas is currently presenting the solo mask plane.
    pub(crate) fn mask_solo_active(&self) -> bool {
        self.mask_solo.is_some() && self.mask_level0.is_some()
    }

    /// The canvas present image at `level` over `rect`: the solo mask crop when
    /// soloing, else the document composite. Nothing but the canvas calls this.
    /// Falls back to the composite if the soloed mask's source is absent (e.g.
    /// the mask was just deleted), so the canvas never goes blank.
    pub(crate) fn canvas_present_image(&self, level: i32, rect: PsdRect) -> QImage {
        match (self.mask_solo.as_ref(), self.mask_level0.as_ref()) {
            (Some(_), Some(level0)) => {
                Self::crop_source(&self.mask_pyramid, Some(level0), level, rect)
            }
            _ => self.display_crop(level, rect),
        }
    }

    /// (Re)build the solo mask plane and its pyramid from the current document,
    /// or drop them when there is no soloable mask. A no-op source change.
    pub(crate) fn rebuild_mask_solo_source(&mut self) {
        let path = self.mask_solo.clone();
        let built = path.and_then(|path| {
            let doc = self.session.doc.as_ref()?;
            let mask = pictura_render::resolve_path(doc, &path)?.mask.as_ref()?;
            Some(mask_display_plane(mask, doc.width, doc.height))
        });
        match built {
            Some(plane) => {
                self.mask_pyramid = planes_of(&plane)
                    .map(ViewPyramid::rebuild)
                    .unwrap_or_default();
                self.mask_level0 = Some(plane);
            }
            None => {
                // The solo target is gone (mask or layer removed, or no
                // document): end the mode, not just the source.
                self.mask_solo = None;
                self.drop_mask_solo_source();
            }
        }
    }

    /// Drop the solo mask plane and pyramid, leaving the composite untouched.
    pub(crate) fn drop_mask_solo_source(&mut self) {
        self.mask_level0 = None;
        self.mask_pyramid = ViewPyramid::default();
    }

    /// Repair the solo mask plane and pyramid over `clipped` after a region
    /// commit patched the composite, so painting the soloed mask shows live
    /// coverage. A no-op when not soloed.
    pub(crate) fn sync_mask_solo_region(&mut self, clipped: PsdRect) {
        if self.mask_solo.is_none() {
            return;
        }
        if self.mask_level0.is_none() {
            self.rebuild_mask_solo_source();
            return;
        }
        let region = self.session.doc.as_ref().and_then(|doc| {
            let path = self.mask_solo.as_ref()?;
            let mask = pictura_render::resolve_path(doc, path)?.mask.as_ref()?;
            Some(mask_region_plane(mask, clipped))
        });
        let Some(region) = region else {
            self.drop_mask_solo_source();
            return;
        };
        if let Some(dst) = self.mask_level0.as_mut() {
            patch_buffer_region(dst, &region, clipped.left, clipped.top);
        }
        if let Some(planes) = self.mask_level0.as_ref().and_then(planes_of) {
            self.mask_pyramid.update(planes, clipped);
        }
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
    let mw = mask.rect.width();
    let mh = mask.rect.height();
    if mw <= 0 || mh <= 0 {
        return mask.default_color;
    }
    let mx = x - mask.rect.left;
    let my = y - mask.rect.top;
    if mx < 0 || my < 0 || mx >= mw || my >= mh {
        return mask.default_color;
    }
    data.sample(my as usize * mw as usize + mx as usize)
        .unwrap_or(mask.default_color)
}

/// The document-sized grayscale mask plane: each sample replicated across RGB,
/// opaque. The grayscale value is the stored channel, not density/feather
/// (composite-time parameters).
fn mask_display_plane(mask: &LayerMask, width: u32, height: u32) -> PixelBuffer {
    let mut buffer = PixelBuffer::new(width, height, 4);
    let plane = buffer.pixel_count();
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let v = mask_sample(mask, x, y);
            let i = y as usize * width as usize + x as usize;
            buffer.data[i] = v;
            buffer.data[plane + i] = v;
            buffer.data[2 * plane + i] = v;
            buffer.data[3 * plane + i] = 255;
        }
    }
    buffer
}

/// The same grayscale plane over `clipped` only, sized and originated at the
/// rectangle for [`patch_buffer_region`].
fn mask_region_plane(mask: &LayerMask, clipped: PsdRect) -> PixelBuffer {
    let rw = clipped.width().max(0) as u32;
    let rh = clipped.height().max(0) as u32;
    let mut buffer = PixelBuffer::new(rw, rh, 4);
    if rw == 0 || rh == 0 {
        return buffer;
    }
    let plane = buffer.pixel_count();
    for oy in 0..rh as i32 {
        let y = clipped.top + oy;
        for ox in 0..rw as i32 {
            let v = mask_sample(mask, clipped.left + ox, y);
            let i = oy as usize * rw as usize + ox as usize;
            buffer.data[i] = v;
            buffer.data[plane + i] = v;
            buffer.data[2 * plane + i] = v;
            buffer.data[3 * plane + i] = 255;
        }
    }
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
}
