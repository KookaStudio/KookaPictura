# Test plan

All rows are automated; no manual reviewer checks.

| Scenario | Steps | Expected result | Coverage |
| --- | --- | --- | --- |
| `mtp_portrait` | portrait and landscape PSD masks are opened at each thumbnail size | their source thumbnails are opaque and their document-aspect cards contain grayscale coverage through every interior edge, with a black outline and no checkerboard strip, including at fractional display scale | `tst_canvas_mask_domain::importedPortraitMaskCardIsOpaque` |
| `mtp_square` | mask previews are displayed on adjustment or fill rows in a portrait document | the cards remain square and their document-aspect previews remain fitted within those cards | `tst_layers_panel_fixes::maskThumbnailKeepsDocumentAspectOnSquareRows` |
| `lmk_row_link` | a row whose mask is linked is shown | a link glyph is drawn between the layer and mask thumbnails | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `lmk_row_disabled` | a row whose mask is disabled is shown | a red cross is drawn over the mask thumbnail | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `lmk_row_link_click` | the user clicks the link glyph on a linked mask row | the mask is unlinked in one undoable step | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `lmk_row_shift` | the user `Shift`-clicks the mask thumbnail of an enabled mask | the mask is disabled in one undoable step | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `lmk_row_shift_solo` | a mask is soloed and the user `Shift`-clicks its thumbnail to toggle its enabled state | the mask's enabled state toggles and the canvas still shows the mask plane | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `lmk_row_solo_exit` | a mask is soloed and the user selects a different layer row | the solo view ends and the canvas shows the composite | `tst_mask_edit::maskSoloFollowsLayerChange` |
| `lmk_row_activate` | the user clicks the mask thumbnail of a row that carries a mask | that mask becomes the mask edit target and the thumbnail draws white corner brackets and a black outline, with no history state | `tst_canvas_mask_domain::maskThumbnailUsesOnlyBrackets` |
| `lmk_row_deactivate` | a mask is the edit target and the user clicks that row's layer thumbnail | the mask edit target is cleared and the mask brackets disappear and the image brackets become active | `tst_canvas_mask_domain::maskBracketsFollowRasterAndLayerClicks` |
| `lmk_row_alt_solo` | the user `Alt`-clicks the mask thumbnail of a raster-mask row | the row is selected, the mask becomes the edit target with its white corner brackets, and the canvas shows the mask plane, with no history state | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `lmk_row_alt_restore` | the user `Alt`-clicks the same mask thumbnail again while it is soloed | the composite is shown again and the mask stays the edit target | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `lmk_row_alt_shift` | the user `Alt`+`Shift`-clicks a mask thumbnail | neither the solo view nor the enabled-state toggle takes effect | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `vmk_row_vector` | a row whose vector mask is linked and enabled is shown | a vector-mask thumbnail is drawn alongside the layer thumbnail with its link glyph | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `vmk_row_vector_disabled` | a row whose vector mask is disabled is shown | a red cross is drawn over the vector-mask thumbnail | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `vmk_row_vector_link_click` | the user clicks the vector-mask link glyph on a linked row | the vector mask is unlinked in one undoable step | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `vmk_row_vector_shift` | the user `Shift`-clicks the vector-mask thumbnail of an enabled vector mask | the vector mask is disabled in one undoable step | `tst_canvas_mask_domain::maskModifierIndicatorsRemain` |
| `mtp_black_outline` | a raster mask thumbnail on a pixel layer or group is selected | its card has the normal black outline and white corner brackets, without an additional white rectangular outline | `tst_canvas_mask_domain::maskThumbnailUsesOnlyBrackets` |
| `mtp_other_layer` | a mask thumbnail is selected and the user clicks another layer's raster thumbnail | only the newly active raster thumbnail has white brackets and the old mask retains only its black outline | `tst_canvas_mask_domain::maskBracketsFollowRasterAndLayerClicks` |
