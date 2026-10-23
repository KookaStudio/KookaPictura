# Spec Delta

## ADDED Requirements

### Requirement: Mixer Brush applies Mode and Opacity

The Mixer Brush deposit SHALL composite through the stroke's paint mode and
SHALL be capped by the stroke's Opacity, using the same blend and coverage rules
as the Brush. The options bar SHALL expose a Mode control and an Opacity control
for the Mixer Brush.

#### Scenario: Multiply mixes through the backdrop [mx_mode_multiply]

- **WHEN** a Mixer dab with Mode Multiply deposits a colour over an opaque base
- **THEN** the pixel becomes the channel-wise product of the deposit and the
  base, matching the shared Multiply paint mode

#### Scenario: Opacity caps the deposit [mx_opacity_cap]

- **WHEN** a Mixer dab runs at Opacity 50 % over an opaque base
- **THEN** a covered pixel moves half way from its base colour to the
  full-opacity result

#### Scenario: The Mixer bar exposes Mode and Opacity [mx_options_expose]

- **WHEN** the Mixer Brush tool is active
- **THEN** the options bar shows a paint-mode control and an opacity control,
  and changing either affects the next stroke
