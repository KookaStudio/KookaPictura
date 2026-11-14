# Tasks

## 1. Fixture

- [x] 1.1 Add a `derived_raster_mask()` builder and its `FIXTURES` entry to
  `scripts/generate-fixtures.py`: an 8x8 RGB pixel layer with record bit 4
  clear, a painted `-2` mask whose `user_mask_from_render` flag is set, a
  closed-rectangle `vmsk`, and no `-3` channel. Run
  `python3 scripts/generate-fixtures.py` and confirm
  `git status --porcelain crates/pictura-codec/tests/fixtures` shows the new
  `derived_raster_mask.psd` and no modified existing fixture.
- [x] 1.2 Add the fixture's row and authored-by paragraph to
  `crates/pictura-codec/tests/fixtures/README.md`; verify the fixture name is
  listed once and the described flags match the builder.

## 2. Tests (written before the implementation)

- [x] 2.1 Add the four predicate unit tests to
  `crates/pictura-codec/src/vector_mask.rs`:
  `a_rendered_source_path_does_not_become_a_vector_mask`,
  `a_pixel_data_irrelevant_layer_keeps_its_outline`,
  `a_real_user_mask_channel_keeps_the_vector_view`, and
  `an_ordinary_raster_mask_keeps_the_vector_view`. Verify the four names exist
  with `grep -n "fn a_rendered_source_path" crates/pictura-codec/src/vector_mask.rs`
  and the test-plan's Coverage column.
- [x] 2.2 Add the integration checks to
  `crates/pictura-codec/tests/vector_mask_oracle.rs`:
  `codec_suppresses_a_rasterized_source_path` (read the fixture; raster mask
  present, vector view absent, raw block preserved, write/read equals the
  input), `codec_preserves_vector_mask_blocks` (round-trip `vector_mask.psd`
  equals the input), and `psd_tools_reads_the_derived_raster_mask_fixture`
  (self-skipping when psd-tools is absent). Verify the three names exist.

## 3. Implementation

- [x] 3.1 Add `FLAG_USER_MASK_FROM_RENDER: u8 = 0x08` beside the other mask
  flags in `crates/pictura-codec/src/mask_params.rs`; verify the constant is
  referenced by the predicate added in 3.2 and the codec still builds.
- [x] 3.2 In `resolve_vector_masks`, skip the `vmsk` decode and the
  density/feather placeholder merge when the layer record bit 4 is clear, the
  raster mask flags have bit 3 set, and `raw_channels` has no id `-3`. Leave a
  `ponytail:` comment naming the ceiling: a suppressed path's vector
  density/feather is normalized on save because the model has no carrier for
  it. Verify `cargo test -p pictura-codec vector_mask` passes, including the
  four unit tests from 2.1.

## 4. Integration checks

- [x] 4.1 Run `cargo test -p pictura-codec` and confirm every test in
  `test-plan.md` passes, including the fixture round trip and the self-skipping
  psd-tools check.
- [x] 4.2 Run `cargo fmt --all`,
  `cargo clippy -p pictura-codec --all-targets -- -D warnings`, and
  `bash scripts/verify-fast.sh`; all must pass with a clean tree apart from the
  change.
- [x] 4.3 Re-run `python3 scripts/generate-fixtures.py` and confirm no tracked
  fixture changes; this proves the generator is deterministic. Two temp-dir runs
  were byte-identical. The pre-existing psd-tools 1.24 vs 1.19 version-string
  drift in `pattern_fill_masked.psd` is restored and not committed.
- [x] 4.4 Archive the change with `openspec archive fix-spurious-vector-masks`
  in the implementation PR and confirm `openspec validate --all --strict`
  passes.

## 5. Review follow-up

- [x] 5.1 Clear the raster mask's render flag on save when a pixel layer carries
  a live vector mask, so an authored or edited mask survives a reload; cover the
  authored and no-`-3` save round trips with tests.
- [x] 5.2 Add the render composite check, the non-default mask-parameter fixture
  and assertions, and the parseable-block assertion; regenerate the fixture.
- [x] 5.3 Clarify the opaque-preservation clause and the derived-view writer
  behavior in the main spec.
