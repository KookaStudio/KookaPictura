# Spec Delta

## ADDED Requirements

### Requirement: The app can create a solid color fill layer

The app SHALL create a solid-color fill layer carrying the current foreground
colour, and `Layer > New Fill Layer > Solid Color…` and the Layers-panel fill
menu SHALL both offer an enabled `Solid Color…` entry that creates one. The
colour SHALL be read from the app's shared foreground colour state when the
command runs, so a colour chosen in the toolbox, the Color panel, the Swatches
panel, or the Eyedropper is the one the new layer is filled with, and both
creation surfaces create the layer in that same colour. The fill SHALL take
the foreground and never the background, so the default pair fills black and
not the background's white. The `SoCo` descriptor carries no alpha, so the new
layer SHALL be opaque whatever the foreground's alpha is.

#### Scenario: The panel fill menu uses the foreground [scf_panel_foreground]

- **WHEN** a non-black foreground colour is set and `Solid Color…` is chosen
  from the Layers panel's New Fill / Adjustment strip menu
- **THEN** the new solid-color fill layer composites to that foreground colour

#### Scenario: The Layer menu fill command uses the foreground [scf_menu_foreground]

- **WHEN** a non-black foreground colour is set and `Layer > New Fill Layer >
  Solid Color…` is run
- **THEN** the new solid-color fill layer composites to that foreground colour

#### Scenario: The default foreground still fills black [scf_default_black]

- **WHEN** the foreground colour is the default black and the background colour
  is the default white, and `Solid Color…` is chosen
- **THEN** the new solid-color fill layer is opaque black, not the background
  white
