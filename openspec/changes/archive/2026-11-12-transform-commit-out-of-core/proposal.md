# Proposal

## Why

Issue #1290: on a memory-limited host, a Free Transform resize or rotate of a
large layer is refused with "not enough memory". Photoshop CS6 completes the
same commit by paging to its scratch disk. Phases 1–2 (#1291) handled integer
moves and made the resample buffers fallible. A resize or rotate still holds
everything dense in RAM: the resampled layer planes, and the composite of the
changed region. Neither can be packed or spilled by the #1217 evictor, and the
gate charges both as RAM (`4·area(out) + 12·area(union)`).

This change is phases 3–4 of the plan on #1290.

## What Changes

- **Tile-streamed resample (phase 3).** The layer transform resamples one strip
  of 256 destination rows at a time into a tiled plane, for 8-bit and deep
  planes and for layer masks. Each block reads only the source window its
  samples reach, and splits when that window is too large. Tiles that come out
  uniform hold no allocation. Dense tiles are ordinary evictable bodies, so the
  governor can pack or spill them while the commit runs. The output is
  byte-identical to the dense resampler.
- **Tiled region composite (phase 4).** The banded compositor writes a large
  region into a tiled buffer, 256 rows at a time, instead of one dense buffer.
  Patching the stored composite reads such a region tile by tile, and the
  display (sRGB) copy is made only when a display update actually uses it.
- **Scratch-aware gate (phase 4).** A transform commit is admitted when its
  dense working set fits the governor's room, and when its spillable output fits
  in that room plus the free space on the scratch disks. When the scratch disks
  can't hold it, the refusal says so.

Warp (#1292), Image Size (#1293), and Canvas Size and Rotate Canvas (#1294) are
split out. The progress window for spilling commits below 4 MP (phase 5) is
not part of this change.

## Capabilities

- **ADDED** `document/out-of-core-transform`
- **ADDED** `compositing/tiled-region-composite`
- **MODIFIED** `document/transform-commit-memory`: a move's gate charge is the 4-byte tiled composite, not the 12-byte dense one.

## Impact

- `pictura-core`: `Plane::build_tiled_strips`, `memory::try_buffer`.
- `pictura-render`: `layer_ops/transform.rs` (resampler moved to
  `transform_tiled.rs`), `transform_native.rs`, `composite_region.rs`,
  `document_composite.rs` (`patch_buffer_region`).
- `pictura-app`: `impl_transform/group.rs` (gate), `helpers/memory_gate.rs`,
  `helpers_composite.rs` (lazy sRGB), the deferred composite landing, and the
  transform refusal message (`tool_transform.cpp`).
