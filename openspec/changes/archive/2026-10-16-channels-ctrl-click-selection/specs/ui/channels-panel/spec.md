# Spec Delta

## ADDED Requirements

### Requirement: Load a channel as a selection

Ctrl-clicking a Channels-panel row SHALL load that row as the selection: the
composite row SHALL load the luminance of the visible channels (a hidden RGB
channel contributing nothing), each colour row SHALL load its own grayscale
plane, and an alpha row SHALL load its stored channel. The load SHALL be
recorded as one "Load Selection" history state. A plain click SHALL only move
the current row and SHALL NOT change the selection.

#### Scenario: A colour row loads its plane [cp_ctrl_click_channel]

- **WHEN** the `tst_channels_panel` test Ctrl-clicks the Red, Green, Blue, and
  composite rows of a flat (100, 150, 200) image
- **THEN** the selection coverage is 100, 150, 200, and 140 respectively

#### Scenario: The composite follows the visible channels [cp_ctrl_click_visible]

- **WHEN** the Red channel is hidden and the composite row is Ctrl-clicked
- **THEN** the selection coverage is the luminance of (0, 150, 200), i.e. 106

#### Scenario: A plain click does not load [cp_ctrl_click_plain]

- **WHEN** a row is clicked without Ctrl
- **THEN** the current row moves and the selection is unchanged
