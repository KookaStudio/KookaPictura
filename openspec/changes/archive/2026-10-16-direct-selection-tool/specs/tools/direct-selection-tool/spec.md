## ADDED Requirements

### Requirement: Direct Selection tool

Dragging a Work Path anchor SHALL move it with its handles, leaving the other
anchors in place, and record exactly one "Drag Anchor Point" state. Dragging a
handle of the selected component SHALL move it and record one "Drag Direction
Point" state; on a smooth point the opposite handle SHALL keep its length and
swing to stay collinear through the anchor. A click that does not move SHALL
record nothing. Alt-clicking a component SHALL select it whole, so Delete
removes it.

#### Scenario: Reshape a square

- **WHEN** the `tst_path_selection_tools` test drags a corner and a smooth anchor of a closed square, drags one handle of the smooth point, clicks an anchor, Alt-clicks inside, and presses Delete
- **THEN** each anchor drag records one "Drag Anchor Point" state moving only that anchor (the smooth one with its handles), the handle drag records one "Drag Direction Point" state with the opposite handle collinear at its old length, the click records nothing, and Delete removes the square as one "Delete Path" state
