# Proposal: cross-document-multi-layer-drag

## Why

Issue #812: dragging a multi-layer selection onto another document's tab copies
only one layer, silently leaving the rest behind. The ticket blames
`file_drop_router.cpp` using `paths.first()`, but the drag payload itself never
contains more than one path: the Layers panel's drag provider returns only the
current row. A router-only loop would change nothing.

## What Changes

- A layer drag that starts from a multi-layer selection expands back to the
  source document's current layer target at drop time. Every selected layer
  whose ancestor is not also selected is deep-copied into the destination, each
  exactly once.
- The clipboard bridge gains a batch `copy_layers_from_document`: it inserts
  every path at the same destination insertion point (never chaining an anchor
  onto a created copy), orders the copies so the source stacking order survives
  both a sibling and a group destination anchor, replays the index shifts to
  recover the final created paths, records one "Duplicate Layer" state, and
  selects every copy with the dragged layer's copy active (the topmost copy
  when the dragged layer was not copied). All source paths are resolved before
  the first insertion so a batch cannot leave untracked layers.
- A Move-tool drag that reaches another document's tab hands the layer target's
  anchor to the same drag, so it crosses with the whole selection, not only the
  active layer, matching CS6's multi-layer move. With a single-selection the
  existing behavior is unchanged.
- The drag payload, the in-panel row drop, and the bottom-strip button drop stay
  single-row; only the cross-document drop consumes the full selection.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/layers-panel`: the requirement "Drag a layer into another document" gains
  the multi-selection copy, its one-undo-step and selection rules, and the
  Move-tool multi-selection wording.

## Impact

- `crates/pictura-app`: `clipboard` bridge (plural function), `frame.cpp` and
  `frame.h` (resolve the source view and call once), `file_drop_router.cpp`
  (the expansion/filter helper; the router keeps passing one dragged path),
  `tool_move.cpp` (hand the target anchor, not the single active layer), and
  `tst_layer_target.cpp` (coverage).
- No engine, codec, render, or dependency changes; the engine already deep-copies
  one node with mode/depth refusal.
