## ADDED Requirements

### Requirement: Tools panel shape and header

The Tools panel SHALL present exactly two layouts: a compact layout with one
column of tool slots and an expanded layout with two columns of tool slots. The
double-arrow control in the panel header SHALL toggle between them, pointing to
the layout it will expand to, and the chosen layout SHALL be persisted.

The panel header SHALL be a CS6-style title row carrying the double-arrow toggle
and a close control, with a full-width `•••` drag grip immediately below the
title row matching the iconic History/Properties panel-group grip. The grip band
SHALL carry vertical padding above and below the dots so they do not hug the
title row or the tool grid. The grip and the title row SHALL both drag the whole
column through
the existing column-move path, including out of the main window as a frameless
`Qt::Tool` overlay. The grip SHALL be present in both layouts and while the
panel is docked or floating.

Neither layout SHALL show a horizontal or vertical scrollbar. When the panel is
docked and the workspace height is smaller than the height the current layout
requires, the panel SHALL remain visible and SHALL clip its content at the
workspace bottom rather than scrolling or hiding. A floating panel SHALL keep
hugging its content.

A floating panel SHALL move freely on the desktop. It SHALL re-dock only when the
pointer is inside the main window and within a narrow band at the workspace's
left or right edge; a point outside the workspace SHALL NOT draw a dock
indicator or re-dock the panel. A floating panel SHALL re-fit its hosted grid
when the layout flips, so both layouts show every tool slot and the footer
without clipping.

The close control SHALL hide the panel, and `Window > Panels > Tools` SHALL show
it again. Both layouts SHALL remain dockable to the left or right of the canvas,
and the existing atomic-column, fixed-width, and no-tabification rules SHALL
continue to hold.

#### Scenario: The compact layout is one column with no scrollbar [tsu_compact]

- **WHEN** the Tools panel is in its compact layout
- **THEN** exactly one column of tool slots is shown, no scrollbar is visible,
  and the header shows the expand double-arrow, the close control, and the
  `•••` grip

#### Scenario: The double-arrow expands to two columns [tsu_expanded]

- **WHEN** the user activates the header double-arrow while the panel is compact
- **THEN** the panel shows two columns of tool slots, keeps the grip and close
  control, shows no scrollbar, and the double-arrow now points back to the
  compact layout

#### Scenario: Flipping a floating panel keeps every tool visible [tsu_float_flip]

- **WHEN** the user flips a floating Tools panel from the expanded layout to the
  compact layout or back
- **THEN** the overlay re-fits to the new grid size and every tool slot and the
  footer stay visible, with no clipping or scrollbar

#### Scenario: The grip drags the panel out of the main window [tsu_grip_drag]

- **WHEN** the user drags the `•••` grip past the edge of the main window
- **THEN** the whole Tools panel follows the pointer as a frameless tool window
  that can sit outside the main window, and it can be re-docked on either side
  of the canvas

#### Scenario: A short workspace clips the panel [tsu_short_window]

- **WHEN** the docked workspace is shorter than the current layout requires
- **THEN** the Tools panel stays visible, clips its content at the workspace
  bottom, and shows no scrollbar

#### Scenario: A floating panel only docks inside the workspace edge [tsu_float_dock_zone]

- **WHEN** the user drags a floating Tools panel to a point outside the main
  window's workspace
- **THEN** no dock indicator is shown and the panel stays floating
- **WHEN** the user drags it into the workspace's left or right edge band
- **THEN** the dock indicator is shown and releasing there re-docks it on that
  side

#### Scenario: The close control hides and the Window menu restores [tsu_close]

- **WHEN** the user activates the header close control
- **THEN** the Tools panel hides, and invoking `Window > Panels > Tools` shows it
  again in the layout it had before
