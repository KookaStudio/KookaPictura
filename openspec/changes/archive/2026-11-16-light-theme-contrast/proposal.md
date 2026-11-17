# Proposal

Tracking issue: #1382.

## Why

The light family hides UI text that the dark family shows. Two custom-painted
surfaces derive their colours with a fixed lightening step: the Layers row
delegate paints a selected row's surface with `Theme::shade(Window, +3)`, which
clamps to white in the light family while the selected name is drawn in
`HighlightedText` (also white); the Channels rows use the same step. The Brush
panel's tip presets paint their glyph and size label `Qt::white`, so they vanish
on the light panel.

The light-family contract already requires derived shades to move toward the
family's shaded direction and custom-painted widgets to follow the family. The
row surfaces and the brush tip ink were missed.

## What Changes

- Add `Theme::contrastShade(color, steps)`: a family-aware step that lightens on
  the dark family and darkens on the light family.
- The Layers row delegate derives its row, gutter, selection, and Blend If chip
  surfaces with `contrastShade`, so the selected row stays a distinct neutral
  grey and its white name stays legible in both families.
- The Channels panel derives its row surfaces with the same helper and re-styles
  on a palette or style change, so a live family switch follows into paint.
- The Brush panel draws its tip presets and size labels in the `icon` token and
  regenerates them on a palette or style change.
- Qt Test coverage for the helper, both row panels, and the brush tip ink.

## Capabilities

### Modified Capabilities

- `ui/design-system`: derived custom-painted surfaces step toward the family's
  contrasting direction; a new scenario pins row and tip contrast in the light
  family.
- `ui/layers-panel`: row, gutter, and selection surfaces step toward contrast
  rather than always lighter, and the selected name stays legible in both
  families.
- `ui/channels-panel`: channel rows follow the same family-aware shading.

### Added Capabilities

- `ui/brush-panel`: tip preset glyphs draw in the family's icon ink and repaint
  when the family changes.

## Impact

- `crates/pictura-app/cpp/theme.h`, `theme.cpp`: the helper.
- `crates/pictura-app/cpp/panels/layers_panel_internal.h`: the row paint.
- `crates/pictura-app/cpp/panels/channels_panel.h`, `channels_panel.cpp`: row
  styling and `changeEvent`.
- `crates/pictura-app/cpp/panels/brush_panel.h`, `brush_panel.cpp`: tip ink and
  `changeEvent`.
- Tests: `tst_design_system.cpp`, `tst_layers_panel.cpp`,
  `tst_panels_parity.cpp`, `tst_brush_dynamics.cpp`.
