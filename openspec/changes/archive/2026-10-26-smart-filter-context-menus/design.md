# Design: smart-filter-context-menus

## Detection

`LayersPanel::showContextMenu` already resolves the clicked index. After the
eye/mask/vector-thumb checks it now branches on the row kind: `smart-filters`
(the group header, path `<layer>/@sf`) and `smart-filter` (path
`<layer>/@sf/<i>`). The base layer path is the text before `/@sf`, and `i` is
the trailing index.

## Menus

`populateSmartFiltersGroupMenu(menu, path)`:

- `Disable Smart Filters` / `Enable Smart Filters` — `layer_smart_filters_enabled`.
- `Delete Filter Mask` — enabled only when `layer_has_smart_filter_mask`.
- `Clear Smart Filters`.

`populateSmartFilterMenu(menu, path, index)`:

- `Edit Smart Filter Blending Options…` — disabled (follow-up).
- `Edit Smart Filter…` — enabled when the filter is one we can edit
  (`layer_smart_filter_edit_kind` 1 = camera-raw, 2 = Lens Correction).
- `Disable Smart Filter` / `Enable Smart Filter` — `layer_smart_filter_visible`.
- `Delete Smart Filter`.

Each action calls the matching `pictura-app` bridge and refreshes the panel.

## Edit Smart Filter

`LayersPanel` emits `smartFilterEditRequested(base, index)`. `PicturaMainWindow`
resolves `layer_smart_filter_edit_kind` and dispatches the filter's own command
(`Filter > Pictura Raw…` for camera-raw, `Filter > Lens Correction…` for Lens
Correction). Both already prefill from the stored record and replace the entry,
so "Edit Smart Filter…" is exactly that command re-run on the current layer.

## Follow-ups

- Per-filter blending options (opacity + blend mode) dialog and codec write (#961).
- The filter-mask thumbnail on the group row (a panel-delegate change).
