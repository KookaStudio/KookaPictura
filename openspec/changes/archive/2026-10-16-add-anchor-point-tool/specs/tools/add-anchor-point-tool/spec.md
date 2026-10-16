## ADDED Requirements

### Requirement: Add Anchor Point tool

A click on a Work Path segment SHALL insert an anchor at the nearest point of
the segment without changing the path's shape, recording exactly one "Add
Anchor Point" state; a click away from the path SHALL record nothing.

#### Scenario: Splitting a segment

- **WHEN** the `tst_pen_tools` test clicks the middle of a square's right edge with Add Anchor Point
- **THEN** one "Add Anchor Point" state is recorded and the new anchor lies under the click
