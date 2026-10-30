//! Filter ▸ Lens Correction (#347): the dialog's editing session. The dialog
//! holds every setting as the `lens-correction` kind's slots and hands them
//! here for a preview at screen resolution; OK returns the slots, so the
//! commit, Last Filter and Last Filter Settings run the ordinary filter path.
//! Free functions (their own bridge); the dialog is modal, so one session at
//! a time. Points cross the bridge in layer pixels.

mod imp;

use super::super::qobject::PictureView;
pub(in crate::cxxqt_object) use imp::*;

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
        /// empty when unknown, the names in the catalogue's spelling when it
        /// knows them; the settings read "50mm, f/2.8, 3m" with `--` for a
        /// missing part.
        fn lc_exif() -> QStringList;

        /// Every make the camera and lens catalogues and the profiles know,
        /// sorted.
        fn lc_makes() -> QStringList;

        /// The camera models of `make`, sorted.
        fn lc_models(make: &QString) -> QStringList;

        /// Lens models for a camera, best first: profiles' lenses, then
        /// `make`'s own lenses, then the rest; each group nearest the
        /// camera's crop factor first. When the photo's EXIF gives a focal
        /// length, only lenses covering it are listed.
        fn lc_lenses(make: &QString, model: &QString) -> QStringList;

        /// One line about the chosen camera and lens: crop factor and
        /// sensor, a fixed lens's range, the lens's range and aperture, and
        /// what lensfun has calibrated for it; empty when nothing is known.
        fn lc_criteria_info(make: &QString, model: &QString, lens: &QString) -> QString;

        /// The ids of the installed profiles that suit the criteria, best
        /// first; `prefer_raw` ranks raw profiles ahead of equal matches.
        fn lc_match_profiles(
            make: &QString,
            model: &QString,
            lens: &QString,
            prefer_raw: bool,
        ) -> Vec<i32>;

        /// Profile `id`'s name, or empty when it is not installed.
        fn lc_profile_name(id: i32) -> QString;

        /// Profile `id`'s sub-profiles as labels ("50 mm, f/2.8, 10 m"),
        /// in index order; empty when it is not installed.
        fn lc_subprofiles(id: i32) -> QStringList;

        /// The index of profile `id`'s sub-profile nearest the photo's EXIF
        /// focal length, f-stop, and focus distance.
        fn lc_auto_subprofile(id: i32) -> i32;

        /// The profile slots after the dialog's 16 for sub-profile `sub` of
        /// profile `id` on the camera `make` / `model` names: the index, the
        /// coefficients flag, and the coefficients rescaled to the camera's
        /// crop factor. Empty when either is not installed.
        fn lc_profile_slots(id: i32, sub: i32, make: &QString, model: &QString) -> Vec<f64>;

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

        /// The layer's stored Lens Correction smart filter slots, for the
        /// dialog's prefill; empty when it carries none.
        fn lc_stored() -> Vec<f64>;

        /// End the session. With `keep`, remember the slots as Previous
        /// Conversion and return them; otherwise empty.
        fn lc_finish(keep: bool) -> Vec<f64>;
    }
}
