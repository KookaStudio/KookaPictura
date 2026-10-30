//! Uploads of CPU-rasterized sources: cached type/smart/fill rasters.

use pictura_core::{Document, PixelBuffer, PsdRect};

use super::backend::{Gpu, SrcLayout};
use super::resident::UploadKey;
use crate::layer_source::CachedBuffer;

impl Gpu {
    /// Upload the cached raster `cached` and return the buffer and its
    /// document-space layout. The raster is clipped to the document before
    /// upload, so a layer with a negative origin (or one larger than the
    /// canvas) never shifts or paints off-canvas content. A whole-document
    /// upload is retained by content key; a region build is not, and a pattern
    /// fill is never retained.
    pub(super) fn build_cached_source(
        &self,
        cached: &CachedBuffer,
        doc: &Document,
    ) -> (wgpu::Buffer, SrcLayout) {
        let rect = clip_to_document(cached.rect, doc);
        let w = rect.width().max(0) as u32;
        let h = rect.height().max(0) as u32;
        let n = w as usize * h as usize;
        let layout = SrcLayout {
            x0: rect.left.max(0) as u32,
            y0: rect.top.max(0) as u32,
            w,
            h,
            gray: false,
            packed: false,
        };
        if n == 0 {
            return (
                self.upload_with("pictura-raster-empty", 4, |_, mut out| out.fill(0)),
                layout,
            );
        }
        let key = (cached.content != 0).then_some(UploadKey::Raster {
            content: cached.content,
            rect: (rect.left, rect.top, rect.right, rect.bottom),
            doc: (doc.width, doc.height),
            gray: false,
        });
        let keep = raster_keep(cached, self.is_document(doc));
        let (buffer, _) = self.kept_or_upload(key, keep, || {
            let data = if rect == cached.rect {
                rgba_planes(&cached.buffer)
            } else {
                clipped_planes(&cached.buffer, cached.rect, rect)
            };
            self.upload_with("pictura-raster", data.len(), move |start, mut out| {
                let end = start + out.len();
                if end <= data.len() {
                    out.copy_from_slice(&data[start..end]);
                } else {
                    out.fill(0);
                }
            })
        });
        (buffer, layout)
    }
}

fn clip_to_document(rect: PsdRect, doc: &Document) -> PsdRect {
    PsdRect {
        top: rect.top.max(0),
        left: rect.left.max(0),
        bottom: rect.bottom.min(doc.height.min(i32::MAX as u32) as i32),
        right: rect.right.min(doc.width.min(i32::MAX as u32) as i32),
    }
}

/// The four planar RGBA planes of a source, expanding a shorter buffer the way
/// the CPU does (absent colour aliases channel 0, absent alpha reads 255).
fn rgba_planes(src: &PixelBuffer) -> Vec<u8> {
    let n = src.pixel_count();
    let channels = src.channels as usize;
    if channels == 4 {
        return src.data.to_vec();
    }
    let mut out = vec![0u8; 4 * n];
    for i in 0..n {
        out[i] = src.data.get(i).copied().unwrap_or(0);
        out[n + i] = if channels >= 2 {
            src.data.get(n + i).copied().unwrap_or(0)
        } else {
            out[i]
        };
        out[2 * n + i] = if channels >= 3 {
            src.data.get(2 * n + i).copied().unwrap_or(0)
        } else {
            out[i]
        };
        out[3 * n + i] = if channels >= 4 {
            src.data.get(3 * n + i).copied().unwrap_or(0)
        } else {
            255
        };
    }
    out
}

/// The planar RGBA planes of `rect` clipped to `clip`, row-sliced from the
/// original raster. Both rectangles are in document coordinates and `clip` is
/// contained in `rect`.
fn clipped_planes(src: &PixelBuffer, rect: PsdRect, clip: PsdRect) -> Vec<u8> {
    let planes = rgba_planes(src);
    let sw = rect.width().max(0) as usize;
    let sh = rect.height().max(0) as usize;
    let cw = clip.width().max(0) as usize;
    let ch = clip.height().max(0) as usize;
    let plane = sw * sh;
    if plane == 0 || planes.len() < 4 * plane || cw == 0 || ch == 0 {
        return Vec::new();
    }
    let xoff = (clip.left - rect.left) as usize;
    let yoff = (clip.top - rect.top) as usize;
    let mut out = vec![0u8; 4 * cw * ch];
    for k in 0..4 {
        let from = k * plane;
        let to = k * cw * ch;
        for y in 0..ch {
            let start = from + (yoff + y) * sw + xoff;
            let dst = to + y * cw;
            out[dst..dst + cw].copy_from_slice(&planes[start..start + cw]);
        }
    }
    out
}

/// Whether a cached raster upload may be retained: a whole-document composite,
/// a retentive source, and a non-zero content key.
fn raster_keep(cached: &CachedBuffer, full: bool) -> bool {
    cached.retain && full && cached.content != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use pictura_core::{BitDepth, ColorMode, Document, PixelBuffer, PsdRect};

    fn cached(retain: bool) -> CachedBuffer {
        CachedBuffer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 2,
            },
            buffer: Arc::new(PixelBuffer::new(2, 2, 4)),
            content: 7,
            retain,
        }
    }

    #[test]
    fn raster_key_absent_for_region_build() {
        let cached = cached(true);
        assert!(!raster_keep(&cached, false), "a region build is not kept");
        assert!(raster_keep(&cached, true), "a whole-document build is kept");
        let _ = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
    }

    #[test]
    fn clipped_planes_slice_the_negative_origin() {
        let src = PixelBuffer {
            width: 3,
            height: 3,
            channels: 4,
            data: (0..36).map(|i| i as u8).collect::<Vec<u8>>().into(),
        };
        let rect = PsdRect {
            top: -1,
            left: -1,
            bottom: 2,
            right: 2,
        };
        let clip = PsdRect {
            top: 0,
            left: 0,
            bottom: 2,
            right: 2,
        };
        let out = clipped_planes(&src, rect, clip);
        let plane = 4;
        assert_eq!(out.len(), 4 * plane);
        // Original column/row 1 (document 0) starts each plane.
        assert_eq!(&out[0..2], &[4, 5]);
        assert_eq!(&out[plane..plane + 2], &[13, 14]);
    }
}
