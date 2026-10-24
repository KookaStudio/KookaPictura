# hdr-conversion Specification

## Purpose
32-bit to 16/8-bit HDR Conversion with the four CS6 tone-mapping methods and the HDR Conversion dialog.

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
dialog and, on accept, invoke the conversion bridge with the dialog's method
and parameters. The dialog SHALL expose a Method control with CS6's four
methods (`Local Adaptation`, `Equalize Histogram`, `Exposure and Gamma`,
`Highlight Compression`) and SHALL default to Local Adaptation with the HDR
Toning dialog's `Default` preset values. It SHALL show the Exposure (EV) and
Gamma fields only for Exposure and Gamma, show no parameter fields for the two
automatic methods, and show the nine Local Adaptation numeric controls (Edge Glow
Radius, Strength, Tone and Detail Gamma, Exposure, Detail, Shadow, Highlight,
Vibrance, Saturation) for Local Adaptation. The Toning Curve/Corner control is
a recorded ceiling in this change. On an 8- or 16-bit document the same commands
SHALL convert without the dialog, as `ui/image-mode-menu` specifies.

The application SHALL call the conversion engine through a dedicated
free-function bridge that carries the requested bits, the method, and its
parameters. The existing `convert_depth(bits, exposure_ev, gamma)` invokable
SHALL remain an Exposure & Gamma convenience wrapper for existing callers. The
bridge SHALL recomposite and record one history state labelled `HDR Conversion`
when the conversion returns true, and SHALL return false without mutating the
document when it does not.

#### Scenario: The bit-depth commands require a 32-bit document

- **WHEN** the active document is 8- or 16-bit
- **THEN** the HDR Conversion dialog is not opened: choosing another depth
  converts directly and records one `Convert Mode` state

#### Scenario: The dialog exposes the four methods and gates their fields

- **WHEN** the HDR Conversion dialog opens on a 32-bit document, and the Method
  control changes from Local Adaptation to Equalize Histogram, Exposure and
  Gamma, and Highlight Compression in turn
- **THEN** Local Adaptation shows its nine numeric controls, Exposure and Gamma
  shows only Exposure and Gamma, and the two automatic methods show no
  parameter fields

#### Scenario: The dialog defaults to Local Adaptation

- **WHEN** the HDR Conversion dialog opens on a 32-bit document
- **THEN** the Method is Local Adaptation and its nine controls match the HDR
  Toning dialog's `Default` preset values

#### Scenario: The dialog converts a 32-bit document

- **WHEN** the dialog is accepted on a 32-bit document after the bridge reports
  success with the selected method and its parameter values
- **THEN** `document_depth_bits()` reflects the requested depth and one
  `HDR Conversion` history state exists

#### Scenario: A failed conversion does not record history

- **WHEN** the bridge returns false (no document, a refused depth, or an
  out-of-range parameter)
- **THEN** no history state is recorded and the document is unchanged

### Requirement: 32-bit to 16/8 HDR Conversion tone-map methods

`pictura_render` SHALL expose
`convert_depth(doc: &mut Document, out: BitDepth, method: HdrToneMap) ->
Result<(), OpsError>` with the same document gate, quantization,
retained-store rebuild, and layer-store clearing as
`convert_depth_exposure_gamma`. It SHALL apply the selected method to the first
`color_channels * width * height` f32 samples and copy any trailing alpha/extra
samples untoned.

The source samples SHALL be the retained `f32` store only while that store is
current. When the document has been edited since the store was captured, the
call SHALL use the current composite, so a conversion never tone-maps
pre-edit pixels.

`HdrToneMap` SHALL offer exactly four methods:

- **Exposure and Gamma** — `sign(x*gain)*|x*gain|^(1/gamma)` with
  `gain = 2^exposure_ev`; values above `1.0` stay unclamped until output
  quantization.
- **Equalize Histogram** — derive a luminance cumulative distribution from the
  finite positive color samples and apply its gain to each colour plane; no
  parameters. Input order SHALL be preserved; a fully uniform input SHALL
  remain finite and unchanged.
- **Highlight Compression** — leave luminance at or below `0.8` unchanged and
  roll values above the knee asymptotically toward `1.0`, so a highlight above
  white maps to white without hard-clipping the midtones; no parameters.
- **Local Adaptation** — accept edge-glow radius, strength, gamma, exposure,
  detail, shadow, highlight, vibrance, and saturation with the same ranges as
  `pictura_filters::hdr_toning::HdrToningParams`. It SHALL tone-map the f32
  luminance without a pre-clamp to `1.0` and preserve the alpha plane. The
  documented Toning Curve and Corner option are **not** part of this change;
  they are a recorded ceiling, and the nine numeric controls are the supported
  Local Adaptation surface.

The call SHALL return an error without mutating `doc` for an out-of-range
parameter, a non-finite value, or a method the document gate refuses. On
success, a layered document SHALL be flattened to one Background built from the
tone-mapped composite, because the application recomposites after the conversion
and a preserved layer tree would restore the pre-conversion pixels.

#### Scenario: Each method converts a 32-bit document

- **WHEN** a native 32-bit RGB document is converted to 8 or 16 bits with any
  one of the four methods
- **THEN** the call succeeds, the depth/store fields follow the conversion
  contract, and the output pixels reflect that method rather than the identity

#### Scenario: A converted layered document is flattened

- **WHEN** a layered 32-bit document is converted and the application
  recomposites
- **THEN** the document has one Background layer carrying the tone-mapped pixels

#### Scenario: Highlight Compression keeps midtones and maps white

- **WHEN** Highlight Compression runs on samples `0.5` and `4.0`
- **THEN** `0.5` is unchanged and `4.0` maps to a finite value below `1.0` and
  above `0.9`, with no hard clamp of the `0.5` sample

#### Scenario: Equalize Histogram keeps a uniform input finite

- **WHEN** Equalize Histogram runs on a uniform colour plane
- **THEN** every output sample is finite and equals the input

#### Scenario: Local Adaptation leaves alpha untouched

- **WHEN** Local Adaptation runs on a source store that carries an alpha plane
- **THEN** the alpha samples are copied unchanged while the colour planes change

#### Scenario: Extra planes are copied for every method

- **WHEN** any conversion method runs on a store with an alpha or extra plane
- **THEN** that plane is copied untoned

#### Scenario: An edited document converts its current composite

- **WHEN** a 32-bit document is edited and then converted without an intervening
  native-store refresh
- **THEN** the conversion uses the edited composite, not the stale store, and
  the output reflects the edit

#### Scenario: Bad method parameters are refused

- **WHEN** a Local Adaptation parameter lies outside its documented range, or a
  parameter is not finite
- **THEN** `convert_depth` returns an `OpsError` and `doc` is unchanged
