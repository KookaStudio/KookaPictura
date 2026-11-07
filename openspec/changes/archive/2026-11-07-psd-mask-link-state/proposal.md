# Proposal

## Why

Photoshop-linked raster masks open as unlinked because Pictura interprets PSD
mask flag bit 0 with reversed polarity (#1200). The user also wants an unlinked
mask's link control to remain visible at 50% opacity.

## What Changes

- Interpret raster-mask bit 0 as unlinked when set and linked when clear;
  linking clears that bit and unlinking sets it, preserving every other bit.
- Create linked raster masks with bit 0 clear, including masks created by
  vector-mask rasterization and Auto Blend.
- Keep PSD mask flags and document-space bounds intact through read/save;
  verify both link states with small independently authored fixtures.
- Draw the existing linked glyph at 100% opacity and the existing unlinked
  glyph at 50% opacity for raster and vector mask controls, keeping both
  clickable and undoable.
- Replace the incorrect `Mask link flag` requirement with `Raster mask link
  flag polarity`, explicitly retiring its reversed-bit scenarios.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `compositing/layer-masks`: correct the raster-mask link-bit polarity and
  defaults for newly created masks.
- `codec/psd-layer-io`: specify link-state preservation and independent
  coverage of both flag values without translating mask bounds.
- `ui/layers-panel`: specify full-opacity linked and half-opacity unlinked
  mask controls and preserve their interaction.

## Impact

- Render mask predicates/setters, the exported flag constant and its raster
  creation callers; codec integration/oracle tests and fixture generation.
- Layers row delegate, Qt Test coverage, and existing row-image test seams.
- No new dependencies or core-model fields. Vector-mask encoding and the
  separate layer-to-layer link badge retain their existing behavior.
- Older Pictura saves may contain the opposite bit for their intended state;
  this change follows the PSD encoding without guessing file provenance or
  silently rewriting stored flags.
- The supplied Photoshop documents are local acceptance references, not
  committed fixtures. Implementation and release checks follow this proposal
  in a separate apply step.
