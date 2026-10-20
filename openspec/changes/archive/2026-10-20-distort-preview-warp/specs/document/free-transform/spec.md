## ADDED Requirements

### Requirement: Live projective preview uses a CPU warp

The live preview for a projective transform SHALL render the moving layer by
inverse-mapping the visible destination region through the projective matrix
and sampling the preview layer image on the CPU, rather than drawing the image
under a projective `QTransform`. The warp SHALL keep the whole shape at every
convex quad, SHALL omit pixels on the far side of the horizon, and SHALL be
bounded by the visible canvas region. When the warp produces no image the
preview MAY fall back to the projective `QTransform` draw.

#### Scenario: A steep Distort keeps the whole shape [distort_preview_full]

- **WHEN** a live Distort preview maps a filled rectangle to a steep convex quad
- **THEN** the preview shows the layer across the whole quad, not one triangle

#### Scenario: Masked and clipped content survives the warp [distort_preview_layers]

- **WHEN** the moving layer's appearance comes from a layer mask, vector mask, or clipping group
- **THEN** the warped preview shows that appearance across the whole quad

#### Scenario: The far side of the horizon is omitted [distort_preview_front]

- **WHEN** the warp's projective map has a horizon through the source
- **THEN** the preview draws only the front side and leaves the far side transparent
