//! Properties-panel mask refinements bridge: Density and Feather for the active
//! layer's layer mask and vector mask, and the layer mask's Invert. A setter
//! with `commit == false` previews (recomposites, records nothing); the commit
//! records one state when its value or an earlier preview changed the mask.
//!
//! [`PictureView`]: super::super::qobject::PictureView

use super::super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use pictura_core::Document;

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

/// The active layer's `(density, feather)` from `read`.
fn refinement(
    view: &PictureView,
    read: fn(&Document, &str) -> Option<(u8, f64)>,
) -> Option<(u8, f64)> {
    let rust = view.rust();
    read(rust.doc.as_ref()?, rust.active_layer.as_deref()?)
}

/// Apply `set` to the active layer; recomposite on a change, and on a commit
/// record `label` when this call or an earlier preview changed the mask. A
/// preview another control left uncommitted is recorded under its own label
/// first, so each state holds one control's change.
fn edit(
    mut view: Pin<&mut PictureView>,
    label: &'static str,
    commit: bool,
    set: impl FnOnce(&mut Document, &str) -> bool,
) -> bool {
    let pending = view.as_mut().rust_mut().mask_refine_preview.take();
    if let Some(other) = pending.filter(|other| *other != label) {
        view.as_mut().record(other);
    }
    let pending = pending == Some(label);
    let changed = {
        let mut rust = view.as_mut().rust_mut();
        let Some(path) = rust.active_layer.clone() else {
            return false;
        };
        rust.doc.as_mut().is_some_and(|doc| set(doc, &path))
    };
    if !commit && (changed || pending) {
        view.as_mut().rust_mut().mask_refine_preview = Some(label);
    }
    if changed || (commit && pending) {
        view.as_mut().recomposite();
    }
    if commit && (changed || pending) {
        view.as_mut().record(label);
    }
    changed
}

fn layer_mask_density(view: &PictureView) -> i32 {
    refinement(view, pictura_render::layer_mask_refinement).map_or(-1, |(d, _)| i32::from(d))
}

fn layer_mask_feather(view: &PictureView) -> f64 {
    refinement(view, pictura_render::layer_mask_refinement).map_or(0.0, |(_, f)| f)
}

fn layer_mask_set_density(view: Pin<&mut PictureView>, density: i32, commit: bool) -> bool {
    let density = density.clamp(0, 255) as u8;
    edit(view, "Mask Density", commit, |doc, path| {
        pictura_render::set_layer_mask_density(doc, path, density)
    })
}

fn layer_mask_set_feather(view: Pin<&mut PictureView>, radius: f64, commit: bool) -> bool {
    edit(view, "Mask Feather", commit, |doc, path| {
        pictura_render::set_layer_mask_feather(doc, path, radius)
    })
}

fn layer_mask_invert(view: Pin<&mut PictureView>) -> bool {
    edit(view, "Invert", true, pictura_render::invert_layer_mask)
}

fn vector_mask_density(view: &PictureView) -> i32 {
    refinement(view, pictura_render::vector_mask_refinement).map_or(-1, |(d, _)| i32::from(d))
}

fn vector_mask_feather(view: &PictureView) -> f64 {
    refinement(view, pictura_render::vector_mask_refinement).map_or(0.0, |(_, f)| f)
}

fn vector_mask_set_density(view: Pin<&mut PictureView>, density: i32, commit: bool) -> bool {
    let density = density.clamp(0, 255) as u8;
    edit(view, "Vector Mask Density", commit, |doc, path| {
        pictura_render::set_vector_mask_density(doc, path, density)
    })
}

fn vector_mask_set_feather(view: Pin<&mut PictureView>, radius: f64, commit: bool) -> bool {
    edit(view, "Vector Mask Feather", commit, |doc, path| {
        pictura_render::set_vector_mask_feather(doc, path, radius)
    })
}
