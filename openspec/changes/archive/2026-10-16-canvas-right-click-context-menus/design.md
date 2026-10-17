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
  ownership, always acting on the clicked document.
- Reuse existing widgets (Brush Preset picker) rather than deriving content that
  already exists in the UI.
- Menus that a Qt Test can inspect without a live event loop.

**Non-Goals:**

- The Zoom menu (owned by the unarchived `canvas-zoom-and-active-layer` change);
  this change routes it but does not re-specify it.
- The vector Shape/Path attribute menus (need clipboard/attribute machinery).
- The Slice menu (its combine/stacking/multi-selection model does not exist;
  #543).
- The Color Sampler sample-size menu and canvas-background Snap settings.
- Non-menu canvas right-click gestures (the `Alt+Right-drag` HUD, the
  `Shift+Alt+right-click` colour picker) — unchanged.

## Decisions

### Route through the active handler, on the clicked document

Add to `ToolHandler`:

```cpp
virtual void onContextMenu(ToolServices& ctx, const QPointF& imagePos,
                           const QPoint& globalPos) {}
```

`ToolController::routeCanvasContextMenu(canvas, imagePos, globalPos) -> bool` binds/uses the given canvas,
resolves its active handler, and calls the hook; it returns false when the tool
has no canvas menu. The frame's `showCanvasContextMenu` receives the source
canvas from the connection (capture `entry.canvas` in the existing
`contextMenuRequested` lambda) and calls that router first — the tool's menu
takes precedence, so the Zoom menu still opens over the pasteboard (#255). When
the tool has no menu and the click is on the pasteboard outside the document
rect, the frame opens the canvas-background colour menu. The canvas argument is
what fixes multi-document routing: the menu acts on the document whose canvas
emitted the event, not the active tab.

Alternatives: keep the frame's `if` chain (rejected — two mechanisms); return a
`bool handled` (rejected — nothing to fall back to over the document); rely on
`activeTool()` alone (rejected — wrong document with two open).

### Shared-widget menus via controller signals

`ToolServices` is implemented by `ToolController` (`tool_services.h:15`,
`tools.h:177`), and the controller holds no `Toolbox`/`OptionsBar`/`LayersPanel`
(the toolbox holds the controller). So the frame cannot implement the
`ToolServices` methods. Instead the controller gains signals the frame wires,
matching the existing `layerSelectionRequested` pattern:

```cpp
void zoomMenuRequested(const QPoint& globalPos);
void brushPickerRequested(const QPoint& globalPos);
void isolationModeRequested();
```

`ZoomToolHandler::onContextMenu` emits `ctx.requestZoomMenu(globalPos)`;
`BrushToolHandler` emits the brush-picker request; `MoveToolHandler`'s Isolation
entry emits the isolation request. The frame connects each to
`Toolbox::zoomMenu()`, `OptionsBar::brushPicker()`, and
`layersPanel_->enterIsolation`.

`BrushPresetPicker` gains `popUpAt(const QPoint& globalPos)`, a sibling of the
existing `popUpUnder(anchor)`, so it can open at the pointer.

Alternatives: give the controller pointers to the widgets (rejected — inverts
ownership and adds cycles); move the widgets into the handlers (rejected —
churns working features); leave Zoom and Brush as frame special-cases (rejected
— reintroduces the second mechanism).

### The brush picker is a non-grabbing overlay

A `Qt::Popup` takes an implicit mouse/keyboard grab: while the picker is open the
canvas receives no pointer movement (so the brush-size ring freezes and its
`leaveEvent` clears it), and the first canvas press is consumed to dismiss the
popup, so painting needs a second press. Neither can be fixed while the picker
grabs. The picker therefore becomes an ordinary in-window child of the main
window (parented through `window()`), positioned and clamped in parent
coordinates, and dismisses itself from a `qApp` event filter on an outside
pointer press — returned **unconsumed**, so the same press reaches the canvas and
begins the stroke — and on Escape. It opens offset a few pixels from the
pointer so the ring stays visible immediately. Both entry points (the options-bar
tip button and the canvas right-click) share this sticky behaviour.

Alternatives: keep `Qt::Popup` and forward the dismissing press (rejected — the
grab cannot be released); a frameless `Qt::Tool` top-level (rejected — Wayland
ignores a client `move()` of a top-level, so it would land in the wrong place;
the in-window child works on X11, Wayland, and offscreen alike, the same split
`PanelFloat` documents).

### Move: every layer under the pointer

`auto_select_path` (`helpers_composite.rs`) walks the layer tree and returns the
first hit. Refactor the walk into a collector returning all hits, topmost first,
and add a free bridge `layers_under_layer(view, x, y) -> QStringList` in
`impl_layers/auto_select.rs` beside `auto_select_layer`. `MoveToolHandler` builds
the list menu when Auto Select is off (each entry calls `ctx.selectLayer(path)`)
and always adds the Isolation Mode entry.

Alternatives: enumerate layers in C++ from the Rust document (rejected — the
alpha hit test already lives in Rust); call `auto_select_layer` per layer
(rejected — O(n²) and re-walks the tree).

### Menus built by handlers; test accessor builds without popping

Each handler builds its `QMenu` (parented to `ctx.canvas()`, so it is destroyed
with the canvas) in `onContextMenu`, adds actions wired to `triggered`, and
calls `.popup(globalPos)` — non-modal, matching the current menus, so the
offscreen run never blocks. `ToolController::contextMenuForTest(canvas, imagePos)
-> QMenu*` builds the active handler's menu without popping it, so a test can
enumerate its actions. Entries that depend on state (the Crop box, the Move
layer list) are computed eagerly at build time, so their state is real in the
un-popped menu; menus are not enabled/disabled. This is the
`LiquifyCanvas::zoomMenuActions` pattern.

Alternatives: `exec()` (rejected — blocks the offscreen harness); a
data-descriptor menu model (rejected — new abstraction for a handful of static
menus).

### Palette content

The canvas-background menu offers exactly the colours the existing `Space+F`
cycle steps through (`frame_canvas.cpp` `kCanvasColors`), read from the same
source so the menu and the cycle cannot drift. The spec deliberately does not
enumerate them.

### Divergence from the documented menu architecture

`docs/02-ui-ux/menus.md` describes a `pictura_ui::menu::context` /
`ContextMenuFactory` keyed by `(surface, activeTool, selectionKind, itemKind)`.
No such factory exists in code, and the current zoom/Move menus are already
built ad hoc; this change keeps that per-tool-handler approach rather than
introducing the factory. Noted here so the divergence is deliberate.

## Risks / Trade-offs

- **Offscreen popups** → all menus use `popup()`, and tests inspect the built
  `QMenu` directly, never a shown menu.
- **Menu content drifting from options-bar controls** → where a menu mirrors an
  option (Eyedropper Sample Size/Sample, Crop Delete Cropped Pixels), the entry
  writes through the same `ToolOptions` group the options bar reads.
- **Move list cost** → one tree walk per right-click; negligible for a click.
- **Pasteboard detection edge cases** (rotated view, fitting canvas) →
  `widgetToImage` already accounts for rotation and zoom, so the document-rect
  test needs no new geometry.

## Open Questions

- Which additional paint tools (beyond Brush and Pencil) should open the Brush
  Preset picker from the canvas. Deferrable; the hook makes each a one-liner.
