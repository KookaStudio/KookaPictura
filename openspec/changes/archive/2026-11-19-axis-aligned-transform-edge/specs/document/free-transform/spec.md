## REMOVED Requirements

### Requirement: Transform preview anti-aliases resampled edges

**Reason**: The preview now follows the commit's edge rule, which keeps a hard
edge for axis-aligned transforms and pads only rotated, skewed, or projective
edges.

**Migration**: Use "Transform preview matches the committed edge" below.

## ADDED Requirements

### Requirement: Axis-aligned transforms keep a hard edge

The layer resampler SHALL pad the source with transparent samples only when the
transform's edges are not axis-aligned. For a similarity map whose rotation is a
multiple of 90°, and for a projective map with no x/y coupling, it SHALL clamp
the kernel at the source edge, so a solid layer remains fully opaque to its
transformed edge and does not gain a gray border. A rotated, skewed, or
projective map SHALL keep the transparent padding so its edge anti-aliases.

#### Scenario: An axis-aligned scale keeps a hard edge [ft_axis_aligned_hard_edge]

- **WHEN** a solid pixel layer is scaled uniformly by an integer factor with no
  rotation
- **THEN** every pixel of the transformed layer, including its edge row and
  column, has full alpha

#### Scenario: A rotated edge still anti-aliases [ft_preview_edge_rotated]

- **WHEN** a solid pixel layer is rotated by an angle that is not a multiple of
  90°
- **THEN** its transformed edge has fractional alpha

### Requirement: Transform preview matches the committed edge

While a Free Transform session previews a transformed layer, the canvas SHALL
sample the moving layer with the same edge rule as the commit: an axis-aligned
map SHALL clamp at the source edge, and a rotated, skewed, or projective map
SHALL sample as if the area outside the source rectangle were transparent. A
destination pixel on a padded edge SHALL be allowed to sample just outside the
source rectangle. The preview's edge SHALL match the committed resample.

#### Scenario: An axis-aligned preview edge is hard [ft_preview_edge]

- **WHEN** a solid pixel layer is scaled vertically or horizontally and the live
  preview is painted
- **THEN** the preview edge is a hard opaque/transparent step, not an
  intermediate gray row, and the committed edge matches it
