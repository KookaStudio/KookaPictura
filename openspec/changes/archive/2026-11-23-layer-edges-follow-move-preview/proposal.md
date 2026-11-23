# Proposal: layer-edges-follow-move-preview

## Why

While dragging a shape layer with the Move tool, the dashed Layer Edges box stays
at the old position until release. The Move preview is client-side, so the
document outline is unchanged; the dashed border looks like a leftover box on the
canvas.

## What Changes

- `ImageView::beginMovePreview` records whether the preview is a layer move (the
  Move tool) or a selection-content move.
- Layer Edges and the Target Path are offset by the preview delta only for a
  layer move, so a content move leaves the document overlays in place.
- `tst_view_menu::layerEdges` checks the dashed box during the drag as well as
  after the commit.

## Capabilities

### Modified Capabilities

- `ui/view-menu`: while the Move tool previews a layer move, the Layer Edges
  outline follows the preview offset.

## Impact

- `pictura-app` C++ canvas overlay and Qt Test only. No engine or document-format
  change.

## Provenance

Behavior follows the existing Layer Edges contract in
`openspec/specs/ui/view-menu/spec.md`; no external implementation is used.
