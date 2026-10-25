# Test plan

Every scenario in the delta specs has a row. CI runs the Qt suites; no local
build is required for this change.

| Scenario | Steps | Expected | Coverage |
|---|---|---|---|
| A non-active row's mask becomes the target | Select a different row, click the masked row's mask thumb, paint | The clicked row and mask are active; coverage changes, pixels do not | `tst_mask_edit::maskThumbnailOnNonActiveRowKeepsTarget` |
| Ctrl-click loads the mask selection | Ctrl-click a raster mask thumb with a layer row selected | Mask coverage becomes the selection; the selected layer set does not grow | `tst_mask_edit::ctrlClickMaskLoadsSelection` |
| Ctrl-I inverts a fill mask | Target a fill's raster mask and run Invert | Mask coverage flips; the fill pixels are untouched | `tst_mask_edit::invertOnMaskTarget` |
| A type row offers its CS6 text controls | Open a real type row's menu | Orientation, anti-alias, point/paragraph, Warp Text enabled; work-path/shape/Copy CSS/SVG disabled | `tst_layers_row_menu::typeRows` |
| Type rows offer export | Open a type row menu | Quick Export and Export As are present | `tst_layers_row_menu::typeRowsOfferExport` |
| A fill row offers its mask and merge rows | Open a solid-fill row menu above a base layer | Mask/vector, clipping, Merge Layers, export, Rasterize Layer present; Convert to Smart Object and Copy CSS disabled | `tst_layers_row_menu::fillRows` |
| A group row offers export and Blending Options | Open a group row menu | Export, Add Vector Mask, Merge Layers, Blending Options present; Convert to Smart Object disabled | `tst_layers_row_menu::groupRows` |
| Group and adjustment rows offer export | Open group and adjustment kind menus | Both contain Quick Export/Export As | `tst_layers_row_menu::groupAndAdjustmentOfferExport` |
| A group opens Blending Options only | Open Blending Options on a group | Blending page current; ten effect rows disabled | `tst_layers_row_menu::groupBlendingOptions` |
| A group's opacity is written | Set `blending.opacity` on a group, then try `dropShadow.on` | Opacity writes; the effect write is refused | `tst_layer_style::groupBlendingOptions` |
| Per-kind Duplicate and Delete labels | Open pixel/group/type/fill menus | `Duplicate Layer…`/`Delete Layer` or `Duplicate Group…`/`Delete Group` | `tst_layers_row_menu::kindTextLists` |
| Disabled placeholders | Open type/fill/group menus | Copy CSS/SVG, Artboard/Frame, work-path/shape rows disabled with the standard tooltip | `tst_layers_row_menu::typeRows`, `tst_layers_row_menu::fillRows` |
