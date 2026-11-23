# Proposal: shape-target-path-drag-follow

## Why

While dragging a shape layer with the Move tool, its Target Path outline stays at
the old position until release. The Move preview is client-side and does not
change the document, so the stored Target Path overlay is not rebuilt; on release
the regional present refreshes it to the new position.

## What Changes

- The canvas marks a Target Path that is the active shape layer's outline, and
  draws that curve offset by the Move preview's delta.
- `tst_view_menu::shapeTargetPathFollowsMove` checks the curve during the drag as
  well as after the commit.

## Capabilities

### Modified Capabilities

- `ui/view-menu`: while the Move tool previews a layer whose outline is the
  Target Path, the curve follows the preview offset.

## Impact

- `pictura-app` C++ canvas overlay and Qt Test only. No engine or document-format
  change.

## Provenance

Behavior follows the existing Target Path contract in
`openspec/specs/ui/view-menu/spec.md`; no external implementation is used.
