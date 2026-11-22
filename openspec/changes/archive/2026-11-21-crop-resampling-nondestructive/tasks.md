# Tasks

## Tests

- [x] Add `pictura-render::document_ops::crop` unit tests for `reveal_all`: it
  restores a non-destructively cropped raster, includes group/mask/store
  content, and is a no-op when everything is inside the canvas.
- [x] Add `tst_crop_options::resolutionCommitResamples`,
  `tst_crop_options::resolutionCommitWritesPerCm`,
  `tst_crop_options::straightenCommitResamples`,
  `tst_crop_options::resolutionTargetSurvivesABoxDrag`,
  `tst_crop_options::resolutionSwapExchangesTheTarget`, and
  `tst_crop_options::clearDropsTheResampleTarget`.
- [x] Add `tst_image_ops::revealAllRestoresCroppedPixels`,
  `tst_image_ops::revealAllIsOneUndoStep`,
  `tst_image_ops::revealAllWithNothingOutsideIsANoOp`, and
  `tst_image_ops::revealAllAfterDestructiveCropDoesNothing`.

## Engine

- [x] Add `pictura_render::reveal_all` over the canvas/layer content union and
  export it from `document_ops`.
- [x] Add the resampled crop commit to `crop_group`: crop, Bicubic resize to the
  target, and write the resolution on a clone; add the straighten variant.

## Bridge and UI

- [x] Add `CropResampleTarget` to `CropOptions` and arm/clear it from
  `options_bar_crop.cpp`; make W/H edit the target and ratio instead of the box.
- [x] Call the resampled crop/straighten bridge from `CropToolHandler::commitCrop`
  when the target is armed.
- [x] Add the `image.revealAll` command-catalogue row and wire the Image menu
  handler and enabled provider to `reveal_all`.

## Trackers and docs

- [x] Update `ROADMAP.md`, `PARITY.md`, and
  `docs/dev/doc-code-parity-gaps.md` TOOL-20.
- [x] Update `docs/dev/STATE.md`.
- [x] Validate with `openspec validate --all --strict` and archive the change.
