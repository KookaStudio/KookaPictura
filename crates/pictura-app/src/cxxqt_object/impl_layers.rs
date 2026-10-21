use super::edit::Edit;
use super::helpers::*;
use super::helpers_composite::*;
use super::qobject;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_core::{BlendMode, ColorLabel, Document, Layer};

mod auto_select;
mod layer_masks;
mod mask_commands;
pub(in crate::cxxqt_object) use mask_commands::coverage_at;
mod layers_surface;
mod mask_refine;
mod matting;
mod smart_object_actions;
mod target_transitions;
mod vector_masks;

fn property_compositing(doc: &Document, path: &str) -> Option<(BlendMode, u8, u8)> {
    pictura_render::resolve_path(doc, path).map(|layer| (layer.blend, layer.opacity, layer.fill))
}

fn changed_property_region(
    doc: &Document,
    refs: &[&str],
    before: &[Option<(BlendMode, u8, u8)>],
) -> Option<Option<pictura_core::PsdRect>> {
    refs.iter()
        .zip(before)
        .filter(|(path, before)| property_compositing(doc, path) != **before)
        .try_fold(None, |union: Option<pictura_core::PsdRect>, (path, _)| {
            let rect = pictura_render::resolve_path(doc, path).and_then(layer_visibility_region)?;
            Some(Some(union.map_or(rect, |u| union_rect(u, rect))))
        })
}

impl qobject::PictureView {
    pub fn layer_count(&self) -> i32 {
        self.rust()
            .session
            .doc
            .as_ref()
            .map_or(0, |d| d.layers.len() as i32)
    }

    pub fn topmost_pixel_layer_index(&self) -> i32 {
        self.rust()
            .session
            .doc
            .as_ref()
            .and_then(topmost_pixel_layer_index)
            .map_or(-1, |index| index as i32)
    }

    pub fn layer_name(&self, i: i32) -> QString {
        self.layer(i)
            .map(|l| QString::from(l.name.as_str()))
            .unwrap_or_default()
    }

    pub fn layer_kind(&self, i: i32) -> QString {
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return QString::default();
        };
        match doc.layers.get(i as usize) {
            Some(layer) => layer_kind_str(doc, &i.to_string(), layer),
            None => QString::default(),
        }
    }

    pub fn layer_visible(&self, i: i32) -> bool {
        self.layer(i).is_some_and(|l| l.visible)
    }

    pub fn set_layer_visible(mut self: Pin<&mut Self>, i: i32, visible: bool) {
        self.as_mut().edit("Layer Visibility", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let Some(layer) = doc.layers.get_mut(i as usize) else {
                return Edit::None;
            };
            layer.visible = visible;
            match layer_visibility_region(layer) {
                Some(rect) => Edit::Regions {
                    rects: vec![rect],
                    notify: false,
                },
                None => Edit::Full,
            }
        });
    }

    pub fn layer_blend(&self, i: i32) -> QString {
        self.layer(i)
            .map(|l| QString::from(blend_key(l.blend).as_str()))
            .unwrap_or_default()
    }

    pub fn set_layer_blend(mut self: Pin<&mut Self>, i: i32, key: &QString) -> bool {
        let key = key.to_string();
        let bytes = key.as_bytes();
        if bytes.len() != 4 {
            return false;
        }
        let Some(mode) = BlendMode::from_psd_key([bytes[0], bytes[1], bytes[2], bytes[3]]) else {
            return false;
        };
        let label = format!("Blending Change: {}", mode.name());
        self.as_mut().mutate_layer(i, &label, |layer, _| {
            layer.blend = mode;
            true
        })
    }

    pub fn layer_opacity(&self, i: i32) -> i32 {
        self.layer(i).map_or(0, |l| l.opacity as i32)
    }

    pub fn set_layer_opacity(mut self: Pin<&mut Self>, i: i32, value: i32) -> bool {
        let value = value.clamp(0, 255) as u8;
        self.as_mut()
            .mutate_layer(i, "Opacity", |layer, background| {
                if background || layer.lock.is_all() {
                    false
                } else {
                    layer.opacity = value;
                    true
                }
            })
    }

    pub fn layer_fill(&self, i: i32) -> i32 {
        self.layer(i).map_or(0, |l| l.fill as i32)
    }

    pub fn set_layer_fill(mut self: Pin<&mut Self>, i: i32, value: i32) -> bool {
        let value = value.clamp(0, 255) as u8;
        self.as_mut()
            .mutate_layer(i, "Fill Opacity", |layer, background| {
                if layer.is_group || background || layer.lock.is_all() {
                    false
                } else {
                    layer.fill = value;
                    true
                }
            })
    }

    pub fn layer_lock(&self, i: i32) -> i32 {
        self.layer(i).map_or(0, |l| l.lock.bits() as i32)
    }

    pub fn set_layer_lock(mut self: Pin<&mut Self>, i: i32, flag: &QString, on: bool) -> bool {
        let Some(mut bit) = lock_bit(flag.to_string().as_str()) else {
            return false;
        };
        self.as_mut().mutate_meta(i, "Lock", |layer, background| {
            if background {
                return false;
            }
            // CS6 forces Transparency and Image on for type and shape layers,
            // so an unlock request keeps them set; other bits still clear.
            if !on && pictura_render::has_forced_locks(layer) {
                bit &= !(pictura_core::LockFlags::TRANSPARENCY | pictura_core::LockFlags::PIXELS);
            }
            if bit == 0 {
                return false;
            }
            layer.lock = layer.lock.with(bit, on);
            true
        })
    }

    pub fn layer_color(&self, i: i32) -> i32 {
        self.layer(i).map_or(0, |l| l.color.to_byte() as i32)
    }

    pub fn set_layer_color(mut self: Pin<&mut Self>, i: i32, value: i32) -> bool {
        if !(0..=7).contains(&value) {
            return false;
        }
        self.as_mut()
            .mutate_meta(i, "Layer Color", |layer, background| {
                if background {
                    false
                } else {
                    layer.color = ColorLabel::from_byte(value as u8);
                    true
                }
            })
    }

    pub fn set_layer_name(mut self: Pin<&mut Self>, i: i32, name: &QString) -> bool {
        let name = name.to_string();
        let edit = self.as_mut().edit("Rename Layer", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let Some(layer) = doc.layers.get_mut(i as usize) else {
                return Edit::None;
            };
            layer.name = name;
            Edit::Meta
        });
        !matches!(edit, Edit::None)
    }

    /// Move the top-level layer at index `i` by `delta`. The render-side
    /// `move_path` shifts the node and clamps at the container bounds, so
    /// `|delta| > 1` shifts rather than swapping two endpoints, and locked
    /// layers or the Background are refused.
    pub fn move_layer(mut self: Pin<&mut Self>, i: i32, delta: i32) -> bool {
        let edit = self.as_mut().edit("Reorder Layer", |rust| {
            let moved = {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                pictura_render::move_path(doc, &i.to_string(), delta)
            };
            if !moved {
                return Edit::None;
            }
            let len = rust.session.doc.as_ref().map_or(0, |doc| doc.layers.len());
            if len > 0 {
                let index = (i + delta).clamp(0, len as i32 - 1) as usize;
                rust.select_only_layer(&index.to_string());
            }
            Edit::Structural
        });
        !matches!(edit, Edit::None)
    }

    pub fn add_layer(mut self: Pin<&mut Self>, above: i32) -> i32 {
        let mut created = -1;
        self.as_mut().edit("New Layer", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let name = pictura_render::next_layer_name(doc, "Layer");
            created = pictura_render::add_layer(doc, above, &name);
            if created >= 0 {
                rust.layer_target.select_only(&created.to_string());
                Edit::Structural
            } else {
                Edit::None
            }
        });
        created
    }

    pub fn add_group(mut self: Pin<&mut Self>, above: i32) -> i32 {
        let mut created = -1;
        self.as_mut().edit("New Group", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let name = pictura_render::next_layer_name(doc, "Group");
            created = pictura_render::add_group(doc, above, &name);
            if created >= 0 {
                rust.layer_target.select_only(&created.to_string());
                Edit::Structural
            } else {
                Edit::None
            }
        });
        created
    }

    pub fn duplicate_layer(mut self: Pin<&mut Self>, index: i32) -> i32 {
        let add_copy = self.rust().add_copy;
        let mut created = -1;
        self.as_mut().edit("Duplicate Layer", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::duplicate_layer(doc, index);
            if created >= 0 && !add_copy {
                doc.layers[created as usize].name = doc.layers[index as usize].name.clone();
            }
            if created >= 0 {
                rust.layer_target.select_only(&created.to_string());
                Edit::Structural
            } else {
                Edit::None
            }
        });
        created
    }

    pub fn group_layer(mut self: Pin<&mut Self>, index: i32) -> i32 {
        let mut created = -1;
        self.as_mut().edit("Group Layers", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::group_layer(doc, index);
            if created >= 0 {
                rust.layer_target.select_only(&created.to_string());
                Edit::Structural
            } else {
                Edit::None
            }
        });
        created
    }

    pub fn ungroup_layer(mut self: Pin<&mut Self>, index: i32) -> bool {
        let edit = self.as_mut().edit("Ungroup Layers", |rust| {
            let ungrouped = {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                pictura_render::ungroup_layer(doc, index)
            };
            if !ungrouped {
                return Edit::None;
            }
            let len = rust.session.doc.as_ref().map_or(0, |doc| doc.layers.len());
            if len > 0 {
                let index = index.clamp(0, len as i32 - 1) as usize;
                rust.select_only_layer(&index.to_string());
            } else {
                rust.layer_target.clear();
            }
            Edit::Structural
        });
        !matches!(edit, Edit::None)
    }

    pub fn layer_thumbnail(&self, i: i32, size: i32) -> QImage {
        if size <= 0 {
            return QImage::default();
        }
        let Some(layer) = self.layer(i) else {
            return QImage::default();
        };
        if layer.is_group || layer.adjustment.is_some() {
            return QImage::default();
        }
        layer_thumbnail_image(layer, size as u32).unwrap_or_default()
    }

    /// One crossing for the whole layer tree: every row's typed fields, its
    /// active effect names `\n`-joined for the adapter to split, and
    /// thumbnails at `thumb_size` (0 skips them). `entire_document` places
    /// pixel thumbnails at their document extent; shape rows always use the
    /// fill-coverage thumbnail.
    pub fn layers_snapshot(
        &self,
        thumb_size: i32,
        entire_document: bool,
    ) -> Vec<qobject::LayerRow> {
        let Some(doc) = self.rust().session.doc.as_ref() else {
            return Vec::new();
        };
        let thumb = if thumb_size > 0 { thumb_size as u32 } else { 0 };
        let effects = pictura_render::layer_style_effect_names();
        let link_sets = &self.rust().link_sets;
        pictura_render::flatten_rows(doc)
            .into_iter()
            .map(
                |(path, depth)| match pictura_render::resolve_path(doc, &path) {
                    Some(layer) => {
                        let shape = pictura_render::is_shape_layer(layer);
                        let style_effects = if pictura_render::has_layer_style(layer) {
                            let active: Vec<&str> = effects
                                .iter()
                                .copied()
                                .filter(|name| {
                                    pictura_render::layer_style_value(
                                        layer,
                                        &format!("{name}.exists"),
                                    ) == Some(1.0)
                                })
                                .collect();
                            QString::from(active.join("\n").as_str())
                        } else {
                            QString::default()
                        };
                        let thumbnail = if thumb == 0 {
                            QImage::default()
                        } else if shape {
                            super::shapes::shape_thumbnail(layer, doc, thumb_size)
                        } else if layer.is_group || layer.adjustment.is_some() {
                            QImage::default()
                        } else if entire_document {
                            layer_thumbnail_positioned(layer, doc.width, doc.height, thumb)
                                .unwrap_or_default()
                        } else {
                            layer_thumbnail_image(layer, thumb).unwrap_or_default()
                        };
                        let mask_thumbnail = if thumb == 0 {
                            QImage::default()
                        } else {
                            layer
                                .mask
                                .as_ref()
                                .and_then(|mask| mask_thumbnail_image(mask, thumb))
                                .unwrap_or_default()
                        };
                        let vector_mask_thumbnail = if thumb == 0 {
                            QImage::default()
                        } else {
                            layer
                                .vector_mask
                                .as_ref()
                                .and_then(|mask| {
                                    vector_mask_thumbnail_image(mask, doc.width, doc.height, thumb)
                                })
                                .unwrap_or_default()
                        };
                        let mut lock = layer.lock.bits();
                        if pictura_render::has_forced_locks(layer) {
                            lock |= pictura_core::LockFlags::TRANSPARENCY
                                | pictura_core::LockFlags::PIXELS;
                        }
                        qobject::LayerRow {
                            path: QString::from(path.as_str()),
                            depth: depth as i32,
                            name: QString::from(layer.name.as_str()),
                            kind: layer_kind_str(doc, &path, layer),
                            visible: layer.visible,
                            blend: QString::from(blend_key(layer.blend).as_str()),
                            opacity: layer.opacity as i32,
                            fill: layer.fill as i32,
                            lock: lock as i32,
                            color: layer.color.to_byte() as i32,
                            clipping: layer.clipping,
                            has_mask: layer.mask.is_some(),
                            mask_linked: pictura_render::layer_mask_linked(doc, &path),
                            mask_disabled: pictura_render::layer_mask_disabled(doc, &path),
                            has_vector_mask: pictura_render::has_vector_mask(doc, &path),
                            vector_mask_linked: pictura_render::vector_mask_linked(doc, &path),
                            vector_mask_disabled: pictura_render::vector_mask_disabled(doc, &path),
                            has_adjustment: layer.adjustment.is_some(),
                            expandable: layer.is_group,
                            child_count: layer.children.len() as i32,
                            linked: link_sets.contains_key(&path),
                            placed: is_placed_smart_object(layer),
                            shape,
                            smart_object: layer.smart_object.is_some(),
                            has_style: pictura_render::has_layer_style(layer),
                            style_effects,
                            has_blend_if: super::layer_style::layer_is_blend_if_customised(layer),
                            thumbnail,
                            mask_thumbnail,
                            vector_mask_thumbnail,
                        }
                    }
                    // Unreachable: every flattened row resolves in the same tree.
                    None => qobject::LayerRow {
                        path: QString::from(path.as_str()),
                        depth: depth as i32,
                        name: QString::default(),
                        kind: QString::default(),
                        visible: false,
                        blend: QString::default(),
                        opacity: 0,
                        fill: 0,
                        lock: 0,
                        color: 0,
                        clipping: false,
                        has_mask: false,
                        mask_linked: false,
                        mask_disabled: false,
                        has_vector_mask: false,
                        vector_mask_linked: false,
                        vector_mask_disabled: false,
                        has_adjustment: false,
                        expandable: false,
                        child_count: 0,
                        linked: false,
                        placed: false,
                        shape: false,
                        smart_object: false,
                        has_style: false,
                        style_effects: QString::default(),
                        has_blend_if: false,
                        thumbnail: QImage::default(),
                        mask_thumbnail: QImage::default(),
                        vector_mask_thumbnail: QImage::default(),
                    },
                },
            )
            .collect()
    }

    /// `Prefix N` where N is one more than the highest existing `Prefix <number>`
    /// name in the whole tree, falling back to 1.
    pub fn next_layer_name(&self, prefix: &QString) -> QString {
        self.rust()
            .session
            .doc
            .as_ref()
            .map_or(QString::default(), |doc| {
                QString::from(pictura_render::next_layer_name(doc, &prefix.to_string()).as_str())
            })
    }

    pub fn set_layer_name_path(mut self: Pin<&mut Self>, path: &QString, name: &QString) -> bool {
        let edit = self.as_mut().edit("Rename Layer", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            if pictura_render::rename_path(doc, &path.to_string(), &name.to_string()) {
                Edit::Meta
            } else {
                Edit::None
            }
        });
        !matches!(edit, Edit::None)
    }

    pub fn move_layer_path(mut self: Pin<&mut Self>, path: &QString, delta: i32) -> bool {
        let path = path.to_string();
        let edit = self.as_mut().edit("Move Layer", |rust| {
            let moved = {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                pictura_render::move_path(doc, &path, delta)
            };
            if !moved {
                return Edit::None;
            }
            if let Some(doc) = rust.session.doc.as_ref() {
                let parent = pictura_render::parent_path(&path).unwrap_or("");
                if let (Some(index), Some(len)) = (
                    path.rsplit('/')
                        .next()
                        .and_then(|segment| segment.parse::<usize>().ok()),
                    container_len(doc, parent),
                ) {
                    if len > 0 {
                        let index = (index as i32 + delta).clamp(0, len as i32 - 1) as usize;
                        rust.select_only_layer(&join_path(parent, index));
                    }
                }
            }
            Edit::Structural
        });
        !matches!(edit, Edit::None)
    }

    pub fn move_layer_to(
        mut self: Pin<&mut Self>,
        path: &QString,
        target: &QString,
        mode: i32,
    ) -> bool {
        let path = path.to_string();
        let target = target.to_string();
        let edit = self.as_mut().edit("Move Layer", |rust| {
            let moved = {
                let Some(doc) = rust.session.doc.as_mut() else {
                    return Edit::None;
                };
                pictura_render::move_path_to_result(doc, &path, &target, mode)
            };
            let Some(moved) = moved else {
                return Edit::None;
            };
            rust.select_only_layer(&moved);
            Edit::Structural
        });
        !matches!(edit, Edit::None)
    }

    pub fn can_move_layer_to(&self, path: &QString, target: &QString, mode: i32) -> bool {
        let path = path.to_string();
        let target = target.to_string();
        match self.rust().session.doc.as_ref() {
            Some(doc) => pictura_render::can_move_path_to(doc, &path, &target, mode),
            None => false,
        }
    }

    pub fn set_layers_visible(mut self: Pin<&mut Self>, paths: &QStringList, visible: bool) -> i32 {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let mut changed = 0;
        self.as_mut().edit("Set Visibility", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let (count, region) = set_visible_paths_union(doc, &refs, visible);
            changed = count;
            if count == 0 {
                return Edit::None;
            }
            match region {
                Some(rect) => Edit::Regions {
                    rects: vec![rect],
                    notify: false,
                },
                None => Edit::Full,
            }
        });
        if changed == 0 {
            paint_timing::report();
        }
        changed as i32
    }

    pub fn set_layers_blend(mut self: Pin<&mut Self>, paths: &QStringList, key: &QString) -> i32 {
        let key = key.to_string();
        let bytes = key.as_bytes();
        if bytes.len() != 4 {
            return 0;
        }
        let Some(mode) = BlendMode::from_psd_key([bytes[0], bytes[1], bytes[2], bytes[3]]) else {
            return 0;
        };
        let label = format!("Blending Change: {}", mode.name());
        self.as_mut()
            .batch_property(paths, &label, true, |doc, paths| {
                pictura_render::set_blend_paths(doc, paths, mode)
            })
    }

    pub fn set_layers_opacity(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        self.as_mut()
            .batch_property(paths, "Opacity", true, |doc, paths| {
                pictura_render::set_opacity_paths(doc, paths, value)
            })
    }

    pub fn set_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        self.as_mut()
            .batch_property(paths, "Fill Opacity", true, |doc, paths| {
                pictura_render::set_fill_paths(doc, paths, value)
            })
    }

    // ponytail: the preview flag is sticky, so a drag that returns to its start
    // value records one redundant state; store per-path originals if that matters.
    pub fn preview_layers_opacity(
        mut self: Pin<&mut Self>,
        paths: &QStringList,
        value: i32,
    ) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let (changed, region) = match self.as_mut().rust_mut().session.doc.as_mut() {
            Some(doc) => {
                let before: Vec<_> = refs
                    .iter()
                    .map(|path| property_compositing(doc, path))
                    .collect();
                let changed = pictura_render::set_opacity_paths(doc, &refs, value);
                let region = if changed > 0 {
                    changed_property_region(doc, &refs, &before)
                } else {
                    Some(None)
                };
                (changed, region)
            }
            None => (0, None),
        };
        if changed > 0 {
            match region {
                Some(Some(rect)) => self.as_mut().refresh_region(rect),
                _ => self.as_mut().recomposite(),
            }
            let mut rust = self.as_mut().rust_mut();
            rust.session.dirty = true;
            rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
            rust.pending_previews.opacity_changed = true;
        }
        changed as i32
    }

    pub fn commit_layers_opacity(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let pending = self.rust().pending_previews.opacity_changed;
        self.as_mut().rust_mut().pending_previews.opacity_changed = false;
        let mut changed = 0;
        self.as_mut().edit("Opacity", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let before: Vec<_> = refs
                .iter()
                .map(|path| property_compositing(doc, path))
                .collect();
            changed = pictura_render::set_opacity_paths(doc, &refs, value);
            if changed == 0 {
                return if pending { Edit::Record } else { Edit::None };
            }
            match changed_property_region(doc, &refs, &before) {
                Some(Some(rect)) => Edit::Regions {
                    rects: vec![rect],
                    notify: true,
                },
                Some(None) => Edit::Record,
                None => Edit::Full,
            }
        });
        changed as i32
    }

    pub fn preview_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let (changed, region) = match self.as_mut().rust_mut().session.doc.as_mut() {
            Some(doc) => {
                let before: Vec<_> = refs
                    .iter()
                    .map(|path| property_compositing(doc, path))
                    .collect();
                let changed = pictura_render::set_fill_paths(doc, &refs, value);
                let region = if changed > 0 {
                    changed_property_region(doc, &refs, &before)
                } else {
                    Some(None)
                };
                (changed, region)
            }
            None => (0, None),
        };
        if changed > 0 {
            match region {
                Some(Some(rect)) => self.as_mut().refresh_region(rect),
                _ => self.as_mut().recomposite(),
            }
            let mut rust = self.as_mut().rust_mut();
            rust.session.dirty = true;
            rust.session.content_revision = rust.session.content_revision.wrapping_add(1);
            rust.pending_previews.fill_changed = true;
        }
        changed as i32
    }

    pub fn commit_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let pending = self.rust().pending_previews.fill_changed;
        self.as_mut().rust_mut().pending_previews.fill_changed = false;
        let mut changed = 0;
        self.as_mut().edit("Fill Opacity", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let before: Vec<_> = refs
                .iter()
                .map(|path| property_compositing(doc, path))
                .collect();
            changed = pictura_render::set_fill_paths(doc, &refs, value);
            if changed == 0 {
                return if pending { Edit::Record } else { Edit::None };
            }
            match changed_property_region(doc, &refs, &before) {
                Some(Some(rect)) => Edit::Regions {
                    rects: vec![rect],
                    notify: true,
                },
                Some(None) => Edit::Record,
                None => Edit::Full,
            }
        });
        changed as i32
    }

    pub fn set_layers_lock(
        mut self: Pin<&mut Self>,
        paths: &QStringList,
        flag: &QString,
        on: bool,
    ) -> i32 {
        let Some(bit) = lock_bit(flag.to_string().as_str()) else {
            return 0;
        };
        self.as_mut()
            .batch_property(paths, "Lock", false, |doc, paths| {
                pictura_render::set_lock_paths(doc, paths, bit, on)
            })
    }

    pub fn set_layers_color(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        if !(0..=7).contains(&value) {
            return 0;
        }
        let color = ColorLabel::from_byte(value as u8);
        self.as_mut()
            .batch_property(paths, "Layer Color", false, |doc, paths| {
                pictura_render::set_color_paths(doc, paths, color)
            })
    }

    pub fn apply_visibility(mut self: Pin<&mut Self>, paths: &QStringList, label: &QString) -> i32 {
        let label = label.to_string();
        self.as_mut().batch_changed(paths, &label, |doc, paths| {
            pictura_render::apply_visibility(doc, paths)
        })
    }

    pub fn duplicate_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> QStringList {
        let add_copy = self.rust().add_copy;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let mut created: Vec<String> = Vec::new();
        self.as_mut().edit("Duplicate Layer", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::duplicate_paths(doc, &refs);
            if !add_copy {
                for path in &created {
                    if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
                        if let Some(stripped) = layer.name.strip_suffix(" copy") {
                            layer.name = stripped.to_string();
                        }
                    }
                }
            }
            if created.is_empty() {
                return Edit::None;
            }
            let anchor = created.last().cloned();
            let target = {
                let mut target = LayerTarget::default();
                target.select(created.clone(), anchor, rust.session.doc.as_ref());
                target
            };
            rust.set_layer_target(target);
            Edit::Structural
        });
        created.into_iter().map(QString::from).collect()
    }

    pub fn add_layer_in(mut self: Pin<&mut Self>, selection_path: &QString) -> QString {
        let selection_path = selection_path.to_string();
        let mut created = String::new();
        self.as_mut().edit("New Layer", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::add_layer_in(doc, &selection_path, "");
            if created.is_empty() {
                return Edit::None;
            }
            rust.select_only_layer(&created);
            Edit::Structural
        });
        QString::from(created.as_str())
    }

    pub fn add_group_in(mut self: Pin<&mut Self>, selection_path: &QString) -> QString {
        let selection_path = selection_path.to_string();
        let mut created = String::new();
        self.as_mut().edit("New Group", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            created = pictura_render::add_group_in(doc, &selection_path, "");
            if created.is_empty() {
                return Edit::None;
            }
            rust.select_only_layer(&created);
            Edit::Structural
        });
        QString::from(created.as_str())
    }

    fn layer(&self, i: i32) -> Option<&Layer> {
        self.rust().session.doc.as_ref()?.layers.get(i as usize)
    }

    /// Shared single-layer mutation wrapper: resolve layer `i`, apply `f` (with
    /// whether it is the Background), then refresh the layer's bounded influence
    /// region and record `label` iff `f` reports a change. A layer whose effect
    /// cannot be bounded (a group, or an unmasked adjustment) falls back to a full
    /// recomposite. A missing document or out-of-range index records nothing and
    /// returns false.
    fn mutate_layer(
        mut self: Pin<&mut Self>,
        i: i32,
        label: &str,
        f: impl FnOnce(&mut Layer, bool) -> bool,
    ) -> bool {
        paint_timing::start(label);
        let edit = self.as_mut().edit(label, |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let background = is_background_layer(doc, i);
            let Some(layer) = doc.layers.get_mut(i as usize) else {
                return Edit::None;
            };
            if !f(layer, background) {
                return Edit::None;
            }
            match layer_visibility_region(layer) {
                Some(rect) => Edit::Regions {
                    rects: vec![rect],
                    notify: false,
                },
                None => Edit::Full,
            }
        });
        paint_timing::report();
        !matches!(edit, Edit::None)
    }

    /// As [`Self::mutate_layer`] for a property that changes no pixel: record
    /// and notify without repainting.
    fn mutate_meta(
        mut self: Pin<&mut Self>,
        i: i32,
        label: &str,
        f: impl FnOnce(&mut Layer, bool) -> bool,
    ) -> bool {
        let edit = self.as_mut().edit(label, |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let background = is_background_layer(doc, i);
            let Some(layer) = doc.layers.get_mut(i as usize) else {
                return Edit::None;
            };
            if f(layer, background) {
                Edit::Meta
            } else {
                Edit::None
            }
        });
        !matches!(edit, Edit::None)
    }

    /// Shared batch mutation wrapper: apply `op` to every listed path, then
    /// recomposite and record `label` iff the change count is non-zero. A
    /// zero-change batch records nothing and emits no `changed` (M34/M39).
    pub(super) fn batch_changed(
        mut self: Pin<&mut Self>,
        paths: &QStringList,
        label: &str,
        op: impl FnOnce(&mut Document, &[&str]) -> usize,
    ) -> i32 {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let mut changed = 0;
        self.as_mut().edit(label, |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            changed = op(doc, &refs);
            if changed > 0 {
                Edit::Full
            } else {
                Edit::None
            }
        });
        changed as i32
    }

    /// As [`Self::batch_changed`] for a per-layer property: repaint only the
    /// changed layers' bounds, or nothing when `repaint` is false (a lock or a
    /// colour label changes no pixel). A layer whose reach is not its bounds (a
    /// group, a layer with effects) falls back to the full recomposite.
    fn batch_property(
        mut self: Pin<&mut Self>,
        paths: &QStringList,
        label: &str,
        repaint: bool,
        op: impl FnOnce(&mut Document, &[&str]) -> usize,
    ) -> i32 {
        paint_timing::start(label);
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let mut changed = 0;
        self.as_mut().edit(label, |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let before: Vec<_> = refs
                .iter()
                .map(|path| property_compositing(doc, path))
                .collect();
            changed = op(doc, &refs);
            if changed == 0 {
                return Edit::None;
            }
            if !repaint {
                return Edit::Meta;
            }
            match changed_property_region(doc, &refs, &before) {
                Some(Some(rect)) => Edit::Regions {
                    rects: vec![rect],
                    notify: true,
                },
                Some(None) => Edit::Meta,
                None => Edit::Full,
            }
        });
        paint_timing::report();
        changed as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pictura_core::{BlendMode, Document};

    fn compositing(doc: &Document, path: &str) -> Option<(BlendMode, u8, u8)> {
        pictura_render::resolve_path(doc, path)
            .map(|layer| (layer.blend, layer.opacity, layer.fill))
    }

    #[test]
    fn changed_property_region_bounds_a_pixel_layer_and_rejects_a_group() {
        let base = vec![80u8; 64 * 64 * 4];
        let mut doc = Document::from_rgba("base", 64, 64, &base);
        let top: Vec<u8> = (0..32 * 32 * 4).map(|i| (i % 200) as u8).collect();
        let path = pictura_render::add_raster_layer_from_rgba(&mut doc, "top", 32, 32, &top);
        assert!(!path.is_empty());
        let target = doc.layers.last().unwrap().rect;
        let before = vec![compositing(&doc, &path)];
        doc.layers.last_mut().unwrap().opacity = 128;
        assert_eq!(
            changed_property_region(&doc, &[path.as_str()], &before),
            Some(Some(target)),
            "a pixel layer is bounded by its rect"
        );

        let group = pictura_render::add_group_in(&mut doc, "", "group");
        assert!(!group.is_empty());
        let before = vec![compositing(&doc, &group)];
        doc.layers.last_mut().unwrap().opacity = 128;
        assert_eq!(
            changed_property_region(&doc, &[group.as_str()], &before),
            None,
            "a group is unbounded"
        );
    }
}
