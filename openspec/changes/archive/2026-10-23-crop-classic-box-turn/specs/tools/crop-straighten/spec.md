## MODIFIED Requirements

### Requirement: Straighten reference line angle

A straighten reference line from A to B SHALL rotate the content by the
negative of the line's inclination from the nearer of the horizontal and the
vertical, so the result lies in (-45°, 45°] whichever direction the line is
drawn. In Modern mode the rotation SHALL add to any angle already set in the
session, so a second line levels what is on screen. In Classic mode the line is
drawn on the fixed image, so the box SHALL level against it absolutely, with no
earlier angle added. While the line is drawn the image SHALL
hold still and the rotation the release will apply SHALL show beside the pointer
in the rotate gesture's `∠: <angle>°` readout; releasing the line SHALL rotate
the image preview by that angle, keep the crop active, and clear the readout.

#### Scenario: A line at θ commits a rotation of −θ [lcs_straighten_angle]

- **WHEN** a reference line at θ = 5° from the horizontal is drawn and the crop is committed
- **THEN** the content is rotated by −θ within 0.1°

#### Scenario: The image rotates on release [lcs_straighten_release]

- **WHEN** a reference line is dragged and then released
- **THEN** the image stays unrotated during the drag, with the readout showing the coming angle, and the preview rotates by that angle on release

#### Scenario: Plumb lines and either direction [lcs_straighten_axis]

- **WHEN** a near-vertical line, or a line drawn right to left, is drawn
- **THEN** the angle levels it against the vertical or horizontal respectively, never rotating by more than 45°


#### Scenario: A second line in Classic mode is absolute [lcs_straighten_classic_absolute]

- **WHEN** Classic mode is on and a 5° line is drawn, then a 3° line
- **THEN** the box is levelled to the 3° line, not to the sum of the two
