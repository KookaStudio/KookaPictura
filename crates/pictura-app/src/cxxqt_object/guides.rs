//! Guide commands: free functions over a [`PictureView`] (their own bridge, so
//! the `PictureView` declaration list does not grow). Every committed edit
//! records one history state and emits `changed`; a live drag step records
//! nothing.
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
        /// Number of guides; 0 without a document.
        fn guide_count(view: &PictureView) -> i32;

        /// Guide `index` as `[vertical (1 or 0), position]` in document pixels; empty when out of range.
        fn guide_at(view: &PictureView, index: i32) -> Vec<f64>;

        /// The guide nearest `(x, y)` within `radius` document pixels, or -1.
        fn guide_near(view: &PictureView, x: f64, y: f64, radius: f64) -> i32;

        /// Add a guide and record one "New Guide" state; its index, or -1 (no state) without a document.
        fn add_guide(view: Pin<&mut PictureView>, vertical: bool, position: f64) -> i32;

        /// Move guide `index`. With `commit` false this is a live drag step (no history, no `changed`); with `commit` true it records one "Move Guide" state. False for a bad index or no document.
        fn move_guide(view: Pin<&mut PictureView>, index: i32, position: f64, commit: bool)
            -> bool;

        /// Delete guide `index` and record one "Delete Guide" state; false for a bad index or no document.
        fn remove_guide(view: Pin<&mut PictureView>, index: i32) -> bool;

        /// Delete every guide and record one "Clear Guides" state; false (no state) when there were none.
        fn clear_guides(view: Pin<&mut PictureView>) -> bool;
    }
}
