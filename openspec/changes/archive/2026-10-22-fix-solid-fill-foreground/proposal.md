# Proposal

## Why

Both Solid Color creation surfaces produce an **opaque black** fill layer and
ignore the toolbox's foreground colour (#793): the Layers panel's
`Solid Color…` entry hard-codes `0xff000000`
(`LayersPanel::LayersPanel`'s strip menu, `layers_panel.cpp`), and
`Layer > New Fill Layer > Solid Color…` reads a third copy of the foreground
that only the alt-eyedropper ever writes (`PicturaMainWindow::foreground_`,
written in `PicturaMainWindow::buildTools`'s eyedropper connect). CS6 fills
with the current foreground colour, and this app's own shape tool already does
(`openspec/specs/tools/rectangle-tool/spec.md`).

## What Changes

- The Layers panel's `Solid Color…` entry and `Layer > New Fill Layer >
  Solid Color…` create the fill layer with the app's current foreground colour.
- The frame's third copy of the foreground (`PicturaMainWindow::foreground_`)
  is deleted. `PicturaMainWindow::foregroundColor()` reads the shared
  foreground colour state instead, so the frame's own accessor for the
  foreground stops reporting a stale value.
- The status-bar tool hint's fallback text now names the current foreground
  colour rather than only the last alt-eyedropper sample. The bar paints that
  fallback only for tools without shortcut hints, so the readout is visible
  for the Eyedropper, the Ruler, the Note, the Count, and the other hint-free
  tools; the default Move and the Brush show keycaps instead.
- Qt Test coverage for both creation surfaces and for the default foreground.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `imaging/adjustment-layer-rendering`: a solid-color fill layer is created
  with the app's current foreground colour, from the Layers-panel fill menu and
  from the Layer menu alike. This capability already owns the sibling
  requirement *The app can create a gradient fill layer*, which covers the
  same two menus, so the solid-colour counterpart belongs beside it.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel.h` — take the shared foreground
  colour state and expose the colour the `Solid Color…` entry fills with.
- `crates/pictura-app/cpp/panels/layers_panel.cpp` — the constructor and the
  `Solid Color…` handler.
- `crates/pictura-app/cpp/frame_build.cpp` — pass the shared colour state to
  the panel; drop the eyedropper's write to the frame's own foreground copy;
  define `foregroundColor()`.
- `crates/pictura-app/cpp/frame.h`, `crates/pictura-app/cpp/frame_menus.cpp` —
  `foregroundColor()` reads the shared colour state; the Layer-menu handler
  uses it.
- `crates/pictura-app/cpp/selftest_workspace_input.cpp` — its alt-eyedropper
  check reads `foregroundColor()`, which now answers from the shared state.
- `crates/pictura-app/cpp/tests/tst_layers_panel_fixes.cpp` and
  `crates/pictura-app/cpp/tests/tst_layer_commands.cpp` — the Qt Test cases.
- `crates/pictura-render/src/document_ops/layer_ops/create.rs` — a comment
  only: the alpha byte is dropped, so a caller may pass a translucent
  foreground's alpha.
- No engine behaviour, bridge, dependency, or data-format change:
  `add_solid_fill` already takes the colour it fills with
  (`crates/pictura-app/src/cxxqt_object.rs`).
