//! A stroke's pre-stroke pixels, saved a tile at a time just before a dab first
//! writes the tile (#1603). The 8-bit planes are the working copy; the optional
//! native planes are the retained store a 16-bit tone dab edits.
//!
//! Tiles are boxed on first save, so a stroke holds only what its dabs reached,
//! and sit on a grid anchored to document coordinates, so a layer that grows
//! under the stroke moves no saved pixel.

use super::PlaneIndex;
use pictura_core::{BitDepth, Layer, Plane, PsdRect, Samples};
use std::ops::Range;

/// Tile edge of the pre-stroke pixels a stroke keeps.
const SAVED_TILE: i32 = 64;

/// One saved tile: the part of a 64² grid tile inside the layer when it was
/// saved, as a `width × height` block per 8-bit plane the stroke edits, then
/// one per native plane. The rest of the grid tile lay outside the layer then,
/// so any of it the layer grew into is new: its pre-stroke value is zero.
struct SavedTile {
    /// The saved box's document origin.
    left: i32,
    top: i32,
    width: usize,
    height: usize,
    planes: Box<[u8]>,
    native: Box<[u16]>,
}

impl SavedTile {
    fn len(&self) -> usize {
        self.width * self.height
    }

    /// The saved box as layer-local columns and rows of a layer at `rect`.
    fn local(&self, rect: PsdRect) -> (Range<usize>, Range<usize>) {
        let x = (self.left - rect.left) as usize;
        let y = (self.top - rect.top) as usize;
        (x..x + self.width, y..y + self.height)
    }
}

/// The target layer's pre-stroke pixels.
pub(super) struct Saved {
    /// Each edited plane's block in a tile, by R, G, B, alpha.
    slots: [Option<usize>; 4],
    slot_count: usize,
    /// The native colour plane ids a 16-bit tone stroke edits.
    native: Option<Vec<i16>>,
    /// The tiles crossing `rect`, row-major from document tile
    /// `(first_column, first_row)`.
    tiles: Vec<Option<Box<SavedTile>>>,
    first_column: i32,
    first_row: i32,
    columns: i32,
    /// The target layer's bounds when the stroke began, or after it grew.
    rect: PsdRect,
    pub(super) width: i32,
    pub(super) height: i32,
}

/// The document tiles `[first, first + count)` that `from..to` crosses.
fn tile_span(from: i32, to: i32) -> (i32, i32) {
    if to <= from {
        return (from.div_euclid(SAVED_TILE), 0);
    }
    let first = from.div_euclid(SAVED_TILE);
    (first, (to - 1).div_euclid(SAVED_TILE) - first + 1)
}

impl Saved {
    pub(super) fn new(layer: &Layer, ch: PlaneIndex) -> Self {
        let mut slots = [None; 4];
        let mut slot_count = 0;
        for (slot, k) in slots.iter_mut().zip(ch) {
            if k.and_then(|k| layer.channels().get(k)).is_some() {
                *slot = Some(slot_count);
                slot_count += 1;
            }
        }
        let mut saved = Self {
            slots,
            slot_count,
            native: None,
            tiles: Vec::new(),
            first_column: 0,
            first_row: 0,
            columns: 0,
            rect: layer.rect,
            width: 0,
            height: 0,
        };
        saved.cover(layer.rect);
        saved
    }

    /// Lay the tile grid over `rect`, moving every saved tile to its slot.
    fn cover(&mut self, rect: PsdRect) {
        let (first_column, columns) = tile_span(rect.left, rect.right);
        let (first_row, rows) = tile_span(rect.top, rect.bottom);
        let mut tiles: Vec<Option<Box<SavedTile>>> = Vec::new();
        tiles.resize_with((columns * rows).max(0) as usize, || None);
        let old = std::mem::take(&mut self.tiles);
        for (t, tile) in old.into_iter().enumerate() {
            let Some(tile) = tile else { continue };
            let tx = self.first_column + t as i32 % self.columns - first_column;
            let ty = self.first_row + t as i32 / self.columns - first_row;
            if (0..columns).contains(&tx) && (0..rows).contains(&ty) {
                tiles[(ty * columns + tx) as usize] = Some(tile);
            }
        }
        self.tiles = tiles;
        self.first_column = first_column;
        self.first_row = first_row;
        self.columns = columns;
        self.rect = rect;
        self.width = rect.width().max(0);
        self.height = rect.height().max(0);
    }

    /// Keep the layer's retained 16-bit colour planes `ids` for a tone stroke.
    /// Called before any dab, so no tile is saved yet.
    pub(super) fn enable_native(&mut self, layer: &Layer, ids: &[i16]) -> bool {
        let Some(store) = layer.source_channels.as_ref() else {
            return false;
        };
        if store.depth != BitDepth::Sixteen || store.rect != self.rect {
            return false;
        }
        let n = (self.width * self.height).max(0) as usize;
        let all_present = ids.iter().all(|&id| {
            store.planes.iter().any(|(cid, samples)| {
                *cid == id && matches!(samples, Samples::U16(plane) if plane.len() == n)
            })
        });
        if !all_present || self.tiles.iter().any(Option::is_some) {
            return false;
        }
        self.native = Some(ids.to_vec());
        true
    }

    /// Grid tile `t` clipped to the layer, as layer-local columns and rows.
    fn local_box(&self, t: usize) -> (Range<usize>, Range<usize>) {
        let left = (self.first_column + t as i32 % self.columns) * SAVED_TILE;
        let top = (self.first_row + t as i32 / self.columns) * SAVED_TILE;
        let x0 = left.max(self.rect.left);
        let y0 = top.max(self.rect.top);
        let x1 = (left + SAVED_TILE).min(self.rect.right).max(x0);
        let y1 = (top + SAVED_TILE).min(self.rect.bottom).max(y0);
        let local = |a: i32, origin: i32| (a - origin) as usize;
        (
            local(x0, self.rect.left)..local(x1, self.rect.left),
            local(y0, self.rect.top)..local(y1, self.rect.top),
        )
    }

    /// Save every unsaved tile meeting the layer-local box `[x0, x1] × [y0, y1]`
    /// (inclusive, clamped to the layer) from `layer`, which still holds them
    /// unpainted.
    pub(super) fn save(
        &mut self,
        layer: &Layer,
        ch: PlaneIndex,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    ) {
        let (x0, y0) = (x0.max(0), y0.max(0));
        let (x1, y1) = (x1.min(self.width - 1), y1.min(self.height - 1));
        if x1 < x0 || y1 < y0 {
            return;
        }
        let w = self.width as usize;
        let (left, top) = (self.rect.left, self.rect.top);
        let column = |x: i32| (left + x).div_euclid(SAVED_TILE) - self.first_column;
        let row = |y: i32| (top + y).div_euclid(SAVED_TILE) - self.first_row;
        let (c0, c1, r0, r1) = (column(x0), column(x1), row(y0), row(y1));
        let native_planes = self.native.as_ref().map_or(0, Vec::len);
        for ty in r0..=r1 {
            for tx in c0..=c1 {
                let t = (ty * self.columns + tx) as usize;
                if self.tiles[t].is_some() {
                    continue;
                }
                let (xs, ys) = self.local_box(t);
                let n = xs.len() * ys.len();
                let mut tile = SavedTile {
                    left: self.rect.left + xs.start as i32,
                    top: self.rect.top + ys.start as i32,
                    width: xs.len(),
                    height: ys.len(),
                    planes: vec![0u8; self.slot_count * n].into_boxed_slice(),
                    native: vec![0u16; native_planes * n].into_boxed_slice(),
                };
                // One read per tile, not per row (#1238): a tiled plane is
                // never materialized, and a 64-row tile costs one lookup.
                for (slot, k) in self.slots.iter().zip(ch) {
                    let (Some(slot), Some(src)) = (*slot, k.and_then(|k| layer.channels().get(k)))
                    else {
                        continue;
                    };
                    let block = &mut tile.planes[slot * n..][..n];
                    src.data.read_rect_into(xs.clone(), ys.clone(), w, block);
                }
                if let (Some(ids), Some(store)) =
                    (self.native.as_ref(), layer.source_channels.as_ref())
                {
                    for (slot, id) in ids.iter().enumerate() {
                        let Some((_, Samples::U16(src))) =
                            store.planes.iter().find(|(cid, _)| cid == id)
                        else {
                            continue;
                        };
                        let block = &mut tile.native[slot * n..][..n];
                        src.read_rect_into(xs.clone(), ys.clone(), w, block);
                    }
                }
                self.tiles[t] = Some(Box::new(tile));
            }
        }
    }

    /// The saved R, G, B and alpha of layer-local `(x, y)`: `None` for a plane
    /// the layer lacks, zero for a tile not saved yet or a pixel the layer grew
    /// into after its tile was saved.
    #[inline]
    pub(super) fn pixel(&self, x: usize, y: usize) -> [Option<u8>; 4] {
        let dx = self.rect.left + x as i32;
        let dy = self.rect.top + y as i32;
        let tx = dx.div_euclid(SAVED_TILE) - self.first_column;
        let ty = dy.div_euclid(SAVED_TILE) - self.first_row;
        let tile = if (0..self.columns).contains(&tx) && ty >= 0 {
            self.tiles
                .get((ty * self.columns + tx) as usize)
                .and_then(Option::as_deref)
        } else {
            None
        };
        let at = tile.and_then(|tile| {
            let x = usize::try_from(dx - tile.left).ok()?;
            let y = usize::try_from(dy - tile.top).ok()?;
            (x < tile.width && y < tile.height).then(|| (tile, y * tile.width + x))
        });
        self.slots.map(|slot| {
            slot.map(|slot| at.map_or(0, |(tile, at)| tile.planes[slot * tile.len() + at]))
        })
    }

    /// Whether `layer` still has the planes and bounds this stroke began on. A
    /// document restructured or a target moved under a live stroke is refused
    /// rather than painted at stale coordinates or indexed out of bounds.
    pub(super) fn fits(&self, layer: &Layer, ch: PlaneIndex) -> bool {
        let n = (self.width * self.height) as usize;
        if layer.rect != self.rect
            || !self.slots.iter().zip(ch).all(|(slot, k)| match (slot, k) {
                (Some(_), Some(k)) => layer.channels().get(k).is_some_and(|c| c.data.len() == n),
                (None, None) => true,
                _ => false,
            })
        {
            return false;
        }
        let Some(ids) = self.native.as_ref() else {
            return true;
        };
        let Some(store) = layer.source_channels.as_ref() else {
            return false;
        };
        if store.depth != BitDepth::Sixteen || store.rect != self.rect {
            return false;
        }
        ids.iter().all(|id| {
            store.planes.iter().any(|(cid, samples)| {
                cid == id && matches!(samples, Samples::U16(plane) if plane.len() == n)
            })
        })
    }

    /// Write every saved tile back into `layer`.
    pub(super) fn restore(&self, layer: &mut Layer, ch: PlaneIndex) {
        let (w, rect) = (self.width as usize, self.rect);
        let saved = || self.tiles.iter().flatten();
        for (slot, k) in self.slots.iter().zip(ch) {
            let (Some(slot), Some(k)) = (*slot, k) else {
                continue;
            };
            let Some(dst) = layer.channels_mut().get_mut(k) else {
                continue;
            };
            for tile in saved() {
                let (xs, ys) = tile.local(rect);
                let n = tile.len();
                dst.data
                    .write_rect_from(xs, ys, w, &tile.planes[slot * n..][..n]);
            }
            dst.data.settle();
        }
        let (Some(ids), Some(store)) = (self.native.as_ref(), layer.source_channels.as_mut())
        else {
            return;
        };
        for (slot, id) in ids.iter().enumerate() {
            let Some(dst): Option<&mut Plane<u16>> =
                store
                    .planes
                    .iter_mut()
                    .find_map(|(cid, samples)| match samples {
                        Samples::U16(plane) if cid == id => Some(plane),
                        _ => None,
                    })
            else {
                continue;
            };
            for tile in saved() {
                let (xs, ys) = tile.local(rect);
                let n = tile.len();
                dst.write_rect_from(xs, ys, w, &tile.native[slot * n..][..n]);
            }
        }
    }

    /// Follow the layer as it grows (#1208): the grid is laid over the new
    /// bounds and the saved tiles keep their document position, so nothing is
    /// copied. A pixel the layer grew into reads zero, its pre-stroke value.
    pub(super) fn grow(&mut self, new_rect: PsdRect) {
        if new_rect != self.rect {
            self.cover(new_rect);
        }
    }

    /// The document tile coordinates of every saved tile, in grid order.
    #[cfg(test)]
    pub(super) fn saved_tiles(&self) -> Vec<(i32, i32)> {
        self.tiles
            .iter()
            .enumerate()
            .filter(|(_, tile)| tile.is_some())
            .map(|(t, _)| {
                (
                    self.first_column + t as i32 % self.columns,
                    self.first_row + t as i32 / self.columns,
                )
            })
            .collect()
    }

    /// The bytes the saved tiles hold.
    #[cfg(test)]
    pub(super) fn held_bytes(&self) -> usize {
        self.tiles
            .iter()
            .flatten()
            .map(|tile| tile.planes.len() + tile.native.len() * 2)
            .sum()
    }
}
