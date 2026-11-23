# Proposal: target-path-follows-active-layer

## Why

Selecting another shape layer does not redraw the Target Path until the next
document edit or view refresh. A Move drag then carries the previous layer's
outline, so the blue border appears to move on another shape. Selecting a layer
in the Layers panel is a target change, and the Target Path contract requires the
curve to update.

## What Changes

- The Layers-panel selection handler refreshes the active window's Target Path
  overlay.
- `refreshTargetPathOverlay` ignores a window that has no canvas yet, so the
  refresh is safe while a document is still loading.
- `tst_view_menu::shapeTargetPathFollowsActiveShape` selects another shape and
  checks the curve before any document edit, then drags it.

## Capabilities

### Modified Capabilities

- `ui/view-menu`: selecting a layer whose outline is the Target Path updates the
  curve immediately.

## Impact

- `pictura-app` C++ frame wiring and Qt Test only. No engine or document-format
  change.

## Provenance

Behavior follows the existing Target Path contract in
`openspec/specs/ui/view-menu/spec.md`; no external implementation is used.
