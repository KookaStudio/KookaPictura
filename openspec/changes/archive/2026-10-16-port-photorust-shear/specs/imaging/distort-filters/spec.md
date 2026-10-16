## REMOVED Requirements

### Requirement: Shear

**Reason**: Shear was a vertical column shift: the curve indexed a column's
normalized horizontal position and shifted that column down by `y · height/2`.
CS6 (and photorust) run the other way round: the curve runs top to bottom and
shifts each row sideways. The requirement is replaced, not amended, because
the old scenarios ("displaces columns", vertical displacement) no longer
describe the filter.

**Migration**: Read "Shear row shift", which keeps the `Filter::Shear`
fields, the `-1.0..=1.0` curve bounds, the strictly-increasing first
coordinate, the two-point minimum, `FilterError::InvalidParams` rejection,
the fill modes, and the no-op and determinism contracts.

## ADDED Requirements

### Requirement: Shear row shift

The system SHALL implement `Filter::Shear { curve: Vec<(f64, f64)>, fill: ShearFill }`. It SHALL shift each row horizontally by the control-point `curve`: a cubic Hermite interpolation of the `(position, offset)` points with Catmull-Rom tangents, with `position` and `offset` in `-1.0..=1.0`, maps a row's normalized vertical position (top row `-1.0`, bottom row `1.0`) to a horizontal displacement of `offset × width/2` pixels, and the row SHALL be resampled bilinearly at the displaced source coordinate. The interpolation SHALL be continuous in slope at every control point, and a two-point curve SHALL be exactly the straight line between its points. A positive `offset` SHALL move that row's content to the right. `curve` SHALL contain at least two finite points with strictly increasing `position`; otherwise the system SHALL return `FilterError::InvalidParams`. A curve whose `offset` values are uniformly `0.0` SHALL be a no-op. `ShearFill::WrapAround` SHALL wrap columns shifted off-canvas around to the opposite edge, and `ShearFill::RepeatEdgePixels` SHALL repeat the nearest edge sample; the two modes MUST be observably different where a row's displacement moves columns beyond the image bounds. `ShearFill` is a closed set that requires no range check. Oracle expectation: the closest ImageMagick operator is `-shear {angle}x0` (a horizontal whole-canvas shear that grows and background-fills the canvas), but it is not faithful, so Shear SHALL be classified as no-equivalent with tolerance 0 and a property or known-value test. The measured delta against `-shear -26.565x0 -crop 16x16+4+0 +repage` for the curve `[(-1,-0.5),(1,0.5)]` is max 255 / mean 18.4 for `RepeatEdgePixels` (mean 23.4 for `WrapAround`).

#### Scenario: A flat curve is a no-op

- **WHEN** Shear is applied with a curve whose `offset` values are all `0.0`
- **THEN** the output equals the input within 1 LSB

#### Scenario: A curved control-point set displaces rows horizontally

- **WHEN** Shear is applied with a curve that is `0.0` at the top row and bends to a non-zero `offset` lower down
- **THEN** the top row is unchanged, the lower rows shift horizontally, and a row with a positive `offset` moves its content to the right

#### Scenario: A two-point curve is a straight line

- **WHEN** Shear is applied with exactly two control points at `(-1.0, -0.5)` and `(1.0, 0.5)`
- **THEN** the horizontal displacement grows linearly with the row position, with no easing at the ends

#### Scenario: An invalid curve is rejected

- **WHEN** Shear is applied with fewer than two points, a non-finite coordinate, `position` values that are not strictly increasing, or a `position` / `offset` outside `-1.0..=1.0`
- **THEN** `apply` returns `FilterError::InvalidParams` and leaves the buffer unchanged

#### Scenario: The two fill modes differ off-canvas

- **WHEN** Shear with the same curve but `ShearFill::WrapAround` and `ShearFill::RepeatEdgePixels` is applied to a buffer whose displacement moves rows beyond an edge
- **THEN** the two outputs differ

#### Scenario: No faithful equivalent is asserted without measurement

- **WHEN** the oracle mapping table is checked
- **THEN** Shear has a row naming `-shear {angle}x0`, tolerance 0, a non-empty no-equivalent note, and the recorded measured maximum and mean delta
