# Spec Delta — ui/icon-assets

## MODIFIED Requirements

### Requirement: Icons follow the active theme

The system SHALL render each icon in the active theme's dedicated icon tint
token at render time, SHALL render the dark family's icons from a `#FFFFFF`
base at 80 % opacity and the light family's icons from a dark base, SHALL render
a disabled icon in the palette's disabled colour, and SHALL produce updated
pixmaps when the theme brightness or family changes so no stale colour survives.
An explicit `icon(id, color)` SHALL keep overriding the theme tint.

#### Scenario: Theme foreground is applied [lia_foreground]

- **WHEN** an icon is rendered with the default call on the dark family
- **THEN** its opaque pixels use the icon token's `#FFFFFF` at 80 % opacity

#### Scenario: Disabled mode differs [lia_disabled]

- **WHEN** `icon(id).pixmap(size, QIcon::Disabled)` is rendered
- **THEN** it uses the palette's disabled colour and differs from the
  `QIcon::Normal` pixmap

#### Scenario: The light family uses a dark tint [lia_light_family]

- **WHEN** an icon is rendered while the light family is applied
- **THEN** its opaque pixels use the light family's dark icon token, not white

#### Scenario: Brightness change updates icons [lia_theme_change]

- **WHEN** the brightness level is changed and the same icon rendered again
- **THEN** the second pixmap differs from the first
