# Proposal

## Why

Some Photoshop PSDs retain a layer's `vmsk` after its vector mask has been
rendered into a raster layer mask. The mask record then carries the
`user_mask_from_render` flag (bit 3, `0x08`), and Photoshop shows only the
raster mask. Pictura imports the leftover `vmsk` as a live vector mask, so the
Layers panel and Properties panel show a second mask Photoshop does not have
(#1286). The reference file
`reference-fixtures/OpenTS_logo2.1_Hyper-V1.psd` reproduces it on its `Triangle`
and `Chevron` pixel layers. The Solid Color adjustment layer named in the issue
is not present in that file; layers whose record bit 4 is set stay out of scope
(see `design.md`).

## What Changes

- On PSD/PSB import, do not derive `Layer.vector_mask` from a layer's `vmsk`
  block when all of these hold:
  - the layer record's pixel-data-irrelevant flag (bit 4) is clear, so the layer
    is a pixel layer, not a shape;
  - the layer's raster mask record has its `user_mask_from_render` flag (bit 3)
    set;
  - the layer has no real-user-mask (`-3`) channel, so no live raster+vector
    composite exists.
- Keep the raw `vmsk` block in `Layer.extra_blocks` and re-emit it verbatim on
  save. The writer clears the raster mask's render flag for a pixel layer with
  a live vector mask, so an authored or edited live mask survives a save; no
  file-format change.
- Do not model the suppressed path's mask-record vector density/feather: with
  no live vector mask there is no carrier, so the writer normalizes those inert
  fields to their defaults on save. The raster mask's user density and feather
  still round-trip unchanged.
- Keep the derived view for shape layers (bit 4 set) and for layers that carry
  a `-3` real-user-mask channel or a raster mask without the render flag.
- Commit one synthetic fixture for the suppressed case and cover the predicate
  with unit tests.
- Out of scope: decoding `vsms` shape layers (#1114) and the display of a
  shape layer's derived raster mask.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/psd-layer-io`: the derived `vmsk` vector-mask view gains a carve-out
  for a rasterized vector-mask source path on a pixel layer.

## Impact

- `crates/pictura-codec/src/vector_mask.rs`: the import-time predicate.
- `crates/pictura-codec/src/mask_params.rs`: one mask-flag constant.
- `crates/pictura-codec/tests/fixtures/`: one new fixture and its README row,
  authored by `scripts/generate-fixtures.py`.
- `crates/pictura-codec/tests/vector_mask_oracle.rs` and the `vector_mask`
  unit tests: coverage.
- No render, UI, application, dependency, or performance change. The writer
  clears the render flag for a live pixel-layer vector mask, and a suppressed
  source path's inert vector density/feather fields are normalized on save; its
  raw `vmsk` block is still preserved. The raster mask's user density and
  feather round-trip unchanged.
