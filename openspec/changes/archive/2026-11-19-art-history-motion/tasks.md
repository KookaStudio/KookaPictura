# Tasks

## 1. Reproduce and attribute

- [x] 1.1 Pin the Art History options in `profile-tools.py`; add
  `art-history-textured`, `history-brush-textured` and the `texture` setup.
- [x] 1.2 Add `brush_source` to the `history` control method and the
  `pictura_history` MCP schema.
- [x] 1.3 Take the 8000×6000 baseline, then attribute the delay with the
  ignored `pictura-paint::motion_profile`,
  `pictura-app::art_history_present_profile` and
  `pictura-app::region_composite_crossover_profile`.

## 2. Tests

- [x] 2.1 `pictura-paint::one_pass_strokes_match_tip_by_tip_stamping`
- [x] 2.2 `pictura-paint::cancelling_on_a_tiled_layer_restores_partial_edge_tiles`
- [x] 2.3 `pictura-app::a_small_region_refresh_composites_on_the_cpu`
- [x] 2.4 `pictura-render::a_repair_with_only_coarse_tiles_cached_matches_a_rebuild`
  and `pictura-render::interleave_matches_a_serial_copy`
- [x] 2.5 `profile-tools::self_check_motion`, run by `--self-check`
- [x] 2.6 `tst_control_editing::historySetsTheBrushSource`

## 3. Implementation

- [x] 3.1 `ArtHistoryBrush::stamp_path`: one row-parallel pass per stroke, a
  hoisted `TipParams`, and each row clipped to the tip's reach.
- [x] 3.2 `Saved::save`: one plane read per 64 px tile.
- [x] 3.3 `region_on_gpu`: region refreshes below `GPU_REGION_PIXELS` use the CPU.
- [x] 3.4 `TilePyramid::repair`: level-0 rows only when a level-0 tile is
  current; `halve_level` takes its input by value; parallel `interleave`.

## 4. Verification

- [x] 4.1 Re-run the 8000×6000 workload three times and record the results in
  `design.md`.
- [x] 4.2 `cargo fmt --all`, clippy, nextest, the Qt suites and
  `openspec validate --all --strict`.
