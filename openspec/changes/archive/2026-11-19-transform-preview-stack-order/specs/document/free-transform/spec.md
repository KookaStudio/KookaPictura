## ADDED Requirements

### Requirement: Transform preview keeps the layer stack order

While a Free Transform session previews a target below other layers, the canvas
SHALL draw the layers above the transform's hidden set over the transformed
image, clipped to the transformed image's footprint, so a lower layer does not
hide the layers above it for the length of the gesture. The above-layer image
SHALL be built when the session begins, with the transform's target, linked set,
or clipping group and every layer below it hidden. A transform of the topmost
layer MUST NOT build an above-layer image. The rule SHALL apply to every
transform mode, including similarity, Distort, Perspective, Skew, and Warp.

#### Scenario: A lower layer stays under the layers above [ft_preview_stack]

- **WHEN** a layer below an opaque layer is scaled so its transformed content
  overlaps the opaque layer
- **THEN** the opaque layer still shows over the transformed content at the
  overlap, and the transformed content shows outside it

#### Scenario: The topmost layer builds no above-layer image [ft_preview_top]

- **WHEN** the topmost layer of a document is transformed
- **THEN** no above-layer image is prepared and the preview is unchanged
