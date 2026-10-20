# Tasks: paintbrush-types-pen-tip

## 1. Tests

- [x] 1.1 Write `tst_brush_tip_preview` cases named in `test-plan.md` before
  verifying the implementation.

## 2. Presets, tip model, and grain

- [x] 2.1 Add `penTip` and `grain` to `PaintOptions` and the tool-options JSON.
- [x] 2.2 Append the four presets, their parameters, the `penTip` flag, and the
  `2B Pencil` grain to `kPresets`; apply them in `applyPreset`.
- [x] 2.3 Add `grain` to `StrokeConfig` and drop each dab's final alpha by the
  fixed per-pixel paper texture.
- [x] 2.4 Keep grainy strokes on the CPU oracle, since the GPU dab shader does
  not model grain.

## 3. Preview overlay

- [x] 3.1 Add `BrushTipPreview` and its CMake entry.
- [x] 3.2 Create one per document and sync its visibility from the frame.

## 4. Verification

- [x] 4.1 `TASK_ALLOWS_DOCS=1 bash scripts/verify-fast.sh` and the Qt suite.
- [x] 4.2 Update `docs/dev/STATE.md` and archive the change.
