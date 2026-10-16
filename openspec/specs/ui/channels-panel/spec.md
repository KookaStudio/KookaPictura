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
the same surface and item styling as the Layers panel.

#### Scenario: Hover shows the hand cursor [uic_channel_hover_cursor]

- **WHEN** the pointer hovers a channel item
- **THEN** the cursor is the pointing hand

#### Scenario: Ctrl shows the select-all symbol [uic_channel_ctrl_symbol]

- **WHEN** Ctrl is held while the pointer is over a channel item
- **THEN** a dashed-square symbol is drawn at the pointer

#### Scenario: Rows match the Layers panel style [uic_channel_row_style]

- **WHEN** the Channels panel is shown
- **THEN** its rows use the same surface shading and item layout as the Layers panel

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
alpha rows SHALL continue the Ctrl+number labels.

#### Scenario: New, delete, save, load [cp_alpha_footer]

- **WHEN** the `tst_channels_panel` test creates and deletes a channel, saves a rectangle selection as a channel, deselects, selects that channel, and loads it
- **THEN** "Alpha 1" appears on Ctrl+6 and is removed with the matching states, the saved channel appears, and loading restores the rectangle as the selection

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
