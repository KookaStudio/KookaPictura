//! Layers panel smart-filter row bridge.
//!
//! Mirrors the read/write accessors the Layers tree needs to show a "Smart
//! Filters" group under a filtered smart object and toggle its rows. Read
//! functions resolve the real layer path; the writer functions persist the
//! `SmartFilter.enabled` / `SmartObject.smart_filters_enabled` toggle into the
//! preserved `SoLd`/`SoLE` descriptor, recomposite, and record one "Smart Filter
//! Visibility" state.
//!
//! [`PictureView`]: super::qobject::PictureView

mod imp;

use super::qobject::PictureView;
use imp::*;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;

        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;

        include!("pictura_app/src/cxxqt_object.cxxqt.h");
        #[namespace = "pictura"]
        type PictureView = super::PictureView;
    }

    #[namespace = "pictura"]
    extern "Rust" {
        /// Whether the layer at `layer_path` carries at least one smart filter.
        fn layer_has_smart_filters(view: &PictureView, layer_path: &QString) -> bool;

        /// The number of smart filters on the layer at `layer_path` (0 when
        /// there is none).
        fn layer_smart_filter_count(view: &PictureView, layer_path: &QString) -> i32;

        /// The display name of filter `i` on `layer_path`, or empty.
        fn layer_smart_filter_name(view: &PictureView, layer_path: &QString, i: i32) -> QString;

        /// Whether filter `i` on `layer_path` is individually enabled.
        fn layer_smart_filter_visible(view: &PictureView, layer_path: &QString, i: i32) -> bool;

        /// Whether the whole smart-filter group on `layer_path` is enabled.
        fn layer_smart_filters_enabled(view: &PictureView, layer_path: &QString) -> bool;

        /// How filter `i` on `layer_path` re-edits: 1 the camera-raw smart
        /// filter, 2 Lens Correction, 3 a generic filter with a dialog, 4 a
        /// Filter Gallery stack, 0 otherwise (no dialog to reopen).
        fn layer_smart_filter_edit_kind(view: &PictureView, layer_path: &QString, i: i32) -> i32;

        /// The filter kinds of generic filter `i` on `layer_path`, in apply
        /// order (one for a single filter); empty for any other filter.
        fn layer_smart_filter_kinds(
            view: &PictureView,
            layer_path: &QString,
            i: i32,
        ) -> QStringList;

        /// The slots of generic filter `i` on `layer_path`, each kind's in
        /// turn, for the dialog's prefill; empty for any other filter.
        fn layer_smart_filter_params(view: &PictureView, layer_path: &QString, i: i32)
            -> QList_f64;

        /// Start re-editing filter `i` on the active Smart Object at
        /// `layer_path`: until `smart_filter_edit_end`, the filter dialog
        /// previews the chain up to it and OK replaces it in place. False when
        /// it has no generic dialog or the layer is not the active one.
        fn smart_filter_edit_begin(
            view: Pin<&mut PictureView>,
            layer_path: &QString,
            i: i32,
        ) -> bool;

        /// End the re-edit started by `smart_filter_edit_begin`.
        fn smart_filter_edit_end(view: Pin<&mut PictureView>);

        /// Set filter `i` on `layer_path` enabled, recomposite, and record one
        /// "Smart Filter Visibility" state. False and no state when unchanged.
        fn set_layer_smart_filter_visible(
            view: Pin<&mut PictureView>,
            layer_path: &QString,
            i: i32,
            visible: bool,
        ) -> bool;

        /// Enable or disable the whole smart-filter group on `layer_path`,
        /// recomposite, and record one "Smart Filter Visibility" state. False
        /// and no state when unchanged.
        fn set_layer_smart_filters_enabled(
            view: Pin<&mut PictureView>,
            layer_path: &QString,
            enabled: bool,
        ) -> bool;

        /// `Filter > Convert for Smart Filters`: convert `path` to an embedded
        /// smart object so filters can attach. Records one "Convert for Smart
        /// Filters" state on success; false (no state) for an ineligible target.
        fn convert_for_smart_filters(view: Pin<&mut PictureView>, path: &QString) -> bool;

        /// Delete filter `i` on `layer_path`, recomposite, and record one
        /// "Delete Smart Filter" state. False and no state for an out-of-range
        /// index, a non-smart layer, or a codec refusal.
        fn delete_layer_smart_filter(
            view: Pin<&mut PictureView>,
            layer_path: &QString,
            i: i32,
        ) -> bool;

        /// Remove every smart filter on `layer_path`, recomposite, and record one
        /// "Clear Smart Filters" state. False and no state when the layer has no
        /// filters.
        fn clear_layer_smart_filters(view: Pin<&mut PictureView>, layer_path: &QString) -> bool;

        /// Whether the smart-object layer at `layer_path` carries a filter mask.
        fn layer_has_smart_filter_mask(view: &PictureView, layer_path: &QString) -> bool;

        /// Delete the filter mask on `layer_path`, recomposite, and record one
        /// "Delete Filter Mask" state. False and no state when it has none.
        fn delete_layer_smart_filter_mask(
            view: Pin<&mut PictureView>,
            layer_path: &QString,
        ) -> bool;

        /// Move filter `from` to `to` on `layer_path` (panel order is apply
        /// order), recomposite, and record one "Reorder Smart Filters" state.
        /// False and no state when the move is a no-op or out of range.
        fn reorder_layer_smart_filter(
            view: Pin<&mut PictureView>,
            layer_path: &QString,
            from: i32,
            to: i32,
        ) -> bool;
    }
}
