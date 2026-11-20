# Tasks

## 1. Tests first (write only; compile after group 3)

- [x] 1.1 Add `tst_transform_options::xySliderStepsAboutHundredPixels`.

## 2. Implementation

- [x] 2.1 Add `NumericFieldConfig::sliderSpan` and the narrowed slider range,
  single step, and page step in `NumericField`.
- [x] 2.2 Configure the X/Y fields with `sliderSpan = 20000`.

## 3. Verify

- [x] 3.1 `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] 3.2 Build and run `tst_transform_options` plus the other transform suites.
- [x] 3.3 `python3 scripts/check-test-plan.py` and `openspec validate --all --strict`.
- [x] 3.4 Archive the change with `openspec archive transform-xy-slider-span`.
