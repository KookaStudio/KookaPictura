//! The per-layer blend context and the CPU composite kernel.
//!
//! Per-layer state is resolved once for a layer pass; the kernel does only the
//! per-pixel arithmetic. See
//! `openspec/changes/composite-hoist-layer-context/design.md`.

use pictura_core::{BlendIf, BlendMode, Document, Layer, PsdRect, SampleRow, Samples};

use crate::composite::{blend_if_factor, Px};
use crate::composite_native::resolve_native_mask;
use crate::composite_rows::blend_parts_row;
use crate::mask_refine::{resolve_raster, resolve_vector, RasterGate, VectorGate};

#[cfg(test)]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// The layer state the blend kernel needs, resolved once for a layer pass.
pub(crate) struct BlendCtx<'a> {
    opacity: f32,
    fill: f32,
    curve: Option<&'static crate::text_gamma::TextCurve>,
    blend_if: Option<&'a BlendIf>,
    mode: BlendMode,
    mask: MaskGate<'a>,
}

impl<'a> BlendCtx<'a> {
    pub(crate) fn new(layer: &'a Layer, doc: &'a Document, clip: PsdRect) -> Self {
        let opacity = layer.opacity as f32 / 255.0;
        let fill = if layer.is_group {
            1.0
        } else {
            layer.fill as f32 / 255.0
        };
        let curve = layer
            .type_tool
            .as_ref()
            .and_then(|_| crate::text_gamma::current_curve());
        let blend_if = layer.blend_if.as_ref().filter(|view| !view.is_default());
        Self {
            opacity,
            fill,
            curve,
            blend_if,
            mode: layer.blend,
            mask: MaskGate::resolve(layer, doc, clip),
        }
    }
}

struct MaskGate<'a> {
    raster: RasterGate<'a>,
    vector: VectorGate<'a>,
}

impl<'a> MaskGate<'a> {
    fn resolve(layer: &'a Layer, doc: &'a Document, clip: PsdRect) -> Self {
        let raster = match resolve_native_mask(doc, layer) {
            Some(native) => RasterGate::Native {
                samples: native.samples,
                rect: native.rect,
                default_color: native.default_color,
            },
            None => resolve_raster(layer.mask.as_ref(), clip),
        };
        Self {
            raster,
            vector: resolve_vector(layer.vector_mask.as_ref(), clip),
        }
    }

    fn alpha(&self, x: i32, y: i32) -> f32 {
        #[cfg(test)]
        if self.counts() {
            MASK_SAMPLES.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(raster) = self.raster.native_at(x, y) {
            return raster * (self.vector.u8_at(x, y) as f32 / 255.0);
        }
        let raster = self.raster.u8_at(x, y);
        let vector = self.vector.u8_at(x, y);
        (((u16::from(raster) * u16::from(vector) + 127) / 255) as u8) as f32 / 255.0
    }

    /// Whether this layer has a mask to sample, so the test-only counter only
    /// counts a masked layer.
    #[cfg(test)]
    fn counts(&self) -> bool {
        !matches!(&self.raster, RasterGate::None) || !matches!(&self.vector, VectorGate::None)
    }
}

/// The layer's authoritative native planes, resolved once per layer pass.
pub(crate) struct SourcePlanes<'a> {
    native: [Option<&'a Samples>; 4],
}

impl<'a> SourcePlanes<'a> {
    pub(crate) fn resolve(layer: &'a Layer, doc: &'a Document) -> Self {
        Self {
            native: [0i16, 1, 2, -1].map(|id| doc.authoritative_plane(layer, id)),
        }
    }

    pub(crate) fn row(&self, y: usize, width: usize) -> [Option<SampleRow<'_>>; 4] {
        self.native
            .map(|samples| samples.and_then(|samples| samples.row(y, width)))
    }
}

/// Blend one source sample over `row` using the resolved layer state.
///
/// The `layer`/`doc`/`clip` arguments feed the test-only legacy branch; a
/// production build does not read them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn blend_into_row_ctx(
    row: &mut [Px],
    cover: Option<&mut [bool]>,
    ox: i32,
    ctx: &BlendCtx<'_>,
    _layer: &Layer,
    _doc: &Document,
    _clip: PsdRect,
    x: i32,
    y: i32,
    cs: [f32; 3],
    src_a: f32,
) {
    #[cfg(test)]
    if legacy_blend() {
        return blend_into_row_legacy(row, cover, ox, _layer, _doc, _clip, x, y, cs, src_a);
    }
    let mut content = (src_a * ctx.opacity) * ctx.fill;
    if content <= 0.0 {
        return;
    }
    if let Some(curve) = ctx.curve {
        content = curve.apply(content);
    }
    if content <= 0.0 {
        return;
    }
    let mask = ctx.mask.alpha(x, y);
    if content * mask <= 0.0 {
        return;
    }
    let gate = match ctx.blend_if {
        Some(view) => {
            let backdrop = row[(x - ox) as usize];
            blend_if_factor(Some(view), cs, [backdrop.r, backdrop.g, backdrop.b])
        }
        None => 1.0,
    };
    blend_parts_row(row, cover, ox, x, y, cs, content * mask * gate, ctx.mode);
}

/// The pre-change per-pixel body, kept as the bitwise oracle.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn blend_into_row_legacy(
    row: &mut [Px],
    cover: Option<&mut [bool]>,
    ox: i32,
    layer: &Layer,
    doc: &Document,
    clip: PsdRect,
    x: i32,
    y: i32,
    cs: [f32; 3],
    src_a: f32,
) {
    let opacity = layer.opacity as f32 / 255.0;
    let fill = if layer.is_group {
        1.0
    } else {
        layer.fill as f32 / 255.0
    };
    let mask = crate::composite_native::mask_alpha_unit(doc, layer, clip, x, y);
    let gate = match layer.blend_if.as_ref() {
        Some(view) if !view.is_default() => {
            let backdrop = row[(x - ox) as usize];
            blend_if_factor(Some(view), cs, [backdrop.r, backdrop.g, backdrop.b])
        }
        _ => 1.0,
    };
    let mut content = src_a * opacity * fill;
    if let Some(curve) = layer
        .type_tool
        .as_ref()
        .and_then(|_| crate::text_gamma::current_curve())
    {
        content = curve.apply(content);
    }
    blend_parts_row(row, cover, ox, x, y, cs, content * mask * gate, layer.blend);
}

/// The per-pixel entry point for tests.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn blend_into_row(
    row: &mut [Px],
    cover: Option<&mut [bool]>,
    ox: i32,
    layer: &Layer,
    doc: &Document,
    clip: PsdRect,
    x: i32,
    y: i32,
    cs: [f32; 3],
    src_a: f32,
) {
    let ctx = BlendCtx::new(layer, doc, clip);
    blend_into_row_ctx(row, cover, ox, &ctx, layer, doc, clip, x, y, cs, src_a);
}

#[cfg(test)]
static FORCE_LEGACY_BLEND: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static FORCE_LEGACY_SOURCE: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
pub(crate) static MASK_SAMPLES: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
pub(crate) fn legacy_blend() -> bool {
    FORCE_LEGACY_BLEND.load(Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn set_legacy_blend(on: bool) {
    FORCE_LEGACY_BLEND.store(on, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn legacy_source() -> bool {
    FORCE_LEGACY_SOURCE.load(Ordering::Relaxed)
}

#[cfg(test)]
pub(crate) fn set_legacy_source(on: bool) {
    FORCE_LEGACY_SOURCE.store(on, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn reset_mask_samples() {
    MASK_SAMPLES.store(0, Ordering::Relaxed);
}

#[cfg(test)]
pub(crate) fn mask_samples() -> usize {
    MASK_SAMPLES.load(Ordering::Relaxed)
}
