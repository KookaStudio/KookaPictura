//! Selection move bridge: origin capture, preview, commit, cancel, and the
//! content move. Free functions over [`PictureView`] (their own bridge, so the
//! `PictureView` declaration list does not grow).
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
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
        /// Capture the current selection as the origin of a move drag; false without one.
        fn begin_selection_move(view: Pin<&mut PictureView>) -> bool;

        /// Set selection to the origin translated by `(dx, dy)`; no history, no `changed`.
        fn preview_selection_move(view: Pin<&mut PictureView>, dx: i32, dy: i32) -> bool;

        /// Record one "Move Selection" state when the selection moved; false on no-op.
        fn commit_selection_move(view: Pin<&mut PictureView>) -> bool;

        /// Restore the origin selection and drop the drag; no history.
        fn cancel_selection_move(view: Pin<&mut PictureView>) -> bool;

        /// Move selected content by `(dx, dy)`; `duplicate` copies. One undo state.
        fn move_selection_content(
            view: Pin<&mut PictureView>,
            dx: i32,
            dy: i32,
            duplicate: bool,
            background: u32,
        ) -> bool;

        /// Flood-select around `(x, y)` within `tolerance` and combine per `mode`.
        fn quick_select(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            tolerance: i32,
            mode: &QString,
        ) -> bool;

        /// Snapshot the selection before a Quick Selection stroke.
        fn quick_select_begin(view: Pin<&mut PictureView>) -> bool;

        /// Apply one Quick Selection dab without recording history. `is_move`
        /// lets a move inside the selected region skip the full flood.
        fn quick_select_preview(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            tolerance: i32,
            mode: &QString,
            is_move: bool,
        ) -> bool;

        /// Record the Quick Selection stroke's one history state.
        fn quick_select_commit(view: Pin<&mut PictureView>) -> bool;

        /// Restore the selection captured at the start of the Quick Selection stroke.
        fn quick_select_cancel(view: Pin<&mut PictureView>) -> bool;

        /// Commit an ellipse, anti-aliased when `anti_alias` is set.
        fn selection_select_ellipse_aa(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            mode: &QString,
            feather: f64,
            anti_alias: bool,
        ) -> bool;

        /// Commit the pending lasso polygon, anti-aliased when `anti_alias` is set.
        fn selection_end_lasso_aa(
            view: Pin<&mut PictureView>,
            feather: f64,
            anti_alias: bool,
        ) -> bool;

        /// Flood-select around `(x, y)`, anti-aliased when `anti_alias` is set.
        fn selection_magic_wand_aa(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            tolerance: i32,
            contiguous: bool,
            mode: &QString,
            anti_alias: bool,
        ) -> bool;

        /// The integer rect a marquee drag marks out. `style` is the C++
        /// `MarqueeStyle` value (0 Normal, 1 Fixed Ratio, 2 Fixed Size).
        fn selection_marquee_rect(
            x0: f64,
            y0: f64,
            x1: f64,
            y1: f64,
            shift: bool,
            mirror: bool,
            style: i32,
            ratio_w: f64,
            ratio_h: f64,
            fixed_w: i32,
            fixed_h: i32,
        ) -> Vec<i32>;

        /// Whether `(x1, y1)` lies within `radius` of `(x0, y0)`.
        fn selection_within_radius(x0: f64, y0: f64, x1: f64, y1: f64, radius: f64) -> bool;

        /// Save into `target` (`new`, `alpha:<index>`, `mask:<path>`) with `mode`.
        fn save_selection_to(
            view: Pin<&mut PictureView>,
            target: &QString,
            name: &QString,
            mode: &QString,
        ) -> bool;

        /// Load `target` (`alpha:<index>` or `mask:<path>`) with invert and `mode`.
        fn load_selection_target(
            view: Pin<&mut PictureView>,
            target: &QString,
            invert: bool,
            mode: &QString,
        ) -> bool;

        /// Every existing raster layer mask as `path|Layer Name Mask`.
        fn selection_mask_targets(view: &PictureView) -> QStringList;

        /// The live selection coverage, row-major; empty without a selection.
        fn selection_coverage_bytes(view: &PictureView) -> Vec<u8>;

        /// The coverage of `target` in this document; empty on failure.
        fn channel_coverage(view: &PictureView, target: &QString) -> Vec<u8>;

        /// Save transferred coverage into `target`; one history state.
        fn save_selection_coverage(
            view: Pin<&mut PictureView>,
            coverage: &[u8],
            width: i32,
            height: i32,
            target: &QString,
            name: &QString,
            mode: &QString,
        ) -> bool;

        /// Load transferred coverage into this document; one history state.
        fn load_selection_coverage(
            view: Pin<&mut PictureView>,
            coverage: &[u8],
            width: i32,
            height: i32,
            invert: bool,
            mode: &QString,
        ) -> bool;

        /// Build a single-channel multichannel document from coverage.
        fn new_selection_document(
            view: Pin<&mut PictureView>,
            coverage: &[u8],
            width: i32,
            height: i32,
            name: &QString,
        ) -> bool;
    }
}
