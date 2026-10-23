## MODIFIED Requirements

### Requirement: Settings menu

The Crop options bar SHALL carry a Settings (cog) menu button. Its menu SHALL be
a panel of checkboxes: Use Classic Mode and Show Cropped Area with their keys
`P` and `H` in a right-hand column, Auto Center Preview, and Enable Crop Shield
with Color (a combo and a swatch), Opacity (the shared numeric field with a
slider popup) and Auto Adjust Opacity indented beneath it.

#### Scenario: The menu lists its items [cs_menu_items]

- **WHEN** the Crop tool is active and the cog button is opened
- **THEN** the menu shows the checkboxes, the Color combo, and the Opacity slider field at 75 %, with `P` and `H` in the key column

## ADDED Requirements

### Requirement: Classic mode turns the box

In Classic mode a rotation SHALL turn the crop box over the fixed image, with
its handles, guides and shield following it, and pointer hit-testing SHALL use
the turned box. Dragging around the box clockwise SHALL turn it clockwise. The
committed crop SHALL equal the content under the turned box. In Modern mode
the content SHALL keep turning behind a fixed box.

#### Scenario: A turned box is dragged on screen [cs_classic_turn_drag]

- **WHEN** Classic mode is on, a box is turned a quarter turn, and its body is dragged down on screen
- **THEN** the box moves down on screen and the image does not move

#### Scenario: The rotate gesture follows the pointer [cs_classic_turn_gesture]

- **WHEN** Classic mode is on and the pointer sweeps a quarter turn clockwise around the box
- **THEN** the box turns a quarter turn clockwise, and in Modern mode the content turns the same way

### Requirement: Shield colour picker

Choosing Custom for the shield colour, or clicking its swatch, SHALL open the
project's Color Picker dialog.

#### Scenario: Custom opens the project picker [cs_picker]

- **WHEN** Custom is chosen in the Color combo
- **THEN** the Color Picker dialog opens with the current shield colour
