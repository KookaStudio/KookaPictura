## REMOVED Requirements

### Requirement: Mixer Brush applies Mode and Opacity

**Reason**: The CC options bar has no Mode or Opacity for the Mixer Brush (#911); the requirement is split into an engine requirement and a bar requirement below.
**Migration**: `mx_mode_multiply` and `mx_opacity_cap` move to "Mixer Brush engine applies Mode and Opacity"; `mx_options_expose` is replaced by `mx_bar_cc`.

## ADDED Requirements

### Requirement: Mixer Brush engine applies Mode and Opacity

The Mixer Brush engine SHALL composite its deposit through the stroke's paint mode and SHALL cap it by the stroke's Opacity, using the same blend and coverage rules as the Brush.

#### Scenario: Multiply mixes through the backdrop [mx_mode_multiply]

- **WHEN** a Mixer dab with Mode Multiply deposits a colour over an opaque base
- **THEN** the pixel becomes the channel-wise product of the deposit and the base, matching the shared Multiply paint mode

#### Scenario: Opacity caps the deposit [mx_opacity_cap]

- **WHEN** a Mixer dab runs at Opacity 50 % over an opaque base
- **THEN** a covered pixel moves half way from its base colour to the full-opacity result

### Requirement: Mixer Brush bar matches CC

The Mixer Brush options bar SHALL NOT show a Mode or Opacity control, and a Mixer Brush stroke SHALL run in Normal mode at 100 % Opacity regardless of the Brush's settings. The bar SHALL show an enabled Sample All Layers checkbox that writes the Mixer options.

#### Scenario: The Mixer bar matches CC [mx_bar_cc]

- **WHEN** the Mixer Brush tool is active
- **THEN** the options bar shows no Mode or Opacity control and shows an enabled Sample All Layers checkbox that writes the Mixer options

### Requirement: Mixer Brush Sample All Layers

With Sample All Layers on, the Mixer Brush pickup SHALL average the colour under the dab from the visible composite, snapshotted when the stroke begins, instead of the active layer. The deposit SHALL still be written to the active layer. With it off the pickup SHALL read the active layer.

#### Scenario: The composite supplies the pickup [mx_sample_all]

- **WHEN** a wet dab lands on a white layer while the composite under it is blue and Sample All Layers is on
- **THEN** the deposit and the brush's paint take on blue, whereas with it off the dab leaves the white layer white

### Requirement: Mixer Brush mixing is an approximation

Adobe's Mixer Brush mixing and dry-out math is closed. The engine SHALL be documented as behavioural parity only, naming its limits: a single flat load colour, a tip-weighted mean pickup, and photorust's rates and preset values. The Dry, Moist and Very Wet settings SHALL order by how much canvas colour a stroke carries.

#### Scenario: Presets order by wetness [mx_preset_order]

- **WHEN** the same red brush strokes from a blue band into white at Dry, Moist and Very Wet settings
- **THEN** Dry lays down pure red, Moist carries blue into the stroke, and Very Wet leaves the stroke with no red tint from the brush
