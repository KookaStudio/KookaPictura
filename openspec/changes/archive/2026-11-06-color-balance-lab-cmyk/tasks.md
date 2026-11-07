# Tasks: color-balance-lab-cmyk

## 1. Tests first

- [x] 1.1 `pictura-adjust::color_balance_rgb_mode_is_the_default`
- [x] 1.2 `pictura-adjust::color_balance_lab_shifts_a_and_b`
- [x] 1.3 `pictura-adjust::color_balance_cmyk_lowers_the_named_ink`
- [x] 1.4 `pictura-adjust::color_balance_other_adjustments_ignore_the_mode`
- [x] 1.5 `pictura-adjust::color_balance_lab_mode_matches_the_eight_bit_shift`
- [x] 1.6 `tst_image_adjustments::colorBalanceReadoutFollowsModeAndTone`

## 2. Colour model

- [x] 2.1 Add f64 unit `srgb_unit_to_lab` / `lab_to_srgb_unit` to
      `pictura-color/src/model.rs`, with the byte conversions delegating to them.
- [x] 2.2 Add the `pictura-color` workspace dependency to `pictura-adjust`.

## 3. Engine

- [x] 3.1 `ColorBalanceMode { Rgb, Lab, Cmyk }` in `pictura-adjust/src/types.rs`,
      re-exported from the crate root.
- [x] 3.2 `apply_in_mode` / `apply_with_coverage_in_mode` and the
      `apply_native_in_mode` twin; the mode-less functions delegate with `Rgb`.
- [x] 3.3 Factor `band_weights` / `band_delta` and add the Lab and CMYK kernels
      to `color.rs`; the RGB arithmetic keeps its operation order.
- [x] 3.4 Mirror the kernels in `native.rs`.
- [x] 3.5 Profile-free RGB↔CMYK unit helpers (`ponytail:` the no-black model).

## 4. Render

- [x] 4.1 `apply_adjustment_region` takes a `ColorBalanceMode`; `Op::Adjustment`
      carries it to the working buffer.
- [x] 4.2 `composite_adjustment` derives the mode from
      `document_color_mode(doc)` for both the native and 8-bit paths.

## 5. App

- [x] 5.1 Pass the document mode in `image_adjust/jobs/imp.rs`,
      `impl_filters.rs` (destructive path), and `filter_tools/jobs/imp.rs`;
      the mask target stays RGB.
- [x] 5.2 `ColorBalanceDialog`: read `channels_mode`, add the
      `colorBalanceReadout` label, and update it from the tone selector and the
      three spin boxes.

## 6. Docs

- [x] 6.1 Tick the roadmap item (#572).
- [x] 6.2 Close parity gap ADJ-2 in `docs/dev/doc-code-parity-gaps.md`.
- [x] 6.3 Add the resume note to `docs/dev/STATE.md`.

## 7. Verification

- [x] 7.1 `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`
- [x] 7.2 `cargo nextest run -p pictura-adjust -p pictura-render -p pictura-color`
      (the unrelated `pictura-render::oracle::imagemagick_fixtures_reproduce`
      fails on `ramp_Divide.rgba`; the same failure reproduces on the base
      commit with the local ImageMagick 7.1.2-27)
- [x] 7.3 `ctest --test-dir build -R '^tst_image_adjustments$' --output-on-failure`
- [x] 7.4 `openspec validate --all --strict`
- [x] 7.5 `bash scripts/verify-fast.sh` (same pre-existing oracle failure only)
