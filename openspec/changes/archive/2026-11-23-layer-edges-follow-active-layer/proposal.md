# Proposal: layer-edges-follow-active-layer

## Why

Selecting another layer in the Layers panel does not refresh the Layer Edges
cache until the next document edit or frame refresh. A Move drag then offsets the
previous active layer's dashed box, so another layer's boundary appears to move.

## What Changes

- The Layers-panel selection handler refreshes the active document's Layer Edges
  as well as its Target Path.
- `tst_view_menu::shapeTargetPathFollowsActiveShape` checks both overlays after a
  panel selection and during a drag.

## Capabilities

### Modified Capabilities

- `ui/view-menu`: selecting another layer updates the Layer Edges outline
  immediately.

## Impact

- `pictura-app` C++ frame wiring and Qt Test only. No engine or document-format
  change.

## Provenance

Behavior follows the existing Layer Edges contract in
`openspec/specs/ui/view-menu/spec.md`; no external implementation is used.
