# Tasks

## 1. Tests first

- [x] Add `tst_mask_edit::maskThumbnailOnNonActiveRowKeepsTarget`.
- [x] Add `tst_mask_edit::ctrlClickMaskLoadsSelection`.
- [x] Add `tst_mask_edit::invertOnMaskTarget`.
- [x] Extend `tst_layers_row_menu` with `typeRows`, `fillRows`,
  `groupBlendingOptions`, per-kind label/export expectations, and the disabled
  placeholder checks.
- [x] Extend `tst_layer_style` with `groupBlendingOptions`.
- [x] Update `tst_last_layer` for the singular `Delete Layer` row label.

## 2. Mask target and selection

- [x] Select the row before setting the mask target in the mouse filter and the
  mask-thumbnail context menu; consume mask-thumbnail clicks.
- [x] Route Ctrl-click on a raster mask thumbnail to `mask_selection(path, "new")`.
- [x] Accept an active mask target in `filter_target_ready()`.
- [x] Add a test seam for Ctrl-clicking a mask thumbnail.

## 3. Row menu

- [x] Add Copy CSS/Copy SVG and Artboard/Frame disabled rows.
- [x] Enable export for type, group, and adjustment/fill rows.
- [x] Add type/group Convert to Smart Object rows (enabled for type, disabled for
  group/fill).
- [x] Extend raster/vector mask, clipping, and merge rows to fill/adjustment and
  groups where the bridge supports them.
- [x] Add per-kind Duplicate/Delete labels and the group/fill Merge Layers label.
- [x] Add the type-only text block: orientation, anti-aliasing, point/paragraph,
  Warp Text, and disabled work-path/shape rows.
- [x] Register `layers_panel_type_menu.cpp` in `CMakeLists.txt`.

## 4. Group Blending Options

- [x] Allow `blending.*` writes on a group in the render style setter.
- [x] Let `layer_style_can_edit` accept a group.
- [x] Disable the effect rows for groups in `LayerStyleDialog`.
- [x] Keep effect writes refused; #939 tracks group-effect rendering.

## 5. Fixture and docs

- [x] Add white document-sized masks to the fixture's Invert and Solid Fill rows.
- [x] Regenerate `assets/layers_panel_smoke.psd`.
- [x] Update the Layers panel and layer-management docs.
- [x] Update `docs/dev/STATE.md`.

## 6. Verification and archive

- [x] Run the focused Qt suites and `openspec validate --all --strict` in CI.
- [x] Archive the change and keep only `archive/` under `openspec/changes/`.
