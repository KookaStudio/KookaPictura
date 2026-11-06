# Spec Delta

## ADDED Requirements

### Requirement: Adjustments panel surface

The Adjustments panel, its scroll viewport, and its grid host SHALL paint the
same `${surface}` background as the other panel surfaces, using the Styles
panel as the reference, instead of the darker `QPalette::Base` that a
`QScrollArea` viewport defaults to. The background SHALL follow the active theme
family and brightness level.

#### Scenario: Adjustments surface matches the other panels [adj_panel_surface]

- **WHEN** the Adjustments panel is displayed in a panel column beside another panel
- **THEN** the panel, its scroll viewport, and its grid host render the same background colour as that panel's surface

#### Scenario: The surface follows the theme [adj_panel_surface_theme]

- **WHEN** the theme family or brightness level changes while the Adjustments panel is displayed
- **THEN** the panel, its scroll viewport, and its grid host repaint with the new `${surface}` colour
