//! Select > Color Range (`SEL-005`): a graded coverage mask for a sampled
//! colour, one of six hue bands, or one of three tonal bands. Ported from
//! photorust's `core/src/wand.rs` (`color_range`).
//!
//! The distance metric and band widths are approximations (behavioural parity
//! only; Adobe's are unpublished). ponytail: no Skin Tones, Detect Faces,
//! Localized Color Clusters, Out Of Gamut, or plus / minus samples.

use crate::{rgb_at, PixelBuffer, Selection};

/// The dialog's Select list, in its order (the index crosses the bridge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRangeSelect {
    Sampled,
    Reds,
    Yellows,
    Greens,
    Cyans,
    Blues,
    Magentas,
    Highlights,
    Midtones,
    Shadows,
}

impl ColorRangeSelect {
    /// 0 Sampled Colors … 9 Shadows; `None` outside the list.
    pub fn from_index(index: i32) -> Option<ColorRangeSelect> {
        use ColorRangeSelect::*;
        [
            Sampled, Reds, Yellows, Greens, Cyans, Blues, Magentas, Highlights, Midtones, Shadows,
        ]
        .get(usize::try_from(index).ok()?)
        .copied()
    }

    /// The hue (degrees) a colour band is centred on.
    fn hue_centre(self) -> Option<f32> {
        Some(match self {
            ColorRangeSelect::Reds => 0.0,
            ColorRangeSelect::Yellows => 60.0,
            ColorRangeSelect::Greens => 120.0,
            ColorRangeSelect::Cyans => 180.0,
            ColorRangeSelect::Blues => 240.0,
            ColorRangeSelect::Magentas => 300.0,
            _ => return None,
        })
    }

    /// The lightness (0..1) a tonal band is centred on.
    fn tone_centre(self) -> Option<f32> {
        Some(match self {
            ColorRangeSelect::Shadows => 0.0,
            ColorRangeSelect::Midtones => 0.5,
            ColorRangeSelect::Highlights => 1.0,
            _ => return None,
        })
    }
}

/// Hue in degrees, then saturation and lightness in 0..1.
fn to_hsl([r, g, b]: [u8; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    let span = max - min;
    if span <= f32::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let saturation = if lightness > 0.5 {
        span / (2.0 - max - min)
    } else {
        span / (max + min)
    };
    let hue = if max == r {
        60.0 * (((g - b) / span) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / span + 2.0)
    } else {
        60.0 * ((r - g) / span + 4.0)
    };
    (
        if hue < 0.0 { hue + 360.0 } else { hue },
        saturation,
        lightness,
    )
}

/// The shorter way round the colour wheel between two hues, in degrees.
fn hue_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    if d > 180.0 {
        360.0 - d
    } else {
        d
    }
}

/// How much of each pixel of `img` Color Range takes (0..255), as a
/// selection: graded, so Fuzziness (0..200, clamped) softens an edge as well
/// as widening it. `target` matters only for [`ColorRangeSelect::Sampled`];
/// `invert` gives `255 - coverage`.
pub fn color_range(
    img: &PixelBuffer,
    select: ColorRangeSelect,
    target: [u8; 3],
    fuzziness: u32,
    invert: bool,
) -> Selection {
    let n = img.width as usize * img.height as usize;
    // As a fraction: how far past an exact match a pixel may be and still count.
    let fuzz = (fuzziness.min(200) as f32 / 200.0).max(0.001);
    let data: Vec<u8> = (0..n)
        .map(|p| {
            let px = rgb_at(img, p);
            let coverage = if select == ColorRangeSelect::Sampled {
                let d: f32 = px
                    .iter()
                    .zip(target)
                    .map(|(a, b)| (*a as f32 - b as f32).powi(2))
                    .sum();
                // Plain RGB distance, normalised by the cube's diagonal.
                1.0 - d.sqrt() / 441.673 / fuzz
            } else {
                let (hue, saturation, lightness) = to_hsl(px);
                if let Some(centre) = select.hue_centre() {
                    // A grey has no hue, so it belongs to no colour band.
                    let reach = 30.0 + fuzz * 60.0;
                    (1.0 - hue_distance(hue, centre) / reach) * saturation
                } else {
                    let centre = select.tone_centre().unwrap_or(0.5);
                    1.0 - (lightness - centre).abs() / (0.25 + fuzz * 0.5)
                }
            };
            let v = (coverage.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            if invert {
                255 - v
            } else {
                v
            }
        })
        .collect();
    Selection {
        width: img.width,
        height: img.height,
        data: data.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-row planar RGB image of `pixels`.
    fn row(pixels: &[[u8; 3]]) -> PixelBuffer {
        let n = pixels.len();
        let mut data = vec![0u8; 3 * n];
        for (i, px) in pixels.iter().enumerate() {
            for c in 0..3 {
                data[c * n + i] = px[c];
            }
        }
        PixelBuffer {
            width: n as u32,
            height: 1,
            channels: 3,
            data: data.into(),
        }
    }

    const RED: [u8; 3] = [255, 0, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    #[test]
    fn sampled_takes_the_match_and_leaves_the_rest() {
        let mask = color_range(
            &row(&[RED, BLUE]),
            ColorRangeSelect::Sampled,
            RED,
            40,
            false,
        );
        assert_eq!(mask.data, vec![255, 0]);
        let inverted = color_range(&row(&[RED, BLUE]), ColorRangeSelect::Sampled, RED, 40, true);
        assert_eq!(inverted.data, vec![0, 255]);
    }

    #[test]
    fn fuzziness_widens_and_softens() {
        let nearly = [200, 40, 40];
        let at = |f| color_range(&row(&[nearly]), ColorRangeSelect::Sampled, RED, f, false).data[0];
        assert!(at(120) > at(10), "more fuzziness takes more of a near miss");
        assert!(
            at(120) > 0 && at(120) < 255,
            "and leaves it partly selected"
        );
        let ramp: Vec<[u8; 3]> = (0..16).map(|x| [x * 16, x * 16, x * 16]).collect();
        let count = |f| {
            color_range(&row(&ramp), ColorRangeSelect::Sampled, [0, 0, 0], f, false)
                .data
                .iter()
                .filter(|&&v| v > 0)
                .count()
        };
        let mut previous = 0;
        for f in [0, 10, 40, 100, 200, 500] {
            assert!(count(f) >= previous, "fuzziness {f} must not shrink it");
            previous = count(f);
        }
    }

    #[test]
    fn a_colour_band_goes_by_hue_and_ignores_greys() {
        let grey = [128, 128, 128];
        let reds = color_range(
            &row(&[RED, grey]),
            ColorRangeSelect::Reds,
            [0; 3],
            40,
            false,
        );
        assert!(reds.data[0] > 200);
        assert_eq!(reds.data[1], 0, "grey belongs to no colour band");
        let green = [0, 255, 0];
        let greens = color_range(
            &row(&[green, RED]),
            ColorRangeSelect::Greens,
            [0; 3],
            40,
            false,
        );
        assert!(greens.data[0] > 200);
        assert_eq!(greens.data[1], 0, "red is not a green");
    }

    #[test]
    fn the_tonal_bands_split_light_from_dark() {
        let img = row(&[[255; 3], [0; 3]]);
        let highlights = color_range(&img, ColorRangeSelect::Highlights, [0; 3], 40, false);
        assert!(highlights.data[0] > 200 && highlights.data[1] == 0);
        let shadows = color_range(&img, ColorRangeSelect::Shadows, [0; 3], 40, false);
        assert!(shadows.data[1] > 200 && shadows.data[0] == 0);
    }

    #[test]
    fn select_indices_follow_the_dialog() {
        assert_eq!(
            ColorRangeSelect::from_index(0),
            Some(ColorRangeSelect::Sampled)
        );
        assert_eq!(
            ColorRangeSelect::from_index(9),
            Some(ColorRangeSelect::Shadows)
        );
        assert_eq!(ColorRangeSelect::from_index(10), None);
        assert_eq!(ColorRangeSelect::from_index(-1), None);
    }
}
