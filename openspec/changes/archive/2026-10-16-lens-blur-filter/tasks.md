# Tasks: lens-blur-filter

## 1. Engine kernel

- [x] 1.1 Add `DepthMap` (one 8-bit plane, `DisplaceMap`-shaped) and `IrisShape` to `pictura-filters/src/lib.rs`, and `Filter::LensBlur { .. }` to `filter/types.rs`; route it through `filter/apply.rs` and export the new types. Verify: `cargo nextest run -p pictura-filters` and the new variants compile with `cargo clippy -p pictura-filters --all-targets -- -D warnings`.
- [x] 1.2 Implement `blur/lens.rs`: the CoC ramp, the precomputed antialiased iris tap tables (blades, curvature, rotation), the specular highlight sum, seeded uniform/Gaussian noise, the clamped bilinear depth stretch, and parameter validation before mutation. Verify: unit/property tests pass — in-focus sharpness, the black-to-white ramp, a point light repeating a triangle iris and its rotation, specular monotonicity, same-seed determinism/different-seed divergence, monochromatic channels, alpha bit-identical, a 1×2 map stretched over a tall buffer, and radius 101/non-finite rotation refused without mutation.
- [x] 1.3 Add the Lens Blur row to `tests/oracle/mapping.rs` (`im: None`, tolerance 0, a no-equivalent note) and `tests/README.md`, and a Lens Blur entry to `tests/kernel_digests.rs`. Verify: the mapping coverage and digest tests pass with the counts bumped.
- [x] 1.4 Add Lens Blur to `pictura-render::filter_preserves_opacity`'s moving list and `preview_apron` (the master radius), and extend `pictura-render/src/filter/tests.rs`. Verify: `cargo nextest run -p pictura-render` and the moving-kernel test pass.

## 2. App

- [x] 2.1 Add `filter_tools/lens_blur_depths.rs` (ids never 0, eight most recent) and the `filter_lens_blur_depth(source)` bridge that materializes Transparency (channel -1), Layer Mask, or Selection cropped to the active layer rect, returning 0 for None or an unavailable source. Verify: store/bridge unit tests (register/lookup/crop/refusal) pass.
- [x] 2.2 Map `lens-blur` to 15 slots with the listed defaults in `filter_map.rs`, resolving Source None to no map and any other source through the store (id 0 refused). Verify: `filter_map_tests` cover the empty-slot defaults, wrong arity, and the missing-map refusal.
- [x] 2.3 Add the `lens-blur` row to `filter_commands.cpp` (Preview, Source, Focal, Invert, Shape, Radius, Curvature, Rotation, Specular Brightness/Threshold, Noise Amount/Distribution/Monochromatic, hidden id + seed), resolve the source in the dialog's value path, force More Accurate on commit, and add `lens-blur` to `filter_preview_needs_whole_layer`. Verify: the row's slots equal the mapping arity and the default values preview/cancel through the shared dialog.
- [x] 2.4 Add `cpp/tests/tst_lens_blur_dialog.cpp` (layout/defaults, transparency source, unavailable-source refusal, Faster-commits-accurate, commit/cancel, Last Filter) and register it in `tests/CMakeLists.txt`; update `tst_filter_menu`'s slot count. Verify: `ctest --test-dir build -R '^tst_' --output-on-failure` is green.

## 3. Documentation

- [x] 3.1 Update `docs/06-filters/blur-filters.md`'s Lens Blur rows (source default, focal 126, shape Triangle (3), radius 50, curvature 33, rotation 191, specular brightness 44 / threshold 93, noise 47 Gaussian) and the module mapping, in a separate `TASK-ALLOWS-DOCS` commit. Verify: `bash scripts/guard.sh` passes with the flag.
- [x] 3.2 Record the change in `docs/dev/STATE.md` (same `TASK-ALLOWS-DOCS` docs commit) with the final verification numbers.

## 4. Integration verification

- [x] 4.1 Run `TASK_ALLOWS_DOCS=1 bash scripts/verify-full.sh` and `openspec validate --all --strict`, confirming the unified report is green and the new spec deltas validate.
