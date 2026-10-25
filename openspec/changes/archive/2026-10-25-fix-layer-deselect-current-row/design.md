# Design

## Context

`LayersPanel::writeTarget()` is the panel-to-engine direction: the tree's row
gesture picks the rows, and the panel writes them to the document target. When
`selectedPaths()` is empty it calls `set_layer_target(view, {}, {})` and stops.
Qt's `clearSelection()` and the empty-space click leave the selection model's
current index untouched, so the current row survives the target it no longer
projects.

The app-to-panel direction (`setTreeSelection`, used by `refresh` and
`selectPaths`) already clears both the selection and the current index for an
empty target. Only the user-driven path skipped the current index.

## Decisions

- Clear the current index in the empty branch of `writeTarget()`, immediately
  after the target is emptied. That is the one place a user gesture empties the
  target, and it keeps the projection rule in one function.
- Set `applyingTarget_` around the `setCurrentIndex` call. The model emits
  `currentChanged`, whose handler would otherwise run `writeCurrentAnchor()` and
  `syncControls()` against a half-reconciled projection. The guard skips the
  anchor write; `syncControls()` and `syncMaskSolo()` still run through the
  signal and again after `writeTarget()` returns.
- Leave a non-empty target alone: a current row outside the selection still
  survives a refresh (`lt_ui_stray_row`), and only an empty target has no
  active path for a current row to name.
