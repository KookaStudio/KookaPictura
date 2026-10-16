# Tasks: port-photorust-shear

## 1. Engine

- [x] 1.1 Port photorust's `shear` into `photorust/distort.rs` on the shared `remap`: a `(position, offset)` curve shifts each row horizontally by `offset · width/2`, top row `-1`, bottom row `1`, `wrap` selecting the edge mode.
- [x] 1.2 Route `Filter::Shear` through `photorust::dispatch::plan` with the curve validation; remove the old `distort/coord.rs` vertical shift and its `apply.rs` arm.
- [x] 1.3 Export `SHEAR_MAX_POINTS = 8` from `pictura-filters`.
- [x] 1.4 Port photorust's Shear unit tests (straight curve no-op, rows pushed by the curve, fill modes differ) and the validation cases.

## 2. App

- [x] 2.1 Map the `shear` kind to 18 slots: a point count, eight `(position, offset)` pairs, then the fill, defaulting to a straight curve and `Wrap Around`.
- [x] 2.2 Add `shear` to `filter_preview_needs_whole_layer`.
- [x] 2.3 Add the `ShearCurve` (a count plus eight pairs = 17 slots) and `Radio` (1 slot) controls and the `previewBelow` dialog layout; switch the Shear spec row to the curve box + `Undefined Areas` radios.
- [x] 2.4 Composite the pane's filtered active layer over the document with it hidden, so only the active layer is filtered (`filter_proxy_base` / `filter_proxy_layer`); cover the multilayer pane in `tst_shear_dialog`.

## 3. Verification

- [x] 3.1 Update the oracle property tests: row shift with the top row pinned, fill modes differ, zero curve no-op.
- [x] 3.2 Re-measure the ImageMagick delta against `-shear {angle}x0`; update `mapping.rs`, `tests/README.md`, and `scripts/filter_oracle.py`.
- [x] 3.3 Update `tst_filter_menu` (shear slots 7 → 18) and add `tst_shear_dialog`: layout, curve add/move/remove, sampled slot round-trip, commit/cancel, whole-layer preview.
- [x] 3.4 Update `docs/06-filters/distort-filters.md` and `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).
- [x] 3.5 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
