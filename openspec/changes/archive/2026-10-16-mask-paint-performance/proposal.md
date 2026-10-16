# Proposal: mask-paint-performance

## Why

Painting on a layer **mask** is far slower than the same brush on a pixel layer,
and it degrades with document size (issue #404). The composite path already
treats a mask as **one coverage byte per pixel** (`LayerMask.data:
Option<Plane<u8>>`; `composite_rows.rs:99,114`), but the **edit** path does not:

- `begin_mask_paint` (`crates/pictura-app/src/cxxqt_object/impl_paint.rs:489`)
  fabricates an **RGBA mask-sized document** (`mask_document` →
  `gray_document`, `crates/pictura-render/src/document_ops/layer_ops/mask_edit.rs:26,52`)
  — a `Vec<u8>` of `w*h*4`, a 4-plane `Document`, a 4-plane composite — then runs
  the generic `Stroke` on it and copies channel 0 back per dab with
  `write_mask_back` (`:70`). Every dab writes four channels to move one.
- `begin_mask_paint` never seeds a `gpu_stroke` or a reduced-level `preview`
  (`:518-526`), so `paint_dab`'s mask branch (`:574`) always runs the **exact
  full-resolution CPU raster** — the pixel path's acceleration
  (`impl_paint.rs:421-478`) is never reachable.

The result: 4x the memory traffic and the raster cost of a mask dab, an O(mask)
allocation at stroke start, and no GPU/preview parity for large brushes.

## What Changes

- The mask edit path SHALL edit the mask's **single coverage plane**, not an RGBA
  stand-in. The mask edit document SHALL carry one grayscale channel (the
  coverage), so a dab writes one byte per pixel and no channel-0 copy-back is a
  correctness requirement.
- The mask stroke SHALL use the **same acceleration policy as a pixel stroke**:
  the exact CPU raster for a dab within the raster budget, and a reduced-level
  preview and/or a GPU stroke for a larger brush, chosen by `preview_level`. The
  committed mask SHALL be byte-identical (within the compositor's 1-LSB
  tolerance) to the exact CPU raster of the same sample log.
- A mask stroke SHALL present the changed region only; it SHALL NOT rebuild the
  full mask document, the full pyramid, or recomposite the full document per dab.
- Scope: only the **raster layer mask**, only painting (Brush/Pencil/Eraser) and
  the shared mask-edit document. Fills and filters keep their current path.

## Capabilities

### Modified Capabilities

- `compositing/layer-masks`: a new requirement fixes the mask edit path as
  grayscale-native and accelerated, with the exact-oracle and region-present
  guarantees.

## Impact

- `crates/pictura-render/src/document_ops/layer_ops/mask_edit.rs` — `mask_document`
  / `gray_document` build a 1-channel document; `write_mask_back` copies the
  single plane.
- `crates/pictura-render/src/gpu/stroke.rs` (and `backend.rs` / `shader`) — the
  GPU stroke gains a channel count so a mask stroke is one plane, not four.
- `crates/pictura-app/src/cxxqt_object/impl_paint.rs` — `begin_mask_paint` /
  `mask_dab` / the commit branch route through the preview/GPU policy.
- `crates/pictura-app/src/cxxqt_object/state.rs` — the mask stroke's acceleration
  fields.
- Tests in `crates/pictura-paint`, `crates/pictura-render`, and
  `crates/pictura-app` (`tst_mask_edit.cpp`), plus `#[ignore]` perf profiles.
- No `docs/` change (the contract is behavioural, not user-visible). No new
  dependency.
