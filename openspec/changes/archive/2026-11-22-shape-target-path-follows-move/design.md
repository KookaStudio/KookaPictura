# Design: shape-target-path-follows-move

## Context

A regional present runs `afterPresent`, which refreshes the layer-edge cache but
not the Target Path overlay. The Target Path curve is rebuilt by
`refreshTargetPathOverlay` only on a frame refresh. A Move commit presents a
region, so the composite and the layer edges move while the Target Path stays at
the old geometry.

## Decision

After a regional present, refresh the Target Path overlay for every window of
the presented document. `refreshTargetPathOverlay` already no-ops when the
overlay is hidden or the curve is unchanged (`setTargetPathOverlay` compares its
fields), so the extra call is cheap for ordinary pixel edits and correct for a
shape or vector-mask move.

## Alternatives considered

- Refresh only in the Move commit path: rejected because transforms and other
  regional edits can move a shape outline too, and the present hook already
  centralizes per-document overlay refresh.
- Refresh on a timer: rejected because the stale outline would remain visible
  until the next tick.

## Risks

- `refreshTargetPathOverlay` temporarily toggles the layer-target flag to read a
  shape's outline; it restores the flag, and `setTargetPathOverlay` skips an
  unchanged overlay, so repeated presents do not add repaints.
