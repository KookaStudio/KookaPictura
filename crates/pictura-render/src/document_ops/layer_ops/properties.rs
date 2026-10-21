use pictura_core::{BlendMode, ColorLabel, Document, Layer, LayerStack, LockFlags};

use super::create::{empty_group, next_layer_name, release_background};
use super::merge::is_visible_in_panel;
use super::paths::{
    container_of, edit_paths, flatten_rows, format_segments, is_background, parse_path,
    resolve_path, resolve_path_mut, selected_paths, unique,
};
use super::shape_layer::has_forced_locks;

/// Set the eye on every listed path (the Background included). Returns the
/// number of nodes changed.
pub fn set_visible_paths(doc: &mut Document, paths: &[&str], visible: bool) -> usize {
    edit_paths(
        doc,
        paths,
        |_, _, _| true,
        |layer| {
            if layer.visible == visible {
                false
            } else {
                layer.visible = visible;
                true
            }
        },
    )
}

/// Solo visibility primitive: set exactly the listed paths visible and every
/// other node in the tree hidden. It is not additive, ancestors are not
/// implied (the caller passes them), an unresolvable path contributes nothing,
/// and a duplicate path counts once. Returns the number of nodes changed.
pub fn apply_visibility(doc: &mut Document, paths: &[&str]) -> usize {
    let visible: std::collections::HashSet<String> = unique(paths)
        .into_iter()
        .filter(|path| resolve_path(doc, path).is_some())
        .map(str::to_string)
        .collect();
    fn walk(
        layers: &mut LayerStack,
        prefix: &str,
        visible: &std::collections::HashSet<String>,
    ) -> usize {
        let mut changed = 0;
        for (index, layer) in layers.iter_mut().enumerate() {
            let path = if prefix.is_empty() {
                index.to_string()
            } else {
                format!("{prefix}/{index}")
            };
            let want = visible.contains(&path);
            if layer.visible != want {
                layer.visible = want;
                changed += 1;
            }
            changed += walk(&mut layer.children, &path, visible);
        }
        changed
    }
    walk(&mut doc.layers, "", &visible)
}

/// Set the blend mode. Skips the Background and fully-locked nodes; groups are
/// eligible. Returns the number of nodes changed.
pub fn set_blend_paths(doc: &mut Document, paths: &[&str], mode: BlendMode) -> usize {
    edit_paths(
        doc,
        paths,
        |doc, path, layer| !is_background(doc, path) && !layer.lock.is_all(),
        |layer| {
            if layer.blend == mode {
                false
            } else {
                layer.blend = mode;
                true
            }
        },
    )
}

/// Set opacity. Skips the Background and fully-locked nodes; groups are
/// eligible. Returns the number of nodes changed.
pub fn set_opacity_paths(doc: &mut Document, paths: &[&str], value: u8) -> usize {
    edit_paths(
        doc,
        paths,
        |doc, path, layer| !is_background(doc, path) && !layer.lock.is_all(),
        |layer| {
            if layer.opacity == value {
                false
            } else {
                layer.opacity = value;
                true
            }
        },
    )
}

/// Set fill opacity. Skips the Background, fully-locked nodes, and groups (a
/// group has no Fill). Returns the number of nodes changed.
pub fn set_fill_paths(doc: &mut Document, paths: &[&str], value: u8) -> usize {
    edit_paths(
        doc,
        paths,
        |doc, path, layer| !is_background(doc, path) && !layer.lock.is_all() && !layer.is_group,
        |layer| {
            if layer.fill == value {
                false
            } else {
                layer.fill = value;
                true
            }
        },
    )
}

/// Set (`on`) or clear a lock bit. Skips the Background; a fully-locked node is
/// still eligible (that is how a lock is released). Transparency and Image
/// cannot be cleared on a layer that forces them (a type or shape layer); any
/// other requested bit still clears. Returns the number changed.
pub fn set_lock_paths(doc: &mut Document, paths: &[&str], flag: u8, on: bool) -> usize {
    edit_paths(
        doc,
        paths,
        |doc, path, _| !is_background(doc, path),
        |layer| {
            let mut flag = flag;
            if !on && has_forced_locks(layer) {
                flag &= !(LockFlags::TRANSPARENCY | LockFlags::PIXELS);
            }
            if flag == 0 {
                return false;
            }
            let next = layer.lock.with(flag, on);
            if next == layer.lock {
                false
            } else {
                layer.lock = next;
                true
            }
        },
    )
}

/// Set the color label. Skips the Background; fully-locked nodes and groups are
/// eligible. Returns the number of nodes changed.
pub fn set_color_paths(doc: &mut Document, paths: &[&str], color: ColorLabel) -> usize {
    edit_paths(
        doc,
        paths,
        |doc, path, _| !is_background(doc, path),
        |layer| {
            if layer.color == color {
                false
            } else {
                layer.color = color;
                true
            }
        },
    )
}

/// Apply the full lock set to every descendant of the group at `path`, leaving
/// the group node's own locks untouched. Returns the number of layers changed;
/// `0` when `path` is not a group or that group has no descendants.
pub fn lock_group_layers(doc: &mut Document, path: &str) -> usize {
    if !resolve_path(doc, path).is_some_and(|layer| layer.is_group) {
        return 0;
    }
    let prefix = format!("{path}/");
    let descendants: Vec<String> = flatten_rows(doc)
        .into_iter()
        .map(|(path, _)| path)
        .filter(|candidate| candidate.starts_with(&prefix))
        .collect();
    if descendants.is_empty() {
        return 0;
    }
    let refs: Vec<&str> = descendants.iter().map(String::as_str).collect();
    set_lock_paths(doc, &refs, LockFlags::all().bits(), true)
}

pub fn delete_paths(doc: &mut Document, paths: &[&str]) -> usize {
    delete_paths_report(doc, paths).len()
}

/// [`delete_paths`] plus the pre-deletion paths it actually removed, topmost
/// first. Used to keep the layer target on a deterministic survivor.
pub fn delete_paths_report(doc: &mut Document, paths: &[&str]) -> Vec<String> {
    let selected = selected_paths(doc, paths, true);
    let mut deleted = Vec::new();
    for (segments, path) in selected {
        if !deletable(doc, &segments, &path) {
            continue;
        }
        if doc.layers.remove(&segments).is_some() {
            deleted.push(path);
        }
    }
    deleted
}

/// Whether [`delete_paths`] would delete anything, read-only: it deletes
/// nothing before its first eligible path, so this is whether any selected
/// path is eligible in the tree as it stands.
pub fn can_delete_paths(doc: &Document, paths: &[&str]) -> bool {
    selected_paths(doc, paths, true)
        .iter()
        .any(|(segments, path)| deletable(doc, segments, path))
}

/// The rule [`delete_paths`] applies to each node, against the tree as it
/// stands: not the Background, not the last top-level node, not fully locked,
/// and present.
fn deletable(doc: &Document, segments: &[usize], path: &str) -> bool {
    !is_background(doc, path)
        && !(segments.len() == 1 && doc.layers.len() <= 1)
        && resolve_path(doc, path).is_some_and(|layer| !layer.lock.is_all())
}

/// Delete every layer whose effective visibility is off (its own eye and every
/// ancestor's), counting a hidden group and its nested hidden descendants as
/// one removal. Returns the number of nodes removed; a tree with no hidden
/// layer is left untouched. *(match inferred: CS6 does not state the nested
/// count or the Background/locked exemptions inherited from [`delete_paths`].)*
pub fn delete_hidden_layers(doc: &mut Document) -> usize {
    let hidden: Vec<String> = flatten_rows(doc)
        .into_iter()
        .filter(|(path, _)| !is_visible_in_panel(doc, path))
        .map(|(path, _)| path)
        .collect();
    if hidden.is_empty() {
        return 0;
    }
    let refs: Vec<&str> = hidden.iter().map(String::as_str).collect();
    delete_paths(doc, &refs)
}

/// The paths of every layer matching the layer at `path`, in panel
/// (depth-first, topmost-first) order and excluding the reference itself.
///
/// The match key is inferred (CS6's exact "similar" attributes are unsourced):
/// the same node class (group, adjustment, background, or pixel), for
/// adjustments the same PSD adjustment key, and the same blend mode. An
/// unknown path matches nothing.
pub fn select_similar(doc: &Document, path: &str) -> Vec<String> {
    let Some(reference) = resolve_path(doc, path) else {
        return Vec::new();
    };
    let key = match_key(reference);
    flatten_rows(doc)
        .into_iter()
        .filter(|(candidate, _)| candidate != path)
        .filter(|(candidate, _)| {
            resolve_path(doc, candidate).is_some_and(|layer| match_key(layer) == key)
        })
        .map(|(candidate, _)| candidate)
        .collect()
}

/// The [`select_similar`] match key: `(class, adjustment key, blend mode)`.
fn match_key(layer: &Layer) -> (u8, [u8; 4], BlendMode) {
    let class = if layer.is_group {
        0
    } else if layer.adjustment.is_some() {
        1
    } else if layer.background {
        2
    } else {
        3
    };
    let adjustment = layer.adjustment.as_ref().map_or([0; 4], |data| data.key);
    (class, adjustment, layer.blend)
}

pub fn duplicate_paths(doc: &mut Document, paths: &[&str]) -> Vec<String> {
    let selected = selected_paths(doc, paths, true);
    let mut created = Vec::new();
    for (segments, path) in selected {
        let Some(source) = resolve_path(doc, &path) else {
            continue;
        };
        let mut copy = source.clone();
        copy.name = format!("{} copy", copy.name);
        if copy.background {
            release_background(&mut copy);
        }
        let Some((&index, parent)) = segments.split_last() else {
            continue;
        };
        if doc.layers.insert(parent, index + 1, copy).is_err() {
            continue;
        }
        let mut new_segments = segments;
        if let Some(last) = new_segments.last_mut() {
            *last = index + 1;
        }
        created.push(format_segments(&new_segments));
    }
    created
}

pub fn group_paths(doc: &mut Document, paths: &[&str]) -> Option<String> {
    let selected = selected_paths(doc, paths, true);
    let (first, _) = selected.first()?;
    let parent = first[..first.len() - 1].to_vec();
    if selected
        .iter()
        .any(|(segments, _)| segments[..segments.len() - 1] != parent[..])
    {
        return None;
    }
    for (_, path) in &selected {
        if is_background(doc, path)
            || resolve_path(doc, path)
                .is_some_and(|layer| layer.lock.is_all() || layer.lock.contains(LockFlags::NESTING))
        {
            return None;
        }
    }
    let name = next_layer_name(doc, "Group");
    let mut indices: Vec<usize> = selected
        .iter()
        .map(|(segments, _)| *segments.last().expect("non-empty"))
        .collect();
    indices.sort_unstable();
    let at = doc
        .layers
        .group(&parent, &indices, empty_group(&name))
        .ok()?;
    let mut new_segments = parent;
    new_segments.push(at);
    Some(format_segments(&new_segments))
}

pub fn ungroup_paths(doc: &mut Document, paths: &[&str]) -> usize {
    let selected = selected_paths(doc, paths, false);
    let mut changed = 0;
    for (segments, path) in selected {
        let Some(layer) = resolve_path(doc, &path) else {
            continue;
        };
        if !layer.is_group || layer.lock.is_all() || layer.lock.contains(LockFlags::NESTING) {
            continue;
        }
        let children: Vec<Layer> = layer.children.iter().cloned().collect();
        if doc.layers.replace(&segments, children).is_ok() {
            changed += 1;
        }
    }
    changed
}

/// Rename the node at `path`. Renaming a Background to another name converts
/// it to an ordinary layer (unflagged, unlocked, with an alpha channel) so the
/// new name survives a PSD round trip; `Layer from Background` remains the
/// explicit conversion command. Renaming any other layer only changes its
/// name. Returns false for a path that does not resolve.
pub fn rename_path(doc: &mut Document, path: &str, name: &str) -> bool {
    if parse_path(path).is_none() {
        return false;
    }
    match resolve_path_mut(doc, path) {
        Some(layer) => {
            if layer.background && name != "Background" {
                release_background(layer);
            }
            layer.name = name.to_string();
            true
        }
        None => false,
    }
}

/// Lowest child index a non-Background node may occupy: one above the highest
/// flagged Background in `container`, or `0` when there is none.
fn background_floor(container: &[Layer]) -> usize {
    container
        .iter()
        .rposition(|layer| layer.background)
        .map_or(0, |index| index + 1)
}

/// Dry-run form of [`move_path`]: whether the delta move would change the
/// order. Performs the same guards and never mutates the document.
pub fn can_move_path(doc: &Document, path: &str, delta: i32) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    if is_background(doc, path) || resolve_path(doc, path).is_some_and(|layer| layer.lock.is_all())
    {
        return false;
    }
    let Some((&index, parent)) = segments.split_last() else {
        return false;
    };
    let Some(container) = container_of(doc, parent) else {
        return false;
    };
    if container.is_empty() {
        return false;
    }
    let floor = background_floor(container);
    let last = container.len() - 1;
    if index < floor || floor > last {
        return false;
    }
    let target = (index as i32 + delta).clamp(floor as i32, last as i32) as usize;
    target != index
}

pub fn move_path(doc: &mut Document, path: &str, delta: i32) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    if is_background(doc, path) || resolve_path(doc, path).is_some_and(|layer| layer.lock.is_all())
    {
        return false;
    }
    let Some((&index, parent)) = segments.split_last() else {
        return false;
    };
    let Some(container) = container_of(doc, parent) else {
        return false;
    };
    if container.is_empty() {
        return false;
    }
    let floor = background_floor(container);
    let last = container.len() - 1;
    if index < floor || floor > last {
        return false;
    }
    let target = (index as i32 + delta).clamp(floor as i32, last as i32) as usize;
    if target == index {
        return false;
    }
    let insertion = if target > index { target + 1 } else { target };
    doc.layers.move_node(&segments, parent, insertion).is_ok()
}

/// Resolve where a `move_path_to` candidate would land: the destination parent
/// container and the insertion index (`None` = append). `None` means the move is
/// refused. `move_path_to` (mutation) and `can_move_path_to` (dry run) both
/// route through this, so the two can never diverge.
fn move_path_to_dest(
    doc: &Document,
    path: &str,
    target: &str,
    mode: i32,
) -> Option<(Vec<usize>, Option<usize>)> {
    let src = parse_path(path)?;
    if is_background(doc, path) {
        return None;
    }
    let layer = resolve_path(doc, path)?;
    if layer.lock.is_all() || layer.lock.contains(LockFlags::NESTING) {
        return None;
    }
    if !target.is_empty() && (target == path || target.starts_with(&format!("{path}/"))) {
        return None;
    }
    let (dest_parent, dest_index) = if target.is_empty() {
        (Vec::new(), None)
    } else {
        let tgt = parse_path(target)?;
        if mode == 2 {
            if !resolve_path(doc, target).is_some_and(|layer| layer.is_group) {
                return None;
            }
            (tgt, None)
        } else {
            let (last, parent) = tgt.split_last()?;
            (parent.to_vec(), Some(last + usize::from(mode == 0)))
        }
    };
    let src_parent = &src[..src.len() - 1];
    // A same-container move from a node already below the Background is
    // malformed; refuse it rather than jumping a sibling. Moving the node out
    // to another container is allowed.
    if dest_parent.as_slice() == src_parent {
        let src_container = container_of(doc, src_parent)?;
        if *src.last().expect("non-empty") < background_floor(src_container) {
            return None;
        }
    }
    // Nesting lock: a drop that changes the layer's parent is refused when
    // either the destination container or the current parent carries `NESTING`.
    // A within-container reorder keeps the same parent and stays allowed.
    if dest_parent.as_slice() != src_parent
        && (parent_nesting_locked(doc, &dest_parent) || parent_nesting_locked(doc, src_parent))
    {
        return None;
    }
    // `dest_parent` is in pre-removal coordinates, so it is validated pre-
    // removal here and converted once in `move_path_to` after the removal.
    let dest_container = container_of(doc, &dest_parent)?;
    if dest_index.is_some_and(|index| index < background_floor(dest_container)) {
        return None;
    }
    Some((dest_parent, dest_index))
}

/// Whether the container named by `parent` (empty = document root) carries the
/// `NESTING` lock. Used to refuse reparenting into or out of a nesting-locked
/// group.
fn parent_nesting_locked(doc: &Document, parent: &[usize]) -> bool {
    let mut layers: &[Layer] = &doc.layers;
    let mut node: Option<&Layer> = None;
    for &index in parent {
        let Some(layer) = layers.get(index) else {
            return false;
        };
        node = Some(layer);
        layers = &layer.children;
    }
    node.is_some_and(|layer| layer.lock.contains(LockFlags::NESTING))
}

/// Dry-run form of [`move_path_to`]: whether the move would be accepted. Performs
/// the same guards and never mutates the document.
pub fn can_move_path_to(doc: &Document, path: &str, target: &str, mode: i32) -> bool {
    move_path_to_dest(doc, path, target, mode).is_some()
}

pub fn move_path_to(doc: &mut Document, path: &str, target: &str, mode: i32) -> bool {
    move_path_to_result(doc, path, target, mode).is_some()
}

/// [`move_path_to`] plus the moved node's post-operation path.
pub fn move_path_to_result(
    doc: &mut Document,
    path: &str,
    target: &str,
    mode: i32,
) -> Option<String> {
    let src = parse_path(path)?;
    let (dest_parent, dest_index) = move_path_to_dest(doc, path, target, mode)?;
    doc.layers
        .move_node(&src, &dest_parent, dest_index.unwrap_or(usize::MAX))
        .ok()?;
    let len = container_of(doc, &dest_parent)?.len();
    let at = match dest_index {
        None => len.checked_sub(1)?,
        Some(mut index) => {
            if dest_parent.as_slice() == &src[..src.len() - 1] && index > *src.last()? {
                index -= 1;
            }
            index.min(len.checked_sub(1)?)
        }
    };
    let mut segments = dest_parent;
    segments.push(at);
    Some(format_segments(&segments))
}

/// Where [`arrange_path`] moves the node within its own container. `Front` is
/// the top of the stack (the highest child index; `doc.layers` is bottom-first).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrange {
    Front,
    Forward,
    Backward,
    Back,
}

/// The child index [`arrange_path`] would move `path` to, or `None` when the
/// path does not resolve, the node carries no reorder (Background/locked), the
/// node already sits below a Background, or the container has no position above
/// its Background.
fn arrange_target(doc: &Document, path: &str, arrange: Arrange) -> Option<usize> {
    let segments = parse_path(path)?;
    if is_background(doc, path) || resolve_path(doc, path).is_some_and(|layer| layer.lock.is_all())
    {
        return None;
    }
    let (last, parent) = segments.split_last()?;
    let container = container_of(doc, parent)?;
    if *last >= container.len() {
        return None;
    }
    let index = *last;
    let front = container.len() - 1;
    let floor = background_floor(container);
    if index < floor || floor > front {
        return None;
    }
    let target = match arrange {
        Arrange::Front => front,
        Arrange::Forward => (index + 1).min(front),
        Arrange::Backward => index.saturating_sub(1),
        Arrange::Back => floor,
    };
    Some(target.clamp(floor, front))
}

/// Move the node at `path` to [`Arrange`]'s position within its own container.
/// Refuses the Background, fully-locked nodes, a boundary no-op, and a path
/// that does not resolve. Returns whether the order changed.
pub fn arrange_path(doc: &mut Document, path: &str, arrange: Arrange) -> bool {
    arrange_path_index(doc, path, arrange).is_some()
}

/// [`arrange_path`] plus the position the node landed at.
pub fn arrange_path_index(doc: &mut Document, path: &str, arrange: Arrange) -> Option<usize> {
    let target = arrange_target(doc, path, arrange)?;
    let segments = parse_path(path)?;
    let index = *segments.last()? as i32;
    move_path(doc, path, target as i32 - index).then_some(target)
}

/// Dry-run form of [`arrange_path`]: whether the move would change the order.
pub fn can_arrange_path(doc: &Document, path: &str, arrange: Arrange) -> bool {
    let Some(segments) = parse_path(path) else {
        return false;
    };
    let Some(target) = arrange_target(doc, path, arrange) else {
        return false;
    };
    segments.last().copied() != Some(target)
}

pub fn reverse_paths(doc: &mut Document, paths: &[&str]) -> bool {
    reverse_run(doc, paths).is_some()
}

/// [`reverse_paths`] plus every selected node's post-operation path.
pub fn reverse_paths_result(doc: &mut Document, paths: &[&str]) -> Option<Vec<String>> {
    let selected = selected_paths(doc, paths, true);
    let (parent, start, end) = reverse_run(doc, paths)?;
    Some(
        selected
            .iter()
            .map(|(segments, _)| {
                let index = *segments.last().expect("selected_paths keeps non-empty");
                let mut mapped = parent.clone();
                mapped.push(start + end - index);
                format_segments(&mapped)
            })
            .collect(),
    )
}

fn reverse_run(doc: &mut Document, paths: &[&str]) -> Option<(Vec<usize>, usize, usize)> {
    let selected = selected_paths(doc, paths, true);
    if selected.len() < 2 {
        return None;
    }
    let parent: Vec<usize> = selected[0]
        .0
        .split_last()
        .map(|(_, parent)| parent.to_vec())?;
    if selected.iter().any(|(segments, _)| {
        segments.len() != parent.len() + 1 || segments[..segments.len() - 1] != parent[..]
    }) {
        return None;
    }
    for (_, path) in &selected {
        if is_background(doc, path)
            || resolve_path(doc, path).is_some_and(|layer| layer.lock.is_all())
        {
            return None;
        }
    }
    let mut indices: Vec<usize> = selected
        .iter()
        .map(|(segments, _)| *segments.last().expect("non-empty"))
        .collect();
    indices.sort_unstable();
    if indices.windows(2).any(|window| window[1] != window[0] + 1) {
        return None;
    }
    let (start, end) = (indices[0], indices[indices.len() - 1]);
    doc.layers
        .reverse(&parent, start, end + 1)
        .then_some((parent, start, end))
}
