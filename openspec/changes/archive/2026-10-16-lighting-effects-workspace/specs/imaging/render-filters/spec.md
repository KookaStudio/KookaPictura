## MODIFIED Requirements

### Requirement: Lighting Effects filter

The system SHALL implement `Filter::Lighting` over a CS6 light rig: shared Colorize colour, Ambience, Exposure, Gloss, Metallic (each −100..=100), a bump Texture channel (None/Red/Green/Blue), and Height (0..=100), plus 1..=16 lights. Each light SHALL carry a type (Spot, Point, Infinite), a visible flag, colour, Intensity and Hotspot (−100..=100), a unit centre, an angle, a size and width (fractions of the half-diagonal), and an elevation (0..=90°). A Spot SHALL light an ellipse with semi-axes size and width along its angle, at full strength inside a hotspot ellipse lying toward the aimed end and fading to nothing at the outer edge. A Point SHALL fall off with distance to its radius. An Infinite light SHALL light the whole frame evenly from its angle and elevation. Every visible light SHALL add an ambient-relative diffuse term and, with Gloss above zero, a Blinn highlight, over the layer colour and the selected bump channel's gradient. A hidden light SHALL contribute nothing. Intensity 50 SHALL leave a lit flat surface at about its own brightness, and negative intensity SHALL remove light. The default SHALL be CS6's Default style: one white Spot at the unit centre with intensity 35 and hotspot 69. Sizes and positions SHALL be frame-relative, so a scaled copy lights the same. Alpha SHALL be preserved, output SHALL be deterministic, and an out-of-range parameter or a rig of no lights or more than 16 SHALL be rejected as `FilterError::InvalidParams` without mutating the buffer. The model is photorust's shading tuned by eye against CS6, a behavioral approximation of a closed algorithm.

#### Scenario: Lighting relights the colour planes and preserves alpha

- **WHEN** the default rig lights an RGBA frame whose alpha is 90
- **THEN** the colour planes change and the alpha plane is bit-identical

#### Scenario: A spot lights its ellipse toward its aim

- **WHEN** the default rig lights an even grey frame, and a Spot aimed along +x is sampled ahead of and behind its centre
- **THEN** the frame's middle is lit, a far corner is black, and the aimed-at side is brighter than the side behind

#### Scenario: Each light type differs

- **WHEN** the same frame is lit by a Spot, a Point, and an Infinite light
- **THEN** the three outputs differ

#### Scenario: Ambience, intensity, and falloff behave as in CS6

- **WHEN** the same frame is lit by each light type, with ambience raised, and with intensity raised, lowered below zero, or exposure raised
- **THEN** an Infinite light is even across the frame, a Point falls off faster than a Spot's hotspot, ambience lifts unlit pixels, and intensity and exposure raise the light while negative intensity lowers it

#### Scenario: Lights add up and hidden lights do not

- **WHEN** a rig of two identical Spots is compared with one, and a rig whose second light is hidden is compared with the single light
- **THEN** the pair lights more brightly, and the hidden-light rig is bit-identical to the single light

#### Scenario: A texture shapes the light and the rig is size-invariant

- **WHEN** a glossy rig lights a striped bump channel, and the same two-light rig lights a 600×400 and a 150×100 frame
- **THEN** the bump spreads the lit values far more than a flat frame, and each small pixel is within 8 levels of its 4×4 large block

#### Scenario: Bad parameters are refused untouched

- **WHEN** the rig has no lights, 17 lights, an out-of-range or non-finite parameter, a zero size, or an elevation above 90
- **THEN** the operator returns `FilterError::InvalidParams` and the buffer is bit-identical to its prior state, while a rig of exactly 16 lights is accepted
