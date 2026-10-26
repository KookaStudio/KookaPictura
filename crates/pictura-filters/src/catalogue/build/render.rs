use super::*;
pub(crate) fn build_median(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Median {
            radius: ctx.u32v(0),
        }
    })
}

pub(crate) fn build_despeckle(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Despeckle
    })
}

pub(crate) fn build_reduce_noise(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        let composite = ChannelDetail {
            strength: ctx.f(0),
            preserve_details: ctx.f(1),
        };
        let detail = |base: usize| ChannelDetail {
            strength: ctx.f(base),
            preserve_details: ctx.f(base + 1),
        };
        Filter::ReduceNoise(ReduceNoiseParams {
            reduce_color_noise: ctx.f(2),
            sharpen_details: ctx.f(3),
            remove_jpeg_artifacts: ctx.flag(4),
            channels: if ctx.flag(5) {
                [detail(6), detail(8), detail(10)]
            } else {
                [composite; 3]
            },
        })
    })
}

pub(crate) fn build_sharpen(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Sharpen
    })
}

pub(crate) fn build_sharpen_more(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::SharpenMore
    })
}

pub(crate) fn build_sharpen_edges(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::SharpenEdges
    })
}

pub(crate) fn build_unsharp_mask(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::UnsharpMask {
            amount: ctx.f(0),
            radius: ctx.f(1),
            threshold: ctx.u8v(2),
        }
    })
}

pub(crate) fn build_add_noise(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::AddNoise {
            amount: ctx.f(0),
            distribution: pick(
                &NOISE_DISTRIBUTIONS,
                ctx,
                1,
                ctx.d(1).round().max(0.0) as usize,
            ),
            monochromatic: ctx.flag(2),
            seed: ctx.u64v(3),
        }
    })
}

pub(crate) fn build_mosaic(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Mosaic {
            cell_size: ctx.u32v(0),
        }
    })
}

pub(crate) fn build_crystallize(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Crystallize {
            cell_size: ctx.u32v(0),
            seed: ctx.u64v(1),
        }
    })
}

pub(crate) fn build_facet(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Facet
    })
}

pub(crate) fn build_fragment(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Fragment
    })
}

pub(crate) fn build_mezzotint(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Mezzotint {
            kind: pick(&MEZZOTINT_TYPES, ctx, 0, ctx.d(0).round().max(0.0) as usize),
            seed: ctx.u64v(1),
        }
    })
}

pub(crate) fn build_pointillize(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Pointillize {
            cell_size: ctx.u32v(0),
            background: rgb(ctx, 1, [ctx.d(1), ctx.d(2), ctx.d(3)]),
            seed: ctx.u64v(4),
        }
    })
}

pub(crate) fn build_color_halftone(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::ColorHalftone {
            max_radius: ctx.u32v(0),
            angles: [ctx.f(1), ctx.f(2), ctx.f(3), ctx.f(4)],
        }
    })
}

pub(crate) fn build_clouds(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Clouds {
            color_a: rgb(ctx, 0, [ctx.d(0), ctx.d(1), ctx.d(2)]),
            color_b: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            starker: ctx.flag(6),
            seed: ctx.u64v(7),
        }
    })
}

pub(crate) fn build_difference_clouds(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::DifferenceClouds {
            color_a: rgb(ctx, 0, [ctx.d(0), ctx.d(1), ctx.d(2)]),
            color_b: rgb(ctx, 3, [ctx.d(3), ctx.d(4), ctx.d(5)]),
            starker: ctx.flag(6),
            seed: ctx.u64v(7),
        }
    })
}

pub(crate) fn build_fibers(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Fibers {
            variance: ctx.f(0),
            strength: ctx.f(1),
            color_a: rgb(ctx, 2, [ctx.d(2), ctx.d(3), ctx.d(4)]),
            color_b: rgb(ctx, 5, [ctx.d(5), ctx.d(6), ctx.d(7)]),
            seed: ctx.u64v(8),
        }
    })
}

pub(crate) fn build_lens_flare(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::LensFlare {
            brightness: ctx.f(0),
            center: (ctx.f(1), ctx.f(2)),
            lens: pick(&LENS_TYPES, ctx, 3, ctx.d(3).round().max(0.0) as usize),
        }
    })
}

pub(crate) fn build_picture_frame(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::PictureFrame {
            options: FrameOptions {
                frame: ctx.u32v(0) + 1,
                vine: rgb(ctx, 1, [ctx.d(1), ctx.d(2), ctx.d(3)]),
                margin: ctx.u32v(4),
                size: ctx.u32v(5),
                arrangement: ctx.u32v(6),
                flower: ctx.u32v(7),
                flower_colour: rgb(ctx, 8, [ctx.d(8), ctx.d(9), ctx.d(10)]),
                flower_size: ctx.u32v(11),
                leaf: ctx.u32v(12),
                leaf_colour: rgb(ctx, 13, [ctx.d(13), ctx.d(14), ctx.d(15)]),
                leaf_size: ctx.u32v(16),
                lines: ctx.u32v(17),
                thickness: ctx.u32v(18),
                angle: ctx.u32v(19),
                fade: ctx.u32v(20),
                invert: ctx.flag(21),
            },
        }
    })
}

pub(crate) fn build_lighting_effects(ctx: &BuildCtx) -> Option<Filter> {
    Some(Filter::Lighting {
        lighting: lighting(ctx)?,
    })
}

pub(crate) fn build_dust_and_scratches(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::DustAndScratches {
            radius: ctx.u32v(0),
            threshold: ctx.u32v(1),
        }
    })
}

pub(crate) fn build_smart_sharpen(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::SmartSharpen {
            amount: ctx.f(0),
            radius: ctx.f(1),
            reduce_noise: ctx.f(2),
            remove: pick(&SHARPEN_REMOVES, ctx, 3, ctx.d(3).round().max(0.0) as usize),
            angle: ctx.f(4),
            more_accurate: ctx.flag(5),
            shadow: tonal_fade(ctx, 6),
            highlight: tonal_fade(ctx, 9),
        }
    })
}
