## ADDED Requirements

### Requirement: Clone source transform

`pictura_paint::stamp::StampSource::transformed` SHALL map each destination
pixel to its source through the inverse of a scale (W / H, negative for Flip
Horizontal / Vertical) followed by a counter-clockwise rotation about the
point the offset was measured at, sampling the source bilinearly and leaving
a pixel unchanged when its source is off the image. An identity transform
SHALL behave exactly as the untransformed source.

#### Scenario: Flip mirrors about the anchor

- **WHEN** a mark lies 6 px right of the source point and Flip Horizontal is on
- **THEN** it paints 6 px left of the anchor

#### Scenario: Rotation is counter-clockwise on screen

- **WHEN** the source is rotated 90°
- **THEN** a mark right of the source point paints above the anchor

### Requirement: Clone Source panel

`Window > Panels > Clone Source` SHALL show or hide a Clone Source panel with
five source slots, each keeping its own Alt-clicked source, measured offset,
and transform; the Clone Stamp SHALL use the active slot. The panel SHALL show
the offset as destination minus source (editable once the slot has a source),
W and H (with Maintain Aspect Ratio on by default), Rotate, Flip Horizontal,
Flip Vertical, and Reset Transform (100 % / 100 % / 0°, no flips). The overlay
controls and the frame controls SHALL be shown disabled as not implemented.

#### Scenario: Slots keep their own sources

- **WHEN** the `clone_source_panel` self-test Alt-clicks in slot 2 and strokes in slot 1
- **THEN** the stroke is refused without a history state

#### Scenario: Flip and offset through the panel

- **WHEN** Flip Horizontal is pressed and a stroke starts 35 px right of the source
- **THEN** the cloned pixels mirror about the stroke start and Offset X reads 35

### Requirement: Clone Stamp panel toggles

The Clone Stamp options bar SHALL carry Toggle the Brush panel and Toggle the
Clone Source panel buttons after the tip fields; each SHALL show its panel when
hidden and hide it when shown.

#### Scenario: A toggle flips the panel

- **WHEN** Toggle the Brush panel is clicked twice
- **THEN** the Brush panel's visibility changes and then returns to what it was
