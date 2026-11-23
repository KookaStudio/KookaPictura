## 1. Engine

- [x] 1.1 Treat a shape layer as content by its outline bounds in
  `layer_ops/align.rs` content and enablement checks.
- [x] 1.2 Translate a shape layer's outline instead of offsetting its
  canvas-sized fill rect when Align or Distribute moves it.
- [x] 1.3 Add `align` unit tests for shape enablement, alignment, and
  distribution.

## 2. UI

- [x] 2.1 Add `tst_align_distribute::shapeLayers` covering menu and Move
  options-bar enablement plus the resulting outline moves.

## 3. Verification

- [x] 3.1 `cargo fmt --all`, clippy, and the `pictura-render` tests pass.
- [x] 3.2 `tst_align_distribute` passes.
- [x] 3.3 `openspec validate --all --strict` passes.
- [x] 3.4 Archive the change.
