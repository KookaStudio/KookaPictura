# Tasks: large-tool-performance

## 1. Measurement

- [x] 1.1 Control server: `tool_options`; `pointer` `down` / `up`, with `move` carrying the held button.
- [x] 1.2 `scripts/profile-tools.py`: scenarios, per-request timing, `--save` / `--compare` / `--timing-log`.
- [x] 1.3 Paint-timing sessions for the brush family's stroke start, shape layers and Shape Pixels.
- [x] 1.4 `shape_commit_profile` (ignored) in `pictura-render`.
- [x] 1.5 Baselines on 4000×3000 and the world map.

## 2. Tests

- [x] 2.1 `pictura-paint::a_banded_large_dab_matches_the_serial_raster`.
- [x] 2.2 `pictura-core::a_row_beyond_the_path_is_outside_whatever_the_cached_clip`, `coverage_does_not_depend_on_the_columns_beside_the_path`.
- [x] 2.3 `tst_tool_commit` (stroke, cancel, shape, fill, styled, `tool_options`, pointer steps); `pictura-app::a_style_anywhere_but_the_exempt_layer_counts`.

## 3. Brush

- [x] 3.1 Banded parallel dab raster for Paint-kind strokes; Dissolve stays serial.
- [x] 3.2 In-stroke presents patch the composite; the commit presents only the pending region; cancel repairs the composite.

## 4. Shapes and fills

- [x] 4.1 Vector-mask rows beyond the path answer without an edge scan.
- [x] 4.2 A new shape layer refreshes its outline + stroke bounds; full recomposite with another layer's style or a clipped layer above.
- [x] 4.3 The fill family refreshes the edit's rectangle; full recomposite when any layer is styled.
- [x] 4.4 `fill_layer` runs rows in parallel; shape coverage sweeps only the path's columns.

## 5. Verification

- [x] 5.1 After profiles on 4000×3000 and the world map (`design.md`).
- [x] 5.2 `cargo fmt`, clippy, nextest, Qt tests, `openspec validate --all --strict`, `scripts/verify-fast.sh`.
