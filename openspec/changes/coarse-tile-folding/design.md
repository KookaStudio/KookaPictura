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

## 3. Strips and side-by-side channels

Sampling the M12 open (gdb, after the folding above) showed one worker thread
in `decode_packbits` with the rayon pool idle. A fit-level tile reads a
4096-wide footprint, but the merged preview decodes whole 17000-wide bands
into a 32 MiB LRU that holds about two band rows, so each band was decoded
once per tile column of the row, about four times, channel after channel.

- `build_keys` builds a run of adjacent keys of one level and row with no
  finer tile under them as one strip (`region_level` over the run's
  rectangle, then a copy per tile). The level-0 source is asked for whole
  rows, so each band decodes about once. The bytes are the per-tile build's:
  `region_level` is exact for any aligned rectangle.
- `MergedPreview::region` decodes a band's channels side by side
  (`par_iter`). It holds the same bands as before, one per channel; the
  every-core band decode #1218 dropped held a chunk's bands at once.

## 4. Cheaper halving

`Region::halved` sums a block without bounds checks when the whole 2×2 block
lies inside the level and the region; the edge pixels keep the general loop.
Same rounding, same bytes. (The display sources' `into_owned` was suspected
too, but a `PixelBuffer` clone shares its `Arc` storage: no copy to save.)

## 5. Heap trim after a tile job

A whole-row strip's chunk buffers (about 31 MiB at 17000 wide) sit just under
glibc's largest mmap threshold, so they come from the thread arenas, which
keep them after they are freed. A tile job ends with
`pictura_core::memory::eviction::trim` (`malloc_trim`, already run after each
eviction pass), on the worker. It brings the resident size after the open
down about 40 MiB but not the peak (owner decision, 2026-11-18: ship the
strips with the trim; the retention is a follow-up). Chunks one band tall
kept the peak at master's but took 4.4 s; rounding chunk rows up did not
help.

## 6. Measurement

M12 at 17000² (`scripts/profile-tools.py --scenarios M12 --peak`), master
`bb911b80` and this branch built side by side, runs alternated, load average
about 10 from other work; medians of three:

| Build | Total | Of eager 9433 ms | Peak | RSS after |
|---|---|---|---|---|
| master | 3837 ms | 41 % | 647 MiB | 588 MiB |
| folding and halving only | ~3650 ms | ~39 % | ~637 MiB | ~same |
| this change | 2263 ms | 24 % | 724 MiB | 677 MiB |

The app's CPU time over the open fell from 11.0 s to 5.5 s (`/proc` stat,
same runs' setup). The 25 % target is met; peak memory is 77 MiB above
master's (glibc arenas, §5) and under half the eager open's 1540 MiB.
