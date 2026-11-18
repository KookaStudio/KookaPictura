//! Folding coarse tiles from the finer tiles at hand (#1321).

use super::*;
use crate::composite_region;
use crate::region_level::tests::{seeded_doc, H, W};
use crate::TileSource;
use pictura_core::{Document, PixelBuffer, PsdRect};
use std::sync::Mutex;

/// The composite of `doc` as a level-0 source that records every rectangle
/// it is asked for.
fn counted<'a>(
    doc: &'a Document,
    asked: &'a Mutex<Vec<PsdRect>>,
) -> impl Fn(PsdRect) -> PixelBuffer + Sync + 'a {
    move |r| {
        asked.lock().expect("unpoisoned").push(r);
        composite_region(doc, r)
    }
}

fn direct(doc: &Document, cache: &TilePyramid, key: TileKey) -> Arc<[u8]> {
    let source = |r: PsdRect| composite_region(doc, r);
    build_tile(None, key, cache.tile_rect(key), (W, H), &source)
}

fn level_keys(cache: &TilePyramid, level: usize) -> Vec<TileKey> {
    let (lw, lh) = cache.level_size(level);
    let whole = PsdRect {
        top: 0,
        left: 0,
        bottom: lh as i32,
        right: lw as i32,
    };
    cache.keys_in(TileSource::Composite, level, whole)
}

fn cache_level(cache: &mut TilePyramid, doc: &Document, keys: &[TileKey]) {
    for &key in keys {
        let data = direct(doc, cache, key);
        cache.insert(key, data);
    }
}

// [ctf_fold_cached]
#[test]
fn the_coarsest_level_folds_from_a_cached_finer_level() {
    let doc = seeded_doc(21);
    let mut local = TilePyramid::new(W, H, u64::MAX);
    assert_eq!(local.level_count(), 4, "levels 0 to 3");
    let fine = level_keys(&local, 1);
    cache_level(&mut local, &doc, &fine);
    let asked = Mutex::new(Vec::new());
    let source = counted(&doc, &asked);
    let mut built = Vec::new();
    for key in level_keys(&local, 3) {
        let data = build_folded(&mut local, key, &source, &mut built);
        assert_eq!(data, direct(&doc, &local, key), "{key:?}");
    }
    assert!(asked.lock().expect("unpoisoned").is_empty(), "no level-0 read");
    for (key, data) in &built {
        assert_eq!(*data, direct(&doc, &local, *key), "{key:?}");
    }
    let between = built.iter().filter(|(key, _)| key.level == 2).count();
    assert_eq!(between, level_keys(&local, 2).len(), "level 2 folded through");
}

// [ctf_partial]
#[test]
fn only_uncovered_parts_read_level_zero() {
    let doc = seeded_doc(22);
    let mut local = TilePyramid::new(W, H, u64::MAX);
    // Level-3 tile (0, 0) stands for level-0 columns 0..2048; the cached
    // level-1 tiles with tx < 2 cover columns 0..1024 of it.
    let left: Vec<TileKey> = level_keys(&local, 1)
        .into_iter()
        .filter(|key| key.tx < 2)
        .collect();
    cache_level(&mut local, &doc, &left);
    let key = TileKey {
        source: TileSource::Composite,
        level: 3,
        tx: 0,
        ty: 0,
    };
    let asked = Mutex::new(Vec::new());
    let mut built = Vec::new();
    let data = build_folded(&mut local, key, &counted(&doc, &asked), &mut built);
    assert_eq!(data, direct(&doc, &local, key));
    let asked = asked.into_inner().expect("unpoisoned");
    assert!(!asked.is_empty(), "the uncovered half is composited");
    assert!(
        asked.iter().all(|r| r.left >= 1024 && r.right <= 2048),
        "only the uncovered columns are read: {asked:?}"
    );
}

// [ctf_same_job]
#[test]
fn one_build_folds_coarser_keys_from_finer_ones() {
    let doc = seeded_doc(23);
    let mut local = TilePyramid::new(W, H, u64::MAX);
    let fine = level_keys(&local, 1);
    // The coarse keys first: the build orders them.
    let mut keys = level_keys(&local, 3);
    keys.extend(&fine);
    let asked = Mutex::new(Vec::new());
    let built = build_keys(&mut local, &keys, &counted(&doc, &asked));
    for (key, data) in &built {
        assert_eq!(*data, direct(&doc, &local, *key), "{key:?}");
    }
    for key in &keys {
        assert!(built.iter().any(|(k, _)| k == key), "{key:?} built");
    }
    let read: i64 = asked
        .into_inner()
        .expect("unpoisoned")
        .iter()
        .map(|r| i64::from(r.width()) * i64::from(r.height()))
        .sum();
    assert_eq!(
        read,
        i64::from(W) * i64::from(H),
        "level 0 is read once, for the level-1 tiles"
    );
}

// [ctf_stale]
#[test]
fn stale_tiles_are_not_folded_from() {
    let mut doc = seeded_doc(24);
    let mut cache = TilePyramid::new(W, H, u64::MAX);
    let fine = level_keys(&cache, 1);
    cache_level(&mut cache, &doc, &fine);
    let top = doc.layers.iter_mut().last().expect("a top layer");
    for channel in top.channels_mut().iter_mut().take(3) {
        for v in channel.data.as_mut_slice() {
            *v = v.wrapping_add(53);
        }
    }
    cache.damage(
        TileSource::Composite,
        PsdRect {
            top: 0,
            left: 0,
            bottom: H as i32,
            right: W as i32,
        },
    );
    let mut local = cache.snapshot(cache.level_count());
    let key = TileKey {
        source: TileSource::Composite,
        level: 3,
        tx: 1,
        ty: 0,
    };
    let asked = Mutex::new(Vec::new());
    let mut built = Vec::new();
    let data = build_folded(&mut local, key, &counted(&doc, &asked), &mut built);
    assert!(!asked.into_inner().expect("unpoisoned").is_empty());
    assert_eq!(data, direct(&doc, &local, key), "built from the edited document");
}
