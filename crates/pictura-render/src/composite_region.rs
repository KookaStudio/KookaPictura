//! Region and banded compositing. Split from `composite.rs` for the file-size
//! cap; the per-layer dispatch and canvas model stay there.

use pictura_core::{Document, Layer, PixelBuffer, PsdRect};

use crate::composite::Canvas;
use crate::composite_knockout::{composite_layers, composite_layers_range, has_knockout};
use crate::composite_plan::{Body, CompositePlan, PlanNode};

/// Rows per band of the banded compositor: a band's `f32` canvas stays a few
/// tens of megabytes on a 16k-wide document.
pub(crate) const BAND: u32 = 64;

/// Composite only the document-space region `[x0, x0+rw) × [y0, y0+rh)`.
///
/// Byte-identical to the same slice of [`crate::composite_rgba`] for every
/// stack: per-layer sources are sampled, and effect passes are replayed into
/// the region canvas.
pub(crate) fn composite_rgba_region(
    doc: &Document,
    x0: u32,
    y0: u32,
    rw: u32,
    rh: u32,
) -> PixelBuffer {
    if rw == 0 || rh == 0 {
        return PixelBuffer::new(0, 0, 4);
    }
    if banded(doc, rh) {
        return composite_banded(doc, x0, y0, rw, rh);
    }
    let mut canvas = Canvas::new_region(x0 as i32, y0 as i32, rw as usize, rh as usize);
    canvas.clip = PsdRect {
        top: y0 as i32,
        left: x0 as i32,
        bottom: (y0 + rh) as i32,
        right: (x0 + rw) as i32,
    };
    if x0 != 0 || y0 != 0 || rw != doc.width || rh != doc.height {
        canvas.effect_region = Some(canvas.clip);
    }
    let plan = CompositePlan::build(doc, canvas.clip, canvas.effect_region);
    composite_layers(&mut canvas, doc, &plan);
    canvas.into_pixel_buffer()
}

/// Whether `rows` rows of `doc` composite in bands. Cached sources make every
/// stack bandable except one whose effect pass build reads the document pattern
/// library: those passes are intentionally not cached, so a band would rebuild
/// them. Such a stack uses the whole-canvas path instead.
pub(crate) fn banded(doc: &Document, rows: u32) -> bool {
    rows > 2 * BAND && !doc.layers.iter().any(pattern_driven)
}

fn pattern_driven(layer: &pictura_core::Layer) -> bool {
    if !layer.visible {
        return false;
    }
    if !layer.is_group {
        let effects = crate::layer_source::decoded_effects(layer);
        if crate::layer_effects::reads_pattern(layer, &effects) {
            return true;
        }
    }
    layer.children.iter().any(pattern_driven)
}

/// Composite `[x0, x0+rw) × [y0, y0+rh)` a band of rows at a time, the bands in
/// parallel, each written straight into the output planes. Exact because a
/// region of any stack composites exactly (see [`composite_rgba_region`]); it
/// never holds the region's whole `f32` canvas, which on a 267-megapixel
/// document is 4 GB.
pub(crate) fn composite_banded(doc: &Document, x0: u32, y0: u32, rw: u32, rh: u32) -> PixelBuffer {
    let clip = PsdRect {
        top: y0 as i32,
        left: x0 as i32,
        bottom: (y0 + rh) as i32,
        right: (x0 + rw) as i32,
    };
    let plan = CompositePlan::build_banded(doc, clip);
    composite_banded_with_plan(doc, x0, y0, rw, rh, &plan)
}

#[cfg(test)]
pub(crate) fn composite_banded_for_test(
    doc: &Document,
    x0: u32,
    y0: u32,
    rw: u32,
    rh: u32,
    parallel: bool,
) -> PixelBuffer {
    let clip = PsdRect {
        top: y0 as i32,
        left: x0 as i32,
        bottom: (y0 + rh) as i32,
        right: (x0 + rw) as i32,
    };
    let plan = CompositePlan::build_banded_for_test(doc, clip, parallel);
    composite_banded_with_plan(doc, x0, y0, rw, rh, &plan)
}

fn composite_banded_with_plan(
    doc: &Document,
    x0: u32,
    y0: u32,
    rw: u32,
    rh: u32,
    plan: &CompositePlan<'_>,
) -> PixelBuffer {
    use rayon::prelude::*;
    let (w, plane) = (rw as usize, rw as usize * rh as usize);
    let run = BAND as usize * w;
    let clip = PsdRect {
        top: y0 as i32,
        left: x0 as i32,
        bottom: (y0 + rh) as i32,
        right: (x0 + rw) as i32,
    };
    let band_into = |band: usize, r: &mut [u8], g: &mut [u8], b: &mut [u8], a: &mut [u8]| {
        #[cfg(test)]
        crate::tests::effect_preparation::probe::observe(
            doc,
            None,
            crate::tests::effect_preparation::probe::Event::Band,
        );
        let top = y0 as i32 + (band * BAND as usize) as i32;
        let region = PsdRect {
            top,
            left: x0 as i32,
            bottom: top + (r.len() / w) as i32,
            right: (x0 + rw) as i32,
        };
        let (start, alone) = first_shown_root(plan, doc, region);
        if alone
            && copy_band(
                plan.roots[start].layer,
                doc,
                region,
                [&mut *r, &mut *g, &mut *b],
            )
        {
            a.fill(255);
            return;
        }
        let mut canvas = Canvas::new_region(x0 as i32, top, w, r.len() / w);
        canvas.clip = clip;
        canvas.effect_region = None;
        composite_layers_range(&mut canvas, doc, plan, start, plan.roots.len());
        for (i, p) in canvas.px.iter().enumerate() {
            r[i] = crate::composite::to_u8(p.r);
            g[i] = crate::composite::to_u8(p.g);
            b[i] = crate::composite::to_u8(p.b);
            a[i] = crate::composite::to_u8(p.a);
        }
    };
    let data = pictura_core::Plane::build(plane * 4, |out| {
        let (r, rest) = out.split_at_mut(plane);
        let (g, rest) = rest.split_at_mut(plane);
        let (b, a) = rest.split_at_mut(plane);
        // The completed plan retains passes and sources even if the shared
        // cache evicts them; replay never races to rebuild an artifact.
        let (r0, r) = r.split_at_mut(run.min(plane));
        let (g0, g) = g.split_at_mut(run.min(plane));
        let (b0, b) = b.split_at_mut(run.min(plane));
        let (a0, a) = a.split_at_mut(run.min(plane));
        band_into(0, r0, g0, b0, a0);
        r.par_chunks_mut(run)
            .zip(g.par_chunks_mut(run))
            .zip(b.par_chunks_mut(run))
            .zip(a.par_chunks_mut(run))
            .enumerate()
            .for_each(|(band, (((r, g), b), a))| band_into(band + 1, r, g, b, a));
    });
    PixelBuffer {
        width: rw,
        height: rh,
        channels: 4,
        data,
    }
}

/// The first root a band of `region` composites, and whether that root is
/// opaque over the band with nothing shown above it (#1149).
///
/// It is the topmost plain Normal layer that is opaque over all of `region`
/// when the stack below it is an opaque bottom layer under plain layers, else
/// the bottom. That stack leaves the backdrop alpha exactly 1: an opaque first
/// layer gives alpha 1, and each plain layer over it computes
/// `as + 1·(1 − as)`, which rounds to exactly 1 in `f32` for any `as` in
/// `[0, 1]`. A Normal source of alpha exactly 1 then yields `cs` and alpha 1
/// over that backdrop and over a transparent one alike, bit for bit, so the
/// layers under it can be skipped. Over the transparent canvas the bottom
/// yields `cs` in any mode.
fn first_shown_root(plan: &CompositePlan<'_>, doc: &Document, region: PsdRect) -> (usize, bool) {
    if doc.depth != pictura_core::BitDepth::Eight || doc.layers.iter().any(has_knockout) {
        return (0, false);
    }
    let shown = |node: &PlanNode<'_>| !matches!(node.body, Body::Hidden);
    let Some(bottom) = plan.roots.iter().position(shown) else {
        return (0, false);
    };
    if !opaque_over(&plan.roots[bottom], region) {
        return (0, false);
    }
    let mut start = bottom;
    for (i, node) in plan.roots.iter().enumerate().skip(bottom + 1) {
        if !shown(node) {
            continue;
        }
        if !plain(node) {
            break;
        }
        if node
            .blend
            .as_ref()
            .is_some_and(|ctx| ctx.mode() == pictura_core::BlendMode::Normal)
            && opaque_over(node, region)
        {
            start = i;
        }
    }
    let alone = !plan.roots[start + 1..].iter().any(shown);
    (start, alone)
}

/// Write `layer`'s colour over `region` straight into a band's output planes:
/// an opaque sample composited alone is `to_u8(k / 255) == k`. False, writing
/// nothing, for anything but an RGB layer whose three colour rows all cover
/// the region.
fn copy_band(layer: &Layer, doc: &Document, region: PsdRect, out: [&mut [u8]; 3]) -> bool {
    if doc.mode != pictura_core::ColorMode::Rgb {
        return false;
    }
    let rect = layer.rect;
    let lw = rect.width() as usize;
    let (from, to) = (
        (region.left - rect.left) as usize,
        (region.right - rect.left) as usize,
    );
    let w = to - from;
    let row = |id: i16, ly: usize| {
        crate::composite::channel_plane(layer, id)?
            .row(ly, lw)?
            .get(from..to)
    };
    let rows: Option<Vec<[&[u8]; 3]>> = (region.top..region.bottom)
        .map(|y| {
            let ly = (y - rect.top) as usize;
            Some([row(0, ly)?, row(1, ly)?, row(2, ly)?])
        })
        .collect();
    let Some(rows) = rows else {
        return false;
    };
    for (c, plane) in out.into_iter().enumerate() {
        for (dst, src) in plane.chunks_mut(w).zip(&rows) {
            dst.copy_from_slice(src[c]);
        }
    }
    true
}

/// A pixel layer drawn by the plain per-pixel blend alone: no effects, no
/// clipping, no group or other source.
fn plain(node: &PlanNode<'_>) -> bool {
    matches!(node.body, Body::Pixels)
        && node
            .passes
            .as_ref()
            .is_none_or(|passes| passes.below.is_empty() && passes.above.is_empty())
        && !node.layer.clipping
        && node.layer.smart_object.is_none()
}

/// Whether `node` is a plain layer whose every sample in `region` blends at
/// alpha exactly 1.
fn opaque_over(node: &PlanNode<'_>, region: PsdRect) -> bool {
    let layer = node.layer;
    let rect = layer.rect;
    if !plain(node)
        || !node.blend.as_ref().is_some_and(|ctx| ctx.passes_alpha())
        || node.sources.as_ref().is_none_or(|s| s.any_native())
        || rect.left > region.left
        || rect.top > region.top
        || rect.right < region.right
        || rect.bottom < region.bottom
    {
        return false;
    }
    // A layer with no alpha channel is opaque: a Background.
    let Some(alpha) = crate::composite::channel_plane(layer, -1) else {
        return true;
    };
    let lw = rect.width() as usize;
    let (from, to) = (
        (region.left - rect.left) as usize,
        (region.right - rect.left) as usize,
    );
    // A missing or short alpha row is not opaque. The compositor reads a
    // missing sample as 255 too, but being strict costs only the skip, never
    // a byte.
    (region.top..region.bottom).all(|y| {
        alpha
            .row((y - rect.top) as usize, lw)
            .and_then(|row| row.get(from..to))
            .is_some_and(|span| span.iter().all(|&v| v == 255))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BitDepth, BlendMode, Channel, ColorMode, Layer, LayerStack};

    const W: u32 = 50;
    const H: u32 = 320;

    /// A layer over `top..bottom` rows of the canvas, its samples from `at`
    /// (layer-local `x`, `y`, channel); no alpha channel when `alpha` is false.
    fn layer(
        (top, bottom): (i32, i32),
        blend: BlendMode,
        opacity: u8,
        alpha: bool,
        at: impl Fn(usize, usize, usize) -> u8,
    ) -> Layer {
        let rows = (bottom - top) as usize;
        let ids: &[i16] = if alpha { &[0, 1, 2, -1] } else { &[0, 1, 2] };
        Layer {
            name: "layer".into(),
            rect: PsdRect {
                top,
                left: 0,
                bottom,
                right: W as i32,
            },
            blend,
            opacity,
            fill: 255,
            visible: true,
            channels: ids
                .iter()
                .map(|&id| Channel {
                    id,
                    data: (0..rows * W as usize)
                        .map(|i| at(i % W as usize, i / W as usize, (id + 1) as usize))
                        .collect::<Vec<u8>>()
                        .into(),
                })
                .collect(),
            ..Default::default()
        }
    }

    fn noise(x: usize, y: usize, c: usize) -> u8 {
        let mut h = (x as u32).wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B);
        h ^= (c as u32).wrapping_mul(0xC2B2_AE35);
        h ^= h >> 15;
        h = h.wrapping_mul(0x2545_F491);
        (h >> 13) as u8
    }

    /// An opaque bottom, a translucent Multiply, an opaque Normal layer over
    /// bands 1–3 with one clear pixel in band 3, a translucent Screen and a
    /// layer clipped to it.
    fn stack(opaque_bottom: bool) -> Document {
        let mut doc = Document::new(W, H, ColorMode::Rgb, BitDepth::Eight);
        let bottom = layer(
            (0, H as i32),
            BlendMode::Normal,
            255,
            !opaque_bottom,
            |x, y, c| {
                if c == 0 {
                    (x as u8).wrapping_mul(5)
                } else {
                    noise(x, y, c)
                }
            },
        );
        let multiply = layer((0, H as i32), BlendMode::Multiply, 128, true, noise);
        let cover = layer((64, 256), BlendMode::Normal, 255, true, |x, y, c| match c {
            0 if (x, y) == (7, 150) => 0,
            0 => 255,
            _ => noise(x, y + 99, c),
        });
        let screen = layer((0, H as i32), BlendMode::Screen, 128, true, |x, y, c| {
            noise(x + 3, y, c)
        });
        let mut clipped = layer((0, H as i32), BlendMode::Normal, 200, true, |x, y, c| {
            noise(x, y + 7, c)
        });
        clipped.clipping = true;
        doc.layers = LayerStack::from_layers(vec![bottom, multiply, cover, screen, clipped]);
        doc
    }

    fn canvas() -> PsdRect {
        PsdRect {
            top: 0,
            left: 0,
            bottom: H as i32,
            right: W as i32,
        }
    }

    fn band_rect(band: i32) -> PsdRect {
        PsdRect {
            top: band * BAND as i32,
            left: 0,
            bottom: (band + 1) * BAND as i32,
            right: W as i32,
        }
    }

    #[test]
    fn a_band_starts_at_the_opaque_layer_over_it() {
        let doc = stack(true);
        let plan = CompositePlan::build(&doc, canvas(), None);
        let starts: Vec<(usize, bool)> = (0..5)
            .map(|band| first_shown_root(&plan, &doc, band_rect(band)))
            .collect();
        assert_eq!(
            starts.iter().map(|s| s.0).collect::<Vec<_>>(),
            [0, 2, 2, 0, 0]
        );
        assert!(starts.iter().all(|s| !s.1), "layers are shown above");
        let open = stack(false);
        let plan = CompositePlan::build(&open, canvas(), None);
        assert_eq!(first_shown_root(&plan, &open, band_rect(1)), (0, false));
    }

    /// The stack with the layers above the opaque one hidden, so its bands
    /// are copied.
    fn covered(opaque_bottom: bool) -> Document {
        let mut doc = stack(opaque_bottom);
        for layer in doc.layers.iter_mut().skip(3) {
            layer.visible = false;
        }
        doc
    }

    /// The opaque layer's alpha cut off after 100 of its 192 rows.
    fn truncated() -> Document {
        let mut doc = covered(true);
        let alpha = doc.layers[2]
            .channels
            .iter_mut()
            .find(|c| c.id == -1)
            .expect("an alpha channel");
        alpha.data.truncate(100 * W as usize);
        doc
    }

    #[test]
    fn a_short_alpha_plane_is_not_opaque() {
        let doc = truncated();
        let plan = CompositePlan::build(&doc, canvas(), None);
        assert_eq!(first_shown_root(&plan, &doc, band_rect(1)), (2, true));
        assert_eq!(first_shown_root(&plan, &doc, band_rect(2)), (0, false));
    }

    #[test]
    fn an_opaque_top_layer_alone_is_copied() {
        let doc = covered(true);
        let plan = CompositePlan::build(&doc, canvas(), None);
        assert_eq!(first_shown_root(&plan, &doc, band_rect(1)), (2, true));
        assert_eq!(first_shown_root(&plan, &doc, band_rect(3)), (0, false));
        for k in 0..=255u8 {
            assert_eq!(crate::composite::to_u8(k as f32 / 255.0), k);
        }
    }

    #[test]
    fn a_skipped_band_composites_as_the_whole_stack_does() {
        for doc in [
            stack(true),
            stack(false),
            covered(true),
            covered(false),
            truncated(),
        ] {
            let opaque_bottom = doc.layers[0].channels.len() == 3;
            assert!(banded(&doc, H));
            let whole = composite_rgba_region(&doc, 0, 0, W, H);
            let whole = whole.data.as_slice();
            let plane = (W * H) as usize;
            for band in 0..(H / BAND) {
                // A region this short composites every layer, unbanded.
                let part = composite_rgba_region(&doc, 0, band * BAND, W, BAND);
                let part = part.data.as_slice();
                let run = (W * BAND) as usize;
                let at = (band * W * BAND) as usize;
                for c in 0..4 {
                    assert_eq!(
                        &whole[c * plane + at..c * plane + at + run],
                        &part[c * run..(c + 1) * run],
                        "band {band}, channel {c}, opaque bottom {opaque_bottom}"
                    );
                }
            }
        }
    }
}
