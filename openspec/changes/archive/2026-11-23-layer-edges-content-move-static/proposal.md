# Proposal: layer-edges-content-move-static

## Why

The Layer Edges requirement says a selection-content move SHALL leave the outline
in place, but no test pins that clause. The PR review noted the behavior is
correct by construction (the content-move preview keeps `movesLayer` false) but
not covered.

## What Changes

- `tst_view_menu::contentMoveLeavesOverlays` starts a preview with the
  content-move default and checks that both the Layer Edges box and the Target
  Path curve stay at their committed positions.
- No production behavior changes; this pins the existing default.

## Capabilities

### Modified Capabilities

- `ui/view-menu`: adds a scenario that a selection-content move leaves the Layer
  Edges outline in place.

## Impact

- Qt Test only. No engine, app behavior, or document-format change.

## Provenance

Behavior follows the existing Layer Edges contract in
`openspec/specs/ui/view-menu/spec.md`; no external implementation is used.
