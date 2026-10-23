# Tasks

## 1. #863 Pencil Auto Erase leak

- [x] Add the failing regression test: `tst_options_bars_parity::pencilAutoEraseDoesNotLeakToBrush`
- [x] Guard the `beginBrush` auto-erase argument with `aliased_ && paint_->autoErase()`; leave the rasterisation argument unchanged.
- [x] Run only that test, then the owning suite.

## 2. #864 healing mask target

- [x] Add the failing regression test: `tst_mask_edit::healingRefusesMaskTarget`
- [x] Refuse `begin_heal` while `active_mask_target` is set, include `pictura_app/src/cxxqt_object/impl_layers/layer_masks.cxxqt.h` in `tool_healing.cpp`, and name the mask target in `refuseHeal`.
- [x] Run only that test, then the owning suite.

## 3. #865 phantom history and stale RGB

- [x] Add the failing regression tests: `pictura-paint::noop_stroke_adds_no_history`, `pictura-paint::clear_zeroes_rgb`, `pictura-paint::clear_keeps_rgb_below_full`
- [x] Update `behind_leaves_opaque_rgb_unchanged` to expect no dirty rectangle (`finish()` returns `None`); that is what keeps history empty.
- [x] Make `Stencil::composite` report byte changes and grow the dirty box only for changed pixels.
- [x] Compare written pixels in `apply_per_dab` and grow its dirty box only for changed bytes.
- [x] Zero RGB from `blend_pixel`'s Clear branch at full coverage and write RGB planes for Clear on a layer with alpha; a layer without alpha stays a no-op.
- [x] Run only those tests, then `pictura-paint`.

## 4. #866 Mixer Mode/Opacity

- [x] Add the failing regression tests: `pictura-paint::mixer_applies_mode_opacity`, `tst_options_bars_parity::mixerExposesModeOpacity`
- [x] Pass Mode and Opacity from `tool_mixerbrush.cpp` into `begin_mixer_brush` and composite the dab through `blend_pixel`.
- [x] Add `optionsMixerMode` and `optionsMixerOpacity` to the Mixer options page.
- [x] Run only those tests, then the owning suites.

## 5. #867 Contract/Border at the edge

- [x] Add the failing regression tests: `pictura-select::contract_all_shrinks`, `pictura-select::border_edge_has_inner_band`
- [x] Treat out-of-canvas morph taps as zero in both passes.
- [x] Run only those tests, then `pictura-select`.

## 6. #868 Quick Selection stroke

- [x] Add the failing regression tests: `tst_tool_commit::quickSelectionOneStatePerStroke`, `tst_tool_commit::quickSelectionCancelRestores`
- [x] Add `quick_select_preview` and `quick_select_commit`; keep `apply_selection` as the only combine path.
- [x] Make a New-mode drag replace with its first dab and extend the stroke's accumulated selection with later dabs; after a New-mode first selection, switch the selection options group to Add.
- [x] Snapshot the selection at press and restore it from `onSessionCancelled` and `onDeactivate`.
- [x] Record one "Selection" state on release only when a dab applied.
- [x] Run only that test, then the owning suite.

## 7. #869 Refine Edge units/scale

- [x] Add the failing regression tests: `pictura-select::refine_distance_is_pixels`, `pictura-app::preview_params_scale_with_the_proxy`, `tst_refine_edge::previewMatchesCommitWithinProxyTolerance`
- [x] Divide the chamfer distance by 3.
- [x] Scale Radius, Smooth, and Feather by the preview proxy's linear factor in the preview path, and compare preview and commit at proxy scale.
- [x] Run only those tests, then the owning suites.

## 8. #870 path Baseline Shift

- [x] Add the failing regression test: `pictura-render::type_on_path_applies_baseline_shift`
- [x] Offset `layout_path` glyphs and `caret_stops` by `-rise * normal`.
- [x] Run only that test, then `pictura-render`.

## Final

- [x] `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `bash scripts/verify-fast.sh`
- [x] `openspec validate --all --strict`
- [x] `openspec archive tool-bugs`
- [x] PR body notes that the test plan has no manual rows.
