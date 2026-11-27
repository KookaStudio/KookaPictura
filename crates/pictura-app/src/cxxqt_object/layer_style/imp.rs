use super::*;

use crate::cxxqt_object::edit::{structural, Edit};
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_codec::action_descriptor::{ActionDescriptor, ActionKey, ActionValue};
use pictura_codec::preset_library::{self, Format, Kind, Payload, Preset};
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

pub(super) fn layer_style_global_angle(view: &PictureView) -> f64 {
    view.rust()
        .session
        .doc
        .as_ref()
        .map_or(f64::NAN, |doc| doc.global_light.angle as f64)
}

pub(super) fn layer_style_global_altitude(view: &PictureView) -> f64 {
    view.rust()
        .session
        .doc
        .as_ref()
        .map_or(f64::NAN, |doc| doc.global_light.altitude as f64)
}

pub(super) fn layer_style_set_global_light(
    mut view: Pin<&mut PictureView>,
    angle: i32,
    altitude: i32,
) -> bool {
    let mut changed = false;
    view.as_mut().edit("Global Light", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        changed = pictura_render::set_document_global_light(doc, angle, altitude);
        if changed {
            Edit::Full
        } else {
            Edit::None
        }
    });
    changed
}

pub(super) fn layer_style_can_rasterize(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_rasterize_layer_style(doc, &path.to_string()))
}

pub(super) fn rasterize_layer_style(mut view: Pin<&mut PictureView>, path: &QString) -> bool {
    let path = path.to_string();
    if path.is_empty() {
        return false;
    }
    let mut changed = false;
    view.as_mut().edit("Rasterize Layer Style", |rust| {
        let before = rust
            .session
            .doc
            .as_ref()
            .and_then(|doc| pictura_render::damage::influence(doc, &path));
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        changed = pictura_render::rasterize_layer_style(doc, &path);
        if changed {
            let after = pictura_render::damage::influence(doc, &path);
            structural(before, after)
        } else {
            Edit::None
        }
    });
    changed
}

pub(super) fn layer_style_can_create_layers(view: &PictureView, path: &QString) -> bool {
    view.rust()
        .session
        .doc
        .as_ref()
        .is_some_and(|doc| pictura_render::can_create_style_layers(doc, &path.to_string()))
}

pub(super) fn layer_style_create_layers(mut view: Pin<&mut PictureView>, path: &QString) -> i32 {
    let path = path.to_string();
    if path.is_empty() {
        return 0;
    }
    let mut created = 0;
    view.as_mut().edit("Create Layers", |rust| {
        let Some(doc) = rust.session.doc.as_mut() else {
            return Edit::None;
        };
        let (paths, base) = pictura_render::create_style_layers_with_base(doc, &path);
        created = paths.len();
        if created == 0 {
            return Edit::None;
        }
        rust.select_only_layer(&base);
        Edit::Structural { damage: None }
    });
    created as i32
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

/// Run one structured style edit with the scalar setter's bounds refresh and
/// content-revision bump.
pub(super) fn apply_style_edit(
    view: Pin<&mut PictureView>,
    path: &QString,
    edit: impl FnOnce(&mut pictura_core::Layer) -> bool,
) -> bool {
    apply_document_style_edit(view, path, |doc, path| {
        pictura_render::resolve_path_mut(doc, path).is_some_and(edit)
    })
}

fn apply_document_style_edit(
    mut view: Pin<&mut PictureView>,
    path: &QString,
    edit: impl FnOnce(&mut pictura_core::Document, &str) -> bool,
) -> bool {
    let path = path.to_string();
    let before = paint_bounds(view.as_ref().get_ref(), &path);
    let changed = view
        .as_mut()
        .rust_mut()
        .session
        .doc
        .as_mut()
        .is_some_and(|doc| edit(doc, &path));
    if changed {
        refresh_paint_bounds(view.as_mut(), &path, before);
        let mut rust = view.as_mut().rust_mut();
        rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
    }
    changed
}

fn native_preset(kind: Kind, data: &[u8]) -> Option<Preset> {
    let mut import = preset_library::decode(kind, Format::Native, data).ok()?;
    (import.library.entries.len() == 1).then(|| import.library.entries.remove(0))
}

fn can_apply_preset(doc: &pictura_core::Document, path: &str) -> bool {
    pictura_render::resolve_path(doc, path)
        .is_some_and(|layer| !layer.is_group() && !layer.background)
}

pub(super) fn layer_style_apply_pattern(
    view: Pin<&mut PictureView>,
    path: &QString,
    data: &[u8],
) -> bool {
    let Some(Preset {
        name,
        payload: Payload::Pattern(pattern),
        ..
    }) = native_preset(Kind::Patterns, data)
    else {
        return false;
    };
    apply_document_style_edit(view, path, |doc, path| {
        if !can_apply_preset(doc, path) {
            return false;
        }
        let Some(id) = preset_library::embed_document_pattern(doc, &name, &pattern) else {
            return false;
        };
        let Some(layer) = pictura_render::resolve_path_mut(doc, path) else {
            return false;
        };
        pictura_render::set_layer_style_pattern(layer, &id, &name);
        true
    })
}

fn rewrite_pattern_reference(desc: &mut ActionDescriptor, source: &str, id: Option<&str>) -> bool {
    let mut found = false;
    for (key, value) in &mut desc.items {
        if matches!(&desc.class.id, ActionKey::Code(code) if code == b"Ptrn")
            && matches!(key, ActionKey::Code(code) if code == b"Idnt")
        {
            if let ActionValue::Text(value) = value {
                if value == source {
                    found = true;
                    if let Some(id) = id {
                        *value = id.into();
                    }
                }
            }
        }
        found |= rewrite_pattern_values(value, source, id);
    }
    found
}

fn rewrite_pattern_values(value: &mut ActionValue, source: &str, id: Option<&str>) -> bool {
    match value {
        ActionValue::Descriptor(desc) | ActionValue::GlobalObject(desc) => {
            rewrite_pattern_reference(desc, source, id)
        }
        ActionValue::List(values) => values.iter_mut().fold(false, |found, value| {
            rewrite_pattern_values(value, source, id) | found
        }),
        _ => false,
    }
}

pub(super) fn layer_style_apply_preset(
    view: Pin<&mut PictureView>,
    path: &QString,
    data: &[u8],
) -> bool {
    let Some(Preset {
        payload: Payload::Style(mut style),
        opaque,
        ..
    }) = native_preset(Kind::Styles, data)
    else {
        return false;
    };
    let Ok(patterns) = preset_library::style_patterns(&opaque) else {
        return false;
    };
    apply_document_style_edit(view, path, |doc, path| {
        if !can_apply_preset(doc, path) {
            return false;
        }
        let before = doc.layer_section_extra.clone();
        for (name, pattern) in &patterns {
            if !rewrite_pattern_reference(&mut style, &pattern.pattern_id, None) {
                continue;
            }
            let Some(id) = preset_library::embed_document_pattern(doc, name, pattern) else {
                doc.layer_section_extra = before;
                return false;
            };
            rewrite_pattern_reference(&mut style, &pattern.pattern_id, Some(&id));
        }
        let Some(layer) = pictura_render::resolve_path_mut(doc, path) else {
            return false;
        };
        let mut bytes = vec![0, 0, 0, 0, 0, 0, 0, 16];
        bytes.extend(pictura_codec::action_descriptor::write_action_descriptor(
            &style, true,
        ));
        pictura_codec::edit_layer(layer, |parts| {
            parts
                .extra_blocks
                .retain(|block| !matches!(&block.key, b"lfx2" | b"lrFX" | b"lmfx"));
            parts.extra_blocks.push(pictura_core::LayerBlock {
                key: *b"lfx2",
                data: bytes.into(),
            });
        });
        true
    })
}

pub(super) fn layer_style_contour(view: &PictureView, path: &QString, key: &QString) -> QString {
    let key = key.to_string();
    with_layer(view, path, |layer| {
        pictura_render::layer_style_contour(layer, &key)
    })
    .flatten()
    .map_or_else(QString::default, |text| QString::from(text.as_str()))
}

pub(super) fn layer_style_set_contour(
    view: Pin<&mut PictureView>,
    path: &QString,
    key: &QString,
    points: &QString,
) -> bool {
    let (key, points) = (key.to_string(), points.to_string());
    apply_style_edit(view, path, |layer| {
        pictura_render::set_layer_style_contour(layer, &key, &points)
    })
}

pub(super) fn layer_style_gradient(view: &PictureView, path: &QString, key: &QString) -> QString {
    let key = key.to_string();
    with_layer(view, path, |layer| {
        pictura_render::layer_style_gradient(layer, &key)
    })
    .flatten()
    .map_or_else(QString::default, |text| QString::from(text.as_str()))
}

pub(super) fn layer_style_set_gradient(
    view: Pin<&mut PictureView>,
    path: &QString,
    key: &QString,
    gradient: &QString,
) -> bool {
    let (key, gradient) = (key.to_string(), gradient.to_string());
    apply_style_edit(view, path, |layer| {
        pictura_render::set_layer_style_gradient(layer, &key, &gradient)
    })
}

/// Whether Copy Layer Style can capture something: an effect block or a
/// channel restriction (the channel-only style has no `lfx2`/`lrFX`).
pub(super) fn layer_style_can_copy(view: &PictureView, path: &QString) -> bool {
    with_layer(view, path, |layer| {
        !layer.background
            && !layer.is_group()
            && (pictura_render::has_layer_style(layer) || !layer.restricted_channels.is_empty())
    })
    .unwrap_or(false)
}

pub(super) fn layer_style_channel_names(view: &PictureView, path: &QString) -> QStringList {
    let mut names = QStringList::default();
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return names;
    };
    if pictura_render::resolve_path(doc, &path.to_string()).is_none() {
        return names;
    }
    for name in channel_names(doc.mode) {
        names.append(QString::from(*name));
    }
    names
}

fn channel_names(mode: pictura_core::ColorMode) -> &'static [&'static str] {
    match mode {
        pictura_core::ColorMode::Rgb => &["Red", "Green", "Blue"],
        _ => &[],
    }
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
        let Some((width, height)) = view
            .as_ref()
            .get_ref()
            .rust()
            .session
            .doc
            .as_ref()
            .map(|doc| (doc.width, doc.height))
        else {
            return;
        };
        let Some(rect) = bounds.clip_to(width, height) else {
            return;
        };
        view.as_mut().refresh_region(rect);
        return;
    };
    let union = a.union(b);
    let Some((width, height)) = view
        .as_ref()
        .get_ref()
        .rust()
        .session
        .doc
        .as_ref()
        .map(|doc| (doc.width, doc.height))
    else {
        return;
    };
    if let Some(rect) = union.clip_to(width, height) {
        view.as_mut().refresh_region(rect);
    }
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

fn default_blend_if() -> pictura_core::BlendIf {
    pictura_core::BlendIf {
        composite_source: (0, 65535),
        composite_dest: (0, 65535),
        channel_ranges: Vec::new(),
    }
}

pub(super) fn layer_style_blend_if_status(view: &PictureView, path: &QString) -> i32 {
    let Some(doc) = view.rust().session.doc.as_ref() else {
        return -1;
    };
    if !matches!(
        doc.document_color_mode(),
        pictura_core::ColorMode::Rgb | pictura_core::ColorMode::Grayscale
    ) {
        return -1;
    }
    let Some(layer) = pictura_render::resolve_path(doc, &path.to_string()) else {
        return -1;
    };
    if layer.is_group() && layer.blend == pictura_core::BlendMode::PassThrough {
        return -2;
    }
    if layer.blend_if.is_none() && !layer.blending_ranges.is_empty() {
        1
    } else {
        0
    }
}

pub(super) fn layer_style_blend_if_word(
    view: &PictureView,
    path: &QString,
    channel: i32,
    dest: bool,
    white: bool,
) -> i32 {
    if layer_style_blend_if_status(view, path) != 0 || !(0..=3).contains(&channel) {
        return -1;
    }
    with_layer(view, path, |layer| {
        let default = default_blend_if();
        let ranges = layer.blend_if.as_ref().unwrap_or(&default);
        let full = ((0, 65535), (0, 65535));
        let pair = if channel == 0 {
            if dest {
                ranges.composite_dest
            } else {
                ranges.composite_source
            }
        } else {
            let group = ranges
                .channel_ranges
                .get((channel - 1) as usize)
                .unwrap_or(&full);
            if dest {
                group.1
            } else {
                group.0
            }
        };
        i32::from(if white { pair.1 } else { pair.0 })
    })
    .unwrap_or(-1)
}

pub(super) fn layer_style_blend_if_range(
    view: Pin<&mut PictureView>,
    path: &QString,
    channel: i32,
    dest: bool,
    black: i32,
    white: i32,
) -> bool {
    if layer_style_blend_if_status(view.as_ref().get_ref(), path) != 0
        || !(0..=3).contains(&channel)
        || !(0..=65535).contains(&black)
        || !(0..=65535).contains(&white)
    {
        return false;
    }
    let [b0, b1] = (black as u16).to_be_bytes();
    let [w0, w1] = (white as u16).to_be_bytes();
    if b0 > b1 || b1 > w0 || w0 > w1 {
        return false;
    }
    apply_style_edit(view, path, |layer| {
        let mut ranges = layer.blend_if.clone().unwrap_or_else(default_blend_if);
        let slot = if channel == 0 {
            if dest {
                &mut ranges.composite_dest
            } else {
                &mut ranges.composite_source
            }
        } else {
            ranges.channel_ranges.resize(
                (channel as usize).max(ranges.channel_ranges.len()),
                ((0, 65535), (0, 65535)),
            );
            let group = &mut ranges.channel_ranges[(channel - 1) as usize];
            if dest {
                &mut group.1
            } else {
                &mut group.0
            }
        };
        let next = (black as u16, white as u16);
        if *slot == next {
            return false;
        }
        *slot = next;
        layer.blend_if = Some(ranges);
        true
    })
}

pub(super) fn layer_style_replace_blend_if(view: Pin<&mut PictureView>, path: &QString) -> bool {
    if layer_style_blend_if_status(view.as_ref().get_ref(), path) != 1 {
        return false;
    }
    apply_style_edit(view, path, |layer| {
        layer.blending_ranges.clear();
        layer.blend_if = Some(default_blend_if());
        true
    })
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

        let raw = pictura_codec::layer_from_parts(pictura_core::LayerParts {
            blending_ranges: vec![1, 2, 3].into(),
            ..Default::default()
        });
        assert!(
            layer_is_blend_if_customised(&raw),
            "a raw block with no typed view is customised"
        );
    }
}
