use super::*;
pub(crate) fn build_ntsc_colors(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::NtscColors
    })
}
