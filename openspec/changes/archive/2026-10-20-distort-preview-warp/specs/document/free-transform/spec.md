## ADDED Requirements

### Requirement: Live projective preview uses a CPU warp

The live preview for a projective transform SHALL render the moving layer by
inverse-mapping the visible destination region through the projective matrix
and sampling the preview layer image on the CPU, rather than drawing the image
under a projective `QTransform`. The warp SHALL keep the whole shape at every
convex quad, SHALL omit pixels on the far side of the horizon, and SHALL be
bounded by the visible canvas region sampled at the view's scale (at most one
sample per document pixel). A source that crosses the horizon SHALL NOT be
bounded by its projected corners. The warp SHALL be rebuilt when the view pans,
zooms, rotates, or resizes. While a preset warp is active the preview SHALL
draw the whole warped preview layer. When the warp produces no image the
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

#### Scenario: A zoomed-out preview samples at the view scale [distort_preview_scale]

- **WHEN** the view is zoomed out below 100% during a live projective preview
- **THEN** the warp samples about one output pixel per screen pixel, covering the same document region

#### Scenario: Changing the viewport redraws the preview [distort_preview_viewport]

- **WHEN** the view zooms out or pans during a live projective preview without another transform update
- **THEN** the newly visible parts of the moving layer are drawn

#### Scenario: A preset warp is not clipped to the original box [warp_preview_whole]

- **WHEN** a preset warp is applied after the transform moved content outside the original box
- **THEN** the preview draws that content
