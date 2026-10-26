//! The dialog's choice lists for `filter_map`: each kind's enum options, in
//! the order its dialog lists them (a choice slot's index picks from these).

use crate::{
    BlurNoiseKind, BrushType, ContourEdge, DiffuseMode, DisplaceFit, ExtrudeType, GlassTexture,
    GrainType, HalftoneType, IrisShape, LensType, LightDirection, LightType, MezzotintType,
    NoiseDistribution, PolarKind, Quality, RadialMethod, RippleSize, SharpenRemove, ShearFill,
    SmartBlurMode, SpherizeMode, StrokeDirection, TextureChannel, TileFill, WaveType, WindMethod,
    ZigZagStyle,
};

pub(crate) const BLUR_NOISE_KINDS: [BlurNoiseKind; 3] = [
    BlurNoiseKind::Gaussian,
    BlurNoiseKind::Uniform,
    BlurNoiseKind::Grain,
];

pub(crate) const NOISE_DISTRIBUTIONS: [NoiseDistribution; 2] =
    [NoiseDistribution::Uniform, NoiseDistribution::Gaussian];
pub(crate) const MEZZOTINT_TYPES: [MezzotintType; 10] = [
    MezzotintType::FineDots,
    MezzotintType::MediumDots,
    MezzotintType::GrainyDots,
    MezzotintType::CoarseDots,
    MezzotintType::ShortLines,
    MezzotintType::MediumLines,
    MezzotintType::LongLines,
    MezzotintType::ShortStrokes,
    MezzotintType::MediumStrokes,
    MezzotintType::LongStrokes,
];
pub(crate) const SPHERIZE_MODES: [SpherizeMode; 3] = [
    SpherizeMode::Normal,
    SpherizeMode::HorizontalOnly,
    SpherizeMode::VerticalOnly,
];
pub(crate) const RIPPLE_SIZES: [RippleSize; 3] =
    [RippleSize::Small, RippleSize::Medium, RippleSize::Large];
pub(crate) const WAVE_TYPES: [WaveType; 3] = [WaveType::Sine, WaveType::Triangle, WaveType::Square];
pub(crate) const POLAR_KINDS: [PolarKind; 2] =
    [PolarKind::RectangularToPolar, PolarKind::PolarToRectangular];
pub(crate) const SHEAR_FILLS: [ShearFill; 2] = [ShearFill::WrapAround, ShearFill::RepeatEdgePixels];
pub(crate) const DISPLACE_FITS: [DisplaceFit; 2] = [DisplaceFit::StretchToFit, DisplaceFit::Tile];
pub(crate) const GLASS_TEXTURES: [GlassTexture; 4] = [
    GlassTexture::Blocks,
    GlassTexture::Canvas,
    GlassTexture::Frosted,
    GlassTexture::TinyLens,
];
pub(crate) const ZIGZAG_STYLES: [ZigZagStyle; 3] = [
    ZigZagStyle::AroundCenter,
    ZigZagStyle::OutFromCenter,
    ZigZagStyle::PondRipples,
];
pub(crate) const BRUSH_TYPES: [BrushType; 6] = [
    BrushType::Simple,
    BrushType::LightRough,
    BrushType::DarkRough,
    BrushType::WideSharp,
    BrushType::WideBlurry,
    BrushType::Sparkle,
];
pub(crate) const STROKE_DIRECTIONS: [StrokeDirection; 4] = [
    StrokeDirection::RightDiagonal,
    StrokeDirection::Horizontal,
    StrokeDirection::LeftDiagonal,
    StrokeDirection::Vertical,
];
pub(crate) const LIGHT_DIRECTIONS: [LightDirection; 8] = [
    LightDirection::Bottom,
    LightDirection::BottomLeft,
    LightDirection::Left,
    LightDirection::TopLeft,
    LightDirection::Top,
    LightDirection::TopRight,
    LightDirection::Right,
    LightDirection::BottomRight,
];
pub(crate) const HALFTONE_TYPES: [HalftoneType; 3] =
    [HalftoneType::Dot, HalftoneType::Line, HalftoneType::Circle];
pub(crate) const GRAIN_TYPES: [GrainType; 10] = [
    GrainType::Regular,
    GrainType::Soft,
    GrainType::Sprinkles,
    GrainType::Clumped,
    GrainType::Contrasty,
    GrainType::Enlarged,
    GrainType::Stippled,
    GrainType::Horizontal,
    GrainType::Vertical,
    GrainType::Speckle,
];
pub(crate) const EXTRUDE_TYPES: [ExtrudeType; 2] = [ExtrudeType::Blocks, ExtrudeType::Pyramids];
pub(crate) const TILE_FILLS: [TileFill; 4] = [
    TileFill::BackgroundColor,
    TileFill::ForegroundColor,
    TileFill::InverseImage,
    TileFill::UnalteredImage,
];
pub(crate) const CONTOUR_EDGES: [ContourEdge; 2] = [ContourEdge::Lower, ContourEdge::Upper];
pub(crate) const WIND_METHODS: [WindMethod; 3] =
    [WindMethod::Wind, WindMethod::Blast, WindMethod::Stagger];
pub(crate) const WIND_FROM_RIGHT: [bool; 2] = [true, false];
pub(crate) const SHARPEN_REMOVES: [SharpenRemove; 3] = [
    SharpenRemove::GaussianBlur,
    SharpenRemove::LensBlur,
    SharpenRemove::MotionBlur,
];
pub(crate) const LENS_TYPES: [LensType; 4] = [
    LensType::Zoom,
    LensType::Prime35,
    LensType::Prime105,
    LensType::MoviePrime,
];
pub(crate) const RADIAL_METHODS: [RadialMethod; 2] = [RadialMethod::Spin, RadialMethod::Zoom];
pub(crate) const QUALITIES: [Quality; 3] = [Quality::Draft, Quality::Good, Quality::Best];
pub(crate) const SMART_BLUR_MODES: [SmartBlurMode; 3] = [
    SmartBlurMode::Normal,
    SmartBlurMode::EdgeOnly,
    SmartBlurMode::OverlayEdge,
];
pub(crate) const LIGHT_TYPES: [LightType; 3] =
    [LightType::Spot, LightType::Point, LightType::Infinite];
pub(crate) const TEXTURE_CHANNELS: [TextureChannel; 4] = [
    TextureChannel::None,
    TextureChannel::Red,
    TextureChannel::Green,
    TextureChannel::Blue,
];
pub(crate) const DIFFUSE_MODES: [DiffuseMode; 4] = [
    DiffuseMode::Normal,
    DiffuseMode::DarkenOnly,
    DiffuseMode::LightenOnly,
    DiffuseMode::Anisotropic,
];
pub(crate) const IRIS_SHAPES: [IrisShape; 6] = [
    IrisShape::Triangle,
    IrisShape::Square,
    IrisShape::Pentagon,
    IrisShape::Hexagon,
    IrisShape::Heptagon,
    IrisShape::Octagon,
];
