//! Map a filter `kind` (and an optional ordered list of parameter values) to a
//! concrete [`pictura_filters::Filter`].
//!
//! An absent parameter list yields the documented defaults; a non-empty list
//! whose length differs from the kind's slot count is refused. Slot order for
//! each kind is the order its controls appear in the app's filter dialog and
//! must stay in lock-step with `cpp/filter_commands.cpp`.
//!
//! Ported from perfecto25/photorust's filter menu parameter mapping.
//! Source: https://github.com/perfecto25/photorust

use super::filter_tools::displace_maps::{displace_map, DISPLACE_SLOTS};
use super::filter_tools::lens_blur_depths::{depth_map as lens_blur_depth_map, LENS_BLUR_SLOTS};
use super::filter_tools::liquify::liquify_mesh;
use choices::*;
use pictura_core::shape::CUSTOM_SHAPE_NAMES;
use pictura_filters::{
    Filter, Light, Lighting, ShearFill, TextureOptions, TextureSurface, TonalFade, SHEAR_MAX_POINTS,
};

/// Refuse a non-empty parameter list that does not match the kind's arity.
macro_rules! arity {
    ($params:ident, $n:expr) => {
        if !$params.is_empty() && $params.len() != $n {
            return None;
        }
    };
}

fn f(params: &[f64], i: usize, d: f64) -> f64 {
    params.get(i).copied().unwrap_or(d)
}

fn u8v(params: &[f64], i: usize, d: u8) -> u8 {
    f(params, i, d as f64).clamp(0.0, 255.0).round() as u8
}

fn u32v(params: &[f64], i: usize, d: u32) -> u32 {
    f(params, i, d as f64).max(0.0).round() as u32
}

fn i32v(params: &[f64], i: usize, d: i32) -> i32 {
    f(params, i, d as f64).round() as i32
}

fn u64v(params: &[f64], i: usize, d: u64) -> u64 {
    f(params, i, d as f64).max(0.0).round() as u64
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

fn rgb(params: &[f64], base: usize, d: [u8; 3]) -> [u8; 3] {
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
fn texture(params: &[f64], base: usize) -> TextureOptions {
    TextureOptions {
        surface: pick(&TEXTURE_SURFACES, params, base, 2),
        scaling: u8v(params, base + 1, 100),
        relief: u8v(params, base + 2, 4),
        light_direction: u8v(params, base + 3, 0),
        invert: flag(params, base + 4, false),
    }
}

/// Custom's slots: the 5×5 weights row by row, then Scale and Offset.
pub(super) const CUSTOM_SLOTS: usize = 25 + 2;
/// Shear's slots: a point count, then `SHEAR_MAX_POINTS` `(position, offset)`
/// pairs, then the fill.
pub(super) const SHEAR_SLOTS: usize = 1 + 2 * SHEAR_MAX_POINTS + 1;

/// Lighting Effects' slots: the rig (colorize r/g/b, exposure, gloss, metallic,
/// ambience, texture, height), then per light: type, visible, colour r/g/b,
/// intensity, hotspot, centre x/y, angle, size, width, elevation. The arity
/// table lists a one-light rig; any whole number of lights up to the cap maps.
pub(super) const LIGHTING_RIG_SLOTS: usize = 9;
pub(super) const LIGHTING_LIGHT_SLOTS: usize = 13;

fn lighting(params: &[f64]) -> Option<Lighting> {
    if params.is_empty() {
        return Some(Lighting::default());
    }
    let light_slots = params.len().checked_sub(LIGHTING_RIG_SLOTS)?;
    let count = light_slots / LIGHTING_LIGHT_SLOTS;
    if count == 0 || count > pictura_filters::MAX_LIGHTS || light_slots % LIGHTING_LIGHT_SLOTS != 0
    {
        return None;
    }
    let d = Light::default();
    let light = |s: &[f64; LIGHTING_LIGHT_SLOTS]| Light {
        kind: pick(&LIGHT_TYPES, s, 0, 0),
        on: flag(s, 1, true),
        color: rgb(s, 2, d.color),
        intensity: f(s, 5, 0.0) as f32,
        hotspot: f(s, 6, 0.0) as f32,
        center: (f(s, 7, 0.0) as f32, f(s, 8, 0.0) as f32),
        angle: f(s, 9, 0.0) as f32,
        size: f(s, 10, 0.0) as f32,
        width: f(s, 11, 0.0) as f32,
        elevation: f(s, 12, 0.0) as f32,
    };
    Some(Lighting {
        colorize: rgb(params, 0, [255, 255, 255]),
        exposure: f(params, 3, 0.0) as f32,
        gloss: f(params, 4, 0.0) as f32,
        metallic: f(params, 5, 0.0) as f32,
        ambience: f(params, 6, 0.0) as f32,
        texture: pick(&TEXTURE_CHANNELS, params, 7, 0),
        height: f(params, 8, 0.0) as f32,
        lights: params[LIGHTING_RIG_SLOTS..]
            .as_chunks::<LIGHTING_LIGHT_SLOTS>()
            .0
            .iter()
            .map(light)
            .collect(),
    })
}

mod arities;
pub(super) use arities::{commit_params, filter_param_arity};

/// Build the filter for `kind` from an ordered parameter list.
///
/// `params` empty means "use the documented defaults". A non-empty `params`
/// whose length is not the kind's slot count returns `None`. An unknown kind
/// returns `None`.
pub(super) fn filter_from_kind_params(kind: &str, params: &[f64]) -> Option<Filter> {
    Some(match kind {
        "gaussian-blur" => {
            arity!(params, 1);
            Filter::GaussianBlur {
                radius: f(params, 0, 5.0),
            }
        }
        "box-blur" => {
            arity!(params, 1);
            Filter::BoxBlur {
                radius: u32v(params, 0, 3),
            }
        }
        // Radius, then the shape's index in the Custom Shape catalogue.
        "shape-blur" => {
            arity!(params, 2);
            Filter::ShapeBlur {
                radius: u32v(params, 0, 10),
                shape: (u32v(params, 1, 0) as usize).min(CUSTOM_SHAPE_NAMES.len() - 1),
            }
        }
        "surface-blur" => {
            arity!(params, 2);
            Filter::SurfaceBlur {
                radius: u32v(params, 0, 10),
                threshold: u8v(params, 1, 20),
            }
        }
        // Radius, Threshold, Quality (Low / Medium / High), Mode.
        "smart-blur" => {
            arity!(params, 4);
            Filter::SmartBlur {
                radius: f(params, 0, 3.0).clamp(0.1, 100.0),
                threshold: f(params, 1, 25.0).clamp(0.1, 100.0),
                quality: pick(&QUALITIES, params, 2, 0),
                mode: pick(&SMART_BLUR_MODES, params, 3, 0),
            }
        }
        "motion-blur" => {
            arity!(params, 2);
            Filter::MotionBlur {
                angle: f(params, 0, 0.0),
                distance: u32v(params, 1, 15),
            }
        }
        "lens-blur" => {
            arity!(params, LENS_BLUR_SLOTS);
            Filter::LensBlur {
                depth: match u32v(params, 1, 0) {
                    0 => None,
                    _ => lens_blur_depth_map(u32v(params, 13, 0)).map(Some)?,
                },
                focal: u8v(params, 2, 126),
                invert: flag(params, 3, false),
                shape: pick(&IRIS_SHAPES, params, 4, 0),
                radius: u32v(params, 5, 50),
                curvature: u8v(params, 6, 33),
                rotation: f(params, 7, 191.0),
                specular_brightness: u8v(params, 8, 44),
                specular_threshold: u8v(params, 9, 93),
                noise_amount: u8v(params, 10, 47),
                noise_distribution: pick(&NOISE_DISTRIBUTIONS, params, 11, 1),
                monochromatic: flag(params, 12, false),
                faster: u32v(params, 0, 0) == 0,
                seed: u64v(params, 14, 1),
            }
        }
        "radial-blur" => {
            arity!(params, 3);
            Filter::RadialBlur {
                method: pick(&RADIAL_METHODS, params, 0, 0),
                amount: f(params, 1, 10.0),
                quality: pick(&QUALITIES, params, 2, 1),
            }
        }
        "average" => {
            arity!(params, 0);
            Filter::Average
        }
        "blur" => {
            arity!(params, 0);
            Filter::Blur
        }
        "blur-more" => {
            arity!(params, 0);
            Filter::BlurMore
        }
        "median" => {
            arity!(params, 1);
            Filter::Median {
                radius: u32v(params, 0, 2),
            }
        }
        "despeckle" => {
            arity!(params, 0);
            Filter::Despeckle
        }
        "sharpen" => {
            arity!(params, 0);
            Filter::Sharpen
        }
        "sharpen-more" => {
            arity!(params, 0);
            Filter::SharpenMore
        }
        "sharpen-edges" => {
            arity!(params, 0);
            Filter::SharpenEdges
        }
        "unsharp-mask" => {
            arity!(params, 3);
            Filter::UnsharpMask {
                amount: f(params, 0, 150.0),
                radius: f(params, 1, 1.0),
                threshold: u8v(params, 2, 0),
            }
        }
        "add-noise" => {
            arity!(params, 4);
            Filter::AddNoise {
                amount: f(params, 0, 25.0),
                distribution: pick(&NOISE_DISTRIBUTIONS, params, 1, 0),
                monochromatic: flag(params, 2, false),
                seed: u64v(params, 3, 1),
            }
        }
        "maximum" => {
            arity!(params, 1);
            Filter::Maximum {
                radius: u32v(params, 0, 2),
            }
        }
        "minimum" => {
            arity!(params, 1);
            Filter::Minimum {
                radius: u32v(params, 0, 2),
            }
        }
        "offset" => {
            arity!(params, 6);
            Filter::Offset {
                horizontal: i32v(params, 0, 4),
                vertical: i32v(params, 1, 4),
                wrap: flag(params, 2, true),
                background: rgb(params, 3, [0, 0, 0]),
            }
        }
        "high-pass" => {
            arity!(params, 1);
            Filter::HighPass {
                radius: f(params, 0, 4.0),
            }
        }
        "custom" => {
            arity!(params, CUSTOM_SLOTS);
            let mut kernel = [[0.0; 5]; 5];
            for (i, k) in kernel.iter_mut().flatten().enumerate() {
                *k = f(params, i, if i == 12 { 1.0 } else { 0.0 });
            }
            Filter::Custom {
                kernel,
                scale: f(params, 25, 1.0),
                offset: f(params, 26, 0.0),
            }
        }
        "emboss" => {
            arity!(params, 3);
            Filter::Emboss {
                angle: f(params, 0, 135.0),
                height: f(params, 1, 2.0),
                amount: f(params, 2, 100.0),
            }
        }
        "find-edges" => {
            arity!(params, 0);
            Filter::FindEdges
        }
        "solarize" => {
            arity!(params, 0);
            Filter::Solarize
        }
        "diffuse" => {
            arity!(params, 1);
            Filter::Diffuse {
                mode: pick(&DIFFUSE_MODES, params, 0, 0),
            }
        }
        "glowing-edges" => {
            arity!(params, 3);
            Filter::GlowingEdges {
                width: u32v(params, 0, 2),
                brightness: u32v(params, 1, 6),
                smoothness: u32v(params, 2, 1),
            }
        }
        "mosaic" => {
            arity!(params, 1);
            Filter::Mosaic {
                cell_size: u32v(params, 0, 10),
            }
        }
        "crystallize" => {
            arity!(params, 2);
            Filter::Crystallize {
                cell_size: u32v(params, 0, 10),
                seed: u64v(params, 1, 1),
            }
        }
        "facet" => {
            arity!(params, 0);
            Filter::Facet
        }
        "fragment" => {
            arity!(params, 0);
            Filter::Fragment
        }
        "mezzotint" => {
            arity!(params, 2);
            Filter::Mezzotint {
                kind: pick(&MEZZOTINT_TYPES, params, 0, 0),
                seed: u64v(params, 1, 1),
            }
        }
        "pointillize" => {
            arity!(params, 5);
            Filter::Pointillize {
                cell_size: u32v(params, 0, 5),
                background: rgb(params, 1, [0, 0, 0]),
                seed: u64v(params, 4, 1),
            }
        }
        "color-halftone" => {
            arity!(params, 5);
            Filter::ColorHalftone {
                max_radius: u32v(params, 0, 5),
                angles: [
                    f(params, 1, 108.0),
                    f(params, 2, 162.0),
                    f(params, 3, 90.0),
                    f(params, 4, 45.0),
                ],
            }
        }
        "twirl" => {
            arity!(params, 1);
            Filter::Twirl {
                angle: f(params, 0, 90.0),
            }
        }
        "pinch" => {
            arity!(params, 1);
            Filter::Pinch {
                amount: f(params, 0, 50.0),
            }
        }
        "spherize" => {
            arity!(params, 2);
            Filter::Spherize {
                amount: f(params, 0, 100.0),
                mode: pick(&SPHERIZE_MODES, params, 1, 0),
            }
        }
        "ripple" => {
            arity!(params, 2);
            Filter::Ripple {
                amount: f(params, 0, 100.0),
                size: pick(&RIPPLE_SIZES, params, 1, 1),
            }
        }
        "wave" => {
            arity!(params, 9);
            Filter::Wave {
                generators: u32v(params, 0, 5),
                wavelength: (f(params, 1, 10.0), f(params, 2, 120.0)),
                amplitude: (f(params, 3, 5.0), f(params, 4, 35.0)),
                kind: pick(&WAVE_TYPES, params, 5, 0),
                scale: (f(params, 6, 100.0), f(params, 7, 100.0)),
                seed: u64v(params, 8, 1),
                repeat_edge: true,
            }
        }
        "polar-coordinates" => {
            arity!(params, 1);
            Filter::PolarCoordinates {
                kind: pick(&POLAR_KINDS, params, 0, 0),
            }
        }
        "shear" => {
            arity!(params, SHEAR_SLOTS);
            // A zero (or absent) point count is the straight default.
            let count = i32v(params, 0, 0);
            let curve = if count == 0 {
                vec![(-1.0, 0.0), (1.0, 0.0)]
            } else if (2..=SHEAR_MAX_POINTS as i32).contains(&count) {
                (0..count as usize)
                    .map(|i| (f(params, 1 + 2 * i, 0.0), f(params, 2 + 2 * i, 0.0)))
                    .collect()
            } else {
                return None;
            };
            Filter::Shear {
                curve,
                fill: pick(&SHEAR_FILLS, params, SHEAR_SLOTS - 1, 0),
            }
        }
        "displace" => {
            arity!(params, DISPLACE_SLOTS);
            Filter::Displace {
                map: displace_map(u32v(params, 4, 0))?,
                horizontal_scale: f(params, 0, 10.0),
                vertical_scale: f(params, 1, 10.0),
                fit: pick(&DISPLACE_FITS, params, 2, 0),
                wrap: pick(&SHEAR_FILLS, params, 3, 1) == ShearFill::WrapAround,
            }
        }
        "liquify" => {
            arity!(params, 1);
            Filter::Liquify {
                mesh: liquify_mesh(u32v(params, 0, 0))?,
            }
        }
        "zigzag" => {
            arity!(params, 3);
            Filter::ZigZag {
                amount: f(params, 0, 50.0),
                ridges: u32v(params, 1, 5),
                style: pick(&ZIGZAG_STYLES, params, 2, 0),
            }
        }
        "ocean-ripple" => {
            arity!(params, 3);
            Filter::OceanRipple {
                size: u32v(params, 0, 9),
                magnitude: u32v(params, 1, 9),
                seed: u64v(params, 2, 1),
            }
        }
        "glass" => {
            arity!(params, 5);
            Filter::Glass {
                distortion: u32v(params, 0, 5),
                smoothness: u32v(params, 1, 3),
                texture: pick(&GLASS_TEXTURES, params, 2, 2),
                scaling: u32v(params, 3, 100),
                invert: flag(params, 4, false),
            }
        }
        "diffuse-glow" => {
            arity!(params, 4);
            Filter::DiffuseGlow {
                graininess: u32v(params, 0, 6),
                glow_amount: u32v(params, 1, 10),
                clear_amount: u32v(params, 2, 15),
                seed: u64v(params, 3, 1),
            }
        }
        "clouds" => {
            arity!(params, 8);
            Filter::Clouds {
                color_a: rgb(params, 0, [0, 0, 0]),
                color_b: rgb(params, 3, [255, 255, 255]),
                starker: flag(params, 6, false),
                seed: u64v(params, 7, 1),
            }
        }
        "difference-clouds" => {
            arity!(params, 8);
            Filter::DifferenceClouds {
                color_a: rgb(params, 0, [0, 0, 0]),
                color_b: rgb(params, 3, [255, 255, 255]),
                starker: flag(params, 6, false),
                seed: u64v(params, 7, 1),
            }
        }
        "fibers" => {
            arity!(params, 9);
            Filter::Fibers {
                variance: f(params, 0, 16.0),
                strength: f(params, 1, 4.0),
                color_a: rgb(params, 2, [0, 0, 0]),
                color_b: rgb(params, 5, [255, 255, 255]),
                seed: u64v(params, 8, 1),
            }
        }
        "lens-flare" => {
            arity!(params, 4);
            Filter::LensFlare {
                brightness: f(params, 0, 100.0),
                center: (f(params, 1, 0.5), f(params, 2, 0.5)),
                lens: pick(&LENS_TYPES, params, 3, 0),
            }
        }
        "lighting-effects" => Filter::Lighting {
            lighting: lighting(params)?,
        },
        "colored-pencil" => {
            arity!(params, 4);
            Filter::ColoredPencil {
                pencil_width: u8v(params, 0, 4),
                stroke_pressure: u8v(params, 1, 8),
                paper_brightness: u8v(params, 2, 25),
                // ponytail: CS6's paper is the document background colour;
                // the dialog shows no swatch, so white stands in until the
                // bridge passes the document colours.
                background: [255, 255, 255],
                seed: u64v(params, 3, 1),
            }
        }
        "cutout" => {
            arity!(params, 3);
            Filter::Cutout {
                levels: u8v(params, 0, 4),
                edge_simplicity: u8v(params, 1, 4),
                edge_fidelity: u8v(params, 2, 2),
            }
        }
        "dry-brush" => {
            arity!(params, 4);
            Filter::DryBrush {
                brush_size: u8v(params, 0, 2),
                brush_detail: u8v(params, 1, 8),
                texture: u8v(params, 2, 1),
                seed: u64v(params, 3, 1),
            }
        }
        "film-grain" => {
            arity!(params, 4);
            Filter::FilmGrain {
                grain: u8v(params, 0, 4),
                highlight_area: u8v(params, 1, 0),
                intensity: u8v(params, 2, 10),
                seed: u64v(params, 3, 1),
            }
        }
        "fresco" => {
            arity!(params, 4);
            Filter::Fresco {
                brush_size: u8v(params, 0, 2),
                brush_detail: u8v(params, 1, 8),
                texture: u8v(params, 2, 1),
                seed: u64v(params, 3, 1),
            }
        }
        "neon-glow" => {
            arity!(params, 5);
            Filter::NeonGlow {
                glow_size: i32v(params, 0, 5),
                glow_brightness: u8v(params, 1, 15),
                glow_color: rgb(params, 2, [0, 0, 255]),
            }
        }
        "paint-daubs" => {
            arity!(params, 4);
            Filter::PaintDaubs {
                brush_size: u8v(params, 0, 8),
                sharpness: u8v(params, 1, 7),
                brush_type: pick(&BRUSH_TYPES, params, 2, 0),
                seed: u64v(params, 3, 1),
            }
        }
        "palette-knife" => {
            arity!(params, 3);
            Filter::PaletteKnife {
                stroke_size: u8v(params, 0, 25),
                stroke_detail: u8v(params, 1, 3),
                softness: u8v(params, 2, 0),
            }
        }
        "plastic-wrap" => {
            arity!(params, 3);
            Filter::PlasticWrap {
                highlight_strength: u8v(params, 0, 15),
                detail: u8v(params, 1, 9),
                smoothness: u8v(params, 2, 7),
            }
        }
        "poster-edges" => {
            arity!(params, 3);
            Filter::PosterEdges {
                edge_thickness: u8v(params, 0, 2),
                edge_intensity: u8v(params, 1, 1),
                posterization: u8v(params, 2, 2),
            }
        }
        "rough-pastels" => {
            arity!(params, 7);
            Filter::RoughPastels {
                stroke_length: u8v(params, 0, 6),
                stroke_detail: u8v(params, 1, 4),
                texture: TextureOptions {
                    relief: u8v(params, 4, 20),
                    ..texture(params, 2)
                },
                seed: 1,
            }
        }
        "smudge-stick" => {
            arity!(params, 4);
            Filter::SmudgeStick {
                stroke_length: u8v(params, 0, 2),
                highlight_area: u8v(params, 1, 0),
                intensity: u8v(params, 2, 10),
                seed: u64v(params, 3, 1),
            }
        }
        "sponge" => {
            arity!(params, 4);
            Filter::Sponge {
                brush_size: u8v(params, 0, 2),
                definition: u8v(params, 1, 12),
                smoothness: u8v(params, 2, 5),
                seed: u64v(params, 3, 1),
            }
        }
        "underpainting" => {
            arity!(params, 8);
            Filter::Underpainting {
                brush_size: u8v(params, 0, 6),
                texture_coverage: u8v(params, 1, 16),
                texture: TextureOptions {
                    light_direction: u8v(params, 5, 4),
                    ..texture(params, 2)
                },
                seed: u64v(params, 7, 1),
            }
        }
        "watercolor" => {
            arity!(params, 4);
            Filter::Watercolor {
                brush_detail: u8v(params, 0, 9),
                shadow_intensity: u8v(params, 1, 1),
                texture: u8v(params, 2, 1),
                seed: u64v(params, 3, 1),
            }
        }
        "accented-edges" => {
            arity!(params, 3);
            Filter::AccentedEdges {
                edge_width: u8v(params, 0, 2),
                edge_brightness: u8v(params, 1, 38),
                smoothness: u8v(params, 2, 5),
            }
        }
        "angled-strokes" => {
            arity!(params, 3);
            Filter::AngledStrokes {
                direction_balance: u8v(params, 0, 50),
                stroke_length: u8v(params, 1, 15),
                sharpness: u8v(params, 2, 3),
            }
        }
        "crosshatch" => {
            arity!(params, 3);
            Filter::Crosshatch {
                stroke_length: u8v(params, 0, 9),
                sharpness: u8v(params, 1, 6),
                strength: u8v(params, 2, 1),
            }
        }
        "dark-strokes" => {
            arity!(params, 3);
            Filter::DarkStrokes {
                balance: u8v(params, 0, 5),
                black_intensity: u8v(params, 1, 6),
                white_intensity: u8v(params, 2, 5),
            }
        }
        "ink-outlines" => {
            arity!(params, 3);
            Filter::InkOutlines {
                stroke_length: u8v(params, 0, 10),
                dark_intensity: u8v(params, 1, 25),
                light_intensity: u8v(params, 2, 25),
            }
        }
        "spatter" => {
            arity!(params, 3);
            Filter::Spatter {
                spray_radius: u8v(params, 0, 10),
                smoothness: u8v(params, 1, 5),
                seed: u64v(params, 2, 1),
            }
        }
        "sprayed-strokes" => {
            arity!(params, 4);
            Filter::SprayedStrokes {
                stroke_length: u8v(params, 0, 12),
                spray_radius: u8v(params, 1, 7),
                direction: pick(&STROKE_DIRECTIONS, params, 2, 0),
                seed: u64v(params, 3, 1),
            }
        }
        "sumi-e" => {
            arity!(params, 3);
            Filter::SumiE {
                stroke_width: u8v(params, 0, 8),
                stroke_pressure: u8v(params, 1, 5),
                contrast: u8v(params, 2, 20),
            }
        }
        "bas-relief" => {
            arity!(params, 9);
            Filter::BasRelief {
                detail: u8v(params, 0, 6),
                smoothness: u8v(params, 1, 3),
                light_direction: pick(&LIGHT_DIRECTIONS, params, 2, 0),
                foreground: rgb(params, 3, [0, 0, 0]),
                background: rgb(params, 6, [255, 255, 255]),
            }
        }
        "chalk-charcoal" => {
            arity!(params, 10);
            Filter::ChalkCharcoal {
                charcoal_area: u8v(params, 0, 6),
                chalk_area: u8v(params, 1, 6),
                stroke_pressure: u8v(params, 2, 1),
                foreground: rgb(params, 3, [0, 0, 0]),
                background: rgb(params, 6, [255, 255, 255]),
                seed: u64v(params, 9, 1),
            }
        }
        "charcoal" => {
            arity!(params, 10);
            Filter::Charcoal {
                thickness: u8v(params, 0, 1),
                detail: u8v(params, 1, 3),
                light_dark_balance: u8v(params, 2, 50),
                foreground: rgb(params, 3, [0, 0, 0]),
                background: rgb(params, 6, [255, 255, 255]),
                seed: u64v(params, 9, 1),
            }
        }
        "chrome" => {
            arity!(params, 2);
            Filter::Chrome {
                detail: u8v(params, 0, 4),
                smoothness: u8v(params, 1, 7),
            }
        }
        "conte-crayon" => {
            arity!(params, 14);
            Filter::ConteCrayon {
                foreground_level: u8v(params, 0, 8),
                background_level: u8v(params, 1, 7),
                texture: texture(params, 2),
                foreground: rgb(params, 7, [0, 0, 0]),
                background: rgb(params, 10, [255, 255, 255]),
                seed: u64v(params, 13, 1),
            }
        }
        "graphic-pen" => {
            arity!(params, 9);
            Filter::GraphicPen {
                stroke_length: u8v(params, 0, 6),
                light_dark_balance: u8v(params, 1, 50),
                direction: pick(&STROKE_DIRECTIONS, params, 2, 0),
                foreground: rgb(params, 3, [0, 0, 0]),
                background: rgb(params, 6, [255, 255, 255]),
            }
        }
        "halftone-pattern" => {
            arity!(params, 3);
            Filter::HalftonePattern {
                size: u8v(params, 0, 5),
                contrast: u8v(params, 1, 5),
                pattern: pick(&HALFTONE_TYPES, params, 2, 0),
            }
        }
        "note-paper" => {
            arity!(params, 4);
            Filter::NotePaper {
                image_balance: u8v(params, 0, 25),
                graininess: u8v(params, 1, 10),
                relief: u8v(params, 2, 11),
                seed: u64v(params, 3, 1),
            }
        }
        "photocopy" => {
            arity!(params, 2);
            Filter::Photocopy {
                detail: u8v(params, 0, 5),
                darkness: u8v(params, 1, 20),
            }
        }
        "plaster" => {
            arity!(params, 9);
            Filter::Plaster {
                image_balance: u8v(params, 0, 25),
                smoothness: u8v(params, 1, 2),
                light_direction: pick(&LIGHT_DIRECTIONS, params, 2, 0),
                foreground: rgb(params, 3, [0, 0, 0]),
                background: rgb(params, 6, [255, 255, 255]),
            }
        }
        "reticulation" => {
            arity!(params, 10);
            Filter::Reticulation {
                density: u8v(params, 0, 13),
                black_level: u8v(params, 1, 10),
                white_level: u8v(params, 2, 40),
                foreground: rgb(params, 3, [0, 0, 0]),
                background: rgb(params, 6, [255, 255, 255]),
                seed: u64v(params, 9, 1),
            }
        }
        "stamp" => {
            arity!(params, 8);
            Filter::Stamp {
                light_dark_balance: u8v(params, 0, 25),
                smoothness: u8v(params, 1, 5),
                foreground: rgb(params, 2, [0, 0, 0]),
                background: rgb(params, 5, [255, 255, 255]),
            }
        }
        "torn-edges" => {
            arity!(params, 9);
            Filter::TornEdges {
                image_balance: u8v(params, 0, 25),
                smoothness: u8v(params, 1, 1),
                contrast: u8v(params, 2, 8),
                foreground: rgb(params, 3, [0, 0, 0]),
                background: rgb(params, 6, [255, 255, 255]),
            }
        }
        "water-paper" => {
            arity!(params, 4);
            Filter::WaterPaper {
                fiber_length: u8v(params, 0, 15),
                brightness: u8v(params, 1, 45),
                contrast: u8v(params, 2, 60),
                seed: u64v(params, 3, 1),
            }
        }
        "craquelure" => {
            arity!(params, 3);
            Filter::Craquelure {
                crack_spacing: u8v(params, 0, 10),
                crack_depth: u8v(params, 1, 6),
                crack_brightness: u8v(params, 2, 9),
            }
        }
        "grain" => {
            arity!(params, 7);
            Filter::Grain {
                intensity: u8v(params, 0, 40),
                contrast: u8v(params, 1, 50),
                grain_type: pick(&GRAIN_TYPES, params, 2, 0),
                background: rgb(params, 3, [255, 255, 255]),
                seed: u64v(params, 6, 1),
            }
        }
        "mosaic-tiles" => {
            arity!(params, 4);
            Filter::MosaicTiles {
                tile_size: u8v(params, 0, 12),
                grout_width: u8v(params, 1, 3),
                lighten_grout: u8v(params, 2, 1),
                seed: u64v(params, 3, 1),
            }
        }
        "patchwork" => {
            arity!(params, 3);
            Filter::Patchwork {
                square_size: u8v(params, 0, 5),
                relief: u8v(params, 1, 8),
                seed: u64v(params, 2, 1),
            }
        }
        "stained-glass" => {
            arity!(params, 7);
            Filter::StainedGlass {
                cell_size: u8v(params, 0, 10),
                border_thickness: u8v(params, 1, 4),
                light_intensity: u8v(params, 2, 5),
                foreground: rgb(params, 3, [0, 0, 0]),
                seed: u64v(params, 6, 1),
            }
        }
        "texturizer" => {
            arity!(params, 5);
            Filter::Texturizer {
                texture: texture(params, 0),
            }
        }
        "oil-paint" => {
            arity!(params, 6);
            Filter::OilPaint {
                stylization: f(params, 0, 3.5),
                cleanliness: f(params, 1, 4.5),
                scale: f(params, 2, 0.75),
                bristle_detail: f(params, 3, 3.0),
                angular_direction: f(params, 4, 85.0),
                shine: f(params, 5, 0.55),
            }
        }
        "dust-and-scratches" => {
            arity!(params, 2);
            Filter::DustAndScratches {
                radius: u32v(params, 0, 1),
                threshold: u32v(params, 1, 0),
            }
        }
        "extrude" => {
            arity!(params, 6);
            Filter::Extrude {
                kind: pick(&EXTRUDE_TYPES, params, 0, 0),
                size: u32v(params, 1, 30),
                depth: f(params, 2, 30.0) as f32,
                level_based: flag(params, 3, true),
                solid_front: flag(params, 4, false),
                mask_incomplete: flag(params, 5, false),
            }
        }
        "tiles" => {
            arity!(params, 9);
            Filter::Tiles {
                count: u32v(params, 0, 10),
                offset: u32v(params, 1, 10),
                fill: pick(&TILE_FILLS, params, 2, 0),
                foreground: rgb(params, 3, [0, 0, 0]),
                background: rgb(params, 6, [255, 255, 255]),
            }
        }
        "trace-contour" => {
            arity!(params, 2);
            Filter::TraceContour {
                level: u8v(params, 0, 128),
                edge: pick(&CONTOUR_EDGES, params, 1, 0),
            }
        }
        "wind" => {
            arity!(params, 2);
            Filter::Wind {
                method: pick(&WIND_METHODS, params, 0, 0),
                from_right: pick(&WIND_FROM_RIGHT, params, 1, 0),
            }
        }
        "smart-sharpen" => {
            arity!(params, 12);
            Filter::SmartSharpen {
                amount: f(params, 0, 100.0),
                radius: f(params, 1, 1.0),
                reduce_noise: f(params, 2, 0.0),
                remove: pick(&SHARPEN_REMOVES, params, 3, 0),
                angle: f(params, 4, 0.0),
                more_accurate: flag(params, 5, false),
                shadow: tonal_fade(params, 6, TonalFade::default()),
                highlight: tonal_fade(params, 9, TonalFade::default()),
            }
        }
        // `Custom` requires a caller-supplied 5x5 kernel, so no meaningful
        // default exists; it stays out of the dock and is left unmapped.
        _ => return None,
    })
}

fn tonal_fade(params: &[f64], base: usize, d: TonalFade) -> TonalFade {
    TonalFade {
        amount: u8v(params, base, d.amount),
        width: u8v(params, base + 1, d.width),
        radius: u32v(params, base + 2, d.radius),
    }
}

#[path = "filter_map_choices.rs"]
mod choices;

#[cfg(test)]
#[path = "filter_map_tests.rs"]
mod tests;
