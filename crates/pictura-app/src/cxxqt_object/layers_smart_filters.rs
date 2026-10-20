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

use super::qobject::PictureView;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pictura_core::{Document, SmartObject};

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

/// The real layer path behind a synthetic smart-filter row path (`"0/@sf"` or
/// `"0/@sf/2"`), so a stray synthetic path never reaches the tree resolver.
fn base_path(path: &str) -> &str {
    match path.find("/@sf") {
        Some(index) => &path[..index],
        None => path,
    }
}

fn smart_object<'a>(doc: &'a Document, path: &str) -> Option<&'a SmartObject> {
    pictura_render::resolve_path(doc, base_path(path)).and_then(|layer| layer.smart_object.as_ref())
}

fn has_smart_filters(doc: &Document, path: &str) -> bool {
    smart_object(doc, path).is_some_and(|so| !so.smart_filters.is_empty())
}

fn smart_filter_count(doc: &Document, path: &str) -> i32 {
    smart_object(doc, path).map_or(0, |so| so.smart_filters.len() as i32)
}

fn smart_filter_name(doc: &Document, path: &str, i: i32) -> String {
    smart_object(doc, path)
        .and_then(|so| so.smart_filters.get(i as usize))
        .map(|filter| filter.name.clone())
        .unwrap_or_default()
}

fn smart_filter_visible(doc: &Document, path: &str, i: i32) -> bool {
    smart_object(doc, path)
        .and_then(|so| so.smart_filters.get(i as usize))
        .is_some_and(|filter| filter.enabled)
}

fn smart_filters_enabled(doc: &Document, path: &str) -> bool {
    smart_object(doc, path).is_some_and(|so| so.smart_filters_enabled)
}

fn set_smart_filter_visible(doc: &mut Document, path: &str, i: i32, visible: bool) -> bool {
    let Some(layer) = pictura_render::resolve_path_mut(doc, base_path(path)) else {
        return false;
    };
    let Some(current) = layer
        .smart_object
        .as_ref()
        .and_then(|so| so.smart_filters.get(i as usize))
        .map(|filter| filter.enabled)
    else {
        return false;
    };
    if current == visible {
        return false;
    }
    pictura_codec::set_smart_filter_enabled(layer, i as usize, visible).is_ok()
}

fn set_smart_filters_enabled(doc: &mut Document, path: &str, enabled: bool) -> bool {
    let Some(layer) = pictura_render::resolve_path_mut(doc, base_path(path)) else {
        return false;
    };
    let Some(current) = layer
        .smart_object
        .as_ref()
        .map(|so| so.smart_filters_enabled)
    else {
        return false;
    };
    if current == enabled {
        return false;
    }
    pictura_codec::set_smart_filters_enabled(layer, enabled).is_ok()
}

fn layer_has_smart_filters(view: &PictureView, layer_path: &QString) -> bool {
    view.rust()
        .doc
        .as_ref()
        .is_some_and(|doc| has_smart_filters(doc, &layer_path.to_string()))
}

fn layer_smart_filter_count(view: &PictureView, layer_path: &QString) -> i32 {
    view.rust()
        .doc
        .as_ref()
        .map_or(0, |doc| smart_filter_count(doc, &layer_path.to_string()))
}

fn layer_smart_filter_name(view: &PictureView, layer_path: &QString, i: i32) -> QString {
    let name = view
        .rust()
        .doc
        .as_ref()
        .map(|doc| smart_filter_name(doc, &layer_path.to_string(), i))
        .unwrap_or_default();
    QString::from(name.as_str())
}

fn layer_smart_filter_visible(view: &PictureView, layer_path: &QString, i: i32) -> bool {
    view.rust()
        .doc
        .as_ref()
        .is_some_and(|doc| smart_filter_visible(doc, &layer_path.to_string(), i))
}

fn layer_smart_filters_enabled(view: &PictureView, layer_path: &QString) -> bool {
    view.rust()
        .doc
        .as_ref()
        .is_some_and(|doc| smart_filters_enabled(doc, &layer_path.to_string()))
}

fn set_layer_smart_filter_visible(
    mut view: Pin<&mut PictureView>,
    layer_path: &QString,
    i: i32,
    visible: bool,
) -> bool {
    let path = layer_path.to_string();
    let changed = match view.as_mut().rust_mut().doc.as_mut() {
        Some(doc) => set_smart_filter_visible(doc, &path, i, visible),
        None => false,
    };
    if changed {
        view.as_mut().recomposite();
        view.as_mut().record("Smart Filter Visibility");
    }
    changed
}

fn set_layer_smart_filters_enabled(
    mut view: Pin<&mut PictureView>,
    layer_path: &QString,
    enabled: bool,
) -> bool {
    let path = layer_path.to_string();
    let changed = match view.as_mut().rust_mut().doc.as_mut() {
        Some(doc) => set_smart_filters_enabled(doc, &path, enabled),
        None => false,
    };
    if changed {
        view.as_mut().recomposite();
        view.as_mut().record("Smart Filter Visibility");
    }
    changed
}

fn convert_for_smart_filters(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    let changed = match view.as_mut().rust_mut().doc.as_mut() {
        Some(doc) => pictura_render::convert_for_smart_filters(doc, &path),
        None => false,
    };
    if changed {
        view.as_mut().clear_link_sets();
        view.as_mut().recomposite();
        view.as_mut().record("Convert for Smart Filters");
    }
    changed
}

fn delete_layer_smart_filter(
    mut view: Pin<&mut PictureView>,
    layer_path: &QString,
    i: i32,
) -> bool {
    let path = layer_path.to_string();
    let changed = match view.as_mut().rust_mut().doc.as_mut() {
        Some(doc) => pictura_render::delete_smart_filter(doc, base_path(&path), i.max(0) as usize),
        None => false,
    };
    if changed {
        view.as_mut().recomposite();
        view.as_mut().record("Delete Smart Filter");
    }
    changed
}

fn clear_layer_smart_filters(mut view: Pin<&mut PictureView>, layer_path: &QString) -> bool {
    let path = layer_path.to_string();
    let changed = match view.as_mut().rust_mut().doc.as_mut() {
        Some(doc) => pictura_render::clear_smart_filters(doc, base_path(&path)),
        None => false,
    };
    if changed {
        view.as_mut().recomposite();
        view.as_mut().record("Clear Smart Filters");
    }
    changed
}

fn reorder_layer_smart_filter(
    mut view: Pin<&mut PictureView>,
    layer_path: &QString,
    from: i32,
    to: i32,
) -> bool {
    if from == to || from < 0 || to < 0 {
        return false;
    }
    let path = layer_path.to_string();
    let changed = match view.as_mut().rust_mut().doc.as_mut() {
        Some(doc) => {
            pictura_render::reorder_smart_filters(doc, base_path(&path), from as usize, to as usize)
        }
        None => false,
    };
    if changed {
        view.as_mut().recomposite();
        view.as_mut().record("Reorder Smart Filters");
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{History, Snapshot};
    use pictura_core::{BitDepth, ColorMode};
    use pictura_render::apply_smart_filter_chain;

    fn camera_raw_exposure_doc() -> Document {
        let mut doc = Document::new(4, 4, ColorMode::Rgb, BitDepth::Eight);
        doc.layers = pictura_core::LayerStack::from_layers(vec![super::super::tests::pixel_layer(
            "Layer",
            4,
            4,
            (40, 80, 120),
        )]);
        assert!(pictura_render::convert_to_smart_object(&mut doc, "0"));
        let layer = pictura_render::resolve_path_mut(&mut doc, "0").expect("layer");
        let settings = pictura_core::PicturaRawSettings {
            basic: pictura_core::CrsSettings {
                exposure: Some(1.0),
                ..Default::default()
            },
            ..Default::default()
        };
        pictura_codec::attach_pictura_raw_filter(layer, &settings).expect("attach filter");
        doc
    }

    /// A round-tripped document, so the smart filter lives in a preserved `SoLd`
    /// block (the save source of truth), as it does for an imported PSD.
    fn preserved_smart_filter_doc() -> Document {
        let bytes = pictura_codec::write_psd(&camera_raw_exposure_doc()).expect("writes");
        pictura_codec::read_psd(&bytes).expect("re-reads with a preserved SoLd")
    }

    #[test]
    fn row_api_round_trips_filter_and_group_flags() {
        let mut doc = camera_raw_exposure_doc();
        assert!(has_smart_filters(&doc, "0"));
        assert_eq!(smart_filter_count(&doc, "0"), 1);
        assert_eq!(
            smart_filter_name(&doc, "0", 0),
            pictura_codec::CAMERA_RAW_FILTER_NAME
        );
        assert!(smart_filter_visible(&doc, "0", 0));
        assert!(smart_filters_enabled(&doc, "0"));

        assert!(set_smart_filter_visible(&mut doc, "0", 0, false));
        assert!(!smart_filter_visible(&doc, "0", 0));
        assert!(!set_smart_filter_visible(&mut doc, "0", 0, false));

        assert!(set_smart_filters_enabled(&mut doc, "0", false));
        assert!(!smart_filters_enabled(&doc, "0"));
        assert!(!set_smart_filters_enabled(&mut doc, "0", false));

        // Synthetic row paths resolve to the same layer.
        assert_eq!(smart_filter_count(&doc, "0/@sf"), 1);
        assert!(has_smart_filters(&doc, "0/@sf/0"));
    }

    #[test]
    fn disabled_group_bypasses_the_chain() {
        let doc = camera_raw_exposure_doc();
        let so = pictura_render::resolve_path(&doc, "0")
            .and_then(|layer| layer.smart_object.as_ref())
            .expect("smart object");
        let base = pictura_core::PixelBuffer {
            width: 4,
            height: 4,
            channels: 4,
            data: vec![30u8; 4 * 4 * 4].into(),
        };
        let filter = so.smart_filters.first().expect("filter");
        let disabled = apply_smart_filter_chain(&base, std::slice::from_ref(filter), false);
        assert_eq!(disabled.data.as_ref(), base.data.as_ref());

        let enabled = apply_smart_filter_chain(&base, std::slice::from_ref(filter), true);
        assert_ne!(enabled.data.as_ref(), base.data.as_ref());
    }

    #[test]
    fn toggles_persist_on_a_preserved_smart_object() {
        let mut doc = preserved_smart_filter_doc();
        let layer = pictura_render::resolve_path(&doc, "0").expect("layer");
        assert!(
            layer.extra_blocks.iter().any(|b| &b.key == b"SoLd"),
            "the round trip leaves a preserved SoLd"
        );

        assert!(set_smart_filter_visible(&mut doc, "0", 0, false));
        assert!(set_smart_filters_enabled(&mut doc, "0", false));

        let bytes = pictura_codec::write_psd(&doc).expect("writes");
        let back = pictura_codec::read_psd(&bytes).expect("re-reads");
        let so = pictura_render::resolve_path(&back, "0")
            .and_then(|layer| layer.smart_object.as_ref())
            .expect("smart object survives");
        assert!(!so.smart_filters[0].enabled, "filter toggle persists");
        assert!(!so.smart_filters_enabled, "group toggle persists");
    }

    #[test]
    fn app_setter_changes_the_composite_and_records_one_state() {
        let mut doc = preserved_smart_filter_doc();
        let before = pictura_render::composite_rgba(&doc);
        let mut history = History::default();

        // Replays the `set_layer_smart_filter_visible` bridge body: the real
        // `PictureView` is a C++-constructed QObject with no Rust constructor
        // (see `tests_impl::move_profile`), so the pure setter plus the bridge's
        // one capture stand in for the ffi row.
        assert!(set_smart_filter_visible(&mut doc, "0", 0, false));
        let after = pictura_render::composite_rgba(&doc);
        assert_ne!(
            before.data, after.data,
            "disabling the filter changes the composite"
        );
        history.capture(
            Snapshot {
                doc: doc.clone(),
                selection: None,
            },
            "Smart Filter Visibility",
        );
        assert_eq!(history.count(), 1, "exactly one history state");
        assert_eq!(history.label(0), "Smart Filter Visibility");
    }

    #[test]
    fn add_reorder_delete_and_clear_reach_the_preserved_descriptor() {
        let ids = |doc: &Document| {
            pictura_render::resolve_path(doc, "0")
                .and_then(|layer| layer.smart_object.as_ref())
                .map(|so| {
                    so.smart_filters
                        .iter()
                        .map(|f| f.filter_id)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        };
        let mut doc = preserved_smart_filter_doc();
        assert!(pictura_render::add_smart_filter(
            &mut doc,
            "0",
            4242,
            "Second",
            &[]
        ));
        assert_eq!(ids(&doc), vec![2683, 4242]);

        assert!(pictura_render::reorder_smart_filters(&mut doc, "0", 0, 1));
        assert_eq!(ids(&doc), vec![4242, 2683]);

        assert!(pictura_render::delete_smart_filter(&mut doc, "0", 0));
        assert_eq!(ids(&doc), vec![2683]);
        let back = pictura_codec::read_psd(&pictura_codec::write_psd(&doc).expect("writes"))
            .expect("re-reads");
        assert_eq!(
            ids(&back),
            vec![2683],
            "the edit survives a write and re-read"
        );

        assert!(pictura_render::clear_smart_filters(&mut doc, "0"));
        assert!(ids(&doc).is_empty());
    }
}
