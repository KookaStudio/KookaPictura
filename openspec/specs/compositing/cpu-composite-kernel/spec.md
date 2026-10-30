# compositing/cpu-composite-kernel Specification

## Purpose
Defines how the CPU composite inner loop resolves per-layer state, skips fully
transparent source pixels, and keeps one blend formula while producing
byte-identical composites and saved files.

## Requirements

### Requirement: Per-layer state is resolved once per layer pass

The CPU compositor SHALL resolve each layer's composite state once for its pass
over a canvas, where a banded composite performs one pass per band. The resolved
state MUST cover the layer's opacity and fill, the text-gamma curve, whether
Blend If applies, the mask gate kind (none, 8-bit raster, native raster, vector,
or feathered), the resolved feathered plane or vector coverage sampler, and the
layer's authoritative native sample planes.

Per-pixel evaluation MUST NOT call the document's authoritative-plane lookup,
MUST NOT look up a feathered plane or vector coverage table in a cache, and MUST
NOT re-derive the layer's opacity or fill, the text-gamma curve, or the Blend If
default check.

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

The compositor SHALL evaluate the layer blend through one shared kernel used by
pixel layers, isolated groups, adjustment layers, fills, smart sources, smart
filters, and type sources. The per-pixel entry point MUST remain available to
the test-only legacy evaluation, which MUST select the pre-change per-pixel
body.

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
