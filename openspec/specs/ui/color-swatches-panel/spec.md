# color-swatches-panel Specification

## Purpose
Foreground and background colour state, the Color panel controls, and the Color and Swatches dock.

## Requirements

### Requirement: Foreground and background colour state
The system SHALL hold a foreground and a background colour for the application.
The Eyedropper tool SHALL set the foreground colour, and the Color and Swatches
panels SHALL read and write the same state.

#### Scenario: Eyedropper sets the foreground
- **WHEN** the Eyedropper samples a pixel
- **THEN** the foreground colour becomes the sampled colour

#### Scenario: Panel edit updates the state
- **WHEN** the user changes the colour in the Color panel
- **THEN** the foreground colour changes and any bound display updates

### Requirement: Color panel controls
The system SHALL provide RGB and HSB sliders, a hexadecimal field, and a colour
spectrum, and SHALL keep them synchronised with the foreground colour. Each
colour slider SHALL use a tracking slider so pressing on the groove and dragging
changes the channel continuously, rather than page-stepping or jumping to the
clicked position and stopping there.

#### Scenario: Slider and hex stay in sync
- **WHEN** the user changes any colour control
- **THEN** the other controls and the foreground swatch reflect the same colour

#### Scenario: Dragging a colour slider tracks continuously [lcs_slider_track]
- **WHEN** the user presses a Color panel slider's groove and drags without
  releasing
- **THEN** that channel follows the pointer continuously for the whole drag and
  the other controls update live

### Requirement: Default swatch grid
The system SHALL provide a Swatches panel with a default swatch grid; clicking a
swatch SHALL set the foreground colour.

#### Scenario: Click a swatch
- **WHEN** the user clicks a swatch
- **THEN** the foreground colour becomes that swatch colour

### Requirement: Color and Swatches dock and toggle
The system SHALL host the Color and Swatches panels in registered docks with
stable `objectName`s and SHALL expose `Window > Panels > Color` and
`Window > Panels > Swatches` toggles.

#### Scenario: Toggle the Color panel
- **WHEN** the user toggles Color from the Window menu
- **THEN** the panel is shown or hidden

### Requirement: Reflowing swatch grid

The Swatches panel SHALL present its swatches in a grid whose column count
follows the width it is given, so the panel shows more swatches per row as it is
made wider and its height follows from the resulting row count. A point SHALL
map to the swatch whose cell contains it, and to no swatch in a gap or past the
end.

#### Scenario: Width decides the columns [sw_reflow_columns]

- **WHEN** the grid is given a width that fits five swatch cells
- **THEN** its column count is five and its height is the rows that implies

#### Scenario: A point maps to a swatch [sw_reflow_index_at]

- **WHEN** a point inside the first swatch's cell is tested
- **THEN** `indexAt` returns the first swatch's index

#### Scenario: A point past the end maps to nothing [sw_reflow_past_end]

- **WHEN** a point below the last row of swatches is tested
- **THEN** `indexAt` returns -1

### Requirement: Swatch click modifiers

A plain click on a swatch SHALL set the foreground colour, a ctrl-click SHALL set
the background colour, and an alt-click SHALL remove the swatch. Each named
swatch SHALL show its name as a tooltip.

#### Scenario: Plain click sets the foreground [sw_click_foreground]

- **WHEN** a swatch is clicked without a modifier
- **THEN** the foreground colour becomes that swatch's colour

#### Scenario: Ctrl-click sets the background [sw_click_background]

- **WHEN** a swatch is ctrl-clicked
- **THEN** the background colour becomes that swatch's colour

#### Scenario: Alt-click deletes [sw_click_alt_delete]

- **WHEN** a swatch is alt-clicked
- **THEN** the swatch is removed from the grid

### Requirement: Swatch footer and context menu

The panel SHALL provide New and Delete actions in a footer and SHALL provide
Reset in the context menu. New SHALL add a swatch from the current foreground
colour, Delete SHALL remove the marked swatch, and Reset SHALL restore the
default palette. Clicking the empty space past the last swatch SHALL also
request a new swatch.

#### Scenario: New adds a foreground swatch [sw_footer_new]

- **WHEN** the New action is invoked with a foreground colour set
- **THEN** a swatch of that colour is appended to the grid

#### Scenario: Reset restores the palette [sw_footer_reset]

- **WHEN** the Reset action is invoked after swatches were added or removed
- **THEN** the grid holds the default palette again

### Requirement: Existing default palette preserved

The default swatch library SHALL preserve the colours of the previous fixed grid.

#### Scenario: Default palette matches [sw_default_palette]

- **WHEN** a new Swatches panel is constructed
- **THEN** the grid holds the forty-eight colours of the previous palette

### Requirement: Swatches added from the picker

The Swatches panel SHALL accept a colour and name added from the color picker
and append them to the grid. The panel SHALL generate `Swatch N` names for
swatches added through its New action, where N is one more than the highest
existing `Swatch N` name.

#### Scenario: The picker adds a swatch [sw_add_from_picker]

- **WHEN** the color picker adds a colour to the panel
- **THEN** the grid contains that colour under the entered name

#### Scenario: New uses a generated name [sw_generated_name]

- **WHEN** New is invoked twice on a fresh panel
- **THEN** the added swatches are named `Swatch 1` and `Swatch 2`

### Requirement: Swatches persist across sessions

The Swatches panel's contents SHALL persist in the application session store and
SHALL be restored on the next launch. A store without swatch data SHALL open the
default palette; a stored empty panel SHALL stay empty. Saving an unrelated
setting SHALL preserve the stored swatches.

#### Scenario: Added swatches survive a restart [sw_persist_restart]

- **WHEN** a swatch is added and a new window is created
- **THEN** the panel contains the added swatch

#### Scenario: An empty panel survives a restart [sw_persist_empty]

- **WHEN** every swatch is removed and a new window is created
- **THEN** the panel is empty and the default palette is not restored

#### Scenario: An older store opens the default palette [sw_persist_default]

- **WHEN** the session store has no swatch data
- **THEN** the panel holds the default palette

#### Scenario: An unrelated save keeps the swatches [sw_persist_unrelated_save]

- **WHEN** another setting is saved without touching the panel
- **THEN** the stored swatches are unchanged

### Requirement: Color panel swatches open the picker

Clicking the Color panel's foreground or background swatch SHALL make that
swatch active and open the Color Picker with its current colour. Accepting the
picker SHALL set the active swatch to the chosen colour; cancelling SHALL leave
the colour unchanged.

#### Scenario: Color panel swatch opens the picker

- **WHEN** the user clicks the Color panel's foreground swatch and accepts a new
  colour in the picker
- **THEN** the picker was opened for the foreground and the shared foreground
  colour becomes the accepted colour
