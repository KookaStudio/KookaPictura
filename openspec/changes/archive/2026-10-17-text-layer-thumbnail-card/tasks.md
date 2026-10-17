# Tasks

- [x] 1.1 New `tst_layers_panel_fixes::typeThumbnailIsSquareWithoutOutline` (next to `groupIconHasNoOutline`) covering `lpr_type_thumb_card`; the existing `tst_layers_panel::thumbnailCtrlClick` and `tst_layers_panel_fixes::groupIconHasNoOutline` cover the hit rect and the regular-layer outline.
- [x] 1.2 `test-plan.md`.
- [x] 2.1 Add `LayerRowDelegate::thumbnailCard` (type = square 1:1, else `letterboxedThumb`) and use it for the image thumbnail in `paint` and `thumbRect` (`layers_panel_internal.h`).
- [x] 2.2 Skip the 1 px black outline for a `type` row in `paint` (`layers_panel_internal.h`).
- [x] 3.1 `cmake --build build --parallel`; run the new Qt Test case and the layers-panel suites offscreen.
- [x] 3.2 `openspec validate --all --strict` and `scripts/check-test-plan.py`.
