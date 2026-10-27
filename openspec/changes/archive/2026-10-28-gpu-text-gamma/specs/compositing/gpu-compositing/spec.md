# Spec Delta

## MODIFIED Requirements

### Requirement: GPU eligibility follows the CPU content route

The compositor SHALL classify each visible layer's content into exactly one
content route:

- isolated group;
- pass-through group;
- decoded fill or adjustment;
- undecodable adjustment (a CPU no-op);
- undecodable vector fill;
- smart-filtered smart-object source;
- channel-less smart-object source;
- channel-less type source;
- pixels.

The CPU compositor SHALL select its content path from this classification.
`composite_gpu` SHALL decide eligibility per content route. It SHALL also check
these layer attributes separately:

- enabled object-based effects;
- the `Dissolve` mode;
- non-default Blend If, knockout, and clipping.

For a document whose source depth is set, `composite_gpu` SHALL decline before
looking at any layer. Otherwise the first layer in stack order that declines
SHALL determine the reason, and within a layer the checks SHALL run in this
order: effects, content route, `Dissolve`, then advanced blending. Every decline
SHALL return a `GpuError` before any dispatch and MUST NOT panic. The
classification MUST NOT change the CPU output: `composite_rgba` SHALL return
exactly the bytes it returned before this change.

Blend Text Colors Using Gamma SHALL NOT be an eligibility attribute for a type
layer that carries a raster proxy: such a layer SHALL take the pixel route and
the blend shader SHALL apply the curve. A channel-less type source SHALL keep
declining with the existing text-unsupported error. The GPU decline counter
SHALL attribute a channel-less type source to `type_source`: the former
`unsupported_text` bucket is renamed to `type_source`; no separate `text_gamma`
bucket is added.

#### Scenario: Every declining content route has a reason

- **WHEN** the eligibility check runs on a one-layer document for each content
  route the GPU does not implement
- **THEN** it returns that route's `GpuError`, with no GPU device needed

#### Scenario: A proxied type layer stays eligible with the gamma on

- **WHEN** the eligibility check runs on a document whose only layer is a type
  layer carrying a raster proxy, with Blend Text Colors Using Gamma on
- **THEN** it accepts the document

#### Scenario: A channel-less type layer declines as a type source

- **WHEN** `composite_gpu` runs on a document whose only layer is a type layer
  with no raster proxy
- **THEN** it returns `GpuError::UnsupportedText` and counts the decline under
  `type_source`

#### Scenario: Effects win over the content route

- **WHEN** a fill layer carries an enabled drop shadow
- **THEN** the eligibility check returns `GpuError::UnsupportedLayerEffect`

#### Scenario: The CPU oracle is unchanged

- **WHEN** one document per content route is composited by `composite_rgba`
- **THEN** each result equals the bytes recorded before the change
