## ADDED Requirements

### Requirement: Healing engine

`pictura_paint::healing` SHALL rebuild a region of an RGBA image from the
pixels around it or from an offset source. `heal_region` SHALL fill the region
marked by a coverage mask using one of the three Spot Healing types — Proximity
Match (a Laplace solve with the surrounding ring as a fixed boundary), Create
Texture (the same base plus noise matched to the ring's roughness), and
Content-Aware (patch synthesis). `clone_region` SHALL transplant a source
offset's texture with a Poisson solve, keeping the destination's lighting as
the boundary; `Transfer::TextureOnly` SHALL keep the destination's colour. A
zero-area mask, a zero clone offset, or a hole with no boundary SHALL be a
no-op. The result SHALL be deterministic.

#### Scenario: A blemish on a flat field disappears

- **WHEN** `heal_region` runs Proximity Match over a blemish on a flat field
- **THEN** every pixel under the mask returns to the field value

#### Scenario: Content-Aware carries an edge

- **WHEN** `heal_region` runs Content-Aware over a hole straddling a brightness step
- **THEN** the fill keeps the step instead of flattening it

### Requirement: Healing gesture

`HealStroke` SHALL accumulate a brush coverage mask over one pixel layer
(`HealStroke::begin`), grow it per dab, and run the heal once on commit
(`HealStroke::commit`), refusing a non-raster layer, a group, an adjustment
layer, or a pixel-locked layer. A transparency lock SHALL preserve every
pixel's alpha.

#### Scenario: An empty gesture commits nothing

- **WHEN** `HealStroke::commit` runs with no dab placed
- **THEN** it returns `None` and the document is unchanged

### Requirement: Spot Healing Brush tool

The Spot Healing Brush SHALL rebuild the region under a click or drag from its
surroundings on release, using the options bar's Type (Proximity Match, Create
Texture, Content-Aware), and SHALL record exactly one "Spot Healing Brush"
history state. It SHALL refuse a locked or non-raster target.

#### Scenario: Healing a blemish

- **WHEN** the `healing_tools` self-test clicks the Spot Healing Brush on a black blemish on white
- **THEN** one "Spot Healing Brush" state is recorded and the sampled pixel is near white

### Requirement: Healing Brush tool

The Healing Brush SHALL require an Alt-click sample point before painting and
SHALL transplant the sampled source's texture with the destination's lighting,
recording exactly one "Healing Brush" history state per stroke. Painting before
a sample SHALL be refused with no history state.

#### Scenario: Sample then paint

- **WHEN** the `healing_tools` self-test paints before sampling and then Alt-clicks and paints
- **THEN** the first press records nothing and the second records one "Healing Brush" state
