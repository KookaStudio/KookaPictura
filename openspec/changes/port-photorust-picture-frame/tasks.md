# Tasks: port-photorust-picture-frame

## 1. Engine

- [x] 1.1 Add `source_over` to `photorust/pixmap.rs` and a `render/picture_frame.rs` port of photorust's frame model (`FrameOptions`, the catalogue, the rasteriser, the frames, the moulding).
- [x] 1.2 Add `Filter::PictureFrame { options: FrameOptions }`, dispatch it through the photorust engine, validate its ranges, and add it to `filters_alpha`; export `FrameOptions` and the name tables.
- [x] 1.3 Amend `imaging/filter-application`'s "Alpha and layer metadata preservation" requirement so a compositing kernel (Picture Frame) may add opacity, not just the moving kernels.
- [x] 1.4 Add a `PictureFrame` instance to `moving_kernels_map_opaque_to_opaque` (`pictura-render`), so the new alpha-moving kind rides the opaque-layer guard.

## 2. App

- [x] 2.1 Add the `picture-frame` kind (22 slots) to `filter_map.rs`, `arities.rs`, and the shared filter contract tests.
- [x] 2.2 Add the `Filter ▸ Render ▸ Picture Frame` menu leaf and its `FilterCommandSpec`.
- [x] 2.3 Add `picture-frame` to `filter_preview_needs_whole_layer`.
- [x] 2.4 Add `PictureFrameDialog` (frame picker, preview, Basic/Advanced pages) and route the menu and Last Filter Settings to it.

## 3. Verification

- [x] 3.1 Port photorust's Picture Frame property tests (every frame draws; flowers/leaves have shapes; holes; colours; margins; arrangement; fade; thickness; lines; invert; moulding lighting) plus rejection.
- [x] 3.2 Extend the shared ported-filter contract and the app mapping tests with `picture-frame`.
- [x] 3.3 Add the `tst_picture_frame_dialog` Qt Test suite and register it in CMake.
- [x] 3.4 Update `docs/06-filters/render-filters.md` (`TASK-ALLOWS-DOCS`).
- [x] 3.5 Run `bash scripts/verify-full.sh` and `openspec validate --all --strict`.
