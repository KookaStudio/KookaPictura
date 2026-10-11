# Spec Delta

## MODIFIED Requirements

### Requirement: Crop tool

The system SHALL place a crop box over the whole canvas when the Crop tool is
chosen (fitted to the aspect ratio when one is set), shading the canvas area
outside it — not the workspace around the canvas — and drawing rule-of-thirds
guides, its frame, and eight handles. Selecting the tool SHALL show a preview
box (dashed, no guides, no handles) in Modern, fitted to the ratio and centered,
and no box in Classic. Dragging a handle of an active box SHALL resize the box
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
document to the box as one "Crop" state, applying any straighten angle; Escape
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
