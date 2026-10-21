# Tasks

## 1. Tests

- [x] 1.1 Rust gesture tests in `impl_transform/perspective.rs`: symmetric corner spread, mid-drag edge switch, tilted edge, bowtie, edge slide, pivot carry, degenerate refusal, readout angle, Free base composition and scale anchor.
- [x] 1.2 Add `tst_transform_perspective` (mode gesture + commit, type refusal, menu disabled for type, Ctrl+Alt+Shift through the tool controller, plain scale after it, commit, type shortcut ignored, readout shown and cleared) and register it in the Qt test CMake list.

## 2. Gesture and session

- [x] 2.1 Add `impl_transform/perspective.rs` with the corner/edge gesture, bilinear pivot carry, and the drag angle; remove the old `gesture_perspective` from `geometry.rs`.
- [x] 2.2 Add `base_quad`, `free_perspective`, and the readout fields to `TransformSession`; make `session_quad`, commit, and the preview matrices honour the base.
- [x] 2.3 Scale a Free box with a base in its own projective frame (`gesture_scale_base`).
- [x] 2.4 Refuse Perspective on editable type in `can_begin_mode`.
- [x] 2.5 Fill a crossed quad as its two straight triangles: `crossed_quad::CrossedMap` in the engine and the same map in the CPU preview warp, with tests.
- [x] 2.6 Lock the Layer menu (registry gate, dispatch refused) and grey the Layers panel while a transform is pending (`frame_transform_lock.cpp`), with `tst_transform_lock`.
- [x] 2.7 Ask "Apply the transformation?" before a tool switch leaves a pending transform (`ToolController::resolvePendingTransform`, `askApplyTransform`), with `tst_transform_lock`.

## 3. Bridge and C++

- [x] 3.1 Bridge `transform_begin_free_perspective` and `transform_drag_angle_degrees`.
- [x] 3.2 `ToolController`: call the shortcut on a Ctrl+Alt+Shift corner press; show and clear the readout.
- [x] 3.3 Disable `Edit > Transform > Perspective` for editable type in the menu provider.

## 4. Verification

- [x] 4.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p pictura-app`.
- [x] 4.2 CMake build and `ctest --test-dir build -R '^tst_transform'`.
- [x] 4.3 `openspec validate perspective-transform-gesture --strict` and `python3 scripts/check-test-plan.py`.
- [x] 4.4 Archive the change and update `docs/dev/STATE.md`.
