# Spec Delta

## MODIFIED Requirements

### Requirement: A raster proxy takes precedence over the embedded payload

A layer that has a color channel (`id` 0) SHALL render from its stored channel
data, and the renderer SHALL NOT decode the smart-object payload for that layer,
unless the object carries an active stack mode for which the reduced stack
source applies (an embedded payload with at least two visible frames and every
enabled Smart Filter decodes). An object with such an active stack mode SHALL
render from the reduced stack source described by
`compositing/smart-object-stack-modes`, even when a colour channel exists.
Existing files that carry a Photoshop raster proxy and no active stack mode
SHALL render unchanged.

#### Scenario: Proxy is used
- **WHEN** a smart-object layer also has a color channel and no active stack mode
- **THEN** the channel data is composited and the payload is not decoded

#### Scenario: No behaviour change for proxied files
- **WHEN** a document whose smart-object layers all carry color channels and no stack mode is composited
- **THEN** the result equals the result before this change

#### Scenario: An active stack mode overrides the proxy [sor_stack_over_proxy]
- **WHEN** a smart-object layer carries both a colour channel and an active stack mode for which the reduced stack source applies
- **THEN** the reduced stack source is composited and the channel data is not

### Requirement: Decoded source is placed by the layer rect with nearest-neighbour scaling

The renderer SHALL scale the decoded source into the layer's rect using
nearest-neighbour sampling, clamped to the canvas, and SHALL NOT apply the
`Trnf`/warp transform or bilinear resampling. The decoded document's stored
merged composite SHALL be preferred when it is usable; otherwise the renderer
SHALL composite the decoded document's layers to obtain the source. When an
eligible active stack mode applies, the renderer SHALL use the reduced stack
source described by `compositing/smart-object-stack-modes` instead of the
decoded document's stored merged composite or its composited layers.

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

#### Scenario: An eligible stack mode replaces the stored composite
- **WHEN** a smart object carries an eligible active stack mode and its decoded document carries a usable stored merged composite
- **THEN** the reduced stack source is used instead of the stored merged composite
