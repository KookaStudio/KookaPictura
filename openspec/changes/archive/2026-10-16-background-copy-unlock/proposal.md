# Proposal: background-copy-unlock

## Why

Issue #216. Duplicating the Background produced a `Background copy` that was
still flagged as the Background and locked, so the document had two
Backgrounds. Its row was italic and showed a lock. Clicking the Background's
lock badge did nothing, so the only way to unlock it was the double-click
dialog.

## What Changes

- `duplicate_layer` and `duplicate_paths`: a Background's copy is released
  (unflagged, unlocked, opaque alpha added) by a shared `release_background`.
- `layer_from_background` uses the same helper, so a converted Background
  can take transparency.
- Layers panel: a click on the Background row's lock badge converts it at once,
  named `Layer N`.

Dragging the Background onto New Layer still converts it in place, as the
existing `lpr_background_drop` contract specifies. That is unchanged here.

## Capabilities

### Modified Capabilities

- `ui/layers-panel`: the Background's copy is ordinary, and the lock badge
  unlocks the Background.

## Impact

- `pictura-render` (layer_ops `create.rs`, `properties.rs`) and `pictura-app`
  (layers panel). No new dependency.
