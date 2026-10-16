## ADDED Requirements

### Requirement: Layer row drop bands

During a layer drag the Layers panel SHALL resolve the release point on a row by
the row's kind. A row that is not a group SHALL be a sibling target across its
whole height: a release in its upper half SHALL drop above it and a release in
its lower half SHALL drop below it, and it SHALL NOT resolve to a drop into the
row. A group row SHALL resolve a release within its upper quarter (at least
2 px) to above, within its lower quarter (at least 2 px) to below, and anywhere
between to a drop into the group.

#### Scenario: A release inside a layer row reorders [lpr_drop_row_halves]

- **WHEN** layer A is dragged and released a quarter of the way down layer B's row
- **THEN** A moves directly above B in one undo step

#### Scenario: The lower half of a layer row drops below it

- **WHEN** a layer is dragged and released three quarters of the way down another layer's row
- **THEN** it moves directly below that layer

#### Scenario: The centre of a group row still drops into it

- **WHEN** a layer is dragged and released at the vertical centre of a group row
- **THEN** it becomes a child of that group
