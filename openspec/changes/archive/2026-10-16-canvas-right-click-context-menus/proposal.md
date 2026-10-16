# Proposal

## Why

Right-clicking the canvas is a no-op for most tools. CS6 opens the active
tool's context menu, and `docs/02-ui-ux/menus.md` already requires the general
behaviour ("Given a right-click on the canvas, the tool/selection-appropriate
context menu appears"), but only the Zoom preset menu and the Move tool's
Isolation Mode entry are wired. Closed #255 added the `ImageView` event and the
`contextMenuRequested` signal; the routing is still a hard-coded frame chain, so
no other tool can participate. This change generalises the routing and fills in
the menus the corpus promises.

## What Changes

- **Seam**: add a canvas context-menu hook to the tool handler and route the
  canvas `contextMenuEvent` through the active handler, replacing the frame's
  `if (activeTool() == …)` chain. A tool with no menu leaves the click
  unhandled, and the menu acts on the document whose canvas was right-clicked.
- **Fallback**: a right-click on the pasteboard (outside the document) opens the
  canvas-background colour menu, independent of the active tool.
- **Move tool**: list every layer with a pixel under the pointer (when Auto
  Select is off), alongside the existing Isolation Mode entry.
- **Eyedropper**: Sample Size and Sample entries, matching the options bar.
- **Crop**: common crop options while a crop box is present.
- **Paint tools**: on canvas right-click, open the existing Brush Preset picker
  for the Brush and Pencil tools. This is a deliberate owner decision, not CS6
  parity — the corpus documents the picker as an options-bar control and the
  on-canvas size/hardness gesture as the `Alt+Right-drag` HUD.

The Zoom preset menu is already specified by the unarchived
`canvas-zoom-and-active-layer` change (its `tools/canvas-tools` delta, landed in
#256); this change only routes it through the new seam and does not re-specify
it.

Out of scope, deferred to their own work:

- The vector Shape/Path attribute menus (Copy/Paste Fill, Complete Stroke,
  Copy/Paste Shape Attributes) — they need clipboard/attribute machinery that
  does not exist yet.
- The Slice menu (Combine Slices, stacking order) — `slices.rs` has no combine,
  reordering, multi-selection, or layer-based-slice model; tracked by the
  Divide/Combine/Slice gap (#543).
- The Color Sampler sample-size menu (`MarkerToolHandler` carries no sample-size
  option).
- Canvas-background **Snap** settings (the stored canvas colours are the only
  pasteboard entries today).

No breaking changes.

## Capabilities

### New Capabilities

- `ui/canvas-context-menus`: routing a canvas right-click to the active tool's
  context menu (acting on the clicked document), the pasteboard fallback to the
  canvas-background menu, and the contents of the Move, Eyedropper, Crop,
  canvas-background, and paint-tool menus.

### Modified Capabilities

None. The Zoom menu stays owned by `tools/canvas-tools` (via the sibling
change); the new capability owns only the menus it introduces.

## Impact

- **C++ app** (`crates/pictura-app/cpp/`): `tool_handler.h` (hook),
  `tool_services.h` (new service methods), `tools.h`/`tools.cpp` (controller
  routing, signals, test hook), `image_view.cpp` / `frame.cpp` /
  `frame_menus_select.cpp` / `frame_canvas.cpp` (router, background menu),
  `tool_move.cpp`, `tool_eyedropper.cpp`, `tool_crop.cpp`, `tool_brush.cpp`,
  `panels/brush_preset_picker.{h,cpp}` (`popUpAt`).
- **Rust bridge** (`crates/pictura-app/src/cxxqt_object/`): a
  layers-under-the-pointer query beside `auto_select_layer`.
- **Tests**: `crates/pictura-app/cpp/tests/tst_canvas_context_menus.cpp`; a
  Rust unit test for the bridge; `CMakeLists.txt` registration.
- **Dependencies**: none new.
