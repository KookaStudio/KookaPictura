# Spec Delta

## ADDED Requirements

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
border, scrollbar thumb hover, and separator light — SHALL stay distinct from
their base colour in each family, deriving toward that family's shaded
direction rather than clamping together.

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

### Requirement: Menu-bar chrome actions

The application SHALL show a theme toggle and GitHub and Discord link buttons at
the top-right of the menu-bar row. The theme button SHALL show a sun while the
dark family is active and a moon while the light family is active, and clicking
it SHALL switch the family. The link buttons SHALL open the project's Discord
invite and GitHub repository in the user's browser. All three SHALL draw from
the theme tokens.

#### Scenario: The theme button reflects and switches the family [ds_chrome_theme]

- **WHEN** the dark family is active
- **THEN** the theme button shows a sun; clicking it switches to the light
  family, and the button then shows a moon

#### Scenario: The links open the project pages [ds_chrome_links]

- **WHEN** the Discord or GitHub button is clicked
- **THEN** the project's Discord invite or GitHub repository URL opens in the
  default browser

## MODIFIED Requirements

### Requirement: Theme token contract

The theme SHALL expose a public accessor that returns the named tokens of a
family and a brightness level: the colour tokens the stylesheet substitutes,
including every derived shade, and the numeric metrics (panel border width,
chrome border width, group divider width, tool-slot footprint and icon maxima,
shade step). The global stylesheet builder SHALL consume the same accessor, so a
token value appears exactly once in the theme and the stylesheet, the dump, and
the widgets cannot disagree. Clamping a level SHALL use the family's level
range.

#### Scenario: Tokens cover colours and metrics [ds_tokens_shape]

- **WHEN** the token accessor is asked for any family and any valid brightness
  level
- **THEN** it returns the named colour tokens (including the derived ones the
  stylesheet uses, such as the alternate table background, menu disabled text,
  panel-header border, and scrollbar thumb hover) and the numeric metrics,
  each exactly once

#### Scenario: The stylesheet and the dump agree [ds_tokens_single_source]

- **WHEN** the stylesheet for a family and level is built and the same family
  and level are dumped
- **THEN** every colour token in the dump appears in the stylesheet with the
  same value, and no colour literal outside the token set is substituted into
  the stylesheet
