use super::*;
pub(crate) fn build_gaussian_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::GaussianBlur { radius: ctx.f(0) }
    })
}

pub(crate) fn build_box_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::BoxBlur {
            radius: ctx.u32v(0),
        }
    })
}

pub(crate) fn build_shape_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::ShapeBlur {
            radius: ctx.u32v(0),
            shape: (ctx.u32v(1) as usize).min(CUSTOM_SHAPE_NAMES.len() - 1),
        }
    })
}

pub(crate) fn build_surface_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::SurfaceBlur {
            radius: ctx.u32v(0),
            threshold: ctx.u8v(1),
        }
    })
}

pub(crate) fn build_smart_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::SmartBlur {
            radius: ctx.f(0),
            threshold: ctx.f(1),
            quality: pick(&QUALITIES, ctx, 2, ctx.d(2).round().max(0.0) as usize),
            mode: pick(
                &SMART_BLUR_MODES,
                ctx,
                3,
                ctx.d(3).round().max(0.0) as usize,
            ),
        }
    })
}

pub(crate) fn build_motion_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::MotionBlur {
            angle: ctx.f(0),
            distance: ctx.u32v(1),
        }
    })
}

pub(crate) fn build_lens_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::LensBlur {
            depth: match ctx.u32v(1) {
                0 => None,
                _ => ctx.resources.depth_map(ctx.u32v(13)).map(Some)?,
            },
            focal: ctx.u8v(2),
            invert: ctx.flag(3),
            shape: pick(&IRIS_SHAPES, ctx, 4, ctx.d(4).round().max(0.0) as usize),
            radius: ctx.u32v(5),
            curvature: ctx.u8v(6),
            rotation: ctx.f(7),
            specular_brightness: ctx.u8v(8),
            specular_threshold: ctx.u8v(9),
            noise_amount: ctx.u8v(10),
            noise_distribution: pick(
                &NOISE_DISTRIBUTIONS,
                ctx,
                11,
                ctx.d(11).round().max(0.0) as usize,
            ),
            monochromatic: ctx.flag(12),
            faster: ctx.u32v(0) == 0,
            seed: ctx.u64v(14),
        }
    })
}

pub(crate) fn build_field_blur(ctx: &BuildCtx) -> Option<Filter> {
    gallery::field_blur(ctx)
}

pub(crate) fn build_iris_blur(ctx: &BuildCtx) -> Option<Filter> {
    gallery::iris_blur(ctx)
}

pub(crate) fn build_tilt_shift(ctx: &BuildCtx) -> Option<Filter> {
    gallery::tilt_shift(ctx)
}

pub(crate) fn build_path_blur(ctx: &BuildCtx) -> Option<Filter> {
    gallery::path_blur(ctx)
}

pub(crate) fn build_spin_blur(ctx: &BuildCtx) -> Option<Filter> {
    gallery::spin_blur(ctx)
}

pub(crate) fn build_blur_gallery(ctx: &BuildCtx) -> Option<Filter> {
    gallery::blur_gallery(ctx)
}

pub(crate) fn build_radial_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::RadialBlur {
            method: pick(&RADIAL_METHODS, ctx, 0, ctx.d(0).round().max(0.0) as usize),
            amount: ctx.f(1),
            quality: pick(&QUALITIES, ctx, 2, ctx.d(2).round().max(0.0) as usize),
            center: (ctx.f(3), ctx.f(4)),
        }
    })
}

pub(crate) fn build_average(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Average
    })
}

pub(crate) fn build_blur(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::Blur
    })
}

pub(crate) fn build_blur_more(ctx: &BuildCtx) -> Option<Filter> {
    Some({
        ctx.arity()?;
        Filter::BlurMore
    })
}
