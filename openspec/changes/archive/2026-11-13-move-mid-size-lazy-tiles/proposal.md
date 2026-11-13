# Keep the Move preview for mid-sized layered moves

## Why

On a large layered RGB document, releasing a Move whose dirty union was between
4 MP and 16 MP showed the layer at its old position for several seconds. The
display refresh leaves any region over its 4 MP repair budget stale, but the
"keep the preview until the tiles land" path only switched on at 16 MP. The
canvas dropped the preview onto stale tiles and waited for the tile worker.

## What Changes

- **Any Move on a large layered document waits for its presented tiles.** The
  lazy-tiles decision no longer has a dirty-area floor. It still needs a document
  of at least 16 MP and an idle display.
- **A move whose tiles stay current finishes at once.** When the refresh repaired
  every presented tile synchronously, the pending flag clears quietly and the
  committing caller reports `moveCommitted` as before, so it is not reported
  twice.

- **A second Move inherits a pending wait.** Releasing a Move before the
  previous Move's tiles landed settled the previous one first, and its
  `moveCommitted` dropped the new drag's preview onto stale tiles. The new commit
  now takes the wait over quietly; its own tile request covers every stale
  presented tile.
- **Stale tiles show the Move's result.** A Move or transform preview draws the
  presented tiles under a backdrop patch of its own layer only, so tiles an
  earlier Move left stale still showed its older position around it. A lazy
  Move with a view preview now paints that preview (the backdrop over the old
  place, the moving image at the new one) into the presented level's stale
  tiles as their stand-in. They stay stale, so the worker still rebuilds them
  exactly, and nothing drawn over them shows the old position. It costs no
  compositing: the preview already built those pixels.

- **A transform commit fixes its stale tiles too.** A transform has no view
  preview to reuse, so its commit composites a reduced copy of the changed area
  at the presented level into the stale tiles. The canvas used to keep the
  layer's untransformed image as its preview until the tiles rebuilt, showing
  the old size for seconds; with the stand-in painted, the commit reports at
  once. A Move without a view preview uses the same reduced stand-in.

## Capabilities

- **MODIFIED** `tools/move-commit-responsiveness`

## Impact

- `pictura-app` Rust: `impl_core/deferred_composite/imp.rs`, `tiles.rs`, `edit.rs`, `impl_transform/session.rs`,
  `impl_transform/session/view_preview.rs`, `impl_transform/session/preview.rs`,
  `state.rs`.
- `pictura-render`: `tile_pyramid.rs` (`paint_stand_in`).
- `pictura-app` tests: `deferred_composite/tests.rs`, `tst_large_document.cpp`.
