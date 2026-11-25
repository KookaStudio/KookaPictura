# Proposal

## Why

A brush stroke on a large, mostly empty layer is refused with "Could not paint:
not enough memory" while the process is far below its ceiling (#1603). The
reproduction is a canvas about 21,000 px square with a sparse scribble layer of
about 15,000 × 17,500 px, at 10 % Memory Usage: a ceiling of 2,729 MB and an RSS
of 802 MB.

The stroke-start gate `brush_bytes` charges the layer's *logical* size
(`Plane::len`, width × height × channels, about 1.0 GB here) plus 3 × the canvas
area (about 1.3 GB). The canvas term predates tiled planes (#1212–#1216) and was
never re-measured. The total, about 2.4 GB, exceeds the room of about 1.9 GB.

The estimate mirrors a real allocation. The stroke's undo copy (`Saved`) holds
one dense, layer-sized buffer per channel, and the same for the 16-bit store of a
tone stroke. Only the pages a stroke writes become resident, so RSS stays low,
but the copy still counts against the allocator. When the layer grows under a
stroke, `Saved::grow` builds a new dense buffer and copies every unsaved pixel
of the old layer into it, so the whole layer becomes resident.

## What Changes

- The undo copy keeps its pre-stroke pixels as 64 × 64 tiles, each allocated
  when a dab first reaches it. The tiles sit on a grid anchored to document
  coordinates, so growing the layer moves no pixels and allocates no tile. The
  8-bit planes and the 16-bit store work the same way.
- `brush_bytes` charges the bytes the target layer holds (`allocated_bytes`, so
  uniform tiles cost nothing) plus one byte per canvas pixel, down from three.
  The canvas term is checked against the measured peaks of M6 and of a new
  dense-layer brush scenario.
- A stroke over a dense layer still pays the layer's full held size, so a stroke
  that genuinely needs the room is still refused with the existing message.

## Capabilities

### New Capabilities

- `document/brush-stroke-memory`: what a stroke's undo copy holds, what the
  stroke-start gate charges, and the scenarios the estimate must cover.

### Modified Capabilities

None. `document/memory-governor` still requires every estimate to cover its
measured peak; this change adds a scenario under the new capability.

## Impact

- `crates/pictura-paint/src/stroke/saved.rs` (rewritten), `stencil.rs`,
  `raster.rs`, `stroke.rs` (the saved-pixel read).
- `crates/pictura-app/src/cxxqt_object/helpers/memory_gate.rs` (`brush_bytes`)
  and its tests.
- `crates/pictura-app/src/cxxqt_object/tests_profiles/memory.rs` and
  `memory/scenarios.rs` (the dense-layer brush scenario).
- No new dependency, no UI change.
