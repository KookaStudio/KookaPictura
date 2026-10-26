//! The filter variant table: one row per [`Filter`] variant, carrying the
//! pattern that matches it, the engine that applies it, and the catalogue row.
//! The macro generates the kind enum, the variant list, the name, the catalogue
//! lookup, and the fact queries from that single list.

use super::Filter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterEngine {
    Native,
    Ported,
}

macro_rules! filter_variants {
    ($( $variant:ident : $pattern:pat => $spec:path ),* $(,)?) => {
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

            pub const fn spec(self) -> &'static crate::catalogue::FilterSpec {
                match self {
                    $(Self::$variant => &$spec),*
                }
            }

            pub fn engine(self) -> FilterEngine {
                self.spec().engine
            }

            pub fn moves_alpha(self) -> bool {
                self.spec().alpha.is_moving_kind()
            }

            pub fn cpu_outruns_gpu(self) -> bool {
                self.spec().gpu.is_cpu_preferred()
            }
        }

        impl Filter {
            pub fn kind(&self) -> FilterKind {
                match self {
                    $($pattern => FilterKind::$variant),*
                }
            }

            pub fn spec(&self) -> &'static crate::catalogue::FilterSpec {
                self.kind().spec()
            }

            pub fn engine(&self) -> FilterEngine {
                self.kind().engine()
            }

            pub fn moves_alpha(&self) -> bool {
                self.spec().alpha.moves(self)
            }

            pub fn cpu_outruns_gpu(&self) -> bool {
                self.kind().cpu_outruns_gpu()
            }

            pub fn preview_apron(&self) -> i32 {
                self.spec().apron.resolve(self)
            }
        }
    };
}

filter_variants! {
    Liquify: Filter::Liquify { .. } => crate::catalogue::rows::paint::LIQUIFY,
    AdaptiveWideAngle: Filter::AdaptiveWideAngle { .. } => crate::catalogue::rows::paint::ADAPTIVE_WIDE_ANGLE,
    LensCorrection: Filter::LensCorrection { .. } => crate::catalogue::rows::paint::LENS_CORRECTION,
    VanishingPoint: Filter::VanishingPoint { .. } => crate::catalogue::rows::paint::VANISHING_POINT,
    OilPaint: Filter::OilPaint { .. } => crate::catalogue::rows::paint::OIL_PAINT,
    ColoredPencil: Filter::ColoredPencil { .. } => crate::catalogue::rows::paint::COLORED_PENCIL,
    Cutout: Filter::Cutout { .. } => crate::catalogue::rows::paint::CUTOUT,
    DryBrush: Filter::DryBrush { .. } => crate::catalogue::rows::paint::DRY_BRUSH,
    FilmGrain: Filter::FilmGrain { .. } => crate::catalogue::rows::paint::FILM_GRAIN,
    Fresco: Filter::Fresco { .. } => crate::catalogue::rows::paint::FRESCO,
    NeonGlow: Filter::NeonGlow { .. } => crate::catalogue::rows::paint::NEON_GLOW,
    PaintDaubs: Filter::PaintDaubs { .. } => crate::catalogue::rows::paint::PAINT_DAUBS,
    PaletteKnife: Filter::PaletteKnife { .. } => crate::catalogue::rows::paint::PALETTE_KNIFE,
    PlasticWrap: Filter::PlasticWrap { .. } => crate::catalogue::rows::paint::PLASTIC_WRAP,
    PosterEdges: Filter::PosterEdges { .. } => crate::catalogue::rows::paint::POSTER_EDGES,
    RoughPastels: Filter::RoughPastels { .. } => crate::catalogue::rows::paint::ROUGH_PASTELS,
    SmudgeStick: Filter::SmudgeStick { .. } => crate::catalogue::rows::paint::SMUDGE_STICK,
    Sponge: Filter::Sponge { .. } => crate::catalogue::rows::paint::SPONGE,
    Underpainting: Filter::Underpainting { .. } => crate::catalogue::rows::paint::UNDERPAINTING,
    Watercolor: Filter::Watercolor { .. } => crate::catalogue::rows::paint::WATERCOLOR,
    FieldBlur: Filter::FieldBlur { .. } => crate::catalogue::rows::blur::FIELD_BLUR,
    IrisBlur: Filter::IrisBlur { .. } => crate::catalogue::rows::blur::IRIS_BLUR,
    TiltShift: Filter::TiltShift { .. } => crate::catalogue::rows::blur::TILT_SHIFT,
    PathBlur: Filter::PathBlur { .. } => crate::catalogue::rows::blur::PATH_BLUR,
    SpinBlur: Filter::SpinBlur { .. } => crate::catalogue::rows::blur::SPIN_BLUR,
    GaussianBlur: Filter::GaussianBlur { .. } => crate::catalogue::rows::blur::GAUSSIAN_BLUR,
    MotionBlur: Filter::MotionBlur { .. } => crate::catalogue::rows::blur::MOTION_BLUR,
    RadialBlur: Filter::RadialBlur { .. } => crate::catalogue::rows::blur::RADIAL_BLUR,
    BoxBlur: Filter::BoxBlur { .. } => crate::catalogue::rows::blur::BOX_BLUR,
    ShapeBlur: Filter::ShapeBlur { .. } => crate::catalogue::rows::blur::SHAPE_BLUR,
    SurfaceBlur: Filter::SurfaceBlur { .. } => crate::catalogue::rows::blur::SURFACE_BLUR,
    SmartBlur: Filter::SmartBlur { .. } => crate::catalogue::rows::blur::SMART_BLUR,
    LensBlur: Filter::LensBlur { .. } => crate::catalogue::rows::blur::LENS_BLUR,
    Average: Filter::Average => crate::catalogue::rows::blur::AVERAGE,
    Blur: Filter::Blur => crate::catalogue::rows::blur::BLUR,
    BlurMore: Filter::BlurMore => crate::catalogue::rows::blur::BLUR_MORE,
    BlurGallery: Filter::BlurGallery { .. } => crate::catalogue::rows::blur::BLUR_GALLERY,
    AccentedEdges: Filter::AccentedEdges { .. } => crate::catalogue::rows::distort::ACCENTED_EDGES,
    AngledStrokes: Filter::AngledStrokes { .. } => crate::catalogue::rows::distort::ANGLED_STROKES,
    Crosshatch: Filter::Crosshatch { .. } => crate::catalogue::rows::distort::CROSSHATCH,
    DarkStrokes: Filter::DarkStrokes { .. } => crate::catalogue::rows::distort::DARK_STROKES,
    InkOutlines: Filter::InkOutlines { .. } => crate::catalogue::rows::distort::INK_OUTLINES,
    Spatter: Filter::Spatter { .. } => crate::catalogue::rows::distort::SPATTER,
    SprayedStrokes: Filter::SprayedStrokes { .. } => crate::catalogue::rows::distort::SPRAYED_STROKES,
    SumiE: Filter::SumiE { .. } => crate::catalogue::rows::distort::SUMI_E,
    Displace: Filter::Displace { .. } => crate::catalogue::rows::distort::DISPLACE,
    DiffuseGlow: Filter::DiffuseGlow { .. } => crate::catalogue::rows::distort::DIFFUSE_GLOW,
    Glass: Filter::Glass { .. } => crate::catalogue::rows::distort::GLASS,
    OceanRipple: Filter::OceanRipple { .. } => crate::catalogue::rows::distort::OCEAN_RIPPLE,
    Pinch: Filter::Pinch { .. } => crate::catalogue::rows::distort::PINCH,
    PolarCoordinates: Filter::PolarCoordinates { .. } => crate::catalogue::rows::distort::POLAR_COORDINATES,
    Ripple: Filter::Ripple { .. } => crate::catalogue::rows::distort::RIPPLE,
    Shear: Filter::Shear { .. } => crate::catalogue::rows::distort::SHEAR,
    Spherize: Filter::Spherize { .. } => crate::catalogue::rows::distort::SPHERIZE,
    Twirl: Filter::Twirl { .. } => crate::catalogue::rows::distort::TWIRL,
    Wave: Filter::Wave { .. } => crate::catalogue::rows::distort::WAVE,
    ZigZag: Filter::ZigZag { .. } => crate::catalogue::rows::distort::ZIG_ZAG,
    AddNoise: Filter::AddNoise { .. } => crate::catalogue::rows::render::ADD_NOISE,
    Despeckle: Filter::Despeckle => crate::catalogue::rows::render::DESPECKLE,
    DustAndScratches: Filter::DustAndScratches { .. } => crate::catalogue::rows::render::DUST_AND_SCRATCHES,
    Median: Filter::Median { .. } => crate::catalogue::rows::render::MEDIAN,
    ReduceNoise: Filter::ReduceNoise(..) => crate::catalogue::rows::render::REDUCE_NOISE,
    ColorHalftone: Filter::ColorHalftone { .. } => crate::catalogue::rows::render::COLOR_HALFTONE,
    Crystallize: Filter::Crystallize { .. } => crate::catalogue::rows::render::CRYSTALLIZE,
    Facet: Filter::Facet => crate::catalogue::rows::render::FACET,
    Fragment: Filter::Fragment => crate::catalogue::rows::render::FRAGMENT,
    Mezzotint: Filter::Mezzotint { .. } => crate::catalogue::rows::render::MEZZOTINT,
    Mosaic: Filter::Mosaic { .. } => crate::catalogue::rows::render::MOSAIC,
    Pointillize: Filter::Pointillize { .. } => crate::catalogue::rows::render::POINTILLIZE,
    Clouds: Filter::Clouds { .. } => crate::catalogue::rows::render::CLOUDS,
    DifferenceClouds: Filter::DifferenceClouds { .. } => crate::catalogue::rows::render::DIFFERENCE_CLOUDS,
    Fibers: Filter::Fibers { .. } => crate::catalogue::rows::render::FIBERS,
    LensFlare: Filter::LensFlare { .. } => crate::catalogue::rows::render::LENS_FLARE,
    Lighting: Filter::Lighting { .. } => crate::catalogue::rows::render::LIGHTING,
    PictureFrame: Filter::PictureFrame { .. } => crate::catalogue::rows::render::PICTURE_FRAME,
    SmartSharpen: Filter::SmartSharpen { .. } => crate::catalogue::rows::render::SMART_SHARPEN,
    UnsharpMask: Filter::UnsharpMask { .. } => crate::catalogue::rows::render::UNSHARP_MASK,
    Sharpen: Filter::Sharpen => crate::catalogue::rows::render::SHARPEN,
    SharpenEdges: Filter::SharpenEdges => crate::catalogue::rows::render::SHARPEN_EDGES,
    SharpenMore: Filter::SharpenMore => crate::catalogue::rows::render::SHARPEN_MORE,
    BasRelief: Filter::BasRelief { .. } => crate::catalogue::rows::stylize::BAS_RELIEF,
    ChalkCharcoal: Filter::ChalkCharcoal { .. } => crate::catalogue::rows::stylize::CHALK_CHARCOAL,
    Charcoal: Filter::Charcoal { .. } => crate::catalogue::rows::stylize::CHARCOAL,
    Chrome: Filter::Chrome { .. } => crate::catalogue::rows::stylize::CHROME,
    ConteCrayon: Filter::ConteCrayon { .. } => crate::catalogue::rows::stylize::CONTE_CRAYON,
    GraphicPen: Filter::GraphicPen { .. } => crate::catalogue::rows::stylize::GRAPHIC_PEN,
    HalftonePattern: Filter::HalftonePattern { .. } => crate::catalogue::rows::stylize::HALFTONE_PATTERN,
    NotePaper: Filter::NotePaper { .. } => crate::catalogue::rows::stylize::NOTE_PAPER,
    Photocopy: Filter::Photocopy { .. } => crate::catalogue::rows::stylize::PHOTOCOPY,
    Plaster: Filter::Plaster { .. } => crate::catalogue::rows::stylize::PLASTER,
    Reticulation: Filter::Reticulation { .. } => crate::catalogue::rows::stylize::RETICULATION,
    Stamp: Filter::Stamp { .. } => crate::catalogue::rows::stylize::STAMP,
    TornEdges: Filter::TornEdges { .. } => crate::catalogue::rows::stylize::TORN_EDGES,
    WaterPaper: Filter::WaterPaper { .. } => crate::catalogue::rows::stylize::WATER_PAPER,
    Emboss: Filter::Emboss { .. } => crate::catalogue::rows::stylize::EMBOSS,
    Extrude: Filter::Extrude { .. } => crate::catalogue::rows::stylize::EXTRUDE,
    FindEdges: Filter::FindEdges => crate::catalogue::rows::stylize::FIND_EDGES,
    Solarize: Filter::Solarize => crate::catalogue::rows::stylize::SOLARIZE,
    Diffuse: Filter::Diffuse { .. } => crate::catalogue::rows::stylize::DIFFUSE,
    GlowingEdges: Filter::GlowingEdges { .. } => crate::catalogue::rows::stylize::GLOWING_EDGES,
    Tiles: Filter::Tiles { .. } => crate::catalogue::rows::stylize::TILES,
    TraceContour: Filter::TraceContour { .. } => crate::catalogue::rows::stylize::TRACE_CONTOUR,
    Wind: Filter::Wind { .. } => crate::catalogue::rows::stylize::WIND,
    Craquelure: Filter::Craquelure { .. } => crate::catalogue::rows::stylize::CRAQUELURE,
    Grain: Filter::Grain { .. } => crate::catalogue::rows::stylize::GRAIN,
    MosaicTiles: Filter::MosaicTiles { .. } => crate::catalogue::rows::stylize::MOSAIC_TILES,
    Patchwork: Filter::Patchwork { .. } => crate::catalogue::rows::stylize::PATCHWORK,
    StainedGlass: Filter::StainedGlass { .. } => crate::catalogue::rows::stylize::STAINED_GLASS,
    Texturizer: Filter::Texturizer { .. } => crate::catalogue::rows::stylize::TEXTURIZER,
    Custom: Filter::Custom { .. } => crate::catalogue::rows::other::CUSTOM,
    HighPass: Filter::HighPass { .. } => crate::catalogue::rows::other::HIGH_PASS,
    Maximum: Filter::Maximum { .. } => crate::catalogue::rows::other::MAXIMUM,
    Minimum: Filter::Minimum { .. } => crate::catalogue::rows::other::MINIMUM,
    Offset: Filter::Offset { .. } => crate::catalogue::rows::other::OFFSET,
    NtscColors: Filter::NtscColors => crate::catalogue::rows::video::NTSC_COLORS,
    HdrToning: Filter::HdrToning(..) => crate::catalogue::rows::engine_only::HDR_TONING,
}
