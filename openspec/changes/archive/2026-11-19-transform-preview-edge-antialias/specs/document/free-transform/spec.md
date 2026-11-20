## ADDED Requirements

### Requirement: Transform preview anti-aliases resampled edges

While a Free Transform session previews a resized or rotated layer, the canvas
SHALL sample the moving layer as if the area outside the session's source
rectangle were transparent, so the preview's edge receives the same partial
coverage as the committed resample. A destination pixel on the transformed edge
SHALL be allowed to sample just outside the source rectangle, and taps outside
the source rectangle or image SHALL contribute zero coverage. The rule SHALL
apply to the similarity, Distort, Perspective, Skew, and Warp preview paths that
use the CPU warp.

#### Scenario: A resized solid edge matches the commit [ft_preview_edge]

- **WHEN** a solid pixel layer is scaled so one edge lands on a half pixel and
  the live preview is painted
- **THEN** the preview's edge pixel has partial coverage, not a hard
  opaque/transparent step, and matches the committed resample's edge
