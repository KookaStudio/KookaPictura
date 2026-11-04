//! The document's maintained composite cache: store, patch, refresh, and shape
//! validity.

use pictura_core::{ColorMode, Document, PixelBuffer, PsdRect};

use crate::gpu::Backend;

/// What a cache refresh covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositeScope {
    /// The whole document.
    Full,
    /// Document-space regions, applied in order.
    Regions(Vec<PsdRect>),
}

/// A rendered buffer and the document rectangle it covers.
#[derive(Clone, Debug)]
pub struct Refreshed {
    /// The raw straight-alpha composite; the caller converts to sRGB.
    pub buffer: PixelBuffer,
    /// The clamped document rectangle the buffer covers.
    pub rect: PsdRect,
    /// The backend that produced the buffer.
    pub backend: Backend,
}

/// A borrowing facade over the document's maintained composite.
///
/// The pixels stay in [`Document::composite`]; this type owns the cache
/// operations and the region-to-full equivalence.
pub struct DocumentComposite<'a> {
    doc: &'a mut Document,
}

impl<'a> DocumentComposite<'a> {
    pub fn new(doc: &'a mut Document) -> Self {
        Self { doc }
    }

    /// Whether the maintained composite has the shape a rendered picture needs:
    /// RGB, four channels, and document dimensions. Freshness is the writer's
    /// responsibility.
    pub fn is_shape_valid(doc: &Document) -> bool {
        doc.mode == ColorMode::Rgb
            && doc.composite.channels == 4
            && doc.composite.width == doc.width
            && doc.composite.height == doc.height
    }

    /// Store a rendered full-frame composite.
    ///
    /// An RGB document takes the rendered 4-plane RGBA frame. A non-RGB mode
    /// keeps its existing composite colour-plane count and copies
    /// `min(rendered, composite)` planes, so a 1-plane grayscale composite stays
    /// 1-plane. A composite whose dimensions no longer match is replaced.
    pub fn store(&mut self, rendered: &PixelBuffer) {
        if rendered.width != self.doc.width || rendered.height != self.doc.height {
            return;
        }
        let rgba = rgba_frame(rendered);
        if self.doc.mode == ColorMode::Rgb
            || self.doc.composite.width != self.doc.width
            || self.doc.composite.height != self.doc.height
        {
            self.doc.composite = rgba;
            return;
        }
        let target = self.doc.composite.channels as usize;
        let plane = (self.doc.width as usize) * (self.doc.height as usize);
        let mut data = vec![0u8; plane * target];
        for c in 0..target.min(rgba.channels as usize) {
            data[c * plane..(c + 1) * plane]
                .copy_from_slice(&rgba.data[c * plane..(c + 1) * plane]);
        }
        self.doc.composite = PixelBuffer {
            width: self.doc.width,
            height: self.doc.height,
            channels: target as u8,
            data: data.into(),
        };
    }

    /// Copy a region of a 4-plane rendered buffer into the maintained composite
    /// at `(x0, y0)`, writing the composite's own colour-plane count.
    pub fn patch(&mut self, region: &PixelBuffer, x0: i32, y0: i32) {
        if self.doc.composite.width != self.doc.width
            || self.doc.composite.height != self.doc.height
        {
            return;
        }
        patch_buffer_region(&mut self.doc.composite, region, x0, y0);
    }

    /// Composite `scope` and write the cache.
    pub fn refresh(&mut self, scope: CompositeScope, gpu: bool) -> Vec<Refreshed> {
        self.render_or_refresh(scope, gpu, true)
    }

    /// Composite `scope` without writing the cache.
    pub fn render(&mut self, scope: CompositeScope, gpu: bool) -> Vec<Refreshed> {
        self.render_or_refresh(scope, gpu, false)
    }

    fn render_or_refresh(
        &mut self,
        scope: CompositeScope,
        gpu: bool,
        write: bool,
    ) -> Vec<Refreshed> {
        match scope {
            CompositeScope::Full => {
                let full = full_rect(self.doc);
                if self.doc.layers.is_empty() {
                    let embedded = self.doc.composite.clone();
                    if write {
                        self.store(&embedded);
                        return vec![Refreshed {
                            buffer: self.doc.composite.clone(),
                            rect: full,
                            backend: Backend::Cpu,
                        }];
                    }
                    return vec![Refreshed {
                        buffer: embedded,
                        rect: full,
                        backend: Backend::Cpu,
                    }];
                }
                let (buffer, backend) =
                    crate::composite(crate::CompositeRequest::whole(self.doc, gpu));
                if write {
                    self.store(&buffer);
                }
                vec![Refreshed {
                    buffer,
                    rect: full,
                    backend,
                }]
            }
            CompositeScope::Regions(rects) => {
                if self.doc.layers.is_empty() {
                    return Vec::new();
                }
                let mut out = Vec::with_capacity(rects.len());
                for rect in rects {
                    let Some((x0, y0, w, h)) = clamp(rect, self.doc.width, self.doc.height) else {
                        continue;
                    };
                    let clipped = PsdRect {
                        top: y0,
                        left: x0,
                        bottom: y0 + h as i32,
                        right: x0 + w as i32,
                    };
                    let (buffer, backend) =
                        crate::composite(crate::CompositeRequest::region(self.doc, clipped, gpu));
                    if write {
                        let whole = (x0, y0, w, h) == (0, 0, self.doc.width, self.doc.height);
                        if whole {
                            self.store(&buffer);
                        } else {
                            self.patch(&buffer, x0, y0);
                        }
                    }
                    out.push(Refreshed {
                        buffer,
                        rect: clipped,
                        backend,
                    });
                }
                out
            }
        }
    }
}

fn full_rect(doc: &Document) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height as i32,
        right: doc.width as i32,
    }
}

fn clamp(rect: PsdRect, width: u32, height: u32) -> Option<(i32, i32, u32, u32)> {
    let x0 = rect.left.max(0);
    let y0 = rect.top.max(0);
    let x1 = rect.right.min(width as i32);
    let y1 = rect.bottom.min(height as i32);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
}

/// The rendered buffer as a 4-plane RGBA frame.
///
/// A 1/2/3-plane render (a layerless document's embedded composite) is expanded
/// with grey replication and opaque alpha.
pub fn rgba_frame(rendered: &PixelBuffer) -> PixelBuffer {
    if rendered.channels == 4 {
        return rendered.clone();
    }
    let plane = rendered.pixel_count();
    let channels = rendered.channels as usize;
    let mut data = vec![0u8; plane * 4];
    for i in 0..plane {
        let (r, g, b, a) = if channels <= 1 {
            let v = rendered.data[i];
            (v, v, v, 255)
        } else if channels == 2 {
            let v = rendered.data[i];
            (v, v, v, rendered.data[plane + i])
        } else {
            (
                rendered.data[i],
                rendered.data[plane + i],
                rendered.data[2 * plane + i],
                255,
            )
        };
        data[i] = r;
        data[plane + i] = g;
        data[2 * plane + i] = b;
        data[3 * plane + i] = a;
    }
    PixelBuffer {
        width: rendered.width,
        height: rendered.height,
        channels: 4,
        data: data.into(),
    }
}

/// Overwrite `dst` at `(x0, y0)` with a 4-channel planar region buffer, mapping
/// region plane `c` to `dst` plane `c` for `min(4, dst.channels)` planes.
pub fn patch_buffer_region(dst: &mut PixelBuffer, region: &PixelBuffer, x0: i32, y0: i32) {
    if region.channels != 4 {
        return;
    }
    if x0 < 0 || y0 < 0 {
        return;
    }
    let (x0, y0) = (x0 as usize, y0 as usize);
    let rw = region.width as usize;
    let rh = region.height as usize;
    let (fw, fh) = (dst.width as usize, dst.height as usize);
    if x0 + rw > fw || y0 + rh > fh {
        return;
    }
    let fplane = fw * fh;
    let rplane = rw * rh;
    let planes = (dst.channels as usize).min(4);
    if fplane == 0 || rw == 0 {
        return;
    }
    // Write row by row: a region patch must not flatten a banded composite
    // plane that history still shares.
    for c in 0..planes {
        for ry in 0..rh {
            let y = c * fh + y0 + ry;
            let Some(row) = dst.data.row_mut(y, fw) else {
                continue;
            };
            let src = c * rplane + ry * rw;
            if x0 + rw <= row.len() {
                row[x0..x0 + rw].copy_from_slice(&region.data[src..src + rw]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{doc, full, rect, solid};
    use pictura_core::{BitDepth, BlendMode, Channel, Layer, LayerStack};

    fn px(buf: &PixelBuffer, x: u32, y: u32) -> [u8; 4] {
        let plane = (buf.width * buf.height) as usize;
        let i = (y * buf.width + x) as usize;
        [
            buf.data[i],
            buf.data[plane + i],
            buf.data[2 * plane + i],
            buf.data[3 * plane + i],
        ]
    }

    fn gray_at(buf: &PixelBuffer, x: u32, y: u32) -> u8 {
        buf.data[(y * buf.width + x) as usize]
    }

    /// A cache seed that no composite of the fixtures produces, so a refresh
    /// that writes nothing or writes the wrong bytes cannot pass.
    fn marker(d: &Document, value: u8) -> PixelBuffer {
        PixelBuffer {
            width: d.width,
            height: d.height,
            channels: 4,
            data: vec![value; d.width as usize * d.height as usize * 4].into(),
        }
    }

    fn rgb_layers() -> Vec<Layer> {
        vec![
            solid(
                "base",
                full(8, 8),
                (30, 60, 90),
                255,
                BlendMode::Normal,
                255,
            ),
            solid(
                "top",
                rect(2, 2, 6, 6),
                (200, 100, 50),
                255,
                BlendMode::Normal,
                255,
            ),
        ]
    }

    fn gray_layer(w: u32, h: u32, value: u8) -> Layer {
        let n = (w * h) as usize;
        Layer {
            name: "gray".into(),
            rect: full(w, h),
            channels: vec![
                Channel {
                    id: 0,
                    data: vec![value; n].into(),
                },
                Channel {
                    id: -1,
                    data: vec![255; n].into(),
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_full_refresh_stores_the_composite() {
        let mut d = doc(8, 8, rgb_layers());
        let expected = crate::composite_rgba(&d);
        let out = DocumentComposite::new(&mut d).refresh(CompositeScope::Full, false);
        assert_eq!(out.len(), 1);
        assert_eq!(d.composite.width, 8);
        assert_eq!(d.composite.height, 8);
        assert_eq!(d.composite.data, expected.data);
    }

    #[test]
    fn a_region_refresh_patches_the_composite() {
        let mut d = doc(8, 8, rgb_layers());
        let full = crate::composite_rgba(&d);
        let seed_marker = marker(&d, 3);
        DocumentComposite::new(&mut d).store(&seed_marker);
        let before = d.composite.clone();
        let region = rect(2, 2, 6, 6);
        let out =
            DocumentComposite::new(&mut d).refresh(CompositeScope::Regions(vec![region]), false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rect, region);
        for y in 0..8 {
            for x in 0..8 {
                let inside = (2..6).contains(&x) && (2..6).contains(&y);
                if inside {
                    assert_eq!(px(&d.composite, x, y), px(&full, x, y));
                } else {
                    assert_eq!(px(&d.composite, x, y), px(&before, x, y));
                }
            }
        }
    }

    #[test]
    fn an_rgb_region_patch_matches_the_full_slice() {
        let mut d = doc(8, 8, rgb_layers());
        let seed_marker = marker(&d, 3);
        DocumentComposite::new(&mut d).store(&seed_marker);
        let seed = d.composite.clone();
        // The visibility change the deleted app oracle covered: the region
        // composite must follow the layer stack, not the seeded cache.
        assert_eq!(crate::set_visible_paths(&mut d, &["1"], false), 1);
        let full = crate::composite_rgba(&d);
        let region = rect(1, 1, 7, 7);
        DocumentComposite::new(&mut d).refresh(CompositeScope::Regions(vec![region]), false);
        for y in 0..8 {
            for x in 0..8 {
                let inside = (1..7).contains(&x) && (1..7).contains(&y);
                if inside {
                    assert_eq!(px(&d.composite, x, y), px(&full, x, y));
                } else {
                    assert_eq!(px(&d.composite, x, y), px(&seed, x, y));
                }
            }
        }
    }

    #[test]
    fn a_grayscale_patch_keeps_one_plane() {
        let mut d = Document::new(8, 8, ColorMode::Grayscale, BitDepth::Eight);
        d.layers = LayerStack::from_layers(vec![gray_layer(8, 8, 120)]);
        let full = crate::composite_rgba(&d);
        DocumentComposite::new(&mut d).refresh(CompositeScope::Full, false);
        assert_eq!(d.composite.channels, 1);
        for y in 0..8 {
            for x in 0..8 {
                assert_eq!(gray_at(&d.composite, x, y), gray_at(&full, x, y));
            }
        }

        d.layers[0].channels[0].data.fill(200);
        let mutated = crate::composite_rgba(&d);
        let before = d.composite.clone();
        let region = rect(2, 2, 6, 6);
        DocumentComposite::new(&mut d).refresh(CompositeScope::Regions(vec![region]), false);
        assert_eq!(d.composite.channels, 1);
        for y in 0..8 {
            for x in 0..8 {
                let inside = (2..6).contains(&x) && (2..6).contains(&y);
                if inside {
                    assert_eq!(gray_at(&d.composite, x, y), gray_at(&mutated, x, y));
                } else {
                    assert_eq!(gray_at(&d.composite, x, y), gray_at(&before, x, y));
                }
            }
        }
    }

    #[test]
    fn a_banded_composite_is_the_patch_reference() {
        let mut d = doc(
            32,
            160,
            vec![
                solid(
                    "base",
                    full(32, 160),
                    (30, 60, 90),
                    255,
                    BlendMode::Normal,
                    255,
                ),
                solid(
                    "top",
                    rect(0, 0, 80, 20),
                    (200, 100, 50),
                    255,
                    BlendMode::Normal,
                    255,
                ),
            ],
        );
        let banded_before = crate::composite_rgba(&d);
        let whole_before = crate::composite::composite_whole(&d);
        assert_eq!(
            banded_before.data, whole_before.data,
            "banded equals single canvas"
        );
        let seed_marker = marker(&d, 3);
        DocumentComposite::new(&mut d).store(&seed_marker);
        let seed = d.composite.clone();
        assert_eq!(crate::set_visible_paths(&mut d, &["1"], false), 1);
        let banded = crate::composite_rgba(&d);
        let whole = crate::composite::composite_whole(&d);
        assert_eq!(banded.data, whole.data, "banded equals single canvas");
        let region = rect(40, 0, 120, 32);
        DocumentComposite::new(&mut d).refresh(CompositeScope::Regions(vec![region]), false);
        for y in 0..160 {
            for x in 0..32 {
                let inside = (40..120).contains(&y);
                if inside {
                    assert_eq!(px(&d.composite, x, y), px(&banded, x, y));
                } else {
                    assert_eq!(px(&d.composite, x, y), px(&seed, x, y));
                }
            }
        }
    }

    #[test]
    fn a_whole_document_region_replaces_the_composite() {
        let mut d = doc(8, 8, rgb_layers());
        let seed_marker = marker(&d, 3);
        DocumentComposite::new(&mut d).store(&seed_marker);
        let seed = d.composite.clone();
        assert_eq!(crate::set_visible_paths(&mut d, &["1"], false), 1);
        let whole = crate::composite_rgba(&d);
        let out = DocumentComposite::new(&mut d)
            .refresh(CompositeScope::Regions(vec![full(8, 8)]), false);
        assert_eq!(out.len(), 1);
        assert_eq!(d.composite.data, whole.data);
        assert_eq!(
            seed.data, seed_marker.data,
            "the seeded bytes are not mutated"
        );
    }

    #[test]
    fn a_region_patch_preserves_a_shared_banded_plane() {
        let mut d = doc(
            32,
            160,
            vec![solid(
                "base",
                full(32, 160),
                (30, 60, 90),
                255,
                BlendMode::Normal,
                255,
            )],
        );
        let full_before = crate::composite_rgba(&d);
        DocumentComposite::new(&mut d).store(&full_before);
        let snapshot = d.composite.clone();
        assert!(snapshot.data.shares(&d.composite.data));
        d.layers[0].channels[0].data.fill(200);
        let full = crate::composite_rgba(&d);
        let region = rect(0, 0, 64, 32);
        DocumentComposite::new(&mut d).refresh(CompositeScope::Regions(vec![region]), false);
        assert_eq!(
            snapshot.data, full_before.data,
            "the snapshot bytes are unchanged"
        );
        for y in 0..160 {
            for x in 0..32 {
                let inside = (0..64).contains(&y);
                if inside {
                    assert_eq!(px(&d.composite, x, y), px(&full, x, y));
                } else {
                    assert_eq!(px(&d.composite, x, y), px(&snapshot, x, y));
                }
            }
        }
    }

    #[test]
    fn a_rendered_buffer_is_stored_without_recompositing() {
        let mut d = doc(8, 8, rgb_layers());
        let stored_marker = PixelBuffer {
            width: 8,
            height: 8,
            channels: 4,
            data: vec![7u8; 8 * 8 * 4].into(),
        };
        DocumentComposite::new(&mut d).store(&stored_marker);
        assert_eq!(d.composite.data, stored_marker.data);
        assert_ne!(
            d.composite.data,
            crate::composite_rgba(&d).data,
            "a recomposite would not match the marker"
        );
    }

    #[test]
    fn render_does_not_write_the_cache() {
        let mut d = doc(8, 8, rgb_layers());
        let seed_marker = marker(&d, 3);
        DocumentComposite::new(&mut d).store(&seed_marker);
        let before = d.composite.clone();
        let full = crate::composite_rgba(&d);
        let out = DocumentComposite::new(&mut d)
            .render(CompositeScope::Regions(vec![rect(2, 2, 6, 6)]), false);
        assert_eq!(out.len(), 1);
        assert_eq!(d.composite.data, before.data);
        assert_eq!(
            d.composite.data, seed_marker.data,
            "render must not write the cache"
        );
        for y in 2..6 {
            for x in 2..6 {
                assert_eq!(px(&out[0].buffer, x - 2, y - 2), px(&full, x, y));
            }
        }
    }

    #[test]
    fn a_size_mismatch_is_not_shape_valid() {
        let mut d = doc(8, 8, rgb_layers());
        d.composite = PixelBuffer::new(4, 4, 4);
        assert!(!DocumentComposite::is_shape_valid(&d));
    }

    #[test]
    fn a_non_rgb_or_non_four_channel_composite_is_not_shape_valid() {
        let mut gray = Document::new(8, 8, ColorMode::Grayscale, BitDepth::Eight);
        gray.composite = PixelBuffer::new(8, 8, 4);
        assert!(!DocumentComposite::is_shape_valid(&gray));

        let mut rgb = doc(8, 8, rgb_layers());
        rgb.composite = PixelBuffer::new(8, 8, 3);
        assert!(!DocumentComposite::is_shape_valid(&rgb));
    }

    #[test]
    fn a_layerless_full_refresh_stores_the_embedded_composite() {
        let mut d = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        d.composite.data.fill(9);
        let embedded = d.composite.clone();
        let out = DocumentComposite::new(&mut d).refresh(CompositeScope::Full, false);
        assert_eq!(out.len(), 1);
        assert_eq!(d.composite.channels, 4);
        let plane = 16;
        for c in 0..3 {
            assert_eq!(
                &d.composite.data[c * plane..(c + 1) * plane],
                &embedded.data[c * plane..(c + 1) * plane]
            );
        }
        assert!(d.composite.data[3 * plane..].iter().all(|&v| v == 255));
    }

    #[test]
    fn a_layerless_region_refresh_is_a_noop() {
        let mut d = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        d.composite.data.fill(9);
        let before = d.composite.clone();
        let out = DocumentComposite::new(&mut d)
            .refresh(CompositeScope::Regions(vec![rect(0, 0, 2, 2)]), false);
        assert!(out.is_empty());
        assert_eq!(d.composite.data, before.data);
    }

    #[test]
    fn store_expands_planes_like_rgba_frame() {
        let rendered = PixelBuffer {
            width: 2,
            height: 2,
            channels: 3,
            data: vec![1, 4, 7, 10, 2, 5, 8, 11, 3, 6, 9, 12].into(),
        };
        let frame = rgba_frame(&rendered);
        assert_eq!(frame.channels, 4);
        assert_eq!(&frame.data[0..4], &[1, 4, 7, 10]);
        assert_eq!(&frame.data[4..8], &[2, 5, 8, 11]);
        assert_eq!(&frame.data[8..12], &[3, 6, 9, 12]);
        assert_eq!(&frame.data[12..16], &[255; 4]);

        let mut rgb = Document::new(2, 2, ColorMode::Rgb, BitDepth::Eight);
        DocumentComposite::new(&mut rgb).store(&rendered);
        assert_eq!(rgb.composite.channels, 4);
        assert_eq!(rgb.composite.data, frame.data);

        let gray_rendered = PixelBuffer {
            width: 2,
            height: 2,
            channels: 1,
            data: vec![10, 20, 30, 40].into(),
        };
        let mut gray = Document::new(2, 2, ColorMode::Grayscale, BitDepth::Eight);
        DocumentComposite::new(&mut gray).store(&gray_rendered);
        assert_eq!(gray.composite.channels, 1);
        assert_eq!(gray.composite.data, gray_rendered.data);
    }

    #[test]
    fn a_grayscale_store_keeps_the_red_plane() {
        let mut d = Document::new(2, 2, ColorMode::Grayscale, BitDepth::Eight);
        let rendered = PixelBuffer {
            width: 2,
            height: 2,
            channels: 4,
            data: vec![
                10, 20, 30, 40, // R
                50, 60, 70, 80, // G
                90, 100, 110, 120, // B
                255, 255, 255, 255, // A
            ]
            .into(),
        };
        DocumentComposite::new(&mut d).store(&rendered);
        assert_eq!(d.composite.channels, 1);
        assert_eq!(d.composite.data.as_ref(), [10u8, 20, 30, 40].as_slice());
    }

    #[test]
    fn a_clipping_region_patch_matches_the_full_slice() {
        let mut top = solid(
            "top",
            rect(2, 2, 6, 6),
            (200, 100, 50),
            255,
            BlendMode::Normal,
            255,
        );
        top.clipping = true;
        let mut d = doc(
            8,
            8,
            vec![
                solid(
                    "base",
                    full(8, 8),
                    (30, 60, 90),
                    255,
                    BlendMode::Normal,
                    255,
                ),
                top,
            ],
        );
        let seed_marker = marker(&d, 3);
        DocumentComposite::new(&mut d).store(&seed_marker);
        let seed = d.composite.clone();
        d.layers[0].channels[0].data.fill(200);
        let full = crate::composite_rgba(&d);
        let region = rect(2, 2, 6, 6);
        DocumentComposite::new(&mut d).refresh(CompositeScope::Regions(vec![region]), false);
        for y in 0..8 {
            for x in 0..8 {
                let inside = (2..6).contains(&x) && (2..6).contains(&y);
                if inside {
                    assert_eq!(px(&d.composite, x, y), px(&full, x, y));
                } else {
                    assert_eq!(px(&d.composite, x, y), px(&seed, x, y));
                }
            }
        }
    }
}
