//! The filter variant table: one row per [`Filter`] variant, carrying the
//! pattern that matches it and the engine that applies it. The macro generates
//! the kind enum, the variant list, the name and the engine property from that
//! single list.

use super::Filter;

macro_rules! filter_variants {
    ($( $variant:ident : $pattern:pat => $engine:ident ),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum FilterKind {
            $($variant),*
        }

        impl FilterKind {
            pub const ALL: &[Self] = &[$(Self::$variant),*];

            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant)),*
                }
            }

            pub const fn engine(self) -> FilterEngine {
                match self {
                    $(Self::$variant => FilterEngine::$engine),*
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum FilterEngine {
            Native,
            Ported,
        }

        impl Filter {
            pub fn kind(&self) -> FilterKind {
                match self {
                    $($pattern => FilterKind::$variant),*
                }
            }

            pub fn engine(&self) -> FilterEngine {
                self.kind().engine()
            }
        }
    };
}

filter_variants! {
    GaussianBlur: Filter::GaussianBlur { .. } => Native,
    BoxBlur: Filter::BoxBlur { .. } => Native,
    ShapeBlur: Filter::ShapeBlur { .. } => Native,
    MotionBlur: Filter::MotionBlur { .. } => Native,
    RadialBlur: Filter::RadialBlur { .. } => Native,
    Average: Filter::Average => Ported,
    Blur: Filter::Blur => Native,
    BlurMore: Filter::BlurMore => Native,
    SurfaceBlur: Filter::SurfaceBlur { .. } => Ported,
    SmartBlur: Filter::SmartBlur { .. } => Native,
    LensBlur: Filter::LensBlur { .. } => Native,
    FieldBlur: Filter::FieldBlur { .. } => Native,
    IrisBlur: Filter::IrisBlur { .. } => Native,
    TiltShift: Filter::TiltShift { .. } => Native,
    PathBlur: Filter::PathBlur { .. } => Native,
    SpinBlur: Filter::SpinBlur { .. } => Native,
    BlurGallery: Filter::BlurGallery { .. } => Native,
    Sharpen: Filter::Sharpen => Native,
    SharpenMore: Filter::SharpenMore => Native,
    SharpenEdges: Filter::SharpenEdges => Ported,
    UnsharpMask: Filter::UnsharpMask { .. } => Native,
    AddNoise: Filter::AddNoise { .. } => Ported,
    Median: Filter::Median { .. } => Native,
    Despeckle: Filter::Despeckle => Native,
    ReduceNoise: Filter::ReduceNoise(..) => Native,
    Maximum: Filter::Maximum { .. } => Native,
    Minimum: Filter::Minimum { .. } => Native,
    Offset: Filter::Offset { .. } => Native,
    HighPass: Filter::HighPass { .. } => Native,
    Custom: Filter::Custom { .. } => Native,
    Emboss: Filter::Emboss { .. } => Ported,
    FindEdges: Filter::FindEdges => Ported,
    Solarize: Filter::Solarize => Native,
    Diffuse: Filter::Diffuse { .. } => Ported,
    GlowingEdges: Filter::GlowingEdges { .. } => Ported,
    Mosaic: Filter::Mosaic { .. } => Ported,
    Crystallize: Filter::Crystallize { .. } => Ported,
    Facet: Filter::Facet => Ported,
    Fragment: Filter::Fragment => Ported,
    Mezzotint: Filter::Mezzotint { .. } => Ported,
    Pointillize: Filter::Pointillize { .. } => Ported,
    ColorHalftone: Filter::ColorHalftone { .. } => Ported,
    Twirl: Filter::Twirl { .. } => Ported,
    Pinch: Filter::Pinch { .. } => Ported,
    Spherize: Filter::Spherize { .. } => Ported,
    Ripple: Filter::Ripple { .. } => Ported,
    Wave: Filter::Wave { .. } => Ported,
    PolarCoordinates: Filter::PolarCoordinates { .. } => Ported,
    Shear: Filter::Shear { .. } => Ported,
    Displace: Filter::Displace { .. } => Ported,
    Liquify: Filter::Liquify { .. } => Native,
    AdaptiveWideAngle: Filter::AdaptiveWideAngle { .. } => Native,
    ZigZag: Filter::ZigZag { .. } => Ported,
    OceanRipple: Filter::OceanRipple { .. } => Native,
    Glass: Filter::Glass { .. } => Native,
    DiffuseGlow: Filter::DiffuseGlow { .. } => Native,
    Clouds: Filter::Clouds { .. } => Native,
    DifferenceClouds: Filter::DifferenceClouds { .. } => Native,
    Fibers: Filter::Fibers { .. } => Native,
    PictureFrame: Filter::PictureFrame { .. } => Ported,
    LensFlare: Filter::LensFlare { .. } => Native,
    Lighting: Filter::Lighting { .. } => Native,
    Cutout: Filter::Cutout { .. } => Ported,
    FilmGrain: Filter::FilmGrain { .. } => Ported,
    NeonGlow: Filter::NeonGlow { .. } => Ported,
    PosterEdges: Filter::PosterEdges { .. } => Ported,
    PaintDaubs: Filter::PaintDaubs { .. } => Ported,
    PaletteKnife: Filter::PaletteKnife { .. } => Ported,
    PlasticWrap: Filter::PlasticWrap { .. } => Ported,
    Sponge: Filter::Sponge { .. } => Ported,
    ColoredPencil: Filter::ColoredPencil { .. } => Ported,
    DryBrush: Filter::DryBrush { .. } => Ported,
    Fresco: Filter::Fresco { .. } => Ported,
    RoughPastels: Filter::RoughPastels { .. } => Ported,
    SmudgeStick: Filter::SmudgeStick { .. } => Ported,
    Underpainting: Filter::Underpainting { .. } => Ported,
    Watercolor: Filter::Watercolor { .. } => Ported,
    AccentedEdges: Filter::AccentedEdges { .. } => Ported,
    AngledStrokes: Filter::AngledStrokes { .. } => Ported,
    Crosshatch: Filter::Crosshatch { .. } => Ported,
    DarkStrokes: Filter::DarkStrokes { .. } => Ported,
    InkOutlines: Filter::InkOutlines { .. } => Ported,
    Spatter: Filter::Spatter { .. } => Ported,
    SprayedStrokes: Filter::SprayedStrokes { .. } => Ported,
    SumiE: Filter::SumiE { .. } => Ported,
    BasRelief: Filter::BasRelief { .. } => Ported,
    ChalkCharcoal: Filter::ChalkCharcoal { .. } => Ported,
    Charcoal: Filter::Charcoal { .. } => Ported,
    Chrome: Filter::Chrome { .. } => Ported,
    ConteCrayon: Filter::ConteCrayon { .. } => Ported,
    GraphicPen: Filter::GraphicPen { .. } => Ported,
    HalftonePattern: Filter::HalftonePattern { .. } => Ported,
    NotePaper: Filter::NotePaper { .. } => Ported,
    Photocopy: Filter::Photocopy { .. } => Ported,
    Plaster: Filter::Plaster { .. } => Ported,
    Reticulation: Filter::Reticulation { .. } => Ported,
    Stamp: Filter::Stamp { .. } => Ported,
    TornEdges: Filter::TornEdges { .. } => Ported,
    WaterPaper: Filter::WaterPaper { .. } => Ported,
    Craquelure: Filter::Craquelure { .. } => Ported,
    Grain: Filter::Grain { .. } => Ported,
    MosaicTiles: Filter::MosaicTiles { .. } => Ported,
    Patchwork: Filter::Patchwork { .. } => Ported,
    StainedGlass: Filter::StainedGlass { .. } => Ported,
    Texturizer: Filter::Texturizer { .. } => Ported,
    OilPaint: Filter::OilPaint { .. } => Native,
    DustAndScratches: Filter::DustAndScratches { .. } => Ported,
    Extrude: Filter::Extrude { .. } => Ported,
    Tiles: Filter::Tiles { .. } => Ported,
    TraceContour: Filter::TraceContour { .. } => Ported,
    Wind: Filter::Wind { .. } => Ported,
    SmartSharpen: Filter::SmartSharpen { .. } => Native,
    HdrToning: Filter::HdrToning(..) => Native,
}
