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

use super::qobject::PictureView;
use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use std::sync::Mutex;

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

        /// Set `key` on the layer at `path` and recomposite, without a history state. False when refused or unchanged.
        fn layer_style_set(
            view: Pin<&mut PictureView>,
            path: &QString,
            key: &QString,
            value: f64,
        ) -> bool;

        /// Set the composite-source Blend If range on the layer at `path` and
        /// recomposite, without a history state; `(0, 65535)` restores the
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

/// The app-wide copied style (Copy Layer Style / Paste Layer Style).
static COPIED: Mutex<Option<pictura_render::LayerStyle>> = Mutex::new(None);

fn with_layer<T>(
    view: &PictureView,
    path: &QString,
    f: impl FnOnce(&pictura_core::Layer) -> T,
) -> Option<T> {
    let doc = view.rust().session.doc.as_ref()?;
    pictura_render::resolve_path(doc, &path.to_string()).map(f)
}

fn layer_style_can_edit(view: &PictureView, path: &QString) -> bool {
    with_layer(view, path, |layer| !layer.background).unwrap_or(false)
}

fn layer_style_has(view: &PictureView, path: &QString) -> bool {
    with_layer(view, path, pictura_render::has_layer_style).unwrap_or(false)
}

/// A layer's Blend If is customised when its typed view is non-default, or when
/// a raw `blending_ranges` block is present but could not be parsed into a view
/// (empty means absent).
pub(super) fn layer_is_blend_if_customised(layer: &pictura_core::Layer) -> bool {
    match layer.blend_if.as_ref() {
        Some(view) => !view.is_default(),
        None => !layer.blending_ranges.is_empty(),
    }
}

fn layer_style_effect_names() -> QStringList {
    let mut names = QStringList::default();
    for name in pictura_render::layer_style_effect_names() {
        names.append(QString::from(name));
    }
    names
}

fn layer_style_value(view: &PictureView, path: &QString, key: &QString) -> f64 {
    let key = key.to_string();
    with_layer(view, path, |layer| {
        pictura_render::layer_style_value(layer, &key)
    })
    .flatten()
    .unwrap_or(f64::NAN)
}

fn layer_style_set(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    key: &QString,
    value: f64,
) -> bool {
    let (path, key) = (path.to_string(), key.to_string());
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| pictura_render::set_document_layer_style_value(doc, &path, &key, value));
    if changed {
        view.as_mut().recomposite();
        let mut rust = view.as_mut().rust_mut();
        rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
    }
    changed
}

fn layer_style_set_blend_if(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    source_black: i32,
    source_white: i32,
) -> bool {
    const FULL: (u16, u16) = (0, 65535);
    let path = path.to_string();
    let range = pictura_core::BlendIf {
        composite_source: (
            source_black.clamp(0, 65535) as u16,
            source_white.clamp(0, 65535) as u16,
        ),
        composite_dest: FULL,
        channel_ranges: Vec::new(),
    };
    let next = (!range.is_default()).then_some(range);
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| {
            let Some(layer) = pictura_render::resolve_path_mut(doc, &path) else {
                return false;
            };
            if layer.blend_if == next {
                return false;
            }
            layer.blend_if = next;
            layer.blending_ranges.clear();
            true
        });
    if changed {
        view.as_mut().recomposite();
        let mut rust = view.as_mut().rust_mut();
        rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
    }
    changed
}

fn layer_style_pattern_names() -> QStringList {
    let mut names = QStringList::default();
    for name in pictura_render::layer_style_pattern_names() {
        names.append(QString::from(name));
    }
    names
}

fn layer_style_commit(mut view: Pin<&mut PictureView>, label: &QString) {
    let label = label.to_string();
    view.as_mut().edit(&label, |_| Edit::Record);
}

fn layer_style_cancel(mut view: Pin<&mut PictureView>) {
    let current = view.history_index();
    view.as_mut().history_jump(current);
}

fn copied() -> std::sync::MutexGuard<'static, Option<pictura_render::LayerStyle>> {
    COPIED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn layer_style_copy(view: &PictureView, path: &QString) -> bool {
    let Some(style) = with_layer(view, path, pictura_render::copy_layer_style).flatten() else {
        return false;
    };
    *copied() = Some(style);
    true
}

fn layer_style_can_paste() -> bool {
    copied().is_some()
}

fn layer_style_paste(view: Pin<&mut PictureView>, paths: &QStringList) -> i32 {
    let Some(style) = copied().clone() else {
        return 0;
    };
    view.batch_changed(paths, "Paste Layer Style", |doc, paths| {
        pictura_render::paste_layer_style(doc, paths, &style)
    })
}

fn layer_style_clear(view: Pin<&mut PictureView>, paths: &QStringList) -> i32 {
    view.batch_changed(
        paths,
        "Clear Layer Style",
        pictura_render::clear_layer_style,
    )
}

fn layer_style_scale(view: Pin<&mut PictureView>, paths: &QStringList, percent: f64) -> i32 {
    view.batch_changed(paths, "Scale Effects", |doc, paths| {
        pictura_render::scale_layer_effects(doc, paths, percent)
    })
}

fn layer_style_any_visible(view: &PictureView, visible: bool) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::any_effects_visible(doc, visible))
}

fn layer_style_set_all_visible(mut view: Pin<&mut PictureView>, visible: bool) -> i32 {
    let label = if visible {
        "Show All Effects"
    } else {
        "Hide All Effects"
    };
    let mut changed = 0;
    view.as_mut().edit(label, |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        changed = pictura_render::set_all_effects_visible(doc, visible);
        if changed > 0 {
            Edit::Full
        } else {
            Edit::None
        }
    });
    changed as i32
}

#[cfg(test)]
mod tests {
    use super::layer_is_blend_if_customised;
    use pictura_core::{BlendIf, Layer};

    fn view(composite_source: (u16, u16)) -> BlendIf {
        BlendIf {
            composite_source,
            composite_dest: (0, 65535),
            channel_ranges: Vec::new(),
        }
    }

    #[test]
    fn blend_if_customised_only_when_non_default() {
        let mut layer = Layer::default();
        assert!(!layer_is_blend_if_customised(&layer), "absent is default");

        layer.blend_if = Some(view((0, 65535)));
        assert!(
            !layer_is_blend_if_customised(&layer),
            "an all-full view is the default and must not badge"
        );

        layer.blend_if = Some(view((0, 40000)));
        assert!(layer_is_blend_if_customised(&layer), "narrowed range");

        let raw = Layer {
            blending_ranges: vec![1, 2, 3],
            ..Layer::default()
        };
        assert!(
            layer_is_blend_if_customised(&raw),
            "a raw block with no typed view is customised"
        );
    }
}
