# Proposal: shape-target-path-follows-move

## Why

Moving a shape layer with the Move tool leaves its Target Path outline behind at
the old position. A shape layer's outline is drawn as the Target Path when no
Paths-panel row is active, but the overlay is only refreshed on a full frame
refresh. A regional Move present updates the layer edges but not the Target
Path, so the blue outline stays where the shape used to be.

## What Changes

- `frame_canvas.cpp` refreshes each document window's Target Path overlay after a
  regional present, so a Move or transform that changes a shape or vector-mask
  outline redraws it.
- `tst_view_menu` gains a shape-move case: the Target Path curve follows the
  moved outline.
- The Layer Edges test also covers a shape move, confirming both overlays track
  the outline.

## Capabilities

### Modified Capabilities

- `ui/view-menu`: the Target Path overlay follows a move or transform of the
  layer whose outline is the target.

## Impact

- `pictura-app` C++ canvas wiring and Qt Test only. No new dependency, no engine
  or document-format change.

## Provenance

Behavior follows the existing Target Path contract in
`openspec/specs/ui/view-menu/spec.md`; no external implementation is used.
