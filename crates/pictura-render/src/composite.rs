pub use pictura_codec::{
    decode_adjustment, encode_brightness_contrast, encode_hue_saturation, encode_invert,
    encode_photo_filter, encode_posterize, encode_solid_color_fill, encode_threshold,
};
use pictura_core::{BlendIf, BlendMode, Document, Layer, LayerMask, PixelBuffer, PsdRect};

pub(crate) use crate::blend::blend;
use crate::composite_blend::BlendOverride;
pub(crate) use crate::composite_knockout::{composite_layer, composite_layers, has_knockout};
use crate::composite_native::composite_adjustment;
use crate::composite_plan::{Body, CompositePlan, PlanNode};
use crate::composite_rows::{composite_canvas, composite_pixels};
use crate::smart_source::composite_smart_source;
pub(crate) use crate::smart_source::render_smart_source;

/// Composite the document's layer stack.
///
/// Returns a 4-channel (R,G,B,A) planar, straight-alpha, 8-bit buffer at
/// document resolution.
pub fn composite_rgba(doc: &Document) -> PixelBuffer {
    if crate::composite_region::banded(doc, doc.height) {
        return crate::composite_region::composite_banded(doc, 0, 0, doc.width, doc.height);
    }
    composite_whole(doc)
}

pub(crate) use crate::composite_region::composite_rgba_region;

/// [`composite_rgba`] on one canvas, the path every stack can take; the
/// banded compositor is held to it.
pub(crate) fn composite_whole(doc: &Document) -> PixelBuffer {
    let clip = PsdRect {
        top: 0,
        left: 0,
        bottom: doc.height.min(i32::MAX as u32) as i32,
        right: doc.width.min(i32::MAX as u32) as i32,
    };
    let plan = CompositePlan::build(doc, clip, None);
    let mut canvas = Canvas::new(doc.width as usize, doc.height as usize);
    let _span = crate::metrics::phase("render_execute", false);
    composite_layers(&mut canvas, doc, &plan);
    canvas.into_pixel_buffer()
}

/// Whether the layer (or a descendant) carries a layer-effects block. The PDF
/// and SVG exporters use this to decide that a layer's appearance is not just
/// its pixels.
pub(crate) fn has_effect_block(layer: &Layer) -> bool {
    layer.extra_block(b"lfx2").is_some()
        || layer.extra_block(b"lrFX").is_some()
        || layer.children().iter().any(has_effect_block)
}

/// Straight-alpha RGBA accumulator in normalized `f32`.
///
/// ponytail: one full-document f32 buffer; streaming/tiled compositing is the
/// ceiling to raise when PSB-size docs no longer fit in memory.
#[derive(Clone, Copy, Default)]
pub(crate) struct Px {
    pub(crate) r: f32,
    pub(crate) g: f32,
    pub(crate) b: f32,
    pub(crate) a: f32,
}

#[derive(Clone)]
pub(crate) struct Canvas {
    pub(crate) w: usize,
    pub(crate) h: usize,
    /// Document x of canvas column 0.
    pub(crate) ox: i32,
    /// Document y of canvas row 0.
    pub(crate) oy: i32,
    pub(crate) px: Vec<Px>,
    /// Per-pixel "the layer contributed here" flag, set only for the temporary
    /// canvas a knockout layer composites into. `None` on the output canvas so
    /// the normal path stays allocation-free.
    pub(crate) cover: Option<Vec<bool>>,
    /// Draw no layer effects: set by clipping/knockout coverage canvases.
    pub(crate) skip_effects: bool,
    /// Multiplier for effect alpha while a layer's effects are drawn. `1.0`
    /// outside an effect pass; the layer's opacity inside one.
    pub(crate) effect_scale: f32,
    /// The overall requested composite region, used as the mask-feather clip.
    /// A band inherits the whole requested region, not its own rows.
    pub(crate) clip: PsdRect,
    /// The transient effect-pass build region, or `None` for the cached
    /// whole-layer pass. A banded composite keeps `None` so one pass serves
    /// every band.
    pub(crate) effect_region: Option<PsdRect>,
}

impl Canvas {
    pub(crate) fn new(w: usize, h: usize) -> Self {
        Self::new_region(0, 0, w, h)
    }

    /// A canvas covering document-space `[ox, ox+w) × [oy, oy+h)`.
    pub(crate) fn new_region(ox: i32, oy: i32, w: usize, h: usize) -> Self {
        crate::metrics::note_canvas(w * h * std::mem::size_of::<Px>());
        Self {
            w,
            h,
            ox,
            oy,
            px: vec![Px::default(); w * h],
            cover: None,
            skip_effects: false,
            effect_scale: 1.0,
            clip: PsdRect {
                top: oy,
                left: ox,
                bottom: oy + h as i32,
                right: ox + w as i32,
            },
            effect_region: None,
        }
    }

    /// A same-region canvas that inherits the parent's clip and effect region.
    pub(crate) fn derived(&self, ox: i32, oy: i32, w: usize, h: usize) -> Canvas {
        let mut canvas = Canvas::new_region(ox, oy, w, h);
        canvas.clip = self.clip;
        canvas.effect_region = self.effect_region;
        canvas.skip_effects = self.skip_effects;
        canvas
    }

    /// The overall requested composite region.
    pub(crate) fn clip(&self) -> PsdRect {
        self.clip
    }

    /// A copy of `base`'s pixels with a fresh all-false coverage mask, so a
    /// knockout composite can record which pixels it actually covered.
    pub(crate) fn with_cover_from(base: &Canvas) -> Canvas {
        Canvas {
            w: base.w,
            h: base.h,
            ox: base.ox,
            oy: base.oy,
            px: base.px.clone(),
            cover: Some(vec![false; base.px.len()]),
            skip_effects: base.skip_effects,
            effect_scale: base.effect_scale,
            clip: base.clip,
            effect_region: base.effect_region,
        }
    }

    /// Document bounds of the canvas.
    pub(crate) fn x0(&self) -> i32 {
        self.ox
    }
    pub(crate) fn y0(&self) -> i32 {
        self.oy
    }
    pub(crate) fn x1(&self) -> i32 {
        self.ox + self.w as i32
    }
    pub(crate) fn y1(&self) -> i32 {
        self.oy + self.h as i32
    }

    /// The accumulator index of document pixel `(x, y)`.
    pub(crate) fn idx(&self, x: usize, y: usize) -> usize {
        debug_assert!((x as i32) >= self.ox && (y as i32) >= self.oy);
        (y as i32 - self.oy) as usize * self.w + (x as i32 - self.ox) as usize
    }

    /// The canvas as a planar RGBA8 buffer. A full document is hundreds of
    /// millions of pixels, so the planes are filled in parallel runs straight
    /// into one allocation (a write through the buffer's plane checks its
    /// refcount every time).
    pub(crate) fn into_pixel_buffer(self) -> PixelBuffer {
        use rayon::prelude::*;
        const RUN: usize = 1 << 16;
        let plane = self.w * self.h;
        let px = &self.px;
        let data = pictura_core::Plane::build(plane * 4, |out| {
            let (r, rest) = out.split_at_mut(plane);
            let (g, rest) = rest.split_at_mut(plane);
            let (b, a) = rest.split_at_mut(plane);
            r.par_chunks_mut(RUN)
                .zip(g.par_chunks_mut(RUN))
                .zip(b.par_chunks_mut(RUN))
                .zip(a.par_chunks_mut(RUN))
                .enumerate()
                .for_each(|(run, (((r, g), b), a))| {
                    let src = &px[run * RUN..run * RUN + r.len()];
                    for (i, p) in src.iter().enumerate() {
                        r[i] = to_u8(p.r);
                        g[i] = to_u8(p.g);
                        b[i] = to_u8(p.b);
                        a[i] = to_u8(p.a);
                    }
                });
        });
        PixelBuffer {
            width: self.w as u32,
            height: self.h as u32,
            channels: 4,
            data,
        }
    }
}

pub(crate) fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

const fn unit_table() -> [f32; 256] {
    let mut table = [0.0f32; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = i as f32 / 255.0;
        i += 1;
    }
    table
}

/// The unit value of each 8-bit sample, bit-identical to the division form.
static UNIT_U8: [f32; 256] = unit_table();

#[inline]
pub(crate) fn unit_u8(v: u8) -> f32 {
    UNIT_U8[v as usize]
}

/// Composite a layer, restoring any channel it excludes at the layer boundary.
///
/// The snapshot is taken before the below-content effects and restored after the
/// above-content effects, so content, effect passes, clipping and a
/// pass-through group's children all leave an excluded channel at the entry
/// backdrop's premultiplied value.
pub(crate) fn composite_layer_inner(
    canvas: &mut Canvas,
    node: &PlanNode<'_>,
    doc: &Document,
    deep: Option<&Canvas>,
    shallow: Option<&Canvas>,
    over: Option<&BlendOverride>,
) {
    if !node.layer.visible {
        return;
    }
    if over.is_some_and(|over| over.no_channel_restrictions) {
        composite_layer_dispatch(canvas, node, doc, deep, shallow, over);
        return;
    }
    let excluded = crate::composite_channels::excluded_planes(node.layer, doc);
    if crate::composite_channels::all_excluded(excluded) {
        return;
    }
    if !excluded.iter().any(|&value| value) {
        composite_layer_dispatch(canvas, node, doc, deep, shallow, over);
        return;
    }
    let Some(region) = crate::composite_channels::region(canvas, node) else {
        composite_layer_dispatch(canvas, node, doc, deep, shallow, over);
        return;
    };
    let snapshot = crate::composite_channels::snapshot(canvas, region);
    composite_layer_dispatch(canvas, node, doc, deep, shallow, over);
    crate::composite_channels::restore(canvas, region, excluded, &snapshot);
}

fn composite_layer_dispatch(
    canvas: &mut Canvas,
    node: &PlanNode<'_>,
    doc: &Document,
    deep: Option<&Canvas>,
    shallow: Option<&Canvas>,
    over: Option<&BlendOverride>,
) {
    let layer = node.layer;
    if !layer.visible {
        return;
    }
    let Some(plan_blend) = node.blend.as_ref() else {
        return;
    };
    let ctx = match over {
        Some(over) => plan_blend.with_override(over),
        None => plan_blend.clone(),
    };
    let passes = (!canvas.skip_effects)
        .then_some(node.passes.as_deref())
        .flatten();
    // Below-content effects (drop shadow, outer glow) render behind the content.
    let effect_scale = canvas.effect_scale;
    canvas.effect_scale = effect_scale * ctx.opacity();
    crate::layer_effects::composite_layer_effects(canvas, passes);
    canvas.effect_scale = effect_scale;
    let content_body = match &node.body {
        Body::Pixels
        | Body::SmartFiltered { .. }
        | Body::SmartSource { .. }
        | Body::TypeSource { .. } => true,
        Body::Fill(_) => true,
        _ => false,
    };
    let fold_group = layer.blend_interior
        && !layer.is_group()
        && !canvas.skip_effects
        && content_body
        && passes.is_some_and(|passes| !passes.fold.is_empty());
    if fold_group {
        let mut inner = canvas.derived(canvas.ox, canvas.oy, canvas.w, canvas.h);
        inner.skip_effects = true;
        inner.effect_scale = effect_scale;
        let inner_over = BlendOverride {
            mode: Some(BlendMode::Normal),
            opacity: Some(255),
            no_blend_if: true,
            no_channel_restrictions: true,
            ..Default::default()
        };
        composite_layer_inner(&mut inner, node, doc, deep, shallow, Some(&inner_over));
        crate::layer_effects::composite_layer_effects_fold(&mut inner, passes);
        let proxy_over = BlendOverride {
            fill: Some(255),
            no_mask: true,
            no_curve: true,
            ..Default::default()
        };
        let final_ctx = ctx.with_override(&proxy_over);
        composite_canvas(canvas, layer, doc, &inner, &final_ctx, Some(&proxy_over));
        let effect_scale = canvas.effect_scale;
        canvas.effect_scale = effect_scale * ctx.opacity();
        crate::layer_effects::composite_layer_effects_above(canvas, passes);
        canvas.effect_scale = effect_scale;
        return;
    }
    // Each arm keeps the fall-through the old if/else chain had: a source
    // that cannot render (an empty region, an unreadable payload) leaves the
    // layer to the next path, ending at `composite_pixels`.
    match &node.body {
        Body::Hidden => return,
        // True pass-through: recurse children onto the running canvas so their
        // blend modes see outside the group (exact only at opacity 255, no
        // mask; else isolated). `deep` threads through so a `Deep` knockout
        // child still reaches the document background; when the subtree knocks
        // out, the backdrop current at group entry is snapshotted as the
        // children's `Shallow` stopping point.
        Body::Group { pass_through } => {
            let pass_through = over
                .and_then(|over| over.isolated_group)
                .map(|isolated| !isolated)
                .unwrap_or(*pass_through);
            if pass_through {
                let snapshot = has_knockout(layer).then(|| canvas.clone());
                let child_shallow = snapshot.as_ref().or(shallow);
                crate::composite_clipping::composite_siblings(
                    canvas,
                    layer.children(),
                    doc,
                    node.children(),
                    |canvas, _, child, over| {
                        composite_layer(canvas, child, doc, deep, child_shallow, over)
                    },
                );
                return;
            }
            // An isolated group is a boundary: `deep` resets (a `Deep` child
            // falls back to the group's own backdrop) and that backdrop is also
            // the `Shallow` stopping point; the incoming bases are ignored.
            let mut inner = canvas.derived(canvas.ox, canvas.oy, canvas.w, canvas.h);
            inner.skip_effects = canvas.skip_effects;
            let ko_base = has_knockout(layer)
                .then(|| canvas.derived(canvas.ox, canvas.oy, canvas.w, canvas.h));
            crate::composite_clipping::composite_siblings(
                &mut inner,
                layer.children(),
                doc,
                node.children(),
                |canvas, _, child, over| {
                    composite_layer(canvas, child, doc, None, ko_base.as_ref(), over)
                },
            );
            composite_canvas(canvas, layer, doc, &inner, &ctx, over);
        }
        Body::Adjustment(adjustment) => {
            composite_adjustment(canvas, layer, doc, adjustment, &ctx, over)
        }
        Body::Fill(fill) => crate::fill::composite_fill(canvas, layer, doc, fill, &ctx, over),
        // A layer with no decoded fill but an adjustment block is a no-op.
        Body::UndecodedAdjustment => {}
        Body::SmartFiltered {
            filtered,
            filter_base,
            source,
            type_raster,
            has_proxy,
        } => {
            let mut drew = false;
            if let Some(filtered) = filtered {
                drew = crate::smart_filter::composite_smart_filtered_source(
                    canvas,
                    layer,
                    doc,
                    filtered,
                    filter_base.as_deref(),
                    &ctx,
                    over,
                );
            }
            if !drew && !*has_proxy {
                if let Some(source) = source {
                    drew = composite_smart_source(canvas, layer, doc, source, &ctx, over);
                }
                if !drew {
                    if let Some(raster) = type_raster {
                        drew = crate::text_render::composite_type_source(
                            canvas, layer, doc, raster, &ctx, over,
                        );
                    }
                }
            }
            if !drew && *has_proxy {
                composite_pixels(canvas, node, doc, &ctx, over);
            }
        }
        Body::SmartSource {
            source,
            type_raster,
            has_proxy,
        } => {
            let mut drew = false;
            if !has_proxy || crate::composite_route::stack_source_applies(layer) {
                if let Some(source) = source {
                    drew = composite_smart_source(canvas, layer, doc, source, &ctx, over);
                }
                if !drew {
                    if let Some(raster) = type_raster {
                        drew = crate::text_render::composite_type_source(
                            canvas, layer, doc, raster, &ctx, over,
                        );
                    }
                }
            }
            if !drew && *has_proxy {
                composite_pixels(canvas, node, doc, &ctx, over);
            }
        }
        Body::TypeSource { raster, has_proxy } => {
            // A proxy-less type layer draws its text. When the text paints
            // nothing, leave the backdrop unchanged: the channel-less fallback
            // would paint an opaque rect over the layer's box.
            if *has_proxy {
                composite_pixels(canvas, node, doc, &ctx, over);
            } else if let Some(raster) = raster {
                crate::text_render::composite_type_source(canvas, layer, doc, raster, &ctx, over);
            }
        }
        // A channel-less layer (for example a shape whose `vscg` did not
        // decode) reaches `composite_pixels`, which paints an opaque black rect
        // over the layer rect: the "vscg does not change the backdrop"
        // guarantee is relative to the bare channel-less layer. ponytail:
        // pre-existing channel-less behavior, left as is; gate it when a
        // fill-less shape layer needs to be truly transparent.
        Body::Pixels => composite_pixels(canvas, node, doc, &ctx, over),
    }
    // The above-content effects render over the layer's content.
    let effect_scale = canvas.effect_scale;
    canvas.effect_scale = effect_scale * ctx.opacity();
    crate::layer_effects::composite_layer_effects_fold(canvas, passes);
    crate::layer_effects::composite_layer_effects_above(canvas, passes);
    canvas.effect_scale = effect_scale;
}

pub(crate) use pictura_codec::layer_content::basic::{be_i16, be_u16, be_u32, desc_item};

pub(crate) fn blend_if_factor(view: Option<&BlendIf>, cs: [f32; 3], backdrop: [f32; 3]) -> f32 {
    let Some(view) = view else {
        return 1.0;
    };
    if view.is_default() {
        return 1.0;
    }
    let gate = |(black, white): (u16, u16), value: f32| {
        if (black, white) == (0, 65535) || black > white {
            return 1.0;
        }
        let lo = black as f32 / 65535.0;
        let hi = white as f32 / 65535.0;
        if value <= lo || value >= hi {
            0.0
        } else {
            1.0
        }
    };
    let gray = |c: [f32; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    let mut factor =
        gate(view.composite_source, gray(cs)) * gate(view.composite_dest, gray(backdrop));
    for (i, (source, dest)) in view.channel_ranges.iter().enumerate().take(3) {
        factor *= gate(*source, cs[i]) * gate(*dest, backdrop[i]);
    }
    factor
}

/// Composite one source sample over the backdrop with a resolved layer context.
#[allow(clippy::too_many_arguments)]
pub(crate) fn blend_into_ctx(
    canvas: &mut Canvas,
    ctx: &crate::composite_blend::BlendCtx<'_>,
    layer: &Layer,
    doc: &Document,
    clip: PsdRect,
    over: Option<&BlendOverride>,
    x: usize,
    y: usize,
    cs: [f32; 3],
    src_a: f32,
) {
    let ox = canvas.ox;
    let w = canvas.w;
    let start = (y - canvas.oy as usize) * w;
    let Canvas { px, cover, .. } = canvas;
    let row = &mut px[start..start + w];
    let cov = cover.as_deref_mut().map(|c| &mut c[start..start + w]);
    crate::composite_blend::blend_into_row_ctx(
        row, cov, ox, ctx, layer, doc, clip, over, x as i32, y as i32, cs, src_a,
    );
}

/// Composite an opacity-weighted source sample over the backdrop with a blend mode.
#[cfg(test)]
pub(crate) fn blend_parts(
    canvas: &mut Canvas,
    x: usize,
    y: usize,
    cs: [f32; 3],
    alpha: f32,
    mode: BlendMode,
) {
    let ox = canvas.ox;
    let w = canvas.w;
    let effect_scale = canvas.effect_scale;
    let start = (y - canvas.oy as usize) * w;
    let Canvas { px, cover, .. } = canvas;
    let row = &mut px[start..start + w];
    let cov = cover.as_deref_mut().map(|c| &mut c[start..start + w]);
    crate::composite_rows::blend_parts_row(
        row,
        cov,
        ox,
        x as i32,
        y as i32,
        cs,
        alpha * effect_scale,
        mode,
    );
}

pub(crate) fn channel(layer: &Layer, id: i16) -> Option<&[u8]> {
    layer
        .channels()
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.data.as_slice())
}

/// The plane of `layer`'s channel `id`, for readers that walk rows instead of
/// materializing a banded plane into one contiguous slice.
pub(crate) fn channel_plane(layer: &Layer, id: i16) -> Option<&pictura_core::Plane<u8>> {
    layer
        .channels()
        .iter()
        .find(|c| c.id == id)
        .map(|c| &c.data)
}

/// Mask value at a canvas pixel inside `clip` (see [`crate::mask_refine`]);
/// `255` when there is no usable mask.
///
/// The raster and vector coverages multiply, so either can suppress the layer;
/// an absent or disabled part contributes `255`.
pub(crate) fn mask_alpha(layer: &Layer, clip: PsdRect, x: i32, y: i32) -> u8 {
    let raster = raster_mask_alpha(layer, clip, x, y);
    let vector = crate::mask_refine::vector(layer.vector_mask.as_ref(), clip, x, y);
    ((raster as u16 * vector as u16 + 127) / 255) as u8
}

/// The raster layer-mask sample at a canvas pixel, with its density and
/// feather; `255` when absent/disabled.
pub(crate) fn raster_mask_alpha(layer: &Layer, clip: PsdRect, x: i32, y: i32) -> u8 {
    match layer.mask.as_ref() {
        Some(mask) => crate::mask_refine::raster(mask, clip, x, y),
        None => 255,
    }
}

/// A raster mask's sample at a document pixel; `255` when disabled or with no
/// decoded pixels, `default_color` outside its rect.
pub(crate) fn mask_value(mask: &LayerMask, x: i32, y: i32) -> u8 {
    if mask.disabled {
        return 255;
    }
    let Some(data) = &mask.data else {
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
    let idx = my as usize * mw as usize + mx as usize;
    data.sample(idx).unwrap_or(mask.default_color)
}
