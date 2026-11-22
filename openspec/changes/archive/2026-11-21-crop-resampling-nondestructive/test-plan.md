# Test Plan — crop resampling and Reveal All

| Scenario | Steps | Expected result | Coverage |
|---|---|---|---|
| cr_mode_fields | Activate Crop, choose `W x H x Resolution` on a 40 x 24 document | W/H show 40/24, Resolution shows the document ppi, unit is visible | `tst_crop_options::resolutionModeShowsPixels` |
| cr_target_lock | In output-size mode type W=20/H=10, then drag a crop handle | Fields keep 20/10, the box stays at 2:1 and its pixel size may differ | `tst_crop_options::resolutionTargetSurvivesABoxDrag` |
| cr_swap | In output-size mode press Swap | W/H exchange and the box ratio inverts | `tst_crop_options::resolutionSwapExchangesTheTarget` |
| cr_clear_target | In output-size mode press Clear, then commit a crop | W/H blank, ratio unconstrained, no resample and no resolution write | `tst_crop_options::clearDropsTheResampleTarget` |
| cr_commit_resample | 40 x 24 document, output target 20 x 10 at 300 px/in, commit | Document is exactly 20 x 10 and reads 300 ppi | `tst_crop_options::resolutionCommitResamples` |
| cr_resolution_unit | Output target 300 with `px/cm`, commit | Document reads 300 pixels per centimetre | `tst_crop_options::resolutionCommitWritesPerCm` |
| cr_straighten_resample | Straighten commit with an output target | Result is the target size at the target resolution | `tst_crop_options::straightenCommitResamples` |
| ndc_retains | Commit a crop with Delete Cropped Pixels off | Layer rects extend past the canvas and planes keep the cropped pixels | `pictura-render::non_destructive_crop_keeps_the_raster` |
| ndc_trims | Commit a crop with Delete Cropped Pixels on | Layer rects, channels, and masks are trimmed to the canvas | `pictura-render::delete_cropped_pixels_trims_layers_and_masks_to_the_canvas` |
| ndc_reveal_all | Crop a 24 x 24 filled block to 5 x 4 non-destructively, run `Image > Reveal All` | Canvas is 24 x 24 and the block returns at (3, 2) unchanged | `tst_image_ops::revealAllRestoresCroppedPixels` |
| ndc_reveal_undo | Run Reveal All, then undo | One `Reveal All` state; undo returns to the cropped canvas | `tst_image_ops::revealAllIsOneUndoStep` |
| ndc_reveal_noop | Run Reveal All when all content is inside the canvas | Canvas and history unchanged | `tst_image_ops::revealAllWithNothingOutsideIsANoOp` |
| ndc_reveal_after_delete | Destructive crop, then Reveal All | Canvas stays at the crop size | `tst_image_ops::revealAllAfterDestructiveCropDoesNothing` |

## Manual checks

None: every scenario is automated.
