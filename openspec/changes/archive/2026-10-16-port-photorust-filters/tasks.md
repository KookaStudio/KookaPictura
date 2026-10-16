# Tasks: port-photorust-filters

## 1. Engine

- [x] 1.1 `photorust::pixmap` shim and the ported modules; `rayon` in `pictura-filters`.
- [x] 1.2 Kooka's sub-type enums aliased in; photorust's menu helpers (`from_i32`, `distort_grid`) dropped.
- [x] 1.3 Seed context and seeded primitives; deterministic reductions.
- [x] 1.4 `dispatch`: spec-range validation, seeded run, colour-only write-back.
- [x] 1.5 Port fixes: Colored Pencil background paper, Mosaic rounding.

## 2. Cleanup

- [x] 2.1 Superseded Kooka implementations and their unit tests removed; Solarize, Shear, Ocean Ripple, Median, Despeckle kept.
- [x] 2.2 Unported photorust code (Sharpen, High Pass, Unsharp Mask, Custom, Radial Blur, Smart Sharpen, Shear) and its tests removed for #222–#226.
- [x] 2.3 Modules split under the file-size caps.

## 3. Verification

- [x] 3.1 `tests/ported`: the shared contract for all 68 ported filters.
- [x] 3.2 `tests/scenarios`: the replaced unit tests through `apply`; divergences reconciled with CS6 Help and recorded here.
- [x] 3.3 Oracle suite: Solarize stays exact, Mosaic stays exact; Emboss, Fragment, Mezzotint property tests updated.
- [x] 3.4 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
