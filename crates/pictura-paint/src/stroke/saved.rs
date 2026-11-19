//! A stroke's pre-stroke pixels, saved a tile at a time just before a dab first
//! writes the tile. The 8-bit planes are the working copy; the optional native
//! planes are the retained store a 16-bit tone dab edits.

use super::PlaneIndex;
use pictura_core::{BitDepth, Layer, Plane, PsdRect, Samples};

/// Tile edge of the pre-stroke pixels a stroke keeps.
const SAVED_TILE: i32 = 64;

/// The target layer's pre-stroke pixels. The planes are layer-sized but
/// zero-filled by the allocator, so only the saved tiles ever take memory.
pub(super) struct Saved {
    planes: [Option<Vec<u8>>; 4],
    /// The native colour planes a 16-bit tone stroke edits, keyed by id.
    native: Option<NativeSaved>,
    tiles: Vec<bool>,
    columns: i32,
    /// The target layer's bounds when the stroke began.
    rect: PsdRect,
    pub(super) width: i32,
    pub(super) height: i32,
}

struct NativeSaved {
    ids: Vec<i16>,
    planes: Vec<Vec<u16>>,
}

impl Saved {
    pub(super) fn new(layer: &Layer, ch: PlaneIndex) -> Self {
        let width = layer.rect.width().max(0);
        let height = layer.rect.height().max(0);
        let columns = (width + SAVED_TILE - 1) / SAVED_TILE;
        let rows = (height + SAVED_TILE - 1) / SAVED_TILE;
        let planes = ch.map(|k| {
            k.and_then(|k| layer.channels().get(k))
                .map(|c| vec![0u8; c.data.len()])
        });
        Self {
            planes,
            native: None,
            tiles: vec![false; (columns * rows) as usize],
            columns,
            rect: layer.rect,
            width,
            height,
        }
    }

    /// Keep the layer's retained 16-bit colour planes `ids` for a tone stroke.
    pub(super) fn enable_native(&mut self, layer: &Layer, ids: &[i16]) -> bool {
        let Some(store) = layer.source_channels.as_ref() else {
            return false;
        };
        if store.depth != BitDepth::Sixteen || store.rect != self.rect {
            return false;
        }
        let n = (self.width * self.height).max(0) as usize;
        let mut planes = Vec::with_capacity(ids.len());
        for &id in ids {
            let Some((_, Samples::U16(plane))) = store.planes.iter().find(|(cid, _)| *cid == id)
            else {
                return false;
            };
            if plane.len() != n {
                return false;
            }
            planes.push(vec![0u16; n]);
        }
        self.native = Some(NativeSaved {
            ids: ids.to_vec(),
            planes,
        });
        true
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
        let mut tile = Vec::new();
        for ty in y0 / SAVED_TILE..=y1 / SAVED_TILE {
            for tx in x0 / SAVED_TILE..=x1 / SAVED_TILE {
                let t = (ty * self.columns + tx) as usize;
                if self.tiles[t] {
                    continue;
                }
                self.tiles[t] = true;
                let (left, top) = (tx * SAVED_TILE, ty * SAVED_TILE);
                let right = (left + SAVED_TILE).min(self.width) as usize;
                let bottom = (top + SAVED_TILE).min(self.height) as usize;
                for (k, plane) in self.planes.iter_mut().enumerate() {
                    let (Some(plane), Some(src)) =
                        (plane.as_mut(), ch[k].and_then(|k| layer.channels().get(k)))
                    else {
                        continue;
                    };
                    // One read per tile, not per row (#1238): a tiled plane
                    // is never materialized, and a 64-row tile costs one
                    // lookup instead of 64.
                    let (xs, ys) = (left as usize..right, top as usize..bottom);
                    let tw = xs.len();
                    tile.clear();
                    tile.resize(tw * ys.len(), 0);
                    src.data
                        .read_rect_into(xs.clone(), ys.clone(), w, &mut tile);
                    for (row, y) in tile.chunks_exact(tw).zip(ys) {
                        let (a, b) = (y * w + xs.start, (y * w + xs.end).min(plane.len()));
                        if a < b {
                            plane[a..b].copy_from_slice(&row[..b - a]);
                        }
                    }
                }
                if let Some(native) = self.native.as_mut() {
                    let Some(store) = layer.source_channels.as_ref() else {
                        continue;
                    };
                    for (slot, id) in native.ids.iter().enumerate() {
                        let Some((_, Samples::U16(src))) =
                            store.planes.iter().find(|(cid, _)| cid == id)
                        else {
                            continue;
                        };
                        let dst = &mut native.planes[slot];
                        for y in top as usize..bottom {
                            let (a, b) = (y * w + left as usize, (y * w + right).min(dst.len()));
                            if a < b {
                                src.read_rect_into(
                                    left as usize..right,
                                    y..y + 1,
                                    w,
                                    &mut dst[a..b],
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Whether `layer` still has the planes and bounds this stroke began on. A
    /// document restructured or a target moved under a live stroke is refused
    /// rather than painted at stale coordinates or indexed out of bounds.
    pub(super) fn fits(&self, layer: &Layer, ch: PlaneIndex) -> bool {
        if layer.rect != self.rect
            || !self
                .planes
                .iter()
                .zip(ch)
                .all(|(plane, k)| match (plane, k) {
                    (Some(plane), Some(k)) => layer
                        .channels()
                        .get(k)
                        .is_some_and(|c| c.data.len() == plane.len()),
                    (None, None) => true,
                    _ => false,
                })
        {
            return false;
        }
        let Some(native) = self.native.as_ref() else {
            return true;
        };
        let Some(store) = layer.source_channels.as_ref() else {
            return false;
        };
        if store.depth != BitDepth::Sixteen || store.rect != self.rect {
            return false;
        }
        native.ids.iter().enumerate().all(|(slot, id)| {
            store
                .planes
                .iter()
                .find(|(cid, _)| cid == id)
                .is_some_and(|(_, samples)| match samples {
                    Samples::U16(plane) => plane.len() == native.planes[slot].len(),
                    _ => false,
                })
        })
    }

    pub(super) fn planes(&self) -> [Option<&[u8]>; 4] {
        [0, 1, 2, 3].map(|k| self.planes[k].as_deref())
    }

    /// Write every saved tile back into `layer`.
    pub(super) fn restore(&self, layer: &mut Layer, ch: PlaneIndex) {
        let w = self.width as usize;
        for (t, _) in self.tiles.iter().enumerate().filter(|(_, saved)| **saved) {
            let (tx, ty) = (t as i32 % self.columns, t as i32 / self.columns);
            let (left, top) = (tx * SAVED_TILE, ty * SAVED_TILE);
            let right = (left + SAVED_TILE).min(self.width) as usize;
            let bottom = (top + SAVED_TILE).min(self.height) as usize;
            for (k, plane) in self.planes.iter().enumerate() {
                let (Some(plane), Some(k)) = (plane.as_ref(), ch[k]) else {
                    continue;
                };
                let Some(dst) = layer.channels_mut().get_mut(k) else {
                    continue;
                };
                for y in top as usize..bottom {
                    let (a, b) = (y * w + left as usize, (y * w + right).min(plane.len()));
                    if a < b {
                        dst.data
                            .write_rect_from(left as usize..right, y..y + 1, w, &plane[a..b]);
                    }
                }
                dst.data.settle();
            }
        }
        let Some(native) = self.native.as_ref() else {
            return;
        };
        let Some(store) = layer.source_channels.as_mut() else {
            return;
        };
        // Resolve each native destination once, then walk the saved tiles once
        // for every plane rather than rescanning the tile grid per plane.
        let mut dests: Vec<Option<&mut Plane<u16>>> = native.ids.iter().map(|_| None).collect();
        for (id, samples) in store.planes.iter_mut() {
            let Some(slot) = native.ids.iter().position(|want| want == id) else {
                continue;
            };
            let Samples::U16(plane) = samples else {
                continue;
            };
            if dests[slot].is_none() {
                dests[slot] = Some(plane);
            }
        }
        for (t, _) in self.tiles.iter().enumerate().filter(|(_, saved)| **saved) {
            let (tx, ty) = (t as i32 % self.columns, t as i32 / self.columns);
            let (left, top) = (tx * SAVED_TILE, ty * SAVED_TILE);
            let right = (left + SAVED_TILE).min(self.width) as usize;
            let bottom = (top + SAVED_TILE).min(self.height) as usize;
            for (slot, dst) in dests.iter_mut().enumerate() {
                let Some(dst) = dst.as_mut() else {
                    continue;
                };
                let saved = &native.planes[slot];
                for y in top as usize..bottom {
                    let (a, b) = (y * w + left as usize, (y * w + right).min(saved.len()));
                    if a < b {
                        dst.write_rect_from(left as usize..right, y..y + 1, w, &saved[a..b]);
                    }
                }
            }
        }
    }

    /// Pad the saved planes when the layer grows (#1208). Saved tiles keep
    /// their pre-stroke bytes; old pixels no tile saved yet are still the
    /// layer's own, so they are copied from `layer` now.
    pub(super) fn grow(&mut self, layer: &Layer, ch: PlaneIndex, new_rect: PsdRect) {
        let old_rect = self.rect;
        let (old_w, old_h) = (self.width, self.height);
        let (new_w, new_h) = (new_rect.width().max(0), new_rect.height().max(0));
        if new_w == old_w && new_h == old_h {
            return;
        }
        let (dx, dy) = (
            (old_rect.left - new_rect.left).max(0),
            (old_rect.top - new_rect.top).max(0),
        );
        let old_tiles = std::mem::take(&mut self.tiles);
        let old_columns = self.columns;

        for (k, plane) in self.planes.iter_mut().enumerate() {
            let Some(plane) = plane else { continue };
            let mut data = vec![0u8; (new_w * new_h) as usize];
            for y in 0..old_h {
                let src = (y * old_w) as usize;
                let dst = ((dy + y) * new_w + dx) as usize;
                let n = old_w as usize;
                if src + n <= plane.len() && dst + n <= data.len() {
                    data[dst..dst + n].copy_from_slice(&plane[src..src + n]);
                }
            }
            if let Some(source) = ch[k].and_then(|i| layer.channels().get(i)) {
                // The layer has already grown, so an old pixel sits at its
                // grown position; the unsaved tiles' rows are read in place.
                let stride = new_w as usize;
                for ty in 0..(old_h + SAVED_TILE - 1) / SAVED_TILE {
                    for tx in 0..old_columns {
                        if old_tiles[(ty * old_columns + tx) as usize] {
                            continue;
                        }
                        let (x0, y0) = (tx * SAVED_TILE, ty * SAVED_TILE);
                        let x1 = (x0 + SAVED_TILE).min(old_w);
                        let n = (x1 - x0) as usize;
                        for y in y0..(y0 + SAVED_TILE).min(old_h) {
                            let (gx, gy) = ((dx + x0) as usize, (dy + y) as usize);
                            let at = gy * stride + gx;
                            if at + n <= data.len() {
                                source.data.read_rect_into(
                                    gx..gx + n,
                                    gy..gy + 1,
                                    stride,
                                    &mut data[at..at + n],
                                );
                            }
                        }
                    }
                }
            }
            *plane = data;
        }
        if let Some(native) = self.native.as_mut() {
            for (slot, id) in native.ids.iter().enumerate() {
                let saved = &native.planes[slot];
                let mut data = vec![0u16; (new_w * new_h) as usize];
                for y in 0..old_h {
                    let src = (y * old_w) as usize;
                    let dst = ((dy + y) * new_w + dx) as usize;
                    let n = old_w as usize;
                    if src + n <= saved.len() && dst + n <= data.len() {
                        data[dst..dst + n].copy_from_slice(&saved[src..src + n]);
                    }
                }
                if let Some(store) = layer.source_channels.as_ref() {
                    if let Some((_, Samples::U16(source))) =
                        store.planes.iter().find(|(cid, _)| cid == id)
                    {
                        let stride = new_w as usize;
                        for ty in 0..(old_h + SAVED_TILE - 1) / SAVED_TILE {
                            for tx in 0..old_columns {
                                if old_tiles[(ty * old_columns + tx) as usize] {
                                    continue;
                                }
                                let (x0, y0) = (tx * SAVED_TILE, ty * SAVED_TILE);
                                let x1 = (x0 + SAVED_TILE).min(old_w);
                                let n = (x1 - x0) as usize;
                                for y in y0..(y0 + SAVED_TILE).min(old_h) {
                                    let (gx, gy) = ((dx + x0) as usize, (dy + y) as usize);
                                    let at = gy * stride + gx;
                                    if at + n <= data.len() {
                                        source.read_rect_into(
                                            gx..gx + n,
                                            gy..gy + 1,
                                            stride,
                                            &mut data[at..at + n],
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                native.planes[slot] = data;
            }
        }
        let columns = (new_w + SAVED_TILE - 1) / SAVED_TILE;
        let rows = (new_h + SAVED_TILE - 1) / SAVED_TILE;
        let mut tiles = vec![false; (columns * rows) as usize];
        let (old_right, old_bottom) = (old_rect.left + old_w, old_rect.top + old_h);
        for ty in 0..rows {
            for tx in 0..columns {
                let (left, top) = (
                    new_rect.left + tx * SAVED_TILE,
                    new_rect.top + ty * SAVED_TILE,
                );
                if left < old_right
                    && left + SAVED_TILE > old_rect.left
                    && top < old_bottom
                    && top + SAVED_TILE > old_rect.top
                {
                    tiles[(ty * columns + tx) as usize] = true;
                }
            }
        }
        self.columns = columns;
        self.tiles = tiles;
        self.rect = new_rect;
        self.width = new_w;
        self.height = new_h;
    }
}
