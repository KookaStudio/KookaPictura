# Design

## Row table

`kRowSpecs` (`layers_panel_menu.cpp`) already carries the smart-object rows
`resetTransform` / `convertToLayers` / `newSmartObjectViaCopy`, gated by a
`SmartObject` kind mask threaded from `SmartObjectRole`. The new rows extend the
same table:

- `convertToSmartObject` — gated `Pixel`; enablement `ConvertToSmartCan` calls
  `layer_can_convert_to_smart_object`.
- `editContents` / `replaceContents` / `exportContents` — gated `SmartObject`;
  enablement calls `layer_can_edit_smart_object_contents`,
  `layer_can_replace_smart_object_contents`, and
  `layer_can_export_smart_object_contents` respectively.
- `relinkToFile` / `convertToLinked` — gated `SmartObject`, `implemented =
  false`, so the table renders them disabled with the shared `— not implemented
  yet` tooltip. Their action ticket is #113.

The row order groups the smart-object commands after the shape rows:
`convertToSmartObject`, `resetTransform`, `convertToLayers`,
`newSmartObjectViaCopy`, `editContents`, `replaceContents`, `exportContents`,
`relinkToFile`, `convertToLinked`.

## Acting on the clicked row

`convertToSmartObject` uses the view-level bridge (`view_->convert_to_smart_object`)
and reselects the created object, exactly as `newSmartObjectViaCopy` does.

Edit / Replace / Export need frame-level behavior: opening an editor tab and
tracking a session (Edit), or file dialogs plus the frame's refresh (Replace,
Export). A panel cannot call the frame directly without inverting the layering,
so the panel emits `smartObjectContentsRequested(action, path)` — the same
signal pattern already used for `selectionChanged` / `editTargetChanged`. The
frame connects it once in `wireSmartObjectRowMenu()` and runs the matching
operation on `path`.

Replace and Export are extracted from the existing `Layer > Smart Objects`
command lambdas into `PicturaMainWindow::replaceSmartObjectContents(path)` and
`exportSmartObjectContents(path)`, placed in `frame_menus_smart_object.cpp`
(`frame.cpp` is at its file-size ceiling). The command handlers delegate to the
same methods, so the menu and the row menu share one implementation.

## Verification

`tst_command_tree` asserts the per-kind row lists (a pixel row gains Convert to
Smart Object). `tst_smart_object_actions` asserts the smart row's new entries and
that Convert to Smart Object from the row menu records one state; the contents
actions are covered by a `QSignalSpy` on the new signal (their dialogs are not
automated).
