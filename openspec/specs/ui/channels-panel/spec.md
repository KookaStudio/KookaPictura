# ui/channels-panel Specification

## Purpose
The presentational CS6 Channels panel: a composite row and the three primary
colour rows, each with a grayscale thumbnail built from the existing composite
and a visibility toggle. It is presentational because the model has no
per-channel visibility and the compositor has no channel-mask path.

## Requirements

### Requirement: Channel rows

The `ChannelsPanel` SHALL list the composite and colour channels of the
document's mode — the opened file's mode when it was converted for editing —
with CS6's Ctrl+number labels (RGB: "RGB" Ctrl+2, "Red" Ctrl+3, "Green" Ctrl+4,
"Blue" Ctrl+5; Gray: "Gray" Ctrl+2; CMYK, Lab, Multichannel, Indexed, Bitmap,
and Duotone likewise), followed by one "Alpha N" row per document alpha channel.
When the active layer carries a raster layer mask, the panel SHALL also list a
`<layer name> Mask` row after the colour rows and before the alpha rows.
`refresh` SHALL keep the row set stable while the mode, alpha count, and active
layer's mask do not change.

#### Scenario: An RGB document shows four rows [cp_rows_four]

- **WHEN** an RGB document is open and the panel is refreshed
- **THEN** the panel reports four rows named "RGB", "Red", "Green", and "Blue", labelled Ctrl+2 to Ctrl+5

#### Scenario: The active layer's mask adds a row [cp_rows_mask]

- **WHEN** the active layer carries a raster layer mask
- **THEN** the panel reports an additional row named "`<layer name>` Mask" after
  "Blue"

#### Scenario: A layer change updates the mask row [cp_rows_mask_change]

- **WHEN** the active layer changes to one that carries no raster layer mask
- **THEN** the mask row is removed

#### Scenario: Refresh keeps the rows [cp_rows_stable]

- **WHEN** `refresh` runs again with the active layer unchanged
- **THEN** the row count is unchanged

### Requirement: Channel thumbnails

The `ChannelsPanel` SHALL build a grayscale thumbnail for each row from the
coarsest display-pyramid level whose longest side is at least 64 px, falling
back to `view_->image()` when no such level exists (a single-level or tiny
document): the composite row SHALL use luminance (`qGray`), the colour rows
SHALL use `qRed`, `qGreen`, and `qBlue` respectively, and the mask row SHALL use
the mask's grayscale coverage, each scaled to a small pixmap. With no document,
the thumbnails SHALL be empty.

#### Scenario: Every row has a thumbnail [cp_thumbnail_present]

- **WHEN** an RGB document is open and the panel is refreshed
- **THEN** every row's thumbnail pixmap is non-null

#### Scenario: The mask row shows the mask coverage [cp_thumbnail_mask]

- **WHEN** the active layer's mask is black in one half and white in the other
- **THEN** the mask row's thumbnail shows that coverage

### Requirement: Channels panel affordances

A channel item SHALL show a pointing-hand cursor while hovered. Holding Ctrl
while over a channel item SHALL show a dashed-square symbol at the cursor,
indicating "select all pixels of this channel". The channel table rows SHALL use
the same surface and item styling as the Layers panel, including the
family-aware row, gutter, and selected-row steps, and SHALL re-style when the
theme family changes so the selection stays distinct in either family.

#### Scenario: Hover shows the hand cursor [uic_channel_hover_cursor]

- **WHEN** the pointer hovers a channel item
- **THEN** the cursor is the pointing hand

#### Scenario: Ctrl shows the select-all symbol [uic_channel_ctrl_symbol]

- **WHEN** Ctrl is held while the pointer is over a channel item
- **THEN** a dashed-square symbol is drawn at the pointer

#### Scenario: Rows match the Layers panel style [uic_channel_row_style]

- **WHEN** the Channels panel is shown
- **THEN** its rows use the same surface shading and item layout as the Layers
  panel

#### Scenario: Channel rows follow the light family [cp_light_row_contrast]

- **WHEN** the light family is applied with a channel row selected
- **THEN** the selected row surface is darker than the unselected rows and the
  channel name stays legible

### Requirement: Mask channel row

When the active layer carries a raster layer mask, the Channels panel SHALL list
that mask as an additional channel after the colour rows, named
`<layer name> Mask`, carrying a `Ctrl+\` shortcut. Selecting the row, or pressing
`Ctrl+\`, SHALL show the mask on the canvas through the view-only mask solo view
(`ui/mask-solo-view`): it SHALL NOT change the document, record history, or alter
the composite, any other channel's thumbnail, or any save/export. The row SHALL
be absent when the active layer has no raster layer mask. The colour rows'
thumbnails SHALL keep reading the document composite while the mask channel is
shown.

#### Scenario: Ctrl+\ shows the mask [cp_mask_shortcut]

- **WHEN** the active layer has a raster mask and `Ctrl+\` is pressed
- **THEN** the canvas shows the mask plane, and the colour-row thumbnails still
  show the composite

#### Scenario: No mask, no row [cp_mask_row_absent]

- **WHEN** the active layer has no raster layer mask
- **THEN** the Channels panel lists no mask row

### Requirement: Channel visibility on the canvas

The eyes of the working RGB channels SHALL hide those channels on the canvas
live: with one hidden the other two SHALL show in colour, with two hidden the
remaining one SHALL show as greyscale, the last visible channel SHALL NOT be
hidden, and the composite eye SHALL be on only while all are shown and SHALL
show them all. Another document SHALL open with every channel shown.

#### Scenario: Eyes change the canvas [cp_live_visibility]

- **WHEN** the `tst_channels_panel` test hides Red, then Green, tries to hide Blue, clicks the composite eye, and switches documents, reading the canvas pixel each time
- **THEN** a (100, 150, 200) image shows (0, 150, 200), then grey 200, Blue stays visible, the composite restores (100, 150, 200), and the other document shows all channels

### Requirement: Alpha channels and the footer

The panel SHALL create a black alpha channel (one "New Channel" state), delete
the selected alpha channel (one "Delete Channel" state), save the selection as
a new alpha channel, and load the selected alpha channel as the selection;
alpha rows SHALL continue the Ctrl+number labels. When the document holds 56
channels, the New Channel button SHALL be disabled with a reason. Save
Selection SHALL refuse with a clear non-modal status-bar message rather
than silently doing nothing when the channel budget is full.

#### Scenario: New, delete, save, load [cp_alpha_footer]

- **WHEN** the `tst_channels_panel` test creates and deletes a channel, saves a rectangle selection as a channel, deselects, selects that channel, and loads it
- **THEN** "Alpha 1" appears on Ctrl+6 and is removed with the matching states, the saved channel appears, and loading restores the rectangle as the selection

#### Scenario: The cap disables and refuses [cp_channel_cap]

- **WHEN** the document holds 56 channels
- **THEN** New Channel is disabled with a reason, and Save Selection shows the limit in the status bar without adding a channel

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

### Requirement: Mask thumbnail is bounded by its output

The Channels panel SHALL build the active layer's mask-row thumbnail in time
bounded by the thumbnail's own pixel count, not by the document's pixel count:
it SHALL sample the mask at the thumbnail grid rather than materialising a
document-sized plane and downsampling it. The thumbnail's pixels SHALL be
unchanged by this: each output pixel SHALL be the mask's coverage at that
pixel's nearest-neighbour source position over the document, using the same
off-rect `default_color` and data-less white as the mask plane, so the mask row
stays correct while it is refreshed after every edit.

#### Scenario: The thumbnail does not scan the document [cp_mask_thumb_bounded]

- **WHEN** the Channels panel builds the mask row's thumbnail for a large document
- **THEN** the mask is sampled once per thumbnail pixel, not once per document
  pixel

#### Scenario: The thumbnail pixels are unchanged [cp_mask_thumb_unchanged]

- **WHEN** the mask thumbnail is built for a document larger than the mask's own
  rectangle
- **THEN** every thumbnail pixel equals the nearest-neighbour sample of the
  mask's coverage over the document

### Requirement: Temporary Quick Mask channel row

While the active document is in Quick Mask, the Channels panel SHALL append one
temporary row named `Quick Mask` after its durable channel rows. Its thumbnail
SHALL show the current grayscale mask and update after mask edits, while the
composite and color rows SHALL continue to show document pixels. The row SHALL
be removed on exit, SHALL NOT consume a durable alpha channel slot, and SHALL
NOT be renamed, deleted, reordered, or passed to durable alpha-channel commands.
Selecting the Quick Mask row SHALL show its grayscale view. Selecting an
ordinary channel SHALL leave Quick Mask through the normal selection-conversion
path before applying that row's ordinary behavior.

#### Scenario: The temporary row follows mode and edits [cp_quick_mask_row]
- **WHEN** Quick Mask is entered, painted, and exited
- **THEN** exactly one temporary row appears, its thumbnail follows the edited mask, durable channel rows and their thumbnails remain intact, and the temporary row disappears on exit

#### Scenario: Temporary row rejects durable-channel operations [cp_quick_mask_protected]
- **WHEN** the Quick Mask row is current and rename, delete, reorder, or a durable alpha-channel action is attempted
- **THEN** the operation is unavailable or refused and neither coverage nor durable channels change

#### Scenario: Channel navigation leaves Quick Mask cleanly [cp_quick_mask_navigation]
- **WHEN** the Quick Mask row is selected and then an ordinary color or alpha row is selected
- **THEN** the first selection shows grayscale mask coverage and the second exits Quick Mask with its edited coverage preserved as the pixel selection before normal row behavior occurs

### Requirement: Duplicate Channel command

The Channels panel menu SHALL offer `Duplicate Channel…` and SHALL enable it
only when the current row is a colour or alpha channel row of an open document.
The command SHALL open a dialog with a Name field, a Destination list, and an
Invert checkbox. The Destination list SHALL contain every open document with the
same pixel dimensions as the source, including the source, followed by a `New`
entry. The default name SHALL be `<source channel name> copy`, and the default
destination SHALL be the active document.

Accepting the dialog SHALL add one extra channel to the destination with the
source coverage, the requested name, and `255 - coverage` when Invert is
checked. A same-document or same-size cross-document duplicate SHALL record
exactly one `Duplicate Channel` history state in the destination and SHALL NOT
record history in the source. The `New` destination SHALL create a grayscale
document holding the coverage as its extra channel. A destination at the
56-channel limit SHALL refuse without adding a channel or recording history.

#### Scenario: The menu opens the dialog [cp_duplicate_menu]

- **WHEN** an alpha channel row is current and `Duplicate Channel…` is invoked
- **THEN** the dialog lists the same-size documents and `New`, defaults the name
  to `<source> copy`, and defaults the destination to the active document

#### Scenario: A colour row duplicates [cp_duplicate_color_row]

- **WHEN** the current row is a colour row and the dialog is accepted
- **THEN** the new extra channel holds that row's grayscale plane

#### Scenario: Duplicate within the document [cp_duplicate_same_doc]

- **WHEN** the dialog is accepted with the source document as the destination
- **THEN** a new channel with the requested name holds the source coverage and
  the document gains exactly one `Duplicate Channel` state

#### Scenario: Duplicate into another document [cp_duplicate_cross_doc]

- **WHEN** the dialog is accepted with another same-size document
- **THEN** the target gains the named channel and one `Duplicate Channel` state,
  and the source document's channel count and history are unchanged

#### Scenario: Duplicate into a new document [cp_duplicate_new_doc]

- **WHEN** the dialog is accepted with `New`
- **THEN** a grayscale document opens with the coverage as its named extra
  channel

#### Scenario: Invert complements the channel [cp_duplicate_invert]

- **WHEN** the dialog is accepted with Invert checked
- **THEN** the new channel holds `255 - source` for every pixel

#### Scenario: Dragging onto New Channel duplicates [cp_duplicate_button_drag]

- **WHEN** a colour or alpha row is dropped on the Create New Channel button
- **THEN** the current document gains one named copy of that row and exactly one
  `Duplicate Channel` state

#### Scenario: Dragging onto another document duplicates [cp_duplicate_document_drag]

- **WHEN** a colour or alpha row is dropped on another document's tab or canvas
- **THEN** the target gains one named copy of the row and exactly one
  `Duplicate Channel` state, the source is unchanged, and a different target
  size scales the coverage to fit and centre it

#### Scenario: The channel cap refuses [cp_duplicate_cap]

- **WHEN** the destination already holds 56 channels
- **THEN** the duplicate is refused with no new channel and no history state

### Requirement: Cross-document channel drag

Dragging a colour or alpha channel row SHALL carry the source document, row,
alpha index, visible-channel mask, and display name. Dropping it on another
document's tab or canvas SHALL duplicate that row into the target without
requiring equal dimensions. The target coverage SHALL be the source row coverage
scaled to fit and centred when the sizes differ, and exact when they match. The
source channel SHALL remain unchanged. Dropping on the source document or
outside a document SHALL do nothing.

#### Scenario: A same-size channel drag is exact [cp_channel_drag_same_size]

- **WHEN** a channel row is dragged onto a same-size document
- **THEN** the target channel bytes equal the source row bytes

#### Scenario: A different-size channel drag fits [cp_channel_drag_fit]

- **WHEN** a channel row is dragged onto a different-size document
- **THEN** the target channel holds the source coverage scaled to fit and
  centred, with the uncovered border empty

#### Scenario: The source channel is unchanged [cp_channel_drag_source]

- **WHEN** a channel row is dragged to another document
- **THEN** the source channel count, channel bytes, and history are unchanged

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
