# Proposal

## Why

Issue #1207: on a 17000×17000 document, a rectangular marquee followed by a
Paint Bucket click inside it gets pictura OOM-killed (anon-rss 4.7 GB on a 7 GB
host). For a 289 Mpx document one click allocated, regardless of the
selection's size:

- a document-sized interleaved RGBA copy of the layer (1.16 GB);
- a document-sized match plane, a document-sized seed plane marking one pixel,
  the flood mask and its anti-aliased copy (289 MB each);
- a whole-layer write that forked every 64-row band of every plane from history
  (1.16 GB), even when the marquee covered a small box.

Edit ▸ Fill (a document-sized mask of 255s and the whole-layer write), Edit ▸
Stroke (selection-sized dilate, erode and band planes over the whole document)
and the Gradient (the whole-layer write) share the pattern. Nothing refused an
edit the host could not hold; the Image Size gate (#1156) covers only Image Size.

## What Changes

- **The fills work inside the selection's box.** With a selection, the Paint
  Bucket's flood, its masks and its write, the Fill, the Gradient and the
  Stroke's band (grown by its reach) cover only the selection's bounding box,
  and write only the plane bands that box crosses. Without a selection they
  cover the layer as before.
- **The selection is a wall for the bucket's contiguous flood** (owner
  decision; CS6's behaviour is undocumented): an unselected pixel never
  matches, so the flood does not pass through it to reach another selected
  region. An unselected pixel is not an edge for the anti-aliasing.
- **The bucket reads the layer or composite planes directly**, without an
  interleaved copy, and floods from its single seed without a seed plane.
- **A memory gate.** Before allocating, each of the four fills estimates its
  peak bytes (its box-sized planes and the bands it forks) and is refused with
  "Could not fill: not enough memory…" on the status bar when they exceed three
  quarters of the memory available now, the Image Size rule.

## Capabilities

### New Capabilities

- `tools/fill-memory-bounds`

### Modified Capabilities

None.

## Impact

`pictura-core` (`mask_ops::flood_from`), `pictura-paint` (`bucket::{flood,
BucketSource, antialias, fill_within, stroke_selection, stroke_window}`,
`gradient::draw`), `pictura-app` (`paint_tools::fills` budget and bounded
targets, the `fill_refused_for_memory` bridge, the tool and Edit-menu refusal
messages).
