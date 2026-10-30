## ADDED Requirements

### Requirement: The first-run layout is built once

When no panel arrangement is stored, constructing the main window SHALL arrange
the panels in the Essentials factory layout directly, without first building a
different arrangement and then regrouping it. The result SHALL equal the
arrangement that applying the Essentials workspace produces: the same columns,
the same groups in the same order, the same panel order within each group, the
same iconic and visible state, and the same active tab in each group.

#### Scenario: the first-run layout equals the applied Essentials layout [wsp_first_run_layout]

- **WHEN** a main window is constructed from an empty session and its arrangement is recorded, and the Essentials workspace is then applied to it
- **THEN** the arrangement after the application equals the recorded one: columns, groups and their order, panel order within each group, iconic and visible state, and each group's active tab
