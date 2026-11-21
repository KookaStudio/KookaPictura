# Spec Delta

## MODIFIED Requirements

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
active while `View > Show > Count` and `View > Extras` are both on. The Count
tool itself SHALL keep them visible while it is active when Extras is on;
Extras SHALL hide them regardless of the active tool. `View > Show > Count`
SHALL be a checkable command, on by default, persisted in the session, and
applied to every canvas.

#### Scenario: Counting objects

- **WHEN** the `healing_tools` self-test clicks the Count tool twice, adds a second group, clicks once more, and hides the first group
- **THEN** the overlay shows two marks, then three, then one, and each step records one state

#### Scenario: Styling a group

- **WHEN** a group's colour, marker size, and label size are set from the options bar
- **THEN** the group stores them and the overlay draws its marks with them

#### Scenario: Show gates count marks for other tools [count_show_gate]

- **WHEN** marks exist and another tool is active with Show > Count on, then off
- **THEN** the marks are visible and then hidden, while the Count tool itself keeps them visible

#### Scenario: Extras hides count marks [count_extras]

- **WHEN** Show > Count is on and Extras is turned off, then on
- **THEN** the marks hide and return with the Show checkmark unchanged

#### Scenario: Extras wins over the Count tool [count_tool_extras]

- **WHEN** the Count tool is active with marks visible and Extras is turned off, then on
- **THEN** the marks hide while Extras is off even with the Count tool active, and return with Extras on
