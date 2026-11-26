# Test plan

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| cs_units_picas | Open at 300 ppi with Traditional points, switch both axes to Picas and edit width to six picas. | The original dimensions survive the switch and six picas resolve to 299 px. | tst_image_ops::canvasSizePicasUseResolution |
| cs_units_columns | Save a width of 1 in and gutter of 0.25 in, then enter two columns for width and half a column for height at 300 ppi. | Width is 675 px and height is 150 px. | tst_image_ops::canvasSizeAbsoluteColumnsUseGutter |
| cs_units_relative | Select Columns, toggle Relative, enter +1 and −1 at 300 ppi with the configured 1.25 in pitch. | Zero initially preserves 600 × 500 and the edited result is 975 × 125. | tst_image_ops::canvasSizeRelativeColumnsUsePitch |
| cs_units_large_restatement | Create a 10000-pixel-wide document at 0.1 ppi with a 0.000001-inch column and no gutter; switch width to Picas and Columns, returning to Pixels after each. | Both unit values exceed 300000 and width remains 10000 px throughout. | tst_image_ops::canvasSizeLargeUnitRestatementPreservesPixels |
| cs_fill_background | Open an opaque PNG, assert that the menu and swatch are enabled, select Foreground and grow from the center. | Both controls are enabled, and samples in the border have the foreground color. | tst_image_ops::canvasSizeGrowsAroundTheAnchor |
| cs_fill_no_background | Create an opaque ordinary layer, open Canvas Size, then grow with a bright supplied fill. | Both controls are disabled and the new border has alpha zero. | tst_image_ops::canvasSizeWithoutBackgroundGrowsTransparently |
| cs_fill_nonpixel_background | Construct a top-level non-pixel layer with the Background flag and inspect the shared predicate used by both dialog and transaction. | It is eligible for extension fill; an unflagged or grouped layer is not. | pictura-app::canvas_background_predicate_uses_flag_for_non_pixel_layers |
| cs_fill_merged_only | Open a minimal layerless PSD with a merged RGB composite, invoke the resize transaction with a bright fill, and inspect the composite. | Original pixels remain at the anchor offset; an uncovered border pixel has alpha zero. | tst_image_ops::canvasSizeMergedOnlyGrowsTransparently |
| cs_fill_retained | Create an ordinary layer extending beyond the canvas, then grow to reveal it and an uncovered area. | The exposed layer pixel is preserved and visible; an uncovered pixel has alpha zero. | tst_image_ops::canvasSizeRevealsRetainedPixels |
