use pictura_core::PixelBuffer;

use crate::{
    blur, distort, hdr_toning, noise, oil_paint, other, render, sharpen, stylize, validate, video,
    FilterError,
};

use super::{Filter, FilterEngine};

/// Apply `filter` in place (planar 8-bit; channels 3 or 4). On a four-channel
/// buffer the kernel itself decides the alpha plane: moving kernels (see
/// [`Filter::moves_alpha`]) process it, every other kernel leaves it
/// bit-identical.
pub fn apply(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError> {
    validate(buf)?;
    match filter.engine() {
        FilterEngine::Native => apply_native(filter, buf),
        FilterEngine::Ported => crate::photorust::dispatch::apply(filter, buf)
            .unwrap_or_else(|| Err(FilterError::Unsupported(format!("{filter:?}")))),
    }
}

fn apply_native(filter: &Filter, buf: &mut PixelBuffer) -> Result<(), FilterError> {
    match filter {
        Filter::GaussianBlur { radius } => blur::gaussian(buf, *radius),
        Filter::BoxBlur { radius } => blur::r#box(buf, *radius),
        Filter::ShapeBlur { shape, radius } => blur::shape(buf, *shape, *radius),
        Filter::Liquify { mesh } => distort::liquify::liquify(buf, mesh),
        Filter::AdaptiveWideAngle { params } => {
            distort::adaptive_wide_angle::adaptive_wide_angle(buf, params)
        }
        Filter::VanishingPoint { edit } => distort::vanishing_point::vanishing_point(buf, edit),
        Filter::SmartBlur {
            radius,
            threshold,
            quality,
            mode,
        } => blur::smart(buf, *radius, *threshold, *quality, *mode),
        Filter::MotionBlur { angle, distance } => blur::motion(buf, *angle, *distance),
        Filter::RadialBlur {
            method,
            amount,
            center,
            quality,
        } => blur::radial(buf, *method, *amount, *center, *quality),
        Filter::LensBlur {
            depth,
            focal,
            invert,
            shape,
            radius,
            curvature,
            rotation,
            specular_brightness,
            specular_threshold,
            noise_amount,
            noise_distribution,
            monochromatic,
            faster,
            seed,
        } => blur::lens::lens_blur(
            buf,
            depth.as_deref(),
            &blur::lens::LensParams {
                focal: *focal,
                invert: *invert,
                shape: *shape,
                radius: *radius,
                curvature: *curvature,
                rotation: *rotation,
                specular_brightness: *specular_brightness,
                specular_threshold: *specular_threshold,
                noise_amount: *noise_amount,
                noise_distribution: *noise_distribution,
                monochromatic: *monochromatic,
                faster: *faster,
                seed: *seed,
            },
        ),
        Filter::FieldBlur { pins, bokeh, noise } => {
            blur::field::field_blur(buf, pins, bokeh, noise)
        }
        Filter::IrisBlur { pins, bokeh, noise } => blur::iris::iris_blur(buf, pins, bokeh, noise),
        Filter::TiltShift { pins, bokeh, noise } => blur::tilt::tilt_shift(buf, pins, bokeh, noise),
        Filter::PathBlur {
            paths,
            motion,
            noise,
        } => blur::path::path_blur(buf, paths, motion, noise),
        Filter::SpinBlur {
            spins,
            strobe,
            noise,
        } => blur::spin::spin_blur(buf, spins, strobe, noise),
        Filter::BlurGallery {
            field,
            iris,
            tilt,
            path,
            motion,
            spin,
            spin_strobe,
            bokeh,
            noise,
        } => blur::gallery::blur_gallery(
            buf,
            field,
            iris,
            tilt,
            path,
            motion,
            spin,
            spin_strobe,
            bokeh,
            noise,
        ),
        Filter::Blur => blur::simple(buf, false),
        Filter::BlurMore => blur::simple(buf, true),
        Filter::Sharpen => sharpen::sharpen(buf),
        Filter::SharpenMore => sharpen::sharpen_more(buf),
        Filter::UnsharpMask {
            amount,
            radius,
            threshold,
        } => sharpen::unsharp_mask(buf, *amount, *radius, *threshold),
        Filter::Median { radius } => noise::median(buf, *radius),
        Filter::Despeckle => noise::despeckle(buf),
        Filter::ReduceNoise(params) => noise::reduce_noise(buf, params),
        Filter::Maximum { radius } => other::maximum(buf, *radius),
        Filter::Minimum { radius } => other::minimum(buf, *radius),
        Filter::Offset {
            horizontal,
            vertical,
            wrap,
            repeat,
            background,
            transparent,
        } => other::offset(
            buf,
            *horizontal,
            *vertical,
            *wrap,
            *repeat,
            *background,
            *transparent,
        ),
        Filter::HighPass { radius } => other::high_pass(buf, *radius),
        Filter::Custom {
            kernel,
            scale,
            offset,
        } => other::custom(buf, kernel, *scale, *offset),
        Filter::Solarize => stylize::solarize(buf),
        Filter::NtscColors => video::ntsc_colors(buf),
        Filter::OceanRipple {
            size,
            magnitude,
            seed,
        } => distort::ocean_ripple(buf, *size, *magnitude, *seed),
        Filter::Glass {
            distortion,
            smoothness,
            texture,
            scaling,
            invert,
        } => distort::glass(buf, *distortion, *smoothness, *texture, *scaling, *invert),
        Filter::DiffuseGlow {
            graininess,
            glow_amount,
            clear_amount,
            seed,
        } => distort::diffuse_glow(buf, *graininess, *glow_amount, *clear_amount, *seed),
        Filter::Clouds {
            color_a,
            color_b,
            starker,
            seed,
        } => render::clouds(buf, *color_a, *color_b, *starker, *seed),
        Filter::DifferenceClouds {
            color_a,
            color_b,
            starker,
            seed,
        } => render::difference_clouds(buf, *color_a, *color_b, *starker, *seed),
        Filter::Fibers {
            variance,
            strength,
            color_a,
            color_b,
            seed,
        } => render::fibers(buf, *variance, *strength, *color_a, *color_b, *seed),
        Filter::LensFlare {
            brightness,
            center,
            lens,
        } => render::lens_flare(buf, *brightness, *center, *lens),
        Filter::Lighting { lighting } => render::lighting_effects(buf, lighting),
        Filter::OilPaint {
            stylization,
            cleanliness,
            scale,
            bristle_detail,
            angular_direction,
            shine,
        } => oil_paint::oil_paint(
            buf,
            *stylization,
            *cleanliness,
            *scale,
            *bristle_detail,
            *angular_direction,
            *shine,
        ),
        Filter::SmartSharpen {
            amount,
            radius,
            reduce_noise,
            remove,
            angle,
            more_accurate,
            shadow,
            highlight,
        } => sharpen::smart_sharpen(
            buf,
            *amount,
            *radius,
            *reduce_noise,
            *remove,
            *angle,
            *more_accurate,
            *shadow,
            *highlight,
        ),
        Filter::HdrToning(params) => hdr_toning::hdr_toning(buf, params),
        Filter::Average
        | Filter::SurfaceBlur { .. }
        | Filter::SharpenEdges
        | Filter::AddNoise { .. }
        | Filter::Emboss { .. }
        | Filter::FindEdges
        | Filter::Diffuse { .. }
        | Filter::GlowingEdges { .. }
        | Filter::Mosaic { .. }
        | Filter::Crystallize { .. }
        | Filter::Facet
        | Filter::Fragment
        | Filter::Mezzotint { .. }
        | Filter::Pointillize { .. }
        | Filter::ColorHalftone { .. }
        | Filter::Twirl { .. }
        | Filter::Pinch { .. }
        | Filter::Spherize { .. }
        | Filter::Ripple { .. }
        | Filter::Wave { .. }
        | Filter::PolarCoordinates { .. }
        | Filter::Shear { .. }
        | Filter::Displace { .. }
        | Filter::ZigZag { .. }
        | Filter::PictureFrame { .. }
        | Filter::Cutout { .. }
        | Filter::FilmGrain { .. }
        | Filter::NeonGlow { .. }
        | Filter::PosterEdges { .. }
        | Filter::PaintDaubs { .. }
        | Filter::PaletteKnife { .. }
        | Filter::PlasticWrap { .. }
        | Filter::Sponge { .. }
        | Filter::ColoredPencil { .. }
        | Filter::DryBrush { .. }
        | Filter::Fresco { .. }
        | Filter::RoughPastels { .. }
        | Filter::SmudgeStick { .. }
        | Filter::Underpainting { .. }
        | Filter::Watercolor { .. }
        | Filter::AccentedEdges { .. }
        | Filter::AngledStrokes { .. }
        | Filter::Crosshatch { .. }
        | Filter::DarkStrokes { .. }
        | Filter::InkOutlines { .. }
        | Filter::Spatter { .. }
        | Filter::SprayedStrokes { .. }
        | Filter::SumiE { .. }
        | Filter::BasRelief { .. }
        | Filter::ChalkCharcoal { .. }
        | Filter::Charcoal { .. }
        | Filter::Chrome { .. }
        | Filter::ConteCrayon { .. }
        | Filter::GraphicPen { .. }
        | Filter::HalftonePattern { .. }
        | Filter::NotePaper { .. }
        | Filter::Photocopy { .. }
        | Filter::Plaster { .. }
        | Filter::Reticulation { .. }
        | Filter::Stamp { .. }
        | Filter::TornEdges { .. }
        | Filter::WaterPaper { .. }
        | Filter::Craquelure { .. }
        | Filter::Grain { .. }
        | Filter::MosaicTiles { .. }
        | Filter::Patchwork { .. }
        | Filter::StainedGlass { .. }
        | Filter::Texturizer { .. }
        | Filter::DustAndScratches { .. }
        | Filter::Extrude { .. }
        | Filter::Tiles { .. }
        | Filter::TraceContour { .. }
        | Filter::Wind { .. } => Err(FilterError::Unsupported(format!("{filter:?}"))),
    }
}
