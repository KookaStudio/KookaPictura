# adjustment-layers Specification

## Purpose
Carries adjustment layers opaquely in the document model and applies the supported subset to the backdrop below.

## Requirements

### Requirement: Adjustment data is carried opaquely in the document model

The document model SHALL retain an adjustment's original four-byte key and
payload without normalization or re-encoding. The retained data SHALL be
accessible independently of the active typed content, through read-only
preservation access. An ordinary pixel layer SHALL have no active adjustment.
The layer's authoritative content SHALL represent either a supported typed
adjustment or an opaque undecodable adjustment, never an independently writable
optional adjustment competing with other content kinds. Core SHALL own the
plain content data and SHALL NOT depend on adjustment execution or PSD decoding.

#### Scenario: AdjustmentData preserves the key and payload verbatim

- **WHEN** an adjustment is imported with a four-byte key and a payload
- **THEN** preservation access returns the exact key and every original payload byte without re-encoding

#### Scenario: Core stays free of an adjustment dependency

- **WHEN** the core crate manifest is inspected
- **THEN** it declares no dependency on adjustment execution or PSD decoding

#### Scenario: A pixel layer has no adjustment

- **WHEN** an ordinary layer without an adjustment is loaded or constructed
- **THEN** its active content is Pixel and its typed adjustment accessor returns no adjustment

### Requirement: Codec preserves adjustment blocks through PSD read/write

`pictura-codec` SHALL recognise the adjustment additional-layer-info keys
(`levl`, `curv`, `brit`, `expA`, `vibA`, `hue2`, `hue `, `blwh`, `phfl`, `mixr`,
`grdm`, `invr`, `nvrt`, `post`, `thrs`, `selc`, `clrL`) and SHALL store each
block's key and payload bytes verbatim in `AdjustmentData`. On write it MUST emit
the same key and payload bytes it read, so a document with adjustment layers
round-trips byte-for-byte and unknown payload fields are not lost.

#### Scenario: Reader captures an adjustment key and payload

- **WHEN** a PSD layer record contains an additional-layer-info block whose key
  is one of the recognised adjustment keys
- **THEN** the resulting layer retains that key and the block's exact payload bytes
  separately from its normalized active content

#### Scenario: Read/write round-trip preserves key and bytes

- **WHEN** a document containing adjustment layers is written with `write_psd`
  and read back with `read_psd`
- **THEN** every adjustment layer's key and payload bytes are identical to the
  originals, including bytes the decoder does not understand

### Requirement: Render decodes the supported adjustment subset

`pictura-render` SHALL decode `AdjustmentData` into a destructive adjustment for
a practical subset: Invert (`nvrt` and the legacy `invr`), Posterize (`post`),
Threshold (`thrs`), Brightness/Contrast (`brit`), and Hue/Saturation (`hue2` and
the legacy `hue `). It MAY additionally decode Levels (`levl`). A key outside the
supported set, or a payload that fails its range checks, MUST decode to `None`
and MUST NOT produce an error.

#### Scenario: Supported keys decode

- **WHEN** a supported key with a valid payload is passed to `decode_adjustment`
- **THEN** it returns the matching adjustment variant with the decoded parameters

#### Scenario: Unknown or invalid payload decodes to no adjustment

- **WHEN** a key outside the supported set, or an out-of-range payload for a known
  key, is passed to `decode_adjustment`
- **THEN** it returns `None` without panicking or returning an error

### Requirement: An adjustment layer applies to the backdrop below it

Applying an adjustment layer SHALL apply the decoded adjustment to the running
backdrop (the accumulated composite of the layers below it) and then gate the
result by the layer's mask, opacity, and blend mode. For a decoded Invert layer
over a pixel layer, the result MUST match applying the same adjustment to the
flattened composite within a per-channel tolerance of ±1. Pixels where the
backdrop is transparent MUST be left unchanged.

#### Scenario: Invert adjustment layer matches the flattened composite

- **WHEN** an Invert adjustment layer is composited over a pixel layer
- **THEN** each output channel is within ±1 of applying Invert to the flattened
  composite of the same pixel layer

#### Scenario: Transparent backdrop gains no content

- **WHEN** the backdrop below the adjustment is transparent at a pixel
- **THEN** the adjustment contributes nothing at that pixel

### Requirement: Mask, opacity, and blend gate the adjustment effect

The adjustment layer's mask, opacity, and blend mode SHALL gate the adjustment
output before it is composited, so that masked-out pixels keep the original
backdrop and reduced opacity scales the effect toward the unadjusted value.

#### Scenario: Mask hides the effect

- **WHEN** a mask value is zero at a pixel
- **THEN** that pixel keeps the unadjusted backdrop

#### Scenario: Opacity scales the effect

- **WHEN** an adjustment layer has opacity 128
- **THEN** the result is approximately halfway between the unadjusted and fully
  adjusted backdrop

### Requirement: Unknown adjustments are preserved and not applied

The compositor SHALL treat an adjustment whose key is unknown to the renderer, or
whose payload cannot be decoded, as a no-op, leaving the composite unchanged
while the raw bytes remain in the document for saving. Neither reading nor
compositing an unsupported adjustment may error or drop its bytes.

#### Scenario: Undecodable key leaves the composite unchanged

- **WHEN** a document with an undecodable adjustment key is composited
- **THEN** the output is identical to compositing the same document without that
  adjustment layer

#### Scenario: Unsupported adjustment survives a save

- **WHEN** a document containing an undecodable adjustment is written to PSD
- **THEN** that adjustment's key and payload bytes are written back unchanged

### Requirement: Additional adjustment-layer kinds

The New Adjustment Layer menu, the Layers panel New Fill / Adjustment menu, and
the adjustment-layer creation path SHALL support all sixteen CS6 adjustment
kinds: Brightness/Contrast, Levels, Curves, Exposure, Vibrance, Hue/Saturation,
Color Balance, Black & White, Photo Filter, Channel Mixer, Color Lookup, Invert,
Posterize, Threshold, Gradient Map, and Selective Color. Each creates a
non-destructive adjustment layer whose block carries that kind's declared
new-layer default and whose display name matches the menu entry. The Layers panel
creation menu SHALL additionally offer the Solid Color…, Gradient…, and Pattern…
fill layers. An unrecognised kind SHALL still be refused without adding a layer.

A new-layer default MAY differ from the kind's dialog default: the dialog opens
on neutral parameters, while a new layer may carry a visible change so that
adding the layer has an effect. Where they differ the catalogue SHALL declare
both, and the new layer SHALL use the new-layer default.

A Color Lookup layer's block SHALL name one of the built-in Color Lookup presets,
and a newly created Color Lookup layer SHALL name `None`, so the stored name
resolves to the `None` preset index, the same preset the dialog opens on.

#### Scenario: Every supported kind creates a named layer

- **WHEN** each of the sixteen supported adjustment kinds is requested
- **THEN** a layer carrying that kind's adjustment block is added with the matching display name

#### Scenario: Unknown kind is refused

- **WHEN** an unrecognised adjustment kind is requested
- **THEN** no layer is added and the request fails

#### Scenario: The new kinds open on neutral defaults

- **WHEN** a Levels, Curves, Exposure, Vibrance, or Black & White layer is created
- **THEN** its parameters are that kind's CS6 dialog defaults, so a Curves layer opens as the identity

#### Scenario: The panel creation menu offers every kind

- **WHEN** the Layers panel New Fill / Adjustment menu is opened
- **THEN** it lists the Solid Color…, Gradient…, and Pattern… fill entries and
  all sixteen adjustment kinds, for nineteen entries total, and choosing an
  adjustment kind adds that layer

#### Scenario: A new Color Lookup layer names its preset [adjust_layer_color_lookup_preset]

- **WHEN** a Color Lookup adjustment layer is created and its stored block is
  passed to `color_lookup_preset_index`
- **THEN** the index is the `None` preset's index, so the stored name resolves to
  a built-in preset instead of a name no preset matches
