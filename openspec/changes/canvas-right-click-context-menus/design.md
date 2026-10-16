# Design

## Context

`ImageView::contextMenuEvent` already emits `contextMenuRequested(globalPos)`,
and `PicturaMainWindow::showCanvasContextMenu` handles exactly two tools with an
`if (activeTool() == …)` chain (Zoom via the toolbox menu, Move via an
Isolation Mode action). Every `ToolHandler` is Qt-aware C++ and already receives
a `ToolServices` context, so a per-tool hook fits the existing shape. Two menus
live in widgets the handlers cannot see: the Zoom preset menu in `Toolbox` and
the Brush Preset picker in `OptionsBar`. The Move tool's hit test
(`auto_select_layer`) returns only the topmost layer, not every layer under the
pointer. See `proposal.md` for motivation and the spec delta for the required
behaviour.

## Goals / Non-Goals

**Goals:**

- One routing point for a canvas right-click, with the tool as the unit of
  ownership.
- Reuse existing widgets (Zoom menu, Brush Preset picker) rather than deriving
  menu content that already exists in the UI.
- Menus that a Qt Test can inspect without a live event loop.

**Non-Goals:**

- The vector Shape/Path attribute menus (deferred to a separate change; they
  need clipboard/attribute machinery that does not exist yet).
- Moving the Zoom menu or Brush Preset picker out of their current widget
  owners.
- Non-menu canvas right-click gestures (the `Alt+Right-drag` HUD, the
  `Shift+Alt+right-click` colour picker) — unchanged.

## Decisions

### Route through the active handler

Add to `ToolHandler`:

```cpp
virtual void onContextMenu(ToolServices& ctx, const QPointF& imagePos,
                           const QPoint& globalPos) {}
```

`ToolController::showCanvasContextMenu(imagePos, globalPos)` resolves the active
handler and calls it. The frame's `showCanvasContextMenu` becomes a splitter: if
`imagePos` is inside the document rect it calls the controller; otherwise (the
pasteboard) it opens the canvas-background menu. No `bool` return is needed — the
inside/outside split is geometric, not a matter of a tool consuming the event, so
a tool with no menu simply opens nothing over the document.

Alternatives: keep the frame's `if` chain and add branches (rejected — two
mechanisms, the thing to avoid); return a `bool handled` (rejected — nothing to
fall back to over the document).

### Shared-widget menus via `ToolServices`

The Zoom menu and Brush Preset picker live in `Toolbox` and `OptionsBar`. Add
two `ToolServices` methods, implemented by the frame by delegating to those
widgets:

```cpp
virtual void showZoomMenu(const QPoint& globalPos) = 0;
virtual void showBrushPicker(const QPoint& globalPos) = 0;
```

`ZoomToolHandler::onContextMenu` calls `ctx.showZoomMenu`; `BrushToolHandler`
calls `ctx.showBrushPicker`. This keeps the handler the single route without
relocating working widgets.

`BrushPresetPicker` gains `popUpAt(const QPoint& globalPos)`, a sibling of the
existing `popUpUnder(anchor)`, so it can open at the pointer instead of under
the options-bar button.

Alternatives: move both menus into their handlers and have the widgets call back
into the controller (rejected — churns working features for no gain); leave Zoom
and Brush as frame special-cases (rejected — reintroduces the second mechanism).

### Move: every layer under the pointer

`auto_select_path` (`helpers_composite.rs`) walks the layer tree and returns the
first hit. Refactor the walk into a collector that returns all hits, topmost
first, and add a free bridge `layers_under_layer(view, x, y) -> QStringList` in
`impl_layers/auto_select.rs` beside `auto_select_layer`. `MoveToolHandler` builds
the list menu (each entry calls `ctx.selectLayer(path)`) when Auto Select is off,
plus an Isolation Mode entry.

Isolation needs a new `ToolServices::enterIsolationMode()`, implemented by the
frame via `layersPanel_->enterIsolation`.

Alternatives: enumerate layers in C++ from the Rust document (rejected — the
alpha hit test already lives in Rust); reuse `auto_select_layer` per layer
(rejected — O(n²) and re-walks the tree).

### Menus built by handlers; test accessor

Each handler builds its `QMenu` in `onContextMenu` and calls `.popup(globalPos)`
(non-modal, matching the current Zoom/Move menus, so the offscreen run never
blocks). The handler keeps the last-built menu; `ToolController` exposes
`contextMenuForTest(imagePos) -> QMenu*` that builds the active handler's menu
without popping it. A Qt Test enumerates actions and enabled state — the pattern
`LiquifyCanvas::zoomMenuActions` already uses.

Alternatives: `exec()` (rejected — blocks, and the offscreen harness cannot
dismiss it); a data-descriptor menu model (rejected — new abstraction for a
handful of static menus).

### Scope of the paint picker

`BrushToolHandler` (shared by Brush and Pencil) implements `onContextMenu` in
this change. Other handlers that also own a `PaintOptions` (Healing, Clone,
Eraser family) are a one-line addition each, deferred.

## Risks / Trade-offs

- **Offscreen popups** → all menus use `popup()`, and tests inspect the built
  `QMenu` directly, never a shown menu.
- **Menu content drifting from options-bar controls** → where a menu mirrors an
  option (Eyedropper Sample Size, Crop Delete Cropped Pixels), the entry writes
  through the same `ToolOptions` group the options bar reads, so both stay in
  sync.
- **Move list cost** → one tree walk per right-click; negligible for a click.
- **Pasteboard detection edge cases** (rotated view, fitting canvas) →
  `widgetToImage` already accounts for rotation and zoom, so the document-rect
  test is correct without new geometry.

## Open Questions

- Which additional paint tools (beyond Brush and Pencil) should open the Brush
  Preset picker from the canvas. Deferrable; the hook makes each a one-liner.
