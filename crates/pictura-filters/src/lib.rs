//! Filter operations for Kooka Pictura.
//!
//! M6 scope: the destructive `Filter > Blur / Sharpen / Noise` math as pure
//! functions over a planar 8-bit [`PixelBuffer`] (channels 3 or 4; alpha is
//! never modified). Specs live in `docs/06-filters/`.
//!
//! The closed kernels are approximated where the specs say so; each
//! approximation is marked inline. Everything is deterministic except Add
//! Noise, which takes a seed. Bad input returns [`FilterError`] instead of
//! panicking.

use pictura_core::PixelBuffer;

pub mod blur;
mod distort;
pub mod hdr_toning;
pub mod kernel;
mod luma;
mod noise;
pub mod oil_paint;
mod other;
mod photorust;
pub mod render;
pub mod sharpen;
mod stylize;

pub use blur::field::{
    BlurNoise, BlurNoiseKind, BlurPin, BokehParams, MAX_FIELD_PINS, MAX_FIELD_RADIUS,
};
pub use blur::lens::{DepthMap, IrisShape, MAX_LENS_RADIUS};
pub use blur::SmartBlurMode;
pub use hdr_toning::HdrToningParams;
pub use render::{LensType, Light, LightType, Lighting, TextureChannel, MAX_LIGHTS};
pub use stylize::DiffuseMode;

#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("invalid parameters: {0}")]
    InvalidParams(String),
    #[error("layer is pixel-locked")]
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadialMethod {
    Spin,
    Zoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Draft,
    Good,
    Best,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseDistribution {
    Uniform,
    Gaussian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MezzotintType {
    FineDots,
    MediumDots,
    GrainyDots,
    CoarseDots,
    ShortLines,
    MediumLines,
    LongLines,
    ShortStrokes,
    MediumStrokes,
    LongStrokes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpherizeMode {
    Normal,
    HorizontalOnly,
    VerticalOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RippleSize {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveType {
    Sine,
    Triangle,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolarKind {
    RectangularToPolar,
    PolarToRectangular,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShearFill {
    WrapAround,
    RepeatEdgePixels,
}

/// The most control points a Shear curve carries. The editor keeps a handful
/// of points, and the filter is a value that can be copied, compared, and
/// replayed from history, so the list is capped rather than open-ended.
pub const SHEAR_MAX_POINTS: usize = 8;

/// How Displace lays its map over the layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplaceFit {
    StretchToFit,
    Tile,
}

/// Displace's map: the flattened map file's first two channels, one byte per
/// pixel each. A one-channel (Grayscale) file carries the same plane twice, so
/// it shifts along the diagonal the two scales define.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplaceMap {
    width: u32,
    height: u32,
    horizontal: Vec<u8>,
    vertical: Vec<u8>,
}

impl DisplaceMap {
    /// `None` unless both planes hold `width * height` samples and the map is
    /// not empty.
    pub fn new(width: u32, height: u32, horizontal: Vec<u8>, vertical: Vec<u8>) -> Option<Self> {
        let n = width as usize * height as usize;
        (n > 0 && horizontal.len() == n && vertical.len() == n).then_some(Self {
            width,
            height,
            horizontal,
            vertical,
        })
    }

    /// A one-pixel mid-grey map, which moves nothing.
    pub fn neutral() -> Self {
        Self {
            width: 1,
            height: 1,
            horizontal: vec![128],
            vertical: vec![128],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// The `(horizontal, vertical)` samples at `(x, y)`, which must be inside.
    fn at(&self, x: u32, y: u32) -> (u8, u8) {
        let i = y as usize * self.width as usize + x as usize;
        (self.horizontal[i], self.vertical[i])
    }
}

/// Glass's built-in surfaces, in CS6's menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassTexture {
    Blocks,
    Canvas,
    Frosted,
    TinyLens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZigZagStyle {
    AroundCenter,
    OutFromCenter,
    PondRipples,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushType {
    Simple,
    LightRough,
    DarkRough,
    WideSharp,
    WideBlurry,
    Sparkle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureSurface {
    Brick,
    Burlap,
    Canvas,
    Sandstone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureOptions {
    pub surface: TextureSurface,
    pub scaling: u8,
    pub relief: u8,
    pub light_direction: u8,
    pub invert: bool,
}

impl Default for TextureOptions {
    fn default() -> Self {
        Self {
            surface: TextureSurface::Canvas,
            scaling: 100,
            relief: 4,
            light_direction: 0,
            invert: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeDirection {
    RightDiagonal,
    Horizontal,
    LeftDiagonal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightDirection {
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalftoneType {
    Dot,
    Line,
    Circle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrainType {
    Regular,
    Soft,
    Sprinkles,
    Clumped,
    Contrasty,
    Enlarged,
    Stippled,
    Horizontal,
    Vertical,
    Speckle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtrudeType {
    Blocks,
    Pyramids,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileFill {
    BackgroundColor,
    ForegroundColor,
    InverseImage,
    UnalteredImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourEdge {
    Lower,
    Upper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindMethod {
    Wind,
    Blast,
    Stagger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharpenRemove {
    GaussianBlur,
    LensBlur,
    MotionBlur,
}

/// Smart Sharpen Shadow/Highlight tab controls: how strongly sharpening is
/// damped in dark or light tones, over what tonal `width`, using which blur
/// `radius`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TonalFade {
    pub amount: u8,
    pub width: u8,
    pub radius: u32,
}

impl Default for TonalFade {
    fn default() -> Self {
        Self {
            amount: 0,
            width: 50,
            radius: 1,
        }
    }
}

mod filter;
pub use filter::{apply, Filter};
pub(crate) fn validate(buf: &PixelBuffer) -> Result<usize, FilterError> {
    if buf.channels != 3 && buf.channels != 4 {
        return Err(FilterError::Unsupported(format!(
            "channel count {} is not supported (expected 3 or 4)",
            buf.channels
        )));
    }
    let n = buf.pixel_count();
    if n == 0 {
        return Err(FilterError::InvalidParams("empty buffer".into()));
    }
    if buf.data.len() != n * buf.channels as usize {
        return Err(FilterError::InvalidParams(
            "buffer length does not match width*height*channels".into(),
        ));
    }
    Ok(n)
}
