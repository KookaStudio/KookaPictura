# Proposal

## Why

Issue #668: the Layers panel's mask controls don't follow CS6
(`docs/05-layers/layer-masks.md`).
- The row menu offers every mask command whatever the mask's state.
- The mask button never changes from "Add Layer Mask".
- The mask thumbnail menu (#675) carries `Select and Mask…`, which the docs
  mark CC-only, in place of CS6's Refine Mask…, and Mask Options… is a
  disabled placeholder.
- Refine Mask, Layer Mask Display Options and the rubylith overlay are missing.
- Apply Layer Mask is offered on groups and adjustment/fill layers, which
  render from their children or content rather than their own alpha, so
  applying drops the mask and reveals what it hid.

## What Changes

- **Row menu:** the mask rows are enabled by the clicked layer's mask state:
  Add only without a mask, Delete with one, and only whichever of
  Enable/Disable would change it.
- **Mask button:** "Add layer mask" without a layer mask; "Add vector mask"
  once one exists, and a click adds a vector mask (Alt: Hide All). Greyed out
  with "Add a mask" when the layer has both.
- **Mask-thumbnail menu:** `Select and Mask…` becomes **Refine Mask…**, Mask
  Options… opens Layer Mask Display Options, and Apply Layer Mask is disabled
  (and refused by the engine) on a group or an adjustment/fill layer.
- **Refine Mask:** the Refine Edge dialog in mask mode, titled "Refine Mask".
  It refines the layer's stored mask coverage rather than the selection, its
  output defaults to Layer Mask, and it replaces the mask of any layer that
  has one. One "Refine Mask" undo state. A preview samples the source at the
  preview size instead of materializing it at document size.
- **Mask Options:** a Layer Mask Display Options dialog for the rubylith colour
  and opacity (red, 50% by default). These are display-only view state.
- **Rubylith overlay:** Alt+Shift-click the mask thumbnail toggles it and
  selects the row. The canvas draws it over the document; it never reaches
  compositing. Like mask solo, it ends when the current layer changes or the
  layer stack is edited.
- **BREAKING**: the mask menu's `Select and Mask…` row is replaced by Refine
  Mask…; Select and Mask stays on the selection tools' options bar.

## Capabilities

- Added `ui/layers-mask-commands`.
- Modified `ui/layers-panel-mask-menu`: the mask context menu commands.
