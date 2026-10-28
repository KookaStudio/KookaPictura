//! The tracked planes of a document, their tile deltas, and the private copy
//! the history keeps of the state at its cursor.

use pictura_core::{fresh_stamp, Document, Layer, Plane, PsdRect};
use rayon::prelude::*;

/// Tile edge for the region deltas: a changed plane keeps only the 64×64 tiles
/// whose bytes differ, so a small edit costs a small delta rather than a whole
/// 61 MiB plane.
pub(super) const TILE: usize = 64;

/// One changed tile of one plane, laid out row-major with the plane's stride.
/// It holds one side of the change, whichever side the materialized state it
/// is walked from does not: the bytes before it while the state it leads to is
/// at or below the cursor, the bytes after it otherwise. A walk that moves a
/// materialized state across it swaps the two (see [`swap_tiles`]), so history
/// keeps each changed byte once.
pub(super) struct TileDelta {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) w: usize,
    pub(super) h: usize,
    pub(super) bytes: Vec<u8>,
}

/// The changed tiles of one tracked plane, anchored at the previous state.
pub(super) struct PlaneDelta {
    pub(super) index: usize,
    pub(super) width: usize,
    pub(super) tiles: Vec<TileDelta>,
}

/// What a tracked plane is, which decides what its change does to the picture.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PlaneKind {
    /// The document composite: its tiles are the picture's change.
    Composite,
    /// A layer's color or alpha channel: shown only through the composite.
    LayerChannel,
    /// A document channel or a layer mask, which a channel or mask view shows
    /// directly.
    Other,
}

/// Where a plane's tile grid sits in document coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PlanePlacement {
    /// The composite: rows are channel blocks stacked vertically.
    Composite,
    /// A document channel: origin 0, one plane.
    Canvas,
    /// A layer channel or raster mask: its rect's top-left is the origin.
    Rect(PsdRect),
    /// A degenerate plane whose geometry does not map cleanly; a hint must not
    /// skip it.
    Unmapped,
}

/// The kind of every tracked plane, in traversal order.
pub(super) fn plane_kinds(doc: &Document) -> Vec<PlaneKind> {
    fn visit(layer: &Layer, out: &mut Vec<PlaneKind>) {
        out.extend(layer.channels.iter().map(|_| PlaneKind::LayerChannel));
        if layer.mask.as_ref().is_some_and(|m| m.data.is_some()) {
            out.push(PlaneKind::Other);
        }
        for child in &layer.children {
            visit(child, out);
        }
    }
    let mut out = vec![PlaneKind::Composite];
    out.extend(doc.channels.iter().map(|_| PlaneKind::Other));
    for l in &doc.layers {
        visit(l, &mut out);
    }
    out
}

/// Bring `current` to `next` and return the changed tiles, each holding
/// `current`'s bytes from before; `None`, with `current` untouched, when the
/// tracked-plane geometry (count, stride, or height) differs or a plane moved
/// to another index (a layer reorder), and a full state is needed.
///
/// A pair with the same nonzero stamp, or one allocation, is equal and skipped
/// without a look at its bytes; every other pair is compared tile by tile on
/// every core, and only the changed tiles are copied in.
pub(super) fn diff_into(
    current: &mut Document,
    next: &Document,
    hint: Option<&[PsdRect]>,
) -> Option<Vec<PlaneDelta>> {
    let hint = hint.filter(|hint| !hint.is_empty());
    let prev_planes = planes(current);
    let next_planes = planes(next);
    if prev_planes.len() != next_planes.len()
        || prev_planes
            .iter()
            .zip(&next_planes)
            .any(|(a, b)| a.0.len() != b.0.len() || a.1 != b.1 || a.2 != b.2)
    {
        return None;
    }
    if planes_moved(&prev_planes, &next_planes) {
        return None;
    }
    let doc_height = current.height as usize;
    drop(prev_planes);
    let mut out = Vec::new();
    let mut index = 0;
    each_plane_mut(current, &mut |plane, width, _, placement| {
        let (after, _, _, _) = &next_planes[index];
        if !known_equal(plane, after) {
            let bands = changed_tiles(
                plane.as_slice(),
                after.as_slice(),
                width,
                placement,
                doc_height,
                hint,
            );
            if bands.iter().any(|band| !band.is_empty()) {
                write_bands(plane.as_mut_slice(), after.as_slice(), width, &bands);
                out.push(PlaneDelta {
                    index,
                    width,
                    tiles: bands.into_iter().flatten().collect(),
                });
            }
        }
        index += 1;
    });
    Some(out)
}

/// Whether a stamped plane of `next` sits at a different index than the
/// `prev` plane carrying its stamp. Stamps travel with a plane and are unique
/// per plane, so this catches a reorder that keeps every count and size.
fn planes_moved(
    prev: &[(Plane<u8>, usize, usize, PlanePlacement)],
    next: &[(Plane<u8>, usize, usize, PlanePlacement)],
) -> bool {
    let index: std::collections::HashMap<u64, usize> = prev
        .iter()
        .enumerate()
        .filter(|(_, p)| p.0.stamp() != 0)
        .map(|(i, p)| (p.0.stamp(), i))
        .collect();
    next.iter()
        .enumerate()
        .any(|(i, p)| index.get(&p.0.stamp()).is_some_and(|&j| j != i))
}

fn known_equal(a: &Plane<u8>, b: &Plane<u8>) -> bool {
    (a.stamp() != 0 && a.stamp() == b.stamp()) || (a.len() == b.len() && a.shares(b))
}

/// The tiles of `before` that differ from `after`, per 64-row band, each
/// holding `before`'s bytes.
fn changed_tiles(
    before: &[u8],
    after: &[u8],
    width: usize,
    placement: PlanePlacement,
    doc_height: usize,
    hint: Option<&[PsdRect]>,
) -> Vec<Vec<TileDelta>> {
    if width == 0 {
        return Vec::new();
    }
    let height = before.len() / width;
    let columns = width.div_ceil(TILE);
    (0..height.div_ceil(TILE))
        .into_par_iter()
        .map(|band| {
            let y0 = band * TILE;
            let h = TILE.min(height - y0);
            let hinted: Vec<bool> = (0..columns)
                .map(|c| {
                    let x0 = c * TILE;
                    let w = TILE.min(width - x0);
                    tile_hits(hint, placement, doc_height, x0, y0, w, h)
                })
                .collect();
            if hinted.iter().all(|hit| !hit) {
                return Vec::new();
            }
            let mut changed = vec![false; columns];
            for row in y0..y0 + h {
                let start = row * width;
                let (b, a) = (&before[start..start + width], &after[start..start + width]);
                if b == a {
                    continue;
                }
                for (c, flag) in changed.iter_mut().enumerate() {
                    if !hinted[c] {
                        continue;
                    }
                    let x0 = c * TILE;
                    let x1 = (x0 + TILE).min(width);
                    *flag = *flag || b[x0..x1] != a[x0..x1];
                }
            }
            changed
                .iter()
                .enumerate()
                .filter(|(_, f)| **f)
                .map(|(c, _)| {
                    let x0 = c * TILE;
                    let w = TILE.min(width - x0);
                    let mut bytes = Vec::with_capacity(w * h);
                    for row in y0..y0 + h {
                        let start = row * width + x0;
                        bytes.extend_from_slice(&before[start..start + w]);
                    }
                    TileDelta {
                        x: x0,
                        y: y0,
                        w,
                        h,
                        bytes,
                    }
                })
                .collect()
        })
        .collect()
}

fn tile_hits(
    hint: Option<&[PsdRect]>,
    placement: PlanePlacement,
    doc_height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> bool {
    let Some(hint) = hint else {
        return true;
    };
    if matches!(placement, PlanePlacement::Unmapped) {
        return true;
    }
    let tile = tile_doc_rect(placement, doc_height, x, y, w, h);
    hint.iter().any(|rect| intersects(*rect, tile))
}

fn tile_doc_rect(
    placement: PlanePlacement,
    doc_height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> PsdRect {
    let (left, origin_top) = match placement {
        PlanePlacement::Composite | PlanePlacement::Canvas | PlanePlacement::Unmapped => {
            (x as i32, y as i32)
        }
        PlanePlacement::Rect(rect) => (
            rect.left.saturating_add(x as i32),
            rect.top.saturating_add(y as i32),
        ),
    };
    let (top, bottom) = if matches!(placement, PlanePlacement::Composite) && doc_height > 0 {
        let first = y / doc_height;
        let last = (y + h.saturating_sub(1)) / doc_height;
        if first == last {
            let top = y % doc_height;
            (top as i32, (top + h) as i32)
        } else {
            (0, doc_height as i32)
        }
    } else {
        (origin_top, origin_top.saturating_add(h as i32))
    };
    PsdRect {
        top,
        left,
        bottom,
        right: left.saturating_add(w as i32),
    }
}

fn intersects(a: PsdRect, b: PsdRect) -> bool {
    a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom
}

/// Copy `after`'s bytes into `plane` over each band's changed tiles, band by
/// band on every core.
fn write_bands(plane: &mut [u8], after: &[u8], width: usize, bands: &[Vec<TileDelta>]) {
    plane
        .par_chunks_mut(TILE * width)
        .zip(after.par_chunks(TILE * width))
        .zip(bands.par_iter())
        .for_each(|((dst, src), tiles)| {
            for tile in tiles {
                for row in 0..tile.h {
                    let at = row * width + tile.x;
                    dst[at..at + tile.w].copy_from_slice(&src[at..at + tile.w]);
                }
            }
        });
}

/// Write each tile's stored bytes into `doc` (and into `live` when given): a
/// walk that leaves the deltas as they are.
pub(super) fn apply_tiles(doc: &mut Document, deltas: &[PlaneDelta]) {
    for delta in deltas {
        let mut index = 0;
        each_plane_mut(doc, &mut |plane, _, _, _| {
            if index == delta.index {
                for tile in &delta.tiles {
                    write_tile(plane, delta.width, tile, &tile.bytes);
                }
            }
            index += 1;
        });
    }
}

/// Exchange each tile's stored bytes with `doc`'s, writing the stored bytes
/// into `live` too when given: a walk that moves a materialized state across
/// the deltas, after which they hold the side it left.
pub(super) fn swap_tiles(
    doc: &mut Document,
    live: Option<&mut Document>,
    deltas: &mut [PlaneDelta],
) {
    if let Some(live) = live {
        apply_tiles(live, deltas);
    }
    for delta in deltas.iter_mut() {
        let mut index = 0;
        each_plane_mut(doc, &mut |plane, _, _, _| {
            if index == delta.index {
                for tile in delta.tiles.iter_mut() {
                    swap_tile(plane, delta.width, tile);
                }
            }
            index += 1;
        });
    }
}

fn write_tile(plane: &mut Plane<u8>, width: usize, tile: &TileDelta, bytes: &[u8]) {
    if width == 0 {
        return;
    }
    for row in 0..tile.h {
        let dst = (tile.y + row) * width + tile.x;
        let src = row * tile.w;
        if dst + tile.w > plane.len() {
            return;
        }
        plane[dst..dst + tile.w].copy_from_slice(&bytes[src..src + tile.w]);
    }
}

fn swap_tile(plane: &mut Plane<u8>, width: usize, tile: &mut TileDelta) {
    let last = (tile.y + tile.h).saturating_sub(1) * width + tile.x + tile.w;
    if width == 0 || tile.h == 0 || last > plane.len() {
        return;
    }
    for row in 0..tile.h {
        let dst = (tile.y + row) * width + tile.x;
        let src = row * tile.w;
        plane[dst..dst + tile.w].swap_with_slice(&mut tile.bytes[src..src + tile.w]);
    }
}

/// The document rectangle a composite tile covers.
pub(super) fn tile_rect(tile: &TileDelta) -> PsdRect {
    PsdRect {
        top: tile.y as i32,
        left: tile.x as i32,
        bottom: (tile.y + tile.h) as i32,
        right: (tile.x + tile.w) as i32,
    }
}

/// Replace `doc`'s metadata with `meta`'s, keeping `doc`'s tracked-plane bytes.
pub(super) fn adopt_metadata(doc: &mut Document, meta: &Document) {
    let mut planes: Vec<Plane<u8>> = Vec::new();
    each_plane(doc, &mut |p, _, _, _| planes.push(p.clone()));
    *doc = meta.clone();
    let mut index = 0;
    each_plane_mut(doc, &mut |p, _, _, _| {
        if let Some(src) = planes.get(index) {
            *p = src.clone();
        }
        index += 1;
    });
}

/// `doc` with every tracked plane cleared: its metadata alone.
pub(super) fn hollow(doc: &Document) -> Document {
    let mut meta = doc.clone();
    each_plane_mut(&mut meta, &mut |p, _, _, _| *p = Plane::default());
    meta
}

/// `source`'s metadata and planes, taking `donor`'s plane for each nonzero
/// stamp the two agree on and copying the rest; the result and `source` are
/// stamped equal and share no tracked plane.
pub(super) fn shared_copy(source: &mut Document, donor: &Document) -> Document {
    let mut reusable: std::collections::HashMap<u64, Plane<u8>> = std::collections::HashMap::new();
    each_plane(donor, &mut |p, _, _| {
        if p.stamp() != 0 {
            reusable.entry(p.stamp()).or_insert_with(|| p.clone());
        }
    });
    // A duplicated layer reuses one allocation; retire the stamp on later
    // reuse so `planes_moved`'s unique-stamp map stays meaningful.
    let mut reused = std::collections::HashSet::new();
    let mut copy = source.clone();
    each_plane_mut(&mut copy, &mut |p, _, _| {
        let stamp = p.stamp();
        *p = match reusable.get(&stamp) {
            Some(plane) if !plane.shares(p) => {
                let mut taken = plane.clone();
                if !reused.insert(stamp) {
                    taken.set_stamp(fresh_stamp());
                }
                taken
            }
            _ => copy_plane(p),
        };
    });
    stamp_agreed(&mut copy, source);
    copy
}

/// A copy of `source` in planes it does not share, each pair stamped equal.
pub(super) fn private_copy(source: &mut Document) -> Document {
    let mut copy = source.clone();
    let mut stamps = Vec::new();
    each_plane_mut(&mut copy, &mut |p, _, _, _| {
        let stamp = fresh_stamp();
        let mut fresh = copy_plane(p);
        fresh.set_stamp(stamp);
        *p = fresh;
        stamps.push(stamp);
    });
    let mut index = 0;
    each_plane_mut(source, &mut |p, _, _, _| {
        p.set_stamp(stamps[index]);
        index += 1;
    });
    copy
}

/// A copy of `doc` sharing no tracked plane with it, so neither side's next
/// write copies a whole plane.
pub(crate) fn detached_copy(doc: &Document) -> Document {
    let mut copy = doc.clone();
    each_plane_mut(&mut copy, &mut |p, _, _, _| *p = copy_plane(p));
    copy
}

/// A uniquely held copy of `plane`, copied on every core.
pub(crate) fn copy_plane(plane: &Plane<u8>) -> Plane<u8> {
    Plane::build(plane.len(), |dst| copy_par(dst, plane.as_slice()))
}

fn copy_par(dst: &mut [u8], src: &[u8]) {
    const CHUNK: usize = 1 << 20;
    dst.par_chunks_mut(CHUNK)
        .zip(src.par_chunks(CHUNK))
        .for_each(|(d, s)| d.copy_from_slice(s));
}

/// Whether every tracked plane of `live` is known to equal `private`'s, so
/// tiles applied to both keep them equal.
pub(super) fn planes_agree(private: &Document, live: &Document) -> bool {
    let a = planes(private);
    let b = planes(live);
    a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| known_equal(&x.0, &y.0))
}

/// Re-stamp `private` and `live`, whose tracked planes the caller has just made
/// equal byte for byte. A pair that shares one allocation is split by giving
/// `private` its own copy, so the next write to `live` happens in place.
pub(super) fn stamp_agreed(private: &mut Document, live: &mut Document) {
    let theirs = planes(live);
    let mut stamps = Vec::new();
    let mut index = 0;
    each_plane_mut(private, &mut |p, _, _, _| {
        let other = &theirs[index].0;
        index += 1;
        if p.len() == other.len() && p.shares(other) {
            *p = copy_plane(other);
        } else if p.stamp() != 0 && p.stamp() == other.stamp() {
            stamps.push(p.stamp());
            return;
        }
        let stamp = fresh_stamp();
        p.set_stamp(stamp);
        stamps.push(stamp);
    });
    drop(theirs);
    let mut index = 0;
    each_plane_mut(live, &mut |p, _, _, _| {
        p.set_stamp(stamps[index]);
        index += 1;
    });
}

fn planes(doc: &Document) -> Vec<(Plane<u8>, usize, usize, PlanePlacement)> {
    let mut out = Vec::new();
    each_plane(doc, &mut |p, w, h, placement| {
        out.push((p.clone(), w, h, placement))
    });
    out
}

fn plane_height(len: usize, width: usize) -> usize {
    len.checked_div(width).unwrap_or(0)
}

/// Visit every tracked plane (composite, document channels, layer channels,
/// layer masks, depth-first) with its row stride, height, and document
/// placement. The traversal is fixed by the document structure, not the plane
/// lengths, so it enumerates the same indices before and after a plane is
/// cleared or written.
pub(super) fn each_plane(
    doc: &Document,
    f: &mut impl FnMut(&Plane<u8>, usize, usize, PlanePlacement),
) {
    let width = doc.width as usize;
    let h = plane_height(doc.composite.data.len(), width);
    f(&doc.composite.data, width, h, PlanePlacement::Composite);
    for channel in &doc.channels {
        let h = plane_height(channel.data.len(), width);
        f(&channel.data, width, h, PlanePlacement::Canvas);
    }
    for layer in &doc.layers {
        each_layer_plane(layer, f);
    }
}

fn each_layer_plane(layer: &Layer, f: &mut impl FnMut(&Plane<u8>, usize, usize, PlanePlacement)) {
    let layer_width = layer.rect.width().max(0) as usize;
    for channel in &layer.channels {
        let width = if layer_width > 0 {
            layer_width
        } else {
            channel.data.len()
        };
        let h = plane_height(channel.data.len(), width);
        let placement = if layer_width > 0 {
            PlanePlacement::Rect(layer.rect)
        } else {
            PlanePlacement::Unmapped
        };
        f(&channel.data, width, h, placement);
    }
    if let Some(mask) = &layer.mask {
        if let Some(data) = &mask.data {
            let mask_width = mask.rect.width().max(0) as usize;
            let width = if mask_width > 0 {
                mask_width
            } else {
                data.len()
            };
            let h = plane_height(data.len(), width);
            let placement = if mask_width > 0 {
                PlanePlacement::Rect(mask.rect)
            } else {
                PlanePlacement::Unmapped
            };
            f(data, width, h, placement);
        }
    }
    for child in &layer.children {
        each_layer_plane(child, &mut *f);
    }
}

pub(super) fn each_plane_mut(
    doc: &mut Document,
    f: &mut impl FnMut(&mut Plane<u8>, usize, usize, PlanePlacement),
) {
    let width = doc.width as usize;
    let h = plane_height(doc.composite.data.len(), width);
    f(&mut doc.composite.data, width, h, PlanePlacement::Composite);
    for channel in doc.channels.iter_mut() {
        let h = plane_height(channel.data.len(), width);
        f(&mut channel.data, width, h, PlanePlacement::Canvas);
    }
    for layer in doc.layers.iter_mut() {
        each_layer_plane_mut(layer, f);
    }
}

fn each_layer_plane_mut(
    layer: &mut Layer,
    f: &mut impl FnMut(&mut Plane<u8>, usize, usize, PlanePlacement),
) {
    let layer_width = layer.rect.width().max(0) as usize;
    for channel in layer.channels.iter_mut() {
        let width = if layer_width > 0 {
            layer_width
        } else {
            channel.data.len()
        };
        let h = plane_height(channel.data.len(), width);
        let placement = if layer_width > 0 {
            PlanePlacement::Rect(layer.rect)
        } else {
            PlanePlacement::Unmapped
        };
        f(&mut channel.data, width, h, placement);
    }
    if let Some(mask) = layer.mask.as_mut() {
        if let Some(data) = mask.data.as_mut() {
            let mask_width = mask.rect.width().max(0) as usize;
            let width = if mask_width > 0 {
                mask_width
            } else {
                data.len()
            };
            let h = plane_height(data.len(), width);
            let placement = if mask_width > 0 {
                PlanePlacement::Rect(mask.rect)
            } else {
                PlanePlacement::Unmapped
            };
            f(data, width, h, placement);
        }
    }
    for child in layer.children.iter_mut() {
        each_layer_plane_mut(child, &mut *f);
    }
}
