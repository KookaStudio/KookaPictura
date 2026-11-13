//! The tile pyramid cache: repair, staleness, budget, pressure, counters.

use super::*;
use crate::composite_region;
use crate::region_level::tests::{seeded_doc, H, W};
use pictura_core::{Document, PsdRect};

const TILE_BYTES: u64 = (TILE_SIDE * TILE_SIDE * 4) as u64;

fn source(doc: &Document) -> impl Fn(PsdRect) -> PixelBuffer + Sync + '_ {
    move |r| composite_region(doc, r)
}

fn whole(level_w: u32, level_h: u32) -> PsdRect {
    PsdRect {
        top: 0,
        left: 0,
        bottom: level_h as i32,
        right: level_w as i32,
    }
}

/// Build and cache every composite tile of levels `0..levels`.
fn fill(cache: &mut TilePyramid, doc: &Document, levels: usize) {
    let src = source(doc);
    for level in 0..levels {
        let (lw, lh) = cache.level_size(level);
        for key in cache.keys_in(TileSource::Composite, level, whole(lw, lh)) {
            let rect = cache.tile_rect(key);
            let data = build_tile(None, key, rect, (W, H), &src);
            cache.insert(key, data);
        }
    }
}

fn edit(doc: &mut Document, rect: PsdRect) {
    let layer = doc.layers.iter_mut().last().expect("a top layer");
    let lr = layer.rect;
    let w = lr.width() as usize;
    for channel in layer.channels.iter_mut().take(3) {
        let data = channel.data.as_mut_slice();
        for y in rect.top.max(lr.top)..rect.bottom.min(lr.bottom) {
            for x in rect.left.max(lr.left)..rect.right.min(lr.right) {
                let i = (y - lr.top) as usize * w + (x - lr.left) as usize;
                data[i] = data[i].wrapping_add(97);
            }
        }
    }
}

#[test]
fn damage_repairs_cached_tiles() {
    let mut doc = seeded_doc(9);
    let mut cache = TilePyramid::new(W, H, u64::MAX);
    fill(&mut cache, &doc, 3);
    let top = doc.layers.iter().last().expect("top").rect;
    let dirty = PsdRect {
        top: top.top.max(0) + 3,
        left: top.left.max(0) + 5,
        bottom: top.top.max(0) + 40,
        right: top.left.max(0) + 70,
    };
    let reach = cache.repair_rect(dirty);
    let far = cache
        .keys_in(TileSource::Composite, 0, whole(W, H))
        .into_iter()
        .find(|&key| {
            let t = cache.tile_rect(key);
            t.right <= reach.left - 2
                || t.left >= reach.right + 2
                || t.bottom <= reach.top - 2
                || t.top >= reach.bottom + 2
        })
        .expect("a tile away from the edit");
    let far_before = cache.get(far).expect("cached");

    edit(&mut doc, dirty);
    let aligned = cache.repair_rect(dirty);
    cache.repair(
        TileSource::Composite,
        aligned,
        &composite_region(&doc, aligned),
    );

    let mut fresh = TilePyramid::new(W, H, u64::MAX);
    fill(&mut fresh, &doc, 3);
    for level in 0..3 {
        let (lw, lh) = cache.level_size(level);
        for key in cache.keys_in(TileSource::Composite, level, whole(lw, lh)) {
            assert_eq!(
                cache.get(key).as_deref(),
                fresh.get(key).as_deref(),
                "{key:?} equals a rebuild after the repair"
            );
        }
    }
    let far_after = cache.get(far).expect("still cached");
    assert!(
        Arc::ptr_eq(&far_before, &far_after),
        "a tile outside the damage keeps its allocation"
    );
}

#[test]
fn a_stale_tile_is_not_installed() {
    let doc = seeded_doc(2);
    let mut cache = TilePyramid::new(W, H, u64::MAX);
    let key = TileKey {
        source: TileSource::Composite,
        level: 1,
        tx: 0,
        ty: 0,
    };
    let rect = cache.tile_rect(key);
    let ticket = cache.ticket();
    let data = build_tile(None, key, rect, (W, H), &source(&doc));
    cache.damage(
        TileSource::Composite,
        PsdRect {
            top: 10,
            left: 10,
            bottom: 20,
            right: 20,
        },
    );
    assert!(!cache.install(key, ticket, Arc::clone(&data)), "superseded");
    assert!(!cache.contains(key));

    let far = TileKey {
        level: 0,
        tx: 3,
        ty: 2,
        ..key
    };
    let far_data = build_tile(None, far, cache.tile_rect(far), (W, H), &source(&doc));
    assert!(
        cache.install(far, ticket, far_data),
        "damage elsewhere does not refuse a tile"
    );
    let fresh = cache.ticket();
    assert!(cache.install(key, fresh, data), "a current ticket installs");
    cache.invalidate();
    assert!(
        !cache.install(far, fresh, Arc::from(vec![0u8; 4])),
        "a new epoch refuses"
    );
}

#[test]
fn the_cache_keeps_to_its_budget() {
    let mut cache = TilePyramid::new(4096, 4096, 6 * TILE_BYTES);
    let tile = || -> Arc<[u8]> { Arc::from(vec![1u8; TILE_BYTES as usize]) };
    for ty in 0..2 {
        for tx in 0..2 {
            cache.insert(
                TileKey {
                    source: TileSource::Composite,
                    level: 2,
                    tx,
                    ty,
                },
                tile(),
            );
        }
    }
    for i in 0..16 {
        cache.insert(
            TileKey {
                source: TileSource::Composite,
                level: 0,
                tx: i,
                ty: 0,
            },
            tile(),
        );
        assert!(
            cache.stats().resident_bytes <= 6 * TILE_BYTES,
            "within budget"
        );
    }
    for ty in 0..2 {
        for tx in 0..2 {
            assert!(
                cache.contains(TileKey {
                    source: TileSource::Composite,
                    level: 2,
                    tx,
                    ty,
                }),
                "level-0 tiles are evicted before other levels"
            );
        }
    }
    assert!(cache.stats().evictions >= 14);
    assert!(cache.contains(TileKey {
        source: TileSource::Composite,
        level: 0,
        tx: 15,
        ty: 0,
    }));
}

#[test]
fn pressure_drops_unpinned_tiles() {
    let mut cache = TilePyramid::new(4096, 4096, u64::MAX);
    let key = |tx| TileKey {
        source: TileSource::Composite,
        level: 0,
        tx,
        ty: 0,
    };
    for tx in 0..8 {
        cache.insert(key(tx), Arc::from(vec![0u8; 16]));
    }
    cache.pin([key(1), key(2)]);
    cache.drop_unpinned();
    assert_eq!(cache.stats().tiles, 2);
    assert!(cache.contains(key(1)) && cache.contains(key(2)));
    assert_eq!(cache.stats().resident_bytes, 32);
}

#[test]
fn hits_and_misses_are_counted() {
    let doc = seeded_doc(4);
    let mut cache = TilePyramid::new(W, H, u64::MAX);
    let view = PsdRect {
        top: 100,
        left: 200,
        bottom: 400,
        right: 700,
    };
    let (_, missing) = cache.crop(TileSource::Composite, 0, view);
    assert!(!missing.is_empty());
    let first = cache.stats();
    assert_eq!(first.misses as usize, missing.len());
    for key in missing {
        let data = build_tile(None, key, cache.tile_rect(key), (W, H), &source(&doc));
        cache.insert(key, data);
    }
    let (crop, missing) = cache.crop(TileSource::Composite, 0, view);
    assert!(missing.is_empty(), "the second presentation is all hits");
    let second = cache.stats();
    assert_eq!(second.misses, first.misses);
    assert_eq!(
        second.hits - first.hits,
        cache.keys_in(TileSource::Composite, 0, view).len() as u64
    );
    let expected = crate::premultiply_planar(&composite_region(&doc, view));
    assert_eq!(crop.data(), &expected[..], "the assembled crop is exact");
}

#[test]
fn a_parent_from_cached_children_equals_a_composite() {
    let doc = seeded_doc(6);
    let mut cache = TilePyramid::new(W, H, u64::MAX);
    fill(&mut cache, &doc, 2);
    let key = TileKey {
        source: TileSource::Composite,
        level: 2,
        tx: 0,
        ty: 0,
    };
    let rect = cache.tile_rect(key);
    let from_children = build_tile(Some(&cache), key, rect, (W, H), &source(&doc));
    let direct = build_tile(None, key, rect, (W, H), &source(&doc));
    assert_eq!(from_children, direct);
}

#[test]
fn a_missing_tile_stands_in_from_a_coarser_one() {
    let doc = seeded_doc(8);
    let mut cache = TilePyramid::new(W, H, u64::MAX);
    let coarse = TileKey {
        source: TileSource::Composite,
        level: 1,
        tx: 0,
        ty: 0,
    };
    let data = build_tile(None, coarse, cache.tile_rect(coarse), (W, H), &source(&doc));
    cache.insert(coarse, Arc::clone(&data));
    let view = PsdRect {
        top: 0,
        left: 0,
        bottom: 4,
        right: 4,
    };
    let (crop, missing) = cache.crop(TileSource::Composite, 0, view);
    assert_eq!(missing.len(), 1);
    assert_eq!(
        &crop.data()[0..4],
        &data[0..4],
        "pixel (0,0) from its parent"
    );
    assert_eq!(
        &crop.data()[4..8],
        &data[0..4],
        "pixel (1,0) from the same parent texel"
    );
}

#[test]
fn level_count_matches_the_view_pyramid() {
    for (w, h) in [
        (1500, 700),
        (511, 300),
        (300, 200),
        (997, 613),
        (2049, 1025),
    ] {
        let n = w as usize * h as usize;
        let planes = vec![0u8; n];
        let pyramid = crate::ViewPyramid::rebuild(crate::Planes::new(
            w, h, &planes, &planes, &planes, &planes,
        ));
        assert_eq!(level_count(w, h), pyramid.level_count(), "{w}x{h}");
        if n > 4_000_000 {
            break;
        }
    }
}

// [mcr_lazy_chain]
#[test]
fn a_stand_in_paints_only_stale_tiles() {
    let mut cache = TilePyramid::new(1024, 512, u64::MAX);
    let side = TILE_SIDE as usize;
    let gray = |v: u8| -> Arc<[u8]> { vec![v; side * side * 4].into() };
    let key = |level: u8, tx: u32| TileKey {
        source: TileSource::Composite,
        level,
        tx,
        ty: 0,
    };
    // Level 1 is premultiplied; an opaque gray is the same either way.
    cache.insert(key(1, 0), gray(200));
    cache.insert(key(1, 1), gray(200));
    cache.damage(
        TileSource::Composite,
        PsdRect {
            top: 0,
            left: 0,
            bottom: 2 * TILE_SIDE as i32,
            right: 2 * TILE_SIDE as i32,
        },
    );
    assert!(!cache.contains(key(1, 0)) && cache.contains(key(1, 1)));

    // A 2x1 region straddling the tiles: opaque red replaces, then half-opaque
    // blue goes over it.
    let edge = TILE_SIDE as i32 - 1;
    let rect = PsdRect {
        top: 0,
        left: edge,
        bottom: 1,
        right: edge + 2,
    };
    let red = PixelBuffer {
        width: 2,
        height: 1,
        channels: 4,
        data: vec![255, 255, 0, 0, 0, 0, 255, 255].into(),
    };
    cache.paint_stand_in(TileSource::Composite, 1, rect, &red, 255, false);
    let blue = PixelBuffer {
        width: 2,
        height: 1,
        channels: 4,
        data: vec![0, 0, 0, 0, 255, 255, 255, 255].into(),
    };
    cache.paint_stand_in(TileSource::Composite, 1, rect, &blue, 128, true);

    let stale = &cache.tiles[&key(1, 0)];
    assert!(stale.stale, "a stand-in leaves the tile stale");
    let at = (edge as usize) * 4;
    assert_eq!(&stale.data[at..at + 4], &[127, 0, 128, 255]);
    let current = &cache.tiles[&key(1, 1)];
    assert_eq!(
        &current.data[0..4],
        &[200, 200, 200, 200],
        "a current tile is untouched"
    );
}
