//! Slot-to-filter builders. Each catalogue row points at the builder for its
//! kind; the shared helpers here decode scalar slots and nested records.

pub(crate) use crate::catalogue::{
    BuildCtx, LIGHTING_LIGHT_SLOTS, LIGHTING_RIG_SLOTS, SHEAR_SLOTS,
};
pub(crate) use crate::filter::Filter;
pub(crate) use crate::{
    AwaCorrection, AwaParams, ChannelDetail, FrameOptions, LensCorrectionParams, Light, Lighting,
    ReduceNoiseParams, ShearFill, TextureOptions, TextureSurface, TonalFade, MAX_LIGHTS,
    SHEAR_MAX_POINTS,
};
pub(crate) use choices::*;
pub(crate) use pictura_core::shape::CUSTOM_SHAPE_NAMES;

pub(crate) mod blur;
mod choices;
pub(crate) mod distort;
pub(crate) mod engine_only;
pub(crate) mod gallery;
pub(crate) mod paint;
pub(crate) mod render;
pub(crate) mod stylize;
pub(crate) mod video;

const TEXTURE_SURFACES: [TextureSurface; 4] = [
    TextureSurface::Brick,
    TextureSurface::Burlap,
    TextureSurface::Canvas,
    TextureSurface::Sandstone,
];

pub(crate) const CROP_RANGE: (f64, f64) = (0.1, 10.0);
const EQUIVALENT_RANGE: (f64, f64) = (4.05 * 0.57, 121.41 * 0.57);

pub(crate) fn focal_range(crop: f64) -> (f64, f64) {
    let crop = crop.clamp(CROP_RANGE.0, CROP_RANGE.1);
    (EQUIVALENT_RANGE.0 / crop, EQUIVALENT_RANGE.1 / crop)
}

pub(crate) fn awa_correction(value: f64) -> AwaCorrection {
    match value.round() as i64 {
        1 => AwaCorrection::Perspective,
        2 => AwaCorrection::FullSpherical,
        _ => AwaCorrection::Fisheye,
    }
}

pub(crate) fn pick<T: Copy>(all: &[T], ctx: &BuildCtx, i: usize, d: usize) -> T {
    ctx.pick(all, i, d)
}

pub(crate) fn rgb(ctx: &BuildCtx, base: usize, _default: [f64; 3]) -> [u8; 3] {
    ctx.rgb(base)
}

pub(crate) fn texture(ctx: &BuildCtx, base: usize) -> TextureOptions {
    TextureOptions {
        surface: ctx.pick(
            &TEXTURE_SURFACES,
            base,
            ctx.d(base).round().max(0.0) as usize,
        ),
        scaling: ctx.u8v(base + 1),
        relief: ctx.u8v(base + 2),
        light_direction: ctx.u8v(base + 3),
        invert: ctx.flag(base + 4),
    }
}

pub(crate) fn tonal_fade(ctx: &BuildCtx, base: usize) -> TonalFade {
    TonalFade {
        amount: ctx.u8v(base),
        width: ctx.u8v(base + 1),
        radius: ctx.u32v(base + 2),
    }
}

pub(crate) fn lighting(ctx: &BuildCtx) -> Option<Lighting> {
    let params = ctx.params;
    if params.is_empty() {
        return Some(Lighting::default());
    }
    let light_slots = params.len().checked_sub(LIGHTING_RIG_SLOTS)?;
    let count = light_slots / LIGHTING_LIGHT_SLOTS;
    if count == 0 || count > MAX_LIGHTS || light_slots % LIGHTING_LIGHT_SLOTS != 0 {
        return None;
    }
    let base = LIGHTING_RIG_SLOTS;
    let light = |s: &[f64]| Light {
        kind: pick_slice(&LIGHT_TYPES, s, 0, ctx.d(base).round().max(0.0) as usize),
        on: s.get(1).copied().unwrap_or(ctx.d(base + 1)) >= 0.5,
        color: rgb_slice(s, 2, [ctx.d(base + 2), ctx.d(base + 3), ctx.d(base + 4)]),
        intensity: s.get(5).copied().unwrap_or(ctx.d(base + 5)) as f32,
        hotspot: s.get(6).copied().unwrap_or(ctx.d(base + 6)) as f32,
        center: (
            s.get(7).copied().unwrap_or(ctx.d(base + 7)) as f32,
            s.get(8).copied().unwrap_or(ctx.d(base + 8)) as f32,
        ),
        angle: s.get(9).copied().unwrap_or(ctx.d(base + 9)) as f32,
        size: s.get(10).copied().unwrap_or(ctx.d(base + 10)) as f32,
        width: s.get(11).copied().unwrap_or(ctx.d(base + 11)) as f32,
        elevation: s.get(12).copied().unwrap_or(ctx.d(base + 12)) as f32,
    };
    Some(Lighting {
        colorize: ctx.rgb(0),
        exposure: ctx.f(3) as f32,
        gloss: ctx.f(4) as f32,
        metallic: ctx.f(5) as f32,
        ambience: ctx.f(6) as f32,
        texture: ctx.pick(&TEXTURE_CHANNELS, 7, ctx.d(7).round().max(0.0) as usize),
        height: ctx.f(8) as f32,
        lights: params[LIGHTING_RIG_SLOTS..]
            .as_chunks::<LIGHTING_LIGHT_SLOTS>()
            .0
            .iter()
            .map(|s| light(s))
            .collect(),
    })
}

fn pick_slice<T: Copy>(all: &[T], s: &[f64], i: usize, d: usize) -> T {
    let j = s.get(i).map(|v| v.round().max(0.0) as usize).unwrap_or(d);
    *all.get(j).unwrap_or(&all[d])
}

fn rgb_slice(s: &[f64], base: usize, d: [f64; 3]) -> [u8; 3] {
    [
        u8_slice(s, base, d[0]),
        u8_slice(s, base + 1, d[1]),
        u8_slice(s, base + 2, d[2]),
    ]
}

fn u8_slice(s: &[f64], i: usize, d: f64) -> u8 {
    s.get(i).copied().unwrap_or(d).clamp(0.0, 255.0).round() as u8
}
