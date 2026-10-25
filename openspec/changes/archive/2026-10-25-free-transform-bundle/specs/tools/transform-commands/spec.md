# Spec Delta

## MODIFIED Requirements

### Requirement: Transform Again

`Edit > Transform > Again` SHALL repeat the last committed similarity transform
on the current target, including the reference point's source-frame offset from
the source rect centre and the translation, and SHALL record one `Transform
Again` state. The recorded offset SHALL be the pivot's place in the item's
source frame, so a pivot automatically carried by a scale or translation does
not add a second offset to the repeat. It SHALL be disabled until a repeatable
transform has been committed and while a session is open.

#### Scenario: Again repeats the last transform [tc_again]

- **WHEN** Rotate 90° Clockwise is invoked and then Again
- **THEN** the layer is rotated 180° in total and the second state is `Transform Again`

#### Scenario: Again is unavailable before a transform [tc_again_disabled]

- **WHEN** a document is open and no transform has been committed
- **THEN** Again is disabled

#### Scenario: Again reuses the source-frame pivot offset [tc_again_pivot]

- **WHEN** a Free Transform moves the layer, the pivot is set to the moved box's top-left corner, the transform is committed, and Again runs on the layer
- **THEN** the repeated transform keeps the pivot at the same place on the layer (the repeated box's top-left corner), with no extra shift from the carried pivot
