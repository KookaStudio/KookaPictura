# Proposal

## Why

Issue #1152, follow-up to #1098. On a 16507×16196 document three whole-document
restore costs remain on the GUI thread:

- Undoing a resize or crop keeps about 104 ms, and about 85 ms of it is copying
  level 0 out of the restored composite. The copy exists because the view
  pyramid reads level 0 as contiguous channel slices, and a copy-on-write plane
  read that way is materialized whole.
- Undoing a reorder that recomposited but left the picture unchanged compares
  the whole composite byte for byte: about 42 ms. A capture that starts a new
  geometry segment never compares the composite, so the undo has to.
- A soloed mask's document-sized display plane is filled serially on every
  whole-document restore, before the pyramid that consumes it is built in
  parallel.

## What Changes

- **The view pyramid reads a banded level-0 plane row by row.** Building,
  coarsening, updating and cropping never materialize a contiguous copy, so
  level 0 can share the restored composite's bands.
- **A whole-document restore installs level 0 without copying** when the
  document needs no display conversion. The frame shares the composite's
  copy-on-write plane and both planes are fractured, so the first write forks
  only the bands it touches. A frame that needs a colour conversion still owns
  the converted plane.
- **A capture that starts a new geometry segment keeps an unchanged composite
  plane.** The composite comparison runs once, on the edit that already
  recomposited, and the undo across that boundary reports an empty rectangle
  without scanning the composite.
- **A soloed mask's display plane is filled on every core.**

## Capabilities

### Modified Capabilities

- `compositing/view-pyramid`: level 0 may be a banded plane read row by row.
- `document/in-place-history`: a segment break keeps an unchanged composite.
- `document/history-restore-responsiveness`: a whole-document restore shares
  level 0, and a soloed mask's source builds in parallel.

## Impact

`pictura-render`'s `ViewPyramid` level-0 source, the app's display and history
paths, and the solo mask plane. Every large whole-document restore copies less,
and a no-op reorder undo stops scanning the composite.
