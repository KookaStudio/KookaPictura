# Design

## 1. The builder (`pictura-render`, `tile_build.rs`)

`build_tile` and `from_children` move out of `tile_pyramid.rs` into
`tile_build.rs` (a pure move), beside the new entry point:

```rust
pub fn build_folded(local: &mut TilePyramid, key, size, source,
                    built: &mut Vec<(TileKey, Arc<[u8]>)>) -> Arc<[u8]>
```

`local` is a job-local pyramid with no budget limit. For `key`:

1. A current tile in `local` is returned as is.
2. Level 0, or no current finer tile of the same source lies under the key's
   footprint (`TilePyramid::any_finer_under`): `build_tile(None, …)`, the
   level-0 pass as today.
3. Otherwise each child at the next finer level is built by the same rule,
   then the key folds from its children (`from_children`).

Each built tile goes into `local` and onto `built`. Rule 3 only descends where
a finer tile exists, so a key with nothing under it costs what it costs today,
and a key whose finer level is complete costs a fold per level. Folding equals
the direct build byte for byte: the region-level rule makes every 2×2 block a
fold sees the block a whole rebuild sees (`region_level.rs` module notes).

`build_keys(local, keys, source)` sorts one source's keys finer first and
builds each, so a coarser key in the same build folds from the finer ones.

`TilePyramid::snapshot(below)` copies the current tiles of levels under
`below` into a new pyramid of the same size with an unlimited budget: `Arc`
clones, no pixels.

## 2. The app (`tiles.rs`)

- `build_one` becomes `build_keys` over the job's keys, one call of the
  render crate's `build_keys` per tile source, with the source closure chosen
  as before (mask, merged preview, or composite).
- `TileJob` carries `local: TilePyramid`, the snapshot taken with the ticket,
  so the tiles it folds from were current at that ticket and `install` refuses
  a result whose footprint damage overtook. `build` returns every tile it built, intermediate ones included.
- `build_now` snapshots the cache, builds the same way, and inserts what it
  built.

At fit, the canvas asks for the fit level's tiles; the Navigator's coarsest
level comes in the next job and folds from them through the levels between.
If the coarsest level is asked for first, it is built from level 0 as today,
and the fit level after it as well.

## 3. Cheaper halving

`Region::halved` sums a block without bounds checks when the whole 2×2 block
lies inside the level and the region; the edge pixels keep the general loop.
Same rounding, same bytes. (The display sources' `into_owned` was suspected
too, but a `PixelBuffer` clone shares its `Arc` storage: no copy to save.)

## 4. Measurement

M12 at 17000² (`scripts/memory_scenarios.py`), first fit-to-screen picture,
before and after on the same binary. The 25 % target is best effort (owner,
2026-11-18); the result goes in the PR and `docs/dev/STATE.md`.
