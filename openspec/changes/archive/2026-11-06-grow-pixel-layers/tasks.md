# Tasks

## Tests first

- [x] 1.1 `pictura_core::layer_bounds::{grows_channels_and_retained_samples,
      refuses_a_transparency_lock, refuses_a_non_superset,
      refuses_when_the_governor_is_full, drops_verbatim_raw_channels}`
- [x] 1.2 `pictura_render::layer_ops::{new_layer_is_empty_rect,
      neutral_fill_layer_is_canvas_sized, trim_keeps_the_composite}`
- [x] 1.3 `pictura_paint::{paint_outside_rect_grows_layer,
      fill_outside_rect_grows_layer, a_bucket_fills_an_empty_layer}`
- [x] 1.4 `pictura_codec::{empty_layer_round_trips, grown_layer_round_trips,
      existing_files_stay_byte_identical}` and
      `crates/pictura-codec/tests/empty_layer_oracle.rs`
- [x] 1.5 `pictura_app::memory_gate::{a_fill_that_grows_the_layer_forks_the_grown_bands,
      a_write_forks_the_bands_it_crosses}` and
      `pictura_app::impl_filters::tests::blur_grows_the_layer`
- [x] 1.6 `tst_fill_tools::fillGrowsAnEmptyLayer` and
      `tst_move_feedback::moveThenFillOutsideTheLayer`

## Implementation

- [x] 2.1 `pictura-core`: `layer_bounds::grow_layer_to` with the governor
      reservation, 8-bit channel padding, retained-sample padding, raw-channel
      drop
- [x] 2.2 `pictura-render`: `empty_pixel_layer`; `add_layer` / `add_layer_full`
      use it (neutral fill stays canvas-sized)
- [x] 2.3 `pictura-paint`: `Stroke` grows its target for a sample that leaves the
      rect, padding its saved planes, coverage buffers and per-dab engines; a
      cancel shrinks the layer back
- [x] 2.4 `pictura-paint`: `fill_layer_within` grows its target to its damage
      box; `bucket` passes the mask's bounds, the gradient the selection box
- [x] 2.5 `pictura-app`: filter commit grows by the stack's apron
- [x] 2.6 `pictura-app`: Move commit and arrow-key translate trim every moved
      pixel layer
- [x] 2.7 `pictura-app`: `fork_bytes` estimates the union rect; `add_layer_bytes`
      no longer charges the canvas for an empty layer
- [x] 2.8 `pictura-codec`: accept and emit a 0×0 layer record; keep existing
      files byte-identical

## Verification

- [x] 3.1 `cargo nextest run -p pictura-core -p pictura-paint -p pictura-render
      -p pictura-codec -p pictura_app` (only the pre-existing
      `imagemagick_fixtures_reproduce::ramp_Divide` failure remains)
- [x] 3.2 `ctest --test-dir build -R '^(tst_fill_tools|tst_move_feedback)$'`
- [x] 3.3 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` (the only failure is the pre-existing `ramp_Divide` oracle)
- [x] 3.4 `openspec validate grow-pixel-layers --strict`
- [x] 3.5 Update `docs/dev/STATE.md`, `ROADMAP.md` and
      `docs/dev/doc-codec-parity-gaps.md` (SEL-1) with `TASK-ALLOWS-DOCS`
