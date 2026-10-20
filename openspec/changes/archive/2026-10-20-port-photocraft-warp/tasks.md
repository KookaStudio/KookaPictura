# Tasks

## 1. Tests

- [x] 1.1 Port photocraft's geometry tests into `pictura-core/src/warp/tests.rs` (split removal and `then_affine` dropped).
- [x] 1.2 Add `layer_ops/warp_tests.rs` (identity, exact translation, Arc, thin curve, shrink prefilter, fold, mask, 16-bit store).
- [x] 1.3 Add `impl_transform/warp.rs` unit tests (grab kinds, handle anchors, preset to custom).
- [x] 1.4 Add the `tst_transform_warp` Qt Test suite; drop the preset-warp cases from `tst_transform_options`.

## 2. Implementation

- [x] 2.1 Port `geom::warp` to `pictura_core::warp` (CS6 scope: no split removal).
- [x] 2.2 Port `algo::warp` to `layer_ops/warp_raster.rs` and rewrite `transform_layer_warp` over `Warp`; remove `warp_styles.rs` and the old native warp helpers.
- [x] 2.3 Replace the session's preset warp with Warp mode (`impl_transform/warp.rs`, dispatch, commit, preview).
- [x] 2.4 C++: Warp menu enters Warp mode; options-bar Warp fields; canvas warp grid; remove `WarpPresetDialog` and the `lpr_warp_preset` self-test (code 527 retired, budget lowered).
- [x] 2.5 Credit photocraft in `NOTICE.md` and `docs/03-tools/move-and-transform.md`.

## 3. Verification

- [x] 3.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`.
- [x] 3.2 CMake build; run `tst_transform_warp`, `tst_transform_options`, the transform suites, and the self-test.
- [x] 3.3 `openspec validate port-photocraft-warp --strict`, then archive the change.
