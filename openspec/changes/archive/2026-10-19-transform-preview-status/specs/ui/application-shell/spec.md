## MODIFIED Requirements

### Requirement: Status bar readouts

The system SHALL show the active document's magnification and file size in a
status bar, along with a tool-hint field, and SHALL expose a view-options popup
from the status bar for selecting which readout is displayed. When no document
is open the status bar SHALL show no readouts and no tool hint — only its
background — and SHALL show them again once a document opens. The status bar
SHALL use a small left padding and SHALL NOT draw the right-side size-grip
corner triangle. The status-bar magnification readout and any zoom control SHALL
show the same value in the same format. While a Free Transform, Skew, Distort,
or Perspective session is active the status bar SHALL also show an action
readout naming it (`Free Transform in progress`, `Skew in progress`, `Distort in
progress`, or `Perspective in progress`); the readout SHALL clear when the
session commits or cancels.

#### Scenario: Magnification and size shown

- **WHEN** a document is displayed
- **THEN** the status bar shows the current magnification and document size

#### Scenario: View options popup

- **WHEN** the status-bar options control is activated
- **THEN** a popup lists the selectable readouts and the chosen readout is displayed

#### Scenario: No document shows an empty status bar [las_status_empty]

- **WHEN** no document is open
- **THEN** the status bar shows no magnification, size, or tool-hint text, only its background, and restores them when a document opens

#### Scenario: No corner size grip [las_status_no_grip]

- **WHEN** the status bar is shown
- **THEN** it draws no right-side size-grip corner triangle and uses a small left padding

#### Scenario: A transform session names its action [las_status_action]

- **WHEN** `Edit > Transform > Skew` begins a session
- **THEN** the status bar shows `Skew in progress` until the session commits or cancels
