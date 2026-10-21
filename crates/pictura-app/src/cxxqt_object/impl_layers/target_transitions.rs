//! Target transitions for the structural commands: group, delete, and ungroup
//! need the pre-operation structure to choose deterministic survivors and
//! remap the paths the operation moved.

use super::super::helpers::{
    as_str_slice, join_path, list_of_strings, survivor_after_delete, topmost_path,
};
use super::super::qobject;
use crate::cxxqt_object::edit::Edit;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};
use pictura_core::Document;

impl qobject::PictureView {
    pub fn group_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> QString {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        // The grouped layers and the user's selection, both as pre-operation
        // paths: the group moves the layers around it.
        let (grouped, selected) = {
            let rust = self.rust();
            (
                grouped_paths(rust.session.doc.as_ref(), &refs),
                rust.layer_target.selected().to_vec(),
            )
        };
        let mut created = String::new();
        self.as_mut().edit("Group Layers", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let Some(path) = pictura_render::group_paths(doc, &refs) else {
                return Edit::None;
            };
            created = path.clone();
            let target_paths =
                group_target_paths(&grouped, &selected, &path, rust.session.doc.as_ref());
            let target = {
                let mut target = LayerTarget::default();
                target.select(target_paths, Some(path.clone()), rust.session.doc.as_ref());
                target
            };
            rust.set_layer_target(target);
            Edit::Structural
        });
        QString::from(created.as_str())
    }

    pub fn delete_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> i32 {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let mut changed = 0;
        self.as_mut().edit("Delete Layers", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let deleted = pictura_render::delete_paths_report(doc, &refs);
            changed = deleted.len();
            if changed == 0 {
                return Edit::None;
            }
            let survivor = rust
                .session
                .doc
                .as_ref()
                .and_then(|doc| survivor_after_delete(doc, &deleted));
            match survivor {
                Some(path) => rust.select_only_layer(&path),
                None => rust.layer_target.clear(),
            }
            Edit::Structural
        });
        changed as i32
    }

    pub fn ungroup_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> i32 {
        let owned = list_of_strings(paths);
        let refs = as_str_slice(&owned);
        let mut changed = 0;
        self.as_mut().edit("Ungroup Layers", |rust| {
            let mut groups: Vec<(String, usize, usize)> = Vec::new();
            let mut survivors: Vec<(String, usize)> = Vec::new();
            if let Some(doc) = rust.session.doc.as_ref() {
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
                for path in rust.layer_target.selected() {
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
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            changed = pictura_render::ungroup_paths(doc, &refs);
            if changed == 0 {
                return Edit::None;
            }
            let mut target_paths: Vec<String> = Vec::new();
            let mut anchor = None;
            if let Some(doc) = rust.session.doc.as_ref() {
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
                target.select(target_paths, anchor, rust.session.doc.as_ref());
                target
            };
            rust.set_layer_target(target);
            Edit::Structural
        });
        changed as i32
    }

    /// Delete the top-level layer at index `i` under the same rules as the
    /// panel's delete (the Background, a fully locked layer, and the
    /// document's last layer stay).
    pub fn remove_layer(mut self: Pin<&mut Self>, i: i32) -> bool {
        let path = i.to_string();
        let removed = self.as_mut().edit("Delete Layer", |rust| {
            if i < 0 {
                return Edit::None;
            }
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            let deleted = pictura_render::delete_paths_report(doc, &[&path]);
            if deleted.is_empty() {
                return Edit::None;
            }
            let survivor = rust
                .session
                .doc
                .as_ref()
                .and_then(|doc| survivor_after_delete(doc, &deleted));
            match survivor {
                Some(path) => rust.select_only_layer(&path),
                None => rust.layer_target.clear(),
            }
            Edit::Structural
        });
        matches!(removed, Edit::Structural)
    }
}

/// The paths a group operation groups: the given ones that resolve, minus the
/// descendants of another given path (the operation swallows them with their
/// parent).
fn grouped_paths(doc: Option<&Document>, paths: &[&str]) -> Vec<String> {
    let Some(doc) = doc else {
        return Vec::new();
    };
    let mut kept: Vec<String> = Vec::new();
    for path in paths {
        if pictura_render::resolve_path(doc, path).is_none() {
            continue;
        }
        if paths
            .iter()
            .any(|other| *other != *path && path.starts_with(&format!("{other}/")))
        {
            continue;
        }
        kept.push(path.to_string());
    }
    kept
}

/// The paths a group operation leaves selected: the new group plus the layers
/// the user had selected that the group did not swallow, remapped to the paths
/// the operation gave them.
fn group_target_paths(
    grouped: &[String],
    selected: &[String],
    group_path: &str,
    doc: Option<&Document>,
) -> Vec<String> {
    let mut paths = vec![group_path.to_string()];
    let Some(first) = grouped.first() else {
        return paths;
    };
    let parent = pictura_render::parent_path(first).unwrap_or("").to_string();
    let indices: Vec<usize> = grouped
        .iter()
        .filter_map(|path| path.rsplit('/').next().and_then(|s| s.parse().ok()))
        .collect();
    let group_at = group_path
        .rsplit('/')
        .next()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    for path in selected {
        if grouped.iter().any(|grouped| grouped == path) {
            continue;
        }
        // A selected descendant of a grouped layer moves into the group; a
        // sibling moves down by the grouped layers below it and up by one past
        // the group.
        let remapped = if let Some(suffix) = grouped
            .iter()
            .find_map(|grouped| path.strip_prefix(&format!("{grouped}/")))
        {
            format!("{group_path}/{suffix}")
        } else if pictura_render::parent_path(path).unwrap_or("") == parent {
            let index = path
                .rsplit('/')
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let below = indices.iter().filter(|&&at| at < index).count();
            let shifted = index - below;
            join_path(&parent, shifted + usize::from(shifted >= group_at))
        } else {
            path.clone()
        };
        if doc
            .and_then(|doc| pictura_render::resolve_path(doc, &remapped))
            .is_some()
        {
            paths.push(remapped);
        }
    }
    paths
}
