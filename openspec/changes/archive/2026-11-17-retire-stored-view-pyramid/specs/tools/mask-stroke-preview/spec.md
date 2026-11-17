## MODIFIED Requirements

### Requirement: Reduced-resolution preview for large mask dabs without the GPU stroke

When a mask dab's bounding box exceeds the raster budget that the pixel-stroke
preview uses and the GPU mask stroke is not taken, the system SHALL present the
in-progress mask stroke at the tile-pyramid level that the pixel preview would
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
