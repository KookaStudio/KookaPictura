# Design

## Sources

- psd-tools 1.19, `psd_tools/psd/layer_and_mask.py`: `MaskFlags` (bit 4
  `parameters_applied`), `MaskData._read_body` (real-mask header before the
  parameters, present only with a `-3` channel; length alone is ambiguous,
  psd-tools #693), `MaskParameters` (selector bits 0-3), write pads the body
  to 4 bytes.
- ag-psd 31.0.3, `dist/psdReader.js` `readLayerMaskData` and
  `dist/psdWriter.js` `writeLayerMaskData`: same field order; density is a
  byte scaled by `0xff`; a record with no raster mask is written with a zero
  rect.

## Decisions

- **Density scale.** Stored as the PSD byte (0-255); the UI shows
  `round(byte * 100 / 255)` %.
- **Feather storage.** `f64::to_bits` in a `u64`, read through `feather()` /
  `set_feather()`: `pictura-core` keeps no floats so `Layer` derives `Eq`, and
  a CS6 value such as `12.3` must re-save byte for byte.
- **Block layout in the model.** `LayerMask.extra` keeps the real-mask header
  (when the layer has a `-3` raw channel) followed by any unknown trailing
  bytes. With parameters, the block and the zero padding after it are parsed
  out and the writer splices the block back after the real-mask header and
  pads to 4; without parameters the tail is kept and re-saved whole, so such
  records stay byte-identical.
- **Vector-only records.** A record whose layer has no `-2` channel holds only
  parameters; its vector half goes to `VectorMask` and the record is dropped,
  so no phantom raster mask appears. When writing, a layer with no raster mask
  but non-default vector parameters gets a zero-rect record with default colour
  255 (ponytail: CS6's own choice of default colour for this record is not
  sourced; 255 is the value that hides nothing in a reader that applies it).
- **Density formula** (inferred, `layer-masks.md` §3):
  `m' = 255 - d * (255 - m) / 255`, so 100 % is the mask as painted and 0 %
  shows the whole layer.
- **Feather** (inferred): `Selection::feather` (`sigma = radius / 2`, box
  stack for wide kernels, clamped at 250 px, the sourced selection-feather
  maximum). The raster plane is padded by `ceil(1.5 * radius)` of its default
  colour before blurring, so the ramp extends past the mask rect. Vector
  coverage is rasterized over the layer's canvas region first. The blurred
  plane is cached on the mask and dropped when the mask's data, rect, or
  feather changes.
- **Invert** is a permanent pixel edit (`255 - m`, default colour flipped
  too), not the obsolete flag bit 2, matching CS6's Properties Invert button.
