//! Select Similar / Select Linked, link sets, and Delete Hidden / Hide Layers.
//!
//! Link sets are transient session state (design D9): a `HashMap<path, set id>`
//! that is never serialized and that every structural bridge command clears via
//! [`PictureView::clear_link_sets`]. Select/Hide are view/visibility operations;
//! Delete Hidden is the only one that records history.

use std::collections::HashMap;

use super::edit::Edit;
use super::helpers::*;
use super::qobject;
use crate::layer_target::LayerTarget;
use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

impl qobject::PictureView {
    /// Select the layer and every layer matching it: same node class,
    /// adjustment kind, and blend mode. Sets the document target (the layer the
    /// command ran on stays active) and notifies the view. No history and no
    /// recomposite.
    pub fn select_similar(mut self: Pin<&mut Self>, path: &QString) -> QStringList {
        let path = path.to_string();
        let (matched, target) = {
            let rust = self.rust();
            let Some(doc) = rust.session.doc.as_ref() else {
                return QStringList::default();
            };
            if pictura_render::resolve_path(doc, &path).is_none() {
                return QStringList::default();
            }
            // The engine matcher excludes the reference; the target adds it
            // back so the command's own layer stays selected and active.
            let matched = pictura_render::select_similar(doc, &path);
            let mut selected = matched.clone();
            if !selected.contains(&path) {
                selected.insert(0, path.clone());
            }
            let mut target = LayerTarget::default();
            target.select(selected, Some(path.clone()), Some(doc));
            (matched, target)
        };
        {
            let rust = &mut *self.as_mut().rust_mut();
            rust.set_layer_target(target.clone());
            rust.session.history.set_current_target(&target);
        }
        self.as_mut().changed();
        matched.into_iter().map(QString::from).collect()
    }

    /// Every member of the link set containing `path`, in panel order; empty
    /// when the layer is unlinked. A linked layer's set becomes the target
    /// with `path` active; an unlinked layer leaves the target unchanged. No
    /// history and no recomposite.
    pub fn select_linked(mut self: Pin<&mut Self>, path: &QString) -> QStringList {
        let path = path.to_string();
        let (members, target) = {
            let rust = self.rust();
            let Some(&id) = rust.link_sets.get(&path) else {
                return QStringList::default();
            };
            let Some(doc) = rust.session.doc.as_ref() else {
                return QStringList::default();
            };
            if pictura_render::resolve_path(doc, &path).is_none() {
                return QStringList::default();
            }
            let members: Vec<String> = pictura_render::flatten_rows(doc)
                .into_iter()
                .filter(|(candidate, _)| rust.link_sets.get(candidate) == Some(&id))
                .map(|(candidate, _)| candidate)
                .collect();
            let mut target = LayerTarget::default();
            target.select(members.clone(), Some(path.clone()), Some(doc));
            (members, target)
        };
        {
            let rust = &mut *self.as_mut().rust_mut();
            rust.set_layer_target(target.clone());
            rust.session.history.set_current_target(&target);
        }
        self.as_mut().changed();
        members.into_iter().map(QString::from).collect()
    }

    /// Link (`on`) or unlink the listed layers. Returns the number of listed
    /// paths whose membership changed. Linking joins every set the selection
    /// touches into one; unlinking drops a set once its last member leaves.
    pub fn link_layers(mut self: Pin<&mut Self>, paths: &QStringList, on: bool) -> i32 {
        let owned = list_of_strings(paths);
        let mut rust = self.as_mut().rust_mut();
        apply_link(&mut rust.link_sets, &owned, on)
    }

    /// Hide the listed layers, recording one "Hide Layers" state when any
    /// changed. Returns the number of layers changed.
    pub fn hide_layers(mut self: Pin<&mut Self>, paths: &QStringList) -> i32 {
        self.as_mut()
            .batch_changed(paths, "Hide Layers", |doc, paths| {
                pictura_render::set_visible_paths(doc, paths, false)
            })
    }

    /// Delete every effectively hidden layer, recording one "Delete Hidden
    /// Layers" state only when at least one is removed. Returns the number
    /// removed.
    pub fn delete_hidden_layers(mut self: Pin<&mut Self>) -> i32 {
        let mut removed = 0;
        self.as_mut().edit("Delete Hidden Layers", |rust| {
            let Some(doc) = rust.session.doc.as_mut() else {
                return Edit::None;
            };
            removed = pictura_render::delete_hidden_layers(doc);
            if removed == 0 {
                return Edit::None;
            }
            let target = {
                let mut target = rust.layer_target.clone();
                if let Some(doc) = rust.session.doc.as_ref() {
                    target.retain_resolving(doc);
                }
                target
            };
            rust.set_layer_target(target);
            if rust.layer_target.is_empty() {
                select_top_layer(rust);
            }
            Edit::Structural
        });
        removed as i32
    }

    /// Drop the transient link map. Structural commands call this: paths are
    /// positional, so any tree edit invalidates them (design D9).
    pub(super) fn clear_link_sets(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().link_sets.clear();
    }
}

/// The next unused set id (ids only need to be unique within the session).
fn next_link_id(link_sets: &HashMap<String, u32>) -> u32 {
    link_sets.values().copied().max().unwrap_or(0) + 1
}

/// Link/unlink `paths` in place and report how many of them changed membership.
///
/// Linking picks the first existing set in the selection as the survivor and
/// folds every other set it touches into it (including members that are not in
/// `paths`); with no existing set it starts a fresh one. Unlinking just removes
/// entries, so an emptied set disappears with its last member.
fn apply_link(link_sets: &mut HashMap<String, u32>, paths: &[String], on: bool) -> i32 {
    if on {
        let existing: Vec<u32> = paths
            .iter()
            .filter_map(|path| link_sets.get(path).copied())
            .collect();
        let target = existing
            .first()
            .copied()
            .unwrap_or_else(|| next_link_id(link_sets));
        if existing.len() > 1 {
            for value in link_sets.values_mut() {
                if existing.contains(value) {
                    *value = target;
                }
            }
        }
        let original: Vec<Option<u32>> = paths
            .iter()
            .map(|path| link_sets.get(path).copied())
            .collect();
        for path in paths {
            link_sets.entry(path.clone()).or_insert(target);
        }
        paths
            .iter()
            .zip(original)
            .filter(|(path, before)| link_sets.get(*path).copied() != *before)
            .count() as i32
    } else {
        let mut changed = 0;
        for path in paths {
            if link_sets.remove(path).is_some() {
                changed += 1;
            }
        }
        changed
    }
}
