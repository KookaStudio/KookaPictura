# Proposal

## Why

Saving an imported PSD turns its shape layers into pixel or fill layers. The
layer record's "pixel data irrelevant" flag (bit 4, `0x10`) is dropped on read
and never written back (#1116). The format's bit 3 (`0x08`, "flags valid") is
what tells Photoshop that bit 4 is meaningful, so a faithful save writes the two
together. The vector data survives, but the layer kind silently changes, so an
unchanged save is not faithful.

## What Changes

- Read the layer record's bit 4 into the layer model and write it back on save
  together with bit 3 (`0x08`, "flags valid") whenever bit 4 is set; a stored
  clear bit 4 leaves both bits clear.
- Never infer bit 4 from vector-mask, vector-fill, or origination blocks: a fill
  layer whose source record leaves it clear keeps it clear.
- Set bits 3 and 4 on app-authored shape layers so a shape drawn in Pictura also
  opens as a shape outside Pictura.
- Clear the stored flag when Rasterize Shape or Rasterize Fill Content bakes the
  shape or fill appearance into pixels, so a rasterized layer never keeps a
  stale shape bit.
- Verify the written records with an independent decoder: a round-tripped
  shape stays `shape` with raw flags `0x18`, a pixel layer stays `pixel`, and a
  vector-masked fill layer stays `solidcolorfill`.
- Keep the existing record-flag handling for the other bits; this change does
  not preserve undocumented bits 5-7 or a source bit 3 on a layer whose bit 4 is
  clear.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `codec/psd-layer-io`: add a requirement that the pixel-data-irrelevant record
  flag and its validity bit survive read/save and that authored shape layers
  emit them.
- `compositing/layer-management`: add a requirement that rasterization clears
  the pixel-data-irrelevant flag when it bakes appearance into pixels.

## Impact

- `pictura-core`: `Layer` gains one boolean; fully spelled `Layer { ... }`
  literals across the workspace gain the field or a `Default`.
- `pictura-codec`: layer record read (`read.rs`) and write (`write_layers.rs`).
- `pictura-render`: `add_shape_layer` marks its created layer;
  `rasterize_shape` and `rasterize_fill_content` clear the flag after baking.
- Codec tests gain a focused `layer_record_flags` suite and psd-tools checks
  for fixtures, authored shapes, no-gain cases, and rasterized output; render
  tests cover the two rasterization paths.
- No new fixtures, dependencies, GUI change, or save-format version change.
  Documents whose layers never set bit 4 serialize as before.
