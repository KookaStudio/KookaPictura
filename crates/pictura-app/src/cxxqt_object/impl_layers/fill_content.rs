//! Fill-content bridge: read and edit a Solid Color, Gradient, or Pattern fill
//! layer, list the document's patterns, and create a fill layer with the
//! editor's chosen content. Free functions over a [`PictureView`] (their own
//! bridge, so the `PictureView` declaration list does not grow). Each write
//! recomposites and records one state.

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
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// `"solid-color"`, `"gradient"`, `"pattern"`, or empty for a non-fill.
        fn layer_fill_kind(view: &PictureView, path: &QString) -> QString;
        /// A Solid Color fill's packed ARGB, or 0.
        fn layer_fill_color(view: &PictureView, path: &QString) -> u32;
        /// `kind|angle|scale|reverse|loc:rrggbb,…`; empty for a non-gradient.
        fn layer_fill_gradient(view: &PictureView, path: &QString) -> QString;
        /// `id|scale|link|x|y`; empty for a non-pattern.
        fn layer_fill_pattern(view: &PictureView, path: &QString) -> QString;
        fn set_layer_fill_color(view: Pin<&mut PictureView>, path: &QString, color: u32) -> bool;
        fn set_layer_fill_gradient(
            view: Pin<&mut PictureView>,
            path: &QString,
            payload: &QString,
        ) -> bool;
        fn set_layer_fill_pattern(
            view: Pin<&mut PictureView>,
            path: &QString,
            payload: &QString,
        ) -> bool;
        /// Whether the document carries any `Patt`/`Pat2`/`Pat3` pattern.
        fn layer_has_patterns(view: &PictureView) -> bool;
        /// Whether the layer is a fill content layer the fill editors can edit
        /// (a shape's vector fill is excluded).
        fn layer_is_editable_fill(view: &PictureView, path: &QString) -> bool;
        /// The document's pattern ids in decode order.
        fn document_pattern_ids(view: &PictureView) -> QStringList;
        /// A `size`-bounded thumbnail for pattern `id`, or a null image.
        fn pattern_thumbnail(view: &PictureView, id: &QString, size: i32) -> QImage;
        /// Create a fill layer of `kind` from the editor payload, anchored at
        /// `selection_path` (empty = top). One history state.
        #[allow(clippy::too_many_arguments)]
        fn add_fill_layer(
            view: Pin<&mut PictureView>,
            kind: &QString,
            payload: &QString,
            selection_path: &QString,
            name: &QString,
            color: i32,
            blend_key: &QString,
            opacity: i32,
            clipping: bool,
        ) -> QString;
    }
}
