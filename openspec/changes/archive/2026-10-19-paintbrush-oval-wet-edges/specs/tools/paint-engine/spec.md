## MODIFIED Requirements

### Requirement: Paint modes

The system SHALL support the paint modes `Normal`, `Dissolve`, `Behind`,
`Clear`, and `Multiply`. `Normal` interpolates the destination toward the
paint colour by the composited coverage; `Dissolve` selects the paint colour
stochastically per pixel in proportion to coverage; `Behind` writes only where
the destination is not fully opaque; `Clear` reduces the destination alpha by
the coverage; `Multiply` interpolates the destination toward the product of
the paint colour and the destination colour by the composited coverage, and
paints the plain colour where the destination is transparent.

#### Scenario: Normal blends by coverage

- **WHEN** a Normal stroke paints at full opacity over an opaque base
- **THEN** every fully covered pixel becomes the paint colour

#### Scenario: Clear reduces alpha

- **WHEN** a Clear stroke paints at full coverage on an opaque pixel
- **THEN** that pixel's alpha becomes zero

#### Scenario: Behind leaves opaque pixels untouched

- **WHEN** a Behind stroke paints over an opaque pixel
- **THEN** that pixel's colour is unchanged

#### Scenario: Multiply darkens by the backdrop

- **WHEN** a Multiply stroke paints at full opacity over an opaque base and
  over a transparent pixel
- **THEN** each opaque pixel becomes the channel-wise product of the paint
  colour and the base, and the transparent pixel takes the plain paint colour

## ADDED Requirements

### Requirement: Wet edges

`StrokeConfig::wet_edges` SHALL remap a Paint stroke's accumulated coverage
`a` through `f(a) = 1.3 · a · (1 − 0.76 · a)` clamped to 0..=1 before the
coverage is composited. A fully built-up stroke interior therefore paints
lighter than a single 50 %-flow dab, and the partially covered pixels at the
stroke's edge form a darker rim. Wet-edge strokes SHALL stay on the CPU path,
because the GPU dab shader does not model the curve. The curve is fitted to
the owner's Photoshop reference and is non-parity.

#### Scenario: Wet-edge transfer curve

- **WHEN** the wet-edge curve is evaluated at coverage 0, 128, and 255
- **THEN** it returns 0, about 40 % (103), and about 31 % (80), and it rises
  to a single peak before falling

#### Scenario: Wet edges lighten the core and darken the rim

- **WHEN** a hard 45 px oval stroke with Wet Edges paints `#47484a` at 35 %
  opacity and 50 % flow over white
- **THEN** the stroke interior lies 18–22 below white, a single dab's centre
  lies 24–28 below white, and the stroke's rim is darker than its interior
