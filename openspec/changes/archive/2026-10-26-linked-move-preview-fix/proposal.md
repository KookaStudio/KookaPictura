# Proposal: A linked move previews and lands at the drop

## Why

Two bugs in `linked-layers-move` (#959) make a multi-layer move jump:

- The group's preview image is cropped to the canvas, but its cached origin was
  the unclamped union rect. After the group moved off the top/left edge, the
  next drag reused the cache and drew the cropped image at the wrong origin, so
  the layers previewed (and appeared to land) far from the pointer.
- The smart-guide references excluded only the active layer, so a multi-selection
  (both linked layers) used its own other layers as snap references and snapped
  to itself.

## What Changes

- A rendered move group's cached origin is clamped to the canvas exactly as its
  crop is, so a cached preview and a fresh one place the image identically and
  the committed move lands where the preview showed.
- Every layer the Move tool is moving — the selected layers and their link
  sets — is excluded from the smart-guide references, not just the anchor.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tools/canvas-tools`: the live move preview matches the committed move.
- `tools/smart-guides`: no moving layer is a snap reference.

## Impact

- Rust: `cxxqt_object/impl_transform/mod.rs` (`move_origin_opacity`),
  `impl_transform/bridge.rs` (`move_group_paths` free function).
- C++: `smart_guides.{h,cpp}` (reference list), `tool_move.cpp`, `tool_type.cpp`.
- Tests: `tst_linked_layers` gains `linkedMovePreviewOriginMatchesTheCrop` and
  `multiSelectionIsNotItsOwnReference`.
