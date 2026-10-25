# brush-tools Specification

## Purpose
Brush and Pencil tools that paint the foreground colour through the stroke engine, with their options bar and shortcuts.

## Requirements

### Requirement: Brush and Pencil tools paint the foreground colour

The system SHALL provide Brush and Pencil as selectable tools that paint the
current foreground colour into the active layer of the active document, where the
active layer is the exactly-one layer resolved by the shared active-layer
resolver. Painting SHALL be refused, leaving the document unchanged, when no layer
is active, when more than one layer is selected, or when the active layer is not
a raster layer; it SHALL NOT fall back to the topmost raster layer. Painting on a
layer whose lock state includes `TRANSPARENCY` SHALL change only the color of
pixels whose pre-existing alpha is greater than zero and SHALL preserve each such
pixel's alpha.

#### Scenario: Painting marks the active layer

- **WHEN** the Brush tool drags over the canvas and a single raster layer is
  active
- **THEN** the composite updates with the foreground colour along the drag on
  that layer

#### Scenario: No active layer

- **WHEN** a paint tool drags with no layer active or with more than one layer
  selected
- **THEN** the stroke is refused, nothing changes, and the document is not
  corrupted

#### Scenario: A transparency-locked layer keeps its alpha [lbt_paint_alpha]

- **WHEN** the Brush paints on a transparency-locked layer
- **THEN** each painted pixel's colour changes and its alpha equals its
  pre-stroke value

### Requirement: Paint options bar

The options bar SHALL expose, for Brush and Pencil, a size, hardness, opacity,
flow, and paint-mode control, and for Pencil an Auto Erase toggle. Changing a
control SHALL affect the next stroke and SHALL NOT change the active tool. The
numeric paint controls SHALL use the shared numeric field control, each with a
scrubbing label and a slider popup, and a change to the size SHALL be reflected
in every control bound to the brush size, including the `[`/`]` shortcuts and the
brush-size outline.

#### Scenario: Options follow the paint tool

- **WHEN** the Brush or Pencil tool becomes active
- **THEN** the options bar shows the paint controls

#### Scenario: Changed option applies to the next stroke

- **WHEN** the size is changed before a stroke
- **THEN** the next stroke is painted at the new size

#### Scenario: The size control and shortcuts stay in sync [lbt_size_sync]

- **WHEN** the brush size is changed from the options bar or with `[`/`]`
- **THEN** the other bound control and the brush-size outline show the new size

### Requirement: Canvas pointer routing to the stroke engine

The system SHALL map canvas pointer press, move, and release into a stroke: press
SHALL begin a stroke, move SHALL add samples, and release SHALL commit it. The
image-space coordinates of every sample SHALL be passed to the engine.

#### Scenario: A drag paints one stroke

- **WHEN** the user presses, drags, and releases with a paint tool active
- **THEN** exactly one stroke is committed covering the drag path

### Requirement: Brush size and hardness shortcuts

The system SHALL change the brush diameter with `[` and `]` and the hardness
with `Shift+[` and `Shift+]` for the Brush tool, clamped to the documented
ranges.

#### Scenario: Diameter shortcut

- **WHEN** `]` is pressed with the Brush tool active
- **THEN** the brush diameter increases by one step, clamped to the maximum

#### Scenario: Hardness shortcut

- **WHEN** `Shift+[` is pressed with the Brush tool active
- **THEN** the brush hardness decreases by one step, clamped to zero

### Requirement: Brush-size outline overlay

While the Brush or Pencil tool is active over the canvas, the system SHALL draw a
circle outline at the pointer position sized to the current brush diameter in
image space, so the outline scales with zoom, and SHALL update it as the brush
size or the pointer position changes. The outline SHALL be a drawn overlay, not an
OS cursor pixmap, and SHALL be hidden when a paint tool is not active or the
pointer leaves the canvas. The outline SHALL NOT be clipped to the document's
image rectangle: it SHALL render outside the canvas bounds while remaining inside
the canvas widget, so it stays visible when the pointer is near or beyond the
document edge.

#### Scenario: The outline tracks the brush size [lbt_outline_size]

- **WHEN** the brush size changes while a paint tool hovers the canvas
- **THEN** the drawn circle's diameter changes to match

#### Scenario: The outline scales with zoom [lbt_outline_zoom]

- **WHEN** the canvas is zoomed while a paint tool hovers the canvas
- **THEN** the circle is drawn at the brush diameter in image space, so its
  on-screen size scales with the zoom

#### Scenario: The outline renders outside the document [lbt_outline_outside]

- **WHEN** the brush pointer is near or beyond the document edge
- **THEN** the portion of the circle outside the document bounds is still drawn,
  bounded only by the canvas widget

#### Scenario: No outline for other tools [lbt_outline_hidden]

- **WHEN** a tool other than Brush or Pencil is active, or the pointer leaves the
  canvas
- **THEN** no brush-size circle is drawn

### Requirement: Brush and Pencil hide the mouse cursor

While Brush or Pencil is active over the canvas, the system SHALL set a blank
mouse cursor (`Qt::BlankCursor`) rather than a brush/pencil cursor pixmap or an
unset cursor, so the drawn brush-size ring is the only pointer affordance. The
ring SHALL remain driven by the existing outline overlay.

#### Scenario: The paint cursor is blank [lbt_blank_cursor]

- **WHEN** Brush or Pencil is active over the canvas
- **THEN** the mouse cursor is blank and the brush-size ring is drawn

#### Scenario: The cursor is restored for other tools [lbt_blank_cursor_restore]

- **WHEN** a non-paint tool becomes active or the pointer leaves the canvas
- **THEN** the blank cursor is cleared and the tool's normal cursor is used

### Requirement: A stroke is continuously visible while dragging

While a paint stroke is in progress, every dab SHALL become visible on the
canvas as it is applied, before the mouse button is released, at the correct
document location under the current pan and zoom. The live path
(`paint_dab` → region refresh → canvas region blit) SHALL keep the scaled
present cache correct at every zoom, not only at 100 %, so a large document
whose initial view is zoomed to fit still shows the growing stroke. The stroke
SHALL write its pixel data and record exactly one history state only on release;
releasing SHALL NOT be the first moment the stroke is visible.

#### Scenario: Dabs show during the drag [lbt_live_visible]

- **WHEN** the Brush drags across the canvas with the button held
- **THEN** each dab is visible on the canvas before release, at the cursor's
  document position

#### Scenario: Live visibility at a non-100% zoom [lbt_live_visible_zoom]

- **WHEN** a large document is shown zoomed to fit (zoom below 100 %) and the
  Brush drags
- **THEN** the dabs are still visible at the correct on-screen position while
  the button is held

#### Scenario: One history state only on release [lbt_live_commit]

- **WHEN** a multi-dab stroke completes
- **THEN** exactly one history state is recorded by `end_paint`, and an undo
  restores the pre-stroke image

### Requirement: Invisible active layer refuses paint but allows selection and copy

When the active layer is invisible, paint and filter edits SHALL be refused with
a user-visible refusal and the canvas SHALL show a Block/Forbidden cursor over
the paint target, while selection, copy, and Move remain available (see the
`layer-management` requirement for the Move contract). The refusal SHALL leave
the document unchanged and record no history state. The policy SHALL be resolved
through one shared `active_layer_visible` helper so paint, filters, and the
cursor branch agree.

#### Scenario: Painting an invisible layer is refused [lbt_invisible_refuse]

- **WHEN** the Brush presses on the canvas with an invisible active layer
- **THEN** no stroke begins, the document is unchanged, no history state is
  added, and the refusal is reported

#### Scenario: The cursor marks the refusal [lbt_invisible_cursor]

- **WHEN** a paint tool hovers with an invisible active layer
- **THEN** the canvas shows a Block/Forbidden cursor instead of the blank paint
  cursor

#### Scenario: Selection and copy still work [lbt_invisible_select]

- **WHEN** the active layer is invisible and a selection or copy command runs
- **THEN** it operates on the layer's content and is not refused

### Requirement: Transient Eyedropper on Alt

Holding Alt while a paint tool is active SHALL transiently switch to the
Eyedropper: the canvas SHALL sample the composited colour under the pointer and
show the Alt/eyedropper cursor, and the sample SHALL update the foreground
colour. Releasing Alt SHALL restore the previous paint tool, its options, and
its cursor without permanently changing the active tool or recording history.

#### Scenario: Alt samples without switching tools [lbt_alt_sample]

- **WHEN** Alt is held with the Brush active and the pointer is over the canvas
- **THEN** the foreground colour becomes the sampled colour and the active tool
  remains the Brush

#### Scenario: Alt shows the eyedropper cursor [lbt_alt_cursor]

- **WHEN** Alt is held with a paint tool active over the canvas
- **THEN** the canvas shows the eyedropper/Alt cursor rather than the blank paint
  cursor

#### Scenario: Releasing Alt restores the tool [lbt_alt_restore]

- **WHEN** Alt is released
- **THEN** the paint tool's blank cursor and ring behaviour are restored and no
  stroke or history state was created

### Requirement: Brush size shortcuts bind by native scan code

The `[` and `]` brush-size shortcuts SHALL work on EU/Scandinavian keyboard
layouts as well as US layouts, by matching `QKeyEvent::nativeScanCode` for the
evdev scan codes for `[` (34) and `]` (35) and their Shift variants, in addition
to the US key values, and SHALL be guarded to the Brush and Pencil tools only.
The mapping from a key event to a brush-size delta SHALL be a single pure
helper so it is unit-testable without a Qt event loop.

#### Scenario: Bracket on a US layout changes the size [lbt_bracket_us]

- **WHEN** `]` is pressed with the Brush active on a US layout
- **THEN** the brush diameter increases by one step

#### Scenario: Bracket on a Nordic layout changes the size [lbt_bracket_native]

- **WHEN** the physical `]` key (evdev scan code 35) is pressed with the Brush
  active on an EU/Scandinavian layout
- **THEN** the brush diameter changes by the same step as on a US layout

#### Scenario: Shift bracket changes hardness [lbt_bracket_shift]

- **WHEN** `Shift` and the physical `[` key are pressed with the Brush active
- **THEN** the brush hardness decreases by one step

#### Scenario: Other tools ignore the bracket keys [lbt_bracket_guard]

- **WHEN** `[` or `]` is pressed with a non-paint tool active
- **THEN** the brush size is unchanged

### Requirement: Brush size shortcut mapping is a testable helper

The native-scan-code and key-value to brush-size delta mapping SHALL be a pure
function with unit tests covering the US and Nordic key codes, the Shift
variants, and the non-paint guard, so the keyboard-layout behaviour does not
depend on a running Qt application.

#### Scenario: The helper maps both layouts [lbt_bracket_helper]

- **WHEN** the helper is called with a US `]` key event and with an evdev-35
  event
- **THEN** both return the same positive diameter delta

### Requirement: Auto Erase is Pencil-only

The Pencil's Auto Erase option SHALL affect Pencil strokes only. The Brush,
Eraser, stamps, and retouch tools SHALL ignore the Auto Erase state even when
the Pencil left it enabled. A Brush stroke that starts over the foreground
colour SHALL paint the foreground colour and record one "Brush" history state.

#### Scenario: Auto Erase does not leak into the Brush [lbt_auto_erase_pencil_only]

- **WHEN** Auto Erase is enabled on the Pencil and the Brush then paints over a
  pixel matching the foreground colour
- **THEN** the Brush paints the foreground colour and records exactly one
  "Brush" history state

### Requirement: The brush outline is hidden during a transform session

While a Free Transform, Scale, Rotate, Skew, Distort, Perspective, or Warp
session is open on the canvas, the system SHALL NOT draw the brush-size ring,
even when Brush, Pencil, or the Eraser is the active tool. Entering the session
SHALL clear an outline that is already drawn before the first transform frame
is painted, and a zoom or brush-size change during the session MUST NOT draw it
again. The outline SHALL be available again on the next hover after the session
ends.

#### Scenario: Entering Free Transform clears the ring [bt_transform_clears]

- **WHEN** the Brush tool is active with its ring drawn and Free Transform opens
- **THEN** the ring is gone in the first painted transform frame

#### Scenario: A zoom during the session keeps the ring hidden [bt_transform_zoom]

- **WHEN** the canvas is zoomed while a transform session is open
- **THEN** no brush ring is drawn

#### Scenario: The ring returns after the session [bt_transform_restores]

- **WHEN** the session is committed or cancelled and the pointer next hovers the canvas
- **THEN** the brush ring is drawn again at the pointer
