//! Lens profiles: a small in-tree text format, the bundled set, the Auto
//! Correction tab's matching and order, and the sub-profile pick. Adobe's
//! `.lcp` profiles are licensed assets and stay out of the tree; lensfun's
//! database is read at runtime (`lensfun.rs`).
//!
//! ```text
//! # comment
//! [profile]
//! name = ACME 50mm
//! make = ACME Cameras
//! model = ACME One
//! lens = ACME 50mm f/1.8
//! raw = yes                  # optional; Prefer RAW Profiles ranks it first
//! distortion = -0.08 0 0     # k1 k2 k3
//! ca = 1.0004 1 0.9996       # red, green, blue radial scale
//! vignette = -0.3 0 0        # v1 v2 v3
//! [sub]                      # optional sub-profiles, each starting from
//! focal = 50                 # the profile's coefficients
//! aperture = 2.8
//! distance = 3
//! vignette = -0.4 0 0
//! ```

use std::sync::OnceLock;

use super::catalog::fold;
use super::LensModel;

/// One calibration of a profile: the shot settings it was measured at
/// (each optional) and its coefficients.
#[derive(Debug, Clone, PartialEq)]
pub struct SubProfile {
    /// Millimetres.
    pub focal: Option<f64>,
    /// f-number.
    pub aperture: Option<f64>,
    /// Focus distance in metres.
    pub distance: Option<f64>,
    /// The calibration camera's crop factor; `None` when the coefficients
    /// apply to any image as they are.
    pub crop: Option<f64>,
    /// On the calibration image's half-diagonal.
    pub coefficients: LensModel,
}

/// One profile and the camera it was measured on. An empty `model` suits
/// any camera (lensfun profiles describe a lens alone).
#[derive(Debug, Clone, PartialEq)]
pub struct LensProfile {
    pub name: String,
    pub make: String,
    pub model: String,
    pub lens: String,
    /// Measured on raw files; Prefer RAW Profiles ranks these first.
    pub raw: bool,
    /// Never empty.
    pub subs: Vec<SubProfile>,
}

/// One decimal, no trailing zeros: "2.8", "50".
fn short(v: f64) -> String {
    format!("{}", (v * 10.0).round() / 10.0)
}

impl SubProfile {
    /// A plain sub-profile with `coefficients` and no shot settings.
    pub fn plain(coefficients: LensModel) -> Self {
        Self {
            focal: None,
            aperture: None,
            distance: None,
            crop: None,
            coefficients,
        }
    }

    /// "24 mm, f/2.8, 3 m" with the parts it has; "Default" with none.
    pub fn label(&self) -> String {
        let parts: Vec<String> = [
            self.focal.map(|f| format!("{} mm", short(f))),
            self.aperture.map(|a| format!("f/{}", short(a))),
            self.distance.map(|d| format!("{} m", short(d))),
        ]
        .into_iter()
        .flatten()
        .collect();
        if parts.is_empty() {
            "Default".into()
        } else {
            parts.join(", ")
        }
    }

    /// The coefficients for a photo taken on a camera of crop factor
    /// `camera_crop`, rescaled from the calibration's when both are known.
    pub fn resolve(&self, camera_crop: Option<f64>) -> LensModel {
        match (self.crop, camera_crop) {
            (Some(cal), Some(cam)) if cal > 0.0 && cam > 0.0 => {
                self.coefficients.rescaled(cal / cam)
            }
            _ => self.coefficients,
        }
    }
}

impl LensProfile {
    /// A stable id in `2³⁰ … 2³¹ − 1`: FNV-1a over the name, make, model,
    /// and lens. Never 1, the bundled profile's id before ids were hashed.
    pub fn id(&self) -> i32 {
        let mut h: u32 = 0x811c_9dc5;
        for part in [&self.name, &self.make, &self.model, &self.lens] {
            for b in part.bytes().chain([0]) {
                h ^= u32::from(b);
                h = h.wrapping_mul(0x0100_0193);
            }
        }
        ((h & 0x3fff_ffff) | 0x4000_0000) as i32
    }

    /// The calibration crop factor the list sorts by (the first
    /// sub-profile's).
    pub fn crop(&self) -> Option<f64> {
        self.subs.first().and_then(|s| s.crop)
    }
}

/// A profile being parsed: its fields, the coefficients `[sub]` blocks
/// start from, and the sub-profiles so far.
struct Draft {
    profile: LensProfile,
    defaults: LensModel,
}

/// Every `[profile]` block in `text`, or the first error with its line.
pub fn parse_profiles(text: &str) -> Result<Vec<LensProfile>, String> {
    let mut out: Vec<Draft> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let err = |what: &str| format!("line {line_no}: {what}");
        if line == "[profile]" {
            out.push(Draft {
                profile: LensProfile {
                    name: String::new(),
                    make: String::new(),
                    model: String::new(),
                    lens: String::new(),
                    raw: false,
                    subs: Vec::new(),
                },
                defaults: LensModel::NONE,
            });
            continue;
        }
        let Some(draft) = out.last_mut() else {
            return Err(err("expected [profile]"));
        };
        if line == "[sub]" {
            let sub = SubProfile::plain(draft.defaults);
            draft.profile.subs.push(sub);
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(err("expected key = value"));
        };
        let (key, value) = (key.trim(), value.trim());
        let triple = || -> Result<[f64; 3], String> {
            let v: Vec<f64> = value
                .split_whitespace()
                .map(str::parse::<f64>)
                .collect::<Result<_, _>>()
                .map_err(|_| err("expected three numbers"))?;
            match v.as_slice() {
                [a, b, c] if v.iter().all(|x| x.is_finite()) => Ok([*a, *b, *c]),
                _ => Err(err("expected three numbers")),
            }
        };
        let number = || -> Result<f64, String> {
            value
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && *v > 0.0)
                .ok_or_else(|| err("expected a positive number"))
        };
        let in_sub = !draft.profile.subs.is_empty();
        let model = match draft.profile.subs.last_mut() {
            Some(sub) => &mut sub.coefficients,
            None => &mut draft.defaults,
        };
        match key {
            "distortion" => model.distortion = LensModel::brown(triple()?),
            "ca" => model.ca = triple()?.map(|v| [v, 0.0, 0.0]),
            "vignette" => model.vignette = triple()?,
            "focal" | "aperture" | "distance" => {
                let v = number()?;
                let Some(sub) = draft.profile.subs.last_mut() else {
                    return Err(err(&format!("`{key}` belongs in a [sub]")));
                };
                match key {
                    "focal" => sub.focal = Some(v),
                    "aperture" => sub.aperture = Some(v),
                    _ => sub.distance = Some(v),
                }
            }
            "name" | "make" | "model" | "lens" | "raw" if in_sub => {
                return Err(err(&format!("`{key}` belongs before the first [sub]")))
            }
            "name" => draft.profile.name = value.to_string(),
            "make" => draft.profile.make = value.to_string(),
            "model" => draft.profile.model = value.to_string(),
            "lens" => draft.profile.lens = value.to_string(),
            "raw" => {
                draft.profile.raw = match value.to_ascii_lowercase().as_str() {
                    "yes" | "true" => true,
                    "no" | "false" => false,
                    _ => return Err(err("expected yes or no")),
                }
            }
            other => return Err(err(&format!("unknown key `{other}`"))),
        }
    }
    if let Some(i) = out
        .iter()
        .position(|d| d.profile.name.is_empty() || d.profile.make.is_empty())
    {
        return Err(format!("profile {} needs a name and a make", i + 1));
    }
    Ok(out
        .into_iter()
        .map(|mut d| {
            if d.profile.subs.is_empty() {
                d.profile.subs.push(SubProfile::plain(d.defaults));
            }
            d.profile
        })
        .collect())
}

/// The profiles that ship with the application.
pub fn builtin_profiles() -> &'static [LensProfile] {
    static PROFILES: OnceLock<Vec<LensProfile>> = OnceLock::new();
    PROFILES.get_or_init(|| {
        parse_profiles(include_str!("profiles.txt")).expect("bundled lens profiles parse")
    })
}

/// Indices into `profiles` that suit a camera, best first.
///
/// A camera-specific profile needs the make (case-insensitively); then the
/// model or the lens, when given, must match too, a lens match ranking above
/// a model match, and with neither given every profile of the make suits. A
/// profile with no model suits any camera: it matches by lens, or by maker
/// when no lens is given. Equal matches put raw profiles first with
/// `prefer_raw`, then the calibration crop factor nearest `crop` (CS6 sorts
/// by sensor size), then the name.
pub fn match_profiles(
    profiles: &[LensProfile],
    make: &str,
    model: &str,
    lens: &str,
    crop: Option<f64>,
    prefer_raw: bool,
) -> Vec<usize> {
    let (make, model, lens) = (make.trim(), model.trim(), lens.trim());
    if make.is_empty() {
        return Vec::new();
    }
    let hit = |a: &str, b: &str| !b.is_empty() && a.trim().eq_ignore_ascii_case(b);
    let lens_key = fold(lens);
    let score = |p: &LensProfile| -> Option<i32> {
        if p.model.trim().is_empty() {
            return if lens.is_empty() {
                hit(&p.make, make).then_some(1)
            } else {
                (fold(&p.lens) == lens_key).then_some(4)
            };
        }
        if !hit(&p.make, make) {
            return None;
        }
        let score = 2 * i32::from(hit(&p.model, model)) + 4 * i32::from(hit(&p.lens, lens));
        (score > 0 || (model.is_empty() && lens.is_empty())).then_some(score)
    };
    let distance = |p: &LensProfile| match (crop, p.crop()) {
        (None, _) => 0.0,
        (Some(c), Some(pc)) => (pc - c).abs(),
        (Some(_), None) => f64::INFINITY,
    };
    let mut hits: Vec<(i32, usize)> = profiles
        .iter()
        .enumerate()
        .filter_map(|(i, p)| score(p).map(|s| (s, i)))
        .collect();
    hits.sort_by(|a, b| {
        let (pa, pb) = (&profiles[a.1], &profiles[b.1]);
        b.0.cmp(&a.0)
            .then_with(|| (prefer_raw && pb.raw).cmp(&(prefer_raw && pa.raw)))
            .then_with(|| distance(pa).total_cmp(&distance(pb)))
            .then_with(|| pa.name.cmp(&pb.name))
    });
    hits.into_iter().map(|(_, i)| i).collect()
}

/// The sub-profile nearest a shot: by focal length (ties within 0.5 mm),
/// then f-stop in stops (ties within ⅙ stop), then focus distance by ratio.
/// A setting the shot lacks is skipped; a sub-profile lacking a setting the
/// shot has ranks after those that carry it; a full tie keeps the first.
/// Adobe's rule is closed: this is an approximation.
pub fn pick_subprofile(
    subs: &[SubProfile],
    focal: Option<f64>,
    aperture: Option<f64>,
    distance: Option<f64>,
) -> usize {
    let known = |v: Option<f64>| v.filter(|v| v.is_finite() && *v > 0.0);
    let cost = |shot: Option<f64>, sub: Option<f64>, gap: fn(f64, f64) -> f64, tie: f64| match (
        known(shot),
        known(sub),
    ) {
        (None, _) => 0.0,
        (Some(_), None) => f64::INFINITY,
        (Some(a), Some(b)) => (gap(a, b) / tie).floor(),
    };
    let key = |s: &SubProfile| {
        [
            cost(focal, s.focal, |a, b| (a - b).abs(), 0.5),
            cost(aperture, s.aperture, |a, b| (a / b).log2().abs(), 1.0 / 6.0),
            cost(distance, s.distance, |a, b| (a / b).ln().abs(), 1e-3),
        ]
    };
    subs.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let (ka, kb) = (key(a), key(b));
            ka[0]
                .total_cmp(&kb[0])
                .then(ka[1].total_cmp(&kb[1]))
                .then(ka[2].total_cmp(&kb[2]))
        })
        .map_or(0, |(i, _)| i)
}
