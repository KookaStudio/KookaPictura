## MODIFIED Requirements

### Requirement: Emboss

The system SHALL implement `Filter::Emboss { angle: f64, height: f64, amount: f64 }`. It SHALL difference each colour channel across `height` pixels along `angle`, the direction the light comes from, add a 128 mid-gray bias, and scale the deviation from mid-gray by `amount` percent, so flat areas become mid-gray and edges carry a light and a dark side traced in the original colour, as CS6 Help describes ("converting its fill color to gray and tracing the edges with the original fill color"). A gray image SHALL stay gray. `angle` SHALL be finite and within `-360.0..=360.0`; `height` and `amount` SHALL be finite and strictly greater than 0. A value outside those ranges SHALL be rejected with `FilterError::InvalidParams`. Oracle expectation: classified as no-equivalent (ImageMagick `-emboss` has no angle, height, or amount), covered by property tests.

#### Scenario: The output is gray

- **WHEN** Emboss is applied to a gray image (R, G, and B planes equal)
- **THEN** every output pixel has R, G, and B equal to one another; a colour image keeps its edge colours (see "Coloured edges keep their colour")

#### Scenario: Coloured edges keep their colour

- **WHEN** Emboss is applied to a colour image with coloured edges
- **THEN** some output pixel has R different from B

#### Scenario: Negating the angle swaps the highlight and shadow

- **WHEN** Emboss is applied to the same edge image with the light at angle A and at the opposite side, A + 180
- **THEN** the highlight side of the edge under one becomes the shadow side under the other

#### Scenario: A uniform field becomes mid-gray

- **WHEN** Emboss is applied to a uniform-color buffer
- **THEN** every output color sample equals 128 within 1 LSB

#### Scenario: Invalid parameters are rejected

- **WHEN** Emboss is applied with an angle outside `-360.0..=360.0`, or with a non-finite angle, height, or amount, or with `height <= 0` or `amount <= 0`
- **THEN** `apply` returns `FilterError::InvalidParams` and does not panic
