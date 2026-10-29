# Tasks — The shared fill path fills the active mask

## 1. Test plan

- [x] 1.1 Write `test-plan.md` mapping every scenario to a test, and add the
      `tst_mask_edit` cases the plan names (`fillToolsWriteCoverage`,
      `gradientFillsALockedLayersMask`) before the implementation.

## 2. Engine and app

- [x] 2.1 Give `fills::apply` a `FillTarget` context and resolve a mask edit
      target: run the fill on the mask's coverage (`mask_document_coverage`),
      translate document-space geometry by the mask's origin, crop the selection
      to the mask's rectangle, and write the result back with `write_mask_back`,
      recording one state.
- [x] 2.2 Route the Gradient and the Paint Bucket through the mask-aware path,
      reading their drag and click in document coordinates.
- [x] 2.3 Fold `Edit ▸ Fill`'s private mask path into the shared path and delete
      `fill_mask` and `gray_image`.
- [x] 2.4 Route `Edit ▸ Stroke`, Fill Path, Stroke Path, and the shape tools'
      Fill Pixels through the mask-aware path, cropping a document-space coverage
      to the mask's rectangle and translating Stroke Path's samples.

## 3. Tests and docs

- [x] 3.1 Update `tst_mask_edit::canvasToolsWriteCoverage`: the Paint Bucket no
      longer refuses a mask target.
- [x] 3.2 Add `tst_mask_edit::fillToolsWriteCoverage` and
      `tst_mask_edit::gradientFillsALockedLayersMask`.
- [x] 3.3 Drop the "cannot paint a text layer's mask" note from
      `public_docs/text-and-typography.md`.
- [x] 3.4 `openspec validate --all --strict` and `scripts/check-test-plan.py`
      pass.
