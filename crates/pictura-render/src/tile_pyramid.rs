//! The display composite as a cache of 256² tiles per pyramid level (#1216).
//!
//! Nothing here is authoritative: every tile can be rebuilt from its level-0
//! source (the composite, or a soloed mask). Level-0 tiles hold straight
//! interleaved RGBA, the source the levels are averaged from; every other
//! level holds premultiplied RGBA, as [`crate::ViewPyramid`] does. A crop of
//! any level is byte-identical to the same crop of a whole rebuild.
//!
//! Damage either repairs the cached tiles it touches from the region the edit
//! already composited ([`TilePyramid::repair`]) or drops them
//! ([`TilePyramid::damage`]). A worker result is installed only if no damage
//! reached its footprint after its [`Ticket`] was taken.
//!
//! The cache keeps to a byte budget. Over it, unpinned level-0 tiles go first
//! (the largest share of any view's footprint, and the cheapest to rebuild a
//! few of), then unpinned tiles of other levels, least recently used first.

use crate::region_level::{halve_level, level_size, region_level, Level0Source};
use crate::view_pyramid::{PyramidLevel, SMALLEST_SIDE};
use pictura_core::{PixelBuffer, PsdRect};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Epochs are unique across caches, so a worker result for a cache that has
/// since been replaced (a resize) is refused by its successor.
static EPOCHS: AtomicU64 = AtomicU64::new(1);

fn next_epoch() -> u64 {
    EPOCHS.fetch_add(1, Ordering::Relaxed)
}

/// A tile's side in its own level's pixels.
pub const TILE_SIDE: u32 = 256;

/// What a tile shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileSource {
    /// The document composite in the display space.
    Composite,
    /// The soloed raster mask as opaque grey.
    Mask,
}

/// One tile of one level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub source: TileSource,
    pub level: u8,
    pub tx: u32,
    pub ty: u32,
}

/// The cache state a worker build answers; see [`TilePyramid::install`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    epoch: u64,
    clock: u64,
}

/// Cache accounting (#942).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileStats {
    pub resident_bytes: u64,
    pub tiles: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

struct Entry {
    data: Arc<[u8]>,
    used: u64,
    /// Damage reached the tile after it was built: it is shown only as a
    /// stand-in until its rebuild lands, and it is evicted first.
    stale: bool,
}

/// The tile cache of one document view. See the module notes.
pub struct TilePyramid {
    width: u32,
    height: u32,
    levels: usize,
    tiles: HashMap<TileKey, Entry>,
    pinned: HashSet<TileKey>,
    budget: u64,
    epoch: u64,
    clock: u64,
    /// Per level-0 tile, the clock of the last damage that reached it.
    damaged: Vec<u64>,
    touch: u64,
    stats: TileStats,
}

/// The number of levels a `width`×`height` image has: level 0, then each
/// halving while the next long side stays at or above [`SMALLEST_SIDE`].
pub fn level_count(width: u32, height: u32) -> usize {
    let (mut w, mut h, mut count) = (width, height, 1);
    loop {
        let (nw, nh) = (w.div_ceil(2), h.div_ceil(2));
        if nw.max(nh) < SMALLEST_SIDE || nw <= 1 || nh <= 1 {
            return count;
        }
        (w, h, count) = (nw, nh, count + 1);
    }
}

impl TilePyramid {
    pub fn new(width: u32, height: u32, budget: u64) -> Self {
        let (cols, rows) = grid(width, height);
        Self {
            width,
            height,
            levels: level_count(width, height),
            tiles: HashMap::new(),
            pinned: HashSet::new(),
            budget,
            epoch: next_epoch(),
            clock: 0,
            damaged: vec![0; cols * rows],
            touch: 0,
            stats: TileStats::default(),
        }
    }

    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.width == width && self.height == height
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn level_count(&self) -> usize {
        self.levels
    }

    /// The size of `level`, `(0, 0)` past the last.
    pub fn level_size(&self, level: usize) -> (u32, u32) {
        if level < self.levels {
            level_size(self.width, self.height, level)
        } else {
            (0, 0)
        }
    }

    pub fn set_budget(&mut self, budget: u64) {
        self.budget = budget;
        self.evict();
    }

    pub fn stats(&self) -> TileStats {
        TileStats {
            tiles: self.tiles.len() as u64,
            ..self.stats
        }
    }

    /// The rectangle `key` covers in its level, clipped to the level.
    pub fn tile_rect(&self, key: TileKey) -> PsdRect {
        let (lw, lh) = self.level_size(key.level as usize);
        let side = TILE_SIDE as i32;
        let (left, top) = (key.tx as i32 * side, key.ty as i32 * side);
        PsdRect {
            top,
            left,
            bottom: (top + side).min(lh as i32),
            right: (left + side).min(lw as i32),
        }
    }

    /// The tiles of `level` that `rect` (in that level's pixels) touches.
    pub fn keys_in(&self, source: TileSource, level: usize, rect: PsdRect) -> Vec<TileKey> {
        let (lw, lh) = self.level_size(level);
        let side = TILE_SIDE as i32;
        let x0 = rect.left.max(0) / side;
        let y0 = rect.top.max(0) / side;
        let x1 = (rect.right.min(lw as i32) + side - 1) / side;
        let y1 = (rect.bottom.min(lh as i32) + side - 1) / side;
        let mut keys = Vec::new();
        for ty in y0..y1.max(y0) {
            for tx in x0..x1.max(x0) {
                keys.push(TileKey {
                    source,
                    level: level as u8,
                    tx: tx as u32,
                    ty: ty as u32,
                });
            }
        }
        keys
    }

    /// Whether a current (not stale) tile is cached.
    pub fn contains(&self, key: TileKey) -> bool {
        self.tiles.get(&key).is_some_and(|entry| !entry.stale)
    }

    /// A current cached tile's bytes, counted as a hit or a miss.
    pub fn get(&mut self, key: TileKey) -> Option<Arc<[u8]>> {
        self.touch += 1;
        match self.tiles.get_mut(&key).filter(|entry| !entry.stale) {
            Some(entry) => {
                entry.used = self.touch;
                self.stats.hits += 1;
                Some(Arc::clone(&entry.data))
            }
            None => {
                self.stats.misses += 1;
                None
            }
        }
    }

    /// Install a tile built from the current state.
    pub fn insert(&mut self, key: TileKey, data: Arc<[u8]>) {
        self.touch += 1;
        let bytes = data.len() as u64;
        if let Some(old) = self.tiles.insert(
            key,
            Entry {
                data,
                used: self.touch,
                stale: false,
            },
        ) {
            self.stats.resident_bytes -= old.data.len() as u64;
        }
        self.stats.resident_bytes += bytes;
        self.evict();
    }

    /// The state a worker build starts from.
    pub fn ticket(&self) -> Ticket {
        Ticket {
            epoch: self.epoch,
            clock: self.clock,
        }
    }

    /// Install a worker's tile unless damage reached its footprint after
    /// `ticket`. Returns whether it was installed.
    pub fn install(&mut self, key: TileKey, ticket: Ticket, data: Arc<[u8]>) -> bool {
        if ticket.epoch != self.epoch || self.damaged_since(key, ticket.clock) {
            return false;
        }
        self.insert(key, data);
        true
    }

    /// The document changed as a whole: every tile becomes stale (still a
    /// stand-in, never served as current) and older worker results are refused.
    pub fn invalidate(&mut self) {
        self.epoch = next_epoch();
        for entry in self.tiles.values_mut() {
            entry.stale = true;
        }
    }

    /// Forget every tile of `source` (a soloed mask that is no longer shown).
    pub fn drop_source(&mut self, source: TileSource) {
        self.tiles.retain(|key, _| key.source != source);
        self.recount();
    }

    /// Mark every cached tile of `source` that `rect` (level-0 pixels) reaches
    /// at any level stale, and stamp the damage so an older worker result is
    /// refused.
    pub fn damage(&mut self, source: TileSource, rect: PsdRect) {
        let rect = clip(rect, self.width, self.height);
        if rect.width() <= 0 || rect.height() <= 0 {
            return;
        }
        self.stamp(rect);
        for level in 0..self.levels {
            for key in self.keys_in(source, level, scale_down(rect, level)) {
                if let Some(entry) = self.tiles.get_mut(&key) {
                    entry.stale = true;
                }
            }
        }
    }

    /// Whether any current tile of `source` lies under `rect` (level-0
    /// pixels) at any level: a repair there has something to write.
    pub fn any_current_under(&self, source: TileSource, rect: PsdRect) -> bool {
        let rect = clip(rect, self.width, self.height);
        (0..self.levels).any(|level| {
            self.keys_in(source, level, scale_down(rect, level))
                .into_iter()
                .any(|key| self.contains(key))
        })
    }

    /// Repair the cached tiles of `source` under `rect` from `region`: straight
    /// planar RGBA of exactly `rect` (level-0 pixels) after the edit. `rect`
    /// must be aligned by [`Self::repair_rect`]. A level's cached tiles are
    /// patched; uncached ones stay absent and are built when asked for.
    pub fn repair(&mut self, source: TileSource, rect: PsdRect, region: &PixelBuffer) {
        if rect.width() <= 0 || rect.height() <= 0 {
            return;
        }
        self.stamp(rect);
        let mut data = crate::region_level::premultiply_planar(region);
        let (mut origin, mut size) = ((rect.left, rect.top), (region.width, region.height));
        // Level 0 is stored straight: copy it from the planes, not the
        // premultiplied rows.
        let straight = interleave(region);
        self.patch_level(source, 0, rect, &straight);
        for level in 1..self.levels {
            if !self
                .tiles
                .iter()
                .any(|(k, e)| !e.stale && k.source == source && k.level as usize >= level)
            {
                break;
            }
            let (halved, next) =
                halve_level(&data, origin, size, (self.width, self.height), level - 1);
            data = next;
            origin = (halved.left, halved.top);
            size = (halved.width() as u32, halved.height() as u32);
            self.patch_level(source, level, halved, &data);
        }
    }

    /// Overwrite `rect` of `level` in the current cached tiles with `rows`
    /// (interleaved, covering `rect`: straight at level 0, premultiplied
    /// above), leaving every other level alone. A display-only preview
    /// presents through this; the commit or cancel that ends it repairs or
    /// invalidates the tiles.
    pub fn patch(&mut self, source: TileSource, level: usize, rect: PsdRect, rows: &[u8]) {
        let (rw, rh) = (rect.width().max(0) as usize, rect.height().max(0) as usize);
        if rw == 0 || rh == 0 || rows.len() < rw * rh * 4 || level >= self.levels {
            return;
        }
        self.patch_level(source, level, rect, rows);
    }

    /// Paint `region` (straight planar RGBA covering `rect`, in `level`
    /// pixels) into the stale cached tiles of `source` at `level` as their
    /// stand-in. The tiles stay stale, so a worker still rebuilds them; until
    /// then the canvas shows this instead of what they held before an edit.
    /// `over` composites it at `opacity` over the stand-in, otherwise it
    /// replaces it. Current and absent tiles are left alone.
    pub fn paint_stand_in(
        &mut self,
        source: TileSource,
        level: usize,
        rect: PsdRect,
        region: &PixelBuffer,
        opacity: u8,
        over: bool,
    ) {
        let (rw, rh) = (region.width as usize, region.height as usize);
        if region.channels != 4
            || level >= self.levels
            || rect.width() != rw as i32
            || rect.height() != rh as i32
            || rw == 0
            || rh == 0
        {
            return;
        }
        let planes: &[u8] = &region.data;
        let plane = rw * rh;
        // Level 0 stores straight alpha; every coarser level premultiplied.
        let straight = level == 0;
        for key in self.keys_in(source, level, rect) {
            let tile = self.tile_rect(key);
            let Some(entry) = self.tiles.get_mut(&key).filter(|e| e.stale) else {
                continue;
            };
            let tw = tile.width() as usize;
            let data = unique(&mut entry.data);
            for y in tile.top.max(rect.top)..tile.bottom.min(rect.bottom) {
                for x in tile.left.max(rect.left)..tile.right.min(rect.right) {
                    let s = (y - rect.top) as usize * rw + (x - rect.left) as usize;
                    let d = ((y - tile.top) as usize * tw + (x - tile.left) as usize) * 4;
                    let src = [0, 1, 2, 3].map(|c| planes[c * plane + s]);
                    let px: &mut [u8; 4] = (&mut data[d..d + 4]).try_into().expect("four bytes");
                    *px = if over {
                        over_pixel(src, opacity, *px, straight)
                    } else if straight {
                        src
                    } else {
                        premultiplied(src)
                    };
                }
            }
        }
    }

    /// `rect` grown to the alignment [`Self::repair`] needs: every level's
    /// 2×2 blocks under it whole, so each level of the region halves exactly.
    pub fn repair_rect(&self, rect: PsdRect) -> PsdRect {
        let step = 1i32 << (self.levels.saturating_sub(1)).min(30);
        let aligned = PsdRect {
            top: rect.top.div_euclid(step) * step,
            left: rect.left.div_euclid(step) * step,
            bottom: (rect.bottom + step - 1).div_euclid(step) * step,
            right: (rect.right + step - 1).div_euclid(step) * step,
        };
        clip(aligned, self.width, self.height)
    }

    /// Mark these keys as the presented view's: eviction skips them.
    pub fn pin(&mut self, keys: impl IntoIterator<Item = TileKey>) {
        self.pinned = keys.into_iter().collect();
        self.evict();
    }

    /// Drop every unpinned tile (high memory pressure).
    pub fn drop_unpinned(&mut self) {
        let before = self.tiles.len();
        let pinned = &self.pinned;
        self.tiles.retain(|key, _| pinned.contains(key));
        self.stats.evictions += (before - self.tiles.len()) as u64;
        self.recount();
    }

    /// `rect` of `level` (that level's pixels) premultiplied, from cached
    /// tiles. A missing tile is filled from its nearest cached coarser tile of
    /// the same source, nearest-neighbour, or left transparent; its key is
    /// returned so the caller can build it.
    pub fn crop(
        &mut self,
        source: TileSource,
        level: usize,
        rect: PsdRect,
    ) -> (PyramidLevel, Vec<TileKey>) {
        let (rw, rh) = (rect.width().max(0) as u32, rect.height().max(0) as u32);
        let mut out = PyramidLevel::new(rw, rh);
        let mut missing = Vec::new();
        if rw == 0 || rh == 0 || level >= self.levels {
            return (out, missing);
        }
        for key in self.keys_in(source, level, rect) {
            let tile = self.tile_rect(key);
            if let Some(data) = self.get(key) {
                paste(&mut out, rect, tile, &data, level == 0);
                continue;
            }
            missing.push(key);
            match self.tiles.get(&key) {
                Some(stale) => paste(&mut out, rect, tile, &stale.data, level == 0),
                None => self.stand_in(&mut out, rect, key),
            }
        }
        (out, missing)
    }

    /// Fill `key`'s part of `out` from the nearest cached coarser tile.
    fn stand_in(&mut self, out: &mut PyramidLevel, rect: PsdRect, key: TileKey) {
        let tile = self.tile_rect(key);
        let level = key.level as usize;
        for coarse in level + 1..self.levels {
            let shift = coarse - level;
            let parent = TileKey {
                level: coarse as u8,
                tx: key.tx >> shift,
                ty: key.ty >> shift,
                ..key
            };
            let Some(entry) = self.tiles.get(&parent) else {
                continue;
            };
            let parent_rect = self.tile_rect(parent);
            let pw = parent_rect.width() as usize;
            let ow = out.width() as usize;
            let top = tile.top.max(rect.top);
            let bottom = tile.bottom.min(rect.bottom);
            let left = tile.left.max(rect.left);
            let right = tile.right.min(rect.right);
            let dst = out.data_mut();
            for y in top..bottom {
                let py = ((y >> shift) - parent_rect.top) as usize;
                for x in left..right {
                    let px = ((x >> shift) - parent_rect.left) as usize;
                    let s = (py * pw + px) * 4;
                    let d = ((y - rect.top) as usize * ow + (x - rect.left) as usize) * 4;
                    dst[d..d + 4].copy_from_slice(&entry.data[s..s + 4]);
                }
            }
            return;
        }
    }

    fn damaged_since(&self, key: TileKey, clock: u64) -> bool {
        let footprint = scale_up(
            self.tile_rect(key),
            key.level as usize,
            self.width,
            self.height,
        );
        let (cols, _) = grid(self.width, self.height);
        let side = TILE_SIDE as i32;
        for ty in footprint.top / side..(footprint.bottom + side - 1) / side {
            for tx in footprint.left / side..(footprint.right + side - 1) / side {
                if self.damaged[ty as usize * cols + tx as usize] > clock {
                    return true;
                }
            }
        }
        false
    }

    fn stamp(&mut self, rect: PsdRect) {
        self.clock += 1;
        let (cols, _) = grid(self.width, self.height);
        let side = TILE_SIDE as i32;
        for ty in rect.top / side..(rect.bottom + side - 1) / side {
            for tx in rect.left / side..(rect.right + side - 1) / side {
                self.damaged[ty as usize * cols + tx as usize] = self.clock;
            }
        }
    }

    /// Write `rows` (interleaved, covering `rect` of `level`) into the cached
    /// tiles of that level it overlaps.
    fn patch_level(&mut self, source: TileSource, level: usize, rect: PsdRect, rows: &[u8]) {
        let rw = rect.width().max(0) as usize;
        for key in self.keys_in(source, level, rect) {
            let tile = self.tile_rect(key);
            let Some(entry) = self.tiles.get_mut(&key).filter(|e| !e.stale) else {
                continue;
            };
            let tw = tile.width() as usize;
            let data = unique(&mut entry.data);
            let top = tile.top.max(rect.top);
            let bottom = tile.bottom.min(rect.bottom);
            let left = tile.left.max(rect.left);
            let right = tile.right.min(rect.right);
            if right <= left {
                continue;
            }
            let len = (right - left) as usize * 4;
            for y in top..bottom {
                let s = ((y - rect.top) as usize * rw + (left - rect.left) as usize) * 4;
                let d = ((y - tile.top) as usize * tw + (left - tile.left) as usize) * 4;
                data[d..d + len].copy_from_slice(&rows[s..s + len]);
            }
        }
    }

    fn evict(&mut self) {
        if self.stats.resident_bytes <= self.budget {
            return;
        }
        let mut order: Vec<(bool, bool, u64, TileKey)> = self
            .tiles
            .iter()
            .filter(|(key, entry)| entry.stale || !self.pinned.contains(key))
            .map(|(key, entry)| (!entry.stale, key.level != 0, entry.used, *key))
            .collect();
        order.sort_unstable_by_key(|&(current, upper, used, _)| (current, upper, used));
        for (_, _, _, key) in order {
            if self.stats.resident_bytes <= self.budget {
                break;
            }
            if let Some(entry) = self.tiles.remove(&key) {
                self.stats.resident_bytes -= entry.data.len() as u64;
                self.stats.evictions += 1;
            }
        }
    }

    fn recount(&mut self) {
        self.stats.resident_bytes = self.tiles.values().map(|e| e.data.len() as u64).sum();
    }
}

/// Build `key`'s bytes from `source` (a `size.0`×`size.1` level-0 image), or
/// from its four cached children in `cache` when they are all there.
pub fn build_tile(
    cache: Option<&TilePyramid>,
    key: TileKey,
    rect: PsdRect,
    size: (u32, u32),
    source: Level0Source<'_>,
) -> Arc<[u8]> {
    let level = key.level as usize;
    if level == 0 {
        return interleave(&source(rect)).into();
    }
    if let Some(data) = cache.and_then(|cache| from_children(cache, key, rect, size)) {
        return data.into();
    }
    region_level(source, size, rect, level).data().into()
}

fn from_children(
    cache: &TilePyramid,
    key: TileKey,
    rect: PsdRect,
    size: (u32, u32),
) -> Option<Vec<u8>> {
    let child_level = key.level as usize - 1;
    let child_rect = PsdRect {
        top: rect.top * 2,
        left: rect.left * 2,
        bottom: (rect.bottom * 2).min(cache.level_size(child_level).1 as i32),
        right: (rect.right * 2).min(cache.level_size(child_level).0 as i32),
    };
    let (cw, ch) = (child_rect.width() as usize, child_rect.height() as usize);
    let mut joined = vec![0u8; cw * ch * 4];
    let keys = cache.keys_in(key.source, child_level, child_rect);
    for child in keys {
        let entry = cache.tiles.get(&child).filter(|e| !e.stale)?;
        let tile = cache.tile_rect(child);
        let mut data = entry.data.to_vec();
        if child_level == 0 {
            premultiply_interleaved(&mut data);
        }
        let tw = tile.width() as usize * 4;
        for y in tile.top..tile.bottom {
            let s = (y - tile.top) as usize * tw;
            let d =
                ((y - child_rect.top) as usize * cw + (tile.left - child_rect.left) as usize) * 4;
            joined[d..d + tw].copy_from_slice(&data[s..s + tw]);
        }
    }
    let (_, halved) = halve_level(
        &joined,
        (child_rect.left, child_rect.top),
        (cw as u32, ch as u32),
        size,
        child_level,
    );
    Some(halved)
}

/// Straight planar RGBA as straight interleaved RGBA.
pub fn interleave(buffer: &PixelBuffer) -> Vec<u8> {
    let plane = buffer.width as usize * buffer.height as usize;
    let mut out = vec![0u8; plane * 4];
    let mut row = vec![0u8; buffer.width as usize];
    let w = buffer.width as usize;
    for c in 0..4 {
        for y in 0..buffer.height as usize {
            buffer.data.read_into(c * plane + y * w, &mut row);
            for (x, v) in row.iter().enumerate() {
                out[(y * w + x) * 4 + c] = *v;
            }
        }
    }
    out
}

/// The tile's own bytes for writing, copied first when a crop or a worker
/// still holds them.
fn mul255(a: u32, b: u32) -> u32 {
    (a * b + 127) / 255
}

fn premultiplied([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    let a32 = u32::from(a);
    let p = |c: u8| mul255(u32::from(c), a32) as u8;
    [p(r), p(g), p(b), a]
}

/// Straight `src` at `opacity` over `dst`, which is stored straight when
/// `straight` and premultiplied otherwise; the result keeps `dst`'s storage.
fn over_pixel(src: [u8; 4], opacity: u8, dst: [u8; 4], straight: bool) -> [u8; 4] {
    let sa = mul255(u32::from(src[3]), u32::from(opacity));
    let dst = if straight { premultiplied(dst) } else { dst };
    let keep = 255 - sa;
    let mut out = [0u8; 4];
    for c in 0..3 {
        out[c] = (mul255(u32::from(src[c]), sa) + mul255(u32::from(dst[c]), keep)).min(255) as u8;
    }
    let oa = (sa + mul255(u32::from(dst[3]), keep)).min(255);
    out[3] = oa as u8;
    if straight && oa > 0 {
        for c in out.iter_mut().take(3) {
            *c = ((u32::from(*c) * 255 + oa / 2) / oa).min(255) as u8;
        }
    }
    out
}

fn unique(data: &mut Arc<[u8]>) -> &mut [u8] {
    if Arc::get_mut(data).is_none() {
        *data = Arc::from(&data[..]);
    }
    Arc::get_mut(data).expect("a fresh copy is unique")
}

fn premultiply_interleaved(data: &mut [u8]) {
    for px in data.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        for c in px.iter_mut().take(3) {
            *c = ((u32::from(*c) * a + 127) / 255) as u8;
        }
    }
}

/// Copy a tile into `out` (which covers `rect`), premultiplying a straight
/// level-0 tile on the way.
fn paste(out: &mut PyramidLevel, rect: PsdRect, tile: PsdRect, data: &[u8], straight: bool) {
    let top = tile.top.max(rect.top);
    let bottom = tile.bottom.min(rect.bottom);
    let left = tile.left.max(rect.left);
    let right = tile.right.min(rect.right);
    if right <= left || bottom <= top {
        return;
    }
    let tw = tile.width() as usize;
    let ow = out.width() as usize;
    let len = (right - left) as usize * 4;
    let dst = out.data_mut();
    for y in top..bottom {
        let s = ((y - tile.top) as usize * tw + (left - tile.left) as usize) * 4;
        let d = ((y - rect.top) as usize * ow + (left - rect.left) as usize) * 4;
        dst[d..d + len].copy_from_slice(&data[s..s + len]);
        if straight {
            premultiply_interleaved(&mut dst[d..d + len]);
        }
    }
}

fn grid(width: u32, height: u32) -> (usize, usize) {
    (
        width.div_ceil(TILE_SIDE) as usize,
        height.div_ceil(TILE_SIDE) as usize,
    )
}

fn clip(rect: PsdRect, width: u32, height: u32) -> PsdRect {
    let top = rect.top.max(0);
    let left = rect.left.max(0);
    let bottom = rect.bottom.min(height as i32);
    let right = rect.right.min(width as i32);
    if right <= left || bottom <= top {
        PsdRect::default()
    } else {
        PsdRect {
            top,
            left,
            bottom,
            right,
        }
    }
}

/// The level-`level` pixels a level-0 rectangle reaches.
fn scale_down(rect: PsdRect, level: usize) -> PsdRect {
    let step = 1i32 << level;
    PsdRect {
        top: rect.top.div_euclid(step),
        left: rect.left.div_euclid(step),
        bottom: (rect.bottom + step - 1).div_euclid(step),
        right: (rect.right + step - 1).div_euclid(step),
    }
}

/// The level-0 pixels a level-`level` rectangle stands for.
fn scale_up(rect: PsdRect, level: usize, width: u32, height: u32) -> PsdRect {
    clip(
        PsdRect {
            top: rect.top << level,
            left: rect.left << level,
            bottom: rect.bottom << level,
            right: rect.right << level,
        },
        width,
        height,
    )
}

#[cfg(test)]
#[path = "tests/tile_pyramid.rs"]
mod tests;
