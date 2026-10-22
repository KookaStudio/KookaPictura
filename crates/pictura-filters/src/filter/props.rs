use crate::{BlurPath, PathMotion, SmartBlurMode};

use super::variants::FilterKind;
use super::Filter;

impl FilterKind {
    /// Whether this kind moves, spreads, or ranks samples independently of
    /// their value, so its kernel runs over the alpha plane too.
    #[allow(clippy::match_like_matches_macro)]
    pub fn moves_alpha(self) -> bool {
        match self {
            FilterKind::GaussianBlur
            | FilterKind::BoxBlur
            | FilterKind::ShapeBlur
            | FilterKind::MotionBlur
            | FilterKind::RadialBlur
            | FilterKind::Average
            | FilterKind::Blur
            | FilterKind::BlurMore
            | FilterKind::SurfaceBlur
            | FilterKind::SmartBlur
            | FilterKind::LensBlur
            | FilterKind::FieldBlur
            | FilterKind::IrisBlur
            | FilterKind::TiltShift
            | FilterKind::PathBlur
            | FilterKind::SpinBlur
            | FilterKind::BlurGallery
            | FilterKind::Sharpen
            | FilterKind::SharpenMore
            | FilterKind::SharpenEdges
            | FilterKind::UnsharpMask
            | FilterKind::Median
            | FilterKind::Despeckle
            | FilterKind::Maximum
            | FilterKind::Minimum
            | FilterKind::Offset
            | FilterKind::Mosaic
            | FilterKind::Crystallize
            | FilterKind::Fragment
            | FilterKind::Twirl
            | FilterKind::Pinch
            | FilterKind::Spherize
            | FilterKind::Ripple
            | FilterKind::Wave
            | FilterKind::PolarCoordinates
            | FilterKind::Shear
            | FilterKind::Displace
            | FilterKind::Liquify
            | FilterKind::ZigZag
            | FilterKind::OceanRipple
            | FilterKind::Glass
            | FilterKind::PictureFrame
            | FilterKind::DustAndScratches
            | FilterKind::SmartSharpen => true,
            FilterKind::AddNoise
            | FilterKind::ReduceNoise
            | FilterKind::HighPass
            | FilterKind::Custom
            | FilterKind::Emboss
            | FilterKind::FindEdges
            | FilterKind::Solarize
            | FilterKind::Diffuse
            | FilterKind::GlowingEdges
            | FilterKind::Facet
            | FilterKind::Mezzotint
            | FilterKind::Pointillize
            | FilterKind::ColorHalftone
            | FilterKind::DiffuseGlow
            | FilterKind::Clouds
            | FilterKind::DifferenceClouds
            | FilterKind::Fibers
            | FilterKind::LensFlare
            | FilterKind::Lighting
            | FilterKind::Cutout
            | FilterKind::FilmGrain
            | FilterKind::NeonGlow
            | FilterKind::PosterEdges
            | FilterKind::PaintDaubs
            | FilterKind::PaletteKnife
            | FilterKind::PlasticWrap
            | FilterKind::Sponge
            | FilterKind::ColoredPencil
            | FilterKind::DryBrush
            | FilterKind::Fresco
            | FilterKind::RoughPastels
            | FilterKind::SmudgeStick
            | FilterKind::Underpainting
            | FilterKind::Watercolor
            | FilterKind::AccentedEdges
            | FilterKind::AngledStrokes
            | FilterKind::Crosshatch
            | FilterKind::DarkStrokes
            | FilterKind::InkOutlines
            | FilterKind::Spatter
            | FilterKind::SprayedStrokes
            | FilterKind::SumiE
            | FilterKind::BasRelief
            | FilterKind::ChalkCharcoal
            | FilterKind::Charcoal
            | FilterKind::Chrome
            | FilterKind::ConteCrayon
            | FilterKind::GraphicPen
            | FilterKind::HalftonePattern
            | FilterKind::NotePaper
            | FilterKind::Photocopy
            | FilterKind::Plaster
            | FilterKind::Reticulation
            | FilterKind::Stamp
            | FilterKind::TornEdges
            | FilterKind::WaterPaper
            | FilterKind::Craquelure
            | FilterKind::Grain
            | FilterKind::MosaicTiles
            | FilterKind::Patchwork
            | FilterKind::StainedGlass
            | FilterKind::Texturizer
            | FilterKind::OilPaint
            | FilterKind::Extrude
            | FilterKind::Tiles
            | FilterKind::TraceContour
            | FilterKind::Wind
            | FilterKind::HdrToning => false,
        }
    }

    /// Kernels whose row-parallel CPU path beats the GPU round trip, so a
    /// layer filter skips the GPU for them. Measured at 8192² on an integrated
    /// AMD 780M (#152): uploading and reading back cost about what the CPU takes
    /// for the whole kernel (Gaussian radius 4: GPU 0.91 s, CPU 0.46 s; Median
    /// radius 10: GPU 11.2 s, CPU 3.4 s). Box, Motion and Surface Blur and Oil
    /// Paint stay on the GPU, which still wins there.
    /// ponytail: one machine's crossover; re-measure on a discrete GPU.
    #[allow(clippy::match_like_matches_macro)]
    pub fn cpu_outruns_gpu(self) -> bool {
        match self {
            FilterKind::GaussianBlur
            | FilterKind::Blur
            | FilterKind::BlurMore
            | FilterKind::Sharpen
            | FilterKind::SharpenMore
            | FilterKind::UnsharpMask
            | FilterKind::Median
            | FilterKind::Maximum
            | FilterKind::Minimum
            | FilterKind::HighPass
            | FilterKind::Custom => true,
            FilterKind::BoxBlur
            | FilterKind::ShapeBlur
            | FilterKind::MotionBlur
            | FilterKind::RadialBlur
            | FilterKind::Average
            | FilterKind::SurfaceBlur
            | FilterKind::SmartBlur
            | FilterKind::LensBlur
            | FilterKind::FieldBlur
            | FilterKind::IrisBlur
            | FilterKind::TiltShift
            | FilterKind::PathBlur
            | FilterKind::SpinBlur
            | FilterKind::BlurGallery
            | FilterKind::SharpenEdges
            | FilterKind::AddNoise
            | FilterKind::Despeckle
            | FilterKind::ReduceNoise
            | FilterKind::Offset
            | FilterKind::Emboss
            | FilterKind::FindEdges
            | FilterKind::Solarize
            | FilterKind::Diffuse
            | FilterKind::GlowingEdges
            | FilterKind::Mosaic
            | FilterKind::Crystallize
            | FilterKind::Facet
            | FilterKind::Fragment
            | FilterKind::Mezzotint
            | FilterKind::Pointillize
            | FilterKind::ColorHalftone
            | FilterKind::Twirl
            | FilterKind::Pinch
            | FilterKind::Spherize
            | FilterKind::Ripple
            | FilterKind::Wave
            | FilterKind::PolarCoordinates
            | FilterKind::Shear
            | FilterKind::Displace
            | FilterKind::Liquify
            | FilterKind::ZigZag
            | FilterKind::OceanRipple
            | FilterKind::Glass
            | FilterKind::DiffuseGlow
            | FilterKind::Clouds
            | FilterKind::DifferenceClouds
            | FilterKind::Fibers
            | FilterKind::PictureFrame
            | FilterKind::LensFlare
            | FilterKind::Lighting
            | FilterKind::Cutout
            | FilterKind::FilmGrain
            | FilterKind::NeonGlow
            | FilterKind::PosterEdges
            | FilterKind::PaintDaubs
            | FilterKind::PaletteKnife
            | FilterKind::PlasticWrap
            | FilterKind::Sponge
            | FilterKind::ColoredPencil
            | FilterKind::DryBrush
            | FilterKind::Fresco
            | FilterKind::RoughPastels
            | FilterKind::SmudgeStick
            | FilterKind::Underpainting
            | FilterKind::Watercolor
            | FilterKind::AccentedEdges
            | FilterKind::AngledStrokes
            | FilterKind::Crosshatch
            | FilterKind::DarkStrokes
            | FilterKind::InkOutlines
            | FilterKind::Spatter
            | FilterKind::SprayedStrokes
            | FilterKind::SumiE
            | FilterKind::BasRelief
            | FilterKind::ChalkCharcoal
            | FilterKind::Charcoal
            | FilterKind::Chrome
            | FilterKind::ConteCrayon
            | FilterKind::GraphicPen
            | FilterKind::HalftonePattern
            | FilterKind::NotePaper
            | FilterKind::Photocopy
            | FilterKind::Plaster
            | FilterKind::Reticulation
            | FilterKind::Stamp
            | FilterKind::TornEdges
            | FilterKind::WaterPaper
            | FilterKind::Craquelure
            | FilterKind::Grain
            | FilterKind::MosaicTiles
            | FilterKind::Patchwork
            | FilterKind::StainedGlass
            | FilterKind::Texturizer
            | FilterKind::OilPaint
            | FilterKind::DustAndScratches
            | FilterKind::Extrude
            | FilterKind::Tiles
            | FilterKind::TraceContour
            | FilterKind::Wind
            | FilterKind::SmartSharpen
            | FilterKind::HdrToning => false,
        }
    }
}

impl Filter {
    /// Whether this filter runs over the fourth (alpha) plane. The kind
    /// decides every variant; Smart Blur is the one value-dependent exception.
    pub fn moves_alpha(&self) -> bool {
        match self {
            Filter::SmartBlur { mode, .. } => *mode == SmartBlurMode::Normal,
            _ => self.kind().moves_alpha(),
        }
    }

    /// Whether the row-parallel CPU path beats the GPU round trip.
    pub fn cpu_outruns_gpu(&self) -> bool {
        self.kind().cpu_outruns_gpu()
    }

    /// A conservative document-pixel margin a preview must keep around a
    /// viewport so a neighborhood filter's result is exact across the visible
    /// area.
    pub fn preview_apron(&self) -> i32 {
        let radius = match self {
            Filter::GaussianBlur { radius } => radius.ceil(),
            Filter::UnsharpMask { radius, .. } => radius.ceil(),
            Filter::HighPass { radius } => radius.ceil(),
            // Smart Sharpen iterates a halo reaching 2.4·radius (3σ at
            // σ = 0.8·radius); later iterations reach further but weigh less.
            Filter::SmartSharpen { radius, .. } => (3.0 * radius).ceil(),
            Filter::HdrToning(p) => (3.0 * p.radius).ceil(),
            // One more pixel for the edge test against the 4-neighbours.
            Filter::SmartBlur { radius, .. } => radius.ceil() + 1.0,
            Filter::BoxBlur { radius }
            | Filter::ShapeBlur { radius, .. }
            | Filter::SurfaceBlur { radius, .. }
            | Filter::LensBlur { radius, .. }
            | Filter::Median { radius }
            | Filter::Maximum { radius }
            | Filter::Minimum { radius } => f64::from(*radius),
            Filter::MotionBlur { distance, .. } => f64::from(*distance),
            Filter::FieldBlur { pins, .. } => pins.iter().map(|pin| pin.blur).fold(0.0, f64::max),
            Filter::IrisBlur { pins, .. } => pins.iter().map(|pin| pin.blur).fold(0.0, f64::max),
            // A full distortion smears across twice the local radius.
            Filter::TiltShift { pins, .. } => pins
                .iter()
                .map(|pin| pin.blur * (1.0 + pin.distortion.abs() / 100.0))
                .fold(0.0, f64::max),
            // A trail reaches its length along the streamline.
            Filter::PathBlur { paths, motion, .. } => path_reach(paths, motion),
            // A spin's reach is its ellipse, sized as a fraction of the buffer
            // this margin cannot see; take the widest.
            Filter::SpinBlur { .. } => 512.0,
            Filter::BlurGallery { spin, .. } if !spin.is_empty() => 512.0,
            Filter::BlurGallery {
                field,
                iris,
                tilt,
                path,
                motion,
                ..
            } => field
                .iter()
                .map(|pin| pin.blur)
                .chain(iris.iter().map(|pin| pin.blur))
                .chain(
                    tilt.iter()
                        .map(|pin| pin.blur * (1.0 + pin.distortion.abs() / 100.0)),
                )
                .fold(0.0, f64::max)
                .max(path_reach(path, motion)),
            Filter::RadialBlur { amount, .. } => *amount,
            Filter::Offset {
                horizontal,
                vertical,
                ..
            } => f64::from(horizontal.abs().max(vertical.abs())),
            // Distort, render, texture, and cell filters can scatter or displace
            // widely; keep a generous margin.
            Filter::Average
            | Filter::Blur
            | Filter::BlurMore
            | Filter::Sharpen
            | Filter::SharpenMore
            | Filter::SharpenEdges
            | Filter::AddNoise { .. }
            | Filter::Despeckle
            | Filter::ReduceNoise(..)
            | Filter::Custom { .. }
            | Filter::Emboss { .. }
            | Filter::FindEdges
            | Filter::Solarize
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
            | Filter::Liquify { .. }
            | Filter::ZigZag { .. }
            | Filter::OceanRipple { .. }
            | Filter::Glass { .. }
            | Filter::DiffuseGlow { .. }
            | Filter::Clouds { .. }
            | Filter::DifferenceClouds { .. }
            | Filter::Fibers { .. }
            | Filter::PictureFrame { .. }
            | Filter::LensFlare { .. }
            | Filter::Lighting { .. }
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
            | Filter::OilPaint { .. }
            | Filter::DustAndScratches { .. }
            | Filter::Extrude { .. }
            | Filter::Tiles { .. }
            | Filter::TraceContour { .. }
            | Filter::Wind { .. } => 128.0,
        };
        (radius.ceil() as i32).clamp(8, 512)
    }
}

/// The longest trail any path's end speeds reach at the motion's Speed.
fn path_reach(paths: &[BlurPath], motion: &PathMotion) -> f64 {
    paths
        .iter()
        .map(|path| path.start_speed.max(path.end_speed) * motion.speed / 100.0)
        .fold(0.0, f64::max)
}
