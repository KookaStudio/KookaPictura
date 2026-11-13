# Large Move commits leave display tiles lazy

## Why

On a large layered RGB document, releasing a Move drag still spawns a worker
that composites the whole dirty union. A canvas-sized layer on a 20000² document
makes that worker composite take seconds, and its CPU use makes the GUI choppy
after the release. The document keeps no stored composite: the display is a
tile pyramid whose visible tiles can be rebuilt from the moved layers. The
whole-area worker composite is duplicate work.

## What Changes

- **Only an authoritative composite defers.** A Move commit still takes the
  deferred worker path when the document keeps a stored composite (layerless,
  non-RGB, non-8-bit, or with source planes). A layered RGB 8-bit document marks
  its dirty display tiles stale instead and returns without a worker composite.
- **History captures immediately.** The stale stored frame is dropped before
  capture, so history records layer tiles rather than an old composite. One
  `Move Layer` state is captured when the commit returns.
- **Visible tiles rebuild.** The canvas rebuilds only the tiles it presents;
  off-screen tiles stay stale until panned into view.
- **The pending-state tests keep their seam.** A non-zero test delay still forces
  the deferred path, so the second-press and preview-survival cases remain
  covered.

## Capabilities

- **MODIFIED** `tools/move-commit-responsiveness`

## Impact

- `pictura-app` Rust: `impl_core/deferred_composite/imp.rs`,
  `impl_core/deferred_composite/tests.rs`, `tiles.rs`.
- `pictura-app` C++ tests: `tst_large_document.cpp`.
