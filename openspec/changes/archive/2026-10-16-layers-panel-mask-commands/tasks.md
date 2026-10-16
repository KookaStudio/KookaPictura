# Tasks

## 1. Row menu

- [x] 1.1 `layer_mask_present_path` / `layer_mask_disabled_path`; mask rows gated by state; Qt Test.

## 2. Mask button

- [x] 2.1 Tooltip/enabled/action follow the active layer's masks; Qt Test.

## 3. Mask thumbnail menu

- [x] 3.1 Refine Mask… and Mask Options… in the #675 menu; Apply gated on a pixel layer.
- [x] 3.2 `apply_layer_mask` refuses groups and adjustment/fill layers; Rust test.

## 4. Refine Mask

- [x] 4.1 Split the refine bridge into source/target helpers; `refine_mask_*`; keep mask flags on a Layer Mask output.
- [x] 4.2 Refine the stored coverage; preview at preview size; Layer Mask output for any masked layer.
- [x] 4.3 `RefineEdgeDialog` mask mode; Qt Tests.

## 5. Display options and overlay

- [x] 5.1 View state, `mask_overlay_toggle`, display option setters, `mask_overlay_image`; Rust test of the overlay pixels.
- [x] 5.2 `MaskDisplayOptionsDialog`; `ImageView::setMaskOverlay`; frame resync; Alt+Shift-click toggle; Qt Tests.
- [x] 5.3 The overlay ends with its layer (current-layer change, structural reset, document reset); Qt Test.

## 6. Test plan

- [x] 6.1 `test-plan.md` maps every scenario.
