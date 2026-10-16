## ADDED Requirements

### Requirement: Delete Anchor Point tool

A click on a Work Path anchor SHALL remove it, recording exactly one "Delete
Anchor Point" state; removing a subpath's last anchor SHALL remove the subpath,
and a click away from every anchor SHALL record nothing.

#### Scenario: Removing anchors

- **WHEN** the `tst_pen_tools` test clicks an anchor with Delete Anchor Point, then every anchor of the square
- **THEN** each click records one "Delete Anchor Point" state and the Work Path ends with no subpaths
