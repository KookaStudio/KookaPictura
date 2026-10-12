//! The dialog's choice lists for `filter_map`: each kind's enum options, in
//! the order its dialog lists them (a choice slot's index picks from these).

use pictura_filters::{
    BrushType, ContourEdge, DiffuseMode, DisplaceFit, ExtrudeType, GlassTexture, GrainType,
    HalftoneType, IrisShape, LensType, LightDirection, LightType, MezzotintType, NoiseDistribution,
    PolarKind, Quality, RadialMethod, RippleSize, SharpenRemove, ShearFill, SmartBlurMode,
    SpherizeMode, StrokeDirection, TextureChannel, TileFill, WaveType, WindMethod, ZigZagStyle,
};

pub(super) const NOISE_DISTRIBUTIONS: [NoiseDistribution; 2] =
    [NoiseDistribution::Uniform, NoiseDistribution::Gaussian];
pub(super) const MEZZOTINT_TYPES: [MezzotintType; 10] = [
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
pub(super) const SPHERIZE_MODES: [SpherizeMode; 3] = [
    SpherizeMode::Normal,
    SpherizeMode::HorizontalOnly,
    SpherizeMode::VerticalOnly,
];
pub(super) const RIPPLE_SIZES: [RippleSize; 3] =
    [RippleSize::Small, RippleSize::Medium, RippleSize::Large];
pub(super) const WAVE_TYPES: [WaveType; 3] = [WaveType::Sine, WaveType::Triangle, WaveType::Square];
pub(super) const POLAR_KINDS: [PolarKind; 2] =
    [PolarKind::RectangularToPolar, PolarKind::PolarToRectangular];
pub(super) const SHEAR_FILLS: [ShearFill; 2] = [ShearFill::WrapAround, ShearFill::RepeatEdgePixels];
pub(super) const DISPLACE_FITS: [DisplaceFit; 2] = [DisplaceFit::StretchToFit, DisplaceFit::Tile];
pub(super) const GLASS_TEXTURES: [GlassTexture; 4] = [
    GlassTexture::Blocks,
    GlassTexture::Canvas,
    GlassTexture::Frosted,
    GlassTexture::TinyLens,
];
pub(super) const ZIGZAG_STYLES: [ZigZagStyle; 3] = [
    ZigZagStyle::AroundCenter,
    ZigZagStyle::OutFromCenter,
    ZigZagStyle::PondRipples,
];
pub(super) const BRUSH_TYPES: [BrushType; 6] = [
    BrushType::Simple,
    BrushType::LightRough,
    BrushType::DarkRough,
    BrushType::WideSharp,
    BrushType::WideBlurry,
    BrushType::Sparkle,
];
pub(super) const STROKE_DIRECTIONS: [StrokeDirection; 4] = [
    StrokeDirection::RightDiagonal,
    StrokeDirection::Horizontal,
    StrokeDirection::LeftDiagonal,
    StrokeDirection::Vertical,
];
pub(super) const LIGHT_DIRECTIONS: [LightDirection; 8] = [
    LightDirection::Bottom,
    LightDirection::BottomLeft,
    LightDirection::Left,
    LightDirection::TopLeft,
    LightDirection::Top,
    LightDirection::TopRight,
    LightDirection::Right,
    LightDirection::BottomRight,
];
pub(super) const HALFTONE_TYPES: [HalftoneType; 3] =
    [HalftoneType::Dot, HalftoneType::Line, HalftoneType::Circle];
pub(super) const GRAIN_TYPES: [GrainType; 10] = [
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
pub(super) const EXTRUDE_TYPES: [ExtrudeType; 2] = [ExtrudeType::Blocks, ExtrudeType::Pyramids];
pub(super) const TILE_FILLS: [TileFill; 4] = [
    TileFill::BackgroundColor,
    TileFill::ForegroundColor,
    TileFill::InverseImage,
    TileFill::UnalteredImage,
];
pub(super) const CONTOUR_EDGES: [ContourEdge; 2] = [ContourEdge::Lower, ContourEdge::Upper];
pub(super) const WIND_METHODS: [WindMethod; 3] =
    [WindMethod::Wind, WindMethod::Blast, WindMethod::Stagger];
pub(super) const WIND_FROM_RIGHT: [bool; 2] = [true, false];
pub(super) const SHARPEN_REMOVES: [SharpenRemove; 3] = [
    SharpenRemove::GaussianBlur,
    SharpenRemove::LensBlur,
    SharpenRemove::MotionBlur,
];
pub(super) const LENS_TYPES: [LensType; 4] = [
    LensType::Zoom,
    LensType::Prime35,
    LensType::Prime105,
    LensType::MoviePrime,
];
pub(super) const RADIAL_METHODS: [RadialMethod; 2] = [RadialMethod::Spin, RadialMethod::Zoom];
pub(super) const QUALITIES: [Quality; 3] = [Quality::Draft, Quality::Good, Quality::Best];
pub(super) const SMART_BLUR_MODES: [SmartBlurMode; 3] = [
    SmartBlurMode::Normal,
    SmartBlurMode::EdgeOnly,
    SmartBlurMode::OverlayEdge,
];
pub(super) const LIGHT_TYPES: [LightType; 3] =
    [LightType::Spot, LightType::Point, LightType::Infinite];
pub(super) const TEXTURE_CHANNELS: [TextureChannel; 4] = [
    TextureChannel::None,
    TextureChannel::Red,
    TextureChannel::Green,
    TextureChannel::Blue,
];
pub(super) const DIFFUSE_MODES: [DiffuseMode; 4] = [
    DiffuseMode::Normal,
    DiffuseMode::DarkenOnly,
    DiffuseMode::LightenOnly,
    DiffuseMode::Anisotropic,
];
pub(super) const IRIS_SHAPES: [IrisShape; 6] = [
    IrisShape::Triangle,
    IrisShape::Square,
    IrisShape::Pentagon,
    IrisShape::Hexagon,
    IrisShape::Heptagon,
    IrisShape::Octagon,
];
