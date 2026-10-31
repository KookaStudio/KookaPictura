//! Stamps: brush strokes whose paint comes from a source image instead of one
//! colour — the Clone Stamp, the Pattern Stamp, and the History Brush.
//!
//! The stroke itself is the ordinary Brush stroke (dabs, spacing, flow,
//! opacity, mode, the transparency lock, one history state); only where each
//! pixel's colour comes from differs. A [`StampSource`] is a document-space
//! image read at `destination + offset`:
//!
//! * **Clone Stamp** — a snapshot of the layer (or the composite, per the
//!   Sample menu) taken when the stroke begins, offset by the Alt-clicked
//!   source point. Reading live would feed each dab its own output and smear
//!   the stroke along itself when source and destination overlap.
//! * **Pattern Stamp** — a pattern tiled across the document from an origin;
//!   Aligned pins the origin to the document, unaligned to the stroke start.
//! * **History Brush** — the same layer in an earlier history state, at the
//!   same location (offset zero).
//!
//! Ported from photorust's `core/src/stamp.rs` and its clone / pattern stroke
//! handling (<https://github.com/perfecto25/photorust>); the History Brush has
//! no photorust source and reuses the same stroke.

use crate::healing::RgbaImage;
use crate::stroke::{layer_at, layer_planes, parse_layer_path};
use crate::Rgba;
use pictura_core::{Document, PixelBuffer, PsdRect};
use rayon::prelude::*;
use std::sync::Arc;

/// Where a stamp stroke reads its paint: a document-space image and the offset
/// added to a destination pixel to find its source, so `(-40, 0)` paints what
/// lies forty pixels to the left, or a pattern tile read per dab.
pub struct StampSource {
    kind: SourceKind,
}

enum SourceKind {
    Image {
        image: Arc<RgbaImage>,
        offset: (i32, i32),
        transform: Option<(SourceTransform, (f32, f32))>,
    },
    Pattern {
        tile: Arc<RgbaImage>,
        origin: (i32, i32),
        width: u32,
        height: u32,
    },
}

/// The Clone Source panel's transform of the source: W / H as scale factors
/// (negative for Flip Horizontal / Vertical) and a rotation in degrees,
/// counter-clockwise on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceTransform {
    pub scale_x: f32,
    pub scale_y: f32,
    pub angle_deg: f32,
}

impl Default for SourceTransform {
    fn default() -> Self {
        SourceTransform {
            scale_x: 1.0,
            scale_y: 1.0,
            angle_deg: 0.0,
        }
    }
}

impl SourceTransform {
    pub fn is_identity(&self) -> bool {
        *self == SourceTransform::default()
    }
}

impl StampSource {
    pub fn new(image: impl Into<Arc<RgbaImage>>, offset: (i32, i32)) -> StampSource {
        StampSource {
            kind: SourceKind::Image {
                image: image.into(),
                offset,
                transform: None,
            },
        }
    }

    /// A source painted through `transform` about the document point
    /// `anchor`, where the offset was measured: the anchor paints its source
    /// point, and the source around it is scaled, flipped, and rotated.
    pub fn transformed(
        image: impl Into<Arc<RgbaImage>>,
        offset: (i32, i32),
        anchor: (f32, f32),
        transform: SourceTransform,
    ) -> StampSource {
        let transform = (!transform.is_identity()).then_some((transform, anchor));
        StampSource {
            kind: SourceKind::Image {
                image: image.into(),
                offset,
                transform,
            },
        }
    }

    /// The pattern source for `tile` pinned to document point `origin` inside
    /// the `width × height` document; `None` for an empty tile.
    pub fn pattern(
        tile: impl Into<Arc<RgbaImage>>,
        origin: (i32, i32),
        width: u32,
        height: u32,
    ) -> Option<StampSource> {
        let tile = tile.into();
        (tile.width > 0 && tile.height > 0).then_some(StampSource {
            kind: SourceKind::Pattern {
                tile,
                origin,
                width,
                height,
            },
        })
    }

    /// The source for document pixel `(x, y)`, or `None` off the image: there
    /// is nothing to copy there, and painting transparency instead would punch
    /// holes in the layer.
    pub(crate) fn at(&self, x: i32, y: i32) -> Option<Rgba> {
        match &self.kind {
            SourceKind::Image {
                image,
                offset,
                transform,
            } => {
                let Some((t, anchor)) = transform else {
                    let (sx, sy) = (x + offset.0, y + offset.1);
                    if sx < 0 || sy < 0 || sx >= image.width || sy >= image.height {
                        return None;
                    }
                    let [r, g, b, a] = image.get(sx, sy);
                    return Some(Rgba { r, g, b, a });
                };
                // Invert the forward map (scale, then rotate) at the pixel centre.
                let (dx, dy) = (x as f32 + 0.5 - anchor.0, y as f32 + 0.5 - anchor.1);
                let (sin, cos) = t.angle_deg.to_radians().sin_cos();
                let (ux, uy) = (dx * cos - dy * sin, dx * sin + dy * cos);
                let sx = anchor.0 + offset.0 as f32 + ux / t.scale_x;
                let sy = anchor.1 + offset.1 as f32 + uy / t.scale_y;
                bilinear(image, sx - 0.5, sy - 0.5)
            }
            SourceKind::Pattern {
                tile,
                origin,
                width,
                height,
            } => {
                if x < 0 || y < 0 || x >= *width as i32 || y >= *height as i32 {
                    return None;
                }
                let tx = (x - origin.0).rem_euclid(tile.width);
                let ty = (y - origin.1).rem_euclid(tile.height);
                let [r, g, b, a] = tile.get(tx, ty);
                Some(Rgba { r, g, b, a })
            }
        }
    }

    #[cfg(test)]
    fn transform(&self) -> Option<&(SourceTransform, (f32, f32))> {
        match &self.kind {
            SourceKind::Image { transform, .. } => transform.as_ref(),
            SourceKind::Pattern { .. } => None,
        }
    }
}

/// Premultiplied bilinear sample at continuous pixel coordinates; `None` off
/// the image. Edge pixels clamp, so the image's border samples its own edge.
fn bilinear(img: &RgbaImage, x: f32, y: f32) -> Option<Rgba> {
    let (w, h) = (img.width as f32, img.height as f32);
    if x < -0.5 || y < -0.5 || x >= w - 0.5 || y >= h - 0.5 {
        return None;
    }
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let px = |xi: f32, yi: f32| {
        let xi = (xi as i32).clamp(0, img.width - 1);
        let yi = (yi as i32).clamp(0, img.height - 1);
        let [r, g, b, a] = img.get(xi, yi);
        let af = a as f32 / 255.0;
        [r as f32 * af, g as f32 * af, b as f32 * af, a as f32]
    };
    let mut acc = [0.0f32; 4];
    for (weight, p) in [
        ((1.0 - fx) * (1.0 - fy), px(x0, y0)),
        (fx * (1.0 - fy), px(x0 + 1.0, y0)),
        ((1.0 - fx) * fy, px(x0, y0 + 1.0)),
        (fx * fy, px(x0 + 1.0, y0 + 1.0)),
    ] {
        for c in 0..4 {
            acc[c] += weight * p[c];
        }
    }
    let a = acc[3];
    if a <= 0.0 {
        return Some(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        });
    }
    let un = |v: f32| (v * 255.0 / a).round().clamp(0.0, 255.0) as u8;
    Some(Rgba {
        r: un(acc[0]),
        g: un(acc[1]),
        b: un(acc[2]),
        a: a.round().clamp(0.0, 255.0) as u8,
    })
}

/// The Clone Stamp's Sample menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CloneSampling {
    /// The active layer alone (the spec's default).
    #[default]
    CurrentLayer,
    /// The active layer composited with the visible layers beneath it.
    CurrentAndBelow,
    /// The whole visible image.
    AllLayers,
}

impl CloneSampling {
    /// 0 Current Layer, 1 Current And Below, 2 All Layers; `None` otherwise.
    pub fn from_i32(v: i32) -> Option<CloneSampling> {
        match v {
            0 => Some(CloneSampling::CurrentLayer),
            1 => Some(CloneSampling::CurrentAndBelow),
            2 => Some(CloneSampling::AllLayers),
            _ => None,
        }
    }
}

/// The document whose composite a clone samples, or `None` for Current Layer
/// (read that with [`layer_surface`]). Current And Below keeps the layer at
/// `path` and everything beneath it — without the active layer itself, the
/// sample would ignore what the user is pointing at. `ignore_adjustments` is
/// CS6's toggle beside All Layers and applies only there.
pub fn sample_scope(
    doc: &Document,
    path: &str,
    sampling: CloneSampling,
    ignore_adjustments: bool,
) -> Option<Document> {
    let indices = parse_layer_path(path)?;
    layer_at(doc, &indices)?;
    let mut scope = doc.clone();
    match sampling {
        CloneSampling::CurrentLayer => return None,
        CloneSampling::CurrentAndBelow => truncate_above(&mut scope.layers, &indices),
        CloneSampling::AllLayers if ignore_adjustments => strip_adjustments(&mut scope.layers),
        CloneSampling::AllLayers => {}
    }
    Some(scope)
}

// Layers are stored bottom-first, so everything after an index is above it.
fn truncate_above(layers: &mut pictura_core::LayerStack, path: &[usize]) {
    fn walk(layers: &mut pictura_core::LayerStack, parent: &mut Vec<usize>, path: &[usize]) {
        let Some((&index, rest)) = path.split_first() else {
            return;
        };
        layers.truncate(parent, index + 1);
        if rest.is_empty() {
            return;
        }
        parent.push(index);
        walk(layers, parent, rest);
        parent.pop();
    }
    walk(layers, &mut Vec::new(), path);
}

fn strip_adjustments(layers: &mut pictura_core::LayerStack) {
    layers.retain_tree(&[], &mut |layer| layer.adjustment.is_none());
}

/// A planar RGBA composite (`pictura_render::composite_rgba`) as a surface.
pub fn surface_from_composite(buf: &PixelBuffer) -> RgbaImage {
    surface_from_composite_rect(
        buf,
        PsdRect {
            top: 0,
            left: 0,
            bottom: buf.height as i32,
            right: buf.width as i32,
        },
    )
}

/// The part of a planar composite inside `rect`, interleaved row by row.
pub fn surface_from_composite_rect(buf: &PixelBuffer, rect: PsdRect) -> RgbaImage {
    let (w, h) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    let mut data = vec![[0u8; 4]; w * h];
    if w == 0 || h == 0 {
        return RgbaImage {
            width: w as i32,
            height: h as i32,
            data,
        };
    }
    let (bw, bh) = (buf.width as usize, buf.height as usize);
    data.par_chunks_mut(w).enumerate().for_each(|(ly, row)| {
        let sy = rect.top + ly as i32;
        if sy < 0 || sy >= bh as i32 {
            return;
        }
        let sy = sy as usize;
        let r = buf.data.row(sy, bw).unwrap_or(&[]);
        let g = buf.data.row(bh + sy, bw).unwrap_or(&[]);
        let b = buf.data.row(2 * bh + sy, bw).unwrap_or(&[]);
        let a = buf.data.row(3 * bh + sy, bw).unwrap_or(&[]);
        for (lx, px) in row.iter_mut().enumerate() {
            let sx = rect.left + lx as i32;
            if sx < 0 || sx >= bw as i32 {
                continue;
            }
            let sx = sx as usize;
            *px = [
                r.get(sx).copied().unwrap_or(0),
                g.get(sx).copied().unwrap_or(0),
                b.get(sx).copied().unwrap_or(0),
                a.get(sx).copied().unwrap_or(0),
            ];
        }
    });
    RgbaImage {
        width: w as i32,
        height: h as i32,
        data,
    }
}

/// The pixel layer at `path` placed in document space (transparent outside
/// its rectangle), or `None` for a group, an adjustment, or a missing layer.
/// The History Brush reads a past state's layer through this, so a layer that
/// has since moved still paints back where its pixels were.
pub fn layer_surface(doc: &Document, path: &str) -> Option<RgbaImage> {
    let layer = layer_at(doc, &parse_layer_path(path)?)?;
    if layer.is_group || layer.adjustment.is_some() {
        return None;
    }
    let (w, h) = (doc.width as i32, doc.height as i32);
    let mut out = RgbaImage {
        width: w,
        height: h,
        data: vec![[0; 4]; (w * h) as usize],
    };
    let (uw, uh) = (w.max(0) as usize, h.max(0) as usize);
    if uw == 0 || uh == 0 {
        return Some(out);
    }
    let rect = layer.rect;
    let (lw, lh) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
    if lw == 0 || lh == 0 {
        return Some(out);
    }
    let planes = layer_planes(layer);
    out.data
        .par_chunks_mut(uw)
        .enumerate()
        .for_each(|(dy, row)| {
            let ly = dy as i32 - rect.top;
            if ly < 0 || ly >= lh as i32 {
                return;
            }
            let ly = ly as usize;
            let r = planes[0].and_then(|p| p.row(ly, lw)).unwrap_or(&[]);
            let g = planes[1].and_then(|p| p.row(ly, lw)).unwrap_or(&[]);
            let b = planes[2].and_then(|p| p.row(ly, lw)).unwrap_or(&[]);
            let a = planes[3].and_then(|p| p.row(ly, lw)).unwrap_or(&[]);
            for (dx, px) in row.iter_mut().enumerate() {
                let lx = dx as i32 - rect.left;
                if lx < 0 || lx >= lw as i32 {
                    continue;
                }
                let lx = lx as usize;
                *px = [
                    r.get(lx).copied().unwrap_or(0),
                    g.get(lx).copied().unwrap_or(0),
                    b.get(lx).copied().unwrap_or(0),
                    a.get(lx).copied().unwrap_or(255),
                ];
            }
        });
    Some(out)
}

/// `tile` repeated over a `width × height` surface with its top-left corner on
/// document point `origin`; `None` for an empty tile. Test-only oracle for the
/// lazy Pattern source.
#[cfg(test)]
pub fn tiled(tile: &RgbaImage, width: u32, height: u32, origin: (i32, i32)) -> Option<RgbaImage> {
    if tile.width <= 0 || tile.height <= 0 {
        return None;
    }
    let (w, h) = (width as i32, height as i32);
    let mut out = RgbaImage {
        width: w,
        height: h,
        data: vec![[0; 4]; (w * h) as usize],
    };
    for y in 0..h {
        let ty = (y - origin.1).rem_euclid(tile.height);
        for x in 0..w {
            out.set(x, y, tile.get((x - origin.0).rem_euclid(tile.width), ty));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stroke::{layer_at, layer_rgba, layer_rgba_serial, parse_layer_path};
    use crate::{pattern, Stroke, StrokeConfig, StrokeSample};
    use pictura_core::{AdjustmentData, BitDepth, Channel, ColorMode, Layer, PsdRect};

    const RED: [u8; 4] = [220, 20, 20, 255];
    const BLUE: [u8; 4] = [20, 20, 220, 255];

    fn layer(w: i32, h: i32, px: impl Fn(i32, i32) -> [u8; 4]) -> Layer {
        let plane = |c: usize| (0..w * h).map(|i| px(i % w, i / w)[c]).collect::<Vec<u8>>();
        Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: h,
                right: w,
            },
            channels: [(0, 0), (1, 1), (2, 2), (-1, 3)]
                .map(|(id, c)| Channel {
                    id,
                    data: plane(c).into(),
                })
                .into(),
            ..Default::default()
        }
    }

    fn doc(layers: Vec<Layer>) -> Document {
        let mut doc = Document::new(32, 32, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(layers);
        doc
    }

    fn pixel(doc: &Document, x: i32, y: i32) -> [u8; 4] {
        layer_rgba(&doc.layers[0]).get(x, y)
    }

    fn stamp(doc: &Document, source: StampSource, xs: &[f32]) -> Document {
        let cfg = StrokeConfig {
            diameter: 8,
            ..StrokeConfig::default()
        };
        let mut stroke = Stroke::begin_source(doc, "0", cfg, source).expect("begin");
        let mut out = doc.clone();
        for &x in xs {
            stroke.sample(
                &mut out,
                StrokeSample {
                    x,
                    y: 16.0,
                    pressure: 1.0,
                },
            );
        }
        stroke.finish().expect("painted");
        out
    }

    #[test]
    fn a_clone_stroke_copies_the_snapshot_at_its_offset_and_skips_off_image_sources() {
        let half = doc(vec![layer(32, 32, |x, _| if x < 16 { RED } else { BLUE })]);
        let snapshot = layer_surface(&half, "0").expect("a pixel layer");
        let cloned = stamp(
            &half,
            StampSource::new(snapshot.clone(), (-16, 0)),
            &[20.0, 28.0],
        );
        assert_eq!(
            pixel(&cloned, 24, 16),
            RED,
            "the left half was not cloned right"
        );
        assert_eq!(pixel(&cloned, 24, 2), BLUE, "the clone escaped the brush");
        // Sources off the left edge leave the destination as it was and store
        // no dirty rectangle, so the stroke records no history.
        let mut off = half.clone();
        let cfg = StrokeConfig {
            diameter: 8,
            ..StrokeConfig::default()
        };
        let mut stroke = Stroke::begin_source(&off, "0", cfg, StampSource::new(snapshot, (-40, 0)))
            .expect("begin");
        for &x in &[20.0, 28.0] {
            stroke.sample(
                &mut off,
                StrokeSample {
                    x,
                    y: 16.0,
                    pressure: 1.0,
                },
            );
        }
        assert!(
            stroke.finish().is_none(),
            "an off-image source changes no pixel"
        );
        assert_eq!(pixel(&off, 24, 16), BLUE);
    }

    #[test]
    fn a_transformed_source_flips_scales_and_rotates_about_the_anchor() {
        // A 4 px red mark 6 px right of the source point (4, 16).
        let img = layer_surface(
            &doc(vec![layer(32, 32, |x, y| {
                if (9..=11).contains(&x) && (15..=17).contains(&y) {
                    RED
                } else {
                    BLUE
                }
            })]),
            "0",
        )
        .unwrap();
        let rgb = |c: Option<Rgba>| c.map(|c| [c.r, c.g, c.b, c.a]);
        let anchor = (20.5, 16.5); // destination pixel (20, 16) paints source (4, 16)
        let at = |t: SourceTransform, x, y| {
            rgb(StampSource::transformed(img.clone(), (-16, 0), anchor, t).at(x, y))
        };
        let identity = SourceTransform::default();
        assert_eq!(at(identity, 26, 16), Some(RED), "identity moved the mark");
        let flip = SourceTransform {
            scale_x: -1.0,
            ..identity
        };
        assert_eq!(
            at(flip, 14, 16),
            Some(RED),
            "Flip Horizontal did not mirror"
        );
        assert_eq!(at(flip, 22, 16), Some(BLUE));
        assert_eq!(
            at(flip, 26, 16),
            None,
            "a mirrored source past the edge painted"
        );
        let double = SourceTransform {
            scale_x: 2.0,
            scale_y: 2.0,
            ..identity
        };
        assert_eq!(
            at(double, 32 - 1, 16),
            Some(RED),
            "200 % did not double the distance"
        );
        // Counter-clockwise on screen: right of the source paints above the anchor.
        let quarter = SourceTransform {
            angle_deg: 90.0,
            ..identity
        };
        assert_eq!(
            at(quarter, 20, 10),
            Some(RED),
            "90° did not rotate the mark up"
        );
        assert!(StampSource::transformed(img, (-16, 0), anchor, identity)
            .transform()
            .is_none());
    }

    #[test]
    fn sampling_scopes_drop_layers_above_and_optionally_adjustments() {
        let adjustment = Layer {
            adjustment: Some(AdjustmentData {
                key: *b"levl",
                data: Vec::new().into(),
            }),
            ..Default::default()
        };
        let stack = doc(vec![
            layer(32, 32, |_, _| RED),
            layer(32, 32, |_, _| BLUE),
            adjustment,
        ]);
        let below = sample_scope(&stack, "0", CloneSampling::CurrentAndBelow, false).unwrap();
        assert_eq!(below.layers.len(), 1);
        let all = sample_scope(&stack, "0", CloneSampling::AllLayers, false).unwrap();
        assert_eq!(all.layers.len(), 3);
        let plain = sample_scope(&stack, "0", CloneSampling::AllLayers, true).unwrap();
        assert_eq!(plain.layers.len(), 2, "the adjustment layer was sampled");
        assert!(sample_scope(&stack, "0", CloneSampling::CurrentLayer, false).is_none());
        assert!(sample_scope(&stack, "7", CloneSampling::AllLayers, false).is_none());
        assert_eq!(
            CloneSampling::from_i32(1),
            Some(CloneSampling::CurrentAndBelow)
        );
        assert_eq!(CloneSampling::from_i32(3), None);
    }

    #[test]
    fn a_pattern_stroke_paints_the_tile_pinned_to_its_origin() {
        let tile = pattern::tile(0).expect("the checkerboard");
        let aligned = tiled(&tile, 32, 32, (0, 0)).unwrap();
        let shifted = tiled(&tile, 32, 32, (-3, -5)).unwrap();
        assert_eq!(
            shifted.get(0, 0),
            tile.get(3, 5),
            "a negative origin did not wrap"
        );
        let blank = doc(vec![layer(32, 32, |_, _| [0; 4])]);
        let painted = stamp(&blank, StampSource::new(aligned, (0, 0)), &[16.0]);
        assert_eq!(pixel(&painted, 16, 16), tile.get(16, 16));
        assert_eq!(
            pixel(&painted, 16, 2),
            [0; 4],
            "the pattern escaped the brush"
        );
    }

    #[test]
    fn a_history_stroke_paints_the_earlier_state_back_in_place() {
        let before = doc(vec![layer(32, 32, |_, _| RED)]);
        let mut after = before.clone();
        after.layers[0] = layer(32, 32, |_, _| BLUE);
        let past = layer_surface(&before, "0").expect("the past layer");
        let restored = stamp(&after, StampSource::new(past, (0, 0)), &[16.0]);
        assert_eq!(pixel(&restored, 16, 16), RED);
        assert_eq!(pixel(&restored, 2, 2), BLUE);
        let grouped = doc(vec![Layer {
            is_group: true,
            ..Default::default()
        }]);
        assert!(layer_surface(&grouped, "0").is_none());
    }

    fn composite(w: u32, h: u32, seed: u8) -> PixelBuffer {
        let mut buf = PixelBuffer::new(w, h, 4);
        let n = (w * h) as usize;
        buf.data = (0..n * 4)
            .map(|i| (i as u8).wrapping_add(seed))
            .collect::<Vec<u8>>()
            .into();
        buf
    }

    fn composite_surface_serial(buf: &PixelBuffer) -> RgbaImage {
        let n = buf.pixel_count();
        let at = |c: usize, i: usize| buf.data.get(c * n + i).copied().unwrap_or(0);
        RgbaImage {
            width: buf.width as i32,
            height: buf.height as i32,
            data: (0..n)
                .map(|i| [at(0, i), at(1, i), at(2, i), at(3, i)])
                .collect(),
        }
    }

    #[test]
    fn surface_from_composite_matches_serial_builder() {
        for (w, h) in [(8, 6), (1, 1), (0, 0)] {
            let mut buf = composite(w, h, 7);
            buf.data.fracture(w.max(1) as usize);
            assert_eq!(
                surface_from_composite(&buf),
                composite_surface_serial(&buf),
                "{w}x{h}"
            );
        }
        let mut three = PixelBuffer::new(8, 6, 3);
        let n = 8 * 6 * 3;
        three.data = (0..n)
            .map(|i| (i as u8).wrapping_add(9))
            .collect::<Vec<u8>>()
            .into();
        three.data.fracture(8);
        assert_eq!(
            surface_from_composite(&three),
            composite_surface_serial(&three),
            "three channels"
        );
    }

    fn offset_layer(w: i32, h: i32, rect: PsdRect, px: impl Fn(i32, i32) -> [u8; 4]) -> Layer {
        let plane = |c: usize| (0..w * h).map(|i| px(i % w, i / w)[c]).collect::<Vec<u8>>();
        let width = w.max(1) as usize;
        Layer {
            rect,
            channels: [(0, 0), (1, 1), (2, 2), (-1, 3)]
                .map(|(id, c)| {
                    let mut channel = Channel {
                        id,
                        data: plane(c).into(),
                    };
                    channel.data.fracture(width);
                    channel
                })
                .into(),
            ..Default::default()
        }
    }

    fn layer_surface_serial(doc: &Document, path: &str) -> Option<RgbaImage> {
        let layer = layer_at(doc, &parse_layer_path(path)?)?;
        if layer.is_group || layer.adjustment.is_some() {
            return None;
        }
        let (w, h) = (doc.width as i32, doc.height as i32);
        let mut out = RgbaImage {
            width: w,
            height: h,
            data: vec![[0; 4]; (w * h) as usize],
        };
        let pixels = layer_rgba_serial(layer);
        let rect = layer.rect;
        for y in rect.top.max(0)..rect.bottom.min(h) {
            for x in rect.left.max(0)..rect.right.min(w) {
                out.set(x, y, pixels.get(x - rect.left, y - rect.top));
            }
        }
        Some(out)
    }

    #[test]
    fn layer_surface_matches_serial_builder() {
        for rect in [
            PsdRect {
                top: 0,
                left: 0,
                bottom: 32,
                right: 32,
            },
            PsdRect {
                top: 2,
                left: 3,
                bottom: 7,
                right: 8,
            },
            PsdRect {
                top: -2,
                left: -3,
                bottom: 3,
                right: 4,
            },
        ] {
            let (w, h) = (rect.width().max(1), rect.height().max(1));
            let layer = offset_layer(w, h, rect, |x, y| {
                [(x * 3) as u8, (y * 5) as u8, (x + y) as u8, 200]
            });
            let doc = doc(vec![layer]);
            assert_eq!(
                layer_surface(&doc, "0"),
                layer_surface_serial(&doc, "0"),
                "{rect:?}"
            );
        }
        let zero = doc(vec![offset_layer(
            0,
            0,
            PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            |_, _| [0; 4],
        )]);
        assert_eq!(layer_surface(&zero, "0"), layer_surface_serial(&zero, "0"));

        let mut zero_doc = Document::new(0, 0, ColorMode::Rgb, BitDepth::Eight);
        zero_doc.layers = pictura_core::LayerStack::from_layers(vec![offset_layer(
            0,
            0,
            PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            |_, _| [0; 4],
        )]);
        assert_eq!(
            layer_surface(&zero_doc, "0"),
            layer_surface_serial(&zero_doc, "0")
        );

        let no_alpha = Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 4,
                right: 4,
            },
            channels: [(0, 10u8), (1, 20), (2, 30)]
                .map(|(id, value)| {
                    let mut channel = Channel {
                        id,
                        data: vec![value; 16].into(),
                    };
                    channel.data.fracture(4);
                    channel
                })
                .into(),
            ..Default::default()
        };
        let no_alpha_doc = doc(vec![no_alpha]);
        assert_eq!(
            layer_surface(&no_alpha_doc, "0"),
            layer_surface_serial(&no_alpha_doc, "0"),
            "a layer without alpha"
        );
    }

    #[test]
    fn pattern_source_matches_tiled_image() {
        let tile = pattern::tile(0).expect("the checkerboard");
        let blank = doc(vec![layer(32, 32, |_, _| [0; 4])]);
        for origin in [(0, 0), (-3, -5), (5, 7)] {
            let from_image = stamp(
                &blank,
                StampSource::new(tiled(&tile, 32, 32, origin).unwrap(), (0, 0)),
                &[16.0],
            );
            let from_tile = stamp(
                &blank,
                StampSource::pattern(tile.clone(), origin, 32, 32).unwrap(),
                &[16.0],
            );
            assert_eq!(from_image, from_tile, "origin {origin:?}");
        }
        // A layer that hangs off the document must not paint where the
        // document-sized pre-tiled image had no source.
        let off = doc(vec![offset_layer(
            40,
            40,
            PsdRect {
                top: -4,
                left: -4,
                bottom: 36,
                right: 36,
            },
            |_, _| [0; 4],
        )]);
        for origin in [(0, 0), (-3, -5), (5, 7)] {
            let from_image = stamp(
                &off,
                StampSource::new(tiled(&tile, 32, 32, origin).unwrap(), (0, 0)),
                &[0.0],
            );
            let from_tile = stamp(
                &off,
                StampSource::pattern(tile.clone(), origin, 32, 32).unwrap(),
                &[0.0],
            );
            assert_eq!(from_image, from_tile, "off-canvas origin {origin:?}");
        }
        let empty = RgbaImage {
            width: 0,
            height: 0,
            data: Vec::new(),
        };
        assert!(StampSource::pattern(empty, (0, 0), 32, 32).is_none());
    }

    #[test]
    fn zero_size_sources_are_empty() {
        let empty = Layer {
            rect: PsdRect {
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
            },
            ..Default::default()
        };
        assert!(layer_rgba(&empty).data.is_empty());
        let buf = PixelBuffer::new(0, 0, 4);
        assert!(surface_from_composite(&buf).data.is_empty());
        let img = RgbaImage {
            width: 0,
            height: 0,
            data: Vec::new(),
        };
        let empty_rect = PsdRect {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        };
        assert!(crate::art_history::layer_local(&img, empty_rect)
            .data
            .is_empty());
        let mut zero_doc = Document::new(0, 0, ColorMode::Rgb, BitDepth::Eight);
        zero_doc.layers = pictura_core::LayerStack::from_layers(vec![Layer {
            rect: empty_rect,
            ..Default::default()
        }]);
        assert!(layer_surface(&zero_doc, "0").unwrap().data.is_empty());
    }
}
