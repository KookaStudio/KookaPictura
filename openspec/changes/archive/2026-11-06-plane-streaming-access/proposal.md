# Proposal

## Why

Issue #1212, step 4 of #1209 (bounded memory for large documents). Calling
`Plane::as_slice()` on a plane whose bands have diverged builds a
**canvas-sized contiguous copy** and caches it until the next write.
`as_mut_slice()`, `Deref`/`DerefMut`, `AsRef` and `&Plane` iteration all go
through the same path, and the copies are counted in `MATERIALIZATIONS`
(`crates/pictura-core/src/plane.rs`). Each such call on a large document
quietly doubles that plane's memory, and none of them can work once planes are
tiled (#1213). That tiled plane has to replace the bands without touching
hundreds of callers, so the callers must stop asking for whole-plane slices
first.

## What Changes

- Add streaming access to `Plane` that never builds a copy: `rows` /
  `rows_mut`, `read_rect_into` / `write_rect_from`, `read_into` (a logical
  range, for buffers that stack channels), and `for_each_band` /
  `for_each_band_mut`. The mutable forms fork only the bands they touch.
- Add `pictura_core::assert_no_materialization(|| …)`, a test guard that fails
  when the closure materialized a diverged banded plane.
- Migrate the `pictura-render` hot paths (compositing mask reads, the
  layer-effect alpha matte, Move's lift, merge, grow and trim, Free
  Transform's resample, clipboard copy and clear, Layer via Copy/Cut, layerless
  RGBA frames), the core layer grow and trim (#1208), and the history's
  detached snapshot copy (`copy_plane`) to row, range, band or sample access.
- Make the memory benchmark's heap guards (`tests_profiles/memory.rs`) report
  each scenario's materializations and assert 0 for the scenarios in
  `STREAMED`.
- Record the per-crate inventory of every remaining slice caller in
  `design.md`. The paint, select, app and codec migrations follow as their own
  changes.

This is a pure refactor: output is bit-identical and no golden changes.

## Capabilities

### New Capabilities

- `document/plane-access`: streaming access to pixel planes, and the rule that
  hot paths never materialize a banded plane.

### Modified Capabilities

None.

## Impact

- `crates/pictura-core/src/plane.rs`, new `plane/access.rs` (+ tests),
  `layer_bounds.rs`, `lib.rs` (export).
- `crates/pictura-render/src`: `composite.rs`, `composite_route.rs`,
  `mask_refine.rs`, `layer_effects/planes.rs`, `layer_source/mod.rs`,
  `document_composite.rs`, `document_ops/layer_ops/{move_content, transform,
  clipboard, neutral, via}.rs`, new `tests/plane_streaming.rs`.
- `crates/pictura-app/src/history/planes.rs` (`copy_plane`) and
  `crates/pictura-app/src/cxxqt_object/tests_profiles/memory.rs`.
- No dependency, document-format, or golden-image changes.
