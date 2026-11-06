//! The tracked planes of a document, their tile deltas, and the shared state
//! the history keeps at its cursor.

use pictura_core::{fresh_stamp, Document, Layer, Plane, PsdRect, BAND_ROWS};
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
    /// A layer mask at this document rectangle: a solo mask view shows it
    /// directly, so its tiles are damage too.
    Mask(PsdRect),
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
    fn visit(layer: &Layer, out: &mut Vec<PlaneKind>) {
        out.extend(layer.channels.iter().map(|_| PlaneKind::LayerChannel));
        if let Some(mask) = layer.mask.as_ref().filter(|m| m.data.is_some()) {
            out.push(PlaneKind::Mask(mask.rect));
        }
        for child in &layer.children {
            visit(child, out);
        }
    }
    let mut out = vec![PlaneKind::Composite];
    out.extend(doc.channels.iter().map(|_| PlaneKind::DocumentChannel));
    for l in &doc.layers {
        visit(l, &mut out);
    }
    out
}

/// The rectangle of every layer mask holding data, in traversal order. Mask
/// tiles only map to the document while these stay put.
pub(super) fn mask_rects(doc: &Document) -> Vec<PsdRect> {
    plane_kinds(doc)
        .into_iter()
        .filter_map(|kind| match kind {
            PlaneKind::Mask(rect) => Some(rect),
            _ => None,
        })
        .collect()
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
    next: &mut Document,
    hint: Option<&[PsdRect]>,
) -> Option<Vec<PlaneDelta>> {
    let hint = hint.filter(|hint| !hint.is_empty());
    fracture_planes(current);
    fracture_planes(next);
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
            let scanned: Vec<(usize, Vec<TileDelta>)> = (0..plane.band_count())
                .into_par_iter()
                .filter(|&band| !plane.shares_band(band, after))
                .map(|band| {
                    let before = plane.band(band).unwrap_or(&[]);
                    let after = after.band(band).unwrap_or(&[]);
                    let tiles = changed_band(
                        before,
                        after,
                        width,
                        placement,
                        doc_height,
                        hint,
                        band * BAND_ROWS,
                    );
                    (band, tiles)
                })
                .collect();
            if scanned.iter().any(|(_, tiles)| !tiles.is_empty()) {
                if hint.is_none() {
                    // Every differing tile was taken, so the plane becomes
                    // `after` byte for byte: share it instead of forking the
                    // bands to copy the tiles in (#1149).
                    *plane = after.clone();
                } else {
                    for (band, tiles) in &scanned {
                        if tiles.is_empty() {
                            continue;
                        }
                        let src = after.band(*band).unwrap_or(&[]);
                        if let Some(dst) = plane.write_band(*band) {
                            write_tiles_in_band(dst, src, width, tiles);
                        }
                    }
                }
                out.push(PlaneDelta {
                    index,
                    width,
                    tiles: scanned.into_iter().flat_map(|(_, tiles)| tiles).collect(),
                });
            }
        }
        index += 1;
    });
    Some(out)
}

/// What a state from another segment changes on screen.
pub(super) struct CompositeChange {
    /// The composite's tile deltas, empty when no byte differs.
    pub(super) delta: PlaneDelta,
    /// A mask differs, in pixels, position or count.
    pub(super) masks_differ: bool,
}

/// The composite's change between the live document and a state from another
/// segment, with the tile deltas a walk could not report. Bands the two still
/// share are skipped, so an edit that kept them costs only what it touched.
///
/// `None` when the composite's own geometry differs. Document channels do not
/// count: no canvas view draws one.
pub(super) fn composite_change(
    live: &mut Document,
    restored: &mut Document,
) -> Option<CompositeChange> {
    fracture_planes(live);
    fracture_planes(restored);
    let shown = |doc: &Document, wanted: fn(PlaneKind) -> bool| {
        planes(doc)
            .into_iter()
            .zip(plane_kinds(doc))
            .filter(|(_, kind)| wanted(*kind))
            .map(|(plane, _)| plane)
            .collect::<Vec<_>>()
    };
    let is_mask = |kind: PlaneKind| matches!(kind, PlaneKind::Mask(_));
    let (before, after) = (shown(live, is_mask), shown(restored, is_mask));
    let masks_differ = before.len() != after.len()
        || before.iter().zip(&after).any(|(a, b)| {
            a.1 != b.1
                || a.2 != b.2
                || a.0.len() != b.0.len()
                || (!known_equal(&a.0, &b.0) && !a.0.bands().eq(b.0.bands()))
        });
    let is_composite = |kind: PlaneKind| matches!(kind, PlaneKind::Composite);
    let (from, to) = (
        shown(live, is_composite).into_iter().next()?,
        shown(restored, is_composite).into_iter().next()?,
    );
    if from.1 != to.1 || from.2 != to.2 || from.0.len() != to.0.len() {
        return None;
    }
    let (plane, width) = (&from.0, from.1);
    let doc_height = live.height as usize;
    let tiles: Vec<TileDelta> = (0..plane.band_count())
        .into_par_iter()
        .filter(|&band| !plane.shares_band(band, &to.0))
        .flat_map_iter(|band| {
            changed_band(
                plane.band(band).unwrap_or(&[]),
                to.0.band(band).unwrap_or(&[]),
                width,
                PlanePlacement::Composite,
                doc_height,
                None,
                band * BAND_ROWS,
            )
        })
        .collect();
    Some(CompositeChange {
        delta: PlaneDelta {
            index: 0,
            width,
            tiles,
        },
        masks_differ,
    })
}

/// Whether `a` and `b` hold byte-identical composites. Both are fractured first,
/// so the scan runs one band per core, and it stops at the first differing band.
/// The fracture is layout only: no pixel is copied.
pub(super) fn composite_unchanged(a: &mut Document, b: &mut Document) -> bool {
    fracture_planes(a);
    fracture_planes(b);
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
    (0..left.band_count())
        .into_par_iter()
        .all(|band| left.shares_band(band, right) || left.band(band) == right.band(band))
}

/// Fracture every tracked plane so a band-level scan and write can share bands.
pub(super) fn fracture_planes(doc: &mut Document) {
    each_plane_mut(doc, &mut |plane, width, _, _| plane.fracture(width));
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

/// The tiles of one band of `before` that differ from `after`, each holding
/// `before`'s bytes. `y0` is the band's first row in plane coordinates.
fn changed_band(
    before: &[u8],
    after: &[u8],
    width: usize,
    placement: PlanePlacement,
    doc_height: usize,
    hint: Option<&[PsdRect]>,
    y0: usize,
) -> Vec<TileDelta> {
    if width == 0 {
        return Vec::new();
    }
    let height = before.len() / width;
    if height == 0 {
        return Vec::new();
    }
    let columns = width.div_ceil(TILE);
    let hinted: Vec<bool> = (0..columns)
        .map(|c| {
            let x0 = c * TILE;
            let w = TILE.min(width - x0);
            tile_hits(hint, placement, doc_height, x0, y0, w, height)
        })
        .collect();
    if hinted.iter().all(|hit| !hit) {
        return Vec::new();
    }
    let mut changed = vec![false; columns];
    for row in 0..height {
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
            let mut bytes = Vec::with_capacity(w * height);
            for row in 0..height {
                let start = row * width + x0;
                bytes.extend_from_slice(&before[start..start + w]);
            }
            TileDelta {
                x: x0,
                y: y0,
                w,
                h: height,
                bytes,
            }
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

/// Copy `after`'s bytes into one band's changed tiles, whose `y` is relative to
/// the band's first row.
fn write_tiles_in_band(dst: &mut [u8], src: &[u8], width: usize, tiles: &[TileDelta]) {
    for tile in tiles {
        for row in 0..tile.h {
            let at = row * width + tile.x;
            let end = at + tile.w;
            if end <= dst.len() && end <= src.len() {
                dst[at..end].copy_from_slice(&src[at..end]);
            }
        }
    }
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
///
/// `live` agrees with `doc` when given, so after the swap each plane a delta
/// touched is `doc`'s byte for byte: `live` shares it instead of having the
/// tiles written a second time (#1149).
pub(super) fn swap_tiles(
    doc: &mut Document,
    live: Option<&mut Document>,
    deltas: &mut [PlaneDelta],
) {
    for delta in deltas.iter_mut() {
        let mut index = 0;
        each_plane_mut(doc, &mut |plane, _, _, _| {
            if index == delta.index {
                swap_plane_tiles(plane, delta.width, &mut delta.tiles);
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

/// Swap one plane's tiles with it. Tiles one band high and band-aligned, as
/// [`diff_into`] makes them, swap a band per core; any other tile swaps row by
/// row.
fn swap_plane_tiles(plane: &mut Plane<u8>, width: usize, tiles: &mut [TileDelta]) {
    let banded = width > 0
        && plane.width() == Some(width)
        && tiles
            .iter()
            .all(|tile| tile.y % BAND_ROWS == 0 && tile.h <= BAND_ROWS);
    if banded {
        let bands: Vec<usize> = tiles
            .chunk_by(|a, b| a.y == b.y)
            .map(|group| group[0].y / BAND_ROWS)
            .collect();
        if let Some(slices) = plane.write_bands(&bands) {
            let groups: Vec<&mut [TileDelta]> = tiles.chunk_by_mut(|a, b| a.y == b.y).collect();
            slices
                .into_par_iter()
                .zip(groups)
                .for_each(|(band, group)| {
                    for tile in group {
                        swap_in_band(band, width, tile);
                    }
                });
            return;
        }
    }
    for tile in tiles {
        swap_tile(plane, width, tile);
    }
}

/// Swap a tile with its rows in `band`, the slice of the band it lies in.
fn swap_in_band(band: &mut [u8], width: usize, tile: &mut TileDelta) {
    for row in 0..tile.h {
        let at = row * width + tile.x;
        let src = row * tile.w;
        if at + tile.w > band.len() || src + tile.w > tile.bytes.len() {
            return;
        }
        band[at..at + tile.w].swap_with_slice(&mut tile.bytes[src..src + tile.w]);
    }
}

fn write_tile(plane: &mut Plane<u8>, width: usize, tile: &TileDelta, bytes: &[u8]) {
    if width == 0 {
        return;
    }
    for row in 0..tile.h {
        let y = tile.y + row;
        let Some(dst) = plane.row_mut(y, width) else {
            return;
        };
        let src = row * tile.w;
        if tile.x + tile.w <= dst.len() {
            dst[tile.x..tile.x + tile.w].copy_from_slice(&bytes[src..src + tile.w]);
        }
    }
}

fn swap_tile(plane: &mut Plane<u8>, width: usize, tile: &mut TileDelta) {
    if width == 0 || tile.h == 0 {
        return;
    }
    for row in 0..tile.h {
        let y = tile.y + row;
        let Some(dst) = plane.row_mut(y, width) else {
            return;
        };
        let src = row * tile.w;
        if tile.x + tile.w <= dst.len() {
            dst[tile.x..tile.x + tile.w].swap_with_slice(&mut tile.bytes[src..src + tile.w]);
        }
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

/// A copy of `doc` sharing no tracked plane with it, so neither side's next
/// write copies a whole plane.
pub(crate) fn detached_copy(doc: &Document) -> Document {
    let mut copy = doc.clone();
    each_plane_mut(&mut copy, &mut |p, _, _, _| *p = copy_plane(p));
    copy
}

/// A uniquely held copy of `plane`, copied on every core band by band, so a
/// diverged banded plane is never materialized first.
pub(crate) fn copy_plane(plane: &Plane<u8>) -> Plane<u8> {
    Plane::build(plane.len(), |dst| {
        let bands: Vec<&[u8]> = plane.bands().collect();
        let mut targets = Vec::with_capacity(bands.len());
        let mut rest = dst;
        for band in &bands {
            let (head, tail) = rest.split_at_mut(band.len());
            targets.push(head);
            rest = tail;
        }
        targets
            .into_par_iter()
            .zip(bands)
            .for_each(|(d, s)| copy_par(d, s));
    })
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
