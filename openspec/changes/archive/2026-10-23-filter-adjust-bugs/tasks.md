# Tasks

## 0. Artifacts

- [x] Write the capability deltas under `specs/imaging/` and keep `test-plan.md` one row per scenario.
- [x] Run `openspec validate filter-adjust-bugs --strict`.
- [x] Update `test-plan.md` and this list if the review changes a scenario or test name.

## 1. #855 proxy parameter scaling

- [x] Add `pixel_distance` to `FilterParam`, set it on every row (true for the listed controls, false otherwise), and expose it through `FilterParamRow.measures_pixels`.
- [x] Add `descriptor::scale_pixel_params`; use it in `filter_gallery_preview` and `filter_thumbnail`.
- [x] Use `FilterParamSpec::measuresPixels` in `FilterParamControls::scaledValues`; delete the suffix heuristic and update the `scaledValues` header comment.
- [x] Compare `measures_pixels` in `tst_filter_menu::rowsComeFromTheRustDescriptor`.
- [x] Add the failing regression tests: `tst_filter_gallery::previewScalesPixelParams`, `tst_filter_gallery::thumbnailScalesPixelParams`, `tst_filter_menu::scalesUnsuffixedPixelParams`.
- [x] Implement the gallery/thumbnail/document changes.
- [x] Update every `filter_thumbnail` caller with the new scale argument: the gallery catalog, Lens Flare, Lighting Effects, Reduce Noise, Picture Frame (picker and pad), and `tst_lighting_effects_dialog`.
- [x] Update the existing `tst_filter_menu::pixelParametersScaleWithTheCopy` Offset case to set non-zero H/V before scaling, because the new opening values are zero.

## 2. #856 whole-layer escalation

- [x] Set `whole_layer: true` on `radial-blur`, `clouds`, `difference-clouds`, and `fibers`.
- [x] Update the `filter_preview_needs_whole_layer` doc comment to name the new kinds.
- [x] Add the failing regression test: `pictura-app::section_preview_escalates_radial_clouds_fibers`.

## 3. #857 refused preview

- [x] Add the failing regression test: `tst_filter_menu::refusedPreviewRestoresCanvas`.
- [x] On refusal, cancel only the live preview owned by the call's target, recomposite, and return false in `filter_preview`, `filter_preview_section`, `filter_stack_preview`, `image_adjust_preview`, and the Replace Color/Match Color preview/apply wrappers (including Match Color's malformed-statistics refusal before `run`); keep `previewShown_` in sync. A refusal because another target owns the preview leaves that preview alive.
- [x] Recomposite in `filter_commit_finish` when the worker refuses or its join fails, and cancel/recomposite on `filter_commit_start` and `filter_stack_commit_start`'s synchronous refusals, so the restored layer is what the canvas shows.
- [x] Make `apply_mask_op` restore its baseline and clear the mask preview session when it refuses, preview or commit, so a previously successful mask preview cannot stay live.
- [ ] Add `pictura-app::adjustment_preview_refusal_recomposites` (including a Match Color malformed-statistics refusal) and extend `tst_filter_menu::refusedPreviewRestoresCanvas` with a layer-mask case and a refused-commit case.

## 4. #858 filter parameters

- [x] Add the failing regression tests: `tst_filter_menu::oilPaintShineRange`, `pictura-filters::offset_repeat_edge_pixels_clamps`, `pictura-app::offset_default_is_identity`.
- [x] Shine becomes `0..10`, default `5.5`.
- [x] Add `repeat` to `Filter::Offset` (default false), map the new descriptor slot (Repeat at 3, Background 4–6, Transparent at 7), default H/V to `0`, and clamp-to-edge in `other::offset`.
- [x] Update every `Filter::Offset` literal and the `filter/apply.rs` destructure with `repeat: false` (plus the direct `offset(...)` test calls), change the `filter_from_kind("offset")` default expectation to `0/0`, and fix `tst_filter_menu::pixelParametersScaleWithTheCopy`.
- [x] Extend `pictura-filters::offset_vertical_shift_and_zero_is_noop` to cover `repeat true` at 0/0 as well as the wrap and transparent modes.
- [x] Update the Offset oracle mapping note (and its row in `crates/pictura-filters/tests/README.md`) for the Repeat Edge Pixels case and add `pictura-filters::offset_mapping_documents_the_fill_and_repeat_cases`.

## 5. #859 Vibrance neutral gray

- [x] Add the failing regression test: `pictura-adjust::vibrance_neutral_pixel_stays_neutral`.
- [x] Guard `S == 0` in `color::vibrance` and `native::vibrance_native`; leave the `1 - S` falloff unchanged.

## 6. #860 Auto uses the selection

- [x] Add the failing regression test: `pictura-render::auto_tone_uses_selection_histogram` (Tone, Contrast, and Color cases).
- [x] Add `pictura_adjust::apply_with_coverage` (and re-export it); make the Auto histograms and neutral-midstone means count selected pixels only.
- [x] Build clip-local coverage from the selection `LayerMask` in `pictura_render::apply_op_region`, and use the selected pixel count (three times it for the joint histogram) as the histogram total.

## 7. #862 native-depth adjustment math

- [x] Add the failing regression tests: `pictura-adjust::native_exposure_linear`, `pictura-adjust::native_posterize_16bit`, `pictura-adjust::native_bw_channel_mixer`.
- [x] Use `Samples::depth()` to choose linear Exposure for `f32` and sRGB for `u8`/`u16`; the existing `Sample` conversion clamps an f32 input to `[0, 1]`, while `from_unit` leaves an output above 1.0 representable.
- [x] Quantize Posterize at the store's resolution, keeping `levels == 255` the identity at every depth.
- [x] Compute Black & White and Channel Mixer in the unit domain, clamp the B&W grey before the tint pass (rounding it to the 8-bit grid only for `u8`), and keep byte-identical u8 parity including the B&W tint path.

## Final

- [x] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] `bash scripts/verify-fast.sh`.
- [x] `openspec validate --all --strict`.
- [x] `openspec archive filter-adjust-bugs`.
- [ ] Rebase onto `master` after PR #879 (`#826`) merges.
- [ ] PR body lists the plan's manual rows as a reviewer checklist.
