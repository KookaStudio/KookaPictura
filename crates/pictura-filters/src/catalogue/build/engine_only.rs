use super::*;

pub(crate) fn build_hdr_toning(ctx: &BuildCtx) -> Option<Filter> {
    ctx.arity()?;
    Some(Filter::HdrToning(crate::HdrToningParams::default()))
}
