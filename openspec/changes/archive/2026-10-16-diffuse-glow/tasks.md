# Tasks: diffuse-glow

## 1. Engine

- [x] 1.1 `distort::diffuse_glow` on the planar buffer; `Filter::DiffuseGlow` and its dispatch.

## 2. App

- [x] 2.1 `diffuse-glow` kind (arity 4) in `filter_map.rs`; dialog sliders in `filter_commands.cpp`.
- [x] 2.2 `tst_filter_gallery` expects Diffuse Glow first under Distort.

## 3. Verification

- [x] 3.1 Property tests: identity at Glow 0 / Clear 20 / Graininess 0, highlight bloom and halo, Clear thins the veil, seeded grain that never darkens and leaves alpha, range rejection.
- [x] 3.2 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
