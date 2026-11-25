# Tasks

## 1. Tiled undo copy

- [x] 1.1 Rewrite `Saved` as a document-anchored grid of boxed 64² tiles (8-bit planes and the 16-bit native store), allocated on first save.
- [x] 1.2 Rebuild the grid on `grow` by moving boxes; copy no pixels.
- [x] 1.3 Read saved pixels through `Saved::pixel` in `Stencil::composite`, passing layer-local `(x, y)`.
- [x] 1.4 Add `stroke/tests/saved.rs`: one dab saves only its tiles; a grow keeps the saved tiles and cancel restores.

## 2. The gate

- [x] 2.1 `brush_bytes` charges the layer's `allocated_bytes` plus one byte per canvas pixel.
- [x] 2.2 Gate tests: a sparse layer on a 21,000² canvas is admitted; a dense layer beyond the room is refused.
- [x] 2.3 Add the dense-layer brush scenario to the memory benchmark guards and to `estimates_cover_the_measured_peaks`; record the measured peaks here.
  Measured at 4250² (one run each, 4 rayon threads): M6 `brush_stroke` 569,891 B (was 791,775 B; a first cut with whole 64² tiles measured 1.0 MiB), `brush_stroke_dense` 2,615,130 B (74,244,859 B with master's dense undo copy). Estimates: 17.2 MiB and 86.1 MiB.

## 3. Close out

- [x] 3.1 Build once, run clippy, nextest and the memory guards, and lower the `brush_stroke` baseline if it fell.
- [x] 3.2 Update `docs/dev/STATE.md`; review `ROADMAP.md` and `PARITY.md`.
- [x] 3.3 Archive the change.
