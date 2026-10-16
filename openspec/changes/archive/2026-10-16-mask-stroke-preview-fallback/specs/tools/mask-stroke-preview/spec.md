## ADDED Requirements

### Requirement: Reduced-resolution preview for large mask dabs without the GPU stroke

When a mask dab's bounding box exceeds the raster budget that the pixel-stroke
preview uses and the GPU mask stroke is not taken, the system SHALL present the
in-progress mask stroke at the view-pyramid level that the pixel preview would
use. The preview MUST NOT write level 0 or the mask. The preview MAY
approximate how the edited coverage appears in the composite. At the commit,
the system SHALL replay the recorded samples through the exact full-resolution
mask stroke, so the committed mask equals the result of the same samples
rasterized without a preview. A cancelled previewed mask stroke SHALL leave the
mask and the displayed document as they were before the stroke.

#### Scenario: A large mask dab without the GPU stroke is previewed [msp_large_mask_dab_previewed]

- **WHEN** a mask stroke runs with GPU compute off and a dab bounding box above the raster budget on a document whose pyramid has that level
- **THEN** the view presents the stroke at that level, and the mask is unchanged until the commit

#### Scenario: The preview interpolates between hidden and revealed [msp_preview_blend]

- **WHEN** the preview presents a level pixel whose coverage moves the mask value from `m0` to `m1`
- **THEN** the presented pixel is the stroke-start snapshot plus `(m1 - m0) / 255` of the difference between the reduced revealed and hidden composites, and zero coverage presents the snapshot unchanged

#### Scenario: The commit writes the exact mask [msp_commit_exact]

- **WHEN** a previewed mask stroke ends
- **THEN** the mask holds the exact stroke's coverage, the composite shows it, and one history state is recorded

#### Scenario: Cancelling a previewed mask stroke restores it [msp_cancel]

- **WHEN** a previewed mask stroke is cancelled
- **THEN** no history state is recorded and the mask and the composite are the pre-stroke ones

### Requirement: The commit removes preview-only pixels

When a previewed mask stroke commits, every preview-level pixel the preview
painted and the exact stroke did not change SHALL show its stroke-start value.

#### Scenario: A sample the exact spacing skips [msp_restore_spacing]

- **WHEN** a previewed mask stroke's last sample lies closer to the previous dab than the brush spacing, so the exact stroke places no dab there while the preview does
- **THEN** after the commit the preview level beyond the exact stroke's extent shows its stroke-start pixels

#### Scenario: A dab crossing the mask's rectangle [msp_restore_mask_edge]

- **WHEN** a previewed mask dab extends past the mask's rectangle over visible layer content
- **THEN** after the commit the preview level outside the mask's rectangle shows its stroke-start pixels

### Requirement: A mask stroke starts without copying the mask

Beginning a brush, pencil or eraser stroke on a layer mask MUST NOT copy the
whole mask and MUST NOT build a composite of it. The stroke SHALL paint a
document whose only channel shares the mask's coverage plane.

#### Scenario: The paint document shares the coverage [msp_stroke_start]

- **WHEN** a mask stroke's paint document is built
- **THEN** its one channel shares the mask's plane and it has no composite
