//! Map a filter `kind` (and an optional ordered list of parameter values) to a
//! concrete [`pictura_filters::Filter`].
//!
//! An absent parameter list yields the descriptor's opening defaults; a
//! non-empty fixed-list kind whose length differs from the descriptor's slot
//! count is refused, and the variable-list kinds accept only their documented
//! shapes. Slot order is the descriptor's control order; the C++ rows are
//! built from that same descriptor.
//!
//! Ported from perfecto25/photorust's filter menu parameter mapping.
//! Source: https://github.com/perfecto25/photorust

use choices::*;
use pictura_filters::{
    Light, Lighting, TextureOptions, TextureSurface, TonalFade, SHEAR_MAX_POINTS,
};

fn f(params: &[f64], i: usize, d: f64) -> f64 {
    params.get(i).copied().unwrap_or(d)
}

fn u8v(params: &[f64], i: usize, d: f64) -> u8 {
    f(params, i, d).clamp(0.0, 255.0).round() as u8
}

fn u32v(params: &[f64], i: usize, d: f64) -> u32 {
    f(params, i, d).max(0.0).round() as u32
}

fn i32v(params: &[f64], i: usize, d: f64) -> i32 {
    f(params, i, d).round() as i32
}

fn u64v(params: &[f64], i: usize, d: f64) -> u64 {
    f(params, i, d).max(0.0).round() as u64
}

fn flag(params: &[f64], i: usize, d: bool) -> bool {
    params.get(i).map(|v| *v >= 0.5).unwrap_or(d)
}

fn pick<T: Copy>(all: &[T], params: &[f64], i: usize, d: usize) -> T {
    let j = params
        .get(i)
        .map(|v| v.round().max(0.0) as usize)
        .unwrap_or(d);
    *all.get(j).unwrap_or(&all[d])
}

fn rgb(params: &[f64], base: usize, d: [f64; 3]) -> [u8; 3] {
    [
        u8v(params, base, d[0]),
        u8v(params, base + 1, d[1]),
        u8v(params, base + 2, d[2]),
    ]
}

const TEXTURE_SURFACES: [TextureSurface; 4] = [
    TextureSurface::Brick,
    TextureSurface::Burlap,
    TextureSurface::Canvas,
    TextureSurface::Sandstone,
];

/// Texture block: surface, scaling, relief, light direction, invert.
fn texture(params: &[f64], base: usize, defaults: &[f64]) -> TextureOptions {
    TextureOptions {
        surface: pick(
            &TEXTURE_SURFACES,
            params,
            base,
            defaults[base].round().max(0.0) as usize,
        ),
        scaling: u8v(params, base + 1, defaults[base + 1]),
        relief: u8v(params, base + 2, defaults[base + 2]),
        light_direction: u8v(params, base + 3, defaults[base + 3]),
        invert: flag(params, base + 4, defaults[base + 4] >= 0.5),
    }
}

mod mapping;
pub(super) use mapping::filter_from_kind_params;

/// Shear's slots: a point count, then `SHEAR_MAX_POINTS` `(position, offset)`
/// pairs, then the fill.
pub(super) const SHEAR_SLOTS: usize = 1 + 2 * SHEAR_MAX_POINTS + 1;

/// Lighting Effects' slots: the rig (colorize r/g/b, exposure, gloss, metallic,
/// ambience, texture, height), then per light: type, visible, colour r/g/b,
/// intensity, hotspot, centre x/y, angle, size, width, elevation. The arity
/// descriptor lists a one-light rig; any whole number of lights up to the cap maps.
pub(super) const LIGHTING_RIG_SLOTS: usize = 9;
pub(super) const LIGHTING_LIGHT_SLOTS: usize = 13;

fn lighting(params: &[f64], defaults: &[f64]) -> Option<Lighting> {
    if params.is_empty() {
        return Some(Lighting::default());
    }
    let light_slots = params.len().checked_sub(LIGHTING_RIG_SLOTS)?;
    let count = light_slots / LIGHTING_LIGHT_SLOTS;
    if count == 0 || count > pictura_filters::MAX_LIGHTS || light_slots % LIGHTING_LIGHT_SLOTS != 0
    {
        return None;
    }
    let base = LIGHTING_RIG_SLOTS;
    let light = |s: &[f64; LIGHTING_LIGHT_SLOTS]| Light {
        kind: pick(&LIGHT_TYPES, s, 0, defaults[base].round().max(0.0) as usize),
        on: flag(s, 1, defaults[base + 1] >= 0.5),
        color: rgb(
            s,
            2,
            [defaults[base + 2], defaults[base + 3], defaults[base + 4]],
        ),
        intensity: f(s, 5, defaults[base + 5]) as f32,
        hotspot: f(s, 6, defaults[base + 6]) as f32,
        center: (
            f(s, 7, defaults[base + 7]) as f32,
            f(s, 8, defaults[base + 8]) as f32,
        ),
        angle: f(s, 9, defaults[base + 9]) as f32,
        size: f(s, 10, defaults[base + 10]) as f32,
        width: f(s, 11, defaults[base + 11]) as f32,
        elevation: f(s, 12, defaults[base + 12]) as f32,
    };
    Some(Lighting {
        colorize: rgb(params, 0, [defaults[0], defaults[1], defaults[2]]),
        exposure: f(params, 3, defaults[3]) as f32,
        gloss: f(params, 4, defaults[4]) as f32,
        metallic: f(params, 5, defaults[5]) as f32,
        ambience: f(params, 6, defaults[6]) as f32,
        texture: pick(
            &TEXTURE_CHANNELS,
            params,
            7,
            defaults[7].round().max(0.0) as usize,
        ),
        height: f(params, 8, defaults[8]) as f32,
        lights: params[LIGHTING_RIG_SLOTS..]
            .as_chunks::<LIGHTING_LIGHT_SLOTS>()
            .0
            .iter()
            .map(light)
            .collect(),
    })
}

pub(crate) mod descriptor;
pub(super) use descriptor::{arity as filter_param_arity, commit_params};
mod gallery;

fn tonal_fade(params: &[f64], base: usize, defaults: &[f64]) -> TonalFade {
    TonalFade {
        amount: u8v(params, base, defaults[base]),
        width: u8v(params, base + 1, defaults[base + 1]),
        radius: u32v(params, base + 2, defaults[base + 2]),
    }
}

#[path = "filter_map_choices.rs"]
mod choices;

#[cfg(test)]
#[path = "filter_map_tests.rs"]
mod tests;
