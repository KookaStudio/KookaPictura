# ui/channels-panel Specification

## Purpose
The presentational CS6 Channels panel: a composite row and the three primary
colour rows, each with a grayscale thumbnail built from the existing composite
and a visibility toggle. It is presentational because the model has no
per-channel visibility and the compositor has no channel-mask path.

## Requirements

### Requirement: Channel rows

The `ChannelsPanel` SHALL present a fixed set of rows: an "RGB" composite row
followed by "Red", "Green", and "Blue", in that order. When the active layer
carries a raster layer mask, the panel SHALL append one more row after the colour
rows, named `<layer name> Mask`. `refresh` SHALL keep the row set stable.

#### Scenario: An RGB document shows four rows [cp_rows_four]

- **WHEN** an RGB document is open, its active layer has no raster layer mask,
  and the panel is refreshed
- **THEN** the panel reports four rows named "RGB", "Red", "Green", and "Blue"

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

### Requirement: Visibility toggles are local

Each row SHALL carry an eye toggle that flips only that row's local visibility
state and SHALL make no engine call. Toggling one row SHALL NOT change any other
row's state.

#### Scenario: Toggling one row flips only it [cp_toggle_local]

- **WHEN** the Red row's toggle is flipped
- **THEN** Red's local visibility changes and the other rows are unchanged

### Requirement: Channels panel is presentational

The `ChannelsPanel` SHALL NOT change the document: with no per-channel model
state and no compositor channel-mask path, a row's toggle is local UI state only,
and the RGB rows merely mirror the composite. The one exception is that selecting
the mask channel row shows the mask on the canvas through the view-only mask solo
view (`ui/mask-solo-view`), which changes only the canvas presentation. These
deferrals SHALL be recorded as known ceilings.

#### Scenario: A toggle leaves the canvas alone [cp_presentational]

- **WHEN** a colour channel toggle is flipped
- **THEN** the document image is unaffected and no engine call is made

#### Scenario: The suite is registered [cp_contract_suite]

- **WHEN** the app tests are built and run
- **THEN** `tst_channels_panel` is one of the registered Qt Test suites

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
