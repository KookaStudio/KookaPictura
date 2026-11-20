# Spec Delta

## ADDED Requirements

### Requirement: Channel Options command

The Channels panel SHALL enable its menu entry `Channel Options…` only while the current row is an alpha channel row of an open document. It SHALL be disabled when the current row is a colour row, the active layer's mask row, the temporary Quick Mask row, a spot channel row, or when no row is current.

Invoking the command, or double-clicking an alpha channel row, SHALL open a modal dialog titled `Channel Options` whose `Color Indicates:` choice offers `Masked Areas` and `Selected Areas`, initialized from the channel's current sense. Accepting a changed sense SHALL store the new sense, flip the channel plane so the selection it loads is unchanged, refresh the channel's thumbnail and the canvas alpha view, and record exactly one `Channel Options` history state. Accepting the same sense, or cancelling, SHALL leave the document, the channel bytes, the selection, and history unchanged. The dialog SHALL NOT offer a name, colour, opacity, or spot conversion in this change, and it SHALL NOT be available for non-alpha rows.

#### Scenario: The menu entry follows the current row [cp_options_menu_row]
- **WHEN** the current row is changed between an alpha row, a colour row, the layer-mask row, the Quick Mask row, and a spot channel row
- **THEN** `Channel Options…` is enabled only for the alpha row

#### Scenario: Double-click opens the dialog [cp_options_double_click]
- **WHEN** an alpha channel row is double-clicked
- **THEN** the Channel Options dialog opens with the channel's current sense selected

#### Scenario: Cancelling changes nothing [cp_options_cancel]
- **WHEN** the dialog is opened and cancelled
- **THEN** the channel plane, sense, selection, and history are unchanged

#### Scenario: Re-applying the same sense records no state [cp_options_same_sense]
- **WHEN** the dialog is accepted without changing the current sense
- **THEN** the channel and history are unchanged

#### Scenario: Accepting flips the plane and keeps the mask [cp_options_apply]
- **WHEN** a Masked Areas channel is switched to Selected Areas and accepted
- **THEN** every plane byte is complemented, the loaded selection is unchanged, and history gains exactly one `Channel Options` state

### Requirement: Sense-aware channel display

The Channels panel SHALL render each alpha row's thumbnail from the stored channel plane, so changing a channel's sense inverts its thumbnail while the selection the channel loads stays the same. While an alpha channel is shown on the canvas, its rubylith SHALL continue to cover the stored plane's black area: under `Masked Areas` that is the masked area, and under `Selected Areas` that is the selected area. An accepted change SHALL refresh both the thumbnail and the shown overlay without another eye click. The alpha view SHALL remain display-only, leaving channel pixels, the live selection, history, and the saved document unchanged.

#### Scenario: The thumbnail follows the plane [cp_options_thumbnail]
- **WHEN** an alpha channel's sense is switched from Masked Areas to Selected Areas
- **THEN** the row thumbnail shows the complemented plane

#### Scenario: The overlay follows the plane's black area [cp_options_overlay]
- **WHEN** a Selected Areas alpha channel is shown on the canvas
- **THEN** the red overlay covers the pixels the channel stores as black, which are its selected pixels

#### Scenario: Toggling a shown channel refreshes its overlay [cp_options_overlay_refresh]
- **WHEN** the sense of an alpha channel currently shown on the canvas is changed
- **THEN** the rubylith follows the complemented plane without another eye click

## MODIFIED Requirements

### Requirement: Load a channel as a selection

Ctrl/Cmd-clicking a Channels-panel row SHALL load that row as the selection with
New; adding Shift SHALL use Add; adding Alt SHALL use Subtract; adding Shift and
Alt SHALL use Intersect. The composite row SHALL load the luminance of the
visible channels (a hidden RGB channel contributing nothing), each colour row
SHALL load its own grayscale plane, and an alpha row SHALL load its channel through its
sense: the stored plane under `Masked Areas` and `255 - plane` under
`Selected Areas`. Each load SHALL record exactly one "Load Selection" history
state. A plain click SHALL only move the current row and SHALL NOT change the selection.
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

### Requirement: Alpha channel visibility

Each alpha-channel row in the Channels panel SHALL have an enabled, checkable
eye. Clicking an alpha eye SHALL toggle that channel in the canvas view; multiple
alpha channels MAY be shown at once. The canvas view SHALL draw each shown
channel as a red 50 % rubylith overlay over the channel's stored plane's black
area (masked under `Masked Areas`, selected under `Selected Areas`), composited
so that each additional shown channel deepens the red, and SHALL scale the
overlay to the document while leaving the composite visible beneath it.
Clicking the composite eye or any colour-channel eye SHALL restore the composite
and clear every shown alpha channel. The alpha view SHALL be display-only: it
MUST NOT change the channel pixels, the live selection, the history, or the
saved document. Deleting a shown channel SHALL remove it from the view and shift
the remaining shown indices; switching documents or reducing the channel count
SHALL clear the affected entries.

#### Scenario: The eye shows the channel [cp_alpha_eye_shows]

- **WHEN** an alpha channel's eye is clicked
- **THEN** the canvas shows a red 50 % overlay over that channel's plane-black area (its masked area under `Masked Areas`;
  its selected area under `Selected Areas`) and the eye is checked

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
