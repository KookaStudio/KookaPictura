//! `Edit > Fade`: the retained last-effect record and the fade compositor's
//! app-side preview/commit bridge. Free functions over a [`PictureView`] (their
//! own bridge, so the `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::qobject::PictureView

use super::qobject::PictureView;

mod imp;
pub(in crate::cxxqt_object) use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether `Edit > Fade` can run: a retained effect exists, no other
        /// session is live, and no lifted transform blocks the edit.
        fn fade_available(view: &PictureView) -> bool;

        /// The retained effect's name (e.g. `Gaussian Blur`), or an empty string.
        fn fade_name(view: &PictureView) -> QString;

        /// The retained fade's opacity, or 100.
        fn fade_opacity(view: &PictureView) -> i32;

        /// The retained fade's mode as a 4-byte PSD key, or `norm`.
        fn fade_mode(view: &PictureView) -> QString;

        /// Whether `key` is allowed on the document: a Lab document refuses
        /// Color Dodge, Color Burn, Lighten, Darken, Difference, and Exclusion.
        fn fade_mode_allowed(view: &PictureView, key: &QString) -> bool;

        /// Show `opacity`/`mode` on the canvas without recording history.
        /// False when no record exists or the mode is invalid.
        fn fade_preview(view: Pin<&mut PictureView>, opacity: i32, mode: &QString) -> bool;

        /// Commit `opacity`/`mode` as one `Fade <effect>` history state.
        /// False when no record exists or the mode is invalid.
        fn fade_apply(view: Pin<&mut PictureView>, opacity: i32, mode: &QString) -> bool;

        /// Restore the last committed fade (the pre-dialog pixels when the
        /// dialog has not committed yet). False when no record exists.
        fn fade_cancel(view: Pin<&mut PictureView>) -> bool;
    }
}
