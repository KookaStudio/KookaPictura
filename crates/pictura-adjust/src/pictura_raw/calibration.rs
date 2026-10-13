//! The Camera Calibration tab: red/green/blue primary hue and saturation, and
//! the shadow tint. The camera profile and process version are metadata and do
//! not transform the pixels here.

use pictura_core::CameraCalibration;

use crate::common::{hsl_to_rgb, rgb_to_hsl};

use super::ranges::hue_weight;
use super::Planes;

const PRIMARY_CENTERS: [f64; 3] = [0.0, 120.0, 240.0];

pub(crate) fn apply(p: &mut Planes, c: &CameraCalibration) {
    let hues = [c.red_hue, c.green_hue, c.blue_hue];
    let sats = [c.red_saturation, c.green_saturation, c.blue_saturation];
    let primaries = hues.iter().any(|v| v.is_some_and(|x| x != 0))
        || sats.iter().any(|v| v.is_some_and(|x| x != 0));
    let tint = c.shadow_tint.unwrap_or(0);
    if !primaries && tint == 0 {
        return;
    }

    if primaries {
        for i in 0..p.len() {
            let (hue, s, l) = rgb_to_hsl(p.r[i] as f64, p.g[i] as f64, p.b[i] as f64);
            let (mut dh, mut ds) = (0.0, 0.0);
            for r in 0..3 {
                let w = hue_weight(hue, PRIMARY_CENTERS[r]);
                if w == 0.0 {
                    continue;
                }
                dh += w * hues[r].unwrap_or(0) as f64;
                ds += w * sats[r].unwrap_or(0) as f64;
            }
            // ponytail: the slider scales are approximate; the reference's
            // colour-matrix model for the primaries is closed.
            let nh = (hue + dh * 0.5).rem_euclid(360.0);
            let ns = (s * (1.0 + ds / 100.0)).clamp(0.0, 1.0);
            let (nr, ng, nb) = hsl_to_rgb(nh, ns, l);
            p.r[i] = nr as f32;
            p.g[i] = ng as f32;
            p.b[i] = nb as f32;
        }
    }

    if tint != 0 {
        // A green <-> magenta shift weighted toward the shadows. ponytail: the
        // reference's shadow-tint curve is closed.
        let t = tint as f32 / 100.0 * 0.2;
        for i in 0..p.len() {
            let luma = 0.299 * p.r[i] + 0.587 * p.g[i] + 0.114 * p.b[i];
            let shadow = (1.0 - luma).clamp(0.0, 1.0);
            p.r[i] = (p.r[i] + t * shadow).clamp(0.0, 1.0);
            p.g[i] = (p.g[i] - t * shadow).clamp(0.0, 1.0);
            p.b[i] = (p.b[i] + t * shadow).clamp(0.0, 1.0);
        }
    }
}
