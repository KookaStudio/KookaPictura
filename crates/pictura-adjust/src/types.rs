use crate::match_color::MatchColorParams;
use crate::replace_color::ReplaceColorParams;

#[derive(Debug, thiserror::Error)]
pub enum AdjustError {
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("invalid parameters: {0}")]
    InvalidParams(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelsParams {
    pub input_black: u8,
    pub input_white: u8,
    pub gamma: f64,
    pub output_black: u8,
    pub output_white: u8,
    /// Optional per-channel records (`levl` records 1–3). Applied to their own
    /// plane before the composite record, as Curves' channel curves are.
    pub red: Option<LevelsChannel>,
    pub green: Option<LevelsChannel>,
    pub blue: Option<LevelsChannel>,
}

/// One channel's Levels record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelsChannel {
    pub input_black: u8,
    pub input_white: u8,
    pub gamma: f64,
    pub output_black: u8,
    pub output_white: u8,
}

impl LevelsParams {
    /// The composite record, in the per-channel shape.
    pub fn composite(&self) -> LevelsChannel {
        LevelsChannel {
            input_black: self.input_black,
            input_white: self.input_white,
            gamma: self.gamma,
            output_black: self.output_black,
            output_white: self.output_white,
        }
    }
}

impl LevelsChannel {
    /// Reject a non-positive gamma or an input black at or past white.
    pub(crate) fn validate(&self) -> Result<(), AdjustError> {
        if !self.gamma.is_finite() || self.gamma <= 0.0 {
            return Err(AdjustError::InvalidParams(
                "levels gamma must be > 0".into(),
            ));
        }
        if self.input_black >= self.input_white {
            return Err(AdjustError::InvalidParams(
                "levels input black must be below input white".into(),
            ));
        }
        Ok(())
    }

    /// The record over `0.0..=255.0`.
    fn eval(&self, x: f64) -> f64 {
        let (ib, iw) = (self.input_black as f64, self.input_white as f64);
        let (ob, ow) = (self.output_black as f64, self.output_white as f64);
        let t = ((x - ib) / (iw - ib))
            .clamp(0.0, 1.0)
            .powf(1.0 / self.gamma);
        ob + t * (ow - ob)
    }

    /// The record as a map over `0.0..=1.0`.
    pub(crate) fn map(&self) -> Result<impl Fn(f64) -> f64, AdjustError> {
        self.validate()?;
        let record = *self;
        Ok(move |u: f64| record.eval(u * 255.0) / 255.0)
    }

    /// The record as an 8-bit lookup table.
    pub(crate) fn lut(&self) -> Result<[u8; 256], AdjustError> {
        self.validate()?;
        let mut lut = [0u8; 256];
        for (i, slot) in lut.iter_mut().enumerate() {
            *slot = self.eval(i as f64).round().clamp(0.0, 255.0) as u8;
        }
        Ok(lut)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurvesParams {
    /// Monotone composite control points in `(input, output)` order, inclusive
    /// of endpoints.
    pub points: Vec<(u8, u8)>,
    /// Optional per-channel curves, each in the same `(input, output)` order as
    /// `points`. Applied to their own plane before the composite `points` curve.
    pub red: Option<Vec<(u8, u8)>>,
    pub green: Option<Vec<(u8, u8)>>,
    pub blue: Option<Vec<(u8, u8)>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrightnessContrastParams {
    pub brightness: i16,
    pub contrast: i16,
    pub use_legacy: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExposureParams {
    /// Exposure in stops (EV).
    pub exposure: f64,
    pub offset: f64,
    pub gamma: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HueSaturationParams {
    pub hue: i16,
    pub saturation: i16,
    pub lightness: i16,
    /// The colour ranges (Reds … Magentas) that carry an edit; empty when only
    /// Master is set.
    pub ranges: Vec<HueRange>,
}

/// One Hue/Saturation colour range: its hue band in degrees (full effect
/// between the sustain points, ramping to none at the ramp points; a band may
/// wrap through 0) and the Hue / Saturation / Lightness applied within it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HueRange {
    pub begin_ramp: i16,
    pub begin_sustain: i16,
    pub end_sustain: i16,
    pub end_ramp: i16,
    pub hue: i16,
    pub saturation: i16,
    pub lightness: i16,
}

impl HueRange {
    /// How strongly the range applies to hue `h` (0..360): 1 inside the
    /// sustain band, linear across each ramp, 0 outside.
    pub fn weight(&self, h: f64) -> f64 {
        let from = f64::from(self.begin_ramp);
        let rel = |v: i16| (f64::from(v) - from).rem_euclid(360.0);
        let (h, b, c, d) = (
            (h - from).rem_euclid(360.0),
            rel(self.begin_sustain),
            rel(self.end_sustain),
            rel(self.end_ramp),
        );
        if h < b {
            h / b
        } else if h <= c {
            1.0
        } else if h < d {
            (d - h) / (d - c)
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlackWhiteParams {
    /// Contributions in percent, matching the CS6 -200…+300 sliders.
    pub red: f64,
    pub yellow: f64,
    pub green: f64,
    pub cyan: f64,
    pub blue: f64,
    pub magenta: f64,
    pub tint: bool,
    /// Tint tone when `tint` is set (spec's Tint colour swatch).
    pub tint_color: [u8; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhotoFilterParams {
    pub color: [u8; 3],
    pub density: f64,
    pub preserve_luminosity: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelMixerParams {
    pub monochrome: bool,
    /// Output-channel mixes in percent, as `[red, green, blue]` sources.
    pub red: [f64; 3],
    pub green: [f64; 3],
    pub blue: [f64; 3],
    /// Per-output-channel constant in percent.
    pub constant: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct VibranceParams {
    pub vibrance: i16,
    pub saturation: i16,
}

/// Shadows/Highlights (`shdH`): the two always-visible amount sliders in
/// `0..=100`. A pointwise approximation of CS6's local operator (see
/// `docs/04-image-ops/adjustments/shadow-highlight.md`); the surround blur and
/// advanced options are not modelled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowsHighlightsParams {
    pub shadows_amount: f64,
    pub highlights_amount: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorBalanceParams {
    /// Per-band `[cyan-red, magenta-green, yellow-blue]` shifts, -100…+100.
    pub shadows: [f64; 3],
    pub midtones: [f64; 3],
    pub highlights: [f64; 3],
    pub preserve_luminosity: bool,
}

/// How a Selective Color correction scales: relative scales by the source ink,
/// absolute by full scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectiveColorMethod {
    Relative,
    Absolute,
}

/// One CMYK correction pair of a Selective Color family, each `-100..=100`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectiveRange {
    pub c: i16,
    pub m: i16,
    pub y: i16,
    pub k: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectiveColorParams {
    pub method: SelectiveColorMethod,
    /// Nine ranges in the reference's order: reds, yellows, greens, cyans, blues,
    /// magentas, whites, neutrals, blacks.
    pub ranges: [SelectiveRange; 9],
}

impl Default for SelectiveColorParams {
    fn default() -> Self {
        Self {
            method: SelectiveColorMethod::Relative,
            ranges: [SelectiveRange::default(); 9],
        }
    }
}

/// One colour stop of a [`GradientMapParams`] gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradientStop {
    /// PSD gradient position, `0..=4096`.
    pub location: u16,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientMapParams {
    pub stops: Vec<GradientStop>,
    pub reverse: bool,
    /// Opacity stops; empty is opaque. Where the gradient is part transparent
    /// the mapped colour is blended over the original pixel.
    pub transparency: Vec<OpacityStop>,
}

/// One opacity stop of a [`GradientMapParams`] gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpacityStop {
    /// PSD gradient position, `0..=4096`.
    pub location: u16,
    /// Percent, `0..=100`.
    pub opacity: u8,
}

/// The geometry of a gradient fill (`GdFl`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientKind {
    Linear,
    Radial,
    Angle,
    Reflected,
    Diamond,
}

/// A gradient fill (`GdFl`): the stops plus the geometry parameters psd-tools
/// reads from the descriptor. Like [`GradientMapParams`] it reuses
/// [`GradientStop`]; unlike a gradient map it is composited over the layer rect
/// rather than applied to the backdrop.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientFillParams {
    pub stops: Vec<GradientStop>,
    pub reverse: bool,
    pub kind: GradientKind,
    pub angle_deg: f32,
    pub scale: f32,
}

/// A pattern fill (`PtFl`): the id of a pattern in the document's pattern
/// library plus the tiling controls. Like [`GradientFillParams`] it is
/// composited over the layer rect rather than applied to the backdrop.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternFillParams {
    pub pattern_id: String,
    /// Tile scale in percent (100 = one pattern pixel per layer pixel).
    pub scale: f32,
    /// Anchor the tile at the layer's top-left when set, else at the document
    /// origin.
    pub link_with_layer: bool,
    /// Pixel offset added to the tiled sample.
    pub origin: (i32, i32),
}

/// The `lookupType` of a Color Lookup (`clrL`) adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorLookupKind {
    /// A 3-D LUT (the only kind this engine renders).
    ThreeDLut,
    AbstractProfile,
    DeviceLinkProfile,
}

/// A parsed 3-D lookup table: `size` grid steps per axis and `size³` RGB points
/// in `.CUBE` order (red index varies fastest, then green, then blue), each
/// component in `0.0..=1.0`.
#[derive(Debug, Clone, PartialEq)]
pub struct Lut3d {
    pub size: usize,
    pub points: Vec<[f32; 3]>,
}

/// A Color Lookup (`clrL`) adjustment. `lookup` is the parsed embedded `.CUBE`
/// for a 3-D LUT; it is `None` for an abstract-profile or device-link kind, a
/// non-`.CUBE` embedded format, or malformed data, and such a layer is a no-op.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorLookupParams {
    pub kind: ColorLookupKind,
    pub lookup: Option<Lut3d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoKind {
    Tone,
    Contrast,
    Color,
}

/// A single destructive adjustment.
#[derive(Debug, Clone, PartialEq)]
pub enum Adjustment {
    Levels(LevelsParams),
    Curves(CurvesParams),
    BrightnessContrast(BrightnessContrastParams),
    Exposure(ExposureParams),
    HueSaturation(HueSaturationParams),
    BlackWhite(BlackWhiteParams),
    PhotoFilter(PhotoFilterParams),
    ChannelMixer(ChannelMixerParams),
    Vibrance(VibranceParams),
    /// Image > Adjustments > Shadows/Highlights: lifts shadows and pulls
    /// highlights by amount, without a local surround.
    ShadowsHighlights(ShadowsHighlightsParams),
    ColorBalance(ColorBalanceParams),
    SelectiveColor(SelectiveColorParams),
    Auto(AutoKind),
    Invert,
    Posterize(u8),
    Threshold(u8),
    Desaturate,
    /// Image > Adjustments > Equalize: spread the combined RGB histogram
    /// evenly over 0..=255 (behavioural approximation; CS6's is unpublished).
    Equalize,
    GradientMap(GradientMapParams),
    /// Gradient fill content (`GdFl`): a generative fill composited over the
    /// layer rect (see `pictura-render`'s `composite_gradient_fill`), refused
    /// by [`apply`] like [`Adjustment::SolidFill`].
    GradientFill(GradientFillParams),
    /// Solid-color fill content (`SoCo`): straight-alpha RGBA. This is not a
    /// destructive adjustment; the renderer composites it generatively (see
    /// `pictura-render`'s `composite_adjustment`), and [`apply`] refuses it.
    SolidFill([u8; 4]),
    /// Pattern fill content (`PtFl`): a generative fill composited over the
    /// layer rect (see `pictura-render`'s `composite_pattern_fill`), refused by
    /// [`apply`] like [`Adjustment::SolidFill`].
    PatternFill(PatternFillParams),
    /// Color Lookup (`clrL`): samples a parsed 3-D lookup, or is a no-op when
    /// [`ColorLookupParams::lookup`] is `None`.
    ColorLookup(ColorLookupParams),
    /// Image > Adjustments > Replace Color (#164): an HSL shift feathered by
    /// each pixel's colour match to the sampled colours. Dialog-only, so it
    /// has no PSD encoding; native-depth apply refuses it.
    ReplaceColor(ReplaceColorParams),
    /// Image > Adjustments > Match Color (#352): a Lab mean/std transfer from
    /// a source image's statistics, with neutralize / luminance / colour
    /// intensity / fade modifiers. Dialog-only, so it has no PSD encoding;
    /// native-depth apply refuses it.
    MatchColor(MatchColorParams),
}
