## ADDED Requirements

### Requirement: Select path components and anchors

Path Selection and Direct Selection SHALL support Shift-click toggling and marquee selection of multiple components or anchors respectively, including across components of the active Work, saved, or editable shape/vector-mask path. An ordinary click SHALL replace the relevant selection. Clicking empty canvas, switching tools, changing the path target, or invalidating selection through document changes SHALL clear it. Selected components and anchors SHALL be visually distinguishable. Selection alone SHALL NOT edit geometry or add history.

#### Scenario: Toggle and replace selection [pt_select]

- **WHEN** components or anchors in different components are selected by Shift-click and then an ordinary click selects another
- **THEN** Shift-click toggles the hit item, the ordinary click replaces the selection, and geometry and history remain unchanged

#### Scenario: Marquee selection [pt_marquee]

- **WHEN** a marquee encloses items in multiple components of the active path
- **THEN** the enclosed components or anchors are selected and visibly distinguished without changing geometry or history

#### Scenario: Clear and invalidate selection [pt_clear]

- **WHEN** multiple items are selected and the user clicks empty canvas, switches tools, changes the target, or a document change invalidates the selection
- **THEN** selection clears without adding history, including when the replacement target has matching indices
