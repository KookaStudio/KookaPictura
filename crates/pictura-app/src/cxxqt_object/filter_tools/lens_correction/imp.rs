use super::*;

use std::sync::Mutex;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QImageFormat, QString, QStringList};
use pictura_codec::Exif;
use pictura_filters::{
    camera_models, catalog_makes, find_camera, find_camera_make, find_lens, lenses_for,
    match_lens_profiles, pick_subprofile, straighten_angle, LensCorrectionParams, LensMap,
    LensSource,
};

use super::super::super::helpers::{active_mask_target, active_pixel_layer};
use super::super::adaptive_wide_angle::exif_text;
use super::super::lens_profiles::{by_id, installed};
use super::super::lens_supported;

/// The kind's slot count; the order is the descriptor's.
pub(in crate::cxxqt_object) const SLOTS: usize = LensCorrectionParams::SLOTS;

/// The slots as engine parameters: the Auto tab's checkboxes gate the
/// profile's parts. `None` for a wrong count or a profile that is not
/// installed.
pub(super) fn params_of(values: &[f64]) -> Option<LensCorrectionParams> {
    LensCorrectionParams::from_slots(values)
}

/// EXIF's make, model, and lens in the catalogue's spelling where it knows
/// them ("NIKON CORPORATION", "NIKON D850" → "Nikon", "D850"), then the
/// camera settings.
pub(super) fn catalogue_names(
    make: &str,
    model: &str,
    lens: &str,
    settings: String,
) -> [String; 4] {
    let camera = find_camera(make, model);
    [
        camera
            .map(|c| c.make)
            .or_else(|| find_camera_make(make))
            .unwrap_or(make)
            .to_string(),
        camera.map_or(model, |c| c.model).to_string(),
        find_lens(lens).map_or(lens, |l| l.model).to_string(),
        settings,
    ]
}

/// "50mm, f/2.8, 3m" from FocalLength, FNumber, and SubjectDistance.
pub(super) fn camera_settings(exif: &Exif) -> String {
    let tag = |t: u16| {
        exif.get(t)
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v > 0.0)
            .map(|v| format!("{}", (v * 10.0).round() / 10.0))
            .unwrap_or_else(|| "--".into())
    };
    format!("{}mm, f/{}, {}m", tag(0x920a), tag(0x829d), tag(0x9206))
}

pub(super) struct Session {
    w: u32,
    h: u32,
    gray: bool,
    source: LensSource,
    slots: Vec<f64>,
    /// The layer's stored Lens Correction smart filter slots, for the dialog's
    /// prefill; empty when it carries none.
    stored: Vec<f64>,
    map: LensMap,
    exif: [String; 4],
    /// The photo's focal length in millimetres, f-number, and focus
    /// distance in metres, from EXIF.
    focal: Option<f64>,
    aperture: Option<f64>,
    distance: Option<f64>,
}

pub(super) static SESSION: Mutex<Option<Session>> = Mutex::new(None);
pub(super) static PREVIOUS: Mutex<Option<Vec<f64>>> = Mutex::new(None);

pub(super) fn with_session<R>(default: R, f: impl FnOnce(&mut Session) -> R) -> R {
    let mut guard = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().map_or(default, f)
}

pub(super) fn list<'a>(items: impl IntoIterator<Item = &'a str>) -> QStringList {
    let mut out: Vec<&str> = items.into_iter().filter(|s| !s.is_empty()).collect();
    out.sort_unstable();
    out.dedup();
    let mut list = QStringList::default();
    for s in out {
        list.append(QString::from(s));
    }
    list
}

pub(super) fn lc_begin(view: &PictureView) -> bool {
    let rust = view.rust();
    if active_mask_target(rust).is_some() {
        return false;
    }
    let Some(doc) = rust.session.doc.as_ref().filter(|d| lens_supported(d)) else {
        return false;
    };
    let Some(path) = rust.layer_target.single().map(str::to_string) else {
        return false;
    };
    let Some(layer) = active_pixel_layer(doc, Some(path.as_str())) else {
        return false;
    };
    let Some((w, h, color, alpha)) = session_source(doc, &path, layer) else {
        return false;
    };
    let exif = pictura_codec::read_metadata(doc).exif;
    let params = LensCorrectionParams::default();
    let session = Session {
        w,
        h,
        gray: doc.mode == pictura_core::ColorMode::Grayscale,
        source: LensSource::new(w, h, color, alpha),
        slots: Vec::new(),
        stored: pictura_render::lens_correction_slots(doc, &path).unwrap_or_default(),
        map: LensMap::new(&params, w, h),
        exif: catalogue_names(
            &exif_text(&exif, &[0x010f]),
            &exif_text(&exif, &[0x0110]),
            &exif_text(&exif, &[0xa434]),
            camera_settings(&exif),
        ),
        focal: exif_number(&exif, 0x920a),
        aperture: exif_number(&exif, 0x829d),
        distance: exif_number(&exif, 0x9206),
    };
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(session);
    true
}

/// A positive EXIF number.
pub(super) fn exif_number(exif: &Exif, tag: u16) -> Option<f64> {
    exif.get(tag)
        .and_then(|v| v.as_f64())
        .filter(|v| v.is_finite() && *v > 0.0)
}

/// The planes a dialog session reads: `(width, height, colour, alpha)`.
pub(super) type SessionPlanes = (
    u32,
    u32,
    [pictura_core::Plane<u8>; 3],
    Option<pictura_core::Plane<u8>>,
);

/// The planes a session reads for the layer at `path`: its embedded smart source
/// when it is a smart object, so a re-edit never re-applies the baked proxy, else
/// its own colour and alpha channels. `None` when a colour channel is missing.
pub(super) fn session_source(
    doc: &pictura_core::Document,
    path: &str,
    layer: &pictura_core::Layer,
) -> Option<SessionPlanes> {
    let (w, h) = (layer.rect.width(), layer.rect.height());
    if w <= 0 || h <= 0 {
        return None;
    }
    if layer
        .smart_object
        .as_ref()
        .is_some_and(|so| so.kind == pictura_core::SmartObjectKind::Embedded)
    {
        let buffer = pictura_render::smart_source_buffer(doc, path)?;
        let n = buffer.width as usize * buffer.height as usize;
        let plane = |c: usize| -> pictura_core::Plane<u8> {
            buffer.data[c * n..(c + 1) * n].to_vec().into()
        };
        let alpha = (buffer.channels == 4).then(|| plane(3));
        return Some((
            buffer.width,
            buffer.height,
            [plane(0), plane(1), plane(2)],
            alpha,
        ));
    }
    let plane = w as usize * h as usize;
    let channel = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id && c.data.len() == plane)
            .map(|c| c.data.clone())
    };
    let c0 = channel(0)?;
    let color = [
        c0.clone(),
        channel(1).unwrap_or_else(|| c0.clone()),
        channel(2).unwrap_or(c0),
    ];
    Some((w as u32, h as u32, color, channel(-1)))
}

pub(super) fn lc_geometry() -> Vec<i32> {
    with_session(Vec::new(), |s| vec![s.w as i32, s.h as i32])
}

pub(super) fn lc_grayscale() -> bool {
    with_session(false, |s| s.gray)
}

pub(super) fn lc_exif() -> QStringList {
    with_session(QStringList::default(), |s| {
        let mut out = QStringList::default();
        for text in &s.exif {
            out.append(QString::from(text.as_str()));
        }
        out
    })
}

pub(super) fn same(a: &str, b: &QString) -> bool {
    let b = b.to_string();
    b.trim().is_empty() || a.eq_ignore_ascii_case(b.trim())
}

/// `items` without repeats (ignoring case), in their order.
pub(super) fn ordered<'a>(items: impl IntoIterator<Item = &'a str>) -> QStringList {
    let mut seen = std::collections::HashSet::new();
    let mut out = QStringList::default();
    for s in items {
        if !s.is_empty() && seen.insert(s.to_lowercase()) {
            out.append(QString::from(s));
        }
    }
    out
}

pub(super) fn lc_makes() -> QStringList {
    let profiles = installed().iter().map(|p| p.make.as_str());
    list(catalog_makes().into_iter().chain(profiles))
}

pub(super) fn lc_models(make: &QString) -> QStringList {
    let make_s = make.to_string();
    let profiles = installed()
        .iter()
        .filter(|p| same(&p.make, make))
        .map(|p| p.model.as_str());
    list(camera_models(make_s.trim()).into_iter().chain(profiles))
}

pub(super) fn lc_lenses(make: &QString, model: &QString) -> QStringList {
    let (make_s, model_s) = (make.to_string(), model.to_string());
    let crop = find_camera(&make_s, &model_s).map(|c| c.crop);
    let focal = with_session(None, |s| s.focal);
    // A profile with no camera model suits any.
    let profiles = installed()
        .iter()
        .filter(|p| same(&p.make, make) && (p.model.is_empty() || same(&p.model, model)))
        .map(|p| p.lens.as_str());
    let catalogue = lenses_for(make_s.trim(), crop, focal);
    ordered(profiles.chain(catalogue.into_iter().map(|l| l.model)))
}

/// "24-70" or "50" from a range.
pub(super) fn span((lo, hi): (f64, f64)) -> String {
    if lo == hi {
        format!("{lo}")
    } else {
        format!("{lo}-{hi}")
    }
}

pub(super) fn criteria_info(make: &str, model: &str, lens: &str) -> String {
    let mut parts = Vec::new();
    if let Some(c) = find_camera(make, model) {
        let mut camera = format!("Crop {}×, {}", c.crop, c.sensor);
        if let Some(f) = c.fixed_focal {
            camera += &format!("; fixed lens {} mm (35 mm equiv.)", span(f));
            if let Some(a) = c.fixed_aperture {
                camera += &format!(" f/{}", span(a));
            }
        }
        parts.push(camera);
    }
    if let Some(l) = find_lens(lens) {
        let mut text = String::new();
        if let Some(f) = l.focal {
            text += &format!("{} mm", span(f));
        }
        if let Some(a) = l.aperture {
            text += &format!(" f/{}", span(a));
        }
        let calibrated: Vec<&str> = [
            (l.distortion, "distortion"),
            (l.tca, "TCA"),
            (l.vignetting, "vignetting"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
        .collect();
        let calibrated = if calibrated.is_empty() {
            "nothing".to_string()
        } else {
            calibrated.join(", ")
        };
        let text = format!("{text}; lensfun calibrates {calibrated} (crop {})", l.crop);
        parts.push(text.trim_start_matches("; ").to_string());
    }
    parts.join(" · ")
}

pub(super) fn lc_criteria_info(make: &QString, model: &QString, lens: &QString) -> QString {
    let info = criteria_info(
        make.to_string().trim(),
        model.to_string().trim(),
        lens.to_string().trim(),
    );
    QString::from(info.as_str())
}

pub(super) fn lc_match_profiles(
    make: &QString,
    model: &QString,
    lens: &QString,
    prefer_raw: bool,
) -> Vec<i32> {
    let (make, model) = (make.to_string(), model.to_string());
    let crop = find_camera(&make, &model).map(|c| c.crop);
    let all = installed();
    match_lens_profiles(all, &make, &model, &lens.to_string(), crop, prefer_raw)
        .into_iter()
        .map(|i| all[i].id())
        .collect()
}

pub(super) fn lc_profile_name(id: i32) -> QString {
    by_id(id).map_or_else(QString::default, |p| QString::from(p.name.as_str()))
}

pub(super) fn lc_subprofiles(id: i32) -> QStringList {
    let mut out = QStringList::default();
    for sub in by_id(id).map_or(&[][..], |p| p.subs.as_slice()) {
        out.append(QString::from(sub.label().as_str()));
    }
    out
}

pub(super) fn lc_auto_subprofile(id: i32) -> i32 {
    let Some(p) = by_id(id) else {
        return 0;
    };
    let (focal, aperture, distance) =
        with_session((None, None, None), |s| (s.focal, s.aperture, s.distance));
    pick_subprofile(&p.subs, focal, aperture, distance) as i32
}

/// The profile slots after the dialog's 16 for a resolved sub-profile.
pub(super) fn profile_slots(id: i32, sub: i32, make: &str, model: &str) -> Vec<f64> {
    let Some(s) = by_id(id).and_then(|p| p.subs.get(usize::try_from(sub).ok()?)) else {
        return Vec::new();
    };
    let crop = find_camera(make, model).map(|c| c.crop);
    let mut out = vec![f64::from(sub), 1.0];
    out.extend(s.resolve(crop).to_slots());
    out
}

pub(super) fn lc_profile_slots(id: i32, sub: i32, make: &QString, model: &QString) -> Vec<f64> {
    profile_slots(id, sub, &make.to_string(), &model.to_string())
}

pub(super) fn lc_set(values: &[f64]) -> bool {
    let Some(params) = params_of(values) else {
        return false;
    };
    with_session(false, |s| {
        s.map = LensMap::new(&params, s.w, s.h);
        s.slots = values.to_vec();
        true
    })
}

pub(super) fn lc_render(x: f64, y: f64, zoom: f64, w: i32, h: i32, preview: bool) -> QImage {
    if w <= 0 || h <= 0 || zoom <= 0.0 {
        return QImage::default();
    }
    with_session(QImage::default(), |s| {
        let (wu, hu) = (w as usize, h as usize);
        let mut bytes = vec![0u8; wu * hu * 4];
        let map = preview.then_some(&s.map);
        s.source.render(map, [x, y], zoom, (wu, hu), &mut bytes);
        // SAFETY: `bytes` is exactly w*h premultiplied RGBA8888 samples.
        unsafe { QImage::from_raw_bytes(bytes, w, h, QImageFormat::Format_RGBA8888_Premultiplied) }
    })
}

pub(super) fn lc_straighten(ax: f64, ay: f64, bx: f64, by: f64, angle: f64) -> f64 {
    if (ax, ay) == (bx, by) {
        return angle.rem_euclid(360.0);
    }
    let level = (angle - straighten_angle([ax, ay], [bx, by])).rem_euclid(360.0);
    (level * 100.0).round() / 100.0 % 360.0
}

pub(super) fn lc_previous() -> Vec<f64> {
    PREVIOUS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_default()
}

pub(super) fn lc_stored() -> Vec<f64> {
    with_session(Vec::new(), |s| s.stored.clone())
}

pub(super) fn lc_finish(keep: bool) -> Vec<f64> {
    let session = SESSION.lock().unwrap_or_else(|e| e.into_inner()).take();
    match session {
        Some(s) if keep && s.slots.len() == SLOTS => {
            *PREVIOUS.lock().unwrap_or_else(|e| e.into_inner()) = Some(s.slots.clone());
            s.slots
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_codec::parse_exif;
    use pictura_filters::{builtin_lens_profiles, LensEdge, LensModel};

    /// The descriptor's opening slots.
    fn defaults() -> Vec<f64> {
        pictura_codec::LENS_CORRECTION_DEFAULT_SLOTS.to_vec()
    }

    #[test]
    fn default_slots_are_the_identity() {
        let p = params_of(&defaults()).unwrap();
        assert!(p.is_identity());
        assert_eq!(p, LensCorrectionParams::default());
    }

    #[test]
    fn slots_map_and_clamp() {
        let mut v = defaults();
        v[4] = 2.0;
        v[6] = 250.0;
        v[7] = -12.5;
        v[11] = 120.0;
        v[14] = 370.0;
        v[15] = 10.0;
        let p = params_of(&v).unwrap();
        assert_eq!(p.edge, LensEdge::Black);
        assert_eq!(p.distortion, 100.0);
        assert_eq!(p.fringe, [-12.5, 0.0, 0.0]);
        assert_eq!(p.vignette_midpoint, 100.0);
        assert!((p.angle - 10.0).abs() < 1e-9);
        assert_eq!(p.scale, 0.5);
        assert!(params_of(&v[..15]).is_none());
        v[6] = f64::NAN;
        assert!(params_of(&v).is_none());
    }

    #[test]
    fn unknown_profile_id_is_refused() {
        let mut v = defaults();
        v[5] = 1.0;
        assert!(params_of(&v).unwrap().profile.is_some());
        v[5] = 2.0;
        assert!(params_of(&v).is_none());
        // Stored coefficients map whatever the id.
        v[17] = 1.0;
        assert!(params_of(&v).unwrap().profile.is_some());
    }

    #[test]
    fn checkboxes_gate_the_profile_parts() {
        let full = builtin_lens_profiles()[0].subs[0].coefficients;
        let mut v = defaults();
        v[5] = 1.0;
        v[0] = 0.0;
        v[1] = 1.0;
        v[2] = 1.0;
        let p = params_of(&v).unwrap().profile.unwrap();
        assert_eq!(p.distortion, [0.0; 6]);
        assert_eq!(p.ca, full.ca);
        assert_eq!(p.vignette, full.vignette);
        v[1] = 0.0;
        v[2] = 0.0;
        assert_eq!(params_of(&v).unwrap().profile, Some(LensModel::NONE));
        assert!(params_of(&v).unwrap().is_identity());
    }

    #[test]
    fn camera_settings_text() {
        // IFD0 with the Exif IFD pointer; the Exif IFD with FocalLength
        // 50/1 and FNumber 28/10.
        let mut t = b"II*\0\x08\0\0\0".to_vec();
        let entry = |t: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: u32| {
            t.extend(tag.to_le_bytes());
            t.extend(kind.to_le_bytes());
            t.extend(count.to_le_bytes());
            t.extend(value.to_le_bytes());
        };
        t.extend(1u16.to_le_bytes());
        entry(&mut t, 0x8769, 4, 1, 26);
        t.extend(0u32.to_le_bytes());
        // Exif IFD at 26: 2 entries end at 56; the rationals at 56 and 64.
        t.extend(2u16.to_le_bytes());
        entry(&mut t, 0x920a, 5, 1, 56);
        entry(&mut t, 0x829d, 5, 1, 64);
        t.extend(0u32.to_le_bytes());
        for v in [50u32, 1, 28, 10] {
            t.extend(v.to_le_bytes());
        }
        assert_eq!(camera_settings(&parse_exif(&t)), "50mm, f/2.8, --m");
        assert_eq!(camera_settings(&Exif::default()), "--mm, f/--, --m");
    }

    #[test]
    fn exif_names_take_the_catalogue_spelling() {
        let names = catalogue_names(
            "NIKON CORPORATION",
            "NIKON D850",
            "Nikkor AF-S 24-70mm f/2.8E ED VR",
            String::new(),
        );
        assert_eq!(
            names[..3],
            ["Nikon", "D850", "Nikkor AF-S 24-70mm f/2.8E ED VR"]
        );
        // Unknown names pass through.
        let names = catalogue_names("ACME Cameras", "ACME One", "", String::new());
        assert_eq!(names[..3], ["ACME Cameras", "ACME One", ""]);
    }

    #[test]
    fn session_source_is_the_unfiltered_smart_source() {
        let mut payload_doc = pictura_core::Document::new(
            2,
            2,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        for i in 0..4 {
            payload_doc.composite.data[i] = 30;
            payload_doc.composite.data[4 + i] = 60;
            payload_doc.composite.data[8 + i] = 90;
        }
        let payload = pictura_codec::write_psd(&payload_doc).expect("payload writes");

        let mut layer = pictura_core::Layer {
            name: "Smart".into(),
            rect: pictura_core::PsdRect {
                top: 0,
                left: 0,
                bottom: 2,
                right: 2,
            },
            smart_object: Some(
                {
                    let mut so = pictura_core::SmartObject::default();
                    so.kind = pictura_core::SmartObjectKind::Embedded;
                    so
                }
                .with_payload(payload),
            ),
            ..Default::default()
        };
        // The baked proxy is the corrected (black) pixels, unlike the source.
        for id in 0..3i16 {
            layer.channels.push(pictura_core::Channel {
                id,
                data: vec![0u8; 4].into(),
            });
        }
        let mut doc = pictura_core::Document::new(
            2,
            2,
            pictura_core::ColorMode::Rgb,
            pictura_core::BitDepth::Eight,
        );
        doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);

        let (_, _, color, _) = session_source(&doc, "0", &doc.layers[0]).expect("source");
        assert_eq!(
            color[0].to_vec(),
            vec![30u8, 30, 30, 30],
            "the source, not the proxy"
        );
    }

    #[test]
    fn criteria_info_describes_camera_and_lens() {
        let info = criteria_info("Canon", "EOS 5D Mark III", "Canon EF 24-70mm f/2.8L II USM");
        assert!(
            info.starts_with("Crop 1×, 36 x 24 mm · 24-70 mm f/2.8"),
            "{info}"
        );
        assert!(
            info.contains("lensfun calibrates distortion, TCA, vignetting"),
            "{info}"
        );
        let compact = criteria_info("Acer", "CE-6430", "");
        assert!(
            compact.contains("fixed lens 34-102 mm (35 mm equiv.) f/2.8-4.8"),
            "{compact}"
        );
        assert_eq!(criteria_info("ACME", "One", ""), "");
    }

    #[test]
    fn straighten_returns_the_levelling_angle() {
        // A line leaning 10° clockwise over an unrotated view.
        let t = 10f64.to_radians();
        let a = lc_straighten(0.0, 0.0, 100.0 * t.cos(), 100.0 * t.sin(), 0.0);
        assert!((a - 350.0).abs() < 1e-9, "{a}");
        assert!((lc_straighten(0.0, 0.0, 100.0, 0.0, 15.0) - 15.0).abs() < 1e-9);
        assert_eq!(lc_straighten(5.0, 5.0, 5.0, 5.0, 20.0), 20.0);
    }
}
