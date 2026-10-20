# Tasks

## 1. Tests

- [x] 1.1 Add Rust gesture tests for corner/edge/Alt/type/convex behavior in `impl_transform/geometry.rs`.
- [x] 1.2 Add `tst_transform_skew` and register it in the Qt test CMake list.
- [x] 1.3 Update the C++ self-test's Skew edge expectation to the axis-constrained result.

## 2. Implementation

- [x] 2.1 Add `type_layer` to `TransformSession`, set from the target layer at session begin.
- [x] 2.2 Port photocraft's `gesture_skew`: corner follows its dominant edge, edge slides along itself, Alt mirrors, type stays a parallelogram, convex clamp.
- [x] 2.3 Pass the existing Alt flag from `transform_move` into the Skew gesture.

## 3. Verification

- [x] 3.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p pictura-app`.
- [x] 3.2 Configure and build with CMake, then run `ctest --test-dir build -R '^tst_transform_skew$'` and `./build/pictura --headless --self-test`.
- [x] 3.3 `openspec validate skew-transform-gestures --strict`, then archive the change.
