//! The Blur Gallery kinds' variable slot lists: a pin count, a fixed-stride
//! record per pin, then the shared 13-slot effects tail (bokeh
//! enabled/light/color/range, noise
//! enabled/amount/size/roughness/color/highlights/kind/seed). The arity
//! table's canonical list is one pin plus the tail; its all-defaults form
//! (count 0) and an empty list both mean one default pin.

use super::choices::BLUR_NOISE_KINDS;
use super::{f, flag, pick, u32v, u64v, u8v};
use pictura_filters::{BlurNoise, BlurPin, BokehParams, Filter, IrisPin, MAX_FIELD_PINS};

pub(in crate::cxxqt_object) const GALLERY_TAIL: usize = 13;
/// Field Blur's pin record: `(x, y, blur)`.
const FIELD_PIN_SLOTS: usize = 3;
pub(in crate::cxxqt_object) const FIELD_BLUR_SLOTS: usize = 1 + FIELD_PIN_SLOTS + GALLERY_TAIL;
/// Iris Blur's pin record: `(x, y, radius x, radius y, angle, squareness,
/// feather, blur)`.
const IRIS_PIN_SLOTS: usize = 8;
pub(in crate::cxxqt_object) const IRIS_BLUR_SLOTS: usize = 1 + IRIS_PIN_SLOTS + GALLERY_TAIL;

const FIELD_DEFAULT_PIN: BlurPin = BlurPin {
    x: 0.5,
    y: 0.5,
    blur: 0.0,
};

pub(super) fn field_blur(params: &[f64]) -> Option<Filter> {
    let (pins, bokeh, noise) = gallery(params, FIELD_PIN_SLOTS, FIELD_DEFAULT_PIN, |s| BlurPin {
        x: f(s, 0, 0.5),
        y: f(s, 1, 0.5),
        blur: f(s, 2, 0.0),
    })?;
    Some(Filter::FieldBlur { pins, bokeh, noise })
}

pub(super) fn iris_blur(params: &[f64]) -> Option<Filter> {
    let d = IrisPin::default();
    let (pins, bokeh, noise) = gallery(params, IRIS_PIN_SLOTS, d, |s| IrisPin {
        x: f(s, 0, d.x),
        y: f(s, 1, d.y),
        radius_x: f(s, 2, d.radius_x),
        radius_y: f(s, 3, d.radius_y),
        angle: f(s, 4, d.angle),
        squareness: f(s, 5, d.squareness),
        feather: f(s, 6, d.feather),
        blur: f(s, 7, d.blur),
    })?;
    Some(Filter::IrisBlur { pins, bokeh, noise })
}

/// Split a gallery slot list into its pins and effects; `None` for a list
/// whose length does not match its count, or more pins than the cap.
fn gallery<P>(
    params: &[f64],
    stride: usize,
    default_pin: P,
    read: impl Fn(&[f64]) -> P,
) -> Option<(Vec<P>, BokehParams, BlurNoise)> {
    if params.is_empty() {
        return Some((
            vec![default_pin],
            BokehParams::default(),
            BlurNoise::default(),
        ));
    }
    let count = u32v(params, 0, 1) as usize;
    if count > MAX_FIELD_PINS {
        return None;
    }
    let canonical = params.len() == 1 + stride + GALLERY_TAIL && count == 0;
    let tail = if canonical {
        1 + stride
    } else {
        1 + stride * count
    };
    if !canonical && params.len() != tail + GALLERY_TAIL {
        return None;
    }
    let pins = if canonical {
        vec![default_pin]
    } else {
        params[1..tail].chunks_exact(stride).map(read).collect()
    };
    let (low, high) = (u8v(params, tail + 3, 40), u8v(params, tail + 4, 118));
    let bokeh = BokehParams {
        enabled: flag(params, tail, true),
        light_bokeh: f(params, tail + 1, 14.0),
        bokeh_color: f(params, tail + 2, 40.0),
        light_range: if low <= high {
            (low, high)
        } else {
            (high, low)
        },
    };
    let noise = BlurNoise {
        enabled: flag(params, tail + 5, false),
        amount: f(params, tail + 6, 0.0),
        size: f(params, tail + 7, 50.0),
        roughness: f(params, tail + 8, 50.0),
        color: f(params, tail + 9, 0.0),
        highlights: f(params, tail + 10, 0.0),
        kind: pick(&BLUR_NOISE_KINDS, params, tail + 11, 0),
        seed: u64v(params, tail + 12, 1),
    };
    Some((pins, bokeh, noise))
}
