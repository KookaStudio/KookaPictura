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
    struct SelectionClipResult {
        valid: bool,
        loop_lengths: Vec<u32>,
        points: Vec<f64>,
    }

    #[namespace = "pictura"]
    struct SelectionRectPreview {
        left: i32,
        top: i32,
        width: i32,
        height: i32,
        contour: Vec<i32>,
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

        /// Whether the active target can move selected pixels; `duplicate`
        /// ignores the cut's transparency-lock refusal.
        fn selection_content_movable(view: &PictureView, duplicate: bool) -> bool;

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

        /// Trace a rectangular selection preview without mutating the committed mask.
        fn selection_rect_preview_contour(
            view: &PictureView,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            mode: &QString,
            feather: f64,
        ) -> SelectionRectPreview;

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

        /// Clip point-count-prefixed selection loops to a canvas in the engine.
        fn selection_clip_shape_to_canvas(
            loop_lengths: &[u32],
            points: &[f64],
            canvas_width: f64,
            canvas_height: f64,
        ) -> SelectionClipResult;

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

        /// A nonempty selection whose coverage never exceeds 50%.
        fn selection_below_half(view: &PictureView) -> bool;

        /// The live selection coverage, row-major; empty without a selection.
        fn selection_coverage_bytes(view: &PictureView) -> Vec<u8>;

        /// The coverage of `target` in this document; empty on failure.
        fn channel_coverage(view: &PictureView, target: &QString) -> Vec<u8>;

        /// The coverage of colour row `row` (0 is the composite) under `mask`.
        fn color_row_coverage(view: &PictureView, row: i32, mask: i32) -> Vec<u8>;

        /// Duplicate `source_target`'s coverage as a new channel in this
        /// document, fitted to its canvas and optionally inverted.
        fn transfer_duplicate_channel(
            view: Pin<&mut PictureView>,
            source: &PictureView,
            source_target: &QString,
            source_mask: i32,
            name: &QString,
            invert: bool,
        ) -> bool;

        /// Save `source_target`'s coverage into this document's `target`.
        fn transfer_save_selection(
            view: Pin<&mut PictureView>,
            source: &PictureView,
            source_target: &QString,
            source_mask: i32,
            target: &QString,
            name: &QString,
            mode: &QString,
        ) -> bool;

        /// Load `source_target`'s coverage into this document's selection.
        fn transfer_load_selection(
            view: Pin<&mut PictureView>,
            source: &PictureView,
            source_target: &QString,
            source_mask: i32,
            invert: bool,
            fit: bool,
            mode: &QString,
        ) -> bool;

        /// Build this view's document from `source_target`'s coverage.
        fn transfer_new_document(
            view: Pin<&mut PictureView>,
            source: &PictureView,
            source_target: &QString,
            source_mask: i32,
            name: &QString,
            invert: bool,
        ) -> bool;

        /// Duplicate this document's own `source_target` as a new channel.
        fn duplicate_channel_in_view(
            view: Pin<&mut PictureView>,
            source_target: &QString,
            source_mask: i32,
            name: &QString,
            invert: bool,
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
    }
}
