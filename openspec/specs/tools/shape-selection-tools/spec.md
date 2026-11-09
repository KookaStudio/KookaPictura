# shape-selection-tools Specification

## Purpose
Rectangular and elliptical marquee, lasso and polygonal lasso, quick selection, combine modes, and a bounds overlay.

## Requirements

### Requirement: Rectangular and elliptical marquee
The system SHALL create a selection from a dragged rectangle (Rectangular
Marquee) or ellipse (Elliptical Marquee) whose bounds are the drag rectangle. At
least one pixel covered at the pixel centre SHALL be selected. Both tools SHALL
be enabled in the toolbox, SHALL support New, Add, Subtract, and Intersect
combine modes, and SHALL apply a tool-time Feather radius (0-250 px, decimal) to
the rasterised shape before combining. The Style option SHALL constrain the drag
geometry before rasterisation: Normal follows the drag, Fixed Ratio keeps the
entered width-to-height ratio while staying inside the non-degenerate box the
cursor draws, and Fixed Size places the entered pixel size with its top-left
corner at the current pointer and follows the pointer until release. A one-axis
Fixed Ratio drag SHALL still derive the other axis from the entered ratio and MAY
extend outside the degenerate box. Fixed Ratio and Fixed Size SHALL round half
away from zero. A Fixed Size selection partly or wholly outside the canvas SHALL
be clipped by the rasteriser. Fixed Ratio and Fixed Size SHALL ignore Shift and
Alt. Anti-alias SHALL be offered only for the Elliptical Marquee; because the
engine rasteriser is binary, the control SHALL be visible but disabled with a
reason rather than silently ignored. Fixed-ratio and fixed-size defaults, and
non-pixel size units, are inferred and MAY be deferred.

#### Scenario: Drag a rectangular selection
- **WHEN** the Rectangular Marquee tool is dragged from one point to another
- **THEN** the selection covers the dragged rectangle

#### Scenario: Drag an elliptical selection
- **WHEN** the Elliptical Marquee tool is dragged from one point to another
- **THEN** the selection covers the ellipse inscribed in the dragged rectangle

#### Scenario: Tool-time feather softens the boundary
- **WHEN** the Elliptical Marquee is dragged with Feather greater than zero
- **THEN** the committed selection has boundary pixels strictly between 0 and 255

#### Scenario: Fixed ratio constrains the geometry
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the tool is dragged
- **THEN** the committed selection's width-to-height ratio is 2 within 1 px

#### Scenario: Fixed ratio fits the drag box [lst_fixed_ratio_fits_drag]
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the cursor draws a 40x10 box
- **THEN** the selection is 20x10 inside that box, not 40x20 past the cursor

#### Scenario: Fixed ratio fits the narrow axis [lst_fixed_ratio_narrow]
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the cursor draws a 10x40 box
- **THEN** the selection is 10x5 inside that box

#### Scenario: A one-axis Fixed Ratio drag derives the other axis [lst_fixed_ratio_one_axis]
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the cursor draws a 0x40 box
- **THEN** the selection is 80x40 and may extend outside the degenerate box

#### Scenario: Fixed ratio fits a reverse drag [lst_fixed_ratio_reverse]
- **WHEN** Style is Fixed Ratio with a 2:1 ratio and the cursor drags from (50,50) to (10,10)
- **THEN** the selection is (10,30,40,20)

#### Scenario: Fixed size ignores the drag extent
- **WHEN** Style is Fixed Size with a 100x50 size and the pointer is dragged from the press point
- **THEN** only the size is fixed; the committed selection is 100x50 with its top-left corner at the release point

#### Scenario: Fixed size follows the pointer on a large document [lst_fixed_size_follows]
- **WHEN** Style is Fixed Size with a 100x50 size on a document large enough to hold it and the pointer is dragged to (120,80)
- **THEN** the selection is 100x50 with its top-left corner at (120,80)

#### Scenario: Fixed size clips at the canvas edge [lst_fixed_size_clip]
- **WHEN** Style is Fixed Size with a 100x50 size on a 64x64 document and the release point is (20,20)
- **THEN** the committed selection is clipped to the canvas

#### Scenario: The ellipse uses its own Fixed Ratio [lst_elliptical_fixed_ratio]
- **WHEN** the Elliptical Marquee is set to Fixed Ratio with a 2:1 ratio and the cursor draws a 40x10 box
- **THEN** the committed ellipse is inscribed in a 20x10 rectangle

#### Scenario: The ellipse uses its own Fixed Size [lst_elliptical_fixed_size]
- **WHEN** the Elliptical Marquee is set to Fixed Size with a 100x50 size on a document large enough to hold it and the pointer is dragged to (120,80)
- **THEN** the committed ellipse is inscribed in a 100x50 rectangle with its top-left corner at (120,80)

#### Scenario: Anti-alias is honest about the ceiling
- **WHEN** the Elliptical Marquee options are inspected
- **THEN** the Anti-alias control is visible and disabled with a reason that the rasteriser is binary

### Requirement: Lasso selection
The system SHALL create a selection from a freehand closed polygon traced by the
Lasso tool, filling the polygon's interior with even-odd winding.

#### Scenario: Lasso a region
- **WHEN** the Lasso tool is dragged around a region and released
- **THEN** the selection covers the polygon interior

#### Scenario: Lasso needs three points
- **WHEN** a lasso drag produces fewer than three points
- **THEN** no selection is created

### Requirement: Quick Selection
The system SHALL grow a selection around the cursor while the Quick Selection
tool is dragged, as a per-point flood selection with the current tolerance.

#### Scenario: Drag grows the selection
- **WHEN** the Quick Selection tool is dragged across a flat-coloured region
- **THEN** the selection grows to cover pixels within the tolerance of the points sampled

### Requirement: Selection combine modes

The system SHALL support New, Add, Subtract, and Intersect combine modes for the
selection tools. The default SHALL be New, which replaces any existing selection.
While a modal selection tool gestures, holding Shift SHALL mean Add, holding Alt
at the press SHALL mean Subtract, and holding Shift+Alt at the press SHALL mean
Intersect, but only when a selection already exists when the gesture starts;
without an existing selection the modifiers SHALL NOT select
Add/Subtract/Intersect (the options-bar mode applies). The quick mode SHALL be
decided at the start of the gesture and locked for its whole duration until the
selection commits, so releasing a modifier mid-gesture does not change the
result. The options-bar combine buttons SHALL remain the default when no
modifier is held. A gesture's combine mode SHALL be derived from two independent
inputs: the **quick mode** locked at press and the **modifier geometry**, and the
two SHALL NOT be conflated — Shift/Alt choose the combine operation while Alt at
the press with no existing selection SHALL select the from-centre (alt-pivot)
geometry; an Alt press after the drag has begun SHALL pivot-mirror per the
Marquee Alt press and mid-drag behavior requirement. Alt SHALL NOT pre-toggle
the combine mode before a drag begins, and a modifier held while merely
hovering SHALL NOT change the mode. The canvas cursor SHALL reflect the mode
that a press would use: with an existing selection and Shift or Alt held over
the canvas, the canvas SHALL show the combine cursor rather than the
move-selection cursor.

#### Scenario: Add extends an existing selection

- **WHEN** Add mode is selected and a new marquee is dragged overlapping the current selection
- **THEN** the result is the union of the old and new coverage

#### Scenario: Subtract removes coverage

- **WHEN** Subtract mode is selected and a marquee is dragged over the selection
- **THEN** the result excludes the marquee coverage

#### Scenario: Intersect keeps the overlap

- **WHEN** Intersect mode is selected and a marquee overlaps the selection
- **THEN** the result is the overlap of the old and new coverage

#### Scenario: New replaces the selection

- **WHEN** New mode is selected and a marquee is dragged
- **THEN** any previous selection is discarded

#### Scenario: Shift adds mid-gesture

- **WHEN** a selection exists and a selection tool is dragged with Shift held from the first press
- **THEN** the committed result is the union of the existing selection and the dragged shape

#### Scenario: Alt subtracts mid-gesture

- **WHEN** a selection exists and a selection tool is dragged with Alt held from the first press
- **THEN** the committed result excludes the dragged shape from the existing selection

#### Scenario: Shift+Alt intersects

- **WHEN** a selection exists and a selection tool is dragged with Shift+Alt held from the first press
- **THEN** the committed result is the overlap of the existing selection and the dragged shape

#### Scenario: The quick mode is locked for the gesture

- **WHEN** a selection exists and the gesture starts with Shift held, and Shift is released before mouse-up
- **THEN** the committed result is still an Add, not a New replacement

#### Scenario: No existing selection ignores the combine modifier

- **WHEN** there is no selection and a selection tool is dragged with Alt held at the press
- **THEN** the modifier chooses the from-centre geometry when applicable and the committed selection is created as New, not subtracted

#### Scenario: The combine cursor shows while hovering [lst_combine_cursor]

- **WHEN** a selection exists and a selection tool hovers it with Shift or Alt held, without any press
- **THEN** the canvas shows the combine/mode cursor and not the move-selection cursor

#### Scenario: Alt does not pre-toggle before a drag [lst_alt_no_pretoggle]

- **WHEN** Alt is held over the canvas with a selection tool active but no drag has started
- **THEN** the locked quick mode is unchanged and no combine operation is selected until a press captures the modifiers

#### Scenario: Alt during the drag mirrors instead of subtracting [lst_alt_drag_mirror_combine]

- **WHEN** a selection exists, a marquee drag starts with no Alt held, and Alt is first pressed during the drag
- **THEN** the rectangle mirrors about the anchor, the combine mode stays the one captured at press, and the committed result is not a Subtract

### Requirement: Selection bounds overlay
The system SHALL show a rubber-band overlay for the selection being dragged and
SHALL expose the committed selection's integer bounds.

#### Scenario: Rubber band during the drag
- **WHEN** a selection tool is being dragged
- **THEN** an outline of the in-progress region is drawn over the canvas

#### Scenario: Committed selection bounds
- **WHEN** a selection has been committed
- **THEN** its bounding rectangle can be queried

### Requirement: Polygonal Lasso
The system SHALL provide an enabled Polygonal Lasso tool that builds a selection
from clicked vertices: each click appends a vertex, the in-progress path SHALL
preview over the canvas, and clicking the first vertex, double-clicking, or
pressing Enter SHALL close and rasterise the polygon with even-odd winding via
the existing polygon path. The in-progress preview SHALL be a solid polyline
that runs from the first vertex through every clicked vertex to the live cursor
position (a rubber band), and SHALL NOT draw the closing edge until the polygon
commits. The tool SHALL support New, Add, Subtract, and Intersect combine modes
and a tool-time Feather radius, and SHALL NOT create a selection from fewer than
three vertices. Pressing Escape SHALL clear the in-progress path and leave the
document selection unchanged.

#### Scenario: Click vertices close into a selection
- **WHEN** the Polygonal Lasso places three or more vertices and closes on the first vertex
- **THEN** the selection covers the polygon interior

#### Scenario: Double-click closes the path
- **WHEN** the Polygonal Lasso places vertices and the user double-clicks
- **THEN** the polygon closes and commits

#### Scenario: The rubber band tracks the cursor
- **WHEN** the Polygonal Lasso has clicked one or more vertices and the pointer moves without clicking
- **THEN** the preview is a solid open path from the first vertex through the clicked vertices to the pointer position

#### Scenario: Too few vertices create nothing
- **WHEN** the in-progress path has fewer than three vertices and is closed
- **THEN** no selection is created and no history state is recorded

#### Scenario: Escape cancels without changing the selection
- **WHEN** Escape is pressed during a polygonal trace
- **THEN** the in-progress path is discarded and the document selection is unchanged

### Requirement: Selection tool options bars
The system SHALL show context-sensitive options for each enabled selection tool:
combine mode (New/Add/Subtract/Intersect) and Feather for the marquee and lasso
tools, Tolerance and Contiguous for the Magic Wand, and mode plus Tolerance for
Quick Selection. Options that the engine does not model SHALL be visible and
disabled with a stated reason: Anti-alias for the ellipse, lasso, and wand;
Sample All Layers for the wand and Quick Selection; and Auto-Enhance for Quick
Selection. Quick Selection SHALL offer New, Add, and Subtract only, with no
Intersect mode.

#### Scenario: The wand options bar exposes the engine parameters
- **WHEN** the Magic Wand tool is active
- **THEN** the options bar shows combine mode, Tolerance, Contiguous, Anti-alias, and Sample All Layers

#### Scenario: Contiguous switches the wand mode
- **WHEN** Contiguous is unchecked and the wand clicks a colour region
- **THEN** every matching pixel in the image is selected, not only the connected region

#### Scenario: Unmodelled options are disabled with a reason
- **WHEN** the wand or Quick Selection options bar is shown
- **THEN** Anti-alias, Sample All Layers, and Auto-Enhance are disabled and carry a reason

#### Scenario: Quick Selection has no Intersect
- **WHEN** the Quick Selection options bar is shown
- **THEN** only New, Add, and Subtract are present

### Requirement: Marquee modifier constraints and size readout

While a Rectangular or Elliptical Marquee drag is active, holding Shift SHALL
constrain the shape to a square (rectangle) or circle (ellipse). Alt held at the
press with an existing selection SHALL choose Subtract and SHALL NOT mirror;
with no existing selection, Alt held at the press SHALL select the from-centre
geometry (the press point is the pivot) and commit New. The first Alt press after
the drag has begun SHALL arm pivot-mirror immediately and SHALL NOT switch to
Subtract. Holding Shift and Alt at the press with no existing selection SHALL
produce a square/circle centred on the press point. These constraints refine the
Normal drag; Fixed Ratio and Fixed Size use their own geometry and SHALL ignore
Shift and Alt. **Shift and the combine mode SHALL be captured at press
(`dragMods_`/`dragMode_`) and used unchanged for the live preview geometry, the
release rasterisation, and the drag cursor, so releasing Shift or Alt partway
through a drag keeps the constrained geometry and the mode that were active at
press. The mid-drag Alt-pivot is the exception: it changes the live and release
geometry to mirror; the captured combine mode and cursor stay unchanged.** For
both tools, a floating readout of the current selection size in document pixels
(`W × H`) SHALL follow the cursor during the drag and SHALL disappear on release.

#### Scenario: Shift makes a square

- **WHEN** the Rectangular Marquee is dragged with Shift held from the first press
- **THEN** the committed selection is a square whose side equals the longer drag axis

#### Scenario: Shift makes a circle

- **WHEN** the Elliptical Marquee is dragged with Shift held
- **THEN** the committed selection is a circle inscribed in a square

#### Scenario: Alt draws from the centre

- **WHEN** there is no existing selection and the Rectangular Marquee is dragged with Alt held from a press point
- **THEN** the committed rectangle is centred on the press point, extends on all sides by the drag delta, and commits New

#### Scenario: Releasing a modifier mid-drag keeps the geometry [lst_marquee_dragmods]

- **WHEN** a marquee drag starts with Shift held, or with Alt held at the press with no existing selection, and the modifier is released before mouse-up
- **THEN** the committed selection keeps the square/centre constraint captured at press and the quick mode is unchanged

#### Scenario: The size readout tracks the drag

- **WHEN** a Marquee or Elliptical Marquee drag is in progress
- **THEN** a `W × H` pixel readout is shown next to the cursor and cleared when the drag ends

#### Scenario: Alt at the press subtracts without mirroring [lst_alt_press_subtract]

- **WHEN** a selection exists and the Rectangular Marquee is dragged with Alt held from the press
- **THEN** the drag subtracts the ordinary drag rectangle and does not mirror it

#### Scenario: Alt during the drag mirrors [lst_alt_drag_mirror]

- **WHEN** a marquee drag begins with no Alt held and Alt is first pressed during the drag
- **THEN** the live and committed rectangle mirror about the anchor and the combine mode stays the one captured at press

#### Scenario: Fixed Ratio ignores Shift and Alt [lst_fixed_ratio_modifiers]

- **WHEN** Style is Fixed Ratio and the drag is made with Shift or Alt held
- **THEN** the committed selection keeps the entered ratio and is not squared, mirrored, or centred

#### Scenario: Fixed Size ignores Shift and Alt [lst_fixed_size_modifiers]

- **WHEN** Style is Fixed Size and the drag is made with Shift or Alt held
- **THEN** the committed selection is the entered size with its top-left at the release point and is not squared, mirrored, or centred

### Requirement: Selection-move hover cursor

The canvas SHALL update the move-selection cursor as the pointer moves over the
document, showing the move cursor while the pointer is inside the active
selection for a selection tool and reverting to the tool cursor when it leaves,
without requiring a mouse press. Holding Ctrl with a selection tool over the
canvas SHALL show the move cursor even before the pointer enters the selection.
**The move-selection cursor SHALL be gated on no Shift and no Alt: with an
existing selection and Shift or Alt held, the combine cursor takes precedence so
a user about to add to or subtract from the selection sees the correct cursor.
While a drag is in progress, the cursor SHALL reflect the captured `dragMods_`
rather than the live keyboard state.**

#### Scenario: Hover shows the move cursor

- **WHEN** a selection exists and the pointer is moved into it with a selection tool active
- **THEN** the move cursor is shown before any click

#### Scenario: Leaving the selection restores the tool cursor

- **WHEN** the pointer is moved from inside the selection to outside it
- **THEN** the tool's own cursor is restored without a click

#### Scenario: Ctrl previews the content move

- **WHEN** a selection exists and Ctrl is pressed with a selection tool active
- **THEN** the move cursor is shown even if the pointer is outside the selection

#### Scenario: A combine modifier beats the move cursor [lst_move_cursor_gate]

- **WHEN** a selection exists, the pointer is inside it with a selection tool active, and Shift or Alt is held
- **THEN** the combine/mode cursor is shown instead of the move-selection cursor

### Requirement: Lasso resolves the combine mode at press

The freehand Lasso SHALL resolve its combine mode at press exactly as the
Rectangular/Elliptical Marquee, the Polygonal Lasso, and the Magic Wand do: from
the modifiers held at press and whether a selection already exists, falling back
to the options-bar mode when no modifier is held. The resolved mode SHALL be
captured for the whole gesture so releasing a modifier mid-drag does not change
the result, and it SHALL drive both `begin_lasso` and the drag cursor.

#### Scenario: Shift extends the selection [lasso_combine_add]

- **WHEN** a selection exists and the Lasso is dragged with Shift held from the
  first press
- **THEN** the committed result is the union of the existing selection and the
  lasso path

#### Scenario: Alt subtracts from the selection [lasso_combine_subtract]

- **WHEN** a selection exists and the Lasso is dragged with Alt held from the
  first press
- **THEN** the committed result excludes the lasso path

#### Scenario: The options-bar mode applies with no modifier [lasso_options_mode]

- **WHEN** no modifier is held and the options-bar combine mode is Subtract
- **THEN** the Lasso subtracts, not replaces

#### Scenario: The mode is locked for the gesture [lasso_combine_locked]

- **WHEN** a Lasso drag starts with Shift held and Shift is released before
  mouse-up
- **THEN** the committed result is still an Add

### Requirement: Combine drag cursor survives a modifier release

While a selection-tool gesture is in progress, the canvas cursor SHALL reflect
the combine mode captured at press (`dragMode_`), not the live keyboard state.
Releasing Shift or Alt partway through a combine drag SHALL keep the
add/subtract cursor until the gesture ends, even when the pointer is inside the
existing selection. Merely hovering an existing selection with no gesture in
progress SHALL still show the move-selection cursor.

#### Scenario: Releasing the modifier keeps the combine cursor [lst_drag_cursor_retained]

- **WHEN** a combine marquee/lasso drag starts with Shift or Alt held and the
  modifier is released before mouse-up
- **THEN** the canvas still shows the add/subtract cursor for the rest of the drag

#### Scenario: Hover still shows the move cursor [lst_drag_cursor_hover]

- **WHEN** no drag is in progress and the selection tool hovers an existing
  selection with no Shift/Alt held
- **THEN** the move-selection cursor is shown

### Requirement: A click outside the selection deselects

When a selection exists and a selection-tool gesture in New mode encloses no
area, the system SHALL run Deselect (one "Deselect" history state that Reselect
can restore). A gesture encloses no area when a Rectangular or Elliptical
Marquee release has zero width or height, a Lasso release has fewer than three
points, or a Polygonal Lasso outline closes with fewer than three vertices. In
Add, Subtract, or Intersect mode such a gesture SHALL leave the selection and
history unchanged. A press inside the selection keeps starting a selection move.

#### Scenario: Marquee click outside deselects

- **WHEN** a selection exists and the Rectangular Marquee clicks outside it without dragging
- **THEN** the selection is cleared as one "Deselect" state and Reselect restores it

#### Scenario: Lasso and Polygonal Lasso clicks deselect

- **WHEN** the Lasso clicks outside the selection, or the Polygonal Lasso double-clicks one spot outside it
- **THEN** the selection is cleared as one "Deselect" state

#### Scenario: An Add-mode click keeps the selection

- **WHEN** the Rectangular Marquee Shift-clicks outside the selection
- **THEN** the selection and history are unchanged

### Requirement: Marquee Style is per tool
The Rectangular Marquee and Elliptical Marquee SHALL each own a whole selection
options group, so Style, ratio, size, combine mode, Feather, Tolerance, and the
magnetic-lasso detection fields are independent between the two tools. Changing
one tool's options SHALL NOT change the other tool's options or the geometry it
commits. A missing persisted elliptical group SHALL restore Normal Style with
the default ratio and size.

#### Scenario: Fixed Size on the rectangle leaves the ellipse Normal [lst_elliptical_style_independent]
- **WHEN** the Rectangular Marquee is set to Fixed Size and the Elliptical Marquee is selected
- **THEN** the Elliptical Marquee still uses Normal Style and commits an ellipse from the drag rectangle

#### Scenario: Fixed Ratio on the ellipse leaves the rectangle Normal [lst_rect_style_independent]
- **WHEN** the Elliptical Marquee is set to Fixed Ratio and the Rectangular Marquee is selected
- **THEN** the Rectangular Marquee still uses Normal Style and commits a rectangle from the drag rectangle

#### Scenario: The ellipse uses its own Fixed Ratio and Fixed Size [lst_elliptical_fixed_style]
- **WHEN** the Elliptical Marquee is set to Fixed Ratio and then Fixed Size while the Rectangular Marquee stays Normal
- **THEN** each elliptical drag uses the ellipse's own style and the rectangle's style is unchanged

### Requirement: Polygonal Lasso close radius scales with zoom
The Polygonal Lasso's close-click and double-click hit radius SHALL be fixed on
screen at 6 px, so the document-space radius is `6 / max(zoom, 1e-6)`. The
`max` guards against division by zero; the view already clamps zoom to 0.01, so
a zero or negative zoom is not reachable in normal use. At 200% zoom a close
click 4 document pixels from the first vertex SHALL NOT close the outline,
while a close click 3 document pixels away SHALL close it. The outline SHALL
already hold three placed vertices when the close click arrives.

#### Scenario: A small selection is possible when zoomed in [lst_polygon_close_zoom]
- **WHEN** the Polygonal Lasso is at 200% zoom, three vertices are placed, and the close click is 4 document pixels from the first vertex
- **THEN** the outline stays open instead of closing

#### Scenario: The close click still fires on the screen radius [lst_polygon_close_zoom_hit]
- **WHEN** the Polygonal Lasso is at 200% zoom, three vertices are placed, and the close click is 3 document pixels from the first vertex
- **THEN** the outline closes and commits

#### Scenario: The double-click fires on the screen radius [lst_polygon_double_click_zoom]
- **WHEN** the Polygonal Lasso is at 200% zoom, three vertices are placed, and the double-click is 3 document pixels from the last vertex
- **THEN** the outline closes and commits

#### Scenario: A double-click outside the radius stays open [lst_polygon_double_click_zoom_miss]
- **WHEN** the Polygonal Lasso is at 200% zoom, three vertices are placed, and the double-click is 4 document pixels from the last vertex
- **THEN** the outline stays open
