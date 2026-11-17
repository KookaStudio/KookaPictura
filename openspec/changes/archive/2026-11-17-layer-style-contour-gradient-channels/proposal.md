# Proposal

## Why

The Layer Style dialog authors only half of CS6's Bevel & Emboss: the contour
that shapes the bevel profile and the gloss contour that remaps its shading are
written as a fixed linear curve and never edited, and the renderer ignores both.
The same dialog cannot edit a Gradient Overlay's gradient beyond two colour
buttons, and Advanced Blending has no per-channel Include checkboxes even though
the compositor can express them. This is parity gap LAY-3 in
`docs/dev/doc-code-parity-gaps.md`, tracked as issue #581.

## What Changes

- The Bevel & Emboss page gains a **Gloss Contour** picker with its Anti-aliased
  box and a **Contour** sub-panel (checkbox, contour picker, Range 1–100 %,
  Anti-aliased). A small contour editor edits a custom curve; a short preset
  list covers the common shapes.
- The **gradient overlay** page replaces its two colour swatches with the
  existing shared gradient editor, reading and writing the effect's full stop
  list. Opacity stops and stop midpoints stay unedited (#598).
- The Blending Options page gains the document's **Channels Include**
  checkboxes. The model stores the PSD `brst` restrictions, the CPU compositor
  restores excluded channels at the layer boundary (covering effects and
  pass-through groups), and the GPU path declines such layers.
- The bevel renderer applies the gloss contour after shading and the edge
  contour to the chamfer profile, with linear curves remaining byte-identical
  to today's output.
- **BREAKING**: no file-format break. The Rust API gains the contour fields on
  `BevelEmboss` (which is no longer `Copy`) and `Layer.channel_exclude`, so an
  external exhaustive struct literal or by-value copy needs updating. Newly
  authored styles default to today's behaviour: the edge contour is off, both
  contours are linear, no channel is excluded, and a gradient's opacity stops
  and midpoints are preserved. A file that already stores a non-linear contour
  or a `brst` block will now render it instead of ignoring it, intentionally.

Bevel Texture and the Pattern Overlay Snap To Origin stay in #606. Blend If
editing stays in #264, and gradient-editor opacity stops and midpoints stay in
#598. Advanced Blending's other toggles (Transparency Shapes Layer, mask-hides
effects) are tracked in both #579 (engine) and #695 (dialog UI).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-effects`: the bevel decoder exposes gloss and edge
  contours, and compositing remaps the shading and profile through them.
- `compositing/layer-style-authoring`: structured contour and gradient values
  read and write through text keys, and `blending.includeN` writes the `brst`
  restriction set.
- `codec/psd-advanced-blending`: `brst` decodes into the layer, writes back when
  non-empty, gates the CPU blend per channel, and makes the GPU decline the
  stack.
- `ui/layer-style-dialog`: the dialog gains the contour picker/editor, the
  gradient editor, and the Channels Include checkboxes.

## Impact

- Rust: `layer_effects` (bevel decode/render, a new contour module),
  `document_ops/layer_ops/layer_style` (structured authoring), `pictura-core`
  (`Layer.channel_exclude`), `pictura-codec` (`brst`), `pictura-render`
  compositing and GPU planning.
- C++: Layer Style pages and dialog bindings, one new contour swatch/editor
  pair, the shared `CurveWidget` gains a linear interpolation mode, the shared
  `GradientEditorDialog` gains a solid-only mode, CMake source registration, and
  the Qt layer-style suite.
- `LayerStyle` copy/paste carries the channel restrictions.
- Specs: the four capabilities above.
- No new dependencies and no file-format break. The only `docs/` change is the
  LAY-3 parity-register close-out, committed with `TASK-ALLOWS-DOCS`.
