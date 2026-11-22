# tools/crop-resampling Specification

## Purpose
The Crop tool's `W x H x Resolution` output-size mode: the pixel dimensions are
an output target, the crop box is locked to their ratio, and the commit
resamples the cropped region to exactly that size at the requested resolution.

## ADDED Requirements

### Requirement: Output-size fields

Selecting `W x H x Resolution` SHALL show the pixel Width and Height fields with
their `px` suffix, the Resolution field, and the `px/in` / `px/cm` unit. The
fields SHALL be seeded from the current crop box and the document's resolution
(72 ppi when it has none).

#### Scenario: The mode shows its fields [cr_mode_fields]

- **WHEN** the Crop tool is active and `W x H x Resolution` is chosen
- **THEN** the W/H fields show the crop box's pixel size, the Resolution field
  shows the document ppi, and the unit combo is visible

### Requirement: Box ratio lock

While the output-size mode is active, the crop box SHALL be constrained to the
W:H ratio. Editing W or H SHALL change the output target and the ratio without
overwriting the other field from the box. Swapping SHALL exchange W and H.
Clear and Reset SHALL clear the target and the ratio.

#### Scenario: Editing W keeps the target [cr_target_lock]

- **WHEN** the output-size mode is active and W is changed
- **THEN** the crop box is refitted to the new W:H ratio, the field keeps the
  typed target value, and the box's own pixel size may differ from the target

#### Scenario: Swap exchanges the target [cr_swap]

- **WHEN** the output-size mode is active and Swap is pressed
- **THEN** the W and H fields exchange and the crop box ratio is inverted

#### Scenario: Clear drops the target [cr_clear_target]

- **WHEN** Clear or Reset is pressed in the output-size mode
- **THEN** the ratio becomes unconstrained, the W/H fields blank, and a commit
  no longer resamples

### Requirement: Resampled commit

Committing an output-size crop SHALL crop to the box, resample the cropped
canvas to exactly the target Width x Height with Bicubic interpolation, and
write the target resolution and unit to the document. The commit SHALL record
one history state. A straighten commit with an output target SHALL apply the
same resampling after the rotation and crop.

#### Scenario: A crop resamples to the target [cr_commit_resample]

- **WHEN** a 40 x 24 document is cropped in the output-size mode with a target
  of 20 x 10 at 300 px/in
- **THEN** the committed document is exactly 20 x 10 pixels and reads 300 ppi

#### Scenario: The unit is written [cr_resolution_unit]

- **WHEN** the output-size target is 300 with the `px/cm` unit
- **THEN** the committed document reports its resolution as 300 pixels per
  centimetre

#### Scenario: Straighten resamples too [cr_straighten_resample]

- **WHEN** a straighten commit is made with an output target
- **THEN** the rotated and cropped result is resampled to the target size and
  the target resolution is written
