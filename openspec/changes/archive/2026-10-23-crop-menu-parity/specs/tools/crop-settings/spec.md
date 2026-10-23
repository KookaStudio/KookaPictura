## MODIFIED Requirements

### Requirement: Settings menu

The Crop options bar SHALL carry a Settings (cog) menu button. Its menu SHALL
look like the guide-overlay menu: checkable items with the same check marks,
padding and separators, with the keys `P` and `H` in the shortcut column. It
SHALL list Use Classic Mode, Show Cropped Area, Auto Center Preview, Enable
Crop Shield, a Color row (a combo and a swatch), an Opacity row (the shared
numeric field with a slider popup) and Auto Adjust Opacity, the last three
indented under Enable Crop Shield.

#### Scenario: The menu lists its items [cs_menu_items]

- **WHEN** the Crop tool is active and the cog button is opened
- **THEN** the menu shows the checkable items, the Color combo, and the Opacity slider field at 75 %, with `P` and `H` in the shortcut column
