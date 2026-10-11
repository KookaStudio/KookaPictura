## 1. Fix

- [x] 1.1 Route `begin_brush` and `begin_eraser` to the mask stroke when a mask edit target is set
- [x] 1.2 Refuse the other painting tools, the Magic Eraser, and `fills::apply` on a mask target
- [x] 1.3 Emit `LayersPanel::editTargetChanged` and swap the FG/BG pair in the main window
- [x] 1.4 Cover the tool entry points and the colour swap in `tst_mask_edit`
