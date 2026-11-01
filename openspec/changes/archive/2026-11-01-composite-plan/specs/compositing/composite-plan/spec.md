# Spec Delta

## Purpose

Resolves a layer stack's per-layer composite state once for a composite call and
reuses it for every band, row, and region, so banded compositing does not repeat
metadata work per band while producing identical pixels.

## ADDED Requirements

### Requirement: One plan serves every band and region of a composite

The compositor SHALL resolve each visible layer's composite state once per
composite call and SHALL reuse that resolved state for every band, row range, and
region of that call. Every entry point — whole-canvas, banded, sub-region,
native-depth, and render-session — SHALL perform one resolution per layer per
composite call, with the entry point's clip and effect region as inputs. A band
MUST NOT repeat route selection, adjustment decoding, effect decoding, pass
building, source building, mask resolution, native-plane lookup, or content-key
computation.

#### Scenario: A banded composite resolves each layer once [cp_once]

- **WHEN** a document taller than two bands with a pixel layer, an adjustment
  layer, a proxy-less type layer, a smart-filtered layer, and a styled layer is
  composited
- **THEN** every layer's route, adjustment, effects, passes, source, mask gates,
  and native planes are resolved once for the composite, and every band reuses
  them

#### Scenario: Banded and whole composites stay identical [cp_identity]

- **WHEN** the same stack is composited in bands and on one canvas
- **THEN** the two outputs are byte-identical

### Requirement: Content resolves at most once per layer per composite

For one layer in one composite, the compositor SHALL decode its adjustment or
lookup table at most once, decode its effect descriptor at most once, build or
fetch its effect passes at most once, and build or fetch each cached source
raster (type, smart source, or smart-filtered) at most once, regardless of the
number of bands or regions. A region-sized generated fill buffer (a gradient or
pattern fill over a band) MAY be built per band.

#### Scenario: An adjustment layer decodes once per composite [cp_adjust_once]

- **WHEN** a document with a Curves layer and a Color Lookup layer is composited
  in bands
- **THEN** each adjustment decodes exactly once for that composite, including the
  embedded `.CUBE` parse

#### Scenario: A source layer resolves once per composite [cp_source_once]

- **WHEN** a document with a proxy-less type layer, a smart-filtered layer, and a
  styled layer is composited in bands
- **THEN** each source raster and effect pass is fetched or built once for that
  composite

#### Scenario: A hidden layer resolves nothing [cp_hidden]

- **WHEN** a layer is not visible
- **THEN** the plan resolves no route, source, effect, or mask for it and the
  composite is unchanged

### Requirement: Content keys come from plane identity, not bytes

The compositor SHALL derive a layer's content key from plane stamps or idents and
stable model values. Computing a key MUST NOT read image-plane bytes and MUST NOT
materialize a fractured plane into one contiguous allocation. The smart-filter
`options` plane contributes its ident, and a smart-object payload without a
cached digest MAY be hashed once per composite.

#### Scenario: A fractured plane is not materialized [cp_no_materialize]

- **WHEN** a banded composite runs over a document whose channel plane has
  diverged bands and whose layer is styled but carries no adjustment
- **THEN** the plane-materialization counter does not increase across the band
  loop after the plan build

#### Scenario: An edited layer still rebuilds [cp_invalidate]

- **WHEN** a layer's channel, mask, adjustment, effect, payload, or type model
  changes and the document is composited again
- **THEN** the new content resolves and the output equals a purge-then-composite

### Requirement: Synthetic composite layers reuse resolved content

When the compositor composites a derived variant of a layer (a clipping base at
full opacity, its coverage pass, or an adjustment's gated blend), it SHALL reuse
that layer's resolved content — its source, effects, and passes — and SHALL
apply only the attribute overrides (opacity, fill, blend mode, mask presence,
Blend If presence). The group arm (pass-through versus isolated) MAY be
re-decided when an override changes blend mode, opacity, or mask presence, so a
variant reproduces the current synthetic clone's routing. It MUST NOT deep-copy
the layer model or re-resolve its content for the variant.

#### Scenario: A clipping group reuses its base's content [cp_clip_reuse]

- **WHEN** a clipping group's base is an adjustment or type layer and the
  document is composited in bands
- **THEN** the group's coverage and opaque composites reuse the base's resolved
  content and decode it once, the proxy composite reuses the group canvas, and
  the output stays byte-identical to the pre-change result

### Requirement: The plan changes no output

Every composite and saved file MUST stay byte-identical to the pre-change result.
Banded, whole, region, parallel, and serial execution MUST remain byte-identical
to each other.

#### Scenario: Existing oracles are unchanged [cp_oracles]

- **WHEN** the document, clipping, knockout, adjustment, merge, native-depth, and
  reference-fixture oracle suites run
- **THEN** every output buffer and encoded file is byte-identical to its recorded
  result
