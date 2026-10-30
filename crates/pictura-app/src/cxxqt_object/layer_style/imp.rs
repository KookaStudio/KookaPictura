use super::*;

use crate::cxxqt_object::edit::Edit;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use std::sync::Mutex;

/// The app-wide copied style (Copy Layer Style / Paste Layer Style).
pub(super) static COPIED: Mutex<Option<pictura_render::LayerStyle>> = Mutex::new(None);

pub(super) fn with_layer<T>(
    view: &PictureView,
    path: &QString,
    f: impl FnOnce(&pictura_core::Layer) -> T,
) -> Option<T> {
    let doc = view.rust().session.doc.as_ref()?;
    pictura_render::resolve_path(doc, &path.to_string()).map(f)
}

pub(super) fn layer_style_can_edit(view: &PictureView, path: &QString) -> bool {
    with_layer(view, path, |layer| !layer.background).unwrap_or(false)
}

pub(super) fn layer_style_has(view: &PictureView, path: &QString) -> bool {
    with_layer(view, path, pictura_render::has_layer_style).unwrap_or(false)
}

/// A layer's Blend If is customised when its typed view is non-default, or when
/// a raw `blending_ranges` block is present but could not be parsed into a view
/// (empty means absent).
pub(in crate::cxxqt_object) fn layer_is_blend_if_customised(layer: &pictura_core::Layer) -> bool {
    match layer.blend_if.as_ref() {
        Some(view) => !view.is_default(),
        None => !layer.blending_ranges.is_empty(),
    }
}

pub(super) fn layer_style_effect_names() -> QStringList {
    let mut names = QStringList::default();
    for name in pictura_render::layer_style_effect_names() {
        names.append(QString::from(name));
    }
    names
}

pub(super) fn layer_style_value(view: &PictureView, path: &QString, key: &QString) -> f64 {
    let key = key.to_string();
    with_layer(view, path, |layer| {
        pictura_render::layer_style_value(layer, &key)
    })
    .flatten()
    .unwrap_or(f64::NAN)
}

pub(super) fn layer_style_set(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    key: &QString,
    value: f64,
) -> bool {
    let (path, key) = (path.to_string(), key.to_string());
    let before = paint_bounds(view.as_ref().get_ref(), &path);
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| pictura_render::set_document_layer_style_value(doc, &path, &key, value));
    if changed {
        refresh_paint_bounds(view.as_mut(), &path, before);
        let mut rust = view.as_mut().rust_mut();
        rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
    }
    changed
}

/// The layer's paint bounds before or after a style change.
pub(super) fn paint_bounds(view: &PictureView, path: &str) -> Option<pictura_core::PsdRect> {
    view.rust()
        .session
        .doc
        .as_ref()
        .and_then(|doc| pictura_render::layer_paint_bounds(doc, path))
}

/// Recomposite only the union of the bounds before and after a style edit,
/// clipped to the canvas.
pub(super) fn refresh_paint_bounds(
    mut view: Pin<&mut PictureView>,
    path: &str,
    before: Option<pictura_core::PsdRect>,
) {
    let after = paint_bounds(view.as_ref().get_ref(), path);
    let (Some(a), Some(b)) = (before, after) else {
        let Some(bounds) = before.or(after) else {
            return;
        };
        let Some(rect) = clip_to_canvas(view.as_ref().get_ref(), bounds) else {
            return;
        };
        view.as_mut().refresh_region(rect);
        return;
    };
    let union = pictura_core::PsdRect {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    };
    if let Some(rect) = clip_to_canvas(view.as_ref().get_ref(), union) {
        view.as_mut().refresh_region(rect);
    }
}

pub(super) fn clip_to_canvas(
    view: &PictureView,
    rect: pictura_core::PsdRect,
) -> Option<pictura_core::PsdRect> {
    let doc = view.rust().session.doc.as_ref()?;
    let clipped = pictura_core::PsdRect {
        left: rect.left.max(0),
        top: rect.top.max(0),
        right: rect.right.min(doc.width.min(i32::MAX as u32) as i32),
        bottom: rect.bottom.min(doc.height.min(i32::MAX as u32) as i32),
    };
    (clipped.width() > 0 && clipped.height() > 0).then_some(clipped)
}

pub(super) fn layer_style_set_blend_if(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    source_black: i32,
    source_white: i32,
) -> bool {
    const FULL: (u16, u16) = (0, 65535);
    let path = path.to_string();
    let before = paint_bounds(view.as_ref().get_ref(), &path);
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
        refresh_paint_bounds(view.as_mut(), &path, before);
        let mut rust = view.as_mut().rust_mut();
        rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
    }
    changed
}

pub(super) fn layer_style_pattern_names() -> QStringList {
    let mut names = QStringList::default();
    for name in pictura_render::layer_style_pattern_names() {
        names.append(QString::from(name));
    }
    names
}

pub(super) fn layer_style_commit(mut view: Pin<&mut PictureView>, label: &QString) {
    let label = label.to_string();
    view.as_mut().edit(&label, |_| Edit::Record);
}

pub(super) fn layer_style_cancel(mut view: Pin<&mut PictureView>) {
    let current = view.history_index();
    view.as_mut().history_jump(current);
}

pub(super) fn copied() -> std::sync::MutexGuard<'static, Option<pictura_render::LayerStyle>> {
    COPIED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn layer_style_copy(view: &PictureView, path: &QString) -> bool {
    let Some(style) = with_layer(view, path, pictura_render::copy_layer_style).flatten() else {
        return false;
    };
    *copied() = Some(style);
    true
}

pub(super) fn layer_style_can_paste() -> bool {
    copied().is_some()
}

pub(super) fn layer_style_paste(view: Pin<&mut PictureView>, paths: &QStringList) -> i32 {
    let Some(style) = copied().clone() else {
        return 0;
    };
    view.batch_changed(paths, "Paste Layer Style", |doc, paths| {
        pictura_render::paste_layer_style(doc, paths, &style)
    })
}

pub(super) fn layer_style_clear(view: Pin<&mut PictureView>, paths: &QStringList) -> i32 {
    view.batch_changed(
        paths,
        "Clear Layer Style",
        pictura_render::clear_layer_style,
    )
}

pub(super) fn layer_style_scale(
    view: Pin<&mut PictureView>,
    paths: &QStringList,
    percent: f64,
) -> i32 {
    view.batch_changed(paths, "Scale Effects", |doc, paths| {
        pictura_render::scale_layer_effects(doc, paths, percent)
    })
}

pub(super) fn layer_style_any_visible(view: &PictureView, visible: bool) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::any_effects_visible(doc, visible))
}

pub(super) fn layer_style_set_all_visible(mut view: Pin<&mut PictureView>, visible: bool) -> i32 {
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
            blending_ranges: vec![1, 2, 3].into(),
            ..Layer::default()
        };
        assert!(
            layer_is_blend_if_customised(&raw),
            "a raw block with no typed view is customised"
        );
    }
}
