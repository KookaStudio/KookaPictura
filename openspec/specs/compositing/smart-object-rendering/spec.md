# smart-object-rendering Specification

## Purpose
Renders embedded smart-object sources without a proxy, with proxy precedence, nearest-neighbour placement, and no-op fallback.

## Requirements

### Requirement: Embedded smart-object source renders without a proxy

The renderer SHALL decode, for a visible non-group non-adjustment layer whose
smart object is `Embedded` with a non-empty payload and no color channel (`id`
0), the payload as a document and composite the decoded pixels into the layer's
rect. The composite SHALL apply the layer's opacity, fill, mask, and blend mode
with the same semantics as a raster layer.

#### Scenario: Source renders in the layer rect
- **WHEN** a layer has an embedded payload and no color channel
- **THEN** the decoded source pixels composite inside the layer's rect instead of a black rectangle

#### Scenario: Layer properties apply
- **WHEN** such a layer has opacity, a mask, or a non-Normal blend mode
- **THEN** those properties modify the decoded source exactly as they would a raster layer

#### Scenario: Invisible layer is skipped
- **WHEN** such a layer is not visible
- **THEN** the payload is not decoded and the backdrop is unchanged

### Requirement: A raster proxy takes precedence over the embedded payload

A layer that has a color channel (`id` 0) SHALL render from its stored channel
data, and the renderer SHALL NOT decode the smart-object payload for that layer.
Existing files that carry a Photoshop raster proxy SHALL render unchanged.

#### Scenario: Proxy is used
- **WHEN** a smart-object layer also has a color channel
- **THEN** the channel data is composited and the payload is not decoded

#### Scenario: No behaviour change for proxied files
- **WHEN** a document whose smart-object layers all carry color channels is composited
- **THEN** the result equals the result before this change

### Requirement: An unusable source is a no-op

The renderer SHALL leave the backdrop unchanged, and SHALL NOT return an error or
panic, when a smart-object layer's source is `External`, `Alias`, or
`Unresolved`, when its payload is empty, or when the payload fails to decode.

#### Scenario: Non-embedded kind is a no-op
- **WHEN** a layer's smart object is external, alias, or unresolved
- **THEN** the composite leaves the backdrop unchanged and raises no error

#### Scenario: Empty payload is a no-op
- **WHEN** an embedded smart object has an empty payload
- **THEN** the composite leaves the backdrop unchanged and raises no error

#### Scenario: Undecodable payload is a no-op
- **WHEN** an embedded payload is not a decodable document
- **THEN** the composite leaves the backdrop unchanged and raises no error

### Requirement: Decoded source is placed by the layer rect with nearest-neighbour scaling

The renderer SHALL scale the decoded source into the layer's rect using
nearest-neighbour sampling, clamped to the canvas, and SHALL NOT apply the
`Trnf`/warp transform or bilinear resampling. The decoded document's stored
merged composite SHALL be preferred when it is usable; otherwise the renderer
SHALL composite the decoded document's layers to obtain the source.

#### Scenario: Scale into the rect
- **WHEN** the source size differs from the layer rect
- **THEN** each destination pixel samples the nearest source pixel

#### Scenario: Stored composite preferred
- **WHEN** the decoded document carries a usable stored merged composite
- **THEN** those pixels are used as the source

#### Scenario: Layers composited as fallback
- **WHEN** the decoded document has no usable stored merged composite
- **THEN** the decoded document's layers are composited to produce the source

#### Scenario: Transform is not applied
- **WHEN** an object carries a `Trnf`/warp transform
- **THEN** the source renders axis-aligned in the layer rect, without bilinear resampling

### Requirement: An authored embedded source composites its colour

The renderer SHALL composite a document authored in memory with a smart-object
layer whose embedded payload is a document of a known solid colour and whose
layer carries no color channel, yielding that colour across the layer's rect.

#### Scenario: Author and render
- **WHEN** a layer is authored with an embedded solid-colour source and no proxy
- **THEN** `composite_rgba` yields that colour inside the layer rect

#### Scenario: Authored source respects the rect
- **WHEN** the authored layer rect covers only part of the canvas
- **THEN** the colour appears only inside that rect and the rest of the canvas is unchanged

### Requirement: Smart-filter chain composites over the embedded source

When a smart-object layer carries a non-empty `filterFX` chain and every enabled filter decodes to a supported operation, the compositor SHALL render the layer from the smart object's embedded source and apply the enabled filters in order, using the group enable flag and each filter's `enab`, rather than compositing the already-baked raster proxy. An unknown enabled filter SHALL leave the layer to the normal proxy path so an imported result is not double-applied. The chain SHALL be deterministic and SHALL preserve alpha.

#### Scenario: A disabled filter is skipped

- **WHEN** a chain's only filter has `enab` false, or the group is disabled
- **THEN** the composite equals the unfiltered embedded source

#### Scenario: The chain is applied to the source, not the proxy

- **WHEN** a layer whose proxy already contains the filtered result carries the same enabled filter
- **THEN** the composite equals the filter applied once to the embedded source, not twice

#### Scenario: An unknown enabled filter falls back to the proxy

- **WHEN** a chain contains an enabled filter whose options cannot be decoded
- **THEN** the layer composites through the existing proxy path unchanged

### Requirement: Embedded nesting is bounded

An embedded payload is untrusted input: it SHALL NOT be able to make the
renderer recurse without bound. Every smart object in a decoded embedded payload
SHALL carry its nesting depth, one more than the object whose payload holds it,
with the objects of an opened document at depth 0. That depth SHALL travel with
the decoded data, so it holds when nested work runs on another worker thread.
An object at depth 8 or deeper SHALL NOT decode its payload, and SHALL render as
any unusable source does.

#### Scenario: Nesting up to the limit renders [sor_nesting_limit]

- **WHEN** a no-proxy embedded object holds payloads nested seven wrappers deep
  around a solid source
- **THEN** the composite shows the innermost source's colour

#### Scenario: Nesting past the limit stops decoding [sor_nesting_past_limit]

- **WHEN** the same source is nested eight wrappers deep
- **THEN** the object past the limit does not paint

#### Scenario: A deep payload renders on a small worker stack [sor_nesting_small_stack]

- **WHEN** a payload nested 200 deep is composited on a two-thread pool with
  512 KiB worker stacks
- **THEN** the composite finishes without overflowing a stack
