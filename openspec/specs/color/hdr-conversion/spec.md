# hdr-conversion Specification

## Purpose
32-bit to 16/8-bit Exposure and Gamma conversion and the HDR Conversion dialog.

## Requirements

### Requirement: 32-bit to 16/8 Exposure and Gamma conversion

`pictura_render` SHALL expose
`convert_depth_exposure_gamma(doc: &mut Document, out: BitDepth, params:
ExposureGamma) -> Result<(), OpsError>`. It SHALL refuse, without mutating
`doc`, unless `doc.source_depth == Some(BitDepth::ThirtyTwo)`,
`doc.source_mode.is_none()`, `doc.mode` is `ColorMode::Rgb` or
`ColorMode::Grayscale`, and `out` is `BitDepth::Sixteen` or `BitDepth::Eight`.

For a permitted conversion it SHALL take the source `f32` samples from
`doc.source_planes.samples` when that store is `F32`, else from a fresh
`composite_native(doc)`, and SHALL apply
`pictura_adjust::hdr_toning::exposure_gamma` to the first
`color_channels * width * height` samples (the color planes) only; any trailing
alpha/extra samples SHALL be copied untoned. It SHALL quantize every sample to
the output depth: `clamp(v, 0, 1) * 65535` rounded at 16 bits and
`Samples::narrow_to_u8` at 8 bits.

At `BitDepth::Sixteen` it SHALL set `doc.depth = Sixteen`,
`doc.source_depth = Some(Sixteen)`, replace `doc.source_planes` with a
`SourcePlanes { depth: Sixteen, .. }` holding the converted samples, and rebuild
`doc.composite` from the color planes of that store. At `BitDepth::Eight` it
SHALL set `doc.depth = Eight`, `doc.source_depth = None`, and
`doc.source_planes = None`, and rebuild `doc.composite` from the 8-bit color
planes. In both cases it SHALL clear every layer's `source_channels` so a 32-bit
layer store cannot be re-emitted into the converted document.

#### Scenario: A clean 32-bit open converts to 16 and keeps values above white

- **WHEN** a 32-bit Grayscale/RGB document whose retained store holds a sample
  above `1.0` is converted to 16 bits with `exposure_ev` low enough that the
  toned value still exceeds `1.0`
- **THEN** `doc.depth`/`doc.source_depth` are `Sixteen`, `retains_source_depth()`
  holds, and the resulting 16-bit sample is the tone-mapped value clamped to
  `65535`, not the value a pre-clamp at `1.0` would produce

#### Scenario: Extra planes are not tone-mapped

- **WHEN** the source store carries an extra plane beyond the color planes
- **THEN** its samples are quantized from the untouched source value, not through
  `exposure_gamma`

#### Scenario: Converting to 8 clears the retained 32-bit store

- **WHEN** a 32-bit document is converted to 8 bits
- **THEN** `doc.depth == Eight`, `doc.source_depth == None`, and
  `doc.source_planes == None`

#### Scenario: A refused conversion leaves the document unchanged

- **WHEN** `out` is neither 16 nor 8, or the document is not a native 32-bit
  Grayscale/RGB document (e.g. `source_depth` is `None`, or `source_mode` is set)
- **THEN** the call returns an `OpsError` and `doc` is unchanged

### Requirement: HDR Conversion dialog and Image Mode bit-depth commands

The application SHALL route `Image > Mode > 16 Bits/Channel` and
`Image > Mode > 8 Bits/Channel` on a 32-bit document through an HDR Conversion
dialog and, on accept, invoke the conversion bridge with the dialog's Exposure
(EV) and Gamma values. The dialog SHALL expose only those two fields,
defaulting to Exposure `0` and Gamma `1.0` (no Method control, since only
Exposure & Gamma is implemented). On an 8- or 16-bit document the same
commands SHALL convert without the dialog, as `ui/image-mode-menu` specifies.

The `PictureView` bridge SHALL expose `convert_depth(bits, exposure_ev, gamma)`;
it SHALL call the engine conversion for `bits` 16 or 8, then recomposite and
record one history state labelled `HDR Conversion` when it returns true, and
SHALL return false without mutating the document when it does not.

#### Scenario: The bit-depth commands require a 32-bit document

- **WHEN** the active document is 8- or 16-bit
- **THEN** the HDR Conversion dialog is not opened: choosing another depth
  converts directly and records one `Convert Mode` state

#### Scenario: The dialog converts a 32-bit document

- **WHEN** the dialog is accepted on a 32-bit document after the bridge reports
  success
- **THEN** `document_depth_bits()` reflects the requested depth and one
  `HDR Conversion` history state exists

#### Scenario: A failed conversion does not record history

- **WHEN** the bridge returns false (no document, or a refused depth)
- **THEN** no history state is recorded and the document is unchanged
