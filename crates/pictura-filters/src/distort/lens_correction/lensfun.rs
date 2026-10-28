//! lensfun's XML database as lens profiles (#932). The database is CC BY-SA
//! 3.0, so none of it ships: the app reads the copy the system or the user
//! installed. Each `<lens>` gives one profile per `<calibration>`; its
//! models are converted exactly to the engine's radius over the half-diagonal.
//!
//! lensfun measures distortion and TCA with radius 1 at half the
//! calibration sensor's shorter side, and vignetting (`pa`) with radius 1 at
//! its corner (`libs/lensfun/modifier.cpp`); the sensor is the lens's
//! `<cropfactor>` and `<aspect-ratio>` (default 3:2), which a calibration's
//! attributes override. `ptlens` and `poly3` have the centre magnification
//! `d` divided out, as lensfun's `rescale_polynomial_coefficients` does.
//! `acm` and unknown models are skipped, and so are non-rectilinear lenses.

use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use super::profile::{LensProfile, SubProfile};
use super::LensModel;

/// One calibration entry as read: its element, focal length, and attributes.
struct Entry {
    kind: Kind,
    attrs: Vec<(String, String)>,
}

#[derive(Clone, Copy)]
enum Kind {
    Distortion,
    Tca,
    Vignetting,
}

#[derive(Default)]
struct Calibration {
    crop: Option<f64>,
    aspect: Option<f64>,
    entries: Vec<Entry>,
}

#[derive(Default)]
struct Lens {
    maker: Option<String>,
    maker_en: Option<String>,
    model: Option<String>,
    model_en: Option<String>,
    crop: Option<f64>,
    aspect: Option<f64>,
    kind: Option<String>,
    calibrations: Vec<Calibration>,
    /// A malformed field: the lens is skipped.
    bad: bool,
}

/// "1.5" or "3:2".
fn ratio(text: &str) -> Option<f64> {
    let v = match text.split_once(':') {
        Some((a, b)) => a.trim().parse::<f64>().ok()? / b.trim().parse::<f64>().ok()?,
        None => text.trim().parse().ok()?,
    };
    (v.is_finite() && v > 0.0).then_some(v)
}

fn attributes(e: &BytesStart) -> Result<Vec<(String, String)>, String> {
    e.attributes()
        .map(|a| {
            let a = a.map_err(|e| e.to_string())?;
            let key = a.key.as_ref().to_owned();
            let value = a
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|e| e.to_string())?
                .into_owned();
            Ok((key, value))
        })
        .collect()
}

/// Every rectilinear lens in one lensfun XML file with at least one model
/// the engine can apply. Malformed XML is an error; a lens with a bad
/// number is skipped.
pub fn parse_lensfun(text: &str) -> Result<Vec<LensProfile>, String> {
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<String> = Vec::new();
    let mut lens: Option<Lens> = None;
    // The lens field being read, its `lang`, and its text so far.
    let mut field: Option<(String, Option<String>, String)> = None;
    let mut out = Vec::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("at byte {}: {e}", reader.error_position()))?;
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let name = e.local_name().as_ref().to_owned();
                let empty = matches!(event, Event::Empty(_));
                let attrs = attributes(e)?;
                let parent = stack.last().map(String::as_str);
                match (parent, name.as_str()) {
                    (_, "lens") if lens.is_none() => lens = Some(Lens::default()),
                    (Some("lens"), "maker" | "model" | "cropfactor" | "aspect-ratio" | "type")
                        if !empty =>
                    {
                        let lang = attrs
                            .iter()
                            .find(|(k, _)| k == "lang")
                            .map(|(_, v)| v.clone());
                        field = Some((name.clone(), lang, String::new()));
                    }
                    (Some("lens"), "calibration") => {
                        if let Some(l) = lens.as_mut() {
                            let get = |key: &str| attrs.iter().find(|(k, _)| k == key);
                            let mut cal = Calibration::default();
                            if let Some((_, v)) = get("cropfactor") {
                                cal.crop = ratio(v);
                                l.bad |= cal.crop.is_none();
                            }
                            if let Some((_, v)) = get("aspect-ratio") {
                                cal.aspect = ratio(v);
                                l.bad |= cal.aspect.is_none();
                            }
                            l.calibrations.push(cal);
                        }
                    }
                    (Some("calibration"), "distortion" | "tca" | "vignetting") => {
                        let kind = match name.as_str() {
                            "distortion" => Kind::Distortion,
                            "tca" => Kind::Tca,
                            _ => Kind::Vignetting,
                        };
                        if let Some(cal) = lens.as_mut().and_then(|l| l.calibrations.last_mut()) {
                            cal.entries.push(Entry { kind, attrs });
                        }
                    }
                    _ => {}
                }
                if !empty {
                    stack.push(name);
                } else if name == "lens" {
                    out.extend(lens.take().map(finish).unwrap_or_default());
                }
            }
            Event::Text(t) => {
                if let Some((_, _, text)) = field.as_mut() {
                    text.push_str(&t.xml10_content());
                }
            }
            Event::GeneralRef(r) => {
                if let Some((_, _, text)) = field.as_mut() {
                    let name = r.xml10_content();
                    match r.resolve_char_ref().map_err(|e| e.to_string())? {
                        Some(c) => text.push(c),
                        None => text.push_str(resolve_predefined_entity(&name).unwrap_or("")),
                    }
                }
            }
            Event::End(_) => {
                let name = stack.pop().unwrap_or_default();
                if let Some((field_name, lang, text)) = field.take_if(|f| f.0 == name) {
                    if let Some(l) = lens.as_mut() {
                        store(l, &field_name, lang.as_deref(), text.trim());
                    }
                }
                if name == "lens" {
                    out.extend(lens.take().map(finish).unwrap_or_default());
                }
            }
            Event::Eof if stack.is_empty() => break,
            Event::Eof => return Err(format!("unclosed <{}>", stack.join("><"))),
            _ => {}
        }
    }
    Ok(out)
}

fn store(l: &mut Lens, field: &str, lang: Option<&str>, text: &str) {
    let text = text.to_string();
    match (field, lang) {
        ("maker", None) => l.maker = Some(text),
        ("maker", Some("en")) => l.maker_en = Some(text),
        ("model", None) => l.model = Some(text),
        ("model", Some("en")) => l.model_en = Some(text),
        ("cropfactor", _) => {
            l.crop = ratio(&text);
            l.bad |= l.crop.is_none();
        }
        ("aspect-ratio", _) => {
            l.aspect = ratio(&text);
            l.bad |= l.aspect.is_none();
        }
        ("type", _) => l.kind = Some(text),
        _ => {}
    }
}

/// A vignetting entry: focal length, aperture, distance, and `k1 k2 k3`.
type Vignette = (f64, Option<f64>, Option<f64>, [f64; 3]);

/// A converted entry: `None` for a model the engine does not apply,
/// `Err` for a malformed number.
type Converted<T> = Result<Option<T>, ()>;

fn num(attrs: &[(String, String)], key: &str, default: Option<f64>) -> Result<f64, ()> {
    match attrs.iter().find(|(k, _)| k == key) {
        Some((_, v)) => v
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or(()),
        None => default.ok_or(()),
    }
}

fn model_of(attrs: &[(String, String)]) -> &str {
    attrs
        .iter()
        .find(|(k, _)| k == "model")
        .map_or("", |(_, v)| v.as_str())
}

/// Distortion terms `c1…c6` on the half-diagonal; `s` is the half-diagonal
/// in lensfun's units (half the shorter side).
fn distortion(attrs: &[(String, String)], s: f64) -> Converted<[f64; 6]> {
    let k = |key| num(attrs, key, Some(0.0));
    let out = match model_of(attrs) {
        "poly3" => {
            let k1 = k("k1")?;
            let d = 1.0 - k1;
            if d.abs() < 1e-9 {
                return Err(());
            }
            [0.0, k1 * s * s / d.powi(3), 0.0, 0.0, 0.0, 0.0]
        }
        "poly5" => [0.0, k("k1")? * s * s, 0.0, k("k2")? * s.powi(4), 0.0, 0.0],
        "ptlens" => {
            let (a, b, c) = (k("a")?, k("b")?, k("c")?);
            let d = 1.0 - a - b - c;
            if d.abs() < 1e-9 {
                return Err(());
            }
            [
                c * s / d.powi(2),
                b * s * s / d.powi(3),
                a * s.powi(3) / d.powi(4),
                0.0,
                0.0,
                0.0,
            ]
        }
        _ => return Ok(None),
    };
    Ok(Some(out))
}

/// The three channels' `[v, c, b]` on the half-diagonal.
fn tca(attrs: &[(String, String)], s: f64) -> Converted<[[f64; 3]; 3]> {
    let k = |key, default| num(attrs, key, Some(default));
    let green = [1.0, 0.0, 0.0];
    let out = match model_of(attrs) {
        "linear" => [[k("kr", 1.0)?, 0.0, 0.0], green, [k("kb", 1.0)?, 0.0, 0.0]],
        "poly3" => [
            [k("vr", 1.0)?, k("cr", 0.0)? * s, k("br", 0.0)? * s * s],
            green,
            [k("vb", 1.0)?, k("cb", 0.0)? * s, k("bb", 0.0)? * s * s],
        ],
        _ => return Ok(None),
    };
    Ok(Some(out))
}

fn vignetting(attrs: &[(String, String)]) -> Converted<[f64; 3]> {
    if model_of(attrs) != "pa" {
        return Ok(None);
    }
    let k = |key| num(attrs, key, Some(0.0));
    Ok(Some([k("k1")?, k("k2")?, k("k3")?]))
}

/// The value at the focal length nearest `focal`.
fn nearest<T: Copy>(items: &[(f64, T)], focal: f64) -> Option<T> {
    items
        .iter()
        .min_by(|a, b| (a.0 - focal).abs().total_cmp(&(b.0 - focal).abs()))
        .map(|(_, v)| *v)
}

/// A finished `<lens>` as profiles, one per calibration; none when it is
/// malformed, not rectilinear, or has nothing the engine can apply.
fn finish(l: Lens) -> Vec<LensProfile> {
    let rectilinear = l.kind.as_deref().is_none_or(|k| k == "rectilinear");
    let (Some(maker), Some(model)) = (l.maker.or(l.maker_en), l.model.or(l.model_en)) else {
        return Vec::new();
    };
    if l.bad || !rectilinear {
        return Vec::new();
    }
    let mut out = Vec::new();
    let many = l.calibrations.len() > 1;
    for cal in &l.calibrations {
        let crop = cal.crop.or(l.crop);
        let s = cal.aspect.or(l.aspect).unwrap_or(1.5).hypot(1.0);
        let mut dist: Vec<(f64, [f64; 6])> = Vec::new();
        let mut ca: Vec<(f64, [[f64; 3]; 3])> = Vec::new();
        let mut vig: Vec<Vignette> = Vec::new();
        for e in &cal.entries {
            let Ok(focal) = num(&e.attrs, "focal", None) else {
                return Vec::new();
            };
            let opt = |key| num(&e.attrs, key, None).ok().filter(|v| *v > 0.0);
            let converted = match e.kind {
                Kind::Distortion => {
                    distortion(&e.attrs, s).map(|v| dist.extend(v.map(|v| (focal, v))))
                }
                Kind::Tca => tca(&e.attrs, s).map(|v| ca.extend(v.map(|v| (focal, v)))),
                Kind::Vignetting => vignetting(&e.attrs)
                    .map(|v| vig.extend(v.map(|v| (focal, opt("aperture"), opt("distance"), v)))),
            };
            if converted.is_err() {
                return Vec::new();
            }
        }
        let sub = |focal: f64, aperture, distance, vignette| SubProfile {
            focal: Some(focal),
            aperture,
            distance,
            crop,
            coefficients: LensModel {
                distortion: nearest(&dist, focal).unwrap_or([0.0; 6]),
                ca: nearest(&ca, focal).unwrap_or(LensModel::NONE.ca),
                vignette,
            },
        };
        let mut subs: Vec<SubProfile> = vig.iter().map(|&(f, a, d, v)| sub(f, a, d, v)).collect();
        let mut focals: Vec<f64> = dist
            .iter()
            .map(|e| e.0)
            .chain(ca.iter().map(|e| e.0))
            .collect();
        focals.sort_by(f64::total_cmp);
        focals.dedup();
        for f in focals {
            if !vig.iter().any(|v| v.0 == f) {
                subs.push(sub(f, None, None, [0.0; 3]));
            }
        }
        if subs.is_empty() {
            continue;
        }
        let key = |s: &SubProfile| (s.focal, s.aperture, s.distance);
        subs.sort_by(|a, b| {
            let (a, b) = (key(a), key(b));
            let o = |x: Option<f64>| x.unwrap_or(0.0);
            o(a.0)
                .total_cmp(&o(b.0))
                .then(o(a.1).total_cmp(&o(b.1)))
                .then(o(a.2).total_cmp(&o(b.2)))
        });
        let mut name = if model.to_lowercase().starts_with(&maker.to_lowercase()) {
            model.clone()
        } else {
            format!("{maker} {model}")
        };
        if many {
            if let Some(c) = crop {
                name += &format!(" (crop {c})");
            }
        }
        out.push(LensProfile {
            name,
            make: maker.clone(),
            model: String::new(),
            lens: model.clone(),
            raw: false,
            subs,
        });
    }
    out
}
