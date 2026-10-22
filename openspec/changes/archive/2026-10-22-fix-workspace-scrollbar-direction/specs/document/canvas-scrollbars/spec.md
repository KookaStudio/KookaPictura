# Spec Delta

## ADDED Requirements

### Requirement: Scrollbars move the canvas against the handle

The workspace scrollbars SHALL map their value onto the canvas offset so that the
handle travels against the content. A bar's value SHALL be the distance from the
shared range's maximum offset, so the bar's minimum shows the start of the
document and its maximum shows its end, and raising the value SHALL lower the
canvas offset on that axis. The bar's range SHALL remain the range the shared
range helper derives, so every position the canvas can be panned to — the
reveal margin beyond the document included — has a handle position, and the
handle SHALL rest at the end of its track only when the canvas has reached the
pannable limit. The canvas offset SHALL keep its meaning as the document's
top-left in widget space; the pan, the zoom, the rulers and the Navigator SHALL
be unaffected.

#### Scenario: The handle travels against the content [lcs_direction]

- **WHEN** the user raises a scrollbar's value
- **THEN** the canvas offset on that axis falls, so the content travels against
  the handle

#### Scenario: The bar's ends are the document's ends [lcs_ends]

- **WHEN** the user reads a scrollbar at its minimum and at its maximum
- **THEN** the canvas offset is the shared range's maximum at the bar's minimum,
  so the start of the document is visible, and the shared range's minimum at the
  bar's maximum, so the end of the document is visible

#### Scenario: The bar covers the reveal margin [lcs_margin]

- **WHEN** the canvas is panned into the reveal margin beyond the document
- **THEN** the scrollbar's value moves with the canvas and reaches the end of
  its range only when the canvas reaches the pannable limit
