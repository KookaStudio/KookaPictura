//! Filter ▸ Adaptive Wide Angle (#346): the workspace's editing session and
//! its kept constraint sets. The dialog edits the correction and constraints
//! here and asks for its view rendered at screen resolution; OK keeps the
//! constraints and the `adaptive-wide-angle` kind's slots name them by id, so
//! the commit, Last Filter and Last Filter Settings run the ordinary filter
//! path, as Liquify's mesh does. Free functions (their own bridge); the
//! dialog is modal, so one session at a time. Points cross the bridge in
//! layer pixels; "display" points are output pixels with Preview on and
//! layer pixels with it off.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QImageFormat, QString};
use pictura_codec::Exif;
use pictura_filters::{
    awa_two_to_one, AwaConstraint, AwaCorrection, AwaOrientation, AwaParams, AwaSource, AwaWarp,
};

use super::super::helpers::{active_mask_target, active_pixel_layer};
use super::super::qobject::PictureView;
use super::document_supported;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Open a session over the active layer. False when the document is
        /// not one the filter takes or there is no unmasked pixel layer with
        /// colour data.
        fn awa_begin(view: &PictureView) -> bool;

        /// The session's layer size, `[width, height]`; empty with no session.
        fn awa_geometry() -> Vec<i32>;

        /// The lens EXIF gives, `[focal length, crop factor]` in the
        /// dialog's ranges; empty when the image has no focal length.
        fn awa_exif_lens() -> Vec<f64>;

        /// Whether the session's layer is the 2:1 panorama Full Spherical
        /// needs.
        fn awa_full_spherical_allowed() -> bool;

        /// The session's `[focal length, crop factor]`.
        fn awa_lens_settings() -> Vec<f64>;

        /// The Focal Length field's `[minimum, maximum]` at `crop`.
        fn awa_focal_range(crop: f64) -> Vec<f64>;

        /// EXIF Make and Model, or empty.
        fn awa_camera_model() -> QString;

        /// EXIF Lens Model, or empty.
        fn awa_lens_model() -> QString;

        /// Set Correction (0 Fisheye, 1 Perspective, 2 Full Spherical),
        /// Scale (%), Focal Length (mm) and Crop Factor, and re-solve.
        fn awa_set_lens(correction: i32, scale: f64, focal: f64, crop: f64);

        /// The Move tool's offset, in layer pixels.
        fn awa_set_offset(x: f64, y: f64);
        fn awa_offset() -> Vec<f64>;

        /// Add a line from layer point `(ax, ay)` to `(bx, by)`; `orientation`
        /// is 0 Unfixed, 1 Horizontal, 2 Vertical. Returns its index.
        fn awa_add_line(ax: f64, ay: f64, bx: f64, by: f64, orientation: i32) -> i32;

        /// Add a polygon through flattened layer points. Returns its index,
        /// or -1 with fewer than three corners.
        fn awa_add_polygon(points: &[f64]) -> i32;

        /// Set constraint `index`'s orientation: 0 Unfixed, 1 Horizontal,
        /// 2 Vertical, 3 Arbitrary (fixed at its current output angle).
        fn awa_set_orientation(index: i32, orientation: i32) -> bool;

        /// Move point `point` of constraint `index` to layer point `(x, y)`.
        fn awa_move_point(index: i32, point: i32, x: f64, y: f64) -> bool;

        fn awa_remove(index: i32) -> bool;
        fn awa_constraint_count() -> i32;

        /// 0 Unfixed, 1 Horizontal, 2 Vertical, 3 Fixed, 4 invalid; -1 when
        /// there is no such constraint.
        fn awa_constraint_state(index: i32) -> i32;

        fn awa_constraint_polygon(index: i32) -> bool;

        /// Constraint `index`'s traced edges in display pixels, flattened
        /// `x, y` pairs with a NaN pair after each edge.
        fn awa_constraint_paths(index: i32, preview: bool) -> Vec<f64>;

        /// Constraint `index`'s points (its handles) in display pixels.
        fn awa_constraint_points(index: i32, preview: bool) -> Vec<f64>;

        /// The traced great circle between two layer points, in display
        /// pixels; empty when it cannot be computed.
        fn awa_arc(ax: f64, ay: f64, bx: f64, by: f64, preview: bool) -> Vec<f64>;

        /// The layer point display point `(x, y)` shows; empty off the
        /// picture.
        fn awa_to_layer(x: f64, y: f64, preview: bool) -> Vec<f64>;

        /// A `w`×`h` premultiplied view whose pixel `(i, j)` shows display
        /// point `(x + (i + 0.5) / zoom, y + (j + 0.5) / zoom)`.
        fn awa_render(x: f64, y: f64, zoom: f64, w: i32, h: i32, preview: bool) -> QImage;

        /// Show Mesh: the solved grid's lines in display pixels, flattened
        /// with a NaN pair after each line.
        fn awa_mesh_lines(preview: bool) -> Vec<f64>;

        /// Restore committed slots (Last Filter Settings): the lens, offset
        /// and constraints. False when the constraint set is no longer kept
        /// or the slots ask for Full Spherical on a layer that is not 2:1.
        fn awa_load(values: &[f64]) -> bool;

        /// End the session. With `keep`, keep the constraints and return the
        /// kind's slots; otherwise empty.
        fn awa_finish(keep: bool) -> Vec<f64>;
    }
}

/// Focal Length × Crop Factor (the 35 mm equivalent) stays in this range, so
/// the Focal Length field's own range follows Crop Factor, as CS6's does:
/// 4.05–121.41 mm at the opening crop of 0.57, 23.1–692.0 at 0.10. Read off
/// CS6 captures.
const EQUIVALENT_RANGE: (f64, f64) = (4.05 * 0.57, 121.41 * 0.57);
pub(in crate::cxxqt_object) const CROP_RANGE: (f64, f64) = (0.1, 10.0);

/// The Focal Length field's range at `crop`.
pub(in crate::cxxqt_object) fn focal_range(crop: f64) -> (f64, f64) {
    let crop = crop.clamp(CROP_RANGE.0, CROP_RANGE.1);
    (EQUIVALENT_RANGE.0 / crop, EQUIVALENT_RANGE.1 / crop)
}

/// ponytail: the eight most recent constraint sets stay in memory for the
/// process; a Last Filter that names an older one is refused.
const KEPT: usize = 8;

struct Store {
    next: u32,
    sets: VecDeque<(u32, Arc<Vec<AwaConstraint>>)>,
}

static STORE: Mutex<Store> = Mutex::new(Store {
    next: 1,
    sets: VecDeque::new(),
});

fn store() -> std::sync::MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

fn register(set: Arc<Vec<AwaConstraint>>) -> u32 {
    let mut store = store();
    let id = store.next;
    store.next = store.next.wrapping_add(1).max(1);
    if store.sets.len() == KEPT {
        store.sets.pop_front();
    }
    store.sets.push_back((id, set));
    id
}

/// The constraint set `id` names. Id 0 is no constraints; an id no longer
/// kept is `None`.
pub(in crate::cxxqt_object) fn awa_constraints(id: u32) -> Option<Arc<Vec<AwaConstraint>>> {
    if id == 0 {
        return Some(Arc::default());
    }
    store()
        .sets
        .iter()
        .find(|(kept, _)| *kept == id)
        .map(|(_, set)| set.clone())
}

/// The correction a slot value names.
pub(in crate::cxxqt_object) fn awa_correction(value: f64) -> AwaCorrection {
    match value.round() as i64 {
        1 => AwaCorrection::Perspective,
        2 => AwaCorrection::FullSpherical,
        _ => AwaCorrection::Fisheye,
    }
}

fn correction_index(c: AwaCorrection) -> f64 {
    match c {
        AwaCorrection::Fisheye => 0.0,
        AwaCorrection::Perspective => 1.0,
        AwaCorrection::FullSpherical => 2.0,
    }
}

/// The dialog's Focal Length and Crop Factor for EXIF FocalLength and
/// FocalLengthIn35mmFilm (crop 1 without it); an equivalent past the range
/// is clamped to it.
fn lens_from_exif(exif: &Exif) -> Option<[f64; 2]> {
    let focal = exif.get(0x920a)?.as_f64().filter(|f| *f > 0.0)?;
    let equivalent = exif
        .get(0xa405)
        .and_then(|v| v.as_f64())
        .filter(|f| *f > 0.0)
        .unwrap_or(focal);
    let round = |v: f64| (v * 100.0).round() / 100.0;
    let crop = round((equivalent / focal).clamp(CROP_RANGE.0, CROP_RANGE.1));
    let (low, high) = focal_range(crop);
    Some([round(focal.clamp(low, high)), crop])
}

fn exif_text(exif: &Exif, tags: &[u16]) -> String {
    tags.iter()
        .filter_map(|&t| exif.get(t).map(|v| v.display().trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

struct Session {
    w: f64,
    h: f64,
    source: AwaSource,
    params: AwaParams,
    warp: AwaWarp,
    exif_lens: Option<[f64; 2]>,
    camera: String,
    lens: String,
}

impl Session {
    fn resolve(&mut self) {
        self.warp = AwaWarp::solve(&self.params, self.w as u32, self.h as u32);
    }

    fn constraints(&mut self) -> &mut Vec<AwaConstraint> {
        Arc::make_mut(&mut self.params.constraints)
    }

    fn fraction(&self, x: f64, y: f64) -> [f64; 2] {
        [x / self.w, y / self.h]
    }

    fn display(&self, p: [f64; 2], preview: bool) -> [f64; 2] {
        if preview {
            self.warp.forward(p)
        } else {
            p
        }
    }
}

static SESSION: Mutex<Option<Session>> = Mutex::new(None);

fn with_session<R>(default: R, f: impl FnOnce(&mut Session) -> R) -> R {
    let mut guard = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_mut().map_or(default, f)
}

fn flatten(paths: impl IntoIterator<Item = Vec<[f64; 2]>>) -> Vec<f64> {
    let mut out = Vec::new();
    for path in paths {
        out.extend(path.iter().flatten());
        out.extend([f64::NAN, f64::NAN]);
    }
    out
}

fn orientation_of(code: i32) -> AwaOrientation {
    match code {
        1 => AwaOrientation::Horizontal,
        2 => AwaOrientation::Vertical,
        _ => AwaOrientation::Unfixed,
    }
}

fn awa_begin(view: &PictureView) -> bool {
    let rust = view.rust();
    if active_mask_target(rust).is_some() {
        return false;
    }
    let Some(doc) = rust.session.doc.as_ref().filter(|d| document_supported(d)) else {
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
    let color = [
        c0.clone(),
        channel(1).unwrap_or_else(|| c0.clone()),
        channel(2).unwrap_or(c0),
    ];
    let exif = pictura_codec::read_metadata(doc).exif;
    let exif_lens = lens_from_exif(&exif);
    let mut params = AwaParams::default();
    if let Some([focal, crop]) = exif_lens {
        params.focal_length = focal;
        params.crop_factor = crop;
    }
    let warp = AwaWarp::solve(&params, w as u32, h as u32);
    let session = Session {
        w: f64::from(w),
        h: f64::from(h),
        source: AwaSource::new(w as u32, h as u32, color, channel(-1)),
        params,
        warp,
        exif_lens,
        camera: exif_text(&exif, &[0x010f, 0x0110]),
        lens: exif_text(&exif, &[0xa434]),
    };
    *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = Some(session);
    true
}

fn awa_geometry() -> Vec<i32> {
    with_session(Vec::new(), |s| vec![s.w as i32, s.h as i32])
}

fn awa_full_spherical_allowed() -> bool {
    with_session(false, |s| awa_two_to_one(s.w as u32, s.h as u32))
}

fn awa_lens_settings() -> Vec<f64> {
    with_session(Vec::new(), |s| {
        vec![s.params.focal_length, s.params.crop_factor]
    })
}

fn awa_focal_range(crop: f64) -> Vec<f64> {
    let (low, high) = focal_range(crop);
    vec![low, high]
}

fn awa_exif_lens() -> Vec<f64> {
    with_session(Vec::new(), |s| {
        s.exif_lens.map(Vec::from).unwrap_or_default()
    })
}

fn awa_camera_model() -> QString {
    with_session(QString::default(), |s| QString::from(s.camera.as_str()))
}

fn awa_lens_model() -> QString {
    with_session(QString::default(), |s| QString::from(s.lens.as_str()))
}

fn awa_set_lens(correction: i32, scale: f64, focal: f64, crop: f64) {
    with_session((), |s| {
        s.params.correction = awa_correction(f64::from(correction));
        s.params.scale = scale / 100.0;
        s.params.focal_length = focal;
        s.params.crop_factor = crop;
        s.resolve();
    });
}

fn awa_set_offset(x: f64, y: f64) {
    with_session((), |s| {
        s.params.offset = [x / s.w, y / s.h];
        s.resolve();
    });
}

fn awa_offset() -> Vec<f64> {
    with_session(Vec::new(), |s| {
        vec![s.params.offset[0] * s.w, s.params.offset[1] * s.h]
    })
}

fn awa_add_line(ax: f64, ay: f64, bx: f64, by: f64, orientation: i32) -> i32 {
    with_session(-1, |s| {
        let points = vec![s.fraction(ax, ay), s.fraction(bx, by)];
        let constraints = s.constraints();
        constraints.push(AwaConstraint {
            points,
            polygon: false,
            orientation: orientation_of(orientation),
        });
        let index = constraints.len() as i32 - 1;
        s.resolve();
        index
    })
}

fn awa_add_polygon(points: &[f64]) -> i32 {
    if points.len() < 6 {
        return -1;
    }
    with_session(-1, |s| {
        let points = points
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[x, y]| s.fraction(*x, *y))
            .collect();
        let constraints = s.constraints();
        constraints.push(AwaConstraint {
            points,
            polygon: true,
            orientation: AwaOrientation::Unfixed,
        });
        let index = constraints.len() as i32 - 1;
        s.resolve();
        index
    })
}

fn awa_set_orientation(index: i32, orientation: i32) -> bool {
    with_session(false, |s| {
        let Ok(i) = usize::try_from(index) else {
            return false;
        };
        if i >= s.params.constraints.len() || s.params.constraints[i].polygon {
            return false;
        }
        let next = if orientation == 3 {
            let Some(path) = s.warp.constraint_paths(i, true).into_iter().next() else {
                return false;
            };
            let (a, b) = (path[0], path[path.len() - 1]);
            AwaOrientation::Fixed((b[1] - a[1]).atan2(b[0] - a[0]))
        } else {
            orientation_of(orientation)
        };
        s.constraints()[i].orientation = next;
        s.resolve();
        true
    })
}

fn awa_move_point(index: i32, point: i32, x: f64, y: f64) -> bool {
    with_session(false, |s| {
        let p = s.fraction(x, y);
        let (Ok(i), Ok(k)) = (usize::try_from(index), usize::try_from(point)) else {
            return false;
        };
        let Some(slot) = s.constraints().get_mut(i).and_then(|c| c.points.get_mut(k)) else {
            return false;
        };
        *slot = p;
        s.resolve();
        true
    })
}

fn awa_remove(index: i32) -> bool {
    with_session(false, |s| {
        let Ok(i) = usize::try_from(index) else {
            return false;
        };
        if i >= s.params.constraints.len() {
            return false;
        }
        s.constraints().remove(i);
        s.resolve();
        true
    })
}

fn awa_constraint_count() -> i32 {
    with_session(0, |s| s.params.constraints.len() as i32)
}

fn awa_constraint_state(index: i32) -> i32 {
    with_session(-1, |s| {
        let Ok(i) = usize::try_from(index) else {
            return -1;
        };
        if i >= s.warp.constraint_count() {
            return -1;
        }
        match s.warp.constraint_state(i) {
            None => 4,
            Some(AwaOrientation::Unfixed) => 0,
            Some(AwaOrientation::Horizontal) => 1,
            Some(AwaOrientation::Vertical) => 2,
            Some(AwaOrientation::Fixed(_)) => 3,
        }
    })
}

fn awa_constraint_polygon(index: i32) -> bool {
    with_session(false, |s| {
        usize::try_from(index)
            .ok()
            .and_then(|i| s.params.constraints.get(i))
            .is_some_and(|c| c.polygon)
    })
}

fn awa_constraint_paths(index: i32, preview: bool) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        let Ok(i) = usize::try_from(index) else {
            return Vec::new();
        };
        flatten(s.warp.constraint_paths(i, preview))
    })
}

fn awa_constraint_points(index: i32, preview: bool) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        let Some(c) = usize::try_from(index)
            .ok()
            .and_then(|i| s.params.constraints.get(i))
        else {
            return Vec::new();
        };
        c.points
            .iter()
            .flat_map(|p| s.display([p[0] * s.w, p[1] * s.h], preview))
            .collect()
    })
}

fn awa_arc(ax: f64, ay: f64, bx: f64, by: f64, preview: bool) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        s.warp
            .arc([ax, ay], [bx, by], preview)
            .into_iter()
            .flatten()
            .collect()
    })
}

fn awa_to_layer(x: f64, y: f64, preview: bool) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        let p = if preview {
            s.warp.inverse([x, y])
        } else {
            Some([x, y])
        };
        p.filter(|p| (0.0..=s.w).contains(&p[0]) && (0.0..=s.h).contains(&p[1]))
            .map(Vec::from)
            .unwrap_or_default()
    })
}

fn awa_render(x: f64, y: f64, zoom: f64, w: i32, h: i32, preview: bool) -> QImage {
    if w <= 0 || h <= 0 || zoom <= 0.0 {
        return QImage::default();
    }
    with_session(QImage::default(), |s| {
        let (wu, hu) = (w as usize, h as usize);
        let mut bytes = vec![0u8; wu * hu * 4];
        if preview {
            s.warp.render(&s.source, [x, y], zoom, (wu, hu), &mut bytes);
        } else {
            s.source.render([x, y], zoom, (wu, hu), &mut bytes);
        }
        // SAFETY: `bytes` is exactly w*h premultiplied RGBA8888 samples.
        unsafe { QImage::from_raw_bytes(bytes, w, h, QImageFormat::Format_RGBA8888_Premultiplied) }
    })
}

fn awa_mesh_lines(preview: bool) -> Vec<f64> {
    with_session(Vec::new(), |s| {
        if preview {
            return flatten(s.warp.mesh_lines());
        }
        // Uncorrected, the grid is the plain one the solve starts from.
        let identity = AwaWarp::solve(
            &AwaParams {
                correction: AwaCorrection::FullSpherical,
                ..AwaParams::default()
            },
            s.w as u32,
            s.h as u32,
        );
        flatten(identity.mesh_lines())
    })
}

fn awa_load(values: &[f64]) -> bool {
    let [correction, scale, focal, crop, ox, oy, id] = values else {
        return false;
    };
    let Some(constraints) = awa_constraints(*id as u32) else {
        return false;
    };
    let correction = awa_correction(*correction);
    with_session(false, |s| {
        if correction == AwaCorrection::FullSpherical && !awa_two_to_one(s.w as u32, s.h as u32) {
            return false;
        }
        s.params = AwaParams {
            correction,
            scale: scale / 100.0,
            focal_length: *focal,
            crop_factor: *crop,
            offset: [*ox, *oy],
            constraints,
        };
        s.resolve();
        true
    })
}

fn awa_finish(keep: bool) -> Vec<f64> {
    let session = SESSION.lock().unwrap_or_else(|e| e.into_inner()).take();
    match session {
        Some(s) if keep => {
            let p = &s.params;
            let id = if p.constraints.is_empty() {
                0
            } else {
                register(p.constraints.clone())
            };
            vec![
                correction_index(p.correction),
                p.scale * 100.0,
                p.focal_length,
                p.crop_factor,
                p.offset[0],
                p.offset[1],
                f64::from(id),
            ]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_codec::parse_exif;

    /// A little-endian EXIF stream: IFD0 with Make, Model and the Exif IFD
    /// pointer; the Exif IFD with FocalLength and FocalLengthIn35mmFilm.
    fn exif(focal: u32, equivalent: u16) -> Exif {
        let mut t = b"II*\0\x08\0\0\0".to_vec();
        let entry = |t: &mut Vec<u8>, tag: u16, kind: u16, count: u32, value: u32| {
            t.extend(tag.to_le_bytes());
            t.extend(kind.to_le_bytes());
            t.extend(count.to_le_bytes());
            t.extend(value.to_le_bytes());
        };
        // IFD0 at 8: 3 entries (2 + 36 + 4 bytes) ends at 50; Make's text
        // at 50, Model's four bytes inline.
        t.extend(3u16.to_le_bytes());
        entry(&mut t, 0x010f, 2, 8, 50);
        entry(&mut t, 0x0110, 2, 4, u32::from_le_bytes(*b"X10\0"));
        entry(&mut t, 0x8769, 4, 1, 60);
        t.extend(0u32.to_le_bytes());
        t.extend(b"ACME Co\0\0\0");
        // Exif IFD at 60: 2 entries ends at 90; the rational at 90.
        t.extend(2u16.to_le_bytes());
        entry(&mut t, 0x920a, 5, 1, 90);
        entry(&mut t, 0xa405, 3, 1, u32::from(equivalent));
        t.extend(0u32.to_le_bytes());
        t.extend(focal.to_le_bytes());
        t.extend(1u32.to_le_bytes());
        parse_exif(&t)
    }

    #[test]
    fn exif_prefills_the_lens() {
        let e = exif(10, 15);
        assert_eq!(lens_from_exif(&e), Some([10.0, 1.5]));
        // The field's range follows the crop factor.
        let (low, high) = focal_range(0.57);
        assert!((low - 4.05).abs() < 1e-9 && (high - 121.41).abs() < 1e-9);
        assert!((focal_range(0.1).1 - 692.037).abs() < 1e-9);
        assert_eq!(exif_text(&e, &[0x010f, 0x0110]), "ACME Co X10");
        // A full-frame 50 mm lens shows as it is; a 200 mm one is past the
        // range and clamped to its top.
        assert_eq!(lens_from_exif(&exif(50, 50)), Some([50.0, 1.0]));
        assert_eq!(lens_from_exif(&exif(200, 200)), Some([69.2, 1.0]));
        assert_eq!(lens_from_exif(&Exif::default()), None);
    }

    #[test]
    fn kept_sets_are_found_by_id_and_zero_is_empty() {
        assert!(awa_constraints(0).unwrap().is_empty());
        let set = Arc::new(vec![AwaConstraint {
            points: vec![[0.1, 0.1], [0.9, 0.2]],
            polygon: false,
            orientation: AwaOrientation::Horizontal,
        }]);
        let id = register(set.clone());
        assert_eq!(awa_constraints(id).unwrap(), set);
        for _ in 0..KEPT {
            register(Arc::default());
        }
        assert!(awa_constraints(id).is_none(), "the oldest set is dropped");
    }
}
