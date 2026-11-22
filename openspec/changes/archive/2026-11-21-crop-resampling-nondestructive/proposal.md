# Crop resampling and non-destructive Reveal All

## Why

Issue #544 closes parity row TOOL-20. The Crop tool already has the interactive
box, straighten, overlay, Classic mode, and the full Settings menu including the
crop shield, but two parts of the CS6 contract are missing:

- `W x H x Resolution` only resizes the crop box. Committing a crop never
  resamples to the requested output size or writes the requested resolution, so
  the mode produces no resampled image.
- With `Delete Cropped Pixels` off the layer rasters are retained, but the
  document has no `Image > Reveal All` command. The catalog id exists
  (`image.revealAll`) with no menu row, handler, or engine operation, so the
  retained pixels cannot be recovered.

`docs/dev/doc-code-parity-gaps.md` records the gap as TOOL-20.

## What Changes

- `W x H x Resolution` becomes an output-size mode: the W/H fields are the
  requested pixel dimensions, the box is locked to their ratio, editing W/H
  keeps the target while refitting the box, and Swap exchanges the target.
- Committing in that mode crops to the box, resamples the result to exactly the
  requested W x H with Bicubic (the documented General-preference default; no
  General interpolation preference exists yet), and writes the requested ppi
  with its `px/in` or `px/cm` unit. Straighten commits honour the same target.
- `Image > Reveal All` is added to the command catalogue and menu. It grows the
  canvas to the union of the canvas and every layer's content (rect, mask, and
  retained native store), records one `Reveal All` history state, and clears the
  selection when the canvas changes. With no content outside the canvas it is a
  no-op.
- `Delete Cropped Pixels` off keeps the existing retained-raster behaviour; the
  new Reveal All makes it usable. Destructive crops stay destructive because the
  pixels are trimmed.
- `ROADMAP.md`, `PARITY.md`, and the parity register move together.

Explicitly out of scope: a stored crop-region metadata resource (the retained
layer rasters and their union provide the reveal extent and survive a PSD
round-trip), rendering retained pixels outside the canvas during the crop
preview, and a General Preferences interpolation control.

## Capabilities

### New Capabilities

- `tools/crop-resampling`: the `W x H x Resolution` output-size mode, its box
  ratio lock, and the resampled, resolution-writing commit.
- `tools/non-destructive-crop`: retaining pixels when `Delete Cropped Pixels` is
  off, and the `Image > Reveal All` command that restores them.

## Impact

- `crates/pictura-render`: a `reveal_all` document operation over the layer
  content union.
- `crates/pictura-app`: `crop_group` gains resampled crop/straighten bridges;
  `image_ops` gains `reveal_all`; `CropOptions` carries the output target;
  `options_bar_crop.cpp` drives the target; `menu_image.cpp` wires the command.
- `crates/pictura-script`: the `image.revealAll` catalogue row.
- Qt tests: `tst_crop_options` for the target mode and resampled commit,
  `tst_image_ops` for Reveal All, its undo, and its no-op.
- Rust tests: `pictura-render::crop` for the reveal bounds.
- No new dependencies; the existing Bicubic resampler and image-resource
  resolution writer are reused.
