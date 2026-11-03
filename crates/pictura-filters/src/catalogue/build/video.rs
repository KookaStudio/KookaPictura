use super::*;

pub(crate) fn build_ntsc_colors(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::NtscColors
    })
}

pub(crate) fn build_de_interlace(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::DeInterlace {
            field: pick(&INTERLACE_FIELDS, ctx, 0, 0),
            method: pick(&DEINTERLACE_METHODS, ctx, 1, 0),
        }
    })
}
