## MODIFIED Requirements

### Requirement: Rulers toggle

`View > Rulers` (`Ctrl+R`) SHALL be a checkable command that shows or hides a
horizontal ruler above and a vertical ruler left of every open document
canvas. The rulers SHALL measure from the current ruler origin in the
ruler unit at the document's resolution, SHALL label distances from 0 without
a sign, SHALL follow the canvas pan and zoom, choosing a labelled major step
of 1, 2, or 5 × 10ⁿ units at least 50 screen pixels apart (eighths for a
1-inch step), and SHALL mark the cursor position. The setting SHALL be global,
off by default, and persisted in the session store.

#### Scenario: Toggle and persist

- **WHEN** `tst_rulers_guides` dispatches `View > Rulers` on a fresh session and saves the session
- **THEN** both rulers of the open document are visible, the stored `rulersVisible` is true, and dispatching again hides them

#### Scenario: Ruler scale follows zoom

- **WHEN** the ruler step is computed for pixels at zoom 1, 0.1, and 8, and for inches at 72 screen pixels per inch
- **THEN** the pixel major steps are 50, 500, and 10, and the inch step is 1 inch in eighths, each at least 50 screen pixels apart

## ADDED Requirements

### Requirement: Ruler origin

Dragging the rulers' intersection onto the canvas SHALL set the ruler origin in document coordinates. The ruler ticks and displayed coordinates SHALL follow the origin as the canvas pans and zooms. The origin SHALL default to the document top-left and be isolated per view. The origin drag SHALL place the origin at the released document point without snapping to guides, grid lines, or slice edges; target snapping is deferred with Snap / Snap To (#305). Changing the origin SHALL NOT change document pixels or history.

#### Scenario: Drag origin and navigate [ruler_origin_drag]

- **WHEN** the user drags the intersection to a canvas point, pans and zooms, and changes the ruler unit
- **THEN** both rulers and the Ruler and Info coordinate readouts measure from that document point in the selected unit and the document history is unchanged

#### Scenario: Origin placement near targets [ruler_origin_unsnapped]

- **WHEN** the user releases the origin near a visible guide, grid line or slice edge
- **THEN** the origin remains at the released document point without snapping

#### Scenario: Independent views [ruler_origin_views]

- **WHEN** the user moves the origin in one view and opens or selects another view
- **THEN** the other view retains its own origin, initially the document top-left
