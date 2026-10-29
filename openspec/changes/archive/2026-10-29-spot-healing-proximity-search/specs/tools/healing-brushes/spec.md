# Spec Delta

## MODIFIED Requirements

### Requirement: Healing engine

`pictura_paint::healing` SHALL rebuild a region of an RGBA image from the
pixels around it or from an offset source. `heal_region` SHALL fill the region
marked by a coverage mask using one of the three Spot Healing types:

- **Proximity Match** SHALL search the area around the hole for a source
  offset. The chosen offset's surroundings SHALL best match the hole's
  surroundings, and its interior SHALL be closest to a smooth fill. The source
  SHALL NOT overlap the hole. The engine SHALL transplant that source with a
  Poisson solve, holding the destination as the boundary. With no source inside
  the image, it SHALL fall back to a Laplace solve with the surrounding ring as
  a fixed boundary.
- **Create Texture** SHALL use the Laplace base plus noise matched to the ring's
  roughness.
- **Content-Aware** SHALL use patch synthesis.

For Proximity Match and Create Texture, the rebuilt hole SHALL extend a margin
past the fully covered brush core. The margin SHALL be sized from the core's
thickness, at least 3 and at most 8 pixels. It SHALL fade out at its outer edge,
and the returned rectangle SHALL include it. A defect's soft rim just outside
the brush is therefore rebuilt and not used as the fill's boundary. Pixels
beyond the margin SHALL NOT change.

`clone_region` SHALL transplant a source offset's texture with a Poisson solve,
keeping the destination's lighting as the boundary. `Transfer::TextureOnly`
SHALL keep the destination's colour. A zero-area mask, a zero clone offset, or
a hole with no boundary SHALL be a no-op. The result SHALL be deterministic.
Adobe's metric is undocumented, so this is behavioural parity only.

#### Scenario: A blemish on a flat field disappears

- **WHEN** `heal_region` runs Proximity Match over a blemish on a flat field
- **THEN** every pixel under the mask returns to the field value

#### Scenario: Content-Aware carries an edge

- **WHEN** `heal_region` runs Content-Aware over a hole straddling a brightness step
- **THEN** the fill keeps the step instead of flattening it

#### Scenario: A brush the size of a dot removes the dot

- **WHEN** a single Spot Healing dab with a hard brush whose diameter equals an
  anti-aliased black dot's lands on the dot on a sky gradient, in Proximity Match
  or Create Texture
- **THEN** every pixel around the dot is within a small tolerance of the clean
  sky, and no grey remains

#### Scenario: A heal next to an edge leaves no trace

- **WHEN** Proximity Match heals a dot whose brush overlaps a high-contrast
  vertical edge
- **THEN** the edge is carried through the hole as a step on every row, not
  smeared into it

#### Scenario: Another defect is not used as the source

- **WHEN** Proximity Match heals one of two identical dots in a row on a sky
  gradient
- **THEN** the healed dot is within a small tolerance of the clean sky, and no
  copy of the other dot appears

#### Scenario: Pixels beyond the margin are untouched

- **WHEN** Proximity Match heals a fully covered square
- **THEN** pixels just past the margin keep their values, and the returned
  rectangle is the region grown by the margin

### Requirement: Spot Healing Brush tool

The Spot Healing Brush SHALL rebuild the region under a click or drag from its
surroundings on release, using the options bar's Type (Proximity Match, Create
Texture, Content-Aware), and SHALL record exactly one "Spot Healing Brush"
history state. It SHALL refuse a locked or non-raster target. A single click
with a brush the size of a defect SHALL remove the defect. It SHALL NOT leave
grey in its place.

#### Scenario: Healing a blemish

- **WHEN** the `healing_tools` self-test clicks the Spot Healing Brush on a black blemish on white
- **THEN** one "Spot Healing Brush" state is recorded and the sampled pixel is near white

#### Scenario: A click with a dot-sized brush removes the dot

- **WHEN** the Spot Healing Brush, in Proximity Match with a brush the size of a
  black dot on a sky gradient, is clicked once on the dot
- **THEN** one "Spot Healing Brush" state is recorded and every pixel around the
  dot is close to the clean sky
