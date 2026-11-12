//! The transform options bar's bridge: the session's reference point, numeric
//! scale/rotation/skew setters and readouts, and the interpolation choice
//! (Warp mode is `warp.rs`). Free functions over [`PictureView`] (their own bridge, so
//! the `PictureView` declaration list does not grow).
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
        /// The active session's mode name ("free", "skew", "distort",
        /// "perspective"), or empty without a session.
        fn transform_session_mode(view: &PictureView) -> QString;
        /// A preview backdrop patch's `[left, top]`; empty for a whole-document
        /// base.
        fn move_preview_base_origin(view: &PictureView) -> Vec<i32>;
        /// Document pixels per pixel of the Move preview's images: 1 for a
        /// full-resolution preview, `2^level` for one built from the view.
        fn move_preview_scale(view: &PictureView) -> i32;
        /// Document pixels per pixel of an open transform session's preview
        /// images: `2^level` for a large plain layer (#1297), else 1.
        fn transform_preview_scale(view: &PictureView) -> i32;
        /// Pin the threshold above which a transform preview is reduced to
        /// `pixels`; a negative value restores the default (16 Mpx).
        fn set_transform_preview_pixels_for_test(pixels: i64);
        /// A view preview's layer content bounds as `[left, top, right,
        /// bottom]`; empty for a full-resolution preview.
        fn move_preview_content_extents(view: &PictureView) -> Vec<i32>;
        /// Where the above-layer image sits, as `[left, top]`; empty when there
        /// is none.
        fn move_preview_above_origin(view: &PictureView) -> Vec<i32>;
        /// Document pixels per pixel of the above-layer image.
        fn move_preview_above_scale(view: &PictureView) -> i32;
        /// Rebuild a view preview's moving image once a drag of `(dx, dy)`
        /// shows layer pixels it does not cover; true when it changed.
        fn move_preview_follow(view: Pin<&mut PictureView>, dx: i32, dy: i32) -> bool;
        /// The session target's content rect as "left top right bottom", or
        /// empty without a session.
        fn transform_session_rect(view: &PictureView) -> QString;
        /// The layer at `path`'s rect as flat `[left, top, right, bottom]`;
        /// empty when the path does not resolve to a layer.
        fn layer_rect_extents(view: &PictureView, path: &QString) -> Vec<i32>;
        /// The paths the Move tool acts on for the view's current selection:
        /// every selected movable layer plus its link-set members, in panel
        /// order; empty when nothing is movable. The Move tool's smart guides
        /// exclude these from their references.
        fn move_group_paths(view: &PictureView) -> QStringList;
        /// Whether a transform in `mode` ("skew", "distort", "perspective", or
        /// empty/"free") may begin on `path`, including Distort's and
        /// Perspective's editable-type refusal.
        fn layer_can_transform_mode(view: &PictureView, path: &QString, mode: &QString) -> bool;
        /// The layer at `path`'s ink box (the box round its non-transparent
        /// pixels, as Align computes it) as flat `[left, top, right, bottom]`;
        /// empty when the path does not resolve or has no content.
        fn layer_content_bounds_extents(view: &PictureView, path: &QString) -> Vec<i32>;
        /// The rect the Move tool's controls box hugs on the layer at `path`
        /// (the content Free Transform trims to) as flat `[left, top, right,
        /// bottom]`, changing nothing; for a linked layer the union of its
        /// set's content. Empty when the path does not resolve.
        fn controls_box_extents(view: &PictureView, path: &QString) -> Vec<i32>;
        /// The link set containing `path` as a cache key for the controls box;
        /// empty when the layer is unlinked.
        fn layer_link_key(view: &PictureView, path: &QString) -> QString;
        /// Hit-test the box `left, top, right, bottom` at document-space
        /// `(x, y)` as `transform_hit_test` does a session's: 0..=7 scale
        /// handles, 8 rotate, 9 inside, -1 none.
        fn controls_box_hit_test(
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
            x: f64,
            y: f64,
            zoom: f64,
        ) -> i32;
        /// The session quad's four document-space corners as flat
        /// `[x0, y0, …, x3, y3]`; empty without a session.
        fn transform_quad_extents(view: &PictureView) -> Vec<f64>;
        /// The nine preview-map coefficients in `QTransform` constructor
        /// order; empty without a session.
        fn transform_preview_matrix_extents(view: &PictureView) -> Vec<f64>;
        /// The nine preview-map coefficients in `QTransform` constructor
        /// order, expressed in the moving image's own coordinates (origin at
        /// the image's top-left), for a similarity or projective session;
        /// empty without a session or a moving image.
        fn transform_preview_local_matrix_extents(view: &PictureView) -> Vec<f64>;
        /// Move the reference point to document-space `(x, y)`, leaving the
        /// rendered quad unchanged.
        fn transform_set_pivot(view: Pin<&mut PictureView>, x: f64, y: f64) -> bool;
        /// Move the reference point to the live quad's anchor `0..=8`.
        fn transform_set_pivot_preset(view: Pin<&mut PictureView>, anchor: i32) -> bool;
        fn transform_set_show_pivot(view: Pin<&mut PictureView>, on: bool);
        /// Mark whether the live transform belongs to the Move tool's controls
        /// box; such a session is per-gesture and never locks.
        fn transform_set_controls(view: Pin<&mut PictureView>, controls: bool);
        fn transform_set_width(view: Pin<&mut PictureView>, pct: f64, link: bool) -> bool;
        fn transform_set_height(view: Pin<&mut PictureView>, pct: f64, link: bool) -> bool;
        fn transform_set_angle(view: Pin<&mut PictureView>, degrees: f64) -> bool;
        fn transform_set_skew(view: Pin<&mut PictureView>, h_degrees: f64, v_degrees: f64) -> bool;
        fn transform_set_interpolation(view: Pin<&mut PictureView>, id: &QString) -> bool;
        fn transform_pivot_x(view: &PictureView) -> f64;
        fn transform_pivot_y(view: &PictureView) -> f64;
        fn transform_pivot_origin_x(view: &PictureView) -> f64;
        fn transform_pivot_origin_y(view: &PictureView) -> f64;
        fn transform_show_pivot(view: &PictureView) -> bool;
        fn transform_width_pct(view: &PictureView) -> f64;
        fn transform_height_pct(view: &PictureView) -> f64;
        fn transform_angle_degrees(view: &PictureView) -> f64;
        fn transform_skew_h_degrees(view: &PictureView) -> f64;
        fn transform_skew_v_degrees(view: &PictureView) -> f64;
        fn transform_interpolation_id(view: &PictureView) -> QString;
        /// Turn the `Free` corner drag just pressed into a Ctrl+Alt+Shift
        /// perspective drag. False (the drag stays a scale) outside a `Free`
        /// corner drag, in Warp mode, or on editable type.
        fn transform_begin_free_perspective(view: Pin<&mut PictureView>) -> bool;
        /// Turn the `Free` corner drag just pressed into a Ctrl distort drag.
        /// False outside a `Free` corner drag, in Warp mode, or on editable type.
        fn transform_begin_free_distort(view: Pin<&mut PictureView>) -> bool;
        /// Turn the `Free` side-handle drag just pressed into a Ctrl+Shift skew
        /// drag. False outside a `Free` side-handle drag or in Warp mode.
        fn transform_begin_free_skew(view: Pin<&mut PictureView>) -> bool;
        /// The live Perspective drag's readout angle in degrees; NaN without
        /// one.
        fn transform_drag_angle_degrees(view: &PictureView) -> f64;
        /// Apply an exact transform op to the open session: `"rotate180"`,
        /// `"rotate90cw"`, `"rotate90ccw"`, `"flipHorizontal"`, or
        /// `"flipVertical"`. False without a session or for an unknown op.
        fn transform_apply(view: Pin<&mut PictureView>, op: &QString) -> bool;
        /// Switch the open session's mode, keeping its box: a similarity
        /// session to Scale or Rotate, or any session to Skew, Distort, or
        /// Perspective, whose quad then carries the pending transform (#1297).
        /// False without a session, out of a projective mode into another
        /// kind, in Warp mode or with a stored warp, or into Distort or
        /// Perspective on editable type.
        fn transform_set_mode(view: Pin<&mut PictureView>, mode: &QString) -> bool;
        /// Whether [`transform_set_mode`] would accept `mode` right now; the
        /// menu uses it so an Edit > Transform mode item is disabled where the
        /// switch cannot act.
        fn transform_can_set_mode(view: &PictureView, mode: &QString) -> bool;
        /// Apply the last committed similarity transform to the open session.
        /// False without a session or a repeatable last transform.
        fn transform_repeat(view: Pin<&mut PictureView>) -> bool;
        /// Whether a repeatable last transform exists.
        fn transform_has_repeat(view: &PictureView) -> bool;
        /// Set the label the next commit records.
        fn transform_set_history_label(view: Pin<&mut PictureView>, label: &QString);
    }
}
