# Proposal: brush-dynamics-option-sets

## Why

The Brush panel lists Shape Dynamics, Scattering, Color Dynamics, Transfer, and
Brush Pose but disables every row, and the engine only knows fixed random
jitter (issue #492). CS6's dynamics also take a **Control** input — pen pressure,
tilt, wheel, rotation, direction, or a fade over a dab count — and combine it
with jitter; the panel has per-set padlocks and the sets must survive preset
switches. None of that exists.

This is the first of three changes for #492. It adds the Control model, the five
dynamics sets that do not need new raster sources, tablet tilt/rotation routing,
and the per-set padlocks. Texture and Dual Brush follow in their own changes.

## What Changes

- `pictura-paint` gains `Dynamics` on `StrokeConfig`: per-attribute Control,
  minimum diameter/roundness, tilt scale, Brush Projection, Both Axes, Count
  jitter, Transfer opacity/flow jitter, Color Dynamics (Apply Per Tip, FG/BG,
  hue/saturation/brightness, purity), and Brush Pose overrides.
- `Stroke::sample_with` takes a `DynamicsInput` (pressure, tilt X/Y, rotation,
  wheel). `Stroke::sample` keeps its old signature and supplies pressure only.
  A dab's random draws still come from the stroke's fixed seed.
- The C++ `PaintTip` carries a `PaintDynamics` block, so every paint tool's
  bridge passes the same dynamics. `begin_brush` and the stamp/retouch begins
  share it.
- `QTabletEvent` tilt and rotation reach the active stroke through
  `ImageView`, `ToolController`, `ToolServices`, and `paint_api::Stroke`.
- The Brush panel enables Shape Dynamics, Scattering, Color Dynamics, Transfer,
  and Brush Pose, with a page per set, enable checkboxes, and a per-set padlock.
  Texture and Dual Brush stay visibly disabled until their changes.
- `BrushDynamics` and its new fields persist in the tool-options JSON and are
  applied by the brush preset picker, except for sets whose padlock is on.

## Capabilities

### New Capabilities

- `tools/brush-dynamics-option-sets`: the Control model and the Shape
  Dynamics, Scattering, Color Dynamics, Transfer, and Brush Pose semantics.

### Modified Capabilities

- `ui/brush-panel`: the option sets become interactive, with padlocks and
  persistence, instead of disabled placeholders.

## Impact

- `crates/pictura-paint`: `lib.rs`, `stroke.rs`, `stroke/raster.rs`,
  `stroke/stencil.rs`, new `dynamics.rs`.
- `crates/pictura-app`: `paint_tools` bridge, `impl_paint`, `tool_options`,
  `tool_options_json`, `paint_tip.h`, `paint_api`, `image_view`, `tools`,
  `panels/brush_panel`, `panels/brush_preset_picker`.
- No new dependency. No document or PSD format change.

## Provenance

`docs/07-color-painting/brush-dynamics.md` (BRU-002) and
`docs/02-ui-ux/panels/brushes-panel.md` (PAN-012). Adobe's exact curves,
ranges, and the jitter×control combine rule are unpublished; this change is
behavioural parity, not a claim of verified Adobe parity.
