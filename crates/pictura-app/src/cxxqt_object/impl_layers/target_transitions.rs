//! Target transitions for delete and ungroup: these need the pre-deletion
//! structure to choose deterministic survivors and remap spliced children.

use super::super::helpers::{
    as_str_slice, join_path, list_of_strings, survivor_after_delete, topmost_path,
};
use super::super::qobject;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

impl qobject::PictureView {
    pub fn delete_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> i32 {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let deleted = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::delete_paths_report(doc, &refs),
            None => Vec::new(),
        };
        let changed = deleted.len();
        if changed > 0 {
            let mut rust = self.as_mut().rust_mut();
            let survivor = rust
                .doc
                .as_ref()
                .and_then(|doc| survivor_after_delete(doc, &deleted));
            match survivor {
                Some(path) => rust.select_only_layer(&path),
                None => rust.layer_target.clear(),
            }
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Delete Layers");
        }
        changed as i32
    }

    pub fn ungroup_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> i32 {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let mut groups: Vec<(String, usize, usize)> = Vec::new();
        let mut survivors: Vec<(String, usize)> = Vec::new();
        if let Some(doc) = self.rust().doc.as_ref() {
            for &path in &refs {
                let Some(layer) = pictura_render::resolve_path(doc, path) else {
                    continue;
                };
                let parent = pictura_render::parent_path(path).unwrap_or("").to_string();
                let index = path
                    .rsplit('/')
                    .next()
                    .and_then(|segment| segment.parse::<usize>().ok())
                    .unwrap_or(0);
                if layer.is_group
                    && !layer.lock.is_all()
                    && !layer.lock.contains(pictura_core::LockFlags::NESTING)
                {
                    groups.push((parent, index, layer.children.len()));
                } else {
                    survivors.push((parent, index));
                }
            }
            for path in self.rust().layer_target.selected() {
                if owned.iter().any(|owned| owned == path) {
                    continue;
                }
                let Some(layer) = pictura_render::resolve_path(doc, path) else {
                    continue;
                };
                let parent = pictura_render::parent_path(path).unwrap_or("").to_string();
                let index = path
                    .rsplit('/')
                    .next()
                    .and_then(|segment| segment.parse::<usize>().ok())
                    .unwrap_or(0);
                if !layer.is_group {
                    survivors.push((parent, index));
                }
            }
        }
        let changed = match self.as_mut().rust_mut().doc.as_mut() {
            Some(doc) => pictura_render::ungroup_paths(doc, &refs),
            None => 0,
        };
        if changed > 0 {
            let mut rust = self.as_mut().rust_mut();
            let mut target_paths: Vec<String> = Vec::new();
            let mut anchor = None;
            if let Some(doc) = rust.doc.as_ref() {
                // Each group splices `children - 1` extra nodes in below it.
                // Per parent, an original index shifts only by groups below it.
                let mut expansions: std::collections::HashMap<String, Vec<(usize, usize)>> =
                    std::collections::HashMap::new();
                for (parent, index, count) in &groups {
                    expansions
                        .entry(parent.clone())
                        .or_default()
                        .push((*index, *count));
                }
                let shifted = |parent: &str, index: usize| {
                    index
                        + expansions.get(parent).map_or(0, |items| {
                            items
                                .iter()
                                .filter(|(at, _)| *at < index)
                                .map(|(_, count)| count.saturating_sub(1))
                                .sum()
                        })
                };
                for (parent, index, count) in &groups {
                    let at = shifted(parent, *index);
                    for offset in 0..*count {
                        target_paths.push(join_path(parent, at + offset));
                    }
                }
                for (parent, index) in &survivors {
                    let path = join_path(parent, shifted(parent, *index));
                    if pictura_render::resolve_path(doc, &path).is_some() {
                        target_paths.push(path);
                    }
                }
                let group_paths: Vec<String> = groups
                    .iter()
                    .map(|(parent, index, _)| join_path(parent, *index))
                    .collect();
                anchor = topmost_path(group_paths.iter().map(String::as_str)).and_then(|path| {
                    let parent = pictura_render::parent_path(path).unwrap_or("");
                    let index = path
                        .rsplit('/')
                        .next()
                        .and_then(|segment| segment.parse::<usize>().ok())?;
                    Some(join_path(parent, shifted(parent, index)))
                });
            }
            target_paths.dedup();
            let anchor = anchor.or_else(|| target_paths.first().cloned());
            let target = {
                let mut target = LayerTarget::default();
                target.select(target_paths, anchor, rust.doc.as_ref());
                target
            };
            rust.set_layer_target(target);
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Ungroup Layers");
        }
        changed as i32
    }

    /// Delete the top-level layer at index `i` under the same rules as the
    /// panel's delete (the Background, a fully locked layer, and the
    /// document's last layer stay).
    pub fn remove_layer(mut self: Pin<&mut Self>, i: i32) -> bool {
        let path = i.to_string();
        let deleted = if i >= 0 {
            match self.as_mut().rust_mut().doc.as_mut() {
                Some(doc) => pictura_render::delete_paths_report(doc, &[&path]),
                None => Vec::new(),
            }
        } else {
            Vec::new()
        };
        let removed = !deleted.is_empty();
        if removed {
            let mut rust = self.as_mut().rust_mut();
            let survivor = rust
                .doc
                .as_ref()
                .and_then(|doc| survivor_after_delete(doc, &deleted));
            match survivor {
                Some(path) => rust.select_only_layer(&path),
                None => rust.layer_target.clear(),
            }
            self.as_mut().clear_link_sets();
            self.as_mut().recomposite();
            self.as_mut().record("Delete Layer");
        }
        removed
    }
}
