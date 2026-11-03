# Proposal

## Why

Issue #1145. On a 16507×16196 document every action was slow: a tool switch
took 0.4–0.6 s, releasing a Marquee 2.5 s, and a Paint Bucket fill of a layer
19.5 s. The edits themselves were cheap. The time went to work around them:

- Every command-registry refresh ran 13 menu enabled checks that each scanned
  the selected layers' alpha planes for their bounds, about 150 ms in all.
- With a selection active, the 12 Edit ▸ Transform checks each copied the whole
  selection mask, about 2 s per refresh.
- A tool switch reported `sessionCancelled` even with nothing live, and its
  handler refreshed every panel at once.
- A whole-document region refresh copied the document into the composite and
  the level-0 frame row by row, serially.

## What Changes

- **Enabled checks test for content instead of measuring it.** A new
  `plane_has_coverage` stops at the first covered row and remembers its answer
  by the plane's ident, so the checks of one refresh read an unchanged plane
  once. Align, Distribute, Auto-Blend, Transform and Refine Edge use it.
- **A tool switch with nothing live reports no cancel.**
- **A whole-document region refresh installs its frames** instead of copying
  them.

## Capabilities

### New Capabilities

- `ui/command-refresh-responsiveness`

## Impact

`pictura-render` layer ops (`plane_has_coverage`, `layer_has_content`,
`can_align`'s signature), the app's align, transform, refine-edge and session
bridges, and the region refresh path.
