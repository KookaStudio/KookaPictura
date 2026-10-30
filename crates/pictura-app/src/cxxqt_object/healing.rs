//! The healing bridges: Spot Healing Brush, Healing Brush, Patch,
//! Content-Aware Move, and Red Eye. Free functions
//! over a [`PictureView`] (their own bridge, so the `PictureView` declaration
//! list does not grow). A gesture accumulates a coverage mask
//! ([`HealStroke`]); `healing_commit` runs the solve on the layer, recomposites,
//! and records one `"Spot Healing Brush"` / `"Healing Brush"` history state.
//! `patch_selection` heals through the selection instead of a stroke mask and
//! records one `"Patch Tool"` state; `content_aware_move` likewise records one
//! `"Content-Aware Move"` state, and `red_eye` one `"Red Eye Tool"` state.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Start a healing gesture on the active pixel layer at `diameter` /
        /// `hardness`. False without a lone active raster layer or when its
        /// pixels are locked.
        fn healing_begin(view: Pin<&mut PictureView>, diameter: i32, hardness: i32) -> bool;

        /// Add a dab at document-space `(x, y)`. False while no gesture is
        /// active.
        fn healing_dab(view: Pin<&mut PictureView>, x: f64, y: f64) -> bool;

        /// Run the heal on release and commit it. `mode` is 0/1/2 for the Spot
        /// Healing types; for the Healing Brush pass the sample offset in
        /// `source_x`/`source_y` and `use_source` true. Records one history
        /// state; false when nothing was healed.
        fn healing_commit(
            view: Pin<&mut PictureView>,
            mode: i32,
            use_source: bool,
            source_x: i32,
            source_y: i32,
            texture_only: bool,
        ) -> bool;

        /// Drop an active gesture without committing (Escape, tool switch).
        fn healing_cancel(view: Pin<&mut PictureView>);

        /// Whether a gesture is active.
        fn healing_active(view: &PictureView) -> bool;

        /// Patch the active pixel layer through the selection after a drag of
        /// `(dx, dy)`: Source repairs the selection from the dragged-to area,
        /// Destination applies the selection there, Content-Aware ignores the
        /// drag and rebuilds the selection from its surroundings. Records one
        /// "Patch Tool" state; false (no state) without a selection, on a zero
        /// drag outside Content-Aware, or on a locked / non-raster layer.
        fn patch_selection(
            view: Pin<&mut PictureView>,
            dx: i32,
            dy: i32,
            content_aware: bool,
            destination: bool,
            transparent: bool,
        ) -> bool;

        /// Move the selection's pixels on the active pixel layer by `(dx, dy)`
        /// and rebuild the hole from its surroundings, or with `extend` copy
        /// them and keep the original. `adaptation` is 0 Very Strict … 4 Very
        /// Loose. The selection follows the pixels. Records one "Content-Aware
        /// Move" state; false (no state) without a selection, on a zero drag,
        /// or on a locked / non-raster layer.
        fn content_aware_move(
            view: Pin<&mut PictureView>,
            dx: i32,
            dy: i32,
            extend: bool,
            adaptation: i32,
        ) -> bool;

        /// Neutralise red-eye on the active pixel layer inside the document
        /// box `(x, y, width, height)`. `pupil` is Pupil Size and `darken`
        /// Darken Amount, both 0–100. Records one "Red Eye Tool" state; false
        /// (no state) when the box holds no red or the layer is locked or not
        /// a pixel layer.
        fn red_eye(
            view: Pin<&mut PictureView>,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            pupil: i32,
            darken: i32,
        ) -> bool;
    }
}
