# ui/channels-panel Specification

## Purpose
The presentational CS6 Channels panel: a composite row and the three primary
colour rows, each with a grayscale thumbnail built from the existing composite
and a visibility toggle. It is presentational because the model has no
per-channel visibility and the compositor has no channel-mask path.

## Requirements

### Requirement: Channel rows

The `ChannelsPanel` SHALL present a fixed set of rows: an "RGB" composite row
followed by "Red", "Green", and "Blue", in that order. `refresh` SHALL keep the
row set stable.

#### Scenario: An RGB document shows four rows [cp_rows_four]

- **WHEN** an RGB document is open and the panel is refreshed
- **THEN** the panel reports four rows named "RGB", "Red", "Green", and "Blue"

#### Scenario: Refresh keeps the rows [cp_rows_stable]

- **WHEN** `refresh` runs again
- **THEN** the row count is unchanged

### Requirement: Channel thumbnails

The `ChannelsPanel` SHALL build a grayscale thumbnail for each row from the
coarsest display-pyramid level whose longest side is at least 64 px, falling
back to `view_->image()` when no such level exists (a single-level or tiny
document): the composite row SHALL use luminance (`qGray`), and the colour rows
SHALL use `qRed`, `qGreen`, and `qBlue` respectively, each scaled to a small
pixmap. With no document, the thumbnails SHALL be empty.

#### Scenario: Every row has a thumbnail [cp_thumbnail_present]

- **WHEN** an RGB document is open and the panel is refreshed
- **THEN** every row's thumbnail pixmap is non-null

### Requirement: Visibility toggles are local

Each row SHALL carry an eye toggle that flips only that row's local visibility
state and SHALL make no engine call. Toggling one row SHALL NOT change any other
row's state.

#### Scenario: Toggling one row flips only it [cp_toggle_local]

- **WHEN** the Red row's toggle is flipped
- **THEN** Red's local visibility changes and the other rows are unchanged

### Requirement: Channels panel is presentational

The `ChannelsPanel` SHALL NOT change the canvas or the document: with no
per-channel model state and no compositor channel-mask path, the toggle is local
UI state only. The panel SHALL list only the composite and RGB rows because
`document_mode()` collapses to grayscale/rgb at the bridge. These deferrals SHALL
be recorded as known ceilings.

#### Scenario: A toggle leaves the canvas alone [cp_presentational]

- **WHEN** a channel toggle is flipped
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
