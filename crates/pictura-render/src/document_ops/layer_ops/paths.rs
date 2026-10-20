use pictura_core::{Document, Layer, LayerStack};

// ---------------------------------------------------------------------------
// M39 path/tree core. See `docs/dev/m39-panel-anatomy.md` §3.1–3.3.
// ---------------------------------------------------------------------------

/// Parse a frozen path (`"0"`, `"2/1"`, …) into bottom-first child indices.
///
/// Rejects the empty string, empty segments (leading/trailing/double `/`),
/// signs, non-digits, leading zeros, and overflow.
pub(super) fn parse_path(path: &str) -> Option<Vec<usize>> {
    if path.is_empty() {
        return None;
    }
    let mut segments = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() || !segment.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if segment.len() > 1 && segment.starts_with('0') {
            return None;
        }
        segments.push(segment.parse::<usize>().ok()?);
    }
    Some(segments)
}

/// The path of `path`'s containing node: `"2/1"` → `Some("2")`, `"0"` →
/// `None` (its container is the document). Malformed paths return `None`.
pub fn parent_path(path: &str) -> Option<&str> {
    parse_path(path)?;
    path.rsplit_once('/').map(|(parent, _)| parent)
}

/// Resolve a path to a layer, or `None` for a malformed or out-of-range path.
pub fn resolve_path<'a>(doc: &'a Document, path: &str) -> Option<&'a Layer> {
    let segments = parse_path(path)?;
    let mut layers: &[Layer] = &doc.layers;
    let mut node: Option<&Layer> = None;
    for &index in &segments {
        let layer = layers.get(index)?;
        node = Some(layer);
        layers = &layer.children;
    }
    node
}

/// Mutable [`resolve_path`].
pub fn resolve_path_mut<'a>(doc: &'a mut Document, path: &str) -> Option<&'a mut Layer> {
    let segments = parse_path(path)?;
    let (container, index) = container_mut(doc, &segments)?;
    container.get_mut(index)
}

/// Flatten the whole tree depth-first, topmost-first, as `(path, depth)` rows.
///
/// Each container is walked last index → 0 because `Layer.children` is
/// bottom-first; top-level rows are depth 0.
pub fn flatten_rows(doc: &Document) -> Vec<(String, u32)> {
    fn walk(layers: &[Layer], prefix: &str, depth: u32, rows: &mut Vec<(String, u32)>) {
        for (index, layer) in layers.iter().enumerate().rev() {
            let path = if prefix.is_empty() {
                index.to_string()
            } else {
                format!("{prefix}/{index}")
            };
            rows.push((path.clone(), depth));
            if layer.is_group {
                walk(&layer.children, &path, depth + 1, rows);
            }
        }
    }
    let mut rows = Vec::new();
    walk(&doc.layers, "", 0, &mut rows);
    rows
}

/// Whether the layer at `path` carries the first-class Background flag (design
/// D5). The flag is authoritative: position and name are irrelevant.
pub fn is_background(doc: &Document, path: &str) -> bool {
    resolve_path(doc, path).is_some_and(|layer| layer.background)
}

/// Resolve the mutable `children` vector holding the node at `segments` and the
/// node's index in it. `segments` must be non-empty.
pub(super) fn container_mut<'a>(
    doc: &'a mut Document,
    segments: &[usize],
) -> Option<(&'a mut LayerStack, usize)> {
    let (last, parent) = segments.split_last()?;
    let container = container_of_mut(doc, parent)?;
    if *last >= container.len() {
        return None;
    }
    Some((container, *last))
}

/// Resolve the immutable container named by a full parent path (empty = document).
pub(super) fn container_of<'a>(doc: &'a Document, parent: &[usize]) -> Option<&'a [Layer]> {
    let mut layers: &'a [Layer] = &doc.layers;
    for &index in parent {
        layers = &layers.get(index)?.children;
    }
    Some(layers)
}

/// Resolve the mutable container named by a full parent path (empty = document).
pub(super) fn container_of_mut<'a>(
    doc: &'a mut Document,
    parent: &[usize],
) -> Option<&'a mut LayerStack> {
    let mut layers: &'a mut LayerStack = &mut doc.layers;
    for &index in parent {
        if index >= layers.len() {
            return None;
        }
        layers = &mut layers[index].children;
    }
    Some(layers)
}

pub(super) fn format_segments(segments: &[usize]) -> String {
    segments
        .iter()
        .map(|segment| segment.to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// Deduplicate a path list, preserving first-seen order.
pub(super) fn unique<'a>(paths: &[&'a str]) -> Vec<&'a str> {
    let mut seen = std::collections::HashSet::new();
    let mut kept = Vec::new();
    for &path in paths {
        if seen.insert(path) {
            kept.push(path);
        }
    }
    kept
}

/// Structural selection: dedup, keep only paths that resolve, optionally drop a
/// path whose ancestor is also selected, and sort so descendants and
/// higher-index siblings come first. That order makes child-before-parent
/// removal safe (an index is never invalidated by a later edit).
pub(super) fn selected_paths(
    doc: &Document,
    paths: &[&str],
    drop_descendants: bool,
) -> Vec<(Vec<usize>, String)> {
    let mut selected: Vec<(Vec<usize>, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &path in paths {
        let Some(segments) = parse_path(path) else {
            continue;
        };
        if !seen.insert(segments.clone()) || resolve_path(doc, path).is_none() {
            continue;
        }
        selected.push((segments, path.to_string()));
    }
    if drop_descendants {
        let snapshot: Vec<Vec<usize>> = selected.iter().map(|(s, _)| s.clone()).collect();
        selected.retain(|(segments, _)| {
            !snapshot
                .iter()
                .any(|other| other.len() < segments.len() && segments.starts_with(other))
        });
    }
    selected.sort_by(|a, b| b.0.iter().rev().cmp(a.0.iter().rev()));
    selected
}

/// Apply `update` to every distinct, resolving path for which `eligible` holds;
/// returns how many nodes actually changed.
pub(super) fn edit_paths(
    doc: &mut Document,
    paths: &[&str],
    eligible: impl Fn(&Document, &str, &Layer) -> bool,
    update: impl Fn(&mut Layer) -> bool,
) -> usize {
    let mut changed = 0;
    for path in unique(paths) {
        let Some(layer) = resolve_path(doc, path) else {
            continue;
        };
        if !eligible(doc, path, layer) {
            continue;
        }
        if let Some(layer) = resolve_path_mut(doc, path) {
            if update(layer) {
                changed += 1;
            }
        }
    }
    changed
}
