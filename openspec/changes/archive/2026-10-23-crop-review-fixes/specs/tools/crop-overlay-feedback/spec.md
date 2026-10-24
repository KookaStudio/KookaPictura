## ADDED Requirements

### Requirement: Overlay show mode and cycle commands

The grid menu's Auto, Always and Never Show Overlay SHALL set when the guide
overlay is drawn on the crop box. Auto, the default, SHALL draw it only while
the box is being adjusted. Always SHALL draw it whenever the box is shown, and
Never SHALL not draw it. Cycle Overlay and `O` SHALL step to the next overlay,
wrapping. Cycle Orientation and `Shift+O` SHALL step the orientation of the
Triangle and Golden Spiral overlays, and the menu entry SHALL be enabled only
for them.

#### Scenario: Show modes gate the guides [cof_show_mode]

- **WHEN** the show mode is Auto, then Always, then Never, and the box is at rest and then dragged
- **THEN** Auto draws the guides only during the drag, Always draws them at rest, and Never draws them during a drag neither

#### Scenario: O and Shift+O cycle the overlay [cof_cycle_keys]

- **WHEN** the Crop tool is active and `O`, then `Shift+O` on Triangle, is typed into the window
- **THEN** the overlay steps to the next one, the Triangle orientation steps once, and the active tool stays Crop
