## MODIFIED Requirements

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

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: Visibility toggles are local

**Reason**: The eyes now hide channels on the canvas (Channel visibility on the
canvas); the composite eye follows the channel eyes as CS6's does.

**Migration**: None; the eyes drive `channelMaskChanged` instead of local state.

### Requirement: Channels panel is presentational

**Reason**: The panel now changes the canvas view (channel visibility) and the
document (alpha channels), and lists the document's mode instead of only RGB.

**Migration**: None; see Channel rows, Channel visibility on the canvas, and
Alpha channels and the footer.
