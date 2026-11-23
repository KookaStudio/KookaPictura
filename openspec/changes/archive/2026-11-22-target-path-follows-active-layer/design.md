# Design: target-path-follows-active-layer

## Context

`PanelHost::layerSelectionChanged` refreshes the align controls and the Move
transform box but not the Target Path overlay. The overlay is rebuilt only on a
frame refresh or a regional present, so a layer selected after the last refresh
keeps the previous layer's outline. A Move drag then offsets that stale outline.

## Decision

The layer-selection handler refreshes the active window's Target Path overlay.
`refreshTargetPathOverlay` returns early for a window with no canvas, which covers
the loading document that can emit a selection change during a tab retarget.

## Alternatives considered

- Rebuild the overlay on every Move press: covers the drag but leaves a stale
  outline while the layer is merely selected.
- Cache the overlay per active layer: more state for a cheap rebuild; the handler
  already runs once per selection change.

## Risks

- The handler now touches the canvas on every layer selection; the rebuild is the
  same one a frame refresh already performs, and `setTargetPathOverlay` skips an
  unchanged overlay.
- A loading window is skipped until its canvas exists.
