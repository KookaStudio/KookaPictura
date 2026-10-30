//! The Layer Style dialog and the Layer > Layer Style commands. Free functions
//! over a [`PictureView`] (their own bridge, so the `PictureView` declaration
//! list does not grow).
//!
//! The dialog edits live: each control writes one `"<effect>.<field>"` value
//! and recomposites without a history state; OK records the visit as one state
//! and Cancel re-applies the current state, dropping every live edit. The
//! copied style is app-wide, as CS6's is, so it pastes across documents.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
pub(super) use imp::*;

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
        /// Whether the layer at `path` can carry a style (not a group or the Background).
        fn layer_style_can_edit(view: &PictureView, path: &QString) -> bool;

        /// Whether the layer at `path` carries any effect, on or off.
        fn layer_style_has(view: &PictureView, path: &QString) -> bool;

        /// The ten effect dialog keys, in CS6 list order.
        fn layer_style_effect_names() -> QStringList;

        /// The value of `key` (`"<effect>.<field>"`, `"<effect>.on"`, `"fx.visible"` or `"blending.<option>"`) on the layer at `path`; colours pack as 0xRRGGBB, blend modes and choices are indices. NaN for an unknown key or path.
        fn layer_style_value(view: &PictureView, path: &QString, key: &QString) -> f64;

        /// Set `key` on the layer at `path` and refresh its paint bounds, without a history state. False when refused or unchanged.
        fn layer_style_set(
            view: Pin<&mut PictureView>,
            path: &QString,
            key: &QString,
            value: f64,
        ) -> bool;

        /// Set the composite-source Blend If range on the layer at `path` and
        /// refresh its paint bounds, without a history state; `(0, 65535)` restores the
        /// default. False when refused or unchanged.
        fn layer_style_set_blend_if(
            view: Pin<&mut PictureView>,
            path: &QString,
            source_black: i32,
            source_white: i32,
        ) -> bool;

        /// The built-in patterns a Pattern Overlay offers, in `patternOverlay.pattern` order.
        fn layer_style_pattern_names() -> QStringList;

        /// Record the live edits as one state named `label`.
        fn layer_style_commit(view: Pin<&mut PictureView>, label: &QString);

        /// Drop every live edit by re-applying the current history state.
        fn layer_style_cancel(view: Pin<&mut PictureView>);

        /// Copy the style of the layer at `path` to the app-wide style clipboard. False when it cannot carry one.
        fn layer_style_copy(view: &PictureView, path: &QString) -> bool;

        /// Whether a copied style is waiting to be pasted.
        fn layer_style_can_paste() -> bool;

        /// Replace the style of every layer in `paths` with the copied one. Returns how many changed.
        fn layer_style_paste(view: Pin<&mut PictureView>, paths: &QStringList) -> i32;

        /// Remove every effect from `paths`. Returns how many changed.
        fn layer_style_clear(view: Pin<&mut PictureView>, paths: &QStringList) -> i32;

        /// Scale the pixel-sized effect parameters on `paths` by `percent`. Returns how many changed.
        fn layer_style_scale(view: Pin<&mut PictureView>, paths: &QStringList, percent: f64)
            -> i32;

        /// Whether any layer's effects are currently shown (`visible`) or hidden by the master switch.
        fn layer_style_any_visible(view: &PictureView, visible: bool) -> bool;

        /// Show or hide every layer's effects. Returns how many layers changed.
        fn layer_style_set_all_visible(view: Pin<&mut PictureView>, visible: bool) -> i32;
    }
}
