# Design: layer-edges-follow-active-layer

## Context

`refreshLayerEdges` caches the active layer path and the canvas revision. A
Layers-panel selection changes the path but not the revision, and the selection
handler did not call the refresh, so the cache kept the previous layer's box. The
new Move-preview offset then moved that stale box.

## Decision

The Layers-panel selection handler calls `refreshLayerEdges` for the active
document before refreshing the Target Path. The cache key (path, revision)
invalidates on the changed path, so the box is recomputed once per selection.

## Alternatives considered

- Recompute the box on every paint: avoids the cache but scans a pixel layer's
  alpha on each repaint.
- Offset only when the cached path matches the active layer: hides the bug but
  leaves the wrong layer outlined while it is merely selected.

## Risks

- The selection handler now recomputes one content box per selection change; the
  existing cache still avoids recomputing on every present.
