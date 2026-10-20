# Tasks

## 1. Tests

- [x] 1.1 Add Rust gesture tests for the Distort delta, grab offset, convex clamp, and pivot carry in `impl_transform/distort.rs`.
- [x] 1.2 Add the projective front-side engine test in `pictura-render`'s transform tests.
- [x] 1.3 Add `tst_transform_distort` (gesture, pivot, guide snap, type refusal, menu enabled state, preview clip) and register it in the Qt test CMake list.

## 2. Gesture and session

- [x] 2.1 Move `keep_convex`/convex helpers into `impl_transform/convex.rs` and keep Skew using them.
- [x] 2.2 Port photocraft's Distort gesture into `impl_transform/distort.rs`: press delta, convex clamp, pivot carry.
- [x] 2.3 Add `start_pivot` to `TransformSession` and capture it at press.
- [x] 2.4 Refuse Distort on editable type at begin time; expose `layer_can_transform_mode` and use it in the menu provider.

## 3. Guide snapping and horizon

- [x] 3.1 Snap Distort corners to visible guides in `ToolController`, drawing and clearing the snap lines.
- [x] 3.2 Add the front-side rule to `QuadMap` and skip far-side samples.
- [x] 3.3 Add the projective preview front-side clip helper and use it in `ImageView`'s projective preview.

## 4. Verification

- [x] 4.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run -p pictura-app -p pictura-render`.
- [x] 4.2 CMake build, `ctest --test-dir build -R '^tst_transform'`, and `./build/pictura --headless --self-test`.
- [x] 4.3 `openspec validate distort-transform-parity --strict` and `python3 scripts/check-test-plan.py`.
- [x] 4.4 Archive the change and update `docs/dev/STATE.md`.
