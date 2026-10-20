# Proposal

## Why

A Move-tool drag with a live selection is unusable on large documents. On a
4000×4000 document with a 2000×2000 selection the app spends ~340 ms of CPU per
pointer-move event in a `Debug` build — about three events per second — and the
release on a raster-mask target adds more than a second of CPU work before the
next frame. The result is correct; only the cost is wrong.

Measured on this branch (4-core machine, medians, `selection_move_perf`):

| Work | When | dev (opt-level 0) | release (opt-level 3) |
|---|---|---|---|
| selection bounds scan | every pointer move | 334 ms | 54 ms |
| `Selection::translate` | every move during a drag | 2.7 ms | 1.6 ms |
| `crop_selection` 4000² | mask release | 359 ms | 41 ms |
| `move_mask_selection_content` | mask release | 727 ms | 77 ms |
| full CPU composite | release | 4.49 s | 0.85 s |

Three separate causes:

1. **The app's engine crates run unoptimised.** `Cargo.toml` mirrors the
   `[profile.test]` overrides under `[profile.test.package.*]` but has no
   `[profile.dev]` overrides, and a CMake `Debug` build makes Corrosion build the
   Rust staticlib with the `dev` profile. Every engine loop therefore runs at
   opt-level 0 — 5–10× slower than the same code at opt-level 2/3.
2. **The per-move path recomputes full-document work.** `InfoPanel::refresh`
   asks for the selection bounds on every mouse move, and
   `selection_bounds_extents` rescans the whole document-sized coverage plane
   with a per-pixel `%` and `/`. `preview_selection_move` then allocates and
   translates a second full-document plane for every event.
3. **The release path scans whole planes.** `move_mask_selection_content`
   walks the entire mask rectangle twice with per-pixel bounds checks and a
   blend, `crop_selection` copies the whole selection plane pixel by pixel,
   `clear_layer` does the same for the layer target, and the commit then runs a
   full-document recomposite instead of the regional refresh the
   `compositing/described-damage` contract already requires.

## What Changes

- **Engine crates are optimised in dev builds.** `[profile.dev.package.*]` sets
  `opt-level = 2` for the nine engine crates, mirroring the existing test-profile
  overrides; dependencies stay at 0 so the cold build does not pay for all 120+.
- **Selection bounds are not rescanned per move.** `Selection::bounds()` scans
  row by row, skipping empty rows and stopping each row at its first and last
  covered pixel. During a drag the bounds are the origin's bounds translated by
  the drag offset and clipped to the canvas, with no plane access at all.
- **A drag stores only the offset.** `preview_selection_move` records
  `(dx, dy)` and leaves the origin plane alone; the translated plane is built
  once, on commit. Coverage hit tests during the drag read the origin at
  `(x - dx, y - dy)`.
- **The mask content move is bounded to the selection.** The engine walks only
  the coverage bounds and their translated destination, clipped to the mask,
  in row slices with no per-pixel bounds checks. `crop_selection` and
  `clear_layer` become row-sliced bulk copies.
- **The commit refreshes only its dirty union.** A content move composites the
  union of the selection's source and destination bounds through the existing
  regional path instead of recompositing the document.
- **The Info panel's cursor refresh is cursor-only.** A pointer move updates
  the position, colour, and selection-size readouts; the document-size line and
  the sampler list update on document or selection changes, so no `stat` runs
  per move.

**Out of scope.** Krita-style reduced-resolution preview during the drag is
unnecessary once the redundant full-plane work is gone. Moving the release work
to a background thread is not part of this change.

**BREAKING**: none. Every changed path keeps its output byte-identical, and the
`changed` / `region_blitted` signal contracts are preserved.

## Capabilities

### New Capabilities

- `tools/move-drag-performance`: a Move-tool drag with a live selection does
  bounded work per pointer move — row-based bounds, offset-only preview,
  selection-bounded mask move — and stays interactive on large documents.

### Modified Capabilities

- `compositing/described-damage`: a selection content move commit refreshes the
  union of the selection's source and destination bounds through the regional
  path instead of the full recomposite.
- `ui/info-histogram-panel`: a pointer move refreshes only the cursor-dependent
  readouts and never stats the document file.

## Impact

- `Cargo.toml`: `[profile.dev.package.*]` overrides.
- `crates/pictura-select`: `Selection::bounds` (new, row-based).
- `crates/pictura-render`: `move_mask_selection_content` (bounded, row-sliced),
  `clear_layer` (row-sliced).
- `crates/pictura-app/src`: `cxxqt_object/state.rs` (drag offset),
  `cxxqt_object/impl_selection.rs` (preview/commit/bounds/coverage),
  `cxxqt_object/impl_core.rs` (`recomposite_region`),
  `cxxqt_object/helpers_composite.rs` (`crop_selection`),
  `cxxqt_object/annotations.rs` (size readout) — plus
  `cpp/panels/info_panel.cpp` (cursor-only refresh).
- Tests: `selection_move_perf` profiling harness; new unit, equivalence, and Qt
  Test cases; existing `tst_mask_edit`, `tst_selection_move`, and
  `tst_info_panel` suites stay green.
- No new dependencies.
