## ADDED Requirements

### Requirement: Count groups

`Document::annotations` SHALL hold Count groups, each with a name, an eye
visibility, a colour, a marker size (1–10), a label size (8–72), and its own
marks numbered in placement order. At least one group SHALL always exist; the
last group SHALL NOT be deletable. Adding a group SHALL make it active. Count
groups and marks SHALL be undoable through the history snapshot and SHALL never
change pixels.

#### Scenario: Independent numbered counters

- **WHEN** two marks are added to the default group and one to a new second group
- **THEN** the first group counts two and the second counts one, and hiding the first leaves only the second's mark visible

#### Scenario: The last group survives

- **WHEN** the only remaining group is deleted
- **THEN** the deletion is refused and one group remains

### Requirement: Count tool

The Count (Extended) tool SHALL add a numbered mark to the active group at a
click on empty canvas as one "New Count" state, move a dragged mark as one
"Move Count" state on release, and delete a mark on Alt-click or when dragged
off the canvas as one "Delete Count" state. The options bar SHALL show the
active group's count, a group dropdown, an eye toggle, a new-group button (with
a name dialog), a delete-group button, Clear, the group colour swatch, and the
marker and label sizes, and SHALL reflect a group add or delete as one "Add
Count Group" or "Delete Count Group" state. The canvas SHALL draw every visible
group's marks as numbered discs in the group's colour and sizes with any tool
active.

#### Scenario: Counting objects

- **WHEN** the `healing_tools` self-test clicks the Count tool twice, adds a second group, clicks once more, and hides the first group
- **THEN** the overlay shows two marks, then three, then one, and each step records one state

#### Scenario: Styling a group

- **WHEN** a group's colour, marker size, and label size are set from the options bar
- **THEN** the group stores them and the overlay draws its marks with them
