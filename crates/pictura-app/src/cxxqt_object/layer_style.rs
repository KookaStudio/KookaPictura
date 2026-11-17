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

        /// The document's global-light angle in degrees; NaN without a document.
        fn layer_style_global_angle(view: &PictureView) -> f64;

        /// The document's global-light altitude in degrees; NaN without a document.
        fn layer_style_global_altitude(view: &PictureView) -> f64;

        /// Set the document's global light, recording one "Global Light" state
        /// when either value changes. False when refused or unchanged.
        fn layer_style_set_global_light(
            view: Pin<&mut PictureView>,
            angle: i32,
            altitude: i32,
        ) -> bool;

        /// Whether the layer at `path` has a style this command can bake.
        fn layer_style_can_rasterize(view: &PictureView, path: &QString) -> bool;

        /// Bake the layer's style into its pixels and remove the style block,
        /// recording one "Rasterize Layer Style" state. False when refused.
        fn rasterize_layer_style(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Whether the layer at `path` can be exploded into style layers.
        fn layer_style_can_create_layers(view: &PictureView, path: &QString) -> bool;

        /// Explode the layer's style into pixel layers, recording one
        /// "Create Layers" state. Returns how many layers were created.
        fn layer_style_create_layers(view: Pin<&mut PictureView>, path: &QString) -> i32;

        /// The value of `key` (`"<effect>.<field>"`, `"<effect>.on"`, `"fx.visible"` or `"blending.<option>"`) on the layer at `path`; colours pack as 0xRRGGBB, blend modes and choices are indices. NaN for an unknown key or path.
        fn layer_style_value(view: &PictureView, path: &QString, key: &QString) -> f64;

        /// Set `key` on the layer at `path` and refresh its paint bounds, without a history state. False when refused or unchanged.
        fn layer_style_set(
            view: Pin<&mut PictureView>,
            path: &QString,
            key: &QString,
            value: f64,
        ) -> bool;

        /// The contour at `key` (`"bevel.glossContour"` / `"bevel.edgeContour"`) as `"x,y x,y …"`; empty for an unknown key.
        fn layer_style_contour(view: &PictureView, path: &QString, key: &QString) -> QString;

        /// Write `key`'s contour from `"x,y x,y …"`. False when refused or unchanged.
        fn layer_style_set_contour(
            view: Pin<&mut PictureView>,
            path: &QString,
            key: &QString,
            points: &QString,
        ) -> bool;

        /// The Gradient Overlay's colour stops as `"loc:rrggbb …"`; empty for an unknown key.
        fn layer_style_gradient(view: &PictureView, path: &QString, key: &QString) -> QString;

        /// Write the Gradient Overlay's colour stops from `"loc:rrggbb …"`. False when refused or unchanged.
        fn layer_style_set_gradient(
            view: Pin<&mut PictureView>,
            path: &QString,
            key: &QString,
            gradient: &QString,
        ) -> bool;

        /// Whether Copy Layer Style can capture the layer's effect blocks or channel restrictions.
        fn layer_style_can_copy(view: &PictureView, path: &QString) -> bool;

        /// The document's colour-channel names; empty unless the document is RGB.
        fn layer_style_channel_names(view: &PictureView, path: &QString) -> QStringList;

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
