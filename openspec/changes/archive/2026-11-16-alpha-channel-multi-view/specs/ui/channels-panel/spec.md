## MODIFIED Requirements

### Requirement: Alpha channel visibility

Each alpha-channel row in the Channels panel SHALL have an enabled, checkable
eye. Clicking an alpha eye SHALL toggle that channel in the canvas view; multiple
alpha channels MAY be shown at once. The canvas view SHALL draw each shown
channel as a red 50 % rubylith overlay over the channel's masked (black) area,
composited so that each additional shown channel deepens the red, and SHALL
scale the overlay to the document while leaving the composite visible beneath it.
Clicking the composite eye or any colour-channel eye SHALL restore the composite
and clear every shown alpha channel. The alpha view SHALL be display-only: it
MUST NOT change the channel pixels, the live selection, the history, or the
saved document. Deleting a shown channel SHALL remove it from the view and shift
the remaining shown indices; switching documents or reducing the channel count
SHALL clear the affected entries.

#### Scenario: The eye shows the channel [cp_alpha_eye_shows]

- **WHEN** an alpha channel's eye is clicked
- **THEN** the canvas shows a red 50 % overlay over that channel's masked area and the eye is checked

#### Scenario: The eye hides the channel [cp_alpha_eye_toggles]

- **WHEN** the checked alpha eye is clicked again
- **THEN** that channel's overlay is removed and its eye is unchecked

#### Scenario: Multiple eyes deepen the red [cp_alpha_multiple]

- **WHEN** a second alpha eye is clicked while the first is shown
- **THEN** both eyes are checked and the canvas red is deeper where both channels are masked

#### Scenario: The composite eye restores the image [cp_alpha_composite_restores]

- **WHEN** alpha channels are shown and the composite eye is clicked
- **THEN** the canvas returns to the composite and all alpha views are cleared

#### Scenario: Deleting the shown channel clears the view [cp_alpha_delete_clears]

- **WHEN** a shown alpha channel is deleted
- **THEN** it is removed from the view and the remaining shown channels stay visible

#### Scenario: The view is display-only [cp_alpha_view_display_only]

- **WHEN** alpha channels are shown on the canvas
- **THEN** the channel pixels, live selection, and history are unchanged

#### Scenario: The eye also selects the row [cp_alpha_eye_selects]

- **WHEN** an alpha eye is clicked
- **THEN** its row becomes the current Channels row
