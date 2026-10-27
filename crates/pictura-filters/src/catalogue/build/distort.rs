use super::*;
pub(crate) fn build_twirl(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Twirl { angle: ctx.f(0) }
    })
}

pub(crate) fn build_pinch(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Pinch { amount: ctx.f(0) }
    })
}

pub(crate) fn build_spherize(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Spherize {
            amount: ctx.f(0),
            mode: pick(&SPHERIZE_MODES, ctx, 1, ctx.d(1).round().max(0.0) as usize),
        }
    })
}

pub(crate) fn build_ripple(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Ripple {
            amount: ctx.f(0),
            size: pick(&RIPPLE_SIZES, ctx, 1, ctx.d(1).round().max(0.0) as usize),
        }
    })
}

pub(crate) fn build_wave(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Wave {
            generators: ctx.u32v(0),
            wavelength: (ctx.f(1), ctx.f(2)),
            amplitude: (ctx.f(3), ctx.f(4)),
            kind: pick(&WAVE_TYPES, ctx, 5, ctx.d(5).round().max(0.0) as usize),
            scale: (ctx.f(6), ctx.f(7)),
            seed: ctx.u64v(8),
            repeat_edge: true,
        }
    })
}

pub(crate) fn build_polar_coordinates(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::PolarCoordinates {
            kind: pick(&POLAR_KINDS, ctx, 0, ctx.d(0).round().max(0.0) as usize),
        }
    })
}

pub(crate) fn build_shear(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;

        let count = ctx.i32v(0);
        let curve = if count == 0 {
            vec![(-1.0, 0.0), (1.0, 0.0)]
        } else if (2..=SHEAR_MAX_POINTS as i32).contains(&count) {
            (0..count as usize)
                .map(|i| (ctx.f(1 + 2 * i), ctx.f(2 + 2 * i)))
                .collect()
        } else {
            return None;
        };
        Filter::Shear {
            curve,
            fill: pick(
                &SHEAR_FILLS,
                ctx,
                SHEAR_SLOTS - 1,
                ctx.d(SHEAR_SLOTS - 1).round().max(0.0) as usize,
            ),
        }
    })
}

pub(crate) fn build_displace(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Displace {
            map: ctx.resources.displace_map(ctx.u32v(4))?,
            horizontal_scale: ctx.f(0),
            vertical_scale: ctx.f(1),
            fit: pick(&DISPLACE_FITS, ctx, 2, ctx.d(2).round().max(0.0) as usize),
            wrap: pick(&SHEAR_FILLS, ctx, 3, ctx.d(3).round().max(0.0) as usize)
                == ShearFill::WrapAround,
        }
    })
}

pub(crate) fn build_zigzag(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::ZigZag {
            amount: ctx.f(0),
            ridges: ctx.u32v(1),
            style: pick(&ZIGZAG_STYLES, ctx, 2, ctx.d(2).round().max(0.0) as usize),
        }
    })
}

pub(crate) fn build_ocean_ripple(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::OceanRipple {
            size: ctx.u32v(0),
            magnitude: ctx.u32v(1),
            seed: ctx.u64v(2),
        }
    })
}

pub(crate) fn build_glass(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Glass {
            distortion: ctx.u32v(0),
            smoothness: ctx.u32v(1),
            texture: pick(&GLASS_TEXTURES, ctx, 2, ctx.d(2).round().max(0.0) as usize),
            scaling: ctx.u32v(3),
            invert: ctx.flag(4),
        }
    })
}

pub(crate) fn build_diffuse_glow(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::DiffuseGlow {
            graininess: ctx.u32v(0),
            glow_amount: ctx.u32v(1),
            clear_amount: ctx.u32v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_accented_edges(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::AccentedEdges {
            edge_width: ctx.u8v(0),
            edge_brightness: ctx.u8v(1),
            smoothness: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_angled_strokes(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::AngledStrokes {
            direction_balance: ctx.u8v(0),
            stroke_length: ctx.u8v(1),
            sharpness: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_crosshatch(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Crosshatch {
            stroke_length: ctx.u8v(0),
            sharpness: ctx.u8v(1),
            strength: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_dark_strokes(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::DarkStrokes {
            balance: ctx.u8v(0),
            black_intensity: ctx.u8v(1),
            white_intensity: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_ink_outlines(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::InkOutlines {
            stroke_length: ctx.u8v(0),
            dark_intensity: ctx.u8v(1),
            light_intensity: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_spatter(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Spatter {
            spray_radius: ctx.u8v(0),
            smoothness: ctx.u8v(1),
            seed: ctx.u64v(2),
        }
    })
}

pub(crate) fn build_sprayed_strokes(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::SprayedStrokes {
            stroke_length: ctx.u8v(0),
            spray_radius: ctx.u8v(1),
            direction: pick(
                &STROKE_DIRECTIONS,
                ctx,
                2,
                ctx.d(2).round().max(0.0) as usize,
            ),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_sumi_e(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::SumiE {
            stroke_width: ctx.u8v(0),
            stroke_pressure: ctx.u8v(1),
            contrast: ctx.u8v(2),
        }
    })
}
