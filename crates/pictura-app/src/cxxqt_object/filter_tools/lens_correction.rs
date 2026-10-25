//! Filter ▸ Lens Correction (#347): the dialog's editing session. The dialog
//! holds every setting as the `lens-correction` kind's slots and hands them
//! here for a preview at screen resolution; OK returns the slots, so the
//! commit, Last Filter and Last Filter Settings run the ordinary filter path.
//! Free functions (their own bridge); the dialog is modal, so one session at
//! a time. Points cross the bridge in layer pixels.

use std::sync::Mutex;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QImageFormat, QString, QStringList};
use pictura_codec::Exif;
use pictura_filters::{
    builtin_lens_profiles, match_lens_profiles, straighten_angle, Filter, LensCorrectionParams,
    LensEdge, LensMap, LensModel, LensSource,
};

use super::super::helpers::{active_mask_target, active_pixel_layer};
use super::super::qobject::PictureView;
use super::adaptive_wide_angle::exif_text;
use super::lens_supported;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Open a session over the active layer. False when the document is
        /// not 8- or 16-bit RGB or Grayscale or there is no unmasked pixel
        /// layer with colour data.
        fn lc_begin(view: &PictureView) -> bool;

        /// The session's layer size, `[width, height]`; empty with no session.
        fn lc_geometry() -> Vec<i32>;

        /// Whether the session's document is Grayscale (no fringe to fix).
        fn lc_grayscale() -> bool;

        /// What EXIF says: `[make, model, lens, camera settings]`, each
        /// empty when unknown; the settings read "50mm, f/2.8, 3m" with
        /// `--` for a missing part.
        fn lc_exif() -> QStringList;

        /// The installed profiles' makes, models of `make`, and lenses of
        /// `make` and `model` (empty criteria match all), sorted.
        fn lc_profile_makes() -> QStringList;
        fn lc_profile_models(make: &QString) -> QStringList;
        fn lc_profile_lenses(make: &QString, model: &QString) -> QStringList;

        /// Profile ids (1-based) that suit the criteria, best first.
        fn lc_match_profiles(make: &QString, model: &QString, lens: &QString) -> Vec<i32>;

        /// Profile `id`'s name, or empty when there is no such profile.
        fn lc_profile_name(id: i32) -> QString;

        /// Set the session's slots. False (and unchanged) when they do not
        /// map, e.g. a profile id that is not installed.
        fn lc_set(values: &[f64]) -> bool;

        /// A `w`×`h` premultiplied view whose pixel `(i, j)` shows layer
        /// point `(x + (i + 0.5) / zoom, y + (j + 0.5) / zoom)`, corrected
        /// with `preview`.
        fn lc_render(x: f64, y: f64, zoom: f64, w: i32, h: i32, preview: bool) -> QImage;

        /// The Angle (0..360) that levels or plumbs the line from `(ax, ay)`
        /// to `(bx, by)` drawn over a view rotated by `angle`.
        fn lc_straighten(ax: f64, ay: f64, bx: f64, by: f64, angle: f64) -> f64;

        /// The slots of the last OK (Previous Conversion); empty before one.
        fn lc_previous() -> Vec<f64>;

        /// End the session. With `keep`, remember the slots as Previous
        /// Conversion and return them; otherwise empty.
        fn lc_finish(keep: bool) -> Vec<f64>;
    }
}

/// The kind's slot count; the order is the descriptor's.
pub(in crate::cxxqt_object) const SLOTS: usize = 16;

/// The slots as engine parameters: the Auto tab's checkboxes gate the
/// profile's parts. `None` for a wrong count or a profile that is not
/// installed.
fn params_of(values: &[f64]) -> Option<LensCorrectionParams> {
    if values.len() != SLOTS || values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let on = |i: usize| values[i] >= 0.5;
    let id = values[5].round();
    let profile = if id <= 0.0 {
        None
    } else {
        let p = builtin_lens_profiles().get(id as usize - 1)?.coefficients;
        Some(LensModel {
            distortion: if on(0) { p.distortion } else { [0.0; 3] },
            ca_scale: if on(1) { p.ca_scale } else { [1.0; 3] },
            vignette: if on(2) { p.vignette } else { [0.0; 3] },
        })
    };
    let slider = |i: usize| values[i].clamp(-100.0, 100.0);
    Some(LensCorrectionParams {
        profile,
        distortion: slider(6),
        fringe: [slider(7), slider(8), slider(9)],
        vignette_amount: slider(10),
        vignette_midpoint: values[11].clamp(0.0, 100.0),
        vertical: slider(12),
        horizontal: slider(13),
        angle: values[14].rem_euclid(360.0),
        scale: values[15].clamp(50.0, 150.0) / 100.0,
        auto_scale: on(3),
        edge: match values[4].round() as i64 {
            0 => LensEdge::Extension,
            2 => LensEdge::Black,
            3 => LensEdge::White,
            _ => LensEdge::Transparency,
        },
    })
}

/// The `lens-correction` kind's filter for a full slot list.
pub(in crate::cxxqt_object) fn lens_correction_filter(values: &[f64]) -> Option<Filter> {
    params_of(values).map(|params| Filter::LensCorrection { params })
}

/// "50mm, f/2.8, 3m" from FocalLength, FNumber, and SubjectDistance.
fn camera_settings(exif: &Exif) -> String {
    let tag = |t: u16| {
        exif.get(t)
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite() && *v > 0.0)
            .map(|v| format!("{}", (v * 10.0).round() / 10.0))
            .unwrap_or_else(|| "--".into())
    };
    format!("{}mm, f/{}, {}m", tag(0x920a), tag(0x829d), tag(0x9206))
}

struct Session {
    w: u32,
    h: u32,
    gray: bool,
    source: LensSource,
    slots: Vec<f64>,
    map: LensMap,
    exif: [String; 4],
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);
static PREVIOUS: Mutex<Option<Vec<f64>>> = Mutex::new(None);

fn with_session<R>(default: R, f: impl FnOnce(&mut Session) -> R) -> R {
    let mut guard = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().map_or(default, f)
}

fn list<'a>(items: impl IntoIterator<Item = &'a str>) -> QStringList {
    let mut out: Vec<&str> = items.into_iter().filter(|s| !s.is_empty()).collect();
    out.sort_unstable();
    out.dedup();
    let mut list = QStringList::default();
    for s in out {
        list.append(QString::from(s));
    }
    list
}

fn lc_begin(view: &PictureView) -> bool {
    let rust = view.rust();
    if active_mask_target(rust).is_some() {
        return false;
    }
    let Some(doc) = rust.session.doc.as_ref().filter(|d| lens_supported(d)) else {
        return false;
    };
    let Some(layer) = active_pixel_layer(doc, rust.layer_target.single()) else {
        return false;
    };
    let (w, h) = (layer.rect.width(), layer.rect.height());
    if w <= 0 || h <= 0 {
        return false;
    }
    let plane = w as usize * h as usize;
    // Planes are shared, not copied: the modal dialog never edits the layer.
    let channel = |id: i16| {
        layer
            .channels
            .iter()
            .find(|c| c.id == id && c.data.len() == plane)
            .map(|c| c.data.clone())
    };
    let Some(c0) = channel(0) else {
        return false;
    };
    // A Grayscale layer carries one colour plane.
    let color = [
        c0.clone(),
        channel(1).unwrap_or_else(|| c0.clone()),
        channel(2).unwrap_or(c0),
    ];
    let exif = pictura_codec::read_metadata(doc).exif;
    let (w, h) = (w as u32, h as u32);
    let params = LensCorrectionParams::default();
    let session = Session {
        w,
        h,
        gray: doc.mode == pictura_core::ColorMode::Grayscale,
        source: LensSource::new(w, h, color, channel(-1)),
        slots: Vec::new(),
        map: LensMap::new(&params, w, h),
        exif: [
            exif_text(&exif, &[0x010f]),
            exif_text(&exif, &[0x0110]),
            exif_text(&exif, &[0xa434]),
            camera_settings(&exif),
        ],
    };
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(session);
    true
}

fn lc_geometry() -> Vec<i32> {
    with_session(Vec::new(), |s| vec![s.w as i32, s.h as i32])
}

fn lc_grayscale() -> bool {
    with_session(false, |s| s.gray)
}

fn lc_exif() -> QStringList {
    with_session(QStringList::default(), |s| {
        let mut out = QStringList::default();
        for text in &s.exif {
            out.append(QString::from(text.as_str()));
        }
        out
    })
}

fn same(a: &str, b: &QString) -> bool {
    let b = b.to_string();
    b.trim().is_empty() || a.eq_ignore_ascii_case(b.trim())
}

fn lc_profile_makes() -> QStringList {
    list(builtin_lens_profiles().iter().map(|p| p.make.as_str()))
}

fn lc_profile_models(make: &QString) -> QStringList {
    list(
        builtin_lens_profiles()
            .iter()
            .filter(|p| same(&p.make, make))
            .map(|p| p.model.as_str()),
    )
}

fn lc_profile_lenses(make: &QString, model: &QString) -> QStringList {
    list(
        builtin_lens_profiles()
            .iter()
            .filter(|p| same(&p.make, make) && same(&p.model, model))
            .map(|p| p.lens.as_str()),
    )
}

fn lc_match_profiles(make: &QString, model: &QString, lens: &QString) -> Vec<i32> {
    match_lens_profiles(
        builtin_lens_profiles(),
        &make.to_string(),
        &model.to_string(),
        &lens.to_string(),
    )
    .into_iter()
    .map(|i| i as i32 + 1)
    .collect()
}

fn lc_profile_name(id: i32) -> QString {
    usize::try_from(id - 1)
        .ok()
        .and_then(|i| builtin_lens_profiles().get(i))
        .map_or_else(QString::default, |p| QString::from(p.name.as_str()))
}

fn lc_set(values: &[f64]) -> bool {
    let Some(params) = params_of(values) else {
        return false;
    };
    with_session(false, |s| {
        s.map = LensMap::new(&params, s.w, s.h);
        s.slots = values.to_vec();
        true
    })
}

fn lc_render(x: f64, y: f64, zoom: f64, w: i32, h: i32, preview: bool) -> QImage {
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

fn lc_straighten(ax: f64, ay: f64, bx: f64, by: f64, angle: f64) -> f64 {
    if (ax, ay) == (bx, by) {
        return angle.rem_euclid(360.0);
    }
    let level = (angle - straighten_angle([ax, ay], [bx, by])).rem_euclid(360.0);
    (level * 100.0).round() / 100.0 % 360.0
}

fn lc_previous() -> Vec<f64> {
    PREVIOUS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_default()
}

fn lc_finish(keep: bool) -> Vec<f64> {
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

    /// The descriptor's opening slots.
    fn defaults() -> Vec<f64> {
        vec![
            1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 50.0, 0.0, 0.0, 0.0, 100.0,
        ]
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
        assert!(lens_correction_filter(&v).is_none());
    }

    #[test]
    fn checkboxes_gate_the_profile_parts() {
        let full = builtin_lens_profiles()[0].coefficients;
        let mut v = defaults();
        v[5] = 1.0;
        v[0] = 0.0;
        v[1] = 1.0;
        v[2] = 1.0;
        let p = params_of(&v).unwrap().profile.unwrap();
        assert_eq!(p.distortion, [0.0; 3]);
        assert_eq!(p.ca_scale, full.ca_scale);
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
    fn straighten_returns_the_levelling_angle() {
        // A line leaning 10° clockwise over an unrotated view.
        let t = 10f64.to_radians();
        let a = lc_straighten(0.0, 0.0, 100.0 * t.cos(), 100.0 * t.sin(), 0.0);
        assert!((a - 350.0).abs() < 1e-9, "{a}");
        assert!((lc_straighten(0.0, 0.0, 100.0, 0.0, 15.0) - 15.0).abs() < 1e-9);
        assert_eq!(lc_straighten(5.0, 5.0, 5.0, 5.0, 20.0), 20.0);
    }
}
