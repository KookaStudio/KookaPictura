# Spec Delta — ui/design-system

## MODIFIED Requirements

### Requirement: Light theme family

The theme SHALL provide a light family in addition to the dark family. Both
families SHALL share the same token keys, and the light family's values SHALL
mirror the dark family role-for-role: light surfaces (window, panel, workspace,
headers, inputs, buttons) and dark text, icons, and borders. The light family
SHALL expose the same number of brightness levels as the dark family. Switching
family SHALL re-apply the palette and the stylesheet and SHALL invalidate the
icon caches so icons and custom-painted widgets follow. The chosen family SHALL
persist across restarts, and a session with no stored family SHALL load the
dark family. The dark family's default-level values SHALL be unchanged. Derived
shades — the alternate table background, menu disabled text, panel-header
border, scrollbar thumb hover, separator light, and the custom-painted row,
gutter, and selected-row surfaces — SHALL stay distinct from their base colour
in each family, deriving toward that family's shaded direction rather than
clamping together. A custom-painted surface that is stepped away from the window
SHALL move toward contrast: lighter on the dark family and darker on the light
family, so a row's neutral selection stays distinct from its row and the
family's selected text stays legible.

#### Scenario: Light roles mirror the dark roles [ds_light_roles_mirror]

- **WHEN** the light family is applied at its default level
- **THEN** every role key resolves to a value, each light surface token is
  lighter than its matching text/icon token, and the dark family's default-level
  colour tokens are unchanged

#### Scenario: Derived shades stay distinct [ds_light_derived_distinct]

- **WHEN** the derived shade tokens are resolved in either family
- **THEN** each derived shade differs from its base token, so alternate table
  rows, disabled menu text, and hover outlines remain distinguishable

#### Scenario: Switching family persists and follows into paint [ds_light_switch_persists]

- **WHEN** the user switches the theme family and restarts the application
- **THEN** the chosen family is restored, and icons and custom-painted widgets
  render with the restored family's colours

#### Scenario: Row and tip surfaces step toward family contrast [ds_light_row_contrast]

- **WHEN** a layer row, a channel row, or a brush tip preset is painted under the
  light family
- **THEN** its row, gutter, selection, and glyph steps move away from the light
  window instead of clamping to white, and the selected text and tip glyph
  remain legible
