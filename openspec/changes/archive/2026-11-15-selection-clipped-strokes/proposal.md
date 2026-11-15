# Proposal

## Why

With an active selection, the Brush paints across the marching-ants boundary
(#1362). The selection is the document's editing mask: the glossary defines it
as "a document-sized coverage mask isolating part of an image so edits affect
only those pixels" (`docs/GLOSSARY.md`, `docs/08-selection/selection-model.md`).
Fills, gradients, bucket fills, filters and adjustments already honour it; the
stroke-based paint tools do not. Painting a layer mask while a selection is
active has the same gap.

The engine already has a per-stroke coverage clip: Quick Mask sets it so a
temporary selection constrains mask painting. It is layer-local, which is why
the ordinary paint tools never used it — a layer smaller than the canvas or one
that grows under the brush needs document coordinates.

## What Changes

- Add a document-space `CoverageClip` to `pictura_paint::Stroke`, with a
  document origin so a canvas-sized selection clips a layer stroke, a mask
  stroke (whose stroke document is mask-local), and Quick Mask alike.
- Every stroke-based paint tool begins with the active selection as its clip:
  Brush, Pencil, Eraser, Clone Stamp, Pattern Stamp, History Brush, Art History
  Brush, Mixer Brush, Smudge, Blur/Sharpen, Dodge/Burn/Sponge, Color
  Replacement, and Background Eraser.
- A stroke that targets a layer mask is clipped by the same selection.
- The reduced-level preview and the GPU stroke honour the clip; a selection
  disables the GPU dab so the exact clipped CPU stroke owns the committed
  pixels.
- No selection keeps today's behaviour, including an all-pixels selection
  behaving as no constraint.

Out of scope: healing gestures (they carry their own stroke mask and heal at
commit), and non-stroke commands that already crop the selection themselves.

## Capabilities

### New Capabilities

- `tools/selection-clipped-painting`: the active selection constrains every
  stroke-based paint tool on layer pixels and layer masks, including the live
  preview and committed pixels.

## Impact

- `crates/pictura-paint`: `CoverageClip` in the stroke engine; the stencil
  samples it in document coordinates.
- `crates/pictura-app` Rust: the selection clip is attached in `begin_kind`,
  `begin_paint`, `begin_mask_paint`, and the Quick Mask path; the preview
  carries it and the GPU path declines while it is set.
- Tests: a `pictura-paint` unit test for document-space sampling, offset
  layers and growth, and Qt Test cases for layer and mask strokes.
- No new dependencies.
