## ADDED Requirements

### Requirement: Color panel swatches open the picker

Clicking the Color panel's foreground or background swatch SHALL make that
swatch active and open the Color Picker with its current colour. Accepting the
picker SHALL set the active swatch to the chosen colour; cancelling SHALL leave
the colour unchanged.

#### Scenario: Color panel swatch opens the picker

- **WHEN** the user clicks the Color panel's foreground swatch and accepts a new
  colour in the picker
- **THEN** the picker was opened for the foreground and the shared foreground
  colour becomes the accepted colour
