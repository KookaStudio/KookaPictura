# Clone Stamp canvas brush picker

## Why

Issue #880 reports that the Clone Stamp (and the other tools built on
`StampToolHandler`) have no canvas context menu: right-clicking over the
document does nothing, while CS6 opens the Brush Preset picker for its paint
tools. The shipped `ui/canvas-context-menus` requirement scopes that picker to
the Brush and Pencil only.

## What Changes

- The Clone Stamp, Pattern Stamp, History Brush, and Art History Brush gain the
  same canvas right-click menu as the Brush and Pencil: the Brush Preset picker
  opens at the pointer with Size, Hardness, and the preset grid.
- Choosing a preset in that picker changes the active paint tip for the stamp
  tool that opened it. All four stamp tools paint through the shared
  `PaintOptions` tip, so no per-tool tip state is introduced.
- The shipped `ui/canvas-context-menus` paint-tool requirement is extended to
  name the four stamp tools.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/canvas-context-menus`: the paint-tool canvas menu requirement now names
  the Clone Stamp, Pattern Stamp, History Brush, and Art History Brush in
  addition to the Brush and Pencil.

## Impact

- **C++ app**: `crates/pictura-app/cpp/tool_stamps.cpp` — the shared
  `StampToolHandler` base gains the context-menu overrides.
- **Tests**: two new cases in `tst_canvas_context_menus`.
- **No document-format change, no new dependency, no `docs/` edit.**
