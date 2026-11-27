# Design

## Context

`History::restore_live` reports where the picture changed as a bounded
`Restored::Region` or as `Restored::Everywhere`. The damage is built from the
crossed tile deltas and, when the document has no stored composite, from the
layer-channel tiles grown by their influence.

For a layered RGB document the stored composite can be dropped by a region
refresh, so a later state may have no composite delta. The display then reads
the cached composite tiles, and a bounded restore that only considered
composite deltas leaves the later strokes in those tiles. A non-linear branch
capture stores the appended state as a full anchor, which has no delta at all.

## Decisions

- **Layer tiles always bound the damage.** `Damage::add` now unions the placed
  layer-channel and mask tiles for every crossed delta, not only for documents
  without a stored composite. The region can be wider than the composite delta,
  but it always covers the picture the display must rebuild.
- **Leaving a full anchor backward is unbounded.** In `History::step_to`, a
  backward walk that leaves an anchor whose stored form is `Stored::Full` sets
  `damage.everywhere`. That is the non-linear branch case: the anchor's pixels
  are not represented by a delta, so the display rebuilds from the restored
  document. Forward walks and hollow-anchor crossings keep their bounded
  damage, preserving the resize/redo bounds tests.

## Risks

- A restore may refresh a slightly larger region than before. The cost is one
  bounded composite, not a full rebuild, except for the deliberate full-anchor
  case.
