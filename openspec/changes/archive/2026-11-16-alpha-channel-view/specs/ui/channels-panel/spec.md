## ADDED Requirements

### Requirement: Alpha channel visibility

Each alpha-channel row in the Channels panel SHALL have an enabled, checkable
eye. Clicking an alpha eye SHALL show that channel on the canvas as a red 50 %
rubylith overlay over its masked (black) area, scaled to the document; clicking
it again SHALL restore the composite. Clicking the composite eye or any
colour-channel eye SHALL also restore the composite and clear the alpha view.
The alpha view SHALL be display-only: it MUST NOT change the channel pixels, the
live selection, the history, or the saved document. Deleting the shown channel,
switching documents, or reducing the channel count below the shown index SHALL
clear the view. At most one alpha channel SHALL be shown at a time.

#### Scenario: The eye shows the channel [cp_alpha_eye_shows]

- **WHEN** an alpha channel's eye is clicked
- **THEN** the canvas shows a red 50 % overlay over that channel's masked area and the eye is checked

#### Scenario: The eye hides the channel [cp_alpha_eye_toggles]

- **WHEN** the checked alpha eye is clicked again
- **THEN** the canvas returns to the composite and the eye is unchecked

#### Scenario: The composite eye restores the image [cp_alpha_composite_restores]

- **WHEN** an alpha channel is shown and the composite eye is clicked
- **THEN** the canvas returns to the composite and the alpha view is cleared

#### Scenario: Deleting the shown channel clears the view [cp_alpha_delete_clears]

- **WHEN** the alpha channel currently shown is deleted
- **THEN** the canvas returns to the composite and the panel reports no shown alpha

#### Scenario: The view is display-only [cp_alpha_view_display_only]

- **WHEN** an alpha channel is shown on the canvas
- **THEN** the channel pixels, live selection, and history are unchanged

#### Scenario: The eye also selects the row [cp_alpha_eye_selects]

- **WHEN** an alpha eye is clicked
- **THEN** its row becomes the current Channels row
