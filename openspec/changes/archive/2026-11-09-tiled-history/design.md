# Design

## Context

Since #1213 every tracked plane is cut into 256² tiles, each `Empty`,
`Uniform(v)` or a shared `Dense` allocation, and a write forks only the tiles
it touches. History already took advantage of that for `current`, the
materialized state at the cursor: it shares every tile the live document has
not written. Three parts still copied pixels or counted them wrongly:

- **Deltas.** A delta copied the 64×64 regions that changed into owned
  `Vec<u8>`s, and an undo swapped bytes back in.
- **Named snapshots.** Taking one and restoring one both deep-copied every
  plane (`detached_copy`). The copy existed so that the next dab would not fork
  a whole plane. That reason belongs to flat planes and no longer holds.
- **The budget.** `held_bytes` walked every state, removed duplicate plane
  allocations, and then added every delta's byte vectors on top without
  removing duplicates.

## Owner decisions (2026-11-08)

- **Deltas hold 256² tile references.** This follows the issue's R1 as written.
  A small edit now keeps its whole tile, 64 KiB per channel where a 64×64 copy
  was 4 KiB. The unique-tile budget absorbs that cost by dropping old states
  sooner.
- **Purge: spec and test only.** Purge already exists, but it acts on the active
  document only, and Purge › Histories drops only the named snapshots. CS6
  instead purges the states of every open document. That difference is left to
  a follow-up issue.
- **M8's refresh is out of scope.** M8 is measured on history and document
  bytes. The restore still rebuilds the display from the snapshot's composite,
  about 12 bytes a pixel, and that belongs to #1216 (the viewport pyramid), as
  M2 did for #1213 and M5 for #1214.
- **The budget moves into the governor.** `pictura_core::memory` owns the
  history share (`HISTORY_SHARE`, `Governor::history_budget`) and the counting
  structure (`TileCensus`). `history/budget.rs` keeps only the drop loop.

## Decisions

### A tile held outside its plane

`SharedTile<T>` wraps the crate-private `Tile<T>` and its generation.
`Plane::shared_tile(i)` clones one, which costs one `Arc` bump.
`Plane::swap_tile(i, &mut held)` exchanges tile and generation, clears the
plane's stamp, its ident and its contiguous view, and copies nothing. Because
the generation travels with the allocation, a tile swapped back is again
`shares_tile`-equal to every plane that still holds it. A generation therefore
still names one content.

### Deltas

`diff_into` keeps its checks for a segment break: plane count, stride and
height, a stamp seen at another index, and, new here, a changed plane that
cannot be tiled. For each plane pair that is not known to be equal, it
compares only the tiles the two do not share, one per core. When a hint is
given, only the tiles the hint meets are compared. For each compared tile,
`current` adopts `next`'s tile. A tile that differs moves its old reference
into the delta.

The hinted and unhinted paths are now one path. The only difference is that a
hint skips the tiles it misses. A hint the caller violates inside a tile it
does meet is now captured in full, because the whole tile is adopted. The old
`hinted_capture_skips_outside_tiles` test is moved to a tile the hint misses.

`swap_tiles` exchanges references. `apply_tiles`, used when a state is rebuilt
away from the cursor, installs clones of the references. The comparison that
finds a changed tile also records the box of its changed samples, and that box,
not the whole tile, is the delta's damage. An undo of a small edit therefore
still redraws only what changed.

### Snapshots and restore

`named_snapshot` clones the live document. Mid-stroke it still composites the
painted layers into the clone, so the snapshot's composite is exact.
`history_restore_snapshot` installs the snapshot's document, records the
state, and presents the stored composite with `reset_pyramid`. It no longer
composites the layers again, unless the composite does not match the canvas.
Plane stamps travel with the clone, and a stamp still promises equal bytes,
so the restore's capture skips every plane the edit since the snapshot did not
touch.

`detached_copy` and `copy_plane` are gone. The memory gate's snapshot estimate
is the tile tables, about 64 bytes a tile, plus the stroke composite when a
stroke is in progress. The restore estimate is the tile tables plus the
display rebuild.

### The tile census

`TileCensus` keeps a refcount for each allocation address on two sides:

- **`Live`**: the tiles of `current`. A capture or a restore leaves the live
  document sharing them.
- **`Kept`**: delta tiles, full anchors, named snapshots and the pinned History
  Brush source.

It maintains `beyond_live`, the bytes of the allocations that are kept and not
live, each counted once. Each update is O(1) per moved tile:

- A capture moves the replaced tiles from `Live` to `Kept` and the adopted
  tiles into `Live`.
- A swap exchanges one tile on each side.
- Only a segment crossing re-counts a whole document. That happens when
  `fill_anchor` or `take_anchor` turns a materialized state into a full anchor
  or back, and it reads the tile tables, not the pixels.

Every path that changes what history holds goes through one helper:
`replace_stored`, `truncate_states`, `set_pinned`, `clear_snapshots`, the
snapshot eviction, `fill_anchor` or `take_anchor`.

Under `cfg(test)`, `check_census` compares the census with a full recount after
every capture, budget pass and walk, so every history test also checks the
bookkeeping. An address cannot be reused under a stale entry, because a
counted allocation is always still alive: history holds it.

ponytail: the live side is `current`, not the live document itself. The two
share every tile after a capture or a restore, which is when the budget is
checked. A caller that violates its hint leaves the live document with a tile
history does not see. That bound is approximate and does not affect
correctness.

### Budget

`enforce_byte_cap` drops the oldest state while `beyond_live` exceeds
`governor().history_budget()`, half the ceiling as before. It always keeps the
current state and one undo, and it never touches named snapshots.

## Risks

- **Larger deltas for tiny edits.** A one-pixel edit keeps a whole 256² tile
  instead of a 64×64 one. This is accepted. `region_deltas_retain_only_changed_tiles`
  now runs on a 1024² document, so ten such tiles still stay below one state.
- **Damage scan cost.** A changed tile is scanned to the end to find its
  changed box, where an equality check would stop at the first difference. The
  scan reads each changed tile once, in place.

## Results

In-process heap guards, measured on 2026-11-08 with 4 rayon threads under nextest:

| Scenario | Before | After |
|---|---|---|
| M8 at 4250² (snapshot, restore, display rebuild) | 93.6 MiB (baseline) | 22.0 MiB; baseline lowered to 23 102 429 B |
| M8 at 17000², whole | refused under 8 GiB (#1177 audit) | 361.7 MiB, nearly all of it the display rebuild (#1216) |
| M8 at 17000², snapshot, restore and record only | n/a | 2.99 MiB in 1.4 ms |
| M7 at 4250² (undo ×3, redo ×3) | 137.7 MiB (baseline) | 138.2 MiB, within the guard; that is the display refresh |
| M7 at 17000² | refused under 8 GiB | 2207 MiB peak; history adds no tile copies, and the rest is the display refresh |

Every M7 and M8 run materializes no plane.

App runs with `scripts/profile-tools.py --peak`, 2026-11-09:

| Run | Outcome | Peak added |
|---|---|---|
| M7 at 17000² (11.5 GiB scope) | ok; was refused under 8 GiB | 2206 MiB, the display refresh |
| M8 at 17000² (11.5 GiB scope) | ok in 1.29 s; was refused under 8 GiB | 359 MiB, the display rebuild (#1216) |
| `M6x20` at 17000², `--memory-max 4G` | refused at setup | New Document's gate charges about 20 B a pixel, mostly display (#1216), before any stroke |
| `M6x20` at 12000², `--memory-max 4G` | ok, not OOM-killed | 78 MiB over twenty strokes |

ponytail: the issue's "twenty M6 strokes at 17000² under 4G" cannot start
until #1216 shrinks the display a new document holds. History's part of that
run is the 12000² row.
