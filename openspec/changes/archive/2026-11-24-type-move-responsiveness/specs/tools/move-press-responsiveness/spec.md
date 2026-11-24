# tools/move-press-responsiveness Specification

## ADDED Requirements

### Requirement: A type layer's Move is sized by its ink

When the Move tool presses on a type layer, the preview and the commit SHALL be
sized by the layer's ink plus its enabled effect pad, not its paragraph box or
the canvas. A type layer with no raster channels SHALL be materialized once on
the press, so the ink can be measured and later presses reuse the raster. A
clipped member, whose base is stationary, is also sized by its own ink. A
knockout, a clipped run above a base, or a vector mask MAY keep the conservative
reach because the layer's appearance then depends on more than its raster. An
empty or disabled effects block MUST NOT make the reach the canvas.

#### Scenario: A large paragraph box moves by its ink [mvp_type_ink]
- **WHEN** a paragraph type layer whose box is large but not near the canvas is moved
- **THEN** the moving image and the damage cover the ink, not the box

#### Scenario: An effect adds only its pad [mvp_type_effects]
- **WHEN** a type layer carrying an enabled drop shadow is moved
- **THEN** the reach is the ink plus the shadow pad and is smaller than the paragraph box

#### Scenario: A channel-less type layer materializes [mvp_type_source]
- **WHEN** a type layer with no raster channels is pressed with the Move tool
- **THEN** the press renders its raster once and the move crop is the ink

#### Scenario: A clipped type layer moves by its own ink [mvp_type_clipped]
- **WHEN** a type layer clipped to a stationary base is moved
- **THEN** the move crop is the type's ink, not the base's reach
