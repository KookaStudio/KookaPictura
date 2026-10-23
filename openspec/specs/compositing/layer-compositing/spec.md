# layer-compositing Specification

## Purpose
Planar source-over compositing of the bottom-to-top stack with opacity, masks, and isolated or pass-through group semantics.

## Requirements

### Requirement: Composited output format

The compositor SHALL return a `PixelBuffer` with four planar channels (R, G, B, A), 8-bit, at the document's width and height, in straight (unpremultiplied) alpha.

#### Scenario: Empty stack is fully transparent

- **WHEN** `composite_rgba` is called on a document with no layers
- **THEN** every sample of the returned buffer MUST be 0

#### Scenario: Output matches document resolution and channel count

- **WHEN** the document is `W × H`
- **THEN** the returned buffer MUST be `W × H` with exactly four channels

### Requirement: Bottom-to-top stack walk

The compositor SHALL composite visible layers in array order, treating index 0 as the bottom, each blended into the running backdrop, and MUST skip any layer whose `visible` flag is false.

#### Scenario: Order determines the result

- **WHEN** layers `[A, B, C]` are composited
- **THEN** each layer MUST be blended over the accumulated composite of every preceding layer

#### Scenario: Hidden layers are skipped

- **WHEN** a layer has `visible == false`
- **THEN** it MUST NOT change any output sample

### Requirement: W3C source-over compositing equation

The compositor SHALL combine a source sample `(Cs, αs)` with a backdrop `(Cb, αb)` using the W3C *Compositing and Blending Level 1* equation `Co = ((1 − αb)·αs·Cs + αs·αb·B(Cb, Cs) + (1 − αs)·αb·Cb) / αo` with `αo = αs + αb·(1 − αs)`, evaluated in normalized `f32` with premultiplied intermediates and converted to 8-bit exactly once at write-back.

#### Scenario: Blend mode is ignored over a transparent backdrop

- **WHEN** the backdrop alpha is 0
- **THEN** the result color MUST equal the source color `Cs` regardless of the blend mode

#### Scenario: Partial source alpha mixes with the backdrop

- **WHEN** a source sample with `0 < αs < 1` is composited in `Normal` mode over an opaque backdrop
- **THEN** the result MUST be the source-over mix of `Cs` and `Cb` weighted by `αs`

#### Scenario: Output alpha is Porter-Duff source-over

- **WHEN** source alpha `αs` is composited over backdrop alpha `αb`
- **THEN** the output alpha MUST be `αs + αb·(1 − αs)`

### Requirement: Layer opacity scales source alpha

Each layer's `opacity` (0..=255) SHALL scale the source alpha before compositing; `255` is fully opaque and `0` contributes nothing.

#### Scenario: Partial opacity blends proportionally

- **WHEN** a full-alpha source layer is composited at `opacity == 128` over an opaque backdrop
- **THEN** the result MUST be the source-over mix with effective source alpha `128/255`

### Requirement: Raster mask multiplies source alpha

An enabled layer raster mask SHALL multiply the source alpha by the mask sample at the corresponding pixel. A missing mask, a disabled mask, or a sample outside the mask rectangle MUST leave alpha unchanged when the mask default color is 255; a mask sample of 0 MUST suppress the layer's contribution at that pixel.

#### Scenario: Black mask hides the layer

- **WHEN** the mask sample is 0 at a pixel
- **THEN** that pixel MUST be left unchanged by the layer

#### Scenario: White mask reveals the layer

- **WHEN** the mask sample is 255
- **THEN** the layer MUST contribute at its full pre-opacity alpha

#### Scenario: Disabled mask is ignored

- **WHEN** `mask.disabled` is true
- **THEN** the mask MUST have no effect on the composite

### Requirement: Isolated group semantics

A group whose blend mode is not `PassThrough`, or a `PassThrough` group with opacity other than 255 or a mask, SHALL composite its children into a private fully transparent buffer and then blend that buffer into the parent as a single layer using the group's blend mode, opacity, and mask.

#### Scenario: Isolated group blends as one layer

- **WHEN** a `Multiply` group containing an opaque child sits over an opaque backdrop
- **THEN** the child MUST be composited into a transparent group buffer and that buffer MUST be multiplied into the backdrop as one layer

#### Scenario: Group opacity scales the isolated result

- **WHEN** an isolated group has `opacity == 128`
- **THEN** the group buffer's alpha MUST be scaled by `128/255` before it is blended

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

### Requirement: Off-canvas layers are clipped

The compositor SHALL clip every layer to the intersection of its `PsdRect` (in canvas coordinates) with the canvas. Zero-area and fully off-canvas layers MUST contribute nothing and MUST NOT panic.

#### Scenario: Partly off-canvas layer is clipped

- **WHEN** a layer rectangle extends beyond one or more canvas edges
- **THEN** only pixels inside the canvas MUST be written and no out-of-bounds index MUST be read

#### Scenario: Fully off-canvas and zero-area layers are inert

- **WHEN** a layer rectangle does not intersect the canvas, or has zero width or height
- **THEN** the output MUST be unchanged

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

### Requirement: Vector mask coverage multiplies source alpha

An enabled layer vector mask SHALL multiply the layer's effective source alpha by
the vector coverage sampled at the canvas pixel, in addition to the layer's
opacity, fill, and any raster mask, in both the CPU oracle and the GPU
compositor. The vector coverage and the raster coverage SHALL combine by
multiplication, so a pixel with either coverage at 0 SHALL suppress the layer's
contribution there. An absent or `disabled` vector mask SHALL leave the coverage
unchanged.

The coverage SHALL be sampled at pixel centres and evaluated in document
coordinates, because the vector mask path is document-relative. Closed subpaths
SHALL be combined under the mask's fill rule: even-odd unless a subpath declares
`non-zero`, in which case non-zero winding SHALL be used. An open subpath SHALL
contribute no coverage. When the mask's `invert` flag is set, the sampled
coverage SHALL be `255 − c`. When the path has no closed subpath, the coverage
SHALL be `255` (no clipping).

A vector mask SHALL make the GPU build its per-pixel mask plane rather than the
constant-coverage fill, so a scene with a vector mask SHALL remain within the
existing ±1 LSB CPU/GPU parity.

#### Scenario: A vector mask clips a fill layer

- **WHEN** a solid-fill layer covering the canvas carries a closed-rectangle
  vector mask over an opaque base
- **THEN** the pixels inside the rectangle carry the fill and the pixels outside
  it carry the base

#### Scenario: An inverted vector mask hides its inside

- **WHEN** a layer's vector mask has the `invert` flag set
- **THEN** the pixels inside the path are suppressed and the pixels outside it
  carry the layer's contribution

#### Scenario: A disabled vector mask is ignored

- **WHEN** a layer's vector mask has the `disable` flag set
- **THEN** the layer's contribution is unchanged by the vector mask

#### Scenario: Vector and raster masks combine

- **WHEN** a layer carries both an enabled raster mask and an enabled vector
  mask
- **THEN** the layer's source alpha is scaled by the product of the two
  coverages

#### Scenario: A vector mask with no closed subpath does not clip

- **WHEN** a layer's vector mask has no closed subpath
- **THEN** the layer renders unmasked by the vector mask

#### Scenario: The GPU agrees with the CPU with a vector mask

- **WHEN** a scene whose layer carries a vector mask is composited on the GPU
  and on the CPU
- **THEN** the two results agree within ±1 LSB per channel

### Requirement: Decode a preserved `vscg` block into typed fill content

The compositor SHALL decode a layer's preserved `'vscg'` additional-layer-info
block into the matching `pictura_adjust::Adjustment` variant when the layer has
no modeled adjustment block. The block SHALL be a big-endian 4-byte fill key
followed by a big-endian `u32` version that MUST equal `16` and a descriptor. The
descriptor SHALL be decoded by the same decoders the top-level `'SoCo'`,
`'GdFl'`, and `'PtFl'` blocks use, dispatching on its content: a `Grad` object
SHALL decode to `Adjustment::GradientFill`, a `Ptrn` object SHALL decode to
`Adjustment::PatternFill`, and a `Clr ` object SHALL decode to
`Adjustment::SolidFill`. The 4-byte key SHALL be consumed but SHALL NOT be the
dispatch authority, so a block whose key disagrees with its descriptor content
still decodes by content.

A missing key, a version other than `16`, a non-object descriptor, a descriptor
with none of those contents, or a malformed descriptor SHALL decode to `None`,
SHALL NOT change the backdrop, and SHALL NOT panic. The preserved `'vscg'` block
SHALL remain in `Layer.extra_blocks` and SHALL be re-emitted verbatim; the decode
SHALL be derived only.

#### Scenario: A `vscg` solid fill decodes to `SolidFill`

- **WHEN** a layer carries a `vscg` block whose descriptor holds a `Clr ` object
  with `Rd `/`Grn `/`Bl  ` doubles
- **THEN** the decoded fill is `Adjustment::SolidFill` with those components and
  alpha 255

#### Scenario: A `vscg` gradient fill reuses the gradient decoder

- **WHEN** a layer carries a `vscg` block whose descriptor holds `Angl`, a `Type`
  enum, and a custom-stop `Grad` object
- **THEN** the decoded fill is `Adjustment::GradientFill` with the same kind,
  stops, and angle a top-level `GdFl` block would yield

#### Scenario: A malformed `vscg` block is a no-op

- **WHEN** a `vscg` block has a version other than 16, a non-object descriptor,
  or a descriptor with none of `Grad`/`Ptrn`/`Clr `
- **THEN** the decode yields `None` and does not panic

#### Scenario: The decoded block is not serialized

- **WHEN** a document carrying a `vscg` block is written and read back
- **THEN** the block's bytes are preserved and the reconstructed document equals
  the input

### Requirement: A vector fill composites and is clipped by the vector mask

A layer whose decoded adjustment comes from `'vscg'` SHALL composite as
generative fill content over the layer rectangle clamped to the canvas, through
the normal layer path, so the layer's mask, opacity, fill, and blend mode apply
and pixels outside the rectangle are unchanged. The fill SHALL be clipped by the
layer's vector-mask coverage when the layer carries an enabled `'vmsk'`, and
SHALL cover the layer rectangle when the layer has no vector mask. A modeled
layer adjustment block SHALL take precedence, so `'vscg'` SHALL be consulted only
when the layer has no adjustment block; an adjustment block that does not decode
SHALL leave the layer a no-op and SHALL NOT fall through to its pixels.

A scene containing a vector fill SHALL composite through the CPU path: the GPU
compositor SHALL fall back rather than render a layer carrying a `vscg` block, so
the CPU/GPU parity contract holds. Layer effects SHALL gate their coverage by the
same decoded vector fill a shape layer's content uses.

#### Scenario: A `vscg` solid fill composites inside its vector mask

- **WHEN** a shape layer carrying a `vscg` solid fill and a closed-rectangle
  `vmsk` is composited over an opaque base
- **THEN** the pixels inside the rectangle carry the fill and the pixels outside
  it carry the base

#### Scenario: A vector fill without a vector mask fills the layer rect

- **WHEN** a layer carrying a `vscg` solid fill has no vector mask
- **THEN** its fill covers the layer rectangle and leaves pixels outside it
  unchanged

#### Scenario: A modeled adjustment block takes precedence over `vscg`

- **WHEN** a layer carries both a decodable `SoCo` adjustment block and a `vscg`
  block
- **THEN** the layer composites the adjustment block's fill

#### Scenario: A malformed `vscg` fill does not change the composite

- **WHEN** a layer's `vscg` block does not decode
- **THEN** the composite equals the same document with the `vscg` block absent

#### Scenario: The GPU falls back for a vector fill

- **WHEN** a document containing a visible layer with a `vscg` block is
  composited with GPU acceleration enabled
- **THEN** the compositor reports the CPU backend and produces the CPU result

### Requirement: Layer visibility toggles use the region fast path

A layer-visibility change SHALL refresh through the **region fast path** — the
Layers panel eye toggle and any `set_layers_visible` call composite the changed
layer's clamped document region and blit that region rather than always running
a full `recomposite()` and recording a full-document snapshot. The path SHALL fall back to `recomposite` only when the layer's region
is not representable (a `None` region, for example a channel-less or unbounded
layer), and the visible result SHALL be byte-identical to a full recomposite
either way. The change SHALL still record exactly one undo state and update the
row projection. On a large reference document (≈4000²) the visibility toggle to
first correct pixel SHALL complete in under one second on the reference run.

#### Scenario: Eye toggle uses the region path [lvc_visibility_region]

- **WHEN** the eye toggle flips a regular raster layer on a large document
- **THEN** a region refresh fires for the layer's clamped region, no full
  `changed` recomposite runs, and the canvas shows the toggled state

#### Scenario: The region path matches a full recomposite [lvc_visibility_parity]

- **WHEN** a visibility toggle completes through the region path
- **THEN** the displayed document equals a full recomposite of the same state
  byte for byte

#### Scenario: An unrepresentable region falls back [lvc_visibility_fallback]

- **WHEN** a visibility change targets a layer with no representable region
- **THEN** the system falls back to the full `recomposite` and still shows the
  correct state

#### Scenario: One undo state per toggle [lvc_visibility_undo]

- **WHEN** a visibility toggle completes
- **THEN** exactly one undo state is recorded and one undo restores the prior
  visibility

#### Scenario: Large-image toggle is sub-second [lvc_visibility_latency]

- **WHEN** the eye toggle runs on the ≈4000² reference document
- **THEN** the first correct pixel is shown in under one second
