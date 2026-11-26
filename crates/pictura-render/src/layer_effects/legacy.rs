//! The legacy `lrFX` (`EFFECTS_LAYER`) effects block: the fixed binary struct
//! The reference's 5.0–6.0 releases wrote and CS still writes for compatibility.
//!
//! The block is an `EffectsLayer`: a `u16` version and a `u16` count, then
//! `count` records each beginning `8BIM` + a 4-byte `ostype` + a `u32` body
//! length. The records this crate renders are `cmnS` (common state, whose
//! `visible` flag gates the whole set), `dsdw`/`isdw` shadows, `oglw`/`iglw`
//! glows, `bevl` bevel and `sofi` solid fill. Everything is big-endian and
//! unpadded.
//!
//! ponytail: the block carries no satin/stroke/gradient/pattern record, no
//! contour, noise or anti-alias, and no gradient inner glow; the bevel renders
//! only as `Inner` + `Smooth`; the document global light is resolved by
//! `light::effective`; and the
//! legacy `intensity` slot is read as libpsd's `spread`/`choke` (psd-tools and
//! libpsd disagree on the underlying blur split, but the body size is the
//! same). Structural faults return `None`, a malformed record is skipped, and
//! no read panics.

use pictura_core::{BlendMode, Layer};

use super::{
    BevelDirection, BevelEmboss, BevelHighlight, BevelShadow, BevelStyle, BevelTechnique,
    ColorOverlay, DropShadow, GlowSource, GlowTechnique, InnerGlow, InnerShadow, OuterGlow,
    MAX_ALTITUDE, MAX_CHOKE, MAX_DEPTH, MAX_DISTANCE, MAX_OPACITY, MAX_SIZE, MAX_SPREAD,
};

/// The typed effects mapped from a layer's legacy `lrFX` block; `None` fields
/// have no legacy record (satin, stroke and the overlays are always `None`).
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct LegacyEffects {
    pub(crate) drop_shadow: Option<DropShadow>,
    pub(crate) inner_shadow: Option<InnerShadow>,
    pub(crate) outer_glow: Option<OuterGlow>,
    pub(crate) inner_glow: Option<InnerGlow>,
    pub(crate) bevel: Option<BevelEmboss>,
    pub(crate) color_overlay: Option<ColorOverlay>,
}

/// A bounds-checked big-endian cursor over a record body.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let slice = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_be_bytes(self.take(2)?.try_into().ok()?))
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }

    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }

    fn fourcc(&mut self) -> Option<[u8; 4]> {
        self.take(4)?.try_into().ok()
    }
}

/// Decode a layer's legacy `lrFX` block, or `None` when it is absent or
/// structurally malformed.
pub(crate) fn decode_legacy_effects(layer: &Layer) -> Option<LegacyEffects> {
    let data = &layer.extra_block(b"lrFX")?.data;
    let mut r = Reader::new(data);
    if r.u16()? != 0 {
        return None;
    }
    let count = r.u16()?;
    let mut effects = LegacyEffects::default();
    let mut visible = true;
    for _ in 0..count {
        if r.fourcc()? != *b"8BIM" {
            return None;
        }
        let ostype = r.fourcc()?;
        let len = r.u32()? as usize;
        // A truncated body or a count that overruns the payload is structural.
        let body = r.take(len)?;
        let mut rr = Reader::new(body);
        match &ostype {
            b"cmnS" => {
                if let Some(v) = decode_common(&mut rr) {
                    visible = v;
                }
            }
            b"dsdw" => {
                if let Some(s) = decode_drop_shadow_record(&mut rr) {
                    effects.drop_shadow = Some(s);
                }
            }
            b"isdw" => {
                if let Some(s) = decode_inner_shadow_record(&mut rr) {
                    effects.inner_shadow = Some(s);
                }
            }
            b"oglw" => {
                if let Some(g) = decode_glow_record(&mut rr, false) {
                    effects.outer_glow = Some(g);
                }
            }
            b"iglw" => {
                if let Some(g) = decode_inner_glow_record(&mut rr) {
                    effects.inner_glow = Some(g);
                }
            }
            b"bevl" => {
                if let Some(b) = decode_bevel_record(&mut rr) {
                    effects.bevel = Some(b);
                }
            }
            b"sofi" => {
                if let Some(c) = decode_solid_fill_record(&mut rr) {
                    effects.color_overlay = Some(c);
                }
            }
            _ => {}
        }
    }
    if !visible {
        hide_all(&mut effects);
    }
    Some(effects)
}

/// `cmnS` common state: `u32` version, `u8` visible (two trailing pad bytes).
fn decode_common(r: &mut Reader) -> Option<bool> {
    let _version = r.u32()?;
    Some(r.u8()? != 0)
}

/// The fields a `dsdw`/`isdw` record shares, before mapping to the typed shadow.
struct ShadowRecord {
    enabled: bool,
    blend_mode: BlendMode,
    color: [u8; 3],
    opacity: f32,
    angle_deg: f32,
    distance: f32,
    amount: f32,
    size: f32,
    use_global_angle: bool,
}

/// A `ShadowInfo` record; `default` is the effect's blend mode for an unknown
/// layer key (Normal for a drop shadow, Multiply for an inner shadow).
fn shadow_record(r: &mut Reader, default: BlendMode) -> Option<ShadowRecord> {
    let _version = r.u32()?;
    let blur = r.u32()?;
    let intensity = r.u32()?;
    let angle = r.i32()?;
    let distance = r.u32()?;
    let color = decode_color(r)?;
    if r.fourcc()? != *b"8BIM" {
        return None;
    }
    let blend_key = r.fourcc()?;
    let enabled = r.u8()? != 0;
    let use_global_angle = r.u8()? != 0;
    let opacity = r.u8()?;
    Some(ShadowRecord {
        enabled,
        blend_mode: layer_blend(blend_key, default),
        color,
        opacity: opacity_pct(opacity),
        angle_deg: angle as f32,
        distance: (distance as f32).clamp(0.0, MAX_DISTANCE),
        amount: (intensity as f32).clamp(0.0, MAX_SPREAD),
        size: (blur as f32).clamp(0.0, MAX_SIZE),
        use_global_angle,
    })
}

fn decode_drop_shadow_record(r: &mut Reader) -> Option<DropShadow> {
    let s = shadow_record(r, BlendMode::Normal)?;
    Some(DropShadow {
        noise: 0.0,
        enabled: s.enabled,
        present: true,
        blend_mode: s.blend_mode,
        color: s.color,
        opacity: s.opacity,
        angle_deg: s.angle_deg,
        distance: s.distance,
        spread: s.amount,
        size: s.size,
        use_global_angle: s.use_global_angle,
        knocks_out: false,
    })
}

fn decode_inner_shadow_record(r: &mut Reader) -> Option<InnerShadow> {
    let s = shadow_record(r, BlendMode::Multiply)?;
    Some(InnerShadow {
        noise: 0.0,
        enabled: s.enabled,
        present: true,
        blend_mode: s.blend_mode,
        color: s.color,
        opacity: s.opacity,
        angle_deg: s.angle_deg,
        distance: s.distance,
        choke: s.amount.min(MAX_CHOKE),
        size: s.size,
        use_global_angle: s.use_global_angle,
        knocks_out: false,
    })
}

/// A `GlowInfo` record, before mapping to the typed outer/inner glow.
struct GlowRecord {
    enabled: bool,
    blend_mode: BlendMode,
    color: [u8; 3],
    opacity: f32,
    amount: f32,
    size: f32,
    invert: bool,
}

/// A `GlowInfo` record. The inner glow carries a version-2 `invert` byte; the
/// outer glow never does.
fn glow_record(r: &mut Reader, inner: bool) -> Option<GlowRecord> {
    let version = r.u32()?;
    let blur = r.u32()?;
    let intensity = r.u32()?;
    let color = decode_color(r)?;
    if r.fourcc()? != *b"8BIM" {
        return None;
    }
    let blend_key = r.fourcc()?;
    let enabled = r.u8()? != 0;
    let opacity = r.u8()?;
    let invert = inner && version >= 2 && r.u8()? != 0;
    Some(GlowRecord {
        enabled,
        blend_mode: layer_blend(blend_key, BlendMode::Screen),
        color,
        opacity: opacity_pct(opacity),
        amount: (intensity as f32).clamp(0.0, MAX_SPREAD),
        size: (blur as f32).clamp(0.0, MAX_SIZE),
        invert,
    })
}

fn decode_glow_record(r: &mut Reader, inner: bool) -> Option<OuterGlow> {
    let g = glow_record(r, inner)?;
    Some(OuterGlow {
        noise: 0.0,
        enabled: g.enabled,
        present: true,
        blend_mode: g.blend_mode,
        color: g.color,
        opacity: g.opacity,
        spread: g.amount,
        size: g.size,
        technique: GlowTechnique::Softer,
    })
}

fn decode_inner_glow_record(r: &mut Reader) -> Option<InnerGlow> {
    let g = glow_record(r, true)?;
    Some(InnerGlow {
        noise: 0.0,
        enabled: g.enabled,
        present: true,
        blend_mode: g.blend_mode,
        color: g.color,
        opacity: g.opacity,
        choke: g.amount.min(MAX_CHOKE),
        size: g.size,
        source: if g.invert {
            GlowSource::Center
        } else {
            GlowSource::Edge
        },
        technique: GlowTechnique::Softer,
    })
}

/// A `BevelInfo` record: angle, depth, blur, the two `8BIM`+blend pairs with
/// their colours and opacities, then style/enabled/use-global/direction.
fn decode_bevel_record(r: &mut Reader) -> Option<BevelEmboss> {
    let _version = r.u32()?;
    let angle = r.i32()?;
    let depth = r.u32()?;
    let blur = r.u32()?;
    if r.fourcc()? != *b"8BIM" {
        return None;
    }
    let highlight_key = r.fourcc()?;
    if r.fourcc()? != *b"8BIM" {
        return None;
    }
    let shadow_key = r.fourcc()?;
    let highlight_color = decode_color(r)?;
    let shadow_color = decode_color(r)?;
    let style = r.u8()?;
    let highlight_opacity = r.u8()?;
    let shadow_opacity = r.u8()?;
    let enabled = r.u8()? != 0;
    let use_global_angle = r.u8()? != 0;
    let direction = r.u8()?;
    Some(BevelEmboss {
        enabled,
        present: true,
        style: match style {
            0 => BevelStyle::Outer,
            1 => BevelStyle::Inner,
            2 => BevelStyle::Emboss,
            3 => BevelStyle::Pillow,
            4 => BevelStyle::Stroke,
            _ => BevelStyle::Inner,
        },
        technique: BevelTechnique::Smooth,
        direction: if direction == 0 {
            BevelDirection::Up
        } else {
            BevelDirection::Down
        },
        depth: (depth as f32).clamp(0.0, MAX_DEPTH),
        size: (blur as f32).clamp(0.0, MAX_SIZE),
        soften: 0.0,
        angle_deg: angle as f32,
        altitude_deg: 30.0f32.clamp(0.0, MAX_ALTITUDE),
        use_global_angle,
        gloss_contour: Default::default(),
        edge_contour: Default::default(),
        edge_contour_on: false,
        edge_range: 50.0,
        edge_anti_alias: false,
        highlight: BevelHighlight {
            mode: layer_blend(highlight_key, BlendMode::Screen),
            color: highlight_color,
            opacity: opacity_pct(highlight_opacity),
        },
        shadow: BevelShadow {
            mode: layer_blend(shadow_key, BlendMode::Multiply),
            color: shadow_color,
            opacity: opacity_pct(shadow_opacity),
        },
    })
}

/// A version-2 `sofi` solid fill; any other version is skipped.
fn decode_solid_fill_record(r: &mut Reader) -> Option<ColorOverlay> {
    if r.u32()? != 2 {
        return None;
    }
    if r.fourcc()? != *b"8BIM" {
        return None;
    }
    let blend_key = r.fourcc()?;
    let color = decode_color(r)?;
    let opacity = r.u8()?;
    let enabled = r.u8()? != 0;
    Some(ColorOverlay {
        enabled,
        present: true,
        blend_mode: layer_blend(blend_key, BlendMode::Normal),
        color,
        opacity: opacity_pct(opacity),
    })
}

/// The shared 16-bit `Color`: `u16` space (0 = RGB) then four `u16` channels.
/// Each channel maps to `u8` by its high byte.
fn decode_color(r: &mut Reader) -> Option<[u8; 3]> {
    if r.u16()? != 0 {
        return None;
    }
    let red = (r.u16()? >> 8) as u8;
    let green = (r.u16()? >> 8) as u8;
    let blue = (r.u16()? >> 8) as u8;
    let _unused = r.u16()?;
    Some([red, green, blue])
}

/// The layer-vocabulary blend key (`mul `/`scrn`), not `lfx2`'s capitalized
/// `BlnM`; an unknown key takes the effect default.
fn layer_blend(key: [u8; 4], default: BlendMode) -> BlendMode {
    BlendMode::from_psd_key(key).unwrap_or(default)
}

/// A stored `0..=255` opacity as a `0..=100` percent.
fn opacity_pct(byte: u8) -> f32 {
    (byte as f32 * 100.0 / 255.0).clamp(0.0, MAX_OPACITY)
}

/// A non-visible common state disables every record.
fn hide_all(e: &mut LegacyEffects) {
    if let Some(s) = e.drop_shadow.as_mut() {
        s.enabled = false;
    }
    if let Some(s) = e.inner_shadow.as_mut() {
        s.enabled = false;
    }
    if let Some(g) = e.outer_glow.as_mut() {
        g.enabled = false;
    }
    if let Some(g) = e.inner_glow.as_mut() {
        g.enabled = false;
    }
    if let Some(b) = e.bevel.as_mut() {
        b.enabled = false;
    }
    if let Some(c) = e.color_overlay.as_mut() {
        c.enabled = false;
    }
}

/// Scale the dimensional fields of supported legacy records without rewriting
/// unknown records or changing their visibility. Used only by Image Size.
pub(crate) fn scale_legacy_effects(layer: &mut Layer, factor: f64) -> bool {
    let Some(block) = layer.extra_block(b"lrFX") else {
        return false;
    };
    let mut data = block.data.to_vec();
    let mut reader = Reader::new(&data);
    if reader.u16() != Some(0) {
        return false;
    }
    let Some(count) = reader.u16() else {
        return false;
    };
    let mut fields = Vec::new();
    for _ in 0..count {
        if reader.fourcc() != Some(*b"8BIM") {
            return false;
        }
        let Some(kind) = reader.fourcc() else {
            return false;
        };
        let Some(len) = reader.u32() else {
            return false;
        };
        let start = reader.pos;
        let Some(body) = reader.take(len as usize) else {
            return false;
        };
        let valid = match &kind {
            b"dsdw" => decode_drop_shadow_record(&mut Reader::new(body)).is_some(),
            b"isdw" => decode_inner_shadow_record(&mut Reader::new(body)).is_some(),
            b"oglw" => decode_glow_record(&mut Reader::new(body), false).is_some(),
            b"iglw" => decode_inner_glow_record(&mut Reader::new(body)).is_some(),
            b"bevl" => decode_bevel_record(&mut Reader::new(body)).is_some(),
            _ => false,
        };
        if valid {
            match &kind {
                b"dsdw" | b"isdw" => fields.extend([start + 4, start + 16]),
                b"oglw" | b"iglw" => fields.push(start + 4),
                b"bevl" => fields.push(start + 12),
                _ => {}
            }
        }
    }
    let mut changed = false;
    for offset in fields {
        let bytes = &mut data[offset..offset + 4];
        let value = u32::from_be_bytes(bytes.try_into().expect("validated record"));
        let scaled = (f64::from(value) * factor)
            .round()
            .clamp(0.0, f64::from(u32::MAX)) as u32;
        if value != scaled {
            bytes.copy_from_slice(&scaled.to_be_bytes());
            changed = true;
        }
    }
    if changed {
        layer.set_metadata_block(*b"lrFX", Some(data.into()));
    }
    changed
}
