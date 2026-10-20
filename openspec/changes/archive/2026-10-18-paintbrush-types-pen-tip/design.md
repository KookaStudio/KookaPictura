# Design: paintbrush-types-pen-tip

## Context

Issue #731 supplies five screenshots and the caption "pen tip model"; it has no
body text and no `.abr` file. The screenshots show Photoshop's Brush Preset
picker with four brushes, the strokes they paint, and a gray panel holding a
black pen nib at the canvas top-left (the "pen tip model"). The current picker
already ships 44 CS6 presets, but a preset only carries tip geometry and dab
dynamics; the Brush tool's `aliased` edge is a tool flag, not a preset field.

## Decisions

### Presets

Append the four presets after the existing 44, so every existing preset index
(and the self-test's `Spatter 24` at index 25) stays put. The settings are
approximated from the screenshots; there is no Photoshop tip data to port.

| Preset | Size | Hardness | Roundness | Spacing | Dynamics | Grain | Pen tip |
|---|---|---|---|---|---|---|---|
| `2B Pencil` | 10 | 100 | 100 | 8 | size jitter 20 | 30 | yes |
| `Airbrush Soft Round 50% flow` | 45 | 0 | 100 | 25 | none | 0 | no |
| `Non-Photo Blue Pencil` | 9 | 100 | 100 | 8 | none | 30 | yes |
| `Paintbrush Oval 45 pixels Multiply` | 45 | 60 | 35 | 15 | none | 0 | no |

The names' `50% flow` and `Multiply` are descriptive only. The answer to the
scope question was "tip only, like Photoshop", so selecting a preset does not
change Mode, Opacity, Flow, or the foreground colour. The pencil presets use
hard anti-aliased Brush tips, not the Pencil tool's aliased edge; `2B Pencil`
adds the paper grain seen in the owner's stroke screenshot. The values are
approximations and are marked non-parity.

### Grain

`StrokeConfig` gains `grain` (0–100 %), set from the preset through
`PaintOptions::grain`. For each painted pixel the stroke modulates its
accumulated alpha by `1 − grain % × (1 − noise)`, where `noise` is
`fill::pixel_noise`, a fixed per-pixel hash keyed to the layer coordinates.
The modulation is applied to the final alpha of every dab, so the grain
pattern stays fixed while overlapping dabs only deepen it, and the same stroke
always renders the same pixels. The GPU dab shader does not model grain, so a
grainy stroke stays on the CPU oracle.

### Tip model

`PaintOptions` gains `penTip_` (default false), set by every preset application
and persisted as `penTip` in the brush tool-options JSON. It is a preview-only
property: the stroke engine never reads it. `PaintOptions` also carries
`grain_`, persisted as `grain`; unlike `penTip`, the stroke engine reads it.

### Preview overlay

`BrushTipPreview` is a `QWidget` child of each document's `ImageView`, created
in `addDocument` and positioned at (8, 8). `PicturaMainWindow` keeps the pointer
in `DocEntry` and calls `syncBrushTipPreview()` from `refresh()`,
`activeToolChanged`, and `brushTipChanged`. It shows only when the active
document's canvas has a document, the active tool is Brush or Pencil, and
`PaintOptions::penTip()` is true; otherwise it is hidden.

The overlay paints a 2D model at the target screenshot's proportions (150 ×
176 px): a gray panel, a dashed ground line near the bottom, a narrow light
barrel from the top edge, and a black nib with a straight ferrule and a tapered
point touching the line. It is draggable (press/move/release, clamped to the
canvas) and consumes its own mouse events, so a drag cannot start a paint
stroke. It is not a 3D render and has no Window-menu toggle.
`ponytail:` the model is a painted 2D approximation of Photoshop's rotatable 3D
Brush Preview; no OpenGL/Qt3D.

### Tests

New `tst_brush_tip_preview` Qt suite: the four preset names/order and count, a
preset applying only the tip (mode/opacity/flow/foreground unchanged), the
pencil presets setting and clearing the model and their grain values,
show/hide on preset and tool changes, a drag that moves the overlay without a
canvas press, clamping, and the per-document preview following the active tab.
A `pictura-paint` unit test proves the grain breaks up the dab alpha and
reduces coverage without erasing the dab.

## Risks

- The four preset values, including the grain strength, are approximations; no
  oracle exists for a custom brush pack.
- The overlay is a 2D approximation of Photoshop's 3D Brush Preview.
- The preview is per-document and does not persist its dragged position across
  restarts.
