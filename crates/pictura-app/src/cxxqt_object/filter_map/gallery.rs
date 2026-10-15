//! The Blur Gallery kinds' variable slot lists: a pin count, a fixed-stride
//! record per pin (Path Blur's records carry their own point count, then its
//! six motion slots), then the shared 13-slot effects tail (bokeh
//! enabled/light/color/range, noise
//! enabled/amount/size/roughness/color/highlights/kind/seed). The arity
//! table's canonical list is one pin plus the tail; its all-defaults form
//! (count 0) and an empty list both mean one default pin.

use super::choices::BLUR_NOISE_KINDS;
use super::{f, flag, pick, u32v, u64v, u8v};
use pictura_filters::{
    BlurNoise, BlurPath, BlurPin, BokehParams, Filter, IrisPin, PathMotion, TiltPin,
    MAX_FIELD_PINS, MAX_PATH_POINTS,
};

pub(in crate::cxxqt_object) const GALLERY_TAIL: usize = 13;
/// Field Blur's pin record: `(x, y, blur)`.
const FIELD_PIN_SLOTS: usize = 3;
pub(in crate::cxxqt_object) const FIELD_BLUR_SLOTS: usize = 1 + FIELD_PIN_SLOTS + GALLERY_TAIL;
/// Iris Blur's pin record: `(x, y, radius x, radius y, angle, squareness,
/// feather, blur)`.
const IRIS_PIN_SLOTS: usize = 8;
pub(in crate::cxxqt_object) const IRIS_BLUR_SLOTS: usize = 1 + IRIS_PIN_SLOTS + GALLERY_TAIL;
/// Tilt-Shift's pin record: `(x, y, angle, focus above, focus below, fade
/// above, fade below, distortion, symmetric, blur)`.
const TILT_PIN_SLOTS: usize = 10;
pub(in crate::cxxqt_object) const TILT_SHIFT_SLOTS: usize = 1 + TILT_PIN_SLOTS + GALLERY_TAIL;
/// Path Blur's canonical path record: `(point count, (x, y)×2, start speed,
/// end speed)`.
const PATH_RECORD_SLOTS: usize = 1 + 2 * 2 + 2;
/// Path Blur's motion section: `(speed, taper, centered, rear sync, strobe
/// strength, strobe flashes)`.
const PATH_MOTION_SLOTS: usize = 6;
pub(in crate::cxxqt_object) const PATH_BLUR_SLOTS: usize =
    1 + PATH_RECORD_SLOTS + PATH_MOTION_SLOTS + GALLERY_TAIL;
/// The combined kind's canonical list: one record of each effect, the motion
/// section, and the tail.
pub(in crate::cxxqt_object) const BLUR_GALLERY_SLOTS: usize = 4
    + FIELD_PIN_SLOTS
    + IRIS_PIN_SLOTS
    + TILT_PIN_SLOTS
    + PATH_RECORD_SLOTS
    + PATH_MOTION_SLOTS
    + GALLERY_TAIL;

const FIELD_DEFAULT_PIN: BlurPin = BlurPin {
    x: 0.5,
    y: 0.5,
    blur: 0.0,
};

fn read_field(s: &[f64]) -> BlurPin {
    BlurPin {
        x: f(s, 0, 0.5),
        y: f(s, 1, 0.5),
        blur: f(s, 2, 0.0),
    }
}

fn read_iris(s: &[f64]) -> IrisPin {
    let d = IrisPin::default();
    IrisPin {
        x: f(s, 0, d.x),
        y: f(s, 1, d.y),
        radius_x: f(s, 2, d.radius_x),
        radius_y: f(s, 3, d.radius_y),
        angle: f(s, 4, d.angle),
        squareness: f(s, 5, d.squareness),
        feather: f(s, 6, d.feather),
        blur: f(s, 7, d.blur),
    }
}

fn read_tilt(s: &[f64]) -> TiltPin {
    let d = TiltPin::default();
    TiltPin {
        x: f(s, 0, d.x),
        y: f(s, 1, d.y),
        angle: f(s, 2, d.angle),
        focus_above: f(s, 3, d.focus_above),
        focus_below: f(s, 4, d.focus_below),
        fade_above: f(s, 5, d.fade_above),
        fade_below: f(s, 6, d.fade_below),
        distortion: f(s, 7, d.distortion),
        symmetric: flag(s, 8, d.symmetric),
        blur: f(s, 9, d.blur),
    }
}

pub(super) fn field_blur(params: &[f64]) -> Option<Filter> {
    let (pins, bokeh, noise) = gallery(params, FIELD_PIN_SLOTS, FIELD_DEFAULT_PIN, read_field)?;
    Some(Filter::FieldBlur { pins, bokeh, noise })
}

pub(super) fn iris_blur(params: &[f64]) -> Option<Filter> {
    let (pins, bokeh, noise) = gallery(params, IRIS_PIN_SLOTS, IrisPin::default(), read_iris)?;
    Some(Filter::IrisBlur { pins, bokeh, noise })
}

pub(super) fn tilt_shift(params: &[f64]) -> Option<Filter> {
    let (pins, bokeh, noise) = gallery(params, TILT_PIN_SLOTS, TiltPin::default(), read_tilt)?;
    Some(Filter::TiltShift { pins, bokeh, noise })
}

/// The list-level default path: the workspace's opening stroke with the
/// owner's 100 % screenshot's ~33 px blur shapes. The dialog sizes its own
/// opening speeds to the document instead.
fn default_path() -> BlurPath {
    BlurPath {
        points: vec![(0.5, 0.5), (0.58, 0.5)],
        start_speed: 33.0,
        end_speed: 33.0,
    }
}

/// The `path-blur` kind. An empty list and the canonical list with count 0
/// map to one default path.
pub(super) fn path_blur(params: &[f64]) -> Option<Filter> {
    if params.is_empty() {
        return Some(Filter::PathBlur {
            paths: vec![default_path()],
            motion: PathMotion::default(),
            noise: BlurNoise::default(),
        });
    }
    let canonical = params.len() == PATH_BLUR_SLOTS && params[0] == 0.0;
    let (paths, at) = if canonical {
        (vec![default_path()], 1 + PATH_RECORD_SLOTS)
    } else {
        let mut at = 0;
        (path_section(params, &mut at)?, at)
    };
    if params.len() != at + PATH_MOTION_SLOTS + GALLERY_TAIL {
        return None;
    }
    let (_, noise) = effects(params, at + PATH_MOTION_SLOTS);
    Some(Filter::PathBlur {
        paths,
        motion: read_motion(params, at),
        noise,
    })
}

/// A path count and its records starting at `*at`, advancing past them.
fn path_section(params: &[f64], at: &mut usize) -> Option<Vec<BlurPath>> {
    let count = whole(*params.get(*at)?, MAX_FIELD_PINS)?;
    *at += 1;
    let mut paths = Vec::with_capacity(count);
    for _ in 0..count {
        let points = whole(*params.get(*at)?, MAX_PATH_POINTS)?;
        if points < 2 {
            return None;
        }
        let record = params.get(*at + 1..*at + 1 + 2 * points + 2)?;
        paths.push(BlurPath {
            points: record[..2 * points]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[x, y]| (x, y))
                .collect(),
            start_speed: record[2 * points],
            end_speed: record[2 * points + 1],
        });
        *at += 1 + 2 * points + 2;
    }
    Some(paths)
}

/// A non-negative integral slot up to `max`.
fn whole(value: f64, max: usize) -> Option<usize> {
    (value >= 0.0 && value.fract() == 0.0 && value <= max as f64).then_some(value as usize)
}

fn read_motion(params: &[f64], at: usize) -> PathMotion {
    let d = PathMotion::default();
    PathMotion {
        speed: f(params, at, d.speed),
        taper: f(params, at + 1, d.taper),
        centered: flag(params, at + 2, d.centered),
        rear_sync: flag(params, at + 3, d.rear_sync),
        strobe_strength: f(params, at + 4, d.strobe_strength),
        strobe_flashes: u32v(params, at + 5, d.strobe_flashes),
    }
}

/// The combined `blur-gallery` kind: each effect's count and records in
/// turn (Field, Iris, Tilt-Shift, Path; a count of 0 turns it off), the
/// motion section, then the tail. An empty list and the canonical
/// all-defaults list (every count 0) map to one default Field pin.
pub(super) fn blur_gallery(params: &[f64]) -> Option<Filter> {
    let canonical_counts = [
        0,
        1 + FIELD_PIN_SLOTS,
        2 + FIELD_PIN_SLOTS + IRIS_PIN_SLOTS,
        3 + FIELD_PIN_SLOTS + IRIS_PIN_SLOTS + TILT_PIN_SLOTS,
    ];
    let defaults = params.is_empty()
        || (params.len() == BLUR_GALLERY_SLOTS
            && canonical_counts.iter().all(|&i| params[i] == 0.0));
    if defaults {
        let (bokeh, noise) = effects(params, BLUR_GALLERY_SLOTS - GALLERY_TAIL);
        return Some(Filter::BlurGallery {
            field: vec![FIELD_DEFAULT_PIN],
            iris: Vec::new(),
            tilt: Vec::new(),
            path: Vec::new(),
            motion: PathMotion::default(),
            bokeh,
            noise,
        });
    }
    let mut at = 0;
    let field = section(params, &mut at, FIELD_PIN_SLOTS, read_field)?;
    let iris = section(params, &mut at, IRIS_PIN_SLOTS, read_iris)?;
    let tilt = section(params, &mut at, TILT_PIN_SLOTS, read_tilt)?;
    let path = path_section(params, &mut at)?;
    if params.len() != at + PATH_MOTION_SLOTS + GALLERY_TAIL {
        return None;
    }
    let motion = read_motion(params, at);
    let (bokeh, noise) = effects(params, at + PATH_MOTION_SLOTS);
    Some(Filter::BlurGallery {
        field,
        iris,
        tilt,
        path,
        motion,
        bokeh,
        noise,
    })
}

/// One effect's count and records starting at `*at`, advancing past them.
fn section<P>(
    params: &[f64],
    at: &mut usize,
    stride: usize,
    read: impl Fn(&[f64]) -> P,
) -> Option<Vec<P>> {
    let count = *params.get(*at)?;
    if count < 0.0 || count.fract() != 0.0 || count > MAX_FIELD_PINS as f64 {
        return None;
    }
    let start = *at + 1;
    let end = start + stride * count as usize;
    let records = params.get(start..end)?;
    *at = end;
    Some(records.chunks_exact(stride).map(read).collect())
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
    let (bokeh, noise) = effects(params, tail);
    Some((pins, bokeh, noise))
}

/// The shared 13-slot effects tail starting at `tail`.
fn effects(params: &[f64], tail: usize) -> (BokehParams, BlurNoise) {
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
        enabled: flag(params, tail + 5, true),
        amount: f(params, tail + 6, 0.0),
        size: f(params, tail + 7, 50.0),
        roughness: f(params, tail + 8, 50.0),
        color: f(params, tail + 9, 0.0),
        highlights: f(params, tail + 10, 0.0),
        kind: pick(&BLUR_NOISE_KINDS, params, tail + 11, 0),
        seed: u64v(params, tail + 12, 1),
    };
    (bokeh, noise)
}
