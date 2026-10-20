# Design: smart-guides-menu-toggle

## State

`PicturaMainWindow::smartGuidesVisible_` is global view state, like
`guidesVisible_`, defaulting to on. `ImageView` keeps its per-canvas
`smartGuidesEnabled_` flag; the frame pushes the global value into every canvas
in `applyGuideViewState()` and when a document's canvas is created
(`prepareCanvasGuides()`).

The command is registered as checkable and uses the same toggle machinery as
`View > Show > Guides`: the handler flips the flag, applies it to every canvas,
saves the session, and refreshes the registry so the checked provider re-syncs
the action. Flipping the flag rather than reading the action also makes a
control-server dispatch toggle correctly.

## Behavior

Turning the flag off calls `ImageView::setSmartGuidesEnabled(false)`, which
clears any active lines and labels and makes the Move and Type tools skip both
the solver and the drawing. Turning it on restores snapping and drawing for the
next drag; no guide is shown until an alignment is active.

## Persistence

`SessionState::smartGuidesVisible` defaults to `true`, so an older store without
the key keeps the current always-on behavior. No schema bump is needed: the key
is independent and backward compatible.
