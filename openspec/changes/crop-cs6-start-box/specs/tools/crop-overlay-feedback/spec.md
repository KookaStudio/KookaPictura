## ADDED Requirements

### Requirement: Rotate grid and angle readout

While the crop box is being rotated, the box SHALL show a dense grid of evenly
spaced lines (cells about 45 screen pixels) in place of the selected guide
overlay, whatever overlay is chosen, and a readout beside the pointer SHALL show
the current angle as `∠: <degrees>°` with one decimal. Both SHALL disappear when
the rotation is released, cancelled, or the tool is deactivated.

#### Scenario: Rotating shows the grid and the angle

- **WHEN** the crop box is dragged in its rotate band with any overlay chosen
- **THEN** the box shows the rotate grid and the pointer readout shows the angle, e.g. `∠: -14.1°`

#### Scenario: Release clears the feedback

- **WHEN** the rotate drag is released
- **THEN** the rotate grid and the angle readout are gone and the angle is kept

### Requirement: Overlay menu icons

Each guide overlay in the Crop options bar's grid menu (Rule of Thirds, Grid,
Diagonal, Triangle, Golden Ratio, Golden Spiral) SHALL carry its own icon
depicting that overlay, and the menu button SHALL show the icon of the selected
overlay.

#### Scenario: The button follows the selection

- **WHEN** an overlay is chosen in the grid menu
- **THEN** the menu entry shows that overlay's icon and the menu button shows the same icon
