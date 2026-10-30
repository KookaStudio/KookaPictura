# canvas-tools Specification

## Purpose
Move, Crop, Eyedropper, Hand, and Zoom tools for navigating and editing the canvas.

## Requirements

### Requirement: Move tool translates the active layer
The system SHALL translate the document's selected layers by a dragged offset,
recompositing the document. With one layer selected it SHALL translate that
layer; with several selected it SHALL translate every selected layer. The live
preview SHALL show the moving layers at the pointer's offset and the committed
move SHALL land them at that offset, including a link set whose preview image is
cropped to the canvas. Group and adjustment layers SHALL be ignored (not
translated), and an empty or absent document SHALL be a no-op. A layer whose lock
state includes the position lock SHALL be refused, and the refusal SHALL leave
that layer unchanged; a move that translates no layer SHALL add no history state
and change no pixels. A selected layer's linked members move with it (see
`compositing/layer-management`).

#### Scenario: Drag moves the layer content
- **WHEN** the Move tool is dragged by a delta and the active document has a pixel layer
- **THEN** that layer's content is translated by the delta and the composite updates

#### Scenario: No pixel layer
- **WHEN** the Move tool is dragged and the document has no pixel layer
- **THEN** nothing changes and the document is not corrupted

#### Scenario: A position-locked layer is refused [lct_move_locked]
- **WHEN** the Move tool is dragged and the topmost pixel layer is position-locked
- **THEN** the layer does not move, the composite is unchanged, and no history
  state is added

#### Scenario: Dragging a multi-selection moves every selected layer [lct_move_multi]
- **WHEN** two pixel layers are selected and the Move tool is dragged by a delta
- **THEN** both layers translate by the delta and the move is one `Move Layer`
  history state

#### Scenario: A cropped group preview lands at the drop [lct_move_preview_origin]
- **WHEN** a linked set that extends past the canvas edge is dragged again and a
  delta is committed
- **THEN** the preview origin matches the preview image's crop and the set lands
  at the drop offset

### Requirement: Crop tool
The system SHALL place a crop box over the whole canvas when the Crop tool is
chosen (fitted to the aspect ratio when one is set), shading the canvas area
outside it — not the workspace around the canvas — and drawing its frame and
eight handles (an active box also draws the selected guide overlay). Selecting
the tool SHALL show a preview box fitted to the ratio and centered in either
mode, drawn with its frame and handles but no guides. Dragging a handle of an active box SHALL resize the box
(keeping the aspect ratio when one is set) and SHALL snap the dragged edge or
corner to the canvas or a layer bounding edge within a screen-pixel threshold;
dragging inside an active box SHALL move the box in Classic mode and reposition
the composite behind a fixed box in Modern mode; and (per the `crop-straighten`
capability), while a box is active, dragging outside it (beyond a resize margin,
including past the canvas edge) SHALL rotate the content behind the fixed box; a
new box SHALL be drawn from the preview or when no box exists, with the matching
move/resize/rotate cursor over each part. The first box drawn SHALL be clamped to
the canvas and its press corner SHALL stay fixed; a handle resize after that MAY
extend the box beyond the canvas and SHALL grow the canvas to contain it. Enter, a double-click inside the box, or the options bar's Apply SHALL crop the
document to the box as one "Crop: <overlay>" state named for the active guide
overlay (e.g. "Crop: Rule of Thirds", "Crop: Grid"), applying any straighten angle; Escape
or the options bar's Cancel SHALL clear the box and angle, discard the tool
session, and leave the Crop tool active with no box shown, so a fresh drag draws
a new box honoring the active ratio. The options bar
SHALL offer aspect-ratio presets (Ratio, 1:1, 4:5, 5:7, 2:3, 16:9), the
full Crop controls defined by `tool-framework`, and Delete Cropped Pixels (on by
default): when on, each pixel layer's pixels and mask outside the new canvas
SHALL be discarded (layers with live type, smart objects, vector masks, or
retained 16/32-bit samples keep theirs); when off they SHALL be kept beyond the
canvas edge. A crop SHALL be pending only while the tool is active and the box
differs from the canvas. The system SHALL also expose `Image > Crop`, which
commits a pending crop or else crops to the current selection bounds.

#### Scenario: Crop by tool
- **WHEN** the Crop tool's box is resized to a region and Enter is pressed
- **THEN** the document is resized to the region and its content is shifted so the region's top-left becomes the origin

#### Scenario: The default box is not a pending crop
- **WHEN** the Crop tool is chosen and its box is left canvas-sized
- **THEN** a crop box is shown, no crop is pending, and `Image > Crop` still crops to the selection

#### Scenario: Move, reset, and ratio lock
- **WHEN** the `crop_tool` self-test resizes the box by its bottom-right handle, drags inside it, presses Escape, and resizes with a 1:1 ratio set
- **THEN** the box resizes, moves by the drag, is cleared by Escape, and stays square

#### Scenario: Delete Cropped Pixels
- **WHEN** a crop is committed with Delete Cropped Pixels on, and again with it off
- **THEN** with it on the layer is trimmed to the new canvas, and with it off the layer keeps its pixels beyond the canvas edge

#### Scenario: Rotate zone versus new box
- **WHEN** a press begins outside the box while a box is active versus in the init mode
- **THEN** the first begins a rotation about the box centre and the second draws a new box clamped to the canvas

#### Scenario: A handle grows the canvas [canvas_crop_grow]
- **WHEN** a crop handle of an existing box is dragged past the canvas edge
- **THEN** the box extends past the canvas and the canvas grows to contain it

#### Scenario: Crop by selection
- **WHEN** `Image > Crop` is invoked with an active selection
- **THEN** the document is cropped to the selection bounds

#### Scenario: Crop without a region
- **WHEN** crop is committed with no pending region and no selection
- **THEN** nothing changes

#### Scenario: History names the guide overlay
- **WHEN** a crop is committed with the Grid overlay chosen in the grid menu
- **THEN** the new History state is named "Crop: Grid" (and "Crop: Rule of Thirds" with the default overlay)

### Requirement: Eyedropper samples a colour
The system SHALL sample the composited colour at a canvas point, expose it as a
packed ARGB value, and record it as the frame's foreground colour.

#### Scenario: Sample a pixel
- **WHEN** the Eyedropper is clicked at a canvas point over an opaque red pixel
- **THEN** the sampled ARGB is opaque red and the frame's foreground colour becomes red

#### Scenario: Sample outside the document
- **WHEN** the Eyedropper is clicked outside the document bounds
- **THEN** sampling fails and the foreground colour is unchanged

### Requirement: Hand and Zoom tools

The system SHALL provide Hand and Zoom as first-class tools. Hand SHALL pan the
canvas on drag; Zoom SHALL magnify on click and reduce on modified click. The
canvas offset SHALL be clamped by one shared range helper at every mutation point
— pan, zoom anchoring, centre, fit, actual pixels, initial view, resize, and the
Navigator proxy — so that at least one display inch of 96 logical pixels of the
canvas remains visible on each axis whenever the document is larger than that
axis. The clamp SHALL slide each axis by the minimum needed rather than
recentering, and the canvas SHALL remain the single source of the offset.
**Zoom-in SHALL target the pointer: the Zoom tool's zoom-in click SHALL anchor
the zoom at the clicked image point, and a wheel zoom-in SHALL keep the point
under the cursor fixed. Zoom-out SHALL anchor at the canvas centre so the image
does not drift off the pointer: a wheel zoom-out, an `Alt`/`Option` (modified)
Zoom-tool click, and `View > Zoom Out` SHALL all step about the viewport centre.
Dragging with the Zoom tool (press, move, release) SHALL draw a marquee, and on
release the view SHALL zoom so that rectangle fills the viewport at the highest
magnification that fits it, centred on the rectangle; holding `Space` mid-marquee
SHALL reposition the marquee instead of resizing it. A press-release with no drag
SHALL keep the click step. The Navigator slider SHALL anchor at the click/cursor
point on the proxy. The wheel SHALL apply one shared modifier precedence: a
horizontal side-wheel (`angleDelta().x() != 0`) SHALL pan horizontally; a
`Ctrl+Alt` vertical wheel SHALL pan vertically; an `Alt` (without `Ctrl`)
vertical wheel SHALL pan horizontally; otherwise the vertical wheel SHALL zoom,
with `Shift` doubling the wheel zoom step.**

#### Scenario: Zoom in by click

- **WHEN** the Zoom tool is clicked on the canvas
- **THEN** the canvas magnification increases

#### Scenario: Zoom out by modified click

- **WHEN** the Zoom tool is clicked while a modifier is held
- **THEN** the canvas magnification decreases

#### Scenario: Zoom click targets the cursor [lct_zoom_cursor]

- **WHEN** the Zoom tool clicks a point away from the canvas centre without a modifier
- **THEN** the zoom is anchored at that point so it stays under the cursor rather
  than the view zooming about the centre

#### Scenario: Zoom-out anchors at the canvas centre [zta_zoom_out_centre]

- **WHEN** the wheel zooms out, or the Zoom tool is clicked with `Alt`/`Option`
- **THEN** the document centre stays at (near) the centre of the viewport and the
  image does not drift toward the pointer

#### Scenario: A Zoom drag draws a marquee [zta_marquee]

- **WHEN** the Zoom tool is pressed, dragged over a sub-rectangle, and released
- **THEN** a marquee is drawn during the drag, and on release the view zooms so
  that rectangle fills the viewport at the highest magnification that fits it,
  centred

#### Scenario: A click without a drag still steps once [zta_click_steps]

- **WHEN** the Zoom tool is pressed and released at the same point
- **THEN** the zoom steps once (in, or out with `Alt`/`Option`) rather than
  drawing a marquee

#### Scenario: Shift doubles the wheel step [lct_wheel_shift]

- **WHEN** the vertical wheel is scrolled with Shift held
- **THEN** the zoom step is twice the unmodified step, still anchored the same way
  (cursor for zoom-in, centre for zoom-out)

#### Scenario: Alt pans horizontally [lct_wheel_alt_pan]

- **WHEN** the vertical wheel is scrolled with Alt held (without Ctrl)
- **THEN** the canvas pans horizontally and does not zoom

#### Scenario: Ctrl+Alt pans vertically [lct_wheel_ctrl_alt_pan]

- **WHEN** the vertical wheel is scrolled with Ctrl+Alt held
- **THEN** the canvas pans vertically and does not zoom

#### Scenario: A side-wheel pans horizontally [lct_wheel_side_pan]

- **WHEN** a horizontal side-wheel event arrives
- **THEN** the canvas pans horizontally and does not zoom

#### Scenario: Panning cannot push the canvas fully off-screen [lct_pan_margin]

- **WHEN** the canvas is panned in any direction past the reveal margin
- **THEN** the offset is clamped so at least 96 logical pixels of the canvas stay
  visible on that axis and the canvas does not jump to the other side

#### Scenario: Zoom anchoring is clamped [lct_zoom_clamp]

- **WHEN** a zoom-out at a corner would place the canvas outside the reveal margin
- **THEN** the resulting offset is clamped by the same shared range helper

#### Scenario: Fit and centre remain fully visible [lct_fit_within]

- **WHEN** Fit on Screen or centre is applied
- **THEN** the resulting offset is inside the shared range and the canvas is not
  pushed off-screen

### Requirement: Space temporarily activates Hand panning

Holding the `Space` key over the canvas SHALL temporarily enable Hand panning
while the previous tool remains logically active: the cursor SHALL become
`Qt::OpenHand`, and pressing and dragging SHALL pan the canvas through the shared
range helper exactly as the Hand tool does. Releasing `Space` SHALL restore the
previous tool's cursor and pan state, so a Space pan does not change the active
tool, the tool's options, or any document state. The `Space` key SHALL NOT be
bound as a prefix of a key sequence, so it is delivered as a plain key and never
consumed waiting for a second key. While `Space` is held, this override SHALL
take precedence over every other cursor and pan update — including pointer
hover, a tool switch, and any document or tool-state refresh — so the hand
cursor and the pan-enabled state persist for the whole hold and no later update
reverts the cursor or disables panning. A cursor-mode update that lands mid-hold
SHALL NOT drop a subsequent pan or let the press reach the active tool. The hold
SHALL end only when the key is physically released: a key auto-repeat event SHALL
NOT be treated as a release, so holding `Space` keeps the override active.

#### Scenario: Space pans without changing the tool [lct_space_pan]

- **WHEN** `Space` is held and the canvas is dragged with a non-Hand tool active
- **THEN** the canvas pans, the active tool is unchanged, and the offset is
  clamped by the shared range

#### Scenario: Releasing Space restores the cursor [lct_space_release]

- **WHEN** `Space` is released after a pan
- **THEN** the previous tool's cursor and pan state are restored

#### Scenario: Space is not a sequence prefix [lct_space_not_prefix]

- **WHEN** `Space` is pressed and released on its own
- **THEN** it is handled as the temporary pan key and is not held waiting for a
  second key in a sequence

#### Scenario: The override survives a cursor refresh [lct_space_refresh]

- **WHEN** a cursor-mode update occurs while `Space` is held — a pointer hover,
  a tool switch, or a document or tool-state refresh
- **THEN** the cursor remains `Qt::OpenHand` and panning remains enabled, so the
  hand cursor does not flash and revert

#### Scenario: Hover does not clobber the hand cursor [lct_space_hover]

- **WHEN** the pointer moves over the canvas with `Space` held and no mouse
  button down
- **THEN** the cursor stays `Qt::OpenHand` and no tool hover or cursor state is
  applied

#### Scenario: A mid-hold update does not drop the pan [lct_space_midhold]

- **WHEN** a cursor-mode update lands while `Space` is held and the left button
  is then pressed and dragged
- **THEN** the canvas pans and no tool edit or drag begins, regardless of the
  tool that was active before the press

#### Scenario: Key auto-repeat does not end the override [lct_space_autorepeat]

- **WHEN** `Space` is held and a key auto-repeat event (a synthetic release that
  is not a physical release) arrives
- **THEN** the hand cursor and pan-enabled state persist and a subsequent drag
  still pans

#### Scenario: A transform session ends the override [lct_space_transform]

- **WHEN** a Free Transform session starts while `Space` is held
- **THEN** the transient hand override ends and the transform session's
  pan-disabled, modal state takes over

#### Scenario: A lost key-up releases the override [lct_space_focus]

- **WHEN** the canvas loses focus or the window deactivates while `Space` is held
- **THEN** the override ends so a missed key-up cannot leave every later press
  routed to panning

### Requirement: Keyboard nudge of the active layer

With the Move tool active, a plain arrow key SHALL translate the active layer by
1 document pixel on that axis and `Shift+Arrow` SHALL translate it by 10 pixels,
through the existing `translate_layer` path, recording one history state per
nudge and refreshing the display. The nudge SHALL be refused, leaving the
document unchanged with no history state, for a position-locked layer, a
Background layer, a group or adjustment layer, or when there is no active
document or active layer. The nudge SHALL use the same translate path the mouse
drag uses so the two cannot disagree; no separate nudge handler exists today and
none may be added that bypasses `translate_layer`.

#### Scenario: Plain arrow nudges one pixel [lct_nudge_one]

- **WHEN** the Move tool is active with a translatable active layer and an arrow
  key is pressed
- **THEN** the layer translates by one pixel on that axis and one history state
  is recorded

#### Scenario: Shift arrow nudges ten pixels [lct_nudge_ten]

- **WHEN** `Shift+Arrow` is pressed with the Move tool active
- **THEN** the layer translates by ten pixels on that axis

#### Scenario: A refused nudge changes nothing [lct_nudge_refused]

- **WHEN** an arrow key is pressed on a position-locked, Background, group, or
  adjustment layer, or with no active layer
- **THEN** the document is unchanged and no history state is recorded

### Requirement: Alt-drag duplicates the active layer

Holding Alt while the Move tool drags a whole layer (no active selection) SHALL
duplicate the layer, preview the duplicate moving under the cursor during the
drag, and make the duplicate the active layer on commit, leaving the source layer
in place. The bridge SHALL expose a `begin_move_duplicate` that duplicates the
active layer, recomposites, and builds the move preview without recording
history; the drag SHALL record exactly one undo state on `commit_move`. With an
active selection the behaviour is the selected-pixel duplicate of the
`selection-content-move` capability, not a whole-layer clone.

#### Scenario: Alt-drag clones the layer [lct_alt_clone]

- **WHEN** the Move tool drags a whole layer with Alt held
- **THEN** a duplicate layer is created and moves with the drag, the source layer
  keeps its pixels, and one history state is recorded on release

#### Scenario: The clone becomes active [lct_alt_clone_active]

- **WHEN** the Alt-drag is released
- **THEN** the duplicate is the active layer and the source remains in the stack

#### Scenario: No record during the drag [lct_alt_clone_record]

- **WHEN** `begin_move_duplicate` runs at press
- **THEN** no history state is recorded until the drag commits, which records
  exactly one

### Requirement: Alt press without movement does not duplicate

A Move-tool press with Alt held that does not move the pointer SHALL leave the
document unchanged: no duplicate layer and no history state. The duplicate SHALL
only come into existence once the pointer has actually moved, or SHALL be rolled
back on a zero-offset release. A non-zero Alt drag SHALL continue to insert the
clone, preview it, and commit one state with the clone active.

#### Scenario: A bare Alt click leaves no layer [mv_alt_zero_noop]

- **WHEN** the Move tool is pressed with Alt held and released at the same point
  without moving
- **THEN** no layer is added, the active layer is unchanged, and no history state
  is recorded

#### Scenario: The first movement creates the clone [mv_alt_first_move]

- **WHEN** an Alt Move press is followed by pointer movement before release
- **THEN** the clone is inserted and previewed, and release commits one state
  with the clone active

### Requirement: Free pan clamp policy

The canvas pan clamp SHALL be per axis. On each axis, when `view.freePan` is
checked (the default), the shared range helper SHALL behave as today: the reveal
margin, so at least 96 logical pixels of the document stay visible. When
`view.freePan` is unchecked, on each axis independently, if the scaled document
is not larger than the viewport the allowed offset range SHALL collapse to the
single centred offset `(viewport − scaledDocument) / 2`; if the scaled document
overflows the viewport on that axis the reveal-margin clamp SHALL be unchanged.
A zero-size or empty document SHALL keep today's behaviour (the collapse SHALL
NOT fire). Every mutation point that routes through the shared range helper —
pan, zoom anchoring, centre, fit, actual pixels, initial view, resize, and a
document-size replacement — SHALL honour the same policy.

#### Scenario: A fitting axis is constrained [lct_free_pan_constrained]

- **WHEN** Free Pan is off and the document fits the viewport along an axis
- **THEN** panning toward the backdrop leaves the offset at the centred position
  on that axis

#### Scenario: Mixed axes constrain only the fitting axis [lct_free_pan_mixed]

- **WHEN** Free Pan is off and the document fits one axis but overflows the
  other
- **THEN** the fitting axis is pinned to centre and the overflowing axis pans
  within the reveal margin

#### Scenario: Zoomed-in pan is unaffected [lct_free_pan_zoomed]

- **WHEN** Free Pan is off and the document overflows the viewport on both axes
- **THEN** panning uses the reveal-margin clamp exactly as it does with Free Pan
  on

#### Scenario: Toggling re-clamps immediately [lct_free_pan_toggle]

- **WHEN** Free Pan is unchecked while a fitting document is panned away from
  centre
- **THEN** the canvas re-clamps to the centred position at once and repaints

#### Scenario: A document resized to fit is re-clamped [lct_free_pan_resize_doc]

- **WHEN** Free Pan is off and a document that overflowed the viewport is reduced
  to fit by an image or canvas resize
- **THEN** the canvas re-clamps to the centred position rather than keeping its
  old off-centre offset

#### Scenario: An empty document is unaffected [lct_free_pan_empty]

- **WHEN** Free Pan is off and the canvas has no document
- **THEN** the clamp keeps today's behaviour and no centring offset is produced

### Requirement: Zoom tool context menu

Right-clicking the Zoom tool's toolbox slot SHALL open a menu with **Fit on
Screen**, **100%**, **200%**, **Print Size**, a separator, **Zoom In**, and
**Zoom Out**. Each entry SHALL apply to the active document's canvas and SHALL be
disabled when no document is open. Fit on Screen, 100%, Zoom In, and Zoom Out
SHALL match the View menu commands; 200% SHALL set the zoom to 200%; Print Size
SHALL set the zoom so the document displays at its print size, one image inch per
logical screen inch, from the document's resolution (CS6's 72 ppi default). The
Zoom slot SHALL show no member flyout triangle.

#### Scenario: The Zoom slot opens the preset menu [ztm_menu]

- **WHEN** the Zoom tool's toolbox slot is right-clicked
- **THEN** a menu with Fit on Screen, 100%, 200%, Print Size, Zoom In, and Zoom
  Out opens, and the slot shows no member flyout triangle

#### Scenario: A preset applies to the canvas [ztm_apply]

- **WHEN** 200%, 100%, Print Size, or Fit on Screen is chosen with a document open
- **THEN** the active canvas zooms to 200%, 100%, the print-size zoom
  (`96 / ppi`), or the fit zoom respectively

### Requirement: Workspace snap clamp policy

When `view.snapCanvasToWorkspace` is checked and the scaled document fits
entirely inside the viewport on both axes, the shared range helper SHALL collapse
both axes to the single centred offset `(viewport − scaledDocument) / 2`, so the
canvas is pinned to the workspace centre and panning cannot move it. While the
scaled document overflows either axis, the helper SHALL behave exactly as if the
flag were unchecked, so the free-pan policy still governs and panning works. A
zero-size or empty document SHALL be unaffected. Every mutation point that routes
through the shared range helper — pan, zoom anchoring, centre, fit, actual
pixels, initial view, resize, and a document-size replacement — SHALL honour the
policy, and the canvas SHALL remain the single source of the offset.

#### Scenario: A fitting document is pinned to centre [lct_ws_snap_pinned]

- **WHEN** Snap Canvas To Workspace is on and the scaled document fits the
  viewport on both axes
- **THEN** the offset is the workspace centre on both axes and a pan attempt
  leaves it unchanged

#### Scenario: An overflowing document pans normally [lct_ws_snap_overflow]

- **WHEN** Snap Canvas To Workspace is on and the scaled document overflows the
  viewport on an axis
- **THEN** the canvas pans through the reveal-margin clamp exactly as it does
  with the flag off

#### Scenario: The toggle re-clamps immediately [lct_ws_snap_toggle]

- **WHEN** Snap Canvas To Workspace is turned on while a fitting document is
  panned away from centre
- **THEN** the canvas re-clamps to the centred position at once and repaints

#### Scenario: An empty document is unaffected [lct_ws_snap_empty]

- **WHEN** Snap Canvas To Workspace is on and the canvas has no document
- **THEN** the clamp keeps today's behaviour and no centring offset is produced

### Requirement: Move preview freshness

The Move tool's live preview SHALL place the moving layers at the pointer's
offset. A prepared preview SHALL be reused only while the moving layers have not
moved: a rendered link set's preview image is composited at the layers'
positions and clipped to the canvas, so a move makes the cached image stale and
the next drag SHALL recompute it rather than draw the stale image at the new
origin.

#### Scenario: A moved link set recomputes its preview [lct_move_preview_fresh]

- **WHEN** a linked set is moved and the Move tool begins another drag without
  moving it again
- **THEN** the moved set's preview is recomputed at the new union rect and the
  set previews and lands at the pointer's offset

### Requirement: Eyedropper samples continuously while dragging

While the Eyedropper tool's press gesture is live, the system SHALL sample the
composited colour again at each pointer position over the document and update
the foreground colour, so the colour follows a held drag until the button is
released. A pointer move with no button held SHALL NOT sample.

#### Scenario: A held drag samples continuously [ct_eyedropper_drag]

- **WHEN** the Eyedropper is pressed over one colour and dragged with the left
  button held across a different colour
- **THEN** the foreground colour becomes the colour under the pointer after
  each move, and a move after the release leaves it unchanged
