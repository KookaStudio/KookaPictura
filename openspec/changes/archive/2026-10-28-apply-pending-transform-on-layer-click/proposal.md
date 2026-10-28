# Proposal

## Why

With a Free Transform session pending, clicking another layer in the Layers
panel silently changes the active layer and leaves the transform uncommitted
(#905). The current guard greys the whole panel, so the click does nothing at
all: the user has to press Enter or Escape before the panel responds. Photoshop
resolves the pending transform before the active layer moves, so the transform
is never stranded behind a layer the user has already left.

## What Changes

- **A layer click resolves the pending transform first (#905).** While a modal
  Free Transform, `Edit > Transform`, or Warp session is pending, the Layers
  panel stays interactive for choosing the active layer. A change of the active
  layer — a row click, a keyboard row move, a right-click that moves the
  selection, or a tree edit gesture — asks the same
  "Apply the transformation?" prompt the tool switch uses, then moves the
  selection. `Apply` commits one `Free Transform` state, `Don't Apply` cancels
  without a state, and `Cancel` keeps the transform and leaves the selection
  where it was.
- **The panel's editing controls stay disabled.** The blend mode, opacity,
  fill, the lock toggles, and the action strip remain disabled until the
  transform is resolved, so a lifted transform's commit/cancel can still not
  discard a panel edit. Only the layer list becomes interactive.
- The `Layer`, `Image`, and `Filter` menus, the `Edit` pixel commands, and the
  lifted-transform direct-API refusals are unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/free-transform`: the "A pending transform locks layer edits"
  requirement no longer disables the whole Layers panel; the layer list stays
  selectable and resolves the pending transform through the Apply / Cancel /
  Don't Apply prompt, while the panel's editing controls remain disabled.

## Impact

Code:

- `crates/pictura-app/cpp/panels/layers_panel.h`,
  `layers_panel.cpp`, `layers_panel_target.cpp`, `layers_panel_events.cpp`,
  `layers_panel_menu.cpp` — a pending-transform resolver callback, the
  target-change gate, the tree-press and context-menu gates, and the
  editing-control disable.
- `crates/pictura-app/cpp/frame_transform_lock.cpp` — wire the resolver to
  `ToolController::resolvePendingTransform` and swap the panel-wide
  `setEnabled(false)` for `setTransformPending`.

Tests:

- Qt Test additions and updates in
  `crates/pictura-app/cpp/tests/tst_transform_lock.cpp`.

No new dependencies. No document or file-format change. The prompt reuses the
existing `askApplyTransform` dialog and the existing tool-switch decision path.
