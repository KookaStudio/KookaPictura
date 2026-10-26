use super::*;
pub(crate) fn build_maximum(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Maximum {
            radius: ctx.u32v(0),
        }
    })
}

pub(crate) fn build_minimum(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Minimum {
            radius: ctx.u32v(0),
        }
    })
}

pub(crate) fn build_offset(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        let repeat = ctx.flag(3);
        Filter::Offset {
            horizontal: ctx.i32v(0),
            vertical: ctx.i32v(1),
            wrap: ctx.flag(2) && !repeat,
            repeat,
            background: rgb(ctx, 4, [ctx.d(4), ctx.d(5), ctx.d(6)]),
            transparent: ctx.flag(7),
        }
    })
}

pub(crate) fn build_high_pass(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::HighPass { radius: ctx.f(0) }
    })
}

pub(crate) fn build_custom(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        let mut kernel = [[0.0; 5]; 5];
        for (i, k) in kernel.iter_mut().flatten().enumerate() {
            *k = ctx.f(i);
        }
        Filter::Custom {
            kernel,
            scale: ctx.f(25),
            offset: ctx.f(26),
        }
    })
}

pub(crate) fn build_emboss(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Emboss {
            angle: ctx.f(0),
            height: ctx.f(1),
            amount: ctx.f(2),
        }
    })
}

pub(crate) fn build_find_edges(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::FindEdges
    })
}

pub(crate) fn build_solarize(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Solarize
    })
}

pub(crate) fn build_diffuse(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Diffuse {
            mode: pick(&DIFFUSE_MODES, ctx, 0, ctx.d(0).round().max(0.0) as usize),
        }
    })
}

pub(crate) fn build_glowing_edges(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::GlowingEdges {
            width: ctx.u32v(0),
            brightness: ctx.u32v(1),
            smoothness: ctx.u32v(2),
        }
    })
}

pub(crate) fn build_bas_relief(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::BasRelief {
            detail: ctx.u8v(0),
            smoothness: ctx.u8v(1),
            light_direction: pick(
                &LIGHT_DIRECTIONS,
                ctx,
                2,
                ctx.d(2).round().max(0.0) as usize,
            ),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            background: rgb(ctx, 6, [ctx.d(6), ctx.d(7), ctx.d(8)]),
        }
    })
}

pub(crate) fn build_chalk_charcoal(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::ChalkCharcoal {
            charcoal_area: ctx.u8v(0),
            chalk_area: ctx.u8v(1),
            stroke_pressure: ctx.u8v(2),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            background: rgb(ctx, 6, [ctx.d(6), ctx.d(7), ctx.d(8)]),
            seed: ctx.u64v(9),
        }
    })
}

pub(crate) fn build_charcoal(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Charcoal {
            thickness: ctx.u8v(0),
            detail: ctx.u8v(1),
            light_dark_balance: ctx.u8v(2),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            background: rgb(ctx, 6, [ctx.d(6), ctx.d(7), ctx.d(8)]),
            seed: ctx.u64v(9),
        }
    })
}

pub(crate) fn build_chrome(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Chrome {
            detail: ctx.u8v(0),
            smoothness: ctx.u8v(1),
        }
    })
}

pub(crate) fn build_conte_crayon(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::ConteCrayon {
            foreground_level: ctx.u8v(0),
            background_level: ctx.u8v(1),
            texture: texture(ctx, 2),
            foreground: rgb(ctx, 7, [ctx.d(7), ctx.d(8), ctx.d(9)]),
            background: rgb(ctx, 10, [ctx.d(10), ctx.d(11), ctx.d(12)]),
            seed: ctx.u64v(13),
        }
    })
}

pub(crate) fn build_graphic_pen(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::GraphicPen {
            stroke_length: ctx.u8v(0),
            light_dark_balance: ctx.u8v(1),
            direction: pick(
                &STROKE_DIRECTIONS,
                ctx,
                2,
                ctx.d(2).round().max(0.0) as usize,
            ),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            background: rgb(ctx, 6, [ctx.d(6), ctx.d(7), ctx.d(8)]),
        }
    })
}

pub(crate) fn build_halftone_pattern(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::HalftonePattern {
            size: ctx.u8v(0),
            contrast: ctx.u8v(1),
            pattern: pick(&HALFTONE_TYPES, ctx, 2, ctx.d(2).round().max(0.0) as usize),
        }
    })
}

pub(crate) fn build_note_paper(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::NotePaper {
            image_balance: ctx.u8v(0),
            graininess: ctx.u8v(1),
            relief: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_photocopy(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Photocopy {
            detail: ctx.u8v(0),
            darkness: ctx.u8v(1),
        }
    })
}

pub(crate) fn build_plaster(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Plaster {
            image_balance: ctx.u8v(0),
            smoothness: ctx.u8v(1),
            light_direction: pick(
                &LIGHT_DIRECTIONS,
                ctx,
                2,
                ctx.d(2).round().max(0.0) as usize,
            ),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            background: rgb(ctx, 6, [ctx.d(6), ctx.d(7), ctx.d(8)]),
        }
    })
}

pub(crate) fn build_reticulation(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Reticulation {
            density: ctx.u8v(0),
            black_level: ctx.u8v(1),
            white_level: ctx.u8v(2),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            background: rgb(ctx, 6, [ctx.d(6), ctx.d(7), ctx.d(8)]),
            seed: ctx.u64v(9),
        }
    })
}

pub(crate) fn build_stamp(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Stamp {
            light_dark_balance: ctx.u8v(0),
            smoothness: ctx.u8v(1),
            foreground: rgb(ctx, 2, [ctx.d(2), ctx.d(3), ctx.d(4)]),
            background: rgb(ctx, 5, [ctx.d(5), ctx.d(6), ctx.d(7)]),
        }
    })
}

pub(crate) fn build_torn_edges(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::TornEdges {
            image_balance: ctx.u8v(0),
            smoothness: ctx.u8v(1),
            contrast: ctx.u8v(2),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            background: rgb(ctx, 6, [ctx.d(6), ctx.d(7), ctx.d(8)]),
        }
    })
}

pub(crate) fn build_water_paper(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::WaterPaper {
            fiber_length: ctx.u8v(0),
            brightness: ctx.u8v(1),
            contrast: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_craquelure(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Craquelure {
            crack_spacing: ctx.u8v(0),
            crack_depth: ctx.u8v(1),
            crack_brightness: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_grain(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Grain {
            intensity: ctx.u8v(0),
            contrast: ctx.u8v(1),
            grain_type: pick(&GRAIN_TYPES, ctx, 2, ctx.d(2).round().max(0.0) as usize),
            background: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            seed: ctx.u64v(6),
        }
    })
}

pub(crate) fn build_mosaic_tiles(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::MosaicTiles {
            tile_size: ctx.u8v(0),
            grout_width: ctx.u8v(1),
            lighten_grout: ctx.u8v(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_patchwork(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Patchwork {
            square_size: ctx.u8v(0),
            relief: ctx.u8v(1),
            seed: ctx.u64v(2),
        }
    })
}

pub(crate) fn build_stained_glass(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::StainedGlass {
            cell_size: ctx.u8v(0),
            border_thickness: ctx.u8v(1),
            light_intensity: ctx.u8v(2),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            seed: ctx.u64v(6),
        }
    })
}

pub(crate) fn build_texturizer(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Texturizer {
            texture: texture(ctx, 0),
        }
    })
}

pub(crate) fn build_extrude(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Extrude {
            kind: pick(&EXTRUDE_TYPES, ctx, 0, ctx.d(0).round().max(0.0) as usize),
            size: ctx.u32v(1),
            depth: ctx.f(2) as f32,
            level_based: ctx.flag(3),
            solid_front: ctx.flag(4),
            mask_incomplete: ctx.flag(5),
        }
    })
}

pub(crate) fn build_tiles(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Tiles {
            count: ctx.u32v(0),
            offset: ctx.u32v(1),
            fill: pick(&TILE_FILLS, ctx, 2, ctx.d(2).round().max(0.0) as usize),
            foreground: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            background: rgb(ctx, 6, [ctx.d(6), ctx.d(7), ctx.d(8)]),
        }
    })
}

pub(crate) fn build_trace_contour(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::TraceContour {
            level: ctx.u8v(0),
            edge: pick(&CONTOUR_EDGES, ctx, 1, ctx.d(1).round().max(0.0) as usize),
        }
    })
}

pub(crate) fn build_wind(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Wind {
            method: pick(&WIND_METHODS, ctx, 0, ctx.d(0).round().max(0.0) as usize),
            from_right: pick(&WIND_FROM_RIGHT, ctx, 1, ctx.d(1).round().max(0.0) as usize),
        }
    })
}
