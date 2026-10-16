# Proposal

## Why

Issue #260: layer and vector masks carry no Density or Feather, so the
Properties panel shows them (and the layer mask's Invert) as disabled
`— not implemented yet` rows. The PSD mask-parameters block that stores them is
kept only as opaque bytes (`LayerMask.extra`), so a CS6 file's density and
feather round-trip but never render and cannot be edited.

The block layout is now sourced from two independent decoders, psd-tools 1.19
(`MaskData`, `MaskParameters`) and ag-psd 31 (`readLayerMaskData`), which
agree on every field: mask flags bit 4 marks the block; a selector byte then
gates user density (`u8`, 0-255), user feather (`f64`), vector density (`u8`),
and vector feather (`f64`), in that order; the 18-byte real-user-mask header
precedes the parameters and is present only with a `-3` channel.

## What Changes

- **Model.** `LayerMask` and `VectorMask` gain `density` (0-255, default 255)
  and a feather radius in pixels (default 0), stored as `f64` bits so `Layer`
  keeps `Eq` and a CS6 value re-saves exactly.
- **Codec.** The reader parses the parameters block (real-mask header keyed on
  the `-3` channel, as psd-tools does), routes the vector half to
  `VectorMask`, and drops a parameter-only record that carries no `-2`
  channel. The writer rebuilds the block and pads such a record to 4 bytes, writing a
  parameter-only record when only the vector mask has non-default values.
- **Compositing.** Density lifts a mask toward white
  (`m' = 255 - d * (255 - m) / 255`); feather blurs the coverage with the
  selection feather's Gaussian (`sigma = radius / 2`, clamped at 250 px),
  cached per mask. Raster and vector parts are refined separately before they
  multiply. Both formulas are inferred; Adobe's are closed.
- **Edits.** `set_{layer,vector}_mask_{density,feather}` (one undo step each)
  and `invert_layer_mask`, a permanent pixel inversion (CS6's Properties
  Invert button), one undo step.
- **UI.** The Properties Mask section gets Density (0-100 %), Feather
  (0-250 px, one decimal), and an Invert button; the Vector Mask section gets
  Density and Feather.
- **BREAKING**: none. A record without parameters re-saves byte for byte.

## Capabilities

### New Capabilities

- `codec/psd-mask-parameters`: read and write the mask-parameters block.
- `compositing/mask-refinement`: density, feather, and invert for layer and
  vector masks.

### Modified Capabilities

- `ui/properties-panel-masks`: Density, Feather, and Invert become live.
- `ui/properties-panel-vector-masks`: Density and Feather become live.

## Impact

- `pictura-core` (`LayerMask`, `VectorMask`), `pictura-codec` (`read.rs`,
  `write.rs`, new `mask_params.rs`), `pictura-render` (mask sampling, document
  ops), `pictura-app` (bridge, Properties panel, Qt Test).
- `pictura-render` gains a dependency on the workspace crate `pictura-select`
  to reuse its feather blur. No new external dependencies.
