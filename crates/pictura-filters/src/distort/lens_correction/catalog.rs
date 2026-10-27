//! The camera and lens catalogues behind the Auto Correction tab's Search
//! Criteria: names, crop factors, sensors, a lens's focal range and
//! aperture, and which corrections lensfun has calibrated for it. They carry
//! no correction data; profiles do (`profile.rs`). Both files are generated
//! by `scripts/gen-lens-catalog.py`; see `assets/PROVENANCE.md`.

use std::sync::OnceLock;

/// A low–high pair: millimetres or f-numbers.
pub type Span = (f64, f64);

/// A camera body.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraSpec {
    pub make: &'static str,
    pub model: &'static str,
    pub crop: f64,
    pub sensor: &'static str,
    /// A fixed lens's 35 mm equivalent range, in millimetres.
    pub fixed_focal: Option<Span>,
    pub fixed_aperture: Option<Span>,
}

/// A lens lensfun lists.
#[derive(Debug, Clone, PartialEq)]
pub struct LensSpec {
    pub make: &'static str,
    pub model: &'static str,
    /// The crop factor of the camera lensfun calibrated it on.
    pub crop: f64,
    /// The focal range read from the name, in millimetres.
    pub focal: Option<Span>,
    /// The maximum aperture (f-number) across the range, from the name.
    pub aperture: Option<Span>,
    pub distortion: bool,
    pub tca: bool,
    pub vignetting: bool,
}

fn rows(text: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').collect())
}

/// `"24-70"` or `"50"` as a range.
fn range(text: &str) -> Option<Span> {
    let mut parts = text.split('-').map(|p| p.trim().parse::<f64>().ok());
    let lo = parts.next()??;
    let hi = match parts.next() {
        Some(v) => v?,
        None => lo,
    };
    (parts.next().is_none() && lo > 0.0 && hi >= lo).then_some((lo, hi))
}

/// The run of digits, dots and dashes that ends at byte `end` of `s`.
fn run_before(s: &str, end: usize) -> &str {
    let head = &s[..end];
    let start = head
        .char_indices()
        .rev()
        .find(|&(_, c)| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .map_or(0, |(i, c)| i + c.len_utf8());
    head[start..].trim_matches('-')
}

/// The focal range and maximum aperture written in a lens name, e.g.
/// "Canon EF 24-70mm f/2.8L USM" → (24, 70), (2.8, 2.8).
pub fn parse_lens_name(name: &str) -> (Option<Span>, Option<Span>) {
    let focal = name
        .match_indices("mm")
        .find_map(|(i, _)| range(run_before(name, i)));
    let aperture = name.match_indices("f/").find_map(|(i, _)| {
        let tail = &name[i + 2..];
        let end = tail
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .unwrap_or(tail.len());
        range(tail[..end].trim_matches('-'))
    });
    (focal, aperture)
}

/// Every camera body, sorted by make and model.
pub fn cameras() -> &'static [CameraSpec] {
    static CAMERAS: OnceLock<Vec<CameraSpec>> = OnceLock::new();
    CAMERAS.get_or_init(|| {
        rows(include_str!("cameras.tsv"))
            .filter_map(|r| match r.as_slice() {
                [make, model, crop, sensor, focal, aperture] => Some(CameraSpec {
                    make,
                    model,
                    crop: crop.parse().ok()?,
                    sensor,
                    fixed_focal: range(focal),
                    fixed_aperture: range(aperture),
                }),
                _ => None,
            })
            .collect()
    })
}

/// Every lens, sorted by make and model.
pub fn lenses() -> &'static [LensSpec] {
    static LENSES: OnceLock<Vec<LensSpec>> = OnceLock::new();
    LENSES.get_or_init(|| {
        rows(include_str!("lenses.tsv"))
            .filter_map(|r| match r.as_slice() {
                [make, model, crop, flags] => {
                    let (focal, aperture) = parse_lens_name(model);
                    Some(LensSpec {
                        make,
                        model,
                        crop: crop.parse().ok()?,
                        focal,
                        aperture,
                        distortion: flags.contains('d'),
                        tca: flags.contains('t'),
                        vignetting: flags.contains('v'),
                    })
                }
                _ => None,
            })
            .collect()
    })
}

/// Lowercase with runs of spaces, dashes and underscores as one space.
pub(super) fn fold(s: &str) -> String {
    s.split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Every make either catalogue knows, sorted.
pub fn makes() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = cameras()
        .iter()
        .map(|c| c.make)
        .chain(lenses().iter().map(|l| l.make))
        .collect();
    out.sort_by_key(|m| m.to_lowercase());
    out.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    out
}

/// The catalogue's spelling of an EXIF make: "NIKON CORPORATION" → "Nikon",
/// "KONICA MINOLTA" → "Konica-Minolta". The longest make the EXIF text
/// starts with wins.
pub fn find_make(exif: &str) -> Option<&'static str> {
    let text = fold(exif);
    makes()
        .into_iter()
        .filter(|m| {
            let m = fold(m);
            text == m || text.starts_with(&format!("{m} "))
        })
        .max_by_key(|m| m.len())
}

/// A model name with its make's prefix dropped and folded, so "Canon EOS
/// 5D Mark III" and "EOS 5D Mark III" compare equal.
fn model_key(make: &str, model: &str) -> String {
    let (m, model) = (fold(make), fold(model));
    match model.strip_prefix(&format!("{m} ")) {
        Some(rest) => rest.to_string(),
        None => model,
    }
}

/// The camera an EXIF make and model name, if the catalogue has it.
pub fn find_camera(make: &str, model: &str) -> Option<&'static CameraSpec> {
    let make = find_make(make)?;
    let key = model_key(make, model);
    cameras()
        .iter()
        .find(|c| c.make == make && model_key(make, c.model) == key)
}

/// The lens an EXIF lens name (any make).
pub fn find_lens(lens: &str) -> Option<&'static LensSpec> {
    let key = fold(lens);
    if key.is_empty() {
        return None;
    }
    lenses().iter().find(|l| fold(l.model) == key)
}

/// The camera models of `make`, sorted.
pub fn camera_models(make: &str) -> Vec<&'static str> {
    cameras()
        .iter()
        .filter(|c| c.make.eq_ignore_ascii_case(make))
        .map(|c| c.model)
        .collect()
}

/// Lenses for a camera, best first: `make`'s own lenses, then every other
/// make's (lensfun lists no mounts). With a focal length (the photo's EXIF
/// setting) only lenses whose range covers it are kept. Within each group
/// the lens calibrated on the nearest crop factor comes first (CS6 sorts
/// its list by sensor size), then by name.
pub fn lenses_for(make: &str, crop: Option<f64>, focal: Option<f64>) -> Vec<&'static LensSpec> {
    let covers = |l: &LensSpec| match (focal, l.focal) {
        (Some(f), Some((lo, hi))) => f >= lo - 0.5 && f <= hi + 0.5,
        (Some(_), None) => false,
        (None, _) => true,
    };
    let mut out: Vec<&'static LensSpec> = lenses().iter().filter(|l| covers(l)).collect();
    let foreign = |l: &LensSpec| !l.make.eq_ignore_ascii_case(make);
    let distance = |l: &LensSpec| crop.map_or(0.0, |c| (l.crop - c).abs());
    out.sort_by(|a, b| {
        foreign(a)
            .cmp(&foreign(b))
            .then(distance(a).total_cmp(&distance(b)))
            .then_with(|| a.make.cmp(b.make))
            .then_with(|| a.model.cmp(b.model))
    });
    out
}
