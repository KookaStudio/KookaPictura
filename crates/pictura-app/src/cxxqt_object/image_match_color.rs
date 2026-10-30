//! Image > Adjustments > Match Color (#352): statistics gathered over whole
//! layers and documents, a Lab mean/std transfer previewed on the canvas
//! through the shared adjustment preview machinery, and committed as one
//! "Match Color" state. Free functions over a [`PictureView`] (their own
//! bridge), like Replace Color.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// The target's Lab statistics — mean then standard deviation, three
        /// channels each — over the whole active layer, restricted to the
        /// selection when `use_selection` is set. Empty when there is no
        /// document or no pixel layer active.
        fn match_color_target_stats(view: &PictureView, use_selection: bool) -> Vec<f64>;

        /// A source document's statistics over its merged composite (an empty
        /// `path`) or one pixel layer (the panel path in `path`), restricted
        /// to that document's selection when `use_selection` is set. Empty when
        /// the document or layer is missing.
        fn match_color_source_stats(
            source: &PictureView,
            path: &QString,
            use_selection: bool,
        ) -> Vec<f64>;

        /// Preview Match Color on the active layer over the document rect
        /// `(x, y, w, h)` (the whole layer when `w` is not positive),
        /// re-applied from the pre-preview pixels; no history. `target` and
        /// `source` are the statistics vectors from the stats calls. False
        /// when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_match_color_preview(
            view: Pin<&mut PictureView>,
            target: &QList_f64,
            source: &QList_f64,
            ignore_selection: bool,
            neutralize: bool,
            luminance: f64,
            color_intensity: f64,
            fade: f64,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
        ) -> bool;

        /// Apply Match Color to the whole active layer as one "Match Color"
        /// state (an open preview is replaced). False when refused.
        #[allow(clippy::too_many_arguments)]
        fn image_match_color_apply(
            view: Pin<&mut PictureView>,
            target: &QList_f64,
            source: &QList_f64,
            ignore_selection: bool,
            neutralize: bool,
            luminance: f64,
            color_intensity: f64,
            fade: f64,
        ) -> bool;
    }
}
