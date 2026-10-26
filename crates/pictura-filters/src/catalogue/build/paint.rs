use super::*;
pub(crate) fn build_liquify(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Liquify {
            mesh: ctx.resources.liquify_mesh(ctx.u32v(0))?,
        }
    })
}

pub(crate) fn build_vanishing_point(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::VanishingPoint {
            edit: ctx.resources.vanishing_point(ctx.u32v(0))?,
        }
    })
}

pub(crate) fn build_adaptive_wide_angle(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        for slot in [0, 1, 3, 4, 5] {
            let value = ctx.f(slot);
            let range = ctx.spec.range_for_slot(slot)?;
            if !value.is_finite() || !range.contains(&value) {
                return None;
            }
        }
        let crop = ctx.f(3);
        let (low, high) = focal_range(crop);
        let focal = ctx.f(2);
        if !focal.is_finite() {
            return None;
        }
        Filter::AdaptiveWideAngle {
            params: AwaParams {
                correction: awa_correction(ctx.f(0)),
                scale: ctx.f(1) / 100.0,
                focal_length: focal.clamp(low, high),
                crop_factor: crop,
                offset: [ctx.f(4), ctx.f(5)],
                constraints: ctx.resources.awa_constraints(ctx.u32v(6))?,
            },
        }
    })
}

pub(crate) fn build_lens_correction(ctx: &BuildCtx) -> Option<Filter> {
    ctx.arity()?;
    let slots = if ctx.params.is_empty() {
        ctx.defaults
    } else {
        ctx.params
    };
    Some(Filter::LensCorrection {
        params: LensCorrectionParams::from_slots(slots)?,
    })
}

pub(crate) fn build_colored_pencil(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::ColoredPencil {
            pencil_width: ctx.u8v(0),
            stroke_pressure: ctx.u8v(1),
            paper_brightness: ctx.u8v(2),

            background: [255, 255, 255],
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_cutout(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Cutout {
            levels: ctx.u8v(0),
            edge_simplicity: ctx.u8v(1),
            edge_fidelity: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_dry_brush(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::DryBrush {
            brush_size: ctx.u8v(0),
            brush_detail: ctx.u8v(1),
            texture: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_film_grain(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::FilmGrain {
            grain: ctx.u8v(0),
            highlight_area: ctx.u8v(1),
            intensity: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_fresco(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Fresco {
            brush_size: ctx.u8v(0),
            brush_detail: ctx.u8v(1),
            texture: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_neon_glow(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::NeonGlow {
            glow_size: ctx.i32v(0),
            glow_brightness: ctx.u8v(1),
            glow_color: rgb(ctx, 2, [ctx.d(2), ctx.d(3), ctx.d(4)]),
        }
    })
}

pub(crate) fn build_paint_daubs(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::PaintDaubs {
            brush_size: ctx.u8v(0),
            sharpness: ctx.u8v(1),
            brush_type: pick(&BRUSH_TYPES, ctx, 2, ctx.d(2).round().max(0.0) as usize),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_palette_knife(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::PaletteKnife {
            stroke_size: ctx.u8v(0),
            stroke_detail: ctx.u8v(1),
            softness: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_plastic_wrap(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::PlasticWrap {
            highlight_strength: ctx.u8v(0),
            detail: ctx.u8v(1),
            smoothness: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_poster_edges(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::PosterEdges {
            edge_thickness: ctx.u8v(0),
            edge_intensity: ctx.u8v(1),
            posterization: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_rough_pastels(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::RoughPastels {
            stroke_length: ctx.u8v(0),
            stroke_detail: ctx.u8v(1),
            texture: TextureOptions {
                relief: ctx.u8v(4),
                ..texture(ctx, 2)
            },
            seed: 1,
        }
    })
}

pub(crate) fn build_smudge_stick(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::SmudgeStick {
            stroke_length: ctx.u8v(0),
            highlight_area: ctx.u8v(1),
            intensity: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_sponge(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Sponge {
            brush_size: ctx.u8v(0),
            definition: ctx.u8v(1),
            smoothness: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_underpainting(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Underpainting {
            brush_size: ctx.u8v(0),
            texture_coverage: ctx.u8v(1),
            texture: TextureOptions {
                light_direction: ctx.u8v(5),
                ..texture(ctx, 2)
            },
            seed: ctx.u64v(7),
        }
    })
}

pub(crate) fn build_watercolor(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Watercolor {
            brush_detail: ctx.u8v(0),
            shadow_intensity: ctx.u8v(1),
            texture: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_oil_paint(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::OilPaint {
            stylization: ctx.f(0),
            cleanliness: ctx.f(1),
            scale: ctx.f(2),
            bristle_detail: ctx.f(3),
            angular_direction: ctx.f(4),
            shine: ctx.f(5),
        }
    })
}
