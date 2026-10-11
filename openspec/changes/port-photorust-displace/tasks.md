# Tasks: port-photorust-displace

## 1. Engine

- [x] 1.1 Add `DisplaceMap`, `DisplaceFit`, and `Filter::Displace` to `pictura-filters`.
- [x] 1.2 Port photorust's `displace` into `photorust/displace.rs` on the shared `remap`, with the contract's 128 px reach, a bilinear stretched map, and the neutral/zero-scale shortcut.
- [x] 1.3 Route `Filter::Displace` through `photorust::dispatch::plan` with the scale range checks; add it to `filter_preserves_opacity`'s moving list.
- [x] 1.4 Port photorust's Displace tests (neutral map, the two planes, wrap vs repeat) and add reach, diagonal, and stretch-vs-tile tests.

## 2. App

- [x] 2.1 Add the map store and PSD loader (`displace_maps.rs`) and the `filter_displace_map_load` bridge.
- [x] 2.2 Map the `displace` kind to 5 slots, map id 0 the neutral map; add it to `filter_preview_needs_whole_layer`.
- [x] 2.3 Add `FilterControl::Hidden` and `FilterCommandSpec::livePreview`; add the Displace row; ask for the map after OK in `runFilterDialog`.

## 3. Verification

- [x] 3.1 Add the ImageMagick `displace` op to `scripts/filter_oracle.py` and the differential test; update `mapping.rs` and `tests/README.md`.
- [x] 3.2 Add `tst_displace` (dialog, loaded map, Last Filter, refusals) and the Displace slot count to `tst_filter_menu`.
- [x] 3.3 Update `docs/06-filters/distort-filters.md` and `docs/dev/STATE.md` (`TASK-ALLOWS-DOCS`).
- [x] 3.4 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
