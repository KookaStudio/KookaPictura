# Tasks: port-photorust-lens-flare

## 1. Engine

- [x] 1.1 Port photorust's `lens_flare` into `render/lens_flare.rs` on the planar buffer; keep `LensType` names, brightness rejection, and centre clamping.
- [x] 1.2 Remove the old Gaussian flare model from `render.rs`.

## 2. App

- [x] 2.1 Add `lens-flare` to `filter_preview_needs_whole_layer` (#168).
- [x] 2.2 Add `LensFlareDialog` (preview pad with crosshair, Brightness field and slider, Lens Type radio group, OK / Cancel / Preview) and route the menu and Last Filter Settings to it.

## 3. Verification

- [x] 3.1 Port photorust's Lens Flare property tests: flare position, brightness scaling, ghost axis, distinct lenses, alpha, size invariance, plus clamping and rejection.
- [x] 3.2 Extend `positional_kinds_preview_the_whole_layer` with `lens-flare`.
- [x] 3.3 Add the `tst_lens_flare_dialog` Qt Test suite.
- [x] 3.4 Update `docs/06-filters/render-filters.md` (`TASK-ALLOWS-DOCS`).
- [x] 3.5 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
