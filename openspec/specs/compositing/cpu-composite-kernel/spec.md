# compositing/cpu-composite-kernel Specification

## Purpose
Defines how the CPU composite inner loop resolves per-layer state, skips fully
transparent source pixels, and keeps one blend formula while producing
byte-identical composites and saved files.

## Requirements

### Requirement: Per-layer state is resolved once per layer pass

The CPU compositor SHALL resolve each layer's composite state once for its pass
over a composite call and SHALL reuse that resolved state for every band of a
banded composite and for every region of the call. The resolved state MUST cover
the layer's opacity and fill, the text-gamma curve, whether Blend If applies, the
mask gate kind (none, 8-bit raster, native raster, vector, or feathered), the
resolved feathered plane or vector coverage sampler, and the layer's
authoritative native sample planes.

Per-pixel evaluation MUST NOT call the document's authoritative-plane lookup,
MUST NOT look up a feathered plane or vector coverage table in a cache, and MUST
NOT re-derive the layer's opacity or fill, the text-gamma curve, or the Blend If
default check. A band MUST NOT re-resolve the mask gates or native planes for a
layer whose state the composite already resolved.

#### Scenario: A high-depth layer keeps its native samples [ck_native]

- **WHEN** a 16-bit or 32-bit document with a retained native plane for a layer
  is composited through the resolved per-layer state
- **THEN** the composite is byte-identical to the composite that resolves the
  native plane per pixel, including the per-channel fallback to the 8-bit plane
  for a channel the native store does not carry

#### Scenario: A refined mask keeps its samples [ck_mask]

- **WHEN** a layer carries a feathered raster mask, a feathered vector mask, an
  unfeathered vector mask, or a high-depth native mask combined with a vector
  mask
- **THEN** every mask gate value is identical to the per-pixel evaluation,
  including the 8-bit `(raster × vector + 127) / 255` combine and the native
  `f32` product

#### Scenario: Per-layer constants keep their values [ck_const]

- **WHEN** a layer has a non-default opacity, a fill below 255, an active Blend
  If range, and a type-tool curve
- **THEN** the composite is byte-identical to the per-pixel evaluation

#### Scenario: A banded composite reuses the resolved state [ck_banded_reuse]

- **WHEN** a stack taller than two bands is composited in bands
- **THEN** each layer's mask gates and native planes are resolved once for the
  composite, every band samples them without a cache lookup, and the output is
  byte-identical to the whole-canvas composite

### Requirement: Fully transparent source pixels are skipped before mask and blend work

For a source sample whose alpha is zero after opacity and fill are applied, the
compositor SHALL NOT sample the layer mask, evaluate Blend If, apply the text
curve, or run the blend arithmetic. A raw source alpha of zero SHALL also skip
the colour-plane reads; a nonzero raw alpha with zero opacity or fill MAY still
read them. It MUST NOT mark the pixel as covered for a knockout composite,
matching the pre-change early return.

#### Scenario: A sparse layer composites identically [ck_skip]

- **WHEN** a layer whose rect covers the canvas but whose source alpha is zero
  over most pixels is composited
- **THEN** the output is byte-identical to the pre-change composite, and the
  test-only mask-sample counter records no sample for the zero-alpha pixels

#### Scenario: A zero-alpha knockout pixel is not covered [ck_skip_cover]

- **WHEN** a knockout layer has zero source alpha at a pixel
- **THEN** the pixel's coverage flag stays false and the backdrop is unchanged

### Requirement: One blend formula serves every content route

The compositor SHALL evaluate the layer blend through one shared formula,
implemented by the shared per-pixel kernel, used by pixel layers, isolated
groups, adjustment layers, fills, smart sources, smart filters, and type
sources. The shared per-pixel kernel MUST remain the reference. A row-span
kernel MAY be derived from the same formula and serve a selected pixel layer
when the selection requirement above chooses it, provided the derived kernel
evaluates the same expression in the same operation order and its composite is
byte-identical to the shared kernel on final `PixelBuffer` bytes. The per-pixel
entry point MUST remain available to the test-only legacy evaluation, which MUST
select the pre-change per-pixel body.

#### Scenario: Every blend mode matches the legacy evaluation [ck_legacy]

- **WHEN** random source and backdrop samples for every layer blend mode are
  evaluated through the shared kernel and through the legacy per-pixel switch
- **THEN** every output channel is bit-for-bit equal in `Px` `f32` bits and in
  the final `u8`

#### Scenario: Every content route keeps its bytes [ck_callers]

- **WHEN** the pixel, group, adjustment, fill, smart-source, smart-filter, and
  type-source routes are composited after the migration
- **THEN** each route produces the same bytes it produced before the migration

### Requirement: Composite and saved output remain byte-identical

The compositor SHALL return the same bytes for the same document after this
change. Banded and whole-canvas composites, region and full composites, and
parallel and serial execution MUST remain byte-identical to each other.

#### Scenario: The oracle suites are unchanged [ck_oracles]

- **WHEN** the document, clipping, knockout, adjustment, merge, native-depth,
  and reference-fixture oracle suites run
- **THEN** every output buffer and encoded file is byte-identical to its
  recorded result

#### Scenario: Banded and region composites still match [ck_region]

- **WHEN** a document is composited in bands or over a sub-region and compared
  with the whole-canvas composite
- **THEN** the corresponding samples are byte-identical

### Requirement: A row-span kernel serves the common layer shape

The CPU compositor SHALL evaluate a plain, 8-bit pixel layer, including a
smart-object or type layer composited from its raster proxy, through a row-span
kernel when the layer's effective blend state has Normal blend mode, no Blend
If, no text-gamma curve, no authoritative native source sample, and no knockout
coverage; it SHALL use the general per-pixel kernel for every other layer. The
selection MUST be derived from the effective blend state after any synthetic
override (clipping, knockout, adjustment gating), not from the layer's
unmodified attributes. A knockout composite that supplies a cover canvas SHALL
take the general kernel.

#### Scenario: A common layer uses the row-span kernel [rk_pick]

- **WHEN** a plain 8-bit layer with Normal mode, no mask, no Blend If, no text
  curve, and no native samples is composited
- **THEN** the row-span kernel evaluates its pixels
- **AND** the output is byte-identical to the forced-general composite

#### Scenario: An advanced layer keeps the general kernel [rk_general]

- **WHEN** a layer has an active Blend If, a text-gamma curve, a native source
  plane, a non-Normal blend mode, or a knockout coverage canvas
- **THEN** the general per-pixel kernel evaluates its pixels and the output is
  unchanged

#### Scenario: Selection follows a synthetic override [rk_override]

- **WHEN** a clipping base or a knockout layer is composited so a synthetic
  override changes the effective mode, fill, or mask presence, or supplies a
  cover canvas
- **THEN** the selected kernel is derived from the effective state, the covered
  knockout takes the general kernel, and the output is byte-identical to the
  forced-general composite

### Requirement: Mask coverage is resolved per row

When a selected kernel serves a masked layer, the compositor SHALL resolve the
layer mask's coverage for each row into an `f32` slice for the pixels whose
source alpha after opacity and fill is greater than zero, and the kernel SHALL
consume that slice. A pixel whose post-opacity/fill content is zero MUST keep
`0.0` in the slice and MUST NOT cause a mask sample, preserving the existing
zero-alpha skip. For every sampled pixel the resolved value MUST equal the
per-pixel mask gate exactly for every mask kind: plain raster with density,
feathered raster, vector coverage with invert and density, the native `f32`
mask combined with the vector mask, and the 8-bit `(raster × vector + 127) /
255` combine. The kernel MUST NOT quantise a native mask to eight bits.

#### Scenario: Every mask kind matches the general path [rk_mask]

- **WHEN** layers carrying a plain raster mask, a feathered raster mask, a
  vector mask, a native mask combined with a vector mask, and masked layers at
  zero opacity and zero fill are composited through the row-span kernel and
  through the forced-general path
- **THEN** both outputs are byte-identical for each case, and the zero-content
  cases record no mask sample

### Requirement: The opaque kernel removes the alpha division

When the selected opaque kernel serves a layer that has no alpha plane, Normal
blend mode, no authoritative native source sample, no knockout coverage, and an
effective state that passes the source alpha through unchanged (opacity 255,
fill 255, no curve, no Blend If, no mask), the compositor SHALL evaluate the
closed form with a constant source alpha of one, which removes the output-alpha
division and the per-pixel alpha gate. The kernel MUST preserve the general
formula's operation order for the remaining terms.

#### Scenario: An opaque layer matches over any backdrop [rk_opaque]

- **WHEN** an alpha-less full-canvas layer with Normal mode and default opacity
  and fill is composited over a transparent backdrop and over a partially
  transparent backdrop
- **THEN** its output is byte-identical to the forced-general composite in both
  cases

### Requirement: Eight-bit samples convert through a unit table

The compositor SHALL convert an 8-bit channel-plane or mask-gate sample to its
unit `f32` value through a 256-entry table whose entries are the same values the
`sample as f32 / 255.0` division produces. The general per-pixel path and every
row-span kernel MUST use the table for those samples, and the conversion MUST
return the same `f32` bits as the division. A native `SampleRow` sample is out
of scope: the row-span selection excludes authoritative native stores.

#### Scenario: A unit conversion keeps its bits [rk_unit]

- **WHEN** each of the 256 possible 8-bit samples is converted through the table
  and through the division for a channel-plane sample and for a mask-gate sample
- **THEN** the two `f32` values have equal bit patterns

### Requirement: Every selected kernel matches the general path

A test-only switch SHALL force the general per-pixel evaluation. For every
kernel the selection can pick, the composite produced through the selected
kernel MUST be byte-identical to the composite produced with the switch on.
That parity MUST hold for whole-canvas and banded composites, for a sub-region
and its matching full-canvas pixels, and for serial and parallel row execution.

#### Scenario: Forced-general parity [rk_parity]

- **WHEN** a document whose layers cover the kernel matrix (alpha row, no alpha
  row, each mask kind, grayscale, non-default opacity and fill, masked and
  unmasked, a smart-object raster proxy, and a type raster proxy) is composited
  with the switch off and on
- **THEN** the two output buffers are byte-identical

#### Scenario: Banded, region, and serial parity [rk_parity_paths]

- **WHEN** the same kernel-eligible stack is composited in bands, over a
  sub-region, and with the serial row switch on
- **THEN** each output is byte-identical to the whole-canvas parallel composite
