# Test plan — The shared fill path fills the active mask

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| The Brush tool hides the layer through its mask | With a mask edit target, `begin_brush` paints a black foreground | One "Brush" state is recorded, the composite hides the layer there, and deleting the mask shows the pixels unchanged | `tst_mask_edit::canvasToolsWriteCoverage` |
| The Eraser reveals with the background colour | With a mask edit target, `begin_eraser` paints a white background over hidden coverage | One "Eraser" state is recorded and the layer shows again | `tst_mask_edit::canvasToolsWriteCoverage` |
| A tool without mask support refuses | With a mask edit target, the Pattern Stamp starts | It is refused and no history state is recorded | `tst_mask_edit::canvasToolsWriteCoverage` |
| `lm_mask_fill_path` | With a mask edit target, run the Gradient, the Paint Bucket, `Edit ▸ Stroke`, Fill Path, Stroke Path, and a shape's Fill Pixels | Each writes the mask's coverage, records its own state, and leaves the layer's pixels unchanged | `tst_mask_edit::fillToolsWriteCoverage` |
| `lm_mask_fill_path_locked` | Target a text layer's mask and drag the Gradient across it | The mask's coverage takes the ramp and one "Gradient" state is recorded | `tst_mask_edit::gradientFillsALockedLayersMask` |
| `Edit ▸ Fill` on a mask still fills it | With a mask edit target, run `Edit ▸ Fill` with a black foreground | One "Fill" state is recorded and the coverage changes | `tst_mask_edit::paintAndFillWriteCoverage` |
| A mask fill does not touch the layer's pixels | After each mask fill, delete the mask | The layer's own pixels are unchanged | `tst_mask_edit::fillToolsWriteCoverage` |
