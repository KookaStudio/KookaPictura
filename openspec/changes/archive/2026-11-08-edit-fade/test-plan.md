# Test Plan — edit-fade

One row per scenario in the change's spec delta. Rows name a test this change
creates unless marked `(existing)`.

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| Opacity zero restores the original | Call `fade_layer` at opacity 0 on two layers with different pixels and rects | Color and alpha equal the original inside its rect and are transparent outside it | `pictura-filters::fade_opacity_zero_restores_the_original` |
| Opacity one hundred Normal returns the effect | Call `fade_layer` at opacity 100 Normal | The result is bit-identical to the effected layer | `pictura-filters::fade_opacity_one_hundred_normal_returns_the_effect` |
| Normal mode is a linear interpolation | Fade an opaque `(20,40,60)` pixel to `(220,140,100)` at 50 % Normal | The pixel is `(120,90,80)` ±1 | `pictura-filters::fade_normal_mode_is_a_linear_interpolation` |
| A blend mode uses its W3C function | Fade an opaque `(64,64,64)` pixel to `(192,192,192)` at 100 % Multiply | The pixel is `(48,48,48)` ±1 | `pictura-filters::fade_multiply_uses_its_w3c_function` |
| Alpha composites source-over | Fade a transparent original to an opaque effected pixel at 50 % Normal | The result is half-transparent and keeps the effected color | `pictura-filters::fade_alpha_composites_source_over` |
| Non-color channels come from the effect | Give the two layers different mask planes and fade at 50 % | The result's mask plane equals the effected layer's | `pictura-filters::fade_keeps_the_effected_non_color_channels` |
| The result covers both rectangles | Grow the effected layer past the original and fade at 50 % | The result rect is the union and the added pixels follow the opacity rule | `pictura-filters::fade_covers_the_union_of_both_rectangles` |
| A filter enables Fade | Commit Gaussian Blur on a pixel layer | `Edit ▸ Fade…` is enabled and its label names Gaussian Blur | `tst_fade::filterCommitEnablesFade` |
| Fade starts disabled | Open a fresh document | `Edit ▸ Fade…` is disabled | `tst_fade::fadeStartsDisabled` |
| Another edit disables Fade | Commit a filter, then a fill | Fade is enabled after the filter and disabled after the fill | `tst_fade::anotherEditDisablesFade` |
| Undo disables Fade | Commit a filter, then undo | Fade is disabled | `tst_fade::undoDisablesFade` |
| An adjustment enables Fade | Commit Invert on a pixel layer | Fade is enabled and names Invert | `tst_fade::adjustmentEnablesFade` |
| A paint or erase stroke enables Fade | Begin, dab, and end a brush stroke on a pixel layer | Fade is enabled and names Brush | `tst_fade::paintEnablesFade` |
| Mask and Smart Filter effects do not enable Fade | Commit an effect on a layer-mask target, in Quick Mask, and as a Smart Filter | Fade remains disabled | `pictura-app::a_mask_target_is_not_captured`, `tst_fade::maskAndSmartFilterDoNotEnableFade` |
| The dialog opens on the retained values | Apply a 40 % Multiply fade, then reopen Fade | Opacity 40, Mode Multiply, Preview checked | `tst_fade::dialogOpensOnRetainedValues` |
| Preview updates the canvas without history | Drag Opacity with Preview on | The canvas changes and the history count is unchanged | `tst_fade::previewUpdatesCanvasWithoutHistory` |
| Cancel restores the pre-dialog pixels | Change Opacity and cancel | The document is bit-identical to before the dialog and history is unchanged | `tst_fade::cancelRestoresPixels` |
| OK commits one Fade state | Accept the dialog | One `Fade <effect>` state is added and the canvas holds the faded result | `tst_fade::okCommitsOneState` |
| Reopening recomputes from the effect output | Fade at 50 %, reopen, fade at 25 % | The result is 25 % of the effect output, not 12.5 % | `tst_fade::reopeningRecomputesFromEffectOutput` |
| Lab disables the restricted modes | Open Fade on a Lab document | The six restricted modes are visible but disabled | `tst_fade::labDisablesRestrictedModes` |
| The command is implemented and dispatchable | Describe `edit.fade`, then dispatch it after a filter | It reports implemented and opens the dialog | `tst_fade::commandIsImplementedAndDispatchable` |
