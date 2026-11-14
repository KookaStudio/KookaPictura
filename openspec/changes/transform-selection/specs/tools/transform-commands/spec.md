# tools/transform-commands Specification Delta

## MODIFIED Requirements

### Requirement: Transform Again

`Edit > Transform > Again` SHALL repeat the last committed similarity transform
on the current target, including the reference point's source-frame offset from
the source rect centre and the translation, and SHALL record one `Transform
Again` state. The current target SHALL be the current selection border when the
last committed transform targeted a selection border and a non-empty selection
is active; otherwise it SHALL be the current layer. The recorded offset SHALL be
the pivot's place in the item's source frame, so a pivot automatically carried
by a scale or translation does not add a second offset to the repeat. It SHALL
be disabled until a repeatable transform has been committed, while a session is
open, when the last transform was projective or warped (not repeatable), and
when the last target was a selection border with no active selection.

#### Scenario: Again repeats the last transform [tc_again]

- **WHEN** Rotate 90° Clockwise is invoked and then Again
- **THEN** the layer is rotated 180° in total and the second state is `Transform Again`

#### Scenario: Again is unavailable before a transform [tc_again_disabled]

- **WHEN** a document is open and no transform has been committed
- **THEN** Again is disabled

#### Scenario: Again reuses the source-frame pivot offset [tc_again_pivot]

- **WHEN** a Free Transform moves the layer, the pivot is set to the moved box's top-left corner, the transform is committed, and Again runs on the layer
- **THEN** the repeated transform keeps the pivot at the same place on the layer (the repeated box's top-left corner), with no extra shift from the carried pivot

#### Scenario: Again repeats a border transform on the selection [tc_again_selection]

- **WHEN** a Transform Selection commit rotates the selection border 90° and Again runs
- **THEN** the border is rotated 180° in total, the selection remains the target, and the second state is `Transform Again`

## ADDED Requirements

### Requirement: Warp takes over an open transform session

`Edit > Transform > Warp` SHALL enter Warp mode on an open modal transform
session instead of cancelling it and starting a layer session, so a pending
layer or selection-border transform keeps its target, box, and applied
gestures. With no open session it SHALL keep beginning a Warp session on the
current layer or selected pixels.

#### Scenario: Warp keeps an open border session [tc_warp_open]

- **WHEN** `Select > Transform Selection` has a pending scale and `Edit > Transform > Warp` is invoked
- **THEN** the same session enters Warp mode with its box and pending scale intact
