## ADDED Requirements

### Requirement: Settings menu

The Crop options bar SHALL carry a Settings (cog) menu button listing Use
Classic Mode (`P`), Show Cropped Area (`H`), Auto Center Preview, Enable Crop
Shield, a Color combo, an Opacity combo and Auto Adjust Opacity. The menu SHALL
be compact: no empty rows, the shield controls grouped and indented under
Enable Crop Shield, and the shortcut letters in a right-hand column.

#### Scenario: The menu lists its items [cs_menu_items]

- **WHEN** the Crop tool is active and the cog button is opened
- **THEN** the menu shows Use Classic Mode, Show Cropped Area, Auto Center Preview, Enable Crop Shield, Color, Opacity and Auto Adjust Opacity

### Requirement: Classic mode gates the Modern settings

While Classic mode is on, Show Cropped Area and Auto Center Preview SHALL be
disabled and shown unchecked. Turning Classic mode off SHALL check both. In
Classic mode a drag inside the box SHALL move the box over a fixed image.

#### Scenario: Classic mode disables two items [cs_classic_gates]

- **WHEN** Classic mode is switched on, and then off
- **THEN** Show Cropped Area and Auto Center Preview are disabled and unchecked while it is on, and enabled and checked once it is off

#### Scenario: A Classic drag moves the box [cs_classic_drag]

- **WHEN** Classic mode is on and a drag starts inside the box
- **THEN** the box moves and the image does not

### Requirement: Settings keys

While the Crop tool is active, `P` SHALL toggle Classic mode and `H` SHALL
toggle Show Cropped Area when Classic mode is off. The keys SHALL NOT switch to
the Pen or Hand tool.

#### Scenario: P and H toggle their settings [cs_keys]

- **WHEN** the Crop tool is active and `P` is pressed, then `H`
- **THEN** Classic mode flips, and Show Cropped Area flips only while Classic mode is off, and the active tool stays Crop

### Requirement: Show Cropped Area

With Classic mode off and Show Cropped Area off, the area outside the crop box
SHALL be hidden rather than dimmed.

#### Scenario: The outside area is hidden [cs_show_area_off]

- **WHEN** Classic mode is off, Show Cropped Area is off, and a crop box is shown
- **THEN** the canvas outside the box is painted opaque in the shield colour

### Requirement: Crop shield

Enable Crop Shield SHALL toggle the tint over the area outside the box. Color
SHALL be Match Canvas, which uses the canvas colour, or Custom, which opens a
colour picker and then uses the chosen colour. Opacity SHALL accept 0 to 100 %
and default to 75 %. With Auto Adjust Opacity on, the shield SHALL thin to half
its opacity while the box is being dragged or rotated.

#### Scenario: The shield can be turned off [cs_shield_off]

- **WHEN** Enable Crop Shield is off and the outside area is shown
- **THEN** the area outside the box is not dimmed

#### Scenario: Opacity sets the tint [cs_shield_opacity]

- **WHEN** Opacity is set to 40 %
- **THEN** the shield paints with 40 % alpha, and values outside 0 to 100 are clamped

#### Scenario: A custom colour tints the shield [cs_shield_color]

- **WHEN** Color is Custom with red chosen
- **THEN** the area outside the box is tinted red, and Match Canvas restores the canvas colour

#### Scenario: Auto Adjust Opacity thins a drag [cs_auto_adjust]

- **WHEN** Auto Adjust Opacity is on and a box drag is in progress
- **THEN** the shield alpha is half its resting value, and returns to it on release

### Requirement: Auto Center Preview

With Classic mode off and Auto Center Preview on, the view SHALL pan so the
crop box is centred in the viewport once a box gesture ends.

#### Scenario: The box is centred after a gesture [cs_auto_center]

- **WHEN** Classic mode is off, Auto Center Preview is on, and a box is drawn away from the view centre
- **THEN** after release the box centre is at the viewport centre

### Requirement: Settings persist

The Settings menu's values SHALL persist across restarts.

#### Scenario: Settings survive a restart [cs_persist]

- **WHEN** the shield opacity, colour and toggles are changed
- **THEN** the session store holds them and a fresh tool controller restores them
