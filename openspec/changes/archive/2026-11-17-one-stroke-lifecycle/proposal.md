# One stroke lifecycle for every painting tool

## Why

Every painting tool repeats the same stroke-begin bookkeeping. `impl_paint::begin_paint`
and `paint_tools::begin` both build a `StrokeSession`, take the live slot, clear the
pending present, reset the stroke tiles, and open tool recording. Mask strokes add a
third variant. `end_paint` then destructures the session and forks on internals the
caller cannot see. The slot/present-frame/tile-set invariant is asserted in three
places instead of one, so a new painting tool can silently miss one of the steps.

## What Changes

- Add one `StrokeSession` constructor that takes a built layer or mask target and owns
  the slot, label, paint pad, fade capture, pending present, tile set, and recording.
- Route `begin_paint`, `paint_tools::begin_*`, and the mask begin through that
  constructor. Layer and mask strokes share one entry.
- Replace the session destructuring in `end_paint` with a typed finish result that
  carries the layer or mask finish data.
- Keep every pixel, history state, preview replay, GPU path, and refusal rule
  unchanged.

## Capabilities

### New Capabilities

- `document/paint-stroke-lifecycle`: one begin and one finish operation own the live
  paint stroke for every painting tool.

### Modified Capabilities

None.

## Impact

- `crates/pictura-app/src/cxxqt_object/document_session/stroke/mod.rs`
- `crates/pictura-app/src/cxxqt_object/impl_paint.rs`
- `crates/pictura-app/src/cxxqt_object/impl_paint/mask.rs`
- `crates/pictura-app/src/cxxqt_object/paint_tools/imp.rs`
- Rust unit tests under `crates/pictura-app/src/cxxqt_object/document_session/tests.rs`

No public API, dependency, document format, or pixel output changes.
