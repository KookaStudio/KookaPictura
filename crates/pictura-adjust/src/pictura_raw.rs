//! Pictura Raw: our reimplementation of the 11 PV2012 Basic tone controls,
//! applied to an 8-bit RGB/RGBA buffer in `f32`.
//!
//! Pipeline order: WB (temperature/tint) -> exposure -> contrast ->
//! highlights/shadows/whites/blacks -> clarity -> vibrance -> saturation.
//! The exact PV2012 curves are closed, so every stage is an approximation
//! and marked `ponytail:` inline. The pass is deterministic and a default
//! (all-`None`) [`PicturaRawSettings`] is a byte-identical no-op.

use pictura_core::{PicturaRawSettings, PixelBuffer};

use crate::color::skin_bump;
use crate::common::{hermite_eval, hsl_to_rgb, linear_to_srgb, rgb_to_hsl, srgb_to_linear};
use crate::types::AdjustError;

/// Apply the Pictura Raw Basic controls to `input`, working in `f32` and
/// preserving the channel count and alpha.
pub fn render_pictura_raw(
    input: &PixelBuffer,
    settings: &PicturaRawSettings,
) -> Result<PixelBuffer, AdjustError> {
    let ch = input.channels as usize;
    if ch != 3 && ch != 4 {
        return Err(AdjustError::Unsupported(format!(
            "channel count {ch} is not supported (expected 3 or 4)"
        )));
    }
    let n = input.pixel_count();
    if n == 0 {
        return Err(AdjustError::InvalidParams("empty buffer".into()));
    }
    if input.data.len() != n * ch {
        return Err(AdjustError::InvalidParams(
            "buffer length does not match width*height*channels".into(),
        ));
    }

    let v = |x: Option<f64>| x.filter(|v| v.is_finite()).unwrap_or(0.0);
    let (wb_temp, wb_tint) = (v(settings.basic.temperature), v(settings.basic.tint));
    let exposure = v(settings.basic.exposure);
    let contrast = v(settings.basic.contrast);
    let (highlights, shadows) = (v(settings.basic.highlights), v(settings.basic.shadows));
    let (whites, blacks) = (v(settings.basic.whites), v(settings.basic.blacks));
    let clarity = v(settings.basic.clarity);
    let vibrance = v(settings.basic.vibrance);
    let saturation = v(settings.basic.saturation);
    if [
        wb_temp, wb_tint, exposure, contrast, highlights, shadows, whites, blacks, clarity,
        vibrance, saturation,
    ]
    .iter()
    .all(|x| *x == 0.0)
    {
        return Ok(input.clone());
    }

    let (w, h) = (input.width as usize, input.height as usize);
    let mut r = vec![0f32; n];
    let mut g = vec![0f32; n];
    let mut b = vec![0f32; n];
    for i in 0..n {
        r[i] = input.data[i] as f32 / 255.0;
        g[i] = input.data[n + i] as f32 / 255.0;
        b[i] = input.data[2 * n + i] as f32 / 255.0;
    }

    if wb_temp != 0.0 || wb_tint != 0.0 {
        // ponytail: approximate JPEG/processed-file slider scale; the reference's
        // camera-matrix white balance is closed.
        let t = (wb_temp / 100.0) as f32;
        let ti = (wb_tint / 100.0) as f32;
        let (gr, gg, gb) = (1.0 + 0.20 * t, 1.0 - 0.10 * ti, 1.0 - 0.20 * t);
        for i in 0..n {
            r[i] = (r[i] * gr).clamp(0.0, 1.0);
            g[i] = (g[i] * gg).clamp(0.0, 1.0);
            b[i] = (b[i] * gb).clamp(0.0, 1.0);
        }
    }

    if exposure != 0.0 {
        // Linear-light gain in EV stops, like the shipped Exposure kernel.
        let gain = 2f64.powf(exposure);
        for plane in [&mut r, &mut g, &mut b] {
            for x in plane.iter_mut() {
                *x = (linear_to_srgb(srgb_to_linear(*x as f64) * gain)).clamp(0.0, 1.0) as f32;
            }
        }
    }

    if contrast != 0.0 {
        // Monotone S-curve about mid-grey; the reference's contrast curve is closed.
        let c = contrast / 100.0;
        let (xs, ys, ms) = ([0.0, 0.5, 1.0], [0.0, 0.5, 1.0], [1.0, 1.0 + c, 1.0]);
        for plane in [&mut r, &mut g, &mut b] {
            for x in plane.iter_mut() {
                *x = hermite_eval(&xs, &ys, &ms, *x as f64).clamp(0.0, 1.0) as f32;
            }
        }
    }

    if highlights != 0.0 || shadows != 0.0 || whites != 0.0 || blacks != 0.0 {
        // ponytail: weighted tone-band lifts, not PV2012's closed curves.
        let (hi, sh) = ((highlights / 100.0) as f32, (shadows / 100.0) as f32);
        let (wh, bl) = ((whites / 100.0) as f32, (blacks / 100.0) as f32);
        for i in 0..n {
            let y = ((r[i] + g[i] + b[i]) / 3.0).clamp(0.0, 1.0);
            let adj = hi * 0.5 * y * y
                + sh * 0.5 * (1.0 - y) * (1.0 - y)
                + wh * 0.25 * y
                + bl * 0.25 * (1.0 - y);
            r[i] = (r[i] + adj).clamp(0.0, 1.0);
            g[i] = (g[i] + adj).clamp(0.0, 1.0);
            b[i] = (b[i] + adj).clamp(0.0, 1.0);
        }
    }

    if clarity != 0.0 {
        // ponytail: local luminance detail against a fixed-radius box blur; the
        // radius is image-size relative and the reference's clarity kernel is closed.
        let radius = (w.min(h) / 8).clamp(1, 16);
        let yplane: Vec<f32> = (0..n)
            .map(|i| 0.299 * r[i] + 0.587 * g[i] + 0.114 * b[i])
            .collect();
        let blurred = box_blur(&yplane, w, h, radius);
        let k = (clarity / 100.0) as f32;
        for i in 0..n {
            let detail = (yplane[i] - blurred[i]) * k;
            r[i] = (r[i] + detail).clamp(0.0, 1.0);
            g[i] = (g[i] + detail).clamp(0.0, 1.0);
            b[i] = (b[i] + detail).clamp(0.0, 1.0);
        }
    }

    if vibrance != 0.0 || saturation != 0.0 {
        // Vibrance falloff and skin damping match the shipped Vibrance kernel.
        let kv = vibrance / 100.0;
        let ks = saturation / 100.0;
        for i in 0..n {
            let (hue, s, l) = rgb_to_hsl(r[i] as f64, g[i] as f64, b[i] as f64);
            let keep = 1.0 - 0.5 * skin_bump(hue);
            let s = (s + kv * (1.0 - s) * keep).clamp(0.0, 1.0);
            let s = (s * (1.0 + ks)).clamp(0.0, 1.0);
            let (nr, ng, nb) = hsl_to_rgb(hue, s, l);
            r[i] = nr as f32;
            g[i] = ng as f32;
            b[i] = nb as f32;
        }
    }

    let mut out = PixelBuffer::new(input.width, input.height, input.channels);
    for i in 0..n {
        out.data[i] = (r[i] * 255.0).round().clamp(0.0, 255.0) as u8;
        out.data[n + i] = (g[i] * 255.0).round().clamp(0.0, 255.0) as u8;
        out.data[2 * n + i] = (b[i] * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    if ch == 4 {
        out.data[3 * n..4 * n].copy_from_slice(&input.data[3 * n..4 * n]);
    }
    Ok(out)
}

/// Separable box blur with clamped edges, `O(pixels)`. `radius` is the half
/// window; the caller keeps it below `min(w, h)`.
fn box_blur(src: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    let mut tmp = vec![0f32; src.len()];
    let mut out = vec![0f32; src.len()];
    let win = (2 * radius + 1) as f32;
    for y in 0..h {
        let row = y * w;
        let mut sum = src[row] * radius as f32;
        for k in 0..=radius {
            sum += src[row + k.min(w - 1)];
        }
        tmp[row] = sum / win;
        for x in 1..w {
            sum += src[row + (x + radius).min(w - 1)] - src[row + (x - 1).saturating_sub(radius)];
            tmp[row + x] = sum / win;
        }
    }
    for x in 0..w {
        let mut sum = tmp[x] * radius as f32;
        for k in 0..=radius {
            sum += tmp[k.min(h - 1) * w + x];
        }
        out[x] = sum / win;
        for y in 1..h {
            sum +=
                tmp[(y + radius).min(h - 1) * w + x] - tmp[(y - 1).saturating_sub(radius) * w + x];
            out[y * w + x] = sum / win;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::CrsSettings;

    fn gray(w: u32, h: u32, value: u8) -> PixelBuffer {
        PixelBuffer {
            width: w,
            height: h,
            channels: 3,
            data: vec![value; (w * h) as usize * 3].into(),
        }
    }

    fn rgb_grid(colors: &[[u8; 3]]) -> PixelBuffer {
        let n = colors.len() as u32;
        let mut data = Vec::new();
        for c in colors {
            data.push(c[0]);
        }
        for c in colors {
            data.push(c[1]);
        }
        for c in colors {
            data.push(c[2]);
        }
        PixelBuffer {
            width: n,
            height: 1,
            channels: 3,
            data: data.into(),
        }
    }

    fn px(buf: &PixelBuffer, x: usize) -> [u8; 3] {
        let n = buf.pixel_count();
        [buf.data[x], buf.data[n + x], buf.data[2 * n + x]]
    }

    #[test]
    fn default_settings_are_a_byte_identical_no_op() {
        let input = gray(4, 4, 128);
        let out = render_pictura_raw(&input, &PicturaRawSettings::default()).unwrap();
        assert_eq!(out, input);
        // NaN fields are ignored, not propagated.
        let weird = PicturaRawSettings {
            basic: CrsSettings {
                exposure: Some(f64::NAN),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(render_pictura_raw(&input, &weird).unwrap(), input);
    }

    #[test]
    fn exposure_is_monotonic_and_preserves_alpha() {
        let input = gray(2, 2, 100);
        let brighter = render_pictura_raw(
            &input,
            &PicturaRawSettings {
                basic: CrsSettings {
                    exposure: Some(1.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let darker = render_pictura_raw(
            &input,
            &PicturaRawSettings {
                basic: CrsSettings {
                    exposure: Some(-1.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        assert!(px(&brighter, 0)[0] > 100);
        assert!(px(&darker, 0)[0] < 100);

        let mut rgba = input.clone();
        rgba.channels = 4;
        let n = rgba.pixel_count();
        rgba.data.extend(std::iter::repeat_n(7u8, n));
        let out = render_pictura_raw(
            &rgba,
            &PicturaRawSettings {
                basic: CrsSettings {
                    exposure: Some(1.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out.channels, 4);
        assert!(out.data[3 * n..].iter().all(|&a| a == 7), "alpha preserved");
    }

    #[test]
    fn temperature_is_monotonic_on_a_neutral() {
        let input = gray(2, 2, 128);
        let warm = render_pictura_raw(
            &input,
            &PicturaRawSettings {
                basic: CrsSettings {
                    temperature: Some(60.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let cool = render_pictura_raw(
            &input,
            &PicturaRawSettings {
                basic: CrsSettings {
                    temperature: Some(-60.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let warm_px = px(&warm, 0);
        let cool_px = px(&cool, 0);
        assert!(warm_px[0] as i32 - warm_px[2] as i32 > 0, "warm is redder");
        assert!(
            (cool_px[0] as i32) - (cool_px[2] as i32) < 0,
            "cool is bluer"
        );
    }

    #[test]
    fn clarity_boosts_local_contrast() {
        let input = rgb_grid(&[[64, 64, 64], [64, 64, 64], [192, 192, 192], [192, 192, 192]]);
        let more = render_pictura_raw(
            &input,
            &PicturaRawSettings {
                basic: CrsSettings {
                    clarity: Some(80.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let less = render_pictura_raw(
            &input,
            &PicturaRawSettings {
                basic: CrsSettings {
                    clarity: Some(-80.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        // The central edge's step is what local contrast sharpens or softens.
        let edge = |b: &PixelBuffer| px(b, 2)[0] as i32 - px(b, 1)[0] as i32;
        assert!(edge(&more) > edge(&input), "positive clarity steepens");
        assert!(edge(&less) < edge(&input), "negative clarity softens");
    }

    #[test]
    fn vibrance_increases_saturation_and_is_deterministic() {
        let input = rgb_grid(&[[130, 150, 130], [130, 150, 130]]);
        let settings = PicturaRawSettings {
            basic: CrsSettings {
                vibrance: Some(70.0),
                ..Default::default()
            },
            ..Default::default()
        };
        let a = render_pictura_raw(&input, &settings).unwrap();
        let b = render_pictura_raw(&input, &settings).unwrap();
        assert_eq!(a, b, "deterministic");
        let sat = |v: [u8; 3]| v[1] as i32 - v[0] as i32;
        assert!(sat(px(&a, 0)) > sat(px(&input, 0)), "more saturated");
    }
}
