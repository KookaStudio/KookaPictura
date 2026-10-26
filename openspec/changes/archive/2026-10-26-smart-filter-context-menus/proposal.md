# Proposal: smart-filter-context-menus

## Why

Issue #934. The Layers panel shows a "Smart Filters" group with one row per
smart filter, but right-clicking either row falls through to the generic
pixel-layer menu. CS6 gives each its own menu: the group row offers Disable
Smart Filters / Delete Filter Mask / Clear Smart Filters, and a filter row
offers Edit Smart Filter Blending Options… / Edit Smart Filter… / Disable Smart
Filter / Delete Smart Filter.

## What Changes

- The Layers panel gives the "Smart Filters" group row and each smart-filter
  row their own context menu with exactly CS6's items.
- **Delete Filter Mask** is enabled only when the smart object carries a filter
  mask (from #956).
- **Edit Smart Filter…** reopens the filter's dialog for the two filters whose
  settings we model (Camera Raw and Lens Correction), replacing the entry; it is
  disabled for other filter kinds, whose editable settings we do not retain.
- **Edit Smart Filter Blending Options…** is shown but disabled; a per-filter
  opacity/blend-mode dialog is a follow-up.

## Capabilities

### New Capabilities

- `ui/smart-filter-context-menus`: the two Layers-panel context menus.

## Impact

- `pictura-app` (`panels/layers_panel_menu.cpp`, `panels/layers_panel.{h,cpp}`,
  `frame_menus_smart_object.cpp`, `layers_smart_filters.rs`), `tst_layers_smart_filters`.
