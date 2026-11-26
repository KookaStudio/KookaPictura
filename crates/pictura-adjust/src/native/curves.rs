use pictura_color::model;
use pictura_core::Samples;

use super::{map, Rgb};
use crate::color::cmyk_unit_to_rgb;
use crate::common::{hermite_eval, monotone_tangents};
use crate::types::{AdjustError, ColorBalanceMode, CurvesParams};

struct Curve {
    xs: Vec<f64>,
    ys: Vec<f64>,
    ms: Vec<f64>,
}

fn curve(points: &[(u8, u8)]) -> Result<Curve, AdjustError> {
    if points.len() < 2 {
        return Err(AdjustError::InvalidParams(
            "curves need at least 2 points".into(),
        ));
    }
    if points.len() > 14 && points.len() != 256 {
        return Err(AdjustError::InvalidParams(
            "curves support at most 14 points".into(),
        ));
    }
    let xs: Vec<f64> = points.iter().map(|q| q.0 as f64).collect();
    let ys: Vec<f64> = points.iter().map(|q| q.1 as f64).collect();
    if xs.windows(2).any(|w| w[0] >= w[1]) {
        return Err(AdjustError::InvalidParams(
            "curve inputs must be strictly increasing".into(),
        ));
    }
    let ms = if points.len() == 256 {
        Vec::new()
    } else {
        monotone_tangents(&xs, &ys)
    };
    Ok(Curve { xs, ys, ms })
}

fn curve_unit(c: &Curve, u: f64) -> f64 {
    let code = u.clamp(0.0, 1.0) * 255.0;
    if c.ms.is_empty() {
        let x = code as usize;
        let left = c.ys[x];
        let right = c.ys[(x + 1).min(255)];
        return (left + (right - left) * (code - x as f64)) / 255.0;
    }
    hermite_eval(&c.xs, &c.ys, &c.ms, code) / 255.0
}

pub(super) fn curves_native_rgb(
    p: &CurvesParams,
    samples: &mut Samples,
    width: usize,
    height: usize,
    channels: u8,
) -> Result<(), AdjustError> {
    let composite = curve(&p.points)?;
    let red = p.red.as_deref().map(curve).transpose()?;
    let green = p.green.as_deref().map(curve).transpose()?;
    let blue = p.blue.as_deref().map(curve).transpose()?;
    if red.is_some() || green.is_some() || blue.is_some() {
        map(samples, width, height, channels, |[r, g, b]| {
            [
                red.as_ref().map_or(r, |c| curve_unit(c, r)),
                green.as_ref().map_or(g, |c| curve_unit(c, g)),
                blue.as_ref().map_or(b, |c| curve_unit(c, b)),
            ]
        });
    }
    map(samples, width, height, channels, |rgb| {
        rgb.map(|v| curve_unit(&composite, v))
    });
    Ok(())
}

pub(crate) fn curve_map(
    p: &CurvesParams,
    mode: ColorBalanceMode,
) -> Result<Option<impl Fn(Rgb) -> Rgb + Sync + Send>, AdjustError> {
    let active = |points: &[(u8, u8)]| {
        if points == [(0, 0), (255, 255)]
            || (points.len() == 256
                && points
                    .iter()
                    .enumerate()
                    .all(|(i, &(x, y))| i == usize::from(x) && x == y))
        {
            Ok(None)
        } else {
            curve(points).map(Some)
        }
    };
    let composite = active(&p.points)?;
    let red = p.red.as_deref().map(active).transpose()?.flatten();
    let green = p.green.as_deref().map(active).transpose()?.flatten();
    let blue = p.blue.as_deref().map(active).transpose()?.flatten();
    let black = p.black.as_deref().map(active).transpose()?.flatten();
    if composite.is_none()
        && red.is_none()
        && green.is_none()
        && blue.is_none()
        && (black.is_none() || mode != ColorBalanceMode::Cmyk)
    {
        return Ok(None);
    }

    let eval = |curve: &Option<Curve>, x| curve.as_ref().map_or(x, |c| curve_unit(c, x));
    Ok(Some(move |rgb: Rgb| {
        let component = |curve: &Option<Curve>, x| eval(&composite, eval(curve, x));
        match mode {
            ColorBalanceMode::Rgb => [
                component(&red, rgb[0]),
                component(&green, rgb[1]),
                component(&blue, rgb[2]),
            ],
            ColorBalanceMode::Lab => {
                let [l, a, b] = model::srgb_unit_to_lab(rgb);
                let l = component(&red, l / 100.0) * 100.0;
                let a = eval(&green, (a + 128.0) / 255.0) * 255.0 - 128.0;
                let b = eval(&blue, (b + 128.0) / 255.0) * 255.0 - 128.0;
                model::lab_to_srgb_unit([l, a, b])
            }
            ColorBalanceMode::Cmyk => {
                let rgb = rgb.map(|x| x.clamp(0.0, 1.0));
                let scale = rgb[0].max(rgb[1]).max(rgb[2]);
                let [c, m, y] = if scale > 0.0 {
                    rgb.map(|x| 1.0 - x / scale)
                } else {
                    [0.0; 3]
                };
                cmyk_unit_to_rgb([
                    component(&red, c),
                    component(&green, m),
                    component(&blue, y),
                    component(&black, 1.0 - scale),
                ])
            }
        }
    }))
}
