# Spec Delta

## ADDED Requirements

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
parameter, a non-finite value, or a method the document gate refuses.

#### Scenario: Each method converts a 32-bit document

- **WHEN** a native 32-bit RGB document is converted to 8 or 16 bits with any
  one of the four methods
- **THEN** the call succeeds, the depth/store fields follow the conversion
  contract, and the output pixels reflect that method rather than the identity

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

## MODIFIED Requirements

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
