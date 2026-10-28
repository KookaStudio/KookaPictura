# Design

## Context

A modal Free Transform (Ctrl+T, `Edit > Transform`, or Warp) owns the session
slot until it commits or cancels. The `#750` lock keeps document edits from
being discarded while the session is live: the `Layer`, `Image`, and `Filter`
menus are blocked by `CommandRegistry::setBlockedProvider`, the direct edit
entry points refuse a lifted transform, and the whole Layers panel is disabled
with `layersPanel_->setEnabled(false)`.

Disabling the whole panel also removes the layer list, so the user cannot
switch layers while the transform is pending. Photoshop instead resolves the
transform first: the transform is applied (or the user is asked), then the
active layer moves. #905 asks for that behavior.

## Goals / Non-Goals

- **Goal:** a change of the active layer while a transform is pending resolves
  the transform through the existing Apply / Cancel / Don't Apply prompt before
  the selection moves.
- **Goal:** a lifted transform's commit/cancel still cannot discard a panel
  edit.
- **Non-Goal:** changing the menu lock, the direct-API refusal, or the tool
  switch prompt. Those keep their current behavior.
- **Non-Goal:** making the panel's blend/opacity/fill/lock/strip controls usable
  during a transform. They stay disabled until the transform is resolved.

## Decisions

### One resolver callback, not a new dependency

`LayersPanel` does not know `ToolController`. It gains a
`std::function<bool()>` resolver set by the frame:

```cpp
layersPanel_->setPendingTransformResolver(
    [this]() { return tools_ && tools_->resolvePendingTransform(); });
```

`ToolController::resolvePendingTransform` already returns true when no transform
is pending (a cheap no-op) and otherwise runs the same prompt the tool switch
uses: `Apply` commits, `Don't Apply` cancels, `Cancel` returns false. The panel
calls the resolver before a change and, on false, aborts the change.

### Gate the change sites, keep the controls disabled

The panel has many mutation entry points (the action strip, the sliders, the
context menu, the tree gestures). Gating every one would need an abort path per
control. Instead the panel keeps its *editing* affordances disabled while a
transform is pending — the same protection the whole-panel greying provided —
and only the layer list becomes interactive. The interactive sites are gated:

- `writeTarget` / `writeCurrentAnchor` — the selection became the document
  target. The gate only fires when the target actually changes, so re-clicking
  the active layer asks nothing.
- `eventFilter` mouse-press — any tree gesture (row select, eye, mask, lock
  badge, Alt-click) resolves first; a `Cancel` answer consumes the press.
- `showContextMenu` — a right-click that moves the selection, and every row
  menu action, resolve first.
- `performPanelMenuAction` — the panel menu's per-widget entries resolve first.

The controls are disabled by `syncControls` / `syncStripButtons`, which already
own their enabled state, so a refresh during the session keeps them disabled.

### Where the disabled state comes from

`syncTransformLock` keeps computing `transformLocked_` from
`tools_->transformPending()` and keeps the menu block. It replaces
`layersPanel_->setEnabled(!locked)` with `layersPanel_->setTransformPending(locked)`,
which stores the flag and re-runs `syncControls` / `syncStripButtons`.

## Risks / Trade-offs

- **Prompt re-entrancy.** The prompt is a modal dialog raised from inside a
  selection-model signal or an event filter. The tool-switch path already
  raises it from a click, so this is the same shape. The `Cancel` answer
  restores the tree selection with `applyTargetSelection`, whose
  `applyingTarget_` guard stops the restore from re-entering the gate.
- **Partial greying.** The layer list is interactive while the sliders and the
  strip are not. This is deliberate: the list is the path #905 needs, and the
  sliders keep the lifted-transform protection without an abort path.
- **No prompt for a no-op selection.** A click on the already-active layer does
  not change the target, so the gate does not fire and nothing is asked.
