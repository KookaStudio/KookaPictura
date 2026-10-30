//! Properties-panel mask refinements bridge: Density and Feather for the active
//! layer's layer mask and vector mask, and the layer mask's Invert. A setter
//! with `commit == false` previews (recomposites, records nothing); the commit
//! records one state when its value or an earlier preview changed the mask.
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
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
        /// The active layer's mask density (0-255); -1 without a mask.
        fn layer_mask_density(view: &PictureView) -> i32;

        /// The active layer's mask feather radius in pixels; 0 without a mask.
        fn layer_mask_feather(view: &PictureView) -> f64;

        /// Set the active layer's mask density (0-255). A commit records one
        /// "Mask Density" state. False without a mask or with no change.
        fn layer_mask_set_density(view: Pin<&mut PictureView>, density: i32, commit: bool) -> bool;

        /// Set the active layer's mask feather radius in pixels. A commit
        /// records one "Mask Feather" state. False without a mask, for a
        /// negative radius, or with no change.
        fn layer_mask_set_feather(view: Pin<&mut PictureView>, radius: f64, commit: bool) -> bool;

        /// Invert the active layer's mask pixels; one "Invert" state. False
        /// without a mask.
        fn layer_mask_invert(view: Pin<&mut PictureView>) -> bool;

        /// The active layer's vector mask density (0-255); -1 without one.
        fn vector_mask_density(view: &PictureView) -> i32;

        /// The active layer's vector mask feather radius in pixels; 0 without one.
        fn vector_mask_feather(view: &PictureView) -> f64;

        /// As `layer_mask_set_density` for the vector mask ("Vector Mask
        /// Density").
        fn vector_mask_set_density(view: Pin<&mut PictureView>, density: i32, commit: bool)
            -> bool;

        /// As `layer_mask_set_feather` for the vector mask ("Vector Mask
        /// Feather").
        fn vector_mask_set_feather(view: Pin<&mut PictureView>, radius: f64, commit: bool) -> bool;
    }
}
