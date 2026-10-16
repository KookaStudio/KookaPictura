## ADDED Requirements

### Requirement: Content-Aware Move engine

`pictura_paint::healing::move_layer` SHALL move the pixels under a
document-sized selection mask on one pixel layer by a drag offset. In Move
mode it SHALL rebuild the vacated region with Content-Aware synthesis and copy
the original pixels verbatim to the target, blended by coverage; in Extend mode
it SHALL copy them and leave the original in place. A zero drag, an empty or
mis-sized mask, or a target entirely off the layer SHALL be a no-op, and a
pixel-locked or non-raster layer SHALL be refused. The result SHALL be
deterministic.

#### Scenario: Move relocates and fills

- **WHEN** `move_layer` moves a dark blob 40 pixels right on a light field
- **THEN** the blob's pixels appear unchanged at the target and the vacated area returns near the field value

#### Scenario: Extend keeps the original

- **WHEN** the same drag runs in Extend mode
- **THEN** the original blob is unchanged and a copy appears at the target

### Requirement: Adaptation

The Content-Aware synthesis SHALL take an Adaptation level (Very Strict,
Strict, Medium, Loose, Very Loose; default Medium) that sets its patch size and
search reach, stricter levels matching larger patches over a smaller reach.
Medium SHALL reproduce the previous fixed synthesis.

#### Scenario: Every level fills deterministically

- **WHEN** a move runs twice at each Adaptation level
- **THEN** both runs are identical and the vacated area is filled

### Requirement: Content-Aware Move tool

The Content-Aware Move tool SHALL trace a freehand outline when dragged from
outside the selection and SHALL move the selection's pixels when dragged from
inside it, previewing the outline at the drag offset, moving the selection with
the pixels, and recording exactly one "Content-Aware Move" history state on a
release that moved. Its options bar SHALL offer Mode (Move / Extend) and
Adaptation. Its cursor SHALL be an arrow whose tip is the hotspot.

#### Scenario: Outline then move

- **WHEN** the `content_aware_move` self-test outlines a black subject on white and drags it 30 pixels right
- **THEN** one "Content-Aware Move" state is recorded, the subject is at the target, its old place is near white, and the selection is at the target
