# Proposal: coarse tile folding (#1321)

## Why

Since #1218 a lazily opened layered RGB PSD paints its first picture from the
file's merged image, but the first fit-to-screen picture still takes 38 % of
the eager open (M12 at 17000²: 3598 ms against 9433 ms; the target was 25 %).
The tile worker builds every coarse tile from its whole level-0 footprint
(`build_tile(None, …)`), and the coarsest level, which the Navigator and the
Histogram read, runs the same level-0 pass again. The synchronous path already
folds a tile from its four cached children; the worker never does.

## What changes

- One tile builder for the worker and the synchronous path: a missing coarse
  tile is folded from the finer tiles already cached or built in the same job,
  through as many levels as needed, and only the parts with no finer tile
  under them read the level-0 source.
- The worker gets a snapshot of the current cached tiles with its ticket, and
  lands the intermediate tiles it folded through.
- The 2×2 halving takes a fast path for blocks wholly inside the level (the
  same bytes).

No pixel changes: every tile stays byte-identical to the whole rebuild
(`compositing/tile-pyramid`).

## Capabilities

- New: `compositing/coarse-tile-folding`.
