# Proposal

## Why

Right-clicking the canvas is a no-op for most tools. CS6 opens the active
tool's context menu, and `docs/02-ui-ux/menus.md` already requires the general
behaviour ("Given a right-click on the canvas, the tool/selection-appropriate
context menu appears"), but only the Zoom preset menu and the Move tool's
Isolation Mode entry are wired. Closed #255 added the `ImageView` event and the
`contextMenuRequested` signal; the routing is still a hard-coded frame chain, so
no other tool can participate. This change generalises the routing and fills in
the menus the corpus already promises.

## What Changes

- **Seam**: add a canvas context-menu hook to the tool handler and route the
  canvas `contextMenuEvent` through the active handler, replacing the frame's
  `if (activeTool() == …)` chain. A tool with no menu leaves the click
  unhandled.
- **Fallback**: a right-click on the pasteboard (outside the document) opens the
  canvas-background menu (colour palette), independent of the active tool.
- **Migrate** the existing Zoom preset menu and the Move tool's Isolation Mode
  entry onto the seam.
- **Move tool**: list every layer with pixels under the pointer (when Auto
  Select is off), alongside Isolation Mode.
- **Eyedropper**: Sample Size and Sample entries.
- **Crop**: common crop options while a crop box is present.
- **Slice / Slice Select**: reposition and Combine Slices.
- **Paint tools** (Brush, Pencil, and the other tools that share the paint
  tip): open the existing Brush Preset picker on canvas right-click. This is a
  deliberate owner decision, not CS6 parity — the corpus documents the picker as
  an options-bar control and the on-canvas size/hardness gesture is the
  `Alt+Right-drag` HUD, not a menu.
- **Deferred**: the vector Shape/Path attribute menus (Copy/Paste Fill, Complete
  Stroke, Copy/Paste Shape Attributes). They need clipboard/attribute machinery
  that does not exist yet, so they belong to their own change.

No breaking changes.

## Capabilities

### New Capabilities

- `ui/canvas-context-menus`: routing a canvas right-click to the active tool's
  context menu, the pasteboard fallback to the canvas-background menu, and the
  contents of each tool's canvas menu.

### Modified Capabilities

None. The new capability owns the canvas-menu behaviour; the tool behaviours it
references stay defined in their existing capabilities.

## Impact

- **C++ app** (`crates/pictura-app/cpp/`): `tool_handler.h` (hook),
  `tools.h`/`tools.cpp` (controller routing + test hook), `image_view.cpp` /
  `frame.cpp` / `frame_menus_select.cpp` (router), `tool_hand_zoom.cpp`,
  `tool_move.cpp`, `tool_eyedropper.cpp`, `tool_crop.cpp`, `tool_slice.cpp`,
  `toolbox.cpp` / `options_bar_paint.cpp` (shared menu accessors),
  `frame_canvas.cpp` (background menu).
- **Rust bridge** (`crates/pictura-app/src/cxxqt_object/`): a
  layers-under-the-pointer query beside `auto_select_layer`.
- **Tests**: a new Qt Test case for the canvas menus; `CMakeLists.txt`
  registration.
- **Dependencies**: none new.
