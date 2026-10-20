# Proposal: paintbrush-types-pen-tip

## Why

The Brush Preset picker ships CS6's default set but none of the four brushes
in issue #731's screenshots (`2B Pencil`, `Airbrush Soft Round 50% flow`,
`Non-Photo Blue Pencil`, `Paintbrush Oval 45 pixels Multiply`), and it has no
on-canvas pen-tip preview. The issue asks for both. The screenshots are the
only reference: there is no `.abr` file or tip data, so the tip settings are
approximated from the strokes and labelled non-parity.

## What Changes

- `crates/pictura-app/cpp/panels/brush_preset_picker.cpp`: append the four
  presets to the default set, each with a `penTip` preview-model flag.
- `crates/pictura-app/cpp/tool_options.h`: `PaintOptions` gains the `penTip`
  and `grain` fields (set by a preset; persisted with the other brush fields).
- `crates/pictura-paint/src/{lib,stroke}.rs`: `StrokeConfig` gains `grain`, a
  fixed per-pixel alpha drop keyed to the layer coordinates that gives the 2B
  Pencil its paper texture.
- New `crates/pictura-app/cpp/brush_tip_preview.{h,cpp}`: a 2D painted pen-tip
  model on a gray panel with a dashed ground line. It is a child of each
  document's `ImageView`, shown at the canvas top-left while the Brush or
  Pencil tool has a pen-tip preset, draggable within the canvas, and never
  starts a stroke.
- `crates/pictura-app/cpp/frame.{h,cpp}`, `frame_build.cpp`: one preview per
  document, synced on document, tool, and tip changes.
- New `crates/pictura-app/cpp/tests/tst_brush_tip_preview.cpp` Qt Test suite
  and its `tests/CMakeLists.txt` entry.
- `docs/dev/STATE.md`: record the feature.

## Capabilities

**Modified:** `ui/brush-preset-picker` (ADDED: the four presets and their
tip-only application).

**Added:** `ui/brush-tip-preview` (the on-canvas pen-tip model).

## Impact

- The existing round/elliptical tip engine paints the presets. `2B Pencil` adds
  a fixed paper grain to the stroke alpha; it is a hard-edged anti-aliased
  Brush tip, while the Pencil tool supplies the aliased edge. Grain stays on
  the CPU oracle; the GPU dab shader does not model it.
- No new dependency. The preview is QPainter 2D, not a 3D render.
- Ceiling: the exact Photoshop tip settings are unavailable; the four presets
  are tuned from the screenshots and marked non-parity. Mode, Opacity, Flow,
  and colour are deliberately not stored in a preset, as in Photoshop.
