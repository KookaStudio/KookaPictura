use super::helpers::*;
use super::helpers_composite::*;
use super::qobject;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QString, QStringList};
use pictura_core::{BlendMode, ColorLabel, Document, Layer};

mod layer_masks;
mod layers_surface;
mod matting;
mod smart_object_actions;
mod vector_masks;

impl qobject::PictureView {
    pub fn layer_count(&self) -> i32 {
        self.rust()
            .doc
            .as_ref()
            .map_or(0, |d| d.layers.len() as i32)
    }

    pub fn topmost_pixel_layer_index(&self) -> i32 {
        self.rust()
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
        let Some(doc) = self.rust().doc.as_ref() else {
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
        let region = {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            let Some(doc) = rust.doc.as_mut() else {
                return;
            };
            match doc.layers.get_mut(i as usize) {
                Some(layer) => {
                    layer.visible = visible;
                    layer_visibility_region(layer)
                }
                None => return,
            }
        };
        match region {
            Some(rect) => self.as_mut().refresh_region(rect),
            None => self.as_mut().recomposite(),
        }
        self.as_mut().record("Layer Visibility");
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
        self.as_mut().mutate_layer(i, "Blend Mode", |layer, _| {
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
        let changed = if let Some(doc) = self.as_mut().rust_mut().doc.as_mut() {
            match doc.layers.get_mut(i as usize) {
                Some(layer) => {
                    layer.name = name;
                    true
                }
                None => false,
            }
        } else {
            false
        };
        if changed {
            self.as_mut().recomposite();
            self.as_mut().record("Rename Layer");
        }
        changed
    }

    pub fn move_layer(mut self: Pin<&mut Self>, i: i32, delta: i32) -> bool {
        let changed = if let Some(doc) = self.as_mut().rust_mut().doc.as_mut() {
            let len = doc.layers.len() as i32;
            let target = i + delta;
            if i < 0 || i >= len || delta == 0 || target < 0 || target >= len {
                false
            } else {
                doc.layers.swap(i as usize, target as usize);
                true
            }
        } else {
            false
        };
        if changed {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Reorder Layer");
        }
        changed
    }

    pub fn add_layer(mut self: Pin<&mut Self>, above: i32) -> i32 {
        let created = {
            let mut rust = self.as_mut().rust_mut();
            match rust.doc.as_mut() {
                Some(doc) => {
                    let name = pictura_render::next_layer_name(doc, "Layer");
                    pictura_render::add_layer(doc, above, &name)
                }
                None => -1,
            }
        };
        if created >= 0 {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("New Layer");
        }
        created
    }

    pub fn add_group(mut self: Pin<&mut Self>, above: i32) -> i32 {
        let created = {
            let mut rust = self.as_mut().rust_mut();
            match rust.doc.as_mut() {
                Some(doc) => {
                    let name = pictura_render::next_layer_name(doc, "Group");
                    pictura_render::add_group(doc, above, &name)
                }
                None => -1,
            }
        };
        if created >= 0 {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("New Group");
        }
        created
    }

    pub fn duplicate_layer(mut self: Pin<&mut Self>, index: i32) -> i32 {
        let add_copy = self.rust().add_copy;
        let created = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => {
                let created = pictura_render::duplicate_layer(doc, index);
                if created >= 0 && !add_copy {
                    doc.layers[created as usize].name = doc.layers[index as usize].name.clone();
                }
                created
            }
            None => -1,
        };
        if created >= 0 {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Duplicate Layer");
        }
        created
    }

    pub fn group_layer(mut self: Pin<&mut Self>, index: i32) -> i32 {
        let created = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::group_layer(doc, index),
            None => -1,
        };
        if created >= 0 {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Group Layers");
        }
        created
    }

    pub fn ungroup_layer(mut self: Pin<&mut Self>, index: i32) -> bool {
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::ungroup_layer(doc, index),
            None => false,
        };
        if changed {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Ungroup Layers");
        }
        changed
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

    pub fn layer_row_count(&self) -> i32 {
        self.rust()
            .doc
            .as_ref()
            .map_or(0, |doc| pictura_render::flatten_rows(doc).len() as i32)
    }

    pub fn layer_row_path(&self, i: i32) -> QString {
        self.row_at(i)
            .map(|(_, path, _, _)| QString::from(path.as_str()))
            .unwrap_or_default()
    }

    pub fn layer_row_depth(&self, i: i32) -> i32 {
        self.row_at(i).map_or(0, |(_, _, depth, _)| depth as i32)
    }

    pub fn layer_row_name(&self, i: i32) -> QString {
        self.row_at(i)
            .map(|(_, _, _, layer)| QString::from(layer.name.as_str()))
            .unwrap_or_default()
    }

    pub fn layer_row_kind(&self, i: i32) -> QString {
        self.row_at(i)
            .map(|(doc, path, _, layer)| layer_kind_str(doc, &path, layer))
            .unwrap_or_default()
    }

    pub fn layer_row_visible(&self, i: i32) -> bool {
        self.row_at(i).is_some_and(|(_, _, _, layer)| layer.visible)
    }

    pub fn layer_row_blend(&self, i: i32) -> QString {
        self.row_at(i)
            .map(|(_, _, _, layer)| QString::from(blend_key(layer.blend).as_str()))
            .unwrap_or_default()
    }

    pub fn layer_row_opacity(&self, i: i32) -> i32 {
        self.row_at(i)
            .map_or(0, |(_, _, _, layer)| layer.opacity as i32)
    }

    pub fn layer_row_fill(&self, i: i32) -> i32 {
        self.row_at(i)
            .map_or(0, |(_, _, _, layer)| layer.fill as i32)
    }

    pub fn layer_row_lock(&self, i: i32) -> i32 {
        self.row_at(i).map_or(0, |(_, _, _, layer)| {
            let mut bits = layer.lock.bits();
            if pictura_render::has_forced_locks(layer) {
                bits |= pictura_core::LockFlags::TRANSPARENCY | pictura_core::LockFlags::PIXELS;
            }
            bits as i32
        })
    }

    pub fn layer_row_color(&self, i: i32) -> i32 {
        self.row_at(i)
            .map_or(0, |(_, _, _, layer)| layer.color.to_byte() as i32)
    }

    pub fn layer_row_clipping(&self, i: i32) -> bool {
        self.row_at(i)
            .is_some_and(|(_, _, _, layer)| layer.clipping)
    }

    pub fn layer_row_has_mask(&self, i: i32) -> bool {
        self.row_at(i)
            .is_some_and(|(_, _, _, layer)| layer.mask.is_some())
    }

    pub fn layer_row_has_adjustment(&self, i: i32) -> bool {
        self.row_at(i)
            .is_some_and(|(_, _, _, layer)| layer.adjustment.is_some())
    }

    pub fn layer_row_expandable(&self, i: i32) -> bool {
        self.row_at(i)
            .is_some_and(|(_, _, _, layer)| layer.is_group)
    }

    pub fn layer_row_child_count(&self, i: i32) -> i32 {
        self.row_at(i)
            .map_or(0, |(_, _, _, layer)| layer.children.len() as i32)
    }

    pub fn layer_row_thumbnail(&self, i: i32, size: i32, entire_document: bool) -> QImage {
        if size <= 0 {
            return QImage::default();
        }
        let Some((doc, _, _, layer)) = self.row_at(i) else {
            return QImage::default();
        };
        if layer.is_group || layer.adjustment.is_some() {
            return QImage::default();
        }
        let rendered = if entire_document {
            layer_thumbnail_positioned(layer, doc.width, doc.height, size as u32)
        } else {
            layer_thumbnail_image(layer, size as u32)
        };
        rendered.unwrap_or_default()
    }

    pub fn layer_row_mask_thumbnail(&self, i: i32, size: i32) -> QImage {
        if size <= 0 {
            return QImage::default();
        }
        self.row_at(i)
            .and_then(|(_, _, _, layer)| layer.mask.as_ref())
            .and_then(|mask| mask_thumbnail_image(mask, size as u32))
            .unwrap_or_default()
    }

    /// Whether row `i`'s path is a member of the frame's link set.
    pub fn layer_row_linked(&self, i: i32) -> bool {
        self.row_at(i)
            .is_some_and(|(_, path, _, _)| self.rust().link_sets.contains_key(&path))
    }

    /// Whether row `i` is a placed (external/alias) smart object.
    pub fn layer_row_placed(&self, i: i32) -> bool {
        self.row_at(i)
            .is_some_and(|(_, _, _, layer)| is_placed_smart_object(layer))
    }

    /// `Prefix N` where N is one more than the highest existing `Prefix <number>`
    /// name in the whole tree, falling back to 1.
    pub fn next_layer_name(&self, prefix: &QString) -> QString {
        self.rust().doc.as_ref().map_or(QString::default(), |doc| {
            QString::from(pictura_render::next_layer_name(doc, &prefix.to_string()).as_str())
        })
    }

    pub fn set_layer_name_path(mut self: Pin<&mut Self>, path: &QString, name: &QString) -> bool {
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::rename_path(doc, &path.to_string(), &name.to_string()),
            None => false,
        };
        if changed {
            self.as_mut().recomposite();
            self.as_mut().record("Rename Layer");
        }
        changed
    }

    pub fn move_layer_path(mut self: Pin<&mut Self>, path: &QString, delta: i32) -> bool {
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::move_path(doc, &path.to_string(), delta),
            None => false,
        };
        if changed {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Move Layer");
        }
        changed
    }

    pub fn move_layer_to(
        mut self: Pin<&mut Self>,
        path: &QString,
        target: &QString,
        mode: i32,
    ) -> bool {
        let path = path.to_string();
        let target = target.to_string();
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::move_path_to(doc, &path, &target, mode),
            None => false,
        };
        if changed {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Move Layer");
        }
        changed
    }

    pub fn can_move_layer_to(&self, path: &QString, target: &QString, mode: i32) -> bool {
        let path = path.to_string();
        let target = target.to_string();
        match self.rust().doc.as_ref() {
            Some(doc) => pictura_render::can_move_path_to(doc, &path, &target, mode),
            None => false,
        }
    }

    pub fn set_layers_visible(mut self: Pin<&mut Self>, paths: &QStringList, visible: bool) -> i32 {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let (changed, region) = {
            let mut rust = self.as_mut().rust_mut();
            let rust = &mut *rust;
            match rust.doc.as_mut() {
                Some(doc) => set_visible_paths_union(doc, &refs, visible),
                None => (0, None),
            }
        };
        if changed == 0 {
            paint_timing::report();
            return 0;
        }
        match region {
            Some(rect) => self.as_mut().refresh_region(rect),
            None => self.as_mut().recomposite(),
        }
        self.as_mut().record("Set Visibility");
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
        self.as_mut()
            .batch_property(paths, "Blend Mode", true, |doc, paths| {
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
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::set_opacity_paths(doc, &refs, value),
            None => 0,
        };
        if changed > 0 {
            self.as_mut().recomposite();
            let mut rust = self.as_mut().rust_mut();
            rust.content_revision = rust.content_revision.wrapping_add(1);
            rust.opacity_preview_changed = true;
        }
        changed as i32
    }

    pub fn commit_layers_opacity(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::set_opacity_paths(doc, &refs, value),
            None => 0,
        };
        let pending = self.rust().opacity_preview_changed;
        self.as_mut().rust_mut().opacity_preview_changed = false;
        if changed > 0 || pending {
            self.as_mut().recomposite();
            self.as_mut().record("Opacity");
        }
        changed as i32
    }

    pub fn preview_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::set_fill_paths(doc, &refs, value),
            None => 0,
        };
        if changed > 0 {
            self.as_mut().recomposite();
            let mut rust = self.as_mut().rust_mut();
            rust.content_revision = rust.content_revision.wrapping_add(1);
            rust.fill_preview_changed = true;
        }
        changed as i32
    }

    pub fn commit_layers_fill(mut self: Pin<&mut Self>, paths: &QStringList, value: i32) -> i32 {
        let value = value.clamp(0, 255) as u8;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::set_fill_paths(doc, &refs, value),
            None => 0,
        };
        let pending = self.rust().fill_preview_changed;
        self.as_mut().rust_mut().fill_preview_changed = false;
        if changed > 0 || pending {
            self.as_mut().recomposite();
            self.as_mut().record("Fill Opacity");
        }
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

    pub fn delete_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> i32 {
        let changed =
            self.as_mut()
                .batch_changed(paths, "Delete Layers", pictura_render::delete_paths);
        if changed > 0 {
            self.as_mut().clear_link_sets();
        }
        changed
    }

    pub fn duplicate_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> QStringList {
        let add_copy = self.rust().add_copy;
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let created = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => {
                let created = pictura_render::duplicate_paths(doc, &refs);
                if !add_copy {
                    for path in &created {
                        if let Some(layer) = pictura_render::resolve_path_mut(doc, path) {
                            if let Some(stripped) = layer.name.strip_suffix(" copy") {
                                layer.name = stripped.to_string();
                            }
                        }
                    }
                }
                created
            }
            None => Vec::new(),
        };
        if !created.is_empty() {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Duplicate Layer");
        }
        created.into_iter().map(QString::from).collect()
    }

    pub fn group_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> QString {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let created = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::group_paths(doc, &refs),
            None => None,
        };
        match created {
            Some(path) => {
                self.as_mut().clear_link_sets();
                self.as_mut().recomposite();
                self.as_mut().record("Group Layers");
                QString::from(path.as_str())
            }
            None => QString::default(),
        }
    }

    pub fn ungroup_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> i32 {
        let changed =
            self.as_mut()
                .batch_changed(paths, "Ungroup Layers", pictura_render::ungroup_paths);
        if changed > 0 {
            self.as_mut().clear_link_sets();
        }
        changed
    }

    pub fn add_layer_in(mut self: Pin<&mut Self>, selection_path: &QString) -> QString {
        let created = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::add_layer_in(doc, &selection_path.to_string(), ""),
            None => String::new(),
        };
        if !created.is_empty() {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("New Layer");
        }
        QString::from(created.as_str())
    }

    pub fn add_group_in(mut self: Pin<&mut Self>, selection_path: &QString) -> QString {
        let created = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::add_group_in(doc, &selection_path.to_string(), ""),
            None => String::new(),
        };
        if !created.is_empty() {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("New Group");
        }
        QString::from(created.as_str())
    }

    pub fn remove_layer(mut self: Pin<&mut Self>, i: i32) {
        let removed = if let Some(doc) = self.as_mut().rust_mut().doc.as_mut() {
            let idx = i as usize;
            if idx < doc.layers.len() {
                doc.layers.remove(idx);
                true
            } else {
                false
            }
        } else {
            false
        };
        if removed {
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Delete Layer");
        }
    }

    fn layer(&self, i: i32) -> Option<&Layer> {
        self.rust().doc.as_ref()?.layers.get(i as usize)
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
        let (changed, region) = if let Some(doc) = self.as_mut().rust_mut().doc.as_mut() {
            let background = is_background_layer(doc, i);
            match doc.layers.get_mut(i as usize) {
                Some(layer) => {
                    let changed = f(layer, background);
                    let region = changed.then(|| layer_visibility_region(layer)).flatten();
                    (changed, region)
                }
                None => (false, None),
            }
        } else {
            (false, None)
        };
        if changed {
            match region {
                Some(rect) => self.as_mut().refresh_region(rect),
                None => self.as_mut().recomposite(),
            }
            self.as_mut().record(label);
        }
        paint_timing::report();
        changed
    }

    /// As [`Self::mutate_layer`] for a property that changes no pixel: record
    /// and notify without repainting.
    fn mutate_meta(
        mut self: Pin<&mut Self>,
        i: i32,
        label: &str,
        f: impl FnOnce(&mut Layer, bool) -> bool,
    ) -> bool {
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => {
                let background = is_background_layer(doc, i);
                doc.layers
                    .get_mut(i as usize)
                    .is_some_and(|layer| f(layer, background))
            }
            None => false,
        };
        if changed {
            self.as_mut().record(label);
            self.as_mut().changed();
        }
        changed
    }

    /// Resolve flat row `i` to its document, path, depth, and layer, or `None`
    /// when out of range or without a document.
    ///
    /// ponytail: re-flattens the whole tree on every row getter (O(n) each, so
    /// a full panel refresh is O(n²)). Cache the projection and invalidate it on
    /// `changed` if a many-row document ever profiles hot.
    fn row_at(&self, i: i32) -> Option<(&Document, String, u32, &Layer)> {
        if i < 0 {
            return None;
        }
        let doc = self.rust().doc.as_ref()?;
        let (path, depth) = pictura_render::flatten_rows(doc)
            .into_iter()
            .nth(i as usize)?;
        let layer = pictura_render::resolve_path(doc, &path)?;
        Some((doc, path, depth, layer))
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
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => op(doc, &refs),
            None => 0,
        };
        if changed > 0 {
            self.as_mut().recomposite();
            self.as_mut().record(label);
        }
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
        let (changed, region) = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => {
                // Only a layer whose compositing inputs moved reaches the
                // repaint, so a skipped or unchanged group cannot unbound it.
                let compositing = |doc: &Document, path: &str| {
                    pictura_render::resolve_path(doc, path)
                        .map(|layer| (layer.blend, layer.opacity, layer.fill))
                };
                let before: Vec<_> = refs.iter().map(|path| compositing(doc, path)).collect();
                let changed = op(doc, &refs);
                // `None`: unbounded; `Some(None)`: no pixel to repaint.
                let region = refs
                    .iter()
                    .zip(before)
                    .filter(|(path, before)| compositing(doc, path) != *before)
                    .try_fold(None, |union: Option<pictura_core::PsdRect>, (path, _)| {
                        let rect = pictura_render::resolve_path(doc, path)
                            .and_then(layer_visibility_region)?;
                        Some(Some(union.map_or(rect, |u| union_rect(u, rect))))
                    });
                (changed, if repaint { region } else { Some(None) })
            }
            None => (0, None),
        };
        if changed == 0 {
            return 0;
        }
        match region {
            None => self.as_mut().recomposite(),
            Some(Some(rect)) => self.as_mut().refresh_region(rect),
            Some(None) => {}
        }
        self.as_mut().record(label);
        if region.is_some() {
            self.as_mut().changed();
        }
        paint_timing::report();
        changed as i32
    }
}
