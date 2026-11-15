# Tasks

## 1. Engine

- [x] 1.1 Add `CoverageClip` and document-space sampling to the stroke stencil.
- [x] 1.2 Keep the Quick Mask path on the new clip type.
- [x] 1.3 Add the engine unit tests for selection clipping, offset layers and
      growth.

## 2. App

- [x] 2.1 Attach the active selection in `begin_kind` for every stroke tool.
- [x] 2.2 Attach it in `begin_paint` and `begin_mask_paint` (mask origin).
- [x] 2.3 Carry the clip on the reduced-level preview and apply it in both
      blends.
- [x] 2.4 Decline the GPU dab while a selection is active.

## 3. Verification

- [x] 3.1 Qt Test: brush stroke on a layer, on a new empty layer, and on a
      layer mask.
- [x] 3.2 Run the focused Rust and Qt suites, `cargo fmt`, `cargo clippy`.
- [x] 3.3 `openspec validate --all --strict`; archive the change.
