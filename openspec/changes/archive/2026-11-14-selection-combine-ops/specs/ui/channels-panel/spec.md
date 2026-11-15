# Spec Delta

## MODIFIED Requirements

### Requirement: Load a channel as a selection

Ctrl/Cmd-clicking a Channels-panel row SHALL load that row as the selection with
New; adding Shift SHALL use Add; adding Alt SHALL use Subtract; adding Shift and
Alt SHALL use Intersect. The composite row SHALL load the luminance of the
visible channels (a hidden RGB channel contributing nothing), each colour row
SHALL load its own grayscale plane, and an alpha row SHALL load its stored
channel. Each load SHALL record exactly one "Load Selection" history state. A
plain click SHALL only move the current row and SHALL NOT change the selection.
Dragging a row onto the Load Selection button SHALL load it as a New selection.
Alt/Option-clicking the Save Selection button SHALL open the Save Selection
dialog instead of saving immediately; a plain click SHALL still save the
selection to a new channel. Alpha rows SHALL display the channel's stored name
when present and `Alpha N` otherwise; the Ctrl+number labels SHALL remain
positional.

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

#### Scenario: Ctrl+Shift adds to the selection [cp_ctrl_shift_add]

- **WHEN** an existing selection is present and a colour or alpha row is Ctrl+Shift-clicked
- **THEN** the row's coverage is added to the selection and one "Load Selection" state is recorded

#### Scenario: Ctrl+Alt subtracts from the selection [cp_ctrl_alt_subtract]

- **WHEN** an existing selection is present and a row is Ctrl+Alt-clicked
- **THEN** the row's coverage is subtracted from the selection and one "Load Selection" state is recorded

#### Scenario: Ctrl+Shift+Alt intersects the selection [cp_ctrl_shift_alt_intersect]

- **WHEN** an existing selection is present and a row is Ctrl+Shift+Alt-clicked
- **THEN** the selection becomes the intersection with the row's coverage and one "Load Selection" state is recorded

#### Scenario: Dragging a channel onto Load loads it [cp_drag_load]

- **WHEN** a channel row is dragged onto the Load Selection button
- **THEN** the row's coverage replaces the selection and one "Load Selection" state is recorded

#### Scenario: Alt-clicking Save opens the dialog [cp_alt_save_dialog]

- **WHEN** the Save Selection button is Alt-clicked
- **THEN** the Save Selection dialog opens and no channel is created until it is accepted

#### Scenario: Alpha rows show stored names [cp_alpha_name]

- **WHEN** an extra channel has a stored name
- **THEN** its row shows that name instead of `Alpha N` while its Ctrl+number label stays positional

#### Scenario: Each modifier click records one state [cp_modifier_one_state]

- **WHEN** a modifier click loads a row
- **THEN** the history gains exactly one state
