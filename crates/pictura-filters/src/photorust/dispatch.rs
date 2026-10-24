//! Route the ported [`Filter`] variants to the photorust engine: check the
//! parameters against the ranges the filter specs fix, run the filter on an
//! interleaved copy under the variant's seed, and write the colour planes
//! back — plus the alpha plane for a moving kernel (see
//! [`Filter::moves_alpha`]) on a four-channel buffer.

use std::ops::RangeInclusive;

use pictura_core::PixelBuffer;

use super::pixmap::{Pixmap, Rgba8};
use super::{
    add_noise, artistic, brush_strokes, convolve, displace, distort, pixelate, sharpen, sketch,
    stylize, texture, with_seed,
};
use crate::{
    Filter, FilterError, LightDirection, NoiseDistribution, PolarKind, ShearFill, TextureOptions,
};

type Run = Box<dyn FnOnce(&mut Pixmap)>;

/// Apply `filter` when it is one the photorust engine runs; `None` leaves it
/// to Kooka's own implementation.
pub(crate) fn apply(filter: &Filter, buf: &mut PixelBuffer) -> Option<Result<(), FilterError>> {
    let (seed, run) = match plan(filter)? {
        Ok(plan) => plan,
        Err(e) => return Some(Err(e)),
    };
    let mut pixmap = Pixmap::from_planar(buf);
    with_seed(seed, || run(&mut pixmap));
    pixmap.write_colour(buf);
    if buf.channels == 4 && filter.moves_alpha() {
        pixmap.write_alpha(buf);
    }
    Some(Ok(()))
}

fn bad(what: &str) -> FilterError {
    FilterError::InvalidParams(format!("{what} is out of range"))
}

fn check<T: PartialOrd + Copy>(
    what: &str,
    v: T,
    range: RangeInclusive<T>,
) -> Result<(), FilterError> {
    if range.contains(&v) {
        Ok(())
    } else {
        Err(bad(what))
    }
}

fn check_f(what: &str, v: f64, range: RangeInclusive<f64>) -> Result<(), FilterError> {
    if v.is_finite() && range.contains(&v) {
        Ok(())
    } else {
        Err(bad(what))
    }
}

/// Texture options: `scaling 50..=200`, `relief 0..=50`, light `0..=7`.
fn check_texture(t: &TextureOptions) -> Result<(), FilterError> {
    check("texture scaling", t.scaling, 50..=200)?;
    check("texture relief", t.relief, 0..=50)?;
    check("texture light direction", t.light_direction, 0..=7)
}

/// The dialog's light index (Bottom, Bottom Left, … Bottom Right).
fn light(index: u8) -> LightDirection {
    use LightDirection::*;
    [
        Bottom,
        BottomLeft,
        Left,
        TopLeft,
        Top,
        TopRight,
        Right,
        BottomRight,
    ][index.min(7) as usize]
}

fn u(v: u8) -> u32 {
    u32::from(v)
}

fn rgb(c: [u8; 3]) -> Rgba8 {
    Rgba8::rgb(c)
}

/// The seed and the run for a ported variant; `None` for a variant the native
/// engine owns, `Some(Err)` for parameters out of range.
#[allow(clippy::too_many_lines)]
fn plan(filter: &Filter) -> Option<Result<(u64, Run), FilterError>> {
    // Each arm validates, then boxes the photorust call with its arguments
    // converted to photorust's types.
    let plan: Result<(u64, Run), FilterError> = match *filter {
        // --- Noise / Other ------------------------------------------------
        Filter::AddNoise {
            amount,
            distribution,
            monochromatic,
            seed,
        } => check_f("add noise amount", amount, 0.0..=400.0).map(|()| {
            let gaussian = distribution == NoiseDistribution::Gaussian;
            let run: Run = Box::new(move |p| add_noise(p, amount as f32, monochromatic, gaussian));
            (seed, run)
        }),
        Filter::DustAndScratches { radius, threshold } => check("dust radius", radius, 1..=16)
            .and(check("dust threshold", threshold, 0..=255))
            .map(|()| unseeded(move |p| convolve::dust_and_scratches(p, radius, threshold))),
        Filter::Average => Ok(unseeded(convolve::average)),
        // --- Render -------------------------------------------------------
        Filter::PictureFrame { options } => options
            .validate()
            .map(|()| unseeded(move |p| crate::render::picture_frame(p, &options))),
        // --- Blur / Sharpen -----------------------------------------------
        Filter::SurfaceBlur { radius, threshold } => check("surface radius", radius, 1..=100)
            .and(check("surface threshold", threshold, 1..=255))
            .map(|()| unseeded(move |p| convolve::surface_blur(p, radius, u32::from(threshold)))),
        Filter::SharpenEdges => Ok(unseeded(sharpen::sharpen_edges)),
        // --- Stylize ------------------------------------------------------
        Filter::Emboss {
            angle,
            height,
            amount,
        } => check_f("emboss angle", angle, -360.0..=360.0)
            .and(check_f("emboss height", height, 1.0..=100.0))
            .and(check_f("emboss amount", amount, 1.0..=500.0))
            .map(|()| {
                unseeded(move |p| stylize::emboss(p, angle as f32, height as f32, amount as f32))
            }),
        Filter::FindEdges => Ok(unseeded(stylize::find_edges)),
        Filter::Diffuse { mode } => Ok(unseeded(move |p| stylize::diffuse(p, mode))),
        Filter::GlowingEdges {
            width,
            brightness,
            smoothness,
        } => check("glowing edges width", width, 1..=14)
            .and(check("glowing edges brightness", brightness, 0..=20))
            .and(check("glowing edges smoothness", smoothness, 1..=15))
            .map(|()| unseeded(move |p| stylize::glowing_edges(p, width, brightness, smoothness))),
        Filter::Extrude {
            kind,
            size,
            depth,
            level_based,
            solid_front,
            mask_incomplete,
        } => check("extrude size", size, 2..=255)
            .and(check_f("extrude depth", f64::from(depth), 1.0..=255.0))
            .map(|()| {
                let opt = stylize::ExtrudeOptions {
                    kind,
                    size,
                    depth,
                    level_based,
                    solid_front,
                    mask_incomplete,
                };
                unseeded(move |p| stylize::extrude(p, opt))
            }),
        Filter::Tiles {
            count,
            offset,
            fill,
            foreground,
            background,
        } => check("tiles count", count, 1..=99)
            .and(check("tiles offset", offset, 1..=99))
            .map(|()| {
                let opt = stylize::TileOptions {
                    count,
                    offset,
                    fill,
                    foreground: rgb(foreground),
                    background: rgb(background),
                };
                unseeded(move |p| stylize::tiles(p, opt))
            }),
        Filter::TraceContour { level, edge } => {
            Ok(unseeded(move |p| stylize::trace_contour(p, level, edge)))
        }
        Filter::Wind { method, from_right } => {
            Ok(unseeded(move |p| stylize::wind(p, method, from_right)))
        }
        // --- Pixelate -----------------------------------------------------
        Filter::Mosaic { cell_size } => check("mosaic cell size", cell_size, 2..=200)
            .map(|()| unseeded(move |p| pixelate::mosaic(p, cell_size))),
        Filter::Crystallize { cell_size, seed } => {
            check("crystallize cell size", cell_size, 3..=300)
                .map(|()| seeded(seed, move |p| pixelate::crystallize(p, cell_size)))
        }
        Filter::Pointillize {
            cell_size,
            background,
            seed,
        } => check("pointillize cell size", cell_size, 3..=300).map(|()| {
            seeded(seed, move |p| {
                pixelate::pointillize(p, cell_size, rgb(background))
            })
        }),
        Filter::Facet => Ok(unseeded(pixelate::facet)),
        Filter::Fragment => Ok(unseeded(pixelate::fragment)),
        Filter::Mezzotint { kind, seed } => Ok(seeded(seed, move |p| pixelate::mezzotint(p, kind))),
        Filter::ColorHalftone { max_radius, angles } => {
            check("color halftone radius", max_radius, 4..=127)
                .and(
                    angles
                        .iter()
                        .try_for_each(|a| check_f("color halftone angle", *a, -360.0..=360.0)),
                )
                .map(|()| {
                    let angles = angles.map(|a| a as f32);
                    unseeded(move |p| pixelate::color_halftone(p, max_radius as f32, angles))
                })
        }
        // --- Distort ------------------------------------------------------
        Filter::Twirl { angle } => check_f("twirl angle", angle, -999.0..=999.0)
            .map(|()| unseeded(move |p| distort::twirl(p, angle as f32))),
        Filter::Pinch { amount } => check_f("pinch amount", amount, -100.0..=100.0)
            .map(|()| unseeded(move |p| distort::pinch(p, amount as f32))),
        Filter::Spherize { amount, mode } => check_f("spherize amount", amount, -100.0..=100.0)
            .map(|()| unseeded(move |p| distort::spherize(p, amount as f32, mode))),
        Filter::Ripple { amount, size } => check_f("ripple amount", amount, -999.0..=999.0)
            .map(|()| unseeded(move |p| distort::ripple(p, amount as f32, size))),
        Filter::PolarCoordinates { kind } => {
            let to_polar = kind == PolarKind::RectangularToPolar;
            Ok(unseeded(move |p| distort::polar_coordinates(p, to_polar)))
        }
        Filter::Shear { ref curve, fill } => check_shear_curve(curve).map(|()| {
            let curve = curve.clone();
            unseeded(move |p| distort::shear(p, &curve, fill == ShearFill::WrapAround))
        }),
        Filter::Displace {
            ref map,
            horizontal_scale,
            vertical_scale,
            fit,
            wrap,
        } => check_f(
            "displace horizontal scale",
            horizontal_scale,
            -999.0..=999.0,
        )
        .and(check_f(
            "displace vertical scale",
            vertical_scale,
            -999.0..=999.0,
        ))
        .map(|()| {
            let map = map.clone();
            unseeded(move |p| {
                displace::displace(
                    p,
                    &map,
                    horizontal_scale as f32,
                    vertical_scale as f32,
                    fit,
                    wrap,
                )
            })
        }),
        Filter::ZigZag {
            amount,
            ridges,
            style,
        } => check_f("zigzag amount", amount, -100.0..=100.0)
            .and(check("zigzag ridges", ridges, 0..=20))
            .map(|()| unseeded(move |p| distort::zigzag(p, amount as f32, ridges, style))),
        Filter::Wave {
            generators,
            wavelength,
            amplitude,
            kind,
            scale,
            seed,
            repeat_edge,
        } => check_wave(generators, wavelength, amplitude, scale).map(|()| {
            let pair = |(a, b): (f64, f64)| (a as f32, b as f32);
            let (wl, amp, sc) = (pair(wavelength), pair(amplitude), pair(scale));
            // Wave rolls its own generators from an explicit seed.
            let wave_seed = (seed ^ (seed >> 32)) as u32;
            unseeded(move |p| {
                distort::wave(p, generators, wl, amp, sc, kind, !repeat_edge, wave_seed)
            })
        }),

        Filter::ColoredPencil {
            pencil_width,
            stroke_pressure,
            paper_brightness,
            background,
            seed,
        } => check("colored pencil width", pencil_width, 1..=24)
            .and(check("colored pencil pressure", stroke_pressure, 0..=15))
            .and(check("colored pencil paper", paper_brightness, 0..=50))
            .map(|()| {
                seeded(seed, move |p| {
                    artistic::colored_pencil(
                        p,
                        u(pencil_width),
                        u(stroke_pressure),
                        u(paper_brightness),
                        rgb(background),
                    )
                })
            }),
        Filter::Cutout {
            levels,
            edge_simplicity,
            edge_fidelity,
        } => check("cutout levels", levels, 2..=8)
            .and(check("cutout simplicity", edge_simplicity, 0..=10))
            .and(check("cutout fidelity", edge_fidelity, 1..=3))
            .map(|()| {
                unseeded(move |p| {
                    artistic::cutout(p, u(levels), u(edge_simplicity), u(edge_fidelity))
                })
            }),
        Filter::DryBrush {
            brush_size,
            brush_detail,
            texture,
            seed,
        } => check_brush(brush_size, brush_detail, texture).map(|()| {
            seeded(seed, move |p| {
                artistic::dry_brush(p, u(brush_size), u(brush_detail), u(texture))
            })
        }),
        Filter::Fresco {
            brush_size,
            brush_detail,
            texture,
            seed,
        } => check_brush(brush_size, brush_detail, texture).map(|()| {
            seeded(seed, move |p| {
                artistic::fresco(p, u(brush_size), u(brush_detail), u(texture))
            })
        }),
        Filter::FilmGrain {
            grain,
            highlight_area,
            intensity,
            seed,
        } => check("film grain", grain, 0..=20)
            .and(check("film grain highlight", highlight_area, 0..=20))
            .and(check("film grain intensity", intensity, 0..=10))
            .map(|()| {
                seeded(seed, move |p| {
                    artistic::film_grain(p, u(grain), u(highlight_area), u(intensity))
                })
            }),
        Filter::NeonGlow {
            glow_size,
            glow_brightness,
            glow_color,
        } => check("neon glow size", glow_size, -24..=24)
            .and(check("neon glow brightness", glow_brightness, 0..=50))
            .map(|()| {
                // ponytail: CS6 tints with the document foreground and
                // background; the variant carries no colours yet, so the
                // defaults stand in.
                unseeded(move |p| {
                    artistic::neon_glow(
                        p,
                        glow_size,
                        u(glow_brightness),
                        rgb(glow_color),
                        Rgba8::BLACK,
                        Rgba8::WHITE,
                    )
                })
            }),
        Filter::PaintDaubs {
            brush_size,
            sharpness,
            brush_type,
            seed,
        } => check("paint daubs size", brush_size, 1..=50)
            .and(check("paint daubs sharpness", sharpness, 0..=40))
            .map(|()| {
                seeded(seed, move |p| {
                    artistic::paint_daubs(p, u(brush_size), u(sharpness), brush_type)
                })
            }),
        Filter::PaletteKnife {
            stroke_size,
            stroke_detail,
            softness,
        } => check("palette knife size", stroke_size, 1..=50)
            .and(check("palette knife detail", stroke_detail, 1..=3))
            .and(check("palette knife softness", softness, 0..=10))
            .map(|()| {
                unseeded(move |p| {
                    artistic::palette_knife(p, u(stroke_size), u(stroke_detail), u(softness))
                })
            }),
        Filter::PlasticWrap {
            highlight_strength,
            detail,
            smoothness,
        } => check("plastic wrap highlight", highlight_strength, 0..=20)
            .and(check("plastic wrap detail", detail, 1..=15))
            .and(check("plastic wrap smoothness", smoothness, 1..=15))
            .map(|()| {
                unseeded(move |p| {
                    artistic::plastic_wrap(p, u(highlight_strength), u(detail), u(smoothness))
                })
            }),
        Filter::PosterEdges {
            edge_thickness,
            edge_intensity,
            posterization,
        } => check("poster edges thickness", edge_thickness, 0..=10)
            .and(check("poster edges intensity", edge_intensity, 0..=10))
            .and(check("poster edges posterization", posterization, 0..=10))
            .map(|()| {
                unseeded(move |p| {
                    artistic::poster_edges(
                        p,
                        u(edge_thickness),
                        u(edge_intensity),
                        u(posterization),
                    )
                })
            }),
        Filter::RoughPastels {
            stroke_length,
            stroke_detail,
            texture,
            seed,
        } => check("rough pastels length", stroke_length, 0..=40)
            .and(check("rough pastels detail", stroke_detail, 1..=20))
            .and(check_texture(&texture))
            .map(|()| {
                seeded(seed, move |p| {
                    artistic::rough_pastels(
                        p,
                        u(stroke_length),
                        u(stroke_detail),
                        texture.surface,
                        u(texture.scaling),
                        u(texture.relief),
                        light(texture.light_direction),
                        texture.invert,
                    )
                })
            }),
        Filter::SmudgeStick {
            stroke_length,
            highlight_area,
            intensity,
            seed,
        } => check("smudge stick length", stroke_length, 0..=10)
            .and(check("smudge stick highlight", highlight_area, 0..=20))
            .and(check("smudge stick intensity", intensity, 0..=10))
            .map(|()| {
                seeded(seed, move |p| {
                    artistic::smudge_stick(p, u(stroke_length), u(highlight_area), u(intensity))
                })
            }),
        Filter::Sponge {
            brush_size,
            definition,
            smoothness,
            seed,
        } => check("sponge size", brush_size, 0..=10)
            .and(check("sponge definition", definition, 0..=25))
            .and(check("sponge smoothness", smoothness, 1..=15))
            .map(|()| {
                seeded(seed, move |p| {
                    artistic::sponge(p, u(brush_size), u(definition), u(smoothness))
                })
            }),
        Filter::Underpainting {
            brush_size,
            texture_coverage,
            texture,
            seed,
        } => check("underpainting size", brush_size, 0..=40)
            .and(check("underpainting coverage", texture_coverage, 0..=40))
            .and(check_texture(&texture))
            .map(|()| {
                seeded(seed, move |p| {
                    artistic::underpainting(
                        p,
                        u(brush_size),
                        u(texture_coverage),
                        texture.surface,
                        u(texture.scaling),
                        u(texture.relief),
                        light(texture.light_direction),
                        texture.invert,
                    )
                })
            }),
        Filter::Watercolor {
            brush_detail,
            shadow_intensity,
            texture,
            seed,
        } => check("watercolor detail", brush_detail, 1..=14)
            .and(check("watercolor shadow", shadow_intensity, 0..=10))
            .and(check("watercolor texture", texture, 1..=3))
            .map(|()| {
                seeded(seed, move |p| {
                    artistic::watercolor(p, u(brush_detail), u(shadow_intensity), u(texture))
                })
            }),
        Filter::AccentedEdges {
            edge_width,
            edge_brightness,
            smoothness,
        } => check("accented edges width", edge_width, 1..=14)
            .and(check("accented edges brightness", edge_brightness, 0..=50))
            .and(check("accented edges smoothness", smoothness, 1..=15))
            .map(|()| {
                unseeded(move |p| {
                    brush_strokes::accented_edges(
                        p,
                        u(edge_width),
                        u(edge_brightness),
                        u(smoothness),
                    )
                })
            }),
        Filter::AngledStrokes {
            direction_balance,
            stroke_length,
            sharpness,
        } => check("angled strokes balance", direction_balance, 0..=100)
            .and(check("angled strokes length", stroke_length, 3..=50))
            .and(check("angled strokes sharpness", sharpness, 0..=10))
            .map(|()| {
                unseeded(move |p| {
                    brush_strokes::angled_strokes(
                        p,
                        u(direction_balance),
                        u(stroke_length),
                        u(sharpness),
                    )
                })
            }),
        Filter::Crosshatch {
            stroke_length,
            sharpness,
            strength,
        } => check("crosshatch length", stroke_length, 3..=50)
            .and(check("crosshatch sharpness", sharpness, 0..=20))
            .and(check("crosshatch strength", strength, 1..=3))
            .map(|()| {
                unseeded(move |p| {
                    brush_strokes::crosshatch(p, u(stroke_length), u(sharpness), u(strength))
                })
            }),
        Filter::DarkStrokes {
            balance,
            black_intensity,
            white_intensity,
        } => check("dark strokes balance", balance, 0..=10)
            .and(check("dark strokes black", black_intensity, 0..=10))
            .and(check("dark strokes white", white_intensity, 0..=10))
            .map(|()| {
                unseeded(move |p| {
                    brush_strokes::dark_strokes(
                        p,
                        u(balance),
                        u(black_intensity),
                        u(white_intensity),
                    )
                })
            }),
        Filter::InkOutlines {
            stroke_length,
            dark_intensity,
            light_intensity,
        } => check("ink outlines length", stroke_length, 1..=50)
            .and(check("ink outlines dark", dark_intensity, 0..=50))
            .and(check("ink outlines light", light_intensity, 0..=50))
            .map(|()| {
                unseeded(move |p| {
                    brush_strokes::ink_outlines(
                        p,
                        u(stroke_length),
                        u(dark_intensity),
                        u(light_intensity),
                    )
                })
            }),
        Filter::Spatter {
            spray_radius,
            smoothness,
            seed,
        } => check("spatter radius", spray_radius, 0..=25)
            .and(check("spatter smoothness", smoothness, 1..=15))
            .map(|()| {
                seeded(seed, move |p| {
                    brush_strokes::spatter(p, u(spray_radius), u(smoothness))
                })
            }),
        Filter::SprayedStrokes {
            stroke_length,
            spray_radius,
            direction,
            seed,
        } => check("sprayed strokes length", stroke_length, 0..=20)
            .and(check("sprayed strokes radius", spray_radius, 0..=25))
            .map(|()| {
                seeded(seed, move |p| {
                    brush_strokes::sprayed_strokes(p, u(stroke_length), u(spray_radius), direction)
                })
            }),
        Filter::SumiE {
            stroke_width,
            stroke_pressure,
            contrast,
        } => check("sumi-e width", stroke_width, 3..=15)
            .and(check("sumi-e pressure", stroke_pressure, 0..=15))
            .and(check("sumi-e contrast", contrast, 0..=40))
            .map(|()| {
                unseeded(move |p| {
                    brush_strokes::sumi_e(p, u(stroke_width), u(stroke_pressure), u(contrast))
                })
            }),

        Filter::BasRelief {
            detail,
            smoothness,
            light_direction,
            foreground,
            background,
        } => check("bas relief detail", detail, 1..=15)
            .and(check("bas relief smoothness", smoothness, 1..=15))
            .map(|()| {
                unseeded(move |p| {
                    sketch::bas_relief(
                        p,
                        u(detail),
                        u(smoothness),
                        light_direction,
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::ChalkCharcoal {
            charcoal_area,
            chalk_area,
            stroke_pressure,
            foreground,
            background,
            seed,
        } => check("chalk charcoal area", charcoal_area, 0..=20)
            .and(check("chalk area", chalk_area, 0..=20))
            .and(check("chalk pressure", stroke_pressure, 0..=5))
            .map(|()| {
                seeded(seed, move |p| {
                    sketch::chalk_and_charcoal(
                        p,
                        u(charcoal_area),
                        u(chalk_area),
                        u(stroke_pressure),
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::Charcoal {
            thickness,
            detail,
            light_dark_balance,
            foreground,
            background,
            seed,
        } => check("charcoal thickness", thickness, 1..=7)
            .and(check("charcoal detail", detail, 0..=5))
            .and(check("charcoal balance", light_dark_balance, 0..=100))
            .map(|()| {
                seeded(seed, move |p| {
                    sketch::charcoal(
                        p,
                        u(thickness),
                        u(detail),
                        u(light_dark_balance),
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::Chrome { detail, smoothness } => check("chrome detail", detail, 0..=10)
            .and(check("chrome smoothness", smoothness, 0..=10))
            .map(|()| unseeded(move |p| sketch::chrome(p, u(detail), u(smoothness)))),
        Filter::ConteCrayon {
            foreground_level,
            background_level,
            texture,
            foreground,
            background,
            seed,
        } => check("conte foreground level", foreground_level, 1..=15)
            .and(check("conte background level", background_level, 1..=15))
            .and(check_texture(&texture))
            .map(|()| {
                seeded(seed, move |p| {
                    sketch::conte_crayon(
                        p,
                        u(foreground_level),
                        u(background_level),
                        texture.surface,
                        u(texture.scaling),
                        u(texture.relief),
                        light(texture.light_direction),
                        texture.invert,
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::GraphicPen {
            stroke_length,
            light_dark_balance,
            direction,
            foreground,
            background,
        } => check("graphic pen length", stroke_length, 1..=15)
            .and(check("graphic pen balance", light_dark_balance, 0..=100))
            .map(|()| {
                unseeded(move |p| {
                    sketch::graphic_pen(
                        p,
                        u(stroke_length),
                        u(light_dark_balance),
                        direction,
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::HalftonePattern {
            size,
            contrast,
            pattern,
        } => check("halftone size", size, 1..=12)
            .and(check("halftone contrast", contrast, 0..=50))
            .map(|()| {
                // ponytail: CS6 draws with the document foreground and
                // background; the variant carries no colours yet, so CS6's
                // default black and white stand in.
                unseeded(move |p| {
                    sketch::halftone_pattern(
                        p,
                        u(size),
                        u(contrast),
                        pattern,
                        Rgba8::BLACK,
                        Rgba8::WHITE,
                    )
                })
            }),
        Filter::NotePaper {
            image_balance,
            graininess,
            relief,
            seed,
        } => check("note paper balance", image_balance, 0..=50)
            .and(check("note paper graininess", graininess, 0..=20))
            .and(check("note paper relief", relief, 0..=25))
            .map(|()| {
                // ponytail: CS6 draws with the document foreground and
                // background; the variant carries no colours yet, so CS6's
                // default black and white stand in.
                seeded(seed, move |p| {
                    sketch::note_paper(
                        p,
                        u(image_balance),
                        u(graininess),
                        u(relief),
                        Rgba8::BLACK,
                        Rgba8::WHITE,
                    )
                })
            }),
        Filter::Photocopy { detail, darkness } => check("photocopy detail", detail, 0..=24)
            .and(check("photocopy darkness", darkness, 1..=50))
            .map(|()| {
                // ponytail: CS6 draws with the document foreground and
                // background; the variant carries no colours yet, so CS6's
                // default black and white stand in.
                unseeded(move |p| {
                    sketch::photocopy(p, u(detail), u(darkness), Rgba8::BLACK, Rgba8::WHITE)
                })
            }),
        Filter::Plaster {
            image_balance,
            smoothness,
            light_direction,
            foreground,
            background,
        } => check("plaster balance", image_balance, 0..=50)
            .and(check("plaster smoothness", smoothness, 0..=15))
            .map(|()| {
                unseeded(move |p| {
                    sketch::plaster(
                        p,
                        u(image_balance),
                        u(smoothness),
                        light_direction,
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::Reticulation {
            density,
            black_level,
            white_level,
            foreground,
            background,
            seed,
        } => check("reticulation density", density, 0..=50)
            .and(check("reticulation black level", black_level, 0..=50))
            .and(check("reticulation white level", white_level, 0..=50))
            .map(|()| {
                seeded(seed, move |p| {
                    sketch::reticulation(
                        p,
                        u(density),
                        u(black_level),
                        u(white_level),
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::Stamp {
            light_dark_balance,
            smoothness,
            foreground,
            background,
        } => check("stamp balance", light_dark_balance, 0..=50)
            .and(check("stamp smoothness", smoothness, 1..=50))
            .map(|()| {
                unseeded(move |p| {
                    sketch::stamp(
                        p,
                        u(light_dark_balance),
                        u(smoothness),
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::TornEdges {
            image_balance,
            smoothness,
            contrast,
            foreground,
            background,
        } => check("torn edges balance", image_balance, 0..=50)
            .and(check("torn edges smoothness", smoothness, 1..=15))
            .and(check("torn edges contrast", contrast, 1..=25))
            .map(|()| {
                unseeded(move |p| {
                    sketch::torn_edges(
                        p,
                        u(image_balance),
                        u(smoothness),
                        u(contrast),
                        rgb(foreground),
                        rgb(background),
                    )
                })
            }),
        Filter::WaterPaper {
            fiber_length,
            brightness,
            contrast,
            seed,
        } => check("water paper fiber", fiber_length, 3..=50)
            .and(check("water paper brightness", brightness, 0..=100))
            .and(check("water paper contrast", contrast, 0..=100))
            .map(|()| {
                seeded(seed, move |p| {
                    sketch::water_paper(p, u(fiber_length), u(brightness), u(contrast))
                })
            }),
        Filter::Craquelure {
            crack_spacing,
            crack_depth,
            crack_brightness,
        } => check("craquelure spacing", crack_spacing, 2..=100)
            .and(check("craquelure depth", crack_depth, 0..=10))
            .and(check("craquelure brightness", crack_brightness, 0..=10))
            .map(|()| {
                unseeded(move |p| {
                    texture::craquelure(p, u(crack_spacing), u(crack_depth), u(crack_brightness))
                })
            }),
        Filter::Grain {
            intensity,
            contrast,
            grain_type,
            background,
            seed,
        } => check("grain intensity", intensity, 0..=100)
            .and(check("grain contrast", contrast, 0..=100))
            .map(|()| {
                seeded(seed, move |p| {
                    texture::grain(
                        p,
                        u(intensity),
                        u(contrast),
                        grain_type,
                        Rgba8::BLACK,
                        rgb(background),
                    )
                })
            }),
        Filter::MosaicTiles {
            tile_size,
            grout_width,
            lighten_grout,
            seed,
        } => check("mosaic tiles size", tile_size, 2..=100)
            .and(check("mosaic tiles grout", grout_width, 1..=15))
            .and(check("mosaic tiles lighten", lighten_grout, 0..=10))
            .map(|()| {
                seeded(seed, move |p| {
                    texture::mosaic_tiles(p, u(tile_size), u(grout_width), u(lighten_grout))
                })
            }),
        Filter::Patchwork {
            square_size,
            relief,
            seed,
        } => check("patchwork square", square_size, 0..=10)
            .and(check("patchwork relief", relief, 0..=25))
            .map(|()| {
                seeded(seed, move |p| {
                    texture::patchwork(p, u(square_size), u(relief))
                })
            }),
        Filter::StainedGlass {
            cell_size,
            border_thickness,
            light_intensity,
            foreground,
            seed,
        } => check("stained glass cell", cell_size, 2..=50)
            .and(check("stained glass border", border_thickness, 1..=20))
            .and(check("stained glass light", light_intensity, 0..=10))
            .map(|()| {
                seeded(seed, move |p| {
                    texture::stained_glass(
                        p,
                        u(cell_size),
                        u(border_thickness),
                        u(light_intensity),
                        rgb(foreground),
                    )
                })
            }),
        Filter::Texturizer { texture } => check_texture(&texture).map(|()| {
            unseeded(move |p| {
                texture::texturizer(
                    p,
                    texture.surface,
                    u(texture.scaling),
                    u(texture.relief),
                    light(texture.light_direction),
                    texture.invert,
                )
            })
        }),
        Filter::GaussianBlur { .. }
        | Filter::BoxBlur { .. }
        | Filter::ShapeBlur { .. }
        | Filter::MotionBlur { .. }
        | Filter::RadialBlur { .. }
        | Filter::Blur
        | Filter::BlurMore
        | Filter::SmartBlur { .. }
        | Filter::LensBlur { .. }
        | Filter::FieldBlur { .. }
        | Filter::IrisBlur { .. }
        | Filter::TiltShift { .. }
        | Filter::PathBlur { .. }
        | Filter::SpinBlur { .. }
        | Filter::BlurGallery { .. }
        | Filter::Sharpen
        | Filter::SharpenMore
        | Filter::UnsharpMask { .. }
        | Filter::Median { .. }
        | Filter::Despeckle
        | Filter::ReduceNoise(..)
        | Filter::Maximum { .. }
        | Filter::Minimum { .. }
        | Filter::Offset { .. }
        | Filter::HighPass { .. }
        | Filter::Custom { .. }
        | Filter::Solarize
        | Filter::NtscColors
        | Filter::Liquify { .. }
        | Filter::AdaptiveWideAngle { .. }
        | Filter::VanishingPoint { .. }
        | Filter::OceanRipple { .. }
        | Filter::Glass { .. }
        | Filter::DiffuseGlow { .. }
        | Filter::Clouds { .. }
        | Filter::DifferenceClouds { .. }
        | Filter::Fibers { .. }
        | Filter::LensFlare { .. }
        | Filter::Lighting { .. }
        | Filter::OilPaint { .. }
        | Filter::SmartSharpen { .. }
        | Filter::HdrToning(..) => return None,
    };
    Some(plan)
}

fn check_wave(
    generators: u32,
    wavelength: (f64, f64),
    amplitude: (f64, f64),
    scale: (f64, f64),
) -> Result<(), FilterError> {
    check("wave generators", generators, 1..=999)?;
    check_f("wave wavelength", wavelength.0, 1.0..=998.0)?;
    check_f("wave amplitude", amplitude.0, 1.0..=998.0)?;
    check_f(
        "wave wavelength max",
        wavelength.1,
        wavelength.0 + 1.0..=999.0,
    )?;
    check_f("wave amplitude max", amplitude.1, amplitude.0 + 1.0..=999.0)?;
    check_f("wave scale", scale.0, 1.0..=100.0)?;
    check_f("wave scale", scale.1, 1.0..=100.0)
}

/// Shear's control-point curve: at least two finite `(position, offset)`
/// points, both in `-1..=1`, with `position` strictly increasing top to
/// bottom.
fn check_shear_curve(curve: &[(f64, f64)]) -> Result<(), FilterError> {
    if curve.len() < 2 {
        return Err(FilterError::InvalidParams(format!(
            "shear curve needs at least two points, got {}",
            curve.len()
        )));
    }
    for (i, &(position, offset)) in curve.iter().enumerate() {
        if !position.is_finite() || !offset.is_finite() {
            return Err(FilterError::InvalidParams(format!(
                "shear curve point {i} must be finite"
            )));
        }
        if !(-1.0..=1.0).contains(&position) || !(-1.0..=1.0).contains(&offset) {
            return Err(FilterError::InvalidParams(format!(
                "shear curve point {i} must have position and offset within -1.0..=1.0"
            )));
        }
        if i > 0 && position <= curve[i - 1].0 {
            return Err(FilterError::InvalidParams(format!(
                "shear curve position must strictly increase at point {i}"
            )));
        }
    }
    Ok(())
}

fn unseeded(f: impl FnOnce(&mut Pixmap) + 'static) -> (u64, Run) {
    (0, Box::new(f))
}

fn seeded(seed: u64, f: impl FnOnce(&mut Pixmap) + 'static) -> (u64, Run) {
    (seed, Box::new(f))
}

fn check_brush(size: u8, detail: u8, texture: u8) -> Result<(), FilterError> {
    check("brush size", size, 0..=10)?;
    check("brush detail", detail, 0..=10)?;
    check("brush texture", texture, 1..=3)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp() -> PixelBuffer {
        let mut buffer = PixelBuffer::new(8, 8, 3);
        for (i, v) in buffer.data.iter_mut().enumerate() {
            *v = (i * 7) as u8;
        }
        buffer
    }

    #[test]
    fn shear_validation_rejects_bad_curves_without_touching_the_buffer() {
        // The rejected shapes the spec fixes: fewer than two points, a
        // non-finite coordinate, a position that does not strictly increase,
        // and a coordinate outside -1..=1.
        let cases: &[&[(f64, f64)]] = &[
            &[(0.0, 0.0)],
            &[(0.0, 0.0), (0.0, 1.0)],
            &[(0.5, 0.0), (0.2, 1.0)],
            &[(0.0, 0.0), (f64::NAN, 1.0)],
            &[(0.0, 0.0), (1.0, f64::INFINITY)],
            &[(-1.5, 0.0), (1.0, 0.0)],
            &[(0.0, 0.0), (1.0, 1.5)],
        ];
        for curve in cases {
            let mut buffer = ramp();
            let before = buffer.data.clone();
            let filter = Filter::Shear {
                curve: curve.to_vec(),
                fill: ShearFill::WrapAround,
            };
            let applied = apply(&filter, &mut buffer);
            assert!(
                matches!(applied, Some(Err(FilterError::InvalidParams(_)))),
                "curve {curve:?} was not rejected: {applied:?}"
            );
            assert_eq!(buffer.data, before, "curve {curve:?} mutated the buffer");
        }
    }
}
