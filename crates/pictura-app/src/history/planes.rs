//! The tracked planes of a document, their tile deltas, and the shared state
//! the history keeps at its cursor.

use pictura_core::memory::{Side, TileCensus};
use pictura_core::{fresh_stamp, Document, Layer, Plane, PsdRect, SharedTile, TileRef};
use rayon::prelude::*;

/// One changed 256² tile of one plane: a reference to the tile, never a copy
/// of its samples (#1215). It holds one side of the change, whichever side the
/// materialized state it is walked from does not: the tile before it while the
/// state it leads to is at or below the cursor, the tile after it otherwise. A
/// walk that moves a materialized state across it swaps the two (see
/// [`swap_tiles`]), so history keeps each changed tile once.
pub(super) struct TileDelta {
    /// The tile's index in its plane.
    pub(super) index: usize,
    /// The box of the samples that differ between the two sides, as first
    /// column, first global row, width and height: the damage an undo or redo
    /// across this tile does to the picture.
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) w: usize,
    pub(super) h: usize,
    pub(super) tile: SharedTile<u8>,
}

/// The changed tiles of one tracked plane, anchored at the previous state.
pub(super) struct PlaneDelta {
    pub(super) index: usize,
    pub(super) tiles: Vec<TileDelta>,
}

/// A tile's allocation as `(address, bytes)`, for the census.
pub(super) fn allocation(tile: &SharedTile<u8>) -> Option<(usize, usize)> {
    tile.allocation()
}

/// Every allocation of `doc`'s tracked planes, as `(address, bytes)`.
pub(super) fn doc_allocations(doc: &Document) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    each_plane(doc, &mut |p, _, _, _| out.extend(p.allocations()));
    out
}

/// Every tile allocation `deltas` hold.
pub(super) fn delta_allocations(deltas: &[PlaneDelta]) -> Vec<(usize, usize)> {
    deltas
        .iter()
        .flat_map(|d| d.tiles.iter())
        .filter_map(|t| allocation(&t.tile))
        .collect()
}

/// What a tracked plane is, which decides what its change does to the picture.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PlaneKind {
    /// The document composite: its tiles are the picture's change.
    Composite,
    /// A layer's color or alpha channel at this document rectangle: shown
    /// through the composite, and reaching `reach` pixels past a changed tile
    /// (the layer's and its groups' effects and mask feather).
    LayerChannel { rect: PsdRect, reach: i32 },
    /// A layer mask at this document rectangle: a solo mask view shows it
    /// directly, so its tiles are damage too. `reach` as for a channel.
    Mask(PsdRect, i32),
    /// A document channel, shown only as its panel thumbnail.
    DocumentChannel,
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
    fn visit(layer: &Layer, inherited: i32, out: &mut Vec<PlaneKind>) {
        let reach = inherited
            + pictura_render::paint_reach(layer)
            + layer
                .mask
                .as_ref()
                .map_or(0, pictura_render::mask_feather_reach);
        out.extend(layer.channels.iter().map(|_| PlaneKind::LayerChannel {
            rect: layer.rect,
            reach,
        }));
        if let Some(mask) = layer.mask.as_ref().filter(|m| m.data.is_some()) {
            out.push(PlaneKind::Mask(mask.rect, reach));
        }
        for child in &layer.children {
            visit(child, reach, out);
        }
    }
    let mut out = vec![PlaneKind::Composite];
    out.extend(doc.channels.iter().map(|_| PlaneKind::DocumentChannel));
    for l in &doc.layers {
        visit(l, 0, &mut out);
    }
    out
}

/// The rectangle of every layer mask holding data, in traversal order. Mask
/// tiles only map to the document while these stay put.
pub(super) fn mask_rects(doc: &Document) -> Vec<PsdRect> {
    plane_kinds(doc)
        .into_iter()
        .filter_map(|kind| match kind {
            PlaneKind::Mask(rect, _) => Some(rect),
            _ => None,
        })
        .collect()
}

/// Bring `current` to `next` and return the changed tiles, each holding
/// `current`'s tile from before; `None`, with `current` untouched, when the
/// tracked-plane geometry (count, stride, or height) differs, a plane moved to
/// another index (a layer reorder), or a plane that cannot be tiled changed,
/// and a full state is needed.
///
/// A pair with the same nonzero stamp, or one allocation, is equal and skipped
/// without a look at its bytes; in every other pair the tiles the two do not
/// share are compared one per core. `current` then takes `next`'s tile at each
/// of them, so the two share every compared tile, and a changed one moves into
/// the delta: no sample is copied. `census` follows both moves: `current`'s
/// tiles are the live side, the delta's are kept.
pub(super) fn diff_into(
    current: &mut Document,
    next: &mut Document,
    hint: Option<&[PsdRect]>,
    census: &mut TileCensus,
) -> Option<Vec<PlaneDelta>> {
    let hint = hint.filter(|hint| !hint.is_empty());
    tile_planes(current);
    tile_planes(next);
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
    // A plane too degenerate to tile has no tiles to share: any change to one
    // is stored with a full state.
    if prev_planes
        .iter()
        .zip(&next_planes)
        .any(|(a, b)| !alike(&a.0, &b.0) && !known_equal(&a.0, &b.0) && a.0 != b.0)
    {
        return None;
    }
    let doc_height = current.height as usize;
    drop(prev_planes);
    let mut out = Vec::new();
    let mut index = 0;
    each_plane_mut(current, &mut |plane, _, _, placement| {
        let (after, _, _, _) = &next_planes[index];
        let at = index;
        index += 1;
        if known_equal(plane, after) || !alike(plane, after) {
            return;
        }
        let compared: Vec<(usize, Option<Changed>)> = unshared(plane, after)
            .into_par_iter()
            .filter(|&i| {
                plane
                    .tile_rect(i)
                    .is_some_and(|(x, y, w, h)| tile_hits(hint, placement, doc_height, x, y, w, h))
            })
            .map(|i| (i, changed_in(plane, after, i)))
            .collect();
        let mut tiles = Vec::new();
        for (i, changed) in compared {
            let Some(before) = plane.shared_tile(i) else {
                continue;
            };
            let old = allocation(&before);
            if let Some((x, y, w, h)) = changed {
                census.add_all(Side::Kept, old);
                tiles.push(TileDelta {
                    index: i,
                    x,
                    y,
                    w,
                    h,
                    tile: before,
                });
            }
            census.remove_all(Side::Live, old);
            plane.share_tile_from(i, after);
            census.add_all(
                Side::Live,
                plane.shared_tile(i).as_ref().and_then(allocation),
            );
        }
        if !tiles.is_empty() {
            out.push(PlaneDelta { index: at, tiles });
        }
    });
    Some(out)
}

/// Whether `a` and `b` are tiled alike, so their tiles pair up by index.
fn alike(a: &Plane<u8>, b: &Plane<u8>) -> bool {
    a.is_tiled() && a.geometry() == b.geometry()
}

/// The tiles of `before` that `after`, tiled alike, does not share.
fn unshared(before: &Plane<u8>, after: &Plane<u8>) -> Vec<usize> {
    (0..before.tile_count())
        .filter(|&i| !before.shares_tile(i, after))
        .collect()
}

/// A box `(x, y, w, h)` in plane coordinates.
type Changed = (usize, usize, usize, usize);

/// The box of the samples that differ in tile `i` of `a` and `b`, tiled
/// alike, in plane coordinates; `None` when the tiles hold the same samples.
fn changed_in(a: &Plane<u8>, b: &Plane<u8>, i: usize) -> Option<Changed> {
    let (ta, tb) = (a.tile_at(i)?, b.tile_at(i)?);
    let (x, y, w, h) = changed_box(&ta, &tb)?;
    Some((ta.x + x, ta.y + y, w, h))
}

/// The tile-local box of the samples that differ between two tiles at the
/// same index, read in place.
fn changed_box(a: &TileRef<'_, u8>, b: &TileRef<'_, u8>) -> Option<Changed> {
    fn span(p: &[u8], q: Option<&[u8]>, v: Option<u8>) -> Option<(usize, usize)> {
        let differs = |k: &usize| match q {
            Some(q) => p[*k] != q[*k],
            None => Some(p[*k]) != v,
        };
        let first = (0..p.len()).find(differs)?;
        let last = (0..p.len()).rfind(differs)?;
        Some((first, last))
    }
    let (mut x0, mut x1, mut y0, mut y1) = (usize::MAX, 0, usize::MAX, 0);
    for r in 0..a.h {
        let row = match (a.row(r), b.row(r)) {
            (Some(p), q) => span(p, q, b.uniform()),
            (None, Some(q)) => span(q, None, a.uniform()),
            (None, None) => (a.uniform() != b.uniform()).then(|| (0, a.w.saturating_sub(1))),
        };
        if let Some((first, last)) = row {
            x0 = x0.min(first);
            x1 = x1.max(last + 1);
            y0 = y0.min(r);
            y1 = r + 1;
        }
    }
    (x0 != usize::MAX).then(|| (x0, y0, x1 - x0, y1 - y0))
}

/// Whether tile `i` holds the same samples in `a` and `b`, tiled alike.
fn tile_eq(a: &Plane<u8>, b: &Plane<u8>, i: usize) -> bool {
    match (a.tile_at(i), b.tile_at(i)) {
        (Some(ta), Some(tb)) => sub_eq(&ta, &tb, 0, 0, ta.w, ta.h),
        _ => false,
    }
}

/// What a state from another segment changes on screen.
pub(super) struct CompositeChange {
    /// The composite's changed tiles, empty when no byte differs.
    pub(super) delta: PlaneDelta,
    /// A mask differs, in pixels, position or count.
    pub(super) masks_differ: bool,
}

/// The composite's change between the live document and a state from another
/// segment, as the tiles a walk could not report. Tiles the two still share
/// are skipped, so an edit that kept them costs only what it touched.
///
/// `None` when the composite's own geometry differs. Document channels do not
/// count: no canvas view draws one.
pub(super) fn composite_change(
    live: &mut Document,
    restored: &mut Document,
) -> Option<CompositeChange> {
    tile_planes(live);
    tile_planes(restored);
    let shown = |doc: &Document, wanted: fn(PlaneKind) -> bool| {
        planes(doc)
            .into_iter()
            .zip(plane_kinds(doc))
            .filter(|(_, kind)| wanted(*kind))
            .map(|(plane, _)| plane)
            .collect::<Vec<_>>()
    };
    let is_mask = |kind: PlaneKind| matches!(kind, PlaneKind::Mask(..));
    let (before, after) = (shown(live, is_mask), shown(restored, is_mask));
    let masks_differ = before.len() != after.len()
        || before.iter().zip(&after).any(|(a, b)| {
            a.1 != b.1
                || a.2 != b.2
                || a.0.len() != b.0.len()
                || (!known_equal(&a.0, &b.0) && a.0 != b.0)
        });
    let is_composite = |kind: PlaneKind| matches!(kind, PlaneKind::Composite);
    let (from, to) = (
        shown(live, is_composite).into_iter().next()?,
        shown(restored, is_composite).into_iter().next()?,
    );
    if from.1 != to.1 || from.2 != to.2 || from.0.len() != to.0.len() {
        return None;
    }
    let (plane, other) = (&from.0, &to.0);
    if known_equal(plane, other) {
        return Some(CompositeChange {
            delta: PlaneDelta {
                index: 0,
                tiles: Vec::new(),
            },
            masks_differ,
        });
    }
    if !alike(plane, other) {
        return None;
    }
    let tiles: Vec<TileDelta> = unshared(plane, other)
        .into_par_iter()
        .filter_map(|i| {
            let (x, y, w, h) = changed_in(plane, other, i)?;
            Some(TileDelta {
                index: i,
                x,
                y,
                w,
                h,
                tile: other.shared_tile(i)?,
            })
        })
        .collect();
    Some(CompositeChange {
        delta: PlaneDelta { index: 0, tiles },
        masks_differ,
    })
}

/// Whether `a` and `b` hold byte-identical composites. Both are tiled first,
/// so tiles the two share are skipped and the rest compare one tile per core,
/// stopping at the first that differs.
pub(super) fn composite_unchanged(a: &mut Document, b: &mut Document) -> bool {
    tile_planes(a);
    tile_planes(b);
    let (left, right) = (&a.composite, &b.composite);
    if (left.width, left.height, left.channels) != (right.width, right.height, right.channels) {
        return false;
    }
    let (left, right) = (&left.data, &right.data);
    if left.len() != right.len() {
        return false;
    }
    if known_equal(left, right) {
        return true;
    }
    if !alike(left, right) {
        return left == right;
    }
    unshared(left, right)
        .into_par_iter()
        .all(|i| tile_eq(left, right, i))
}

/// Tile every tracked plane (the composite per channel), and settle it, so a
/// tile-level scan and write can share tiles and uniform tiles hold nothing.
pub(super) fn tile_planes(doc: &mut Document) {
    let doc_height = doc.height as usize;
    each_plane_mut(doc, &mut |plane, width, height, placement| {
        let height = match placement {
            PlanePlacement::Composite => doc_height,
            _ => height,
        };
        plane.tile(width, height);
        plane.settle();
    });
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

/// Whether sub-rectangle `(x, y, w, h)` (tile-local) holds the same samples in
/// two tiles at the same index, read in place.
fn sub_eq(
    a: &TileRef<'_, u8>,
    b: &TileRef<'_, u8>,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) -> bool {
    fn span<'t>(t: &'t TileRef<'_, u8>, r: usize, x: usize, w: usize) -> Option<&'t [u8]> {
        t.row(r).map(|row| &row[x..x + w])
    }
    (y..y + h).all(|r| match (span(a, r, x, w), span(b, r, x, w)) {
        (Some(p), Some(q)) => p == q,
        (Some(p), None) => p.iter().all(|&v| Some(v) == b.uniform()),
        (None, Some(q)) => q.iter().all(|&v| Some(v) == a.uniform()),
        (None, None) => a.uniform() == b.uniform(),
    })
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

/// Put each delta's tile into `doc`, leaving the deltas as they are: a walk
/// that rebuilds a state apart from the cursor. Tiles are shared, not copied.
pub(super) fn apply_tiles(doc: &mut Document, deltas: &[PlaneDelta]) {
    for delta in deltas {
        let mut index = 0;
        each_plane_mut(doc, &mut |plane, _, _, _| {
            if index == delta.index {
                for t in &delta.tiles {
                    let mut held = t.tile.clone();
                    plane.swap_tile(t.index, &mut held);
                }
            }
            index += 1;
        });
    }
}

/// Exchange each delta's tile with `doc`'s, and let `live` share the planes
/// that changed when given: a walk that moves the materialized state at the
/// cursor across the deltas, after which they hold the side it left. Only
/// references move. `census` follows them: `doc`'s tiles are the live side,
/// the deltas' are kept.
///
/// `live` agrees with `doc` when given, so after the swap each plane a delta
/// touched is `doc`'s byte for byte: `live` shares it instead of having the
/// tiles written a second time (#1149).
pub(super) fn swap_tiles(
    doc: &mut Document,
    live: Option<&mut Document>,
    deltas: &mut [PlaneDelta],
    census: &mut TileCensus,
) {
    for delta in deltas.iter_mut() {
        let mut index = 0;
        each_plane_mut(doc, &mut |plane, _, _, _| {
            if index == delta.index {
                for t in delta.tiles.iter_mut() {
                    let held = allocation(&t.tile);
                    if plane.swap_tile(t.index, &mut t.tile) {
                        let taken = allocation(&t.tile);
                        census.add_all(Side::Live, held);
                        census.add_all(Side::Kept, taken);
                        census.remove_all(Side::Kept, held);
                        census.remove_all(Side::Live, taken);
                    }
                }
            }
            index += 1;
        });
    }
    let Some(live) = live else {
        return;
    };
    let mut touched: Vec<Option<Plane<u8>>> = Vec::new();
    let mut index = 0;
    each_plane(doc, &mut |plane, _, _, _| {
        touched.push(
            deltas
                .iter()
                .any(|delta| delta.index == index)
                .then(|| plane.clone()),
        );
        index += 1;
    });
    let mut index = 0;
    each_plane_mut(live, &mut |plane, _, _, _| {
        if let Some(Some(shared)) = touched.get(index) {
            *plane = shared.clone();
        }
        index += 1;
    });
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
/// stamp the two agree on and cloning the rest; the result and `source` are
/// stamped equal and share every unchanged band, so a later write forks only
/// the bands it touches.
pub(super) fn shared_copy(source: &mut Document, donor: &Document) -> Document {
    let mut reusable: std::collections::HashMap<u64, Plane<u8>> = std::collections::HashMap::new();
    each_plane(donor, &mut |p, _, _, _| {
        if p.stamp() != 0 {
            reusable.entry(p.stamp()).or_insert_with(|| p.clone());
        }
    });
    // A duplicated layer reuses one allocation; retire the stamp on later
    // reuse so `planes_moved`'s unique-stamp map stays meaningful.
    let mut reused = std::collections::HashSet::new();
    let mut copy = source.clone();
    each_plane_mut(&mut copy, &mut |p, _, _, _| {
        let stamp = p.stamp();
        *p = match reusable.get(&stamp) {
            Some(plane) => {
                let mut taken = plane.clone();
                if !reused.insert(stamp) {
                    taken.set_stamp(fresh_stamp());
                }
                taken
            }
            None => p.clone(),
        };
    });
    stamp_agreed(&mut copy, source);
    copy
}

/// Whether every tracked plane of `live` is known to equal `private`'s, so
/// tiles applied to both keep them equal.
pub(super) fn planes_agree(private: &Document, live: &Document) -> bool {
    let a = planes(private);
    let b = planes(live);
    a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| known_equal(&x.0, &y.0))
}

/// Stamp `private` and `live`, whose tracked planes the caller has just made
/// equal byte for byte, so each pair agrees. A pair that already agrees keeps
/// its stamp, which is what keeps an unchanged layer resident on the GPU; a
/// written plane, or a stamp a duplicated layer repeats, gets a fresh one.
/// Shared bands are already the same allocation, so nothing is copied.
pub(super) fn stamp_agreed(private: &mut Document, live: &mut Document) {
    let mut held = Vec::new();
    each_plane(live, &mut |p, _, _, _| held.push(p.stamp()));
    let mut seen = std::collections::HashSet::new();
    let mut index = 0;
    each_plane_mut(private, &mut |p, _, _, _| {
        let own = p.stamp();
        let agreed = own != 0 && held.get(index) == Some(&own) && seen.insert(own);
        let stamp = if agreed { own } else { fresh_stamp() };
        p.set_stamp(stamp);
        held[index] = stamp;
        index += 1;
    });
    let mut index = 0;
    each_plane_mut(live, &mut |p, _, _, _| {
        p.set_stamp(held[index]);
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
