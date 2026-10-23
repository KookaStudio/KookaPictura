//! The filter-kind mapping: turn a kind and its ordered slots into an engine
//! filter. The descriptor supplies the kind's canonical arity and opening
//! defaults; the variable-list decoders keep their own record grammar.

use super::super::filter_tools::adaptive_wide_angle::{awa_constraints, awa_correction};
use super::super::filter_tools::displace_maps::displace_map;
use super::super::filter_tools::lens_blur_depths::depth_map as lens_blur_depth_map;
use super::super::filter_tools::liquify::liquify_mesh;
use super::choices::*;
use super::descriptor;
use super::gallery;
use super::{
    f, flag, i32v, lighting, pick, rgb, texture, tonal_fade, u32v, u64v, u8v, SHEAR_SLOTS,
};
use pictura_core::shape::CUSTOM_SHAPE_NAMES;
use pictura_filters::{
    AwaParams, ChannelDetail, Filter, FrameOptions, ReduceNoiseParams, ShearFill, TextureOptions,
    SHEAR_MAX_POINTS,
};

/// Refuse a non-empty parameter list that does not match the kind's arity.
macro_rules! arity {
    ($params:ident, $n:expr) => {
        if !$params.is_empty() && $params.len() != $n {
            return None;
        }
    };
}

/// Build the filter for `kind` from an ordered parameter list.
///
/// `params` empty means "use the documented defaults". A non-empty `params`
/// whose length is not the kind's slot count returns `None`. An unknown kind
/// returns `None`.
pub(crate) fn filter_from_kind_params(kind: &str, params: &[f64]) -> Option<Filter> {
    let descriptor = descriptor::descriptor(kind);
    let slots = descriptor.map_or(0, |descriptor| descriptor.slots());
    let defaults = descriptor
        .map(|descriptor| descriptor.defaults())
        .unwrap_or_default();
    let params = if params.is_empty() {
        defaults.as_slice()
    } else {
        params
    };
    Some(match kind {
        "gaussian-blur" => {
            arity!(params, slots);
            Filter::GaussianBlur {
                radius: f(params, 0, defaults[0]),
            }
        }
        "box-blur" => {
            arity!(params, slots);
            Filter::BoxBlur {
                radius: u32v(params, 0, defaults[0]),
            }
        }
        // Radius, then the shape's index in the Custom Shape catalogue.
        "shape-blur" => {
            arity!(params, slots);
            Filter::ShapeBlur {
                radius: u32v(params, 0, defaults[0]),
                shape: (u32v(params, 1, defaults[1]) as usize).min(CUSTOM_SHAPE_NAMES.len() - 1),
            }
        }
        "surface-blur" => {
            arity!(params, slots);
            Filter::SurfaceBlur {
                radius: u32v(params, 0, defaults[0]),
                threshold: u8v(params, 1, defaults[1]),
            }
        }
        // Radius, Threshold, Quality (Low / Medium / High), Mode.
        "smart-blur" => {
            arity!(params, slots);
            Filter::SmartBlur {
                radius: f(params, 0, defaults[0]).clamp(0.1, 100.0),
                threshold: f(params, 1, defaults[1]).clamp(0.1, 100.0),
                quality: pick(&QUALITIES, params, 2, defaults[2].round().max(0.0) as usize),
                mode: pick(
                    &SMART_BLUR_MODES,
                    params,
                    3,
                    defaults[3].round().max(0.0) as usize,
                ),
            }
        }
        "motion-blur" => {
            arity!(params, slots);
            Filter::MotionBlur {
                angle: f(params, 0, defaults[0]),
                distance: u32v(params, 1, defaults[1]),
            }
        }
        "lens-blur" => {
            arity!(params, slots);
            Filter::LensBlur {
                depth: match u32v(params, 1, defaults[1]) {
                    0 => None,
                    _ => lens_blur_depth_map(u32v(params, 13, defaults[13])).map(Some)?,
                },
                focal: u8v(params, 2, defaults[2]),
                invert: flag(params, 3, defaults[3] >= 0.5),
                shape: pick(
                    &IRIS_SHAPES,
                    params,
                    4,
                    defaults[4].round().max(0.0) as usize,
                ),
                radius: u32v(params, 5, defaults[5]),
                curvature: u8v(params, 6, defaults[6]),
                rotation: f(params, 7, defaults[7]),
                specular_brightness: u8v(params, 8, defaults[8]),
                specular_threshold: u8v(params, 9, defaults[9]),
                noise_amount: u8v(params, 10, defaults[10]),
                noise_distribution: pick(
                    &NOISE_DISTRIBUTIONS,
                    params,
                    11,
                    defaults[11].round().max(0.0) as usize,
                ),
                monochromatic: flag(params, 12, defaults[12] >= 0.5),
                faster: u32v(params, 0, defaults[0]) == 0,
                seed: u64v(params, 14, defaults[14]),
            }
        }
        "field-blur" => gallery::field_blur(params)?,
        "iris-blur" => gallery::iris_blur(params)?,
        "tilt-shift" => gallery::tilt_shift(params)?,
        "path-blur" => gallery::path_blur(params)?,
        "spin-blur" => gallery::spin_blur(params)?,
        "blur-gallery" => gallery::blur_gallery(params)?,
        "radial-blur" => {
            arity!(params, slots);
            Filter::RadialBlur {
                method: pick(
                    &RADIAL_METHODS,
                    params,
                    0,
                    defaults[0].round().max(0.0) as usize,
                ),
                amount: f(params, 1, defaults[1]),
                quality: pick(&QUALITIES, params, 2, defaults[2].round().max(0.0) as usize),
                center: (f(params, 3, defaults[3]), f(params, 4, defaults[4])),
            }
        }
        "average" => {
            arity!(params, slots);
            Filter::Average
        }
        "blur" => {
            arity!(params, slots);
            Filter::Blur
        }
        "blur-more" => {
            arity!(params, slots);
            Filter::BlurMore
        }
        "median" => {
            arity!(params, slots);
            Filter::Median {
                radius: u32v(params, 0, defaults[0]),
            }
        }
        "despeckle" => {
            arity!(params, slots);
            Filter::Despeckle
        }
        // Composite Strength/Preserve Details, Reduce Color Noise, Sharpen
        // Details, Remove JPEG Artifacts, an Advanced flag, then the per-channel
        // Strength/Preserve pairs for R, G and B. With Advanced off the
        // composite pair stands for every channel.
        "reduce-noise" => {
            arity!(params, slots);
            let composite = ChannelDetail {
                strength: f(params, 0, defaults[0]),
                preserve_details: f(params, 1, defaults[1]),
            };
            let detail = |base: usize| ChannelDetail {
                strength: f(params, base, defaults[base]),
                preserve_details: f(params, base + 1, defaults[base + 1]),
            };
            Filter::ReduceNoise(ReduceNoiseParams {
                reduce_color_noise: f(params, 2, defaults[2]),
                sharpen_details: f(params, 3, defaults[3]),
                remove_jpeg_artifacts: flag(params, 4, defaults[4] >= 0.5),
                channels: if flag(params, 5, defaults[5] >= 0.5) {
                    [detail(6), detail(8), detail(10)]
                } else {
                    [composite; 3]
                },
            })
        }
        "sharpen" => {
            arity!(params, slots);
            Filter::Sharpen
        }
        "sharpen-more" => {
            arity!(params, slots);
            Filter::SharpenMore
        }
        "sharpen-edges" => {
            arity!(params, slots);
            Filter::SharpenEdges
        }
        "unsharp-mask" => {
            arity!(params, slots);
            Filter::UnsharpMask {
                amount: f(params, 0, defaults[0]),
                radius: f(params, 1, defaults[1]),
                threshold: u8v(params, 2, defaults[2]),
            }
        }
        "add-noise" => {
            arity!(params, slots);
            Filter::AddNoise {
                amount: f(params, 0, defaults[0]),
                distribution: pick(
                    &NOISE_DISTRIBUTIONS,
                    params,
                    1,
                    defaults[1].round().max(0.0) as usize,
                ),
                monochromatic: flag(params, 2, defaults[2] >= 0.5),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "maximum" => {
            arity!(params, slots);
            Filter::Maximum {
                radius: u32v(params, 0, defaults[0]),
            }
        }
        "minimum" => {
            arity!(params, slots);
            Filter::Minimum {
                radius: u32v(params, 0, defaults[0]),
            }
        }
        "offset" => {
            arity!(params, slots);
            Filter::Offset {
                horizontal: i32v(params, 0, defaults[0]),
                vertical: i32v(params, 1, defaults[1]),
                wrap: flag(params, 2, defaults[2] >= 0.5),
                background: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                transparent: flag(params, 6, defaults[6] >= 0.5),
            }
        }
        "high-pass" => {
            arity!(params, slots);
            Filter::HighPass {
                radius: f(params, 0, defaults[0]),
            }
        }
        "custom" => {
            arity!(params, slots);
            let mut kernel = [[0.0; 5]; 5];
            for (i, k) in kernel.iter_mut().flatten().enumerate() {
                *k = f(params, i, defaults[i]);
            }
            Filter::Custom {
                kernel,
                scale: f(params, 25, defaults[25]),
                offset: f(params, 26, defaults[26]),
            }
        }
        "emboss" => {
            arity!(params, slots);
            Filter::Emboss {
                angle: f(params, 0, defaults[0]),
                height: f(params, 1, defaults[1]),
                amount: f(params, 2, defaults[2]),
            }
        }
        "find-edges" => {
            arity!(params, slots);
            Filter::FindEdges
        }
        "solarize" => {
            arity!(params, slots);
            Filter::Solarize
        }
        "diffuse" => {
            arity!(params, slots);
            Filter::Diffuse {
                mode: pick(
                    &DIFFUSE_MODES,
                    params,
                    0,
                    defaults[0].round().max(0.0) as usize,
                ),
            }
        }
        "glowing-edges" => {
            arity!(params, slots);
            Filter::GlowingEdges {
                width: u32v(params, 0, defaults[0]),
                brightness: u32v(params, 1, defaults[1]),
                smoothness: u32v(params, 2, defaults[2]),
            }
        }
        "mosaic" => {
            arity!(params, slots);
            Filter::Mosaic {
                cell_size: u32v(params, 0, defaults[0]),
            }
        }
        "crystallize" => {
            arity!(params, slots);
            Filter::Crystallize {
                cell_size: u32v(params, 0, defaults[0]),
                seed: u64v(params, 1, defaults[1]),
            }
        }
        "facet" => {
            arity!(params, slots);
            Filter::Facet
        }
        "fragment" => {
            arity!(params, slots);
            Filter::Fragment
        }
        "mezzotint" => {
            arity!(params, slots);
            Filter::Mezzotint {
                kind: pick(
                    &MEZZOTINT_TYPES,
                    params,
                    0,
                    defaults[0].round().max(0.0) as usize,
                ),
                seed: u64v(params, 1, defaults[1]),
            }
        }
        "pointillize" => {
            arity!(params, slots);
            Filter::Pointillize {
                cell_size: u32v(params, 0, defaults[0]),
                background: rgb(params, 1, [defaults[1], defaults[2], defaults[3]]),
                seed: u64v(params, 4, defaults[4]),
            }
        }
        "color-halftone" => {
            arity!(params, slots);
            Filter::ColorHalftone {
                max_radius: u32v(params, 0, defaults[0]),
                angles: [
                    f(params, 1, defaults[1]),
                    f(params, 2, defaults[2]),
                    f(params, 3, defaults[3]),
                    f(params, 4, defaults[4]),
                ],
            }
        }
        "twirl" => {
            arity!(params, slots);
            Filter::Twirl {
                angle: f(params, 0, defaults[0]),
            }
        }
        "pinch" => {
            arity!(params, slots);
            Filter::Pinch {
                amount: f(params, 0, defaults[0]),
            }
        }
        "spherize" => {
            arity!(params, slots);
            Filter::Spherize {
                amount: f(params, 0, defaults[0]),
                mode: pick(
                    &SPHERIZE_MODES,
                    params,
                    1,
                    defaults[1].round().max(0.0) as usize,
                ),
            }
        }
        "ripple" => {
            arity!(params, slots);
            Filter::Ripple {
                amount: f(params, 0, defaults[0]),
                size: pick(
                    &RIPPLE_SIZES,
                    params,
                    1,
                    defaults[1].round().max(0.0) as usize,
                ),
            }
        }
        "wave" => {
            arity!(params, slots);
            Filter::Wave {
                generators: u32v(params, 0, defaults[0]),
                wavelength: (f(params, 1, defaults[1]), f(params, 2, defaults[2])),
                amplitude: (f(params, 3, defaults[3]), f(params, 4, defaults[4])),
                kind: pick(
                    &WAVE_TYPES,
                    params,
                    5,
                    defaults[5].round().max(0.0) as usize,
                ),
                scale: (f(params, 6, defaults[6]), f(params, 7, defaults[7])),
                seed: u64v(params, 8, defaults[8]),
                repeat_edge: true,
            }
        }
        "polar-coordinates" => {
            arity!(params, slots);
            Filter::PolarCoordinates {
                kind: pick(
                    &POLAR_KINDS,
                    params,
                    0,
                    defaults[0].round().max(0.0) as usize,
                ),
            }
        }
        "shear" => {
            arity!(params, slots);
            // A zero (or absent) point count is the straight default.
            let count = i32v(params, 0, defaults[0]);
            let curve = if count == 0 {
                vec![(-1.0, 0.0), (1.0, 0.0)]
            } else if (2..=SHEAR_MAX_POINTS as i32).contains(&count) {
                (0..count as usize)
                    .map(|i| {
                        (
                            f(params, 1 + 2 * i, defaults[1 + 2 * i]),
                            f(params, 2 + 2 * i, defaults[2 + 2 * i]),
                        )
                    })
                    .collect()
            } else {
                return None;
            };
            Filter::Shear {
                curve,
                fill: pick(
                    &SHEAR_FILLS,
                    params,
                    SHEAR_SLOTS - 1,
                    defaults[SHEAR_SLOTS - 1].round().max(0.0) as usize,
                ),
            }
        }
        "displace" => {
            arity!(params, slots);
            Filter::Displace {
                map: displace_map(u32v(params, 4, defaults[4]))?,
                horizontal_scale: f(params, 0, defaults[0]),
                vertical_scale: f(params, 1, defaults[1]),
                fit: pick(
                    &DISPLACE_FITS,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                wrap: pick(
                    &SHEAR_FILLS,
                    params,
                    3,
                    defaults[3].round().max(0.0) as usize,
                ) == ShearFill::WrapAround,
            }
        }
        "liquify" => {
            arity!(params, slots);
            Filter::Liquify {
                mesh: liquify_mesh(u32v(params, 0, defaults[0]))?,
            }
        }
        "adaptive-wide-angle" => {
            arity!(params, slots);
            Filter::AdaptiveWideAngle {
                params: AwaParams {
                    correction: awa_correction(f(params, 0, defaults[0])),
                    scale: f(params, 1, defaults[1]).clamp(50.0, 150.0) / 100.0,
                    focal_length: f(params, 2, defaults[2]).clamp(1.0, 15.31),
                    crop_factor: f(params, 3, defaults[3]).clamp(0.2, 10.0),
                    offset: [f(params, 4, defaults[4]), f(params, 5, defaults[5])],
                    constraints: awa_constraints(u32v(params, 6, defaults[6]))?,
                },
            }
        }
        "zigzag" => {
            arity!(params, slots);
            Filter::ZigZag {
                amount: f(params, 0, defaults[0]),
                ridges: u32v(params, 1, defaults[1]),
                style: pick(
                    &ZIGZAG_STYLES,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
            }
        }
        "ocean-ripple" => {
            arity!(params, slots);
            Filter::OceanRipple {
                size: u32v(params, 0, defaults[0]),
                magnitude: u32v(params, 1, defaults[1]),
                seed: u64v(params, 2, defaults[2]),
            }
        }
        "glass" => {
            arity!(params, slots);
            Filter::Glass {
                distortion: u32v(params, 0, defaults[0]),
                smoothness: u32v(params, 1, defaults[1]),
                texture: pick(
                    &GLASS_TEXTURES,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                scaling: u32v(params, 3, defaults[3]),
                invert: flag(params, 4, defaults[4] >= 0.5),
            }
        }
        "diffuse-glow" => {
            arity!(params, slots);
            Filter::DiffuseGlow {
                graininess: u32v(params, 0, defaults[0]),
                glow_amount: u32v(params, 1, defaults[1]),
                clear_amount: u32v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "clouds" => {
            arity!(params, slots);
            Filter::Clouds {
                color_a: rgb(params, 0, [defaults[0], defaults[1], defaults[2]]),
                color_b: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                starker: flag(params, 6, defaults[6] >= 0.5),
                seed: u64v(params, 7, defaults[7]),
            }
        }
        "difference-clouds" => {
            arity!(params, slots);
            Filter::DifferenceClouds {
                color_a: rgb(params, 0, [defaults[0], defaults[1], defaults[2]]),
                color_b: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                starker: flag(params, 6, defaults[6] >= 0.5),
                seed: u64v(params, 7, defaults[7]),
            }
        }
        "fibers" => {
            arity!(params, slots);
            Filter::Fibers {
                variance: f(params, 0, defaults[0]),
                strength: f(params, 1, defaults[1]),
                color_a: rgb(params, 2, [defaults[2], defaults[3], defaults[4]]),
                color_b: rgb(params, 5, [defaults[5], defaults[6], defaults[7]]),
                seed: u64v(params, 8, defaults[8]),
            }
        }
        "lens-flare" => {
            arity!(params, slots);
            Filter::LensFlare {
                brightness: f(params, 0, defaults[0]),
                center: (f(params, 1, defaults[1]), f(params, 2, defaults[2])),
                lens: pick(
                    &LENS_TYPES,
                    params,
                    3,
                    defaults[3].round().max(0.0) as usize,
                ),
            }
        }
        "picture-frame" => {
            arity!(params, slots);
            Filter::PictureFrame {
                options: FrameOptions {
                    // The frame is a Choice slot: the 0-based picker index,
                    // where index 0 is Happy Vine (frame 1).
                    frame: u32v(params, 0, defaults[0]) + 1,
                    vine: rgb(params, 1, [defaults[1], defaults[2], defaults[3]]),
                    margin: u32v(params, 4, defaults[4]),
                    size: u32v(params, 5, defaults[5]),
                    arrangement: u32v(params, 6, defaults[6]),
                    flower: u32v(params, 7, defaults[7]),
                    flower_colour: rgb(params, 8, [defaults[8], defaults[9], defaults[10]]),
                    flower_size: u32v(params, 11, defaults[11]),
                    leaf: u32v(params, 12, defaults[12]),
                    leaf_colour: rgb(params, 13, [defaults[13], defaults[14], defaults[15]]),
                    leaf_size: u32v(params, 16, defaults[16]),
                    lines: u32v(params, 17, defaults[17]),
                    thickness: u32v(params, 18, defaults[18]),
                    angle: u32v(params, 19, defaults[19]),
                    fade: u32v(params, 20, defaults[20]),
                    invert: flag(params, 21, defaults[21] >= 0.5),
                },
            }
        }
        "lighting-effects" => Filter::Lighting {
            lighting: lighting(params, &defaults)?,
        },
        "colored-pencil" => {
            arity!(params, slots);
            Filter::ColoredPencil {
                pencil_width: u8v(params, 0, defaults[0]),
                stroke_pressure: u8v(params, 1, defaults[1]),
                paper_brightness: u8v(params, 2, defaults[2]),
                // ponytail: CS6's paper is the document background colour;
                // the dialog shows no swatch, so white stands in until the
                // bridge passes the document colours.
                background: [255, 255, 255],
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "cutout" => {
            arity!(params, slots);
            Filter::Cutout {
                levels: u8v(params, 0, defaults[0]),
                edge_simplicity: u8v(params, 1, defaults[1]),
                edge_fidelity: u8v(params, 2, defaults[2]),
            }
        }
        "dry-brush" => {
            arity!(params, slots);
            Filter::DryBrush {
                brush_size: u8v(params, 0, defaults[0]),
                brush_detail: u8v(params, 1, defaults[1]),
                texture: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "film-grain" => {
            arity!(params, slots);
            Filter::FilmGrain {
                grain: u8v(params, 0, defaults[0]),
                highlight_area: u8v(params, 1, defaults[1]),
                intensity: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "fresco" => {
            arity!(params, slots);
            Filter::Fresco {
                brush_size: u8v(params, 0, defaults[0]),
                brush_detail: u8v(params, 1, defaults[1]),
                texture: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "neon-glow" => {
            arity!(params, slots);
            Filter::NeonGlow {
                glow_size: i32v(params, 0, defaults[0]),
                glow_brightness: u8v(params, 1, defaults[1]),
                glow_color: rgb(params, 2, [defaults[2], defaults[3], defaults[4]]),
            }
        }
        "paint-daubs" => {
            arity!(params, slots);
            Filter::PaintDaubs {
                brush_size: u8v(params, 0, defaults[0]),
                sharpness: u8v(params, 1, defaults[1]),
                brush_type: pick(
                    &BRUSH_TYPES,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "palette-knife" => {
            arity!(params, slots);
            Filter::PaletteKnife {
                stroke_size: u8v(params, 0, defaults[0]),
                stroke_detail: u8v(params, 1, defaults[1]),
                softness: u8v(params, 2, defaults[2]),
            }
        }
        "plastic-wrap" => {
            arity!(params, slots);
            Filter::PlasticWrap {
                highlight_strength: u8v(params, 0, defaults[0]),
                detail: u8v(params, 1, defaults[1]),
                smoothness: u8v(params, 2, defaults[2]),
            }
        }
        "poster-edges" => {
            arity!(params, slots);
            Filter::PosterEdges {
                edge_thickness: u8v(params, 0, defaults[0]),
                edge_intensity: u8v(params, 1, defaults[1]),
                posterization: u8v(params, 2, defaults[2]),
            }
        }
        "rough-pastels" => {
            arity!(params, slots);
            Filter::RoughPastels {
                stroke_length: u8v(params, 0, defaults[0]),
                stroke_detail: u8v(params, 1, defaults[1]),
                texture: TextureOptions {
                    relief: u8v(params, 4, defaults[4]),
                    ..texture(params, 2, &defaults)
                },
                seed: 1,
            }
        }
        "smudge-stick" => {
            arity!(params, slots);
            Filter::SmudgeStick {
                stroke_length: u8v(params, 0, defaults[0]),
                highlight_area: u8v(params, 1, defaults[1]),
                intensity: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "sponge" => {
            arity!(params, slots);
            Filter::Sponge {
                brush_size: u8v(params, 0, defaults[0]),
                definition: u8v(params, 1, defaults[1]),
                smoothness: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "underpainting" => {
            arity!(params, slots);
            Filter::Underpainting {
                brush_size: u8v(params, 0, defaults[0]),
                texture_coverage: u8v(params, 1, defaults[1]),
                texture: TextureOptions {
                    light_direction: u8v(params, 5, defaults[5]),
                    ..texture(params, 2, &defaults)
                },
                seed: u64v(params, 7, defaults[7]),
            }
        }
        "watercolor" => {
            arity!(params, slots);
            Filter::Watercolor {
                brush_detail: u8v(params, 0, defaults[0]),
                shadow_intensity: u8v(params, 1, defaults[1]),
                texture: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "accented-edges" => {
            arity!(params, slots);
            Filter::AccentedEdges {
                edge_width: u8v(params, 0, defaults[0]),
                edge_brightness: u8v(params, 1, defaults[1]),
                smoothness: u8v(params, 2, defaults[2]),
            }
        }
        "angled-strokes" => {
            arity!(params, slots);
            Filter::AngledStrokes {
                direction_balance: u8v(params, 0, defaults[0]),
                stroke_length: u8v(params, 1, defaults[1]),
                sharpness: u8v(params, 2, defaults[2]),
            }
        }
        "crosshatch" => {
            arity!(params, slots);
            Filter::Crosshatch {
                stroke_length: u8v(params, 0, defaults[0]),
                sharpness: u8v(params, 1, defaults[1]),
                strength: u8v(params, 2, defaults[2]),
            }
        }
        "dark-strokes" => {
            arity!(params, slots);
            Filter::DarkStrokes {
                balance: u8v(params, 0, defaults[0]),
                black_intensity: u8v(params, 1, defaults[1]),
                white_intensity: u8v(params, 2, defaults[2]),
            }
        }
        "ink-outlines" => {
            arity!(params, slots);
            Filter::InkOutlines {
                stroke_length: u8v(params, 0, defaults[0]),
                dark_intensity: u8v(params, 1, defaults[1]),
                light_intensity: u8v(params, 2, defaults[2]),
            }
        }
        "spatter" => {
            arity!(params, slots);
            Filter::Spatter {
                spray_radius: u8v(params, 0, defaults[0]),
                smoothness: u8v(params, 1, defaults[1]),
                seed: u64v(params, 2, defaults[2]),
            }
        }
        "sprayed-strokes" => {
            arity!(params, slots);
            Filter::SprayedStrokes {
                stroke_length: u8v(params, 0, defaults[0]),
                spray_radius: u8v(params, 1, defaults[1]),
                direction: pick(
                    &STROKE_DIRECTIONS,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "sumi-e" => {
            arity!(params, slots);
            Filter::SumiE {
                stroke_width: u8v(params, 0, defaults[0]),
                stroke_pressure: u8v(params, 1, defaults[1]),
                contrast: u8v(params, 2, defaults[2]),
            }
        }
        "bas-relief" => {
            arity!(params, slots);
            Filter::BasRelief {
                detail: u8v(params, 0, defaults[0]),
                smoothness: u8v(params, 1, defaults[1]),
                light_direction: pick(
                    &LIGHT_DIRECTIONS,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                background: rgb(params, 6, [defaults[6], defaults[7], defaults[8]]),
            }
        }
        "chalk-charcoal" => {
            arity!(params, slots);
            Filter::ChalkCharcoal {
                charcoal_area: u8v(params, 0, defaults[0]),
                chalk_area: u8v(params, 1, defaults[1]),
                stroke_pressure: u8v(params, 2, defaults[2]),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                background: rgb(params, 6, [defaults[6], defaults[7], defaults[8]]),
                seed: u64v(params, 9, defaults[9]),
            }
        }
        "charcoal" => {
            arity!(params, slots);
            Filter::Charcoal {
                thickness: u8v(params, 0, defaults[0]),
                detail: u8v(params, 1, defaults[1]),
                light_dark_balance: u8v(params, 2, defaults[2]),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                background: rgb(params, 6, [defaults[6], defaults[7], defaults[8]]),
                seed: u64v(params, 9, defaults[9]),
            }
        }
        "chrome" => {
            arity!(params, slots);
            Filter::Chrome {
                detail: u8v(params, 0, defaults[0]),
                smoothness: u8v(params, 1, defaults[1]),
            }
        }
        "conte-crayon" => {
            arity!(params, slots);
            Filter::ConteCrayon {
                foreground_level: u8v(params, 0, defaults[0]),
                background_level: u8v(params, 1, defaults[1]),
                texture: texture(params, 2, &defaults),
                foreground: rgb(params, 7, [defaults[7], defaults[8], defaults[9]]),
                background: rgb(params, 10, [defaults[10], defaults[11], defaults[12]]),
                seed: u64v(params, 13, defaults[13]),
            }
        }
        "graphic-pen" => {
            arity!(params, slots);
            Filter::GraphicPen {
                stroke_length: u8v(params, 0, defaults[0]),
                light_dark_balance: u8v(params, 1, defaults[1]),
                direction: pick(
                    &STROKE_DIRECTIONS,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                background: rgb(params, 6, [defaults[6], defaults[7], defaults[8]]),
            }
        }
        "halftone-pattern" => {
            arity!(params, slots);
            Filter::HalftonePattern {
                size: u8v(params, 0, defaults[0]),
                contrast: u8v(params, 1, defaults[1]),
                pattern: pick(
                    &HALFTONE_TYPES,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
            }
        }
        "note-paper" => {
            arity!(params, slots);
            Filter::NotePaper {
                image_balance: u8v(params, 0, defaults[0]),
                graininess: u8v(params, 1, defaults[1]),
                relief: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "photocopy" => {
            arity!(params, slots);
            Filter::Photocopy {
                detail: u8v(params, 0, defaults[0]),
                darkness: u8v(params, 1, defaults[1]),
            }
        }
        "plaster" => {
            arity!(params, slots);
            Filter::Plaster {
                image_balance: u8v(params, 0, defaults[0]),
                smoothness: u8v(params, 1, defaults[1]),
                light_direction: pick(
                    &LIGHT_DIRECTIONS,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                background: rgb(params, 6, [defaults[6], defaults[7], defaults[8]]),
            }
        }
        "reticulation" => {
            arity!(params, slots);
            Filter::Reticulation {
                density: u8v(params, 0, defaults[0]),
                black_level: u8v(params, 1, defaults[1]),
                white_level: u8v(params, 2, defaults[2]),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                background: rgb(params, 6, [defaults[6], defaults[7], defaults[8]]),
                seed: u64v(params, 9, defaults[9]),
            }
        }
        "stamp" => {
            arity!(params, slots);
            Filter::Stamp {
                light_dark_balance: u8v(params, 0, defaults[0]),
                smoothness: u8v(params, 1, defaults[1]),
                foreground: rgb(params, 2, [defaults[2], defaults[3], defaults[4]]),
                background: rgb(params, 5, [defaults[5], defaults[6], defaults[7]]),
            }
        }
        "torn-edges" => {
            arity!(params, slots);
            Filter::TornEdges {
                image_balance: u8v(params, 0, defaults[0]),
                smoothness: u8v(params, 1, defaults[1]),
                contrast: u8v(params, 2, defaults[2]),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                background: rgb(params, 6, [defaults[6], defaults[7], defaults[8]]),
            }
        }
        "water-paper" => {
            arity!(params, slots);
            Filter::WaterPaper {
                fiber_length: u8v(params, 0, defaults[0]),
                brightness: u8v(params, 1, defaults[1]),
                contrast: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "craquelure" => {
            arity!(params, slots);
            Filter::Craquelure {
                crack_spacing: u8v(params, 0, defaults[0]),
                crack_depth: u8v(params, 1, defaults[1]),
                crack_brightness: u8v(params, 2, defaults[2]),
            }
        }
        "grain" => {
            arity!(params, slots);
            Filter::Grain {
                intensity: u8v(params, 0, defaults[0]),
                contrast: u8v(params, 1, defaults[1]),
                grain_type: pick(
                    &GRAIN_TYPES,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                background: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                seed: u64v(params, 6, defaults[6]),
            }
        }
        "mosaic-tiles" => {
            arity!(params, slots);
            Filter::MosaicTiles {
                tile_size: u8v(params, 0, defaults[0]),
                grout_width: u8v(params, 1, defaults[1]),
                lighten_grout: u8v(params, 2, defaults[2]),
                seed: u64v(params, 3, defaults[3]),
            }
        }
        "patchwork" => {
            arity!(params, slots);
            Filter::Patchwork {
                square_size: u8v(params, 0, defaults[0]),
                relief: u8v(params, 1, defaults[1]),
                seed: u64v(params, 2, defaults[2]),
            }
        }
        "stained-glass" => {
            arity!(params, slots);
            Filter::StainedGlass {
                cell_size: u8v(params, 0, defaults[0]),
                border_thickness: u8v(params, 1, defaults[1]),
                light_intensity: u8v(params, 2, defaults[2]),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                seed: u64v(params, 6, defaults[6]),
            }
        }
        "texturizer" => {
            arity!(params, slots);
            Filter::Texturizer {
                texture: texture(params, 0, &defaults),
            }
        }
        "oil-paint" => {
            arity!(params, slots);
            Filter::OilPaint {
                stylization: f(params, 0, defaults[0]),
                cleanliness: f(params, 1, defaults[1]),
                scale: f(params, 2, defaults[2]),
                bristle_detail: f(params, 3, defaults[3]),
                angular_direction: f(params, 4, defaults[4]),
                shine: f(params, 5, defaults[5]),
            }
        }
        "dust-and-scratches" => {
            arity!(params, slots);
            Filter::DustAndScratches {
                radius: u32v(params, 0, defaults[0]),
                threshold: u32v(params, 1, defaults[1]),
            }
        }
        "extrude" => {
            arity!(params, slots);
            Filter::Extrude {
                kind: pick(
                    &EXTRUDE_TYPES,
                    params,
                    0,
                    defaults[0].round().max(0.0) as usize,
                ),
                size: u32v(params, 1, defaults[1]),
                depth: f(params, 2, defaults[2]) as f32,
                level_based: flag(params, 3, defaults[3] >= 0.5),
                solid_front: flag(params, 4, defaults[4] >= 0.5),
                mask_incomplete: flag(params, 5, defaults[5] >= 0.5),
            }
        }
        "tiles" => {
            arity!(params, slots);
            Filter::Tiles {
                count: u32v(params, 0, defaults[0]),
                offset: u32v(params, 1, defaults[1]),
                fill: pick(
                    &TILE_FILLS,
                    params,
                    2,
                    defaults[2].round().max(0.0) as usize,
                ),
                foreground: rgb(params, 3, [defaults[3], defaults[4], defaults[5]]),
                background: rgb(params, 6, [defaults[6], defaults[7], defaults[8]]),
            }
        }
        "trace-contour" => {
            arity!(params, slots);
            Filter::TraceContour {
                level: u8v(params, 0, defaults[0]),
                edge: pick(
                    &CONTOUR_EDGES,
                    params,
                    1,
                    defaults[1].round().max(0.0) as usize,
                ),
            }
        }
        "wind" => {
            arity!(params, slots);
            Filter::Wind {
                method: pick(
                    &WIND_METHODS,
                    params,
                    0,
                    defaults[0].round().max(0.0) as usize,
                ),
                from_right: pick(
                    &WIND_FROM_RIGHT,
                    params,
                    1,
                    defaults[1].round().max(0.0) as usize,
                ),
            }
        }
        "smart-sharpen" => {
            arity!(params, slots);
            Filter::SmartSharpen {
                amount: f(params, 0, defaults[0]),
                radius: f(params, 1, defaults[1]),
                reduce_noise: f(params, 2, defaults[2]),
                remove: pick(
                    &SHARPEN_REMOVES,
                    params,
                    3,
                    defaults[3].round().max(0.0) as usize,
                ),
                angle: f(params, 4, defaults[4]),
                more_accurate: flag(params, 5, defaults[5] >= 0.5),
                shadow: tonal_fade(params, 6, &defaults),
                highlight: tonal_fade(params, 9, &defaults),
            }
        }
        _ => return None,
    })
}
