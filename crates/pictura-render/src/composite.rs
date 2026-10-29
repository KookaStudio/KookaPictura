use pictura_adjust::{
    Adjustment, BlackWhiteParams, BrightnessContrastParams, ExposureParams, HueRange,
    HueSaturationParams, LevelsChannel, LevelsParams, PhotoFilterParams, ShadowsHighlightsParams,
    VibranceParams,
};
use pictura_codec::DescValue;
use pictura_core::{
    AdjustmentData, BlendIf, BlendMode, Document, Layer, LayerMask, PixelBuffer, PsdRect,
};

pub(crate) use crate::blend::blend;
pub(crate) use crate::composite_knockout::{composite_layer, composite_layers, has_knockout};
use crate::composite_native::composite_adjustment;
use crate::composite_route::ContentRoute;
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

pub use crate::composite_region::composite_rgba_region;

/// [`composite_rgba`] on one canvas, the path every stack can take; the
/// banded compositor is held to it.
pub(crate) fn composite_whole(doc: &Document) -> PixelBuffer {
    let mut canvas = Canvas::new(doc.width as usize, doc.height as usize);
    composite_layers(&mut canvas, doc);
    canvas.into_pixel_buffer()
}

/// Whether the layer (or a descendant) carries a layer-effects block. The PDF
/// and SVG exporters use this to decide that a layer's appearance is not just
/// its pixels.
pub(crate) fn has_effect_block(layer: &Layer) -> bool {
    layer.extra_block(b"lfx2").is_some()
        || layer.extra_block(b"lrFX").is_some()
        || layer.children.iter().any(has_effect_block)
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

pub(crate) fn composite_layer_inner(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    deep: Option<&Canvas>,
    shallow: Option<&Canvas>,
) {
    if !layer.visible {
        return;
    }
    let effects = crate::layer_source::decoded_effects(layer);
    let passes = (!canvas.skip_effects
        && !layer.is_group
        && !crate::layer_effects::is_destructive_adjustment(layer))
    .then(|| crate::layer_source::effect_passes(layer, doc, &effects, canvas.effect_region));
    // Below-content effects (drop shadow, outer glow) render behind the content.
    let effect_scale = canvas.effect_scale;
    canvas.effect_scale = effect_scale * layer.opacity as f32 / 255.0;
    crate::layer_effects::composite_layer_effects(canvas, passes.as_deref());
    canvas.effect_scale = effect_scale;
    // Each arm keeps the fall-through the old if/else chain had: a source
    // that cannot render (an empty region, an unreadable payload) leaves the
    // layer to the next path, ending at `composite_pixels`.
    match crate::composite_route::content_route(layer) {
        // True pass-through: recurse children onto the running canvas so their
        // blend modes see outside the group (exact only at opacity 255, no
        // mask; else isolated). `deep` threads through so a `Deep` knockout
        // child still reaches the document background; when the subtree knocks
        // out, the backdrop current at group entry is snapshotted as the
        // children's `Shallow` stopping point.
        ContentRoute::PassThroughGroup => {
            let snapshot = has_knockout(layer).then(|| canvas.clone());
            let child_shallow = snapshot.as_ref().or(shallow);
            crate::composite_clipping::composite_siblings(
                canvas,
                &layer.children,
                doc,
                |canvas, _, child| composite_layer(canvas, child, doc, deep, child_shallow),
            );
            return;
        }
        // An isolated group is a boundary: `deep` resets (a `Deep` child falls
        // back to the group's own backdrop) and that backdrop is also the
        // `Shallow` stopping point; the incoming bases are ignored.
        ContentRoute::IsolatedGroup => {
            let mut inner = canvas.derived(canvas.ox, canvas.oy, canvas.w, canvas.h);
            inner.skip_effects = canvas.skip_effects;
            let ko_base = has_knockout(layer)
                .then(|| canvas.derived(canvas.ox, canvas.oy, canvas.w, canvas.h));
            crate::composite_clipping::composite_siblings(
                &mut inner,
                &layer.children,
                doc,
                |canvas, _, child| composite_layer(canvas, child, doc, None, ko_base.as_ref()),
            );
            composite_canvas(canvas, layer, doc, &inner);
        }
        ContentRoute::Fill(adjustment) => composite_adjustment(canvas, layer, doc, &adjustment),
        // A layer with no decoded fill but an adjustment block is a no-op.
        ContentRoute::UndecodedAdjustment => {}
        ContentRoute::SmartFiltered => {
            if !crate::smart_filter::composite_smart_filtered_source(canvas, layer, doc)
                && !composite_smart_source(canvas, layer, doc)
                && !crate::text_render::composite_type_source(canvas, layer, doc)
            {
                composite_pixels(canvas, layer, doc);
            }
        }
        ContentRoute::SmartSource => {
            if !composite_smart_source(canvas, layer, doc)
                && !crate::text_render::composite_type_source(canvas, layer, doc)
            {
                composite_pixels(canvas, layer, doc);
            }
        }
        ContentRoute::TypeSource => {
            if !crate::text_render::composite_type_source(canvas, layer, doc) {
                composite_pixels(canvas, layer, doc);
            }
        }
        // A channel-less layer (for example a shape whose `vscg` did not
        // decode) reaches `composite_pixels`, which paints an opaque black rect
        // over the layer rect: the "vscg does not change the backdrop"
        // guarantee is relative to the bare channel-less layer. ponytail:
        // pre-existing channel-less behavior, left as is; gate it when a
        // fill-less shape layer needs to be truly transparent.
        ContentRoute::UndecodedVectorFill | ContentRoute::Pixels => {
            composite_pixels(canvas, layer, doc);
        }
    }
    // The above-content effect (inner shadow) renders over the layer's content.
    let effect_scale = canvas.effect_scale;
    canvas.effect_scale = effect_scale * layer.opacity as f32 / 255.0;
    crate::layer_effects::composite_layer_effects_above(canvas, passes.as_deref());
    canvas.effect_scale = effect_scale;
}

/// Decode a raw PSD adjustment block into a destructive [`Adjustment`] for the
/// encodings this crate supports. `None` means "not understood": the caller
/// leaves the backdrop unchanged (no-op), never errors.
///
/// Supported keys: `nvrt`/`invr` (Invert, no payload), `post` (Posterize),
/// `thrs` (Threshold), `brit` (Brightness/Contrast), `levl` (Levels, composite
/// record), `hue2`/`hue ` (Hue/Saturation), `expA` (Exposure), `vibA`
/// (Vibrance), `shdH` (Shadows/Highlights), `blwh` (Black & White), `phfl`
/// (Photo Filter, versions 2/3), `grdm` (Gradient Map, versions 1/3), `blnc`
/// (Color Balance), `mixr`
/// (Channel Mixer), `curv` (Curves), `selc` (Selective Color), `SoCo`
/// (solid-color fill content, either the 4-byte in-house RGBA tuple or the
/// standard PSD descriptor), `GdFl` (gradient fill content), `PtFl`
/// (pattern fill content), both fill descriptors, and `clrL` (Color Lookup,
/// whose embedded `.CUBE` is parsed when the kind is a 3-D LUT).
pub fn decode_adjustment(data: &AdjustmentData) -> Option<Adjustment> {
    match &data.key {
        b"nvrt" | b"invr" => Some(Adjustment::Invert),
        b"post" => be_u16(&data.data, 0)
            .filter(|v| (2..=255).contains(v))
            .map(|v| Adjustment::Posterize(v as u8)),
        b"thrs" => be_u16(&data.data, 0)
            .filter(|v| (1..=255).contains(v))
            .map(|v| Adjustment::Threshold(v as u8)),
        b"brit" => decode_brightness_contrast(&data.data),
        b"levl" => decode_levels(&data.data),
        b"hue2" | b"hue " => decode_hue_saturation(&data.data),
        b"expA" => decode_exposure(&data.data),
        b"phfl" => decode_photo_filter(&data.data),
        b"vibA" => decode_vibrance(&data.data),
        b"shdH" => decode_shadows_highlights(&data.data),
        b"blwh" => decode_black_white(&data.data),
        b"gdrm" | b"grdm" => crate::gradient_map::decode_gradient_map(&data.data),
        b"blnc" => crate::color_balance::decode_color_balance(&data.data),
        b"mixr" => crate::channel_mixer::decode_channel_mixer(&data.data),
        b"curv" => crate::curves::decode_curves(&data.data),
        b"selc" => crate::selective_color::decode_selective_color(&data.data),
        b"clrL" => crate::color_lookup::decode_color_lookup(&data.data),
        b"SoCo" => match data.data.as_slice() {
            [r, g, b, a] => Some(Adjustment::SolidFill([*r, *g, *b, *a])),
            _ => decode_solid_fill(&data.data),
        },
        b"GdFl" => crate::fill::decode_gradient_fill(&data.data),
        b"PtFl" => crate::fill::decode_pattern_fill(&data.data),
        _ => None,
    }
}

/// `SoCo`: the standard PSD solid-color fill descriptor. A version-16
/// `DescriptorBlock` whose `Clr ` object carries `Rd  `/`Grn `/`Bl  ` `doub`
/// values on the `0..=255` scale; each is rounded and clamped, and alpha is
/// forced to 255 (the descriptor has none). Missing key, wrong type,
/// non-finite value, or a parse error is `None`, never a panic.
pub(crate) fn decode_solid_fill(d: &[u8]) -> Option<Adjustment> {
    let obj = pictura_codec::read_descriptor(d).ok()?;
    let clr = desc_item(&obj, b"Clr ")?;
    if !matches!(clr, DescValue::Object { .. }) {
        return None;
    }
    let component = |key: &[u8]| -> Option<u8> {
        match desc_item(clr, key) {
            Some(DescValue::Double(c)) if c.is_finite() => Some(c.round().clamp(0.0, 255.0) as u8),
            _ => None,
        }
    };
    Some(Adjustment::SolidFill([
        component(b"Rd  ")?,
        component(b"Grn ")?,
        component(b"Bl  ")?,
        255,
    ]))
}

pub(crate) fn be_u16(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at + 2)?;
    Some(u16::from_be_bytes([s[0], s[1]]))
}

pub(crate) fn be_i16(d: &[u8], at: usize) -> Option<i16> {
    Some(be_u16(d, at)? as i16)
}

fn be_f32(d: &[u8], at: usize) -> Option<f32> {
    let s = d.get(at..at + 4)?;
    Some(f32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

pub(crate) fn be_u32(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at + 4)?;
    Some(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

/// `brit`: brightness (i16), contrast (i16), mean (i16), lab_only (u8), pad.
///
/// The reference's `brit` block carries no explicit "Use Legacy" flag; CS6 defaults
/// new adjustment layers to the modern curve, so decode with `use_legacy:false`.
/// ponytail: legacy-vs-modern detection is not encoded here; revisit if a CS6
/// baseline for legacy `brit` files appears.
fn decode_brightness_contrast(d: &[u8]) -> Option<Adjustment> {
    let brightness = be_i16(d, 0)?;
    let contrast = be_i16(d, 2)?;
    if !(-150..=150).contains(&brightness) || !(-50..=100).contains(&contrast) {
        return None;
    }
    Some(Adjustment::BrightnessContrast(BrightnessContrastParams {
        brightness,
        contrast,
        use_legacy: false,
    }))
}

/// `levl`: `u16 version` (2) then 29 five-`u16` records (input black, input
/// white, output black, output white, gamma × 100). Record 0 is the composite;
/// records 1–3 are red, green, and blue, kept when they differ from identity.
/// The rest (CMYK's fourth plate and the unused tail) are ignored.
fn decode_levels(d: &[u8]) -> Option<Adjustment> {
    if be_u16(d, 0)? != 2 {
        return None;
    }
    let composite = levels_record(d, 0)?;
    // A short block (some writers stop after the composite) or a zero-filled
    // record has no channel adjustment.
    let channel = |k: usize| {
        let record = d.get(2 + 10 * k..2 + 10 * (k + 1));
        if record.is_none_or(|bytes| bytes.iter().all(|&b| b == 0)) {
            return Some(None);
        }
        let record = levels_record(d, k)?;
        Some((record != LEVELS_IDENTITY).then_some(record))
    };
    Some(Adjustment::Levels(LevelsParams {
        input_black: composite.input_black,
        input_white: composite.input_white,
        gamma: composite.gamma,
        output_black: composite.output_black,
        output_white: composite.output_white,
        red: channel(1)?,
        green: channel(2)?,
        blue: channel(3)?,
    }))
}

const LEVELS_IDENTITY: LevelsChannel = LevelsChannel {
    input_black: 0,
    input_white: 255,
    gamma: 1.0,
    output_black: 0,
    output_white: 255,
};

/// `levl` record `k`, or `None` when it is out of range or invalid.
fn levels_record(d: &[u8], k: usize) -> Option<LevelsChannel> {
    let at = 2 + 10 * k;
    let input_black = be_u16(d, at)?;
    let input_white = be_u16(d, at + 2)?;
    let output_black = be_u16(d, at + 4)?;
    let output_white = be_u16(d, at + 6)?;
    let gamma = be_u16(d, at + 8)?;
    if input_black >= input_white || gamma == 0 {
        return None;
    }
    if [input_black, input_white, output_black, output_white]
        .into_iter()
        .any(|v| v > 255)
    {
        return None;
    }
    Some(LevelsChannel {
        input_black: input_black as u8,
        input_white: input_white as u8,
        gamma: gamma as f64 / 100.0,
        output_black: output_black as u8,
        output_white: output_white as u8,
    })
}

/// `hue2` (and legacy `hue `): version `u16`, enable `u8`, pad, colorization
/// (3×i16), the master Hue/Saturation/Lightness triplet (3×i16), then six
/// 14-byte colour-range records (Reds … Magentas): the band's four hue points
/// (4×i16) and its Hue/Saturation/Lightness (3×i16). Only ranges carrying
/// an edit are kept; a band stored as all zeros reads as CS6's default band.
fn decode_hue_saturation(d: &[u8]) -> Option<Adjustment> {
    if be_u16(d, 0)? != 2 {
        return None;
    }
    let hue = be_i16(d, 10)?;
    let saturation = be_i16(d, 12)?;
    let lightness = be_i16(d, 14)?;
    if !(-180..=180).contains(&hue)
        || !(-100..=100).contains(&saturation)
        || !(-100..=100).contains(&lightness)
    {
        return None;
    }
    let mut ranges = Vec::new();
    for (k, default) in HUE_RANGE_BANDS.iter().enumerate() {
        let at = 16 + 14 * k;
        // An older or truncated block stops after Master.
        let Some(record) = d.get(at..at + 14) else {
            break;
        };
        let v = |i: usize| i16::from_be_bytes([record[2 * i], record[2 * i + 1]]);
        let (hue, saturation, lightness) = (v(4), v(5), v(6));
        if (hue, saturation, lightness) == (0, 0, 0) {
            continue;
        }
        if !(-180..=180).contains(&hue)
            || !(-100..=100).contains(&saturation)
            || !(-100..=100).contains(&lightness)
        {
            return None;
        }
        let band = if record[..8].iter().all(|&b| b == 0) {
            *default
        } else {
            [v(0), v(1), v(2), v(3)]
        };
        ranges.push(HueRange {
            begin_ramp: band[0],
            begin_sustain: band[1],
            end_sustain: band[2],
            end_ramp: band[3],
            hue,
            saturation,
            lightness,
        });
    }
    Some(Adjustment::HueSaturation(HueSaturationParams {
        hue,
        saturation,
        lightness,
        ranges,
    }))
}

/// CS6's default colour-range bands, Reds through Magentas, as (begin ramp,
/// begin sustain, end sustain, end ramp) in degrees.
pub(crate) const HUE_RANGE_BANDS: [[i16; 4]; 6] = [
    [315, 345, 15, 45],
    [15, 45, 75, 105],
    [75, 105, 135, 165],
    [135, 165, 195, 225],
    [195, 225, 255, 285],
    [255, 285, 315, 345],
];

/// `expA`: `u16` version (= 1), then big-endian `f32` exposure, offset, and
/// gamma.
fn decode_exposure(d: &[u8]) -> Option<Adjustment> {
    if be_u16(d, 0)? != 1 {
        return None;
    }
    let exposure = be_f32(d, 2)? as f64;
    let offset = be_f32(d, 6)? as f64;
    let gamma = be_f32(d, 10)? as f64;
    if !exposure.is_finite() || !offset.is_finite() || !gamma.is_finite() || gamma <= 0.0 {
        return None;
    }
    Some(Adjustment::Exposure(ExposureParams {
        exposure,
        offset,
        gamma,
    }))
}

/// `phfl` version 2: `u16` version, `u16` colour space (ignored), four `u16`
/// colour components (first three = R, G, B; fourth ignored), `u32` density,
/// `u8` luminosity. A colour space other than RGB is decoded as RGB.
/// Version 3: three `be_u32` CIE XYZ values (offsets 2/6/10), `u32` density
/// (14), `u8` luminosity (18).
///
/// ponytail: the version-2 block length is flexible — only the 17 field bytes
/// are read, so an unpadded 17–19 byte payload still decodes while 20-byte
/// writers round-trip.
fn decode_photo_filter(d: &[u8]) -> Option<Adjustment> {
    let (color, density, preserve_luminosity) = match be_u16(d, 0)? {
        2 => {
            let component = |at: usize| -> Option<u8> {
                let v = be_u16(d, at)?;
                (v <= 255).then_some(v as u8)
            };
            (
                [component(4)?, component(6)?, component(8)?],
                be_u32(d, 12)?,
                *d.get(16)? != 0,
            )
        }
        3 => (
            xyz_d50_to_srgb_u8([be_u32(d, 2)?, be_u32(d, 6)?, be_u32(d, 10)?]),
            be_u32(d, 14)?,
            *d.get(18)? != 0,
        ),
        _ => return None,
    };
    if density > 100 {
        return None;
    }
    Some(Adjustment::PhotoFilter(PhotoFilterParams {
        color,
        density: density as f64,
        preserve_luminosity,
    }))
}

// ponytail: 16.16 scale and D50 white are unproven without a CS6 v3 fixture;
// layout grounded on psd-tools. If a real file disagrees, only the scale
// constant / white adaptation changes, not the field layout.
fn xyz_d50_to_srgb_u8(xyz: [u32; 3]) -> [u8; 3] {
    let scale = |v: u32| v as f64 / 65536.0;
    pictura_codec::xyz_d50_to_srgb_u8([scale(xyz[0]), scale(xyz[1]), scale(xyz[2])])
}

/// `vibA`: descriptor block with `vibrance` and the legacy `Strt` (saturation)
/// integer keys. Missing keys are 0; wrong types and out-of-range values are
/// rejected so a corrupt file cannot silently render a different adjustment.
fn decode_vibrance(d: &[u8]) -> Option<Adjustment> {
    let obj = pictura_codec::read_descriptor(d).ok()?;
    let vibrance = desc_long_or(&obj, b"vibrance", 0.0)?;
    let saturation = desc_long_or(&obj, b"Strt", 0.0)?;
    if !(-100.0..=100.0).contains(&vibrance) || !(-100.0..=100.0).contains(&saturation) {
        return None;
    }
    Some(Adjustment::Vibrance(VibranceParams {
        vibrance: vibrance as i16,
        saturation: saturation as i16,
    }))
}

/// `shdH`: two big-endian `u16` amounts, Shadows then Highlights, each
/// `0..=100`. An out-of-range amount is rejected so a corrupt file cannot render
/// a different adjustment.
fn decode_shadows_highlights(d: &[u8]) -> Option<Adjustment> {
    let shadows = be_u16(d, 0)?;
    let highlights = be_u16(d, 2)?;
    if shadows > 100 || highlights > 100 {
        return None;
    }
    Some(Adjustment::ShadowsHighlights(ShadowsHighlightsParams {
        shadows_amount: shadows as f64,
        highlights_amount: highlights as f64,
    }))
}

/// `blwh`: descriptor block with the six channel-percentage longs, a `useTint`
/// bool, and a nested `Clr ` `tintColor` object whose `Rd  `/`Grn `/`Bl  `
/// doubles are 0..1. Missing numeric keys default to the reference's Black & White
/// defaults; absent `tintColor` is black.
fn decode_black_white(d: &[u8]) -> Option<Adjustment> {
    let obj = pictura_codec::read_descriptor(d).ok()?;
    let red = desc_long_or(&obj, b"Rd  ", 40.0)?;
    let yellow = desc_long_or(&obj, b"Yllw", 60.0)?;
    let green = desc_long_or(&obj, b"Grn ", 40.0)?;
    let cyan = desc_long_or(&obj, b"Cyn ", 60.0)?;
    let blue = desc_long_or(&obj, b"Bl  ", 20.0)?;
    let magenta = desc_long_or(&obj, b"Mgnt", 80.0)?;
    if [red, yellow, green, cyan, blue, magenta]
        .into_iter()
        .any(|v| !(-200.0..=300.0).contains(&v))
    {
        return None;
    }
    let tint = match desc_item(&obj, b"useTint") {
        Some(DescValue::Bool(b)) => *b,
        Some(_) => return None,
        None => false,
    };
    let tint_color = match desc_item(&obj, b"tintColor") {
        Some(value) => decode_tint_color(value)?,
        None => [0, 0, 0],
    };
    Some(Adjustment::BlackWhite(BlackWhiteParams {
        red,
        yellow,
        green,
        cyan,
        blue,
        magenta,
        tint,
        tint_color,
    }))
}

fn decode_tint_color(value: &DescValue) -> Option<[u8; 3]> {
    if !matches!(value, DescValue::Object { .. }) {
        return None;
    }
    let component = |key: &[u8]| -> Option<u8> {
        match desc_item(value, key) {
            None => Some(0),
            Some(DescValue::Double(c)) => Some((c * 255.0).round().clamp(0.0, 255.0) as u8),
            Some(_) => None,
        }
    };
    Some([
        component(b"Rd  ")?,
        component(b"Grn ")?,
        component(b"Bl  ")?,
    ])
}

pub(crate) fn desc_item<'a>(obj: &'a DescValue, key: &[u8]) -> Option<&'a DescValue> {
    let DescValue::Object { items, .. } = obj else {
        return None;
    };
    items
        .iter()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, v)| v)
}

/// Read a `long` item as `f64`; `None` when the key is present but not a long.
fn desc_long_or(obj: &DescValue, key: &[u8], default: f64) -> Option<f64> {
    match desc_item(obj, key) {
        None => Some(default),
        Some(DescValue::Long(n)) => Some(*n as f64),
        Some(_) => None,
    }
}

// --- Encoders for the same subset ------------------------------------------
//
// These build the raw `AdjustmentData` the decoder above reads, so the app can
// create adjustment layers in memory. Byte layouts mirror psd-tools' adjustment
// structs (the independent oracle the codec fixtures come from); each is the
// minimal block for the key, not a full re-implementation of the reference's writer.

/// `nvrt`: Invert carries no payload.
pub fn encode_invert() -> AdjustmentData {
    AdjustmentData {
        key: *b"nvrt",
        data: Vec::new().into(),
    }
}

/// `post`: a `u16` levels value (2..=255) plus 2 pad bytes (psd-tools
/// `ShortIntegerElement`, `H2x`). Out-of-range input is clamped.
pub fn encode_posterize(levels: u8) -> AdjustmentData {
    encode_short(*b"post", levels.clamp(2, 255) as u16)
}

/// `thrs`: a `u16` level (1..=255) plus 2 pad bytes. Out-of-range input is clamped.
pub fn encode_threshold(level: u8) -> AdjustmentData {
    encode_short(*b"thrs", level.clamp(1, 255) as u16)
}

/// `brit`: brightness (i16), contrast (i16), mean (i16), lab_only (u8), pad
/// (psd-tools `3HBx`). Inputs are clamped to the decoder's accepted ranges
/// (brightness -150..=150, contrast -50..=100); `use_legacy` is not representable
/// and the decoder always uses the modern curve.
pub fn encode_brightness_contrast(brightness: i16, contrast: i16) -> AdjustmentData {
    let b = brightness.clamp(-150, 150);
    let c = contrast.clamp(-50, 100);
    let mut data = Vec::with_capacity(8);
    data.extend_from_slice(&b.to_be_bytes());
    data.extend_from_slice(&c.to_be_bytes());
    data.extend_from_slice(&0i16.to_be_bytes());
    data.push(0);
    data.push(0);
    AdjustmentData {
        key: *b"brit",
        data: data.into(),
    }
}

/// `hue2`: version (2), enable (1), pad, colorization (3×i16), then the master
/// Hue/Saturation/Lightness triplet (3×i16), followed by the six per-band range
/// records (6 × 7 i16) that the reference stores. The decoder only reads the version
/// and the master triplet; the trailing records are zeroed so the block matches
/// the real 100-byte layout.
pub fn encode_hue_saturation(hue: i16, saturation: i16, lightness: i16) -> AdjustmentData {
    let hue = hue.clamp(-180, 180);
    let saturation = saturation.clamp(-100, 100);
    let lightness = lightness.clamp(-100, 100);
    let mut data = Vec::with_capacity(100);
    data.extend_from_slice(&2u16.to_be_bytes());
    data.push(1);
    data.push(0);
    data.extend_from_slice(&[0u8; 6]);
    data.extend_from_slice(&hue.to_be_bytes());
    data.extend_from_slice(&saturation.to_be_bytes());
    data.extend_from_slice(&lightness.to_be_bytes());
    // Each colour range: its default band, no edit.
    for band in HUE_RANGE_BANDS {
        for v in band.into_iter().chain([0; 3]) {
            data.extend_from_slice(&v.to_be_bytes());
        }
    }
    AdjustmentData {
        key: *b"hue2",
        data: data.into(),
    }
}

/// `phfl`: the version-2 fixed struct the decoder reads. `density` clamps to
/// `0..=100`; the colour space and fourth component are zero, and three pad
/// bytes round the block to 20.
pub fn encode_photo_filter(
    color: [u8; 3],
    density: f64,
    preserve_luminosity: bool,
) -> AdjustmentData {
    let density = density.clamp(0.0, 100.0).round() as u32;
    let mut data = Vec::with_capacity(20);
    data.extend_from_slice(&2u16.to_be_bytes());
    data.extend_from_slice(&0u16.to_be_bytes());
    for c in color {
        data.extend_from_slice(&(c as u16).to_be_bytes());
    }
    data.extend_from_slice(&0u16.to_be_bytes());
    data.extend_from_slice(&density.to_be_bytes());
    data.push(u8::from(preserve_luminosity));
    data.extend_from_slice(&[0u8; 3]);
    AdjustmentData {
        key: *b"phfl",
        data: data.into(),
    }
}

/// `SoCo`: the standard PSD solid-color fill descriptor. `Clr ` is an
/// `RGBC` object carrying the three components as `doub` values on the
/// `0..=255` scale. The descriptor has no alpha, so authored fills are opaque.
pub fn encode_solid_color_fill(color: [u8; 3]) -> AdjustmentData {
    let clr = DescValue::Object {
        name: String::new(),
        class_id: b"RGBC".to_vec(),
        items: [b"Rd  ", b"Grn ", b"Bl  "]
            .into_iter()
            .zip(color)
            .map(|(key, c)| (key.to_vec(), DescValue::Double(c as f64)))
            .collect(),
    };
    let desc = DescValue::Object {
        name: String::new(),
        class_id: b"SoCo".to_vec(),
        items: vec![(b"Clr ".to_vec(), clr)],
    };
    AdjustmentData {
        key: *b"SoCo",
        data: pictura_codec::write_descriptor(&desc).into(),
    }
}

/// `H2x` payload: a big-endian `u16` value plus two zero pad bytes.
fn encode_short(key: [u8; 4], value: u16) -> AdjustmentData {
    let mut data = value.to_be_bytes().to_vec();
    data.extend_from_slice(&[0, 0]);
    AdjustmentData {
        key,
        data: data.into(),
    }
}

/// The Blend If weight for a source colour over a running backdrop: `1.0` when
/// the layer carries no non-default ranges, else the product of the active
/// composite and per-channel gates (each `0` or `1`).
///
/// ponytail: Rec.601 composite gray and a hard 0/1 gate; the reference's weighting
/// and split-slider feather are unpublished and the model stores no feather.
/// ponytail: channel groups are assumed R,G,B in order; the model does not label
/// them, and groups beyond RGB are ignored.
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

/// Composite one source sample over the backdrop with the layer's opacity, fill, mask, and gate.
pub(crate) fn blend_into(
    canvas: &mut Canvas,
    layer: &Layer,
    doc: &Document,
    x: usize,
    y: usize,
    cs: [f32; 3],
    src_a: f32,
) {
    let ox = canvas.ox;
    let w = canvas.w;
    let start = (y - canvas.oy as usize) * w;
    let clip = canvas.clip;
    let Canvas { px, cover, .. } = canvas;
    let row = &mut px[start..start + w];
    let cov = cover.as_deref_mut().map(|c| &mut c[start..start + w]);
    crate::composite_rows::blend_into_row(
        row, cov, ox, layer, doc, clip, x as i32, y as i32, cs, src_a,
    );
}

/// Composite an opacity-weighted source sample over the backdrop with a blend mode.
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
        .channels
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.data.as_slice())
}

pub(crate) fn sample(data: Option<&[u8]>, idx: usize) -> Option<u8> {
    data.and_then(|d| d.get(idx).copied())
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
    let idx = my as usize * mw as usize + mx as usize;
    data.get(idx).copied().unwrap_or(mask.default_color)
}
