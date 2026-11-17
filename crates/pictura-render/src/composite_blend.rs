//! The per-layer blend context and the CPU composite kernel.
//!
//! Per-layer state is resolved once for a layer pass; the kernel does only the
//! per-pixel arithmetic. See
//! `openspec/changes/composite-hoist-layer-context/design.md`.

use pictura_core::{BlendIf, BlendMode, Document, Layer, PsdRect, SampleRow, Samples};

use crate::composite::{blend_if_factor, unit_u8, Px};
use crate::composite_native::resolve_native_mask;
use crate::composite_rows::blend_parts_row;
use crate::mask_refine::{resolve_raster, resolve_vector, RasterGate, VectorGate};

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

/// The row-span kernel selected for a layer pass.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kernel {
    Masked,
    Alpha,
    Opaque,
}

/// The layer state the blend kernel needs, resolved once for a layer pass.
#[derive(Clone)]
pub(crate) struct BlendCtx<'a> {
    opacity: f32,
    fill: f32,
    curve: Option<&'static crate::text_gamma::TextCurve>,
    blend_if: Option<&'a BlendIf>,
    mode: BlendMode,
    mask: MaskGate<'a>,
}

/// Attributes a synthetic composite variant overrides without re-resolving the
/// layer's content. `isolated_group` re-decides the group arm: `Some(true)`
/// forces isolated, `Some(false)` forces pass-through.
#[derive(Clone, Copy, Default)]
pub(crate) struct BlendOverride {
    pub(crate) opacity: Option<u8>,
    pub(crate) fill: Option<u8>,
    pub(crate) mode: Option<BlendMode>,
    pub(crate) no_mask: bool,
    pub(crate) no_blend_if: bool,
    pub(crate) no_curve: bool,
    pub(crate) no_channel_restrictions: bool,
    pub(crate) isolated_group: Option<bool>,
}

impl<'a> BlendCtx<'a> {
    pub(crate) fn new(layer: &'a Layer, doc: &'a Document, clip: PsdRect) -> Self {
        #[cfg(test)]
        crate::composite_plan::counters::note_mask_gate(layer);
        let opacity = layer.opacity as f32 / 255.0;
        let fill = if layer.is_group() {
            1.0
        } else {
            layer.fill as f32 / 255.0
        };
        let curve = crate::text_gamma::applies_to(layer)
            .then(crate::text_gamma::current_curve).flatten();
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

    pub(crate) fn opacity(&self) -> f32 {
        self.opacity
    }

    pub(crate) fn fill(&self) -> f32 {
        self.fill
    }

    pub(crate) fn mode(&self) -> BlendMode {
        self.mode
    }

    /// Whether the content's own alpha reaches the blend unscaled: full
    /// opacity and fill, no type curve, no Blend If and no mask, so a sample of
    /// alpha 255 blends at exactly 1.
    pub(crate) fn passes_alpha(&self) -> bool {
        self.opacity == 1.0
            && self.fill == 1.0
            && self.curve.is_none()
            && self.blend_if.is_none()
            && self.mask.is_none()
    }

    /// The row-span kernel for the effective state, or `None` when the layer
    /// must use the general per-pixel kernel.
    pub(crate) fn kernel(&self, native: bool, alpha_plane: bool) -> Option<Kernel> {
        if self.mode != BlendMode::Normal
            || self.curve.is_some()
            || self.blend_if.is_some()
            || native
        {
            return None;
        }
        Some(if self.mask.is_none() {
            if self.passes_alpha() && !alpha_plane {
                Kernel::Opaque
            } else {
                Kernel::Alpha
            }
        } else {
            Kernel::Masked
        })
    }

    /// Fill `out` with the mask coverage for `[x0, x1)` on row `y`. Entries
    /// whose source alpha after opacity and fill is zero stay `0.0` and do not
    /// sample the mask, matching the general path's zero-alpha skip.
    pub(crate) fn mask_row(
        &self,
        x0: i32,
        x1: i32,
        y: i32,
        alpha: Option<&[u8]>,
        out: &mut Vec<f32>,
    ) {
        out.clear();
        out.resize((x1 - x0).max(0) as usize, 0.0);
        for (i, x) in (x0..x1).enumerate() {
            let a = alpha.map_or(1.0, |line| unit_u8(line[i]));
            let content = (a * self.opacity) * self.fill;
            if content > 0.0 {
                out[i] = self.mask.alpha(x, y);
            }
        }
    }

    pub(crate) fn with_override(&self, over: &BlendOverride) -> Self {
        let mut ctx = self.clone();
        if let Some(opacity) = over.opacity {
            ctx.opacity = opacity as f32 / 255.0;
        }
        if let Some(fill) = over.fill {
            ctx.fill = fill as f32 / 255.0;
        }
        if let Some(mode) = over.mode {
            ctx.mode = mode;
        }
        if over.no_mask {
            ctx.mask = MaskGate::none();
        }
        if over.no_blend_if {
            ctx.blend_if = None;
        }
        if over.no_curve {
            ctx.curve = None;
        }
        ctx
    }
}

#[derive(Clone)]
struct MaskGate<'a> {
    raster: RasterGate<'a>,
    vector: VectorGate<'a>,
}

impl<'a> MaskGate<'a> {
    fn none() -> Self {
        Self {
            raster: RasterGate::None,
            vector: VectorGate::None,
        }
    }

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

    fn is_none(&self) -> bool {
        matches!(self.raster, RasterGate::None) && matches!(self.vector, VectorGate::None)
    }

    fn alpha(&self, x: i32, y: i32) -> f32 {
        #[cfg(test)]
        if self.counts() {
            MASK_SAMPLES.with(|samples| samples.set(samples.get() + 1));
        }
        if let Some(raster) = self.raster.native_at(x, y) {
            return raster * unit_u8(self.vector.u8_at(x, y));
        }
        let raster = self.raster.u8_at(x, y);
        let vector = self.vector.u8_at(x, y);
        unit_u8(((u16::from(raster) * u16::from(vector) + 127) / 255) as u8)
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
        #[cfg(test)]
        crate::composite_plan::counters::note_native_plane(layer);
        Self {
            native: [0i16, 1, 2, -1].map(|id| doc.authoritative_plane(layer, id)),
        }
    }

    /// Whether any channel reads a native store instead of its 8-bit plane.
    pub(crate) fn any_native(&self) -> bool {
        self.native.iter().any(Option::is_some)
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
    _over: Option<&BlendOverride>,
    x: i32,
    y: i32,
    cs: [f32; 3],
    src_a: f32,
) {
    #[cfg(test)]
    if legacy_blend() {
        return blend_into_row_legacy(row, cover, ox, _layer, _doc, _clip, _over, x, y, cs, src_a);
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
    over: Option<&BlendOverride>,
    x: i32,
    y: i32,
    cs: [f32; 3],
    src_a: f32,
) {
    let opacity = over.and_then(|over| over.opacity).unwrap_or(layer.opacity) as f32 / 255.0;
    let fill = if layer.is_group() {
        1.0
    } else {
        over.and_then(|over| over.fill).unwrap_or(layer.fill) as f32 / 255.0
    };
    let mask = if over.is_some_and(|over| over.no_mask) {
        1.0
    } else {
        crate::composite_native::mask_alpha_unit(doc, layer, clip, x, y)
    };
    let blend_if = if over.is_some_and(|over| over.no_blend_if) {
        None
    } else {
        layer.blend_if.as_ref()
    };
    let gate = match blend_if {
        Some(view) if !view.is_default() => {
            let backdrop = row[(x - ox) as usize];
            blend_if_factor(Some(view), cs, [backdrop.r, backdrop.g, backdrop.b])
        }
        _ => 1.0,
    };
    let mut content = src_a * opacity * fill;
    if let Some(curve) = crate::text_gamma::applies_to(layer)
        .then(crate::text_gamma::current_curve).flatten()
    {
        content = curve.apply(content);
    }
    let mode = over.and_then(|over| over.mode).unwrap_or(layer.blend);
    blend_parts_row(row, cover, ox, x, y, cs, content * mask * gate, mode);
}

#[cfg(test)]
static FORCE_LEGACY_BLEND: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
static FORCE_LEGACY_SOURCE: AtomicBool = AtomicBool::new(false);
#[cfg(test)]
thread_local! {
    /// Mask samples on the current thread. Counting tests force the serial
    /// compositor branch and read this thread's counter, so a concurrent test
    /// on another thread cannot pollute the count.
    static MASK_SAMPLES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

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
    MASK_SAMPLES.with(|samples| samples.set(0));
}

#[cfg(test)]
pub(crate) fn mask_samples() -> usize {
    MASK_SAMPLES.with(std::cell::Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_alpha_uses_the_unit_table() {
        let clip = PsdRect {
            top: 0,
            left: 0,
            bottom: 1,
            right: 1,
        };
        let layer = pictura_codec::layer_from_parts(pictura_core::LayerParts {
            rect: clip,
            mask: Some(pictura_core::LayerMask {
                rect: clip,
                default_color: 255,
                data: Some(vec![128].into()),
                ..Default::default()
            }),
            ..Default::default()
        });
        let doc = Document::new(
            1,
            1,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        let gate = MaskGate::resolve(&layer, &doc, clip);
        assert_eq!(
            gate.alpha(0, 0).to_bits(),
            crate::composite::unit_u8(128).to_bits()
        );
    }
}
