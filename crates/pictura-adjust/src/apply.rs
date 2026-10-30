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

/// The CPU's Gradient Map luma index for an 8-bit RGB triple. The GPU mirrors
/// this with integer arithmetic plus a tie-correction table so both select the
/// same gradient entry.
pub fn gradient_luma_index(r: u8, g: u8, b: u8) -> u8 {
    crate::common::luma(f64::from(r), f64::from(g), f64::from(b))
        .round()
        .clamp(0.0, 255.0) as u8
}

/// A 256-entry lookup table equivalent to applying an adjustment to an 8-bit
/// sample. The GPU uploads this instead of re-deriving the adjustment math.
#[allow(clippy::large_enum_variant)] // 1 KiB table; the plan carries it once per layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdjustmentLut {
    /// One table per colour channel (Levels, Curves, Exposure).
    PerChannel([[u8; 256]; 3]),
    /// Luma-indexed `(r, g, b, alpha)` entries (Gradient Map).
    Luma([[u8; 4]; 256]),
}

/// Build the lookup table for an adjustment the GPU can apply from a table, or
/// `None` when the adjustment has no table form.
///
/// The per-channel tables are produced by applying the adjustment to a neutral
/// `0..=255` ramp, so the channel table and the composite table are composed by
/// the same code the CPU runs. Gradient Map's alpha is carried separately
/// because `apply` discards it.
pub fn adjustment_lut(adjustment: &Adjustment) -> Option<AdjustmentLut> {
    match adjustment {
        Adjustment::Levels(_) | Adjustment::Curves(_) | Adjustment::Exposure(_) => {
            let mut ramp = PixelBuffer::new(256, 1, 4);
            for i in 0..256usize {
                let v = i as u8;
                ramp.data[i] = v;
                ramp.data[256 + i] = v;
                ramp.data[512 + i] = v;
                ramp.data[768 + i] = 255;
            }
            apply(adjustment, &mut ramp).ok()?;
            let mut table = [[0u8; 256]; 3];
            table[0].copy_from_slice(&ramp.data[..256]);
            table[1].copy_from_slice(&ramp.data[256..512]);
            table[2].copy_from_slice(&ramp.data[512..768]);
            Some(AdjustmentLut::PerChannel(table))
        }
        Adjustment::GradientMap(params) => {
            let lut = crate::tonal::gradient_lut(params).ok()?;
            let mut table = [[0u8; 4]; 256];
            for (i, (color, alpha)) in lut.iter().enumerate() {
                table[i] = [
                    color[0],
                    color[1],
                    color[2],
                    (*alpha * 255.0).round().clamp(0.0, 255.0) as u8,
                ];
            }
            Some(AdjustmentLut::Luma(table))
        }
        _ => None,
    }
}

#[cfg(test)]
mod lut_tests {
    use super::*;
    use crate::types::{GradientMapParams, GradientStop};

    fn ramp() -> PixelBuffer {
        let mut buf = PixelBuffer::new(256, 1, 4);
        for i in 0..256usize {
            buf.data[i] = i as u8;
            buf.data[256 + i] = i as u8;
            buf.data[512 + i] = i as u8;
            buf.data[768 + i] = 255;
        }
        buf
    }

    #[test]
    #[allow(clippy::needless_range_loop)]
    fn adjustment_lut_matches_apply() {
        let adjustments = [
            Adjustment::Levels(crate::types::LevelsParams {
                input_black: 10,
                input_white: 240,
                gamma: 1.2,
                output_black: 5,
                output_white: 250,
                red: None,
                green: None,
                blue: None,
            }),
            Adjustment::Curves(crate::types::CurvesParams {
                points: vec![(0, 0), (64, 90), (192, 170), (255, 255)],
                red: None,
                green: None,
                blue: None,
            }),
            Adjustment::Exposure(crate::types::ExposureParams {
                exposure: 0.5,
                offset: 0.02,
                gamma: 1.1,
            }),
        ];
        for adjustment in adjustments {
            let mut expected = ramp();
            apply(&adjustment, &mut expected).expect("cpu apply");
            let Some(AdjustmentLut::PerChannel(table)) = adjustment_lut(&adjustment) else {
                panic!("expected a per-channel table");
            };
            for i in 0..256 {
                assert_eq!(table[0][i], expected.data[i], "red {i}");
                assert_eq!(table[1][i], expected.data[256 + i], "green {i}");
                assert_eq!(table[2][i], expected.data[512 + i], "blue {i}");
            }
        }
    }

    #[test]
    fn gradient_map_lut_carries_alpha() {
        let adjustment = Adjustment::GradientMap(GradientMapParams {
            stops: vec![
                GradientStop {
                    location: 0,
                    color: [255, 0, 0],
                },
                GradientStop {
                    location: 4096,
                    color: [0, 0, 255],
                },
            ],
            transparency: Vec::new(),
            reverse: false,
        });
        let Some(AdjustmentLut::Luma(table)) = adjustment_lut(&adjustment) else {
            panic!("expected a luma table");
        };
        assert_eq!(&table[0][..3], &[255, 0, 0]);
        assert_eq!(&table[255][..3], &[0, 0, 255]);
        assert!(table.iter().all(|entry| entry[3] == 255));
    }
}
