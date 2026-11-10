# Tasks: auto-align-layers

## 1. Engine

- [x] 1.1 Add `auto_align` module skeleton: public types, eligibility, reference selection, progress/cancel, `can_auto_align`, `plan_auto_align`, `apply_auto_align`.
- [x] 1.2 `image.rs`: bounded-scale luma/alpha `WorkImage`, optional lens-corrected sampling.
- [x] 1.3 `features.rs`: Harris corners, dominant orientation, SIFT-lite descriptors, ratio-test matching.
- [x] 1.4 `estimate.rs`: RANSAC translation/similarity/homography, normalised DLT, overlap scoring.
- [x] 1.5 `projection.rs`: cylindrical/spherical maps and the `PlaneMap` composition.
- [x] 1.6 `lens.rs`: bake a `LensCorrectionParams` through `LensSource`/`LensMap`.
- [x] 1.7 Expose `transform_layer_with_map` from `layer_ops/transform.rs`; export the new module from `layer_ops/mod.rs`.
- [x] 1.8 Unit tests for all engine scenarios in the test plan.

## 2. App bridge

- [x] 2.1 Add `cxxqt_object/auto_align.rs` + `auto_align/imp.rs` worker, progress, cancel, stale guard, one-edit commit, notice.
- [x] 2.2 Add the app-side lens-parameter resolver from EXIF + installed profiles.

## 3. Dialog and command

- [x] 3.1 Add `auto_align_dialog.{h,cpp}` with seven projection radios and the two Lens Correction sub-options.
- [x] 3.2 Add `EditAutoAlignLayers` to the Rust catalogue, `const_names`, `modal_ids`, regenerate `command_ids.h`.
- [x] 3.3 Wire the menu handler and enablement in `frame_menus_align.cpp`; add CMake source entries.

## 4. Verification

- [x] 4.1 Qt Test `tst_auto_align.cpp`; register it in `cpp/tests/CMakeLists.txt`.
- [x] 4.2 Targeted verification: `cargo test -p pictura-render --lib auto_align`, `ctest -R '^tst_auto_align$' -j 1`, clippy on `pictura-render`/`pictura_app`, file-size, guard, test-plan, and `openspec validate --all --strict`. Full `verify-fast.sh` is deferred to CI: it OOM-killed the 7.3 GiB local workstation.
- [x] 4.3 Tick `ROADMAP.md`, mark `LAY-4` shipped, update `docs/02-ui-ux/menus.md` and `docs/dev/STATE.md` with `TASK-ALLOWS-DOCS`.
