# Design

## Context

Adobe Help, "Merging adjustment or fill layers", allows an adjustment or fill
layer to merge with the layer below, with selected layers, inside a group, or
with all visible layers, but not to serve as the target. It also states that
merging an adjustment with the layer beneath rasterizes the adjustment into that
layer. The current `can_merge_target` guard is applied to both sides of Merge
Down, so a legal source-side merge is refused. Merge Layers already checks only
the bottommost selected layer.

The bridge returns only a replaced count from `merge_scope_changed`, so the UI
cannot select the node the engine created. `LayersPanel::refresh()` restores the
pre-merge paths; when those paths are gone it falls back to the document's active
layer and then row 0. That fallback is what selects a surviving hidden layer
after Merge Visible.

`CommandRegistry::refresh()` runs on menu `aboutToShow` and on document refresh.
A QAction shortcut consults the action's current enabled state, so a stale
disabled state makes `Ctrl+E` do nothing even though the menu entry works after
opening the menu.

## Goals / Non-Goals

**Goals:**

- Match Photoshop's source/target distinction for adjustment and fill layers.
- Make the merged node the sole panel selection after Merge Down, Merge Layers,
  and Merge Visible.
- Make selection-dependent command enablement and labels live, not menu-open
  dependent.
- Cover each behavior with an engine or Qt test.

**Non-Goals:**

- Changing merge result naming, blending, or history labels.
- Changing Merge Clipping Mask or Flatten selection behavior.
- Adding documentation changes; the existing docs already describe the target
  rule.

## Decisions

### D1 - Validate only the merge target

`merge_down` and `can_merge_down` SHALL check `can_merge_target` on the lower
layer only. `merge_selected` and `can_merge_selected` already check the
bottommost selected layer only, so they stay as they are. This lets the
compositor apply an adjustment or fill source to the target and bake the result
into a pixel node. An adjustment or fill layer remains invalid as the target.

Alternative rejected: special-casing adjustment layers in the compositor.
`composite_rgba` already decodes the adjustment block and applies it to the
running backdrop, so the existing scratch-document path is the correct one.

### D2 - Return the merged path and select it after rebuilding rows

`merge_scope_changed` will expose the `MergeOutcome` instead of discarding its
path. `merge_layers` and `merge_visible` return the path as `QString`, empty on
refusal. The Layer-menu and panel-menu handlers then:

1. rebuild the Layers panel rows,
2. select the returned path as the sole selection,
3. run the normal frame refresh.

The panel rebuild must happen before selecting, because the model still holds
the old row paths immediately after the engine mutation. Selecting afterwards
also overrides the stale-path fallback in `LayersPanel::refresh`.

Alternative rejected: storing a "last merged path" on the view. It leaves stale
state around and still needs the same handler work.

### D3 - Refresh the registry on panel selection changes

The Layers panel already emits `selectionChanged` after `syncControls` updates
the active layer. Add `registry_->refresh()` to the main window's
`selectionChanged` handler. This updates every selection-dependent action's text
and enabled state immediately. The menu-open refresh remains as a second path.

Alternative rejected: refreshing only `layer.merge.layers`. The registry
refresh is the existing mechanism and other selection-dependent commands have
the same stale-shortcut risk.

### D4 - Tests

Engine tests cover the new source-side guard and the retained target refusal.
Qt tests cover the menu label and enablement, the merge-result selection for all
three merge commands, and a selection change that enables `Ctrl+E` without
opening the Layer menu. The existing `tst_merge_commands` suite already owns the
merge command wiring, so the new cases live there.
