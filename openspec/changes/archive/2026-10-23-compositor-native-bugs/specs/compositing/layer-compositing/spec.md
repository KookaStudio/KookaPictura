# Spec Delta

## MODIFIED Requirements

### Requirement: Pass-through group semantics

A `PassThrough` group with opacity 255 and neither a raster nor a vector mask
SHALL composite its children directly onto the running backdrop so that child
blend modes observe content outside the group. A `PassThrough` group carrying
any mask SHALL be composited as an isolated group so the mask can gate the
group's result.

#### Scenario: Pass-through equals the flattened stack

- **WHEN** the same layers are composited with and without a `PassThrough` group wrapper
- **THEN** the two output buffers MUST be identical

#### Scenario: Pass-through with opacity falls back to isolated

- **WHEN** a `PassThrough` group has opacity other than 255, or a mask
- **THEN** it MUST be composited as an isolated group with the group blend mode treated as `Normal`

#### Scenario: A Pass Through group with a vector mask is isolated [lc_pass_through_vector_mask]

- **WHEN** a `PassThrough` group with opacity 255 carries an enabled vector mask
- **THEN** it MUST be composited as an isolated group with the group blend mode treated as `Normal`, and the group result MUST be multiplied by the vector-mask coverage on both the CPU and the GPU

### Requirement: Layer fill opacity folds into effective source alpha

Each raster or fill-content layer's `fill` (0..=255) SHALL multiply the effective
source alpha, in addition to its `opacity`, in both the CPU oracle and the GPU
compositor: a sample's effective alpha SHALL be `opacity/255 × fill/255` (with
the raster mask and clipping folded in as before). An adjustment layer's stored
`fill` SHALL NOT affect compositing: an adjustment transforms the backdrop at
its opacity alone, because CS6 exposes no Fill control for adjustments. When
`fill == 255` the composite MUST be byte-identical to the layer's contribution
without a fill factor, so existing documents are unchanged. When `fill != 255`
the GPU result MUST remain within the existing ±1 LSB CPU parity. A group's
`fill` SHALL have no effect on compositing (CS6 exposes no group Fill), but the
stored value SHALL round-trip through PSD.

#### Scenario: Default fill is byte-identical

- **WHEN** a document whose layers all have fill 255 is composited
- **THEN** the result is byte-identical to compositing the same document without any fill factor

#### Scenario: Fill scales the layer contribution

- **WHEN** a full-alpha source layer with fill 128 is composited over an opaque backdrop
- **THEN** the result is the source-over mix with effective source alpha `opacity/255 × 128/255`

#### Scenario: Fill combines with opacity and mask

- **WHEN** a masked layer with opacity 200 and fill 128 is composited
- **THEN** its contribution is scaled by `200/255 × 128/255` and the mask sample, in the same order as the CPU oracle

#### Scenario: GPU matches the CPU oracle with fill

- **WHEN** a scene with a non-default fill is composited on the GPU and on the CPU
- **THEN** the two results agree within ±1 LSB per channel

#### Scenario: Group fill is inert

- **WHEN** a group carries a non-default fill value
- **THEN** the group composites as if its fill were 255, while the stored value survives a PSD round-trip

#### Scenario: An adjustment layer ignores Fill [lc_adjustment_ignores_fill]

- **WHEN** an Invert adjustment layer with a non-default stored fill is composited at opacity 255, 128, and 0, at fill 0, 50, and 100 percent
- **THEN** the backdrop is inverted at the layer's opacity alone, with no fill factor, and opacity 0 leaves the backdrop unchanged
