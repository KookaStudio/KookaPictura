# tools/shape-layer-transform Specification

## Purpose

Moving and transforming a shape layer: the Move tool and the `Edit > Transform`
commands map the shape's outline instead of resampling pixels.

## ADDED Requirements

### Requirement: The Move tool moves a shape layer

The Move tool SHALL drag the active shape layer: the commit SHALL translate
every anchor and handle of its outline by the drag, keep a live shape live,
and record one `Move Layer` history state. The Move tool's transform controls
box SHALL hug the outline's bounds. A position-locked shape layer SHALL NOT
move.

#### Scenario: A drag moves the shape [slt_move]

- **WHEN** the Move tool drags from inside a shape layer by (20, 15)
- **THEN** the shape's fill is drawn 20 px right and 15 px down, the layer is still a shape layer, and one `Move Layer` state is recorded
- **AND** the controls box follows the moved outline, and undo puts the shape back

#### Scenario: A move keeps a live shape live [slt_move_live]

- **WHEN** `translate_layer_path` moves a live rectangle shape layer
- **THEN** the outline and the live shape's bounds move by the same offset and the layer's rect is unchanged

#### Scenario: A position lock refuses the move [slt_lock]

- **WHEN** a position-locked shape layer is translated or transformed
- **THEN** the call returns false and the document is unchanged

### Requirement: Transforms map a shape layer's outline

Free Transform and the `Edit > Transform` commands SHALL accept a visible,
unlocked shape layer whose outline has area. The session's box SHALL be the
outline's bounds. The commit SHALL map the outline's anchors and handles
through the session's similarity, projective, or warp map, so the layer stays a
shape layer with no pixel channels added. Any map other than a pure translation
SHALL leave a regular (not live) path.

#### Scenario: A quarter turn rotates the outline [slt_rotate]

- **WHEN** `Edit > Transform > Rotate 90° Clockwise` runs on a 20×10 rectangle shape centred at (20, 15)
- **THEN** one history state is recorded, the layer is still a shape layer, and its box is 10×20 about the same centre

#### Scenario: Free Transform's box hugs the outline [slt_free]

- **WHEN** Free Transform begins on a shape layer and its width is set to 200%
- **THEN** the session's box is the outline's bounds, not the canvas, and the commit doubles the outline's width about its centre

#### Scenario: Quad and warp maps move the outline [slt_quad_warp]

- **WHEN** `transform_layer_quad` or `transform_layer_warp` runs on a shape layer
- **THEN** the outline's box lands on the target quad and the layer gains no pixel channels

#### Scenario: A shape layer is a transform target [slt_target]

- **WHEN** Free Transform's target check runs on a shape layer, and then on the same layer position-locked
- **THEN** the first is accepted and the second is refused
