## ADDED Requirements

### Requirement: Source stroke engine

`pictura_paint::Stroke::begin_source` SHALL start a Brush stroke whose colour
at each pixel is read from a document-space source image at the pixel's
document position plus the source offset; coverage, spacing, flow, opacity,
mode, and the transparency lock SHALL behave as for the Brush. A pixel whose
source falls outside the image SHALL be left unchanged. A layer without an
alpha channel SHALL be treated as opaque by every Brush stroke.

#### Scenario: A clone stroke copies at its offset

- **WHEN** a source stroke with offset (−16, 0) paints over the blue right half of a red / blue layer
- **THEN** the painted pixels are red and pixels outside the brush are unchanged

#### Scenario: An off-image source leaves the layer alone

- **WHEN** a source stroke's offset points past the image edge
- **THEN** the painted pixels keep their colour

#### Scenario: A locked Background takes paint

- **WHEN** a Brush stroke paints a transparency-locked layer without an alpha channel
- **THEN** the pixels take the paint, and a soft edge blends with the colour beneath

### Requirement: Clone sampling scope

`pictura_paint::stamp::sample_scope` SHALL return no document for Current
Layer (the layer itself is sampled), the document without the layers above the
active one for Current And Below, and the whole document for All Layers, with
adjustment layers removed when Ignore Adjustment Layers is on.

#### Scenario: Current And Below drops the layers above

- **WHEN** the bottom layer of a three-layer stack is sampled Current And Below
- **THEN** the scope holds one layer

#### Scenario: Ignore Adjustment Layers

- **WHEN** a stack with an adjustment layer is sampled All Layers with Ignore Adjustment Layers on
- **THEN** the adjustment layer is not in the scope

### Requirement: Clone Stamp tool

The Clone Stamp SHALL set its source point on Alt-click and paint the sampled
surface, snapshotted when each stroke begins, at the source offset, recording
exactly one "Clone Stamp" history state per stroke that changed pixels. With
Aligned on the offset SHALL persist across strokes; with it off every stroke
SHALL restart at the source point. A stroke without a source point, or at a
zero offset, SHALL be refused without a history state. The options bar SHALL
offer Size, Hardness, Mode, Opacity, Flow, Aligned (default on), Sample
(default Current Layer), and Ignore Adjustment Layers, shown only with All
Layers.

#### Scenario: A stroke before Alt-click is refused

- **WHEN** the `clone_stamp_tool` self-test strokes before setting a source
- **THEN** no history state is recorded

#### Scenario: A stroke copies the source

- **WHEN** a stroke 30 px right of an Alt-clicked red source paints white
- **THEN** one "Clone Stamp" state is recorded and the painted pixels are red

#### Scenario: Aligned keeps the offset

- **WHEN** a second stroke starts 10 px further right with Aligned on
- **THEN** it copies the pixels 30 px to its left, not the source point
