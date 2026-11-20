# Tasks

## 1. Tests first (write only; compile after group 6)

- [x] 1.1 Add the `pictura-select` tests named in `test-plan.md`:
  `every_sample_contributes_its_best_match`,
  `localized_clusters_fall_off_with_range`,
  `localized_ignores_a_positionless_sample`,
  `skin_tones_ignore_non_skin_colours`, and
  `transparent_pixels_take_no_coverage`; keep the band, fuzziness, and index
  tests. Do not build yet.
- [x] 1.2 Add the `pictura-core` tests for the moved match kernel and the
  `spatial_sigma` / `localization_strength` helpers. Do not build yet.
- [x] 1.3 Add `pictura-color::gamut_check_marks_unprintable_colours` plus its
  equivalence test, and the `pictura-app` tests
  `color_range_gamut_follows_the_proof_setup`,
  `color_range_gamut_uses_the_working_profile`, and
  `localized_samples_stay_in_document_space`. Do not build yet.
- [x] 1.4 Extend `tst_color_range` with `samplesAddAndSubtract`,
  `localizedClustersNarrowThePreview`, `skinTonesSelectSkin`,
  `outOfGamutFollowsTheProofSetup`, `selectionAndImagePreviews`,
  `ctrlTogglesTheDisplayPreview`, `imageWindowPreviewModes`,
  `previewIgnoresProofColors`, `saveLoadRoundTrip`, `saveLoadRejectsInvalid`,
  and `previewReflectsTheLiveSelection`; update `sampleAndSelect` and
  `refinesALiveSelection`; keep the suite registered in
  `crates/pictura-app/cpp/tests/CMakeLists.txt`. Do not build yet.

## 2. Shared match kernel

- [x] 2.1 Move `match_weight` and `sigma_sq` to `pictura-core`, generalising
  the spatial term to `1 - localization * (1 - gaussian)` and sigma to a
  diagonal fraction; verify with `cargo test -p pictura-core`.
- [x] 2.2 Rewire Replace Color to the shared helper at `localization` 1.0
  (localized) or 0.0 (not) and its 0.18 fraction; verify with
  `cargo test -p pictura-adjust replace_color` and `cargo test -p pictura-core`.
- [x] 2.3 Add `spatial_sigma(diagonal_px) = 0.18 * diagonal_px` and
  `localization_strength(range) = 1 - range / 100`; verify with
  `cargo test -p pictura-core`.

## 3. Color Range coverage

- [x] 3.1 Add `SkinTones` only to `ColorRangeSelect` (Out Of Gamut stays
  app-side); update `from_index`; verify with
  `cargo test -p pictura-select color_range`.
- [x] 3.2 Rewrite `color_range` around `ReplaceColorSample`, the shared
  best-match model, localization strength, Skin Tones, and the alpha-zero rule;
  verify with `cargo test -p pictura-select color_range`.
- [x] 3.3 Keep the hue and tonal bands and Invert behaviour unchanged; verify
  with `cargo test -p pictura-select color_range`.

## 4. Gamut check

- [x] 4.1 Add `GamutCheck` to `pictura-color` and make
  `print_gamut_mask_source` delegate to it; verify with
  `cargo test -p pictura-color`.
- [x] 4.2 Add the RGB-only availability predicate, the working-profile source
  (falling back to sRGB), the shared `ProofState -> ProofSetup` mapping, and
  the banded gamut mask helper in the app bridge; verify with
  `cargo test -p pictura_app color_range`.

## 5. Bridge

- [x] 5.1 Parse the `x,y,r,g,b;…` sample text, translate each band's sample
  `y` by the `stream_picture` offset, pass the document-derived sigma, and
  return a bounded composite for the Image thumbnail; verify with
  `cargo test -p pictura_app localized_samples_stay_in_document_space` and the
  parser test.
- [x] 5.2 Change `color_range_preview` and `color_range_apply` to take the
  settings, intersect a live selection in the preview, and apply the alpha
  rule; verify with `cargo test -p pictura_app color_range`.
- [x] 5.3 Add the display-only canvas preview state and the
  `canvas_present_image` branch for None, Grayscale, Black Matte, White Matte,
  and Quick Mask, matching the unproofed composite, with a clear entry point;
  covered by `tst_color_range::imageWindowPreviewModes` and
  `tst_color_range::previewIgnoresProofColors` in 6.5.
- [x] 5.4 Map the dialog's Out Of Gamut index before `ColorRangeSelect` and add
  `color_range_out_of_gamut_available`; verify with
  `cargo test -p pictura_app color_range`.

## 6. Dialog

- [x] 6.1 Add the Select items in CS6 order (Skin Tones after Sampled Colors,
  Out Of Gamut last and only on RGB), the three droppers, the
  modifier-carrying `setCanvasSampler` callback in `tools.h` / `tools.cpp`
  (including Replace Color's call site), Localized Color Clusters with Range,
  and the no-sample state for Skin Tones and Out Of Gamut; covered by
  `tst_color_range::samplesAddAndSubtract` and
  `tst_color_range::skinTonesSelectSkin` in 6.5.
- [x] 6.2 Add the Selection/Image display radios with the bounded preview and
  a refresh only on a mode change, the Ctrl/Cmd toggle, the Range enabled
  state, and the Selection Preview combo; covered by
  `tst_color_range::selectionAndImagePreviews`,
  `tst_color_range::ctrlTogglesTheDisplayPreview`, and
  `tst_color_range::imageWindowPreviewModes` in 6.5.
- [x] 6.3 Add Save/Load through `saveSettings(path)` / `loadSettings(path)`
  with format/version validation, clamping, and the unavailable-mode refusal,
  plus the file dialog; covered by `tst_color_range::saveLoadRoundTrip` and
  `tst_color_range::saveLoadRejectsInvalid` in 6.5.
- [x] 6.4 Clear the canvas preview and sampler on close, and pass the settings
  from `menu_select.cpp` on accept; covered by
  `tst_color_range::sampleAndSelect`,
  `tst_color_range::refinesALiveSelection`, and
  `tst_color_range::localizedClustersNarrowThePreview` in 6.5.
- [x] 6.5 Build and run the suite: `cmake --build build --target tst_color_range`
  and `QT_QPA_PLATFORM=offscreen python3 scripts/cpu-budget.py exec -- ctest
  --test-dir build -R '^tst_color_range$' --output-on-failure`; every case
  passes.

## 7. Close-out

- [x] 7.1 Update `docs/dev/doc-code-parity-gaps.md`, `docs/dev/STATE.md`, and
  `ROADMAP.md`, recording the remaining ceilings (Detect Faces, Alt-click
  Reset, `.axt` import, Bitmap/Indexed availability, Grayscale behaviour,
  Properties-panel layer-mask refinement, the no-match warning, and per-session
  persistence); verify `bash scripts/guard.sh` passes with `TASK-ALLOWS-DOCS`
  recorded in the commit.
- [x] 7.2 Run `python3 scripts/check-test-plan.py`,
  `openspec validate --all --strict`, `cargo fmt --all`,
  `cargo clippy -p pictura-select -p pictura-core -p pictura-color -p pictura-adjust -p pictura_app --all-targets -- -D warnings`,
  and the targeted Rust tests.
- [x] 7.3 Archive with `openspec archive colour-range-extras` and verify
  `bash scripts/check-openspec-archived.sh` passes.
