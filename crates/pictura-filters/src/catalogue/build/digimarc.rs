use crate::digimarc::{Durability, WatermarkKind, MAX_CREATOR_ID, MAX_ID};

use super::*;

pub(crate) fn build_embed_watermark(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::EmbedWatermark {
            kind: pick(
                &[WatermarkKind::ImageId, WatermarkKind::TransactionId],
                ctx,
                0,
                0,
            ),
            creator_id: ctx.u32v(1).min(MAX_CREATOR_ID),
            id: ctx.u32v(2).min(MAX_ID),
            durability: pick(&Durability::ALL, ctx, 3, 1),
        }
    })
}
