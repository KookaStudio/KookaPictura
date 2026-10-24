## MODIFIED Requirements

### Requirement: Settings menu

The Crop options bar SHALL carry a Settings (cog) menu button. Its menu SHALL
be a panel of checkboxes: Use Classic Mode and Show Cropped Area with their keys
`P` and `H` in a right-hand column, Auto Center Preview, and Enable Crop Shield
with Color (a combo and a swatch), Opacity (the shared numeric field with a
slider popup) and Auto Adjust Opacity indented beneath it.

#### Scenario: The menu lists its items [cs_menu_items]

- **WHEN** the Crop tool is active and the cog button is opened
- **THEN** the menu shows the checkboxes, the Color combo, and the Opacity slider field at 75 %, with `P` and `H` in the key column
