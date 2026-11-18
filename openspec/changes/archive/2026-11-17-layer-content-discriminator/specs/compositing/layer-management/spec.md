# Spec Delta

## MODIFIED Requirements

### Requirement: Rasterize subset

The system SHALL provide a `Rasterize` submenu. `Fill Content` SHALL be
implemented: for a fill-content layer (a layer whose active Fill content retains
a fill-content key — `SoCo`, `GdFl`, or `PtFl` — and whose payload decodes to
`FillContent::Solid`, `FillContent::Gradient`, or
`FillContent::Pattern`) it SHALL render the fill content to a full-layer
pixel node and clear the fill/adjustment data. A solid fill SHALL bake its
decoded colour across the layer rect; a gradient fill SHALL bake the generated
five-kind ramp with opaque alpha across the layer rect; a pattern fill SHALL
bake the pattern the document's `Patt` resource tiles over the layer rect (with
the pattern's own alpha, or the grey placeholder when the referenced pattern is
absent or was skipped as non-8-bit/malformed). `Shape` SHALL be implemented: a
shape layer (a solid fill cut to a `vmsk` vector mask) SHALL have its rendered
appearance — the fill cut to the outline, plus its stroke — baked into ordinary
pixels, and its shape/vector definition dropped (the solid-fill adjustment, the
`vmsk` vector mask, the live-shape `vogk` block, and the stroke effect), while
keeping its name, opacity, fill, blend mode, and layer mask. Its rect SHALL grow
when an effect (an outside or centered stroke) paints past it, so the baked
pixels cover the whole rendered appearance. `Rasterize
Layer` SHALL rasterize the active layer only when it is a fill-content layer,
and SHALL otherwise refuse without changing the document. `Rasterize All
Layers` SHALL rasterize every fill-content layer and every shape layer in the
document. The `Type`, `Vector Mask`, `Smart Object`, `Video`, and `3D` variants
SHALL remain visible and disabled, with a documented reason that those layer
kinds do not exist or are handled elsewhere. Later capabilities that implement
Type or Smart Object rasterization SHALL retain their existing enabled behavior;
this subset SHALL NOT disable them during the content-model migration. A rasterization whose content is
not decodable SHALL be disabled and SHALL refuse without changing the document,
and every applied rasterization SHALL recomposite and record exactly one undo
state.

#### Scenario: Fill Content becomes pixels

- **WHEN** Rasterize Fill Content runs on a solid fill-content layer
- **THEN** the layer becomes a pixel layer holding the rendered fill, its fill /
  adjustment data is cleared, and the change is one undo step

#### Scenario: A descriptor-form fill is rasterizable

- **WHEN** Rasterize Fill Content runs on a layer whose `SoCo` payload is the
  standard Photoshop descriptor
- **THEN** the layer becomes a pixel layer holding the descriptor's colour and
  the change is one undo step

#### Scenario: A gradient fill is rasterizable

- **WHEN** Rasterize Fill Content runs on a layer whose `GdFl` payload decodes to
  `FillContent::Gradient`
- **THEN** the layer becomes a pixel layer holding the generated gradient over
  its rect, its fill / adjustment data is cleared, and the change is one undo
  step

#### Scenario: A pattern fill is rasterizable

- **WHEN** Rasterize Fill Content runs on a layer whose `PtFl` payload decodes to
  `FillContent::Pattern`
- **THEN** the layer becomes a pixel layer holding the tiled pattern over its
  rect, its fill / adjustment data is cleared, and the change is one undo step

#### Scenario: A pattern fill with a missing pattern is still rasterizable

- **WHEN** Rasterize Fill Content runs on a `PtFl` layer whose `pattern_id` is
  not present in the document's `Patt` resource
- **THEN** the layer becomes a pixel layer holding the grey placeholder and
  the change is one undo step

#### Scenario: A shape layer rasterizes to pixels

- **WHEN** Rasterize Shape runs on a shape layer
- **THEN** the layer's fill cut to its outline (and its stroke) is baked into
  the layer's pixels, its fill/adjustment, vector mask, live-shape, and stroke
  data are dropped, its appearance is unchanged when recomposited, and the
  change is one undo step

#### Scenario: Rasterize Shape refuses a non-shape

- **WHEN** Rasterize Shape runs on a pixel, group, adjustment, type, or
  Background layer
- **THEN** the document is unchanged and no undo state is recorded

#### Scenario: A kind-less variant stays disabled

- **WHEN** the Rasterize submenu is shown
- **THEN** Vector Mask, Video, and 3D remain visible but disabled with their
  documented reason; Type and Smart Object follow their later implemented
  capabilities rather than being disabled by this subset

#### Scenario: Rasterize Layer refuses a non-fill layer

- **WHEN** Rasterize Layer runs on a group, adjustment, or plain pixel layer
- **THEN** the document is unchanged and no undo state is recorded
