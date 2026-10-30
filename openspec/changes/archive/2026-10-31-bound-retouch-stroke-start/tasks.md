# Tasks

## 1. Measurement harness

- [x] 1.1 Add `crates/pictura-app/src/cxxqt_object/tests_profiles_retouch.rs`
  with an ignored `retouch_stroke_start_profile` at 8000×6000 that measures the
  public engine path directly, because `begin_*` in `paint_tools.rs` are private
  to that module and take `Pin<&mut PictureView>`:
  - plain Brush control: `Stroke::begin_at`
  - Dodge/Burn/Sponge: `Stroke::begin_tone`
  - Smudge/Blur/Sharpen with Sample All: `Stroke::begin_smudge` /
    `begin_focus` with a layer-local sampled surface built from
    `pictura_render::composite_rgba` + `surface_from_composite_rect`
  - Clone Stamp All Layers: `sample_scope` + `composite_rgba` +
    `surface_from_composite` + `Stroke::begin_source`
  - History Brush: `layer_surface` + `Stroke::begin_source`
  - Pattern Stamp: `StampSource::pattern` + `Stroke::begin_source`

  Time each phase with `Instant` and `println!` like `tests_profiles.rs` (do not
  rely on `paint_timing::level()`, a process-wide `OnceLock`); optionally wrap
  phases in `paint_timing` and run with `PICTURA_PAINT_TIMING=1`. Declare the
  module in `cxxqt_object.rs`. Verify
  `cargo test -p pictura_app --release --lib retouch_stroke_start_profile --
  --ignored --nocapture` prints one line per tool.
- [x] 1.2 Run the profile on the unchanged builders and record the baseline
  numbers in `design.md` under a Measurements heading. This is an explicit
  measurement exception to the COMPILE-LAST rule: add the profile, build once,
  and capture the baseline before changing the builders (or capture it from a
  clean checkout with the profile applied). Verify the numbers exist for all six
  tools. 8000×6000 keeps the profile cheap; the 267 MP figure is the
  `ips_first_dab` measurement, not this profile.

## 2. Layer source builder

- [x] 2.1 Move `layer_rgba` from `stroke.rs` into a new
  `crates/pictura-paint/src/stroke/surface.rs`, rewrite it as one parallel row
  pass that resolves the channel indices once, reads each row through
  `Plane::row`, returns an empty image when width or height is zero, and is
  re-exported `pub(crate)` from `stroke.rs`. Verify
  `cargo nextest run -p pictura-paint` and that `stroke.rs` is under 1200 lines.
- [x] 2.2 Add a serial `layer_rgba` oracle and the
  `layer_rgba_matches_serial_builder` test in a new
  `crates/pictura-paint/src/stroke/tests/surface.rs` (declared from
  `stroke/tests.rs`) covering a full-document layer, an offset/partial layer, a
  Background without alpha, a missing channel, and a zero-size layer. Verify the
  parity test passes.

## 3. Composite and history surface builders

- [x] 3.1 Rewrite `surface_from_composite` as one parallel row pass and add a
  `pub` `surface_from_composite_rect` for a layer-local crop; both return an
  empty image for a zero-size source. Add a serial oracle and the
  `surface_from_composite_matches_serial_builder` test in `stamp.rs`. Verify the
  test passes.
- [x] 3.2 Rewrite `layer_surface` to interleave directly from the layer planes
  without the intermediate `layer_rgba` image and to return an empty image for a
  zero-size document; add a serial oracle and the
  `layer_surface_matches_serial_builder` test for a moved layer, a rect hanging
  off the document, and a zero-size document. Verify the test passes.
- [x] 3.3 Rewrite `layer_local` as one parallel row pass that returns an empty
  image for a zero-size source; add a serial oracle plus the
  `sampled_surface_matches_two_pass_crop` test for a rect hanging off the source
  and a zero-size rect, and a `zero_size_sources_are_empty` test covering all
  four builders. Verify the tests pass.

## 4. Sample All Layers fold

- [x] 4.1 Change `composite_surface` in `paint_tools.rs` to build the
  layer-local source with `surface_from_composite_rect`, resolve the layer
  rectangle from the target path in `begin_focus`, `begin_smudge`, and
  `begin_mixer`, and make `Stroke::begin_focus` / `begin_smudge` / `begin_mixer`
  accept an already layer-local `sampled` surface, and update
  `tests_profiles_retouch.rs` to build the Sample All source with
  `surface_from_composite_rect`. Verify `ss_sample_all_parity` and the existing
  Blur/Sharpen/Smudge/Mixer tests pass, and that `paint_tools.rs` is still under
  its 1200-line cap (1158 before this task).
- [x] 4.2 Update the `sampled` doc comments and any tests that built a
  document-space surface. Verify the crate's tests compile and pass.

## 5. Pattern Stamp lazy source

- [x] 5.1 Change `StampSource` to an `Image` / `Pattern` enum holding
  `Arc<RgbaImage>`, add
  `StampSource::pattern(tile, origin, width, height) -> Option<StampSource>`
  that returns `None` for a zero-size tile and otherwise samples with
  `rem_euclid` inside the document bounds in `at()`, switch `begin_pattern_stamp`
  to it (keeping its refusal), and keep `tiled` as a test-only oracle. Verify the
  existing Pattern Stamp test and the `pattern_source_matches_tiled_image` test
  for aligned, unaligned, negative origins, an off-canvas layer, and a zero-size
  tile pass, and that `paint_tools.rs` stays under its 1200-line cap (1158 before
  this task).
- [x] 5.2 Update `StampSource` callers and tests for the `impl
  Into<Arc<RgbaImage>>` constructor shape; `Arc<RgbaImage>: From<RgbaImage>`
  keeps existing callers source-compatible. Replace the removed `.transform`
  field read (`stamp.rs:438-439`) with a `transform()` accessor or `matches!`.
  Verify `cargo nextest run -p pictura-paint -p pictura-app` passes and that
  `history.rs` and `paint_tools.rs` stay under their caps after the edits.

## 6. History source cache

- [x] 6.1 Key the history cache on the resolved source id instead of a
  generation: add `id: u64` to `Named` (assign it from `next_id` and increment
  the counter), add `pinned_id: Option<u64>` to `History` and record it in
  `pin_source`, add a `pub(crate) brush_source_key()`, and add a
  `pub(crate) brush_source_surface(path)` backed by
  `RefCell<Option<((u64, String), Arc<RgbaImage>)>>`. Resolve the key before
  `materialize` and only build on a miss. Put the tests in a new
  `crates/pictura-app/src/history/tests/cache.rs` declared from
  `history/tests.rs` (which is 1344/1400 lines):
  `brush_source_surface_is_reused`,
  `brush_source_surface_reuses_a_pinned_source`,
  `brush_source_surface_reuses_a_source_dropped_by_depth`, and
  `brush_source_surface_invalidates_on_history_change`. Verify the tests pass.
- [x] 6.2 Use `brush_source_surface` in `begin_history_brush`,
  `begin_art_history_brush`, and the Erase to History path in `begin_eraser`,
  and make `Stroke::begin_art_history` accept the
  shared surface; update its callers and tests in
  `crates/pictura-paint/src/art_history.rs` (lines ~362 and ~442 pass an
  `RgbaImage`). Art History still crops the cached document-space surface with
  `layer_local` (one parallel crop pass); the cache saves the document-space
  build, not the crop. Verify the existing History Brush and Art History Brush
  tests pass, and that `history.rs` and `paint_tools.rs` stay under their caps
  after the edits.

## 7. Verification

- [x] 7.1 Re-run `retouch_stroke_start_profile` and record the after numbers in
  `design.md`; verify every tool's stroke start dropped and the spec's reported
  scenarios are covered by a named test.
- [ ] 7.2 Run `cargo fmt --all`, `cargo clippy --workspace --all-targets --
  -D warnings`, `cargo nextest run --workspace`, and `cargo test --workspace
  --doc`. Verify all pass.
- [ ] 7.3 Run `openspec validate --all --strict` and
  `bash scripts/verify-fast.sh`. Verify both pass, including
  `scripts/check-test-plan.py` and the file-size guard.
- [ ] 7.4 Add the `docs/dev/STATE.md` entry and commit it separately with
  `TASK-ALLOWS-DOCS`. Verify `scripts/guard.sh` accepts the commit.
- [ ] 7.5 Archive the change with `openspec archive bound-retouch-stroke-start`,
  file the tile-backed follow-up issue, and open the PR. Verify
  `scripts/check-openspec-archived.sh` passes.
