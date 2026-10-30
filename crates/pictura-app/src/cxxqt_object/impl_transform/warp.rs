//! Edit ▸ Transform ▸ Warp: the Free Transform session's Warp mode. Entering
//! Warp starts a one-patch custom mesh over the source box; the box's current
//! transform stays as a projective frame after the mesh (Photoshop keeps `Trnf`
//! apart from `warp`), so a Distort or Perspective made first carries over
//! exactly. A press on an anchor or handle drags it, a press anywhere else on
//! the grid pulls the surface under the pointer, and the options bar picks a
//! preset style with its bend and distortions. Commit applies the warp and the
//! frame together. Ported from photocraft's `transform_tool` Warp mode (see
//! `NOTICE.md`); its split warps and Grid menu (post-CS6) are not.
//!
//! Free functions over [`PictureView`] (their own bridge, so the `PictureView`
//! declaration list does not grow).
//!
//! [`PictureView`]: super::super::qobject::PictureView

mod imp;

use super::super::qobject::PictureView;
pub(super) use imp::*;

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
        /// Switch the session into Warp mode, reusing a stored mesh or making
        /// an identity one. False without a session, or when a new mesh needs
        /// a box frame and the box has none.
        fn transform_enter_warp(view: Pin<&mut PictureView>) -> bool;
        /// Back to the box: the warp stays in the session, so the box edits it.
        fn transform_leave_warp(view: Pin<&mut PictureView>) -> bool;
        fn transform_warp_active(view: &PictureView) -> bool;
        /// Whether the session holds a warp, whether or not Warp mode is active.
        fn transform_warp_stored(view: &PictureView) -> bool;
        /// Pick a warp style by id (`none`, `custom`, `arc`, `arcLower`…).
        /// Custom keeps the current shape as a mesh, None resets it to the
        /// box, a preset starts at the current bend (50% from a non-preset).
        fn transform_set_warp_style(view: Pin<&mut PictureView>, id: &QString) -> bool;
        /// Set a preset's bend and horizontal/vertical distortion (percent,
        /// clamped to ±100); false unless a preset is active.
        fn transform_set_warp_values(
            view: Pin<&mut PictureView>,
            bend: f64,
            h_distort: f64,
            v_distort: f64,
        ) -> bool;
        /// Set a preset's orientation; false unless a preset is active.
        fn transform_set_warp_vertical(view: Pin<&mut PictureView>, vertical: bool) -> bool;
        fn transform_warp_style_id(view: &PictureView) -> QString;
        fn transform_warp_bend(view: &PictureView) -> f64;
        fn transform_warp_h_distort(view: &PictureView) -> f64;
        fn transform_warp_v_distort(view: &PictureView) -> f64;
        fn transform_warp_vertical(view: &PictureView) -> bool;
        /// The grid's patch boundaries, each as [`CURVE_POINTS`] document-space
        /// points flattened `[x0, y0, x1, y1, …]`, curve after curve.
        fn transform_warp_grid_lines(view: &PictureView) -> Vec<f64>;
        /// A one-patch grid's rule-of-thirds guides, in the same layout.
        fn transform_warp_guide_lines(view: &PictureView) -> Vec<f64>;
        /// A custom warp's draggable points as `[kind, x, y, anchor_x,
        /// anchor_y]` records: kind 0 an anchor (its own anchor), 1 a handle,
        /// 2 a handle of the anchor being dragged. Empty for a preset.
        fn transform_warp_points(view: &PictureView) -> Vec<f64>;
    }
}
