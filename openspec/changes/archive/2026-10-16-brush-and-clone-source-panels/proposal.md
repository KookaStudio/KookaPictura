# Proposal: brush-and-clone-source-panels

## Why

CS6's Clone Stamp options bar carries Toggle the Brush panel and Toggle the
Clone Source panel buttons (issue #17 follow-up). Neither panel existed:
`Window > Panels > Brush` and `Clone Source` were disabled leaves, the brush
tip's roundness, angle, and spacing were hard-coded at every paint-tool begin,
and the Clone Stamp had a single source with no transform
(`docs/02-ui-ux/panels/brushes-panel.md`, `docs/02-ui-ux/panels/clone-source-panel.md`).

## What Changes

- Brush panel (`Window > Panels > Brush`, `F5`): the Brush Tip Shape page — a
  row of hard / soft round tips, Size, Flip X / Flip Y, Angle, Roundness (with
  a tip-shape indicator), Hardness, Spacing, and an engine-rendered stroke
  preview. Every painting tool reads the tip: the controller gains Roundness,
  Angle, Spacing, and the flips (a single flip mirrors an elliptical tip's
  angle), and the paint-tool bridge takes one shared `PaintTip`. The dynamics
  list, Brush Presets, and the Spacing checkbox are shown disabled; the panel
  menu lists the spec's entries, disabled.
- Clone Source panel (`Window > Panels > Clone Source`): five source slots,
  each keeping its own Alt-clicked source and offset; Offset X / Y (destination
  minus source, editable once a source exists); W / H with Maintain Aspect
  Ratio, Rotate, Flip Horizontal / Vertical, and Reset Transform. The overlay
  controls and the Extended frame controls are shown disabled.
- `pictura_paint::stamp::SourceTransform` and `StampSource::transformed`: the
  clone source is scaled, flipped, and rotated about the point the offset was
  measured at, with premultiplied bilinear sampling.
- Clone Stamp options bar: Toggle the Brush panel and Toggle the Clone Source
  panel after the tip fields; they show a hidden panel or hide a shown one.
- Both panels join the hidden overflow panel group.
- C++ self-tests `brush_panel` (545) and `clone_source_panel` (546).

## Capabilities

### New Capabilities

- `ui/brush-panel`: the Brush panel and the shared brush tip.
- `ui/clone-source-panel`: the Clone Source panel, the source transform, and
  the Clone Stamp's panel toggles.

## Impact

- `pictura-paint` (`stamp.rs`), `pictura-app` bridge (`paint_tools.rs`) and C++
  (panels, controller, options bar, frame, menus).
- No new dependency.

## Provenance

Original to Kooka, from the CS6 specs above. Behavioural parity only: tip
presets are generated round tips, not Adobe's set; the source overlay, sampled /
bristle / erodible tips, brush dynamics, velocity spacing, and Brush Presets are
`ponytail:` ceilings.
