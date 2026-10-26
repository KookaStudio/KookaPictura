//! Lens profiles: a small in-tree text format, the bundled set, and the
//! Auto Correction tab's EXIF matching. Adobe's `.lcp` profiles are licensed
//! assets and stay out of the tree.
//!
//! ```text
//! # comment
//! [profile]
//! name = ACME 50mm
//! make = ACME Cameras
//! model = ACME One
//! lens = ACME 50mm f/1.8
//! distortion = -0.08 0 0     # k1 k2 k3
//! ca = 1.0004 1 0.9996       # red, green, blue radial scale
//! vignette = -0.3 0 0        # v1 v2 v3
//! ```

use std::sync::OnceLock;

use super::LensModel;

/// One profile and the camera it was measured on.
#[derive(Debug, Clone, PartialEq)]
pub struct LensProfile {
    pub name: String,
    pub make: String,
    pub model: String,
    pub lens: String,
    pub coefficients: LensModel,
}

/// Every `[profile]` block in `text`, or the first error with its line.
pub fn parse_profiles(text: &str) -> Result<Vec<LensProfile>, String> {
    let mut out: Vec<LensProfile> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line == "[profile]" {
            out.push(LensProfile {
                name: String::new(),
                make: String::new(),
                model: String::new(),
                lens: String::new(),
                coefficients: LensModel::NONE,
            });
            continue;
        }
        let err = |what: &str| format!("line {line_no}: {what}");
        let Some(profile) = out.last_mut() else {
            return Err(err("expected [profile]"));
        };
        let Some((key, value)) = line.split_once('=') else {
            return Err(err("expected key = value"));
        };
        let value = value.trim();
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
        match key.trim() {
            "name" => profile.name = value.to_string(),
            "make" => profile.make = value.to_string(),
            "model" => profile.model = value.to_string(),
            "lens" => profile.lens = value.to_string(),
            "distortion" => profile.coefficients.distortion = triple()?,
            "ca" => profile.coefficients.ca_scale = triple()?,
            "vignette" => profile.coefficients.vignette = triple()?,
            other => return Err(err(&format!("unknown key `{other}`"))),
        }
    }
    if let Some(i) = out
        .iter()
        .position(|p| p.name.is_empty() || p.make.is_empty())
    {
        return Err(format!("profile {} needs a name and a make", i + 1));
    }
    Ok(out)
}

/// The profiles that ship with the application.
pub fn builtin_profiles() -> &'static [LensProfile] {
    static PROFILES: OnceLock<Vec<LensProfile>> = OnceLock::new();
    PROFILES.get_or_init(|| {
        parse_profiles(include_str!("profiles.txt")).expect("bundled lens profiles parse")
    })
}

/// Indices into `profiles` that suit a camera, best first. The make must
/// match (case-insensitively); then the model or the lens, when given, must
/// match too, a lens match ranking above a model match. With neither given,
/// every profile of the make suits.
pub fn match_profiles(profiles: &[LensProfile], make: &str, model: &str, lens: &str) -> Vec<usize> {
    let (make, model, lens) = (make.trim(), model.trim(), lens.trim());
    if make.is_empty() {
        return Vec::new();
    }
    let hit = |a: &str, b: &str| !b.is_empty() && a.trim().eq_ignore_ascii_case(b);
    let mut hits: Vec<(i32, usize)> = profiles
        .iter()
        .enumerate()
        .filter(|(_, p)| hit(&p.make, make))
        .filter_map(|(i, p)| {
            let score = 2 * i32::from(hit(&p.model, model)) + 4 * i32::from(hit(&p.lens, lens));
            (score > 0 || (model.is_empty() && lens.is_empty())).then_some((score, i))
        })
        .collect();
    hits.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| profiles[a.1].name.cmp(&profiles[b.1].name))
    });
    hits.into_iter().map(|(_, i)| i).collect()
}
