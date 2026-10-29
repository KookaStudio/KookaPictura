//! The layer tree: one owner for structural edits and the stack invariants.
//!
//! Reads and element writes are slice-like; inserting, removing, moving,
//! splicing, grouping, reversing, truncating, and filtering go through the
//! primitives here, which restore the invariants before returning.

use std::ops::{Deref, Index, IndexMut};
use std::sync::Arc;

use crate::Layer;

/// A bottom-first layer container used for both `Document.layers` and every
/// group's `Layer.children`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayerStack(Arc<Vec<Layer>>);

/// The first well-formedness violation found in a tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackViolation {
    /// The bottom node of a container is clipped.
    ClippedBottom(Vec<usize>),
    /// A clipped node has no non-clipped sibling below it, or is clipped to a
    /// Background.
    ClippedWithoutBase(Vec<usize>),
    /// A flagged Background is not the bottom-most node of its container.
    BackgroundNotBottom(Vec<usize>),
    /// A group carries the Background flag.
    BackgroundIsGroup(Vec<usize>),
    /// A second Background exists.
    MultipleBackgrounds(Vec<usize>),
}

/// Why a structural primitive refused an edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackError {
    /// The path or destination container does not exist.
    NoSuchPath,
    /// The edit would place a node below the Background floor.
    BackgroundFloor,
    /// The document already has a Background.
    SecondBackground,
    /// A node carrying the Background flag entered through a primitive other
    /// than `promote_background`, or a group was promoted.
    IllegalBackgroundNode,
}

#[derive(Clone, Copy)]
struct Moved {
    index: usize,
    was_clipped: bool,
    same_base_below: bool,
}

impl LayerStack {
    /// Install a tree without validating, normalizing, or reordering it. Used
    /// by the codec, scratch documents, and test fixtures. A malformed vendor
    /// file therefore loads exactly as authored.
    pub fn from_layers(layers: Vec<Layer>) -> Self {
        Self(Arc::new(layers))
    }

    fn vec_mut(&mut self) -> &mut Vec<Layer> {
        Arc::make_mut(&mut self.0)
    }

    pub fn as_slice(&self) -> &[Layer] {
        self.0.as_slice()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Layer> {
        self.0.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Layer> {
        self.vec_mut().get_mut(index)
    }

    pub fn first_mut(&mut self) -> Option<&mut Layer> {
        self.vec_mut().first_mut()
    }

    pub fn last_mut(&mut self) -> Option<&mut Layer> {
        self.vec_mut().last_mut()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Layer> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Layer> {
        self.vec_mut().iter_mut()
    }

    /// The first invariant violation in the tree, or `Ok(())`.
    pub fn well_formed(&self) -> Result<(), StackViolation> {
        let mut backgrounds = 0;
        self.check(&mut Vec::new(), &mut backgrounds)
    }

    fn check(&self, path: &mut Vec<usize>, backgrounds: &mut usize) -> Result<(), StackViolation> {
        if self.0.first().is_some_and(|layer| layer.clipping) {
            let mut at = path.clone();
            at.push(0);
            return Err(StackViolation::ClippedBottom(at));
        }
        let mut nearest_non_clipped: Option<usize> = None;
        for (index, layer) in self.0.iter().enumerate() {
            path.push(index);
            if layer.background {
                *backgrounds += 1;
                if *backgrounds > 1 {
                    return Err(StackViolation::MultipleBackgrounds(path.clone()));
                }
                if layer.is_group {
                    return Err(StackViolation::BackgroundIsGroup(path.clone()));
                }
                if index != 0 {
                    return Err(StackViolation::BackgroundNotBottom(path.clone()));
                }
            }
            if layer.clipping
                && index > 0
                && nearest_non_clipped.is_none_or(|base| self.0[base].background)
            {
                return Err(StackViolation::ClippedWithoutBase(path.clone()));
            }
            if !layer.clipping {
                nearest_non_clipped = Some(index);
            }
            layer.children.check(path, backgrounds)?;
            path.pop();
        }
        Ok(())
    }

    fn resolve(&self, path: &[usize]) -> Option<&Layer> {
        let (&last, parent) = path.split_last()?;
        self.container(parent)?.get(last)
    }

    fn container(&self, parent: &[usize]) -> Option<&LayerStack> {
        let mut layers = self;
        for &index in parent {
            let layer = layers.get(index)?;
            if !layer.is_group {
                return None;
            }
            layers = &layer.children;
        }
        Some(layers)
    }

    fn container_mut(&mut self, parent: &[usize]) -> Option<&mut LayerStack> {
        let mut layers = self;
        for &index in parent {
            let layer = layers.get_mut(index)?;
            if !layer.is_group {
                return None;
            }
            layers = &mut layer.children;
        }
        Some(layers)
    }

    /// Insert `node` at `index` in the container at `parent`.
    pub fn insert(
        &mut self,
        parent: &[usize],
        index: usize,
        mut node: Layer,
    ) -> Result<(), StackError> {
        let was_well_formed = self.snapshot_valid();
        if node.background {
            return Err(StackError::IllegalBackgroundNode);
        }
        if count_backgrounds_in(&node) > 0 && self.count_backgrounds() > 0 {
            return Err(StackError::SecondBackground);
        }
        node.children.normalize_tree();
        let layers = self.container_mut(parent).ok_or(StackError::NoSuchPath)?;
        if index > layers.len() {
            return Err(StackError::NoSuchPath);
        }
        if index < background_floor(layers.0.as_slice()) {
            return Err(StackError::BackgroundFloor);
        }
        if node.clipping && !clipping_base_allowed(layers.0.as_slice(), index) {
            node.clipping = false;
        }
        layers.vec_mut().insert(index, node);
        layers.normalize(None);
        self.debug_check(was_well_formed);
        Ok(())
    }

    /// Append `node` to this container.
    pub fn push(&mut self, node: Layer) -> Result<(), StackError> {
        self.insert(&[], self.len(), node)
    }

    /// Remove and return the top node of this container.
    pub fn pop(&mut self) -> Option<Layer> {
        let index = self.len().checked_sub(1)?;
        self.remove(&[index])
    }

    /// Remove the node at `path` and return it. The root may become empty;
    /// the delete command keeps the last-layer rule.
    pub fn remove(&mut self, path: &[usize]) -> Option<Layer> {
        let was_well_formed = self.snapshot_valid();
        let (&index, parent) = path.split_last()?;
        let layers = self.container_mut(parent)?;
        if index >= layers.len() {
            return None;
        }
        let was_clipped = layers.0[index].clipping;
        release_run_above(layers.vec_mut(), index, was_clipped);
        let removed = layers.vec_mut().remove(index);
        layers.normalize(None);
        self.debug_check(was_well_formed);
        Some(removed)
    }

    /// Move a node between containers, or within one. `to_index == usize::MAX`
    /// appends. Coordinates are pre-removal. Returns the moved node's path
    /// after the move.
    pub fn move_node(
        &mut self,
        from: &[usize],
        to_parent: &[usize],
        to_index: usize,
    ) -> Result<Vec<usize>, StackError> {
        let was_well_formed = self.snapshot_valid();
        let (&src_index, src_parent) = from.split_last().ok_or(StackError::NoSuchPath)?;
        let source = self.container(src_parent).ok_or(StackError::NoSuchPath)?;
        if src_index >= source.len() {
            return Err(StackError::NoSuchPath);
        }
        if source.0[src_index].background {
            return Err(StackError::IllegalBackgroundNode);
        }
        let same_container = src_parent == to_parent;
        if same_container && src_index < background_floor(source.0.as_slice()) {
            return Err(StackError::BackgroundFloor);
        }
        if to_parent.len() >= from.len() && to_parent[..from.len()] == *from {
            return Err(StackError::NoSuchPath);
        }
        let destination = self.container(to_parent).ok_or(StackError::NoSuchPath)?;
        let destination_floor = background_floor(destination.0.as_slice());
        if to_index != usize::MAX && to_index < destination_floor {
            return Err(StackError::BackgroundFloor);
        }
        let post_parent = post_removal_parent(to_parent, src_parent, src_index);
        let was_clipped = source.0[src_index].clipping;
        let former_base = if same_container {
            base_below(source.0.as_slice(), src_index)
        } else {
            None
        };

        let removed = {
            let layers = self
                .container_mut(src_parent)
                .ok_or(StackError::NoSuchPath)?;
            release_run_above(layers.vec_mut(), src_index, was_clipped);
            let removed = layers.vec_mut().remove(src_index);
            if !same_container {
                layers.normalize(None);
            }
            removed
        };

        let mut destination_index = to_index;
        if same_container && destination_index != usize::MAX && destination_index > src_index {
            destination_index -= 1;
        }
        let destination_len = self
            .container(&post_parent)
            .ok_or(StackError::NoSuchPath)?
            .len();
        let at = if destination_index == usize::MAX {
            destination_len
        } else {
            destination_index.min(destination_len)
        };
        let layers = self
            .container_mut(&post_parent)
            .ok_or(StackError::NoSuchPath)?;
        layers.vec_mut().insert(at, removed);
        let same_base_below = former_base.is_some_and(|base| {
            let base = if at <= base { base + 1 } else { base };
            at > 0 && at - 1 == base
        });
        layers.normalize(Some(Moved {
            index: at,
            was_clipped,
            same_base_below,
        }));
        self.debug_check(was_well_formed);
        let mut moved = post_parent;
        moved.push(at);
        Ok(moved)
    }

    /// Remove several nodes and insert a replacement block at the anchor.
    /// Returns the inserted path (or the insertion point when the replacement
    /// is empty). A refusal leaves the tree unchanged.
    pub fn splice(
        &mut self,
        removals: &[Vec<usize>],
        anchor_parent: &[usize],
        anchor_index: usize,
        mut replacement: Vec<Layer>,
    ) -> Result<Vec<usize>, StackError> {
        let was_well_formed = self.snapshot_valid();
        let Some(pre_container) = self.container(anchor_parent) else {
            return Err(StackError::NoSuchPath);
        };
        if anchor_index > pre_container.len() {
            return Err(StackError::NoSuchPath);
        }
        let anchor = anchor_parent
            .iter()
            .copied()
            .chain(std::iter::once(anchor_index))
            .collect::<Vec<_>>();
        for removal in removals {
            if self.resolve(removal).is_none() {
                return Err(StackError::NoSuchPath);
            }
            if anchor_parent.len() >= removal.len() && anchor_parent[..removal.len()] == removal[..]
            {
                return Err(StackError::NoSuchPath);
            }
        }
        for (index, a) in removals.iter().enumerate() {
            for b in &removals[index + 1..] {
                let a_prefix = a.len() <= b.len() && b[..a.len()] == a[..];
                let b_prefix = b.len() <= a.len() && a[..b.len()] == b[..];
                if a_prefix || b_prefix {
                    return Err(StackError::NoSuchPath);
                }
            }
        }

        let incoming_backgrounds: usize = replacement.iter().map(count_backgrounds_in).sum();
        let removed_backgrounds: usize = removals
            .iter()
            .filter_map(|path| self.resolve(path))
            .map(count_backgrounds_in)
            .sum();
        if self.count_backgrounds() - removed_backgrounds + incoming_backgrounds > 1 {
            return Err(StackError::SecondBackground);
        }

        let post_anchor = post_removal_path(&anchor, removals);
        let at = *post_anchor.last().expect("non-empty anchor");
        let post_parent = &post_anchor[..post_anchor.len() - 1];
        if replacement.first().is_some_and(|layer| layer.background) && at != 0 {
            return Err(StackError::IllegalBackgroundNode);
        }
        let removed_here: Vec<usize> = removals
            .iter()
            .filter(|path| {
                path.len() == anchor_parent.len() + 1 && path[..path.len() - 1] == anchor_parent[..]
            })
            .map(|path| *path.last().expect("non-empty removal"))
            .collect();
        if at < projected_background_floor(pre_container, &removed_here) {
            return Err(StackError::BackgroundFloor);
        }

        let backup = self.clone();
        let mut ordered = removals.to_vec();
        ordered.sort_by(|a, b| b.iter().rev().cmp(a.iter().rev()));
        for path in &ordered {
            if self.remove(path).is_none() {
                *self = backup;
                return Err(StackError::NoSuchPath);
            }
        }

        for node in &mut replacement {
            node.children.normalize_tree();
        }
        let Some(layers) = self.container_mut(post_parent) else {
            *self = backup;
            return Err(StackError::NoSuchPath);
        };
        if at > layers.len() {
            *self = backup;
            return Err(StackError::NoSuchPath);
        }
        for (offset, node) in replacement.into_iter().enumerate() {
            layers.vec_mut().insert(at + offset, node);
        }
        layers.normalize(None);
        self.debug_check(was_well_formed);
        Ok(post_anchor)
    }

    /// Replace one node with a sequence, using [`LayerStack::splice`].
    pub fn replace(&mut self, path: &[usize], with: Vec<Layer>) -> Result<(), StackError> {
        let (&index, parent) = path.split_last().ok_or(StackError::NoSuchPath)?;
        self.splice(&[path.to_vec()], parent, index, with)
            .map(|_| ())
    }

    /// Remove the selected indices from `parent`, place them inside `group` in
    /// stack order, and insert the group at the selection's position.
    pub fn group(
        &mut self,
        parent: &[usize],
        indices: &[usize],
        mut group: Layer,
    ) -> Result<usize, StackError> {
        let was_well_formed = self.snapshot_valid();
        if group.background {
            return Err(StackError::IllegalBackgroundNode);
        }
        let mut indices = indices.to_vec();
        indices.sort_unstable();
        indices.dedup();
        if indices.is_empty() {
            return Err(StackError::NoSuchPath);
        }
        let layers = self.container_mut(parent).ok_or(StackError::NoSuchPath)?;
        if indices.last().copied().unwrap_or(0) >= layers.len() {
            return Err(StackError::NoSuchPath);
        }
        let top = *indices.last().expect("non-empty");
        let removed_below = indices.iter().filter(|&&index| index < top).count();
        let at = top - removed_below;
        let mut children = Vec::with_capacity(indices.len());
        if indices
            .iter()
            .any(|&index| layers.0.get(index).is_some_and(|layer| layer.background))
        {
            return Err(StackError::IllegalBackgroundNode);
        }
        for &index in indices.iter().rev() {
            children.push(layers.vec_mut().remove(index));
        }
        children.reverse();
        group.children = LayerStack::from_layers(children);
        group.children.normalize_tree();
        layers.vec_mut().insert(at, group);
        layers.normalize(None);
        self.debug_check(was_well_formed);
        Ok(at)
    }

    /// Reverse the half-open range `start..end` in `parent`.
    pub fn reverse(&mut self, parent: &[usize], start: usize, end: usize) -> bool {
        let was_well_formed = self.snapshot_valid();
        let Some(layers) = self.container_mut(parent) else {
            return false;
        };
        if start >= end || end > layers.len() {
            return false;
        }
        if layers.0[start..end].iter().any(|layer| layer.background) {
            return false;
        }
        layers.vec_mut()[start..end].reverse();
        layers.normalize(None);
        self.debug_check(was_well_formed);
        true
    }

    /// Truncate the container at `parent` to `len` nodes, removing the top.
    pub fn truncate(&mut self, parent: &[usize], len: usize) -> usize {
        let was_well_formed = self.snapshot_valid();
        let Some(layers) = self.container_mut(parent) else {
            return 0;
        };
        if len >= layers.len() {
            return 0;
        }
        let removed = layers.len() - len;
        layers.vec_mut().truncate(len);
        layers.normalize(None);
        self.debug_check(was_well_formed);
        removed
    }

    /// Recursively keep the nodes whose predicate is true. Removing an
    /// unclipped base releases the kept members of the run above it.
    pub fn retain_tree(
        &mut self,
        parent: &[usize],
        predicate: &mut impl FnMut(&Layer) -> bool,
    ) -> usize {
        let was_well_formed = self.snapshot_valid();
        let Some(layers) = self.container_mut(parent) else {
            return 0;
        };
        let removed = layers.retain_recursive(predicate);
        self.debug_check(was_well_formed);
        removed
    }

    fn retain_recursive(&mut self, predicate: &mut impl FnMut(&Layer) -> bool) -> usize {
        let old = Arc::unwrap_or_clone(std::mem::take(&mut self.0));
        let mut kept = Vec::with_capacity(old.len());
        let mut removed_here = 0;
        let mut removed_children = 0;
        let mut release_run = false;
        for mut layer in old {
            let was_clipped = layer.clipping;
            if !predicate(&layer) {
                if !layer.clipping {
                    release_run = true;
                }
                removed_here += 1;
                continue;
            }
            if release_run && was_clipped {
                layer.clipping = false;
            } else if !was_clipped {
                release_run = false;
            }
            removed_children += layer.children.retain_recursive(predicate);
            kept.push(layer);
        }
        self.0 = Arc::new(kept);
        if removed_here > 0 {
            self.normalize(None);
        }
        removed_here + removed_children
    }

    /// Set a node's clipping flag. Creating a clip at a container bottom or
    /// above a Background is refused; clearing always applies.
    pub fn set_clipping(&mut self, path: &[usize], clipped: bool) -> bool {
        let was_well_formed = self.snapshot_valid();
        let Some((&index, parent)) = path.split_last() else {
            return false;
        };
        let Some(layers) = self.container_mut(parent) else {
            return false;
        };
        if index >= layers.len() || layers.0[index].clipping == clipped {
            return false;
        }
        if clipped
            && (layers.0[index].background || !clipping_base_allowed(layers.0.as_slice(), index))
        {
            return false;
        }
        layers.vec_mut()[index].clipping = clipped;
        self.debug_check(was_well_formed);
        true
    }

    /// Make the node at `path` the single document Background at root index 0.
    pub fn promote_background(&mut self, path: &[usize]) -> Result<(), StackError> {
        let was_well_formed = self.snapshot_valid();
        if self.has_background() {
            return Err(StackError::SecondBackground);
        }
        let Some(layer) = self.resolve(path) else {
            return Err(StackError::NoSuchPath);
        };
        if layer.is_group {
            return Err(StackError::IllegalBackgroundNode);
        }
        let (&index, parent) = path.split_last().ok_or(StackError::NoSuchPath)?;
        let node = {
            let layers = self.container_mut(parent).ok_or(StackError::NoSuchPath)?;
            if index >= layers.len() {
                return Err(StackError::NoSuchPath);
            }
            let was_clipped = layers.0[index].clipping;
            release_run_above(layers.vec_mut(), index, was_clipped);
            let mut node = layers.vec_mut().remove(index);
            layers.normalize(None);
            node.clipping = false;
            node.background = true;
            node
        };
        self.vec_mut().insert(0, node);
        self.normalize(None);
        self.debug_check(was_well_formed);
        Ok(())
    }

    fn count_backgrounds(&self) -> usize {
        self.0
            .iter()
            .map(|layer| usize::from(layer.background) + layer.children.count_backgrounds())
            .sum()
    }

    /// Whether any node in the tree carries the Background flag.
    pub fn has_background(&self) -> bool {
        self.count_backgrounds() > 0
    }

    fn normalize(&mut self, moved: Option<Moved>) -> bool {
        let container = self.vec_mut().as_mut_slice();
        let mut changed = false;
        if container.first().is_some_and(|layer| layer.clipping) {
            container[0].clipping = false;
            changed = true;
        }
        if let Some(moved) = moved {
            if moved.index < container.len() {
                let above = container
                    .get(moved.index + 1)
                    .is_some_and(|layer| layer.clipping);
                let below = moved.index > 0 && container[moved.index - 1].clipping;
                let clipped = moved.index > 0
                    && (above || (moved.was_clipped && (below || moved.same_base_below)));
                if container[moved.index].clipping != clipped {
                    container[moved.index].clipping = clipped;
                    changed = true;
                }
            }
        }
        for background in (0..container.len()).rev() {
            if !container[background].background {
                continue;
            }
            if container[background].clipping {
                container[background].clipping = false;
                changed = true;
            }
            for layer in container.iter_mut().skip(background + 1) {
                if !layer.clipping {
                    break;
                }
                layer.clipping = false;
                changed = true;
            }
        }
        changed
    }

    fn normalize_tree(&mut self) {
        for layer in self.vec_mut() {
            layer.children.normalize_tree();
        }
        self.normalize(None);
    }

    #[cfg(debug_assertions)]
    fn snapshot_valid(&self) -> bool {
        self.well_formed().is_ok()
    }

    #[cfg(not(debug_assertions))]
    fn snapshot_valid(&self) -> bool {
        true
    }

    fn debug_check(&self, was_well_formed: bool) {
        debug_assert!(
            !was_well_formed || self.well_formed().is_ok(),
            "layer stack became invalid: {:?}",
            self.well_formed()
        );
    }
}

fn post_removal_path(path: &[usize], removals: &[Vec<usize>]) -> Vec<usize> {
    path.iter()
        .enumerate()
        .map(|(depth, &index)| {
            let below = removals
                .iter()
                .filter(|removal| {
                    removal.len() == depth + 1
                        && removal[..depth] == path[..depth]
                        && removal[depth] < index
                })
                .count();
            index - below
        })
        .collect()
}

fn projected_background_floor(container: &[Layer], removed: &[usize]) -> usize {
    let mut floor = 0;
    for (index, layer) in container.iter().enumerate() {
        if layer.background && !removed.contains(&index) {
            let below = removed.iter().filter(|&&i| i < index).count();
            floor = floor.max(index - below + 1);
        }
    }
    floor
}

fn count_backgrounds_in(layer: &Layer) -> usize {
    usize::from(layer.background) + layer.children.count_backgrounds()
}

fn post_removal_parent(parent: &[usize], src_parent: &[usize], src_index: usize) -> Vec<usize> {
    parent
        .iter()
        .enumerate()
        .map(|(depth, &index)| {
            if &parent[..depth] == src_parent && index > src_index {
                index - 1
            } else {
                index
            }
        })
        .collect()
}

fn base_below(container: &[Layer], index: usize) -> Option<usize> {
    (0..index).rev().find(|&i| !container[i].clipping)
}

fn clipping_base_allowed(container: &[Layer], index: usize) -> bool {
    base_below(container, index).is_some_and(|base| !container[base].background)
}

fn background_floor(container: &[Layer]) -> usize {
    container
        .iter()
        .rposition(|layer| layer.background)
        .map_or(0, |index| index + 1)
}

fn release_run_above(container: &mut [Layer], index: usize, was_clipped: bool) -> bool {
    if was_clipped {
        return false;
    }
    let mut changed = false;
    for layer in container.iter_mut().skip(index.saturating_add(1)) {
        if !layer.clipping {
            break;
        }
        layer.clipping = false;
        changed = true;
    }
    changed
}

impl Deref for LayerStack {
    type Target = [Layer];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl Index<usize> for LayerStack {
    type Output = Layer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for LayerStack {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.vec_mut()[index]
    }
}

impl IntoIterator for LayerStack {
    type Item = Layer;
    type IntoIter = std::vec::IntoIter<Layer>;

    fn into_iter(self) -> Self::IntoIter {
        Arc::unwrap_or_clone(self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a LayerStack {
    type Item = &'a Layer;
    type IntoIter = std::slice::Iter<'a, Layer>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut LayerStack {
    type Item = &'a mut Layer;
    type IntoIter = std::slice::IterMut<'a, Layer>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec_mut().iter_mut()
    }
}

#[cfg(test)]
mod tests;
