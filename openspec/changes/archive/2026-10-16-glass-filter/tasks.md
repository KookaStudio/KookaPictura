# Tasks: glass-filter

## 1. Engine

- [x] 1.1 `GlassTexture`, `Filter::Glass`, `distort::glass` with the four procedural surfaces.
- [x] 1.2 Share `kernel::blur_plane` with Diffuse Glow; Glass joins the alpha-moving allowlist.

## 2. App

- [x] 2.1 `glass` kind (arity 5) and its dialog controls; gallery lists it under Distort.

## 3. Verification

- [x] 3.1 Property tests: zero no-op, every surface bends and grows with Distortion, Invert flips, deterministic, alpha untouched, range rejection.
- [x] 3.2 `filter_map_tests` defaults and texture/invert mapping; `tst_filter_gallery` Distort lists three.
- [x] 3.3 `bash scripts/verify-full.sh`; `openspec validate --all --strict`.
