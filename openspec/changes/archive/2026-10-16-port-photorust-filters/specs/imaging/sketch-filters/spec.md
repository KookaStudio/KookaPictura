## MODIFIED Requirements

### Requirement: Bas Relief

The system SHALL implement `Filter::BasRelief { detail, smoothness, light_direction, foreground, background }` as a low relief: the image's luminance, softened by `smoothness`, is read as a height field lit from `light_direction`; a slope facing the light SHALL be driven toward the `background` colour and a slope facing away toward the `foreground` colour, by a gain that grows with `detail`, and a flat area, which catches no light, SHALL sit at the midpoint of the two colours.

#### Scenario: Flat areas sit at the midpoint

- **WHEN** Bas Relief is applied to an image with broad flat regions
- **THEN** the flat regions take the midpoint of the `foreground` and `background` colours within 2 levels

#### Scenario: Dark and light areas take foreground and background

- **WHEN** Bas Relief is applied across a strong step between a dark and a light region
- **THEN** the pixels at the step move away from the midpoint, the slope facing the light toward the `background` colour and the slope facing away toward the `foreground` colour

#### Scenario: Light direction moves the illumination

- **WHEN** Bas Relief is applied with two different `light_direction` values
- **THEN** the two outputs differ
