//! The Basic tab: the 11 PV2012 tone/colour controls, in pipeline order
//! WB (temperature/tint) -> exposure -> contrast -> highlights/shadows/whites/
//! blacks -> clarity -> vibrance -> saturation.

use pictura_core::CrsSettings;

use crate::color::skin_bump;
use crate::common::{hermite_eval, hsl_to_rgb, linear_to_srgb, rgb_to_hsl, srgb_to_linear};

use super::{box_blur, Planes};

pub(crate) fn apply(p: &mut Planes, s: &CrsSettings) {
    let v = |x: Option<f64>| x.filter(|v| v.is_finite()).unwrap_or(0.0);
    let (wb_temp, wb_tint) = (v(s.temperature), v(s.tint));
    let exposure = v(s.exposure);
    let contrast = v(s.contrast);
    let (highlights, shadows) = (v(s.highlights), v(s.shadows));
    let (whites, blacks) = (v(s.whites), v(s.blacks));
    let clarity = v(s.clarity);
    let vibrance = v(s.vibrance);
    let saturation = v(s.saturation);
    if [
        wb_temp, wb_tint, exposure, contrast, highlights, shadows, whites, blacks, clarity,
        vibrance, saturation,
    ]
    .iter()
    .all(|x| *x == 0.0)
    {
        return;
    }

    let n = p.len();
    let (w, h) = (p.w, p.h);
    let Planes { r, g, b, .. } = p;

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
        for plane in [&mut *r, &mut *g, &mut *b] {
            for x in plane.iter_mut() {
                *x = (linear_to_srgb(srgb_to_linear(*x as f64) * gain)).clamp(0.0, 1.0) as f32;
            }
        }
    }

    if contrast != 0.0 {
        // Monotone S-curve about mid-grey; the reference's contrast curve is closed.
        let c = contrast / 100.0;
        let (xs, ys, ms) = ([0.0, 0.5, 1.0], [0.0, 0.5, 1.0], [1.0, 1.0 + c, 1.0]);
        for plane in [&mut *r, &mut *g, &mut *b] {
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
}
