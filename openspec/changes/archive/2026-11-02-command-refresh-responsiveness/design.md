# Design

## Measurements

The driver in the PR body runs a new 16507×16196 document with one empty layer
selected, with `PICTURA_PAINT_TIMING=1`, and times each registry command whose
providers cost at least 1 ms.

| Action | Before |
|---|---|
| Registry refresh, no selection | 150 ms: 13 Align, Distribute and Auto-Blend checks at about 11 ms each |
| Registry refresh, with a selection | 2.3 s: 12 Transform checks at about 165 ms each |
| Switch to Marquee, Paint Bucket or Brush | 375–590 ms |
| Marquee mouse release | 2.5 s |
| Paint Bucket fill of the layer | 19.5 s, 13.5 s of it the region refresh |

## The coverage memo

`Plane::ident` names one set of bytes: every write resets it, so two planes with
one ident hold the same bytes. `(ident, width, height)` is therefore a key that
never goes stale, with no revision to keep in step. The memo is a process-wide
map, bounded at 256 entries and cleared when full.

The early exit helps a layer with content; an empty layer is still read in
full, once. That is why the memo, not the early exit, carries the fix.

## Out of scope

The Move tool's activation still bakes a full-document preview, about 8 s at
this size. That needs the drag preview to stop depending on full-document
images, and is tracked separately.
