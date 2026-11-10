## Context

The Brush and Pencil handlers override `hasCanvasContextMenu()` and
`onContextMenu()` to call `ToolServices::requestBrushPicker(globalPos)`. The
frame owns the picker and opens it as a non-grabbing overlay. The four stamp
tools (Clone Stamp, Pattern Stamp, History Brush, Art History Brush) derive from
the shared `StampToolHandler` in `tool_stamps.cpp` and define neither override,
so the router returns false and the right-click is a no-op.

## Decisions

### Put the overrides on `StampToolHandler`

All four stamp tools already paint through the same `PaintOptions` tip and all
four options bars show the same Brush tip button. Adding the two overrides once
on the base class gives every stamp tool the picker and keeps the behavior from
drifting per subclass. A per-subclass override would repeat the same two lines
four times.

### Reuse the shared `PaintOptions` tip

`BrushPresetPicker` resolves its target through
`optionsFor<PaintOptions>(ToolId::Brush)`, but the tool registry passes one
shared `PaintOptions` instance to the Brush, Pencil, and every stamp handler.
The picker therefore edits the tip the active stamp tool reads on its next
stroke; no picker change or per-tool tip copy is needed.

### Keep the picker's non-grabbing contract

The stamp tools use the same `requestBrushPicker` signal as the Brush, so they
inherit the shipped overlay rules: no pointer grab, outside-press dismissal
without consuming the press, and Escape dismissal. The right-click itself does
not begin a stamp stroke, and the Alt-click source sampling path is untouched.

## Alternatives

- **Give only Clone Stamp the picker.** The issue's expected behavior names all
  four tools that share the handler, and the shared `PaintOptions` tip makes the
  subset unnecessary.
- **Build a stamp-specific QMenu.** The picker is the CS6 behavior and is
  already reachable from the options bar, so a second menu would duplicate it.
