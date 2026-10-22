# Proposal: type on a path

## Why

The Type tools can set point and paragraph type, but the third entry mode in
`docs/03-tools/type-tools.md` (TOOL-050) is missing: clicking a path with a Type
tool should flow text along the path, and clicking inside a closed path should
set area type. Issue #683 is the remaining sub-issue of #541 after paragraph
type landed in #682.

## What changes

- A Type or Type Mask click on the Work Path or a saved path starts a path-type
  session. The click's nearest point on the path is the text start; text flows
  in anchor order by arc length. An open path ends at its last anchor; a closed
  path wraps once around from the click. Overflow renders a `+` at the path end.
- A Type or Type Mask click inside a closed path starts area type. The text
  wraps to the path's bounding box and is clipped to the path interior (an
  approximation of Photoshop's line-to-shape reflow, recorded as a ceiling).
- The caret, selection, baseline guide, preview, commit, reopen, and Type Mask
  coverage all follow the path.
- Path type round-trips through PSD: the document-level `Txt2` engine data
  carries a `TextFrameSet` entry for each text layer (`Type` 2 for on-path,
  `Type` 1 for area), so a saved file reopens with its path type intact.

## Capabilities

- Added: `tools/type-on-path`.

## Impact

- `pictura-core`: `TypeSpec` / `TypeTool` carry an optional text path; path
  arc-length helpers.
- `pictura-render`: path layout, on-path rendering, area clipping, caret and
  baseline geometry, path hit-testing.
- `pictura-codec`: read/write the `Txt2` `TextFrameSet`.
- `pictura-app`: Type tool entry and overlay, path hit bridge, path fields in
  `TypeSetting`.

## Out of scope

- Dragging the path start marker or flipping text to the other side of a path.
- Photoshop's every-line area-type composer (the clipped bounding-box layout is
  the recorded approximation).
- Warp Text (#684) and gamma blend (#685).
