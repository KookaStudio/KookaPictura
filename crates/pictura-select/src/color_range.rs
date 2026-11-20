//! Select > Color Range (`SEL-005`): a graded coverage mask for a set of
//! sampled colours, one of six hue bands, one of three tonal bands, Skin
//! Tones, or Out Of Gamut. The sampled-colour kernel is shared with Replace
//! Color (`pictura_core::match_weight`).
//!
//! The distance metric, band widths, and skin window are approximations
//! (behavioural parity only; Adobe's are unpublished). ponytail: no Detect
//! Faces; Out Of Gamut is evaluated in the app against the proof setup.

use crate::{rgb_at, PixelBuffer, Selection};
use pictura_core::{match_weight, ReplaceColorSample};

/// The dialog's Select list, in its order (the index crosses the bridge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRangeSelect {
    Sampled,
    SkinTones,
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
    /// 0 Sampled Colors … 10 Shadows; `None` outside the list. Out Of Gamut is
    /// an app-level mode with its own bridge index, not an engine variant.
    pub fn from_index(index: i32) -> Option<ColorRangeSelect> {
        use ColorRangeSelect::*;
        [
            Sampled, SkinTones, Reds, Yellows, Greens, Cyans, Blues, Magentas, Highlights,
            Midtones, Shadows,
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

/// The dialog's settings: the select mode, sampled colours with their document
/// positions, Fuzziness, localization, and Invert.
///
/// `samples` and `origin` are in the evaluated buffer's coordinate system:
/// callers scale document positions into buffer pixels and pass the buffer's
/// top-left as `origin`. A sample with a negative coordinate has no document
/// position and carries no proximity weight. `localization` is 0 (global) to 1
/// (tight clusters); `spatial_sigma` is the cluster width in buffer pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorRangeSettings {
    pub select: ColorRangeSelect,
    pub samples: Vec<ReplaceColorSample>,
    pub fuzziness: u32,
    pub localization: f64,
    pub spatial_sigma: f32,
    pub invert: bool,
    pub origin: (i32, i32),
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

/// A skin heuristic in BT.601 YCbCr: an elliptical chroma window around a
/// mid-tone skin centre, widened by Fuzziness. Greys and saturated non-skin
/// colours are excluded before the distance is measured.
fn skin_coverage([r, g, b]: [u8; 3], fuzziness: u32) -> f32 {
    let (rf, gf, bf) = (r as f32, g as f32, b as f32);
    let cb = 128.0 - 0.168_736 * rf - 0.331_264 * gf + 0.5 * bf;
    let cr = 128.0 + 0.5 * rf - 0.418_688 * gf - 0.081_312 * bf;
    let (_, saturation, lightness) = to_hsl([r, g, b]);
    if saturation < 0.05 || lightness < 0.05 || lightness > 0.97 {
        return 0.0;
    }
    let d = (((cb - 108.0) / 22.0).powi(2) + ((cr - 152.0) / 20.0).powi(2)).sqrt();
    let reach = 1.0 + (fuzziness.min(200) as f32 / 200.0) * 1.5;
    (1.0 - d / reach).clamp(0.0, 1.0)
}

fn band_coverage(px: [u8; 3], select: ColorRangeSelect, fuzz: f32) -> f32 {
    let (hue, saturation, lightness) = to_hsl(px);
    if let Some(centre) = select.hue_centre() {
        // A grey has no hue, so it belongs to no colour band.
        let reach = 30.0 + fuzz * 60.0;
        (1.0 - hue_distance(hue, centre) / reach) * saturation
    } else {
        let centre = select.tone_centre().unwrap_or(0.5);
        1.0 - (lightness - centre).abs() / (0.25 + fuzz * 0.5)
    }
}

/// How much of each pixel of `img` Color Range takes (0..255), as a
/// selection. Fuzziness (0..200, clamped) widens the match; Invert gives
/// `255 - coverage`. Fully transparent pixels take no coverage before Invert.
pub fn color_range(img: &PixelBuffer, settings: &ColorRangeSettings) -> Selection {
    let w = img.width as usize;
    let n = w * img.height as usize;
    let sigma = settings.spatial_sigma.max(f32::MIN_POSITIVE) as f64;
    let sigma_sq = sigma * sigma;
    // As a fraction: how far past an exact match a band may reach.
    let fuzz = (settings.fuzziness.min(200) as f32 / 200.0).max(0.001);
    let data: Vec<u8> = (0..n)
        .map(|p| {
            let px = rgb_at(img, p);
            let x = (p % w) as i32 + settings.origin.0;
            let y = (p / w) as i32 + settings.origin.1;
            let transparent = img.channels == 4 && img.data.sample(3 * n + p).unwrap_or(255) == 0;
            let coverage = if transparent {
                0.0
            } else {
                match settings.select {
                    ColorRangeSelect::Sampled => match_weight(
                        px[0],
                        px[1],
                        px[2],
                        x,
                        y,
                        &settings.samples,
                        settings.fuzziness as f64,
                        settings.localization,
                        sigma_sq,
                    ) as f32,
                    ColorRangeSelect::SkinTones => skin_coverage(px, settings.fuzziness),
                    select => band_coverage(px, select, fuzz),
                }
            };
            let v = (coverage.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            if settings.invert {
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
    use pictura_core::{localization_strength, spatial_sigma};

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

    fn rgba_row(pixels: &[[u8; 4]]) -> PixelBuffer {
        let n = pixels.len();
        let mut data = vec![0u8; 4 * n];
        for (i, px) in pixels.iter().enumerate() {
            for c in 0..4 {
                data[c * n + i] = px[c];
            }
        }
        PixelBuffer {
            width: n as u32,
            height: 1,
            channels: 4,
            data: data.into(),
        }
    }

    const RED: [u8; 3] = [255, 0, 0];
    const GREEN: [u8; 3] = [0, 255, 0];
    const BLUE: [u8; 3] = [0, 0, 255];

    fn sample(x: i32, rgb: [u8; 3]) -> ReplaceColorSample {
        ReplaceColorSample { x, y: 0, rgb }
    }

    fn settings(select: ColorRangeSelect, samples: Vec<ReplaceColorSample>) -> ColorRangeSettings {
        ColorRangeSettings {
            select,
            samples,
            fuzziness: 40,
            localization: 0.0,
            spatial_sigma: 1.0,
            invert: false,
            origin: (0, 0),
        }
    }

    #[test]
    fn sampled_takes_the_match_and_leaves_the_rest() {
        let mask = color_range(
            &row(&[RED, BLUE]),
            &settings(ColorRangeSelect::Sampled, vec![sample(-1, RED)]),
        );
        assert_eq!(mask.data, vec![255, 0]);
        let mut inverted = settings(ColorRangeSelect::Sampled, vec![sample(-1, RED)]);
        inverted.invert = true;
        assert_eq!(
            color_range(&row(&[RED, BLUE]), &inverted).data,
            vec![0, 255]
        );
    }

    #[test]
    fn fuzziness_widens_and_softens() {
        let nearly = [200, 40, 40];
        let at = |f| {
            let mut s = settings(ColorRangeSelect::Sampled, vec![sample(-1, RED)]);
            s.fuzziness = f;
            color_range(&row(&[nearly]), &s).data[0]
        };
        assert!(at(120) > at(10), "more fuzziness takes more of a near miss");
        assert!(
            at(120) > 0 && at(120) < 255,
            "and leaves it partly selected"
        );
        let ramp: Vec<[u8; 3]> = (0..16).map(|x| [x * 16, x * 16, x * 16]).collect();
        let count = |f| {
            let mut s = settings(ColorRangeSelect::Sampled, vec![sample(-1, [0, 0, 0])]);
            s.fuzziness = f;
            color_range(&row(&ramp), &s)
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
    fn every_sample_contributes_its_best_match() {
        let img = row(&[RED, GREEN, BLUE]);
        let two = color_range(
            &img,
            &settings(
                ColorRangeSelect::Sampled,
                vec![sample(0, RED), sample(2, BLUE)],
            ),
        );
        assert_eq!(two.data, vec![255, 0, 255]);
        let three = color_range(
            &img,
            &settings(
                ColorRangeSelect::Sampled,
                vec![sample(0, RED), sample(1, GREEN), sample(2, BLUE)],
            ),
        );
        assert_eq!(three.data, vec![255, 255, 255]);
    }

    #[test]
    fn localized_clusters_fall_off_with_range() {
        let n = 41;
        let mut pixels = vec![[0u8; 3]; n];
        pixels[0] = RED;
        pixels[n - 1] = RED;
        let img = row(&pixels);
        let sigma = spatial_sigma(n as f32);
        let mut s = settings(ColorRangeSelect::Sampled, vec![sample(0, RED)]);
        s.spatial_sigma = sigma;
        s.localization = localization_strength(0);
        let tight = color_range(&img, &s);
        assert_eq!(tight.data[0], 255);
        assert_eq!(tight.data[n - 1], 0, "the far end is outside the cluster");
        s.localization = localization_strength(100);
        let wide = color_range(&img, &s);
        assert_eq!(wide.data[0], 255);
        assert_eq!(wide.data[n - 1], 255, "Range 100 has no proximity penalty");
        s.localization = 0.0;
        assert_eq!(color_range(&img, &s).data[n - 1], 255);
    }

    #[test]
    fn localized_ignores_a_positionless_sample() {
        let n = 41;
        let mut pixels = vec![[0u8; 3]; n];
        pixels[n - 1] = RED;
        let mut s = settings(
            ColorRangeSelect::Sampled,
            vec![ReplaceColorSample {
                x: -1,
                y: -1,
                rgb: RED,
            }],
        );
        s.spatial_sigma = spatial_sigma(n as f32);
        s.localization = 1.0;
        assert_eq!(color_range(&row(&pixels), &s).data[n - 1], 255);
    }

    #[test]
    fn a_colour_band_goes_by_hue_and_ignores_greys() {
        let grey = [128, 128, 128];
        let reds = color_range(
            &row(&[RED, grey]),
            &settings(ColorRangeSelect::Reds, Vec::new()),
        );
        assert!(reds.data[0] > 200);
        assert_eq!(reds.data[1], 0, "grey belongs to no colour band");
        let greens = color_range(
            &row(&[GREEN, RED]),
            &settings(ColorRangeSelect::Greens, Vec::new()),
        );
        assert!(greens.data[0] > 200);
        assert_eq!(greens.data[1], 0, "red is not a green");
    }

    #[test]
    fn the_tonal_bands_split_light_from_dark() {
        let img = row(&[[255; 3], [0; 3]]);
        let highlights = color_range(&img, &settings(ColorRangeSelect::Highlights, Vec::new()));
        assert!(highlights.data[0] > 200 && highlights.data[1] == 0);
        let shadows = color_range(&img, &settings(ColorRangeSelect::Shadows, Vec::new()));
        assert!(shadows.data[1] > 200 && shadows.data[0] == 0);
    }

    #[test]
    fn skin_tones_ignore_non_skin_colours() {
        let skin = [224, 172, 140];
        let img = row(&[skin, BLUE, [128, 128, 128]]);
        let mut s = settings(ColorRangeSelect::SkinTones, Vec::new());
        let mask = color_range(&img, &s);
        assert!(mask.data[0] > 0, "the skin swatch is selected");
        assert_eq!(mask.data[1], 0, "saturated blue is not skin");
        assert_eq!(mask.data[2], 0, "grey is not skin");
        s.fuzziness = 200;
        let wide = color_range(&img, &s);
        assert!(wide.data[0] >= mask.data[0], "fuzziness never removes skin");
        assert_eq!(wide.data[2], 0, "grey stays excluded at high fuzziness");
    }

    #[test]
    fn transparent_pixels_take_no_coverage() {
        let black = [0u8; 3];
        let img = rgba_row(&[[0, 0, 0, 0], [255, 0, 0, 255]]);
        let mut s = settings(ColorRangeSelect::Sampled, vec![sample(-1, black)]);
        let mask = color_range(&img, &s);
        assert_eq!(mask.data[0], 0, "transparent black is not matched");
        assert_eq!(mask.data[1], 0, "opaque red is not black");
        s.invert = true;
        let inverted = color_range(&img, &s);
        assert_eq!(inverted.data[0], 255);
        assert_eq!(inverted.data[1], 255);
    }

    #[test]
    fn select_indices_follow_the_dialog() {
        assert_eq!(
            ColorRangeSelect::from_index(0),
            Some(ColorRangeSelect::Sampled)
        );
        assert_eq!(
            ColorRangeSelect::from_index(1),
            Some(ColorRangeSelect::SkinTones)
        );
        assert_eq!(
            ColorRangeSelect::from_index(10),
            Some(ColorRangeSelect::Shadows)
        );
        assert_eq!(ColorRangeSelect::from_index(11), None);
        assert_eq!(ColorRangeSelect::from_index(-1), None);
    }
}
