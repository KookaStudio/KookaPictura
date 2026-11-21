# Proposal

## Why

Issue [#598](https://github.com/KookaStudio/KookaPictura/issues/598) records the
last Gradient Editor gaps from the code-parity audit (#486):

- The Gradient tool's options bar can only pick a built-in preset. Clicking the
  wide sample does not open the Gradient Editor, so a user cannot define the
  gradient the tool draws.
- The editor's opacity row is permanently disabled, and its two opacity markers
  are painted at 100 % and do not edit. A gradient's transparency therefore
  cannot be authored in the editor.
- Colour stops have no midpoint handle or field, so the even-mix point cannot be
  moved.

The shared `GradientEditorDialog` already serves Gradient Map, Gradient Fill,
and the layer-style Gradient Overlay. Fixing the editor's model once gives all
four consumers opacity stops and midpoints.

## What Changes

- Extend the solid-gradient model with an opacity-stop list and a per-stop
  midpoint. The engine samples colour and opacity independently and applies the
  midpoint as a piecewise bias between adjacent stops.
- Add the Gradient Editor to the Gradient tool's sample: the main part of the
  sample opens the editor, the arrow still opens the preset menu. Accepting the
  editor makes the edited gradient the tool's current ramp; choosing a preset
  returns to the built-in ramp.
- Paint, select, drag, add, and delete opacity stops above the ramp, and draw
  and drag a midpoint diamond between adjacent colour or opacity stops.
- Keep the selected stop's row live in the Stops group: colour stops expose
  Color / Location / Delete, opacity stops expose Opacity / Location / Delete,
  and both expose Midpoint.
- Bake opacity stops and midpoints into the RGBA ramp for the consumers that
  store only a stop list (Gradient Map, Gradient Fill, Gradient Overlay).

## Capabilities

### Added Capabilities

- `color/gradient-editor`: the solid gradient carries opacity stops and
  midpoints, and the editor exposes them.

### Modified Capabilities

- `tools/gradient-tool`: the Gradient Editor is reachable from the tool and the
  edited ramp is what the next drag draws.

## Impact

- `crates/pictura-paint/src/gradient.rs`
- `crates/pictura-app/src/cxxqt_object/paint_tools/fills.rs` and `fills/imp.rs`
- `crates/pictura-app/cpp/gradient_editor_dialog.{h,cpp}`
- `crates/pictura-app/cpp/options_bar_fill.cpp`
- `crates/pictura-app/cpp/tool_fills.cpp`, `tool_options.{h,json.cpp}`
- `crates/pictura-app/cpp/gradient_{map,fill}_dialog.cpp`,
  `layer_style_pages.cpp`
- `crates/pictura-app/cpp/tests/tst_fill_tools.cpp`,
  `tst_image_adjustments.cpp`
- `ROADMAP.md`, `PARITY.md`, `docs/dev/STATE.md`
