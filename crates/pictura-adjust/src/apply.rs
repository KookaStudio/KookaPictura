use pictura_core::PixelBuffer;

use crate::auto::auto;
use crate::color::{
    black_white, channel_mixer, color_balance, hue_saturation, photo_filter, selective_color,
    vibrance,
};
use crate::common::{map_float, validate};
use crate::match_color::match_color;
use crate::replace_color::replace_color;
use crate::tonal::{
    brightness_contrast, curves, desaturate, equalize, exposure, gradient_map, levels, posterize,
    shadows_highlights, threshold,
};
use crate::types::{AdjustError, Adjustment};

/// Apply `adjustment` in place to a planar 8-bit buffer (alpha untouched).
pub fn apply(adjustment: &Adjustment, buf: &mut PixelBuffer) -> Result<(), AdjustError> {
    apply_with_coverage(adjustment, buf, None)
}

/// As [`apply`], with an optional one-byte-per-pixel selection coverage used by
/// the Auto histograms. Other adjustments ignore it.
pub fn apply_with_coverage(
    adjustment: &Adjustment,
    buf: &mut PixelBuffer,
    coverage: Option<&[u8]>,
) -> Result<(), AdjustError> {
    let n = validate(buf)?;
    if coverage.is_some_and(|coverage| coverage.len() < n) {
        return Err(AdjustError::InvalidParams(
            "selection coverage is shorter than the buffer".into(),
        ));
    }
    match adjustment {
        Adjustment::Levels(p) => levels(p, buf, n),
        Adjustment::Curves(p) => curves(p, buf, n),
        Adjustment::BrightnessContrast(p) => brightness_contrast(p, buf, n),
        Adjustment::Exposure(p) => exposure(p, buf, n),
        Adjustment::HueSaturation(p) => hue_saturation(p, buf, n),
        Adjustment::BlackWhite(p) => black_white(p, buf, n),
        Adjustment::PhotoFilter(p) => photo_filter(p, buf, n),
        Adjustment::ChannelMixer(p) => channel_mixer(p, buf, n),
        Adjustment::Vibrance(p) => vibrance(p, buf, n),
        Adjustment::ShadowsHighlights(p) => shadows_highlights(p, buf, n),
        Adjustment::ColorBalance(p) => color_balance(p, buf, n),
        Adjustment::SelectiveColor(p) => selective_color(p, buf, n),
        Adjustment::Auto(kind) => auto(*kind, buf, n, coverage),
        Adjustment::Invert => {
            map_float(buf, n, |v| 255 - v);
            Ok(())
        }
        Adjustment::Posterize(levels) => posterize(*levels, buf, n),
        Adjustment::Threshold(level) => threshold(*level, buf, n),
        Adjustment::Desaturate => {
            desaturate(buf, n);
            Ok(())
        }
        Adjustment::Equalize => {
            equalize(buf, n);
            Ok(())
        }
        Adjustment::GradientMap(p) => gradient_map(p, buf, n),
        Adjustment::GradientFill(_) => Err(AdjustError::Unsupported(
            "gradient fill is composited, not applied destructively".into(),
        )),
        Adjustment::SolidFill(_) => Err(AdjustError::Unsupported(
            "solid fill is composited, not applied destructively".into(),
        )),
        Adjustment::PatternFill(_) => Err(AdjustError::Unsupported(
            "pattern fill is composited, not applied destructively".into(),
        )),
        Adjustment::ColorLookup(p) => crate::lut::apply_color_lookup(p, buf),
        Adjustment::ReplaceColor(p) => replace_color(p, buf, n),
        Adjustment::MatchColor(p) => match_color(p, buf, n),
    }
}
