//! Building one tile of the pyramid: from its level-0 source, or folded from
//! the finer tiles at hand (#1216, #1321).
//!
//! A fold equals the direct build byte for byte: by the reduced-level rule
//! (`region_level.rs`), every 2×2 block a fold sees is the block a whole
//! rebuild sees.

use crate::region_level::{halve_level, region_level, Level0Source};
use crate::tile_pyramid::{premultiply_interleaved, TileKey, TilePyramid};
use pictura_core::PsdRect;
use std::sync::Arc;

/// Build `key`'s bytes from `source` (a `size.0`×`size.1` level-0 image), or
/// from its four cached children in `cache` when they are all there.
pub(crate) fn build_tile(
    cache: Option<&TilePyramid>,
    key: TileKey,
    rect: PsdRect,
    size: (u32, u32),
    source: Level0Source<'_>,
) -> Arc<[u8]> {
    let level = key.level as usize;
    if level == 0 {
        return crate::tile_pyramid::interleave(&source(rect)).into();
    }
    if let Some(data) = cache.and_then(|cache| from_children(cache, key, rect, size)) {
        return data.into();
    }
    region_level(source, size, rect, level).data().into()
}

/// Build `keys` of one source, finer levels first, so a coarser key folds
/// from the finer ones built before it. Returns every tile built, the levels
/// folded through included, each once.
pub fn build_keys(
    local: &mut TilePyramid,
    keys: &[TileKey],
    source: Level0Source<'_>,
) -> Vec<(TileKey, Arc<[u8]>)> {
    let mut keys = keys.to_vec();
    keys.sort_by_key(|key| key.level);
    let mut built = Vec::new();
    for key in keys {
        build_folded(local, key, source, &mut built);
    }
    built
}

/// `key`'s bytes, folded one level at a time from the current finer tiles of
/// `local` that lie under it, and built from `source` only where none does.
/// Every tile built on the way, `key`'s included, goes into `local` and onto
/// `built`; a tile already current in `local` is returned as it is.
pub(crate) fn build_folded(
    local: &mut TilePyramid,
    key: TileKey,
    source: Level0Source<'_>,
    built: &mut Vec<(TileKey, Arc<[u8]>)>,
) -> Arc<[u8]> {
    if let Some(data) = local.current(key) {
        return Arc::clone(data);
    }
    let rect = local.tile_rect(key);
    let size = local.size();
    let data = if key.level == 0 || !local.any_finer_under(key) {
        build_tile(None, key, rect, size, source)
    } else {
        let child_level = key.level as usize - 1;
        let children = local.keys_in(
            key.source,
            child_level,
            child_rect(local, rect, child_level),
        );
        for child in children {
            build_folded(local, child, source, built);
        }
        from_children(local, key, rect, size)
            .expect("every child is current")
            .into()
    };
    local.insert(key, Arc::clone(&data));
    built.push((key, Arc::clone(&data)));
    data
}

/// The rectangle of `child_level` a tile `rect` one level up covers.
fn child_rect(cache: &TilePyramid, rect: PsdRect, child_level: usize) -> PsdRect {
    let (cw, ch) = cache.level_size(child_level);
    PsdRect {
        top: rect.top * 2,
        left: rect.left * 2,
        bottom: (rect.bottom * 2).min(ch as i32),
        right: (rect.right * 2).min(cw as i32),
    }
}

fn from_children(
    cache: &TilePyramid,
    key: TileKey,
    rect: PsdRect,
    size: (u32, u32),
) -> Option<Vec<u8>> {
    let child_level = key.level as usize - 1;
    let child_rect = child_rect(cache, rect, child_level);
    let (cw, ch) = (child_rect.width() as usize, child_rect.height() as usize);
    let mut joined = vec![0u8; cw * ch * 4];
    let keys = cache.keys_in(key.source, child_level, child_rect);
    for child in keys {
        let data = cache.current(child)?;
        let tile = cache.tile_rect(child);
        let mut data = data.to_vec();
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

#[cfg(test)]
#[path = "tests/tile_build.rs"]
mod tests;
