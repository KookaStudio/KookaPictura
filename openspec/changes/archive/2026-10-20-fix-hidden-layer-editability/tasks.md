# Tasks

## 1. Regression tests

- [x] 1.1 Add `hiddenLayerShowsNoControlsOrMove` to `TransformControlsTest`:
  show the box, hide the layer, assert the box and session are gone, then drag
  on the canvas and assert the rect and history are unchanged.
- [x] 1.2 Add `hiddenLayerRefusesTransforms`: hide the layer, assert
  `layer_can_free_transform` and the `EditFreeTransform` action are false,
  `begin_free_transform`/`beginFreeTransform`/`begin_move_preview` refuse, and
  a selection still cannot lift the hidden layer.

## 2. Visibility gates

- [x] 2.1 In `impl_transform/session.rs`, refuse a hidden layer in
  `can_free_transform`, `lifts_selection`, the move preview/prepare/commit
  paths.
- [x] 2.2 In `impl_transform/mod.rs`, refuse a hidden layer in
  `duplicate_move_target`.
- [x] 2.3 In `impl_selection.rs`, refuse a hidden active layer in
  `move_selection_content` and `begin_selection_duplicate_preview`.

## 3. Integration verification

- [x] 3.1 Build with CMake/Ninja.
- [x] 3.2 Run `ctest --test-dir build -R '^tst_transform_controls$' --output-on-failure`.
- [x] 3.3 Run `python3 scripts/check-test-plan.py` and
  `openspec validate --all --strict`.
- [x] 3.4 Archive the change with
  `openspec archive fix-hidden-layer-editability -y`.
