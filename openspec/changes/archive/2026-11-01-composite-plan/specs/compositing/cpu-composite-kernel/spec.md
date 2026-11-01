# Spec Delta

## MODIFIED Requirements

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
