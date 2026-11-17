# Tasks: one stroke lifecycle

## 1. Tests

- [x] 1.1 Add the layer-constructor unit test named in the test plan.
- [x] 1.2 Add the mask-constructor unit test named in the test plan.
- [x] 1.3 Add the refused-begin unit test named in the test plan.

## 2. Implementation

- [x] 2.1 Add `StrokePlan`, `StrokeTarget`, and the one `StrokeSession::begin` constructor.
- [x] 2.2 Route `begin_paint` and the layer plan through the constructor.
- [x] 2.3 Route `begin_mask_paint` and the mask plan through the constructor.
- [x] 2.4 Route `paint_tools::begin` through the constructor and drop the duplicated begin block.
- [x] 2.5 Add `StrokeFinish` and the typed layer/mask finish results.
- [x] 2.6 Make `end_paint` consume the finish result without destructuring `StrokeSession`.

## 3. Verification

- [x] 3.1 `cargo fmt --all --check`
- [x] 3.2 `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 3.3 Paint/eraser/healing/mixer Qt suites and the self-test paint checks pass unchanged.
- [x] 3.4 `openspec validate --all --strict`
- [x] 3.5 Archive the change with `openspec archive one-stroke-lifecycle`.
