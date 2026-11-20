# Tasks

## 1. Tests first (write only; compile after group 4)

- [x] 1.1 Add the geometry units `translate_to_moves_a_similarity_quad`,
  `translate_to_moves_a_stored_quad`, and `pivot_preset_finds_the_reference_anchor`.
- [x] 1.2 Extend `tst_transform_options` with `xAndYTranslateTheTarget`,
  `widthSliderIsSplitAtHundredPercent`, and the locator-highlight assertions;
  update `relativePositioningOffsetsFromTheOrigin` to the fixed origin.

## 2. Transform geometry and bridge

- [x] 2.1 Add `geometry::translate_to` and `geometry::pivot_preset`.
- [x] 2.2 Expose `transform_set_position` and `transform_pivot_preset` through
  the transform bridge; make the relative origin fixed.

## 3. Options page and numeric field

- [x] 3.1 Highlight the active locator cell in `refreshTransformPage`.
- [x] 3.2 Wire X/Y to `ToolController::setTransformPosition`.
- [x] 3.3 Add the split slider mapping to `NumericField` and configure W/H for
  1000% with a 100% midpoint.

## 4. Verify

- [x] 4.1 `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 4.2 Build and run `tst_transform_options`, `tst_transform_selection`, and
  `tst_transform_controls`.
- [x] 4.3 `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`.
- [x] 4.4 Archive the change with `openspec archive transform-pivot-position-slider`.
